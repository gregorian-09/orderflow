# `of_analytics` Reference

`of_analytics` is the additive advanced analytics crate for Orderflow. It
keeps heavier microstructure modules out of `of_core` so users who only need
canonical market-data types and the basic accumulator do not pay compile-time
or dependency cost for every advanced model.

The first public slice is dependency-light and live-path friendly:

- market-quality/TCA analytics,
- liquidity/depth analytics,
- market-impact analytics,
- VPIN-style toxicity analytics,
- fixed-window volatility/noise analytics,
- threshold-based regime classification,
- feature profiles for future impact, toxicity, volatility, regime, pattern,
  derivatives, institutional, and ML-feature modules.

The crate consumes normalized `of_core` data and returns typed snapshots. It
does not own runtime state, persistence, sockets, bindings, OMS state, or order
submission.

## Feature Profiles

Default features:

- `market-quality`
- `liquidity`

Reserved additive profiles:

- `impact`
- `toxicity`
- `volatility`
- `regime`
- `patterns`
- `derivatives`
- `institutional`
- `ml-features`
- `all`

## Public Types

Market quality:

- `AnalyticsError`
- `QuoteContext`
- `TradeContext`
- `MarketQualitySnapshot`
- `MarketQualityTracker`

Liquidity/depth:

- `LiquidityDepthSnapshot`
- `LiquidityDepthAnalyzer`

Impact/toxicity:

- `ImpactSample`
- `ImpactSnapshot`
- `ImpactTracker`
- `VpinSnapshot`
- `VpinTracker`
- `VolatilitySnapshot`
- `VolatilityTracker`
- `RegimeKind`
- `RegimeInput`
- `RegimeSnapshot`
- `RegimeClassifier`

## Market Quality

`MarketQualityTracker` keeps the latest quote and evaluates a trade against
that quote. It reports:

- quoted spread,
- quoted spread in basis points,
- effective spread in basis points,
- realized spread in basis points when a future midpoint is supplied,
- price improvement in basis points,
- stale-quote flag.

Quote freshness is explicit through `max_quote_age_ns`. If trade/quote age
exceeds that window, the snapshot marks the quote stale instead of hiding the
data-quality problem.

```rust
use of_analytics::{MarketQualityTracker, QuoteContext, TradeContext};
use of_core::Side;

let mut tracker = MarketQualityTracker::new(1_000_000);
tracker.on_quote(QuoteContext::new(499_000, 501_000, 100, 120, 1_000)?);
let snapshot = tracker.evaluate_trade(
    TradeContext::new(500_000, 10, Side::Ask, 1_500)?,
    Some(500_010),
)?;

assert_eq!(snapshot.quoted_spread(), 2_000);
assert!(snapshot.price_improvement_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Liquidity And Depth

`LiquidityDepthAnalyzer` evaluates borrowed `of_core::BookLevel` slices. It
does not copy the book and does not allocate in the analysis path.

The snapshot reports:

- levels used,
- top bid and ask quantity,
- cumulative bid depth,
- cumulative ask depth,
- bid-minus-ask proportional imbalance,
- simple depth slope proxy,
- quantity sweepable by a buy order up to a target quantity,
- quantity sweepable by a sell order up to a target quantity.

```rust
use of_analytics::LiquidityDepthAnalyzer;
use of_core::BookLevel;

let bids = [
    BookLevel { level: 0, price: 499_975, size: 100 },
    BookLevel { level: 1, price: 499_950, size: 80 },
];
let asks = [
    BookLevel { level: 0, price: 500_025, size: 120 },
    BookLevel { level: 1, price: 500_050, size: 90 },
];
let snapshot = LiquidityDepthAnalyzer::new(2).analyze(&bids, &asks, 150)?;

assert_eq!(snapshot.bid_depth(), 180);
assert_eq!(snapshot.ask_depth(), 210);
assert_eq!(snapshot.sweepable_buy_qty(), 150);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Market Impact

`ImpactTracker` accumulates explicit interval samples. It reports:

- sample count,
- signed volume,
- absolute volume,
- signed price change,
- Kyle-style lambda scaled by 1,000,000,
- Amihud-style illiquidity scaled by 1,000,000.

The tracker does not run regressions or allocate rolling matrices. More
advanced batch estimators can be added behind feature profiles without changing
this live-path accumulator.

```rust
use of_analytics::{ImpactSample, ImpactTracker};

let mut tracker = ImpactTracker::new();
tracker.on_sample(ImpactSample::new(500_000, 501_000, 100, 50_000_000)?);
let snapshot = tracker.snapshot();

assert_eq!(snapshot.samples(), 1);
assert!(snapshot.kyle_lambda_ppm() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## VPIN-Style Toxicity

`VpinTracker` is a fixed-capacity bucket tracker. `Side::Ask` is interpreted as
buyer-initiated flow and `Side::Bid` as seller-initiated flow. Completed bucket
imbalances are retained in a const-generic ring buffer.

```rust
use of_analytics::{TradeContext, VpinTracker};
use of_core::Side;

let mut tracker = VpinTracker::<4>::new(100)?;
tracker.on_trade(TradeContext::new(500_000, 80, Side::Ask, 1)?);
tracker.on_trade(TradeContext::new(500_000, 20, Side::Bid, 2)?);
let snapshot = tracker.snapshot();

assert_eq!(snapshot.bucket_count(), 1);
assert_eq!(snapshot.vpin_bps(), 6_000);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Volatility And Noise

`VolatilityTracker` stores a fixed-size ring of integer return samples. It
reports realized volatility, mean absolute return, and a simple noise proxy
based on return sign flips.

```rust
use of_analytics::VolatilityTracker;

let mut tracker = VolatilityTracker::<8>::new()?;
tracker.on_price(100_000)?;
tracker.on_price(101_000)?;
tracker.on_price(100_500)?;
let snapshot = tracker.snapshot();

assert_eq!(snapshot.samples(), 2);
assert!(snapshot.realized_vol_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Regime Classification

`RegimeClassifier` maps spread, volatility, toxicity, and imbalance inputs into
a compact `RegimeSnapshot`. The default classifier prioritizes toxic flow, then
illiquidity, then volatility.

```rust
use of_analytics::{RegimeClassifier, RegimeInput, RegimeKind};

let regime = RegimeClassifier::default().classify(RegimeInput::new(1, 10, 8_000, 0));

assert_eq!(regime.kind(), RegimeKind::Toxic);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Boundary

`of_analytics` is not a migration break:

- existing `of_core` APIs remain valid;
- existing runtime/binding analytics APIs remain valid;
- advanced modules can be wired into runtime and bindings additively later;
- hosts can choose feature profiles based on deployment needs.

This crate is the implementation home for heavier analytics, not a forced
replacement for existing users.
