# Source-of-Truth Map

This map prevents conflicting descriptions of the same behavior across the
handbook, READMEs, bindings, and generated references.

| Concern | Canonical source | Documentation responsibility |
| --- | --- | --- |
| Workspace membership and versions | Root `Cargo.toml` and package manifests | Explain dependency and release relationships |
| Rust public API | Rust source and generated rustdoc | Explain semantics, invariants, examples, and performance |
| C ABI names and signatures | `crates/of_ffi_c/include/orderflow.h` | Explain ABI ownership, errors, layouts, and compatibility |
| C ABI binding exposure | `bindings/api_manifest.toml` and generated signatures | Explain exposure level and supported language facade |
| Python API | `bindings/python/orderflow/` | Explain Python lifecycle, exceptions, callbacks, and threading |
| Java API | `bindings/java/src/main/java/` | Explain JNA loading, lifecycle, exceptions, and threading |
| Serialized payloads | Codec implementations, schemas, and tests | Document fields, defaults, ordering, compatibility, and migration |
| Persistence layout | `of_persist`, `of_persist_parquet`, fixtures, and tests | Document durability, retention, replay, and recovery guarantees |
| Runtime behavior | `of_runtime` implementation and tests | Document lifecycle, concurrency, health, and failure semantics |
| Execution behavior | `of_execution*`, tests, and certification fixtures | Document state machines, routing, risk, reports, and recovery |
| Provider behavior | Adapter implementations and conformance tests | Document provider-specific limitations and guarantees |
| Release behavior | Changelog, release notes, workflows, and version manifests | Document compatibility, migration, and artifact availability |

## Conflict Resolution

When sources disagree, the documentation must not hide the conflict. The
correct process is:

1. Identify the authoritative source for the concern.
2. Add or correct a regression test if behavior is ambiguous.
3. Update the implementation or documentation deliberately.
4. Record compatibility impact and release version.
5. Link the affected reference and migration pages.
