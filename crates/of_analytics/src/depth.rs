use super::*;

/// Borrowed depth analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityDepthAnalyzer {
    levels: usize,
}

impl LiquidityDepthAnalyzer {
    /// Creates a depth analyzer.
    pub const fn new(levels: usize) -> Self {
        Self { levels }
    }

    /// Returns configured level count.
    pub const fn levels(&self) -> usize {
        self.levels
    }

    /// Analyzes borrowed bid/ask levels.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDepth`] when level count is zero,
    /// levels are missing, or a level has a non-positive price or negative
    /// size.
    pub fn analyze(
        &self,
        bids: &[BookLevel],
        asks: &[BookLevel],
        target_qty: i64,
    ) -> Result<LiquidityDepthSnapshot, AnalyticsError> {
        if self.levels == 0 || bids.is_empty() || asks.is_empty() || target_qty < 0 {
            return Err(AnalyticsError::InvalidDepth);
        }
        validate_levels(bids)?;
        validate_levels(asks)?;
        let bid_depth = depth_sum(bids, self.levels);
        let ask_depth = depth_sum(asks, self.levels);
        let total_depth = bid_depth.saturating_add(ask_depth);
        let proportional_imbalance_bps = if total_depth <= 0 {
            0
        } else {
            i32::try_from(
                (i128::from(bid_depth.saturating_sub(ask_depth)) * 10_000)
                    / i128::from(total_depth),
            )
            .unwrap_or(0)
        };
        let sweepable_buy_qty = sweep_qty(asks, target_qty);
        let sweepable_sell_qty = sweep_qty(bids, target_qty);
        let buy_sweepability_bps = sweepability_bps(sweepable_buy_qty, target_qty);
        let sell_sweepability_bps = sweepability_bps(sweepable_sell_qty, target_qty);
        Ok(LiquidityDepthSnapshot {
            levels_used: self.levels.min(bids.len()).min(asks.len()),
            top_bid_qty: bids[0].size,
            top_ask_qty: asks[0].size,
            bid_depth,
            ask_depth,
            proportional_imbalance_bps,
            depth_slope_bps: depth_slope_bps(bids, asks, self.levels),
            depth_convexity_bps: depth_convexity_bps(bids, asks, self.levels),
            book_pressure_bps: book_pressure_bps(bids, asks, self.levels),
            sweepable_buy_qty,
            sweepable_sell_qty,
            buy_sweepability_bps,
            sell_sweepability_bps,
            sweepability_score_bps: buy_sweepability_bps.min(sell_sweepability_bps),
        })
    }
}
