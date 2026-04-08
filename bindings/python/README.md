# Orderflow Python Binding (`orderflow-gregorian09`)

[![PyPI version](https://img.shields.io/pypi/v/orderflow-gregorian09.svg)](https://pypi.org/project/orderflow-gregorian09/)
[![Python versions](https://img.shields.io/pypi/pyversions/orderflow-gregorian09.svg)](https://pypi.org/project/orderflow-gregorian09/)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](https://opensource.org/license/mit)

Production-focused Python API for the Orderflow runtime.  
This package wraps the stable `of_ffi_c` ABI via `ctypes` and provides a typed,
high-level interface for lifecycle management, subscriptions, snapshots, and
external feed ingestion.

The README is intentionally API-complete so the PyPI page can be used as a
single reference, similar to high-signal package pages such as TA-Lib and
FastAPI.

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
be available at runtime.

Library resolution order:

1. `library_path=` passed to `Engine(...)`
2. `ORDERFLOW_LIBRARY_PATH` environment variable
3. default local debug path (`target/debug/libof_ffi_c.*`)

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
