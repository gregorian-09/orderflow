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
