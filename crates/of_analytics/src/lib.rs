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
    /// Feed-quality event fields are invalid.
    InvalidQuality,
    /// Feature schema, feature id, or vector index is invalid.
    InvalidFeature,
    /// Resiliency configuration or sample fields are invalid.
    InvalidResiliency,
    /// Queue/fill probability configuration or event fields are invalid.
    InvalidQueue,
    /// Pattern-risk configuration or input fields are invalid.
    InvalidPattern,
    /// Venue/route analytics configuration or event fields are invalid.
    InvalidRoute,
    /// Cross-asset analytics configuration or sample fields are invalid.
    InvalidCrossAsset,
    /// Derivatives analytics configuration or sample fields are invalid.
    InvalidDerivative,
    /// Execution-quality benchmark fields are invalid.
    InvalidExecution,
    /// Requested analytics require a quote but no quote is available.
    MissingQuote,
}

impl fmt::Display for AnalyticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuote => write!(f, "invalid quote context"),
            Self::InvalidTrade => write!(f, "invalid trade context"),
            Self::InvalidDepth => write!(f, "invalid depth context"),
            Self::InvalidQuality => write!(f, "invalid feed quality context"),
            Self::InvalidFeature => write!(f, "invalid feature vector context"),
            Self::InvalidResiliency => write!(f, "invalid resiliency context"),
            Self::InvalidQueue => write!(f, "invalid queue/fill context"),
            Self::InvalidPattern => write!(f, "invalid pattern risk context"),
            Self::InvalidRoute => write!(f, "invalid venue/route context"),
            Self::InvalidCrossAsset => write!(f, "invalid cross-asset context"),
            Self::InvalidDerivative => write!(f, "invalid derivatives context"),
            Self::InvalidExecution => write!(f, "invalid execution quality context"),
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
    depth_convexity_bps: i32,
    book_pressure_bps: i32,
    sweepable_buy_qty: i64,
    sweepable_sell_qty: i64,
    buy_sweepability_bps: u16,
    sell_sweepability_bps: u16,
    sweepability_score_bps: u16,
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

    /// Returns aggregate second-difference depth curvature in basis points.
    pub const fn depth_convexity_bps(&self) -> i32 {
        self.depth_convexity_bps
    }

    /// Returns distance-weighted bid-minus-ask book pressure in basis points.
    pub const fn book_pressure_bps(&self) -> i32 {
        self.book_pressure_bps
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

    /// Returns buy-side target sweepability in basis points.
    pub const fn buy_sweepability_bps(&self) -> u16 {
        self.buy_sweepability_bps
    }

    /// Returns sell-side target sweepability in basis points.
    pub const fn sell_sweepability_bps(&self) -> u16 {
        self.sell_sweepability_bps
    }

    /// Returns conservative target sweepability in basis points.
    pub const fn sweepability_score_bps(&self) -> u16 {
        self.sweepability_score_bps
    }
}

/// Liquidity-flow event over a book observation interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityFlowEvent {
    side: Side,
    added_qty: i64,
    removed_qty: i64,
    traded_qty: i64,
    ts_ns: u64,
}

impl LiquidityFlowEvent {
    /// Creates a liquidity-flow event.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDepth`] when quantities are negative,
    /// timestamp is zero, or the event carries no quantity.
    pub const fn new(
        side: Side,
        added_qty: i64,
        removed_qty: i64,
        traded_qty: i64,
        ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if added_qty < 0
            || removed_qty < 0
            || traded_qty < 0
            || ts_ns == 0
            || added_qty
                .saturating_add(removed_qty)
                .saturating_add(traded_qty)
                == 0
        {
            return Err(AnalyticsError::InvalidDepth);
        }
        Ok(Self {
            side,
            added_qty,
            removed_qty,
            traded_qty,
            ts_ns,
        })
    }

    /// Returns the book side affected by the event.
    pub const fn side(&self) -> Side {
        self.side
    }

    /// Returns quantity added to the side.
    pub const fn added_qty(&self) -> i64 {
        self.added_qty
    }

    /// Returns quantity removed or canceled from the side.
    pub const fn removed_qty(&self) -> i64 {
        self.removed_qty
    }

    /// Returns quantity traded through the side.
    pub const fn traded_qty(&self) -> i64 {
        self.traded_qty
    }

    /// Returns event timestamp in nanoseconds.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }
}

/// Liquidity-flow tracker configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityFlowConfig {
    window_ns: u64,
    drought_replenishment_bps: u16,
    drought_min_depletion_qty: i64,
}

impl LiquidityFlowConfig {
    /// Creates liquidity-flow configuration.
    ///
    /// `drought_replenishment_bps` compares added quantity with depleted
    /// quantity. For example, `2_500` marks drought risk when replenishment is
    /// less than 25% of depletion.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDepth`] when the window is zero,
    /// threshold exceeds 10,000 basis points, or the minimum depletion quantity
    /// is negative.
    pub const fn new(
        window_ns: u64,
        drought_replenishment_bps: u16,
        drought_min_depletion_qty: i64,
    ) -> Result<Self, AnalyticsError> {
        if window_ns == 0 || drought_replenishment_bps > 10_000 || drought_min_depletion_qty < 0 {
            return Err(AnalyticsError::InvalidDepth);
        }
        Ok(Self {
            window_ns,
            drought_replenishment_bps,
            drought_min_depletion_qty,
        })
    }

    /// Returns configured observation window in nanoseconds.
    pub const fn window_ns(&self) -> u64 {
        self.window_ns
    }

    /// Returns drought replenishment threshold in basis points.
    pub const fn drought_replenishment_bps(&self) -> u16 {
        self.drought_replenishment_bps
    }

    /// Returns minimum depletion quantity before drought can be flagged.
    pub const fn drought_min_depletion_qty(&self) -> i64 {
        self.drought_min_depletion_qty
    }
}

impl Default for LiquidityFlowConfig {
    fn default() -> Self {
        Self {
            window_ns: 1_000_000_000,
            drought_replenishment_bps: 2_500,
            drought_min_depletion_qty: 1,
        }
    }
}

/// Liquidity-flow snapshot over accumulated book events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityFlowSnapshot {
    events: u64,
    elapsed_ns: u64,
    bid_added_qty: i64,
    ask_added_qty: i64,
    bid_depleted_qty: i64,
    ask_depleted_qty: i64,
    bid_traded_qty: i64,
    ask_traded_qty: i64,
    order_flow_imbalance_bps: i32,
    replenishment_rate_per_sec: i64,
    depletion_rate_per_sec: i64,
    liquidity_drought: bool,
}

impl LiquidityFlowSnapshot {
    /// Returns number of events included in the snapshot.
    pub const fn events(&self) -> u64 {
        self.events
    }

    /// Returns elapsed observation time in nanoseconds.
    pub const fn elapsed_ns(&self) -> u64 {
        self.elapsed_ns
    }

    /// Returns bid-side added quantity.
    pub const fn bid_added_qty(&self) -> i64 {
        self.bid_added_qty
    }

    /// Returns ask-side added quantity.
    pub const fn ask_added_qty(&self) -> i64 {
        self.ask_added_qty
    }

    /// Returns bid-side canceled or removed quantity.
    pub const fn bid_depleted_qty(&self) -> i64 {
        self.bid_depleted_qty
    }

    /// Returns ask-side canceled or removed quantity.
    pub const fn ask_depleted_qty(&self) -> i64 {
        self.ask_depleted_qty
    }

    /// Returns bid-side traded quantity.
    pub const fn bid_traded_qty(&self) -> i64 {
        self.bid_traded_qty
    }

    /// Returns ask-side traded quantity.
    pub const fn ask_traded_qty(&self) -> i64 {
        self.ask_traded_qty
    }

    /// Returns signed order-flow imbalance in basis points.
    pub const fn order_flow_imbalance_bps(&self) -> i32 {
        self.order_flow_imbalance_bps
    }

    /// Returns replenishment rate in quantity per second.
    pub const fn replenishment_rate_per_sec(&self) -> i64 {
        self.replenishment_rate_per_sec
    }

    /// Returns depletion rate in quantity per second.
    pub const fn depletion_rate_per_sec(&self) -> i64 {
        self.depletion_rate_per_sec
    }

    /// Returns whether replenishment is low relative to depletion.
    pub const fn liquidity_drought(&self) -> bool {
        self.liquidity_drought
    }
}

/// Allocation-free liquidity-flow tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiquidityFlowTracker {
    config: LiquidityFlowConfig,
    events: u64,
    first_ts_ns: u64,
    last_ts_ns: u64,
    bid_added_qty: i64,
    ask_added_qty: i64,
    bid_depleted_qty: i64,
    ask_depleted_qty: i64,
    bid_traded_qty: i64,
    ask_traded_qty: i64,
}

impl LiquidityFlowTracker {
    /// Creates a liquidity-flow tracker.
    pub const fn new(config: LiquidityFlowConfig) -> Self {
        Self {
            config,
            events: 0,
            first_ts_ns: 0,
            last_ts_ns: 0,
            bid_added_qty: 0,
            ask_added_qty: 0,
            bid_depleted_qty: 0,
            ask_depleted_qty: 0,
            bid_traded_qty: 0,
            ask_traded_qty: 0,
        }
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> LiquidityFlowConfig {
        self.config
    }

    /// Records a liquidity-flow event.
    pub fn on_event(&mut self, event: LiquidityFlowEvent) {
        if self.first_ts_ns == 0 {
            self.first_ts_ns = event.ts_ns();
        }
        self.last_ts_ns = self.last_ts_ns.max(event.ts_ns());
        self.events = self.events.saturating_add(1);
        match event.side() {
            Side::Bid => {
                self.bid_added_qty = self.bid_added_qty.saturating_add(event.added_qty());
                self.bid_depleted_qty = self.bid_depleted_qty.saturating_add(event.removed_qty());
                self.bid_traded_qty = self.bid_traded_qty.saturating_add(event.traded_qty());
            }
            Side::Ask => {
                self.ask_added_qty = self.ask_added_qty.saturating_add(event.added_qty());
                self.ask_depleted_qty = self.ask_depleted_qty.saturating_add(event.removed_qty());
                self.ask_traded_qty = self.ask_traded_qty.saturating_add(event.traded_qty());
            }
        }
    }

    /// Returns current liquidity-flow snapshot.
    pub fn snapshot(&self) -> LiquidityFlowSnapshot {
        let elapsed_ns = self.elapsed_ns();
        let replenishment = self.bid_added_qty.saturating_add(self.ask_added_qty);
        let depletion = self
            .bid_depleted_qty
            .saturating_add(self.ask_depleted_qty)
            .saturating_add(self.bid_traded_qty)
            .saturating_add(self.ask_traded_qty);
        let upward_pressure = self
            .bid_added_qty
            .saturating_add(self.ask_depleted_qty)
            .saturating_add(self.ask_traded_qty);
        let downward_pressure = self
            .ask_added_qty
            .saturating_add(self.bid_depleted_qty)
            .saturating_add(self.bid_traded_qty);
        let total_pressure = upward_pressure.saturating_add(downward_pressure);
        let order_flow_imbalance_bps = if total_pressure <= 0 {
            0
        } else {
            i32::try_from(
                (i128::from(upward_pressure.saturating_sub(downward_pressure)) * 10_000)
                    / i128::from(total_pressure),
            )
            .unwrap_or(0)
        };
        let liquidity_drought = depletion >= self.config.drought_min_depletion_qty()
            && replenishment.saturating_mul(10_000)
                < depletion.saturating_mul(i64::from(self.config.drought_replenishment_bps()));
        LiquidityFlowSnapshot {
            events: self.events,
            elapsed_ns,
            bid_added_qty: self.bid_added_qty,
            ask_added_qty: self.ask_added_qty,
            bid_depleted_qty: self.bid_depleted_qty,
            ask_depleted_qty: self.ask_depleted_qty,
            bid_traded_qty: self.bid_traded_qty,
            ask_traded_qty: self.ask_traded_qty,
            order_flow_imbalance_bps,
            replenishment_rate_per_sec: qty_rate_per_sec(replenishment, elapsed_ns),
            depletion_rate_per_sec: qty_rate_per_sec(depletion, elapsed_ns),
            liquidity_drought,
        }
    }

    /// Resets accumulated liquidity-flow state.
    pub fn reset(&mut self) {
        self.events = 0;
        self.first_ts_ns = 0;
        self.last_ts_ns = 0;
        self.bid_added_qty = 0;
        self.ask_added_qty = 0;
        self.bid_depleted_qty = 0;
        self.ask_depleted_qty = 0;
        self.bid_traded_qty = 0;
        self.ask_traded_qty = 0;
    }

    fn elapsed_ns(&self) -> u64 {
        let observed = self.last_ts_ns.saturating_sub(self.first_ts_ns);
        if observed == 0 {
            self.config.window_ns()
        } else {
            observed.min(self.config.window_ns())
        }
    }
}

impl Default for LiquidityFlowTracker {
    fn default() -> Self {
        Self::new(LiquidityFlowConfig::default())
    }
}

/// Market-impact sample over a measurement interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactSample {
    start_midpoint: i64,
    end_midpoint: i64,
    signed_qty: i64,
    notional: i128,
}

impl ImpactSample {
    /// Creates an impact sample.
    ///
    /// Positive signed quantity represents buyer-initiated flow; negative
    /// signed quantity represents seller-initiated flow.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when prices, quantity, or
    /// notional are invalid.
    pub const fn new(
        start_midpoint: i64,
        end_midpoint: i64,
        signed_qty: i64,
        notional: i128,
    ) -> Result<Self, AnalyticsError> {
        if start_midpoint <= 0 || end_midpoint <= 0 || signed_qty == 0 || notional <= 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            start_midpoint,
            end_midpoint,
            signed_qty,
            notional,
        })
    }

    /// Returns starting midpoint.
    pub const fn start_midpoint(&self) -> i64 {
        self.start_midpoint
    }

    /// Returns ending midpoint.
    pub const fn end_midpoint(&self) -> i64 {
        self.end_midpoint
    }

    /// Returns signed quantity.
    pub const fn signed_qty(&self) -> i64 {
        self.signed_qty
    }

    /// Returns traded notional over the interval.
    pub const fn notional(&self) -> i128 {
        self.notional
    }

    /// Returns signed price change aligned to flow direction.
    pub const fn signed_price_change(&self) -> i64 {
        if self.signed_qty > 0 {
            self.end_midpoint - self.start_midpoint
        } else {
            self.start_midpoint - self.end_midpoint
        }
    }
}

/// Cumulative market-impact snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactSnapshot {
    samples: u64,
    signed_volume: i64,
    absolute_volume: i64,
    signed_price_change: i64,
    kyle_lambda_ppm: i64,
    amihud_illiquidity_ppm: i64,
}

impl ImpactSnapshot {
    /// Returns sample count.
    pub const fn samples(&self) -> u64 {
        self.samples
    }

    /// Returns cumulative signed volume.
    pub const fn signed_volume(&self) -> i64 {
        self.signed_volume
    }

    /// Returns cumulative absolute volume.
    pub const fn absolute_volume(&self) -> i64 {
        self.absolute_volume
    }

    /// Returns cumulative signed price change.
    pub const fn signed_price_change(&self) -> i64 {
        self.signed_price_change
    }

    /// Returns Kyle-style price impact per unit signed volume, scaled by
    /// 1,000,000.
    pub const fn kyle_lambda_ppm(&self) -> i64 {
        self.kyle_lambda_ppm
    }

    /// Returns Amihud-style absolute return per notional, scaled by 1,000,000.
    pub const fn amihud_illiquidity_ppm(&self) -> i64 {
        self.amihud_illiquidity_ppm
    }
}

/// Allocation-free cumulative impact tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactTracker {
    samples: u64,
    signed_volume: i64,
    absolute_volume: i64,
    signed_price_change: i64,
    absolute_return_ppm_sum: i128,
    notional_sum: i128,
}

impl ImpactTracker {
    /// Creates an empty impact tracker.
    pub const fn new() -> Self {
        Self {
            samples: 0,
            signed_volume: 0,
            absolute_volume: 0,
            signed_price_change: 0,
            absolute_return_ppm_sum: 0,
            notional_sum: 0,
        }
    }

    /// Records one impact sample.
    pub fn on_sample(&mut self, sample: ImpactSample) {
        self.samples = self.samples.saturating_add(1);
        self.signed_volume = self.signed_volume.saturating_add(sample.signed_qty());
        self.absolute_volume = self
            .absolute_volume
            .saturating_add(sample.signed_qty().abs());
        self.signed_price_change = self
            .signed_price_change
            .saturating_add(sample.signed_price_change());
        self.absolute_return_ppm_sum = self.absolute_return_ppm_sum.saturating_add(
            i128::from(sample.end_midpoint().abs_diff(sample.start_midpoint()) as i64)
                .saturating_mul(1_000_000)
                / i128::from(sample.start_midpoint()),
        );
        self.notional_sum = self.notional_sum.saturating_add(sample.notional());
    }

    /// Returns current impact snapshot.
    pub fn snapshot(&self) -> ImpactSnapshot {
        ImpactSnapshot {
            samples: self.samples,
            signed_volume: self.signed_volume,
            absolute_volume: self.absolute_volume,
            signed_price_change: self.signed_price_change,
            kyle_lambda_ppm: if self.signed_volume == 0 {
                0
            } else {
                i64::try_from(
                    (i128::from(self.signed_price_change) * 1_000_000)
                        / i128::from(self.signed_volume),
                )
                .unwrap_or(0)
            },
            amihud_illiquidity_ppm: if self.notional_sum <= 0 {
                0
            } else {
                i64::try_from((self.absolute_return_ppm_sum * 1_000_000) / self.notional_sum)
                    .unwrap_or(0)
            },
        }
    }
}

impl Default for ImpactTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Calibrated market-impact parameters for pre-trade estimates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactCalibration {
    daily_volume: i64,
    volatility_bps: u16,
    square_root_coefficient_bps: u16,
    temporary_impact_coefficient_bps: u16,
    permanent_impact_coefficient_bps: u16,
    decay_half_life_ns: u64,
}

impl ImpactCalibration {
    /// Creates calibrated impact parameters.
    ///
    /// Coefficients are basis-point scaled. A `square_root_coefficient_bps` of
    /// `10_000` means one volatility unit times the square-root participation
    /// estimate.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when daily volume is
    /// non-positive.
    pub const fn new(
        daily_volume: i64,
        volatility_bps: u16,
        square_root_coefficient_bps: u16,
        temporary_impact_coefficient_bps: u16,
        permanent_impact_coefficient_bps: u16,
        decay_half_life_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if daily_volume <= 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            daily_volume,
            volatility_bps,
            square_root_coefficient_bps,
            temporary_impact_coefficient_bps,
            permanent_impact_coefficient_bps,
            decay_half_life_ns,
        })
    }

    /// Returns expected daily volume.
    pub const fn daily_volume(&self) -> i64 {
        self.daily_volume
    }

    /// Returns volatility in basis points.
    pub const fn volatility_bps(&self) -> u16 {
        self.volatility_bps
    }

    /// Returns square-root impact coefficient in basis points.
    pub const fn square_root_coefficient_bps(&self) -> u16 {
        self.square_root_coefficient_bps
    }

    /// Returns temporary impact coefficient in basis points.
    pub const fn temporary_impact_coefficient_bps(&self) -> u16 {
        self.temporary_impact_coefficient_bps
    }

    /// Returns permanent impact coefficient in basis points.
    pub const fn permanent_impact_coefficient_bps(&self) -> u16 {
        self.permanent_impact_coefficient_bps
    }

    /// Returns impact decay half-life in nanoseconds.
    pub const fn decay_half_life_ns(&self) -> u64 {
        self.decay_half_life_ns
    }
}

/// Pre-trade impact estimate input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedImpactInput {
    side: Side,
    order_qty: i64,
    expected_interval_volume: i64,
    arrival_midpoint: i64,
    horizon_ns: u64,
    calibration: ImpactCalibration,
}

impl ExpectedImpactInput {
    /// Creates pre-trade impact estimate input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when quantities, price, or
    /// horizon are invalid.
    pub const fn new(
        side: Side,
        order_qty: i64,
        expected_interval_volume: i64,
        arrival_midpoint: i64,
        horizon_ns: u64,
        calibration: ImpactCalibration,
    ) -> Result<Self, AnalyticsError> {
        if order_qty <= 0
            || expected_interval_volume <= 0
            || arrival_midpoint <= 0
            || horizon_ns == 0
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            side,
            order_qty,
            expected_interval_volume,
            arrival_midpoint,
            horizon_ns,
            calibration,
        })
    }

    /// Returns execution side.
    pub const fn side(&self) -> Side {
        self.side
    }

    /// Returns proposed order quantity.
    pub const fn order_qty(&self) -> i64 {
        self.order_qty
    }

    /// Returns expected interval volume.
    pub const fn expected_interval_volume(&self) -> i64 {
        self.expected_interval_volume
    }

    /// Returns arrival midpoint.
    pub const fn arrival_midpoint(&self) -> i64 {
        self.arrival_midpoint
    }

    /// Returns execution horizon in nanoseconds.
    pub const fn horizon_ns(&self) -> u64 {
        self.horizon_ns
    }

    /// Returns impact calibration.
    pub const fn calibration(&self) -> ImpactCalibration {
        self.calibration
    }
}

/// Pre-trade market-impact estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedImpactSnapshot {
    participation_bps: u16,
    daily_participation_bps: u16,
    square_root_impact_bps: i32,
    temporary_impact_bps: i32,
    permanent_impact_bps: i32,
    instantaneous_impact_bps: i32,
    decay_remaining_bps: u16,
    expected_total_impact_bps: i32,
    expected_signed_price_move: i64,
}

impl ExpectedImpactSnapshot {
    /// Returns interval participation in basis points.
    pub const fn participation_bps(&self) -> u16 {
        self.participation_bps
    }

    /// Returns daily participation in basis points.
    pub const fn daily_participation_bps(&self) -> u16 {
        self.daily_participation_bps
    }

    /// Returns square-root impact estimate in basis points.
    pub const fn square_root_impact_bps(&self) -> i32 {
        self.square_root_impact_bps
    }

    /// Returns temporary impact estimate in basis points.
    pub const fn temporary_impact_bps(&self) -> i32 {
        self.temporary_impact_bps
    }

    /// Returns permanent impact estimate in basis points.
    pub const fn permanent_impact_bps(&self) -> i32 {
        self.permanent_impact_bps
    }

    /// Returns instantaneous impact estimate in basis points.
    pub const fn instantaneous_impact_bps(&self) -> i32 {
        self.instantaneous_impact_bps
    }

    /// Returns remaining temporary impact after the horizon in basis points.
    pub const fn decay_remaining_bps(&self) -> u16 {
        self.decay_remaining_bps
    }

    /// Returns expected total impact cost in basis points.
    pub const fn expected_total_impact_bps(&self) -> i32 {
        self.expected_total_impact_bps
    }

    /// Returns expected signed midpoint move in normalized price units.
    pub const fn expected_signed_price_move(&self) -> i64 {
        self.expected_signed_price_move
    }
}

/// Deterministic pre-trade market-impact estimator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedImpactEstimator;

impl ExpectedImpactEstimator {
    /// Estimates pre-trade market impact from explicit calibration.
    pub fn estimate(input: ExpectedImpactInput) -> ExpectedImpactSnapshot {
        let participation_bps = ratio_bps_i64(input.order_qty(), input.expected_interval_volume());
        let daily_participation_bps =
            ratio_bps_i64(input.order_qty(), input.calibration().daily_volume());
        let sqrt_participation_bps =
            integer_sqrt_u128(u128::from(daily_participation_bps) * 10_000);
        let square_root_impact_bps = i32::try_from(
            (u128::from(input.calibration().volatility_bps())
                * u128::from(input.calibration().square_root_coefficient_bps())
                * sqrt_participation_bps)
                / 100_000_000,
        )
        .unwrap_or(i32::MAX);
        let temporary_impact_bps = coefficient_impact_bps(
            participation_bps,
            input.calibration().temporary_impact_coefficient_bps(),
        );
        let permanent_impact_bps = coefficient_impact_bps(
            participation_bps,
            input.calibration().permanent_impact_coefficient_bps(),
        );
        let instantaneous_impact_bps = temporary_impact_bps.saturating_add(permanent_impact_bps);
        let decay_remaining_bps =
            decay_remaining_bps(input.horizon_ns(), input.calibration().decay_half_life_ns());
        let decayed_temporary_bps = i32::try_from(
            (i128::from(temporary_impact_bps) * i128::from(decay_remaining_bps)) / 10_000,
        )
        .unwrap_or(0);
        let expected_total_impact_bps = square_root_impact_bps
            .saturating_add(permanent_impact_bps)
            .saturating_add(decayed_temporary_bps);
        let price_move =
            bps_to_price_delta(input.arrival_midpoint(), expected_total_impact_bps.abs());
        let expected_signed_price_move = match input.side() {
            Side::Ask => price_move,
            Side::Bid => price_move.saturating_neg(),
        };
        ExpectedImpactSnapshot {
            participation_bps,
            daily_participation_bps,
            square_root_impact_bps,
            temporary_impact_bps,
            permanent_impact_bps,
            instantaneous_impact_bps,
            decay_remaining_bps,
            expected_total_impact_bps,
            expected_signed_price_move,
        }
    }
}

/// Child-order impact attribution context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildOrderImpactContext {
    side: Side,
    parent_qty: i64,
    child_qty: i64,
    arrival_midpoint: i64,
    child_fill_price: i64,
    post_child_midpoint: i64,
    final_midpoint: i64,
}

impl ChildOrderImpactContext {
    /// Creates child-order impact attribution context.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when quantities or prices are
    /// invalid.
    pub const fn new(
        side: Side,
        parent_qty: i64,
        child_qty: i64,
        arrival_midpoint: i64,
        child_fill_price: i64,
        post_child_midpoint: i64,
        final_midpoint: i64,
    ) -> Result<Self, AnalyticsError> {
        if parent_qty <= 0
            || child_qty <= 0
            || child_qty > parent_qty
            || arrival_midpoint <= 0
            || child_fill_price <= 0
            || post_child_midpoint <= 0
            || final_midpoint <= 0
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            side,
            parent_qty,
            child_qty,
            arrival_midpoint,
            child_fill_price,
            post_child_midpoint,
            final_midpoint,
        })
    }

    /// Returns execution side.
    pub const fn side(&self) -> Side {
        self.side
    }

    /// Returns parent order quantity.
    pub const fn parent_qty(&self) -> i64 {
        self.parent_qty
    }

    /// Returns child order quantity.
    pub const fn child_qty(&self) -> i64 {
        self.child_qty
    }

    /// Returns arrival midpoint.
    pub const fn arrival_midpoint(&self) -> i64 {
        self.arrival_midpoint
    }

    /// Returns child fill price.
    pub const fn child_fill_price(&self) -> i64 {
        self.child_fill_price
    }

    /// Returns midpoint immediately after the child order.
    pub const fn post_child_midpoint(&self) -> i64 {
        self.post_child_midpoint
    }

    /// Returns final midpoint after the attribution horizon.
    pub const fn final_midpoint(&self) -> i64 {
        self.final_midpoint
    }
}

/// Child-order impact attribution snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildOrderImpactSnapshot {
    child_participation_bps: u16,
    child_slippage_bps: i32,
    instantaneous_impact_bps: i32,
    permanent_impact_bps: i32,
    temporary_impact_bps: i32,
    impact_decay_bps: i32,
    attributed_impact_bps: i32,
}

impl ChildOrderImpactSnapshot {
    /// Returns child share of parent quantity in basis points.
    pub const fn child_participation_bps(&self) -> u16 {
        self.child_participation_bps
    }

    /// Returns child fill slippage versus arrival in basis points.
    pub const fn child_slippage_bps(&self) -> i32 {
        self.child_slippage_bps
    }

    /// Returns immediate post-child impact in basis points.
    pub const fn instantaneous_impact_bps(&self) -> i32 {
        self.instantaneous_impact_bps
    }

    /// Returns permanent impact at the attribution horizon in basis points.
    pub const fn permanent_impact_bps(&self) -> i32 {
        self.permanent_impact_bps
    }

    /// Returns temporary impact component in basis points.
    pub const fn temporary_impact_bps(&self) -> i32 {
        self.temporary_impact_bps
    }

    /// Returns impact decay from immediate to final mark in basis points.
    pub const fn impact_decay_bps(&self) -> i32 {
        self.impact_decay_bps
    }

    /// Returns parent-weighted child attribution in basis points.
    pub const fn attributed_impact_bps(&self) -> i32 {
        self.attributed_impact_bps
    }
}

/// Deterministic child-order impact attribution analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildOrderImpactAnalyzer;

impl ChildOrderImpactAnalyzer {
    /// Evaluates child-order impact attribution.
    pub fn evaluate(context: ChildOrderImpactContext) -> ChildOrderImpactSnapshot {
        let child_participation_bps = ratio_bps_i64(context.child_qty(), context.parent_qty());
        let child_slippage_bps = side_aware_price_move_bps(
            context.side(),
            context.arrival_midpoint(),
            context.child_fill_price(),
        );
        let instantaneous_impact_bps = side_aware_price_move_bps(
            context.side(),
            context.arrival_midpoint(),
            context.post_child_midpoint(),
        );
        let permanent_impact_bps = side_aware_price_move_bps(
            context.side(),
            context.arrival_midpoint(),
            context.final_midpoint(),
        );
        let temporary_impact_bps = child_slippage_bps.saturating_sub(permanent_impact_bps);
        let impact_decay_bps = instantaneous_impact_bps.saturating_sub(permanent_impact_bps);
        let attributed_impact_bps = i32::try_from(
            (i128::from(child_slippage_bps) * i128::from(child_participation_bps)) / 10_000,
        )
        .unwrap_or(0);
        ChildOrderImpactSnapshot {
            child_participation_bps,
            child_slippage_bps,
            instantaneous_impact_bps,
            permanent_impact_bps,
            temporary_impact_bps,
            impact_decay_bps,
            attributed_impact_bps,
        }
    }
}

/// VPIN-style toxicity snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VpinSnapshot {
    bucket_count: usize,
    current_bucket_volume: i64,
    vpin_bps: u16,
    toxicity_bps: u16,
}

impl VpinSnapshot {
    /// Returns completed bucket count.
    pub const fn bucket_count(&self) -> usize {
        self.bucket_count
    }

    /// Returns current open bucket volume.
    pub const fn current_bucket_volume(&self) -> i64 {
        self.current_bucket_volume
    }

    /// Returns VPIN in basis points.
    pub const fn vpin_bps(&self) -> u16 {
        self.vpin_bps
    }

    /// Returns current toxicity basis points including only completed buckets.
    pub const fn toxicity_bps(&self) -> u16 {
        self.toxicity_bps
    }
}

/// Fixed-capacity VPIN-style bucket tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VpinTracker<const N: usize = 32> {
    bucket_volume: i64,
    bucket_imbalances: [i64; N],
    next_bucket: usize,
    bucket_count: usize,
    current_buy_volume: i64,
    current_sell_volume: i64,
}

impl<const N: usize> VpinTracker<N> {
    /// Creates a VPIN tracker.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when bucket volume is not
    /// positive or capacity is zero.
    pub const fn new(bucket_volume: i64) -> Result<Self, AnalyticsError> {
        if bucket_volume <= 0 || N == 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            bucket_volume,
            bucket_imbalances: [0; N],
            next_bucket: 0,
            bucket_count: 0,
            current_buy_volume: 0,
            current_sell_volume: 0,
        })
    }

    /// Records one trade.
    pub fn on_trade(&mut self, trade: TradeContext) {
        match trade.aggressor_side() {
            Side::Ask => {
                self.current_buy_volume = self.current_buy_volume.saturating_add(trade.qty())
            }
            Side::Bid => {
                self.current_sell_volume = self.current_sell_volume.saturating_add(trade.qty())
            }
        }
        if self.current_bucket_volume() >= self.bucket_volume {
            self.close_bucket();
        }
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> VpinSnapshot {
        let count = self.bucket_count.min(N);
        let mut imbalance_sum = 0_i64;
        for imbalance in self.bucket_imbalances.iter().take(count) {
            imbalance_sum = imbalance_sum.saturating_add(*imbalance);
        }
        let denominator = self
            .bucket_volume
            .saturating_mul(i64::try_from(count).unwrap_or(0));
        let vpin_bps = if denominator <= 0 {
            0
        } else {
            u16::try_from(
                ((i128::from(imbalance_sum) * 10_000) / i128::from(denominator)).clamp(0, 10_000),
            )
            .unwrap_or(10_000)
        };
        VpinSnapshot {
            bucket_count: count,
            current_bucket_volume: self.current_bucket_volume(),
            vpin_bps,
            toxicity_bps: vpin_bps,
        }
    }

    /// Returns configured bucket volume.
    pub const fn bucket_volume(&self) -> i64 {
        self.bucket_volume
    }

    /// Returns current open bucket volume.
    pub const fn current_bucket_volume(&self) -> i64 {
        self.current_buy_volume + self.current_sell_volume
    }

    fn close_bucket(&mut self) {
        self.bucket_imbalances[self.next_bucket] =
            self.current_buy_volume.abs_diff(self.current_sell_volume) as i64;
        self.next_bucket = (self.next_bucket + 1) % N;
        self.bucket_count = self.bucket_count.saturating_add(1).min(N);
        self.current_buy_volume = 0;
        self.current_sell_volume = 0;
    }
}

/// Toxicity/adverse-selection thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToxicityConfig {
    markout_threshold_bps: u16,
    quote_fade_threshold_bps: u16,
    vpin_threshold_bps: u16,
    intensity_threshold_bps: u16,
}

impl ToxicityConfig {
    /// Creates toxicity thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when any threshold is zero or
    /// exceeds 10,000 basis points.
    pub const fn new(
        markout_threshold_bps: u16,
        quote_fade_threshold_bps: u16,
        vpin_threshold_bps: u16,
        intensity_threshold_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if markout_threshold_bps == 0
            || quote_fade_threshold_bps == 0
            || vpin_threshold_bps == 0
            || intensity_threshold_bps == 0
            || markout_threshold_bps > 10_000
            || quote_fade_threshold_bps > 10_000
            || vpin_threshold_bps > 10_000
            || intensity_threshold_bps > 10_000
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            markout_threshold_bps,
            quote_fade_threshold_bps,
            vpin_threshold_bps,
            intensity_threshold_bps,
        })
    }

    /// Returns adverse markout threshold in basis points.
    pub const fn markout_threshold_bps(&self) -> u16 {
        self.markout_threshold_bps
    }

    /// Returns quote-fade threshold in basis points.
    pub const fn quote_fade_threshold_bps(&self) -> u16 {
        self.quote_fade_threshold_bps
    }

    /// Returns VPIN threshold in basis points.
    pub const fn vpin_threshold_bps(&self) -> u16 {
        self.vpin_threshold_bps
    }

    /// Returns trade-intensity threshold in basis points.
    pub const fn intensity_threshold_bps(&self) -> u16 {
        self.intensity_threshold_bps
    }
}

impl Default for ToxicityConfig {
    fn default() -> Self {
        Self {
            markout_threshold_bps: 10,
            quote_fade_threshold_bps: 2_500,
            vpin_threshold_bps: 7_000,
            intensity_threshold_bps: 7_000,
        }
    }
}

/// Toxicity/adverse-selection observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToxicityInput {
    trade: TradeContext,
    future_midpoint: i64,
    pre_bid_qty: i64,
    pre_ask_qty: i64,
    post_bid_qty: i64,
    post_ask_qty: i64,
    vpin_bps: u16,
    trade_intensity_bps: u16,
}

impl ToxicityInput {
    /// Creates toxicity input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when future midpoint is
    /// non-positive, quantities are negative, or basis-point values exceed
    /// 10,000.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        trade: TradeContext,
        future_midpoint: i64,
        pre_bid_qty: i64,
        pre_ask_qty: i64,
        post_bid_qty: i64,
        post_ask_qty: i64,
        vpin_bps: u16,
        trade_intensity_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if future_midpoint <= 0
            || pre_bid_qty < 0
            || pre_ask_qty < 0
            || post_bid_qty < 0
            || post_ask_qty < 0
            || vpin_bps > 10_000
            || trade_intensity_bps > 10_000
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            trade,
            future_midpoint,
            pre_bid_qty,
            pre_ask_qty,
            post_bid_qty,
            post_ask_qty,
            vpin_bps,
            trade_intensity_bps,
        })
    }

    /// Returns trade context.
    pub const fn trade(&self) -> TradeContext {
        self.trade
    }

    /// Returns future midpoint.
    pub const fn future_midpoint(&self) -> i64 {
        self.future_midpoint
    }

    /// Returns pre-trade bid quantity.
    pub const fn pre_bid_qty(&self) -> i64 {
        self.pre_bid_qty
    }

    /// Returns pre-trade ask quantity.
    pub const fn pre_ask_qty(&self) -> i64 {
        self.pre_ask_qty
    }

    /// Returns post-trade bid quantity.
    pub const fn post_bid_qty(&self) -> i64 {
        self.post_bid_qty
    }

    /// Returns post-trade ask quantity.
    pub const fn post_ask_qty(&self) -> i64 {
        self.post_ask_qty
    }

    /// Returns VPIN or equivalent flow-imbalance score in basis points.
    pub const fn vpin_bps(&self) -> u16 {
        self.vpin_bps
    }

    /// Returns trade-intensity score in basis points.
    pub const fn trade_intensity_bps(&self) -> u16 {
        self.trade_intensity_bps
    }
}

/// Toxicity/adverse-selection risk snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToxicitySnapshot {
    post_trade_markout_bps: i32,
    adverse_selection_score_bps: u16,
    quote_fade_bps: i32,
    informed_flow_proxy_bps: u16,
    toxic_flow_burst: bool,
    toxicity_score_bps: u16,
}

impl ToxicitySnapshot {
    /// Returns side-aware post-trade markout in basis points.
    pub const fn post_trade_markout_bps(&self) -> i32 {
        self.post_trade_markout_bps
    }

    /// Returns adverse-selection score in basis points.
    pub const fn adverse_selection_score_bps(&self) -> u16 {
        self.adverse_selection_score_bps
    }

    /// Returns same-side quote fade in basis points.
    pub const fn quote_fade_bps(&self) -> i32 {
        self.quote_fade_bps
    }

    /// Returns informed-flow proxy score in basis points.
    pub const fn informed_flow_proxy_bps(&self) -> u16 {
        self.informed_flow_proxy_bps
    }

    /// Returns whether the observation crosses toxic-burst thresholds.
    pub const fn toxic_flow_burst(&self) -> bool {
        self.toxic_flow_burst
    }

    /// Returns aggregate toxicity score in basis points.
    pub const fn toxicity_score_bps(&self) -> u16 {
        self.toxicity_score_bps
    }
}

/// Deterministic toxicity/adverse-selection analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToxicityAnalyzer {
    config: ToxicityConfig,
}

impl ToxicityAnalyzer {
    /// Creates a toxicity analyzer.
    pub const fn new(config: ToxicityConfig) -> Self {
        Self { config }
    }

    /// Returns analyzer configuration.
    pub const fn config(&self) -> ToxicityConfig {
        self.config
    }

    /// Evaluates one toxicity/adverse-selection observation.
    pub fn evaluate(&self, input: ToxicityInput) -> ToxicitySnapshot {
        let post_trade_markout_bps = side_aware_price_move_bps(
            input.trade().aggressor_side(),
            input.trade().price(),
            input.future_midpoint(),
        );
        let quote_fade_bps = quote_fade_bps(input);
        let adverse_selection_score_bps = score_ratio(
            positive_bps(post_trade_markout_bps),
            u32::from(self.config.markout_threshold_bps()),
        );
        let quote_fade_score_bps = score_ratio(
            positive_bps(quote_fade_bps),
            u32::from(self.config.quote_fade_threshold_bps()),
        );
        let vpin_score_bps = score_ratio(
            u32::from(input.vpin_bps()),
            u32::from(self.config.vpin_threshold_bps()),
        );
        let intensity_score_bps = score_ratio(
            u32::from(input.trade_intensity_bps()),
            u32::from(self.config.intensity_threshold_bps()),
        );
        let informed_flow_proxy_bps = average_bps4(
            adverse_selection_score_bps,
            quote_fade_score_bps,
            vpin_score_bps,
            intensity_score_bps,
        );
        let toxic_flow_burst = positive_bps(post_trade_markout_bps)
            >= u32::from(self.config.markout_threshold_bps())
            && (input.vpin_bps() >= self.config.vpin_threshold_bps()
                || positive_bps(quote_fade_bps)
                    >= u32::from(self.config.quote_fade_threshold_bps())
                || input.trade_intensity_bps() >= self.config.intensity_threshold_bps());
        let toxicity_score_bps =
            informed_flow_proxy_bps.max(adverse_selection_score_bps.min(10_000));
        ToxicitySnapshot {
            post_trade_markout_bps,
            adverse_selection_score_bps,
            quote_fade_bps,
            informed_flow_proxy_bps,
            toxic_flow_burst,
            toxicity_score_bps,
        }
    }
}

impl Default for ToxicityAnalyzer {
    fn default() -> Self {
        Self::new(ToxicityConfig::default())
    }
}

/// Rolling volatility/noise snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilitySnapshot {
    samples: usize,
    last_price: i64,
    realized_vol_bps: u32,
    mean_abs_return_bps: u32,
    bipower_vol_bps: u32,
    jump_variation_bps: u32,
    noise_ratio_bps: u16,
}

impl VolatilitySnapshot {
    /// Returns retained return sample count.
    pub const fn samples(&self) -> usize {
        self.samples
    }

    /// Returns last observed price.
    pub const fn last_price(&self) -> i64 {
        self.last_price
    }

    /// Returns realized volatility in basis points.
    pub const fn realized_vol_bps(&self) -> u32 {
        self.realized_vol_bps
    }

    /// Returns mean absolute return in basis points.
    pub const fn mean_abs_return_bps(&self) -> u32 {
        self.mean_abs_return_bps
    }

    /// Returns bipower variation volatility proxy in basis points.
    pub const fn bipower_vol_bps(&self) -> u32 {
        self.bipower_vol_bps
    }

    /// Returns jump variation proxy in basis points.
    pub const fn jump_variation_bps(&self) -> u32 {
        self.jump_variation_bps
    }

    /// Returns microstructure noise proxy in basis points.
    pub const fn noise_ratio_bps(&self) -> u16 {
        self.noise_ratio_bps
    }
}

/// Fixed-window volatility tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilityTracker<const N: usize = 64> {
    returns_bps: [i32; N],
    next: usize,
    len: usize,
    last_price: i64,
}

impl<const N: usize> VolatilityTracker<N> {
    /// Creates an empty volatility tracker.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when capacity is zero.
    pub const fn new() -> Result<Self, AnalyticsError> {
        if N == 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            returns_bps: [0; N],
            next: 0,
            len: 0,
            last_price: 0,
        })
    }

    /// Records a price observation.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when price is not positive.
    pub fn on_price(&mut self, price: i64) -> Result<(), AnalyticsError> {
        if price <= 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        if self.last_price > 0 {
            let ret = i32::try_from(
                ((i128::from(price) - i128::from(self.last_price)) * 10_000)
                    / i128::from(self.last_price),
            )
            .unwrap_or(0);
            self.returns_bps[self.next] = ret;
            self.next = (self.next + 1) % N;
            self.len = self.len.saturating_add(1).min(N);
        }
        self.last_price = price;
        Ok(())
    }

    /// Returns current volatility snapshot.
    pub fn snapshot(&self) -> VolatilitySnapshot {
        if self.len == 0 {
            return VolatilitySnapshot {
                samples: 0,
                last_price: self.last_price,
                realized_vol_bps: 0,
                mean_abs_return_bps: 0,
                bipower_vol_bps: 0,
                jump_variation_bps: 0,
                noise_ratio_bps: 0,
            };
        }
        let mut sum_sq = 0_u128;
        let mut sum_abs = 0_u128;
        let mut bipower_sum = 0_u128;
        let mut sign_flips = 0_u32;
        let mut prev_sign = 0_i32;
        let mut prev_abs = 0_u32;
        for offset in 0..self.len {
            let idx = if self.len == N {
                (self.next + offset) % N
            } else {
                offset
            };
            let ret = self.returns_bps[idx];
            let abs = ret.unsigned_abs();
            sum_abs = sum_abs.saturating_add(u128::from(abs));
            sum_sq = sum_sq.saturating_add(u128::from(abs).saturating_mul(u128::from(abs)));
            if prev_abs > 0 {
                bipower_sum = bipower_sum
                    .saturating_add(u128::from(prev_abs).saturating_mul(u128::from(abs)));
            }
            prev_abs = abs;
            let sign = ret.signum();
            if prev_sign != 0 && sign != 0 && sign != prev_sign {
                sign_flips = sign_flips.saturating_add(1);
            }
            if sign != 0 {
                prev_sign = sign;
            }
        }
        let len = u128::try_from(self.len).unwrap_or(1);
        let realized = isqrt_u128(sum_sq / len);
        let mean_abs = sum_abs / len;
        let bipower = if self.len <= 1 {
            0
        } else {
            let mean_bipower = bipower_sum / u128::try_from(self.len - 1).unwrap_or(1);
            isqrt_u128((mean_bipower.saturating_mul(15_708)) / 10_000)
        };
        let jump_variation_bps = realized.saturating_sub(bipower);
        let noise_ratio_bps = if self.len <= 1 {
            0
        } else {
            u16::try_from(
                (u128::from(sign_flips) * 10_000) / u128::try_from(self.len - 1).unwrap_or(1),
            )
            .unwrap_or(10_000)
        };
        VolatilitySnapshot {
            samples: self.len,
            last_price: self.last_price,
            realized_vol_bps: u32::try_from(realized).unwrap_or(u32::MAX),
            mean_abs_return_bps: u32::try_from(mean_abs).unwrap_or(u32::MAX),
            bipower_vol_bps: u32::try_from(bipower).unwrap_or(u32::MAX),
            jump_variation_bps: u32::try_from(jump_variation_bps).unwrap_or(u32::MAX),
            noise_ratio_bps,
        }
    }
}

/// OHLC volatility estimator input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OhlcVolatilityInput {
    open: i64,
    high: i64,
    low: i64,
    close: i64,
    previous_close: Option<i64>,
}

impl OhlcVolatilityInput {
    /// Creates OHLC volatility input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when prices are non-positive,
    /// inconsistent, or previous close is non-positive when supplied.
    pub const fn new(
        open: i64,
        high: i64,
        low: i64,
        close: i64,
        previous_close: Option<i64>,
    ) -> Result<Self, AnalyticsError> {
        if open <= 0
            || high <= 0
            || low <= 0
            || close <= 0
            || low > high
            || open > high
            || open < low
            || close > high
            || close < low
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        if let Some(previous_close) = previous_close {
            if previous_close <= 0 {
                return Err(AnalyticsError::InvalidTrade);
            }
        }
        Ok(Self {
            open,
            high,
            low,
            close,
            previous_close,
        })
    }

    /// Returns open price.
    pub const fn open(&self) -> i64 {
        self.open
    }

    /// Returns high price.
    pub const fn high(&self) -> i64 {
        self.high
    }

    /// Returns low price.
    pub const fn low(&self) -> i64 {
        self.low
    }

    /// Returns close price.
    pub const fn close(&self) -> i64 {
        self.close
    }

    /// Returns previous close when available.
    pub const fn previous_close(&self) -> Option<i64> {
        self.previous_close
    }
}

/// OHLC volatility estimator snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OhlcVolatilitySnapshot {
    close_to_close_vol_bps: u32,
    parkinson_vol_bps: u32,
    garman_klass_vol_bps: u32,
    rogers_satchell_vol_bps: u32,
    jump_gap_bps: i32,
}

impl OhlcVolatilitySnapshot {
    /// Returns close-to-close volatility proxy in basis points.
    pub const fn close_to_close_vol_bps(&self) -> u32 {
        self.close_to_close_vol_bps
    }

    /// Returns Parkinson range volatility proxy in basis points.
    pub const fn parkinson_vol_bps(&self) -> u32 {
        self.parkinson_vol_bps
    }

    /// Returns Garman-Klass volatility proxy in basis points.
    pub const fn garman_klass_vol_bps(&self) -> u32 {
        self.garman_klass_vol_bps
    }

    /// Returns Rogers-Satchell volatility proxy in basis points.
    pub const fn rogers_satchell_vol_bps(&self) -> u32 {
        self.rogers_satchell_vol_bps
    }

    /// Returns signed open gap versus previous close in basis points.
    pub const fn jump_gap_bps(&self) -> i32 {
        self.jump_gap_bps
    }
}

/// Deterministic OHLC volatility estimator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OhlcVolatilityEstimator;

impl OhlcVolatilityEstimator {
    /// Estimates OHLC volatility from one candle.
    pub fn estimate(input: OhlcVolatilityInput) -> OhlcVolatilitySnapshot {
        let high_low_bps = abs_return_bps(input.high(), input.low());
        let open_close_bps = abs_return_bps(input.close(), input.open());
        let close_to_close_vol_bps = input
            .previous_close()
            .map(|previous| abs_return_bps(input.close(), previous))
            .unwrap_or(open_close_bps);
        let parkinson_vol_bps =
            u32::try_from((u128::from(high_low_bps) * 6_006) / 10_000).unwrap_or(u32::MAX);
        let gk_var = ((u128::from(high_low_bps).saturating_mul(u128::from(high_low_bps)) * 5_000)
            / 10_000)
            .saturating_sub(
                (u128::from(open_close_bps).saturating_mul(u128::from(open_close_bps)) * 3_863)
                    / 10_000,
            );
        let high_open = i128::from(return_bps(input.high(), input.open()));
        let high_close = i128::from(return_bps(input.high(), input.close()));
        let low_open = i128::from(return_bps(input.low(), input.open()));
        let low_close = i128::from(return_bps(input.low(), input.close()));
        let rs_var = high_open
            .saturating_mul(high_close)
            .saturating_add(low_open.saturating_mul(low_close))
            .max(0) as u128;
        let jump_gap_bps = input
            .previous_close()
            .map(|previous| return_bps(input.open(), previous))
            .unwrap_or(0);
        OhlcVolatilitySnapshot {
            close_to_close_vol_bps,
            parkinson_vol_bps,
            garman_klass_vol_bps: u32::try_from(isqrt_u128(gk_var)).unwrap_or(u32::MAX),
            rogers_satchell_vol_bps: u32::try_from(isqrt_u128(rs_var)).unwrap_or(u32::MAX),
            jump_gap_bps,
        }
    }
}

/// Volatility signature point over borrowed returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilitySignatureSnapshot {
    sampling_interval_ns: u64,
    samples: usize,
    realized_vol_bps: u32,
    noise_ratio_bps: u16,
}

impl VolatilitySignatureSnapshot {
    /// Returns sampling interval in nanoseconds.
    pub const fn sampling_interval_ns(&self) -> u64 {
        self.sampling_interval_ns
    }

    /// Returns return sample count.
    pub const fn samples(&self) -> usize {
        self.samples
    }

    /// Returns realized volatility in basis points.
    pub const fn realized_vol_bps(&self) -> u32 {
        self.realized_vol_bps
    }

    /// Returns sign-flip microstructure noise proxy in basis points.
    pub const fn noise_ratio_bps(&self) -> u16 {
        self.noise_ratio_bps
    }
}

/// Borrowed volatility signature estimator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilitySignatureEstimator;

impl VolatilitySignatureEstimator {
    /// Estimates one signature-plot point from borrowed returns.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when interval or returns are
    /// empty.
    pub fn estimate(
        sampling_interval_ns: u64,
        returns_bps: &[i32],
    ) -> Result<VolatilitySignatureSnapshot, AnalyticsError> {
        if sampling_interval_ns == 0 || returns_bps.is_empty() {
            return Err(AnalyticsError::InvalidTrade);
        }
        let (realized_vol_bps, noise_ratio_bps) = volatility_and_noise(returns_bps);
        Ok(VolatilitySignatureSnapshot {
            sampling_interval_ns,
            samples: returns_bps.len(),
            realized_vol_bps,
            noise_ratio_bps,
        })
    }
}

/// Intraday volatility seasonality bucket snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilitySeasonalitySnapshot {
    bucket: usize,
    samples: u64,
    realized_vol_bps: u32,
    mean_abs_return_bps: u32,
    jump_count: u64,
}

impl VolatilitySeasonalitySnapshot {
    /// Returns bucket index.
    pub const fn bucket(&self) -> usize {
        self.bucket
    }

    /// Returns sample count.
    pub const fn samples(&self) -> u64 {
        self.samples
    }

    /// Returns realized volatility in basis points.
    pub const fn realized_vol_bps(&self) -> u32 {
        self.realized_vol_bps
    }

    /// Returns mean absolute return in basis points.
    pub const fn mean_abs_return_bps(&self) -> u32 {
        self.mean_abs_return_bps
    }

    /// Returns jump count.
    pub const fn jump_count(&self) -> u64 {
        self.jump_count
    }
}

/// Fixed-bucket intraday volatility seasonality tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolatilitySeasonalityTracker<const N: usize = 96> {
    sum_sq: [u128; N],
    sum_abs: [u128; N],
    samples: [u64; N],
    jump_count: [u64; N],
    jump_threshold_bps: u32,
}

impl<const N: usize> VolatilitySeasonalityTracker<N> {
    /// Creates a seasonality tracker.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when capacity is zero.
    pub const fn new(jump_threshold_bps: u32) -> Result<Self, AnalyticsError> {
        if N == 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            sum_sq: [0; N],
            sum_abs: [0; N],
            samples: [0; N],
            jump_count: [0; N],
            jump_threshold_bps,
        })
    }

    /// Records a return for a bucket.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when bucket is out of range.
    pub fn on_return(&mut self, bucket: usize, return_bps: i32) -> Result<(), AnalyticsError> {
        if bucket >= N {
            return Err(AnalyticsError::InvalidTrade);
        }
        let abs = return_bps.unsigned_abs();
        self.sum_abs[bucket] = self.sum_abs[bucket].saturating_add(u128::from(abs));
        self.sum_sq[bucket] =
            self.sum_sq[bucket].saturating_add(u128::from(abs).saturating_mul(u128::from(abs)));
        self.samples[bucket] = self.samples[bucket].saturating_add(1);
        if abs >= self.jump_threshold_bps {
            self.jump_count[bucket] = self.jump_count[bucket].saturating_add(1);
        }
        Ok(())
    }

    /// Returns one bucket snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when bucket is out of range.
    pub fn snapshot(&self, bucket: usize) -> Result<VolatilitySeasonalitySnapshot, AnalyticsError> {
        if bucket >= N {
            return Err(AnalyticsError::InvalidTrade);
        }
        let samples = self.samples[bucket];
        let realized_vol_bps = if samples == 0 {
            0
        } else {
            u32::try_from(isqrt_u128(self.sum_sq[bucket] / u128::from(samples))).unwrap_or(u32::MAX)
        };
        let mean_abs_return_bps = if samples == 0 {
            0
        } else {
            u32::try_from(self.sum_abs[bucket] / u128::from(samples)).unwrap_or(u32::MAX)
        };
        Ok(VolatilitySeasonalitySnapshot {
            bucket,
            samples,
            realized_vol_bps,
            mean_abs_return_bps,
            jump_count: self.jump_count[bucket],
        })
    }

    /// Resets all buckets.
    pub fn reset(&mut self) {
        self.sum_sq = [0; N];
        self.sum_abs = [0; N];
        self.samples = [0; N];
        self.jump_count = [0; N];
    }
}

/// Market regime classification.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RegimeKind {
    /// Quiet conditions.
    Quiet = 1,
    /// Normal conditions.
    Normal = 2,
    /// Volatility is elevated.
    Volatile = 3,
    /// Toxic flow or adverse-selection risk is elevated.
    Toxic = 4,
    /// Spread/imbalance suggests illiquidity.
    Illiquid = 5,
}

/// Regime classifier input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegimeInput {
    spread_bps: u32,
    volatility_bps: u32,
    toxicity_bps: u16,
    imbalance_abs_bps: u16,
}

impl RegimeInput {
    /// Creates regime input.
    pub const fn new(
        spread_bps: u32,
        volatility_bps: u32,
        toxicity_bps: u16,
        imbalance_abs_bps: u16,
    ) -> Self {
        Self {
            spread_bps,
            volatility_bps,
            toxicity_bps,
            imbalance_abs_bps,
        }
    }

    /// Returns spread in basis points.
    pub const fn spread_bps(&self) -> u32 {
        self.spread_bps
    }

    /// Returns volatility in basis points.
    pub const fn volatility_bps(&self) -> u32 {
        self.volatility_bps
    }

    /// Returns toxicity in basis points.
    pub const fn toxicity_bps(&self) -> u16 {
        self.toxicity_bps
    }

    /// Returns absolute imbalance in basis points.
    pub const fn imbalance_abs_bps(&self) -> u16 {
        self.imbalance_abs_bps
    }
}

/// Regime snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegimeSnapshot {
    kind: RegimeKind,
    stress_bps: u16,
}

impl RegimeSnapshot {
    /// Returns regime kind.
    pub const fn kind(&self) -> RegimeKind {
        self.kind
    }

    /// Returns aggregate stress score in basis points.
    pub const fn stress_bps(&self) -> u16 {
        self.stress_bps
    }
}

/// Threshold-based market regime classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegimeClassifier {
    quiet_vol_bps: u32,
    volatile_vol_bps: u32,
    wide_spread_bps: u32,
    toxic_bps: u16,
    imbalance_bps: u16,
}

impl RegimeClassifier {
    /// Creates regime classifier thresholds.
    pub const fn new(
        quiet_vol_bps: u32,
        volatile_vol_bps: u32,
        wide_spread_bps: u32,
        toxic_bps: u16,
        imbalance_bps: u16,
    ) -> Self {
        Self {
            quiet_vol_bps,
            volatile_vol_bps,
            wide_spread_bps,
            toxic_bps,
            imbalance_bps,
        }
    }

    /// Classifies a regime input.
    pub fn classify(&self, input: RegimeInput) -> RegimeSnapshot {
        let kind = if input.toxicity_bps() >= self.toxic_bps {
            RegimeKind::Toxic
        } else if input.spread_bps() >= self.wide_spread_bps
            || input.imbalance_abs_bps() >= self.imbalance_bps
        {
            RegimeKind::Illiquid
        } else if input.volatility_bps() >= self.volatile_vol_bps {
            RegimeKind::Volatile
        } else if input.volatility_bps() <= self.quiet_vol_bps && input.spread_bps() == 0 {
            RegimeKind::Quiet
        } else {
            RegimeKind::Normal
        };
        let stress = input
            .volatility_bps()
            .min(10_000)
            .max(u32::from(input.toxicity_bps()))
            .max(u32::from(input.imbalance_abs_bps()))
            .max(input.spread_bps().min(10_000));
        RegimeSnapshot {
            kind,
            stress_bps: u16::try_from(stress).unwrap_or(10_000),
        }
    }
}

impl Default for RegimeClassifier {
    fn default() -> Self {
        Self::new(5, 50, 25, 7_500, 7_500)
    }
}

/// Trend/range/chop regime label.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TrendRegimeKind {
    /// Directional trend conditions.
    Trend = 1,
    /// Range-bound conditions.
    Range = 2,
    /// Choppy, reversal-prone conditions.
    Chop = 3,
}

/// Liquidity regime label.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LiquidityRegimeKind {
    /// Deep displayed liquidity.
    Deep = 1,
    /// Normal displayed liquidity.
    Normal = 2,
    /// Thin displayed liquidity.
    Thin = 3,
    /// Hidden-liquidity proxy is elevated.
    Hidden = 4,
}

/// Spread regime label.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpreadRegimeKind {
    /// Tight spread conditions.
    Tight = 1,
    /// Normal spread conditions.
    Normal = 2,
    /// Wide spread conditions.
    Wide = 3,
}

/// Session phase regime label.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SessionRegimeKind {
    /// Continuous trading phase.
    Continuous = 1,
    /// Auction or uncrossing phase.
    Auction = 2,
    /// Opening window.
    Open = 3,
    /// Closing window.
    Close = 4,
    /// News-shock or event-risk window.
    NewsShock = 5,
}

/// Composite regime classifier configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeRegimeConfig {
    trend_threshold_bps: u16,
    chop_threshold_bps: u16,
    wide_spread_bps: u32,
    tight_spread_bps: u32,
    volatile_threshold_bps: u32,
    thin_depth: i64,
    deep_depth: i64,
    open_window_ns: u64,
    close_window_ns: u64,
    news_shock_threshold_bps: u16,
    hidden_liquidity_threshold_bps: u16,
}

impl CompositeRegimeConfig {
    /// Creates composite regime configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when thresholds are
    /// inconsistent or basis-point values exceed 10,000.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        trend_threshold_bps: u16,
        chop_threshold_bps: u16,
        wide_spread_bps: u32,
        tight_spread_bps: u32,
        volatile_threshold_bps: u32,
        thin_depth: i64,
        deep_depth: i64,
        open_window_ns: u64,
        close_window_ns: u64,
        news_shock_threshold_bps: u16,
        hidden_liquidity_threshold_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if trend_threshold_bps > 10_000
            || chop_threshold_bps > 10_000
            || news_shock_threshold_bps > 10_000
            || hidden_liquidity_threshold_bps > 10_000
            || thin_depth < 0
            || deep_depth < thin_depth
            || tight_spread_bps > wide_spread_bps
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            trend_threshold_bps,
            chop_threshold_bps,
            wide_spread_bps,
            tight_spread_bps,
            volatile_threshold_bps,
            thin_depth,
            deep_depth,
            open_window_ns,
            close_window_ns,
            news_shock_threshold_bps,
            hidden_liquidity_threshold_bps,
        })
    }

    /// Returns trend threshold in basis points.
    pub const fn trend_threshold_bps(&self) -> u16 {
        self.trend_threshold_bps
    }

    /// Returns chop threshold in basis points.
    pub const fn chop_threshold_bps(&self) -> u16 {
        self.chop_threshold_bps
    }

    /// Returns wide-spread threshold in basis points.
    pub const fn wide_spread_bps(&self) -> u32 {
        self.wide_spread_bps
    }

    /// Returns tight-spread threshold in basis points.
    pub const fn tight_spread_bps(&self) -> u32 {
        self.tight_spread_bps
    }

    /// Returns volatile threshold in basis points.
    pub const fn volatile_threshold_bps(&self) -> u32 {
        self.volatile_threshold_bps
    }

    /// Returns thin-depth threshold.
    pub const fn thin_depth(&self) -> i64 {
        self.thin_depth
    }

    /// Returns deep-depth threshold.
    pub const fn deep_depth(&self) -> i64 {
        self.deep_depth
    }

    /// Returns open-window duration in nanoseconds.
    pub const fn open_window_ns(&self) -> u64 {
        self.open_window_ns
    }

    /// Returns close-window duration in nanoseconds.
    pub const fn close_window_ns(&self) -> u64 {
        self.close_window_ns
    }

    /// Returns news-shock threshold in basis points.
    pub const fn news_shock_threshold_bps(&self) -> u16 {
        self.news_shock_threshold_bps
    }

    /// Returns hidden-liquidity threshold in basis points.
    pub const fn hidden_liquidity_threshold_bps(&self) -> u16 {
        self.hidden_liquidity_threshold_bps
    }
}

impl Default for CompositeRegimeConfig {
    fn default() -> Self {
        Self {
            trend_threshold_bps: 6_000,
            chop_threshold_bps: 6_000,
            wide_spread_bps: 25,
            tight_spread_bps: 5,
            volatile_threshold_bps: 75,
            thin_depth: 100,
            deep_depth: 1_000,
            open_window_ns: 15 * 60 * 1_000_000_000,
            close_window_ns: 15 * 60 * 1_000_000_000,
            news_shock_threshold_bps: 7_000,
            hidden_liquidity_threshold_bps: 7_000,
        }
    }
}

/// Composite regime classifier input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeRegimeInput {
    trend_strength_bps: u16,
    chop_score_bps: u16,
    spread_bps: u32,
    volatility_bps: u32,
    displayed_depth: i64,
    elapsed_since_open_ns: u64,
    remaining_to_close_ns: u64,
    auction: bool,
    news_intensity_bps: u16,
    hidden_liquidity_proxy_bps: u16,
}

impl CompositeRegimeInput {
    /// Creates composite regime input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when basis-point fields exceed
    /// 10,000 or displayed depth is negative.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        trend_strength_bps: u16,
        chop_score_bps: u16,
        spread_bps: u32,
        volatility_bps: u32,
        displayed_depth: i64,
        elapsed_since_open_ns: u64,
        remaining_to_close_ns: u64,
        auction: bool,
        news_intensity_bps: u16,
        hidden_liquidity_proxy_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if trend_strength_bps > 10_000
            || chop_score_bps > 10_000
            || displayed_depth < 0
            || news_intensity_bps > 10_000
            || hidden_liquidity_proxy_bps > 10_000
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            trend_strength_bps,
            chop_score_bps,
            spread_bps,
            volatility_bps,
            displayed_depth,
            elapsed_since_open_ns,
            remaining_to_close_ns,
            auction,
            news_intensity_bps,
            hidden_liquidity_proxy_bps,
        })
    }

    /// Returns trend-strength score in basis points.
    pub const fn trend_strength_bps(&self) -> u16 {
        self.trend_strength_bps
    }

    /// Returns chop score in basis points.
    pub const fn chop_score_bps(&self) -> u16 {
        self.chop_score_bps
    }

    /// Returns spread in basis points.
    pub const fn spread_bps(&self) -> u32 {
        self.spread_bps
    }

    /// Returns volatility in basis points.
    pub const fn volatility_bps(&self) -> u32 {
        self.volatility_bps
    }

    /// Returns displayed depth.
    pub const fn displayed_depth(&self) -> i64 {
        self.displayed_depth
    }

    /// Returns elapsed time since open in nanoseconds.
    pub const fn elapsed_since_open_ns(&self) -> u64 {
        self.elapsed_since_open_ns
    }

    /// Returns remaining time to close in nanoseconds.
    pub const fn remaining_to_close_ns(&self) -> u64 {
        self.remaining_to_close_ns
    }

    /// Returns whether the instrument is in an auction phase.
    pub const fn auction(&self) -> bool {
        self.auction
    }

    /// Returns news intensity in basis points.
    pub const fn news_intensity_bps(&self) -> u16 {
        self.news_intensity_bps
    }

    /// Returns hidden-liquidity proxy in basis points.
    pub const fn hidden_liquidity_proxy_bps(&self) -> u16 {
        self.hidden_liquidity_proxy_bps
    }
}

/// Composite regime snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeRegimeSnapshot {
    trend: TrendRegimeKind,
    liquidity: LiquidityRegimeKind,
    spread: SpreadRegimeKind,
    session: SessionRegimeKind,
    volatile: bool,
    hidden_liquidity_proxy_bps: u16,
    transition_confidence_bps: u16,
}

impl CompositeRegimeSnapshot {
    /// Returns trend/range/chop label.
    pub const fn trend(&self) -> TrendRegimeKind {
        self.trend
    }

    /// Returns liquidity label.
    pub const fn liquidity(&self) -> LiquidityRegimeKind {
        self.liquidity
    }

    /// Returns spread label.
    pub const fn spread(&self) -> SpreadRegimeKind {
        self.spread
    }

    /// Returns session phase label.
    pub const fn session(&self) -> SessionRegimeKind {
        self.session
    }

    /// Returns whether volatility is elevated.
    pub const fn volatile(&self) -> bool {
        self.volatile
    }

    /// Returns hidden-liquidity proxy in basis points.
    pub const fn hidden_liquidity_proxy_bps(&self) -> u16 {
        self.hidden_liquidity_proxy_bps
    }

    /// Returns transition confidence in basis points.
    pub const fn transition_confidence_bps(&self) -> u16 {
        self.transition_confidence_bps
    }
}

/// Deterministic composite regime classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeRegimeClassifier {
    config: CompositeRegimeConfig,
}

impl CompositeRegimeClassifier {
    /// Creates a composite regime classifier.
    pub const fn new(config: CompositeRegimeConfig) -> Self {
        Self { config }
    }

    /// Returns classifier configuration.
    pub const fn config(&self) -> CompositeRegimeConfig {
        self.config
    }

    /// Classifies composite market regime.
    pub fn classify(&self, input: CompositeRegimeInput) -> CompositeRegimeSnapshot {
        let trend = if input.chop_score_bps() >= self.config.chop_threshold_bps() {
            TrendRegimeKind::Chop
        } else if input.trend_strength_bps() >= self.config.trend_threshold_bps() {
            TrendRegimeKind::Trend
        } else {
            TrendRegimeKind::Range
        };
        let liquidity =
            if input.hidden_liquidity_proxy_bps() >= self.config.hidden_liquidity_threshold_bps() {
                LiquidityRegimeKind::Hidden
            } else if input.displayed_depth() <= self.config.thin_depth() {
                LiquidityRegimeKind::Thin
            } else if input.displayed_depth() >= self.config.deep_depth() {
                LiquidityRegimeKind::Deep
            } else {
                LiquidityRegimeKind::Normal
            };
        let spread = if input.spread_bps() >= self.config.wide_spread_bps() {
            SpreadRegimeKind::Wide
        } else if input.spread_bps() <= self.config.tight_spread_bps() {
            SpreadRegimeKind::Tight
        } else {
            SpreadRegimeKind::Normal
        };
        let session = if input.news_intensity_bps() >= self.config.news_shock_threshold_bps() {
            SessionRegimeKind::NewsShock
        } else if input.auction() {
            SessionRegimeKind::Auction
        } else if input.elapsed_since_open_ns() <= self.config.open_window_ns() {
            SessionRegimeKind::Open
        } else if input.remaining_to_close_ns() <= self.config.close_window_ns() {
            SessionRegimeKind::Close
        } else {
            SessionRegimeKind::Continuous
        };
        let transition_confidence_bps =
            transition_confidence_bps(input, self.config, trend, liquidity, spread, session);
        CompositeRegimeSnapshot {
            trend,
            liquidity,
            spread,
            session,
            volatile: input.volatility_bps() >= self.config.volatile_threshold_bps(),
            hidden_liquidity_proxy_bps: input.hidden_liquidity_proxy_bps(),
            transition_confidence_bps,
        }
    }
}

impl Default for CompositeRegimeClassifier {
    fn default() -> Self {
        Self::new(CompositeRegimeConfig::default())
    }
}

/// Feed-quality degradation flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeedQualityFlags {
    bits: u32,
}

impl FeedQualityFlags {
    /// No feed-quality issues.
    pub const OK: Self = Self { bits: 0 };
    /// Sequence gap was detected.
    pub const SEQUENCE_GAP: Self = Self { bits: 1 << 0 };
    /// Sequence or event timestamp moved backward.
    pub const OUT_OF_ORDER: Self = Self { bits: 1 << 1 };
    /// Consecutive duplicate sequence was observed.
    pub const DUPLICATE: Self = Self { bits: 1 << 2 };
    /// Event was older than the configured freshness window.
    pub const STALE: Self = Self { bits: 1 << 3 };
    /// Best bid equals best ask.
    pub const LOCKED_BOOK: Self = Self { bits: 1 << 4 };
    /// Best bid is greater than best ask.
    pub const CROSSED_BOOK: Self = Self { bits: 1 << 5 };
    /// Event and receive timestamps exceeded the configured skew limit.
    pub const TIMESTAMP_SKEW: Self = Self { bits: 1 << 6 };
    /// Sequence moved backward in a way that looks like a feed reset.
    pub const SEQUENCE_RESET: Self = Self { bits: 1 << 7 };

    /// Creates flags from raw bits.
    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    /// Returns raw flag bits.
    pub const fn bits(&self) -> u32 {
        self.bits
    }

    /// Returns true when no flags are set.
    pub const fn is_ok(&self) -> bool {
        self.bits == 0
    }

    /// Returns true when `other` is present in this set.
    pub const fn contains(&self, other: Self) -> bool {
        self.bits & other.bits == other.bits
    }

    /// Adds `other` flags into this set.
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

impl Default for FeedQualityFlags {
    fn default() -> Self {
        Self::OK
    }
}

/// Feed-quality tracker configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedQualityConfig {
    stale_after_ns: u64,
    max_timestamp_skew_ns: u64,
    expected_sequence_step: u64,
}

impl FeedQualityConfig {
    /// Creates feed-quality configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQuality`] when sequence step is zero.
    pub const fn new(
        stale_after_ns: u64,
        max_timestamp_skew_ns: u64,
        expected_sequence_step: u64,
    ) -> Result<Self, AnalyticsError> {
        if expected_sequence_step == 0 {
            return Err(AnalyticsError::InvalidQuality);
        }
        Ok(Self {
            stale_after_ns,
            max_timestamp_skew_ns,
            expected_sequence_step,
        })
    }

    /// Returns freshness threshold in nanoseconds.
    pub const fn stale_after_ns(&self) -> u64 {
        self.stale_after_ns
    }

    /// Returns maximum event/receive timestamp skew in nanoseconds.
    pub const fn max_timestamp_skew_ns(&self) -> u64 {
        self.max_timestamp_skew_ns
    }

    /// Returns expected sequence increment.
    pub const fn expected_sequence_step(&self) -> u64 {
        self.expected_sequence_step
    }
}

impl Default for FeedQualityConfig {
    fn default() -> Self {
        Self {
            stale_after_ns: 1_000_000_000,
            max_timestamp_skew_ns: 100_000_000,
            expected_sequence_step: 1,
        }
    }
}

/// Market-data event context used by feed-quality analytics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedQualityEvent {
    sequence: Option<u64>,
    event_ts_ns: u64,
    receive_ts_ns: u64,
    bid_price: Option<i64>,
    ask_price: Option<i64>,
}

impl FeedQualityEvent {
    /// Creates feed-quality event context.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQuality`] when timestamps are zero or
    /// supplied quote prices are non-positive.
    pub const fn new(
        sequence: Option<u64>,
        event_ts_ns: u64,
        receive_ts_ns: u64,
        bid_price: Option<i64>,
        ask_price: Option<i64>,
    ) -> Result<Self, AnalyticsError> {
        if event_ts_ns == 0 || receive_ts_ns == 0 {
            return Err(AnalyticsError::InvalidQuality);
        }
        if let Some(price) = bid_price {
            if price <= 0 {
                return Err(AnalyticsError::InvalidQuality);
            }
        }
        if let Some(price) = ask_price {
            if price <= 0 {
                return Err(AnalyticsError::InvalidQuality);
            }
        }
        Ok(Self {
            sequence,
            event_ts_ns,
            receive_ts_ns,
            bid_price,
            ask_price,
        })
    }

    /// Creates feed-quality context from a quote.
    pub const fn from_quote(
        sequence: Option<u64>,
        quote: QuoteContext,
        receive_ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        Self::new(
            sequence,
            quote.ts_ns(),
            receive_ts_ns,
            Some(quote.bid_price()),
            Some(quote.ask_price()),
        )
    }

    /// Returns optional venue/provider sequence.
    pub const fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    /// Returns event timestamp.
    pub const fn event_ts_ns(&self) -> u64 {
        self.event_ts_ns
    }

    /// Returns local receive timestamp.
    pub const fn receive_ts_ns(&self) -> u64 {
        self.receive_ts_ns
    }

    /// Returns optional best bid price.
    pub const fn bid_price(&self) -> Option<i64> {
        self.bid_price
    }

    /// Returns optional best ask price.
    pub const fn ask_price(&self) -> Option<i64> {
        self.ask_price
    }
}

/// Cumulative feed-quality snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedQualitySnapshot {
    events: u64,
    sequence_gap_events: u64,
    sequence_gap_units: u64,
    out_of_order_events: u64,
    duplicate_events: u64,
    stale_events: u64,
    locked_book_events: u64,
    crossed_book_events: u64,
    timestamp_skew_events: u64,
    sequence_reset_events: u64,
    last_sequence: Option<u64>,
    last_event_ts_ns: u64,
    flags: FeedQualityFlags,
    health_score_bps: u16,
}

impl FeedQualitySnapshot {
    /// Returns observed event count.
    pub const fn events(&self) -> u64 {
        self.events
    }

    /// Returns sequence gap event count.
    pub const fn sequence_gap_events(&self) -> u64 {
        self.sequence_gap_events
    }

    /// Returns total missing sequence units.
    pub const fn sequence_gap_units(&self) -> u64 {
        self.sequence_gap_units
    }

    /// Returns out-of-order event count.
    pub const fn out_of_order_events(&self) -> u64 {
        self.out_of_order_events
    }

    /// Returns duplicate event count.
    pub const fn duplicate_events(&self) -> u64 {
        self.duplicate_events
    }

    /// Returns stale event count.
    pub const fn stale_events(&self) -> u64 {
        self.stale_events
    }

    /// Returns locked-book event count.
    pub const fn locked_book_events(&self) -> u64 {
        self.locked_book_events
    }

    /// Returns crossed-book event count.
    pub const fn crossed_book_events(&self) -> u64 {
        self.crossed_book_events
    }

    /// Returns timestamp-skew event count.
    pub const fn timestamp_skew_events(&self) -> u64 {
        self.timestamp_skew_events
    }

    /// Returns sequence-reset event count.
    pub const fn sequence_reset_events(&self) -> u64 {
        self.sequence_reset_events
    }

    /// Returns latest accepted sequence.
    pub const fn last_sequence(&self) -> Option<u64> {
        self.last_sequence
    }

    /// Returns latest event timestamp.
    pub const fn last_event_ts_ns(&self) -> u64 {
        self.last_event_ts_ns
    }

    /// Returns cumulative degradation flags.
    pub const fn flags(&self) -> FeedQualityFlags {
        self.flags
    }

    /// Returns aggregate health score in basis points, where 10,000 is best.
    pub const fn health_score_bps(&self) -> u16 {
        self.health_score_bps
    }

    /// Returns sequence gap event rate in basis points.
    pub fn sequence_gap_rate_bps(&self) -> u16 {
        rate_bps(self.sequence_gap_events, self.events)
    }

    /// Returns out-of-order event rate in basis points.
    pub fn out_of_order_rate_bps(&self) -> u16 {
        rate_bps(self.out_of_order_events, self.events)
    }

    /// Returns duplicate event rate in basis points.
    pub fn duplicate_rate_bps(&self) -> u16 {
        rate_bps(self.duplicate_events, self.events)
    }

    /// Returns stale event rate in basis points.
    pub fn stale_rate_bps(&self) -> u16 {
        rate_bps(self.stale_events, self.events)
    }

    /// Returns bad top-of-book rate in basis points.
    pub fn bad_book_rate_bps(&self) -> u16 {
        rate_bps(
            self.locked_book_events
                .saturating_add(self.crossed_book_events),
            self.events,
        )
    }
}

/// Allocation-free feed-quality tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedQualityTracker {
    config: FeedQualityConfig,
    events: u64,
    sequence_gap_events: u64,
    sequence_gap_units: u64,
    out_of_order_events: u64,
    duplicate_events: u64,
    stale_events: u64,
    locked_book_events: u64,
    crossed_book_events: u64,
    timestamp_skew_events: u64,
    sequence_reset_events: u64,
    last_sequence: Option<u64>,
    last_event_ts_ns: u64,
    flags: FeedQualityFlags,
}

impl FeedQualityTracker {
    /// Creates a feed-quality tracker.
    pub const fn new(config: FeedQualityConfig) -> Self {
        Self {
            config,
            events: 0,
            sequence_gap_events: 0,
            sequence_gap_units: 0,
            out_of_order_events: 0,
            duplicate_events: 0,
            stale_events: 0,
            locked_book_events: 0,
            crossed_book_events: 0,
            timestamp_skew_events: 0,
            sequence_reset_events: 0,
            last_sequence: None,
            last_event_ts_ns: 0,
            flags: FeedQualityFlags::OK,
        }
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> FeedQualityConfig {
        self.config
    }

    /// Records one market-data event and returns flags for that event.
    pub fn on_event(&mut self, event: FeedQualityEvent) -> FeedQualityFlags {
        self.events = self.events.saturating_add(1);
        let mut event_flags = FeedQualityFlags::OK;
        event_flags = self.observe_sequence(event.sequence(), event_flags);
        event_flags =
            self.observe_timestamps(event.event_ts_ns(), event.receive_ts_ns(), event_flags);
        event_flags = self.observe_book(event.bid_price(), event.ask_price(), event_flags);
        self.flags = self.flags.union(event_flags);
        event_flags
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> FeedQualitySnapshot {
        FeedQualitySnapshot {
            events: self.events,
            sequence_gap_events: self.sequence_gap_events,
            sequence_gap_units: self.sequence_gap_units,
            out_of_order_events: self.out_of_order_events,
            duplicate_events: self.duplicate_events,
            stale_events: self.stale_events,
            locked_book_events: self.locked_book_events,
            crossed_book_events: self.crossed_book_events,
            timestamp_skew_events: self.timestamp_skew_events,
            sequence_reset_events: self.sequence_reset_events,
            last_sequence: self.last_sequence,
            last_event_ts_ns: self.last_event_ts_ns,
            flags: self.flags,
            health_score_bps: self.health_score_bps(),
        }
    }

    /// Clears accumulated counters and last-seen state.
    pub fn reset(&mut self) {
        self.events = 0;
        self.sequence_gap_events = 0;
        self.sequence_gap_units = 0;
        self.out_of_order_events = 0;
        self.duplicate_events = 0;
        self.stale_events = 0;
        self.locked_book_events = 0;
        self.crossed_book_events = 0;
        self.timestamp_skew_events = 0;
        self.sequence_reset_events = 0;
        self.last_sequence = None;
        self.last_event_ts_ns = 0;
        self.flags = FeedQualityFlags::OK;
    }

    fn observe_sequence(
        &mut self,
        sequence: Option<u64>,
        mut flags: FeedQualityFlags,
    ) -> FeedQualityFlags {
        if let Some(sequence) = sequence {
            match self.last_sequence {
                Some(last) if sequence == last => {
                    self.duplicate_events = self.duplicate_events.saturating_add(1);
                    flags = flags.union(FeedQualityFlags::DUPLICATE);
                }
                Some(last) if sequence < last => {
                    if sequence <= self.config.expected_sequence_step() {
                        self.sequence_reset_events = self.sequence_reset_events.saturating_add(1);
                        flags = flags.union(FeedQualityFlags::SEQUENCE_RESET);
                        self.last_sequence = Some(sequence);
                    } else {
                        self.out_of_order_events = self.out_of_order_events.saturating_add(1);
                        flags = flags.union(FeedQualityFlags::OUT_OF_ORDER);
                    }
                }
                Some(last) => {
                    let expected = last.saturating_add(self.config.expected_sequence_step());
                    if sequence > expected {
                        self.sequence_gap_events = self.sequence_gap_events.saturating_add(1);
                        self.sequence_gap_units = self
                            .sequence_gap_units
                            .saturating_add(sequence.saturating_sub(expected));
                        flags = flags.union(FeedQualityFlags::SEQUENCE_GAP);
                    }
                    self.last_sequence = Some(sequence);
                }
                None => self.last_sequence = Some(sequence),
            }
        }
        flags
    }

    fn observe_timestamps(
        &mut self,
        event_ts_ns: u64,
        receive_ts_ns: u64,
        mut flags: FeedQualityFlags,
    ) -> FeedQualityFlags {
        if self.last_event_ts_ns != 0 && event_ts_ns < self.last_event_ts_ns {
            self.out_of_order_events = self.out_of_order_events.saturating_add(1);
            flags = flags.union(FeedQualityFlags::OUT_OF_ORDER);
        }
        if receive_ts_ns.saturating_sub(event_ts_ns) > self.config.stale_after_ns() {
            self.stale_events = self.stale_events.saturating_add(1);
            flags = flags.union(FeedQualityFlags::STALE);
        }
        if receive_ts_ns.abs_diff(event_ts_ns) > self.config.max_timestamp_skew_ns() {
            self.timestamp_skew_events = self.timestamp_skew_events.saturating_add(1);
            flags = flags.union(FeedQualityFlags::TIMESTAMP_SKEW);
        }
        self.last_event_ts_ns = self.last_event_ts_ns.max(event_ts_ns);
        flags
    }

    fn observe_book(
        &mut self,
        bid_price: Option<i64>,
        ask_price: Option<i64>,
        mut flags: FeedQualityFlags,
    ) -> FeedQualityFlags {
        if let (Some(bid), Some(ask)) = (bid_price, ask_price) {
            if bid > ask {
                self.crossed_book_events = self.crossed_book_events.saturating_add(1);
                flags = flags.union(FeedQualityFlags::CROSSED_BOOK);
            } else if bid == ask {
                self.locked_book_events = self.locked_book_events.saturating_add(1);
                flags = flags.union(FeedQualityFlags::LOCKED_BOOK);
            }
        }
        flags
    }

    fn health_score_bps(&self) -> u16 {
        if self.events == 0 {
            return 10_000;
        }
        let weighted_penalty = self
            .sequence_gap_events
            .saturating_mul(1_000)
            .saturating_add(self.out_of_order_events.saturating_mul(1_500))
            .saturating_add(self.duplicate_events.saturating_mul(500))
            .saturating_add(self.stale_events.saturating_mul(800))
            .saturating_add(self.locked_book_events.saturating_mul(600))
            .saturating_add(self.crossed_book_events.saturating_mul(2_000))
            .saturating_add(self.timestamp_skew_events.saturating_mul(1_000))
            .saturating_add(self.sequence_reset_events.saturating_mul(1_500));
        let penalty_bps = weighted_penalty / self.events;
        u16::try_from(10_000_u64.saturating_sub(penalty_bps.min(10_000))).unwrap_or(0)
    }
}

impl Default for FeedQualityTracker {
    fn default() -> Self {
        Self::new(FeedQualityConfig::default())
    }
}

/// Stable feature identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureId {
    raw: u32,
}

impl FeatureId {
    /// Creates a feature id.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidFeature`] when id is zero.
    pub const fn new(raw: u32) -> Result<Self, AnalyticsError> {
        if raw == 0 {
            return Err(AnalyticsError::InvalidFeature);
        }
        Ok(Self { raw })
    }

    /// Returns raw feature id.
    pub const fn raw(&self) -> u32 {
        self.raw
    }
}

/// Feature value unit.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FeatureUnit {
    /// Unitless raw value.
    Raw = 0,
    /// Integer-normalized price units.
    Price = 1,
    /// Quantity units.
    Quantity = 2,
    /// Basis points.
    BasisPoints = 3,
    /// Parts per million.
    PartsPerMillion = 4,
    /// Boolean encoded as zero or one.
    Boolean = 5,
    /// Nanoseconds.
    Nanoseconds = 6,
    /// Score in basis points, usually 0..=10,000.
    ScoreBasisPoints = 7,
}

impl FeatureUnit {
    const fn code(self) -> u8 {
        match self {
            Self::Raw => 0,
            Self::Price => 1,
            Self::Quantity => 2,
            Self::BasisPoints => 3,
            Self::PartsPerMillion => 4,
            Self::Boolean => 5,
            Self::Nanoseconds => 6,
            Self::ScoreBasisPoints => 7,
        }
    }
}

/// Per-feature extraction quality.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FeatureQuality {
    /// Feature value is present and usable.
    Good = 0,
    /// Feature value is missing.
    Missing = 1,
    /// Feature was extracted from stale input.
    Stale = 2,
    /// Feature was extracted from degraded input.
    Degraded = 3,
    /// Feature value failed validation.
    Invalid = 4,
}

/// Missing-feature fill policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MissingValuePolicy {
    /// Fill missing integer feature values with zero.
    Zero,
    /// Fill missing values with an explicit sentinel.
    Sentinel(i64),
    /// Reuse the last known value when the host maintains one.
    LastKnown,
}

impl MissingValuePolicy {
    /// Returns deterministic integer fill value for this policy.
    pub const fn fill_value(&self) -> i64 {
        match self {
            Self::Zero | Self::LastKnown => 0,
            Self::Sentinel(value) => *value,
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::Sentinel(_) => 1,
            Self::LastKnown => 2,
        }
    }
}

/// Feature definition inside a stable schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureDefinition {
    id: FeatureId,
    name: &'static str,
    unit: FeatureUnit,
    scale: i32,
    missing_policy: MissingValuePolicy,
}

impl FeatureDefinition {
    /// Creates a feature definition.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidFeature`] when the name is empty.
    pub const fn new(
        id: FeatureId,
        name: &'static str,
        unit: FeatureUnit,
        scale: i32,
        missing_policy: MissingValuePolicy,
    ) -> Result<Self, AnalyticsError> {
        if name.is_empty() {
            return Err(AnalyticsError::InvalidFeature);
        }
        Ok(Self {
            id,
            name,
            unit,
            scale,
            missing_policy,
        })
    }

    /// Returns feature id.
    pub const fn id(&self) -> FeatureId {
        self.id
    }

    /// Returns stable feature name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns feature unit.
    pub const fn unit(&self) -> FeatureUnit {
        self.unit
    }

    /// Returns feature scale.
    pub const fn scale(&self) -> i32 {
        self.scale
    }

    /// Returns missing-value policy.
    pub const fn missing_policy(&self) -> MissingValuePolicy {
        self.missing_policy
    }
}

/// Fixed-capacity feature schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureSchema<const N: usize> {
    definitions: [Option<FeatureDefinition>; N],
    len: usize,
}

impl<const N: usize> FeatureSchema<N> {
    /// Creates an empty feature schema.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidFeature`] when capacity is zero.
    pub const fn new() -> Result<Self, AnalyticsError> {
        if N == 0 {
            return Err(AnalyticsError::InvalidFeature);
        }
        Ok(Self {
            definitions: [None; N],
            len: 0,
        })
    }

    /// Registers a feature definition at the next stable index.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidFeature`] when capacity is full or the
    /// id is already registered.
    pub fn register(&mut self, definition: FeatureDefinition) -> Result<usize, AnalyticsError> {
        if self.len >= N || self.contains_id(definition.id()) {
            return Err(AnalyticsError::InvalidFeature);
        }
        let index = self.len;
        self.definitions[index] = Some(definition);
        self.len += 1;
        Ok(index)
    }

    /// Returns registered feature count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no features are registered.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns definition by stable index.
    pub const fn definition(&self, index: usize) -> Option<FeatureDefinition> {
        if index >= self.len {
            None
        } else {
            self.definitions[index]
        }
    }

    /// Returns true when a feature id is registered.
    pub fn contains_id(&self, id: FeatureId) -> bool {
        self.index_of(id).is_some()
    }

    /// Returns stable index for a feature id.
    pub fn index_of(&self, id: FeatureId) -> Option<usize> {
        for index in 0..self.len {
            if let Some(definition) = self.definitions[index] {
                if definition.id() == id {
                    return Some(index);
                }
            }
        }
        None
    }

    /// Returns deterministic schema hash.
    pub fn schema_hash(&self) -> u64 {
        let mut hash = fnv1a64(0xcbf2_9ce4_8422_2325, &(self.len as u64).to_le_bytes());
        for index in 0..self.len {
            if let Some(definition) = self.definitions[index] {
                hash = hash_feature_definition(hash, definition);
            }
        }
        hash
    }
}

/// Fixed-capacity feature registry alias.
pub type FeatureRegistry<const N: usize> = FeatureSchema<N>;

/// Completed fixed-capacity feature vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureVector<const N: usize> {
    values: [i64; N],
    qualities: [FeatureQuality; N],
    len: usize,
    schema_hash: u64,
}

impl<const N: usize> FeatureVector<N> {
    /// Returns feature values in schema order.
    pub fn values(&self) -> &[i64] {
        &self.values[..self.len]
    }

    /// Returns feature qualities in schema order.
    pub fn qualities(&self) -> &[FeatureQuality] {
        &self.qualities[..self.len]
    }

    /// Returns feature count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when the vector is empty.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns schema hash used to produce this vector.
    pub const fn schema_hash(&self) -> u64 {
        self.schema_hash
    }

    /// Returns feature value by stable index.
    pub fn value(&self, index: usize) -> Option<i64> {
        self.values().get(index).copied()
    }

    /// Returns feature quality by stable index.
    pub fn quality(&self, index: usize) -> Option<FeatureQuality> {
        self.qualities().get(index).copied()
    }
}

/// Reusable fixed-capacity feature-vector writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureVectorWriter<const N: usize> {
    values: [i64; N],
    qualities: [FeatureQuality; N],
    len: usize,
    schema_hash: u64,
}

impl<const N: usize> FeatureVectorWriter<N> {
    /// Creates a writer initialized from a schema.
    pub fn new(schema: &FeatureSchema<N>) -> Self {
        let mut writer = Self {
            values: [0; N],
            qualities: [FeatureQuality::Missing; N],
            len: schema.len(),
            schema_hash: schema.schema_hash(),
        };
        writer.reset(schema);
        writer
    }

    /// Resets the writer from schema missing-value policies.
    pub fn reset(&mut self, schema: &FeatureSchema<N>) {
        self.len = schema.len();
        self.schema_hash = schema.schema_hash();
        for index in 0..self.len {
            if let Some(definition) = schema.definition(index) {
                self.values[index] = definition.missing_policy().fill_value();
                self.qualities[index] = FeatureQuality::Missing;
            }
        }
    }

    /// Sets a feature value by stable index.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidFeature`] when index is outside the
    /// active schema length.
    pub fn set(
        &mut self,
        index: usize,
        value: i64,
        quality: FeatureQuality,
    ) -> Result<(), AnalyticsError> {
        if index >= self.len {
            return Err(AnalyticsError::InvalidFeature);
        }
        self.values[index] = value;
        self.qualities[index] = quality;
        Ok(())
    }

    /// Finishes the current vector.
    pub const fn finish(&self) -> FeatureVector<N> {
        FeatureVector {
            values: self.values,
            qualities: self.qualities,
            len: self.len,
            schema_hash: self.schema_hash,
        }
    }
}

/// Feature extractor contract.
pub trait FeatureExtractor<const N: usize> {
    /// Input consumed by the extractor.
    type Input;

    /// Returns extractor schema.
    fn schema(&self) -> &FeatureSchema<N>;

    /// Extracts a feature vector into a caller-owned writer.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError`] when the input cannot be converted into a
    /// valid vector.
    fn extract(
        &mut self,
        input: Self::Input,
        writer: &mut FeatureVectorWriter<N>,
    ) -> Result<FeatureVector<N>, AnalyticsError>;
}

/// Liquidity resiliency sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResiliencySample {
    ts_ns: u64,
    spread_bps: u32,
    bid_depth: i64,
    ask_depth: i64,
}

impl ResiliencySample {
    /// Creates a resiliency sample.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidResiliency`] when timestamp is zero or
    /// depth is negative.
    pub const fn new(
        ts_ns: u64,
        spread_bps: u32,
        bid_depth: i64,
        ask_depth: i64,
    ) -> Result<Self, AnalyticsError> {
        if ts_ns == 0 || bid_depth < 0 || ask_depth < 0 {
            return Err(AnalyticsError::InvalidResiliency);
        }
        Ok(Self {
            ts_ns,
            spread_bps,
            bid_depth,
            ask_depth,
        })
    }

    /// Returns sample timestamp.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }

    /// Returns spread in basis points.
    pub const fn spread_bps(&self) -> u32 {
        self.spread_bps
    }

    /// Returns bid-side depth.
    pub const fn bid_depth(&self) -> i64 {
        self.bid_depth
    }

    /// Returns ask-side depth.
    pub const fn ask_depth(&self) -> i64 {
        self.ask_depth
    }

    /// Returns total displayed depth.
    pub const fn total_depth(&self) -> i64 {
        self.bid_depth + self.ask_depth
    }
}

/// Liquidity resiliency thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResiliencyConfig {
    baseline_spread_bps: u32,
    baseline_depth: i64,
    shock_spread_bps: u32,
    depth_floor_bps: u16,
    recovery_spread_bps: u32,
    recovery_depth_bps: u16,
}

impl ResiliencyConfig {
    /// Creates resiliency thresholds.
    ///
    /// `depth_floor_bps` and `recovery_depth_bps` are percentages of baseline
    /// depth in basis points.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidResiliency`] when depth is non-positive
    /// or percentage thresholds are above 10,000.
    pub const fn new(
        baseline_spread_bps: u32,
        baseline_depth: i64,
        shock_spread_bps: u32,
        depth_floor_bps: u16,
        recovery_spread_bps: u32,
        recovery_depth_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if baseline_depth <= 0 || depth_floor_bps > 10_000 || recovery_depth_bps > 10_000 {
            return Err(AnalyticsError::InvalidResiliency);
        }
        Ok(Self {
            baseline_spread_bps,
            baseline_depth,
            shock_spread_bps,
            depth_floor_bps,
            recovery_spread_bps,
            recovery_depth_bps,
        })
    }

    /// Returns baseline spread in basis points.
    pub const fn baseline_spread_bps(&self) -> u32 {
        self.baseline_spread_bps
    }

    /// Returns baseline total depth.
    pub const fn baseline_depth(&self) -> i64 {
        self.baseline_depth
    }

    /// Returns spread threshold that starts a shock.
    pub const fn shock_spread_bps(&self) -> u32 {
        self.shock_spread_bps
    }

    /// Returns depth floor threshold as basis points of baseline depth.
    pub const fn depth_floor_bps(&self) -> u16 {
        self.depth_floor_bps
    }

    /// Returns spread threshold that allows recovery.
    pub const fn recovery_spread_bps(&self) -> u32 {
        self.recovery_spread_bps
    }

    /// Returns recovery depth threshold as basis points of baseline depth.
    pub const fn recovery_depth_bps(&self) -> u16 {
        self.recovery_depth_bps
    }

    const fn depth_floor_qty(&self) -> i64 {
        ((self.baseline_depth as i128 * self.depth_floor_bps as i128) / 10_000) as i64
    }

    const fn recovery_depth_qty(&self) -> i64 {
        ((self.baseline_depth as i128 * self.recovery_depth_bps as i128) / 10_000) as i64
    }
}

/// Liquidity resiliency snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResiliencySnapshot {
    samples: u64,
    shock_count: u64,
    recovery_count: u64,
    active_shock: bool,
    last_shock_ts_ns: u64,
    last_recovery_ts_ns: u64,
    last_recovery_time_ns: u64,
    max_spread_bps: u32,
    min_depth: i64,
    score_bps: u16,
}

impl ResiliencySnapshot {
    /// Returns sample count.
    pub const fn samples(&self) -> u64 {
        self.samples
    }

    /// Returns shock count.
    pub const fn shock_count(&self) -> u64 {
        self.shock_count
    }

    /// Returns recovery count.
    pub const fn recovery_count(&self) -> u64 {
        self.recovery_count
    }

    /// Returns true when a shock is active.
    pub const fn active_shock(&self) -> bool {
        self.active_shock
    }

    /// Returns latest shock timestamp.
    pub const fn last_shock_ts_ns(&self) -> u64 {
        self.last_shock_ts_ns
    }

    /// Returns latest recovery timestamp.
    pub const fn last_recovery_ts_ns(&self) -> u64 {
        self.last_recovery_ts_ns
    }

    /// Returns latest recovery duration in nanoseconds.
    pub const fn last_recovery_time_ns(&self) -> u64 {
        self.last_recovery_time_ns
    }

    /// Returns maximum spread observed during the active or latest shock.
    pub const fn max_spread_bps(&self) -> u32 {
        self.max_spread_bps
    }

    /// Returns minimum total depth observed during the active or latest shock.
    pub const fn min_depth(&self) -> i64 {
        self.min_depth
    }

    /// Returns resiliency score in basis points, where 10,000 is best.
    pub const fn score_bps(&self) -> u16 {
        self.score_bps
    }
}

/// Threshold-based liquidity resiliency tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResiliencyTracker {
    config: ResiliencyConfig,
    samples: u64,
    shock_count: u64,
    recovery_count: u64,
    active_shock: bool,
    current_shock_ts_ns: u64,
    last_shock_ts_ns: u64,
    last_recovery_ts_ns: u64,
    last_recovery_time_ns: u64,
    max_spread_bps: u32,
    min_depth: i64,
}

impl ResiliencyTracker {
    /// Creates a resiliency tracker.
    pub const fn new(config: ResiliencyConfig) -> Self {
        Self {
            config,
            samples: 0,
            shock_count: 0,
            recovery_count: 0,
            active_shock: false,
            current_shock_ts_ns: 0,
            last_shock_ts_ns: 0,
            last_recovery_ts_ns: 0,
            last_recovery_time_ns: 0,
            max_spread_bps: 0,
            min_depth: i64::MAX,
        }
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> ResiliencyConfig {
        self.config
    }

    /// Records one spread/depth sample.
    pub fn on_sample(&mut self, sample: ResiliencySample) -> ResiliencySnapshot {
        self.samples = self.samples.saturating_add(1);
        let total_depth = sample.total_depth();
        if self.active_shock {
            self.max_spread_bps = self.max_spread_bps.max(sample.spread_bps());
            self.min_depth = self.min_depth.min(total_depth);
            if self.is_recovered(sample) {
                self.active_shock = false;
                self.recovery_count = self.recovery_count.saturating_add(1);
                self.last_recovery_ts_ns = sample.ts_ns();
                self.last_recovery_time_ns =
                    sample.ts_ns().saturating_sub(self.current_shock_ts_ns);
            }
        } else if self.is_shock(sample) {
            self.active_shock = true;
            self.shock_count = self.shock_count.saturating_add(1);
            self.current_shock_ts_ns = sample.ts_ns();
            self.last_shock_ts_ns = sample.ts_ns();
            self.max_spread_bps = sample.spread_bps();
            self.min_depth = total_depth;
        }
        self.snapshot()
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> ResiliencySnapshot {
        ResiliencySnapshot {
            samples: self.samples,
            shock_count: self.shock_count,
            recovery_count: self.recovery_count,
            active_shock: self.active_shock,
            last_shock_ts_ns: self.last_shock_ts_ns,
            last_recovery_ts_ns: self.last_recovery_ts_ns,
            last_recovery_time_ns: self.last_recovery_time_ns,
            max_spread_bps: self.max_spread_bps,
            min_depth: if self.min_depth == i64::MAX {
                0
            } else {
                self.min_depth
            },
            score_bps: self.score_bps(),
        }
    }

    /// Clears accumulated state.
    pub fn reset(&mut self) {
        self.samples = 0;
        self.shock_count = 0;
        self.recovery_count = 0;
        self.active_shock = false;
        self.current_shock_ts_ns = 0;
        self.last_shock_ts_ns = 0;
        self.last_recovery_ts_ns = 0;
        self.last_recovery_time_ns = 0;
        self.max_spread_bps = 0;
        self.min_depth = i64::MAX;
    }

    fn is_shock(&self, sample: ResiliencySample) -> bool {
        sample.spread_bps() >= self.config.shock_spread_bps()
            || sample.total_depth() <= self.config.depth_floor_qty()
    }

    fn is_recovered(&self, sample: ResiliencySample) -> bool {
        sample.spread_bps() <= self.config.recovery_spread_bps()
            && sample.total_depth() >= self.config.recovery_depth_qty()
    }

    fn score_bps(&self) -> u16 {
        if self.shock_count == 0 {
            return 10_000;
        }
        if self.active_shock {
            return 5_000;
        }
        let recovery_rate = rate_bps(self.recovery_count, self.shock_count);
        let latency_penalty = (self.last_recovery_time_ns / 1_000_000).min(5_000);
        u16::try_from(u64::from(recovery_rate).saturating_sub(latency_penalty)).unwrap_or(0)
    }
}

/// Queue update kind.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum QueueUpdateKind {
    /// Aggressive trade consumed displayed quantity at the order price.
    Trade = 1,
    /// Displayed quantity decreased without a known trade.
    Cancel = 2,
    /// Local order was amended and likely lost queue priority.
    Amend = 3,
}

/// Passive order queue estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuePositionEstimate {
    qty_ahead: i64,
    own_qty: i64,
    total_queue_qty: i64,
    ts_ns: u64,
}

impl QueuePositionEstimate {
    /// Creates a queue position estimate.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQueue`] when quantities or timestamp
    /// are invalid.
    pub const fn new(
        qty_ahead: i64,
        own_qty: i64,
        total_queue_qty: i64,
        ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if qty_ahead < 0 || own_qty <= 0 || total_queue_qty < qty_ahead || ts_ns == 0 {
            return Err(AnalyticsError::InvalidQueue);
        }
        Ok(Self {
            qty_ahead,
            own_qty,
            total_queue_qty,
            ts_ns,
        })
    }

    /// Returns estimated quantity ahead.
    pub const fn qty_ahead(&self) -> i64 {
        self.qty_ahead
    }

    /// Returns local displayed quantity.
    pub const fn own_qty(&self) -> i64 {
        self.own_qty
    }

    /// Returns total displayed queue quantity at the price level.
    pub const fn total_queue_qty(&self) -> i64 {
        self.total_queue_qty
    }

    /// Returns estimate timestamp.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }
}

/// Queue/fill probability configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFillConfig {
    cancel_ahead_bps: u16,
    expected_depletion_per_sec: i64,
    horizon_ns: u64,
}

impl QueueFillConfig {
    /// Creates queue/fill configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQueue`] when basis points exceed
    /// 10,000, expected depletion is negative, or horizon is zero.
    pub const fn new(
        cancel_ahead_bps: u16,
        expected_depletion_per_sec: i64,
        horizon_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if cancel_ahead_bps > 10_000 || expected_depletion_per_sec < 0 || horizon_ns == 0 {
            return Err(AnalyticsError::InvalidQueue);
        }
        Ok(Self {
            cancel_ahead_bps,
            expected_depletion_per_sec,
            horizon_ns,
        })
    }

    /// Returns assumed percentage of cancels ahead of the local order.
    pub const fn cancel_ahead_bps(&self) -> u16 {
        self.cancel_ahead_bps
    }

    /// Returns expected queue depletion per second.
    pub const fn expected_depletion_per_sec(&self) -> i64 {
        self.expected_depletion_per_sec
    }

    /// Returns fill-probability horizon.
    pub const fn horizon_ns(&self) -> u64 {
        self.horizon_ns
    }
}

/// Queue update at the local order price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFillUpdate {
    kind: QueueUpdateKind,
    qty: i64,
    total_queue_qty: i64,
    ts_ns: u64,
}

impl QueueFillUpdate {
    /// Creates a queue update.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQueue`] when quantity, total queue, or
    /// timestamp is invalid.
    pub const fn new(
        kind: QueueUpdateKind,
        qty: i64,
        total_queue_qty: i64,
        ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if qty < 0 || total_queue_qty < 0 || ts_ns == 0 {
            return Err(AnalyticsError::InvalidQueue);
        }
        Ok(Self {
            kind,
            qty,
            total_queue_qty,
            ts_ns,
        })
    }

    /// Returns update kind.
    pub const fn kind(&self) -> QueueUpdateKind {
        self.kind
    }

    /// Returns update quantity.
    pub const fn qty(&self) -> i64 {
        self.qty
    }

    /// Returns current total queue quantity.
    pub const fn total_queue_qty(&self) -> i64 {
        self.total_queue_qty
    }

    /// Returns update timestamp.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }
}

/// Passive fill probability snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFillSnapshot {
    qty_ahead: i64,
    own_qty_remaining: i64,
    total_queue_qty: i64,
    fill_probability_bps: u16,
    expected_time_to_fill_ns: u64,
    queue_loss_after_amend: i64,
    maker_taker_score_bps: u16,
    top_level_survival_bps: u16,
    last_update_ts_ns: u64,
}

impl QueueFillSnapshot {
    /// Returns estimated quantity ahead.
    pub const fn qty_ahead(&self) -> i64 {
        self.qty_ahead
    }

    /// Returns remaining local quantity.
    pub const fn own_qty_remaining(&self) -> i64 {
        self.own_qty_remaining
    }

    /// Returns current total queue quantity.
    pub const fn total_queue_qty(&self) -> i64 {
        self.total_queue_qty
    }

    /// Returns fill probability over configured horizon in basis points.
    pub const fn fill_probability_bps(&self) -> u16 {
        self.fill_probability_bps
    }

    /// Returns expected time to fill in nanoseconds.
    pub const fn expected_time_to_fill_ns(&self) -> u64 {
        self.expected_time_to_fill_ns
    }

    /// Returns estimated queue quantity lost after amend.
    pub const fn queue_loss_after_amend(&self) -> i64 {
        self.queue_loss_after_amend
    }

    /// Returns maker/taker preference score in basis points.
    pub const fn maker_taker_score_bps(&self) -> u16 {
        self.maker_taker_score_bps
    }

    /// Returns top-level survival probability proxy in basis points.
    pub const fn top_level_survival_bps(&self) -> u16 {
        self.top_level_survival_bps
    }

    /// Returns latest update timestamp.
    pub const fn last_update_ts_ns(&self) -> u64 {
        self.last_update_ts_ns
    }
}

/// Queue/fill probability tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFillTracker {
    config: QueueFillConfig,
    qty_ahead: i64,
    own_qty_remaining: i64,
    total_queue_qty: i64,
    queue_loss_after_amend: i64,
    last_update_ts_ns: u64,
}

impl QueueFillTracker {
    /// Creates a queue/fill tracker.
    pub const fn new(config: QueueFillConfig, estimate: QueuePositionEstimate) -> Self {
        Self {
            config,
            qty_ahead: estimate.qty_ahead(),
            own_qty_remaining: estimate.own_qty(),
            total_queue_qty: estimate.total_queue_qty(),
            queue_loss_after_amend: 0,
            last_update_ts_ns: estimate.ts_ns(),
        }
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> QueueFillConfig {
        self.config
    }

    /// Records one queue update.
    pub fn on_update(&mut self, update: QueueFillUpdate) -> QueueFillSnapshot {
        match update.kind() {
            QueueUpdateKind::Trade => self.apply_trade(update.qty()),
            QueueUpdateKind::Cancel => self.apply_cancel(update.qty()),
            QueueUpdateKind::Amend => self.apply_amend(update.total_queue_qty()),
        }
        self.total_queue_qty = update.total_queue_qty();
        self.last_update_ts_ns = update.ts_ns();
        self.snapshot()
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> QueueFillSnapshot {
        let fill_probability_bps = self.fill_probability_bps();
        QueueFillSnapshot {
            qty_ahead: self.qty_ahead,
            own_qty_remaining: self.own_qty_remaining,
            total_queue_qty: self.total_queue_qty,
            fill_probability_bps,
            expected_time_to_fill_ns: self.expected_time_to_fill_ns(),
            queue_loss_after_amend: self.queue_loss_after_amend,
            maker_taker_score_bps: fill_probability_bps,
            top_level_survival_bps: self.top_level_survival_bps(),
            last_update_ts_ns: self.last_update_ts_ns,
        }
    }

    fn apply_trade(&mut self, qty: i64) {
        let ahead_take = self.qty_ahead.min(qty);
        self.qty_ahead = self.qty_ahead.saturating_sub(ahead_take);
        let remaining = qty.saturating_sub(ahead_take);
        self.own_qty_remaining = self.own_qty_remaining.saturating_sub(remaining);
    }

    fn apply_cancel(&mut self, qty: i64) {
        let ahead_cancel =
            ((i128::from(qty) * i128::from(self.config.cancel_ahead_bps())) / 10_000) as i64;
        self.qty_ahead = self.qty_ahead.saturating_sub(ahead_cancel);
    }

    fn apply_amend(&mut self, new_total_queue_qty: i64) {
        self.queue_loss_after_amend = new_total_queue_qty;
        self.qty_ahead = new_total_queue_qty;
    }

    fn fill_probability_bps(&self) -> u16 {
        let needed = self.qty_ahead.saturating_add(self.own_qty_remaining);
        if needed <= 0 {
            return 10_000;
        }
        let expected = (i128::from(self.config.expected_depletion_per_sec())
            * i128::from(self.config.horizon_ns()))
            / 1_000_000_000;
        if expected <= 0 {
            return 0;
        }
        u16::try_from(((expected * 10_000) / i128::from(needed)).clamp(0, 10_000)).unwrap_or(10_000)
    }

    fn expected_time_to_fill_ns(&self) -> u64 {
        let needed = self.qty_ahead.saturating_add(self.own_qty_remaining);
        if needed <= 0 {
            return 0;
        }
        let rate = self.config.expected_depletion_per_sec();
        if rate <= 0 {
            return u64::MAX;
        }
        u64::try_from((i128::from(needed) * 1_000_000_000) / i128::from(rate)).unwrap_or(u64::MAX)
    }

    fn top_level_survival_bps(&self) -> u16 {
        if self.total_queue_qty <= 0 {
            return 0;
        }
        let pressure =
            ((i128::from(self.qty_ahead) * 10_000) / i128::from(self.total_queue_qty)).min(10_000);
        u16::try_from(10_000_i128.saturating_sub(pressure)).unwrap_or(0)
    }
}

/// Pattern-risk liquidity summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRiskLiquidity {
    executed_qty: i64,
    displayed_depth: i64,
}

impl PatternRiskLiquidity {
    /// Creates a liquidity summary for pattern-risk classification.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidPattern`] when quantities are negative.
    pub const fn new(executed_qty: i64, displayed_depth: i64) -> Result<Self, AnalyticsError> {
        if executed_qty < 0 || displayed_depth < 0 {
            return Err(AnalyticsError::InvalidPattern);
        }
        Ok(Self {
            executed_qty,
            displayed_depth,
        })
    }

    /// Returns executed quantity.
    pub const fn executed_qty(&self) -> i64 {
        self.executed_qty
    }

    /// Returns displayed depth.
    pub const fn displayed_depth(&self) -> i64 {
        self.displayed_depth
    }
}

/// Pattern-risk input over a bounded observation window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRiskInput {
    quote_adds: u64,
    quote_cancels: u64,
    trades: u64,
    depth_imbalance_bps: i32,
    price_move_bps: i32,
    liquidity: PatternRiskLiquidity,
    window_ns: u64,
}

impl PatternRiskInput {
    /// Creates pattern-risk input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidPattern`] when quantities or window are
    /// invalid.
    pub const fn new(
        quote_adds: u64,
        quote_cancels: u64,
        trades: u64,
        depth_imbalance_bps: i32,
        price_move_bps: i32,
        liquidity: PatternRiskLiquidity,
        window_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if window_ns == 0 {
            return Err(AnalyticsError::InvalidPattern);
        }
        Ok(Self {
            quote_adds,
            quote_cancels,
            trades,
            depth_imbalance_bps,
            price_move_bps,
            liquidity,
            window_ns,
        })
    }

    /// Returns quote add count.
    pub const fn quote_adds(&self) -> u64 {
        self.quote_adds
    }

    /// Returns quote cancel count.
    pub const fn quote_cancels(&self) -> u64 {
        self.quote_cancels
    }

    /// Returns trade count.
    pub const fn trades(&self) -> u64 {
        self.trades
    }

    /// Returns depth imbalance in basis points.
    pub const fn depth_imbalance_bps(&self) -> i32 {
        self.depth_imbalance_bps
    }

    /// Returns price movement in basis points.
    pub const fn price_move_bps(&self) -> i32 {
        self.price_move_bps
    }

    /// Returns executed quantity.
    pub const fn executed_qty(&self) -> i64 {
        self.liquidity.executed_qty()
    }

    /// Returns displayed depth.
    pub const fn displayed_depth(&self) -> i64 {
        self.liquidity.displayed_depth()
    }

    /// Returns liquidity summary.
    pub const fn liquidity(&self) -> PatternRiskLiquidity {
        self.liquidity
    }

    /// Returns observation window in nanoseconds.
    pub const fn window_ns(&self) -> u64 {
        self.window_ns
    }
}

/// Pattern-risk classifier thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRiskConfig {
    high_order_to_trade_bps: u32,
    high_cancel_ratio_bps: u16,
    high_imbalance_bps: u16,
    high_price_move_bps: u32,
    high_quote_events: u64,
}

impl PatternRiskConfig {
    /// Creates pattern-risk thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidPattern`] when basis-point thresholds
    /// exceed 10,000 where applicable.
    pub const fn new(
        high_order_to_trade_bps: u32,
        high_cancel_ratio_bps: u16,
        high_imbalance_bps: u16,
        high_price_move_bps: u32,
        high_quote_events: u64,
    ) -> Result<Self, AnalyticsError> {
        if high_cancel_ratio_bps > 10_000 || high_imbalance_bps > 10_000 {
            return Err(AnalyticsError::InvalidPattern);
        }
        Ok(Self {
            high_order_to_trade_bps,
            high_cancel_ratio_bps,
            high_imbalance_bps,
            high_price_move_bps,
            high_quote_events,
        })
    }
}

impl Default for PatternRiskConfig {
    fn default() -> Self {
        Self {
            high_order_to_trade_bps: 50_000,
            high_cancel_ratio_bps: 8_000,
            high_imbalance_bps: 7_000,
            high_price_move_bps: 25,
            high_quote_events: 1_000,
        }
    }
}

/// Pattern-risk snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRiskSnapshot {
    spoofing_layering_risk_bps: u16,
    quote_stuffing_risk_bps: u16,
    stop_run_risk_bps: u16,
    absorption_risk_bps: u16,
    momentum_ignition_risk_bps: u16,
    overall_risk_bps: u16,
}

impl PatternRiskSnapshot {
    /// Returns spoofing/layering risk indicator in basis points.
    pub const fn spoofing_layering_risk_bps(&self) -> u16 {
        self.spoofing_layering_risk_bps
    }

    /// Returns quote-stuffing risk indicator in basis points.
    pub const fn quote_stuffing_risk_bps(&self) -> u16 {
        self.quote_stuffing_risk_bps
    }

    /// Returns stop-run/liquidity-sweep risk indicator in basis points.
    pub const fn stop_run_risk_bps(&self) -> u16 {
        self.stop_run_risk_bps
    }

    /// Returns absorption risk indicator in basis points.
    pub const fn absorption_risk_bps(&self) -> u16 {
        self.absorption_risk_bps
    }

    /// Returns momentum-ignition risk indicator in basis points.
    pub const fn momentum_ignition_risk_bps(&self) -> u16 {
        self.momentum_ignition_risk_bps
    }

    /// Returns maximum component risk in basis points.
    pub const fn overall_risk_bps(&self) -> u16 {
        self.overall_risk_bps
    }
}

/// Deterministic pattern-risk classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternRiskClassifier {
    config: PatternRiskConfig,
}

impl PatternRiskClassifier {
    /// Creates a pattern-risk classifier.
    pub const fn new(config: PatternRiskConfig) -> Self {
        Self { config }
    }

    /// Classifies pattern risk.
    pub fn classify(&self, input: PatternRiskInput) -> PatternRiskSnapshot {
        let quote_events = input.quote_adds().saturating_add(input.quote_cancels());
        let order_to_trade_bps = rate_scaled(quote_events, input.trades().max(1));
        let cancel_ratio_bps = rate_bps(input.quote_cancels(), quote_events.max(1));
        let imbalance = input.depth_imbalance_bps().unsigned_abs();
        let move_abs = input.price_move_bps().unsigned_abs();
        let spoofing_layering = average_score(&[
            score_ratio(order_to_trade_bps, self.config.high_order_to_trade_bps),
            score_ratio(
                u32::from(cancel_ratio_bps),
                u32::from(self.config.high_cancel_ratio_bps),
            ),
            score_ratio(imbalance, u32::from(self.config.high_imbalance_bps)),
        ]);
        let quote_stuffing = score_ratio_u64(quote_events, self.config.high_quote_events);
        let stop_run = average_score(&[
            score_ratio(move_abs, self.config.high_price_move_bps),
            score_ratio_i64(input.executed_qty(), input.displayed_depth().max(1)),
        ]);
        let absorption = average_score(&[
            score_ratio_i64(input.executed_qty(), input.displayed_depth().max(1)),
            10_000_u16.saturating_sub(score_ratio(move_abs, self.config.high_price_move_bps)),
        ]);
        let momentum_ignition = average_score(&[
            score_ratio(move_abs, self.config.high_price_move_bps),
            score_ratio(order_to_trade_bps, self.config.high_order_to_trade_bps),
        ]);
        let overall = spoofing_layering
            .max(quote_stuffing)
            .max(stop_run)
            .max(absorption)
            .max(momentum_ignition);
        PatternRiskSnapshot {
            spoofing_layering_risk_bps: spoofing_layering,
            quote_stuffing_risk_bps: quote_stuffing,
            stop_run_risk_bps: stop_run,
            absorption_risk_bps: absorption,
            momentum_ignition_risk_bps: momentum_ignition,
            overall_risk_bps: overall,
        }
    }
}

impl Default for PatternRiskClassifier {
    fn default() -> Self {
        Self::new(PatternRiskConfig::default())
    }
}

/// Detailed pattern-risk configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternDetailConfig {
    iceberg_fill_multiple_bps: u32,
    stacked_imbalance_threshold_bps: u16,
    absorption_move_threshold_bps: u32,
    failed_breakout_threshold_bps: u32,
}

impl PatternDetailConfig {
    /// Creates detailed pattern-risk configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidPattern`] when basis-point thresholds
    /// exceed 10,000 where applicable.
    pub const fn new(
        iceberg_fill_multiple_bps: u32,
        stacked_imbalance_threshold_bps: u16,
        absorption_move_threshold_bps: u32,
        failed_breakout_threshold_bps: u32,
    ) -> Result<Self, AnalyticsError> {
        if stacked_imbalance_threshold_bps > 10_000 {
            return Err(AnalyticsError::InvalidPattern);
        }
        Ok(Self {
            iceberg_fill_multiple_bps,
            stacked_imbalance_threshold_bps,
            absorption_move_threshold_bps,
            failed_breakout_threshold_bps,
        })
    }

    /// Returns iceberg fill/displayed-depth threshold in basis points.
    pub const fn iceberg_fill_multiple_bps(&self) -> u32 {
        self.iceberg_fill_multiple_bps
    }

    /// Returns stacked-imbalance threshold in basis points.
    pub const fn stacked_imbalance_threshold_bps(&self) -> u16 {
        self.stacked_imbalance_threshold_bps
    }

    /// Returns absorption price-move threshold in basis points.
    pub const fn absorption_move_threshold_bps(&self) -> u32 {
        self.absorption_move_threshold_bps
    }

    /// Returns failed-breakout reversal threshold in basis points.
    pub const fn failed_breakout_threshold_bps(&self) -> u32 {
        self.failed_breakout_threshold_bps
    }
}

impl Default for PatternDetailConfig {
    fn default() -> Self {
        Self {
            iceberg_fill_multiple_bps: 20_000,
            stacked_imbalance_threshold_bps: 7_000,
            absorption_move_threshold_bps: 10,
            failed_breakout_threshold_bps: 20,
        }
    }
}

/// Detailed pattern-risk input over a bounded observation window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternDetailInput {
    repeated_fills_at_level: u64,
    replenishments_at_level: u64,
    executed_at_level_qty: i64,
    displayed_at_level_qty: i64,
    stacked_imbalance_levels: u8,
    stacked_imbalance_bps: u16,
    absorbed_qty: i64,
    absorption_price_move_bps: u32,
    breakout_move_bps: u32,
    reversal_move_bps: u32,
    signed_accumulation_qty: i64,
}

impl PatternDetailInput {
    /// Creates detailed pattern-risk input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidPattern`] when quantities are negative
    /// where unsigned semantics are required or basis-point values exceed
    /// 10,000 where applicable.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        repeated_fills_at_level: u64,
        replenishments_at_level: u64,
        executed_at_level_qty: i64,
        displayed_at_level_qty: i64,
        stacked_imbalance_levels: u8,
        stacked_imbalance_bps: u16,
        absorbed_qty: i64,
        absorption_price_move_bps: u32,
        breakout_move_bps: u32,
        reversal_move_bps: u32,
        signed_accumulation_qty: i64,
    ) -> Result<Self, AnalyticsError> {
        if executed_at_level_qty < 0
            || displayed_at_level_qty < 0
            || absorbed_qty < 0
            || stacked_imbalance_bps > 10_000
        {
            return Err(AnalyticsError::InvalidPattern);
        }
        Ok(Self {
            repeated_fills_at_level,
            replenishments_at_level,
            executed_at_level_qty,
            displayed_at_level_qty,
            stacked_imbalance_levels,
            stacked_imbalance_bps,
            absorbed_qty,
            absorption_price_move_bps,
            breakout_move_bps,
            reversal_move_bps,
            signed_accumulation_qty,
        })
    }

    /// Returns repeated fills at the same price level.
    pub const fn repeated_fills_at_level(&self) -> u64 {
        self.repeated_fills_at_level
    }

    /// Returns replenishments at the same price level.
    pub const fn replenishments_at_level(&self) -> u64 {
        self.replenishments_at_level
    }

    /// Returns executed quantity at the price level.
    pub const fn executed_at_level_qty(&self) -> i64 {
        self.executed_at_level_qty
    }

    /// Returns displayed quantity at the price level.
    pub const fn displayed_at_level_qty(&self) -> i64 {
        self.displayed_at_level_qty
    }

    /// Returns stacked imbalance level count.
    pub const fn stacked_imbalance_levels(&self) -> u8 {
        self.stacked_imbalance_levels
    }

    /// Returns stacked imbalance strength in basis points.
    pub const fn stacked_imbalance_bps(&self) -> u16 {
        self.stacked_imbalance_bps
    }

    /// Returns absorbed quantity.
    pub const fn absorbed_qty(&self) -> i64 {
        self.absorbed_qty
    }

    /// Returns price move during absorption in basis points.
    pub const fn absorption_price_move_bps(&self) -> u32 {
        self.absorption_price_move_bps
    }

    /// Returns breakout move in basis points.
    pub const fn breakout_move_bps(&self) -> u32 {
        self.breakout_move_bps
    }

    /// Returns reversal move after breakout in basis points.
    pub const fn reversal_move_bps(&self) -> u32 {
        self.reversal_move_bps
    }

    /// Returns signed accumulation quantity.
    pub const fn signed_accumulation_qty(&self) -> i64 {
        self.signed_accumulation_qty
    }
}

/// Detailed pattern-risk snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternDetailSnapshot {
    iceberg_risk_bps: u16,
    hidden_accumulation_bps: u16,
    hidden_distribution_bps: u16,
    stacked_imbalance_risk_bps: u16,
    absorption_strength_bps: u16,
    failed_breakout_risk_bps: u16,
    overall_risk_bps: u16,
}

impl PatternDetailSnapshot {
    /// Returns iceberg/hidden-refresh risk in basis points.
    pub const fn iceberg_risk_bps(&self) -> u16 {
        self.iceberg_risk_bps
    }

    /// Returns hidden accumulation risk in basis points.
    pub const fn hidden_accumulation_bps(&self) -> u16 {
        self.hidden_accumulation_bps
    }

    /// Returns hidden distribution risk in basis points.
    pub const fn hidden_distribution_bps(&self) -> u16 {
        self.hidden_distribution_bps
    }

    /// Returns stacked imbalance risk in basis points.
    pub const fn stacked_imbalance_risk_bps(&self) -> u16 {
        self.stacked_imbalance_risk_bps
    }

    /// Returns absorption strength in basis points.
    pub const fn absorption_strength_bps(&self) -> u16 {
        self.absorption_strength_bps
    }

    /// Returns failed breakout risk in basis points.
    pub const fn failed_breakout_risk_bps(&self) -> u16 {
        self.failed_breakout_risk_bps
    }

    /// Returns maximum detailed pattern risk in basis points.
    pub const fn overall_risk_bps(&self) -> u16 {
        self.overall_risk_bps
    }
}

/// Deterministic detailed pattern-risk analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternDetailAnalyzer {
    config: PatternDetailConfig,
}

impl PatternDetailAnalyzer {
    /// Creates a detailed pattern-risk analyzer.
    pub const fn new(config: PatternDetailConfig) -> Self {
        Self { config }
    }

    /// Returns analyzer configuration.
    pub const fn config(&self) -> PatternDetailConfig {
        self.config
    }

    /// Evaluates detailed pattern risk.
    pub fn evaluate(&self, input: PatternDetailInput) -> PatternDetailSnapshot {
        let fill_multiple_bps = ratio_i64_to_u32(
            input.executed_at_level_qty(),
            input.displayed_at_level_qty().max(1),
        );
        let iceberg_risk = average_score(&[
            score_ratio(fill_multiple_bps, self.config.iceberg_fill_multiple_bps()),
            score_ratio_u64(input.repeated_fills_at_level(), 5),
            score_ratio_u64(input.replenishments_at_level(), 3),
        ]);
        let hidden_accumulation = if input.signed_accumulation_qty() > 0 {
            iceberg_risk
        } else {
            0
        };
        let hidden_distribution = if input.signed_accumulation_qty() < 0 {
            iceberg_risk
        } else {
            0
        };
        let stacked_imbalance = average_score(&[
            score_ratio_u64(u64::from(input.stacked_imbalance_levels()), 3),
            score_ratio(
                u32::from(input.stacked_imbalance_bps()),
                u32::from(self.config.stacked_imbalance_threshold_bps()),
            ),
        ]);
        let absorption = average_score(&[
            score_ratio_i64(input.absorbed_qty(), input.displayed_at_level_qty().max(1)),
            10_000_u16.saturating_sub(score_ratio(
                input.absorption_price_move_bps(),
                self.config.absorption_move_threshold_bps(),
            )),
        ]);
        let failed_breakout = average_score(&[
            score_ratio(
                input.breakout_move_bps(),
                self.config.failed_breakout_threshold_bps(),
            ),
            score_ratio(
                input.reversal_move_bps(),
                self.config.failed_breakout_threshold_bps(),
            ),
        ]);
        let overall = iceberg_risk
            .max(hidden_accumulation)
            .max(hidden_distribution)
            .max(stacked_imbalance)
            .max(absorption)
            .max(failed_breakout);
        PatternDetailSnapshot {
            iceberg_risk_bps: iceberg_risk,
            hidden_accumulation_bps: hidden_accumulation,
            hidden_distribution_bps: hidden_distribution,
            stacked_imbalance_risk_bps: stacked_imbalance,
            absorption_strength_bps: absorption,
            failed_breakout_risk_bps: failed_breakout,
            overall_risk_bps: overall,
        }
    }
}

impl Default for PatternDetailAnalyzer {
    fn default() -> Self {
        Self::new(PatternDetailConfig::default())
    }
}

/// Venue route event kind.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VenueRouteEventKind {
    /// Child order was sent.
    Sent = 1,
    /// Child order received a fill.
    Fill = 2,
    /// Child order was rejected.
    Reject = 3,
    /// Child order was canceled.
    Cancel = 4,
}

/// Venue route analytics event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueRouteEvent {
    kind: VenueRouteEventKind,
    qty: i64,
    quote_to_fill_latency_ns: u64,
    market_data_to_order_latency_ns: u64,
}

impl VenueRouteEvent {
    /// Creates a venue route event.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidRoute`] when quantity is negative.
    pub const fn new(
        kind: VenueRouteEventKind,
        qty: i64,
        quote_to_fill_latency_ns: u64,
        market_data_to_order_latency_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if qty < 0 {
            return Err(AnalyticsError::InvalidRoute);
        }
        Ok(Self {
            kind,
            qty,
            quote_to_fill_latency_ns,
            market_data_to_order_latency_ns,
        })
    }

    /// Returns event kind.
    pub const fn kind(&self) -> VenueRouteEventKind {
        self.kind
    }

    /// Returns event quantity.
    pub const fn qty(&self) -> i64 {
        self.qty
    }

    /// Returns quote-to-fill latency in nanoseconds.
    pub const fn quote_to_fill_latency_ns(&self) -> u64 {
        self.quote_to_fill_latency_ns
    }

    /// Returns market-data-to-order latency in nanoseconds.
    pub const fn market_data_to_order_latency_ns(&self) -> u64 {
        self.market_data_to_order_latency_ns
    }
}

/// Venue route analytics snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueRouteSnapshot {
    sent: u64,
    fills: u64,
    rejects: u64,
    cancels: u64,
    sent_qty: i64,
    filled_qty: i64,
    fill_rate_bps: u16,
    reject_rate_bps: u16,
    cancel_rate_bps: u16,
    avg_quote_to_fill_latency_ns: u64,
    max_quote_to_fill_latency_ns: u64,
    avg_market_data_to_order_latency_ns: u64,
    max_market_data_to_order_latency_ns: u64,
    route_health_bps: u16,
}

impl VenueRouteSnapshot {
    /// Returns sent order count.
    pub const fn sent(&self) -> u64 {
        self.sent
    }

    /// Returns fill count.
    pub const fn fills(&self) -> u64 {
        self.fills
    }

    /// Returns reject count.
    pub const fn rejects(&self) -> u64 {
        self.rejects
    }

    /// Returns cancel count.
    pub const fn cancels(&self) -> u64 {
        self.cancels
    }

    /// Returns sent quantity.
    pub const fn sent_qty(&self) -> i64 {
        self.sent_qty
    }

    /// Returns filled quantity.
    pub const fn filled_qty(&self) -> i64 {
        self.filled_qty
    }

    /// Returns fill rate in basis points.
    pub const fn fill_rate_bps(&self) -> u16 {
        self.fill_rate_bps
    }

    /// Returns reject rate in basis points.
    pub const fn reject_rate_bps(&self) -> u16 {
        self.reject_rate_bps
    }

    /// Returns cancel rate in basis points.
    pub const fn cancel_rate_bps(&self) -> u16 {
        self.cancel_rate_bps
    }

    /// Returns average quote-to-fill latency.
    pub const fn avg_quote_to_fill_latency_ns(&self) -> u64 {
        self.avg_quote_to_fill_latency_ns
    }

    /// Returns maximum quote-to-fill latency.
    pub const fn max_quote_to_fill_latency_ns(&self) -> u64 {
        self.max_quote_to_fill_latency_ns
    }

    /// Returns average market-data-to-order latency.
    pub const fn avg_market_data_to_order_latency_ns(&self) -> u64 {
        self.avg_market_data_to_order_latency_ns
    }

    /// Returns maximum market-data-to-order latency.
    pub const fn max_market_data_to_order_latency_ns(&self) -> u64 {
        self.max_market_data_to_order_latency_ns
    }

    /// Returns route health score in basis points.
    pub const fn route_health_bps(&self) -> u16 {
        self.route_health_bps
    }
}

/// Venue route analytics tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueRouteTracker {
    sent: u64,
    fills: u64,
    rejects: u64,
    cancels: u64,
    sent_qty: i64,
    filled_qty: i64,
    quote_to_fill_latency_sum: u128,
    quote_to_fill_latency_samples: u64,
    max_quote_to_fill_latency_ns: u64,
    md_to_order_latency_sum: u128,
    md_to_order_latency_samples: u64,
    max_md_to_order_latency_ns: u64,
}

impl VenueRouteTracker {
    /// Creates an empty venue route tracker.
    pub const fn new() -> Self {
        Self {
            sent: 0,
            fills: 0,
            rejects: 0,
            cancels: 0,
            sent_qty: 0,
            filled_qty: 0,
            quote_to_fill_latency_sum: 0,
            quote_to_fill_latency_samples: 0,
            max_quote_to_fill_latency_ns: 0,
            md_to_order_latency_sum: 0,
            md_to_order_latency_samples: 0,
            max_md_to_order_latency_ns: 0,
        }
    }

    /// Records a venue route event.
    pub fn on_event(&mut self, event: VenueRouteEvent) {
        match event.kind() {
            VenueRouteEventKind::Sent => {
                self.sent = self.sent.saturating_add(1);
                self.sent_qty = self.sent_qty.saturating_add(event.qty());
            }
            VenueRouteEventKind::Fill => {
                self.fills = self.fills.saturating_add(1);
                self.filled_qty = self.filled_qty.saturating_add(event.qty());
                self.record_quote_to_fill(event.quote_to_fill_latency_ns());
            }
            VenueRouteEventKind::Reject => self.rejects = self.rejects.saturating_add(1),
            VenueRouteEventKind::Cancel => self.cancels = self.cancels.saturating_add(1),
        }
        self.record_md_to_order(event.market_data_to_order_latency_ns());
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> VenueRouteSnapshot {
        let terminal = self
            .fills
            .saturating_add(self.rejects)
            .saturating_add(self.cancels);
        let reject_rate = rate_bps(self.rejects, terminal.max(1));
        let cancel_rate = rate_bps(self.cancels, terminal.max(1));
        let fill_rate = rate_bps(self.fills, terminal.max(1));
        let route_health = fill_rate
            .saturating_sub(reject_rate / 2)
            .saturating_sub(cancel_rate / 4);
        VenueRouteSnapshot {
            sent: self.sent,
            fills: self.fills,
            rejects: self.rejects,
            cancels: self.cancels,
            sent_qty: self.sent_qty,
            filled_qty: self.filled_qty,
            fill_rate_bps: fill_rate,
            reject_rate_bps: reject_rate,
            cancel_rate_bps: cancel_rate,
            avg_quote_to_fill_latency_ns: avg_u128(
                self.quote_to_fill_latency_sum,
                self.quote_to_fill_latency_samples,
            ),
            max_quote_to_fill_latency_ns: self.max_quote_to_fill_latency_ns,
            avg_market_data_to_order_latency_ns: avg_u128(
                self.md_to_order_latency_sum,
                self.md_to_order_latency_samples,
            ),
            max_market_data_to_order_latency_ns: self.max_md_to_order_latency_ns,
            route_health_bps: route_health,
        }
    }

    /// Clears accumulated route state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    fn record_quote_to_fill(&mut self, latency_ns: u64) {
        if latency_ns == 0 {
            return;
        }
        self.quote_to_fill_latency_sum = self
            .quote_to_fill_latency_sum
            .saturating_add(u128::from(latency_ns));
        self.quote_to_fill_latency_samples = self.quote_to_fill_latency_samples.saturating_add(1);
        self.max_quote_to_fill_latency_ns = self.max_quote_to_fill_latency_ns.max(latency_ns);
    }

    fn record_md_to_order(&mut self, latency_ns: u64) {
        if latency_ns == 0 {
            return;
        }
        self.md_to_order_latency_sum = self
            .md_to_order_latency_sum
            .saturating_add(u128::from(latency_ns));
        self.md_to_order_latency_samples = self.md_to_order_latency_samples.saturating_add(1);
        self.max_md_to_order_latency_ns = self.max_md_to_order_latency_ns.max(latency_ns);
    }
}

impl Default for VenueRouteTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Cross-asset paired price sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetSample {
    leader_price: i64,
    follower_price: i64,
    ts_ns: u64,
}

impl CrossAssetSample {
    /// Creates a cross-asset paired price sample.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidCrossAsset`] when prices or timestamp
    /// are invalid.
    pub const fn new(
        leader_price: i64,
        follower_price: i64,
        ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if leader_price <= 0 || follower_price <= 0 || ts_ns == 0 {
            return Err(AnalyticsError::InvalidCrossAsset);
        }
        Ok(Self {
            leader_price,
            follower_price,
            ts_ns,
        })
    }

    /// Returns leader price.
    pub const fn leader_price(&self) -> i64 {
        self.leader_price
    }

    /// Returns follower price.
    pub const fn follower_price(&self) -> i64 {
        self.follower_price
    }

    /// Returns sample timestamp.
    pub const fn ts_ns(&self) -> u64 {
        self.ts_ns
    }

    /// Returns leader-minus-follower spread.
    pub const fn spread(&self) -> i64 {
        self.leader_price - self.follower_price
    }
}

/// Cross-asset analytics configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetConfig {
    divergence_threshold_bps: u32,
    basis_threshold_bps: u32,
    correlation_breakdown_bps: u16,
}

impl CrossAssetConfig {
    /// Creates cross-asset thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidCrossAsset`] when the correlation
    /// threshold exceeds 10,000.
    pub const fn new(
        divergence_threshold_bps: u32,
        basis_threshold_bps: u32,
        correlation_breakdown_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if correlation_breakdown_bps > 10_000 {
            return Err(AnalyticsError::InvalidCrossAsset);
        }
        Ok(Self {
            divergence_threshold_bps,
            basis_threshold_bps,
            correlation_breakdown_bps,
        })
    }

    /// Returns pair-divergence threshold.
    pub const fn divergence_threshold_bps(&self) -> u32 {
        self.divergence_threshold_bps
    }

    /// Returns futures/spot basis pressure threshold.
    pub const fn basis_threshold_bps(&self) -> u32 {
        self.basis_threshold_bps
    }

    /// Returns absolute-correlation threshold below which breakdown is flagged.
    pub const fn correlation_breakdown_bps(&self) -> u16 {
        self.correlation_breakdown_bps
    }
}

impl Default for CrossAssetConfig {
    fn default() -> Self {
        Self {
            divergence_threshold_bps: 50,
            basis_threshold_bps: 25,
            correlation_breakdown_bps: 2_000,
        }
    }
}

/// Cross-asset analytics snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetSnapshot {
    samples: usize,
    last_ts_ns: u64,
    correlation_bps: i32,
    beta_bps: i32,
    pair_divergence_bps: i32,
    basis_pressure_bps: i32,
    pair_divergence: bool,
    basis_pressure: bool,
    correlation_breakdown: bool,
    lead_lag_score_bps: u16,
}

impl CrossAssetSnapshot {
    /// Returns retained paired return count.
    pub const fn samples(&self) -> usize {
        self.samples
    }

    /// Returns latest sample timestamp.
    pub const fn last_ts_ns(&self) -> u64 {
        self.last_ts_ns
    }

    /// Returns rolling Pearson-style correlation scaled by 10,000.
    pub const fn correlation_bps(&self) -> i32 {
        self.correlation_bps
    }

    /// Returns follower-versus-leader beta scaled by 10,000.
    pub const fn beta_bps(&self) -> i32 {
        self.beta_bps
    }

    /// Returns latest pair divergence in basis points.
    pub const fn pair_divergence_bps(&self) -> i32 {
        self.pair_divergence_bps
    }

    /// Returns latest basis pressure in basis points.
    pub const fn basis_pressure_bps(&self) -> i32 {
        self.basis_pressure_bps
    }

    /// Returns true when pair divergence exceeds configured threshold.
    pub const fn pair_divergence(&self) -> bool {
        self.pair_divergence
    }

    /// Returns true when basis pressure exceeds configured threshold.
    pub const fn basis_pressure(&self) -> bool {
        self.basis_pressure
    }

    /// Returns true when absolute correlation is below configured threshold.
    pub const fn correlation_breakdown(&self) -> bool {
        self.correlation_breakdown
    }

    /// Returns lead/lag strength score in basis points.
    pub const fn lead_lag_score_bps(&self) -> u16 {
        self.lead_lag_score_bps
    }
}

/// Fixed-window cross-asset lead/lag tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrossAssetTracker<const N: usize = 64> {
    config: CrossAssetConfig,
    leader_returns_bps: [i32; N],
    follower_returns_bps: [i32; N],
    next: usize,
    len: usize,
    last_sample: Option<CrossAssetSample>,
    last_pair_divergence_bps: i32,
    last_basis_pressure_bps: i32,
}

impl<const N: usize> CrossAssetTracker<N> {
    /// Creates a cross-asset tracker.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidCrossAsset`] when capacity is zero.
    pub const fn new(config: CrossAssetConfig) -> Result<Self, AnalyticsError> {
        if N == 0 {
            return Err(AnalyticsError::InvalidCrossAsset);
        }
        Ok(Self {
            config,
            leader_returns_bps: [0; N],
            follower_returns_bps: [0; N],
            next: 0,
            len: 0,
            last_sample: None,
            last_pair_divergence_bps: 0,
            last_basis_pressure_bps: 0,
        })
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> CrossAssetConfig {
        self.config
    }

    /// Records one paired sample.
    pub fn on_sample(&mut self, sample: CrossAssetSample) -> CrossAssetSnapshot {
        if let Some(previous) = self.last_sample {
            let leader_ret = price_to_bps(
                sample
                    .leader_price()
                    .saturating_sub(previous.leader_price()),
                previous.leader_price(),
            );
            let follower_ret = price_to_bps(
                sample
                    .follower_price()
                    .saturating_sub(previous.follower_price()),
                previous.follower_price(),
            );
            self.leader_returns_bps[self.next] = leader_ret;
            self.follower_returns_bps[self.next] = follower_ret;
            self.next = (self.next + 1) % N;
            self.len = self.len.saturating_add(1).min(N);
            self.last_pair_divergence_bps = follower_ret.saturating_sub(leader_ret);
        }
        self.last_basis_pressure_bps = price_to_bps(sample.spread(), sample.follower_price());
        self.last_sample = Some(sample);
        self.snapshot()
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> CrossAssetSnapshot {
        let correlation = self.correlation_bps();
        let beta = self.beta_bps();
        let abs_corr = correlation.unsigned_abs();
        CrossAssetSnapshot {
            samples: self.len,
            last_ts_ns: self.last_sample.map(|sample| sample.ts_ns()).unwrap_or(0),
            correlation_bps: correlation,
            beta_bps: beta,
            pair_divergence_bps: self.last_pair_divergence_bps,
            basis_pressure_bps: self.last_basis_pressure_bps,
            pair_divergence: self.last_pair_divergence_bps.unsigned_abs()
                >= self.config.divergence_threshold_bps(),
            basis_pressure: self.last_basis_pressure_bps.unsigned_abs()
                >= self.config.basis_threshold_bps(),
            correlation_breakdown: self.len > 1
                && abs_corr < u32::from(self.config.correlation_breakdown_bps()),
            lead_lag_score_bps: score_ratio(abs_corr, 10_000),
        }
    }

    /// Clears accumulated state.
    pub fn reset(&mut self) {
        self.next = 0;
        self.len = 0;
        self.last_sample = None;
        self.last_pair_divergence_bps = 0;
        self.last_basis_pressure_bps = 0;
        self.leader_returns_bps = [0; N];
        self.follower_returns_bps = [0; N];
    }

    fn correlation_bps(&self) -> i32 {
        if self.len <= 1 {
            return 0;
        }
        let mut sum_xy = 0_i128;
        let mut sum_x2 = 0_u128;
        let mut sum_y2 = 0_u128;
        for offset in 0..self.len {
            let idx = if self.len == N {
                (self.next + offset) % N
            } else {
                offset
            };
            let x = i128::from(self.leader_returns_bps[idx]);
            let y = i128::from(self.follower_returns_bps[idx]);
            sum_xy = sum_xy.saturating_add(x.saturating_mul(y));
            sum_x2 = sum_x2.saturating_add(x.unsigned_abs().saturating_mul(x.unsigned_abs()));
            sum_y2 = sum_y2.saturating_add(y.unsigned_abs().saturating_mul(y.unsigned_abs()));
        }
        if sum_x2 == 0 || sum_y2 == 0 {
            return 0;
        }
        let denominator = isqrt_u128(sum_x2.saturating_mul(sum_y2));
        if denominator == 0 {
            return 0;
        }
        let scaled = (sum_xy.saturating_mul(10_000)) / denominator as i128;
        i32::try_from(scaled.clamp(-10_000, 10_000)).unwrap_or(0)
    }

    fn beta_bps(&self) -> i32 {
        if self.len == 0 {
            return 0;
        }
        let mut sum_xy = 0_i128;
        let mut sum_x2 = 0_i128;
        for offset in 0..self.len {
            let idx = if self.len == N {
                (self.next + offset) % N
            } else {
                offset
            };
            let x = i128::from(self.leader_returns_bps[idx]);
            let y = i128::from(self.follower_returns_bps[idx]);
            sum_xy = sum_xy.saturating_add(x.saturating_mul(y));
            sum_x2 = sum_x2.saturating_add(x.saturating_mul(x));
        }
        if sum_x2 == 0 {
            return 0;
        }
        i32::try_from(
            ((sum_xy * 10_000) / sum_x2).clamp(i128::from(i32::MIN), i128::from(i32::MAX)),
        )
        .unwrap_or(0)
    }
}

/// Option contract kind.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OptionKind {
    /// Call option.
    Call = 1,
    /// Put option.
    Put = 2,
}

/// Option flow sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionFlowSample {
    kind: OptionKind,
    volume: i64,
    open_interest: i64,
    premium: i128,
    implied_vol_bps: u32,
    gamma_exposure: i128,
}

impl OptionFlowSample {
    /// Creates an option flow sample.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDerivative`] when quantity, premium,
    /// or open interest is invalid.
    pub const fn new(
        kind: OptionKind,
        volume: i64,
        open_interest: i64,
        premium: i128,
        implied_vol_bps: u32,
        gamma_exposure: i128,
    ) -> Result<Self, AnalyticsError> {
        if volume < 0 || open_interest < 0 || premium < 0 {
            return Err(AnalyticsError::InvalidDerivative);
        }
        Ok(Self {
            kind,
            volume,
            open_interest,
            premium,
            implied_vol_bps,
            gamma_exposure,
        })
    }

    /// Returns option kind.
    pub const fn kind(&self) -> OptionKind {
        self.kind
    }

    /// Returns contract volume.
    pub const fn volume(&self) -> i64 {
        self.volume
    }

    /// Returns open interest.
    pub const fn open_interest(&self) -> i64 {
        self.open_interest
    }

    /// Returns premium/notional contribution.
    pub const fn premium(&self) -> i128 {
        self.premium
    }

    /// Returns implied volatility in basis points.
    pub const fn implied_vol_bps(&self) -> u32 {
        self.implied_vol_bps
    }

    /// Returns caller-supplied gamma exposure contribution.
    pub const fn gamma_exposure(&self) -> i128 {
        self.gamma_exposure
    }
}

/// Option flow snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionFlowSnapshot {
    call_volume: i64,
    put_volume: i64,
    call_open_interest: i64,
    put_open_interest: i64,
    call_premium: i128,
    put_premium: i128,
    put_call_volume_ratio_bps: u32,
    put_call_open_interest_ratio_bps: u32,
    put_call_premium_ratio_bps: u32,
    volume_open_interest_anomaly_bps: u32,
    implied_vol_flow_bps: u32,
    net_gamma_exposure: i128,
    put_call_pressure_bps: i32,
}

impl OptionFlowSnapshot {
    /// Returns call volume.
    pub const fn call_volume(&self) -> i64 {
        self.call_volume
    }

    /// Returns put volume.
    pub const fn put_volume(&self) -> i64 {
        self.put_volume
    }

    /// Returns call open interest.
    pub const fn call_open_interest(&self) -> i64 {
        self.call_open_interest
    }

    /// Returns put open interest.
    pub const fn put_open_interest(&self) -> i64 {
        self.put_open_interest
    }

    /// Returns call premium.
    pub const fn call_premium(&self) -> i128 {
        self.call_premium
    }

    /// Returns put premium.
    pub const fn put_premium(&self) -> i128 {
        self.put_premium
    }

    /// Returns put/call volume ratio scaled by 10,000.
    pub const fn put_call_volume_ratio_bps(&self) -> u32 {
        self.put_call_volume_ratio_bps
    }

    /// Returns put/call open-interest ratio scaled by 10,000.
    pub const fn put_call_open_interest_ratio_bps(&self) -> u32 {
        self.put_call_open_interest_ratio_bps
    }

    /// Returns put/call premium ratio scaled by 10,000.
    pub const fn put_call_premium_ratio_bps(&self) -> u32 {
        self.put_call_premium_ratio_bps
    }

    /// Returns aggregate volume/open-interest anomaly ratio.
    pub const fn volume_open_interest_anomaly_bps(&self) -> u32 {
        self.volume_open_interest_anomaly_bps
    }

    /// Returns premium-weighted implied-volatility flow in basis points.
    pub const fn implied_vol_flow_bps(&self) -> u32 {
        self.implied_vol_flow_bps
    }

    /// Returns net gamma exposure.
    pub const fn net_gamma_exposure(&self) -> i128 {
        self.net_gamma_exposure
    }

    /// Returns put-minus-call directional pressure in basis points.
    pub const fn put_call_pressure_bps(&self) -> i32 {
        self.put_call_pressure_bps
    }
}

/// Cumulative option flow tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionFlowTracker {
    call_volume: i64,
    put_volume: i64,
    call_open_interest: i64,
    put_open_interest: i64,
    call_premium: i128,
    put_premium: i128,
    implied_vol_premium_sum: u128,
    premium_sum: u128,
    net_gamma_exposure: i128,
}

impl OptionFlowTracker {
    /// Creates an empty option flow tracker.
    pub const fn new() -> Self {
        Self {
            call_volume: 0,
            put_volume: 0,
            call_open_interest: 0,
            put_open_interest: 0,
            call_premium: 0,
            put_premium: 0,
            implied_vol_premium_sum: 0,
            premium_sum: 0,
            net_gamma_exposure: 0,
        }
    }

    /// Records one option flow sample.
    pub fn on_sample(&mut self, sample: OptionFlowSample) {
        match sample.kind() {
            OptionKind::Call => {
                self.call_volume = self.call_volume.saturating_add(sample.volume());
                self.call_open_interest = self
                    .call_open_interest
                    .saturating_add(sample.open_interest());
                self.call_premium = self.call_premium.saturating_add(sample.premium());
            }
            OptionKind::Put => {
                self.put_volume = self.put_volume.saturating_add(sample.volume());
                self.put_open_interest = self
                    .put_open_interest
                    .saturating_add(sample.open_interest());
                self.put_premium = self.put_premium.saturating_add(sample.premium());
            }
        }
        self.implied_vol_premium_sum = self.implied_vol_premium_sum.saturating_add(
            u128::from(sample.implied_vol_bps()).saturating_mul(sample.premium() as u128),
        );
        self.premium_sum = self.premium_sum.saturating_add(sample.premium() as u128);
        self.net_gamma_exposure = self
            .net_gamma_exposure
            .saturating_add(sample.gamma_exposure());
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> OptionFlowSnapshot {
        let total_volume = self.call_volume.saturating_add(self.put_volume);
        let total_oi = self
            .call_open_interest
            .saturating_add(self.put_open_interest);
        OptionFlowSnapshot {
            call_volume: self.call_volume,
            put_volume: self.put_volume,
            call_open_interest: self.call_open_interest,
            put_open_interest: self.put_open_interest,
            call_premium: self.call_premium,
            put_premium: self.put_premium,
            put_call_volume_ratio_bps: ratio_i64_to_u32(self.put_volume, self.call_volume),
            put_call_open_interest_ratio_bps: ratio_i64_to_u32(
                self.put_open_interest,
                self.call_open_interest,
            ),
            put_call_premium_ratio_bps: ratio_i128_to_u32(self.put_premium, self.call_premium),
            volume_open_interest_anomaly_bps: ratio_i64_to_u32(total_volume, total_oi),
            implied_vol_flow_bps: self
                .implied_vol_premium_sum
                .checked_div(self.premium_sum)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(0),
            net_gamma_exposure: self.net_gamma_exposure,
            put_call_pressure_bps: signed_pressure_bps(self.put_volume, self.call_volume),
        }
    }

    /// Clears accumulated option flow state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for OptionFlowTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Futures basis input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuturesBasisInput {
    spot_price: i64,
    futures_price: i64,
    fair_value_price: i64,
    near_contract_price: i64,
    far_contract_price: i64,
    funding_rate_bps: i32,
}

impl FuturesBasisInput {
    /// Creates futures basis input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDerivative`] when prices are
    /// non-positive.
    pub const fn new(
        spot_price: i64,
        futures_price: i64,
        fair_value_price: i64,
        near_contract_price: i64,
        far_contract_price: i64,
        funding_rate_bps: i32,
    ) -> Result<Self, AnalyticsError> {
        if spot_price <= 0
            || futures_price <= 0
            || fair_value_price <= 0
            || near_contract_price <= 0
            || far_contract_price <= 0
        {
            return Err(AnalyticsError::InvalidDerivative);
        }
        Ok(Self {
            spot_price,
            futures_price,
            fair_value_price,
            near_contract_price,
            far_contract_price,
            funding_rate_bps,
        })
    }

    /// Returns spot price.
    pub const fn spot_price(&self) -> i64 {
        self.spot_price
    }

    /// Returns futures price.
    pub const fn futures_price(&self) -> i64 {
        self.futures_price
    }

    /// Returns fair-value futures price.
    pub const fn fair_value_price(&self) -> i64 {
        self.fair_value_price
    }

    /// Returns near contract price.
    pub const fn near_contract_price(&self) -> i64 {
        self.near_contract_price
    }

    /// Returns far contract price.
    pub const fn far_contract_price(&self) -> i64 {
        self.far_contract_price
    }

    /// Returns funding rate in basis points.
    pub const fn funding_rate_bps(&self) -> i32 {
        self.funding_rate_bps
    }
}

/// Futures basis snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuturesBasisSnapshot {
    basis_bps: i32,
    fair_value_gap_bps: i32,
    calendar_spread_bps: i32,
    roll_pressure_bps: i32,
    funding_basis_divergence_bps: i32,
}

impl FuturesBasisSnapshot {
    /// Returns futures-minus-spot basis in basis points.
    pub const fn basis_bps(&self) -> i32 {
        self.basis_bps
    }

    /// Returns futures-minus-fair-value gap in basis points.
    pub const fn fair_value_gap_bps(&self) -> i32 {
        self.fair_value_gap_bps
    }

    /// Returns far-minus-near calendar spread in basis points.
    pub const fn calendar_spread_bps(&self) -> i32 {
        self.calendar_spread_bps
    }

    /// Returns calendar spread minus basis as roll-pressure proxy.
    pub const fn roll_pressure_bps(&self) -> i32 {
        self.roll_pressure_bps
    }

    /// Returns basis minus funding-rate divergence in basis points.
    pub const fn funding_basis_divergence_bps(&self) -> i32 {
        self.funding_basis_divergence_bps
    }
}

/// Futures basis analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuturesBasisAnalyzer;

impl FuturesBasisAnalyzer {
    /// Analyzes futures basis input.
    pub fn analyze(input: FuturesBasisInput) -> FuturesBasisSnapshot {
        let basis_bps = price_to_bps(
            input.futures_price().saturating_sub(input.spot_price()),
            input.spot_price(),
        );
        let fair_value_gap_bps = price_to_bps(
            input
                .futures_price()
                .saturating_sub(input.fair_value_price()),
            input.fair_value_price(),
        );
        let calendar_spread_bps = price_to_bps(
            input
                .far_contract_price()
                .saturating_sub(input.near_contract_price()),
            input.near_contract_price(),
        );
        FuturesBasisSnapshot {
            basis_bps,
            fair_value_gap_bps,
            calendar_spread_bps,
            roll_pressure_bps: calendar_spread_bps.saturating_sub(basis_bps),
            funding_basis_divergence_bps: basis_bps.saturating_sub(input.funding_rate_bps()),
        }
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
        let sweepable_buy_qty = sweep_qty(asks, target_qty);
        let sweepable_sell_qty = sweep_qty(bids, target_qty);
        let buy_sweepability_bps = sweepability_bps(sweepable_buy_qty, target_qty);
        let sell_sweepability_bps = sweepability_bps(sweepable_sell_qty, target_qty);
        Ok(LiquidityDepthSnapshot {
            levels_used: self.levels.min(bids.len()).min(asks.len()),
            top_bid_qty: bids[0].size,
            top_ask_qty: asks[0].size,
            bid_depth,
            ask_depth,
            proportional_imbalance_bps,
            depth_slope_bps: depth_slope_bps(bids, asks, self.levels),
            depth_convexity_bps: depth_convexity_bps(bids, asks, self.levels),
            book_pressure_bps: book_pressure_bps(bids, asks, self.levels),
            sweepable_buy_qty,
            sweepable_sell_qty,
            buy_sweepability_bps,
            sell_sweepability_bps,
            sweepability_score_bps: buy_sweepability_bps.min(sell_sweepability_bps),
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

fn depth_convexity_bps(bids: &[BookLevel], asks: &[BookLevel], max_levels: usize) -> i32 {
    let used = max_levels.min(bids.len()).min(asks.len());
    if used < 3 {
        return 0;
    }
    let first = bids[0].size.saturating_add(asks[0].size);
    let mid_index = used / 2;
    let mid = bids[mid_index].size.saturating_add(asks[mid_index].size);
    let outer = bids[used - 1].size.saturating_add(asks[used - 1].size);
    if mid <= 0 {
        return 0;
    }
    let first_slope = mid.saturating_sub(first);
    let second_slope = outer.saturating_sub(mid);
    i32::try_from((i128::from(second_slope.saturating_sub(first_slope)) * 10_000) / i128::from(mid))
        .unwrap_or(0)
}

fn book_pressure_bps(bids: &[BookLevel], asks: &[BookLevel], max_levels: usize) -> i32 {
    let used = max_levels.min(bids.len()).min(asks.len());
    if used == 0 {
        return 0;
    }
    let mut signed = 0_i128;
    let mut total = 0_i128;
    for index in 0..used {
        let weight = i128::try_from(used.saturating_sub(index)).unwrap_or(i128::MAX);
        let bid = i128::from(bids[index].size);
        let ask = i128::from(asks[index].size);
        signed = signed.saturating_add(weight.saturating_mul(bid.saturating_sub(ask)));
        total = total.saturating_add(weight.saturating_mul(bid.saturating_add(ask)));
    }
    if total <= 0 {
        return 0;
    }
    i32::try_from((signed * 10_000) / total).unwrap_or(0)
}

fn sweepability_bps(sweepable_qty: i64, target_qty: i64) -> u16 {
    if target_qty <= 0 {
        return 10_000;
    }
    u16::try_from(((i128::from(sweepable_qty) * 10_000) / i128::from(target_qty)).min(10_000))
        .unwrap_or(10_000)
}

fn qty_rate_per_sec(qty: i64, elapsed_ns: u64) -> i64 {
    if elapsed_ns == 0 {
        return 0;
    }
    i64::try_from((i128::from(qty) * 1_000_000_000_i128) / i128::from(elapsed_ns))
        .unwrap_or(i64::MAX)
}

fn ratio_bps_i64(value: i64, total: i64) -> u16 {
    if value <= 0 || total <= 0 {
        return 0;
    }
    u16::try_from(((i128::from(value) * 10_000) / i128::from(total)).min(10_000)).unwrap_or(10_000)
}

fn coefficient_impact_bps(participation_bps: u16, coefficient_bps: u16) -> i32 {
    i32::try_from((u128::from(participation_bps) * u128::from(coefficient_bps)) / 10_000)
        .unwrap_or(i32::MAX)
}

fn decay_remaining_bps(horizon_ns: u64, half_life_ns: u64) -> u16 {
    if half_life_ns == 0 {
        return 0;
    }
    let denominator = u128::from(half_life_ns).saturating_add(u128::from(horizon_ns));
    u16::try_from((u128::from(half_life_ns) * 10_000) / denominator).unwrap_or(0)
}

fn integer_sqrt_u128(value: u128) -> u128 {
    if value <= 1 {
        return value;
    }
    let mut low = 1_u128;
    let mut high = value;
    let mut answer = 1_u128;
    while low <= high {
        let mid = low + ((high - low) / 2);
        let square = mid.saturating_mul(mid);
        if square <= value {
            answer = mid;
            low = mid.saturating_add(1);
        } else {
            high = mid.saturating_sub(1);
        }
    }
    answer
}

fn bps_to_price_delta(price: i64, bps: i32) -> i64 {
    if price <= 0 || bps <= 0 {
        return 0;
    }
    i64::try_from((i128::from(price) * i128::from(bps)) / 10_000).unwrap_or(i64::MAX)
}

fn side_aware_price_move_bps(side: Side, reference_price: i64, observed_price: i64) -> i32 {
    let distance = match side {
        Side::Ask => observed_price.saturating_sub(reference_price),
        Side::Bid => reference_price.saturating_sub(observed_price),
    };
    price_to_bps(distance, reference_price)
}

fn quote_fade_bps(input: ToxicityInput) -> i32 {
    let (pre_qty, post_qty) = match input.trade().aggressor_side() {
        Side::Ask => (input.pre_ask_qty(), input.post_ask_qty()),
        Side::Bid => (input.pre_bid_qty(), input.post_bid_qty()),
    };
    if pre_qty <= 0 {
        return 0;
    }
    i32::try_from((i128::from(pre_qty.saturating_sub(post_qty)) * 10_000) / i128::from(pre_qty))
        .unwrap_or(0)
}

fn positive_bps(value: i32) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(0)
}

fn average_bps4(a: u16, b: u16, c: u16, d: u16) -> u16 {
    u16::try_from(
        (u32::from(a)
            .saturating_add(u32::from(b))
            .saturating_add(u32::from(c))
            .saturating_add(u32::from(d))
            / 4)
        .min(10_000),
    )
    .unwrap_or(10_000)
}

fn return_bps(to_price: i64, from_price: i64) -> i32 {
    if from_price <= 0 {
        return 0;
    }
    i32::try_from(
        ((i128::from(to_price) - i128::from(from_price)) * 10_000) / i128::from(from_price),
    )
    .unwrap_or(0)
}

fn abs_return_bps(to_price: i64, from_price: i64) -> u32 {
    return_bps(to_price, from_price).unsigned_abs()
}

fn volatility_and_noise(returns_bps: &[i32]) -> (u32, u16) {
    let mut sum_sq = 0_u128;
    let mut sign_flips = 0_u32;
    let mut prev_sign = 0_i32;
    for ret in returns_bps {
        let abs = ret.unsigned_abs();
        sum_sq = sum_sq.saturating_add(u128::from(abs).saturating_mul(u128::from(abs)));
        let sign = ret.signum();
        if prev_sign != 0 && sign != 0 && sign != prev_sign {
            sign_flips = sign_flips.saturating_add(1);
        }
        if sign != 0 {
            prev_sign = sign;
        }
    }
    let len = u128::try_from(returns_bps.len()).unwrap_or(1);
    let realized = u32::try_from(isqrt_u128(sum_sq / len)).unwrap_or(u32::MAX);
    let noise = if returns_bps.len() <= 1 {
        0
    } else {
        u16::try_from(
            (u128::from(sign_flips) * 10_000) / u128::try_from(returns_bps.len() - 1).unwrap_or(1),
        )
        .unwrap_or(10_000)
    };
    (realized, noise)
}

fn transition_confidence_bps(
    input: CompositeRegimeInput,
    config: CompositeRegimeConfig,
    trend: TrendRegimeKind,
    liquidity: LiquidityRegimeKind,
    spread: SpreadRegimeKind,
    session: SessionRegimeKind,
) -> u16 {
    let trend_margin = match trend {
        TrendRegimeKind::Trend => margin_bps_u32(
            u32::from(input.trend_strength_bps()),
            u32::from(config.trend_threshold_bps()),
        ),
        TrendRegimeKind::Chop => margin_bps_u32(
            u32::from(input.chop_score_bps()),
            u32::from(config.chop_threshold_bps()),
        ),
        TrendRegimeKind::Range => {
            let trend_gap = u32::from(
                config
                    .trend_threshold_bps()
                    .saturating_sub(input.trend_strength_bps()),
            );
            let chop_gap = u32::from(
                config
                    .chop_threshold_bps()
                    .saturating_sub(input.chop_score_bps()),
            );
            u16::try_from(trend_gap.min(chop_gap).min(10_000)).unwrap_or(10_000)
        }
    };
    let liquidity_margin = match liquidity {
        LiquidityRegimeKind::Hidden => margin_bps_u32(
            u32::from(input.hidden_liquidity_proxy_bps()),
            u32::from(config.hidden_liquidity_threshold_bps()),
        ),
        LiquidityRegimeKind::Thin => {
            i64_distance_to_bps(config.thin_depth(), input.displayed_depth())
        }
        LiquidityRegimeKind::Deep => {
            i64_distance_to_bps(input.displayed_depth(), config.deep_depth())
        }
        LiquidityRegimeKind::Normal => {
            let above_thin = input.displayed_depth().saturating_sub(config.thin_depth());
            let below_deep = config.deep_depth().saturating_sub(input.displayed_depth());
            i64_distance_to_bps(above_thin.min(below_deep), config.deep_depth().max(1))
        }
    };
    let spread_margin = match spread {
        SpreadRegimeKind::Wide => margin_bps_u32(input.spread_bps(), config.wide_spread_bps()),
        SpreadRegimeKind::Tight => margin_bps_u32(config.tight_spread_bps(), input.spread_bps()),
        SpreadRegimeKind::Normal => u16::try_from(
            input
                .spread_bps()
                .saturating_sub(config.tight_spread_bps())
                .min(config.wide_spread_bps().saturating_sub(input.spread_bps()))
                .min(10_000),
        )
        .unwrap_or(10_000),
    };
    let session_margin = match session {
        SessionRegimeKind::NewsShock => margin_bps_u32(
            u32::from(input.news_intensity_bps()),
            u32::from(config.news_shock_threshold_bps()),
        ),
        SessionRegimeKind::Auction => 10_000,
        SessionRegimeKind::Open => {
            time_margin_bps(config.open_window_ns(), input.elapsed_since_open_ns())
        }
        SessionRegimeKind::Close => {
            time_margin_bps(config.close_window_ns(), input.remaining_to_close_ns())
        }
        SessionRegimeKind::Continuous => {
            let from_open = input
                .elapsed_since_open_ns()
                .saturating_sub(config.open_window_ns());
            let from_close = input
                .remaining_to_close_ns()
                .saturating_sub(config.close_window_ns());
            u16::try_from((from_open.min(from_close) / 1_000_000_000).min(10_000)).unwrap_or(10_000)
        }
    };
    trend_margin
        .min(liquidity_margin)
        .min(spread_margin)
        .min(session_margin)
}

fn margin_bps_u32(value: u32, threshold: u32) -> u16 {
    if threshold == 0 {
        return 10_000;
    }
    u16::try_from(
        ((u128::from(value.saturating_sub(threshold)) * 10_000) / u128::from(threshold))
            .min(10_000),
    )
    .unwrap_or(10_000)
}

fn i64_distance_to_bps(distance: i64, reference: i64) -> u16 {
    if reference <= 0 {
        return 10_000;
    }
    u16::try_from(((i128::from(distance.max(0)) * 10_000) / i128::from(reference)).min(10_000))
        .unwrap_or(10_000)
}

fn time_margin_bps(window_ns: u64, value_ns: u64) -> u16 {
    if window_ns == 0 {
        return 10_000;
    }
    u16::try_from(
        ((u128::from(window_ns.saturating_sub(value_ns)) * 10_000) / u128::from(window_ns))
            .min(10_000),
    )
    .unwrap_or(10_000)
}

fn price_to_bps(value: i64, reference: i64) -> i32 {
    if reference <= 0 {
        return 0;
    }
    let bps = (i128::from(value) * 10_000) / i128::from(reference);
    i32::try_from(bps.clamp(i128::from(i32::MIN), i128::from(i32::MAX))).unwrap_or(0)
}

fn side_aware_slippage_bps(trade: TradeContext, benchmark_price: i64) -> i32 {
    let distance = match trade.aggressor_side() {
        Side::Ask => trade.price().saturating_sub(benchmark_price),
        Side::Bid => benchmark_price.saturating_sub(trade.price()),
    };
    price_to_bps(distance, benchmark_price)
}

fn rate_bps(count: u64, total: u64) -> u16 {
    if total == 0 {
        return 0;
    }
    u16::try_from(((u128::from(count) * 10_000) / u128::from(total)).min(10_000)).unwrap_or(10_000)
}

fn rate_scaled(count: u64, total: u64) -> u32 {
    if total == 0 {
        return 0;
    }
    u32::try_from(((u128::from(count) * 10_000) / u128::from(total)).min(u128::from(u32::MAX)))
        .unwrap_or(u32::MAX)
}

fn score_ratio(value: u32, threshold: u32) -> u16 {
    if threshold == 0 {
        return 0;
    }
    u16::try_from(((u128::from(value) * 10_000) / u128::from(threshold)).min(10_000))
        .unwrap_or(10_000)
}

fn score_ratio_u64(value: u64, threshold: u64) -> u16 {
    if threshold == 0 {
        return 0;
    }
    u16::try_from(((u128::from(value) * 10_000) / u128::from(threshold)).min(10_000))
        .unwrap_or(10_000)
}

fn score_ratio_i64(value: i64, threshold: i64) -> u16 {
    if threshold <= 0 {
        return 0;
    }
    u16::try_from(((value.max(0) as u128 * 10_000) / threshold as u128).min(10_000))
        .unwrap_or(10_000)
}

fn ratio_i64_to_u32(numerator: i64, denominator: i64) -> u32 {
    if denominator <= 0 {
        return 0;
    }
    u32::try_from(
        ((numerator.max(0) as u128 * 10_000) / denominator.max(1) as u128)
            .min(u128::from(u32::MAX)),
    )
    .unwrap_or(u32::MAX)
}

fn ratio_i128_to_u32(numerator: i128, denominator: i128) -> u32 {
    if denominator <= 0 {
        return 0;
    }
    u32::try_from(
        ((numerator.max(0) as u128 * 10_000) / denominator.max(1) as u128)
            .min(u128::from(u32::MAX)),
    )
    .unwrap_or(u32::MAX)
}

fn signed_pressure_bps(left: i64, right: i64) -> i32 {
    let total = left.saturating_add(right);
    if total <= 0 {
        return 0;
    }
    i32::try_from((i128::from(left.saturating_sub(right)) * 10_000) / i128::from(total))
        .unwrap_or(0)
}

fn average_score(scores: &[u16]) -> u16 {
    if scores.is_empty() {
        return 0;
    }
    let sum = scores
        .iter()
        .fold(0_u32, |acc, score| acc.saturating_add(u32::from(*score)));
    u16::try_from(sum / u32::try_from(scores.len()).unwrap_or(1)).unwrap_or(10_000)
}

fn avg_u128(sum: u128, count: u64) -> u64 {
    if count == 0 {
        return 0;
    }
    u64::try_from(sum / u128::from(count)).unwrap_or(u64::MAX)
}

fn fnv1a64(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn hash_feature_definition(mut hash: u64, definition: FeatureDefinition) -> u64 {
    hash = fnv1a64(hash, &definition.id().raw().to_le_bytes());
    hash = fnv1a64(hash, definition.name().as_bytes());
    hash = fnv1a64(hash, &[definition.unit().code()]);
    hash = fnv1a64(hash, &definition.scale().to_le_bytes());
    hash = fnv1a64(hash, &[definition.missing_policy().code()]);
    fnv1a64(
        hash,
        &definition.missing_policy().fill_value().to_le_bytes(),
    )
}

fn isqrt_u128(value: u128) -> u128 {
    if value <= 1 {
        return value;
    }
    let mut x0 = value / 2;
    let mut x1 = (x0 + value / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + value / x0) / 2;
    }
    x0
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
    fn execution_quality_scores_buy_fill_against_benchmarks() {
        let trade = TradeContext::new(101_000, 10, Side::Ask, 1).expect("trade");
        let benchmark = ExecutionBenchmark::new(100_000, 100_500, 99_950, 100_050, Some(100_750))
            .expect("benchmark");

        let snapshot = ExecutionQualityAnalyzer::evaluate(trade, benchmark);

        assert_eq!(snapshot.arrival_slippage_bps(), 100);
        assert_eq!(snapshot.decision_slippage_bps(), 49);
        assert_eq!(snapshot.implementation_shortfall_bps(), 49);
        assert_eq!(snapshot.adverse_selection_bps(), 24);
        assert!(snapshot.trade_through());
        assert!(snapshot.fill_quality_score_bps() < 10_000);
    }

    #[test]
    fn execution_quality_scores_sell_price_improvement() {
        let trade = TradeContext::new(100_500, 10, Side::Bid, 1).expect("trade");
        let benchmark = ExecutionBenchmark::new(100_000, 100_000, 99_950, 100_050, Some(100_250))
            .expect("benchmark");

        let snapshot = ExecutionQualityAnalyzer::evaluate(trade, benchmark);

        assert!(snapshot.arrival_slippage_bps() < 0);
        assert!(snapshot.decision_slippage_bps() < 0);
        assert!(snapshot.adverse_selection_bps() < 0);
        assert!(!snapshot.trade_through());
        assert_eq!(snapshot.fill_quality_score_bps(), 10_000);
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
        assert_eq!(snapshot.buy_sweepability_bps(), 10_000);
        assert_eq!(snapshot.sell_sweepability_bps(), 10_000);
        assert_eq!(snapshot.sweepability_score_bps(), 10_000);
        assert!(snapshot.proportional_imbalance_bps() < 0);
    }

    #[test]
    fn liquidity_depth_reports_shape_pressure_and_partial_sweepability() {
        let bids = [
            BookLevel {
                level: 0,
                price: 500_000,
                size: 300,
            },
            BookLevel {
                level: 1,
                price: 499_975,
                size: 200,
            },
            BookLevel {
                level: 2,
                price: 499_950,
                size: 100,
            },
        ];
        let asks = [
            BookLevel {
                level: 0,
                price: 500_025,
                size: 50,
            },
            BookLevel {
                level: 1,
                price: 500_050,
                size: 100,
            },
            BookLevel {
                level: 2,
                price: 500_075,
                size: 150,
            },
        ];

        let snapshot = LiquidityDepthAnalyzer::new(3)
            .analyze(&bids, &asks, 500)
            .expect("snapshot");

        assert_eq!(snapshot.depth_slope_bps(), -2_857);
        assert_eq!(snapshot.depth_convexity_bps(), 0);
        assert!(snapshot.book_pressure_bps() > 0);
        assert_eq!(snapshot.buy_sweepability_bps(), 6_000);
        assert_eq!(snapshot.sell_sweepability_bps(), 10_000);
        assert_eq!(snapshot.sweepability_score_bps(), 6_000);
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

    #[test]
    fn liquidity_flow_tracks_imbalance_rates_and_drought() {
        let config = LiquidityFlowConfig::new(1_000_000_000, 2_500, 100).unwrap();
        let mut tracker = LiquidityFlowTracker::new(config);

        tracker.on_event(LiquidityFlowEvent::new(Side::Bid, 100, 0, 0, 1).unwrap());
        tracker.on_event(LiquidityFlowEvent::new(Side::Ask, 0, 250, 50, 500_000_001).unwrap());

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.events(), 2);
        assert_eq!(snapshot.bid_added_qty(), 100);
        assert_eq!(snapshot.ask_depleted_qty(), 250);
        assert_eq!(snapshot.ask_traded_qty(), 50);
        assert_eq!(snapshot.order_flow_imbalance_bps(), 10_000);
        assert_eq!(snapshot.replenishment_rate_per_sec(), 200);
        assert_eq!(snapshot.depletion_rate_per_sec(), 600);
        assert!(!snapshot.liquidity_drought());

        tracker.on_event(LiquidityFlowEvent::new(Side::Bid, 0, 600, 0, 1_000_000_001).unwrap());
        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.depletion_rate_per_sec(), 900);
        assert!(snapshot.liquidity_drought());

        tracker.reset();
        assert_eq!(tracker.snapshot().events(), 0);
    }

    #[test]
    fn liquidity_flow_rejects_empty_or_invalid_events() {
        assert_eq!(
            LiquidityFlowEvent::new(Side::Bid, 0, 0, 0, 1),
            Err(AnalyticsError::InvalidDepth)
        );
        assert_eq!(
            LiquidityFlowEvent::new(Side::Ask, 1, 0, 0, 0),
            Err(AnalyticsError::InvalidDepth)
        );
        assert_eq!(
            LiquidityFlowConfig::new(0, 0, 0),
            Err(AnalyticsError::InvalidDepth)
        );
    }

    #[test]
    fn impact_tracker_computes_kyle_lambda() {
        let mut tracker = ImpactTracker::new();
        tracker.on_sample(ImpactSample::new(500_000, 501_000, 100, 50_000_000).expect("sample"));
        tracker.on_sample(ImpactSample::new(501_000, 500_500, -50, 25_000_000).expect("sample"));

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 2);
        assert_eq!(snapshot.signed_volume(), 50);
        assert_eq!(snapshot.absolute_volume(), 150);
        assert_eq!(snapshot.signed_price_change(), 1_500);
        assert!(snapshot.kyle_lambda_ppm() > 0);
    }

    #[test]
    fn expected_impact_estimator_combines_calibrated_components() {
        let calibration = ImpactCalibration::new(1_000_000, 200, 10_000, 500, 250, 1_000_000_000)
            .expect("calibration");
        let input = ExpectedImpactInput::new(
            Side::Ask,
            1_000,
            10_000,
            100_000,
            1_000_000_000,
            calibration,
        )
        .expect("input");

        let snapshot = ExpectedImpactEstimator::estimate(input);

        assert_eq!(snapshot.participation_bps(), 1_000);
        assert_eq!(snapshot.daily_participation_bps(), 10);
        assert_eq!(snapshot.square_root_impact_bps(), 6);
        assert_eq!(snapshot.temporary_impact_bps(), 50);
        assert_eq!(snapshot.permanent_impact_bps(), 25);
        assert_eq!(snapshot.instantaneous_impact_bps(), 75);
        assert_eq!(snapshot.decay_remaining_bps(), 5_000);
        assert_eq!(snapshot.expected_total_impact_bps(), 56);
        assert_eq!(snapshot.expected_signed_price_move(), 560);
    }

    #[test]
    fn child_order_impact_attribution_is_side_aware() {
        let buy =
            ChildOrderImpactContext::new(Side::Ask, 1_000, 100, 100_000, 100_500, 100_800, 100_200)
                .expect("buy context");
        let buy_snapshot = ChildOrderImpactAnalyzer::evaluate(buy);

        assert_eq!(buy_snapshot.child_participation_bps(), 1_000);
        assert_eq!(buy_snapshot.child_slippage_bps(), 50);
        assert_eq!(buy_snapshot.instantaneous_impact_bps(), 80);
        assert_eq!(buy_snapshot.permanent_impact_bps(), 20);
        assert_eq!(buy_snapshot.temporary_impact_bps(), 30);
        assert_eq!(buy_snapshot.impact_decay_bps(), 60);
        assert_eq!(buy_snapshot.attributed_impact_bps(), 5);

        let sell =
            ChildOrderImpactContext::new(Side::Bid, 1_000, 100, 100_000, 99_500, 99_200, 99_800)
                .expect("sell context");
        assert_eq!(
            ChildOrderImpactAnalyzer::evaluate(sell).child_slippage_bps(),
            50
        );
    }

    #[test]
    fn impact_primitives_reject_invalid_inputs() {
        assert_eq!(
            ImpactCalibration::new(0, 0, 0, 0, 0, 0),
            Err(AnalyticsError::InvalidTrade)
        );
        let calibration = ImpactCalibration::new(1_000, 100, 10_000, 100, 100, 1).unwrap();
        assert_eq!(
            ExpectedImpactInput::new(Side::Ask, 0, 1, 1, 1, calibration),
            Err(AnalyticsError::InvalidTrade)
        );
        assert_eq!(
            ChildOrderImpactContext::new(Side::Ask, 100, 101, 1, 1, 1, 1),
            Err(AnalyticsError::InvalidTrade)
        );
    }

    #[test]
    fn vpin_tracker_closes_fixed_buckets() {
        let mut tracker = VpinTracker::<2>::new(100).expect("tracker");
        tracker.on_trade(TradeContext::new(500_000, 80, Side::Ask, 1).expect("trade"));
        tracker.on_trade(TradeContext::new(500_000, 20, Side::Bid, 2).expect("trade"));
        tracker.on_trade(TradeContext::new(500_000, 100, Side::Bid, 3).expect("trade"));

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.bucket_count(), 2);
        assert_eq!(snapshot.current_bucket_volume(), 0);
        assert_eq!(snapshot.vpin_bps(), 8_000);
        assert_eq!(snapshot.toxicity_bps(), snapshot.vpin_bps());
    }

    #[test]
    fn toxicity_analyzer_detects_adverse_burst_and_quote_fade() {
        let analyzer = ToxicityAnalyzer::new(ToxicityConfig::new(20, 3_000, 7_000, 6_000).unwrap());
        let input = ToxicityInput::new(
            TradeContext::new(100_000, 10, Side::Ask, 1).unwrap(),
            100_500,
            1_000,
            1_000,
            1_000,
            500,
            8_000,
            7_000,
        )
        .unwrap();

        let snapshot = analyzer.evaluate(input);

        assert_eq!(snapshot.post_trade_markout_bps(), 50);
        assert_eq!(snapshot.quote_fade_bps(), 5_000);
        assert_eq!(snapshot.adverse_selection_score_bps(), 10_000);
        assert!(snapshot.informed_flow_proxy_bps() > 8_000);
        assert!(snapshot.toxic_flow_burst());
        assert!(snapshot.toxicity_score_bps() >= snapshot.informed_flow_proxy_bps());
    }

    #[test]
    fn toxicity_analyzer_handles_favorable_markout() {
        let input = ToxicityInput::new(
            TradeContext::new(100_000, 10, Side::Bid, 1).unwrap(),
            100_500,
            1_000,
            1_000,
            700,
            1_000,
            1_000,
            1_000,
        )
        .unwrap();

        let snapshot = ToxicityAnalyzer::default().evaluate(input);

        assert!(snapshot.post_trade_markout_bps() < 0);
        assert_eq!(snapshot.adverse_selection_score_bps(), 0);
        assert_eq!(snapshot.quote_fade_bps(), 3_000);
        assert!(!snapshot.toxic_flow_burst());
    }

    #[test]
    fn toxicity_primitives_reject_invalid_inputs() {
        assert_eq!(
            ToxicityConfig::new(0, 1, 1, 1),
            Err(AnalyticsError::InvalidTrade)
        );
        assert_eq!(
            ToxicityInput::new(
                TradeContext::new(1, 1, Side::Ask, 1).unwrap(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ),
            Err(AnalyticsError::InvalidTrade)
        );
    }

    #[test]
    fn volatility_tracker_computes_realized_noise() {
        let mut tracker = VolatilityTracker::<4>::new().expect("tracker");
        tracker.on_price(100_000).expect("price");
        tracker.on_price(101_000).expect("price");
        tracker.on_price(100_000).expect("price");
        tracker.on_price(101_000).expect("price");

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 3);
        assert_eq!(snapshot.last_price(), 101_000);
        assert!(snapshot.realized_vol_bps() > 0);
        assert!(snapshot.mean_abs_return_bps() > 0);
        assert!(snapshot.bipower_vol_bps() > 0);
        assert!(snapshot.jump_variation_bps() <= snapshot.realized_vol_bps());
        assert!(snapshot.noise_ratio_bps() > 0);
    }

    #[test]
    fn volatility_tracker_preserves_ring_order_for_noise() {
        let mut tracker = VolatilityTracker::<3>::new().expect("tracker");
        for price in [100_000, 101_000, 102_000, 101_000, 100_000] {
            tracker.on_price(price).expect("price");
        }

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 3);
        assert_eq!(snapshot.noise_ratio_bps(), 5_000);
    }

    #[test]
    fn ohlc_volatility_estimator_reports_range_estimators() {
        let input = OhlcVolatilityInput::new(100_000, 102_000, 99_000, 101_000, Some(99_500))
            .expect("ohlc");
        let snapshot = OhlcVolatilityEstimator::estimate(input);

        assert_eq!(snapshot.close_to_close_vol_bps(), 150);
        assert_eq!(snapshot.jump_gap_bps(), 50);
        assert!(snapshot.parkinson_vol_bps() > 0);
        assert!(snapshot.garman_klass_vol_bps() > 0);
        assert!(snapshot.rogers_satchell_vol_bps() > 0);
        assert_eq!(
            OhlcVolatilityInput::new(100_000, 99_000, 100_000, 100_000, None),
            Err(AnalyticsError::InvalidTrade)
        );
    }

    #[test]
    fn volatility_signature_estimator_reports_noise() {
        let snapshot = VolatilitySignatureEstimator::estimate(1_000_000, &[10, -10, 20, -20])
            .expect("signature");

        assert_eq!(snapshot.sampling_interval_ns(), 1_000_000);
        assert_eq!(snapshot.samples(), 4);
        assert!(snapshot.realized_vol_bps() > 0);
        assert_eq!(snapshot.noise_ratio_bps(), 10_000);
        assert_eq!(
            VolatilitySignatureEstimator::estimate(0, &[1]),
            Err(AnalyticsError::InvalidTrade)
        );
    }

    #[test]
    fn volatility_seasonality_tracker_accumulates_buckets() {
        let mut tracker = VolatilitySeasonalityTracker::<2>::new(25).expect("tracker");
        tracker.on_return(0, 10).expect("return");
        tracker.on_return(0, -30).expect("return");
        tracker.on_return(1, 5).expect("return");

        let bucket = tracker.snapshot(0).expect("bucket");

        assert_eq!(bucket.bucket(), 0);
        assert_eq!(bucket.samples(), 2);
        assert_eq!(bucket.mean_abs_return_bps(), 20);
        assert_eq!(bucket.jump_count(), 1);
        assert!(bucket.realized_vol_bps() > 0);
        assert_eq!(tracker.on_return(2, 1), Err(AnalyticsError::InvalidTrade));

        tracker.reset();
        assert_eq!(tracker.snapshot(0).unwrap().samples(), 0);
    }

    #[test]
    fn regime_classifier_prioritizes_toxicity_and_illiquidity() {
        let classifier = RegimeClassifier::default();

        assert_eq!(
            classifier
                .classify(RegimeInput::new(1, 10, 8_000, 0))
                .kind(),
            RegimeKind::Toxic
        );
        assert_eq!(
            classifier.classify(RegimeInput::new(50, 10, 0, 0)).kind(),
            RegimeKind::Illiquid
        );
        assert_eq!(
            classifier.classify(RegimeInput::new(1, 100, 0, 0)).kind(),
            RegimeKind::Volatile
        );
    }

    #[test]
    fn composite_regime_classifier_detects_news_hidden_chop() {
        let classifier = CompositeRegimeClassifier::default();
        let snapshot = classifier.classify(
            CompositeRegimeInput::new(
                2_000,
                8_000,
                50,
                100,
                50,
                1_000_000_000,
                3_600_000_000_000,
                false,
                8_000,
                8_000,
            )
            .expect("input"),
        );

        assert_eq!(snapshot.trend(), TrendRegimeKind::Chop);
        assert_eq!(snapshot.liquidity(), LiquidityRegimeKind::Hidden);
        assert_eq!(snapshot.spread(), SpreadRegimeKind::Wide);
        assert_eq!(snapshot.session(), SessionRegimeKind::NewsShock);
        assert!(snapshot.volatile());
        assert_eq!(snapshot.hidden_liquidity_proxy_bps(), 8_000);
        assert!(snapshot.transition_confidence_bps() > 0);
    }

    #[test]
    fn composite_regime_classifier_detects_continuous_trend() {
        let config = CompositeRegimeConfig::default();
        let classifier = CompositeRegimeClassifier::new(config);
        let snapshot = classifier.classify(
            CompositeRegimeInput::new(
                8_000,
                1_000,
                1,
                20,
                2_000,
                config.open_window_ns().saturating_add(60_000_000_000),
                config.close_window_ns().saturating_add(60_000_000_000),
                false,
                0,
                0,
            )
            .expect("input"),
        );

        assert_eq!(snapshot.trend(), TrendRegimeKind::Trend);
        assert_eq!(snapshot.liquidity(), LiquidityRegimeKind::Deep);
        assert_eq!(snapshot.spread(), SpreadRegimeKind::Tight);
        assert_eq!(snapshot.session(), SessionRegimeKind::Continuous);
        assert!(!snapshot.volatile());
    }

    #[test]
    fn composite_regime_primitives_reject_invalid_inputs() {
        assert_eq!(
            CompositeRegimeConfig::new(0, 0, 10, 20, 0, 0, 0, 0, 0, 0, 0),
            Err(AnalyticsError::InvalidTrade)
        );
        assert_eq!(
            CompositeRegimeInput::new(10_001, 0, 0, 0, 0, 0, 0, false, 0, 0),
            Err(AnalyticsError::InvalidTrade)
        );
    }

    #[test]
    fn feed_quality_tracks_sequence_and_book_degradation() {
        let config = FeedQualityConfig::new(10, 20, 1).expect("config");
        let mut tracker = FeedQualityTracker::new(config);

        assert!(tracker
            .on_event(FeedQualityEvent::new(Some(10), 100, 105, Some(99), Some(101)).unwrap())
            .is_ok());
        let gap = tracker
            .on_event(FeedQualityEvent::new(Some(12), 110, 115, Some(100), Some(100)).unwrap());
        assert!(gap.contains(FeedQualityFlags::SEQUENCE_GAP));
        assert!(gap.contains(FeedQualityFlags::LOCKED_BOOK));
        let crossed = tracker
            .on_event(FeedQualityEvent::new(Some(12), 111, 200, Some(102), Some(101)).unwrap());
        assert!(crossed.contains(FeedQualityFlags::DUPLICATE));
        assert!(crossed.contains(FeedQualityFlags::STALE));
        assert!(crossed.contains(FeedQualityFlags::TIMESTAMP_SKEW));
        assert!(crossed.contains(FeedQualityFlags::CROSSED_BOOK));

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.events(), 3);
        assert_eq!(snapshot.sequence_gap_events(), 1);
        assert_eq!(snapshot.sequence_gap_units(), 1);
        assert_eq!(snapshot.duplicate_events(), 1);
        assert_eq!(snapshot.locked_book_events(), 1);
        assert_eq!(snapshot.crossed_book_events(), 1);
        assert_eq!(snapshot.stale_events(), 1);
        assert_eq!(snapshot.timestamp_skew_events(), 1);
        assert_eq!(snapshot.last_sequence(), Some(12));
        assert!(snapshot.health_score_bps() < 10_000);
        assert_eq!(snapshot.sequence_gap_rate_bps(), 3_333);
    }

    #[test]
    fn feed_quality_tracks_out_of_order_and_resets() {
        let mut tracker = FeedQualityTracker::default();
        tracker.on_event(FeedQualityEvent::new(Some(100), 100, 100, None, None).unwrap());

        let old = tracker.on_event(FeedQualityEvent::new(Some(90), 90, 100, None, None).unwrap());
        assert!(old.contains(FeedQualityFlags::OUT_OF_ORDER));

        let reset = tracker.on_event(FeedQualityEvent::new(Some(1), 101, 101, None, None).unwrap());
        assert!(reset.contains(FeedQualityFlags::SEQUENCE_RESET));

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.out_of_order_events(), 2);
        assert_eq!(snapshot.sequence_reset_events(), 1);
        assert_eq!(snapshot.last_sequence(), Some(1));
        assert_eq!(snapshot.last_event_ts_ns(), 101);
    }

    #[test]
    fn feature_schema_registers_stable_order_and_hash() {
        let mut schema = FeatureSchema::<4>::new().expect("schema");
        let spread = FeatureDefinition::new(
            FeatureId::new(1).unwrap(),
            "spread_bps",
            FeatureUnit::BasisPoints,
            1,
            MissingValuePolicy::Sentinel(i64::MIN),
        )
        .unwrap();
        let quality = FeatureDefinition::new(
            FeatureId::new(2).unwrap(),
            "quality_bps",
            FeatureUnit::ScoreBasisPoints,
            1,
            MissingValuePolicy::Zero,
        )
        .unwrap();

        assert_eq!(schema.register(spread), Ok(0));
        assert_eq!(schema.register(quality), Ok(1));
        assert_eq!(schema.register(spread), Err(AnalyticsError::InvalidFeature));

        assert_eq!(schema.len(), 2);
        assert_eq!(schema.index_of(FeatureId::new(2).unwrap()), Some(1));
        assert_eq!(schema.definition(0).unwrap().name(), "spread_bps");
        assert_ne!(schema.schema_hash(), 0);
    }

    #[test]
    fn feature_vector_writer_reuses_schema_defaults() {
        let mut schema = FeatureSchema::<2>::new().expect("schema");
        schema
            .register(
                FeatureDefinition::new(
                    FeatureId::new(1).unwrap(),
                    "spread_bps",
                    FeatureUnit::BasisPoints,
                    1,
                    MissingValuePolicy::Sentinel(-1),
                )
                .unwrap(),
            )
            .unwrap();
        schema
            .register(
                FeatureDefinition::new(
                    FeatureId::new(2).unwrap(),
                    "is_stale",
                    FeatureUnit::Boolean,
                    1,
                    MissingValuePolicy::Zero,
                )
                .unwrap(),
            )
            .unwrap();

        let mut writer = FeatureVectorWriter::new(&schema);
        assert_eq!(writer.finish().values(), &[-1, 0]);
        writer.set(0, 25, FeatureQuality::Good).unwrap();
        assert_eq!(
            writer.set(2, 1, FeatureQuality::Good),
            Err(AnalyticsError::InvalidFeature)
        );

        let vector = writer.finish();

        assert_eq!(vector.len(), 2);
        assert_eq!(vector.value(0), Some(25));
        assert_eq!(vector.value(1), Some(0));
        assert_eq!(vector.quality(0), Some(FeatureQuality::Good));
        assert_eq!(vector.quality(1), Some(FeatureQuality::Missing));
        assert_eq!(vector.schema_hash(), schema.schema_hash());
    }

    #[test]
    fn resiliency_tracker_measures_spread_recovery() {
        let config = ResiliencyConfig::new(5, 1_000, 25, 5_000, 8, 9_000).expect("config");
        let mut tracker = ResiliencyTracker::new(config);

        let quiet = tracker.on_sample(ResiliencySample::new(100, 5, 500, 500).unwrap());
        assert!(!quiet.active_shock());
        assert_eq!(quiet.score_bps(), 10_000);

        let shock = tracker.on_sample(ResiliencySample::new(200, 30, 400, 400).unwrap());
        assert!(shock.active_shock());
        assert_eq!(shock.shock_count(), 1);
        assert_eq!(shock.last_shock_ts_ns(), 200);
        assert_eq!(shock.max_spread_bps(), 30);

        let recovery = tracker.on_sample(ResiliencySample::new(1_000_200, 7, 500, 500).unwrap());
        assert!(!recovery.active_shock());
        assert_eq!(recovery.recovery_count(), 1);
        assert_eq!(recovery.last_recovery_time_ns(), 1_000_000);
        assert!(recovery.score_bps() < 10_000);
    }

    #[test]
    fn resiliency_tracker_detects_depth_depletion() {
        let config = ResiliencyConfig::new(5, 1_000, 25, 5_000, 8, 9_000).expect("config");
        let mut tracker = ResiliencyTracker::new(config);

        let shock = tracker.on_sample(ResiliencySample::new(100, 5, 200, 200).unwrap());
        assert!(shock.active_shock());
        assert_eq!(shock.min_depth(), 400);

        tracker.reset();
        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 0);
        assert!(!snapshot.active_shock());
        assert_eq!(snapshot.min_depth(), 0);
        assert_eq!(snapshot.score_bps(), 10_000);
    }

    #[test]
    fn queue_fill_tracker_updates_on_trade_and_cancel() {
        let config = QueueFillConfig::new(5_000, 100, 1_000_000_000).expect("config");
        let estimate = QueuePositionEstimate::new(100, 50, 200, 1).expect("estimate");
        let mut tracker = QueueFillTracker::new(config, estimate);

        let after_trade = tracker
            .on_update(QueueFillUpdate::new(QueueUpdateKind::Trade, 40, 160, 2).expect("update"));
        assert_eq!(after_trade.qty_ahead(), 60);
        assert_eq!(after_trade.own_qty_remaining(), 50);
        assert!(after_trade.fill_probability_bps() > 0);
        assert_eq!(after_trade.expected_time_to_fill_ns(), 1_100_000_000);

        let after_cancel = tracker
            .on_update(QueueFillUpdate::new(QueueUpdateKind::Cancel, 20, 140, 3).expect("update"));
        assert_eq!(after_cancel.qty_ahead(), 50);
        assert!(after_cancel.top_level_survival_bps() > 0);
    }

    #[test]
    fn queue_fill_tracker_tracks_amend_queue_loss() {
        let config = QueueFillConfig::new(0, 100, 1_000_000_000).expect("config");
        let estimate = QueuePositionEstimate::new(10, 10, 20, 1).expect("estimate");
        let mut tracker = QueueFillTracker::new(config, estimate);

        let snapshot = tracker
            .on_update(QueueFillUpdate::new(QueueUpdateKind::Amend, 0, 100, 2).expect("update"));

        assert_eq!(snapshot.qty_ahead(), 100);
        assert_eq!(snapshot.queue_loss_after_amend(), 100);
        assert_eq!(snapshot.last_update_ts_ns(), 2);
        assert!(snapshot.maker_taker_score_bps() < 10_000);
    }

    #[test]
    fn pattern_risk_flags_layering_and_quote_stuffing() {
        let classifier = PatternRiskClassifier::default();
        let snapshot = classifier.classify(
            PatternRiskInput::new(
                800,
                900,
                10,
                8_000,
                5,
                PatternRiskLiquidity::new(10, 1_000).unwrap(),
                1_000_000,
            )
            .unwrap(),
        );

        assert!(snapshot.spoofing_layering_risk_bps() > 5_000);
        assert_eq!(snapshot.quote_stuffing_risk_bps(), 10_000);
        assert_eq!(
            snapshot.overall_risk_bps(),
            snapshot.quote_stuffing_risk_bps()
        );
    }

    #[test]
    fn pattern_risk_scores_stop_run_and_absorption() {
        let classifier = PatternRiskClassifier::default();
        let stop_run = classifier.classify(
            PatternRiskInput::new(
                10,
                10,
                100,
                1_000,
                50,
                PatternRiskLiquidity::new(2_000, 1_000).unwrap(),
                1_000_000,
            )
            .unwrap(),
        );
        let absorption = classifier.classify(
            PatternRiskInput::new(
                10,
                10,
                100,
                1_000,
                1,
                PatternRiskLiquidity::new(2_000, 1_000).unwrap(),
                1_000_000,
            )
            .unwrap(),
        );

        assert!(stop_run.stop_run_risk_bps() > absorption.stop_run_risk_bps());
        assert!(absorption.absorption_risk_bps() > stop_run.absorption_risk_bps());
    }

    #[test]
    fn pattern_detail_analyzer_detects_iceberg_and_failed_breakout() {
        let analyzer = PatternDetailAnalyzer::default();
        let snapshot = analyzer.evaluate(
            PatternDetailInput::new(8, 4, 1_000, 100, 4, 8_000, 500, 2, 30, 35, 1_000)
                .expect("input"),
        );

        assert!(snapshot.iceberg_risk_bps() > 0);
        assert_eq!(snapshot.hidden_distribution_bps(), 0);
        assert!(snapshot.hidden_accumulation_bps() > 0);
        assert_eq!(snapshot.stacked_imbalance_risk_bps(), 10_000);
        assert!(snapshot.absorption_strength_bps() > 0);
        assert_eq!(snapshot.failed_breakout_risk_bps(), 10_000);
        assert_eq!(snapshot.overall_risk_bps(), 10_000);
    }

    #[test]
    fn pattern_detail_analyzer_detects_hidden_distribution() {
        let snapshot = PatternDetailAnalyzer::default()
            .evaluate(PatternDetailInput::new(3, 3, 500, 100, 0, 0, 0, 0, 0, 0, -500).unwrap());

        assert_eq!(snapshot.hidden_accumulation_bps(), 0);
        assert!(snapshot.hidden_distribution_bps() > 0);
    }

    #[test]
    fn pattern_detail_primitives_reject_invalid_inputs() {
        assert_eq!(
            PatternDetailConfig::new(0, 10_001, 0, 0),
            Err(AnalyticsError::InvalidPattern)
        );
        assert_eq!(
            PatternDetailInput::new(0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0),
            Err(AnalyticsError::InvalidPattern)
        );
    }

    #[test]
    fn venue_route_tracker_computes_rates_and_latency() {
        let mut tracker = VenueRouteTracker::new();
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Sent, 100, 0, 10).unwrap());
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Fill, 60, 100, 20).unwrap());
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Reject, 0, 0, 30).unwrap());
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Cancel, 40, 0, 40).unwrap());

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.sent(), 1);
        assert_eq!(snapshot.fills(), 1);
        assert_eq!(snapshot.rejects(), 1);
        assert_eq!(snapshot.cancels(), 1);
        assert_eq!(snapshot.sent_qty(), 100);
        assert_eq!(snapshot.filled_qty(), 60);
        assert_eq!(snapshot.fill_rate_bps(), 3_333);
        assert_eq!(snapshot.avg_quote_to_fill_latency_ns(), 100);
        assert_eq!(snapshot.max_market_data_to_order_latency_ns(), 40);
        assert!(snapshot.route_health_bps() < snapshot.fill_rate_bps());
    }

    #[test]
    fn venue_route_tracker_resets_state() {
        let mut tracker = VenueRouteTracker::new();
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Sent, 100, 0, 10).unwrap());

        tracker.reset();

        assert_eq!(tracker.snapshot().sent(), 0);
        assert_eq!(tracker.snapshot().route_health_bps(), 0);
    }

    #[test]
    fn cross_asset_tracker_computes_correlation_beta_and_basis() {
        let mut tracker =
            CrossAssetTracker::<4>::new(CrossAssetConfig::default()).expect("tracker");
        tracker.on_sample(CrossAssetSample::new(100_000, 200_000, 1).unwrap());
        tracker.on_sample(CrossAssetSample::new(101_000, 202_000, 2).unwrap());
        tracker.on_sample(CrossAssetSample::new(102_000, 204_000, 3).unwrap());

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 2);
        assert_eq!(snapshot.last_ts_ns(), 3);
        assert!(snapshot.correlation_bps() > 9_000);
        assert!(snapshot.beta_bps() > 9_000);
        assert!(!snapshot.correlation_breakdown());
        assert!(snapshot.basis_pressure_bps() < 0);
        assert!(snapshot.basis_pressure());
        assert!(snapshot.lead_lag_score_bps() > 9_000);
    }

    #[test]
    fn cross_asset_tracker_flags_divergence_and_resets() {
        let config = CrossAssetConfig::new(50, 25, 2_000).expect("config");
        let mut tracker = CrossAssetTracker::<3>::new(config).expect("tracker");
        tracker.on_sample(CrossAssetSample::new(100_000, 100_000, 1).unwrap());
        tracker.on_sample(CrossAssetSample::new(101_000, 99_000, 2).unwrap());
        tracker.on_sample(CrossAssetSample::new(102_000, 98_000, 3).unwrap());
        tracker.on_sample(CrossAssetSample::new(103_000, 97_000, 4).unwrap());

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 3);
        assert!(snapshot.correlation_bps() < 0);
        assert!(snapshot.pair_divergence_bps() < 0);
        assert!(snapshot.pair_divergence());

        tracker.reset();

        let reset = tracker.snapshot();
        assert_eq!(reset.samples(), 0);
        assert_eq!(reset.last_ts_ns(), 0);
        assert_eq!(reset.correlation_bps(), 0);
    }

    #[test]
    fn option_flow_tracker_computes_pressure_iv_and_gamma() {
        let mut tracker = OptionFlowTracker::new();
        tracker.on_sample(
            OptionFlowSample::new(OptionKind::Call, 100, 1_000, 50_000, 2_000, 1_000)
                .expect("call"),
        );
        tracker.on_sample(
            OptionFlowSample::new(OptionKind::Put, 200, 1_500, 150_000, 3_000, -2_000)
                .expect("put"),
        );

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.call_volume(), 100);
        assert_eq!(snapshot.put_volume(), 200);
        assert_eq!(snapshot.put_call_volume_ratio_bps(), 20_000);
        assert_eq!(snapshot.put_call_open_interest_ratio_bps(), 15_000);
        assert_eq!(snapshot.put_call_premium_ratio_bps(), 30_000);
        assert_eq!(snapshot.implied_vol_flow_bps(), 2_750);
        assert_eq!(snapshot.net_gamma_exposure(), -1_000);
        assert!(snapshot.put_call_pressure_bps() > 0);

        tracker.reset();
        assert_eq!(tracker.snapshot().call_volume(), 0);
    }

    #[test]
    fn futures_basis_analyzer_computes_roll_and_funding_divergence() {
        let snapshot = FuturesBasisAnalyzer::analyze(
            FuturesBasisInput::new(100_000, 101_000, 100_500, 101_000, 102_000, 25).expect("basis"),
        );

        assert_eq!(snapshot.basis_bps(), 100);
        assert_eq!(snapshot.fair_value_gap_bps(), 49);
        assert_eq!(snapshot.calendar_spread_bps(), 99);
        assert_eq!(snapshot.roll_pressure_bps(), -1);
        assert_eq!(snapshot.funding_basis_divergence_bps(), 75);
    }
}
