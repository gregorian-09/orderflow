use super::*;

/// Aggressive sweep configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepConfig {
    max_routes: usize,
    price_collar: OrderPrice,
    min_fill_qty: OrderQty,
}

impl SweepConfig {
    /// Creates sweep configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSweepParameters`] when route count, collar,
    /// or minimum fill quantity is invalid.
    pub const fn new(
        max_routes: usize,
        price_collar: OrderPrice,
        min_fill_qty: OrderQty,
    ) -> Result<Self, AlgoError> {
        if max_routes == 0 || price_collar.0 <= 0 || min_fill_qty.0 < 0 {
            return Err(AlgoError::InvalidSweepParameters);
        }
        Ok(Self {
            max_routes,
            price_collar,
            min_fill_qty,
        })
    }

    /// Returns maximum route/level allocations per decision.
    pub const fn max_routes(&self) -> usize {
        self.max_routes
    }

    /// Returns side-aware price collar.
    pub const fn price_collar(&self) -> OrderPrice {
        self.price_collar
    }

    /// Returns minimum total fill quantity required for a non-empty decision.
    pub const fn min_fill_qty(&self) -> OrderQty {
        self.min_fill_qty
    }
}

/// One aggressive sweep allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepAllocation {
    plan: ChildOrderPlan,
}

impl SweepAllocation {
    /// Returns planned child order for this sweep level.
    pub const fn plan(&self) -> ChildOrderPlan {
        self.plan
    }
}

/// Fixed-capacity aggressive sweep decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepDecision<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    allocations: [Option<SweepAllocation>; N],
    len: usize,
    considered_levels: usize,
    skipped_levels: usize,
    total_qty: OrderQty,
    average_price: OrderPrice,
    collar_reached: bool,
}

impl<const N: usize> SweepDecision<N> {
    /// Creates an empty sweep decision.
    pub const fn new(considered_levels: usize) -> Self {
        Self {
            allocations: [None; N],
            len: 0,
            considered_levels,
            skipped_levels: 0,
            total_qty: OrderQty(0),
            average_price: OrderPrice(0),
            collar_reached: false,
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

    /// Returns considered book/route levels.
    pub const fn considered_levels(&self) -> usize {
        self.considered_levels
    }

    /// Returns skipped book/route levels.
    pub const fn skipped_levels(&self) -> usize {
        self.skipped_levels
    }

    /// Returns total planned sweep quantity.
    pub const fn total_qty(&self) -> OrderQty {
        self.total_qty
    }

    /// Returns average planned sweep price.
    pub const fn average_price(&self) -> OrderPrice {
        self.average_price
    }

    /// Returns true when at least one candidate was outside the price collar.
    pub const fn collar_reached(&self) -> bool {
        self.collar_reached
    }

    /// Returns allocations in selected order.
    pub fn allocations(&self) -> impl Iterator<Item = &SweepAllocation> {
        self.allocations[..self.len]
            .iter()
            .filter_map(Option::as_ref)
    }

    fn push(&mut self, allocation: SweepAllocation) -> Result<(), AlgoError> {
        if self.len == N {
            return Err(AlgoError::DecisionFull { capacity: N });
        }
        let qty = allocation.plan().request().quantity.0;
        let price = allocation.plan().request().limit_price.0;
        let previous_notional = i128::from(self.average_price.0) * i128::from(self.total_qty.0);
        let new_notional = previous_notional.saturating_add(i128::from(price) * i128::from(qty));
        self.total_qty = OrderQty(self.total_qty.0.saturating_add(qty));
        if self.total_qty.0 > 0 {
            self.average_price = OrderPrice(
                i64::try_from(new_notional / i128::from(self.total_qty.0)).unwrap_or(i64::MAX),
            );
        }
        self.allocations[self.len] = Some(allocation);
        self.len += 1;
        Ok(())
    }

    fn mark_skipped(&mut self) {
        self.skipped_levels = self.skipped_levels.saturating_add(1);
    }

    fn mark_collar_reached(&mut self) {
        self.collar_reached = true;
    }
}

/// Deterministic aggressive sweep planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepPlanner {
    config: SweepConfig,
}

impl SweepPlanner {
    /// Creates a sweep planner.
    pub const fn new(config: SweepConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> SweepConfig {
        self.config
    }

    /// Plans aggressive route/level child orders up to the price collar.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/id slices are invalid, a
    /// generated child order is invalid, or fixed output capacity is exhausted.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns route candidates, ids, and timestamps"
    )]
    pub fn plan_sweep<const N: usize>(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        candidates: &[SorRouteCandidate],
        child_ids: &[ChildOrderId],
        client_order_ids: &[ClientOrderId],
        ts_recv_ns: u64,
    ) -> Result<SweepDecision<N>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if child_ids.len() < N || client_order_ids.len() < N {
            return Err(AlgoError::InvalidSweepParameters);
        }
        if now_ns < parent.start_ns() || progress.is_complete() || candidates.is_empty() {
            return Ok(SweepDecision::new(candidates.len()));
        }

        let mut decision = SweepDecision::<N>::new(candidates.len());
        let mut leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0)
            .min(parent.max_clip().0);
        let route_limit = self.config.max_routes().min(N);

        while leaves > 0 && decision.len() < route_limit {
            let mut best_index = None;
            let mut best_price = match parent.side() {
                OrderSide::Buy => OrderPrice(i64::MAX),
                OrderSide::Sell => OrderPrice(0),
            };
            for (index, candidate) in candidates.iter().copied().enumerate() {
                if sweep_decision_has_level(&decision, candidate.route_id(), candidate.price()) {
                    continue;
                }
                if !candidate.status().is_routable()
                    || !candidate
                        .capability()
                        .supports_order_type(parent.order_type())
                    || candidate.available_qty().0 <= 0
                    || candidate.price().0 <= 0
                {
                    decision.mark_skipped();
                    continue;
                }
                if !price_inside_collar(
                    parent.side(),
                    candidate.price(),
                    self.config.price_collar(),
                ) {
                    decision.mark_collar_reached();
                    continue;
                }
                let better = match parent.side() {
                    OrderSide::Buy => candidate.price().0 < best_price.0,
                    OrderSide::Sell => candidate.price().0 > best_price.0,
                };
                if better {
                    best_price = candidate.price();
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
            let qty = candidate.available_qty().0.min(leaves);
            if qty <= 0 {
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
            decision.push(SweepAllocation { plan })?;
            leaves = leaves.saturating_sub(qty);
        }

        if decision.total_qty().0 < self.config.min_fill_qty().0 {
            return Ok(SweepDecision::new(candidates.len()));
        }
        Ok(decision)
    }
}
