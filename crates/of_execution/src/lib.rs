//! Execution routing and adapter contracts for Orderflow.
#![doc = include_str!("../README.md")]

mod oms;
pub use oms::*;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{
    self, Receiver, RecvError, RecvTimeoutError, SyncSender, TryRecvError, TrySendError,
};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use of_execution_core::{
    AccountId, AmendRequest, BasicRiskGate, CancelRequest, ClientOrderId, ExecutionCoreError,
    ExecutionEvent, ExecutionId, ExecutionSymbol, ExecutionText, ExecutionType, OrderPrice,
    OrderQty, OrderRequest, OrderState, OrderStateMachine, OrderStatus, OrderType, RiskCheck,
    RiskContext, RiskLimits, RiskRejectReason, RouteId, TimeInForce, VenueOrderId,
};

/// Execution result alias.
pub type ExecutionResult<T> = Result<T, ExecutionError>;

/// Execution-layer error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    /// Adapter is disconnected.
    Disconnected,
    /// Command or event buffer has reached its configured bound.
    BufferFull,
    /// Route/account/symbol is not configured.
    RouteNotFound,
    /// Pre-trade risk rejected the request.
    RiskRejected(RiskRejectReason),
    /// Core model/state-machine error.
    Core(ExecutionCoreError),
    /// Adapter-specific error.
    Adapter(String),
    /// Journal-specific error.
    Journal(String),
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "execution adapter disconnected"),
            Self::BufferFull => write!(f, "execution event buffer is full"),
            Self::RouteNotFound => write!(f, "execution route not found"),
            Self::RiskRejected(reason) => write!(f, "risk rejected order: {reason:?}"),
            Self::Core(err) => write!(f, "execution core error: {err}"),
            Self::Adapter(err) => write!(f, "execution adapter error: {err}"),
            Self::Journal(err) => write!(f, "execution journal error: {err}"),
        }
    }
}

impl Error for ExecutionError {}

impl From<ExecutionCoreError> for ExecutionError {
    fn from(value: ExecutionCoreError) -> Self {
        Self::Core(value)
    }
}

/// Caller-owned event buffer used by execution adapters.
#[derive(Debug, Clone)]
pub struct ExecutionEventBuffer {
    events: Vec<ExecutionEvent>,
    max_len: usize,
}

impl ExecutionEventBuffer {
    /// Creates an empty event buffer with bounded capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            max_len: capacity,
        }
    }

    /// Appends one event.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::BufferFull`] when the configured bound is hit.
    pub fn push(&mut self, event: ExecutionEvent) -> ExecutionResult<()> {
        if self.events.len() >= self.max_len {
            return Err(ExecutionError::BufferFull);
        }
        self.events.push(event);
        Ok(())
    }

    /// Clears all buffered events without releasing capacity.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Returns buffered events.
    pub fn as_slice(&self) -> &[ExecutionEvent] {
        &self.events
    }

    /// Returns mutable buffered events.
    pub fn as_mut_slice(&mut self) -> &mut [ExecutionEvent] {
        &mut self.events
    }

    /// Drains events into `out`.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::BufferFull`] when `out` cannot accept an event.
    pub fn drain_into(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        let mut count = 0;
        for event in self.events.drain(..) {
            out.push(event)?;
            count += 1;
        }
        Ok(count)
    }

    /// Returns the number of buffered events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true when the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns the configured maximum event count.
    pub const fn max_len(&self) -> usize {
        self.max_len
    }
}

impl Default for ExecutionEventBuffer {
    fn default() -> Self {
        Self::with_capacity(64)
    }
}

/// Adapter latency classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyClass {
    /// Native FIX session.
    NativeFix,
    /// Native binary exchange protocol.
    NativeBinary,
    /// Streaming websocket protocol.
    StreamingWebSocket,
    /// REST or request/response convenience protocol.
    RestConvenience,
    /// Deterministic simulation.
    Simulated,
}

/// Execution adapter capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionCapabilities {
    /// Latency class.
    pub latency_class: LatencyClass,
    /// Market orders are supported.
    pub market: bool,
    /// Limit orders are supported.
    pub limit: bool,
    /// Stop orders are supported.
    pub stop: bool,
    /// Stop-limit orders are supported.
    pub stop_limit: bool,
    /// Day TIF is supported.
    pub tif_day: bool,
    /// GTC TIF is supported.
    pub tif_gtc: bool,
    /// IOC TIF is supported.
    pub tif_ioc: bool,
    /// FOK TIF is supported.
    pub tif_fok: bool,
    /// GTD TIF is supported.
    pub tif_gtd: bool,
    /// Cancel/replace is supported.
    pub amend: bool,
    /// Venue preserves client-order-id semantics.
    pub native_client_order_id: bool,
}

impl ExecutionCapabilities {
    /// Returns deterministic simulation capabilities.
    pub const fn simulated() -> Self {
        Self {
            latency_class: LatencyClass::Simulated,
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

    /// Returns true when an order type is supported.
    pub const fn supports_order_type(self, order_type: OrderType) -> bool {
        match order_type {
            OrderType::Market => self.market,
            OrderType::Limit => self.limit,
            OrderType::Stop => self.stop,
            OrderType::StopLimit => self.stop_limit,
        }
    }

    /// Returns true when a time-in-force value is supported.
    pub const fn supports_tif(self, tif: TimeInForce) -> bool {
        match tif {
            TimeInForce::Day => self.tif_day,
            TimeInForce::Gtc => self.tif_gtc,
            TimeInForce::Ioc => self.tif_ioc,
            TimeInForce::Fok => self.tif_fok,
            TimeInForce::Gtd => self.tif_gtd,
        }
    }
}

/// Execution adapter health snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecutionHealth {
    /// True when transport/session is connected.
    pub connected: bool,
    /// True when transport/session is degraded.
    pub degraded: bool,
    /// Monotonic health sequence.
    pub health_seq: u64,
    /// Last error text.
    pub last_error: Option<String>,
    /// Protocol/session diagnostics.
    pub protocol_info: Option<String>,
}

/// Common execution adapter interface.
pub trait ExecutionAdapter: Send {
    /// Establishes adapter transport/session.
    fn connect(&mut self) -> ExecutionResult<()>;
    /// Submits a new order.
    fn submit(&mut self, req: &OrderRequest, out: &mut ExecutionEventBuffer)
        -> ExecutionResult<()>;
    /// Cancels an existing order.
    fn cancel(
        &mut self,
        req: &CancelRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()>;
    /// Amends an existing order.
    fn amend(&mut self, req: &AmendRequest, out: &mut ExecutionEventBuffer) -> ExecutionResult<()>;
    /// Drains ready execution events.
    fn poll(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize>;
    /// Emits open-order recovery state.
    fn recover_open_orders(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize>;
    /// Returns adapter capabilities.
    fn capabilities(&self) -> ExecutionCapabilities;
    /// Returns adapter health.
    fn health(&self) -> ExecutionHealth;
}

impl ExecutionAdapter for Box<dyn ExecutionAdapter> {
    fn connect(&mut self) -> ExecutionResult<()> {
        self.as_mut().connect()
    }

    fn submit(
        &mut self,
        req: &OrderRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        self.as_mut().submit(req, out)
    }

    fn cancel(
        &mut self,
        req: &CancelRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        self.as_mut().cancel(req, out)
    }

    fn amend(&mut self, req: &AmendRequest, out: &mut ExecutionEventBuffer) -> ExecutionResult<()> {
        self.as_mut().amend(req, out)
    }

    fn poll(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        self.as_mut().poll(out)
    }

    fn recover_open_orders(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        self.as_mut().recover_open_orders(out)
    }

    fn capabilities(&self) -> ExecutionCapabilities {
        self.as_ref().capabilities()
    }

    fn health(&self) -> ExecutionHealth {
        self.as_ref().health()
    }
}

/// Execution route configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteConfig {
    /// Route id.
    pub route_id: RouteId,
    /// Account id.
    pub account_id: AccountId,
    /// Symbol allowed on the route.
    pub symbol: ExecutionSymbol,
    /// Route enabled flag.
    pub enabled: bool,
    /// Static route risk limits.
    pub risk_limits: RiskLimits,
}

/// Stable lookup key for a configured execution route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RouteKey {
    /// Route id.
    pub route_id: RouteId,
    /// Trading account.
    pub account_id: AccountId,
    /// Venue-native execution symbol.
    pub symbol: ExecutionSymbol,
}

impl RouteKey {
    /// Creates a route lookup key.
    pub const fn new(route_id: RouteId, account_id: AccountId, symbol: ExecutionSymbol) -> Self {
        Self {
            route_id,
            account_id,
            symbol,
        }
    }
}

/// Pass-through risk hook for engines that rely on route-scoped limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AllowAllRiskGate;

impl RiskCheck for AllowAllRiskGate {
    fn check_new(
        &self,
        _req: &OrderRequest,
        _ctx: &RiskContext,
    ) -> of_execution_core::RiskDecision {
        of_execution_core::RiskDecision::allow()
    }

    fn check_amend(
        &self,
        _req: &AmendRequest,
        _ctx: &RiskContext,
    ) -> of_execution_core::RiskDecision {
        of_execution_core::RiskDecision::allow()
    }

    fn check_cancel(
        &self,
        _req: &CancelRequest,
        _ctx: &RiskContext,
    ) -> of_execution_core::RiskDecision {
        of_execution_core::RiskDecision::allow()
    }
}

/// Journal command kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalCommandKind {
    /// New order command.
    Submit,
    /// Cancel command.
    Cancel,
    /// Amend command.
    Amend,
}

/// Execution journal record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalRecord {
    /// Command accepted by the local engine.
    Command {
        /// Command kind.
        kind: JournalCommandKind,
        /// Client order id.
        client_order_id: ClientOrderId,
        /// Nanosecond timestamp.
        ts_ns: u64,
    },
    /// Execution event.
    Event(Box<ExecutionEvent>),
}

/// Execution journal hook.
pub trait ExecutionJournal: Send {
    /// Records a command.
    fn record_command(
        &mut self,
        kind: JournalCommandKind,
        id: ClientOrderId,
        ts_ns: u64,
    ) -> ExecutionResult<()>;
    /// Records an execution event.
    fn record_event(&mut self, event: &ExecutionEvent) -> ExecutionResult<()>;
    /// Replays known records into `out`.
    fn replay(&self, out: &mut Vec<JournalRecord>) -> ExecutionResult<usize>;
}

/// In-memory execution journal for tests and embedded hosts.
#[derive(Debug, Default, Clone)]
pub struct InMemoryJournal {
    records: Vec<JournalRecord>,
}

impl InMemoryJournal {
    /// Returns journal records.
    pub fn records(&self) -> &[JournalRecord] {
        &self.records
    }
}

impl ExecutionJournal for InMemoryJournal {
    fn record_command(
        &mut self,
        kind: JournalCommandKind,
        id: ClientOrderId,
        ts_ns: u64,
    ) -> ExecutionResult<()> {
        self.records.push(JournalRecord::Command {
            kind,
            client_order_id: id,
            ts_ns,
        });
        Ok(())
    }

    fn record_event(&mut self, event: &ExecutionEvent) -> ExecutionResult<()> {
        self.records.push(JournalRecord::Event(Box::new(*event)));
        Ok(())
    }

    fn replay(&self, out: &mut Vec<JournalRecord>) -> ExecutionResult<usize> {
        let len = self.records.len();
        out.extend_from_slice(&self.records);
        Ok(len)
    }
}

/// Execution metrics snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExecutionMetrics {
    /// Submitted orders accepted by local engine.
    pub submitted: u64,
    /// Cancel commands accepted by local engine.
    pub cancelled: u64,
    /// Amend commands accepted by local engine.
    pub amended: u64,
    /// Events applied to state machines.
    pub events_applied: u64,
    /// Risk rejections.
    pub risk_rejected: u64,
    /// Adapter errors.
    pub adapter_errors: u64,
    /// Recovery events applied.
    pub recovered: u64,
}

/// Read-only operator runbook summary for an execution engine.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExecutionRunbookSnapshot {
    /// True when the engine has started its adapter.
    pub started: bool,
    /// True when the adapter reports connected.
    pub connected: bool,
    /// True when the adapter reports degraded.
    pub degraded: bool,
    /// Latest adapter health sequence.
    pub health_seq: u64,
    /// Configured route count.
    pub route_count: usize,
    /// Enabled route count.
    pub enabled_route_count: usize,
    /// Disabled route count.
    pub disabled_route_count: usize,
    /// Enabled route count with route-level kill switch active.
    pub kill_switch_route_count: usize,
    /// Locally known non-terminal order count.
    pub open_order_count: usize,
    /// Locally known terminal order count.
    pub terminal_order_count: usize,
    /// Submitted orders accepted by the local engine.
    pub submitted: u64,
    /// Cancel commands accepted by the local engine.
    pub cancelled: u64,
    /// Amend commands accepted by the local engine.
    pub amended: u64,
    /// Execution events applied to state machines.
    pub events_applied: u64,
    /// Risk rejection count.
    pub risk_rejected: u64,
    /// Adapter error count.
    pub adapter_errors: u64,
    /// Recovery event count.
    pub recovered: u64,
    /// True when new submissions are blocked for all configured routes.
    pub new_submissions_blocked: bool,
    /// True when an operator should inspect the route or adapter state.
    pub operator_attention_required: bool,
}

impl ExecutionRunbookSnapshot {
    /// Returns true when the engine can accept at least one new-order route.
    pub const fn can_submit_new_orders(self) -> bool {
        !self.new_submissions_blocked
    }
}

/// Configuration for the concurrent execution worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConcurrentExecutionConfig {
    /// Bounded command queue capacity.
    pub command_capacity: usize,
    /// Bounded report queue capacity.
    pub report_capacity: usize,
    /// Per-command event buffer capacity used by the worker.
    pub event_buffer_capacity: usize,
}

impl Default for ConcurrentExecutionConfig {
    fn default() -> Self {
        Self {
            command_capacity: 1024,
            report_capacity: 1024,
            event_buffer_capacity: 64,
        }
    }
}

/// Command kind sent to a concurrent execution worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionCommandKind {
    /// Submit a new order.
    Submit,
    /// Cancel an order.
    Cancel,
    /// Amend an order.
    Amend,
    /// Poll adapter events.
    Poll,
    /// Recover open orders.
    RecoverOpenOrders,
    /// Stop the worker.
    Stop,
}

/// Command payload sent to a concurrent execution worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionCommand {
    /// Submit a new order.
    Submit(OrderRequest),
    /// Cancel an existing order.
    Cancel(CancelRequest),
    /// Amend an existing order.
    Amend(AmendRequest),
    /// Poll adapter events.
    Poll,
    /// Recover open orders.
    RecoverOpenOrders,
    /// Stop the worker after prior queued commands are processed.
    Stop,
}

impl ExecutionCommand {
    /// Returns this command's kind.
    pub const fn kind(self) -> ExecutionCommandKind {
        match self {
            Self::Submit(_) => ExecutionCommandKind::Submit,
            Self::Cancel(_) => ExecutionCommandKind::Cancel,
            Self::Amend(_) => ExecutionCommandKind::Amend,
            Self::Poll => ExecutionCommandKind::Poll,
            Self::RecoverOpenOrders => ExecutionCommandKind::RecoverOpenOrders,
            Self::Stop => ExecutionCommandKind::Stop,
        }
    }
}

/// Result report emitted by a concurrent execution worker.
#[derive(Debug, Clone)]
pub struct ExecutionCommandReport {
    /// Monotonic local command sequence assigned by the sender.
    pub sequence: u64,
    /// Command kind.
    pub kind: ExecutionCommandKind,
    /// Command result. The `Ok` value is the number of events in `events`.
    pub result: ExecutionResult<usize>,
    /// Events produced while processing the command.
    pub events: ExecutionEventBuffer,
}

/// Error returned by the concurrent execution wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcurrentExecutionError {
    /// The bounded command queue is full.
    Backpressure,
    /// The worker has stopped or the channel is disconnected.
    Stopped,
    /// The worker thread panicked.
    WorkerPanic,
    /// The synchronous execution engine failed.
    Execution(ExecutionError),
}

impl fmt::Display for ConcurrentExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backpressure => write!(f, "concurrent execution command queue is full"),
            Self::Stopped => write!(f, "concurrent execution worker is stopped"),
            Self::WorkerPanic => write!(f, "concurrent execution worker panicked"),
            Self::Execution(err) => write!(f, "{err}"),
        }
    }
}

impl Error for ConcurrentExecutionError {}

impl From<ExecutionError> for ConcurrentExecutionError {
    fn from(value: ExecutionError) -> Self {
        Self::Execution(value)
    }
}

#[derive(Debug)]
struct WorkerCommand {
    sequence: u64,
    command: ExecutionCommand,
}

/// Cloneable concurrent command handle for an execution worker.
#[derive(Debug, Clone)]
pub struct ExecutionCommandSender {
    tx: SyncSender<WorkerCommand>,
    next_sequence: Arc<AtomicU64>,
}

impl ExecutionCommandSender {
    /// Sends a command, blocking when the bounded command queue is full.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::Stopped`] if the worker has stopped.
    pub fn send(&self, command: ExecutionCommand) -> Result<u64, ConcurrentExecutionError> {
        let sequence = self.next_sequence();
        self.tx
            .send(WorkerCommand { sequence, command })
            .map_err(|_| ConcurrentExecutionError::Stopped)?;
        Ok(sequence)
    }

    /// Attempts to send a command without blocking.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::Backpressure`] when the command queue
    /// is full, or [`ConcurrentExecutionError::Stopped`] when the worker has
    /// stopped.
    pub fn try_send(&self, command: ExecutionCommand) -> Result<u64, ConcurrentExecutionError> {
        let sequence = self.next_sequence();
        match self.tx.try_send(WorkerCommand { sequence, command }) {
            Ok(()) => Ok(sequence),
            Err(TrySendError::Full(_)) => Err(ConcurrentExecutionError::Backpressure),
            Err(TrySendError::Disconnected(_)) => Err(ConcurrentExecutionError::Stopped),
        }
    }

    /// Sends a submit command, blocking on command-queue capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn submit(&self, req: OrderRequest) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::Submit(req))
    }

    /// Attempts to send a submit command without blocking.
    ///
    /// # Errors
    ///
    /// Returns an error on backpressure or stopped worker.
    pub fn try_submit(&self, req: OrderRequest) -> Result<u64, ConcurrentExecutionError> {
        self.try_send(ExecutionCommand::Submit(req))
    }

    /// Sends a cancel command, blocking on command-queue capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn cancel(&self, req: CancelRequest) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::Cancel(req))
    }

    /// Sends an amend command, blocking on command-queue capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn amend(&self, req: AmendRequest) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::Amend(req))
    }

    /// Sends a poll command, blocking on command-queue capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn poll(&self) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::Poll)
    }

    /// Sends a recovery command, blocking on command-queue capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn recover_open_orders(&self) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::RecoverOpenOrders)
    }

    /// Requests worker shutdown after previously queued commands complete.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has already stopped.
    pub fn stop(&self) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::Stop)
    }

    fn next_sequence(&self) -> u64 {
        self.next_sequence.fetch_add(1, Ordering::Relaxed)
    }
}

/// Concurrent owner for a synchronous execution engine.
pub struct ConcurrentExecutionEngine {
    sender: ExecutionCommandSender,
    reports: Option<Receiver<ExecutionCommandReport>>,
    join: Option<JoinHandle<()>>,
}

impl ConcurrentExecutionEngine {
    /// Spawns a worker thread that owns and serially drives `engine`.
    ///
    /// # Errors
    ///
    /// Returns an error when the engine fails to start or the worker cannot
    /// publish its startup result.
    pub fn spawn<A, R, J>(
        engine: ExecutionEngine<A, R, J>,
        config: ConcurrentExecutionConfig,
    ) -> Result<Self, ConcurrentExecutionError>
    where
        A: ExecutionAdapter + 'static,
        R: RiskCheck + 'static,
        J: ExecutionJournal + 'static,
    {
        let (tx, rx) = mpsc::sync_channel(config.command_capacity);
        let (report_tx, reports) = mpsc::sync_channel(config.report_capacity);
        let (start_tx, start_rx) = mpsc::sync_channel(1);
        let join = thread::Builder::new()
            .name("orderflow-execution-worker".to_string())
            .spawn(move || {
                run_execution_worker(
                    engine,
                    rx,
                    report_tx,
                    start_tx,
                    config.event_buffer_capacity,
                );
            })
            .map_err(|_| ConcurrentExecutionError::Stopped)?;

        match start_rx
            .recv()
            .map_err(|_| ConcurrentExecutionError::Stopped)?
        {
            Ok(()) => Ok(Self {
                sender: ExecutionCommandSender {
                    tx,
                    next_sequence: Arc::new(AtomicU64::new(1)),
                },
                reports: Some(reports),
                join: Some(join),
            }),
            Err(err) => {
                let _ = join.join();
                Err(ConcurrentExecutionError::Execution(err))
            }
        }
    }

    /// Returns a cloneable command sender for concurrent producer threads.
    pub fn command_sender(&self) -> ExecutionCommandSender {
        self.sender.clone()
    }

    /// Sends a command, blocking when the bounded command queue is full.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn send(&self, command: ExecutionCommand) -> Result<u64, ConcurrentExecutionError> {
        self.sender.send(command)
    }

    /// Attempts to send a command without blocking.
    ///
    /// # Errors
    ///
    /// Returns an error on backpressure or stopped worker.
    pub fn try_send(&self, command: ExecutionCommand) -> Result<u64, ConcurrentExecutionError> {
        self.sender.try_send(command)
    }

    /// Receives the next command report.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::Stopped`] if the report channel is
    /// disconnected.
    pub fn recv_report(&self) -> Result<ExecutionCommandReport, ConcurrentExecutionError> {
        self.reports
            .as_ref()
            .ok_or(ConcurrentExecutionError::Stopped)?
            .recv()
            .map_err(|_: RecvError| ConcurrentExecutionError::Stopped)
    }

    /// Attempts to receive the next command report without blocking.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::Backpressure`] when no report is
    /// available, or [`ConcurrentExecutionError::Stopped`] when the worker has
    /// stopped and the report channel is disconnected.
    pub fn try_recv_report(&self) -> Result<ExecutionCommandReport, ConcurrentExecutionError> {
        let reports = self
            .reports
            .as_ref()
            .ok_or(ConcurrentExecutionError::Stopped)?;
        match reports.try_recv() {
            Ok(report) => Ok(report),
            Err(TryRecvError::Empty) => Err(ConcurrentExecutionError::Backpressure),
            Err(TryRecvError::Disconnected) => Err(ConcurrentExecutionError::Stopped),
        }
    }

    /// Receives the next report with a timeout.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::Backpressure`] on timeout.
    pub fn recv_report_timeout(
        &self,
        timeout: Duration,
    ) -> Result<ExecutionCommandReport, ConcurrentExecutionError> {
        let reports = self
            .reports
            .as_ref()
            .ok_or(ConcurrentExecutionError::Stopped)?;
        match reports.recv_timeout(timeout) {
            Ok(report) => Ok(report),
            Err(RecvTimeoutError::Timeout) => Err(ConcurrentExecutionError::Backpressure),
            Err(RecvTimeoutError::Disconnected) => Err(ConcurrentExecutionError::Stopped),
        }
    }

    /// Requests worker shutdown after previously queued commands complete.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn request_stop(&self) -> Result<u64, ConcurrentExecutionError> {
        self.sender.stop()
    }

    /// Joins the worker thread.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::WorkerPanic`] if the worker panicked.
    pub fn join(&mut self) -> Result<(), ConcurrentExecutionError> {
        if let Some(join) = self.join.take() {
            join.join()
                .map_err(|_| ConcurrentExecutionError::WorkerPanic)?;
        }
        Ok(())
    }
}

impl Drop for ConcurrentExecutionEngine {
    fn drop(&mut self) {
        let _ = self.sender.try_send(ExecutionCommand::Stop);
        drop(self.reports.take());
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn run_execution_worker<A, R, J>(
    mut engine: ExecutionEngine<A, R, J>,
    rx: Receiver<WorkerCommand>,
    report_tx: SyncSender<ExecutionCommandReport>,
    start_tx: SyncSender<ExecutionResult<()>>,
    event_buffer_capacity: usize,
) where
    A: ExecutionAdapter,
    R: RiskCheck,
    J: ExecutionJournal,
{
    let start = engine.start();
    let started = start.is_ok();
    if start_tx.send(start).is_err() || !started {
        return;
    }

    let mut events = ExecutionEventBuffer::with_capacity(event_buffer_capacity);
    while let Ok(worker_command) = rx.recv() {
        events.clear();
        let kind = worker_command.command.kind();
        let result = match worker_command.command {
            ExecutionCommand::Submit(req) => engine.submit(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Cancel(req) => engine.cancel(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Amend(req) => engine.amend(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Poll => engine.poll(&mut events),
            ExecutionCommand::RecoverOpenOrders => engine.recover_open_orders(&mut events),
            ExecutionCommand::Stop => Ok(0),
        };
        let report = ExecutionCommandReport {
            sequence: worker_command.sequence,
            kind,
            result,
            events: events.clone(),
        };
        if report_tx.send(report).is_err() || kind == ExecutionCommandKind::Stop {
            break;
        }
    }
}

/// Execution engine for one adapter and one route set.
pub struct ExecutionEngine<A: ExecutionAdapter, R: RiskCheck, J: ExecutionJournal> {
    adapter: A,
    risk: R,
    journal: J,
    routes: Vec<RouteConfig>,
    route_index: HashMap<RouteKey, usize>,
    orders: HashMap<ClientOrderId, OrderStateMachine>,
    order_prices: HashMap<ClientOrderId, OrderPrice>,
    pending_amend_prices: HashMap<ClientOrderId, OrderPrice>,
    metrics: ExecutionMetrics,
    scratch: ExecutionEventBuffer,
    started: bool,
}

impl<A: ExecutionAdapter, R: RiskCheck, J: ExecutionJournal> ExecutionEngine<A, R, J> {
    /// Creates an execution engine.
    pub fn new(adapter: A, risk: R, journal: J, routes: Vec<RouteConfig>) -> Self {
        let route_index = build_route_index(&routes);
        Self {
            adapter,
            risk,
            journal,
            routes,
            route_index,
            orders: HashMap::new(),
            order_prices: HashMap::new(),
            pending_amend_prices: HashMap::new(),
            metrics: ExecutionMetrics::default(),
            scratch: ExecutionEventBuffer::default(),
            started: false,
        }
    }

    /// Starts the execution adapter.
    pub fn start(&mut self) -> ExecutionResult<()> {
        self.adapter.connect()?;
        self.started = true;
        Ok(())
    }

    /// Returns true when the engine is started.
    pub const fn started(&self) -> bool {
        self.started
    }

    /// Returns execution metrics.
    pub const fn metrics(&self) -> ExecutionMetrics {
        self.metrics
    }

    /// Returns adapter health.
    pub fn health(&self) -> ExecutionHealth {
        self.adapter.health()
    }

    /// Returns a read-only operator runbook summary.
    pub fn runbook_snapshot(&self) -> ExecutionRunbookSnapshot {
        let health = self.adapter.health();
        let enabled_route_count = self.routes.iter().filter(|route| route.enabled).count();
        let kill_switch_route_count = self
            .routes
            .iter()
            .filter(|route| route.enabled && route.risk_limits.kill_switch)
            .count();
        let open_order_count = self
            .orders
            .values()
            .filter(|state| !state.state().status.is_terminal())
            .count();
        let terminal_order_count = self.orders.len().saturating_sub(open_order_count);
        let new_submissions_blocked = !self.started
            || !health.connected
            || enabled_route_count == 0
            || enabled_route_count == kill_switch_route_count;
        let operator_attention_required = !self.started
            || !health.connected
            || health.degraded
            || enabled_route_count == 0
            || kill_switch_route_count > 0
            || self.metrics.adapter_errors > 0;

        ExecutionRunbookSnapshot {
            started: self.started,
            connected: health.connected,
            degraded: health.degraded,
            health_seq: health.health_seq,
            route_count: self.routes.len(),
            enabled_route_count,
            disabled_route_count: self.routes.len().saturating_sub(enabled_route_count),
            kill_switch_route_count,
            open_order_count,
            terminal_order_count,
            submitted: self.metrics.submitted,
            cancelled: self.metrics.cancelled,
            amended: self.metrics.amended,
            events_applied: self.metrics.events_applied,
            risk_rejected: self.metrics.risk_rejected,
            adapter_errors: self.metrics.adapter_errors,
            recovered: self.metrics.recovered,
            new_submissions_blocked,
            operator_attention_required,
        }
    }

    /// Returns an order state by client id.
    pub fn order_state(&self, id: &ClientOrderId) -> Option<OrderState> {
        self.orders.get(id).map(|sm| *sm.state())
    }

    /// Returns all non-terminal local order states.
    pub fn open_order_states(&self) -> Vec<OrderState> {
        self.orders
            .values()
            .map(|sm| *sm.state())
            .filter(|state| !state.status.is_terminal())
            .collect()
    }

    /// Builds a detailed reconciliation report for a venue open-order snapshot.
    pub fn reconcile_open_orders_with(
        &self,
        venue_open_orders: &[OrderState],
    ) -> VenueReconciliationReport {
        let local = self.open_order_states();
        reconcile_open_orders_detailed(&local, venue_open_orders)
    }

    /// Evaluates a venue open-order snapshot against a reconciliation policy.
    pub fn evaluate_reconciliation(
        &self,
        venue_open_orders: &[OrderState],
        policy: ReconciliationPolicy,
    ) -> ReconciliationPolicyDecision {
        let report = self.reconcile_open_orders_with(venue_open_orders);
        evaluate_reconciliation_policy(&report, policy)
    }

    /// Returns journal records.
    pub fn replay_journal(&self, out: &mut Vec<JournalRecord>) -> ExecutionResult<usize> {
        self.journal.replay(out)
    }

    /// Returns configured execution routes.
    pub fn routes(&self) -> &[RouteConfig] {
        &self.routes
    }

    /// Submits an order.
    pub fn submit(
        &mut self,
        req: OrderRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        if !self.started {
            return Err(ExecutionError::Disconnected);
        }
        req.validate()?;

        let Some(route) = self
            .find_route(req.route_id, req.account_id, req.symbol)
            .copied()
        else {
            return Err(ExecutionError::RouteNotFound);
        };
        let caps = self.adapter.capabilities();
        let ctx = self.risk_context(
            &req.client_order_id,
            &route,
            caps.supports_order_type(req.order_type),
            caps.supports_tif(req.time_in_force),
        );
        let decision = self.check_route_new(&req, &route, &ctx);
        if !decision.allowed {
            self.metrics.risk_rejected = self.metrics.risk_rejected.saturating_add(1);
            let event = ExecutionEvent::rejected(&req, decision.reason, decision.text);
            self.journal.record_event(&event)?;
            out.push(event)?;
            return Err(ExecutionError::RiskRejected(decision.reason));
        }
        let decision = self.risk.check_new(&req, &ctx);
        if !decision.allowed {
            self.metrics.risk_rejected = self.metrics.risk_rejected.saturating_add(1);
            let event = ExecutionEvent::rejected(&req, decision.reason, decision.text);
            self.journal.record_event(&event)?;
            out.push(event)?;
            return Err(ExecutionError::RiskRejected(decision.reason));
        }

        self.journal.record_command(
            JournalCommandKind::Submit,
            req.client_order_id,
            req.ts_recv_ns,
        )?;
        self.orders
            .insert(req.client_order_id, OrderStateMachine::new(&req));
        self.order_prices.insert(
            req.client_order_id,
            execution_price(req.limit_price, req.stop_price),
        );
        self.scratch.clear();
        self.adapter.submit(&req, &mut self.scratch)?;
        self.metrics.submitted = self.metrics.submitted.saturating_add(1);
        self.apply_scratch(out)?;
        Ok(())
    }

    /// Cancels an order.
    pub fn cancel(
        &mut self,
        req: CancelRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        if !self.started {
            return Err(ExecutionError::Disconnected);
        }
        if !self.orders.contains_key(&req.orig_client_order_id) {
            return Err(ExecutionError::RouteNotFound);
        }
        let Some(route) = self
            .find_route(req.route_id, req.account_id, req.symbol)
            .copied()
        else {
            return Err(ExecutionError::RouteNotFound);
        };
        let ctx = self.risk_context(&req.client_order_id, &route, true, true);
        let decision = self.risk.check_cancel(&req, &ctx);
        if !decision.allowed {
            self.metrics.risk_rejected = self.metrics.risk_rejected.saturating_add(1);
            return Err(ExecutionError::RiskRejected(decision.reason));
        }
        self.journal.record_command(
            JournalCommandKind::Cancel,
            req.client_order_id,
            req.ts_recv_ns,
        )?;
        self.scratch.clear();
        self.adapter.cancel(&req, &mut self.scratch)?;
        self.metrics.cancelled = self.metrics.cancelled.saturating_add(1);
        self.apply_scratch(out)?;
        Ok(())
    }

    /// Amends an order.
    pub fn amend(
        &mut self,
        req: AmendRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        if !self.started {
            return Err(ExecutionError::Disconnected);
        }
        if req.quantity.0 <= 0 {
            return Err(ExecutionError::Core(ExecutionCoreError::InvalidQuantity));
        }
        if !self.orders.contains_key(&req.orig_client_order_id) {
            return Err(ExecutionError::RouteNotFound);
        }
        let Some(route) = self
            .find_route(req.route_id, req.account_id, req.symbol)
            .copied()
        else {
            return Err(ExecutionError::RouteNotFound);
        };
        let ctx = self.risk_context(&req.client_order_id, &route, true, true);
        let decision = self.check_route_amend(&req, &route, &ctx);
        if !decision.allowed {
            self.metrics.risk_rejected = self.metrics.risk_rejected.saturating_add(1);
            return Err(ExecutionError::RiskRejected(decision.reason));
        }
        let decision = self.risk.check_amend(&req, &ctx);
        if !decision.allowed {
            self.metrics.risk_rejected = self.metrics.risk_rejected.saturating_add(1);
            return Err(ExecutionError::RiskRejected(decision.reason));
        }
        self.journal.record_command(
            JournalCommandKind::Amend,
            req.client_order_id,
            req.ts_recv_ns,
        )?;
        self.pending_amend_prices
            .insert(req.client_order_id, req.limit_price);
        self.scratch.clear();
        self.adapter.amend(&req, &mut self.scratch)?;
        self.metrics.amended = self.metrics.amended.saturating_add(1);
        self.apply_scratch(out)?;
        Ok(())
    }

    /// Polls adapter events and applies them to local state.
    pub fn poll(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        if !self.started {
            return Err(ExecutionError::Disconnected);
        }
        self.scratch.clear();
        let _ = self.adapter.poll(&mut self.scratch)?;
        self.apply_scratch(out)
    }

    /// Recovers open orders from the adapter.
    pub fn recover_open_orders(
        &mut self,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<usize> {
        if !self.started {
            return Err(ExecutionError::Disconnected);
        }
        self.scratch.clear();
        let _ = self.adapter.recover_open_orders(&mut self.scratch)?;
        let count = self.apply_scratch(out)?;
        self.metrics.recovered = self.metrics.recovered.saturating_add(count as u64);
        Ok(count)
    }

    fn find_route(
        &self,
        route_id: RouteId,
        account_id: AccountId,
        symbol: ExecutionSymbol,
    ) -> Option<&RouteConfig> {
        let key = RouteKey::new(route_id, account_id, symbol);
        self.route_index
            .get(&key)
            .and_then(|index| self.routes.get(*index))
            .filter(|route| route.enabled)
    }

    fn risk_context(
        &self,
        id: &ClientOrderId,
        route: &RouteConfig,
        order_type_supported: bool,
        tif_supported: bool,
    ) -> RiskContext {
        RiskContext {
            open_orders: self.scoped_open_orders(route),
            open_notional: self.scoped_open_notional(route),
            reference_price: OrderPrice(0),
            duplicate_client_order_id: self.orders.contains_key(id),
            account_enabled: route.enabled,
            route_enabled: route.enabled,
            symbol_enabled: route.enabled,
            order_type_supported,
            tif_supported,
        }
    }

    fn scoped_open_orders(&self, route: &RouteConfig) -> u32 {
        self.orders
            .values()
            .filter(|sm| !sm.state().status.is_terminal())
            .filter(|sm| state_matches_route(sm.state(), route))
            .count() as u32
    }

    fn scoped_open_notional(&self, route: &RouteConfig) -> i128 {
        self.orders
            .values()
            .filter(|sm| !sm.state().status.is_terminal())
            .filter(|sm| state_matches_route(sm.state(), route))
            .map(|sm| self.open_state_notional(sm.state()))
            .sum()
    }

    fn open_state_notional(&self, state: &OrderState) -> i128 {
        let price = self
            .order_prices
            .get(&state.client_order_id)
            .or_else(|| self.order_prices.get(&state.last_accepted_client_order_id))
            .copied()
            .unwrap_or(state.average_price);
        i128::from(state.leaves_qty.0).saturating_mul(i128::from(price.0))
    }

    fn check_route_new(
        &self,
        req: &OrderRequest,
        route: &RouteConfig,
        ctx: &RiskContext,
    ) -> of_execution_core::RiskDecision {
        self.check_route_common(&route.risk_limits, ctx)
            .or_else(|| {
                self.check_route_size_price(&route.risk_limits, req.quantity, req.limit_price, ctx)
            })
            .unwrap_or_else(of_execution_core::RiskDecision::allow)
    }

    fn check_route_amend(
        &self,
        req: &AmendRequest,
        route: &RouteConfig,
        ctx: &RiskContext,
    ) -> of_execution_core::RiskDecision {
        self.check_route_common(&route.risk_limits, ctx)
            .or_else(|| {
                self.check_route_size_price(&route.risk_limits, req.quantity, req.limit_price, ctx)
            })
            .unwrap_or_else(of_execution_core::RiskDecision::allow)
    }

    fn check_route_common(
        &self,
        limits: &RiskLimits,
        ctx: &RiskContext,
    ) -> Option<of_execution_core::RiskDecision> {
        if limits.kill_switch {
            return Some(route_reject(
                RiskRejectReason::KillSwitch,
                "kill switch active",
            ));
        }
        if !ctx.account_enabled {
            return Some(route_reject(
                RiskRejectReason::AccountDisabled,
                "account disabled",
            ));
        }
        if !ctx.route_enabled {
            return Some(route_reject(
                RiskRejectReason::RouteDisabled,
                "route disabled",
            ));
        }
        if !ctx.symbol_enabled {
            return Some(route_reject(
                RiskRejectReason::SymbolDisabled,
                "symbol disabled",
            ));
        }
        if ctx.duplicate_client_order_id {
            return Some(route_reject(
                RiskRejectReason::DuplicateClientOrderId,
                "duplicate client order id",
            ));
        }
        if !ctx.order_type_supported {
            return Some(route_reject(
                RiskRejectReason::UnsupportedOrderType,
                "unsupported order type",
            ));
        }
        if !ctx.tif_supported {
            return Some(route_reject(
                RiskRejectReason::UnsupportedTimeInForce,
                "unsupported time in force",
            ));
        }
        if limits.max_open_orders > 0 && ctx.open_orders >= limits.max_open_orders {
            return Some(route_reject(
                RiskRejectReason::MaxOpenOrders,
                "max open orders exceeded",
            ));
        }
        None
    }

    fn check_route_size_price(
        &self,
        limits: &RiskLimits,
        qty: OrderQty,
        price: OrderPrice,
        ctx: &RiskContext,
    ) -> Option<of_execution_core::RiskDecision> {
        if limits.max_order_qty > 0 && qty.0 > limits.max_order_qty {
            return Some(route_reject(
                RiskRejectReason::MaxOrderQty,
                "max order quantity exceeded",
            ));
        }
        let notional = i128::from(qty.0).saturating_mul(i128::from(price.0));
        if limits.max_order_notional > 0 && notional > limits.max_order_notional {
            return Some(route_reject(
                RiskRejectReason::MaxOrderNotional,
                "max order notional exceeded",
            ));
        }
        if limits.max_open_notional > 0
            && ctx.open_notional.saturating_add(notional) > limits.max_open_notional
        {
            return Some(route_reject(
                RiskRejectReason::MaxOpenNotional,
                "max open notional exceeded",
            ));
        }
        if limits.price_band_ticks > 0 && ctx.reference_price.0 > 0 && price.0 > 0 {
            let distance = price.0.saturating_sub(ctx.reference_price.0).abs();
            if distance > limits.price_band_ticks {
                return Some(route_reject(
                    RiskRejectReason::PriceBand,
                    "price outside risk band",
                ));
            }
        }
        None
    }

    fn apply_scratch(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        let mut applied = 0;
        let mut events = Vec::new();
        std::mem::swap(&mut events, &mut self.scratch.events);
        for event in events.drain(..) {
            self.apply_event(event)?;
            out.push(event)?;
            applied += 1;
        }
        std::mem::swap(&mut events, &mut self.scratch.events);
        Ok(applied)
    }

    fn apply_event(&mut self, event: ExecutionEvent) -> ExecutionResult<()> {
        let key = if !event.orig_client_order_id.is_empty()
            && matches!(
                event.exec_type,
                ExecutionType::CancelPending
                    | ExecutionType::CancelAck
                    | ExecutionType::CancelReject
                    | ExecutionType::ReplacePending
                    | ExecutionType::ReplaceAck
                    | ExecutionType::ReplaceReject
            ) {
            event.orig_client_order_id
        } else {
            event.client_order_id
        };

        if let Some(sm) = self.orders.get_mut(&key) {
            sm.apply(&event)?;
            if matches!(event.exec_type, ExecutionType::ReplaceAck) {
                let replaced = *sm;
                self.orders.remove(&key);
                self.orders.insert(event.client_order_id, replaced);
                let price = self
                    .pending_amend_prices
                    .remove(&event.client_order_id)
                    .unwrap_or(event.average_price);
                self.order_prices.remove(&key);
                self.order_prices.insert(event.client_order_id, price);
            }
        } else if matches!(event.exec_type, ExecutionType::Restated) {
            self.orders.insert(
                event.client_order_id,
                OrderStateMachine::new(&OrderRequest {
                    client_order_id: event.client_order_id,
                    account_id: event.account_id,
                    route_id: event.route_id,
                    strategy_id: Default::default(),
                    symbol: event.symbol,
                    side: of_execution_core::OrderSide::Buy,
                    order_type: OrderType::Limit,
                    time_in_force: TimeInForce::Day,
                    quantity: OrderQty(event.cumulative_qty.0 + event.leaves_qty.0),
                    limit_price: event.average_price,
                    stop_price: OrderPrice(0),
                    ts_exchange_ns: event.ts_exchange_ns,
                    ts_recv_ns: event.ts_recv_ns,
                }),
            );
            self.order_prices
                .insert(event.client_order_id, event.average_price);
        }
        self.journal.record_event(&event)?;
        self.metrics.events_applied = self.metrics.events_applied.saturating_add(1);
        Ok(())
    }
}

/// Deterministic simulated execution adapter.
#[derive(Debug, Clone)]
pub struct SimExecutionAdapter {
    connected: bool,
    health_seq: u64,
    next_venue_order_id: u64,
    next_execution_id: u64,
    queue: ExecutionEventBuffer,
    partial_fill: bool,
}

impl Default for SimExecutionAdapter {
    fn default() -> Self {
        Self {
            connected: false,
            health_seq: 0,
            next_venue_order_id: 1,
            next_execution_id: 1,
            queue: ExecutionEventBuffer::with_capacity(1024),
            partial_fill: false,
        }
    }
}

impl SimExecutionAdapter {
    /// Enables or disables deterministic partial fills.
    pub fn with_partial_fill(mut self, enabled: bool) -> Self {
        self.partial_fill = enabled;
        self
    }

    fn venue_order_id(&mut self) -> VenueOrderId {
        let raw = format!("SIM-{}", self.next_venue_order_id);
        self.next_venue_order_id = self.next_venue_order_id.saturating_add(1);
        VenueOrderId::new(&raw).unwrap_or_default()
    }

    fn execution_id(&mut self) -> ExecutionId {
        let raw = format!("SIMX-{}", self.next_execution_id);
        self.next_execution_id = self.next_execution_id.saturating_add(1);
        ExecutionId::new(&raw).unwrap_or_default()
    }

    fn fill_event(&mut self, req: &OrderRequest, venue_order_id: VenueOrderId) -> ExecutionEvent {
        let fill_qty = if self.partial_fill && req.quantity.0 > 1 {
            OrderQty(req.quantity.0 / 2)
        } else {
            req.quantity
        };
        let leaves = OrderQty(req.quantity.0.saturating_sub(fill_qty.0));
        ExecutionEvent {
            exec_type: ExecutionType::Trade,
            order_status: if leaves.0 == 0 {
                OrderStatus::Filled
            } else {
                OrderStatus::PartiallyFilled
            },
            client_order_id: req.client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id,
            execution_id: self.execution_id(),
            account_id: req.account_id,
            route_id: req.route_id,
            symbol: req.symbol,
            last_qty: fill_qty,
            last_price: req.limit_price,
            cumulative_qty: fill_qty,
            leaves_qty: leaves,
            average_price: req.limit_price,
            ts_exchange_ns: req.ts_exchange_ns,
            ts_recv_ns: req.ts_recv_ns.saturating_add(1),
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        }
    }
}

impl ExecutionAdapter for SimExecutionAdapter {
    fn connect(&mut self) -> ExecutionResult<()> {
        self.connected = true;
        self.health_seq = self.health_seq.saturating_add(1);
        Ok(())
    }

    fn submit(
        &mut self,
        req: &OrderRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        if !self.connected {
            return Err(ExecutionError::Disconnected);
        }
        let venue_order_id = self.venue_order_id();
        out.push(ExecutionEvent::accepted(req, venue_order_id))?;
        out.push(self.fill_event(req, venue_order_id))
    }

    fn cancel(
        &mut self,
        req: &CancelRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        if !self.connected {
            return Err(ExecutionError::Disconnected);
        }
        out.push(ExecutionEvent {
            exec_type: ExecutionType::CancelAck,
            order_status: OrderStatus::Cancelled,
            client_order_id: req.client_order_id,
            orig_client_order_id: req.orig_client_order_id,
            venue_order_id: req.venue_order_id,
            execution_id: ExecutionId::empty(),
            account_id: req.account_id,
            route_id: req.route_id,
            symbol: req.symbol,
            last_qty: OrderQty(0),
            last_price: OrderPrice(0),
            cumulative_qty: OrderQty(0),
            leaves_qty: OrderQty(0),
            average_price: OrderPrice(0),
            ts_exchange_ns: 0,
            ts_recv_ns: req.ts_recv_ns,
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        })
    }

    fn amend(&mut self, req: &AmendRequest, out: &mut ExecutionEventBuffer) -> ExecutionResult<()> {
        if !self.connected {
            return Err(ExecutionError::Disconnected);
        }
        out.push(ExecutionEvent {
            exec_type: ExecutionType::ReplaceAck,
            order_status: OrderStatus::Replaced,
            client_order_id: req.client_order_id,
            orig_client_order_id: req.orig_client_order_id,
            venue_order_id: req.venue_order_id,
            execution_id: ExecutionId::empty(),
            account_id: req.account_id,
            route_id: req.route_id,
            symbol: req.symbol,
            last_qty: OrderQty(0),
            last_price: OrderPrice(0),
            cumulative_qty: OrderQty(0),
            leaves_qty: req.quantity,
            average_price: req.limit_price,
            ts_exchange_ns: 0,
            ts_recv_ns: req.ts_recv_ns,
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        })
    }

    fn poll(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        self.queue.drain_into(out)
    }

    fn recover_open_orders(&mut self, _out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        Ok(0)
    }

    fn capabilities(&self) -> ExecutionCapabilities {
        ExecutionCapabilities::simulated()
    }

    fn health(&self) -> ExecutionHealth {
        ExecutionHealth {
            connected: self.connected,
            degraded: false,
            health_seq: self.health_seq,
            last_error: None,
            protocol_info: Some("simulated".to_string()),
        }
    }
}

/// Creates a one-route simulated execution engine.
pub fn simulated_engine(
    route: RouteConfig,
) -> ExecutionEngine<SimExecutionAdapter, BasicRiskGate, InMemoryJournal> {
    ExecutionEngine::new(
        SimExecutionAdapter::default(),
        BasicRiskGate::new(route.risk_limits),
        InMemoryJournal::default(),
        vec![route],
    )
}

/// Creates a simulated execution engine for multiple configured routes.
pub fn simulated_engine_with_routes(
    routes: Vec<RouteConfig>,
) -> ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal> {
    ExecutionEngine::new(
        SimExecutionAdapter::default(),
        AllowAllRiskGate,
        InMemoryJournal::default(),
        routes,
    )
}

fn build_route_index(routes: &[RouteConfig]) -> HashMap<RouteKey, usize> {
    let mut index = HashMap::with_capacity(routes.len());
    for (position, route) in routes.iter().enumerate() {
        index.insert(
            RouteKey::new(route.route_id, route.account_id, route.symbol),
            position,
        );
    }
    index
}

fn state_matches_route(state: &OrderState, route: &RouteConfig) -> bool {
    state.route_id == route.route_id
        && state.account_id == route.account_id
        && state.symbol == route.symbol
}

fn execution_price(limit_price: OrderPrice, stop_price: OrderPrice) -> OrderPrice {
    if limit_price.0 > 0 {
        limit_price
    } else {
        stop_price
    }
}

fn route_reject(reason: RiskRejectReason, text: &str) -> of_execution_core::RiskDecision {
    let text = ExecutionText::new(text).unwrap_or_else(|_| ExecutionText::empty());
    of_execution_core::RiskDecision::reject(reason, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use of_execution_core::{ExecutionSymbol, FixedAscii, OrderSide};

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn route() -> RouteConfig {
        route_for("ES", 10)
    }

    fn route_for(instrument: &str, max_open_orders: u32) -> RouteConfig {
        RouteConfig {
            route_id: id("SIM"),
            account_id: id("ACC"),
            symbol: ExecutionSymbol::new("SIM", instrument).unwrap(),
            enabled: true,
            risk_limits: RiskLimits {
                kill_switch: false,
                max_order_qty: 100,
                max_order_notional: 1_000_000,
                max_open_orders,
                max_open_notional: 10_000_000,
                price_band_ticks: 0,
            },
        }
    }

    fn order() -> OrderRequest {
        order_for("C1", "ES")
    }

    fn order_for(client_order_id: &str, instrument: &str) -> OrderRequest {
        OrderRequest {
            client_order_id: id(client_order_id),
            account_id: id("ACC"),
            route_id: id("SIM"),
            strategy_id: id("STRAT"),
            symbol: ExecutionSymbol::new("SIM", instrument).unwrap(),
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

    #[test]
    fn simulated_engine_submits_and_fills() {
        let mut engine = simulated_engine(route());
        let stopped = engine.runbook_snapshot();
        assert!(!stopped.started);
        assert!(stopped.new_submissions_blocked);
        assert!(stopped.operator_attention_required);

        engine.start().unwrap();
        let started = engine.runbook_snapshot();
        assert!(started.started);
        assert!(started.connected);
        assert_eq!(started.route_count, 1);
        assert_eq!(started.enabled_route_count, 1);
        assert_eq!(started.kill_switch_route_count, 0);
        assert!(started.can_submit_new_orders());
        assert!(!started.operator_attention_required);

        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order(), &mut out).unwrap();

        assert_eq!(out.len(), 2);
        assert_eq!(out.as_slice()[0].exec_type, ExecutionType::Ack);
        assert_eq!(out.as_slice()[1].exec_type, ExecutionType::Trade);
        assert_eq!(
            engine.order_state(&id("C1")).unwrap().status,
            OrderStatus::Filled
        );
        assert_eq!(engine.metrics().submitted, 1);
        assert_eq!(engine.metrics().events_applied, 2);
        let after_fill = engine.runbook_snapshot();
        assert_eq!(after_fill.submitted, 1);
        assert_eq!(after_fill.events_applied, 2);
        assert_eq!(after_fill.open_order_count, 0);
        assert_eq!(after_fill.terminal_order_count, 1);
    }

    #[test]
    fn runbook_snapshot_marks_killed_routes_without_mutating_engine() {
        let mut killed = route();
        killed.risk_limits.kill_switch = true;
        let mut engine = simulated_engine(killed);
        engine.start().unwrap();

        let snapshot = engine.runbook_snapshot();
        assert!(snapshot.started);
        assert!(snapshot.connected);
        assert_eq!(snapshot.kill_switch_route_count, 1);
        assert!(snapshot.new_submissions_blocked);
        assert!(!snapshot.can_submit_new_orders());
        assert!(snapshot.operator_attention_required);
    }

    #[test]
    fn risk_rejects_duplicate_client_order_id() {
        let mut engine = simulated_engine(route());
        engine.start().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order(), &mut out).unwrap();
        out.clear();

        let err = engine.submit(order(), &mut out).unwrap_err();
        assert!(matches!(err, ExecutionError::RiskRejected(_)));
        assert_eq!(
            out.as_slice()[0].reason,
            RiskRejectReason::DuplicateClientOrderId
        );
    }

    #[test]
    fn buffer_bound_is_enforced() {
        let mut buffer = ExecutionEventBuffer::with_capacity(0);
        let req = order();
        let event = ExecutionEvent::accepted(&req, id("V1"));
        assert_eq!(buffer.push(event).unwrap_err(), ExecutionError::BufferFull);
    }

    #[test]
    fn journal_records_commands_and_events() {
        let mut engine = simulated_engine(route());
        engine.start().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order(), &mut out).unwrap();

        let mut records = Vec::new();
        let count = engine.replay_journal(&mut records).unwrap();
        assert_eq!(count, 3);
        assert!(matches!(
            records[0],
            JournalRecord::Command {
                kind: JournalCommandKind::Submit,
                ..
            }
        ));
    }

    #[test]
    fn multi_route_engine_submits_multiple_symbols() {
        let routes = vec![route_for("ES", 10), route_for("NQ", 10)];
        let mut engine = simulated_engine_with_routes(routes);
        engine.start().unwrap();

        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order_for("ES-1", "ES"), &mut out).unwrap();
        out.clear();
        engine.submit(order_for("NQ-1", "NQ"), &mut out).unwrap();

        assert_eq!(
            engine.order_state(&id("ES-1")).unwrap().status,
            OrderStatus::Filled
        );
        assert_eq!(
            engine.order_state(&id("NQ-1")).unwrap().status,
            OrderStatus::Filled
        );
        assert_eq!(engine.metrics().submitted, 2);
    }

    #[test]
    fn route_limits_are_scoped_by_symbol() {
        let routes = vec![route_for("ES", 1), route_for("NQ", 1)];
        let mut engine = ExecutionEngine::new(
            SimExecutionAdapter::default().with_partial_fill(true),
            AllowAllRiskGate,
            InMemoryJournal::default(),
            routes,
        );
        engine.start().unwrap();

        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order_for("ES-1", "ES"), &mut out).unwrap();
        out.clear();
        engine.submit(order_for("NQ-1", "NQ"), &mut out).unwrap();
        out.clear();

        let err = engine
            .submit(order_for("ES-2", "ES"), &mut out)
            .unwrap_err();
        assert!(matches!(
            err,
            ExecutionError::RiskRejected(RiskRejectReason::MaxOpenOrders)
        ));
        assert_eq!(out.as_slice()[0].reason, RiskRejectReason::MaxOpenOrders);
    }

    #[test]
    fn engine_evaluates_venue_reconciliation_policy() {
        let mut engine = ExecutionEngine::new(
            SimExecutionAdapter::default().with_partial_fill(true),
            AllowAllRiskGate,
            InMemoryJournal::default(),
            vec![route()],
        );
        engine.start().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order(), &mut out).unwrap();

        let mut venue = engine.open_order_states();
        assert_eq!(venue.len(), 1);
        venue[0].status = OrderStatus::Cancelled;

        let report = engine.reconcile_open_orders_with(&venue);
        assert_eq!(report.details.len(), 1);
        assert_eq!(
            report.details[0].issue,
            ReconciliationIssueKind::StatusMismatch
        );

        let decision = engine.evaluate_reconciliation(
            &venue,
            ReconciliationPolicy::fail_closed()
                .with_status_mismatch(ReconciliationPolicyAction::AcceptVenueTruth),
        );
        assert!(!decision.submissions_enabled);
        assert!(decision.local_restates_required);
        assert_eq!(
            decision.items[0].action,
            ReconciliationPolicyAction::AcceptVenueTruth
        );
    }

    #[test]
    fn concurrent_worker_processes_multiple_producers_serially() {
        let routes = vec![route_for("ES", 10), route_for("NQ", 10)];
        let engine = simulated_engine_with_routes(routes);
        let mut concurrent =
            ConcurrentExecutionEngine::spawn(engine, ConcurrentExecutionConfig::default()).unwrap();
        let es_sender = concurrent.command_sender();
        let nq_sender = concurrent.command_sender();

        let es_thread =
            std::thread::spawn(move || es_sender.submit(order_for("ES-C", "ES")).unwrap());
        let nq_thread =
            std::thread::spawn(move || nq_sender.submit(order_for("NQ-C", "NQ")).unwrap());
        let es_seq = es_thread.join().unwrap();
        let nq_seq = nq_thread.join().unwrap();

        let first = concurrent.recv_report().unwrap();
        let second = concurrent.recv_report().unwrap();
        assert!(first.result.is_ok());
        assert!(second.result.is_ok());
        assert_eq!(first.events.len(), 2);
        assert_eq!(second.events.len(), 2);
        assert!(first.sequence == es_seq || second.sequence == es_seq);
        assert!(first.sequence == nq_seq || second.sequence == nq_seq);

        concurrent.request_stop().unwrap();
        let stop = concurrent.recv_report().unwrap();
        assert_eq!(stop.kind, ExecutionCommandKind::Stop);
        assert!(stop.result.is_ok());
        concurrent.join().unwrap();
    }

    #[test]
    fn concurrent_worker_reports_risk_rejection() {
        let routes = vec![route_for("ES", 1)];
        let engine = ExecutionEngine::new(
            SimExecutionAdapter::default().with_partial_fill(true),
            AllowAllRiskGate,
            InMemoryJournal::default(),
            routes,
        );
        let mut concurrent =
            ConcurrentExecutionEngine::spawn(engine, ConcurrentExecutionConfig::default()).unwrap();

        concurrent
            .send(ExecutionCommand::Submit(order_for("ES-OPEN", "ES")))
            .unwrap();
        assert!(concurrent.recv_report().unwrap().result.is_ok());
        concurrent
            .send(ExecutionCommand::Submit(order_for("ES-REJECT", "ES")))
            .unwrap();
        let report = concurrent.recv_report().unwrap();

        assert!(matches!(
            report.result,
            Err(ExecutionError::RiskRejected(
                RiskRejectReason::MaxOpenOrders
            ))
        ));
        assert_eq!(report.events.len(), 1);
        assert_eq!(
            report.events.as_slice()[0].reason,
            RiskRejectReason::MaxOpenOrders
        );

        concurrent.request_stop().unwrap();
        let _ = concurrent.recv_report().unwrap();
        concurrent.join().unwrap();
    }
}
