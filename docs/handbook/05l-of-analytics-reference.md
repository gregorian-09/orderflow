# `of_analytics` Reference

`of_analytics` is the additive advanced analytics crate for Orderflow. It
keeps heavier microstructure modules out of `of_core` so users who only need
canonical market-data types and the basic accumulator do not pay compile-time
or dependency cost for every advanced model.

The first public slice is dependency-light and live-path friendly:

- market-quality/TCA analytics,
- execution-quality/TCA analytics,
- liquidity/depth analytics,
- market-impact analytics,
- VPIN-style toxicity analytics,
- fixed-window volatility/noise analytics,
- threshold-based regime classification,
- feed-quality analytics,
- liquidity resiliency analytics,
- queue/fill probability analytics,
- pattern-risk analytics,
- venue/route analytics,
- cross-asset analytics,
- derivatives analytics,
- feature profiles for future impact, toxicity, volatility, regime,
  data-quality, feature-vector, resiliency, queue-fill, cross-asset, pattern,
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

## Public Types

Market quality:

- `AnalyticsError`
- `QuoteContext`
- `TradeContext`
- `MarketQualitySnapshot`
- `MarketQualityTracker`
- `ExecutionBenchmark`
- `ExecutionQualitySnapshot`
- `ExecutionQualityAnalyzer`

Liquidity/depth:

- `LiquidityDepthSnapshot`
- `LiquidityDepthAnalyzer`
- `LiquidityFlowEvent`
- `LiquidityFlowConfig`
- `LiquidityFlowSnapshot`
- `LiquidityFlowTracker`

Impact/toxicity:

- `ImpactSample`
- `ImpactSnapshot`
- `ImpactTracker`
- `ImpactCalibration`
- `ExpectedImpactInput`
- `ExpectedImpactSnapshot`
- `ExpectedImpactEstimator`
- `ChildOrderImpactContext`
- `ChildOrderImpactSnapshot`
- `ChildOrderImpactAnalyzer`
- `VpinSnapshot`
- `VpinTracker`
- `ToxicityConfig`
- `ToxicityInput`
- `ToxicitySnapshot`
- `ToxicityAnalyzer`
- `VolatilitySnapshot`
- `VolatilityTracker`
- `OhlcVolatilityInput`
- `OhlcVolatilitySnapshot`
- `OhlcVolatilityEstimator`
- `VolatilitySignatureSnapshot`
- `VolatilitySignatureEstimator`
- `VolatilitySeasonalitySnapshot`
- `VolatilitySeasonalityTracker`
- `RegimeKind`
- `RegimeInput`
- `RegimeSnapshot`
- `RegimeClassifier`
- `TrendRegimeKind`
- `LiquidityRegimeKind`
- `SpreadRegimeKind`
- `SessionRegimeKind`
- `CompositeRegimeConfig`
- `CompositeRegimeInput`
- `CompositeRegimeSnapshot`
- `CompositeRegimeClassifier`

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

Queue and fill:

- `QueueUpdateKind`
- `QueuePositionEstimate`
- `QueueFillConfig`
- `QueueFillUpdate`
- `QueueFillSnapshot`
- `QueueFillTracker`
- `QueueDecisionConfig`
- `QueueDecisionInput`
- `QueueDecisionSnapshot`
- `QueueDecisionAnalyzer`

Pattern risk:

- `PatternRiskInput`
- `PatternRiskLiquidity`
- `PatternRiskConfig`
- `PatternRiskSnapshot`
- `PatternRiskClassifier`
- `PatternDetailConfig`
- `PatternDetailInput`
- `PatternDetailSnapshot`
- `PatternDetailAnalyzer`

Venue and route:

- `VenueRouteEventKind`
- `VenueRouteEvent`
- `VenueRouteSnapshot`
- `VenueRouteTracker`
- `VenueRouteQualityConfig`
- `VenueRouteQualityInput`
- `VenueRouteQualitySnapshot`
- `VenueRouteQualityAnalyzer`

Cross asset:

- `CrossAssetSample`
- `CrossAssetConfig`
- `CrossAssetSnapshot`
- `CrossAssetTracker`
- `CrossAssetDiagnosticConfig`
- `CrossAssetDiagnosticInput`
- `CrossAssetDiagnosticSnapshot`
- `CrossAssetDiagnosticAnalyzer`

Derivatives:

- `OptionKind`
- `OptionFlowSample`
- `OptionFlowSnapshot`
- `OptionFlowTracker`
- `FuturesBasisInput`
- `FuturesBasisSnapshot`
- `FuturesBasisAnalyzer`
- `DerivativesVolatilitySurface`
- `DerivativesDiagnosticConfig`
- `DerivativesDiagnosticInput`
- `DerivativesDiagnosticSnapshot`
- `DerivativesDiagnosticAnalyzer`

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

## Execution Quality

`ExecutionQualityAnalyzer` evaluates one fill against arrival, decision,
same-side touch, and optional future-midpoint benchmarks. It reports
implementation shortfall, arrival/decision slippage, adverse selection,
trade-through, and a bounded fill-quality score.

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
- depth convexity proxy,
- distance-weighted book pressure,
- quantity sweepable by a buy order up to a target quantity,
- quantity sweepable by a sell order up to a target quantity,
- buy, sell, and conservative sweepability scores.

`LiquidityFlowTracker` consumes explicit book-flow events. It is intentionally
separate from `LiquidityDepthAnalyzer` because not every provider exposes enough
book-delta, market-order, and cancellation detail to estimate order-flow
imbalance correctly. When those events are available, the tracker reports:

- bid and ask replenishment,
- bid and ask depletion,
- bid and ask traded quantity,
- signed order-flow imbalance,
- replenishment and depletion rates,
- liquidity-drought flag.

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
assert_eq!(snapshot.sweepable_buy_qty(), 150);
assert!(snapshot.book_pressure_bps() < 0);

let mut flow = LiquidityFlowTracker::new(LiquidityFlowConfig::default());
flow.on_event(LiquidityFlowEvent::new(Side::Bid, 100, 0, 0, 1)?);
flow.on_event(LiquidityFlowEvent::new(Side::Ask, 0, 250, 25, 500_000_001)?);
assert!(flow.snapshot().order_flow_imbalance_bps() > 0);
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

`ExpectedImpactEstimator` evaluates a proposed order from explicit
`ImpactCalibration`. It reports interval participation, daily participation,
square-root impact, temporary impact, permanent impact, instantaneous impact,
impact decay, expected total impact, and expected signed midpoint movement.

`ChildOrderImpactAnalyzer` evaluates one child order against arrival, immediate
post-child midpoint, and final midpoint. It reports child participation,
realized slippage, instantaneous impact, permanent impact, temporary component,
impact decay, and parent-weighted attribution.

These types do not run regressions or allocate rolling matrices. Calibration is
host-owned so users can source parameters from replay research, venue models,
or risk configuration without changing the live API.

```rust
use of_analytics::{
    ChildOrderImpactAnalyzer, ChildOrderImpactContext, ExpectedImpactEstimator,
    ExpectedImpactInput, ImpactCalibration, ImpactSample, ImpactTracker,
};
use of_core::Side;

let mut tracker = ImpactTracker::new();
tracker.on_sample(ImpactSample::new(500_000, 501_000, 100, 50_000_000)?);
let snapshot = tracker.snapshot();

assert_eq!(snapshot.samples(), 1);
assert!(snapshot.kyle_lambda_ppm() > 0);

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

let child = ChildOrderImpactContext::new(
    Side::Ask,
    1_000,
    100,
    100_000,
    100_500,
    100_800,
    100_200,
)?;
assert_eq!(
    ChildOrderImpactAnalyzer::evaluate(child).child_participation_bps(),
    1_000
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## VPIN-Style Toxicity

`VpinTracker` is a fixed-capacity bucket tracker. `Side::Ask` is interpreted as
buyer-initiated flow and `Side::Bid` as seller-initiated flow. Completed bucket
imbalances are retained in a const-generic ring buffer.

`ToxicityAnalyzer` complements VPIN with explicit post-trade diagnostics:
side-aware markout, adverse-selection score, same-side quote fade, informed-flow
proxy, toxic-flow burst flag, and aggregate toxicity score. The output is a
risk indicator for routing, quoting, and operator controls; it is not a
regulatory conclusion.

```rust
use of_analytics::{ToxicityAnalyzer, ToxicityConfig, ToxicityInput, TradeContext, VpinTracker};
use of_core::Side;

let mut tracker = VpinTracker::<4>::new(100)?;
tracker.on_trade(TradeContext::new(500_000, 80, Side::Ask, 1)?);
tracker.on_trade(TradeContext::new(500_000, 20, Side::Bid, 2)?);
let snapshot = tracker.snapshot();

assert_eq!(snapshot.bucket_count(), 1);
assert_eq!(snapshot.vpin_bps(), 6_000);

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

## Volatility And Noise

`VolatilityTracker` stores a fixed-size ring of integer return samples. It
reports realized volatility, mean absolute return, bipower variation
volatility, jump variation, and a simple noise proxy based on return sign
flips.

`OhlcVolatilityEstimator` provides deterministic OHLC range estimators:
close-to-close, Parkinson, Garman-Klass, Rogers-Satchell, and signed open-gap
jump. The formulas use integer-scaled return proxies, not floating-point logs,
so the live path stays deterministic.

`VolatilitySignatureEstimator` computes one signature-plot point over borrowed
returns. `VolatilitySeasonalityTracker` keeps fixed intraday buckets for
realized volatility, mean absolute return, and jump counts.

```rust
use of_analytics::{
    OhlcVolatilityEstimator, OhlcVolatilityInput, VolatilitySeasonalityTracker,
    VolatilitySignatureEstimator, VolatilityTracker,
};

let mut tracker = VolatilityTracker::<8>::new()?;
tracker.on_price(100_000)?;
tracker.on_price(101_000)?;
tracker.on_price(100_500)?;
let snapshot = tracker.snapshot();

assert_eq!(snapshot.samples(), 2);
assert!(snapshot.realized_vol_bps() > 0);
assert!(snapshot.bipower_vol_bps() > 0);

let ohlc = OhlcVolatilityEstimator::estimate(OhlcVolatilityInput::new(
    100_000,
    102_000,
    99_000,
    101_000,
    Some(99_500),
)?);
assert!(ohlc.garman_klass_vol_bps() > 0);

let signature = VolatilitySignatureEstimator::estimate(1_000_000, &[10, -10, 20, -20])?;
assert_eq!(signature.noise_ratio_bps(), 10_000);

let mut seasonality = VolatilitySeasonalityTracker::<2>::new(25)?;
seasonality.on_return(0, 10)?;
seasonality.on_return(0, -30)?;
assert_eq!(seasonality.snapshot(0)?.jump_count(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Regime Classification

`RegimeClassifier` maps spread, volatility, toxicity, and imbalance inputs into
a compact `RegimeSnapshot`. The default classifier prioritizes toxic flow, then
illiquidity, then volatility.

`CompositeRegimeClassifier` is the richer rule-based classifier. It emits
separate trend/range/chop, liquidity, spread, and session labels, plus a
volatility flag, hidden-liquidity proxy, and transition confidence. It is
additive and does not change the existing `RegimeClassifier` behavior.

```rust
use of_analytics::{
    CompositeRegimeClassifier, CompositeRegimeInput, RegimeClassifier, RegimeInput,
    RegimeKind, SessionRegimeKind, TrendRegimeKind,
};

let regime = RegimeClassifier::default().classify(RegimeInput::new(1, 10, 8_000, 0));

assert_eq!(regime.kind(), RegimeKind::Toxic);

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

## Queue And Fill Probability

`QueueFillTracker` estimates passive order progress using aggregate quantity
ahead, own remaining quantity, queue updates, and explicit assumptions about
where cancels occur. It is designed for venues where full order-by-order queue
priority may not be available.

The snapshot reports:

- estimated quantity ahead,
- own quantity remaining,
- total queue quantity,
- fill probability over the configured horizon,
- expected time-to-fill,
- estimated queue loss after amend,
- maker/taker score,
- top-level survival proxy,
- latest update timestamp.

`QueueDecisionAnalyzer` evaluates the queue snapshot against explicit economics:
spread, passive price improvement, maker rebate, taker fee, adverse-selection
cost, urgency, replacement price improvement, and replacement queue loss. It
reports passive edge, aggressive cost, wait penalty, priority loss,
cancel/replace cost, maker/taker decision score, and passive/replace
preferences.

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

## Pattern Risk

`PatternRiskClassifier` maps bounded order-book activity summaries into risk
indicator scores. These scores are diagnostics for operators and strategies;
they are not accusations or regulatory conclusions.

The snapshot reports:

- spoofing/layering risk,
- quote-stuffing risk,
- stop-run/liquidity-sweep risk,
- absorption risk,
- momentum-ignition risk,
- overall maximum component risk.

`PatternDetailAnalyzer` adds focused diagnostics for iceberg/hidden-refresh
risk, hidden accumulation/distribution, stacked imbalance, absorption strength,
and failed breakouts. These are still risk indicators, not claims about intent.

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

## Venue And Route Analytics

`VenueRouteTracker` records route lifecycle events and latency samples for one
caller-defined venue, route, account, strategy, symbol, or other key. The crate
does not own those identifiers, which keeps this analytics layer reusable
across OMS implementations.

The route snapshot reports:

- sent, fill, reject, and cancel counts,
- sent and filled quantity,
- fill, reject, and cancel rates,
- average and max quote-to-fill latency,
- average and max market-data-to-order latency,
- route health score.

`VenueRouteQualityAnalyzer` adds a second deterministic scoring layer for
route-selection and route-monitoring loops. It combines the lifecycle snapshot
with caller-owned venue liquidity, venue toxicity, venue fill-quality, and
baseline route health scores. The resulting quality snapshot reports combined
latency quality, reliability, aggregate route quality, route-health drift, and a
degraded flag. This keeps exchange-specific identifiers, fee models, and venue
classification outside the crate while still giving execution systems a stable
low-allocation diagnostic contract.

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

## Cross-Asset And Lead-Lag

`CrossAssetTracker` records paired leader/follower samples and keeps a
fixed-size ring of integer returns. It reports rolling correlation, beta,
latest pair divergence, latest basis pressure, thresholded divergence and
basis-pressure flags, a correlation-breakdown flag, and a lead/lag strength
score.

These values are diagnostics, not proof that a relationship is stable. Rolling
cross-asset relationships can change quickly, so production users should
calibrate windows and thresholds out of sample before using them in live risk
or routing decisions.

`CrossAssetDiagnosticAnalyzer` adds an opt-in second-stage diagnostic for
production routing and strategy monitoring. It combines the tracker snapshot
with caller-owned event timestamps, cross-venue divergence, and ETF/component
imbalance. The output includes sample skew, synchronization quality,
latency-adjusted correlation, aggregate divergence pressure, cross-venue and
component flags, and a relationship-degraded flag.

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

## Derivatives

`OptionFlowTracker` accumulates caller-normalized option flow and reports
put/call pressure, volume/open-interest anomaly, premium-weighted implied
volatility flow, and net gamma exposure. The crate does not price options or
derive Greeks; hosts pass provider- or model-supplied implied volatility and
gamma exposure into the tracker.

`FuturesBasisAnalyzer` computes futures-minus-spot basis, fair-value gap,
calendar spread, roll-pressure proxy, and funding/basis divergence from one
caller-supplied input snapshot.

`DerivativesDiagnosticAnalyzer` adds an opt-in second-stage diagnostic over
option flow, a caller-supplied volatility surface summary, and futures basis.
It reports IV skew, volatility term structure, implied-versus-realized
richness, gamma pressure, option risk, futures stress, aggregate derivatives
stress, and explicit stress flags. The crate still does not price options,
calibrate volatility surfaces, or derive Greeks; production hosts retain those
model and data-vendor responsibilities.

```rust
use of_analytics::{
    DerivativesDiagnosticAnalyzer, DerivativesDiagnosticInput,
    DerivativesVolatilitySurface, FuturesBasisAnalyzer, FuturesBasisInput,
    OptionFlowSample, OptionFlowTracker, OptionKind,
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
let diagnostic = DerivativesDiagnosticAnalyzer::default().evaluate(
    DerivativesDiagnosticInput::new(
        options.snapshot(),
        basis,
        DerivativesVolatilitySurface::new(3_000, 3_500, 2_500, 3_000, 3_250, 2_000)?,
    ),
);
assert!(basis.basis_bps() > 0);
assert!(diagnostic.iv_richness_bps() > 0);
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
