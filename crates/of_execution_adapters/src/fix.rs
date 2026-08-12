//! FIX execution adapters, transport contracts, and report mapping.

mod certification;
mod live;

pub use certification::*;
pub use live::*;

use of_execution::{
    ExecutionAdapter, ExecutionCapabilities, ExecutionError, ExecutionEventBuffer, ExecutionHealth,
    ExecutionResult, LatencyClass,
};
use of_execution_core::{
    AccountId, AmendRequest, CancelRequest, ClientOrderId, ExecutionCoreError, ExecutionEvent,
    ExecutionId, ExecutionSymbol, ExecutionText, ExecutionType, FixedAscii, InstrumentId,
    OrderPrice, OrderQty, OrderRequest, OrderSide, OrderStatus, OrderType, RiskRejectReason,
    RouteId, TimeInForce, VenueId, VenueOrderId,
};
use of_fix::{
    encode_new_order_single, encode_order_cancel_replace_request, encode_order_cancel_request,
    FixEncodeError, FixMessageView, FixMsgType, FixNewOrderSingle, FixOrdType,
    FixOrderCancelReplaceRequest, FixOrderCancelRequest, FixOrderSide, FixSessionHeader, FixTag,
    FixTimeInForce, FixVersion,
};
use std::error::Error;
use std::fmt;

const ACCOUNT_TAG: FixTag = FixTag(1);
const CXL_REJ_REASON_TAG: FixTag = FixTag(102);
const CXL_REJ_RESPONSE_TO_TAG: FixTag = FixTag(434);

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

/// Parses a validated FIX `ExecutionReport(35=8)` into a normalized report.
///
/// The mapper is deliberately profile-aware through `config`: quantity and
/// price fields are scaled into Orderflow's integer-normalized OMS types using
/// caller-provided scale factors.
///
/// # Errors
///
/// Returns [`FixReportParseError`] when required fields are absent, enum values
/// are unsupported, ASCII identifiers cannot fit their canonical bounds, or
/// decimal fields cannot be represented with the configured scale.
pub fn parse_execution_report(
    message: &FixMessageView<'_>,
    config: FixReportParseConfig,
    ts_recv_ns: u64,
) -> Result<FixExecutionReport, FixReportParseError> {
    if message.msg_type() != Some(FixMsgType::EXECUTION_REPORT.as_bytes()) {
        return Err(FixReportParseError::InvalidMsgType);
    }

    let exec_type = parse_exec_type(required(message, FixTag::EXEC_TYPE)?)?;
    let ord_status = parse_ord_status(required(message, FixTag::ORD_STATUS)?)?;
    let cl_ord_id = fixed_required(message, FixTag::CL_ORD_ID)?;
    let orig_cl_ord_id = fixed_optional(message, FixTag::ORIG_CL_ORD_ID)?;
    let order_id = fixed_required(message, FixTag::ORDER_ID)?;
    let exec_id = fixed_required(message, FixTag::EXEC_ID)?;
    let account_id = if let Some(account) = message.get(ACCOUNT_TAG) {
        fixed_from_bytes(ACCOUNT_TAG, account)?
    } else {
        config.account_id
    };
    let instrument: InstrumentId = fixed_required(message, FixTag::SYMBOL)?;

    Ok(FixExecutionReport {
        exec_type,
        ord_status,
        cl_ord_id,
        orig_cl_ord_id,
        order_id,
        exec_id,
        account_id,
        route_id: config.route_id,
        symbol: ExecutionSymbol {
            venue: config.venue,
            instrument,
        },
        last_qty: OrderQty(parse_optional_scaled(
            message,
            FixTag::LAST_QTY,
            config.quantity_scale,
        )?),
        last_price: OrderPrice(parse_optional_scaled(
            message,
            FixTag::LAST_PX,
            config.price_scale,
        )?),
        cumulative_qty: OrderQty(parse_optional_scaled(
            message,
            FixTag::CUM_QTY,
            config.quantity_scale,
        )?),
        leaves_qty: OrderQty(parse_optional_scaled(
            message,
            FixTag::LEAVES_QTY,
            config.quantity_scale,
        )?),
        average_price: OrderPrice(parse_optional_scaled(
            message,
            FixTag::AVG_PX,
            config.price_scale,
        )?),
        ts_exchange_ns: parse_optional_u64(message, FixTag::TRANSACT_TIME)?,
        ts_recv_ns,
        text: fixed_optional(message, FixTag::TEXT)?,
    })
}

/// Parses a validated FIX `OrderCancelReject(35=9)` into a normalized report.
///
/// # Errors
///
/// Returns [`FixReportParseError`] when required fields are absent, enum values
/// are unsupported, ASCII identifiers cannot fit their canonical bounds, or
/// numeric fields cannot be parsed.
pub fn parse_order_cancel_reject(
    message: &FixMessageView<'_>,
    config: FixReportParseConfig,
    ts_recv_ns: u64,
) -> Result<FixOrderCancelReject, FixReportParseError> {
    if message.msg_type() != Some(FixMsgType::ORDER_CANCEL_REJECT.as_bytes()) {
        return Err(FixReportParseError::InvalidMsgType);
    }

    let response_to = parse_cancel_reject_response_to(required(message, CXL_REJ_RESPONSE_TO_TAG)?)?;
    let ord_status = parse_ord_status(required(message, FixTag::ORD_STATUS)?)?;
    let cl_ord_id = fixed_required(message, FixTag::CL_ORD_ID)?;
    let orig_cl_ord_id = fixed_required(message, FixTag::ORIG_CL_ORD_ID)?;
    let account_id = if let Some(account) = message.get(ACCOUNT_TAG) {
        fixed_from_bytes(ACCOUNT_TAG, account)?
    } else {
        config.account_id
    };
    let instrument: InstrumentId = fixed_optional(message, FixTag::SYMBOL)?;

    Ok(FixOrderCancelReject {
        response_to,
        ord_status,
        cl_ord_id,
        orig_cl_ord_id,
        order_id: fixed_optional(message, FixTag::ORDER_ID)?,
        account_id,
        route_id: config.route_id,
        symbol: ExecutionSymbol {
            venue: config.venue,
            instrument,
        },
        cxl_rej_reason: parse_optional_u64(message, CXL_REJ_REASON_TAG)?,
        ts_exchange_ns: parse_optional_u64(message, FixTag::TRANSACT_TIME)?,
        ts_recv_ns,
        text: fixed_optional(message, FixTag::TEXT)?,
    })
}

/// Encodes a canonical new-order request as FIX NewOrderSingle `<D>`.
///
/// `transact_time` must already be in the venue/profile's accepted FIX wire
/// format. The bridge scales integer-normalized quantity and price fields into
/// decimal ASCII using `config`.
///
/// # Errors
///
/// Returns [`FixRequestEncodeError`] when scales are invalid, required
/// quantity/price fields are not positive, the order type needs unsupported
/// fields, or the underlying FIX encoder rejects a field value.
pub fn encode_order_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    config: FixRequestEncodeConfig,
    request: &OrderRequest,
    transact_time: &[u8],
) -> Result<(), FixRequestEncodeError> {
    let mut qty_buf = [0u8; 40];
    let qty = encode_scaled(
        &mut qty_buf,
        request.quantity.0,
        config.quantity_scale,
        ScaledField::Quantity,
    )?;
    let mut price_buf = [0u8; 40];
    let price = encode_order_price(
        &mut price_buf,
        request.order_type,
        request.limit_price,
        config,
    )?;
    let mut stop_px_buf = [0u8; 40];
    let stop_px = encode_order_stop_price(
        &mut stop_px_buf,
        request.order_type,
        request.stop_price,
        config,
    )?;
    let mut fix_request = FixNewOrderSingle::new(
        request.client_order_id.as_str().as_bytes(),
        request.symbol.instrument.as_str().as_bytes(),
        map_side_to_fix(request.side),
        transact_time,
        qty,
        map_order_type_to_fix(request.order_type)?,
    )
    .with_account(request.account_id.as_str().as_bytes())
    .with_time_in_force(map_tif_to_fix(request.time_in_force));
    if let Some(price) = price {
        fix_request = fix_request.with_price(price);
    }
    if let Some(stop_px) = stop_px {
        fix_request = fix_request.with_stop_px(stop_px);
    }
    encode_new_order_single(out, version, header, fix_request)?;
    Ok(())
}

/// Encodes a canonical cancel request as FIX OrderCancelRequest `<F>`.
///
/// Canonical cancel requests do not carry side, so callers must supply the side
/// from local order state or venue profile context.
///
/// # Errors
///
/// Returns [`FixRequestEncodeError`] when the underlying FIX encoder rejects a
/// field value.
pub fn encode_cancel_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    request: &CancelRequest,
    context: FixCancelEncodeContext<'_>,
) -> Result<(), FixRequestEncodeError> {
    let fix_request = FixOrderCancelRequest::new(
        request.orig_client_order_id.as_str().as_bytes(),
        request.client_order_id.as_str().as_bytes(),
        request.symbol.instrument.as_str().as_bytes(),
        map_side_to_fix(context.side),
        context.transact_time,
    )
    .with_account(request.account_id.as_str().as_bytes());
    encode_order_cancel_request(out, version, header, fix_request)?;
    Ok(())
}

/// Encodes a canonical amend request as FIX OrderCancelReplaceRequest `<G>`.
///
/// Canonical amend requests do not carry side, order type, or TIF, so callers
/// must supply them from local order state or venue profile context.
///
/// # Errors
///
/// Returns [`FixRequestEncodeError`] when scales are invalid, required
/// quantity/price fields are not positive, the order type needs unsupported
/// fields, or the underlying FIX encoder rejects a field value.
pub fn encode_amend_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    config: FixRequestEncodeConfig,
    request: &AmendRequest,
    context: FixAmendEncodeContext<'_>,
) -> Result<(), FixRequestEncodeError> {
    let mut qty_buf = [0u8; 40];
    let qty = encode_scaled(
        &mut qty_buf,
        request.quantity.0,
        config.quantity_scale,
        ScaledField::Quantity,
    )?;
    let mut price_buf = [0u8; 40];
    let price = encode_order_price(
        &mut price_buf,
        context.order_type,
        request.limit_price,
        config,
    )?;
    if matches!(context.order_type, OrderType::Stop | OrderType::StopLimit) {
        return Err(FixRequestEncodeError::UnsupportedOrderType);
    }
    let mut fix_request = FixOrderCancelReplaceRequest::new(
        request.orig_client_order_id.as_str().as_bytes(),
        request.client_order_id.as_str().as_bytes(),
        request.symbol.instrument.as_str().as_bytes(),
        map_side_to_fix(context.side),
        context.transact_time,
        qty,
        map_order_type_to_fix(context.order_type)?,
    )
    .with_account(request.account_id.as_str().as_bytes())
    .with_time_in_force(map_tif_to_fix(context.time_in_force));
    if let Some(price) = price {
        fix_request = fix_request.with_price(price);
    }
    encode_order_cancel_replace_request(out, version, header, fix_request)?;
    Ok(())
}

/// Encodes a stop/stop-limit amend request as FIX OrderCancelReplaceRequest `<G>`.
///
/// This helper is separate from [`encode_amend_request`] because canonical
/// [`AmendRequest`] carries one replacement limit price but no separate stop
/// price. Callers supply the stop price explicitly through `context`.
///
/// # Errors
///
/// Returns [`FixRequestEncodeError`] when scales are invalid, required
/// quantity/price fields are not positive, `context.order_type` is not stop or
/// stop-limit, or the underlying FIX encoder rejects a field value.
pub fn encode_stop_amend_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    config: FixRequestEncodeConfig,
    request: &AmendRequest,
    context: FixStopAmendEncodeContext<'_>,
) -> Result<(), FixRequestEncodeError> {
    if !matches!(context.order_type, OrderType::Stop | OrderType::StopLimit) {
        return Err(FixRequestEncodeError::UnsupportedOrderType);
    }
    let mut qty_buf = [0u8; 40];
    let qty = encode_scaled(
        &mut qty_buf,
        request.quantity.0,
        config.quantity_scale,
        ScaledField::Quantity,
    )?;
    let mut price_buf = [0u8; 40];
    let price = encode_order_price(
        &mut price_buf,
        context.order_type,
        request.limit_price,
        config,
    )?;
    let mut stop_px_buf = [0u8; 40];
    let stop_px = encode_order_stop_price(
        &mut stop_px_buf,
        context.order_type,
        context.stop_price,
        config,
    )?
    .ok_or(FixRequestEncodeError::InvalidPrice)?;
    let mut fix_request = FixOrderCancelReplaceRequest::new(
        request.orig_client_order_id.as_str().as_bytes(),
        request.client_order_id.as_str().as_bytes(),
        request.symbol.instrument.as_str().as_bytes(),
        map_side_to_fix(context.side),
        context.transact_time,
        qty,
        map_order_type_to_fix(context.order_type)?,
    )
    .with_account(request.account_id.as_str().as_bytes())
    .with_stop_px(stop_px)
    .with_time_in_force(map_tif_to_fix(context.time_in_force));
    if let Some(price) = price {
        fix_request = fix_request.with_price(price);
    }
    encode_order_cancel_replace_request(out, version, header, fix_request)?;
    Ok(())
}

/// Maps a parsed FIX execution report into a canonical execution event.
pub fn map_execution_report(report: &FixExecutionReport) -> ExecutionEvent {
    ExecutionEvent {
        exec_type: map_exec_type(report.exec_type),
        order_status: map_ord_status(report.ord_status),
        client_order_id: report.cl_ord_id,
        orig_client_order_id: report.orig_cl_ord_id,
        venue_order_id: report.order_id,
        execution_id: report.exec_id,
        account_id: report.account_id,
        route_id: report.route_id,
        symbol: report.symbol,
        last_qty: report.last_qty,
        last_price: report.last_price,
        cumulative_qty: report.cumulative_qty,
        leaves_qty: report.leaves_qty,
        average_price: report.average_price,
        ts_exchange_ns: report.ts_exchange_ns,
        ts_recv_ns: report.ts_recv_ns,
        reason: if report.exec_type == FixExecType::Rejected {
            RiskRejectReason::UnsupportedOrderType
        } else {
            RiskRejectReason::None
        },
        text: report.text,
    }
}

/// Maps a parsed FIX OrderCancelReject into a canonical execution event.
pub fn map_order_cancel_reject(report: &FixOrderCancelReject) -> ExecutionEvent {
    ExecutionEvent {
        exec_type: match report.response_to {
            FixCancelRejectResponseTo::OrderCancelRequest => ExecutionType::CancelReject,
            FixCancelRejectResponseTo::OrderCancelReplaceRequest => ExecutionType::ReplaceReject,
        },
        order_status: map_ord_status(report.ord_status),
        client_order_id: report.cl_ord_id,
        orig_client_order_id: report.orig_cl_ord_id,
        venue_order_id: report.order_id,
        execution_id: ExecutionId::empty(),
        account_id: report.account_id,
        route_id: report.route_id,
        symbol: report.symbol,
        last_qty: OrderQty(0),
        last_price: OrderPrice(0),
        cumulative_qty: OrderQty(0),
        leaves_qty: OrderQty(0),
        average_price: OrderPrice(0),
        ts_exchange_ns: report.ts_exchange_ns,
        ts_recv_ns: report.ts_recv_ns,
        reason: RiskRejectReason::None,
        text: report.text,
    }
}

/// FIX execution adapter shell.
#[derive(Debug, Clone)]
pub struct FixExecutionAdapter {
    config: FixSessionConfig,
    connected: bool,
    health_seq: u64,
}

impl FixExecutionAdapter {
    /// Creates a FIX adapter shell.
    pub const fn new(config: FixSessionConfig) -> Self {
        Self {
            config,
            connected: false,
            health_seq: 0,
        }
    }

    /// Returns FIX session config.
    pub const fn config(&self) -> FixSessionConfig {
        self.config
    }
}

impl ExecutionAdapter for FixExecutionAdapter {
    fn connect(&mut self) -> ExecutionResult<()> {
        self.connected = false;
        self.health_seq = self.health_seq.saturating_add(1);
        Err(ExecutionError::Adapter(
            "FIX transport is not configured".to_string(),
        ))
    }

    fn submit(
        &mut self,
        _req: &OrderRequest,
        _out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        Err(ExecutionError::Disconnected)
    }

    fn cancel(
        &mut self,
        _req: &CancelRequest,
        _out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        Err(ExecutionError::Disconnected)
    }

    fn amend(
        &mut self,
        _req: &AmendRequest,
        _out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        Err(ExecutionError::Disconnected)
    }

    fn poll(&mut self, _out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        Err(ExecutionError::Disconnected)
    }

    fn recover_open_orders(&mut self, _out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        Err(ExecutionError::Disconnected)
    }

    fn capabilities(&self) -> ExecutionCapabilities {
        ExecutionCapabilities {
            latency_class: LatencyClass::NativeFix,
            market: true,
            limit: true,
            stop: true,
            stop_limit: true,
            tif_day: true,
            tif_gtc: true,
            tif_ioc: true,
            tif_fok: true,
            tif_gtd: true,
            amend: true,
            native_client_order_id: true,
        }
    }

    fn health(&self) -> ExecutionHealth {
        ExecutionHealth {
            connected: self.connected,
            degraded: !self.connected,
            health_seq: self.health_seq,
            last_error: Some("FIX transport is not configured".to_string()),
            protocol_info: Some(format!(
                "{}:{}->{}",
                self.config.begin_string, self.config.sender_comp_id, self.config.target_comp_id
            )),
        }
    }
}

fn map_exec_type(value: FixExecType) -> ExecutionType {
    match value {
        FixExecType::New => ExecutionType::Ack,
        FixExecType::Rejected => ExecutionType::Reject,
        FixExecType::Trade => ExecutionType::Trade,
        FixExecType::PendingCancel => ExecutionType::CancelPending,
        FixExecType::Canceled => ExecutionType::CancelAck,
        FixExecType::PendingReplace => ExecutionType::ReplacePending,
        FixExecType::Replaced => ExecutionType::ReplaceAck,
        FixExecType::Expired => ExecutionType::Expire,
        FixExecType::Restated => ExecutionType::Restated,
    }
}

fn map_ord_status(value: FixOrdStatus) -> OrderStatus {
    match value {
        FixOrdStatus::New => OrderStatus::New,
        FixOrdStatus::PartiallyFilled => OrderStatus::PartiallyFilled,
        FixOrdStatus::Filled => OrderStatus::Filled,
        FixOrdStatus::DoneForDay => OrderStatus::Suspended,
        FixOrdStatus::Canceled => OrderStatus::Cancelled,
        FixOrdStatus::Replaced => OrderStatus::Replaced,
        FixOrdStatus::PendingCancel => OrderStatus::PendingCancel,
        FixOrdStatus::Stopped => OrderStatus::Suspended,
        FixOrdStatus::Rejected => OrderStatus::Rejected,
        FixOrdStatus::Suspended => OrderStatus::Suspended,
        FixOrdStatus::PendingNew => OrderStatus::PendingNew,
        FixOrdStatus::Expired => OrderStatus::Expired,
        FixOrdStatus::PendingReplace => OrderStatus::PendingReplace,
    }
}

fn parse_exec_type(value: &[u8]) -> Result<FixExecType, FixReportParseError> {
    match value {
        b"0" => Ok(FixExecType::New),
        b"1" | b"2" | b"F" => Ok(FixExecType::Trade),
        b"4" => Ok(FixExecType::Canceled),
        b"5" => Ok(FixExecType::Replaced),
        b"6" => Ok(FixExecType::PendingCancel),
        b"8" => Ok(FixExecType::Rejected),
        b"C" => Ok(FixExecType::Expired),
        b"D" | b"I" => Ok(FixExecType::Restated),
        b"E" => Ok(FixExecType::PendingReplace),
        _ => Err(FixReportParseError::InvalidExecType),
    }
}

fn parse_ord_status(value: &[u8]) -> Result<FixOrdStatus, FixReportParseError> {
    match value {
        b"0" => Ok(FixOrdStatus::New),
        b"1" => Ok(FixOrdStatus::PartiallyFilled),
        b"2" => Ok(FixOrdStatus::Filled),
        b"3" => Ok(FixOrdStatus::DoneForDay),
        b"4" => Ok(FixOrdStatus::Canceled),
        b"5" => Ok(FixOrdStatus::Replaced),
        b"6" => Ok(FixOrdStatus::PendingCancel),
        b"7" => Ok(FixOrdStatus::Stopped),
        b"8" => Ok(FixOrdStatus::Rejected),
        b"9" => Ok(FixOrdStatus::Suspended),
        b"A" => Ok(FixOrdStatus::PendingNew),
        b"C" => Ok(FixOrdStatus::Expired),
        b"E" => Ok(FixOrdStatus::PendingReplace),
        _ => Err(FixReportParseError::InvalidOrdStatus),
    }
}

fn parse_cancel_reject_response_to(
    value: &[u8],
) -> Result<FixCancelRejectResponseTo, FixReportParseError> {
    match value {
        b"1" => Ok(FixCancelRejectResponseTo::OrderCancelRequest),
        b"2" => Ok(FixCancelRejectResponseTo::OrderCancelReplaceRequest),
        _ => Err(FixReportParseError::InvalidCancelRejectResponseTo),
    }
}

fn map_side_to_fix(value: OrderSide) -> FixOrderSide {
    match value {
        OrderSide::Buy => FixOrderSide::Buy,
        OrderSide::Sell => FixOrderSide::Sell,
    }
}

fn map_order_type_to_fix(value: OrderType) -> Result<FixOrdType, FixRequestEncodeError> {
    match value {
        OrderType::Market => Ok(FixOrdType::Market),
        OrderType::Limit => Ok(FixOrdType::Limit),
        OrderType::Stop => Ok(FixOrdType::Stop),
        OrderType::StopLimit => Ok(FixOrdType::StopLimit),
    }
}

fn map_tif_to_fix(value: TimeInForce) -> FixTimeInForce {
    match value {
        TimeInForce::Day => FixTimeInForce::Day,
        TimeInForce::Gtc => FixTimeInForce::GoodTillCancel,
        TimeInForce::Ioc => FixTimeInForce::ImmediateOrCancel,
        TimeInForce::Fok => FixTimeInForce::FillOrKill,
        TimeInForce::Gtd => FixTimeInForce::GoodTillDate,
    }
}

#[derive(Debug, Clone, Copy)]
enum ScaledField {
    Quantity,
    Price,
}

fn encode_order_price(
    buf: &mut [u8; 40],
    order_type: OrderType,
    price: OrderPrice,
    config: FixRequestEncodeConfig,
) -> Result<Option<&[u8]>, FixRequestEncodeError> {
    match order_type {
        OrderType::Market => Ok(None),
        OrderType::Limit => {
            encode_scaled(buf, price.0, config.price_scale, ScaledField::Price).map(Some)
        }
        OrderType::Stop => Ok(None),
        OrderType::StopLimit => {
            encode_scaled(buf, price.0, config.price_scale, ScaledField::Price).map(Some)
        }
    }
}

fn encode_order_stop_price(
    buf: &mut [u8; 40],
    order_type: OrderType,
    stop_price: OrderPrice,
    config: FixRequestEncodeConfig,
) -> Result<Option<&[u8]>, FixRequestEncodeError> {
    match order_type {
        OrderType::Market | OrderType::Limit => Ok(None),
        OrderType::Stop | OrderType::StopLimit => {
            encode_scaled(buf, stop_price.0, config.price_scale, ScaledField::Price).map(Some)
        }
    }
}

fn encode_scaled(
    buf: &mut [u8; 40],
    value: i64,
    scale: i64,
    field: ScaledField,
) -> Result<&[u8], FixRequestEncodeError> {
    if value <= 0 {
        return Err(match field {
            ScaledField::Quantity => FixRequestEncodeError::InvalidQuantity,
            ScaledField::Price => FixRequestEncodeError::InvalidPrice,
        });
    }
    let places = decimal_places(scale)?;
    let value = u64::try_from(value).map_err(|_| match field {
        ScaledField::Quantity => FixRequestEncodeError::InvalidQuantity,
        ScaledField::Price => FixRequestEncodeError::InvalidPrice,
    })?;
    let scale = u64::try_from(scale).map_err(|_| FixRequestEncodeError::InvalidScale)?;
    let whole = value / scale;
    let mut pos = write_u64_ascii(buf, whole);
    let rem = value % scale;
    if rem == 0 {
        return Ok(&buf[..pos]);
    }
    buf[pos] = b'.';
    pos += 1;
    let frac_start = pos;
    pos += write_padded_u64_ascii(&mut buf[pos..], rem, places);
    while pos > frac_start && buf[pos - 1] == b'0' {
        pos -= 1;
    }
    Ok(&buf[..pos])
}

fn decimal_places(scale: i64) -> Result<usize, FixRequestEncodeError> {
    if scale < 1 {
        return Err(FixRequestEncodeError::InvalidScale);
    }
    let mut scale = scale;
    let mut places = 0usize;
    while scale > 1 {
        if scale % 10 != 0 {
            return Err(FixRequestEncodeError::InvalidScale);
        }
        scale /= 10;
        places += 1;
    }
    Ok(places)
}

fn write_u64_ascii(buf: &mut [u8], value: u64) -> usize {
    if value == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 20];
    let mut len = 0usize;
    let mut n = value;
    while n > 0 {
        tmp[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    for idx in 0..len {
        buf[idx] = tmp[len - idx - 1];
    }
    len
}

fn write_padded_u64_ascii(buf: &mut [u8], value: u64, width: usize) -> usize {
    let mut tmp = [b'0'; 20];
    let mut len = 0usize;
    let mut n = value;
    while n > 0 {
        tmp[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    let padding = width.saturating_sub(len);
    for slot in buf.iter_mut().take(padding) {
        *slot = b'0';
    }
    for idx in 0..len {
        buf[padding + idx] = tmp[len - idx - 1];
    }
    padding + len
}

fn required<'a>(
    message: &FixMessageView<'a>,
    tag: FixTag,
) -> Result<&'a [u8], FixReportParseError> {
    message.get(tag).ok_or(FixReportParseError::MissingTag(tag))
}

fn fixed_required<const N: usize>(
    message: &FixMessageView<'_>,
    tag: FixTag,
) -> Result<FixedAscii<N>, FixReportParseError> {
    fixed_from_bytes(tag, required(message, tag)?)
}

fn fixed_optional<const N: usize>(
    message: &FixMessageView<'_>,
    tag: FixTag,
) -> Result<FixedAscii<N>, FixReportParseError> {
    if let Some(value) = message.get(tag) {
        fixed_from_bytes(tag, value)
    } else {
        Ok(FixedAscii::empty())
    }
}

fn fixed_from_bytes<const N: usize>(
    tag: FixTag,
    value: &[u8],
) -> Result<FixedAscii<N>, FixReportParseError> {
    let value = std::str::from_utf8(value).map_err(|_| FixReportParseError::InvalidAscii {
        tag,
        source: ExecutionCoreError::NonAsciiIdentifier,
    })?;
    FixedAscii::new(value).map_err(|source| FixReportParseError::InvalidAscii { tag, source })
}

fn parse_optional_scaled(
    message: &FixMessageView<'_>,
    tag: FixTag,
    scale: i64,
) -> Result<i64, FixReportParseError> {
    if let Some(value) = message.get(tag) {
        parse_scaled(value, scale).ok_or(FixReportParseError::InvalidNumber(tag))
    } else {
        Ok(0)
    }
}

fn parse_optional_u64(
    message: &FixMessageView<'_>,
    tag: FixTag,
) -> Result<u64, FixReportParseError> {
    if let Some(value) = message.get(tag) {
        if tag == FixTag::TRANSACT_TIME {
            parse_u64_digits(value)
                .or_else(|| parse_fix_utc_timestamp_ns(value))
                .ok_or(FixReportParseError::InvalidNumber(tag))
        } else {
            parse_u64_digits(value).ok_or(FixReportParseError::InvalidNumber(tag))
        }
    } else {
        Ok(0)
    }
}

fn parse_fix_utc_timestamp_ns(value: &[u8]) -> Option<u64> {
    if value.len() < 17
        || value.get(8) != Some(&b'-')
        || value.get(11) != Some(&b':')
        || value.get(14) != Some(&b':')
    {
        return None;
    }
    let year = parse_fixed_digits(value.get(0..4)?)? as i64;
    let month = parse_fixed_digits(value.get(4..6)?)? as u32;
    let day = parse_fixed_digits(value.get(6..8)?)? as u32;
    let hour = parse_fixed_digits(value.get(9..11)?)? as u32;
    let minute = parse_fixed_digits(value.get(12..14)?)? as u32;
    let second = parse_fixed_digits(value.get(15..17)?)? as u32;
    if year < 1970
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let fractional_ns = match value.get(17..) {
        Some([]) | None => 0,
        Some(fraction) if fraction.first() == Some(&b'.') => {
            let digits = &fraction[1..];
            if digits.is_empty() || digits.len() > 9 || !digits.iter().all(u8::is_ascii_digit) {
                return None;
            }
            let fraction = parse_fixed_digits(digits)?;
            fraction.checked_mul(10u64.checked_pow(9u32.saturating_sub(digits.len() as u32))?)?
        }
        _ => return None,
    };

    let days = days_from_civil(year, month, day)?;
    let seconds = u64::try_from(days)
        .ok()?
        .checked_mul(86_400)?
        .checked_add(u64::from(hour) * 3_600)?
        .checked_add(u64::from(minute) * 60)?
        .checked_add(u64::from(second))?;
    seconds
        .checked_mul(1_000_000_000)?
        .checked_add(fractional_ns)
}

fn parse_fixed_digits(value: &[u8]) -> Option<u64> {
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return None;
    }
    value.iter().try_fold(0u64, |current, byte| {
        current
            .checked_mul(10)?
            .checked_add(u64::from(*byte - b'0'))
    })
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    let adjusted_year = year.checked_sub(i64::from(month <= 2))?;
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year.checked_sub(era.checked_mul(400)?)?;
    let adjusted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153i64.checked_mul(adjusted_month)?.checked_add(2)? / 5)
        .checked_add(i64::from(day).checked_sub(1)?)?;
    let day_of_era = year_of_era
        .checked_mul(365)?
        .checked_add(year_of_era / 4)?
        .checked_sub(year_of_era / 100)?
        .checked_add(day_of_year)?;
    era.checked_mul(146_097)?
        .checked_add(day_of_era)?
        .checked_sub(719_468)
}

fn parse_scaled(value: &[u8], scale: i64) -> Option<i64> {
    if value.is_empty() || scale < 1 {
        return None;
    }

    let mut int = 0i64;
    let mut frac = 0i64;
    let mut frac_divisor = 1i64;
    let mut seen_dot = false;

    for byte in value {
        if *byte == b'.' && !seen_dot {
            seen_dot = true;
            continue;
        }
        if !byte.is_ascii_digit() {
            return None;
        }
        let digit = i64::from(*byte - b'0');
        if seen_dot {
            frac_divisor = frac_divisor.checked_mul(10)?;
            frac = frac.checked_mul(10)?.checked_add(digit)?;
        } else {
            int = int.checked_mul(10)?.checked_add(digit)?;
        }
    }

    let scaled_int = int.checked_mul(scale)?;
    if frac_divisor == 1 {
        return Some(scaled_int);
    }
    if scale % frac_divisor != 0 {
        return None;
    }
    scaled_int.checked_add(frac.checked_mul(scale / frac_divisor)?)
}

fn parse_u64_digits(value: &[u8]) -> Option<u64> {
    if value.is_empty() {
        return None;
    }
    let mut out = 0u64;
    for byte in value {
        if !byte.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add(u64::from(*byte - b'0'))?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use of_fix::{encode_message, parse_message, FixFieldView};

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn parse_config() -> FixReportParseConfig {
        FixReportParseConfig::new(id("ACC"), id("FIX"), id("BINANCE"))
            .with_quantity_scale(100)
            .with_price_scale(10)
    }

    #[test]
    fn maps_trade_report_to_execution_event() {
        let report = FixExecutionReport {
            exec_type: FixExecType::Trade,
            ord_status: FixOrdStatus::PartiallyFilled,
            cl_ord_id: id("C1"),
            orig_cl_ord_id: ClientOrderId::empty(),
            order_id: id("V1"),
            exec_id: id("E1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            symbol: ExecutionSymbol::new("CME", "ESM6").unwrap(),
            last_qty: OrderQty(5),
            last_price: OrderPrice(5000),
            cumulative_qty: OrderQty(5),
            leaves_qty: OrderQty(5),
            average_price: OrderPrice(5000),
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
            text: ExecutionText::empty(),
        };

        let event = map_execution_report(&report);
        assert_eq!(event.exec_type, ExecutionType::Trade);
        assert_eq!(event.order_status, OrderStatus::PartiallyFilled);
        assert_eq!(event.cumulative_qty, OrderQty(5));
    }

    #[test]
    fn parses_fix_execution_report_from_message_view() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"8",
            &[
                (FixTag::EXEC_TYPE, b"1".as_slice()),
                (FixTag::ORD_STATUS, b"1".as_slice()),
                (FixTag::CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORDER_ID, b"V1".as_slice()),
                (FixTag::EXEC_ID, b"E1".as_slice()),
                (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
                (FixTag::LAST_QTY, b"1.25".as_slice()),
                (FixTag::LAST_PX, b"65000.5".as_slice()),
                (FixTag::CUM_QTY, b"1.25".as_slice()),
                (FixTag::LEAVES_QTY, b"0.75".as_slice()),
                (FixTag::AVG_PX, b"65000.5".as_slice()),
                (FixTag::TRANSACT_TIME, b"1784275200000000000".as_slice()),
                (FixTag::TEXT, b"partial".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let report =
            parse_execution_report(&message, parse_config(), 1784275200000000100).expect("map");

        assert_eq!(report.exec_type, FixExecType::Trade);
        assert_eq!(report.ord_status, FixOrdStatus::PartiallyFilled);
        assert_eq!(report.cl_ord_id, id("C1"));
        assert_eq!(report.order_id, id("V1"));
        assert_eq!(report.exec_id, id("E1"));
        assert_eq!(report.account_id, id("ACC"));
        assert_eq!(report.route_id, id("FIX"));
        assert_eq!(report.symbol.venue, id("BINANCE"));
        assert_eq!(report.symbol.instrument, id("BTCUSDT"));
        assert_eq!(report.last_qty, OrderQty(125));
        assert_eq!(report.last_price, OrderPrice(650005));
        assert_eq!(report.cumulative_qty, OrderQty(125));
        assert_eq!(report.leaves_qty, OrderQty(75));
        assert_eq!(report.average_price, OrderPrice(650005));
        assert_eq!(report.ts_exchange_ns, 1_784_275_200_000_000_000);
        assert_eq!(report.ts_recv_ns, 1_784_275_200_000_000_100);
        assert_eq!(report.text, id("partial"));
    }

    #[test]
    fn parses_account_from_fix_when_present() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"8",
            &[
                (FixTag::EXEC_TYPE, b"0".as_slice()),
                (FixTag::ORD_STATUS, b"0".as_slice()),
                (FixTag::CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORDER_ID, b"V1".as_slice()),
                (FixTag::EXEC_ID, b"E1".as_slice()),
                (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
                (FixTag(1), b"SUBACC".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let report = parse_execution_report(&message, parse_config(), 10).expect("map");
        assert_eq!(report.account_id, id("SUBACC"));
        assert_eq!(report.exec_type, FixExecType::New);
        assert_eq!(report.ord_status, FixOrdStatus::New);
    }

    #[test]
    fn parses_order_cancel_reject_from_message_view() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"9",
            &[
                (FixTag::ORDER_ID, b"V1".as_slice()),
                (FixTag::CL_ORD_ID, b"CANCEL-1".as_slice()),
                (FixTag::ORIG_CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORD_STATUS, b"0".as_slice()),
                (CXL_REJ_RESPONSE_TO_TAG, b"1".as_slice()),
                (CXL_REJ_REASON_TAG, b"1".as_slice()),
                (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
                (FixTag::TRANSACT_TIME, b"1784275200000000000".as_slice()),
                (FixTag::TEXT, b"too late".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let report = parse_order_cancel_reject(&message, parse_config(), 1_784_275_200_000_000_100)
            .expect("cancel reject");

        assert_eq!(
            report.response_to,
            FixCancelRejectResponseTo::OrderCancelRequest
        );
        assert_eq!(report.ord_status, FixOrdStatus::New);
        assert_eq!(report.cl_ord_id, id("CANCEL-1"));
        assert_eq!(report.orig_cl_ord_id, id("C1"));
        assert_eq!(report.order_id, id("V1"));
        assert_eq!(report.account_id, id("ACC"));
        assert_eq!(report.route_id, id("FIX"));
        assert_eq!(report.symbol.venue, id("BINANCE"));
        assert_eq!(report.symbol.instrument, id("BTCUSDT"));
        assert_eq!(report.cxl_rej_reason, 1);
        assert_eq!(report.ts_exchange_ns, 1_784_275_200_000_000_000);
        assert_eq!(report.ts_recv_ns, 1_784_275_200_000_000_100);
        assert_eq!(report.text, id("too late"));

        let event = map_order_cancel_reject(&report);
        assert_eq!(event.exec_type, ExecutionType::CancelReject);
        assert_eq!(event.order_status, OrderStatus::New);
        assert_eq!(event.client_order_id, id("CANCEL-1"));
        assert_eq!(event.orig_client_order_id, id("C1"));
        assert_eq!(event.reason, RiskRejectReason::None);
    }

    #[test]
    fn maps_order_cancel_replace_reject() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"9",
            &[
                (FixTag::CL_ORD_ID, b"REPLACE-1".as_slice()),
                (FixTag::ORIG_CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORD_STATUS, b"0".as_slice()),
                (CXL_REJ_RESPONSE_TO_TAG, b"2".as_slice()),
                (ACCOUNT_TAG, b"SUBACC".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 24];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let report = parse_order_cancel_reject(&message, parse_config(), 10).expect("map");
        let event = map_order_cancel_reject(&report);

        assert_eq!(
            report.response_to,
            FixCancelRejectResponseTo::OrderCancelReplaceRequest
        );
        assert_eq!(report.account_id, id("SUBACC"));
        assert_eq!(event.exec_type, ExecutionType::ReplaceReject);
        assert_eq!(event.venue_order_id, VenueOrderId::empty());
        assert_eq!(event.symbol.instrument, InstrumentId::empty());
    }

    #[test]
    fn rejects_invalid_cancel_reject_response_to() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"9",
            &[
                (FixTag::CL_ORD_ID, b"CANCEL-1".as_slice()),
                (FixTag::ORIG_CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORD_STATUS, b"0".as_slice()),
                (CXL_REJ_RESPONSE_TO_TAG, b"9".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 24];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            parse_order_cancel_reject(&message, parse_config(), 0),
            Err(FixReportParseError::InvalidCancelRejectResponseTo)
        );
    }

    #[test]
    fn rejects_missing_required_report_tag() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"8",
            &[
                (FixTag::EXEC_TYPE, b"0".as_slice()),
                (FixTag::ORD_STATUS, b"0".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            parse_execution_report(&message, parse_config(), 0),
            Err(FixReportParseError::MissingTag(FixTag::CL_ORD_ID))
        );
    }

    #[test]
    fn rejects_unrepresentable_decimal_scale() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"8",
            &[
                (FixTag::EXEC_TYPE, b"1".as_slice()),
                (FixTag::ORD_STATUS, b"1".as_slice()),
                (FixTag::CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORDER_ID, b"V1".as_slice()),
                (FixTag::EXEC_ID, b"E1".as_slice()),
                (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
                (FixTag::LAST_QTY, b"1.234".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            parse_execution_report(&message, parse_config(), 0),
            Err(FixReportParseError::InvalidNumber(FixTag::LAST_QTY))
        );
    }

    #[test]
    fn encodes_order_request_to_fix_new_order_single() {
        let request = OrderRequest {
            client_order_id: id("C1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            strategy_id: id("S1"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(125),
            limit_price: OrderPrice(650005),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 0,
            ts_recv_ns: 10,
        };
        let config = FixRequestEncodeConfig::new()
            .with_quantity_scale(100)
            .with_price_scale(10);
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:00.000");
        let mut raw = Vec::new();
        encode_order_request(
            &mut raw,
            FixVersion::Fix44,
            header,
            config,
            &request,
            b"20260717-12:00:00.000",
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.msg_type(), Some(b"D".as_slice()));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"C1".as_slice()));
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::SYMBOL), Some(b"BTCUSDT".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"1".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"1.25".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65000.5".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"0".as_slice()));
    }

    #[test]
    fn encodes_stop_limit_order_request_to_fix_new_order_single() {
        let request = OrderRequest {
            client_order_id: id("C2"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            strategy_id: id("S1"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            side: OrderSide::Sell,
            order_type: OrderType::StopLimit,
            time_in_force: TimeInForce::Gtc,
            quantity: OrderQty(100),
            limit_price: OrderPrice(650005),
            stop_price: OrderPrice(649505),
            ts_exchange_ns: 0,
            ts_recv_ns: 10,
        };
        let config = FixRequestEncodeConfig::new()
            .with_quantity_scale(100)
            .with_price_scale(10);
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:00.000");
        let mut raw = Vec::new();
        encode_order_request(
            &mut raw,
            FixVersion::Fix44,
            header,
            config,
            &request,
            b"20260717-12:00:00.000",
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.msg_type(), Some(b"D".as_slice()));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"C2".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"2".as_slice()));
        assert_eq!(message.get(FixTag::ORD_TYPE), Some(b"4".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"1".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65000.5".as_slice()));
        assert_eq!(message.get(FixTag::STOP_PX), Some(b"64950.5".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"1".as_slice()));
    }

    #[test]
    fn encodes_cancel_request_with_explicit_side_context() {
        let request = CancelRequest {
            client_order_id: id("CXL-1"),
            orig_client_order_id: id("C1"),
            venue_order_id: id("V1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            ts_recv_ns: 10,
        };
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 8, b"20260717-12:00:01.000");
        let context = FixCancelEncodeContext::new(OrderSide::Sell, b"20260717-12:00:01.000");
        let mut raw = Vec::new();
        encode_cancel_request(&mut raw, FixVersion::Fix44, header, &request, context)
            .expect("encode");

        let mut scratch = [FixFieldView::empty(); 24];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.msg_type(), Some(b"F".as_slice()));
        assert_eq!(message.get(FixTag::ORIG_CL_ORD_ID), Some(b"C1".as_slice()));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"CXL-1".as_slice()));
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"2".as_slice()));
    }

    #[test]
    fn encodes_amend_request_with_explicit_order_context() {
        let request = AmendRequest {
            client_order_id: id("RPL-1"),
            orig_client_order_id: id("C1"),
            venue_order_id: id("V1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            quantity: OrderQty(200),
            limit_price: OrderPrice(650100),
            ts_recv_ns: 10,
        };
        let config = FixRequestEncodeConfig::new()
            .with_quantity_scale(100)
            .with_price_scale(10);
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 9, b"20260717-12:00:02.000");
        let context = FixAmendEncodeContext::new(
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::Gtc,
            b"20260717-12:00:02.000",
        );
        let mut raw = Vec::new();
        encode_amend_request(
            &mut raw,
            FixVersion::Fix44,
            header,
            config,
            &request,
            context,
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.msg_type(), Some(b"G".as_slice()));
        assert_eq!(message.get(FixTag::ORIG_CL_ORD_ID), Some(b"C1".as_slice()));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"RPL-1".as_slice()));
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"2".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65010".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"1".as_slice()));
    }

    #[test]
    fn encodes_stop_amend_request_with_explicit_stop_context() {
        let request = AmendRequest {
            client_order_id: id("RPL-STP"),
            orig_client_order_id: id("C1"),
            venue_order_id: id("V1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            quantity: OrderQty(200),
            limit_price: OrderPrice(650100),
            ts_recv_ns: 10,
        };
        let config = FixRequestEncodeConfig::new()
            .with_quantity_scale(100)
            .with_price_scale(10);
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 9, b"20260717-12:00:02.000");
        let context = FixStopAmendEncodeContext::new(
            OrderSide::Buy,
            OrderType::StopLimit,
            TimeInForce::Gtc,
            OrderPrice(649900),
            b"20260717-12:00:02.000",
        );
        let mut raw = Vec::new();
        encode_stop_amend_request(
            &mut raw,
            FixVersion::Fix44,
            header,
            config,
            &request,
            context,
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.msg_type(), Some(b"G".as_slice()));
        assert_eq!(message.get(FixTag::ORD_TYPE), Some(b"4".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65010".as_slice()));
        assert_eq!(message.get(FixTag::STOP_PX), Some(b"64990".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"1".as_slice()));
    }

    #[test]
    fn request_encoder_rejects_invalid_shapes() {
        let mut request = OrderRequest {
            client_order_id: id("C1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            strategy_id: id("S1"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            side: OrderSide::Buy,
            order_type: OrderType::Stop,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(125),
            limit_price: OrderPrice(0),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 0,
            ts_recv_ns: 10,
        };
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:00.000");
        let mut raw = Vec::new();
        assert_eq!(
            encode_order_request(
                &mut raw,
                FixVersion::Fix44,
                header,
                FixRequestEncodeConfig::new().with_quantity_scale(100),
                &request,
                b"20260717-12:00:00.000",
            ),
            Err(FixRequestEncodeError::InvalidPrice)
        );

        request.order_type = OrderType::Limit;
        request.limit_price = OrderPrice(650005);
        assert_eq!(
            encode_order_request(
                &mut raw,
                FixVersion::Fix44,
                header,
                FixRequestEncodeConfig {
                    quantity_scale: 3,
                    price_scale: 10,
                },
                &request,
                b"20260717-12:00:00.000",
            ),
            Err(FixRequestEncodeError::InvalidScale)
        );

        let amend = AmendRequest {
            client_order_id: id("RPL-STOP"),
            orig_client_order_id: id("C1"),
            venue_order_id: id("V1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            quantity: OrderQty(125),
            limit_price: OrderPrice(650005),
            ts_recv_ns: 10,
        };
        let context = FixAmendEncodeContext::new(
            OrderSide::Buy,
            OrderType::StopLimit,
            TimeInForce::Day,
            b"20260717-12:00:00.000",
        );
        assert_eq!(
            encode_amend_request(
                &mut raw,
                FixVersion::Fix44,
                header,
                FixRequestEncodeConfig::new()
                    .with_quantity_scale(100)
                    .with_price_scale(10),
                &amend,
                context,
            ),
            Err(FixRequestEncodeError::UnsupportedOrderType)
        );

        let non_stop_context = FixStopAmendEncodeContext::new(
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::Day,
            OrderPrice(650000),
            b"20260717-12:00:00.000",
        );
        assert_eq!(
            encode_stop_amend_request(
                &mut raw,
                FixVersion::Fix44,
                header,
                FixRequestEncodeConfig::new()
                    .with_quantity_scale(100)
                    .with_price_scale(10),
                &amend,
                non_stop_context,
            ),
            Err(FixRequestEncodeError::UnsupportedOrderType)
        );
    }

    #[test]
    fn fix_adapter_fails_closed_without_transport() {
        let cfg = FixSessionConfig::new("FIX.4.4", "SENDER", "TARGET", 30).unwrap();
        let mut adapter = FixExecutionAdapter::new(cfg);
        assert!(adapter.connect().is_err());
        assert_eq!(
            adapter.capabilities().latency_class,
            LatencyClass::NativeFix
        );
        assert!(adapter.health().degraded);
    }
}
