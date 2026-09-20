use super::*;

/// All detected practitioner patterns in one snapshot.
/// Snapshot of all detected practitioner patterns.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternSnapshot {
    /// Imbalance: ask/bid volume > threshold at a level.
    pub imbalance_detected: bool,
    /// Stacked imbalance: 3+ consecutive levels same-direction imbalance.
    pub stacked_imbalance_detected: bool,
    /// Absorption: high volume at level, delta positive, price stalls.
    pub absorption_detected: bool,
    /// Exhaustion: shrinking delta on successive pushes in trend.
    pub exhaustion_detected: bool,
    /// Initiation: sudden volume + delta spike breaking through level.
    pub initiation_detected: bool,
    /// Tailing: price rejects a level with large one-sided volume.
    pub tailing_detected: bool,
    /// Iceberg: same level refills after being hit.
    pub iceberg_detected: bool,
    /// Spoofing: large order appears, sits briefly, cancels.
    pub spoofing_detected: bool,
    /// Flip: large bid cancels, reappears as large ask (or vice versa).
    pub flip_detected: bool,
    /// Liquidity gap: price zone with minimal orders between dense zones.
    pub liquidity_gap_detected: bool,
    /// Stop hunt: price pierces known level, immediately reverses.
    pub stop_hunt_detected: bool,
    /// Hidden accumulation: price flat/declining, CVD rising.
    pub hidden_accumulation: bool,
    /// Hidden distribution: price flat/rising, CVD declining.
    pub hidden_distribution: bool,
    /// Trapped traders: aggressive push through level, then rejection.
    pub trapped_traders_detected: bool,
    /// Delta-clock elapsed (ns) since last significant delta event.
    pub delta_clock_ns: u64,
    /// Trend day: price beyond initial balance with sustained delta.
    pub trend_day: bool,
    /// Range day: price oscillates inside initial balance.
    pub range_day: bool,
    /// Reversal day: trend fails at key level, delta diverges.
    pub reversal_day: bool,
    /// Session type score: 0.0=range, 1.0=trend.
    pub session_type_score: f64,
    // --- Volume Profile (2.5) ---
    /// Entropy of volume distribution across price bins.
    pub volume_entropy: f64,
    /// Volume-weighted skew of distribution.
    pub volume_skew: f64,
    /// Initial balance high price.
    pub initial_balance_high: i64,
    /// Initial balance low price.
    pub initial_balance_low: i64,
    /// Number of high volume nodes (bins above mean).
    pub hvn_count: u32,
    /// Number of low volume nodes (bins below mean / 2).
    pub lvn_count: u32,
    /// VWAP per bin (price→vwap map serialized as JSON in string form).
    pub vwap_per_bin_json: [u8; 512],
    /// Composite profile: multi-session merged HVN/LVN count.
    pub composite_hvn: u32,
    /// Composite profile: multi-session merged LVN count.
    pub composite_lvn: u32,
}

impl Default for PatternSnapshot {
    fn default() -> Self {
        Self {
            imbalance_detected: false,
            stacked_imbalance_detected: false,
            absorption_detected: false,
            exhaustion_detected: false,
            initiation_detected: false,
            tailing_detected: false,
            iceberg_detected: false,
            spoofing_detected: false,
            flip_detected: false,
            liquidity_gap_detected: false,
            stop_hunt_detected: false,
            hidden_accumulation: false,
            hidden_distribution: false,
            trapped_traders_detected: false,
            delta_clock_ns: 0,
            trend_day: false,
            range_day: false,
            reversal_day: false,
            session_type_score: 0.5,
            volume_entropy: 0.0,
            volume_skew: 0.0,
            initial_balance_high: 0,
            initial_balance_low: 0,
            hvn_count: 0,
            lvn_count: 0,
            vwap_per_bin_json: [0u8; 512],
            composite_hvn: 0,
            composite_lvn: 0,
        }
    }
}

/// Detects practitioner orderflow patterns from book and trade data.
///
/// Covers 2.1 (footprint), 2.2 (DOM), 2.3 (delta), 2.4 (session classification),
/// and 2.5 (volume profile).
#[derive(Debug, Clone)]
pub struct PatternDetector {
    // --- Book level tracking ---
    bid_level_sizes: HashMap<i64, i64>,
    ask_level_sizes: HashMap<i64, i64>,
    bid_level_timestamps: HashMap<i64, u64>,
    ask_level_timestamps: HashMap<i64, u64>,
    bid_prev_sizes: HashMap<i64, i64>,
    ask_prev_sizes: HashMap<i64, i64>,

    // --- Trade tracking ---
    prior_price: i64,
    prior_cvd: i64,
    session_start_ns: u64,
    session_delta: i64,
    ib_trades: Vec<(i64, i64)>,

    // --- Push tracking (exhaustion / hidden acc/dist) ---
    push_highs: Vec<i64>,
    push_lows: Vec<i64>,
    push_deltas: Vec<i64>,
    push_volumes: Vec<i64>,

    // --- Absorption tracking ---
    level_volume: HashMap<i64, i64>,
    price_stall_count: u32,

    // --- Stacked imbalance tracking ---
    prev_level_imbalance_side: i8,
    stacked_count: u32,

    // --- Spoofing tracking ---
    level_first_seen: HashMap<(i8, i64), u64>,
    level_last_seen: HashMap<(i8, i64), u64>,
    level_max_size: HashMap<(i8, i64), i64>,

    // --- Flip tracking ---
    large_bid_prices: Vec<(u64, i64, i64)>,
    large_ask_prices: Vec<(u64, i64, i64)>,

    // --- Stop hunt / trapped tracking ---
    recent_levels: Vec<(u64, i64, i64)>,

    // --- Delta-clock ---
    last_significant_delta_ns: u64,
    last_significant_delta_value: i64,

    // --- Max price range for session type ---
    session_high: i64,
    session_low: i64,
    /// Composite (multi-session) volume profile.
    composite_level_volume: HashMap<i64, i64>,
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternDetector {
    /// Creates an empty pattern detector.
    pub fn new() -> Self {
        Self {
            bid_level_sizes: HashMap::new(),
            ask_level_sizes: HashMap::new(),
            bid_level_timestamps: HashMap::new(),
            ask_level_timestamps: HashMap::new(),
            bid_prev_sizes: HashMap::new(),
            ask_prev_sizes: HashMap::new(),
            prior_price: 0,
            prior_cvd: 0,
            session_start_ns: 0,
            session_delta: 0,
            ib_trades: Vec::new(),
            push_highs: Vec::new(),
            push_lows: Vec::new(),
            push_deltas: Vec::new(),
            push_volumes: Vec::new(),
            level_volume: HashMap::new(),
            price_stall_count: 0,
            prev_level_imbalance_side: 0,
            stacked_count: 0,
            level_first_seen: HashMap::new(),
            level_last_seen: HashMap::new(),
            level_max_size: HashMap::new(),
            large_bid_prices: Vec::new(),
            large_ask_prices: Vec::new(),
            recent_levels: Vec::new(),
            last_significant_delta_ns: 0,
            last_significant_delta_value: 0,
            session_high: i64::MIN,
            session_low: i64::MAX,
            composite_level_volume: HashMap::new(),
        }
    }

    /// Feeds a trade into the detector.
    #[allow(clippy::too_many_arguments)]
    pub fn on_trade(
        &mut self,
        price: i64,
        size: i64,
        _side: Side,
        ts_exchange_ns: u64,
        cumulative_delta: i64,
        _buy_volume: i64,
        _sell_volume: i64,
    ) {
        if self.session_start_ns == 0 {
            self.session_start_ns = ts_exchange_ns;
        }

        // Track session high/low
        if price > self.session_high {
            self.session_high = price;
        }
        if price < self.session_low {
            self.session_low = price;
        }

        // Track initial balance (first 30 min)
        let ib_window_ns = 30 * 60 * 1_000_000_000u64;
        if ts_exchange_ns.saturating_sub(self.session_start_ns) <= ib_window_ns {
            bounded_push(
                &mut self.ib_trades,
                DEFAULT_PATTERN_HISTORY_CAP,
                (price, size),
            );
        }

        // Accumulate level volume for absorption detection
        *self.level_volume.entry(price).or_insert(0) += size;
        // Accumulate composite (multi-session) volume profile
        *self.composite_level_volume.entry(price).or_insert(0) += size;
        prune_hash_map(&mut self.level_volume, DEFAULT_PATTERN_PRICE_LEVEL_CAP);
        prune_hash_map(
            &mut self.composite_level_volume,
            DEFAULT_PATTERN_PRICE_LEVEL_CAP,
        );

        // Delta-clock: track last significant delta change
        let delta_change = (cumulative_delta - self.last_significant_delta_value).abs();
        if delta_change > 1000 {
            self.last_significant_delta_ns = ts_exchange_ns;
            self.last_significant_delta_value = cumulative_delta;
        }

        self.session_delta = cumulative_delta;
        self.prior_cvd = cumulative_delta;

        // Track price pushes for exhaustion detection
        let is_new_push = if price > self.prior_price {
            self.push_highs.last().map(|&h| price > h).unwrap_or(true)
        } else if price < self.prior_price {
            self.push_lows.last().map(|&l| price < l).unwrap_or(true)
        } else {
            false
        };

        if is_new_push {
            if self.push_highs.len() >= 20 {
                self.push_highs.remove(0);
                self.push_lows.remove(0);
                self.push_deltas.remove(0);
                self.push_volumes.remove(0);
            }
            self.push_highs.push(price);
            self.push_lows.push(price);
            self.push_deltas.push(cumulative_delta);
            self.push_volumes.push(size);
        }

        // Track price stall (for absorption): mid price hasn't moved
        if price == self.prior_price {
            self.price_stall_count += 1;
        } else {
            self.price_stall_count = 0;
        }

        self.prior_price = price;
    }

    /// Feeds a book update for DOM/liquidity pattern detection.
    pub fn on_book_update(&mut self, side: Side, price: i64, size: i64) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let side_key: i8 = match side {
            Side::Bid => 0,
            Side::Ask => 1,
        };
        let level_key = (side_key, price);

        if size > 0 {
            // --- Iceberg detection: track size history ---
            let level_sizes = match side {
                Side::Bid => &mut self.bid_level_sizes,
                Side::Ask => &mut self.ask_level_sizes,
            };
            let level_timestamps = match side {
                Side::Bid => &mut self.bid_level_timestamps,
                Side::Ask => &mut self.ask_level_timestamps,
            };
            let prev_sizes = match side {
                Side::Bid => &mut self.bid_prev_sizes,
                Side::Ask => &mut self.ask_prev_sizes,
            };

            if let Some(&prev_size) = level_sizes.get(&price) {
                prev_sizes.insert(price, prev_size);
            }
            level_sizes.insert(price, size);
            level_timestamps.insert(price, ts);

            // --- Spoofing detection: track order duration ---
            self.level_first_seen.entry(level_key).or_insert(ts);
            self.level_last_seen.insert(level_key, ts);
            let max_entry = self.level_max_size.entry(level_key).or_insert(0);
            if size > *max_entry {
                *max_entry = size;
            }

            // --- Flip detection: track large orders ---
            let avg_size = 5000;
            if size > avg_size * 10 {
                match side {
                    Side::Bid => bounded_push(
                        &mut self.large_bid_prices,
                        DEFAULT_PATTERN_HISTORY_CAP,
                        (ts, price, size),
                    ),
                    Side::Ask => bounded_push(
                        &mut self.large_ask_prices,
                        DEFAULT_PATTERN_HISTORY_CAP,
                        (ts, price, size),
                    ),
                }
            }

            // --- Recent levels for stop hunt / trapped ---
            bounded_push(
                &mut self.recent_levels,
                DEFAULT_PATTERN_HISTORY_CAP,
                (ts, price, size),
            );
        } else {
            // Level removed
            match side {
                Side::Bid => {
                    self.bid_level_sizes.remove(&price);
                    self.bid_level_timestamps.remove(&price);
                }
                Side::Ask => {
                    self.ask_level_sizes.remove(&price);
                    self.ask_level_timestamps.remove(&price);
                }
            }

            // Check if this was a large order that cancelled quickly (spoofing candidate)
            if let Some(&first_seen) = self.level_first_seen.get(&level_key) {
                let dwell = ts.saturating_sub(first_seen);
                let max_sz = *self.level_max_size.get(&level_key).unwrap_or(&0);
                if max_sz > 10000 && dwell < 500_000_000 {
                    // Large order, cancelled within 500ms — spoofing candidate
                }
            }
            self.level_first_seen.remove(&level_key);
            self.level_last_seen.remove(&level_key);
            self.level_max_size.remove(&level_key);
        }

        // Prune old entries
        let cutoff = ts.saturating_sub(10_000_000_000);
        self.large_bid_prices.retain(|(t, _, _)| *t > cutoff);
        self.large_ask_prices.retain(|(t, _, _)| *t > cutoff);
        self.recent_levels.retain(|(t, _, _)| *t > cutoff);
        prune_hash_map(&mut self.bid_level_sizes, DEFAULT_PATTERN_PRICE_LEVEL_CAP);
        prune_hash_map(&mut self.ask_level_sizes, DEFAULT_PATTERN_PRICE_LEVEL_CAP);
        prune_hash_map(
            &mut self.bid_level_timestamps,
            DEFAULT_PATTERN_PRICE_LEVEL_CAP,
        );
        prune_hash_map(
            &mut self.ask_level_timestamps,
            DEFAULT_PATTERN_PRICE_LEVEL_CAP,
        );
        prune_hash_map(&mut self.bid_prev_sizes, DEFAULT_PATTERN_PRICE_LEVEL_CAP);
        prune_hash_map(&mut self.ask_prev_sizes, DEFAULT_PATTERN_PRICE_LEVEL_CAP);
        prune_hash_map(
            &mut self.level_first_seen,
            DEFAULT_PATTERN_PRICE_LEVEL_CAP * 2,
        );
        prune_hash_map(
            &mut self.level_last_seen,
            DEFAULT_PATTERN_PRICE_LEVEL_CAP * 2,
        );
        prune_hash_map(
            &mut self.level_max_size,
            DEFAULT_PATTERN_PRICE_LEVEL_CAP * 2,
        );
    }

    /// Computes the current pattern snapshot.
    pub fn snapshot(
        &self,
        book: &BookSnapshot,
        _total_volume: i64,
        _mean_volume: f64,
        _std_volume: f64,
    ) -> PatternSnapshot {
        let mut snap = PatternSnapshot::default();
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        // ---- 2.1 Footprint Chart Patterns ----

        // Imbalance: ask volume / bid volume > 3:1 at a level
        // Also tracks stacked imbalance (3+ consecutive levels same direction)
        let mut stacked_count = 0u32;
        let mut prev_imbalance_side: i8 = 0;
        for level in &book.asks {
            if let Some(bid_level) = book.bids.iter().find(|b| b.level == level.level) {
                let ask_vol = level.size;
                let bid_vol = bid_level.size;
                let ratio = ask_vol.max(bid_vol) as f64 / bid_vol.min(ask_vol).max(1) as f64;
                if ratio > 3.0 {
                    snap.imbalance_detected = true;
                    let side: i8 = if ask_vol > bid_vol { 1 } else { -1 };
                    if side == prev_imbalance_side {
                        stacked_count += 1;
                    } else {
                        stacked_count = 1;
                    }
                    prev_imbalance_side = side;
                }
            }
        }
        if stacked_count >= 3 {
            snap.stacked_imbalance_detected = true;
        }

        // Absorption: high volume at level, delta positive, price stalls
        for (&px, &vol) in &self.level_volume {
            if vol > 50000 && self.price_stall_count >= 3 {
                // High volume at a level, price hasn't moved — absorption
                let bid_at_level = book.bids.iter().any(|l| l.price == px);
                let ask_at_level = book.asks.iter().any(|l| l.price == px);
                if bid_at_level || ask_at_level {
                    snap.absorption_detected = true;
                    break;
                }
            }
        }

        // Exhaustion: shrinking delta on successive pushes in trend direction
        if self.push_deltas.len() >= 4 {
            let recent = &self.push_deltas[self.push_deltas.len().saturating_sub(4)..];
            let first_delta = recent[0];
            let last_delta = recent[recent.len() - 1];
            let first_abs = first_delta.unsigned_abs();
            let last_abs = last_delta.unsigned_abs();
            if first_abs > 1000 && last_abs < first_abs / 2 {
                snap.exhaustion_detected = true;
            }
        }

        // Initiation: sudden volume + delta spike above recent mean
        if self.push_volumes.len() >= 5 {
            let recent_vols: Vec<i64> = self.push_volumes.iter().rev().take(5).copied().collect();
            let mean_vol: i64 = recent_vols.iter().sum::<i64>() / recent_vols.len() as i64;
            if let Some(&last_vol) = self.push_volumes.last() {
                if last_vol > mean_vol * 3 && last_vol > 10000 {
                    let last_delta = self.push_deltas.last().copied().unwrap_or(0);
                    let prev_delta = self.push_deltas.iter().rev().nth(1).copied().unwrap_or(0);
                    if (last_delta - prev_delta).abs() > mean_vol {
                        snap.initiation_detected = true;
                    }
                }
            }
        }

        // Tailing: price rejects a level with large one-sided volume
        if self.recent_levels.len() >= 2 {
            let last_two: Vec<(u64, i64, i64)> =
                self.recent_levels.iter().rev().take(2).copied().collect();
            if last_two.len() == 2 {
                let (_, px1, sz1) = last_two[0];
                let (_, px2, sz2) = last_two[1];
                if (px1 - px2).abs() <= 1 && sz1 > 10000 && sz2 > 10000 {
                    // Large volume at consecutive prices — potential tailing
                    snap.tailing_detected = true;
                }
            }
        }

        // ---- 2.2 DOM / Liquidity Patterns ----

        // Iceberg: level refilled with same size after being removed
        for (&price, &size) in &self.bid_level_sizes {
            if let Some(&prev_size) = self.bid_prev_sizes.get(&price) {
                if size > 0 && prev_size == size {
                    snap.iceberg_detected = true;
                    break;
                }
            }
        }
        if !snap.iceberg_detected {
            for (&price, &size) in &self.ask_level_sizes {
                if let Some(&prev_size) = self.ask_prev_sizes.get(&price) {
                    if size > 0 && prev_size == size {
                        snap.iceberg_detected = true;
                        break;
                    }
                }
            }
        }

        // Spoofing: large order that appeared briefly but cancelled
        for (&key, &first_seen) in &self.level_first_seen {
            if let Some(&last_seen) = self.level_last_seen.get(&key) {
                let dwell = last_seen.saturating_sub(first_seen);
                let max_sz = self.level_max_size.get(&key).copied().unwrap_or(0);
                // Order still present: check if it's been sitting too long to be spoofing
                // We mark as candidate if large and recent
                if max_sz > 10000 && dwell > 100_000_000 && dwell < 2_000_000_000 {
                    snap.spoofing_detected = true;
                }
            }
        }

        // Flip: large bid cancels, similar large ask appears (or vice versa)
        for &(_ts_bid, px_bid, sz_bid) in &self.large_bid_prices {
            for &(_ts_ask, px_ask, sz_ask) in &self.large_ask_prices {
                let px_diff = (px_ask - px_bid).abs();
                let sz_ratio = sz_bid.max(sz_ask) as f64 / sz_bid.min(sz_ask).max(1) as f64;
                if px_diff <= 5 && sz_ratio < 2.0 {
                    snap.flip_detected = true;
                    break;
                }
            }
            if snap.flip_detected {
                break;
            }
        }

        // Liquidity gap: price zone with zero/minimal orders between two dense zones
        if !book.bids.is_empty() && !book.asks.is_empty() {
            let mut prices: Vec<i64> = book.bids.iter().map(|l| l.price).collect();
            prices.extend(book.asks.iter().map(|l| l.price));
            prices.sort();
            for w in prices.windows(2) {
                let gap = w[1].saturating_sub(w[0]);
                if gap > 5 {
                    snap.liquidity_gap_detected = true;
                    break;
                }
            }
        }

        // Stop hunt: price pierced a known level then immediately reversed
        if self.recent_levels.len() >= 4 {
            let recent: Vec<(u64, i64, i64)> =
                self.recent_levels.iter().rev().take(4).copied().collect();
            if recent.len() == 4 {
                let (_, px0, _) = recent[0];
                let (_, px1, _) = recent[1];
                let (_, px2, _) = recent[2];
                let (_, px3, _) = recent[3];
                // Price moved through a level then back
                let pierced =
                    (px0 > px1 && px2 < px1 && px3 > px2) || (px0 < px1 && px2 > px1 && px3 < px2);
                if pierced {
                    snap.stop_hunt_detected = true;
                }
            }
        }

        // ---- 2.3 Delta Pattern Detection ----

        // Hidden accumulation/distribution (existing logic, refined)
        if self.push_deltas.len() >= 3 {
            let last = self.push_deltas.last().copied().unwrap_or(0);
            let first = self.push_deltas.first().copied().unwrap_or(0);
            let price_rising =
                self.push_highs.len() >= 2 && self.push_highs.last() > self.push_highs.first();
            let price_falling =
                self.push_lows.len() >= 2 && self.push_lows.last() < self.push_lows.first();
            let cvd_rising = last > first;
            let cvd_falling = last < first;

            snap.hidden_accumulation = !price_rising && cvd_rising;
            snap.hidden_distribution = !price_falling && cvd_falling;
        }

        // Trapped traders: aggressive push through level, immediate rejection
        if self.recent_levels.len() >= 6 {
            let recent: Vec<(u64, i64, i64)> =
                self.recent_levels.iter().rev().take(6).copied().collect();
            if recent.len() == 6 {
                let px = |i: usize| recent[i].1;
                let spread = (px(0) - px(5)).abs();
                let max_px = recent.iter().map(|(_, p, _)| p).max().copied().unwrap_or(0);
                let min_px = recent.iter().map(|(_, p, _)| p).min().copied().unwrap_or(0);
                let push_range = max_px - min_px;
                if push_range > spread * 2 && push_range > 10 {
                    // Big push then snap back
                    let push_high = px(0) > px(2) && px(2) > px(4);
                    let push_low = px(0) < px(2) && px(2) < px(4);
                    let snapped = (px(0) - px(1)).abs() <= 2;
                    if (push_high || push_low) && snapped {
                        snap.trapped_traders_detected = true;
                    }
                }
            }
        }

        // Delta-clock: time since last significant delta event
        if self.last_significant_delta_ns > 0 {
            snap.delta_clock_ns = now_ns.saturating_sub(self.last_significant_delta_ns);
        }

        // ---- 2.4 Session Classification ----

        if !self.ib_trades.is_empty() {
            let ib_high = self
                .ib_trades
                .iter()
                .map(|(p, _)| p)
                .max()
                .copied()
                .unwrap_or(0);
            let ib_low = self
                .ib_trades
                .iter()
                .map(|(p, _)| p)
                .min()
                .copied()
                .unwrap_or(0);
            let current_price = self.prior_price;
            let ib_range = ib_high.saturating_sub(ib_low);
            let session_range = self.session_high.saturating_sub(self.session_low);

            if ib_range > 0 && session_range > 0 {
                let price_from_ib = if current_price > ib_high {
                    current_price.saturating_sub(ib_high) as f64 / ib_range as f64
                } else if current_price < ib_low {
                    ib_low.saturating_sub(current_price) as f64 / ib_range as f64
                } else {
                    0.0
                };

                // Trend day: beyond IB with sustained delta
                if price_from_ib > 0.5 && self.session_delta.abs() > (ib_range / 2) {
                    snap.trend_day = true;
                } else if price_from_ib < 0.2 {
                    snap.range_day = true;
                }

                // Reversal day: trend failed, delta diverges
                if session_range > ib_range * 2 {
                    let push_cvd_start = self.push_deltas.first().copied().unwrap_or(0);
                    let push_cvd_end = self.push_deltas.last().copied().unwrap_or(0);
                    let cvd_diverged = (push_cvd_end - push_cvd_start).abs() < ib_range / 4
                        && session_range > ib_range * 3;
                    if cvd_diverged {
                        snap.reversal_day = true;
                    }
                }

                // Session type score: 0=range, 1=trend
                let delta_magnitude = self.session_delta.unsigned_abs() as f64;
                let range_factor = session_range as f64 / ib_range.max(1) as f64;
                let sustained = delta_magnitude / session_range.max(1) as f64;
                let trend_score = range_factor.min(5.0) / 5.0 * 0.6 + sustained.min(1.0) * 0.4;
                snap.session_type_score = trend_score.clamp(0.0, 1.0);
            }
        }

        // ---- 2.5 Volume Profile ----
        let total_vol: i64 = self.level_volume.values().sum();
        if total_vol > 0 && !self.level_volume.is_empty() {
            let n_bins = self.level_volume.len() as f64;
            let mean_vol_f = total_vol as f64 / n_bins;

            // Entropy: -(1/ln N) * Σ(p_k * ln(p_k))
            let mut entropy_sum = 0.0;
            // Skew: volume-weighted third moment
            let mut skew_num = 0.0;
            let mut skew_den = 0.0;
            let mut hvn = 0u32;
            let mut lvn = 0u32;

            for &vol in self.level_volume.values() {
                if vol <= 0 {
                    continue;
                }
                let p_k = vol as f64 / total_vol as f64;
                entropy_sum += p_k * p_k.ln();
                let dev = vol as f64 - mean_vol_f;
                skew_num += dev.powi(3);
                skew_den += dev.powi(2);
                if vol as f64 > mean_vol_f * 1.2 {
                    hvn += 1;
                }
                if (vol as f64) < mean_vol_f * 0.5 {
                    lvn += 1;
                }
            }

            snap.volume_entropy = -(1.0 / n_bins.ln()) * entropy_sum;
            snap.volume_skew = if skew_den > 0.0 {
                skew_num / (skew_den.powf(1.5)).max(f64::EPSILON)
            } else {
                0.0
            };
            snap.hvn_count = hvn;
            snap.lvn_count = lvn;

            // VWAP per bin: serialize top 10 price levels as JSON
            let mut vwap_buf = String::with_capacity(128);
            vwap_buf.push('{');
            let mut sorted_prices: Vec<&i64> = self.level_volume.keys().collect();
            sorted_prices.sort();
            let bin_count = sorted_prices.len().min(10);
            for (i, px) in sorted_prices.iter().rev().take(bin_count).enumerate() {
                let vol = self.level_volume.get(px).copied().unwrap_or(1);
                if i > 0 {
                    vwap_buf.push(',');
                }
                vwap_buf.push_str(&format!("\"{}\":{}", px, vol));
            }
            vwap_buf.push('}');
            let bytes = vwap_buf.as_bytes();
            let copy_len = bytes.len().min(511);
            snap.vwap_per_bin_json[..copy_len].copy_from_slice(&bytes[..copy_len]);
        }

        // Composite volume profile (multi-session)
        let composite_total: i64 = self.composite_level_volume.values().sum();
        if composite_total > 0 {
            let n_bins_c = self.composite_level_volume.len() as f64;
            let mean_c = composite_total as f64 / n_bins_c;
            for &vol in self.composite_level_volume.values() {
                let vf = vol as f64;
                if vf > mean_c * 1.2 {
                    snap.composite_hvn += 1;
                }
                if vf < mean_c * 0.5 {
                    snap.composite_lvn += 1;
                }
            }
        }

        // Initial balance high/low
        if !self.ib_trades.is_empty() {
            snap.initial_balance_high = self
                .ib_trades
                .iter()
                .map(|(p, _)| p)
                .max()
                .copied()
                .unwrap_or(0);
            snap.initial_balance_low = self
                .ib_trades
                .iter()
                .map(|(p, _)| p)
                .min()
                .copied()
                .unwrap_or(0);
        }

        snap
    }

    /// Clears all detector state and rolling pattern history.
    pub fn reset(&mut self) {
        self.bid_level_sizes.clear();
        self.ask_level_sizes.clear();
        self.bid_level_timestamps.clear();
        self.ask_level_timestamps.clear();
        self.bid_prev_sizes.clear();
        self.ask_prev_sizes.clear();
        self.prior_price = 0;
        self.prior_cvd = 0;
        self.session_start_ns = 0;
        self.session_delta = 0;
        self.ib_trades.clear();
        self.push_highs.clear();
        self.push_lows.clear();
        self.push_deltas.clear();
        self.push_volumes.clear();
        self.level_volume.clear();
        self.price_stall_count = 0;
        self.prev_level_imbalance_side = 0;
        self.stacked_count = 0;
        self.level_first_seen.clear();
        self.level_last_seen.clear();
        self.level_max_size.clear();
        self.large_bid_prices.clear();
        self.large_ask_prices.clear();
        self.recent_levels.clear();
        self.last_significant_delta_ns = 0;
        self.last_significant_delta_value = 0;
        self.session_high = i64::MIN;
        self.session_low = i64::MAX;
    }
}
