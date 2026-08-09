//! Bounded OMS order-intent and parent/child lifecycle primitives.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use of_execution_core::{
    AccountId, ClientOrderId, ExecutionEvent, ExecutionSymbol, ExecutionType, FixedAscii,
    OrderPrice, OrderQty, OrderSide, OrderStatus, OrderType, RouteId, StrategyId, TimeInForce,
};

/// Maximum bytes stored in an OMS intent, parent, or child identifier.
pub const OMS_ORDER_TREE_ID_CAPACITY: usize = 40;

/// Stable strategy intent identifier.
pub type OrderIntentId = FixedAscii<OMS_ORDER_TREE_ID_CAPACITY>;
/// Stable OMS parent-order identifier.
pub type OmsParentOrderId = FixedAscii<OMS_ORDER_TREE_ID_CAPACITY>;
/// Stable OMS child-order identifier.
pub type OmsChildOrderId = FixedAscii<OMS_ORDER_TREE_ID_CAPACITY>;

/// Parent intent lifecycle state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OrderIntentState {
    /// Validated but not yet activated.
    Pending = 0,
    /// May allocate and route child orders.
    Active = 1,
    /// Child release is paused while existing children remain managed.
    Paused = 2,
    /// Parent cancellation is waiting for every working child to terminate.
    PendingCancel = 3,
    /// Target quantity filled completely.
    Completed = 4,
    /// Cancel tree completed before full fill.
    Cancelled = 5,
    /// Intent was rejected before activation.
    Rejected = 6,
    /// Lifecycle is uncertain and requires operator/recovery action.
    Failed = 7,
    /// Restored state awaits reconciliation before resume.
    Recovering = 8,
}

impl OrderIntentState {
    /// Returns true when no further lifecycle mutation is expected.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Rejected | Self::Failed
        )
    }
}

/// OMS-owned child lifecycle state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OmsChildOrderState {
    /// Allocated locally but not submitted.
    Planned = 0,
    /// Submitted to the execution engine.
    Submitted = 1,
    /// Accepted by the venue.
    Working = 2,
    /// Partially filled and still working.
    PartiallyFilled = 3,
    /// Fully filled.
    Filled = 4,
    /// Cancel request is pending.
    PendingCancel = 5,
    /// Cancelled with zero or partial fill.
    Cancelled = 6,
    /// Replacement superseded this child.
    Replaced = 7,
    /// Venue or local risk rejected the child.
    Rejected = 8,
    /// Child expired.
    Expired = 9,
    /// State is uncertain after recovery/reconciliation.
    Unknown = 10,
}

impl OmsChildOrderState {
    /// Returns true when the child no longer owns working leaves.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Filled | Self::Cancelled | Self::Replaced | Self::Rejected | Self::Expired
        )
    }
}

/// Routing and venue-order instructions for one child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionInstruction {
    /// Execution route.
    pub route_id: RouteId,
    /// Venue order type.
    pub order_type: OrderType,
    /// Time in force.
    pub time_in_force: TimeInForce,
    /// Limit price, or zero when not applicable.
    pub limit_price: OrderPrice,
    /// Stop price, or zero when not applicable.
    pub stop_price: OrderPrice,
    /// Displayed quantity for synthetic/native reserve handling.
    pub display_qty: OrderQty,
    /// Minimum acceptable execution quantity.
    pub minimum_qty: OrderQty,
    /// Do not permit the child to provide liquidity.
    pub post_only: bool,
    /// Require the child to reduce existing exposure.
    pub reduce_only: bool,
}

impl ExecutionInstruction {
    /// Creates basic route/type/TIF/price instructions.
    pub const fn new(
        route_id: RouteId,
        order_type: OrderType,
        time_in_force: TimeInForce,
        limit_price: OrderPrice,
    ) -> Self {
        Self {
            route_id,
            order_type,
            time_in_force,
            limit_price,
            stop_price: OrderPrice(0),
            display_qty: OrderQty(0),
            minimum_qty: OrderQty(0),
            post_only: false,
            reduce_only: false,
        }
    }

    /// Sets stop price.
    pub const fn with_stop_price(mut self, stop_price: OrderPrice) -> Self {
        self.stop_price = stop_price;
        self
    }

    /// Sets displayed quantity.
    pub const fn with_display_qty(mut self, display_qty: OrderQty) -> Self {
        self.display_qty = display_qty;
        self
    }

    /// Sets minimum execution quantity.
    pub const fn with_minimum_qty(mut self, minimum_qty: OrderQty) -> Self {
        self.minimum_qty = minimum_qty;
        self
    }

    /// Sets post-only behavior.
    pub const fn with_post_only(mut self, post_only: bool) -> Self {
        self.post_only = post_only;
        self
    }

    /// Sets reduce-only behavior.
    pub const fn with_reduce_only(mut self, reduce_only: bool) -> Self {
        self.reduce_only = reduce_only;
        self
    }
}

/// Immutable strategy intent and parent-level constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OrderIntent {
    /// Strategy intent id.
    pub intent_id: OrderIntentId,
    /// OMS parent id.
    pub parent_id: OmsParentOrderId,
    /// Account.
    pub account_id: AccountId,
    /// Strategy attribution.
    pub strategy_id: StrategyId,
    /// Venue-native symbol.
    pub symbol: ExecutionSymbol,
    /// Parent side.
    pub side: OrderSide,
    /// Parent target quantity.
    pub total_qty: OrderQty,
    /// Parent price cap/floor; zero means host policy supplies it.
    pub limit_price: OrderPrice,
    /// Maximum quantity allocated to one child.
    pub max_child_qty: OrderQty,
    /// Maximum simultaneous non-terminal children.
    pub max_open_children: u32,
    /// Maximum participation target in basis points; zero disables the field.
    pub participation_target_bps: u16,
    /// Earliest child release timestamp.
    pub start_ns: u64,
    /// Latest child release timestamp; zero means no lifecycle deadline.
    pub end_ns: u64,
    /// Intent creation timestamp.
    pub created_ns: u64,
}

impl OrderIntent {
    /// Creates a constrained order intent.
    #[allow(clippy::too_many_arguments, reason = "intent ownership is explicit")]
    pub fn new(
        intent_id: OrderIntentId,
        parent_id: OmsParentOrderId,
        account_id: AccountId,
        strategy_id: StrategyId,
        symbol: ExecutionSymbol,
        side: OrderSide,
        total_qty: OrderQty,
        limit_price: OrderPrice,
        max_child_qty: OrderQty,
        max_open_children: u32,
        participation_target_bps: u16,
        start_ns: u64,
        end_ns: u64,
        created_ns: u64,
    ) -> Result<Self, OrderIntentError> {
        let value = Self {
            intent_id,
            parent_id,
            account_id,
            strategy_id,
            symbol,
            side,
            total_qty,
            limit_price,
            max_child_qty,
            max_open_children,
            participation_target_bps,
            start_ns,
            end_ns,
            created_ns,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), OrderIntentError> {
        if self.intent_id.is_empty() || self.parent_id.is_empty() {
            return Err(OrderIntentError::MissingId);
        }
        if self.total_qty.0 <= 0
            || self.max_child_qty.0 <= 0
            || self.max_child_qty.0 > self.total_qty.0
        {
            return Err(OrderIntentError::InvalidQuantity);
        }
        if self.max_open_children == 0 {
            return Err(OrderIntentError::InvalidOpenChildLimit);
        }
        if self.participation_target_bps > 10_000 {
            return Err(OrderIntentError::InvalidParticipation);
        }
        if self.end_ns != 0 && self.start_ns >= self.end_ns {
            return Err(OrderIntentError::InvalidTimeWindow);
        }
        Ok(())
    }
}

/// OMS child order with replacement lineage and aggregate fill state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OmsChildOrder {
    /// Child id.
    pub child_id: OmsChildOrderId,
    /// Owning parent id.
    pub parent_id: OmsParentOrderId,
    /// Current client order id.
    pub client_order_id: ClientOrderId,
    /// Child this order replaces, when any.
    pub replaces_child_id: Option<OmsChildOrderId>,
    /// Child that superseded this order, when any.
    pub replaced_by_child_id: Option<OmsChildOrderId>,
    /// Routing/order instructions.
    pub instruction: ExecutionInstruction,
    /// Allocated quantity.
    pub order_qty: OrderQty,
    /// Cumulative fill quantity.
    pub cumulative_qty: OrderQty,
    /// Remaining working quantity.
    pub leaves_qty: OrderQty,
    /// Exact cumulative fill notional.
    pub fill_notional: i128,
    /// Derived average fill price.
    pub average_fill_price: OrderPrice,
    /// Lifecycle state.
    pub state: OmsChildOrderState,
    /// Last mutation sequence.
    pub last_sequence: u64,
    /// Last update timestamp.
    pub updated_ns: u64,
}

/// Read-only parent aggregate snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OrderIntentSnapshot {
    /// Immutable intent.
    pub intent: OrderIntent,
    /// Parent lifecycle state.
    pub state: OrderIntentState,
    /// Aggregate child-filled quantity.
    pub filled_qty: OrderQty,
    /// Parent target leaves.
    pub leaves_qty: OrderQty,
    /// Quantity currently owned by working children.
    pub working_qty: OrderQty,
    /// Quantity available for a new child.
    pub allocatable_qty: OrderQty,
    /// Exact aggregate child-fill notional.
    pub fill_notional: i128,
    /// Derived parent average fill price.
    pub average_fill_price: OrderPrice,
    /// Total child records.
    pub child_count: u32,
    /// Non-terminal child count.
    pub open_child_count: u32,
    /// Last mutation sequence.
    pub last_sequence: u64,
    /// Last update timestamp.
    pub updated_ns: u64,
}

/// Parent/child lifecycle validation and capacity error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OrderIntentError {
    /// Required intent, parent, child, or client id is empty.
    MissingId,
    /// Quantity is invalid or exceeds parent/child constraints.
    InvalidQuantity,
    /// Maximum open child count is zero.
    InvalidOpenChildLimit,
    /// Participation target exceeds 100 percent.
    InvalidParticipation,
    /// Start/end timestamps are invalid.
    InvalidTimeWindow,
    /// Mutation sequence is zero or did not advance.
    SequenceRegression,
    /// Lifecycle transition is invalid from the current state.
    InvalidTransition,
    /// Child id or client order id already exists.
    DuplicateChild,
    /// Child id or client order id is unknown.
    ChildNotFound,
    /// Configured child-record capacity is exhausted.
    ChildCapacityExceeded,
    /// Parent has reached its simultaneous open-child limit.
    OpenChildLimitExceeded,
    /// Execution report cumulative quantity regressed or exceeded allocation.
    InvalidExecutionProgress,
    /// Caller-owned cancel output is too small; no state changed.
    CancelBufferFull,
    /// Recovery snapshot is inconsistent or exceeds capacity.
    InvalidSnapshot,
}

impl fmt::Display for OrderIntentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::MissingId => "order intent id is empty",
                Self::InvalidQuantity => "order intent quantity is invalid",
                Self::InvalidOpenChildLimit => "open child limit is invalid",
                Self::InvalidParticipation => "participation target is invalid",
                Self::InvalidTimeWindow => "order intent time window is invalid",
                Self::SequenceRegression => "order intent sequence did not advance",
                Self::InvalidTransition => "order intent lifecycle transition is invalid",
                Self::DuplicateChild => "order intent child identity is duplicated",
                Self::ChildNotFound => "order intent child was not found",
                Self::ChildCapacityExceeded => "order intent child capacity exceeded",
                Self::OpenChildLimitExceeded => "order intent open child limit exceeded",
                Self::InvalidExecutionProgress => "child execution progress is invalid",
                Self::CancelBufferFull => "order intent cancel buffer is full",
                Self::InvalidSnapshot => "order intent recovery snapshot is invalid",
            }
        )
    }
}

impl Error for OrderIntentError {}

/// Child cancellation target selected by parent cancel-tree processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OmsChildCancelTarget {
    /// Child id.
    pub child_id: OmsChildOrderId,
    /// Current client order id.
    pub client_order_id: ClientOrderId,
    /// Route for cancel dispatch.
    pub route_id: RouteId,
    /// Remaining quantity at selection time.
    pub leaves_qty: OrderQty,
}

/// Caller-owned bounded cancel-tree output.
#[derive(Debug, Clone)]
pub struct OmsChildCancelBuffer {
    targets: Vec<OmsChildCancelTarget>,
    capacity: usize,
}

impl OmsChildCancelBuffer {
    /// Creates an empty bounded target buffer.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            targets: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Clears targets without releasing allocation.
    pub fn clear(&mut self) {
        self.targets.clear();
    }

    /// Returns selected targets.
    pub fn as_slice(&self) -> &[OmsChildCancelTarget] {
        &self.targets
    }
}

/// Recovery payload for one complete parent/child tree.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OrderIntentRecoverySnapshot {
    /// Parent aggregate state.
    pub parent: OrderIntentSnapshot,
    /// Child rows sorted by child id.
    pub children: Vec<OmsChildOrder>,
}

/// Bounded single-owner OMS parent/child lifecycle.
#[derive(Debug, Clone)]
pub struct OrderIntentLifecycle {
    intent: OrderIntent,
    state: OrderIntentState,
    children: HashMap<OmsChildOrderId, OmsChildOrder>,
    client_index: HashMap<ClientOrderId, OmsChildOrderId>,
    cancel_scratch: Vec<OmsChildOrderId>,
    child_capacity: usize,
    filled_qty: i64,
    working_qty: i64,
    fill_notional: i128,
    last_sequence: u64,
    updated_ns: u64,
}

impl OrderIntentLifecycle {
    /// Creates a pending lifecycle with pre-sized child indexes.
    pub fn new(intent: OrderIntent, child_capacity: usize) -> Result<Self, OrderIntentError> {
        intent.validate()?;
        if child_capacity == 0 || child_capacity < intent.max_open_children as usize {
            return Err(OrderIntentError::ChildCapacityExceeded);
        }
        Ok(Self {
            intent,
            state: OrderIntentState::Pending,
            children: HashMap::with_capacity(child_capacity),
            client_index: HashMap::with_capacity(child_capacity),
            cancel_scratch: Vec::with_capacity(child_capacity),
            child_capacity,
            filled_qty: 0,
            working_qty: 0,
            fill_notional: 0,
            last_sequence: 0,
            updated_ns: intent.created_ns,
        })
    }

    /// Activates a pending or reconciled recovering parent.
    pub fn activate(&mut self, sequence: u64, timestamp_ns: u64) -> Result<(), OrderIntentError> {
        self.check_sequence(sequence)?;
        if !matches!(
            self.state,
            OrderIntentState::Pending | OrderIntentState::Recovering
        ) {
            return Err(OrderIntentError::InvalidTransition);
        }
        self.state = OrderIntentState::Active;
        self.commit(sequence, timestamp_ns);
        Ok(())
    }

    /// Pauses new child release while preserving existing child management.
    pub fn pause(&mut self, sequence: u64, timestamp_ns: u64) -> Result<(), OrderIntentError> {
        self.check_sequence(sequence)?;
        if self.state != OrderIntentState::Active {
            return Err(OrderIntentError::InvalidTransition);
        }
        self.state = OrderIntentState::Paused;
        self.commit(sequence, timestamp_ns);
        Ok(())
    }

    /// Resumes child release after host risk/reconciliation approval.
    pub fn resume(&mut self, sequence: u64, timestamp_ns: u64) -> Result<(), OrderIntentError> {
        self.check_sequence(sequence)?;
        if self.state != OrderIntentState::Paused {
            return Err(OrderIntentError::InvalidTransition);
        }
        self.state = OrderIntentState::Active;
        self.commit(sequence, timestamp_ns);
        Ok(())
    }

    /// Marks an unactivated intent rejected.
    pub fn reject(&mut self, sequence: u64, timestamp_ns: u64) -> Result<(), OrderIntentError> {
        self.check_sequence(sequence)?;
        if self.state != OrderIntentState::Pending {
            return Err(OrderIntentError::InvalidTransition);
        }
        self.state = OrderIntentState::Rejected;
        self.commit(sequence, timestamp_ns);
        Ok(())
    }

    /// Marks uncertain lifecycle state failed and blocks further child release.
    pub fn fail(&mut self, sequence: u64, timestamp_ns: u64) -> Result<(), OrderIntentError> {
        self.check_sequence(sequence)?;
        if self.state.is_terminal() {
            return Err(OrderIntentError::InvalidTransition);
        }
        self.state = OrderIntentState::Failed;
        self.commit(sequence, timestamp_ns);
        Ok(())
    }

    /// Allocates a new child under parent quantity and concurrency constraints.
    #[allow(
        clippy::too_many_arguments,
        reason = "child identity and routing stay explicit"
    )]
    pub fn plan_child(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        child_id: OmsChildOrderId,
        client_order_id: ClientOrderId,
        quantity: OrderQty,
        instruction: ExecutionInstruction,
    ) -> Result<OmsChildOrder, OrderIntentError> {
        self.check_sequence(sequence)?;
        if self.state != OrderIntentState::Active {
            return Err(OrderIntentError::InvalidTransition);
        }
        if timestamp_ns < self.intent.start_ns
            || self.intent.end_ns != 0 && timestamp_ns >= self.intent.end_ns
        {
            return Err(OrderIntentError::InvalidTimeWindow);
        }
        let mut child =
            self.validate_new_child(child_id, client_order_id, quantity, instruction, None, 0)?;
        child.last_sequence = sequence;
        child.updated_ns = timestamp_ns;
        self.children.insert(child_id, child);
        self.client_index.insert(client_order_id, child_id);
        self.working_qty = self.working_qty.saturating_add(quantity.0);
        self.commit(sequence, timestamp_ns);
        Ok(child)
    }

    /// Replaces one live child with a new lineage id and allocation.
    ///
    /// The old child's unfilled leaves are released before validating the new
    /// allocation. State changes are committed only after every check passes.
    #[allow(
        clippy::too_many_arguments,
        reason = "replacement lineage stays explicit"
    )]
    pub fn replace_child(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        old_child_id: OmsChildOrderId,
        new_child_id: OmsChildOrderId,
        new_client_order_id: ClientOrderId,
        quantity: OrderQty,
        instruction: ExecutionInstruction,
    ) -> Result<OmsChildOrder, OrderIntentError> {
        self.check_sequence(sequence)?;
        if !matches!(
            self.state,
            OrderIntentState::Active | OrderIntentState::Paused
        ) {
            return Err(OrderIntentError::InvalidTransition);
        }
        let old = self
            .children
            .get(&old_child_id)
            .copied()
            .ok_or(OrderIntentError::ChildNotFound)?;
        if old.state.is_terminal() || old.state == OmsChildOrderState::PendingCancel {
            return Err(OrderIntentError::InvalidTransition);
        }
        let mut replacement = self.validate_new_child(
            new_child_id,
            new_client_order_id,
            quantity,
            instruction,
            Some(old_child_id),
            old.leaves_qty.0,
        )?;
        replacement.last_sequence = sequence;
        replacement.updated_ns = timestamp_ns;
        let mut updated_old = old;
        updated_old.state = OmsChildOrderState::Replaced;
        updated_old.replaced_by_child_id = Some(new_child_id);
        updated_old.leaves_qty = OrderQty(0);
        updated_old.last_sequence = sequence;
        updated_old.updated_ns = timestamp_ns;
        self.working_qty = self
            .working_qty
            .saturating_sub(old.leaves_qty.0)
            .saturating_add(quantity.0);
        self.children.insert(old_child_id, updated_old);
        self.children.insert(new_child_id, replacement);
        self.client_index.insert(new_client_order_id, new_child_id);
        self.commit(sequence, timestamp_ns);
        Ok(replacement)
    }

    /// Marks a planned child submitted to the existing execution engine.
    pub fn mark_submitted(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        child_id: OmsChildOrderId,
    ) -> Result<OmsChildOrder, OrderIntentError> {
        self.transition_child(
            sequence,
            timestamp_ns,
            child_id,
            OmsChildOrderState::Planned,
            OmsChildOrderState::Submitted,
        )
    }

    /// Folds one canonical execution report into child and parent aggregates.
    pub fn apply_execution_event(
        &mut self,
        sequence: u64,
        event: &ExecutionEvent,
    ) -> Result<OmsChildOrder, OrderIntentError> {
        self.check_sequence(sequence)?;
        let child_id = self
            .client_index
            .get(&event.client_order_id)
            .copied()
            .ok_or(OrderIntentError::ChildNotFound)?;
        let mut child = self.children[&child_id];
        if event.account_id != self.intent.account_id
            || event.symbol != self.intent.symbol
            || event.route_id != child.instruction.route_id
        {
            return Err(OrderIntentError::InvalidExecutionProgress);
        }
        let was_working = !child.state.is_terminal();
        let preserved_terminal = matches!(
            child.state,
            OmsChildOrderState::Cancelled
                | OmsChildOrderState::Replaced
                | OmsChildOrderState::Expired
        )
        .then_some(child.state);
        if child.state.is_terminal()
            && (preserved_terminal.is_none() || event.exec_type != ExecutionType::Trade)
        {
            return Err(OrderIntentError::InvalidTransition);
        }
        if event.cumulative_qty.0 < child.cumulative_qty.0
            || event.cumulative_qty.0 > child.order_qty.0
        {
            return Err(OrderIntentError::InvalidExecutionProgress);
        }
        let fill_delta = event
            .cumulative_qty
            .0
            .saturating_sub(child.cumulative_qty.0);
        if fill_delta > 0
            && (event.exec_type != ExecutionType::Trade || event.last_qty.0 != fill_delta)
        {
            return Err(OrderIntentError::InvalidExecutionProgress);
        }
        if self.filled_qty.saturating_add(fill_delta) > self.intent.total_qty.0 {
            return Err(OrderIntentError::InvalidExecutionProgress);
        }
        if fill_delta > 0 {
            if event.last_price.0 <= 0 {
                return Err(OrderIntentError::InvalidExecutionProgress);
            }
            child.fill_notional = child.fill_notional.saturating_add(
                i128::from(fill_delta).saturating_mul(i128::from(event.last_price.0)),
            );
            self.fill_notional = self.fill_notional.saturating_add(
                i128::from(fill_delta).saturating_mul(i128::from(event.last_price.0)),
            );
            self.filled_qty = self.filled_qty.saturating_add(fill_delta);
            if was_working {
                self.working_qty = self.working_qty.saturating_sub(fill_delta);
            }
        }
        child.cumulative_qty = event.cumulative_qty;
        child.leaves_qty = OrderQty(child.order_qty.0.saturating_sub(event.cumulative_qty.0));
        child.average_fill_price = if child.cumulative_qty.0 == 0 {
            OrderPrice(0)
        } else {
            OrderPrice(
                i64::try_from(child.fill_notional / i128::from(child.cumulative_qty.0))
                    .unwrap_or(i64::MAX),
            )
        };
        let next = if let Some(state) = preserved_terminal {
            state
        } else {
            child_state_from_order_status(event.order_status)
        };
        if next.is_terminal() {
            if was_working {
                self.working_qty = self.working_qty.saturating_sub(child.leaves_qty.0);
            }
            child.leaves_qty = OrderQty(0);
        }
        child.state = next;
        child.last_sequence = sequence;
        child.updated_ns = event.ts_recv_ns;
        self.children.insert(child_id, child);
        self.commit(sequence, event.ts_recv_ns);
        self.refresh_parent_terminal_state();
        Ok(child)
    }

    /// Selects every non-terminal child for cancellation atomically.
    pub fn request_cancel_tree(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        out: &mut OmsChildCancelBuffer,
    ) -> Result<OrderIntentSnapshot, OrderIntentError> {
        self.check_sequence(sequence)?;
        if matches!(
            self.state,
            OrderIntentState::Completed
                | OrderIntentState::Cancelled
                | OrderIntentState::Rejected
                | OrderIntentState::PendingCancel
        ) {
            return Err(OrderIntentError::InvalidTransition);
        }
        let required = self
            .children
            .values()
            .filter(|child| child_requires_venue_cancel(child.state))
            .count();
        if required > out.capacity {
            return Err(OrderIntentError::CancelBufferFull);
        }
        out.clear();
        self.cancel_scratch.clear();
        self.cancel_scratch.extend(
            self.children
                .values()
                .filter(|child| child_requires_venue_cancel(child.state))
                .map(|child| child.child_id),
        );
        self.cancel_scratch
            .sort_unstable_by(|left, right| left.as_str().cmp(right.as_str()));
        for id in self.cancel_scratch.iter().copied() {
            let child = self.children.get_mut(&id).expect("selected child exists");
            out.targets.push(OmsChildCancelTarget {
                child_id: id,
                client_order_id: child.client_order_id,
                route_id: child.instruction.route_id,
                leaves_qty: child.leaves_qty,
            });
            child.state = OmsChildOrderState::PendingCancel;
            child.last_sequence = sequence;
            child.updated_ns = timestamp_ns;
        }
        for child in self
            .children
            .values_mut()
            .filter(|child| child.state == OmsChildOrderState::Planned)
        {
            self.working_qty = self.working_qty.saturating_sub(child.leaves_qty.0);
            child.leaves_qty = OrderQty(0);
            child.state = OmsChildOrderState::Cancelled;
            child.last_sequence = sequence;
            child.updated_ns = timestamp_ns;
        }
        self.state = if required == 0 {
            if self.filled_qty == self.intent.total_qty.0 {
                OrderIntentState::Completed
            } else {
                OrderIntentState::Cancelled
            }
        } else {
            OrderIntentState::PendingCancel
        };
        self.commit(sequence, timestamp_ns);
        Ok(self.snapshot())
    }

    /// Returns parent aggregate state.
    pub fn snapshot(&self) -> OrderIntentSnapshot {
        let leaves = self.intent.total_qty.0.saturating_sub(self.filled_qty);
        let allocatable = leaves.saturating_sub(self.working_qty);
        OrderIntentSnapshot {
            intent: self.intent,
            state: self.state,
            filled_qty: OrderQty(self.filled_qty),
            leaves_qty: OrderQty(leaves),
            working_qty: OrderQty(self.working_qty),
            allocatable_qty: OrderQty(allocatable),
            fill_notional: self.fill_notional,
            average_fill_price: if self.filled_qty == 0 {
                OrderPrice(0)
            } else {
                OrderPrice(
                    i64::try_from(self.fill_notional / i128::from(self.filled_qty))
                        .unwrap_or(i64::MAX),
                )
            },
            child_count: self.children.len().min(u32::MAX as usize) as u32,
            open_child_count: self.open_child_count(),
            last_sequence: self.last_sequence,
            updated_ns: self.updated_ns,
        }
    }

    /// Returns one child by id.
    pub fn child(&self, child_id: OmsChildOrderId) -> Option<OmsChildOrder> {
        self.children.get(&child_id).copied()
    }

    /// Captures a deterministic recovery snapshot.
    pub fn recovery_snapshot(&self) -> OrderIntentRecoverySnapshot {
        let mut children = self.children.values().copied().collect::<Vec<_>>();
        children.sort_by(|left, right| left.child_id.as_str().cmp(right.child_id.as_str()));
        OrderIntentRecoverySnapshot {
            parent: self.snapshot(),
            children,
        }
    }

    /// Restores a tree in `Recovering` state for host reconciliation.
    pub fn restore(
        snapshot: &OrderIntentRecoverySnapshot,
        child_capacity: usize,
    ) -> Result<Self, OrderIntentError> {
        if snapshot.children.len() > child_capacity || snapshot.parent.intent.validate().is_err() {
            return Err(OrderIntentError::InvalidSnapshot);
        }
        let mut lifecycle = Self::new(snapshot.parent.intent, child_capacity)?;
        lifecycle.state = if snapshot.parent.state.is_terminal() {
            snapshot.parent.state
        } else {
            OrderIntentState::Recovering
        };
        lifecycle.filled_qty = 0;
        lifecycle.working_qty = 0;
        lifecycle.fill_notional = 0;
        lifecycle.last_sequence = snapshot.parent.last_sequence;
        lifecycle.updated_ns = snapshot.parent.updated_ns;
        for child in &snapshot.children {
            if !valid_recovery_child(child, lifecycle.intent, snapshot.parent.last_sequence)
                || lifecycle.children.insert(child.child_id, *child).is_some()
                || lifecycle
                    .client_index
                    .insert(child.client_order_id, child.child_id)
                    .is_some()
            {
                return Err(OrderIntentError::InvalidSnapshot);
            }
            lifecycle.filled_qty = lifecycle.filled_qty.saturating_add(child.cumulative_qty.0);
            lifecycle.fill_notional = lifecycle.fill_notional.saturating_add(child.fill_notional);
            if !child.state.is_terminal() {
                lifecycle.working_qty = lifecycle.working_qty.saturating_add(child.leaves_qty.0);
            }
        }
        if lifecycle
            .children
            .values()
            .any(|child| !valid_replacement_lineage(child, &lifecycle.children))
        {
            return Err(OrderIntentError::InvalidSnapshot);
        }
        let computed = lifecycle.snapshot();
        if computed.filled_qty != snapshot.parent.filled_qty
            || computed.working_qty != snapshot.parent.working_qty
            || computed.leaves_qty != snapshot.parent.leaves_qty
            || computed.allocatable_qty != snapshot.parent.allocatable_qty
            || computed.fill_notional != snapshot.parent.fill_notional
            || computed.average_fill_price != snapshot.parent.average_fill_price
            || computed.child_count != snapshot.parent.child_count
            || computed.open_child_count != snapshot.parent.open_child_count
            || computed.open_child_count > lifecycle.intent.max_open_children
            || computed.filled_qty.0 > lifecycle.intent.total_qty.0
        {
            return Err(OrderIntentError::InvalidSnapshot);
        }
        Ok(lifecycle)
    }

    fn validate_new_child(
        &self,
        child_id: OmsChildOrderId,
        client_order_id: ClientOrderId,
        quantity: OrderQty,
        instruction: ExecutionInstruction,
        replaces: Option<OmsChildOrderId>,
        released_qty: i64,
    ) -> Result<OmsChildOrder, OrderIntentError> {
        if child_id.is_empty() || client_order_id.is_empty() {
            return Err(OrderIntentError::MissingId);
        }
        if instruction.route_id.is_empty() {
            return Err(OrderIntentError::MissingId);
        }
        if self.children.contains_key(&child_id) || self.client_index.contains_key(&client_order_id)
        {
            return Err(OrderIntentError::DuplicateChild);
        }
        if self.children.len() >= self.child_capacity {
            return Err(OrderIntentError::ChildCapacityExceeded);
        }
        let replacing_open = usize::from(replaces.is_some());
        if self.open_child_count() as usize
            >= self.intent.max_open_children as usize + replacing_open
        {
            return Err(OrderIntentError::OpenChildLimitExceeded);
        }
        let allocatable = self
            .intent
            .total_qty
            .0
            .saturating_sub(self.filled_qty)
            .saturating_sub(self.working_qty)
            .saturating_add(released_qty);
        if quantity.0 <= 0
            || quantity.0 > self.intent.max_child_qty.0
            || quantity.0 > allocatable
            || instruction.display_qty.0 < 0
            || instruction.display_qty.0 > quantity.0
            || instruction.minimum_qty.0 < 0
            || instruction.minimum_qty.0 > quantity.0
        {
            return Err(OrderIntentError::InvalidQuantity);
        }
        validate_instruction(self.intent, instruction)?;
        Ok(OmsChildOrder {
            child_id,
            parent_id: self.intent.parent_id,
            client_order_id,
            replaces_child_id: replaces,
            replaced_by_child_id: None,
            instruction,
            order_qty: quantity,
            cumulative_qty: OrderQty(0),
            leaves_qty: quantity,
            fill_notional: 0,
            average_fill_price: OrderPrice(0),
            state: OmsChildOrderState::Planned,
            last_sequence: self.last_sequence,
            updated_ns: self.updated_ns,
        })
    }

    fn transition_child(
        &mut self,
        sequence: u64,
        timestamp_ns: u64,
        child_id: OmsChildOrderId,
        expected: OmsChildOrderState,
        next: OmsChildOrderState,
    ) -> Result<OmsChildOrder, OrderIntentError> {
        self.check_sequence(sequence)?;
        let mut child = self
            .children
            .get(&child_id)
            .copied()
            .ok_or(OrderIntentError::ChildNotFound)?;
        if child.state != expected {
            return Err(OrderIntentError::InvalidTransition);
        }
        child.state = next;
        child.last_sequence = sequence;
        child.updated_ns = timestamp_ns;
        self.children.insert(child_id, child);
        self.commit(sequence, timestamp_ns);
        Ok(child)
    }

    fn open_child_count(&self) -> u32 {
        self.children
            .values()
            .filter(|child| !child.state.is_terminal())
            .count()
            .min(u32::MAX as usize) as u32
    }

    fn check_sequence(&self, sequence: u64) -> Result<(), OrderIntentError> {
        if sequence == 0 || sequence <= self.last_sequence {
            return Err(OrderIntentError::SequenceRegression);
        }
        Ok(())
    }

    fn commit(&mut self, sequence: u64, timestamp_ns: u64) {
        self.last_sequence = sequence;
        self.updated_ns = timestamp_ns;
    }

    fn refresh_parent_terminal_state(&mut self) {
        if self.filled_qty == self.intent.total_qty.0 {
            self.state = OrderIntentState::Completed;
        } else if self.state == OrderIntentState::PendingCancel
            && self
                .children
                .values()
                .all(|child| child.state.is_terminal())
        {
            self.state = OrderIntentState::Cancelled;
        }
    }
}

fn child_state_from_order_status(status: OrderStatus) -> OmsChildOrderState {
    match status {
        OrderStatus::PendingNew => OmsChildOrderState::Submitted,
        OrderStatus::New => OmsChildOrderState::Working,
        OrderStatus::PartiallyFilled => OmsChildOrderState::PartiallyFilled,
        OrderStatus::Filled => OmsChildOrderState::Filled,
        OrderStatus::PendingCancel => OmsChildOrderState::PendingCancel,
        OrderStatus::Cancelled => OmsChildOrderState::Cancelled,
        OrderStatus::PendingReplace => OmsChildOrderState::Submitted,
        OrderStatus::Replaced => OmsChildOrderState::Working,
        OrderStatus::Rejected => OmsChildOrderState::Rejected,
        OrderStatus::Expired => OmsChildOrderState::Expired,
        OrderStatus::Suspended | OrderStatus::Unknown => OmsChildOrderState::Unknown,
    }
}

fn valid_recovery_child(child: &OmsChildOrder, intent: OrderIntent, parent_sequence: u64) -> bool {
    if child.parent_id != intent.parent_id
        || child.child_id.is_empty()
        || child.client_order_id.is_empty()
        || child.instruction.route_id.is_empty()
        || child.order_qty.0 <= 0
        || child.order_qty.0 > intent.max_child_qty.0
        || child.instruction.display_qty.0 < 0
        || child.instruction.display_qty.0 > child.order_qty.0
        || child.instruction.minimum_qty.0 < 0
        || child.instruction.minimum_qty.0 > child.order_qty.0
        || child.cumulative_qty.0 < 0
        || child.cumulative_qty.0 > child.order_qty.0
        || child.fill_notional < 0
        || child.last_sequence > parent_sequence
    {
        return false;
    }
    if validate_instruction(intent, child.instruction).is_err() {
        return false;
    }
    let expected_leaves = if child.state.is_terminal() {
        0
    } else {
        child.order_qty.0.saturating_sub(child.cumulative_qty.0)
    };
    let expected_average = if child.cumulative_qty.0 == 0 {
        0
    } else {
        i64::try_from(child.fill_notional / i128::from(child.cumulative_qty.0)).unwrap_or(i64::MAX)
    };
    child.leaves_qty.0 == expected_leaves && child.average_fill_price.0 == expected_average
}

fn valid_replacement_lineage(
    child: &OmsChildOrder,
    children: &HashMap<OmsChildOrderId, OmsChildOrder>,
) -> bool {
    if let Some(previous_id) = child.replaces_child_id {
        let Some(previous) = children.get(&previous_id) else {
            return false;
        };
        if previous_id == child.child_id
            || previous.replaced_by_child_id != Some(child.child_id)
            || previous.last_sequence > child.last_sequence
        {
            return false;
        }
    }
    if let Some(next_id) = child.replaced_by_child_id {
        let Some(next) = children.get(&next_id) else {
            return false;
        };
        if next_id == child.child_id || next.replaces_child_id != Some(child.child_id) {
            return false;
        }
    }

    let mut cursor = child.replaces_child_id;
    for _ in 0..children.len() {
        let Some(ancestor_id) = cursor else {
            return true;
        };
        if ancestor_id == child.child_id {
            return false;
        }
        cursor = children
            .get(&ancestor_id)
            .and_then(|ancestor| ancestor.replaces_child_id);
    }
    cursor.is_none()
}

fn child_requires_venue_cancel(state: OmsChildOrderState) -> bool {
    matches!(
        state,
        OmsChildOrderState::Submitted
            | OmsChildOrderState::Working
            | OmsChildOrderState::PartiallyFilled
            | OmsChildOrderState::PendingCancel
            | OmsChildOrderState::Unknown
    )
}

fn validate_instruction(
    intent: OrderIntent,
    instruction: ExecutionInstruction,
) -> Result<(), OrderIntentError> {
    let valid_prices = match instruction.order_type {
        OrderType::Market => true,
        OrderType::Limit => instruction.limit_price.0 > 0,
        OrderType::Stop => instruction.stop_price.0 > 0,
        OrderType::StopLimit => instruction.limit_price.0 > 0 && instruction.stop_price.0 > 0,
    };
    if !valid_prices || instruction.post_only && instruction.order_type != OrderType::Limit {
        return Err(OrderIntentError::InvalidTransition);
    }
    if intent.limit_price.0 > 0 && instruction.limit_price.0 > 0 {
        let violates = match intent.side {
            OrderSide::Buy => instruction.limit_price.0 > intent.limit_price.0,
            OrderSide::Sell => instruction.limit_price.0 < intent.limit_price.0,
        };
        if violates {
            return Err(OrderIntentError::InvalidTransition);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use of_execution_core::{ExecutionId, ExecutionText, RiskRejectReason, VenueOrderId};

    use super::*;

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn intent(max_open: u32) -> OrderIntent {
        OrderIntent::new(
            id("intent-1"),
            id("parent-1"),
            id("account-a"),
            id("strategy-a"),
            ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            OrderSide::Buy,
            OrderQty(10),
            OrderPrice(110),
            OrderQty(6),
            max_open,
            2_000,
            100,
            1_000,
            50,
        )
        .unwrap()
    }

    fn instruction() -> ExecutionInstruction {
        ExecutionInstruction::new(
            id("route-a"),
            OrderType::Limit,
            TimeInForce::Day,
            OrderPrice(105),
        )
    }

    fn event(
        client: &str,
        status: OrderStatus,
        cumulative: i64,
        last_qty: i64,
        last_price: i64,
        timestamp: u64,
    ) -> ExecutionEvent {
        ExecutionEvent {
            exec_type: if cumulative > 0 {
                ExecutionType::Trade
            } else {
                ExecutionType::Ack
            },
            order_status: status,
            client_order_id: id(client),
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: VenueOrderId::new("venue-1").unwrap(),
            execution_id: ExecutionId::new(&format!("exec-{timestamp}")).unwrap(),
            account_id: id("account-a"),
            route_id: id("route-a"),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            last_qty: OrderQty(last_qty),
            last_price: OrderPrice(last_price),
            cumulative_qty: OrderQty(cumulative),
            leaves_qty: OrderQty(0),
            average_price: OrderPrice(0),
            ts_exchange_ns: timestamp.saturating_sub(1),
            ts_recv_ns: timestamp,
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        }
    }

    #[test]
    fn intent_validation_rejects_unsafe_constraints() {
        assert!(matches!(
            OrderIntent::new(
                id(""),
                id("parent"),
                id("a"),
                id("s"),
                ExecutionSymbol::new("X", "Y").unwrap(),
                OrderSide::Buy,
                OrderQty(1),
                OrderPrice(0),
                OrderQty(1),
                1,
                0,
                0,
                0,
                0,
            ),
            Err(OrderIntentError::MissingId)
        ));
        assert!(matches!(
            OrderIntent::new(
                id("i"),
                id("p"),
                id("a"),
                id("s"),
                ExecutionSymbol::new("X", "Y").unwrap(),
                OrderSide::Buy,
                OrderQty(1),
                OrderPrice(0),
                OrderQty(1),
                1,
                10_001,
                0,
                0,
                0,
            ),
            Err(OrderIntentError::InvalidParticipation)
        ));
    }

    #[test]
    fn lifecycle_aggregates_children_and_completes_parent() {
        let mut lifecycle = OrderIntentLifecycle::new(intent(2), 4).unwrap();
        lifecycle.activate(1, 60).unwrap();
        assert!(matches!(
            lifecycle.plan_child(2, 99, id("c0"), id("client-0"), OrderQty(1), instruction()),
            Err(OrderIntentError::InvalidTimeWindow)
        ));
        lifecycle
            .plan_child(2, 100, id("c1"), id("client-1"), OrderQty(5), instruction())
            .unwrap();
        lifecycle.mark_submitted(3, 101, id("c1")).unwrap();
        lifecycle
            .apply_execution_event(
                4,
                &event("client-1", OrderStatus::PartiallyFilled, 3, 3, 101, 102),
            )
            .unwrap();
        let snapshot = lifecycle.snapshot();
        assert_eq!(snapshot.filled_qty, OrderQty(3));
        assert_eq!(snapshot.working_qty, OrderQty(2));
        assert_eq!(snapshot.allocatable_qty, OrderQty(5));

        lifecycle
            .plan_child(5, 103, id("c2"), id("client-2"), OrderQty(5), instruction())
            .unwrap();
        lifecycle.mark_submitted(6, 104, id("c2")).unwrap();
        lifecycle
            .apply_execution_event(7, &event("client-2", OrderStatus::Filled, 5, 5, 102, 105))
            .unwrap();
        lifecycle
            .apply_execution_event(8, &event("client-1", OrderStatus::Filled, 5, 2, 103, 106))
            .unwrap();
        let snapshot = lifecycle.snapshot();
        assert_eq!(snapshot.state, OrderIntentState::Completed);
        assert_eq!(snapshot.filled_qty, OrderQty(10));
        assert_eq!(snapshot.leaves_qty, OrderQty(0));
        assert_eq!(snapshot.average_fill_price, OrderPrice(101));
    }

    #[test]
    fn pause_resume_and_limits_gate_new_children() {
        let mut lifecycle = OrderIntentLifecycle::new(intent(1), 3).unwrap();
        lifecycle.activate(1, 60).unwrap();
        lifecycle.pause(2, 70).unwrap();
        assert!(matches!(
            lifecycle.plan_child(3, 100, id("c1"), id("client-1"), OrderQty(1), instruction()),
            Err(OrderIntentError::InvalidTransition)
        ));
        lifecycle.resume(3, 80).unwrap();
        lifecycle
            .plan_child(4, 100, id("c1"), id("client-1"), OrderQty(6), instruction())
            .unwrap();
        assert!(matches!(
            lifecycle.plan_child(5, 101, id("c2"), id("client-2"), OrderQty(1), instruction()),
            Err(OrderIntentError::OpenChildLimitExceeded)
        ));
        let expensive = ExecutionInstruction::new(
            id("route-a"),
            OrderType::Limit,
            TimeInForce::Day,
            OrderPrice(111),
        );
        assert!(matches!(
            lifecycle.replace_child(
                5,
                101,
                id("c1"),
                id("c2"),
                id("client-2"),
                OrderQty(1),
                expensive
            ),
            Err(OrderIntentError::InvalidTransition)
        ));
    }

    #[test]
    fn replace_lineage_releases_old_leaves_and_handles_late_fill() {
        let mut lifecycle = OrderIntentLifecycle::new(intent(1), 3).unwrap();
        lifecycle.activate(1, 60).unwrap();
        lifecycle
            .plan_child(
                2,
                100,
                id("old"),
                id("client-old"),
                OrderQty(6),
                instruction(),
            )
            .unwrap();
        lifecycle.mark_submitted(3, 101, id("old")).unwrap();
        let replacement = lifecycle
            .replace_child(
                4,
                102,
                id("old"),
                id("new"),
                id("client-new"),
                OrderQty(6),
                instruction(),
            )
            .unwrap();
        assert_eq!(replacement.replaces_child_id, Some(id("old")));
        assert_eq!(
            lifecycle.child(id("old")).unwrap().replaced_by_child_id,
            Some(id("new"))
        );
        lifecycle
            .apply_execution_event(
                5,
                &event("client-old", OrderStatus::PartiallyFilled, 2, 2, 100, 103),
            )
            .unwrap();
        assert_eq!(
            lifecycle.child(id("old")).unwrap().state,
            OmsChildOrderState::Replaced
        );
        assert_eq!(lifecycle.snapshot().working_qty, OrderQty(6));
        assert_eq!(lifecycle.snapshot().allocatable_qty, OrderQty(2));
    }

    #[test]
    fn cancel_tree_is_atomic_and_cancels_planned_children_locally() {
        let mut lifecycle = OrderIntentLifecycle::new(intent(2), 4).unwrap();
        lifecycle.activate(1, 60).unwrap();
        lifecycle
            .plan_child(
                2,
                100,
                id("planned"),
                id("client-p"),
                OrderQty(2),
                instruction(),
            )
            .unwrap();
        lifecycle
            .plan_child(
                3,
                101,
                id("working"),
                id("client-w"),
                OrderQty(2),
                instruction(),
            )
            .unwrap();
        lifecycle.mark_submitted(4, 102, id("working")).unwrap();
        let mut too_small = OmsChildCancelBuffer::with_capacity(0);
        assert!(matches!(
            lifecycle.request_cancel_tree(5, 103, &mut too_small),
            Err(OrderIntentError::CancelBufferFull)
        ));
        assert_eq!(lifecycle.snapshot().last_sequence, 4);
        let mut out = OmsChildCancelBuffer::with_capacity(1);
        let snapshot = lifecycle.request_cancel_tree(5, 103, &mut out).unwrap();
        assert_eq!(snapshot.state, OrderIntentState::PendingCancel);
        assert_eq!(out.as_slice().len(), 1);
        assert_eq!(out.as_slice()[0].child_id, id("working"));
        assert_eq!(
            lifecycle.child(id("planned")).unwrap().state,
            OmsChildOrderState::Cancelled
        );
        lifecycle
            .apply_execution_event(6, &event("client-w", OrderStatus::Cancelled, 0, 0, 0, 104))
            .unwrap();
        assert_eq!(lifecycle.snapshot().state, OrderIntentState::Cancelled);
        assert_eq!(lifecycle.snapshot().working_qty, OrderQty(0));
    }

    #[test]
    fn failed_parent_still_allows_emergency_child_cancellation() {
        let mut lifecycle = OrderIntentLifecycle::new(intent(1), 2).unwrap();
        lifecycle.activate(1, 60).unwrap();
        lifecycle
            .plan_child(
                2,
                100,
                id("working"),
                id("client-w"),
                OrderQty(2),
                instruction(),
            )
            .unwrap();
        lifecycle.mark_submitted(3, 101, id("working")).unwrap();
        lifecycle.fail(4, 102).unwrap();
        let mut out = OmsChildCancelBuffer::with_capacity(1);
        let snapshot = lifecycle.request_cancel_tree(5, 103, &mut out).unwrap();
        assert_eq!(snapshot.state, OrderIntentState::PendingCancel);
        assert_eq!(out.as_slice().len(), 1);
    }

    #[test]
    fn late_fill_after_cancel_is_counted_without_reopening_child() {
        let mut lifecycle = OrderIntentLifecycle::new(intent(1), 2).unwrap();
        lifecycle.activate(1, 60).unwrap();
        lifecycle
            .plan_child(
                2,
                100,
                id("child"),
                id("client"),
                OrderQty(2),
                instruction(),
            )
            .unwrap();
        lifecycle.mark_submitted(3, 101, id("child")).unwrap();
        let mut out = OmsChildCancelBuffer::with_capacity(1);
        lifecycle.request_cancel_tree(4, 102, &mut out).unwrap();
        lifecycle
            .apply_execution_event(5, &event("client", OrderStatus::Cancelled, 0, 0, 0, 103))
            .unwrap();
        lifecycle
            .apply_execution_event(
                6,
                &event("client", OrderStatus::PartiallyFilled, 1, 1, 100, 104),
            )
            .unwrap();
        assert_eq!(
            lifecycle.child(id("child")).unwrap().state,
            OmsChildOrderState::Cancelled
        );
        assert_eq!(lifecycle.snapshot().filled_qty, OrderQty(1));
        assert_eq!(lifecycle.snapshot().working_qty, OrderQty(0));
    }

    #[test]
    fn recovery_recomputes_aggregates_and_requires_reconciliation_activation() {
        let mut lifecycle = OrderIntentLifecycle::new(intent(2), 4).unwrap();
        lifecycle.activate(1, 60).unwrap();
        lifecycle
            .plan_child(2, 100, id("c1"), id("client-1"), OrderQty(2), instruction())
            .unwrap();
        let snapshot = lifecycle.recovery_snapshot();
        let mut restored = OrderIntentLifecycle::restore(&snapshot, 4).unwrap();
        assert_eq!(restored.snapshot().state, OrderIntentState::Recovering);
        assert!(matches!(
            restored.plan_child(3, 101, id("c2"), id("client-2"), OrderQty(1), instruction()),
            Err(OrderIntentError::InvalidTransition)
        ));
        restored.activate(3, 101).unwrap();
        assert_eq!(restored.snapshot().state, OrderIntentState::Active);

        let mut corrupt = snapshot.clone();
        corrupt.parent.filled_qty = OrderQty(1);
        assert!(matches!(
            OrderIntentLifecycle::restore(&corrupt, 4),
            Err(OrderIntentError::InvalidSnapshot)
        ));

        let mut corrupt = snapshot.clone();
        corrupt.children[0].replaces_child_id = Some(id("missing"));
        assert!(matches!(
            OrderIntentLifecycle::restore(&corrupt, 4),
            Err(OrderIntentError::InvalidSnapshot)
        ));

        let mut corrupt = snapshot;
        corrupt.children[0].instruction.display_qty = OrderQty(3);
        assert!(matches!(
            OrderIntentLifecycle::restore(&corrupt, 4),
            Err(OrderIntentError::InvalidSnapshot)
        ));
    }

    #[test]
    fn regressing_or_overfill_reports_leave_state_unchanged() {
        let mut lifecycle = OrderIntentLifecycle::new(intent(1), 2).unwrap();
        lifecycle.activate(1, 60).unwrap();
        lifecycle
            .plan_child(2, 100, id("c1"), id("client-1"), OrderQty(6), instruction())
            .unwrap();
        lifecycle.mark_submitted(3, 101, id("c1")).unwrap();
        lifecycle
            .apply_execution_event(
                4,
                &event("client-1", OrderStatus::PartiallyFilled, 3, 3, 100, 102),
            )
            .unwrap();
        assert!(matches!(
            lifecycle.apply_execution_event(
                5,
                &event("client-1", OrderStatus::PartiallyFilled, 2, 0, 100, 103)
            ),
            Err(OrderIntentError::InvalidExecutionProgress)
        ));
        assert_eq!(lifecycle.snapshot().last_sequence, 4);
        assert_eq!(lifecycle.snapshot().filled_qty, OrderQty(3));
    }
}
