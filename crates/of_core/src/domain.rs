use super::*;

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
