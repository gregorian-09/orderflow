use super::*;

/// VPIN-style toxicity snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VpinSnapshot {
    bucket_count: usize,
    current_bucket_volume: i64,
    vpin_bps: u16,
    toxicity_bps: u16,
}

impl VpinSnapshot {
    /// Returns completed bucket count.
    pub const fn bucket_count(&self) -> usize {
        self.bucket_count
    }

    /// Returns current open bucket volume.
    pub const fn current_bucket_volume(&self) -> i64 {
        self.current_bucket_volume
    }

    /// Returns VPIN in basis points.
    pub const fn vpin_bps(&self) -> u16 {
        self.vpin_bps
    }

    /// Returns current toxicity basis points including only completed buckets.
    pub const fn toxicity_bps(&self) -> u16 {
        self.toxicity_bps
    }
}

/// Fixed-capacity VPIN-style bucket tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VpinTracker<const N: usize = 32> {
    bucket_volume: i64,
    bucket_imbalances: [i64; N],
    next_bucket: usize,
    bucket_count: usize,
    current_buy_volume: i64,
    current_sell_volume: i64,
}

impl<const N: usize> VpinTracker<N> {
    /// Creates a VPIN tracker.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when bucket volume is not
    /// positive or capacity is zero.
    pub const fn new(bucket_volume: i64) -> Result<Self, AnalyticsError> {
        if bucket_volume <= 0 || N == 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            bucket_volume,
            bucket_imbalances: [0; N],
            next_bucket: 0,
            bucket_count: 0,
            current_buy_volume: 0,
            current_sell_volume: 0,
        })
    }

    /// Records one trade.
    pub fn on_trade(&mut self, trade: TradeContext) {
        match trade.aggressor_side() {
            Side::Ask => {
                self.current_buy_volume = self.current_buy_volume.saturating_add(trade.qty())
            }
            Side::Bid => {
                self.current_sell_volume = self.current_sell_volume.saturating_add(trade.qty())
            }
        }
        if self.current_bucket_volume() >= self.bucket_volume {
            self.close_bucket();
        }
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> VpinSnapshot {
        let count = self.bucket_count.min(N);
        let mut imbalance_sum = 0_i64;
        for imbalance in self.bucket_imbalances.iter().take(count) {
            imbalance_sum = imbalance_sum.saturating_add(*imbalance);
        }
        let denominator = self
            .bucket_volume
            .saturating_mul(i64::try_from(count).unwrap_or(0));
        let vpin_bps = if denominator <= 0 {
            0
        } else {
            u16::try_from(
                ((i128::from(imbalance_sum) * 10_000) / i128::from(denominator)).clamp(0, 10_000),
            )
            .unwrap_or(10_000)
        };
        VpinSnapshot {
            bucket_count: count,
            current_bucket_volume: self.current_bucket_volume(),
            vpin_bps,
            toxicity_bps: vpin_bps,
        }
    }

    /// Returns configured bucket volume.
    pub const fn bucket_volume(&self) -> i64 {
        self.bucket_volume
    }

    /// Returns current open bucket volume.
    pub const fn current_bucket_volume(&self) -> i64 {
        self.current_buy_volume + self.current_sell_volume
    }

    fn close_bucket(&mut self) {
        self.bucket_imbalances[self.next_bucket] =
            self.current_buy_volume.abs_diff(self.current_sell_volume) as i64;
        self.next_bucket = (self.next_bucket + 1) % N;
        self.bucket_count = self.bucket_count.saturating_add(1).min(N);
        self.current_buy_volume = 0;
        self.current_sell_volume = 0;
    }
}

/// Toxicity/adverse-selection thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToxicityConfig {
    markout_threshold_bps: u16,
    quote_fade_threshold_bps: u16,
    vpin_threshold_bps: u16,
    intensity_threshold_bps: u16,
}

impl ToxicityConfig {
    /// Creates toxicity thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when any threshold is zero or
    /// exceeds 10,000 basis points.
    pub const fn new(
        markout_threshold_bps: u16,
        quote_fade_threshold_bps: u16,
        vpin_threshold_bps: u16,
        intensity_threshold_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if markout_threshold_bps == 0
            || quote_fade_threshold_bps == 0
            || vpin_threshold_bps == 0
            || intensity_threshold_bps == 0
            || markout_threshold_bps > 10_000
            || quote_fade_threshold_bps > 10_000
            || vpin_threshold_bps > 10_000
            || intensity_threshold_bps > 10_000
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            markout_threshold_bps,
            quote_fade_threshold_bps,
            vpin_threshold_bps,
            intensity_threshold_bps,
        })
    }

    /// Returns adverse markout threshold in basis points.
    pub const fn markout_threshold_bps(&self) -> u16 {
        self.markout_threshold_bps
    }

    /// Returns quote-fade threshold in basis points.
    pub const fn quote_fade_threshold_bps(&self) -> u16 {
        self.quote_fade_threshold_bps
    }

    /// Returns VPIN threshold in basis points.
    pub const fn vpin_threshold_bps(&self) -> u16 {
        self.vpin_threshold_bps
    }

    /// Returns trade-intensity threshold in basis points.
    pub const fn intensity_threshold_bps(&self) -> u16 {
        self.intensity_threshold_bps
    }
}

impl Default for ToxicityConfig {
    fn default() -> Self {
        Self {
            markout_threshold_bps: 10,
            quote_fade_threshold_bps: 2_500,
            vpin_threshold_bps: 7_000,
            intensity_threshold_bps: 7_000,
        }
    }
}

/// Toxicity/adverse-selection observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToxicityInput {
    trade: TradeContext,
    future_midpoint: i64,
    pre_bid_qty: i64,
    pre_ask_qty: i64,
    post_bid_qty: i64,
    post_ask_qty: i64,
    vpin_bps: u16,
    trade_intensity_bps: u16,
}

impl ToxicityInput {
    /// Creates toxicity input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when future midpoint is
    /// non-positive, quantities are negative, or basis-point values exceed
    /// 10,000.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        trade: TradeContext,
        future_midpoint: i64,
        pre_bid_qty: i64,
        pre_ask_qty: i64,
        post_bid_qty: i64,
        post_ask_qty: i64,
        vpin_bps: u16,
        trade_intensity_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if future_midpoint <= 0
            || pre_bid_qty < 0
            || pre_ask_qty < 0
            || post_bid_qty < 0
            || post_ask_qty < 0
            || vpin_bps > 10_000
            || trade_intensity_bps > 10_000
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            trade,
            future_midpoint,
            pre_bid_qty,
            pre_ask_qty,
            post_bid_qty,
            post_ask_qty,
            vpin_bps,
            trade_intensity_bps,
        })
    }

    /// Returns trade context.
    pub const fn trade(&self) -> TradeContext {
        self.trade
    }

    /// Returns future midpoint.
    pub const fn future_midpoint(&self) -> i64 {
        self.future_midpoint
    }

    /// Returns pre-trade bid quantity.
    pub const fn pre_bid_qty(&self) -> i64 {
        self.pre_bid_qty
    }

    /// Returns pre-trade ask quantity.
    pub const fn pre_ask_qty(&self) -> i64 {
        self.pre_ask_qty
    }

    /// Returns post-trade bid quantity.
    pub const fn post_bid_qty(&self) -> i64 {
        self.post_bid_qty
    }

    /// Returns post-trade ask quantity.
    pub const fn post_ask_qty(&self) -> i64 {
        self.post_ask_qty
    }

    /// Returns VPIN or equivalent flow-imbalance score in basis points.
    pub const fn vpin_bps(&self) -> u16 {
        self.vpin_bps
    }

    /// Returns trade-intensity score in basis points.
    pub const fn trade_intensity_bps(&self) -> u16 {
        self.trade_intensity_bps
    }
}

/// Toxicity/adverse-selection risk snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToxicitySnapshot {
    post_trade_markout_bps: i32,
    adverse_selection_score_bps: u16,
    quote_fade_bps: i32,
    informed_flow_proxy_bps: u16,
    toxic_flow_burst: bool,
    toxicity_score_bps: u16,
}

impl ToxicitySnapshot {
    /// Returns side-aware post-trade markout in basis points.
    pub const fn post_trade_markout_bps(&self) -> i32 {
        self.post_trade_markout_bps
    }

    /// Returns adverse-selection score in basis points.
    pub const fn adverse_selection_score_bps(&self) -> u16 {
        self.adverse_selection_score_bps
    }

    /// Returns same-side quote fade in basis points.
    pub const fn quote_fade_bps(&self) -> i32 {
        self.quote_fade_bps
    }

    /// Returns informed-flow proxy score in basis points.
    pub const fn informed_flow_proxy_bps(&self) -> u16 {
        self.informed_flow_proxy_bps
    }

    /// Returns whether the observation crosses toxic-burst thresholds.
    pub const fn toxic_flow_burst(&self) -> bool {
        self.toxic_flow_burst
    }

    /// Returns aggregate toxicity score in basis points.
    pub const fn toxicity_score_bps(&self) -> u16 {
        self.toxicity_score_bps
    }
}

/// Deterministic toxicity/adverse-selection analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToxicityAnalyzer {
    config: ToxicityConfig,
}

impl ToxicityAnalyzer {
    /// Creates a toxicity analyzer.
    pub const fn new(config: ToxicityConfig) -> Self {
        Self { config }
    }

    /// Returns analyzer configuration.
    pub const fn config(&self) -> ToxicityConfig {
        self.config
    }

    /// Evaluates one toxicity/adverse-selection observation.
    pub fn evaluate(&self, input: ToxicityInput) -> ToxicitySnapshot {
        let post_trade_markout_bps = side_aware_price_move_bps(
            input.trade().aggressor_side(),
            input.trade().price(),
            input.future_midpoint(),
        );
        let quote_fade_bps = quote_fade_bps(input);
        let adverse_selection_score_bps = score_ratio(
            positive_bps(post_trade_markout_bps),
            u32::from(self.config.markout_threshold_bps()),
        );
        let quote_fade_score_bps = score_ratio(
            positive_bps(quote_fade_bps),
            u32::from(self.config.quote_fade_threshold_bps()),
        );
        let vpin_score_bps = score_ratio(
            u32::from(input.vpin_bps()),
            u32::from(self.config.vpin_threshold_bps()),
        );
        let intensity_score_bps = score_ratio(
            u32::from(input.trade_intensity_bps()),
            u32::from(self.config.intensity_threshold_bps()),
        );
        let informed_flow_proxy_bps = average_bps4(
            adverse_selection_score_bps,
            quote_fade_score_bps,
            vpin_score_bps,
            intensity_score_bps,
        );
        let toxic_flow_burst = positive_bps(post_trade_markout_bps)
            >= u32::from(self.config.markout_threshold_bps())
            && (input.vpin_bps() >= self.config.vpin_threshold_bps()
                || positive_bps(quote_fade_bps)
                    >= u32::from(self.config.quote_fade_threshold_bps())
                || input.trade_intensity_bps() >= self.config.intensity_threshold_bps());
        let toxicity_score_bps =
            informed_flow_proxy_bps.max(adverse_selection_score_bps.min(10_000));
        ToxicitySnapshot {
            post_trade_markout_bps,
            adverse_selection_score_bps,
            quote_fade_bps,
            informed_flow_proxy_bps,
            toxic_flow_burst,
            toxicity_score_bps,
        }
    }
}

impl Default for ToxicityAnalyzer {
    fn default() -> Self {
        Self::new(ToxicityConfig::default())
    }
}
