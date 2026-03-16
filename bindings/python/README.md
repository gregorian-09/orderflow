# Orderflow Python Binding (`orderflow-gregorian09`)

[![PyPI version](https://img.shields.io/pypi/v/orderflow-gregorian09.svg)](https://pypi.org/project/orderflow-gregorian09/)
[![Python versions](https://img.shields.io/pypi/pyversions/orderflow-gregorian09.svg)](https://pypi.org/project/orderflow-gregorian09/)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](https://opensource.org/license/mit)

Production-oriented Python wrapper over the stable Orderflow C ABI (`of_ffi_c`)
for orderflow analytics, signal generation, and health-aware ingestion pipelines.

## System View

![Orderflow architecture](https://raw.githubusercontent.com/gregorian-09/orderflow/main/docs/handbook/assets/diagrams/png/04-architecture-01.png)

## What You Get

- Runtime lifecycle control (`start`, `stop`, context-manager `with Engine(...)`)
- Symbol stream subscriptions with optional callback listeners
- Poll-based processing plus push-style external ingest (`ingest_trade`, `ingest_book`)
- Snapshot retrieval (`book`, `analytics`, `signal`, `metrics`)
- External feed supervision (`stale`, `sequence`, `reconnect`) via health APIs

## Install

```bash
pip install orderflow-gregorian09
```

The Python wheel wraps the C ABI; it expects a compatible `libof_ffi_c` shared
library at runtime.

## Native Library Resolution

Lookup order:

1. Explicit `library_path=` argument on `Engine(...)`
2. `ORDERFLOW_LIBRARY_PATH` environment variable
3. Local debug build default (`target/debug/libof_ffi_c.*`)

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
    print("analytics", eng.analytics_snapshot(sym))
    print("metrics", eng.metrics_json())
```

## External Feed Ingest Workflow

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

sym = Symbol("CME", "ESM6", depth_levels=10)

with Engine(EngineConfig(instance_id="ext-feed")) as eng:
    eng.subscribe(sym, StreamKind.HEALTH, callback=lambda ev: print("health", ev))
    eng.configure_external_feed(
        ExternalFeedPolicy(stale_after_ms=2_000, enforce_sequence=True)
    )

    eng.ingest_book(
        sym, side=Side.BID, level=0, price=504900, size=20, action=BookAction.UPSERT
    )
    eng.ingest_trade(sym, price=505000, size=7, aggressor_side=Side.ASK)
    eng.poll_once(DataQualityFlags.NONE)
    print("signal", eng.signal_snapshot(sym))
```

## API Surface

- `Engine`: lifecycle, subscribe/unsubscribe, poll, ingest, snapshots
- `EngineConfig`: runtime, persistence, audit, and retention settings
- `Symbol`: venue + symbol + depth descriptor
- `ExternalFeedPolicy`: stale and sequence policy for external data
- constants: `StreamKind`, `Side`, `BookAction`, `DataQualityFlags`

## Operations Notes

- `subscribe(..., callback=None)` is supported for polling-only flows.
- `StreamKind.HEALTH` emits transition-oriented health payloads.
- `reset_symbol_session(symbol)` clears per-session analytics/profile state.
- `external_health_tick()` re-evaluates stale/degraded status without ingest.

## Learn More

- Handbook: https://github.com/gregorian-09/orderflow/tree/main/docs/handbook
- API reference: https://github.com/gregorian-09/orderflow/tree/main/docs/api
- Binding docs: https://github.com/gregorian-09/orderflow/tree/main/docs/bindings
- Changelog: https://github.com/gregorian-09/orderflow/blob/main/CHANGELOG.md
