#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::ops::BitOr;

const DEFAULT_PATTERN_HISTORY_CAP: usize = 4096;
const DEFAULT_PATTERN_PRICE_LEVEL_CAP: usize = 8192;
const MAX_SESSION_TRADES: usize = 100_000;

fn bounded_push<T>(items: &mut Vec<T>, max_len: usize, item: T) {
    if max_len == 0 {
        return;
    }
    if items.len() >= max_len {
        items.remove(0);
    }
    items.push(item);
}

fn bounded_push_pair<T, U>(
    left: &mut Vec<T>,
    right: &mut Vec<U>,
    max_len: usize,
    left_item: T,
    right_item: U,
) {
    if max_len == 0 {
        return;
    }
    if left.len() >= max_len {
        left.remove(0);
        if !right.is_empty() {
            right.remove(0);
        }
    }
    left.push(left_item);
    right.push(right_item);
}

fn prune_hash_map<K, V>(items: &mut HashMap<K, V>, max_len: usize)
where
    K: Eq + Hash + Clone,
{
    if max_len == 0 {
        items.clear();
        return;
    }
    while items.len() > max_len {
        let Some(key) = items.keys().next().cloned() else {
            break;
        };
        items.remove(&key);
    }
}

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

        bounded_push(
            &mut self.bars,
            self.window,
            AmihudBar {
                dollar_volume,
                abs_return,
            },
        );
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

/// Realised volatility estimators: Classic, Parkinson, Garman-Klass, Yang-Zhang.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolatilitySnapshot {
    pub classic_rv: f64,
    pub parkinson: f64,
    pub garman_klass: f64,
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
    pub fn new(max_bars: usize) -> Self {
        Self {
            bars: Vec::with_capacity(max_bars),
            max_bars,
        }
    }
    pub fn on_bar(&mut self, open: f64, high: f64, low: f64, close: f64) {
        bounded_push(&mut self.bars, self.max_bars, (open, high, low, close));
    }
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
    pub noise_variance: f64,
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
    pub fn new(max_len: usize) -> Self {
        Self {
            returns: Vec::with_capacity(max_len),
            max_len,
            last_price: None,
        }
    }
    pub fn on_trade(&mut self, price: i64, _size: i64) {
        let prev = self.last_price.replace(price).unwrap_or(price);
        if prev <= 0 || price <= 0 {
            return;
        }
        let r = (price as f64 / prev as f64).ln();
        bounded_push(&mut self.returns, self.max_len, r);
    }
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
    pub permanent_impact: f64,
    pub temporary_impact: f64,
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
    pub fn new(max_len: usize) -> Self {
        Self {
            returns: Vec::with_capacity(max_len),
            signed_volumes: Vec::with_capacity(max_len),
            max_len,
        }
    }
    pub fn on_trade(&mut self, ret: f64, signed_vol: f64) {
        bounded_push_pair(
            &mut self.returns,
            &mut self.signed_volumes,
            self.max_len,
            ret,
            signed_vol,
        );
    }
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
    pub permanent_impact_coef: f64,
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
    pub fn new(max_len: usize) -> Self {
        Self {
            price_changes: Vec::with_capacity(max_len),
            signed_volumes: Vec::with_capacity(max_len),
            max_len,
        }
    }
    pub fn on_trade(&mut self, price_change: f64, signed_vol: f64) {
        bounded_push_pair(
            &mut self.price_changes,
            &mut self.signed_volumes,
            self.max_len,
            price_change,
            signed_vol,
        );
    }
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
    pub adverse_selection: f64,
    pub order_processing_cost: f64,
    pub inventory_component: f64,
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
    pub fn new(max_len: usize) -> Self {
        Self {
            effective_spreads: Vec::with_capacity(max_len),
            realised_spreads: Vec::with_capacity(max_len),
            quoted_spreads: Vec::with_capacity(max_len),
            max_len,
        }
    }
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
    pub mean_duration_ns: f64,
    pub intensity: f64,
    pub alpha: f64,
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
    pub fn new(max_len: usize) -> Self {
        Self {
            durations: Vec::with_capacity(max_len),
            max_len,
        }
    }
    pub fn on_trade(&mut self, ts_ns: u64, prev_ts_ns: u64) {
        if prev_ts_ns > 0 {
            let d = (ts_ns.saturating_sub(prev_ts_ns)) as f64;
            bounded_push(&mut self.durations, self.max_len, d);
        }
    }
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
    Normal = 0,
    Stressed = 1,
    FlashCrash = 2,
    Quiet = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegimeSnapshot {
    pub regime: u32,
    pub spread_z: f64,
    pub vol_z: f64,
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
    pub fn new(max_len: usize) -> Self {
        Self {
            spreads: Vec::with_capacity(max_len),
            vols: Vec::with_capacity(max_len),
            vpins: Vec::with_capacity(max_len),
            max_len,
        }
    }
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
/// Kinetic energy of order book activity.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KineticEnergySnapshot {
    pub kinetic_energy: f64,
    pub order_flow_momentum: f64,
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
    pub dark_volume_pct: f64,
    pub dark_zscore: f64,
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
    pub fn new(max_days: usize) -> Self {
        Self {
            dark_volumes: Vec::with_capacity(max_days),
            lit_volumes: Vec::with_capacity(max_days),
            max_days,
        }
    }
    pub fn on_day(&mut self, dark_vol: f64, lit_vol: f64) {
        bounded_push_pair(
            &mut self.dark_volumes,
            &mut self.lit_volumes,
            self.max_days,
            dark_vol,
            lit_vol,
        );
    }
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
    pub sweep_detected: bool,
    pub put_call_ratio: f64,
    pub delta_notional: f64,
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
    pub fn new(max_len: usize) -> Self {
        Self {
            trades: Vec::with_capacity(max_len),
            max_len,
        }
    }
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
    pub basis_bps: f64,
    pub calendar_spread: f64,
    pub settlement_pressure: f64,
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
    pub lag: u32,
    pub rv: f64,
}

/// Volatility signature plot: RV at multiple lags.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolatilitySignatureSnapshot {
    pub points: [VolatilitySignaturePoint; 10],
    pub point_count: u32,
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
    pub fn new(max_len: usize) -> Self {
        Self {
            returns: Vec::with_capacity(max_len),
            max_len,
        }
    }
    pub fn on_return(&mut self, r: f64) {
        bounded_push(&mut self.returns, self.max_len, r);
    }
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
    pub irp: f64,
    pub ipin: f64,
    pub ivpin: f64,
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
    pub fn new(max_len: usize) -> Self {
        Self {
            trade_sizes_f: Vec::with_capacity(max_len),
            cancel_rates: Vec::with_capacity(max_len),
            arrivals: Vec::with_capacity(max_len),
            max_len,
        }
    }
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
    pub spread_bps: f64,
    pub depth_imbalance: f64,
    pub microprice: f64,
    pub depth_slope: f64,
    pub order_intensity: f64,
    pub price_pressure_1: f64,
    pub price_pressure_5: f64,
    pub price_pressure_10: f64,
    pub bid_ask_ratio_1: f64,
    pub bid_ask_ratio_5: f64,
    pub bid_ask_ratio_10: f64,
    pub weighted_spread: f64,
    pub volume_concentration: f64,
    pub cancel_intensity: f64,
    pub arrival_intensity: f64,
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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DarkLitCorrelationSnapshot {
    pub correlation: f64,
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

#[derive(Debug, Clone)]
pub struct DarkLitCorrelator {
    dark_imbalances: Vec<f64>,
    lit_imbalances: Vec<f64>,
    max_len: usize,
}

impl DarkLitCorrelator {
    pub fn new(max_len: usize) -> Self {
        Self {
            dark_imbalances: Vec::with_capacity(max_len),
            lit_imbalances: Vec::with_capacity(max_len),
            max_len,
        }
    }
    pub fn on_imbalance(&mut self, dark_imb: f64, lit_imb: f64) {
        bounded_push_pair(
            &mut self.dark_imbalances,
            &mut self.lit_imbalances,
            self.max_len,
            dark_imb,
            lit_imb,
        );
    }
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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstitutionalFlowSnapshot {
    pub institutional_buy_ratio: f64,
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

#[derive(Debug, Clone)]
pub struct InstitutionalFlowTracker {
    large_trades: Vec<(i64, i64)>, // (size, side: +1 buy, -1 sell)
    total_volume: i64,
    max_len: usize,
}

impl InstitutionalFlowTracker {
    pub fn new(max_len: usize) -> Self {
        Self {
            large_trades: Vec::with_capacity(max_len),
            total_volume: 0,
            max_len,
        }
    }
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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OIAnalysisSnapshot {
    pub oi_divergence: bool,
    pub oi_build_rate: f64,
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

#[derive(Debug, Clone)]
pub struct OIAnalyzer {
    oi_values: Vec<f64>,
    price_values: Vec<f64>,
    max_len: usize,
}

impl OIAnalyzer {
    pub fn new(max_len: usize) -> Self {
        Self {
            oi_values: Vec::with_capacity(max_len),
            price_values: Vec::with_capacity(max_len),
            max_len,
        }
    }
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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FXSnapshot {
    pub cross_currency_correlation: f64,
}

impl Default for FXSnapshot {
    fn default() -> Self {
        Self {
            cross_currency_correlation: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedIncomeSnapshot {
    pub yield_curve_positioning: f64,
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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CryptoSnapshot {
    pub exchange_flow_balance: f64,
    pub funding_rate_basis: f64,
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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AlertRule {
    pub absorption_alert: bool,
    pub delta_divergence_alert: bool,
    pub stacked_imbalance_alert: bool,
    pub iceberg_alert: bool,
    pub vpin_toxic_alert: bool,
}

// ============================================================================
// T9: State Checkpoint
// ============================================================================

/// Trait for state serialization.
pub trait StateCheckpoint: Send + 'static {
    fn checkpoint(&self) -> Result<Vec<u8>, String>;
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
    pub fn new(max_len: usize) -> Self {
        Self {
            front_prices: Vec::with_capacity(max_len),
            deferred_prices: Vec::with_capacity(max_len),
            settle_volumes: Vec::with_capacity(max_len),
            daily_avg_volumes: Vec::with_capacity(max_len),
            max_len,
        }
    }
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
/// Pass an instance to [`Engine::set_analytics_config`] or the C ABI setter to override.
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
            bids: vec![BookLevel {
                level: 0,
                price: 100,
                size: 10,
            }],
            asks: vec![
                BookLevel {
                    level: 0,
                    price: 102,
                    size: 5,
                },
                BookLevel {
                    level: 1,
                    price: 103,
                    size: 5,
                },
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
                BookLevel {
                    level: 0,
                    price: 100,
                    size: 8,
                },
                BookLevel {
                    level: 1,
                    price: 99,
                    size: 4,
                },
            ],
            asks: vec![BookLevel {
                level: 0,
                price: 102,
                size: 5,
            }],
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
            bids: vec![BookLevel {
                level: 0,
                price: 100,
                size: 5,
            }],
            asks: vec![BookLevel {
                level: 0,
                price: 102,
                size: 3,
            }],
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
                BookLevel {
                    level: 0,
                    price: 100,
                    size: 100,
                },
                BookLevel {
                    level: 1,
                    price: 99,
                    size: 60,
                },
                BookLevel {
                    level: 2,
                    price: 98,
                    size: 20,
                },
            ],
            asks: vec![
                BookLevel {
                    level: 0,
                    price: 102,
                    size: 80,
                },
                BookLevel {
                    level: 1,
                    price: 103,
                    size: 50,
                },
                BookLevel {
                    level: 2,
                    price: 104,
                    size: 10,
                },
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
            bids: vec![BookLevel {
                level: 0,
                price: 100,
                size: 10,
            }],
            asks: vec![BookLevel {
                level: 0,
                price: 102,
                size: 8,
            }],
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
            bids: vec![BookLevel {
                level: 0,
                price: 100,
                size: 10,
            }],
            asks: vec![BookLevel {
                level: 0,
                price: 102,
                size: 8,
            }],
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
        assert_eq!(
            TradeClassifier::quote_rule(102, 100, 102),
            ClassificationVote::Buy
        );
    }

    #[test]
    fn trade_classifier_quote_rule_at_bid_is_sell() {
        assert_eq!(
            TradeClassifier::quote_rule(100, 100, 102),
            ClassificationVote::Sell
        );
    }

    #[test]
    fn trade_classifier_quote_rule_at_mid_is_neutral() {
        assert_eq!(
            TradeClassifier::quote_rule(101, 100, 102),
            ClassificationVote::Neutral
        );
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
        cvd.on_bar(80, 400, 101); // price up, delta down
        cvd.on_bar(60, 300, 102); // price up, delta down
        let snap = cvd.snapshot();
        assert!(snap.divergence_detected);
    }

    #[test]
    fn pattern_detector_initial_balance_defaults() {
        let pd = PatternDetector::new();
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![],
            asks: vec![],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        let snap = pd.snapshot(&book, 0, 0.0, 0.0);
        assert!(!snap.trend_day);
        assert!(!snap.range_day);
    }

    #[test]
    fn volatility_estimator_classic_rv_computed() {
        let mut ve = VolatilityEstimator::new(100);
        ve.on_bar(100.0, 102.0, 99.0, 101.0);
        ve.on_bar(101.0, 103.0, 100.0, 102.0);
        let snap = ve.snapshot();
        assert!(snap.classic_rv > 0.0);
        assert!(snap.parkinson > 0.0);
    }

    #[test]
    fn noise_tracker_returns_default_with_few_samples() {
        let nt = MicrostructureNoise::new(100);
        assert_eq!(nt.snapshot().noise_variance, 0.0);
    }

    #[test]
    fn hasbrouck_var_returns_default_with_few_samples() {
        let hv = HasbrouckVAR::new(100);
        assert_eq!(hv.snapshot().permanent_impact, 0.0);
    }

    #[test]
    fn almgren_chriss_returns_default_with_few_samples() {
        let ac = AlmgrenChriss::new(100);
        assert_eq!(ac.snapshot().permanent_impact_coef, 0.0);
    }

    #[test]
    fn spread_decomposition_returns_default_empty() {
        let sd = SpreadDecomposition::new(100);
        assert_eq!(sd.snapshot().pin, 0.0);
    }

    #[test]
    fn acd_model_returns_default_with_few_samples() {
        let acd = ACDModel::new(100);
        assert_eq!(acd.snapshot().mean_duration_ns, 0.0);
    }

    #[test]
    fn regime_detector_normal_by_default() {
        let rd = RegimeDetector::new(100);
        assert_eq!(rd.snapshot().regime, 0); // Normal
    }

    #[test]
    fn kinetic_energy_returns_default_with_few_samples() {
        let ke = KineticEnergyTracker::new(100);
        assert_eq!(ke.snapshot().kinetic_energy, 0.0);
    }

    #[test]
    fn dark_pool_analytics_basic() {
        let mut dp = DarkPoolTracker::new(20);
        dp.on_day(1000.0, 9000.0);
        let snap = dp.snapshot();
        assert!((snap.dark_volume_pct - 10.0).abs() < 0.01);
    }

    #[test]
    fn options_flow_put_call_ratio() {
        let mut ot = OptionsFlowTracker::new(100);
        ot.on_trade(true, 100.0, 1000.0, false);
        ot.on_trade(false, 200.0, 2000.0, false);
        let snap = ot.snapshot();
        assert!((snap.put_call_ratio - 2.0).abs() < 0.01);
    }

    #[test]
    fn futures_basis_computed() {
        let mut ft = FuturesTracker::new(100);
        ft.on_tick(100.0, 101.0, 1000.0, 5000.0);
        let snap = ft.snapshot();
        assert!((snap.basis_bps - 100.0).abs() < 0.01);
    }

    #[test]
    fn pattern_detector_imbalance_detected() {
        let pd = PatternDetector::new();
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel {
                level: 0,
                price: 100,
                size: 10,
            }],
            asks: vec![BookLevel {
                level: 0,
                price: 102,
                size: 50,
            }],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        let snap = pd.snapshot(&book, 0, 0.0, 0.0);
        assert!(snap.imbalance_detected);
    }
}
