use super::*;

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

impl std::fmt::Display for PersistError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PersistError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
        }
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
    /// Materialized order-book snapshot marker payload.
    BookSnapshotMarker = 6,
    /// Data-quality flag transition payload.
    QualityFlag = 7,
    /// Adapter health transition payload.
    AdapterHealth = 8,
    /// Subscription lifecycle transition payload.
    SubscriptionState = 9,
    /// Explicit out-of-order event marker payload.
    OutOfOrderMarker = 10,
    /// Checkpoint boundary marker payload.
    CheckpointMarker = 11,
    /// Provider-native message captured before normalization.
    RawProviderMessage = 12,
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
    pub(crate) fn from_u16(value: u16) -> Option<Self> {
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
    pub(crate) fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::BookUpdate),
            2 => Some(Self::TradePrint),
            3 => Some(Self::Heartbeat),
            4 => Some(Self::GapMarker),
            5 => Some(Self::SegmentSeal),
            6 => Some(Self::BookSnapshotMarker),
            7 => Some(Self::QualityFlag),
            8 => Some(Self::AdapterHealth),
            9 => Some(Self::SubscriptionState),
            10 => Some(Self::OutOfOrderMarker),
            11 => Some(Self::CheckpointMarker),
            12 => Some(Self::RawProviderMessage),
            _ => None,
        }
    }
}

/// Configuration for [`MarketDataWal`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MarketDataWalConfig {
    pub(crate) path: PathBuf,
    pub(crate) sync_on_write: bool,
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

impl MarketDataWalRecord {
    /// Creates a decoded/replay record value.
    ///
    /// Writers still assign WAL sequence and checksum linkage. This constructor
    /// is intended for external exporters, replay fixtures, and format bridges.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sequence: MarketDataWalSequence,
        kind: MarketDataWalRecordKind,
        provider_sequence: u64,
        event_sequence: u64,
        ts_exchange_ns: u64,
        ts_recv_ns: u64,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            sequence,
            kind,
            provider_sequence,
            event_sequence,
            ts_exchange_ns,
            ts_recv_ns,
            payload,
        }
    }
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

/// Filter for deterministic market-data WAL replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataWalReplayFilter {
    /// Inclusive lower WAL sequence bound.
    pub from_sequence: Option<MarketDataWalSequence>,
    /// Inclusive upper WAL sequence bound.
    pub to_sequence: Option<MarketDataWalSequence>,
    /// Inclusive lower provider sequence bound.
    pub from_provider_sequence: Option<u64>,
    /// Inclusive upper provider sequence bound.
    pub to_provider_sequence: Option<u64>,
    /// Inclusive lower normalized event sequence bound.
    pub from_event_sequence: Option<u64>,
    /// Inclusive upper normalized event sequence bound.
    pub to_event_sequence: Option<u64>,
    /// Inclusive lower exchange timestamp bound.
    pub from_ts_exchange_ns: Option<u64>,
    /// Inclusive upper exchange timestamp bound.
    pub to_ts_exchange_ns: Option<u64>,
    /// Inclusive lower receive timestamp bound.
    pub from_ts_recv_ns: Option<u64>,
    /// Inclusive upper receive timestamp bound.
    pub to_ts_recv_ns: Option<u64>,
    /// Optional record kind filter.
    pub kind: Option<MarketDataWalRecordKind>,
}

impl MarketDataWalReplayFilter {
    /// Creates an empty filter that matches every record.
    pub const fn new() -> Self {
        Self {
            from_sequence: None,
            to_sequence: None,
            from_provider_sequence: None,
            to_provider_sequence: None,
            from_event_sequence: None,
            to_event_sequence: None,
            from_ts_exchange_ns: None,
            to_ts_exchange_ns: None,
            from_ts_recv_ns: None,
            to_ts_recv_ns: None,
            kind: None,
        }
    }

    /// Sets an inclusive WAL sequence range.
    pub const fn with_sequence_range(
        mut self,
        from_sequence: Option<MarketDataWalSequence>,
        to_sequence: Option<MarketDataWalSequence>,
    ) -> Self {
        self.from_sequence = from_sequence;
        self.to_sequence = to_sequence;
        self
    }

    /// Sets an inclusive provider sequence range.
    pub const fn with_provider_sequence_range(
        mut self,
        from_provider_sequence: Option<u64>,
        to_provider_sequence: Option<u64>,
    ) -> Self {
        self.from_provider_sequence = from_provider_sequence;
        self.to_provider_sequence = to_provider_sequence;
        self
    }

    /// Sets an inclusive normalized event sequence range.
    pub const fn with_event_sequence_range(
        mut self,
        from_event_sequence: Option<u64>,
        to_event_sequence: Option<u64>,
    ) -> Self {
        self.from_event_sequence = from_event_sequence;
        self.to_event_sequence = to_event_sequence;
        self
    }

    /// Sets an inclusive exchange timestamp range.
    pub const fn with_exchange_time_range(
        mut self,
        from_ts_exchange_ns: Option<u64>,
        to_ts_exchange_ns: Option<u64>,
    ) -> Self {
        self.from_ts_exchange_ns = from_ts_exchange_ns;
        self.to_ts_exchange_ns = to_ts_exchange_ns;
        self
    }

    /// Sets an inclusive receive timestamp range.
    pub const fn with_receive_time_range(
        mut self,
        from_ts_recv_ns: Option<u64>,
        to_ts_recv_ns: Option<u64>,
    ) -> Self {
        self.from_ts_recv_ns = from_ts_recv_ns;
        self.to_ts_recv_ns = to_ts_recv_ns;
        self
    }

    /// Sets an exact record kind filter.
    pub const fn with_kind(mut self, kind: Option<MarketDataWalRecordKind>) -> Self {
        self.kind = kind;
        self
    }

    pub(crate) fn matches(self, record: &MarketDataWalRecordHeader) -> bool {
        range_contains(self.from_sequence, self.to_sequence, record.sequence)
            && range_contains(
                self.from_provider_sequence,
                self.to_provider_sequence,
                record.provider_sequence,
            )
            && range_contains(
                self.from_event_sequence,
                self.to_event_sequence,
                record.event_sequence,
            )
            && range_contains(
                self.from_ts_exchange_ns,
                self.to_ts_exchange_ns,
                record.ts_exchange_ns,
            )
            && range_contains(self.from_ts_recv_ns, self.to_ts_recv_ns, record.ts_recv_ns)
            && self.kind.is_none_or(|kind| kind == record.kind)
    }
}
