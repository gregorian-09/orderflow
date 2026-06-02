# Orderflow Engine and Bindings

Orderflow is a multi-language market data and analytics engine that computes
**29 categories of microstructural analytics** across Rust, C, Python, and
Java — from spread metrics and VPIN toxicity through fingerprint patterns,
volatility signatures, Almgren-Chriss impact models, options flow, futures
basis, dark pool siphon detection, and machine-learning-ready LOB features.

The current development line also includes an additive execution-core
foundation for developer-built order-management workflows. Execution APIs use
separate handles and crates, so existing analytics integrations remain stable.
The execution layer provides typed order requests, FIX-style state transitions,
structured risk rejection, simulated execution, C/Python/Java bindings, and a
FIX mapping scaffold. It is not a broker-certified production OMS by itself.

## Documentation

Start here:

- **[Strategy Cookbook](./docs/handbook/08-strategy-cookbook.md)** — 30
  exhaustive strategy examples covering every analytics concept across all
  four API layers (Rust, C, Python, Java), plus a full multi-concept live
  trading loop, an API compatibility map, and the `AnalyticsConfig` tuning
  guide.
- [Handbook Home](./docs/handbook/README.md) — primer, architecture, API reference.
- [docs/README.md](./docs/README.md) — full navigation.
- [docs/bindings/README.md](./docs/bindings/README.md) — Python (ctypes) and Java (JNA) setup.

## Analytics at a Glance

| Tier | Concepts | Exposed |
|------|----------|---------|
| T0   | Spread, depth, book events, resiliency | Rust, C, Python, Java |
| T1   | Trade classification, VPIN, Kyle's λ, Amihud, CVD | Rust, C, Python, Java |
| T2   | 19 pattern flags (footprint, DOM, delta, session, volume profile) | Rust, C, Python, Java |
| T3   | Volatility (Parkinson/GK/YZ, signature), noise, Hasbrouck, Almgren-Chriss, spread decomp, ACD, regime | Rust, C, Python, Java |
| T4   | Kinetic energy, agent-type ID, LOB features (16 fields) | Rust, C, Python, Java |
| T5   | Dark pool, dark-lit correlation, institutional flow | Rust, C, Python, Java |
| T6   | Options flow, OI analysis | Rust, C, Python, Java |
| T7   | Futures basis, calendar spread, settlement | Rust, C, Python, Java |

## Execution Core at a Glance

| Layer | Additive API |
|------|--------------|
| Rust core | `of_execution_core` order IDs, requests, events, state machine, risk |
| Rust engine | `of_execution` adapter trait, bounded event buffer, simulator, journal hooks |
| Adapter scaffold | `of_execution_adapters::fix` execution-report mapper and FIX capabilities |
| C ABI | `of_execution_engine_t`, submit/cancel/amend/poll/state/health/metrics |
| Python | `ExecutionEngine`, `OrderRequest`, `CancelRequest`, `AmendRequest` |
| Java | `OrderflowExecutionEngine`, `OrderRequest`, `CancelRequest`, `AmendRequest` |

Low-latency-sensitive paths use typed structs and caller-owned event buffers,
not JSON payloads. JSON remains for analytics snapshots and diagnostics.

Every analytics type is configurable via the 22-field `AnalyticsConfig` struct
and queryable through the same buffer-negotiation C ABI pattern.

## Quick Build

```bash
cargo build --all-features
cargo test --all-features
```

Build C ABI for bindings, then test FFI exports:

```bash
cargo build -p of_ffi_c --features tickbar
tools/check_ffi_exports.sh
```

## Bindings Quickstart

### Python
```python
from orderflow import Engine, EngineConfig, Symbol

with Engine(EngineConfig()) as engine:
    engine.start()
    engine.subscribe(Symbol("CME", "ESM6", 10))
    engine.poll_once()

    # Read any of the 29 analytics
    print(engine.analytics_snapshot(Symbol("CME", "ESM6", 10)))
    print(engine.pattern_snapshot(Symbol("CME", "ESM6", 10)))
    print(engine.lob_features(Symbol("CME", "ESM6", 10), 0.0, 0.0, 0.0))
```

Execution simulation:

```python
from orderflow import (
    ConcurrentExecutionEngine, ExecutionEngine, ExecutionOrderType, ExecutionSide, ExecutionTimeInForce,
    OrderRequest, RiskLimits, RouteConfig,
)

limits = RiskLimits(False, 100, 1_000_000, 10, 10_000_000, 0)
routes = [
    RouteConfig("SIM", "ACC", "SIM", "ES", True, limits),
    RouteConfig("SIM", "ACC", "SIM", "NQ", True, limits),
]

with ExecutionEngine(routes) as execution:
    events = execution.submit_order(OrderRequest(
        "C1", "ACC", "SIM", "STRAT", "SIM", "ES",
        ExecutionSide.BUY, ExecutionOrderType.LIMIT, ExecutionTimeInForce.DAY,
        10, 5000,
    ))
    print(events[-1].order_status)

with ConcurrentExecutionEngine(routes) as execution:
    sequence = execution.submit_order(OrderRequest(
        "C2", "ACC", "SIM", "STRAT", "SIM", "NQ",
        ExecutionSide.BUY, ExecutionOrderType.LIMIT, ExecutionTimeInForce.DAY,
        10, 17000,
    ))
    report = execution.try_recv_report()
```

### Java
```java
try (OrderflowEngine engine = new OrderflowEngine()) {
    engine.start();
    Symbol sym = new Symbol("CME", "ESM6", (short) 10);
    engine.subscribe(sym, StreamKind.ANALYTICS);

    System.out.println(engine.analyticsSnapshot(sym));
    System.out.println(engine.volatilitySnapshot(sym));
}
```

Execution simulation:

```java
RiskLimits limits = new RiskLimits(false, 100, 1_000_000, 10, 10_000_000, 0);
List<RouteConfig> routes = List.of(
    new RouteConfig("SIM", "ACC", "SIM", "ES", true, limits),
    new RouteConfig("SIM", "ACC", "SIM", "NQ", true, limits)
);

try (OrderflowExecutionEngine execution = new OrderflowExecutionEngine(null, routes)) {
    execution.start();
    execution.submitOrder(new OrderRequest(
        "C1", "ACC", "SIM", "STRAT", "SIM", "ES",
        ExecutionSide.BUY, ExecutionOrderType.LIMIT, ExecutionTimeInForce.DAY,
        10, 5000, 0, 1, 2
    ));
}

try (ConcurrentOrderflowExecutionEngine execution =
         new ConcurrentOrderflowExecutionEngine(null, routes)) {
    long sequence = execution.submitOrder(new OrderRequest(
        "C2", "ACC", "SIM", "STRAT", "SIM", "NQ",
        ExecutionSide.BUY, ExecutionOrderType.LIMIT, ExecutionTimeInForce.DAY,
        10, 17000, 0, 1, 3
    ));
    Optional<ExecutionCommandReport> report = execution.tryRecvReport();
}
```

## Tooling

Replay utility:

```bash
cargo run -p replay_cli -- data              # list venues
cargo run -p replay_cli -- data CME          # list symbols
cargo run -p replay_cli -- data CME ESM6 100 200  # replay range
```
