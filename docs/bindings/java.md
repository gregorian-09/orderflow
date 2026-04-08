# Java Binding

Artifact: `io.github.gregorian-09:orderflow-java-binding`  
Source: `bindings/java`

This guide complements Maven Central + JavaDoc and captures binding behavior
from an integration and release-engineering perspective.

## Scope

- JNA wrapper over stable `of_ffi_c` symbols
- lifecycle/session control through `OrderflowEngine`
- subscription callbacks, polling, and external ingest
- snapshot retrieval and metrics access

## Runtime Dependency Model

The JAR provides Java wrappers; native runtime (`libof_ffi_c`) is loaded at
runtime.

Native lookup order:

1. explicit path in `new OrderflowEngine(path, cfg)`
2. `ORDERFLOW_LIBRARY_PATH`
3. default local debug path under `target/debug`

## Public API Index

### Constants

- `StreamKind`: `BOOK`, `TRADES`, `ANALYTICS`, `SIGNALS`, `HEALTH`, `BOOK_SNAPSHOT`
- `Side`: `BID`, `ASK`
- `BookAction`: `UPSERT`, `DELETE`
- `DataQualityFlags`: quality bit flags used for ingest/poll context

### Types

- `EngineConfig` + `EngineConfig.defaults()`
- `Symbol`
- `OrderflowEvent`
- `EventListener`

### Engine API

- constructor: `OrderflowEngine(String nativePath, EngineConfig config)`
- metadata: `apiVersion()`, `buildInfo()`
- lifecycle: `start()`, `stop()`, `close()`
- subscriptions: `subscribe(...)`, `unsubscribe(...)`, `pollOnce(...)`,
  `resetSymbolSession(...)`
- external feed controls: `configureExternalFeed(...)`,
  `setExternalReconnecting(...)`, `externalHealthTick()`
- ingest: `ingestTrade(...)`, `ingestBook(...)` (convenience + full overloads)
- snapshots: `bookSnapshot(...)`, `analyticsSnapshot(...)`, `derivedAnalyticsSnapshot(...)`,
  `signalSnapshot(...)`, `metricsJson()`

`bookSnapshot(...)` returns JSON with:

- `venue`
- `symbol`
- `bids`
- `asks`
- `last_sequence`
- `ts_exchange_ns`
- `ts_recv_ns`

The Java binding retries with a larger native buffer automatically when snapshot payloads exceed the initial allocation.

`derivedAnalyticsSnapshot(...)` returns additive session metrics such as
`total_volume`, `trade_count`, `vwap`, `average_trade_size`, and `imbalance_bps`.

### Exceptions

- `OrderflowException`
- `OrderflowStateException`
- `OrderflowArgException`

## Integration Patterns

### Analytics polling loop

- subscribe analytics stream
- execute `pollOnce(DataQualityFlags.NONE)` on app schedule
- consume snapshot JSON with your JSON parser

### Health stream monitoring

- subscribe `StreamKind.HEALTH` with `EventListener`
- record transition payloads for SRE/ops telemetry

### External broker bridge

- configure policy with `configureExternalFeed(staleAfterMs, enforceSequence)`
- map broker payloads into `ingestTrade` / `ingestBook`
- set reconnect state explicitly during broker reconnect windows

## Distribution and Quality Controls

- version authority: `bindings/versions.toml`
- sync/check tool: `python3 tools/release/sync_binding_versions.py --check`
- docs coverage gate: `python3 tools/docs_coverage.py --enforce`
- publish workflow: `.github/workflows/publish-java.yml`
- JavaDoc landing page: `bindings/java/src/main/javadoc/overview.html`

## Reference

- Maven README (full API reference): `bindings/java/README.md`
- JavaDoc package docs: `bindings/java/src/main/java/com/orderflow/bindings/package-info.java`
