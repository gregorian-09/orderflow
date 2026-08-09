//! Scoped, auditable, fail-closed kill-switch primitives.

use std::error::Error;
use std::fmt;

use of_execution_core::{
    AccountId, ClientOrderId, ExecutionCoreError, ExecutionSymbol, FixedAscii, OrderQty,
    OrderRequest, OrderSide, OrderState, OrderType, RouteId, StrategyId, VenueId, VenueOrderId,
    WalSequence,
};

/// Maximum bytes stored in an adapter-session identifier.
pub const KILL_SWITCH_SESSION_ID_CAPACITY: usize = 32;
/// Maximum bytes stored in a kill-switch actor identifier.
pub const KILL_SWITCH_ACTOR_ID_CAPACITY: usize = 32;

/// Adapter or protocol-session identity used by session-scoped switches.
pub type KillSwitchSessionId = FixedAscii<KILL_SWITCH_SESSION_ID_CAPACITY>;
/// Human or system identity recorded with kill-switch operations.
pub type KillSwitchActorId = FixedAscii<KILL_SWITCH_ACTOR_ID_CAPACITY>;

/// Stable identifier for one kill-switch activation lifecycle.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KillSwitchId(u64);

impl KillSwitchId {
    /// Creates a nonzero kill-switch id.
    ///
    /// # Errors
    ///
    /// Returns [`KillSwitchError::InvalidSwitchId`] when `value` is zero.
    pub const fn new(value: u64) -> Result<Self, KillSwitchError> {
        if value == 0 {
            Err(KillSwitchError::InvalidSwitchId)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the numeric identifier.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Source category responsible for a kill-switch operation.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KillSwitchSourceKind {
    /// Authenticated human operator.
    Operator = 0,
    /// Automated risk policy.
    RiskSystem = 1,
    /// OMS or adapter supervisor.
    Supervisor = 2,
    /// Venue or broker control plane.
    Venue = 3,
    /// Recovery or reconciliation workflow.
    Recovery = 4,
}

/// Actor responsible for activating, updating, or clearing a switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KillSwitchSource {
    kind: KillSwitchSourceKind,
    actor_id: KillSwitchActorId,
}

impl KillSwitchSource {
    /// Creates a source from a typed category and fixed-size actor id.
    pub const fn new(kind: KillSwitchSourceKind, actor_id: KillSwitchActorId) -> Self {
        Self { kind, actor_id }
    }

    /// Creates a source from an ASCII actor id.
    ///
    /// # Errors
    ///
    /// Returns an identifier error when the id is non-ASCII or too long.
    pub fn from_id(kind: KillSwitchSourceKind, actor_id: &str) -> Result<Self, ExecutionCoreError> {
        Ok(Self::new(kind, KillSwitchActorId::new(actor_id)?))
    }

    /// Returns the source category.
    pub const fn kind(self) -> KillSwitchSourceKind {
        self.kind
    }

    /// Returns the actor identifier.
    pub const fn actor_id(self) -> KillSwitchActorId {
        self.actor_id
    }
}

/// Scope selected by a kill switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KillSwitchScope {
    /// Every route and order managed by the host.
    Global,
    /// One venue or exchange.
    Venue(VenueId),
    /// One execution route.
    Route(RouteId),
    /// One trading account.
    Account(AccountId),
    /// One strategy.
    Strategy(StrategyId),
    /// One venue-native symbol.
    Symbol(ExecutionSymbol),
    /// One canonical order type.
    OrderType(OrderType),
    /// One adapter or protocol session.
    AdapterSession(KillSwitchSessionId),
}

/// Operational behavior selected by an active switch.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KillSwitchMode {
    /// Reject matching new orders while preserving cancel flow.
    RejectNew = 0,
    /// Reject all new orders and cancel every supplied open order.
    CancelAll = 1,
    /// Reject matching new orders and cancel matching open orders.
    CancelScope = 2,
    /// Permit only orders that strictly reduce the supplied position.
    ReduceOnly = 3,
    /// Pause matching strategy submissions while preserving cancel flow.
    PauseStrategy = 4,
    /// Stop matching adapter flow after cancellation is attempted.
    HardStopAdapter = 5,
}

impl KillSwitchMode {
    const fn requires_cancellation(self) -> bool {
        matches!(
            self,
            Self::CancelAll | Self::CancelScope | Self::HardStopAdapter
        )
    }
}

/// Structured reason for a kill-switch operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KillSwitchReasonCode {
    /// Manual operator intervention.
    Manual,
    /// A configured risk limit was breached.
    RiskLimit,
    /// Market data is stale or unavailable.
    MarketDataStale,
    /// Execution adapter or session is degraded.
    AdapterDegraded,
    /// Required persistence is degraded.
    PersistenceDegraded,
    /// Recovery or reconciliation found a mismatch.
    ReconciliationMismatch,
    /// A strategy or system is emitting runaway flow.
    RunawayFlow,
    /// Regulatory, compliance, or venue direction.
    Regulatory,
    /// Recovery state is incomplete or uncertain.
    RecoveryUncertain,
    /// Host-defined reason code.
    Custom(u16),
}

/// Registry certainty used to enforce fail-closed startup and recovery.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KillSwitchStateCertainty {
    /// Active switch state has not been authoritatively restored.
    #[default]
    Uncertain = 0,
    /// Active switch state is known and can be evaluated.
    Confirmed = 1,
}

/// Kind of auditable kill-switch event.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KillSwitchEventKind {
    /// A switch became active.
    Activated = 0,
    /// One order cancellation result was recorded.
    CancelProgress = 1,
    /// A switch was cleared.
    Cleared = 2,
}

/// Aggregate cancellation state for one switch activation.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KillSwitchCancelOutcome {
    /// Selected mode does not require cancellation.
    NotRequired = 0,
    /// One or more affected orders still lack a recorded attempt.
    Pending = 1,
    /// Every affected order cancellation succeeded.
    AllSucceeded = 2,
    /// All attempts completed and at least one succeeded and one failed.
    PartiallyFailed = 3,
    /// All completed attempts failed.
    AllFailed = 4,
}

/// Errors returned by bounded kill-switch state management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KillSwitchError {
    /// Switch ids must be nonzero.
    InvalidSwitchId,
    /// Active-switch capacity is exhausted.
    SwitchCapacityExceeded,
    /// Cancel-result retention capacity is exhausted.
    CancelResultCapacityExceeded,
    /// The switch id is already active.
    DuplicateSwitchId,
    /// The switch id is not active.
    SwitchNotFound,
    /// A cancel result for this switch/order pair was already recorded.
    DuplicateCancelResult,
    /// Cancel result does not identify a captured affected order.
    UnexpectedCancelResult,
    /// Selected mode does not track order cancellation.
    CancellationNotRequired,
    /// Clear was requested before required cancellation completed cleanly.
    OutstandingCancellations,
}

impl fmt::Display for KillSwitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSwitchId => write!(f, "kill-switch id must be nonzero"),
            Self::SwitchCapacityExceeded => write!(f, "active kill-switch capacity exceeded"),
            Self::CancelResultCapacityExceeded => {
                write!(f, "kill-switch cancel-result capacity exceeded")
            }
            Self::DuplicateSwitchId => write!(f, "kill-switch id is already active"),
            Self::SwitchNotFound => write!(f, "kill-switch id is not active"),
            Self::DuplicateCancelResult => {
                write!(f, "kill-switch cancel result was already recorded")
            }
            Self::UnexpectedCancelResult => {
                write!(
                    f,
                    "kill-switch cancel result does not match an affected order"
                )
            }
            Self::CancellationNotRequired => {
                write!(f, "kill-switch mode does not require cancellation")
            }
            Self::OutstandingCancellations => {
                write!(f, "kill-switch cancellation is incomplete or failed")
            }
        }
    }
}

impl Error for KillSwitchError {}

/// Order metadata needed for scope matching and reduce-only evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct KillSwitchOrderContext {
    /// Current client order id.
    pub client_order_id: ClientOrderId,
    /// Venue order id when known.
    pub venue_order_id: VenueOrderId,
    /// Trading account.
    pub account_id: AccountId,
    /// Execution route.
    pub route_id: RouteId,
    /// Strategy attribution.
    pub strategy_id: StrategyId,
    /// Venue-native symbol.
    pub symbol: ExecutionSymbol,
    /// Canonical order side.
    pub side: OrderSide,
    /// Canonical order type.
    pub order_type: OrderType,
    /// Requested or original quantity.
    pub quantity: OrderQty,
    /// Current signed position for reduce-only evaluation.
    pub current_position: i64,
    /// Adapter/session identity when known.
    pub adapter_session_id: KillSwitchSessionId,
    /// True when this context represents a currently open order.
    pub open: bool,
}

impl KillSwitchOrderContext {
    /// Creates a new-order evaluation context.
    pub const fn from_request(
        request: &OrderRequest,
        current_position: i64,
        adapter_session_id: KillSwitchSessionId,
    ) -> Self {
        Self {
            client_order_id: request.client_order_id,
            venue_order_id: VenueOrderId::empty(),
            account_id: request.account_id,
            route_id: request.route_id,
            strategy_id: request.strategy_id,
            symbol: request.symbol,
            side: request.side,
            order_type: request.order_type,
            quantity: request.quantity,
            current_position,
            adapter_session_id,
            open: false,
        }
    }

    /// Creates an open-order scope context from local OMS state plus metadata
    /// not retained by [`OrderState`].
    pub const fn from_state(
        state: &OrderState,
        strategy_id: StrategyId,
        order_type: OrderType,
        adapter_session_id: KillSwitchSessionId,
        current_position: i64,
    ) -> Self {
        Self {
            client_order_id: state.client_order_id,
            venue_order_id: state.venue_order_id,
            account_id: state.account_id,
            route_id: state.route_id,
            strategy_id,
            symbol: state.symbol,
            side: state.side,
            order_type,
            quantity: state.order_qty,
            current_position,
            adapter_session_id,
            open: !state.status.is_terminal(),
        }
    }

    fn is_strictly_reduce_only(self) -> bool {
        match self.side {
            OrderSide::Buy => {
                self.current_position < 0
                    && self.quantity.0 > 0
                    && self.quantity.0 <= self.current_position.saturating_abs()
            }
            OrderSide::Sell => {
                self.current_position > 0
                    && self.quantity.0 > 0
                    && self.quantity.0 <= self.current_position
            }
        }
    }
}

/// One affected open order emitted into a caller-owned cancellation buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct KillSwitchAffectedOrder {
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Venue order id when known.
    pub venue_order_id: VenueOrderId,
    /// Trading account.
    pub account_id: AccountId,
    /// Route id.
    pub route_id: RouteId,
    /// Strategy id.
    pub strategy_id: StrategyId,
    /// Venue-native symbol.
    pub symbol: ExecutionSymbol,
    /// Adapter/session identity.
    pub adapter_session_id: KillSwitchSessionId,
}

impl From<&KillSwitchOrderContext> for KillSwitchAffectedOrder {
    fn from(context: &KillSwitchOrderContext) -> Self {
        Self {
            client_order_id: context.client_order_id,
            venue_order_id: context.venue_order_id,
            account_id: context.account_id,
            route_id: context.route_id,
            strategy_id: context.strategy_id,
            symbol: context.symbol,
            adapter_session_id: context.adapter_session_id,
        }
    }
}

/// Caller-owned bounded output for affected open orders.
#[derive(Debug, Clone)]
pub struct KillSwitchAffectedOrderBuffer {
    orders: Vec<KillSwitchAffectedOrder>,
    max_len: usize,
}

impl KillSwitchAffectedOrderBuffer {
    /// Creates an empty output buffer with a hard maximum length.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            orders: Vec::with_capacity(capacity),
            max_len: capacity,
        }
    }

    /// Returns captured affected orders.
    pub fn as_slice(&self) -> &[KillSwitchAffectedOrder] {
        &self.orders
    }

    /// Clears captured orders without releasing capacity.
    pub fn clear(&mut self) {
        self.orders.clear();
    }

    /// Returns captured order count.
    pub fn len(&self) -> usize {
        self.orders.len()
    }

    /// Returns true when no order was captured.
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    /// Returns configured maximum captured order count.
    pub const fn max_len(&self) -> usize {
        self.max_len
    }

    fn push_if_available(&mut self, order: KillSwitchAffectedOrder) -> bool {
        if self.orders.len() >= self.max_len {
            return false;
        }
        self.orders.push(order);
        true
    }
}

impl Default for KillSwitchAffectedOrderBuffer {
    fn default() -> Self {
        Self::with_capacity(256)
    }
}

/// Command that activates a scoped kill switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillSwitchActivation {
    /// Switch lifecycle id.
    pub switch_id: KillSwitchId,
    /// Scope selected by the switch.
    pub scope: KillSwitchScope,
    /// Operational mode.
    pub mode: KillSwitchMode,
    /// Actor or system responsible for activation.
    pub source: KillSwitchSource,
    /// Structured activation reason.
    pub reason: KillSwitchReasonCode,
    /// Caller-supplied activation timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// WAL sequence assigned to the activation record.
    pub wal_sequence: WalSequence,
}

impl KillSwitchActivation {
    /// Creates an activation command with complete audit metadata.
    pub const fn new(
        switch_id: KillSwitchId,
        scope: KillSwitchScope,
        mode: KillSwitchMode,
        source: KillSwitchSource,
        reason: KillSwitchReasonCode,
        timestamp_ns: u64,
        wal_sequence: WalSequence,
    ) -> Self {
        Self {
            switch_id,
            scope,
            mode,
            source,
            reason,
            timestamp_ns,
            wal_sequence,
        }
    }
}

/// Command that records one attempted cancellation for an active switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillSwitchCancelResult {
    /// Active switch id.
    pub switch_id: KillSwitchId,
    /// Affected client order id.
    pub client_order_id: ClientOrderId,
    /// True when the cancellation request succeeded or was acknowledged.
    pub succeeded: bool,
    /// Actor or system recording the result.
    pub source: KillSwitchSource,
    /// Structured result reason.
    pub reason: KillSwitchReasonCode,
    /// Caller-supplied result timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// WAL sequence assigned to this result record.
    pub wal_sequence: WalSequence,
}

/// Command that clears one active switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillSwitchClear {
    /// Active switch id.
    pub switch_id: KillSwitchId,
    /// Actor or system responsible for clearing.
    pub source: KillSwitchSource,
    /// Structured clear reason.
    pub reason: KillSwitchReasonCode,
    /// Caller-supplied clear timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// WAL sequence assigned to the clear record.
    pub wal_sequence: WalSequence,
    /// Allows an explicit operator override of incomplete/failed cancels.
    pub force: bool,
}

impl KillSwitchClear {
    /// Creates a conservative clear command that requires clean cancellation.
    pub const fn new(
        switch_id: KillSwitchId,
        source: KillSwitchSource,
        reason: KillSwitchReasonCode,
        timestamp_ns: u64,
        wal_sequence: WalSequence,
    ) -> Self {
        Self {
            switch_id,
            source,
            reason,
            timestamp_ns,
            wal_sequence,
            force: false,
        }
    }

    /// Enables or disables explicit forced clearing.
    pub const fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

/// Immutable audit event emitted for kill-switch state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct KillSwitchEvent {
    /// Event kind.
    pub kind: KillSwitchEventKind,
    /// Switch lifecycle id.
    pub switch_id: KillSwitchId,
    /// Active scope.
    pub scope: KillSwitchScope,
    /// Active mode.
    pub mode: KillSwitchMode,
    /// Actor or system responsible for this event.
    pub source: KillSwitchSource,
    /// Structured event reason.
    pub reason: KillSwitchReasonCode,
    /// Caller-supplied event timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// WAL sequence assigned to this event.
    pub wal_sequence: WalSequence,
    /// Number of matching open orders at activation.
    pub affected_order_count: u32,
    /// Number of affected ids captured in the caller-owned output buffer.
    pub captured_order_count: u32,
    /// True when affected ids exceeded output capacity.
    pub affected_orders_truncated: bool,
    /// Number of unique cancellation attempts recorded.
    pub cancel_attempted: u32,
    /// Number of successful cancellation attempts.
    pub cancel_succeeded: u32,
    /// Number of failed cancellation attempts.
    pub cancel_failed: u32,
    /// Aggregate cancellation outcome.
    pub cancel_outcome: KillSwitchCancelOutcome,
    /// True when registry state was authoritatively confirmed.
    pub state_confirmed: bool,
    /// True when clear bypassed incomplete or failed cancellations.
    pub forced: bool,
}

/// Read-only active switch entry retained by [`KillSwitchRegistry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ActiveKillSwitch {
    activation: KillSwitchActivation,
    affected_order_count: u32,
    captured_order_count: u32,
    affected_orders_truncated: bool,
    cancel_attempted: u32,
    cancel_succeeded: u32,
    cancel_failed: u32,
}

impl ActiveKillSwitch {
    /// Returns activation metadata.
    pub const fn activation(self) -> KillSwitchActivation {
        self.activation
    }

    /// Returns matching open-order count captured at activation.
    pub const fn affected_order_count(self) -> u32 {
        self.affected_order_count
    }

    /// Returns aggregate cancellation outcome.
    pub const fn cancel_outcome(self) -> KillSwitchCancelOutcome {
        cancel_outcome(
            self.activation.mode,
            self.affected_order_count,
            self.cancel_attempted,
            self.cancel_succeeded,
            self.cancel_failed,
        )
    }
}

/// Reason selected by a kill-switch order decision.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KillSwitchDecisionReason {
    /// No active switch blocks the order.
    Allowed = 0,
    /// Registry state is uncertain, so evaluation fails closed.
    StateUncertain = 1,
    /// Matching switch rejects new orders.
    RejectNew = 2,
    /// Order would not strictly reduce the supplied position.
    ReduceOnlyViolation = 3,
    /// Matching strategy flow is paused.
    StrategyPaused = 4,
    /// Matching adapter/session must be stopped.
    AdapterHardStop = 5,
}

/// Allocation-free decision for one prospective order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct KillSwitchDecision {
    /// True when the new order may continue to normal risk checks.
    pub allow_new_order: bool,
    /// True when cancel flow should remain available.
    pub allow_cancels: bool,
    /// True when a matching reduce-only policy is active.
    pub reduce_only: bool,
    /// True when matching strategy commands should pause.
    pub pause_strategy: bool,
    /// True when matching adapter/session shutdown is required.
    pub hard_stop_adapter: bool,
    /// Number of active switches matching this context.
    pub matched_switches: u32,
    /// Primary decision reason.
    pub reason: KillSwitchDecisionReason,
    /// First blocking switch id when known.
    pub blocking_switch_id: Option<KillSwitchId>,
}

impl KillSwitchDecision {
    const fn allowed() -> Self {
        Self {
            allow_new_order: true,
            allow_cancels: true,
            reduce_only: false,
            pause_strategy: false,
            hard_stop_adapter: false,
            matched_switches: 0,
            reason: KillSwitchDecisionReason::Allowed,
            blocking_switch_id: None,
        }
    }

    const fn uncertain() -> Self {
        Self {
            allow_new_order: false,
            allow_cancels: true,
            reduce_only: false,
            pause_strategy: true,
            hard_stop_adapter: false,
            matched_switches: 0,
            reason: KillSwitchDecisionReason::StateUncertain,
            blocking_switch_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RecordedCancelResult {
    switch_id: KillSwitchId,
    client_order_id: ClientOrderId,
}

/// Bounded registry for scoped kill-switch state and cancellation progress.
///
/// [`Self::new`] starts uncertain and therefore fails closed. Use
/// [`Self::confirmed_empty`] only for an authoritative new session, or call
/// [`Self::confirm_state`] after restoring active state from durable storage.
#[derive(Debug)]
pub struct KillSwitchRegistry {
    entries: Vec<ActiveKillSwitch>,
    switch_capacity: usize,
    cancel_targets: Vec<RecordedCancelResult>,
    cancel_results: Vec<RecordedCancelResult>,
    cancel_result_capacity: usize,
    certainty: KillSwitchStateCertainty,
}

impl KillSwitchRegistry {
    /// Creates an uncertain registry with bounded active and result capacity.
    pub fn new(switch_capacity: usize, cancel_result_capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(switch_capacity),
            switch_capacity,
            cancel_targets: Vec::with_capacity(cancel_result_capacity),
            cancel_results: Vec::with_capacity(cancel_result_capacity),
            cancel_result_capacity,
            certainty: KillSwitchStateCertainty::Uncertain,
        }
    }

    /// Creates an authoritatively empty registry for a new session.
    pub fn confirmed_empty(switch_capacity: usize, cancel_result_capacity: usize) -> Self {
        let mut registry = Self::new(switch_capacity, cancel_result_capacity);
        registry.certainty = KillSwitchStateCertainty::Confirmed;
        registry
    }

    /// Marks restored registry state as authoritative.
    pub fn confirm_state(&mut self) {
        self.certainty = KillSwitchStateCertainty::Confirmed;
    }

    /// Marks registry state uncertain so all new orders fail closed.
    pub fn mark_state_uncertain(&mut self) {
        self.certainty = KillSwitchStateCertainty::Uncertain;
    }

    /// Returns current state certainty.
    pub const fn certainty(&self) -> KillSwitchStateCertainty {
        self.certainty
    }

    /// Returns active switch entries in activation order.
    pub fn active_switches(&self) -> &[ActiveKillSwitch] {
        &self.entries
    }

    /// Returns active switch count.
    pub fn active_count(&self) -> usize {
        self.entries.len()
    }

    /// Activates a switch and captures matching open orders without growing the
    /// caller-owned output beyond its configured bound.
    ///
    /// # Errors
    ///
    /// Returns an error for duplicate ids or exhausted active-switch capacity.
    pub fn activate(
        &mut self,
        activation: KillSwitchActivation,
        open_orders: &[KillSwitchOrderContext],
        affected: &mut KillSwitchAffectedOrderBuffer,
    ) -> Result<KillSwitchEvent, KillSwitchError> {
        if self
            .entries
            .iter()
            .any(|entry| entry.activation.switch_id == activation.switch_id)
        {
            return Err(KillSwitchError::DuplicateSwitchId);
        }
        if self.entries.len() >= self.switch_capacity {
            return Err(KillSwitchError::SwitchCapacityExceeded);
        }

        affected.clear();
        let mut affected_order_count = 0_u32;
        for order in open_orders {
            if order.open && activation_cancels_order(activation, order) {
                affected_order_count = affected_order_count.saturating_add(1);
                if self.cancel_targets.len().saturating_add(affected.len())
                    < self.cancel_result_capacity
                {
                    affected.push_if_available(KillSwitchAffectedOrder::from(order));
                }
            }
        }
        let captured_order_count = affected.len().min(u32::MAX as usize) as u32;
        self.cancel_targets.extend(
            affected
                .as_slice()
                .iter()
                .map(|order| RecordedCancelResult {
                    switch_id: activation.switch_id,
                    client_order_id: order.client_order_id,
                }),
        );
        let entry = ActiveKillSwitch {
            activation,
            affected_order_count,
            captured_order_count,
            affected_orders_truncated: affected_order_count > captured_order_count,
            cancel_attempted: 0,
            cancel_succeeded: 0,
            cancel_failed: 0,
        };
        self.entries.push(entry);
        Ok(event_from_entry(
            KillSwitchEventKind::Activated,
            entry,
            activation.source,
            activation.reason,
            activation.timestamp_ns,
            activation.wal_sequence,
            self.certainty,
            false,
        ))
    }

    /// Records one unique affected-order cancellation result.
    ///
    /// # Errors
    ///
    /// Returns an error when the switch is absent, does not require
    /// cancellation, the result is duplicated, or result capacity is full.
    pub fn record_cancel_result(
        &mut self,
        result: KillSwitchCancelResult,
    ) -> Result<KillSwitchEvent, KillSwitchError> {
        let index = self.entry_index(result.switch_id)?;
        if !self.entries[index].activation.mode.requires_cancellation() {
            return Err(KillSwitchError::CancellationNotRequired);
        }
        if !self.cancel_targets.iter().any(|target| {
            target.switch_id == result.switch_id && target.client_order_id == result.client_order_id
        }) {
            return Err(KillSwitchError::UnexpectedCancelResult);
        }
        if self.cancel_results.iter().any(|record| {
            record.switch_id == result.switch_id && record.client_order_id == result.client_order_id
        }) {
            return Err(KillSwitchError::DuplicateCancelResult);
        }
        if self.cancel_results.len() >= self.cancel_result_capacity {
            return Err(KillSwitchError::CancelResultCapacityExceeded);
        }
        self.cancel_results.push(RecordedCancelResult {
            switch_id: result.switch_id,
            client_order_id: result.client_order_id,
        });
        let entry = &mut self.entries[index];
        entry.cancel_attempted = entry.cancel_attempted.saturating_add(1);
        if result.succeeded {
            entry.cancel_succeeded = entry.cancel_succeeded.saturating_add(1);
        } else {
            entry.cancel_failed = entry.cancel_failed.saturating_add(1);
        }
        Ok(event_from_entry(
            KillSwitchEventKind::CancelProgress,
            *entry,
            result.source,
            result.reason,
            result.timestamp_ns,
            result.wal_sequence,
            self.certainty,
            false,
        ))
    }

    /// Clears one switch after required cancellations complete successfully, or
    /// after an explicit forced override.
    ///
    /// # Errors
    ///
    /// Returns an error when the switch is absent or cancellation remains
    /// incomplete/failed without `force`.
    pub fn clear(&mut self, command: KillSwitchClear) -> Result<KillSwitchEvent, KillSwitchError> {
        let index = self.entry_index(command.switch_id)?;
        let entry = self.entries[index];
        let outcome = entry.cancel_outcome();
        let clean = matches!(
            outcome,
            KillSwitchCancelOutcome::NotRequired | KillSwitchCancelOutcome::AllSucceeded
        );
        if !clean && !command.force {
            return Err(KillSwitchError::OutstandingCancellations);
        }

        self.entries.remove(index);
        self.cancel_targets
            .retain(|target| target.switch_id != command.switch_id);
        self.cancel_results
            .retain(|result| result.switch_id != command.switch_id);
        Ok(event_from_entry(
            KillSwitchEventKind::Cleared,
            entry,
            command.source,
            command.reason,
            command.timestamp_ns,
            command.wal_sequence,
            self.certainty,
            command.force,
        ))
    }

    /// Evaluates all active switches against one prospective order.
    pub fn evaluate_new_order(&self, order: &KillSwitchOrderContext) -> KillSwitchDecision {
        if self.certainty != KillSwitchStateCertainty::Confirmed {
            return KillSwitchDecision::uncertain();
        }

        let mut decision = KillSwitchDecision::allowed();
        for entry in &self.entries {
            let activation = entry.activation;
            let matches = matches!(activation.mode, KillSwitchMode::CancelAll)
                || scope_matches(activation.scope, order);
            if !matches {
                continue;
            }
            decision.matched_switches = decision.matched_switches.saturating_add(1);
            match activation.mode {
                KillSwitchMode::RejectNew
                | KillSwitchMode::CancelAll
                | KillSwitchMode::CancelScope => block(
                    &mut decision,
                    activation.switch_id,
                    KillSwitchDecisionReason::RejectNew,
                ),
                KillSwitchMode::ReduceOnly => {
                    decision.reduce_only = true;
                    if !order.is_strictly_reduce_only() {
                        block(
                            &mut decision,
                            activation.switch_id,
                            KillSwitchDecisionReason::ReduceOnlyViolation,
                        );
                    }
                }
                KillSwitchMode::PauseStrategy => {
                    decision.pause_strategy = true;
                    block(
                        &mut decision,
                        activation.switch_id,
                        KillSwitchDecisionReason::StrategyPaused,
                    );
                }
                KillSwitchMode::HardStopAdapter => {
                    decision.hard_stop_adapter = true;
                    decision.allow_cancels = false;
                    block(
                        &mut decision,
                        activation.switch_id,
                        KillSwitchDecisionReason::AdapterHardStop,
                    );
                }
            }
        }
        decision
    }

    /// Builds a prospective-order context and evaluates all active switches.
    pub fn evaluate_request(
        &self,
        request: &OrderRequest,
        current_position: i64,
        adapter_session_id: KillSwitchSessionId,
    ) -> KillSwitchDecision {
        self.evaluate_new_order(&KillSwitchOrderContext::from_request(
            request,
            current_position,
            adapter_session_id,
        ))
    }

    fn entry_index(&self, switch_id: KillSwitchId) -> Result<usize, KillSwitchError> {
        self.entries
            .iter()
            .position(|entry| entry.activation.switch_id == switch_id)
            .ok_or(KillSwitchError::SwitchNotFound)
    }
}

impl Default for KillSwitchRegistry {
    fn default() -> Self {
        Self::new(64, 4_096)
    }
}

fn block(
    decision: &mut KillSwitchDecision,
    switch_id: KillSwitchId,
    reason: KillSwitchDecisionReason,
) {
    decision.allow_new_order = false;
    if decision.blocking_switch_id.is_none() {
        decision.blocking_switch_id = Some(switch_id);
        decision.reason = reason;
    }
}

fn activation_cancels_order(
    activation: KillSwitchActivation,
    order: &KillSwitchOrderContext,
) -> bool {
    match activation.mode {
        KillSwitchMode::CancelAll => true,
        KillSwitchMode::CancelScope | KillSwitchMode::HardStopAdapter => {
            scope_matches(activation.scope, order)
        }
        KillSwitchMode::RejectNew | KillSwitchMode::ReduceOnly | KillSwitchMode::PauseStrategy => {
            false
        }
    }
}

fn scope_matches(scope: KillSwitchScope, order: &KillSwitchOrderContext) -> bool {
    match scope {
        KillSwitchScope::Global => true,
        KillSwitchScope::Venue(venue) => order.symbol.venue == venue,
        KillSwitchScope::Route(route) => order.route_id == route,
        KillSwitchScope::Account(account) => order.account_id == account,
        KillSwitchScope::Strategy(strategy) => order.strategy_id == strategy,
        KillSwitchScope::Symbol(symbol) => order.symbol == symbol,
        KillSwitchScope::OrderType(order_type) => order.order_type == order_type,
        KillSwitchScope::AdapterSession(session_id) => {
            !session_id.is_empty() && order.adapter_session_id == session_id
        }
    }
}

const fn cancel_outcome(
    mode: KillSwitchMode,
    affected: u32,
    attempted: u32,
    succeeded: u32,
    failed: u32,
) -> KillSwitchCancelOutcome {
    if !mode.requires_cancellation() || affected == 0 {
        KillSwitchCancelOutcome::NotRequired
    } else if attempted < affected {
        KillSwitchCancelOutcome::Pending
    } else if failed == 0 && succeeded >= affected {
        KillSwitchCancelOutcome::AllSucceeded
    } else if succeeded == 0 {
        KillSwitchCancelOutcome::AllFailed
    } else {
        KillSwitchCancelOutcome::PartiallyFailed
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "audit event fields stay explicit"
)]
fn event_from_entry(
    kind: KillSwitchEventKind,
    entry: ActiveKillSwitch,
    source: KillSwitchSource,
    reason: KillSwitchReasonCode,
    timestamp_ns: u64,
    wal_sequence: WalSequence,
    certainty: KillSwitchStateCertainty,
    forced: bool,
) -> KillSwitchEvent {
    KillSwitchEvent {
        kind,
        switch_id: entry.activation.switch_id,
        scope: entry.activation.scope,
        mode: entry.activation.mode,
        source,
        reason,
        timestamp_ns,
        wal_sequence,
        affected_order_count: entry.affected_order_count,
        captured_order_count: entry.captured_order_count,
        affected_orders_truncated: entry.affected_orders_truncated,
        cancel_attempted: entry.cancel_attempted,
        cancel_succeeded: entry.cancel_succeeded,
        cancel_failed: entry.cancel_failed,
        cancel_outcome: entry.cancel_outcome(),
        state_confirmed: certainty == KillSwitchStateCertainty::Confirmed,
        forced,
    }
}

#[cfg(test)]
mod tests {
    use of_execution_core::{OrderPrice, OrderStatus, StrategyId};

    use super::*;

    fn fixed<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn source(actor: &str) -> KillSwitchSource {
        KillSwitchSource::from_id(KillSwitchSourceKind::Operator, actor).unwrap()
    }

    fn request(client: &str, strategy: &str, side: OrderSide, quantity: i64) -> OrderRequest {
        OrderRequest {
            client_order_id: fixed(client),
            account_id: fixed("account-a"),
            route_id: fixed("route-a"),
            strategy_id: fixed(strategy),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            side,
            order_type: OrderType::Limit,
            time_in_force: of_execution_core::TimeInForce::Day,
            quantity: OrderQty(quantity),
            limit_price: OrderPrice(5_000),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 0,
            ts_recv_ns: 100,
        }
    }

    fn context(client: &str, strategy: &str) -> KillSwitchOrderContext {
        KillSwitchOrderContext::from_request(
            &request(client, strategy, OrderSide::Buy, 2),
            0,
            fixed("session-a"),
        )
    }

    fn open_context(client: &str, strategy: &str) -> KillSwitchOrderContext {
        let req = request(client, strategy, OrderSide::Buy, 2);
        let state = OrderState {
            client_order_id: req.client_order_id,
            last_accepted_client_order_id: req.client_order_id,
            venue_order_id: fixed(&format!("venue-{client}")),
            account_id: req.account_id,
            route_id: req.route_id,
            symbol: req.symbol,
            side: req.side,
            status: OrderStatus::New,
            order_qty: req.quantity,
            cumulative_qty: OrderQty(0),
            leaves_qty: req.quantity,
            average_price: OrderPrice(0),
            updated_ns: 100,
        };
        KillSwitchOrderContext::from_state(
            &state,
            req.strategy_id,
            req.order_type,
            fixed("session-a"),
            0,
        )
    }

    fn activation(id: u64, scope: KillSwitchScope, mode: KillSwitchMode) -> KillSwitchActivation {
        KillSwitchActivation::new(
            KillSwitchId::new(id).unwrap(),
            scope,
            mode,
            source("ops-a"),
            KillSwitchReasonCode::Manual,
            1_000 + id,
            WalSequence(100 + id),
        )
    }

    #[test]
    fn default_registry_fails_closed_until_state_is_confirmed() {
        let mut registry = KillSwitchRegistry::default();
        let decision = registry.evaluate_new_order(&context("c1", "strategy-a"));
        assert!(!decision.allow_new_order);
        assert!(decision.allow_cancels);
        assert_eq!(decision.reason, KillSwitchDecisionReason::StateUncertain);

        registry.confirm_state();
        assert!(
            registry
                .evaluate_new_order(&context("c1", "strategy-a"))
                .allow_new_order
        );
    }

    #[test]
    fn scope_matching_covers_all_supported_dimensions() {
        let ctx = context("c1", "strategy-a");
        let matching = [
            KillSwitchScope::Global,
            KillSwitchScope::Venue(fixed("XCME")),
            KillSwitchScope::Route(fixed("route-a")),
            KillSwitchScope::Account(fixed("account-a")),
            KillSwitchScope::Strategy(fixed("strategy-a")),
            KillSwitchScope::Symbol(ExecutionSymbol::new("XCME", "ESM6").unwrap()),
            KillSwitchScope::OrderType(OrderType::Limit),
            KillSwitchScope::AdapterSession(fixed("session-a")),
        ];

        for (offset, scope) in matching.into_iter().enumerate() {
            let mut registry = KillSwitchRegistry::confirmed_empty(1, 0);
            registry
                .activate(
                    activation(offset as u64 + 1, scope, KillSwitchMode::RejectNew),
                    &[],
                    &mut KillSwitchAffectedOrderBuffer::with_capacity(0),
                )
                .unwrap();
            let decision = registry.evaluate_new_order(&ctx);
            assert!(!decision.allow_new_order, "scope {scope:?}");
            assert_eq!(decision.matched_switches, 1);
        }
    }

    #[test]
    fn nonmatching_scope_preserves_order_flow() {
        let mut registry = KillSwitchRegistry::confirmed_empty(2, 0);
        registry
            .activate(
                activation(
                    1,
                    KillSwitchScope::Strategy(StrategyId::new("strategy-b").unwrap()),
                    KillSwitchMode::RejectNew,
                ),
                &[],
                &mut KillSwitchAffectedOrderBuffer::with_capacity(0),
            )
            .unwrap();
        assert!(
            registry
                .evaluate_new_order(&context("c1", "strategy-a"))
                .allow_new_order
        );
    }

    #[test]
    fn reduce_only_requires_strict_position_reduction() {
        let mut registry = KillSwitchRegistry::confirmed_empty(1, 0);
        registry
            .activate(
                activation(1, KillSwitchScope::Global, KillSwitchMode::ReduceOnly),
                &[],
                &mut KillSwitchAffectedOrderBuffer::with_capacity(0),
            )
            .unwrap();

        let reduce = KillSwitchOrderContext::from_request(
            &request("buy-cover", "strategy-a", OrderSide::Buy, 2),
            -5,
            fixed("session-a"),
        );
        let reverse = KillSwitchOrderContext::from_request(
            &request("buy-reverse", "strategy-a", OrderSide::Buy, 6),
            -5,
            fixed("session-a"),
        );
        assert!(registry.evaluate_new_order(&reduce).allow_new_order);
        let decision = registry.evaluate_new_order(&reverse);
        assert!(!decision.allow_new_order);
        assert_eq!(
            decision.reason,
            KillSwitchDecisionReason::ReduceOnlyViolation
        );
    }

    #[test]
    fn cancel_scope_captures_matching_orders_and_reports_truncation() {
        let mut registry = KillSwitchRegistry::confirmed_empty(2, 8);
        let orders = [
            open_context("c1", "strategy-a"),
            open_context("c2", "strategy-a"),
            open_context("c3", "strategy-b"),
        ];
        let mut affected = KillSwitchAffectedOrderBuffer::with_capacity(1);
        let event = registry
            .activate(
                activation(
                    1,
                    KillSwitchScope::Strategy(fixed("strategy-a")),
                    KillSwitchMode::CancelScope,
                ),
                &orders,
                &mut affected,
            )
            .unwrap();

        assert_eq!(event.affected_order_count, 2);
        assert_eq!(event.captured_order_count, 1);
        assert!(event.affected_orders_truncated);
        assert_eq!(event.cancel_outcome, KillSwitchCancelOutcome::Pending);
        assert_eq!(affected.as_slice()[0].client_order_id.as_str(), "c1");
        assert_eq!(event.wal_sequence, WalSequence(101));
    }

    #[test]
    fn cancel_all_ignores_narrow_scope_for_emergency_behavior() {
        let mut registry = KillSwitchRegistry::confirmed_empty(1, 8);
        let orders = [
            open_context("c1", "strategy-a"),
            open_context("c2", "strategy-b"),
        ];
        let event = registry
            .activate(
                activation(
                    1,
                    KillSwitchScope::Strategy(fixed("nobody")),
                    KillSwitchMode::CancelAll,
                ),
                &orders,
                &mut KillSwitchAffectedOrderBuffer::with_capacity(8),
            )
            .unwrap();
        assert_eq!(event.affected_order_count, 2);
        assert!(
            !registry
                .evaluate_new_order(&context("new", "strategy-z"))
                .allow_new_order
        );
    }

    #[test]
    fn cancellation_progress_is_idempotent_and_clear_is_conservative() {
        let mut registry = KillSwitchRegistry::confirmed_empty(1, 8);
        let orders = [
            open_context("c1", "strategy-a"),
            open_context("c2", "strategy-a"),
        ];
        registry
            .activate(
                activation(1, KillSwitchScope::Global, KillSwitchMode::CancelScope),
                &orders,
                &mut KillSwitchAffectedOrderBuffer::with_capacity(8),
            )
            .unwrap();
        let result = KillSwitchCancelResult {
            switch_id: KillSwitchId::new(1).unwrap(),
            client_order_id: fixed("c1"),
            succeeded: true,
            source: source("cancel-worker"),
            reason: KillSwitchReasonCode::Manual,
            timestamp_ns: 2_000,
            wal_sequence: WalSequence(200),
        };
        let event = registry.record_cancel_result(result).unwrap();
        assert_eq!(event.cancel_attempted, 1);
        assert_eq!(event.cancel_outcome, KillSwitchCancelOutcome::Pending);
        assert_eq!(
            registry.record_cancel_result(result),
            Err(KillSwitchError::DuplicateCancelResult)
        );
        let unexpected = KillSwitchCancelResult {
            client_order_id: fixed("unknown"),
            ..result
        };
        assert_eq!(
            registry.record_cancel_result(unexpected),
            Err(KillSwitchError::UnexpectedCancelResult)
        );

        let clear = KillSwitchClear::new(
            KillSwitchId::new(1).unwrap(),
            source("ops-b"),
            KillSwitchReasonCode::Manual,
            3_000,
            WalSequence(300),
        );
        assert_eq!(
            registry.clear(clear),
            Err(KillSwitchError::OutstandingCancellations)
        );
    }

    #[test]
    fn successful_cancels_allow_clear_with_complete_audit_counts() {
        let mut registry = KillSwitchRegistry::confirmed_empty(1, 8);
        let orders = [
            open_context("c1", "strategy-a"),
            open_context("c2", "strategy-a"),
        ];
        registry
            .activate(
                activation(1, KillSwitchScope::Global, KillSwitchMode::CancelScope),
                &orders,
                &mut KillSwitchAffectedOrderBuffer::with_capacity(8),
            )
            .unwrap();
        for (index, client) in ["c1", "c2"].into_iter().enumerate() {
            registry
                .record_cancel_result(KillSwitchCancelResult {
                    switch_id: KillSwitchId::new(1).unwrap(),
                    client_order_id: fixed(client),
                    succeeded: true,
                    source: source("cancel-worker"),
                    reason: KillSwitchReasonCode::Manual,
                    timestamp_ns: 2_000 + index as u64,
                    wal_sequence: WalSequence(200 + index as u64),
                })
                .unwrap();
        }
        let event = registry
            .clear(KillSwitchClear::new(
                KillSwitchId::new(1).unwrap(),
                source("ops-b"),
                KillSwitchReasonCode::Manual,
                3_000,
                WalSequence(300),
            ))
            .unwrap();
        assert_eq!(event.kind, KillSwitchEventKind::Cleared);
        assert_eq!(event.cancel_outcome, KillSwitchCancelOutcome::AllSucceeded);
        assert_eq!(event.cancel_succeeded, 2);
        assert_eq!(event.source.actor_id().as_str(), "ops-b");
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn failed_cancellation_requires_explicit_forced_clear() {
        let mut registry = KillSwitchRegistry::confirmed_empty(1, 8);
        registry
            .activate(
                activation(1, KillSwitchScope::Global, KillSwitchMode::CancelScope),
                &[open_context("c1", "strategy-a")],
                &mut KillSwitchAffectedOrderBuffer::with_capacity(8),
            )
            .unwrap();
        registry
            .record_cancel_result(KillSwitchCancelResult {
                switch_id: KillSwitchId::new(1).unwrap(),
                client_order_id: fixed("c1"),
                succeeded: false,
                source: source("cancel-worker"),
                reason: KillSwitchReasonCode::AdapterDegraded,
                timestamp_ns: 2_000,
                wal_sequence: WalSequence(200),
            })
            .unwrap();
        let clear = KillSwitchClear::new(
            KillSwitchId::new(1).unwrap(),
            source("risk-admin"),
            KillSwitchReasonCode::Manual,
            3_000,
            WalSequence(300),
        )
        .with_force(true);
        let event = registry.clear(clear).unwrap();
        assert!(event.forced);
        assert_eq!(event.cancel_outcome, KillSwitchCancelOutcome::AllFailed);
    }

    #[test]
    fn hard_stop_blocks_new_and_cancel_flow_for_matching_session() {
        let mut registry = KillSwitchRegistry::confirmed_empty(1, 8);
        registry
            .activate(
                activation(
                    1,
                    KillSwitchScope::AdapterSession(fixed("session-a")),
                    KillSwitchMode::HardStopAdapter,
                ),
                &[],
                &mut KillSwitchAffectedOrderBuffer::with_capacity(0),
            )
            .unwrap();
        let decision = registry.evaluate_new_order(&context("c1", "strategy-a"));
        assert!(!decision.allow_new_order);
        assert!(!decision.allow_cancels);
        assert!(decision.hard_stop_adapter);
        assert_eq!(decision.reason, KillSwitchDecisionReason::AdapterHardStop);
    }

    #[test]
    fn bounded_capacities_fail_explicitly() {
        let mut registry = KillSwitchRegistry::confirmed_empty(1, 0);
        registry
            .activate(
                activation(1, KillSwitchScope::Global, KillSwitchMode::RejectNew),
                &[],
                &mut KillSwitchAffectedOrderBuffer::with_capacity(0),
            )
            .unwrap();
        assert_eq!(
            registry.activate(
                activation(2, KillSwitchScope::Global, KillSwitchMode::RejectNew),
                &[],
                &mut KillSwitchAffectedOrderBuffer::with_capacity(0),
            ),
            Err(KillSwitchError::SwitchCapacityExceeded)
        );
    }
}
