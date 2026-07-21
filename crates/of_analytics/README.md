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
- execution-quality/TCA primitives for implementation shortfall, arrival and
  decision slippage, adverse selection, trade-through, and fill-quality score,
- liquidity/depth primitives for top-of-book depth, multi-level depth,
  proportional imbalance, order-flow imbalance, depth slope, depth convexity,
  book pressure, replenishment/depletion rates, drought detection, and
  sweepability,
- market-impact primitives for Kyle-style lambda, Amihud-style illiquidity,
  calibrated expected impact, square-root impact, temporary/permanent impact,
  impact decay, and child-order attribution,
- VPIN-style fixed-bucket toxicity primitives plus post-trade markout,
  adverse-selection, quote-fade, toxic-burst, and informed-flow proxy signals,
- fixed-window volatility/noise primitives with bipower variation, jump
  variation, OHLC range estimators, signature-plot points, and intraday
  seasonality buckets,
- threshold-based market regime classification plus composite trend/range/chop,
  liquidity, spread, session, hidden-liquidity, and transition-confidence
  labels,
- feed-quality primitives for sequence gaps, out-of-order events, duplicates,
  stale events, locked/crossed books, timestamp skew, resets, and health
  scoring,
- feature-vector primitives for stable feature ids, ordered schemas, schema
  hashes, missing-value policy, quality labels, and reusable fixed-capacity
  writers,
- resiliency primitives for threshold-based spread/depth shock detection,
  recovery timing, and liquidity resiliency scoring,
- queue/fill primitives for passive queue position estimates, fill
  probability, expected time-to-fill, amend queue loss, top-level survival,
  cancel/replace cost, and maker/taker decision scoring,
- pattern-risk primitives for spoofing/layering, quote-stuffing, stop-run,
  absorption, momentum-ignition, iceberg, hidden accumulation/distribution,
  stacked-imbalance, and failed-breakout risk indicators,
- venue/route primitives for fill, reject, cancel, latency, route-health,
  venue-liquidity, toxicity, fill-quality, reliability, route-quality, drift,
  and degradation diagnostics,
- cross-asset primitives for rolling correlation, beta, pair divergence,
  thresholded basis pressure, latency-adjusted correlation, cross-venue
  divergence, ETF/component imbalance, and relationship-degradation
  diagnostics,
- derivatives primitives for put/call pressure, volume/open-interest anomaly,
  implied-volatility flow, gamma exposure, futures basis, roll pressure, and
  funding divergence,
- explicit feature profiles so users can opt into future impact, toxicity,
  volatility, regime, data-quality, feature-vector, resiliency, queue-fill,
  cross-asset, pattern, derivatives, institutional, and ML feature modules
  without forcing all downstream users to compile them.

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

## Execution Quality Example

```rust
use of_analytics::{ExecutionBenchmark, ExecutionQualityAnalyzer, TradeContext};
use of_core::Side;

let trade = TradeContext::new(101_000, 10, Side::Ask, 1)?;
let benchmark = ExecutionBenchmark::new(
    100_000,
    100_500,
    99_950,
    100_050,
    Some(100_750),
)?;
let snapshot = ExecutionQualityAnalyzer::evaluate(trade, benchmark);

assert!(snapshot.implementation_shortfall_bps() > 0);
assert!(snapshot.trade_through());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Liquidity Example

```rust
use of_analytics::{
    LiquidityDepthAnalyzer, LiquidityFlowConfig, LiquidityFlowEvent,
    LiquidityFlowTracker,
};
use of_core::BookLevel;
use of_core::Side;

let bids = [
    BookLevel { level: 0, price: 499_975, size: 100 },
    BookLevel { level: 1, price: 499_950, size: 80 },
    BookLevel { level: 2, price: 499_925, size: 70 },
];
let asks = [
    BookLevel { level: 0, price: 500_025, size: 120 },
    BookLevel { level: 1, price: 500_050, size: 90 },
    BookLevel { level: 2, price: 500_075, size: 60 },
];
let snapshot = LiquidityDepthAnalyzer::new(3).analyze(&bids, &asks, 150)?;

assert_eq!(snapshot.bid_depth(), 250);
assert_eq!(snapshot.ask_depth(), 270);
assert!(snapshot.sweepable_buy_qty() >= 120);
assert!(snapshot.book_pressure_bps() < 0);

let mut flow = LiquidityFlowTracker::new(LiquidityFlowConfig::default());
flow.on_event(LiquidityFlowEvent::new(Side::Bid, 100, 0, 0, 1)?);
flow.on_event(LiquidityFlowEvent::new(Side::Ask, 0, 250, 25, 500_000_001)?);
assert!(flow.snapshot().order_flow_imbalance_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Impact And Toxicity Example

```rust
use of_analytics::{
    ExpectedImpactEstimator, ExpectedImpactInput, ImpactCalibration,
    ImpactSample, ImpactTracker, ToxicityAnalyzer, ToxicityConfig,
    ToxicityInput, TradeContext, VpinTracker,
};
use of_core::Side;

let mut impact = ImpactTracker::new();
impact.on_sample(ImpactSample::new(500_000, 501_000, 100, 50_000_000)?);
assert!(impact.snapshot().kyle_lambda_ppm() > 0);

let calibration = ImpactCalibration::new(
    1_000_000,
    200,
    10_000,
    500,
    250,
    1_000_000_000,
)?;
let estimate = ExpectedImpactEstimator::estimate(ExpectedImpactInput::new(
    Side::Ask,
    1_000,
    10_000,
    100_000,
    1_000_000_000,
    calibration,
)?);
assert!(estimate.expected_total_impact_bps() > 0);

let mut vpin = VpinTracker::<4>::new(100)?;
vpin.on_trade(TradeContext::new(500_000, 80, Side::Ask, 1)?);
vpin.on_trade(TradeContext::new(500_000, 20, Side::Bid, 2)?);
assert_eq!(vpin.snapshot().bucket_count(), 1);

let toxicity = ToxicityAnalyzer::new(ToxicityConfig::default()).evaluate(
    ToxicityInput::new(
        TradeContext::new(100_000, 10, Side::Ask, 1)?,
        100_500,
        1_000,
        1_000,
        1_000,
        500,
        8_000,
        7_000,
    )?,
);
assert!(toxicity.toxic_flow_burst());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Volatility And Regime Example

```rust
use of_analytics::{
    CompositeRegimeClassifier, CompositeRegimeInput, OhlcVolatilityEstimator,
    OhlcVolatilityInput, RegimeClassifier, RegimeInput, RegimeKind,
    SessionRegimeKind, TrendRegimeKind, VolatilitySeasonalityTracker,
    VolatilitySignatureEstimator, VolatilityTracker,
};

let mut vol = VolatilityTracker::<8>::new()?;
vol.on_price(100_000)?;
vol.on_price(101_000)?;
vol.on_price(100_500)?;
let snapshot = vol.snapshot();
assert!(snapshot.bipower_vol_bps() > 0);

let regime = RegimeClassifier::default().classify(RegimeInput::new(
    5,
    snapshot.realized_vol_bps(),
    0,
    0,
));

assert!(matches!(regime.kind(), RegimeKind::Normal | RegimeKind::Volatile));

let composite = CompositeRegimeClassifier::default().classify(
    CompositeRegimeInput::new(
        8_000,
        1_000,
        1,
        20,
        2_000,
        3_600_000_000_000,
        3_600_000_000_000,
        false,
        0,
        0,
    )?,
);
assert_eq!(composite.trend(), TrendRegimeKind::Trend);
assert_eq!(composite.session(), SessionRegimeKind::Continuous);

let ohlc = OhlcVolatilityEstimator::estimate(OhlcVolatilityInput::new(
    100_000,
    102_000,
    99_000,
    101_000,
    Some(99_500),
)?);
assert!(ohlc.parkinson_vol_bps() > 0);

let signature = VolatilitySignatureEstimator::estimate(1_000_000, &[10, -10, 20, -20])?;
assert_eq!(signature.noise_ratio_bps(), 10_000);

let mut seasonality = VolatilitySeasonalityTracker::<2>::new(25)?;
seasonality.on_return(0, 10)?;
seasonality.on_return(0, -30)?;
assert_eq!(seasonality.snapshot(0)?.jump_count(), 1);
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
    QueueDecisionAnalyzer, QueueDecisionConfig, QueueDecisionInput,
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

let decision = QueueDecisionAnalyzer::new(
    QueueDecisionConfig::new(5_000, 5_000, 10_000_000_000)?,
)
.evaluate(QueueDecisionInput::new(
    snapshot,
    10,
    5,
    1,
    2,
    1,
    1_000,
    0,
    0,
)?);
assert!(decision.maker_taker_decision_score_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Pattern Risk Example

```rust
use of_analytics::{
    PatternDetailAnalyzer, PatternDetailInput, PatternRiskClassifier,
    PatternRiskInput, PatternRiskLiquidity,
};

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

let detail = PatternDetailAnalyzer::default().evaluate(
    PatternDetailInput::new(8, 4, 1_000, 100, 4, 8_000, 500, 2, 30, 35, 1_000)?,
);
assert!(detail.iceberg_risk_bps() > 0);
assert!(detail.failed_breakout_risk_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Venue Route Example

```rust
use of_analytics::{
    VenueRouteEvent, VenueRouteEventKind, VenueRouteQualityAnalyzer,
    VenueRouteQualityInput, VenueRouteTracker,
};

let mut tracker = VenueRouteTracker::new();
tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Sent, 100, 0, 10)?);
tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Fill, 60, 100, 20)?);
let snapshot = tracker.snapshot();
let quality = VenueRouteQualityAnalyzer::default().evaluate(
    VenueRouteQualityInput::new(snapshot, 9_000, 500, 9_500, 9_500)?,
);

assert_eq!(snapshot.sent(), 1);
assert_eq!(snapshot.fills(), 1);
assert_eq!(snapshot.avg_quote_to_fill_latency_ns(), 100);
assert!(!quality.route_degraded());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Cross-Asset Example

```rust
use of_analytics::{
    CrossAssetConfig, CrossAssetDiagnosticAnalyzer, CrossAssetDiagnosticInput,
    CrossAssetSample, CrossAssetTracker,
};

let mut tracker = CrossAssetTracker::<8>::new(CrossAssetConfig::default())?;
tracker.on_sample(CrossAssetSample::new(100_000, 200_000, 1)?);
tracker.on_sample(CrossAssetSample::new(101_000, 202_000, 2)?);
tracker.on_sample(CrossAssetSample::new(102_000, 204_000, 3)?);
let snapshot = tracker.snapshot();
let diagnostic = CrossAssetDiagnosticAnalyzer::default().evaluate(
    CrossAssetDiagnosticInput::new(snapshot, 1_000_000, 1_100_000, 5, 100)?,
);

assert!(snapshot.correlation_bps() > 0);
assert!(snapshot.lead_lag_score_bps() > 0);
assert!(diagnostic.synchronization_quality_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Derivatives Example

```rust
use of_analytics::{
    FuturesBasisAnalyzer, FuturesBasisInput, OptionFlowSample, OptionFlowTracker,
    OptionKind,
};

let mut options = OptionFlowTracker::new();
options.on_sample(OptionFlowSample::new(
    OptionKind::Call,
    100,
    1_000,
    50_000,
    2_000,
    1_000,
)?);
options.on_sample(OptionFlowSample::new(
    OptionKind::Put,
    200,
    1_500,
    150_000,
    3_000,
    -2_000,
)?);
assert!(options.snapshot().put_call_pressure_bps() > 0);

let basis = FuturesBasisAnalyzer::analyze(FuturesBasisInput::new(
    100_000,
    101_000,
    100_500,
    101_000,
    102_000,
    25,
)?);
assert!(basis.basis_bps() > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Low-Latency Principles

- No async runtime dependency.
- No heap allocation in hot update/evaluate methods.
- Borrow existing `of_core` book levels instead of copying snapshots.
- Use integer arithmetic for prices, quantities, spreads, and basis points.
- Keep batch/research features separate from live hot-path trackers.
