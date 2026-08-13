# API Reference

This section will become the exhaustive symbol-level reference. Exact API
signatures remain generated or derived from source authorities; explanatory
pages add semantics, invariants, values, examples, and operational behavior.

## Reference Sources

| Surface | Exact authority | Current companion |
| --- | --- | --- |
| Rust | `crates/*/src/**/*.rs` and rustdoc | [Crate handbook references](../handbook/05-api-reference.md) |
| C ABI | `crates/of_ffi_c/include/orderflow.h` | [C binding guide](../bindings/c.md) |
| Python | `bindings/python/orderflow/` | [Python binding guide](../bindings/python.md) |
| Java | `bindings/java/src/main/java/` | [Java binding guide](../bindings/java.md) |
| ABI inventory | `bindings/api_manifest.toml` | [Generated inventory](../bindings/api-inventory.md) |

## Symbol Coverage Contract

For each public item, the final reference must identify:

- name and fully qualified path;
- kind: module, type, field, enum variant, trait, method, function, constant,
  static, error, feature, or configuration key;
- visibility and availability feature;
- exact type, layout, default, range, and sentinel values;
- ownership, borrowing, allocation, and lifetime behavior;
- thread-safety and reentrancy expectations;
- errors and invalid-input behavior;
- determinism and ordering guarantees;
- latency, blocking, and I/O characteristics;
- persistence, serialization, and migration implications;
- examples and related tests;
- introduction version and compatibility status.

The [coverage inventory](../knowledge-system/coverage-inventory.md) is the
machine-generated baseline. A declaration count is not completion: every item
must be semantically reviewed.

The [Rust public surface audit](./rust-surface.md) provides the item-by-item
source checklist and links each declaration to its owning file and line.

The [Rust values and layout audit](./rust-values.md) indexes public constants,
type aliases, struct fields, and enum variants with their declared types or
source values. Complex conditional declarations remain subject to source and
Rustdoc review.

## Generated References

The documentation build should eventually publish generated Rustdoc, C/Doxygen
reference, Python API reference, and JavaDoc alongside this manual. Generated
references are exact lookup surfaces; this portal remains the place for
cross-language concepts and complete workflows.

Rustdoc is already built as part of the documentation workflow with:

```text
cargo doc --workspace --all-features --no-deps
```

For a complete local artifact, use:

```text
bash tools/build_docs.sh /tmp/orderflow-docs-site
```

This refreshes the generated inventories, builds Rustdoc, builds the strict
MkDocs portal, and places Rustdoc under `reference/rust/` in the same output.

Read the Docs publishes the generated Rust reference under
`reference/rust/`. The source-level inventory remains necessary because it
tracks the audit scope, including constants, fields, variants, feature gates,
and compatibility notes that generated API pages do not organize by concept.

## Related Manuals

- [Foundations](../foundations/README.md)
- [Architecture](../architecture/README.md)
- [Bindings](../bindings/README.md)
- [Compatibility and release notes](../ops/README.md)
- [Workspace package and feature matrix](./package-matrix.md)
