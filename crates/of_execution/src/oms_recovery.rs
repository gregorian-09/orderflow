use super::*;

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
    pub(crate) root: PathBuf,
    pub(crate) sync_on_save: bool,
    pub(crate) max_retained: usize,
    pub(crate) policy: CheckpointPolicy,
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

/// Read-only integrity summary for a checkpoint store root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct CheckpointStoreIntegrityReport {
    /// Number of checkpoint files discovered.
    pub checkpoint_files: u64,
    /// Number of checkpoint files decoded and checksum-validated.
    pub valid_checkpoints: u64,
    /// Number of checkpoint files that failed decode or checksum validation.
    pub invalid_checkpoints: u64,
    /// Total bytes across discovered checkpoint files.
    pub bytes: u64,
    /// Latest valid checkpoint id, when one exists.
    pub latest_checkpoint_id: Option<u64>,
    /// Last WAL sequence covered by the latest valid checkpoint.
    pub latest_last_applied_sequence: Option<WalSequence>,
    /// Creation timestamp for the latest valid checkpoint.
    pub latest_created_ns: Option<u64>,
    /// True when all discovered checkpoint files are valid.
    pub valid: bool,
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
    pub(crate) config: CheckpointConfig,
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

    /// Inspects a checkpoint root without creating or deleting files.
    ///
    /// This helper is intended for operator diagnostics and binding layers. It
    /// counts checkpoint files, validates each payload, and identifies the
    /// latest valid checkpoint without mutating the store.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the root cannot be listed.
    pub fn inspect_root(root: impl AsRef<Path>) -> ExecutionResult<CheckpointStoreIntegrityReport> {
        inspect_checkpoint_store_root(root.as_ref())
    }

    /// Loads the latest valid checkpoint from an existing root without
    /// creating or modifying the directory.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the root is missing, cannot be
    /// listed, or any discovered checkpoint fails decoding or checksum
    /// validation.
    pub fn load_latest_from_root(
        root: impl AsRef<Path>,
    ) -> ExecutionResult<Option<ExecutionCheckpoint>> {
        let root = root.as_ref();
        if !root.exists() {
            return Err(ExecutionError::Journal(format!(
                "checkpoint root does not exist: {}",
                root.display()
            )));
        }
        let manifests = list_checkpoint_manifests(root)?;
        manifests
            .last()
            .map(|manifest| load_checkpoint_file(&manifest.path))
            .transpose()
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
        if !self.config.root().exists() {
            return Ok(Vec::new());
        }
        list_checkpoint_manifests(self.config.root())
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
    pub(crate) replay_from: WalSequence,
    pub(crate) expected_latest_sequence: Option<WalSequence>,
    pub(crate) corruption_policy: RecoveryCorruptionPolicy,
    pub(crate) venue_policy: RecoveryVenuePolicy,
    pub(crate) submissions_disabled: bool,
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
    pub(crate) checkpoint_id: Option<u64>,
    pub(crate) route_config_hash: u64,
    pub(crate) kill_switch: bool,
    pub(crate) orders: Vec<OrderState>,
    pub(crate) positions: Vec<CheckpointPosition>,
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

    /// Returns the number of recovered non-terminal orders without allocating.
    pub fn open_order_count(&self) -> usize {
        self.orders
            .iter()
            .filter(|state| !state.status.is_terminal())
            .count()
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

impl RecoveryResult {
    /// Serializes a bounded operational recovery summary as schema-versioned
    /// JSON.
    ///
    /// The summary intentionally excludes individual order/account/symbol
    /// identifiers. Rust callers can inspect [`RecoveryResult::state`] when
    /// full reconciliation detail is required.
    pub fn json_report(&self) -> String {
        let mut out = String::with_capacity(384);
        out.push_str("{\"schema_version\":1,\"checkpoint_id\":");
        write_optional_json_u64(&mut out, self.state.checkpoint_id());
        let _ = write!(
            out,
            ",\"route_config_hash\":{},\"kill_switch\":{},\"orders\":{},\"open_orders\":{},\"positions\":{},\"commands_seen\":{},\"events_applied\":{},\"replay\":{{\"records\":{},\"bytes\":{},\"first_sequence\":",
            self.state.route_config_hash(),
            self.state.kill_switch(),
            self.state.orders().len(),
            self.state.open_order_count(),
            self.state.positions().len(),
            self.commands_seen,
            self.events_applied,
            self.replay.records,
            self.replay.bytes,
        );
        write_optional_json_u64(
            &mut out,
            self.replay.first_sequence.map(|sequence| sequence.0),
        );
        out.push_str(",\"last_sequence\":");
        write_optional_json_u64(
            &mut out,
            self.replay.last_sequence.map(|sequence| sequence.0),
        );
        let _ = write!(
            out,
            "}},\"venue_reconciliation_required\":{},\"submissions_enabled\":{}}}",
            self.venue_reconciliation_required, self.submissions_enabled
        );
        out
    }
}

pub(crate) fn write_optional_json_u64(out: &mut String, value: Option<u64>) {
    if let Some(value) = value {
        let _ = write!(out, "{value}");
    } else {
        out.push_str("null");
    }
}

/// Fail-closed reason emitted by recovery-readiness evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RecoveryReadinessBlocker {
    /// Segmented WAL integrity inspection reported corruption or sequence issues.
    WalIntegrityFailed,
    /// A checkpoint is required but no valid checkpoint was discovered.
    CheckpointMissing,
    /// Checkpoint-store integrity inspection reported at least one invalid file.
    CheckpointIntegrityFailed,
    /// Recovery did not replay through the latest inspected WAL sequence.
    RecoveryDidNotReachLatestWal,
    /// The recovery plan/result keeps strategy submissions disabled.
    RecoverySubmissionsDisabled,
    /// Venue reconciliation is required but no policy decision was supplied.
    VenueReconciliationMissing,
    /// Reconciliation policy selected at least one blocking action.
    ReconciliationPolicyBlocks,
    /// Reconciliation requires explicit operator approval.
    OperatorApprovalRequired,
    /// Reconciliation requires cancelling venue-side orders before resume.
    VenueCancelsRequired,
    /// Reconciliation requires restating local state from venue truth.
    LocalRestatesRequired,
}

impl RecoveryReadinessBlocker {
    /// Returns true when this blocker requires human/operator attention.
    pub const fn requires_operator_attention(self) -> bool {
        matches!(
            self,
            Self::CheckpointIntegrityFailed
                | Self::RecoverySubmissionsDisabled
                | Self::VenueReconciliationMissing
                | Self::ReconciliationPolicyBlocks
                | Self::OperatorApprovalRequired
        )
    }
}

/// Policy for evaluating whether recovered OMS state may resume submissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RecoveryReadinessConfig {
    pub(crate) require_valid_wal: bool,
    pub(crate) require_checkpoint: bool,
    pub(crate) require_valid_checkpoint_store: bool,
    pub(crate) require_recovery_latest_wal: bool,
    pub(crate) require_recovery_submissions_enabled: bool,
    pub(crate) require_reconciliation_when_required: bool,
    pub(crate) require_reconciliation_policy_allows_submissions: bool,
}

impl RecoveryReadinessConfig {
    /// Creates a fail-closed recovery-readiness policy.
    pub const fn strict() -> Self {
        Self {
            require_valid_wal: true,
            require_checkpoint: true,
            require_valid_checkpoint_store: true,
            require_recovery_latest_wal: true,
            require_recovery_submissions_enabled: true,
            require_reconciliation_when_required: true,
            require_reconciliation_policy_allows_submissions: true,
        }
    }

    /// Sets whether WAL integrity must be valid.
    pub const fn with_require_valid_wal(mut self, require_valid_wal: bool) -> Self {
        self.require_valid_wal = require_valid_wal;
        self
    }

    /// Sets whether at least one valid checkpoint is required.
    pub const fn with_require_checkpoint(mut self, require_checkpoint: bool) -> Self {
        self.require_checkpoint = require_checkpoint;
        self
    }

    /// Sets whether every discovered checkpoint file must validate.
    pub const fn with_require_valid_checkpoint_store(
        mut self,
        require_valid_checkpoint_store: bool,
    ) -> Self {
        self.require_valid_checkpoint_store = require_valid_checkpoint_store;
        self
    }

    /// Sets whether recovery must reach the latest inspected WAL sequence.
    pub const fn with_require_recovery_latest_wal(
        mut self,
        require_recovery_latest_wal: bool,
    ) -> Self {
        self.require_recovery_latest_wal = require_recovery_latest_wal;
        self
    }

    /// Sets whether the recovery result itself must enable submissions.
    pub const fn with_require_recovery_submissions_enabled(
        mut self,
        require_recovery_submissions_enabled: bool,
    ) -> Self {
        self.require_recovery_submissions_enabled = require_recovery_submissions_enabled;
        self
    }

    /// Sets whether required venue reconciliation must be represented.
    pub const fn with_require_reconciliation_when_required(
        mut self,
        require_reconciliation_when_required: bool,
    ) -> Self {
        self.require_reconciliation_when_required = require_reconciliation_when_required;
        self
    }

    /// Sets whether reconciliation policy must allow submissions.
    pub const fn with_require_reconciliation_policy_allows_submissions(
        mut self,
        require_reconciliation_policy_allows_submissions: bool,
    ) -> Self {
        self.require_reconciliation_policy_allows_submissions =
            require_reconciliation_policy_allows_submissions;
        self
    }

    /// Returns whether WAL integrity must be valid.
    pub const fn require_valid_wal(&self) -> bool {
        self.require_valid_wal
    }

    /// Returns whether a valid checkpoint is required.
    pub const fn require_checkpoint(&self) -> bool {
        self.require_checkpoint
    }

    /// Returns whether every discovered checkpoint file must validate.
    pub const fn require_valid_checkpoint_store(&self) -> bool {
        self.require_valid_checkpoint_store
    }

    /// Returns whether recovery must reach the latest inspected WAL sequence.
    pub const fn require_recovery_latest_wal(&self) -> bool {
        self.require_recovery_latest_wal
    }

    /// Returns whether recovery must enable submissions.
    pub const fn require_recovery_submissions_enabled(&self) -> bool {
        self.require_recovery_submissions_enabled
    }

    /// Returns whether required venue reconciliation must be represented.
    pub const fn require_reconciliation_when_required(&self) -> bool {
        self.require_reconciliation_when_required
    }

    /// Returns whether reconciliation policy must allow submissions.
    pub const fn require_reconciliation_policy_allows_submissions(&self) -> bool {
        self.require_reconciliation_policy_allows_submissions
    }
}

impl Default for RecoveryReadinessConfig {
    fn default() -> Self {
        Self::strict()
    }
}

/// Aggregate recovery-readiness decision for restart workflows.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RecoveryReadinessDecision {
    /// True when strategy submissions may resume under the selected policy.
    pub submissions_enabled: bool,
    /// True when at least one blocker requires fail-closed behavior.
    pub fail_closed: bool,
    /// True when at least one blocker requires operator attention.
    pub operator_attention_required: bool,
    /// Latest sequence decoded during WAL integrity inspection.
    pub latest_wal_sequence: Option<WalSequence>,
    /// Latest sequence covered by a valid checkpoint.
    pub latest_checkpoint_sequence: Option<WalSequence>,
    /// Latest sequence applied by recovery.
    pub latest_recovered_sequence: Option<WalSequence>,
    /// Number of reconciliation policy items evaluated.
    pub reconciliation_items: usize,
    /// Fail-closed blockers found by the evaluator.
    pub blockers: Vec<RecoveryReadinessBlocker>,
}

impl RecoveryReadinessDecision {
    /// Returns true when the decision contains no blockers.
    pub fn is_ready(&self) -> bool {
        self.blockers.is_empty() && self.submissions_enabled
    }

    /// Returns true when `blocker` is present.
    pub fn has_blocker(&self, blocker: RecoveryReadinessBlocker) -> bool {
        self.blockers.contains(&blocker)
    }
}

/// Evaluates WAL, checkpoint, recovery, and reconciliation evidence before
/// live submissions resume.
pub fn evaluate_recovery_readiness(
    recovery: &RecoveryResult,
    wal: &WalSegmentIntegrityReport,
    checkpoint_store: &CheckpointStoreIntegrityReport,
    reconciliation: Option<&ReconciliationPolicyDecision>,
    config: RecoveryReadinessConfig,
) -> RecoveryReadinessDecision {
    let mut blockers = Vec::with_capacity(8);

    if config.require_valid_wal()
        && (!wal.valid || wal.checksum_failures > 0 || wal.sequence_failures > 0)
    {
        blockers.push(RecoveryReadinessBlocker::WalIntegrityFailed);
    }

    if config.require_checkpoint() && checkpoint_store.latest_checkpoint_id.is_none() {
        blockers.push(RecoveryReadinessBlocker::CheckpointMissing);
    }

    if config.require_valid_checkpoint_store()
        && (!checkpoint_store.valid || checkpoint_store.invalid_checkpoints > 0)
    {
        blockers.push(RecoveryReadinessBlocker::CheckpointIntegrityFailed);
    }

    let latest_recovered_sequence = recovery
        .replay
        .last_sequence
        .or(checkpoint_store.latest_last_applied_sequence);
    if config.require_recovery_latest_wal()
        && wal.last_sequence.is_some()
        && latest_recovered_sequence != wal.last_sequence
    {
        blockers.push(RecoveryReadinessBlocker::RecoveryDidNotReachLatestWal);
    }

    if config.require_recovery_submissions_enabled() && !recovery.submissions_enabled {
        blockers.push(RecoveryReadinessBlocker::RecoverySubmissionsDisabled);
    }

    let reconciliation_items = reconciliation.map_or(0, |decision| decision.items.len());
    if recovery.venue_reconciliation_required
        && config.require_reconciliation_when_required()
        && reconciliation.is_none()
    {
        blockers.push(RecoveryReadinessBlocker::VenueReconciliationMissing);
    }

    if let Some(decision) = reconciliation {
        if config.require_reconciliation_policy_allows_submissions()
            && !decision.submissions_enabled
        {
            blockers.push(RecoveryReadinessBlocker::ReconciliationPolicyBlocks);
        }
        if decision.operator_approval_required {
            blockers.push(RecoveryReadinessBlocker::OperatorApprovalRequired);
        }
        if decision.venue_cancels_required {
            blockers.push(RecoveryReadinessBlocker::VenueCancelsRequired);
        }
        if decision.local_restates_required {
            blockers.push(RecoveryReadinessBlocker::LocalRestatesRequired);
        }
    }

    let operator_attention_required = blockers
        .iter()
        .copied()
        .any(RecoveryReadinessBlocker::requires_operator_attention);
    let submissions_enabled = blockers.is_empty();

    RecoveryReadinessDecision {
        submissions_enabled,
        fail_closed: !submissions_enabled,
        operator_attention_required,
        latest_wal_sequence: wal.last_sequence,
        latest_checkpoint_sequence: checkpoint_store.latest_last_applied_sequence,
        latest_recovered_sequence,
        reconciliation_items,
        blockers,
    }
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
    let replay = journal.replay_recovery_from(plan.replay_from(), &mut records)?;
    finish_decoded_recovery(plan, checkpoint, records, replay)
}

/// Recovers OMS state from an existing segmented WAL root without creating an
/// append handle or modifying files.
///
/// # Errors
///
/// Returns an execution journal error when the root is missing, WAL replay or
/// state reconstruction fails, or the optional expected latest sequence is not
/// reached.
pub fn recover_oms_state_from_segmented_wal_root(
    plan: RecoveryPlan,
    checkpoint: Option<&ExecutionCheckpoint>,
    root: impl AsRef<Path>,
) -> ExecutionResult<RecoveryResult> {
    let root = root.as_ref();
    if !root.exists() {
        return Err(ExecutionError::Journal(format!(
            "segmented WAL root does not exist: {}",
            root.display()
        )));
    }
    let paths = list_segment_ids(root)?
        .into_iter()
        .map(|segment_id| segment_path(root, segment_id))
        .collect::<Vec<_>>();
    let mut records = Vec::new();
    let replay = replay_segmented_recovery_records(
        paths.iter().map(PathBuf::as_path),
        plan.replay_from(),
        &mut records,
    )?;
    finish_decoded_recovery(plan, checkpoint, records, replay)
}

pub(crate) fn finish_decoded_recovery(
    plan: RecoveryPlan,
    checkpoint: Option<&ExecutionCheckpoint>,
    records: Vec<DecodedRecoveryRecord>,
    replay: WalReplayResult,
) -> ExecutionResult<RecoveryResult> {
    let mut result = recover_oms_state_from_decoded_records(plan, checkpoint, &records)?;
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

pub(crate) fn recover_oms_state_from_decoded_records(
    plan: RecoveryPlan,
    checkpoint: Option<&ExecutionCheckpoint>,
    records: &[DecodedRecoveryRecord],
) -> ExecutionResult<RecoveryResult> {
    let mut state = checkpoint
        .map(RecoveredOmsState::from_checkpoint)
        .unwrap_or_default();
    let mut commands_seen = 0_usize;
    let mut events_applied = 0_usize;

    for record in records {
        match record {
            DecodedRecoveryRecord::Command(command) => {
                apply_recovered_command(&mut state.orders, **command)?;
                commands_seen = commands_seen.saturating_add(1);
            }
            DecodedRecoveryRecord::Event(event) => {
                apply_recovered_event(&mut state.orders, event)?;
                events_applied = events_applied.saturating_add(1);
            }
        }
    }

    Ok(recovery_result(
        plan,
        state,
        records.len(),
        commands_seen,
        events_applied,
    ))
}

pub(crate) fn recovery_result(
    plan: RecoveryPlan,
    state: RecoveredOmsState,
    records: usize,
    commands_seen: usize,
    events_applied: usize,
) -> RecoveryResult {
    let venue_reconciliation_required =
        plan.venue_policy() == RecoveryVenuePolicy::RequireReconciliation;
    let submissions_enabled =
        !plan.submissions_disabled() && plan.venue_policy() == RecoveryVenuePolicy::HostControlled;

    RecoveryResult {
        plan,
        state,
        replay: WalReplayResult {
            records,
            bytes: 0,
            first_sequence: None,
            last_sequence: None,
        },
        commands_seen,
        events_applied,
        venue_reconciliation_required,
        submissions_enabled,
    }
}

pub(crate) fn apply_recovered_command(
    orders: &mut Vec<OrderState>,
    command: DecodedWalCommand,
) -> ExecutionResult<()> {
    match command {
        DecodedWalCommand::Legacy {
            kind,
            client_order_id,
            ..
        } => Err(ExecutionError::Journal(format!(
            "legacy {kind:?} command {client_order_id} lacks the full payload required for recovery"
        ))),
        DecodedWalCommand::Submit(request) => {
            if orders
                .iter()
                .any(|state| state.client_order_id == request.client_order_id)
            {
                return Err(ExecutionError::Journal(format!(
                    "duplicate recovered submit command {}",
                    request.client_order_id
                )));
            }
            orders.push(OrderState::pending_new(&request));
            Ok(())
        }
        DecodedWalCommand::Cancel(request) => {
            let state = recovered_order_mut(orders, request.orig_client_order_id)?;
            if state.status.is_terminal() {
                return Err(ExecutionError::Journal(format!(
                    "recovered cancel command references terminal order {}",
                    request.orig_client_order_id
                )));
            }
            state.status = OrderStatus::PendingCancel;
            state.updated_ns = request.ts_recv_ns;
            Ok(())
        }
        DecodedWalCommand::Amend(request) => {
            let state = recovered_order_mut(orders, request.orig_client_order_id)?;
            if state.status.is_terminal() {
                return Err(ExecutionError::Journal(format!(
                    "recovered amend command references terminal order {}",
                    request.orig_client_order_id
                )));
            }
            state.status = OrderStatus::PendingReplace;
            state.updated_ns = request.ts_recv_ns;
            Ok(())
        }
    }
}

pub(crate) fn recovered_order_mut(
    orders: &mut [OrderState],
    client_order_id: ClientOrderId,
) -> ExecutionResult<&mut OrderState> {
    orders
        .iter_mut()
        .find(|state| state.client_order_id == client_order_id)
        .ok_or_else(|| {
            ExecutionError::Journal(format!(
                "recovered command references unknown order {client_order_id}"
            ))
        })
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

/// Loads an optional latest checkpoint and recovers an existing segmented WAL
/// root without creating or modifying either root.
///
/// Set `require_checkpoint` for production policies that prohibit replay from
/// WAL sequence one. A supplied checkpoint root is validated strictly: any
/// corrupt checkpoint aborts recovery rather than silently selecting another
/// file.
///
/// # Errors
///
/// Returns an execution journal error when a required checkpoint is absent,
/// either root is missing or corrupt, WAL replay fails, or reconstruction does
/// not reach the latest validated WAL sequence.
pub fn recover_latest_checkpoint_from_segmented_wal_roots(
    wal_root: impl AsRef<Path>,
    checkpoint_root: Option<&Path>,
    require_checkpoint: bool,
) -> ExecutionResult<RecoveryResult> {
    let wal_root = wal_root.as_ref();
    let wal = SegmentedWalExecutionJournal::inspect_root(wal_root)?;
    if !wal.valid || wal.checksum_failures > 0 || wal.sequence_failures > 0 {
        return Err(ExecutionError::Journal(
            "segmented WAL integrity validation failed".to_string(),
        ));
    }
    let checkpoint = checkpoint_root
        .map(FileExecutionCheckpointStore::load_latest_from_root)
        .transpose()?
        .flatten();
    if require_checkpoint && checkpoint.is_none() {
        return Err(ExecutionError::Journal(
            "recovery requires a valid checkpoint".to_string(),
        ));
    }
    let plan = checkpoint
        .as_ref()
        .map(RecoveryPlan::from_checkpoint)
        .unwrap_or_default()
        .with_expected_latest_sequence(wal.last_sequence);
    recover_oms_state_from_segmented_wal_root(plan, checkpoint.as_ref(), wal_root)
}

pub(crate) fn apply_recovered_event(
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

pub(crate) fn recovery_event_key(event: &ExecutionEvent) -> ClientOrderId {
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

pub(crate) fn apply_recovered_state_transition(
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

pub(crate) fn encode_checkpoint(checkpoint: &ExecutionCheckpoint) -> Vec<u8> {
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

pub(crate) fn decode_checkpoint(bytes: &[u8]) -> ExecutionResult<ExecutionCheckpoint> {
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

pub(crate) fn load_checkpoint_file(path: &Path) -> ExecutionResult<ExecutionCheckpoint> {
    let bytes = fs::read(path).map_err(|err| ExecutionError::Journal(err.to_string()))?;
    decode_checkpoint(&bytes)
}

pub(crate) fn list_checkpoint_manifests(root: &Path) -> ExecutionResult<Vec<CheckpointManifest>> {
    let mut manifests = Vec::new();
    for entry in fs::read_dir(root).map_err(|err| ExecutionError::Journal(err.to_string()))? {
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

pub(crate) fn inspect_checkpoint_store_root(
    root: &Path,
) -> ExecutionResult<CheckpointStoreIntegrityReport> {
    if !root.exists() {
        return Err(ExecutionError::Journal(format!(
            "checkpoint root does not exist: {}",
            root.display()
        )));
    }

    let mut report = CheckpointStoreIntegrityReport {
        valid: true,
        ..CheckpointStoreIntegrityReport::default()
    };
    let mut latest: Option<CheckpointManifest> = None;

    for entry in fs::read_dir(root).map_err(|err| ExecutionError::Journal(err.to_string()))? {
        let entry = entry.map_err(|err| ExecutionError::Journal(err.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some(CHECKPOINT_EXT) {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        report.checkpoint_files = report.checkpoint_files.saturating_add(1);
        report.bytes = report.bytes.saturating_add(metadata.len());

        match load_checkpoint_file(&path) {
            Ok(checkpoint) => {
                report.valid_checkpoints = report.valid_checkpoints.saturating_add(1);
                let manifest =
                    CheckpointManifest::from_checkpoint(path, metadata.len(), &checkpoint);
                let is_newer = match latest.as_ref() {
                    Some(current) => {
                        (
                            manifest.last_applied_sequence,
                            manifest.created_ns,
                            manifest.checkpoint_id,
                        ) > (
                            current.last_applied_sequence,
                            current.created_ns,
                            current.checkpoint_id,
                        )
                    }
                    None => true,
                };
                if is_newer {
                    latest = Some(manifest);
                }
            }
            Err(_) => {
                report.invalid_checkpoints = report.invalid_checkpoints.saturating_add(1);
                report.valid = false;
            }
        }
    }

    if let Some(manifest) = latest {
        report.latest_checkpoint_id = Some(manifest.checkpoint_id);
        report.latest_last_applied_sequence = Some(manifest.last_applied_sequence);
        report.latest_created_ns = Some(manifest.created_ns);
    }

    Ok(report)
}

pub(crate) fn checkpoint_checksum(checkpoint: &ExecutionCheckpoint) -> u64 {
    let mut cloned = checkpoint.clone();
    cloned.checksum = 0;
    let bytes = encode_checkpoint(&cloned);
    execution_wal_checksum(&bytes)
}

pub(crate) fn encode_order_state(state: &OrderState, out: &mut Vec<u8>) {
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

pub(crate) fn decode_order_state(reader: &mut PayloadReader<'_>) -> ExecutionResult<OrderState> {
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

pub(crate) fn encode_checkpoint_position(position: &CheckpointPosition, out: &mut Vec<u8>) {
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

pub(crate) fn decode_checkpoint_position(
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

pub(crate) fn order_side_from_u8(value: u8) -> ExecutionResult<OrderSide> {
    match value {
        1 => Ok(OrderSide::Buy),
        2 => Ok(OrderSide::Sell),
        _ => Err(ExecutionError::Journal("invalid order side".to_string())),
    }
}

pub(crate) fn order_type_from_u8(value: u8) -> ExecutionResult<OrderType> {
    match value {
        1 => Ok(OrderType::Market),
        2 => Ok(OrderType::Limit),
        3 => Ok(OrderType::Stop),
        4 => Ok(OrderType::StopLimit),
        _ => Err(ExecutionError::Journal("invalid order type".to_string())),
    }
}

pub(crate) fn time_in_force_from_u8(value: u8) -> ExecutionResult<TimeInForce> {
    match value {
        1 => Ok(TimeInForce::Day),
        2 => Ok(TimeInForce::Gtc),
        3 => Ok(TimeInForce::Ioc),
        4 => Ok(TimeInForce::Fok),
        5 => Ok(TimeInForce::Gtd),
        _ => Err(ExecutionError::Journal("invalid time in force".to_string())),
    }
}

pub(crate) fn put_payload_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_payload_i128(out: &mut Vec<u8>, value: i128) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn sync_directory(path: &Path) -> ExecutionResult<()> {
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
