//! Advanced market microstructure analytics for Orderflow.
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::error::Error;
use std::fmt;

use of_core::{BookLevel, Side};

/// Advanced analytics error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalyticsError {
    /// Quote prices, sizes, or timestamps are invalid.
    InvalidQuote,
    /// Trade price, size, side, or timestamp is invalid.
    InvalidTrade,
    /// Depth configuration or book levels are invalid.
    InvalidDepth,
    /// Requested analytics require a quote but no quote is available.
    MissingQuote,
}

impl fmt::Display for AnalyticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuote => write!(f, "invalid quote context"),
            Self::InvalidTrade => write!(f, "invalid trade context"),
            Self::InvalidDepth => write!(f, "invalid depth context"),
            Self::MissingQuote => write!(f, "missing quote context"),
        }
    }
}

impl Error for AnalyticsError {}

/// Best bid/ask context used by market-quality analytics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuoteContext {
    bid_price: i64,
    ask_price: i64,
    bid_qty: i64,
    ask_qty: i64,
    ts_ns: u64,
}

impl QuoteContext {
    /// Creates quote context.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQuote`] when prices are crossed,
    /// non-positive, quantities are negative, or timestamp is zero.
    pub const fn new(
        bid_price: i64,
        ask_price: i64,
        bid_qty: i64,
        ask_qty: i64,
        ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if bid_price <= 0
            || ask_price <= 0
            || bid_price >= ask_price
            || bid_qty < 0
            || ask_qty < 0
            || ts_ns == 0
        {
            return Err(AnalyticsError::InvalidQuote);
        }
        Ok(Self {
            bid_price,
            ask_price,
            bid_qty,
            ask_qty,
            ts_ns,
        })
    }

    /// Returns best bid price.
    pub const fn bid_price(&self) -> i64 {
        self.bid_price
    }

    /// Returns best ask price.
    pub const fn ask_price(&self) -> i64 {
        self.ask_price
    }

    /// Returns best bid quantity.
    pub const fn bid_qty(&self) -> i64 {
        self.bid_qty
    }

    /// Returns best ask quantity.
    pub const fn ask_qty(&self) -> i64 {
        self.ask_qty
    }

    /// Returns quote timestamp.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }

    /// Returns midpoint price using integer arithmetic.
    pub const fn midpoint(&self) -> i64 {
        self.bid_price + (self.ask_price - self.bid_price) / 2
    }

    /// Returns quoted spread.
    pub const fn quoted_spread(&self) -> i64 {
        self.ask_price - self.bid_price
    }
}

/// Trade context aligned to a quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeContext {
    price: i64,
    qty: i64,
    aggressor_side: Side,
    ts_ns: u64,
}

impl TradeContext {
    /// Creates trade context.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when price, quantity, or
    /// timestamp is invalid.
    pub const fn new(
        price: i64,
        qty: i64,
        aggressor_side: Side,
        ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if price <= 0 || qty <= 0 || ts_ns == 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            price,
            qty,
            aggressor_side,
            ts_ns,
        })
    }

    /// Returns trade price.
    pub const fn price(&self) -> i64 {
        self.price
    }

    /// Returns trade quantity.
    pub const fn qty(&self) -> i64 {
        self.qty
    }

    /// Returns aggressive trade side.
    pub const fn aggressor_side(&self) -> Side {
        self.aggressor_side
    }

    /// Returns trade timestamp.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }
}

/// Market-quality and transaction-cost snapshot for one trade/quote pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketQualitySnapshot {
    quoted_spread: i64,
    quoted_spread_bps: i32,
    effective_spread_bps: i32,
    realized_spread_bps: i32,
    price_improvement_bps: i32,
    stale_quote: bool,
}

impl MarketQualitySnapshot {
    /// Returns quoted spread in integer price units.
    pub const fn quoted_spread(&self) -> i64 {
        self.quoted_spread
    }

    /// Returns quoted spread in basis points of midpoint.
    pub const fn quoted_spread_bps(&self) -> i32 {
        self.quoted_spread_bps
    }

    /// Returns effective spread in basis points.
    pub const fn effective_spread_bps(&self) -> i32 {
        self.effective_spread_bps
    }

    /// Returns realized spread in basis points, or zero when no future
    /// midpoint was provided.
    pub const fn realized_spread_bps(&self) -> i32 {
        self.realized_spread_bps
    }

    /// Returns price improvement in basis points versus same-side touch.
    pub const fn price_improvement_bps(&self) -> i32 {
        self.price_improvement_bps
    }

    /// Returns true when trade/quote age exceeded the configured freshness
    /// window.
    pub const fn stale_quote(&self) -> bool {
        self.stale_quote
    }
}

/// Market-quality tracker retaining the latest quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketQualityTracker {
    max_quote_age_ns: u64,
    last_quote: Option<QuoteContext>,
}

impl MarketQualityTracker {
    /// Creates a market-quality tracker.
    pub const fn new(max_quote_age_ns: u64) -> Self {
        Self {
            max_quote_age_ns,
            last_quote: None,
        }
    }

    /// Returns configured quote freshness window.
    pub const fn max_quote_age_ns(&self) -> u64 {
        self.max_quote_age_ns
    }

    /// Records the latest quote.
    pub fn on_quote(&mut self, quote: QuoteContext) {
        self.last_quote = Some(quote);
    }

    /// Returns latest quote.
    pub const fn last_quote(&self) -> Option<QuoteContext> {
        self.last_quote
    }

    /// Evaluates one trade against the latest quote.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::MissingQuote`] when no quote is available.
    pub fn evaluate_trade(
        &self,
        trade: TradeContext,
        future_midpoint: Option<i64>,
    ) -> Result<MarketQualitySnapshot, AnalyticsError> {
        let quote = self.last_quote.ok_or(AnalyticsError::MissingQuote)?;
        let midpoint = quote.midpoint();
        let stale_quote = trade.ts_ns().saturating_sub(quote.ts_ns()) > self.max_quote_age_ns;
        let signed_distance = match trade.aggressor_side() {
            Side::Ask => trade.price().saturating_sub(midpoint),
            Side::Bid => midpoint.saturating_sub(trade.price()),
        };
        let effective_spread_bps = price_to_bps(signed_distance.saturating_mul(2), midpoint);
        let realized_spread_bps = future_midpoint
            .map(|future_mid| {
                let realized_distance = match trade.aggressor_side() {
                    Side::Ask => trade.price().saturating_sub(future_mid),
                    Side::Bid => future_mid.saturating_sub(trade.price()),
                };
                price_to_bps(realized_distance.saturating_mul(2), midpoint)
            })
            .unwrap_or(0);
        let same_side_touch = match trade.aggressor_side() {
            Side::Ask => quote.ask_price(),
            Side::Bid => quote.bid_price(),
        };
        let price_improvement = match trade.aggressor_side() {
            Side::Ask => same_side_touch.saturating_sub(trade.price()),
            Side::Bid => trade.price().saturating_sub(same_side_touch),
        };
        Ok(MarketQualitySnapshot {
            quoted_spread: quote.quoted_spread(),
            quoted_spread_bps: price_to_bps(quote.quoted_spread(), midpoint),
            effective_spread_bps,
            realized_spread_bps,
            price_improvement_bps: price_to_bps(price_improvement, midpoint),
            stale_quote,
        })
    }
}

/// Liquidity/depth snapshot over borrowed book levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityDepthSnapshot {
    levels_used: usize,
    top_bid_qty: i64,
    top_ask_qty: i64,
    bid_depth: i64,
    ask_depth: i64,
    proportional_imbalance_bps: i32,
    depth_slope_bps: i32,
    sweepable_buy_qty: i64,
    sweepable_sell_qty: i64,
}

impl LiquidityDepthSnapshot {
    /// Returns number of levels included per side.
    pub const fn levels_used(&self) -> usize {
        self.levels_used
    }

    /// Returns top bid quantity.
    pub const fn top_bid_qty(&self) -> i64 {
        self.top_bid_qty
    }

    /// Returns top ask quantity.
    pub const fn top_ask_qty(&self) -> i64 {
        self.top_ask_qty
    }

    /// Returns cumulative bid depth.
    pub const fn bid_depth(&self) -> i64 {
        self.bid_depth
    }

    /// Returns cumulative ask depth.
    pub const fn ask_depth(&self) -> i64 {
        self.ask_depth
    }

    /// Returns bid-minus-ask depth imbalance in basis points.
    pub const fn proportional_imbalance_bps(&self) -> i32 {
        self.proportional_imbalance_bps
    }

    /// Returns simple depth slope proxy in basis points.
    pub const fn depth_slope_bps(&self) -> i32 {
        self.depth_slope_bps
    }

    /// Returns ask-side quantity sweepable by a buy order up to target
    /// quantity.
    pub const fn sweepable_buy_qty(&self) -> i64 {
        self.sweepable_buy_qty
    }

    /// Returns bid-side quantity sweepable by a sell order up to target
    /// quantity.
    pub const fn sweepable_sell_qty(&self) -> i64 {
        self.sweepable_sell_qty
    }
}

/// Borrowed depth analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityDepthAnalyzer {
    levels: usize,
}

impl LiquidityDepthAnalyzer {
    /// Creates a depth analyzer.
    pub const fn new(levels: usize) -> Self {
        Self { levels }
    }

    /// Returns configured level count.
    pub const fn levels(&self) -> usize {
        self.levels
    }

    /// Analyzes borrowed bid/ask levels.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDepth`] when level count is zero,
    /// levels are missing, or a level has a non-positive price or negative
    /// size.
    pub fn analyze(
        &self,
        bids: &[BookLevel],
        asks: &[BookLevel],
        target_qty: i64,
    ) -> Result<LiquidityDepthSnapshot, AnalyticsError> {
        if self.levels == 0 || bids.is_empty() || asks.is_empty() || target_qty < 0 {
            return Err(AnalyticsError::InvalidDepth);
        }
        validate_levels(bids)?;
        validate_levels(asks)?;
        let bid_depth = depth_sum(bids, self.levels);
        let ask_depth = depth_sum(asks, self.levels);
        let total_depth = bid_depth.saturating_add(ask_depth);
        let proportional_imbalance_bps = if total_depth <= 0 {
            0
        } else {
            i32::try_from(
                (i128::from(bid_depth.saturating_sub(ask_depth)) * 10_000)
                    / i128::from(total_depth),
            )
            .unwrap_or(0)
        };
        Ok(LiquidityDepthSnapshot {
            levels_used: self.levels.min(bids.len()).min(asks.len()),
            top_bid_qty: bids[0].size,
            top_ask_qty: asks[0].size,
            bid_depth,
            ask_depth,
            proportional_imbalance_bps,
            depth_slope_bps: depth_slope_bps(bids, asks, self.levels),
            sweepable_buy_qty: sweep_qty(asks, target_qty),
            sweepable_sell_qty: sweep_qty(bids, target_qty),
        })
    }
}

fn validate_levels(levels: &[BookLevel]) -> Result<(), AnalyticsError> {
    for level in levels {
        if level.price <= 0 || level.size < 0 {
            return Err(AnalyticsError::InvalidDepth);
        }
    }
    Ok(())
}

fn depth_sum(levels: &[BookLevel], max_levels: usize) -> i64 {
    levels
        .iter()
        .take(max_levels)
        .fold(0_i64, |sum, level| sum.saturating_add(level.size))
}

fn sweep_qty(levels: &[BookLevel], target_qty: i64) -> i64 {
    let mut remaining = target_qty;
    let mut swept = 0_i64;
    for level in levels {
        if remaining <= 0 {
            break;
        }
        let take = level.size.min(remaining);
        swept = swept.saturating_add(take);
        remaining = remaining.saturating_sub(take);
    }
    swept
}

fn depth_slope_bps(bids: &[BookLevel], asks: &[BookLevel], max_levels: usize) -> i32 {
    let used = max_levels.min(bids.len()).min(asks.len());
    if used <= 1 {
        return 0;
    }
    let top = bids[0].size.saturating_add(asks[0].size);
    let outer = bids[used - 1].size.saturating_add(asks[used - 1].size);
    if top <= 0 {
        return 0;
    }
    i32::try_from((i128::from(outer.saturating_sub(top)) * 10_000) / i128::from(top)).unwrap_or(0)
}

fn price_to_bps(value: i64, reference: i64) -> i32 {
    if reference <= 0 {
        return 0;
    }
    let bps = (i128::from(value) * 10_000) / i128::from(reference);
    i32::try_from(bps.clamp(i128::from(i32::MIN), i128::from(i32::MAX))).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_quality_computes_spreads_and_improvement() {
        let mut tracker = MarketQualityTracker::new(1_000);
        tracker.on_quote(QuoteContext::new(499_000, 501_000, 100, 120, 1).expect("quote"));

        let snapshot = tracker
            .evaluate_trade(
                TradeContext::new(500_000, 10, Side::Ask, 500).expect("trade"),
                Some(500_010),
            )
            .expect("snapshot");

        assert_eq!(snapshot.quoted_spread(), 2_000);
        assert_eq!(snapshot.quoted_spread_bps(), 40);
        assert_eq!(snapshot.effective_spread_bps(), 0);
        assert!(snapshot.price_improvement_bps() > 0);
        assert!(!snapshot.stale_quote());
    }

    #[test]
    fn market_quality_flags_stale_quote() {
        let mut tracker = MarketQualityTracker::new(10);
        tracker.on_quote(QuoteContext::new(499_975, 500_025, 100, 120, 1).expect("quote"));

        let snapshot = tracker
            .evaluate_trade(
                TradeContext::new(500_025, 10, Side::Ask, 100).expect("trade"),
                None,
            )
            .expect("snapshot");

        assert!(snapshot.stale_quote());
    }

    #[test]
    fn liquidity_depth_uses_borrowed_levels() {
        let bids = [
            BookLevel {
                level: 0,
                price: 499_975,
                size: 100,
            },
            BookLevel {
                level: 1,
                price: 499_950,
                size: 80,
            },
        ];
        let asks = [
            BookLevel {
                level: 0,
                price: 500_025,
                size: 120,
            },
            BookLevel {
                level: 1,
                price: 500_050,
                size: 90,
            },
        ];

        let snapshot = LiquidityDepthAnalyzer::new(2)
            .analyze(&bids, &asks, 150)
            .expect("snapshot");

        assert_eq!(snapshot.levels_used(), 2);
        assert_eq!(snapshot.bid_depth(), 180);
        assert_eq!(snapshot.ask_depth(), 210);
        assert_eq!(snapshot.sweepable_buy_qty(), 150);
        assert_eq!(snapshot.sweepable_sell_qty(), 150);
        assert!(snapshot.proportional_imbalance_bps() < 0);
    }

    #[test]
    fn liquidity_depth_rejects_invalid_levels() {
        let bids = [BookLevel {
            level: 0,
            price: 0,
            size: 100,
        }];
        let asks = [BookLevel {
            level: 0,
            price: 500_025,
            size: 100,
        }];

        assert_eq!(
            LiquidityDepthAnalyzer::new(1).analyze(&bids, &asks, 10),
            Err(AnalyticsError::InvalidDepth)
        );
    }
}
