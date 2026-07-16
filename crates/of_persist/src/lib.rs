#![doc = include_str!("../README.md")]

use std::collections::BTreeSet;
use std::fs::{self, create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use of_core::{BookAction, BookUpdate, Side, TradePrint};
use serde::Deserialize;

const JSONL_SCHEMA_VERSION: u32 = 1;
const MARKET_DATA_WAL_MAGIC: [u8; 4] = *b"OFMW";
const MARKET_DATA_WAL_VERSION: u16 = 1;
const MARKET_DATA_WAL_HEADER_LEN: usize = 64;
const MARKET_DATA_CHECKPOINT_MAGIC: [u8; 4] = *b"OFMC";
const MARKET_DATA_CHECKPOINT_VERSION: u16 = 1;
const MARKET_DATA_CHECKPOINT_HEADER_LEN: usize = 64;

/// Persistence-layer errors.
#[derive(Debug)]
pub enum PersistError {
    /// Filesystem I/O failure.
    Io(std::io::Error),
}

impl From<std::io::Error> for PersistError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Result type alias used by persistence APIs.
pub type PersistResult<T> = Result<T, PersistError>;

/// Monotonic normalized market-data WAL sequence.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MarketDataWalSequence(pub u64);

/// Monotonic market-data checkpoint identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MarketDataCheckpointId(pub u64);

/// Normalized market-data WAL record kind.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MarketDataWalRecordKind {
    /// Normalized book update payload.
    BookUpdate = 1,
    /// Normalized trade print payload.
    TradePrint = 2,
    /// Writer heartbeat payload.
    Heartbeat = 3,
    /// Sequence gap marker payload.
    GapMarker = 4,
    /// Segment seal marker payload.
    SegmentSeal = 5,
}

/// Opaque market-data checkpoint payload category.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MarketDataCheckpointKind {
    /// Materialized order-book state.
    Book = 1,
    /// Analytics accumulator state.
    Analytics = 2,
    /// Combined book and analytics state.
    BookAndAnalytics = 3,
    /// Deterministic signal state.
    SignalState = 4,
    /// Provider and normalized sequence cache state.
    SequenceState = 5,
    /// Runtime subscription, quality, and metric baseline state.
    RuntimeState = 6,
    /// User-defined checkpoint payload.
    Custom = 65_535,
}

impl MarketDataCheckpointKind {
    fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::Book),
            2 => Some(Self::Analytics),
            3 => Some(Self::BookAndAnalytics),
            4 => Some(Self::SignalState),
            5 => Some(Self::SequenceState),
            6 => Some(Self::RuntimeState),
            65_535 => Some(Self::Custom),
            _ => None,
        }
    }
}

impl MarketDataWalRecordKind {
    fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::BookUpdate),
            2 => Some(Self::TradePrint),
            3 => Some(Self::Heartbeat),
            4 => Some(Self::GapMarker),
            5 => Some(Self::SegmentSeal),
            _ => None,
        }
    }
}

/// Configuration for [`MarketDataWal`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MarketDataWalConfig {
    path: PathBuf,
    sync_on_write: bool,
}

impl MarketDataWalConfig {
    /// Creates WAL config for `path`.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            sync_on_write: false,
        }
    }

    /// Sets whether every append calls `sync_data`.
    pub fn with_sync_on_write(mut self, sync_on_write: bool) -> Self {
        self.sync_on_write = sync_on_write;
        self
    }

    /// Returns the WAL path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns whether every append syncs data.
    pub const fn sync_on_write(&self) -> bool {
        self.sync_on_write
    }
}

/// One decoded normalized market-data WAL record.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataWalRecord {
    /// WAL sequence assigned by the writer.
    pub sequence: MarketDataWalSequence,
    /// Record kind.
    pub kind: MarketDataWalRecordKind,
    /// Provider-native sequence when known.
    pub provider_sequence: u64,
    /// Normalized book/trade event sequence when known.
    pub event_sequence: u64,
    /// Exchange timestamp in nanoseconds when known.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds when known.
    pub ts_recv_ns: u64,
    /// Raw encoded payload bytes.
    pub payload: Vec<u8>,
}

/// Replay summary for normalized market-data WAL reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataWalReplayResult {
    /// Number of records replayed.
    pub records: usize,
    /// Number of bytes consumed.
    pub bytes: u64,
    /// First replayed sequence.
    pub first_sequence: Option<MarketDataWalSequence>,
    /// Last replayed sequence.
    pub last_sequence: Option<MarketDataWalSequence>,
}

/// Integrity report for a normalized market-data WAL file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataWalIntegrityReport {
    /// True when all complete records validate.
    pub valid: bool,
    /// Complete records inspected.
    pub records: u64,
    /// Bytes consumed by complete records.
    pub bytes: u64,
    /// Number of checksum failures.
    pub checksum_failures: u64,
    /// Number of sequence continuity failures.
    pub sequence_failures: u64,
    /// True when the file ends in an incomplete frame.
    pub truncated_tail: bool,
    /// Last valid sequence.
    pub last_sequence: Option<MarketDataWalSequence>,
}

/// Low-latency market-data WAL metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataWalMetrics {
    /// Number of records written successfully.
    pub records_written: u64,
    /// Number of bytes written successfully.
    pub bytes_written: u64,
    /// Number of successful sync operations.
    pub sync_count: u64,
    /// Number of append failures.
    pub write_failures: u64,
    /// Number of sync failures.
    pub sync_failures: u64,
}

/// Production market-data persistence writer mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataPersistenceMode {
    /// Production market-data persistence is disabled.
    #[default]
    Disabled,
    /// Writes occur on the caller path and errors are returned immediately.
    InlineStrict,
    /// Writes are expected to be queued to a bounded single-writer worker.
    BoundedAsync,
    /// Writes may be dropped according to policy, with every drop counted.
    BestEffort,
}

/// Host action when production market-data persistence is degraded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataPersistenceFailureAction {
    /// Continue processing and mark persistence degraded.
    #[default]
    MarkDegraded,
    /// Stop market-data processing.
    StopMarketData,
    /// Stop trading while allowing market-data processing to continue.
    StopTrading,
    /// Fail the process.
    FailProcess,
    /// Switch to memory-only retention.
    MemoryOnly,
}

/// Production market-data persistence policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataPersistencePolicy {
    /// Writer mode.
    pub mode: MarketDataPersistenceMode,
    /// Bounded queue depth for async writer modes. Zero means unspecified.
    pub max_queue_depth: u32,
    /// Failure action selected by the host.
    pub failure_action: MarketDataPersistenceFailureAction,
}

impl MarketDataPersistencePolicy {
    /// Creates a disabled persistence policy.
    pub const fn disabled() -> Self {
        Self {
            mode: MarketDataPersistenceMode::Disabled,
            max_queue_depth: 0,
            failure_action: MarketDataPersistenceFailureAction::MarkDegraded,
        }
    }

    /// Creates an inline strict persistence policy.
    pub const fn inline_strict() -> Self {
        Self {
            mode: MarketDataPersistenceMode::InlineStrict,
            max_queue_depth: 0,
            failure_action: MarketDataPersistenceFailureAction::StopTrading,
        }
    }

    /// Creates a bounded async persistence policy.
    pub const fn bounded_async(max_queue_depth: u32) -> Self {
        Self {
            mode: MarketDataPersistenceMode::BoundedAsync,
            max_queue_depth,
            failure_action: MarketDataPersistenceFailureAction::MarkDegraded,
        }
    }

    /// Sets the failure action.
    pub const fn with_failure_action(
        mut self,
        failure_action: MarketDataPersistenceFailureAction,
    ) -> Self {
        self.failure_action = failure_action;
        self
    }

    /// Returns true when production persistence is enabled.
    pub const fn enabled(self) -> bool {
        !matches!(self.mode, MarketDataPersistenceMode::Disabled)
    }
}

impl Default for MarketDataPersistencePolicy {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Production market-data persistence health snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataPersistenceHealth {
    /// Configured persistence mode.
    pub mode: MarketDataPersistenceMode,
    /// True when production persistence is enabled.
    pub enabled: bool,
    /// True when the persistence path is degraded.
    pub degraded: bool,
    /// Current writer queue depth.
    pub queue_depth: u32,
    /// Writer lag measured in records.
    pub records_lag: u64,
    /// Writer lag measured in nanoseconds.
    pub lag_ns: u64,
    /// Bytes waiting to be persisted.
    pub bytes_pending: u64,
    /// Number of dropped records.
    pub dropped_records: u64,
    /// Number of WAL write failures.
    pub write_failures: u64,
    /// Number of WAL sync failures.
    pub sync_failures: u64,
    /// Last persistence error text.
    pub last_error: Option<String>,
}

impl MarketDataPersistenceHealth {
    /// Creates a health snapshot from policy and WAL metrics.
    pub fn from_wal_metrics(
        policy: MarketDataPersistencePolicy,
        metrics: MarketDataWalMetrics,
    ) -> Self {
        let degraded = metrics.write_failures > 0 || metrics.sync_failures > 0;
        Self {
            mode: policy.mode,
            enabled: policy.enabled(),
            degraded,
            write_failures: metrics.write_failures,
            sync_failures: metrics.sync_failures,
            ..Self::default()
        }
    }

    /// Sets queue and lag fields.
    pub const fn with_lag(
        mut self,
        queue_depth: u32,
        records_lag: u64,
        lag_ns: u64,
        bytes_pending: u64,
    ) -> Self {
        self.queue_depth = queue_depth;
        self.records_lag = records_lag;
        self.lag_ns = lag_ns;
        self.bytes_pending = bytes_pending;
        self
    }

    /// Sets drop count and marks the path degraded when records were dropped.
    pub const fn with_dropped_records(mut self, dropped_records: u64) -> Self {
        self.dropped_records = dropped_records;
        self.degraded = self.degraded || dropped_records > 0;
        self
    }

    /// Sets the last error and marks the path degraded.
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.last_error = Some(error.into());
        self.degraded = true;
        self
    }

    /// Returns true when the configured persistence path is enabled and not degraded.
    pub const fn is_healthy(&self) -> bool {
        self.enabled && !self.degraded
    }
}

/// Relative importance of a market-data persistence record under backpressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataRecordCriticality {
    /// Low-priority diagnostic or redundant depth record.
    Low,
    /// Normal market-data record.
    #[default]
    Normal,
    /// High-priority state transition or quality marker.
    High,
    /// Critical record that should not be dropped by policy helpers.
    Critical,
}

/// Backpressure drop policy for bounded market-data persistence writers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataBackpressureDropPolicy {
    /// Reject new persistence records when limits are exceeded.
    #[default]
    RejectNew,
    /// Drop the candidate record.
    DropNewest,
    /// Ask the host queue to drop its oldest queued record.
    DropOldest,
    /// Ask the host queue to drop a queued low-priority record.
    DropLowestPriority,
    /// Preserve trade records and drop lower-priority non-trade records first.
    PreserveTrades,
}

/// Backpressure condition that triggered a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataBackpressureReason {
    /// No backpressure condition is active.
    #[default]
    None,
    /// Writer queue depth reached or exceeded the configured bound.
    QueueDepth,
    /// Writer record lag reached or exceeded the configured bound.
    RecordsLag,
    /// Writer lag in nanoseconds reached or exceeded the configured bound.
    TimeLag,
    /// Pending bytes reached or exceeded the configured bound.
    BytesPending,
    /// Persistence path is already degraded.
    Degraded,
}

/// Backpressure action for one candidate persistence record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataBackpressureAction {
    /// Accept the candidate record.
    #[default]
    Accept,
    /// Reject the candidate record without selecting a queued record to drop.
    Reject,
    /// Drop the candidate record.
    DropCurrent,
    /// Ask the host queue to drop its oldest queued record before accepting.
    DropQueuedOldest,
    /// Ask the host queue to drop a queued low-priority record before accepting.
    DropQueuedLowestPriority,
    /// Stop market-data processing.
    StopMarketData,
    /// Stop trading while allowing market-data processing to continue.
    StopTrading,
    /// Fail the process.
    FailProcess,
    /// Switch the host to memory-only retention.
    MemoryOnly,
}

/// Bounded writer backpressure policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataBackpressurePolicy {
    /// Maximum writer queue depth. Zero disables this bound.
    pub max_queue_depth: u32,
    /// Maximum writer lag in records. Zero disables this bound.
    pub max_records_lag: u64,
    /// Maximum writer lag in nanoseconds. Zero disables this bound.
    pub max_lag_ns: u64,
    /// Maximum pending bytes. Zero disables this bound.
    pub max_bytes_pending: u64,
    /// Drop policy used when a limit is exceeded.
    pub drop_policy: MarketDataBackpressureDropPolicy,
    /// Minimum criticality that cannot be dropped by policy helpers.
    pub protected_criticality: MarketDataRecordCriticality,
    /// Failure action used when the persistence path is already degraded.
    pub failure_action: MarketDataPersistenceFailureAction,
}

impl MarketDataBackpressurePolicy {
    /// Creates a reject-new policy with queue-depth bound.
    pub const fn reject_new(max_queue_depth: u32) -> Self {
        Self {
            max_queue_depth,
            max_records_lag: 0,
            max_lag_ns: 0,
            max_bytes_pending: 0,
            drop_policy: MarketDataBackpressureDropPolicy::RejectNew,
            protected_criticality: MarketDataRecordCriticality::Critical,
            failure_action: MarketDataPersistenceFailureAction::MarkDegraded,
        }
    }

    /// Sets record lag bound.
    pub const fn with_max_records_lag(mut self, max_records_lag: u64) -> Self {
        self.max_records_lag = max_records_lag;
        self
    }

    /// Sets nanosecond lag bound.
    pub const fn with_max_lag_ns(mut self, max_lag_ns: u64) -> Self {
        self.max_lag_ns = max_lag_ns;
        self
    }

    /// Sets pending-byte bound.
    pub const fn with_max_bytes_pending(mut self, max_bytes_pending: u64) -> Self {
        self.max_bytes_pending = max_bytes_pending;
        self
    }

    /// Sets drop policy.
    pub const fn with_drop_policy(mut self, drop_policy: MarketDataBackpressureDropPolicy) -> Self {
        self.drop_policy = drop_policy;
        self
    }

    /// Sets minimum protected criticality.
    pub const fn with_protected_criticality(
        mut self,
        protected_criticality: MarketDataRecordCriticality,
    ) -> Self {
        self.protected_criticality = protected_criticality;
        self
    }

    /// Sets failure action for degraded persistence.
    pub const fn with_failure_action(
        mut self,
        failure_action: MarketDataPersistenceFailureAction,
    ) -> Self {
        self.failure_action = failure_action;
        self
    }
}

impl Default for MarketDataBackpressurePolicy {
    fn default() -> Self {
        Self::reject_new(0)
    }
}

/// Backpressure decision for one candidate market-data persistence record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataBackpressureDecision {
    /// Selected action.
    pub action: MarketDataBackpressureAction,
    /// Triggering reason.
    pub reason: MarketDataBackpressureReason,
    /// True when any backpressure condition was active.
    pub backpressured: bool,
    /// True when the selected action allows the candidate to be persisted.
    pub accepts_current: bool,
    /// True when the selected action drops a record.
    pub drops_record: bool,
    /// True when the selected action preserves trade records over lower-priority records.
    pub preserves_trade: bool,
}

impl MarketDataBackpressureDecision {
    /// Returns true when the decision is a hard stop instead of a drop/reject.
    pub const fn is_stop(self) -> bool {
        matches!(
            self.action,
            MarketDataBackpressureAction::StopMarketData
                | MarketDataBackpressureAction::StopTrading
                | MarketDataBackpressureAction::FailProcess
        )
    }
}

/// Configuration for [`FileMarketDataCheckpointStore`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MarketDataCheckpointConfig {
    root: PathBuf,
    retain_last: usize,
    sync_on_save: bool,
}

impl MarketDataCheckpointConfig {
    /// Creates checkpoint config rooted at `root`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            retain_last: 0,
            sync_on_save: false,
        }
    }

    /// Sets how many recent checkpoints to retain per venue/symbol.
    ///
    /// A value of `0` disables automatic pruning.
    pub const fn with_retain_last(mut self, retain_last: usize) -> Self {
        self.retain_last = retain_last;
        self
    }

    /// Sets whether each saved checkpoint calls `sync_data`.
    pub const fn with_sync_on_save(mut self, sync_on_save: bool) -> Self {
        self.sync_on_save = sync_on_save;
        self
    }

    /// Returns the configured checkpoint root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the number of recent checkpoints retained per venue/symbol.
    pub const fn retain_last(&self) -> usize {
        self.retain_last
    }

    /// Returns whether saved checkpoint files are synced before rename.
    pub const fn sync_on_save(&self) -> bool {
        self.sync_on_save
    }
}

/// Opaque market-data checkpoint payload and sequence anchors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataCheckpoint {
    /// Checkpoint identifier. Zero lets the file store assign the next id.
    pub id: MarketDataCheckpointId,
    /// Checkpoint payload category.
    pub kind: MarketDataCheckpointKind,
    /// Venue name associated with the checkpoint.
    pub venue: String,
    /// Symbol name associated with the checkpoint.
    pub symbol: String,
    /// Last applied market-data WAL sequence.
    pub wal_sequence: MarketDataWalSequence,
    /// Last applied provider-native sequence when known.
    pub provider_sequence: u64,
    /// Last applied normalized event sequence when known.
    pub event_sequence: u64,
    /// Checkpoint creation timestamp in nanoseconds since Unix epoch.
    pub created_ns: u64,
    /// User payload schema/version tag.
    pub payload_version: u32,
    /// Opaque encoded checkpoint payload.
    pub payload: Vec<u8>,
}

impl MarketDataCheckpoint {
    /// Creates an opaque checkpoint payload with sequence anchor.
    pub fn new(
        kind: MarketDataCheckpointKind,
        venue: impl Into<String>,
        symbol: impl Into<String>,
        wal_sequence: MarketDataWalSequence,
        payload: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            id: MarketDataCheckpointId(0),
            kind,
            venue: venue.into(),
            symbol: symbol.into(),
            wal_sequence,
            provider_sequence: 0,
            event_sequence: 0,
            created_ns: 0,
            payload_version: 1,
            payload: payload.into(),
        }
    }

    /// Sets checkpoint id.
    pub const fn with_id(mut self, id: MarketDataCheckpointId) -> Self {
        self.id = id;
        self
    }

    /// Sets provider-native sequence anchor.
    pub const fn with_provider_sequence(mut self, provider_sequence: u64) -> Self {
        self.provider_sequence = provider_sequence;
        self
    }

    /// Sets normalized event sequence anchor.
    pub const fn with_event_sequence(mut self, event_sequence: u64) -> Self {
        self.event_sequence = event_sequence;
        self
    }

    /// Sets creation timestamp in nanoseconds since Unix epoch.
    pub const fn with_created_ns(mut self, created_ns: u64) -> Self {
        self.created_ns = created_ns;
        self
    }

    /// Sets opaque payload version.
    pub const fn with_payload_version(mut self, payload_version: u32) -> Self {
        self.payload_version = payload_version;
        self
    }
}

/// Metadata for a persisted market-data checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataCheckpointManifest {
    /// Checkpoint identifier.
    pub id: MarketDataCheckpointId,
    /// Checkpoint payload category.
    pub kind: MarketDataCheckpointKind,
    /// Venue name associated with the checkpoint.
    pub venue: String,
    /// Symbol name associated with the checkpoint.
    pub symbol: String,
    /// Last applied market-data WAL sequence.
    pub wal_sequence: MarketDataWalSequence,
    /// Last applied provider-native sequence when known.
    pub provider_sequence: u64,
    /// Last applied normalized event sequence when known.
    pub event_sequence: u64,
    /// Checkpoint creation timestamp in nanoseconds since Unix epoch.
    pub created_ns: u64,
    /// User payload schema/version tag.
    pub payload_version: u32,
    /// Opaque payload byte length.
    pub payload_bytes: u64,
    /// Checksum over checkpoint header and payload.
    pub checksum: u32,
    /// Checkpoint file path.
    pub path: PathBuf,
}

/// Integrity report for one persisted market-data checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataCheckpointValidation {
    /// True when header, version, checksum, and payload length validate.
    pub valid: bool,
    /// Manifest decoded from the checkpoint header when available.
    pub manifest: Option<MarketDataCheckpointManifest>,
    /// Number of checksum failures.
    pub checksum_failures: u64,
    /// True when the checkpoint file ends before the declared payload length.
    pub truncated: bool,
}

impl Default for MarketDataCheckpointValidation {
    fn default() -> Self {
        Self {
            valid: true,
            manifest: None,
            checksum_failures: 0,
            truncated: false,
        }
    }
}

/// File-backed store for opaque market-data checkpoints.
#[derive(Debug, Clone)]
pub struct FileMarketDataCheckpointStore {
    config: MarketDataCheckpointConfig,
}

impl FileMarketDataCheckpointStore {
    /// Opens or creates a checkpoint store root.
    pub fn open(config: MarketDataCheckpointConfig) -> PersistResult<Self> {
        create_dir_all(&config.root)?;
        Ok(Self { config })
    }

    /// Returns the checkpoint store configuration.
    pub const fn config(&self) -> &MarketDataCheckpointConfig {
        &self.config
    }

    /// Saves a checkpoint and returns its manifest.
    ///
    /// When `checkpoint.id` is zero, the next id for the venue/symbol is
    /// assigned from existing checkpoint filenames.
    pub fn save_checkpoint(
        &self,
        checkpoint: &MarketDataCheckpoint,
    ) -> PersistResult<MarketDataCheckpointManifest> {
        let next_id = self.next_checkpoint_id(&checkpoint.venue, &checkpoint.symbol)?;
        let id = if checkpoint.id.0 == 0 {
            next_id
        } else if checkpoint.id < next_id {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "market-data checkpoint id must be greater than existing ids",
            )
            .into());
        } else {
            checkpoint.id
        };
        let created_ns = if checkpoint.created_ns == 0 {
            current_unix_nanos()
        } else {
            checkpoint.created_ns
        };
        let dir = self.symbol_checkpoint_dir(&checkpoint.venue, &checkpoint.symbol);
        create_dir_all(&dir)?;
        let path = checkpoint_path(&dir, id);
        if path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "market-data checkpoint id already exists",
            )
            .into());
        }
        let temp_path = checkpoint_temp_path(&dir, id);
        let frame = encode_market_data_checkpoint_frame(MarketDataCheckpointFrameInput {
            id,
            kind: checkpoint.kind,
            wal_sequence: checkpoint.wal_sequence,
            provider_sequence: checkpoint.provider_sequence,
            event_sequence: checkpoint.event_sequence,
            created_ns,
            payload_version: checkpoint.payload_version,
            payload: &checkpoint.payload,
        });

        {
            let mut file = File::create(&temp_path)?;
            file.write_all(&frame)?;
            if self.config.sync_on_save {
                file.sync_data()?;
            }
        }
        fs::rename(&temp_path, &path)?;

        if self.config.retain_last > 0 {
            self.prune_old(
                &checkpoint.venue,
                &checkpoint.symbol,
                self.config.retain_last,
            )?;
        }

        Ok(MarketDataCheckpointManifest {
            id,
            kind: checkpoint.kind,
            venue: checkpoint.venue.clone(),
            symbol: checkpoint.symbol.clone(),
            wal_sequence: checkpoint.wal_sequence,
            provider_sequence: checkpoint.provider_sequence,
            event_sequence: checkpoint.event_sequence,
            created_ns,
            payload_version: checkpoint.payload_version,
            payload_bytes: checkpoint.payload.len() as u64,
            checksum: read_u32(&frame[60..64]),
            path,
        })
    }

    /// Loads a checkpoint by id.
    pub fn load_checkpoint(
        &self,
        venue: &str,
        symbol: &str,
        id: MarketDataCheckpointId,
    ) -> PersistResult<MarketDataCheckpoint> {
        let path = checkpoint_path(&self.symbol_checkpoint_dir(venue, symbol), id);
        decode_market_data_checkpoint_file(&path, venue, symbol)
    }

    /// Loads the latest valid checkpoint, optionally filtered by kind.
    pub fn load_latest(
        &self,
        venue: &str,
        symbol: &str,
        kind: Option<MarketDataCheckpointKind>,
    ) -> PersistResult<Option<MarketDataCheckpoint>> {
        let mut ids = checkpoint_ids(&self.symbol_checkpoint_dir(venue, symbol))?;
        ids.sort_unstable_by(|left, right| right.cmp(left));
        for id in ids {
            let checkpoint = match self.load_checkpoint(venue, symbol, id) {
                Ok(checkpoint) => checkpoint,
                Err(_) => continue,
            };
            if kind.is_none_or(|expected| checkpoint.kind == expected) {
                return Ok(Some(checkpoint));
            }
        }
        Ok(None)
    }

    /// Lists checkpoint manifests for one venue/symbol ordered by id.
    pub fn list_checkpoints(
        &self,
        venue: &str,
        symbol: &str,
    ) -> PersistResult<Vec<MarketDataCheckpointManifest>> {
        let mut manifests = Vec::new();
        let dir = self.symbol_checkpoint_dir(venue, symbol);
        for id in checkpoint_ids(&dir)? {
            let path = checkpoint_path(&dir, id);
            manifests.push(read_market_data_checkpoint_manifest(&path, venue, symbol)?);
        }
        manifests.sort_unstable_by_key(|manifest| manifest.id);
        Ok(manifests)
    }

    /// Validates one checkpoint file.
    pub fn validate_checkpoint(
        &self,
        venue: &str,
        symbol: &str,
        id: MarketDataCheckpointId,
    ) -> PersistResult<MarketDataCheckpointValidation> {
        let path = checkpoint_path(&self.symbol_checkpoint_dir(venue, symbol), id);
        validate_market_data_checkpoint_file(&path, venue, symbol)
    }

    /// Prunes old checkpoints, keeping the newest `retain_last` by id.
    pub fn prune_old(&self, venue: &str, symbol: &str, retain_last: usize) -> PersistResult<usize> {
        if retain_last == 0 {
            return Ok(0);
        }
        let dir = self.symbol_checkpoint_dir(venue, symbol);
        let mut ids = checkpoint_ids(&dir)?;
        ids.sort_unstable();
        let prune_count = ids.len().saturating_sub(retain_last);
        for id in ids.into_iter().take(prune_count) {
            fs::remove_file(checkpoint_path(&dir, id))?;
        }
        Ok(prune_count)
    }

    fn next_checkpoint_id(
        &self,
        venue: &str,
        symbol: &str,
    ) -> PersistResult<MarketDataCheckpointId> {
        let max_id = checkpoint_ids(&self.symbol_checkpoint_dir(venue, symbol))?
            .into_iter()
            .map(|id| id.0)
            .max()
            .unwrap_or(0);
        Ok(MarketDataCheckpointId(max_id.saturating_add(1)))
    }

    fn symbol_checkpoint_dir(&self, venue: &str, symbol: &str) -> PathBuf {
        self.config
            .root
            .join(venue)
            .join(symbol)
            .join("checkpoints")
    }
}

#[derive(Debug, Clone, Copy)]
struct MarketDataCheckpointFrameInput<'a> {
    id: MarketDataCheckpointId,
    kind: MarketDataCheckpointKind,
    wal_sequence: MarketDataWalSequence,
    provider_sequence: u64,
    event_sequence: u64,
    created_ns: u64,
    payload_version: u32,
    payload: &'a [u8],
}

fn encode_market_data_checkpoint_frame(input: MarketDataCheckpointFrameInput<'_>) -> Vec<u8> {
    let mut frame = vec![0_u8; MARKET_DATA_CHECKPOINT_HEADER_LEN + input.payload.len()];
    frame[0..4].copy_from_slice(&MARKET_DATA_CHECKPOINT_MAGIC);
    write_u16(&mut frame[4..6], MARKET_DATA_CHECKPOINT_VERSION);
    write_u16(&mut frame[6..8], input.kind as u16);
    write_u64(&mut frame[8..16], input.id.0);
    write_u64(&mut frame[16..24], input.wal_sequence.0);
    write_u64(&mut frame[24..32], input.provider_sequence);
    write_u64(&mut frame[32..40], input.event_sequence);
    write_u64(&mut frame[40..48], input.created_ns);
    write_u32(&mut frame[48..52], input.payload_version);
    write_u64(&mut frame[52..60], input.payload.len() as u64);
    frame[MARKET_DATA_CHECKPOINT_HEADER_LEN..].copy_from_slice(input.payload);
    let checksum = market_data_checkpoint_checksum(&frame);
    write_u32(&mut frame[60..64], checksum);
    frame
}

fn decode_market_data_checkpoint_file(
    path: &Path,
    venue: &str,
    symbol: &str,
) -> PersistResult<MarketDataCheckpoint> {
    let mut file = File::open(path)?;
    let mut header = [0_u8; MARKET_DATA_CHECKPOINT_HEADER_LEN];
    let read = read_exact_or_tail(&mut file, &mut header)?;
    if read < MARKET_DATA_CHECKPOINT_HEADER_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "truncated market-data checkpoint header",
        )
        .into());
    }
    let manifest = decode_market_data_checkpoint_header(&header, path, venue, symbol)?;
    let payload_len = usize::try_from(manifest.payload_bytes).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "market-data checkpoint payload is too large for this platform",
        )
    })?;
    let mut payload = vec![0_u8; payload_len];
    let payload_read = read_exact_or_tail(&mut file, &mut payload)?;
    if payload_read < payload_len {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "truncated market-data checkpoint payload",
        )
        .into());
    }
    let mut frame = Vec::with_capacity(MARKET_DATA_CHECKPOINT_HEADER_LEN + payload_len);
    frame.extend_from_slice(&header);
    frame.extend_from_slice(&payload);
    if market_data_checkpoint_checksum(&frame) != manifest.checksum {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "market-data checkpoint checksum mismatch",
        )
        .into());
    }
    Ok(MarketDataCheckpoint {
        id: manifest.id,
        kind: manifest.kind,
        venue: manifest.venue,
        symbol: manifest.symbol,
        wal_sequence: manifest.wal_sequence,
        provider_sequence: manifest.provider_sequence,
        event_sequence: manifest.event_sequence,
        created_ns: manifest.created_ns,
        payload_version: manifest.payload_version,
        payload,
    })
}

fn read_market_data_checkpoint_manifest(
    path: &Path,
    venue: &str,
    symbol: &str,
) -> PersistResult<MarketDataCheckpointManifest> {
    let mut file = File::open(path)?;
    let mut header = [0_u8; MARKET_DATA_CHECKPOINT_HEADER_LEN];
    let read = read_exact_or_tail(&mut file, &mut header)?;
    if read < MARKET_DATA_CHECKPOINT_HEADER_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "truncated market-data checkpoint header",
        )
        .into());
    }
    decode_market_data_checkpoint_header(&header, path, venue, symbol)
}

fn validate_market_data_checkpoint_file(
    path: &Path,
    venue: &str,
    symbol: &str,
) -> PersistResult<MarketDataCheckpointValidation> {
    let mut file = File::open(path)?;
    let mut header = [0_u8; MARKET_DATA_CHECKPOINT_HEADER_LEN];
    let read = read_exact_or_tail(&mut file, &mut header)?;
    if read < MARKET_DATA_CHECKPOINT_HEADER_LEN {
        return Ok(MarketDataCheckpointValidation {
            valid: false,
            manifest: None,
            checksum_failures: 0,
            truncated: true,
        });
    }
    let manifest = match decode_market_data_checkpoint_header(&header, path, venue, symbol) {
        Ok(manifest) => manifest,
        Err(_) => {
            return Ok(MarketDataCheckpointValidation {
                valid: false,
                manifest: None,
                checksum_failures: 1,
                truncated: false,
            });
        }
    };
    let payload_len = usize::try_from(manifest.payload_bytes).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "market-data checkpoint payload is too large for this platform",
        )
    })?;
    let mut payload = vec![0_u8; payload_len];
    let payload_read = read_exact_or_tail(&mut file, &mut payload)?;
    if payload_read < payload_len {
        return Ok(MarketDataCheckpointValidation {
            valid: false,
            manifest: Some(manifest),
            checksum_failures: 0,
            truncated: true,
        });
    }
    let mut frame = Vec::with_capacity(MARKET_DATA_CHECKPOINT_HEADER_LEN + payload_len);
    frame.extend_from_slice(&header);
    frame.extend_from_slice(&payload);
    let checksum_failures = u64::from(market_data_checkpoint_checksum(&frame) != manifest.checksum);
    Ok(MarketDataCheckpointValidation {
        valid: checksum_failures == 0,
        manifest: Some(manifest),
        checksum_failures,
        truncated: false,
    })
}

fn decode_market_data_checkpoint_header(
    header: &[u8; MARKET_DATA_CHECKPOINT_HEADER_LEN],
    path: &Path,
    venue: &str,
    symbol: &str,
) -> PersistResult<MarketDataCheckpointManifest> {
    if header[0..4] != MARKET_DATA_CHECKPOINT_MAGIC
        || read_u16(&header[4..6]) != MARKET_DATA_CHECKPOINT_VERSION
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid market-data checkpoint header",
        )
        .into());
    }
    let kind = MarketDataCheckpointKind::from_u16(read_u16(&header[6..8])).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid market-data checkpoint kind",
        )
    })?;
    Ok(MarketDataCheckpointManifest {
        id: MarketDataCheckpointId(read_u64(&header[8..16])),
        kind,
        venue: venue.to_owned(),
        symbol: symbol.to_owned(),
        wal_sequence: MarketDataWalSequence(read_u64(&header[16..24])),
        provider_sequence: read_u64(&header[24..32]),
        event_sequence: read_u64(&header[32..40]),
        created_ns: read_u64(&header[40..48]),
        payload_version: read_u32(&header[48..52]),
        payload_bytes: read_u64(&header[52..60]),
        checksum: read_u32(&header[60..64]),
        path: path.to_path_buf(),
    })
}

fn checkpoint_ids(dir: &Path) -> PersistResult<Vec<MarketDataCheckpointId>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("ofmc") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(id) = stem.parse::<u64>() else {
            continue;
        };
        ids.push(MarketDataCheckpointId(id));
    }
    ids.sort_unstable();
    Ok(ids)
}

fn checkpoint_path(dir: &Path, id: MarketDataCheckpointId) -> PathBuf {
    dir.join(format!("{:020}.ofmc", id.0))
}

fn checkpoint_temp_path(dir: &Path, id: MarketDataCheckpointId) -> PathBuf {
    dir.join(format!("{:020}.ofmc.tmp", id.0))
}

fn market_data_checkpoint_checksum(frame: &[u8]) -> u32 {
    let mut hash = 0x811c9dc5_u32;
    for (idx, byte) in frame.iter().enumerate() {
        if (60..64).contains(&idx) {
            hash ^= 0;
        } else {
            hash ^= u32::from(*byte);
        }
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

fn current_unix_nanos() -> u64 {
    let Ok(duration) = SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return 0;
    };
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

/// Evaluates backpressure policy for one candidate persistence record.
pub fn evaluate_market_data_backpressure(
    policy: MarketDataBackpressurePolicy,
    health: &MarketDataPersistenceHealth,
    record_kind: MarketDataWalRecordKind,
    criticality: MarketDataRecordCriticality,
) -> MarketDataBackpressureDecision {
    let reason = active_backpressure_reason(policy, health);
    if reason == MarketDataBackpressureReason::None {
        return MarketDataBackpressureDecision {
            action: MarketDataBackpressureAction::Accept,
            reason,
            backpressured: false,
            accepts_current: true,
            drops_record: false,
            preserves_trade: false,
        };
    }
    let action = if reason == MarketDataBackpressureReason::Degraded {
        failure_action_to_backpressure_action(policy.failure_action)
    } else if criticality >= policy.protected_criticality {
        MarketDataBackpressureAction::Reject
    } else {
        drop_policy_to_action(policy.drop_policy, record_kind)
    };
    MarketDataBackpressureDecision {
        action,
        reason,
        backpressured: true,
        accepts_current: matches!(
            action,
            MarketDataBackpressureAction::Accept
                | MarketDataBackpressureAction::DropQueuedOldest
                | MarketDataBackpressureAction::DropQueuedLowestPriority
        ),
        drops_record: matches!(
            action,
            MarketDataBackpressureAction::DropCurrent
                | MarketDataBackpressureAction::DropQueuedOldest
                | MarketDataBackpressureAction::DropQueuedLowestPriority
        ),
        preserves_trade: action == MarketDataBackpressureAction::DropQueuedLowestPriority
            && matches!(
                policy.drop_policy,
                MarketDataBackpressureDropPolicy::PreserveTrades
            )
            && record_kind == MarketDataWalRecordKind::TradePrint,
    }
}

fn active_backpressure_reason(
    policy: MarketDataBackpressurePolicy,
    health: &MarketDataPersistenceHealth,
) -> MarketDataBackpressureReason {
    if health.degraded {
        return MarketDataBackpressureReason::Degraded;
    }
    if policy.max_queue_depth > 0 && health.queue_depth >= policy.max_queue_depth {
        return MarketDataBackpressureReason::QueueDepth;
    }
    if policy.max_records_lag > 0 && health.records_lag >= policy.max_records_lag {
        return MarketDataBackpressureReason::RecordsLag;
    }
    if policy.max_lag_ns > 0 && health.lag_ns >= policy.max_lag_ns {
        return MarketDataBackpressureReason::TimeLag;
    }
    if policy.max_bytes_pending > 0 && health.bytes_pending >= policy.max_bytes_pending {
        return MarketDataBackpressureReason::BytesPending;
    }
    MarketDataBackpressureReason::None
}

fn drop_policy_to_action(
    drop_policy: MarketDataBackpressureDropPolicy,
    record_kind: MarketDataWalRecordKind,
) -> MarketDataBackpressureAction {
    match drop_policy {
        MarketDataBackpressureDropPolicy::RejectNew => MarketDataBackpressureAction::Reject,
        MarketDataBackpressureDropPolicy::DropNewest => MarketDataBackpressureAction::DropCurrent,
        MarketDataBackpressureDropPolicy::DropOldest => {
            MarketDataBackpressureAction::DropQueuedOldest
        }
        MarketDataBackpressureDropPolicy::DropLowestPriority => {
            MarketDataBackpressureAction::DropQueuedLowestPriority
        }
        MarketDataBackpressureDropPolicy::PreserveTrades
            if record_kind == MarketDataWalRecordKind::TradePrint =>
        {
            MarketDataBackpressureAction::DropQueuedLowestPriority
        }
        MarketDataBackpressureDropPolicy::PreserveTrades => {
            MarketDataBackpressureAction::DropCurrent
        }
    }
}

fn failure_action_to_backpressure_action(
    failure_action: MarketDataPersistenceFailureAction,
) -> MarketDataBackpressureAction {
    match failure_action {
        MarketDataPersistenceFailureAction::MarkDegraded => MarketDataBackpressureAction::Reject,
        MarketDataPersistenceFailureAction::StopMarketData => {
            MarketDataBackpressureAction::StopMarketData
        }
        MarketDataPersistenceFailureAction::StopTrading => {
            MarketDataBackpressureAction::StopTrading
        }
        MarketDataPersistenceFailureAction::FailProcess => {
            MarketDataBackpressureAction::FailProcess
        }
        MarketDataPersistenceFailureAction::MemoryOnly => MarketDataBackpressureAction::MemoryOnly,
    }
}

/// Single-file binary WAL for normalized market-data events.
#[derive(Debug)]
pub struct MarketDataWal {
    path: PathBuf,
    file: File,
    sync_on_write: bool,
    next_sequence: MarketDataWalSequence,
    previous_checksum: u32,
    metrics: MarketDataWalMetrics,
}

impl MarketDataWal {
    /// Opens or creates a market-data WAL.
    ///
    /// Existing bytes are validated before append state is initialized.
    pub fn open(config: MarketDataWalConfig) -> PersistResult<Self> {
        if let Some(parent) = config.path.parent() {
            create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&config.path)?;
        let scan = scan_market_data_wal(&config.path, false)?;
        if !scan.report.valid {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "market-data WAL failed integrity validation",
            )
            .into());
        }
        Ok(Self {
            path: config.path,
            file,
            sync_on_write: config.sync_on_write,
            next_sequence: MarketDataWalSequence(
                scan.report
                    .last_sequence
                    .map_or(1, |sequence| sequence.0 + 1),
            ),
            previous_checksum: scan.previous_checksum,
            metrics: MarketDataWalMetrics::default(),
        })
    }

    /// Returns WAL path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns next sequence that will be assigned.
    pub const fn next_sequence(&self) -> MarketDataWalSequence {
        self.next_sequence
    }

    /// Returns metrics.
    pub const fn metrics(&self) -> MarketDataWalMetrics {
        self.metrics
    }

    /// Appends one encoded WAL record and returns its assigned sequence.
    pub fn append_record(
        &mut self,
        kind: MarketDataWalRecordKind,
        provider_sequence: u64,
        event_sequence: u64,
        ts_exchange_ns: u64,
        ts_recv_ns: u64,
        payload: &[u8],
    ) -> PersistResult<MarketDataWalSequence> {
        let sequence = self.next_sequence;
        let frame = encode_market_data_wal_frame(MarketDataWalFrameInput {
            sequence,
            kind,
            provider_sequence,
            event_sequence,
            ts_exchange_ns,
            ts_recv_ns,
            payload,
            previous_checksum: self.previous_checksum,
        })?;
        if let Err(err) = self.file.write_all(&frame) {
            self.metrics.write_failures = self.metrics.write_failures.saturating_add(1);
            return Err(err.into());
        }
        if self.sync_on_write {
            if let Err(err) = self.file.sync_data() {
                self.metrics.sync_failures = self.metrics.sync_failures.saturating_add(1);
                return Err(err.into());
            }
            self.metrics.sync_count = self.metrics.sync_count.saturating_add(1);
        }
        self.metrics.records_written = self.metrics.records_written.saturating_add(1);
        self.metrics.bytes_written = self
            .metrics
            .bytes_written
            .saturating_add(frame.len() as u64);
        self.previous_checksum = read_u32(&frame[52..56]);
        self.next_sequence = MarketDataWalSequence(self.next_sequence.0.saturating_add(1));
        Ok(sequence)
    }

    /// Replays all records into `out`.
    pub fn replay(
        &self,
        out: &mut Vec<MarketDataWalRecord>,
    ) -> PersistResult<MarketDataWalReplayResult> {
        replay_market_data_wal(&self.path, out)
    }

    /// Inspects a WAL path for integrity without materializing payloads.
    pub fn inspect_path(path: impl AsRef<Path>) -> PersistResult<MarketDataWalIntegrityReport> {
        Ok(scan_market_data_wal(path.as_ref(), false)?.report)
    }
}

#[derive(Debug, Default)]
struct MarketDataWalScan {
    report: MarketDataWalIntegrityReport,
    previous_checksum: u32,
}

#[derive(Debug, Clone, Copy)]
struct MarketDataWalFrameInput<'a> {
    sequence: MarketDataWalSequence,
    kind: MarketDataWalRecordKind,
    provider_sequence: u64,
    event_sequence: u64,
    ts_exchange_ns: u64,
    ts_recv_ns: u64,
    payload: &'a [u8],
    previous_checksum: u32,
}

fn encode_market_data_wal_frame(input: MarketDataWalFrameInput<'_>) -> PersistResult<Vec<u8>> {
    let payload = input.payload;
    if payload.len() > u32::MAX as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "market-data WAL payload is too large",
        )
        .into());
    }
    let mut frame = vec![0_u8; MARKET_DATA_WAL_HEADER_LEN + payload.len()];
    frame[0..4].copy_from_slice(&MARKET_DATA_WAL_MAGIC);
    write_u16(&mut frame[4..6], MARKET_DATA_WAL_VERSION);
    write_u16(&mut frame[6..8], input.kind as u16);
    write_u64(&mut frame[8..16], input.sequence.0);
    write_u64(&mut frame[16..24], input.provider_sequence);
    write_u64(&mut frame[24..32], input.event_sequence);
    write_u64(&mut frame[32..40], input.ts_exchange_ns);
    write_u64(&mut frame[40..48], input.ts_recv_ns);
    write_u32(&mut frame[48..52], payload.len() as u32);
    write_u32(&mut frame[56..60], input.previous_checksum);
    frame[MARKET_DATA_WAL_HEADER_LEN..].copy_from_slice(payload);
    let checksum = market_data_wal_checksum(&frame);
    write_u32(&mut frame[52..56], checksum);
    Ok(frame)
}

fn replay_market_data_wal(
    path: &Path,
    out: &mut Vec<MarketDataWalRecord>,
) -> PersistResult<MarketDataWalReplayResult> {
    let before = out.len();
    let scan = scan_market_data_wal_into(path, Some(out))?;
    let records = out.len().saturating_sub(before);
    Ok(MarketDataWalReplayResult {
        records,
        bytes: scan.report.bytes,
        first_sequence: out.get(before).map(|record| record.sequence),
        last_sequence: records
            .checked_sub(1)
            .and_then(|offset| out.get(before + offset))
            .map(|record| record.sequence),
    })
}

fn scan_market_data_wal(path: &Path, materialize: bool) -> PersistResult<MarketDataWalScan> {
    if materialize {
        let mut records = Vec::new();
        scan_market_data_wal_into(path, Some(&mut records))
    } else {
        scan_market_data_wal_into(path, None)
    }
}

fn scan_market_data_wal_into(
    path: &Path,
    mut out: Option<&mut Vec<MarketDataWalRecord>>,
) -> PersistResult<MarketDataWalScan> {
    if !path.exists() {
        return Ok(MarketDataWalScan {
            report: MarketDataWalIntegrityReport {
                valid: true,
                ..MarketDataWalIntegrityReport::default()
            },
            previous_checksum: 0,
        });
    }
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(0))?;
    let mut scan = MarketDataWalScan {
        report: MarketDataWalIntegrityReport {
            valid: true,
            ..MarketDataWalIntegrityReport::default()
        },
        previous_checksum: 0,
    };
    let mut expected_sequence = 1_u64;
    loop {
        let mut header = [0_u8; MARKET_DATA_WAL_HEADER_LEN];
        let read = read_exact_or_tail(&mut file, &mut header)?;
        if read == 0 {
            break;
        }
        if read < MARKET_DATA_WAL_HEADER_LEN {
            scan.report.valid = false;
            scan.report.truncated_tail = true;
            break;
        }
        if header[0..4] != MARKET_DATA_WAL_MAGIC
            || read_u16(&header[4..6]) != MARKET_DATA_WAL_VERSION
        {
            scan.report.valid = false;
            scan.report.checksum_failures = scan.report.checksum_failures.saturating_add(1);
            break;
        }
        let kind = MarketDataWalRecordKind::from_u16(read_u16(&header[6..8])).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid market-data WAL record kind",
            )
        })?;
        let sequence = read_u64(&header[8..16]);
        if sequence != expected_sequence {
            scan.report.valid = false;
            scan.report.sequence_failures = scan.report.sequence_failures.saturating_add(1);
        }
        let provider_sequence = read_u64(&header[16..24]);
        let event_sequence = read_u64(&header[24..32]);
        let ts_exchange_ns = read_u64(&header[32..40]);
        let ts_recv_ns = read_u64(&header[40..48]);
        let payload_len = read_u32(&header[48..52]) as usize;
        let expected_checksum = read_u32(&header[52..56]);
        let previous_checksum = read_u32(&header[56..60]);
        if previous_checksum != scan.previous_checksum {
            scan.report.valid = false;
            scan.report.checksum_failures = scan.report.checksum_failures.saturating_add(1);
        }
        let mut payload = vec![0_u8; payload_len];
        let payload_read = read_exact_or_tail(&mut file, &mut payload)?;
        if payload_read < payload_len {
            scan.report.valid = false;
            scan.report.truncated_tail = true;
            break;
        }
        let mut frame = Vec::with_capacity(MARKET_DATA_WAL_HEADER_LEN + payload_len);
        frame.extend_from_slice(&header);
        frame.extend_from_slice(&payload);
        let actual_checksum = market_data_wal_checksum(&frame);
        if actual_checksum != expected_checksum {
            scan.report.valid = false;
            scan.report.checksum_failures = scan.report.checksum_failures.saturating_add(1);
        }
        if let Some(records) = out.as_deref_mut() {
            records.push(MarketDataWalRecord {
                sequence: MarketDataWalSequence(sequence),
                kind,
                provider_sequence,
                event_sequence,
                ts_exchange_ns,
                ts_recv_ns,
                payload,
            });
        }
        scan.report.records = scan.report.records.saturating_add(1);
        scan.report.bytes = scan
            .report
            .bytes
            .saturating_add((MARKET_DATA_WAL_HEADER_LEN + payload_len) as u64);
        scan.report.last_sequence = Some(MarketDataWalSequence(sequence));
        scan.previous_checksum = expected_checksum;
        expected_sequence = sequence.saturating_add(1);
    }
    Ok(scan)
}

fn read_exact_or_tail(file: &mut File, buf: &mut [u8]) -> PersistResult<usize> {
    let mut offset = 0;
    while offset < buf.len() {
        match file.read(&mut buf[offset..]) {
            Ok(0) => break,
            Ok(n) => offset += n,
            Err(err) => return Err(err.into()),
        }
    }
    Ok(offset)
}

fn market_data_wal_checksum(frame: &[u8]) -> u32 {
    let mut hash = 0x811c9dc5_u32;
    for (idx, byte) in frame.iter().enumerate() {
        if (52..56).contains(&idx) {
            hash ^= 0;
        } else {
            hash ^= u32::from(*byte);
        }
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

fn write_u16(out: &mut [u8], value: u16) {
    out.copy_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut [u8], value: u32) {
    out.copy_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut [u8], value: u64) {
    out.copy_from_slice(&value.to_le_bytes());
}

/// Retention policy used by [`RollingStore`].
#[derive(Debug, Clone, Copy)]
pub struct RetentionPolicy {
    /// Maximum bytes to keep under store root (0 disables size pruning).
    pub max_total_bytes: u64,
    /// Maximum file age in seconds (0 disables age pruning).
    pub max_age_secs: u64,
}

/// JSONL rolling store for book/trade stream persistence.
#[derive(Debug, Clone)]
pub struct RollingStore {
    root: PathBuf,
    retention: Option<RetentionPolicy>,
}

/// Parsed book event read back from persisted JSONL storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBookEvent {
    /// Event sequence number.
    pub sequence: u64,
    /// Book side for the level update.
    pub side: Side,
    /// Price level index carried by the persisted update.
    pub level: u16,
    /// Price for the persisted update.
    pub price: i64,
    /// Size for the persisted update.
    pub size: i64,
    /// Book action recorded for the update.
    pub action: BookAction,
}

/// Parsed trade event read back from persisted JSONL storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTradeEvent {
    /// Event sequence number.
    pub sequence: u64,
    /// Trade price.
    pub price: i64,
    /// Trade size.
    pub size: i64,
    /// Aggressor side stored for the trade.
    pub aggressor_side: Side,
}

/// Merged persisted event used for replay-oriented symbol reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredEvent {
    /// Materialized book update record.
    Book(StoredBookEvent),
    /// Materialized trade record.
    Trade(StoredTradeEvent),
}

impl StoredEvent {
    /// Returns the persisted sequence number used for replay ordering.
    pub fn sequence(&self) -> u64 {
        match self {
            Self::Book(book) => book.sequence,
            Self::Trade(trade) => trade.sequence,
        }
    }
}

impl RollingStore {
    /// Creates a store rooted at `root`, creating directories as needed.
    pub fn new(root: impl AsRef<Path>) -> PersistResult<Self> {
        create_dir_all(root.as_ref())?;
        Ok(Self {
            root: root.as_ref().to_path_buf(),
            retention: None,
        })
    }

    /// Sets optional retention policy used after each append.
    pub fn with_retention(mut self, retention: Option<RetentionPolicy>) -> Self {
        self.retention = retention;
        self
    }

    /// Appends a single book event as JSON line.
    pub fn append_book(&self, event: &BookUpdate) -> PersistResult<()> {
        self.append_line(
            &event.symbol.venue,
            &event.symbol.symbol,
            "book",
            &format!(
                "{{\"schema\":{},\"seq\":{},\"side\":\"{:?}\",\"level\":{},\"price\":{},\"size\":{},\"action\":\"{:?}\",\"ts_exchange_ns\":{},\"ts_recv_ns\":{}}}",
                JSONL_SCHEMA_VERSION,
                event.sequence,
                event.side,
                event.level,
                event.price,
                event.size,
                event.action,
                event.ts_exchange_ns,
                event.ts_recv_ns
            ),
        )
    }

    /// Appends a single trade event as JSON line.
    pub fn append_trade(&self, event: &TradePrint) -> PersistResult<()> {
        self.append_line(
            &event.symbol.venue,
            &event.symbol.symbol,
            "trades",
            &format!(
                "{{\"schema\":{},\"seq\":{},\"price\":{},\"size\":{},\"aggressor\":\"{:?}\",\"ts_exchange_ns\":{},\"ts_recv_ns\":{}}}",
                JSONL_SCHEMA_VERSION,
                event.sequence,
                event.price,
                event.size,
                event.aggressor_side,
                event.ts_exchange_ns,
                event.ts_recv_ns
            ),
        )
    }

    /// Reads persisted book events for the given venue and symbol.
    ///
    /// Missing streams return an empty vector.
    pub fn read_books(&self, venue: &str, symbol: &str) -> PersistResult<Vec<StoredBookEvent>> {
        let path = self.stream_path(venue, symbol, "book");
        read_jsonl_stream(&path, parse_book_line)
    }

    /// Reads persisted book events filtered by an inclusive sequence range.
    ///
    /// `from_sequence` and `to_sequence` are optional inclusive bounds.
    pub fn read_books_in_range(
        &self,
        venue: &str,
        symbol: &str,
        from_sequence: Option<u64>,
        to_sequence: Option<u64>,
    ) -> PersistResult<Vec<StoredBookEvent>> {
        let events = self.read_books(venue, symbol)?;
        Ok(filter_by_sequence_range(events, from_sequence, to_sequence))
    }

    /// Reads persisted trade events for the given venue and symbol.
    ///
    /// Missing streams return an empty vector.
    pub fn read_trades(&self, venue: &str, symbol: &str) -> PersistResult<Vec<StoredTradeEvent>> {
        let path = self.stream_path(venue, symbol, "trades");
        read_jsonl_stream(&path, parse_trade_line)
    }

    /// Reads persisted trade events filtered by an inclusive sequence range.
    ///
    /// `from_sequence` and `to_sequence` are optional inclusive bounds.
    pub fn read_trades_in_range(
        &self,
        venue: &str,
        symbol: &str,
        from_sequence: Option<u64>,
        to_sequence: Option<u64>,
    ) -> PersistResult<Vec<StoredTradeEvent>> {
        let events = self.read_trades(venue, symbol)?;
        Ok(filter_by_sequence_range(events, from_sequence, to_sequence))
    }

    /// Reads and merges persisted book and trade events for the given venue and symbol.
    ///
    /// Events are ordered by ascending sequence number. When two events share the
    /// same sequence, book events are returned before trade events so replay order
    /// remains deterministic across runs.
    pub fn read_events(&self, venue: &str, symbol: &str) -> PersistResult<Vec<StoredEvent>> {
        let mut events = self
            .read_books(venue, symbol)?
            .into_iter()
            .map(StoredEvent::Book)
            .chain(
                self.read_trades(venue, symbol)?
                    .into_iter()
                    .map(StoredEvent::Trade),
            )
            .collect::<Vec<_>>();
        events.sort_by(|left, right| {
            left.sequence()
                .cmp(&right.sequence())
                .then_with(|| stored_event_kind_rank(left).cmp(&stored_event_kind_rank(right)))
        });
        Ok(events)
    }

    /// Reads merged persisted events filtered by an inclusive sequence range.
    ///
    /// `from_sequence` and `to_sequence` are optional inclusive bounds.
    pub fn read_events_in_range(
        &self,
        venue: &str,
        symbol: &str,
        from_sequence: Option<u64>,
        to_sequence: Option<u64>,
    ) -> PersistResult<Vec<StoredEvent>> {
        let events = self.read_events(venue, symbol)?;
        Ok(filter_by_sequence_range(events, from_sequence, to_sequence))
    }

    /// Lists venue directories currently present under the store root.
    ///
    /// The returned list is sorted for deterministic discovery and replay tooling.
    pub fn list_venues(&self) -> PersistResult<Vec<String>> {
        let mut venues = BTreeSet::new();
        for entry in read_dir_if_exists(&self.root)? {
            if entry.file_type()?.is_dir() {
                venues.insert(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(venues.into_iter().collect())
    }

    /// Lists symbol directories for a given venue currently present under the store root.
    ///
    /// Missing venues return an empty vector. The returned list is sorted for deterministic discovery.
    pub fn list_symbols(&self, venue: &str) -> PersistResult<Vec<String>> {
        let mut path = self.root.clone();
        path.push(venue);

        let mut symbols = BTreeSet::new();
        for entry in read_dir_if_exists(&path)? {
            if entry.file_type()?.is_dir() {
                symbols.insert(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(symbols.into_iter().collect())
    }

    /// Lists stream files currently present for a given venue and symbol.
    ///
    /// Returned names omit the `.jsonl` suffix and are sorted for deterministic replay tooling.
    /// Missing symbols return an empty vector.
    pub fn list_streams(&self, venue: &str, symbol: &str) -> PersistResult<Vec<String>> {
        let mut path = self.root.clone();
        path.push(venue);
        path.push(symbol);

        let mut streams = BTreeSet::new();
        for entry in read_dir_if_exists(&path)? {
            if !entry.file_type()?.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if let Some(stem) = name.strip_suffix(".jsonl") {
                streams.insert(stem.to_string());
            }
        }
        Ok(streams.into_iter().collect())
    }

    fn append_line(
        &self,
        venue: &str,
        symbol: &str,
        stream: &str,
        line: &str,
    ) -> PersistResult<()> {
        let mut dir = self.root.clone();
        dir.push(venue);
        dir.push(symbol);
        create_dir_all(&dir)?;

        let mut path = dir;
        path.push(format!("{stream}.jsonl"));

        let mut f = OpenOptions::new().create(true).append(true).open(path)?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;

        self.prune_if_needed()?;
        Ok(())
    }

    fn stream_path(&self, venue: &str, symbol: &str, stream: &str) -> PathBuf {
        let mut path = self.root.clone();
        path.push(venue);
        path.push(symbol);
        path.push(format!("{stream}.jsonl"));
        path
    }

    fn prune_if_needed(&self) -> PersistResult<()> {
        let Some(policy) = self.retention else {
            return Ok(());
        };

        let mut files = Vec::new();
        collect_files(&self.root, &mut files)?;

        if policy.max_age_secs > 0 {
            let now = SystemTime::now();
            for f in &files {
                let age = now
                    .duration_since(f.modified)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                if age > policy.max_age_secs {
                    let _ = fs::remove_file(&f.path);
                }
            }
            files.clear();
            collect_files(&self.root, &mut files)?;
        }

        if policy.max_total_bytes > 0 {
            let mut total: u64 = files.iter().map(|f| f.len).sum();
            if total > policy.max_total_bytes {
                files.sort_by_key(|f| f.modified);
                for f in files {
                    if total <= policy.max_total_bytes {
                        break;
                    }
                    if fs::remove_file(&f.path).is_ok() {
                        total = total.saturating_sub(f.len);
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct FileMeta {
    path: PathBuf,
    len: u64,
    modified: SystemTime,
}

fn collect_files(root: &Path, out: &mut Vec<FileMeta>) -> PersistResult<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let ty = entry.file_type()?;
        if ty.is_dir() {
            collect_files(&path, out)?;
        } else if ty.is_file() {
            let meta = entry.metadata()?;
            out.push(FileMeta {
                path,
                len: meta.len(),
                modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            });
        }
    }
    Ok(())
}

fn read_dir_if_exists(path: &Path) -> PersistResult<Vec<fs::DirEntry>> {
    match fs::read_dir(path) {
        Ok(dir) => dir.collect::<Result<Vec<_>, _>>().map_err(PersistError::Io),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(PersistError::Io(err)),
    }
}

#[derive(Debug, Deserialize)]
struct StoredBookEventWire {
    seq: u64,
    side: String,
    level: u16,
    price: i64,
    size: i64,
    action: String,
}

#[derive(Debug, Deserialize)]
struct StoredTradeEventWire {
    seq: u64,
    price: i64,
    size: i64,
    aggressor: String,
}

fn read_jsonl_stream<T>(
    path: &Path,
    parse_line: fn(&Path, usize, &str) -> PersistResult<T>,
) -> PersistResult<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(parse_line(path, line_no + 1, &line)?);
    }
    Ok(out)
}

fn parse_book_line(path: &Path, line_no: usize, line: &str) -> PersistResult<StoredBookEvent> {
    let raw: StoredBookEventWire = serde_json::from_str(line)
        .map_err(|err| invalid_data(path, line_no, format!("invalid book json: {err}")))?;
    Ok(StoredBookEvent {
        sequence: raw.seq,
        side: parse_side(path, line_no, "side", &raw.side)?,
        level: raw.level,
        price: raw.price,
        size: raw.size,
        action: parse_book_action(path, line_no, &raw.action)?,
    })
}

fn parse_trade_line(path: &Path, line_no: usize, line: &str) -> PersistResult<StoredTradeEvent> {
    let raw: StoredTradeEventWire = serde_json::from_str(line)
        .map_err(|err| invalid_data(path, line_no, format!("invalid trade json: {err}")))?;
    Ok(StoredTradeEvent {
        sequence: raw.seq,
        price: raw.price,
        size: raw.size,
        aggressor_side: parse_side(path, line_no, "aggressor", &raw.aggressor)?,
    })
}

fn parse_side(path: &Path, line_no: usize, field: &str, raw: &str) -> PersistResult<Side> {
    match raw {
        "Bid" => Ok(Side::Bid),
        "Ask" => Ok(Side::Ask),
        _ => Err(invalid_data(
            path,
            line_no,
            format!("invalid {field} value: {raw}"),
        )),
    }
}

fn parse_book_action(path: &Path, line_no: usize, raw: &str) -> PersistResult<BookAction> {
    match raw {
        "Upsert" => Ok(BookAction::Upsert),
        "Delete" => Ok(BookAction::Delete),
        _ => Err(invalid_data(
            path,
            line_no,
            format!("invalid action value: {raw}"),
        )),
    }
}

fn invalid_data(path: &Path, line_no: usize, message: String) -> PersistError {
    PersistError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("{}:{line_no}: {message}", path.display()),
    ))
}

fn stored_event_kind_rank(event: &StoredEvent) -> u8 {
    match event {
        StoredEvent::Book(_) => 0,
        StoredEvent::Trade(_) => 1,
    }
}

trait SequenceNumber {
    fn sequence(&self) -> u64;
}

impl SequenceNumber for StoredBookEvent {
    fn sequence(&self) -> u64 {
        self.sequence
    }
}

impl SequenceNumber for StoredTradeEvent {
    fn sequence(&self) -> u64 {
        self.sequence
    }
}

impl SequenceNumber for StoredEvent {
    fn sequence(&self) -> u64 {
        StoredEvent::sequence(self)
    }
}

fn filter_by_sequence_range<T>(
    events: Vec<T>,
    from_sequence: Option<u64>,
    to_sequence: Option<u64>,
) -> Vec<T>
where
    T: SequenceNumber,
{
    events
        .into_iter()
        .filter(|event| {
            let seq = event.sequence();
            from_sequence.is_none_or(|from| seq >= from) && to_sequence.is_none_or(|to| seq <= to)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use of_core::{BookAction, BookUpdate, Side, SymbolId};

    use super::*;

    #[test]
    fn prunes_by_total_size() {
        let root = temp_dir("persist_prune_size");
        let store = RollingStore::new(&root)
            .expect("store")
            .with_retention(Some(RetentionPolicy {
                max_total_bytes: 150,
                max_age_secs: 0,
            }));

        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        for seq in 0..20 {
            store
                .append_book(&BookUpdate {
                    symbol: symbol.clone(),
                    side: Side::Bid,
                    level: 0,
                    price: 100,
                    size: 1,
                    action: BookAction::Upsert,
                    sequence: seq,
                    ts_exchange_ns: 0,
                    ts_recv_ns: 0,
                })
                .expect("append");
        }

        let mut files = Vec::new();
        collect_files(&root, &mut files).expect("collect");
        let total: u64 = files.iter().map(|f| f.len).sum();
        assert!(total <= 150);
    }

    #[test]
    fn prunes_by_age() {
        let root = temp_dir("persist_prune_age");
        let old_path = root.join("old.jsonl");
        fs::write(&old_path, b"old").expect("write old");
        std::thread::sleep(std::time::Duration::from_millis(2200));

        let store = RollingStore::new(&root)
            .expect("store")
            .with_retention(Some(RetentionPolicy {
                max_total_bytes: 0,
                max_age_secs: 1,
            }));

        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_book(&BookUpdate {
                symbol,
                side: Side::Bid,
                level: 0,
                price: 100,
                size: 1,
                action: BookAction::Upsert,
                sequence: 1,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append");

        assert!(!old_path.exists());
    }

    #[test]
    fn reads_back_appended_book_and_trade_streams() {
        let root = temp_dir("persist_readback");
        let store = RollingStore::new(&root).expect("store");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_book(&BookUpdate {
                symbol: symbol.clone(),
                side: Side::Bid,
                level: 1,
                price: 505_000,
                size: 7,
                action: BookAction::Upsert,
                sequence: 10,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append book");
        store
            .append_trade(&TradePrint {
                symbol: symbol.clone(),
                price: 505_025,
                size: 3,
                aggressor_side: Side::Ask,
                sequence: 11,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append trade");

        let books = store
            .read_books(&symbol.venue, &symbol.symbol)
            .expect("read books");
        let trades = store
            .read_trades(&symbol.venue, &symbol.symbol)
            .expect("read trades");

        assert_eq!(
            books,
            vec![StoredBookEvent {
                sequence: 10,
                side: Side::Bid,
                level: 1,
                price: 505_000,
                size: 7,
                action: BookAction::Upsert,
            }]
        );
        assert_eq!(
            trades,
            vec![StoredTradeEvent {
                sequence: 11,
                price: 505_025,
                size: 3,
                aggressor_side: Side::Ask,
            }]
        );
    }

    #[test]
    fn writes_schema_metadata_without_breaking_readback() {
        let root = temp_dir("persist_schema_metadata");
        let store = RollingStore::new(&root).expect("store");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_trade(&TradePrint {
                symbol: symbol.clone(),
                price: 505_025,
                size: 3,
                aggressor_side: Side::Ask,
                sequence: 11,
                ts_exchange_ns: 123,
                ts_recv_ns: 456,
            })
            .expect("append trade");

        let raw = fs::read_to_string(root.join("CME").join("ESM6").join("trades.jsonl"))
            .expect("read raw stream");
        assert!(raw.contains("\"schema\":1"));
        assert!(raw.contains("\"ts_exchange_ns\":123"));
        assert!(raw.contains("\"ts_recv_ns\":456"));

        let trades = store
            .read_trades(&symbol.venue, &symbol.symbol)
            .expect("read trades");
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].sequence, 11);
    }

    #[test]
    fn reads_legacy_records_without_schema_metadata() {
        let root = temp_dir("persist_legacy_schema");
        let stream_dir = root.join("CME").join("ESM6");
        fs::create_dir_all(&stream_dir).expect("create dir");
        fs::write(
            stream_dir.join("trades.jsonl"),
            b"{\"seq\":11,\"price\":505025,\"size\":3,\"aggressor\":\"Ask\"}\n",
        )
        .expect("write legacy trade");

        let store = RollingStore::new(&root).expect("store");
        let trades = store
            .read_trades("CME", "ESM6")
            .expect("read legacy trades");

        assert_eq!(
            trades,
            vec![StoredTradeEvent {
                sequence: 11,
                price: 505_025,
                size: 3,
                aggressor_side: Side::Ask,
            }]
        );
    }

    #[test]
    fn missing_stream_reads_back_as_empty() {
        let root = temp_dir("persist_missing_stream");
        let store = RollingStore::new(&root).expect("store");

        let books = store.read_books("CME", "ESM6").expect("read books");
        let trades = store.read_trades("CME", "ESM6").expect("read trades");

        assert!(books.is_empty());
        assert!(trades.is_empty());
    }

    #[test]
    fn invalid_stream_data_returns_invalid_data_error() {
        let root = temp_dir("persist_invalid_stream");
        let stream_dir = root.join("CME").join("ESM6");
        fs::create_dir_all(&stream_dir).expect("create dir");
        fs::write(
            stream_dir.join("book.jsonl"),
            b"{\"seq\":1,\"side\":\"Middle\",\"level\":0,\"price\":1,\"size\":1,\"action\":\"Upsert\"}\n",
        )
        .expect("write");

        let store = RollingStore::new(&root).expect("store");
        let err = store.read_books("CME", "ESM6").expect_err("invalid data");

        match err {
            PersistError::Io(inner) => assert_eq!(inner.kind(), std::io::ErrorKind::InvalidData),
        }
    }

    #[test]
    fn reads_merged_symbol_events_in_sequence_order() {
        let root = temp_dir("persist_merged_readback");
        let store = RollingStore::new(&root).expect("store");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_trade(&TradePrint {
                symbol: symbol.clone(),
                price: 505_050,
                size: 2,
                aggressor_side: Side::Ask,
                sequence: 12,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append trade");
        store
            .append_book(&BookUpdate {
                symbol: symbol.clone(),
                side: Side::Bid,
                level: 0,
                price: 505_000,
                size: 10,
                action: BookAction::Upsert,
                sequence: 10,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append book");
        store
            .append_book(&BookUpdate {
                symbol: symbol.clone(),
                side: Side::Ask,
                level: 0,
                price: 505_075,
                size: 9,
                action: BookAction::Upsert,
                sequence: 12,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append book");

        let events = store
            .read_events(&symbol.venue, &symbol.symbol)
            .expect("read events");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].sequence(), 10);
        assert_eq!(events[1].sequence(), 12);
        assert_eq!(events[2].sequence(), 12);
        assert!(matches!(events[1], StoredEvent::Book(_)));
        assert!(matches!(events[2], StoredEvent::Trade(_)));
    }

    #[test]
    fn lists_venues_and_symbols_in_sorted_order() {
        let root = temp_dir("persist_discovery");
        fs::create_dir_all(root.join("BINANCE").join("BTCUSDT")).expect("btc dir");
        fs::create_dir_all(root.join("CME").join("NQM6")).expect("nq dir");
        fs::create_dir_all(root.join("CME").join("ESM6")).expect("es dir");

        let store = RollingStore::new(&root).expect("store");

        let venues = store.list_venues().expect("venues");
        let symbols = store.list_symbols("CME").expect("symbols");
        let missing = store.list_symbols("ICE").expect("missing");

        assert_eq!(venues, vec!["BINANCE".to_string(), "CME".to_string()]);
        assert_eq!(symbols, vec!["ESM6".to_string(), "NQM6".to_string()]);
        assert!(missing.is_empty());
    }

    #[test]
    fn lists_symbol_streams_without_suffixes() {
        let root = temp_dir("persist_stream_discovery");
        let stream_dir = root.join("CME").join("ESM6");
        fs::create_dir_all(&stream_dir).expect("stream dir");
        fs::write(stream_dir.join("book.jsonl"), b"{}\n").expect("write book");
        fs::write(stream_dir.join("trades.jsonl"), b"{}\n").expect("write trades");
        fs::write(stream_dir.join("notes.txt"), b"ignore").expect("write notes");

        let store = RollingStore::new(&root).expect("store");
        let streams = store.list_streams("CME", "ESM6").expect("streams");
        let missing = store.list_streams("CME", "NQM6").expect("missing");

        assert_eq!(streams, vec!["book".to_string(), "trades".to_string()]);
        assert!(missing.is_empty());
    }

    #[test]
    fn reads_range_filtered_events_inclusively() {
        let root = temp_dir("persist_range_filter");
        let store = RollingStore::new(&root).expect("store");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        for sequence in [10_u64, 11, 12] {
            store
                .append_trade(&TradePrint {
                    symbol: symbol.clone(),
                    price: 505000 + (sequence as i64),
                    size: 1,
                    aggressor_side: Side::Ask,
                    sequence,
                    ts_exchange_ns: 0,
                    ts_recv_ns: 0,
                })
                .expect("append trade");
        }

        let trades = store
            .read_trades_in_range(&symbol.venue, &symbol.symbol, Some(11), Some(12))
            .expect("trades in range");
        let events = store
            .read_events_in_range(&symbol.venue, &symbol.symbol, Some(10), Some(11))
            .expect("events in range");

        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].sequence, 11);
        assert_eq!(trades[1].sequence, 12);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence(), 10);
        assert_eq!(events[1].sequence(), 11);
    }

    #[test]
    fn market_data_wal_appends_and_replays_records() {
        let root = temp_dir("persist_market_data_wal");
        let path = root.join("normalized.wal");
        let mut wal = MarketDataWal::open(MarketDataWalConfig::new(&path)).expect("open wal");

        let first = wal
            .append_record(
                MarketDataWalRecordKind::TradePrint,
                10,
                20,
                30,
                40,
                b"trade",
            )
            .expect("append first");
        let second = wal
            .append_record(MarketDataWalRecordKind::BookUpdate, 11, 21, 31, 41, b"book")
            .expect("append second");

        assert_eq!(first, MarketDataWalSequence(1));
        assert_eq!(second, MarketDataWalSequence(2));
        assert_eq!(wal.metrics().records_written, 2);

        let mut records = Vec::new();
        let replay = wal.replay(&mut records).expect("replay");
        assert_eq!(replay.records, 2);
        assert_eq!(replay.first_sequence, Some(MarketDataWalSequence(1)));
        assert_eq!(replay.last_sequence, Some(MarketDataWalSequence(2)));
        assert_eq!(records[0].kind, MarketDataWalRecordKind::TradePrint);
        assert_eq!(records[0].payload, b"trade");
        assert_eq!(records[1].kind, MarketDataWalRecordKind::BookUpdate);
        assert_eq!(records[1].payload, b"book");
    }

    #[test]
    fn market_data_wal_reopens_after_valid_existing_records() {
        let root = temp_dir("persist_market_data_wal_reopen");
        let path = root.join("normalized.wal");
        {
            let mut wal = MarketDataWal::open(MarketDataWalConfig::new(&path)).expect("open wal");
            wal.append_record(MarketDataWalRecordKind::Heartbeat, 0, 0, 1, 2, b"")
                .expect("append");
        }

        let wal = MarketDataWal::open(MarketDataWalConfig::new(&path)).expect("reopen wal");
        assert_eq!(wal.next_sequence(), MarketDataWalSequence(2));
        let report = MarketDataWal::inspect_path(&path).expect("inspect");
        assert!(report.valid);
        assert_eq!(report.records, 1);
        assert_eq!(report.last_sequence, Some(MarketDataWalSequence(1)));
    }

    #[test]
    fn market_data_wal_detects_corruption() {
        let root = temp_dir("persist_market_data_wal_corrupt");
        let path = root.join("normalized.wal");
        let mut wal = MarketDataWal::open(MarketDataWalConfig::new(&path)).expect("open wal");
        wal.append_record(MarketDataWalRecordKind::GapMarker, 0, 9, 1, 2, b"gap")
            .expect("append");

        let mut bytes = fs::read(&path).expect("read wal");
        let last = bytes.last_mut().expect("last byte");
        *last ^= 0x01;
        fs::write(&path, bytes).expect("corrupt wal");

        let report = MarketDataWal::inspect_path(&path).expect("inspect");
        assert!(!report.valid);
        assert_eq!(report.checksum_failures, 1);
        assert!(MarketDataWal::open(MarketDataWalConfig::new(&path)).is_err());
    }

    #[test]
    fn market_data_checkpoint_store_saves_and_loads_payload() {
        let root = temp_dir("persist_market_data_checkpoint");
        let store = FileMarketDataCheckpointStore::open(MarketDataCheckpointConfig::new(&root))
            .expect("open checkpoint store");
        let checkpoint = MarketDataCheckpoint::new(
            MarketDataCheckpointKind::BookAndAnalytics,
            "CME",
            "ESZ6",
            MarketDataWalSequence(42),
            b"checkpoint-bytes".to_vec(),
        )
        .with_provider_sequence(100)
        .with_event_sequence(200)
        .with_created_ns(300)
        .with_payload_version(7);

        let manifest = store.save_checkpoint(&checkpoint).expect("save checkpoint");
        let loaded = store
            .load_checkpoint("CME", "ESZ6", manifest.id)
            .expect("load checkpoint");
        let validation = store
            .validate_checkpoint("CME", "ESZ6", manifest.id)
            .expect("validate checkpoint");

        assert_eq!(manifest.id, MarketDataCheckpointId(1));
        assert_eq!(manifest.kind, MarketDataCheckpointKind::BookAndAnalytics);
        assert_eq!(manifest.wal_sequence, MarketDataWalSequence(42));
        assert_eq!(manifest.payload_bytes, b"checkpoint-bytes".len() as u64);
        assert_eq!(loaded.payload, b"checkpoint-bytes");
        assert_eq!(loaded.provider_sequence, 100);
        assert_eq!(loaded.event_sequence, 200);
        assert!(validation.valid);
        assert_eq!(
            validation.manifest.as_ref().map(|manifest| manifest.id),
            Some(MarketDataCheckpointId(1))
        );
    }

    #[test]
    fn market_data_checkpoint_store_loads_latest_valid_by_kind() {
        let root = temp_dir("persist_market_data_checkpoint_latest");
        let store = FileMarketDataCheckpointStore::open(MarketDataCheckpointConfig::new(&root))
            .expect("open checkpoint store");
        store
            .save_checkpoint(&MarketDataCheckpoint::new(
                MarketDataCheckpointKind::Book,
                "CME",
                "NQZ6",
                MarketDataWalSequence(1),
                b"book-1".to_vec(),
            ))
            .expect("save book");
        store
            .save_checkpoint(&MarketDataCheckpoint::new(
                MarketDataCheckpointKind::Analytics,
                "CME",
                "NQZ6",
                MarketDataWalSequence(2),
                b"analytics-1".to_vec(),
            ))
            .expect("save analytics");
        store
            .save_checkpoint(&MarketDataCheckpoint::new(
                MarketDataCheckpointKind::Book,
                "CME",
                "NQZ6",
                MarketDataWalSequence(3),
                b"book-2".to_vec(),
            ))
            .expect("save second book");

        let latest_any = store
            .load_latest("CME", "NQZ6", None)
            .expect("latest any")
            .expect("checkpoint exists");
        let latest_analytics = store
            .load_latest("CME", "NQZ6", Some(MarketDataCheckpointKind::Analytics))
            .expect("latest analytics")
            .expect("analytics checkpoint exists");

        assert_eq!(latest_any.id, MarketDataCheckpointId(3));
        assert_eq!(latest_any.payload, b"book-2");
        assert_eq!(latest_analytics.id, MarketDataCheckpointId(2));
        assert_eq!(latest_analytics.payload, b"analytics-1");
    }

    #[test]
    fn market_data_checkpoint_validation_detects_corruption() {
        let root = temp_dir("persist_market_data_checkpoint_corrupt");
        let store = FileMarketDataCheckpointStore::open(MarketDataCheckpointConfig::new(&root))
            .expect("open checkpoint store");
        let manifest = store
            .save_checkpoint(&MarketDataCheckpoint::new(
                MarketDataCheckpointKind::SequenceState,
                "CME",
                "YMZ6",
                MarketDataWalSequence(9),
                b"sequence-state".to_vec(),
            ))
            .expect("save checkpoint");
        let mut bytes = fs::read(&manifest.path).expect("read checkpoint");
        let last = bytes.last_mut().expect("last byte");
        *last ^= 0x01;
        fs::write(&manifest.path, bytes).expect("corrupt checkpoint");

        let validation = store
            .validate_checkpoint("CME", "YMZ6", manifest.id)
            .expect("validate checkpoint");

        assert!(!validation.valid);
        assert_eq!(validation.checksum_failures, 1);
        assert!(store.load_checkpoint("CME", "YMZ6", manifest.id).is_err());
    }

    #[test]
    fn market_data_checkpoint_store_prunes_old_checkpoints() {
        let root = temp_dir("persist_market_data_checkpoint_prune");
        let store = FileMarketDataCheckpointStore::open(
            MarketDataCheckpointConfig::new(&root).with_retain_last(2),
        )
        .expect("open checkpoint store");
        for sequence in 1..=4 {
            store
                .save_checkpoint(&MarketDataCheckpoint::new(
                    MarketDataCheckpointKind::RuntimeState,
                    "CME",
                    "RTYZ6",
                    MarketDataWalSequence(sequence),
                    vec![sequence as u8],
                ))
                .expect("save checkpoint");
        }

        let manifests = store
            .list_checkpoints("CME", "RTYZ6")
            .expect("list checkpoints");

        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, MarketDataCheckpointId(3));
        assert_eq!(manifests[1].id, MarketDataCheckpointId(4));
    }

    #[test]
    fn market_data_checkpoint_store_rejects_non_monotonic_explicit_id() {
        let root = temp_dir("persist_market_data_checkpoint_monotonic");
        let store = FileMarketDataCheckpointStore::open(
            MarketDataCheckpointConfig::new(&root).with_retain_last(1),
        )
        .expect("open checkpoint store");
        store
            .save_checkpoint(&MarketDataCheckpoint::new(
                MarketDataCheckpointKind::RuntimeState,
                "CME",
                "MNQZ6",
                MarketDataWalSequence(1),
                b"one".to_vec(),
            ))
            .expect("save checkpoint");
        let err = store
            .save_checkpoint(
                &MarketDataCheckpoint::new(
                    MarketDataCheckpointKind::RuntimeState,
                    "CME",
                    "MNQZ6",
                    MarketDataWalSequence(2),
                    b"older-id".to_vec(),
                )
                .with_id(MarketDataCheckpointId(1)),
            )
            .expect_err("reject non-monotonic id");

        match err {
            PersistError::Io(err) => assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput),
        }
    }

    #[test]
    fn market_data_persistence_policy_reports_enabled_modes() {
        let disabled = MarketDataPersistencePolicy::default();
        let strict = MarketDataPersistencePolicy::inline_strict();
        let async_policy = MarketDataPersistencePolicy::bounded_async(1024)
            .with_failure_action(MarketDataPersistenceFailureAction::StopTrading);

        assert!(!disabled.enabled());
        assert!(strict.enabled());
        assert_eq!(strict.mode, MarketDataPersistenceMode::InlineStrict);
        assert!(async_policy.enabled());
        assert_eq!(async_policy.max_queue_depth, 1024);
        assert_eq!(
            async_policy.failure_action,
            MarketDataPersistenceFailureAction::StopTrading
        );
    }

    #[test]
    fn market_data_persistence_health_marks_failures_and_drops_degraded() {
        let policy = MarketDataPersistencePolicy::bounded_async(64);
        let metrics = MarketDataWalMetrics {
            write_failures: 1,
            ..MarketDataWalMetrics::default()
        };

        let health = MarketDataPersistenceHealth::from_wal_metrics(policy, metrics)
            .with_lag(3, 9, 100, 4096)
            .with_dropped_records(2)
            .with_error("disk full");

        assert!(health.enabled);
        assert!(health.degraded);
        assert!(!health.is_healthy());
        assert_eq!(health.queue_depth, 3);
        assert_eq!(health.records_lag, 9);
        assert_eq!(health.bytes_pending, 4096);
        assert_eq!(health.dropped_records, 2);
        assert_eq!(health.write_failures, 1);
        assert_eq!(health.last_error.as_deref(), Some("disk full"));
    }

    #[test]
    fn market_data_backpressure_accepts_when_under_limits() {
        let policy = MarketDataBackpressurePolicy::reject_new(8);
        let health = MarketDataPersistenceHealth {
            enabled: true,
            queue_depth: 4,
            ..MarketDataPersistenceHealth::default()
        };

        let decision = evaluate_market_data_backpressure(
            policy,
            &health,
            MarketDataWalRecordKind::BookUpdate,
            MarketDataRecordCriticality::Normal,
        );

        assert_eq!(decision.action, MarketDataBackpressureAction::Accept);
        assert!(!decision.backpressured);
        assert!(decision.accepts_current);
        assert!(!decision.drops_record);
    }

    #[test]
    fn market_data_backpressure_preserves_trades_under_queue_pressure() {
        let policy = MarketDataBackpressurePolicy::reject_new(8)
            .with_drop_policy(MarketDataBackpressureDropPolicy::PreserveTrades);
        let health = MarketDataPersistenceHealth {
            enabled: true,
            queue_depth: 8,
            ..MarketDataPersistenceHealth::default()
        };

        let trade_decision = evaluate_market_data_backpressure(
            policy,
            &health,
            MarketDataWalRecordKind::TradePrint,
            MarketDataRecordCriticality::Normal,
        );
        let book_decision = evaluate_market_data_backpressure(
            policy,
            &health,
            MarketDataWalRecordKind::BookUpdate,
            MarketDataRecordCriticality::Normal,
        );

        assert_eq!(
            trade_decision.action,
            MarketDataBackpressureAction::DropQueuedLowestPriority
        );
        assert!(trade_decision.preserves_trade);
        assert!(trade_decision.accepts_current);
        assert_eq!(
            book_decision.action,
            MarketDataBackpressureAction::DropCurrent
        );
        assert!(!book_decision.accepts_current);
    }

    #[test]
    fn market_data_backpressure_protects_critical_records() {
        let policy = MarketDataBackpressurePolicy::reject_new(1)
            .with_drop_policy(MarketDataBackpressureDropPolicy::DropNewest)
            .with_protected_criticality(MarketDataRecordCriticality::High);
        let health = MarketDataPersistenceHealth {
            enabled: true,
            queue_depth: 1,
            ..MarketDataPersistenceHealth::default()
        };

        let decision = evaluate_market_data_backpressure(
            policy,
            &health,
            MarketDataWalRecordKind::GapMarker,
            MarketDataRecordCriticality::High,
        );

        assert_eq!(decision.action, MarketDataBackpressureAction::Reject);
        assert!(decision.backpressured);
        assert!(!decision.drops_record);
    }

    #[test]
    fn market_data_backpressure_maps_degraded_failure_action() {
        let policy = MarketDataBackpressurePolicy::reject_new(8)
            .with_failure_action(MarketDataPersistenceFailureAction::StopTrading);
        let health = MarketDataPersistenceHealth {
            enabled: true,
            degraded: true,
            ..MarketDataPersistenceHealth::default()
        };

        let decision = evaluate_market_data_backpressure(
            policy,
            &health,
            MarketDataWalRecordKind::TradePrint,
            MarketDataRecordCriticality::Normal,
        );

        assert_eq!(decision.reason, MarketDataBackpressureReason::Degraded);
        assert_eq!(decision.action, MarketDataBackpressureAction::StopTrading);
        assert!(decision.is_stop());
    }

    fn temp_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{}_{}_{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock ok")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("temp dir");
        path
    }
}
