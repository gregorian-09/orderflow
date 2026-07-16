//! Additive OMS building blocks for execution integrations.

use std::collections::{HashMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use of_execution_core::{
    execution_wal_checksum, AccountId, AmendRequest, CancelRequest, ClientOrderId,
    ExecutionCoreError, ExecutionEvent, ExecutionSymbol, ExecutionText, ExecutionType, FixedAscii,
    OrderPrice, OrderQty, OrderRequest, OrderSide, OrderState, OrderStatus, OrderType, RiskCheck,
    RiskContext, RiskDecision, RiskLimits, RiskRejectReason, RouteId, StrategyId, TimeInForce,
    WalIntegrityReport, WalRecordKind, WalRecordView, WalReplayCursor, WalSegmentId, WalSequence,
    WalSyncPolicy,
};

use crate::{
    AllowAllRiskGate, ExecutionAdapter, ExecutionCapabilities, ExecutionCommand,
    ExecutionCommandKind, ExecutionCommandReport, ExecutionEngine, ExecutionError,
    ExecutionEventBuffer, ExecutionJournal, ExecutionMetrics, ExecutionResult, InMemoryJournal,
    JournalCommandKind, JournalRecord, RouteConfig, RouteKey, SimExecutionAdapter,
};

/// Monotonic command identifier assigned before a command enters an OMS queue.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CommandId(pub u64);

/// Request identifier used to correlate strategy intent, command queue entry,
/// and downstream execution reports.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RequestId(pub FixedAscii<40>);

impl RequestId {
    /// Creates a request id from ASCII text.
    ///
    /// # Errors
    ///
    /// Returns an error when the id exceeds capacity or is not ASCII.
    pub fn new(value: &str) -> Result<Self, of_execution_core::ExecutionCoreError> {
        Ok(Self(FixedAscii::new(value)?))
    }

    /// Returns the request id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Lock-free monotonic command id generator.
#[derive(Debug, Default)]
pub struct CommandIdGenerator {
    next: AtomicU64,
}

impl CommandIdGenerator {
    /// Creates a generator starting at `first`.
    pub const fn new(first: u64) -> Self {
        Self {
            next: AtomicU64::new(first),
        }
    }

    /// Returns the next command id.
    pub fn next(&self) -> CommandId {
        CommandId(self.next.fetch_add(1, Ordering::Relaxed))
    }
}

/// Correlation envelope for an execution command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandCorrelation {
    /// Monotonic command id.
    pub command_id: CommandId,
    /// Optional strategy/request id.
    pub request_id: RequestId,
    /// Client order id associated with the command.
    pub client_order_id: ClientOrderId,
    /// Command kind.
    pub kind: ExecutionCommandKind,
}

impl CommandCorrelation {
    /// Creates a command correlation envelope.
    pub const fn new(
        command_id: CommandId,
        request_id: RequestId,
        client_order_id: ClientOrderId,
        kind: ExecutionCommandKind,
    ) -> Self {
        Self {
            command_id,
            request_id,
            client_order_id,
            kind,
        }
    }
}

/// Event subscriber for execution fanout.
#[derive(Debug)]
pub struct ExecutionEventSubscriber {
    receiver: Receiver<ExecutionEvent>,
}

impl ExecutionEventSubscriber {
    /// Receives the next execution event.
    ///
    /// # Errors
    ///
    /// Returns an error when the fanout source has been dropped.
    pub fn recv(&self) -> Result<ExecutionEvent, ExecutionError> {
        self.receiver
            .recv()
            .map_err(|_| ExecutionError::Adapter("execution event fanout closed".to_string()))
    }

    /// Attempts to receive one execution event without blocking.
    pub fn try_recv(&self) -> Option<ExecutionEvent> {
        self.receiver.try_recv().ok()
    }
}

#[derive(Debug)]
struct FanoutInner {
    subscribers: Vec<SyncSender<ExecutionEvent>>,
    dropped_events: u64,
}

/// Bounded execution-event fanout for multiple consumers.
#[derive(Debug, Clone)]
pub struct ExecutionEventFanout {
    inner: Arc<Mutex<FanoutInner>>,
    subscriber_capacity: usize,
}

impl ExecutionEventFanout {
    /// Creates an empty fanout with per-subscriber queue capacity.
    pub fn new(subscriber_capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(FanoutInner {
                subscribers: Vec::new(),
                dropped_events: 0,
            })),
            subscriber_capacity,
        }
    }

    /// Adds a subscriber.
    pub fn subscribe(&self) -> ExecutionEventSubscriber {
        let (tx, receiver) = mpsc::sync_channel(self.subscriber_capacity);
        let mut inner = self.inner.lock().expect("fanout mutex");
        inner.subscribers.push(tx);
        ExecutionEventSubscriber { receiver }
    }

    /// Publishes an event to all active subscribers.
    pub fn publish(&self, event: ExecutionEvent) {
        let mut inner = self.inner.lock().expect("fanout mutex");
        let mut dropped = 0_u64;
        inner
            .subscribers
            .retain(|subscriber| match subscriber.try_send(event) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) => {
                    dropped = dropped.saturating_add(1);
                    true
                }
                Err(TrySendError::Disconnected(_)) => false,
            });
        inner.dropped_events = inner.dropped_events.saturating_add(dropped);
    }

    /// Publishes all events in `events`.
    pub fn publish_buffer(&self, events: &ExecutionEventBuffer) {
        for event in events.as_slice() {
            self.publish(*event);
        }
    }

    /// Returns the number of event deliveries dropped because a subscriber
    /// queue was full.
    pub fn dropped_events(&self) -> u64 {
        self.inner.lock().expect("fanout mutex").dropped_events
    }

    /// Returns current active subscriber count.
    pub fn subscriber_count(&self) -> usize {
        self.inner.lock().expect("fanout mutex").subscribers.len()
    }
}

/// Venue adapter/session lifecycle state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExecutionAdapterState {
    /// Transport is disconnected.
    #[default]
    Disconnected = 0,
    /// Transport connect is in progress.
    Connecting = 1,
    /// Protocol logon/authentication is in progress.
    LogonPending = 2,
    /// Session is ready for order flow.
    Ready = 3,
    /// Session is recovering state after reconnect.
    Recovering = 4,
    /// Session is connected but degraded.
    Degraded = 5,
    /// Session is stopped intentionally.
    Stopped = 6,
}

/// Execution lifecycle snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecutionLifecycleSnapshot {
    /// Current adapter/session state.
    pub state: ExecutionAdapterState,
    /// Monotonic lifecycle sequence.
    pub sequence: u64,
    /// Last transition timestamp in nanoseconds.
    pub updated_ns: u64,
    /// Last lifecycle error.
    pub last_error: Option<String>,
}

/// Mutable lifecycle tracker for adapters and supervisors.
#[derive(Debug, Clone, Default)]
pub struct ExecutionLifecycle {
    snapshot: ExecutionLifecycleSnapshot,
}

impl ExecutionLifecycle {
    /// Creates a disconnected lifecycle tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Transitions to `state`.
    pub fn transition(
        &mut self,
        state: ExecutionAdapterState,
        updated_ns: u64,
        last_error: Option<String>,
    ) -> ExecutionLifecycleSnapshot {
        self.snapshot.state = state;
        self.snapshot.sequence = self.snapshot.sequence.saturating_add(1);
        self.snapshot.updated_ns = updated_ns;
        self.snapshot.last_error = last_error;
        self.snapshot.clone()
    }

    /// Returns current lifecycle snapshot.
    pub fn snapshot(&self) -> ExecutionLifecycleSnapshot {
        self.snapshot.clone()
    }
}

/// Durable append-only execution journal.
#[derive(Debug)]
pub struct FileExecutionJournal {
    path: PathBuf,
    file: File,
    sync_on_write: bool,
}

impl FileExecutionJournal {
    /// Opens or creates a file-backed execution journal.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the file cannot be opened.
    pub fn open(path: impl AsRef<Path>, sync_on_write: bool) -> ExecutionResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        Ok(Self {
            path,
            file,
            sync_on_write,
        })
    }

    /// Returns the journal path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn write_line(&mut self, line: &str) -> ExecutionResult<()> {
        self.file
            .write_all(line.as_bytes())
            .and_then(|()| self.file.write_all(b"\n"))
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        if self.sync_on_write {
            self.file
                .sync_data()
                .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        }
        Ok(())
    }
}

impl ExecutionJournal for FileExecutionJournal {
    fn record_command(
        &mut self,
        kind: JournalCommandKind,
        id: ClientOrderId,
        ts_ns: u64,
    ) -> ExecutionResult<()> {
        self.write_line(&format!("C|{}|{}|{}", command_kind_u8(kind), id, ts_ns))
    }

    fn record_event(&mut self, event: &ExecutionEvent) -> ExecutionResult<()> {
        self.write_line(&event_to_journal_line(event))
    }

    fn replay(&self, out: &mut Vec<JournalRecord>) -> ExecutionResult<usize> {
        let file =
            File::open(&self.path).map_err(|err| ExecutionError::Journal(err.to_string()))?;
        let reader = BufReader::new(file);
        let start = out.len();
        for line in reader.lines() {
            let line = line.map_err(|err| ExecutionError::Journal(err.to_string()))?;
            if line.is_empty() {
                continue;
            }
            if let Some(record) = parse_journal_line(&line)? {
                out.push(record);
            }
        }
        Ok(out.len().saturating_sub(start))
    }
}

/// Configuration for [`WalExecutionJournal`].
#[derive(Debug, Clone)]
pub struct WalJournalConfig {
    path: PathBuf,
    sync_policy: WalSyncPolicy,
}

impl WalJournalConfig {
    /// Creates a WAL journal config for `path`.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            sync_policy: WalSyncPolicy::EveryRecord,
        }
    }

    /// Sets the durability sync policy.
    pub fn with_sync_policy(mut self, sync_policy: WalSyncPolicy) -> Self {
        self.sync_policy = sync_policy;
        self
    }

    /// Returns the WAL file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the configured sync policy.
    pub const fn sync_policy(&self) -> WalSyncPolicy {
        self.sync_policy
    }
}

/// Replay summary returned by WAL replay helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct WalReplayResult {
    /// Number of records replayed into the output vector.
    pub records: usize,
    /// Number of encoded bytes consumed by replay.
    pub bytes: u64,
    /// First replayed sequence.
    pub first_sequence: Option<WalSequence>,
    /// Last replayed sequence.
    pub last_sequence: Option<WalSequence>,
}

/// Low-latency execution WAL metrics snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct WalJournalMetrics {
    /// Number of WAL frames written successfully.
    pub records_written: u64,
    /// Number of encoded WAL bytes written successfully.
    pub bytes_written: u64,
    /// Number of durable sync operations completed successfully.
    pub sync_count: u64,
    /// Number of segment rotations completed successfully.
    pub segment_rotations: u64,
    /// Number of manifest writes completed successfully.
    pub manifest_writes: u64,
    /// Number of WAL frame write failures.
    pub write_failures: u64,
    /// Number of sync failures.
    pub sync_failures: u64,
    /// Number of manifest write failures.
    pub manifest_write_failures: u64,
    /// Cumulative write latency in nanoseconds.
    pub total_write_latency_ns: u128,
    /// Maximum observed write latency in nanoseconds.
    pub max_write_latency_ns: u64,
    /// Cumulative sync latency in nanoseconds.
    pub total_sync_latency_ns: u128,
    /// Maximum observed sync latency in nanoseconds.
    pub max_sync_latency_ns: u64,
}

impl WalJournalMetrics {
    /// Returns average successful write latency in nanoseconds.
    pub fn average_write_latency_ns(&self) -> u64 {
        if self.records_written == 0 {
            0
        } else {
            (self.total_write_latency_ns / u128::from(self.records_written)) as u64
        }
    }

    /// Returns average successful sync latency in nanoseconds.
    pub fn average_sync_latency_ns(&self) -> u64 {
        if self.sync_count == 0 {
            0
        } else {
            (self.total_sync_latency_ns / u128::from(self.sync_count)) as u64
        }
    }

    fn observe_write(&mut self, bytes: u64, latency_ns: u64) {
        self.records_written = self.records_written.saturating_add(1);
        self.bytes_written = self.bytes_written.saturating_add(bytes);
        self.total_write_latency_ns = self
            .total_write_latency_ns
            .saturating_add(u128::from(latency_ns));
        self.max_write_latency_ns = self.max_write_latency_ns.max(latency_ns);
    }

    fn observe_sync(&mut self, latency_ns: u64) {
        self.sync_count = self.sync_count.saturating_add(1);
        self.total_sync_latency_ns = self
            .total_sync_latency_ns
            .saturating_add(u128::from(latency_ns));
        self.max_sync_latency_ns = self.max_sync_latency_ns.max(latency_ns);
    }
}

/// Configuration for [`SegmentedWalExecutionJournal`].
#[derive(Debug, Clone)]
pub struct WalSegmentConfig {
    root: PathBuf,
    sync_policy: WalSyncPolicy,
    max_segment_bytes: u64,
    max_segment_records: u64,
}

impl WalSegmentConfig {
    /// Creates a segmented WAL config rooted at `root`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            sync_policy: WalSyncPolicy::EveryRecord,
            max_segment_bytes: 64 * 1024 * 1024,
            max_segment_records: 1_000_000,
        }
    }

    /// Sets the durability sync policy.
    pub fn with_sync_policy(mut self, sync_policy: WalSyncPolicy) -> Self {
        self.sync_policy = sync_policy;
        self
    }

    /// Sets the segment rotation threshold in bytes.
    pub fn with_max_segment_bytes(mut self, max_segment_bytes: u64) -> Self {
        self.max_segment_bytes = max_segment_bytes.max(1);
        self
    }

    /// Sets the segment rotation threshold in WAL records.
    pub fn with_max_segment_records(mut self, max_segment_records: u64) -> Self {
        self.max_segment_records = max_segment_records.max(1);
        self
    }

    /// Returns the segmented WAL root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the configured sync policy.
    pub const fn sync_policy(&self) -> WalSyncPolicy {
        self.sync_policy
    }

    /// Returns the segment rotation threshold in bytes.
    pub const fn max_segment_bytes(&self) -> u64 {
        self.max_segment_bytes
    }

    /// Returns the segment rotation threshold in WAL records.
    pub const fn max_segment_records(&self) -> u64 {
        self.max_segment_records
    }
}

/// Metadata for one execution WAL segment file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WalSegmentMetadata {
    /// Segment identifier.
    pub segment_id: WalSegmentId,
    /// Segment file path.
    pub path: PathBuf,
    /// First WAL sequence observed in the segment.
    pub first_sequence: Option<WalSequence>,
    /// Last WAL sequence observed in the segment.
    pub last_sequence: Option<WalSequence>,
    /// Number of WAL frames in the segment.
    pub records: u64,
    /// Number of encoded bytes in the segment.
    pub bytes: u64,
    /// True when the segment ends with a segment-seal marker.
    pub sealed: bool,
    /// Segment creation timestamp in nanoseconds.
    pub created_ns: u64,
    /// Last metadata update timestamp in nanoseconds.
    pub updated_ns: u64,
}

impl WalSegmentMetadata {
    fn empty(segment_id: WalSegmentId, path: PathBuf, timestamp_ns: u64) -> Self {
        Self {
            segment_id,
            path,
            first_sequence: None,
            last_sequence: None,
            records: 0,
            bytes: 0,
            sealed: false,
            created_ns: timestamp_ns,
            updated_ns: timestamp_ns,
        }
    }

    fn observe(&mut self, sequence: WalSequence, bytes: u64, kind: WalRecordKind, now_ns: u64) {
        self.first_sequence.get_or_insert(sequence);
        self.last_sequence = Some(sequence);
        self.records = self.records.saturating_add(1);
        self.bytes = self.bytes.saturating_add(bytes);
        self.sealed = kind == WalRecordKind::SegmentSeal;
        self.updated_ns = now_ns;
    }
}

/// Manifest inventory for a segmented execution WAL.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct WalSegmentManifest {
    /// Segment metadata ordered by segment id.
    pub segments: Vec<WalSegmentMetadata>,
}

impl WalSegmentManifest {
    /// Returns the currently active segment metadata.
    pub fn active_segment(&self) -> Option<&WalSegmentMetadata> {
        self.segments.last()
    }

    /// Returns the first WAL sequence in the manifest.
    pub fn first_sequence(&self) -> Option<WalSequence> {
        self.segments
            .iter()
            .find_map(|segment| segment.first_sequence)
    }

    /// Returns the last WAL sequence in the manifest.
    pub fn last_sequence(&self) -> Option<WalSequence> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.last_sequence)
    }
}

/// Integrity summary for a segmented execution WAL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct WalSegmentIntegrityReport {
    /// Number of segment files inspected.
    pub segments: usize,
    /// Number of valid WAL frames decoded.
    pub records: u64,
    /// Number of encoded bytes consumed.
    pub bytes: u64,
    /// First decoded WAL sequence.
    pub first_sequence: Option<WalSequence>,
    /// Last decoded WAL sequence.
    pub last_sequence: Option<WalSequence>,
    /// Number of checksum-link failures.
    pub checksum_failures: u64,
    /// Number of sequence gaps or regressions.
    pub sequence_failures: u64,
    /// True when all inspected segments decoded cleanly.
    pub valid: bool,
}

/// Binary append-only execution WAL journal.
///
/// This journal implements the existing [`ExecutionJournal`] trait. It records
/// the same command/event model as [`FileExecutionJournal`], but uses binary
/// WAL frames from `of_execution_core` instead of text lines.
#[derive(Debug)]
pub struct WalExecutionJournal {
    config: WalJournalConfig,
    file: File,
    next_sequence: WalSequence,
    previous_checksum: u64,
    records_since_sync: u32,
    last_sync_ns: u64,
    metrics: WalJournalMetrics,
    scratch: Vec<u8>,
    frame_scratch: Vec<u8>,
}

impl WalExecutionJournal {
    /// Opens or creates a binary WAL-backed execution journal.
    ///
    /// Existing WAL bytes are validated before the journal accepts new
    /// records. Corrupt or non-contiguous WAL data returns a journal error so
    /// callers can fail closed before trading resumes.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the file cannot be opened or
    /// existing WAL bytes fail validation.
    pub fn open(config: WalJournalConfig) -> ExecutionResult<Self> {
        let (next_sequence, previous_checksum) = scan_wal_file(config.path())?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(config.path())
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        Ok(Self {
            config,
            file,
            next_sequence,
            previous_checksum,
            records_since_sync: 0,
            last_sync_ns: now_ns(),
            metrics: WalJournalMetrics::default(),
            scratch: Vec::with_capacity(256),
            frame_scratch: Vec::with_capacity(384),
        })
    }

    /// Opens a WAL journal at `path` with a durability sync policy.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the WAL cannot be opened or its
    /// existing bytes fail validation.
    pub fn open_path(path: impl AsRef<Path>, sync_policy: WalSyncPolicy) -> ExecutionResult<Self> {
        Self::open(WalJournalConfig::new(path).with_sync_policy(sync_policy))
    }

    /// Returns the WAL file path.
    pub fn path(&self) -> &Path {
        self.config.path()
    }

    /// Returns the configured sync policy.
    pub const fn sync_policy(&self) -> WalSyncPolicy {
        self.config.sync_policy()
    }

    /// Returns the next sequence that will be assigned.
    pub const fn next_sequence(&self) -> WalSequence {
        self.next_sequence
    }

    /// Returns the current WAL metrics snapshot.
    pub const fn metrics(&self) -> WalJournalMetrics {
        self.metrics
    }

    /// Flushes and syncs the WAL file.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the OS reports a flush/sync
    /// failure.
    pub fn sync(&mut self) -> ExecutionResult<()> {
        let started_ns = now_ns();
        if let Err(err) = self.file.flush().and_then(|()| self.file.sync_data()) {
            self.metrics.sync_failures = self.metrics.sync_failures.saturating_add(1);
            return Err(ExecutionError::Journal(err.to_string()));
        }
        self.metrics
            .observe_sync(now_ns().saturating_sub(started_ns));
        self.records_since_sync = 0;
        self.last_sync_ns = now_ns();
        Ok(())
    }

    /// Returns an integrity report for the WAL file.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the WAL file cannot be read.
    pub fn integrity_report(&self) -> ExecutionResult<WalIntegrityReport> {
        let bytes =
            std::fs::read(self.path()).map_err(|err| ExecutionError::Journal(err.to_string()))?;
        let report = WalIntegrityReport::inspect(&bytes, true);
        if report.valid {
            let mut records = Vec::new();
            let _ = replay_wal_bytes(&bytes, None, &mut records)?;
        }
        Ok(report)
    }

    /// Replays records with sequence greater than or equal to `sequence`.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the WAL cannot be read, decoded,
    /// or mapped back into a journal record.
    pub fn replay_from(
        &self,
        sequence: WalSequence,
        out: &mut Vec<JournalRecord>,
    ) -> ExecutionResult<WalReplayResult> {
        let bytes =
            std::fs::read(self.path()).map_err(|err| ExecutionError::Journal(err.to_string()))?;
        replay_wal_bytes(&bytes, Some(sequence), out)
    }

    fn append_record(&mut self, kind: WalRecordKind, timestamp_ns: u64) -> ExecutionResult<()> {
        let payload_len = self.scratch.len();
        let payload = &self.scratch[..payload_len];
        let header = WalRecordView::new(kind, self.next_sequence, timestamp_ns, payload)
            .map_err(wal_error)?
            .header
            .with_previous_checksum(self.previous_checksum);
        let record = WalRecordView::from_header(header, payload).map_err(wal_error)?;
        self.frame_scratch.clear();
        self.frame_scratch.resize(record.encoded_len(), 0);
        record
            .encode_into(&mut self.frame_scratch)
            .map_err(wal_error)?;

        let write_started_ns = now_ns();
        if let Err(err) = self.file.write_all(&self.frame_scratch) {
            self.metrics.write_failures = self.metrics.write_failures.saturating_add(1);
            return Err(ExecutionError::Journal(err.to_string()));
        }
        self.metrics.observe_write(
            record.encoded_len() as u64,
            now_ns().saturating_sub(write_started_ns),
        );
        self.previous_checksum = record.header.header_checksum;
        self.next_sequence = self.next_sequence.next();
        self.records_since_sync = self.records_since_sync.saturating_add(1);
        self.maybe_sync(kind)
    }

    fn maybe_sync(&mut self, kind: WalRecordKind) -> ExecutionResult<()> {
        match self.config.sync_policy() {
            WalSyncPolicy::Never | WalSyncPolicy::Manual => Ok(()),
            WalSyncPolicy::EveryRecord => self.sync(),
            WalSyncPolicy::EveryNRecords(records) => {
                if records > 0 && self.records_since_sync >= records {
                    self.sync()
                } else {
                    Ok(())
                }
            }
            WalSyncPolicy::EveryDurationNs(duration_ns) => {
                if duration_ns > 0 && now_ns().saturating_sub(self.last_sync_ns) >= duration_ns {
                    self.sync()
                } else {
                    Ok(())
                }
            }
            WalSyncPolicy::OnRiskBoundary => {
                if is_risk_boundary_wal_kind(kind) {
                    self.sync()
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
}

impl ExecutionJournal for WalExecutionJournal {
    fn record_command(
        &mut self,
        kind: JournalCommandKind,
        id: ClientOrderId,
        ts_ns: u64,
    ) -> ExecutionResult<()> {
        self.scratch.clear();
        encode_command_payload(kind, id, ts_ns, &mut self.scratch);
        self.append_record(command_wal_kind(kind), ts_ns)
    }

    fn record_event(&mut self, event: &ExecutionEvent) -> ExecutionResult<()> {
        self.scratch.clear();
        encode_event_payload(event, &mut self.scratch);
        self.append_record(event_wal_kind(event), event.ts_recv_ns)
    }

    fn replay(&self, out: &mut Vec<JournalRecord>) -> ExecutionResult<usize> {
        let start = out.len();
        let _ = self.replay_from(WalSequence(1), out)?;
        Ok(out.len().saturating_sub(start))
    }
}

/// Segmented binary execution WAL journal.
///
/// This journal is additive to [`WalExecutionJournal`]. It stores WAL frames in
/// ordered segment files under a root directory, writes an operator-readable
/// manifest after metadata changes, and preserves the same [`ExecutionJournal`]
/// replay model.
#[derive(Debug)]
pub struct SegmentedWalExecutionJournal {
    config: WalSegmentConfig,
    manifest: WalSegmentManifest,
    file: File,
    next_sequence: WalSequence,
    previous_checksum: u64,
    records_since_sync: u32,
    last_sync_ns: u64,
    metrics: WalJournalMetrics,
    scratch: Vec<u8>,
    frame_scratch: Vec<u8>,
}

impl SegmentedWalExecutionJournal {
    /// Opens or creates a segmented binary WAL-backed execution journal.
    ///
    /// Existing segment files are scanned in segment-id order and checksum
    /// links are validated across segment boundaries before the journal accepts
    /// new records.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the directory cannot be opened,
    /// a segment cannot be read, or existing WAL bytes fail validation.
    pub fn open(config: WalSegmentConfig) -> ExecutionResult<Self> {
        fs::create_dir_all(config.root())
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;

        let (manifest, next_sequence, previous_checksum) = load_segment_manifest(&config)?;
        let active_id = manifest
            .active_segment()
            .map(|segment| {
                if segment.sealed {
                    WalSegmentId(segment.segment_id.0.saturating_add(1))
                } else {
                    segment.segment_id
                }
            })
            .unwrap_or(WalSegmentId(1));
        let active_path = segment_path(config.root(), active_id);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&active_path)
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;

        let mut journal = Self {
            config,
            manifest,
            file,
            next_sequence,
            previous_checksum,
            records_since_sync: 0,
            last_sync_ns: now_ns(),
            metrics: WalJournalMetrics::default(),
            scratch: Vec::with_capacity(256),
            frame_scratch: Vec::with_capacity(384),
        };

        let needs_active_segment = journal
            .manifest
            .active_segment()
            .is_none_or(|segment| segment.segment_id != active_id);
        if needs_active_segment {
            journal.manifest.segments.push(WalSegmentMetadata::empty(
                active_id,
                active_path,
                journal.last_sync_ns,
            ));
        }
        journal.write_manifest()?;
        Ok(journal)
    }

    /// Opens a segmented WAL at `root` with a durability sync policy.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the WAL cannot be opened or its
    /// existing segments fail validation.
    pub fn open_root(root: impl AsRef<Path>, sync_policy: WalSyncPolicy) -> ExecutionResult<Self> {
        Self::open(WalSegmentConfig::new(root).with_sync_policy(sync_policy))
    }

    /// Returns the segmented WAL root directory.
    pub fn root(&self) -> &Path {
        self.config.root()
    }

    /// Returns the configured sync policy.
    pub const fn sync_policy(&self) -> WalSyncPolicy {
        self.config.sync_policy()
    }

    /// Returns the next sequence that will be assigned.
    pub const fn next_sequence(&self) -> WalSequence {
        self.next_sequence
    }

    /// Returns the current manifest snapshot.
    pub fn manifest(&self) -> &WalSegmentManifest {
        &self.manifest
    }

    /// Returns the current segmented WAL metrics snapshot.
    pub const fn metrics(&self) -> WalJournalMetrics {
        self.metrics
    }

    /// Inspects a segmented WAL root without opening it for append.
    ///
    /// This helper is intended for operator diagnostics and binding layers. It
    /// scans `wal-*.ofwal` files in segment-id order, validates checksum links
    /// and sequence continuity, and returns a report instead of creating a new
    /// active segment.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the root cannot be listed or a
    /// segment file cannot be read.
    pub fn inspect_root(root: impl AsRef<Path>) -> ExecutionResult<WalSegmentIntegrityReport> {
        inspect_segmented_wal_root(root.as_ref())
    }

    /// Flushes and syncs the active segment file.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the OS reports a flush/sync
    /// failure.
    pub fn sync(&mut self) -> ExecutionResult<()> {
        let started_ns = now_ns();
        if let Err(err) = self.file.flush().and_then(|()| self.file.sync_data()) {
            self.metrics.sync_failures = self.metrics.sync_failures.saturating_add(1);
            return Err(ExecutionError::Journal(err.to_string()));
        }
        self.metrics
            .observe_sync(now_ns().saturating_sub(started_ns));
        self.write_manifest()?;
        self.records_since_sync = 0;
        self.last_sync_ns = now_ns();
        Ok(())
    }

    /// Rotates to a new empty WAL segment.
    ///
    /// A seal marker is appended to the current active segment before the new
    /// file is opened. The marker consumes a WAL sequence and is skipped by
    /// journal replay.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the current segment cannot be
    /// sealed, the manifest cannot be written, or the new segment cannot be
    /// opened.
    pub fn rotate_segment(&mut self) -> ExecutionResult<WalSegmentMetadata> {
        if self
            .manifest
            .active_segment()
            .is_some_and(|segment| segment.records > 0 && !segment.sealed)
        {
            self.append_record(WalRecordKind::SegmentSeal, now_ns())?;
            self.sync()?;
        }

        let next_id = self
            .manifest
            .active_segment()
            .map(|segment| WalSegmentId(segment.segment_id.0.saturating_add(1)))
            .unwrap_or(WalSegmentId(1));
        let path = segment_path(self.config.root(), next_id);
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        let metadata = WalSegmentMetadata::empty(next_id, path, now_ns());
        self.manifest.segments.push(metadata.clone());
        self.metrics.segment_rotations = self.metrics.segment_rotations.saturating_add(1);
        self.write_manifest()?;
        Ok(metadata)
    }

    /// Returns an aggregate integrity report across all segment files.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when a segment cannot be read.
    pub fn integrity_report(&self) -> ExecutionResult<WalSegmentIntegrityReport> {
        let mut previous_checksum = 0;
        let mut expected_sequence = WalSequence(1);
        let mut report = WalSegmentIntegrityReport {
            segments: self.manifest.segments.len(),
            valid: true,
            ..WalSegmentIntegrityReport::default()
        };

        for segment in &self.manifest.segments {
            match scan_segment_file(&segment.path, previous_checksum, Some(expected_sequence)) {
                Ok(scan) => {
                    report.records = report.records.saturating_add(scan.records);
                    report.bytes = report.bytes.saturating_add(scan.bytes);
                    if report.first_sequence.is_none() {
                        report.first_sequence = scan.first_sequence;
                    }
                    report.last_sequence = scan.last_sequence.or(report.last_sequence);
                    previous_checksum = scan.previous_checksum;
                    expected_sequence = scan.next_sequence;
                }
                Err(err) => {
                    let message = err.to_string();
                    if message.contains("sequence") {
                        report.sequence_failures = report.sequence_failures.saturating_add(1);
                    } else {
                        report.checksum_failures = report.checksum_failures.saturating_add(1);
                    }
                    report.valid = false;
                    return Ok(report);
                }
            }
        }
        Ok(report)
    }

    /// Replays records with sequence greater than or equal to `sequence`.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when any segment cannot be read,
    /// decoded, or mapped back into a journal record.
    pub fn replay_from(
        &self,
        sequence: WalSequence,
        out: &mut Vec<JournalRecord>,
    ) -> ExecutionResult<WalReplayResult> {
        let start = out.len();
        let mut result = WalReplayResult::default();
        let mut previous_checksum = 0;
        let mut expected_sequence = WalSequence(1);

        for segment in &self.manifest.segments {
            let bytes =
                fs::read(&segment.path).map_err(|err| ExecutionError::Journal(err.to_string()))?;
            let mut cursor = WalReplayCursor::new(&bytes).with_strict_sequence(false);
            while let Some(record) = cursor.next_record().map_err(wal_error)? {
                validate_wal_sequence(record.header.sequence, expected_sequence)?;
                validate_wal_link(&record, previous_checksum)?;
                previous_checksum = record.header.header_checksum;
                expected_sequence = record.header.sequence.next();
                result.bytes = result.bytes.saturating_add(record.encoded_len() as u64);
                if record.header.sequence < sequence {
                    continue;
                }
                if let Some(journal_record) = decode_wal_payload(&record)? {
                    out.push(journal_record);
                    result.records = out.len().saturating_sub(start);
                    result.first_sequence.get_or_insert(record.header.sequence);
                    result.last_sequence = Some(record.header.sequence);
                }
            }
        }
        Ok(result)
    }

    fn append_record(&mut self, kind: WalRecordKind, timestamp_ns: u64) -> ExecutionResult<()> {
        let encoded_len = {
            let payload = &self.scratch;
            let header = WalRecordView::new(kind, self.next_sequence, timestamp_ns, payload)
                .map_err(wal_error)?
                .header
                .with_previous_checksum(self.previous_checksum);
            WalRecordView::from_header(header, payload)
                .map_err(wal_error)?
                .encoded_len() as u64
        };
        if kind != WalRecordKind::SegmentSeal && self.should_rotate_before(encoded_len) {
            self.rotate_segment()?;
        }

        let payload = &self.scratch;
        let header = WalRecordView::new(kind, self.next_sequence, timestamp_ns, payload)
            .map_err(wal_error)?
            .header
            .with_previous_checksum(self.previous_checksum);
        let record = WalRecordView::from_header(header, payload).map_err(wal_error)?;
        self.frame_scratch.clear();
        self.frame_scratch.resize(record.encoded_len(), 0);
        record
            .encode_into(&mut self.frame_scratch)
            .map_err(wal_error)?;

        let write_started_ns = now_ns();
        if let Err(err) = self.file.write_all(&self.frame_scratch) {
            self.metrics.write_failures = self.metrics.write_failures.saturating_add(1);
            return Err(ExecutionError::Journal(err.to_string()));
        }
        self.metrics.observe_write(
            record.encoded_len() as u64,
            now_ns().saturating_sub(write_started_ns),
        );
        self.previous_checksum = record.header.header_checksum;
        self.next_sequence = self.next_sequence.next();
        self.records_since_sync = self.records_since_sync.saturating_add(1);
        if let Some(active) = self.manifest.segments.last_mut() {
            active.observe(
                record.header.sequence,
                record.encoded_len() as u64,
                kind,
                now_ns(),
            );
        }
        self.maybe_sync(kind)
    }

    fn should_rotate_before(&self, next_record_bytes: u64) -> bool {
        self.manifest.active_segment().is_some_and(|segment| {
            segment.records > 0
                && (segment.records >= self.config.max_segment_records()
                    || segment.bytes.saturating_add(next_record_bytes)
                        > self.config.max_segment_bytes())
        })
    }

    fn write_manifest(&mut self) -> ExecutionResult<()> {
        match write_segment_manifest(self.config.root(), &self.manifest) {
            Ok(()) => {
                self.metrics.manifest_writes = self.metrics.manifest_writes.saturating_add(1);
                Ok(())
            }
            Err(err) => {
                self.metrics.manifest_write_failures =
                    self.metrics.manifest_write_failures.saturating_add(1);
                Err(err)
            }
        }
    }

    fn maybe_sync(&mut self, kind: WalRecordKind) -> ExecutionResult<()> {
        match self.config.sync_policy() {
            WalSyncPolicy::Never | WalSyncPolicy::Manual => Ok(()),
            WalSyncPolicy::EveryRecord => self.sync(),
            WalSyncPolicy::EveryNRecords(records) => {
                if records > 0 && self.records_since_sync >= records {
                    self.sync()
                } else {
                    Ok(())
                }
            }
            WalSyncPolicy::EveryDurationNs(duration_ns) => {
                if duration_ns > 0 && now_ns().saturating_sub(self.last_sync_ns) >= duration_ns {
                    self.sync()
                } else {
                    Ok(())
                }
            }
            WalSyncPolicy::OnRiskBoundary => {
                if is_risk_boundary_wal_kind(kind) {
                    self.sync()
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
}

impl ExecutionJournal for SegmentedWalExecutionJournal {
    fn record_command(
        &mut self,
        kind: JournalCommandKind,
        id: ClientOrderId,
        ts_ns: u64,
    ) -> ExecutionResult<()> {
        self.scratch.clear();
        encode_command_payload(kind, id, ts_ns, &mut self.scratch);
        self.append_record(command_wal_kind(kind), ts_ns)
    }

    fn record_event(&mut self, event: &ExecutionEvent) -> ExecutionResult<()> {
        self.scratch.clear();
        encode_event_payload(event, &mut self.scratch);
        self.append_record(event_wal_kind(event), event.ts_recv_ns)
    }

    fn replay(&self, out: &mut Vec<JournalRecord>) -> ExecutionResult<usize> {
        let start = out.len();
        let _ = self.replay_from(WalSequence(1), out)?;
        Ok(out.len().saturating_sub(start))
    }
}

const WAL_PAYLOAD_VERSION: u16 = 1;

fn scan_wal_file(path: &Path) -> ExecutionResult<(WalSequence, u64)> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok((WalSequence(1), 0));
        }
        Err(err) => return Err(ExecutionError::Journal(err.to_string())),
    };
    if bytes.is_empty() {
        return Ok((WalSequence(1), 0));
    }

    let mut cursor = WalReplayCursor::new(&bytes);
    let mut next_sequence = WalSequence(1);
    let mut previous_checksum = 0;
    while let Some(record) = cursor.next_record().map_err(wal_error)? {
        validate_wal_link(&record, previous_checksum)?;
        let _ = decode_wal_payload(&record)?;
        next_sequence = record.header.sequence.next();
        previous_checksum = record.header.header_checksum;
    }
    Ok((next_sequence, previous_checksum))
}

#[derive(Debug, Clone, Copy, Default)]
struct SegmentScan {
    first_sequence: Option<WalSequence>,
    last_sequence: Option<WalSequence>,
    next_sequence: WalSequence,
    previous_checksum: u64,
    records: u64,
    bytes: u64,
    sealed: bool,
}

fn load_segment_manifest(
    config: &WalSegmentConfig,
) -> ExecutionResult<(WalSegmentManifest, WalSequence, u64)> {
    let segment_ids = list_segment_ids(config.root())?;
    if segment_ids.is_empty() {
        return Ok((WalSegmentManifest::default(), WalSequence(1), 0));
    }

    let mut manifest = WalSegmentManifest::default();
    let mut expected_sequence = WalSequence(1);
    let mut previous_checksum = 0;
    for segment_id in segment_ids {
        let path = segment_path(config.root(), segment_id);
        let scan = scan_segment_file(&path, previous_checksum, Some(expected_sequence))?;
        let updated_ns = now_ns();
        manifest.segments.push(WalSegmentMetadata {
            segment_id,
            path,
            first_sequence: scan.first_sequence,
            last_sequence: scan.last_sequence,
            records: scan.records,
            bytes: scan.bytes,
            sealed: scan.sealed,
            created_ns: updated_ns,
            updated_ns,
        });
        expected_sequence = scan.next_sequence;
        previous_checksum = scan.previous_checksum;
    }

    Ok((manifest, expected_sequence, previous_checksum))
}

fn inspect_segmented_wal_root(root: &Path) -> ExecutionResult<WalSegmentIntegrityReport> {
    if !root.exists() {
        return Err(ExecutionError::Journal(format!(
            "segmented WAL root does not exist: {}",
            root.display()
        )));
    }
    let segment_ids = list_segment_ids(root)?;
    let mut report = WalSegmentIntegrityReport {
        segments: segment_ids.len(),
        valid: true,
        ..WalSegmentIntegrityReport::default()
    };
    let mut expected_sequence = WalSequence(1);
    let mut previous_checksum = 0;

    for segment_id in segment_ids {
        let path = segment_path(root, segment_id);
        match scan_segment_file(&path, previous_checksum, Some(expected_sequence)) {
            Ok(scan) => {
                report.records = report.records.saturating_add(scan.records);
                report.bytes = report.bytes.saturating_add(scan.bytes);
                if report.first_sequence.is_none() {
                    report.first_sequence = scan.first_sequence;
                }
                report.last_sequence = scan.last_sequence.or(report.last_sequence);
                expected_sequence = scan.next_sequence;
                previous_checksum = scan.previous_checksum;
            }
            Err(err) => {
                let message = err.to_string();
                if message.contains("sequence") {
                    report.sequence_failures = report.sequence_failures.saturating_add(1);
                } else {
                    report.checksum_failures = report.checksum_failures.saturating_add(1);
                }
                report.valid = false;
                break;
            }
        }
    }
    Ok(report)
}

fn scan_segment_file(
    path: &Path,
    initial_previous_checksum: u64,
    initial_expected_sequence: Option<WalSequence>,
) -> ExecutionResult<SegmentScan> {
    let bytes = fs::read(path).map_err(|err| ExecutionError::Journal(err.to_string()))?;
    if bytes.is_empty() {
        return Ok(SegmentScan {
            next_sequence: initial_expected_sequence.unwrap_or(WalSequence(1)),
            previous_checksum: initial_previous_checksum,
            ..SegmentScan::default()
        });
    }

    let mut cursor = WalReplayCursor::new(&bytes).with_strict_sequence(false);
    let mut scan = SegmentScan {
        next_sequence: initial_expected_sequence.unwrap_or(WalSequence(1)),
        previous_checksum: initial_previous_checksum,
        ..SegmentScan::default()
    };
    while let Some(record) = cursor.next_record().map_err(wal_error)? {
        validate_wal_sequence(record.header.sequence, scan.next_sequence)?;
        validate_wal_link(&record, scan.previous_checksum)?;
        let _ = decode_wal_payload(&record)?;
        scan.first_sequence.get_or_insert(record.header.sequence);
        scan.last_sequence = Some(record.header.sequence);
        scan.next_sequence = record.header.sequence.next();
        scan.previous_checksum = record.header.header_checksum;
        scan.records = scan.records.saturating_add(1);
        scan.bytes = cursor.offset() as u64;
        scan.sealed = record.header.kind == WalRecordKind::SegmentSeal;
    }
    Ok(scan)
}

fn validate_wal_sequence(actual: WalSequence, expected: WalSequence) -> ExecutionResult<()> {
    if actual == expected {
        Ok(())
    } else if actual < expected {
        Err(ExecutionError::Journal(format!(
            "WAL sequence regression: expected {}, actual {}",
            expected.0, actual.0
        )))
    } else {
        Err(ExecutionError::Journal(format!(
            "WAL sequence gap: expected {}, actual {}",
            expected.0, actual.0
        )))
    }
}

fn list_segment_ids(root: &Path) -> ExecutionResult<Vec<WalSegmentId>> {
    let mut ids = Vec::new();
    if !root.exists() {
        return Ok(ids);
    }
    for entry in fs::read_dir(root).map_err(|err| ExecutionError::Journal(err.to_string()))? {
        let entry = entry.map_err(|err| ExecutionError::Journal(err.to_string()))?;
        let path = entry.path();
        if let Some(id) = parse_segment_id(&path) {
            ids.push(id);
        }
    }
    ids.sort_unstable_by_key(|id| id.0);
    Ok(ids)
}

fn parse_segment_id(path: &Path) -> Option<WalSegmentId> {
    let file_name = path.file_name()?.to_str()?;
    let digits = file_name.strip_prefix("wal-")?.strip_suffix(".ofwal")?;
    digits.parse::<u64>().ok().map(WalSegmentId)
}

fn segment_path(root: &Path, segment_id: WalSegmentId) -> PathBuf {
    root.join(format!("wal-{:012}.ofwal", segment_id.0))
}

fn write_segment_manifest(root: &Path, manifest: &WalSegmentManifest) -> ExecutionResult<()> {
    let final_path = root.join("manifest");
    let tmp_path = root.join("manifest.tmp");
    let mut bytes = Vec::with_capacity(manifest.segments.len().saturating_mul(128));
    bytes.extend_from_slice(b"version=1\n");
    for segment in &manifest.segments {
        let file_name = segment
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        bytes.extend_from_slice(
            format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
                segment.segment_id.0,
                file_name,
                segment.first_sequence.map_or(0, |sequence| sequence.0),
                segment.last_sequence.map_or(0, |sequence| sequence.0),
                segment.records,
                segment.bytes,
                u8::from(segment.sealed),
                segment.created_ns,
                segment.updated_ns
            )
            .as_bytes(),
        );
    }
    {
        let mut file =
            File::create(&tmp_path).map_err(|err| ExecutionError::Journal(err.to_string()))?;
        file.write_all(&bytes)
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        file.flush()
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
    }
    fs::rename(&tmp_path, &final_path).map_err(|err| ExecutionError::Journal(err.to_string()))?;
    Ok(())
}

fn replay_wal_bytes(
    bytes: &[u8],
    from_sequence: Option<WalSequence>,
    out: &mut Vec<JournalRecord>,
) -> ExecutionResult<WalReplayResult> {
    let start = out.len();
    let mut result = WalReplayResult::default();
    let mut cursor = WalReplayCursor::new(bytes);
    let mut previous_checksum = 0;

    while let Some(record) = cursor.next_record().map_err(wal_error)? {
        validate_wal_link(&record, previous_checksum)?;
        previous_checksum = record.header.header_checksum;
        result.bytes = cursor.offset() as u64;
        if from_sequence.is_some_and(|sequence| record.header.sequence < sequence) {
            continue;
        }
        if let Some(journal_record) = decode_wal_payload(&record)? {
            out.push(journal_record);
            result.records = out.len().saturating_sub(start);
            result.first_sequence.get_or_insert(record.header.sequence);
            result.last_sequence = Some(record.header.sequence);
        }
    }

    Ok(result)
}

fn validate_wal_link(record: &WalRecordView<'_>, previous_checksum: u64) -> ExecutionResult<()> {
    if record.header.previous_checksum == previous_checksum {
        Ok(())
    } else {
        Err(ExecutionError::Journal(format!(
            "WAL checksum link mismatch at sequence {}",
            record.header.sequence.0
        )))
    }
}

fn encode_command_payload(
    kind: JournalCommandKind,
    id: ClientOrderId,
    ts_ns: u64,
    out: &mut Vec<u8>,
) {
    put_payload_u16(out, WAL_PAYLOAD_VERSION);
    put_payload_u8(out, command_kind_u8(kind));
    put_payload_u8(out, 0);
    put_fixed(out, &id);
    put_payload_u64(out, ts_ns);
}

fn decode_command_payload(payload: &[u8]) -> ExecutionResult<JournalRecord> {
    let mut reader = PayloadReader::new(payload);
    reader.read_version()?;
    let kind = command_kind_from_u8(reader.read_u8()?)
        .ok_or_else(|| ExecutionError::Journal("invalid WAL command kind".to_string()))?;
    let _reserved = reader.read_u8()?;
    let client_order_id = reader.read_fixed::<40>()?;
    let ts_ns = reader.read_u64()?;
    reader.finish()?;
    Ok(JournalRecord::Command {
        kind,
        client_order_id,
        ts_ns,
    })
}

fn encode_event_payload(event: &ExecutionEvent, out: &mut Vec<u8>) {
    put_payload_u16(out, WAL_PAYLOAD_VERSION);
    put_payload_u8(out, event.exec_type as u8);
    put_payload_u8(out, event.order_status as u8);
    put_fixed(out, &event.client_order_id);
    put_fixed(out, &event.orig_client_order_id);
    put_fixed(out, &event.venue_order_id);
    put_fixed(out, &event.execution_id);
    put_fixed(out, &event.account_id);
    put_fixed(out, &event.route_id);
    put_fixed(out, &event.symbol.venue);
    put_fixed(out, &event.symbol.instrument);
    put_payload_i64(out, event.last_qty.0);
    put_payload_i64(out, event.last_price.0);
    put_payload_i64(out, event.cumulative_qty.0);
    put_payload_i64(out, event.leaves_qty.0);
    put_payload_i64(out, event.average_price.0);
    put_payload_u64(out, event.ts_exchange_ns);
    put_payload_u64(out, event.ts_recv_ns);
    put_payload_u8(out, event.reason as u8);
    put_fixed(out, &event.text);
}

fn decode_event_payload(payload: &[u8]) -> ExecutionResult<ExecutionEvent> {
    let mut reader = PayloadReader::new(payload);
    reader.read_version()?;
    let exec_type = execution_type_from_u8(reader.read_u8()?)?;
    let order_status = order_status_from_u8(reader.read_u8()?)?;
    let client_order_id = reader.read_fixed::<40>()?;
    let orig_client_order_id = reader.read_fixed::<40>()?;
    let venue_order_id = reader.read_fixed::<48>()?;
    let execution_id = reader.read_fixed::<48>()?;
    let account_id = reader.read_fixed::<32>()?;
    let route_id = reader.read_fixed::<32>()?;
    let venue = reader.read_fixed::<16>()?;
    let instrument = reader.read_fixed::<32>()?;
    let last_qty = OrderQty(reader.read_i64()?);
    let last_price = OrderPrice(reader.read_i64()?);
    let cumulative_qty = OrderQty(reader.read_i64()?);
    let leaves_qty = OrderQty(reader.read_i64()?);
    let average_price = OrderPrice(reader.read_i64()?);
    let ts_exchange_ns = reader.read_u64()?;
    let ts_recv_ns = reader.read_u64()?;
    let reason = risk_reason_from_u8(reader.read_u8()?)?;
    let text = reader.read_fixed::<128>()?;
    reader.finish()?;
    Ok(ExecutionEvent {
        exec_type,
        order_status,
        client_order_id,
        orig_client_order_id,
        venue_order_id,
        execution_id,
        account_id,
        route_id,
        symbol: ExecutionSymbol { venue, instrument },
        last_qty,
        last_price,
        cumulative_qty,
        leaves_qty,
        average_price,
        ts_exchange_ns,
        ts_recv_ns,
        reason,
        text,
    })
}

fn decode_wal_payload(record: &WalRecordView<'_>) -> ExecutionResult<Option<JournalRecord>> {
    match record.header.kind {
        WalRecordKind::CommandSubmit
        | WalRecordKind::CommandCancel
        | WalRecordKind::CommandAmend => Ok(Some(decode_command_payload(record.payload)?)),
        WalRecordKind::ExecutionEvent
        | WalRecordKind::RiskReject
        | WalRecordKind::RecoveryEvent => Ok(Some(JournalRecord::Event(Box::new(
            decode_event_payload(record.payload)?,
        )))),
        WalRecordKind::CheckpointMarker | WalRecordKind::SegmentSeal | WalRecordKind::Heartbeat => {
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn command_wal_kind(kind: JournalCommandKind) -> WalRecordKind {
    match kind {
        JournalCommandKind::Submit => WalRecordKind::CommandSubmit,
        JournalCommandKind::Cancel => WalRecordKind::CommandCancel,
        JournalCommandKind::Amend => WalRecordKind::CommandAmend,
    }
}

fn event_wal_kind(event: &ExecutionEvent) -> WalRecordKind {
    match event.exec_type {
        ExecutionType::Reject => WalRecordKind::RiskReject,
        ExecutionType::Restated => WalRecordKind::RecoveryEvent,
        _ => WalRecordKind::ExecutionEvent,
    }
}

fn is_risk_boundary_wal_kind(kind: WalRecordKind) -> bool {
    matches!(
        kind,
        WalRecordKind::CommandSubmit
            | WalRecordKind::CommandCancel
            | WalRecordKind::CommandAmend
            | WalRecordKind::RiskReject
            | WalRecordKind::RecoveryEvent
            | WalRecordKind::CheckpointMarker
            | WalRecordKind::SegmentSeal
    )
}

fn put_fixed<const N: usize>(out: &mut Vec<u8>, value: &FixedAscii<N>) {
    put_payload_u8(out, value.as_str().len() as u8);
    let start = out.len();
    out.resize(start + N, 0);
    out[start..start + value.as_str().len()].copy_from_slice(value.as_str().as_bytes());
}

fn put_payload_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn put_payload_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_payload_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_payload_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[derive(Debug, Clone, Copy)]
struct PayloadReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> PayloadReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_version(&mut self) -> ExecutionResult<()> {
        let version = self.read_u16()?;
        if version == WAL_PAYLOAD_VERSION {
            Ok(())
        } else {
            Err(ExecutionError::Journal(format!(
                "unsupported WAL payload version {version}"
            )))
        }
    }

    fn read_u8(&mut self) -> ExecutionResult<u8> {
        let bytes = self.take(1)?;
        Ok(bytes[0])
    }

    fn read_u16(&mut self) -> ExecutionResult<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> ExecutionResult<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes(
            bytes
                .try_into()
                .expect("payload reader returned four bytes"),
        ))
    }

    fn read_u64(&mut self) -> ExecutionResult<u64> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes(
            bytes
                .try_into()
                .expect("payload reader returned eight bytes"),
        ))
    }

    fn read_i64(&mut self) -> ExecutionResult<i64> {
        let bytes = self.take(8)?;
        Ok(i64::from_le_bytes(
            bytes
                .try_into()
                .expect("payload reader returned eight bytes"),
        ))
    }

    fn read_i128(&mut self) -> ExecutionResult<i128> {
        let bytes = self.take(16)?;
        Ok(i128::from_le_bytes(
            bytes
                .try_into()
                .expect("payload reader returned sixteen bytes"),
        ))
    }

    fn read_fixed<const N: usize>(&mut self) -> ExecutionResult<FixedAscii<N>> {
        let len = usize::from(self.read_u8()?);
        let bytes = self.take(N)?;
        if len > N {
            return Err(ExecutionError::Journal(
                "WAL fixed field length exceeds capacity".to_string(),
            ));
        }
        let value = std::str::from_utf8(&bytes[..len])
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        FixedAscii::new(value).map_err(|err| ExecutionError::Journal(err.to_string()))
    }

    fn finish(&self) -> ExecutionResult<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(ExecutionError::Journal(
                "trailing WAL payload bytes".to_string(),
            ))
        }
    }

    fn take(&mut self, len: usize) -> ExecutionResult<&'a [u8]> {
        let end = self.offset.saturating_add(len);
        if end > self.bytes.len() {
            return Err(ExecutionError::Journal("truncated WAL payload".to_string()));
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }
}

fn wal_error(err: of_execution_core::ExecutionWalError) -> ExecutionError {
    ExecutionError::Journal(err.to_string())
}

fn now_ns() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    u64::try_from(nanos).unwrap_or(u64::MAX)
}

const CHECKPOINT_MAGIC: u32 = 0x4b48_434f;
const CHECKPOINT_SCHEMA_VERSION: u16 = 1;
const CHECKPOINT_EXT: &str = "ofchk";

/// Snapshot of one position included in an execution checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CheckpointPosition {
    /// Position key.
    pub key: PositionKey,
    /// Position value.
    pub position: Position,
}

/// Versioned OMS checkpoint payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionCheckpoint {
    /// Checkpoint schema version.
    pub schema_version: u16,
    /// Caller-assigned checkpoint identifier.
    pub checkpoint_id: u64,
    /// Creation timestamp in nanoseconds.
    pub created_ns: u64,
    /// Last fully applied WAL sequence covered by this checkpoint.
    pub last_applied_sequence: WalSequence,
    /// Route/account/symbol configuration hash selected by the host.
    pub route_config_hash: u64,
    /// Open order states captured in this checkpoint.
    pub open_orders: Vec<OrderState>,
    /// Position snapshots captured in this checkpoint.
    pub positions: Vec<CheckpointPosition>,
    /// Kill-switch state at checkpoint time.
    pub kill_switch: bool,
    /// Deterministic checksum over the checkpoint payload.
    pub checksum: u64,
}

impl ExecutionCheckpoint {
    /// Creates an empty checkpoint.
    pub fn new(checkpoint_id: u64, last_applied_sequence: WalSequence, created_ns: u64) -> Self {
        let mut checkpoint = Self {
            schema_version: CHECKPOINT_SCHEMA_VERSION,
            checkpoint_id,
            created_ns,
            last_applied_sequence,
            route_config_hash: 0,
            open_orders: Vec::new(),
            positions: Vec::new(),
            kill_switch: false,
            checksum: 0,
        };
        checkpoint.refresh_checksum();
        checkpoint
    }

    /// Sets route configuration hash metadata.
    pub fn with_route_config_hash(mut self, route_config_hash: u64) -> Self {
        self.route_config_hash = route_config_hash;
        self.refresh_checksum();
        self
    }

    /// Sets open order states.
    pub fn with_open_orders(mut self, open_orders: Vec<OrderState>) -> Self {
        self.open_orders = open_orders;
        self.refresh_checksum();
        self
    }

    /// Sets position snapshots.
    pub fn with_positions(mut self, positions: Vec<CheckpointPosition>) -> Self {
        self.positions = positions;
        self.refresh_checksum();
        self
    }

    /// Sets kill-switch state.
    pub fn with_kill_switch(mut self, kill_switch: bool) -> Self {
        self.kill_switch = kill_switch;
        self.refresh_checksum();
        self
    }

    /// Recomputes and stores the checkpoint checksum.
    pub fn refresh_checksum(&mut self) {
        self.checksum = checkpoint_checksum(self);
    }

    /// Returns true when the stored checksum matches the checkpoint payload.
    pub fn validate_checksum(&self) -> bool {
        self.checksum == checkpoint_checksum(self)
    }
}

/// Checkpoint creation policy vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CheckpointPolicy {
    /// Caller explicitly decides when to save checkpoints.
    #[default]
    Manual,
    /// Save after every configured number of WAL records.
    EveryNWalRecords(u64),
    /// Save after the configured elapsed nanoseconds budget.
    EveryDurationNs(u64),
    /// Save after risk-sensitive transitions.
    AfterRiskBoundary,
    /// Save during clean shutdown.
    OnShutdown,
}

/// File-backed checkpoint store configuration.
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    root: PathBuf,
    sync_on_save: bool,
    max_retained: usize,
    policy: CheckpointPolicy,
}

impl CheckpointConfig {
    /// Creates checkpoint config rooted at `root`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            sync_on_save: true,
            max_retained: 8,
            policy: CheckpointPolicy::Manual,
        }
    }

    /// Sets whether checkpoint files are synced before atomic rename.
    pub fn with_sync_on_save(mut self, sync_on_save: bool) -> Self {
        self.sync_on_save = sync_on_save;
        self
    }

    /// Sets the maximum retained checkpoints used by checkpoint-store pruning.
    pub fn with_max_retained(mut self, max_retained: usize) -> Self {
        self.max_retained = max_retained;
        self
    }

    /// Sets checkpoint creation policy metadata.
    pub fn with_policy(mut self, policy: CheckpointPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Returns checkpoint root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns whether save operations sync checkpoint files.
    pub const fn sync_on_save(&self) -> bool {
        self.sync_on_save
    }

    /// Returns maximum retained checkpoints.
    pub const fn max_retained(&self) -> usize {
        self.max_retained
    }

    /// Returns configured checkpoint policy.
    pub const fn policy(&self) -> CheckpointPolicy {
        self.policy
    }
}

/// Metadata for one checkpoint file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CheckpointManifest {
    /// Checkpoint identifier.
    pub checkpoint_id: u64,
    /// Last WAL sequence covered by the checkpoint.
    pub last_applied_sequence: WalSequence,
    /// Checkpoint creation timestamp.
    pub created_ns: u64,
    /// Checkpoint file path.
    pub path: PathBuf,
    /// Encoded checkpoint bytes.
    pub bytes: u64,
    /// Checkpoint checksum.
    pub checksum: u64,
}

impl CheckpointManifest {
    fn from_checkpoint(path: PathBuf, bytes: u64, checkpoint: &ExecutionCheckpoint) -> Self {
        Self {
            checkpoint_id: checkpoint.checkpoint_id,
            last_applied_sequence: checkpoint.last_applied_sequence,
            created_ns: checkpoint.created_ns,
            path,
            bytes,
            checksum: checkpoint.checksum,
        }
    }
}

/// Execution checkpoint store contract.
pub trait ExecutionCheckpointStore: Send {
    /// Saves a checkpoint and returns installed file metadata.
    fn save_checkpoint(
        &mut self,
        checkpoint: &ExecutionCheckpoint,
    ) -> ExecutionResult<CheckpointManifest>;

    /// Loads the latest valid checkpoint, if any.
    fn load_latest(&self) -> ExecutionResult<Option<ExecutionCheckpoint>>;

    /// Lists valid checkpoints.
    fn list_checkpoints(&self) -> ExecutionResult<Vec<CheckpointManifest>>;

    /// Validates a checkpoint payload.
    fn validate_checkpoint(&self, checkpoint: &ExecutionCheckpoint) -> ExecutionResult<bool>;

    /// Prunes old checkpoints according to the store policy.
    fn prune_old(&mut self) -> ExecutionResult<usize>;
}

/// Atomic file-backed execution checkpoint store.
#[derive(Debug, Clone)]
pub struct FileExecutionCheckpointStore {
    config: CheckpointConfig,
}

impl FileExecutionCheckpointStore {
    /// Opens or creates a file-backed checkpoint store.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the checkpoint directory cannot
    /// be created.
    pub fn open(config: CheckpointConfig) -> ExecutionResult<Self> {
        fs::create_dir_all(config.root())
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        Ok(Self { config })
    }

    /// Returns checkpoint store config.
    pub const fn config(&self) -> &CheckpointConfig {
        &self.config
    }

    /// Builds a checkpoint path for `checkpoint`.
    pub fn checkpoint_path(&self, checkpoint: &ExecutionCheckpoint) -> PathBuf {
        self.config.root().join(format!(
            "checkpoint-{:020}-{:020}.{}",
            checkpoint.checkpoint_id, checkpoint.last_applied_sequence.0, CHECKPOINT_EXT
        ))
    }

    fn temp_path(&self, checkpoint: &ExecutionCheckpoint) -> PathBuf {
        self.config.root().join(format!(
            "checkpoint-{:020}-{:020}.{}.tmp",
            checkpoint.checkpoint_id, checkpoint.last_applied_sequence.0, CHECKPOINT_EXT
        ))
    }
}

impl ExecutionCheckpointStore for FileExecutionCheckpointStore {
    fn save_checkpoint(
        &mut self,
        checkpoint: &ExecutionCheckpoint,
    ) -> ExecutionResult<CheckpointManifest> {
        let mut checkpoint = checkpoint.clone();
        checkpoint.refresh_checksum();
        let bytes = encode_checkpoint(&checkpoint);
        let final_path = self.checkpoint_path(&checkpoint);
        let tmp_path = self.temp_path(&checkpoint);

        {
            let mut file =
                File::create(&tmp_path).map_err(|err| ExecutionError::Journal(err.to_string()))?;
            file.write_all(&bytes)
                .map_err(|err| ExecutionError::Journal(err.to_string()))?;
            file.flush()
                .map_err(|err| ExecutionError::Journal(err.to_string()))?;
            if self.config.sync_on_save() {
                file.sync_data()
                    .map_err(|err| ExecutionError::Journal(err.to_string()))?;
            }
        }

        fs::rename(&tmp_path, &final_path)
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        if self.config.sync_on_save() {
            sync_directory(self.config.root())?;
        }
        Ok(CheckpointManifest::from_checkpoint(
            final_path,
            bytes.len() as u64,
            &checkpoint,
        ))
    }

    fn load_latest(&self) -> ExecutionResult<Option<ExecutionCheckpoint>> {
        let mut manifests = self.list_checkpoints()?;
        manifests.sort_by_key(|manifest| {
            (
                manifest.last_applied_sequence,
                manifest.created_ns,
                manifest.checkpoint_id,
            )
        });
        manifests
            .last()
            .map(|manifest| load_checkpoint_file(&manifest.path))
            .transpose()
    }

    fn list_checkpoints(&self) -> ExecutionResult<Vec<CheckpointManifest>> {
        let mut manifests = Vec::new();
        if !self.config.root().exists() {
            return Ok(manifests);
        }

        for entry in fs::read_dir(self.config.root())
            .map_err(|err| ExecutionError::Journal(err.to_string()))?
        {
            let entry = entry.map_err(|err| ExecutionError::Journal(err.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some(CHECKPOINT_EXT) {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|err| ExecutionError::Journal(err.to_string()))?;
            let checkpoint = load_checkpoint_file(&path)?;
            manifests.push(CheckpointManifest::from_checkpoint(
                path,
                metadata.len(),
                &checkpoint,
            ));
        }
        manifests.sort_by_key(|manifest| {
            (
                manifest.last_applied_sequence,
                manifest.created_ns,
                manifest.checkpoint_id,
            )
        });
        Ok(manifests)
    }

    fn validate_checkpoint(&self, checkpoint: &ExecutionCheckpoint) -> ExecutionResult<bool> {
        Ok(
            checkpoint.schema_version == CHECKPOINT_SCHEMA_VERSION
                && checkpoint.validate_checksum(),
        )
    }

    fn prune_old(&mut self) -> ExecutionResult<usize> {
        let manifests = self.list_checkpoints()?;
        let retain = self.config.max_retained();
        if retain == 0 || manifests.len() <= retain {
            return Ok(0);
        }

        let prune_count = manifests.len() - retain;
        for manifest in manifests.iter().take(prune_count) {
            fs::remove_file(&manifest.path)
                .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        }
        if self.config.sync_on_save() {
            sync_directory(self.config.root())?;
        }
        Ok(prune_count)
    }
}

/// Recovery behavior when WAL replay encounters unusable data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum RecoveryCorruptionPolicy {
    /// Fail recovery on the first invalid, corrupt, or incomplete transition.
    #[default]
    FailClosed,
}

/// Venue reconciliation requirement selected for a recovery run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum RecoveryVenuePolicy {
    /// Require the host to reconcile against venue truth before submissions.
    #[default]
    RequireReconciliation,
    /// Let the host decide whether reconciliation is required.
    HostControlled,
}

/// Deterministic OMS recovery plan.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RecoveryPlan {
    replay_from: WalSequence,
    expected_latest_sequence: Option<WalSequence>,
    corruption_policy: RecoveryCorruptionPolicy,
    venue_policy: RecoveryVenuePolicy,
    submissions_disabled: bool,
}

impl RecoveryPlan {
    /// Creates a recovery plan that replays from `replay_from`.
    pub fn new(replay_from: WalSequence) -> Self {
        Self {
            replay_from,
            expected_latest_sequence: None,
            corruption_policy: RecoveryCorruptionPolicy::FailClosed,
            venue_policy: RecoveryVenuePolicy::RequireReconciliation,
            submissions_disabled: true,
        }
    }

    /// Creates a recovery plan that starts after `checkpoint`.
    pub fn from_checkpoint(checkpoint: &ExecutionCheckpoint) -> Self {
        Self::new(checkpoint.last_applied_sequence.next())
    }

    /// Sets the latest WAL sequence expected by the caller.
    pub fn with_expected_latest_sequence(
        mut self,
        expected_latest_sequence: Option<WalSequence>,
    ) -> Self {
        self.expected_latest_sequence = expected_latest_sequence;
        self
    }

    /// Sets the corruption policy.
    pub fn with_corruption_policy(mut self, corruption_policy: RecoveryCorruptionPolicy) -> Self {
        self.corruption_policy = corruption_policy;
        self
    }

    /// Sets the venue reconciliation policy.
    pub fn with_venue_policy(mut self, venue_policy: RecoveryVenuePolicy) -> Self {
        self.venue_policy = venue_policy;
        self
    }

    /// Sets whether strategy submissions stay disabled after recovery.
    pub fn with_submissions_disabled(mut self, submissions_disabled: bool) -> Self {
        self.submissions_disabled = submissions_disabled;
        self
    }

    /// Returns the first WAL sequence to replay.
    pub const fn replay_from(&self) -> WalSequence {
        self.replay_from
    }

    /// Returns the optional latest expected WAL sequence.
    pub const fn expected_latest_sequence(&self) -> Option<WalSequence> {
        self.expected_latest_sequence
    }

    /// Returns the corruption policy.
    pub const fn corruption_policy(&self) -> RecoveryCorruptionPolicy {
        self.corruption_policy
    }

    /// Returns the venue reconciliation policy.
    pub const fn venue_policy(&self) -> RecoveryVenuePolicy {
        self.venue_policy
    }

    /// Returns true when submissions should remain disabled after recovery.
    pub const fn submissions_disabled(&self) -> bool {
        self.submissions_disabled
    }
}

impl Default for RecoveryPlan {
    fn default() -> Self {
        Self::new(WalSequence(1))
    }
}

/// Recovered OMS state reconstructed from a checkpoint and WAL replay.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct RecoveredOmsState {
    checkpoint_id: Option<u64>,
    route_config_hash: u64,
    kill_switch: bool,
    orders: Vec<OrderState>,
    positions: Vec<CheckpointPosition>,
}

impl RecoveredOmsState {
    /// Creates an empty recovered state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates recovered state from a checkpoint.
    pub fn from_checkpoint(checkpoint: &ExecutionCheckpoint) -> Self {
        Self {
            checkpoint_id: Some(checkpoint.checkpoint_id),
            route_config_hash: checkpoint.route_config_hash,
            kill_switch: checkpoint.kill_switch,
            orders: checkpoint.open_orders.clone(),
            positions: checkpoint.positions.clone(),
        }
    }

    /// Returns the checkpoint id used for recovery, if any.
    pub const fn checkpoint_id(&self) -> Option<u64> {
        self.checkpoint_id
    }

    /// Returns the recovered route config hash.
    pub const fn route_config_hash(&self) -> u64 {
        self.route_config_hash
    }

    /// Returns whether the recovered kill switch is active.
    pub const fn kill_switch(&self) -> bool {
        self.kill_switch
    }

    /// Returns all recovered order states.
    pub fn orders(&self) -> &[OrderState] {
        &self.orders
    }

    /// Returns recovered non-terminal order states.
    pub fn open_orders(&self) -> Vec<OrderState> {
        self.orders
            .iter()
            .copied()
            .filter(|state| !state.status.is_terminal())
            .collect()
    }

    /// Returns recovered checkpoint positions.
    pub fn positions(&self) -> &[CheckpointPosition] {
        &self.positions
    }
}

/// Summary of one deterministic recovery run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RecoveryResult {
    /// Recovery plan used for the run.
    pub plan: RecoveryPlan,
    /// Recovered OMS state.
    pub state: RecoveredOmsState,
    /// WAL replay summary.
    pub replay: WalReplayResult,
    /// Number of command records observed during replay.
    pub commands_seen: usize,
    /// Number of execution events applied during replay.
    pub events_applied: usize,
    /// True when venue reconciliation must run before submissions resume.
    pub venue_reconciliation_required: bool,
    /// True when strategy submissions may resume after recovery.
    pub submissions_enabled: bool,
}

/// Recovers OMS state from already decoded journal records.
///
/// # Errors
///
/// Returns an execution journal error when replay contains an event for an
/// unknown order or an invalid state transition.
pub fn recover_oms_state_from_records(
    plan: RecoveryPlan,
    checkpoint: Option<&ExecutionCheckpoint>,
    records: &[JournalRecord],
) -> ExecutionResult<RecoveryResult> {
    let mut state = checkpoint
        .map(RecoveredOmsState::from_checkpoint)
        .unwrap_or_default();
    let mut commands_seen = 0_usize;
    let mut events_applied = 0_usize;

    for record in records {
        match record {
            JournalRecord::Command { .. } => {
                commands_seen = commands_seen.saturating_add(1);
            }
            JournalRecord::Event(event) => {
                apply_recovered_event(&mut state.orders, event)?;
                events_applied = events_applied.saturating_add(1);
            }
        }
    }

    let venue_reconciliation_required =
        plan.venue_policy() == RecoveryVenuePolicy::RequireReconciliation;
    let submissions_enabled =
        !plan.submissions_disabled() && plan.venue_policy() == RecoveryVenuePolicy::HostControlled;

    Ok(RecoveryResult {
        plan,
        state,
        replay: WalReplayResult {
            records: records.len(),
            bytes: 0,
            first_sequence: None,
            last_sequence: None,
        },
        commands_seen,
        events_applied,
        venue_reconciliation_required,
        submissions_enabled,
    })
}

/// Recovers OMS state from a segmented WAL and an optional checkpoint.
///
/// # Errors
///
/// Returns an execution journal error when WAL replay or state reconstruction
/// fails.
pub fn recover_oms_state_from_segmented_wal(
    plan: RecoveryPlan,
    checkpoint: Option<&ExecutionCheckpoint>,
    journal: &SegmentedWalExecutionJournal,
) -> ExecutionResult<RecoveryResult> {
    let mut records = Vec::new();
    let replay = journal.replay_from(plan.replay_from(), &mut records)?;
    let mut result = recover_oms_state_from_records(plan, checkpoint, &records)?;
    if let Some(expected) = result.plan.expected_latest_sequence() {
        let actual = replay
            .last_sequence
            .or_else(|| checkpoint.map(|checkpoint| checkpoint.last_applied_sequence));
        if actual != Some(expected) {
            return Err(ExecutionError::Journal(format!(
                "recovery latest sequence mismatch: expected {}, actual {}",
                expected.0,
                actual.map_or(0, |sequence| sequence.0)
            )));
        }
    }
    result.replay = replay;
    Ok(result)
}

/// Loads the latest checkpoint and recovers state from a segmented WAL.
///
/// # Errors
///
/// Returns an execution journal error when checkpoint loading, WAL replay, or
/// state reconstruction fails.
pub fn recover_latest_checkpoint_from_segmented_wal<S>(
    store: &S,
    journal: &SegmentedWalExecutionJournal,
) -> ExecutionResult<RecoveryResult>
where
    S: ExecutionCheckpointStore + ?Sized,
{
    let checkpoint = store.load_latest()?;
    let plan = checkpoint
        .as_ref()
        .map(RecoveryPlan::from_checkpoint)
        .unwrap_or_default();
    recover_oms_state_from_segmented_wal(plan, checkpoint.as_ref(), journal)
}

fn apply_recovered_event(
    orders: &mut Vec<OrderState>,
    event: &ExecutionEvent,
) -> ExecutionResult<()> {
    let key = recovery_event_key(event);
    let Some(index) = orders.iter().position(|state| state.client_order_id == key) else {
        return Err(ExecutionError::Journal(format!(
            "recovery event references unknown order {}",
            key.as_str()
        )));
    };

    let mut state = orders[index];
    apply_recovered_state_transition(&mut state, event)?;
    if event.exec_type == ExecutionType::ReplaceAck {
        orders.remove(index);
        orders.push(state);
    } else {
        orders[index] = state;
    }
    Ok(())
}

fn recovery_event_key(event: &ExecutionEvent) -> ClientOrderId {
    if !event.orig_client_order_id.is_empty()
        && matches!(
            event.exec_type,
            ExecutionType::CancelPending
                | ExecutionType::CancelAck
                | ExecutionType::CancelReject
                | ExecutionType::ReplacePending
                | ExecutionType::ReplaceAck
                | ExecutionType::ReplaceReject
        )
    {
        event.orig_client_order_id
    } else {
        event.client_order_id
    }
}

fn apply_recovered_state_transition(
    state: &mut OrderState,
    event: &ExecutionEvent,
) -> ExecutionResult<()> {
    match event.exec_type {
        ExecutionType::Ack => {
            if state.status != OrderStatus::PendingNew {
                return Err(ExecutionError::Core(ExecutionCoreError::InvalidTransition));
            }
            state.status = OrderStatus::New;
            state.venue_order_id = event.venue_order_id;
            state.leaves_qty = event.leaves_qty;
        }
        ExecutionType::Trade => {
            if event.cumulative_qty.0 > state.order_qty.0 {
                return Err(ExecutionError::Core(ExecutionCoreError::InvalidTransition));
            }
            state.cumulative_qty = event.cumulative_qty;
            state.leaves_qty = event.leaves_qty;
            state.average_price = event.average_price;
            state.status = if event.leaves_qty.0 == 0 {
                OrderStatus::Filled
            } else {
                OrderStatus::PartiallyFilled
            };
        }
        ExecutionType::CancelPending => state.status = OrderStatus::PendingCancel,
        ExecutionType::CancelAck => {
            state.status = OrderStatus::Cancelled;
            state.cumulative_qty = event.cumulative_qty;
            state.leaves_qty = event.leaves_qty;
            state.average_price = event.average_price;
        }
        ExecutionType::ReplacePending => state.status = OrderStatus::PendingReplace,
        ExecutionType::ReplaceAck => {
            state.client_order_id = event.client_order_id;
            state.last_accepted_client_order_id = event.client_order_id;
            state.status = OrderStatus::Replaced;
            state.order_qty = OrderQty(event.cumulative_qty.0 + event.leaves_qty.0);
            state.cumulative_qty = event.cumulative_qty;
            state.leaves_qty = event.leaves_qty;
            state.average_price = event.average_price;
        }
        ExecutionType::Reject
        | ExecutionType::Expire
        | ExecutionType::CancelReject
        | ExecutionType::ReplaceReject
        | ExecutionType::Status
        | ExecutionType::Restated
        | ExecutionType::AdapterDegraded => {
            if event.order_status != OrderStatus::Unknown {
                state.status = event.order_status;
            }
            state.cumulative_qty = event.cumulative_qty;
            state.leaves_qty = event.leaves_qty;
            state.average_price = event.average_price;
        }
    }
    state.updated_ns = event.ts_recv_ns;
    Ok(())
}

fn encode_checkpoint(checkpoint: &ExecutionCheckpoint) -> Vec<u8> {
    let mut out = Vec::with_capacity(128 + checkpoint.open_orders.len() * 320);
    put_payload_u32(&mut out, CHECKPOINT_MAGIC);
    put_payload_u16(&mut out, checkpoint.schema_version);
    put_payload_u16(&mut out, 0);
    put_payload_u64(&mut out, checkpoint.checkpoint_id);
    put_payload_u64(&mut out, checkpoint.created_ns);
    put_payload_u64(&mut out, checkpoint.last_applied_sequence.0);
    put_payload_u64(&mut out, checkpoint.route_config_hash);
    put_payload_u8(&mut out, u8::from(checkpoint.kill_switch));
    put_payload_u32(&mut out, checkpoint.open_orders.len() as u32);
    put_payload_u32(&mut out, checkpoint.positions.len() as u32);
    for state in &checkpoint.open_orders {
        encode_order_state(state, &mut out);
    }
    for position in &checkpoint.positions {
        encode_checkpoint_position(position, &mut out);
    }
    put_payload_u64(&mut out, checkpoint.checksum);
    out
}

fn decode_checkpoint(bytes: &[u8]) -> ExecutionResult<ExecutionCheckpoint> {
    let mut reader = PayloadReader::new(bytes);
    let magic = reader.read_u32()?;
    if magic != CHECKPOINT_MAGIC {
        return Err(ExecutionError::Journal(
            "invalid checkpoint magic".to_string(),
        ));
    }
    let schema_version = reader.read_u16()?;
    if schema_version != CHECKPOINT_SCHEMA_VERSION {
        return Err(ExecutionError::Journal(format!(
            "unsupported checkpoint schema version {schema_version}"
        )));
    }
    let _reserved = reader.read_u16()?;
    let checkpoint_id = reader.read_u64()?;
    let created_ns = reader.read_u64()?;
    let last_applied_sequence = WalSequence(reader.read_u64()?);
    let route_config_hash = reader.read_u64()?;
    let kill_switch = reader.read_u8()? != 0;
    let order_count = reader.read_u32()? as usize;
    let position_count = reader.read_u32()? as usize;
    let mut open_orders = Vec::with_capacity(order_count);
    for _ in 0..order_count {
        open_orders.push(decode_order_state(&mut reader)?);
    }
    let mut positions = Vec::with_capacity(position_count);
    for _ in 0..position_count {
        positions.push(decode_checkpoint_position(&mut reader)?);
    }
    let checksum = reader.read_u64()?;
    reader.finish()?;

    let checkpoint = ExecutionCheckpoint {
        schema_version,
        checkpoint_id,
        created_ns,
        last_applied_sequence,
        route_config_hash,
        open_orders,
        positions,
        kill_switch,
        checksum,
    };
    if checkpoint.validate_checksum() {
        Ok(checkpoint)
    } else {
        Err(ExecutionError::Journal(
            "checkpoint checksum mismatch".to_string(),
        ))
    }
}

fn load_checkpoint_file(path: &Path) -> ExecutionResult<ExecutionCheckpoint> {
    let bytes = fs::read(path).map_err(|err| ExecutionError::Journal(err.to_string()))?;
    decode_checkpoint(&bytes)
}

fn checkpoint_checksum(checkpoint: &ExecutionCheckpoint) -> u64 {
    let mut cloned = checkpoint.clone();
    cloned.checksum = 0;
    let bytes = encode_checkpoint(&cloned);
    execution_wal_checksum(&bytes)
}

fn encode_order_state(state: &OrderState, out: &mut Vec<u8>) {
    put_fixed(out, &state.client_order_id);
    put_fixed(out, &state.last_accepted_client_order_id);
    put_fixed(out, &state.venue_order_id);
    put_fixed(out, &state.account_id);
    put_fixed(out, &state.route_id);
    put_fixed(out, &state.symbol.venue);
    put_fixed(out, &state.symbol.instrument);
    put_payload_u8(out, state.side as u8);
    put_payload_u8(out, state.status as u8);
    put_payload_i64(out, state.order_qty.0);
    put_payload_i64(out, state.cumulative_qty.0);
    put_payload_i64(out, state.leaves_qty.0);
    put_payload_i64(out, state.average_price.0);
    put_payload_u64(out, state.updated_ns);
}

fn decode_order_state(reader: &mut PayloadReader<'_>) -> ExecutionResult<OrderState> {
    let client_order_id = reader.read_fixed::<40>()?;
    let last_accepted_client_order_id = reader.read_fixed::<40>()?;
    let venue_order_id = reader.read_fixed::<48>()?;
    let account_id = reader.read_fixed::<32>()?;
    let route_id = reader.read_fixed::<32>()?;
    let venue = reader.read_fixed::<16>()?;
    let instrument = reader.read_fixed::<32>()?;
    let side = order_side_from_u8(reader.read_u8()?)?;
    let status = order_status_from_u8(reader.read_u8()?)?;
    Ok(OrderState {
        client_order_id,
        last_accepted_client_order_id,
        venue_order_id,
        account_id,
        route_id,
        symbol: ExecutionSymbol { venue, instrument },
        side,
        status,
        order_qty: OrderQty(reader.read_i64()?),
        cumulative_qty: OrderQty(reader.read_i64()?),
        leaves_qty: OrderQty(reader.read_i64()?),
        average_price: OrderPrice(reader.read_i64()?),
        updated_ns: reader.read_u64()?,
    })
}

fn encode_checkpoint_position(position: &CheckpointPosition, out: &mut Vec<u8>) {
    put_fixed(out, &position.key.account_id);
    put_fixed(out, &position.key.strategy_id);
    put_fixed(out, &position.key.symbol.venue);
    put_fixed(out, &position.key.symbol.instrument);
    put_payload_i64(out, position.position.net_qty);
    put_payload_i64(out, position.position.buy_qty);
    put_payload_i64(out, position.position.sell_qty);
    put_payload_i128(out, position.position.gross_notional);
    put_payload_i64(out, position.position.average_price);
}

fn decode_checkpoint_position(
    reader: &mut PayloadReader<'_>,
) -> ExecutionResult<CheckpointPosition> {
    let account_id = reader.read_fixed::<32>()?;
    let strategy_id = reader.read_fixed::<32>()?;
    let venue = reader.read_fixed::<16>()?;
    let instrument = reader.read_fixed::<32>()?;
    Ok(CheckpointPosition {
        key: PositionKey {
            account_id,
            strategy_id,
            symbol: ExecutionSymbol { venue, instrument },
        },
        position: Position {
            net_qty: reader.read_i64()?,
            buy_qty: reader.read_i64()?,
            sell_qty: reader.read_i64()?,
            gross_notional: reader.read_i128()?,
            average_price: reader.read_i64()?,
        },
    })
}

fn order_side_from_u8(value: u8) -> ExecutionResult<OrderSide> {
    match value {
        1 => Ok(OrderSide::Buy),
        2 => Ok(OrderSide::Sell),
        _ => Err(ExecutionError::Journal("invalid order side".to_string())),
    }
}

fn put_payload_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_payload_i128(out: &mut Vec<u8>, value: i128) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn sync_directory(path: &Path) -> ExecutionResult<()> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// Open-order reconciliation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationAction {
    /// Local and venue state match.
    Matched,
    /// Venue has an order not present locally.
    VenueOnly,
    /// Local has an order not present at the venue.
    LocalOnly,
    /// Local state should be restated from venue state.
    RestateFromVenue,
}

/// One reconciliation difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconciliationItem {
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Reconciliation action.
    pub action: ReconciliationAction,
    /// Local state when present.
    pub local: Option<OrderState>,
    /// Venue state when present.
    pub venue: Option<OrderState>,
}

/// Open-order reconciliation report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReconciliationReport {
    /// Reconciliation items.
    pub items: Vec<ReconciliationItem>,
}

impl ReconciliationReport {
    /// Returns true when no local/venue differences were found.
    pub fn is_clean(&self) -> bool {
        self.items
            .iter()
            .all(|item| item.action == ReconciliationAction::Matched)
    }
}

/// Fine-grained reconciliation issue classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReconciliationIssueKind {
    /// Local and venue state match.
    Matched,
    /// Venue has an order not present locally.
    VenueOnly,
    /// Local has an order not present at the venue.
    LocalOnly,
    /// Cumulative, leaves, or original quantity differs.
    QuantityMismatch,
    /// Local and venue lifecycle statuses differ.
    StatusMismatch,
    /// Average execution price differs.
    PriceMismatch,
    /// The discrepancy does not fit a more specific category.
    Unknown,
}

/// One detailed reconciliation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReconciliationDetail {
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Fine-grained issue kind.
    pub issue: ReconciliationIssueKind,
    /// Local state when present.
    pub local: Option<OrderState>,
    /// Venue state when present.
    pub venue: Option<OrderState>,
}

/// Detailed venue reconciliation report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct VenueReconciliationReport {
    /// Detailed reconciliation findings.
    pub details: Vec<ReconciliationDetail>,
}

impl VenueReconciliationReport {
    /// Returns true when all details are matched.
    pub fn is_clean(&self) -> bool {
        self.details
            .iter()
            .all(|detail| detail.issue == ReconciliationIssueKind::Matched)
    }

    /// Returns true when at least one detail requires reconciliation action.
    pub fn has_discrepancies(&self) -> bool {
        !self.is_clean()
    }
}

/// Host action selected for a reconciliation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ReconciliationPolicyAction {
    /// No action is required.
    #[default]
    Noop,
    /// Stop recovery or live submissions until a human or host system resolves it.
    FailClosed,
    /// Accept venue state as truth and restate local cache.
    AcceptVenueTruth,
    /// Cancel the venue-only order before resuming submissions.
    CancelVenueOrder,
    /// Restate a venue-only order locally before resuming submissions.
    RestateVenueOrder,
    /// Require explicit operator approval.
    RequireOperatorApproval,
}

impl ReconciliationPolicyAction {
    /// Returns true when the action blocks immediate strategy submissions.
    pub const fn blocks_submissions(self) -> bool {
        !matches!(self, Self::Noop)
    }
}

/// Policy for mapping reconciliation issues to host actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ReconciliationPolicy {
    venue_only: ReconciliationPolicyAction,
    local_only: ReconciliationPolicyAction,
    quantity_mismatch: ReconciliationPolicyAction,
    status_mismatch: ReconciliationPolicyAction,
    price_mismatch: ReconciliationPolicyAction,
    unknown: ReconciliationPolicyAction,
}

impl ReconciliationPolicy {
    /// Creates a fail-closed reconciliation policy.
    pub const fn fail_closed() -> Self {
        Self {
            venue_only: ReconciliationPolicyAction::FailClosed,
            local_only: ReconciliationPolicyAction::FailClosed,
            quantity_mismatch: ReconciliationPolicyAction::FailClosed,
            status_mismatch: ReconciliationPolicyAction::FailClosed,
            price_mismatch: ReconciliationPolicyAction::FailClosed,
            unknown: ReconciliationPolicyAction::FailClosed,
        }
    }

    /// Creates an operator-approval reconciliation policy.
    pub const fn require_operator_approval() -> Self {
        Self {
            venue_only: ReconciliationPolicyAction::RequireOperatorApproval,
            local_only: ReconciliationPolicyAction::RequireOperatorApproval,
            quantity_mismatch: ReconciliationPolicyAction::RequireOperatorApproval,
            status_mismatch: ReconciliationPolicyAction::RequireOperatorApproval,
            price_mismatch: ReconciliationPolicyAction::RequireOperatorApproval,
            unknown: ReconciliationPolicyAction::RequireOperatorApproval,
        }
    }

    /// Sets the action for venue-only orders.
    pub const fn with_venue_only(mut self, action: ReconciliationPolicyAction) -> Self {
        self.venue_only = action;
        self
    }

    /// Sets the action for local-only orders.
    pub const fn with_local_only(mut self, action: ReconciliationPolicyAction) -> Self {
        self.local_only = action;
        self
    }

    /// Sets the action for quantity mismatches.
    pub const fn with_quantity_mismatch(mut self, action: ReconciliationPolicyAction) -> Self {
        self.quantity_mismatch = action;
        self
    }

    /// Sets the action for status mismatches.
    pub const fn with_status_mismatch(mut self, action: ReconciliationPolicyAction) -> Self {
        self.status_mismatch = action;
        self
    }

    /// Sets the action for price mismatches.
    pub const fn with_price_mismatch(mut self, action: ReconciliationPolicyAction) -> Self {
        self.price_mismatch = action;
        self
    }

    /// Sets the action for unknown mismatches.
    pub const fn with_unknown(mut self, action: ReconciliationPolicyAction) -> Self {
        self.unknown = action;
        self
    }

    /// Returns the action for `issue`.
    pub const fn action_for(self, issue: ReconciliationIssueKind) -> ReconciliationPolicyAction {
        match issue {
            ReconciliationIssueKind::Matched => ReconciliationPolicyAction::Noop,
            ReconciliationIssueKind::VenueOnly => self.venue_only,
            ReconciliationIssueKind::LocalOnly => self.local_only,
            ReconciliationIssueKind::QuantityMismatch => self.quantity_mismatch,
            ReconciliationIssueKind::StatusMismatch => self.status_mismatch,
            ReconciliationIssueKind::PriceMismatch => self.price_mismatch,
            ReconciliationIssueKind::Unknown => self.unknown,
        }
    }
}

impl Default for ReconciliationPolicy {
    fn default() -> Self {
        Self::fail_closed()
    }
}

/// Policy decision for one reconciliation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReconciliationPolicyItem {
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Issue found during reconciliation.
    pub issue: ReconciliationIssueKind,
    /// Action selected by policy.
    pub action: ReconciliationPolicyAction,
    /// Local state when present.
    pub local: Option<OrderState>,
    /// Venue state when present.
    pub venue: Option<OrderState>,
}

/// Aggregate policy decision for a reconciliation report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ReconciliationPolicyDecision {
    /// Per-order policy decisions.
    pub items: Vec<ReconciliationPolicyItem>,
    /// True when submissions may resume immediately.
    pub submissions_enabled: bool,
    /// True when at least one action fails closed.
    pub fail_closed: bool,
    /// True when at least one action requires operator approval.
    pub operator_approval_required: bool,
    /// True when at least one venue order should be cancelled.
    pub venue_cancels_required: bool,
    /// True when local state must be restated from venue truth.
    pub local_restates_required: bool,
}

/// Compares local open-order state against venue open-order state with
/// fine-grained discrepancy classification.
pub fn reconcile_open_orders_detailed(
    local: &[OrderState],
    venue: &[OrderState],
) -> VenueReconciliationReport {
    let mut report = VenueReconciliationReport::default();
    let mut venue_by_id: HashMap<ClientOrderId, OrderState> = HashMap::with_capacity(venue.len());
    for state in venue {
        venue_by_id.insert(state.client_order_id, *state);
    }

    for local_state in local {
        match venue_by_id.remove(&local_state.client_order_id) {
            Some(venue_state) => {
                report.details.push(ReconciliationDetail {
                    client_order_id: local_state.client_order_id,
                    issue: classify_reconciliation_issue(local_state, &venue_state),
                    local: Some(*local_state),
                    venue: Some(venue_state),
                });
            }
            None => report.details.push(ReconciliationDetail {
                client_order_id: local_state.client_order_id,
                issue: ReconciliationIssueKind::LocalOnly,
                local: Some(*local_state),
                venue: None,
            }),
        }
    }

    for venue_state in venue_by_id.into_values() {
        report.details.push(ReconciliationDetail {
            client_order_id: venue_state.client_order_id,
            issue: ReconciliationIssueKind::VenueOnly,
            local: None,
            venue: Some(venue_state),
        });
    }
    report
}

/// Evaluates a detailed reconciliation report against a host policy.
pub fn evaluate_reconciliation_policy(
    report: &VenueReconciliationReport,
    policy: ReconciliationPolicy,
) -> ReconciliationPolicyDecision {
    let mut decision = ReconciliationPolicyDecision {
        submissions_enabled: true,
        ..ReconciliationPolicyDecision::default()
    };

    for detail in &report.details {
        let action = policy.action_for(detail.issue);
        decision.fail_closed |= action == ReconciliationPolicyAction::FailClosed;
        decision.operator_approval_required |=
            action == ReconciliationPolicyAction::RequireOperatorApproval;
        decision.venue_cancels_required |= action == ReconciliationPolicyAction::CancelVenueOrder;
        decision.local_restates_required |= matches!(
            action,
            ReconciliationPolicyAction::AcceptVenueTruth
                | ReconciliationPolicyAction::RestateVenueOrder
        );
        if action.blocks_submissions() {
            decision.submissions_enabled = false;
        }
        decision.items.push(ReconciliationPolicyItem {
            client_order_id: detail.client_order_id,
            issue: detail.issue,
            action,
            local: detail.local,
            venue: detail.venue,
        });
    }

    decision
}

/// Reconciles local open-order state against venue state.
pub fn reconcile_open_orders(local: &[OrderState], venue: &[OrderState]) -> ReconciliationReport {
    let mut report = ReconciliationReport::default();
    let mut venue_by_id: HashMap<ClientOrderId, OrderState> = HashMap::with_capacity(venue.len());
    for state in venue {
        venue_by_id.insert(state.client_order_id, *state);
    }

    for local_state in local {
        match venue_by_id.remove(&local_state.client_order_id) {
            Some(venue_state) if venue_state == *local_state => {
                report.items.push(ReconciliationItem {
                    client_order_id: local_state.client_order_id,
                    action: ReconciliationAction::Matched,
                    local: Some(*local_state),
                    venue: Some(venue_state),
                })
            }
            Some(venue_state) => report.items.push(ReconciliationItem {
                client_order_id: local_state.client_order_id,
                action: ReconciliationAction::RestateFromVenue,
                local: Some(*local_state),
                venue: Some(venue_state),
            }),
            None => report.items.push(ReconciliationItem {
                client_order_id: local_state.client_order_id,
                action: ReconciliationAction::LocalOnly,
                local: Some(*local_state),
                venue: None,
            }),
        }
    }

    for venue_state in venue_by_id.into_values() {
        report.items.push(ReconciliationItem {
            client_order_id: venue_state.client_order_id,
            action: ReconciliationAction::VenueOnly,
            local: None,
            venue: Some(venue_state),
        });
    }
    report
}

fn classify_reconciliation_issue(
    local: &OrderState,
    venue: &OrderState,
) -> ReconciliationIssueKind {
    if local == venue {
        return ReconciliationIssueKind::Matched;
    }
    if local.account_id != venue.account_id
        || local.route_id != venue.route_id
        || local.symbol != venue.symbol
        || local.side != venue.side
    {
        return ReconciliationIssueKind::Unknown;
    }
    if local.status != venue.status {
        return ReconciliationIssueKind::StatusMismatch;
    }
    if local.order_qty != venue.order_qty
        || local.cumulative_qty != venue.cumulative_qty
        || local.leaves_qty != venue.leaves_qty
    {
        return ReconciliationIssueKind::QuantityMismatch;
    }
    if local.average_price != venue.average_price {
        return ReconciliationIssueKind::PriceMismatch;
    }
    ReconciliationIssueKind::Unknown
}

/// Route safety behavior during disconnects and kill switches.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisconnectPolicy {
    /// Leave working orders untouched.
    Hold = 0,
    /// Reject new orders while allowing cancels.
    RejectNew = 1,
    /// Cancel open orders on disconnect.
    CancelOpenOrders = 2,
    /// Reject all order commands.
    Freeze = 3,
}

/// Safety policy for one route scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteSafetyPolicy {
    /// Route key.
    pub route: RouteKey,
    /// Disconnect behavior.
    pub disconnect_policy: DisconnectPolicy,
    /// Non-zero rejects new order flow.
    pub kill_switch: bool,
    /// Non-zero allows cancel commands while killed/frozen.
    pub allow_cancels_when_killed: bool,
}

impl RouteSafetyPolicy {
    /// Returns true when a new order should be rejected.
    pub const fn reject_new(self, disconnected: bool) -> bool {
        self.kill_switch
            || matches!(self.disconnect_policy, DisconnectPolicy::Freeze)
            || (disconnected
                && matches!(
                    self.disconnect_policy,
                    DisconnectPolicy::RejectNew | DisconnectPolicy::CancelOpenOrders
                ))
    }

    /// Returns true when a cancel command should be allowed.
    pub const fn allow_cancel(self) -> bool {
        !self.kill_switch
            || self.allow_cancels_when_killed
            || matches!(self.disconnect_policy, DisconnectPolicy::RejectNew)
    }
}

/// Advanced additive risk limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AdvancedRiskLimits {
    /// Basic route limits.
    pub basic: RiskLimits,
    /// Maximum messages per one-second window. Zero disables.
    pub max_message_rate_per_sec: u32,
    /// Maximum absolute net position. Zero disables.
    pub max_position_abs: i64,
    /// Maximum gross notional. Zero disables.
    pub max_gross_notional: i128,
    /// Reduce-only mode rejects orders that increase exposure.
    pub reduce_only: bool,
}

#[derive(Debug, Default)]
struct MessageRateWindow {
    timestamps_ns: VecDeque<u64>,
}

impl MessageRateWindow {
    fn allow(&mut self, ts_recv_ns: u64, max_rate: u32) -> bool {
        if max_rate == 0 {
            return true;
        }
        let cutoff = ts_recv_ns.saturating_sub(1_000_000_000);
        while self
            .timestamps_ns
            .front()
            .is_some_and(|timestamp| *timestamp <= cutoff)
        {
            self.timestamps_ns.pop_front();
        }
        if self.timestamps_ns.len() >= max_rate as usize {
            return false;
        }
        self.timestamps_ns.push_back(ts_recv_ns);
        true
    }
}

/// Advanced risk gate with basic limits plus message-rate checks.
#[derive(Debug)]
pub struct AdvancedRiskGate {
    limits: AdvancedRiskLimits,
    window: Mutex<MessageRateWindow>,
}

impl AdvancedRiskGate {
    /// Creates an advanced risk gate.
    pub fn new(limits: AdvancedRiskLimits) -> Self {
        Self {
            limits,
            window: Mutex::new(MessageRateWindow {
                timestamps_ns: VecDeque::new(),
            }),
        }
    }

    fn check_common(&self, ctx: &RiskContext, ts_recv_ns: u64) -> RiskDecision {
        if self.limits.basic.kill_switch {
            return reject(RiskRejectReason::KillSwitch, "kill switch active");
        }
        if ctx.duplicate_client_order_id {
            return reject(
                RiskRejectReason::DuplicateClientOrderId,
                "duplicate client order id",
            );
        }
        if !self
            .window
            .lock()
            .expect("risk window mutex")
            .allow(ts_recv_ns, self.limits.max_message_rate_per_sec)
        {
            return reject(RiskRejectReason::MaxOpenOrders, "message rate exceeded");
        }
        RiskDecision::allow()
    }
}

impl RiskCheck for AdvancedRiskGate {
    fn check_new(&self, req: &OrderRequest, ctx: &RiskContext) -> RiskDecision {
        let common = self.check_common(ctx, req.ts_recv_ns);
        if !common.allowed {
            return common;
        }
        if self.limits.basic.max_order_qty > 0 && req.quantity.0 > self.limits.basic.max_order_qty {
            return reject(RiskRejectReason::MaxOrderQty, "max order quantity exceeded");
        }
        let notional = i128::from(req.quantity.0).saturating_mul(i128::from(req.limit_price.0));
        if self.limits.basic.max_order_notional > 0
            && notional > self.limits.basic.max_order_notional
        {
            return reject(
                RiskRejectReason::MaxOrderNotional,
                "max order notional exceeded",
            );
        }
        if self.limits.max_gross_notional > 0
            && ctx.open_notional.saturating_add(notional) > self.limits.max_gross_notional
        {
            return reject(
                RiskRejectReason::MaxOpenNotional,
                "max gross notional exceeded",
            );
        }
        RiskDecision::allow()
    }

    fn check_amend(&self, req: &AmendRequest, ctx: &RiskContext) -> RiskDecision {
        let common = self.check_common(ctx, req.ts_recv_ns);
        if !common.allowed {
            return common;
        }
        if self.limits.basic.max_order_qty > 0 && req.quantity.0 > self.limits.basic.max_order_qty {
            return reject(RiskRejectReason::MaxOrderQty, "max order quantity exceeded");
        }
        RiskDecision::allow()
    }

    fn check_cancel(&self, req: &CancelRequest, ctx: &RiskContext) -> RiskDecision {
        self.check_common(ctx, req.ts_recv_ns)
    }
}

/// Position for one account/strategy/symbol scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    /// Net signed quantity.
    pub net_qty: i64,
    /// Buy quantity.
    pub buy_qty: i64,
    /// Sell quantity.
    pub sell_qty: i64,
    /// Gross traded notional.
    pub gross_notional: i128,
    /// Average price of the current net position.
    pub average_price: i64,
}

/// Position key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionKey {
    /// Account id.
    pub account_id: AccountId,
    /// Strategy id.
    pub strategy_id: StrategyId,
    /// Symbol.
    pub symbol: ExecutionSymbol,
}

/// Fill and position ledger.
#[derive(Debug, Default, Clone)]
pub struct PositionLedger {
    positions: HashMap<PositionKey, Position>,
}

impl PositionLedger {
    /// Creates an empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies a trade execution report to the ledger.
    pub fn apply_fill(&mut self, event: &ExecutionEvent, strategy_id: StrategyId, side: OrderSide) {
        if event.exec_type != ExecutionType::Trade || event.last_qty.0 <= 0 {
            return;
        }
        let key = PositionKey {
            account_id: event.account_id,
            strategy_id,
            symbol: event.symbol,
        };
        let position = self.positions.entry(key).or_default();
        let signed = match side {
            OrderSide::Buy => event.last_qty.0,
            OrderSide::Sell => -event.last_qty.0,
        };
        position.net_qty = position.net_qty.saturating_add(signed);
        if side == OrderSide::Buy {
            position.buy_qty = position.buy_qty.saturating_add(event.last_qty.0);
        } else {
            position.sell_qty = position.sell_qty.saturating_add(event.last_qty.0);
        }
        let fill_notional =
            i128::from(event.last_qty.0).saturating_mul(i128::from(event.last_price.0));
        position.gross_notional = position.gross_notional.saturating_add(fill_notional);
        if position.net_qty != 0 {
            position.average_price =
                (position.gross_notional / i128::from(position.net_qty.abs())) as i64;
        } else {
            position.average_price = 0;
        }
    }

    /// Returns a position by key.
    pub fn position(&self, key: &PositionKey) -> Option<Position> {
        self.positions.get(key).copied()
    }

    /// Returns all positions.
    pub fn positions(&self) -> &HashMap<PositionKey, Position> {
        &self.positions
    }
}

/// Venue-specific order type and TIF capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueOrderCapabilities {
    /// Market orders are supported.
    pub market: bool,
    /// Limit orders are supported.
    pub limit: bool,
    /// Stop orders are supported.
    pub stop: bool,
    /// Stop-limit orders are supported.
    pub stop_limit: bool,
    /// Day orders are supported.
    pub tif_day: bool,
    /// GTC orders are supported.
    pub tif_gtc: bool,
    /// IOC orders are supported.
    pub tif_ioc: bool,
    /// FOK orders are supported.
    pub tif_fok: bool,
    /// GTD orders are supported.
    pub tif_gtd: bool,
}

impl From<ExecutionCapabilities> for VenueOrderCapabilities {
    fn from(value: ExecutionCapabilities) -> Self {
        Self {
            market: value.market,
            limit: value.limit,
            stop: value.stop,
            stop_limit: value.stop_limit,
            tif_day: value.tif_day,
            tif_gtc: value.tif_gtc,
            tif_ioc: value.tif_ioc,
            tif_fok: value.tif_fok,
            tif_gtd: value.tif_gtd,
        }
    }
}

/// Normalized venue order encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizedOrderType {
    /// Canonical order type.
    pub order_type: OrderType,
    /// Canonical time-in-force.
    pub time_in_force: TimeInForce,
}

/// Validates and normalizes order type/TIF against venue capabilities.
pub fn normalize_order_type(
    order_type: OrderType,
    time_in_force: TimeInForce,
    capabilities: VenueOrderCapabilities,
) -> Result<NormalizedOrderType, RiskRejectReason> {
    let order_supported = match order_type {
        OrderType::Market => capabilities.market,
        OrderType::Limit => capabilities.limit,
        OrderType::Stop => capabilities.stop,
        OrderType::StopLimit => capabilities.stop_limit,
    };
    if !order_supported {
        return Err(RiskRejectReason::UnsupportedOrderType);
    }
    let tif_supported = match time_in_force {
        TimeInForce::Day => capabilities.tif_day,
        TimeInForce::Gtc => capabilities.tif_gtc,
        TimeInForce::Ioc => capabilities.tif_ioc,
        TimeInForce::Fok => capabilities.tif_fok,
        TimeInForce::Gtd => capabilities.tif_gtd,
    };
    if !tif_supported {
        return Err(RiskRejectReason::UnsupportedTimeInForce);
    }
    Ok(NormalizedOrderType {
        order_type,
        time_in_force,
    })
}

/// Additive execution telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExecutionTelemetry {
    /// Last observed command queue depth.
    pub command_queue_depth: u32,
    /// Last observed report queue depth.
    pub report_queue_depth: u32,
    /// Submit-to-report latency sample count.
    pub latency_samples: u64,
    /// Minimum latency in nanoseconds.
    pub min_latency_ns: u64,
    /// Maximum latency in nanoseconds.
    pub max_latency_ns: u64,
    /// Sum of latency samples in nanoseconds.
    pub total_latency_ns: u128,
}

impl ExecutionTelemetry {
    /// Records one latency sample.
    pub fn record_latency(&mut self, latency_ns: u64) {
        self.latency_samples = self.latency_samples.saturating_add(1);
        self.min_latency_ns = if self.min_latency_ns == 0 {
            latency_ns
        } else {
            self.min_latency_ns.min(latency_ns)
        };
        self.max_latency_ns = self.max_latency_ns.max(latency_ns);
        self.total_latency_ns = self.total_latency_ns.saturating_add(u128::from(latency_ns));
    }

    /// Returns average latency in nanoseconds.
    pub fn average_latency_ns(&self) -> u64 {
        if self.latency_samples == 0 {
            0
        } else {
            (self.total_latency_ns / u128::from(self.latency_samples)) as u64
        }
    }
}

/// Route sharding key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShardKey {
    /// Route id.
    pub route_id: RouteId,
    /// Account id.
    pub account_id: AccountId,
    /// Symbol.
    pub symbol: ExecutionSymbol,
}

impl From<RouteKey> for ShardKey {
    fn from(value: RouteKey) -> Self {
        Self {
            route_id: value.route_id,
            account_id: value.account_id,
            symbol: value.symbol,
        }
    }
}

/// Deterministic sharding helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardRouter {
    /// Number of configured shards.
    pub shard_count: usize,
}

impl ShardRouter {
    /// Creates a sharding helper.
    pub const fn new(shard_count: usize) -> Self {
        Self { shard_count }
    }

    /// Returns the shard index for `key`.
    pub fn shard_for(&self, key: ShardKey) -> usize {
        if self.shard_count == 0 {
            return 0;
        }
        let mut hasher = StableHasher::default();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shard_count
    }
}

#[derive(Debug, Default)]
struct StableHasher(u64);

impl Hasher for StableHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Token-bucket style order throttler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderThrottle {
    capacity: u32,
    refill_per_sec: u32,
    tokens: u32,
    last_refill_ns: u64,
}

impl OrderThrottle {
    /// Creates a throttler.
    pub const fn new(capacity: u32, refill_per_sec: u32) -> Self {
        Self {
            capacity,
            refill_per_sec,
            tokens: capacity,
            last_refill_ns: 0,
        }
    }

    /// Attempts to consume one token at `now_ns`.
    pub fn allow(&mut self, now_ns: u64) -> bool {
        self.refill(now_ns);
        if self.tokens == 0 {
            return false;
        }
        self.tokens -= 1;
        true
    }

    /// Returns currently available tokens.
    pub const fn tokens(&self) -> u32 {
        self.tokens
    }

    fn refill(&mut self, now_ns: u64) {
        if self.last_refill_ns == 0 {
            self.last_refill_ns = now_ns;
            return;
        }
        let elapsed_ns = now_ns.saturating_sub(self.last_refill_ns);
        let add = elapsed_ns.saturating_mul(u64::from(self.refill_per_sec)) / 1_000_000_000;
        if add > 0 {
            self.tokens = self.capacity.min(self.tokens.saturating_add(add as u32));
            self.last_refill_ns = now_ns;
        }
    }
}

/// Replay decision used by the OMS simulation harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayDecision {
    /// Decision timestamp.
    pub ts_recv_ns: u64,
    /// Command to execute.
    pub command: ExecutionCommand,
}

/// Replay result for deterministic OMS simulation.
#[derive(Debug, Clone)]
pub struct ReplayResult {
    /// Command reports in replay order.
    pub reports: Vec<ExecutionCommandReport>,
    /// Final execution metrics.
    pub metrics: ExecutionMetrics,
}

/// Runs a deterministic simulated OMS replay.
pub fn replay_simulated_oms(
    routes: Vec<RouteConfig>,
    decisions: &[ReplayDecision],
) -> ExecutionResult<ReplayResult> {
    let mut engine = ExecutionEngine::new(
        SimExecutionAdapter::default(),
        AllowAllRiskGate,
        InMemoryJournal::default(),
        routes,
    );
    engine.start()?;
    let mut reports = Vec::with_capacity(decisions.len());
    let mut events = ExecutionEventBuffer::with_capacity(64);
    for (idx, decision) in decisions.iter().enumerate() {
        events.clear();
        let kind = decision.command.kind();
        let result = match decision.command {
            ExecutionCommand::Submit(req) => engine.submit(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Cancel(req) => engine.cancel(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Amend(req) => engine.amend(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Poll => engine.poll(&mut events),
            ExecutionCommand::RecoverOpenOrders => engine.recover_open_orders(&mut events),
            ExecutionCommand::Stop => Ok(0),
        };
        reports.push(ExecutionCommandReport {
            sequence: (idx + 1) as u64,
            kind,
            result,
            events: events.clone(),
        });
    }
    let metrics = engine.metrics();
    Ok(ReplayResult { reports, metrics })
}

/// Provider adapter context supplied to convenience adapter builders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAdapterContext {
    /// Adapter name.
    pub name: String,
    /// Route configs handled by the adapter.
    pub routes: Vec<RouteConfig>,
    /// Lifecycle state.
    pub lifecycle: ExecutionLifecycleSnapshot,
}

/// Factory trait for provider-specific execution adapters.
pub trait ExecutionAdapterFactory {
    /// Adapter type produced by the factory.
    type Adapter: ExecutionAdapter;

    /// Builds an adapter for `context`.
    ///
    /// # Errors
    ///
    /// Returns an execution error when required provider configuration is
    /// missing or invalid.
    fn build(&self, context: &ProviderAdapterContext) -> ExecutionResult<Self::Adapter>;
}

/// Convenience SDK helpers for provider adapters.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProviderAdapterSdk;

impl ProviderAdapterSdk {
    /// Returns default simulated capabilities for adapter tests.
    pub const fn simulated_capabilities() -> ExecutionCapabilities {
        ExecutionCapabilities::simulated()
    }

    /// Validates route configs for a provider adapter.
    ///
    /// # Errors
    ///
    /// Returns an error when no routes are configured.
    pub fn validate_routes(routes: &[RouteConfig]) -> ExecutionResult<()> {
        if routes.is_empty() {
            return Err(ExecutionError::RouteNotFound);
        }
        Ok(())
    }
}

fn command_kind_u8(kind: JournalCommandKind) -> u8 {
    match kind {
        JournalCommandKind::Submit => 1,
        JournalCommandKind::Cancel => 2,
        JournalCommandKind::Amend => 3,
    }
}

fn command_kind_from_u8(value: u8) -> Option<JournalCommandKind> {
    match value {
        1 => Some(JournalCommandKind::Submit),
        2 => Some(JournalCommandKind::Cancel),
        3 => Some(JournalCommandKind::Amend),
        _ => None,
    }
}

fn event_to_journal_line(event: &ExecutionEvent) -> String {
    format!(
        "E|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        event.exec_type as u8,
        event.order_status as u8,
        event.client_order_id,
        event.orig_client_order_id,
        event.venue_order_id,
        event.execution_id,
        event.account_id,
        event.route_id,
        event.symbol.venue,
        event.symbol.instrument,
        event.last_qty.0,
        event.last_price.0,
        event.cumulative_qty.0,
        event.leaves_qty.0,
        event.average_price.0,
        event.ts_exchange_ns,
        event.ts_recv_ns,
        event.reason as u8,
        sanitize_field(event.text.as_str())
    )
}

fn parse_journal_line(line: &str) -> ExecutionResult<Option<JournalRecord>> {
    let parts: Vec<&str> = line.split('|').collect();
    match parts.first().copied() {
        Some("C") if parts.len() == 4 => {
            let kind = parts[1]
                .parse::<u8>()
                .ok()
                .and_then(command_kind_from_u8)
                .ok_or_else(|| ExecutionError::Journal("invalid command kind".to_string()))?;
            let client_order_id = ClientOrderId::new(parts[2])
                .map_err(|err| ExecutionError::Journal(err.to_string()))?;
            let ts_ns = parts[3]
                .parse::<u64>()
                .map_err(|err| ExecutionError::Journal(err.to_string()))?;
            Ok(Some(JournalRecord::Command {
                kind,
                client_order_id,
                ts_ns,
            }))
        }
        Some("E") if parts.len() == 20 => Ok(Some(JournalRecord::Event(Box::new(
            parse_event_parts(&parts)?,
        )))),
        Some(_) => Err(ExecutionError::Journal("invalid journal line".to_string())),
        None => Ok(None),
    }
}

fn parse_event_parts(parts: &[&str]) -> ExecutionResult<ExecutionEvent> {
    Ok(ExecutionEvent {
        exec_type: execution_type_from_u8(parse_u8(parts[1])?)?,
        order_status: order_status_from_u8(parse_u8(parts[2])?)?,
        client_order_id: fixed(parts[3])?,
        orig_client_order_id: fixed(parts[4])?,
        venue_order_id: fixed(parts[5])?,
        execution_id: fixed(parts[6])?,
        account_id: fixed(parts[7])?,
        route_id: fixed(parts[8])?,
        symbol: ExecutionSymbol {
            venue: fixed(parts[9])?,
            instrument: fixed(parts[10])?,
        },
        last_qty: OrderQty(parse_i64(parts[11])?),
        last_price: OrderPrice(parse_i64(parts[12])?),
        cumulative_qty: OrderQty(parse_i64(parts[13])?),
        leaves_qty: OrderQty(parse_i64(parts[14])?),
        average_price: OrderPrice(parse_i64(parts[15])?),
        ts_exchange_ns: parse_u64(parts[16])?,
        ts_recv_ns: parse_u64(parts[17])?,
        reason: risk_reason_from_u8(parse_u8(parts[18])?)?,
        text: fixed(parts[19])?,
    })
}

fn fixed<const N: usize>(value: &str) -> ExecutionResult<FixedAscii<N>> {
    FixedAscii::new(value).map_err(|err| ExecutionError::Journal(err.to_string()))
}

fn parse_u8(value: &str) -> ExecutionResult<u8> {
    value
        .parse::<u8>()
        .map_err(|err| ExecutionError::Journal(err.to_string()))
}

fn parse_i64(value: &str) -> ExecutionResult<i64> {
    value
        .parse::<i64>()
        .map_err(|err| ExecutionError::Journal(err.to_string()))
}

fn parse_u64(value: &str) -> ExecutionResult<u64> {
    value
        .parse::<u64>()
        .map_err(|err| ExecutionError::Journal(err.to_string()))
}

fn execution_type_from_u8(value: u8) -> ExecutionResult<ExecutionType> {
    match value {
        1 => Ok(ExecutionType::Ack),
        2 => Ok(ExecutionType::Reject),
        3 => Ok(ExecutionType::Trade),
        4 => Ok(ExecutionType::CancelPending),
        5 => Ok(ExecutionType::CancelAck),
        6 => Ok(ExecutionType::CancelReject),
        7 => Ok(ExecutionType::ReplacePending),
        8 => Ok(ExecutionType::ReplaceAck),
        9 => Ok(ExecutionType::ReplaceReject),
        10 => Ok(ExecutionType::Expire),
        11 => Ok(ExecutionType::Status),
        12 => Ok(ExecutionType::Restated),
        13 => Ok(ExecutionType::AdapterDegraded),
        _ => Err(ExecutionError::Journal(
            "invalid execution type".to_string(),
        )),
    }
}

fn order_status_from_u8(value: u8) -> ExecutionResult<OrderStatus> {
    match value {
        1 => Ok(OrderStatus::PendingNew),
        2 => Ok(OrderStatus::New),
        3 => Ok(OrderStatus::PartiallyFilled),
        4 => Ok(OrderStatus::Filled),
        5 => Ok(OrderStatus::PendingCancel),
        6 => Ok(OrderStatus::Cancelled),
        7 => Ok(OrderStatus::PendingReplace),
        8 => Ok(OrderStatus::Replaced),
        9 => Ok(OrderStatus::Rejected),
        10 => Ok(OrderStatus::Expired),
        11 => Ok(OrderStatus::Suspended),
        12 => Ok(OrderStatus::Unknown),
        _ => Err(ExecutionError::Journal("invalid order status".to_string())),
    }
}

fn risk_reason_from_u8(value: u8) -> ExecutionResult<RiskRejectReason> {
    match value {
        0 => Ok(RiskRejectReason::None),
        1 => Ok(RiskRejectReason::KillSwitch),
        2 => Ok(RiskRejectReason::AccountDisabled),
        3 => Ok(RiskRejectReason::RouteDisabled),
        4 => Ok(RiskRejectReason::SymbolDisabled),
        5 => Ok(RiskRejectReason::MaxOrderQty),
        6 => Ok(RiskRejectReason::MaxOrderNotional),
        7 => Ok(RiskRejectReason::MaxOpenOrders),
        8 => Ok(RiskRejectReason::MaxOpenNotional),
        9 => Ok(RiskRejectReason::PriceBand),
        10 => Ok(RiskRejectReason::DuplicateClientOrderId),
        11 => Ok(RiskRejectReason::UnsupportedOrderType),
        12 => Ok(RiskRejectReason::UnsupportedTimeInForce),
        _ => Err(ExecutionError::Journal("invalid risk reason".to_string())),
    }
}

fn sanitize_field(value: &str) -> String {
    value.replace('|', " ")
}

fn reject(reason: RiskRejectReason, text: &str) -> RiskDecision {
    let text = ExecutionText::new(text).unwrap_or_else(|_| ExecutionText::empty());
    RiskDecision::reject(reason, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn symbol(instrument: &str) -> ExecutionSymbol {
        ExecutionSymbol {
            venue: id::<16>("SIM"),
            instrument: id::<32>(instrument),
        }
    }

    fn route() -> RouteConfig {
        RouteConfig {
            route_id: id("SIM"),
            account_id: id("ACC"),
            symbol: symbol("ES"),
            enabled: true,
            risk_limits: RiskLimits {
                kill_switch: false,
                max_order_qty: 100,
                max_order_notional: 1_000_000,
                max_open_orders: 10,
                max_open_notional: 10_000_000,
                price_band_ticks: 0,
            },
        }
    }

    fn order(client_order_id: &str) -> OrderRequest {
        OrderRequest {
            client_order_id: id(client_order_id),
            account_id: id("ACC"),
            route_id: id("SIM"),
            strategy_id: id("STRAT"),
            symbol: symbol("ES"),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(10),
            limit_price: OrderPrice(5000),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }
    }

    #[test]
    fn fanout_drops_when_subscriber_queue_is_full() {
        let fanout = ExecutionEventFanout::new(1);
        let sub = fanout.subscribe();
        let req = order("C1");
        fanout.publish(ExecutionEvent::accepted(&req, id("V1")));
        fanout.publish(ExecutionEvent::accepted(&req, id("V2")));

        assert!(sub.try_recv().is_some());
        assert_eq!(fanout.dropped_events(), 1);
    }

    #[test]
    fn file_journal_replays_commands_and_events() {
        let path =
            std::env::temp_dir().join(format!("orderflow-journal-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut journal = FileExecutionJournal::open(&path, false).unwrap();
        let req = order("C1");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req.client_order_id,
                req.ts_recv_ns,
            )
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req, id("V1")))
            .unwrap();
        drop(journal);

        let journal = FileExecutionJournal::open(&path, false).unwrap();
        let mut records = Vec::new();
        assert_eq!(journal.replay(&mut records).unwrap(), 2);
        assert!(matches!(records[0], JournalRecord::Command { .. }));
        assert!(matches!(records[1], JournalRecord::Event(_)));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wal_journal_replays_commands_and_events() {
        let path = std::env::temp_dir().join(format!("orderflow-wal-{}.ofwal", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut journal = WalExecutionJournal::open_path(&path, WalSyncPolicy::Never).unwrap();
        let req = order("C1");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req.client_order_id,
                req.ts_recv_ns,
            )
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req, id("V1")))
            .unwrap();
        assert_eq!(journal.next_sequence(), WalSequence(3));
        let metrics = journal.metrics();
        assert_eq!(metrics.records_written, 2);
        assert!(metrics.bytes_written > 0);
        assert_eq!(metrics.write_failures, 0);
        drop(journal);

        let journal = WalExecutionJournal::open_path(&path, WalSyncPolicy::Never).unwrap();
        assert_eq!(journal.next_sequence(), WalSequence(3));
        let report = journal.integrity_report().unwrap();
        assert!(report.valid);
        assert_eq!(report.records, 2);

        let mut records = Vec::new();
        assert_eq!(journal.replay(&mut records).unwrap(), 2);
        assert!(matches!(records[0], JournalRecord::Command { .. }));
        assert!(matches!(records[1], JournalRecord::Event(_)));

        let mut tail = Vec::new();
        let replay = journal.replay_from(WalSequence(2), &mut tail).unwrap();
        assert_eq!(replay.records, 1);
        assert_eq!(replay.first_sequence, Some(WalSequence(2)));
        assert!(matches!(tail[0], JournalRecord::Event(_)));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wal_journal_fails_closed_on_corruption() {
        let path = std::env::temp_dir().join(format!(
            "orderflow-wal-corrupt-{}.ofwal",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut journal = WalExecutionJournal::open_path(&path, WalSyncPolicy::Never).unwrap();
        let req = order("C2");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req.client_order_id,
                req.ts_recv_ns,
            )
            .unwrap();
        drop(journal);

        let mut bytes = std::fs::read(&path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x01;
        std::fs::write(&path, bytes).unwrap();

        assert!(WalExecutionJournal::open_path(&path, WalSyncPolicy::Never).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn segmented_wal_rotates_and_replays_across_segments() {
        let root =
            std::env::temp_dir().join(format!("orderflow-segmented-wal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root)
                .with_sync_policy(WalSyncPolicy::Never)
                .with_max_segment_records(2),
        )
        .unwrap();

        let req1 = order("C1");
        let req2 = order("C2");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req1.client_order_id,
                req1.ts_recv_ns,
            )
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req1, id("V1")))
            .unwrap();
        journal
            .record_command(
                JournalCommandKind::Submit,
                req2.client_order_id,
                req2.ts_recv_ns,
            )
            .unwrap();
        journal.sync().unwrap();

        assert_eq!(journal.next_sequence(), WalSequence(5));
        let metrics = journal.metrics();
        assert_eq!(metrics.records_written, 4);
        assert!(metrics.bytes_written > 0);
        assert_eq!(metrics.segment_rotations, 1);
        assert!(metrics.sync_count > 0);
        assert!(metrics.manifest_writes > 0);
        assert_eq!(journal.manifest().segments.len(), 2);
        assert!(journal.manifest().segments[0].sealed);
        assert_eq!(
            journal.manifest().segments[0].last_sequence,
            Some(WalSequence(3))
        );
        assert_eq!(
            journal.manifest().segments[1].first_sequence,
            Some(WalSequence(4))
        );

        let report = journal.integrity_report().unwrap();
        assert!(report.valid);
        assert_eq!(report.segments, 2);
        assert_eq!(report.records, 4);

        let mut replayed = Vec::new();
        assert_eq!(journal.replay(&mut replayed).unwrap(), 3);
        assert!(matches!(replayed[0], JournalRecord::Command { .. }));
        assert!(matches!(replayed[1], JournalRecord::Event(_)));
        assert!(matches!(replayed[2], JournalRecord::Command { .. }));

        let mut tail = Vec::new();
        let replay = journal.replay_from(WalSequence(4), &mut tail).unwrap();
        assert_eq!(replay.records, 1);
        assert_eq!(replay.first_sequence, Some(WalSequence(4)));
        assert!(matches!(tail[0], JournalRecord::Command { .. }));
        assert!(root.join("manifest").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn segmented_wal_reopens_and_continues_after_sealed_segment() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-segmented-wal-reopen-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut journal = SegmentedWalExecutionJournal::open(
                WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
            )
            .unwrap();
            let req = order("C1");
            journal
                .record_command(
                    JournalCommandKind::Submit,
                    req.client_order_id,
                    req.ts_recv_ns,
                )
                .unwrap();
            journal.rotate_segment().unwrap();
            journal.sync().unwrap();
            assert_eq!(journal.next_sequence(), WalSequence(3));
            assert_eq!(journal.manifest().segments.len(), 2);
        }

        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        assert_eq!(journal.next_sequence(), WalSequence(3));
        let req = order("C2");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req.client_order_id,
                req.ts_recv_ns,
            )
            .unwrap();

        let mut replayed = Vec::new();
        assert_eq!(journal.replay(&mut replayed).unwrap(), 2);
        assert_eq!(journal.manifest().segments.len(), 2);
        assert_eq!(
            journal.manifest().segments[1].first_sequence,
            Some(WalSequence(3))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn segmented_wal_fails_closed_on_corrupt_segment() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-segmented-wal-corrupt-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut journal = SegmentedWalExecutionJournal::open(
                WalSegmentConfig::new(&root)
                    .with_sync_policy(WalSyncPolicy::Never)
                    .with_max_segment_records(1),
            )
            .unwrap();
            let req = order("C1");
            journal
                .record_command(
                    JournalCommandKind::Submit,
                    req.client_order_id,
                    req.ts_recv_ns,
                )
                .unwrap();
            journal.sync().unwrap();
        }

        let segment_path = root.join("wal-000000000001.ofwal");
        let mut bytes = std::fs::read(&segment_path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x01;
        std::fs::write(&segment_path, bytes).unwrap();

        assert!(SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never)
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn segmented_wal_inspect_root_reports_valid_and_corrupt_segments() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-segmented-wal-inspect-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut journal = SegmentedWalExecutionJournal::open(
                WalSegmentConfig::new(&root)
                    .with_sync_policy(WalSyncPolicy::Never)
                    .with_max_segment_records(1),
            )
            .unwrap();
            let req = order("C1");
            journal
                .record_command(
                    JournalCommandKind::Submit,
                    req.client_order_id,
                    req.ts_recv_ns,
                )
                .unwrap();
            journal.sync().unwrap();
        }

        let report = SegmentedWalExecutionJournal::inspect_root(&root).unwrap();
        assert!(report.valid);
        assert_eq!(report.segments, 1);
        assert_eq!(report.records, 1);
        assert_eq!(report.first_sequence, Some(WalSequence(1)));
        assert_eq!(report.last_sequence, Some(WalSequence(1)));

        let segment_path = root.join("wal-000000000001.ofwal");
        let mut bytes = std::fs::read(&segment_path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x01;
        std::fs::write(&segment_path, bytes).unwrap();

        let report = SegmentedWalExecutionJournal::inspect_root(&root).unwrap();
        assert!(!report.valid);
        assert_eq!(report.segments, 1);
        assert_eq!(report.checksum_failures, 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn checkpoint_store_saves_loads_and_prunes() {
        let root =
            std::env::temp_dir().join(format!("orderflow-checkpoints-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut store = FileExecutionCheckpointStore::open(
            CheckpointConfig::new(&root)
                .with_sync_on_save(false)
                .with_max_retained(1),
        )
        .unwrap();
        let req = order("C3");
        let mut state = OrderState::pending_new(&req);
        state.venue_order_id = id("V3");
        let position = CheckpointPosition {
            key: PositionKey {
                account_id: req.account_id,
                strategy_id: req.strategy_id,
                symbol: req.symbol,
            },
            position: Position {
                net_qty: 10,
                buy_qty: 10,
                sell_qty: 0,
                gross_notional: 50_000,
                average_price: 5_000,
            },
        };

        let first = ExecutionCheckpoint::new(1, WalSequence(10), 100)
            .with_open_orders(vec![state])
            .with_positions(vec![position])
            .with_route_config_hash(7)
            .with_kill_switch(true);
        let second = ExecutionCheckpoint::new(2, WalSequence(20), 200);

        let manifest = store.save_checkpoint(&first).unwrap();
        assert_eq!(manifest.last_applied_sequence, WalSequence(10));
        store.save_checkpoint(&second).unwrap();

        let latest = store.load_latest().unwrap().unwrap();
        assert_eq!(latest.checkpoint_id, 2);
        assert_eq!(latest.last_applied_sequence, WalSequence(20));
        assert!(store.validate_checkpoint(&latest).unwrap());

        let checkpoints = store.list_checkpoints().unwrap();
        assert_eq!(checkpoints.len(), 2);
        assert_eq!(store.prune_old().unwrap(), 1);
        assert_eq!(store.list_checkpoints().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn checkpoint_store_rejects_corrupt_checkpoint() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-checkpoints-corrupt-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut store = FileExecutionCheckpointStore::open(
            CheckpointConfig::new(&root).with_sync_on_save(false),
        )
        .unwrap();
        let checkpoint = ExecutionCheckpoint::new(1, WalSequence(1), 1);
        let manifest = store.save_checkpoint(&checkpoint).unwrap();

        let mut bytes = std::fs::read(&manifest.path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x01;
        std::fs::write(&manifest.path, bytes).unwrap();

        assert!(store.load_latest().is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recovery_replays_segmented_wal_after_latest_checkpoint() {
        let root =
            std::env::temp_dir().join(format!("orderflow-recovery-wal-{}", std::process::id()));
        let checkpoint_root = std::env::temp_dir().join(format!(
            "orderflow-recovery-checkpoints-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&checkpoint_root);

        let req = order("C1");
        let mut state = OrderState::pending_new(&req);
        state.status = OrderStatus::New;
        state.venue_order_id = id("V1");
        state.leaves_qty = req.quantity;
        state.updated_ns = 2;

        let mut store = FileExecutionCheckpointStore::open(
            CheckpointConfig::new(&checkpoint_root).with_sync_on_save(false),
        )
        .unwrap();
        store
            .save_checkpoint(
                &ExecutionCheckpoint::new(1, WalSequence(2), 100).with_open_orders(vec![state]),
            )
            .unwrap();

        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        journal
            .record_command(JournalCommandKind::Submit, req.client_order_id, 1)
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req, id("V1")))
            .unwrap();
        let mut cancel_ack = ExecutionEvent::accepted(&req, id("V1"));
        cancel_ack.exec_type = ExecutionType::CancelAck;
        cancel_ack.order_status = OrderStatus::Cancelled;
        cancel_ack.orig_client_order_id = req.client_order_id;
        cancel_ack.client_order_id = id("CXL1");
        cancel_ack.leaves_qty = OrderQty(0);
        cancel_ack.ts_recv_ns = 3;
        journal.record_event(&cancel_ack).unwrap();
        journal.sync().unwrap();

        let result = recover_latest_checkpoint_from_segmented_wal(&store, &journal).unwrap();
        assert_eq!(result.replay.first_sequence, Some(WalSequence(3)));
        assert_eq!(result.events_applied, 1);
        assert_eq!(result.commands_seen, 0);
        assert!(result.venue_reconciliation_required);
        assert!(!result.submissions_enabled);
        assert_eq!(result.state.orders().len(), 1);
        assert_eq!(result.state.orders()[0].status, OrderStatus::Cancelled);
        assert!(result.state.open_orders().is_empty());

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&checkpoint_root);
    }

    #[test]
    fn recovery_is_deterministic_for_same_checkpoint_and_wal() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-recovery-deterministic-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);

        let req = order("C1");
        let mut state = OrderState::pending_new(&req);
        state.status = OrderStatus::New;
        state.venue_order_id = id("V1");
        let checkpoint =
            ExecutionCheckpoint::new(1, WalSequence(2), 100).with_open_orders(vec![state]);

        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        journal
            .record_command(JournalCommandKind::Submit, req.client_order_id, 1)
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req, id("V1")))
            .unwrap();
        let mut fill = ExecutionEvent::accepted(&req, id("V1"));
        fill.exec_type = ExecutionType::Trade;
        fill.order_status = OrderStatus::Filled;
        fill.last_qty = req.quantity;
        fill.last_price = req.limit_price;
        fill.cumulative_qty = req.quantity;
        fill.leaves_qty = OrderQty(0);
        fill.average_price = req.limit_price;
        fill.ts_recv_ns = 3;
        journal.record_event(&fill).unwrap();

        let plan = RecoveryPlan::from_checkpoint(&checkpoint);
        let first = recover_oms_state_from_segmented_wal(plan.clone(), Some(&checkpoint), &journal)
            .unwrap();
        let second =
            recover_oms_state_from_segmented_wal(plan, Some(&checkpoint), &journal).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.state.orders()[0].status, OrderStatus::Filled);
        assert_eq!(first.state.orders()[0].average_price, req.limit_price);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recovery_fails_closed_on_unknown_order_event() {
        let req = order("C1");
        let record = JournalRecord::Event(Box::new(ExecutionEvent::accepted(&req, id("V1"))));
        let err =
            recover_oms_state_from_records(RecoveryPlan::default(), None, &[record]).unwrap_err();
        assert!(err.to_string().contains("unknown order"));
    }

    #[test]
    fn reconciliation_detects_venue_only_state() {
        let req = order("C1");
        let state = OrderState::pending_new(&req);
        let report = reconcile_open_orders(&[], &[state]);
        assert_eq!(report.items.len(), 1);
        assert_eq!(report.items[0].action, ReconciliationAction::VenueOnly);
    }

    #[test]
    fn detailed_reconciliation_classifies_mismatches() {
        let req = order("C1");
        let local = OrderState::pending_new(&req);
        let mut venue = local;
        venue.status = OrderStatus::New;

        let report = reconcile_open_orders_detailed(&[local], &[venue]);
        assert_eq!(report.details.len(), 1);
        assert_eq!(
            report.details[0].issue,
            ReconciliationIssueKind::StatusMismatch
        );
        assert!(report.has_discrepancies());

        let mut venue = local;
        venue.leaves_qty = OrderQty(5);
        let report = reconcile_open_orders_detailed(&[local], &[venue]);
        assert_eq!(
            report.details[0].issue,
            ReconciliationIssueKind::QuantityMismatch
        );

        let mut venue = local;
        venue.average_price = OrderPrice(5000);
        let report = reconcile_open_orders_detailed(&[local], &[venue]);
        assert_eq!(
            report.details[0].issue,
            ReconciliationIssueKind::PriceMismatch
        );
    }

    #[test]
    fn reconciliation_policy_blocks_until_host_action_completes() {
        let req = order("C1");
        let state = OrderState::pending_new(&req);
        let report = reconcile_open_orders_detailed(&[], &[state]);
        let policy = ReconciliationPolicy::fail_closed()
            .with_venue_only(ReconciliationPolicyAction::CancelVenueOrder);

        let decision = evaluate_reconciliation_policy(&report, policy);
        assert!(!decision.submissions_enabled);
        assert!(decision.venue_cancels_required);
        assert!(!decision.fail_closed);
        assert_eq!(
            decision.items[0].action,
            ReconciliationPolicyAction::CancelVenueOrder
        );

        let approval = evaluate_reconciliation_policy(
            &report,
            ReconciliationPolicy::require_operator_approval(),
        );
        assert!(approval.operator_approval_required);
        assert!(!approval.submissions_enabled);
    }

    #[test]
    fn throttle_refills_over_time() {
        let mut throttle = OrderThrottle::new(1, 1);
        assert!(throttle.allow(1));
        assert!(!throttle.allow(2));
        assert!(throttle.allow(1_000_000_002));
    }

    #[test]
    fn replay_simulation_is_deterministic() {
        let decisions = [ReplayDecision {
            ts_recv_ns: 2,
            command: ExecutionCommand::Submit(order("C1")),
        }];
        let result = replay_simulated_oms(vec![route()], &decisions).unwrap();
        assert_eq!(result.reports.len(), 1);
        assert_eq!(result.reports[0].events.len(), 2);
        assert_eq!(result.metrics.submitted, 1);
    }

    #[test]
    fn normalize_rejects_unsupported_tif() {
        let caps = VenueOrderCapabilities {
            market: true,
            limit: true,
            stop: false,
            stop_limit: false,
            tif_day: true,
            tif_gtc: false,
            tif_ioc: false,
            tif_fok: false,
            tif_gtd: false,
        };
        assert_eq!(
            normalize_order_type(OrderType::Limit, TimeInForce::Gtc, caps).unwrap_err(),
            RiskRejectReason::UnsupportedTimeInForce
        );
    }
}
