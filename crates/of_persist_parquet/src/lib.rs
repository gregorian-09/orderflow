#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::builder::{
    BinaryBuilder, StringBuilder, UInt16Builder, UInt32Builder, UInt64Builder,
};
use arrow_array::{Array, BinaryArray, RecordBatch, UInt16Array, UInt32Array, UInt64Array};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use of_persist::{
    decode_normalized_market_data_record, MarketDataColdExportFormat,
    MarketDataColdExportPartition, MarketDataRetentionInput, MarketDataWal, MarketDataWalRecord,
    MarketDataWalRecordKind, MarketDataWalReplayFilter, MarketDataWalSequence,
    NormalizedMarketDataCodecError, SegmentedMarketDataWal,
};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::errors::ParquetError;
use parquet::file::properties::WriterProperties;
use sha2::{Digest, Sha256};

const EXPORT_SCHEMA_VERSION: u16 = 1;
const DEFAULT_BATCH_ROWS: usize = 32_768;
const DEFAULT_ROW_GROUP_ROWS: usize = 131_072;
const HASH_BUFFER_BYTES: usize = 64 * 1024;

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
    root: PathBuf,
    compression: MarketDataParquetCompression,
    batch_rows: usize,
    row_group_rows: usize,
    sync_on_write: bool,
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

/// Synchronous verified Parquet cold-export writer.
#[derive(Debug, Clone)]
pub struct MarketDataParquetWriter {
    config: MarketDataParquetExportConfig,
    schema: SchemaRef,
}

impl MarketDataParquetWriter {
    /// Opens an export root after validating memory and row-group bounds.
    ///
    /// # Errors
    /// Returns an error for zero/inverted bounds or root creation failure.
    pub fn open(config: MarketDataParquetExportConfig) -> MarketDataParquetResult<Self> {
        if config.batch_rows == 0 {
            return Err(MarketDataParquetError::InvalidConfig(
                "batch_rows must be greater than zero".to_owned(),
            ));
        }
        if config.row_group_rows == 0 {
            return Err(MarketDataParquetError::InvalidConfig(
                "row_group_rows must be greater than zero".to_owned(),
            ));
        }
        if config.batch_rows > config.row_group_rows {
            return Err(MarketDataParquetError::InvalidConfig(
                "batch_rows cannot exceed row_group_rows".to_owned(),
            ));
        }
        fs::create_dir_all(&config.root)?;
        Ok(Self {
            config,
            schema: export_schema(),
        })
    }

    /// Returns export configuration.
    pub const fn config(&self) -> &MarketDataParquetExportConfig {
        &self.config
    }

    /// Returns the stable Arrow schema written by this crate version.
    pub fn schema(&self) -> &SchemaRef {
        &self.schema
    }

    /// Exports caller-provided decoded WAL records and verifies the result.
    ///
    /// # Errors
    /// Returns an error for invalid metadata, malformed normalized events,
    /// existing destinations, encoding failures, or failed post-write checks.
    pub fn export_records(
        &self,
        key: &MarketDataParquetPartitionKey,
        source: &MarketDataParquetSourceMetadata,
        records: &[MarketDataWalRecord],
        derived_snapshots: &[MarketDataDerivedSnapshotRef<'_>],
    ) -> MarketDataParquetResult<VerifiedMarketDataParquetExport> {
        validate_partition_key(key)?;
        validate_source(source)?;
        validate_records(records)?;
        validate_derived_snapshots(records, derived_snapshots)?;

        let dir = self
            .config
            .root
            .join(format!("date={}", key.date))
            .join(format!("venue={}", key.venue))
            .join(format!("symbol={}", key.symbol))
            .join(format!("stream={}", key.stream));
        fs::create_dir_all(&dir)?;
        let file_name = export_file_name(records);
        let path = dir.join(file_name);
        let temp_path = path.with_extension("parquet.tmp");
        if path.exists() || temp_path.exists() {
            return Err(MarketDataParquetError::InvalidMetadata(format!(
                "export destination already exists: {}",
                path.display()
            )));
        }

        let properties = WriterProperties::builder()
            .set_compression(parquet_compression(self.config.compression))
            .set_max_row_group_row_count(Some(self.config.row_group_rows))
            .set_created_by(format!(
                "of_persist_parquet/{} schema={EXPORT_SCHEMA_VERSION}",
                env!("CARGO_PKG_VERSION")
            ))
            .build();
        let (mut temp_guard, file) = TemporaryExport::create(&temp_path)?;
        let mut writer = ArrowWriter::try_new(file, Arc::clone(&self.schema), Some(properties))?;
        let mut snapshot_index = 0usize;
        let mut quality_flagged_records = 0u64;
        for records_batch in records.chunks(self.config.batch_rows) {
            let batch = build_batch(
                Arc::clone(&self.schema),
                key,
                source,
                records_batch,
                derived_snapshots,
                &mut snapshot_index,
                &mut quality_flagged_records,
            )?;
            writer.write(&batch)?;
        }
        let metadata = writer.close()?;
        if snapshot_index != derived_snapshots.len() {
            return Err(MarketDataParquetError::InvalidDerivedSnapshots(
                "one or more snapshots did not match an exported record".to_owned(),
            ));
        }
        if self.config.sync_on_write {
            File::open(temp_guard.path())?.sync_all()?;
        }
        temp_guard.link_to(&path)?;

        let bytes = fs::metadata(&path)?.len();
        let (sha256_hex, file_checksum) = hash_file(&path)?;
        let partition = MarketDataColdExportPartition::new(
            MarketDataColdExportFormat::Parquet,
            &key.venue,
            &key.symbol,
            &key.stream,
            path,
        )
        .with_summary(
            records.len() as u64,
            bytes,
            records.first().map(|record| record.sequence),
            records.last().map(|record| record.sequence),
            records.first().map(|record| record.ts_exchange_ns),
            records.last().map(|record| record.ts_exchange_ns),
            file_checksum,
        );
        let proof = VerifiedMarketDataParquetExport {
            partition,
            partition_date: key.date.clone(),
            schema_version: EXPORT_SCHEMA_VERSION,
            sha256_hex,
            row_groups: metadata.num_row_groups() as u64,
            quality_flagged_records,
            derived_snapshot_records: derived_snapshots.len() as u64,
            source: source.clone(),
            verified: true,
        };
        self.verify_export(&proof)?;
        temp_guard.publish();
        Ok(proof)
    }

    /// Replays matching records from a single-file WAL and exports them.
    ///
    /// # Errors
    /// Returns replay, validation, write, or verification failure.
    pub fn export_wal(
        &self,
        key: &MarketDataParquetPartitionKey,
        source: &MarketDataParquetSourceMetadata,
        wal: &MarketDataWal,
        filter: MarketDataWalReplayFilter,
        derived_snapshots: &[MarketDataDerivedSnapshotRef<'_>],
    ) -> MarketDataParquetResult<VerifiedMarketDataParquetExport> {
        let mut records = Vec::new();
        wal.replay_filtered(filter, &mut records)?;
        self.export_records(key, source, &records, derived_snapshots)
    }

    /// Replays matching records from a segmented WAL and exports them.
    ///
    /// # Errors
    /// Returns replay, validation, write, or verification failure.
    pub fn export_segmented_wal(
        &self,
        key: &MarketDataParquetPartitionKey,
        source: &MarketDataParquetSourceMetadata,
        wal: &SegmentedMarketDataWal,
        filter: MarketDataWalReplayFilter,
        derived_snapshots: &[MarketDataDerivedSnapshotRef<'_>],
    ) -> MarketDataParquetResult<VerifiedMarketDataParquetExport> {
        let mut records = Vec::new();
        wal.replay_filtered(filter, &mut records)?;
        self.export_records(key, source, &records, derived_snapshots)
    }

    /// Reopens and fully verifies a previously produced export proof.
    ///
    /// # Errors
    /// Returns an error when bytes, schema, rows, sequence range, row groups,
    /// source metadata, or derived/quality counts differ from the proof.
    pub fn verify_export(
        &self,
        proof: &VerifiedMarketDataParquetExport,
    ) -> MarketDataParquetResult<()> {
        if !proof.verified
            || proof.schema_version != EXPORT_SCHEMA_VERSION
            || proof.partition.format != MarketDataColdExportFormat::Parquet
        {
            return Err(MarketDataParquetError::Verification(
                "proof is not a supported verified schema".to_owned(),
            ));
        }
        let bytes = fs::metadata(&proof.partition.path)?.len();
        if bytes != proof.partition.bytes {
            return Err(MarketDataParquetError::Verification(format!(
                "file byte count changed: expected {}, observed {bytes}",
                proof.partition.bytes
            )));
        }
        let (sha256_hex, checksum) = hash_file(&proof.partition.path)?;
        if sha256_hex != proof.sha256_hex || checksum != proof.partition.checksum {
            return Err(MarketDataParquetError::Verification(
                "file digest changed".to_owned(),
            ));
        }

        let file = File::open(&proof.partition.path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let row_groups = builder.metadata().num_row_groups() as u64;
        if row_groups != proof.row_groups {
            return Err(MarketDataParquetError::Verification(format!(
                "row-group count changed: expected {}, observed {row_groups}",
                proof.row_groups
            )));
        }
        if builder.schema().as_ref() != self.schema.as_ref() {
            return Err(MarketDataParquetError::Verification(
                "Arrow schema changed".to_owned(),
            ));
        }
        let mut reader = builder.with_batch_size(self.config.batch_rows).build()?;
        let mut rows = 0u64;
        let mut first_sequence = None;
        let mut last_sequence = None;
        let mut quality_flagged_records = 0u64;
        let mut derived_snapshot_records = 0u64;
        for batch in &mut reader {
            let batch = batch?;
            verify_constant_columns(&batch, proof)?;
            let sequence = downcast_u64(&batch, 10, "wal_sequence")?;
            let schema_version = downcast_u16(&batch, 0, "schema_version")?;
            if schema_version
                .iter()
                .flatten()
                .any(|value| value != EXPORT_SCHEMA_VERSION)
            {
                return Err(MarketDataParquetError::Verification(
                    "row schema version changed".to_owned(),
                ));
            }
            if let Some(value) = sequence.iter().flatten().next() {
                first_sequence.get_or_insert(MarketDataWalSequence(value));
            }
            if let Some(value) = sequence.iter().flatten().last() {
                last_sequence = Some(MarketDataWalSequence(value));
            }
            let quality = batch
                .column(15)
                .as_any()
                .downcast_ref::<arrow_array::UInt32Array>()
                .ok_or_else(|| column_type_error("quality_flags"))?;
            quality_flagged_records =
                quality_flagged_records.saturating_add(
                    quality.iter().flatten().filter(|value| *value != 0).count() as u64,
                );
            let derived = batch
                .column(19)
                .as_any()
                .downcast_ref::<BinaryArray>()
                .ok_or_else(|| column_type_error("derived_payload"))?;
            derived_snapshot_records = derived_snapshot_records
                .saturating_add((derived.len() - derived.null_count()) as u64);
            verify_payload_checksums(&batch)?;
            rows = rows.saturating_add(batch.num_rows() as u64);
        }
        if rows != proof.partition.records
            || first_sequence != proof.partition.first_sequence
            || last_sequence != proof.partition.last_sequence
            || quality_flagged_records != proof.quality_flagged_records
            || derived_snapshot_records != proof.derived_snapshot_records
        {
            return Err(MarketDataParquetError::Verification(
                "decoded row summary differs from proof".to_owned(),
            ));
        }
        Ok(())
    }
}

fn export_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("schema_version", DataType::UInt16, false),
        Field::new("partition_date", DataType::Utf8, false),
        Field::new("venue", DataType::Utf8, false),
        Field::new("symbol", DataType::Utf8, false),
        Field::new("stream", DataType::Utf8, false),
        Field::new("source_id", DataType::Utf8, false),
        Field::new("adapter_id", DataType::Utf8, false),
        Field::new("session_id", DataType::Utf8, false),
        Field::new("record_kind", DataType::UInt16, false),
        Field::new("record_kind_name", DataType::Utf8, false),
        Field::new("wal_sequence", DataType::UInt64, false),
        Field::new("provider_sequence", DataType::UInt64, false),
        Field::new("event_sequence", DataType::UInt64, false),
        Field::new("ts_exchange_ns", DataType::UInt64, false),
        Field::new("ts_recv_ns", DataType::UInt64, false),
        Field::new("quality_flags", DataType::UInt32, false),
        Field::new("payload", DataType::Binary, false),
        Field::new("payload_checksum", DataType::UInt32, false),
        Field::new("derived_schema_id", DataType::UInt32, true),
        Field::new("derived_payload", DataType::Binary, true),
    ]))
}

fn build_batch(
    schema: SchemaRef,
    key: &MarketDataParquetPartitionKey,
    source: &MarketDataParquetSourceMetadata,
    records: &[MarketDataWalRecord],
    derived_snapshots: &[MarketDataDerivedSnapshotRef<'_>],
    snapshot_index: &mut usize,
    quality_flagged_records: &mut u64,
) -> MarketDataParquetResult<RecordBatch> {
    let rows = records.len();
    let payload_bytes = records
        .iter()
        .map(|record| record.payload.len())
        .sum::<usize>();
    let mut schema_version = UInt16Builder::with_capacity(rows);
    let mut partition_date =
        StringBuilder::with_capacity(rows, rows.saturating_mul(key.date.len()));
    let mut venue = StringBuilder::with_capacity(rows, rows.saturating_mul(key.venue.len()));
    let mut symbol = StringBuilder::with_capacity(rows, rows.saturating_mul(key.symbol.len()));
    let mut stream = StringBuilder::with_capacity(rows, rows.saturating_mul(key.stream.len()));
    let mut source_id =
        StringBuilder::with_capacity(rows, rows.saturating_mul(source.source_id.len()));
    let mut adapter_id =
        StringBuilder::with_capacity(rows, rows.saturating_mul(source.adapter_id.len()));
    let mut session_id =
        StringBuilder::with_capacity(rows, rows.saturating_mul(source.session_id.len()));
    let mut record_kind = UInt16Builder::with_capacity(rows);
    let mut record_kind_name = StringBuilder::with_capacity(rows, rows.saturating_mul(24));
    let mut wal_sequence = UInt64Builder::with_capacity(rows);
    let mut provider_sequence = UInt64Builder::with_capacity(rows);
    let mut event_sequence = UInt64Builder::with_capacity(rows);
    let mut ts_exchange_ns = UInt64Builder::with_capacity(rows);
    let mut ts_recv_ns = UInt64Builder::with_capacity(rows);
    let mut quality_flags = UInt32Builder::with_capacity(rows);
    let mut payload = BinaryBuilder::with_capacity(rows, payload_bytes);
    let mut payload_checksum = UInt32Builder::with_capacity(rows);
    let mut derived_payload = BinaryBuilder::with_capacity(rows, 0);
    let mut derived_schema_id = UInt32Builder::with_capacity(rows);

    for record in records {
        let flags = record_quality_flags_for_partition(record, key)?;
        *quality_flagged_records = quality_flagged_records.saturating_add(u64::from(flags != 0));
        schema_version.append_value(EXPORT_SCHEMA_VERSION);
        partition_date.append_value(&key.date);
        venue.append_value(&key.venue);
        symbol.append_value(&key.symbol);
        stream.append_value(&key.stream);
        source_id.append_value(&source.source_id);
        adapter_id.append_value(&source.adapter_id);
        session_id.append_value(&source.session_id);
        record_kind.append_value(record.kind as u16);
        record_kind_name.append_value(record_kind_name_value(record.kind));
        wal_sequence.append_value(record.sequence.0);
        provider_sequence.append_value(record.provider_sequence);
        event_sequence.append_value(record.event_sequence);
        ts_exchange_ns.append_value(record.ts_exchange_ns);
        ts_recv_ns.append_value(record.ts_recv_ns);
        quality_flags.append_value(flags);
        payload.append_value(&record.payload);
        payload_checksum.append_value(fnv1a(&record.payload));
        if derived_snapshots
            .get(*snapshot_index)
            .is_some_and(|snapshot| snapshot.wal_sequence == record.sequence)
        {
            let snapshot = derived_snapshots[*snapshot_index];
            derived_payload.append_value(snapshot.payload);
            derived_schema_id.append_value(snapshot.schema_id);
            *snapshot_index += 1;
        } else {
            derived_payload.append_null();
            derived_schema_id.append_null();
        }
    }

    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(schema_version.finish()),
            Arc::new(partition_date.finish()),
            Arc::new(venue.finish()),
            Arc::new(symbol.finish()),
            Arc::new(stream.finish()),
            Arc::new(source_id.finish()),
            Arc::new(adapter_id.finish()),
            Arc::new(session_id.finish()),
            Arc::new(record_kind.finish()),
            Arc::new(record_kind_name.finish()),
            Arc::new(wal_sequence.finish()),
            Arc::new(provider_sequence.finish()),
            Arc::new(event_sequence.finish()),
            Arc::new(ts_exchange_ns.finish()),
            Arc::new(ts_recv_ns.finish()),
            Arc::new(quality_flags.finish()),
            Arc::new(payload.finish()),
            Arc::new(payload_checksum.finish()),
            Arc::new(derived_schema_id.finish()),
            Arc::new(derived_payload.finish()),
        ],
    )?)
}

fn validate_partition_key(key: &MarketDataParquetPartitionKey) -> MarketDataParquetResult<()> {
    if !is_iso_date(&key.date) {
        return Err(MarketDataParquetError::InvalidMetadata(
            "date must use YYYY-MM-DD".to_owned(),
        ));
    }
    validate_component("venue", &key.venue)?;
    validate_component("symbol", &key.symbol)?;
    validate_component("stream", &key.stream)
}

fn validate_source(source: &MarketDataParquetSourceMetadata) -> MarketDataParquetResult<()> {
    validate_text("source_id", &source.source_id)?;
    validate_text("adapter_id", &source.adapter_id)?;
    validate_text("session_id", &source.session_id)
}

fn validate_records(records: &[MarketDataWalRecord]) -> MarketDataParquetResult<()> {
    let mut previous = None;
    for record in records {
        if let Some(previous) = previous {
            if record.sequence.0 <= previous {
                return Err(MarketDataParquetError::InvalidMetadata(
                    "records must be strictly ordered by WAL sequence".to_owned(),
                ));
            }
        }
        previous = Some(record.sequence.0);
    }
    Ok(())
}

fn validate_derived_snapshots(
    records: &[MarketDataWalRecord],
    snapshots: &[MarketDataDerivedSnapshotRef<'_>],
) -> MarketDataParquetResult<()> {
    let range = records
        .first()
        .zip(records.last())
        .map(|(first, last)| (first.sequence.0, last.sequence.0));
    let mut previous = None;
    for snapshot in snapshots {
        if snapshot.schema_id == 0 {
            return Err(MarketDataParquetError::InvalidDerivedSnapshots(
                "schema_id must be non-zero".to_owned(),
            ));
        }
        if let Some(previous) = previous {
            if snapshot.wal_sequence.0 <= previous {
                return Err(MarketDataParquetError::InvalidDerivedSnapshots(
                    "snapshots must be strictly ordered by WAL sequence".to_owned(),
                ));
            }
        }
        if !range.is_some_and(|(first, last)| {
            snapshot.wal_sequence.0 >= first && snapshot.wal_sequence.0 <= last
        }) {
            return Err(MarketDataParquetError::InvalidDerivedSnapshots(
                "snapshot sequence is outside the export range".to_owned(),
            ));
        }
        previous = Some(snapshot.wal_sequence.0);
    }
    Ok(())
}

fn record_quality_flags_for_partition(
    record: &MarketDataWalRecord,
    key: &MarketDataParquetPartitionKey,
) -> MarketDataParquetResult<u32> {
    if matches!(
        record.kind,
        MarketDataWalRecordKind::BookUpdate | MarketDataWalRecordKind::TradePrint
    ) {
        let decoded = decode_normalized_market_data_record(record)?;
        let symbol = decoded.symbol();
        if symbol.venue != key.venue || symbol.symbol != key.symbol {
            return Err(MarketDataParquetError::InvalidMetadata(
                "normalized event does not match the target venue/symbol partition".to_owned(),
            ));
        }
        return Ok(decoded.quality_flags_bits());
    }
    Ok(0)
}

fn verify_payload_checksums(batch: &RecordBatch) -> MarketDataParquetResult<()> {
    let payloads = batch
        .column(16)
        .as_any()
        .downcast_ref::<BinaryArray>()
        .ok_or_else(|| column_type_error("payload"))?;
    let checksums = batch
        .column(17)
        .as_any()
        .downcast_ref::<UInt32Array>()
        .ok_or_else(|| column_type_error("payload_checksum"))?;
    if payloads.len() != checksums.len()
        || payloads
            .iter()
            .zip(checksums.iter())
            .any(|(payload, checksum)| match (payload, checksum) {
                (Some(payload), Some(checksum)) => fnv1a(payload) != checksum,
                _ => true,
            })
    {
        return Err(MarketDataParquetError::Verification(
            "payload checksum column is inconsistent".to_owned(),
        ));
    }
    Ok(())
}

fn verify_constant_columns(
    batch: &RecordBatch,
    proof: &VerifiedMarketDataParquetExport,
) -> MarketDataParquetResult<()> {
    for (index, name, expected) in [
        (1, "partition_date", proof.partition_date.as_str()),
        (2, "venue", proof.partition.venue.as_str()),
        (3, "symbol", proof.partition.symbol.as_str()),
        (4, "stream", proof.partition.stream.as_str()),
        (5, "source_id", proof.source.source_id.as_str()),
        (6, "adapter_id", proof.source.adapter_id.as_str()),
        (7, "session_id", proof.source.session_id.as_str()),
    ] {
        let values = batch
            .column(index)
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .ok_or_else(|| column_type_error(name))?;
        if values.iter().flatten().any(|value| value != expected) {
            return Err(MarketDataParquetError::Verification(format!(
                "{name} column differs from proof"
            )));
        }
    }
    Ok(())
}

fn downcast_u64<'a>(
    batch: &'a RecordBatch,
    index: usize,
    name: &str,
) -> MarketDataParquetResult<&'a UInt64Array> {
    batch
        .column(index)
        .as_any()
        .downcast_ref::<UInt64Array>()
        .ok_or_else(|| column_type_error(name))
}

fn downcast_u16<'a>(
    batch: &'a RecordBatch,
    index: usize,
    name: &str,
) -> MarketDataParquetResult<&'a UInt16Array> {
    batch
        .column(index)
        .as_any()
        .downcast_ref::<UInt16Array>()
        .ok_or_else(|| column_type_error(name))
}

fn column_type_error(name: &str) -> MarketDataParquetError {
    MarketDataParquetError::Verification(format!("unexpected type for {name} column"))
}

fn validate_component(name: &str, value: &str) -> MarketDataParquetResult<()> {
    validate_text(name, value)?;
    if value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains('=')
        || value.ends_with(".tmp")
    {
        return Err(MarketDataParquetError::InvalidMetadata(format!(
            "{name} is not a safe partition component"
        )));
    }
    Ok(())
}

fn validate_text(name: &str, value: &str) -> MarketDataParquetResult<()> {
    if value.is_empty() || value.len() > 255 || value.chars().any(char::is_control) {
        return Err(MarketDataParquetError::InvalidMetadata(format!(
            "{name} must contain 1..=255 non-control bytes"
        )));
    }
    Ok(())
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    let syntax_valid = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit());
    if !syntax_valid {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u16>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return false;
    };
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

fn export_file_name(records: &[MarketDataWalRecord]) -> String {
    match (records.first(), records.last()) {
        (Some(first), Some(last)) => format!(
            "wal-{:020}-{:020}.parquet",
            first.sequence.0, last.sequence.0
        ),
        _ => "wal-empty.parquet".to_owned(),
    }
}

fn parquet_compression(compression: MarketDataParquetCompression) -> Compression {
    match compression {
        MarketDataParquetCompression::Uncompressed => Compression::UNCOMPRESSED,
        MarketDataParquetCompression::Snappy => Compression::SNAPPY,
        MarketDataParquetCompression::Zstd => Compression::ZSTD(ZstdLevel::default()),
    }
}

fn record_kind_name_value(kind: MarketDataWalRecordKind) -> &'static str {
    match kind {
        MarketDataWalRecordKind::BookUpdate => "BookUpdate",
        MarketDataWalRecordKind::TradePrint => "TradePrint",
        MarketDataWalRecordKind::Heartbeat => "Heartbeat",
        MarketDataWalRecordKind::GapMarker => "GapMarker",
        MarketDataWalRecordKind::SegmentSeal => "SegmentSeal",
        MarketDataWalRecordKind::BookSnapshotMarker => "BookSnapshotMarker",
        MarketDataWalRecordKind::QualityFlag => "QualityFlag",
        MarketDataWalRecordKind::AdapterHealth => "AdapterHealth",
        MarketDataWalRecordKind::SubscriptionState => "SubscriptionState",
        MarketDataWalRecordKind::OutOfOrderMarker => "OutOfOrderMarker",
        MarketDataWalRecordKind::CheckpointMarker => "CheckpointMarker",
        MarketDataWalRecordKind::RawProviderMessage => "RawProviderMessage",
        _ => "Unknown",
    }
}

fn hash_file(path: &Path) -> MarketDataParquetResult<(String, u32)> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(HASH_BUFFER_BYTES, file);
    let mut sha256 = Sha256::new();
    let mut fnv = 0x811c9dc5_u32;
    let mut buffer = [0u8; HASH_BUFFER_BYTES];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        sha256.update(&buffer[..read]);
        fnv = update_fnv1a(fnv, &buffer[..read]);
    }
    Ok((format!("{:x}", sha256.finalize()), fnv))
}

fn fnv1a(bytes: &[u8]) -> u32 {
    update_fnv1a(0x811c9dc5_u32, bytes)
}

fn update_fnv1a(mut checksum: u32, bytes: &[u8]) -> u32 {
    for byte in bytes {
        checksum ^= u32::from(*byte);
        checksum = checksum.wrapping_mul(0x01000193);
    }
    checksum
}

struct TemporaryExport {
    path: PathBuf,
    published: bool,
}

impl TemporaryExport {
    fn create(path: &Path) -> MarketDataParquetResult<(Self, File)> {
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    MarketDataParquetError::InvalidMetadata(format!(
                        "temporary export already exists: {}",
                        path.display()
                    ))
                } else {
                    MarketDataParquetError::Io(error)
                }
            })?;
        Ok((
            Self {
                path: path.to_owned(),
                published: false,
            },
            file,
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn link_to(&mut self, destination: &Path) -> MarketDataParquetResult<()> {
        fs::hard_link(&self.path, destination).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                MarketDataParquetError::InvalidMetadata(format!(
                    "export destination already exists: {}",
                    destination.display()
                ))
            } else {
                MarketDataParquetError::Io(error)
            }
        })?;
        if let Err(error) = fs::remove_file(&self.path) {
            let _ = fs::remove_file(destination);
            return Err(MarketDataParquetError::Io(error));
        }
        self.path = destination.to_owned();
        Ok(())
    }

    fn publish(&mut self) {
        self.published = true;
    }
}

impl Drop for TemporaryExport {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use of_core::{Side, SymbolId, TradePrint};
    use of_persist::{
        MarketDataRetentionPolicy, MarketDataWalConfig, MarketDataWalRecordKind,
        MarketDataWalSyncPolicy, NormalizedMarketDataRecordInput, SegmentedMarketDataWalConfig,
    };

    use super::*;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "of-persist-parquet-{name}-{}-{nonce}",
                std::process::id()
            )))
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn trade_record(sequence: u64, quality_flags_bits: u32) -> MarketDataWalRecord {
        let input = NormalizedMarketDataRecordInput::trade_with_quality(
            TradePrint {
                symbol: SymbolId {
                    venue: "CME".to_owned(),
                    symbol: "ESM6".to_owned(),
                },
                price: 5_050_000 + sequence as i64,
                size: sequence as i64,
                aggressor_side: Side::Ask,
                sequence,
                ts_exchange_ns: sequence * 10,
                ts_recv_ns: sequence * 10 + 1,
            },
            quality_flags_bits,
        );
        let mut payload = Vec::new();
        input.encode_into(&mut payload).expect("encode");
        MarketDataWalRecord::new(
            MarketDataWalSequence(sequence),
            MarketDataWalRecordKind::TradePrint,
            sequence,
            sequence,
            sequence * 10,
            sequence * 10 + 1,
            payload,
        )
    }

    fn key() -> MarketDataParquetPartitionKey {
        MarketDataParquetPartitionKey::new("2026-08-12", "CME", "ESM6", "trades")
    }

    fn source() -> MarketDataParquetSourceMetadata {
        MarketDataParquetSourceMetadata::new("primary", "cqg", "session-1")
    }

    #[test]
    fn export_is_partitioned_verified_and_retention_safe() {
        let root = TestRoot::new("round-trip");
        let writer = MarketDataParquetWriter::open(
            MarketDataParquetExportConfig::new(&root.0)
                .with_batch_rows(1)
                .with_row_group_rows(1)
                .with_compression(MarketDataParquetCompression::Snappy),
        )
        .expect("writer");
        let records = vec![trade_record(1, 0), trade_record(2, 0x20)];
        let snapshots = [MarketDataDerivedSnapshotRef::new(
            MarketDataWalSequence(2),
            7,
            b"derived",
        )];
        let proof = writer
            .export_records(&key(), &source(), &records, &snapshots)
            .expect("export");

        assert!(proof.verified);
        assert_eq!(proof.partition.format, MarketDataColdExportFormat::Parquet);
        assert_eq!(proof.partition.records, 2);
        assert_eq!(
            proof.partition.first_sequence,
            Some(MarketDataWalSequence(1))
        );
        assert_eq!(
            proof.partition.last_sequence,
            Some(MarketDataWalSequence(2))
        );
        assert_eq!(proof.row_groups, 2);
        assert_eq!(proof.quality_flagged_records, 1);
        assert_eq!(proof.derived_snapshot_records, 1);
        assert_eq!(proof.sha256_hex.len(), 64);
        assert!(proof.partition.path.ends_with(
            "date=2026-08-12/venue=CME/symbol=ESM6/stream=trades/wal-00000000000000000001-00000000000000000002.parquet"
        ));
        writer.verify_export(&proof).expect("reverify");

        let retention = proof.retention_input(10, 100);
        assert!(retention.cold_export_verified);
        let policy = MarketDataRetentionPolicy::conservative()
            .with_hot_retention_ns(1)
            .with_min_checkpoints_retained(0);
        let decision = of_persist::plan_market_data_retention(policy, 20, &retention);
        assert!(decision
            .actions
            .contains(&of_persist::MarketDataRetentionAction::DeleteHotWal));
    }

    #[test]
    fn corruption_invalidates_existing_proof() {
        let root = TestRoot::new("corruption");
        let writer = MarketDataParquetWriter::open(
            MarketDataParquetExportConfig::new(&root.0)
                .with_batch_rows(1)
                .with_row_group_rows(1),
        )
        .expect("writer");
        let proof = writer
            .export_records(&key(), &source(), &[trade_record(1, 0)], &[])
            .expect("export");
        let file = fs::OpenOptions::new()
            .append(true)
            .open(&proof.partition.path)
            .expect("open append");
        file.set_len(proof.partition.bytes + 1).expect("corrupt");
        let error = writer.verify_export(&proof).expect_err("must reject");
        assert!(matches!(error, MarketDataParquetError::Verification(_)));
    }

    #[test]
    fn metadata_and_ordering_fail_before_publication() {
        let root = TestRoot::new("validation");
        let writer = MarketDataParquetWriter::open(MarketDataParquetExportConfig::new(&root.0))
            .expect("writer");
        let invalid_key =
            MarketDataParquetPartitionKey::new("2026-13-12", "../CME", "ESM6", "trades");
        assert!(matches!(
            writer.export_records(&invalid_key, &source(), &[trade_record(1, 0)], &[]),
            Err(MarketDataParquetError::InvalidMetadata(_))
        ));
        let impossible_date =
            MarketDataParquetPartitionKey::new("2026-02-29", "CME", "ESM6", "trades");
        assert!(matches!(
            writer.export_records(&impossible_date, &source(), &[trade_record(1, 0)], &[]),
            Err(MarketDataParquetError::InvalidMetadata(_))
        ));

        let records = vec![trade_record(1, 0), trade_record(2, 0)];
        let snapshots = [
            MarketDataDerivedSnapshotRef::new(MarketDataWalSequence(2), 1, b"two"),
            MarketDataDerivedSnapshotRef::new(MarketDataWalSequence(1), 1, b"one"),
        ];
        assert!(matches!(
            writer.export_records(&key(), &source(), &records, &snapshots),
            Err(MarketDataParquetError::InvalidDerivedSnapshots(_))
        ));

        let mut malformed = trade_record(3, 0);
        malformed.payload[0] ^= 0xff;
        assert!(matches!(
            writer.export_records(&key(), &source(), &[malformed], &[]),
            Err(MarketDataParquetError::Normalized(_))
        ));
        assert!(!walk_files(&root.0)
            .iter()
            .any(|path| path.extension().is_some_and(|extension| extension == "tmp")));
    }

    #[test]
    fn single_and_segmented_wal_export_paths_replay_filters() {
        let root = TestRoot::new("wal-bridges");
        let mut single = MarketDataWal::open(MarketDataWalConfig::new(root.0.join("single.ofmw")))
            .expect("single WAL");
        let record = trade_record(1, 0x10);
        single
            .append_record(
                record.kind,
                record.provider_sequence,
                record.event_sequence,
                record.ts_exchange_ns,
                record.ts_recv_ns,
                &record.payload,
            )
            .expect("single append");

        let writer = MarketDataParquetWriter::open(
            MarketDataParquetExportConfig::new(root.0.join("cold"))
                .with_batch_rows(1)
                .with_row_group_rows(1),
        )
        .expect("writer");
        let single_proof = writer
            .export_wal(
                &key(),
                &source(),
                &single,
                MarketDataWalReplayFilter::new().with_sequence_range(
                    Some(MarketDataWalSequence(1)),
                    Some(MarketDataWalSequence(1)),
                ),
                &[],
            )
            .expect("single export");
        assert_eq!(single_proof.partition.records, 1);

        let mut segmented = SegmentedMarketDataWal::open(
            SegmentedMarketDataWalConfig::new(root.0.join("segmented"))
                .with_sync_policy(MarketDataWalSyncPolicy::Never),
        )
        .expect("segmented WAL");
        segmented
            .append_record(
                record.kind,
                record.provider_sequence,
                record.event_sequence,
                record.ts_exchange_ns,
                record.ts_recv_ns,
                &record.payload,
            )
            .expect("segmented append");
        let segmented_key =
            MarketDataParquetPartitionKey::new("2026-08-12", "CME", "ESM6", "trades-segmented");
        let segmented_proof = writer
            .export_segmented_wal(
                &segmented_key,
                &source(),
                &segmented,
                MarketDataWalReplayFilter::new(),
                &[],
            )
            .expect("segmented export");
        assert_eq!(segmented_proof.partition.records, 1);
        assert_eq!(segmented_proof.quality_flagged_records, 1);
    }

    #[test]
    fn config_rejects_unbounded_or_inverted_batches() {
        let root = TestRoot::new("config");
        assert!(MarketDataParquetWriter::open(
            MarketDataParquetExportConfig::new(&root.0).with_batch_rows(0)
        )
        .is_err());
        assert!(MarketDataParquetWriter::open(
            MarketDataParquetExportConfig::new(&root.0)
                .with_batch_rows(2)
                .with_row_group_rows(1)
        )
        .is_err());
    }

    fn walk_files(root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let Ok(entries) = fs::read_dir(root) else {
            return files;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_files(&path));
            } else {
                files.push(path);
            }
        }
        files
    }
}
