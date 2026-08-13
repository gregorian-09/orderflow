# Signals and Strategy Evaluation

`of_signals` converts analytics snapshots into directional state under explicit
quality, lifecycle, confidence, and gating rules. It is strategy infrastructure
and does not submit orders or own venue connectivity.

## Signal Pipeline

```mermaid
flowchart LR
    Snapshot[Analytics snapshot] --> Inputs[Required inputs and quality]
    Inputs --> Module[SignalModule]
    Module --> Raw[Directional decision]
    Raw --> Gate[Quality and policy gate]
    Gate --> Stabilize[Lifecycle, debounce, hysteresis]
    Stabilize --> Output[SignalSnapshot and explanation]
    Output --> Host[Host-owned order intent]
```

## Core Contract

A signal module should be deterministic for the same snapshot sequence and
configuration. It must identify whether it is warming up, disabled, blocked by
quality, or producing a directional result. `Unknown`/blocked output is a valid
safety result, not an error to be coerced into `Flat` or a trade direction.

## Built-in Modules

The built-in modules cover delta momentum, volume imbalance, cumulative delta,
absorption, exhaustion, sweep detection, and composite voting. Their thresholds
and input requirements belong in descriptors so applications can discover and
validate configuration instead of guessing parameter names.

## Quality Gating

Quality gating can block a signal when the feed is stale, has sequence gaps,
contains out-of-order events, has clock skew, has truncated depth, or is
otherwise degraded. The gate decision should preserve the underlying reason and
the observed quality flags for audit and operator explanation.

## Lifecycle and Stabilization

Signal lifecycle handles warmup requirements, activation, disablement, and
reset. Stabilization handles debounce, hysteresis, cooldown, and transition-only
emission. These policies change when a result is eligible for host action; they
must not rewrite the underlying analytics snapshot.

## Explainability and Validation

Descriptors expose name, version, required inputs, configuration metadata, and
availability. Explanations expose reason codes and decision context. Replay
validation evaluates ordered historical snapshots and markout outcomes without
mutating live engine state. Calibration, ensembles, shadow recording, and model
metadata must preserve schema identity and quality context.

## Execution Boundary

A signal output is not an order. The host must translate it into an order intent,
apply position/risk/quality policy, and submit it through the execution plane.
This separation prevents a signal module from bypassing idempotency, risk,
route health, or reconciliation controls.

## References

- [Signals crate reference](../handbook/05c-of-signals-reference.md)
- [Strategy cookbook](../handbook/08-strategy-cookbook.md)
- [Core quality model](../foundations/README.md)
- [OMS execution boundary](../execution/README.md)
