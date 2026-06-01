//! Execution routing and adapter contracts for Orderflow.
#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

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

/// Execution engine for one adapter and one route set.
pub struct ExecutionEngine<A: ExecutionAdapter, R: RiskCheck, J: ExecutionJournal> {
    adapter: A,
    risk: R,
    journal: J,
    routes: Vec<RouteConfig>,
    orders: HashMap<ClientOrderId, OrderStateMachine>,
    metrics: ExecutionMetrics,
    scratch: ExecutionEventBuffer,
    started: bool,
}

impl<A: ExecutionAdapter, R: RiskCheck, J: ExecutionJournal> ExecutionEngine<A, R, J> {
    /// Creates an execution engine.
    pub fn new(adapter: A, risk: R, journal: J, routes: Vec<RouteConfig>) -> Self {
        Self {
            adapter,
            risk,
            journal,
            routes,
            orders: HashMap::new(),
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

    /// Returns an order state by client id.
    pub fn order_state(&self, id: &ClientOrderId) -> Option<OrderState> {
        self.orders.get(id).map(|sm| *sm.state())
    }

    /// Returns journal records.
    pub fn replay_journal(&self, out: &mut Vec<JournalRecord>) -> ExecutionResult<usize> {
        self.journal.replay(out)
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

        let Some(route) = self.find_route(req.route_id, req.account_id, req.symbol) else {
            return Err(ExecutionError::RouteNotFound);
        };
        let caps = self.adapter.capabilities();
        let ctx = self.risk_context(
            &req.client_order_id,
            route,
            caps.supports_order_type(req.order_type),
            caps.supports_tif(req.time_in_force),
        );
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
        self.journal.record_command(
            JournalCommandKind::Amend,
            req.client_order_id,
            req.ts_recv_ns,
        )?;
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
        self.routes.iter().find(|route| {
            route.enabled
                && route.route_id == route_id
                && route.account_id == account_id
                && route.symbol == symbol
        })
    }

    fn risk_context(
        &self,
        id: &ClientOrderId,
        route: &RouteConfig,
        order_type_supported: bool,
        tif_supported: bool,
    ) -> RiskContext {
        RiskContext {
            open_orders: self
                .orders
                .values()
                .filter(|sm| !sm.state().status.is_terminal())
                .count() as u32,
            open_notional: self.open_notional(),
            reference_price: OrderPrice(0),
            duplicate_client_order_id: self.orders.contains_key(id),
            account_enabled: route.enabled,
            route_enabled: route.enabled,
            symbol_enabled: route.enabled,
            order_type_supported,
            tif_supported,
        }
    }

    fn open_notional(&self) -> i128 {
        self.orders
            .values()
            .filter(|sm| !sm.state().status.is_terminal())
            .map(|sm| {
                i128::from(sm.state().leaves_qty.0)
                    .saturating_mul(i128::from(sm.state().average_price.0))
            })
            .sum()
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

#[cfg(test)]
mod tests {
    use super::*;
    use of_execution_core::{ExecutionSymbol, FixedAscii, OrderSide};

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn route() -> RouteConfig {
        RouteConfig {
            route_id: id("SIM"),
            account_id: id("ACC"),
            symbol: ExecutionSymbol::new("SIM", "ES").unwrap(),
            enabled: true,
            risk_limits: RiskLimits {
                kill_switch: false,
                max_order_qty: 100,
                max_order_notional: 1_000_000,
                max_open_orders: 10,
                max_open_notional: 10_000_000,
                price_band_ticks: 0,
            },
        }
    }

    fn order() -> OrderRequest {
        OrderRequest {
            client_order_id: id("C1"),
            account_id: id("ACC"),
            route_id: id("SIM"),
            strategy_id: id("STRAT"),
            symbol: ExecutionSymbol::new("SIM", "ES").unwrap(),
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
        engine.start().unwrap();
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
}
