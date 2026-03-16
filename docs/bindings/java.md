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

## API Surface

- `OrderflowEngine`: create/start/stop/close runtime, subscription lifecycle,
  callback dispatch, ingest, and snapshot retrieval.
- `EngineConfig`: immutable runtime options object.
- `Symbol`: venue/symbol/depth descriptor.
- `OrderflowEvent` + `EventListener`: callback envelope and listener interface.
- constants: `StreamKind`, `Side`, `BookAction`, `DataQualityFlags`.

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

## JavaDoc Structure

The binding ships package-level docs (`package-info.java`) for:

- `com.orderflow.bindings` (library architecture + usage contract)
- `com.orderflow.examples` (runnable example index)

This improves generated JavaDoc discoverability and mirrors production SDK style.

The published JavaDoc is additionally enriched via
`bindings/java/src/main/javadoc/overview.html` as a top-level API landing page.

## Release pipeline

Workflow: `.github/workflows/publish-java.yml`

Version source of truth: `bindings/versions.toml`  
Sync command: `python3 tools/release/sync_binding_versions.py`

## Release prerequisites

Required repository secrets:

- `MAVEN_CENTRAL_TOKEN_USERNAME`
- `MAVEN_CENTRAL_TOKEN_PASSWORD`
- `MAVEN_GPG_PRIVATE_KEY`
- `MAVEN_GPG_PASSPHRASE`

The workflow runs a preflight that verifies the imported secret key's fingerprint
is discoverable on `keys.openpgp.org`. If this fails, publish/verify the public
key first, then rerun the workflow.
