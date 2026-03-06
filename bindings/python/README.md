# Python Binding (ctypes)

This package wraps `crates/of_ffi_c` through `ctypes`.

## Build the native library

From workspace root:

```bash
cargo build -p of_ffi_c
```

Default lookup path is `target/debug/libof_ffi_c.so` (Linux), with platform-specific suffix handling.

You can override lookup with:

```bash
export ORDERFLOW_LIBRARY_PATH=/absolute/path/to/libof_ffi_c.so
```

## Install package (editable)

From `bindings/python`:

```bash
python -m pip install -e .
```

## Quick usage

```python
from orderflow import BookAction, Engine, EngineConfig, ExternalFeedPolicy, Side, Symbol, StreamKind

cfg = EngineConfig(instance_id="py-client", config_path="")

with Engine(cfg) as eng:
    sym = Symbol("CME", "ESM6", depth_levels=10)
    eng.subscribe(sym, StreamKind.ANALYTICS)
    eng.subscribe(sym, StreamKind.HEALTH, callback=lambda ev: print("health", ev))
    eng.configure_external_feed(ExternalFeedPolicy(stale_after_ms=2_000, enforce_sequence=True))
    eng.poll_once()
    eng.ingest_book(sym, side=Side.BID, level=0, price=504900, size=20, action=BookAction.UPSERT)
    eng.ingest_trade(sym, price=505000, size=7, aggressor_side=Side.ASK)
    eng.unsubscribe(sym)
    print(eng.analytics_snapshot(sym))
    print(eng.signal_snapshot(sym))
    print(eng.metrics())
```

## Notes

- `config_path` can point to runtime `.toml` or `.json` config consumed by Rust runtime.
- `ORDERFLOW_LIBRARY_PATH` is honored when `library_path` is not passed to `Engine(...)`.
- Optional callback is supported on `subscribe(...)` and fires during `poll_once(...)` and external `ingest_*` calls.
- `StreamKind.HEALTH` emits only on health state changes and includes fields like `health_seq`, `connected`, `degraded`, `reconnect_state`, `quality_flags`, and `protocol_info`.
- `Engine.reset_symbol_session(symbol)` resets per-session analytics/profile context for a symbol.
- `Engine.ingest_trade(...)` and `Engine.ingest_book(...)` let you inject external feed events directly without using a market-data adapter stream.
- `Engine.configure_external_feed(...)`, `Engine.set_external_reconnecting(...)`, and `Engine.external_health_tick()` provide uniform sequence-gap/stale/reconnect supervision for external feeds.

## Examples

- `bindings/python/examples/basic.py`
- `bindings/python/examples/health_example.py`
- `bindings/python/examples/external_ingest.py`

Run:

```bash
python bindings/python/examples/health_example.py
```
