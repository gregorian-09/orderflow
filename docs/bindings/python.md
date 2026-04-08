# Python Binding

Package: `orderflow-gregorian09`  
Source: `bindings/python`

This guide complements the PyPI page and serves as an engineering/operator view
of the Python binding.

## Scope

- Pythonic runtime control over the Orderflow C ABI.
- Streaming and poll-driven processing workflows.
- External feed ingestion and data-quality supervision.
- Snapshot and metrics retrieval.

## Runtime Dependency Model

The Python wheel ships wrapper code only. Native runtime (`libof_ffi_c`) is
distributed separately.

Native lookup order:

1. `library_path=` passed to `Engine(...)`
2. `ORDERFLOW_LIBRARY_PATH`
3. local default path under `target/debug`

## Public API Index

### Constants

- `StreamKind`: `BOOK`, `TRADES`, `ANALYTICS`, `SIGNALS`, `HEALTH`, `BOOK_SNAPSHOT`, `DERIVED_ANALYTICS`
- `Side`: `BID`, `ASK`
- `BookAction`: `UPSERT`, `DELETE`
- `DataQualityFlags`: `NONE`, `STALE_FEED`, `SEQUENCE_GAP`, `CLOCK_SKEW`,
  `DEPTH_TRUNCATED`, `OUT_OF_ORDER`, `ADAPTER_DEGRADED`

### Types

- `Symbol(venue, symbol, depth_levels=10)`
- `EngineConfig(...)`
- `ExternalFeedPolicy(stale_after_ms=15000, enforce_sequence=True)`

### Engine

- metadata: `api_version`, `build_info`
- lifecycle: `start`, `stop`, `close`, context-manager support
- subscription: `subscribe`, `unsubscribe`, `poll_once`, `reset_symbol_session`
- external feed policy: `configure_external_feed`,
  `set_external_reconnecting`, `external_health_tick`
- ingest: `ingest_trade`, `ingest_book`
- snapshots: `book_snapshot`, `analytics_snapshot`, `derived_analytics_snapshot`, `signal_snapshot`, `metrics`

`book_snapshot(symbol)` returns a dictionary with:

- `venue`
- `symbol`
- `bids`
- `asks`
- `last_sequence`
- `ts_exchange_ns`
- `ts_recv_ns`

The Python binding automatically retries with a larger native buffer when a snapshot exceeds the default initial allocation.

`derived_analytics_snapshot(symbol)` returns additive session metrics such as
`total_volume`, `trade_count`, `vwap`, `average_trade_size`, and `imbalance_bps`.

### Exceptions

- `OrderflowError`
- `OrderflowStateError`
- `OrderflowArgError`

## Integration Workflows

### Poll-driven

- subscribe with `callback=None`
- call `poll_once` on your scheduler
- consume snapshots from `book_snapshot`, `analytics_snapshot`, `signal_snapshot`

### Listener-driven

- pass callback to `subscribe(...)`
- keep callbacks short and non-blocking
- use `poll_once(...)` cadence for adapter-driven feeds

### External bridge-driven

- configure policy with `configure_external_feed(...)`
- inject events with `ingest_trade` / `ingest_book`
- use `set_external_reconnecting` and `external_health_tick` for feed-state signaling

## Distribution and Quality Controls

- version authority: `bindings/versions.toml`
- sync/check tool: `python3 tools/release/sync_binding_versions.py --check`
- docs coverage gate: `python3 tools/docs_coverage.py --enforce`
- publish workflow: `.github/workflows/publish-python.yml`

## Reference

- PyPI package page and complete API README: `bindings/python/README.md`
- package source: `bindings/python/orderflow`
