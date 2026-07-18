//! Parent/child execution algorithm primitives for Orderflow.
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::error::Error;
use std::fmt;

use of_execution_core::{
    AccountId, ClientOrderId, ExecutionCoreError, ExecutionEvent, ExecutionId, ExecutionSymbol,
    ExecutionText, ExecutionType, FixedAscii, OrderPrice, OrderQty, OrderRequest, OrderSide,
    OrderStatus, OrderType, RiskRejectReason, RouteId, StrategyId, TimeInForce, VenueOrderId,
};

/// Default maximum number of actions retained in an [`AlgoDecision`].
pub const DEFAULT_ALGO_DECISION_CAPACITY: usize = 16;
/// Default maximum number of retained violations in an [`AlgoRiskReport`].
pub const DEFAULT_ALGO_RISK_VIOLATION_CAPACITY: usize = 16;
/// Current algorithm checkpoint schema version.
pub const ALGO_CHECKPOINT_SCHEMA_VERSION: u16 = 1;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Algorithm parent-order identifier.
pub type ParentOrderId = FixedAscii<40>;
/// Algorithm child-order identifier.
pub type ChildOrderId = FixedAscii<40>;
/// Strategy intent identifier.
pub type AlgoIntentId = FixedAscii<40>;
/// Running algorithm instance identifier.
pub type AlgoInstanceId = FixedAscii<40>;

/// Execution-algorithm status for a parent order.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParentOrderStatus {
    /// Parent order has been accepted locally but is not yet active.
    Pending = 1,
    /// Parent order is actively releasing child orders.
    Active = 2,
    /// Parent order is paused by policy, operator, or market condition.
    Paused = 3,
    /// Parent order completed its target quantity.
    Completed = 4,
    /// Parent order was cancelled before completion.
    Cancelled = 5,
    /// Parent order was rejected before activation.
    Rejected = 6,
    /// Parent order expired before completion.
    Expired = 7,
    /// Parent order failed and requires operator or recovery action.
    Failed = 8,
    /// Parent order is being recovered from journal/checkpoint state.
    Recovering = 9,
}

impl ParentOrderStatus {
    /// Returns true when no further child-order release is expected.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Rejected | Self::Expired | Self::Failed
        )
    }
}

/// Execution-algorithm status for a child order.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChildOrderStatus {
    /// Child order is planned but not yet submitted to the OMS.
    Planned = 1,
    /// Child order was submitted to the OMS.
    Submitted = 2,
    /// Child order was accepted by the venue.
    Accepted = 3,
    /// Child order is partially filled.
    PartiallyFilled = 4,
    /// Child order is fully filled.
    Filled = 5,
    /// Child cancel is pending.
    PendingCancel = 6,
    /// Child order was cancelled.
    Cancelled = 7,
    /// Child replace is pending.
    PendingReplace = 8,
    /// Child order was rejected.
    Rejected = 9,
    /// Child order expired.
    Expired = 10,
    /// Child order state is unknown after recovery/reconciliation.
    Unknown = 11,
}

impl ChildOrderStatus {
    /// Maps a canonical OMS order status into a child-order status.
    pub const fn from_order_status(status: OrderStatus) -> Self {
        match status {
            OrderStatus::PendingNew => Self::Submitted,
            OrderStatus::New => Self::Accepted,
            OrderStatus::PartiallyFilled => Self::PartiallyFilled,
            OrderStatus::Filled => Self::Filled,
            OrderStatus::PendingCancel => Self::PendingCancel,
            OrderStatus::Cancelled => Self::Cancelled,
            OrderStatus::PendingReplace | OrderStatus::Replaced => Self::PendingReplace,
            OrderStatus::Rejected => Self::Rejected,
            OrderStatus::Expired => Self::Expired,
            OrderStatus::Suspended | OrderStatus::Unknown => Self::Unknown,
        }
    }

    /// Returns true when no further venue activity is expected.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Filled | Self::Cancelled | Self::Rejected | Self::Expired
        )
    }
}

/// Execution-algorithm error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AlgoError {
    /// Core execution-domain validation failed.
    Core(ExecutionCoreError),
    /// Parent start time must be strictly before end time.
    InvalidTimeWindow,
    /// Slice interval must be positive.
    InvalidSliceInterval,
    /// Minimum clip must be positive and no larger than maximum clip.
    InvalidClipBounds,
    /// Parent order is terminal and cannot release more child orders.
    ParentTerminal,
    /// Progress quantities are internally inconsistent.
    InvalidProgress,
    /// Algorithm risk limits or context are invalid.
    InvalidRiskParameters,
    /// Algorithm checkpoint or recovery state is invalid.
    InvalidRecoveryState,
    /// Algorithm simulation inputs are invalid.
    InvalidSimulationParameters,
    /// Algorithm metrics or benchmark inputs are invalid.
    InvalidMetricsParameters,
    /// Fixed-capacity decision buffer is full.
    DecisionFull {
        /// Configured decision capacity.
        capacity: usize,
    },
    /// Deterministic replay generated an identifier that exceeded capacity.
    GeneratedIdentifierTooLong,
    /// Participation rate must be positive and no greater than the cap.
    InvalidParticipationRate,
    /// VWAP volume profile is empty, non-monotonic, or has zero total weight.
    InvalidVolumeProfile,
    /// Iceberg display quantity or replenish threshold is invalid.
    InvalidDisplayQuantity,
    /// Passive queue configuration or market context is invalid.
    InvalidPassiveQueueParameters,
    /// Smart-order-router configuration or route candidate is invalid.
    InvalidSorParameters,
    /// Liquidity-seeking configuration or candidate is invalid.
    InvalidLiquiditySeekingParameters,
    /// Sweep/aggressive-take configuration or candidate is invalid.
    InvalidSweepParameters,
    /// Basket or spread leg configuration is invalid.
    InvalidBasketParameters,
    /// Pairs/spread configuration or quote context is invalid.
    InvalidSpreadParameters,
    /// Market-making configuration or quote context is invalid.
    InvalidMarketMakingParameters,
    /// Implementation shortfall configuration or market context is invalid.
    InvalidShortfallParameters,
}

impl fmt::Display for AlgoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(err) => write!(f, "{err}"),
            Self::InvalidTimeWindow => write!(f, "parent start time must be before end time"),
            Self::InvalidSliceInterval => write!(f, "slice interval must be positive"),
            Self::InvalidClipBounds => {
                write!(f, "minimum clip must be positive and <= maximum clip")
            }
            Self::ParentTerminal => write!(f, "terminal parent cannot release child orders"),
            Self::InvalidProgress => write!(f, "algorithm progress is inconsistent"),
            Self::InvalidRiskParameters => write!(f, "invalid algorithm risk parameters"),
            Self::InvalidRecoveryState => write!(f, "invalid algorithm recovery state"),
            Self::InvalidSimulationParameters => {
                write!(f, "invalid algorithm simulation parameters")
            }
            Self::InvalidMetricsParameters => write!(f, "invalid algorithm metrics parameters"),
            Self::DecisionFull { capacity } => {
                write!(f, "algorithm decision capacity {capacity} is full")
            }
            Self::GeneratedIdentifierTooLong => write!(f, "generated identifier is too long"),
            Self::InvalidParticipationRate => {
                write!(f, "participation rate must be positive and <= cap")
            }
            Self::InvalidVolumeProfile => write!(f, "invalid VWAP volume profile"),
            Self::InvalidDisplayQuantity => write!(f, "invalid iceberg display quantity"),
            Self::InvalidPassiveQueueParameters => {
                write!(f, "invalid passive queue parameters")
            }
            Self::InvalidSorParameters => write!(f, "invalid SOR parameters"),
            Self::InvalidLiquiditySeekingParameters => {
                write!(f, "invalid liquidity-seeking parameters")
            }
            Self::InvalidSweepParameters => write!(f, "invalid sweep parameters"),
            Self::InvalidBasketParameters => write!(f, "invalid basket parameters"),
            Self::InvalidSpreadParameters => write!(f, "invalid spread parameters"),
            Self::InvalidMarketMakingParameters => {
                write!(f, "invalid market-making parameters")
            }
            Self::InvalidShortfallParameters => {
                write!(f, "invalid implementation shortfall parameters")
            }
        }
    }
}

impl Error for AlgoError {}

impl From<ExecutionCoreError> for AlgoError {
    fn from(value: ExecutionCoreError) -> Self {
        Self::Core(value)
    }
}

/// Parent order controlled by an execution algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentOrder {
    id: ParentOrderId,
    account_id: AccountId,
    route_id: RouteId,
    strategy_id: StrategyId,
    symbol: ExecutionSymbol,
    side: OrderSide,
    order_type: OrderType,
    time_in_force: TimeInForce,
    total_qty: OrderQty,
    limit_price: OrderPrice,
    stop_price: OrderPrice,
    start_ns: u64,
    end_ns: u64,
    min_clip: OrderQty,
    max_clip: OrderQty,
    participation_cap_bps: u16,
    status: ParentOrderStatus,
}

impl ParentOrder {
    /// Creates an active parent order.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when order shape, schedule, or clip bounds are
    /// invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "flat constructor mirrors order ticket fields"
    )]
    pub fn new(
        id: ParentOrderId,
        account_id: AccountId,
        route_id: RouteId,
        strategy_id: StrategyId,
        symbol: ExecutionSymbol,
        side: OrderSide,
        order_type: OrderType,
        time_in_force: TimeInForce,
        total_qty: OrderQty,
        limit_price: OrderPrice,
        stop_price: OrderPrice,
        start_ns: u64,
        end_ns: u64,
        min_clip: OrderQty,
        max_clip: OrderQty,
        participation_cap_bps: u16,
    ) -> Result<Self, AlgoError> {
        let parent = Self {
            id,
            account_id,
            route_id,
            strategy_id,
            symbol,
            side,
            order_type,
            time_in_force,
            total_qty,
            limit_price,
            stop_price,
            start_ns,
            end_ns,
            min_clip,
            max_clip,
            participation_cap_bps,
            status: ParentOrderStatus::Active,
        };
        parent.validate()?;
        Ok(parent)
    }

    /// Validates parent order shape and schedule.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when order shape, schedule, or clip bounds are
    /// invalid.
    pub fn validate(&self) -> Result<(), AlgoError> {
        if self.start_ns >= self.end_ns {
            return Err(AlgoError::InvalidTimeWindow);
        }
        if self.min_clip.0 <= 0 || self.max_clip.0 <= 0 || self.min_clip.0 > self.max_clip.0 {
            return Err(AlgoError::InvalidClipBounds);
        }
        let request = self.build_order_request(ClientOrderId::empty(), self.total_qty, 0);
        request.validate()?;
        Ok(())
    }

    /// Returns the parent identifier.
    pub const fn id(&self) -> ParentOrderId {
        self.id
    }

    /// Returns the account identifier.
    pub const fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the default route identifier for child orders.
    pub const fn route_id(&self) -> RouteId {
        self.route_id
    }

    /// Returns the strategy attribution identifier.
    pub const fn strategy_id(&self) -> StrategyId {
        self.strategy_id
    }

    /// Returns the execution symbol.
    pub const fn symbol(&self) -> ExecutionSymbol {
        self.symbol
    }

    /// Returns the order side.
    pub const fn side(&self) -> OrderSide {
        self.side
    }

    /// Returns the child order type.
    pub const fn order_type(&self) -> OrderType {
        self.order_type
    }

    /// Returns the child time-in-force.
    pub const fn time_in_force(&self) -> TimeInForce {
        self.time_in_force
    }

    /// Returns the target total quantity.
    pub const fn total_qty(&self) -> OrderQty {
        self.total_qty
    }

    /// Returns the parent limit price.
    pub const fn limit_price(&self) -> OrderPrice {
        self.limit_price
    }

    /// Returns the parent stop price.
    pub const fn stop_price(&self) -> OrderPrice {
        self.stop_price
    }

    /// Returns the algorithm start timestamp.
    pub const fn start_ns(&self) -> u64 {
        self.start_ns
    }

    /// Returns the algorithm end timestamp.
    pub const fn end_ns(&self) -> u64 {
        self.end_ns
    }

    /// Returns the minimum child clip.
    pub const fn min_clip(&self) -> OrderQty {
        self.min_clip
    }

    /// Returns the maximum child clip.
    pub const fn max_clip(&self) -> OrderQty {
        self.max_clip
    }

    /// Returns the optional participation cap in basis points, or zero when
    /// unset.
    pub const fn participation_cap_bps(&self) -> u16 {
        self.participation_cap_bps
    }

    /// Returns the parent lifecycle status.
    pub const fn status(&self) -> ParentOrderStatus {
        self.status
    }

    /// Returns a copy with a different lifecycle status.
    pub const fn with_status(mut self, status: ParentOrderStatus) -> Self {
        self.status = status;
        self
    }

    fn build_order_request(
        &self,
        client_order_id: ClientOrderId,
        quantity: OrderQty,
        ts_recv_ns: u64,
    ) -> OrderRequest {
        self.build_order_request_at_price(client_order_id, quantity, self.limit_price, ts_recv_ns)
    }

    fn build_order_request_at_price(
        &self,
        client_order_id: ClientOrderId,
        quantity: OrderQty,
        limit_price: OrderPrice,
        ts_recv_ns: u64,
    ) -> OrderRequest {
        self.build_order_request_for_route_at_price(
            self.route_id,
            client_order_id,
            quantity,
            limit_price,
            ts_recv_ns,
        )
    }

    fn build_order_request_for_route_at_price(
        &self,
        route_id: RouteId,
        client_order_id: ClientOrderId,
        quantity: OrderQty,
        limit_price: OrderPrice,
        ts_recv_ns: u64,
    ) -> OrderRequest {
        OrderRequest {
            client_order_id,
            account_id: self.account_id,
            route_id,
            strategy_id: self.strategy_id,
            symbol: self.symbol,
            side: self.side,
            order_type: self.order_type,
            time_in_force: self.time_in_force,
            quantity,
            limit_price,
            stop_price: self.stop_price,
            ts_exchange_ns: 0,
            ts_recv_ns,
        }
    }

    fn build_order_request_for_side_at_price(
        &self,
        side: OrderSide,
        client_order_id: ClientOrderId,
        quantity: OrderQty,
        limit_price: OrderPrice,
        ts_recv_ns: u64,
    ) -> OrderRequest {
        OrderRequest {
            client_order_id,
            account_id: self.account_id,
            route_id: self.route_id,
            strategy_id: self.strategy_id,
            symbol: self.symbol,
            side,
            order_type: self.order_type,
            time_in_force: self.time_in_force,
            quantity,
            limit_price,
            stop_price: self.stop_price,
            ts_exchange_ns: 0,
            ts_recv_ns,
        }
    }
}

/// Planned child order generated by an execution algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildOrderPlan {
    child_id: ChildOrderId,
    parent_id: ParentOrderId,
    request: OrderRequest,
    due_ns: u64,
    status: ChildOrderStatus,
}

impl ChildOrderPlan {
    /// Creates a planned child order.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when the generated OMS request is invalid.
    pub fn new(
        child_id: ChildOrderId,
        parent_id: ParentOrderId,
        request: OrderRequest,
        due_ns: u64,
    ) -> Result<Self, AlgoError> {
        request.validate()?;
        Ok(Self {
            child_id,
            parent_id,
            request,
            due_ns,
            status: ChildOrderStatus::Planned,
        })
    }

    /// Returns the child identifier.
    pub const fn child_id(&self) -> ChildOrderId {
        self.child_id
    }

    /// Returns the parent identifier.
    pub const fn parent_id(&self) -> ParentOrderId {
        self.parent_id
    }

    /// Returns the canonical OMS order request.
    pub const fn request(&self) -> &OrderRequest {
        &self.request
    }

    /// Returns the planned release timestamp.
    pub const fn due_ns(&self) -> u64 {
        self.due_ns
    }

    /// Returns the child lifecycle status.
    pub const fn status(&self) -> ChildOrderStatus {
        self.status
    }

    /// Returns a copy with a different lifecycle status.
    pub const fn with_status(mut self, status: ChildOrderStatus) -> Self {
        self.status = status;
        self
    }
}

/// Aggregate parent execution progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoProgress {
    parent_id: ParentOrderId,
    target_qty: OrderQty,
    released_qty: OrderQty,
    completed_qty: OrderQty,
    open_qty: OrderQty,
    rejected_children: u64,
    terminal_children: u64,
}

impl AlgoProgress {
    /// Creates empty progress for a parent order.
    pub const fn new(parent_id: ParentOrderId, target_qty: OrderQty) -> Self {
        Self {
            parent_id,
            target_qty,
            released_qty: OrderQty(0),
            completed_qty: OrderQty(0),
            open_qty: OrderQty(0),
            rejected_children: 0,
            terminal_children: 0,
        }
    }

    /// Returns the parent identifier.
    pub const fn parent_id(&self) -> ParentOrderId {
        self.parent_id
    }

    /// Returns target parent quantity.
    pub const fn target_qty(&self) -> OrderQty {
        self.target_qty
    }

    /// Returns quantity released as child orders.
    pub const fn released_qty(&self) -> OrderQty {
        self.released_qty
    }

    /// Returns quantity fully executed.
    pub const fn completed_qty(&self) -> OrderQty {
        self.completed_qty
    }

    /// Returns currently open child quantity estimate.
    pub const fn open_qty(&self) -> OrderQty {
        self.open_qty
    }

    /// Returns rejected child count.
    pub const fn rejected_children(&self) -> u64 {
        self.rejected_children
    }

    /// Returns terminal child count.
    pub const fn terminal_children(&self) -> u64 {
        self.terminal_children
    }

    /// Returns unreleased target quantity.
    pub const fn unreleased_qty(&self) -> OrderQty {
        OrderQty(self.target_qty.0.saturating_sub(self.released_qty.0))
    }

    /// Returns true when completed quantity reached target quantity.
    pub const fn is_complete(&self) -> bool {
        self.completed_qty.0 >= self.target_qty.0
    }

    /// Records a planned child release.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidProgress`] when the release would exceed the
    /// parent target quantity.
    pub fn on_child_released(&mut self, plan: &ChildOrderPlan) -> Result<(), AlgoError> {
        if plan.parent_id != self.parent_id {
            return Err(AlgoError::InvalidProgress);
        }
        let qty = plan.request.quantity.0;
        let released = self.released_qty.0.saturating_add(qty);
        if released > self.target_qty.0 {
            return Err(AlgoError::InvalidProgress);
        }
        self.released_qty = OrderQty(released);
        self.open_qty = OrderQty(self.open_qty.0.saturating_add(qty));
        Ok(())
    }

    /// Folds a canonical OMS execution event into parent progress.
    pub fn on_execution_event(&mut self, event: &ExecutionEvent) {
        if event.last_qty.0 > 0 {
            self.completed_qty = OrderQty(
                self.completed_qty
                    .0
                    .saturating_add(event.last_qty.0)
                    .min(self.target_qty.0),
            );
            self.open_qty = OrderQty(self.open_qty.0.saturating_sub(event.last_qty.0));
        }
        if event.order_status.is_terminal() {
            self.terminal_children = self.terminal_children.saturating_add(1);
            if matches!(event.order_status, OrderStatus::Rejected) {
                self.rejected_children = self.rejected_children.saturating_add(1);
            }
            self.open_qty = OrderQty(self.open_qty.0.min(event.leaves_qty.0.max(0)));
        }
    }
}

/// Execution-algorithm action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[allow(
    clippy::large_enum_variant,
    reason = "SubmitChild intentionally keeps the action Copy and allocation-free"
)]
pub enum AlgoAction {
    /// Submit a planned child order through the OMS.
    SubmitChild(ChildOrderPlan),
    /// Pause the parent order.
    PauseParent {
        /// Parent to pause.
        parent_id: ParentOrderId,
    },
    /// Mark the parent order complete.
    CompleteParent {
        /// Parent to complete.
        parent_id: ParentOrderId,
    },
    /// Escalate the parent for risk or operator handling.
    EscalateRisk {
        /// Parent requiring attention.
        parent_id: ParentOrderId,
    },
}

/// Fixed-capacity algorithm decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoDecision<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    decision_seq: u64,
    actions: [Option<AlgoAction>; N],
    len: usize,
}

impl<const N: usize> AlgoDecision<N> {
    /// Creates an empty decision.
    pub const fn new(decision_seq: u64) -> Self {
        Self {
            decision_seq,
            actions: [None; N],
            len: 0,
        }
    }

    /// Returns the monotonic decision sequence assigned by the caller.
    pub const fn decision_seq(&self) -> u64 {
        self.decision_seq
    }

    /// Returns the number of retained actions.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no actions are present.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Adds an action to the fixed-capacity decision.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::DecisionFull`] when capacity is exhausted.
    pub fn push(&mut self, action: AlgoAction) -> Result<(), AlgoError> {
        if self.len == N {
            return Err(AlgoError::DecisionFull { capacity: N });
        }
        self.actions[self.len] = Some(action);
        self.len += 1;
        Ok(())
    }

    /// Returns actions in insertion order.
    pub fn actions(&self) -> impl Iterator<Item = &AlgoAction> {
        self.actions[..self.len].iter().filter_map(Option::as_ref)
    }
}

impl<const N: usize> Default for AlgoDecision<N> {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Algorithm risk-policy outcome.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlgoRiskOutcome {
    /// All configured checks passed.
    Allow = 1,
    /// One or more configured limits blocked submission.
    Block = 2,
    /// Kill switch or operator pause requires immediate halt semantics.
    KillSwitch = 3,
}

/// Algorithm risk violation category.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlgoRiskViolationKind {
    /// Kill switch is active.
    KillSwitchActive = 1,
    /// Operator pause is active.
    OperatorPaused = 2,
    /// Parent order is terminal.
    ParentTerminal = 3,
    /// Parent/progress/child relationship is inconsistent.
    InvalidProgress = 4,
    /// Market data is stale.
    StaleMarketData = 5,
    /// Route is degraded.
    RouteDegraded = 6,
    /// Persistence/journaling path is degraded.
    PersistenceDegraded = 7,
    /// Parent quantity exceeds configured maximum.
    ParentQuantityExceeded = 8,
    /// Child quantity exceeds configured maximum.
    ChildQuantityExceeded = 9,
    /// Child notional exceeds configured maximum.
    ChildNotionalExceeded = 10,
    /// Child price is outside configured collar.
    PriceCollarExceeded = 11,
    /// Child would exceed configured participation cap.
    ParticipationExceeded = 12,
    /// Open child quantity would exceed configured limit.
    OpenQuantityExceeded = 13,
    /// Decision contains too many child submissions.
    ChildrenPerDecisionExceeded = 14,
    /// Caller-reported child order rate exceeds configured limit.
    ChildOrderRateExceeded = 15,
    /// Generated child request failed canonical OMS validation.
    InvalidChildPlan = 16,
}

/// One algorithm risk violation retained in a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoRiskViolation {
    kind: AlgoRiskViolationKind,
    child_id: Option<ChildOrderId>,
    measured: u128,
    limit: u128,
}

impl AlgoRiskViolation {
    /// Creates a violation.
    pub const fn new(
        kind: AlgoRiskViolationKind,
        child_id: Option<ChildOrderId>,
        measured: u128,
        limit: u128,
    ) -> Self {
        Self {
            kind,
            child_id,
            measured,
            limit,
        }
    }

    /// Returns violation category.
    pub const fn kind(&self) -> AlgoRiskViolationKind {
        self.kind
    }

    /// Returns associated child identifier when the violation is child-specific.
    pub const fn child_id(&self) -> Option<ChildOrderId> {
        self.child_id
    }

    /// Returns measured value.
    pub const fn measured(&self) -> u128 {
        self.measured
    }

    /// Returns configured limit.
    pub const fn limit(&self) -> u128 {
        self.limit
    }
}

/// Algorithm risk limits.
///
/// Zero-valued limits are disabled, except basis-point limits where `10_000`
/// means 100 percent. This keeps one struct usable for schedule-driven and
/// latency-sensitive planners without forcing all hosts to configure every
/// possible control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoRiskLimits {
    max_parent_qty: OrderQty,
    max_child_qty: OrderQty,
    max_child_notional: u128,
    max_participation_bps: u16,
    price_collar_bps: u16,
    max_open_qty: OrderQty,
    max_children_per_decision: u16,
    max_child_orders_in_window: u32,
}

impl AlgoRiskLimits {
    /// Creates algorithm risk limits.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidRiskParameters`] when quantities are
    /// negative or basis-point limits exceed `10_000`.
    #[allow(
        clippy::too_many_arguments,
        reason = "flat risk limit constructor keeps every limit explicit"
    )]
    pub const fn new(
        max_parent_qty: OrderQty,
        max_child_qty: OrderQty,
        max_child_notional: u128,
        max_participation_bps: u16,
        price_collar_bps: u16,
        max_open_qty: OrderQty,
        max_children_per_decision: u16,
        max_child_orders_in_window: u32,
    ) -> Result<Self, AlgoError> {
        if max_parent_qty.0 < 0
            || max_child_qty.0 < 0
            || max_open_qty.0 < 0
            || max_participation_bps > 10_000
            || price_collar_bps > 10_000
        {
            return Err(AlgoError::InvalidRiskParameters);
        }
        Ok(Self {
            max_parent_qty,
            max_child_qty,
            max_child_notional,
            max_participation_bps,
            price_collar_bps,
            max_open_qty,
            max_children_per_decision,
            max_child_orders_in_window,
        })
    }

    /// Returns a policy with all optional limits disabled.
    pub const fn unbounded() -> Self {
        Self {
            max_parent_qty: OrderQty(0),
            max_child_qty: OrderQty(0),
            max_child_notional: 0,
            max_participation_bps: 0,
            price_collar_bps: 0,
            max_open_qty: OrderQty(0),
            max_children_per_decision: 0,
            max_child_orders_in_window: 0,
        }
    }

    /// Returns maximum parent quantity, or zero when disabled.
    pub const fn max_parent_qty(&self) -> OrderQty {
        self.max_parent_qty
    }

    /// Returns maximum child quantity, or zero when disabled.
    pub const fn max_child_qty(&self) -> OrderQty {
        self.max_child_qty
    }

    /// Returns maximum child notional, or zero when disabled.
    pub const fn max_child_notional(&self) -> u128 {
        self.max_child_notional
    }

    /// Returns maximum participation in basis points, or zero when disabled.
    pub const fn max_participation_bps(&self) -> u16 {
        self.max_participation_bps
    }

    /// Returns price collar in basis points, or zero when disabled.
    pub const fn price_collar_bps(&self) -> u16 {
        self.price_collar_bps
    }

    /// Returns maximum open child quantity, or zero when disabled.
    pub const fn max_open_qty(&self) -> OrderQty {
        self.max_open_qty
    }

    /// Returns maximum child submissions per decision, or zero when disabled.
    pub const fn max_children_per_decision(&self) -> u16 {
        self.max_children_per_decision
    }

    /// Returns maximum child submissions in the caller's rate window, or zero
    /// when disabled.
    pub const fn max_child_orders_in_window(&self) -> u32 {
        self.max_child_orders_in_window
    }
}

impl Default for AlgoRiskLimits {
    fn default() -> Self {
        Self::unbounded()
    }
}

/// Host-supplied risk context for one algorithm decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoRiskContext {
    reference_price: OrderPrice,
    observed_market_volume: OrderQty,
    open_child_orders: u32,
    child_orders_in_window: u32,
    stale_market_data: bool,
    route_degraded: bool,
    persistence_degraded: bool,
    kill_switch_active: bool,
    operator_paused: bool,
}

impl AlgoRiskContext {
    /// Creates risk context around a positive reference price.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidRiskParameters`] when the reference price is
    /// not positive.
    pub const fn new(reference_price: OrderPrice) -> Result<Self, AlgoError> {
        if reference_price.0 <= 0 {
            return Err(AlgoError::InvalidRiskParameters);
        }
        Ok(Self {
            reference_price,
            observed_market_volume: OrderQty(0),
            open_child_orders: 0,
            child_orders_in_window: 0,
            stale_market_data: false,
            route_degraded: false,
            persistence_degraded: false,
            kill_switch_active: false,
            operator_paused: false,
        })
    }

    /// Returns the reference price used for collar checks.
    pub const fn reference_price(&self) -> OrderPrice {
        self.reference_price
    }

    /// Returns observed market volume used for participation checks.
    pub const fn observed_market_volume(&self) -> OrderQty {
        self.observed_market_volume
    }

    /// Returns currently open child order count supplied by the host.
    pub const fn open_child_orders(&self) -> u32 {
        self.open_child_orders
    }

    /// Returns child order count in the caller's rate-limit window.
    pub const fn child_orders_in_window(&self) -> u32 {
        self.child_orders_in_window
    }

    /// Returns true when market data should block child release.
    pub const fn stale_market_data(&self) -> bool {
        self.stale_market_data
    }

    /// Returns true when route degradation should block child release.
    pub const fn route_degraded(&self) -> bool {
        self.route_degraded
    }

    /// Returns true when persistence degradation should block child release.
    pub const fn persistence_degraded(&self) -> bool {
        self.persistence_degraded
    }

    /// Returns true when kill switch is active.
    pub const fn kill_switch_active(&self) -> bool {
        self.kill_switch_active
    }

    /// Returns true when operator pause is active.
    pub const fn operator_paused(&self) -> bool {
        self.operator_paused
    }

    /// Returns a copy with observed market volume.
    pub const fn with_observed_market_volume(mut self, volume: OrderQty) -> Self {
        self.observed_market_volume = volume;
        self
    }

    /// Returns a copy with currently open child order count.
    pub const fn with_open_child_orders(mut self, count: u32) -> Self {
        self.open_child_orders = count;
        self
    }

    /// Returns a copy with child order count in the caller's rate window.
    pub const fn with_child_orders_in_window(mut self, count: u32) -> Self {
        self.child_orders_in_window = count;
        self
    }

    /// Returns a copy with stale market-data flag.
    pub const fn with_stale_market_data(mut self, value: bool) -> Self {
        self.stale_market_data = value;
        self
    }

    /// Returns a copy with route-degraded flag.
    pub const fn with_route_degraded(mut self, value: bool) -> Self {
        self.route_degraded = value;
        self
    }

    /// Returns a copy with persistence-degraded flag.
    pub const fn with_persistence_degraded(mut self, value: bool) -> Self {
        self.persistence_degraded = value;
        self
    }

    /// Returns a copy with kill-switch flag.
    pub const fn with_kill_switch_active(mut self, value: bool) -> Self {
        self.kill_switch_active = value;
        self
    }

    /// Returns a copy with operator-pause flag.
    pub const fn with_operator_paused(mut self, value: bool) -> Self {
        self.operator_paused = value;
        self
    }
}

/// Fixed-capacity risk report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoRiskReport<const N: usize = DEFAULT_ALGO_RISK_VIOLATION_CAPACITY> {
    outcome: AlgoRiskOutcome,
    violations: [Option<AlgoRiskViolation>; N],
    len: usize,
    truncated: bool,
}

impl<const N: usize> AlgoRiskReport<N> {
    /// Creates an empty allow report.
    pub const fn new() -> Self {
        Self {
            outcome: AlgoRiskOutcome::Allow,
            violations: [None; N],
            len: 0,
            truncated: false,
        }
    }

    /// Returns risk outcome.
    pub const fn outcome(&self) -> AlgoRiskOutcome {
        self.outcome
    }

    /// Returns true when submission is allowed.
    pub const fn is_allowed(&self) -> bool {
        matches!(self.outcome, AlgoRiskOutcome::Allow)
    }

    /// Returns retained violation count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no retained violations are present.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns true when more violations occurred than the report retained.
    pub const fn truncated(&self) -> bool {
        self.truncated
    }

    /// Returns retained violations in insertion order.
    pub fn violations(&self) -> impl Iterator<Item = &AlgoRiskViolation> {
        self.violations[..self.len]
            .iter()
            .filter_map(Option::as_ref)
    }

    /// Returns first retained violation.
    pub fn first_violation(&self) -> Option<&AlgoRiskViolation> {
        self.violations().next()
    }

    fn push(&mut self, violation: AlgoRiskViolation) {
        if self.len == N {
            self.truncated = true;
            self.outcome = stronger_risk_outcome(self.outcome, violation.kind());
            return;
        }
        self.outcome = stronger_risk_outcome(self.outcome, violation.kind());
        self.violations[self.len] = Some(violation);
        self.len += 1;
    }
}

impl<const N: usize> Default for AlgoRiskReport<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Additive algorithm risk policy for validating child plans before OMS submit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoRiskPolicy {
    limits: AlgoRiskLimits,
}

impl AlgoRiskPolicy {
    /// Creates a policy from limits.
    pub const fn new(limits: AlgoRiskLimits) -> Self {
        Self { limits }
    }

    /// Returns configured limits.
    pub const fn limits(&self) -> AlgoRiskLimits {
        self.limits
    }

    /// Evaluates one planned child order.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent, progress, or child state is invalid.
    pub fn evaluate_child<const N: usize>(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        child: &ChildOrderPlan,
        context: AlgoRiskContext,
    ) -> Result<AlgoRiskReport<N>, AlgoError> {
        parent.validate()?;
        let mut report = AlgoRiskReport::new();
        self.check_static_context(&mut report, parent, progress)?;
        self.check_dynamic_context(&mut report, context);
        self.check_child(&mut report, parent, progress, child, context, 1)?;
        Ok(report)
    }

    /// Evaluates all child submissions inside one [`AlgoDecision`].
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent, progress, or child state is invalid.
    pub fn evaluate_decision<const REPORT_N: usize, const DECISION_N: usize>(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        decision: &AlgoDecision<DECISION_N>,
        context: AlgoRiskContext,
    ) -> Result<AlgoRiskReport<REPORT_N>, AlgoError> {
        parent.validate()?;
        let mut report = AlgoRiskReport::new();
        self.check_static_context(&mut report, parent, progress)?;
        self.check_dynamic_context(&mut report, context);

        let mut child_count = 0_u16;
        let mut planned_open_qty = progress.open_qty().0;
        for action in decision.actions() {
            if let AlgoAction::SubmitChild(child) = action {
                child_count = child_count.saturating_add(1);
                self.check_child(&mut report, parent, progress, child, context, child_count)?;
                planned_open_qty = planned_open_qty.saturating_add(child.request().quantity.0);
            }
        }

        self.check_open_qty(&mut report, planned_open_qty, None);
        Ok(report)
    }

    fn check_static_context<const N: usize>(
        &self,
        report: &mut AlgoRiskReport<N>,
        parent: &ParentOrder,
        progress: AlgoProgress,
    ) -> Result<(), AlgoError> {
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::InvalidProgress,
                None,
                1,
                0,
            ));
            return Err(AlgoError::InvalidProgress);
        }
        if parent.status().is_terminal() {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::ParentTerminal,
                None,
                u128::from(parent.status() as u8),
                0,
            ));
        }
        if self.limits.max_parent_qty().0 > 0
            && parent.total_qty().0 > self.limits.max_parent_qty().0
        {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::ParentQuantityExceeded,
                None,
                i64_to_u128(parent.total_qty().0),
                i64_to_u128(self.limits.max_parent_qty().0),
            ));
        }
        Ok(())
    }

    fn check_dynamic_context<const N: usize>(
        &self,
        report: &mut AlgoRiskReport<N>,
        context: AlgoRiskContext,
    ) {
        if context.kill_switch_active() {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::KillSwitchActive,
                None,
                1,
                0,
            ));
        }
        if context.operator_paused() {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::OperatorPaused,
                None,
                1,
                0,
            ));
        }
        if context.stale_market_data() {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::StaleMarketData,
                None,
                1,
                0,
            ));
        }
        if context.route_degraded() {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::RouteDegraded,
                None,
                1,
                0,
            ));
        }
        if context.persistence_degraded() {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::PersistenceDegraded,
                None,
                1,
                0,
            ));
        }
    }

    fn check_child<const N: usize>(
        &self,
        report: &mut AlgoRiskReport<N>,
        parent: &ParentOrder,
        progress: AlgoProgress,
        child: &ChildOrderPlan,
        context: AlgoRiskContext,
        child_count: u16,
    ) -> Result<(), AlgoError> {
        if child.parent_id() != parent.id() {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::InvalidProgress,
                Some(child.child_id()),
                1,
                0,
            ));
            return Err(AlgoError::InvalidProgress);
        }
        if let Err(err) = child.request().validate() {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::InvalidChildPlan,
                Some(child.child_id()),
                1,
                0,
            ));
            return Err(AlgoError::Core(err));
        }

        let quantity = child.request().quantity.0;
        if self.limits.max_child_qty().0 > 0 && quantity > self.limits.max_child_qty().0 {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::ChildQuantityExceeded,
                Some(child.child_id()),
                i64_to_u128(quantity),
                i64_to_u128(self.limits.max_child_qty().0),
            ));
        }

        let notional = child_notional(child);
        if self.limits.max_child_notional() > 0 && notional > self.limits.max_child_notional() {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::ChildNotionalExceeded,
                Some(child.child_id()),
                notional,
                self.limits.max_child_notional(),
            ));
        }

        let price_distance =
            price_distance_bps(child.request().limit_price, context.reference_price());
        if self.limits.price_collar_bps() > 0
            && price_distance > u32::from(self.limits.price_collar_bps())
        {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::PriceCollarExceeded,
                Some(child.child_id()),
                u128::from(price_distance),
                u128::from(self.limits.price_collar_bps()),
            ));
        }

        if self.limits.max_participation_bps() > 0 && context.observed_market_volume().0 > 0 {
            let participation = participation_bps(quantity, context.observed_market_volume().0);
            if participation > u32::from(self.limits.max_participation_bps()) {
                report.push(AlgoRiskViolation::new(
                    AlgoRiskViolationKind::ParticipationExceeded,
                    Some(child.child_id()),
                    u128::from(participation),
                    u128::from(self.limits.max_participation_bps()),
                ));
            }
        }

        self.check_child_count(report, child_count, context);
        self.check_open_qty(
            report,
            progress.open_qty().0.saturating_add(quantity),
            Some(child.child_id()),
        );
        Ok(())
    }

    fn check_child_count<const N: usize>(
        &self,
        report: &mut AlgoRiskReport<N>,
        child_count: u16,
        context: AlgoRiskContext,
    ) {
        if self.limits.max_children_per_decision() > 0
            && child_count > self.limits.max_children_per_decision()
        {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::ChildrenPerDecisionExceeded,
                None,
                u128::from(child_count),
                u128::from(self.limits.max_children_per_decision()),
            ));
        }
        if self.limits.max_child_orders_in_window() > 0 {
            let projected = context
                .child_orders_in_window()
                .saturating_add(u32::from(child_count));
            if projected > self.limits.max_child_orders_in_window() {
                report.push(AlgoRiskViolation::new(
                    AlgoRiskViolationKind::ChildOrderRateExceeded,
                    None,
                    u128::from(projected),
                    u128::from(self.limits.max_child_orders_in_window()),
                ));
            }
        }
    }

    fn check_open_qty<const N: usize>(
        &self,
        report: &mut AlgoRiskReport<N>,
        open_qty: i64,
        child_id: Option<ChildOrderId>,
    ) {
        if self.limits.max_open_qty().0 > 0 && open_qty > self.limits.max_open_qty().0 {
            report.push(AlgoRiskViolation::new(
                AlgoRiskViolationKind::OpenQuantityExceeded,
                child_id,
                i64_to_u128(open_qty),
                i64_to_u128(self.limits.max_open_qty().0),
            ));
        }
    }
}

impl Default for AlgoRiskPolicy {
    fn default() -> Self {
        Self::new(AlgoRiskLimits::default())
    }
}

/// Recovery action recommended for an algorithm instance.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlgoRecoveryAction {
    /// Resume planning after host reconciliation.
    Resume = 1,
    /// Keep the parent paused until an operator or host policy resumes it.
    Pause = 2,
    /// Mark parent complete because recovered progress reached target.
    CompleteParent = 3,
    /// Escalate for risk/operator handling.
    EscalateRisk = 4,
}

/// Deterministic checkpoint for one algorithm parent instance.
///
/// This type intentionally stores only algorithm-owned state. OMS order
/// journals, adapter sequence state, venue order ids, and fill reconciliation
/// remain owned by `of_execution` and adapter-specific recovery flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoCheckpoint {
    schema_version: u16,
    parent: ParentOrder,
    progress: AlgoProgress,
    next_decision_seq: u64,
    last_input_sequence: u64,
}

impl AlgoCheckpoint {
    /// Creates an algorithm checkpoint.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidRecoveryState`] when parent/progress state
    /// is inconsistent or sequence counters are invalid.
    pub fn new(
        parent: ParentOrder,
        progress: AlgoProgress,
        next_decision_seq: u64,
        last_input_sequence: u64,
    ) -> Result<Self, AlgoError> {
        parent.validate()?;
        if progress.parent_id() != parent.id()
            || progress.target_qty() != parent.total_qty()
            || progress.released_qty().0 > parent.total_qty().0
            || progress.completed_qty().0 > parent.total_qty().0
            || progress.open_qty().0 > progress.released_qty().0
            || next_decision_seq == 0
        {
            return Err(AlgoError::InvalidRecoveryState);
        }
        Ok(Self {
            schema_version: ALGO_CHECKPOINT_SCHEMA_VERSION,
            parent,
            progress,
            next_decision_seq,
            last_input_sequence,
        })
    }

    /// Returns checkpoint schema version.
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// Returns checkpointed parent order.
    pub const fn parent(&self) -> ParentOrder {
        self.parent
    }

    /// Returns checkpointed progress.
    pub const fn progress(&self) -> AlgoProgress {
        self.progress
    }

    /// Returns next decision sequence to assign after recovery.
    pub const fn next_decision_seq(&self) -> u64 {
        self.next_decision_seq
    }

    /// Returns last consumed input sequence.
    pub const fn last_input_sequence(&self) -> u64 {
        self.last_input_sequence
    }
}

/// Algorithm recovery policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoRecoveryPolicy {
    pause_on_recovery: bool,
    require_reconciliation: bool,
    complete_when_progress_complete: bool,
}

impl AlgoRecoveryPolicy {
    /// Creates recovery policy.
    pub const fn new(
        pause_on_recovery: bool,
        require_reconciliation: bool,
        complete_when_progress_complete: bool,
    ) -> Self {
        Self {
            pause_on_recovery,
            require_reconciliation,
            complete_when_progress_complete,
        }
    }

    /// Returns true when recovered parents should pause by default.
    pub const fn pause_on_recovery(&self) -> bool {
        self.pause_on_recovery
    }

    /// Returns true when OMS/venue reconciliation is required before resume.
    pub const fn require_reconciliation(&self) -> bool {
        self.require_reconciliation
    }

    /// Returns true when complete recovered progress should complete parent.
    pub const fn complete_when_progress_complete(&self) -> bool {
        self.complete_when_progress_complete
    }

    /// Returns a copy with pause-on-recovery behavior changed.
    pub const fn with_pause_on_recovery(mut self, value: bool) -> Self {
        self.pause_on_recovery = value;
        self
    }

    /// Returns a copy with reconciliation requirement changed.
    pub const fn with_require_reconciliation(mut self, value: bool) -> Self {
        self.require_reconciliation = value;
        self
    }

    /// Returns a copy with complete-on-recovered-progress behavior changed.
    pub const fn with_complete_when_progress_complete(mut self, value: bool) -> Self {
        self.complete_when_progress_complete = value;
        self
    }
}

impl Default for AlgoRecoveryPolicy {
    fn default() -> Self {
        Self::new(true, true, true)
    }
}

/// Deterministic recovery plan derived from a checkpoint and policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoRecoveryPlan {
    checkpoint: AlgoCheckpoint,
    action: AlgoRecoveryAction,
    replay_from_sequence: u64,
    next_decision_seq: u64,
    reconciliation_required: bool,
}

impl AlgoRecoveryPlan {
    /// Builds a recovery plan.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidRecoveryState`] when checkpoint state is not
    /// usable for deterministic recovery.
    pub fn new(checkpoint: AlgoCheckpoint, policy: AlgoRecoveryPolicy) -> Result<Self, AlgoError> {
        if checkpoint.schema_version() != ALGO_CHECKPOINT_SCHEMA_VERSION
            || checkpoint.next_decision_seq() == 0
        {
            return Err(AlgoError::InvalidRecoveryState);
        }
        let parent = checkpoint.parent();
        let progress = checkpoint.progress();
        let reconciliation_required = policy.require_reconciliation();
        let action = if policy.complete_when_progress_complete() && progress.is_complete() {
            AlgoRecoveryAction::CompleteParent
        } else if parent.status().is_terminal() {
            AlgoRecoveryAction::EscalateRisk
        } else if policy.pause_on_recovery() || reconciliation_required {
            AlgoRecoveryAction::Pause
        } else {
            AlgoRecoveryAction::Resume
        };
        Ok(Self {
            checkpoint,
            action,
            replay_from_sequence: checkpoint.last_input_sequence().saturating_add(1),
            next_decision_seq: checkpoint.next_decision_seq(),
            reconciliation_required,
        })
    }

    /// Returns the source checkpoint.
    pub const fn checkpoint(&self) -> AlgoCheckpoint {
        self.checkpoint
    }

    /// Returns recommended recovery action.
    pub const fn action(&self) -> AlgoRecoveryAction {
        self.action
    }

    /// Returns first input sequence to replay after the checkpoint.
    pub const fn replay_from_sequence(&self) -> u64 {
        self.replay_from_sequence
    }

    /// Returns next decision sequence to assign.
    pub const fn next_decision_seq(&self) -> u64 {
        self.next_decision_seq
    }

    /// Returns true when OMS/venue reconciliation should gate resume.
    pub const fn reconciliation_required(&self) -> bool {
        self.reconciliation_required
    }
}

/// Deterministic child-order simulation outcome.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlgoSimOutcome {
    /// Child was fully filled.
    Filled = 1,
    /// Child was partially filled and remaining quantity stays open.
    PartiallyFilled = 2,
    /// Child was rejected.
    Rejected = 3,
    /// Child was partially filled and remaining quantity was cancelled.
    CancelledRemainder = 4,
    /// Child did not fill and remains resting.
    Resting = 5,
}

/// Deterministic market/fill model for one simulation pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoSimMarket {
    available_qty: OrderQty,
    fill_price: OrderPrice,
    reject: bool,
    cancel_unfilled: bool,
    latency_ns: u64,
}

impl AlgoSimMarket {
    /// Creates a simulation market model.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSimulationParameters`] when quantities are
    /// negative or a non-rejecting model has a non-positive fill price.
    pub const fn new(
        available_qty: OrderQty,
        fill_price: OrderPrice,
        reject: bool,
        cancel_unfilled: bool,
        latency_ns: u64,
    ) -> Result<Self, AlgoError> {
        if available_qty.0 < 0 || (!reject && fill_price.0 <= 0) {
            return Err(AlgoError::InvalidSimulationParameters);
        }
        Ok(Self {
            available_qty,
            fill_price,
            reject,
            cancel_unfilled,
            latency_ns,
        })
    }

    /// Returns simulated available quantity.
    pub const fn available_qty(&self) -> OrderQty {
        self.available_qty
    }

    /// Returns simulated fill price.
    pub const fn fill_price(&self) -> OrderPrice {
        self.fill_price
    }

    /// Returns true when children should be rejected.
    pub const fn reject(&self) -> bool {
        self.reject
    }

    /// Returns true when unfilled leaves should be cancelled.
    pub const fn cancel_unfilled(&self) -> bool {
        self.cancel_unfilled
    }

    /// Returns simulated exchange-to-receive latency in nanoseconds.
    pub const fn latency_ns(&self) -> u64 {
        self.latency_ns
    }
}

/// One simulated child-order result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoSimStep {
    sequence: u64,
    child_id: ChildOrderId,
    outcome: AlgoSimOutcome,
    filled_qty: OrderQty,
    leaves_qty: OrderQty,
    fill_price: OrderPrice,
    event: ExecutionEvent,
}

impl AlgoSimStep {
    /// Returns simulation sequence.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Returns simulated child identifier.
    pub const fn child_id(&self) -> ChildOrderId {
        self.child_id
    }

    /// Returns simulation outcome.
    pub const fn outcome(&self) -> AlgoSimOutcome {
        self.outcome
    }

    /// Returns simulated filled quantity.
    pub const fn filled_qty(&self) -> OrderQty {
        self.filled_qty
    }

    /// Returns simulated leaves quantity.
    pub const fn leaves_qty(&self) -> OrderQty {
        self.leaves_qty
    }

    /// Returns simulated fill price.
    pub const fn fill_price(&self) -> OrderPrice {
        self.fill_price
    }

    /// Returns canonical simulated execution event.
    pub const fn event(&self) -> ExecutionEvent {
        self.event
    }
}

/// Fixed-capacity algorithm simulation report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoSimReport<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    steps: [Option<AlgoSimStep>; N],
    len: usize,
    truncated: bool,
    total_filled_qty: OrderQty,
    rejected_children: u64,
    cancelled_children: u64,
}

impl<const N: usize> AlgoSimReport<N> {
    /// Creates an empty report.
    pub const fn new() -> Self {
        Self {
            steps: [None; N],
            len: 0,
            truncated: false,
            total_filled_qty: OrderQty(0),
            rejected_children: 0,
            cancelled_children: 0,
        }
    }

    /// Returns retained simulation step count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no steps are retained.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns true when more child submissions existed than report capacity.
    pub const fn truncated(&self) -> bool {
        self.truncated
    }

    /// Returns total simulated filled quantity.
    pub const fn total_filled_qty(&self) -> OrderQty {
        self.total_filled_qty
    }

    /// Returns rejected child count.
    pub const fn rejected_children(&self) -> u64 {
        self.rejected_children
    }

    /// Returns cancelled child count.
    pub const fn cancelled_children(&self) -> u64 {
        self.cancelled_children
    }

    /// Returns simulation steps in insertion order.
    pub fn steps(&self) -> impl Iterator<Item = &AlgoSimStep> {
        self.steps[..self.len].iter().filter_map(Option::as_ref)
    }

    fn push(&mut self, step: AlgoSimStep) {
        self.total_filled_qty =
            OrderQty(self.total_filled_qty.0.saturating_add(step.filled_qty().0));
        if matches!(step.outcome(), AlgoSimOutcome::Rejected) {
            self.rejected_children = self.rejected_children.saturating_add(1);
        }
        if matches!(step.outcome(), AlgoSimOutcome::CancelledRemainder) {
            self.cancelled_children = self.cancelled_children.saturating_add(1);
        }
        if self.len == N {
            self.truncated = true;
            return;
        }
        self.steps[self.len] = Some(step);
        self.len += 1;
    }
}

impl<const N: usize> Default for AlgoSimReport<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic simulator for generated child plans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoSimulator {
    market: AlgoSimMarket,
}

impl AlgoSimulator {
    /// Creates a simulator.
    pub const fn new(market: AlgoSimMarket) -> Self {
        Self { market }
    }

    /// Returns the market model.
    pub const fn market(&self) -> AlgoSimMarket {
        self.market
    }

    /// Simulates one child plan.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when deterministic simulator identifiers exceed
    /// fixed identifier capacity.
    pub fn simulate_child(
        &self,
        child: &ChildOrderPlan,
        sequence: u64,
    ) -> Result<AlgoSimStep, AlgoError> {
        let request = child.request();
        let venue_order_id: VenueOrderId = fixed_id_with_index("sim-venue", sequence)?;
        let execution_id: ExecutionId = fixed_id_with_index("sim-exec", sequence)?;
        if self.market.reject() {
            let event = ExecutionEvent {
                exec_type: ExecutionType::Reject,
                order_status: OrderStatus::Rejected,
                client_order_id: request.client_order_id,
                orig_client_order_id: ClientOrderId::empty(),
                venue_order_id,
                execution_id,
                account_id: request.account_id,
                route_id: request.route_id,
                symbol: request.symbol,
                last_qty: OrderQty(0),
                last_price: OrderPrice(0),
                cumulative_qty: OrderQty(0),
                leaves_qty: request.quantity,
                average_price: OrderPrice(0),
                ts_exchange_ns: request.ts_recv_ns,
                ts_recv_ns: request.ts_recv_ns.saturating_add(self.market.latency_ns()),
                reason: RiskRejectReason::PriceBand,
                text: ExecutionText::empty(),
            };
            return Ok(AlgoSimStep {
                sequence,
                child_id: child.child_id(),
                outcome: AlgoSimOutcome::Rejected,
                filled_qty: OrderQty(0),
                leaves_qty: request.quantity,
                fill_price: OrderPrice(0),
                event,
            });
        }

        let filled_qty = request.quantity.0.min(self.market.available_qty().0).max(0);
        let leaves_qty = request.quantity.0.saturating_sub(filled_qty);
        let (outcome, status, exec_type) =
            match (filled_qty, leaves_qty, self.market.cancel_unfilled()) {
                (0, _, false) => (
                    AlgoSimOutcome::Resting,
                    OrderStatus::New,
                    ExecutionType::Ack,
                ),
                (0, _, true) => (
                    AlgoSimOutcome::CancelledRemainder,
                    OrderStatus::Cancelled,
                    ExecutionType::CancelAck,
                ),
                (_, 0, _) => (
                    AlgoSimOutcome::Filled,
                    OrderStatus::Filled,
                    ExecutionType::Trade,
                ),
                (_, _, true) => (
                    AlgoSimOutcome::CancelledRemainder,
                    OrderStatus::Cancelled,
                    ExecutionType::Trade,
                ),
                _ => (
                    AlgoSimOutcome::PartiallyFilled,
                    OrderStatus::PartiallyFilled,
                    ExecutionType::Trade,
                ),
            };
        let fill_price = if filled_qty > 0 {
            self.market.fill_price()
        } else {
            OrderPrice(0)
        };
        let event = ExecutionEvent {
            exec_type,
            order_status: status,
            client_order_id: request.client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id,
            execution_id,
            account_id: request.account_id,
            route_id: request.route_id,
            symbol: request.symbol,
            last_qty: OrderQty(filled_qty),
            last_price: fill_price,
            cumulative_qty: OrderQty(filled_qty),
            leaves_qty: OrderQty(leaves_qty),
            average_price: fill_price,
            ts_exchange_ns: request.ts_recv_ns,
            ts_recv_ns: request.ts_recv_ns.saturating_add(self.market.latency_ns()),
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        };
        Ok(AlgoSimStep {
            sequence,
            child_id: child.child_id(),
            outcome,
            filled_qty: OrderQty(filled_qty),
            leaves_qty: OrderQty(leaves_qty),
            fill_price,
            event,
        })
    }

    /// Simulates every child submission in an algorithm decision.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when deterministic simulator identifiers exceed
    /// fixed identifier capacity.
    pub fn simulate_decision<const REPORT_N: usize, const DECISION_N: usize>(
        &self,
        decision: &AlgoDecision<DECISION_N>,
        first_sequence: u64,
    ) -> Result<AlgoSimReport<REPORT_N>, AlgoError> {
        let mut report = AlgoSimReport::new();
        let mut sequence = first_sequence;
        for action in decision.actions() {
            if let AlgoAction::SubmitChild(child) = action {
                let step = self.simulate_child(child, sequence)?;
                report.push(step);
                sequence = sequence.saturating_add(1);
            }
        }
        Ok(report)
    }
}

/// Optional TCA benchmark prices for an algorithm parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoTcaBenchmark {
    arrival_price: OrderPrice,
    vwap_price: OrderPrice,
    twap_price: OrderPrice,
}

impl AlgoTcaBenchmark {
    /// Creates benchmark prices.
    ///
    /// `vwap_price` and `twap_price` may be zero when unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidMetricsParameters`] when arrival price is
    /// not positive or optional benchmark prices are negative.
    pub const fn new(
        arrival_price: OrderPrice,
        vwap_price: OrderPrice,
        twap_price: OrderPrice,
    ) -> Result<Self, AlgoError> {
        if arrival_price.0 <= 0 || vwap_price.0 < 0 || twap_price.0 < 0 {
            return Err(AlgoError::InvalidMetricsParameters);
        }
        Ok(Self {
            arrival_price,
            vwap_price,
            twap_price,
        })
    }

    /// Returns arrival/decision price.
    pub const fn arrival_price(&self) -> OrderPrice {
        self.arrival_price
    }

    /// Returns VWAP benchmark price, or zero when unavailable.
    pub const fn vwap_price(&self) -> OrderPrice {
        self.vwap_price
    }

    /// Returns TWAP benchmark price, or zero when unavailable.
    pub const fn twap_price(&self) -> OrderPrice {
        self.twap_price
    }
}

/// Snapshot of algorithm execution metrics and TCA fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoMetricsSnapshot {
    parent_id: ParentOrderId,
    target_qty: OrderQty,
    submitted_children: u64,
    filled_children: u64,
    rejected_children: u64,
    cancelled_children: u64,
    completed_qty: OrderQty,
    completion_bps: u16,
    average_price: OrderPrice,
    arrival_slippage_bps: i32,
    vwap_slippage_bps: i32,
    twap_slippage_bps: i32,
    first_submit_ns: u64,
    last_event_ns: u64,
    average_latency_ns: u64,
}

impl AlgoMetricsSnapshot {
    /// Returns parent identifier.
    pub const fn parent_id(&self) -> ParentOrderId {
        self.parent_id
    }

    /// Returns target parent quantity.
    pub const fn target_qty(&self) -> OrderQty {
        self.target_qty
    }

    /// Returns submitted child count.
    pub const fn submitted_children(&self) -> u64 {
        self.submitted_children
    }

    /// Returns child count with at least one fill.
    pub const fn filled_children(&self) -> u64 {
        self.filled_children
    }

    /// Returns rejected child count.
    pub const fn rejected_children(&self) -> u64 {
        self.rejected_children
    }

    /// Returns cancelled child count.
    pub const fn cancelled_children(&self) -> u64 {
        self.cancelled_children
    }

    /// Returns completed quantity.
    pub const fn completed_qty(&self) -> OrderQty {
        self.completed_qty
    }

    /// Returns completion in basis points of target quantity.
    pub const fn completion_bps(&self) -> u16 {
        self.completion_bps
    }

    /// Returns average execution price, or zero when no fills exist.
    pub const fn average_price(&self) -> OrderPrice {
        self.average_price
    }

    /// Returns side-aware arrival slippage in basis points.
    pub const fn arrival_slippage_bps(&self) -> i32 {
        self.arrival_slippage_bps
    }

    /// Returns side-aware VWAP benchmark slippage in basis points, or zero when
    /// no VWAP benchmark is configured.
    pub const fn vwap_slippage_bps(&self) -> i32 {
        self.vwap_slippage_bps
    }

    /// Returns side-aware TWAP benchmark slippage in basis points, or zero when
    /// no TWAP benchmark is configured.
    pub const fn twap_slippage_bps(&self) -> i32 {
        self.twap_slippage_bps
    }

    /// Returns first child submit timestamp.
    pub const fn first_submit_ns(&self) -> u64 {
        self.first_submit_ns
    }

    /// Returns last execution-event receive timestamp.
    pub const fn last_event_ns(&self) -> u64 {
        self.last_event_ns
    }

    /// Returns average event latency in nanoseconds.
    pub const fn average_latency_ns(&self) -> u64 {
        self.average_latency_ns
    }
}

/// Allocation-free accumulator for algo execution metrics and TCA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoMetricsAccumulator {
    parent_id: ParentOrderId,
    side: OrderSide,
    target_qty: OrderQty,
    benchmarks: AlgoTcaBenchmark,
    submitted_children: u64,
    filled_children: u64,
    rejected_children: u64,
    cancelled_children: u64,
    completed_qty: OrderQty,
    cumulative_notional: u128,
    first_submit_ns: u64,
    last_event_ns: u64,
    total_latency_ns: u128,
    latency_samples: u64,
}

impl AlgoMetricsAccumulator {
    /// Creates a metrics accumulator for a parent order.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent or benchmark inputs are invalid.
    pub fn new(parent: &ParentOrder, benchmarks: AlgoTcaBenchmark) -> Result<Self, AlgoError> {
        parent.validate()?;
        Ok(Self {
            parent_id: parent.id(),
            side: parent.side(),
            target_qty: parent.total_qty(),
            benchmarks,
            submitted_children: 0,
            filled_children: 0,
            rejected_children: 0,
            cancelled_children: 0,
            completed_qty: OrderQty(0),
            cumulative_notional: 0,
            first_submit_ns: 0,
            last_event_ns: 0,
            total_latency_ns: 0,
            latency_samples: 0,
        })
    }

    /// Records one child submission.
    pub fn on_child_submitted(&mut self, child: &ChildOrderPlan) {
        self.submitted_children = self.submitted_children.saturating_add(1);
        let ts = child.request().ts_recv_ns;
        if self.first_submit_ns == 0 || ts < self.first_submit_ns {
            self.first_submit_ns = ts;
        }
    }

    /// Folds one canonical execution event into metrics.
    pub fn on_execution_event(&mut self, event: &ExecutionEvent) {
        self.last_event_ns = self.last_event_ns.max(event.ts_recv_ns);
        if event.ts_recv_ns >= event.ts_exchange_ns {
            self.total_latency_ns = self
                .total_latency_ns
                .saturating_add(u128::from(event.ts_recv_ns - event.ts_exchange_ns));
            self.latency_samples = self.latency_samples.saturating_add(1);
        }
        match event.order_status {
            OrderStatus::Rejected => {
                self.rejected_children = self.rejected_children.saturating_add(1);
            }
            OrderStatus::Cancelled | OrderStatus::Expired => {
                self.cancelled_children = self.cancelled_children.saturating_add(1);
            }
            _ => {}
        }
        if event.last_qty.0 > 0 {
            self.filled_children = self.filled_children.saturating_add(1);
            self.completed_qty = OrderQty(
                self.completed_qty
                    .0
                    .saturating_add(event.last_qty.0)
                    .min(self.target_qty.0),
            );
            self.cumulative_notional = self.cumulative_notional.saturating_add(
                i64_to_u128(event.last_qty.0).saturating_mul(i64_to_u128(event.last_price.0)),
            );
        }
    }

    /// Returns current metrics snapshot.
    pub fn snapshot(&self) -> AlgoMetricsSnapshot {
        let average_price =
            average_price_from_notional(self.cumulative_notional, self.completed_qty);
        AlgoMetricsSnapshot {
            parent_id: self.parent_id,
            target_qty: self.target_qty,
            submitted_children: self.submitted_children,
            filled_children: self.filled_children,
            rejected_children: self.rejected_children,
            cancelled_children: self.cancelled_children,
            completed_qty: self.completed_qty,
            completion_bps: completion_bps(self.completed_qty.0, self.target_qty.0),
            average_price,
            arrival_slippage_bps: side_slippage_bps(
                self.side,
                average_price,
                self.benchmarks.arrival_price(),
            ),
            vwap_slippage_bps: optional_slippage_bps(
                self.side,
                average_price,
                self.benchmarks.vwap_price(),
            ),
            twap_slippage_bps: optional_slippage_bps(
                self.side,
                average_price,
                self.benchmarks.twap_price(),
            ),
            first_submit_ns: self.first_submit_ns,
            last_event_ns: self.last_event_ns,
            average_latency_ns: average_latency_ns(self.total_latency_ns, self.latency_samples),
        }
    }
}

/// Deterministic TWAP slice planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwapSlicePlanner {
    slice_interval_ns: u64,
}

impl TwapSlicePlanner {
    /// Creates a TWAP planner.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSliceInterval`] when the interval is zero.
    pub const fn try_new(slice_interval_ns: u64) -> Result<Self, AlgoError> {
        if slice_interval_ns == 0 {
            return Err(AlgoError::InvalidSliceInterval);
        }
        Ok(Self { slice_interval_ns })
    }

    /// Creates a TWAP planner, panicking when the interval is zero.
    pub const fn new(slice_interval_ns: u64) -> Self {
        assert!(slice_interval_ns > 0, "slice interval must be positive");
        Self { slice_interval_ns }
    }

    /// Returns the configured slice interval.
    pub const fn slice_interval_ns(&self) -> u64 {
        self.slice_interval_ns
    }

    /// Plans one due child slice for `now_ns`.
    ///
    /// The planner uses only caller-provided timestamps and integer arithmetic.
    /// It returns `Ok(None)` when no additional child quantity is due.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when the parent/progress state is invalid or the
    /// generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_due_slice(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<Option<ChildOrderPlan>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if progress.released_qty().0 > parent.total_qty().0
            || progress.completed_qty().0 > parent.total_qty().0
        {
            return Err(AlgoError::InvalidProgress);
        }
        if now_ns < parent.start_ns() || progress.is_complete() {
            return Ok(None);
        }

        let total_slices = div_ceil_u64(
            parent.end_ns().saturating_sub(parent.start_ns()),
            self.slice_interval_ns,
        )
        .max(1);
        let elapsed_ns = now_ns
            .min(parent.end_ns())
            .saturating_sub(parent.start_ns());
        let due_slices = (elapsed_ns / self.slice_interval_ns)
            .saturating_add(1)
            .min(total_slices);
        let desired = div_ceil_i128(
            i128::from(parent.total_qty().0) * i128::from(due_slices),
            i128::from(total_slices),
        );
        let desired_qty = i64::try_from(desired).unwrap_or(i64::MAX);
        let due_qty = desired_qty.saturating_sub(progress.released_qty().0);
        if due_qty <= 0 {
            return Ok(None);
        }
        let leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        let mut child_qty = due_qty.min(parent.max_clip().0).min(leaves);
        let final_slice = progress.released_qty().0.saturating_add(child_qty)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if child_qty < parent.min_clip().0 && !final_slice {
            return Ok(None);
        }
        if child_qty <= 0 {
            return Ok(None);
        }
        child_qty = child_qty.min(
            parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0),
        );
        let request = parent.build_order_request(client_order_id, OrderQty(child_qty), ts_recv_ns);
        Ok(Some(ChildOrderPlan::new(
            child_id,
            parent.id(),
            request,
            now_ns,
        )?))
    }
}

/// Deterministic percentage-of-volume child slice planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PovSlicePlanner {
    target_participation_bps: u16,
    max_participation_bps: u16,
}

impl PovSlicePlanner {
    /// Creates a POV planner.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidParticipationRate`] when target or cap is
    /// zero, or when target exceeds cap.
    pub const fn try_new(
        target_participation_bps: u16,
        max_participation_bps: u16,
    ) -> Result<Self, AlgoError> {
        if target_participation_bps == 0
            || max_participation_bps == 0
            || target_participation_bps > max_participation_bps
        {
            return Err(AlgoError::InvalidParticipationRate);
        }
        Ok(Self {
            target_participation_bps,
            max_participation_bps,
        })
    }

    /// Creates a POV planner, panicking when rates are invalid.
    pub const fn new(target_participation_bps: u16, max_participation_bps: u16) -> Self {
        assert!(
            target_participation_bps > 0
                && max_participation_bps > 0
                && target_participation_bps <= max_participation_bps,
            "participation target must be positive and <= cap"
        );
        Self {
            target_participation_bps,
            max_participation_bps,
        }
    }

    /// Returns target participation in basis points.
    pub const fn target_participation_bps(&self) -> u16 {
        self.target_participation_bps
    }

    /// Returns maximum participation in basis points.
    pub const fn max_participation_bps(&self) -> u16 {
        self.max_participation_bps
    }

    /// Plans one child slice from cumulative observed market volume.
    ///
    /// The planner assumes `observed_market_volume` excludes the algo's own
    /// child fills when the host can provide that view. If self-volume cannot
    /// be excluded, hosts should choose conservative rates and caps.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress state is invalid or the
    /// generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_volume_slice(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        observed_market_volume: OrderQty,
        now_ns: u64,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<Option<ChildOrderPlan>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if observed_market_volume.0 <= 0 || now_ns < parent.start_ns() || progress.is_complete() {
            return Ok(None);
        }

        let effective_max_bps = if parent.participation_cap_bps() == 0 {
            self.max_participation_bps
        } else {
            self.max_participation_bps
                .min(parent.participation_cap_bps())
        };
        if self.target_participation_bps == 0
            || effective_max_bps == 0
            || self.target_participation_bps > effective_max_bps
        {
            return Err(AlgoError::InvalidParticipationRate);
        }

        let desired = participation_qty(observed_market_volume.0, self.target_participation_bps)
            .min(parent.total_qty().0);
        let max_allowed = participation_qty(observed_market_volume.0, effective_max_bps)
            .min(parent.total_qty().0);
        let due_qty = desired
            .min(max_allowed)
            .saturating_sub(progress.released_qty().0);
        if due_qty <= 0 {
            return Ok(None);
        }

        let leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        let mut child_qty = due_qty.min(parent.max_clip().0).min(leaves);
        let final_slice = progress.released_qty().0.saturating_add(child_qty)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if child_qty < parent.min_clip().0 && !final_slice {
            return Ok(None);
        }
        if child_qty <= 0 {
            return Ok(None);
        }
        child_qty = child_qty.min(
            parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0),
        );
        let request = parent.build_order_request(client_order_id, OrderQty(child_qty), ts_recv_ns);
        Ok(Some(ChildOrderPlan::new(
            child_id,
            parent.id(),
            request,
            now_ns,
        )?))
    }
}

/// Borrowed cumulative volume curve for VWAP planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VwapVolumeCurve<'a> {
    start_ns: u64,
    bucket_interval_ns: u64,
    cumulative_weights: &'a [u64],
}

impl<'a> VwapVolumeCurve<'a> {
    /// Creates a borrowed cumulative VWAP volume curve.
    ///
    /// `cumulative_weights` must be non-empty, strictly positive at the end,
    /// and monotonically non-decreasing. The last element is the total expected
    /// volume weight for the parent interval.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidVolumeProfile`] when the interval is zero,
    /// the curve is empty, total weight is zero, or cumulative weights regress.
    pub fn new(
        start_ns: u64,
        bucket_interval_ns: u64,
        cumulative_weights: &'a [u64],
    ) -> Result<Self, AlgoError> {
        if bucket_interval_ns == 0 || cumulative_weights.is_empty() {
            return Err(AlgoError::InvalidVolumeProfile);
        }
        let Some(total) = cumulative_weights.last().copied() else {
            return Err(AlgoError::InvalidVolumeProfile);
        };
        if total == 0 {
            return Err(AlgoError::InvalidVolumeProfile);
        }
        let mut previous = 0_u64;
        for weight in cumulative_weights {
            if *weight < previous {
                return Err(AlgoError::InvalidVolumeProfile);
            }
            previous = *weight;
        }
        Ok(Self {
            start_ns,
            bucket_interval_ns,
            cumulative_weights,
        })
    }

    /// Returns curve start timestamp.
    pub const fn start_ns(&self) -> u64 {
        self.start_ns
    }

    /// Returns bucket interval in nanoseconds.
    pub const fn bucket_interval_ns(&self) -> u64 {
        self.bucket_interval_ns
    }

    /// Returns cumulative profile weights.
    pub const fn cumulative_weights(&self) -> &'a [u64] {
        self.cumulative_weights
    }

    /// Returns total curve weight.
    pub fn total_weight(&self) -> u64 {
        self.cumulative_weights
            .last()
            .copied()
            .expect("validated curve is non-empty")
    }

    fn cumulative_weight_at(&self, now_ns: u64) -> u64 {
        if now_ns < self.start_ns {
            return 0;
        }
        let elapsed = now_ns.saturating_sub(self.start_ns);
        let index = (elapsed / self.bucket_interval_ns) as usize;
        let capped = index.min(self.cumulative_weights.len().saturating_sub(1));
        self.cumulative_weights[capped]
    }
}

/// Deterministic VWAP child slice planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VwapSlicePlanner<'a> {
    curve: VwapVolumeCurve<'a>,
}

impl<'a> VwapSlicePlanner<'a> {
    /// Creates a VWAP planner from a validated volume curve.
    pub const fn new(curve: VwapVolumeCurve<'a>) -> Self {
        Self { curve }
    }

    /// Returns the borrowed volume curve.
    pub const fn curve(&self) -> VwapVolumeCurve<'a> {
        self.curve
    }

    /// Plans one child slice from the expected cumulative volume curve.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress state is invalid or the
    /// generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_curve_slice(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<Option<ChildOrderPlan>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if now_ns < parent.start_ns() || progress.is_complete() {
            return Ok(None);
        }

        let cumulative_weight = self.curve.cumulative_weight_at(now_ns);
        if cumulative_weight == 0 {
            return Ok(None);
        }
        let desired = vwap_target_qty(
            parent.total_qty().0,
            cumulative_weight,
            self.curve.total_weight(),
        );
        let due_qty = desired.saturating_sub(progress.released_qty().0);
        if due_qty <= 0 {
            return Ok(None);
        }

        let leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        let mut child_qty = due_qty.min(parent.max_clip().0).min(leaves);
        let final_slice = progress.released_qty().0.saturating_add(child_qty)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if child_qty < parent.min_clip().0 && !final_slice {
            return Ok(None);
        }
        if child_qty <= 0 {
            return Ok(None);
        }
        child_qty = child_qty.min(
            parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0),
        );
        let request = parent.build_order_request(client_order_id, OrderQty(child_qty), ts_recv_ns);
        Ok(Some(ChildOrderPlan::new(
            child_id,
            parent.id(),
            request,
            now_ns,
        )?))
    }
}

/// Deterministic synthetic iceberg replenishment planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IcebergSlicePlanner {
    display_qty: OrderQty,
    replenish_threshold: OrderQty,
}

impl IcebergSlicePlanner {
    /// Creates an iceberg planner.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidDisplayQuantity`] when display quantity is
    /// non-positive, threshold is negative, or threshold exceeds display size.
    pub const fn try_new(
        display_qty: OrderQty,
        replenish_threshold: OrderQty,
    ) -> Result<Self, AlgoError> {
        if display_qty.0 <= 0 || replenish_threshold.0 < 0 || replenish_threshold.0 > display_qty.0
        {
            return Err(AlgoError::InvalidDisplayQuantity);
        }
        Ok(Self {
            display_qty,
            replenish_threshold,
        })
    }

    /// Creates an iceberg planner, panicking when display settings are invalid.
    pub const fn new(display_qty: OrderQty, replenish_threshold: OrderQty) -> Self {
        assert!(
            display_qty.0 > 0
                && replenish_threshold.0 >= 0
                && replenish_threshold.0 <= display_qty.0,
            "iceberg display quantity must be positive and threshold must be within display"
        );
        Self {
            display_qty,
            replenish_threshold,
        }
    }

    /// Returns the target displayed child quantity.
    pub const fn display_qty(&self) -> OrderQty {
        self.display_qty
    }

    /// Returns the open-quantity threshold at or below which replenishment is
    /// due.
    pub const fn replenish_threshold(&self) -> OrderQty {
        self.replenish_threshold
    }

    /// Plans one synthetic iceberg replenishment child.
    ///
    /// The host remains responsible for deciding whether to use native venue
    /// reserve/iceberg order support or submit synthetic child orders through
    /// the OMS.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress state is invalid or the
    /// generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_replenishment(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<Option<ChildOrderPlan>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if now_ns < parent.start_ns()
            || progress.is_complete()
            || progress.open_qty().0 > self.replenish_threshold.0
        {
            return Ok(None);
        }

        let leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        if leaves <= 0 {
            return Ok(None);
        }
        let mut child_qty = self.display_qty.0.min(parent.max_clip().0).min(leaves);
        let final_slice = progress.released_qty().0.saturating_add(child_qty)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if child_qty < parent.min_clip().0 && !final_slice {
            return Ok(None);
        }
        if child_qty <= 0 {
            return Ok(None);
        }
        child_qty = child_qty.min(
            parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0),
        );
        let request = parent.build_order_request(client_order_id, OrderQty(child_qty), ts_recv_ns);
        Ok(Some(ChildOrderPlan::new(
            child_id,
            parent.id(),
            request,
            now_ns,
        )?))
    }
}

/// Passive peg reference used by [`PassiveQueuePlanner`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PassivePegMode {
    /// Quote at the same-side best price.
    SameSide = 1,
    /// Quote at the midpoint when the spread allows it.
    Midpoint = 2,
    /// Quote one passive tick inside the spread when possible.
    ImproveOneTick = 3,
}

/// Passive queue action selected by [`PassiveQueuePlanner`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PassiveQueueAction {
    /// Do not release a child order for this decision.
    Wait = 1,
    /// Join the selected passive queue.
    JoinQueue = 2,
    /// Improve the selected passive price inside the spread.
    ImprovePrice = 3,
    /// Cross the spread because the configured time budget is exhausted.
    CrossSpread = 4,
}

impl PassiveQueueAction {
    /// Returns true when the action releases a child order.
    pub const fn releases_child(self) -> bool {
        matches!(
            self,
            Self::JoinQueue | Self::ImprovePrice | Self::CrossSpread
        )
    }
}

/// Market context for passive queue planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveQueueContext {
    best_bid: OrderPrice,
    best_ask: OrderPrice,
    queue_ahead_qty: OrderQty,
    expected_take_qty: OrderQty,
    adverse_selection_bps: u16,
}

impl PassiveQueueContext {
    /// Creates passive queue market context.
    ///
    /// `queue_ahead_qty` is the visible or modeled quantity ahead of the
    /// planned child at the candidate price. `expected_take_qty` is the host's
    /// short-horizon estimate of contra-side volume that can trade through the
    /// queue.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidPassiveQueueParameters`] when best bid/ask
    /// are non-positive, crossed, or queue quantities are negative.
    pub const fn new(
        best_bid: OrderPrice,
        best_ask: OrderPrice,
        queue_ahead_qty: OrderQty,
        expected_take_qty: OrderQty,
        adverse_selection_bps: u16,
    ) -> Result<Self, AlgoError> {
        if best_bid.0 <= 0
            || best_ask.0 <= 0
            || best_bid.0 >= best_ask.0
            || queue_ahead_qty.0 < 0
            || expected_take_qty.0 < 0
        {
            return Err(AlgoError::InvalidPassiveQueueParameters);
        }
        Ok(Self {
            best_bid,
            best_ask,
            queue_ahead_qty,
            expected_take_qty,
            adverse_selection_bps,
        })
    }

    /// Returns the current best bid.
    pub const fn best_bid(&self) -> OrderPrice {
        self.best_bid
    }

    /// Returns the current best ask.
    pub const fn best_ask(&self) -> OrderPrice {
        self.best_ask
    }

    /// Returns estimated quantity ahead at the candidate queue.
    pub const fn queue_ahead_qty(&self) -> OrderQty {
        self.queue_ahead_qty
    }

    /// Returns expected contra-side trade quantity at the candidate queue.
    pub const fn expected_take_qty(&self) -> OrderQty {
        self.expected_take_qty
    }

    /// Returns adverse-selection estimate in basis points.
    pub const fn adverse_selection_bps(&self) -> u16 {
        self.adverse_selection_bps
    }
}

/// Configuration for passive queue planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveQueueConfig {
    peg_mode: PassivePegMode,
    tick_size: OrderPrice,
    min_fill_probability_bps: u16,
    max_adverse_selection_bps: u16,
    improve_when_fill_below_bps: u16,
    max_improvement_ticks: u16,
    allow_cross: bool,
    cross_after_elapsed_bps: u16,
}

impl PassiveQueueConfig {
    /// Creates passive queue configuration with conservative defaults.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidPassiveQueueParameters`] when tick size is
    /// not positive.
    pub const fn new(peg_mode: PassivePegMode, tick_size: OrderPrice) -> Result<Self, AlgoError> {
        if tick_size.0 <= 0 {
            return Err(AlgoError::InvalidPassiveQueueParameters);
        }
        Ok(Self {
            peg_mode,
            tick_size,
            min_fill_probability_bps: 2_500,
            max_adverse_selection_bps: 250,
            improve_when_fill_below_bps: 1_500,
            max_improvement_ticks: 1,
            allow_cross: false,
            cross_after_elapsed_bps: 9_500,
        })
    }

    /// Returns a copy with queue thresholds updated.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidPassiveQueueParameters`] when any basis
    /// point value exceeds 10,000.
    pub const fn with_thresholds(
        mut self,
        min_fill_probability_bps: u16,
        max_adverse_selection_bps: u16,
        improve_when_fill_below_bps: u16,
    ) -> Result<Self, AlgoError> {
        if min_fill_probability_bps > 10_000
            || max_adverse_selection_bps > 10_000
            || improve_when_fill_below_bps > 10_000
        {
            return Err(AlgoError::InvalidPassiveQueueParameters);
        }
        self.min_fill_probability_bps = min_fill_probability_bps;
        self.max_adverse_selection_bps = max_adverse_selection_bps;
        self.improve_when_fill_below_bps = improve_when_fill_below_bps;
        Ok(self)
    }

    /// Returns a copy with maximum inside-spread improvement ticks updated.
    pub const fn with_max_improvement_ticks(mut self, max_improvement_ticks: u16) -> Self {
        self.max_improvement_ticks = max_improvement_ticks;
        self
    }

    /// Returns a copy with crossing behavior updated.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidPassiveQueueParameters`] when elapsed basis
    /// points exceed 10,000.
    pub const fn with_crossing(
        mut self,
        allow_cross: bool,
        cross_after_elapsed_bps: u16,
    ) -> Result<Self, AlgoError> {
        if cross_after_elapsed_bps > 10_000 {
            return Err(AlgoError::InvalidPassiveQueueParameters);
        }
        self.allow_cross = allow_cross;
        self.cross_after_elapsed_bps = cross_after_elapsed_bps;
        Ok(self)
    }

    /// Returns peg mode.
    pub const fn peg_mode(&self) -> PassivePegMode {
        self.peg_mode
    }

    /// Returns tick size in normalized price units.
    pub const fn tick_size(&self) -> OrderPrice {
        self.tick_size
    }

    /// Returns minimum fill probability in basis points.
    pub const fn min_fill_probability_bps(&self) -> u16 {
        self.min_fill_probability_bps
    }

    /// Returns maximum tolerated adverse selection in basis points.
    pub const fn max_adverse_selection_bps(&self) -> u16 {
        self.max_adverse_selection_bps
    }

    /// Returns fill-probability threshold below which improvement is preferred.
    pub const fn improve_when_fill_below_bps(&self) -> u16 {
        self.improve_when_fill_below_bps
    }

    /// Returns maximum passive improvement ticks.
    pub const fn max_improvement_ticks(&self) -> u16 {
        self.max_improvement_ticks
    }

    /// Returns true when the planner may cross the spread after the time
    /// threshold.
    pub const fn allow_cross(&self) -> bool {
        self.allow_cross
    }

    /// Returns elapsed parent interval threshold for crossing.
    pub const fn cross_after_elapsed_bps(&self) -> u16 {
        self.cross_after_elapsed_bps
    }
}

impl Default for PassiveQueueConfig {
    fn default() -> Self {
        Self::new(PassivePegMode::SameSide, OrderPrice(1))
            .expect("static passive queue defaults are valid")
    }
}

/// Passive queue planning estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveQueueEstimate {
    action: PassiveQueueAction,
    candidate_price: OrderPrice,
    fill_probability_bps: u16,
    elapsed_bps: u16,
    candidate_qty: OrderQty,
}

impl PassiveQueueEstimate {
    /// Returns selected passive queue action.
    pub const fn action(&self) -> PassiveQueueAction {
        self.action
    }

    /// Returns selected limit price.
    pub const fn candidate_price(&self) -> OrderPrice {
        self.candidate_price
    }

    /// Returns estimated fill probability in basis points.
    pub const fn fill_probability_bps(&self) -> u16 {
        self.fill_probability_bps
    }

    /// Returns elapsed parent interval in basis points.
    pub const fn elapsed_bps(&self) -> u16 {
        self.elapsed_bps
    }

    /// Returns candidate child quantity used for the estimate.
    pub const fn candidate_qty(&self) -> OrderQty {
        self.candidate_qty
    }
}

/// Passive queue decision with an optional child order plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveQueueDecision {
    estimate: PassiveQueueEstimate,
    child: Option<ChildOrderPlan>,
}

impl PassiveQueueDecision {
    /// Returns the estimate that drove the decision.
    pub const fn estimate(&self) -> PassiveQueueEstimate {
        self.estimate
    }

    /// Returns the selected action.
    pub const fn action(&self) -> PassiveQueueAction {
        self.estimate.action()
    }

    /// Returns the optional child order plan.
    pub const fn child(&self) -> Option<ChildOrderPlan> {
        self.child
    }
}

/// Deterministic passive peg and queue-position planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveQueuePlanner {
    config: PassiveQueueConfig,
}

impl PassiveQueuePlanner {
    /// Creates a passive queue planner.
    pub const fn new(config: PassiveQueueConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> PassiveQueueConfig {
        self.config
    }

    /// Estimates passive queue action, price, fill probability, and quantity.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/context state is invalid.
    pub fn estimate(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        context: PassiveQueueContext,
    ) -> Result<PassiveQueueEstimate, AlgoError> {
        parent.validate()?;
        self.validate_context(context)?;
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }

        let candidate_qty = passive_candidate_qty(parent, progress, now_ns);
        let fill_probability_bps = queue_fill_probability_bps(
            context.queue_ahead_qty().0,
            candidate_qty.0,
            context.expected_take_qty().0,
        );
        let elapsed = elapsed_bps(parent.start_ns(), parent.end_ns(), now_ns);
        let action = self.select_action(parent, now_ns, context, fill_probability_bps, elapsed);
        let candidate_price = self.price_for_action(parent.side(), context, action);

        Ok(PassiveQueueEstimate {
            action,
            candidate_price,
            fill_probability_bps,
            elapsed_bps: elapsed,
            candidate_qty,
        })
    }

    /// Plans one passive queue child decision.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/context state is invalid or
    /// the generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_passive_slice(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        context: PassiveQueueContext,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<PassiveQueueDecision, AlgoError> {
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if now_ns < parent.start_ns() || progress.is_complete() {
            let estimate = self.estimate(parent, progress, now_ns, context)?;
            return Ok(PassiveQueueDecision {
                estimate: PassiveQueueEstimate {
                    action: PassiveQueueAction::Wait,
                    ..estimate
                },
                child: None,
            });
        }

        let estimate = self.estimate(parent, progress, now_ns, context)?;
        if !estimate.action().releases_child() || estimate.candidate_qty().0 <= 0 {
            return Ok(PassiveQueueDecision {
                estimate,
                child: None,
            });
        }
        let final_slice = progress
            .released_qty()
            .0
            .saturating_add(estimate.candidate_qty().0)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if estimate.candidate_qty().0 < parent.min_clip().0 && !final_slice {
            return Ok(PassiveQueueDecision {
                estimate: PassiveQueueEstimate {
                    action: PassiveQueueAction::Wait,
                    ..estimate
                },
                child: None,
            });
        }

        let request = parent.build_order_request_at_price(
            client_order_id,
            estimate.candidate_qty(),
            estimate.candidate_price(),
            ts_recv_ns,
        );
        Ok(PassiveQueueDecision {
            estimate,
            child: Some(ChildOrderPlan::new(child_id, parent.id(), request, now_ns)?),
        })
    }

    fn validate_context(&self, context: PassiveQueueContext) -> Result<(), AlgoError> {
        if self.config.tick_size().0 <= 0
            || self.config.min_fill_probability_bps() > 10_000
            || self.config.max_adverse_selection_bps() > 10_000
            || self.config.improve_when_fill_below_bps() > 10_000
            || self.config.cross_after_elapsed_bps() > 10_000
            || context.best_bid().0 <= 0
            || context.best_ask().0 <= 0
            || context.best_bid().0 >= context.best_ask().0
            || context.queue_ahead_qty().0 < 0
            || context.expected_take_qty().0 < 0
        {
            return Err(AlgoError::InvalidPassiveQueueParameters);
        }
        Ok(())
    }

    fn select_action(
        &self,
        parent: &ParentOrder,
        now_ns: u64,
        context: PassiveQueueContext,
        fill_probability_bps: u16,
        elapsed: u16,
    ) -> PassiveQueueAction {
        if context.adverse_selection_bps() > self.config.max_adverse_selection_bps() {
            return PassiveQueueAction::Wait;
        }
        if self.config.allow_cross() && elapsed >= self.config.cross_after_elapsed_bps() {
            return PassiveQueueAction::CrossSpread;
        }
        if fill_probability_bps < self.config.improve_when_fill_below_bps()
            && self.can_improve(parent.side(), context)
            && now_ns < parent.end_ns()
        {
            return PassiveQueueAction::ImprovePrice;
        }
        if fill_probability_bps >= self.config.min_fill_probability_bps()
            || self.config.peg_mode() != PassivePegMode::SameSide
        {
            return PassiveQueueAction::JoinQueue;
        }
        PassiveQueueAction::Wait
    }

    fn price_for_action(
        &self,
        side: OrderSide,
        context: PassiveQueueContext,
        action: PassiveQueueAction,
    ) -> OrderPrice {
        match action {
            PassiveQueueAction::Wait | PassiveQueueAction::JoinQueue => {
                self.base_price(side, context)
            }
            PassiveQueueAction::ImprovePrice => self.improved_price(side, context),
            PassiveQueueAction::CrossSpread => match side {
                OrderSide::Buy => context.best_ask(),
                OrderSide::Sell => context.best_bid(),
            },
        }
    }

    fn base_price(&self, side: OrderSide, context: PassiveQueueContext) -> OrderPrice {
        match self.config.peg_mode() {
            PassivePegMode::SameSide => match side {
                OrderSide::Buy => context.best_bid(),
                OrderSide::Sell => context.best_ask(),
            },
            PassivePegMode::Midpoint => midpoint_price(side, context, self.config.tick_size()),
            PassivePegMode::ImproveOneTick => self.improved_price(side, context),
        }
    }

    fn improved_price(&self, side: OrderSide, context: PassiveQueueContext) -> OrderPrice {
        let ticks = i64::from(self.config.max_improvement_ticks().max(1));
        let improvement = self.config.tick_size().0.saturating_mul(ticks);
        match side {
            OrderSide::Buy => {
                let max_passive = context
                    .best_ask()
                    .0
                    .saturating_sub(self.config.tick_size().0);
                OrderPrice(
                    context
                        .best_bid()
                        .0
                        .saturating_add(improvement)
                        .min(max_passive),
                )
            }
            OrderSide::Sell => {
                let min_passive = context
                    .best_bid()
                    .0
                    .saturating_add(self.config.tick_size().0);
                OrderPrice(
                    context
                        .best_ask()
                        .0
                        .saturating_sub(improvement)
                        .max(min_passive),
                )
            }
        }
    }

    fn can_improve(&self, side: OrderSide, context: PassiveQueueContext) -> bool {
        if self.config.max_improvement_ticks() == 0 {
            return false;
        }
        let spread = context.best_ask().0.saturating_sub(context.best_bid().0);
        if spread <= self.config.tick_size().0 {
            return false;
        }
        self.improved_price(side, context) != self.base_price(side, context)
    }
}

/// Route availability state used by [`SorPlanner`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SorRouteStatus {
    /// Route is available for new child orders.
    Available = 1,
    /// Route is blocked by policy, risk, or operator action.
    Blocked = 2,
    /// Route is degraded and should be skipped by default routing.
    Degraded = 3,
    /// Route is temporarily cooling down after rejects, disconnects, or throttles.
    Cooldown = 4,
}

impl SorRouteStatus {
    /// Returns true when the route can receive a new child order.
    pub const fn is_routable(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// Order-type capability advertised by a SOR route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorRouteCapability {
    supports_limit: bool,
    supports_market: bool,
}

impl SorRouteCapability {
    /// Creates route order-type capability flags.
    pub const fn new(supports_limit: bool, supports_market: bool) -> Self {
        Self {
            supports_limit,
            supports_market,
        }
    }

    /// Returns true when limit orders are supported.
    pub const fn supports_limit(&self) -> bool {
        self.supports_limit
    }

    /// Returns true when market orders are supported.
    pub const fn supports_market(&self) -> bool {
        self.supports_market
    }

    fn supports_order_type(&self, order_type: OrderType) -> bool {
        match order_type {
            OrderType::Limit | OrderType::StopLimit => self.supports_limit,
            OrderType::Market | OrderType::Stop => self.supports_market,
        }
    }
}

impl Default for SorRouteCapability {
    fn default() -> Self {
        Self::new(true, true)
    }
}

/// Route quality metrics used for smart-order-router scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorRouteMetrics {
    fee_bps: i16,
    latency_us: u32,
    reject_rate_bps: u16,
    fill_probability_bps: u16,
    toxicity_bps: u16,
    data_quality_bps: u16,
}

impl SorRouteMetrics {
    /// Creates route metrics for deterministic scoring.
    ///
    /// `fee_bps` may be negative for rebates. All unsigned basis-point values
    /// must be at most 10,000.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSorParameters`] when a metric is outside
    /// its supported range.
    pub const fn new(
        fee_bps: i16,
        latency_us: u32,
        reject_rate_bps: u16,
        fill_probability_bps: u16,
        toxicity_bps: u16,
        data_quality_bps: u16,
    ) -> Result<Self, AlgoError> {
        if fee_bps < -10_000
            || fee_bps > 10_000
            || reject_rate_bps > 10_000
            || fill_probability_bps > 10_000
            || toxicity_bps > 10_000
            || data_quality_bps > 10_000
        {
            return Err(AlgoError::InvalidSorParameters);
        }
        Ok(Self {
            fee_bps,
            latency_us,
            reject_rate_bps,
            fill_probability_bps,
            toxicity_bps,
            data_quality_bps,
        })
    }

    /// Returns fee or rebate estimate in basis points.
    pub const fn fee_bps(&self) -> i16 {
        self.fee_bps
    }

    /// Returns route latency estimate in microseconds.
    pub const fn latency_us(&self) -> u32 {
        self.latency_us
    }

    /// Returns recent route reject rate in basis points.
    pub const fn reject_rate_bps(&self) -> u16 {
        self.reject_rate_bps
    }

    /// Returns estimated route fill probability in basis points.
    pub const fn fill_probability_bps(&self) -> u16 {
        self.fill_probability_bps
    }

    /// Returns route toxicity estimate in basis points.
    pub const fn toxicity_bps(&self) -> u16 {
        self.toxicity_bps
    }

    /// Returns market-data quality score in basis points.
    pub const fn data_quality_bps(&self) -> u16 {
        self.data_quality_bps
    }
}

impl Default for SorRouteMetrics {
    fn default() -> Self {
        Self {
            fee_bps: 0,
            latency_us: 0,
            reject_rate_bps: 0,
            fill_probability_bps: 10_000,
            toxicity_bps: 0,
            data_quality_bps: 10_000,
        }
    }
}

/// Routable liquidity candidate consumed by [`SorPlanner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorRouteCandidate {
    route_id: RouteId,
    price: OrderPrice,
    available_qty: OrderQty,
    status: SorRouteStatus,
    capability: SorRouteCapability,
    metrics: SorRouteMetrics,
}

impl SorRouteCandidate {
    /// Creates an available route candidate with default capabilities and
    /// metrics.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSorParameters`] when price or available
    /// quantity is non-positive.
    pub const fn new(
        route_id: RouteId,
        price: OrderPrice,
        available_qty: OrderQty,
    ) -> Result<Self, AlgoError> {
        if price.0 <= 0 || available_qty.0 <= 0 {
            return Err(AlgoError::InvalidSorParameters);
        }
        Ok(Self {
            route_id,
            price,
            available_qty,
            status: SorRouteStatus::Available,
            capability: SorRouteCapability::new(true, true),
            metrics: SorRouteMetrics {
                fee_bps: 0,
                latency_us: 0,
                reject_rate_bps: 0,
                fill_probability_bps: 10_000,
                toxicity_bps: 0,
                data_quality_bps: 10_000,
            },
        })
    }

    /// Returns a copy with route status updated.
    pub const fn with_status(mut self, status: SorRouteStatus) -> Self {
        self.status = status;
        self
    }

    /// Returns a copy with capability updated.
    pub const fn with_capability(mut self, capability: SorRouteCapability) -> Self {
        self.capability = capability;
        self
    }

    /// Returns a copy with route metrics updated.
    pub const fn with_metrics(mut self, metrics: SorRouteMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    /// Returns route identifier.
    pub const fn route_id(&self) -> RouteId {
        self.route_id
    }

    /// Returns route price.
    pub const fn price(&self) -> OrderPrice {
        self.price
    }

    /// Returns available route quantity.
    pub const fn available_qty(&self) -> OrderQty {
        self.available_qty
    }

    /// Returns route status.
    pub const fn status(&self) -> SorRouteStatus {
        self.status
    }

    /// Returns route capability flags.
    pub const fn capability(&self) -> SorRouteCapability {
        self.capability
    }

    /// Returns route metrics.
    pub const fn metrics(&self) -> SorRouteMetrics {
        self.metrics
    }
}

/// Integer score weights for [`SorPlanner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorScoreWeights {
    price_weight: u16,
    liquidity_weight: u16,
    fee_weight: u16,
    latency_weight: u16,
    reject_weight: u16,
    fill_weight: u16,
    toxicity_weight: u16,
    data_quality_weight: u16,
}

impl SorScoreWeights {
    /// Creates SOR score weights.
    #[allow(
        clippy::too_many_arguments,
        reason = "flat constructor keeps score weights explicit and allocation-free"
    )]
    pub const fn new(
        price_weight: u16,
        liquidity_weight: u16,
        fee_weight: u16,
        latency_weight: u16,
        reject_weight: u16,
        fill_weight: u16,
        toxicity_weight: u16,
        data_quality_weight: u16,
    ) -> Self {
        Self {
            price_weight,
            liquidity_weight,
            fee_weight,
            latency_weight,
            reject_weight,
            fill_weight,
            toxicity_weight,
            data_quality_weight,
        }
    }

    /// Returns price weight.
    pub const fn price_weight(&self) -> u16 {
        self.price_weight
    }

    /// Returns liquidity weight.
    pub const fn liquidity_weight(&self) -> u16 {
        self.liquidity_weight
    }

    /// Returns fee weight.
    pub const fn fee_weight(&self) -> u16 {
        self.fee_weight
    }

    /// Returns latency weight.
    pub const fn latency_weight(&self) -> u16 {
        self.latency_weight
    }

    /// Returns reject-rate weight.
    pub const fn reject_weight(&self) -> u16 {
        self.reject_weight
    }

    /// Returns fill-probability weight.
    pub const fn fill_weight(&self) -> u16 {
        self.fill_weight
    }

    /// Returns toxicity weight.
    pub const fn toxicity_weight(&self) -> u16 {
        self.toxicity_weight
    }

    /// Returns data-quality weight.
    pub const fn data_quality_weight(&self) -> u16 {
        self.data_quality_weight
    }
}

impl Default for SorScoreWeights {
    fn default() -> Self {
        Self::new(8, 2, 2, 1, 3, 3, 4, 1)
    }
}

/// Smart-order-router configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorConfig {
    max_route_count: usize,
    weights: SorScoreWeights,
}

impl SorConfig {
    /// Creates SOR configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSorParameters`] when max route count is
    /// zero.
    pub const fn new(max_route_count: usize, weights: SorScoreWeights) -> Result<Self, AlgoError> {
        if max_route_count == 0 {
            return Err(AlgoError::InvalidSorParameters);
        }
        Ok(Self {
            max_route_count,
            weights,
        })
    }

    /// Returns maximum number of routes to allocate in one decision.
    pub const fn max_route_count(&self) -> usize {
        self.max_route_count
    }

    /// Returns score weights.
    pub const fn weights(&self) -> SorScoreWeights {
        self.weights
    }
}

impl Default for SorConfig {
    fn default() -> Self {
        Self::new(4, SorScoreWeights::default()).expect("static SOR defaults are valid")
    }
}

/// Scored child allocation produced by [`SorPlanner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorChildAllocation {
    score: i64,
    plan: ChildOrderPlan,
}

impl SorChildAllocation {
    /// Returns route score used for selection.
    pub const fn score(&self) -> i64 {
        self.score
    }

    /// Returns planned child order for the selected route.
    pub const fn plan(&self) -> ChildOrderPlan {
        self.plan
    }
}

/// Fixed-capacity SOR decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorDecision<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    allocations: [Option<SorChildAllocation>; N],
    len: usize,
    considered_routes: usize,
    blocked_routes: usize,
}

impl<const N: usize> SorDecision<N> {
    /// Creates an empty SOR decision.
    pub const fn new(considered_routes: usize) -> Self {
        Self {
            allocations: [None; N],
            len: 0,
            considered_routes,
            blocked_routes: 0,
        }
    }

    /// Returns allocation count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no allocations were produced.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns number of considered route candidates.
    pub const fn considered_routes(&self) -> usize {
        self.considered_routes
    }

    /// Returns number of skipped routes.
    pub const fn blocked_routes(&self) -> usize {
        self.blocked_routes
    }

    /// Returns allocations in selected order.
    pub fn allocations(&self) -> impl Iterator<Item = &SorChildAllocation> {
        self.allocations[..self.len]
            .iter()
            .filter_map(Option::as_ref)
    }

    fn push(&mut self, allocation: SorChildAllocation) -> Result<(), AlgoError> {
        if self.len == N {
            return Err(AlgoError::DecisionFull { capacity: N });
        }
        self.allocations[self.len] = Some(allocation);
        self.len += 1;
        Ok(())
    }

    fn mark_blocked(&mut self) {
        self.blocked_routes = self.blocked_routes.saturating_add(1);
    }
}

/// Deterministic smart-order-router planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorPlanner {
    config: SorConfig,
}

impl SorPlanner {
    /// Creates a SOR planner.
    pub const fn new(config: SorConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> SorConfig {
        self.config
    }

    /// Scores one route candidate for a parent order.
    ///
    /// Higher scores are preferred. Non-routable candidates return `None`.
    pub fn score_route(
        &self,
        parent: &ParentOrder,
        candidate: SorRouteCandidate,
        best_price: OrderPrice,
    ) -> Option<i64> {
        if !candidate.status().is_routable()
            || !candidate
                .capability()
                .supports_order_type(parent.order_type())
            || candidate.available_qty().0 <= 0
            || candidate.price().0 <= 0
        {
            return None;
        }
        let weights = self.config.weights();
        let liquidity_bps = route_liquidity_bps(candidate.available_qty().0, parent.max_clip().0);
        let price_penalty = route_price_penalty_bps(parent.side(), candidate.price(), best_price);
        let latency_penalty = candidate.metrics().latency_us() / 100;
        let fee_penalty =
            i64::from(candidate.metrics().fee_bps()) * i64::from(weights.fee_weight());

        let mut score = 0_i64;
        score = score.saturating_add(
            i64::from(candidate.metrics().fill_probability_bps())
                * i64::from(weights.fill_weight()),
        );
        score =
            score.saturating_add(i64::from(liquidity_bps) * i64::from(weights.liquidity_weight()));
        score = score.saturating_add(
            i64::from(candidate.metrics().data_quality_bps())
                * i64::from(weights.data_quality_weight()),
        );
        score = score.saturating_sub(i64::from(price_penalty) * i64::from(weights.price_weight()));
        score = score.saturating_sub(fee_penalty);
        score =
            score.saturating_sub(i64::from(latency_penalty) * i64::from(weights.latency_weight()));
        score = score.saturating_sub(
            i64::from(candidate.metrics().reject_rate_bps()) * i64::from(weights.reject_weight()),
        );
        score = score.saturating_sub(
            i64::from(candidate.metrics().toxicity_bps()) * i64::from(weights.toxicity_weight()),
        );
        Some(score)
    }

    /// Plans route allocations for one child-routing decision.
    ///
    /// `child_ids` and `client_order_ids` must contain at least `N` entries
    /// when the caller wants to allow `N` allocations. The planner never
    /// generates ids internally.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress state is invalid, candidate
    /// data is invalid, capacity is exhausted, or not enough ids are supplied.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers, timestamps, and route candidates"
    )]
    pub fn plan_routes<const N: usize>(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        candidates: &[SorRouteCandidate],
        child_ids: &[ChildOrderId],
        client_order_ids: &[ClientOrderId],
        ts_recv_ns: u64,
    ) -> Result<SorDecision<N>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if child_ids.len() < N || client_order_ids.len() < N {
            return Err(AlgoError::InvalidSorParameters);
        }
        if now_ns < parent.start_ns() || progress.is_complete() || candidates.is_empty() {
            return Ok(SorDecision::new(candidates.len()));
        }

        let mut decision = SorDecision::<N>::new(candidates.len());
        let mut leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0)
            .min(parent.max_clip().0);
        let route_limit = self.config.max_route_count().min(N);
        let Some(best_price) = best_sor_price(parent.side(), candidates) else {
            return Ok(decision);
        };

        while leaves > 0 && decision.len() < route_limit {
            let mut best_index = None;
            let mut best_score = i64::MIN;
            for (index, candidate) in candidates.iter().copied().enumerate() {
                if decision_has_route(&decision, candidate.route_id()) {
                    continue;
                }
                let Some(score) = self.score_route(parent, candidate, best_price) else {
                    if decision.is_empty() {
                        decision.mark_blocked();
                    }
                    continue;
                };
                if score > best_score {
                    best_score = score;
                    best_index = Some(index);
                }
            }
            let Some(index) = best_index else {
                break;
            };
            let candidate = candidates[index];
            let qty = leaves.min(candidate.available_qty().0);
            let final_slice = progress.released_qty().0.saturating_add(qty) >= parent.total_qty().0
                || now_ns >= parent.end_ns();
            if qty < parent.min_clip().0 && !final_slice {
                break;
            }
            let request = parent.build_order_request_for_route_at_price(
                candidate.route_id(),
                client_order_ids[decision.len()],
                OrderQty(qty),
                candidate.price(),
                ts_recv_ns,
            );
            let plan =
                ChildOrderPlan::new(child_ids[decision.len()], parent.id(), request, now_ns)?;
            decision.push(SorChildAllocation {
                score: best_score,
                plan,
            })?;
            leaves = leaves.saturating_sub(qty);
        }

        Ok(decision)
    }
}

/// Liquidity-seeking action selected for one route.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LiquiditySeekingAction {
    /// Skip the route for this decision.
    Skip = 1,
    /// Send a small probe child order.
    Probe = 2,
    /// Send a larger liquidity-taking child order.
    Take = 3,
}

impl LiquiditySeekingAction {
    /// Returns true when this action releases a child order.
    pub const fn releases_child(self) -> bool {
        matches!(self, Self::Probe | Self::Take)
    }
}

/// Liquidity-seeking candidate derived from a routable venue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquiditySeekingCandidate {
    route: SorRouteCandidate,
    hidden_liquidity_bps: u16,
    price_improvement_bps: u16,
    minimum_quantity: OrderQty,
}

impl LiquiditySeekingCandidate {
    /// Creates a liquidity-seeking route candidate.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidLiquiditySeekingParameters`] when bps
    /// fields exceed 10,000 or minimum quantity is negative.
    pub const fn new(
        route: SorRouteCandidate,
        hidden_liquidity_bps: u16,
        price_improvement_bps: u16,
        minimum_quantity: OrderQty,
    ) -> Result<Self, AlgoError> {
        if hidden_liquidity_bps > 10_000 || price_improvement_bps > 10_000 || minimum_quantity.0 < 0
        {
            return Err(AlgoError::InvalidLiquiditySeekingParameters);
        }
        Ok(Self {
            route,
            hidden_liquidity_bps,
            price_improvement_bps,
            minimum_quantity,
        })
    }

    /// Returns embedded SOR route candidate.
    pub const fn route(&self) -> SorRouteCandidate {
        self.route
    }

    /// Returns hidden-liquidity estimate in basis points.
    pub const fn hidden_liquidity_bps(&self) -> u16 {
        self.hidden_liquidity_bps
    }

    /// Returns price-improvement estimate in basis points.
    pub const fn price_improvement_bps(&self) -> u16 {
        self.price_improvement_bps
    }

    /// Returns minimum executable quantity requirement.
    pub const fn minimum_quantity(&self) -> OrderQty {
        self.minimum_quantity
    }
}

/// Configuration for liquidity-seeking route selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquiditySeekingConfig {
    max_routes: usize,
    probe_qty: OrderQty,
    min_score: i64,
    sweep_fill_probability_bps: u16,
    max_toxicity_bps: u16,
    hidden_liquidity_weight: u16,
    price_improvement_weight: u16,
}

impl LiquiditySeekingConfig {
    /// Creates liquidity-seeking configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidLiquiditySeekingParameters`] when route
    /// count, quantity, or bps bounds are invalid.
    pub const fn new(
        max_routes: usize,
        probe_qty: OrderQty,
        min_score: i64,
        sweep_fill_probability_bps: u16,
        max_toxicity_bps: u16,
        hidden_liquidity_weight: u16,
        price_improvement_weight: u16,
    ) -> Result<Self, AlgoError> {
        if max_routes == 0
            || probe_qty.0 <= 0
            || sweep_fill_probability_bps > 10_000
            || max_toxicity_bps > 10_000
        {
            return Err(AlgoError::InvalidLiquiditySeekingParameters);
        }
        Ok(Self {
            max_routes,
            probe_qty,
            min_score,
            sweep_fill_probability_bps,
            max_toxicity_bps,
            hidden_liquidity_weight,
            price_improvement_weight,
        })
    }

    /// Returns maximum route allocations per decision.
    pub const fn max_routes(&self) -> usize {
        self.max_routes
    }

    /// Returns probe child quantity.
    pub const fn probe_qty(&self) -> OrderQty {
        self.probe_qty
    }

    /// Returns minimum score required for route selection.
    pub const fn min_score(&self) -> i64 {
        self.min_score
    }

    /// Returns fill-probability threshold for take/sweep behavior.
    pub const fn sweep_fill_probability_bps(&self) -> u16 {
        self.sweep_fill_probability_bps
    }

    /// Returns maximum tolerated venue toxicity.
    pub const fn max_toxicity_bps(&self) -> u16 {
        self.max_toxicity_bps
    }

    /// Returns hidden-liquidity score weight.
    pub const fn hidden_liquidity_weight(&self) -> u16 {
        self.hidden_liquidity_weight
    }

    /// Returns price-improvement score weight.
    pub const fn price_improvement_weight(&self) -> u16 {
        self.price_improvement_weight
    }
}

impl Default for LiquiditySeekingConfig {
    fn default() -> Self {
        Self {
            max_routes: 4,
            probe_qty: OrderQty(1),
            min_score: 0,
            sweep_fill_probability_bps: 7_500,
            max_toxicity_bps: 1_500,
            hidden_liquidity_weight: 3,
            price_improvement_weight: 4,
        }
    }
}

/// One liquidity-seeking allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquiditySeekingAllocation {
    action: LiquiditySeekingAction,
    score: i64,
    plan: ChildOrderPlan,
}

impl LiquiditySeekingAllocation {
    /// Returns selected action.
    pub const fn action(&self) -> LiquiditySeekingAction {
        self.action
    }

    /// Returns route score.
    pub const fn score(&self) -> i64 {
        self.score
    }

    /// Returns planned child order.
    pub const fn plan(&self) -> ChildOrderPlan {
        self.plan
    }
}

/// Fixed-capacity liquidity-seeking decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquiditySeekingDecision<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    allocations: [Option<LiquiditySeekingAllocation>; N],
    len: usize,
    considered_routes: usize,
    skipped_routes: usize,
}

impl<const N: usize> LiquiditySeekingDecision<N> {
    /// Creates an empty liquidity-seeking decision.
    pub const fn new(considered_routes: usize) -> Self {
        Self {
            allocations: [None; N],
            len: 0,
            considered_routes,
            skipped_routes: 0,
        }
    }

    /// Returns allocation count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no allocations were produced.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns number of route candidates considered.
    pub const fn considered_routes(&self) -> usize {
        self.considered_routes
    }

    /// Returns number of skipped route candidates.
    pub const fn skipped_routes(&self) -> usize {
        self.skipped_routes
    }

    /// Returns allocations in selected order.
    pub fn allocations(&self) -> impl Iterator<Item = &LiquiditySeekingAllocation> {
        self.allocations[..self.len]
            .iter()
            .filter_map(Option::as_ref)
    }

    fn push(&mut self, allocation: LiquiditySeekingAllocation) -> Result<(), AlgoError> {
        if self.len == N {
            return Err(AlgoError::DecisionFull { capacity: N });
        }
        self.allocations[self.len] = Some(allocation);
        self.len += 1;
        Ok(())
    }

    fn mark_skipped(&mut self) {
        self.skipped_routes = self.skipped_routes.saturating_add(1);
    }
}

/// Deterministic liquidity-seeking planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquiditySeekingPlanner {
    config: LiquiditySeekingConfig,
    sor: SorPlanner,
}

impl LiquiditySeekingPlanner {
    /// Creates a liquidity-seeking planner.
    pub const fn new(config: LiquiditySeekingConfig, sor_config: SorConfig) -> Self {
        Self {
            config,
            sor: SorPlanner::new(sor_config),
        }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> LiquiditySeekingConfig {
        self.config
    }

    /// Scores one liquidity-seeking candidate.
    pub fn score_candidate(
        &self,
        parent: &ParentOrder,
        candidate: LiquiditySeekingCandidate,
        best_price: OrderPrice,
    ) -> Option<i64> {
        if candidate.route().metrics().toxicity_bps() > self.config.max_toxicity_bps()
            || candidate.route().available_qty().0 < candidate.minimum_quantity().0
        {
            return None;
        }
        let mut score = self
            .sor
            .score_route(parent, candidate.route(), best_price)?;
        score = score.saturating_add(
            i64::from(candidate.hidden_liquidity_bps())
                * i64::from(self.config.hidden_liquidity_weight()),
        );
        score = score.saturating_add(
            i64::from(candidate.price_improvement_bps())
                * i64::from(self.config.price_improvement_weight()),
        );
        Some(score)
    }

    /// Plans route probes or larger liquidity-taking child orders.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/id slices are invalid, a
    /// generated child order is invalid, or fixed output capacity is exhausted.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns route candidates, ids, and timestamps"
    )]
    pub fn plan_liquidity<const N: usize>(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        candidates: &[LiquiditySeekingCandidate],
        child_ids: &[ChildOrderId],
        client_order_ids: &[ClientOrderId],
        ts_recv_ns: u64,
    ) -> Result<LiquiditySeekingDecision<N>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if child_ids.len() < N || client_order_ids.len() < N {
            return Err(AlgoError::InvalidLiquiditySeekingParameters);
        }
        if now_ns < parent.start_ns() || progress.is_complete() || candidates.is_empty() {
            return Ok(LiquiditySeekingDecision::new(candidates.len()));
        }

        let mut decision = LiquiditySeekingDecision::<N>::new(candidates.len());
        let mut leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0)
            .min(parent.max_clip().0);
        let route_limit = self.config.max_routes().min(N);
        let Some(best_price) = best_liquidity_price(parent.side(), candidates) else {
            return Ok(decision);
        };

        while leaves > 0 && decision.len() < route_limit {
            let mut best_index = None;
            let mut best_score = i64::MIN;
            for (index, candidate) in candidates.iter().copied().enumerate() {
                if liquidity_decision_has_route(&decision, candidate.route().route_id()) {
                    continue;
                }
                let Some(score) = self.score_candidate(parent, candidate, best_price) else {
                    if decision.is_empty() {
                        decision.mark_skipped();
                    }
                    continue;
                };
                if score >= self.config.min_score() && score > best_score {
                    best_score = score;
                    best_index = Some(index);
                }
            }
            let Some(index) = best_index else {
                break;
            };
            if decision.len() == N {
                return Err(AlgoError::DecisionFull { capacity: N });
            }
            let candidate = candidates[index];
            let action = if candidate.route().metrics().fill_probability_bps()
                >= self.config.sweep_fill_probability_bps()
            {
                LiquiditySeekingAction::Take
            } else {
                LiquiditySeekingAction::Probe
            };
            let desired_qty = match action {
                LiquiditySeekingAction::Take => candidate.route().available_qty().0,
                LiquiditySeekingAction::Probe => self.config.probe_qty().0,
                LiquiditySeekingAction::Skip => 0,
            };
            let qty = desired_qty
                .min(candidate.route().available_qty().0)
                .min(leaves);
            let final_slice = progress.released_qty().0.saturating_add(qty) >= parent.total_qty().0
                || now_ns >= parent.end_ns();
            if qty < parent.min_clip().0 && !final_slice {
                decision.mark_skipped();
                break;
            }
            if qty <= 0 {
                break;
            }
            let request = parent.build_order_request_for_route_at_price(
                candidate.route().route_id(),
                client_order_ids[decision.len()],
                OrderQty(qty),
                candidate.route().price(),
                ts_recv_ns,
            );
            let plan =
                ChildOrderPlan::new(child_ids[decision.len()], parent.id(), request, now_ns)?;
            decision.push(LiquiditySeekingAllocation {
                action,
                score: best_score,
                plan,
            })?;
            leaves = leaves.saturating_sub(qty);
        }

        Ok(decision)
    }
}

/// Aggressive sweep configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepConfig {
    max_routes: usize,
    price_collar: OrderPrice,
    min_fill_qty: OrderQty,
}

impl SweepConfig {
    /// Creates sweep configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSweepParameters`] when route count, collar,
    /// or minimum fill quantity is invalid.
    pub const fn new(
        max_routes: usize,
        price_collar: OrderPrice,
        min_fill_qty: OrderQty,
    ) -> Result<Self, AlgoError> {
        if max_routes == 0 || price_collar.0 <= 0 || min_fill_qty.0 < 0 {
            return Err(AlgoError::InvalidSweepParameters);
        }
        Ok(Self {
            max_routes,
            price_collar,
            min_fill_qty,
        })
    }

    /// Returns maximum route/level allocations per decision.
    pub const fn max_routes(&self) -> usize {
        self.max_routes
    }

    /// Returns side-aware price collar.
    pub const fn price_collar(&self) -> OrderPrice {
        self.price_collar
    }

    /// Returns minimum total fill quantity required for a non-empty decision.
    pub const fn min_fill_qty(&self) -> OrderQty {
        self.min_fill_qty
    }
}

/// One aggressive sweep allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepAllocation {
    plan: ChildOrderPlan,
}

impl SweepAllocation {
    /// Returns planned child order for this sweep level.
    pub const fn plan(&self) -> ChildOrderPlan {
        self.plan
    }
}

/// Fixed-capacity aggressive sweep decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepDecision<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    allocations: [Option<SweepAllocation>; N],
    len: usize,
    considered_levels: usize,
    skipped_levels: usize,
    total_qty: OrderQty,
    average_price: OrderPrice,
    collar_reached: bool,
}

impl<const N: usize> SweepDecision<N> {
    /// Creates an empty sweep decision.
    pub const fn new(considered_levels: usize) -> Self {
        Self {
            allocations: [None; N],
            len: 0,
            considered_levels,
            skipped_levels: 0,
            total_qty: OrderQty(0),
            average_price: OrderPrice(0),
            collar_reached: false,
        }
    }

    /// Returns allocation count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no allocations were produced.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns considered book/route levels.
    pub const fn considered_levels(&self) -> usize {
        self.considered_levels
    }

    /// Returns skipped book/route levels.
    pub const fn skipped_levels(&self) -> usize {
        self.skipped_levels
    }

    /// Returns total planned sweep quantity.
    pub const fn total_qty(&self) -> OrderQty {
        self.total_qty
    }

    /// Returns average planned sweep price.
    pub const fn average_price(&self) -> OrderPrice {
        self.average_price
    }

    /// Returns true when at least one candidate was outside the price collar.
    pub const fn collar_reached(&self) -> bool {
        self.collar_reached
    }

    /// Returns allocations in selected order.
    pub fn allocations(&self) -> impl Iterator<Item = &SweepAllocation> {
        self.allocations[..self.len]
            .iter()
            .filter_map(Option::as_ref)
    }

    fn push(&mut self, allocation: SweepAllocation) -> Result<(), AlgoError> {
        if self.len == N {
            return Err(AlgoError::DecisionFull { capacity: N });
        }
        let qty = allocation.plan().request().quantity.0;
        let price = allocation.plan().request().limit_price.0;
        let previous_notional = i128::from(self.average_price.0) * i128::from(self.total_qty.0);
        let new_notional = previous_notional.saturating_add(i128::from(price) * i128::from(qty));
        self.total_qty = OrderQty(self.total_qty.0.saturating_add(qty));
        if self.total_qty.0 > 0 {
            self.average_price = OrderPrice(
                i64::try_from(new_notional / i128::from(self.total_qty.0)).unwrap_or(i64::MAX),
            );
        }
        self.allocations[self.len] = Some(allocation);
        self.len += 1;
        Ok(())
    }

    fn mark_skipped(&mut self) {
        self.skipped_levels = self.skipped_levels.saturating_add(1);
    }

    fn mark_collar_reached(&mut self) {
        self.collar_reached = true;
    }
}

/// Deterministic aggressive sweep planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepPlanner {
    config: SweepConfig,
}

impl SweepPlanner {
    /// Creates a sweep planner.
    pub const fn new(config: SweepConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> SweepConfig {
        self.config
    }

    /// Plans aggressive route/level child orders up to the price collar.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/id slices are invalid, a
    /// generated child order is invalid, or fixed output capacity is exhausted.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns route candidates, ids, and timestamps"
    )]
    pub fn plan_sweep<const N: usize>(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        candidates: &[SorRouteCandidate],
        child_ids: &[ChildOrderId],
        client_order_ids: &[ClientOrderId],
        ts_recv_ns: u64,
    ) -> Result<SweepDecision<N>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if child_ids.len() < N || client_order_ids.len() < N {
            return Err(AlgoError::InvalidSweepParameters);
        }
        if now_ns < parent.start_ns() || progress.is_complete() || candidates.is_empty() {
            return Ok(SweepDecision::new(candidates.len()));
        }

        let mut decision = SweepDecision::<N>::new(candidates.len());
        let mut leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0)
            .min(parent.max_clip().0);
        let route_limit = self.config.max_routes().min(N);

        while leaves > 0 && decision.len() < route_limit {
            let mut best_index = None;
            let mut best_price = match parent.side() {
                OrderSide::Buy => OrderPrice(i64::MAX),
                OrderSide::Sell => OrderPrice(0),
            };
            for (index, candidate) in candidates.iter().copied().enumerate() {
                if sweep_decision_has_level(&decision, candidate.route_id(), candidate.price()) {
                    continue;
                }
                if !candidate.status().is_routable()
                    || !candidate
                        .capability()
                        .supports_order_type(parent.order_type())
                    || candidate.available_qty().0 <= 0
                    || candidate.price().0 <= 0
                {
                    decision.mark_skipped();
                    continue;
                }
                if !price_inside_collar(
                    parent.side(),
                    candidate.price(),
                    self.config.price_collar(),
                ) {
                    decision.mark_collar_reached();
                    continue;
                }
                let better = match parent.side() {
                    OrderSide::Buy => candidate.price().0 < best_price.0,
                    OrderSide::Sell => candidate.price().0 > best_price.0,
                };
                if better {
                    best_price = candidate.price();
                    best_index = Some(index);
                }
            }
            let Some(index) = best_index else {
                break;
            };
            if decision.len() == N {
                return Err(AlgoError::DecisionFull { capacity: N });
            }
            let candidate = candidates[index];
            let qty = candidate.available_qty().0.min(leaves);
            if qty <= 0 {
                break;
            }
            let request = parent.build_order_request_for_route_at_price(
                candidate.route_id(),
                client_order_ids[decision.len()],
                OrderQty(qty),
                candidate.price(),
                ts_recv_ns,
            );
            let plan =
                ChildOrderPlan::new(child_ids[decision.len()], parent.id(), request, now_ns)?;
            decision.push(SweepAllocation { plan })?;
            leaves = leaves.saturating_sub(qty);
        }

        if decision.total_qty().0 < self.config.min_fill_qty().0 {
            return Ok(SweepDecision::new(candidates.len()));
        }
        Ok(decision)
    }
}

/// Basket or spread leg side in the portfolio objective.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BasketLegRole {
    /// Primary alpha or exposure leg.
    Primary = 1,
    /// Hedge leg intended to reduce exposure drift.
    Hedge = 2,
    /// Offset leg in a spread or relative-value structure.
    Offset = 3,
}

/// One parent order participating in a basket or spread execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasketLeg {
    parent: ParentOrder,
    role: BasketLegRole,
    hedge_ratio_bps: i32,
}

impl BasketLeg {
    /// Creates a basket leg.
    ///
    /// `hedge_ratio_bps` is metadata for audit and host-side risk checks. The
    /// first planner slice uses each leg's own parent quantity as the executable
    /// target so existing single-leg OMS semantics remain unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when the parent is invalid or hedge ratio is zero.
    pub fn new(
        parent: ParentOrder,
        role: BasketLegRole,
        hedge_ratio_bps: i32,
    ) -> Result<Self, AlgoError> {
        parent.validate()?;
        if hedge_ratio_bps == 0 {
            return Err(AlgoError::InvalidBasketParameters);
        }
        Ok(Self {
            parent,
            role,
            hedge_ratio_bps,
        })
    }

    /// Returns the leg parent order.
    pub const fn parent(&self) -> ParentOrder {
        self.parent
    }

    /// Returns leg role.
    pub const fn role(&self) -> BasketLegRole {
        self.role
    }

    /// Returns hedge ratio metadata in basis points.
    pub const fn hedge_ratio_bps(&self) -> i32 {
        self.hedge_ratio_bps
    }
}

/// Planned child allocation for one basket leg.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasketChildAllocation {
    leg_index: usize,
    role: BasketLegRole,
    target_release_qty: OrderQty,
    plan: ChildOrderPlan,
}

impl BasketChildAllocation {
    /// Returns leg index from the caller-provided leg slice.
    pub const fn leg_index(&self) -> usize {
        self.leg_index
    }

    /// Returns leg role.
    pub const fn role(&self) -> BasketLegRole {
        self.role
    }

    /// Returns cumulative target release quantity for the leg.
    pub const fn target_release_qty(&self) -> OrderQty {
        self.target_release_qty
    }

    /// Returns planned child order.
    pub const fn plan(&self) -> ChildOrderPlan {
        self.plan
    }
}

/// Fixed-capacity basket decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasketDecision<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    allocations: [Option<BasketChildAllocation>; N],
    len: usize,
    considered_legs: usize,
    blocked_legs: usize,
}

impl<const N: usize> BasketDecision<N> {
    /// Creates an empty basket decision.
    pub const fn new(considered_legs: usize) -> Self {
        Self {
            allocations: [None; N],
            len: 0,
            considered_legs,
            blocked_legs: 0,
        }
    }

    /// Returns allocation count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no allocations were produced.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns considered leg count.
    pub const fn considered_legs(&self) -> usize {
        self.considered_legs
    }

    /// Returns blocked leg count.
    pub const fn blocked_legs(&self) -> usize {
        self.blocked_legs
    }

    /// Returns allocations in leg order.
    pub fn allocations(&self) -> impl Iterator<Item = &BasketChildAllocation> {
        self.allocations[..self.len]
            .iter()
            .filter_map(Option::as_ref)
    }

    fn push(&mut self, allocation: BasketChildAllocation) -> Result<(), AlgoError> {
        if self.len == N {
            return Err(AlgoError::DecisionFull { capacity: N });
        }
        self.allocations[self.len] = Some(allocation);
        self.len += 1;
        Ok(())
    }

    fn mark_blocked(&mut self) {
        self.blocked_legs = self.blocked_legs.saturating_add(1);
    }
}

/// Deterministic synchronized basket/spread planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BasketPlanner;

impl BasketPlanner {
    /// Creates a basket planner.
    pub const fn new() -> Self {
        Self
    }

    /// Plans synchronized child slices for a basket.
    ///
    /// Each leg uses its own parent schedule and clip bounds. The function
    /// emits at most one child per due leg and writes allocations into a
    /// fixed-capacity decision. Hosts remain responsible for atomic package
    /// semantics, linked-order support, hedge drift monitoring, and venue
    /// cancel/replace orchestration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when leg/progress/id slices are inconsistent, a
    /// parent/progress pair is invalid, or the fixed output capacity is full.
    pub fn plan_synchronized_slice<const N: usize>(
        &self,
        legs: &[BasketLeg],
        progresses: &[AlgoProgress],
        now_ns: u64,
        child_ids: &[ChildOrderId],
        client_order_ids: &[ClientOrderId],
        ts_recv_ns: u64,
    ) -> Result<BasketDecision<N>, AlgoError> {
        if legs.len() != progresses.len()
            || child_ids.len() < legs.len().min(N)
            || client_order_ids.len() < legs.len().min(N)
        {
            return Err(AlgoError::InvalidBasketParameters);
        }

        let mut decision = BasketDecision::<N>::new(legs.len());
        for (index, (leg, progress)) in legs.iter().zip(progresses.iter()).enumerate() {
            let parent = leg.parent();
            parent.validate()?;
            if parent.status().is_terminal() {
                decision.mark_blocked();
                continue;
            }
            if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
                return Err(AlgoError::InvalidProgress);
            }
            if now_ns < parent.start_ns() || progress.is_complete() {
                continue;
            }
            let target_release_qty = scale_qty_bps(
                parent.total_qty().0,
                u32::from(elapsed_bps(parent.start_ns(), parent.end_ns(), now_ns)),
            );
            let due_qty = target_release_qty.saturating_sub(progress.released_qty().0);
            if due_qty <= 0 {
                continue;
            }
            let leaves = parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0);
            let mut child_qty = due_qty.min(parent.max_clip().0).min(leaves);
            let final_slice = progress.released_qty().0.saturating_add(child_qty)
                >= parent.total_qty().0
                || now_ns >= parent.end_ns();
            if child_qty < parent.min_clip().0 && !final_slice {
                continue;
            }
            if child_qty <= 0 {
                continue;
            }
            child_qty = child_qty.min(leaves);
            if decision.len() == N {
                return Err(AlgoError::DecisionFull { capacity: N });
            }
            let request = parent.build_order_request(
                client_order_ids[decision.len()],
                OrderQty(child_qty),
                ts_recv_ns,
            );
            let plan =
                ChildOrderPlan::new(child_ids[decision.len()], parent.id(), request, now_ns)?;
            decision.push(BasketChildAllocation {
                leg_index: index,
                role: leg.role(),
                target_release_qty: OrderQty(target_release_qty),
                plan,
            })?;
        }
        Ok(decision)
    }
}

/// Two-leg spread execution configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadConfig {
    ratio_bps: u32,
    min_edge_bps: i32,
}

impl SpreadConfig {
    /// Creates spread configuration.
    ///
    /// `ratio_bps` scales the sell leg price and quantity relative to the buy
    /// leg. A value of 10,000 means one-to-one.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSpreadParameters`] when ratio is zero.
    pub const fn new(ratio_bps: u32, min_edge_bps: i32) -> Result<Self, AlgoError> {
        if ratio_bps == 0 {
            return Err(AlgoError::InvalidSpreadParameters);
        }
        Ok(Self {
            ratio_bps,
            min_edge_bps,
        })
    }

    /// Returns sell-leg ratio in basis points.
    pub const fn ratio_bps(&self) -> u32 {
        self.ratio_bps
    }

    /// Returns minimum required spread edge in basis points.
    pub const fn min_edge_bps(&self) -> i32 {
        self.min_edge_bps
    }
}

/// Current executable two-leg spread prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadQuote {
    buy_price: OrderPrice,
    sell_price: OrderPrice,
}

impl SpreadQuote {
    /// Creates spread quote prices.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSpreadParameters`] when either price is not
    /// positive.
    pub const fn new(buy_price: OrderPrice, sell_price: OrderPrice) -> Result<Self, AlgoError> {
        if buy_price.0 <= 0 || sell_price.0 <= 0 {
            return Err(AlgoError::InvalidSpreadParameters);
        }
        Ok(Self {
            buy_price,
            sell_price,
        })
    }

    /// Returns executable buy-leg price.
    pub const fn buy_price(&self) -> OrderPrice {
        self.buy_price
    }

    /// Returns executable sell-leg price.
    pub const fn sell_price(&self) -> OrderPrice {
        self.sell_price
    }
}

/// Spread estimate used by [`SpreadPlanner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadEstimate {
    edge_bps: i32,
    executable: bool,
    buy_qty: OrderQty,
    sell_qty: OrderQty,
}

impl SpreadEstimate {
    /// Returns current spread edge in basis points.
    pub const fn edge_bps(&self) -> i32 {
        self.edge_bps
    }

    /// Returns true when edge meets the configured threshold.
    pub const fn executable(&self) -> bool {
        self.executable
    }

    /// Returns planned buy-leg quantity.
    pub const fn buy_qty(&self) -> OrderQty {
        self.buy_qty
    }

    /// Returns planned sell-leg quantity.
    pub const fn sell_qty(&self) -> OrderQty {
        self.sell_qty
    }
}

/// Two-leg spread decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadDecision {
    estimate: SpreadEstimate,
    buy: Option<ChildOrderPlan>,
    sell: Option<ChildOrderPlan>,
}

impl SpreadDecision {
    /// Returns estimate used by the decision.
    pub const fn estimate(&self) -> SpreadEstimate {
        self.estimate
    }

    /// Returns optional buy-leg child order.
    pub const fn buy(&self) -> Option<ChildOrderPlan> {
        self.buy
    }

    /// Returns optional sell-leg child order.
    pub const fn sell(&self) -> Option<ChildOrderPlan> {
        self.sell
    }
}

/// Deterministic two-leg pairs/spread planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadPlanner {
    config: SpreadConfig,
}

impl SpreadPlanner {
    /// Creates a spread planner.
    pub const fn new(config: SpreadConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> SpreadConfig {
        self.config
    }

    /// Estimates edge and executable leg quantities.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when leg, progress, or quote inputs are invalid.
    pub fn estimate(
        &self,
        buy_parent: &ParentOrder,
        buy_progress: AlgoProgress,
        sell_parent: &ParentOrder,
        sell_progress: AlgoProgress,
        quote: SpreadQuote,
    ) -> Result<SpreadEstimate, AlgoError> {
        self.validate_legs(buy_parent, buy_progress, sell_parent, sell_progress)?;
        let edge_bps = spread_edge_bps(
            quote.buy_price(),
            quote.sell_price(),
            self.config.ratio_bps(),
        );
        let executable = edge_bps >= self.config.min_edge_bps();
        let buy_leaves = buy_parent
            .total_qty()
            .0
            .saturating_sub(buy_progress.released_qty().0)
            .min(buy_parent.max_clip().0);
        let sell_leaves = sell_parent
            .total_qty()
            .0
            .saturating_sub(sell_progress.released_qty().0)
            .min(sell_parent.max_clip().0);
        let buy_from_sell = scale_qty_inverse_bps(sell_leaves, self.config.ratio_bps());
        let buy_qty = buy_leaves.min(buy_from_sell);
        let sell_qty = scale_qty_bps(buy_qty, self.config.ratio_bps());
        Ok(SpreadEstimate {
            edge_bps,
            executable,
            buy_qty: OrderQty(buy_qty),
            sell_qty: OrderQty(sell_qty.min(sell_leaves)),
        })
    }

    /// Plans synchronized buy/sell spread child orders.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when inputs are invalid or generated child orders
    /// fail OMS request validation.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns both legs, ids, quotes, and timestamps"
    )]
    pub fn plan_spread(
        &self,
        buy_parent: &ParentOrder,
        buy_progress: AlgoProgress,
        sell_parent: &ParentOrder,
        sell_progress: AlgoProgress,
        now_ns: u64,
        quote: SpreadQuote,
        buy_child_id: ChildOrderId,
        buy_client_order_id: ClientOrderId,
        sell_child_id: ChildOrderId,
        sell_client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<SpreadDecision, AlgoError> {
        if now_ns < buy_parent.start_ns().max(sell_parent.start_ns())
            || buy_progress.is_complete()
            || sell_progress.is_complete()
        {
            let estimate =
                self.estimate(buy_parent, buy_progress, sell_parent, sell_progress, quote)?;
            return Ok(SpreadDecision {
                estimate: SpreadEstimate {
                    executable: false,
                    ..estimate
                },
                buy: None,
                sell: None,
            });
        }
        let estimate =
            self.estimate(buy_parent, buy_progress, sell_parent, sell_progress, quote)?;
        if !estimate.executable()
            || estimate.buy_qty().0 < buy_parent.min_clip().0
            || estimate.sell_qty().0 < sell_parent.min_clip().0
        {
            return Ok(SpreadDecision {
                estimate,
                buy: None,
                sell: None,
            });
        }
        let buy_request = buy_parent.build_order_request_for_side_at_price(
            OrderSide::Buy,
            buy_client_order_id,
            estimate.buy_qty(),
            quote.buy_price(),
            ts_recv_ns,
        );
        let sell_request = sell_parent.build_order_request_for_side_at_price(
            OrderSide::Sell,
            sell_client_order_id,
            estimate.sell_qty(),
            quote.sell_price(),
            ts_recv_ns,
        );
        Ok(SpreadDecision {
            estimate,
            buy: Some(ChildOrderPlan::new(
                buy_child_id,
                buy_parent.id(),
                buy_request,
                now_ns,
            )?),
            sell: Some(ChildOrderPlan::new(
                sell_child_id,
                sell_parent.id(),
                sell_request,
                now_ns,
            )?),
        })
    }

    fn validate_legs(
        &self,
        buy_parent: &ParentOrder,
        buy_progress: AlgoProgress,
        sell_parent: &ParentOrder,
        sell_progress: AlgoProgress,
    ) -> Result<(), AlgoError> {
        buy_parent.validate()?;
        sell_parent.validate()?;
        if buy_parent.status().is_terminal() || sell_parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if buy_parent.side() != OrderSide::Buy || sell_parent.side() != OrderSide::Sell {
            return Err(AlgoError::InvalidSpreadParameters);
        }
        if buy_progress.parent_id() != buy_parent.id()
            || buy_progress.target_qty() != buy_parent.total_qty()
            || sell_progress.parent_id() != sell_parent.id()
            || sell_progress.target_qty() != sell_parent.total_qty()
        {
            return Err(AlgoError::InvalidProgress);
        }
        Ok(())
    }
}

/// Market-making context supplied by the host quote model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketMakerContext {
    fair_value: OrderPrice,
    best_bid: OrderPrice,
    best_ask: OrderPrice,
    inventory_qty: OrderQty,
    max_inventory_qty: OrderQty,
    volatility_bps: u16,
    adverse_selection_bps: u16,
}

impl MarketMakerContext {
    /// Creates market-making quote context.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidMarketMakingParameters`] when prices are
    /// non-positive/crossed or maximum inventory is not positive.
    #[allow(
        clippy::too_many_arguments,
        reason = "flat context mirrors one hot-path quote snapshot"
    )]
    pub const fn new(
        fair_value: OrderPrice,
        best_bid: OrderPrice,
        best_ask: OrderPrice,
        inventory_qty: OrderQty,
        max_inventory_qty: OrderQty,
        volatility_bps: u16,
        adverse_selection_bps: u16,
    ) -> Result<Self, AlgoError> {
        if fair_value.0 <= 0
            || best_bid.0 <= 0
            || best_ask.0 <= 0
            || best_bid.0 >= best_ask.0
            || max_inventory_qty.0 <= 0
        {
            return Err(AlgoError::InvalidMarketMakingParameters);
        }
        Ok(Self {
            fair_value,
            best_bid,
            best_ask,
            inventory_qty,
            max_inventory_qty,
            volatility_bps,
            adverse_selection_bps,
        })
    }

    /// Returns model fair value.
    pub const fn fair_value(&self) -> OrderPrice {
        self.fair_value
    }

    /// Returns current best bid.
    pub const fn best_bid(&self) -> OrderPrice {
        self.best_bid
    }

    /// Returns current best ask.
    pub const fn best_ask(&self) -> OrderPrice {
        self.best_ask
    }

    /// Returns signed inventory quantity.
    pub const fn inventory_qty(&self) -> OrderQty {
        self.inventory_qty
    }

    /// Returns absolute maximum inventory quantity.
    pub const fn max_inventory_qty(&self) -> OrderQty {
        self.max_inventory_qty
    }

    /// Returns volatility estimate in basis points.
    pub const fn volatility_bps(&self) -> u16 {
        self.volatility_bps
    }

    /// Returns adverse-selection estimate in basis points.
    pub const fn adverse_selection_bps(&self) -> u16 {
        self.adverse_selection_bps
    }
}

/// Market-making quote configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketMakerConfig {
    tick_size: OrderPrice,
    quote_qty: OrderQty,
    base_spread_bps: u16,
    min_spread_ticks: u16,
    max_spread_bps: u16,
    inventory_skew_bps: u16,
    volatility_weight_bps: u16,
    adverse_selection_weight_bps: u16,
}

impl MarketMakerConfig {
    /// Creates market-making configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidMarketMakingParameters`] when tick size,
    /// quote quantity, or spread bounds are invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "flat constructor keeps quote controls explicit"
    )]
    pub const fn new(
        tick_size: OrderPrice,
        quote_qty: OrderQty,
        base_spread_bps: u16,
        min_spread_ticks: u16,
        max_spread_bps: u16,
        inventory_skew_bps: u16,
        volatility_weight_bps: u16,
        adverse_selection_weight_bps: u16,
    ) -> Result<Self, AlgoError> {
        if tick_size.0 <= 0
            || quote_qty.0 <= 0
            || min_spread_ticks == 0
            || base_spread_bps > 10_000
            || max_spread_bps > 10_000
            || base_spread_bps > max_spread_bps
        {
            return Err(AlgoError::InvalidMarketMakingParameters);
        }
        Ok(Self {
            tick_size,
            quote_qty,
            base_spread_bps,
            min_spread_ticks,
            max_spread_bps,
            inventory_skew_bps,
            volatility_weight_bps,
            adverse_selection_weight_bps,
        })
    }

    /// Returns tick size in normalized price units.
    pub const fn tick_size(&self) -> OrderPrice {
        self.tick_size
    }

    /// Returns quote quantity.
    pub const fn quote_qty(&self) -> OrderQty {
        self.quote_qty
    }

    /// Returns base spread in basis points.
    pub const fn base_spread_bps(&self) -> u16 {
        self.base_spread_bps
    }

    /// Returns minimum spread in ticks.
    pub const fn min_spread_ticks(&self) -> u16 {
        self.min_spread_ticks
    }

    /// Returns maximum spread in basis points.
    pub const fn max_spread_bps(&self) -> u16 {
        self.max_spread_bps
    }

    /// Returns inventory skew strength in basis points.
    pub const fn inventory_skew_bps(&self) -> u16 {
        self.inventory_skew_bps
    }

    /// Returns volatility spread weight in basis points.
    pub const fn volatility_weight_bps(&self) -> u16 {
        self.volatility_weight_bps
    }

    /// Returns adverse-selection spread weight in basis points.
    pub const fn adverse_selection_weight_bps(&self) -> u16 {
        self.adverse_selection_weight_bps
    }
}

impl Default for MarketMakerConfig {
    fn default() -> Self {
        Self::new(
            OrderPrice(1),
            OrderQty(1),
            10,
            1,
            1_000,
            1_000,
            5_000,
            5_000,
        )
        .expect("static market-making defaults are valid")
    }
}

/// Market-making quote estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketMakerQuoteEstimate {
    adjusted_fair_value: OrderPrice,
    bid_price: OrderPrice,
    ask_price: OrderPrice,
    spread_bps: u16,
    quote_bid: bool,
    quote_ask: bool,
}

impl MarketMakerQuoteEstimate {
    /// Returns inventory-skewed fair value.
    pub const fn adjusted_fair_value(&self) -> OrderPrice {
        self.adjusted_fair_value
    }

    /// Returns bid quote price.
    pub const fn bid_price(&self) -> OrderPrice {
        self.bid_price
    }

    /// Returns ask quote price.
    pub const fn ask_price(&self) -> OrderPrice {
        self.ask_price
    }

    /// Returns selected spread in basis points.
    pub const fn spread_bps(&self) -> u16 {
        self.spread_bps
    }

    /// Returns true when a bid quote is allowed.
    pub const fn quote_bid(&self) -> bool {
        self.quote_bid
    }

    /// Returns true when an ask quote is allowed.
    pub const fn quote_ask(&self) -> bool {
        self.quote_ask
    }
}

/// Market-making quote decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketMakerQuoteDecision {
    estimate: MarketMakerQuoteEstimate,
    bid: Option<ChildOrderPlan>,
    ask: Option<ChildOrderPlan>,
}

impl MarketMakerQuoteDecision {
    /// Returns the quote estimate.
    pub const fn estimate(&self) -> MarketMakerQuoteEstimate {
        self.estimate
    }

    /// Returns optional bid child order plan.
    pub const fn bid(&self) -> Option<ChildOrderPlan> {
        self.bid
    }

    /// Returns optional ask child order plan.
    pub const fn ask(&self) -> Option<ChildOrderPlan> {
        self.ask
    }
}

/// Deterministic market-making quote planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketMakerPlanner {
    config: MarketMakerConfig,
}

impl MarketMakerPlanner {
    /// Creates a market-making planner.
    pub const fn new(config: MarketMakerConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> MarketMakerConfig {
        self.config
    }

    /// Estimates bid/ask quote prices and inventory-side quote suppression.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when template or context is invalid.
    pub fn estimate(
        &self,
        template: &ParentOrder,
        context: MarketMakerContext,
    ) -> Result<MarketMakerQuoteEstimate, AlgoError> {
        template.validate()?;
        self.validate_context(context)?;
        let inventory_ratio_bps =
            signed_inventory_ratio_bps(context.inventory_qty().0, context.max_inventory_qty().0);
        let skew_bps = scale_signed_bps(
            inventory_ratio_bps,
            i32::from(self.config.inventory_skew_bps()),
        );
        let adjusted_fair_value = apply_price_bps(context.fair_value(), -skew_bps);

        let mut spread_bps = self.config.base_spread_bps();
        spread_bps = spread_bps.saturating_add(
            u16::try_from(scale_bps_u32(
                u32::from(context.volatility_bps()),
                u32::from(self.config.volatility_weight_bps()),
            ))
            .unwrap_or(u16::MAX),
        );
        spread_bps = spread_bps.saturating_add(
            u16::try_from(scale_bps_u32(
                u32::from(context.adverse_selection_bps()),
                u32::from(self.config.adverse_selection_weight_bps()),
            ))
            .unwrap_or(u16::MAX),
        );
        spread_bps = spread_bps.min(self.config.max_spread_bps());

        let min_spread = self
            .config
            .tick_size()
            .0
            .saturating_mul(i64::from(self.config.min_spread_ticks()));
        let spread_price = price_bps_to_ticks(adjusted_fair_value, spread_bps).max(min_spread);
        let half = (spread_price / 2).max(self.config.tick_size().0);
        let bid_price = snap_down_to_tick(
            adjusted_fair_value.0.saturating_sub(half),
            self.config.tick_size().0,
        );
        let ask_price = snap_up_to_tick(
            adjusted_fair_value.0.saturating_add(half),
            self.config.tick_size().0,
        );
        let quote_bid = context.inventory_qty().0 < context.max_inventory_qty().0;
        let quote_ask = context.inventory_qty().0 > -context.max_inventory_qty().0;

        Ok(MarketMakerQuoteEstimate {
            adjusted_fair_value,
            bid_price: OrderPrice(bid_price.max(self.config.tick_size().0)),
            ask_price: OrderPrice(
                ask_price.max(bid_price.saturating_add(self.config.tick_size().0)),
            ),
            spread_bps,
            quote_bid,
            quote_ask,
        })
    }

    /// Plans bid and ask child quote orders.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when inputs are invalid or a generated quote order
    /// fails OMS request validation.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns quote identifiers and timestamps"
    )]
    pub fn plan_quotes(
        &self,
        template: &ParentOrder,
        now_ns: u64,
        context: MarketMakerContext,
        bid_child_id: ChildOrderId,
        bid_client_order_id: ClientOrderId,
        ask_child_id: ChildOrderId,
        ask_client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<MarketMakerQuoteDecision, AlgoError> {
        if template.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        let estimate = self.estimate(template, context)?;
        let bid = if estimate.quote_bid() {
            let request = template.build_order_request_for_side_at_price(
                OrderSide::Buy,
                bid_client_order_id,
                self.config.quote_qty(),
                estimate.bid_price(),
                ts_recv_ns,
            );
            Some(ChildOrderPlan::new(
                bid_child_id,
                template.id(),
                request,
                now_ns,
            )?)
        } else {
            None
        };
        let ask = if estimate.quote_ask() {
            let request = template.build_order_request_for_side_at_price(
                OrderSide::Sell,
                ask_client_order_id,
                self.config.quote_qty(),
                estimate.ask_price(),
                ts_recv_ns,
            );
            Some(ChildOrderPlan::new(
                ask_child_id,
                template.id(),
                request,
                now_ns,
            )?)
        } else {
            None
        };
        Ok(MarketMakerQuoteDecision { estimate, bid, ask })
    }

    fn validate_context(&self, context: MarketMakerContext) -> Result<(), AlgoError> {
        if self.config.tick_size().0 <= 0
            || self.config.quote_qty().0 <= 0
            || self.config.base_spread_bps() > self.config.max_spread_bps()
            || context.fair_value().0 <= 0
            || context.best_bid().0 <= 0
            || context.best_ask().0 <= 0
            || context.best_bid().0 >= context.best_ask().0
            || context.max_inventory_qty().0 <= 0
        {
            return Err(AlgoError::InvalidMarketMakingParameters);
        }
        Ok(())
    }
}

/// Market context for implementation-shortfall planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplementationShortfallContext {
    arrival_price: OrderPrice,
    reference_price: OrderPrice,
    volatility_bps: u16,
    spread_bps: u16,
    temporary_impact_bps: u16,
}

impl ImplementationShortfallContext {
    /// Creates implementation-shortfall market context.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidShortfallParameters`] when either price is
    /// not positive.
    pub const fn new(
        arrival_price: OrderPrice,
        reference_price: OrderPrice,
        volatility_bps: u16,
        spread_bps: u16,
        temporary_impact_bps: u16,
    ) -> Result<Self, AlgoError> {
        if arrival_price.0 <= 0 || reference_price.0 <= 0 {
            return Err(AlgoError::InvalidShortfallParameters);
        }
        Ok(Self {
            arrival_price,
            reference_price,
            volatility_bps,
            spread_bps,
            temporary_impact_bps,
        })
    }

    /// Returns the arrival benchmark price.
    pub const fn arrival_price(&self) -> OrderPrice {
        self.arrival_price
    }

    /// Returns the current decision/reference price.
    pub const fn reference_price(&self) -> OrderPrice {
        self.reference_price
    }

    /// Returns estimated short-horizon volatility in basis points.
    pub const fn volatility_bps(&self) -> u16 {
        self.volatility_bps
    }

    /// Returns current spread in basis points.
    pub const fn spread_bps(&self) -> u16 {
        self.spread_bps
    }

    /// Returns temporary market-impact estimate in basis points.
    pub const fn temporary_impact_bps(&self) -> u16 {
        self.temporary_impact_bps
    }
}

/// Configuration for implementation-shortfall planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplementationShortfallConfig {
    base_urgency_bps: u16,
    max_urgency_bps: u16,
    volatility_weight_bps: u16,
    spread_weight_bps: u16,
    adverse_move_weight_bps: u16,
    impact_weight_bps: u16,
}

impl ImplementationShortfallConfig {
    /// Creates implementation-shortfall planner configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidShortfallParameters`] when urgency values
    /// exceed 10,000 bps or base urgency exceeds the cap.
    pub const fn new(
        base_urgency_bps: u16,
        max_urgency_bps: u16,
        volatility_weight_bps: u16,
        spread_weight_bps: u16,
        adverse_move_weight_bps: u16,
        impact_weight_bps: u16,
    ) -> Result<Self, AlgoError> {
        if base_urgency_bps > 10_000
            || max_urgency_bps > 10_000
            || base_urgency_bps > max_urgency_bps
        {
            return Err(AlgoError::InvalidShortfallParameters);
        }
        Ok(Self {
            base_urgency_bps,
            max_urgency_bps,
            volatility_weight_bps,
            spread_weight_bps,
            adverse_move_weight_bps,
            impact_weight_bps,
        })
    }

    /// Returns base urgency in basis points.
    pub const fn base_urgency_bps(&self) -> u16 {
        self.base_urgency_bps
    }

    /// Returns maximum urgency in basis points.
    pub const fn max_urgency_bps(&self) -> u16 {
        self.max_urgency_bps
    }

    /// Returns volatility urgency weight in basis points.
    pub const fn volatility_weight_bps(&self) -> u16 {
        self.volatility_weight_bps
    }

    /// Returns spread urgency weight in basis points.
    pub const fn spread_weight_bps(&self) -> u16 {
        self.spread_weight_bps
    }

    /// Returns adverse move urgency weight in basis points.
    pub const fn adverse_move_weight_bps(&self) -> u16 {
        self.adverse_move_weight_bps
    }

    /// Returns temporary impact patience weight in basis points.
    pub const fn impact_weight_bps(&self) -> u16 {
        self.impact_weight_bps
    }
}

impl Default for ImplementationShortfallConfig {
    fn default() -> Self {
        Self {
            base_urgency_bps: 1_000,
            max_urgency_bps: 7_500,
            volatility_weight_bps: 5_000,
            spread_weight_bps: 2_500,
            adverse_move_weight_bps: 7_500,
            impact_weight_bps: 3_000,
        }
    }
}

/// Implementation-shortfall planning estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplementationShortfallEstimate {
    elapsed_bps: u16,
    adverse_move_bps: u16,
    urgency_bps: u16,
    target_release_qty: OrderQty,
}

impl ImplementationShortfallEstimate {
    /// Returns elapsed parent interval in basis points.
    pub const fn elapsed_bps(&self) -> u16 {
        self.elapsed_bps
    }

    /// Returns adverse move from arrival price in basis points.
    pub const fn adverse_move_bps(&self) -> u16 {
        self.adverse_move_bps
    }

    /// Returns urgency used for the decision in basis points.
    pub const fn urgency_bps(&self) -> u16 {
        self.urgency_bps
    }

    /// Returns target cumulative released quantity.
    pub const fn target_release_qty(&self) -> OrderQty {
        self.target_release_qty
    }
}

/// Deterministic implementation-shortfall planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplementationShortfallPlanner {
    config: ImplementationShortfallConfig,
}

impl ImplementationShortfallPlanner {
    /// Creates an implementation-shortfall planner.
    pub const fn new(config: ImplementationShortfallConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> ImplementationShortfallConfig {
        self.config
    }

    /// Estimates the current implementation-shortfall target.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/context state is invalid.
    pub fn estimate(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        context: ImplementationShortfallContext,
    ) -> Result<ImplementationShortfallEstimate, AlgoError> {
        parent.validate()?;
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if context.arrival_price().0 <= 0 || context.reference_price().0 <= 0 {
            return Err(AlgoError::InvalidShortfallParameters);
        }
        let elapsed_bps = elapsed_bps(parent.start_ns(), parent.end_ns(), now_ns);
        let adverse_move_bps = adverse_move_bps(
            parent.side(),
            context.arrival_price(),
            context.reference_price(),
        );
        let urgency_bps = self.urgency_bps(context, adverse_move_bps);
        let remaining_time_bps = 10_000_u16.saturating_sub(elapsed_bps);
        let target_bps = u32::from(elapsed_bps).saturating_add(scale_bps_u32(
            u32::from(remaining_time_bps),
            u32::from(urgency_bps),
        ));
        let target_release_qty = scale_qty_bps(parent.total_qty().0, target_bps.min(10_000));
        Ok(ImplementationShortfallEstimate {
            elapsed_bps,
            adverse_move_bps,
            urgency_bps,
            target_release_qty: OrderQty(target_release_qty),
        })
    }

    /// Plans one implementation-shortfall child slice.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/context state is invalid or
    /// the generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_shortfall_slice(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        context: ImplementationShortfallContext,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<Option<ChildOrderPlan>, AlgoError> {
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if now_ns < parent.start_ns() || progress.is_complete() {
            return Ok(None);
        }

        let estimate = self.estimate(parent, progress, now_ns, context)?;
        let due_qty = estimate
            .target_release_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        if due_qty <= 0 {
            return Ok(None);
        }
        let leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        let mut child_qty = due_qty.min(parent.max_clip().0).min(leaves);
        let final_slice = progress.released_qty().0.saturating_add(child_qty)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if child_qty < parent.min_clip().0 && !final_slice {
            return Ok(None);
        }
        if child_qty <= 0 {
            return Ok(None);
        }
        child_qty = child_qty.min(
            parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0),
        );
        let request = parent.build_order_request(client_order_id, OrderQty(child_qty), ts_recv_ns);
        Ok(Some(ChildOrderPlan::new(
            child_id,
            parent.id(),
            request,
            now_ns,
        )?))
    }

    fn urgency_bps(&self, context: ImplementationShortfallContext, adverse_move_bps: u16) -> u16 {
        let mut urgency = i64::from(self.config.base_urgency_bps());
        urgency = urgency.saturating_add(i64::from(scale_bps_u32(
            u32::from(context.volatility_bps()),
            u32::from(self.config.volatility_weight_bps()),
        )));
        urgency = urgency.saturating_add(i64::from(scale_bps_u32(
            u32::from(context.spread_bps()),
            u32::from(self.config.spread_weight_bps()),
        )));
        urgency = urgency.saturating_add(i64::from(scale_bps_u32(
            u32::from(adverse_move_bps),
            u32::from(self.config.adverse_move_weight_bps()),
        )));
        urgency = urgency.saturating_sub(i64::from(scale_bps_u32(
            u32::from(context.temporary_impact_bps()),
            u32::from(self.config.impact_weight_bps()),
        )));
        let bounded = urgency.clamp(0, i64::from(self.config.max_urgency_bps()));
        u16::try_from(bounded).unwrap_or(self.config.max_urgency_bps())
    }
}

/// Replay event consumed by an algorithm harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[allow(
    clippy::large_enum_variant,
    reason = "replay events remain Copy so caller-owned replay buffers avoid boxing"
)]
pub enum AlgoReplayEvent {
    /// Deterministic timer tick at `timestamp_ns`.
    Timer {
        /// Timer timestamp in nanoseconds.
        timestamp_ns: u64,
    },
    /// Canonical OMS execution event.
    Execution(ExecutionEvent),
    /// Parent lifecycle status update.
    ParentStatus {
        /// New parent status.
        status: ParentOrderStatus,
    },
}

/// Sequenced replay input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoReplayInput {
    sequence: u64,
    event: AlgoReplayEvent,
}

impl AlgoReplayInput {
    /// Creates a replay input.
    pub const fn new(sequence: u64, event: AlgoReplayEvent) -> Self {
        Self { sequence, event }
    }

    /// Returns the replay input sequence.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Returns the replay event.
    pub const fn event(&self) -> AlgoReplayEvent {
        self.event
    }
}

/// Deterministic child/client id generation prefixes for replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoReplayIdScheme {
    child_prefix: FixedAscii<24>,
    client_prefix: FixedAscii<24>,
}

impl AlgoReplayIdScheme {
    /// Creates replay id prefixes.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionCoreError`] when a prefix is non-ASCII or too long.
    pub fn new(child_prefix: &str, client_prefix: &str) -> Result<Self, ExecutionCoreError> {
        Ok(Self {
            child_prefix: FixedAscii::new(child_prefix)?,
            client_prefix: FixedAscii::new(client_prefix)?,
        })
    }

    /// Returns the child id prefix.
    pub const fn child_prefix(&self) -> FixedAscii<24> {
        self.child_prefix
    }

    /// Returns the client order id prefix.
    pub const fn client_prefix(&self) -> FixedAscii<24> {
        self.client_prefix
    }

    fn child_id(&self, index: u64) -> Result<ChildOrderId, AlgoError> {
        fixed_id_with_index(self.child_prefix.as_str(), index)
    }

    fn client_order_id(&self, index: u64) -> Result<ClientOrderId, AlgoError> {
        fixed_id_with_index(self.client_prefix.as_str(), index)
    }
}

impl Default for AlgoReplayIdScheme {
    fn default() -> Self {
        Self::new("child", "cl").expect("static replay id prefixes are valid")
    }
}

/// Replay step emitted for one input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoReplayStep<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    input_sequence: u64,
    event: AlgoReplayEvent,
    progress_before: AlgoProgress,
    progress_after: AlgoProgress,
    decision: AlgoDecision<N>,
}

impl<const N: usize> AlgoReplayStep<N> {
    /// Returns the replay input sequence.
    pub const fn input_sequence(&self) -> u64 {
        self.input_sequence
    }

    /// Returns the replay event.
    pub const fn event(&self) -> AlgoReplayEvent {
        self.event
    }

    /// Returns progress before applying the event.
    pub const fn progress_before(&self) -> AlgoProgress {
        self.progress_before
    }

    /// Returns progress after applying the event and any generated decision.
    pub const fn progress_after(&self) -> AlgoProgress {
        self.progress_after
    }

    /// Returns the decision generated for this input.
    pub const fn decision(&self) -> &AlgoDecision<N> {
        &self.decision
    }
}

/// Summary returned by deterministic TWAP replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoReplaySummary {
    input_events: u64,
    decisions: u64,
    actions: u64,
    submitted_children: u64,
    final_progress: AlgoProgress,
    deterministic_hash: u64,
}

impl AlgoReplaySummary {
    /// Returns replay input event count.
    pub const fn input_events(&self) -> u64 {
        self.input_events
    }

    /// Returns emitted decision count.
    pub const fn decisions(&self) -> u64 {
        self.decisions
    }

    /// Returns total emitted action count.
    pub const fn actions(&self) -> u64 {
        self.actions
    }

    /// Returns submitted child action count.
    pub const fn submitted_children(&self) -> u64 {
        self.submitted_children
    }

    /// Returns final parent progress.
    pub const fn final_progress(&self) -> AlgoProgress {
        self.final_progress
    }

    /// Returns deterministic replay hash for regression checks.
    pub const fn deterministic_hash(&self) -> u64 {
        self.deterministic_hash
    }
}

/// Replays TWAP planning over explicit deterministic inputs.
///
/// `out` is caller-owned and cleared before replay. This keeps allocation
/// policy outside the harness and lets test/benchmark callers reuse capacity.
///
/// # Errors
///
/// Returns [`AlgoError`] when parent/progress state is invalid, a generated
/// child plan is invalid, or the fixed-capacity decision cannot hold a due
/// action.
pub fn replay_twap_into<const N: usize>(
    parent: ParentOrder,
    planner: TwapSlicePlanner,
    inputs: &[AlgoReplayInput],
    id_scheme: AlgoReplayIdScheme,
    out: &mut Vec<AlgoReplayStep<N>>,
) -> Result<AlgoReplaySummary, AlgoError> {
    parent.validate()?;
    out.clear();

    let mut active_parent = parent;
    let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());
    let mut decisions = 0_u64;
    let mut actions = 0_u64;
    let mut submitted_children = 0_u64;
    let mut hash = FNV_OFFSET_BASIS;

    for input in inputs {
        let before = progress;
        let mut decision = AlgoDecision::<N>::new(decisions.saturating_add(1));
        match input.event() {
            AlgoReplayEvent::Timer { timestamp_ns } => {
                let next_child = submitted_children.saturating_add(1);
                if let Some(plan) = planner.plan_due_slice(
                    &active_parent,
                    progress,
                    timestamp_ns,
                    id_scheme.child_id(next_child)?,
                    id_scheme.client_order_id(next_child)?,
                    timestamp_ns,
                )? {
                    decision.push(AlgoAction::SubmitChild(plan))?;
                    progress.on_child_released(&plan)?;
                    submitted_children = submitted_children.saturating_add(1);
                }
            }
            AlgoReplayEvent::Execution(event) => {
                progress.on_execution_event(&event);
            }
            AlgoReplayEvent::ParentStatus { status } => {
                active_parent = active_parent.with_status(status);
            }
        }

        decisions = decisions.saturating_add(1);
        actions = actions.saturating_add(usize_to_u64(decision.len()));
        hash = hash_replay_step(hash, input.sequence(), input.event(), progress, &decision);
        out.push(AlgoReplayStep {
            input_sequence: input.sequence(),
            event: input.event(),
            progress_before: before,
            progress_after: progress,
            decision,
        });
    }

    Ok(AlgoReplaySummary {
        input_events: usize_to_u64(inputs.len()),
        decisions,
        actions,
        submitted_children,
        final_progress: progress,
        deterministic_hash: hash,
    })
}

fn div_ceil_u64(lhs: u64, rhs: u64) -> u64 {
    lhs / rhs + u64::from(!lhs.is_multiple_of(rhs))
}

fn div_ceil_i128(lhs: i128, rhs: i128) -> i128 {
    lhs / rhs + i128::from(lhs % rhs != 0)
}

fn participation_qty(volume: i64, bps: u16) -> i64 {
    let value = i128::from(volume) * i128::from(bps);
    i64::try_from(value / 10_000).unwrap_or(i64::MAX)
}

fn vwap_target_qty(parent_qty: i64, cumulative_weight: u64, total_weight: u64) -> i64 {
    let value = i128::from(parent_qty) * i128::from(cumulative_weight);
    i64::try_from(value / i128::from(total_weight)).unwrap_or(i64::MAX)
}

fn elapsed_bps(start_ns: u64, end_ns: u64, now_ns: u64) -> u16 {
    if now_ns <= start_ns {
        return 0;
    }
    if now_ns >= end_ns {
        return 10_000;
    }
    let elapsed = now_ns.saturating_sub(start_ns);
    let total = end_ns.saturating_sub(start_ns).max(1);
    u16::try_from((u128::from(elapsed) * 10_000) / u128::from(total)).unwrap_or(10_000)
}

fn scale_bps_u32(value: u32, bps: u32) -> u32 {
    let scaled = (u128::from(value) * u128::from(bps)) / 10_000;
    u32::try_from(scaled).unwrap_or(u32::MAX)
}

fn scale_qty_bps(quantity: i64, bps: u32) -> i64 {
    let scaled = (i128::from(quantity) * i128::from(bps)) / 10_000;
    i64::try_from(scaled).unwrap_or(i64::MAX)
}

fn participation_bps(child_qty: i64, market_volume: i64) -> u32 {
    if child_qty <= 0 || market_volume <= 0 {
        return 0;
    }
    let bps = (i128::from(child_qty) * 10_000) / i128::from(market_volume);
    u32::try_from(bps.clamp(0, i128::from(u32::MAX))).unwrap_or(u32::MAX)
}

fn price_distance_bps(price: OrderPrice, reference: OrderPrice) -> u32 {
    if price.0 <= 0 || reference.0 <= 0 {
        return u32::MAX;
    }
    let distance = price.0.abs_diff(reference.0);
    let bps = (u128::from(distance) * 10_000) / i64_to_u128(reference.0);
    u32::try_from(bps.min(u128::from(u32::MAX))).unwrap_or(u32::MAX)
}

fn child_notional(child: &ChildOrderPlan) -> u128 {
    i64_to_u128(child.request().quantity.0)
        .saturating_mul(i64_to_u128(child.request().limit_price.0))
}

fn completion_bps(completed_qty: i64, target_qty: i64) -> u16 {
    if completed_qty <= 0 || target_qty <= 0 {
        return 0;
    }
    let bps = (i128::from(completed_qty) * 10_000) / i128::from(target_qty);
    u16::try_from(bps.clamp(0, 10_000)).unwrap_or(10_000)
}

fn average_price_from_notional(notional: u128, quantity: OrderQty) -> OrderPrice {
    if notional == 0 || quantity.0 <= 0 {
        return OrderPrice(0);
    }
    let avg = notional / i64_to_u128(quantity.0);
    OrderPrice(i64::try_from(avg).unwrap_or(i64::MAX))
}

fn side_slippage_bps(side: OrderSide, average_price: OrderPrice, benchmark: OrderPrice) -> i32 {
    if average_price.0 <= 0 || benchmark.0 <= 0 {
        return 0;
    }
    let raw = match side {
        OrderSide::Buy => i128::from(average_price.0) - i128::from(benchmark.0),
        OrderSide::Sell => i128::from(benchmark.0) - i128::from(average_price.0),
    };
    let bps = (raw * 10_000) / i128::from(benchmark.0);
    i32::try_from(bps.clamp(i128::from(i32::MIN), i128::from(i32::MAX))).unwrap_or(0)
}

fn optional_slippage_bps(side: OrderSide, average_price: OrderPrice, benchmark: OrderPrice) -> i32 {
    if benchmark.0 <= 0 {
        0
    } else {
        side_slippage_bps(side, average_price, benchmark)
    }
}

fn average_latency_ns(total_latency_ns: u128, samples: u64) -> u64 {
    if samples == 0 {
        return 0;
    }
    u64::try_from(total_latency_ns / u128::from(samples)).unwrap_or(u64::MAX)
}

fn i64_to_u128(value: i64) -> u128 {
    u128::try_from(value.max(0)).unwrap_or(0)
}

fn stronger_risk_outcome(current: AlgoRiskOutcome, kind: AlgoRiskViolationKind) -> AlgoRiskOutcome {
    if matches!(
        kind,
        AlgoRiskViolationKind::KillSwitchActive | AlgoRiskViolationKind::OperatorPaused
    ) {
        return AlgoRiskOutcome::KillSwitch;
    }
    if matches!(current, AlgoRiskOutcome::KillSwitch) {
        AlgoRiskOutcome::KillSwitch
    } else {
        AlgoRiskOutcome::Block
    }
}

fn scale_qty_inverse_bps(quantity: i64, bps: u32) -> i64 {
    if bps == 0 {
        return 0;
    }
    let scaled = (i128::from(quantity) * 10_000) / i128::from(bps);
    i64::try_from(scaled).unwrap_or(i64::MAX)
}

fn spread_edge_bps(buy_price: OrderPrice, sell_price: OrderPrice, ratio_bps: u32) -> i32 {
    if buy_price.0 <= 0 || sell_price.0 <= 0 || ratio_bps == 0 {
        return i32::MIN;
    }
    let scaled_sell = (i128::from(sell_price.0) * i128::from(ratio_bps)) / 10_000;
    let edge = ((scaled_sell - i128::from(buy_price.0)) * 10_000) / i128::from(buy_price.0);
    i32::try_from(edge.clamp(i128::from(i32::MIN), i128::from(i32::MAX))).unwrap_or(0)
}

fn passive_candidate_qty(parent: &ParentOrder, progress: AlgoProgress, now_ns: u64) -> OrderQty {
    let leaves = parent
        .total_qty()
        .0
        .saturating_sub(progress.released_qty().0);
    if leaves <= 0 {
        return OrderQty(0);
    }
    let mut qty = leaves.min(parent.max_clip().0);
    let final_slice = progress.released_qty().0.saturating_add(qty) >= parent.total_qty().0
        || now_ns >= parent.end_ns();
    if qty < parent.min_clip().0 && final_slice {
        qty = leaves;
    }
    OrderQty(qty.min(leaves))
}

fn queue_fill_probability_bps(queue_ahead_qty: i64, child_qty: i64, expected_take_qty: i64) -> u16 {
    if child_qty <= 0 || expected_take_qty <= 0 {
        return 0;
    }
    let required = queue_ahead_qty.max(0).saturating_add(child_qty);
    if required <= 0 {
        return 10_000;
    }
    let bps = (i128::from(expected_take_qty) * 10_000) / i128::from(required);
    u16::try_from(bps.clamp(0, 10_000)).unwrap_or(10_000)
}

fn midpoint_price(
    side: OrderSide,
    context: PassiveQueueContext,
    tick_size: OrderPrice,
) -> OrderPrice {
    let bid = context.best_bid().0;
    let ask = context.best_ask().0;
    let mid = bid.saturating_add(ask.saturating_sub(bid) / 2);
    match side {
        OrderSide::Buy => {
            let ticks_from_bid = mid.saturating_sub(bid) / tick_size.0;
            OrderPrice(bid.saturating_add(ticks_from_bid.saturating_mul(tick_size.0)))
        }
        OrderSide::Sell => {
            let ticks_from_ask = ask.saturating_sub(mid) / tick_size.0;
            OrderPrice(ask.saturating_sub(ticks_from_ask.saturating_mul(tick_size.0)))
        }
    }
}

fn route_liquidity_bps(available_qty: i64, target_qty: i64) -> u16 {
    if available_qty <= 0 || target_qty <= 0 {
        return 0;
    }
    let bps = (i128::from(available_qty) * 10_000) / i128::from(target_qty);
    u16::try_from(bps.clamp(0, 10_000)).unwrap_or(10_000)
}

fn route_price_penalty_bps(side: OrderSide, price: OrderPrice, best_price: OrderPrice) -> u16 {
    if price.0 <= 0 || best_price.0 <= 0 {
        return 10_000;
    }
    let penalty_ticks = match side {
        OrderSide::Buy => price.0.saturating_sub(best_price.0),
        OrderSide::Sell => best_price.0.saturating_sub(price.0),
    };
    if penalty_ticks <= 0 {
        return 0;
    }
    let bps = (i128::from(penalty_ticks) * 10_000) / i128::from(best_price.0);
    u16::try_from(bps.clamp(0, 10_000)).unwrap_or(10_000)
}

fn best_sor_price(side: OrderSide, candidates: &[SorRouteCandidate]) -> Option<OrderPrice> {
    let mut best: Option<OrderPrice> = None;
    for candidate in candidates {
        if !candidate.status().is_routable()
            || candidate.price().0 <= 0
            || candidate.available_qty().0 <= 0
        {
            continue;
        }
        best = Some(match (side, best) {
            (_, None) => candidate.price(),
            (OrderSide::Buy, Some(current)) => OrderPrice(current.0.min(candidate.price().0)),
            (OrderSide::Sell, Some(current)) => OrderPrice(current.0.max(candidate.price().0)),
        });
    }
    best
}

fn decision_has_route<const N: usize>(decision: &SorDecision<N>, route_id: RouteId) -> bool {
    decision
        .allocations()
        .any(|allocation| allocation.plan().request().route_id == route_id)
}

fn liquidity_decision_has_route<const N: usize>(
    decision: &LiquiditySeekingDecision<N>,
    route_id: RouteId,
) -> bool {
    decision
        .allocations()
        .any(|allocation| allocation.plan().request().route_id == route_id)
}

fn sweep_decision_has_level<const N: usize>(
    decision: &SweepDecision<N>,
    route_id: RouteId,
    price: OrderPrice,
) -> bool {
    decision.allocations().any(|allocation| {
        allocation.plan().request().route_id == route_id
            && allocation.plan().request().limit_price == price
    })
}

fn price_inside_collar(side: OrderSide, price: OrderPrice, collar: OrderPrice) -> bool {
    match side {
        OrderSide::Buy => price.0 <= collar.0,
        OrderSide::Sell => price.0 >= collar.0,
    }
}

fn best_liquidity_price(
    side: OrderSide,
    candidates: &[LiquiditySeekingCandidate],
) -> Option<OrderPrice> {
    let mut best: Option<OrderPrice> = None;
    for candidate in candidates {
        let route = candidate.route();
        if !route.status().is_routable() || route.price().0 <= 0 || route.available_qty().0 <= 0 {
            continue;
        }
        best = Some(match (side, best) {
            (_, None) => route.price(),
            (OrderSide::Buy, Some(current)) => OrderPrice(current.0.min(route.price().0)),
            (OrderSide::Sell, Some(current)) => OrderPrice(current.0.max(route.price().0)),
        });
    }
    best
}

fn signed_inventory_ratio_bps(inventory_qty: i64, max_inventory_qty: i64) -> i32 {
    if max_inventory_qty <= 0 {
        return 0;
    }
    let ratio = (i128::from(inventory_qty) * 10_000) / i128::from(max_inventory_qty);
    i32::try_from(ratio.clamp(-10_000, 10_000)).unwrap_or(0)
}

fn scale_signed_bps(value_bps: i32, weight_bps: i32) -> i32 {
    let scaled = (i64::from(value_bps) * i64::from(weight_bps)) / 10_000;
    i32::try_from(scaled.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(0)
}

fn apply_price_bps(price: OrderPrice, adjustment_bps: i32) -> OrderPrice {
    let adjustment = (i128::from(price.0) * i128::from(adjustment_bps)) / 10_000;
    let adjusted = i128::from(price.0).saturating_add(adjustment);
    OrderPrice(i64::try_from(adjusted.max(1)).unwrap_or(i64::MAX))
}

fn price_bps_to_ticks(price: OrderPrice, bps: u16) -> i64 {
    let value = (i128::from(price.0) * i128::from(bps)) / 10_000;
    i64::try_from(value.max(1)).unwrap_or(i64::MAX)
}

fn snap_down_to_tick(price: i64, tick_size: i64) -> i64 {
    if tick_size <= 0 {
        return price;
    }
    price - price.rem_euclid(tick_size)
}

fn snap_up_to_tick(price: i64, tick_size: i64) -> i64 {
    if tick_size <= 0 {
        return price;
    }
    let rem = price.rem_euclid(tick_size);
    if rem == 0 {
        price
    } else {
        price.saturating_add(tick_size.saturating_sub(rem))
    }
}

fn adverse_move_bps(side: OrderSide, arrival: OrderPrice, reference: OrderPrice) -> u16 {
    let adverse_ticks = match side {
        OrderSide::Buy => reference.0.saturating_sub(arrival.0),
        OrderSide::Sell => arrival.0.saturating_sub(reference.0),
    };
    if adverse_ticks <= 0 || arrival.0 <= 0 {
        return 0;
    }
    let bps = (i128::from(adverse_ticks) * 10_000) / i128::from(arrival.0);
    u16::try_from(bps.min(i128::from(u16::MAX))).unwrap_or(u16::MAX)
}

fn fixed_id_with_index<const N: usize>(
    prefix: &str,
    index: u64,
) -> Result<FixedAscii<N>, AlgoError> {
    let mut bytes = [0_u8; N];
    let prefix_bytes = prefix.as_bytes();
    if prefix_bytes.len().saturating_add(1) > N {
        return Err(AlgoError::GeneratedIdentifierTooLong);
    }
    bytes[..prefix_bytes.len()].copy_from_slice(prefix_bytes);
    let mut len = prefix_bytes.len();
    bytes[len] = b'-';
    len += 1;

    let mut digits = [0_u8; 20];
    let mut value = index;
    let mut digit_len = 0_usize;
    loop {
        digits[digit_len] = b'0' + u8::try_from(value % 10).unwrap_or(0);
        digit_len += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    if len.saturating_add(digit_len) > N {
        return Err(AlgoError::GeneratedIdentifierTooLong);
    }
    for digit in digits[..digit_len].iter().rev() {
        bytes[len] = *digit;
        len += 1;
    }
    let value =
        std::str::from_utf8(&bytes[..len]).expect("prefix is ASCII and generated digits are ASCII");
    Ok(FixedAscii::new(value)?)
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn hash_u64(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn hash_i64(hash: u64, value: i64) -> u64 {
    hash_u64(hash, value as u64)
}

fn hash_replay_step<const N: usize>(
    mut hash: u64,
    input_sequence: u64,
    event: AlgoReplayEvent,
    progress: AlgoProgress,
    decision: &AlgoDecision<N>,
) -> u64 {
    hash = hash_u64(hash, input_sequence);
    hash = match event {
        AlgoReplayEvent::Timer { timestamp_ns } => hash_u64(hash_u64(hash, 1), timestamp_ns),
        AlgoReplayEvent::Execution(event) => hash_i64(
            hash_u64(hash_u64(hash, 2), u64::from(event.order_status as u8)),
            event.last_qty.0,
        ),
        AlgoReplayEvent::ParentStatus { status } => hash_u64(hash_u64(hash, 3), status as u64),
    };
    hash = hash_i64(hash, progress.released_qty().0);
    hash = hash_i64(hash, progress.completed_qty().0);
    hash = hash_i64(hash, progress.open_qty().0);
    hash_u64(hash, usize_to_u64(decision.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use of_execution_core::{ExecutionEvent, ExecutionId, RiskRejectReason, VenueOrderId};

    fn parent() -> ParentOrder {
        ParentOrder::new(
            ParentOrderId::new("parent-1").expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("sim").expect("route"),
            StrategyId::new("twap").expect("strategy"),
            ExecutionSymbol::new("SIM", "ESZ6").expect("symbol"),
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::Day,
            OrderQty(100),
            OrderPrice(500_000),
            OrderPrice(0),
            1_000,
            11_000,
            OrderQty(10),
            OrderQty(25),
            0,
        )
        .expect("parent")
    }

    fn sell_parent(
        id: &str,
        total_qty: OrderQty,
        min_clip: OrderQty,
        max_clip: OrderQty,
    ) -> ParentOrder {
        ParentOrder::new(
            ParentOrderId::new(id).expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("sim").expect("route"),
            StrategyId::new("spread").expect("strategy"),
            ExecutionSymbol::new("SIM", "NQZ6").expect("symbol"),
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::Day,
            total_qty,
            OrderPrice(1_000_000),
            OrderPrice(0),
            1_000,
            11_000,
            min_clip,
            max_clip,
            0,
        )
        .expect("sell parent")
    }

    #[test]
    fn twap_plans_due_slices_without_over_release() {
        let parent = parent();
        let planner = TwapSlicePlanner::new(1_000);
        let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let first = planner
            .plan_due_slice(
                &parent,
                progress,
                1_000,
                ChildOrderId::new("child-1").expect("child"),
                ClientOrderId::new("cl-1").expect("client"),
                1_000,
            )
            .expect("plan")
            .expect("due");
        assert_eq!(first.request().quantity, OrderQty(10));
        progress.on_child_released(&first).expect("release");

        let second_same_bucket = planner
            .plan_due_slice(
                &parent,
                progress,
                1_500,
                ChildOrderId::new("child-2").expect("child"),
                ClientOrderId::new("cl-2").expect("client"),
                1_500,
            )
            .expect("plan");
        assert!(second_same_bucket.is_none());

        let later = planner
            .plan_due_slice(
                &parent,
                progress,
                3_000,
                ChildOrderId::new("child-3").expect("child"),
                ClientOrderId::new("cl-3").expect("client"),
                3_000,
            )
            .expect("plan")
            .expect("due");
        assert_eq!(later.request().quantity, OrderQty(20));
    }

    #[test]
    fn twap_respects_max_clip_and_final_leaves() {
        let parent = parent();
        let planner = TwapSlicePlanner::new(1_000);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let late = planner
            .plan_due_slice(
                &parent,
                progress,
                10_500,
                ChildOrderId::new("child-late").expect("child"),
                ClientOrderId::new("cl-late").expect("client"),
                10_500,
            )
            .expect("plan")
            .expect("due");
        assert_eq!(late.request().quantity, parent.max_clip());
    }

    #[test]
    fn decision_buffer_is_bounded() {
        let mut decision = AlgoDecision::<1>::new(7);
        let plan = ChildOrderPlan::new(
            ChildOrderId::new("child-1").expect("child"),
            parent().id(),
            parent().build_order_request(
                ClientOrderId::new("cl-1").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("plan");

        decision
            .push(AlgoAction::SubmitChild(plan))
            .expect("first action");
        assert_eq!(decision.len(), 1);
        assert_eq!(
            decision.push(AlgoAction::CompleteParent {
                parent_id: parent().id()
            }),
            Err(AlgoError::DecisionFull { capacity: 1 })
        );
        assert_eq!(decision.actions().count(), 1);
    }

    #[test]
    fn risk_policy_allows_child_inside_limits() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("risk-ok").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("risk-ok-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let limits = AlgoRiskLimits::new(
            OrderQty(200),
            OrderQty(25),
            10_000_000,
            1_500,
            100,
            OrderQty(50),
            2,
            10,
        )
        .expect("limits");
        let context = AlgoRiskContext::new(OrderPrice(500_000))
            .expect("context")
            .with_observed_market_volume(OrderQty(1_000));

        let report = AlgoRiskPolicy::new(limits)
            .evaluate_child::<DEFAULT_ALGO_RISK_VIOLATION_CAPACITY>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                &child,
                context,
            )
            .expect("report");

        assert!(report.is_allowed());
        assert!(report.is_empty());
    }

    #[test]
    fn risk_policy_blocks_limit_breaches() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("risk-block").expect("child"),
            parent.id(),
            parent.build_order_request_at_price(
                ClientOrderId::new("risk-block-cl").expect("client"),
                OrderQty(25),
                OrderPrice(510_000),
                1,
            ),
            1,
        )
        .expect("child");
        let limits = AlgoRiskLimits::new(
            OrderQty(100),
            OrderQty(10),
            5_000_000,
            1_000,
            100,
            OrderQty(50),
            2,
            10,
        )
        .expect("limits");
        let context = AlgoRiskContext::new(OrderPrice(500_000))
            .expect("context")
            .with_observed_market_volume(OrderQty(100));

        let report = AlgoRiskPolicy::new(limits)
            .evaluate_child::<DEFAULT_ALGO_RISK_VIOLATION_CAPACITY>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                &child,
                context,
            )
            .expect("report");

        assert_eq!(report.outcome(), AlgoRiskOutcome::Block);
        assert!(report
            .violations()
            .any(|violation| violation.kind() == AlgoRiskViolationKind::ChildQuantityExceeded));
        assert!(report
            .violations()
            .any(|violation| violation.kind() == AlgoRiskViolationKind::PriceCollarExceeded));
        assert!(report
            .violations()
            .any(|violation| violation.kind() == AlgoRiskViolationKind::ParticipationExceeded));
    }

    #[test]
    fn risk_policy_kill_switch_halts_submission() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("risk-kill").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("risk-kill-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let context = AlgoRiskContext::new(OrderPrice(500_000))
            .expect("context")
            .with_kill_switch_active(true);

        let report = AlgoRiskPolicy::default()
            .evaluate_child::<DEFAULT_ALGO_RISK_VIOLATION_CAPACITY>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                &child,
                context,
            )
            .expect("report");

        assert_eq!(report.outcome(), AlgoRiskOutcome::KillSwitch);
        assert_eq!(
            report.first_violation().expect("violation").kind(),
            AlgoRiskViolationKind::KillSwitchActive
        );
    }

    #[test]
    fn risk_policy_checks_decision_level_limits() {
        let parent = parent();
        let mut decision = AlgoDecision::<2>::new(1);
        for index in 1..=2 {
            let suffix = index.to_string();
            let child = ChildOrderPlan::new(
                ChildOrderId::new(&format!("risk-d-{suffix}")).expect("child"),
                parent.id(),
                parent.build_order_request(
                    ClientOrderId::new(&format!("risk-d-cl-{suffix}")).expect("client"),
                    OrderQty(10),
                    index,
                ),
                index,
            )
            .expect("child");
            decision.push(AlgoAction::SubmitChild(child)).expect("push");
        }
        let limits = AlgoRiskLimits::new(OrderQty(0), OrderQty(0), 0, 0, 0, OrderQty(15), 1, 1)
            .expect("limits");
        let context = AlgoRiskContext::new(OrderPrice(500_000))
            .expect("context")
            .with_child_orders_in_window(0);

        let report = AlgoRiskPolicy::new(limits)
            .evaluate_decision::<DEFAULT_ALGO_RISK_VIOLATION_CAPACITY, 2>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                &decision,
                context,
            )
            .expect("report");

        assert_eq!(report.outcome(), AlgoRiskOutcome::Block);
        assert!(report.violations().any(
            |violation| violation.kind() == AlgoRiskViolationKind::ChildrenPerDecisionExceeded
        ));
        assert!(report
            .violations()
            .any(|violation| violation.kind() == AlgoRiskViolationKind::OpenQuantityExceeded));
    }

    #[test]
    fn algo_checkpoint_records_replay_cursors() {
        let parent = parent();
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let checkpoint = AlgoCheckpoint::new(parent, progress, 7, 42).expect("checkpoint");

        assert_eq!(checkpoint.schema_version(), ALGO_CHECKPOINT_SCHEMA_VERSION);
        assert_eq!(checkpoint.parent(), parent);
        assert_eq!(checkpoint.progress(), progress);
        assert_eq!(checkpoint.next_decision_seq(), 7);
        assert_eq!(checkpoint.last_input_sequence(), 42);
        assert_eq!(
            AlgoCheckpoint::new(parent, progress, 0, 42),
            Err(AlgoError::InvalidRecoveryState)
        );
    }

    #[test]
    fn recovery_plan_pauses_by_default() {
        let parent = parent();
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let checkpoint = AlgoCheckpoint::new(parent, progress, 7, 42).expect("checkpoint");
        let plan = AlgoRecoveryPlan::new(checkpoint, AlgoRecoveryPolicy::default()).expect("plan");

        assert_eq!(plan.action(), AlgoRecoveryAction::Pause);
        assert_eq!(plan.replay_from_sequence(), 43);
        assert_eq!(plan.next_decision_seq(), 7);
        assert!(plan.reconciliation_required());
    }

    #[test]
    fn recovery_plan_can_resume_when_policy_allows() {
        let parent = parent();
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let checkpoint = AlgoCheckpoint::new(parent, progress, 3, 9).expect("checkpoint");
        let policy = AlgoRecoveryPolicy::default()
            .with_pause_on_recovery(false)
            .with_require_reconciliation(false);
        let plan = AlgoRecoveryPlan::new(checkpoint, policy).expect("plan");

        assert_eq!(plan.action(), AlgoRecoveryAction::Resume);
        assert!(!plan.reconciliation_required());
    }

    #[test]
    fn recovery_plan_completes_finished_progress() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("rec-child").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("rec-cl").expect("client"),
                parent.total_qty(),
                1,
            ),
            1,
        )
        .expect("child");
        let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());
        progress.on_child_released(&child).expect("release");
        progress.on_execution_event(&ExecutionEvent {
            exec_type: of_execution_core::ExecutionType::Trade,
            order_status: OrderStatus::Filled,
            client_order_id: child.request().client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: VenueOrderId::new("rec-venue").expect("venue"),
            execution_id: ExecutionId::new("rec-exec").expect("exec"),
            account_id: parent.account_id(),
            route_id: parent.route_id(),
            symbol: parent.symbol(),
            last_qty: parent.total_qty(),
            last_price: parent.limit_price(),
            cumulative_qty: parent.total_qty(),
            leaves_qty: OrderQty(0),
            average_price: parent.limit_price(),
            ts_exchange_ns: 2,
            ts_recv_ns: 3,
            reason: RiskRejectReason::None,
            text: of_execution_core::ExecutionText::empty(),
        });
        let checkpoint = AlgoCheckpoint::new(parent, progress, 10, 99).expect("checkpoint");
        let plan = AlgoRecoveryPlan::new(checkpoint, AlgoRecoveryPolicy::default()).expect("plan");

        assert_eq!(plan.action(), AlgoRecoveryAction::CompleteParent);
    }

    #[test]
    fn recovery_plan_escalates_terminal_parent() {
        let parent = parent().with_status(ParentOrderStatus::Failed);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let checkpoint = AlgoCheckpoint::new(parent, progress, 10, 99).expect("checkpoint");
        let policy = AlgoRecoveryPolicy::default()
            .with_pause_on_recovery(false)
            .with_require_reconciliation(false);
        let plan = AlgoRecoveryPlan::new(checkpoint, policy).expect("plan");

        assert_eq!(plan.action(), AlgoRecoveryAction::EscalateRisk);
    }

    #[test]
    fn simulator_fills_child_and_updates_progress() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("sim-fill").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("sim-fill-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let simulator = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(10), OrderPrice(500_025), false, false, 25)
                .expect("market"),
        );
        let step = simulator.simulate_child(&child, 1).expect("step");
        let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());
        progress.on_child_released(&child).expect("release");
        progress.on_execution_event(&step.event());

        assert_eq!(step.outcome(), AlgoSimOutcome::Filled);
        assert_eq!(step.filled_qty(), OrderQty(10));
        assert_eq!(step.event().order_status, OrderStatus::Filled);
        assert_eq!(step.event().ts_recv_ns, 26);
        assert_eq!(progress.completed_qty(), OrderQty(10));
        assert_eq!(progress.open_qty(), OrderQty(0));
    }

    #[test]
    fn simulator_partially_fills_child() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("sim-partial").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("sim-partial-cl").expect("client"),
                OrderQty(25),
                1,
            ),
            1,
        )
        .expect("child");
        let simulator = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(5), OrderPrice(500_000), false, false, 0).expect("market"),
        );
        let step = simulator.simulate_child(&child, 7).expect("step");

        assert_eq!(step.outcome(), AlgoSimOutcome::PartiallyFilled);
        assert_eq!(step.filled_qty(), OrderQty(5));
        assert_eq!(step.leaves_qty(), OrderQty(20));
        assert_eq!(step.event().order_status, OrderStatus::PartiallyFilled);
    }

    #[test]
    fn simulator_rejects_child() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("sim-reject").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("sim-reject-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let simulator = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(0), OrderPrice(0), true, false, 0).expect("market"),
        );
        let step = simulator.simulate_child(&child, 2).expect("step");

        assert_eq!(step.outcome(), AlgoSimOutcome::Rejected);
        assert_eq!(step.event().exec_type, ExecutionType::Reject);
        assert_eq!(step.event().order_status, OrderStatus::Rejected);
        assert_eq!(step.event().leaves_qty, OrderQty(10));
    }

    #[test]
    fn simulator_reports_decision_totals() {
        let parent = parent();
        let mut decision = AlgoDecision::<2>::new(1);
        for index in 1..=2 {
            let child = ChildOrderPlan::new(
                ChildOrderId::new(&format!("sim-d-{index}")).expect("child"),
                parent.id(),
                parent.build_order_request(
                    ClientOrderId::new(&format!("sim-d-cl-{index}")).expect("client"),
                    OrderQty(10),
                    index,
                ),
                index,
            )
            .expect("child");
            decision.push(AlgoAction::SubmitChild(child)).expect("push");
        }
        let simulator = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(10), OrderPrice(500_000), false, true, 0).expect("market"),
        );
        let report = simulator
            .simulate_decision::<1, 2>(&decision, 10)
            .expect("report");

        assert!(report.truncated());
        assert_eq!(report.len(), 1);
        assert_eq!(report.total_filled_qty(), OrderQty(20));
        assert_eq!(report.cancelled_children(), 0);
    }

    #[test]
    fn metrics_accumulate_fill_quality() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("met-fill").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("met-fill-cl").expect("client"),
                OrderQty(10),
                1_000,
            ),
            1_000,
        )
        .expect("child");
        let mut metrics = AlgoMetricsAccumulator::new(
            &parent,
            AlgoTcaBenchmark::new(
                OrderPrice(500_000),
                OrderPrice(502_500),
                OrderPrice(501_000),
            )
            .expect("benchmark"),
        )
        .expect("metrics");
        metrics.on_child_submitted(&child);
        let step = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(10), OrderPrice(505_000), false, false, 25)
                .expect("market"),
        )
        .simulate_child(&child, 1)
        .expect("step");
        metrics.on_execution_event(&step.event());
        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.submitted_children(), 1);
        assert_eq!(snapshot.filled_children(), 1);
        assert_eq!(snapshot.completed_qty(), OrderQty(10));
        assert_eq!(snapshot.completion_bps(), 1_000);
        assert_eq!(snapshot.average_price(), OrderPrice(505_000));
        assert_eq!(snapshot.arrival_slippage_bps(), 100);
        assert_eq!(snapshot.average_latency_ns(), 25);
    }

    #[test]
    fn metrics_record_rejects_and_cancels() {
        let parent = parent();
        let rejected_child = ChildOrderPlan::new(
            ChildOrderId::new("met-reject").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("met-reject-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let cancelled_child = ChildOrderPlan::new(
            ChildOrderId::new("met-cancel").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("met-cancel-cl").expect("client"),
                OrderQty(10),
                2,
            ),
            2,
        )
        .expect("child");
        let mut metrics = AlgoMetricsAccumulator::new(
            &parent,
            AlgoTcaBenchmark::new(OrderPrice(500_000), OrderPrice(0), OrderPrice(0))
                .expect("benchmark"),
        )
        .expect("metrics");
        metrics.on_child_submitted(&rejected_child);
        metrics.on_child_submitted(&cancelled_child);
        let reject = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(0), OrderPrice(0), true, false, 0).expect("market"),
        )
        .simulate_child(&rejected_child, 1)
        .expect("reject");
        let cancel = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(0), OrderPrice(500_000), false, true, 0).expect("market"),
        )
        .simulate_child(&cancelled_child, 2)
        .expect("cancel");
        metrics.on_execution_event(&reject.event());
        metrics.on_execution_event(&cancel.event());
        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.submitted_children(), 2);
        assert_eq!(snapshot.rejected_children(), 1);
        assert_eq!(snapshot.cancelled_children(), 1);
        assert_eq!(snapshot.completed_qty(), OrderQty(0));
    }

    #[test]
    fn metrics_preserve_favorable_sell_slippage() {
        let parent = sell_parent("met-sell", OrderQty(100), OrderQty(10), OrderQty(25));
        let child = ChildOrderPlan::new(
            ChildOrderId::new("met-sell-child").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("met-sell-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let mut metrics = AlgoMetricsAccumulator::new(
            &parent,
            AlgoTcaBenchmark::new(OrderPrice(500_000), OrderPrice(0), OrderPrice(0))
                .expect("benchmark"),
        )
        .expect("metrics");
        metrics.on_child_submitted(&child);
        let step = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(10), OrderPrice(505_000), false, false, 0).expect("market"),
        )
        .simulate_child(&child, 1)
        .expect("step");
        metrics.on_execution_event(&step.event());

        assert_eq!(metrics.snapshot().arrival_slippage_bps(), -100);
    }

    #[test]
    fn progress_folds_fill_events() {
        let parent = parent();
        let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let plan = ChildOrderPlan::new(
            ChildOrderId::new("child-1").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("cl-1").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("plan");
        progress.on_child_released(&plan).expect("release");

        progress.on_execution_event(&ExecutionEvent {
            exec_type: of_execution_core::ExecutionType::Trade,
            order_status: OrderStatus::Filled,
            client_order_id: plan.request().client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: VenueOrderId::new("venue-1").expect("venue"),
            execution_id: ExecutionId::new("exec-1").expect("exec"),
            account_id: parent.account_id(),
            route_id: parent.route_id(),
            symbol: parent.symbol(),
            last_qty: OrderQty(10),
            last_price: OrderPrice(500_000),
            cumulative_qty: OrderQty(10),
            leaves_qty: OrderQty(0),
            average_price: OrderPrice(500_000),
            ts_exchange_ns: 2,
            ts_recv_ns: 3,
            reason: RiskRejectReason::None,
            text: of_execution_core::ExecutionText::empty(),
        });

        assert_eq!(progress.completed_qty(), OrderQty(10));
        assert_eq!(progress.open_qty(), OrderQty(0));
        assert_eq!(progress.terminal_children(), 1);
    }

    #[test]
    fn invalid_parent_schedule_is_rejected() {
        assert_eq!(
            ParentOrder::new(
                ParentOrderId::new("parent-1").expect("id"),
                AccountId::new("acct").expect("account"),
                RouteId::new("sim").expect("route"),
                StrategyId::new("twap").expect("strategy"),
                ExecutionSymbol::new("SIM", "ESZ6").expect("symbol"),
                OrderSide::Buy,
                OrderType::Limit,
                TimeInForce::Day,
                OrderQty(100),
                OrderPrice(500_000),
                OrderPrice(0),
                10,
                10,
                OrderQty(10),
                OrderQty(25),
                0,
            ),
            Err(AlgoError::InvalidTimeWindow)
        );
    }

    #[test]
    fn replay_twap_is_deterministic() {
        let parent = parent();
        let planner = TwapSlicePlanner::new(1_000);
        let inputs = [
            AlgoReplayInput::new(
                1,
                AlgoReplayEvent::Timer {
                    timestamp_ns: 1_000,
                },
            ),
            AlgoReplayInput::new(
                2,
                AlgoReplayEvent::Timer {
                    timestamp_ns: 2_000,
                },
            ),
            AlgoReplayInput::new(
                3,
                AlgoReplayEvent::Timer {
                    timestamp_ns: 3_000,
                },
            ),
        ];
        let mut first_steps = Vec::new();
        let mut second_steps = Vec::new();
        let first = replay_twap_into::<DEFAULT_ALGO_DECISION_CAPACITY>(
            parent,
            planner,
            &inputs,
            AlgoReplayIdScheme::default(),
            &mut first_steps,
        )
        .expect("first replay");
        let second = replay_twap_into::<DEFAULT_ALGO_DECISION_CAPACITY>(
            parent,
            planner,
            &inputs,
            AlgoReplayIdScheme::default(),
            &mut second_steps,
        )
        .expect("second replay");

        assert_eq!(first, second);
        assert_eq!(first_steps, second_steps);
        assert_eq!(first.input_events(), 3);
        assert_eq!(first.submitted_children(), 3);
        assert_eq!(first.final_progress().released_qty(), OrderQty(30));
    }

    #[test]
    fn replay_folds_execution_events() {
        let parent = parent();
        let planner = TwapSlicePlanner::new(1_000);
        let fill = ExecutionEvent {
            exec_type: of_execution_core::ExecutionType::Trade,
            order_status: OrderStatus::Filled,
            client_order_id: ClientOrderId::new("cl-1").expect("client"),
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: VenueOrderId::new("venue-1").expect("venue"),
            execution_id: ExecutionId::new("exec-1").expect("exec"),
            account_id: parent.account_id(),
            route_id: parent.route_id(),
            symbol: parent.symbol(),
            last_qty: OrderQty(10),
            last_price: OrderPrice(500_000),
            cumulative_qty: OrderQty(10),
            leaves_qty: OrderQty(0),
            average_price: OrderPrice(500_000),
            ts_exchange_ns: 2,
            ts_recv_ns: 3,
            reason: RiskRejectReason::None,
            text: of_execution_core::ExecutionText::empty(),
        };
        let inputs = [
            AlgoReplayInput::new(
                1,
                AlgoReplayEvent::Timer {
                    timestamp_ns: 1_000,
                },
            ),
            AlgoReplayInput::new(2, AlgoReplayEvent::Execution(fill)),
        ];
        let mut steps = Vec::new();
        let summary = replay_twap_into::<DEFAULT_ALGO_DECISION_CAPACITY>(
            parent,
            planner,
            &inputs,
            AlgoReplayIdScheme::default(),
            &mut steps,
        )
        .expect("replay");

        assert_eq!(summary.final_progress().released_qty(), OrderQty(10));
        assert_eq!(summary.final_progress().completed_qty(), OrderQty(10));
        assert_eq!(summary.final_progress().open_qty(), OrderQty(0));
        assert_eq!(steps[0].decision().len(), 1);
        assert!(steps[1].decision().is_empty());
    }

    #[test]
    fn replay_reports_decision_capacity_errors() {
        let parent = parent();
        let inputs = [AlgoReplayInput::new(
            1,
            AlgoReplayEvent::Timer {
                timestamp_ns: 1_000,
            },
        )];
        let mut steps = Vec::new();
        assert_eq!(
            replay_twap_into::<0>(
                parent,
                TwapSlicePlanner::new(1_000),
                &inputs,
                AlgoReplayIdScheme::default(),
                &mut steps,
            ),
            Err(AlgoError::DecisionFull { capacity: 0 })
        );
        assert!(steps.is_empty());
    }

    #[test]
    fn pov_plans_from_observed_volume() {
        let parent = parent();
        let planner = PovSlicePlanner::new(1_000, 1_500);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_volume_slice(
                &parent,
                progress,
                OrderQty(1_000),
                2_000,
                ChildOrderId::new("child-pov").expect("child"),
                ClientOrderId::new("cl-pov").expect("client"),
                2_000,
            )
            .expect("plan")
            .expect("due");

        assert_eq!(child.request().quantity, parent.max_clip());
    }

    #[test]
    fn pov_waits_when_due_quantity_is_below_min_clip() {
        let parent = parent();
        let planner = PovSlicePlanner::new(1_000, 1_500);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_volume_slice(
                &parent,
                progress,
                OrderQty(50),
                2_000,
                ChildOrderId::new("child-small").expect("child"),
                ClientOrderId::new("cl-small").expect("client"),
                2_000,
            )
            .expect("plan");

        assert!(child.is_none());
    }

    #[test]
    fn pov_rejects_parent_cap_below_target() {
        let capped_parent = parent().with_status(ParentOrderStatus::Active);
        let capped_parent = ParentOrder::new(
            capped_parent.id(),
            capped_parent.account_id(),
            capped_parent.route_id(),
            capped_parent.strategy_id(),
            capped_parent.symbol(),
            capped_parent.side(),
            capped_parent.order_type(),
            capped_parent.time_in_force(),
            capped_parent.total_qty(),
            capped_parent.limit_price(),
            capped_parent.stop_price(),
            capped_parent.start_ns(),
            capped_parent.end_ns(),
            capped_parent.min_clip(),
            capped_parent.max_clip(),
            500,
        )
        .expect("capped parent");
        let planner = PovSlicePlanner::new(1_000, 1_500);
        let progress = AlgoProgress::new(capped_parent.id(), capped_parent.total_qty());

        assert_eq!(
            planner.plan_volume_slice(
                &capped_parent,
                progress,
                OrderQty(1_000),
                2_000,
                ChildOrderId::new("child-cap").expect("child"),
                ClientOrderId::new("cl-cap").expect("client"),
                2_000,
            ),
            Err(AlgoError::InvalidParticipationRate)
        );
    }

    #[test]
    fn vwap_plans_from_cumulative_curve() {
        let parent = parent();
        let curve = VwapVolumeCurve::new(1_000, 1_000, &[10, 30, 60, 100]).expect("curve");
        let planner = VwapSlicePlanner::new(curve);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_curve_slice(
                &parent,
                progress,
                2_000,
                ChildOrderId::new("child-vwap").expect("child"),
                ClientOrderId::new("cl-vwap").expect("client"),
                2_000,
            )
            .expect("plan")
            .expect("due");

        assert_eq!(child.request().quantity, OrderQty(25));
    }

    #[test]
    fn vwap_waits_for_min_clip() {
        let parent = parent();
        let curve = VwapVolumeCurve::new(1_000, 1_000, &[1, 2, 100]).expect("curve");
        let planner = VwapSlicePlanner::new(curve);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_curve_slice(
                &parent,
                progress,
                1_000,
                ChildOrderId::new("child-small").expect("child"),
                ClientOrderId::new("cl-small").expect("client"),
                1_000,
            )
            .expect("plan");

        assert!(child.is_none());
    }

    #[test]
    fn vwap_rejects_invalid_curve() {
        assert_eq!(
            VwapVolumeCurve::new(1, 1, &[10, 9]),
            Err(AlgoError::InvalidVolumeProfile)
        );
        assert_eq!(
            VwapVolumeCurve::new(1, 0, &[10]),
            Err(AlgoError::InvalidVolumeProfile)
        );
    }

    #[test]
    fn iceberg_replenishes_when_open_quantity_is_at_threshold() {
        let parent = parent();
        let planner = IcebergSlicePlanner::new(OrderQty(20), OrderQty(0));
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_replenishment(
                &parent,
                progress,
                1_000,
                ChildOrderId::new("child-ice").expect("child"),
                ClientOrderId::new("cl-ice").expect("client"),
                1_000,
            )
            .expect("plan")
            .expect("due");

        assert_eq!(child.request().quantity, OrderQty(20));
    }

    #[test]
    fn iceberg_waits_while_display_quantity_is_working() {
        let parent = parent();
        let planner = IcebergSlicePlanner::new(OrderQty(20), OrderQty(0));
        let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let working = ChildOrderPlan::new(
            ChildOrderId::new("child-ice").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("cl-ice").expect("client"),
                OrderQty(20),
                1,
            ),
            1,
        )
        .expect("child");
        progress.on_child_released(&working).expect("release");

        let child = planner
            .plan_replenishment(
                &parent,
                progress,
                2_000,
                ChildOrderId::new("child-next").expect("child"),
                ClientOrderId::new("cl-next").expect("client"),
                2_000,
            )
            .expect("plan");

        assert!(child.is_none());
    }

    #[test]
    fn iceberg_rejects_invalid_display_settings() {
        assert_eq!(
            IcebergSlicePlanner::try_new(OrderQty(0), OrderQty(0)),
            Err(AlgoError::InvalidDisplayQuantity)
        );
        assert_eq!(
            IcebergSlicePlanner::try_new(OrderQty(10), OrderQty(11)),
            Err(AlgoError::InvalidDisplayQuantity)
        );
    }

    #[test]
    fn passive_queue_joins_when_fill_probability_is_good() {
        let parent = parent();
        let planner = PassiveQueuePlanner::new(
            PassiveQueueConfig::new(PassivePegMode::SameSide, OrderPrice(25)).expect("config"),
        );
        let context = PassiveQueueContext::new(
            OrderPrice(499_975),
            OrderPrice(500_025),
            OrderQty(25),
            OrderQty(100),
            10,
        )
        .expect("context");
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let decision = planner
            .plan_passive_slice(
                &parent,
                progress,
                2_000,
                context,
                ChildOrderId::new("child-pq").expect("child"),
                ClientOrderId::new("cl-pq").expect("client"),
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.action(), PassiveQueueAction::JoinQueue);
        let child = decision.child().expect("child");
        assert_eq!(child.request().limit_price, context.best_bid());
        assert_eq!(child.request().quantity, parent.max_clip());
        assert!(decision.estimate().fill_probability_bps() >= 2_500);
    }

    #[test]
    fn passive_queue_improves_when_queue_is_unlikely_to_fill() {
        let parent = parent();
        let config = PassiveQueueConfig::new(PassivePegMode::SameSide, OrderPrice(25))
            .expect("config")
            .with_thresholds(2_500, 250, 1_500)
            .expect("thresholds")
            .with_max_improvement_ticks(1);
        let planner = PassiveQueuePlanner::new(config);
        let context = PassiveQueueContext::new(
            OrderPrice(499_950),
            OrderPrice(500_050),
            OrderQty(10_000),
            OrderQty(10),
            10,
        )
        .expect("context");
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let decision = planner
            .plan_passive_slice(
                &parent,
                progress,
                2_000,
                context,
                ChildOrderId::new("child-imp").expect("child"),
                ClientOrderId::new("cl-imp").expect("client"),
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.action(), PassiveQueueAction::ImprovePrice);
        let child = decision.child().expect("child");
        assert_eq!(child.request().limit_price, OrderPrice(499_975));
        assert!(child.request().limit_price.0 < context.best_ask().0);
    }

    #[test]
    fn passive_queue_waits_when_adverse_selection_is_high() {
        let parent = parent();
        let planner = PassiveQueuePlanner::new(
            PassiveQueueConfig::new(PassivePegMode::SameSide, OrderPrice(25)).expect("config"),
        );
        let context = PassiveQueueContext::new(
            OrderPrice(499_975),
            OrderPrice(500_025),
            OrderQty(0),
            OrderQty(100),
            1_000,
        )
        .expect("context");

        let decision = planner
            .plan_passive_slice(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                context,
                ChildOrderId::new("child-wait").expect("child"),
                ClientOrderId::new("cl-wait").expect("client"),
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.action(), PassiveQueueAction::Wait);
        assert!(decision.child().is_none());
    }

    #[test]
    fn passive_queue_can_cross_when_allowed_and_late() {
        let parent = parent();
        let config = PassiveQueueConfig::new(PassivePegMode::SameSide, OrderPrice(25))
            .expect("config")
            .with_crossing(true, 8_000)
            .expect("crossing");
        let planner = PassiveQueuePlanner::new(config);
        let context = PassiveQueueContext::new(
            OrderPrice(499_975),
            OrderPrice(500_025),
            OrderQty(10_000),
            OrderQty(0),
            10,
        )
        .expect("context");

        let decision = planner
            .plan_passive_slice(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                10_000,
                context,
                ChildOrderId::new("child-cross").expect("child"),
                ClientOrderId::new("cl-cross").expect("client"),
                10_000,
            )
            .expect("decision");

        assert_eq!(decision.action(), PassiveQueueAction::CrossSpread);
        assert_eq!(
            decision.child().expect("child").request().limit_price,
            context.best_ask()
        );
    }

    #[test]
    fn passive_queue_rejects_invalid_context() {
        assert_eq!(
            PassiveQueueConfig::new(PassivePegMode::SameSide, OrderPrice(0)),
            Err(AlgoError::InvalidPassiveQueueParameters)
        );
        assert_eq!(
            PassiveQueueContext::new(OrderPrice(10), OrderPrice(10), OrderQty(0), OrderQty(0), 0),
            Err(AlgoError::InvalidPassiveQueueParameters)
        );
    }

    #[test]
    fn sor_prefers_better_scored_route() {
        let parent = parent();
        let planner = SorPlanner::new(SorConfig::default());
        let candidates = [
            SorRouteCandidate::new(
                RouteId::new("slow").expect("route"),
                OrderPrice(500_050),
                OrderQty(25),
            )
            .expect("candidate")
            .with_metrics(SorRouteMetrics::new(0, 2_000, 200, 4_000, 100, 9_000).expect("metrics")),
            SorRouteCandidate::new(
                RouteId::new("fast").expect("route"),
                OrderPrice(499_975),
                OrderQty(25),
            )
            .expect("candidate")
            .with_metrics(SorRouteMetrics::new(0, 100, 0, 9_000, 10, 10_000).expect("metrics")),
        ];
        let child_ids = [
            ChildOrderId::new("child-sor-1").expect("child"),
            ChildOrderId::new("child-sor-2").expect("child"),
        ];
        let client_ids = [
            ClientOrderId::new("cl-sor-1").expect("client"),
            ClientOrderId::new("cl-sor-2").expect("client"),
        ];

        let decision = planner
            .plan_routes::<2>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        let first = decision.allocations().next().expect("allocation");
        assert_eq!(
            first.plan().request().route_id,
            RouteId::new("fast").expect("route")
        );
        assert_eq!(first.plan().request().limit_price, OrderPrice(499_975));
    }

    #[test]
    fn sor_splits_across_routes_until_max_clip_is_reached() {
        let parent = parent();
        let planner = SorPlanner::new(SorConfig::new(3, SorScoreWeights::default()).expect("cfg"));
        let candidates = [
            SorRouteCandidate::new(
                RouteId::new("r1").expect("route"),
                OrderPrice(499_975),
                OrderQty(10),
            )
            .expect("candidate")
            .with_metrics(SorRouteMetrics::new(0, 0, 0, 10_000, 0, 10_000).expect("metrics")),
            SorRouteCandidate::new(
                RouteId::new("r2").expect("route"),
                OrderPrice(499_980),
                OrderQty(30),
            )
            .expect("candidate")
            .with_metrics(SorRouteMetrics::new(0, 0, 0, 1_000, 0, 10_000).expect("metrics")),
        ];
        let child_ids = [
            ChildOrderId::new("child-r1").expect("child"),
            ChildOrderId::new("child-r2").expect("child"),
            ChildOrderId::new("child-r3").expect("child"),
        ];
        let client_ids = [
            ClientOrderId::new("cl-r1").expect("client"),
            ClientOrderId::new("cl-r2").expect("client"),
            ClientOrderId::new("cl-r3").expect("client"),
        ];

        let decision = planner
            .plan_routes::<3>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        let qty: i64 = decision
            .allocations()
            .map(|allocation| allocation.plan().request().quantity.0)
            .sum();
        assert_eq!(decision.len(), 2);
        assert_eq!(qty, parent.max_clip().0);
    }

    #[test]
    fn sor_skips_blocked_and_unsupported_routes() {
        let parent = parent();
        let planner = SorPlanner::new(SorConfig::default());
        let candidates = [
            SorRouteCandidate::new(
                RouteId::new("blocked").expect("route"),
                OrderPrice(499_900),
                OrderQty(25),
            )
            .expect("candidate")
            .with_status(SorRouteStatus::Blocked),
            SorRouteCandidate::new(
                RouteId::new("market-only").expect("route"),
                OrderPrice(499_925),
                OrderQty(25),
            )
            .expect("candidate")
            .with_capability(SorRouteCapability::new(false, true)),
            SorRouteCandidate::new(
                RouteId::new("limit-ok").expect("route"),
                OrderPrice(500_000),
                OrderQty(25),
            )
            .expect("candidate"),
        ];
        let child_ids = [ChildOrderId::new("child-ok").expect("child")];
        let client_ids = [ClientOrderId::new("cl-ok").expect("client")];

        let decision = planner
            .plan_routes::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        let allocation = decision.allocations().next().expect("allocation");
        assert_eq!(
            allocation.plan().request().route_id,
            RouteId::new("limit-ok").expect("route")
        );
    }

    #[test]
    fn sor_requires_enough_caller_owned_ids() {
        let parent = parent();
        let planner = SorPlanner::new(SorConfig::default());
        let candidates = [SorRouteCandidate::new(
            RouteId::new("route").expect("route"),
            OrderPrice(500_000),
            OrderQty(25),
        )
        .expect("candidate")];

        assert_eq!(
            planner.plan_routes::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &[],
                &[],
                2_000,
            ),
            Err(AlgoError::InvalidSorParameters)
        );
    }

    #[test]
    fn liquidity_seeker_takes_high_fill_route() {
        let parent = parent();
        let config =
            LiquiditySeekingConfig::new(2, OrderQty(5), 0, 7_500, 1_500, 3, 4).expect("config");
        let planner = LiquiditySeekingPlanner::new(config, SorConfig::default());
        let route = SorRouteCandidate::new(
            RouteId::new("lit").expect("route"),
            OrderPrice(499_975),
            OrderQty(25),
        )
        .expect("route")
        .with_metrics(SorRouteMetrics::new(0, 100, 0, 9_000, 100, 10_000).expect("metrics"));
        let candidates =
            [LiquiditySeekingCandidate::new(route, 500, 25, OrderQty(0)).expect("candidate")];
        let child_ids = [ChildOrderId::new("liq-child").expect("child")];
        let client_ids = [ClientOrderId::new("liq-cl").expect("client")];

        let decision = planner
            .plan_liquidity::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        let allocation = decision.allocations().next().expect("allocation");
        assert_eq!(allocation.action(), LiquiditySeekingAction::Take);
        assert_eq!(allocation.plan().request().quantity, parent.max_clip());
        assert_eq!(allocation.plan().request().route_id, route.route_id());
    }

    #[test]
    fn liquidity_seeker_probes_lower_fill_hidden_route() {
        let parent = parent();
        let config =
            LiquiditySeekingConfig::new(1, OrderQty(10), 0, 7_500, 1_500, 3, 4).expect("config");
        let planner = LiquiditySeekingPlanner::new(config, SorConfig::default());
        let route = SorRouteCandidate::new(
            RouteId::new("dark").expect("route"),
            OrderPrice(500_000),
            OrderQty(100),
        )
        .expect("route")
        .with_metrics(SorRouteMetrics::new(0, 200, 0, 4_000, 100, 9_000).expect("metrics"));
        let candidates =
            [LiquiditySeekingCandidate::new(route, 2_500, 50, OrderQty(10)).expect("candidate")];
        let child_ids = [ChildOrderId::new("probe-child").expect("child")];
        let client_ids = [ClientOrderId::new("probe-cl").expect("client")];

        let decision = planner
            .plan_liquidity::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        let allocation = decision.allocations().next().expect("allocation");
        assert_eq!(allocation.action(), LiquiditySeekingAction::Probe);
        assert_eq!(allocation.plan().request().quantity, OrderQty(10));
    }

    #[test]
    fn liquidity_seeker_skips_toxic_routes() {
        let parent = parent();
        let planner = LiquiditySeekingPlanner::new(
            LiquiditySeekingConfig::new(1, OrderQty(10), 0, 7_500, 500, 3, 4).expect("config"),
            SorConfig::default(),
        );
        let route = SorRouteCandidate::new(
            RouteId::new("toxic").expect("route"),
            OrderPrice(499_975),
            OrderQty(100),
        )
        .expect("route")
        .with_metrics(SorRouteMetrics::new(0, 100, 0, 9_000, 1_000, 10_000).expect("metrics"));
        let candidates =
            [LiquiditySeekingCandidate::new(route, 0, 0, OrderQty(0)).expect("candidate")];
        let child_ids = [ChildOrderId::new("skip-child").expect("child")];
        let client_ids = [ClientOrderId::new("skip-cl").expect("client")];

        let decision = planner
            .plan_liquidity::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        assert!(decision.is_empty());
        assert_eq!(decision.skipped_routes(), 1);
    }

    #[test]
    fn liquidity_seeker_rejects_invalid_inputs() {
        assert_eq!(
            LiquiditySeekingConfig::new(0, OrderQty(1), 0, 0, 0, 0, 0),
            Err(AlgoError::InvalidLiquiditySeekingParameters)
        );
        let route = SorRouteCandidate::new(
            RouteId::new("route").expect("route"),
            OrderPrice(500_000),
            OrderQty(1),
        )
        .expect("route");
        assert_eq!(
            LiquiditySeekingCandidate::new(route, 10_001, 0, OrderQty(0)),
            Err(AlgoError::InvalidLiquiditySeekingParameters)
        );
    }

    #[test]
    fn sweep_walks_buy_levels_until_clip_or_collar() {
        let parent = parent();
        let planner =
            SweepPlanner::new(SweepConfig::new(3, OrderPrice(500_025), OrderQty(0)).expect("cfg"));
        let candidates = [
            SorRouteCandidate::new(
                RouteId::new("r2").expect("route"),
                OrderPrice(500_000),
                OrderQty(10),
            )
            .expect("candidate"),
            SorRouteCandidate::new(
                RouteId::new("r1").expect("route"),
                OrderPrice(499_975),
                OrderQty(10),
            )
            .expect("candidate"),
            SorRouteCandidate::new(
                RouteId::new("r3").expect("route"),
                OrderPrice(500_050),
                OrderQty(10),
            )
            .expect("candidate"),
        ];
        let child_ids = [
            ChildOrderId::new("sw-1").expect("child"),
            ChildOrderId::new("sw-2").expect("child"),
            ChildOrderId::new("sw-3").expect("child"),
        ];
        let client_ids = [
            ClientOrderId::new("sw-cl-1").expect("client"),
            ClientOrderId::new("sw-cl-2").expect("client"),
            ClientOrderId::new("sw-cl-3").expect("client"),
        ];

        let decision = planner
            .plan_sweep::<3>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.len(), 2);
        assert_eq!(decision.total_qty(), OrderQty(20));
        assert!(decision.collar_reached());
        let prices: Vec<i64> = decision
            .allocations()
            .map(|allocation| allocation.plan().request().limit_price.0)
            .collect();
        assert_eq!(prices, vec![499_975, 500_000]);
    }

    #[test]
    fn sweep_respects_sell_side_collar() {
        let sell_parent = ParentOrder::new(
            ParentOrderId::new("parent-sweep-sell").expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("sim").expect("route"),
            StrategyId::new("sweep").expect("strategy"),
            ExecutionSymbol::new("SIM", "ESZ6").expect("symbol"),
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::Day,
            OrderQty(100),
            OrderPrice(500_000),
            OrderPrice(0),
            1_000,
            11_000,
            OrderQty(10),
            OrderQty(25),
            0,
        )
        .expect("parent");
        let planner =
            SweepPlanner::new(SweepConfig::new(2, OrderPrice(499_975), OrderQty(0)).expect("cfg"));
        let candidates = [
            SorRouteCandidate::new(
                RouteId::new("sell-good").expect("route"),
                OrderPrice(500_025),
                OrderQty(25),
            )
            .expect("candidate"),
            SorRouteCandidate::new(
                RouteId::new("sell-bad").expect("route"),
                OrderPrice(499_950),
                OrderQty(25),
            )
            .expect("candidate"),
        ];
        let child_ids = [
            ChildOrderId::new("sell-sw-1").expect("child"),
            ChildOrderId::new("sell-sw-2").expect("child"),
        ];
        let client_ids = [
            ClientOrderId::new("sell-cl-1").expect("client"),
            ClientOrderId::new("sell-cl-2").expect("client"),
        ];

        let decision = planner
            .plan_sweep::<2>(
                &sell_parent,
                AlgoProgress::new(sell_parent.id(), sell_parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.len(), 1);
        assert_eq!(
            decision
                .allocations()
                .next()
                .expect("allocation")
                .plan()
                .request()
                .limit_price,
            OrderPrice(500_025)
        );
        assert!(decision.collar_reached());
    }

    #[test]
    fn sweep_suppresses_decision_below_min_fill() {
        let parent = parent();
        let planner =
            SweepPlanner::new(SweepConfig::new(1, OrderPrice(500_000), OrderQty(20)).expect("cfg"));
        let candidates = [SorRouteCandidate::new(
            RouteId::new("small").expect("route"),
            OrderPrice(499_975),
            OrderQty(10),
        )
        .expect("candidate")];
        let child_ids = [ChildOrderId::new("small-sw").expect("child")];
        let client_ids = [ClientOrderId::new("small-cl").expect("client")];

        let decision = planner
            .plan_sweep::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        assert!(decision.is_empty());
    }

    #[test]
    fn sweep_rejects_invalid_config() {
        assert_eq!(
            SweepConfig::new(0, OrderPrice(1), OrderQty(0)),
            Err(AlgoError::InvalidSweepParameters)
        );
        assert_eq!(
            SweepConfig::new(1, OrderPrice(0), OrderQty(0)),
            Err(AlgoError::InvalidSweepParameters)
        );
    }

    #[test]
    fn basket_plans_synchronized_due_legs() {
        let first = BasketLeg::new(parent(), BasketLegRole::Primary, 10_000).expect("leg");
        let second_parent = ParentOrder::new(
            ParentOrderId::new("parent-2").expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("hedge").expect("route"),
            StrategyId::new("basket").expect("strategy"),
            ExecutionSymbol::new("SIM", "NQZ6").expect("symbol"),
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::Day,
            OrderQty(50),
            OrderPrice(1_800_000),
            OrderPrice(0),
            1_000,
            11_000,
            OrderQty(5),
            OrderQty(20),
            0,
        )
        .expect("parent");
        let second = BasketLeg::new(second_parent, BasketLegRole::Hedge, -5_000).expect("leg");
        let legs = [first, second];
        let progresses = [
            AlgoProgress::new(first.parent().id(), first.parent().total_qty()),
            AlgoProgress::new(second.parent().id(), second.parent().total_qty()),
        ];
        let child_ids = [
            ChildOrderId::new("child-b1").expect("child"),
            ChildOrderId::new("child-b2").expect("child"),
        ];
        let client_ids = [
            ClientOrderId::new("cl-b1").expect("client"),
            ClientOrderId::new("cl-b2").expect("client"),
        ];

        let decision = BasketPlanner::new()
            .plan_synchronized_slice::<2>(&legs, &progresses, 6_000, &child_ids, &client_ids, 6_000)
            .expect("decision");

        assert_eq!(decision.len(), 2);
        let quantities: Vec<i64> = decision
            .allocations()
            .map(|allocation| allocation.plan().request().quantity.0)
            .collect();
        assert_eq!(quantities, vec![25, 20]);
    }

    #[test]
    fn basket_blocks_terminal_legs_without_releasing() {
        let terminal_parent = parent().with_status(ParentOrderStatus::Cancelled);
        let leg = BasketLeg::new(terminal_parent, BasketLegRole::Primary, 10_000).expect("leg");
        let progress = AlgoProgress::new(terminal_parent.id(), terminal_parent.total_qty());
        let child_ids = [ChildOrderId::new("child-b").expect("child")];
        let client_ids = [ClientOrderId::new("cl-b").expect("client")];

        let decision = BasketPlanner::new()
            .plan_synchronized_slice::<1>(
                &[leg],
                &[progress],
                6_000,
                &child_ids,
                &client_ids,
                6_000,
            )
            .expect("decision");

        assert!(decision.is_empty());
        assert_eq!(decision.blocked_legs(), 1);
    }

    #[test]
    fn basket_rejects_mismatched_inputs() {
        let leg = BasketLeg::new(parent(), BasketLegRole::Primary, 10_000).expect("leg");
        assert_eq!(
            BasketPlanner::new().plan_synchronized_slice::<1>(&[leg], &[], 6_000, &[], &[], 6_000),
            Err(AlgoError::InvalidBasketParameters)
        );
        assert_eq!(
            BasketLeg::new(parent(), BasketLegRole::Primary, 0),
            Err(AlgoError::InvalidBasketParameters)
        );
    }

    #[test]
    fn spread_plans_when_edge_meets_limit() {
        let buy_parent = parent();
        let sell_parent = sell_parent("spread-sell-1", OrderQty(100), OrderQty(10), OrderQty(25));
        let planner = SpreadPlanner::new(SpreadConfig::new(10_000, 50).expect("config"));
        let decision = planner
            .plan_spread(
                &buy_parent,
                AlgoProgress::new(buy_parent.id(), buy_parent.total_qty()),
                &sell_parent,
                AlgoProgress::new(sell_parent.id(), sell_parent.total_qty()),
                2_000,
                SpreadQuote::new(OrderPrice(500_000), OrderPrice(505_000)).expect("quote"),
                ChildOrderId::new("sp-buy").expect("child"),
                ClientOrderId::new("sp-buy-cl").expect("client"),
                ChildOrderId::new("sp-sell").expect("child"),
                ClientOrderId::new("sp-sell-cl").expect("client"),
                2_000,
            )
            .expect("decision");

        assert!(decision.estimate().executable());
        assert_eq!(decision.estimate().edge_bps(), 100);
        let buy = decision.buy().expect("buy");
        let sell = decision.sell().expect("sell");
        assert_eq!(buy.request().side, OrderSide::Buy);
        assert_eq!(sell.request().side, OrderSide::Sell);
        assert_eq!(buy.request().quantity, OrderQty(25));
        assert_eq!(sell.request().quantity, OrderQty(25));
        assert_eq!(buy.request().limit_price, OrderPrice(500_000));
        assert_eq!(sell.request().limit_price, OrderPrice(505_000));
    }

    #[test]
    fn spread_waits_when_edge_is_below_limit() {
        let buy_parent = parent();
        let sell_parent = sell_parent("spread-sell-2", OrderQty(100), OrderQty(10), OrderQty(25));
        let planner = SpreadPlanner::new(SpreadConfig::new(10_000, 50).expect("config"));
        let decision = planner
            .plan_spread(
                &buy_parent,
                AlgoProgress::new(buy_parent.id(), buy_parent.total_qty()),
                &sell_parent,
                AlgoProgress::new(sell_parent.id(), sell_parent.total_qty()),
                2_000,
                SpreadQuote::new(OrderPrice(500_000), OrderPrice(500_500)).expect("quote"),
                ChildOrderId::new("sp-wait-buy").expect("child"),
                ClientOrderId::new("sp-wait-buy-cl").expect("client"),
                ChildOrderId::new("sp-wait-sell").expect("child"),
                ClientOrderId::new("sp-wait-sell-cl").expect("client"),
                2_000,
            )
            .expect("decision");

        assert!(!decision.estimate().executable());
        assert!(decision.buy().is_none());
        assert!(decision.sell().is_none());
    }

    #[test]
    fn spread_sizes_by_ratio_and_available_leaves() {
        let buy_parent = parent();
        let sell_parent = sell_parent("spread-sell-3", OrderQty(8), OrderQty(1), OrderQty(8));
        let planner = SpreadPlanner::new(SpreadConfig::new(5_000, 100).expect("config"));
        let decision = planner
            .plan_spread(
                &buy_parent,
                AlgoProgress::new(buy_parent.id(), buy_parent.total_qty()),
                &sell_parent,
                AlgoProgress::new(sell_parent.id(), sell_parent.total_qty()),
                2_000,
                SpreadQuote::new(OrderPrice(500_000), OrderPrice(1_100_000)).expect("quote"),
                ChildOrderId::new("sp-ratio-buy").expect("child"),
                ClientOrderId::new("sp-ratio-buy-cl").expect("client"),
                ChildOrderId::new("sp-ratio-sell").expect("child"),
                ClientOrderId::new("sp-ratio-sell-cl").expect("client"),
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.estimate().buy_qty(), OrderQty(16));
        assert_eq!(decision.estimate().sell_qty(), OrderQty(8));
        assert_eq!(
            decision.buy().expect("buy").request().quantity,
            OrderQty(16)
        );
        assert_eq!(
            decision.sell().expect("sell").request().quantity,
            OrderQty(8)
        );
    }

    #[test]
    fn spread_rejects_invalid_inputs() {
        assert_eq!(
            SpreadConfig::new(0, 0),
            Err(AlgoError::InvalidSpreadParameters)
        );
        assert_eq!(
            SpreadQuote::new(OrderPrice(0), OrderPrice(1)),
            Err(AlgoError::InvalidSpreadParameters)
        );
        let buy_parent = parent();
        let wrong_side_parent = ParentOrder::new(
            ParentOrderId::new("spread-wrong").expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("sim").expect("route"),
            StrategyId::new("spread").expect("strategy"),
            ExecutionSymbol::new("SIM", "NQZ6").expect("symbol"),
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::Day,
            OrderQty(100),
            OrderPrice(1_000_000),
            OrderPrice(0),
            1_000,
            11_000,
            OrderQty(10),
            OrderQty(25),
            0,
        )
        .expect("wrong side");

        assert_eq!(
            SpreadPlanner::new(SpreadConfig::new(10_000, 0).expect("config")).estimate(
                &buy_parent,
                AlgoProgress::new(buy_parent.id(), buy_parent.total_qty()),
                &wrong_side_parent,
                AlgoProgress::new(wrong_side_parent.id(), wrong_side_parent.total_qty()),
                SpreadQuote::new(OrderPrice(500_000), OrderPrice(505_000)).expect("quote"),
            ),
            Err(AlgoError::InvalidSpreadParameters)
        );
    }

    #[test]
    fn market_maker_quotes_both_sides_around_fair_value() {
        let template = parent();
        let config = MarketMakerConfig::new(
            OrderPrice(25),
            OrderQty(5),
            20,
            2,
            1_000,
            1_000,
            5_000,
            5_000,
        )
        .expect("config");
        let planner = MarketMakerPlanner::new(config);
        let context = MarketMakerContext::new(
            OrderPrice(500_000),
            OrderPrice(499_975),
            OrderPrice(500_025),
            OrderQty(0),
            OrderQty(100),
            10,
            10,
        )
        .expect("context");

        let decision = planner
            .plan_quotes(
                &template,
                2_000,
                context,
                ChildOrderId::new("mm-bid").expect("child"),
                ClientOrderId::new("mm-bid-cl").expect("client"),
                ChildOrderId::new("mm-ask").expect("child"),
                ClientOrderId::new("mm-ask-cl").expect("client"),
                2_000,
            )
            .expect("decision");

        let bid = decision.bid().expect("bid");
        let ask = decision.ask().expect("ask");
        assert_eq!(bid.request().side, OrderSide::Buy);
        assert_eq!(ask.request().side, OrderSide::Sell);
        assert!(bid.request().limit_price.0 < ask.request().limit_price.0);
        assert_eq!(bid.request().quantity, OrderQty(5));
        assert_eq!(ask.request().quantity, OrderQty(5));
    }

    #[test]
    fn market_maker_suppresses_bid_at_long_inventory_limit() {
        let template = parent();
        let planner = MarketMakerPlanner::new(MarketMakerConfig::default());
        let context = MarketMakerContext::new(
            OrderPrice(500_000),
            OrderPrice(499_975),
            OrderPrice(500_025),
            OrderQty(100),
            OrderQty(100),
            0,
            0,
        )
        .expect("context");

        let decision = planner
            .plan_quotes(
                &template,
                2_000,
                context,
                ChildOrderId::new("mm-bid").expect("child"),
                ClientOrderId::new("mm-bid-cl").expect("client"),
                ChildOrderId::new("mm-ask").expect("child"),
                ClientOrderId::new("mm-ask-cl").expect("client"),
                2_000,
            )
            .expect("decision");

        assert!(decision.bid().is_none());
        assert!(decision.ask().is_some());
        assert!(decision.estimate().adjusted_fair_value().0 < context.fair_value().0);
    }

    #[test]
    fn market_maker_rejects_invalid_parameters() {
        assert_eq!(
            MarketMakerConfig::new(OrderPrice(0), OrderQty(1), 10, 1, 100, 0, 0, 0),
            Err(AlgoError::InvalidMarketMakingParameters)
        );
        assert_eq!(
            MarketMakerContext::new(
                OrderPrice(1),
                OrderPrice(10),
                OrderPrice(9),
                OrderQty(0),
                OrderQty(1),
                0,
                0
            ),
            Err(AlgoError::InvalidMarketMakingParameters)
        );
    }

    #[test]
    fn shortfall_front_loads_on_adverse_buy_move() {
        let parent = parent();
        let planner = ImplementationShortfallPlanner::new(ImplementationShortfallConfig::default());
        let context = ImplementationShortfallContext::new(
            OrderPrice(500_000),
            OrderPrice(510_000),
            200,
            20,
            10,
        )
        .expect("context");
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let estimate = planner
            .estimate(&parent, progress, 1_000, context)
            .expect("estimate");

        assert!(estimate.adverse_move_bps() > 0);
        assert!(estimate.urgency_bps() > 1_000);
        assert!(estimate.target_release_qty().0 > 0);
        let child = planner
            .plan_shortfall_slice(
                &parent,
                progress,
                1_000,
                context,
                ChildOrderId::new("child-is").expect("child"),
                ClientOrderId::new("cl-is").expect("client"),
                1_000,
            )
            .expect("plan")
            .expect("due");
        assert!(child.request().quantity.0 >= parent.min_clip().0);
        assert!(child.request().quantity.0 <= parent.max_clip().0);
    }

    #[test]
    fn shortfall_high_impact_can_wait_below_min_clip() {
        let parent = parent();
        let config =
            ImplementationShortfallConfig::new(100, 1_000, 0, 0, 0, 10_000).expect("config");
        let planner = ImplementationShortfallPlanner::new(config);
        let context = ImplementationShortfallContext::new(
            OrderPrice(500_000),
            OrderPrice(500_000),
            0,
            0,
            500,
        )
        .expect("context");
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_shortfall_slice(
                &parent,
                progress,
                1_000,
                context,
                ChildOrderId::new("child-wait").expect("child"),
                ClientOrderId::new("cl-wait").expect("client"),
                1_000,
            )
            .expect("plan");

        assert!(child.is_none());
    }

    #[test]
    fn shortfall_detects_adverse_sell_move() {
        let sell_parent = ParentOrder::new(
            ParentOrderId::new("parent-sell").expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("sim").expect("route"),
            StrategyId::new("is").expect("strategy"),
            ExecutionSymbol::new("SIM", "ESZ6").expect("symbol"),
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::Day,
            OrderQty(100),
            OrderPrice(500_000),
            OrderPrice(0),
            1_000,
            11_000,
            OrderQty(10),
            OrderQty(25),
            0,
        )
        .expect("parent");
        let planner = ImplementationShortfallPlanner::new(ImplementationShortfallConfig::default());
        let context = ImplementationShortfallContext::new(
            OrderPrice(500_000),
            OrderPrice(490_000),
            50,
            10,
            0,
        )
        .expect("context");
        let estimate = planner
            .estimate(
                &sell_parent,
                AlgoProgress::new(sell_parent.id(), sell_parent.total_qty()),
                2_000,
                context,
            )
            .expect("estimate");

        assert!(estimate.adverse_move_bps() > 0);
        assert!(estimate.target_release_qty().0 > 10);
    }

    #[test]
    fn shortfall_rejects_invalid_parameters() {
        assert_eq!(
            ImplementationShortfallConfig::new(8_000, 7_000, 0, 0, 0, 0),
            Err(AlgoError::InvalidShortfallParameters)
        );
        assert_eq!(
            ImplementationShortfallContext::new(OrderPrice(0), OrderPrice(1), 0, 0, 0),
            Err(AlgoError::InvalidShortfallParameters)
        );
    }
}
