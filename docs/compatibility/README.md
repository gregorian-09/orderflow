# Compatibility and Releases

Orderflow is an open-source library with multiple public surfaces. Compatibility
must be evaluated independently for Rust crates, the C ABI, bindings,
serialized data, and operational configuration.

## Current Version Families

| Surface | Current line | Compatibility note |
| --- | --- | --- |
| Existing runtime/analytics crates | `0.5.x` | Established market-data API family |
| Advanced analytics and FIX crates | `0.1.x` | Newer additive public surfaces with independent compatibility lines |
| Execution algorithms | `0.1.0` first release | Not previously published; current dependency changes do not consume a patch version |
| Execution core, OMS, and execution adapters | `0.2.x` | Additive expansion after the published `0.1.0` foundations |
| C ABI | `0.5.x` package line | Symbol and layout compatibility is explicit |
| Python binding | `0.5.x` package line | High-level methods preserve existing behavior |
| Java binding | `0.5.x` package line | JNA mappings and lifecycle remain compatible |
| Persistence schemas | Versioned per format | Readers preserve documented legacy formats |

The exact release state is determined by workspace manifests, binding version
files, release notes, and published artifacts. This table explains the model;
it is not a substitute for those authorities.

## Compatibility Surfaces

### Rust

Do not remove, rename, reorder, or change the meaning of an existing public
item. New behavior should be additive. Feature flags must not silently change
default behavior for existing users.

### C ABI

Existing function signatures, `repr(C)` field order/types, enum values, opaque
handle ownership, error codes, and buffer negotiation are ABI contracts. New
symbols are additive.

### Python and Java

Existing constructors, method names, exceptions, callback behavior, native
loading, and snapshot semantics remain stable. Generated low-level declarations
and manually designed high-level facades are validated separately.

### Persistence

Readers document and test legacy data. Writers add schema metadata or new
record versions without making old files unreadable. Migration is explicit,
deterministic, and separately verifiable from normal replay.

## Release Documentation Requirements

Every release needs what changed and why, package and binding versions, new
symbols and values, defaults, migration steps, persistence impact, performance
impact, validation evidence, known limitations, and deferred work.

## References

- [API reference](../reference/README.md)
- [Source-of-truth map](../knowledge-system/source-of-truth.md)
- [Release operations](../ops/README.md)
- [Contributor guide](../handbook/06-contributor-guide.md)
