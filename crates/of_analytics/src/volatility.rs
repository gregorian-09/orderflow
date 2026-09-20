use super::*;

/// Rolling volatility/noise snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilitySnapshot {
    samples: usize,
    last_price: i64,
    realized_vol_bps: u32,
    mean_abs_return_bps: u32,
    bipower_vol_bps: u32,
    jump_variation_bps: u32,
    noise_ratio_bps: u16,
}

impl VolatilitySnapshot {
    /// Returns retained return sample count.
    pub const fn samples(&self) -> usize {
        self.samples
    }

    /// Returns last observed price.
    pub const fn last_price(&self) -> i64 {
        self.last_price
    }

    /// Returns realized volatility in basis points.
    pub const fn realized_vol_bps(&self) -> u32 {
        self.realized_vol_bps
    }

    /// Returns mean absolute return in basis points.
    pub const fn mean_abs_return_bps(&self) -> u32 {
        self.mean_abs_return_bps
    }

    /// Returns bipower variation volatility proxy in basis points.
    pub const fn bipower_vol_bps(&self) -> u32 {
        self.bipower_vol_bps
    }

    /// Returns jump variation proxy in basis points.
    pub const fn jump_variation_bps(&self) -> u32 {
        self.jump_variation_bps
    }

    /// Returns microstructure noise proxy in basis points.
    pub const fn noise_ratio_bps(&self) -> u16 {
        self.noise_ratio_bps
    }
}

/// Fixed-window volatility tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilityTracker<const N: usize = 64> {
    returns_bps: [i32; N],
    next: usize,
    len: usize,
    last_price: i64,
}

impl<const N: usize> VolatilityTracker<N> {
    /// Creates an empty volatility tracker.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when capacity is zero.
    pub const fn new() -> Result<Self, AnalyticsError> {
        if N == 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            returns_bps: [0; N],
            next: 0,
            len: 0,
            last_price: 0,
        })
    }

    /// Records a price observation.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when price is not positive.
    pub fn on_price(&mut self, price: i64) -> Result<(), AnalyticsError> {
        if price <= 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        if self.last_price > 0 {
            let ret = i32::try_from(
                ((i128::from(price) - i128::from(self.last_price)) * 10_000)
                    / i128::from(self.last_price),
            )
            .unwrap_or(0);
            self.returns_bps[self.next] = ret;
            self.next = (self.next + 1) % N;
            self.len = self.len.saturating_add(1).min(N);
        }
        self.last_price = price;
        Ok(())
    }

    /// Returns current volatility snapshot.
    pub fn snapshot(&self) -> VolatilitySnapshot {
        if self.len == 0 {
            return VolatilitySnapshot {
                samples: 0,
                last_price: self.last_price,
                realized_vol_bps: 0,
                mean_abs_return_bps: 0,
                bipower_vol_bps: 0,
                jump_variation_bps: 0,
                noise_ratio_bps: 0,
            };
        }
        let mut sum_sq = 0_u128;
        let mut sum_abs = 0_u128;
        let mut bipower_sum = 0_u128;
        let mut sign_flips = 0_u32;
        let mut prev_sign = 0_i32;
        let mut prev_abs = 0_u32;
        for offset in 0..self.len {
            let idx = if self.len == N {
                (self.next + offset) % N
            } else {
                offset
            };
            let ret = self.returns_bps[idx];
            let abs = ret.unsigned_abs();
            sum_abs = sum_abs.saturating_add(u128::from(abs));
            sum_sq = sum_sq.saturating_add(u128::from(abs).saturating_mul(u128::from(abs)));
            if prev_abs > 0 {
                bipower_sum = bipower_sum
                    .saturating_add(u128::from(prev_abs).saturating_mul(u128::from(abs)));
            }
            prev_abs = abs;
            let sign = ret.signum();
            if prev_sign != 0 && sign != 0 && sign != prev_sign {
                sign_flips = sign_flips.saturating_add(1);
            }
            if sign != 0 {
                prev_sign = sign;
            }
        }
        let len = u128::try_from(self.len).unwrap_or(1);
        let realized = isqrt_u128(sum_sq / len);
        let mean_abs = sum_abs / len;
        let bipower = if self.len <= 1 {
            0
        } else {
            let mean_bipower = bipower_sum / u128::try_from(self.len - 1).unwrap_or(1);
            isqrt_u128((mean_bipower.saturating_mul(15_708)) / 10_000)
        };
        let jump_variation_bps = realized.saturating_sub(bipower);
        let noise_ratio_bps = if self.len <= 1 {
            0
        } else {
            u16::try_from(
                (u128::from(sign_flips) * 10_000) / u128::try_from(self.len - 1).unwrap_or(1),
            )
            .unwrap_or(10_000)
        };
        VolatilitySnapshot {
            samples: self.len,
            last_price: self.last_price,
            realized_vol_bps: u32::try_from(realized).unwrap_or(u32::MAX),
            mean_abs_return_bps: u32::try_from(mean_abs).unwrap_or(u32::MAX),
            bipower_vol_bps: u32::try_from(bipower).unwrap_or(u32::MAX),
            jump_variation_bps: u32::try_from(jump_variation_bps).unwrap_or(u32::MAX),
            noise_ratio_bps,
        }
    }
}

/// OHLC volatility estimator input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OhlcVolatilityInput {
    open: i64,
    high: i64,
    low: i64,
    close: i64,
    previous_close: Option<i64>,
}

impl OhlcVolatilityInput {
    /// Creates OHLC volatility input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when prices are non-positive,
    /// inconsistent, or previous close is non-positive when supplied.
    pub const fn new(
        open: i64,
        high: i64,
        low: i64,
        close: i64,
        previous_close: Option<i64>,
    ) -> Result<Self, AnalyticsError> {
        if open <= 0
            || high <= 0
            || low <= 0
            || close <= 0
            || low > high
            || open > high
            || open < low
            || close > high
            || close < low
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        if let Some(previous_close) = previous_close {
            if previous_close <= 0 {
                return Err(AnalyticsError::InvalidTrade);
            }
        }
        Ok(Self {
            open,
            high,
            low,
            close,
            previous_close,
        })
    }

    /// Returns open price.
    pub const fn open(&self) -> i64 {
        self.open
    }

    /// Returns high price.
    pub const fn high(&self) -> i64 {
        self.high
    }

    /// Returns low price.
    pub const fn low(&self) -> i64 {
        self.low
    }

    /// Returns close price.
    pub const fn close(&self) -> i64 {
        self.close
    }

    /// Returns previous close when available.
    pub const fn previous_close(&self) -> Option<i64> {
        self.previous_close
    }
}

/// OHLC volatility estimator snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OhlcVolatilitySnapshot {
    close_to_close_vol_bps: u32,
    parkinson_vol_bps: u32,
    garman_klass_vol_bps: u32,
    rogers_satchell_vol_bps: u32,
    jump_gap_bps: i32,
}

impl OhlcVolatilitySnapshot {
    /// Returns close-to-close volatility proxy in basis points.
    pub const fn close_to_close_vol_bps(&self) -> u32 {
        self.close_to_close_vol_bps
    }

    /// Returns Parkinson range volatility proxy in basis points.
    pub const fn parkinson_vol_bps(&self) -> u32 {
        self.parkinson_vol_bps
    }

    /// Returns Garman-Klass volatility proxy in basis points.
    pub const fn garman_klass_vol_bps(&self) -> u32 {
        self.garman_klass_vol_bps
    }

    /// Returns Rogers-Satchell volatility proxy in basis points.
    pub const fn rogers_satchell_vol_bps(&self) -> u32 {
        self.rogers_satchell_vol_bps
    }

    /// Returns signed open gap versus previous close in basis points.
    pub const fn jump_gap_bps(&self) -> i32 {
        self.jump_gap_bps
    }
}

/// Deterministic OHLC volatility estimator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OhlcVolatilityEstimator;

impl OhlcVolatilityEstimator {
    /// Estimates OHLC volatility from one candle.
    pub fn estimate(input: OhlcVolatilityInput) -> OhlcVolatilitySnapshot {
        let high_low_bps = abs_return_bps(input.high(), input.low());
        let open_close_bps = abs_return_bps(input.close(), input.open());
        let close_to_close_vol_bps = input
            .previous_close()
            .map(|previous| abs_return_bps(input.close(), previous))
            .unwrap_or(open_close_bps);
        let parkinson_vol_bps =
            u32::try_from((u128::from(high_low_bps) * 6_006) / 10_000).unwrap_or(u32::MAX);
        let gk_var = ((u128::from(high_low_bps).saturating_mul(u128::from(high_low_bps)) * 5_000)
            / 10_000)
            .saturating_sub(
                (u128::from(open_close_bps).saturating_mul(u128::from(open_close_bps)) * 3_863)
                    / 10_000,
            );
        let high_open = i128::from(return_bps(input.high(), input.open()));
        let high_close = i128::from(return_bps(input.high(), input.close()));
        let low_open = i128::from(return_bps(input.low(), input.open()));
        let low_close = i128::from(return_bps(input.low(), input.close()));
        let rs_var = high_open
            .saturating_mul(high_close)
            .saturating_add(low_open.saturating_mul(low_close))
            .max(0) as u128;
        let jump_gap_bps = input
            .previous_close()
            .map(|previous| return_bps(input.open(), previous))
            .unwrap_or(0);
        OhlcVolatilitySnapshot {
            close_to_close_vol_bps,
            parkinson_vol_bps,
            garman_klass_vol_bps: u32::try_from(isqrt_u128(gk_var)).unwrap_or(u32::MAX),
            rogers_satchell_vol_bps: u32::try_from(isqrt_u128(rs_var)).unwrap_or(u32::MAX),
            jump_gap_bps,
        }
    }
}

/// Volatility signature point over borrowed returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilitySignatureSnapshot {
    sampling_interval_ns: u64,
    samples: usize,
    realized_vol_bps: u32,
    noise_ratio_bps: u16,
}

impl VolatilitySignatureSnapshot {
    /// Returns sampling interval in nanoseconds.
    pub const fn sampling_interval_ns(&self) -> u64 {
        self.sampling_interval_ns
    }

    /// Returns return sample count.
    pub const fn samples(&self) -> usize {
        self.samples
    }

    /// Returns realized volatility in basis points.
    pub const fn realized_vol_bps(&self) -> u32 {
        self.realized_vol_bps
    }

    /// Returns sign-flip microstructure noise proxy in basis points.
    pub const fn noise_ratio_bps(&self) -> u16 {
        self.noise_ratio_bps
    }
}

/// Borrowed volatility signature estimator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilitySignatureEstimator;

impl VolatilitySignatureEstimator {
    /// Estimates one signature-plot point from borrowed returns.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when interval or returns are
    /// empty.
    pub fn estimate(
        sampling_interval_ns: u64,
        returns_bps: &[i32],
    ) -> Result<VolatilitySignatureSnapshot, AnalyticsError> {
        if sampling_interval_ns == 0 || returns_bps.is_empty() {
            return Err(AnalyticsError::InvalidTrade);
        }
        let (realized_vol_bps, noise_ratio_bps) = volatility_and_noise(returns_bps);
        Ok(VolatilitySignatureSnapshot {
            sampling_interval_ns,
            samples: returns_bps.len(),
            realized_vol_bps,
            noise_ratio_bps,
        })
    }
}

/// Intraday volatility seasonality bucket snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilitySeasonalitySnapshot {
    bucket: usize,
    samples: u64,
    realized_vol_bps: u32,
    mean_abs_return_bps: u32,
    jump_count: u64,
}

impl VolatilitySeasonalitySnapshot {
    /// Returns bucket index.
    pub const fn bucket(&self) -> usize {
        self.bucket
    }

    /// Returns sample count.
    pub const fn samples(&self) -> u64 {
        self.samples
    }

    /// Returns realized volatility in basis points.
    pub const fn realized_vol_bps(&self) -> u32 {
        self.realized_vol_bps
    }

    /// Returns mean absolute return in basis points.
    pub const fn mean_abs_return_bps(&self) -> u32 {
        self.mean_abs_return_bps
    }

    /// Returns jump count.
    pub const fn jump_count(&self) -> u64 {
        self.jump_count
    }
}

/// Fixed-bucket intraday volatility seasonality tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolatilitySeasonalityTracker<const N: usize = 96> {
    sum_sq: [u128; N],
    sum_abs: [u128; N],
    samples: [u64; N],
    jump_count: [u64; N],
    jump_threshold_bps: u32,
}

impl<const N: usize> VolatilitySeasonalityTracker<N> {
    /// Creates a seasonality tracker.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when capacity is zero.
    pub const fn new(jump_threshold_bps: u32) -> Result<Self, AnalyticsError> {
        if N == 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            sum_sq: [0; N],
            sum_abs: [0; N],
            samples: [0; N],
            jump_count: [0; N],
            jump_threshold_bps,
        })
    }

    /// Records a return for a bucket.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when bucket is out of range.
    pub fn on_return(&mut self, bucket: usize, return_bps: i32) -> Result<(), AnalyticsError> {
        if bucket >= N {
            return Err(AnalyticsError::InvalidTrade);
        }
        let abs = return_bps.unsigned_abs();
        self.sum_abs[bucket] = self.sum_abs[bucket].saturating_add(u128::from(abs));
        self.sum_sq[bucket] =
            self.sum_sq[bucket].saturating_add(u128::from(abs).saturating_mul(u128::from(abs)));
        self.samples[bucket] = self.samples[bucket].saturating_add(1);
        if abs >= self.jump_threshold_bps {
            self.jump_count[bucket] = self.jump_count[bucket].saturating_add(1);
        }
        Ok(())
    }

    /// Returns one bucket snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when bucket is out of range.
    pub fn snapshot(&self, bucket: usize) -> Result<VolatilitySeasonalitySnapshot, AnalyticsError> {
        if bucket >= N {
            return Err(AnalyticsError::InvalidTrade);
        }
        let samples = self.samples[bucket];
        let realized_vol_bps = if samples == 0 {
            0
        } else {
            u32::try_from(isqrt_u128(self.sum_sq[bucket] / u128::from(samples))).unwrap_or(u32::MAX)
        };
        let mean_abs_return_bps = if samples == 0 {
            0
        } else {
            u32::try_from(self.sum_abs[bucket] / u128::from(samples)).unwrap_or(u32::MAX)
        };
        Ok(VolatilitySeasonalitySnapshot {
            bucket,
            samples,
            realized_vol_bps,
            mean_abs_return_bps,
            jump_count: self.jump_count[bucket],
        })
    }

    /// Resets all buckets.
    pub fn reset(&mut self) {
        self.sum_sq = [0; N];
        self.sum_abs = [0; N];
        self.samples = [0; N];
        self.jump_count = [0; N];
    }
}
