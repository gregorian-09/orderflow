//! Bounded background writer for normalized market-data WAL records.

use super::{
    MarketDataWalRecordKind, MarketDataWalSequence, NormalizedMarketDataCodecError,
    NormalizedMarketDataRecordInput, PersistError, PersistResult, SegmentedMarketDataWal,
    SegmentedMarketDataWalConfig,
};
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

const DEFAULT_QUEUE_CAPACITY: usize = 4_096;
const DEFAULT_MAX_QUEUED_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_WRITER_THREAD_NAME: &str = "orderflow-market-data-wal";

/// Owned normalized record accepted by a bounded market-data WAL producer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketDataWalRecordInput {
    kind: MarketDataWalRecordKind,
    provider_sequence: u64,
    event_sequence: u64,
    ts_exchange_ns: u64,
    ts_recv_ns: u64,
    payload: Vec<u8>,
}

impl MarketDataWalRecordInput {
    /// Creates one owned record input.
    pub fn new(
        kind: MarketDataWalRecordKind,
        provider_sequence: u64,
        event_sequence: u64,
        ts_exchange_ns: u64,
        ts_recv_ns: u64,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            kind,
            provider_sequence,
            event_sequence,
            ts_exchange_ns,
            ts_recv_ns,
            payload,
        }
    }

    /// Returns record kind.
    pub const fn kind(&self) -> MarketDataWalRecordKind {
        self.kind
    }

    /// Returns provider sequence.
    pub const fn provider_sequence(&self) -> u64 {
        self.provider_sequence
    }

    /// Returns normalized event sequence.
    pub const fn event_sequence(&self) -> u64 {
        self.event_sequence
    }

    /// Returns exchange timestamp in nanoseconds.
    pub const fn ts_exchange_ns(&self) -> u64 {
        self.ts_exchange_ns
    }

    /// Returns receive timestamp in nanoseconds.
    pub const fn ts_recv_ns(&self) -> u64 {
        self.ts_recv_ns
    }

    /// Returns encoded payload bytes.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Returns encoded payload length.
    pub const fn payload_len(&self) -> usize {
        self.payload.len()
    }

    /// Consumes the input and returns its payload allocation.
    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }
}

/// Configuration for [`BoundedMarketDataWalWriter`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedMarketDataWriterConfig {
    queue_capacity: usize,
    max_queued_payload_bytes: usize,
    thread_name: String,
}

impl Default for BoundedMarketDataWriterConfig {
    fn default() -> Self {
        Self {
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            max_queued_payload_bytes: DEFAULT_MAX_QUEUED_PAYLOAD_BYTES,
            thread_name: DEFAULT_WRITER_THREAD_NAME.to_owned(),
        }
    }
}

impl BoundedMarketDataWriterConfig {
    /// Creates the default bounded writer configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets maximum queued record count.
    pub const fn with_queue_capacity(mut self, queue_capacity: usize) -> Self {
        self.queue_capacity = queue_capacity;
        self
    }

    /// Sets maximum aggregate payload bytes waiting in the queue.
    pub const fn with_max_queued_payload_bytes(mut self, max_queued_payload_bytes: usize) -> Self {
        self.max_queued_payload_bytes = max_queued_payload_bytes;
        self
    }

    /// Sets the background thread name.
    pub fn with_thread_name(mut self, thread_name: impl Into<String>) -> Self {
        self.thread_name = thread_name.into();
        self
    }

    /// Returns maximum queued record count.
    pub const fn queue_capacity(&self) -> usize {
        self.queue_capacity
    }

    /// Returns maximum aggregate queued payload bytes.
    pub const fn max_queued_payload_bytes(&self) -> usize {
        self.max_queued_payload_bytes
    }

    /// Returns background thread name.
    pub fn thread_name(&self) -> &str {
        &self.thread_name
    }
}

/// Nonblocking append failure that returns ownership of the rejected record.
#[derive(Debug)]
#[non_exhaustive]
pub enum MarketDataWriterTryError {
    /// Record-count capacity is exhausted.
    Full(MarketDataWalRecordInput),
    /// Aggregate queued payload-byte capacity is exhausted.
    BytesFull(MarketDataWalRecordInput),
    /// Payload exceeds the WAL record limit.
    PayloadTooLarge(MarketDataWalRecordInput),
    /// Segment seals are controlled by the WAL rotation lifecycle.
    ReservedRecordKind(MarketDataWalRecordInput),
    /// Writer no longer accepts records.
    Stopped(MarketDataWalRecordInput),
}

impl MarketDataWriterTryError {
    /// Returns the rejected input for retry, alternate persistence, or reuse.
    pub fn into_input(self) -> MarketDataWalRecordInput {
        match self {
            Self::Full(input)
            | Self::BytesFull(input)
            | Self::PayloadTooLarge(input)
            | Self::ReservedRecordKind(input)
            | Self::Stopped(input) => input,
        }
    }
}

impl fmt::Display for MarketDataWriterTryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Full(_) => "market-data WAL record queue is full",
            Self::BytesFull(_) => "market-data WAL payload-byte queue is full",
            Self::PayloadTooLarge(_) => "market-data WAL payload exceeds configured maximum",
            Self::ReservedRecordKind(_) => "market-data WAL record kind is writer-reserved",
            Self::Stopped(_) => "market-data WAL writer is stopped",
        };
        formatter.write_str(message)
    }
}

impl Error for MarketDataWriterTryError {}

/// Nonblocking typed-event admission failure that returns the canonical event.
#[derive(Debug)]
#[non_exhaustive]
pub enum NormalizedMarketDataWriterTryError {
    /// Record-count capacity is exhausted.
    Full(NormalizedMarketDataRecordInput),
    /// Aggregate queued payload-byte capacity is exhausted.
    BytesFull(NormalizedMarketDataRecordInput),
    /// Encoded event exceeds the WAL record limit.
    PayloadTooLarge(NormalizedMarketDataRecordInput),
    /// Event cannot be represented by the normalized payload codec.
    InvalidInput {
        /// Rejected canonical event.
        input: NormalizedMarketDataRecordInput,
        /// Codec validation failure.
        error: NormalizedMarketDataCodecError,
    },
    /// Writer no longer accepts records.
    Stopped(NormalizedMarketDataRecordInput),
}

impl NormalizedMarketDataWriterTryError {
    /// Returns the rejected canonical input.
    pub fn into_input(self) -> NormalizedMarketDataRecordInput {
        match self {
            Self::Full(input)
            | Self::BytesFull(input)
            | Self::PayloadTooLarge(input)
            | Self::Stopped(input)
            | Self::InvalidInput { input, .. } => input,
        }
    }

    /// Returns the codec cause for an invalid input.
    pub const fn codec_error(&self) -> Option<&NormalizedMarketDataCodecError> {
        match self {
            Self::InvalidInput { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl fmt::Display for NormalizedMarketDataWriterTryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(_) => formatter.write_str("normalized market-data record queue is full"),
            Self::BytesFull(_) => {
                formatter.write_str("normalized market-data payload-byte queue is full")
            }
            Self::PayloadTooLarge(_) => {
                formatter.write_str("normalized market-data payload exceeds configured maximum")
            }
            Self::InvalidInput { error, .. } => error.fmt(formatter),
            Self::Stopped(_) => formatter.write_str("normalized market-data writer is stopped"),
        }
    }
}

impl Error for NormalizedMarketDataWriterTryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.codec_error().map(|error| error as &dyn Error)
    }
}

/// Control-plane failure from a flush or shutdown barrier.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarketDataWriterControlError {
    /// Worker has stopped or disconnected.
    WorkerStopped,
    /// WAL append or sync failed.
    Persistence(String),
    /// Worker thread panicked outside its guarded loop.
    WorkerPanicked,
}

impl fmt::Display for MarketDataWriterControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkerStopped => formatter.write_str("market-data WAL worker stopped"),
            Self::Persistence(message) => {
                write!(formatter, "market-data WAL persistence failed: {message}")
            }
            Self::WorkerPanicked => formatter.write_str("market-data WAL worker panicked"),
        }
    }
}

impl Error for MarketDataWriterControlError {}

/// Lock-free metrics snapshot for the bounded writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct BoundedMarketDataWriterMetrics {
    /// Inputs accepted by the bounded queue.
    pub accepted_records: u64,
    /// Accepted payload bytes.
    pub accepted_payload_bytes: u64,
    /// Records successfully appended.
    pub written_records: u64,
    /// Successfully appended payload bytes.
    pub written_payload_bytes: u64,
    /// Highest local receive timestamp accepted by the queue.
    pub latest_accepted_ts_recv_ns: u64,
    /// Highest local receive timestamp successfully appended.
    pub latest_written_ts_recv_ns: u64,
    /// Event-time backlog between accepted and written receive timestamps.
    pub event_time_lag_ns: u64,
    /// Inputs abandoned after a worker failure.
    pub abandoned_records: u64,
    /// Payload bytes abandoned after a worker failure.
    pub abandoned_payload_bytes: u64,
    /// Current queued record count, excluding the record being written.
    pub queue_depth: usize,
    /// Highest observed queued record count.
    pub queue_high_watermark: usize,
    /// Current queued payload bytes, excluding the record being written.
    pub queued_payload_bytes: usize,
    /// Highest observed queued payload bytes.
    pub queued_payload_high_watermark: usize,
    /// Record-capacity rejections.
    pub full_rejections: u64,
    /// Payload-byte-capacity rejections.
    pub bytes_full_rejections: u64,
    /// Per-record payload-limit rejections.
    pub payload_too_large_rejections: u64,
    /// Writer-reserved record-kind rejections.
    pub reserved_kind_rejections: u64,
    /// Canonical events rejected because their identifiers are not encodable.
    pub invalid_normalized_rejections: u64,
    /// Rejections after stop or failure.
    pub stopped_rejections: u64,
    /// WAL append failures.
    pub write_failures: u64,
    /// WAL sync failures.
    pub sync_failures: u64,
    /// Successful explicit flush or shutdown sync barriers.
    pub flushes: u64,
    /// Most recent appended global WAL sequence.
    pub last_written_sequence: Option<MarketDataWalSequence>,
    /// Most recent sequence covered by a known sync barrier.
    pub last_synced_sequence: Option<MarketDataWalSequence>,
    /// Whether persistence has failed.
    pub degraded: bool,
    /// Whether the worker no longer accepts work.
    pub stopped: bool,
    /// Guarded worker panics.
    pub worker_panics: u64,
}

#[derive(Debug)]
struct WriterState {
    accepting: AtomicBool,
    stopped: AtomicBool,
    degraded: AtomicBool,
    active_submissions: AtomicUsize,
    queue_depth: AtomicUsize,
    queue_high_watermark: AtomicUsize,
    queued_payload_bytes: AtomicUsize,
    queued_payload_high_watermark: AtomicUsize,
    accepted_records: AtomicU64,
    accepted_payload_bytes: AtomicU64,
    written_records: AtomicU64,
    written_payload_bytes: AtomicU64,
    latest_accepted_ts_recv_ns: AtomicU64,
    latest_written_ts_recv_ns: AtomicU64,
    abandoned_records: AtomicU64,
    abandoned_payload_bytes: AtomicU64,
    full_rejections: AtomicU64,
    bytes_full_rejections: AtomicU64,
    payload_too_large_rejections: AtomicU64,
    reserved_kind_rejections: AtomicU64,
    invalid_normalized_rejections: AtomicU64,
    stopped_rejections: AtomicU64,
    write_failures: AtomicU64,
    sync_failures: AtomicU64,
    flushes: AtomicU64,
    last_written_sequence: AtomicU64,
    last_synced_sequence: AtomicU64,
    worker_panics: AtomicU64,
    last_error: Mutex<Option<String>>,
}

impl WriterState {
    fn new() -> Self {
        Self {
            accepting: AtomicBool::new(true),
            stopped: AtomicBool::new(false),
            degraded: AtomicBool::new(false),
            active_submissions: AtomicUsize::new(0),
            queue_depth: AtomicUsize::new(0),
            queue_high_watermark: AtomicUsize::new(0),
            queued_payload_bytes: AtomicUsize::new(0),
            queued_payload_high_watermark: AtomicUsize::new(0),
            accepted_records: AtomicU64::new(0),
            accepted_payload_bytes: AtomicU64::new(0),
            written_records: AtomicU64::new(0),
            written_payload_bytes: AtomicU64::new(0),
            latest_accepted_ts_recv_ns: AtomicU64::new(0),
            latest_written_ts_recv_ns: AtomicU64::new(0),
            abandoned_records: AtomicU64::new(0),
            abandoned_payload_bytes: AtomicU64::new(0),
            full_rejections: AtomicU64::new(0),
            bytes_full_rejections: AtomicU64::new(0),
            payload_too_large_rejections: AtomicU64::new(0),
            reserved_kind_rejections: AtomicU64::new(0),
            invalid_normalized_rejections: AtomicU64::new(0),
            stopped_rejections: AtomicU64::new(0),
            write_failures: AtomicU64::new(0),
            sync_failures: AtomicU64::new(0),
            flushes: AtomicU64::new(0),
            last_written_sequence: AtomicU64::new(0),
            last_synced_sequence: AtomicU64::new(0),
            worker_panics: AtomicU64::new(0),
            last_error: Mutex::new(None),
        }
    }

    fn metrics(&self) -> BoundedMarketDataWriterMetrics {
        let latest_accepted_ts_recv_ns = self.latest_accepted_ts_recv_ns.load(Ordering::Relaxed);
        let latest_written_ts_recv_ns = self.latest_written_ts_recv_ns.load(Ordering::Relaxed);
        BoundedMarketDataWriterMetrics {
            accepted_records: self.accepted_records.load(Ordering::Relaxed),
            accepted_payload_bytes: self.accepted_payload_bytes.load(Ordering::Relaxed),
            written_records: self.written_records.load(Ordering::Relaxed),
            written_payload_bytes: self.written_payload_bytes.load(Ordering::Relaxed),
            latest_accepted_ts_recv_ns,
            latest_written_ts_recv_ns,
            event_time_lag_ns: latest_accepted_ts_recv_ns.saturating_sub(latest_written_ts_recv_ns),
            abandoned_records: self.abandoned_records.load(Ordering::Relaxed),
            abandoned_payload_bytes: self.abandoned_payload_bytes.load(Ordering::Relaxed),
            queue_depth: self.queue_depth.load(Ordering::Acquire),
            queue_high_watermark: self.queue_high_watermark.load(Ordering::Relaxed),
            queued_payload_bytes: self.queued_payload_bytes.load(Ordering::Acquire),
            queued_payload_high_watermark: self
                .queued_payload_high_watermark
                .load(Ordering::Relaxed),
            full_rejections: self.full_rejections.load(Ordering::Relaxed),
            bytes_full_rejections: self.bytes_full_rejections.load(Ordering::Relaxed),
            payload_too_large_rejections: self.payload_too_large_rejections.load(Ordering::Relaxed),
            reserved_kind_rejections: self.reserved_kind_rejections.load(Ordering::Relaxed),
            invalid_normalized_rejections: self
                .invalid_normalized_rejections
                .load(Ordering::Relaxed),
            stopped_rejections: self.stopped_rejections.load(Ordering::Relaxed),
            write_failures: self.write_failures.load(Ordering::Relaxed),
            sync_failures: self.sync_failures.load(Ordering::Relaxed),
            flushes: self.flushes.load(Ordering::Relaxed),
            last_written_sequence: nonzero_sequence(
                self.last_written_sequence.load(Ordering::Acquire),
            ),
            last_synced_sequence: nonzero_sequence(
                self.last_synced_sequence.load(Ordering::Acquire),
            ),
            degraded: self.degraded.load(Ordering::Acquire),
            stopped: self.stopped.load(Ordering::Acquire),
            worker_panics: self.worker_panics.load(Ordering::Relaxed),
        }
    }

    fn set_error(&self, message: String) {
        *lock_unpoisoned(&self.last_error) = Some(message);
        self.degraded.store(true, Ordering::Release);
        self.accepting.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
enum WriterCommand {
    Append(MarketDataWalRecordInput),
    AppendNormalized {
        input: NormalizedMarketDataRecordInput,
        encoded_len: usize,
    },
    Flush(SyncSender<Result<(), MarketDataWriterControlError>>),
    Shutdown(SyncSender<Result<(), MarketDataWriterControlError>>),
    #[cfg(test)]
    Pause {
        entered: SyncSender<()>,
        resume: Receiver<()>,
    },
}

/// Cloneable nonblocking producer for a bounded market-data WAL writer.
#[derive(Debug, Clone)]
pub struct MarketDataWalProducer {
    sender: SyncSender<WriterCommand>,
    state: Arc<WriterState>,
    queue_capacity: usize,
    max_queued_payload_bytes: usize,
    max_record_payload_bytes: usize,
}

impl MarketDataWalProducer {
    /// Attempts to enqueue an owned record without blocking.
    ///
    /// On pressure or shutdown, the error returns the original input so callers
    /// can retry, route it elsewhere, or return its allocation to a pool.
    pub fn try_append_owned(
        &self,
        input: MarketDataWalRecordInput,
    ) -> Result<(), MarketDataWriterTryError> {
        let submission = SubmissionGuard::enter(&self.state);
        let input_ts_recv_ns = input.ts_recv_ns();
        if !self.state.accepting.load(Ordering::Acquire) {
            self.state
                .stopped_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(MarketDataWriterTryError::Stopped(input));
        }
        if input.payload_len() > self.max_record_payload_bytes {
            self.state
                .payload_too_large_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(MarketDataWriterTryError::PayloadTooLarge(input));
        }
        if input.kind() == MarketDataWalRecordKind::SegmentSeal {
            self.state
                .reserved_kind_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(MarketDataWriterTryError::ReservedRecordKind(input));
        }
        let payload_bytes = input.payload_len();
        let Some(queued_payload_bytes) = reserve_bounded(
            &self.state.queued_payload_bytes,
            payload_bytes,
            self.max_queued_payload_bytes,
        ) else {
            self.state
                .bytes_full_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(MarketDataWriterTryError::BytesFull(input));
        };
        let queue_depth = self.state.queue_depth.fetch_add(1, Ordering::AcqRel) + 1;

        match self.sender.try_send(WriterCommand::Append(input)) {
            Ok(()) => {
                self.state.accepted_records.fetch_add(1, Ordering::Relaxed);
                self.state
                    .accepted_payload_bytes
                    .fetch_add(payload_bytes as u64, Ordering::Relaxed);
                update_atomic_max_u64(&self.state.latest_accepted_ts_recv_ns, input_ts_recv_ns);
                update_high_watermark(&self.state.queue_high_watermark, queue_depth);
                update_high_watermark(
                    &self.state.queued_payload_high_watermark,
                    queued_payload_bytes,
                );
                drop(submission);
                Ok(())
            }
            Err(TrySendError::Full(WriterCommand::Append(input))) => {
                release_queue_reservation(&self.state, payload_bytes);
                self.state.full_rejections.fetch_add(1, Ordering::Relaxed);
                Err(MarketDataWriterTryError::Full(input))
            }
            Err(TrySendError::Disconnected(WriterCommand::Append(input))) => {
                release_queue_reservation(&self.state, payload_bytes);
                self.state.accepting.store(false, Ordering::Release);
                self.state.stopped.store(true, Ordering::Release);
                self.state
                    .stopped_rejections
                    .fetch_add(1, Ordering::Relaxed);
                Err(MarketDataWriterTryError::Stopped(input))
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                unreachable!("try_append_owned sends only append commands")
            }
        }
    }

    /// Copies and attempts to enqueue one record without blocking.
    ///
    /// Prefer [`Self::try_append_owned`] with pooled payload allocations on a
    /// latency-sensitive producer path.
    pub fn try_append_copy(
        &self,
        kind: MarketDataWalRecordKind,
        provider_sequence: u64,
        event_sequence: u64,
        ts_exchange_ns: u64,
        ts_recv_ns: u64,
        payload: &[u8],
    ) -> Result<(), MarketDataWriterTryError> {
        self.try_append_owned(MarketDataWalRecordInput::new(
            kind,
            provider_sequence,
            event_sequence,
            ts_exchange_ns,
            ts_recv_ns,
            payload.to_vec(),
        ))
    }

    /// Attempts to enqueue an owned canonical book/trade event without
    /// encoding or waiting on the caller thread.
    ///
    /// The writer thread encodes accepted events into reusable scratch storage.
    /// Every rejection returns the canonical event to the caller.
    pub fn try_append_normalized_owned(
        &self,
        input: NormalizedMarketDataRecordInput,
    ) -> Result<(), NormalizedMarketDataWriterTryError> {
        let submission = SubmissionGuard::enter(&self.state);
        let input_ts_recv_ns = input.ts_recv_ns();
        if !self.state.accepting.load(Ordering::Acquire) {
            self.state
                .stopped_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(NormalizedMarketDataWriterTryError::Stopped(input));
        }
        let payload_bytes = match input.encoded_len() {
            Ok(length) => length,
            Err(error) => {
                self.state
                    .invalid_normalized_rejections
                    .fetch_add(1, Ordering::Relaxed);
                return Err(NormalizedMarketDataWriterTryError::InvalidInput { input, error });
            }
        };
        if payload_bytes > self.max_record_payload_bytes {
            self.state
                .payload_too_large_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(NormalizedMarketDataWriterTryError::PayloadTooLarge(input));
        }
        let Some(queued_payload_bytes) = reserve_bounded(
            &self.state.queued_payload_bytes,
            payload_bytes,
            self.max_queued_payload_bytes,
        ) else {
            self.state
                .bytes_full_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(NormalizedMarketDataWriterTryError::BytesFull(input));
        };
        let queue_depth = self.state.queue_depth.fetch_add(1, Ordering::AcqRel) + 1;
        match self.sender.try_send(WriterCommand::AppendNormalized {
            input,
            encoded_len: payload_bytes,
        }) {
            Ok(()) => {
                self.state.accepted_records.fetch_add(1, Ordering::Relaxed);
                self.state
                    .accepted_payload_bytes
                    .fetch_add(payload_bytes as u64, Ordering::Relaxed);
                update_atomic_max_u64(&self.state.latest_accepted_ts_recv_ns, input_ts_recv_ns);
                update_high_watermark(&self.state.queue_high_watermark, queue_depth);
                update_high_watermark(
                    &self.state.queued_payload_high_watermark,
                    queued_payload_bytes,
                );
                drop(submission);
                Ok(())
            }
            Err(TrySendError::Full(WriterCommand::AppendNormalized { input, encoded_len })) => {
                debug_assert_eq!(encoded_len, payload_bytes);
                release_queue_reservation(&self.state, payload_bytes);
                self.state.full_rejections.fetch_add(1, Ordering::Relaxed);
                Err(NormalizedMarketDataWriterTryError::Full(input))
            }
            Err(TrySendError::Disconnected(WriterCommand::AppendNormalized {
                input,
                encoded_len,
            })) => {
                debug_assert_eq!(encoded_len, payload_bytes);
                release_queue_reservation(&self.state, payload_bytes);
                self.state.accepting.store(false, Ordering::Release);
                self.state.stopped.store(true, Ordering::Release);
                self.state
                    .stopped_rejections
                    .fetch_add(1, Ordering::Relaxed);
                Err(NormalizedMarketDataWriterTryError::Stopped(input))
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                unreachable!("typed admission sends only normalized commands")
            }
        }
    }

    /// Returns a lock-free writer metrics snapshot.
    pub fn metrics(&self) -> BoundedMarketDataWriterMetrics {
        self.state.metrics()
    }

    /// Returns the latest persistence failure text, if any.
    pub fn last_error(&self) -> Option<String> {
        lock_unpoisoned(&self.state.last_error).clone()
    }

    /// Returns configured record queue capacity.
    pub const fn queue_capacity(&self) -> usize {
        self.queue_capacity
    }

    /// Returns configured queued payload-byte capacity.
    pub const fn max_queued_payload_bytes(&self) -> usize {
        self.max_queued_payload_bytes
    }
}

/// Single-owner segmented WAL worker with bounded nonblocking producers.
#[derive(Debug)]
pub struct BoundedMarketDataWalWriter {
    producer: MarketDataWalProducer,
    worker: Option<JoinHandle<()>>,
}

impl BoundedMarketDataWalWriter {
    /// Opens the segmented WAL and starts its background writer.
    ///
    /// # Errors
    /// Returns an error for invalid bounds, WAL recovery failure, or thread
    /// creation failure.
    pub fn start(
        wal_config: SegmentedMarketDataWalConfig,
        writer_config: BoundedMarketDataWriterConfig,
    ) -> PersistResult<Self> {
        validate_writer_config(&writer_config)?;
        let max_record_payload_bytes = wal_config.max_payload_bytes();
        let wal = SegmentedMarketDataWal::open(wal_config)?;
        let (sender, receiver) = mpsc::sync_channel(writer_config.queue_capacity);
        let state = Arc::new(WriterState::new());
        let worker_state = Arc::clone(&state);
        let worker = thread::Builder::new()
            .name(writer_config.thread_name.clone())
            .spawn(move || guarded_worker_loop(wal, receiver, worker_state))?;
        Ok(Self {
            producer: MarketDataWalProducer {
                sender,
                state,
                queue_capacity: writer_config.queue_capacity,
                max_queued_payload_bytes: writer_config.max_queued_payload_bytes,
                max_record_payload_bytes,
            },
            worker: Some(worker),
        })
    }

    /// Returns a cloneable nonblocking producer handle.
    pub fn producer(&self) -> MarketDataWalProducer {
        self.producer.clone()
    }

    /// Attempts to enqueue an owned record without blocking.
    pub fn try_append_owned(
        &self,
        input: MarketDataWalRecordInput,
    ) -> Result<(), MarketDataWriterTryError> {
        self.producer.try_append_owned(input)
    }

    /// Copies and attempts to enqueue one record without blocking.
    pub fn try_append_copy(
        &self,
        kind: MarketDataWalRecordKind,
        provider_sequence: u64,
        event_sequence: u64,
        ts_exchange_ns: u64,
        ts_recv_ns: u64,
        payload: &[u8],
    ) -> Result<(), MarketDataWriterTryError> {
        self.producer.try_append_copy(
            kind,
            provider_sequence,
            event_sequence,
            ts_exchange_ns,
            ts_recv_ns,
            payload,
        )
    }

    /// Attempts to enqueue an owned canonical event for worker-side encoding.
    pub fn try_append_normalized_owned(
        &self,
        input: NormalizedMarketDataRecordInput,
    ) -> Result<(), NormalizedMarketDataWriterTryError> {
        self.producer.try_append_normalized_owned(input)
    }

    /// Waits for all earlier records and synchronizes the active segment.
    ///
    /// This is a blocking control-plane barrier and must not run on a hot-path
    /// producer thread.
    pub fn flush(&self) -> Result<(), MarketDataWriterControlError> {
        send_barrier(&self.producer.sender, WriterCommand::Flush)
    }

    /// Returns a lock-free writer metrics snapshot.
    pub fn metrics(&self) -> BoundedMarketDataWriterMetrics {
        self.producer.metrics()
    }

    /// Returns the latest persistence failure text, if any.
    pub fn last_error(&self) -> Option<String> {
        self.producer.last_error()
    }

    /// Stops admission, drains earlier submissions, syncs, and joins the worker.
    ///
    /// This is a blocking control-plane operation.
    pub fn shutdown(
        mut self,
    ) -> Result<BoundedMarketDataWriterMetrics, MarketDataWriterControlError> {
        self.producer
            .state
            .accepting
            .store(false, Ordering::Release);
        wait_for_submissions(&self.producer.state);
        let barrier_result = send_barrier(&self.producer.sender, WriterCommand::Shutdown);
        let join_result = self.join_worker();
        barrier_result?;
        join_result?;
        Ok(self.metrics())
    }

    fn join_worker(&mut self) -> Result<(), MarketDataWriterControlError> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        worker
            .join()
            .map_err(|_| MarketDataWriterControlError::WorkerPanicked)
    }

    #[cfg(test)]
    fn pause_worker(&self) -> SyncSender<()> {
        let (entered_tx, entered_rx) = mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = mpsc::sync_channel(0);
        self.producer
            .sender
            .send(WriterCommand::Pause {
                entered: entered_tx,
                resume: resume_rx,
            })
            .expect("send pause");
        entered_rx.recv().expect("worker entered pause");
        resume_tx
    }
}

impl Drop for BoundedMarketDataWalWriter {
    fn drop(&mut self) {
        self.producer
            .state
            .accepting
            .store(false, Ordering::Release);
    }
}

struct SubmissionGuard<'a> {
    state: &'a WriterState,
}

impl<'a> SubmissionGuard<'a> {
    fn enter(state: &'a WriterState) -> Self {
        state.active_submissions.fetch_add(1, Ordering::AcqRel);
        Self { state }
    }
}

impl Drop for SubmissionGuard<'_> {
    fn drop(&mut self) {
        self.state.active_submissions.fetch_sub(1, Ordering::AcqRel);
    }
}

fn guarded_worker_loop(
    wal: SegmentedMarketDataWal,
    receiver: Receiver<WriterCommand>,
    state: Arc<WriterState>,
) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        worker_loop(wal, &receiver, &state);
    }));
    if result.is_err() {
        state.worker_panics.fetch_add(1, Ordering::Relaxed);
        state.set_error("worker panicked".to_owned());
        wait_for_submissions(&state);
        drain_abandoned(&receiver, &state, "worker panicked");
    }
    state.accepting.store(false, Ordering::Release);
    state.stopped.store(true, Ordering::Release);
}

fn worker_loop(
    mut wal: SegmentedMarketDataWal,
    receiver: &Receiver<WriterCommand>,
    state: &WriterState,
) {
    let mut normalized_scratch = Vec::with_capacity(512);
    while let Ok(command) = receiver.recv() {
        match command {
            WriterCommand::Append(input) => {
                release_queue_reservation(state, input.payload_len());
                let payload_bytes = input.payload_len();
                let sync_count = wal.metrics().wal.sync_count;
                match wal.append_record(
                    input.kind,
                    input.provider_sequence,
                    input.event_sequence,
                    input.ts_exchange_ns,
                    input.ts_recv_ns,
                    &input.payload,
                ) {
                    Ok(sequence) => {
                        state.written_records.fetch_add(1, Ordering::Relaxed);
                        state
                            .written_payload_bytes
                            .fetch_add(payload_bytes as u64, Ordering::Relaxed);
                        state
                            .last_written_sequence
                            .store(sequence.0, Ordering::Release);
                        update_atomic_max_u64(&state.latest_written_ts_recv_ns, input.ts_recv_ns);
                        if wal.metrics().wal.sync_count > sync_count {
                            state
                                .last_synced_sequence
                                .store(sequence.0, Ordering::Release);
                        }
                    }
                    Err(error) => {
                        state.write_failures.fetch_add(1, Ordering::Relaxed);
                        let message = error.to_string();
                        state.set_error(message.clone());
                        wait_for_submissions(state);
                        drain_abandoned(receiver, state, &message);
                        return;
                    }
                }
            }
            WriterCommand::AppendNormalized {
                input,
                encoded_len: payload_bytes,
            } => {
                release_queue_reservation(state, payload_bytes);
                let kind = input.record_kind();
                let sequence = input.sequence();
                let ts_exchange_ns = input.ts_exchange_ns();
                let ts_recv_ns = input.ts_recv_ns();
                if let Err(error) = input.encode_into(&mut normalized_scratch) {
                    state.write_failures.fetch_add(1, Ordering::Relaxed);
                    let message = error.to_string();
                    state.set_error(message.clone());
                    wait_for_submissions(state);
                    drain_abandoned(receiver, state, &message);
                    return;
                }
                let sync_count = wal.metrics().wal.sync_count;
                match wal.append_record(
                    kind,
                    sequence,
                    sequence,
                    ts_exchange_ns,
                    ts_recv_ns,
                    &normalized_scratch,
                ) {
                    Ok(wal_sequence) => {
                        state.written_records.fetch_add(1, Ordering::Relaxed);
                        state
                            .written_payload_bytes
                            .fetch_add(payload_bytes as u64, Ordering::Relaxed);
                        state
                            .last_written_sequence
                            .store(wal_sequence.0, Ordering::Release);
                        update_atomic_max_u64(&state.latest_written_ts_recv_ns, ts_recv_ns);
                        if wal.metrics().wal.sync_count > sync_count {
                            state
                                .last_synced_sequence
                                .store(wal_sequence.0, Ordering::Release);
                        }
                    }
                    Err(error) => {
                        state.write_failures.fetch_add(1, Ordering::Relaxed);
                        let message = error.to_string();
                        state.set_error(message.clone());
                        wait_for_submissions(state);
                        drain_abandoned(receiver, state, &message);
                        return;
                    }
                }
            }
            WriterCommand::Flush(acknowledge) => {
                let result = sync_wal(&mut wal, state);
                let failed = result.is_err();
                let _ = acknowledge.send(result);
                if failed {
                    wait_for_submissions(state);
                    drain_abandoned(receiver, state, "flush failed");
                    return;
                }
            }
            WriterCommand::Shutdown(acknowledge) => {
                let result = sync_wal(&mut wal, state);
                let _ = acknowledge.send(result);
                return;
            }
            #[cfg(test)]
            WriterCommand::Pause { entered, resume } => {
                let _ = entered.send(());
                let _ = resume.recv();
            }
        }
    }
    let _ = sync_wal(&mut wal, state);
}

fn sync_wal(
    wal: &mut SegmentedMarketDataWal,
    state: &WriterState,
) -> Result<(), MarketDataWriterControlError> {
    match wal.sync_data() {
        Ok(()) => {
            state.flushes.fetch_add(1, Ordering::Relaxed);
            let sequence = state.last_written_sequence.load(Ordering::Acquire);
            state
                .last_synced_sequence
                .store(sequence, Ordering::Release);
            Ok(())
        }
        Err(error) => {
            let message = error.to_string();
            state.sync_failures.fetch_add(1, Ordering::Relaxed);
            state.set_error(message.clone());
            Err(MarketDataWriterControlError::Persistence(message))
        }
    }
}

fn drain_abandoned(receiver: &Receiver<WriterCommand>, state: &WriterState, message: &str) {
    loop {
        match receiver.try_recv() {
            Ok(WriterCommand::Append(input)) => {
                let payload_bytes = input.payload_len();
                release_queue_reservation(state, payload_bytes);
                state.abandoned_records.fetch_add(1, Ordering::Relaxed);
                state
                    .abandoned_payload_bytes
                    .fetch_add(payload_bytes as u64, Ordering::Relaxed);
            }
            Ok(WriterCommand::AppendNormalized {
                encoded_len: payload_bytes,
                ..
            }) => {
                release_queue_reservation(state, payload_bytes);
                state.abandoned_records.fetch_add(1, Ordering::Relaxed);
                state
                    .abandoned_payload_bytes
                    .fetch_add(payload_bytes as u64, Ordering::Relaxed);
            }
            Ok(WriterCommand::Flush(acknowledge)) | Ok(WriterCommand::Shutdown(acknowledge)) => {
                let _ = acknowledge.send(Err(MarketDataWriterControlError::Persistence(
                    message.to_owned(),
                )));
            }
            #[cfg(test)]
            Ok(WriterCommand::Pause { entered, .. }) => {
                let _ = entered.send(());
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
        }
    }
}

fn send_barrier(
    sender: &SyncSender<WriterCommand>,
    command: impl FnOnce(SyncSender<Result<(), MarketDataWriterControlError>>) -> WriterCommand,
) -> Result<(), MarketDataWriterControlError> {
    let (acknowledge_tx, acknowledge_rx) = mpsc::sync_channel(0);
    sender
        .send(command(acknowledge_tx))
        .map_err(|_| MarketDataWriterControlError::WorkerStopped)?;
    acknowledge_rx
        .recv()
        .map_err(|_| MarketDataWriterControlError::WorkerStopped)?
}

fn validate_writer_config(config: &BoundedMarketDataWriterConfig) -> PersistResult<()> {
    if config.queue_capacity == 0 {
        return Err(invalid_input("market-data WAL queue capacity is zero"));
    }
    if config.max_queued_payload_bytes == 0 {
        return Err(invalid_input(
            "market-data WAL queued payload byte capacity is zero",
        ));
    }
    if config.thread_name.is_empty() {
        return Err(invalid_input("market-data WAL worker thread name is empty"));
    }
    Ok(())
}

fn reserve_bounded(value: &AtomicUsize, amount: usize, maximum: usize) -> Option<usize> {
    let mut current = value.load(Ordering::Acquire);
    loop {
        let next = current.checked_add(amount)?;
        if next > maximum {
            return None;
        }
        match value.compare_exchange_weak(current, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return Some(next),
            Err(observed) => current = observed,
        }
    }
}

fn release_queue_reservation(state: &WriterState, payload_bytes: usize) {
    state.queue_depth.fetch_sub(1, Ordering::AcqRel);
    state
        .queued_payload_bytes
        .fetch_sub(payload_bytes, Ordering::AcqRel);
}

fn update_high_watermark(high: &AtomicUsize, candidate: usize) {
    let mut current = high.load(Ordering::Relaxed);
    while candidate > current {
        match high.compare_exchange_weak(current, candidate, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn update_atomic_max_u64(high: &AtomicU64, candidate: u64) {
    let mut current = high.load(Ordering::Relaxed);
    while candidate > current {
        match high.compare_exchange_weak(current, candidate, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn wait_for_submissions(state: &WriterState) {
    while state.active_submissions.load(Ordering::Acquire) != 0 {
        thread::yield_now();
    }
}

fn nonzero_sequence(value: u64) -> Option<MarketDataWalSequence> {
    (value != 0).then_some(MarketDataWalSequence(value))
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn invalid_input(message: &'static str) -> PersistError {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decode_normalized_market_data_record, MarketDataWalRecord, MarketDataWalSyncPolicy,
    };
    use of_core::{Side, SymbolId, TradePrint};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(1);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(name: &str) -> Self {
            let id = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("orderflow-{name}-{}-{id}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn wal_config(root: &Path) -> SegmentedMarketDataWalConfig {
        SegmentedMarketDataWalConfig::new(root)
            .with_max_segment_bytes(1024 * 1024)
            .with_max_payload_bytes(1024)
            .with_sync_policy(MarketDataWalSyncPolicy::Never)
            .with_sync_manifest(false)
    }

    fn input(event_sequence: u64, payload: &[u8]) -> MarketDataWalRecordInput {
        MarketDataWalRecordInput::new(
            MarketDataWalRecordKind::TradePrint,
            event_sequence,
            event_sequence,
            event_sequence * 10,
            event_sequence * 10 + 1,
            payload.to_vec(),
        )
    }

    #[test]
    fn writes_fifo_and_flushes_before_replay() {
        let root = TestRoot::new("bounded-writer-fifo");
        let wal_config = wal_config(&root.0);
        let writer = BoundedMarketDataWalWriter::start(
            wal_config.clone(),
            BoundedMarketDataWriterConfig::new()
                .with_queue_capacity(64)
                .with_max_queued_payload_bytes(4096),
        )
        .expect("start writer");
        for sequence in 1..=32 {
            writer
                .try_append_owned(input(sequence, &[sequence as u8]))
                .expect("enqueue");
        }
        writer.flush().expect("flush");
        let metrics = writer.metrics();
        assert_eq!(metrics.accepted_records, 32);
        assert_eq!(metrics.written_records, 32);
        assert_eq!(metrics.queue_depth, 0);
        assert_eq!(
            metrics.last_written_sequence,
            Some(MarketDataWalSequence(32))
        );
        assert_eq!(
            metrics.last_synced_sequence,
            Some(MarketDataWalSequence(32))
        );
        let shutdown_metrics = writer.shutdown().expect("shutdown");
        assert!(shutdown_metrics.stopped);
        assert_eq!(shutdown_metrics.flushes, 2);

        let wal = SegmentedMarketDataWal::open(wal_config).expect("reopen WAL");
        let mut records = Vec::<MarketDataWalRecord>::new();
        wal.replay(&mut records).expect("replay");
        assert_eq!(records.len(), 32);
        assert_eq!(
            records
                .iter()
                .map(|record| record.event_sequence)
                .collect::<Vec<_>>(),
            (1..=32).collect::<Vec<_>>()
        );
    }

    #[test]
    fn returns_owned_input_when_record_queue_is_full() {
        let root = TestRoot::new("bounded-writer-full");
        let writer = BoundedMarketDataWalWriter::start(
            wal_config(&root.0),
            BoundedMarketDataWriterConfig::new()
                .with_queue_capacity(1)
                .with_max_queued_payload_bytes(64),
        )
        .expect("start writer");
        let resume = writer.pause_worker();
        writer.try_append_owned(input(1, b"one")).expect("first");
        let rejected = writer
            .try_append_owned(input(2, b"two"))
            .expect_err("queue must be full");
        assert!(matches!(rejected, MarketDataWriterTryError::Full(_)));
        assert_eq!(rejected.into_input().payload(), b"two");
        let metrics = writer.metrics();
        assert_eq!(metrics.queue_depth, 1);
        assert_eq!(metrics.queue_high_watermark, 1);
        assert_eq!(metrics.full_rejections, 1);
        resume.send(()).expect("resume");
        writer.shutdown().expect("shutdown");
    }

    #[test]
    fn reports_event_time_backlog_without_reading_the_wall_clock() {
        let root = TestRoot::new("bounded-writer-event-time-lag");
        let writer = BoundedMarketDataWalWriter::start(
            wal_config(&root.0),
            BoundedMarketDataWriterConfig::new()
                .with_queue_capacity(1)
                .with_max_queued_payload_bytes(64),
        )
        .expect("start writer");
        let resume = writer.pause_worker();
        writer.try_append_owned(input(7, b"one")).expect("enqueue");

        let queued = writer.metrics();
        assert_eq!(queued.latest_accepted_ts_recv_ns, 71);
        assert_eq!(queued.latest_written_ts_recv_ns, 0);
        assert_eq!(queued.event_time_lag_ns, 71);

        resume.send(()).expect("resume");
        let drained = writer.shutdown().expect("shutdown");
        assert_eq!(drained.latest_accepted_ts_recv_ns, 71);
        assert_eq!(drained.latest_written_ts_recv_ns, 71);
        assert_eq!(drained.event_time_lag_ns, 0);
    }

    #[test]
    fn bounds_queued_payload_bytes_independently() {
        let root = TestRoot::new("bounded-writer-bytes");
        let writer = BoundedMarketDataWalWriter::start(
            wal_config(&root.0),
            BoundedMarketDataWriterConfig::new()
                .with_queue_capacity(2)
                .with_max_queued_payload_bytes(3),
        )
        .expect("start writer");
        let resume = writer.pause_worker();
        writer.try_append_owned(input(1, b"aa")).expect("first");
        let rejected = writer
            .try_append_owned(input(2, b"bb"))
            .expect_err("byte limit must reject");
        assert!(matches!(rejected, MarketDataWriterTryError::BytesFull(_)));
        assert_eq!(rejected.into_input().payload(), b"bb");
        let metrics = writer.metrics();
        assert_eq!(metrics.queued_payload_bytes, 2);
        assert_eq!(metrics.queued_payload_high_watermark, 2);
        assert_eq!(metrics.bytes_full_rejections, 1);
        resume.send(()).expect("resume");
        writer.shutdown().expect("shutdown");
    }

    #[test]
    fn rejects_oversized_and_reserved_inputs_before_enqueue() {
        let root = TestRoot::new("bounded-writer-input");
        let writer = BoundedMarketDataWalWriter::start(
            wal_config(&root.0).with_max_payload_bytes(3),
            BoundedMarketDataWriterConfig::new(),
        )
        .expect("start writer");
        let too_large = writer
            .try_append_owned(input(1, b"four"))
            .expect_err("oversized");
        assert!(matches!(
            too_large,
            MarketDataWriterTryError::PayloadTooLarge(_)
        ));
        assert_eq!(too_large.into_input().payload(), b"four");
        let seal = MarketDataWalRecordInput::new(
            MarketDataWalRecordKind::SegmentSeal,
            0,
            0,
            0,
            0,
            Vec::new(),
        );
        let reserved = writer.try_append_owned(seal).expect_err("reserved kind");
        assert!(matches!(
            reserved,
            MarketDataWriterTryError::ReservedRecordKind(_)
        ));
        let metrics = writer.metrics();
        assert_eq!(metrics.payload_too_large_rejections, 1);
        assert_eq!(metrics.reserved_kind_rejections, 1);
        assert_eq!(metrics.accepted_records, 0);
        writer.shutdown().expect("shutdown");
    }

    #[test]
    fn cloned_producer_observes_shutdown_and_returns_ownership() {
        let root = TestRoot::new("bounded-writer-stopped");
        let writer = BoundedMarketDataWalWriter::start(
            wal_config(&root.0),
            BoundedMarketDataWriterConfig::new(),
        )
        .expect("start writer");
        let producer = writer.producer();
        assert_eq!(producer.queue_capacity(), DEFAULT_QUEUE_CAPACITY);
        assert_eq!(
            producer.max_queued_payload_bytes(),
            DEFAULT_MAX_QUEUED_PAYLOAD_BYTES
        );
        writer.shutdown().expect("shutdown");

        let rejected = producer
            .try_append_owned(input(7, b"returned"))
            .expect_err("stopped");
        assert!(matches!(rejected, MarketDataWriterTryError::Stopped(_)));
        assert_eq!(rejected.into_input().payload(), b"returned");
        assert!(producer.metrics().stopped);
        assert_eq!(producer.metrics().stopped_rejections, 1);
    }

    #[test]
    fn supports_multiple_nonblocking_producers() {
        let root = TestRoot::new("bounded-writer-producers");
        let writer = BoundedMarketDataWalWriter::start(
            wal_config(&root.0),
            BoundedMarketDataWriterConfig::new()
                .with_queue_capacity(128)
                .with_max_queued_payload_bytes(4096),
        )
        .expect("start writer");
        let mut threads = Vec::new();
        for producer_id in 0..4_u64 {
            let producer = writer.producer();
            threads.push(thread::spawn(move || {
                for offset in 1..=16_u64 {
                    let sequence = producer_id * 100 + offset;
                    producer
                        .try_append_owned(input(sequence, b"x"))
                        .expect("enqueue from producer");
                }
            }));
        }
        for thread in threads {
            thread.join().expect("join producer");
        }
        writer.flush().expect("flush");
        assert_eq!(writer.metrics().written_records, 64);
        writer.shutdown().expect("shutdown");
    }

    #[test]
    fn validates_writer_configuration_before_opening_wal() {
        let root = TestRoot::new("bounded-writer-config");
        let zero_records = BoundedMarketDataWriterConfig::new().with_queue_capacity(0);
        assert!(BoundedMarketDataWalWriter::start(wal_config(&root.0), zero_records).is_err());
        assert!(!root.0.exists());

        let zero_bytes = BoundedMarketDataWriterConfig::new().with_max_queued_payload_bytes(0);
        assert!(BoundedMarketDataWalWriter::start(wal_config(&root.0), zero_bytes).is_err());

        let empty_name = BoundedMarketDataWriterConfig::new().with_thread_name("");
        assert!(BoundedMarketDataWalWriter::start(wal_config(&root.0), empty_name).is_err());
    }

    #[test]
    fn input_accessors_and_payload_reuse_are_lossless() {
        let input = MarketDataWalRecordInput::new(
            MarketDataWalRecordKind::BookUpdate,
            11,
            12,
            13,
            14,
            vec![1, 2, 3],
        );
        assert_eq!(input.kind(), MarketDataWalRecordKind::BookUpdate);
        assert_eq!(input.provider_sequence(), 11);
        assert_eq!(input.event_sequence(), 12);
        assert_eq!(input.ts_exchange_ns(), 13);
        assert_eq!(input.ts_recv_ns(), 14);
        assert_eq!(input.payload_len(), 3);
        assert_eq!(input.into_payload(), vec![1, 2, 3]);
    }

    #[test]
    fn worker_encodes_owned_normalized_events_for_replay() {
        let root = TestRoot::new("bounded-writer-normalized");
        let writer = BoundedMarketDataWalWriter::start(
            wal_config(&root.0),
            BoundedMarketDataWriterConfig::new(),
        )
        .expect("start writer");
        writer
            .try_append_normalized_owned(NormalizedMarketDataRecordInput::trade(TradePrint {
                symbol: SymbolId {
                    venue: "CME".to_owned(),
                    symbol: "ESM6".to_owned(),
                },
                price: 5_050_000,
                size: 7,
                aggressor_side: Side::Ask,
                sequence: 41,
                ts_exchange_ns: 43,
                ts_recv_ns: 47,
            }))
            .expect("enqueue normalized trade");
        let metrics = writer.shutdown().expect("shutdown");
        assert_eq!(metrics.accepted_records, 1);
        assert_eq!(metrics.written_records, 1);
        assert_eq!(metrics.latest_accepted_ts_recv_ns, 47);
        assert_eq!(metrics.latest_written_ts_recv_ns, 47);
        assert_eq!(metrics.event_time_lag_ns, 0);

        let wal = SegmentedMarketDataWal::open(wal_config(&root.0)).expect("reopen");
        let mut records = Vec::new();
        wal.replay(&mut records).expect("replay");
        let record = records
            .iter()
            .find(|record| record.kind == MarketDataWalRecordKind::TradePrint)
            .expect("trade frame");
        let decoded = decode_normalized_market_data_record(record).expect("decode");
        match decoded {
            NormalizedMarketDataRecordInput::Trade { event: trade, .. } => {
                assert_eq!(trade.symbol.venue, "CME");
                assert_eq!(trade.symbol.symbol, "ESM6");
                assert_eq!(trade.sequence, 41);
                assert_eq!(trade.price, 5_050_000);
                assert_eq!(trade.size, 7);
            }
            _ => panic!("expected trade"),
        }
    }

    #[test]
    fn typed_admission_returns_invalid_canonical_event() {
        let root = TestRoot::new("bounded-writer-normalized-invalid");
        let writer = BoundedMarketDataWalWriter::start(
            wal_config(&root.0),
            BoundedMarketDataWriterConfig::new(),
        )
        .expect("start writer");
        let input = NormalizedMarketDataRecordInput::trade(TradePrint {
            symbol: SymbolId {
                venue: "X".repeat(u16::MAX as usize + 1),
                symbol: "Y".to_owned(),
            },
            price: 1,
            size: 1,
            aggressor_side: Side::Bid,
            sequence: 1,
            ts_exchange_ns: 1,
            ts_recv_ns: 1,
        });
        let error = writer
            .try_append_normalized_owned(input)
            .expect_err("invalid identifier");
        assert_eq!(
            error.codec_error(),
            Some(&NormalizedMarketDataCodecError::VenueTooLong)
        );
        match error.into_input() {
            NormalizedMarketDataRecordInput::Trade { event: trade, .. } => {
                assert_eq!(trade.symbol.venue.len(), u16::MAX as usize + 1);
            }
            _ => panic!("expected returned trade"),
        }
        assert_eq!(writer.metrics().invalid_normalized_rejections, 1);
        writer.shutdown().expect("shutdown");
    }
}
