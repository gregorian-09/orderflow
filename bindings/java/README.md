# Orderflow Java Binding (`orderflow-java-binding`)

[![Maven Central](https://img.shields.io/maven-central/v/io.github.gregorian-09/orderflow-java-binding.svg)](https://search.maven.org/artifact/io.github.gregorian-09/orderflow-java-binding)
[![JavaDoc](https://javadoc.io/badge2/io.github.gregorian-09/orderflow-java-binding/javadoc.svg)](https://javadoc.io/doc/io.github.gregorian-09/orderflow-java-binding)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)

Production-oriented Java SDK for Orderflow using JNA over the stable `of_ffi_c`
ABI. Designed for low-latency analytics workflows, deterministic replay, and
external feed bridges.

The binding also includes an additive execution API through
`OrderflowExecutionEngine`. Execution uses a separate native handle from
analytics and returns typed execution events rather than JSON on the order path.

This README is intentionally API-complete so Maven users can understand the
entire public surface from one place.

## Maven Coordinates

```xml
<dependency>
  <groupId>io.github.gregorian-09</groupId>
  <artifactId>orderflow-java-binding</artifactId>
  <version>0.4.0</version>
</dependency>
```

## What's New In 0.4.0

`0.4.0` is the first Java release with end-to-end analytics plus execution
concepts in one binding artifact. Existing `OrderflowEngine` users keep the
same market-data API; execution is exposed through separate classes and native
handles.

Highlights:

- additive simulated execution APIs: `OrderflowExecutionEngine`,
  `ConcurrentOrderflowExecutionEngine`, `OrderRequest`, `CancelRequest`,
  `AmendRequest`, `RiskLimits`, `RouteConfig`, and typed execution event
  classes
- multi-route execution construction for multi-symbol order flow
- bounded concurrent execution worker for many producers and one deterministic
  native order-state owner
- typed execution events and command reports instead of JSON on the order path
- route/account/symbol-scoped risk checks before adapter routing
- offline WAL and checkpoint-store diagnostics for recovery checks without
  opening an execution engine
- adapter inventory/status helpers for provider capability discovery before
  connecting a feed
- signal descriptor discovery through `OrderflowEngine.signalDescriptors(...)`
  and `engine.signalDescriptors()` for dashboard/configuration inventory
- signal explanation discovery through `engine.signalExplanation(symbol)` for
  audit and dashboard diagnostics
- signal metrics through `engine.signalMetrics()` for state counts,
  confidence, quality, and explanation coverage diagnostics
- offline config-driven signal validation through `SignalConfig`,
  `OrderflowEngine.validateSignalConfig`, and
  `OrderflowEngine.validateSignalReplay`
- manifest/header-driven JNA declaration generation with CI drift checks,
  while typed high-level wrappers and lifecycle ownership remain manual
- analytics-to-execution examples in this README and the handbook
- existing native resolution behavior remains available: explicit path,
  `ORDERFLOW_LIBRARY_PATH`, then local debug library

Version policy:

- Java artifact: `0.4.0`
- compatible native `of_ffi_c` library/header: `0.4.0`
- new Rust execution crates behind the native ABI: `0.1.0`

Keep the Java artifact and native runtime on the same release version. If you
build custom native execution providers, pin the Rust execution crates to a
compatible `0.1.x` line while those traits mature.

### Execution quick start

```java
import java.util.List;

RiskLimits limits = new RiskLimits(false, 100, 1_000_000, 10, 10_000_000, 0);
List<RouteConfig> routes = List.of(
    new RouteConfig("SIM", "ACC", "SIM", "ES", true, limits),
    new RouteConfig("SIM", "ACC", "SIM", "NQ", true, limits)
);

try (OrderflowExecutionEngine execution = new OrderflowExecutionEngine(null, routes)) {
    execution.start();
    execution.submitOrder(new OrderRequest(
        "C1", "ACC", "SIM", "STRAT", "SIM", "ES",
        ExecutionSide.BUY, ExecutionOrderType.LIMIT, ExecutionTimeInForce.DAY,
        10, 5000, 0, 1, 2
    ));
}
```

Deterministic TWAP planning remains separate from OMS submission:

```java
TwapConfig config = new TwapConfig(
    "parent-1", "ACC", "SIM", "TWAP", "SIM", "ES",
    ExecutionSide.BUY, ExecutionOrderType.LIMIT, ExecutionTimeInForce.DAY,
    100, 5000, 0, 1_000, 11_000, 10, 25, 0, 2_000
);
try (OrderflowExecutionEngine execution = new OrderflowExecutionEngine(null, routes);
     TwapExecutionAlgo twap = new TwapExecutionAlgo(null, config)) {
    execution.start();
    twap.plan(1_000, "child-1", "order-1", 1_001).ifPresent(child -> {
        List<ExecutionEvent> events = execution.submitOrder(child.request);
        twap.commitPending();
        events.forEach(event ->
            twap.recordExecution(event.lastQty, event.leavesQty, event.orderStatus));
    });
}
```

`plan()` is retry-stable while a child is pending. Commit only after OMS
submission succeeds; discard an abandoned plan so the same quantity can be
planned again. The planner itself cannot bypass OMS risk, journaling, or
adapters.

`new OrderflowExecutionEngine(path, route)` remains supported for single-symbol
integrations. The `List<RouteConfig>` constructor configures one engine for
multi-symbol routing with native route/account/symbol-scoped risk checks.

Use `ConcurrentOrderflowExecutionEngine` when multiple producer threads need to
queue commands into one deterministic native worker. Command methods return a
sequence number; `tryRecvReport()` returns completed command reports without
blocking.

### Binding manifest policy

Low-level native symbols are tracked in `bindings/api_manifest.toml`. The
manifest controls membership, ordering, ownership metadata, and exposure. The
validated `orderflow.h` supplies exact parameter and return types to
`tools/generate_binding_signatures.py`, which commits the generated
`OrderflowNative.java` interface. Context-sensitive mappings preserve JNA
arrays, `Memory`, opaque `Pointer` handles, pointer references, callbacks, and
integer-width reference types. CI runs the generator with `--check`, unit-tests
the mappings, and compiles the Java binding.

Do not edit `OrderflowNative.java` directly. After an additive C ABI change,
update the header and manifest, run:

```bash
python3 tools/check_api_manifest.py
python3 tools/generate_binding_signatures.py
python3 tools/test_generate_binding_signatures.py
python3 tools/check_binding_parity.py
```

Generation does not replace hand-written `AutoCloseable` lifecycle, typed
request/event classes, exception translation, buffer growth, or Java naming.
Those high-level contracts remain manual because they define user-facing
ownership and ergonomics rather than native function types.

### Signal descriptor discovery

```java
String inventory = OrderflowEngine.signalDescriptors(null);
System.out.println(inventory);
```

The descriptor inventory is read-only metadata. It helps dashboards and config
tools list built-in signals, required inputs, warmup, parameters, and output
semantics without constructing a live strategy or submitting orders.

After a signal has evaluated for a symbol, `engine.signalExplanation(symbol)`
returns the latest explanation payload with reason code, observed inputs,
thresholds, and confidence contributors. This is a diagnostics surface; order
submission decisions should still flow through explicit strategy/risk/OMS code.

`engine.signalMetrics()` returns a compact runtime summary of the current
signal cache: state counts, directional count, average confidence, quality
flagged signals, and explanation coverage.

### Config-driven replay validation

The static validation facade does not require a live `OrderflowEngine`. It
validates descriptor parameters, constructs the selected built-in natively,
evaluates ordered observations, and parses the operational report summary.

```java
import com.orderflow.bindings.OrderflowEngine;
import com.orderflow.bindings.SignalConfig;
import com.orderflow.bindings.SignalConfigParameter;
import com.orderflow.bindings.SignalValidationConfig;
import com.orderflow.bindings.SignalValidationEvent;
import java.util.List;

SignalConfig signal = new SignalConfig(
    "delta_momentum_v1",
    List.of(SignalConfigParameter.integer("threshold", 10L))
);

var configResult = OrderflowEngine.validateSignalConfig(signal, null);
if (!configResult.valid) {
    throw new IllegalArgumentException(configResult.error);
}

var report = OrderflowEngine.validateSignalReplay(
    signal,
    List.of(
        new SignalValidationEvent(20, 20, 20, 0, 100, 100, 99, 101, 1L),
        new SignalValidationEvent(-20, 0, 20, 20, 90, 95, 89, 101, 2L),
        new SignalValidationEvent(-20, -20, 20, 40, 80, 90, 79, 101, 3L)
    ),
    new SignalValidationConfig(1, 0, 0, true, true),
    null
);

System.out.println(report.directionalAccuracyBps);
System.out.println(report.labelCoverageBps);
System.out.println(report.rawJson); // config, samples, and structured warnings
```

`SignalValidationReport` parses fixed operational counters and retains the
complete versioned JSON in `rawJson`. Invalid registry construction throws
`OrderflowArgException`; call `validateSignalConfig` first for a non-throwing
configuration result. Replay validation is a research/CI path and does not
mutate live engine state or submit OMS orders.

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
    System.out.println("derived=" + eng.derivedAnalyticsSnapshot(sym));
    System.out.println("signal=" + eng.signalSnapshot(sym));
    System.out.println("metrics=" + eng.metricsJson());
    eng.stop();
}
```

## Complete End-To-End Example

This example uses deterministic external ingest and simulated execution. It is
safe for documentation, CI smoke tests, and first user experiments because it
does not connect to a broker.

```java
import java.util.List;

import com.orderflow.bindings.*;

public final class EndToEndExample {
    private static boolean signalAllowsLong(String analyticsJson, String signalJson) {
        return analyticsJson.contains("\"quality_flags\":0")
            && analyticsJson.contains("\"delta\":")
            && signalJson.contains("\"confidence\"");
    }

    public static void main(String[] args) {
        Symbol sym = new Symbol("SIM", "ES", 10);
        RiskLimits limits = new RiskLimits(false, 5, 1_000_000, 1, 1_000_000, 0);
        List<RouteConfig> routes = List.of(
            new RouteConfig("SIM", "ACC", "SIM", "ES", true, limits)
        );

        try (OrderflowEngine market = new OrderflowEngine(null, EngineConfig.defaults());
             OrderflowExecutionEngine execution = new OrderflowExecutionEngine(null, routes)) {
            market.start();
            execution.start();

            market.configureExternalFeed(2_000, true);
            market.subscribe(sym, StreamKind.ANALYTICS);
            market.subscribe(sym, StreamKind.SIGNALS);

            market.ingestBook(sym, Side.BID, 0, 500_000L, 100L, BookAction.UPSERT, 1L, 0L, 0L, DataQualityFlags.NONE);
            market.ingestBook(sym, Side.ASK, 0, 500_025L, 120L, BookAction.UPSERT, 2L, 0L, 0L, DataQualityFlags.NONE);
            market.ingestTrade(sym, 500_025L, 2L, Side.ASK, 3L, 0L, 0L, DataQualityFlags.NONE);
            market.pollOnce(DataQualityFlags.NONE);

            String analytics = market.analyticsSnapshot(sym);
            String signal = market.signalSnapshot(sym);

            if (signalAllowsLong(analytics, signal)) {
                List<ExecutionEvent> events = execution.submitOrder(new OrderRequest(
                    "JAVA-0001",
                    "ACC",
                    "SIM",
                    "DOCS",
                    "SIM",
                    "ES",
                    ExecutionSide.BUY,
                    ExecutionOrderType.LIMIT,
                    ExecutionTimeInForce.DAY,
                    1,
                    500_025L,
                    0L,
                    0L,
                    4L
                ));
                ExecutionOrderState state = execution.orderState("JAVA-0001");
                ExecutionMetrics metrics = execution.executionMetrics();

                System.out.println("events=" + events);
                System.out.println("state=" + state.clientOrderId + " status=" + state.status);
                System.out.println("submitted=" + metrics.submitted);
            } else {
                System.out.println("blocked analytics=" + analytics + " signal=" + signal);
            }
        }
    }
}
```

Use your preferred JSON parser for real decision logic. The example keeps JSON
inspection minimal so the binding remains dependency-light.

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
| `DERIVED_ANALYTICS` | 7 | Derived analytics stream after trade-driven analytics changes |

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

#### Adapter discovery

| Signature | Description |
|---|---|
| `static String adapterInventory(String nativePath)` | Returns native build adapter descriptor inventory JSON |
| `String adapterInventory()` | Returns native build adapter descriptor inventory JSON |
| `String adapterStatus()` | Returns configured adapter descriptor plus current health JSON |

Adapter inventory JSON includes provider metadata and additive capability flags
such as `supports_backpressure`, `supports_raw_capture`,
`supports_fixture_replay`, `supports_stale_detection`, and
`supports_latency_metrics` when the native runtime exposes them.

`adapterStatus()` also returns additive operational fields: `mode`,
`connection_state`, `endpoint_redacted`, `app_name`, `reconnect_attempt`,
`subscription_count`, sorted `subscribed_symbols`, queue depth/capacity,
drop/gap counters, stale state, raw-capture utilization, and optional activity
ages. Endpoint output contains only scheme and authority; user information,
paths, queries, and fragments are omitted. JSON consumers should tolerate
future additive keys and treat `null` as unavailable rather than zero.

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
| `String derivedAnalyticsSnapshot(Symbol symbol)` | Derived analytics snapshot JSON |
| `String sessionCandleSnapshot(Symbol symbol)` | Session candle snapshot JSON |
| `String intervalCandleSnapshot(Symbol symbol, long windowNs)` | Rolling interval candle snapshot JSON |
| `String signalSnapshot(Symbol symbol)` | Signal snapshot JSON |
| `String metricsJson()` | Runtime metrics JSON |

`bookSnapshot(Symbol symbol)` returns JSON with:

- `venue`
- `symbol`
- `bids`
- `asks`
- `last_sequence`
- `ts_exchange_ns`

`sessionCandleSnapshot(Symbol symbol)` returns JSON with:

- `open`
- `high`
- `low`
- `close`
- `trade_count`
- `first_ts_exchange_ns`
- `last_ts_exchange_ns`

`intervalCandleSnapshot(Symbol symbol, long windowNs)` returns JSON with:

- `window_ns`
- `open`
- `high`
- `low`
- `close`
- `trade_count`
- `total_volume`
- `vwap`
- `first_ts_exchange_ns`
- `last_ts_exchange_ns`

The Java binding retries automatically with a larger native buffer when a snapshot payload exceeds the initial allocation.

### Execution API Reference

Execution objects use typed Java classes and separate native handles from the
analytics runtime.

#### Execution constants

| Class | Values |
|---|---|
| `ExecutionSide` | `BUY`, `SELL` |
| `ExecutionOrderType` | `MARKET`, `LIMIT`, `STOP`, `STOP_LIMIT` |
| `ExecutionTimeInForce` | `DAY`, `GTC`, `IOC`, `FOK`, `GTD` |

#### Execution classes

| Class | Purpose |
|---|---|
| `RiskLimits` | Per-route pre-trade limits: kill switch, max quantity, max notional, max open orders, max open notional, price band |
| `RouteConfig` | Route/account/venue/instrument binding plus `RiskLimits` |
| `OrderRequest` | New-order command |
| `CancelRequest` | Cancel command with new cancel id and original client id |
| `AmendRequest` | Cancel/replace command |
| `ExecutionEvent` | Typed native execution event |
| `ExecutionOrderState` | Current native order state for one client order id |
| `ExecutionHealth` | Connected/degraded/sequence health snapshot |
| `ExecutionMetrics` | Submitted/cancelled/amended/events/risk/adapter/recovery counters |
| `ExecutionWalIntegrityReport` | Offline WAL scan summary for operator diagnostics |
| `ExecutionSegmentedWalIntegrityReport` | Offline segmented WAL directory scan summary |
| `ExecutionCheckpointStoreIntegrityReport` | Offline checkpoint store scan summary |
| `ConcurrentExecutionConfig` | Command/report/event-buffer capacities |
| `ExecutionCommandReport` | Concurrent command result, sequence, result code, and events |
| `TwapConfig` | Parent ticket, clip bounds, schedule, and slice interval |
| `AlgoChildPlan` | Owned child id, parent id, due time, and canonical `OrderRequest` |
| `AlgoProgress` | Target/released/completed/open quantities and child counters |

#### `TwapExecutionAlgo`

| Signature | Description |
|---|---|
| `TwapExecutionAlgo(String nativePath, TwapConfig config)` | Creates a validated native parent handle |
| `Optional<AlgoChildPlan> plan(...)` | Plans without advancing released quantity |
| `void commitPending()` | Commits a successfully submitted child |
| `void discardPending()` | Clears an unsubmitted child plan |
| `void recordExecution(...)` | Folds canonical fill/status progress |
| `AlgoProgress progress()` | Returns current progress |
| `void close()` | Destroys the native handle |

#### `OrderflowExecutionEngine`

| Signature | Description |
|---|---|
| `OrderflowExecutionEngine(String nativePath, RouteConfig route)` | Creates a single-route simulated execution engine |
| `OrderflowExecutionEngine(String nativePath, List<RouteConfig> routes)` | Creates a multi-route simulated execution engine |
| `void start()` | Starts adapter/session |
| `void stop()` | Stops adapter/session |
| `void close()` | Destroys native execution handle |
| `List<ExecutionEvent> submitOrder(OrderRequest request)` | Submits a new order |
| `List<ExecutionEvent> cancelOrder(CancelRequest request)` | Cancels an order |
| `List<ExecutionEvent> amendOrder(AmendRequest request)` | Amends an order |
| `List<ExecutionEvent> pollExecution()` | Polls execution adapter |
| `ExecutionOrderState orderState(String clientOrderId)` | Returns current order state |
| `ExecutionHealth executionHealth()` | Returns execution health |
| `ExecutionMetrics executionMetrics()` | Returns execution metrics |
| `static ExecutionWalIntegrityReport inspectWal(String nativePath, String walPath)` | Inspects a single execution WAL file without creating an execution engine |
| `static ExecutionSegmentedWalIntegrityReport inspectSegmentedWal(String nativePath, String walRoot)` | Inspects a segmented execution WAL directory without creating an execution engine |
| `static ExecutionCheckpointStoreIntegrityReport inspectCheckpointStore(String nativePath, String checkpointRoot)` | Inspects an execution checkpoint store directory without creating an execution engine |

#### `ConcurrentOrderflowExecutionEngine`

| Signature | Description |
|---|---|
| `ConcurrentOrderflowExecutionEngine(String nativePath, List<RouteConfig> routes)` | Creates a bounded worker with defaults |
| `ConcurrentOrderflowExecutionEngine(String nativePath, List<RouteConfig> routes, ConcurrentExecutionConfig config)` | Creates a bounded worker with explicit capacities |
| `long submitOrder(OrderRequest request)` | Queues submit and returns command sequence |
| `long cancelOrder(CancelRequest request)` | Queues cancel and returns command sequence |
| `long amendOrder(AmendRequest request)` | Queues amend and returns command sequence |
| `long pollExecution()` | Queues poll and returns command sequence |
| `Optional<ExecutionCommandReport> tryRecvReport()` | Receives a command report without blocking |
| `long stop()` | Queues worker stop and returns command sequence |

#### Recovery integrity diagnostics

Use `OrderflowExecutionEngine.inspectWal()` and
`OrderflowExecutionEngine.inspectSegmentedWal()` before recovery drills, after
crash restart, or in an operations health check. Both helpers read bytes
outside the order path and return counts, byte position, optional sequence
range, checksum/sequence failure counts, and validity flags. Use the segmented
helper for production rotated WAL roots.

Use `OrderflowExecutionEngine.inspectCheckpointStore()` with the same restart
workflow to validate checkpoint files before selecting a restart point. It
reports discovered, valid, and invalid checkpoint counts, total checkpoint
bytes, and the latest valid checkpoint id, covered WAL sequence, and creation
timestamp. Corrupt checkpoint files do not throw when the root can be listed;
they return a report with `valid == false` so operators can inspect the failure
and fall back to the latest valid checkpoint. Missing or unreadable roots throw
the mapped native I/O exception.

```java
import com.orderflow.bindings.ExecutionCheckpointStoreIntegrityReport;
import com.orderflow.bindings.ExecutionSegmentedWalIntegrityReport;
import com.orderflow.bindings.ExecutionWalIntegrityReport;
import com.orderflow.bindings.OrderflowExecutionEngine;

ExecutionWalIntegrityReport report =
    OrderflowExecutionEngine.inspectWal(null, "execution-wal/wal-000000000001.ofwal");
ExecutionSegmentedWalIntegrityReport segmented =
    OrderflowExecutionEngine.inspectSegmentedWal(null, "execution-wal");
ExecutionCheckpointStoreIntegrityReport checkpoints =
    OrderflowExecutionEngine.inspectCheckpointStore(null, "execution-checkpoints");
if (!report.valid || !segmented.valid || !checkpoints.valid) {
    throw new IllegalStateException("unsafe execution recovery inputs");
}
if (checkpoints.latestCheckpointId == null) {
    throw new IllegalStateException("no valid checkpoint available");
}
```

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
