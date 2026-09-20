use super::*;

/// Liquidity/depth snapshot over borrowed book levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityDepthSnapshot {
    pub(crate) levels_used: usize,
    pub(crate) top_bid_qty: i64,
    pub(crate) top_ask_qty: i64,
    pub(crate) bid_depth: i64,
    pub(crate) ask_depth: i64,
    pub(crate) proportional_imbalance_bps: i32,
    pub(crate) depth_slope_bps: i32,
    pub(crate) depth_convexity_bps: i32,
    pub(crate) book_pressure_bps: i32,
    pub(crate) sweepable_buy_qty: i64,
    pub(crate) sweepable_sell_qty: i64,
    pub(crate) buy_sweepability_bps: u16,
    pub(crate) sell_sweepability_bps: u16,
    pub(crate) sweepability_score_bps: u16,
}

impl LiquidityDepthSnapshot {
    /// Returns number of levels included per side.
    pub const fn levels_used(&self) -> usize {
        self.levels_used
    }

    /// Returns top bid quantity.
    pub const fn top_bid_qty(&self) -> i64 {
        self.top_bid_qty
    }

    /// Returns top ask quantity.
    pub const fn top_ask_qty(&self) -> i64 {
        self.top_ask_qty
    }

    /// Returns cumulative bid depth.
    pub const fn bid_depth(&self) -> i64 {
        self.bid_depth
    }

    /// Returns cumulative ask depth.
    pub const fn ask_depth(&self) -> i64 {
        self.ask_depth
    }

    /// Returns bid-minus-ask depth imbalance in basis points.
    pub const fn proportional_imbalance_bps(&self) -> i32 {
        self.proportional_imbalance_bps
    }

    /// Returns simple depth slope proxy in basis points.
    pub const fn depth_slope_bps(&self) -> i32 {
        self.depth_slope_bps
    }

    /// Returns aggregate second-difference depth curvature in basis points.
    pub const fn depth_convexity_bps(&self) -> i32 {
        self.depth_convexity_bps
    }

    /// Returns distance-weighted bid-minus-ask book pressure in basis points.
    pub const fn book_pressure_bps(&self) -> i32 {
        self.book_pressure_bps
    }

    /// Returns ask-side quantity sweepable by a buy order up to target
    /// quantity.
    pub const fn sweepable_buy_qty(&self) -> i64 {
        self.sweepable_buy_qty
    }

    /// Returns bid-side quantity sweepable by a sell order up to target
    /// quantity.
    pub const fn sweepable_sell_qty(&self) -> i64 {
        self.sweepable_sell_qty
    }

    /// Returns buy-side target sweepability in basis points.
    pub const fn buy_sweepability_bps(&self) -> u16 {
        self.buy_sweepability_bps
    }

    /// Returns sell-side target sweepability in basis points.
    pub const fn sell_sweepability_bps(&self) -> u16 {
        self.sell_sweepability_bps
    }

    /// Returns conservative target sweepability in basis points.
    pub const fn sweepability_score_bps(&self) -> u16 {
        self.sweepability_score_bps
    }
}

/// Liquidity-flow event over a book observation interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityFlowEvent {
    side: Side,
    added_qty: i64,
    removed_qty: i64,
    traded_qty: i64,
    ts_ns: u64,
}

impl LiquidityFlowEvent {
    /// Creates a liquidity-flow event.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDepth`] when quantities are negative,
    /// timestamp is zero, or the event carries no quantity.
    pub const fn new(
        side: Side,
        added_qty: i64,
        removed_qty: i64,
        traded_qty: i64,
        ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if added_qty < 0
            || removed_qty < 0
            || traded_qty < 0
            || ts_ns == 0
            || added_qty
                .saturating_add(removed_qty)
                .saturating_add(traded_qty)
                == 0
        {
            return Err(AnalyticsError::InvalidDepth);
        }
        Ok(Self {
            side,
            added_qty,
            removed_qty,
            traded_qty,
            ts_ns,
        })
    }

    /// Returns the book side affected by the event.
    pub const fn side(&self) -> Side {
        self.side
    }

    /// Returns quantity added to the side.
    pub const fn added_qty(&self) -> i64 {
        self.added_qty
    }

    /// Returns quantity removed or canceled from the side.
    pub const fn removed_qty(&self) -> i64 {
        self.removed_qty
    }

    /// Returns quantity traded through the side.
    pub const fn traded_qty(&self) -> i64 {
        self.traded_qty
    }

    /// Returns event timestamp in nanoseconds.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }
}

/// Liquidity-flow tracker configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityFlowConfig {
    window_ns: u64,
    drought_replenishment_bps: u16,
    drought_min_depletion_qty: i64,
}

impl LiquidityFlowConfig {
    /// Creates liquidity-flow configuration.
    ///
    /// `drought_replenishment_bps` compares added quantity with depleted
    /// quantity. For example, `2_500` marks drought risk when replenishment is
    /// less than 25% of depletion.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDepth`] when the window is zero,
    /// threshold exceeds 10,000 basis points, or the minimum depletion quantity
    /// is negative.
    pub const fn new(
        window_ns: u64,
        drought_replenishment_bps: u16,
        drought_min_depletion_qty: i64,
    ) -> Result<Self, AnalyticsError> {
        if window_ns == 0 || drought_replenishment_bps > 10_000 || drought_min_depletion_qty < 0 {
            return Err(AnalyticsError::InvalidDepth);
        }
        Ok(Self {
            window_ns,
            drought_replenishment_bps,
            drought_min_depletion_qty,
        })
    }

    /// Returns configured observation window in nanoseconds.
    pub const fn window_ns(&self) -> u64 {
        self.window_ns
    }

    /// Returns drought replenishment threshold in basis points.
    pub const fn drought_replenishment_bps(&self) -> u16 {
        self.drought_replenishment_bps
    }

    /// Returns minimum depletion quantity before drought can be flagged.
    pub const fn drought_min_depletion_qty(&self) -> i64 {
        self.drought_min_depletion_qty
    }
}

impl Default for LiquidityFlowConfig {
    fn default() -> Self {
        Self {
            window_ns: 1_000_000_000,
            drought_replenishment_bps: 2_500,
            drought_min_depletion_qty: 1,
        }
    }
}

/// Liquidity-flow snapshot over accumulated book events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityFlowSnapshot {
    events: u64,
    elapsed_ns: u64,
    bid_added_qty: i64,
    ask_added_qty: i64,
    bid_depleted_qty: i64,
    ask_depleted_qty: i64,
    bid_traded_qty: i64,
    ask_traded_qty: i64,
    order_flow_imbalance_bps: i32,
    replenishment_rate_per_sec: i64,
    depletion_rate_per_sec: i64,
    liquidity_drought: bool,
}

impl LiquidityFlowSnapshot {
    /// Returns number of events included in the snapshot.
    pub const fn events(&self) -> u64 {
        self.events
    }

    /// Returns elapsed observation time in nanoseconds.
    pub const fn elapsed_ns(&self) -> u64 {
        self.elapsed_ns
    }

    /// Returns bid-side added quantity.
    pub const fn bid_added_qty(&self) -> i64 {
        self.bid_added_qty
    }

    /// Returns ask-side added quantity.
    pub const fn ask_added_qty(&self) -> i64 {
        self.ask_added_qty
    }

    /// Returns bid-side canceled or removed quantity.
    pub const fn bid_depleted_qty(&self) -> i64 {
        self.bid_depleted_qty
    }

    /// Returns ask-side canceled or removed quantity.
    pub const fn ask_depleted_qty(&self) -> i64 {
        self.ask_depleted_qty
    }

    /// Returns bid-side traded quantity.
    pub const fn bid_traded_qty(&self) -> i64 {
        self.bid_traded_qty
    }

    /// Returns ask-side traded quantity.
    pub const fn ask_traded_qty(&self) -> i64 {
        self.ask_traded_qty
    }

    /// Returns signed order-flow imbalance in basis points.
    pub const fn order_flow_imbalance_bps(&self) -> i32 {
        self.order_flow_imbalance_bps
    }

    /// Returns replenishment rate in quantity per second.
    pub const fn replenishment_rate_per_sec(&self) -> i64 {
        self.replenishment_rate_per_sec
    }

    /// Returns depletion rate in quantity per second.
    pub const fn depletion_rate_per_sec(&self) -> i64 {
        self.depletion_rate_per_sec
    }

    /// Returns whether replenishment is low relative to depletion.
    pub const fn liquidity_drought(&self) -> bool {
        self.liquidity_drought
    }
}

/// Allocation-free liquidity-flow tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiquidityFlowTracker {
    config: LiquidityFlowConfig,
    events: u64,
    first_ts_ns: u64,
    last_ts_ns: u64,
    bid_added_qty: i64,
    ask_added_qty: i64,
    bid_depleted_qty: i64,
    ask_depleted_qty: i64,
    bid_traded_qty: i64,
    ask_traded_qty: i64,
}

impl LiquidityFlowTracker {
    /// Creates a liquidity-flow tracker.
    pub const fn new(config: LiquidityFlowConfig) -> Self {
        Self {
            config,
            events: 0,
            first_ts_ns: 0,
            last_ts_ns: 0,
            bid_added_qty: 0,
            ask_added_qty: 0,
            bid_depleted_qty: 0,
            ask_depleted_qty: 0,
            bid_traded_qty: 0,
            ask_traded_qty: 0,
        }
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> LiquidityFlowConfig {
        self.config
    }

    /// Records a liquidity-flow event.
    pub fn on_event(&mut self, event: LiquidityFlowEvent) {
        if self.first_ts_ns == 0 {
            self.first_ts_ns = event.ts_ns();
        }
        self.last_ts_ns = self.last_ts_ns.max(event.ts_ns());
        self.events = self.events.saturating_add(1);
        match event.side() {
            Side::Bid => {
                self.bid_added_qty = self.bid_added_qty.saturating_add(event.added_qty());
                self.bid_depleted_qty = self.bid_depleted_qty.saturating_add(event.removed_qty());
                self.bid_traded_qty = self.bid_traded_qty.saturating_add(event.traded_qty());
            }
            Side::Ask => {
                self.ask_added_qty = self.ask_added_qty.saturating_add(event.added_qty());
                self.ask_depleted_qty = self.ask_depleted_qty.saturating_add(event.removed_qty());
                self.ask_traded_qty = self.ask_traded_qty.saturating_add(event.traded_qty());
            }
        }
    }

    /// Returns current liquidity-flow snapshot.
    pub fn snapshot(&self) -> LiquidityFlowSnapshot {
        let elapsed_ns = self.elapsed_ns();
        let replenishment = self.bid_added_qty.saturating_add(self.ask_added_qty);
        let depletion = self
            .bid_depleted_qty
            .saturating_add(self.ask_depleted_qty)
            .saturating_add(self.bid_traded_qty)
            .saturating_add(self.ask_traded_qty);
        let upward_pressure = self
            .bid_added_qty
            .saturating_add(self.ask_depleted_qty)
            .saturating_add(self.ask_traded_qty);
        let downward_pressure = self
            .ask_added_qty
            .saturating_add(self.bid_depleted_qty)
            .saturating_add(self.bid_traded_qty);
        let total_pressure = upward_pressure.saturating_add(downward_pressure);
        let order_flow_imbalance_bps = if total_pressure <= 0 {
            0
        } else {
            i32::try_from(
                (i128::from(upward_pressure.saturating_sub(downward_pressure)) * 10_000)
                    / i128::from(total_pressure),
            )
            .unwrap_or(0)
        };
        let liquidity_drought = depletion >= self.config.drought_min_depletion_qty()
            && replenishment.saturating_mul(10_000)
                < depletion.saturating_mul(i64::from(self.config.drought_replenishment_bps()));
        LiquidityFlowSnapshot {
            events: self.events,
            elapsed_ns,
            bid_added_qty: self.bid_added_qty,
            ask_added_qty: self.ask_added_qty,
            bid_depleted_qty: self.bid_depleted_qty,
            ask_depleted_qty: self.ask_depleted_qty,
            bid_traded_qty: self.bid_traded_qty,
            ask_traded_qty: self.ask_traded_qty,
            order_flow_imbalance_bps,
            replenishment_rate_per_sec: qty_rate_per_sec(replenishment, elapsed_ns),
            depletion_rate_per_sec: qty_rate_per_sec(depletion, elapsed_ns),
            liquidity_drought,
        }
    }

    /// Resets accumulated liquidity-flow state.
    pub fn reset(&mut self) {
        self.events = 0;
        self.first_ts_ns = 0;
        self.last_ts_ns = 0;
        self.bid_added_qty = 0;
        self.ask_added_qty = 0;
        self.bid_depleted_qty = 0;
        self.ask_depleted_qty = 0;
        self.bid_traded_qty = 0;
        self.ask_traded_qty = 0;
    }

    fn elapsed_ns(&self) -> u64 {
        let observed = self.last_ts_ns.saturating_sub(self.first_ts_ns);
        if observed == 0 {
            self.config.window_ns()
        } else {
            observed.min(self.config.window_ns())
        }
    }
}

impl Default for LiquidityFlowTracker {
    fn default() -> Self {
        Self::new(LiquidityFlowConfig::default())
    }
}
