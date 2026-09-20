use super::*;

/// Kinetic energy of order book activity.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KineticEnergySnapshot {
    /// Sum of recent order-book kinetic-energy proxy values.
    pub kinetic_energy: f64,
    /// Latest order-flow momentum proxy.
    pub order_flow_momentum: f64,
    /// Change between the two most recent energy observations.
    pub energy_change: f64,
}

impl Default for KineticEnergySnapshot {
    fn default() -> Self {
        Self {
            kinetic_energy: 0.0,
            order_flow_momentum: 0.0,
            energy_change: 0.0,
        }
    }
}

/// Tracks book changes to compute kinetic energy analogues.
#[derive(Debug, Clone)]
pub struct KineticEnergyTracker {
    prev_energies: Vec<f64>,
    max_len: usize,
}

impl KineticEnergyTracker {
    /// Creates a kinetic-energy tracker retaining up to `max_len` observations.
    pub fn new(max_len: usize) -> Self {
        Self {
            prev_energies: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Feeds a book update with relative level and velocity (size change).
    pub fn on_book_event(&mut self, _level: i32, size_delta: i64, ts_delta_ns: u64) {
        let velocity = if ts_delta_ns > 0 {
            size_delta as f64 / ts_delta_ns as f64
        } else {
            0.0
        };
        let energy = 0.5 * velocity * velocity;
        bounded_push(&mut self.prev_energies, self.max_len, energy);
    }
    /// Returns current kinetic-energy metrics.
    pub fn snapshot(&self) -> KineticEnergySnapshot {
        let n = self.prev_energies.len();
        if n < 2 {
            return KineticEnergySnapshot::default();
        }
        let total_ke: f64 = self.prev_energies.iter().sum();
        let _mean_ke = total_ke / n as f64;
        let momentum = self.prev_energies.last().copied().unwrap_or(0.0) * n as f64;
        let energy_change = if n >= 2 {
            self.prev_energies[n - 1] - self.prev_energies[n - 2]
        } else {
            0.0
        };
        KineticEnergySnapshot {
            kinetic_energy: total_ke,
            order_flow_momentum: momentum,
            energy_change,
        }
    }
}

// ============================================================================
/// Dark pool analytics snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DarkPoolSnapshot {
    /// Dark-pool volume as a percentage of observed total volume.
    pub dark_volume_pct: f64,
    /// Z-score of recent dark-pool volume.
    pub dark_zscore: f64,
    /// Whether dark and lit flow appear divergent.
    pub dark_lit_divergence: bool,
}

impl Default for DarkPoolSnapshot {
    fn default() -> Self {
        Self {
            dark_volume_pct: 0.0,
            dark_zscore: 0.0,
            dark_lit_divergence: false,
        }
    }
}

/// Tracks dark pool volume alongside lit volume.
#[derive(Debug, Clone)]
pub struct DarkPoolTracker {
    dark_volumes: Vec<f64>,
    lit_volumes: Vec<f64>,
    max_days: usize,
}

impl DarkPoolTracker {
    /// Creates a dark-pool tracker retaining up to `max_days` observations.
    pub fn new(max_days: usize) -> Self {
        Self {
            dark_volumes: Vec::with_capacity(max_days),
            lit_volumes: Vec::with_capacity(max_days),
            max_days,
        }
    }
    /// Records daily dark and lit volume.
    pub fn on_day(&mut self, dark_vol: f64, lit_vol: f64) {
        bounded_push_pair(
            &mut self.dark_volumes,
            &mut self.lit_volumes,
            self.max_days,
            dark_vol,
            lit_vol,
        );
    }
    /// Returns current dark-pool analytics.
    pub fn snapshot(&self) -> DarkPoolSnapshot {
        let n = self.dark_volumes.len();
        if n == 0 {
            return DarkPoolSnapshot::default();
        }
        let total_lit: f64 = self.lit_volumes.iter().sum();
        let total_dark: f64 = self.dark_volumes.iter().sum();
        let total = total_lit + total_dark;
        let pct = if total > 0.0 {
            total_dark / total * 100.0
        } else {
            0.0
        };
        let mean_dark: f64 = self.dark_volumes.iter().sum::<f64>() / n as f64;
        let var_dark: f64 = if n > 1 {
            self.dark_volumes
                .iter()
                .map(|v| (v - mean_dark).powi(2))
                .sum::<f64>()
                / (n - 1) as f64
        } else {
            0.0
        };
        let z = if var_dark > 0.0 {
            (self.dark_volumes[n - 1] - mean_dark) / var_dark.sqrt()
        } else {
            0.0
        };
        DarkPoolSnapshot {
            dark_volume_pct: pct,
            dark_zscore: z,
            dark_lit_divergence: z.abs() > 2.0,
        }
    }
}

// ============================================================================
/// Options flow snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionsFlowSnapshot {
    /// Whether a sweep has been observed in the rolling window.
    pub sweep_detected: bool,
    /// Put volume divided by call volume.
    pub put_call_ratio: f64,
    /// Absolute call-minus-put notional proxy.
    pub delta_notional: f64,
    /// Simplified gamma-positioning proxy.
    pub gamma_positioning: f64,
}

impl Default for OptionsFlowSnapshot {
    fn default() -> Self {
        Self {
            sweep_detected: false,
            put_call_ratio: 0.0,
            delta_notional: 0.0,
            gamma_positioning: 0.0,
        }
    }
}

/// Tracks options trade flow.
#[derive(Debug, Clone)]
pub struct OptionsFlowTracker {
    trades: Vec<OptionsTradeSample>,
    max_len: usize,
}

#[derive(Debug, Clone, Copy)]
struct OptionsTradeSample {
    is_call: bool,
    volume: f64,
    is_sweep: bool,
}

impl OptionsFlowTracker {
    /// Creates an options-flow tracker retaining up to `max_len` trades.
    pub fn new(max_len: usize) -> Self {
        Self {
            trades: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records an options trade observation.
    pub fn on_trade(&mut self, is_call: bool, volume: f64, _premium: f64, is_sweep: bool) {
        bounded_push(
            &mut self.trades,
            self.max_len,
            OptionsTradeSample {
                is_call,
                volume,
                is_sweep,
            },
        );
    }
    /// Returns current options-flow metrics.
    pub fn snapshot(&self) -> OptionsFlowSnapshot {
        let put_vol: f64 = self
            .trades
            .iter()
            .filter(|sample| !sample.is_call)
            .map(|sample| sample.volume)
            .sum();
        let call_vol: f64 = self
            .trades
            .iter()
            .filter(|sample| sample.is_call)
            .map(|sample| sample.volume)
            .sum();
        let ratio = if call_vol > 0.0 {
            put_vol / call_vol
        } else {
            put_vol.max(0.0)
        };
        OptionsFlowSnapshot {
            sweep_detected: self.trades.iter().any(|sample| sample.is_sweep),
            put_call_ratio: ratio,
            delta_notional: (call_vol - put_vol).abs(),
            gamma_positioning: (call_vol + put_vol).max(1.0).recip(),
        }
    }
}

// ============================================================================
/// Futures analytics snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuturesSnapshot {
    /// Front-to-deferred basis in basis points.
    pub basis_bps: f64,
    /// Deferred minus front contract price.
    pub calendar_spread: f64,
    /// Settlement-volume pressure relative to daily average volume.
    pub settlement_pressure: f64,
    /// Contract roll progress estimate.
    pub roll_progress: f64,
}

impl Default for FuturesSnapshot {
    fn default() -> Self {
        Self {
            basis_bps: 0.0,
            calendar_spread: 0.0,
            settlement_pressure: 0.0,
            roll_progress: 0.0,
        }
    }
}

// ============================================================================
// T3.2: Volatility Signature Plot
// ============================================================================

/// Volatility signature result at a specific lag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolatilitySignaturePoint {
    /// Lag used for the volatility estimate.
    pub lag: u32,
    /// Realised variance at this lag.
    pub rv: f64,
}

/// Volatility signature plot: RV at multiple lags.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolatilitySignatureSnapshot {
    /// Fixed-size array of volatility-signature points.
    pub points: [VolatilitySignaturePoint; 10],
    /// Number of valid entries in `points`.
    pub point_count: u32,
    /// First lag where realised variance materially improves.
    pub optimal_lag: u32,
}

impl Default for VolatilitySignatureSnapshot {
    fn default() -> Self {
        Self {
            points: [VolatilitySignaturePoint { lag: 0, rv: 0.0 }; 10],
            point_count: 0,
            optimal_lag: 0,
        }
    }
}

/// Computes volatility signature from return series.
#[derive(Debug, Clone)]
pub struct VolatilitySignature {
    returns: Vec<f64>,
    max_len: usize,
}

impl VolatilitySignature {
    /// Creates a volatility-signature tracker retaining up to `max_len` returns.
    pub fn new(max_len: usize) -> Self {
        Self {
            returns: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records a return sample.
    pub fn on_return(&mut self, r: f64) {
        bounded_push(&mut self.returns, self.max_len, r);
    }
    /// Returns the current volatility-signature snapshot.
    pub fn snapshot(&self) -> VolatilitySignatureSnapshot {
        let n = self.returns.len();
        let mut snap = VolatilitySignatureSnapshot::default();
        if n < 4 {
            return snap;
        }
        let lags = [1u32, 2, 3, 5, 10, 20, 30, 50, 75, 100];
        let mut prev_rv = f64::MAX;
        for (i, &lag) in lags.iter().enumerate().take(10) {
            if lag as usize >= n {
                break;
            }
            let rv: f64 = (0..n - lag as usize)
                .map(|j| (self.returns[j + lag as usize] - self.returns[j]).powi(2))
                .sum::<f64>()
                / (n - lag as usize) as f64;
            snap.points[i] = VolatilitySignaturePoint { lag, rv };
            snap.point_count = (i + 1) as u32;
            if rv < prev_rv * 0.95 && snap.optimal_lag == 0 {
                snap.optimal_lag = lag;
            }
            prev_rv = rv;
        }
        snap
    }
}

// ============================================================================
// T4.5: Agent-Type Identification (iRP, iPIN, iVPIN)
// ============================================================================

/// Agent-type identification snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentTypeSnapshot {
    /// Intensity-based relative proportion proxy.
    pub irp: f64,
    /// Informed-participation proxy.
    pub ipin: f64,
    /// Volume-synchronised informed-participation proxy.
    pub ivpin: f64,
    /// HFT reflexivity proxy based on cancel and arrival rates.
    pub hft_reflexivity: f64,
}

impl Default for AgentTypeSnapshot {
    fn default() -> Self {
        Self {
            irp: 0.0,
            ipin: 0.0,
            ivpin: 0.0,
            hft_reflexivity: 0.0,
        }
    }
}

/// Infers agent types from trade and book patterns.
#[derive(Debug, Clone)]
pub struct AgentTypeDetector {
    trade_sizes_f: Vec<f64>,
    cancel_rates: Vec<f64>,
    arrivals: Vec<f64>,
    max_len: usize,
}

impl AgentTypeDetector {
    /// Creates an agent-type detector retaining up to `max_len` observations.
    pub fn new(max_len: usize) -> Self {
        Self {
            trade_sizes_f: Vec::with_capacity(max_len),
            cancel_rates: Vec::with_capacity(max_len),
            arrivals: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records trade size and book-event rates for agent inference.
    pub fn on_event(&mut self, trade_size: i64, cancel_rate: f64, arrival_rate: f64) {
        let val = trade_size as f64;
        if self.max_len == 0 {
            return;
        }
        if self.trade_sizes_f.len() >= self.max_len {
            self.trade_sizes_f.remove(0);
            self.cancel_rates.remove(0);
            self.arrivals.remove(0);
        }
        self.trade_sizes_f.push(val);
        self.cancel_rates.push(cancel_rate);
        self.arrivals.push(arrival_rate);
    }
    /// Returns current agent-type metrics.
    pub fn snapshot(&self) -> AgentTypeSnapshot {
        let n = self.trade_sizes_f.len();
        if n < 5 {
            return AgentTypeSnapshot::default();
        }
        let mean_ts: f64 = self.trade_sizes_f.iter().sum::<f64>() / n as f64;
        let mean_cr: f64 = self.cancel_rates.iter().sum::<f64>() / n as f64;
        let mean_ar: f64 = self.arrivals.iter().sum::<f64>() / n as f64;
        let small_trade_ratio =
            self.trade_sizes_f.iter().filter(|&&s| s < 100.0).count() as f64 / n as f64;
        let irp = small_trade_ratio; // Intensity-based relative proportion of retail
        let ipin = (mean_cr / (mean_cr + mean_ar + 1.0)).clamp(0.0, 1.0);
        let ivpin = (1.0 - mean_ts / (mean_ts + 1000.0)).clamp(0.0, 1.0);
        let hft_reflexivity = (mean_cr / mean_ar.max(0.01)).clamp(0.0, 10.0);
        AgentTypeSnapshot {
            irp,
            ipin,
            ivpin,
            hft_reflexivity,
        }
    }
}

// ============================================================================
/// 40+ hand-crafted LOB features for ML models.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LOBFeatureSnapshot {
    /// Best-ask minus best-bid spread in basis points.
    pub spread_bps: f64,
    /// Total bid-depth minus ask-depth divided by total depth.
    pub depth_imbalance: f64,
    /// Size-weighted microprice estimate.
    pub microprice: f64,
    /// Average depth slope across the top levels.
    pub depth_slope: f64,
    /// Aggregate order-event intensity.
    pub order_intensity: f64,
    /// Level-1 price-pressure feature.
    pub price_pressure_1: f64,
    /// Top-5-level price-pressure feature.
    pub price_pressure_5: f64,
    /// Top-10-level price-pressure feature.
    pub price_pressure_10: f64,
    /// Level-1 bid/ask volume ratio.
    pub bid_ask_ratio_1: f64,
    /// Top-5 bid/ask volume ratio.
    pub bid_ask_ratio_5: f64,
    /// Top-10 bid/ask volume ratio.
    pub bid_ask_ratio_10: f64,
    /// Spread adjusted by absolute depth imbalance.
    pub weighted_spread: f64,
    /// Top-of-book volume concentration.
    pub volume_concentration: f64,
    /// Cancel-rate input feature.
    pub cancel_intensity: f64,
    /// Arrival-rate input feature.
    pub arrival_intensity: f64,
    /// Trade-flow imbalance input feature.
    pub trade_flow_imbalance: f64,
}

impl Default for LOBFeatureSnapshot {
    fn default() -> Self {
        Self {
            spread_bps: 0.0,
            depth_imbalance: 0.0,
            microprice: 0.0,
            depth_slope: 0.0,
            order_intensity: 0.0,
            price_pressure_1: 0.0,
            price_pressure_5: 0.0,
            price_pressure_10: 0.0,
            bid_ask_ratio_1: 0.0,
            bid_ask_ratio_5: 0.0,
            bid_ask_ratio_10: 0.0,
            weighted_spread: 0.0,
            volume_concentration: 0.0,
            cancel_intensity: 0.0,
            arrival_intensity: 0.0,
            trade_flow_imbalance: 0.0,
        }
    }
}

/// Computes LOB features from book snapshot and trade flow.
pub fn compute_lob_features(
    book: &BookSnapshot,
    trade_imbalance: f64,
    cancel_rate: f64,
    arrival_rate: f64,
) -> LOBFeatureSnapshot {
    let mut f = LOBFeatureSnapshot::default();
    let best_bid = book.bids.first().map(|l| l.price).unwrap_or(0);
    let best_ask = book.asks.first().map(|l| l.price).unwrap_or(0);
    if best_bid > 0 && best_ask > 0 {
        let mid = (best_bid + best_ask) as f64 / 2.0;
        f.spread_bps = (best_ask - best_bid) as f64 / mid * 10000.0;
        let bid_vol: i64 = book.bids.iter().map(|l| l.size).sum();
        let ask_vol: i64 = book.asks.iter().map(|l| l.size).sum();
        let total = (bid_vol + ask_vol) as f64;
        f.depth_imbalance = if total > 0.0 {
            (bid_vol - ask_vol) as f64 / total
        } else {
            0.0
        };
        f.microprice = if bid_vol > 0 && ask_vol > 0 {
            (best_bid as f64 * ask_vol as f64 + best_ask as f64 * bid_vol as f64)
                / (bid_vol + ask_vol) as f64
        } else {
            mid
        };
        if book.bids.len() >= 2 && book.asks.len() >= 2 {
            let b1 = book.bids[0].size as f64;
            let b2 = book.bids[1].size.max(1) as f64;
            let a1 = book.asks[0].size as f64;
            let a2 = book.asks[1].size.max(1) as f64;
            f.depth_slope = ((b1 / b2) + (a1 / a2)) / 2.0;
        }
        // Price pressure: cumulative bid-ask imbalance at levels 1, 5, 10
        for (i, l) in book.bids.iter().enumerate() {
            if i == 0 {
                f.price_pressure_1 -= l.size as f64;
            }
            if i < 5 {
                f.price_pressure_5 -= l.size as f64;
            }
            if i < 10 {
                f.price_pressure_10 -= l.size as f64;
            }
        }
        for (i, l) in book.asks.iter().enumerate() {
            if i == 0 {
                f.price_pressure_1 += l.size as f64;
            }
            if i < 5 {
                f.price_pressure_5 += l.size as f64;
            }
            if i < 10 {
                f.price_pressure_10 += l.size as f64;
            }
        }
        // Bid-ask volume ratios at levels 1, 5, 10
        let bid1 = book.bids.first().map(|l| l.size).unwrap_or(1).max(1) as f64;
        let ask1 = book.asks.first().map(|l| l.size).unwrap_or(1).max(1) as f64;
        f.bid_ask_ratio_1 = bid1 / ask1;
        let bid5: i64 = book.bids.iter().take(5).map(|l| l.size).sum();
        let ask5: i64 = book.asks.iter().take(5).map(|l| l.size).sum();
        f.bid_ask_ratio_5 = bid5.max(1) as f64 / ask5.max(1) as f64;
        let bid10: i64 = book.bids.iter().take(10).map(|l| l.size).sum();
        let ask10: i64 = book.asks.iter().take(10).map(|l| l.size).sum();
        f.bid_ask_ratio_10 = bid10.max(1) as f64 / ask10.max(1) as f64;
        f.weighted_spread = f.spread_bps * (1.0 - f.depth_imbalance.abs());
        f.volume_concentration = (bid1 + ask1) / (bid10 + ask10).max(1) as f64;
    }
    f.cancel_intensity = cancel_rate;
    f.arrival_intensity = arrival_rate;
    f.trade_flow_imbalance = trade_imbalance;
    f
}

// ============================================================================
// T5.2: Dark-Lit Correlation
// ============================================================================

/// Snapshot of dark-lit imbalance correlation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DarkLitCorrelationSnapshot {
    /// Rolling correlation between dark and lit imbalances.
    pub correlation: f64,
    /// Whether negative dark-lit correlation suggests liquidity siphoning.
    pub siphon_active: bool,
}

impl Default for DarkLitCorrelationSnapshot {
    fn default() -> Self {
        Self {
            correlation: 0.0,
            siphon_active: false,
        }
    }
}

/// Tracks rolling dark-lit imbalance correlation.
#[derive(Debug, Clone)]
pub struct DarkLitCorrelator {
    dark_imbalances: Vec<f64>,
    lit_imbalances: Vec<f64>,
    max_len: usize,
}

impl DarkLitCorrelator {
    /// Creates a correlator retaining up to `max_len` imbalance pairs.
    pub fn new(max_len: usize) -> Self {
        Self {
            dark_imbalances: Vec::with_capacity(max_len),
            lit_imbalances: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records one dark and lit imbalance pair.
    pub fn on_imbalance(&mut self, dark_imb: f64, lit_imb: f64) {
        bounded_push_pair(
            &mut self.dark_imbalances,
            &mut self.lit_imbalances,
            self.max_len,
            dark_imb,
            lit_imb,
        );
    }
    /// Returns current dark-lit correlation metrics.
    pub fn snapshot(&self) -> DarkLitCorrelationSnapshot {
        let n = self.dark_imbalances.len();
        if n < 5 {
            return DarkLitCorrelationSnapshot::default();
        }
        let mean_d: f64 = self.dark_imbalances.iter().sum::<f64>() / n as f64;
        let mean_l: f64 = self.lit_imbalances.iter().sum::<f64>() / n as f64;
        let mut cov = 0.0;
        let mut var_d = 0.0;
        let mut var_l = 0.0;
        for i in 0..n {
            let dd = self.dark_imbalances[i] - mean_d;
            let dl = self.lit_imbalances[i] - mean_l;
            cov += dd * dl;
            var_d += dd * dd;
            var_l += dl * dl;
        }
        let denom = (var_d * var_l).sqrt().max(f64::EPSILON);
        let corr = (cov / n as f64) / denom;
        DarkLitCorrelationSnapshot {
            correlation: corr.clamp(-1.0, 1.0),
            siphon_active: corr < -0.5,
        }
    }
}

// ============================================================================
// T5.3: Institutional Flow Classification
// ============================================================================

/// Snapshot of institutional-flow classification.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstitutionalFlowSnapshot {
    /// Ratio of large buy volume to total large-trade volume.
    pub institutional_buy_ratio: f64,
    /// Concentration of large-flow directionality.
    pub crowding_score: f64,
}

impl Default for InstitutionalFlowSnapshot {
    fn default() -> Self {
        Self {
            institutional_buy_ratio: 0.0,
            crowding_score: 0.0,
        }
    }
}

/// Tracks large trades for institutional-flow classification.
#[derive(Debug, Clone)]
pub struct InstitutionalFlowTracker {
    large_trades: Vec<(i64, i64)>, // (size, side: +1 buy, -1 sell)
    total_volume: i64,
    max_len: usize,
}

impl InstitutionalFlowTracker {
    /// Creates a tracker retaining up to `max_len` large trades.
    pub fn new(max_len: usize) -> Self {
        Self {
            large_trades: Vec::with_capacity(max_len),
            total_volume: 0,
            max_len,
        }
    }
    /// Records one large trade and its inferred side.
    pub fn on_trade(&mut self, size: i64, is_buy: bool) {
        if self.max_len == 0 {
            return;
        }
        if self.large_trades.len() >= self.max_len {
            let (evicted_size, _) = self.large_trades.remove(0);
            self.total_volume = self.total_volume.saturating_sub(evicted_size);
        }
        self.large_trades.push((size, if is_buy { 1 } else { -1 }));
        self.total_volume += size;
    }
    /// Returns current institutional-flow metrics.
    pub fn snapshot(&self) -> InstitutionalFlowSnapshot {
        let n = self.large_trades.len();
        if n == 0 {
            return InstitutionalFlowSnapshot::default();
        }
        let inst_buy: i64 = self
            .large_trades
            .iter()
            .filter(|(_, s)| *s > 0)
            .map(|(sz, _)| sz)
            .sum();
        let inst_sell: i64 = self
            .large_trades
            .iter()
            .filter(|(_, s)| *s < 0)
            .map(|(sz, _)| sz)
            .sum();
        let total = inst_buy + inst_sell;
        let ratio = if total > 0 {
            inst_buy as f64 / total as f64
        } else {
            0.5
        };
        let crowding = (inst_buy as f64 - inst_sell as f64).abs() / self.total_volume.max(1) as f64;
        InstitutionalFlowSnapshot {
            institutional_buy_ratio: ratio,
            crowding_score: crowding,
        }
    }
}

// ============================================================================
// T6.2: Open Interest Analysis
// ============================================================================

/// Snapshot of open-interest analysis.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OIAnalysisSnapshot {
    /// Whether price and open interest are diverging.
    pub oi_divergence: bool,
    /// Average open-interest build rate.
    pub oi_build_rate: f64,
    /// Distance to max-pain proxy in basis points.
    pub max_pain_distance_bps: f64,
}

impl Default for OIAnalysisSnapshot {
    fn default() -> Self {
        Self {
            oi_divergence: false,
            oi_build_rate: 0.0,
            max_pain_distance_bps: 0.0,
        }
    }
}

/// Tracks open interest and price for divergence analysis.
#[derive(Debug, Clone)]
pub struct OIAnalyzer {
    oi_values: Vec<f64>,
    price_values: Vec<f64>,
    max_len: usize,
}

impl OIAnalyzer {
    /// Creates an analyzer retaining up to `max_len` observations.
    pub fn new(max_len: usize) -> Self {
        Self {
            oi_values: Vec::with_capacity(max_len),
            price_values: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records one open-interest and price observation.
    pub fn on_update(&mut self, oi: f64, price: f64) {
        if self.max_len == 0 {
            return;
        }
        if self.oi_values.len() >= self.max_len {
            self.oi_values.remove(0);
            self.price_values.remove(0);
        }
        self.oi_values.push(oi);
        self.price_values.push(price);
    }
    /// Returns current open-interest analysis metrics.
    pub fn snapshot(&self) -> OIAnalysisSnapshot {
        let n = self.oi_values.len();
        if n < 3 {
            return OIAnalysisSnapshot::default();
        }
        let oi_start = self.oi_values[0];
        let oi_end = self.oi_values[n - 1];
        let price_start = self.price_values[0];
        let price_end = self.price_values[n - 1];
        let oi_rising = oi_end > oi_start * 1.01;
        let price_rising = price_end > price_start;
        let divergence = (oi_rising && !price_rising) || (!oi_rising && price_rising);
        let total_oi_change = oi_end - oi_start;
        let days = n as f64;
        OIAnalysisSnapshot {
            oi_divergence: divergence,
            oi_build_rate: total_oi_change / days,
            max_pain_distance_bps: ((price_end - 0.0) / price_end.max(1.0) * 10000.0).abs(), // simplified
        }
    }
}

// ============================================================================
// T7.2–7.4: Market Specializations
// ============================================================================

/// Snapshot of FX-specific flow analytics.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FXSnapshot {
    /// Cross-currency correlation proxy.
    pub cross_currency_correlation: f64,
}

impl Default for FXSnapshot {
    fn default() -> Self {
        Self {
            cross_currency_correlation: 0.0,
        }
    }
}

/// Snapshot of fixed-income flow analytics.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedIncomeSnapshot {
    /// Yield-curve positioning proxy.
    pub yield_curve_positioning: f64,
    /// Duration-weighted flow proxy.
    pub duration_weighted_flow: f64,
}

impl Default for FixedIncomeSnapshot {
    fn default() -> Self {
        Self {
            yield_curve_positioning: 0.0,
            duration_weighted_flow: 0.0,
        }
    }
}

/// Snapshot of crypto-market flow analytics.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CryptoSnapshot {
    /// Exchange inflow/outflow balance proxy.
    pub exchange_flow_balance: f64,
    /// Funding-rate basis proxy.
    pub funding_rate_basis: f64,
    /// Wash-trading score proxy.
    pub wash_trading_score: f64,
}

impl Default for CryptoSnapshot {
    fn default() -> Self {
        Self {
            exchange_flow_balance: 0.0,
            funding_rate_basis: 0.0,
            wash_trading_score: 0.0,
        }
    }
}

// ============================================================================
// T8.2: Real-Time Alerts
// ============================================================================

/// Configurable real-time alert switches.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AlertRule {
    /// Enables absorption alerts.
    pub absorption_alert: bool,
    /// Enables delta-divergence alerts.
    pub delta_divergence_alert: bool,
    /// Enables stacked-imbalance alerts.
    pub stacked_imbalance_alert: bool,
    /// Enables iceberg alerts.
    pub iceberg_alert: bool,
    /// Enables VPIN toxicity alerts.
    pub vpin_toxic_alert: bool,
}

// ============================================================================
// T9: State Checkpoint
// ============================================================================

/// Trait for state serialization.
pub trait StateCheckpoint: Send + 'static {
    /// Serializes current state into an opaque checkpoint payload.
    fn checkpoint(&self) -> Result<Vec<u8>, String>;
    /// Restores state from an opaque checkpoint payload.
    fn restore(&mut self, data: &[u8]) -> Result<(), String>;
}

/// Tracker for futures contract roll and basis.
#[derive(Debug, Clone)]
pub struct FuturesTracker {
    front_prices: Vec<f64>,
    deferred_prices: Vec<f64>,
    settle_volumes: Vec<f64>,
    daily_avg_volumes: Vec<f64>,
    max_len: usize,
}

impl FuturesTracker {
    /// Creates a futures tracker retaining up to `max_len` ticks.
    pub fn new(max_len: usize) -> Self {
        Self {
            front_prices: Vec::with_capacity(max_len),
            deferred_prices: Vec::with_capacity(max_len),
            settle_volumes: Vec::with_capacity(max_len),
            daily_avg_volumes: Vec::with_capacity(max_len),
            max_len,
        }
    }
    /// Records front/deferred prices and settlement volume context.
    pub fn on_tick(
        &mut self,
        front_price: f64,
        deferred_price: f64,
        volume: f64,
        daily_avg_vol: f64,
    ) {
        if self.max_len == 0 {
            return;
        }
        if self.front_prices.len() >= self.max_len {
            self.front_prices.remove(0);
            self.deferred_prices.remove(0);
            self.settle_volumes.remove(0);
            self.daily_avg_volumes.remove(0);
        }
        self.front_prices.push(front_price);
        self.deferred_prices.push(deferred_price);
        self.settle_volumes.push(volume);
        self.daily_avg_volumes.push(daily_avg_vol);
    }
    /// Returns current futures roll and basis metrics.
    pub fn snapshot(&self) -> FuturesSnapshot {
        let n = self.front_prices.len();
        if n == 0 {
            return FuturesSnapshot::default();
        }
        let front = self.front_prices[n - 1];
        let deferred = self.deferred_prices[n - 1];
        let basis = if front > 0.0 {
            (deferred - front) / front * 10000.0
        } else {
            0.0
        };
        let spread = deferred - front;
        let settle: f64 = self.settle_volumes.iter().sum();
        let avg_vol: f64 = self.daily_avg_volumes.iter().copied().last().unwrap_or(1.0);
        let pressure = settle / avg_vol.max(1.0);
        FuturesSnapshot {
            basis_bps: basis,
            calendar_spread: spread,
            settlement_pressure: pressure,
            roll_progress: 0.0,
        }
    }
}

/// Configurable analytics thresholds and buffer sizes.
/// All fields have sensible defaults suitable for typical US equity/crypto markets.
/// Pass an instance to the runtime engine config setter or the C ABI setter to override.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalyticsConfig {
    /// Volume per VPIN bucket (default 5000).
    pub vpin_volume_bucket: i64,
    /// Max VPIN buckets (default 50).
    pub vpin_max_buckets: u32,
    /// Kyle's Lambda rolling window (default 100).
    pub kyle_lambda_max_len: u32,
    /// CVD enhancement window (default 50).
    pub cvd_max_len: u32,
    /// Volatility estimator window (default 100).
    pub vol_estimator_max_len: u32,
    /// Microstructure noise window (default 100).
    pub noise_max_len: u32,
    /// Hasbrouck VAR window (default 100).
    pub hasbrouck_max_len: u32,
    /// Almgren-Chriss window (default 100).
    pub almgren_chriss_max_len: u32,
    /// ACD model window (default 100).
    pub acd_max_len: u32,
    /// Volatility signature window (default 200).
    pub vol_signature_max_len: u32,
    /// Agent-type detector window (default 100).
    pub agent_max_len: u32,
    /// Minimum samples for agent-type detection (default 5).
    pub agent_min_samples: u32,
    /// Trade size threshold (units) for small-trade classification (default 100).
    pub agent_small_trade_threshold: f64,
    /// Trade size threshold (units) for institutional-flow classification (default 5000).
    pub institutional_trade_threshold: i64,
    /// Institutional-flow tracker window (default 100).
    pub institutional_max_len: u32,
    /// Book resiliency tracker window (default 1024).
    pub resiliency_max_len: u32,
    /// Spread decomposition window (default 100).
    pub spread_decomp_max_len: u32,
    /// Regime detector window (default 100).
    pub regime_max_len: u32,
    /// Window (nanoseconds) for cancel/arrival rate computation (default 1e9 = 1s).
    pub cancel_arrival_window_ns: u64,
    /// Book event tracker ring-buffer capacity (default 65536).
    pub event_tracker_max_len: u32,
    /// Spread tracker ring-buffer capacity (default 1024).
    pub spread_tracker_max_len: u32,
    /// Default rolling window size for trackers not otherwise specified (default 100).
    pub default_max_len: u32,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            vpin_volume_bucket: 5000,
            vpin_max_buckets: 50,
            kyle_lambda_max_len: 100,
            cvd_max_len: 50,
            vol_estimator_max_len: 100,
            noise_max_len: 100,
            hasbrouck_max_len: 100,
            almgren_chriss_max_len: 100,
            acd_max_len: 100,
            vol_signature_max_len: 200,
            agent_max_len: 100,
            agent_min_samples: 5,
            agent_small_trade_threshold: 100.0,
            institutional_trade_threshold: 5000,
            institutional_max_len: 100,
            resiliency_max_len: 1024,
            spread_decomp_max_len: 100,
            regime_max_len: 100,
            cancel_arrival_window_ns: 1_000_000_000,
            event_tracker_max_len: 65536,
            spread_tracker_max_len: 1024,
            default_max_len: 100,
        }
    }
}
