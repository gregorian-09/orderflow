use super::*;

/// Tracks effective and realised spread for recent trades.
///
/// Records each trade's price together with the prevailing mid price,
/// enabling rolling computation of half-spread cost and (when queried N ticks
/// later) realised spread based on mid-price movement.
#[derive(Debug, Clone)]
pub struct SpreadTracker {
    /// Rolling buffer of trade samples (price, mid price at trade time, timestamp).
    samples: Vec<SpreadSample>,
    /// Maximum number of samples retained.
    max_samples: usize,
}

/// One recorded trade for spread tracking.
#[derive(Debug, Clone, Copy)]
pub struct SpreadSample {
    /// Trade execution price.
    pub trade_price: i64,
    /// Mid price at the time of the trade.
    pub mid_price: i64,
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
}

impl SpreadTracker {
    /// Creates a new tracker that retains up to `max_samples` recent trades.
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples.min(4096)),
            max_samples,
        }
    }

    /// Records a trade with the prevailing mid price.
    pub fn on_trade(&mut self, trade_price: i64, mid_price: i64, ts_exchange_ns: u64) {
        bounded_push(
            &mut self.samples,
            self.max_samples,
            SpreadSample {
                trade_price,
                mid_price,
                ts_exchange_ns,
            },
        );
    }

    /// Returns the effective spread in bps for the most recent trade.
    /// Returns 0 if no trades recorded.
    pub fn last_effective_spread_bps(&self) -> i64 {
        self.samples
            .last()
            .map(|s| compute_effective_spread_bps(s.trade_price, s.mid_price))
            .unwrap_or(0)
    }

    /// Returns the average half-spread cost (`effective_spread / 2`) over the last `window` trades.
    pub fn average_half_spread_cost_bps(&self, window: usize) -> i64 {
        let start = self.samples.len().saturating_sub(window);
        let slice = &self.samples[start..];
        if slice.is_empty() {
            return 0;
        }
        let sum: i64 = slice
            .iter()
            .map(|s| compute_effective_spread_bps(s.trade_price, s.mid_price) / 2)
            .sum();
        sum / slice.len() as i64
    }

    /// Returns the realised spread in bps for the trade `hold_ticks` ago.
    ///
    /// Compares the mid price at that trade vs the latest mid price.
    /// Returns 0 if insufficient history.
    pub fn realised_spread_bps(&self, hold_ticks: usize) -> i64 {
        if self.samples.len() < hold_ticks + 1 {
            return 0;
        }
        let entry_idx = self.samples.len().saturating_sub(hold_ticks + 1);
        let entry = self.samples[entry_idx];
        let latest = self.samples[self.samples.len() - 1];
        let mid_move = compute_effective_spread_bps(latest.mid_price, entry.mid_price);
        let eff = compute_effective_spread_bps(entry.trade_price, entry.mid_price);
        compute_realised_spread_bps(eff, mid_move)
    }

    /// Returns the number of samples currently tracked.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Clears all samples.
    pub fn reset(&mut self) {
        self.samples.clear();
    }
}

/// Tracks order-book update events for rate and size-distribution analytics.
#[derive(Debug, Clone)]
pub struct BookEventTracker {
    /// Rolling buffer of book events.
    events: Vec<BookEventSample>,
    /// Max events retained.
    max_events: usize,
}

/// A single book update event for analytics.
#[derive(Debug, Clone, Copy)]
pub struct BookEventSample {
    /// Side of the book that was modified.
    pub side: Side,
    /// Action type.
    pub action: BookAction,
    /// Size affected.
    pub size: i64,
    /// Timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
}

impl BookEventTracker {
    /// Creates a new tracker retaining up to `max_events` recent events.
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_events.min(65536)),
            max_events,
        }
    }

    /// Records a book update event.
    pub fn on_book_update(
        &mut self,
        side: Side,
        action: BookAction,
        size: i64,
        ts_exchange_ns: u64,
    ) {
        bounded_push(
            &mut self.events,
            self.max_events,
            BookEventSample {
                side,
                action,
                size,
                ts_exchange_ns,
            },
        );
    }

    /// Returns the number of events in the time window `window_ns` (nanoseconds) per side.
    pub fn event_count_in_window(&self, window_ns: u64, side: Option<Side>) -> (usize, usize) {
        let Some(latest) = self.events.last() else {
            return (0, 0);
        };
        let cutoff = latest.ts_exchange_ns.saturating_sub(window_ns);
        let mut bid_count = 0usize;
        let mut ask_count = 0usize;
        for e in self.events.iter().rev() {
            if e.ts_exchange_ns < cutoff {
                break;
            }
            match e.side {
                Side::Bid => bid_count += 1,
                Side::Ask => ask_count += 1,
            }
        }
        match side {
            Some(Side::Bid) => (bid_count, 0),
            Some(Side::Ask) => (0, ask_count),
            None => (bid_count, ask_count),
        }
    }

    /// Returns the per-side arrival (upsert) rate per second over `window_ns`.
    pub fn arrival_rate_per_sec(&self, window_ns: u64) -> (f64, f64) {
        let Some(latest) = self.events.last() else {
            return (0.0, 0.0);
        };
        let cutoff = latest.ts_exchange_ns.saturating_sub(window_ns);
        let mut bid = 0usize;
        let mut ask = 0usize;
        for e in self.events.iter().rev() {
            if e.ts_exchange_ns < cutoff {
                break;
            }
            if e.action == BookAction::Upsert {
                match e.side {
                    Side::Bid => bid += 1,
                    Side::Ask => ask += 1,
                }
            }
        }
        let secs = (window_ns as f64) / 1_000_000_000.0;
        if secs <= 0.0 {
            return (0.0, 0.0);
        }
        (bid as f64 / secs, ask as f64 / secs)
    }

    /// Returns the per-side cancel (delete) rate per second over `window_ns`.
    pub fn cancel_rate_per_sec(&self, window_ns: u64) -> (f64, f64) {
        let Some(latest) = self.events.last() else {
            return (0.0, 0.0);
        };
        let cutoff = latest.ts_exchange_ns.saturating_sub(window_ns);
        let mut bid = 0usize;
        let mut ask = 0usize;
        for e in self.events.iter().rev() {
            if e.ts_exchange_ns < cutoff {
                break;
            }
            if e.action == BookAction::Delete {
                match e.side {
                    Side::Bid => bid += 1,
                    Side::Ask => ask += 1,
                }
            }
        }
        let secs = (window_ns as f64) / 1_000_000_000.0;
        if secs <= 0.0 {
            return (0.0, 0.0);
        }
        (bid as f64 / secs, ask as f64 / secs)
    }

    /// Returns the total volume of order-book events per side in `window_ns`.
    pub fn event_volume_in_window(&self, window_ns: u64) -> (i64, i64) {
        let Some(latest) = self.events.last() else {
            return (0, 0);
        };
        let cutoff = latest.ts_exchange_ns.saturating_sub(window_ns);
        let mut bid_vol = 0i64;
        let mut ask_vol = 0i64;
        for e in self.events.iter().rev() {
            if e.ts_exchange_ns < cutoff {
                break;
            }
            match e.side {
                Side::Bid => bid_vol += e.size,
                Side::Ask => ask_vol += e.size,
            }
        }
        (bid_vol, ask_vol)
    }

    /// Returns the number of events recorded.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Clears all events.
    pub fn reset(&mut self) {
        self.events.clear();
    }
}

/// A snapshot of book-event analytics.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct BookEventAnalyticsSnapshot {
    /// Bid-side arrival (upsert) rate per second.
    pub bid_arrival_rate: f64,
    /// Ask-side arrival rate per second.
    pub ask_arrival_rate: f64,
    /// Bid-side cancel (delete) rate per second.
    pub bid_cancel_rate: f64,
    /// Ask-side cancel rate per second.
    pub ask_cancel_rate: f64,
    /// Rate of total book updates per second.
    pub change_intensity: f64,
    /// Bid event volume in window.
    pub bid_event_volume: i64,
    /// Ask event volume in window.
    pub ask_event_volume: i64,
}

impl BookEventAnalyticsSnapshot {
    /// Returns true if all fields are zero (no data).
    pub fn is_empty(&self) -> bool {
        self.bid_arrival_rate == 0.0
            && self.ask_arrival_rate == 0.0
            && self.bid_cancel_rate == 0.0
            && self.ask_cancel_rate == 0.0
            && self.change_intensity == 0.0
            && self.bid_event_volume == 0
            && self.ask_event_volume == 0
    }
}

impl Default for BookEventAnalyticsSnapshot {
    fn default() -> Self {
        Self {
            bid_arrival_rate: 0.0,
            ask_arrival_rate: 0.0,
            bid_cancel_rate: 0.0,
            ask_cancel_rate: 0.0,
            change_intensity: 0.0,
            bid_event_volume: 0,
            ask_event_volume: 0,
        }
    }
}

/// Tracks book depth before and after trades for resiliency metrics.
#[derive(Debug, Clone)]
pub struct ResiliencyTracker {
    /// Snapshots of book depth around trades.
    snapshots: Vec<ResiliencySample>,
    /// Maximum samples retained.
    max_samples: usize,
}

/// Book depth around a single trade.
#[derive(Debug, Clone, Copy)]
pub struct ResiliencySample {
    /// Bid depth immediately before trade.
    pub pre_bid_depth: i64,
    /// Ask depth immediately before trade.
    pub pre_ask_depth: i64,
    /// Timestamp right after trade (nanoseconds).
    pub post_ts: u64,
    /// Bid depth at recovery check.
    pub post_bid_depth: i64,
    /// Ask depth at recovery check.
    pub post_ask_depth: i64,
    /// Timestamp of recovery check.
    pub recovery_ts: u64,
}

impl ResiliencyTracker {
    /// Creates a new tracker with a maximum sample count.
    pub fn new(max_samples: usize) -> Self {
        Self {
            snapshots: Vec::with_capacity(max_samples.min(1024)),
            max_samples,
        }
    }

    /// Records book depth before a trade is applied.
    /// Call this before the trade updates the book.
    pub fn on_trade_pre(&mut self, bid_depth: i64, ask_depth: i64) {
        let sample = ResiliencySample {
            pre_bid_depth: bid_depth,
            pre_ask_depth: ask_depth,
            post_ts: 0,
            post_bid_depth: bid_depth,
            post_ask_depth: ask_depth,
            recovery_ts: 0,
        };
        // Place a partial sample; on_trade_post fills in the rest
        bounded_push(&mut self.snapshots, self.max_samples, sample);
    }

    /// Records book depth after a trade and sets the post-trade depth.
    /// Should be called some time after the trade (the "recovery check" point).
    pub fn on_trade_post(&mut self, bid_depth: i64, ask_depth: i64, ts_exchange_ns: u64) {
        if let Some(sample) = self.snapshots.last_mut() {
            // Only update if the post fields haven't been set yet
            if sample.post_ts == 0 {
                sample.post_bid_depth = bid_depth;
                sample.post_ask_depth = ask_depth;
                sample.post_ts = ts_exchange_ns;
                sample.recovery_ts = ts_exchange_ns;
            } else {
                // Update recovery check: later observation
                sample.recovery_ts = ts_exchange_ns;
            }
        }
    }

    /// Returns estimated recovery time in milliseconds for the most recent trade.
    /// Recovery is considered achieved when depth returns to 95% of pre-trade level.
    /// This is a heuristic based on the latest observation.
    pub fn latest_recovery_time_ms(&self) -> Option<f64> {
        let s = self.snapshots.last()?;
        if s.pre_bid_depth == 0 && s.pre_ask_depth == 0 {
            return None;
        }
        if s.post_ts == 0 || s.recovery_ts <= s.post_ts {
            return None;
        }
        let pre_total = s.pre_bid_depth + s.pre_ask_depth;
        if pre_total == 0 {
            return None;
        }
        let post_total = s.post_bid_depth + s.post_ask_depth;
        let threshold = (pre_total as f64) * 0.95;
        if (post_total as f64) >= threshold {
            // Already recovered at post_ts
            Some(0.0)
        } else {
            // Not yet recovered; estimate based on recovery_ts
            let elapsed = (s.recovery_ts - s.post_ts) as f64 / 1_000_000.0;
            let remaining = threshold - post_total as f64;
            let rate = (post_total as f64 - s.pre_bid_depth as f64 - s.pre_ask_depth as f64).abs()
                / elapsed.max(1.0);
            if rate > 0.0 {
                Some(elapsed + (remaining / rate))
            } else {
                Some(elapsed)
            }
        }
    }

    /// Returns depth elasticity: `pre_trade_depth / recovery_time_ms`.
    pub fn latest_depth_elasticity(&self) -> Option<f64> {
        let s = self.snapshots.last()?;
        let pre_total = s.pre_bid_depth + s.pre_ask_depth;
        if pre_total == 0 {
            return None;
        }
        let recovery = self.latest_recovery_time_ms()?;
        if recovery <= 0.0 {
            return None;
        }
        Some(pre_total as f64 / recovery)
    }

    /// Returns the number of samples tracked.
    pub fn sample_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Clears all samples.
    pub fn reset(&mut self) {
        self.snapshots.clear();
    }
}

/// A snapshot of book resiliency metrics for the most recent trade.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct ResiliencySnapshot {
    /// Recovery time in milliseconds (estimate).
    pub recovery_time_ms: f64,
    /// Depth elasticity (pre-trade depth / recovery time).
    pub depth_elasticity: f64,
}

impl Default for ResiliencySnapshot {
    fn default() -> Self {
        Self {
            recovery_time_ms: 0.0,
            depth_elasticity: 0.0,
        }
    }
}

/// Result of a single trade classification method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationVote {
    /// Buy / aggressive buy.
    Buy,
    /// Sell / aggressive sell.
    Sell,
    /// Unable to classify (e.g., mid-price print).
    Neutral,
}

/// Classifies trades using multiple methods: tick rule, quote rule, Lee-Ready, and consensus.
///
/// Maintains last price for tick rule and requires book access for quote-based methods.
/// The Engine feeds this tracker with `on_trade(mid_price)` and queries via `classify()`.
#[derive(Debug, Clone)]
pub struct TradeClassifier {
    /// Last observed trade price (for tick rule).
    pub(crate) last_price: Option<i64>,
    /// Configurable voting weights.
    weights: ClassifierWeights,
    /// Remote. of classifications for debugging.
    last_votes: [ClassificationVote; 3],
}

/// Weights for the consensus voting classifier.
#[derive(Debug, Clone, Copy)]
pub struct ClassifierWeights {
    /// Weight for tick rule vote.
    pub tick_weight: f64,
    /// Weight for quote rule vote.
    pub quote_weight: f64,
    /// Weight for Lee-Ready vote (0 if not used).
    pub lee_ready_weight: f64,
}

impl Default for ClassifierWeights {
    fn default() -> Self {
        Self {
            tick_weight: 0.3,
            quote_weight: 0.4,
            lee_ready_weight: 0.3,
        }
    }
}

impl Default for TradeClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl TradeClassifier {
    /// Creates a new classifier with default weights.
    pub fn new() -> Self {
        Self {
            last_price: None,
            weights: ClassifierWeights::default(),
            last_votes: [ClassificationVote::Neutral; 3],
        }
    }

    /// Creates a classifier with custom weights.
    pub fn with_weights(weights: ClassifierWeights) -> Self {
        Self {
            last_price: None,
            weights,
            last_votes: [ClassificationVote::Neutral; 3],
        }
    }

    /// Classifies a trade by tick rule based on price vs last price.
    ///
    /// Tick rule: price > last_price → buy, price < last_price → sell,
    /// price == last_price → check volume vs last volume (zero-tick).
    pub fn tick_rule(&self, price: i64, volume: i64, last_volume: i64) -> ClassificationVote {
        match self.last_price {
            Some(last) if price > last => ClassificationVote::Buy,
            Some(last) if price < last => ClassificationVote::Sell,
            Some(_) => {
                // Zero-tick: classify by comparing to last volume
                if volume > last_volume {
                    // Assume aggressive if larger volume at same price
                    ClassificationVote::Buy // conservative: default to buy for volume increase
                } else {
                    ClassificationVote::Sell
                }
            }
            None => ClassificationVote::Neutral,
        }
    }

    /// Classifies a trade by quote rule (compare to bid/ask).
    pub fn quote_rule(price: i64, best_bid: i64, best_ask: i64) -> ClassificationVote {
        if best_bid > 0 && price <= best_bid {
            ClassificationVote::Sell
        } else if best_ask > 0 && price >= best_ask {
            ClassificationVote::Buy
        } else {
            ClassificationVote::Neutral
        }
    }

    /// Classifies using Lee-Ready: quote rule at bid/ask, tick rule at mid.
    pub fn lee_ready(
        price: i64,
        best_bid: i64,
        best_ask: i64,
        last_price: Option<i64>,
        volume: i64,
        last_volume: i64,
    ) -> ClassificationVote {
        let quote = Self::quote_rule(price, best_bid, best_ask);
        if quote != ClassificationVote::Neutral {
            return quote;
        }
        // At mid price, fall back to tick rule
        let classifier = TradeClassifier {
            last_price,
            weights: ClassifierWeights::default(),
            last_votes: [ClassificationVote::Neutral; 3],
        };
        classifier.tick_rule(price, volume, last_volume)
    }

    /// Returns the consensus classification by weighted majority vote across all methods.
    ///
    /// Requires the current and last trade data plus book snapshot.
    pub fn classify(
        &mut self,
        price: i64,
        volume: i64,
        best_bid: i64,
        best_ask: i64,
    ) -> ClassificationVote {
        let last_vol = 0; // Simplified: tracker doesn't track per-trade volume
        let tick = self.tick_rule(price, volume, last_vol);
        let quote = Self::quote_rule(price, best_bid, best_ask);
        let lr = Self::lee_ready(price, best_bid, best_ask, self.last_price, volume, last_vol);

        self.last_votes = [tick, quote, lr];
        self.last_price = Some(price);

        // Weighted consensus
        let mut buy_score = 0.0f64;
        let mut sell_score = 0.0f64;

        match tick {
            ClassificationVote::Buy => buy_score += self.weights.tick_weight,
            ClassificationVote::Sell => sell_score += self.weights.tick_weight,
            _ => {}
        }
        match quote {
            ClassificationVote::Buy => buy_score += self.weights.quote_weight,
            ClassificationVote::Sell => sell_score += self.weights.quote_weight,
            _ => {}
        }
        match lr {
            ClassificationVote::Buy => buy_score += self.weights.lee_ready_weight,
            ClassificationVote::Sell => sell_score += self.weights.lee_ready_weight,
            _ => {}
        }

        if buy_score > sell_score {
            ClassificationVote::Buy
        } else if sell_score > buy_score {
            ClassificationVote::Sell
        } else {
            // Tie: prefer quote rule (most reliable)
            quote
        }
    }

    /// Returns the last votes for debug/diagnostics.
    pub fn last_votes(&self) -> [ClassificationVote; 3] {
        self.last_votes
    }

    /// Resets the classifier state.
    pub fn reset(&mut self) {
        self.last_price = None;
        self.last_votes = [ClassificationVote::Neutral; 3];
    }
}

/// A single VPIN snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct VpinSnapshot {
    /// Current VPIN value (0..1).
    pub vpin: f64,
    /// VPIN z-score relative to rolling mean/std.
    pub vpin_zscore: f64,
    /// Rolling mean VPIN.
    pub vpin_mean: f64,
    /// Rolling std VPIN.
    pub vpin_std: f64,
    /// Whether VPIN exceeds the toxicity threshold.
    pub is_toxic: bool,
    /// Number of complete buckets processed.
    pub bucket_count: u64,
}

impl Default for VpinSnapshot {
    fn default() -> Self {
        Self {
            vpin: 0.0,
            vpin_zscore: 0.0,
            vpin_mean: 0.0,
            vpin_std: 0.0,
            is_toxic: false,
            bucket_count: 0,
        }
    }
}

/// Tracks Volume-Synchronized Probability of Informed Trading (VPIN).
///
/// Accumulates buy/sell volume into fixed-size buckets, computes
/// `|buy_vol - sell_vol| / bucket_vol` per bucket, and maintains
/// a rolling window of VPIN values for mean/std and toxicity detection.
#[derive(Debug, Clone)]
pub struct VpinTracker {
    /// Volume threshold per bucket.
    bucket_volume: i64,
    /// Current bucket's buy volume.
    current_buy_vol: i64,
    /// Current bucket's sell volume.
    current_sell_vol: i64,
    /// Completed bucket VPIN values in rolling window.
    bucket_vpins: Vec<f64>,
    /// Maximum number of buckets to retain.
    max_buckets: usize,
    /// Toxicity threshold (z-score).
    toxicity_threshold: f64,
}

impl VpinTracker {
    /// Creates a new VPIN tracker with specified bucket volume and rolling window size.
    pub fn new(bucket_volume: i64, rolling_buckets: usize) -> Self {
        Self {
            bucket_volume,
            current_buy_vol: 0,
            current_sell_vol: 0,
            bucket_vpins: Vec::with_capacity(rolling_buckets),
            max_buckets: rolling_buckets,
            toxicity_threshold: 2.0,
        }
    }

    /// Sets the toxicity threshold (z-score).
    pub fn with_toxicity_threshold(mut self, threshold: f64) -> Self {
        self.toxicity_threshold = threshold;
        self
    }

    /// Feeds classified volumes into the VPIN tracker.
    ///
    /// `buy_volume` and `sell_volume` are the volumes for this event.
    /// When cumulative volume exceeds `bucket_volume`, a VPIN value is emitted.
    pub fn on_trade(&mut self, buy_volume: i64, sell_volume: i64) {
        if self.bucket_volume <= 0 || self.max_buckets == 0 {
            return;
        }
        self.current_buy_vol += buy_volume;
        self.current_sell_vol += sell_volume;

        let total = self.current_buy_vol + self.current_sell_vol;
        if total >= self.bucket_volume {
            let vpin = (self.current_buy_vol - self.current_sell_vol).unsigned_abs() as f64
                / self.bucket_volume as f64;

            bounded_push(&mut self.bucket_vpins, self.max_buckets, vpin);

            // Carry over excess volume to next bucket
            let excess = total - self.bucket_volume;
            let excess_ratio = excess as f64 / total.max(1) as f64;
            self.current_buy_vol = (self.current_buy_vol as f64 * excess_ratio) as i64;
            self.current_sell_vol = (self.current_sell_vol as f64 * excess_ratio) as i64;
        }
    }

    /// Returns the current VPIN snapshot.
    pub fn snapshot(&self) -> VpinSnapshot {
        if self.bucket_vpins.is_empty() {
            return VpinSnapshot::default();
        }

        let latest = *self.bucket_vpins.last().unwrap_or(&0.0);
        let n = self.bucket_vpins.len() as f64;
        let mean = self.bucket_vpins.iter().sum::<f64>() / n;
        let variance = self
            .bucket_vpins
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            / n;
        let std = variance.sqrt();
        let zscore = if std > 0.0 {
            (latest - mean) / std
        } else {
            0.0
        };

        VpinSnapshot {
            vpin: latest,
            vpin_zscore: zscore,
            vpin_mean: mean,
            vpin_std: std,
            is_toxic: zscore.abs() > self.toxicity_threshold,
            bucket_count: self.bucket_vpins.len() as u64,
        }
    }

    /// Resets all state.
    pub fn reset(&mut self) {
        self.current_buy_vol = 0;
        self.current_sell_vol = 0;
        self.bucket_vpins.clear();
    }
}

/// Tracks Kyle's Lambda: `ΔP = α + λ * signed_volume + ε` over a rolling window.
///
/// Measures price impact per unit of signed order flow.
#[derive(Debug, Clone)]
pub struct KyleLambdaTracker {
    /// Rolling samples of (signed_volume, price_change).
    samples: Vec<(i64, i64)>,
    /// Maximum samples kept.
    max_samples: usize,
}

/// Snapshot of Kyle's Lambda estimation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct KyleLambdaSnapshot {
    /// Price impact coefficient λ (in bps per unit volume).
    pub lambda_bps: f64,
    /// R² of the regression.
    pub r_squared: f64,
    /// Smoothed λ over a larger window.
    pub average_lambda_bps: f64,
    /// Number of samples used.
    pub sample_count: u32,
}

impl Default for KyleLambdaSnapshot {
    fn default() -> Self {
        Self {
            lambda_bps: 0.0,
            r_squared: 0.0,
            average_lambda_bps: 0.0,
            sample_count: 0,
        }
    }
}

impl KyleLambdaTracker {
    /// Creates a tracker that retains up to `window` samples.
    pub fn new(window: usize) -> Self {
        Self {
            samples: Vec::with_capacity(window),
            max_samples: window,
        }
    }

    /// Records a trade: signed volume (positive = buy) and price change.
    pub fn on_trade(&mut self, signed_volume: i64, price_change: i64) {
        bounded_push(
            &mut self.samples,
            self.max_samples,
            (signed_volume, price_change),
        );
    }

    /// Computes λ via OLS: `λ = cov(x,y) / var(x)`, α = mean(y) - λ * mean(x).
    /// Returns (lambda_bps, r_squared, avg_bps) where lambda is scaled to bps per unit volume.
    pub fn snapshot(&self) -> KyleLambdaSnapshot {
        let n = self.samples.len() as f64;
        if n < 3.0 {
            return KyleLambdaSnapshot::default();
        }

        let mean_x = self.samples.iter().map(|(x, _)| *x as f64).sum::<f64>() / n;
        let mean_y = self.samples.iter().map(|(_, y)| *y as f64).sum::<f64>() / n;

        let cov = self
            .samples
            .iter()
            .map(|(x, y)| (*x as f64 - mean_x) * (*y as f64 - mean_y))
            .sum::<f64>()
            / n;
        let var_x = self
            .samples
            .iter()
            .map(|(x, _)| (*x as f64 - mean_x).powi(2))
            .sum::<f64>()
            / n;

        if var_x <= 0.0 {
            return KyleLambdaSnapshot::default();
        }

        let lambda = cov / var_x;
        let alpha = mean_y - lambda * mean_x;

        let ss_res: f64 = self
            .samples
            .iter()
            .map(|(x, y)| {
                let y_pred = alpha + lambda * *x as f64;
                (*y as f64 - y_pred).powi(2)
            })
            .sum();
        let ss_tot: f64 = self
            .samples
            .iter()
            .map(|(_, y)| (*y as f64 - mean_y).powi(2))
            .sum();
        let r_squared = if ss_tot > 0.0 {
            1.0 - ss_res / ss_tot
        } else {
            0.0
        };

        // Average lambda: same computation but could be smoothed with larger window
        // For now, use current lambda as average
        let avg_lambda = lambda;

        KyleLambdaSnapshot {
            lambda_bps: lambda * 10_000.0,
            r_squared,
            average_lambda_bps: avg_lambda * 10_000.0,
            sample_count: self.samples.len() as u32,
        }
    }

    /// Clears all recorded samples.
    pub fn reset(&mut self) {
        self.samples.clear();
    }
}

/// Tracks Amihud Illiquidity: `|return| / dollar_volume` per bar.
#[derive(Debug, Clone)]
pub struct AmihudTracker {
    /// Per-bar snapshots.
    bars: Vec<AmihudBar>,
    /// Rolling window size.
    window: usize,
}

#[derive(Debug, Clone, Copy)]
struct AmihudBar {
    dollar_volume: f64,
    abs_return: f64,
}

/// Snapshot of Amihud illiquidity.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct AmihudSnapshot {
    /// Current Amihud ratio.
    pub amihud_ratio: f64,
    /// Average illiquidity over window.
    pub average_illiquidity: f64,
    /// Number of bars used.
    pub bar_count: u32,
}

impl Default for AmihudSnapshot {
    fn default() -> Self {
        Self {
            amihud_ratio: 0.0,
            average_illiquidity: 0.0,
            bar_count: 0,
        }
    }
}

impl AmihudTracker {
    /// Creates a tracker with a rolling `window` of bars.
    pub fn new(window: usize) -> Self {
        Self {
            bars: Vec::with_capacity(window),
            window,
        }
    }

    /// Records a bar: close price, dollar volume, previous close.
    pub fn on_bar(&mut self, close_price: f64, dollar_volume: f64, prev_close: f64) {
        let abs_return = if prev_close > 0.0 {
            ((close_price - prev_close) / prev_close).abs()
        } else {
            0.0
        };

        bounded_push(
            &mut self.bars,
            self.window,
            AmihudBar {
                dollar_volume,
                abs_return,
            },
        );
    }

    /// Returns the current Amihud illiquidity snapshot.
    pub fn snapshot(&self) -> AmihudSnapshot {
        let n = self.bars.len() as f64;
        if n == 0.0 {
            return AmihudSnapshot::default();
        }

        let ratios: Vec<f64> = self
            .bars
            .iter()
            .map(|b| {
                if b.dollar_volume > 0.0 {
                    b.abs_return / b.dollar_volume
                } else {
                    0.0
                }
            })
            .collect();

        let latest = *ratios.last().unwrap_or(&0.0);
        let avg = ratios.iter().sum::<f64>() / n;

        AmihudSnapshot {
            amihud_ratio: latest,
            average_illiquidity: avg,
            bar_count: self.bars.len() as u32,
        }
    }

    /// Clears all recorded bars.
    pub fn reset(&mut self) {
        self.bars.clear();
    }
}

/// Tracks CVD (Cumulative Volume Delta) enhancements: ratio, z-score, divergence.
#[derive(Debug, Clone)]
pub struct CvdEnhancements {
    /// Rolling delta values over lookback window.
    delta_window: Vec<i64>,
    /// Rolling volume values over lookback window.
    volume_window: Vec<i64>,
    /// Price values for divergence detection.
    price_window: Vec<i64>,
    /// Max window size.
    window: usize,
}

/// Snapshot of CVD enhancement metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct CvdEnhancementSnapshot {
    /// Delta ratio: delta / volume in [-1, +1].
    pub delta_ratio: f64,
    /// Z-score of delta.
    pub delta_zscore: f64,
    /// Delta divergence detected (price high vs CVD low, etc.).
    pub divergence_detected: bool,
}

impl Default for CvdEnhancementSnapshot {
    fn default() -> Self {
        Self {
            delta_ratio: 0.0,
            delta_zscore: 0.0,
            divergence_detected: false,
        }
    }
}

impl CvdEnhancements {
    /// Creates a CVD enhancement tracker with the given rolling `window`.
    pub fn new(window: usize) -> Self {
        Self {
            delta_window: Vec::with_capacity(window),
            volume_window: Vec::with_capacity(window),
            price_window: Vec::with_capacity(window),
            window,
        }
    }

    /// Records a bar's worth of delta, volume, and close price.
    pub fn on_bar(&mut self, delta: i64, volume: i64, price: i64) {
        if self.window == 0 {
            return;
        }
        if self.delta_window.len() >= self.window {
            self.delta_window.remove(0);
            self.volume_window.remove(0);
            self.price_window.remove(0);
        }
        self.delta_window.push(delta);
        self.volume_window.push(volume);
        self.price_window.push(price);
    }

    /// Returns current CVD enhancement metrics.
    pub fn snapshot(&self) -> CvdEnhancementSnapshot {
        if self.delta_window.is_empty() {
            return CvdEnhancementSnapshot::default();
        }

        let n = self.delta_window.len() as f64;
        let sum_delta: i64 = self.delta_window.iter().sum();
        let sum_vol: i64 = self.volume_window.iter().sum();
        let delta_ratio = if sum_vol > 0 {
            sum_delta as f64 / sum_vol as f64
        } else {
            0.0
        };

        let mean_delta = sum_delta as f64 / n;
        let var_delta = self
            .delta_window
            .iter()
            .map(|d| (*d as f64 - mean_delta).powi(2))
            .sum::<f64>()
            / n;
        let std_delta = var_delta.sqrt();
        let last_delta = *self.delta_window.last().unwrap_or(&0) as f64;
        let delta_zscore = if std_delta > 0.0 {
            (last_delta - mean_delta) / std_delta
        } else {
            0.0
        };

        // Divergence detection: price making new highs while CVD making lower highs
        let divergence_detected = if self.price_window.len() >= 3 && self.delta_window.len() >= 3 {
            let price_rising = self.price_window.last() > self.price_window.first();
            let cvd_falling = self.delta_window.last() < self.delta_window.first();
            (price_rising && cvd_falling) || (!price_rising && !cvd_falling)
        } else {
            false
        };

        CvdEnhancementSnapshot {
            delta_ratio: delta_ratio.clamp(-1.0, 1.0),
            delta_zscore,
            divergence_detected,
        }
    }

    /// Clears all rolling CVD, volume, and price samples.
    pub fn reset(&mut self) {
        self.delta_window.clear();
        self.volume_window.clear();
        self.price_window.clear();
    }
}
