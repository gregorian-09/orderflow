use super::*;

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

pub(crate) fn build_route_index(routes: &[RouteConfig]) -> HashMap<RouteKey, usize> {
    let mut index = HashMap::with_capacity(routes.len());
    for (position, route) in routes.iter().enumerate() {
        index.insert(
            RouteKey::new(route.route_id, route.account_id, route.symbol),
            position,
        );
    }
    index
}

pub(crate) fn state_matches_route(state: &OrderState, route: &RouteConfig) -> bool {
    state.route_id == route.route_id
        && state.account_id == route.account_id
        && state.symbol == route.symbol
}

pub(crate) fn execution_price(limit_price: OrderPrice, stop_price: OrderPrice) -> OrderPrice {
    if limit_price.0 > 0 {
        limit_price
    } else {
        stop_price
    }
}

pub(crate) fn route_reject(
    reason: RiskRejectReason,
    text: &str,
) -> of_execution_core::RiskDecision {
    let text = ExecutionText::new(text).unwrap_or_else(|_| ExecutionText::empty());
    of_execution_core::RiskDecision::reject(reason, text)
}

pub(crate) fn unix_ts_nanos() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    u64::try_from(nanos).unwrap_or(u64::MAX)
}
