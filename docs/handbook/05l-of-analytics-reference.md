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
- feed-quality analytics,
- liquidity resiliency analytics,
- feature profiles for future impact, toxicity, volatility, regime,
  data-quality, feature-vector, resiliency, pattern, derivatives,
  institutional, and ML-feature modules.

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
- `data-quality`
- `feature-vector`
- `resiliency`
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

Feed quality:

- `FeedQualityFlags`
- `FeedQualityConfig`
- `FeedQualityEvent`
- `FeedQualitySnapshot`
- `FeedQualityTracker`

Feature vectors:

- `FeatureId`
- `FeatureUnit`
- `FeatureQuality`
- `MissingValuePolicy`
- `FeatureDefinition`
- `FeatureSchema`
- `FeatureRegistry`
- `FeatureVector`
- `FeatureVectorWriter`
- `FeatureExtractor`

Resiliency:

- `ResiliencySample`
- `ResiliencyConfig`
- `ResiliencySnapshot`
- `ResiliencyTracker`

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

## Feed Quality

`FeedQualityTracker` is an allocation-free counter and flag accumulator for
market-data degradation. It does not correct, reorder, or discard market data.
The host records normalized observations and the tracker reports whether the
stream should be trusted, degraded, or investigated.

The tracker reports:

- sequence gap events and missing sequence units,
- out-of-order sequence or event-time movement,
- consecutive duplicate sequences,
- stale events based on receive minus event timestamp,
- locked and crossed top-of-book observations,
- timestamp skew between event and receive time,
- sequence-reset-like movement,
- cumulative degradation flags,
- event-rate metrics in basis points,
- aggregate health score where 10,000 is best.

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
let snapshot = tracker.snapshot();

assert!(flags.contains(FeedQualityFlags::SEQUENCE_GAP));
assert!(flags.contains(FeedQualityFlags::LOCKED_BOOK));
assert_eq!(snapshot.sequence_gap_events(), 1);
assert!(snapshot.health_score_bps() < 10_000);
# Ok::<(), Box<dyn std::error::Error>>(())
```

This follows the production convention of preserving anomalous records with
quality flags. Replay, investigation, and venue support workflows can then use
the original sequence numbers and timestamps instead of relying on an opaque
cleaned stream.

## Feature Vectors

`FeatureSchema` and `FeatureVectorWriter` provide a stable bridge between
offline research and live extraction. A schema owns feature ordering, ids,
names, units, scale, and missing-value policy. The writer reuses fixed arrays,
fills missing defaults from the schema, and returns a `FeatureVector` carrying
the schema hash used to produce it.

The intended compatibility model is append-only:

- keep existing feature ids and indices stable,
- append new features at the end of a schema,
- compare `schema_hash()` before replay/live model use,
- treat missing values explicitly through `MissingValuePolicy`,
- mark each value with `FeatureQuality`.

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

## Resiliency

`ResiliencyTracker` observes spread/depth samples and measures whether the book
has recovered after a liquidity shock. It uses explicit thresholds, which keeps
the live path deterministic and cheap. Batch calibration can provide better
thresholds later without changing the tracker API.

The snapshot reports:

- sample count,
- shock and recovery counts,
- active-shock state,
- latest shock and recovery timestamps,
- latest recovery duration,
- maximum spread during the active or latest shock,
- minimum depth during the active or latest shock,
- resiliency score in basis points.

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

## Boundary

`of_analytics` is not a migration break:

- existing `of_core` APIs remain valid;
- existing runtime/binding analytics APIs remain valid;
- advanced modules can be wired into runtime and bindings additively later;
- hosts can choose feature profiles based on deployment needs.

This crate is the implementation home for heavier analytics, not a forced
replacement for existing users.
