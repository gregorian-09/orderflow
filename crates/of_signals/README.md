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
