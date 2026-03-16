# Java Binding (JNA)

Java wrapper over the stable Orderflow C ABI (`of_ffi_c`) using JNA.

## What You Get

- Runtime lifecycle control (`start`, `stop`, `close`)
- Symbol subscription and callback streaming
- Adapter polling + external ingest (`ingestTrade`, `ingestBook`)
- JSON snapshots (`book`, `analytics`, `signal`, `metrics`)
- Feed supervision helpers (stale/sequence/reconnect policy)

## Prerequisites

1. Build native runtime:

```bash
cargo build -p of_ffi_c
```

2. Build Java binding:

```bash
mvn -q -f bindings/java/pom.xml package
```

## Native Library Resolution

`OrderflowEngine` resolves native library in this order:

1. explicit constructor path
2. `ORDERFLOW_LIBRARY_PATH`
3. default debug path (`target/debug/<platform-lib-name>`)

Example:

```java
new OrderflowEngine("/absolute/path/to/libof_ffi_c.so", EngineConfig.defaults());
```

## Minimal Usage

```java
import com.orderflow.bindings.*;

EngineConfig cfg = EngineConfig.defaults();
try (OrderflowEngine eng = new OrderflowEngine(null, cfg)) {
    eng.start();
    Symbol sym = new Symbol("CME", "ESM6", 10);
    eng.subscribe(sym, StreamKind.ANALYTICS);
    eng.pollOnce(DataQualityFlags.NONE);
    System.out.println(eng.analyticsSnapshot(sym));
    System.out.println(eng.metricsJson());
}
```

## Example Apps

```bash
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.BasicExample
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.HealthExample
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.ExternalIngestExample
```

## API Map

- `OrderflowEngine`: lifecycle, subscribe/unsubscribe, poll, ingest, snapshots.
- `EngineConfig`: runtime options + persistence/audit knobs.
- `Symbol`: venue/instrument/depth descriptor.
- `OrderflowEvent` + `EventListener`: callback event envelope.
- `StreamKind`, `Side`, `BookAction`, `DataQualityFlags`: stable constants.

## Operational Notes

- Callback listeners are delivered during `pollOnce(...)` and external ingest calls.
- `StreamKind.HEALTH` emits transition events with health sequence and quality flags.
- `resetSymbolSession(symbol)` clears per-session analytics/profile state for the symbol.
