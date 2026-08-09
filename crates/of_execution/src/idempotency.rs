//! Bounded command idempotency and execution-report duplicate protection.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

use of_execution_core::{
    AmendRequest, CancelRequest, ClientOrderId, ExecutionEvent, ExecutionId, ExecutionSymbol,
    FixedAscii, OrderPrice, OrderQty, OrderRequest, OrderSide, OrderType, TimeInForce,
};

use crate::{CommandId, ExecutionCommandKind, RequestId};

/// Maximum bytes stored in an idempotency scope identifier.
pub const IDEMPOTENCY_SCOPE_ID_CAPACITY: usize = 40;
/// Maximum bytes stored in an adapter command identifier.
pub const ADAPTER_COMMAND_ID_CAPACITY: usize = 64;
/// Maximum bytes stored in an execution-report source identifier.
pub const EXECUTION_REPORT_SOURCE_ID_CAPACITY: usize = 40;

const IDEMPOTENCY_CHECKPOINT_MAGIC: [u8; 4] = *b"OFIC";
const REPORT_DEDUP_CHECKPOINT_MAGIC: [u8; 4] = *b"OFRD";
const CHECKPOINT_HEADER_LEN: usize = 20;
const REPORT_CHECKPOINT_HEADER_LEN: usize = 12;
const CHECKPOINT_TRAILER_LEN: usize = 8;

/// Caller-defined tenant, strategy gateway, or session scope for request IDs.
pub type IdempotencyScopeId = FixedAscii<IDEMPOTENCY_SCOPE_ID_CAPACITY>;
/// Provider-specific identifier attached to an outbound command.
pub type AdapterCommandId = FixedAscii<ADAPTER_COMMAND_ID_CAPACITY>;
/// Source/session identity used to scope execution-report identities.
pub type ExecutionReportSourceId = FixedAscii<EXECUTION_REPORT_SOURCE_ID_CAPACITY>;

/// Scope plus caller request ID forming one idempotency key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdempotencyKey {
    /// Stable caller/session scope.
    pub scope_id: IdempotencyScopeId,
    /// Stable request ID reused by retries of the same semantic command.
    pub request_id: RequestId,
}

impl IdempotencyKey {
    /// Creates a non-empty idempotency key.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::MissingIdentity`] when either component is
    /// empty.
    pub fn new(
        scope_id: IdempotencyScopeId,
        request_id: RequestId,
    ) -> Result<Self, IdempotencyError> {
        let key = Self {
            scope_id,
            request_id,
        };
        if !key.is_valid() {
            return Err(IdempotencyError::MissingIdentity);
        }
        Ok(key)
    }

    fn is_valid(self) -> bool {
        !self.scope_id.is_empty() && !self.request_id.0.is_empty()
    }
}

/// Mutating execution command protected by an idempotency key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IdempotentExecutionCommand {
    /// Submit a new order.
    Submit(OrderRequest),
    /// Cancel an existing order.
    Cancel(CancelRequest),
    /// Amend or cancel-replace an existing order.
    Amend(AmendRequest),
}

impl IdempotentExecutionCommand {
    /// Returns the existing execution command kind.
    pub const fn kind(self) -> ExecutionCommandKind {
        match self {
            Self::Submit(_) => ExecutionCommandKind::Submit,
            Self::Cancel(_) => ExecutionCommandKind::Cancel,
            Self::Amend(_) => ExecutionCommandKind::Amend,
        }
    }

    /// Returns the client order ID carried by this command.
    pub const fn client_order_id(self) -> ClientOrderId {
        match self {
            Self::Submit(request) => request.client_order_id,
            Self::Cancel(request) => request.client_order_id,
            Self::Amend(request) => request.client_order_id,
        }
    }

    /// Compares command intent while ignoring transport timestamps.
    ///
    /// Retry callers may reconstruct a request with a new receive timestamp.
    /// Every economic, routing, ownership, and order-lifecycle field must still
    /// match the first accepted request.
    pub fn semantically_matches(self, other: Self) -> bool {
        match (self, other) {
            (Self::Submit(left), Self::Submit(right)) => submit_matches(left, right),
            (Self::Cancel(left), Self::Cancel(right)) => cancel_matches(left, right),
            (Self::Amend(left), Self::Amend(right)) => amend_matches(left, right),
            _ => false,
        }
    }

    fn is_valid(self) -> bool {
        match self {
            Self::Submit(request) => {
                !request.client_order_id.is_empty()
                    && !request.account_id.is_empty()
                    && !request.route_id.is_empty()
                    && !request.symbol.venue.is_empty()
                    && !request.symbol.instrument.is_empty()
                    && request.validate().is_ok()
            }
            Self::Cancel(request) => {
                !request.client_order_id.is_empty()
                    && !request.orig_client_order_id.is_empty()
                    && !request.account_id.is_empty()
                    && !request.route_id.is_empty()
                    && !request.symbol.venue.is_empty()
                    && !request.symbol.instrument.is_empty()
            }
            Self::Amend(request) => {
                !request.client_order_id.is_empty()
                    && !request.orig_client_order_id.is_empty()
                    && !request.account_id.is_empty()
                    && !request.route_id.is_empty()
                    && !request.symbol.venue.is_empty()
                    && !request.symbol.instrument.is_empty()
                    && request.quantity.0 > 0
            }
        }
    }
}

/// Durable lifecycle state for one idempotent command.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IdempotencyState {
    /// Key and semantic command are reserved but not yet journaled.
    Reserved = 1,
    /// Command has crossed the host's durable journal boundary.
    Journaled = 2,
    /// Adapter send was attempted with a stable provider identifier.
    Sent = 3,
    /// Venue or authoritative downstream source acknowledged the command.
    Acknowledged = 4,
    /// Risk, adapter, or venue rejected the command definitively.
    Rejected = 5,
    /// Command failed definitively and must not be retried under this key.
    FailedDefinitive = 6,
    /// Outcome is uncertain and reconciliation is required before any retry.
    RecoveryPending = 7,
}

impl IdempotencyState {
    /// Returns whether this state is a definitive terminal outcome.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Acknowledged | Self::Rejected | Self::FailedDefinitive
        )
    }
}

/// Definitive outcome supplied after local or venue processing.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IdempotencyCompletion {
    /// Command was accepted or completed authoritatively.
    Acknowledged = 1,
    /// Command was rejected authoritatively.
    Rejected = 2,
    /// Command failed without any possibility of later acceptance.
    FailedDefinitive = 3,
}

impl IdempotencyCompletion {
    const fn state(self) -> IdempotencyState {
        match self {
            Self::Acknowledged => IdempotencyState::Acknowledged,
            Self::Rejected => IdempotencyState::Rejected,
            Self::FailedDefinitive => IdempotencyState::FailedDefinitive,
        }
    }
}

/// Stored command correlation and retry state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct IdempotencyRecord {
    /// Idempotency key.
    pub key: IdempotencyKey,
    /// OMS command ID assigned to the original request.
    pub command_id: CommandId,
    /// Original semantic command and canonical IDs.
    pub command: IdempotentExecutionCommand,
    /// Current durable lifecycle state.
    pub state: IdempotencyState,
    /// Stable provider command ID, once assigned.
    pub adapter_command_id: Option<AdapterCommandId>,
    /// Number of adapter send attempts using the same semantic command and IDs.
    pub send_attempts: u32,
    /// First reservation timestamp supplied by the host.
    pub created_ns: u64,
    /// Last mutation timestamp supplied by the host.
    pub updated_ns: u64,
    /// Registry mutation sequence of the last state change.
    pub last_sequence: u64,
}

/// Result of reserving a command key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IdempotencyDecision {
    /// A new key was reserved and the command may proceed to journaling.
    Accepted(IdempotencyRecord),
    /// The same key and semantic parameters were observed previously.
    Duplicate(IdempotencyRecord),
}

impl IdempotencyDecision {
    /// Returns the original record for either decision.
    pub const fn record(self) -> IdempotencyRecord {
        match self {
            Self::Accepted(record) | Self::Duplicate(record) => record,
        }
    }

    /// Returns true when no new command should be emitted.
    pub const fn is_duplicate(self) -> bool {
        matches!(self, Self::Duplicate(_))
    }
}

/// Bounded idempotency registry metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct IdempotencyMetrics {
    /// New command reservations.
    pub accepted: u64,
    /// Matching retries suppressed and resolved from retained state.
    pub duplicates: u64,
    /// Same keys presented with different semantic parameters.
    pub parameter_mismatches: u64,
    /// Reused command IDs associated with another key.
    pub command_id_collisions: u64,
    /// Reused client order IDs associated with another key.
    pub client_order_id_collisions: u64,
    /// Reused provider command IDs associated with another key.
    pub adapter_id_collisions: u64,
    /// New reservations refused at configured capacity.
    pub capacity_rejections: u64,
    /// Successful lifecycle transitions.
    pub transitions: u64,
    /// Restored or explicitly marked uncertain commands awaiting reconciliation.
    pub recovery_pending: u64,
    /// Explicitly retired terminal records.
    pub retired: u64,
}

/// Idempotency validation or lifecycle error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IdempotencyError {
    /// Required scope, request, command, or order identity is empty.
    MissingIdentity,
    /// Command payload is not valid for idempotent admission.
    InvalidCommand,
    /// Registry capacity is exhausted; no key was forgotten or admitted.
    CapacityExceeded,
    /// The same key was reused with different semantic parameters.
    ParameterMismatch,
    /// A command ID is already associated with another idempotency key.
    CommandIdCollision,
    /// A client order ID is already associated with another idempotency key.
    ClientOrderIdCollision,
    /// A provider command ID is already associated with another idempotency key.
    AdapterIdCollision,
    /// The key is not retained by this registry.
    NotFound,
    /// Mutation sequence is zero or not greater than the registry sequence.
    SequenceRegression,
    /// Mutation timestamp is earlier than the record's last timestamp.
    TimestampRegression,
    /// Requested lifecycle transition is invalid.
    InvalidTransition,
    /// A retry changed the provider command identifier.
    AdapterIdMismatch,
    /// Only a definitive terminal record may be retired.
    NotTerminal,
    /// Recovery checkpoint failed schema, checksum, capacity, or invariant checks.
    InvalidCheckpoint,
    /// Caller-provided checkpoint output buffer is too small.
    CheckpointBufferTooSmall {
        /// Required encoded bytes.
        required: usize,
        /// Available output bytes.
        actual: usize,
    },
}

impl fmt::Display for IdempotencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingIdentity => "idempotency identity is missing",
            Self::InvalidCommand => "idempotent command is invalid",
            Self::CapacityExceeded => "idempotency registry capacity is exhausted",
            Self::ParameterMismatch => "idempotency key parameters do not match",
            Self::CommandIdCollision => "command id is associated with another request",
            Self::ClientOrderIdCollision => "client order id is associated with another request",
            Self::AdapterIdCollision => "adapter command id is associated with another request",
            Self::NotFound => "idempotency key is not retained",
            Self::SequenceRegression => "idempotency mutation sequence regressed",
            Self::TimestampRegression => "idempotency mutation timestamp regressed",
            Self::InvalidTransition => "idempotency lifecycle transition is invalid",
            Self::AdapterIdMismatch => "adapter command id changed across retry",
            Self::NotTerminal => "idempotency record is not terminal",
            Self::InvalidCheckpoint => "idempotency checkpoint is invalid",
            Self::CheckpointBufferTooSmall { .. } => {
                "idempotency checkpoint output buffer is too small"
            }
        };
        f.write_str(message)
    }
}

impl Error for IdempotencyError {}

/// Checksummed control-plane snapshot for recovery-safe command retries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IdempotencyCheckpoint {
    /// Checkpoint schema version.
    pub schema_version: u16,
    /// Highest applied registry mutation sequence.
    pub last_sequence: u64,
    /// Stable key-sorted records.
    pub records: Vec<IdempotencyRecord>,
    /// Deterministic checksum over schema, sequence, and records.
    pub checksum: u64,
}

impl IdempotencyCheckpoint {
    /// Current checkpoint schema version.
    pub const SCHEMA_VERSION: u16 = 1;

    /// Returns the exact canonical binary encoding length.
    pub fn encoded_len(&self) -> usize {
        CHECKPOINT_HEADER_LEN
            .saturating_add(
                self.records
                    .iter()
                    .map(idempotency_record_encoded_len)
                    .fold(0_usize, usize::saturating_add),
            )
            .saturating_add(CHECKPOINT_TRAILER_LEN)
    }

    /// Encodes the checkpoint into a caller-owned buffer.
    ///
    /// The format includes magic, schema, record count, complete command
    /// payloads, and the deterministic checkpoint checksum. Encoding performs
    /// no allocation.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::InvalidCheckpoint`] if the object was
    /// modified without refreshing its checksum, or
    /// [`IdempotencyError::CheckpointBufferTooSmall`] when `out` is too small.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, IdempotencyError> {
        if self.schema_version != Self::SCHEMA_VERSION
            || self.checksum != idempotency_checkpoint_checksum(self)
            || self.records.len() > u32::MAX as usize
        {
            return Err(IdempotencyError::InvalidCheckpoint);
        }
        let required = self.encoded_len();
        if out.len() < required {
            return Err(IdempotencyError::CheckpointBufferTooSmall {
                required,
                actual: out.len(),
            });
        }
        let mut writer = BinaryWriter::new(&mut out[..required]);
        writer.bytes(&IDEMPOTENCY_CHECKPOINT_MAGIC);
        writer.u16(self.schema_version);
        writer.u16(0);
        writer.u64(self.last_sequence);
        writer.u32(self.records.len() as u32);
        for record in &self.records {
            encode_idempotency_record(&mut writer, *record);
        }
        writer.u64(self.checksum);
        Ok(writer.position())
    }

    /// Decodes and checksum-validates a canonical binary checkpoint.
    ///
    /// Record lifecycle and capacity invariants are validated by
    /// [`IdempotencyRegistry::restore`]. The decoder rejects unknown schema,
    /// malformed lengths/enums, impossible record counts, and trailing bytes.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::InvalidCheckpoint`] for malformed or
    /// checksum-invalid input.
    pub fn decode(bytes: &[u8]) -> Result<Self, IdempotencyError> {
        if bytes.len() < CHECKPOINT_HEADER_LEN + CHECKPOINT_TRAILER_LEN {
            return Err(IdempotencyError::InvalidCheckpoint);
        }
        let mut reader = BinaryReader::new(bytes);
        if reader.bytes(4)? != IDEMPOTENCY_CHECKPOINT_MAGIC
            || reader.u16()? != Self::SCHEMA_VERSION
            || reader.u16()? != 0
        {
            return Err(IdempotencyError::InvalidCheckpoint);
        }
        let last_sequence = reader.u64()?;
        let count = reader.u32()? as usize;
        if count > bytes.len() / 32 {
            return Err(IdempotencyError::InvalidCheckpoint);
        }
        let mut records = Vec::with_capacity(count);
        for _ in 0..count {
            records.push(decode_idempotency_record(&mut reader)?);
        }
        let checksum = reader.u64()?;
        if !reader.is_empty() {
            return Err(IdempotencyError::InvalidCheckpoint);
        }
        let checkpoint = Self {
            schema_version: Self::SCHEMA_VERSION,
            last_sequence,
            records,
            checksum,
        };
        if checkpoint.checksum != idempotency_checkpoint_checksum(&checkpoint) {
            return Err(IdempotencyError::InvalidCheckpoint);
        }
        Ok(checkpoint)
    }
}

/// Bounded, allocation-free-after-construction command idempotency registry.
///
/// The registry performs no I/O. Hosts reserve first, durably append the
/// command and checkpoint through their WAL, call [`Self::mark_journaled`],
/// then send through an adapter. Capacity exhaustion is fail-closed: records
/// are never evicted implicitly.
#[derive(Debug)]
pub struct IdempotencyRegistry {
    capacity: usize,
    records: HashMap<IdempotencyKey, IdempotencyRecord>,
    command_index: HashMap<CommandId, IdempotencyKey>,
    client_index: HashMap<ClientOrderId, IdempotencyKey>,
    adapter_index: HashMap<AdapterCommandId, IdempotencyKey>,
    last_sequence: u64,
    metrics: IdempotencyMetrics,
}

impl IdempotencyRegistry {
    /// Creates an empty registry with fixed record capacity.
    ///
    /// # Errors
    ///
    /// Returns [`IdempotencyError::CapacityExceeded`] when `capacity` is zero.
    pub fn new(capacity: usize) -> Result<Self, IdempotencyError> {
        if capacity == 0 {
            return Err(IdempotencyError::CapacityExceeded);
        }
        Ok(Self {
            capacity,
            records: HashMap::with_capacity(capacity),
            command_index: HashMap::with_capacity(capacity),
            client_index: HashMap::with_capacity(capacity),
            adapter_index: HashMap::with_capacity(capacity),
            last_sequence: 0,
            metrics: IdempotencyMetrics::default(),
        })
    }

    /// Reserves a new command or returns the original record for a matching retry.
    ///
    /// Matching retries do not consume a sequence or mutate lifecycle state.
    /// A mismatching payload is rejected even when its transport timestamps
    /// differ legitimately.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid identities/payloads, parameter mismatch,
    /// command-ID collision, sequence regression, or capacity exhaustion.
    pub fn reserve(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        key: IdempotencyKey,
        command_id: CommandId,
        command: IdempotentExecutionCommand,
    ) -> Result<IdempotencyDecision, IdempotencyError> {
        if !key.is_valid() || command_id.0 == 0 {
            return Err(IdempotencyError::MissingIdentity);
        }
        if !command.is_valid() {
            return Err(IdempotencyError::InvalidCommand);
        }
        if let Some(existing) = self.records.get(&key).copied() {
            if existing.command.semantically_matches(command) {
                self.metrics.duplicates = self.metrics.duplicates.saturating_add(1);
                return Ok(IdempotencyDecision::Duplicate(existing));
            }
            self.metrics.parameter_mismatches = self.metrics.parameter_mismatches.saturating_add(1);
            return Err(IdempotencyError::ParameterMismatch);
        }
        if self.command_index.contains_key(&command_id) {
            self.metrics.command_id_collisions =
                self.metrics.command_id_collisions.saturating_add(1);
            return Err(IdempotencyError::CommandIdCollision);
        }
        let client_order_id = command.client_order_id();
        if self.client_index.contains_key(&client_order_id) {
            self.metrics.client_order_id_collisions =
                self.metrics.client_order_id_collisions.saturating_add(1);
            return Err(IdempotencyError::ClientOrderIdCollision);
        }
        self.check_sequence(sequence)?;
        if self.records.len() >= self.capacity {
            self.metrics.capacity_rejections = self.metrics.capacity_rejections.saturating_add(1);
            return Err(IdempotencyError::CapacityExceeded);
        }
        let record = IdempotencyRecord {
            key,
            command_id,
            command,
            state: IdempotencyState::Reserved,
            adapter_command_id: None,
            send_attempts: 0,
            created_ns: timestamp_ns,
            updated_ns: timestamp_ns,
            last_sequence: sequence,
        };
        self.records.insert(key, record);
        self.command_index.insert(command_id, key);
        self.client_index.insert(client_order_id, key);
        self.last_sequence = sequence;
        self.metrics.accepted = self.metrics.accepted.saturating_add(1);
        Ok(IdempotencyDecision::Accepted(record))
    }

    /// Marks that the command crossed the host's durable journal boundary.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing key, sequence regression, or invalid
    /// transition.
    pub fn mark_journaled(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        key: IdempotencyKey,
    ) -> Result<IdempotencyRecord, IdempotencyError> {
        self.transition(
            sequence,
            timestamp_ns,
            key,
            &[IdempotencyState::Reserved],
            IdempotencyState::Journaled,
        )
    }

    /// Marks an adapter send while preserving the first provider ID on retries.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty/changed adapter ID, missing key, sequence
    /// regression, or invalid transition.
    pub fn mark_sent(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        key: IdempotencyKey,
        adapter_command_id: AdapterCommandId,
    ) -> Result<IdempotencyRecord, IdempotencyError> {
        if adapter_command_id.is_empty() {
            return Err(IdempotencyError::MissingIdentity);
        }
        self.check_sequence(sequence)?;
        let mut record = self
            .records
            .get(&key)
            .copied()
            .ok_or(IdempotencyError::NotFound)?;
        if record.state != IdempotencyState::Journaled {
            return Err(IdempotencyError::InvalidTransition);
        }
        if record
            .adapter_command_id
            .is_some_and(|existing| existing != adapter_command_id)
        {
            return Err(IdempotencyError::AdapterIdMismatch);
        }
        if record.adapter_command_id.is_none()
            && self
                .adapter_index
                .get(&adapter_command_id)
                .is_some_and(|owner| *owner != key)
        {
            self.metrics.adapter_id_collisions =
                self.metrics.adapter_id_collisions.saturating_add(1);
            return Err(IdempotencyError::AdapterIdCollision);
        }
        record.adapter_command_id = Some(adapter_command_id);
        record.send_attempts = record.send_attempts.saturating_add(1);
        let committed =
            self.commit_record(record, sequence, timestamp_ns, IdempotencyState::Sent)?;
        self.adapter_index.insert(adapter_command_id, key);
        Ok(committed)
    }

    /// Records a definitive command outcome.
    ///
    /// Acknowledgement requires a prior send or recovery reconciliation.
    /// Rejection and definitive local failure may occur before adapter send.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing key, sequence regression, terminal
    /// record, or acknowledgement before send.
    pub fn complete(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        key: IdempotencyKey,
        completion: IdempotencyCompletion,
    ) -> Result<IdempotencyRecord, IdempotencyError> {
        self.check_sequence(sequence)?;
        let record = self
            .records
            .get(&key)
            .copied()
            .ok_or(IdempotencyError::NotFound)?;
        if record.state.is_terminal()
            || completion == IdempotencyCompletion::Acknowledged
                && !matches!(
                    record.state,
                    IdempotencyState::Sent | IdempotencyState::RecoveryPending
                )
        {
            return Err(IdempotencyError::InvalidTransition);
        }
        self.commit_record(record, sequence, timestamp_ns, completion.state())
    }

    /// Marks an outcome uncertain and blocks blind retry pending reconciliation.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing key, sequence regression, or terminal
    /// record.
    pub fn mark_recovery_pending(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        key: IdempotencyKey,
    ) -> Result<IdempotencyRecord, IdempotencyError> {
        self.check_sequence(sequence)?;
        let record = self
            .records
            .get(&key)
            .copied()
            .ok_or(IdempotencyError::NotFound)?;
        if record.state.is_terminal() || record.state == IdempotencyState::RecoveryPending {
            return Err(IdempotencyError::InvalidTransition);
        }
        let committed = self.commit_record(
            record,
            sequence,
            timestamp_ns,
            IdempotencyState::RecoveryPending,
        )?;
        self.metrics.recovery_pending = self.metrics.recovery_pending.saturating_add(1);
        Ok(committed)
    }

    /// Re-enables the exact original command after authoritative reconciliation.
    ///
    /// The retained adapter ID is not changed, allowing FIX `ClOrdID` or a
    /// provider request token to be reused according to the adapter profile.
    ///
    /// # Errors
    ///
    /// Returns an error unless the record is recovery-pending and sequencing is
    /// monotonic.
    pub fn retry_after_reconciliation(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        key: IdempotencyKey,
    ) -> Result<IdempotencyRecord, IdempotencyError> {
        self.transition(
            sequence,
            timestamp_ns,
            key,
            &[IdempotencyState::RecoveryPending],
            IdempotencyState::Journaled,
        )
    }

    /// Explicitly retires a definitive record after the caller's retry horizon.
    ///
    /// Retirement removes duplicate protection for this key and therefore must
    /// occur only after durable archival and upstream retry expiry.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing/non-terminal record or sequence regression.
    pub fn retire_terminal(
        &mut self,
        sequence: u64,
        key: IdempotencyKey,
    ) -> Result<IdempotencyRecord, IdempotencyError> {
        self.check_sequence(sequence)?;
        let record = self
            .records
            .get(&key)
            .copied()
            .ok_or(IdempotencyError::NotFound)?;
        if !record.state.is_terminal() {
            return Err(IdempotencyError::NotTerminal);
        }
        self.records.remove(&key);
        self.command_index.remove(&record.command_id);
        self.client_index.remove(&record.command.client_order_id());
        if let Some(adapter_command_id) = record.adapter_command_id {
            self.adapter_index.remove(&adapter_command_id);
        }
        self.last_sequence = sequence;
        self.metrics.retired = self.metrics.retired.saturating_add(1);
        Ok(record)
    }

    /// Returns a retained record without mutation.
    pub fn get(&self, key: IdempotencyKey) -> Option<IdempotencyRecord> {
        self.records.get(&key).copied()
    }

    /// Returns a retained record by original OMS command ID.
    pub fn get_by_command_id(&self, command_id: CommandId) -> Option<IdempotencyRecord> {
        self.command_index
            .get(&command_id)
            .and_then(|key| self.records.get(key))
            .copied()
    }

    /// Returns configured record capacity.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns retained record count.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns true when no command records are retained.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns the latest registry mutation sequence.
    pub const fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    /// Returns cumulative registry metrics.
    pub const fn metrics(&self) -> IdempotencyMetrics {
        self.metrics
    }

    /// Captures a deterministic checksummed checkpoint.
    ///
    /// This control-plane operation allocates and sorts; it is intentionally
    /// outside command admission and adapter-send hot paths.
    pub fn checkpoint(&self) -> IdempotencyCheckpoint {
        let mut records = self.records.values().copied().collect::<Vec<_>>();
        records.sort_by(compare_idempotency_records);
        let mut checkpoint = IdempotencyCheckpoint {
            schema_version: IdempotencyCheckpoint::SCHEMA_VERSION,
            last_sequence: self.last_sequence,
            records,
            checksum: 0,
        };
        checkpoint.checksum = idempotency_checkpoint_checksum(&checkpoint);
        checkpoint
    }

    /// Restores a registry and marks every non-terminal command recovery-pending.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid schema/checksum, duplicates, capacity
    /// overflow, malformed records, or inconsistent sequence/state.
    pub fn restore(
        checkpoint: &IdempotencyCheckpoint,
        capacity: usize,
    ) -> Result<Self, IdempotencyError> {
        if capacity == 0
            || checkpoint.schema_version != IdempotencyCheckpoint::SCHEMA_VERSION
            || checkpoint.records.len() > capacity
            || checkpoint.checksum != idempotency_checkpoint_checksum(checkpoint)
        {
            return Err(IdempotencyError::InvalidCheckpoint);
        }
        let mut registry = Self::new(capacity)?;
        registry.last_sequence = checkpoint.last_sequence;
        for persisted in &checkpoint.records {
            if !valid_persisted_record(*persisted, checkpoint.last_sequence)
                || registry.records.contains_key(&persisted.key)
                || registry.command_index.contains_key(&persisted.command_id)
                || registry
                    .client_index
                    .contains_key(&persisted.command.client_order_id())
                || persisted
                    .adapter_command_id
                    .is_some_and(|id| registry.adapter_index.contains_key(&id))
            {
                return Err(IdempotencyError::InvalidCheckpoint);
            }
            let mut restored = *persisted;
            if !restored.state.is_terminal() {
                restored.state = IdempotencyState::RecoveryPending;
                registry.metrics.recovery_pending =
                    registry.metrics.recovery_pending.saturating_add(1);
            }
            registry.records.insert(restored.key, restored);
            registry
                .command_index
                .insert(restored.command_id, restored.key);
            registry
                .client_index
                .insert(restored.command.client_order_id(), restored.key);
            if let Some(adapter_command_id) = restored.adapter_command_id {
                registry
                    .adapter_index
                    .insert(adapter_command_id, restored.key);
            }
        }
        Ok(registry)
    }

    fn transition(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        key: IdempotencyKey,
        expected: &[IdempotencyState],
        next: IdempotencyState,
    ) -> Result<IdempotencyRecord, IdempotencyError> {
        self.check_sequence(sequence)?;
        let record = self
            .records
            .get(&key)
            .copied()
            .ok_or(IdempotencyError::NotFound)?;
        if !expected.contains(&record.state) {
            return Err(IdempotencyError::InvalidTransition);
        }
        self.commit_record(record, sequence, timestamp_ns, next)
    }

    fn commit_record(
        &mut self,
        mut record: IdempotencyRecord,
        sequence: u64,
        timestamp_ns: u64,
        state: IdempotencyState,
    ) -> Result<IdempotencyRecord, IdempotencyError> {
        if timestamp_ns < record.updated_ns {
            return Err(IdempotencyError::TimestampRegression);
        }
        record.state = state;
        record.updated_ns = timestamp_ns;
        record.last_sequence = sequence;
        self.records.insert(record.key, record);
        self.last_sequence = sequence;
        self.metrics.transitions = self.metrics.transitions.saturating_add(1);
        Ok(record)
    }

    fn check_sequence(&self, sequence: u64) -> Result<(), IdempotencyError> {
        if sequence == 0 || sequence <= self.last_sequence {
            return Err(IdempotencyError::SequenceRegression);
        }
        Ok(())
    }
}

/// Canonical execution-report identity scoped to one adapter/session source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecutionReportKey {
    /// Adapter, session, or drop-copy source identity.
    pub source_id: ExecutionReportSourceId,
    /// Venue execution/report ID when present.
    pub execution_id: ExecutionId,
    /// Source sequence used only when no execution ID is available.
    pub source_sequence: u64,
}

impl ExecutionReportKey {
    /// Creates a report key, preferring the provider execution ID over sequence.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionReportDedupError::MissingIdentity`] for an empty
    /// source or when both report identity forms are absent.
    pub fn new(
        source_id: ExecutionReportSourceId,
        execution_id: ExecutionId,
        source_sequence: u64,
    ) -> Result<Self, ExecutionReportDedupError> {
        if source_id.is_empty() || execution_id.is_empty() && source_sequence == 0 {
            return Err(ExecutionReportDedupError::MissingIdentity);
        }
        Ok(Self {
            source_id,
            execution_id,
            source_sequence: if execution_id.is_empty() {
                source_sequence
            } else {
                0
            },
        })
    }

    /// Creates a key from a canonical execution event.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionReportDedupError::MissingIdentity`] when the event
    /// has no execution ID and the source supplies no sequence.
    pub fn from_event(
        source_id: ExecutionReportSourceId,
        event: &ExecutionEvent,
        source_sequence: u64,
    ) -> Result<Self, ExecutionReportDedupError> {
        Self::new(source_id, event.execution_id, source_sequence)
    }

    fn is_valid(self) -> bool {
        !self.source_id.is_empty()
            && (self.source_sequence > 0 && self.execution_id.is_empty()
                || self.source_sequence == 0 && !self.execution_id.is_empty())
    }
}

/// Fresh/duplicate result from the report window.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExecutionReportDisposition {
    /// Identity was not retained and is now admitted.
    Fresh = 1,
    /// Identity was already retained and must not be applied again.
    Duplicate = 2,
}

/// Execution-report duplicate-window metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ExecutionReportDedupMetrics {
    /// Fresh report identities admitted.
    pub fresh: u64,
    /// Duplicate report identities suppressed.
    pub duplicates: u64,
    /// Oldest identities evicted as the configured window advanced.
    pub evicted: u64,
}

/// Execution-report duplicate-window error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExecutionReportDedupError {
    /// Source and report identity are insufficient.
    MissingIdentity,
    /// Duplicate window capacity must be positive.
    InvalidCapacity,
    /// Recovery checkpoint failed schema/checksum/capacity/invariant checks.
    InvalidCheckpoint,
    /// Caller-provided checkpoint output buffer is too small.
    CheckpointBufferTooSmall {
        /// Required encoded bytes.
        required: usize,
        /// Available output bytes.
        actual: usize,
    },
}

impl fmt::Display for ExecutionReportDedupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingIdentity => "execution report identity is missing",
            Self::InvalidCapacity => "execution report duplicate capacity is invalid",
            Self::InvalidCheckpoint => "execution report duplicate checkpoint is invalid",
            Self::CheckpointBufferTooSmall { .. } => {
                "execution report checkpoint output buffer is too small"
            }
        };
        f.write_str(message)
    }
}

impl Error for ExecutionReportDedupError {}

/// Checksummed oldest-to-newest duplicate-window checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionReportDedupCheckpoint {
    /// Checkpoint schema version.
    pub schema_version: u16,
    /// Retained identities in eviction order.
    pub keys: Vec<ExecutionReportKey>,
    /// Deterministic checksum over schema and identities.
    pub checksum: u64,
}

impl ExecutionReportDedupCheckpoint {
    /// Current checkpoint schema version.
    pub const SCHEMA_VERSION: u16 = 1;

    /// Returns the exact canonical binary encoding length.
    pub fn encoded_len(&self) -> usize {
        REPORT_CHECKPOINT_HEADER_LEN
            .saturating_add(
                self.keys
                    .iter()
                    .map(report_key_encoded_len)
                    .fold(0_usize, usize::saturating_add),
            )
            .saturating_add(CHECKPOINT_TRAILER_LEN)
    }

    /// Encodes the checkpoint into a caller-owned buffer without allocation.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionReportDedupError::InvalidCheckpoint`] if the object
    /// was modified without refreshing its checksum, or
    /// [`ExecutionReportDedupError::CheckpointBufferTooSmall`] when `out` is
    /// too small.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, ExecutionReportDedupError> {
        if self.schema_version != Self::SCHEMA_VERSION
            || self.checksum != report_checkpoint_checksum(self)
            || self.keys.len() > u32::MAX as usize
        {
            return Err(ExecutionReportDedupError::InvalidCheckpoint);
        }
        let required = self.encoded_len();
        if out.len() < required {
            return Err(ExecutionReportDedupError::CheckpointBufferTooSmall {
                required,
                actual: out.len(),
            });
        }
        let mut writer = BinaryWriter::new(&mut out[..required]);
        writer.bytes(&REPORT_DEDUP_CHECKPOINT_MAGIC);
        writer.u16(self.schema_version);
        writer.u16(0);
        writer.u32(self.keys.len() as u32);
        for key in &self.keys {
            writer.ascii(key.source_id);
            writer.ascii(key.execution_id);
            writer.u64(key.source_sequence);
        }
        writer.u64(self.checksum);
        Ok(writer.position())
    }

    /// Decodes and checksum-validates a canonical binary checkpoint.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionReportDedupError::InvalidCheckpoint`] for malformed
    /// magic/schema/lengths/identities, checksum mismatch, or trailing bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, ExecutionReportDedupError> {
        if bytes.len() < REPORT_CHECKPOINT_HEADER_LEN + CHECKPOINT_TRAILER_LEN {
            return Err(ExecutionReportDedupError::InvalidCheckpoint);
        }
        let mut reader = BinaryReader::new(bytes);
        let invalid = |_| ExecutionReportDedupError::InvalidCheckpoint;
        if reader.bytes(4).map_err(invalid)? != REPORT_DEDUP_CHECKPOINT_MAGIC
            || reader.u16().map_err(invalid)? != Self::SCHEMA_VERSION
            || reader.u16().map_err(invalid)? != 0
        {
            return Err(ExecutionReportDedupError::InvalidCheckpoint);
        }
        let count = reader.u32().map_err(invalid)? as usize;
        if count > bytes.len() / 16 {
            return Err(ExecutionReportDedupError::InvalidCheckpoint);
        }
        let mut keys = Vec::with_capacity(count);
        for _ in 0..count {
            let key = ExecutionReportKey {
                source_id: reader.ascii().map_err(invalid)?,
                execution_id: reader.ascii().map_err(invalid)?,
                source_sequence: reader.u64().map_err(invalid)?,
            };
            if !key.is_valid() {
                return Err(ExecutionReportDedupError::InvalidCheckpoint);
            }
            keys.push(key);
        }
        let checksum = reader.u64().map_err(invalid)?;
        if !reader.is_empty() {
            return Err(ExecutionReportDedupError::InvalidCheckpoint);
        }
        let checkpoint = Self {
            schema_version: Self::SCHEMA_VERSION,
            keys,
            checksum,
        };
        if checkpoint.checksum != report_checkpoint_checksum(&checkpoint) {
            return Err(ExecutionReportDedupError::InvalidCheckpoint);
        }
        Ok(checkpoint)
    }
}

/// Fixed-capacity FIFO duplicate window for normal, replay, and drop-copy events.
///
/// Construction reserves all map/deque storage. New identities evict the oldest
/// retained key when full; metrics make that deduplication horizon explicit.
#[derive(Debug)]
pub struct ExecutionReportDeduplicator {
    capacity: usize,
    retained: HashSet<ExecutionReportKey>,
    order: VecDeque<ExecutionReportKey>,
    metrics: ExecutionReportDedupMetrics,
}

impl ExecutionReportDeduplicator {
    /// Creates an empty fixed-capacity duplicate window.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionReportDedupError::InvalidCapacity`] for zero capacity.
    pub fn new(capacity: usize) -> Result<Self, ExecutionReportDedupError> {
        if capacity == 0 {
            return Err(ExecutionReportDedupError::InvalidCapacity);
        }
        Ok(Self {
            capacity,
            retained: HashSet::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
            metrics: ExecutionReportDedupMetrics::default(),
        })
    }

    /// Observes one report identity and suppresses exact retained duplicates.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionReportDedupError::MissingIdentity`] when a caller
    /// bypassed [`ExecutionReportKey::new`] and supplied an invalid key.
    pub fn observe(
        &mut self,
        key: ExecutionReportKey,
    ) -> Result<ExecutionReportDisposition, ExecutionReportDedupError> {
        if !key.is_valid() {
            return Err(ExecutionReportDedupError::MissingIdentity);
        }
        if self.retained.contains(&key) {
            self.metrics.duplicates = self.metrics.duplicates.saturating_add(1);
            return Ok(ExecutionReportDisposition::Duplicate);
        }
        if self.order.len() == self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.retained.remove(&oldest);
                self.metrics.evicted = self.metrics.evicted.saturating_add(1);
            }
        }
        self.order.push_back(key);
        self.retained.insert(key);
        self.metrics.fresh = self.metrics.fresh.saturating_add(1);
        Ok(ExecutionReportDisposition::Fresh)
    }

    /// Returns true when the exact identity is retained.
    pub fn contains(&self, key: ExecutionReportKey) -> bool {
        self.retained.contains(&key)
    }

    /// Returns configured retention capacity.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns retained identity count.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Returns true when no identities are retained.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Returns duplicate-window metrics.
    pub const fn metrics(&self) -> ExecutionReportDedupMetrics {
        self.metrics
    }

    /// Captures a deterministic checksummed checkpoint.
    pub fn checkpoint(&self) -> ExecutionReportDedupCheckpoint {
        let keys = self.order.iter().copied().collect::<Vec<_>>();
        let mut checkpoint = ExecutionReportDedupCheckpoint {
            schema_version: ExecutionReportDedupCheckpoint::SCHEMA_VERSION,
            keys,
            checksum: 0,
        };
        checkpoint.checksum = report_checkpoint_checksum(&checkpoint);
        checkpoint
    }

    /// Restores the exact oldest-to-newest duplicate horizon.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid schema/checksum, duplicates, malformed
    /// identities, or capacity overflow.
    pub fn restore(
        checkpoint: &ExecutionReportDedupCheckpoint,
        capacity: usize,
    ) -> Result<Self, ExecutionReportDedupError> {
        if capacity == 0
            || checkpoint.schema_version != ExecutionReportDedupCheckpoint::SCHEMA_VERSION
            || checkpoint.keys.len() > capacity
            || checkpoint.checksum != report_checkpoint_checksum(checkpoint)
        {
            return Err(ExecutionReportDedupError::InvalidCheckpoint);
        }
        let mut restored = Self::new(capacity)?;
        for key in &checkpoint.keys {
            if !key.is_valid() || !restored.retained.insert(*key) {
                return Err(ExecutionReportDedupError::InvalidCheckpoint);
            }
            restored.order.push_back(*key);
        }
        Ok(restored)
    }
}

struct BinaryWriter<'a> {
    out: &'a mut [u8],
    position: usize,
}

impl<'a> BinaryWriter<'a> {
    fn new(out: &'a mut [u8]) -> Self {
        Self { out, position: 0 }
    }

    fn position(&self) -> usize {
        self.position
    }

    fn bytes(&mut self, value: &[u8]) {
        let end = self.position + value.len();
        self.out[self.position..end].copy_from_slice(value);
        self.position = end;
    }

    fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }

    fn u16(&mut self, value: u16) {
        self.bytes(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes(&value.to_le_bytes());
    }

    fn ascii<const N: usize>(&mut self, value: FixedAscii<N>) {
        self.u8(value.as_str().len() as u8);
        self.bytes(value.as_str().as_bytes());
    }
}

struct BinaryReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> BinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn is_empty(&self) -> bool {
        self.position == self.bytes.len()
    }

    fn bytes(&mut self, len: usize) -> Result<&'a [u8], IdempotencyError> {
        let end = self
            .position
            .checked_add(len)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(IdempotencyError::InvalidCheckpoint)?;
        let value = &self.bytes[self.position..end];
        self.position = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, IdempotencyError> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, IdempotencyError> {
        let bytes = self.bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, IdempotencyError> {
        let bytes = self.bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, IdempotencyError> {
        let bytes = self.bytes(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn i64(&mut self) -> Result<i64, IdempotencyError> {
        let bytes = self.bytes(8)?;
        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn ascii<const N: usize>(&mut self) -> Result<FixedAscii<N>, IdempotencyError> {
        let len = self.u8()? as usize;
        if len > N {
            return Err(IdempotencyError::InvalidCheckpoint);
        }
        let value = std::str::from_utf8(self.bytes(len)?)
            .map_err(|_| IdempotencyError::InvalidCheckpoint)?;
        FixedAscii::new(value).map_err(|_| IdempotencyError::InvalidCheckpoint)
    }
}

fn ascii_encoded_len<const N: usize>(value: FixedAscii<N>) -> usize {
    1 + value.as_str().len()
}

fn idempotency_record_encoded_len(record: &IdempotencyRecord) -> usize {
    ascii_encoded_len(record.key.scope_id)
        + ascii_encoded_len(record.key.request_id.0)
        + 8
        + 1
        + command_payload_encoded_len(record.command)
        + 1
        + 1
        + record.adapter_command_id.map_or(0, ascii_encoded_len)
        + 4
        + 8
        + 8
        + 8
}

fn command_payload_encoded_len(command: IdempotentExecutionCommand) -> usize {
    match command {
        IdempotentExecutionCommand::Submit(request) => {
            ascii_encoded_len(request.client_order_id)
                + ascii_encoded_len(request.account_id)
                + ascii_encoded_len(request.route_id)
                + ascii_encoded_len(request.strategy_id)
                + ascii_encoded_len(request.symbol.venue)
                + ascii_encoded_len(request.symbol.instrument)
                + 3
                + 5 * 8
        }
        IdempotentExecutionCommand::Cancel(request) => {
            ascii_encoded_len(request.client_order_id)
                + ascii_encoded_len(request.orig_client_order_id)
                + ascii_encoded_len(request.venue_order_id)
                + ascii_encoded_len(request.account_id)
                + ascii_encoded_len(request.route_id)
                + ascii_encoded_len(request.symbol.venue)
                + ascii_encoded_len(request.symbol.instrument)
                + 8
        }
        IdempotentExecutionCommand::Amend(request) => {
            ascii_encoded_len(request.client_order_id)
                + ascii_encoded_len(request.orig_client_order_id)
                + ascii_encoded_len(request.venue_order_id)
                + ascii_encoded_len(request.account_id)
                + ascii_encoded_len(request.route_id)
                + ascii_encoded_len(request.symbol.venue)
                + ascii_encoded_len(request.symbol.instrument)
                + 3 * 8
        }
    }
}

fn encode_idempotency_record(writer: &mut BinaryWriter<'_>, record: IdempotencyRecord) {
    writer.ascii(record.key.scope_id);
    writer.ascii(record.key.request_id.0);
    writer.u64(record.command_id.0);
    match record.command {
        IdempotentExecutionCommand::Submit(request) => {
            writer.u8(1);
            writer.ascii(request.client_order_id);
            writer.ascii(request.account_id);
            writer.ascii(request.route_id);
            writer.ascii(request.strategy_id);
            writer.ascii(request.symbol.venue);
            writer.ascii(request.symbol.instrument);
            writer.u8(request.side as u8);
            writer.u8(request.order_type as u8);
            writer.u8(request.time_in_force as u8);
            writer.i64(request.quantity.0);
            writer.i64(request.limit_price.0);
            writer.i64(request.stop_price.0);
            writer.u64(request.ts_exchange_ns);
            writer.u64(request.ts_recv_ns);
        }
        IdempotentExecutionCommand::Cancel(request) => {
            writer.u8(2);
            writer.ascii(request.client_order_id);
            writer.ascii(request.orig_client_order_id);
            writer.ascii(request.venue_order_id);
            writer.ascii(request.account_id);
            writer.ascii(request.route_id);
            writer.ascii(request.symbol.venue);
            writer.ascii(request.symbol.instrument);
            writer.u64(request.ts_recv_ns);
        }
        IdempotentExecutionCommand::Amend(request) => {
            writer.u8(3);
            writer.ascii(request.client_order_id);
            writer.ascii(request.orig_client_order_id);
            writer.ascii(request.venue_order_id);
            writer.ascii(request.account_id);
            writer.ascii(request.route_id);
            writer.ascii(request.symbol.venue);
            writer.ascii(request.symbol.instrument);
            writer.i64(request.quantity.0);
            writer.i64(request.limit_price.0);
            writer.u64(request.ts_recv_ns);
        }
    }
    writer.u8(record.state as u8);
    match record.adapter_command_id {
        Some(adapter_id) => {
            writer.u8(1);
            writer.ascii(adapter_id);
        }
        None => writer.u8(0),
    }
    writer.u32(record.send_attempts);
    writer.u64(record.created_ns);
    writer.u64(record.updated_ns);
    writer.u64(record.last_sequence);
}

fn decode_idempotency_record(
    reader: &mut BinaryReader<'_>,
) -> Result<IdempotencyRecord, IdempotencyError> {
    let key = IdempotencyKey {
        scope_id: reader.ascii()?,
        request_id: RequestId(reader.ascii()?),
    };
    let command_id = CommandId(reader.u64()?);
    let command = decode_command(reader)?;
    let state = decode_idempotency_state(reader.u8()?)?;
    let adapter_command_id = match reader.u8()? {
        0 => None,
        1 => Some(reader.ascii()?),
        _ => return Err(IdempotencyError::InvalidCheckpoint),
    };
    Ok(IdempotencyRecord {
        key,
        command_id,
        command,
        state,
        adapter_command_id,
        send_attempts: reader.u32()?,
        created_ns: reader.u64()?,
        updated_ns: reader.u64()?,
        last_sequence: reader.u64()?,
    })
}

fn decode_command(
    reader: &mut BinaryReader<'_>,
) -> Result<IdempotentExecutionCommand, IdempotencyError> {
    match reader.u8()? {
        1 => Ok(IdempotentExecutionCommand::Submit(OrderRequest {
            client_order_id: reader.ascii()?,
            account_id: reader.ascii()?,
            route_id: reader.ascii()?,
            strategy_id: reader.ascii()?,
            symbol: ExecutionSymbol {
                venue: reader.ascii()?,
                instrument: reader.ascii()?,
            },
            side: decode_order_side(reader.u8()?)?,
            order_type: decode_order_type(reader.u8()?)?,
            time_in_force: decode_time_in_force(reader.u8()?)?,
            quantity: OrderQty(reader.i64()?),
            limit_price: OrderPrice(reader.i64()?),
            stop_price: OrderPrice(reader.i64()?),
            ts_exchange_ns: reader.u64()?,
            ts_recv_ns: reader.u64()?,
        })),
        2 => Ok(IdempotentExecutionCommand::Cancel(CancelRequest {
            client_order_id: reader.ascii()?,
            orig_client_order_id: reader.ascii()?,
            venue_order_id: reader.ascii()?,
            account_id: reader.ascii()?,
            route_id: reader.ascii()?,
            symbol: ExecutionSymbol {
                venue: reader.ascii()?,
                instrument: reader.ascii()?,
            },
            ts_recv_ns: reader.u64()?,
        })),
        3 => Ok(IdempotentExecutionCommand::Amend(AmendRequest {
            client_order_id: reader.ascii()?,
            orig_client_order_id: reader.ascii()?,
            venue_order_id: reader.ascii()?,
            account_id: reader.ascii()?,
            route_id: reader.ascii()?,
            symbol: ExecutionSymbol {
                venue: reader.ascii()?,
                instrument: reader.ascii()?,
            },
            quantity: OrderQty(reader.i64()?),
            limit_price: OrderPrice(reader.i64()?),
            ts_recv_ns: reader.u64()?,
        })),
        _ => Err(IdempotencyError::InvalidCheckpoint),
    }
}

fn decode_order_side(value: u8) -> Result<OrderSide, IdempotencyError> {
    match value {
        1 => Ok(OrderSide::Buy),
        2 => Ok(OrderSide::Sell),
        _ => Err(IdempotencyError::InvalidCheckpoint),
    }
}

fn decode_order_type(value: u8) -> Result<OrderType, IdempotencyError> {
    match value {
        1 => Ok(OrderType::Market),
        2 => Ok(OrderType::Limit),
        3 => Ok(OrderType::Stop),
        4 => Ok(OrderType::StopLimit),
        _ => Err(IdempotencyError::InvalidCheckpoint),
    }
}

fn decode_time_in_force(value: u8) -> Result<TimeInForce, IdempotencyError> {
    match value {
        1 => Ok(TimeInForce::Day),
        2 => Ok(TimeInForce::Gtc),
        3 => Ok(TimeInForce::Ioc),
        4 => Ok(TimeInForce::Fok),
        5 => Ok(TimeInForce::Gtd),
        _ => Err(IdempotencyError::InvalidCheckpoint),
    }
}

fn decode_idempotency_state(value: u8) -> Result<IdempotencyState, IdempotencyError> {
    match value {
        1 => Ok(IdempotencyState::Reserved),
        2 => Ok(IdempotencyState::Journaled),
        3 => Ok(IdempotencyState::Sent),
        4 => Ok(IdempotencyState::Acknowledged),
        5 => Ok(IdempotencyState::Rejected),
        6 => Ok(IdempotencyState::FailedDefinitive),
        7 => Ok(IdempotencyState::RecoveryPending),
        _ => Err(IdempotencyError::InvalidCheckpoint),
    }
}

fn report_key_encoded_len(key: &ExecutionReportKey) -> usize {
    ascii_encoded_len(key.source_id) + ascii_encoded_len(key.execution_id) + 8
}

fn submit_matches(left: OrderRequest, right: OrderRequest) -> bool {
    left.client_order_id == right.client_order_id
        && left.account_id == right.account_id
        && left.route_id == right.route_id
        && left.strategy_id == right.strategy_id
        && left.symbol == right.symbol
        && left.side == right.side
        && left.order_type == right.order_type
        && left.time_in_force == right.time_in_force
        && left.quantity == right.quantity
        && left.limit_price == right.limit_price
        && left.stop_price == right.stop_price
}

fn cancel_matches(left: CancelRequest, right: CancelRequest) -> bool {
    left.client_order_id == right.client_order_id
        && left.orig_client_order_id == right.orig_client_order_id
        && left.venue_order_id == right.venue_order_id
        && left.account_id == right.account_id
        && left.route_id == right.route_id
        && left.symbol == right.symbol
}

fn amend_matches(left: AmendRequest, right: AmendRequest) -> bool {
    left.client_order_id == right.client_order_id
        && left.orig_client_order_id == right.orig_client_order_id
        && left.venue_order_id == right.venue_order_id
        && left.account_id == right.account_id
        && left.route_id == right.route_id
        && left.symbol == right.symbol
        && left.quantity == right.quantity
        && left.limit_price == right.limit_price
}

fn valid_persisted_record(record: IdempotencyRecord, checkpoint_sequence: u64) -> bool {
    record.key.is_valid()
        && record.command_id.0 > 0
        && record.command.is_valid()
        && record.last_sequence > 0
        && record.last_sequence <= checkpoint_sequence
        && record.created_ns <= record.updated_ns
        && match record.state {
            IdempotencyState::Reserved | IdempotencyState::Journaled => {
                record.send_attempts == 0 && record.adapter_command_id.is_none()
            }
            IdempotencyState::Sent => {
                record.send_attempts > 0 && record.adapter_command_id.is_some()
            }
            IdempotencyState::Acknowledged | IdempotencyState::RecoveryPending => {
                record.send_attempts == 0 && record.adapter_command_id.is_none()
                    || record.send_attempts > 0 && record.adapter_command_id.is_some()
            }
            IdempotencyState::Rejected | IdempotencyState::FailedDefinitive => {
                record.send_attempts == 0 && record.adapter_command_id.is_none()
                    || record.send_attempts > 0 && record.adapter_command_id.is_some()
            }
        }
}

fn compare_idempotency_records(
    left: &IdempotencyRecord,
    right: &IdempotencyRecord,
) -> std::cmp::Ordering {
    left.key
        .scope_id
        .as_str()
        .cmp(right.key.scope_id.as_str())
        .then_with(|| {
            left.key
                .request_id
                .as_str()
                .cmp(right.key.request_id.as_str())
        })
}

const CHECKSUM_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const CHECKSUM_PRIME: u64 = 0x0000_0100_0000_01b3;

struct Checksum(u64);

impl Checksum {
    fn new() -> Self {
        Self(CHECKSUM_OFFSET)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(CHECKSUM_PRIME);
        }
    }

    fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }

    fn u16(&mut self, value: u16) {
        self.bytes(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes(&value.to_le_bytes());
    }

    fn ascii<const N: usize>(&mut self, value: FixedAscii<N>) {
        self.u64(value.as_str().len() as u64);
        self.bytes(value.as_str().as_bytes());
    }

    fn finish(self) -> u64 {
        self.0
    }
}

fn idempotency_checkpoint_checksum(checkpoint: &IdempotencyCheckpoint) -> u64 {
    let mut checksum = Checksum::new();
    checksum.u16(checkpoint.schema_version);
    checksum.u64(checkpoint.last_sequence);
    checksum.u64(checkpoint.records.len() as u64);
    for record in &checkpoint.records {
        checksum.ascii(record.key.scope_id);
        checksum.ascii(record.key.request_id.0);
        checksum.u64(record.command_id.0);
        checksum_command(&mut checksum, record.command);
        checksum.u8(record.state as u8);
        match record.adapter_command_id {
            Some(id) => {
                checksum.u8(1);
                checksum.ascii(id);
            }
            None => checksum.u8(0),
        }
        checksum.u32(record.send_attempts);
        checksum.u64(record.created_ns);
        checksum.u64(record.updated_ns);
        checksum.u64(record.last_sequence);
    }
    checksum.finish()
}

fn checksum_command(checksum: &mut Checksum, command: IdempotentExecutionCommand) {
    checksum.u8(match command {
        IdempotentExecutionCommand::Submit(_) => 1,
        IdempotentExecutionCommand::Cancel(_) => 2,
        IdempotentExecutionCommand::Amend(_) => 3,
    });
    match command {
        IdempotentExecutionCommand::Submit(request) => {
            checksum.ascii(request.client_order_id);
            checksum.ascii(request.account_id);
            checksum.ascii(request.route_id);
            checksum.ascii(request.strategy_id);
            checksum.ascii(request.symbol.venue);
            checksum.ascii(request.symbol.instrument);
            checksum.u8(request.side as u8);
            checksum.u8(request.order_type as u8);
            checksum.u8(request.time_in_force as u8);
            checksum.i64(request.quantity.0);
            checksum.i64(request.limit_price.0);
            checksum.i64(request.stop_price.0);
            checksum.u64(request.ts_exchange_ns);
            checksum.u64(request.ts_recv_ns);
        }
        IdempotentExecutionCommand::Cancel(request) => {
            checksum.ascii(request.client_order_id);
            checksum.ascii(request.orig_client_order_id);
            checksum.ascii(request.venue_order_id);
            checksum.ascii(request.account_id);
            checksum.ascii(request.route_id);
            checksum.ascii(request.symbol.venue);
            checksum.ascii(request.symbol.instrument);
            checksum.u64(request.ts_recv_ns);
        }
        IdempotentExecutionCommand::Amend(request) => {
            checksum.ascii(request.client_order_id);
            checksum.ascii(request.orig_client_order_id);
            checksum.ascii(request.venue_order_id);
            checksum.ascii(request.account_id);
            checksum.ascii(request.route_id);
            checksum.ascii(request.symbol.venue);
            checksum.ascii(request.symbol.instrument);
            checksum.i64(request.quantity.0);
            checksum.i64(request.limit_price.0);
            checksum.u64(request.ts_recv_ns);
        }
    }
}

fn report_checkpoint_checksum(checkpoint: &ExecutionReportDedupCheckpoint) -> u64 {
    let mut checksum = Checksum::new();
    checksum.u16(checkpoint.schema_version);
    checksum.u64(checkpoint.keys.len() as u64);
    for key in &checkpoint.keys {
        checksum.ascii(key.source_id);
        checksum.ascii(key.execution_id);
        checksum.u64(key.source_sequence);
    }
    checksum.finish()
}

#[cfg(test)]
mod tests {
    use of_execution_core::{
        AccountId, ExecutionSymbol, OrderPrice, OrderQty, OrderSide, OrderType, RouteId,
        StrategyId, TimeInForce,
    };

    use super::*;

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn key(value: &str) -> IdempotencyKey {
        IdempotencyKey::new(id("gateway-a"), RequestId::new(value).unwrap()).unwrap()
    }

    fn submit(client: &str, quantity: i64, timestamp: u64) -> IdempotentExecutionCommand {
        IdempotentExecutionCommand::Submit(OrderRequest {
            client_order_id: id(client),
            account_id: AccountId::new("account-a").unwrap(),
            route_id: RouteId::new("route-a").unwrap(),
            strategy_id: StrategyId::new("strategy-a").unwrap(),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(quantity),
            limit_price: OrderPrice(100),
            stop_price: OrderPrice(0),
            ts_exchange_ns: timestamp.saturating_sub(1),
            ts_recv_ns: timestamp,
        })
    }

    fn cancel(client: &str, timestamp: u64) -> IdempotentExecutionCommand {
        IdempotentExecutionCommand::Cancel(CancelRequest {
            client_order_id: id(client),
            orig_client_order_id: id("original"),
            venue_order_id: id("venue-1"),
            account_id: id("account-a"),
            route_id: id("route-a"),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            ts_recv_ns: timestamp,
        })
    }

    fn amend(client: &str, timestamp: u64) -> IdempotentExecutionCommand {
        IdempotentExecutionCommand::Amend(AmendRequest {
            client_order_id: id(client),
            orig_client_order_id: id("original"),
            venue_order_id: id("venue-1"),
            account_id: id("account-a"),
            route_id: id("route-a"),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            quantity: OrderQty(3),
            limit_price: OrderPrice(101),
            ts_recv_ns: timestamp,
        })
    }

    #[test]
    fn matching_retry_returns_original_without_consuming_sequence() {
        let mut registry = IdempotencyRegistry::new(2).unwrap();
        let first = registry
            .reserve(1, 100, key("request-1"), CommandId(9), submit("c1", 2, 100))
            .unwrap();
        assert!(matches!(first, IdempotencyDecision::Accepted(_)));
        let retry = registry
            .reserve(
                1,
                200,
                key("request-1"),
                CommandId(10),
                submit("c1", 2, 200),
            )
            .unwrap();
        assert!(retry.is_duplicate());
        assert_eq!(retry.record().command_id, CommandId(9));
        assert_eq!(registry.last_sequence(), 1);
        assert_eq!(registry.metrics().duplicates, 1);
    }

    #[test]
    fn parameter_mismatch_and_command_collision_fail_closed() {
        let mut registry = IdempotencyRegistry::new(2).unwrap();
        registry
            .reserve(1, 100, key("request-1"), CommandId(9), submit("c1", 2, 100))
            .unwrap();
        assert_eq!(
            registry.reserve(
                2,
                101,
                key("request-1"),
                CommandId(10),
                submit("c1", 3, 101)
            ),
            Err(IdempotencyError::ParameterMismatch)
        );
        assert_eq!(
            registry.reserve(2, 101, key("request-2"), CommandId(9), submit("c2", 2, 101)),
            Err(IdempotencyError::CommandIdCollision)
        );
        assert_eq!(
            registry.reserve(
                2,
                101,
                key("request-2"),
                CommandId(10),
                submit("c1", 2, 101)
            ),
            Err(IdempotencyError::ClientOrderIdCollision)
        );
        assert_eq!(registry.last_sequence(), 1);
    }

    #[test]
    fn provider_identity_cannot_be_shared_by_distinct_commands() {
        let mut registry = IdempotencyRegistry::new(2).unwrap();
        let first = key("request-1");
        let second = key("request-2");
        registry
            .reserve(1, 100, first, CommandId(9), submit("c1", 2, 100))
            .unwrap();
        registry.mark_journaled(2, 101, first).unwrap();
        registry.mark_sent(3, 102, first, id("FIX-SHARED")).unwrap();
        registry
            .reserve(4, 103, second, CommandId(10), submit("c2", 2, 103))
            .unwrap();
        registry.mark_journaled(5, 104, second).unwrap();
        assert_eq!(
            registry.mark_sent(6, 105, second, id("FIX-SHARED")),
            Err(IdempotencyError::AdapterIdCollision)
        );
        assert_eq!(
            registry.get(second).unwrap().state,
            IdempotencyState::Journaled
        );
        assert_eq!(registry.last_sequence(), 5);
    }

    #[test]
    fn lifecycle_preserves_adapter_id_across_reconciled_retry() {
        let mut registry = IdempotencyRegistry::new(2).unwrap();
        let key = key("request-1");
        registry
            .reserve(1, 100, key, CommandId(9), submit("c1", 2, 100))
            .unwrap();
        registry.mark_journaled(2, 101, key).unwrap();
        registry.mark_sent(3, 102, key, id("FIX-C1")).unwrap();
        registry.mark_recovery_pending(4, 103, key).unwrap();
        registry.retry_after_reconciliation(5, 104, key).unwrap();
        assert_eq!(
            registry.mark_sent(6, 105, key, id("FIX-C2")),
            Err(IdempotencyError::AdapterIdMismatch)
        );
        let sent = registry.mark_sent(6, 105, key, id("FIX-C1")).unwrap();
        assert_eq!(sent.send_attempts, 2);
        let done = registry
            .complete(7, 106, key, IdempotencyCompletion::Acknowledged)
            .unwrap();
        assert_eq!(done.state, IdempotencyState::Acknowledged);
    }

    #[test]
    fn invalid_transition_and_timestamp_regression_are_atomic() {
        let mut registry = IdempotencyRegistry::new(1).unwrap();
        let key = key("request-1");
        registry
            .reserve(1, 100, key, CommandId(9), submit("c1", 2, 100))
            .unwrap();
        assert_eq!(
            registry.mark_sent(2, 101, key, id("FIX-C1")),
            Err(IdempotencyError::InvalidTransition)
        );
        assert_eq!(
            registry.mark_journaled(2, 99, key),
            Err(IdempotencyError::TimestampRegression)
        );
        let retained = registry.get(key).unwrap();
        assert_eq!(retained.state, IdempotencyState::Reserved);
        assert_eq!(retained.last_sequence, 1);
        assert_eq!(registry.last_sequence(), 1);
    }

    #[test]
    fn capacity_never_evicts_live_or_terminal_command_keys() {
        let mut registry = IdempotencyRegistry::new(1).unwrap();
        let first = key("request-1");
        registry
            .reserve(1, 100, first, CommandId(9), submit("c1", 2, 100))
            .unwrap();
        assert_eq!(
            registry.reserve(
                2,
                101,
                key("request-2"),
                CommandId(10),
                submit("c2", 2, 101)
            ),
            Err(IdempotencyError::CapacityExceeded)
        );
        assert!(registry.get(first).is_some());
        assert_eq!(registry.last_sequence(), 1);
    }

    #[test]
    fn terminal_retirement_is_explicit_and_sequence_checked() {
        let mut registry = IdempotencyRegistry::new(1).unwrap();
        let first = key("request-1");
        registry
            .reserve(1, 100, first, CommandId(9), submit("c1", 2, 100))
            .unwrap();
        assert_eq!(
            registry.retire_terminal(2, first),
            Err(IdempotencyError::NotTerminal)
        );
        registry
            .complete(2, 101, first, IdempotencyCompletion::Rejected)
            .unwrap();
        registry.retire_terminal(3, first).unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn checkpoint_restore_verifies_integrity_and_requires_reconciliation() {
        let mut registry = IdempotencyRegistry::new(2).unwrap();
        let first = key("request-1");
        let second = key("request-2");
        registry
            .reserve(1, 100, first, CommandId(9), submit("c1", 2, 100))
            .unwrap();
        registry.mark_journaled(2, 101, first).unwrap();
        registry.mark_sent(3, 102, first, id("FIX-C1")).unwrap();
        registry
            .reserve(4, 103, second, CommandId(10), submit("c2", 2, 103))
            .unwrap();
        registry
            .complete(5, 104, second, IdempotencyCompletion::Rejected)
            .unwrap();
        let checkpoint = registry.checkpoint();
        let restored = IdempotencyRegistry::restore(&checkpoint, 2).unwrap();
        assert_eq!(
            restored.get(first).unwrap().state,
            IdempotencyState::RecoveryPending
        );
        assert_eq!(
            restored.get(second).unwrap().state,
            IdempotencyState::Rejected
        );

        let mut duplicate_client = checkpoint.clone();
        duplicate_client.records[1].command = submit("c1", 2, 103);
        duplicate_client.checksum = idempotency_checkpoint_checksum(&duplicate_client);
        assert_eq!(
            IdempotencyRegistry::restore(&duplicate_client, 2).unwrap_err(),
            IdempotencyError::InvalidCheckpoint
        );

        let mut corrupt = checkpoint;
        corrupt.records[0].command_id = CommandId(99);
        assert_eq!(
            IdempotencyRegistry::restore(&corrupt, 2).unwrap_err(),
            IdempotencyError::InvalidCheckpoint
        );
    }

    #[test]
    fn command_checkpoint_binary_codec_round_trips_every_command_kind() {
        let mut registry = IdempotencyRegistry::new(3).unwrap();
        registry
            .reserve(1, 100, key("submit"), CommandId(1), submit("new-1", 2, 100))
            .unwrap();
        registry.mark_journaled(2, 101, key("submit")).unwrap();
        registry
            .mark_sent(3, 102, key("submit"), id("provider-new-1"))
            .unwrap();
        registry
            .reserve(4, 103, key("cancel"), CommandId(2), cancel("cancel-1", 103))
            .unwrap();
        registry
            .complete(5, 104, key("cancel"), IdempotencyCompletion::Rejected)
            .unwrap();
        registry
            .reserve(6, 105, key("amend"), CommandId(3), amend("amend-1", 105))
            .unwrap();

        let checkpoint = registry.checkpoint();
        let mut bytes = vec![0_u8; checkpoint.encoded_len()];
        assert_eq!(checkpoint.encode_into(&mut bytes).unwrap(), bytes.len());
        let decoded = IdempotencyCheckpoint::decode(&bytes).unwrap();
        assert_eq!(decoded, checkpoint);
        let restored = IdempotencyRegistry::restore(&decoded, 3).unwrap();
        assert_eq!(restored.len(), 3);

        let mut short = vec![0_u8; checkpoint.encoded_len() - 1];
        assert!(matches!(
            checkpoint.encode_into(&mut short),
            Err(IdempotencyError::CheckpointBufferTooSmall { .. })
        ));
        let corrupt_index = bytes.len() - 9;
        bytes[corrupt_index] ^= 0x01;
        assert_eq!(
            IdempotencyCheckpoint::decode(&bytes),
            Err(IdempotencyError::InvalidCheckpoint)
        );
    }

    #[test]
    fn duplicate_window_prefers_execution_id_and_evicts_oldest() {
        let source = id("fix-drop-copy-a");
        let first = ExecutionReportKey::new(source, id("exec-1"), 99).unwrap();
        let same_execution = ExecutionReportKey::new(source, id("exec-1"), 100).unwrap();
        let second = ExecutionReportKey::new(source, id("exec-2"), 0).unwrap();
        let third = ExecutionReportKey::new(source, id("exec-3"), 0).unwrap();
        let mut dedup = ExecutionReportDeduplicator::new(2).unwrap();
        assert_eq!(
            dedup.observe(first).unwrap(),
            ExecutionReportDisposition::Fresh
        );
        assert_eq!(
            dedup.observe(same_execution).unwrap(),
            ExecutionReportDisposition::Duplicate
        );
        assert_eq!(
            dedup.observe(second).unwrap(),
            ExecutionReportDisposition::Fresh
        );
        assert_eq!(
            dedup.observe(third).unwrap(),
            ExecutionReportDisposition::Fresh
        );
        assert!(!dedup.contains(first));
        assert_eq!(dedup.metrics().evicted, 1);
    }

    #[test]
    fn duplicate_window_checkpoint_preserves_eviction_order() {
        let source = id("fix-a");
        let first = ExecutionReportKey::new(source, ExecutionId::empty(), 1).unwrap();
        let second = ExecutionReportKey::new(source, ExecutionId::empty(), 2).unwrap();
        let mut dedup = ExecutionReportDeduplicator::new(2).unwrap();
        dedup.observe(first).unwrap();
        dedup.observe(second).unwrap();
        let checkpoint = dedup.checkpoint();
        let mut restored = ExecutionReportDeduplicator::restore(&checkpoint, 2).unwrap();
        assert_eq!(
            restored.observe(first).unwrap(),
            ExecutionReportDisposition::Duplicate
        );

        let mut corrupt = checkpoint;
        corrupt.keys.push(first);
        corrupt.checksum = report_checkpoint_checksum(&corrupt);
        assert_eq!(
            ExecutionReportDeduplicator::restore(&corrupt, 3).unwrap_err(),
            ExecutionReportDedupError::InvalidCheckpoint
        );
    }

    #[test]
    fn duplicate_window_checkpoint_binary_codec_is_exact() {
        let source = id("fix-a");
        let mut dedup = ExecutionReportDeduplicator::new(2).unwrap();
        dedup
            .observe(ExecutionReportKey::new(source, id("exec-1"), 9).unwrap())
            .unwrap();
        dedup
            .observe(ExecutionReportKey::new(source, ExecutionId::empty(), 10).unwrap())
            .unwrap();
        let checkpoint = dedup.checkpoint();
        let mut bytes = vec![0_u8; checkpoint.encoded_len()];
        assert_eq!(checkpoint.encode_into(&mut bytes).unwrap(), bytes.len());
        let decoded = ExecutionReportDedupCheckpoint::decode(&bytes).unwrap();
        assert_eq!(decoded, checkpoint);
        assert!(ExecutionReportDeduplicator::restore(&decoded, 2).is_ok());

        let mut short = vec![0_u8; checkpoint.encoded_len() - 1];
        assert!(matches!(
            checkpoint.encode_into(&mut short),
            Err(ExecutionReportDedupError::CheckpointBufferTooSmall { .. })
        ));
        bytes.push(0);
        assert_eq!(
            ExecutionReportDedupCheckpoint::decode(&bytes),
            Err(ExecutionReportDedupError::InvalidCheckpoint)
        );
    }

    #[test]
    fn duplicate_window_rejects_bypassed_invalid_key() {
        let mut dedup = ExecutionReportDeduplicator::new(2).unwrap();
        let invalid = ExecutionReportKey {
            source_id: ExecutionReportSourceId::empty(),
            execution_id: ExecutionId::empty(),
            source_sequence: 0,
        };
        assert_eq!(
            dedup.observe(invalid),
            Err(ExecutionReportDedupError::MissingIdentity)
        );
        assert!(dedup.is_empty());
    }
}
