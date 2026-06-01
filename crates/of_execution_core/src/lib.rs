//! Low-latency execution-domain primitives for Orderflow.
#![doc = include_str!("../README.md")]

use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Maximum bytes stored in an execution diagnostic text field.
pub const EXECUTION_TEXT_CAP: usize = 128;

/// Fixed-size ASCII field used for low-allocation identifiers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FixedAscii<const N: usize> {
    len: u8,
    bytes: [u8; N],
}

impl<const N: usize> FixedAscii<N> {
    /// Creates an empty fixed ASCII value.
    pub const fn empty() -> Self {
        Self {
            len: 0,
            bytes: [0; N],
        }
    }

    /// Creates a fixed ASCII value from `value`.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionCoreError::IdentifierTooLong`] when `value` exceeds
    /// the fixed capacity, or [`ExecutionCoreError::NonAsciiIdentifier`] when
    /// it contains non-ASCII bytes.
    pub fn new(value: &str) -> Result<Self, ExecutionCoreError> {
        if value.len() > N {
            return Err(ExecutionCoreError::IdentifierTooLong {
                capacity: N,
                actual: value.len(),
            });
        }
        if !value.is_ascii() {
            return Err(ExecutionCoreError::NonAsciiIdentifier);
        }

        let mut bytes = [0; N];
        bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(Self {
            len: value.len() as u8,
            bytes,
        })
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len as usize])
            .expect("FixedAscii stores only validated ASCII")
    }

    /// Returns the fixed field capacity in bytes.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Returns true when the identifier is empty.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> Default for FixedAscii<N> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<const N: usize> fmt::Debug for FixedAscii<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FixedAscii").field(&self.as_str()).finish()
    }
}

impl<const N: usize> fmt::Display for FixedAscii<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<const N: usize> PartialEq for FixedAscii<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<const N: usize> Eq for FixedAscii<N> {}

impl<const N: usize> Hash for FixedAscii<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

/// Client-assigned order identifier.
pub type ClientOrderId = FixedAscii<40>;
/// Venue-assigned order identifier.
pub type VenueOrderId = FixedAscii<48>;
/// Venue execution/fill identifier.
pub type ExecutionId = FixedAscii<48>;
/// Trading account identifier.
pub type AccountId = FixedAscii<32>;
/// Execution route identifier.
pub type RouteId = FixedAscii<32>;
/// Strategy identifier used for attribution.
pub type StrategyId = FixedAscii<32>;
/// Venue identifier used by execution routing.
pub type VenueId = FixedAscii<16>;
/// Instrument identifier in venue/native format.
pub type InstrumentId = FixedAscii<32>;
/// Bounded diagnostic text.
pub type ExecutionText = FixedAscii<EXECUTION_TEXT_CAP>;

/// Execution-core error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionCoreError {
    /// Fixed-size identifier capacity was exceeded.
    IdentifierTooLong {
        /// Configured capacity in bytes.
        capacity: usize,
        /// Actual input length in bytes.
        actual: usize,
    },
    /// Identifier contained a non-ASCII byte.
    NonAsciiIdentifier,
    /// Order quantity must be positive for the requested operation.
    InvalidQuantity,
    /// Price must be positive for price-bearing orders.
    InvalidPrice,
    /// State transition is not valid for the current order state.
    InvalidTransition,
}

impl fmt::Display for ExecutionCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentifierTooLong { capacity, actual } => {
                write!(f, "identifier length {actual} exceeds capacity {capacity}")
            }
            Self::NonAsciiIdentifier => write!(f, "identifier must be ASCII"),
            Self::InvalidQuantity => write!(f, "quantity must be positive"),
            Self::InvalidPrice => write!(f, "price must be positive"),
            Self::InvalidTransition => write!(f, "invalid order state transition"),
        }
    }
}

impl Error for ExecutionCoreError {}

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

/// Structured risk rejection reason.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiskRejectReason {
    /// No rejection.
    None = 0,
    /// Kill switch is active.
    KillSwitch = 1,
    /// Account is not enabled.
    AccountDisabled = 2,
    /// Route is not enabled.
    RouteDisabled = 3,
    /// Symbol is not enabled.
    SymbolDisabled = 4,
    /// Quantity exceeds configured max.
    MaxOrderQty = 5,
    /// Notional exceeds configured max.
    MaxOrderNotional = 6,
    /// Open order count exceeds configured max.
    MaxOpenOrders = 7,
    /// Open notional exceeds configured max.
    MaxOpenNotional = 8,
    /// Price is outside configured band.
    PriceBand = 9,
    /// Client order id is already in use.
    DuplicateClientOrderId = 10,
    /// Order type is unsupported on the route.
    UnsupportedOrderType = 11,
    /// Time-in-force is unsupported on the route.
    UnsupportedTimeInForce = 12,
}

/// Risk decision.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskDecision {
    /// True when the request is allowed to route.
    pub allowed: bool,
    /// Structured reject reason.
    pub reason: RiskRejectReason,
    /// Bounded diagnostic text.
    pub text: ExecutionText,
}

impl RiskDecision {
    /// Creates an allow decision.
    pub const fn allow() -> Self {
        Self {
            allowed: true,
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        }
    }

    /// Creates a reject decision.
    pub fn reject(reason: RiskRejectReason, text: ExecutionText) -> Self {
        Self {
            allowed: false,
            reason,
            text,
        }
    }
}

/// Static risk limits for one route/account scope.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskLimits {
    /// Kill switch flag.
    pub kill_switch: bool,
    /// Maximum quantity per order. Zero disables the check.
    pub max_order_qty: i64,
    /// Maximum notional per order. Zero disables the check.
    pub max_order_notional: i128,
    /// Maximum open orders. Zero disables the check.
    pub max_open_orders: u32,
    /// Maximum open notional. Zero disables the check.
    pub max_open_notional: i128,
    /// Allowed absolute distance from reference price. Zero disables the check.
    pub price_band_ticks: i64,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            kill_switch: true,
            max_order_qty: 0,
            max_order_notional: 0,
            max_open_orders: 0,
            max_open_notional: 0,
            price_band_ticks: 0,
        }
    }
}

/// Runtime risk context supplied by the execution engine.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskContext {
    /// Current open order count for the account/route scope.
    pub open_orders: u32,
    /// Current open notional for the account/route scope.
    pub open_notional: i128,
    /// Reference price used for price-band checks. Zero disables price-band checks.
    pub reference_price: OrderPrice,
    /// True when the client order id is already known.
    pub duplicate_client_order_id: bool,
    /// True when account is enabled.
    pub account_enabled: bool,
    /// True when route is enabled.
    pub route_enabled: bool,
    /// True when symbol is enabled.
    pub symbol_enabled: bool,
    /// True when order type is supported.
    pub order_type_supported: bool,
    /// True when time-in-force is supported.
    pub tif_supported: bool,
}

impl Default for RiskContext {
    fn default() -> Self {
        Self {
            open_orders: 0,
            open_notional: 0,
            reference_price: OrderPrice(0),
            duplicate_client_order_id: false,
            account_enabled: false,
            route_enabled: false,
            symbol_enabled: false,
            order_type_supported: false,
            tif_supported: false,
        }
    }
}

/// Pre-trade risk-check contract.
pub trait RiskCheck: Send + Sync {
    /// Checks a new order request.
    fn check_new(&self, req: &OrderRequest, ctx: &RiskContext) -> RiskDecision;
    /// Checks an amend request.
    fn check_amend(&self, req: &AmendRequest, ctx: &RiskContext) -> RiskDecision;
    /// Checks a cancel request.
    fn check_cancel(&self, req: &CancelRequest, ctx: &RiskContext) -> RiskDecision;
}

/// Deterministic pre-trade risk gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasicRiskGate {
    limits: RiskLimits,
}

impl BasicRiskGate {
    /// Creates a risk gate from static limits.
    pub const fn new(limits: RiskLimits) -> Self {
        Self { limits }
    }

    fn check_common(&self, ctx: &RiskContext) -> RiskDecision {
        if self.limits.kill_switch {
            return reject(RiskRejectReason::KillSwitch, "kill switch active");
        }
        if !ctx.account_enabled {
            return reject(RiskRejectReason::AccountDisabled, "account disabled");
        }
        if !ctx.route_enabled {
            return reject(RiskRejectReason::RouteDisabled, "route disabled");
        }
        if !ctx.symbol_enabled {
            return reject(RiskRejectReason::SymbolDisabled, "symbol disabled");
        }
        if ctx.duplicate_client_order_id {
            return reject(
                RiskRejectReason::DuplicateClientOrderId,
                "duplicate client order id",
            );
        }
        if !ctx.order_type_supported {
            return reject(
                RiskRejectReason::UnsupportedOrderType,
                "unsupported order type",
            );
        }
        if !ctx.tif_supported {
            return reject(
                RiskRejectReason::UnsupportedTimeInForce,
                "unsupported time in force",
            );
        }
        if self.limits.max_open_orders > 0 && ctx.open_orders >= self.limits.max_open_orders {
            return reject(RiskRejectReason::MaxOpenOrders, "max open orders exceeded");
        }
        if self.limits.max_open_notional > 0 && ctx.open_notional >= self.limits.max_open_notional {
            return reject(
                RiskRejectReason::MaxOpenNotional,
                "max open notional exceeded",
            );
        }
        RiskDecision::allow()
    }

    fn check_size_price(
        &self,
        qty: OrderQty,
        price: OrderPrice,
        ctx: &RiskContext,
    ) -> RiskDecision {
        if self.limits.max_order_qty > 0 && qty.0 > self.limits.max_order_qty {
            return reject(RiskRejectReason::MaxOrderQty, "max order quantity exceeded");
        }
        if self.limits.max_order_notional > 0 {
            let notional = i128::from(qty.0).saturating_mul(i128::from(price.0));
            if notional > self.limits.max_order_notional {
                return reject(
                    RiskRejectReason::MaxOrderNotional,
                    "max order notional exceeded",
                );
            }
        }
        if self.limits.price_band_ticks > 0 && ctx.reference_price.0 > 0 && price.0 > 0 {
            let distance = price.0.saturating_sub(ctx.reference_price.0).abs();
            if distance > self.limits.price_band_ticks {
                return reject(RiskRejectReason::PriceBand, "price outside risk band");
            }
        }
        RiskDecision::allow()
    }
}

impl RiskCheck for BasicRiskGate {
    fn check_new(&self, req: &OrderRequest, ctx: &RiskContext) -> RiskDecision {
        let common = self.check_common(ctx);
        if !common.allowed {
            return common;
        }
        self.check_size_price(req.quantity, req.limit_price, ctx)
    }

    fn check_amend(&self, req: &AmendRequest, ctx: &RiskContext) -> RiskDecision {
        let common = self.check_common(ctx);
        if !common.allowed {
            return common;
        }
        self.check_size_price(req.quantity, req.limit_price, ctx)
    }

    fn check_cancel(&self, _req: &CancelRequest, ctx: &RiskContext) -> RiskDecision {
        if self.limits.kill_switch {
            return reject(RiskRejectReason::KillSwitch, "kill switch active");
        }
        if !ctx.account_enabled {
            return reject(RiskRejectReason::AccountDisabled, "account disabled");
        }
        if !ctx.route_enabled {
            return reject(RiskRejectReason::RouteDisabled, "route disabled");
        }
        RiskDecision::allow()
    }
}

fn reject(reason: RiskRejectReason, text: &str) -> RiskDecision {
    let text = ExecutionText::new(text).unwrap_or_else(|_| ExecutionText::empty());
    RiskDecision::reject(reason, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn order_request() -> OrderRequest {
        OrderRequest {
            client_order_id: id("C1"),
            account_id: id("A1"),
            route_id: id("R1"),
            strategy_id: id("S1"),
            symbol: ExecutionSymbol::new("CME", "ESM6").unwrap(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(10),
            limit_price: OrderPrice(5000),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }
    }

    fn live_ctx() -> RiskContext {
        RiskContext {
            open_orders: 0,
            open_notional: 0,
            reference_price: OrderPrice(5000),
            duplicate_client_order_id: false,
            account_enabled: true,
            route_enabled: true,
            symbol_enabled: true,
            order_type_supported: true,
            tif_supported: true,
        }
    }

    #[test]
    fn fixed_ascii_rejects_invalid_input() {
        assert_eq!(
            ClientOrderId::new("abcdefghijklmnopqrstuvwxyz1234567890ABCDE").unwrap_err(),
            ExecutionCoreError::IdentifierTooLong {
                capacity: 40,
                actual: 41
            }
        );
        assert_eq!(
            ClientOrderId::new("ordé").unwrap_err(),
            ExecutionCoreError::NonAsciiIdentifier
        );
    }

    #[test]
    fn order_validation_requires_limit_price() {
        let mut req = order_request();
        req.limit_price = OrderPrice(0);
        assert_eq!(req.validate(), Err(ExecutionCoreError::InvalidPrice));
    }

    #[test]
    fn state_machine_accepts_and_fills_order() {
        let req = order_request();
        let mut sm = OrderStateMachine::new(&req);
        let ack = ExecutionEvent::accepted(&req, id("V1"));
        sm.apply(&ack).unwrap();
        assert_eq!(sm.state().status, OrderStatus::New);

        let mut fill = ack;
        fill.exec_type = ExecutionType::Trade;
        fill.order_status = OrderStatus::Filled;
        fill.execution_id = id("E1");
        fill.last_qty = OrderQty(10);
        fill.last_price = OrderPrice(5001);
        fill.cumulative_qty = OrderQty(10);
        fill.leaves_qty = OrderQty(0);
        fill.average_price = OrderPrice(5001);
        fill.ts_recv_ns = 3;
        sm.apply(&fill).unwrap();

        assert_eq!(sm.state().status, OrderStatus::Filled);
        assert_eq!(sm.state().cumulative_qty, OrderQty(10));
        assert!(sm.apply(&fill).is_err());
    }

    #[test]
    fn state_machine_handles_cancel_reject_as_status() {
        let req = order_request();
        let mut sm = OrderStateMachine::new(&req);
        sm.apply(&ExecutionEvent::accepted(&req, id("V1"))).unwrap();

        let mut pending_cancel = ExecutionEvent::accepted(&req, id("V1"));
        pending_cancel.exec_type = ExecutionType::CancelPending;
        pending_cancel.order_status = OrderStatus::PendingCancel;
        pending_cancel.ts_recv_ns = 4;
        sm.apply(&pending_cancel).unwrap();

        let mut reject = pending_cancel;
        reject.exec_type = ExecutionType::CancelReject;
        reject.order_status = OrderStatus::New;
        reject.ts_recv_ns = 5;
        sm.apply(&reject).unwrap();

        assert_eq!(sm.state().status, OrderStatus::New);
    }

    #[test]
    fn risk_gate_denies_by_default() {
        let req = order_request();
        let gate = BasicRiskGate::new(RiskLimits::default());
        let decision = gate.check_new(&req, &RiskContext::default());
        assert!(!decision.allowed);
        assert_eq!(decision.reason, RiskRejectReason::KillSwitch);
    }

    #[test]
    fn risk_gate_allows_configured_order() {
        let req = order_request();
        let gate = BasicRiskGate::new(RiskLimits {
            kill_switch: false,
            max_order_qty: 100,
            max_order_notional: 1_000_000,
            max_open_orders: 10,
            max_open_notional: 10_000_000,
            price_band_ticks: 10,
        });
        let decision = gate.check_new(&req, &live_ctx());
        assert!(decision.allowed);
    }

    #[test]
    fn risk_gate_rejects_price_band() {
        let mut req = order_request();
        req.limit_price = OrderPrice(5020);
        let gate = BasicRiskGate::new(RiskLimits {
            kill_switch: false,
            max_order_qty: 100,
            max_order_notional: 1_000_000,
            max_open_orders: 10,
            max_open_notional: 10_000_000,
            price_band_ticks: 10,
        });
        let decision = gate.check_new(&req, &live_ctx());
        assert!(!decision.allowed);
        assert_eq!(decision.reason, RiskRejectReason::PriceBand);
    }
}
