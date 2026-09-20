use super::*;

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

/// Execution-quality benchmark context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionBenchmark {
    arrival_midpoint: i64,
    decision_price: i64,
    best_bid: i64,
    best_ask: i64,
    future_midpoint: Option<i64>,
}

impl ExecutionBenchmark {
    /// Creates execution-quality benchmark context.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidExecution`] when prices are
    /// non-positive or the book is crossed/locked.
    pub const fn new(
        arrival_midpoint: i64,
        decision_price: i64,
        best_bid: i64,
        best_ask: i64,
        future_midpoint: Option<i64>,
    ) -> Result<Self, AnalyticsError> {
        if arrival_midpoint <= 0
            || decision_price <= 0
            || best_bid <= 0
            || best_ask <= 0
            || best_bid >= best_ask
        {
            return Err(AnalyticsError::InvalidExecution);
        }
        if let Some(future_midpoint) = future_midpoint {
            if future_midpoint <= 0 {
                return Err(AnalyticsError::InvalidExecution);
            }
        }
        Ok(Self {
            arrival_midpoint,
            decision_price,
            best_bid,
            best_ask,
            future_midpoint,
        })
    }

    /// Returns arrival midpoint.
    pub const fn arrival_midpoint(&self) -> i64 {
        self.arrival_midpoint
    }

    /// Returns decision price.
    pub const fn decision_price(&self) -> i64 {
        self.decision_price
    }

    /// Returns best bid.
    pub const fn best_bid(&self) -> i64 {
        self.best_bid
    }

    /// Returns best ask.
    pub const fn best_ask(&self) -> i64 {
        self.best_ask
    }

    /// Returns optional future midpoint.
    pub const fn future_midpoint(&self) -> Option<i64> {
        self.future_midpoint
    }
}

/// Execution-quality/TCA snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionQualitySnapshot {
    implementation_shortfall_bps: i32,
    arrival_slippage_bps: i32,
    decision_slippage_bps: i32,
    adverse_selection_bps: i32,
    trade_through: bool,
    fill_quality_score_bps: u16,
}

impl ExecutionQualitySnapshot {
    /// Returns implementation shortfall versus decision price.
    pub const fn implementation_shortfall_bps(&self) -> i32 {
        self.implementation_shortfall_bps
    }

    /// Returns slippage versus arrival midpoint.
    pub const fn arrival_slippage_bps(&self) -> i32 {
        self.arrival_slippage_bps
    }

    /// Returns slippage versus decision price.
    pub const fn decision_slippage_bps(&self) -> i32 {
        self.decision_slippage_bps
    }

    /// Returns future-midpoint adverse selection, or zero when unavailable.
    pub const fn adverse_selection_bps(&self) -> i32 {
        self.adverse_selection_bps
    }

    /// Returns true when the fill traded through same-side touch.
    pub const fn trade_through(&self) -> bool {
        self.trade_through
    }

    /// Returns fill-quality score in basis points, where 10,000 is best.
    pub const fn fill_quality_score_bps(&self) -> u16 {
        self.fill_quality_score_bps
    }
}

/// Execution-quality/TCA analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionQualityAnalyzer;

impl ExecutionQualityAnalyzer {
    /// Evaluates one fill against execution benchmarks.
    pub fn evaluate(
        trade: TradeContext,
        benchmark: ExecutionBenchmark,
    ) -> ExecutionQualitySnapshot {
        let arrival_slippage_bps = side_aware_slippage_bps(trade, benchmark.arrival_midpoint());
        let decision_slippage_bps = side_aware_slippage_bps(trade, benchmark.decision_price());
        let adverse_selection_bps = benchmark
            .future_midpoint()
            .map(|future_mid| side_aware_slippage_bps(trade, future_mid))
            .unwrap_or(0);
        let trade_through = match trade.aggressor_side() {
            Side::Ask => trade.price() > benchmark.best_ask(),
            Side::Bid => trade.price() < benchmark.best_bid(),
        };
        let penalty = arrival_slippage_bps
            .max(0)
            .saturating_add(decision_slippage_bps.max(0))
            .saturating_add(adverse_selection_bps.max(0));
        let score = 10_000_i32.saturating_sub(penalty.min(10_000));
        ExecutionQualitySnapshot {
            implementation_shortfall_bps: decision_slippage_bps,
            arrival_slippage_bps,
            decision_slippage_bps,
            adverse_selection_bps,
            trade_through,
            fill_quality_score_bps: u16::try_from(score).unwrap_or(0),
        }
    }
}
