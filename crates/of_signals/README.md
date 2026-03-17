# of_signals

`of_signals` contains strategy modules that transform analytics snapshots into stable directional state.
It is intentionally separated from ingestion/runtime plumbing so strategy logic remains easy to test and evolve.

## Core API

- Trait: [`SignalModule`]
- Gate result: [`SignalGateDecision`]
- Built-in module: [`DeltaMomentumSignal`]

Signal output uses `of_core::SignalSnapshot` and states such as `LongBias`, `ShortBias`, `Neutral`, and `Blocked`.

## Delta Momentum Strategy

[`DeltaMomentumSignal`] is a reference implementation that:

- emits `LongBias` when `delta >= threshold`
- emits `ShortBias` when `delta <= -threshold`
- emits `Neutral` otherwise
- emits `Blocked` in runtime when quality gate fails

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
