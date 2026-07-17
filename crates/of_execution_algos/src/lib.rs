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
        OrderRequest {
            client_order_id,
            account_id: self.account_id,
            route_id: self.route_id,
            strategy_id: self.strategy_id,
            symbol: self.symbol,
            side: self.side,
            order_type: self.order_type,
            time_in_force: self.time_in_force,
            quantity,
            limit_price: self.limit_price,
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
}
