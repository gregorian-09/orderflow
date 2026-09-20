use super::*;

/// Basket or spread leg side in the portfolio objective.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BasketLegRole {
    /// Primary alpha or exposure leg.
    Primary = 1,
    /// Hedge leg intended to reduce exposure drift.
    Hedge = 2,
    /// Offset leg in a spread or relative-value structure.
    Offset = 3,
}

/// One parent order participating in a basket or spread execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasketLeg {
    parent: ParentOrder,
    role: BasketLegRole,
    hedge_ratio_bps: i32,
}

impl BasketLeg {
    /// Creates a basket leg.
    ///
    /// `hedge_ratio_bps` is metadata for audit and host-side risk checks. The
    /// first planner slice uses each leg's own parent quantity as the executable
    /// target so existing single-leg OMS semantics remain unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when the parent is invalid or hedge ratio is zero.
    pub fn new(
        parent: ParentOrder,
        role: BasketLegRole,
        hedge_ratio_bps: i32,
    ) -> Result<Self, AlgoError> {
        parent.validate()?;
        if hedge_ratio_bps == 0 {
            return Err(AlgoError::InvalidBasketParameters);
        }
        Ok(Self {
            parent,
            role,
            hedge_ratio_bps,
        })
    }

    /// Returns the leg parent order.
    pub const fn parent(&self) -> ParentOrder {
        self.parent
    }

    /// Returns leg role.
    pub const fn role(&self) -> BasketLegRole {
        self.role
    }

    /// Returns hedge ratio metadata in basis points.
    pub const fn hedge_ratio_bps(&self) -> i32 {
        self.hedge_ratio_bps
    }
}

/// Planned child allocation for one basket leg.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasketChildAllocation {
    leg_index: usize,
    role: BasketLegRole,
    target_release_qty: OrderQty,
    plan: ChildOrderPlan,
}

impl BasketChildAllocation {
    /// Returns leg index from the caller-provided leg slice.
    pub const fn leg_index(&self) -> usize {
        self.leg_index
    }

    /// Returns leg role.
    pub const fn role(&self) -> BasketLegRole {
        self.role
    }

    /// Returns cumulative target release quantity for the leg.
    pub const fn target_release_qty(&self) -> OrderQty {
        self.target_release_qty
    }

    /// Returns planned child order.
    pub const fn plan(&self) -> ChildOrderPlan {
        self.plan
    }
}

/// Fixed-capacity basket decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasketDecision<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    allocations: [Option<BasketChildAllocation>; N],
    len: usize,
    considered_legs: usize,
    blocked_legs: usize,
}

impl<const N: usize> BasketDecision<N> {
    /// Creates an empty basket decision.
    pub const fn new(considered_legs: usize) -> Self {
        Self {
            allocations: [None; N],
            len: 0,
            considered_legs,
            blocked_legs: 0,
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

    /// Returns considered leg count.
    pub const fn considered_legs(&self) -> usize {
        self.considered_legs
    }

    /// Returns blocked leg count.
    pub const fn blocked_legs(&self) -> usize {
        self.blocked_legs
    }

    /// Returns allocations in leg order.
    pub fn allocations(&self) -> impl Iterator<Item = &BasketChildAllocation> {
        self.allocations[..self.len]
            .iter()
            .filter_map(Option::as_ref)
    }

    fn push(&mut self, allocation: BasketChildAllocation) -> Result<(), AlgoError> {
        if self.len == N {
            return Err(AlgoError::DecisionFull { capacity: N });
        }
        self.allocations[self.len] = Some(allocation);
        self.len += 1;
        Ok(())
    }

    fn mark_blocked(&mut self) {
        self.blocked_legs = self.blocked_legs.saturating_add(1);
    }
}

/// Deterministic synchronized basket/spread planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BasketPlanner;

impl BasketPlanner {
    /// Creates a basket planner.
    pub const fn new() -> Self {
        Self
    }

    /// Plans synchronized child slices for a basket.
    ///
    /// Each leg uses its own parent schedule and clip bounds. The function
    /// emits at most one child per due leg and writes allocations into a
    /// fixed-capacity decision. Hosts remain responsible for atomic package
    /// semantics, linked-order support, hedge drift monitoring, and venue
    /// cancel/replace orchestration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when leg/progress/id slices are inconsistent, a
    /// parent/progress pair is invalid, or the fixed output capacity is full.
    pub fn plan_synchronized_slice<const N: usize>(
        &self,
        legs: &[BasketLeg],
        progresses: &[AlgoProgress],
        now_ns: u64,
        child_ids: &[ChildOrderId],
        client_order_ids: &[ClientOrderId],
        ts_recv_ns: u64,
    ) -> Result<BasketDecision<N>, AlgoError> {
        if legs.len() != progresses.len()
            || child_ids.len() < legs.len().min(N)
            || client_order_ids.len() < legs.len().min(N)
        {
            return Err(AlgoError::InvalidBasketParameters);
        }

        let mut decision = BasketDecision::<N>::new(legs.len());
        for (index, (leg, progress)) in legs.iter().zip(progresses.iter()).enumerate() {
            let parent = leg.parent();
            parent.validate()?;
            if parent.status().is_terminal() {
                decision.mark_blocked();
                continue;
            }
            if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
                return Err(AlgoError::InvalidProgress);
            }
            if now_ns < parent.start_ns() || progress.is_complete() {
                continue;
            }
            let target_release_qty = scale_qty_bps(
                parent.total_qty().0,
                u32::from(elapsed_bps(parent.start_ns(), parent.end_ns(), now_ns)),
            );
            let due_qty = target_release_qty.saturating_sub(progress.released_qty().0);
            if due_qty <= 0 {
                continue;
            }
            let leaves = parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0);
            let mut child_qty = due_qty.min(parent.max_clip().0).min(leaves);
            let final_slice = progress.released_qty().0.saturating_add(child_qty)
                >= parent.total_qty().0
                || now_ns >= parent.end_ns();
            if child_qty < parent.min_clip().0 && !final_slice {
                continue;
            }
            if child_qty <= 0 {
                continue;
            }
            child_qty = child_qty.min(leaves);
            if decision.len() == N {
                return Err(AlgoError::DecisionFull { capacity: N });
            }
            let request = parent.build_order_request(
                client_order_ids[decision.len()],
                OrderQty(child_qty),
                ts_recv_ns,
            );
            let plan =
                ChildOrderPlan::new(child_ids[decision.len()], parent.id(), request, now_ns)?;
            decision.push(BasketChildAllocation {
                leg_index: index,
                role: leg.role(),
                target_release_qty: OrderQty(target_release_qty),
                plan,
            })?;
        }
        Ok(decision)
    }
}

/// Two-leg spread execution configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadConfig {
    ratio_bps: u32,
    min_edge_bps: i32,
}

impl SpreadConfig {
    /// Creates spread configuration.
    ///
    /// `ratio_bps` scales the sell leg price and quantity relative to the buy
    /// leg. A value of 10,000 means one-to-one.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSpreadParameters`] when ratio is zero.
    pub const fn new(ratio_bps: u32, min_edge_bps: i32) -> Result<Self, AlgoError> {
        if ratio_bps == 0 {
            return Err(AlgoError::InvalidSpreadParameters);
        }
        Ok(Self {
            ratio_bps,
            min_edge_bps,
        })
    }

    /// Returns sell-leg ratio in basis points.
    pub const fn ratio_bps(&self) -> u32 {
        self.ratio_bps
    }

    /// Returns minimum required spread edge in basis points.
    pub const fn min_edge_bps(&self) -> i32 {
        self.min_edge_bps
    }
}

/// Current executable two-leg spread prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadQuote {
    buy_price: OrderPrice,
    sell_price: OrderPrice,
}

impl SpreadQuote {
    /// Creates spread quote prices.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSpreadParameters`] when either price is not
    /// positive.
    pub const fn new(buy_price: OrderPrice, sell_price: OrderPrice) -> Result<Self, AlgoError> {
        if buy_price.0 <= 0 || sell_price.0 <= 0 {
            return Err(AlgoError::InvalidSpreadParameters);
        }
        Ok(Self {
            buy_price,
            sell_price,
        })
    }

    /// Returns executable buy-leg price.
    pub const fn buy_price(&self) -> OrderPrice {
        self.buy_price
    }

    /// Returns executable sell-leg price.
    pub const fn sell_price(&self) -> OrderPrice {
        self.sell_price
    }
}

/// Spread estimate used by [`SpreadPlanner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadEstimate {
    edge_bps: i32,
    executable: bool,
    buy_qty: OrderQty,
    sell_qty: OrderQty,
}

impl SpreadEstimate {
    /// Returns current spread edge in basis points.
    pub const fn edge_bps(&self) -> i32 {
        self.edge_bps
    }

    /// Returns true when edge meets the configured threshold.
    pub const fn executable(&self) -> bool {
        self.executable
    }

    /// Returns planned buy-leg quantity.
    pub const fn buy_qty(&self) -> OrderQty {
        self.buy_qty
    }

    /// Returns planned sell-leg quantity.
    pub const fn sell_qty(&self) -> OrderQty {
        self.sell_qty
    }
}

/// Two-leg spread decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadDecision {
    estimate: SpreadEstimate,
    buy: Option<ChildOrderPlan>,
    sell: Option<ChildOrderPlan>,
}

impl SpreadDecision {
    /// Returns estimate used by the decision.
    pub const fn estimate(&self) -> SpreadEstimate {
        self.estimate
    }

    /// Returns optional buy-leg child order.
    pub const fn buy(&self) -> Option<ChildOrderPlan> {
        self.buy
    }

    /// Returns optional sell-leg child order.
    pub const fn sell(&self) -> Option<ChildOrderPlan> {
        self.sell
    }
}

/// Deterministic two-leg pairs/spread planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadPlanner {
    config: SpreadConfig,
}

impl SpreadPlanner {
    /// Creates a spread planner.
    pub const fn new(config: SpreadConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> SpreadConfig {
        self.config
    }

    /// Estimates edge and executable leg quantities.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when leg, progress, or quote inputs are invalid.
    pub fn estimate(
        &self,
        buy_parent: &ParentOrder,
        buy_progress: AlgoProgress,
        sell_parent: &ParentOrder,
        sell_progress: AlgoProgress,
        quote: SpreadQuote,
    ) -> Result<SpreadEstimate, AlgoError> {
        self.validate_legs(buy_parent, buy_progress, sell_parent, sell_progress)?;
        let edge_bps = spread_edge_bps(
            quote.buy_price(),
            quote.sell_price(),
            self.config.ratio_bps(),
        );
        let executable = edge_bps >= self.config.min_edge_bps();
        let buy_leaves = buy_parent
            .total_qty()
            .0
            .saturating_sub(buy_progress.released_qty().0)
            .min(buy_parent.max_clip().0);
        let sell_leaves = sell_parent
            .total_qty()
            .0
            .saturating_sub(sell_progress.released_qty().0)
            .min(sell_parent.max_clip().0);
        let buy_from_sell = scale_qty_inverse_bps(sell_leaves, self.config.ratio_bps());
        let buy_qty = buy_leaves.min(buy_from_sell);
        let sell_qty = scale_qty_bps(buy_qty, self.config.ratio_bps());
        Ok(SpreadEstimate {
            edge_bps,
            executable,
            buy_qty: OrderQty(buy_qty),
            sell_qty: OrderQty(sell_qty.min(sell_leaves)),
        })
    }

    /// Plans synchronized buy/sell spread child orders.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when inputs are invalid or generated child orders
    /// fail OMS request validation.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns both legs, ids, quotes, and timestamps"
    )]
    pub fn plan_spread(
        &self,
        buy_parent: &ParentOrder,
        buy_progress: AlgoProgress,
        sell_parent: &ParentOrder,
        sell_progress: AlgoProgress,
        now_ns: u64,
        quote: SpreadQuote,
        buy_child_id: ChildOrderId,
        buy_client_order_id: ClientOrderId,
        sell_child_id: ChildOrderId,
        sell_client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<SpreadDecision, AlgoError> {
        if now_ns < buy_parent.start_ns().max(sell_parent.start_ns())
            || buy_progress.is_complete()
            || sell_progress.is_complete()
        {
            let estimate =
                self.estimate(buy_parent, buy_progress, sell_parent, sell_progress, quote)?;
            return Ok(SpreadDecision {
                estimate: SpreadEstimate {
                    executable: false,
                    ..estimate
                },
                buy: None,
                sell: None,
            });
        }
        let estimate =
            self.estimate(buy_parent, buy_progress, sell_parent, sell_progress, quote)?;
        if !estimate.executable()
            || estimate.buy_qty().0 < buy_parent.min_clip().0
            || estimate.sell_qty().0 < sell_parent.min_clip().0
        {
            return Ok(SpreadDecision {
                estimate,
                buy: None,
                sell: None,
            });
        }
        let buy_request = buy_parent.build_order_request_for_side_at_price(
            OrderSide::Buy,
            buy_client_order_id,
            estimate.buy_qty(),
            quote.buy_price(),
            ts_recv_ns,
        );
        let sell_request = sell_parent.build_order_request_for_side_at_price(
            OrderSide::Sell,
            sell_client_order_id,
            estimate.sell_qty(),
            quote.sell_price(),
            ts_recv_ns,
        );
        Ok(SpreadDecision {
            estimate,
            buy: Some(ChildOrderPlan::new(
                buy_child_id,
                buy_parent.id(),
                buy_request,
                now_ns,
            )?),
            sell: Some(ChildOrderPlan::new(
                sell_child_id,
                sell_parent.id(),
                sell_request,
                now_ns,
            )?),
        })
    }

    fn validate_legs(
        &self,
        buy_parent: &ParentOrder,
        buy_progress: AlgoProgress,
        sell_parent: &ParentOrder,
        sell_progress: AlgoProgress,
    ) -> Result<(), AlgoError> {
        buy_parent.validate()?;
        sell_parent.validate()?;
        if buy_parent.status().is_terminal() || sell_parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if buy_parent.side() != OrderSide::Buy || sell_parent.side() != OrderSide::Sell {
            return Err(AlgoError::InvalidSpreadParameters);
        }
        if buy_progress.parent_id() != buy_parent.id()
            || buy_progress.target_qty() != buy_parent.total_qty()
            || sell_progress.parent_id() != sell_parent.id()
            || sell_progress.target_qty() != sell_parent.total_qty()
        {
            return Err(AlgoError::InvalidProgress);
        }
        Ok(())
    }
}
