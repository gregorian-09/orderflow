# Orderflow Java Binding (`orderflow-java-binding`)

[![Maven Central](https://img.shields.io/maven-central/v/io.github.gregorian-09/orderflow-java-binding.svg)](https://search.maven.org/artifact/io.github.gregorian-09/orderflow-java-binding)
[![JavaDoc](https://javadoc.io/badge2/io.github.gregorian-09/orderflow-java-binding/javadoc.svg)](https://javadoc.io/doc/io.github.gregorian-09/orderflow-java-binding)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)

Java (JNA) wrapper over the stable Orderflow C ABI (`of_ffi_c`) for
event-driven orderflow analytics and external feed ingestion.

## Coordinates

```xml
<dependency>
  <groupId>io.github.gregorian-09</groupId>
  <artifactId>orderflow-java-binding</artifactId>
  <version>0.1.3</version>
</dependency>
```

## Features

- Runtime lifecycle (`start`, `stop`, `close`)
- Stream subscriptions and listener callbacks
- Polling and external ingest (`ingestTrade`, `ingestBook`)
- JSON snapshots (`book`, `analytics`, `signal`, `metrics`)
- Feed-health controls (stale/sequence/reconnect)

## Native Library Resolution

`OrderflowEngine` resolves the native library in this order:

1. Explicit constructor path
2. `ORDERFLOW_LIBRARY_PATH` environment variable
3. Default debug path (`target/debug/<platform-lib-name>`)

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
    System.out.println("analytics=" + eng.analyticsSnapshot(sym));
    System.out.println("metrics=" + eng.metricsJson());
}
```

## Examples

```bash
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.BasicExample
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.HealthExample
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.ExternalIngestExample
```

## API Map

- `OrderflowEngine`: high-level runtime operations
- `EngineConfig`: immutable runtime configuration
- `Symbol`: venue/instrument/depth descriptor
- `OrderflowEvent` + `EventListener`: callback envelope + handler
- `StreamKind`, `Side`, `BookAction`, `DataQualityFlags`: stable constants

## Documentation

- JavaDoc: https://javadoc.io/doc/io.github.gregorian-09/orderflow-java-binding
- Handbook: https://github.com/gregorian-09/orderflow/tree/main/docs/handbook
- API reference: https://github.com/gregorian-09/orderflow/tree/main/docs/api
- Binding guide: https://github.com/gregorian-09/orderflow/tree/main/docs/bindings/java.md
