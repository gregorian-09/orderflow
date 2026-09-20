use super::*;

/// Queue update kind.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum QueueUpdateKind {
    /// Aggressive trade consumed displayed quantity at the order price.
    Trade = 1,
    /// Displayed quantity decreased without a known trade.
    Cancel = 2,
    /// Local order was amended and likely lost queue priority.
    Amend = 3,
}

/// Passive order queue estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuePositionEstimate {
    qty_ahead: i64,
    own_qty: i64,
    total_queue_qty: i64,
    ts_ns: u64,
}

impl QueuePositionEstimate {
    /// Creates a queue position estimate.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQueue`] when quantities or timestamp
    /// are invalid.
    pub const fn new(
        qty_ahead: i64,
        own_qty: i64,
        total_queue_qty: i64,
        ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if qty_ahead < 0 || own_qty <= 0 || total_queue_qty < qty_ahead || ts_ns == 0 {
            return Err(AnalyticsError::InvalidQueue);
        }
        Ok(Self {
            qty_ahead,
            own_qty,
            total_queue_qty,
            ts_ns,
        })
    }

    /// Returns estimated quantity ahead.
    pub const fn qty_ahead(&self) -> i64 {
        self.qty_ahead
    }

    /// Returns local displayed quantity.
    pub const fn own_qty(&self) -> i64 {
        self.own_qty
    }

    /// Returns total displayed queue quantity at the price level.
    pub const fn total_queue_qty(&self) -> i64 {
        self.total_queue_qty
    }

    /// Returns estimate timestamp.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }
}

/// Queue/fill probability configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFillConfig {
    cancel_ahead_bps: u16,
    expected_depletion_per_sec: i64,
    horizon_ns: u64,
}

impl QueueFillConfig {
    /// Creates queue/fill configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQueue`] when basis points exceed
    /// 10,000, expected depletion is negative, or horizon is zero.
    pub const fn new(
        cancel_ahead_bps: u16,
        expected_depletion_per_sec: i64,
        horizon_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if cancel_ahead_bps > 10_000 || expected_depletion_per_sec < 0 || horizon_ns == 0 {
            return Err(AnalyticsError::InvalidQueue);
        }
        Ok(Self {
            cancel_ahead_bps,
            expected_depletion_per_sec,
            horizon_ns,
        })
    }

    /// Returns assumed percentage of cancels ahead of the local order.
    pub const fn cancel_ahead_bps(&self) -> u16 {
        self.cancel_ahead_bps
    }

    /// Returns expected queue depletion per second.
    pub const fn expected_depletion_per_sec(&self) -> i64 {
        self.expected_depletion_per_sec
    }

    /// Returns fill-probability horizon.
    pub const fn horizon_ns(&self) -> u64 {
        self.horizon_ns
    }
}

/// Queue update at the local order price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFillUpdate {
    kind: QueueUpdateKind,
    qty: i64,
    total_queue_qty: i64,
    ts_ns: u64,
}

impl QueueFillUpdate {
    /// Creates a queue update.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQueue`] when quantity, total queue, or
    /// timestamp is invalid.
    pub const fn new(
        kind: QueueUpdateKind,
        qty: i64,
        total_queue_qty: i64,
        ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if qty < 0 || total_queue_qty < 0 || ts_ns == 0 {
            return Err(AnalyticsError::InvalidQueue);
        }
        Ok(Self {
            kind,
            qty,
            total_queue_qty,
            ts_ns,
        })
    }

    /// Returns update kind.
    pub const fn kind(&self) -> QueueUpdateKind {
        self.kind
    }

    /// Returns update quantity.
    pub const fn qty(&self) -> i64 {
        self.qty
    }

    /// Returns current total queue quantity.
    pub const fn total_queue_qty(&self) -> i64 {
        self.total_queue_qty
    }

    /// Returns update timestamp.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }
}

/// Passive fill probability snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFillSnapshot {
    qty_ahead: i64,
    own_qty_remaining: i64,
    total_queue_qty: i64,
    fill_probability_bps: u16,
    expected_time_to_fill_ns: u64,
    queue_loss_after_amend: i64,
    maker_taker_score_bps: u16,
    top_level_survival_bps: u16,
    last_update_ts_ns: u64,
}

impl QueueFillSnapshot {
    /// Returns estimated quantity ahead.
    pub const fn qty_ahead(&self) -> i64 {
        self.qty_ahead
    }

    /// Returns remaining local quantity.
    pub const fn own_qty_remaining(&self) -> i64 {
        self.own_qty_remaining
    }

    /// Returns current total queue quantity.
    pub const fn total_queue_qty(&self) -> i64 {
        self.total_queue_qty
    }

    /// Returns fill probability over configured horizon in basis points.
    pub const fn fill_probability_bps(&self) -> u16 {
        self.fill_probability_bps
    }

    /// Returns expected time to fill in nanoseconds.
    pub const fn expected_time_to_fill_ns(&self) -> u64 {
        self.expected_time_to_fill_ns
    }

    /// Returns estimated queue quantity lost after amend.
    pub const fn queue_loss_after_amend(&self) -> i64 {
        self.queue_loss_after_amend
    }

    /// Returns maker/taker preference score in basis points.
    pub const fn maker_taker_score_bps(&self) -> u16 {
        self.maker_taker_score_bps
    }

    /// Returns top-level survival probability proxy in basis points.
    pub const fn top_level_survival_bps(&self) -> u16 {
        self.top_level_survival_bps
    }

    /// Returns latest update timestamp.
    pub const fn last_update_ts_ns(&self) -> u64 {
        self.last_update_ts_ns
    }
}

/// Queue/fill probability tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFillTracker {
    config: QueueFillConfig,
    qty_ahead: i64,
    own_qty_remaining: i64,
    total_queue_qty: i64,
    queue_loss_after_amend: i64,
    last_update_ts_ns: u64,
}

impl QueueFillTracker {
    /// Creates a queue/fill tracker.
    pub const fn new(config: QueueFillConfig, estimate: QueuePositionEstimate) -> Self {
        Self {
            config,
            qty_ahead: estimate.qty_ahead(),
            own_qty_remaining: estimate.own_qty(),
            total_queue_qty: estimate.total_queue_qty(),
            queue_loss_after_amend: 0,
            last_update_ts_ns: estimate.ts_ns(),
        }
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> QueueFillConfig {
        self.config
    }

    /// Records one queue update.
    pub fn on_update(&mut self, update: QueueFillUpdate) -> QueueFillSnapshot {
        match update.kind() {
            QueueUpdateKind::Trade => self.apply_trade(update.qty()),
            QueueUpdateKind::Cancel => self.apply_cancel(update.qty()),
            QueueUpdateKind::Amend => self.apply_amend(update.total_queue_qty()),
        }
        self.total_queue_qty = update.total_queue_qty();
        self.last_update_ts_ns = update.ts_ns();
        self.snapshot()
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> QueueFillSnapshot {
        let fill_probability_bps = self.fill_probability_bps();
        QueueFillSnapshot {
            qty_ahead: self.qty_ahead,
            own_qty_remaining: self.own_qty_remaining,
            total_queue_qty: self.total_queue_qty,
            fill_probability_bps,
            expected_time_to_fill_ns: self.expected_time_to_fill_ns(),
            queue_loss_after_amend: self.queue_loss_after_amend,
            maker_taker_score_bps: fill_probability_bps,
            top_level_survival_bps: self.top_level_survival_bps(),
            last_update_ts_ns: self.last_update_ts_ns,
        }
    }

    fn apply_trade(&mut self, qty: i64) {
        let ahead_take = self.qty_ahead.min(qty);
        self.qty_ahead = self.qty_ahead.saturating_sub(ahead_take);
        let remaining = qty.saturating_sub(ahead_take);
        self.own_qty_remaining = self.own_qty_remaining.saturating_sub(remaining);
    }

    fn apply_cancel(&mut self, qty: i64) {
        let ahead_cancel =
            ((i128::from(qty) * i128::from(self.config.cancel_ahead_bps())) / 10_000) as i64;
        self.qty_ahead = self.qty_ahead.saturating_sub(ahead_cancel);
    }

    fn apply_amend(&mut self, new_total_queue_qty: i64) {
        self.queue_loss_after_amend = new_total_queue_qty;
        self.qty_ahead = new_total_queue_qty;
    }

    fn fill_probability_bps(&self) -> u16 {
        let needed = self.qty_ahead.saturating_add(self.own_qty_remaining);
        if needed <= 0 {
            return 10_000;
        }
        let expected = (i128::from(self.config.expected_depletion_per_sec())
            * i128::from(self.config.horizon_ns()))
            / 1_000_000_000;
        if expected <= 0 {
            return 0;
        }
        u16::try_from(((expected * 10_000) / i128::from(needed)).clamp(0, 10_000)).unwrap_or(10_000)
    }

    fn expected_time_to_fill_ns(&self) -> u64 {
        let needed = self.qty_ahead.saturating_add(self.own_qty_remaining);
        if needed <= 0 {
            return 0;
        }
        let rate = self.config.expected_depletion_per_sec();
        if rate <= 0 {
            return u64::MAX;
        }
        u64::try_from((i128::from(needed) * 1_000_000_000) / i128::from(rate)).unwrap_or(u64::MAX)
    }

    fn top_level_survival_bps(&self) -> u16 {
        if self.total_queue_qty <= 0 {
            return 0;
        }
        let pressure =
            ((i128::from(self.qty_ahead) * 10_000) / i128::from(self.total_queue_qty)).min(10_000);
        u16::try_from(10_000_i128.saturating_sub(pressure)).unwrap_or(0)
    }
}

/// Queue decision thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueDecisionConfig {
    min_fill_probability_bps: u16,
    min_top_level_survival_bps: u16,
    max_wait_ns: u64,
}

impl QueueDecisionConfig {
    /// Creates queue decision thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQueue`] when basis-point thresholds
    /// exceed 10,000 or the maximum wait is zero.
    pub const fn new(
        min_fill_probability_bps: u16,
        min_top_level_survival_bps: u16,
        max_wait_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if min_fill_probability_bps > 10_000
            || min_top_level_survival_bps > 10_000
            || max_wait_ns == 0
        {
            return Err(AnalyticsError::InvalidQueue);
        }
        Ok(Self {
            min_fill_probability_bps,
            min_top_level_survival_bps,
            max_wait_ns,
        })
    }

    /// Returns minimum passive fill probability in basis points.
    pub const fn min_fill_probability_bps(&self) -> u16 {
        self.min_fill_probability_bps
    }

    /// Returns minimum top-level survival probability in basis points.
    pub const fn min_top_level_survival_bps(&self) -> u16 {
        self.min_top_level_survival_bps
    }

    /// Returns maximum acceptable passive wait in nanoseconds.
    pub const fn max_wait_ns(&self) -> u64 {
        self.max_wait_ns
    }
}

impl Default for QueueDecisionConfig {
    fn default() -> Self {
        Self {
            min_fill_probability_bps: 5_000,
            min_top_level_survival_bps: 5_000,
            max_wait_ns: 1_000_000_000,
        }
    }
}

/// Queue decision economics and replacement context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueDecisionInput {
    snapshot: QueueFillSnapshot,
    spread_bps: u32,
    price_improvement_bps: u32,
    maker_rebate_bps: u32,
    taker_fee_bps: u32,
    adverse_selection_bps: u32,
    urgency_bps: u16,
    replace_price_improvement_bps: u32,
    replace_queue_loss_qty: i64,
}

impl QueueDecisionInput {
    /// Creates queue decision input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQueue`] when urgency exceeds 10,000
    /// basis points or replacement queue loss is negative.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        snapshot: QueueFillSnapshot,
        spread_bps: u32,
        price_improvement_bps: u32,
        maker_rebate_bps: u32,
        taker_fee_bps: u32,
        adverse_selection_bps: u32,
        urgency_bps: u16,
        replace_price_improvement_bps: u32,
        replace_queue_loss_qty: i64,
    ) -> Result<Self, AnalyticsError> {
        if urgency_bps > 10_000 || replace_queue_loss_qty < 0 {
            return Err(AnalyticsError::InvalidQueue);
        }
        Ok(Self {
            snapshot,
            spread_bps,
            price_improvement_bps,
            maker_rebate_bps,
            taker_fee_bps,
            adverse_selection_bps,
            urgency_bps,
            replace_price_improvement_bps,
            replace_queue_loss_qty,
        })
    }

    /// Returns queue/fill snapshot.
    pub const fn snapshot(&self) -> QueueFillSnapshot {
        self.snapshot
    }

    /// Returns current spread in basis points.
    pub const fn spread_bps(&self) -> u32 {
        self.spread_bps
    }

    /// Returns passive price-improvement benefit in basis points.
    pub const fn price_improvement_bps(&self) -> u32 {
        self.price_improvement_bps
    }

    /// Returns maker rebate benefit in basis points.
    pub const fn maker_rebate_bps(&self) -> u32 {
        self.maker_rebate_bps
    }

    /// Returns taker fee cost in basis points.
    pub const fn taker_fee_bps(&self) -> u32 {
        self.taker_fee_bps
    }

    /// Returns passive adverse-selection cost in basis points.
    pub const fn adverse_selection_bps(&self) -> u32 {
        self.adverse_selection_bps
    }

    /// Returns execution urgency in basis points.
    pub const fn urgency_bps(&self) -> u16 {
        self.urgency_bps
    }

    /// Returns replacement price-improvement benefit in basis points.
    pub const fn replace_price_improvement_bps(&self) -> u32 {
        self.replace_price_improvement_bps
    }

    /// Returns estimated replacement queue loss quantity.
    pub const fn replace_queue_loss_qty(&self) -> i64 {
        self.replace_queue_loss_qty
    }
}

/// Queue decision snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueDecisionSnapshot {
    passive_edge_bps: i32,
    aggressive_cost_bps: u32,
    expected_wait_penalty_bps: u16,
    queue_priority_loss_bps: u16,
    cancel_replace_cost_bps: i32,
    maker_taker_decision_score_bps: u16,
    prefer_passive: bool,
    prefer_replace: bool,
}

impl QueueDecisionSnapshot {
    /// Returns passive edge estimate in basis points.
    pub const fn passive_edge_bps(&self) -> i32 {
        self.passive_edge_bps
    }

    /// Returns aggressive execution cost estimate in basis points.
    pub const fn aggressive_cost_bps(&self) -> u32 {
        self.aggressive_cost_bps
    }

    /// Returns passive wait penalty in basis points.
    pub const fn expected_wait_penalty_bps(&self) -> u16 {
        self.expected_wait_penalty_bps
    }

    /// Returns queue priority loss from replacement in basis points.
    pub const fn queue_priority_loss_bps(&self) -> u16 {
        self.queue_priority_loss_bps
    }

    /// Returns cancel/replace cost estimate in basis points.
    pub const fn cancel_replace_cost_bps(&self) -> i32 {
        self.cancel_replace_cost_bps
    }

    /// Returns maker-versus-taker decision score in basis points.
    pub const fn maker_taker_decision_score_bps(&self) -> u16 {
        self.maker_taker_decision_score_bps
    }

    /// Returns whether passive execution is preferred.
    pub const fn prefer_passive(&self) -> bool {
        self.prefer_passive
    }

    /// Returns whether cancel/replace is preferred over keeping queue priority.
    pub const fn prefer_replace(&self) -> bool {
        self.prefer_replace
    }
}

/// Deterministic queue decision analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueDecisionAnalyzer {
    config: QueueDecisionConfig,
}

impl QueueDecisionAnalyzer {
    /// Creates a queue decision analyzer.
    pub const fn new(config: QueueDecisionConfig) -> Self {
        Self { config }
    }

    /// Returns analyzer configuration.
    pub const fn config(&self) -> QueueDecisionConfig {
        self.config
    }

    /// Evaluates passive, aggressive, and replacement queue trade-offs.
    pub fn evaluate(&self, input: QueueDecisionInput) -> QueueDecisionSnapshot {
        let snapshot = input.snapshot();
        let wait_penalty = wait_penalty_bps(
            snapshot.expected_time_to_fill_ns(),
            self.config.max_wait_ns(),
            input.urgency_bps(),
        );
        let queue_priority_loss =
            queue_priority_loss_bps(input.replace_queue_loss_qty(), snapshot.total_queue_qty());
        let passive_edge = i32::try_from(
            input
                .spread_bps()
                .saturating_add(input.price_improvement_bps())
                .saturating_add(input.maker_rebate_bps()),
        )
        .unwrap_or(i32::MAX)
        .saturating_sub(i32::try_from(input.adverse_selection_bps()).unwrap_or(i32::MAX))
        .saturating_sub(i32::from(wait_penalty));
        let aggressive_cost = input.taker_fee_bps().saturating_add(input.spread_bps() / 2);
        let cancel_replace_cost = i32::from(queue_priority_loss)
            .saturating_add(i32::from(wait_penalty))
            .saturating_sub(
                i32::try_from(input.replace_price_improvement_bps()).unwrap_or(i32::MAX),
            );
        let decision_score = maker_taker_score_bps(
            passive_edge,
            aggressive_cost,
            snapshot.fill_probability_bps(),
            snapshot.top_level_survival_bps(),
            input.urgency_bps(),
        );
        let prefer_passive = decision_score >= 5_000
            && snapshot.fill_probability_bps() >= self.config.min_fill_probability_bps()
            && snapshot.top_level_survival_bps() >= self.config.min_top_level_survival_bps();
        QueueDecisionSnapshot {
            passive_edge_bps: passive_edge,
            aggressive_cost_bps: aggressive_cost,
            expected_wait_penalty_bps: wait_penalty,
            queue_priority_loss_bps: queue_priority_loss,
            cancel_replace_cost_bps: cancel_replace_cost,
            maker_taker_decision_score_bps: decision_score,
            prefer_passive,
            prefer_replace: cancel_replace_cost < 0 && input.replace_price_improvement_bps() > 0,
        }
    }
}

impl Default for QueueDecisionAnalyzer {
    fn default() -> Self {
        Self::new(QueueDecisionConfig::default())
    }
}
