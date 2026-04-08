# Orderflow Java Binding (`orderflow-java-binding`)

[![Maven Central](https://img.shields.io/maven-central/v/io.github.gregorian-09/orderflow-java-binding.svg)](https://search.maven.org/artifact/io.github.gregorian-09/orderflow-java-binding)
[![JavaDoc](https://javadoc.io/badge2/io.github.gregorian-09/orderflow-java-binding/javadoc.svg)](https://javadoc.io/doc/io.github.gregorian-09/orderflow-java-binding)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)

Production-oriented Java SDK for Orderflow using JNA over the stable `of_ffi_c`
ABI. Designed for low-latency analytics workflows, deterministic replay, and
external feed bridges.

This README is intentionally API-complete so Maven users can understand the
entire public surface from one place.

## Maven Coordinates

```xml
<dependency>
  <groupId>io.github.gregorian-09</groupId>
  <artifactId>orderflow-java-binding</artifactId>
  <version>0.1.4</version>
</dependency>
```

## Java Version

- Java 17+

## Native Runtime Requirement

The Java artifact is a wrapper. You also need a compatible `libof_ffi_c`
dynamic library at runtime.

Native library resolution order:

1. explicit constructor path in `new OrderflowEngine(path, cfg)`
2. `ORDERFLOW_LIBRARY_PATH` environment variable
3. default local path `target/debug/<mapped-lib-name>`

## Quick Start

```java
import com.orderflow.bindings.DataQualityFlags;
import com.orderflow.bindings.EngineConfig;
import com.orderflow.bindings.OrderflowEngine;
import com.orderflow.bindings.StreamKind;
import com.orderflow.bindings.Symbol;

EngineConfig cfg = EngineConfig.defaults();
try (OrderflowEngine eng = new OrderflowEngine(null, cfg)) {
    eng.start();
    Symbol sym = new Symbol("CME", "ESM6", 10);
    eng.subscribe(sym, StreamKind.ANALYTICS);
    eng.pollOnce(DataQualityFlags.NONE);
    System.out.println("apiVersion=" + eng.apiVersion());
    System.out.println("buildInfo=" + eng.buildInfo());
    System.out.println("analytics=" + eng.analyticsSnapshot(sym));
    System.out.println("signal=" + eng.signalSnapshot(sym));
    System.out.println("metrics=" + eng.metricsJson());
    eng.stop();
}
```

## Public API Reference

### Constants

#### `StreamKind`

| Name | Value | Meaning |
|---|---:|---|
| `BOOK` | 1 | Level-2 book stream |
| `TRADES` | 2 | Trade stream |
| `ANALYTICS` | 3 | Analytics stream |
| `SIGNALS` | 4 | Signal stream |
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
| `UPSERT` | 0 | Insert/update level |
| `DELETE` | 1 | Delete level |

#### `DataQualityFlags`

| Name | Value | Meaning |
|---|---:|---|
| `NONE` | `0` | No quality issues |
| `STALE_FEED` | `1 << 0` | Feed stale |
| `SEQUENCE_GAP` | `1 << 1` | Sequence gap |
| `CLOCK_SKEW` | `1 << 2` | Clock skew |
| `DEPTH_TRUNCATED` | `1 << 3` | Depth truncation |
| `OUT_OF_ORDER` | `1 << 4` | Out-of-order sequence |
| `ADAPTER_DEGRADED` | `1 << 5` | Adapter/feed degraded |

### Core Data Types

#### `EngineConfig`

Immutable runtime configuration:

| Field | Type | Meaning |
|---|---|---|
| `instanceId` | `String` | Runtime instance identifier |
| `configPath` | `String` | Optional config file path |
| `logLevel` | `int` | Reserved log-level field |
| `enablePersistence` | `boolean` | Enables persistence |
| `auditMaxBytes` | `long` | Audit file rotation threshold |
| `auditMaxFiles` | `int` | Rotated audit files retained |
| `auditRedactTokensCsv` | `String` | Redaction token list |
| `dataRetentionMaxBytes` | `long` | Retention byte cap |
| `dataRetentionMaxAgeSecs` | `long` | Retention age cap |

Factory:

- `EngineConfig.defaults()`

#### `Symbol`

`Symbol(String venue, String symbol, int depthLevels)`

#### `OrderflowEvent`

Callback event envelope fields:

- `tsExchangeNs`, `tsRecvNs`
- `kind`, `schemaId`, `qualityFlags`
- `payloadJson`

#### `EventListener`

- `void onEvent(OrderflowEvent event)`

### Exceptions

| Exception | Purpose |
|---|---|
| `OrderflowException` | Base runtime/binding failure |
| `OrderflowStateException` | Invalid lifecycle/state usage |
| `OrderflowArgException` | Invalid argument passed to native API |

### `OrderflowEngine` API

#### Constructor and metadata

| Signature | Description |
|---|---|
| `OrderflowEngine(String nativePath, EngineConfig config)` | Creates native runtime wrapper |
| `int apiVersion()` | Returns ABI version |
| `String buildInfo()` | Returns native build info |

#### Lifecycle

| Signature | Description |
|---|---|
| `void start()` | Starts runtime |
| `void stop()` | Stops runtime |
| `void close()` | Releases subscriptions and native handle |

#### Subscription and polling

| Signature | Description |
|---|---|
| `void subscribe(Symbol symbol, int streamKind)` | Subscribe without listener |
| `void subscribe(Symbol symbol, int streamKind, EventListener listener)` | Subscribe with listener |
| `void pollOnce(int qualityFlags)` | Poll runtime/adapter once |
| `void unsubscribe(Symbol symbol)` | Remove symbol subscriptions |
| `void resetSymbolSession(Symbol symbol)` | Reset symbol session state |

#### External feed supervision

| Signature | Description |
|---|---|
| `void configureExternalFeed(long staleAfterMs, boolean enforceSequence)` | Configure stale/sequence checks |
| `void setExternalReconnecting(boolean reconnecting)` | Set reconnect/degraded state |
| `void externalHealthTick()` | Trigger health reevaluation |

#### External ingest

| Signature | Description |
|---|---|
| `void ingestTrade(Symbol symbol, long price, long size, int aggressorSide)` | Trade ingest convenience overload |
| `void ingestTrade(Symbol symbol, long price, long size, int aggressorSide, long sequence, long tsExchangeNs, long tsRecvNs, int qualityFlags)` | Full trade ingest |
| `void ingestBook(Symbol symbol, int side, int level, long price, long size)` | Book ingest convenience overload |
| `void ingestBook(Symbol symbol, int side, int level, long price, long size, int action, long sequence, long tsExchangeNs, long tsRecvNs, int qualityFlags)` | Full book ingest |

#### Snapshots and metrics

| Signature | Description |
|---|---|
| `String bookSnapshot(Symbol symbol)` | Book snapshot JSON |
| `String analyticsSnapshot(Symbol symbol)` | Analytics snapshot JSON |
| `String signalSnapshot(Symbol symbol)` | Signal snapshot JSON |
| `String metricsJson()` | Runtime metrics JSON |

`bookSnapshot(Symbol symbol)` returns JSON with:

- `venue`
- `symbol`
- `bids`
- `asks`
- `last_sequence`
- `ts_exchange_ns`
- `ts_recv_ns`

The Java binding retries automatically with a larger native buffer when a snapshot payload exceeds the initial allocation.

## Usage Patterns

### Listener-based flow

```java
import com.orderflow.bindings.*;

try (OrderflowEngine eng = new OrderflowEngine(null, EngineConfig.defaults())) {
    eng.start();
    Symbol sym = new Symbol("CME", "ESM6", 10);
    eng.subscribe(sym, StreamKind.HEALTH, ev -> System.out.println("health=" + ev.payloadJson));
    eng.subscribe(sym, StreamKind.ANALYTICS, ev -> System.out.println("analytics=" + ev.payloadJson));
    eng.pollOnce(DataQualityFlags.NONE);
}
```

### External ingest flow

```java
import com.orderflow.bindings.*;

try (OrderflowEngine eng = new OrderflowEngine(null, EngineConfig.defaults())) {
    eng.start();
    Symbol sym = new Symbol("BINANCE", "BTCUSDT", 20);
    eng.configureExternalFeed(2_000, true);
    eng.ingestBook(sym, Side.BID, 0, 62500000L, 1000L, BookAction.UPSERT, 1L, 0L, 0L, DataQualityFlags.NONE);
    eng.ingestTrade(sym, 62510000L, 200L, Side.ASK, 2L, 0L, 0L, DataQualityFlags.NONE);
    System.out.println(eng.signalSnapshot(sym));
}
```

## Operational Guidance

- keep listener callbacks fast and non-blocking.
- listeners are invoked from runtime callback context during `pollOnce(...)` and
  `ingest*` paths.
- snapshot methods return JSON strings; parse with your preferred JSON library.

## Troubleshooting

### Native load failure

- verify `ORDERFLOW_LIBRARY_PATH`.
- verify architecture match (JVM arch must match native library arch).
- verify file permissions and dependency resolution for shared objects.

### State exceptions

- `OrderflowStateException("engine is closed")` means `close()` was already called.
- start engine before poll/subscribe/ingest calls.

### No events

- ensure subscription stream kind matches expected callback channel.
- call `pollOnce(...)` regularly in adapter-driven mode.

## Example Programs

```bash
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.BasicExample
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.HealthExample
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.ExternalIngestExample
```

## Documentation and Links

- JavaDoc: https://javadoc.io/doc/io.github.gregorian-09/orderflow-java-binding
- Binding guide: https://github.com/gregorian-09/orderflow/tree/main/docs/bindings/java.md
- Handbook: https://github.com/gregorian-09/orderflow/tree/main/docs/handbook
- Changelog: https://github.com/gregorian-09/orderflow/blob/main/CHANGELOG.md
