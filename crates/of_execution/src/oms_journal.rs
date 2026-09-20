use super::*;

/// Durable append-only execution journal.
#[derive(Debug)]
pub struct FileExecutionJournal {
    pub(crate) path: PathBuf,
    pub(crate) file: File,
    pub(crate) sync_on_write: bool,
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
    pub(crate) path: PathBuf,
    pub(crate) sync_policy: WalSyncPolicy,
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
    pub(crate) root: PathBuf,
    pub(crate) sync_policy: WalSyncPolicy,
    pub(crate) max_segment_bytes: u64,
    pub(crate) max_segment_records: u64,
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
    pub(crate) config: WalJournalConfig,
    pub(crate) file: File,
    pub(crate) next_sequence: WalSequence,
    pub(crate) previous_checksum: u64,
    pub(crate) records_since_sync: u32,
    pub(crate) last_sync_ns: u64,
    pub(crate) metrics: WalJournalMetrics,
    pub(crate) scratch: Vec<u8>,
    pub(crate) frame_scratch: Vec<u8>,
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

    fn record_submit(&mut self, request: &OrderRequest) -> ExecutionResult<()> {
        self.scratch.clear();
        encode_submit_payload(request, &mut self.scratch);
        self.append_record(WalRecordKind::CommandSubmit, request.ts_recv_ns)
    }

    fn record_cancel(&mut self, request: &CancelRequest) -> ExecutionResult<()> {
        self.scratch.clear();
        encode_cancel_payload(request, &mut self.scratch);
        self.append_record(WalRecordKind::CommandCancel, request.ts_recv_ns)
    }

    fn record_amend(&mut self, request: &AmendRequest) -> ExecutionResult<()> {
        self.scratch.clear();
        encode_amend_payload(request, &mut self.scratch);
        self.append_record(WalRecordKind::CommandAmend, request.ts_recv_ns)
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
    pub(crate) config: WalSegmentConfig,
    pub(crate) manifest: WalSegmentManifest,
    pub(crate) file: File,
    pub(crate) next_sequence: WalSequence,
    pub(crate) previous_checksum: u64,
    pub(crate) records_since_sync: u32,
    pub(crate) last_sync_ns: u64,
    pub(crate) metrics: WalJournalMetrics,
    pub(crate) scratch: Vec<u8>,
    pub(crate) frame_scratch: Vec<u8>,
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
                result.first_sequence.get_or_insert(record.header.sequence);
                result.last_sequence = Some(record.header.sequence);
                if let Some(journal_record) = decode_wal_payload(&record)? {
                    out.push(journal_record);
                    result.records = out.len().saturating_sub(start);
                }
            }
        }
        Ok(result)
    }

    pub(crate) fn replay_recovery_from(
        &self,
        sequence: WalSequence,
        out: &mut Vec<DecodedRecoveryRecord>,
    ) -> ExecutionResult<WalReplayResult> {
        replay_segmented_recovery_records(
            self.manifest
                .segments
                .iter()
                .map(|segment| segment.path.as_path()),
            sequence,
            out,
        )
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

    fn record_submit(&mut self, request: &OrderRequest) -> ExecutionResult<()> {
        self.scratch.clear();
        encode_submit_payload(request, &mut self.scratch);
        self.append_record(WalRecordKind::CommandSubmit, request.ts_recv_ns)
    }

    fn record_cancel(&mut self, request: &CancelRequest) -> ExecutionResult<()> {
        self.scratch.clear();
        encode_cancel_payload(request, &mut self.scratch);
        self.append_record(WalRecordKind::CommandCancel, request.ts_recv_ns)
    }

    fn record_amend(&mut self, request: &AmendRequest) -> ExecutionResult<()> {
        self.scratch.clear();
        encode_amend_payload(request, &mut self.scratch);
        self.append_record(WalRecordKind::CommandAmend, request.ts_recv_ns)
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
const WAL_COMMAND_PAYLOAD_VERSION: u16 = 2;

pub(crate) fn scan_wal_file(path: &Path) -> ExecutionResult<(WalSequence, u64)> {
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
pub(crate) struct SegmentScan {
    pub(crate) first_sequence: Option<WalSequence>,
    pub(crate) last_sequence: Option<WalSequence>,
    pub(crate) next_sequence: WalSequence,
    pub(crate) previous_checksum: u64,
    pub(crate) records: u64,
    pub(crate) bytes: u64,
    pub(crate) sealed: bool,
}

pub(crate) fn load_segment_manifest(
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

pub(crate) fn inspect_segmented_wal_root(
    root: &Path,
) -> ExecutionResult<WalSegmentIntegrityReport> {
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

pub(crate) fn scan_segment_file(
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

pub(crate) fn validate_wal_sequence(
    actual: WalSequence,
    expected: WalSequence,
) -> ExecutionResult<()> {
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

pub(crate) fn list_segment_ids(root: &Path) -> ExecutionResult<Vec<WalSegmentId>> {
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

pub(crate) fn parse_segment_id(path: &Path) -> Option<WalSegmentId> {
    let file_name = path.file_name()?.to_str()?;
    let digits = file_name.strip_prefix("wal-")?.strip_suffix(".ofwal")?;
    digits.parse::<u64>().ok().map(WalSegmentId)
}

pub(crate) fn segment_path(root: &Path, segment_id: WalSegmentId) -> PathBuf {
    root.join(format!("wal-{:012}.ofwal", segment_id.0))
}

pub(crate) fn write_segment_manifest(
    root: &Path,
    manifest: &WalSegmentManifest,
) -> ExecutionResult<()> {
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

pub(crate) fn replay_wal_bytes(
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
        result.first_sequence.get_or_insert(record.header.sequence);
        result.last_sequence = Some(record.header.sequence);
        if let Some(journal_record) = decode_wal_payload(&record)? {
            out.push(journal_record);
            result.records = out.len().saturating_sub(start);
        }
    }

    Ok(result)
}

pub(crate) fn replay_segmented_recovery_records<'a>(
    paths: impl IntoIterator<Item = &'a Path>,
    from_sequence: WalSequence,
    out: &mut Vec<DecodedRecoveryRecord>,
) -> ExecutionResult<WalReplayResult> {
    let start = out.len();
    let mut result = WalReplayResult::default();
    let mut previous_checksum = 0;
    let mut expected_sequence = WalSequence(1);

    for path in paths {
        let bytes = fs::read(path).map_err(|err| ExecutionError::Journal(err.to_string()))?;
        let mut cursor = WalReplayCursor::new(&bytes).with_strict_sequence(false);
        while let Some(record) = cursor.next_record().map_err(wal_error)? {
            validate_wal_sequence(record.header.sequence, expected_sequence)?;
            validate_wal_link(&record, previous_checksum)?;
            previous_checksum = record.header.header_checksum;
            expected_sequence = record.header.sequence.next();
            result.bytes = result.bytes.saturating_add(record.encoded_len() as u64);
            if record.header.sequence < from_sequence {
                continue;
            }
            result.first_sequence.get_or_insert(record.header.sequence);
            result.last_sequence = Some(record.header.sequence);
            if let Some(recovery_record) = decode_recovery_wal_payload(&record)? {
                out.push(recovery_record);
                result.records = out.len().saturating_sub(start);
            }
        }
    }
    Ok(result)
}

pub(crate) fn validate_wal_link(
    record: &WalRecordView<'_>,
    previous_checksum: u64,
) -> ExecutionResult<()> {
    if record.header.previous_checksum == previous_checksum {
        Ok(())
    } else {
        Err(ExecutionError::Journal(format!(
            "WAL checksum link mismatch at sequence {}",
            record.header.sequence.0
        )))
    }
}

pub(crate) fn encode_command_payload(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecodedWalCommand {
    Legacy {
        kind: JournalCommandKind,
        client_order_id: ClientOrderId,
        ts_ns: u64,
    },
    Submit(OrderRequest),
    Cancel(CancelRequest),
    Amend(AmendRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DecodedRecoveryRecord {
    Command(Box<DecodedWalCommand>),
    Event(Box<ExecutionEvent>),
}

impl DecodedWalCommand {
    const fn kind(self) -> JournalCommandKind {
        match self {
            Self::Legacy { kind, .. } => kind,
            Self::Submit(_) => JournalCommandKind::Submit,
            Self::Cancel(_) => JournalCommandKind::Cancel,
            Self::Amend(_) => JournalCommandKind::Amend,
        }
    }

    const fn client_order_id(self) -> ClientOrderId {
        match self {
            Self::Legacy {
                client_order_id, ..
            } => client_order_id,
            Self::Submit(request) => request.client_order_id,
            Self::Cancel(request) => request.client_order_id,
            Self::Amend(request) => request.client_order_id,
        }
    }

    const fn timestamp_ns(self) -> u64 {
        match self {
            Self::Legacy { ts_ns, .. } => ts_ns,
            Self::Submit(request) => request.ts_recv_ns,
            Self::Cancel(request) => request.ts_recv_ns,
            Self::Amend(request) => request.ts_recv_ns,
        }
    }

    const fn legacy_record(self) -> JournalRecord {
        JournalRecord::Command {
            kind: self.kind(),
            client_order_id: self.client_order_id(),
            ts_ns: self.timestamp_ns(),
        }
    }
}

pub(crate) fn encode_submit_payload(request: &OrderRequest, out: &mut Vec<u8>) {
    put_payload_u16(out, WAL_COMMAND_PAYLOAD_VERSION);
    put_payload_u8(out, command_kind_u8(JournalCommandKind::Submit));
    put_payload_u8(out, 0);
    put_fixed(out, &request.client_order_id);
    put_fixed(out, &request.account_id);
    put_fixed(out, &request.route_id);
    put_fixed(out, &request.strategy_id);
    put_fixed(out, &request.symbol.venue);
    put_fixed(out, &request.symbol.instrument);
    put_payload_u8(out, request.side as u8);
    put_payload_u8(out, request.order_type as u8);
    put_payload_u8(out, request.time_in_force as u8);
    put_payload_u8(out, 0);
    put_payload_i64(out, request.quantity.0);
    put_payload_i64(out, request.limit_price.0);
    put_payload_i64(out, request.stop_price.0);
    put_payload_u64(out, request.ts_exchange_ns);
    put_payload_u64(out, request.ts_recv_ns);
}

pub(crate) fn encode_cancel_payload(request: &CancelRequest, out: &mut Vec<u8>) {
    put_payload_u16(out, WAL_COMMAND_PAYLOAD_VERSION);
    put_payload_u8(out, command_kind_u8(JournalCommandKind::Cancel));
    put_payload_u8(out, 0);
    put_fixed(out, &request.client_order_id);
    put_fixed(out, &request.orig_client_order_id);
    put_fixed(out, &request.venue_order_id);
    put_fixed(out, &request.account_id);
    put_fixed(out, &request.route_id);
    put_fixed(out, &request.symbol.venue);
    put_fixed(out, &request.symbol.instrument);
    put_payload_u64(out, request.ts_recv_ns);
}

pub(crate) fn encode_amend_payload(request: &AmendRequest, out: &mut Vec<u8>) {
    put_payload_u16(out, WAL_COMMAND_PAYLOAD_VERSION);
    put_payload_u8(out, command_kind_u8(JournalCommandKind::Amend));
    put_payload_u8(out, 0);
    put_fixed(out, &request.client_order_id);
    put_fixed(out, &request.orig_client_order_id);
    put_fixed(out, &request.venue_order_id);
    put_fixed(out, &request.account_id);
    put_fixed(out, &request.route_id);
    put_fixed(out, &request.symbol.venue);
    put_fixed(out, &request.symbol.instrument);
    put_payload_i64(out, request.quantity.0);
    put_payload_i64(out, request.limit_price.0);
    put_payload_u64(out, request.ts_recv_ns);
}

pub(crate) fn decode_command_payload(payload: &[u8]) -> ExecutionResult<JournalRecord> {
    Ok(decode_wal_command(payload)?.legacy_record())
}

pub(crate) fn decode_wal_command(payload: &[u8]) -> ExecutionResult<DecodedWalCommand> {
    let mut reader = PayloadReader::new(payload);
    let version = reader.read_u16()?;
    let kind = command_kind_from_u8(reader.read_u8()?)
        .ok_or_else(|| ExecutionError::Journal("invalid WAL command kind".to_string()))?;
    let _reserved = reader.read_u8()?;
    let command = match version {
        WAL_PAYLOAD_VERSION => DecodedWalCommand::Legacy {
            kind,
            client_order_id: reader.read_fixed::<40>()?,
            ts_ns: reader.read_u64()?,
        },
        WAL_COMMAND_PAYLOAD_VERSION => match kind {
            JournalCommandKind::Submit => DecodedWalCommand::Submit(OrderRequest {
                client_order_id: reader.read_fixed::<40>()?,
                account_id: reader.read_fixed::<32>()?,
                route_id: reader.read_fixed::<32>()?,
                strategy_id: reader.read_fixed::<32>()?,
                symbol: ExecutionSymbol {
                    venue: reader.read_fixed::<16>()?,
                    instrument: reader.read_fixed::<32>()?,
                },
                side: order_side_from_u8(reader.read_u8()?)?,
                order_type: order_type_from_u8(reader.read_u8()?)?,
                time_in_force: time_in_force_from_u8(reader.read_u8()?)?,
                quantity: {
                    let _reserved = reader.read_u8()?;
                    OrderQty(reader.read_i64()?)
                },
                limit_price: OrderPrice(reader.read_i64()?),
                stop_price: OrderPrice(reader.read_i64()?),
                ts_exchange_ns: reader.read_u64()?,
                ts_recv_ns: reader.read_u64()?,
            }),
            JournalCommandKind::Cancel => DecodedWalCommand::Cancel(CancelRequest {
                client_order_id: reader.read_fixed::<40>()?,
                orig_client_order_id: reader.read_fixed::<40>()?,
                venue_order_id: reader.read_fixed::<48>()?,
                account_id: reader.read_fixed::<32>()?,
                route_id: reader.read_fixed::<32>()?,
                symbol: ExecutionSymbol {
                    venue: reader.read_fixed::<16>()?,
                    instrument: reader.read_fixed::<32>()?,
                },
                ts_recv_ns: reader.read_u64()?,
            }),
            JournalCommandKind::Amend => DecodedWalCommand::Amend(AmendRequest {
                client_order_id: reader.read_fixed::<40>()?,
                orig_client_order_id: reader.read_fixed::<40>()?,
                venue_order_id: reader.read_fixed::<48>()?,
                account_id: reader.read_fixed::<32>()?,
                route_id: reader.read_fixed::<32>()?,
                symbol: ExecutionSymbol {
                    venue: reader.read_fixed::<16>()?,
                    instrument: reader.read_fixed::<32>()?,
                },
                quantity: OrderQty(reader.read_i64()?),
                limit_price: OrderPrice(reader.read_i64()?),
                ts_recv_ns: reader.read_u64()?,
            }),
        },
        _ => {
            return Err(ExecutionError::Journal(format!(
                "unsupported WAL command payload version {version}"
            )))
        }
    };
    reader.finish()?;
    Ok(command)
}

pub(crate) fn encode_event_payload(event: &ExecutionEvent, out: &mut Vec<u8>) {
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

pub(crate) fn decode_event_payload(payload: &[u8]) -> ExecutionResult<ExecutionEvent> {
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

pub(crate) fn decode_wal_payload(
    record: &WalRecordView<'_>,
) -> ExecutionResult<Option<JournalRecord>> {
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

pub(crate) fn decode_recovery_wal_payload(
    record: &WalRecordView<'_>,
) -> ExecutionResult<Option<DecodedRecoveryRecord>> {
    match record.header.kind {
        WalRecordKind::CommandSubmit
        | WalRecordKind::CommandCancel
        | WalRecordKind::CommandAmend => Ok(Some(DecodedRecoveryRecord::Command(Box::new(
            decode_wal_command(record.payload)?,
        )))),
        WalRecordKind::ExecutionEvent
        | WalRecordKind::RiskReject
        | WalRecordKind::RecoveryEvent => Ok(Some(DecodedRecoveryRecord::Event(Box::new(
            decode_event_payload(record.payload)?,
        )))),
        WalRecordKind::CheckpointMarker | WalRecordKind::SegmentSeal | WalRecordKind::Heartbeat => {
            Ok(None)
        }
        _ => Ok(None),
    }
}

pub(crate) fn command_wal_kind(kind: JournalCommandKind) -> WalRecordKind {
    match kind {
        JournalCommandKind::Submit => WalRecordKind::CommandSubmit,
        JournalCommandKind::Cancel => WalRecordKind::CommandCancel,
        JournalCommandKind::Amend => WalRecordKind::CommandAmend,
    }
}

pub(crate) fn event_wal_kind(event: &ExecutionEvent) -> WalRecordKind {
    match event.exec_type {
        ExecutionType::Reject => WalRecordKind::RiskReject,
        ExecutionType::Restated => WalRecordKind::RecoveryEvent,
        _ => WalRecordKind::ExecutionEvent,
    }
}

pub(crate) fn is_risk_boundary_wal_kind(kind: WalRecordKind) -> bool {
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

pub(crate) fn put_fixed<const N: usize>(out: &mut Vec<u8>, value: &FixedAscii<N>) {
    put_payload_u8(out, value.as_str().len() as u8);
    let start = out.len();
    out.resize(start + N, 0);
    out[start..start + value.as_str().len()].copy_from_slice(value.as_str().as_bytes());
}

pub(crate) fn put_payload_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

pub(crate) fn put_payload_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_payload_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_payload_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PayloadReader<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) offset: usize,
}

impl<'a> PayloadReader<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub(crate) fn read_version(&mut self) -> ExecutionResult<()> {
        let version = self.read_u16()?;
        if version == WAL_PAYLOAD_VERSION {
            Ok(())
        } else {
            Err(ExecutionError::Journal(format!(
                "unsupported WAL payload version {version}"
            )))
        }
    }

    pub(crate) fn read_u8(&mut self) -> ExecutionResult<u8> {
        let bytes = self.take(1)?;
        Ok(bytes[0])
    }

    pub(crate) fn read_u16(&mut self) -> ExecutionResult<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub(crate) fn read_u32(&mut self) -> ExecutionResult<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes(
            bytes
                .try_into()
                .expect("payload reader returned four bytes"),
        ))
    }

    pub(crate) fn read_u64(&mut self) -> ExecutionResult<u64> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes(
            bytes
                .try_into()
                .expect("payload reader returned eight bytes"),
        ))
    }

    pub(crate) fn read_i64(&mut self) -> ExecutionResult<i64> {
        let bytes = self.take(8)?;
        Ok(i64::from_le_bytes(
            bytes
                .try_into()
                .expect("payload reader returned eight bytes"),
        ))
    }

    pub(crate) fn read_i128(&mut self) -> ExecutionResult<i128> {
        let bytes = self.take(16)?;
        Ok(i128::from_le_bytes(
            bytes
                .try_into()
                .expect("payload reader returned sixteen bytes"),
        ))
    }

    pub(crate) fn read_fixed<const N: usize>(&mut self) -> ExecutionResult<FixedAscii<N>> {
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

    pub(crate) fn finish(&self) -> ExecutionResult<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(ExecutionError::Journal(
                "trailing WAL payload bytes".to_string(),
            ))
        }
    }

    pub(crate) fn take(&mut self, len: usize) -> ExecutionResult<&'a [u8]> {
        let end = self.offset.saturating_add(len);
        if end > self.bytes.len() {
            return Err(ExecutionError::Journal("truncated WAL payload".to_string()));
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }
}

pub(crate) fn wal_error(err: of_execution_core::ExecutionWalError) -> ExecutionError {
    ExecutionError::Journal(err.to_string())
}

pub(crate) fn now_ns() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    u64::try_from(nanos).unwrap_or(u64::MAX)
}
