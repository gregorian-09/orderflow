# Python Binding (ctypes)

Python wrapper over the stable Orderflow C ABI (`of_ffi_c`).

## Capability Overview

- Engine lifecycle control (`start`, `stop`, context manager support)
- Subscription and callback streaming
- Adapter polling and external event ingest
- Snapshot access (`book`, `analytics`, `signal`, `metrics`)
- Data-quality supervision for external feeds

## Prerequisites

Build native runtime from repository root:

```bash
cargo build -p of_ffi_c
```

Install Python package locally (editable):

```bash
python -m pip install -e bindings/python
```

## Native Library Resolution

Resolution order:

1. `library_path=` argument passed to `Engine(...)`
2. `ORDERFLOW_LIBRARY_PATH` environment variable
3. default debug target path (`target/debug/libof_ffi_c.*`)

```bash
export ORDERFLOW_LIBRARY_PATH=/absolute/path/to/libof_ffi_c.so
```

## Minimal Usage

```python
from orderflow import DataQualityFlags, Engine, EngineConfig, Symbol, StreamKind

with Engine(EngineConfig(instance_id="py-client")) as eng:
    sym = Symbol("CME", "ESM6", depth_levels=10)
    eng.subscribe(sym, StreamKind.ANALYTICS)
    eng.poll_once(DataQualityFlags.NONE)
    print(eng.analytics_snapshot(sym))
```

## External Ingest Example

```python
from orderflow import (
    BookAction, DataQualityFlags, Engine, EngineConfig, ExternalFeedPolicy,
    Side, Symbol, StreamKind
)

sym = Symbol("CME", "ESM6", depth_levels=10)
with Engine(EngineConfig(instance_id="ingest")) as eng:
    eng.subscribe(sym, StreamKind.HEALTH, callback=lambda ev: print("health", ev))
    eng.configure_external_feed(ExternalFeedPolicy(stale_after_ms=2_000, enforce_sequence=True))
    eng.ingest_book(sym, side=Side.BID, level=0, price=504900, size=20, action=BookAction.UPSERT)
    eng.ingest_trade(sym, price=505000, size=7, aggressor_side=Side.ASK)
    eng.poll_once(DataQualityFlags.NONE)
    print(eng.signal_snapshot(sym))
```

## Runtime Notes

- `config_path` may point to runtime TOML consumed by Rust engine config loader.
- Optional callback on `subscribe(...)` is supported; callbacks fire during polling and ingest.
- `StreamKind.HEALTH` emits transition-only events with health sequence and quality flags.
- `reset_symbol_session(symbol)` clears per-session analytics/profile context.
- `configure_external_feed(...)`, `set_external_reconnecting(...)`, and
  `external_health_tick()` provide sequence/stale/reconnect supervision.

## Included Examples

- `bindings/python/examples/basic.py`
- `bindings/python/examples/health_example.py`
- `bindings/python/examples/external_ingest.py`
