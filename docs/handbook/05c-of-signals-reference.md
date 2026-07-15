# `of_signals` Reference

`of_signals` contains the runtime-facing strategy modules that convert
analytics snapshots into stable directional states. The crate is intentionally
separated from transport and persistence so strategy logic stays deterministic
and easy to test.

## Public API Map

| Item | Kind | Purpose |
| --- | --- | --- |
| `SignalGateDecision` | enum | Quality gate result |
| `SignalModule` | trait | Signal extension point |
| `SignalContext` | struct | Borrowed contextual evaluation input |
| `ContextualSignalModule` | trait | Additive richer signal extension point |
| `LegacySignalAdapter` | struct | Adapter from `SignalModule` to contextual API |
| `SignalInputMask` | struct | Bitset of inputs required by a signal |
| `SignalLifecycleState` | enum | Production lifecycle state |
| `SignalWarmupProgress` | struct | Counters used to evaluate warmup |
| `SignalWarmupRequirement` | enum | Warmup policy for a signal |
| `SignalLifecycle` | struct | Small lifecycle/warmup helper |
| `HysteresisPolicy` | struct | Confidence thresholds for transition acceptance |
| `DebouncePolicy` | struct | Event/time confirmation policy |
| `CooldownPolicy` | struct | Time-based post-transition suppression policy |
| `SignalStabilizer` | struct | Opt-in hysteresis/debounce/cooldown helper |
| `StabilizedSignal` | struct | Requested and emitted signal pair |
| `SignalTransitionKind` | enum | Classified transition type |
| `SignalSuppressionReason` | enum | Reason a request was suppressed |
| `SignalReasonCode` | enum | Stable machine-readable signal rationale |
| `SignalInputValue` | struct | Observed input included in an explanation |
| `SignalThreshold` | struct | Configured threshold included in an explanation |
| `SignalConfidenceComponent` | struct | Confidence contributor included in an explanation |
| `SignalExplanation` | struct | Structured diagnostic payload for a snapshot |
| `SignalExplanationMode` | enum | Always-on or transition-only explanation emission |
| `ExplainableSignalModule` | trait | Optional extension trait for structured explanations |
| `SignalDescriptor` | struct | Static metadata for signal discovery |
| `SignalParameterDescriptor` | struct | Static metadata for one parameter |
| `SignalOutputSemantics` | enum | How consumers should interpret output |
| `built_in_signal_descriptors` | function | Returns all built-in signal descriptors |
| `built_in_signal_registrations` | function | Returns built-in descriptor/factory registrations |
| `built_in_signal_descriptors_json` | function | Returns compact descriptor JSON |
| `describe_signal` | function | Looks up one built-in descriptor by id |
| `SignalConfig` | struct | Borrowed signal construction config |
| `SignalConfigParameter` | struct | Named config parameter |
| `SignalConfigValue` | enum | Typed config value |
| `SignalRegistration` | struct | Descriptor plus optional construction factory |
| `SignalRegistry` | struct | Discovery, validation, construction, and JSON export |
| `SignalRegistryError` | enum | Typed validation/construction error |
| `SignalMarkoutDirection` | enum | Future markout label for validation |
| `SignalReplayEvent` | struct | Analytics snapshot plus optional timestamp |
| `SignalValidationConfig` | struct | Replay validation settings |
| `SignalValidationWarning` | enum | Validation warnings |
| `SignalValidationSample` | struct | One scored replay sample |
| `SignalValidationReport` | struct | Aggregate validation report |
| `SignalValidationHarness` | struct | Configured replay validation runner |
| `validate_signal_replay` | function | Validates a signal over analytics snapshots |
| `validate_signal_replay_events` | function | Validates a signal over timestamped replay events |
| `SignalCalibrationConfig` | struct | Confidence-bin and drift settings |
| `SignalConfidenceCalibrator` | trait | Maps raw confidence into calibrated confidence |
| `IdentitySignalCalibrator` | struct | Pass-through confidence calibrator |
| `SignalCalibrationPoint` | struct | One calibration curve point |
| `SignalCalibrationCurve` | struct | Piecewise-linear confidence calibration curve |
| `SignalOutcomeRecord` | struct | One realized prediction/markout outcome |
| `SignalCalibrationBin` | struct | Per-confidence-bin calibration summary |
| `SignalRegimeSummary` | struct | Per-regime calibration summary |
| `SignalCalibrationReport` | struct | Aggregate confidence calibration report |
| `SignalCalibrationBinDrift` | struct | Per-bin baseline/current comparison |
| `SignalCalibrationDriftReport` | struct | Expected calibration error drift report |
| `SignalOutcomeTracker` | struct | Incremental outcome recorder and reporter |
| `DeltaMomentumSignal` | struct | Base delta threshold module |
| `VolumeImbalanceSignal` | struct | Session volume imbalance module |
| `CumulativeDeltaSignal` | struct | Session cumulative delta module |
| `AbsorptionSignal` | struct | Near-POC absorption heuristic |
| `ExhaustionSignal` | struct | Directional exhaustion heuristic |
| `SweepDetectionSignal` | struct | Value-area breakout heuristic |
| `CompositeSignal` | struct | Majority-vote aggregator |

## Shared Types

### `SignalGateDecision`

| Variant | Meaning |
| --- | --- |
| `Pass` | Runtime may use or emit the signal |
| `Block` | Runtime should block the signal under current quality conditions |

### `SignalModule` Trait

| Method | Returns | Meaning |
| --- | --- | --- |
| `on_analytics(&AnalyticsSnapshot)` | `()` | Updates module state from latest analytics |
| `snapshot()` | `SignalSnapshot` | Returns current signal output |
| `quality_gate(DataQualityFlags)` | `SignalGateDecision` | Decides if quality state should block the module |

#### Implementation rules

- Modules should remain deterministic so replay and live behavior match.
- `snapshot()` should be cheap and side-effect free.
- `quality_gate(...)` should be conservative for stale, gap, or degraded feed
  conditions when the model should not trade through uncertainty.

## Explainability

The explainability API is additive. It does not change `SignalModule`,
`SignalSnapshot`, built-in constructors, or existing runtime/binding outputs.

### `ExplainableSignalModule`

`ExplainableSignalModule` is an optional extension trait for modules that can
return structured diagnostics for their current snapshot.

| Method | Returns | Meaning |
| --- | --- | --- |
| `explanation()` | `SignalExplanation` | Returns rationale, inputs, thresholds, and confidence contributors |

Built-in modules implement this trait. Custom modules can implement it beside
`SignalModule` when audit/replay consumers need more than a reason string.

### `SignalExplanation`

| Field | Meaning |
| --- | --- |
| `module_id` | Stable signal module id |
| `state` | Explained signal state |
| `confidence_bps` | Confidence copied from the snapshot |
| `quality_flags` | Quality flags copied from the snapshot |
| `reason_code` | Stable `SignalReasonCode` |
| `reason` | Existing human-readable reason text |
| `inputs` | Observed `SignalInputValue` entries |
| `thresholds` | Configured `SignalThreshold` entries |
| `confidence_components` | Optional confidence contributors |

`SignalReasonCode::as_str()` returns stable snake-case values for logs,
dashboards, and binding adapters.

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

### `SignalExplanationMode`

| Variant | Meaning |
| --- | --- |
| `Always` | Emit an explanation for every evaluation |
| `TransitionsOnly` | Emit only when state changes from the previous snapshot |

Use transition-only mode when audit volume matters more than every intermediate
evaluation. It is a host-side policy helper; it does not suppress or mutate the
actual signal snapshot.

## Stabilization

`SignalStabilizer` is an opt-in helper for reducing signal flapping before a
host passes signal output into strategy, risk, or OMS code. It does not change
any built-in signal behavior automatically.

### Policies

| Policy | Purpose |
| --- | --- |
| `HysteresisPolicy` | Requires minimum confidence for entry, exit, or reversal |
| `DebouncePolicy` | Requires repeated and/or time-stable candidate states |
| `CooldownPolicy` | Suppresses transitions after accepted entries, exits, or reversals |

### Output

`SignalStabilizer::stabilize(...)` returns `StabilizedSignal`.

| Field | Meaning |
| --- | --- |
| `requested` | Raw snapshot requested by the underlying signal |
| `emitted` | Snapshot emitted after stabilization |
| `accepted` | Whether requested became emitted |
| `suppression_reason` | `None`, `Hysteresis`, `DebouncePending`, or `CooldownActive` |
| `transition` | `None`, `Entry`, `Exit`, `Reversal`, or `StateChange` |

`SignalState::Blocked` is accepted immediately. Stabilization must not delay a
quality or risk block.

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
assert_eq!(first.suppression_reason, SignalSuppressionReason::DebouncePending);

let second = stabilizer.stabilize(requested, 1_001);
assert!(second.accepted);
```

Use stabilization in the host or strategy layer that owns timing policy. Keep
raw signal modules deterministic and simple.

## Contextual API

The contextual API is additive. Existing `SignalModule` implementations remain
valid and do not need to be rewritten.

### `SignalContext`

`SignalContext` is a borrowed evaluation object for richer signal hosts.

| Field | Meaning |
| --- | --- |
| `analytics` | Required `AnalyticsSnapshot` |
| `data_quality` | Current `DataQualityFlags` |
| `symbol` | Optional `SymbolId` for multi-symbol hosts |
| `book` | Optional materialized `BookSnapshot` |
| `ts_exchange_ns` | Optional exchange timestamp |
| `ts_recv_ns` | Optional receive/evaluation timestamp |
| `lifecycle_state` | Optional host lifecycle state |
| `extension_tags` | Optional borrowed key/value tags for host-specific context |

Example:

```rust
use of_core::{AnalyticsSnapshot, DataQualityFlags, SymbolId};
use of_signals::{SignalContext, SignalLifecycleState};

let analytics = AnalyticsSnapshot {
    delta: 125,
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

assert_eq!(ctx.analytics.delta, 125);
```

### `ContextualSignalModule`

`ContextualSignalModule` is for modules that consume `SignalContext`.

| Method | Returns | Meaning |
| --- | --- | --- |
| `on_context(&SignalContext)` | `()` | Updates module state from contextual input |
| `snapshot()` | `SignalSnapshot` | Returns current signal output |
| `quality_gate(&SignalContext)` | `SignalGateDecision` | Applies contextual quality gate |
| `descriptor()` | `Option<&'static SignalDescriptor>` | Returns metadata if available |
| `lifecycle_state()` | `Option<SignalLifecycleState>` | Returns lifecycle state if tracked |

### `LegacySignalAdapter`

`LegacySignalAdapter` wraps an existing `SignalModule` and implements
`ContextualSignalModule` by forwarding `ctx.analytics` to `on_analytics` and
`ctx.data_quality` to the wrapped quality gate.

Example:

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
```

Use the legacy adapter when migrating existing modules into contextual hosts.
Implement `ContextualSignalModule` directly only when the module needs symbol,
book, timestamp, lifecycle, or extension-tag context.

## Metadata And Lifecycle

The metadata and lifecycle APIs are additive. They do not change
`SignalModule`, built-in constructors, or existing signal outputs.

### `SignalDescriptor`

`SignalDescriptor` describes a signal for discovery, configuration validation,
dashboards, bindings, and documentation.

| Field | Meaning |
| --- | --- |
| `id` | Stable signal id, matching `SignalSnapshot::module_id` |
| `name` | Human-readable name |
| `version` | Descriptor/schema version for the signal definition |
| `description` | Human-readable description |
| `required_inputs` | `SignalInputMask` declaring required context |
| `warmup` | `SignalWarmupRequirement` before production use |
| `parameters` | Static parameter metadata |
| `output_semantics` | Directional, composite, informational, or veto output |
| `deterministic` | Whether replay should match live for the same ordered inputs |
| `checkpointable` | Whether the implementation exposes restorable signal state |

Built-in descriptors:

| Constant | Signal id |
| --- | --- |
| `DELTA_MOMENTUM_DESCRIPTOR` | `delta_momentum_v1` |
| `VOLUME_IMBALANCE_DESCRIPTOR` | `volume_imbalance_v1` |
| `CUMULATIVE_DELTA_DESCRIPTOR` | `cumulative_delta_v1` |
| `ABSORPTION_DESCRIPTOR` | `absorption_v1` |
| `EXHAUSTION_DESCRIPTOR` | `exhaustion_v1` |
| `SWEEP_DETECTION_DESCRIPTOR` | `sweep_detection_v1` |
| `COMPOSITE_DESCRIPTOR` | `composite_v1` |

External signal authors can construct descriptors with `SignalDescriptor::new`
and parameter metadata with `SignalParameterDescriptor::new` or
`SignalParameterDescriptor::integer`. Use the `with_*` descriptor methods to
attach required inputs, warmup, parameters, output semantics, and capability
flags. This keeps descriptor structs future-compatible while still allowing
downstream crates to describe custom signals.

Example:

```rust
use of_signals::{describe_signal, SignalInputMask, SignalParameterValue};

let descriptor = describe_signal("delta_momentum_v1").unwrap();
assert!(descriptor.requires_input(SignalInputMask::ANALYTICS));

let threshold = descriptor.parameter("threshold").unwrap();
assert_eq!(threshold.default, Some(SignalParameterValue::Integer(100)));
```

## Registry And Config

`SignalRegistry` is an additive startup/configuration API. It lets hosts
discover signals, validate user config, construct built-in modules, filter by
available inputs, and export descriptor metadata without changing the
`SignalModule` trait.

### Registry Types

| Type | Meaning |
| --- | --- |
| `SignalConfig` | Borrowed signal id plus parameter slice |
| `SignalConfigParameter` | One named config parameter |
| `SignalConfigValue` | Integer, float, boolean, or text value |
| `SignalRegistration` | Descriptor plus optional factory |
| `SignalFactory` | Function pointer that builds `Box<dyn SignalModule>` |
| `SignalRegistry` | Owned registry of signal registrations |
| `SignalRegistryError` | Unknown id, duplicate id/parameter, type/range, or missing factory error |

### Registry Operations

| Method | Meaning |
| --- | --- |
| `SignalRegistry::with_built_ins()` | Creates a registry with bundled modules |
| `register(...)` | Adds a custom descriptor/factory pair |
| `registrations()` | Returns registered metadata |
| `descriptor(id)` | Looks up one descriptor |
| `descriptors_matching_inputs(mask)` | Filters descriptors by available context |
| `validate_config(&config)` | Validates config without constructing |
| `create_signal(&config)` | Constructs a boxed module from valid config |
| `descriptors_json()` | Exports compact descriptor metadata |

Validation catches:

- unknown signal ids;
- duplicate parameter names;
- unknown parameter names;
- parameter type mismatches;
- configured values outside descriptor min/max bounds;
- registrations without factories during construction.

Example:

```rust
use of_core::{AnalyticsSnapshot, SignalState};
use of_signals::{SignalConfig, SignalConfigParameter, SignalRegistry};

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

Descriptor JSON is generated without adding `serde` to `of_signals`:

```rust
use of_signals::built_in_signal_descriptors_json;

let json = built_in_signal_descriptors_json();
assert!(json.contains("\"id\":\"delta_momentum_v1\""));
```

## Replay Validation Harness

The validation harness is an additive research/replay API. It evaluates a
signal over ordered analytics snapshots and scores directional outputs against
future event-horizon markout labels.

The harness avoids lookahead in its own sequencing:

1. feed the current snapshot into the signal;
2. capture the signal output;
3. compute the future markout label after the output has been captured.

Use timestamped `SignalReplayEvent` inputs when the replay source can provide
exchange timestamps. The harness warns when timestamps move backward.

### Validation Types

| Type | Meaning |
| --- | --- |
| `SignalMarkoutDirection` | `Up`, `Down`, or `Flat` future price label |
| `SignalReplayEvent` | Borrowed analytics snapshot plus optional exchange timestamp |
| `SignalValidationConfig` | Horizon, flat threshold, confidence filter, sample retention, timestamp checks |
| `SignalValidationWarning` | Empty input, zero horizon, missing markout, or non-monotonic timestamp |
| `SignalValidationSample` | One event-level prediction/label pair |
| `SignalValidationReport` | Aggregate counts, accuracy, coverage, retained samples, warnings |
| `SignalValidationHarness` | Reusable configured validator |

Example:

```rust
use of_core::AnalyticsSnapshot;
use of_signals::{validate_signal_replay, DeltaMomentumSignal, SignalValidationConfig};

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

let report = validate_signal_replay(
    &mut signal,
    &events,
    SignalValidationConfig::new(1).with_store_samples(true),
);

assert_eq!(report.labeled_events, 1);
assert_eq!(report.directional_accuracy_bps(), Some(10_000));
```

Useful report methods:

| Method | Meaning |
| --- | --- |
| `directional_accuracy_bps()` | Correct directional predictions divided by scored directional predictions |
| `label_coverage_bps()` | Events with labels divided by evaluated events |
| `has_warnings()` | Whether validation emitted warnings |
| `json_summary()` | Compact JSON summary for notebooks, Python, dashboards, or CI artifacts |

## Calibration And Outcome Tracking

Calibration reports are additive research and monitoring APIs. They do not
change `SignalModule`, built-in constructors, signal snapshots, registry
factories, runtime behavior, or binding ABI.

The goal is to measure whether signal confidence aligns with realized
directional outcomes. A signal can be directionally accurate but poorly
calibrated if its confidence is systematically too high or too low for the
observed markout accuracy.

### Calibration Types

| Type | Meaning |
| --- | --- |
| `SignalCalibrationConfig` | Confidence bin width, minimum samples per bin, and drift alert threshold |
| `SignalConfidenceCalibrator` | Trait for mapping raw confidence basis points to calibrated confidence basis points |
| `IdentitySignalCalibrator` | Default pass-through calibrator |
| `SignalCalibrationPoint` | One raw-to-calibrated confidence point |
| `SignalCalibrationCurve` | Piecewise-linear calibrator over sorted points |
| `SignalOutcomeRecord` | Module id, state, confidence, prediction, markout label, correctness, and optional regime |
| `SignalCalibrationBin` | Samples, correct count, average confidence, accuracy, and absolute calibration error for one bin |
| `SignalRegimeSummary` | Samples, correct count, average confidence, and accuracy by regime label |
| `SignalCalibrationReport` | Aggregate record counts, ECE, bins, and regime summaries |
| `SignalCalibrationBinDrift` | Baseline/current sample counts and accuracy delta for one bin |
| `SignalCalibrationDriftReport` | Baseline/current ECE comparison and significance flag |
| `SignalOutcomeTracker` | Incremental recorder that can emit calibration and drift reports |

### Report Semantics

| Field | Meaning |
| --- | --- |
| `total_records` | All inspected outcome records |
| `scored_records` | Records with `correct: Some(...)` |
| `ignored_records` | Records not eligible for calibration scoring |
| `correct_records` | Correct scored directional predictions |
| `expected_calibration_error_bps` | Weighted average bin gap between accuracy and confidence |
| `bins` | Confidence-bin summaries |
| `regimes` | Optional per-regime summaries |

The ECE calculation uses populated bins that satisfy
`min_samples_per_bin`. Empty or under-sampled bins do not contribute to the
weighted error, which avoids making sparse bins look more certain than they are.

### Replay-To-Calibration Example

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
```

### Calibrator Example

```rust
use of_signals::{
    SignalCalibrationCurve, SignalCalibrationPoint, SignalConfidenceCalibrator,
};

let curve = SignalCalibrationCurve::new(vec![
    SignalCalibrationPoint::new(0, 0),
    SignalCalibrationPoint::new(5_000, 4_000),
    SignalCalibrationPoint::new(10_000, 9_000),
]);

assert_eq!(curve.calibrate_confidence_bps(7_500), 6_500);
```

Use the tracker for online or batch monitoring when the host has already
resolved realized outcomes:

```rust
use of_signals::{
    SignalCalibrationConfig, SignalMarkoutDirection, SignalOutcomeRecord,
    SignalOutcomeTracker,
};

let mut tracker = SignalOutcomeTracker::new(SignalCalibrationConfig::default());
tracker.record(
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

let report = tracker.calibration_report();
assert_eq!(report.scored_records, 1);
assert_eq!(report.regimes[0].regime, "trend");
```

Operational guidance:

- retain validation samples with `with_store_samples(true)` when calibration
  analysis is required;
- set `min_samples_per_bin` high enough for the deployment venue and symbol
  before trusting ECE in production;
- compare current reports against a baseline with
  `SignalCalibrationDriftReport::compare` or
  `SignalOutcomeTracker::drift_report`;
- use per-regime summaries when market regime, session, or symbol behavior
  changes signal reliability.

### `SignalInputMask`

`SignalInputMask` is a compact bitset for required signal context.

| Constant | Meaning |
| --- | --- |
| `NONE` | No declared inputs |
| `ANALYTICS` | Requires `AnalyticsSnapshot` |
| `DATA_QUALITY` | Requires `DataQualityFlags` |
| `BOOK` | Requires reconstructed book state |
| `ADVANCED_ANALYTICS` | Requires advanced analytics or feature vectors |
| `MARKET_REGIME` | Requires market-regime context |
| `POSITION` | Requires current position context |
| `RISK` | Requires risk or OMS gating context |

Use `contains`, `intersects`, `union`, `bits`, or `from_bits_truncate` when
bridging this metadata into config files or bindings.

### `SignalLifecycle`

`SignalLifecycle` tracks warmup progress and explicit production states without
changing signal logic.

| State | Meaning |
| --- | --- |
| `Initializing` | Object exists but is not evaluating yet |
| `WarmingUp` | Inputs are flowing but warmup is incomplete |
| `Active` | Output can be consumed normally |
| `Degraded` | Output exists but should be treated cautiously |
| `Blocked` | Output must not be used for trading decisions |
| `CoolingDown` | Rapid transitions are intentionally suppressed |
| `Disabled` | Evaluation is configured off |

Warmup requirements can be based on event count, market time, completed bars, or
an `All(...)` composite requirement.

Example:

```rust
use of_signals::{SignalLifecycle, SignalLifecycleState, SignalWarmupRequirement};

let mut lifecycle = SignalLifecycle::new(SignalWarmupRequirement::Events(2));
assert_eq!(lifecycle.state(), SignalLifecycleState::WarmingUp);

lifecycle.record_event();
lifecycle.record_event();

assert_eq!(lifecycle.state(), SignalLifecycleState::Active);
```

## Built-in Modules

### `DeltaMomentumSignal`

Constructor:

| Method | Parameters | Meaning |
| --- | --- | --- |
| `new(threshold)` | `threshold: i64` | Absolute `delta` threshold |

Behavior:

- emits `LongBias` when `delta >= threshold`
- emits `ShortBias` when `delta <= -threshold`
- emits `Neutral` otherwise

### `VolumeImbalanceSignal`

Constructor:

| Method | Parameters | Meaning |
| --- | --- | --- |
| `new(threshold)` | `threshold: i64` | Absolute `buy_volume - sell_volume` threshold |

Behavior:

- evaluates session `buy_volume` versus `sell_volume`
- emits directional bias only when the absolute imbalance exceeds the threshold

### `CumulativeDeltaSignal`

Constructor:

| Method | Parameters | Meaning |
| --- | --- | --- |
| `new(threshold)` | `threshold: i64` | Absolute `cumulative_delta` threshold |

Behavior:

- uses longer-running directional accumulation rather than just current delta

### `AbsorptionSignal`

Constructor:

| Method | Parameters | Meaning |
| --- | --- | --- |
| `new(threshold, price_band)` | `threshold: i64`, `price_band: i64` | Pressure threshold and max distance from POC |

Behavior:

- looks for strong directional flow that fails to displace price away from the
  key traded area
- can emit `LongBias` on sell absorption or `ShortBias` on buy absorption

### `ExhaustionSignal`

Constructor:

| Method | Parameters | Meaning |
| --- | --- | --- |
| `new(threshold)` | `threshold: i64` | Absolute delta threshold for exhaustion detection |

Behavior:

- looks for strong directional flow that stalls instead of continuing

### `SweepDetectionSignal`

Constructor:

| Method | Parameters | Meaning |
| --- | --- | --- |
| `new(threshold, breakout_ticks)` | `threshold: i64`, `breakout_ticks: i64` | Directional threshold and breakout distance |

Behavior:

- combines directional pressure with breaks outside value area

### `CompositeSignal`

Constructor:

| Method | Parameters | Meaning |
| --- | --- | --- |
| `new(modules)` | `modules: Vec<Box<dyn SignalModule>>` | Owned child modules |

Behavior:

- updates each child with the same analytics input
- emits the majority directional view
- returns `Neutral` when no side has a majority

## Output Contract

All built-in modules return the shared `of_core::SignalSnapshot` contract:

| Field | Meaning |
| --- | --- |
| `state` | Stable directional state |
| `confidence` | Module-defined confidence score |
| `reason` | Human-readable rationale |
| `quality_flags` | Quality flags associated with the decision |

## Default Implementations

The built-in modules also implement `Default` where sensible. Those defaults are
intended as practical examples and runtime-ready baselines, not as universal
trading recommendations.

## When To Use `of_signals`

- Use it directly when you want deterministic, runtime-compatible signal logic.
- Implement `SignalModule` when writing your own strategy layer for the runtime.
- Use `of_runtime` when you want orchestration, adapter polling, persistence,
  book reconstruction, and health handling around signals.
