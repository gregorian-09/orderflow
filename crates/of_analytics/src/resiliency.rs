use super::*;

/// Liquidity resiliency sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResiliencySample {
    ts_ns: u64,
    spread_bps: u32,
    bid_depth: i64,
    ask_depth: i64,
}

impl ResiliencySample {
    /// Creates a resiliency sample.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidResiliency`] when timestamp is zero or
    /// depth is negative.
    pub const fn new(
        ts_ns: u64,
        spread_bps: u32,
        bid_depth: i64,
        ask_depth: i64,
    ) -> Result<Self, AnalyticsError> {
        if ts_ns == 0 || bid_depth < 0 || ask_depth < 0 {
            return Err(AnalyticsError::InvalidResiliency);
        }
        Ok(Self {
            ts_ns,
            spread_bps,
            bid_depth,
            ask_depth,
        })
    }

    /// Returns sample timestamp.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }

    /// Returns spread in basis points.
    pub const fn spread_bps(&self) -> u32 {
        self.spread_bps
    }

    /// Returns bid-side depth.
    pub const fn bid_depth(&self) -> i64 {
        self.bid_depth
    }

    /// Returns ask-side depth.
    pub const fn ask_depth(&self) -> i64 {
        self.ask_depth
    }

    /// Returns total displayed depth.
    pub const fn total_depth(&self) -> i64 {
        self.bid_depth + self.ask_depth
    }
}

/// Liquidity resiliency thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResiliencyConfig {
    baseline_spread_bps: u32,
    baseline_depth: i64,
    shock_spread_bps: u32,
    depth_floor_bps: u16,
    recovery_spread_bps: u32,
    recovery_depth_bps: u16,
}

impl ResiliencyConfig {
    /// Creates resiliency thresholds.
    ///
    /// `depth_floor_bps` and `recovery_depth_bps` are percentages of baseline
    /// depth in basis points.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidResiliency`] when depth is non-positive
    /// or percentage thresholds are above 10,000.
    pub const fn new(
        baseline_spread_bps: u32,
        baseline_depth: i64,
        shock_spread_bps: u32,
        depth_floor_bps: u16,
        recovery_spread_bps: u32,
        recovery_depth_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if baseline_depth <= 0 || depth_floor_bps > 10_000 || recovery_depth_bps > 10_000 {
            return Err(AnalyticsError::InvalidResiliency);
        }
        Ok(Self {
            baseline_spread_bps,
            baseline_depth,
            shock_spread_bps,
            depth_floor_bps,
            recovery_spread_bps,
            recovery_depth_bps,
        })
    }

    /// Returns baseline spread in basis points.
    pub const fn baseline_spread_bps(&self) -> u32 {
        self.baseline_spread_bps
    }

    /// Returns baseline total depth.
    pub const fn baseline_depth(&self) -> i64 {
        self.baseline_depth
    }

    /// Returns spread threshold that starts a shock.
    pub const fn shock_spread_bps(&self) -> u32 {
        self.shock_spread_bps
    }

    /// Returns depth floor threshold as basis points of baseline depth.
    pub const fn depth_floor_bps(&self) -> u16 {
        self.depth_floor_bps
    }

    /// Returns spread threshold that allows recovery.
    pub const fn recovery_spread_bps(&self) -> u32 {
        self.recovery_spread_bps
    }

    /// Returns recovery depth threshold as basis points of baseline depth.
    pub const fn recovery_depth_bps(&self) -> u16 {
        self.recovery_depth_bps
    }

    const fn depth_floor_qty(&self) -> i64 {
        ((self.baseline_depth as i128 * self.depth_floor_bps as i128) / 10_000) as i64
    }

    const fn recovery_depth_qty(&self) -> i64 {
        ((self.baseline_depth as i128 * self.recovery_depth_bps as i128) / 10_000) as i64
    }
}

/// Liquidity resiliency snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResiliencySnapshot {
    samples: u64,
    shock_count: u64,
    recovery_count: u64,
    active_shock: bool,
    last_shock_ts_ns: u64,
    last_recovery_ts_ns: u64,
    last_recovery_time_ns: u64,
    max_spread_bps: u32,
    min_depth: i64,
    score_bps: u16,
}

impl ResiliencySnapshot {
    /// Returns sample count.
    pub const fn samples(&self) -> u64 {
        self.samples
    }

    /// Returns shock count.
    pub const fn shock_count(&self) -> u64 {
        self.shock_count
    }

    /// Returns recovery count.
    pub const fn recovery_count(&self) -> u64 {
        self.recovery_count
    }

    /// Returns true when a shock is active.
    pub const fn active_shock(&self) -> bool {
        self.active_shock
    }

    /// Returns latest shock timestamp.
    pub const fn last_shock_ts_ns(&self) -> u64 {
        self.last_shock_ts_ns
    }

    /// Returns latest recovery timestamp.
    pub const fn last_recovery_ts_ns(&self) -> u64 {
        self.last_recovery_ts_ns
    }

    /// Returns latest recovery duration in nanoseconds.
    pub const fn last_recovery_time_ns(&self) -> u64 {
        self.last_recovery_time_ns
    }

    /// Returns maximum spread observed during the active or latest shock.
    pub const fn max_spread_bps(&self) -> u32 {
        self.max_spread_bps
    }

    /// Returns minimum total depth observed during the active or latest shock.
    pub const fn min_depth(&self) -> i64 {
        self.min_depth
    }

    /// Returns resiliency score in basis points, where 10,000 is best.
    pub const fn score_bps(&self) -> u16 {
        self.score_bps
    }
}

/// Threshold-based liquidity resiliency tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResiliencyTracker {
    config: ResiliencyConfig,
    samples: u64,
    shock_count: u64,
    recovery_count: u64,
    active_shock: bool,
    current_shock_ts_ns: u64,
    last_shock_ts_ns: u64,
    last_recovery_ts_ns: u64,
    last_recovery_time_ns: u64,
    max_spread_bps: u32,
    min_depth: i64,
}

impl ResiliencyTracker {
    /// Creates a resiliency tracker.
    pub const fn new(config: ResiliencyConfig) -> Self {
        Self {
            config,
            samples: 0,
            shock_count: 0,
            recovery_count: 0,
            active_shock: false,
            current_shock_ts_ns: 0,
            last_shock_ts_ns: 0,
            last_recovery_ts_ns: 0,
            last_recovery_time_ns: 0,
            max_spread_bps: 0,
            min_depth: i64::MAX,
        }
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> ResiliencyConfig {
        self.config
    }

    /// Records one spread/depth sample.
    pub fn on_sample(&mut self, sample: ResiliencySample) -> ResiliencySnapshot {
        self.samples = self.samples.saturating_add(1);
        let total_depth = sample.total_depth();
        if self.active_shock {
            self.max_spread_bps = self.max_spread_bps.max(sample.spread_bps());
            self.min_depth = self.min_depth.min(total_depth);
            if self.is_recovered(sample) {
                self.active_shock = false;
                self.recovery_count = self.recovery_count.saturating_add(1);
                self.last_recovery_ts_ns = sample.ts_ns();
                self.last_recovery_time_ns =
                    sample.ts_ns().saturating_sub(self.current_shock_ts_ns);
            }
        } else if self.is_shock(sample) {
            self.active_shock = true;
            self.shock_count = self.shock_count.saturating_add(1);
            self.current_shock_ts_ns = sample.ts_ns();
            self.last_shock_ts_ns = sample.ts_ns();
            self.max_spread_bps = sample.spread_bps();
            self.min_depth = total_depth;
        }
        self.snapshot()
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> ResiliencySnapshot {
        ResiliencySnapshot {
            samples: self.samples,
            shock_count: self.shock_count,
            recovery_count: self.recovery_count,
            active_shock: self.active_shock,
            last_shock_ts_ns: self.last_shock_ts_ns,
            last_recovery_ts_ns: self.last_recovery_ts_ns,
            last_recovery_time_ns: self.last_recovery_time_ns,
            max_spread_bps: self.max_spread_bps,
            min_depth: if self.min_depth == i64::MAX {
                0
            } else {
                self.min_depth
            },
            score_bps: self.score_bps(),
        }
    }

    /// Clears accumulated state.
    pub fn reset(&mut self) {
        self.samples = 0;
        self.shock_count = 0;
        self.recovery_count = 0;
        self.active_shock = false;
        self.current_shock_ts_ns = 0;
        self.last_shock_ts_ns = 0;
        self.last_recovery_ts_ns = 0;
        self.last_recovery_time_ns = 0;
        self.max_spread_bps = 0;
        self.min_depth = i64::MAX;
    }

    fn is_shock(&self, sample: ResiliencySample) -> bool {
        sample.spread_bps() >= self.config.shock_spread_bps()
            || sample.total_depth() <= self.config.depth_floor_qty()
    }

    fn is_recovered(&self, sample: ResiliencySample) -> bool {
        sample.spread_bps() <= self.config.recovery_spread_bps()
            && sample.total_depth() >= self.config.recovery_depth_qty()
    }

    fn score_bps(&self) -> u16 {
        if self.shock_count == 0 {
            return 10_000;
        }
        if self.active_shock {
            return 5_000;
        }
        let recovery_rate = rate_bps(self.recovery_count, self.shock_count);
        let latency_penalty = (self.last_recovery_time_ns / 1_000_000).min(5_000);
        u16::try_from(u64::from(recovery_rate).saturating_sub(latency_penalty)).unwrap_or(0)
    }
}
