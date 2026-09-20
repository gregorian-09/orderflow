use super::*;

/// Execution engine for one adapter and one route set.
pub struct ExecutionEngine<A: ExecutionAdapter, R: RiskCheck, J: ExecutionJournal> {
    pub(crate) adapter: A,
    pub(crate) risk: R,
    pub(crate) journal: J,
    pub(crate) routes: Vec<RouteConfig>,
    pub(crate) route_index: HashMap<RouteKey, usize>,
    pub(crate) orders: HashMap<ClientOrderId, OrderStateMachine>,
    pub(crate) order_prices: HashMap<ClientOrderId, OrderPrice>,
    pub(crate) pending_amend_prices: HashMap<ClientOrderId, OrderPrice>,
    pub(crate) order_strategies: HashMap<ClientOrderId, of_execution_core::StrategyId>,
    pub(crate) operator_submissions_paused: bool,
    pub(crate) operator_draining_routes: HashSet<RouteKey>,
    pub(crate) operator_degraded_routes: HashSet<RouteKey>,
    pub(crate) metrics: ExecutionMetrics,
    pub(crate) scratch: ExecutionEventBuffer,
    pub(crate) started: bool,
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
            order_strategies: HashMap::new(),
            operator_submissions_paused: false,
            operator_draining_routes: HashSet::new(),
            operator_degraded_routes: HashSet::new(),
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

    /// Returns the configured execution journal.
    ///
    /// This accessor is intended for control-plane integrity inspection and
    /// operator services. Do not perform blocking journal work on an order
    /// submission thread.
    pub const fn journal(&self) -> &J {
        &self.journal
    }

    /// Returns the configured execution journal mutably.
    ///
    /// This allows typed operator services to call concrete journal controls,
    /// such as segmented-WAL rotation, without weakening the existing journal
    /// trait or adding filesystem work to normal order processing.
    pub const fn journal_mut(&mut self) -> &mut J {
        &mut self.journal
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
        let available_route_count = self
            .routes
            .iter()
            .filter(|route| {
                let key = RouteKey::new(route.route_id, route.account_id, route.symbol);
                route.enabled
                    && !route.risk_limits.kill_switch
                    && !self.operator_draining_routes.contains(&key)
                    && !self.operator_degraded_routes.contains(&key)
            })
            .count();
        let open_order_count = self
            .orders
            .values()
            .filter(|state| !state.state().status.is_terminal())
            .count();
        let terminal_order_count = self.orders.len().saturating_sub(open_order_count);
        let new_submissions_blocked = self.operator_submissions_paused
            || !self.started
            || !health.connected
            || available_route_count == 0;
        let operator_attention_required = !self.started
            || !health.connected
            || health.degraded
            || enabled_route_count == 0
            || kill_switch_route_count > 0
            || self.operator_submissions_paused
            || !self.operator_draining_routes.is_empty()
            || !self.operator_degraded_routes.is_empty()
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
            submissions_paused: self.operator_submissions_paused,
            draining_route_count: self.operator_draining_routes.len(),
            degraded_route_count: self.operator_degraded_routes.len(),
            available_route_count,
            new_submissions_blocked,
            operator_attention_required,
        }
    }

    /// Returns an audit-bundle manifest with a wall-clock creation timestamp.
    ///
    /// # Errors
    ///
    /// Returns journal errors from the configured execution journal.
    pub fn audit_bundle_manifest(&self) -> ExecutionResult<ExecutionAuditBundleManifest> {
        self.audit_bundle_manifest_at(unix_ts_nanos())
    }

    /// Returns an audit-bundle manifest with a caller-provided timestamp.
    ///
    /// # Errors
    ///
    /// Returns journal errors from the configured execution journal.
    pub fn audit_bundle_manifest_at(
        &self,
        generated_ns: u64,
    ) -> ExecutionResult<ExecutionAuditBundleManifest> {
        let mut records = Vec::new();
        self.journal.replay(&mut records)?;
        let journal_command_count = records
            .iter()
            .filter(|record| matches!(record, JournalRecord::Command { .. }))
            .count();
        let journal_event_count = records
            .iter()
            .filter(|record| matches!(record, JournalRecord::Event(_)))
            .count();
        let runbook = self.runbook_snapshot();

        Ok(ExecutionAuditBundleManifest {
            schema_version: ExecutionAuditBundleManifest::SCHEMA_VERSION,
            generated_ns,
            runbook,
            route_count: runbook.route_count,
            enabled_route_count: runbook.enabled_route_count,
            open_order_count: runbook.open_order_count,
            terminal_order_count: runbook.terminal_order_count,
            journal_record_count: records.len(),
            journal_command_count,
            journal_event_count,
            metrics: self.metrics,
            submissions_enabled: runbook.can_submit_new_orders(),
            operator_attention_required: runbook.operator_attention_required,
        })
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
        let route_key = RouteKey::new(route.route_id, route.account_id, route.symbol);
        if self.operator_submissions_paused
            || self.operator_draining_routes.contains(&route_key)
            || self.operator_degraded_routes.contains(&route_key)
        {
            self.metrics.risk_rejected = self.metrics.risk_rejected.saturating_add(1);
            let event = ExecutionEvent::rejected(
                &req,
                RiskRejectReason::KillSwitch,
                ExecutionText::new("operator submission control active")?,
            );
            self.journal.record_event(&event)?;
            out.push(event)?;
            return Err(ExecutionError::RiskRejected(RiskRejectReason::KillSwitch));
        }
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

        self.journal.record_submit(&req)?;
        self.orders
            .insert(req.client_order_id, OrderStateMachine::new(&req));
        self.order_strategies
            .insert(req.client_order_id, req.strategy_id);
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
        self.journal.record_cancel(&req)?;
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
        self.journal.record_amend(&req)?;
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
        self.check_route_order(&route.risk_limits, req.quantity, req.limit_price, ctx)
    }

    fn check_route_amend(
        &self,
        req: &AmendRequest,
        route: &RouteConfig,
        ctx: &RiskContext,
    ) -> of_execution_core::RiskDecision {
        self.check_route_order(&route.risk_limits, req.quantity, req.limit_price, ctx)
    }

    fn check_route_order(
        &self,
        limits: &RiskLimits,
        quantity: OrderQty,
        limit_price: OrderPrice,
        ctx: &RiskContext,
    ) -> of_execution_core::RiskDecision {
        self.check_route_common(limits, ctx)
            .or_else(|| self.check_route_size_price(limits, quantity, limit_price, ctx))
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

    pub(crate) fn apply_scratch(
        &mut self,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<usize> {
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
                if let Some(strategy_id) = self.order_strategies.remove(&key) {
                    self.order_strategies
                        .insert(event.client_order_id, strategy_id);
                }
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
            self.order_strategies
                .insert(event.client_order_id, Default::default());
        }
        self.journal.record_event(&event)?;
        self.metrics.events_applied = self.metrics.events_applied.saturating_add(1);
        Ok(())
    }
}
