use super::*;

/// Route availability state used by [`SorPlanner`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SorRouteStatus {
    /// Route is available for new child orders.
    Available = 1,
    /// Route is blocked by policy, risk, or operator action.
    Blocked = 2,
    /// Route is degraded and should be skipped by default routing.
    Degraded = 3,
    /// Route is temporarily cooling down after rejects, disconnects, or throttles.
    Cooldown = 4,
}

impl SorRouteStatus {
    /// Returns true when the route can receive a new child order.
    pub const fn is_routable(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// Order-type capability advertised by a SOR route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorRouteCapability {
    supports_limit: bool,
    supports_market: bool,
}

impl SorRouteCapability {
    /// Creates route order-type capability flags.
    pub const fn new(supports_limit: bool, supports_market: bool) -> Self {
        Self {
            supports_limit,
            supports_market,
        }
    }

    /// Returns true when limit orders are supported.
    pub const fn supports_limit(&self) -> bool {
        self.supports_limit
    }

    /// Returns true when market orders are supported.
    pub const fn supports_market(&self) -> bool {
        self.supports_market
    }

    pub(crate) fn supports_order_type(&self, order_type: OrderType) -> bool {
        match order_type {
            OrderType::Limit | OrderType::StopLimit => self.supports_limit,
            OrderType::Market | OrderType::Stop => self.supports_market,
        }
    }
}

impl Default for SorRouteCapability {
    fn default() -> Self {
        Self::new(true, true)
    }
}

/// Route quality metrics used for smart-order-router scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorRouteMetrics {
    fee_bps: i16,
    latency_us: u32,
    reject_rate_bps: u16,
    fill_probability_bps: u16,
    toxicity_bps: u16,
    data_quality_bps: u16,
}

impl SorRouteMetrics {
    /// Creates route metrics for deterministic scoring.
    ///
    /// `fee_bps` may be negative for rebates. All unsigned basis-point values
    /// must be at most 10,000.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSorParameters`] when a metric is outside
    /// its supported range.
    pub const fn new(
        fee_bps: i16,
        latency_us: u32,
        reject_rate_bps: u16,
        fill_probability_bps: u16,
        toxicity_bps: u16,
        data_quality_bps: u16,
    ) -> Result<Self, AlgoError> {
        if fee_bps < -10_000
            || fee_bps > 10_000
            || reject_rate_bps > 10_000
            || fill_probability_bps > 10_000
            || toxicity_bps > 10_000
            || data_quality_bps > 10_000
        {
            return Err(AlgoError::InvalidSorParameters);
        }
        Ok(Self {
            fee_bps,
            latency_us,
            reject_rate_bps,
            fill_probability_bps,
            toxicity_bps,
            data_quality_bps,
        })
    }

    /// Returns fee or rebate estimate in basis points.
    pub const fn fee_bps(&self) -> i16 {
        self.fee_bps
    }

    /// Returns route latency estimate in microseconds.
    pub const fn latency_us(&self) -> u32 {
        self.latency_us
    }

    /// Returns recent route reject rate in basis points.
    pub const fn reject_rate_bps(&self) -> u16 {
        self.reject_rate_bps
    }

    /// Returns estimated route fill probability in basis points.
    pub const fn fill_probability_bps(&self) -> u16 {
        self.fill_probability_bps
    }

    /// Returns route toxicity estimate in basis points.
    pub const fn toxicity_bps(&self) -> u16 {
        self.toxicity_bps
    }

    /// Returns market-data quality score in basis points.
    pub const fn data_quality_bps(&self) -> u16 {
        self.data_quality_bps
    }
}

impl Default for SorRouteMetrics {
    fn default() -> Self {
        Self {
            fee_bps: 0,
            latency_us: 0,
            reject_rate_bps: 0,
            fill_probability_bps: 10_000,
            toxicity_bps: 0,
            data_quality_bps: 10_000,
        }
    }
}

/// Routable liquidity candidate consumed by [`SorPlanner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorRouteCandidate {
    route_id: RouteId,
    price: OrderPrice,
    available_qty: OrderQty,
    status: SorRouteStatus,
    capability: SorRouteCapability,
    metrics: SorRouteMetrics,
}

impl SorRouteCandidate {
    /// Creates an available route candidate with default capabilities and
    /// metrics.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSorParameters`] when price or available
    /// quantity is non-positive.
    pub const fn new(
        route_id: RouteId,
        price: OrderPrice,
        available_qty: OrderQty,
    ) -> Result<Self, AlgoError> {
        if price.0 <= 0 || available_qty.0 <= 0 {
            return Err(AlgoError::InvalidSorParameters);
        }
        Ok(Self {
            route_id,
            price,
            available_qty,
            status: SorRouteStatus::Available,
            capability: SorRouteCapability::new(true, true),
            metrics: SorRouteMetrics {
                fee_bps: 0,
                latency_us: 0,
                reject_rate_bps: 0,
                fill_probability_bps: 10_000,
                toxicity_bps: 0,
                data_quality_bps: 10_000,
            },
        })
    }

    /// Returns a copy with route status updated.
    pub const fn with_status(mut self, status: SorRouteStatus) -> Self {
        self.status = status;
        self
    }

    /// Returns a copy with capability updated.
    pub const fn with_capability(mut self, capability: SorRouteCapability) -> Self {
        self.capability = capability;
        self
    }

    /// Returns a copy with route metrics updated.
    pub const fn with_metrics(mut self, metrics: SorRouteMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    /// Returns route identifier.
    pub const fn route_id(&self) -> RouteId {
        self.route_id
    }

    /// Returns route price.
    pub const fn price(&self) -> OrderPrice {
        self.price
    }

    /// Returns available route quantity.
    pub const fn available_qty(&self) -> OrderQty {
        self.available_qty
    }

    /// Returns route status.
    pub const fn status(&self) -> SorRouteStatus {
        self.status
    }

    /// Returns route capability flags.
    pub const fn capability(&self) -> SorRouteCapability {
        self.capability
    }

    /// Returns route metrics.
    pub const fn metrics(&self) -> SorRouteMetrics {
        self.metrics
    }
}

/// Integer score weights for [`SorPlanner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorScoreWeights {
    price_weight: u16,
    liquidity_weight: u16,
    fee_weight: u16,
    latency_weight: u16,
    reject_weight: u16,
    fill_weight: u16,
    toxicity_weight: u16,
    data_quality_weight: u16,
}

impl SorScoreWeights {
    /// Creates SOR score weights.
    #[allow(
        clippy::too_many_arguments,
        reason = "flat constructor keeps score weights explicit and allocation-free"
    )]
    pub const fn new(
        price_weight: u16,
        liquidity_weight: u16,
        fee_weight: u16,
        latency_weight: u16,
        reject_weight: u16,
        fill_weight: u16,
        toxicity_weight: u16,
        data_quality_weight: u16,
    ) -> Self {
        Self {
            price_weight,
            liquidity_weight,
            fee_weight,
            latency_weight,
            reject_weight,
            fill_weight,
            toxicity_weight,
            data_quality_weight,
        }
    }

    /// Returns price weight.
    pub const fn price_weight(&self) -> u16 {
        self.price_weight
    }

    /// Returns liquidity weight.
    pub const fn liquidity_weight(&self) -> u16 {
        self.liquidity_weight
    }

    /// Returns fee weight.
    pub const fn fee_weight(&self) -> u16 {
        self.fee_weight
    }

    /// Returns latency weight.
    pub const fn latency_weight(&self) -> u16 {
        self.latency_weight
    }

    /// Returns reject-rate weight.
    pub const fn reject_weight(&self) -> u16 {
        self.reject_weight
    }

    /// Returns fill-probability weight.
    pub const fn fill_weight(&self) -> u16 {
        self.fill_weight
    }

    /// Returns toxicity weight.
    pub const fn toxicity_weight(&self) -> u16 {
        self.toxicity_weight
    }

    /// Returns data-quality weight.
    pub const fn data_quality_weight(&self) -> u16 {
        self.data_quality_weight
    }
}

impl Default for SorScoreWeights {
    fn default() -> Self {
        Self::new(8, 2, 2, 1, 3, 3, 4, 1)
    }
}

/// Smart-order-router configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorConfig {
    max_route_count: usize,
    weights: SorScoreWeights,
}

impl SorConfig {
    /// Creates SOR configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSorParameters`] when max route count is
    /// zero.
    pub const fn new(max_route_count: usize, weights: SorScoreWeights) -> Result<Self, AlgoError> {
        if max_route_count == 0 {
            return Err(AlgoError::InvalidSorParameters);
        }
        Ok(Self {
            max_route_count,
            weights,
        })
    }

    /// Returns maximum number of routes to allocate in one decision.
    pub const fn max_route_count(&self) -> usize {
        self.max_route_count
    }

    /// Returns score weights.
    pub const fn weights(&self) -> SorScoreWeights {
        self.weights
    }
}

impl Default for SorConfig {
    fn default() -> Self {
        Self::new(4, SorScoreWeights::default()).expect("static SOR defaults are valid")
    }
}

/// Scored child allocation produced by [`SorPlanner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorChildAllocation {
    score: i64,
    plan: ChildOrderPlan,
}

impl SorChildAllocation {
    /// Returns route score used for selection.
    pub const fn score(&self) -> i64 {
        self.score
    }

    /// Returns planned child order for the selected route.
    pub const fn plan(&self) -> ChildOrderPlan {
        self.plan
    }
}

/// Fixed-capacity SOR decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorDecision<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    allocations: [Option<SorChildAllocation>; N],
    len: usize,
    considered_routes: usize,
    blocked_routes: usize,
}

impl<const N: usize> SorDecision<N> {
    /// Creates an empty SOR decision.
    pub const fn new(considered_routes: usize) -> Self {
        Self {
            allocations: [None; N],
            len: 0,
            considered_routes,
            blocked_routes: 0,
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

    /// Returns number of considered route candidates.
    pub const fn considered_routes(&self) -> usize {
        self.considered_routes
    }

    /// Returns number of skipped routes.
    pub const fn blocked_routes(&self) -> usize {
        self.blocked_routes
    }

    /// Returns allocations in selected order.
    pub fn allocations(&self) -> impl Iterator<Item = &SorChildAllocation> {
        self.allocations[..self.len]
            .iter()
            .filter_map(Option::as_ref)
    }

    fn push(&mut self, allocation: SorChildAllocation) -> Result<(), AlgoError> {
        if self.len == N {
            return Err(AlgoError::DecisionFull { capacity: N });
        }
        self.allocations[self.len] = Some(allocation);
        self.len += 1;
        Ok(())
    }

    fn mark_blocked(&mut self) {
        self.blocked_routes = self.blocked_routes.saturating_add(1);
    }
}

/// Deterministic smart-order-router planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SorPlanner {
    config: SorConfig,
}

impl SorPlanner {
    /// Creates a SOR planner.
    pub const fn new(config: SorConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> SorConfig {
        self.config
    }

    /// Scores one route candidate for a parent order.
    ///
    /// Higher scores are preferred. Non-routable candidates return `None`.
    pub fn score_route(
        &self,
        parent: &ParentOrder,
        candidate: SorRouteCandidate,
        best_price: OrderPrice,
    ) -> Option<i64> {
        if !candidate.status().is_routable()
            || !candidate
                .capability()
                .supports_order_type(parent.order_type())
            || candidate.available_qty().0 <= 0
            || candidate.price().0 <= 0
        {
            return None;
        }
        let weights = self.config.weights();
        let liquidity_bps = route_liquidity_bps(candidate.available_qty().0, parent.max_clip().0);
        let price_penalty = route_price_penalty_bps(parent.side(), candidate.price(), best_price);
        let latency_penalty = candidate.metrics().latency_us() / 100;
        let fee_penalty =
            i64::from(candidate.metrics().fee_bps()) * i64::from(weights.fee_weight());

        let mut score = 0_i64;
        score = score.saturating_add(
            i64::from(candidate.metrics().fill_probability_bps())
                * i64::from(weights.fill_weight()),
        );
        score =
            score.saturating_add(i64::from(liquidity_bps) * i64::from(weights.liquidity_weight()));
        score = score.saturating_add(
            i64::from(candidate.metrics().data_quality_bps())
                * i64::from(weights.data_quality_weight()),
        );
        score = score.saturating_sub(i64::from(price_penalty) * i64::from(weights.price_weight()));
        score = score.saturating_sub(fee_penalty);
        score =
            score.saturating_sub(i64::from(latency_penalty) * i64::from(weights.latency_weight()));
        score = score.saturating_sub(
            i64::from(candidate.metrics().reject_rate_bps()) * i64::from(weights.reject_weight()),
        );
        score = score.saturating_sub(
            i64::from(candidate.metrics().toxicity_bps()) * i64::from(weights.toxicity_weight()),
        );
        Some(score)
    }

    /// Plans route allocations for one child-routing decision.
    ///
    /// `child_ids` and `client_order_ids` must contain at least `N` entries
    /// when the caller wants to allow `N` allocations. The planner never
    /// generates ids internally.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress state is invalid, candidate
    /// data is invalid, capacity is exhausted, or not enough ids are supplied.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers, timestamps, and route candidates"
    )]
    pub fn plan_routes<const N: usize>(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        candidates: &[SorRouteCandidate],
        child_ids: &[ChildOrderId],
        client_order_ids: &[ClientOrderId],
        ts_recv_ns: u64,
    ) -> Result<SorDecision<N>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if child_ids.len() < N || client_order_ids.len() < N {
            return Err(AlgoError::InvalidSorParameters);
        }
        if now_ns < parent.start_ns() || progress.is_complete() || candidates.is_empty() {
            return Ok(SorDecision::new(candidates.len()));
        }

        let mut decision = SorDecision::<N>::new(candidates.len());
        let mut leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0)
            .min(parent.max_clip().0);
        let route_limit = self.config.max_route_count().min(N);
        let Some(best_price) = best_sor_price(parent.side(), candidates) else {
            return Ok(decision);
        };

        while leaves > 0 && decision.len() < route_limit {
            let mut best_index = None;
            let mut best_score = i64::MIN;
            for (index, candidate) in candidates.iter().copied().enumerate() {
                if decision_has_route(&decision, candidate.route_id()) {
                    continue;
                }
                let Some(score) = self.score_route(parent, candidate, best_price) else {
                    if decision.is_empty() {
                        decision.mark_blocked();
                    }
                    continue;
                };
                if score > best_score {
                    best_score = score;
                    best_index = Some(index);
                }
            }
            let Some(index) = best_index else {
                break;
            };
            let candidate = candidates[index];
            let qty = leaves.min(candidate.available_qty().0);
            let final_slice = progress.released_qty().0.saturating_add(qty) >= parent.total_qty().0
                || now_ns >= parent.end_ns();
            if qty < parent.min_clip().0 && !final_slice {
                break;
            }
            let request = parent.build_order_request_for_route_at_price(
                candidate.route_id(),
                client_order_ids[decision.len()],
                OrderQty(qty),
                candidate.price(),
                ts_recv_ns,
            );
            let plan =
                ChildOrderPlan::new(child_ids[decision.len()], parent.id(), request, now_ns)?;
            decision.push(SorChildAllocation {
                score: best_score,
                plan,
            })?;
            leaves = leaves.saturating_sub(qty);
        }

        Ok(decision)
    }
}
