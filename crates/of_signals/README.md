# of_signals

`of_signals` contains strategy modules that transform analytics snapshots into stable directional state.
It is intentionally separated from ingestion/runtime plumbing so strategy logic remains easy to test and evolve.

## Core API

- Trait: [`SignalModule`]
- Gate result: [`SignalGateDecision`]
- Descriptor inventory: [`built_in_signal_descriptors`] and [`describe_signal`]
- Lifecycle helper: [`SignalLifecycle`]
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

## Unreleased Additive Metadata And Lifecycle APIs

The crate now exposes production-oriented signal metadata and lifecycle helpers
without changing the existing [`SignalModule`] contract.

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
- [`SignalInputMask`]
- [`SignalLifecycleState`]
- [`SignalWarmupProgress`]
- [`SignalWarmupRequirement`]
- [`SignalLifecycle`]
- [`SignalOutputSemantics`]
- [`SignalParameterKind`]
- [`SignalParameterValue`]
- [`SignalParameterDescriptor`]
- [`SignalDescriptor`]
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
- [`describe_signal`]

Public constructors:

- [`DeltaMomentumSignal::new`]
- [`VolumeImbalanceSignal::new`]
- [`CumulativeDeltaSignal::new`]
- [`AbsorptionSignal::new`]
- [`ExhaustionSignal::new`]
- [`SweepDetectionSignal::new`]
- [`CompositeSignal::new`]

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
