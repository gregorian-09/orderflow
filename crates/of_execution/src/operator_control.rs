//! Permission-ready, journaled OMS operator command orchestration.

use std::error::Error;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use of_execution_core::{
    AccountId, CancelRequest, ClientOrderId, ExecutionCoreError, ExecutionSymbol, ExecutionText,
    FixedAscii, OrderState, RouteId, StrategyId,
};

use crate::{
    ExecutionAdapter, ExecutionEngine, ExecutionEventBuffer, ExecutionJournal, ExecutionResult,
    RiskCheck, RouteKey,
};

/// Maximum bytes retained for an operator identity.
pub const EXECUTION_OPERATOR_ACTOR_CAPACITY: usize = 32;
const OPERATOR_AUDIT_MAGIC: u32 = 0x4F46_4F41;
const OPERATOR_AUDIT_VERSION: u16 = 1;
const OPERATOR_AUDIT_HEADER_LEN: usize = 4 + 2 + 2 + 8;

/// Authenticated human or system identity supplied by the host.
pub type ExecutionOperatorActorId = FixedAscii<EXECUTION_OPERATOR_ACTOR_CAPACITY>;

/// Stable nonzero identity for one operator command.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutionOperatorCommandId(u64);

impl ExecutionOperatorCommandId {
    /// Creates a stable command identity.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionOperatorError::InvalidCommandId`] for zero.
    pub const fn new(value: u64) -> Result<Self, ExecutionOperatorError> {
        if value == 0 {
            Err(ExecutionOperatorError::InvalidCommandId)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the numeric command identity.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Permission checked by the controller before one operator action.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExecutionOperatorPermission {
    /// Pause all new submissions.
    PauseSubmissions = 0,
    /// Resume globally paused submissions.
    ResumeSubmissions = 1,
    /// Drain or restore one route.
    ManageRouteDrain = 2,
    /// Cancel every open order.
    CancelAll = 3,
    /// Cancel orders in a narrower scope.
    CancelScope = 4,
    /// Request provider open-order recovery.
    RecoverOpenOrders = 5,
    /// Run reconciliation.
    Reconcile = 6,
    /// Export an incident audit bundle.
    ExportAuditBundle = 7,
    /// Inspect locally stuck orders.
    InspectStuckOrders = 8,
    /// Rotate the execution WAL.
    RotateWalSegment = 9,
    /// Force an execution checkpoint.
    ForceCheckpoint = 10,
    /// Mark or restore route health.
    ManageRouteHealth = 11,
    /// Clear a kill switch with explicit evidence.
    ClearKillSwitch = 12,
}

impl ExecutionOperatorPermission {
    const fn bit(self) -> u64 {
        1_u64 << self as u8
    }
}

/// Fixed permission set authenticated and supplied by the host application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ExecutionOperatorPermissions(u64);

impl ExecutionOperatorPermissions {
    /// Creates an empty permission set.
    pub const fn none() -> Self {
        Self(0)
    }

    /// Creates a permission set containing every built-in command permission.
    pub const fn all() -> Self {
        Self((1_u64 << 13) - 1)
    }

    /// Adds one permission.
    pub const fn with(mut self, permission: ExecutionOperatorPermission) -> Self {
        self.0 |= permission.bit();
        self
    }

    /// Returns true when one permission is present.
    pub const fn contains(self, permission: ExecutionOperatorPermission) -> bool {
        self.0 & permission.bit() != 0
    }

    /// Returns the stable raw bit mask.
    pub const fn bits(self) -> u64 {
        self.0
    }
}

/// Host-authenticated operator and its current authorization set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecutionOperatorAuthorization {
    actor_id: ExecutionOperatorActorId,
    permissions: ExecutionOperatorPermissions,
}

impl ExecutionOperatorAuthorization {
    /// Creates host-supplied authorization evidence.
    pub const fn new(
        actor_id: ExecutionOperatorActorId,
        permissions: ExecutionOperatorPermissions,
    ) -> Self {
        Self {
            actor_id,
            permissions,
        }
    }

    /// Creates authorization from an ASCII actor id.
    ///
    /// # Errors
    ///
    /// Returns an identifier error for invalid actor text.
    pub fn from_actor(
        actor_id: &str,
        permissions: ExecutionOperatorPermissions,
    ) -> Result<Self, ExecutionCoreError> {
        Ok(Self::new(
            ExecutionOperatorActorId::new(actor_id)?,
            permissions,
        ))
    }

    /// Returns the authenticated actor.
    pub const fn actor_id(self) -> ExecutionOperatorActorId {
        self.actor_id
    }

    /// Returns the host-authorized permission set.
    pub const fn permissions(self) -> ExecutionOperatorPermissions {
        self.permissions
    }
}

/// Order selection used by scoped cancellation and stuck-order inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExecutionOperatorOrderScope {
    /// Every locally open order.
    Global,
    /// Orders on one complete route/account/symbol key.
    Route(RouteKey),
    /// Orders for one account.
    Account(AccountId),
    /// Orders attributed to one strategy.
    Strategy(StrategyId),
    /// Orders for one venue-native symbol.
    Symbol(ExecutionSymbol),
    /// One client order lifecycle.
    Order(ClientOrderId),
}

/// One typed operator action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExecutionOperatorAction {
    /// Pause all new submissions while preserving cancel/report flow.
    PauseSubmissions,
    /// Resume the global operator pause; route controls remain intact.
    ResumeSubmissions,
    /// Stop new submissions on one route while existing orders remain managed.
    DrainRoute(RouteKey),
    /// Remove the draining state from one route.
    RestoreRoute(RouteKey),
    /// Cancel every locally open order.
    CancelAll,
    /// Cancel locally open orders matching one scope.
    CancelScope(ExecutionOperatorOrderScope),
    /// Ask the adapter to restate open orders.
    RecoverOpenOrders,
    /// Run the host's authoritative multi-source reconciliation workflow.
    Reconcile,
    /// Export the host's configured incident audit bundle.
    ExportAuditBundle,
    /// Select orders with no update inside the supplied duration.
    InspectStuckOrders {
        /// Selection scope.
        scope: ExecutionOperatorOrderScope,
        /// Minimum age relative to command issue time.
        stale_after_ns: u64,
    },
    /// Rotate the configured execution WAL segment.
    RotateWalSegment,
    /// Force creation and durable installation of an OMS checkpoint.
    ForceCheckpoint,
    /// Mark or clear operator degradation for one route.
    MarkRouteDegraded {
        /// Complete route key.
        route: RouteKey,
        /// True to degrade, false to restore.
        degraded: bool,
    },
    /// Clear one scoped kill switch through deployment-owned registry logic.
    ClearKillSwitch {
        /// Stable kill-switch activation identity.
        switch_id: crate::KillSwitchId,
        /// Explicitly permit forced clear when policy and host authorization allow it.
        force: bool,
    },
}

impl ExecutionOperatorAction {
    /// Returns the permission required for this action.
    pub const fn required_permission(self) -> ExecutionOperatorPermission {
        match self {
            Self::PauseSubmissions => ExecutionOperatorPermission::PauseSubmissions,
            Self::ResumeSubmissions => ExecutionOperatorPermission::ResumeSubmissions,
            Self::DrainRoute(_) | Self::RestoreRoute(_) => {
                ExecutionOperatorPermission::ManageRouteDrain
            }
            Self::CancelAll => ExecutionOperatorPermission::CancelAll,
            Self::CancelScope(_) => ExecutionOperatorPermission::CancelScope,
            Self::RecoverOpenOrders => ExecutionOperatorPermission::RecoverOpenOrders,
            Self::Reconcile => ExecutionOperatorPermission::Reconcile,
            Self::ExportAuditBundle => ExecutionOperatorPermission::ExportAuditBundle,
            Self::InspectStuckOrders { .. } => ExecutionOperatorPermission::InspectStuckOrders,
            Self::RotateWalSegment => ExecutionOperatorPermission::RotateWalSegment,
            Self::ForceCheckpoint => ExecutionOperatorPermission::ForceCheckpoint,
            Self::MarkRouteDegraded { .. } => ExecutionOperatorPermission::ManageRouteHealth,
            Self::ClearKillSwitch { .. } => ExecutionOperatorPermission::ClearKillSwitch,
        }
    }

    /// Returns a stable action discriminant for audit exporters.
    pub const fn code(self) -> u8 {
        match self {
            Self::PauseSubmissions => 0,
            Self::ResumeSubmissions => 1,
            Self::DrainRoute(_) => 2,
            Self::RestoreRoute(_) => 3,
            Self::CancelAll => 4,
            Self::CancelScope(_) => 5,
            Self::RecoverOpenOrders => 6,
            Self::Reconcile => 7,
            Self::ExportAuditBundle => 8,
            Self::InspectStuckOrders { .. } => 9,
            Self::RotateWalSegment => 10,
            Self::ForceCheckpoint => 11,
            Self::MarkRouteDegraded { .. } => 12,
            Self::ClearKillSwitch { .. } => 13,
        }
    }
}

/// Validated operator command envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecutionOperatorCommand {
    id: ExecutionOperatorCommandId,
    action: ExecutionOperatorAction,
    issued_ns: u64,
    reason: ExecutionText,
}

impl ExecutionOperatorCommand {
    /// Creates a command with caller-owned id, timestamp, and reason.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionOperatorError::MissingTimestamp`] when `issued_ns`
    /// is zero or [`ExecutionOperatorError::MissingReason`] for empty text.
    pub const fn new(
        id: ExecutionOperatorCommandId,
        action: ExecutionOperatorAction,
        issued_ns: u64,
        reason: ExecutionText,
    ) -> Result<Self, ExecutionOperatorError> {
        if issued_ns == 0 {
            return Err(ExecutionOperatorError::MissingTimestamp);
        }
        if reason.is_empty() {
            return Err(ExecutionOperatorError::MissingReason);
        }
        Ok(Self {
            id,
            action,
            issued_ns,
            reason,
        })
    }

    /// Creates a command from an ASCII reason.
    ///
    /// # Errors
    ///
    /// Returns command validation or identifier errors.
    pub fn from_reason(
        id: ExecutionOperatorCommandId,
        action: ExecutionOperatorAction,
        issued_ns: u64,
        reason: &str,
    ) -> Result<Self, ExecutionOperatorError> {
        Self::new(id, action, issued_ns, ExecutionText::new(reason)?)
    }

    /// Returns the command id.
    pub const fn id(self) -> ExecutionOperatorCommandId {
        self.id
    }

    /// Returns the selected action.
    pub const fn action(self) -> ExecutionOperatorAction {
        self.action
    }

    /// Returns the caller-supplied issue timestamp.
    pub const fn issued_ns(self) -> u64 {
        self.issued_ns
    }

    /// Returns the required operator reason.
    pub const fn reason(self) -> ExecutionText {
        self.reason
    }
}

/// Stable command completion classification.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExecutionOperatorStatus {
    /// Intent was durably accepted before action dispatch.
    Requested = 0,
    /// Host authorization denied the action; no mutation occurred.
    Denied = 1,
    /// Action completed without a reported failure.
    Succeeded = 2,
    /// Action ran but reported one or more failures.
    Failed = 3,
}

/// Stable failure code suitable for audit records and bindings.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ExecutionOperatorFailureCode {
    /// No failure.
    #[default]
    None = 0,
    /// Host authorization denied the command.
    PermissionDenied = 1,
    /// Target route was not configured.
    RouteNotFound = 2,
    /// Adapter, engine, or bounded event output failed.
    Execution = 3,
    /// Reconciliation did not establish safe convergence.
    Reconciliation = 4,
    /// Audit bundle export failed.
    AuditExport = 5,
    /// WAL rotation failed.
    WalRotation = 6,
    /// Checkpoint creation or installation failed.
    Checkpoint = 7,
    /// Kill-switch clear was rejected.
    KillSwitch = 8,
    /// Deployment does not configure the selected service.
    Unsupported = 9,
}

/// Fixed command outcome counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ExecutionOperatorOutcome {
    /// Objects selected before dispatch.
    pub selected: u64,
    /// Actions attempted.
    pub attempted: u64,
    /// Actions that succeeded.
    pub succeeded: u64,
    /// Actions that failed.
    pub failed: u64,
    /// Canonical execution events emitted into the caller buffer.
    pub events: u64,
    /// Deployment-specific numeric result, such as checkpoint or segment id.
    pub value: u64,
    /// Stable failure classification.
    pub failure: ExecutionOperatorFailureCode,
}

impl ExecutionOperatorOutcome {
    /// Creates a successful single-action outcome.
    pub const fn success(value: u64) -> Self {
        Self {
            selected: 1,
            attempted: 1,
            succeeded: 1,
            failed: 0,
            events: 0,
            value,
            failure: ExecutionOperatorFailureCode::None,
        }
    }

    /// Creates a failed single-action outcome.
    pub const fn failure(code: ExecutionOperatorFailureCode) -> Self {
        Self {
            selected: 1,
            attempted: 1,
            succeeded: 0,
            failed: 1,
            events: 0,
            value: 0,
            failure: code,
        }
    }

    /// Returns true when no sub-action failed.
    pub const fn is_success(self) -> bool {
        self.failed == 0 && matches!(self.failure, ExecutionOperatorFailureCode::None)
    }
}

/// Immutable idempotent result retained by the controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionOperatorReceipt {
    /// Original command.
    pub command: ExecutionOperatorCommand,
    /// Authenticated actor.
    pub actor_id: ExecutionOperatorActorId,
    /// Permission set supplied by the host for the original decision.
    pub authorization_permissions: ExecutionOperatorPermissions,
    /// Final command status.
    pub status: ExecutionOperatorStatus,
    /// Completion timestamp supplied to [`ExecutionOperatorController::execute`].
    pub completed_ns: u64,
    /// Typed action outcome.
    pub outcome: ExecutionOperatorOutcome,
}

/// One command audit phase written before or after dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionOperatorAuditRecord {
    /// Monotonic audit sequence assigned by the controller.
    pub sequence: u64,
    /// Command envelope.
    pub command: ExecutionOperatorCommand,
    /// Authenticated actor.
    pub actor_id: ExecutionOperatorActorId,
    /// Permission set supplied by the host for this decision.
    pub authorization_permissions: ExecutionOperatorPermissions,
    /// Requested, denied, succeeded, or failed phase.
    pub status: ExecutionOperatorStatus,
    /// Record timestamp supplied by the host.
    pub recorded_ns: u64,
    /// Outcome counters; zero for the requested phase.
    pub outcome: ExecutionOperatorOutcome,
}

/// Replaceable operator audit sink.
pub trait ExecutionOperatorAuditSink {
    /// Reserves capacity for a complete intent/outcome pair.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionOperatorError::AuditCapacityExceeded`] when the
    /// complete pair cannot be retained.
    fn reserve(&self, additional: usize) -> Result<(), ExecutionOperatorError>;

    /// Appends one ordered audit record.
    ///
    /// # Errors
    ///
    /// Returns an audit error when the record cannot be retained or persisted.
    fn append(
        &mut self,
        record: ExecutionOperatorAuditRecord,
    ) -> Result<(), ExecutionOperatorError>;
}

/// Bounded preallocated audit sink for embedded hosts and tests.
#[derive(Debug, Clone)]
pub struct InMemoryExecutionOperatorAudit {
    records: Vec<ExecutionOperatorAuditRecord>,
    max_records: usize,
}

impl InMemoryExecutionOperatorAudit {
    /// Creates an empty bounded sink and reserves its complete storage.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionOperatorError::InvalidCapacity`] for zero.
    pub fn with_capacity(max_records: usize) -> Result<Self, ExecutionOperatorError> {
        if max_records == 0 {
            return Err(ExecutionOperatorError::InvalidCapacity);
        }
        Ok(Self {
            records: Vec::with_capacity(max_records),
            max_records,
        })
    }

    /// Returns retained records in audit sequence order.
    pub fn records(&self) -> &[ExecutionOperatorAuditRecord] {
        &self.records
    }

    /// Returns remaining record capacity.
    pub fn remaining_capacity(&self) -> usize {
        self.max_records.saturating_sub(self.records.len())
    }
}

impl ExecutionOperatorAuditSink for InMemoryExecutionOperatorAudit {
    fn reserve(&self, additional: usize) -> Result<(), ExecutionOperatorError> {
        if additional > self.remaining_capacity() {
            Err(ExecutionOperatorError::AuditCapacityExceeded)
        } else {
            Ok(())
        }
    }

    fn append(
        &mut self,
        record: ExecutionOperatorAuditRecord,
    ) -> Result<(), ExecutionOperatorError> {
        self.reserve(1)?;
        self.records.push(record);
        Ok(())
    }
}

/// Append-only checksummed operator audit journal.
///
/// The file stores complete command intent and outcome records as versioned
/// binary frames. Opening an existing file validates every checksum and the
/// contiguous audit sequence before append is allowed.
#[derive(Debug)]
pub struct FileExecutionOperatorAudit {
    path: PathBuf,
    file: File,
    sync_on_append: bool,
    records: u64,
    last_sequence: u64,
}

impl FileExecutionOperatorAudit {
    /// Opens or creates a single-writer operator audit journal.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionOperatorError::AuditIo`] for filesystem failures or
    /// a corruption/sequence error when existing frames do not validate.
    pub fn open(
        path: impl AsRef<Path>,
        sync_on_append: bool,
    ) -> Result<Self, ExecutionOperatorError> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .map_err(|_| ExecutionOperatorError::AuditIo)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|_| ExecutionOperatorError::AuditIo)?;
        let records = decode_operator_audit_frames(&bytes)?;
        let last_sequence = records.last().map_or(0, |record| record.sequence);
        Ok(Self {
            path,
            file,
            sync_on_append,
            records: records.len() as u64,
            last_sequence,
        })
    }

    /// Returns the backing file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the number of validated records including current-process appends.
    pub const fn record_count(&self) -> u64 {
        self.records
    }

    /// Loads and validates every record from disk.
    ///
    /// # Errors
    ///
    /// Returns an I/O, frame, checksum, codec, or sequence error.
    pub fn replay(&self) -> Result<Vec<ExecutionOperatorAuditRecord>, ExecutionOperatorError> {
        let mut file = File::open(&self.path).map_err(|_| ExecutionOperatorError::AuditIo)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|_| ExecutionOperatorError::AuditIo)?;
        decode_operator_audit_frames(&bytes)
    }
}

impl ExecutionOperatorAuditSink for FileExecutionOperatorAudit {
    fn reserve(&self, _additional: usize) -> Result<(), ExecutionOperatorError> {
        Ok(())
    }

    fn append(
        &mut self,
        record: ExecutionOperatorAuditRecord,
    ) -> Result<(), ExecutionOperatorError> {
        if record.sequence != self.last_sequence.saturating_add(1) {
            return Err(ExecutionOperatorError::AuditSequence);
        }
        let frame = encode_operator_audit_record(&record)?;
        self.file
            .write_all(&frame)
            .and_then(|()| self.file.flush())
            .map_err(|_| ExecutionOperatorError::AuditIo)?;
        if self.sync_on_append {
            self.file
                .sync_data()
                .map_err(|_| ExecutionOperatorError::AuditIo)?;
        }
        self.last_sequence = record.sequence;
        self.records = self.records.saturating_add(1);
        Ok(())
    }
}

/// Host-owned services for control-plane operations outside the core engine.
///
/// Implementations own filesystem paths, checkpoint ids, external evidence,
/// kill-switch registries, access controls, and export destinations.
pub trait ExecutionOperatorServices<A, R, J>
where
    A: ExecutionAdapter,
    R: RiskCheck,
    J: ExecutionJournal,
{
    /// Runs authoritative reconciliation and returns its machine result.
    fn reconcile(
        &mut self,
        engine: &mut ExecutionEngine<A, R, J>,
        command: &ExecutionOperatorCommand,
    ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError>;

    /// Creates an incident audit bundle at the configured destination.
    fn export_audit_bundle(
        &mut self,
        engine: &mut ExecutionEngine<A, R, J>,
        command: &ExecutionOperatorCommand,
    ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError>;

    /// Rotates the active execution WAL segment.
    fn rotate_wal_segment(
        &mut self,
        engine: &mut ExecutionEngine<A, R, J>,
        command: &ExecutionOperatorCommand,
    ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError>;

    /// Creates and durably installs an execution checkpoint.
    fn force_checkpoint(
        &mut self,
        engine: &mut ExecutionEngine<A, R, J>,
        command: &ExecutionOperatorCommand,
    ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError>;

    /// Clears one kill-switch activation under host policy.
    fn clear_kill_switch(
        &mut self,
        engine: &mut ExecutionEngine<A, R, J>,
        switch_id: crate::KillSwitchId,
        force: bool,
        command: &ExecutionOperatorCommand,
    ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError>;
}

/// Explicit service set for deployments using only engine-native commands.
///
/// Pause/resume, route control, scoped cancel, recovery, and stuck inspection
/// remain available. Reconciliation, export, WAL rotation, checkpoint, and
/// kill-switch clear return [`ExecutionOperatorFailureCode::Unsupported`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoExternalExecutionOperatorServices;

impl<A, R, J> ExecutionOperatorServices<A, R, J> for NoExternalExecutionOperatorServices
where
    A: ExecutionAdapter,
    R: RiskCheck,
    J: ExecutionJournal,
{
    fn reconcile(
        &mut self,
        _engine: &mut ExecutionEngine<A, R, J>,
        _command: &ExecutionOperatorCommand,
    ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError> {
        Err(ExecutionOperatorServiceError::new(
            ExecutionOperatorFailureCode::Unsupported,
        ))
    }

    fn export_audit_bundle(
        &mut self,
        _engine: &mut ExecutionEngine<A, R, J>,
        _command: &ExecutionOperatorCommand,
    ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError> {
        Err(ExecutionOperatorServiceError::new(
            ExecutionOperatorFailureCode::Unsupported,
        ))
    }

    fn rotate_wal_segment(
        &mut self,
        _engine: &mut ExecutionEngine<A, R, J>,
        _command: &ExecutionOperatorCommand,
    ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError> {
        Err(ExecutionOperatorServiceError::new(
            ExecutionOperatorFailureCode::Unsupported,
        ))
    }

    fn force_checkpoint(
        &mut self,
        _engine: &mut ExecutionEngine<A, R, J>,
        _command: &ExecutionOperatorCommand,
    ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError> {
        Err(ExecutionOperatorServiceError::new(
            ExecutionOperatorFailureCode::Unsupported,
        ))
    }

    fn clear_kill_switch(
        &mut self,
        _engine: &mut ExecutionEngine<A, R, J>,
        _switch_id: crate::KillSwitchId,
        _force: bool,
        _command: &ExecutionOperatorCommand,
    ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError> {
        Err(ExecutionOperatorServiceError::new(
            ExecutionOperatorFailureCode::Unsupported,
        ))
    }
}

/// Deployment-service failure returned to the operator controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionOperatorServiceError {
    /// Stable failure class.
    pub code: ExecutionOperatorFailureCode,
}

impl ExecutionOperatorServiceError {
    /// Creates a typed service failure.
    pub const fn new(code: ExecutionOperatorFailureCode) -> Self {
        Self { code }
    }
}

impl fmt::Display for ExecutionOperatorServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "operator service failed ({:?})", self.code)
    }
}

impl Error for ExecutionOperatorServiceError {}

/// Error returned before a command can produce a journaled receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExecutionOperatorError {
    /// Command ids must be nonzero.
    InvalidCommandId,
    /// Required timestamp is zero.
    MissingTimestamp,
    /// A reason is required for every operator command.
    MissingReason,
    /// Controller or sink capacity must be nonzero.
    InvalidCapacity,
    /// A new command id regressed behind the accepted sequence.
    CommandIdRegression,
    /// A retained command id was reused with different semantics or actor.
    CommandIdCollision,
    /// Idempotency receipt capacity is exhausted.
    ReceiptCapacityExceeded,
    /// Audit capacity cannot hold the complete intent/outcome pair.
    AuditCapacityExceeded,
    /// Intent could not be journaled; no action ran.
    AuditIntentFailed,
    /// Outcome journaling failed after dispatch; submissions were paused.
    AuditOutcomeFailed,
    /// A prior post-mutation outcome record must be repaired before new commands.
    AuditRepairRequired,
    /// File-backed audit I/O failed.
    AuditIo,
    /// File-backed audit data is truncated, corrupt, or unsupported.
    AuditCorrupt,
    /// Audit record sequences are not contiguous.
    AuditSequence,
    /// Completion time precedes command issue time.
    CompletionTimeRegression,
    /// Fixed identifier text was invalid.
    Core(ExecutionCoreError),
}

impl fmt::Display for ExecutionOperatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommandId => f.write_str("operator command id must be nonzero"),
            Self::MissingTimestamp => f.write_str("operator command timestamp is missing"),
            Self::MissingReason => f.write_str("operator command reason is required"),
            Self::InvalidCapacity => f.write_str("operator capacity must be nonzero"),
            Self::CommandIdRegression => f.write_str("operator command id regressed"),
            Self::CommandIdCollision => f.write_str("operator command id collision"),
            Self::ReceiptCapacityExceeded => f.write_str("operator receipt capacity exceeded"),
            Self::AuditCapacityExceeded => f.write_str("operator audit capacity exceeded"),
            Self::AuditIntentFailed => f.write_str("operator intent audit failed"),
            Self::AuditOutcomeFailed => {
                f.write_str("operator outcome audit failed after command dispatch")
            }
            Self::AuditRepairRequired => f.write_str("operator audit outcome repair is required"),
            Self::AuditIo => f.write_str("operator audit I/O failed"),
            Self::AuditCorrupt => f.write_str("operator audit data is corrupt"),
            Self::AuditSequence => f.write_str("operator audit sequence is not contiguous"),
            Self::CompletionTimeRegression => {
                f.write_str("operator completion timestamp precedes issue timestamp")
            }
            Self::Core(error) => write!(f, "operator identifier error: {error}"),
        }
    }
}

impl Error for ExecutionOperatorError {}

impl From<ExecutionCoreError> for ExecutionOperatorError {
    fn from(value: ExecutionCoreError) -> Self {
        Self::Core(value)
    }
}

/// Caller-owned bounded result for stuck-order inspection.
#[derive(Debug, Clone)]
pub struct ExecutionStuckOrderBuffer {
    orders: Vec<OrderState>,
    max_len: usize,
}

impl ExecutionStuckOrderBuffer {
    /// Creates an empty preallocated output buffer.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionOperatorError::InvalidCapacity`] for zero.
    pub fn with_capacity(max_len: usize) -> Result<Self, ExecutionOperatorError> {
        if max_len == 0 {
            return Err(ExecutionOperatorError::InvalidCapacity);
        }
        Ok(Self {
            orders: Vec::with_capacity(max_len),
            max_len,
        })
    }

    /// Returns selected orders in deterministic route/account/symbol/id order.
    pub fn as_slice(&self) -> &[OrderState] {
        &self.orders
    }

    /// Clears retained output without releasing storage.
    pub fn clear(&mut self) {
        self.orders.clear();
    }
}

/// Bounded idempotent operator command coordinator.
#[derive(Debug, Clone)]
pub struct ExecutionOperatorController {
    receipts: Vec<ExecutionOperatorReceipt>,
    max_receipts: usize,
    last_command_id: Option<ExecutionOperatorCommandId>,
    next_audit_sequence: u64,
    pending_audit_outcome: Option<ExecutionOperatorAuditRecord>,
}

impl ExecutionOperatorController {
    /// Creates a controller with preallocated, fail-closed receipt retention.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionOperatorError::InvalidCapacity`] for zero.
    pub fn with_capacity(max_receipts: usize) -> Result<Self, ExecutionOperatorError> {
        if max_receipts == 0 {
            return Err(ExecutionOperatorError::InvalidCapacity);
        }
        Ok(Self {
            receipts: Vec::with_capacity(max_receipts),
            max_receipts,
            last_command_id: None,
            next_audit_sequence: 1,
            pending_audit_outcome: None,
        })
    }

    /// Restores idempotency state from validated intent/outcome audit pairs.
    ///
    /// Every command must have exactly one adjacent `Requested` record and one
    /// terminal record with matching command and actor. An unpaired intent is
    /// ambiguous after restart and fails closed for operator investigation.
    ///
    /// # Errors
    ///
    /// Returns a capacity, corruption, sequence, or command-id ordering error.
    pub fn restore(
        records: &[ExecutionOperatorAuditRecord],
        max_receipts: usize,
    ) -> Result<Self, ExecutionOperatorError> {
        if max_receipts == 0 {
            return Err(ExecutionOperatorError::InvalidCapacity);
        }
        if records.len() & 1 != 0 || records.len() / 2 > max_receipts {
            return Err(if records.len() / 2 > max_receipts {
                ExecutionOperatorError::ReceiptCapacityExceeded
            } else {
                ExecutionOperatorError::AuditCorrupt
            });
        }
        let mut controller = Self::with_capacity(max_receipts)?;
        let mut expected_sequence = 1_u64;
        for pair in records.chunks_exact(2) {
            let requested = pair[0];
            let completed = pair[1];
            if requested.sequence != expected_sequence
                || completed.sequence != expected_sequence.saturating_add(1)
            {
                return Err(ExecutionOperatorError::AuditSequence);
            }
            expected_sequence = expected_sequence.saturating_add(2);
            if requested.status != ExecutionOperatorStatus::Requested
                || matches!(completed.status, ExecutionOperatorStatus::Requested)
                || requested.command != completed.command
                || requested.actor_id != completed.actor_id
                || requested.authorization_permissions != completed.authorization_permissions
                || requested.outcome != ExecutionOperatorOutcome::default()
                || requested.recorded_ns < requested.command.issued_ns()
                || completed.recorded_ns < completed.command.issued_ns()
                || !terminal_status_matches_outcome(completed.status, completed.outcome)
            {
                return Err(ExecutionOperatorError::AuditCorrupt);
            }
            if controller
                .last_command_id
                .is_some_and(|last| completed.command.id() <= last)
            {
                return Err(ExecutionOperatorError::CommandIdRegression);
            }
            controller.remember(ExecutionOperatorReceipt {
                command: completed.command,
                actor_id: completed.actor_id,
                authorization_permissions: completed.authorization_permissions,
                status: completed.status,
                completed_ns: completed.recorded_ns,
                outcome: completed.outcome,
            });
        }
        controller.next_audit_sequence = expected_sequence;
        Ok(controller)
    }

    /// Returns retained command receipts in accepted-id order.
    pub fn receipts(&self) -> &[ExecutionOperatorReceipt] {
        &self.receipts
    }

    /// Restores local pause, drain, and route-degradation controls into an engine.
    ///
    /// Call this after [`Self::restore`] and before starting strategy flow.
    /// External actions such as cancel, checkpoint, export, and kill-switch
    /// clear are never replayed.
    pub fn restore_engine_controls<A, R, J>(&self, engine: &mut ExecutionEngine<A, R, J>)
    where
        A: ExecutionAdapter,
        R: RiskCheck,
        J: ExecutionJournal,
    {
        engine.operator_submissions_paused = false;
        engine.operator_draining_routes.clear();
        engine.operator_degraded_routes.clear();
        for receipt in &self.receipts {
            if receipt.status != ExecutionOperatorStatus::Succeeded {
                continue;
            }
            match receipt.command.action() {
                ExecutionOperatorAction::PauseSubmissions => {
                    engine.operator_submissions_paused = true;
                }
                ExecutionOperatorAction::ResumeSubmissions => {
                    engine.operator_submissions_paused = false;
                }
                ExecutionOperatorAction::DrainRoute(route) => {
                    engine.operator_draining_routes.insert(route);
                }
                ExecutionOperatorAction::RestoreRoute(route) => {
                    engine.operator_draining_routes.remove(&route);
                }
                ExecutionOperatorAction::MarkRouteDegraded { route, degraded } => {
                    if degraded {
                        engine.operator_degraded_routes.insert(route);
                    } else {
                        engine.operator_degraded_routes.remove(&route);
                    }
                }
                ExecutionOperatorAction::CancelAll
                | ExecutionOperatorAction::CancelScope(_)
                | ExecutionOperatorAction::RecoverOpenOrders
                | ExecutionOperatorAction::Reconcile
                | ExecutionOperatorAction::ExportAuditBundle
                | ExecutionOperatorAction::InspectStuckOrders { .. }
                | ExecutionOperatorAction::RotateWalSegment
                | ExecutionOperatorAction::ForceCheckpoint
                | ExecutionOperatorAction::ClearKillSwitch { .. } => {}
            }
        }
    }

    /// Executes one authenticated command with intent-before-effect journaling.
    ///
    /// Exact command retries return the retained receipt without redispatch.
    /// Command-id collisions, regressions, exhausted retention, and audit
    /// failures fail closed. If outcome journaling fails after a mutation, the
    /// engine is immediately paused before the error is returned.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionOperatorError`] when validation, idempotency, or
    /// audit durability prevents a safe receipt.
    #[allow(clippy::too_many_arguments)]
    pub fn execute<A, R, J, S, O>(
        &mut self,
        engine: &mut ExecutionEngine<A, R, J>,
        services: &mut S,
        audit: &mut O,
        authorization: ExecutionOperatorAuthorization,
        command: ExecutionOperatorCommand,
        completed_ns: u64,
        events: &mut ExecutionEventBuffer,
        stuck_orders: &mut ExecutionStuckOrderBuffer,
    ) -> Result<ExecutionOperatorReceipt, ExecutionOperatorError>
    where
        A: ExecutionAdapter,
        R: RiskCheck,
        J: ExecutionJournal,
        S: ExecutionOperatorServices<A, R, J>,
        O: ExecutionOperatorAuditSink,
    {
        if completed_ns < command.issued_ns() || completed_ns == 0 {
            return Err(ExecutionOperatorError::CompletionTimeRegression);
        }
        if let Some(pending) = self.pending_audit_outcome {
            if pending.command != command || pending.actor_id != authorization.actor_id() {
                return Err(ExecutionOperatorError::AuditRepairRequired);
            }
            audit.reserve(1)?;
            audit
                .append(pending)
                .map_err(|_| ExecutionOperatorError::AuditOutcomeFailed)?;
            self.next_audit_sequence = self.next_audit_sequence.saturating_add(1);
            self.pending_audit_outcome = None;
            return self
                .receipts
                .iter()
                .find(|receipt| receipt.command.id() == command.id())
                .copied()
                .ok_or(ExecutionOperatorError::AuditRepairRequired);
        }
        if let Some(receipt) = self
            .receipts
            .iter()
            .find(|receipt| receipt.command.id() == command.id())
        {
            if receipt.command == command && receipt.actor_id == authorization.actor_id() {
                return Ok(*receipt);
            }
            return Err(ExecutionOperatorError::CommandIdCollision);
        }
        if self
            .last_command_id
            .is_some_and(|last| command.id() <= last)
        {
            return Err(ExecutionOperatorError::CommandIdRegression);
        }
        if self.receipts.len() >= self.max_receipts {
            return Err(ExecutionOperatorError::ReceiptCapacityExceeded);
        }
        audit.reserve(2)?;

        let requested = self.audit_record(
            command,
            authorization.actor_id(),
            authorization.permissions(),
            ExecutionOperatorStatus::Requested,
            command.issued_ns(),
            ExecutionOperatorOutcome::default(),
        );
        audit
            .append(requested)
            .map_err(|_| ExecutionOperatorError::AuditIntentFailed)?;
        self.next_audit_sequence = self.next_audit_sequence.saturating_add(1);

        let permitted = authorization
            .permissions()
            .contains(command.action().required_permission());
        let outcome = if permitted {
            dispatch(engine, services, command, events, stuck_orders)
        } else {
            ExecutionOperatorOutcome::failure(ExecutionOperatorFailureCode::PermissionDenied)
        };
        let status = if !permitted {
            ExecutionOperatorStatus::Denied
        } else if outcome.is_success() {
            ExecutionOperatorStatus::Succeeded
        } else {
            ExecutionOperatorStatus::Failed
        };
        let receipt = ExecutionOperatorReceipt {
            command,
            actor_id: authorization.actor_id(),
            authorization_permissions: authorization.permissions(),
            status,
            completed_ns,
            outcome,
        };
        let completed = self.audit_record(
            command,
            authorization.actor_id(),
            authorization.permissions(),
            status,
            completed_ns,
            outcome,
        );
        if audit.append(completed).is_err() {
            engine.operator_submissions_paused = true;
            self.pending_audit_outcome = Some(completed);
            self.remember(receipt);
            return Err(ExecutionOperatorError::AuditOutcomeFailed);
        }
        self.next_audit_sequence = self.next_audit_sequence.saturating_add(1);
        self.remember(receipt);
        Ok(receipt)
    }

    fn audit_record(
        &self,
        command: ExecutionOperatorCommand,
        actor_id: ExecutionOperatorActorId,
        authorization_permissions: ExecutionOperatorPermissions,
        status: ExecutionOperatorStatus,
        recorded_ns: u64,
        outcome: ExecutionOperatorOutcome,
    ) -> ExecutionOperatorAuditRecord {
        ExecutionOperatorAuditRecord {
            sequence: self.next_audit_sequence,
            command,
            actor_id,
            authorization_permissions,
            status,
            recorded_ns,
            outcome,
        }
    }

    fn remember(&mut self, receipt: ExecutionOperatorReceipt) {
        self.last_command_id = Some(receipt.command.id());
        self.receipts.push(receipt);
    }
}

fn dispatch<A, R, J, S>(
    engine: &mut ExecutionEngine<A, R, J>,
    services: &mut S,
    command: ExecutionOperatorCommand,
    events: &mut ExecutionEventBuffer,
    stuck_orders: &mut ExecutionStuckOrderBuffer,
) -> ExecutionOperatorOutcome
where
    A: ExecutionAdapter,
    R: RiskCheck,
    J: ExecutionJournal,
    S: ExecutionOperatorServices<A, R, J>,
{
    match command.action() {
        ExecutionOperatorAction::PauseSubmissions => {
            engine.operator_submissions_paused = true;
            ExecutionOperatorOutcome::success(1)
        }
        ExecutionOperatorAction::ResumeSubmissions => {
            engine.operator_submissions_paused = false;
            ExecutionOperatorOutcome::success(1)
        }
        ExecutionOperatorAction::DrainRoute(route) => set_route_control(engine, route, true, false),
        ExecutionOperatorAction::RestoreRoute(route) => {
            set_route_control(engine, route, false, false)
        }
        ExecutionOperatorAction::CancelAll => engine.operator_cancel_scope(
            ExecutionOperatorOrderScope::Global,
            command.id(),
            command.issued_ns(),
            events,
        ),
        ExecutionOperatorAction::CancelScope(scope) => {
            engine.operator_cancel_scope(scope, command.id(), command.issued_ns(), events)
        }
        ExecutionOperatorAction::RecoverOpenOrders => {
            let before = events.len();
            match engine.recover_open_orders(events) {
                Ok(count) => ExecutionOperatorOutcome {
                    selected: count as u64,
                    attempted: 1,
                    succeeded: 1,
                    failed: 0,
                    events: events.len().saturating_sub(before) as u64,
                    value: count as u64,
                    failure: ExecutionOperatorFailureCode::None,
                },
                Err(_) => {
                    ExecutionOperatorOutcome::failure(ExecutionOperatorFailureCode::Execution)
                }
            }
        }
        ExecutionOperatorAction::Reconcile => service_outcome(
            services.reconcile(engine, &command),
            ExecutionOperatorFailureCode::Reconciliation,
        ),
        ExecutionOperatorAction::ExportAuditBundle => service_outcome(
            services.export_audit_bundle(engine, &command),
            ExecutionOperatorFailureCode::AuditExport,
        ),
        ExecutionOperatorAction::InspectStuckOrders {
            scope,
            stale_after_ns,
        } => {
            engine.operator_inspect_stuck(scope, command.issued_ns(), stale_after_ns, stuck_orders)
        }
        ExecutionOperatorAction::RotateWalSegment => service_outcome(
            services.rotate_wal_segment(engine, &command),
            ExecutionOperatorFailureCode::WalRotation,
        ),
        ExecutionOperatorAction::ForceCheckpoint => service_outcome(
            services.force_checkpoint(engine, &command),
            ExecutionOperatorFailureCode::Checkpoint,
        ),
        ExecutionOperatorAction::MarkRouteDegraded { route, degraded } => {
            set_route_control(engine, route, degraded, true)
        }
        ExecutionOperatorAction::ClearKillSwitch { switch_id, force } => service_outcome(
            services.clear_kill_switch(engine, switch_id, force, &command),
            ExecutionOperatorFailureCode::KillSwitch,
        ),
    }
}

fn service_outcome(
    result: Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError>,
    fallback: ExecutionOperatorFailureCode,
) -> ExecutionOperatorOutcome {
    match result {
        Ok(outcome) => outcome,
        Err(error) => {
            ExecutionOperatorOutcome::failure(if error.code == ExecutionOperatorFailureCode::None {
                fallback
            } else {
                error.code
            })
        }
    }
}

fn terminal_status_matches_outcome(
    status: ExecutionOperatorStatus,
    outcome: ExecutionOperatorOutcome,
) -> bool {
    match status {
        ExecutionOperatorStatus::Requested => false,
        ExecutionOperatorStatus::Denied => {
            outcome.failure == ExecutionOperatorFailureCode::PermissionDenied && outcome.failed > 0
        }
        ExecutionOperatorStatus::Succeeded => outcome.is_success(),
        ExecutionOperatorStatus::Failed => !outcome.is_success(),
    }
}

fn set_route_control<A, R, J>(
    engine: &mut ExecutionEngine<A, R, J>,
    route: RouteKey,
    enabled: bool,
    degraded: bool,
) -> ExecutionOperatorOutcome
where
    A: ExecutionAdapter,
    R: RiskCheck,
    J: ExecutionJournal,
{
    if !engine.routes.iter().any(|candidate| {
        RouteKey::new(candidate.route_id, candidate.account_id, candidate.symbol) == route
    }) {
        return ExecutionOperatorOutcome::failure(ExecutionOperatorFailureCode::RouteNotFound);
    }
    let set = if degraded {
        &mut engine.operator_degraded_routes
    } else {
        &mut engine.operator_draining_routes
    };
    if enabled {
        set.insert(route);
    } else {
        set.remove(&route);
    }
    ExecutionOperatorOutcome::success(1)
}

impl<A, R, J> ExecutionEngine<A, R, J>
where
    A: ExecutionAdapter,
    R: RiskCheck,
    J: ExecutionJournal,
{
    fn operator_cancel_scope(
        &mut self,
        scope: ExecutionOperatorOrderScope,
        command_id: ExecutionOperatorCommandId,
        timestamp_ns: u64,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionOperatorOutcome {
        let mut targets: Vec<OrderState> = self
            .orders
            .values()
            .map(|state| *state.state())
            .filter(|state| !state.status.is_terminal())
            .filter(|state| self.operator_scope_matches(scope, state))
            .collect();
        targets.sort_by(|left, right| {
            left.route_id
                .as_str()
                .cmp(right.route_id.as_str())
                .then_with(|| left.account_id.as_str().cmp(right.account_id.as_str()))
                .then_with(|| left.symbol.venue.as_str().cmp(right.symbol.venue.as_str()))
                .then_with(|| {
                    left.symbol
                        .instrument
                        .as_str()
                        .cmp(right.symbol.instrument.as_str())
                })
                .then_with(|| {
                    left.client_order_id
                        .as_str()
                        .cmp(right.client_order_id.as_str())
                })
        });

        let mut outcome = ExecutionOperatorOutcome {
            selected: targets.len() as u64,
            ..ExecutionOperatorOutcome::default()
        };
        for (index, state) in targets.into_iter().enumerate() {
            outcome.attempted = outcome.attempted.saturating_add(1);
            let cancel_id = operator_cancel_id(command_id, index as u64 + 1);
            let before = out.len();
            let result =
                cancel_id
                    .map_err(crate::ExecutionError::Core)
                    .and_then(|client_order_id| {
                        self.operator_cancel(
                            CancelRequest {
                                client_order_id,
                                orig_client_order_id: state.last_accepted_client_order_id,
                                venue_order_id: state.venue_order_id,
                                account_id: state.account_id,
                                route_id: state.route_id,
                                symbol: state.symbol,
                                ts_recv_ns: timestamp_ns,
                            },
                            out,
                        )
                    });
            if result.is_ok() {
                outcome.succeeded = outcome.succeeded.saturating_add(1);
                outcome.events = outcome
                    .events
                    .saturating_add(out.len().saturating_sub(before) as u64);
            } else {
                outcome.failed = outcome.failed.saturating_add(1);
            }
        }
        if outcome.failed > 0 {
            outcome.failure = ExecutionOperatorFailureCode::Execution;
        }
        outcome
    }

    fn operator_cancel(
        &mut self,
        request: CancelRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        self.journal.record_command(
            crate::JournalCommandKind::Cancel,
            request.client_order_id,
            request.ts_recv_ns,
        )?;
        self.scratch.clear();
        self.adapter.cancel(&request, &mut self.scratch)?;
        self.metrics.cancelled = self.metrics.cancelled.saturating_add(1);
        self.apply_scratch(out)?;
        Ok(())
    }

    fn operator_inspect_stuck(
        &self,
        scope: ExecutionOperatorOrderScope,
        now_ns: u64,
        stale_after_ns: u64,
        out: &mut ExecutionStuckOrderBuffer,
    ) -> ExecutionOperatorOutcome {
        let cutoff = now_ns.saturating_sub(stale_after_ns);
        let mut selected: Vec<OrderState> = self
            .orders
            .values()
            .map(|state| *state.state())
            .filter(|state| !state.status.is_terminal())
            .filter(|state| self.operator_scope_matches(scope, state))
            .filter(|state| state.updated_ns <= cutoff)
            .collect();
        selected.sort_by(|left, right| {
            left.updated_ns.cmp(&right.updated_ns).then_with(|| {
                left.client_order_id
                    .as_str()
                    .cmp(right.client_order_id.as_str())
            })
        });
        out.clear();
        if selected.len() > out.max_len {
            return ExecutionOperatorOutcome::failure(ExecutionOperatorFailureCode::Execution);
        }
        out.orders.extend_from_slice(&selected);
        ExecutionOperatorOutcome {
            selected: selected.len() as u64,
            attempted: 1,
            succeeded: 1,
            failed: 0,
            events: 0,
            value: selected.len() as u64,
            failure: ExecutionOperatorFailureCode::None,
        }
    }

    fn operator_scope_matches(
        &self,
        scope: ExecutionOperatorOrderScope,
        state: &OrderState,
    ) -> bool {
        match scope {
            ExecutionOperatorOrderScope::Global => true,
            ExecutionOperatorOrderScope::Route(route) => {
                route == RouteKey::new(state.route_id, state.account_id, state.symbol)
            }
            ExecutionOperatorOrderScope::Account(account) => state.account_id == account,
            ExecutionOperatorOrderScope::Strategy(strategy) => self
                .order_strategies
                .get(&state.client_order_id)
                .or_else(|| {
                    self.order_strategies
                        .get(&state.last_accepted_client_order_id)
                })
                .is_some_and(|known| *known == strategy),
            ExecutionOperatorOrderScope::Symbol(symbol) => state.symbol == symbol,
            ExecutionOperatorOrderScope::Order(order) => {
                state.client_order_id == order || state.last_accepted_client_order_id == order
            }
        }
    }
}

fn operator_cancel_id(
    command_id: ExecutionOperatorCommandId,
    ordinal: u64,
) -> Result<ClientOrderId, ExecutionCoreError> {
    ClientOrderId::new(&format!("OPC-{:016X}-{ordinal:016X}", command_id.get()))
}

fn encode_operator_audit_record(
    record: &ExecutionOperatorAuditRecord,
) -> Result<Vec<u8>, ExecutionOperatorError> {
    let mut payload = Vec::with_capacity(256);
    put_u64(&mut payload, record.sequence);
    put_u64(&mut payload, record.command.id().get());
    encode_action(&mut payload, record.command.action());
    put_u64(&mut payload, record.command.issued_ns());
    put_text_u16(&mut payload, record.command.reason().as_str())?;
    put_text_u8(&mut payload, record.actor_id.as_str())?;
    put_u64(&mut payload, record.authorization_permissions.bits());
    payload.push(record.status as u8);
    put_u64(&mut payload, record.recorded_ns);
    put_u64(&mut payload, record.outcome.selected);
    put_u64(&mut payload, record.outcome.attempted);
    put_u64(&mut payload, record.outcome.succeeded);
    put_u64(&mut payload, record.outcome.failed);
    put_u64(&mut payload, record.outcome.events);
    put_u64(&mut payload, record.outcome.value);
    put_u16(&mut payload, record.outcome.failure as u16);
    let payload_len =
        u16::try_from(payload.len()).map_err(|_| ExecutionOperatorError::AuditCorrupt)?;

    let mut frame = Vec::with_capacity(OPERATOR_AUDIT_HEADER_LEN + payload.len());
    put_u32(&mut frame, OPERATOR_AUDIT_MAGIC);
    put_u16(&mut frame, OPERATOR_AUDIT_VERSION);
    put_u16(&mut frame, payload_len);
    put_u64(&mut frame, operator_audit_checksum(&payload));
    frame.extend_from_slice(&payload);
    Ok(frame)
}

fn decode_operator_audit_frames(
    bytes: &[u8],
) -> Result<Vec<ExecutionOperatorAuditRecord>, ExecutionOperatorError> {
    let mut offset = 0;
    let mut records = Vec::new();
    let mut expected_sequence = 1_u64;
    while offset < bytes.len() {
        if bytes.len().saturating_sub(offset) < OPERATOR_AUDIT_HEADER_LEN {
            return Err(ExecutionOperatorError::AuditCorrupt);
        }
        let magic = u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| ExecutionOperatorError::AuditCorrupt)?,
        );
        let version = u16::from_le_bytes(
            bytes[offset + 4..offset + 6]
                .try_into()
                .map_err(|_| ExecutionOperatorError::AuditCorrupt)?,
        );
        let payload_len = usize::from(u16::from_le_bytes(
            bytes[offset + 6..offset + 8]
                .try_into()
                .map_err(|_| ExecutionOperatorError::AuditCorrupt)?,
        ));
        let checksum = u64::from_le_bytes(
            bytes[offset + 8..offset + 16]
                .try_into()
                .map_err(|_| ExecutionOperatorError::AuditCorrupt)?,
        );
        if magic != OPERATOR_AUDIT_MAGIC || version != OPERATOR_AUDIT_VERSION {
            return Err(ExecutionOperatorError::AuditCorrupt);
        }
        let frame_len = OPERATOR_AUDIT_HEADER_LEN.saturating_add(payload_len);
        if bytes.len().saturating_sub(offset) < frame_len {
            return Err(ExecutionOperatorError::AuditCorrupt);
        }
        let payload = &bytes[offset + OPERATOR_AUDIT_HEADER_LEN..offset + frame_len];
        if operator_audit_checksum(payload) != checksum {
            return Err(ExecutionOperatorError::AuditCorrupt);
        }
        let record = decode_operator_audit_record(payload)?;
        if record.sequence != expected_sequence {
            return Err(ExecutionOperatorError::AuditSequence);
        }
        expected_sequence = expected_sequence.saturating_add(1);
        records.push(record);
        offset += frame_len;
    }
    Ok(records)
}

fn decode_operator_audit_record(
    payload: &[u8],
) -> Result<ExecutionOperatorAuditRecord, ExecutionOperatorError> {
    let mut cursor = OperatorAuditCursor::new(payload);
    let sequence = cursor.u64()?;
    let id = ExecutionOperatorCommandId::new(cursor.u64()?)?;
    let action = decode_action(&mut cursor)?;
    let issued_ns = cursor.u64()?;
    let reason = ExecutionText::new(cursor.text_u16()?)?;
    let actor_id = ExecutionOperatorActorId::new(cursor.text_u8()?)?;
    let authorization_permissions = ExecutionOperatorPermissions(cursor.u64()?);
    let status = match cursor.u8()? {
        0 => ExecutionOperatorStatus::Requested,
        1 => ExecutionOperatorStatus::Denied,
        2 => ExecutionOperatorStatus::Succeeded,
        3 => ExecutionOperatorStatus::Failed,
        _ => return Err(ExecutionOperatorError::AuditCorrupt),
    };
    let recorded_ns = cursor.u64()?;
    let outcome = ExecutionOperatorOutcome {
        selected: cursor.u64()?,
        attempted: cursor.u64()?,
        succeeded: cursor.u64()?,
        failed: cursor.u64()?,
        events: cursor.u64()?,
        value: cursor.u64()?,
        failure: decode_failure(cursor.u16()?)?,
    };
    if !cursor.is_empty() {
        return Err(ExecutionOperatorError::AuditCorrupt);
    }
    Ok(ExecutionOperatorAuditRecord {
        sequence,
        command: ExecutionOperatorCommand::new(id, action, issued_ns, reason)?,
        actor_id,
        authorization_permissions,
        status,
        recorded_ns,
        outcome,
    })
}

fn encode_action(out: &mut Vec<u8>, action: ExecutionOperatorAction) {
    out.push(action.code());
    match action {
        ExecutionOperatorAction::DrainRoute(route)
        | ExecutionOperatorAction::RestoreRoute(route) => encode_route(out, route),
        ExecutionOperatorAction::CancelScope(scope) => encode_scope(out, scope),
        ExecutionOperatorAction::InspectStuckOrders {
            scope,
            stale_after_ns,
        } => {
            encode_scope(out, scope);
            put_u64(out, stale_after_ns);
        }
        ExecutionOperatorAction::MarkRouteDegraded { route, degraded } => {
            encode_route(out, route);
            out.push(u8::from(degraded));
        }
        ExecutionOperatorAction::ClearKillSwitch { switch_id, force } => {
            put_u64(out, switch_id.get());
            out.push(u8::from(force));
        }
        ExecutionOperatorAction::PauseSubmissions
        | ExecutionOperatorAction::ResumeSubmissions
        | ExecutionOperatorAction::CancelAll
        | ExecutionOperatorAction::RecoverOpenOrders
        | ExecutionOperatorAction::Reconcile
        | ExecutionOperatorAction::ExportAuditBundle
        | ExecutionOperatorAction::RotateWalSegment
        | ExecutionOperatorAction::ForceCheckpoint => {}
    }
}

fn decode_action(
    cursor: &mut OperatorAuditCursor<'_>,
) -> Result<ExecutionOperatorAction, ExecutionOperatorError> {
    Ok(match cursor.u8()? {
        0 => ExecutionOperatorAction::PauseSubmissions,
        1 => ExecutionOperatorAction::ResumeSubmissions,
        2 => ExecutionOperatorAction::DrainRoute(decode_route(cursor)?),
        3 => ExecutionOperatorAction::RestoreRoute(decode_route(cursor)?),
        4 => ExecutionOperatorAction::CancelAll,
        5 => ExecutionOperatorAction::CancelScope(decode_scope(cursor)?),
        6 => ExecutionOperatorAction::RecoverOpenOrders,
        7 => ExecutionOperatorAction::Reconcile,
        8 => ExecutionOperatorAction::ExportAuditBundle,
        9 => ExecutionOperatorAction::InspectStuckOrders {
            scope: decode_scope(cursor)?,
            stale_after_ns: cursor.u64()?,
        },
        10 => ExecutionOperatorAction::RotateWalSegment,
        11 => ExecutionOperatorAction::ForceCheckpoint,
        12 => ExecutionOperatorAction::MarkRouteDegraded {
            route: decode_route(cursor)?,
            degraded: decode_bool(cursor.u8()?)?,
        },
        13 => ExecutionOperatorAction::ClearKillSwitch {
            switch_id: crate::KillSwitchId::new(cursor.u64()?)
                .map_err(|_| ExecutionOperatorError::AuditCorrupt)?,
            force: decode_bool(cursor.u8()?)?,
        },
        _ => return Err(ExecutionOperatorError::AuditCorrupt),
    })
}

fn encode_route(out: &mut Vec<u8>, route: RouteKey) {
    put_text_u8(out, route.route_id.as_str()).expect("fixed route id fits u8");
    put_text_u8(out, route.account_id.as_str()).expect("fixed account id fits u8");
    put_text_u8(out, route.symbol.venue.as_str()).expect("fixed venue id fits u8");
    put_text_u8(out, route.symbol.instrument.as_str()).expect("fixed instrument id fits u8");
}

fn decode_route(cursor: &mut OperatorAuditCursor<'_>) -> Result<RouteKey, ExecutionOperatorError> {
    let route_id = RouteId::new(cursor.text_u8()?)?;
    let account_id = AccountId::new(cursor.text_u8()?)?;
    let venue = cursor.text_u8()?;
    let instrument = cursor.text_u8()?;
    Ok(RouteKey::new(
        route_id,
        account_id,
        ExecutionSymbol::new(venue, instrument)?,
    ))
}

fn encode_scope(out: &mut Vec<u8>, scope: ExecutionOperatorOrderScope) {
    match scope {
        ExecutionOperatorOrderScope::Global => out.push(0),
        ExecutionOperatorOrderScope::Route(route) => {
            out.push(1);
            encode_route(out, route);
        }
        ExecutionOperatorOrderScope::Account(account) => {
            out.push(2);
            put_text_u8(out, account.as_str()).expect("fixed account id fits u8");
        }
        ExecutionOperatorOrderScope::Strategy(strategy) => {
            out.push(3);
            put_text_u8(out, strategy.as_str()).expect("fixed strategy id fits u8");
        }
        ExecutionOperatorOrderScope::Symbol(symbol) => {
            out.push(4);
            put_text_u8(out, symbol.venue.as_str()).expect("fixed venue id fits u8");
            put_text_u8(out, symbol.instrument.as_str()).expect("fixed instrument id fits u8");
        }
        ExecutionOperatorOrderScope::Order(order) => {
            out.push(5);
            put_text_u8(out, order.as_str()).expect("fixed client id fits u8");
        }
    }
}

fn decode_scope(
    cursor: &mut OperatorAuditCursor<'_>,
) -> Result<ExecutionOperatorOrderScope, ExecutionOperatorError> {
    Ok(match cursor.u8()? {
        0 => ExecutionOperatorOrderScope::Global,
        1 => ExecutionOperatorOrderScope::Route(decode_route(cursor)?),
        2 => ExecutionOperatorOrderScope::Account(AccountId::new(cursor.text_u8()?)?),
        3 => ExecutionOperatorOrderScope::Strategy(StrategyId::new(cursor.text_u8()?)?),
        4 => {
            let venue = cursor.text_u8()?;
            let instrument = cursor.text_u8()?;
            ExecutionOperatorOrderScope::Symbol(ExecutionSymbol::new(venue, instrument)?)
        }
        5 => ExecutionOperatorOrderScope::Order(ClientOrderId::new(cursor.text_u8()?)?),
        _ => return Err(ExecutionOperatorError::AuditCorrupt),
    })
}

fn decode_failure(value: u16) -> Result<ExecutionOperatorFailureCode, ExecutionOperatorError> {
    Ok(match value {
        0 => ExecutionOperatorFailureCode::None,
        1 => ExecutionOperatorFailureCode::PermissionDenied,
        2 => ExecutionOperatorFailureCode::RouteNotFound,
        3 => ExecutionOperatorFailureCode::Execution,
        4 => ExecutionOperatorFailureCode::Reconciliation,
        5 => ExecutionOperatorFailureCode::AuditExport,
        6 => ExecutionOperatorFailureCode::WalRotation,
        7 => ExecutionOperatorFailureCode::Checkpoint,
        8 => ExecutionOperatorFailureCode::KillSwitch,
        9 => ExecutionOperatorFailureCode::Unsupported,
        _ => return Err(ExecutionOperatorError::AuditCorrupt),
    })
}

fn decode_bool(value: u8) -> Result<bool, ExecutionOperatorError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ExecutionOperatorError::AuditCorrupt),
    }
}

fn put_text_u8(out: &mut Vec<u8>, value: &str) -> Result<(), ExecutionOperatorError> {
    let len = u8::try_from(value.len()).map_err(|_| ExecutionOperatorError::AuditCorrupt)?;
    out.push(len);
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn put_text_u16(out: &mut Vec<u8>, value: &str) -> Result<(), ExecutionOperatorError> {
    let len = u16::try_from(value.len()).map_err(|_| ExecutionOperatorError::AuditCorrupt)?;
    put_u16(out, len);
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn operator_audit_checksum(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

struct OperatorAuditCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> OperatorAuditCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], ExecutionOperatorError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(ExecutionOperatorError::AuditCorrupt)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(ExecutionOperatorError::AuditCorrupt)?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, ExecutionOperatorError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ExecutionOperatorError> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| ExecutionOperatorError::AuditCorrupt)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, ExecutionOperatorError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| ExecutionOperatorError::AuditCorrupt)?,
        ))
    }

    fn text_u8(&mut self) -> Result<&'a str, ExecutionOperatorError> {
        let len = usize::from(self.u8()?);
        std::str::from_utf8(self.take(len)?).map_err(|_| ExecutionOperatorError::AuditCorrupt)
    }

    fn text_u16(&mut self) -> Result<&'a str, ExecutionOperatorError> {
        let len = usize::from(self.u16()?);
        std::str::from_utf8(self.take(len)?).map_err(|_| ExecutionOperatorError::AuditCorrupt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AllowAllRiskGate, InMemoryJournal, RouteConfig, SimExecutionAdapter};
    use of_execution_core::{
        OrderPrice, OrderQty, OrderRequest, OrderSide, OrderType, RiskLimits, RouteId, TimeInForce,
    };

    #[derive(Default)]
    struct Services {
        reconcile_calls: u64,
        export_calls: u64,
        rotate_calls: u64,
        checkpoint_calls: u64,
        clear_calls: u64,
    }

    #[derive(Default)]
    struct FlakyAudit {
        records: Vec<ExecutionOperatorAuditRecord>,
        fail_sequence: Option<u64>,
    }

    impl ExecutionOperatorAuditSink for FlakyAudit {
        fn reserve(&self, _additional: usize) -> Result<(), ExecutionOperatorError> {
            Ok(())
        }

        fn append(
            &mut self,
            record: ExecutionOperatorAuditRecord,
        ) -> Result<(), ExecutionOperatorError> {
            if self.fail_sequence == Some(record.sequence) {
                return Err(ExecutionOperatorError::AuditIo);
            }
            self.records.push(record);
            Ok(())
        }
    }

    impl ExecutionOperatorServices<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal>
        for Services
    {
        fn reconcile(
            &mut self,
            _engine: &mut ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal>,
            _command: &ExecutionOperatorCommand,
        ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError> {
            self.reconcile_calls += 1;
            Ok(ExecutionOperatorOutcome::success(11))
        }

        fn export_audit_bundle(
            &mut self,
            _engine: &mut ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal>,
            _command: &ExecutionOperatorCommand,
        ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError> {
            self.export_calls += 1;
            Ok(ExecutionOperatorOutcome::success(12))
        }

        fn rotate_wal_segment(
            &mut self,
            _engine: &mut ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal>,
            _command: &ExecutionOperatorCommand,
        ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError> {
            self.rotate_calls += 1;
            Ok(ExecutionOperatorOutcome::success(13))
        }

        fn force_checkpoint(
            &mut self,
            _engine: &mut ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal>,
            _command: &ExecutionOperatorCommand,
        ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError> {
            self.checkpoint_calls += 1;
            Ok(ExecutionOperatorOutcome::success(14))
        }

        fn clear_kill_switch(
            &mut self,
            _engine: &mut ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal>,
            _switch_id: crate::KillSwitchId,
            _force: bool,
            _command: &ExecutionOperatorCommand,
        ) -> Result<ExecutionOperatorOutcome, ExecutionOperatorServiceError> {
            self.clear_calls += 1;
            Ok(ExecutionOperatorOutcome::success(15))
        }
    }

    fn route() -> RouteConfig {
        let risk_limits = RiskLimits {
            kill_switch: false,
            ..RiskLimits::default()
        };
        RouteConfig {
            route_id: RouteId::new("SIM").unwrap(),
            account_id: AccountId::new("A1").unwrap(),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            enabled: true,
            risk_limits,
        }
    }

    fn engine() -> ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal> {
        let mut engine = ExecutionEngine::new(
            SimExecutionAdapter::default().with_partial_fill(true),
            AllowAllRiskGate,
            InMemoryJournal::default(),
            vec![route()],
        );
        engine.start().unwrap();
        engine
    }

    fn auth(permissions: ExecutionOperatorPermissions) -> ExecutionOperatorAuthorization {
        ExecutionOperatorAuthorization::from_actor("ops-user", permissions).unwrap()
    }

    fn command(id: u64, action: ExecutionOperatorAction) -> ExecutionOperatorCommand {
        ExecutionOperatorCommand::from_reason(
            ExecutionOperatorCommandId::new(id).unwrap(),
            action,
            10_000 + id,
            "incident response",
        )
        .unwrap()
    }

    fn order(id: &str, strategy: &str) -> OrderRequest {
        let route = route();
        OrderRequest {
            client_order_id: ClientOrderId::new(id).unwrap(),
            account_id: route.account_id,
            route_id: route.route_id,
            strategy_id: StrategyId::new(strategy).unwrap(),
            symbol: route.symbol,
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(10),
            limit_price: OrderPrice(5_000),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }
    }

    #[test]
    fn authorization_denial_is_journaled_without_mutation() {
        let mut engine = engine();
        let mut controller = ExecutionOperatorController::with_capacity(4).unwrap();
        let mut audit = InMemoryExecutionOperatorAudit::with_capacity(8).unwrap();
        let mut services = Services::default();
        let mut events = ExecutionEventBuffer::with_capacity(16);
        let mut stuck = ExecutionStuckOrderBuffer::with_capacity(16).unwrap();
        let receipt = controller
            .execute(
                &mut engine,
                &mut services,
                &mut audit,
                auth(ExecutionOperatorPermissions::none()),
                command(1, ExecutionOperatorAction::PauseSubmissions),
                20_000,
                &mut events,
                &mut stuck,
            )
            .unwrap();
        assert_eq!(receipt.status, ExecutionOperatorStatus::Denied);
        assert!(!engine.runbook_snapshot().submissions_paused);
        assert_eq!(audit.records().len(), 2);
    }

    #[test]
    fn exact_retry_is_idempotent_and_collision_fails() {
        let mut engine = engine();
        let mut controller = ExecutionOperatorController::with_capacity(4).unwrap();
        let mut audit = InMemoryExecutionOperatorAudit::with_capacity(8).unwrap();
        let mut services = Services::default();
        let mut events = ExecutionEventBuffer::with_capacity(16);
        let mut stuck = ExecutionStuckOrderBuffer::with_capacity(16).unwrap();
        let auth = auth(ExecutionOperatorPermissions::all());
        let original = command(1, ExecutionOperatorAction::PauseSubmissions);
        let first = controller
            .execute(
                &mut engine,
                &mut services,
                &mut audit,
                auth,
                original,
                20_000,
                &mut events,
                &mut stuck,
            )
            .unwrap();
        let second = controller
            .execute(
                &mut engine,
                &mut services,
                &mut audit,
                auth,
                original,
                30_000,
                &mut events,
                &mut stuck,
            )
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(audit.records().len(), 2);
        let collision = command(1, ExecutionOperatorAction::ResumeSubmissions);
        assert_eq!(
            controller.execute(
                &mut engine,
                &mut services,
                &mut audit,
                auth,
                collision,
                30_000,
                &mut events,
                &mut stuck,
            ),
            Err(ExecutionOperatorError::CommandIdCollision)
        );
    }

    #[test]
    fn pause_drain_degrade_and_restore_gate_only_new_orders() {
        let mut engine = engine();
        let mut controller = ExecutionOperatorController::with_capacity(8).unwrap();
        let mut audit = InMemoryExecutionOperatorAudit::with_capacity(16).unwrap();
        let mut services = Services::default();
        let mut events = ExecutionEventBuffer::with_capacity(32);
        let mut stuck = ExecutionStuckOrderBuffer::with_capacity(16).unwrap();
        let auth = auth(ExecutionOperatorPermissions::all());
        let key = RouteKey::new(route().route_id, route().account_id, route().symbol);

        for (id, action) in [
            (1, ExecutionOperatorAction::PauseSubmissions),
            (2, ExecutionOperatorAction::ResumeSubmissions),
            (3, ExecutionOperatorAction::DrainRoute(key)),
            (4, ExecutionOperatorAction::RestoreRoute(key)),
            (
                5,
                ExecutionOperatorAction::MarkRouteDegraded {
                    route: key,
                    degraded: true,
                },
            ),
            (
                6,
                ExecutionOperatorAction::MarkRouteDegraded {
                    route: key,
                    degraded: false,
                },
            ),
        ] {
            controller
                .execute(
                    &mut engine,
                    &mut services,
                    &mut audit,
                    auth,
                    command(id, action),
                    20_000 + id,
                    &mut events,
                    &mut stuck,
                )
                .unwrap();
        }
        assert!(engine.runbook_snapshot().can_submit_new_orders());
        engine.submit(order("ORDER-1", "S1"), &mut events).unwrap();
    }

    #[test]
    fn scoped_cancel_and_stuck_inspection_are_deterministic() {
        let mut engine = engine();
        let mut events = ExecutionEventBuffer::with_capacity(64);
        engine.submit(order("ORDER-1", "S1"), &mut events).unwrap();
        engine.submit(order("ORDER-2", "S2"), &mut events).unwrap();
        let mut controller = ExecutionOperatorController::with_capacity(4).unwrap();
        let mut audit = InMemoryExecutionOperatorAudit::with_capacity(8).unwrap();
        let mut services = Services::default();
        let mut stuck = ExecutionStuckOrderBuffer::with_capacity(8).unwrap();
        let auth = auth(ExecutionOperatorPermissions::all());

        let inspect = controller
            .execute(
                &mut engine,
                &mut services,
                &mut audit,
                auth,
                command(
                    1,
                    ExecutionOperatorAction::InspectStuckOrders {
                        scope: ExecutionOperatorOrderScope::Strategy(
                            StrategyId::new("S1").unwrap(),
                        ),
                        stale_after_ns: 1,
                    },
                ),
                20_000,
                &mut events,
                &mut stuck,
            )
            .unwrap();
        assert_eq!(inspect.outcome.selected, 1);
        assert_eq!(stuck.as_slice()[0].client_order_id.as_str(), "ORDER-1");

        let cancel = controller
            .execute(
                &mut engine,
                &mut services,
                &mut audit,
                auth,
                command(
                    2,
                    ExecutionOperatorAction::CancelScope(ExecutionOperatorOrderScope::Strategy(
                        StrategyId::new("S1").unwrap(),
                    )),
                ),
                20_001,
                &mut events,
                &mut stuck,
            )
            .unwrap();
        assert_eq!(cancel.outcome.selected, 1);
        assert_eq!(cancel.outcome.succeeded, 1);
    }

    #[test]
    fn deployment_owned_actions_dispatch_exactly_once() {
        let mut engine = engine();
        let mut controller = ExecutionOperatorController::with_capacity(8).unwrap();
        let mut audit = InMemoryExecutionOperatorAudit::with_capacity(16).unwrap();
        let mut services = Services::default();
        let mut events = ExecutionEventBuffer::with_capacity(16);
        let mut stuck = ExecutionStuckOrderBuffer::with_capacity(16).unwrap();
        let auth = auth(ExecutionOperatorPermissions::all());
        let switch = crate::KillSwitchId::new(1).unwrap();
        let actions = [
            ExecutionOperatorAction::Reconcile,
            ExecutionOperatorAction::ExportAuditBundle,
            ExecutionOperatorAction::RotateWalSegment,
            ExecutionOperatorAction::ForceCheckpoint,
            ExecutionOperatorAction::ClearKillSwitch {
                switch_id: switch,
                force: false,
            },
        ];
        for (index, action) in actions.into_iter().enumerate() {
            controller
                .execute(
                    &mut engine,
                    &mut services,
                    &mut audit,
                    auth,
                    command(index as u64 + 1, action),
                    20_000 + index as u64,
                    &mut events,
                    &mut stuck,
                )
                .unwrap();
        }
        assert_eq!(services.reconcile_calls, 1);
        assert_eq!(services.export_calls, 1);
        assert_eq!(services.rotate_calls, 1);
        assert_eq!(services.checkpoint_calls, 1);
        assert_eq!(services.clear_calls, 1);
    }

    #[test]
    fn failed_outcome_audit_pauses_and_retry_repairs_without_redispatch() {
        let mut engine = engine();
        let mut controller = ExecutionOperatorController::with_capacity(4).unwrap();
        let mut audit = FlakyAudit {
            fail_sequence: Some(2),
            ..FlakyAudit::default()
        };
        let mut services = Services::default();
        let mut events = ExecutionEventBuffer::with_capacity(16);
        let mut stuck = ExecutionStuckOrderBuffer::with_capacity(16).unwrap();
        let auth = auth(ExecutionOperatorPermissions::all());
        let rotate = command(1, ExecutionOperatorAction::RotateWalSegment);
        assert_eq!(
            controller.execute(
                &mut engine,
                &mut services,
                &mut audit,
                auth,
                rotate,
                20_000,
                &mut events,
                &mut stuck,
            ),
            Err(ExecutionOperatorError::AuditOutcomeFailed)
        );
        assert!(engine.runbook_snapshot().submissions_paused);
        assert_eq!(services.rotate_calls, 1);
        assert_eq!(
            controller.execute(
                &mut engine,
                &mut services,
                &mut audit,
                auth,
                command(2, ExecutionOperatorAction::ForceCheckpoint),
                20_001,
                &mut events,
                &mut stuck,
            ),
            Err(ExecutionOperatorError::AuditRepairRequired)
        );

        audit.fail_sequence = None;
        let repaired = controller
            .execute(
                &mut engine,
                &mut services,
                &mut audit,
                auth,
                rotate,
                30_000,
                &mut events,
                &mut stuck,
            )
            .unwrap();
        assert_eq!(repaired.status, ExecutionOperatorStatus::Succeeded);
        assert_eq!(services.rotate_calls, 1);
        assert_eq!(audit.records.len(), 2);
    }

    #[test]
    fn file_audit_round_trips_all_actions_and_restores_idempotency() {
        let path = std::env::temp_dir().join(format!(
            "orderflow-operator-audit-{}.wal",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let route = RouteKey::new(route().route_id, route().account_id, route().symbol);
        let actions = [
            ExecutionOperatorAction::PauseSubmissions,
            ExecutionOperatorAction::ResumeSubmissions,
            ExecutionOperatorAction::DrainRoute(route),
            ExecutionOperatorAction::RestoreRoute(route),
            ExecutionOperatorAction::CancelAll,
            ExecutionOperatorAction::CancelScope(ExecutionOperatorOrderScope::Account(
                route.account_id,
            )),
            ExecutionOperatorAction::RecoverOpenOrders,
            ExecutionOperatorAction::Reconcile,
            ExecutionOperatorAction::ExportAuditBundle,
            ExecutionOperatorAction::InspectStuckOrders {
                scope: ExecutionOperatorOrderScope::Symbol(route.symbol),
                stale_after_ns: 99,
            },
            ExecutionOperatorAction::RotateWalSegment,
            ExecutionOperatorAction::ForceCheckpoint,
            ExecutionOperatorAction::MarkRouteDegraded {
                route,
                degraded: true,
            },
            ExecutionOperatorAction::ClearKillSwitch {
                switch_id: crate::KillSwitchId::new(9).unwrap(),
                force: true,
            },
        ];
        let actor = ExecutionOperatorActorId::new("audit-user").unwrap();
        let mut expected = Vec::new();
        let mut audit = FileExecutionOperatorAudit::open(&path, true).unwrap();
        for (index, action) in actions.into_iter().enumerate() {
            let command = command(index as u64 + 1, action);
            let requested = ExecutionOperatorAuditRecord {
                sequence: index as u64 * 2 + 1,
                command,
                actor_id: actor,
                authorization_permissions: ExecutionOperatorPermissions::all(),
                status: ExecutionOperatorStatus::Requested,
                recorded_ns: command.issued_ns(),
                outcome: ExecutionOperatorOutcome::default(),
            };
            let completed = ExecutionOperatorAuditRecord {
                sequence: requested.sequence + 1,
                command,
                actor_id: actor,
                authorization_permissions: ExecutionOperatorPermissions::all(),
                status: ExecutionOperatorStatus::Succeeded,
                recorded_ns: 30_000 + index as u64,
                outcome: ExecutionOperatorOutcome::success(index as u64),
            };
            audit.append(requested).unwrap();
            audit.append(completed).unwrap();
            expected.extend([requested, completed]);
        }
        assert_eq!(audit.replay().unwrap(), expected);
        let restored = ExecutionOperatorController::restore(&expected, 32).unwrap();
        assert_eq!(restored.receipts().len(), actions.len());
        let mut restored_engine = engine();
        restored.restore_engine_controls(&mut restored_engine);
        assert_eq!(restored_engine.runbook_snapshot().degraded_route_count, 1);
        assert!(!restored_engine.runbook_snapshot().can_submit_new_orders());
        drop(audit);

        let mut bytes = std::fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x80;
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(
            FileExecutionOperatorAudit::open(&path, false).unwrap_err(),
            ExecutionOperatorError::AuditCorrupt
        );
        let _ = std::fs::remove_file(path);
    }
}
