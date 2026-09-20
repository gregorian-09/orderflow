use super::*;

/// Cold export file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataColdExportFormat {
    /// Newline-delimited JSON.
    #[default]
    JsonLines,
    /// Comma-separated values.
    Csv,
    /// Apache Parquet.
    Parquet,
    /// Apache Arrow/Feather.
    Arrow,
    /// User-defined export format.
    Custom,
}

/// Configuration for [`FileMarketDataJsonlExportWriter`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MarketDataJsonlExportConfig {
    pub(crate) root: PathBuf,
    pub(crate) sync_on_write: bool,
}

impl MarketDataJsonlExportConfig {
    /// Creates JSONL export config rooted at `root`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            sync_on_write: false,
        }
    }

    /// Sets whether exported files call `sync_data`.
    pub const fn with_sync_on_write(mut self, sync_on_write: bool) -> Self {
        self.sync_on_write = sync_on_write;
        self
    }

    /// Returns the configured export root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns whether exported files call `sync_data`.
    pub const fn sync_on_write(&self) -> bool {
        self.sync_on_write
    }
}

/// Metadata for one cold-export partition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataColdExportPartition {
    /// Export file format.
    pub format: MarketDataColdExportFormat,
    /// Venue name.
    pub venue: String,
    /// Symbol name.
    pub symbol: String,
    /// Stream name.
    pub stream: String,
    /// Export file path.
    pub path: PathBuf,
    /// Number of records exported.
    pub records: u64,
    /// Number of bytes written.
    pub bytes: u64,
    /// First WAL sequence exported.
    pub first_sequence: Option<MarketDataWalSequence>,
    /// Last WAL sequence exported.
    pub last_sequence: Option<MarketDataWalSequence>,
    /// First exchange timestamp exported.
    pub first_ts_exchange_ns: Option<u64>,
    /// Last exchange timestamp exported.
    pub last_ts_exchange_ns: Option<u64>,
    /// Checksum over exported bytes.
    pub checksum: u32,
}

impl MarketDataColdExportPartition {
    /// Creates empty metadata for one externally implemented cold partition.
    ///
    /// This constructor lets optional columnar export crates participate in
    /// the stable retention model without exposing this non-exhaustive
    /// struct's layout as a construction contract.
    pub fn new(
        format: MarketDataColdExportFormat,
        venue: impl Into<String>,
        symbol: impl Into<String>,
        stream: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            format,
            venue: venue.into(),
            symbol: symbol.into(),
            stream: stream.into(),
            path: path.into(),
            records: 0,
            bytes: 0,
            first_sequence: None,
            last_sequence: None,
            first_ts_exchange_ns: None,
            last_ts_exchange_ns: None,
            checksum: 0,
        }
    }

    /// Sets record, byte, range, timestamp, and checksum summary fields.
    #[allow(clippy::too_many_arguments)]
    pub const fn with_summary(
        mut self,
        records: u64,
        bytes: u64,
        first_sequence: Option<MarketDataWalSequence>,
        last_sequence: Option<MarketDataWalSequence>,
        first_ts_exchange_ns: Option<u64>,
        last_ts_exchange_ns: Option<u64>,
        checksum: u32,
    ) -> Self {
        self.records = records;
        self.bytes = bytes;
        self.first_sequence = first_sequence;
        self.last_sequence = last_sequence;
        self.first_ts_exchange_ns = first_ts_exchange_ns;
        self.last_ts_exchange_ns = last_ts_exchange_ns;
        self.checksum = checksum;
        self
    }
}

/// Manifest for a cold-export operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataColdExportManifest {
    /// Export file format.
    pub format: MarketDataColdExportFormat,
    /// Exported partitions.
    pub partitions: Vec<MarketDataColdExportPartition>,
    /// Total records exported.
    pub total_records: u64,
    /// Total bytes written.
    pub total_bytes: u64,
}

impl MarketDataColdExportManifest {
    /// Creates a manifest from exported partitions.
    pub fn from_partitions(
        format: MarketDataColdExportFormat,
        partitions: Vec<MarketDataColdExportPartition>,
    ) -> Self {
        let total_records = partitions.iter().map(|partition| partition.records).sum();
        let total_bytes = partitions.iter().map(|partition| partition.bytes).sum();
        Self {
            format,
            partitions,
            total_records,
            total_bytes,
        }
    }
}

/// File-backed JSONL cold-export writer for decoded market-data WAL records.
#[derive(Debug, Clone)]
pub struct FileMarketDataJsonlExportWriter {
    pub(crate) config: MarketDataJsonlExportConfig,
}

impl FileMarketDataJsonlExportWriter {
    /// Opens or creates a JSONL export writer root.
    pub fn open(config: MarketDataJsonlExportConfig) -> PersistResult<Self> {
        create_dir_all(&config.root)?;
        Ok(Self { config })
    }

    /// Returns the export writer configuration.
    pub const fn config(&self) -> &MarketDataJsonlExportConfig {
        &self.config
    }

    /// Exports decoded records into one JSONL partition.
    pub fn export_records(
        &self,
        venue: &str,
        symbol: &str,
        stream: &str,
        records: &[MarketDataWalRecord],
    ) -> PersistResult<MarketDataColdExportPartition> {
        let dir = self.config.root.join(venue).join(symbol);
        create_dir_all(&dir)?;
        let path = dir.join(cold_export_file_name(stream, records));
        let mut file = File::create(&path)?;
        let mut partition = MarketDataColdExportPartition {
            format: MarketDataColdExportFormat::JsonLines,
            venue: venue.to_owned(),
            symbol: symbol.to_owned(),
            stream: stream.to_owned(),
            path,
            records: 0,
            bytes: 0,
            first_sequence: None,
            last_sequence: None,
            first_ts_exchange_ns: None,
            last_ts_exchange_ns: None,
            checksum: 0x811c9dc5_u32,
        };
        for record in records {
            let line = format_cold_export_record(venue, symbol, stream, record);
            file.write_all(line.as_bytes())?;
            partition.checksum = update_fnv1a(partition.checksum, line.as_bytes());
            partition.bytes = partition.bytes.saturating_add(line.len() as u64);
            partition.records = partition.records.saturating_add(1);
            partition.first_sequence.get_or_insert(record.sequence);
            partition.last_sequence = Some(record.sequence);
            partition
                .first_ts_exchange_ns
                .get_or_insert(record.ts_exchange_ns);
            partition.last_ts_exchange_ns = Some(record.ts_exchange_ns);
        }
        if self.config.sync_on_write {
            file.sync_data()?;
        }
        Ok(partition)
    }

    /// Replays matching WAL records and exports them into one JSONL partition.
    pub fn export_wal(
        &self,
        venue: &str,
        symbol: &str,
        stream: &str,
        wal: &MarketDataWal,
        filter: MarketDataWalReplayFilter,
    ) -> PersistResult<MarketDataColdExportPartition> {
        let mut records = Vec::new();
        wal.replay_filtered(filter, &mut records)?;
        self.export_records(venue, symbol, stream, &records)
    }
}

/// Retention/tiering action selected for market-data persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MarketDataRetentionAction {
    /// Keep the hot WAL segment.
    RetainHotWal,
    /// Export the WAL range to cold storage.
    ExportCold,
    /// Delete the hot WAL range.
    DeleteHotWal,
    /// Keep checkpoints that depend on the WAL range.
    RetainCheckpoint,
    /// Delete checkpoints that no longer depend on retained WAL.
    DeleteCheckpoint,
    /// Preserve records because they are part of an incident window.
    PreserveIncidentWindow,
}

/// Reason attached to a retention/tiering decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MarketDataRetentionReason {
    /// WAL is still within the hot retention window.
    WithinHotWindow,
    /// WAL age exceeds the hot retention window.
    AgeExceeded,
    /// Hot bytes exceed the configured byte budget.
    BytesExceeded,
    /// Cold export has not been verified.
    MissingVerifiedColdExport,
    /// Cold export has been verified.
    VerifiedColdExport,
    /// A checkpoint still depends on this WAL range.
    CheckpointDependsOnWal,
    /// Records are inside an incident preservation window.
    IncidentWindow,
}

/// Market-data retention and tiering policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataRetentionPolicy {
    /// Hot WAL retention window in nanoseconds. Zero disables age pressure.
    pub hot_retention_ns: u64,
    /// Maximum hot WAL bytes. Zero disables byte pressure.
    pub max_hot_bytes: u64,
    /// Require verified cold export before hot WAL deletion.
    pub require_verified_cold_export: bool,
    /// Preserve incident windows regardless of age/size pressure.
    pub preserve_incident_windows: bool,
    /// Minimum checkpoints to retain after their WAL dependencies are gone.
    pub min_checkpoints_retained: usize,
}

impl MarketDataRetentionPolicy {
    /// Creates a conservative policy that never deletes without verified export.
    pub const fn conservative() -> Self {
        Self {
            hot_retention_ns: 0,
            max_hot_bytes: 0,
            require_verified_cold_export: true,
            preserve_incident_windows: true,
            min_checkpoints_retained: 2,
        }
    }

    /// Sets hot WAL retention window in nanoseconds.
    pub const fn with_hot_retention_ns(mut self, hot_retention_ns: u64) -> Self {
        self.hot_retention_ns = hot_retention_ns;
        self
    }

    /// Sets maximum hot WAL bytes.
    pub const fn with_max_hot_bytes(mut self, max_hot_bytes: u64) -> Self {
        self.max_hot_bytes = max_hot_bytes;
        self
    }

    /// Sets whether cold export verification is required before deletion.
    pub const fn with_require_verified_cold_export(
        mut self,
        require_verified_cold_export: bool,
    ) -> Self {
        self.require_verified_cold_export = require_verified_cold_export;
        self
    }

    /// Sets whether incident windows are preserved.
    pub const fn with_preserve_incident_windows(mut self, preserve_incident_windows: bool) -> Self {
        self.preserve_incident_windows = preserve_incident_windows;
        self
    }

    /// Sets minimum checkpoints retained after WAL dependencies are gone.
    pub const fn with_min_checkpoints_retained(mut self, min_checkpoints_retained: usize) -> Self {
        self.min_checkpoints_retained = min_checkpoints_retained;
        self
    }
}

impl Default for MarketDataRetentionPolicy {
    fn default() -> Self {
        Self::conservative()
    }
}

/// Inputs for retention/tiering planning over one WAL range.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataRetentionInput {
    /// First WAL sequence in the range.
    pub first_sequence: Option<MarketDataWalSequence>,
    /// Last WAL sequence in the range.
    pub last_sequence: Option<MarketDataWalSequence>,
    /// Range creation timestamp in nanoseconds.
    pub created_ns: u64,
    /// Bytes occupied by the hot WAL range.
    pub hot_bytes: u64,
    /// True when cold export for this range has been verified.
    pub cold_export_verified: bool,
    /// True when this range belongs to an incident window.
    pub incident_window: bool,
    /// Latest checkpoint sequence that depends on this WAL range.
    pub dependent_checkpoint_sequence: Option<MarketDataWalSequence>,
    /// Checkpoints currently retained for the stream.
    pub retained_checkpoints: usize,
}

impl MarketDataRetentionInput {
    /// Creates retention input for one WAL sequence range.
    pub const fn new(
        first_sequence: Option<MarketDataWalSequence>,
        last_sequence: Option<MarketDataWalSequence>,
        created_ns: u64,
        hot_bytes: u64,
    ) -> Self {
        Self {
            first_sequence,
            last_sequence,
            created_ns,
            hot_bytes,
            cold_export_verified: false,
            incident_window: false,
            dependent_checkpoint_sequence: None,
            retained_checkpoints: 0,
        }
    }

    /// Sets cold export verification state.
    pub const fn with_cold_export_verified(mut self, cold_export_verified: bool) -> Self {
        self.cold_export_verified = cold_export_verified;
        self
    }

    /// Sets incident-window state.
    pub const fn with_incident_window(mut self, incident_window: bool) -> Self {
        self.incident_window = incident_window;
        self
    }

    /// Sets latest dependent checkpoint sequence.
    pub const fn with_dependent_checkpoint_sequence(
        mut self,
        dependent_checkpoint_sequence: Option<MarketDataWalSequence>,
    ) -> Self {
        self.dependent_checkpoint_sequence = dependent_checkpoint_sequence;
        self
    }

    /// Sets retained checkpoint count for the stream.
    pub const fn with_retained_checkpoints(mut self, retained_checkpoints: usize) -> Self {
        self.retained_checkpoints = retained_checkpoints;
        self
    }
}

/// Retention/tiering decision for one WAL range.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataRetentionDecision {
    /// Selected actions.
    pub actions: Vec<MarketDataRetentionAction>,
    /// Reasons for selected actions.
    pub reasons: Vec<MarketDataRetentionReason>,
    /// True when hot WAL can be deleted.
    pub may_delete_hot_wal: bool,
    /// True when cold export should run before deletion.
    pub should_export_cold: bool,
    /// True when checkpoints should be retained.
    pub should_retain_checkpoints: bool,
}

/// Builds a deterministic retention/tiering decision for one WAL range.
pub fn plan_market_data_retention(
    policy: MarketDataRetentionPolicy,
    now_ns: u64,
    input: &MarketDataRetentionInput,
) -> MarketDataRetentionDecision {
    let mut decision = MarketDataRetentionDecision::default();
    if policy.preserve_incident_windows && input.incident_window {
        decision
            .actions
            .push(MarketDataRetentionAction::PreserveIncidentWindow);
        decision
            .actions
            .push(MarketDataRetentionAction::RetainHotWal);
        decision
            .reasons
            .push(MarketDataRetentionReason::IncidentWindow);
        return decision;
    }

    let age_exceeded = policy.hot_retention_ns > 0
        && now_ns.saturating_sub(input.created_ns) >= policy.hot_retention_ns;
    let bytes_exceeded = policy.max_hot_bytes > 0 && input.hot_bytes >= policy.max_hot_bytes;
    if !age_exceeded && !bytes_exceeded {
        decision
            .actions
            .push(MarketDataRetentionAction::RetainHotWal);
        decision
            .reasons
            .push(MarketDataRetentionReason::WithinHotWindow);
        return decision;
    }
    if age_exceeded {
        decision
            .reasons
            .push(MarketDataRetentionReason::AgeExceeded);
    }
    if bytes_exceeded {
        decision
            .reasons
            .push(MarketDataRetentionReason::BytesExceeded);
    }

    if input.dependent_checkpoint_sequence.is_some() {
        decision
            .actions
            .push(MarketDataRetentionAction::RetainHotWal);
        decision
            .actions
            .push(MarketDataRetentionAction::RetainCheckpoint);
        decision
            .reasons
            .push(MarketDataRetentionReason::CheckpointDependsOnWal);
        return decision;
    }

    if policy.require_verified_cold_export && !input.cold_export_verified {
        decision.actions.push(MarketDataRetentionAction::ExportCold);
        decision
            .actions
            .push(MarketDataRetentionAction::RetainHotWal);
        decision
            .reasons
            .push(MarketDataRetentionReason::MissingVerifiedColdExport);
        decision.should_export_cold = true;
        return decision;
    }

    if input.cold_export_verified {
        decision
            .reasons
            .push(MarketDataRetentionReason::VerifiedColdExport);
    }
    decision
        .actions
        .push(MarketDataRetentionAction::DeleteHotWal);
    decision.may_delete_hot_wal = true;
    if input.retained_checkpoints <= policy.min_checkpoints_retained {
        decision
            .actions
            .push(MarketDataRetentionAction::RetainCheckpoint);
        decision.should_retain_checkpoints = true;
    } else {
        decision
            .actions
            .push(MarketDataRetentionAction::DeleteCheckpoint);
    }
    decision
}

pub(crate) fn cold_export_file_name(stream: &str, records: &[MarketDataWalRecord]) -> String {
    let escaped_stream = path_component(stream);
    match (records.first(), records.last()) {
        (Some(first), Some(last)) => {
            format!(
                "{}-{:020}-{:020}.jsonl",
                escaped_stream, first.sequence.0, last.sequence.0
            )
        }
        _ => format!("{}-empty-{}.jsonl", escaped_stream, current_unix_nanos()),
    }
}

pub(crate) fn format_cold_export_record(
    venue: &str,
    symbol: &str,
    stream: &str,
    record: &MarketDataWalRecord,
) -> String {
    format!(
        "{{\"schema\":1,\"venue\":\"{}\",\"symbol\":\"{}\",\"stream\":\"{}\",\"kind\":\"{}\",\"wal_sequence\":{},\"provider_sequence\":{},\"event_sequence\":{},\"ts_exchange_ns\":{},\"ts_recv_ns\":{},\"payload_hex\":\"{}\"}}\n",
        escape_json(venue),
        escape_json(symbol),
        escape_json(stream),
        market_data_wal_record_kind_name(record.kind),
        record.sequence.0,
        record.provider_sequence,
        record.event_sequence,
        record.ts_exchange_ns,
        record.ts_recv_ns,
        bytes_to_hex(&record.payload)
    )
}

pub(crate) fn market_data_wal_record_kind_name(kind: MarketDataWalRecordKind) -> &'static str {
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
    }
}

pub(crate) fn path_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

pub(crate) fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MarketDataCheckpointFrameInput<'a> {
    pub(crate) id: MarketDataCheckpointId,
    pub(crate) kind: MarketDataCheckpointKind,
    pub(crate) wal_sequence: MarketDataWalSequence,
    pub(crate) provider_sequence: u64,
    pub(crate) event_sequence: u64,
    pub(crate) created_ns: u64,
    pub(crate) payload_version: u32,
    pub(crate) payload: &'a [u8],
}

pub(crate) fn encode_market_data_checkpoint_frame(
    input: MarketDataCheckpointFrameInput<'_>,
) -> Vec<u8> {
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

pub(crate) fn decode_market_data_checkpoint_file(
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

pub(crate) fn read_market_data_checkpoint_manifest(
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

pub(crate) fn validate_market_data_checkpoint_file(
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

pub(crate) fn decode_market_data_checkpoint_header(
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

pub(crate) fn checkpoint_ids(dir: &Path) -> PersistResult<Vec<MarketDataCheckpointId>> {
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

pub(crate) fn checkpoint_path(dir: &Path, id: MarketDataCheckpointId) -> PathBuf {
    dir.join(format!("{:020}.ofmc", id.0))
}

pub(crate) fn checkpoint_temp_path(dir: &Path, id: MarketDataCheckpointId) -> PathBuf {
    dir.join(format!("{:020}.ofmc.tmp", id.0))
}

pub(crate) fn market_data_checkpoint_checksum(frame: &[u8]) -> u32 {
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

pub(crate) fn current_unix_nanos() -> u64 {
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

pub(crate) fn active_backpressure_reason(
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

pub(crate) fn drop_policy_to_action(
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

pub(crate) fn failure_action_to_backpressure_action(
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
