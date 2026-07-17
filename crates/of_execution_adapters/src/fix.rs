//! FIX execution adapter scaffold and report mapper.

use of_execution::{
    ExecutionAdapter, ExecutionCapabilities, ExecutionError, ExecutionEventBuffer, ExecutionHealth,
    ExecutionResult, LatencyClass,
};
use of_execution_core::{
    AccountId, AmendRequest, CancelRequest, ClientOrderId, ExecutionCoreError, ExecutionEvent,
    ExecutionId, ExecutionSymbol, ExecutionText, ExecutionType, FixedAscii, InstrumentId,
    OrderPrice, OrderQty, OrderRequest, OrderStatus, RiskRejectReason, RouteId, VenueId,
    VenueOrderId,
};
use of_fix::{FixMessageView, FixMsgType, FixTag};
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
        b"1" | b"2" => Ok(FixExecType::Trade),
        b"4" => Ok(FixExecType::Canceled),
        b"5" => Ok(FixExecType::Replaced),
        b"6" => Ok(FixExecType::PendingCancel),
        b"8" => Ok(FixExecType::Rejected),
        b"C" => Ok(FixExecType::Expired),
        b"D" => Ok(FixExecType::Restated),
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
        parse_u64_digits(value).ok_or(FixReportParseError::InvalidNumber(tag))
    } else {
        Ok(0)
    }
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
