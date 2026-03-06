# Java Binding (JNA)

This module wraps the C ABI from `crates/of_ffi_c` using JNA.

## Build native library

From workspace root:

```bash
cargo build -p of_ffi_c
```

## Build Java binding

From `bindings/java`:

```bash
mvn -q package
```

## Run example

From repo root:

```bash
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.BasicExample
```

Health-focused example:

```bash
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.HealthExample
```

External ingest example:

```bash
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.ExternalIngestExample
```

If needed, pass explicit library path:

```java
new OrderflowEngine("/absolute/path/to/libof_ffi_c.so", EngineConfig.defaults())
```

## API surface

- `OrderflowEngine` (`AutoCloseable`): lifecycle, subscribe, poll, snapshots, metrics.
- `OrderflowEngine.unsubscribe(Symbol)` for explicit symbol unsubscription at adapter/runtime level.
- `OrderflowEngine.resetSymbolSession(Symbol)` to reset per-session analytics/profile state for a symbol.
- `OrderflowEngine.ingestTrade(...)` and `OrderflowEngine.ingestBook(...)` for injecting external broker/feed events continuously.
- `OrderflowEngine.configureExternalFeed(...)`, `setExternalReconnecting(...)`, and `externalHealthTick()` for shared sequence-gap/stale/reconnect supervision.
- `OrderflowEngine.subscribe(..., EventListener)` for callback delivery during `pollOnce(...)`.
- `EngineConfig`: instance/config path/log level/persistence.
- `OrderflowEvent` and `EventListener` for stream callbacks.
- `Symbol`: venue/symbol/depth levels.
- `StreamKind`, `DataQualityFlags`, `Side`, and `BookAction` constants.

## Notes

- Polling + snapshots are supported; callback listeners are delivered during `pollOnce(...)`.
- Callback listeners are also delivered when `ingestTrade(...)` / `ingestBook(...)` is called.
- The default library path is `target/debug/<mapped lib name>`.
- `StreamKind.HEALTH` callbacks emit only on health transitions and include `health_seq`, connection/degraded state, reconnect state, quality flags, and protocol marker.
