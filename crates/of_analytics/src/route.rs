use super::*;

/// Venue route event kind.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VenueRouteEventKind {
    /// Child order was sent.
    Sent = 1,
    /// Child order received a fill.
    Fill = 2,
    /// Child order was rejected.
    Reject = 3,
    /// Child order was canceled.
    Cancel = 4,
}

/// Venue route analytics event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueRouteEvent {
    kind: VenueRouteEventKind,
    qty: i64,
    quote_to_fill_latency_ns: u64,
    market_data_to_order_latency_ns: u64,
}

impl VenueRouteEvent {
    /// Creates a venue route event.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidRoute`] when quantity is negative.
    pub const fn new(
        kind: VenueRouteEventKind,
        qty: i64,
        quote_to_fill_latency_ns: u64,
        market_data_to_order_latency_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if qty < 0 {
            return Err(AnalyticsError::InvalidRoute);
        }
        Ok(Self {
            kind,
            qty,
            quote_to_fill_latency_ns,
            market_data_to_order_latency_ns,
        })
    }

    /// Returns event kind.
    pub const fn kind(&self) -> VenueRouteEventKind {
        self.kind
    }

    /// Returns event quantity.
    pub const fn qty(&self) -> i64 {
        self.qty
    }

    /// Returns quote-to-fill latency in nanoseconds.
    pub const fn quote_to_fill_latency_ns(&self) -> u64 {
        self.quote_to_fill_latency_ns
    }

    /// Returns market-data-to-order latency in nanoseconds.
    pub const fn market_data_to_order_latency_ns(&self) -> u64 {
        self.market_data_to_order_latency_ns
    }
}

/// Venue route analytics snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueRouteSnapshot {
    sent: u64,
    fills: u64,
    rejects: u64,
    cancels: u64,
    sent_qty: i64,
    filled_qty: i64,
    fill_rate_bps: u16,
    reject_rate_bps: u16,
    cancel_rate_bps: u16,
    avg_quote_to_fill_latency_ns: u64,
    max_quote_to_fill_latency_ns: u64,
    avg_market_data_to_order_latency_ns: u64,
    max_market_data_to_order_latency_ns: u64,
    route_health_bps: u16,
}

impl VenueRouteSnapshot {
    /// Returns sent order count.
    pub const fn sent(&self) -> u64 {
        self.sent
    }

    /// Returns fill count.
    pub const fn fills(&self) -> u64 {
        self.fills
    }

    /// Returns reject count.
    pub const fn rejects(&self) -> u64 {
        self.rejects
    }

    /// Returns cancel count.
    pub const fn cancels(&self) -> u64 {
        self.cancels
    }

    /// Returns sent quantity.
    pub const fn sent_qty(&self) -> i64 {
        self.sent_qty
    }

    /// Returns filled quantity.
    pub const fn filled_qty(&self) -> i64 {
        self.filled_qty
    }

    /// Returns fill rate in basis points.
    pub const fn fill_rate_bps(&self) -> u16 {
        self.fill_rate_bps
    }

    /// Returns reject rate in basis points.
    pub const fn reject_rate_bps(&self) -> u16 {
        self.reject_rate_bps
    }

    /// Returns cancel rate in basis points.
    pub const fn cancel_rate_bps(&self) -> u16 {
        self.cancel_rate_bps
    }

    /// Returns average quote-to-fill latency.
    pub const fn avg_quote_to_fill_latency_ns(&self) -> u64 {
        self.avg_quote_to_fill_latency_ns
    }

    /// Returns maximum quote-to-fill latency.
    pub const fn max_quote_to_fill_latency_ns(&self) -> u64 {
        self.max_quote_to_fill_latency_ns
    }

    /// Returns average market-data-to-order latency.
    pub const fn avg_market_data_to_order_latency_ns(&self) -> u64 {
        self.avg_market_data_to_order_latency_ns
    }

    /// Returns maximum market-data-to-order latency.
    pub const fn max_market_data_to_order_latency_ns(&self) -> u64 {
        self.max_market_data_to_order_latency_ns
    }

    /// Returns route health score in basis points.
    pub const fn route_health_bps(&self) -> u16 {
        self.route_health_bps
    }
}

/// Venue route analytics tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueRouteTracker {
    sent: u64,
    fills: u64,
    rejects: u64,
    cancels: u64,
    sent_qty: i64,
    filled_qty: i64,
    quote_to_fill_latency_sum: u128,
    quote_to_fill_latency_samples: u64,
    max_quote_to_fill_latency_ns: u64,
    md_to_order_latency_sum: u128,
    md_to_order_latency_samples: u64,
    max_md_to_order_latency_ns: u64,
}

impl VenueRouteTracker {
    /// Creates an empty venue route tracker.
    pub const fn new() -> Self {
        Self {
            sent: 0,
            fills: 0,
            rejects: 0,
            cancels: 0,
            sent_qty: 0,
            filled_qty: 0,
            quote_to_fill_latency_sum: 0,
            quote_to_fill_latency_samples: 0,
            max_quote_to_fill_latency_ns: 0,
            md_to_order_latency_sum: 0,
            md_to_order_latency_samples: 0,
            max_md_to_order_latency_ns: 0,
        }
    }

    /// Records a venue route event.
    pub fn on_event(&mut self, event: VenueRouteEvent) {
        match event.kind() {
            VenueRouteEventKind::Sent => {
                self.sent = self.sent.saturating_add(1);
                self.sent_qty = self.sent_qty.saturating_add(event.qty());
            }
            VenueRouteEventKind::Fill => {
                self.fills = self.fills.saturating_add(1);
                self.filled_qty = self.filled_qty.saturating_add(event.qty());
                self.record_quote_to_fill(event.quote_to_fill_latency_ns());
            }
            VenueRouteEventKind::Reject => self.rejects = self.rejects.saturating_add(1),
            VenueRouteEventKind::Cancel => self.cancels = self.cancels.saturating_add(1),
        }
        self.record_md_to_order(event.market_data_to_order_latency_ns());
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> VenueRouteSnapshot {
        let terminal = self
            .fills
            .saturating_add(self.rejects)
            .saturating_add(self.cancels);
        let reject_rate = rate_bps(self.rejects, terminal.max(1));
        let cancel_rate = rate_bps(self.cancels, terminal.max(1));
        let fill_rate = rate_bps(self.fills, terminal.max(1));
        let route_health = fill_rate
            .saturating_sub(reject_rate / 2)
            .saturating_sub(cancel_rate / 4);
        VenueRouteSnapshot {
            sent: self.sent,
            fills: self.fills,
            rejects: self.rejects,
            cancels: self.cancels,
            sent_qty: self.sent_qty,
            filled_qty: self.filled_qty,
            fill_rate_bps: fill_rate,
            reject_rate_bps: reject_rate,
            cancel_rate_bps: cancel_rate,
            avg_quote_to_fill_latency_ns: avg_u128(
                self.quote_to_fill_latency_sum,
                self.quote_to_fill_latency_samples,
            ),
            max_quote_to_fill_latency_ns: self.max_quote_to_fill_latency_ns,
            avg_market_data_to_order_latency_ns: avg_u128(
                self.md_to_order_latency_sum,
                self.md_to_order_latency_samples,
            ),
            max_market_data_to_order_latency_ns: self.max_md_to_order_latency_ns,
            route_health_bps: route_health,
        }
    }

    /// Clears accumulated route state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    fn record_quote_to_fill(&mut self, latency_ns: u64) {
        if latency_ns == 0 {
            return;
        }
        self.quote_to_fill_latency_sum = self
            .quote_to_fill_latency_sum
            .saturating_add(u128::from(latency_ns));
        self.quote_to_fill_latency_samples = self.quote_to_fill_latency_samples.saturating_add(1);
        self.max_quote_to_fill_latency_ns = self.max_quote_to_fill_latency_ns.max(latency_ns);
    }

    fn record_md_to_order(&mut self, latency_ns: u64) {
        if latency_ns == 0 {
            return;
        }
        self.md_to_order_latency_sum = self
            .md_to_order_latency_sum
            .saturating_add(u128::from(latency_ns));
        self.md_to_order_latency_samples = self.md_to_order_latency_samples.saturating_add(1);
        self.max_md_to_order_latency_ns = self.max_md_to_order_latency_ns.max(latency_ns);
    }
}

impl Default for VenueRouteTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Venue route quality thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueRouteQualityConfig {
    target_fill_rate_bps: u16,
    max_reject_rate_bps: u16,
    max_cancel_rate_bps: u16,
    max_quote_to_fill_latency_ns: u64,
    max_market_data_to_order_latency_ns: u64,
    degradation_threshold_bps: u16,
    drift_threshold_bps: u16,
}

impl VenueRouteQualityConfig {
    /// Creates venue route quality thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidRoute`] when basis-point thresholds
    /// exceed 10,000, target fill rate is zero, or latency limits are zero.
    pub const fn new(
        target_fill_rate_bps: u16,
        max_reject_rate_bps: u16,
        max_cancel_rate_bps: u16,
        max_quote_to_fill_latency_ns: u64,
        max_market_data_to_order_latency_ns: u64,
        degradation_threshold_bps: u16,
        drift_threshold_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if target_fill_rate_bps > 10_000
            || max_reject_rate_bps > 10_000
            || max_cancel_rate_bps > 10_000
            || degradation_threshold_bps > 10_000
            || drift_threshold_bps > 10_000
            || target_fill_rate_bps == 0
            || max_quote_to_fill_latency_ns == 0
            || max_market_data_to_order_latency_ns == 0
        {
            return Err(AnalyticsError::InvalidRoute);
        }
        Ok(Self {
            target_fill_rate_bps,
            max_reject_rate_bps,
            max_cancel_rate_bps,
            max_quote_to_fill_latency_ns,
            max_market_data_to_order_latency_ns,
            degradation_threshold_bps,
            drift_threshold_bps,
        })
    }

    /// Returns target fill rate in basis points.
    pub const fn target_fill_rate_bps(&self) -> u16 {
        self.target_fill_rate_bps
    }

    /// Returns maximum acceptable reject rate in basis points.
    pub const fn max_reject_rate_bps(&self) -> u16 {
        self.max_reject_rate_bps
    }

    /// Returns maximum acceptable cancel rate in basis points.
    pub const fn max_cancel_rate_bps(&self) -> u16 {
        self.max_cancel_rate_bps
    }

    /// Returns maximum acceptable quote-to-fill latency in nanoseconds.
    pub const fn max_quote_to_fill_latency_ns(&self) -> u64 {
        self.max_quote_to_fill_latency_ns
    }

    /// Returns maximum acceptable market-data-to-order latency in nanoseconds.
    pub const fn max_market_data_to_order_latency_ns(&self) -> u64 {
        self.max_market_data_to_order_latency_ns
    }

    /// Returns degradation threshold in basis points.
    pub const fn degradation_threshold_bps(&self) -> u16 {
        self.degradation_threshold_bps
    }

    /// Returns route drift threshold in basis points.
    pub const fn drift_threshold_bps(&self) -> u16 {
        self.drift_threshold_bps
    }
}

impl Default for VenueRouteQualityConfig {
    fn default() -> Self {
        Self {
            target_fill_rate_bps: 7_500,
            max_reject_rate_bps: 1_000,
            max_cancel_rate_bps: 5_000,
            max_quote_to_fill_latency_ns: 1_000_000,
            max_market_data_to_order_latency_ns: 1_000_000,
            degradation_threshold_bps: 5_000,
            drift_threshold_bps: 2_000,
        }
    }
}

/// Venue route quality input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueRouteQualityInput {
    snapshot: VenueRouteSnapshot,
    venue_liquidity_bps: u16,
    venue_toxicity_bps: u16,
    venue_fill_quality_bps: u16,
    baseline_route_health_bps: u16,
}

impl VenueRouteQualityInput {
    /// Creates venue route quality input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidRoute`] when any basis-point value
    /// exceeds 10,000.
    pub const fn new(
        snapshot: VenueRouteSnapshot,
        venue_liquidity_bps: u16,
        venue_toxicity_bps: u16,
        venue_fill_quality_bps: u16,
        baseline_route_health_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if venue_liquidity_bps > 10_000
            || venue_toxicity_bps > 10_000
            || venue_fill_quality_bps > 10_000
            || baseline_route_health_bps > 10_000
        {
            return Err(AnalyticsError::InvalidRoute);
        }
        Ok(Self {
            snapshot,
            venue_liquidity_bps,
            venue_toxicity_bps,
            venue_fill_quality_bps,
            baseline_route_health_bps,
        })
    }

    /// Returns route lifecycle snapshot.
    pub const fn snapshot(&self) -> VenueRouteSnapshot {
        self.snapshot
    }

    /// Returns venue liquidity score in basis points.
    pub const fn venue_liquidity_bps(&self) -> u16 {
        self.venue_liquidity_bps
    }

    /// Returns venue toxicity risk in basis points.
    pub const fn venue_toxicity_bps(&self) -> u16 {
        self.venue_toxicity_bps
    }

    /// Returns venue fill-quality score in basis points.
    pub const fn venue_fill_quality_bps(&self) -> u16 {
        self.venue_fill_quality_bps
    }

    /// Returns baseline route health in basis points.
    pub const fn baseline_route_health_bps(&self) -> u16 {
        self.baseline_route_health_bps
    }
}

/// Venue route quality snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueRouteQualitySnapshot {
    venue_liquidity_score_bps: u16,
    venue_toxicity_score_bps: u16,
    venue_fill_quality_bps: u16,
    latency_score_bps: u16,
    reliability_score_bps: u16,
    route_quality_score_bps: u16,
    route_drift_bps: u16,
    route_degraded: bool,
}

impl VenueRouteQualitySnapshot {
    /// Returns venue liquidity score in basis points.
    pub const fn venue_liquidity_score_bps(&self) -> u16 {
        self.venue_liquidity_score_bps
    }

    /// Returns venue toxicity risk score in basis points.
    pub const fn venue_toxicity_score_bps(&self) -> u16 {
        self.venue_toxicity_score_bps
    }

    /// Returns venue fill-quality score in basis points.
    pub const fn venue_fill_quality_bps(&self) -> u16 {
        self.venue_fill_quality_bps
    }

    /// Returns combined latency score in basis points.
    pub const fn latency_score_bps(&self) -> u16 {
        self.latency_score_bps
    }

    /// Returns route reliability score in basis points.
    pub const fn reliability_score_bps(&self) -> u16 {
        self.reliability_score_bps
    }

    /// Returns aggregate route quality score in basis points.
    pub const fn route_quality_score_bps(&self) -> u16 {
        self.route_quality_score_bps
    }

    /// Returns degradation drift from baseline route health in basis points.
    pub const fn route_drift_bps(&self) -> u16 {
        self.route_drift_bps
    }

    /// Returns whether the route is degraded.
    pub const fn route_degraded(&self) -> bool {
        self.route_degraded
    }
}

/// Deterministic venue route quality analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueRouteQualityAnalyzer {
    config: VenueRouteQualityConfig,
}

impl VenueRouteQualityAnalyzer {
    /// Creates a venue route quality analyzer.
    pub const fn new(config: VenueRouteQualityConfig) -> Self {
        Self { config }
    }

    /// Returns analyzer configuration.
    pub const fn config(&self) -> VenueRouteQualityConfig {
        self.config
    }

    /// Evaluates venue route quality and degradation.
    pub fn evaluate(&self, input: VenueRouteQualityInput) -> VenueRouteQualitySnapshot {
        let snapshot = input.snapshot();
        let latency_score = average_score(&[
            latency_quality_bps(
                snapshot.avg_quote_to_fill_latency_ns(),
                self.config.max_quote_to_fill_latency_ns(),
            ),
            latency_quality_bps(
                snapshot.avg_market_data_to_order_latency_ns(),
                self.config.max_market_data_to_order_latency_ns(),
            ),
        ]);
        let reliability = route_reliability_bps(snapshot, self.config);
        let route_quality = average_score(&[
            input.venue_liquidity_bps(),
            10_000_u16.saturating_sub(input.venue_toxicity_bps()),
            input.venue_fill_quality_bps(),
            latency_score,
            reliability,
            snapshot.route_health_bps(),
        ]);
        let route_drift = input
            .baseline_route_health_bps()
            .saturating_sub(snapshot.route_health_bps());
        VenueRouteQualitySnapshot {
            venue_liquidity_score_bps: input.venue_liquidity_bps(),
            venue_toxicity_score_bps: input.venue_toxicity_bps(),
            venue_fill_quality_bps: input.venue_fill_quality_bps(),
            latency_score_bps: latency_score,
            reliability_score_bps: reliability,
            route_quality_score_bps: route_quality,
            route_drift_bps: route_drift,
            route_degraded: route_quality < self.config.degradation_threshold_bps()
                || route_drift >= self.config.drift_threshold_bps(),
        }
    }
}

impl Default for VenueRouteQualityAnalyzer {
    fn default() -> Self {
        Self::new(VenueRouteQualityConfig::default())
    }
}
