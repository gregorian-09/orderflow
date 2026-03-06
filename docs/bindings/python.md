# Python Binding

Package: `orderflow`  
Source: `bindings/python`

## Distribution Model

- Published to PyPI as the Python wrapper package.
- Native runtime (`libof_ffi_c`) is distributed separately as release artifacts.

At runtime, the wrapper resolves the native library in this order:

1. `library_path=` argument passed to `Engine(...)`.
2. `ORDERFLOW_LIBRARY_PATH` environment variable.
3. Local default path (`target/debug/libof_ffi_c.*`).

## Install

### From PyPI

```bash
pip install orderflow
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

## Release pipeline

Workflow: `.github/workflows/publish-python.yml`
