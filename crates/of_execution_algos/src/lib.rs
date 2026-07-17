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

fn div_ceil_u64(lhs: u64, rhs: u64) -> u64 {
    lhs / rhs + u64::from(!lhs.is_multiple_of(rhs))
}

fn div_ceil_i128(lhs: i128, rhs: i128) -> i128 {
    lhs / rhs + i128::from(lhs % rhs != 0)
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
}
