use super::*;

/// Configuration for [`FileMarketDataCheckpointStore`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MarketDataCheckpointConfig {
    pub(crate) root: PathBuf,
    pub(crate) retain_last: usize,
    pub(crate) sync_on_save: bool,
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
    pub(crate) config: MarketDataCheckpointConfig,
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

/// Recovery status for market-data checkpoint plus WAL restore planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataRecoveryStatus {
    /// WAL and checkpoint inputs permit deterministic recovery.
    #[default]
    CleanReplay,
    /// No checkpoint is available.
    NoCheckpoint,
    /// WAL contains sequence gaps after the selected checkpoint.
    ReplayWithGaps,
    /// WAL contains checksum, magic, version, or checksum-link failures.
    CorruptWal,
    /// WAL ends with an incomplete frame.
    TruncatedWalTail,
    /// Host must request a fresh provider snapshot before strategy submission.
    NeedsFreshSnapshot,
    /// Recovery policy cannot safely restore this stream.
    Impossible,
}

/// Host action selected by market-data recovery planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MarketDataRecoveryAction {
    /// Restore the selected checkpoint payload.
    RestoreCheckpoint,
    /// Replay WAL records after the checkpoint sequence.
    ReplayWalTail,
    /// Request a fresh provider book snapshot before live processing resumes.
    RequestFreshSnapshot,
    /// Mark the recovered stream degraded.
    MarkDegraded,
    /// Keep strategy order submission disabled.
    DisableTrading,
    /// Resume market-data processing.
    ResumeMarketData,
    /// Abort recovery for this stream.
    AbortRecovery,
}

/// Policy for deterministic market-data checkpoint/WAL recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataRecoveryPolicy {
    /// Require a checkpoint before replaying WAL records.
    pub require_checkpoint: bool,
    /// Allow recovery from a WAL that ends with a truncated tail.
    pub allow_truncated_tail: bool,
    /// Allow recovery when WAL sequence gaps are detected.
    pub allow_sequence_gaps: bool,
    /// Request a fresh provider snapshot when gaps are detected.
    pub request_snapshot_on_gap: bool,
    /// Disable trading unless recovery is fully clean.
    pub disable_trading_until_clean: bool,
}

impl MarketDataRecoveryPolicy {
    /// Creates a fail-closed recovery policy.
    pub const fn fail_closed() -> Self {
        Self {
            require_checkpoint: true,
            allow_truncated_tail: false,
            allow_sequence_gaps: false,
            request_snapshot_on_gap: true,
            disable_trading_until_clean: true,
        }
    }

    /// Creates a policy that can rebuild from WAL without a checkpoint.
    pub const fn replay_from_wal_start() -> Self {
        Self {
            require_checkpoint: false,
            allow_truncated_tail: false,
            allow_sequence_gaps: false,
            request_snapshot_on_gap: true,
            disable_trading_until_clean: true,
        }
    }

    /// Sets whether checkpoints are required.
    pub const fn with_require_checkpoint(mut self, require_checkpoint: bool) -> Self {
        self.require_checkpoint = require_checkpoint;
        self
    }

    /// Sets whether truncated WAL tails are recoverable.
    pub const fn with_allow_truncated_tail(mut self, allow_truncated_tail: bool) -> Self {
        self.allow_truncated_tail = allow_truncated_tail;
        self
    }

    /// Sets whether WAL sequence gaps are recoverable.
    pub const fn with_allow_sequence_gaps(mut self, allow_sequence_gaps: bool) -> Self {
        self.allow_sequence_gaps = allow_sequence_gaps;
        self
    }

    /// Sets whether gaps should request a fresh provider snapshot.
    pub const fn with_request_snapshot_on_gap(mut self, request_snapshot_on_gap: bool) -> Self {
        self.request_snapshot_on_gap = request_snapshot_on_gap;
        self
    }

    /// Sets whether strategy submission remains disabled unless replay is clean.
    pub const fn with_disable_trading_until_clean(
        mut self,
        disable_trading_until_clean: bool,
    ) -> Self {
        self.disable_trading_until_clean = disable_trading_until_clean;
        self
    }
}

impl Default for MarketDataRecoveryPolicy {
    fn default() -> Self {
        Self::fail_closed()
    }
}

/// Inputs for market-data recovery planning.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataRecoveryInput {
    /// Selected checkpoint manifest, when one is available.
    pub checkpoint: Option<MarketDataCheckpointManifest>,
    /// WAL integrity report from [`MarketDataWal::inspect_path`].
    pub wal_integrity: MarketDataWalIntegrityReport,
}

impl MarketDataRecoveryInput {
    /// Creates recovery input from checkpoint metadata and WAL integrity.
    pub const fn new(
        checkpoint: Option<MarketDataCheckpointManifest>,
        wal_integrity: MarketDataWalIntegrityReport,
    ) -> Self {
        Self {
            checkpoint,
            wal_integrity,
        }
    }
}

/// Deterministic recovery plan for one venue/symbol stream.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataRecoveryPlan {
    /// Overall recovery classification.
    pub status: MarketDataRecoveryStatus,
    /// Checkpoint sequence used as replay anchor.
    pub checkpoint_sequence: Option<MarketDataWalSequence>,
    /// First WAL sequence the host should replay.
    pub replay_from_sequence: Option<MarketDataWalSequence>,
    /// Last WAL sequence known from integrity inspection.
    pub replay_to_sequence: Option<MarketDataWalSequence>,
    /// True when provider snapshot reconciliation is required.
    pub requires_fresh_snapshot: bool,
    /// True when strategy order submission can resume under this plan.
    pub trading_enabled: bool,
    /// Ordered host actions.
    pub actions: Vec<MarketDataRecoveryAction>,
}

impl MarketDataRecoveryPlan {
    /// Returns true when recovery cannot safely continue.
    pub fn is_impossible(&self) -> bool {
        self.actions
            .contains(&MarketDataRecoveryAction::AbortRecovery)
    }
}

/// Builds a deterministic recovery plan from checkpoint and WAL integrity.
pub fn plan_market_data_recovery(
    policy: MarketDataRecoveryPolicy,
    input: &MarketDataRecoveryInput,
) -> MarketDataRecoveryPlan {
    let checkpoint_sequence = input
        .checkpoint
        .as_ref()
        .map(|checkpoint| checkpoint.wal_sequence);
    let replay_from_sequence = checkpoint_sequence
        .map(|sequence| MarketDataWalSequence(sequence.0.saturating_add(1)))
        .or_else(|| (!policy.require_checkpoint).then_some(MarketDataWalSequence(1)));
    let replay_to_sequence = input.wal_integrity.last_sequence;

    if checkpoint_sequence.is_none() && policy.require_checkpoint {
        return impossible_recovery_plan(
            MarketDataRecoveryStatus::NoCheckpoint,
            checkpoint_sequence,
            replay_from_sequence,
            replay_to_sequence,
        );
    }
    if input.wal_integrity.checksum_failures > 0 {
        return impossible_recovery_plan(
            MarketDataRecoveryStatus::CorruptWal,
            checkpoint_sequence,
            replay_from_sequence,
            replay_to_sequence,
        );
    }
    if input.wal_integrity.truncated_tail {
        if input.wal_integrity.checksum_failures == 0
            && input.wal_integrity.sequence_failures == 0
            && policy.allow_truncated_tail
        {
            return degraded_recovery_plan(
                policy,
                MarketDataRecoveryStatus::TruncatedWalTail,
                checkpoint_sequence,
                replay_from_sequence,
                replay_to_sequence,
                false,
            );
        }
        return impossible_recovery_plan(
            MarketDataRecoveryStatus::TruncatedWalTail,
            checkpoint_sequence,
            replay_from_sequence,
            replay_to_sequence,
        );
    }
    if input.wal_integrity.sequence_failures > 0 {
        if !policy.allow_sequence_gaps {
            return impossible_recovery_plan(
                MarketDataRecoveryStatus::ReplayWithGaps,
                checkpoint_sequence,
                replay_from_sequence,
                replay_to_sequence,
            );
        }
        return degraded_recovery_plan(
            policy,
            if policy.request_snapshot_on_gap {
                MarketDataRecoveryStatus::NeedsFreshSnapshot
            } else {
                MarketDataRecoveryStatus::ReplayWithGaps
            },
            checkpoint_sequence,
            replay_from_sequence,
            replay_to_sequence,
            policy.request_snapshot_on_gap,
        );
    }
    if !input.wal_integrity.valid {
        return impossible_recovery_plan(
            MarketDataRecoveryStatus::CorruptWal,
            checkpoint_sequence,
            replay_from_sequence,
            replay_to_sequence,
        );
    }
    clean_recovery_plan(
        checkpoint_sequence,
        replay_from_sequence,
        replay_to_sequence,
    )
}

pub(crate) fn clean_recovery_plan(
    checkpoint_sequence: Option<MarketDataWalSequence>,
    replay_from_sequence: Option<MarketDataWalSequence>,
    replay_to_sequence: Option<MarketDataWalSequence>,
) -> MarketDataRecoveryPlan {
    let mut actions = Vec::new();
    if checkpoint_sequence.is_some() {
        actions.push(MarketDataRecoveryAction::RestoreCheckpoint);
    }
    if replay_from_sequence.is_some() {
        actions.push(MarketDataRecoveryAction::ReplayWalTail);
    }
    actions.push(MarketDataRecoveryAction::ResumeMarketData);
    MarketDataRecoveryPlan {
        status: MarketDataRecoveryStatus::CleanReplay,
        checkpoint_sequence,
        replay_from_sequence,
        replay_to_sequence,
        requires_fresh_snapshot: false,
        trading_enabled: true,
        actions,
    }
}

pub(crate) fn degraded_recovery_plan(
    policy: MarketDataRecoveryPolicy,
    status: MarketDataRecoveryStatus,
    checkpoint_sequence: Option<MarketDataWalSequence>,
    replay_from_sequence: Option<MarketDataWalSequence>,
    replay_to_sequence: Option<MarketDataWalSequence>,
    requires_fresh_snapshot: bool,
) -> MarketDataRecoveryPlan {
    let mut actions = Vec::new();
    if checkpoint_sequence.is_some() {
        actions.push(MarketDataRecoveryAction::RestoreCheckpoint);
    }
    if replay_from_sequence.is_some() {
        actions.push(MarketDataRecoveryAction::ReplayWalTail);
    }
    if requires_fresh_snapshot {
        actions.push(MarketDataRecoveryAction::RequestFreshSnapshot);
    }
    actions.push(MarketDataRecoveryAction::MarkDegraded);
    if policy.disable_trading_until_clean {
        actions.push(MarketDataRecoveryAction::DisableTrading);
    }
    actions.push(MarketDataRecoveryAction::ResumeMarketData);
    MarketDataRecoveryPlan {
        status,
        checkpoint_sequence,
        replay_from_sequence,
        replay_to_sequence,
        requires_fresh_snapshot,
        trading_enabled: !policy.disable_trading_until_clean && !requires_fresh_snapshot,
        actions,
    }
}

pub(crate) fn impossible_recovery_plan(
    status: MarketDataRecoveryStatus,
    checkpoint_sequence: Option<MarketDataWalSequence>,
    replay_from_sequence: Option<MarketDataWalSequence>,
    replay_to_sequence: Option<MarketDataWalSequence>,
) -> MarketDataRecoveryPlan {
    MarketDataRecoveryPlan {
        status,
        checkpoint_sequence,
        replay_from_sequence,
        replay_to_sequence,
        requires_fresh_snapshot: false,
        trading_enabled: false,
        actions: vec![
            MarketDataRecoveryAction::DisableTrading,
            MarketDataRecoveryAction::AbortRecovery,
        ],
    }
}
