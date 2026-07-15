# of_signals

`of_signals` contains strategy modules that transform analytics snapshots into stable directional state.
It is intentionally separated from ingestion/runtime plumbing so strategy logic remains easy to test and evolve.

## Core API

- Trait: [`SignalModule`]
- Contextual trait: [`ContextualSignalModule`]
- Gate result: [`SignalGateDecision`]
- Descriptor inventory: [`built_in_signal_descriptors`] and [`describe_signal`]
- Registry/config: [`SignalRegistry`], [`SignalConfig`], and
  [`built_in_signal_descriptors_json`]
- Validation: [`SignalValidationHarness`], [`SignalValidationConfig`], and
  [`validate_signal_replay`]
- Lifecycle helper: [`SignalLifecycle`]
- Stabilizer: [`SignalStabilizer`]
- Explainability: [`ExplainableSignalModule`], [`SignalExplanation`], and
  [`SignalReasonCode`]
- Calibration/outcomes: [`SignalOutcomeTracker`], [`SignalCalibrationReport`],
  and [`SignalConfidenceCalibrator`]
- Legacy adapter: [`LegacySignalAdapter`]
- Built-in modules:
  - [`DeltaMomentumSignal`]
  - [`VolumeImbalanceSignal`]
  - [`CumulativeDeltaSignal`]
  - [`AbsorptionSignal`]
  - [`ExhaustionSignal`]
  - [`SweepDetectionSignal`]
  - [`CompositeSignal`]

## New In 0.4.0

`0.4.0` keeps the [`SignalModule`] trait and built-in signal constructors
stable. The major release-level change is that signals can now sit in a full
developer workflow: analytics -> signal snapshot -> quality gate -> risk gate
-> simulated execution -> journal/replay review.

What changes for signal authors:

- signal modules still consume analytics and quality state; they do not submit
  orders directly;
- execution is an application-level decision made after signal and risk gating;
- the strategy handbook now documents complete signal-to-execution examples in
  Rust, Python, Java, and C-oriented flows;
- replay parity remains the recommended validation path before a signal is
  allowed to drive an execution adapter;
- custom signal modules remain source-compatible with the existing trait.

Version policy:

- `of_signals` publishes as `0.4.0`;
- execution crates publish as `0.1.0` and depend on their own execution-domain
  contracts, not on this signal trait.

## Unreleased Additive Metadata, Lifecycle, And Context APIs

The crate now exposes production-oriented signal metadata, lifecycle helpers,
and contextual signal evaluation without changing the existing [`SignalModule`]
contract.

New additive APIs:

- [`SignalDescriptor`] describes a signal id, version, input requirements,
  warmup policy, parameters, output semantics, determinism, and checkpoint
  support.
- [`SignalInputMask`] declares whether a signal needs analytics, data quality,
  book state, advanced analytics, market-regime, position, or risk context.
- [`SignalWarmupRequirement`] and [`SignalWarmupProgress`] define when a signal
  has enough observed data to become active.
- [`SignalLifecycle`] tracks warmup progress and explicit lifecycle states such
  as `WarmingUp`, `Active`, `Degraded`, `Blocked`, `CoolingDown`, and
  `Disabled`.
- [`SignalOutputSemantics`] lets dashboards and strategy hosts distinguish
  directional, composite, informational, and veto-style outputs.
- [`SignalParameterDescriptor`] and [`SignalParameterValue`] document built-in
  signal constructor parameters.
- [`built_in_signal_descriptors()`] returns metadata for every built-in signal.
- [`describe_signal`] looks up one built-in descriptor by stable signal id.
- [`SignalContext`] carries analytics, quality flags, optional symbol/book
  references, timestamps, lifecycle state, and host extension tags.
- [`ContextualSignalModule`] is a richer additive trait for hosts that evaluate
  signals with more than an analytics snapshot.
- [`LegacySignalAdapter`] lets existing [`SignalModule`] implementations run in
  contextual hosts without being rewritten.
- [`SignalStabilizer`] applies opt-in hysteresis, debounce, and cooldown
  policies to reduce signal flapping without changing built-in signal behavior.
- [`ExplainableSignalModule`] exposes opt-in structured explanations for
  built-in signals without changing [`SignalModule`] or
  [`SignalSnapshot`](of_core::SignalSnapshot).
- [`SignalReasonCode`] adds stable machine-readable rationale codes beside the
  existing human-readable snapshot reason strings.
- [`SignalExplanationMode`] lets hosts emit explanations for every evaluation
  or only when signal state transitions occur.
- [`SignalRegistry`] supports config-time discovery, validation, construction,
  input filtering, and descriptor JSON export for bindings/dashboards.
- [`SignalValidationHarness`] supports replay-based signal validation with
  event-horizon markout labels, monotonic timestamp warnings, confidence
  filters, retained samples, and compact JSON summaries.
- [`SignalOutcomeTracker`] records realized signal outcomes for calibration
  review, per-regime summaries, and baseline/current drift checks.
- [`SignalCalibrationReport`] calculates confidence-bin accuracy and expected
  calibration error without adding serialization or ML dependencies.
- [`SignalConfidenceCalibrator`] and [`SignalCalibrationCurve`] let hosts map
  raw heuristic confidence into calibrated basis-point confidence before
  reporting or gating.

This is intentionally metadata-first. Built-in signal behavior is unchanged:
existing users still call `on_analytics`, `quality_gate`, and `snapshot` exactly
as before.

Custom signal authors can construct descriptors with [`SignalDescriptor::new`]
and parameter metadata with [`SignalParameterDescriptor::new`] or
[`SignalParameterDescriptor::integer`]. Those constructors keep the public
structs future-compatible while still allowing downstream crates to describe
their own modules.

## Public API Inventory

Public types:

- [`SignalGateDecision`]
- [`SignalModule`]
- [`SignalContext`]
- [`ContextualSignalModule`]
- [`LegacySignalAdapter`]
- [`SignalInputMask`]
- [`SignalLifecycleState`]
- [`SignalWarmupProgress`]
- [`SignalWarmupRequirement`]
- [`SignalLifecycle`]
- [`SignalOutputSemantics`]
- [`HysteresisPolicy`]
- [`DebouncePolicy`]
- [`CooldownPolicy`]
- [`SignalTransitionKind`]
- [`SignalSuppressionReason`]
- [`StabilizedSignal`]
- [`SignalStabilizer`]
- [`SignalReasonCode`]
- [`SignalInputValue`]
- [`SignalThreshold`]
- [`SignalConfidenceComponent`]
- [`SignalExplanation`]
- [`SignalExplanationMode`]
- [`ExplainableSignalModule`]
- [`SignalParameterKind`]
- [`SignalParameterValue`]
- [`SignalParameterDescriptor`]
- [`SignalDescriptor`]
- [`SignalConfigValue`]
- [`SignalConfigParameter`]
- [`SignalConfig`]
- [`SignalRegistryError`]
- [`SignalRegistryResult`]
- [`SignalFactory`]
- [`SignalRegistration`]
- [`SignalRegistry`]
- [`SignalMarkoutDirection`]
- [`SignalReplayEvent`]
- [`SignalValidationConfig`]
- [`SignalValidationWarning`]
- [`SignalValidationSample`]
- [`SignalValidationReport`]
- [`SignalValidationHarness`]
- [`SignalCalibrationConfig`]
- [`SignalConfidenceCalibrator`]
- [`IdentitySignalCalibrator`]
- [`SignalCalibrationPoint`]
- [`SignalCalibrationCurve`]
- [`SignalOutcomeRecord`]
- [`SignalCalibrationBin`]
- [`SignalRegimeSummary`]
- [`SignalCalibrationReport`]
- [`SignalCalibrationBinDrift`]
- [`SignalCalibrationDriftReport`]
- [`SignalOutcomeTracker`]
- [`DeltaMomentumSignal`]
- [`VolumeImbalanceSignal`]
- [`CumulativeDeltaSignal`]
- [`AbsorptionSignal`]
- [`ExhaustionSignal`]
- [`SweepDetectionSignal`]
- [`CompositeSignal`]

Public descriptor constants and functions:

- [`DELTA_MOMENTUM_DESCRIPTOR`]
- [`VOLUME_IMBALANCE_DESCRIPTOR`]
- [`CUMULATIVE_DELTA_DESCRIPTOR`]
- [`ABSORPTION_DESCRIPTOR`]
- [`EXHAUSTION_DESCRIPTOR`]
- [`SWEEP_DETECTION_DESCRIPTOR`]
- [`COMPOSITE_DESCRIPTOR`]
- [`built_in_signal_descriptors`]
- [`built_in_signal_registrations`]
- [`built_in_signal_descriptors_json`]
- [`describe_signal`]
- [`validate_signal_replay`]
- [`validate_signal_replay_events`]

Public constructors:

- [`DeltaMomentumSignal::new`]
- [`VolumeImbalanceSignal::new`]
- [`CumulativeDeltaSignal::new`]
- [`AbsorptionSignal::new`]
- [`ExhaustionSignal::new`]
- [`SweepDetectionSignal::new`]
- [`CompositeSignal::new`]
- [`SignalCalibrationConfig::new`]
- [`SignalCalibrationPoint::new`]
- [`SignalCalibrationCurve::new`]
- [`SignalOutcomeRecord::new`]
- [`SignalOutcomeTracker::new`]

[`SignalModule`] trait methods:

- `on_analytics(&AnalyticsSnapshot)`
- `snapshot() -> SignalSnapshot`
- `quality_gate(DataQualityFlags) -> SignalGateDecision`

Signal output uses `of_core::SignalSnapshot` and states such as `LongBias`, `ShortBias`, `Neutral`, and `Blocked`.

## SignalModule Contract

[`SignalModule`] is the extension point for strategy logic.

- `on_analytics(&AnalyticsSnapshot)` consumes the latest analytics state and updates internal signal state.
- `snapshot()` returns the last computed [`SignalSnapshot`](of_core::SignalSnapshot).
- `quality_gate(DataQualityFlags)` tells the runtime whether the signal should be blocked under the current feed-quality conditions.

Recommended implementation rules:

- keep updates deterministic so replay and live runs match
- include human-readable `reason` text in the snapshot when practical
- use `confidence` consistently so downstream hosts can compare modules
- block aggressively on stale, gap, or degraded feed conditions when a strategy should not trade through uncertainty

## Signal Explainability

[`SignalExplanation`] is an optional diagnostic payload for audit logs,
dashboards, replay review, and strategy debugging. It preserves the existing
low-latency path: consumers that only need state still call `snapshot()`, while
hosts that need structured rationale call [`ExplainableSignalModule::explanation`]
on modules that implement the extension trait.

An explanation includes:

- `module_id`, `state`, `confidence_bps`, and `quality_flags` copied from the
  explained snapshot;
- a stable [`SignalReasonCode`] such as `DeltaMomentumPositive` or
  `CompositeNoMajority`;
- the existing human-readable `reason` string;
- observed [`SignalInputValue`] entries;
- configured [`SignalThreshold`] entries;
- optional [`SignalConfidenceComponent`] entries.

The built-in modules implement [`ExplainableSignalModule`]. Custom signal
authors can implement it beside [`SignalModule`] when they want structured
diagnostics without changing their runtime-facing trait contract.

Example:

```rust
use of_core::{AnalyticsSnapshot, SignalState};
use of_signals::{
    DeltaMomentumSignal, ExplainableSignalModule, SignalModule, SignalReasonCode,
};

let mut signal = DeltaMomentumSignal::new(100);
signal.on_analytics(&AnalyticsSnapshot {
    delta: 125,
    ..Default::default()
});

let explanation = signal.explanation();
assert_eq!(explanation.state, SignalState::LongBias);
assert_eq!(
    explanation.reason_code,
    SignalReasonCode::DeltaMomentumPositive
);
```

For high-volume audit streams, use [`SignalExplanationMode::TransitionsOnly`]
to emit explanations only when the signal state changes:

```rust
use of_core::{SignalSnapshot, SignalState};
use of_signals::SignalExplanationMode;

let previous = SignalSnapshot {
    module_id: "delta_momentum_v1",
    state: SignalState::Neutral,
    confidence_bps: 500,
    quality_flags: 0,
    reason: "delta_inside_band".to_string(),
};
let current = SignalSnapshot {
    state: SignalState::LongBias,
    reason: "delta_above_threshold".to_string(),
    ..previous.clone()
};

assert!(SignalExplanationMode::TransitionsOnly
    .should_emit_snapshot(Some(&previous), &current));
```

## Stabilization Policies

[`SignalStabilizer`] is an opt-in helper for hosts that need to reduce noisy
state transitions before a signal reaches strategy/risk/OMS code.

It does not change built-in signal behavior. A host explicitly creates a
stabilizer, feeds requested [`SignalSnapshot`](of_core::SignalSnapshot) values
into it, and uses the returned emitted snapshot.

Policies:

- [`HysteresisPolicy`] requires stronger confidence for entries, exits, or
  direct long/short reversals.
- [`DebouncePolicy`] requires the same candidate state to repeat for a number
  of events and/or remain stable for a time window.
- [`CooldownPolicy`] suppresses new transitions for configured durations after
  accepted entries, exits, or reversals.

Fail-safe rule:

- `SignalState::Blocked` is accepted immediately. Stabilization must not delay
  a quality/risk block.

Example:

```rust
use of_core::{SignalSnapshot, SignalState};
use of_signals::{
    CooldownPolicy, DebouncePolicy, HysteresisPolicy, SignalStabilizer,
    SignalSuppressionReason,
};

let mut stabilizer = SignalStabilizer::with_policies(
    HysteresisPolicy::new(700, 0, 900),
    DebouncePolicy::new(2, 0),
    CooldownPolicy::new(1_000_000, 0, 2_000_000),
);

let requested = SignalSnapshot {
    module_id: "delta_momentum_v1",
    state: SignalState::LongBias,
    confidence_bps: 800,
    quality_flags: 0,
    reason: "delta_above_threshold".to_string(),
};

let first = stabilizer.stabilize(requested.clone(), 1_000);
assert!(!first.accepted);
assert_eq!(first.suppression_reason, SignalSuppressionReason::DebouncePending);

let second = stabilizer.stabilize(requested, 1_001);
assert!(second.accepted);
assert_eq!(second.emitted.state, SignalState::LongBias);
```

Recommended use:

- keep raw signal modules deterministic and simple;
- apply stabilization in the host, runtime, or strategy layer that owns timing
  policy;
- log both `requested` and `emitted` from [`StabilizedSignal`] when reviewing
  suppressed transitions;
- keep policy thresholds explicit per strategy and symbol.

## Contextual Signal Contract

[`ContextualSignalModule`] is the additive extension point for hosts that have
more context than a single analytics snapshot.

It does not replace [`SignalModule`]. Use it when a signal host needs to pass
symbol identity, book state, data-quality state, timestamps, lifecycle state, or
opaque host tags through one evaluation object.

[`SignalContext`] is borrowed. This keeps the hot path allocation-light and
avoids cloning book snapshots just to evaluate a signal.

Context fields:

- `analytics`: required latest [`AnalyticsSnapshot`](of_core::AnalyticsSnapshot)
- `data_quality`: current [`DataQualityFlags`](of_core::DataQualityFlags)
- `symbol`: optional [`SymbolId`](of_core::SymbolId)
- `book`: optional [`BookSnapshot`](of_core::BookSnapshot)
- `ts_exchange_ns`: optional exchange timestamp
- `ts_recv_ns`: optional local receive/evaluation timestamp
- `lifecycle_state`: optional host lifecycle state
- `extension_tags`: optional borrowed host-specific key/value tags

Context example:

```rust
use of_core::{AnalyticsSnapshot, DataQualityFlags, SymbolId};
use of_signals::{SignalContext, SignalLifecycleState};

let analytics = AnalyticsSnapshot {
    delta: 150,
    ..Default::default()
};
let symbol = SymbolId {
    venue: "SIM".to_string(),
    symbol: "ES".to_string(),
};

let ctx = SignalContext::new(&analytics, DataQualityFlags::NONE)
    .with_symbol(&symbol)
    .with_timestamps(Some(1_000), Some(1_010))
    .with_lifecycle_state(SignalLifecycleState::Active);

assert_eq!(ctx.symbol.unwrap().symbol, "ES");
```

Legacy adapter example:

```rust
use of_core::{AnalyticsSnapshot, DataQualityFlags, SignalState};
use of_signals::{
    ContextualSignalModule, DeltaMomentumSignal, LegacySignalAdapter,
    SignalContext, DELTA_MOMENTUM_DESCRIPTOR,
};

let mut signal = LegacySignalAdapter::with_descriptor(
    DeltaMomentumSignal::new(100),
    &DELTA_MOMENTUM_DESCRIPTOR,
);

let analytics = AnalyticsSnapshot {
    delta: 150,
    ..Default::default()
};
let ctx = SignalContext::new(&analytics, DataQualityFlags::NONE);

signal.on_context(&ctx);
assert_eq!(signal.snapshot().state, SignalState::LongBias);
assert_eq!(signal.descriptor().unwrap().id, "delta_momentum_v1");
```

Migration rule:

- keep existing `SignalModule` implementations as-is;
- wrap them with [`LegacySignalAdapter`] when a contextual host requires
  [`ContextualSignalModule`];
- implement [`ContextualSignalModule`] directly only when the signal actually
  needs symbol, book, lifecycle, timestamp, or host-extension context.

## Signal Metadata Contract

[`SignalDescriptor`] is a read-only description of a signal module. It does not
construct or mutate a signal. Use it when an application needs to show users
which signals are compiled in, validate configuration, build dashboard labels,
or document strategy requirements.

Important fields:

- `id` should match the `SignalSnapshot::module_id` emitted by the signal.
- `version` is the descriptor/schema version for the signal definition.
- `required_inputs` declares the context needed by the signal.
- `warmup` declares when output should be considered production-ready.
- `parameters` describes constructor/configuration inputs.
- `output_semantics` tells consumers how to interpret the output.
- `deterministic` records whether replay should match live behavior for the
  same ordered inputs.
- `checkpointable` records whether the current signal implementation exposes
  restorable state.

Descriptor lookup example:

```rust
use of_signals::{
    built_in_signal_descriptors, describe_signal, SignalInputMask,
    SignalParameterValue,
};

let descriptors = built_in_signal_descriptors();
assert!(descriptors.len() >= 7);

let delta = describe_signal("delta_momentum_v1").unwrap();
assert!(delta.required_inputs.contains(SignalInputMask::ANALYTICS));

let threshold = delta.parameter("threshold").unwrap();
assert_eq!(threshold.default, Some(SignalParameterValue::Integer(100)));
```

## Signal Registry And Config Validation

[`SignalRegistry`] is a startup/configuration helper for hosts that need to
discover signals, validate strategy config, construct built-ins, filter by
available inputs, or export metadata to bindings and dashboards.

It is not part of the per-tick hot path. Signal evaluation still happens through
concrete modules and [`SignalModule`].

Example:

```rust
use of_core::{AnalyticsSnapshot, SignalState};
use of_signals::{
    SignalConfig, SignalConfigParameter, SignalModule, SignalRegistry,
};

let registry = SignalRegistry::with_built_ins();
let params = [SignalConfigParameter::integer("threshold", 25)];
let config = SignalConfig::with_parameters("delta_momentum_v1", &params);

registry.validate_config(&config)?;
let mut signal = registry.create_signal(&config)?;

signal.on_analytics(&AnalyticsSnapshot {
    delta: 30,
    ..Default::default()
});

assert_eq!(signal.snapshot().state, SignalState::LongBias);
# Ok::<(), of_signals::SignalRegistryError>(())
```

Common registry operations:

- [`SignalRegistry::with_built_ins`] creates a registry with all bundled
  signals and factories.
- [`SignalRegistry::register`] adds custom descriptor/factory pairs.
- [`SignalRegistry::descriptor`] looks up one descriptor by stable id.
- [`SignalRegistry::descriptors_matching_inputs`] filters descriptors by
  available context.
- [`SignalRegistry::validate_config`] checks unknown signals, duplicate
  parameters, unknown parameters, type mismatches, and descriptor ranges.
- [`SignalRegistry::create_signal`] constructs a boxed [`SignalModule`] from a
  valid config.
- [`SignalRegistry::descriptors_json`] exports compact descriptor metadata.

The crate intentionally avoids adding `serde` to the signal core for this
feature. JSON export is generated from static descriptor metadata so binding
layers can expose inventory data without forcing serialization dependencies into
every Rust user.

## Replay Signal Validation

[`SignalValidationHarness`] validates a signal against ordered analytics
snapshots. It is designed for research, replay review, CI checks, and notebook
workflows, not for the live per-tick decision path.

The harness sequence is explicit:

1. feed the current [`AnalyticsSnapshot`](of_core::AnalyticsSnapshot) into the
   signal;
2. capture the current [`SignalSnapshot`](of_core::SignalSnapshot);
3. compute a future event-horizon markout label only for scoring.

That keeps future prices out of signal evaluation and reduces lookahead-bias
risk in validation code. When timestamp metadata is available, use
[`SignalReplayEvent`] and [`validate_signal_replay_events`] to warn on
non-monotonic replay ordering.

Example:

```rust
use of_core::AnalyticsSnapshot;
use of_signals::{
    validate_signal_replay, DeltaMomentumSignal, SignalValidationConfig,
};

let mut signal = DeltaMomentumSignal::new(10);
let events = vec![
    AnalyticsSnapshot {
        delta: 20,
        last_price: 100,
        ..Default::default()
    },
    AnalyticsSnapshot {
        delta: 20,
        last_price: 110,
        ..Default::default()
    },
];

let config = SignalValidationConfig::new(1).with_store_samples(true);
let report = validate_signal_replay(&mut signal, &events, config);

assert_eq!(report.labeled_events, 1);
assert_eq!(report.directional_accuracy_bps(), Some(10_000));
assert!(report.json_summary().contains("\"evaluated_events\":2"));
```

Validation concepts:

- [`SignalMarkoutDirection`] labels future price movement as `Up`, `Down`, or
  `Flat`.
- [`SignalValidationConfig::markout_horizon_events`] controls the future event
  offset used for labels.
- [`SignalValidationConfig::flat_price_threshold`] prevents tiny price changes
  from being counted as directional.
- [`SignalValidationConfig::min_confidence_bps`] filters weak directional
  predictions from accuracy scoring.
- [`SignalValidationConfig::store_samples`] retains per-event
  [`SignalValidationSample`] values for deeper review.
- [`SignalValidationReport::json_summary`] returns dependency-free JSON for
  Python, notebooks, dashboards, or CI artifacts.

## Calibration And Outcome Tracking

[`SignalOutcomeTracker`] converts retained validation samples or live/post-trade
outcome records into calibration reports. This helps operators answer a
different question from raw accuracy: when a signal says it is 70% confident,
does it behave like a 70% signal over realized markouts?

The implementation is intentionally lightweight:

- confidence values are basis points (`0..=10_000`), matching the existing
  [`SignalSnapshot`](of_core::SignalSnapshot) convention;
- [`SignalCalibrationReport`] uses binned empirical accuracy and expected
  calibration error (ECE);
- [`SignalCalibrationCurve`] can map raw heuristic confidence into calibrated
  confidence before reporting;
- [`SignalCalibrationDriftReport`] compares a current report to a baseline;
- [`SignalRegimeSummary`] keeps per-regime accuracy and confidence summaries;
- no ML, dataframe, or serialization dependency is added to the signal hot
  path.

Example from replay validation:

```rust
use of_core::AnalyticsSnapshot;
use of_signals::{
    validate_signal_replay, DeltaMomentumSignal, SignalCalibrationConfig,
    SignalCalibrationReport, SignalValidationConfig,
};

let mut signal = DeltaMomentumSignal::new(10);
let events = vec![
    AnalyticsSnapshot {
        delta: 20,
        last_price: 100,
        ..Default::default()
    },
    AnalyticsSnapshot {
        delta: -20,
        last_price: 90,
        ..Default::default()
    },
    AnalyticsSnapshot {
        delta: -20,
        last_price: 80,
        ..Default::default()
    },
];

let validation = validate_signal_replay(
    &mut signal,
    &events,
    SignalValidationConfig::new(1).with_store_samples(true),
);
let calibration = SignalCalibrationReport::from_validation_report(
    &validation,
    SignalCalibrationConfig::new(1_000),
);

assert_eq!(calibration.scored_records, 2);
assert_eq!(calibration.accuracy_bps(), Some(5_000));
assert!(calibration.expected_calibration_error_bps > 0);
```

Example with an explicit tracker and drift comparison:

```rust
use of_signals::{
    SignalCalibrationConfig, SignalCalibrationDriftReport,
    SignalMarkoutDirection, SignalOutcomeRecord, SignalOutcomeTracker,
};

let config = SignalCalibrationConfig::new(1_000)
    .with_min_samples_per_bin(1)
    .with_drift_alert_threshold_bps(500);
let mut baseline_tracker = SignalOutcomeTracker::new(config);
baseline_tracker.record(
    SignalOutcomeRecord::new(
        "delta_momentum_v1",
        of_core::SignalState::LongBias,
        8_000,
        Some(SignalMarkoutDirection::Up),
        SignalMarkoutDirection::Up,
        Some(true),
    )
    .with_regime("trend"),
);

let baseline = baseline_tracker.calibration_report();
let mut current_tracker = SignalOutcomeTracker::new(config);
current_tracker.record(
    SignalOutcomeRecord::new(
        "delta_momentum_v1",
        of_core::SignalState::LongBias,
        8_000,
        Some(SignalMarkoutDirection::Up),
        SignalMarkoutDirection::Down,
        Some(false),
    )
    .with_regime("trend"),
);

let drift: SignalCalibrationDriftReport =
    current_tracker.drift_report(&baseline);
assert!(drift.significant);
```

Use calibration reports for research, deployment review, model governance, and
drift monitoring. Do not treat them as an automatic trading permission system:
the strategy host should still apply data-quality, risk, OMS, and venue-health
gates before order submission.

Custom descriptor example:

```rust
use of_signals::{
    SignalDescriptor, SignalInputMask, SignalOutputSemantics,
    SignalParameterDescriptor, SignalWarmupRequirement,
};

const PARAMS: &[SignalParameterDescriptor] = &[
    SignalParameterDescriptor::integer(
        "lookback_events",
        "Number of events used by the custom signal.",
        Some(32),
        Some(1),
        Some(10_000),
    ),
];

let descriptor = SignalDescriptor::new(
    "custom_signal_v1",
    "Custom Signal",
    "1",
    "Example custom signal descriptor.",
)
.with_required_inputs(SignalInputMask::ANALYTICS | SignalInputMask::DATA_QUALITY)
.with_warmup(SignalWarmupRequirement::Events(32))
.with_parameters(PARAMS)
.with_output_semantics(SignalOutputSemantics::DirectionalBias)
.with_checkpointable(true);

assert_eq!(descriptor.id, "custom_signal_v1");
```

## Signal Lifecycle And Warmup

[`SignalLifecycle`] is a small utility for production wrappers that need to
avoid using a signal before enough data has arrived.

It is not automatically wired into existing built-ins because that would change
runtime behavior. Instead, it gives host applications and future contextual
signal wrappers a deterministic lifecycle model.

Lifecycle example:

```rust
use of_signals::{
    SignalLifecycle, SignalLifecycleState, SignalWarmupRequirement,
};

let mut lifecycle = SignalLifecycle::new(SignalWarmupRequirement::Events(2));
assert_eq!(lifecycle.state(), SignalLifecycleState::WarmingUp);

lifecycle.record_event();
assert_eq!(lifecycle.state(), SignalLifecycleState::WarmingUp);

lifecycle.record_event();
assert_eq!(lifecycle.state(), SignalLifecycleState::Active);
```

Lifecycle states:

- `Initializing`: object exists but is not evaluating yet.
- `WarmingUp`: data is flowing, but the warmup requirement is not met.
- `Active`: output can be consumed normally.
- `Degraded`: output exists but should be treated cautiously.
- `Blocked`: output must not be used for trading decisions.
- `CoolingDown`: output is intentionally suppressing rapid transitions.
- `Disabled`: evaluation is configured off.

## Constructor Parameter Reference

- [`DeltaMomentumSignal::new`] takes an absolute `delta` threshold.
- [`VolumeImbalanceSignal::new`] takes an absolute session `buy_volume - sell_volume` threshold.
- [`CumulativeDeltaSignal::new`] takes an absolute `cumulative_delta` threshold.
- [`AbsorptionSignal::new`] takes:
  - `threshold`: directional pressure required before checking for absorption
  - `price_band`: max distance from POC/value location used by the heuristic
- [`ExhaustionSignal::new`] takes an absolute delta threshold for stalled reversal detection.
- [`SweepDetectionSignal::new`] takes:
  - `threshold`: directional delta threshold
  - `breakout_ticks`: minimum break outside value area
- [`CompositeSignal::new`] takes owned child modules and aggregates their votes.

## Delta Momentum Strategy

[`DeltaMomentumSignal`] is a reference implementation that:

- emits `LongBias` when `delta >= threshold`
- emits `ShortBias` when `delta <= -threshold`
- emits `Neutral` otherwise
- emits `Blocked` in runtime when quality gate fails

## Volume Imbalance Strategy

[`VolumeImbalanceSignal`] is a reference implementation that:

- compares session `buy_volume - sell_volume` against an absolute threshold
- emits `LongBias` when buy pressure dominates
- emits `ShortBias` when sell pressure dominates
- remains `Neutral` while the session imbalance stays inside the configured band

## Cumulative Delta Strategy

[`CumulativeDeltaSignal`] is a session-bias module that:

- compares `cumulative_delta` against an absolute threshold
- emits `LongBias` when session delta remains strongly positive
- emits `ShortBias` when session delta remains strongly negative
- remains `Neutral` while cumulative delta stays inside the configured band

## Absorption Strategy

[`AbsorptionSignal`] is a heuristic module that:

- looks for strong directional delta that fails to move price away from POC
- emits `LongBias` on sell absorption near POC
- emits `ShortBias` on buy absorption near POC

## Exhaustion Strategy

[`ExhaustionSignal`] is a heuristic reversal module that:

- looks for strong directional delta that stalls back near POC
- emits `ShortBias` when buying appears exhausted
- emits `LongBias` when selling appears exhausted

## Sweep Detection Strategy

[`SweepDetectionSignal`] is a breakout module that:

- looks for strong delta alongside a break outside value area
- emits `LongBias` on upside sweeps
- emits `ShortBias` on downside sweeps

## Composite Strategy

[`CompositeSignal`] combines multiple child modules and:

- updates each child on the same analytics snapshot
- emits the majority directional view when one side has more votes
- remains `Neutral` when there is no directional majority

## Output Interpretation

All built-in modules return [`SignalSnapshot`](of_core::SignalSnapshot), which downstream runtimes and bindings expose unchanged.

- `state` is the durable directional state
- `confidence` is a normalized score chosen by the module
- `reason` is short human-readable rationale
- `quality_flags` echoes the quality context that contributed to blocking or caution

Built-in modules are intentionally heuristic rather than venue-specific alpha models.
They are meant as production-ready references and defaults, not as the only strategy approach.

## Quick Example

```rust
use of_core::{AnalyticsSnapshot, SignalState};
use of_signals::{DeltaMomentumSignal, SignalModule};

let mut signal = DeltaMomentumSignal::new(100);
signal.on_analytics(&AnalyticsSnapshot {
    delta: 150,
    ..Default::default()
});

let snapshot = signal.snapshot();
assert!(matches!(snapshot.state, SignalState::LongBias));
```

## Alternative Module Example

```rust
use of_core::{AnalyticsSnapshot, SignalState};
use of_signals::{SignalModule, VolumeImbalanceSignal};

let mut signal = VolumeImbalanceSignal::new(100);
signal.on_analytics(&AnalyticsSnapshot {
    buy_volume: 350,
    sell_volume: 200,
    ..Default::default()
});

let snapshot = signal.snapshot();
assert!(matches!(snapshot.state, SignalState::LongBias));
```

## Composite Example

```rust
use of_core::{AnalyticsSnapshot, SignalState};
use of_signals::{
    CompositeSignal, CumulativeDeltaSignal, DeltaMomentumSignal, SignalModule,
    VolumeImbalanceSignal,
};

let mut signal = CompositeSignal::new(vec![
    Box::new(DeltaMomentumSignal::new(100)),
    Box::new(VolumeImbalanceSignal::new(100)),
    Box::new(CumulativeDeltaSignal::new(150)),
]);
signal.on_analytics(&AnalyticsSnapshot {
    delta: 200,
    cumulative_delta: 250,
    buy_volume: 400,
    sell_volume: 100,
    ..Default::default()
});

let snapshot = signal.snapshot();
assert!(matches!(snapshot.state, SignalState::LongBias));
```

## Quality Gate Example

```rust
use of_core::DataQualityFlags;
use of_signals::{DeltaMomentumSignal, SignalGateDecision, SignalModule};

let signal = DeltaMomentumSignal::default();
let gate = signal.quality_gate(DataQualityFlags::SEQUENCE_GAP);
assert_eq!(gate, SignalGateDecision::Block);
```

## Implementing Your Own Signal Module

Implement [`SignalModule`] and keep it:

- deterministic (important for replay parity)
- explicit about confidence and reason fields
- strict about quality gating for unsafe feed states
- compatible with the stable `SignalSnapshot` contract so bindings and FFI callers keep working

## Real-World Use Cases

### 1. Gate entries on feed quality

Even a simple threshold signal becomes materially safer when `quality_gate(...)`
blocks action during stale, gap, or degraded feed states.

### 2. Build multi-factor orderflow strategies

Use several modules together to express:

- context: `CumulativeDeltaSignal`
- trigger: `SweepDetectionSignal`
- reversal filter: `AbsorptionSignal` or `ExhaustionSignal`

### 3. Keep strategy logic deterministic in replay

Because modules consume normalized analytics instead of live provider payloads,
the same module can be replayed over historical sessions and compared to live
behavior with much less drift.

## Strategy Pattern: Composite Confirmation

A practical strategy stack often looks like:

1. `CumulativeDeltaSignal` for directional regime bias
2. `VolumeImbalanceSignal` for immediate orderflow pressure
3. `SweepDetectionSignal` for breakout confirmation
4. `CompositeSignal` to require majority confirmation

## Detailed Example: Build A Composite Intraday Bias Module

```rust
use of_core::{AnalyticsSnapshot, DataQualityFlags, SignalState};
use of_signals::{
    CompositeSignal, CumulativeDeltaSignal, SweepDetectionSignal, SignalGateDecision,
    SignalModule, VolumeImbalanceSignal,
};

fn main() {
    let mut signal = CompositeSignal::new(vec![
        Box::new(CumulativeDeltaSignal::new(400)),
        Box::new(VolumeImbalanceSignal::new(150)),
        Box::new(SweepDetectionSignal::new(200, 4)),
    ]);

    let analytics = AnalyticsSnapshot {
        buy_volume: 900,
        sell_volume: 500,
        delta: 400,
        cumulative_delta: 650,
        last_price: 505_075,
        point_of_control: 505_000,
        value_area_low: 504_750,
        value_area_high: 505_050,
        ..Default::default()
    };

    if signal.quality_gate(DataQualityFlags::NONE) == SignalGateDecision::Pass {
        signal.on_analytics(&analytics);
        let snapshot = signal.snapshot();
        if matches!(snapshot.state, SignalState::LongBias) {
            println!("long bias: {}", snapshot.reason);
        }
    }
}
```

## Detailed Example: Write Your Own Signal Module

```rust
use of_core::{AnalyticsSnapshot, DataQualityFlags, SignalSnapshot, SignalState};
use of_signals::{SignalGateDecision, SignalModule};

struct POCReclaimSignal {
    last: SignalSnapshot,
}

impl Default for POCReclaimSignal {
    fn default() -> Self {
        Self {
            last: SignalSnapshot {
                module_id: "poc_reclaim_v1",
                state: SignalState::Neutral,
                confidence_bps: 0,
                quality_flags: 0,
                reason: "init".to_string(),
            },
        }
    }
}

impl SignalModule for POCReclaimSignal {
    fn on_analytics(&mut self, analytics: &AnalyticsSnapshot) {
        let state = if analytics.delta > 200 && analytics.point_of_control >= analytics.value_area_low {
            SignalState::LongBias
        } else if analytics.delta < -200 && analytics.point_of_control <= analytics.value_area_high {
            SignalState::ShortBias
        } else {
            SignalState::Neutral
        };

        self.last = SignalSnapshot {
            module_id: "poc_reclaim_v1",
            state,
            confidence_bps: 7000,
            reason: "POC reclaim heuristic".to_string(),
            quality_flags: 0,
        };
    }

    fn snapshot(&self) -> SignalSnapshot {
        self.last.clone()
    }

    fn quality_gate(&self, flags: DataQualityFlags) -> SignalGateDecision {
        if flags.intersects(
            DataQualityFlags::STALE_FEED
                | DataQualityFlags::SEQUENCE_GAP
                | DataQualityFlags::ADAPTER_DEGRADED,
        ) {
            SignalGateDecision::Block
        } else {
            SignalGateDecision::Pass
        }
    }
}
```
