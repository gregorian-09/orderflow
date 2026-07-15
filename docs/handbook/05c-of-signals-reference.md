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
| `SignalInputMask` | struct | Bitset of inputs required by a signal |
| `SignalLifecycleState` | enum | Production lifecycle state |
| `SignalWarmupProgress` | struct | Counters used to evaluate warmup |
| `SignalWarmupRequirement` | enum | Warmup policy for a signal |
| `SignalLifecycle` | struct | Small lifecycle/warmup helper |
| `SignalDescriptor` | struct | Static metadata for signal discovery |
| `SignalParameterDescriptor` | struct | Static metadata for one parameter |
| `SignalOutputSemantics` | enum | How consumers should interpret output |
| `built_in_signal_descriptors` | function | Returns all built-in signal descriptors |
| `describe_signal` | function | Looks up one built-in descriptor by id |
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
