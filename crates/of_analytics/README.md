# `of_analytics`

[![Crates.io](https://img.shields.io/crates/v/of_analytics.svg)](https://crates.io/crates/of_analytics)
[![Docs.rs](https://docs.rs/of_analytics/badge.svg)](https://docs.rs/of_analytics)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

`of_analytics` contains additive advanced market microstructure analytics for
Orderflow. The crate exists so heavier analytics can evolve outside `of_core`
without breaking existing `AnalyticsAccumulator`, runtime, C ABI, Python, or
Java APIs.

`of_analytics` starts at `0.1.0` in the broader Orderflow `0.4.0` development
line because it is a new public Rust surface.

The first foundation is dependency-light:

- market-quality/TCA primitives for quoted spread, effective spread, realized
  spread, price improvement, quote freshness, and side-aware slippage,
- liquidity/depth primitives for top-of-book depth, multi-level depth,
  proportional imbalance, depth slope, and sweepability,
- market-impact primitives for Kyle-style lambda and Amihud-style
  illiquidity,
- VPIN-style fixed-bucket toxicity primitives,
- fixed-window volatility/noise primitives,
- threshold-based market regime classification,
- feed-quality primitives for sequence gaps, out-of-order events, duplicates,
  stale events, locked/crossed books, timestamp skew, resets, and health
  scoring,
- explicit feature profiles so users can opt into future impact, toxicity,
  volatility, regime, data-quality, pattern, derivatives, institutional, and
  ML feature modules without forcing all downstream users to compile them.

The crate does not submit orders, manage runtime state, own persistence, or
replace existing `of_core` APIs. It consumes normalized market data and returns
typed snapshots that hosts can wire into runtime, research, or execution
systems.

## Feature Profiles

Default features:

- `market-quality`
- `liquidity`

Reserved additive profiles:

- `impact`
- `toxicity`
- `volatility`
- `regime`
- `data-quality`
- `patterns`
- `derivatives`
- `institutional`
- `ml-features`
- `all`

## Market Quality Example

```rust
use of_analytics::{MarketQualityTracker, QuoteContext, TradeContext};
use of_core::Side;

let mut tracker = MarketQualityTracker::new(1_000_000);
tracker.on_quote(QuoteContext::new(499_975, 500_025, 100, 120, 1_000)?);
let snapshot = tracker.evaluate_trade(
    TradeContext::new(500_050, 10, Side::Ask, 1_500)?,
    Some(500_075),
)?;

assert_eq!(snapshot.quoted_spread(), 50);
assert!(snapshot.effective_spread_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Liquidity Example

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
assert!(snapshot.sweepable_buy_qty() >= 120);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Impact And Toxicity Example

```rust
use of_analytics::{ImpactSample, ImpactTracker, TradeContext, VpinTracker};
use of_core::Side;

let mut impact = ImpactTracker::new();
impact.on_sample(ImpactSample::new(500_000, 501_000, 100, 50_000_000)?);
assert!(impact.snapshot().kyle_lambda_ppm() > 0);

let mut vpin = VpinTracker::<4>::new(100)?;
vpin.on_trade(TradeContext::new(500_000, 80, Side::Ask, 1)?);
vpin.on_trade(TradeContext::new(500_000, 20, Side::Bid, 2)?);
assert_eq!(vpin.snapshot().bucket_count(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Volatility And Regime Example

```rust
use of_analytics::{RegimeClassifier, RegimeInput, RegimeKind, VolatilityTracker};

let mut vol = VolatilityTracker::<8>::new()?;
vol.on_price(100_000)?;
vol.on_price(101_000)?;
vol.on_price(100_500)?;
let snapshot = vol.snapshot();

let regime = RegimeClassifier::default().classify(RegimeInput::new(
    5,
    snapshot.realized_vol_bps(),
    0,
    0,
));

assert!(matches!(regime.kind(), RegimeKind::Normal | RegimeKind::Volatile));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Data Quality Example

```rust
use of_analytics::{
    FeedQualityConfig, FeedQualityEvent, FeedQualityFlags, FeedQualityTracker,
};

let config = FeedQualityConfig::new(10, 20, 1)?;
let mut tracker = FeedQualityTracker::new(config);

tracker.on_event(FeedQualityEvent::new(
    Some(10),
    100,
    105,
    Some(99),
    Some(101),
)?);
let flags = tracker.on_event(FeedQualityEvent::new(
    Some(12),
    110,
    140,
    Some(100),
    Some(100),
)?);

assert!(flags.contains(FeedQualityFlags::SEQUENCE_GAP));
assert!(flags.contains(FeedQualityFlags::LOCKED_BOOK));
assert!(tracker.snapshot().health_score_bps() < 10_000);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Low-Latency Principles

- No async runtime dependency.
- No heap allocation in hot update/evaluate methods.
- Borrow existing `of_core` book levels instead of copying snapshots.
- Use integer arithmetic for prices, quantities, spreads, and basis points.
- Keep batch/research features separate from live hot-path trackers.
