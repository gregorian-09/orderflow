#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::fmt;
use std::ops::BitOr;

/// Canonical market symbol identifier used across venues.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId {
    /// Venue/exchange identifier, e.g. `CME` or `BINANCE`.
    pub venue: String,
    /// Instrument symbol in venue format.
    pub symbol: String,
}

/// Trade or book side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Bid/buy side.
    Bid,
    /// Ask/sell side.
    Ask,
}

/// Book mutation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookAction {
    /// Insert or update a price level.
    Upsert,
    /// Remove a price level.
    Delete,
}

/// Level-2 order book update.
#[derive(Debug, Clone)]
pub struct BookUpdate {
    /// Symbol that produced the update.
    pub symbol: SymbolId,
    /// Side being mutated.
    pub side: Side,
    /// Level index from top of book.
    pub level: u16,
    /// Price in integer ticks or price units.
    pub price: i64,
    /// Quantity/size at level.
    pub size: i64,
    /// Mutation operation.
    pub action: BookAction,
    /// Venue sequence number when available.
    pub sequence: u64,
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// One normalized price level in a materialized book snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookLevel {
    /// Level index from top of book.
    pub level: u16,
    /// Level price in integer ticks or price units.
    pub price: i64,
    /// Aggregated size at this level.
    pub size: i64,
}

/// Materialized order-book snapshot for a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookSnapshot {
    /// Snapshot symbol identity.
    pub symbol: SymbolId,
    /// Bid-side levels ordered by `level`.
    pub bids: Vec<BookLevel>,
    /// Ask-side levels ordered by `level`.
    pub asks: Vec<BookLevel>,
    /// Sequence number from the last applied book event.
    pub last_sequence: u64,
    /// Exchange timestamp from the last applied book event.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp from the last applied book event.
    pub ts_recv_ns: u64,
}

/// Last-trade print/tick.
#[derive(Debug, Clone)]
pub struct TradePrint {
    /// Symbol that traded.
    pub symbol: SymbolId,
    /// Trade price.
    pub price: i64,
    /// Trade size.
    pub size: i64,
    /// Aggressor side for the print.
    pub aggressor_side: Side,
    /// Venue sequence number when available.
    pub sequence: u64,
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// Aggregated analytics for a symbol/session.
#[derive(Debug, Clone, Default)]
pub struct AnalyticsSnapshot {
    /// Session delta (buy minus sell).
    pub delta: i64,
    /// Cumulative delta across session.
    pub cumulative_delta: i64,
    /// Total buy-side volume.
    pub buy_volume: i64,
    /// Total sell-side volume.
    pub sell_volume: i64,
    /// Last traded price.
    pub last_price: i64,
    /// Point of control (highest volume price).
    pub point_of_control: i64,
    /// Lower bound of value area.
    pub value_area_low: i64,
    /// Upper bound of value area.
    pub value_area_high: i64,
}

/// Additive derived analytics computed from the current session accumulator state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DerivedAnalyticsSnapshot {
    /// Session total volume (`buy_volume + sell_volume`).
    pub total_volume: i64,
    /// Number of trades observed in the current analytics session.
    pub trade_count: u64,
    /// Session volume-weighted average price in integer price units.
    pub vwap: i64,
    /// Mean trade size for the current analytics session.
    pub average_trade_size: i64,
    /// Directional imbalance expressed in basis points of total volume.
    pub imbalance_bps: i64,
}

/// Session candle-style summary derived from the current analytics session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionCandleSnapshot {
    /// First trade price observed in the current analytics session.
    pub open: i64,
    /// Highest trade price observed in the current analytics session.
    pub high: i64,
    /// Lowest trade price observed in the current analytics session.
    pub low: i64,
    /// Most recent trade price observed in the current analytics session.
    pub close: i64,
    /// Number of trades included in the current candle/session view.
    pub trade_count: u64,
    /// Exchange timestamp of the first trade in the current session candle.
    pub first_ts_exchange_ns: u64,
    /// Exchange timestamp of the latest trade in the current session candle.
    pub last_ts_exchange_ns: u64,
}

/// Rolling interval candle-style summary derived from recent session trades.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntervalCandleSnapshot {
    /// Width of the rolling interval represented by this snapshot.
    pub window_ns: u64,
    /// First trade price included in the interval.
    pub open: i64,
    /// Highest trade price included in the interval.
    pub high: i64,
    /// Lowest trade price included in the interval.
    pub low: i64,
    /// Latest trade price included in the interval.
    pub close: i64,
    /// Number of trades included in the interval.
    pub trade_count: u64,
    /// Total traded volume in the interval.
    pub total_volume: i64,
    /// Interval volume-weighted average price in integer price units.
    pub vwap: i64,
    /// Exchange timestamp of the first trade in the interval.
    pub first_ts_exchange_ns: u64,
    /// Exchange timestamp of the latest trade in the interval.
    pub last_ts_exchange_ns: u64,
}

/// A completed fixed-interval OHLCV bar.
#[derive(Debug, Clone, PartialEq)]
pub struct CompletedBar {
    /// Bar timestamp (start of interval) in nanoseconds.
    pub timestamp_ns: i64,
    /// Open price in integer price units.
    pub open: i64,
    /// High price in integer price units.
    pub high: i64,
    /// Low price in integer price units.
    pub low: i64,
    /// Close price in integer price units.
    pub close: i64,
    /// Total volume traded in the interval.
    pub volume: i64,
    /// Number of ticks in the interval.
    pub tick_count: u64,
    /// Volume-weighted average price.
    pub vwap: i64,
}

/// Snapshot of book-derived analytics computed from an order book snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BookAnalyticsSnapshot {
    /// Best bid price.
    pub best_bid: i64,
    /// Best ask price.
    pub best_ask: i64,
    /// Quoted spread (`best_ask - best_bid`) in price units.
    pub quoted_spread: i64,
    /// Relative spread in basis points of the mid-price.
    pub relative_spread_bps: i64,
    /// Microprice: volume-weighted price using inside bid/ask depth.
    pub microprice: i64,
    /// Total bid-side volume across all levels.
    pub bid_depth: i64,
    /// Total ask-side volume across all levels.
    pub ask_depth: i64,
    /// Depth imbalance in basis points (`(bid - ask) / (bid + ask) * 10000`).
    /// Positive values indicate bid-heavy imbalance; negative indicate ask-heavy.
    pub depth_imbalance_bps: i64,
}

/// Computes book-derived analytics from a materialized order book snapshot.
///
/// Returns a [`BookAnalyticsSnapshot`] with spread, depth, imbalance, and
/// microprice metrics. When the book has no bids or asks, the relevant fields
/// are set to zero.
pub fn compute_book_analytics(snapshot: &BookSnapshot) -> BookAnalyticsSnapshot {
    let best_bid = snapshot.bids.first().map(|l| l.price).unwrap_or(0);
    let best_ask = snapshot.asks.first().map(|l| l.price).unwrap_or(0);
    let quoted_spread = if best_bid > 0 && best_ask > 0 {
        best_ask.saturating_sub(best_bid)
    } else {
        0
    };
    let mid = if best_bid > 0 && best_ask > 0 {
        (best_bid.saturating_add(best_ask)) / 2
    } else {
        0
    };
    let relative_spread_bps = if mid > 0 {
        (quoted_spread.saturating_mul(10_000)) / mid
    } else {
        0
    };

    let bid_vol_0 = snapshot.bids.first().map(|l| l.size).unwrap_or(0);
    let ask_vol_0 = snapshot.asks.first().map(|l| l.size).unwrap_or(0);
    let microprice = if bid_vol_0 > 0 && ask_vol_0 > 0 && best_bid > 0 && best_ask > 0 {
        (best_bid.saturating_mul(ask_vol_0) + best_ask.saturating_mul(bid_vol_0))
            / (bid_vol_0 + ask_vol_0)
    } else if best_bid > 0 && best_ask > 0 {
        (best_bid + best_ask) / 2
    } else {
        0
    };

    let bid_depth: i64 = snapshot.bids.iter().map(|l| l.size).sum();
    let ask_depth: i64 = snapshot.asks.iter().map(|l| l.size).sum();
    let depth_imbalance_bps = if bid_depth.saturating_add(ask_depth) > 0 {
        (bid_depth.saturating_sub(ask_depth).saturating_mul(10_000))
            / bid_depth.saturating_add(ask_depth)
    } else {
        0
    };

    BookAnalyticsSnapshot {
        best_bid,
        best_ask,
        quoted_spread,
        relative_spread_bps,
        microprice,
        bid_depth,
        ask_depth,
        depth_imbalance_bps,
    }
}

/// Output state emitted by signal modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalState {
    /// No directional bias.
    Neutral,
    /// Long/buy bias.
    LongBias,
    /// Short/sell bias.
    ShortBias,
    /// Blocked due to data-quality gating.
    Blocked,
}

/// Snapshot of a signal module evaluation.
#[derive(Debug, Clone)]
pub struct SignalSnapshot {
    /// Stable signal module identifier.
    pub module_id: &'static str,
    /// Current state.
    pub state: SignalState,
    /// Confidence in basis points.
    pub confidence_bps: u16,
    /// Active quality flags bits.
    pub quality_flags: u32,
    /// Human-readable reason for current state.
    pub reason: String,
}

/// Bitset wrapper for feed-quality flags.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataQualityFlags(u32);

impl DataQualityFlags {
    /// No quality issues detected.
    pub const NONE: Self = Self(0);
    /// Feed is stale beyond policy threshold.
    pub const STALE_FEED: Self = Self(1 << 0);
    /// A sequence number gap was detected.
    pub const SEQUENCE_GAP: Self = Self(1 << 1);
    /// Clock skew detected between source and consumer.
    pub const CLOCK_SKEW: Self = Self(1 << 2);
    /// Book depth was truncated.
    pub const DEPTH_TRUNCATED: Self = Self(1 << 3);
    /// Event arrived out-of-order.
    pub const OUT_OF_ORDER: Self = Self(1 << 4);
    /// Adapter/external feed is degraded or reconnecting.
    pub const ADAPTER_DEGRADED: Self = Self(1 << 5);

    /// Returns raw bit representation.
    pub fn bits(self) -> u32 {
        self.0
    }

    /// Builds flags from raw bits, preserving unknown bits.
    pub fn from_bits_truncate(bits: u32) -> Self {
        Self(bits)
    }

    /// Returns true when any flag in `other` is set in `self`.
    pub fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

impl fmt::Debug for DataQualityFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataQualityFlags({:#x})", self.0)
    }
}

impl BitOr for DataQualityFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// In-memory accumulator that updates analytics state from normalized trades.
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

impl Default for AnalyticsAccumulator {
    fn default() -> Self {
        Self {
            snapshot: AnalyticsSnapshot::default(),
            volume_profile: HashMap::new(),
            session_trade_count: 0,
            session_turnover: 0,
            session_candle: SessionCandleSnapshot::default(),
            session_trades: Vec::new(),
            #[cfg(feature = "tickbar")]
            tick_aggregator: None,
            #[cfg(feature = "tickbar")]
            tick_interval_ns: 0,
        }
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
        let agg = tickbar::TickAggregator::builder()
            .interval(std::time::Duration::from_nanos(interval_ns as u64))
            .build()
            .expect("TickAggregator build should not fail with valid interval");
        acc.tick_aggregator = Some(agg);
        acc.tick_interval_ns = interval_ns;
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

        let new_agg = tickbar::TickAggregator::builder()
            .interval(std::time::Duration::from_nanos(interval_ns as u64))
            .build()
            .expect("TickAggregator rebuild should not fail");
        self.tick_aggregator = Some(new_agg);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol() -> SymbolId {
        SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        }
    }

    #[test]
    fn tracks_delta_and_cumulative_delta() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 100,
            size: 5,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 99,
            size: 2,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });

        let snap = acc.snapshot();
        assert_eq!(snap.delta, 3);
        assert_eq!(snap.cumulative_delta, 3);
        assert_eq!(snap.buy_volume, 5);
        assert_eq!(snap.sell_volume, 2);
        assert_eq!(snap.last_price, 99);
        assert_eq!(snap.point_of_control, 100);
        assert_eq!(snap.value_area_low, 100);
        assert_eq!(snap.value_area_high, 100);

        acc.reset_session_delta();
        let reset = acc.snapshot();
        assert_eq!(reset.delta, 0);
        assert_eq!(reset.buy_volume, 0);
        assert_eq!(reset.sell_volume, 0);
        assert_eq!(reset.cumulative_delta, 3);
    }

    #[test]
    fn tracks_poc_and_value_area() {
        let mut acc = AnalyticsAccumulator::default();
        let s = symbol();
        let prints = [
            (100, 5, Side::Ask),
            (101, 7, Side::Ask),
            (99, 3, Side::Bid),
            (102, 2, Side::Ask),
            (101, 5, Side::Bid),
        ];
        for (i, (price, size, side)) in prints.iter().enumerate() {
            acc.on_trade(&TradePrint {
                symbol: s.clone(),
                price: *price,
                size: *size,
                aggressor_side: *side,
                sequence: i as u64 + 1,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            });
        }
        let snap = acc.snapshot();
        assert_eq!(snap.point_of_control, 101);
        assert!(snap.value_area_low <= snap.point_of_control);
        assert!(snap.value_area_high >= snap.point_of_control);
    }

    #[test]
    fn computes_derived_session_metrics() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 100,
            size: 5,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 98,
            size: 3,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });

        let derived = acc.derived_snapshot();
        assert_eq!(derived.total_volume, 8);
        assert_eq!(derived.trade_count, 2);
        assert_eq!(derived.vwap, 99);
        assert_eq!(derived.average_trade_size, 4);
        assert_eq!(derived.imbalance_bps, 2500);

        acc.reset_session_delta();
        let reset = acc.derived_snapshot();
        assert_eq!(reset.total_volume, 0);
        assert_eq!(reset.trade_count, 0);
        assert_eq!(reset.vwap, 0);
    }

    #[test]
    fn computes_session_candle_snapshot() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 100,
            size: 5,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 10,
            ts_recv_ns: 11,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 98,
            size: 3,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 20,
            ts_recv_ns: 21,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 101,
            size: 2,
            aggressor_side: Side::Ask,
            sequence: 3,
            ts_exchange_ns: 30,
            ts_recv_ns: 31,
        });

        let candle = acc.session_candle_snapshot();
        assert_eq!(candle.open, 100);
        assert_eq!(candle.high, 101);
        assert_eq!(candle.low, 98);
        assert_eq!(candle.close, 101);
        assert_eq!(candle.trade_count, 3);
        assert_eq!(candle.first_ts_exchange_ns, 10);
        assert_eq!(candle.last_ts_exchange_ns, 30);

        acc.reset_session_delta();
        let reset = acc.session_candle_snapshot();
        assert_eq!(reset, SessionCandleSnapshot::default());
    }

    #[test]
    fn computes_interval_candle_snapshot() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 100,
            size: 5,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 10,
            ts_recv_ns: 11,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 98,
            size: 3,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 40,
            ts_recv_ns: 41,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 101,
            size: 2,
            aggressor_side: Side::Ask,
            sequence: 3,
            ts_exchange_ns: 100,
            ts_recv_ns: 101,
        });

        let recent = acc.interval_candle_snapshot(70);
        assert_eq!(recent.window_ns, 70);
        assert_eq!(recent.open, 98);
        assert_eq!(recent.high, 101);
        assert_eq!(recent.low, 98);
        assert_eq!(recent.close, 101);
        assert_eq!(recent.trade_count, 2);
        assert_eq!(recent.total_volume, 5);
        assert_eq!(recent.vwap, 99);
        assert_eq!(recent.first_ts_exchange_ns, 40);
        assert_eq!(recent.last_ts_exchange_ns, 100);

        acc.reset_session_delta();
        let reset = acc.interval_candle_snapshot(70);
        assert_eq!(
            reset,
            IntervalCandleSnapshot {
                window_ns: 70,
                ..IntervalCandleSnapshot::default()
            }
        );
    }

    #[test]
    fn full_session_reset_clears_profile_and_cumulative() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 101,
            size: 4,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });
        acc.reset_session();
        let snap = acc.snapshot();
        assert_eq!(snap.delta, 0);
        assert_eq!(snap.cumulative_delta, 0);
        assert_eq!(snap.buy_volume, 0);
        assert_eq!(snap.sell_volume, 0);
        assert_eq!(snap.point_of_control, 0);
        assert_eq!(snap.value_area_low, 0);
        assert_eq!(snap.value_area_high, 0);
    }

    #[cfg(feature = "tickbar")]
    #[test]
    fn tickbar_aggregates_bars_from_trades() {
        let mut acc = AnalyticsAccumulator::with_tickbar(1000);
        let s = symbol();

        acc.on_trade(&TradePrint {
            symbol: s.clone(),
            price: 505000,
            size: 9,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        });
        acc.on_trade(&TradePrint {
            symbol: s.clone(),
            price: 504900,
            size: 4,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 500,
            ts_recv_ns: 501,
        });
        acc.on_trade(&TradePrint {
            symbol: s.clone(),
            price: 505100,
            size: 8,
            aggressor_side: Side::Ask,
            sequence: 3,
            ts_exchange_ns: 1500,
            ts_recv_ns: 1501,
        });

        let bars = acc.bar_series().expect("should have bars");
        assert_eq!(bars.len(), 2, "expected 2 bars, got {}", bars.len());

        // First bar: trades at 0 and 500 ns → interval [0, 1000)
        assert_eq!(bars[0].timestamp_ns, 0);
        assert_eq!(bars[0].open, 505000);
        assert_eq!(bars[0].high, 505000);
        assert_eq!(bars[0].low, 504900);
        assert_eq!(bars[0].close, 504900);
        assert_eq!(bars[0].volume, 13);
        assert_eq!(bars[0].tick_count, 2);

        // Second bar: trade at 1500 ns → interval [1000, 2000)
        assert_eq!(bars[1].timestamp_ns, 1000);
        assert_eq!(bars[1].open, 505100);
        assert_eq!(bars[1].high, 505100);
        assert_eq!(bars[1].low, 505100);
        assert_eq!(bars[1].close, 505100);
        assert_eq!(bars[1].volume, 8);
        assert_eq!(bars[1].tick_count, 1);
    }

    #[cfg(feature = "tickbar")]
    #[test]
    fn tickbar_default_accumulator_returns_none() {
        let mut acc = AnalyticsAccumulator::default();
        let s = symbol();
        acc.on_trade(&TradePrint {
            symbol: s,
            price: 505000,
            size: 9,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        });
        assert!(acc.bar_series().is_none());
    }

    #[cfg(feature = "tickbar")]
    #[test]
    fn tickbar_reset_removes_aggregator() {
        let mut acc = AnalyticsAccumulator::with_tickbar(1000);
        let s = symbol();
        acc.on_trade(&TradePrint {
            symbol: s,
            price: 505000,
            size: 9,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        });
        assert!(acc.bar_series().is_some());

        // After bar_series() the aggregator is rebuilt internally, but reset_tickbar removes it fully
        acc.reset_tickbar();
        let s2 = symbol();
        acc.on_trade(&TradePrint {
            symbol: s2,
            price: 505000,
            size: 9,
            aggressor_side: Side::Ask,
            sequence: 2,
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        });
        assert!(acc.bar_series().is_none());
    }

    #[test]
    fn compute_book_analytics_returns_spread_and_depth_metrics() {
        let snapshot = BookSnapshot {
            symbol: symbol(),
            bids: vec![
                BookLevel {
                    level: 0,
                    price: 100,
                    size: 10,
                },
                BookLevel {
                    level: 1,
                    price: 99,
                    size: 5,
                },
            ],
            asks: vec![
                BookLevel {
                    level: 0,
                    price: 102,
                    size: 8,
                },
                BookLevel {
                    level: 1,
                    price: 103,
                    size: 3,
                },
            ],
            last_sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };

        let analytics = compute_book_analytics(&snapshot);
        assert_eq!(analytics.best_bid, 100);
        assert_eq!(analytics.best_ask, 102);
        assert_eq!(analytics.quoted_spread, 2);
        assert!(analytics.relative_spread_bps > 0);
        assert!(analytics.microprice > 0);
        assert_eq!(analytics.bid_depth, 15);
        assert_eq!(analytics.ask_depth, 11);
        assert!(analytics.depth_imbalance_bps > 0);
    }

    #[test]
    fn compute_book_analytics_empty_book_returns_defaults() {
        let snapshot = BookSnapshot {
            symbol: symbol(),
            bids: vec![],
            asks: vec![],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };

        let analytics = compute_book_analytics(&snapshot);
        assert_eq!(analytics, BookAnalyticsSnapshot::default());
    }

    #[test]
    fn book_snapshot_keeps_level_order() {
        let snapshot = BookSnapshot {
            symbol: symbol(),
            bids: vec![
                BookLevel {
                    level: 0,
                    price: 100,
                    size: 5,
                },
                BookLevel {
                    level: 2,
                    price: 98,
                    size: 3,
                },
            ],
            asks: vec![BookLevel {
                level: 1,
                price: 101,
                size: 4,
            }],
            last_sequence: 7,
            ts_exchange_ns: 11,
            ts_recv_ns: 12,
        };

        assert_eq!(snapshot.bids[0].level, 0);
        assert_eq!(snapshot.bids[1].level, 2);
        assert_eq!(snapshot.asks[0].level, 1);
        assert_eq!(snapshot.last_sequence, 7);
    }
}
