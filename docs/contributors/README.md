# Contributor and Documentation Workflow

The repository-wide coding and review contract is in the tracked root
[`CONTRIBUTING.md`](https://github.com/gregorian-09/orderflow/blob/main/CONTRIBUTING.md).
Read it first. This page focuses on the documentation-specific workflow and
generated reference maintenance.

Orderflow documentation is maintained with the same discipline as library
code. A change is incomplete when its public behavior, configuration, wire
format, or operational consequence is undocumented.

## Change Classification

Before editing, classify the change:

| Change | Required documentation |
| --- | --- |
| New Rust public item | Rustdoc, crate README if user-facing, reference page, example or test |
| New field or enum variant | Field/variant meaning, value/default, compatibility and serialization notes |
| New C symbol | Header docs, API manifest, generated signatures, ownership/error docs |
| New Python/Java facade | High-level docs, exception/error mapping, lifecycle and binding example |
| New feature flag | Feature matrix, default behavior, dependency/cost, CI combination |
| Persistence change | Schema/version, reader compatibility, migration and replay evidence |
| Runtime/OMS behavior | Lifecycle, concurrency, failure, recovery, metrics and latency notes |
| Provider adapter | Capability, certification status, credentials, reconnect, quality and limits |

## Source of Truth

Use the [source-of-truth map](../knowledge-system/source-of-truth.md). Do not
copy a signature into a narrative page and treat the copy as authoritative.
Generated inventories must be refreshed after source or manifest changes.

## Documentation Review

Reviewers should ask:

- Can a new user understand the concept before seeing the API?
- Are every field, value, default, sentinel, and error explained?
- Are ownership, allocation, blocking, thread-safety, and callback rules clear?
- Does the example compile and exercise a meaningful path?
- Does the page distinguish deterministic guarantees from implementation detail?
- Are version and migration implications stated?
- Are failure and recovery paths documented rather than only the happy path?

## Local Validation

Create the virtual environment once, then build the complete portal:

```bash
python3 -m venv .venv
.venv/bin/pip install --requirement docs/requirements.txt
bash tools/build_docs.sh /tmp/orderflow-docs-site
```

Run the repository gates:

```bash
python3 tools/docs_coverage.py --enforce
python3 tools/check_api_manifest.py
python3 tools/generate_api_inventory.py --check
cargo test --workspace --all-features
```

## Generated Files

The following committed files are generated from source and should be refreshed
through their tools rather than edited manually:

- `docs/knowledge-system/coverage-inventory.md`;
- `docs/reference/rust-surface.md`;
- `docs/reference/rust-values.md`;
- `docs/reference/package-matrix.md`;
- `docs/bindings/surface-audit.md`;
- `docs/bindings/api-inventory.md`.

## Compatibility Discipline

Do not remove or silently redefine an existing API, ABI field, binding method,
feature, environment variable, serialized field, or error value. Additive
changes need tests and documentation. Breaking changes require explicit
approval, migration guidance, and a release decision.
