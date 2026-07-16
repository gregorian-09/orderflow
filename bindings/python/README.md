# Orderflow Python Binding (`orderflow-gregorian09`)

[![PyPI version](https://img.shields.io/pypi/v/orderflow-gregorian09.svg)](https://pypi.org/project/orderflow-gregorian09/)
[![Python versions](https://img.shields.io/pypi/pyversions/orderflow-gregorian09.svg)](https://pypi.org/project/orderflow-gregorian09/)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](https://opensource.org/license/mit)

Production-focused Python API for the Orderflow runtime.  
This package wraps the stable `of_ffi_c` ABI via `ctypes` and provides a typed,
high-level interface for lifecycle management, subscriptions, snapshots, and
external feed ingestion.
The package includes a PEP 561 `py.typed` marker so type checkers can consume
the inline annotations shipped with the binding.

The binding also exposes an additive execution API through `ExecutionEngine`.
Execution uses a separate native handle from analytics and returns typed
execution events rather than JSON on the order path.

The README is intentionally API-complete so the PyPI page can be used as a
single reference, similar to high-signal package pages such as TA-Lib and
FastAPI.

## What's New In 0.4.0

`0.4.0` is the first Python release with end-to-end analytics plus execution
concepts in one binding package. Existing `Engine` users keep the same market
data API; execution is exposed through separate classes and native handles.

Highlights:

- additive simulated execution APIs: `ExecutionEngine`,
  `ConcurrentExecutionEngine`, `OrderRequest`, `CancelRequest`, `AmendRequest`,
  `RiskLimits`, `RouteConfig`, and execution event dataclasses
- multi-route execution construction for multi-symbol order flow
- bounded concurrent execution worker for many producers and one deterministic
  native order-state owner
- typed execution events and command reports instead of JSON on the order path
- route/account/symbol-scoped risk checks before adapter routing
- offline WAL and checkpoint-store diagnostics for recovery checks without
  opening an execution engine
- adapter inventory/status helpers for provider capability discovery before
  connecting a feed
- signal descriptor discovery through `signal_descriptors()` and
  `Engine.signal_descriptors()` for dashboard/configuration inventory
- signal explanation discovery through `Engine.signal_explanation(symbol)` for
  audit and dashboard diagnostics
- signal metrics through `Engine.signal_metrics()` for state counts,
  confidence, quality, and explanation coverage diagnostics
- native C ABI inventory validation through `bindings/api_manifest.toml` so
  future low-level `ctypes` declarations can be checked against `orderflow.h`
- analytics-to-execution examples in this README and the handbook
- continued PEP 561 `py.typed` support and bundled native library lookup

Version policy:

- Python package: `0.4.0`
- compatible native `of_ffi_c` library/header: `0.4.0`
- new Rust execution crates behind the native ABI: `0.1.0`

Install the Python package and native runtime at the same release version.
Execution users should treat the Rust execution crates as a new `0.1.x`
surface if they build custom native providers.

### Execution quick start

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
```

`ExecutionEngine(route)` remains supported for single-symbol integrations. When
you pass a route list, native risk accounting is scoped per
route/account/symbol.

Use `ConcurrentExecutionEngine(routes)` when multiple producer threads need to
queue commands into one deterministic native worker. Command methods return a
sequence number; `try_recv_report()` returns completed command reports without
blocking.

### Binding manifest policy

Low-level native symbols are tracked in `bindings/api_manifest.toml`. The
manifest validates the stable C ABI boundary and powers export checks; it does
not replace the hand-written `Engine`, `ExecutionEngine`, and
`ConcurrentExecutionEngine` wrappers. New public Python conveniences should
remain ergonomic and typed, while repetitive native declarations can move toward
manifest-backed generation.

### Signal descriptor discovery

```python
from orderflow import signal_descriptors

inventory = signal_descriptors()
for descriptor in inventory.get("signals", []):
    print(descriptor["id"], descriptor["required_inputs"], descriptor["output_semantics"])
```

The descriptor inventory is read-only metadata. It helps dashboards and config
tools list built-in signals, required inputs, warmup, parameters, and output
semantics without constructing a live strategy or submitting orders.

After a signal has evaluated for a symbol, `Engine.signal_explanation(symbol)`
returns the latest explanation payload with reason code, observed inputs,
thresholds, and confidence contributors. This is a diagnostics surface; order
submission decisions should still flow through explicit strategy/risk/OMS code.

`Engine.signal_metrics()` returns a compact runtime summary of the current
signal cache: state counts, directional count, average confidence, quality
flagged signals, and explanation coverage.

## Architecture

![Orderflow architecture](https://raw.githubusercontent.com/gregorian-09/orderflow/main/docs/handbook/assets/diagrams/png/04-architecture-01.png)

## Installation

```bash
pip install orderflow-gregorian09
```

### Python support

- Python 3.10+

### Native runtime requirement

The Python package is a wrapper. A compatible `libof_ffi_c` shared library must
be available at runtime. Binary wheels can bundle this library under
`orderflow/native/`; source installs can still use an externally-built runtime.

Library resolution order:

1. `library_path=` passed to `Engine(...)`
2. `ORDERFLOW_LIBRARY_PATH` environment variable
3. bundled wheel library (`orderflow/native/libof_ffi_c.*`)
4. default local debug path (`target/debug/libof_ffi_c.*`)

```bash
export ORDERFLOW_LIBRARY_PATH=/absolute/path/to/libof_ffi_c.so
```

## Quick Start

```python
from orderflow import DataQualityFlags, Engine, EngineConfig, Symbol, StreamKind

with Engine(EngineConfig(instance_id="py-client")) as eng:
    sym = Symbol("CME", "ESM6", depth_levels=10)
    eng.subscribe(sym, StreamKind.ANALYTICS)
    eng.poll_once(DataQualityFlags.NONE)
    print("api_version", eng.api_version)
    print("build_info", eng.build_info)
    print("analytics", eng.analytics_snapshot(sym))
    print("derived", eng.derived_analytics_snapshot(sym))
    print("signal", eng.signal_snapshot(sym))
    print("metrics", eng.metrics())
```

## Complete End-To-End Example

This example uses deterministic external ingest and simulated execution. It is
safe for documentation, CI smoke tests, and first user experiments because it
does not connect to a broker.

```python
from orderflow import (
    BookAction,
    DataQualityFlags,
    Engine,
    EngineConfig,
    ExecutionEngine,
    ExecutionOrderType,
    ExecutionSide,
    ExecutionTimeInForce,
    ExternalFeedPolicy,
    OrderRequest,
    RiskLimits,
    RouteConfig,
    Side,
    StreamKind,
    Symbol,
)


def signal_allows_long(analytics: dict, signal: dict) -> bool:
    quality = int(analytics.get("quality_flags", 0))
    delta = int(analytics.get("delta", 0))
    cumulative_delta = float(analytics.get("cumulative_delta", 0.0))
    confidence = float(signal.get("confidence", 0.0))
    return (
        quality == DataQualityFlags.NONE
        and delta > 0
        and cumulative_delta > 0.0
        and confidence >= 0.50
    )


sym = Symbol("SIM", "ES", 10)
limits = RiskLimits(False, 5, 1_000_000, 1, 1_000_000, 0)
routes = [RouteConfig("SIM", "ACC", "SIM", "ES", True, limits)]

with Engine(EngineConfig(instance_id="py-end-to-end")) as market, ExecutionEngine(routes) as execution:
    market.configure_external_feed(ExternalFeedPolicy(2_000, True))
    market.subscribe(sym, StreamKind.ANALYTICS)
    market.subscribe(sym, StreamKind.SIGNALS)

    market.ingest_book(sym, Side.BID, 0, 500_000, 100, BookAction.UPSERT, sequence=1)
    market.ingest_book(sym, Side.ASK, 0, 500_025, 120, BookAction.UPSERT, sequence=2)
    market.ingest_trade(sym, 500_025, 2, Side.ASK, sequence=3)
    market.poll_once(DataQualityFlags.NONE)

    analytics = market.analytics_snapshot(sym)
    signal = market.signal_snapshot(sym)

    if signal_allows_long(analytics, signal):
        events = execution.submit_order(OrderRequest(
            "PY-0001",
            "ACC",
            "SIM",
            "DOCS",
            "SIM",
            "ES",
            ExecutionSide.BUY,
            ExecutionOrderType.LIMIT,
            ExecutionTimeInForce.DAY,
            1,
            500_025,
        ))
        print("events", events)
        print("state", execution.order_state("PY-0001"))
        print("execution metrics", execution.execution_metrics())
    else:
        print("blocked", {"analytics": analytics, "signal": signal})
```

Production applications add durable persistence, execution journaling, provider
adapter ownership, reconnect/recovery policy, and monitoring around this shape.

## Public API Reference

### Constants

#### `StreamKind`

| Name | Value | Meaning |
|---|---:|---|
| `BOOK` | 1 | Level-2 book update stream |
| `TRADES` | 2 | Trade print stream |
| `ANALYTICS` | 3 | Analytics snapshot stream |
| `SIGNALS` | 4 | Signal snapshot stream |
| `HEALTH` | 5 | Health transition stream |
| `BOOK_SNAPSHOT` | 6 | Materialized book snapshot stream after book changes |
| `DERIVED_ANALYTICS` | 7 | Derived analytics stream after trade-driven analytics changes |

#### `Side`

| Name | Value | Meaning |
|---|---:|---|
| `BID` | 0 | Bid / buy side |
| `ASK` | 1 | Ask / sell side |

#### `BookAction`

| Name | Value | Meaning |
|---|---:|---|
| `UPSERT` | 0 | Insert or update price level |
| `DELETE` | 1 | Delete price level |

#### `DataQualityFlags`

| Name | Value | Meaning |
|---|---:|---|
| `NONE` | `0` | No quality issues |
| `STALE_FEED` | `1 << 0` | Feed became stale |
| `SEQUENCE_GAP` | `1 << 1` | Sequence gap detected |
| `CLOCK_SKEW` | `1 << 2` | Clock skew detected |
| `DEPTH_TRUNCATED` | `1 << 3` | Depth truncation occurred |
| `OUT_OF_ORDER` | `1 << 4` | Out-of-order sequence detected |
| `ADAPTER_DEGRADED` | `1 << 5` | Adapter/feed degraded |

### Exceptions

| Exception | Purpose |
|---|---|
| `OrderflowError` | Base binding/runtime failure |
| `OrderflowStateError` | Invalid lifecycle/state transition |
| `OrderflowArgError` | Invalid argument passed to native API |

### Data Classes

#### `Symbol(venue: str, symbol: str, depth_levels: int = 10)`

- venue/instrument descriptor used by subscribe/snapshot/ingest APIs.

#### `EngineConfig(...)`

`EngineConfig` fields:

| Field | Type | Default | Notes |
|---|---|---|---|
| `instance_id` | `str` | `"python"` | Runtime instance id |
| `config_path` | `str` | `""` | Optional runtime config file path |
| `log_level` | `int` | `0` | Reserved log-level field |
| `enable_persistence` | `bool` | `False` | Enable local persistence |
| `audit_max_bytes` | `int` | `10*1024*1024` | Per-file audit size before rotation |
| `audit_max_files` | `int` | `5` | Number of rotated audit files |
| `audit_redact_tokens_csv` | `str` | `"secret,password,token,api_key"` | Redaction tokens |
| `data_retention_max_bytes` | `int` | `10*1024*1024` | Persistence retention limit |
| `data_retention_max_age_secs` | `int` | `7*24*60*60` | Max retention age |

#### `ExternalFeedPolicy(stale_after_ms: int = 15000, enforce_sequence: bool = True)`

- external ingest supervision policy for stale and sequence validation.

### `Engine` API

#### Constructor and properties

| Signature | Description |
|---|---|
| `Engine(config: EngineConfig, library_path: Optional[str] = None)` | Creates native engine handle |
| `engine.api_version -> int` | Returns native ABI version |
| `engine.build_info -> str` | Returns native build descriptor |

#### Lifecycle and session

| Signature | Description |
|---|---|
| `start() -> None` | Starts runtime |
| `stop() -> None` | Stops runtime |
| `close() -> None` | Unsubscribes and destroys native handle |
| context-manager (`with Engine(...)`) | Calls `start()` / `close()` automatically |

#### Subscription and polling

| Signature | Description |
|---|---|
| `subscribe(symbol, stream_kind=StreamKind.ANALYTICS, callback=None)` | Registers stream subscription with optional callback |
| `unsubscribe(symbol)` | Unsubscribes all streams for symbol |
| `poll_once(quality_flags=DataQualityFlags.NONE)` | Drains adapter/runtime once |
| `reset_symbol_session(symbol)` | Resets per-symbol session/profile state |

#### Adapter discovery

| Signature | Description |
|---|---|
| `adapter_inventory(library_path=None)` | Returns native build adapter descriptor inventory |
| `available_adapters(library_path=None)` | Returns the inventory `adapters` list |
| `engine.adapter_inventory()` | Returns adapter inventory with this engine's active provider marked |
| `engine.adapter_status()` | Returns configured adapter descriptor plus current health |

Adapter inventory records include provider metadata and additive capability
flags such as `supports_backpressure`, `supports_raw_capture`,
`supports_fixture_replay`, `supports_stale_detection`, and
`supports_latency_metrics` when exposed by the native runtime.

#### External feed supervision

| Signature | Description |
|---|---|
| `configure_external_feed(policy)` | Sets stale/sequence policy |
| `set_external_reconnecting(reconnecting)` | Marks reconnect/degraded state |
| `external_health_tick()` | Re-evaluates stale status without ingest |

#### External ingest

| Signature | Description |
|---|---|
| `ingest_trade(symbol, price, size, aggressor_side, sequence=0, ts_exchange_ns=0, ts_recv_ns=0, quality_flags=DataQualityFlags.NONE)` | Injects one external trade |
| `ingest_book(symbol, side, level, price, size, action=BookAction.UPSERT, sequence=0, ts_exchange_ns=0, ts_recv_ns=0, quality_flags=DataQualityFlags.NONE)` | Injects one external book update |

#### Snapshots and metrics

| Signature | Description | Return |
|---|---|---|
| `book_snapshot(symbol)` | Current book snapshot | `dict[str, Any]` |
| `analytics_snapshot(symbol)` | Current analytics snapshot | `dict[str, Any]` |
| `derived_analytics_snapshot(symbol)` | Current derived analytics snapshot | `dict[str, Any]` |
| `session_candle_snapshot(symbol)` | Current session candle snapshot | `dict[str, Any]` |
| `interval_candle_snapshot(symbol, window_ns)` | Current rolling interval candle snapshot | `dict[str, Any]` |
| `signal_snapshot(symbol)` | Current signal snapshot | `dict[str, Any]` |
| `metrics()` | Runtime metrics | `dict[str, Any]` |

`book_snapshot(symbol)` returns a dictionary with:

- `venue`
- `symbol`
- `bids`
- `asks`
- `last_sequence`
- `ts_exchange_ns`
- `ts_recv_ns`

The Python wrapper retries automatically if the native snapshot payload is larger than the initial buffer.

`session_candle_snapshot(symbol)` returns a dictionary with:

- `open`
- `high`
- `low`
- `close`
- `trade_count`
- `first_ts_exchange_ns`
- `last_ts_exchange_ns`

`interval_candle_snapshot(symbol, window_ns)` returns a dictionary with:

- `window_ns`
- `open`
- `high`
- `low`
- `close`
- `trade_count`
- `total_volume`
- `vwap`
- `first_ts_exchange_ns`
- `last_ts_exchange_ns`

### Execution API Reference

Execution objects use typed dataclasses and separate native handles from the
analytics runtime.

#### Execution constants

| Class | Values |
|---|---|
| `ExecutionSide` | `BUY`, `SELL` |
| `ExecutionOrderType` | `MARKET`, `LIMIT`, `STOP`, `STOP_LIMIT` |
| `ExecutionTimeInForce` | `DAY`, `GTC`, `IOC`, `FOK`, `GTD` |

#### Execution dataclasses

| Dataclass | Purpose |
|---|---|
| `RiskLimits` | Per-route pre-trade limits: kill switch, max quantity, max notional, max open orders, max open notional, price band |
| `RouteConfig` | Route/account/venue/instrument binding plus `RiskLimits` |
| `OrderRequest` | New-order command |
| `CancelRequest` | Cancel command with new cancel id and original client id |
| `AmendRequest` | Cancel/replace command |
| `ExecutionEvent` | Typed native execution event |
| `ExecutionOrderState` | Current native order state for one client order id |
| `ExecutionHealth` | Connected/degraded/sequence health snapshot |
| `ExecutionMetrics` | Submitted/cancelled/amended/events/risk/adapter/recovery counters |
| `ExecutionWalIntegrityReport` | Offline WAL scan summary for operator diagnostics |
| `ExecutionSegmentedWalIntegrityReport` | Offline segmented WAL directory scan summary |
| `ExecutionCheckpointStoreIntegrityReport` | Offline checkpoint store scan summary |
| `ConcurrentExecutionConfig` | Command/report/event-buffer capacities |
| `ExecutionCommandReport` | Concurrent command result, sequence, result code, and events |

#### `ExecutionEngine`

| Signature | Description |
|---|---|
| `ExecutionEngine(route_or_routes, library_path=None)` | Creates a simulated execution engine for one route or a route list |
| `start()` | Starts adapter/session |
| `stop()` | Stops adapter/session |
| `close()` | Destroys native execution handle |
| `submit_order(request)` | Submits a new order and returns `list[ExecutionEvent]` |
| `cancel_order(request)` | Cancels an order and returns `list[ExecutionEvent]` |
| `amend_order(request)` | Amends an order and returns `list[ExecutionEvent]` |
| `poll_execution()` | Polls execution adapter and returns `list[ExecutionEvent]` |
| `order_state(client_order_id)` | Returns `ExecutionOrderState` |
| `execution_health()` | Returns `ExecutionHealth` |
| `execution_metrics()` | Returns `ExecutionMetrics` |

#### `ConcurrentExecutionEngine`

| Signature | Description |
|---|---|
| `ConcurrentExecutionEngine(routes, config=ConcurrentExecutionConfig(), library_path=None)` | Creates a bounded worker |
| `submit_order(request)` | Queues submit and returns command sequence |
| `cancel_order(request)` | Queues cancel and returns command sequence |
| `amend_order(request)` | Queues amend and returns command sequence |
| `poll_execution()` | Queues poll and returns command sequence |
| `try_recv_report()` | Returns `ExecutionCommandReport` or `None` without blocking |
| `stop()` | Queues worker stop and returns command sequence |

#### Top-level execution helpers

| Signature | Description |
|---|---|
| `inspect_execution_wal(path, library_path=None)` | Inspects a single execution WAL file without creating an execution engine |
| `inspect_execution_segmented_wal(root, library_path=None)` | Inspects a segmented execution WAL directory without creating an execution engine |
| `inspect_execution_checkpoint_store(root, library_path=None)` | Inspects an execution checkpoint store directory without creating an execution engine |

#### Recovery integrity diagnostics

Use `inspect_execution_wal()` and `inspect_execution_segmented_wal()` before
recovery drills, after crash restart, or in an operations health check. Both
helpers read bytes outside the order path and return counts, byte position,
optional sequence range, checksum/sequence failure counts, and validity flags.
Use the segmented helper for production rotated WAL roots.

Use `inspect_execution_checkpoint_store()` with the same restart workflow to
validate checkpoint files before selecting a restart point. It reports
discovered, valid, and invalid checkpoint counts, total checkpoint bytes, and
the latest valid checkpoint id, covered WAL sequence, and creation timestamp.
Corrupt checkpoint files do not raise when the root can be listed; they return a
report with `valid == False` so operators can inspect the failure and fall back
to the latest valid checkpoint. Missing or unreadable roots raise the mapped
native I/O error.

```python
from orderflow import (
    inspect_execution_checkpoint_store,
    inspect_execution_segmented_wal,
    inspect_execution_wal,
)

single = inspect_execution_wal("execution-wal/wal-000000000001.ofwal")
segmented = inspect_execution_segmented_wal("execution-wal")
checkpoints = inspect_execution_checkpoint_store("execution-checkpoints")
if not single.valid or not segmented.valid or not checkpoints.valid:
    raise RuntimeError("unsafe execution recovery inputs")
if checkpoints.latest_checkpoint_id is None:
    raise RuntimeError("no valid checkpoint available")
```

## Usage Patterns

### Poll-only flow (no callback)

```python
from orderflow import DataQualityFlags, Engine, EngineConfig, Symbol, StreamKind

with Engine(EngineConfig(instance_id="poll-only")) as eng:
    sym = Symbol("CME", "ESM6", 10)
    eng.subscribe(sym, StreamKind.ANALYTICS, callback=None)
    eng.poll_once(DataQualityFlags.NONE)
    snap = eng.analytics_snapshot(sym)
    print("delta", snap.get("delta"))
```

### Callback flow

```python
from orderflow import Engine, EngineConfig, Symbol, StreamKind

def on_analytics(ev: dict) -> None:
    print("analytics event:", ev)

with Engine(EngineConfig(instance_id="cb-flow")) as eng:
    sym = Symbol("CME", "ESM6", 10)
    eng.subscribe(sym, StreamKind.ANALYTICS, callback=on_analytics)
    eng.poll_once()
```

### External ingest + quality gating

```python
from orderflow import (
    BookAction,
    DataQualityFlags,
    Engine,
    EngineConfig,
    ExternalFeedPolicy,
    Side,
    Symbol,
    StreamKind,
)

sym = Symbol("BINANCE", "BTCUSDT", depth_levels=20)

with Engine(EngineConfig(instance_id="external-ingest")) as eng:
    eng.configure_external_feed(
        ExternalFeedPolicy(stale_after_ms=2_000, enforce_sequence=True)
    )
    eng.subscribe(sym, StreamKind.HEALTH, callback=lambda ev: print("health:", ev))

    eng.ingest_book(sym, Side.BID, 0, 62500000, 1000, BookAction.UPSERT, sequence=1)
    eng.ingest_trade(sym, 62510000, 200, Side.ASK, sequence=2)
    eng.poll_once(DataQualityFlags.NONE)
```

## Operational Notes

- callbacks fire during `poll_once(...)` and `ingest_*` calls.
- callback handlers should remain non-blocking.
- snapshot APIs decode runtime JSON and return Python `dict`.
- `OrderflowStateError("engine is closed")` means `close()` was already called.

## Troubleshooting

### `FileNotFoundError: Orderflow shared library not found`

- build native runtime (`cargo build -p of_ffi_c`) or provide explicit path.
- verify `ORDERFLOW_LIBRARY_PATH` points to the correct platform library.

### `OrderflowArgError` from subscribe/ingest

- validate symbol fields (`venue`, `symbol` not empty).
- validate enum-like integer constants (`Side`, `BookAction`, `StreamKind`).

### No callback events

- ensure subscription callback is not `None`.
- call `poll_once(...)` regularly if using adapter-driven mode.

## Documentation and Links

- Project docs: https://github.com/gregorian-09/orderflow/tree/main/docs
- Binding guide: https://github.com/gregorian-09/orderflow/tree/main/docs/bindings/python.md
- Handbook: https://github.com/gregorian-09/orderflow/tree/main/docs/handbook
- Changelog: https://github.com/gregorian-09/orderflow/blob/main/CHANGELOG.md
