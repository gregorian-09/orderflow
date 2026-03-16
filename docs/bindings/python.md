# Python Binding

Package: `orderflow-gregorian09`  
Source: `bindings/python`

## Distribution Model

- Published to PyPI as the Python wrapper package.
- Native runtime (`libof_ffi_c`) is distributed separately as release artifacts.

At runtime, the wrapper resolves the native library in this order:

1. `library_path=` argument passed to `Engine(...)`.
2. `ORDERFLOW_LIBRARY_PATH` environment variable.
3. Local default path (`target/debug/libof_ffi_c.*`).

## API Surface

- `Engine`: lifecycle, poll loop, subscriptions, ingest, snapshots.
- `EngineConfig`: runtime creation parameters and persistence/audit knobs.
- `Symbol`: normalized venue/instrument descriptor.
- `ExternalFeedPolicy`: stale/sequence supervision policy.
- constants: `StreamKind`, `Side`, `BookAction`, `DataQualityFlags`.

## Runtime Behavior

- `subscribe(..., callback=...)` callbacks fire on `poll_once(...)` and `ingest_*`.
- `subscribe(..., callback=None)` is supported for polling-only flows.
- Snapshot methods return decoded dictionaries from runtime JSON payloads.
- `StreamKind.HEALTH` emits transition events (`connected/degraded/reconnect_state`).

## Install

### From PyPI

```bash
pip install orderflow-gregorian09
```

Then point to native library:

```bash
export ORDERFLOW_LIBRARY_PATH=/opt/orderflow/lib/libof_ffi_c.so
```

### Local editable install

```bash
pip install -e bindings/python
```

## Minimal usage

```python
from orderflow import Engine, EngineConfig, Symbol, StreamKind

with Engine(EngineConfig(instance_id="py")) as eng:
    sym = Symbol("CME", "ESM6", 10)
    eng.subscribe(sym, StreamKind.ANALYTICS)
    eng.poll_once()
    print(eng.analytics_snapshot(sym))
```

## Metadata Quality

PyPI metadata includes:

- project URLs (Homepage/Docs/Repository/Issues)
- classifiers for trading + library categories
- package keywords for discoverability

## Release pipeline

Workflow: `.github/workflows/publish-python.yml`

Version source of truth: `bindings/versions.toml`  
Sync command: `python3 tools/release/sync_binding_versions.py`
