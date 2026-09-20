use super::*;

/// Realised volatility estimators: Classic, Parkinson, Garman-Klass, Yang-Zhang.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolatilitySnapshot {
    /// Close-to-close realised volatility.
    pub classic_rv: f64,
    /// Parkinson high-low volatility estimate.
    pub parkinson: f64,
    /// Garman-Klass OHLC volatility estimate.
    pub garman_klass: f64,
    /// Yang-Zhang OHLC volatility estimate.
    pub yang_zhang: f64,
}

impl Default for VolatilitySnapshot {
    fn default() -> Self {
        Self {
            classic_rv: 0.0,
            parkinson: 0.0,
            garman_klass: 0.0,
            yang_zhang: 0.0,
        }
    }
}

/// Tracks OHLC prices per bar for volatility estimation.
#[derive(Debug, Clone)]
pub struct VolatilityEstimator {
    bars: Vec<(f64, f64, f64, f64)>, // open, high, low, close
    max_bars: usize,
}

impl VolatilityEstimator {
    /// Creates an estimator retaining up to `max_bars` OHLC bars.
    pub fn new(max_bars: usize) -> Self {
        Self {
            bars: Vec::with_capacity(max_bars),
            max_bars,
        }
    }
    /// Records one OHLC bar for volatility estimation.
    pub fn on_bar(&mut self, open: f64, high: f64, low: f64, close: f64) {
        bounded_push(&mut self.bars, self.max_bars, (open, high, low, close));
    }
    /// Returns current realised-volatility estimates.
    pub fn snapshot(&self) -> VolatilitySnapshot {
        let n = self.bars.len() as f64;
        if n < 2.0 {
            return VolatilitySnapshot::default();
        }
        let mut classic_sum = 0.0;
        let mut park_sum = 0.0;
        let mut gk_sum = 0.0;
        let mut yz_sum_o = 0.0;
        let mut yz_sum_c = 0.0;
        let mut prev_close = 0.0;
        for (i, &(open, high, low, close)) in self.bars.iter().enumerate() {
            let r = (close / open).ln();
            classic_sum += r * r;
            park_sum += ((high / low).ln()).powi(2);
            gk_sum += 0.5 * ((high / low).ln()).powi(2)
                - (2.0 * (2.0_f64).ln() - 1.0) * ((close / open).ln()).powi(2);
            if i > 0 {
                let o_c = (open / prev_close).ln();
                let c_c = (close / open).ln();
                yz_sum_o += o_c * o_c;
                yz_sum_c += c_c * c_c;
            }
            prev_close = close;
        }
        VolatilitySnapshot {
            classic_rv: (classic_sum / n).sqrt(),
            parkinson: (park_sum / (4.0 * (2.0_f64).ln() * n)).sqrt(),
            garman_klass: (gk_sum / n).sqrt(),
            yang_zhang: ((yz_sum_o / (n - 1.0)) + (yz_sum_c / (n - 1.0)) * 0.5).sqrt(),
        }
    }
}

/// Microstructure noise estimate and signal-to-noise ratio.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseSnapshot {
    /// Estimated microstructure noise variance.
    pub noise_variance: f64,
    /// Inverse-noise signal-to-noise proxy.
    pub signal_to_noise: f64,
}

impl Default for NoiseSnapshot {
    fn default() -> Self {
        Self {
            noise_variance: 0.0,
            signal_to_noise: 0.0,
        }
    }
}

/// Tracks price returns for noise estimation.
#[derive(Debug, Clone)]
pub struct MicrostructureNoise {
    returns: Vec<f64>,
    max_len: usize,
    last_price: Option<i64>,
}

impl MicrostructureNoise {
    /// Creates a noise estimator retaining up to `max_len` returns.
    pub fn new(max_len: usize) -> Self {
        Self {
            returns: Vec::with_capacity(max_len),
            max_len,
            last_price: None,
        }
    }
    /// Records a trade price for return/noise estimation.
    pub fn on_trade(&mut self, price: i64, _size: i64) {
        let prev = self.last_price.replace(price).unwrap_or(price);
        if prev <= 0 || price <= 0 {
            return;
        }
        let r = (price as f64 / prev as f64).ln();
        bounded_push(&mut self.returns, self.max_len, r);
    }
    /// Returns the current microstructure-noise snapshot.
    pub fn snapshot(&self) -> NoiseSnapshot {
        let n = self.returns.len();
        if n < 4 {
            return NoiseSnapshot::default();
        }
        let rv_lag1: f64 = self
            .returns
            .windows(2)
            .map(|w| (w[1] - w[0]).powi(2))
            .sum::<f64>()
            / (n - 1) as f64;
        let rv_lag2: f64 = if n >= 4 {
            self.returns
                .windows(3)
                .map(|w| (w[2] - w[0]).powi(2))
                .sum::<f64>()
                / (n - 2) as f64
        } else {
            rv_lag1
        };
        let noise = (rv_lag1 - rv_lag2 / 2.0) / 2.0;
        let noise = noise.max(0.0);
        NoiseSnapshot {
            noise_variance: noise,
            signal_to_noise: if noise > 0.0 { 1.0 / noise } else { 0.0 },
        }
    }
}

/// Hasbrouck bivariate VAR(1) for price impact decomposition.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HasbrouckSnapshot {
    /// Permanent price-impact component estimate.
    pub permanent_impact: f64,
    /// Temporary price-impact component estimate.
    pub temporary_impact: f64,
    /// Permanent-impact share of total impact.
    pub information_share: f64,
}

impl Default for HasbrouckSnapshot {
    fn default() -> Self {
        Self {
            permanent_impact: 0.0,
            temporary_impact: 0.0,
            information_share: 0.0,
        }
    }
}

/// Tracks returns and signed volume for Hasbrouck VAR.
#[derive(Debug, Clone)]
pub struct HasbrouckVAR {
    returns: Vec<f64>,
    signed_volumes: Vec<f64>,
    max_len: usize,
}

impl HasbrouckVAR {
    /// Creates a VAR estimator retaining up to `max_len` samples.
    pub fn new(max_len: usize) -> Self {
        Self {
            returns: Vec::with_capacity(max_len),
            signed_volumes: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records one return and signed-volume sample.
    pub fn on_trade(&mut self, ret: f64, signed_vol: f64) {
        bounded_push_pair(
            &mut self.returns,
            &mut self.signed_volumes,
            self.max_len,
            ret,
            signed_vol,
        );
    }
    /// Returns current Hasbrouck impact estimates.
    pub fn snapshot(&self) -> HasbrouckSnapshot {
        let n = self.returns.len();
        if n < 10 {
            return HasbrouckSnapshot::default();
        }
        // Simple OLS for VAR(1): r_t = a1*r_{t-1} + b1*x_{t-1}, x_t = a2*r_{t-1} + b2*x_{t-1}
        let mut sum_r1 = 0.0;
        let mut sum_x1 = 0.0;
        let mut sum_r2 = 0.0;
        let mut sum_x2 = 0.0;
        let mut sum_x1r2 = 0.0;
        let mut sum_x1x2 = 0.0;
        let mut sum_r1sq = 0.0;
        let mut sum_x1sq = 0.0;
        for i in 1..n {
            let r1 = self.returns[i - 1];
            let x1 = self.signed_volumes[i - 1];
            let r2 = self.returns[i];
            let x2 = self.signed_volumes[i];
            sum_r1 += r1;
            sum_x1 += x1;
            sum_r2 += r2;
            sum_x2 += x2;
            sum_x1r2 += x1 * r2;
            sum_x1x2 += x1 * x2;
            sum_r1sq += r1 * r1;
            sum_x1sq += x1 * x1;
        }
        let m = (n - 1) as f64;
        let denom_r = m * sum_r1sq - sum_r1 * sum_r1;
        let denom_x = m * sum_x1sq - sum_x1 * sum_x1;
        if denom_r.abs() < 1e-10 || denom_x.abs() < 1e-10 {
            return HasbrouckSnapshot::default();
        }
        let b1 = (m * sum_x1r2 - sum_x1 * sum_r2) / denom_x;
        let b2 = (m * sum_x1x2 - sum_x1 * sum_x2) / denom_x;
        // Permanent impact = IRF sum: b1 / (1 - a1) simplified
        let permanent = b1.abs();
        let temporary = b2.abs();
        let total = permanent + temporary;
        HasbrouckSnapshot {
            permanent_impact: permanent,
            temporary_impact: temporary,
            information_share: if total > 0.0 { permanent / total } else { 0.0 },
        }
    }
}

/// Almgren-Chriss market impact model.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlmgrenChrissSnapshot {
    /// Permanent impact coefficient estimate.
    pub permanent_impact_coef: f64,
    /// Temporary impact coefficient estimate.
    pub temporary_impact_coef: f64,
}

impl Default for AlmgrenChrissSnapshot {
    fn default() -> Self {
        Self {
            permanent_impact_coef: 0.0,
            temporary_impact_coef: 0.0,
        }
    }
}

/// Tracks volume and price changes for impact estimation.
#[derive(Debug, Clone)]
pub struct AlmgrenChriss {
    price_changes: Vec<f64>,
    signed_volumes: Vec<f64>,
    max_len: usize,
}

impl AlmgrenChriss {
    /// Creates an impact estimator retaining up to `max_len` samples.
    pub fn new(max_len: usize) -> Self {
        Self {
            price_changes: Vec::with_capacity(max_len),
            signed_volumes: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records one price-change and signed-volume sample.
    pub fn on_trade(&mut self, price_change: f64, signed_vol: f64) {
        bounded_push_pair(
            &mut self.price_changes,
            &mut self.signed_volumes,
            self.max_len,
            price_change,
            signed_vol,
        );
    }
    /// Returns current Almgren-Chriss impact estimates.
    pub fn snapshot(&self) -> AlmgrenChrissSnapshot {
        let n = self.price_changes.len();
        if n < 10 {
            return AlmgrenChrissSnapshot::default();
        }
        // Regress price_change ~ signed_vol (simplified power law with alpha=1)
        let mean_pc: f64 = self.price_changes.iter().sum::<f64>() / n as f64;
        let mean_sv: f64 = self.signed_volumes.iter().sum::<f64>() / n as f64;
        let mut num = 0.0;
        let mut den = 0.0;
        for i in 0..n {
            let dpc = self.price_changes[i] - mean_pc;
            let dsv = self.signed_volumes[i] - mean_sv;
            num += dpc * dsv;
            den += dsv * dsv;
        }
        let beta = if den.abs() > 1e-10 { num / den } else { 0.0 };
        AlmgrenChrissSnapshot {
            permanent_impact_coef: beta.abs() * 0.3,
            temporary_impact_coef: beta.abs() * 0.7,
        }
    }
}

/// Huang-Stoll spread decomposition.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpreadDecompositionSnapshot {
    /// Adverse-selection component estimate.
    pub adverse_selection: f64,
    /// Order-processing-cost component estimate.
    pub order_processing_cost: f64,
    /// Inventory component estimate.
    pub inventory_component: f64,
    /// Probability-of-informed-trading style proxy.
    pub pin: f64,
}

impl Default for SpreadDecompositionSnapshot {
    fn default() -> Self {
        Self {
            adverse_selection: 0.0,
            order_processing_cost: 0.0,
            inventory_component: 0.0,
            pin: 0.0,
        }
    }
}

/// Tracks spreads for Huang-Stoll decomposition.
#[derive(Debug, Clone)]
pub struct SpreadDecomposition {
    effective_spreads: Vec<f64>,
    realised_spreads: Vec<f64>,
    quoted_spreads: Vec<f64>,
    max_len: usize,
}

impl SpreadDecomposition {
    /// Creates a spread decomposition tracker retaining up to `max_len` samples.
    pub fn new(max_len: usize) -> Self {
        Self {
            effective_spreads: Vec::with_capacity(max_len),
            realised_spreads: Vec::with_capacity(max_len),
            quoted_spreads: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records effective, realised, and quoted spread observations.
    pub fn on_spread(&mut self, effective: f64, realised: f64, quoted: f64) {
        if self.max_len == 0 {
            return;
        }
        if self.effective_spreads.len() >= self.max_len {
            self.effective_spreads.remove(0);
            self.realised_spreads.remove(0);
            self.quoted_spreads.remove(0);
        }
        self.effective_spreads.push(effective);
        self.realised_spreads.push(realised);
        self.quoted_spreads.push(quoted);
    }
    /// Returns current spread decomposition metrics.
    pub fn snapshot(&self) -> SpreadDecompositionSnapshot {
        let n = self.effective_spreads.len();
        if n == 0 {
            return SpreadDecompositionSnapshot::default();
        }
        let mean_eff: f64 = self.effective_spreads.iter().sum::<f64>() / n as f64;
        let mean_real: f64 = self.realised_spreads.iter().sum::<f64>() / n as f64;
        let mean_quot: f64 = self.quoted_spreads.iter().sum::<f64>() / n as f64;
        let as_ = (mean_eff - mean_real) / 2.0;
        let opc = mean_real / 2.0;
        let inv = (mean_quot - 2.0 * mean_eff) / 2.0;
        let total = as_ + opc;
        SpreadDecompositionSnapshot {
            adverse_selection: as_.max(0.0),
            order_processing_cost: opc.max(0.0),
            inventory_component: inv.max(0.0),
            pin: if total > 0.0 { as_ / total } else { 0.0 },
        }
    }
}

/// ACD(1,1) model for trade duration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ACDSnapshot {
    /// Mean observed trade duration in nanoseconds.
    pub mean_duration_ns: f64,
    /// Trade-arrival intensity proxy.
    pub intensity: f64,
    /// Estimated ACD alpha parameter.
    pub alpha: f64,
    /// Estimated ACD beta parameter.
    pub beta: f64,
}

impl Default for ACDSnapshot {
    fn default() -> Self {
        Self {
            mean_duration_ns: 0.0,
            intensity: 0.0,
            alpha: 0.0,
            beta: 0.0,
        }
    }
}

/// Tracks trade durations and estimates ACD(1,1).
#[derive(Debug, Clone)]
pub struct ACDModel {
    durations: Vec<f64>,
    max_len: usize,
}

impl ACDModel {
    /// Creates an ACD estimator retaining up to `max_len` durations.
    pub fn new(max_len: usize) -> Self {
        Self {
            durations: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records a trade timestamp and previous trade timestamp.
    pub fn on_trade(&mut self, ts_ns: u64, prev_ts_ns: u64) {
        if prev_ts_ns > 0 {
            let d = (ts_ns.saturating_sub(prev_ts_ns)) as f64;
            bounded_push(&mut self.durations, self.max_len, d);
        }
    }
    /// Returns current ACD model estimates.
    pub fn snapshot(&self) -> ACDSnapshot {
        let n = self.durations.len();
        if n < 5 {
            return ACDSnapshot::default();
        }
        let mean_d: f64 = self.durations.iter().sum::<f64>() / n as f64;
        // Grid search for ACD(1,1): psi_i = omega + alpha * d_{i-1} + beta * psi_{i-1}
        let mut best_ll = f64::NEG_INFINITY;
        let mut best_a = 0.3;
        let mut best_b = 0.3;
        for a in [0.1, 0.2, 0.3, 0.4, 0.5] {
            for b in [0.1, 0.2, 0.3, 0.4, 0.5] {
                if a + b >= 1.0 {
                    continue;
                }
                let omega = mean_d * (1.0 - a - b);
                let mut psi = mean_d;
                let mut ll = 0.0;
                for &d in &self.durations {
                    psi = omega + a * d + b * psi;
                    if psi <= 0.0 {
                        ll = f64::NEG_INFINITY;
                        break;
                    }
                    ll += -(d / psi).ln() - d / psi;
                }
                if ll > best_ll {
                    best_ll = ll;
                    best_a = a;
                    best_b = b;
                }
            }
        }
        ACDSnapshot {
            mean_duration_ns: mean_d,
            intensity: 1.0 / mean_d.max(1.0),
            alpha: best_a,
            beta: best_b,
        }
    }
}

/// Market regime classification.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// Normal trading regime.
    Normal = 0,
    /// Stressed but not crash-like regime.
    Stressed = 1,
    /// Flash-crash style spread and volatility shock.
    FlashCrash = 2,
    /// Quiet low-spread and low-volatility regime.
    Quiet = 3,
}

/// Snapshot of market-regime classification metrics.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegimeSnapshot {
    /// Regime discriminant matching [`Regime`].
    pub regime: u32,
    /// Spread z-score.
    pub spread_z: f64,
    /// Volatility z-score.
    pub vol_z: f64,
    /// VPIN z-score.
    pub vpin_z: f64,
}

impl Default for RegimeSnapshot {
    fn default() -> Self {
        Self {
            regime: 0,
            spread_z: 0.0,
            vol_z: 0.0,
            vpin_z: 0.0,
        }
    }
}

/// Classifies market regime from spread, volatility, and VPIN.
#[derive(Debug, Clone)]
pub struct RegimeDetector {
    spreads: Vec<f64>,
    vols: Vec<f64>,
    vpins: Vec<f64>,
    max_len: usize,
}

impl RegimeDetector {
    /// Creates a regime detector retaining up to `max_len` samples.
    pub fn new(max_len: usize) -> Self {
        Self {
            spreads: Vec::with_capacity(max_len),
            vols: Vec::with_capacity(max_len),
            vpins: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records spread, volatility, and VPIN metrics.
    pub fn on_metrics(&mut self, spread: f64, vol: f64, vpin: f64) {
        if self.max_len == 0 {
            return;
        }
        if self.spreads.len() >= self.max_len {
            self.spreads.remove(0);
            self.vols.remove(0);
            self.vpins.remove(0);
        }
        self.spreads.push(spread);
        self.vols.push(vol);
        self.vpins.push(vpin);
    }
    /// Returns the current regime classification.
    pub fn snapshot(&self) -> RegimeSnapshot {
        let z = |vals: &[f64], current: f64| -> f64 {
            let n = vals.len();
            if n < 2 {
                return 0.0;
            }
            let mean: f64 = vals.iter().sum::<f64>() / n as f64;
            let var: f64 = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
            if var <= 0.0 {
                return 0.0;
            }
            (current - mean) / var.sqrt()
        };
        let spread_z = z(&self.spreads, *self.spreads.last().unwrap_or(&0.0));
        let vol_z = z(&self.vols, *self.vols.last().unwrap_or(&0.0));
        let vpin_z = z(&self.vpins, *self.vpins.last().unwrap_or(&0.0));
        let regime = if spread_z > 3.0 && vol_z > 3.0 {
            Regime::FlashCrash
        } else if spread_z > 2.0 && vol_z > 2.0 && vpin_z > 0.8 {
            Regime::Stressed
        } else if spread_z < -0.5 && vol_z < -0.5 && vpin_z < -0.5 {
            Regime::Quiet
        } else {
            Regime::Normal
        };
        RegimeSnapshot {
            regime: regime as u32,
            spread_z,
            vol_z,
            vpin_z,
        }
    }
}

// ============================================================================
