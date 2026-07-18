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
- feature-vector primitives for stable feature ids, ordered schemas, schema
  hashes, missing-value policy, quality labels, and reusable fixed-capacity
  writers,
- resiliency primitives for threshold-based spread/depth shock detection,
  recovery timing, and liquidity resiliency scoring,
- queue/fill primitives for passive queue position estimates, fill
  probability, expected time-to-fill, amend queue loss, and maker/taker scoring,
- pattern-risk primitives for spoofing/layering, quote-stuffing, stop-run,
  absorption, and momentum-ignition risk indicators,
- venue/route primitives for fill, reject, cancel, latency, and route-health
  diagnostics,
- cross-asset primitives for rolling correlation, beta, pair divergence,
  thresholded basis pressure, and correlation-breakdown diagnostics,
- explicit feature profiles so users can opt into future impact, toxicity,
  volatility, regime, data-quality, feature-vector, resiliency, queue-fill,
  cross-asset, pattern, derivatives, institutional, and ML feature modules without forcing all
  downstream users to compile them.

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
- `feature-vector`
- `resiliency`
- `queue-fill`
- `cross-asset`
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

## Feature Vector Example

```rust
use of_analytics::{
    FeatureDefinition, FeatureId, FeatureQuality, FeatureSchema, FeatureUnit,
    FeatureVectorWriter, MissingValuePolicy,
};

let mut schema = FeatureSchema::<2>::new()?;
let spread = schema.register(FeatureDefinition::new(
    FeatureId::new(1)?,
    "spread_bps",
    FeatureUnit::BasisPoints,
    1,
    MissingValuePolicy::Sentinel(-1),
)?)?;
schema.register(FeatureDefinition::new(
    FeatureId::new(2)?,
    "quality_bps",
    FeatureUnit::ScoreBasisPoints,
    1,
    MissingValuePolicy::Zero,
)?)?;

let mut writer = FeatureVectorWriter::new(&schema);
writer.set(spread, 25, FeatureQuality::Good)?;
let vector = writer.finish();

assert_eq!(vector.values(), &[25, 0]);
assert_eq!(vector.schema_hash(), schema.schema_hash());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Resiliency Example

```rust
use of_analytics::{ResiliencyConfig, ResiliencySample, ResiliencyTracker};

let config = ResiliencyConfig::new(5, 1_000, 25, 5_000, 8, 9_000)?;
let mut tracker = ResiliencyTracker::new(config);

tracker.on_sample(ResiliencySample::new(100, 5, 500, 500)?);
let shock = tracker.on_sample(ResiliencySample::new(200, 30, 400, 400)?);
let recovered = tracker.on_sample(ResiliencySample::new(1_000_200, 7, 500, 500)?);

assert!(shock.active_shock());
assert_eq!(recovered.recovery_count(), 1);
assert_eq!(recovered.last_recovery_time_ns(), 1_000_000);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Queue And Fill Example

```rust
use of_analytics::{
    QueueFillConfig, QueueFillTracker, QueueFillUpdate, QueuePositionEstimate,
    QueueUpdateKind,
};

let config = QueueFillConfig::new(5_000, 100, 1_000_000_000)?;
let estimate = QueuePositionEstimate::new(100, 50, 200, 1)?;
let mut tracker = QueueFillTracker::new(config, estimate);

let snapshot = tracker.on_update(QueueFillUpdate::new(
    QueueUpdateKind::Trade,
    40,
    160,
    2,
)?);

assert_eq!(snapshot.qty_ahead(), 60);
assert!(snapshot.fill_probability_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Pattern Risk Example

```rust
use of_analytics::{PatternRiskClassifier, PatternRiskInput, PatternRiskLiquidity};

let classifier = PatternRiskClassifier::default();
let snapshot = classifier.classify(PatternRiskInput::new(
    800,
    900,
    10,
    8_000,
    5,
    PatternRiskLiquidity::new(10, 1_000)?,
    1_000_000,
)?);

assert!(snapshot.spoofing_layering_risk_bps() > 0);
assert!(snapshot.quote_stuffing_risk_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Venue Route Example

```rust
use of_analytics::{VenueRouteEvent, VenueRouteEventKind, VenueRouteTracker};

let mut tracker = VenueRouteTracker::new();
tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Sent, 100, 0, 10)?);
tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Fill, 60, 100, 20)?);
let snapshot = tracker.snapshot();

assert_eq!(snapshot.sent(), 1);
assert_eq!(snapshot.fills(), 1);
assert_eq!(snapshot.avg_quote_to_fill_latency_ns(), 100);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Cross-Asset Example

```rust
use of_analytics::{CrossAssetConfig, CrossAssetSample, CrossAssetTracker};

let mut tracker = CrossAssetTracker::<8>::new(CrossAssetConfig::default())?;
tracker.on_sample(CrossAssetSample::new(100_000, 200_000, 1)?);
tracker.on_sample(CrossAssetSample::new(101_000, 202_000, 2)?);
tracker.on_sample(CrossAssetSample::new(102_000, 204_000, 3)?);
let snapshot = tracker.snapshot();

assert!(snapshot.correlation_bps() > 0);
assert!(snapshot.lead_lag_score_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Low-Latency Principles

- No async runtime dependency.
- No heap allocation in hot update/evaluate methods.
- Borrow existing `of_core` book levels instead of copying snapshots.
- Use integer arithmetic for prices, quantities, spreads, and basis points.
- Keep batch/research features separate from live hot-path trackers.
