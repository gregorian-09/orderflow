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

/// Rolling volatility/noise snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilitySnapshot {
    samples: usize,
    last_price: i64,
    realized_vol_bps: u32,
    mean_abs_return_bps: u32,
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
                noise_ratio_bps: 0,
            };
        }
        let mut sum_sq = 0_u128;
        let mut sum_abs = 0_u128;
        let mut sign_flips = 0_u32;
        let mut prev_sign = 0_i32;
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
            noise_ratio_bps,
        }
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
}
