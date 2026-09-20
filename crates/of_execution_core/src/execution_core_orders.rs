use super::*;

/// Execution symbol in venue-native format.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ExecutionSymbol {
    /// Venue/exchange identifier.
    pub venue: VenueId,
    /// Venue-native instrument symbol.
    pub instrument: InstrumentId,
}

impl ExecutionSymbol {
    /// Creates a symbol from ASCII venue and instrument identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error when either identifier is non-ASCII or too long.
    pub fn new(venue: &str, instrument: &str) -> Result<Self, ExecutionCoreError> {
        Ok(Self {
            venue: VenueId::new(venue)?,
            instrument: InstrumentId::new(instrument)?,
        })
    }
}

/// Integer-normalized order quantity.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct OrderQty(pub i64);

impl OrderQty {
    /// Creates a positive order quantity.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionCoreError::InvalidQuantity`] if `value <= 0`.
    pub fn new(value: i64) -> Result<Self, ExecutionCoreError> {
        if value <= 0 {
            return Err(ExecutionCoreError::InvalidQuantity);
        }
        Ok(Self(value))
    }
}

/// Integer-normalized order price.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct OrderPrice(pub i64);

impl OrderPrice {
    /// Creates a positive order price.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionCoreError::InvalidPrice`] if `value <= 0`.
    pub fn new(value: i64) -> Result<Self, ExecutionCoreError> {
        if value <= 0 {
            return Err(ExecutionCoreError::InvalidPrice);
        }
        Ok(Self(value))
    }
}

/// Buy/sell order side.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderSide {
    /// Buy order.
    Buy = 1,
    /// Sell order.
    Sell = 2,
}

/// Supported canonical order types.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderType {
    /// Market order.
    Market = 1,
    /// Limit order.
    Limit = 2,
    /// Stop order.
    Stop = 3,
    /// Stop-limit order.
    StopLimit = 4,
}

/// Time-in-force policy.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeInForce {
    /// Day order.
    Day = 1,
    /// Good-till-cancelled order.
    Gtc = 2,
    /// Immediate-or-cancel order.
    Ioc = 3,
    /// Fill-or-kill order.
    Fok = 4,
    /// Good-till-date order.
    Gtd = 5,
}

/// FIX-style canonical order status.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    /// Local request is pending submission/acknowledgement.
    PendingNew = 1,
    /// Venue accepted the live order.
    New = 2,
    /// Order has at least one fill and remaining leaves quantity.
    PartiallyFilled = 3,
    /// Order is fully filled.
    Filled = 4,
    /// Cancel request is pending.
    PendingCancel = 5,
    /// Order is cancelled.
    Cancelled = 6,
    /// Cancel/replace request is pending.
    PendingReplace = 7,
    /// Order was replaced.
    Replaced = 8,
    /// Order was rejected.
    Rejected = 9,
    /// Order expired.
    Expired = 10,
    /// Order is suspended.
    Suspended = 11,
    /// Order state could not be reconciled.
    Unknown = 12,
}

impl OrderStatus {
    /// Returns true when no further venue activity is expected.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Filled | Self::Cancelled | Self::Rejected | Self::Expired
        )
    }
}

/// Canonical execution report purpose.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionType {
    /// Order accepted.
    Ack = 1,
    /// Order rejected.
    Reject = 2,
    /// Trade/fill report.
    Trade = 3,
    /// Cancel request accepted or pending.
    CancelPending = 4,
    /// Cancel completed.
    CancelAck = 5,
    /// Cancel request rejected.
    CancelReject = 6,
    /// Replace request accepted or pending.
    ReplacePending = 7,
    /// Replace completed.
    ReplaceAck = 8,
    /// Replace request rejected.
    ReplaceReject = 9,
    /// Order expired.
    Expire = 10,
    /// Status-only report.
    Status = 11,
    /// Recovered or restated state.
    Restated = 12,
    /// Adapter degradation report.
    AdapterDegraded = 13,
}

/// New order request.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderRequest {
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Trading account.
    pub account_id: AccountId,
    /// Execution route.
    pub route_id: RouteId,
    /// Strategy attribution id.
    pub strategy_id: StrategyId,
    /// Target symbol.
    pub symbol: ExecutionSymbol,
    /// Order side.
    pub side: OrderSide,
    /// Order type.
    pub order_type: OrderType,
    /// Time-in-force.
    pub time_in_force: TimeInForce,
    /// Requested quantity.
    pub quantity: OrderQty,
    /// Limit price, or zero for orders without limit price.
    pub limit_price: OrderPrice,
    /// Stop price, or zero for orders without stop price.
    pub stop_price: OrderPrice,
    /// Exchange/session timestamp in nanoseconds when known.
    pub ts_exchange_ns: u64,
    /// Local receive/create timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

impl OrderRequest {
    /// Validates basic order shape.
    ///
    /// # Errors
    ///
    /// Returns invalid quantity/price errors when required fields are not
    /// positive for the selected order type.
    pub fn validate(&self) -> Result<(), ExecutionCoreError> {
        if self.quantity.0 <= 0 {
            return Err(ExecutionCoreError::InvalidQuantity);
        }
        match self.order_type {
            OrderType::Limit | OrderType::StopLimit if self.limit_price.0 <= 0 => {
                Err(ExecutionCoreError::InvalidPrice)
            }
            OrderType::Stop | OrderType::StopLimit if self.stop_price.0 <= 0 => {
                Err(ExecutionCoreError::InvalidPrice)
            }
            _ => Ok(()),
        }
    }
}

/// Cancel request.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelRequest {
    /// New client id for the cancel request.
    pub client_order_id: ClientOrderId,
    /// Last accepted client order id being cancelled.
    pub orig_client_order_id: ClientOrderId,
    /// Venue order id when known.
    pub venue_order_id: VenueOrderId,
    /// Trading account.
    pub account_id: AccountId,
    /// Execution route.
    pub route_id: RouteId,
    /// Target symbol.
    pub symbol: ExecutionSymbol,
    /// Local request timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// Amend/cancel-replace request.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmendRequest {
    /// New client id for the replacement request.
    pub client_order_id: ClientOrderId,
    /// Last accepted client order id being replaced.
    pub orig_client_order_id: ClientOrderId,
    /// Venue order id when known.
    pub venue_order_id: VenueOrderId,
    /// Trading account.
    pub account_id: AccountId,
    /// Execution route.
    pub route_id: RouteId,
    /// Target symbol.
    pub symbol: ExecutionSymbol,
    /// Replacement quantity.
    pub quantity: OrderQty,
    /// Replacement limit price.
    pub limit_price: OrderPrice,
    /// Local request timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// Canonical execution event.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionEvent {
    /// Execution report type.
    pub exec_type: ExecutionType,
    /// Current order status after applying the event.
    pub order_status: OrderStatus,
    /// Current client order id.
    pub client_order_id: ClientOrderId,
    /// Original client order id for cancel/replace flows.
    pub orig_client_order_id: ClientOrderId,
    /// Venue order id.
    pub venue_order_id: VenueOrderId,
    /// Execution/fill id.
    pub execution_id: ExecutionId,
    /// Account id.
    pub account_id: AccountId,
    /// Route id.
    pub route_id: RouteId,
    /// Symbol.
    pub symbol: ExecutionSymbol,
    /// Last fill quantity.
    pub last_qty: OrderQty,
    /// Last fill price.
    pub last_price: OrderPrice,
    /// Cumulative filled quantity.
    pub cumulative_qty: OrderQty,
    /// Remaining quantity.
    pub leaves_qty: OrderQty,
    /// Average fill price.
    pub average_price: OrderPrice,
    /// Exchange/session timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
    /// Structured rejection/degradation reason.
    pub reason: RiskRejectReason,
    /// Bounded diagnostic text.
    pub text: ExecutionText,
}

impl ExecutionEvent {
    /// Creates an accepted event from a new order request.
    pub fn accepted(req: &OrderRequest, venue_order_id: VenueOrderId) -> Self {
        Self {
            exec_type: ExecutionType::Ack,
            order_status: OrderStatus::New,
            client_order_id: req.client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id,
            execution_id: ExecutionId::empty(),
            account_id: req.account_id,
            route_id: req.route_id,
            symbol: req.symbol,
            last_qty: OrderQty(0),
            last_price: OrderPrice(0),
            cumulative_qty: OrderQty(0),
            leaves_qty: req.quantity,
            average_price: OrderPrice(0),
            ts_exchange_ns: req.ts_exchange_ns,
            ts_recv_ns: req.ts_recv_ns,
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        }
    }

    /// Creates a structured local rejection event from a request.
    pub fn rejected(req: &OrderRequest, reason: RiskRejectReason, text: ExecutionText) -> Self {
        Self {
            exec_type: ExecutionType::Reject,
            order_status: OrderStatus::Rejected,
            client_order_id: req.client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: VenueOrderId::empty(),
            execution_id: ExecutionId::empty(),
            account_id: req.account_id,
            route_id: req.route_id,
            symbol: req.symbol,
            last_qty: OrderQty(0),
            last_price: OrderPrice(0),
            cumulative_qty: OrderQty(0),
            leaves_qty: req.quantity,
            average_price: OrderPrice(0),
            ts_exchange_ns: req.ts_exchange_ns,
            ts_recv_ns: req.ts_recv_ns,
            reason,
            text,
        }
    }
}

/// Current order state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderState {
    /// Current client order id.
    pub client_order_id: ClientOrderId,
    /// Last accepted client order id.
    pub last_accepted_client_order_id: ClientOrderId,
    /// Venue order id.
    pub venue_order_id: VenueOrderId,
    /// Account id.
    pub account_id: AccountId,
    /// Route id.
    pub route_id: RouteId,
    /// Symbol.
    pub symbol: ExecutionSymbol,
    /// Side.
    pub side: OrderSide,
    /// Current order status.
    pub status: OrderStatus,
    /// Original order quantity.
    pub order_qty: OrderQty,
    /// Cumulative filled quantity.
    pub cumulative_qty: OrderQty,
    /// Remaining quantity.
    pub leaves_qty: OrderQty,
    /// Average fill price.
    pub average_price: OrderPrice,
    /// Last state update timestamp in nanoseconds.
    pub updated_ns: u64,
}

impl OrderState {
    /// Creates local pending-new state from a request.
    pub fn pending_new(req: &OrderRequest) -> Self {
        Self {
            client_order_id: req.client_order_id,
            last_accepted_client_order_id: req.client_order_id,
            venue_order_id: VenueOrderId::empty(),
            account_id: req.account_id,
            route_id: req.route_id,
            symbol: req.symbol,
            side: req.side,
            status: OrderStatus::PendingNew,
            order_qty: req.quantity,
            cumulative_qty: OrderQty(0),
            leaves_qty: req.quantity,
            average_price: OrderPrice(0),
            updated_ns: req.ts_recv_ns,
        }
    }
}

/// Deterministic order state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderStateMachine {
    state: OrderState,
}

impl OrderStateMachine {
    /// Creates a state machine from an order request.
    pub fn new(req: &OrderRequest) -> Self {
        Self {
            state: OrderState::pending_new(req),
        }
    }

    /// Returns the current order state.
    pub const fn state(&self) -> &OrderState {
        &self.state
    }

    /// Applies an execution event.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionCoreError::InvalidTransition`] when the event cannot
    /// legally apply to the current state.
    pub fn apply(&mut self, event: &ExecutionEvent) -> Result<(), ExecutionCoreError> {
        if self.state.status.is_terminal() && event.exec_type != ExecutionType::Status {
            return Err(ExecutionCoreError::InvalidTransition);
        }

        match event.exec_type {
            ExecutionType::Ack => self.apply_ack(event),
            ExecutionType::Reject => self.apply_terminal(event, OrderStatus::Rejected),
            ExecutionType::Trade => self.apply_trade(event),
            ExecutionType::CancelPending => self.apply_pending(event, OrderStatus::PendingCancel),
            ExecutionType::CancelAck => self.apply_terminal(event, OrderStatus::Cancelled),
            ExecutionType::CancelReject => self.apply_status(event),
            ExecutionType::ReplacePending => self.apply_pending(event, OrderStatus::PendingReplace),
            ExecutionType::ReplaceAck => self.apply_replace(event),
            ExecutionType::ReplaceReject => self.apply_status(event),
            ExecutionType::Expire => self.apply_terminal(event, OrderStatus::Expired),
            ExecutionType::Status | ExecutionType::Restated | ExecutionType::AdapterDegraded => {
                self.apply_status(event)
            }
        }
    }

    fn apply_ack(&mut self, event: &ExecutionEvent) -> Result<(), ExecutionCoreError> {
        if self.state.status != OrderStatus::PendingNew {
            return Err(ExecutionCoreError::InvalidTransition);
        }
        self.state.status = OrderStatus::New;
        self.state.venue_order_id = event.venue_order_id;
        self.state.leaves_qty = event.leaves_qty;
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }

    fn apply_trade(&mut self, event: &ExecutionEvent) -> Result<(), ExecutionCoreError> {
        if event.cumulative_qty.0 > self.state.order_qty.0 {
            return Err(ExecutionCoreError::InvalidTransition);
        }
        self.state.cumulative_qty = event.cumulative_qty;
        self.state.leaves_qty = event.leaves_qty;
        self.state.average_price = event.average_price;
        self.state.status = if event.leaves_qty.0 == 0 {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }

    fn apply_pending(
        &mut self,
        event: &ExecutionEvent,
        status: OrderStatus,
    ) -> Result<(), ExecutionCoreError> {
        if matches!(
            self.state.status,
            OrderStatus::PendingNew | OrderStatus::PendingCancel | OrderStatus::PendingReplace
        ) {
            return Err(ExecutionCoreError::InvalidTransition);
        }
        self.state.status = status;
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }

    fn apply_replace(&mut self, event: &ExecutionEvent) -> Result<(), ExecutionCoreError> {
        if self.state.status != OrderStatus::PendingReplace {
            return Err(ExecutionCoreError::InvalidTransition);
        }
        self.state.client_order_id = event.client_order_id;
        self.state.last_accepted_client_order_id = event.client_order_id;
        self.state.status = OrderStatus::Replaced;
        self.state.order_qty = OrderQty(event.cumulative_qty.0 + event.leaves_qty.0);
        self.state.cumulative_qty = event.cumulative_qty;
        self.state.leaves_qty = event.leaves_qty;
        self.state.average_price = event.average_price;
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }

    fn apply_terminal(
        &mut self,
        event: &ExecutionEvent,
        status: OrderStatus,
    ) -> Result<(), ExecutionCoreError> {
        self.state.status = status;
        self.state.cumulative_qty = event.cumulative_qty;
        self.state.leaves_qty = event.leaves_qty;
        self.state.average_price = event.average_price;
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }

    fn apply_status(&mut self, event: &ExecutionEvent) -> Result<(), ExecutionCoreError> {
        if event.order_status == OrderStatus::Unknown {
            self.state.status = OrderStatus::Unknown;
        } else if event.order_status == OrderStatus::Suspended {
            self.state.status = OrderStatus::Suspended;
        } else if matches!(
            event.exec_type,
            ExecutionType::CancelReject | ExecutionType::ReplaceReject
        ) {
            self.state.status = event.order_status;
        }
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }
}
