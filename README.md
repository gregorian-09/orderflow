# Orderflow Engine and Bindings

Orderflow is a multi-language market data and analytics engine that computes
**29 categories of microstructural analytics** across Rust, C, Python, and
Java — from spread metrics and VPIN toxicity through fingerprint patterns,
volatility signatures, Almgren-Chriss impact models, options flow, futures
basis, dark pool siphon detection, and machine-learning-ready LOB features.

Version `0.4.0` adds an execution and OMS foundation for developer-built order
management workflows. Execution APIs use separate Rust crates, C handles,
Python classes, and Java classes, so existing analytics integrations remain
stable. The execution layer provides typed order requests, FIX-style state
transitions, structured risk rejection, simulated execution, bounded
concurrent command workers, journals, recovery helpers, C/Python/Java bindings,
a reusable low-allocation FIX codec foundation, a FIX mapping scaffold, and a
separate parent/child execution-algorithm crate. It is not a broker-certified
production OMS by itself; it is the reusable foundation developers use to build
one.

## What's New In 0.4.0

`0.4.0` is a non-breaking expansion release. The mature analytics/runtime/C
ABI/binding packages move from `0.3.0` to `0.4.0`; the new execution crates are
published as `0.1.0` because they are new public crate surfaces with their own
stabilization path.

Package versions for this release:

| Package family | Version | Why |
| --- | ---: | --- |
| Existing Rust crates: `of_core`, `of_adapters`, `of_signals`, `of_persist`, `of_runtime`, `of_ffi_c` | `0.4.0` | Same established API line, additive analytics/execution exposure |
| Python binding: `orderflow-gregorian09` | `0.4.0` | Matches the native `of_ffi_c` ABI package line |
| Java binding: `orderflow-java-binding` | `0.4.0` | Matches the native `of_ffi_c` ABI package line |
| New Rust crates: `of_analytics`, `of_execution_core`, `of_fix`, `of_execution`, `of_execution_adapters`, `of_execution_algos` | `0.1.0` | New advanced analytics/execution/FIX/algo crate family, intentionally versioned from its own first public release |

Major additions:

- execution-domain primitives: fixed-size identifiers, typed submit/cancel/amend
  requests, execution reports, strict order-state machine, and basic risk gates
- execution engine: route/account/symbol routing, simulated adapter, bounded
  event buffers, route-scoped risk accounting, journaling, metrics, and health
- concurrent OMS worker: bounded command/report queues for many producers while
  one native worker owns order state deterministically
- OMS helpers: command correlation, event fanout, lifecycle snapshots, durable
  file/WAL journals, checkpoint recovery, native order-intent and parent/child
  lifecycle, scoped kill switches, production risk, exact position/PnL ledger,
  command idempotency, execution-report deduplication, reconciliation, safety
  policies, generalized multi-source recovery reconciliation, a deterministic
  18-scenario adapter/OMS certification venue, fixed-memory percentile
  execution SLIs and explicit SLO evaluation, permission-ready idempotent
  operator commands with durable intent/outcome audit, bounded production
  incident bundles with SHA-256 inventory and atomic publication, allocation,
  telemetry, sharding, throttling, and replay simulation
- FIX foundation: borrowed tag-value codec, `BodyLength(9)` and `CheckSum(10)`
  validation, common tag extraction, typed `FixVersion`/`FixMsgType` helpers,
  static dictionary/profile validation, deterministic session sequence
  primitives, typed session/admin and order-entry builders, caller-owned encode
  buffers, and debug rendering outside the hot path
- adapter scaffolding: FIX execution-report mapping and fail-closed adapter shell
  for provider adapter authors
- execution algorithms: additive parent/child order primitives, fixed-capacity
  decision buffers, OMS execution-event progress folding, and deterministic
  TWAP slice planning in `of_execution_algos`
- advanced analytics split: new `of_analytics` crate with dependency-light
  market-quality/TCA and liquidity/depth primitives plus feature profiles for
  future impact, toxicity, volatility, regime, pattern, derivatives,
  institutional, and ML-feature modules
- bindings: additive C ABI, Python, and Java execution APIs that do not change
  existing analytics APIs
- documentation: end-to-end strategy workflow, OMS architecture, OMS cookbook,
  low-latency design, provider adapter authoring, recovery operations, and
  exhaustive binding READMEs

Upgrade rule: existing analytics users can upgrade without renaming existing
APIs. Execution adopters should treat the new execution crates as a `0.1.0`
surface and pin them explicitly if building provider adapters against their
traits.

## Documentation

Start here:

- **[Strategy Cookbook](./docs/handbook/08-strategy-cookbook.md)** — 30
  exhaustive strategy examples covering every analytics concept across all
  four API layers (Rust, C, Python, Java), plus a full multi-concept live
  trading loop, an API compatibility map, and the `AnalyticsConfig` tuning
  guide.
- **[Building an Orderflow Strategy](./docs/handbook/02-strategy-design.md)** —
  complete idea-to-execution workflow: ingest, analytics, signal gating, risk,
  simulated OMS execution, reports, journal, replay, and promotion checklist.
- **[OMS Cookbook](./docs/handbook/10-oms-cookbook.md)** — multi-symbol routes,
  synchronous and concurrent execution, risk rejection, recovery, reconciliation,
  throttling, replay, and C/Python/Java worker examples.
- [Handbook Home](./docs/handbook/README.md) — primer, architecture, API reference.
- [docs/README.md](./docs/README.md) — full navigation.
- [docs/bindings/README.md](./docs/bindings/README.md) — Python (ctypes) and Java (JNA) setup.

## Analytics at a Glance

| Category | Concepts | Exposed |
|----------|----------|---------|
| Book and spread analytics | Spread, depth, book events, resiliency | Rust, C, Python, Java |
| Trade-flow analytics | Trade classification, VPIN, Kyle's lambda, Amihud, CVD | Rust, C, Python, Java |
| Pattern detection | 19 pattern flags across footprint, DOM, delta, session, and volume profile context | Rust, C, Python, Java |
| Volatility and impact | Parkinson/Garman-Klass/Yang-Zhang volatility, signature plots, noise, Hasbrouck, Almgren-Chriss, spread decomposition, ACD, regime | Rust, C, Python, Java |
| Advanced market structure | Kinetic energy, agent-type ID, LOB features | Rust, C, Python, Java |
| Institutional flow | Dark pool, dark-lit correlation, institutional flow | Rust, C, Python, Java |
| Derivatives analytics | Options flow, open-interest analysis, futures basis, calendar spread, settlement | Rust, C, Python, Java |

## Execution Core at a Glance

| Layer | Additive API |
|------|--------------|
| Rust core | `of_execution_core` order IDs, requests, events, state machine, risk |
| FIX codec | `of_fix` borrowed tag-value parser, validator, profile rules, and encoder |
| Rust engine | `of_execution` adapter trait, bounded event buffer, simulator, journal/recovery, operator controls, and verifiable incident bundles |
| Adapter scaffold | `of_execution_adapters::fix` execution-report mapper and FIX capabilities |
| Execution algos | `of_execution_algos` parent/child substrate and deterministic TWAP planner |
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
python3 tools/check_api_manifest.py
python3 tools/check_binding_parity.py
python3 tools/generate_api_inventory.py --check
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
