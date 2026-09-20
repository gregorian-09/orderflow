use super::*;

/// In-memory accumulator that updates analytics state from normalized trades.
#[derive(Default)]
pub struct AnalyticsAccumulator {
    snapshot: AnalyticsSnapshot,
    volume_profile: HashMap<i64, i64>,
    session_trade_count: u64,
    session_turnover: i128,
    session_candle: SessionCandleSnapshot,
    session_trades: Vec<RecentTradeSample>,
    #[cfg(feature = "tickbar")]
    tick_aggregator: Option<tickbar::TickAggregator>,
    #[cfg(feature = "tickbar")]
    tick_interval_ns: i64,
}

impl std::fmt::Debug for AnalyticsAccumulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyticsAccumulator")
            .field("snapshot", &self.snapshot)
            .field("session_trade_count", &self.session_trade_count)
            .field("session_turnover", &self.session_turnover)
            .field("session_candle", &self.session_candle)
            .field("session_trades", &self.session_trades.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
struct RecentTradeSample {
    price: i64,
    size: i64,
    ts_exchange_ns: u64,
}

impl AnalyticsAccumulator {
    /// Applies a trade print to analytics and recomputes profile levels.
    pub fn on_trade(&mut self, trade: &TradePrint) {
        self.snapshot.last_price = trade.price;
        if self.session_trade_count == 0 {
            self.session_candle.open = trade.price;
            self.session_candle.high = trade.price;
            self.session_candle.low = trade.price;
            self.session_candle.first_ts_exchange_ns = trade.ts_exchange_ns;
        } else {
            self.session_candle.high = self.session_candle.high.max(trade.price);
            self.session_candle.low = self.session_candle.low.min(trade.price);
        }
        self.session_candle.close = trade.price;
        self.session_candle.trade_count = self.session_trade_count.saturating_add(1);
        self.session_candle.last_ts_exchange_ns = trade.ts_exchange_ns;
        self.session_trade_count = self.session_trade_count.saturating_add(1);
        self.session_turnover += (trade.price as i128) * (trade.size as i128);
        self.session_trades.push(RecentTradeSample {
            price: trade.price,
            size: trade.size,
            ts_exchange_ns: trade.ts_exchange_ns,
        });
        if self.session_trades.len() > MAX_SESSION_TRADES {
            self.session_trades.remove(0);
        }
        *self.volume_profile.entry(trade.price).or_insert(0) += trade.size;
        match trade.aggressor_side {
            Side::Bid => {
                self.snapshot.sell_volume += trade.size;
                self.snapshot.delta -= trade.size;
                self.snapshot.cumulative_delta -= trade.size;
            }
            Side::Ask => {
                self.snapshot.buy_volume += trade.size;
                self.snapshot.delta += trade.size;
                self.snapshot.cumulative_delta += trade.size;
            }
        }
        #[cfg(feature = "tickbar")]
        if let Some(ref mut agg) = self.tick_aggregator {
            let tick = tickbar::Tick::from_trade(
                trade.ts_exchange_ns as i64,
                trade.price as f64,
                trade.size as f64,
            );
            let _ = agg.push_tick(tick);
        }
        self.recompute_profile_levels();
    }

    /// Resets session delta and directional volume, keeps cumulative profile.
    pub fn reset_session_delta(&mut self) {
        self.snapshot.delta = 0;
        self.snapshot.buy_volume = 0;
        self.snapshot.sell_volume = 0;
        self.session_trade_count = 0;
        self.session_turnover = 0;
        self.session_candle = SessionCandleSnapshot::default();
        self.session_trades.clear();
    }

    /// Resets all session analytics and volume-profile state.
    pub fn reset_session(&mut self) {
        self.snapshot = AnalyticsSnapshot::default();
        self.volume_profile.clear();
        self.session_trade_count = 0;
        self.session_turnover = 0;
        self.session_candle = SessionCandleSnapshot::default();
        self.session_trades.clear();
    }

    /// Returns a copy of current analytics state.
    pub fn snapshot(&self) -> AnalyticsSnapshot {
        self.snapshot.clone()
    }

    /// Returns additive derived analytics for the current session accumulator state.
    pub fn derived_snapshot(&self) -> DerivedAnalyticsSnapshot {
        let total_volume = self.snapshot.buy_volume + self.snapshot.sell_volume;
        let vwap = if total_volume > 0 {
            (self.session_turnover / total_volume as i128) as i64
        } else {
            0
        };
        let average_trade_size = if self.session_trade_count > 0 {
            total_volume / self.session_trade_count as i64
        } else {
            0
        };
        let imbalance_bps = if total_volume > 0 {
            (self.snapshot.delta * 10_000) / total_volume
        } else {
            0
        };
        DerivedAnalyticsSnapshot {
            total_volume,
            trade_count: self.session_trade_count,
            vwap,
            average_trade_size,
            imbalance_bps,
        }
    }

    /// Returns candle-style session summary for the current analytics session.
    pub fn session_candle_snapshot(&self) -> SessionCandleSnapshot {
        self.session_candle.clone()
    }

    /// Returns candle-style summary for trades observed inside a rolling interval.
    pub fn interval_candle_snapshot(&self, window_ns: u64) -> IntervalCandleSnapshot {
        let Some(last_trade) = self.session_trades.last() else {
            return IntervalCandleSnapshot {
                window_ns,
                ..IntervalCandleSnapshot::default()
            };
        };
        let cutoff = last_trade.ts_exchange_ns.saturating_sub(window_ns);
        let mut trades = self
            .session_trades
            .iter()
            .filter(|trade| trade.ts_exchange_ns >= cutoff);

        let Some(first) = trades.next() else {
            return IntervalCandleSnapshot {
                window_ns,
                ..IntervalCandleSnapshot::default()
            };
        };

        let mut snap = IntervalCandleSnapshot {
            window_ns,
            open: first.price,
            high: first.price,
            low: first.price,
            close: first.price,
            trade_count: 1,
            total_volume: first.size,
            vwap: 0,
            first_ts_exchange_ns: first.ts_exchange_ns,
            last_ts_exchange_ns: first.ts_exchange_ns,
        };
        let mut turnover = (first.price as i128) * (first.size as i128);

        for trade in trades {
            snap.high = snap.high.max(trade.price);
            snap.low = snap.low.min(trade.price);
            snap.close = trade.price;
            snap.trade_count = snap.trade_count.saturating_add(1);
            snap.total_volume += trade.size;
            snap.last_ts_exchange_ns = trade.ts_exchange_ns;
            turnover += (trade.price as i128) * (trade.size as i128);
        }

        if snap.total_volume > 0 {
            snap.vwap = (turnover / snap.total_volume as i128) as i64;
        }

        snap
    }

    /// Creates an accumulator with a tickbar aggregator at the given interval.
    #[cfg(feature = "tickbar")]
    pub fn with_tickbar(interval_ns: i64) -> Self {
        let mut acc = Self::default();
        if interval_ns > 0 {
            if let Ok(agg) = tickbar::TickAggregator::builder()
                .interval(std::time::Duration::from_nanos(interval_ns as u64))
                .build()
            {
                acc.tick_aggregator = Some(agg);
                acc.tick_interval_ns = interval_ns;
            }
        }
        acc
    }

    /// Returns completed bars from the tickbar aggregator and resets for continued collection.
    #[cfg(feature = "tickbar")]
    pub fn bar_series(&mut self) -> Option<Vec<CompletedBar>> {
        let agg = self.tick_aggregator.take()?;
        let interval_ns = self.tick_interval_ns;
        let series = agg.finalize();
        let bars: Vec<CompletedBar> = series
            .as_slice()
            .iter()
            .map(|b| CompletedBar {
                timestamp_ns: b.timestamp_nanos,
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                volume: b.volume,
                tick_count: b.tick_count as u64,
                vwap: b.vwap,
            })
            .collect();

        self.tick_aggregator = tickbar::TickAggregator::builder()
            .interval(std::time::Duration::from_nanos(interval_ns as u64))
            .build()
            .ok();

        if bars.is_empty() {
            None
        } else {
            Some(bars)
        }
    }

    /// Removes the tickbar aggregator, freeing associated state.
    #[cfg(feature = "tickbar")]
    pub fn reset_tickbar(&mut self) {
        self.tick_aggregator = None;
        self.tick_interval_ns = 0;
    }

    fn recompute_profile_levels(&mut self) {
        if self.volume_profile.is_empty() {
            return;
        }

        let mut prices: Vec<i64> = self.volume_profile.keys().copied().collect();
        prices.sort_unstable();
        let total_volume: i64 = self.volume_profile.values().sum();
        if total_volume <= 0 {
            return;
        }

        let mut poc_price = prices[0];
        let mut poc_volume = self.volume_profile[&poc_price];
        for p in &prices {
            let v = self.volume_profile[p];
            if v > poc_volume || (v == poc_volume && *p > poc_price) {
                poc_price = *p;
                poc_volume = v;
            }
        }
        self.snapshot.point_of_control = poc_price;

        let target = ((total_volume as f64) * 0.70).ceil() as i64;
        let mut covered = poc_volume;
        let mut low = poc_price;
        let mut high = poc_price;

        let poc_idx = prices.iter().position(|p| *p == poc_price).unwrap_or(0);
        let mut left: isize = poc_idx as isize - 1;
        let mut right: usize = poc_idx + 1;

        while covered < target && (left >= 0 || right < prices.len()) {
            let left_vol = if left >= 0 {
                self.volume_profile[&prices[left as usize]]
            } else {
                -1
            };
            let right_vol = if right < prices.len() {
                self.volume_profile[&prices[right]]
            } else {
                -1
            };

            if right_vol > left_vol {
                covered += right_vol.max(0);
                high = prices[right];
                right += 1;
            } else {
                covered += left_vol.max(0);
                low = prices[left as usize];
                left -= 1;
            }
        }

        self.snapshot.value_area_low = low;
        self.snapshot.value_area_high = high;
    }
}
