//! Segmented normalized market-data WAL and bounded writer ownership.

use super::{
    create_dir_all, encode_market_data_wal_frame_into, read_u32, scan_market_data_wal_into_from,
    MarketDataWalFrameInput, MarketDataWalMetrics, MarketDataWalRecord, MarketDataWalRecordKind,
    MarketDataWalReplayFilter, MarketDataWalReplayResult, MarketDataWalSequence, PersistError,
    PersistResult, MARKET_DATA_WAL_HEADER_LEN,
};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const SEGMENT_PREFIX: &str = "segment-";
const SEGMENT_SUFFIX: &str = ".ofmw";
const SEGMENT_MANIFEST: &str = "manifest.ofmm";
const SEGMENT_MANIFEST_TEMP: &str = "manifest.ofmm.tmp";
const SEGMENT_MANIFEST_MAGIC: &str = "OFMMS1";
const DEFAULT_SEGMENT_BYTES: u64 = 256 * 1024 * 1024;
const DEFAULT_MAX_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
const INITIAL_FRAME_CAPACITY: usize = 64 * 1024;

/// Monotonic market-data WAL segment identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MarketDataWalSegmentId(pub u64);

/// Sync cadence for segmented normalized market-data persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MarketDataWalSyncPolicy {
    /// Rely on the operating-system page cache until an explicit sync.
    Never,
    /// Call `sync_data` after every appended frame.
    EveryRecord,
    /// Call `sync_data` after every configured number of appended frames.
    EveryRecords(u64),
    /// Call `sync_data` when a segment is sealed.
    #[default]
    OnSegmentSeal,
}

/// Configuration for [`SegmentedMarketDataWal`].
#[derive(Debug, Clone)]
pub struct SegmentedMarketDataWalConfig {
    root: PathBuf,
    max_segment_bytes: u64,
    max_payload_bytes: usize,
    sync_policy: MarketDataWalSyncPolicy,
    sync_manifest: bool,
}

impl SegmentedMarketDataWalConfig {
    /// Creates a segmented WAL configuration rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            max_segment_bytes: DEFAULT_SEGMENT_BYTES,
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
            sync_policy: MarketDataWalSyncPolicy::default(),
            sync_manifest: true,
        }
    }

    /// Sets the soft segment byte target.
    pub const fn with_max_segment_bytes(mut self, max_segment_bytes: u64) -> Self {
        self.max_segment_bytes = max_segment_bytes;
        self
    }

    /// Sets the maximum accepted payload bytes for one record.
    pub const fn with_max_payload_bytes(mut self, max_payload_bytes: usize) -> Self {
        self.max_payload_bytes = max_payload_bytes;
        self
    }

    /// Sets data sync cadence.
    pub const fn with_sync_policy(mut self, sync_policy: MarketDataWalSyncPolicy) -> Self {
        self.sync_policy = sync_policy;
        self
    }

    /// Sets whether atomic manifest snapshots sync before rename.
    pub const fn with_sync_manifest(mut self, sync_manifest: bool) -> Self {
        self.sync_manifest = sync_manifest;
        self
    }

    /// Returns the segment root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the soft segment byte target.
    pub const fn max_segment_bytes(&self) -> u64 {
        self.max_segment_bytes
    }

    /// Returns the maximum payload size.
    pub const fn max_payload_bytes(&self) -> usize {
        self.max_payload_bytes
    }

    /// Returns data sync cadence.
    pub const fn sync_policy(&self) -> MarketDataWalSyncPolicy {
        self.sync_policy
    }

    /// Returns whether manifest snapshots sync before rename.
    pub const fn sync_manifest(&self) -> bool {
        self.sync_manifest
    }
}

/// One validated segment inventory row.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataWalSegmentMetadata {
    /// Segment id.
    pub id: MarketDataWalSegmentId,
    /// Segment path.
    pub path: PathBuf,
    /// Stored bytes.
    pub bytes: u64,
    /// Stored record count, including a seal record.
    pub records: u64,
    /// First global WAL sequence in the segment.
    pub first_sequence: Option<MarketDataWalSequence>,
    /// Last global WAL sequence in the segment.
    pub last_sequence: Option<MarketDataWalSequence>,
    /// Checksum of the final frame, or inherited checksum for an empty segment.
    pub last_checksum: u32,
    /// Whether the final frame is `SegmentSeal`.
    pub sealed: bool,
}

/// Rebuilt manifest for a segmented market-data WAL.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataWalManifest {
    /// Ordered segment inventory.
    pub segments: Vec<MarketDataWalSegmentMetadata>,
    /// Next global WAL sequence.
    pub next_sequence: MarketDataWalSequence,
    /// Checksum that the next frame must link to.
    pub previous_checksum: u32,
}

/// Aggregate fail-closed integrity report for a segment root.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataWalSegmentIntegrityReport {
    /// True when every discovered segment and cross-segment link is valid.
    pub valid: bool,
    /// Ordered validated segment inventory.
    pub segments: Vec<MarketDataWalSegmentMetadata>,
    /// Aggregate records.
    pub records: u64,
    /// Aggregate bytes.
    pub bytes: u64,
    /// Checksum/header failures.
    pub checksum_failures: u64,
    /// Sequence continuity failures.
    pub sequence_failures: u64,
    /// Whether a segment has a truncated tail.
    pub truncated_tail: bool,
    /// Last valid global sequence.
    pub last_sequence: Option<MarketDataWalSequence>,
}

/// Segmented writer counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SegmentedMarketDataWalMetrics {
    /// Aggregate frame metrics.
    pub wal: MarketDataWalMetrics,
    /// Segment rotations.
    pub rotations: u64,
    /// Explicit seal frames written.
    pub seals: u64,
    /// Atomic manifest snapshots installed.
    pub manifest_writes: u64,
    /// Manifest write/sync/rename failures.
    pub manifest_failures: u64,
}

#[derive(Debug)]
struct ActiveSegment {
    metadata: MarketDataWalSegmentMetadata,
    file: File,
}

/// Checksum-linked segmented binary WAL for normalized market-data events.
///
/// Record sequence and checksum links continue globally across segment files.
/// A manifest is an atomic recovery accelerator; segment scans remain the
/// source of truth and rebuild the manifest on every open.
#[derive(Debug)]
pub struct SegmentedMarketDataWal {
    config: SegmentedMarketDataWalConfig,
    manifest: MarketDataWalManifest,
    active: ActiveSegment,
    metrics: SegmentedMarketDataWalMetrics,
    records_since_sync: u64,
    frame_scratch: Vec<u8>,
}

impl SegmentedMarketDataWal {
    /// Opens or creates a segmented normalized market-data WAL.
    ///
    /// Existing segments are scanned in id order with global sequence and
    /// checksum continuity. Any corrupt, missing, duplicated, or unsealed
    /// non-final segment fails closed before append access is returned.
    pub fn open(config: SegmentedMarketDataWalConfig) -> PersistResult<Self> {
        validate_segmented_config(&config)?;
        create_dir_all(config.root())?;
        let report = inspect_segment_root(config.root())?;
        if !report.valid {
            return Err(invalid_data(
                "segmented market-data WAL failed integrity validation",
            ));
        }
        let next_sequence = report
            .last_sequence
            .map(|sequence| {
                sequence
                    .0
                    .checked_add(1)
                    .map(MarketDataWalSequence)
                    .ok_or_else(|| invalid_data("market-data WAL sequence space is exhausted"))
            })
            .transpose()?
            .unwrap_or(MarketDataWalSequence(1));
        let previous_checksum = report
            .segments
            .last()
            .map_or(0, |segment| segment.last_checksum);
        let mut segments = report.segments;
        let active = match segments.last() {
            Some(last) if !last.sealed => open_active_segment(last.clone())?,
            _ => {
                let id = match segments.last() {
                    Some(last) => {
                        MarketDataWalSegmentId(last.id.0.checked_add(1).ok_or_else(|| {
                            invalid_data("market-data WAL segment id space is exhausted")
                        })?)
                    }
                    None => MarketDataWalSegmentId(1),
                };
                let active = create_active_segment(config.root(), id, previous_checksum)?;
                segments.push(active.metadata.clone());
                active
            }
        };
        let manifest = MarketDataWalManifest {
            segments,
            next_sequence,
            previous_checksum,
        };
        let frame_capacity = MARKET_DATA_WAL_HEADER_LEN
            .saturating_add(config.max_payload_bytes.min(INITIAL_FRAME_CAPACITY));
        let mut wal = Self {
            config,
            manifest,
            active,
            metrics: SegmentedMarketDataWalMetrics::default(),
            records_since_sync: 0,
            frame_scratch: Vec::with_capacity(frame_capacity),
        };
        wal.install_manifest()?;
        Ok(wal)
    }

    /// Returns configuration.
    pub const fn config(&self) -> &SegmentedMarketDataWalConfig {
        &self.config
    }

    /// Returns the next global sequence.
    pub const fn next_sequence(&self) -> MarketDataWalSequence {
        self.manifest.next_sequence
    }

    /// Returns a rebuilt in-memory manifest snapshot.
    pub const fn manifest(&self) -> &MarketDataWalManifest {
        &self.manifest
    }

    /// Returns counters.
    pub const fn metrics(&self) -> SegmentedMarketDataWalMetrics {
        self.metrics
    }

    /// Appends one normalized record, rotating and sealing as needed.
    pub fn append_record(
        &mut self,
        kind: MarketDataWalRecordKind,
        provider_sequence: u64,
        event_sequence: u64,
        ts_exchange_ns: u64,
        ts_recv_ns: u64,
        payload: &[u8],
    ) -> PersistResult<MarketDataWalSequence> {
        if kind == MarketDataWalRecordKind::SegmentSeal {
            return Err(invalid_input(
                "use seal_active_segment to append a segment seal",
            ));
        }
        if payload.len() > self.config.max_payload_bytes {
            return Err(invalid_input(
                "market-data WAL payload exceeds configured maximum",
            ));
        }
        if self.active.metadata.sealed {
            self.rotate_to_new_segment()?;
        }
        let candidate_len = MARKET_DATA_WAL_HEADER_LEN.saturating_add(payload.len()) as u64;
        let seal_reserve = MARKET_DATA_WAL_HEADER_LEN as u64;
        if self.active.metadata.records > 0
            && self
                .active
                .metadata
                .bytes
                .saturating_add(candidate_len)
                .saturating_add(seal_reserve)
                > self.config.max_segment_bytes
        {
            self.seal_active_segment()?;
            self.rotate_to_new_segment()?;
        }
        self.append_frame(
            kind,
            provider_sequence,
            event_sequence,
            ts_exchange_ns,
            ts_recv_ns,
            payload,
        )
    }

    /// Writes an explicit seal frame and atomically refreshes the manifest.
    pub fn seal_active_segment(&mut self) -> PersistResult<()> {
        if self.active.metadata.sealed {
            return Ok(());
        }
        self.append_frame(MarketDataWalRecordKind::SegmentSeal, 0, 0, 0, 0, &[])?;
        self.metrics.seals = self.metrics.seals.saturating_add(1);
        self.active.metadata.sealed = true;
        self.refresh_active_manifest_row();
        if self.config.sync_policy == MarketDataWalSyncPolicy::OnSegmentSeal {
            self.sync_active_data()?;
        }
        self.install_manifest()
    }

    /// Flushes userspace buffers and calls `sync_data` on the active segment.
    pub fn sync_data(&mut self) -> PersistResult<()> {
        self.sync_active_data()
    }

    /// Replays every segment in global sequence order.
    pub fn replay(
        &self,
        out: &mut Vec<MarketDataWalRecord>,
    ) -> PersistResult<MarketDataWalReplayResult> {
        self.replay_filtered(MarketDataWalReplayFilter::new(), out)
    }

    /// Replays matching records while validating every segment and link.
    pub fn replay_filtered(
        &self,
        filter: MarketDataWalReplayFilter,
        out: &mut Vec<MarketDataWalRecord>,
    ) -> PersistResult<MarketDataWalReplayResult> {
        let before = out.len();
        let mut expected = MarketDataWalSequence(1);
        let mut previous_checksum = 0u32;
        let mut bytes = 0u64;
        for segment in &self.manifest.segments {
            let scan = match scan_market_data_wal_into_from(
                &segment.path,
                expected,
                previous_checksum,
                Some((out, filter)),
            ) {
                Ok(scan) => scan,
                Err(error) => {
                    out.truncate(before);
                    return Err(error);
                }
            };
            if !scan.report.valid {
                out.truncate(before);
                return Err(invalid_data(
                    "segmented market-data WAL replay failed integrity validation",
                ));
            }
            bytes = bytes.saturating_add(scan.report.bytes);
            previous_checksum = scan.previous_checksum;
            if let Some(last) = scan.report.last_sequence {
                expected = MarketDataWalSequence(last.0.saturating_add(1));
            }
        }
        let records = out.len().saturating_sub(before);
        Ok(MarketDataWalReplayResult {
            records,
            bytes,
            first_sequence: out.get(before).map(|record| record.sequence),
            last_sequence: records
                .checked_sub(1)
                .and_then(|offset| out.get(before + offset))
                .map(|record| record.sequence),
        })
    }

    /// Inspects an existing segment root without creating or changing files.
    pub fn inspect_root(
        root: impl AsRef<Path>,
    ) -> PersistResult<MarketDataWalSegmentIntegrityReport> {
        inspect_segment_root(root.as_ref())
    }

    fn append_frame(
        &mut self,
        kind: MarketDataWalRecordKind,
        provider_sequence: u64,
        event_sequence: u64,
        ts_exchange_ns: u64,
        ts_recv_ns: u64,
        payload: &[u8],
    ) -> PersistResult<MarketDataWalSequence> {
        let sequence = self.manifest.next_sequence;
        if sequence.0 == u64::MAX {
            return Err(invalid_data("market-data WAL sequence space is exhausted"));
        }
        encode_market_data_wal_frame_into(
            &mut self.frame_scratch,
            MarketDataWalFrameInput {
                sequence,
                kind,
                provider_sequence,
                event_sequence,
                ts_exchange_ns,
                ts_recv_ns,
                payload,
                previous_checksum: self.manifest.previous_checksum,
            },
        )?;
        if let Err(error) = self.active.file.write_all(&self.frame_scratch) {
            self.metrics.wal.write_failures = self.metrics.wal.write_failures.saturating_add(1);
            return Err(error.into());
        }

        let checksum = read_u32(&self.frame_scratch[52..56]);
        self.active.metadata.bytes = self
            .active
            .metadata
            .bytes
            .saturating_add(self.frame_scratch.len() as u64);
        self.active.metadata.records = self.active.metadata.records.saturating_add(1);
        self.active.metadata.first_sequence.get_or_insert(sequence);
        self.active.metadata.last_sequence = Some(sequence);
        self.active.metadata.last_checksum = checksum;
        self.active.metadata.sealed = kind == MarketDataWalRecordKind::SegmentSeal;
        self.manifest.next_sequence = MarketDataWalSequence(sequence.0 + 1);
        self.manifest.previous_checksum = checksum;
        self.metrics.wal.records_written = self.metrics.wal.records_written.saturating_add(1);
        self.metrics.wal.bytes_written = self
            .metrics
            .wal
            .bytes_written
            .saturating_add(self.frame_scratch.len() as u64);
        self.records_since_sync = self.records_since_sync.saturating_add(1);
        self.refresh_active_manifest_row();

        let should_sync = match self.config.sync_policy {
            MarketDataWalSyncPolicy::Never | MarketDataWalSyncPolicy::OnSegmentSeal => false,
            MarketDataWalSyncPolicy::EveryRecord => true,
            MarketDataWalSyncPolicy::EveryRecords(records) => self.records_since_sync >= records,
        };
        if should_sync {
            self.sync_active_data()?;
        }
        Ok(sequence)
    }

    fn rotate_to_new_segment(&mut self) -> PersistResult<()> {
        let id = MarketDataWalSegmentId(
            self.active
                .metadata
                .id
                .0
                .checked_add(1)
                .ok_or_else(|| invalid_data("market-data WAL segment id space is exhausted"))?,
        );
        let active =
            create_active_segment(self.config.root(), id, self.manifest.previous_checksum)?;
        self.active = active;
        self.manifest.segments.push(self.active.metadata.clone());
        self.metrics.rotations = self.metrics.rotations.saturating_add(1);
        self.install_manifest()
    }

    fn sync_active_data(&mut self) -> PersistResult<()> {
        if let Err(error) = self
            .active
            .file
            .flush()
            .and_then(|()| self.active.file.sync_data())
        {
            self.metrics.wal.sync_failures = self.metrics.wal.sync_failures.saturating_add(1);
            return Err(error.into());
        }
        self.metrics.wal.sync_count = self.metrics.wal.sync_count.saturating_add(1);
        self.records_since_sync = 0;
        Ok(())
    }

    fn refresh_active_manifest_row(&mut self) {
        if let Some(last) = self.manifest.segments.last_mut() {
            *last = self.active.metadata.clone();
        }
    }

    fn install_manifest(&mut self) -> PersistResult<()> {
        match write_manifest(&self.config, &self.manifest) {
            Ok(()) => {
                self.metrics.manifest_writes = self.metrics.manifest_writes.saturating_add(1);
                Ok(())
            }
            Err(error) => {
                self.metrics.manifest_failures = self.metrics.manifest_failures.saturating_add(1);
                Err(error)
            }
        }
    }
}

fn validate_segmented_config(config: &SegmentedMarketDataWalConfig) -> PersistResult<()> {
    if config.max_segment_bytes < (MARKET_DATA_WAL_HEADER_LEN * 2) as u64 {
        return Err(invalid_input("segment byte target is too small"));
    }
    if config.max_payload_bytes == 0 {
        return Err(invalid_input("maximum market-data WAL payload is zero"));
    }
    if matches!(config.sync_policy, MarketDataWalSyncPolicy::EveryRecords(0)) {
        return Err(invalid_input(
            "market-data WAL sync record interval is zero",
        ));
    }
    Ok(())
}

fn inspect_segment_root(root: &Path) -> PersistResult<MarketDataWalSegmentIntegrityReport> {
    if !root.exists() {
        return Ok(MarketDataWalSegmentIntegrityReport {
            valid: true,
            ..MarketDataWalSegmentIntegrityReport::default()
        });
    }
    let paths = segment_paths(root)?;
    let mut report = MarketDataWalSegmentIntegrityReport {
        valid: true,
        ..MarketDataWalSegmentIntegrityReport::default()
    };
    let mut expected_sequence = MarketDataWalSequence(1);
    let mut previous_checksum = 0u32;
    let mut previous_id = None;
    let mut previous_sealed = true;
    for (index, (id, path)) in paths.iter().enumerate() {
        if let Some(previous_id) = previous_id {
            if id.0 != previous_id + 1 {
                report.valid = false;
                report.sequence_failures = report.sequence_failures.saturating_add(1);
            }
            if !previous_sealed {
                report.valid = false;
                report.sequence_failures = report.sequence_failures.saturating_add(1);
            }
        } else if id.0 != 1 {
            report.valid = false;
            report.sequence_failures = report.sequence_failures.saturating_add(1);
        }
        let initial_checksum = previous_checksum;
        let scan =
            scan_market_data_wal_into_from(path, expected_sequence, previous_checksum, None)?;
        let sealed = scan.last_kind == Some(MarketDataWalRecordKind::SegmentSeal);
        let first_sequence = (scan.report.records > 0).then_some(expected_sequence);
        let metadata = MarketDataWalSegmentMetadata {
            id: *id,
            path: path.clone(),
            bytes: scan.report.bytes,
            records: scan.report.records,
            first_sequence,
            last_sequence: scan.report.last_sequence,
            last_checksum: scan.previous_checksum,
            sealed,
        };
        report.valid &= scan.report.valid;
        report.records = report.records.saturating_add(scan.report.records);
        report.bytes = report.bytes.saturating_add(scan.report.bytes);
        report.checksum_failures = report
            .checksum_failures
            .saturating_add(scan.report.checksum_failures);
        report.sequence_failures = report
            .sequence_failures
            .saturating_add(scan.report.sequence_failures);
        report.truncated_tail |= scan.report.truncated_tail;
        if let Some(last) = scan.report.last_sequence {
            expected_sequence = MarketDataWalSequence(last.0.saturating_add(1));
            report.last_sequence = Some(last);
        }
        previous_checksum = scan.previous_checksum;
        previous_id = Some(id.0);
        previous_sealed = sealed || index + 1 == paths.len();
        if scan.report.records == 0 {
            previous_checksum = initial_checksum;
        }
        report.segments.push(metadata);
    }
    Ok(report)
}

fn segment_paths(root: &Path) -> PersistResult<Vec<(MarketDataWalSegmentId, PathBuf)>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(id_text) = name
            .strip_prefix(SEGMENT_PREFIX)
            .and_then(|rest| rest.strip_suffix(SEGMENT_SUFFIX))
        else {
            continue;
        };
        let id = id_text
            .parse::<u64>()
            .map_err(|_| invalid_data("invalid market-data WAL segment filename"))?;
        if id == 0 {
            return Err(invalid_data("market-data WAL segment id is zero"));
        }
        paths.push((MarketDataWalSegmentId(id), entry.path()));
    }
    paths.sort_unstable_by_key(|(id, _)| *id);
    for pair in paths.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(invalid_data("duplicate market-data WAL segment id"));
        }
    }
    Ok(paths)
}

fn create_active_segment(
    root: &Path,
    id: MarketDataWalSegmentId,
    previous_checksum: u32,
) -> PersistResult<ActiveSegment> {
    let path = segment_path(root, id);
    let file = OpenOptions::new()
        .create_new(true)
        .append(true)
        .read(true)
        .open(&path)?;
    Ok(ActiveSegment {
        metadata: MarketDataWalSegmentMetadata {
            id,
            path,
            bytes: 0,
            records: 0,
            first_sequence: None,
            last_sequence: None,
            last_checksum: previous_checksum,
            sealed: false,
        },
        file,
    })
}

fn open_active_segment(metadata: MarketDataWalSegmentMetadata) -> PersistResult<ActiveSegment> {
    let file = OpenOptions::new()
        .append(true)
        .read(true)
        .open(&metadata.path)?;
    Ok(ActiveSegment { metadata, file })
}

fn segment_path(root: &Path, id: MarketDataWalSegmentId) -> PathBuf {
    root.join(format!("{SEGMENT_PREFIX}{:020}{SEGMENT_SUFFIX}", id.0))
}

fn write_manifest(
    config: &SegmentedMarketDataWalConfig,
    manifest: &MarketDataWalManifest,
) -> PersistResult<()> {
    let mut body = String::new();
    body.push_str(SEGMENT_MANIFEST_MAGIC);
    body.push('\n');
    body.push_str(&format!(
        "next={},previous={}\n",
        manifest.next_sequence.0, manifest.previous_checksum
    ));
    for segment in &manifest.segments {
        let filename = segment
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid_data("market-data WAL segment filename is not UTF-8"))?;
        body.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            segment.id.0,
            segment.first_sequence.map_or(0, |value| value.0),
            segment.last_sequence.map_or(0, |value| value.0),
            segment.records,
            segment.bytes,
            segment.last_checksum,
            u8::from(segment.sealed),
            filename
        ));
    }
    let checksum = manifest_checksum(body.as_bytes());
    body.push_str(&format!("checksum={checksum}\n"));
    let temp = config.root().join(SEGMENT_MANIFEST_TEMP);
    let final_path = config.root().join(SEGMENT_MANIFEST);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp)?;
    file.write_all(body.as_bytes())?;
    file.flush()?;
    if config.sync_manifest {
        file.sync_data()?;
    }
    drop(file);
    fs::rename(temp, final_path)?;
    Ok(())
}

fn manifest_checksum(bytes: &[u8]) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

fn invalid_data(message: &'static str) -> PersistError {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message).into()
}

fn invalid_input(message: &'static str) -> PersistError {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom};
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

    fn rotating_config(root: &Path) -> SegmentedMarketDataWalConfig {
        SegmentedMarketDataWalConfig::new(root)
            .with_max_segment_bytes((MARKET_DATA_WAL_HEADER_LEN * 2 + 16) as u64)
            .with_max_payload_bytes(128)
            .with_sync_policy(MarketDataWalSyncPolicy::Never)
            .with_sync_manifest(false)
    }

    fn append_trade(wal: &mut SegmentedMarketDataWal, event_sequence: u64, payload: &[u8]) {
        wal.append_record(
            MarketDataWalRecordKind::TradePrint,
            event_sequence,
            event_sequence,
            event_sequence * 10,
            event_sequence * 10 + 1,
            payload,
        )
        .expect("append trade");
    }

    #[test]
    fn rotates_with_global_sequence_and_checksum_continuity() {
        let root = TestRoot::new("segmented-rotate");
        let mut wal = SegmentedMarketDataWal::open(rotating_config(&root.0)).expect("open");

        append_trade(&mut wal, 1, b"one");
        append_trade(&mut wal, 2, b"two");
        append_trade(&mut wal, 3, b"three");
        wal.seal_active_segment().expect("seal");

        let report = SegmentedMarketDataWal::inspect_root(&root.0).expect("inspect");
        assert!(report.valid);
        assert_eq!(report.segments.len(), 3);
        assert!(report.segments.iter().all(|segment| segment.sealed));
        assert_eq!(report.records, 6);
        assert_eq!(report.last_sequence, Some(MarketDataWalSequence(6)));
        assert_eq!(wal.metrics().rotations, 2);
        assert_eq!(wal.metrics().seals, 3);

        let mut records = Vec::new();
        let replay = wal.replay(&mut records).expect("replay");
        assert_eq!(replay.records, 6);
        assert_eq!(
            records
                .iter()
                .map(|record| record.sequence.0)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6]
        );
        assert_eq!(records[0].payload, b"one");
        assert_eq!(records[2].payload, b"two");
        assert_eq!(records[4].payload, b"three");
    }

    #[test]
    fn replay_filter_validates_all_segments_but_materializes_matches() {
        let root = TestRoot::new("segmented-filter");
        let mut wal = SegmentedMarketDataWal::open(rotating_config(&root.0)).expect("open");
        append_trade(&mut wal, 1, b"one");
        wal.append_record(MarketDataWalRecordKind::BookUpdate, 2, 2, 20, 21, b"book")
            .expect("append book");
        append_trade(&mut wal, 3, b"three");

        let mut records = Vec::new();
        let result = wal
            .replay_filtered(
                MarketDataWalReplayFilter::new()
                    .with_kind(Some(MarketDataWalRecordKind::TradePrint)),
                &mut records,
            )
            .expect("filtered replay");
        assert_eq!(result.records, 2);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].payload, b"one");
        assert_eq!(records[1].payload, b"three");
    }

    #[test]
    fn reopens_unsealed_and_sealed_final_segments_deterministically() {
        let root = TestRoot::new("segmented-reopen");
        let config = SegmentedMarketDataWalConfig::new(&root.0)
            .with_max_segment_bytes(1024)
            .with_sync_manifest(false);
        {
            let mut wal = SegmentedMarketDataWal::open(config.clone()).expect("open");
            append_trade(&mut wal, 1, b"one");
        }
        {
            let mut wal = SegmentedMarketDataWal::open(config.clone()).expect("reopen active");
            assert_eq!(wal.next_sequence(), MarketDataWalSequence(2));
            assert_eq!(wal.manifest().segments.len(), 1);
            append_trade(&mut wal, 2, b"two");
            wal.seal_active_segment().expect("seal");
        }
        let wal = SegmentedMarketDataWal::open(config).expect("reopen sealed");
        assert_eq!(wal.next_sequence(), MarketDataWalSequence(4));
        assert_eq!(wal.manifest().segments.len(), 2);
        assert!(wal.manifest().segments[0].sealed);
        assert_eq!(wal.manifest().segments[1].records, 0);
    }

    #[test]
    fn seals_empty_segment_on_disk() {
        let root = TestRoot::new("segmented-empty-seal");
        let config = SegmentedMarketDataWalConfig::new(&root.0).with_sync_manifest(false);
        {
            let mut wal = SegmentedMarketDataWal::open(config.clone()).expect("open");
            wal.seal_active_segment().expect("seal empty");
            assert_eq!(wal.next_sequence(), MarketDataWalSequence(2));
        }
        let report = SegmentedMarketDataWal::inspect_root(&root.0).expect("inspect");
        assert!(report.valid);
        assert_eq!(report.records, 1);
        assert!(report.segments[0].sealed);
        let reopened = SegmentedMarketDataWal::open(config).expect("reopen");
        assert_eq!(reopened.manifest().segments.len(), 2);
    }

    #[test]
    fn rejects_payload_and_reserved_seal_without_mutation() {
        let root = TestRoot::new("segmented-limits");
        let config = SegmentedMarketDataWalConfig::new(&root.0)
            .with_max_payload_bytes(3)
            .with_sync_manifest(false);
        let mut wal = SegmentedMarketDataWal::open(config).expect("open");
        assert!(wal
            .append_record(MarketDataWalRecordKind::TradePrint, 1, 1, 1, 1, b"four")
            .is_err());
        assert!(wal
            .append_record(MarketDataWalRecordKind::SegmentSeal, 0, 0, 0, 0, b"")
            .is_err());
        assert_eq!(wal.next_sequence(), MarketDataWalSequence(1));
        assert_eq!(wal.manifest().segments[0].records, 0);
    }

    #[test]
    fn rebuilds_atomic_manifest_from_segment_truth() {
        let root = TestRoot::new("segmented-manifest");
        let config = SegmentedMarketDataWalConfig::new(&root.0).with_sync_manifest(false);
        {
            let mut wal = SegmentedMarketDataWal::open(config.clone()).expect("open");
            append_trade(&mut wal, 1, b"one");
        }
        let manifest_path = root.0.join(SEGMENT_MANIFEST);
        fs::write(&manifest_path, b"corrupt manifest").expect("corrupt manifest");

        let wal = SegmentedMarketDataWal::open(config).expect("rebuild");
        assert_eq!(wal.next_sequence(), MarketDataWalSequence(2));
        let manifest = fs::read_to_string(manifest_path).expect("read manifest");
        assert!(manifest.starts_with(SEGMENT_MANIFEST_MAGIC));
        assert!(manifest.contains("checksum="));
        assert!(!root.0.join(SEGMENT_MANIFEST_TEMP).exists());
    }

    #[test]
    fn detects_corruption_and_rolls_back_replay_output() {
        let root = TestRoot::new("segmented-corrupt");
        let config = SegmentedMarketDataWalConfig::new(&root.0).with_sync_manifest(false);
        let mut wal = SegmentedMarketDataWal::open(config).expect("open");
        append_trade(&mut wal, 1, b"one");
        wal.sync_data().expect("sync");
        let path = wal.manifest().segments[0].path.clone();
        let mut bytes = fs::read(&path).expect("read segment");
        bytes[MARKET_DATA_WAL_HEADER_LEN] ^= 0xff;
        fs::write(&path, bytes).expect("corrupt segment");

        let report = SegmentedMarketDataWal::inspect_root(&root.0).expect("inspect");
        assert!(!report.valid);
        assert_eq!(report.checksum_failures, 1);
        let mut out = vec![MarketDataWalRecord {
            sequence: MarketDataWalSequence(99),
            kind: MarketDataWalRecordKind::Heartbeat,
            provider_sequence: 0,
            event_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
            payload: Vec::new(),
        }];
        assert!(wal.replay(&mut out).is_err());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].sequence, MarketDataWalSequence(99));
    }

    #[test]
    fn detects_missing_segment_and_cross_segment_link_breaks() {
        let root = TestRoot::new("segmented-link");
        let mut wal = SegmentedMarketDataWal::open(rotating_config(&root.0)).expect("open");
        append_trade(&mut wal, 1, b"one");
        append_trade(&mut wal, 2, b"two");
        wal.sync_data().expect("sync");
        let first = wal.manifest().segments[0].path.clone();
        fs::remove_file(first).expect("remove first segment");

        let report = SegmentedMarketDataWal::inspect_root(&root.0).expect("inspect");
        assert!(!report.valid);
        assert!(report.sequence_failures > 0);
        assert!(report.checksum_failures > 0);
    }

    #[test]
    fn applies_record_sync_cadence_and_reuses_frame_capacity() {
        let root = TestRoot::new("segmented-sync");
        let config = SegmentedMarketDataWalConfig::new(&root.0)
            .with_sync_policy(MarketDataWalSyncPolicy::EveryRecords(2))
            .with_sync_manifest(false);
        let mut wal = SegmentedMarketDataWal::open(config).expect("open");
        append_trade(&mut wal, 1, b"payload");
        let capacity = wal.frame_scratch.capacity();
        append_trade(&mut wal, 2, b"x");
        append_trade(&mut wal, 3, b"y");
        assert_eq!(wal.metrics().wal.sync_count, 1);
        assert_eq!(wal.frame_scratch.capacity(), capacity);
        wal.seal_active_segment().expect("seal");
        assert_eq!(wal.metrics().wal.sync_count, 2);
    }

    #[test]
    fn validates_configuration_before_creating_files() {
        let root = TestRoot::new("segmented-config");
        let too_small = SegmentedMarketDataWalConfig::new(&root.0)
            .with_max_segment_bytes(1)
            .with_sync_manifest(false);
        assert!(SegmentedMarketDataWal::open(too_small).is_err());
        assert!(!root.0.exists());

        let zero_payload = SegmentedMarketDataWalConfig::new(&root.0)
            .with_max_payload_bytes(0)
            .with_sync_manifest(false);
        assert!(SegmentedMarketDataWal::open(zero_payload).is_err());

        let zero_sync = SegmentedMarketDataWalConfig::new(&root.0)
            .with_sync_policy(MarketDataWalSyncPolicy::EveryRecords(0))
            .with_sync_manifest(false);
        assert!(SegmentedMarketDataWal::open(zero_sync).is_err());
    }

    #[test]
    fn manifest_checksum_changes_when_inventory_changes() {
        assert_ne!(manifest_checksum(b"one"), manifest_checksum(b"two"));
    }

    #[test]
    fn segment_file_can_be_read_while_writer_is_open() {
        let root = TestRoot::new("segmented-readable");
        let config = SegmentedMarketDataWalConfig::new(&root.0).with_sync_manifest(false);
        let mut wal = SegmentedMarketDataWal::open(config).expect("open");
        append_trade(&mut wal, 1, b"one");
        wal.sync_data().expect("sync");
        let mut file = File::open(&wal.manifest().segments[0].path).expect("open reader");
        file.seek(SeekFrom::Start(MARKET_DATA_WAL_HEADER_LEN as u64))
            .expect("seek payload");
        let mut payload = [0_u8; 3];
        file.read_exact(&mut payload).expect("read payload");
        assert_eq!(&payload, b"one");
    }
}
