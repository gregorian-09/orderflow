use super::*;

/// Liquidity-seeking action selected for one route.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LiquiditySeekingAction {
    /// Skip the route for this decision.
    Skip = 1,
    /// Send a small probe child order.
    Probe = 2,
    /// Send a larger liquidity-taking child order.
    Take = 3,
}

impl LiquiditySeekingAction {
    /// Returns true when this action releases a child order.
    pub const fn releases_child(self) -> bool {
        matches!(self, Self::Probe | Self::Take)
    }
}

/// Liquidity-seeking candidate derived from a routable venue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquiditySeekingCandidate {
    route: SorRouteCandidate,
    hidden_liquidity_bps: u16,
    price_improvement_bps: u16,
    minimum_quantity: OrderQty,
}

impl LiquiditySeekingCandidate {
    /// Creates a liquidity-seeking route candidate.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidLiquiditySeekingParameters`] when bps
    /// fields exceed 10,000 or minimum quantity is negative.
    pub const fn new(
        route: SorRouteCandidate,
        hidden_liquidity_bps: u16,
        price_improvement_bps: u16,
        minimum_quantity: OrderQty,
    ) -> Result<Self, AlgoError> {
        if hidden_liquidity_bps > 10_000 || price_improvement_bps > 10_000 || minimum_quantity.0 < 0
        {
            return Err(AlgoError::InvalidLiquiditySeekingParameters);
        }
        Ok(Self {
            route,
            hidden_liquidity_bps,
            price_improvement_bps,
            minimum_quantity,
        })
    }

    /// Returns embedded SOR route candidate.
    pub const fn route(&self) -> SorRouteCandidate {
        self.route
    }

    /// Returns hidden-liquidity estimate in basis points.
    pub const fn hidden_liquidity_bps(&self) -> u16 {
        self.hidden_liquidity_bps
    }

    /// Returns price-improvement estimate in basis points.
    pub const fn price_improvement_bps(&self) -> u16 {
        self.price_improvement_bps
    }

    /// Returns minimum executable quantity requirement.
    pub const fn minimum_quantity(&self) -> OrderQty {
        self.minimum_quantity
    }
}

/// Configuration for liquidity-seeking route selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquiditySeekingConfig {
    max_routes: usize,
    probe_qty: OrderQty,
    min_score: i64,
    sweep_fill_probability_bps: u16,
    max_toxicity_bps: u16,
    hidden_liquidity_weight: u16,
    price_improvement_weight: u16,
}

impl LiquiditySeekingConfig {
    /// Creates liquidity-seeking configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidLiquiditySeekingParameters`] when route
    /// count, quantity, or bps bounds are invalid.
    pub const fn new(
        max_routes: usize,
        probe_qty: OrderQty,
        min_score: i64,
        sweep_fill_probability_bps: u16,
        max_toxicity_bps: u16,
        hidden_liquidity_weight: u16,
        price_improvement_weight: u16,
    ) -> Result<Self, AlgoError> {
        if max_routes == 0
            || probe_qty.0 <= 0
            || sweep_fill_probability_bps > 10_000
            || max_toxicity_bps > 10_000
        {
            return Err(AlgoError::InvalidLiquiditySeekingParameters);
        }
        Ok(Self {
            max_routes,
            probe_qty,
            min_score,
            sweep_fill_probability_bps,
            max_toxicity_bps,
            hidden_liquidity_weight,
            price_improvement_weight,
        })
    }

    /// Returns maximum route allocations per decision.
    pub const fn max_routes(&self) -> usize {
        self.max_routes
    }

    /// Returns probe child quantity.
    pub const fn probe_qty(&self) -> OrderQty {
        self.probe_qty
    }

    /// Returns minimum score required for route selection.
    pub const fn min_score(&self) -> i64 {
        self.min_score
    }

    /// Returns fill-probability threshold for take/sweep behavior.
    pub const fn sweep_fill_probability_bps(&self) -> u16 {
        self.sweep_fill_probability_bps
    }

    /// Returns maximum tolerated venue toxicity.
    pub const fn max_toxicity_bps(&self) -> u16 {
        self.max_toxicity_bps
    }

    /// Returns hidden-liquidity score weight.
    pub const fn hidden_liquidity_weight(&self) -> u16 {
        self.hidden_liquidity_weight
    }

    /// Returns price-improvement score weight.
    pub const fn price_improvement_weight(&self) -> u16 {
        self.price_improvement_weight
    }
}

impl Default for LiquiditySeekingConfig {
    fn default() -> Self {
        Self {
            max_routes: 4,
            probe_qty: OrderQty(1),
            min_score: 0,
            sweep_fill_probability_bps: 7_500,
            max_toxicity_bps: 1_500,
            hidden_liquidity_weight: 3,
            price_improvement_weight: 4,
        }
    }
}

/// One liquidity-seeking allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquiditySeekingAllocation {
    action: LiquiditySeekingAction,
    score: i64,
    plan: ChildOrderPlan,
}

impl LiquiditySeekingAllocation {
    /// Returns selected action.
    pub const fn action(&self) -> LiquiditySeekingAction {
        self.action
    }

    /// Returns route score.
    pub const fn score(&self) -> i64 {
        self.score
    }

    /// Returns planned child order.
    pub const fn plan(&self) -> ChildOrderPlan {
        self.plan
    }
}

/// Fixed-capacity liquidity-seeking decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquiditySeekingDecision<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    allocations: [Option<LiquiditySeekingAllocation>; N],
    len: usize,
    considered_routes: usize,
    skipped_routes: usize,
}

impl<const N: usize> LiquiditySeekingDecision<N> {
    /// Creates an empty liquidity-seeking decision.
    pub const fn new(considered_routes: usize) -> Self {
        Self {
            allocations: [None; N],
            len: 0,
            considered_routes,
            skipped_routes: 0,
        }
    }

    /// Returns allocation count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no allocations were produced.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns number of route candidates considered.
    pub const fn considered_routes(&self) -> usize {
        self.considered_routes
    }

    /// Returns number of skipped route candidates.
    pub const fn skipped_routes(&self) -> usize {
        self.skipped_routes
    }

    /// Returns allocations in selected order.
    pub fn allocations(&self) -> impl Iterator<Item = &LiquiditySeekingAllocation> {
        self.allocations[..self.len]
            .iter()
            .filter_map(Option::as_ref)
    }

    fn push(&mut self, allocation: LiquiditySeekingAllocation) -> Result<(), AlgoError> {
        if self.len == N {
            return Err(AlgoError::DecisionFull { capacity: N });
        }
        self.allocations[self.len] = Some(allocation);
        self.len += 1;
        Ok(())
    }

    fn mark_skipped(&mut self) {
        self.skipped_routes = self.skipped_routes.saturating_add(1);
    }
}

/// Deterministic liquidity-seeking planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquiditySeekingPlanner {
    config: LiquiditySeekingConfig,
    sor: SorPlanner,
}

impl LiquiditySeekingPlanner {
    /// Creates a liquidity-seeking planner.
    pub const fn new(config: LiquiditySeekingConfig, sor_config: SorConfig) -> Self {
        Self {
            config,
            sor: SorPlanner::new(sor_config),
        }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> LiquiditySeekingConfig {
        self.config
    }

    /// Scores one liquidity-seeking candidate.
    pub fn score_candidate(
        &self,
        parent: &ParentOrder,
        candidate: LiquiditySeekingCandidate,
        best_price: OrderPrice,
    ) -> Option<i64> {
        if candidate.route().metrics().toxicity_bps() > self.config.max_toxicity_bps()
            || candidate.route().available_qty().0 < candidate.minimum_quantity().0
        {
            return None;
        }
        let mut score = self
            .sor
            .score_route(parent, candidate.route(), best_price)?;
        score = score.saturating_add(
            i64::from(candidate.hidden_liquidity_bps())
                * i64::from(self.config.hidden_liquidity_weight()),
        );
        score = score.saturating_add(
            i64::from(candidate.price_improvement_bps())
                * i64::from(self.config.price_improvement_weight()),
        );
        Some(score)
    }

    /// Plans route probes or larger liquidity-taking child orders.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/id slices are invalid, a
    /// generated child order is invalid, or fixed output capacity is exhausted.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns route candidates, ids, and timestamps"
    )]
    pub fn plan_liquidity<const N: usize>(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        candidates: &[LiquiditySeekingCandidate],
        child_ids: &[ChildOrderId],
        client_order_ids: &[ClientOrderId],
        ts_recv_ns: u64,
    ) -> Result<LiquiditySeekingDecision<N>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if child_ids.len() < N || client_order_ids.len() < N {
            return Err(AlgoError::InvalidLiquiditySeekingParameters);
        }
        if now_ns < parent.start_ns() || progress.is_complete() || candidates.is_empty() {
            return Ok(LiquiditySeekingDecision::new(candidates.len()));
        }

        let mut decision = LiquiditySeekingDecision::<N>::new(candidates.len());
        let mut leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0)
            .min(parent.max_clip().0);
        let route_limit = self.config.max_routes().min(N);
        let Some(best_price) = best_liquidity_price(parent.side(), candidates) else {
            return Ok(decision);
        };

        while leaves > 0 && decision.len() < route_limit {
            let mut best_index = None;
            let mut best_score = i64::MIN;
            for (index, candidate) in candidates.iter().copied().enumerate() {
                if liquidity_decision_has_route(&decision, candidate.route().route_id()) {
                    continue;
                }
                let Some(score) = self.score_candidate(parent, candidate, best_price) else {
                    if decision.is_empty() {
                        decision.mark_skipped();
                    }
                    continue;
                };
                if score >= self.config.min_score() && score > best_score {
                    best_score = score;
                    best_index = Some(index);
                }
            }
            let Some(index) = best_index else {
                break;
            };
            if decision.len() == N {
                return Err(AlgoError::DecisionFull { capacity: N });
            }
            let candidate = candidates[index];
            let action = if candidate.route().metrics().fill_probability_bps()
                >= self.config.sweep_fill_probability_bps()
            {
                LiquiditySeekingAction::Take
            } else {
                LiquiditySeekingAction::Probe
            };
            let desired_qty = match action {
                LiquiditySeekingAction::Take => candidate.route().available_qty().0,
                LiquiditySeekingAction::Probe => self.config.probe_qty().0,
                LiquiditySeekingAction::Skip => 0,
            };
            let qty = desired_qty
                .min(candidate.route().available_qty().0)
                .min(leaves);
            let final_slice = progress.released_qty().0.saturating_add(qty) >= parent.total_qty().0
                || now_ns >= parent.end_ns();
            if qty < parent.min_clip().0 && !final_slice {
                decision.mark_skipped();
                break;
            }
            if qty <= 0 {
                break;
            }
            let request = parent.build_order_request_for_route_at_price(
                candidate.route().route_id(),
                client_order_ids[decision.len()],
                OrderQty(qty),
                candidate.route().price(),
                ts_recv_ns,
            );
            let plan =
                ChildOrderPlan::new(child_ids[decision.len()], parent.id(), request, now_ns)?;
            decision.push(LiquiditySeekingAllocation {
                action,
                score: best_score,
                plan,
            })?;
            leaves = leaves.saturating_sub(qty);
        }

        Ok(decision)
    }
}
