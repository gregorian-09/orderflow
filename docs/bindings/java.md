# Java Binding

Artifact: `io.github.gregorian-09:orderflow-java-binding`  
Source: `bindings/java`

## Distribution Model

- Published as a Maven artifact (JNA wrapper).
- Native runtime (`libof_ffi_c`) is distributed separately as release artifacts.

Runtime native library lookup:

1. Explicit constructor path: `new OrderflowEngine("/abs/path/libof_ffi_c.so", cfg)`.
2. `ORDERFLOW_LIBRARY_PATH` environment variable.
3. Default local path: `target/debug/<mapped-lib-name>`.

## Usage

```java
EngineConfig cfg = EngineConfig.defaults();
try (OrderflowEngine eng = new OrderflowEngine(null, cfg)) {
    eng.start();
    Symbol sym = new Symbol("CME", "ESM6", 10);
    eng.subscribe(sym, StreamKind.ANALYTICS);
    eng.pollOnce(DataQualityFlags.NONE);
    System.out.println(eng.analyticsSnapshot(sym));
    eng.stop();
}
```

## Release pipeline

Workflow: `.github/workflows/publish-java.yml`
