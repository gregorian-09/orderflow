use super::*;

/// Passive peg reference used by [`PassiveQueuePlanner`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PassivePegMode {
    /// Quote at the same-side best price.
    SameSide = 1,
    /// Quote at the midpoint when the spread allows it.
    Midpoint = 2,
    /// Quote one passive tick inside the spread when possible.
    ImproveOneTick = 3,
}

/// Passive queue action selected by [`PassiveQueuePlanner`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PassiveQueueAction {
    /// Do not release a child order for this decision.
    Wait = 1,
    /// Join the selected passive queue.
    JoinQueue = 2,
    /// Improve the selected passive price inside the spread.
    ImprovePrice = 3,
    /// Cross the spread because the configured time budget is exhausted.
    CrossSpread = 4,
}

impl PassiveQueueAction {
    /// Returns true when the action releases a child order.
    pub const fn releases_child(self) -> bool {
        matches!(
            self,
            Self::JoinQueue | Self::ImprovePrice | Self::CrossSpread
        )
    }
}

/// Market context for passive queue planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveQueueContext {
    best_bid: OrderPrice,
    best_ask: OrderPrice,
    queue_ahead_qty: OrderQty,
    expected_take_qty: OrderQty,
    adverse_selection_bps: u16,
}

impl PassiveQueueContext {
    /// Creates passive queue market context.
    ///
    /// `queue_ahead_qty` is the visible or modeled quantity ahead of the
    /// planned child at the candidate price. `expected_take_qty` is the host's
    /// short-horizon estimate of contra-side volume that can trade through the
    /// queue.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidPassiveQueueParameters`] when best bid/ask
    /// are non-positive, crossed, or queue quantities are negative.
    pub const fn new(
        best_bid: OrderPrice,
        best_ask: OrderPrice,
        queue_ahead_qty: OrderQty,
        expected_take_qty: OrderQty,
        adverse_selection_bps: u16,
    ) -> Result<Self, AlgoError> {
        if best_bid.0 <= 0
            || best_ask.0 <= 0
            || best_bid.0 >= best_ask.0
            || queue_ahead_qty.0 < 0
            || expected_take_qty.0 < 0
        {
            return Err(AlgoError::InvalidPassiveQueueParameters);
        }
        Ok(Self {
            best_bid,
            best_ask,
            queue_ahead_qty,
            expected_take_qty,
            adverse_selection_bps,
        })
    }

    /// Returns the current best bid.
    pub const fn best_bid(&self) -> OrderPrice {
        self.best_bid
    }

    /// Returns the current best ask.
    pub const fn best_ask(&self) -> OrderPrice {
        self.best_ask
    }

    /// Returns estimated quantity ahead at the candidate queue.
    pub const fn queue_ahead_qty(&self) -> OrderQty {
        self.queue_ahead_qty
    }

    /// Returns expected contra-side trade quantity at the candidate queue.
    pub const fn expected_take_qty(&self) -> OrderQty {
        self.expected_take_qty
    }

    /// Returns adverse-selection estimate in basis points.
    pub const fn adverse_selection_bps(&self) -> u16 {
        self.adverse_selection_bps
    }
}

/// Configuration for passive queue planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveQueueConfig {
    peg_mode: PassivePegMode,
    tick_size: OrderPrice,
    min_fill_probability_bps: u16,
    max_adverse_selection_bps: u16,
    improve_when_fill_below_bps: u16,
    max_improvement_ticks: u16,
    allow_cross: bool,
    cross_after_elapsed_bps: u16,
}

impl PassiveQueueConfig {
    /// Creates passive queue configuration with conservative defaults.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidPassiveQueueParameters`] when tick size is
    /// not positive.
    pub const fn new(peg_mode: PassivePegMode, tick_size: OrderPrice) -> Result<Self, AlgoError> {
        if tick_size.0 <= 0 {
            return Err(AlgoError::InvalidPassiveQueueParameters);
        }
        Ok(Self {
            peg_mode,
            tick_size,
            min_fill_probability_bps: 2_500,
            max_adverse_selection_bps: 250,
            improve_when_fill_below_bps: 1_500,
            max_improvement_ticks: 1,
            allow_cross: false,
            cross_after_elapsed_bps: 9_500,
        })
    }

    /// Returns a copy with queue thresholds updated.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidPassiveQueueParameters`] when any basis
    /// point value exceeds 10,000.
    pub const fn with_thresholds(
        mut self,
        min_fill_probability_bps: u16,
        max_adverse_selection_bps: u16,
        improve_when_fill_below_bps: u16,
    ) -> Result<Self, AlgoError> {
        if min_fill_probability_bps > 10_000
            || max_adverse_selection_bps > 10_000
            || improve_when_fill_below_bps > 10_000
        {
            return Err(AlgoError::InvalidPassiveQueueParameters);
        }
        self.min_fill_probability_bps = min_fill_probability_bps;
        self.max_adverse_selection_bps = max_adverse_selection_bps;
        self.improve_when_fill_below_bps = improve_when_fill_below_bps;
        Ok(self)
    }

    /// Returns a copy with maximum inside-spread improvement ticks updated.
    pub const fn with_max_improvement_ticks(mut self, max_improvement_ticks: u16) -> Self {
        self.max_improvement_ticks = max_improvement_ticks;
        self
    }

    /// Returns a copy with crossing behavior updated.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidPassiveQueueParameters`] when elapsed basis
    /// points exceed 10,000.
    pub const fn with_crossing(
        mut self,
        allow_cross: bool,
        cross_after_elapsed_bps: u16,
    ) -> Result<Self, AlgoError> {
        if cross_after_elapsed_bps > 10_000 {
            return Err(AlgoError::InvalidPassiveQueueParameters);
        }
        self.allow_cross = allow_cross;
        self.cross_after_elapsed_bps = cross_after_elapsed_bps;
        Ok(self)
    }

    /// Returns peg mode.
    pub const fn peg_mode(&self) -> PassivePegMode {
        self.peg_mode
    }

    /// Returns tick size in normalized price units.
    pub const fn tick_size(&self) -> OrderPrice {
        self.tick_size
    }

    /// Returns minimum fill probability in basis points.
    pub const fn min_fill_probability_bps(&self) -> u16 {
        self.min_fill_probability_bps
    }

    /// Returns maximum tolerated adverse selection in basis points.
    pub const fn max_adverse_selection_bps(&self) -> u16 {
        self.max_adverse_selection_bps
    }

    /// Returns fill-probability threshold below which improvement is preferred.
    pub const fn improve_when_fill_below_bps(&self) -> u16 {
        self.improve_when_fill_below_bps
    }

    /// Returns maximum passive improvement ticks.
    pub const fn max_improvement_ticks(&self) -> u16 {
        self.max_improvement_ticks
    }

    /// Returns true when the planner may cross the spread after the time
    /// threshold.
    pub const fn allow_cross(&self) -> bool {
        self.allow_cross
    }

    /// Returns elapsed parent interval threshold for crossing.
    pub const fn cross_after_elapsed_bps(&self) -> u16 {
        self.cross_after_elapsed_bps
    }
}

impl Default for PassiveQueueConfig {
    fn default() -> Self {
        Self::new(PassivePegMode::SameSide, OrderPrice(1))
            .expect("static passive queue defaults are valid")
    }
}

/// Passive queue planning estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveQueueEstimate {
    action: PassiveQueueAction,
    candidate_price: OrderPrice,
    fill_probability_bps: u16,
    elapsed_bps: u16,
    candidate_qty: OrderQty,
}

impl PassiveQueueEstimate {
    /// Returns selected passive queue action.
    pub const fn action(&self) -> PassiveQueueAction {
        self.action
    }

    /// Returns selected limit price.
    pub const fn candidate_price(&self) -> OrderPrice {
        self.candidate_price
    }

    /// Returns estimated fill probability in basis points.
    pub const fn fill_probability_bps(&self) -> u16 {
        self.fill_probability_bps
    }

    /// Returns elapsed parent interval in basis points.
    pub const fn elapsed_bps(&self) -> u16 {
        self.elapsed_bps
    }

    /// Returns candidate child quantity used for the estimate.
    pub const fn candidate_qty(&self) -> OrderQty {
        self.candidate_qty
    }
}

/// Passive queue decision with an optional child order plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveQueueDecision {
    estimate: PassiveQueueEstimate,
    child: Option<ChildOrderPlan>,
}

impl PassiveQueueDecision {
    /// Returns the estimate that drove the decision.
    pub const fn estimate(&self) -> PassiveQueueEstimate {
        self.estimate
    }

    /// Returns the selected action.
    pub const fn action(&self) -> PassiveQueueAction {
        self.estimate.action()
    }

    /// Returns the optional child order plan.
    pub const fn child(&self) -> Option<ChildOrderPlan> {
        self.child
    }
}

/// Deterministic passive peg and queue-position planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveQueuePlanner {
    config: PassiveQueueConfig,
}

impl PassiveQueuePlanner {
    /// Creates a passive queue planner.
    pub const fn new(config: PassiveQueueConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> PassiveQueueConfig {
        self.config
    }

    /// Estimates passive queue action, price, fill probability, and quantity.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/context state is invalid.
    pub fn estimate(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        context: PassiveQueueContext,
    ) -> Result<PassiveQueueEstimate, AlgoError> {
        parent.validate()?;
        self.validate_context(context)?;
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }

        let candidate_qty = passive_candidate_qty(parent, progress, now_ns);
        let fill_probability_bps = queue_fill_probability_bps(
            context.queue_ahead_qty().0,
            candidate_qty.0,
            context.expected_take_qty().0,
        );
        let elapsed = elapsed_bps(parent.start_ns(), parent.end_ns(), now_ns);
        let action = self.select_action(parent, now_ns, context, fill_probability_bps, elapsed);
        let candidate_price = self.price_for_action(parent.side(), context, action);

        Ok(PassiveQueueEstimate {
            action,
            candidate_price,
            fill_probability_bps,
            elapsed_bps: elapsed,
            candidate_qty,
        })
    }

    /// Plans one passive queue child decision.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/context state is invalid or
    /// the generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_passive_slice(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        context: PassiveQueueContext,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<PassiveQueueDecision, AlgoError> {
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if now_ns < parent.start_ns() || progress.is_complete() {
            let estimate = self.estimate(parent, progress, now_ns, context)?;
            return Ok(PassiveQueueDecision {
                estimate: PassiveQueueEstimate {
                    action: PassiveQueueAction::Wait,
                    ..estimate
                },
                child: None,
            });
        }

        let estimate = self.estimate(parent, progress, now_ns, context)?;
        if !estimate.action().releases_child() || estimate.candidate_qty().0 <= 0 {
            return Ok(PassiveQueueDecision {
                estimate,
                child: None,
            });
        }
        let final_slice = progress
            .released_qty()
            .0
            .saturating_add(estimate.candidate_qty().0)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if estimate.candidate_qty().0 < parent.min_clip().0 && !final_slice {
            return Ok(PassiveQueueDecision {
                estimate: PassiveQueueEstimate {
                    action: PassiveQueueAction::Wait,
                    ..estimate
                },
                child: None,
            });
        }

        let request = parent.build_order_request_at_price(
            client_order_id,
            estimate.candidate_qty(),
            estimate.candidate_price(),
            ts_recv_ns,
        );
        Ok(PassiveQueueDecision {
            estimate,
            child: Some(ChildOrderPlan::new(child_id, parent.id(), request, now_ns)?),
        })
    }

    fn validate_context(&self, context: PassiveQueueContext) -> Result<(), AlgoError> {
        if self.config.tick_size().0 <= 0
            || self.config.min_fill_probability_bps() > 10_000
            || self.config.max_adverse_selection_bps() > 10_000
            || self.config.improve_when_fill_below_bps() > 10_000
            || self.config.cross_after_elapsed_bps() > 10_000
            || context.best_bid().0 <= 0
            || context.best_ask().0 <= 0
            || context.best_bid().0 >= context.best_ask().0
            || context.queue_ahead_qty().0 < 0
            || context.expected_take_qty().0 < 0
        {
            return Err(AlgoError::InvalidPassiveQueueParameters);
        }
        Ok(())
    }

    fn select_action(
        &self,
        parent: &ParentOrder,
        now_ns: u64,
        context: PassiveQueueContext,
        fill_probability_bps: u16,
        elapsed: u16,
    ) -> PassiveQueueAction {
        if context.adverse_selection_bps() > self.config.max_adverse_selection_bps() {
            return PassiveQueueAction::Wait;
        }
        if self.config.allow_cross() && elapsed >= self.config.cross_after_elapsed_bps() {
            return PassiveQueueAction::CrossSpread;
        }
        if fill_probability_bps < self.config.improve_when_fill_below_bps()
            && self.can_improve(parent.side(), context)
            && now_ns < parent.end_ns()
        {
            return PassiveQueueAction::ImprovePrice;
        }
        if fill_probability_bps >= self.config.min_fill_probability_bps()
            || self.config.peg_mode() != PassivePegMode::SameSide
        {
            return PassiveQueueAction::JoinQueue;
        }
        PassiveQueueAction::Wait
    }

    fn price_for_action(
        &self,
        side: OrderSide,
        context: PassiveQueueContext,
        action: PassiveQueueAction,
    ) -> OrderPrice {
        match action {
            PassiveQueueAction::Wait | PassiveQueueAction::JoinQueue => {
                self.base_price(side, context)
            }
            PassiveQueueAction::ImprovePrice => self.improved_price(side, context),
            PassiveQueueAction::CrossSpread => match side {
                OrderSide::Buy => context.best_ask(),
                OrderSide::Sell => context.best_bid(),
            },
        }
    }

    fn base_price(&self, side: OrderSide, context: PassiveQueueContext) -> OrderPrice {
        match self.config.peg_mode() {
            PassivePegMode::SameSide => match side {
                OrderSide::Buy => context.best_bid(),
                OrderSide::Sell => context.best_ask(),
            },
            PassivePegMode::Midpoint => midpoint_price(side, context, self.config.tick_size()),
            PassivePegMode::ImproveOneTick => self.improved_price(side, context),
        }
    }

    fn improved_price(&self, side: OrderSide, context: PassiveQueueContext) -> OrderPrice {
        let ticks = i64::from(self.config.max_improvement_ticks().max(1));
        let improvement = self.config.tick_size().0.saturating_mul(ticks);
        match side {
            OrderSide::Buy => {
                let max_passive = context
                    .best_ask()
                    .0
                    .saturating_sub(self.config.tick_size().0);
                OrderPrice(
                    context
                        .best_bid()
                        .0
                        .saturating_add(improvement)
                        .min(max_passive),
                )
            }
            OrderSide::Sell => {
                let min_passive = context
                    .best_bid()
                    .0
                    .saturating_add(self.config.tick_size().0);
                OrderPrice(
                    context
                        .best_ask()
                        .0
                        .saturating_sub(improvement)
                        .max(min_passive),
                )
            }
        }
    }

    fn can_improve(&self, side: OrderSide, context: PassiveQueueContext) -> bool {
        if self.config.max_improvement_ticks() == 0 {
            return false;
        }
        let spread = context.best_ask().0.saturating_sub(context.best_bid().0);
        if spread <= self.config.tick_size().0 {
            return false;
        }
        self.improved_price(side, context) != self.base_price(side, context)
    }
}
