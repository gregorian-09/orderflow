use super::*;

/// Cross-asset paired price sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetSample {
    leader_price: i64,
    follower_price: i64,
    ts_ns: u64,
}

impl CrossAssetSample {
    /// Creates a cross-asset paired price sample.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidCrossAsset`] when prices or timestamp
    /// are invalid.
    pub const fn new(
        leader_price: i64,
        follower_price: i64,
        ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if leader_price <= 0 || follower_price <= 0 || ts_ns == 0 {
            return Err(AnalyticsError::InvalidCrossAsset);
        }
        Ok(Self {
            leader_price,
            follower_price,
            ts_ns,
        })
    }

    /// Returns leader price.
    pub const fn leader_price(&self) -> i64 {
        self.leader_price
    }

    /// Returns follower price.
    pub const fn follower_price(&self) -> i64 {
        self.follower_price
    }

    /// Returns sample timestamp.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }

    /// Returns leader-minus-follower spread.
    pub const fn spread(&self) -> i64 {
        self.leader_price - self.follower_price
    }
}

/// Cross-asset analytics configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetConfig {
    divergence_threshold_bps: u32,
    basis_threshold_bps: u32,
    correlation_breakdown_bps: u16,
}

impl CrossAssetConfig {
    /// Creates cross-asset thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidCrossAsset`] when the correlation
    /// threshold exceeds 10,000.
    pub const fn new(
        divergence_threshold_bps: u32,
        basis_threshold_bps: u32,
        correlation_breakdown_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if correlation_breakdown_bps > 10_000 {
            return Err(AnalyticsError::InvalidCrossAsset);
        }
        Ok(Self {
            divergence_threshold_bps,
            basis_threshold_bps,
            correlation_breakdown_bps,
        })
    }

    /// Returns pair-divergence threshold.
    pub const fn divergence_threshold_bps(&self) -> u32 {
        self.divergence_threshold_bps
    }

    /// Returns futures/spot basis pressure threshold.
    pub const fn basis_threshold_bps(&self) -> u32 {
        self.basis_threshold_bps
    }

    /// Returns absolute-correlation threshold below which breakdown is flagged.
    pub const fn correlation_breakdown_bps(&self) -> u16 {
        self.correlation_breakdown_bps
    }
}

impl Default for CrossAssetConfig {
    fn default() -> Self {
        Self {
            divergence_threshold_bps: 50,
            basis_threshold_bps: 25,
            correlation_breakdown_bps: 2_000,
        }
    }
}

/// Cross-asset analytics snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetSnapshot {
    samples: usize,
    last_ts_ns: u64,
    correlation_bps: i32,
    beta_bps: i32,
    pair_divergence_bps: i32,
    basis_pressure_bps: i32,
    pair_divergence: bool,
    basis_pressure: bool,
    correlation_breakdown: bool,
    lead_lag_score_bps: u16,
}

impl CrossAssetSnapshot {
    /// Returns retained paired return count.
    pub const fn samples(&self) -> usize {
        self.samples
    }

    /// Returns latest sample timestamp.
    pub const fn last_ts_ns(&self) -> u64 {
        self.last_ts_ns
    }

    /// Returns rolling Pearson-style correlation scaled by 10,000.
    pub const fn correlation_bps(&self) -> i32 {
        self.correlation_bps
    }

    /// Returns follower-versus-leader beta scaled by 10,000.
    pub const fn beta_bps(&self) -> i32 {
        self.beta_bps
    }

    /// Returns latest pair divergence in basis points.
    pub const fn pair_divergence_bps(&self) -> i32 {
        self.pair_divergence_bps
    }

    /// Returns latest basis pressure in basis points.
    pub const fn basis_pressure_bps(&self) -> i32 {
        self.basis_pressure_bps
    }

    /// Returns true when pair divergence exceeds configured threshold.
    pub const fn pair_divergence(&self) -> bool {
        self.pair_divergence
    }

    /// Returns true when basis pressure exceeds configured threshold.
    pub const fn basis_pressure(&self) -> bool {
        self.basis_pressure
    }

    /// Returns true when absolute correlation is below configured threshold.
    pub const fn correlation_breakdown(&self) -> bool {
        self.correlation_breakdown
    }

    /// Returns lead/lag strength score in basis points.
    pub const fn lead_lag_score_bps(&self) -> u16 {
        self.lead_lag_score_bps
    }
}

/// Fixed-window cross-asset lead/lag tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetTracker<const N: usize = 64> {
    config: CrossAssetConfig,
    leader_returns_bps: [i32; N],
    follower_returns_bps: [i32; N],
    next: usize,
    len: usize,
    last_sample: Option<CrossAssetSample>,
    last_pair_divergence_bps: i32,
    last_basis_pressure_bps: i32,
}

impl<const N: usize> CrossAssetTracker<N> {
    /// Creates a cross-asset tracker.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidCrossAsset`] when capacity is zero.
    pub const fn new(config: CrossAssetConfig) -> Result<Self, AnalyticsError> {
        if N == 0 {
            return Err(AnalyticsError::InvalidCrossAsset);
        }
        Ok(Self {
            config,
            leader_returns_bps: [0; N],
            follower_returns_bps: [0; N],
            next: 0,
            len: 0,
            last_sample: None,
            last_pair_divergence_bps: 0,
            last_basis_pressure_bps: 0,
        })
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> CrossAssetConfig {
        self.config
    }

    /// Records one paired sample.
    pub fn on_sample(&mut self, sample: CrossAssetSample) -> CrossAssetSnapshot {
        if let Some(previous) = self.last_sample {
            let leader_ret = price_to_bps(
                sample
                    .leader_price()
                    .saturating_sub(previous.leader_price()),
                previous.leader_price(),
            );
            let follower_ret = price_to_bps(
                sample
                    .follower_price()
                    .saturating_sub(previous.follower_price()),
                previous.follower_price(),
            );
            self.leader_returns_bps[self.next] = leader_ret;
            self.follower_returns_bps[self.next] = follower_ret;
            self.next = (self.next + 1) % N;
            self.len = self.len.saturating_add(1).min(N);
            self.last_pair_divergence_bps = follower_ret.saturating_sub(leader_ret);
        }
        self.last_basis_pressure_bps = price_to_bps(sample.spread(), sample.follower_price());
        self.last_sample = Some(sample);
        self.snapshot()
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> CrossAssetSnapshot {
        let correlation = self.correlation_bps();
        let beta = self.beta_bps();
        let abs_corr = correlation.unsigned_abs();
        CrossAssetSnapshot {
            samples: self.len,
            last_ts_ns: self.last_sample.map(|sample| sample.ts_ns()).unwrap_or(0),
            correlation_bps: correlation,
            beta_bps: beta,
            pair_divergence_bps: self.last_pair_divergence_bps,
            basis_pressure_bps: self.last_basis_pressure_bps,
            pair_divergence: self.last_pair_divergence_bps.unsigned_abs()
                >= self.config.divergence_threshold_bps(),
            basis_pressure: self.last_basis_pressure_bps.unsigned_abs()
                >= self.config.basis_threshold_bps(),
            correlation_breakdown: self.len > 1
                && abs_corr < u32::from(self.config.correlation_breakdown_bps()),
            lead_lag_score_bps: score_ratio(abs_corr, 10_000),
        }
    }

    /// Clears accumulated state.
    pub fn reset(&mut self) {
        self.next = 0;
        self.len = 0;
        self.last_sample = None;
        self.last_pair_divergence_bps = 0;
        self.last_basis_pressure_bps = 0;
        self.leader_returns_bps = [0; N];
        self.follower_returns_bps = [0; N];
    }

    fn correlation_bps(&self) -> i32 {
        if self.len <= 1 {
            return 0;
        }
        let mut sum_xy = 0_i128;
        let mut sum_x2 = 0_u128;
        let mut sum_y2 = 0_u128;
        for offset in 0..self.len {
            let idx = if self.len == N {
                (self.next + offset) % N
            } else {
                offset
            };
            let x = i128::from(self.leader_returns_bps[idx]);
            let y = i128::from(self.follower_returns_bps[idx]);
            sum_xy = sum_xy.saturating_add(x.saturating_mul(y));
            sum_x2 = sum_x2.saturating_add(x.unsigned_abs().saturating_mul(x.unsigned_abs()));
            sum_y2 = sum_y2.saturating_add(y.unsigned_abs().saturating_mul(y.unsigned_abs()));
        }
        if sum_x2 == 0 || sum_y2 == 0 {
            return 0;
        }
        let denominator = isqrt_u128(sum_x2.saturating_mul(sum_y2));
        if denominator == 0 {
            return 0;
        }
        let scaled = (sum_xy.saturating_mul(10_000)) / denominator as i128;
        i32::try_from(scaled.clamp(-10_000, 10_000)).unwrap_or(0)
    }

    fn beta_bps(&self) -> i32 {
        if self.len == 0 {
            return 0;
        }
        let mut sum_xy = 0_i128;
        let mut sum_x2 = 0_i128;
        for offset in 0..self.len {
            let idx = if self.len == N {
                (self.next + offset) % N
            } else {
                offset
            };
            let x = i128::from(self.leader_returns_bps[idx]);
            let y = i128::from(self.follower_returns_bps[idx]);
            sum_xy = sum_xy.saturating_add(x.saturating_mul(y));
            sum_x2 = sum_x2.saturating_add(x.saturating_mul(x));
        }
        if sum_x2 == 0 {
            return 0;
        }
        i32::try_from(
            ((sum_xy * 10_000) / sum_x2).clamp(i128::from(i32::MIN), i128::from(i32::MAX)),
        )
        .unwrap_or(0)
    }
}

/// Cross-asset diagnostic thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetDiagnosticConfig {
    min_latency_adjusted_correlation_bps: u16,
    aggregate_divergence_threshold_bps: u16,
    cross_venue_divergence_threshold_bps: u16,
    component_imbalance_threshold_bps: u16,
    max_sample_skew_ns: u64,
    min_sync_quality_bps: u16,
}

impl CrossAssetDiagnosticConfig {
    /// Creates cross-asset diagnostic thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidCrossAsset`] when basis-point
    /// thresholds exceed 10,000 or maximum sample skew is zero.
    pub const fn new(
        min_latency_adjusted_correlation_bps: u16,
        aggregate_divergence_threshold_bps: u16,
        cross_venue_divergence_threshold_bps: u16,
        component_imbalance_threshold_bps: u16,
        max_sample_skew_ns: u64,
        min_sync_quality_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if min_latency_adjusted_correlation_bps > 10_000
            || aggregate_divergence_threshold_bps > 10_000
            || cross_venue_divergence_threshold_bps > 10_000
            || component_imbalance_threshold_bps > 10_000
            || max_sample_skew_ns == 0
            || min_sync_quality_bps > 10_000
        {
            return Err(AnalyticsError::InvalidCrossAsset);
        }
        Ok(Self {
            min_latency_adjusted_correlation_bps,
            aggregate_divergence_threshold_bps,
            cross_venue_divergence_threshold_bps,
            component_imbalance_threshold_bps,
            max_sample_skew_ns,
            min_sync_quality_bps,
        })
    }

    /// Returns minimum latency-adjusted absolute correlation in basis points.
    pub const fn min_latency_adjusted_correlation_bps(&self) -> u16 {
        self.min_latency_adjusted_correlation_bps
    }

    /// Returns aggregate divergence threshold in basis points.
    pub const fn aggregate_divergence_threshold_bps(&self) -> u16 {
        self.aggregate_divergence_threshold_bps
    }

    /// Returns cross-venue divergence threshold in basis points.
    pub const fn cross_venue_divergence_threshold_bps(&self) -> u16 {
        self.cross_venue_divergence_threshold_bps
    }

    /// Returns ETF/component imbalance threshold in basis points.
    pub const fn component_imbalance_threshold_bps(&self) -> u16 {
        self.component_imbalance_threshold_bps
    }

    /// Returns maximum acceptable sample timestamp skew in nanoseconds.
    pub const fn max_sample_skew_ns(&self) -> u64 {
        self.max_sample_skew_ns
    }

    /// Returns minimum synchronization quality in basis points.
    pub const fn min_sync_quality_bps(&self) -> u16 {
        self.min_sync_quality_bps
    }
}

impl Default for CrossAssetDiagnosticConfig {
    fn default() -> Self {
        Self {
            min_latency_adjusted_correlation_bps: 2_000,
            aggregate_divergence_threshold_bps: 250,
            cross_venue_divergence_threshold_bps: 25,
            component_imbalance_threshold_bps: 1_000,
            max_sample_skew_ns: 1_000_000,
            min_sync_quality_bps: 5_000,
        }
    }
}

/// Cross-asset diagnostic input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetDiagnosticInput {
    snapshot: CrossAssetSnapshot,
    leader_event_ts_ns: u64,
    follower_event_ts_ns: u64,
    cross_venue_divergence_bps: i32,
    component_imbalance_bps: i32,
}

impl CrossAssetDiagnosticInput {
    /// Creates cross-asset diagnostic input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidCrossAsset`] when either event
    /// timestamp is zero.
    pub const fn new(
        snapshot: CrossAssetSnapshot,
        leader_event_ts_ns: u64,
        follower_event_ts_ns: u64,
        cross_venue_divergence_bps: i32,
        component_imbalance_bps: i32,
    ) -> Result<Self, AnalyticsError> {
        if leader_event_ts_ns == 0 || follower_event_ts_ns == 0 {
            return Err(AnalyticsError::InvalidCrossAsset);
        }
        Ok(Self {
            snapshot,
            leader_event_ts_ns,
            follower_event_ts_ns,
            cross_venue_divergence_bps,
            component_imbalance_bps,
        })
    }

    /// Returns the base cross-asset tracker snapshot.
    pub const fn snapshot(&self) -> CrossAssetSnapshot {
        self.snapshot
    }

    /// Returns leader event timestamp in nanoseconds.
    pub const fn leader_event_ts_ns(&self) -> u64 {
        self.leader_event_ts_ns
    }

    /// Returns follower event timestamp in nanoseconds.
    pub const fn follower_event_ts_ns(&self) -> u64 {
        self.follower_event_ts_ns
    }

    /// Returns signed cross-venue divergence in basis points.
    pub const fn cross_venue_divergence_bps(&self) -> i32 {
        self.cross_venue_divergence_bps
    }

    /// Returns signed ETF/component imbalance in basis points.
    pub const fn component_imbalance_bps(&self) -> i32 {
        self.component_imbalance_bps
    }
}

/// Cross-asset diagnostic snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetDiagnosticSnapshot {
    sample_skew_ns: u64,
    synchronization_quality_bps: u16,
    latency_adjusted_correlation_bps: i32,
    cross_venue_divergence_bps: i32,
    component_imbalance_bps: i32,
    aggregate_divergence_score_bps: u16,
    cross_venue_divergence: bool,
    component_imbalance: bool,
    relationship_degraded: bool,
}

impl CrossAssetDiagnosticSnapshot {
    /// Returns absolute sample timestamp skew in nanoseconds.
    pub const fn sample_skew_ns(&self) -> u64 {
        self.sample_skew_ns
    }

    /// Returns synchronization quality in basis points.
    pub const fn synchronization_quality_bps(&self) -> u16 {
        self.synchronization_quality_bps
    }

    /// Returns signed latency-adjusted correlation in basis points.
    pub const fn latency_adjusted_correlation_bps(&self) -> i32 {
        self.latency_adjusted_correlation_bps
    }

    /// Returns signed cross-venue divergence in basis points.
    pub const fn cross_venue_divergence_bps(&self) -> i32 {
        self.cross_venue_divergence_bps
    }

    /// Returns signed ETF/component imbalance in basis points.
    pub const fn component_imbalance_bps(&self) -> i32 {
        self.component_imbalance_bps
    }

    /// Returns aggregate divergence score in basis points.
    pub const fn aggregate_divergence_score_bps(&self) -> u16 {
        self.aggregate_divergence_score_bps
    }

    /// Returns true when cross-venue divergence exceeds threshold.
    pub const fn cross_venue_divergence(&self) -> bool {
        self.cross_venue_divergence
    }

    /// Returns true when ETF/component imbalance exceeds threshold.
    pub const fn component_imbalance(&self) -> bool {
        self.component_imbalance
    }

    /// Returns true when the relationship is degraded.
    pub const fn relationship_degraded(&self) -> bool {
        self.relationship_degraded
    }
}

/// Deterministic cross-asset diagnostic analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetDiagnosticAnalyzer {
    config: CrossAssetDiagnosticConfig,
}

impl CrossAssetDiagnosticAnalyzer {
    /// Creates a cross-asset diagnostic analyzer.
    pub const fn new(config: CrossAssetDiagnosticConfig) -> Self {
        Self { config }
    }

    /// Returns analyzer configuration.
    pub const fn config(&self) -> CrossAssetDiagnosticConfig {
        self.config
    }

    /// Evaluates cross-asset synchronization, divergence, and degradation.
    pub fn evaluate(&self, input: CrossAssetDiagnosticInput) -> CrossAssetDiagnosticSnapshot {
        let sample_skew = abs_diff_u64(input.leader_event_ts_ns(), input.follower_event_ts_ns());
        let sync_quality = latency_quality_bps(sample_skew, self.config.max_sample_skew_ns());
        let latency_adjusted_correlation =
            signed_scaled_bps(input.snapshot().correlation_bps(), sync_quality);
        let cross_venue_score = bounded_abs_bps(input.cross_venue_divergence_bps());
        let component_score = bounded_abs_bps(input.component_imbalance_bps());
        let aggregate_divergence = average_score(&[
            bounded_abs_bps(input.snapshot().pair_divergence_bps()),
            bounded_abs_bps(input.snapshot().basis_pressure_bps()),
            cross_venue_score,
            component_score,
        ]);
        let cross_venue_divergence =
            cross_venue_score >= self.config.cross_venue_divergence_threshold_bps();
        let component_imbalance =
            component_score >= self.config.component_imbalance_threshold_bps();
        let relationship_degraded = input.snapshot().correlation_breakdown()
            || latency_adjusted_correlation.unsigned_abs()
                < u32::from(self.config.min_latency_adjusted_correlation_bps())
            || aggregate_divergence >= self.config.aggregate_divergence_threshold_bps()
            || sync_quality < self.config.min_sync_quality_bps();
        CrossAssetDiagnosticSnapshot {
            sample_skew_ns: sample_skew,
            synchronization_quality_bps: sync_quality,
            latency_adjusted_correlation_bps: latency_adjusted_correlation,
            cross_venue_divergence_bps: input.cross_venue_divergence_bps(),
            component_imbalance_bps: input.component_imbalance_bps(),
            aggregate_divergence_score_bps: aggregate_divergence,
            cross_venue_divergence,
            component_imbalance,
            relationship_degraded,
        }
    }
}

impl Default for CrossAssetDiagnosticAnalyzer {
    fn default() -> Self {
        Self::new(CrossAssetDiagnosticConfig::default())
    }
}
