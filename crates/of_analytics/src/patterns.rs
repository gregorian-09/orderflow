use super::*;

/// Pattern-risk liquidity summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRiskLiquidity {
    executed_qty: i64,
    displayed_depth: i64,
}

impl PatternRiskLiquidity {
    /// Creates a liquidity summary for pattern-risk classification.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidPattern`] when quantities are negative.
    pub const fn new(executed_qty: i64, displayed_depth: i64) -> Result<Self, AnalyticsError> {
        if executed_qty < 0 || displayed_depth < 0 {
            return Err(AnalyticsError::InvalidPattern);
        }
        Ok(Self {
            executed_qty,
            displayed_depth,
        })
    }

    /// Returns executed quantity.
    pub const fn executed_qty(&self) -> i64 {
        self.executed_qty
    }

    /// Returns displayed depth.
    pub const fn displayed_depth(&self) -> i64 {
        self.displayed_depth
    }
}

/// Pattern-risk input over a bounded observation window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRiskInput {
    quote_adds: u64,
    quote_cancels: u64,
    trades: u64,
    depth_imbalance_bps: i32,
    price_move_bps: i32,
    liquidity: PatternRiskLiquidity,
    window_ns: u64,
}

impl PatternRiskInput {
    /// Creates pattern-risk input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidPattern`] when quantities or window are
    /// invalid.
    pub const fn new(
        quote_adds: u64,
        quote_cancels: u64,
        trades: u64,
        depth_imbalance_bps: i32,
        price_move_bps: i32,
        liquidity: PatternRiskLiquidity,
        window_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if window_ns == 0 {
            return Err(AnalyticsError::InvalidPattern);
        }
        Ok(Self {
            quote_adds,
            quote_cancels,
            trades,
            depth_imbalance_bps,
            price_move_bps,
            liquidity,
            window_ns,
        })
    }

    /// Returns quote add count.
    pub const fn quote_adds(&self) -> u64 {
        self.quote_adds
    }

    /// Returns quote cancel count.
    pub const fn quote_cancels(&self) -> u64 {
        self.quote_cancels
    }

    /// Returns trade count.
    pub const fn trades(&self) -> u64 {
        self.trades
    }

    /// Returns depth imbalance in basis points.
    pub const fn depth_imbalance_bps(&self) -> i32 {
        self.depth_imbalance_bps
    }

    /// Returns price movement in basis points.
    pub const fn price_move_bps(&self) -> i32 {
        self.price_move_bps
    }

    /// Returns executed quantity.
    pub const fn executed_qty(&self) -> i64 {
        self.liquidity.executed_qty()
    }

    /// Returns displayed depth.
    pub const fn displayed_depth(&self) -> i64 {
        self.liquidity.displayed_depth()
    }

    /// Returns liquidity summary.
    pub const fn liquidity(&self) -> PatternRiskLiquidity {
        self.liquidity
    }

    /// Returns observation window in nanoseconds.
    pub const fn window_ns(&self) -> u64 {
        self.window_ns
    }
}

/// Pattern-risk classifier thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRiskConfig {
    high_order_to_trade_bps: u32,
    high_cancel_ratio_bps: u16,
    high_imbalance_bps: u16,
    high_price_move_bps: u32,
    high_quote_events: u64,
}

impl PatternRiskConfig {
    /// Creates pattern-risk thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidPattern`] when basis-point thresholds
    /// exceed 10,000 where applicable.
    pub const fn new(
        high_order_to_trade_bps: u32,
        high_cancel_ratio_bps: u16,
        high_imbalance_bps: u16,
        high_price_move_bps: u32,
        high_quote_events: u64,
    ) -> Result<Self, AnalyticsError> {
        if high_cancel_ratio_bps > 10_000 || high_imbalance_bps > 10_000 {
            return Err(AnalyticsError::InvalidPattern);
        }
        Ok(Self {
            high_order_to_trade_bps,
            high_cancel_ratio_bps,
            high_imbalance_bps,
            high_price_move_bps,
            high_quote_events,
        })
    }
}

impl Default for PatternRiskConfig {
    fn default() -> Self {
        Self {
            high_order_to_trade_bps: 50_000,
            high_cancel_ratio_bps: 8_000,
            high_imbalance_bps: 7_000,
            high_price_move_bps: 25,
            high_quote_events: 1_000,
        }
    }
}

/// Pattern-risk snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRiskSnapshot {
    spoofing_layering_risk_bps: u16,
    quote_stuffing_risk_bps: u16,
    stop_run_risk_bps: u16,
    absorption_risk_bps: u16,
    momentum_ignition_risk_bps: u16,
    overall_risk_bps: u16,
}

impl PatternRiskSnapshot {
    /// Returns spoofing/layering risk indicator in basis points.
    pub const fn spoofing_layering_risk_bps(&self) -> u16 {
        self.spoofing_layering_risk_bps
    }

    /// Returns quote-stuffing risk indicator in basis points.
    pub const fn quote_stuffing_risk_bps(&self) -> u16 {
        self.quote_stuffing_risk_bps
    }

    /// Returns stop-run/liquidity-sweep risk indicator in basis points.
    pub const fn stop_run_risk_bps(&self) -> u16 {
        self.stop_run_risk_bps
    }

    /// Returns absorption risk indicator in basis points.
    pub const fn absorption_risk_bps(&self) -> u16 {
        self.absorption_risk_bps
    }

    /// Returns momentum-ignition risk indicator in basis points.
    pub const fn momentum_ignition_risk_bps(&self) -> u16 {
        self.momentum_ignition_risk_bps
    }

    /// Returns maximum component risk in basis points.
    pub const fn overall_risk_bps(&self) -> u16 {
        self.overall_risk_bps
    }
}

/// Deterministic pattern-risk classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRiskClassifier {
    config: PatternRiskConfig,
}

impl PatternRiskClassifier {
    /// Creates a pattern-risk classifier.
    pub const fn new(config: PatternRiskConfig) -> Self {
        Self { config }
    }

    /// Classifies pattern risk.
    pub fn classify(&self, input: PatternRiskInput) -> PatternRiskSnapshot {
        let quote_events = input.quote_adds().saturating_add(input.quote_cancels());
        let order_to_trade_bps = rate_scaled(quote_events, input.trades().max(1));
        let cancel_ratio_bps = rate_bps(input.quote_cancels(), quote_events.max(1));
        let imbalance = input.depth_imbalance_bps().unsigned_abs();
        let move_abs = input.price_move_bps().unsigned_abs();
        let spoofing_layering = average_score(&[
            score_ratio(order_to_trade_bps, self.config.high_order_to_trade_bps),
            score_ratio(
                u32::from(cancel_ratio_bps),
                u32::from(self.config.high_cancel_ratio_bps),
            ),
            score_ratio(imbalance, u32::from(self.config.high_imbalance_bps)),
        ]);
        let quote_stuffing = score_ratio_u64(quote_events, self.config.high_quote_events);
        let stop_run = average_score(&[
            score_ratio(move_abs, self.config.high_price_move_bps),
            score_ratio_i64(input.executed_qty(), input.displayed_depth().max(1)),
        ]);
        let absorption = average_score(&[
            score_ratio_i64(input.executed_qty(), input.displayed_depth().max(1)),
            10_000_u16.saturating_sub(score_ratio(move_abs, self.config.high_price_move_bps)),
        ]);
        let momentum_ignition = average_score(&[
            score_ratio(move_abs, self.config.high_price_move_bps),
            score_ratio(order_to_trade_bps, self.config.high_order_to_trade_bps),
        ]);
        let overall = spoofing_layering
            .max(quote_stuffing)
            .max(stop_run)
            .max(absorption)
            .max(momentum_ignition);
        PatternRiskSnapshot {
            spoofing_layering_risk_bps: spoofing_layering,
            quote_stuffing_risk_bps: quote_stuffing,
            stop_run_risk_bps: stop_run,
            absorption_risk_bps: absorption,
            momentum_ignition_risk_bps: momentum_ignition,
            overall_risk_bps: overall,
        }
    }
}

impl Default for PatternRiskClassifier {
    fn default() -> Self {
        Self::new(PatternRiskConfig::default())
    }
}

/// Detailed pattern-risk configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternDetailConfig {
    iceberg_fill_multiple_bps: u32,
    stacked_imbalance_threshold_bps: u16,
    absorption_move_threshold_bps: u32,
    failed_breakout_threshold_bps: u32,
}

impl PatternDetailConfig {
    /// Creates detailed pattern-risk configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidPattern`] when basis-point thresholds
    /// exceed 10,000 where applicable.
    pub const fn new(
        iceberg_fill_multiple_bps: u32,
        stacked_imbalance_threshold_bps: u16,
        absorption_move_threshold_bps: u32,
        failed_breakout_threshold_bps: u32,
    ) -> Result<Self, AnalyticsError> {
        if stacked_imbalance_threshold_bps > 10_000 {
            return Err(AnalyticsError::InvalidPattern);
        }
        Ok(Self {
            iceberg_fill_multiple_bps,
            stacked_imbalance_threshold_bps,
            absorption_move_threshold_bps,
            failed_breakout_threshold_bps,
        })
    }

    /// Returns iceberg fill/displayed-depth threshold in basis points.
    pub const fn iceberg_fill_multiple_bps(&self) -> u32 {
        self.iceberg_fill_multiple_bps
    }

    /// Returns stacked-imbalance threshold in basis points.
    pub const fn stacked_imbalance_threshold_bps(&self) -> u16 {
        self.stacked_imbalance_threshold_bps
    }

    /// Returns absorption price-move threshold in basis points.
    pub const fn absorption_move_threshold_bps(&self) -> u32 {
        self.absorption_move_threshold_bps
    }

    /// Returns failed-breakout reversal threshold in basis points.
    pub const fn failed_breakout_threshold_bps(&self) -> u32 {
        self.failed_breakout_threshold_bps
    }
}

impl Default for PatternDetailConfig {
    fn default() -> Self {
        Self {
            iceberg_fill_multiple_bps: 20_000,
            stacked_imbalance_threshold_bps: 7_000,
            absorption_move_threshold_bps: 10,
            failed_breakout_threshold_bps: 20,
        }
    }
}

/// Detailed pattern-risk input over a bounded observation window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternDetailInput {
    repeated_fills_at_level: u64,
    replenishments_at_level: u64,
    executed_at_level_qty: i64,
    displayed_at_level_qty: i64,
    stacked_imbalance_levels: u8,
    stacked_imbalance_bps: u16,
    absorbed_qty: i64,
    absorption_price_move_bps: u32,
    breakout_move_bps: u32,
    reversal_move_bps: u32,
    signed_accumulation_qty: i64,
}

impl PatternDetailInput {
    /// Creates detailed pattern-risk input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidPattern`] when quantities are negative
    /// where unsigned semantics are required or basis-point values exceed
    /// 10,000 where applicable.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        repeated_fills_at_level: u64,
        replenishments_at_level: u64,
        executed_at_level_qty: i64,
        displayed_at_level_qty: i64,
        stacked_imbalance_levels: u8,
        stacked_imbalance_bps: u16,
        absorbed_qty: i64,
        absorption_price_move_bps: u32,
        breakout_move_bps: u32,
        reversal_move_bps: u32,
        signed_accumulation_qty: i64,
    ) -> Result<Self, AnalyticsError> {
        if executed_at_level_qty < 0
            || displayed_at_level_qty < 0
            || absorbed_qty < 0
            || stacked_imbalance_bps > 10_000
        {
            return Err(AnalyticsError::InvalidPattern);
        }
        Ok(Self {
            repeated_fills_at_level,
            replenishments_at_level,
            executed_at_level_qty,
            displayed_at_level_qty,
            stacked_imbalance_levels,
            stacked_imbalance_bps,
            absorbed_qty,
            absorption_price_move_bps,
            breakout_move_bps,
            reversal_move_bps,
            signed_accumulation_qty,
        })
    }

    /// Returns repeated fills at the same price level.
    pub const fn repeated_fills_at_level(&self) -> u64 {
        self.repeated_fills_at_level
    }

    /// Returns replenishments at the same price level.
    pub const fn replenishments_at_level(&self) -> u64 {
        self.replenishments_at_level
    }

    /// Returns executed quantity at the price level.
    pub const fn executed_at_level_qty(&self) -> i64 {
        self.executed_at_level_qty
    }

    /// Returns displayed quantity at the price level.
    pub const fn displayed_at_level_qty(&self) -> i64 {
        self.displayed_at_level_qty
    }

    /// Returns stacked imbalance level count.
    pub const fn stacked_imbalance_levels(&self) -> u8 {
        self.stacked_imbalance_levels
    }

    /// Returns stacked imbalance strength in basis points.
    pub const fn stacked_imbalance_bps(&self) -> u16 {
        self.stacked_imbalance_bps
    }

    /// Returns absorbed quantity.
    pub const fn absorbed_qty(&self) -> i64 {
        self.absorbed_qty
    }

    /// Returns price move during absorption in basis points.
    pub const fn absorption_price_move_bps(&self) -> u32 {
        self.absorption_price_move_bps
    }

    /// Returns breakout move in basis points.
    pub const fn breakout_move_bps(&self) -> u32 {
        self.breakout_move_bps
    }

    /// Returns reversal move after breakout in basis points.
    pub const fn reversal_move_bps(&self) -> u32 {
        self.reversal_move_bps
    }

    /// Returns signed accumulation quantity.
    pub const fn signed_accumulation_qty(&self) -> i64 {
        self.signed_accumulation_qty
    }
}

/// Detailed pattern-risk snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternDetailSnapshot {
    iceberg_risk_bps: u16,
    hidden_accumulation_bps: u16,
    hidden_distribution_bps: u16,
    stacked_imbalance_risk_bps: u16,
    absorption_strength_bps: u16,
    failed_breakout_risk_bps: u16,
    overall_risk_bps: u16,
}

impl PatternDetailSnapshot {
    /// Returns iceberg/hidden-refresh risk in basis points.
    pub const fn iceberg_risk_bps(&self) -> u16 {
        self.iceberg_risk_bps
    }

    /// Returns hidden accumulation risk in basis points.
    pub const fn hidden_accumulation_bps(&self) -> u16 {
        self.hidden_accumulation_bps
    }

    /// Returns hidden distribution risk in basis points.
    pub const fn hidden_distribution_bps(&self) -> u16 {
        self.hidden_distribution_bps
    }

    /// Returns stacked imbalance risk in basis points.
    pub const fn stacked_imbalance_risk_bps(&self) -> u16 {
        self.stacked_imbalance_risk_bps
    }

    /// Returns absorption strength in basis points.
    pub const fn absorption_strength_bps(&self) -> u16 {
        self.absorption_strength_bps
    }

    /// Returns failed breakout risk in basis points.
    pub const fn failed_breakout_risk_bps(&self) -> u16 {
        self.failed_breakout_risk_bps
    }

    /// Returns maximum detailed pattern risk in basis points.
    pub const fn overall_risk_bps(&self) -> u16 {
        self.overall_risk_bps
    }
}

/// Deterministic detailed pattern-risk analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternDetailAnalyzer {
    config: PatternDetailConfig,
}

impl PatternDetailAnalyzer {
    /// Creates a detailed pattern-risk analyzer.
    pub const fn new(config: PatternDetailConfig) -> Self {
        Self { config }
    }

    /// Returns analyzer configuration.
    pub const fn config(&self) -> PatternDetailConfig {
        self.config
    }

    /// Evaluates detailed pattern risk.
    pub fn evaluate(&self, input: PatternDetailInput) -> PatternDetailSnapshot {
        let fill_multiple_bps = ratio_i64_to_u32(
            input.executed_at_level_qty(),
            input.displayed_at_level_qty().max(1),
        );
        let iceberg_risk = average_score(&[
            score_ratio(fill_multiple_bps, self.config.iceberg_fill_multiple_bps()),
            score_ratio_u64(input.repeated_fills_at_level(), 5),
            score_ratio_u64(input.replenishments_at_level(), 3),
        ]);
        let hidden_accumulation = if input.signed_accumulation_qty() > 0 {
            iceberg_risk
        } else {
            0
        };
        let hidden_distribution = if input.signed_accumulation_qty() < 0 {
            iceberg_risk
        } else {
            0
        };
        let stacked_imbalance = average_score(&[
            score_ratio_u64(u64::from(input.stacked_imbalance_levels()), 3),
            score_ratio(
                u32::from(input.stacked_imbalance_bps()),
                u32::from(self.config.stacked_imbalance_threshold_bps()),
            ),
        ]);
        let absorption = average_score(&[
            score_ratio_i64(input.absorbed_qty(), input.displayed_at_level_qty().max(1)),
            10_000_u16.saturating_sub(score_ratio(
                input.absorption_price_move_bps(),
                self.config.absorption_move_threshold_bps(),
            )),
        ]);
        let failed_breakout = average_score(&[
            score_ratio(
                input.breakout_move_bps(),
                self.config.failed_breakout_threshold_bps(),
            ),
            score_ratio(
                input.reversal_move_bps(),
                self.config.failed_breakout_threshold_bps(),
            ),
        ]);
        let overall = iceberg_risk
            .max(hidden_accumulation)
            .max(hidden_distribution)
            .max(stacked_imbalance)
            .max(absorption)
            .max(failed_breakout);
        PatternDetailSnapshot {
            iceberg_risk_bps: iceberg_risk,
            hidden_accumulation_bps: hidden_accumulation,
            hidden_distribution_bps: hidden_distribution,
            stacked_imbalance_risk_bps: stacked_imbalance,
            absorption_strength_bps: absorption,
            failed_breakout_risk_bps: failed_breakout,
            overall_risk_bps: overall,
        }
    }
}

impl Default for PatternDetailAnalyzer {
    fn default() -> Self {
        Self::new(PatternDetailConfig::default())
    }
}
