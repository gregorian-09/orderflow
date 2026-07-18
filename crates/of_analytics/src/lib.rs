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
}
