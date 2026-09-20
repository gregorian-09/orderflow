use super::*;

/// Result type for verified Parquet export operations.
pub type MarketDataParquetResult<T> = Result<T, MarketDataParquetError>;

/// Verified Parquet export failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum MarketDataParquetError {
    /// Filesystem operation failed.
    Io(io::Error),
    /// Arrow schema or record-batch construction failed.
    Arrow(arrow_schema::ArrowError),
    /// Parquet encoding, metadata, or decoding failed.
    Parquet(ParquetError),
    /// Underlying market-data WAL replay failed.
    Persist(of_persist::PersistError),
    /// Normalized event payload failed strict decoding.
    Normalized(NormalizedMarketDataCodecError),
    /// Export configuration is invalid.
    InvalidConfig(String),
    /// Partition or source metadata is invalid.
    InvalidMetadata(String),
    /// Derived snapshots violate ordering or range requirements.
    InvalidDerivedSnapshots(String),
    /// A reopened export does not match its expected proof.
    Verification(String),
}

impl fmt::Display for MarketDataParquetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Arrow(error) => write!(formatter, "Arrow error: {error}"),
            Self::Parquet(error) => write!(formatter, "Parquet error: {error}"),
            Self::Persist(error) => write!(formatter, "WAL replay error: {error}"),
            Self::Normalized(error) => write!(formatter, "normalized payload error: {error}"),
            Self::InvalidConfig(message) => write!(formatter, "invalid export config: {message}"),
            Self::InvalidMetadata(message) => write!(formatter, "invalid metadata: {message}"),
            Self::InvalidDerivedSnapshots(message) => {
                write!(formatter, "invalid derived snapshots: {message}")
            }
            Self::Verification(message) => write!(formatter, "verification failed: {message}"),
        }
    }
}

impl Error for MarketDataParquetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Arrow(error) => Some(error),
            Self::Parquet(error) => Some(error),
            Self::Persist(error) => Some(error),
            Self::Normalized(error) => Some(error),
            Self::InvalidConfig(_)
            | Self::InvalidMetadata(_)
            | Self::InvalidDerivedSnapshots(_)
            | Self::Verification(_) => None,
        }
    }
}

impl From<io::Error> for MarketDataParquetError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<arrow_schema::ArrowError> for MarketDataParquetError {
    fn from(value: arrow_schema::ArrowError) -> Self {
        Self::Arrow(value)
    }
}

impl From<ParquetError> for MarketDataParquetError {
    fn from(value: ParquetError) -> Self {
        Self::Parquet(value)
    }
}

impl From<of_persist::PersistError> for MarketDataParquetError {
    fn from(value: of_persist::PersistError) -> Self {
        Self::Persist(value)
    }
}

impl From<NormalizedMarketDataCodecError> for MarketDataParquetError {
    fn from(value: NormalizedMarketDataCodecError) -> Self {
        Self::Normalized(value)
    }
}

/// Compression used for a Parquet export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MarketDataParquetCompression {
    /// No column compression.
    Uncompressed,
    /// Snappy compression for broad reader interoperability.
    Snappy,
    /// Zstandard compression at the Apache Arrow default level.
    #[default]
    Zstd,
}

/// Configuration for [`MarketDataParquetWriter`].
#[derive(Debug, Clone)]
pub struct MarketDataParquetExportConfig {
    pub(crate) root: PathBuf,
    pub(crate) compression: MarketDataParquetCompression,
    pub(crate) batch_rows: usize,
    pub(crate) row_group_rows: usize,
    pub(crate) sync_on_write: bool,
}

impl MarketDataParquetExportConfig {
    /// Creates export configuration rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            compression: MarketDataParquetCompression::default(),
            batch_rows: DEFAULT_BATCH_ROWS,
            row_group_rows: DEFAULT_ROW_GROUP_ROWS,
            sync_on_write: true,
        }
    }

    /// Sets Parquet column compression.
    pub const fn with_compression(mut self, compression: MarketDataParquetCompression) -> Self {
        self.compression = compression;
        self
    }

    /// Sets the maximum records materialized in one Arrow batch.
    pub const fn with_batch_rows(mut self, batch_rows: usize) -> Self {
        self.batch_rows = batch_rows;
        self
    }

    /// Sets the maximum rows per Parquet row group.
    pub const fn with_row_group_rows(mut self, row_group_rows: usize) -> Self {
        self.row_group_rows = row_group_rows;
        self
    }

    /// Sets whether the completed temporary file is synchronized before rename.
    pub const fn with_sync_on_write(mut self, sync_on_write: bool) -> Self {
        self.sync_on_write = sync_on_write;
        self
    }

    /// Returns the cold-export root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns configured compression.
    pub const fn compression(&self) -> MarketDataParquetCompression {
        self.compression
    }

    /// Returns the Arrow batch row bound.
    pub const fn batch_rows(&self) -> usize {
        self.batch_rows
    }

    /// Returns the Parquet row-group row bound.
    pub const fn row_group_rows(&self) -> usize {
        self.row_group_rows
    }

    /// Returns whether completed temporary files are synchronized.
    pub const fn sync_on_write(&self) -> bool {
        self.sync_on_write
    }
}

/// Hive-style cold-export partition identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketDataParquetPartitionKey {
    /// ISO `YYYY-MM-DD` date selected by the host's session/UTC policy.
    pub date: String,
    /// Venue identifier.
    pub venue: String,
    /// Instrument symbol.
    pub symbol: String,
    /// Logical stream identifier.
    pub stream: String,
}

impl MarketDataParquetPartitionKey {
    /// Creates a partition key.
    pub fn new(
        date: impl Into<String>,
        venue: impl Into<String>,
        symbol: impl Into<String>,
        stream: impl Into<String>,
    ) -> Self {
        Self {
            date: date.into(),
            venue: venue.into(),
            symbol: symbol.into(),
            stream: stream.into(),
        }
    }
}

/// Source provenance repeated in each exported row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketDataParquetSourceMetadata {
    /// Deployment-specific feed source identifier.
    pub source_id: String,
    /// Adapter implementation/profile identifier.
    pub adapter_id: String,
    /// Capture session identifier.
    pub session_id: String,
}

impl MarketDataParquetSourceMetadata {
    /// Creates source provenance metadata.
    pub fn new(
        source_id: impl Into<String>,
        adapter_id: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            adapter_id: adapter_id.into(),
            session_id: session_id.into(),
        }
    }
}

/// Optional derived analytics snapshot joined to one WAL sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketDataDerivedSnapshotRef<'a> {
    /// WAL sequence whose event state the snapshot follows.
    pub wal_sequence: MarketDataWalSequence,
    /// Caller-defined payload schema identifier.
    pub schema_id: u32,
    /// Caller-defined serialized snapshot bytes.
    pub payload: &'a [u8],
}

impl<'a> MarketDataDerivedSnapshotRef<'a> {
    /// Creates one borrowed derived snapshot row.
    pub const fn new(
        wal_sequence: MarketDataWalSequence,
        schema_id: u32,
        payload: &'a [u8],
    ) -> Self {
        Self {
            wal_sequence,
            schema_id,
            payload,
        }
    }
}

/// Proof produced only after a Parquet file passes full post-write verification.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VerifiedMarketDataParquetExport {
    /// Generic cold-export partition metadata.
    pub partition: MarketDataColdExportPartition,
    /// ISO date partition written to every row.
    pub partition_date: String,
    /// Stable Parquet schema version.
    pub schema_version: u16,
    /// Lowercase SHA-256 digest of complete file bytes.
    pub sha256_hex: String,
    /// Number of encoded Parquet row groups.
    pub row_groups: u64,
    /// Rows carrying non-zero quality flags.
    pub quality_flagged_records: u64,
    /// Rows carrying a derived analytics payload.
    pub derived_snapshot_records: u64,
    /// Source provenance written to every row.
    pub source: MarketDataParquetSourceMetadata,
    /// Always true for a successfully constructed proof.
    pub verified: bool,
}

impl VerifiedMarketDataParquetExport {
    /// Creates conservative retention input backed by this verified proof.
    pub fn retention_input(&self, created_ns: u64, hot_bytes: u64) -> MarketDataRetentionInput {
        MarketDataRetentionInput::new(
            self.partition.first_sequence,
            self.partition.last_sequence,
            created_ns,
            hot_bytes,
        )
        .with_cold_export_verified(true)
    }
}
