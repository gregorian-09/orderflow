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
