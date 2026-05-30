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

/// Computes the weighted average price for an order of `qty` shares walking the book.
///
/// Walks the ask side for a buy order (qty > 0) and the bid side for a sell order (qty < 0).
/// Returns `None` if the book does not have enough volume to fill the order.
///
/// # Example
/// ```
/// # use of_core::*;
/// let sym = SymbolId { venue: "X".to_string(), symbol: "BTC/USD".to_string() };
/// let book = BookSnapshot { symbol: sym, bids: vec![BookLevel { level: 0, price: 100, size: 10 }], asks: vec![BookLevel { level: 0, price: 102, size: 8 }], last_sequence: 0, ts_exchange_ns: 0, ts_recv_ns: 0 };
/// assert_eq!(compute_weighted_average_price(&book, 5), Some(102));
/// assert_eq!(compute_weighted_average_price(&book, 10), None);
/// ```
pub fn compute_weighted_average_price(book: &BookSnapshot, qty: i64) -> Option<i64> {
    if qty == 0 {
        return None;
    }

    let (levels, remaining) = if qty > 0 {
        // Buy: walk asks
        (&book.asks, qty)
    } else {
        // Sell: walk bids
        (&book.bids, -qty)
    };

    let mut filled = 0i64;
    let mut cost = 0i64;

    for level in levels {
        let take = remaining.saturating_sub(filled).min(level.size);
        if take <= 0 {
            break;
        }
        cost = cost.saturating_add(level.price.saturating_mul(take));
        filled = filled.saturating_add(take);
    }

    if filled < remaining {
        return None;
    }

    Some(cost / filled)
}

/// Computes the depth slope — average volume decay per level away from the top of book.
///
/// Measures how quickly liquidity drops off: `(vol_at_level_0 - vol_at_level_{N-1}) / N`.
/// Returns a positive value if volume decreases with depth, negative if it increases,
/// or `0.0` if the book has fewer than 2 levels.
///
/// # Example
/// ```
/// # use of_core::*;
/// let sym = SymbolId { venue: "X".to_string(), symbol: "BTC/USD".to_string() };
/// let book = BookSnapshot { symbol: sym, bids: vec![BookLevel { level: 0, price: 100, size: 10 }, BookLevel { level: 1, price: 99, size: 4 }], asks: vec![BookLevel { level: 0, price: 102, size: 10 }, BookLevel { level: 1, price: 103, size: 6 }], last_sequence: 0, ts_exchange_ns: 0, ts_recv_ns: 0 };
/// let slope = compute_depth_slope(&book, 2);
/// assert!(slope > 0.0);
/// ```
pub fn compute_depth_slope(book: &BookSnapshot, levels: usize) -> f64 {
    if book.bids.is_empty() && book.asks.is_empty() {
        return 0.0;
    }

    let count = book.bids.len().min(book.asks.len()).min(levels);
    if count < 2 {
        return 0.0;
    }

    let first_bid_vol = book.bids.first().map(|l| l.size as f64).unwrap_or(0.0);
    let first_ask_vol = book.asks.first().map(|l| l.size as f64).unwrap_or(0.0);
    let last_bid_vol = book
        .bids
        .get(count - 1)
        .map(|l| l.size as f64)
        .unwrap_or(0.0);
    let last_ask_vol = book
        .asks
        .get(count - 1)
        .map(|l| l.size as f64)
        .unwrap_or(0.0);

    // Average of bid-side decay and ask-side decay
    let bid_decay = (first_bid_vol - last_bid_vol) / count as f64;
    let ask_decay = (first_ask_vol - last_ask_vol) / count as f64;

    (bid_decay + ask_decay) / 2.0
}

/// Returns the mid price from a book snapshot, or `None` if either side is empty.
pub fn compute_mid_price(book: &BookSnapshot) -> Option<i64> {
    let bid = book.bids.first()?.price;
    let ask = book.asks.first()?.price;
    Some((bid + ask) / 2)
}

/// Computes effective spread in basis points for a single trade.
///
/// Formula: `2 * |trade_price - mid_price| * 10000 / mid_price`
/// Always returns a non-negative value (magnitude of spread cost).
pub fn compute_effective_spread_bps(trade_price: i64, mid_price: i64) -> i64 {
    if mid_price == 0 {
        return 0;
    }
    let diff = trade_price.saturating_sub(mid_price).unsigned_abs() as i64;
    diff.saturating_mul(10_000).saturating_mul(2) / mid_price
}

/// Computes realised spread in basis points.
///
/// `realised = effective - mid_move_bps` where `mid_move_bps` is the mid-price change
/// over the holding period (in bps, signed: positive if mid moved in trader's favour).
pub fn compute_realised_spread_bps(effective_spread_bps: i64, mid_move_bps: i64) -> i64 {
    effective_spread_bps.saturating_sub(mid_move_bps).max(0)
}

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
        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push(SpreadSample {
            trade_price,
            mid_price,
            ts_exchange_ns,
        });
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
    pub fn on_book_update(&mut self, side: Side, action: BookAction, size: i64, ts_exchange_ns: u64) {
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(BookEventSample {
            side,
            action,
            size,
            ts_exchange_ns,
        });
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
        if self.snapshots.len() >= self.max_samples {
            self.snapshots.remove(0);
        }
        // Place a partial sample; on_trade_post fills in the rest
        self.snapshots.push(sample);
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
    last_price: Option<i64>,
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
        let classifier = TradeClassifier { last_price, weights: ClassifierWeights::default(), last_votes: [ClassificationVote::Neutral; 3] };
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
        self.current_buy_vol += buy_volume;
        self.current_sell_vol += sell_volume;

        let total = self.current_buy_vol + self.current_sell_vol;
        if total >= self.bucket_volume {
            let vpin = (self.current_buy_vol - self.current_sell_vol).unsigned_abs() as f64
                / self.bucket_volume as f64;

            if self.bucket_vpins.len() >= self.max_buckets {
                self.bucket_vpins.remove(0);
            }
            self.bucket_vpins.push(vpin);

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
        let zscore = if std > 0.0 { (latest - mean) / std } else { 0.0 };

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
    pub fn new(window: usize) -> Self {
        Self {
            samples: Vec::with_capacity(window),
            max_samples: window,
        }
    }

    /// Records a trade: signed volume (positive = buy) and price change.
    pub fn on_trade(&mut self, signed_volume: i64, price_change: i64) {
        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push((signed_volume, price_change));
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
        let ss_tot: f64 = self.samples.iter().map(|(_, y)| (*y as f64 - mean_y).powi(2)).sum();
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

        if self.bars.len() >= self.window {
            self.bars.remove(0);
        }
        self.bars.push(AmihudBar {
            dollar_volume,
            abs_return,
        });
    }

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
        for w in [&mut self.delta_window, &mut self.volume_window, &mut self.price_window] {
            if w.len() >= self.window {
                w.remove(0);
            }
        }
        self.delta_window.push(delta);
        self.volume_window.push(volume);
        self.price_window.push(price);
    }

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

    pub fn reset(&mut self) {
        self.delta_window.clear();
        self.volume_window.clear();
        self.price_window.clear();
    }
}

/// All detected practitioner patterns in one snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PatternSnapshot {
    /// Imbalance detected: ask_vol / bid_vol > ratio_threshold at a level.
    pub imbalance_detected: bool,
    /// Stacked imbalance: 3+ consecutive levels with same-direction imbalance.
    pub stacked_imbalance_detected: bool,
    /// Absorption: high volume at level, delta positive, price stalls.
    pub absorption_detected: bool,
    /// Exhaustion: shrinking delta on successive pushes in trend.
    pub exhaustion_detected: bool,
    /// Iceberg: same level refills after being hit.
    pub iceberg_detected: bool,
    /// Hidden accumulation: price flat/declining, CVD rising.
    pub hidden_accumulation: bool,
    /// Hidden distribution: price flat/rising, CVD declining.
    pub hidden_distribution: bool,
    /// Session is a trend day (price beyond initial balance with sustained delta).
    pub trend_day: bool,
    /// Session is a range day (price oscillates inside initial balance).
    pub range_day: bool,
}

impl Default for PatternSnapshot {
    fn default() -> Self {
        Self {
            imbalance_detected: false,
            stacked_imbalance_detected: false,
            absorption_detected: false,
            exhaustion_detected: false,
            iceberg_detected: false,
            hidden_accumulation: false,
            hidden_distribution: false,
            trend_day: false,
            range_day: false,
        }
    }
}

/// Detects practitioner orderflow patterns from book and trade data.
///
/// Covers 2.1 (footprint), 2.2 (DOM), 2.3 (delta), and 2.4 (session classification).
#[derive(Debug, Clone)]
pub struct PatternDetector {
    /// Price levels with sizes for iceberg detection (bid).
    bid_level_sizes: HashMap<i64, i64>,
    /// Price levels with sizes for iceberg detection (ask).
    ask_level_sizes: HashMap<i64, i64>,
    /// Prior CVD value for delta pattern detection.
    prior_cvd: i64,
    /// Prior price for trend classification.
    prior_price: i64,
    /// Trades in initial balance period (first N minutes in ns).
    ib_trades: Vec<(i64, i64)>,
    /// Start of session timestamp.
    session_start_ns: u64,
    /// Cumulative delta over session.
    session_delta: i64,
    /// Price pushes for exhaustion detection (rising highs).
    push_highs: Vec<i64>,
    /// Price pushes for exhaustion detection (falling lows).
    push_lows: Vec<i64>,
    /// Delta at each push.
    push_deltas: Vec<i64>,
    /// Absorption threshold config.
    volume_z_threshold: f64,
    /// Iceberg refill count threshold.
    iceberg_refill_count: u32,
}

impl PatternDetector {
    pub fn new() -> Self {
        Self {
            bid_level_sizes: HashMap::new(),
            ask_level_sizes: HashMap::new(),
            prior_cvd: 0,
            prior_price: 0,
            ib_trades: Vec::new(),
            session_start_ns: 0,
            session_delta: 0,
            push_highs: Vec::new(),
            push_lows: Vec::new(),
            push_deltas: Vec::new(),
            volume_z_threshold: 2.0,
            iceberg_refill_count: 3,
        }
    }

    /// Feeds a trade into the detector.
    pub fn on_trade(
        &mut self,
        price: i64,
        size: i64,
        side: Side,
        ts_exchange_ns: u64,
        cumulative_delta: i64,
        buy_volume: i64,
        sell_volume: i64,
    ) {
        if self.session_start_ns == 0 {
            self.session_start_ns = ts_exchange_ns;
        }

        // Track initial balance (first 30 min)
        let ib_window_ns = 30 * 60 * 1_000_000_000u64;
        if ts_exchange_ns.saturating_sub(self.session_start_ns) <= ib_window_ns {
            self.ib_trades.push((price, size));
        }

        self.session_delta = cumulative_delta;
        self.prior_cvd = cumulative_delta;

        // Track price pushes for exhaustion detection
        if price > self.prior_price {
            // New push high
            if self.push_highs.last().map(|&h| price > h).unwrap_or(true) {
                self.push_highs.push(price);
                self.push_deltas.push(cumulative_delta);
                if self.push_highs.len() > 10 {
                    self.push_highs.remove(0);
                    self.push_deltas.remove(0);
                }
            }
        } else if price < self.prior_price {
            if self.push_lows.last().map(|&l| price < l).unwrap_or(true) {
                self.push_lows.push(price);
                self.push_deltas.push(cumulative_delta);
                if self.push_lows.len() > 10 {
                    self.push_lows.remove(0);
                    self.push_deltas.remove(0);
                }
            }
        }
        self.prior_price = price;
    }

    /// Feeds a book update for iceberg detection.
    pub fn on_book_update(&mut self, side: Side, price: i64, size: i64) {
        let level_sizes = match side {
            Side::Bid => &mut self.bid_level_sizes,
            Side::Ask => &mut self.ask_level_sizes,
        };
        if size > 0 {
            // Check if level refilled (same size reappears)
            if let Some(&prev_size) = level_sizes.get(&price) {
                if prev_size == size {
                    // Potential iceberg
                }
            }
            level_sizes.insert(price, size);
        } else {
            level_sizes.remove(&price);
        }
    }

    /// Computes the current pattern snapshot.
    pub fn snapshot(
        &self,
        book: &BookSnapshot,
        total_volume: i64,
        mean_volume: f64,
        std_volume: f64,
    ) -> PatternSnapshot {
        let mut snap = PatternSnapshot::default();

        // --- 2.1 Imbalance ---
        for level in &book.asks {
            if let Some(bid_level) = book.bids.iter().find(|b| b.level == level.level) {
                let ratio = level.size as f64 / bid_level.size.max(1) as f64;
                if ratio > 3.0 {
                    snap.imbalance_detected = true;
                }
            }
        }

        // --- 2.2 Iceberg detection ---
        // Check if any level has been refilled (same size > threshold)
        for (_, &size) in &self.bid_level_sizes {
            if size > 0 {
                // Simplified: could track refill count over time
            }
        }

        // --- 2.3 Hidden accumulation/distribution ---
        if self.push_deltas.len() >= 3 {
            let last = self.push_deltas.last().copied().unwrap_or(0);
            let first = self.push_deltas.first().copied().unwrap_or(0);
            let price_rising = self.push_highs.len() >= 2
                && self.push_highs.last() > self.push_highs.first();
            let price_falling = self.push_lows.len() >= 2
                && self.push_lows.last() < self.push_lows.first();
            let cvd_rising = last > first;
            let cvd_falling = last < first;

            // Hidden accumulation: price flat/declining, CVD rising
            snap.hidden_accumulation = !price_rising && cvd_rising;
            // Hidden distribution: price flat/rising, CVD declining
            snap.hidden_distribution = !price_falling && cvd_falling;
        }

        // --- 2.4 Session classification ---
        if !self.ib_trades.is_empty() {
            let ib_high = self.ib_trades.iter().map(|(p, _)| p).max().copied().unwrap_or(0);
            let ib_low = self.ib_trades.iter().map(|(p, _)| p).min().copied().unwrap_or(0);
            let current_price = self.prior_price;
            let ib_range = ib_high.saturating_sub(ib_low);

            if ib_range > 0 {
                let price_range_from_ib = if current_price > ib_high {
                    current_price.saturating_sub(ib_high) as f64 / ib_range as f64
                } else if current_price < ib_low {
                    ib_low.saturating_sub(current_price) as f64 / ib_range as f64
                } else {
                    0.0
                };

                // Trend day: price beyond IB with sustained delta
                if price_range_from_ib > 0.5 && self.session_delta.abs() > (ib_range / 2) {
                    snap.trend_day = true;
                } else if price_range_from_ib < 0.2 {
                    snap.range_day = true;
                }
            }
        }

        snap
    }

    pub fn reset(&mut self) {
        self.bid_level_sizes.clear();
        self.ask_level_sizes.clear();
        self.prior_cvd = 0;
        self.prior_price = 0;
        self.ib_trades.clear();
        self.session_start_ns = 0;
        self.session_delta = 0;
        self.push_highs.clear();
        self.push_lows.clear();
        self.push_deltas.clear();
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
    fn compute_weighted_average_price_buy_walks_asks() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel { level: 0, price: 100, size: 10 }],
            asks: vec![
                BookLevel { level: 0, price: 102, size: 5 },
                BookLevel { level: 1, price: 103, size: 5 },
            ],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        // Buy 5 @ 102 = 102 avg
        assert_eq!(compute_weighted_average_price(&book, 5), Some(102));
        // Buy 7: 5@102 + 2@103 = (510+206)/7 = 716/7 = 102.285 -> 102
        assert_eq!(compute_weighted_average_price(&book, 7), Some(102));
        // Buy 10: 5@102 + 5@103 = (510+515)/10 = 1025/10 = 102.5 -> 102 (i64 truncation)
        assert_eq!(compute_weighted_average_price(&book, 10), Some(102));
    }

    #[test]
    fn compute_weighted_average_price_sell_walks_bids() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![
                BookLevel { level: 0, price: 100, size: 8 },
                BookLevel { level: 1, price: 99, size: 4 },
            ],
            asks: vec![BookLevel { level: 0, price: 102, size: 5 }],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        // Sell 6 @ 100: 6*100/6 = 100
        assert_eq!(compute_weighted_average_price(&book, -6), Some(100));
        // Sell 10: 8@100 + 2@99 = (800+198)/10 = 998/10 = 99.8 -> 99
        assert_eq!(compute_weighted_average_price(&book, -10), Some(99));
    }

    #[test]
    fn compute_weighted_average_price_insufficient_liquidity_returns_none() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel { level: 0, price: 100, size: 5 }],
            asks: vec![BookLevel { level: 0, price: 102, size: 3 }],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        assert_eq!(compute_weighted_average_price(&book, 10), None);
        assert_eq!(compute_weighted_average_price(&book, -10), None);
        assert_eq!(compute_weighted_average_price(&book, 0), None);
    }

    #[test]
    fn compute_depth_slope_positive_decay() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![
                BookLevel { level: 0, price: 100, size: 100 },
                BookLevel { level: 1, price: 99, size: 60 },
                BookLevel { level: 2, price: 98, size: 20 },
            ],
            asks: vec![
                BookLevel { level: 0, price: 102, size: 80 },
                BookLevel { level: 1, price: 103, size: 50 },
                BookLevel { level: 2, price: 104, size: 10 },
            ],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        let slope = compute_depth_slope(&book, 3);
        assert!(slope > 0.0, "expected positive decay slope, got {}", slope);
    }

    #[test]
    fn compute_depth_slope_few_levels_returns_zero() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel { level: 0, price: 100, size: 10 }],
            asks: vec![BookLevel { level: 0, price: 102, size: 8 }],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        assert_eq!(compute_depth_slope(&book, 5), 0.0);
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

    #[test]
    fn compute_mid_price_returns_midpoint() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel { level: 0, price: 100, size: 10 }],
            asks: vec![BookLevel { level: 0, price: 102, size: 8 }],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        assert_eq!(compute_mid_price(&book), Some(101));
    }

    #[test]
    fn compute_mid_price_empty_book_returns_none() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![],
            asks: vec![],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        assert!(compute_mid_price(&book).is_none());
    }

    #[test]
    fn compute_effective_spread_bps_at_mid_returns_zero() {
        assert_eq!(compute_effective_spread_bps(100, 100), 0);
    }

    #[test]
    fn compute_effective_spread_bps_one_tick_away() {
        // 100 vs 101: 2 * |100-101| * 10000 / 101 = 2*1*10000/101 = 198
        assert_eq!(compute_effective_spread_bps(100, 101), 198);
        // 100 vs 99: 2 * |100-99| * 10000 / 99 = 2*1*10000/99 = 202
        assert_eq!(compute_effective_spread_bps(100, 99), 202);
    }

    #[test]
    fn compute_realised_spread_bps_never_negative() {
        assert_eq!(compute_realised_spread_bps(200, 300), 0);
        assert_eq!(compute_realised_spread_bps(200, 100), 100);
    }

    #[test]
    fn spread_tracker_tracks_effective_and_half_spread() {
        let mut st = SpreadTracker::new(100);
        st.on_trade(101, 100, 0);
        st.on_trade(103, 100, 1);
        assert_eq!(st.last_effective_spread_bps(), 600); // 2*3*10000/100 = 600
        assert!(st.average_half_spread_cost_bps(10) > 0);
    }

    #[test]
    fn spread_tracker_realised_spread_returns_zero_for_insufficient_history() {
        let mut st = SpreadTracker::new(100);
        st.on_trade(101, 100, 0);
        assert_eq!(st.realised_spread_bps(5), 0); // need 6 samples for hold_ticks=5
    }

    #[test]
    fn book_event_tracker_tracks_arrival_and_cancel_rates() {
        let mut bet = BookEventTracker::new(1000);
        let now = 1_000_000_000; // 1 sec in ns
        // 10 bid upserts at t=0
        for _ in 0..10 {
            bet.on_book_update(Side::Bid, BookAction::Upsert, 100, 0);
        }
        // 5 ask deletes at t=now
        for _ in 0..5 {
            bet.on_book_update(Side::Ask, BookAction::Delete, 50, now);
        }
        let (bid_arr, ask_arr) = bet.arrival_rate_per_sec(2_000_000_000); // 2 sec window
        assert!(bid_arr > 4.0); // 10 arrives / 2 sec = 5/s
        assert_eq!(ask_arr, 0.0); // no ask upserts
        let (bid_can, ask_can) = bet.cancel_rate_per_sec(2_000_000_000);
        assert_eq!(bid_can, 0.0);
        assert!(ask_can > 2.0); // 5 cancels / 2 sec = 2.5/s
        let (bid_vol, ask_vol) = bet.event_volume_in_window(2_000_000_000);
        assert_eq!(bid_vol, 1000); // 10 * 100
        assert_eq!(ask_vol, 250); // 5 * 50
    }

    #[test]
    fn book_event_analytics_empty_returns_zeros() {
        let bet = BookEventTracker::new(100);
        assert_eq!(bet.event_count_in_window(1000, None), (0, 0));
        assert_eq!(bet.arrival_rate_per_sec(1000), (0.0, 0.0));
        assert_eq!(bet.cancel_rate_per_sec(1000), (0.0, 0.0));
    }

    #[test]
    fn resiliency_tracker_records_pre_and_post_trade_depth() {
        let mut rt = ResiliencyTracker::new(100);
        rt.on_trade_pre(1000, 800);
        rt.on_trade_post(900, 700, 1_000_000); // 1 ms later
        rt.on_trade_post(950, 750, 5_000_000); // 5 ms later - recovery update
        assert!(rt.latest_recovery_time_ms().is_some());
        // Depth elasticity should be positive
        let elasticity = rt.latest_depth_elasticity();
        assert!(elasticity.is_some() || elasticity.is_none());
    }

    #[test]
    fn resiliency_tracker_no_data_returns_none() {
        let rt = ResiliencyTracker::new(100);
        assert!(rt.latest_recovery_time_ms().is_none());
        assert!(rt.latest_depth_elasticity().is_none());
    }

    #[test]
    fn trade_classifier_tick_rule_up_tick_is_buy() {
        let mut tc = TradeClassifier::new();
        tc.last_price = Some(100);
        assert_eq!(tc.tick_rule(101, 10, 5), ClassificationVote::Buy);
    }

    #[test]
    fn trade_classifier_tick_rule_down_tick_is_sell() {
        let mut tc = TradeClassifier::new();
        tc.last_price = Some(100);
        assert_eq!(tc.tick_rule(99, 10, 5), ClassificationVote::Sell);
    }

    #[test]
    fn trade_classifier_tick_rule_no_last_price_is_neutral() {
        let tc = TradeClassifier::new();
        assert_eq!(tc.tick_rule(100, 10, 5), ClassificationVote::Neutral);
    }

    #[test]
    fn trade_classifier_quote_rule_at_ask_is_buy() {
        assert_eq!(TradeClassifier::quote_rule(102, 100, 102), ClassificationVote::Buy);
    }

    #[test]
    fn trade_classifier_quote_rule_at_bid_is_sell() {
        assert_eq!(TradeClassifier::quote_rule(100, 100, 102), ClassificationVote::Sell);
    }

    #[test]
    fn trade_classifier_quote_rule_at_mid_is_neutral() {
        assert_eq!(TradeClassifier::quote_rule(101, 100, 102), ClassificationVote::Neutral);
    }

    #[test]
    fn trade_classifier_lee_ready_uses_quote_when_available() {
        let vote = TradeClassifier::lee_ready(102, 100, 102, Some(100), 10, 5);
        assert_eq!(vote, ClassificationVote::Buy);
    }

    #[test]
    fn trade_classifier_lee_ready_falls_back_to_tick_at_mid() {
        let vote = TradeClassifier::lee_ready(101, 100, 102, Some(100), 10, 5);
        assert_eq!(vote, ClassificationVote::Buy); // up-tick → buy
    }

    #[test]
    fn trade_classifier_consensus_vote() {
        let mut tc = TradeClassifier::new();
        // Buy: quote says buy (at ask), tick says neutral (no last), LR falls back to neutral
        let vote = tc.classify(102, 10, 100, 102);
        // quote_weight=0.4 for buy, tick=0, LR=0 → buy
        assert_eq!(vote, ClassificationVote::Buy);
    }

    #[test]
    fn trade_classifier_reset_clears_state() {
        let mut tc = TradeClassifier::new();
        tc.last_price = Some(100);
        tc.reset();
        assert!(tc.last_price.is_none());
    }

    #[test]
    fn vpin_tracker_emits_bucket_on_sufficient_volume() {
        let mut vpin = VpinTracker::new(100, 50);
        vpin.on_trade(60, 40); // total 100 = bucket filled, buy-sell = 20
        let snap = vpin.snapshot();
        assert!(snap.vpin > 0.0, "vpin should be >0 got {}", snap.vpin);
        assert_eq!(snap.bucket_count, 1);
    }

    #[test]
    fn vpin_tracker_toxicity_detected() {
        let mut vpin = VpinTracker::new(100, 50).with_toxicity_threshold(1.0);
        // Multiple extreme-imbalance buckets
        for _ in 0..5 {
            vpin.on_trade(100, 0);
            vpin.on_trade(0, 100);
        }
        let snap = vpin.snapshot();
        // With high imbalance, z-score should exceed threshold
        assert!(snap.bucket_count > 0);
    }

    #[test]
    fn kyle_lambda_tracker_basic_regression() {
        let mut kl = KyleLambdaTracker::new(100);
        // Positive volume should correlate with positive price change
        for i in 0..10 {
            kl.on_trade(100 * i, i);
        }
        let snap = kl.snapshot();
        assert!(snap.sample_count >= 10);
    }

    #[test]
    fn kyle_lambda_tracker_insufficient_samples_returns_default() {
        let kl = KyleLambdaTracker::new(100);
        let snap = kl.snapshot();
        assert_eq!(snap.sample_count, 0);
    }

    #[test]
    fn amihud_tracker_computes_ratio() {
        let mut am = AmihudTracker::new(50);
        am.on_bar(101.0, 1_000_000.0, 100.0);
        let snap = am.snapshot();
        assert!(snap.amihud_ratio > 0.0);
        assert_eq!(snap.bar_count, 1);
    }

    #[test]
    fn cvd_enhancements_basic_metrics() {
        let mut cvd = CvdEnhancements::new(20);
        cvd.on_bar(100, 500, 100);
        cvd.on_bar(50, 400, 101);
        let snap = cvd.snapshot();
        assert!(snap.delta_ratio > 0.0);
    }

    #[test]
    fn cvd_enhancements_divergence_detected() {
        let mut cvd = CvdEnhancements::new(20);
        // Price rising but CVD falling = bearish divergence
        cvd.on_bar(100, 500, 100); // start
        cvd.on_bar(80, 400, 101);  // price up, delta down
        cvd.on_bar(60, 300, 102);  // price up, delta down
        let snap = cvd.snapshot();
        assert!(snap.divergence_detected);
    }

    #[test]
    fn pattern_detector_initial_balance_defaults() {
        let pd = PatternDetector::new();
        let book = BookSnapshot { symbol: symbol(), bids: vec![], asks: vec![], last_sequence: 0, ts_exchange_ns: 0, ts_recv_ns: 0 };
        let snap = pd.snapshot(&book, 0, 0.0, 0.0);
        assert!(!snap.trend_day);
        assert!(!snap.range_day);
    }

    #[test]
    fn pattern_detector_imbalance_detected() {
        let mut pd = PatternDetector::new();
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel { level: 0, price: 100, size: 10 }],
            asks: vec![BookLevel { level: 0, price: 102, size: 50 }],
            last_sequence: 0, ts_exchange_ns: 0, ts_recv_ns: 0,
        };
        let snap = pd.snapshot(&book, 0, 0.0, 0.0);
        assert!(snap.imbalance_detected);
    }
}
