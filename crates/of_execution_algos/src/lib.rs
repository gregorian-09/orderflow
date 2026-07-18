//! Parent/child execution algorithm primitives for Orderflow.
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::error::Error;
use std::fmt;

use of_execution_core::{
    AccountId, ClientOrderId, ExecutionCoreError, ExecutionEvent, ExecutionSymbol, FixedAscii,
    OrderPrice, OrderQty, OrderRequest, OrderSide, OrderStatus, OrderType, RouteId, StrategyId,
    TimeInForce,
};

/// Default maximum number of actions retained in an [`AlgoDecision`].
pub const DEFAULT_ALGO_DECISION_CAPACITY: usize = 16;

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
