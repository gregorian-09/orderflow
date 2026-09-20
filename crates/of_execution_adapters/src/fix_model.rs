use super::*;

pub(crate) const ACCOUNT_TAG: FixTag = FixTag(1);
pub(crate) const CXL_REJ_REASON_TAG: FixTag = FixTag(102);
pub(crate) const CXL_REJ_RESPONSE_TO_TAG: FixTag = FixTag(434);

/// FIX sender/target configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixSessionConfig {
    /// FIX begin string, such as `FIX.4.4`.
    pub begin_string: FixedAscii<16>,
    /// SenderCompID.
    pub sender_comp_id: FixedAscii<32>,
    /// TargetCompID.
    pub target_comp_id: FixedAscii<32>,
    /// Heartbeat interval in seconds.
    pub heartbeat_secs: u16,
}

impl FixSessionConfig {
    /// Creates a FIX session config from ASCII fields.
    ///
    /// # Errors
    ///
    /// Returns an error if any field is non-ASCII or too long.
    pub fn new(
        begin_string: &str,
        sender_comp_id: &str,
        target_comp_id: &str,
        heartbeat_secs: u16,
    ) -> Result<Self, of_execution_core::ExecutionCoreError> {
        Ok(Self {
            begin_string: FixedAscii::new(begin_string)?,
            sender_comp_id: FixedAscii::new(sender_comp_id)?,
            target_comp_id: FixedAscii::new(target_comp_id)?,
            heartbeat_secs,
        })
    }
}

/// Minimal FIX execution-report payload after transport parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixExecutionReport {
    /// ExecType value.
    pub exec_type: FixExecType,
    /// OrdStatus value.
    pub ord_status: FixOrdStatus,
    /// ClOrdID.
    pub cl_ord_id: ClientOrderId,
    /// OrigClOrdID.
    pub orig_cl_ord_id: ClientOrderId,
    /// OrderID.
    pub order_id: VenueOrderId,
    /// ExecID.
    pub exec_id: ExecutionId,
    /// Account.
    pub account_id: AccountId,
    /// Route id associated with the session.
    pub route_id: RouteId,
    /// Symbol.
    pub symbol: ExecutionSymbol,
    /// LastQty.
    pub last_qty: OrderQty,
    /// LastPx.
    pub last_price: OrderPrice,
    /// CumQty.
    pub cumulative_qty: OrderQty,
    /// LeavesQty.
    pub leaves_qty: OrderQty,
    /// AvgPx.
    pub average_price: OrderPrice,
    /// TransactTime in nanoseconds when available.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
    /// Text.
    pub text: ExecutionText,
}

/// Minimal FIX OrderCancelReject payload after transport parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixOrderCancelReject {
    /// Cancel-reject response target.
    pub response_to: FixCancelRejectResponseTo,
    /// Current order status reported by the venue.
    pub ord_status: FixOrdStatus,
    /// ClOrdID of the rejected cancel or replace request.
    pub cl_ord_id: ClientOrderId,
    /// OrigClOrdID identifying the order that was being cancelled/replaced.
    pub orig_cl_ord_id: ClientOrderId,
    /// Venue order id when provided.
    pub order_id: VenueOrderId,
    /// Account.
    pub account_id: AccountId,
    /// Route id associated with the session.
    pub route_id: RouteId,
    /// Symbol when provided by the counterparty.
    pub symbol: ExecutionSymbol,
    /// Raw CxlRejReason(102) value when provided.
    pub cxl_rej_reason: u64,
    /// TransactTime in nanoseconds when available.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
    /// Text.
    pub text: ExecutionText,
}

/// Context required to map raw FIX execution reports into canonical OMS fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixReportParseConfig {
    /// Default account used when `Account(1)` is absent.
    pub account_id: AccountId,
    /// Route associated with this FIX session.
    pub route_id: RouteId,
    /// Venue/exchange identifier assigned to parsed `Symbol(55)` values.
    pub venue: VenueId,
    /// Decimal scale for quantity fields. For example, `100` maps `1.25` to
    /// `OrderQty(125)`.
    pub quantity_scale: i64,
    /// Decimal scale for price fields. For example, `10` maps `65000.5` to
    /// `OrderPrice(650005)`.
    pub price_scale: i64,
}

impl FixReportParseConfig {
    /// Creates a parse config with unit quantity and price scales.
    pub const fn new(account_id: AccountId, route_id: RouteId, venue: VenueId) -> Self {
        Self {
            account_id,
            route_id,
            venue,
            quantity_scale: 1,
            price_scale: 1,
        }
    }

    /// Sets the quantity scale. Values lower than one are clamped to one.
    pub const fn with_quantity_scale(mut self, quantity_scale: i64) -> Self {
        self.quantity_scale = if quantity_scale < 1 {
            1
        } else {
            quantity_scale
        };
        self
    }

    /// Sets the price scale. Values lower than one are clamped to one.
    pub const fn with_price_scale(mut self, price_scale: i64) -> Self {
        self.price_scale = if price_scale < 1 { 1 } else { price_scale };
        self
    }
}

/// Context required to encode canonical OMS requests as FIX order-entry frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixRequestEncodeConfig {
    /// Decimal scale for quantity fields. For example, `100` maps
    /// `OrderQty(125)` to `1.25`.
    pub quantity_scale: i64,
    /// Decimal scale for price fields. For example, `10` maps
    /// `OrderPrice(650005)` to `65000.5`.
    pub price_scale: i64,
}

impl FixRequestEncodeConfig {
    /// Creates a request encode config with unit quantity and price scales.
    pub const fn new() -> Self {
        Self {
            quantity_scale: 1,
            price_scale: 1,
        }
    }

    /// Sets the quantity scale. Values lower than one are clamped to one.
    pub const fn with_quantity_scale(mut self, quantity_scale: i64) -> Self {
        self.quantity_scale = if quantity_scale < 1 {
            1
        } else {
            quantity_scale
        };
        self
    }

    /// Sets the price scale. Values lower than one are clamped to one.
    pub const fn with_price_scale(mut self, price_scale: i64) -> Self {
        self.price_scale = if price_scale < 1 { 1 } else { price_scale };
        self
    }
}

impl Default for FixRequestEncodeConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Extra fields required to encode a canonical cancel request as FIX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixCancelEncodeContext<'a> {
    /// Side of the original order.
    pub side: OrderSide,
    /// FIX wire-format `TransactTime(60)` bytes.
    pub transact_time: &'a [u8],
}

impl<'a> FixCancelEncodeContext<'a> {
    /// Creates cancel encode context.
    pub const fn new(side: OrderSide, transact_time: &'a [u8]) -> Self {
        Self {
            side,
            transact_time,
        }
    }
}

/// Extra fields required to encode a canonical amend request as FIX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixAmendEncodeContext<'a> {
    /// Side of the original order.
    pub side: OrderSide,
    /// Replacement FIX order type.
    pub order_type: OrderType,
    /// Replacement time-in-force.
    pub time_in_force: TimeInForce,
    /// FIX wire-format `TransactTime(60)` bytes.
    pub transact_time: &'a [u8],
}

impl<'a> FixAmendEncodeContext<'a> {
    /// Creates amend encode context.
    pub const fn new(
        side: OrderSide,
        order_type: OrderType,
        time_in_force: TimeInForce,
        transact_time: &'a [u8],
    ) -> Self {
        Self {
            side,
            order_type,
            time_in_force,
            transact_time,
        }
    }
}

/// Extra fields required to encode a stop/stop-limit amend request as FIX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixStopAmendEncodeContext<'a> {
    /// Side of the original order.
    pub side: OrderSide,
    /// Replacement FIX order type. Must be stop or stop-limit.
    pub order_type: OrderType,
    /// Replacement time-in-force.
    pub time_in_force: TimeInForce,
    /// Replacement stop price.
    pub stop_price: OrderPrice,
    /// FIX wire-format `TransactTime(60)` bytes.
    pub transact_time: &'a [u8],
}

impl<'a> FixStopAmendEncodeContext<'a> {
    /// Creates stop-amend encode context.
    pub const fn new(
        side: OrderSide,
        order_type: OrderType,
        time_in_force: TimeInForce,
        stop_price: OrderPrice,
        transact_time: &'a [u8],
    ) -> Self {
        Self {
            side,
            order_type,
            time_in_force,
            stop_price,
            transact_time,
        }
    }
}

/// Errors returned while encoding canonical OMS requests as FIX frames.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixRequestEncodeError {
    /// Quantity scale or price scale is not a positive power of ten.
    InvalidScale,
    /// Quantity must be positive.
    InvalidQuantity,
    /// Price must be positive for this order type.
    InvalidPrice,
    /// The canonical order type needs fields this bridge does not encode yet.
    UnsupportedOrderType,
    /// The underlying FIX encoder rejected the frame.
    Encode(FixEncodeError),
}

impl fmt::Display for FixRequestEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScale => write!(f, "FIX request encode scale must be a power of ten"),
            Self::InvalidQuantity => write!(f, "FIX request quantity must be positive"),
            Self::InvalidPrice => write!(f, "FIX request price must be positive"),
            Self::UnsupportedOrderType => write!(
                f,
                "FIX request order type requires fields not encoded by this bridge"
            ),
            Self::Encode(source) => write!(f, "FIX request encode failed: {source}"),
        }
    }
}

impl Error for FixRequestEncodeError {}

impl From<FixEncodeError> for FixRequestEncodeError {
    fn from(source: FixEncodeError) -> Self {
        Self::Encode(source)
    }
}

/// Errors returned while converting a parsed FIX execution report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixReportParseError {
    /// The message is not `ExecutionReport(35=8)`.
    InvalidMsgType,
    /// A required tag is missing.
    MissingTag(FixTag),
    /// `ExecType(150)` is not supported by the mapper.
    InvalidExecType,
    /// `OrdStatus(39)` is not supported by the mapper.
    InvalidOrdStatus,
    /// `CxlRejResponseTo(434)` is not supported by the mapper.
    InvalidCancelRejectResponseTo,
    /// A fixed ASCII canonical field could not be built from this tag.
    InvalidAscii {
        /// Source FIX tag.
        tag: FixTag,
        /// Underlying execution-core validation error.
        source: ExecutionCoreError,
    },
    /// A numeric field could not be parsed or scaled.
    InvalidNumber(FixTag),
}

impl fmt::Display for FixReportParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMsgType => write!(f, "FIX message is not an ExecutionReport(35=8)"),
            Self::MissingTag(tag) => write!(f, "FIX execution report is missing tag {tag}"),
            Self::InvalidExecType => write!(f, "FIX ExecType(150) is unsupported"),
            Self::InvalidOrdStatus => write!(f, "FIX OrdStatus(39) is unsupported"),
            Self::InvalidCancelRejectResponseTo => {
                write!(f, "FIX CxlRejResponseTo(434) is unsupported")
            }
            Self::InvalidAscii { tag, source } => {
                write!(
                    f,
                    "FIX tag {tag} cannot be converted to fixed ASCII: {source}"
                )
            }
            Self::InvalidNumber(tag) => write!(f, "FIX numeric tag {tag} is invalid"),
        }
    }
}

impl Error for FixReportParseError {}

/// FIX ExecType values normalized for mapping.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixExecType {
    /// New/accepted.
    New = 1,
    /// Rejected.
    Rejected = 2,
    /// Trade.
    Trade = 3,
    /// Pending cancel.
    PendingCancel = 4,
    /// Canceled.
    Canceled = 5,
    /// Pending replace.
    PendingReplace = 6,
    /// Replaced.
    Replaced = 7,
    /// Expired.
    Expired = 8,
    /// Restated/status.
    Restated = 9,
}

/// FIX OrdStatus values normalized for mapping.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixOrdStatus {
    /// New.
    New = 1,
    /// Partially filled.
    PartiallyFilled = 2,
    /// Filled.
    Filled = 3,
    /// Done for day.
    DoneForDay = 4,
    /// Canceled.
    Canceled = 5,
    /// Replaced.
    Replaced = 6,
    /// Pending cancel.
    PendingCancel = 7,
    /// Stopped.
    Stopped = 8,
    /// Rejected.
    Rejected = 9,
    /// Suspended.
    Suspended = 10,
    /// Pending new.
    PendingNew = 11,
    /// Expired.
    Expired = 12,
    /// Pending replace.
    PendingReplace = 13,
}

/// FIX CxlRejResponseTo values normalized for mapping.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixCancelRejectResponseTo {
    /// Response to OrderCancelRequest `<F>`.
    OrderCancelRequest = 1,
    /// Response to OrderCancelReplaceRequest `<G>`.
    OrderCancelReplaceRequest = 2,
}
