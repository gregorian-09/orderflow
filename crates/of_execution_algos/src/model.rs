use super::*;

/// Default maximum number of actions retained in an [`AlgoDecision`].
pub const DEFAULT_ALGO_DECISION_CAPACITY: usize = 16;
/// Default maximum number of retained violations in an [`AlgoRiskReport`].
pub const DEFAULT_ALGO_RISK_VIOLATION_CAPACITY: usize = 16;
/// Current algorithm checkpoint schema version.
pub const ALGO_CHECKPOINT_SCHEMA_VERSION: u16 = 1;

pub(crate) const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
pub(crate) const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

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
    /// Algorithm configuration is invalid.
    InvalidConfigParameters,
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
            Self::InvalidConfigParameters => write!(f, "invalid algorithm config parameters"),
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
    pub(crate) id: ParentOrderId,
    pub(crate) account_id: AccountId,
    pub(crate) route_id: RouteId,
    pub(crate) strategy_id: StrategyId,
    pub(crate) symbol: ExecutionSymbol,
    pub(crate) side: OrderSide,
    pub(crate) order_type: OrderType,
    pub(crate) time_in_force: TimeInForce,
    pub(crate) total_qty: OrderQty,
    pub(crate) limit_price: OrderPrice,
    pub(crate) stop_price: OrderPrice,
    pub(crate) start_ns: u64,
    pub(crate) end_ns: u64,
    pub(crate) min_clip: OrderQty,
    pub(crate) max_clip: OrderQty,
    pub(crate) participation_cap_bps: u16,
    pub(crate) status: ParentOrderStatus,
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

    pub(crate) fn build_order_request(
        &self,
        client_order_id: ClientOrderId,
        quantity: OrderQty,
        ts_recv_ns: u64,
    ) -> OrderRequest {
        self.build_order_request_at_price(client_order_id, quantity, self.limit_price, ts_recv_ns)
    }

    pub(crate) fn build_order_request_at_price(
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

    pub(crate) fn build_order_request_for_route_at_price(
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

    pub(crate) fn build_order_request_for_side_at_price(
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
    pub(crate) child_id: ChildOrderId,
    pub(crate) parent_id: ParentOrderId,
    pub(crate) request: OrderRequest,
    pub(crate) due_ns: u64,
    pub(crate) status: ChildOrderStatus,
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
    pub(crate) parent_id: ParentOrderId,
    pub(crate) target_qty: OrderQty,
    pub(crate) released_qty: OrderQty,
    pub(crate) completed_qty: OrderQty,
    pub(crate) open_qty: OrderQty,
    pub(crate) rejected_children: u64,
    pub(crate) terminal_children: u64,
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
    pub(crate) decision_seq: u64,
    pub(crate) actions: [Option<AlgoAction>; N],
    pub(crate) len: usize,
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
    pub(crate) kind: AlgoRiskViolationKind,
    pub(crate) child_id: Option<ChildOrderId>,
    pub(crate) measured: u128,
    pub(crate) limit: u128,
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
    pub(crate) max_parent_qty: OrderQty,
    pub(crate) max_child_qty: OrderQty,
    pub(crate) max_child_notional: u128,
    pub(crate) max_participation_bps: u16,
    pub(crate) price_collar_bps: u16,
    pub(crate) max_open_qty: OrderQty,
    pub(crate) max_children_per_decision: u16,
    pub(crate) max_child_orders_in_window: u32,
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
    pub(crate) reference_price: OrderPrice,
    pub(crate) observed_market_volume: OrderQty,
    pub(crate) open_child_orders: u32,
    pub(crate) child_orders_in_window: u32,
    pub(crate) stale_market_data: bool,
    pub(crate) route_degraded: bool,
    pub(crate) persistence_degraded: bool,
    pub(crate) kill_switch_active: bool,
    pub(crate) operator_paused: bool,
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
    pub(crate) outcome: AlgoRiskOutcome,
    pub(crate) violations: [Option<AlgoRiskViolation>; N],
    pub(crate) len: usize,
    pub(crate) truncated: bool,
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
    pub(crate) limits: AlgoRiskLimits,
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
    pub(crate) schema_version: u16,
    pub(crate) parent: ParentOrder,
    pub(crate) progress: AlgoProgress,
    pub(crate) next_decision_seq: u64,
    pub(crate) last_input_sequence: u64,
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
    pub(crate) pause_on_recovery: bool,
    pub(crate) require_reconciliation: bool,
    pub(crate) complete_when_progress_complete: bool,
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
    pub(crate) checkpoint: AlgoCheckpoint,
    pub(crate) action: AlgoRecoveryAction,
    pub(crate) replay_from_sequence: u64,
    pub(crate) next_decision_seq: u64,
    pub(crate) reconciliation_required: bool,
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
    pub(crate) available_qty: OrderQty,
    pub(crate) fill_price: OrderPrice,
    pub(crate) reject: bool,
    pub(crate) cancel_unfilled: bool,
    pub(crate) latency_ns: u64,
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
    pub(crate) sequence: u64,
    pub(crate) child_id: ChildOrderId,
    pub(crate) outcome: AlgoSimOutcome,
    pub(crate) filled_qty: OrderQty,
    pub(crate) leaves_qty: OrderQty,
    pub(crate) fill_price: OrderPrice,
    pub(crate) event: ExecutionEvent,
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
    pub(crate) steps: [Option<AlgoSimStep>; N],
    pub(crate) len: usize,
    pub(crate) truncated: bool,
    pub(crate) total_filled_qty: OrderQty,
    pub(crate) rejected_children: u64,
    pub(crate) cancelled_children: u64,
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
    pub(crate) market: AlgoSimMarket,
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
    pub(crate) arrival_price: OrderPrice,
    pub(crate) vwap_price: OrderPrice,
    pub(crate) twap_price: OrderPrice,
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
    pub(crate) parent_id: ParentOrderId,
    pub(crate) target_qty: OrderQty,
    pub(crate) submitted_children: u64,
    pub(crate) filled_children: u64,
    pub(crate) rejected_children: u64,
    pub(crate) cancelled_children: u64,
    pub(crate) completed_qty: OrderQty,
    pub(crate) completion_bps: u16,
    pub(crate) average_price: OrderPrice,
    pub(crate) arrival_slippage_bps: i32,
    pub(crate) vwap_slippage_bps: i32,
    pub(crate) twap_slippage_bps: i32,
    pub(crate) first_submit_ns: u64,
    pub(crate) last_event_ns: u64,
    pub(crate) average_latency_ns: u64,
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
    pub(crate) parent_id: ParentOrderId,
    pub(crate) side: OrderSide,
    pub(crate) target_qty: OrderQty,
    pub(crate) benchmarks: AlgoTcaBenchmark,
    pub(crate) submitted_children: u64,
    pub(crate) filled_children: u64,
    pub(crate) rejected_children: u64,
    pub(crate) cancelled_children: u64,
    pub(crate) completed_qty: OrderQty,
    pub(crate) cumulative_notional: u128,
    pub(crate) first_submit_ns: u64,
    pub(crate) last_event_ns: u64,
    pub(crate) total_latency_ns: u128,
    pub(crate) latency_samples: u64,
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

/// Execution algorithm category for typed configuration.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlgoKind {
    /// Time-weighted average price execution.
    Twap = 1,
    /// Percentage-of-volume execution.
    Pov = 2,
    /// Volume-weighted average price execution.
    Vwap = 3,
    /// Synthetic reserve/iceberg execution.
    Iceberg = 4,
    /// Implementation shortfall execution.
    ImplementationShortfall = 5,
    /// Passive queue/peg optimization.
    PassiveQueue = 6,
    /// Smart order routing.
    SmartOrderRouter = 7,
    /// Liquidity-seeking execution.
    LiquiditySeeking = 8,
    /// Aggressive sweep execution.
    Sweep = 9,
    /// Basket execution.
    Basket = 10,
    /// Two-leg spread execution.
    Spread = 11,
    /// Market-making quote planning.
    MarketMaking = 12,
    /// Host-defined algorithm.
    Custom = 255,
}

/// Typed parent-order configuration for algorithm construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoParentConfig {
    pub(crate) id: ParentOrderId,
    pub(crate) account_id: AccountId,
    pub(crate) route_id: RouteId,
    pub(crate) strategy_id: StrategyId,
    pub(crate) symbol: ExecutionSymbol,
    pub(crate) side: OrderSide,
    pub(crate) order_type: OrderType,
    pub(crate) time_in_force: TimeInForce,
    pub(crate) total_qty: OrderQty,
    pub(crate) limit_price: OrderPrice,
    pub(crate) stop_price: OrderPrice,
    pub(crate) start_ns: u64,
    pub(crate) end_ns: u64,
    pub(crate) min_clip: OrderQty,
    pub(crate) max_clip: OrderQty,
    pub(crate) participation_cap_bps: u16,
}

impl AlgoParentConfig {
    /// Creates typed parent-order configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when the resulting parent order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "flat config mirrors externally supplied order ticket fields"
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
        let config = Self {
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
        };
        config.to_parent_order()?;
        Ok(config)
    }

    /// Creates config from an existing parent order.
    pub const fn from_parent(parent: ParentOrder) -> Self {
        Self {
            id: parent.id(),
            account_id: parent.account_id(),
            route_id: parent.route_id(),
            strategy_id: parent.strategy_id(),
            symbol: parent.symbol(),
            side: parent.side(),
            order_type: parent.order_type(),
            time_in_force: parent.time_in_force(),
            total_qty: parent.total_qty(),
            limit_price: parent.limit_price(),
            stop_price: parent.stop_price(),
            start_ns: parent.start_ns(),
            end_ns: parent.end_ns(),
            min_clip: parent.min_clip(),
            max_clip: parent.max_clip(),
            participation_cap_bps: parent.participation_cap_bps(),
        }
    }

    /// Builds a validated active parent order from the config.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when the configured parent is invalid.
    pub fn to_parent_order(&self) -> Result<ParentOrder, AlgoError> {
        ParentOrder::new(
            self.id,
            self.account_id,
            self.route_id,
            self.strategy_id,
            self.symbol,
            self.side,
            self.order_type,
            self.time_in_force,
            self.total_qty,
            self.limit_price,
            self.stop_price,
            self.start_ns,
            self.end_ns,
            self.min_clip,
            self.max_clip,
            self.participation_cap_bps,
        )
    }

    /// Returns parent identifier.
    pub const fn id(&self) -> ParentOrderId {
        self.id
    }

    /// Returns account identifier.
    pub const fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns default route identifier.
    pub const fn route_id(&self) -> RouteId {
        self.route_id
    }

    /// Returns strategy identifier.
    pub const fn strategy_id(&self) -> StrategyId {
        self.strategy_id
    }

    /// Returns execution symbol.
    pub const fn symbol(&self) -> ExecutionSymbol {
        self.symbol
    }

    /// Returns order side.
    pub const fn side(&self) -> OrderSide {
        self.side
    }

    /// Returns order type.
    pub const fn order_type(&self) -> OrderType {
        self.order_type
    }

    /// Returns time in force.
    pub const fn time_in_force(&self) -> TimeInForce {
        self.time_in_force
    }

    /// Returns total quantity.
    pub const fn total_qty(&self) -> OrderQty {
        self.total_qty
    }

    /// Returns limit price.
    pub const fn limit_price(&self) -> OrderPrice {
        self.limit_price
    }

    /// Returns stop price.
    pub const fn stop_price(&self) -> OrderPrice {
        self.stop_price
    }

    /// Returns algorithm start timestamp.
    pub const fn start_ns(&self) -> u64 {
        self.start_ns
    }

    /// Returns algorithm end timestamp.
    pub const fn end_ns(&self) -> u64 {
        self.end_ns
    }

    /// Returns minimum clip.
    pub const fn min_clip(&self) -> OrderQty {
        self.min_clip
    }

    /// Returns maximum clip.
    pub const fn max_clip(&self) -> OrderQty {
        self.max_clip
    }

    /// Returns participation cap in basis points, or zero when unset.
    pub const fn participation_cap_bps(&self) -> u16 {
        self.participation_cap_bps
    }
}

/// Typed top-level algorithm configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoConfig {
    pub(crate) kind: AlgoKind,
    pub(crate) parent: AlgoParentConfig,
    pub(crate) risk_limits: AlgoRiskLimits,
    pub(crate) recovery_policy: AlgoRecoveryPolicy,
}

impl AlgoConfig {
    /// Creates algorithm configuration.
    pub const fn new(kind: AlgoKind, parent: AlgoParentConfig) -> Self {
        Self {
            kind,
            parent,
            risk_limits: AlgoRiskLimits::unbounded(),
            recovery_policy: AlgoRecoveryPolicy::new(true, true, true),
        }
    }

    /// Returns algorithm kind.
    pub const fn kind(&self) -> AlgoKind {
        self.kind
    }

    /// Returns parent config.
    pub const fn parent(&self) -> AlgoParentConfig {
        self.parent
    }

    /// Returns risk limits.
    pub const fn risk_limits(&self) -> AlgoRiskLimits {
        self.risk_limits
    }

    /// Returns recovery policy.
    pub const fn recovery_policy(&self) -> AlgoRecoveryPolicy {
        self.recovery_policy
    }

    /// Returns a copy with risk limits.
    pub const fn with_risk_limits(mut self, limits: AlgoRiskLimits) -> Self {
        self.risk_limits = limits;
        self
    }

    /// Returns a copy with recovery policy.
    pub const fn with_recovery_policy(mut self, policy: AlgoRecoveryPolicy) -> Self {
        self.recovery_policy = policy;
        self
    }

    /// Builds a validated active parent order from the config.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when the configured parent is invalid.
    pub fn to_parent_order(&self) -> Result<ParentOrder, AlgoError> {
        self.parent.to_parent_order()
    }

    /// Builds a risk policy from configured risk limits.
    pub const fn to_risk_policy(&self) -> AlgoRiskPolicy {
        AlgoRiskPolicy::new(self.risk_limits)
    }
}
