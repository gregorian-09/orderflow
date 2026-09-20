use super::*;

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
    pub(crate) events: Vec<ExecutionEvent>,
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
    /// Records a new-order request with its recovery payload.
    ///
    /// The default preserves the original command-only journal contract so
    /// existing third-party journal implementations continue to compile.
    /// Durable journals may override this hook to retain the full request.
    fn record_submit(&mut self, request: &OrderRequest) -> ExecutionResult<()> {
        self.record_command(
            JournalCommandKind::Submit,
            request.client_order_id,
            request.ts_recv_ns,
        )
    }
    /// Records a cancel request with its recovery payload.
    ///
    /// The default delegates to [`ExecutionJournal::record_command`].
    fn record_cancel(&mut self, request: &CancelRequest) -> ExecutionResult<()> {
        self.record_command(
            JournalCommandKind::Cancel,
            request.client_order_id,
            request.ts_recv_ns,
        )
    }
    /// Records an amend request with its recovery payload.
    ///
    /// The default delegates to [`ExecutionJournal::record_command`].
    fn record_amend(&mut self, request: &AmendRequest) -> ExecutionResult<()> {
        self.record_command(
            JournalCommandKind::Amend,
            request.client_order_id,
            request.ts_recv_ns,
        )
    }
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
    /// True when an operator has paused all new submissions.
    pub submissions_paused: bool,
    /// Number of configured routes currently draining.
    pub draining_route_count: usize,
    /// Number of configured routes marked degraded by operator control.
    pub degraded_route_count: usize,
    /// Enabled routes currently eligible for new submissions.
    pub available_route_count: usize,
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

/// Read-only manifest describing the current execution incident bundle inputs.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExecutionAuditBundleManifest {
    /// Manifest schema version.
    pub schema_version: u16,
    /// Manifest creation timestamp in Unix nanoseconds.
    pub generated_ns: u64,
    /// Operator runbook snapshot captured with the manifest.
    pub runbook: ExecutionRunbookSnapshot,
    /// Configured route count.
    pub route_count: usize,
    /// Enabled route count.
    pub enabled_route_count: usize,
    /// Locally known non-terminal order count.
    pub open_order_count: usize,
    /// Locally known terminal order count.
    pub terminal_order_count: usize,
    /// Journal records visible through the configured journal.
    pub journal_record_count: usize,
    /// Journal command records visible through the configured journal.
    pub journal_command_count: usize,
    /// Journal execution-event records visible through the configured journal.
    pub journal_event_count: usize,
    /// Execution metrics captured with the manifest.
    pub metrics: ExecutionMetrics,
    /// True when the engine can accept at least one new-order route.
    pub submissions_enabled: bool,
    /// True when an operator should inspect the bundle before resuming flow.
    pub operator_attention_required: bool,
}

impl ExecutionAuditBundleManifest {
    /// Current manifest schema version.
    pub const SCHEMA_VERSION: u16 = 1;

    /// Returns true when the manifest indicates operator review is needed.
    pub const fn requires_operator_review(self) -> bool {
        self.operator_attention_required
    }
}
