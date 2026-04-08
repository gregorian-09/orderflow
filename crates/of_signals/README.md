# of_signals

`of_signals` contains strategy modules that transform analytics snapshots into stable directional state.
It is intentionally separated from ingestion/runtime plumbing so strategy logic remains easy to test and evolve.

## Core API

- Trait: [`SignalModule`]
- Gate result: [`SignalGateDecision`]
- Built-in modules:
  - [`DeltaMomentumSignal`]
  - [`VolumeImbalanceSignal`]
  - [`CumulativeDeltaSignal`]
  - [`AbsorptionSignal`]
  - [`ExhaustionSignal`]
  - [`SweepDetectionSignal`]
  - [`CompositeSignal`]

## Public API Inventory

Public types:

- [`SignalGateDecision`]
- [`SignalModule`]
- [`DeltaMomentumSignal`]
- [`VolumeImbalanceSignal`]
- [`CumulativeDeltaSignal`]
- [`AbsorptionSignal`]
- [`ExhaustionSignal`]
- [`SweepDetectionSignal`]
- [`CompositeSignal`]

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
