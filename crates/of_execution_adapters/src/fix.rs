//! FIX execution adapter scaffold and report mapper.

use of_execution::{
    ExecutionAdapter, ExecutionCapabilities, ExecutionError, ExecutionEventBuffer, ExecutionHealth,
    ExecutionResult, LatencyClass,
};
use of_execution_core::{
    AccountId, AmendRequest, CancelRequest, ClientOrderId, ExecutionEvent, ExecutionId,
    ExecutionSymbol, ExecutionText, ExecutionType, FixedAscii, OrderPrice, OrderQty, OrderRequest,
    OrderStatus, RiskRejectReason, RouteId, VenueOrderId,
};

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

#[cfg(test)]
mod tests {
    use super::*;

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
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
