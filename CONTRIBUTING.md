# Contributing to Orderflow

This is the tracked, public contributor contract for Orderflow. It replaces the
need for contributors to know about any local-only AGENTS.md file.

Orderflow is a multi-language library for normalized market-data analytics,
deterministic replay, signals, order management, execution algorithms, FIX
infrastructure, provider adapters, persistence, and C/Python/Java bindings.
Changes are reviewed as library changes: public behavior, compatibility,
documentation, operational behavior, and failure semantics all matter.

A contribution is complete only when:

1. the implementation is correct;
2. the public contract is documented;
3. existing consumers remain compatible;
4. normal and failure paths are tested;
5. generated references are refreshed;
6. performance and allocation consequences are understood;
7. the change is reviewable and reproducible.

## 1. Repository Snapshot

The workspace currently uses:

- Rust edition 2021;
- MSRV 1.88;
- workspace version 0.5.0;
- Cargo resolver 2;
- MIT license;
- format, lint, test, documentation, ABI, binding, and packaging gates.

### 1.1 Workspace crates

| Crate | Purpose |
| --- | --- |
| of_core | Canonical market-data types, integer-normalized values, accumulator, snapshots, and quality flags |
| of_analytics | Optional advanced market-quality, liquidity, TCA, toxicity, volatility, regime, queue, and feature analytics |
| of_signals | Signal traits, built-in signals, confidence, explanations, and quality gating |
| of_adapters | Market-data provider trait, normalization, health, runtime mode, and provider implementations |
| of_persist | Normalized event persistence, raw/normalized WAL, replay, retention, and recovery contracts |
| of_persist_parquet | Verified Parquet cold export and columnar readback outside the hot path |
| of_runtime | Engine lifecycle, polling, subscriptions, snapshots, supervision, health, and metrics |
| of_execution_core | Canonical order, execution event, risk, state-machine, fixed identifiers, and WAL frame primitives |
| of_execution | OMS routing, journals, idempotency, reconciliation, concurrency, safety controls, and position ledger |
| of_execution_algos | Deterministic parent/child execution planners, simulation, recovery, replay, and TCA |
| of_fix | Reusable FIX tag-value codec, validation, framing, sessions, sequencing, and resend infrastructure |
| of_execution_adapters | Venue execution bridges, FIX transport composition, provider profiles, and certification |
| of_ffi_c | Stable C ABI, opaque handles, buffers, callbacks, errors, and native exports |

### 1.2 Supporting directories

- bindings/python: ctypes low-level layer and Pythonic Engine facade.
- bindings/java: JNA low-level interface and Java AutoCloseable facade.
- dashboard: local live/replay dashboard and operational endpoints.
- examples: executable Rust examples and integration paths.
- tools: validation, code-generation, API audits, conformance, smoke tests,
  release checks, and documentation tooling.
- docs/handbook: explanatory and reference-oriented project knowledge.
- docs/reference: generated crate, Rust surface, values, package, and binding
  inventories.
- docs/contributors: documentation workflow and source-of-truth guidance.
- .github/workflows: CI, documentation, wheel, Maven, Rust, and native release
  automation.

### 1.3 Files contributors must know

This is the repository's file-level navigation map. A contributor does not need
to read every implementation file before making a small change, but must read
the owning contract and the relevant files below before editing behavior. All
paths are repository-relative.

| Area | Files | Contract |
| --- | --- | --- |
| Project entry point | `README.md`, `LICENSE`, `CHANGELOG.md`, `RELEASE_NOTES.md` | Project scope, license obligations, user-visible changes, and release history. |
| Workspace | `Cargo.toml`, `Cargo.lock`, `deny.toml` | Workspace members, versions, feature resolution, dependencies, and supply-chain policy. |
| Public contribution rules | `CONTRIBUTING.md` | Coding, compatibility, documentation, testing, security, and commit requirements. |
| Local-only context | `AGENTS.md` | May exist in a maintainer or agent environment, but is not a public project contract and must not be required to contribute. |
| Version and ABI manifests | `bindings/versions.toml`, `bindings/api_manifest.toml` | Binding version coordination and the C ABI surface that must remain compatible. |
| Core domain | `crates/of_core/Cargo.toml`, `crates/of_core/README.md`, `crates/of_core/src/lib.rs` | Normalized market-data types, accumulator state, snapshots, quality flags, and hot-path invariants. |
| Advanced analytics | `crates/of_analytics/Cargo.toml`, `crates/of_analytics/README.md`, `crates/of_analytics/src/lib.rs` | Optional liquidity, market-quality, TCA, toxicity, volatility, regime, queue, and feature analytics. |
| Market-data adapters | `crates/of_adapters/Cargo.toml`, `crates/of_adapters/README.md`, `crates/of_adapters/src/lib.rs`, `crates/of_adapters/src/cqg/`, `crates/of_adapters/src/binance.rs`, `crates/of_adapters/src/rithmic.rs` | Provider normalization, subscriptions, health, sequencing, reconnect, capabilities, and provider behavior. |
| Signals | `crates/of_signals/Cargo.toml`, `crates/of_signals/README.md`, `crates/of_signals/src/lib.rs` | Signal traits, built-in modules, confidence, explanations, validation, and quality gating. |
| Market-data persistence | `crates/of_persist/Cargo.toml`, `crates/of_persist/README.md`, `crates/of_persist/src/lib.rs`, `crates/of_persist/src/normalized_codec.rs`, `crates/of_persist/src/raw_capture.rs`, `crates/of_persist/src/market_data_writer.rs`, `crates/of_persist/src/market_data_segmented.rs` | Normalized history, raw capture, WAL/segments, replay ordering, retention, durability, and corruption handling. |
| Cold storage | `crates/of_persist_parquet/Cargo.toml`, `crates/of_persist_parquet/README.md`, `crates/of_persist_parquet/src/lib.rs` | Verified Parquet export and readback outside the hot path. |
| Runtime | `crates/of_runtime/Cargo.toml`, `crates/of_runtime/README.md`, `crates/of_runtime/src/lib.rs`, `crates/of_runtime/src/config.rs`, `crates/of_runtime/src/engine.rs`, `crates/of_runtime/src/tests.rs` | Lifecycle, polling, subscriptions, external ingest, snapshots, supervision, health, metrics, and persistence integration. |
| Execution domain | `crates/of_execution_core/Cargo.toml`, `crates/of_execution_core/README.md`, `crates/of_execution_core/src/lib.rs` | Canonical orders, commands, reports, states, risks, identifiers, and WAL frames. |
| OMS and execution | `crates/of_execution/Cargo.toml`, `crates/of_execution/README.md`, `crates/of_execution/src/lib.rs`, `crates/of_execution/src/oms.rs`, `crates/of_execution/src/order_intent.rs`, `crates/of_execution/src/reconciliation.rs`, `crates/of_execution/src/position_ledger.rs` | Order lifecycle, routing, idempotency, journals, recovery, reconciliation, positions, controls, and safety. |
| Execution algorithms | `crates/of_execution_algos/Cargo.toml`, `crates/of_execution_algos/README.md`, `crates/of_execution_algos/src/lib.rs` | Deterministic parent/child planning, TWAP and algorithm primitives, simulation, replay, and TCA. |
| FIX protocol | `crates/of_fix/Cargo.toml`, `crates/of_fix/README.md`, `crates/of_fix/src/lib.rs`, `crates/of_fix/src/session.rs` | Tag definitions, borrowed parsing, framing, validation, sequencing, resend, heartbeat, and sessions. |
| Execution adapters | `crates/of_execution_adapters/Cargo.toml`, `crates/of_execution_adapters/README.md`, `crates/of_execution_adapters/src/lib.rs`, `crates/of_execution_adapters/src/fix.rs`, `crates/of_execution_adapters/src/fix/live.rs`, `crates/of_execution_adapters/src/fix/certification.rs` | Venue profiles, transport composition, command/report mapping, capabilities, recovery, and certification. |
| C ABI | `crates/of_ffi_c/Cargo.toml`, `crates/of_ffi_c/README.md`, `crates/of_ffi_c/include/orderflow.h`, `crates/of_ffi_c/src/lib.rs`, `crates/of_ffi_c/src/support.rs`, `crates/of_ffi_c/src/tests.rs` | Stable native symbols, layouts, ownership, errors, buffers, callbacks, exports, and native tests. |
| Python binding | `bindings/python/README.md`, `bindings/python/pyproject.toml`, `bindings/python/orderflow/_ffi.py`, `bindings/python/orderflow/_generated_signatures.py`, `bindings/python/orderflow/api.py`, `bindings/python/tests/`, `bindings/python/examples/` | ctypes declarations, loading, high-level lifecycle, exceptions, buffer handling, examples, and compatibility. |
| Java binding | `bindings/java/README.md`, `bindings/java/pom.xml`, `bindings/java/src/main/java/com/orderflow/bindings/OrderflowNative.java`, `bindings/java/src/main/java/com/orderflow/bindings/OrderflowEngine.java`, `bindings/java/src/main/java/com/orderflow/bindings/OrderflowExecutionEngine.java`, `bindings/java/src/main/java/com/orderflow/examples/` | JNA signatures, lifecycle, exceptions, execution wrappers, examples, and Maven packaging. |
| Dashboard and deployment | `dashboard/README.md`, `dashboard/server.py`, `dashboard/static/index.html`, `Dockerfile`, `docker-compose.yml` | Live/replay operation, HTTP state/session behavior, and deployment defaults. |
| Replay and performance binaries | `examples/replay_cli/Cargo.toml`, `examples/replay_cli/src/main.rs`, `examples/perf_harness/Cargo.toml`, `examples/perf_harness/src/main.rs`, `docs/ops/performance.md` | Persistence discovery/replay and synthetic throughput, p99 latency, soak, and memory measurements. Both packages are `publish = false`. |
| Rust examples | `examples/` | Supported end-to-end Rust usage and integration paths. |
| Handbook entry points | `docs/handbook/README.md`, `docs/handbook/00-how-to-read.md`, `docs/handbook/01-orderflow-primer.md`, `docs/handbook/04-architecture.md` | Documentation navigation, domain vocabulary, and system boundaries. |
| API references | `docs/handbook/05-api-reference.md`, `docs/handbook/05a-of-core-reference.md` through `docs/handbook/05m-of-persist-parquet-reference.md` | Meaning and behavior of public Rust types, methods, fields, values, and compatibility rules. |
| Contributor and knowledge system | `docs/handbook/06-contributor-guide.md`, `docs/contributors/README.md`, `docs/knowledge-system/README.md`, `docs/knowledge-system/source-of-truth.md`, `docs/knowledge-system/portal-tree.md`, `docs/knowledge-system/documentation-charter.md`, `docs/knowledge-system/coverage-inventory.md` | Documentation workflow, fact ownership, portal structure, coverage, and generated-file policy. |
| Strategy and OMS guides | `docs/handbook/02-strategy-design.md`, `docs/handbook/08-strategy-cookbook.md`, `docs/handbook/09-oms-architecture.md`, `docs/handbook/10-oms-cookbook.md`, `docs/handbook/11-low-latency-design.md`, `docs/handbook/13-recovery-and-operations.md` | Concepts, complete workflows, execution architecture, latency constraints, recovery, and operations. |
| Adapter and FIX guides | `docs/handbook/12-provider-adapter-authoring.md`, `docs/fix/README.md`, `docs/execution/README.md`, `docs/execution-algorithms/README.md` | Adapter implementation, FIX infrastructure, execution bridges, and algorithm contracts. |
| Binding and compatibility guides | `docs/bindings/README.md`, `docs/bindings/c.md`, `docs/bindings/python.md`, `docs/bindings/java.md`, `docs/bindings/end-to-end.md`, `docs/bindings/api-inventory.md`, `docs/bindings/surface-audit.md`, `docs/compatibility/README.md` | Cross-language parity, loading, lifecycle, examples, API coverage, and compatibility audits. |
| Diagram sources | `docs/handbook/assets/diagrams/src/`, `docs/handbook/assets/diagrams/svg/`, `docs/handbook/assets/diagrams/png/` | Mermaid sources are authoritative; SVG and PNG files are rendered distribution artifacts. |
| Validation tools | `tools/check_ffi_exports.sh`, `tools/check_docs.sh`, `tools/docs_coverage.py`, `tools/provider_conformance.py`, `tools/dashboard_smoke_test.py`, `tools/smoke_python_binding.py` | ABI, documentation, provider, dashboard, and Python validation. |
| Generated-reference tools | `tools/generate_docs_inventory.py`, `tools/generate_rust_surface.py`, `tools/generate_rust_values.py`, `tools/generate_binding_surface.py`, `tools/generate_package_matrix.py`, `tools/generate_crate_pages.py`, `tools/enrich_api_reference.py`, `tools/enrich_handbook_public_lists.py` | Regenerates committed inventories and reference pages from source. |
| Binding and release automation | `tools/generate_binding_signatures.py`, `tools/check_api_manifest.py`, `tools/check_binding_parity.py`, `tools/release/`, `.github/workflows/` | Synchronizes native declarations and automates CI, packaging, documentation, and releases. |

When a row names a directory, inspect its README, manifest, public module root,
tests, and relevant examples before changing behavior. When a row names a
generated file, edit its source inputs and run the owning generator rather than
editing the generated output directly.

## 2. Repository Orientation

Read the relevant documentation before editing:

1. README.md for project scope and entry points;
2. docs/handbook/00-how-to-read.md for documentation navigation;
3. docs/handbook/04-architecture.md for system boundaries;
4. docs/handbook/06-contributor-guide.md for detailed engineering workflow;
5. docs/handbook/12-provider-adapter-authoring.md for provider work;
6. docs/handbook/13-recovery-and-operations.md for recovery and deployment;
7. the owning crate README and generated reference page.

The source-of-truth map is at docs/knowledge-system/source-of-truth.md.

### 2.1 Source-of-truth rules

- Rust implementation and rustdoc own Rust signatures and invariants.
- orderflow.h and the ABI manifest own C declarations.
- Python low-level declarations and Java JNA declarations must match the C ABI.
- High-level Python and Java wrappers own language-specific lifecycle and
  exception behavior.
- Cargo manifests own package versions and feature declarations.
- Generated inventories are regenerated by tools, never hand-maintained.
- Handbook prose explains concepts and consequences, not authoritative exact
  signatures.
- Provider documentation and certification evidence own provider assumptions.
- Persisted schema definitions own record compatibility and migration rules.

## 3. Branch and Working-Tree Discipline

Before editing:

~~~bash
pwd
git status --short --branch
git log -5 --oneline --decorate
git branch --show-current
~~~

Do not discard changes that you did not create. Do not use destructive commands
such as git reset --hard or git checkout -- to remove unrelated work.

Use a focused branch for a non-trivial change:

~~~bash
git switch -c docs/provider-adapter-guide
~~~

Keep generated build output, credentials, local plans, captures, and IDE state
out of commits. Existing local exclusions include target, .venv, Java targets,
Python caches, logs, data_capture, internal, plan.md, and new_features.md.

A clean working tree is not required before reading code, but the final diff must
contain only the intended change.

## 4. Change Classification

| Change | Primary owner | Required evidence |
| --- | --- | --- |
| Trade, book, or accumulator behavior | of_core | Deterministic arithmetic and state tests |
| Advanced analytics | of_analytics | Input, window, quality, numerical, and empty-state tests |
| Signal behavior | of_signals | Transition, confidence, and quality-gate tests |
| Market-data provider | of_adapters | Provider conformance, reconnect, sequence, and capability tests |
| Persistence or replay | of_persist | Schema, round-trip, corruption, retention, and replay tests |
| Cold columnar export | of_persist_parquet | Partition, schema, compression, and readback tests |
| Runtime orchestration | of_runtime | Lifecycle, polling, health, backpressure, and integration tests |
| OMS behavior | of_execution_core or of_execution | State, risk, idempotency, journal, recovery, and ledger tests |
| Execution algorithm | of_execution_algos | Allocation, deterministic replay, and recovery tests |
| FIX protocol | of_fix | Frame, tag, sequence, resend, malformed-input, and session tests |
| Venue execution bridge | of_execution_adapters | Capability, report, recovery, certification, and transcript evidence |
| C ABI | of_ffi_c | Header, manifest, export, buffer, ownership, and native tests |
| Python/Java API | bindings/* | Parity, lifecycle, error, and smoke tests |
| Dashboard | dashboard | Endpoint, replay, state, and security tests |
| Documentation | docs, tools | Generator checks and strict MkDocs build |

Do not place provider protocol logic in of_core, strategy logic in adapters, or
binding-specific behavior in Rust domain crates.

## 5. Rust Style

### 5.1 General rules

Use standard formatting and idioms:

~~~bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
~~~

Prefer:

- clear ownership over unnecessary cloning;
- small functions with one responsibility;
- explicit types at FFI and numeric boundaries;
- checked arithmetic for external inputs;
- Result for recoverable failures;
- typed errors over string matching;
- deterministic iteration when output is observable;
- bounded collections for provider-controlled or hot-path data;
- comments that explain why, not obvious narration.

Avoid:

- hidden global mutable state;
- unwrap or expect on provider-controlled data;
- unchecked integer conversions;
- floating-point state in normalized financial calculations;
- allocations in measured hot paths without evidence;
- silent fallback from live to mock or replay mode;
- unrelated refactors in feature commits.

### 5.2 Crate-level documentation

Each crate should use the existing pattern:

~~~rust
//! Short crate description.
#![doc = include_str!("../README.md")]
~~~

Cargo metadata should use workspace inheritance. Docs.rs metadata should enable
all relevant features and preserve the project documentation configuration.

### 5.3 Public item documentation

Every public item must define meaning and behavior. Document:

- what the type represents;
- why it exists;
- every field and unit;
- defaults and sentinel values;
- enum variants and discriminants where relevant;
- method inputs and outputs;
- mutation and ownership;
- allocation and blocking behavior;
- errors and recovery;
- thread-safety and callback rules;
- compatibility and serialization consequences;
- an executable example when construction is non-trivial.

### 5.4 Structs, enums, and traits

Document every public struct field and every enum variant. Explain the lifecycle
meaning of each value and the safety consequence of using it.

Never reorder public repr(C) fields or numeric ABI enum values. Never change the
meaning of an existing variant.

Traits must document implementor obligations, method ordering, thread bounds,
blocking behavior, error semantics, and whether callbacks may re-enter the
implementation.

### 5.5 Errors

Use typed errors when callers need to distinguish behavior. Document whether an
error means no side effect, possible side effect, failed durability, safe retry,
required reconciliation, degraded connection, or local policy rejection.

## 6. Numeric, Time, and Determinism Rules

Prices and quantities use integer-normalized values in the core and execution
domains. Keep floating point at presentation or explicitly statistical edges.

For every conversion:

- state source and destination units;
- state scale and denominator;
- define rounding direction;
- check overflow and underflow;
- reject unrepresentable values;
- test zero, minimum, maximum, negative, and boundary values;
- preserve tick size, lot size, multiplier, and currency metadata.

Timestamps must retain meaning:

- exchange timestamp;
- provider timestamp;
- local receive timestamp;
- monotonic latency timestamp;
- journal append timestamp;
- state-application timestamp.

Do not substitute one for another without a documented quality consequence.
Use monotonic time for elapsed durations and UTC/provider formats for wire fields.

Deterministic code must not depend on wall-clock reads, random identifiers,
network response timing, global mutable state, unordered observable iteration,
locale-dependent parsing, or hidden environment values.

## 7. Low-Latency and Allocation Rules

Before adding code to a hot path, answer:

1. Does it allocate?
2. Can it grow a collection?
3. Does it lock?
4. Can it block?
5. Does it perform file or network I/O?
6. Does it read a clock?
7. Does it format strings?
8. Is work bounded by a configured maximum?
9. Does it clone or copy a large frame?
10. What happens when the bound is reached?

Move serialization, checkpoint writes, metrics export, reconciliation, transcript
rendering, diagnostics, schema migration, and cold Parquet export to explicit
control-plane methods when possible.

A performance claim must include workload, hardware, build profile, allocator,
clock source, warm-up, sample count, p50, p95, p99, maximum, allocation count,
and queue behavior. “Low latency” alone is not evidence.

## 8. Market-Data Adapter Rules

Market-data adapters in of_adapters translate provider events into RawEvent
values for of_runtime.

They must:

- preserve provider sequence and timestamps;
- normalize price and quantity exactly;
- map provider sides and book actions explicitly;
- define snapshot/delta ordering;
- define duplicate and out-of-order handling;
- bound raw queues and frame sizes;
- report health and operational status;
- implement reconnect and resubscription;
- expose actual depth and capability;
- distinguish mock, live, replay, bridge, and unknown mode.

Use SubscribeReq for canonical symbol/depth requests. If provider depth is lower
than requested, reject, clamp with visible degradation, or document actual
depth. Never claim unavailable depth.

Sequence policy must be explicit: preserve, reject, flag, or bounded reorder.
Reordering needs maximum holdback, timeout, memory bound, and overflow behavior.

Read docs/handbook/12-provider-adapter-authoring.md for the complete adapter
authoring manual.

## 9. Execution Adapter Rules

Execution adapters implement of_execution::ExecutionAdapter and translate
canonical order commands into provider operations.

The contract includes connect, submit, cancel, amend, poll,
recover_open_orders, capabilities, and health.

The adapter must not own authoritative order state. The OMS owns idempotency,
journal state, report deduplication, state transitions, reconciliation, position
and PnL ledger, and operator controls.

A successful send is not an acknowledgement. A successful cancel request is not
proof of cancellation. An uncertain command must be reconciled before retry.

## 10. FIX Rules

of_fix owns reusable protocol infrastructure:

- tag definitions;
- borrowed parsing;
- frame validation;
- BodyLength(9) and CheckSum(10);
- sequence tracking;
- resend and gap fill;
- logon, heartbeat, test request, and logout;
- caller-owned encoding;
- bounded resend stores.

of_execution_adapters::fix owns venue profile, execution request mapping,
execution report mapping, live transport composition, certification scenarios,
and provider capabilities.

Use FixFrameTransport, FixTimeSource, and FixOutboundJournal. Do not implement a
local parser or duplicate generic session sequencing in a venue adapter.

## 11. OMS and Recovery Rules

The canonical execution flow is:

~~~mermaid
flowchart LR
    Command[Order command] --> Idempotency[Identity and idempotency]
    Idempotency --> Risk[Risk and safety]
    Risk --> Journal[Durable command record]
    Journal --> Adapter[Provider adapter]
    Adapter --> Report[Canonical report]
    Report --> Dedup[Report deduplication]
    Dedup --> State[Order state machine]
    State --> Ledger[Position and PnL ledger]
    State --> Reconcile[Venue reconciliation]
~~~

State transitions must be explicit. Duplicates must not double-apply. Unknown
outcomes must remain recoverable. Risk must fail closed when required context is
missing. Recovery must reconcile provider state before retrying uncertainty.

ExecutionEngine is the deterministic state owner. Concurrent wrappers may accept
commands from multiple producers, but state mutation remains serialized.

## 12. C ABI Rules

The C ABI is a public compatibility surface.

Preserve exported symbol names, calling convention, repr(C) field order and
types, alignment, ownership, enum/error numeric values, opaque handle lifecycle,
null-pointer behavior, invalid-handle behavior, buffer negotiation, string
termination, and callback lifetime/thread behavior.

Add a new symbol instead of changing an existing signature. Add a new struct
instead of changing an existing layout. Update the Rust export, orderflow.h,
API manifest, generated signatures, Python declarations, Java declarations,
export check, native tests, API inventory, and documentation together.

For caller-owned output buffers:

1. validate pointers and handle;
2. compute required size;
3. return BufferTooSmall with required capacity;
4. write payload and terminator on success;
5. return stable error code.

Never unwind across the C ABI.

## 13. Python and Java Binding Rules

Python has a low-level ctypes layer and a high-level Engine facade. The low-level
layer owns library loading, structures, argtypes, restype, pointers, and error
codes. The high-level layer owns Pythonic names, context manager behavior,
closed-engine checks, exception mapping, buffer retry, and user-facing values.

Java has a JNA interface and AutoCloseable facade. The low-level interface owns
exact native signatures. The high-level wrapper owns lifecycle, native-path
loading, Java exceptions, buffer growth, and user-facing objects.

Use exact native widths and size types. Document callback thread and lifetime
behavior. Test loading, close, double-close, buffer growth, invalid handles,
errors, callbacks, and examples.

Do not change existing exception classes or method semantics silently.

## 14. Persistence and Schema Rules

For every persistence change define schema version, record type, required and
optional fields, units, defaults, unknown-field behavior, ordering, sequence
guarantees, atomicity, durability, corruption recovery, retention, and migration.

of_persist remains suitable for normalized history and replay. Parquet is a
cold export boundary and must not make the hot path depend on columnar storage.

Test empty streams, round trips, bounded reads, gaps, duplicates, interrupted
writes, truncated tails, old fixtures, and deterministic replay after restore.

## 15. Documentation Rules

Documentation is part of the API. For every user-visible change update the
applicable crate README, rustdoc, handbook page, C header documentation,
Python/Java README, examples, changelog/release notes, and generated inventory.

When listing a public type, method, class, enum, struct, constant, or feature,
define what it means and what it does. Explain fields, units, values, defaults,
ownership, errors, allocation, blocking, threads, and compatibility.

Use Mermaid for lifecycle, dependency, and data-flow diagrams where a diagram
makes the contract clearer. Do not use diagrams to hide missing explanations.

Run:

~~~bash
python3 tools/docs_coverage.py --enforce
python3 tools/generate_docs_inventory.py --check
python3 tools/generate_rust_surface.py --check
python3 tools/generate_rust_values.py --check
python3 tools/generate_binding_surface.py --check
python3 tools/generate_package_matrix.py --check
python3 tools/generate_crate_pages.py --check
python3 tools/enrich_api_reference.py --check
python3 tools/enrich_handbook_public_lists.py --check
.venv/bin/mkdocs build --strict --site-dir /tmp/orderflow-docs-check
~~~

Generated files must be refreshed through tools, never edited manually.

## 16. Testing Rules

Use the narrowest test first, then expand:

~~~bash
cargo test -p <changed-crate>
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
~~~

Native and binding checks:

~~~bash
cargo build -p of_ffi_c
./tools/check_ffi_exports.sh target/debug/libof_ffi_c.so
python3 tools/check_api_manifest.py
python3 tools/generate_binding_signatures.py --check
python3 tools/test_generate_binding_signatures.py
python3 tools/check_binding_parity.py
python3 tools/generate_api_inventory.py --check
PYTHONPATH=bindings/python python3 -m unittest discover -s bindings/python/tests -v
mvn -q -f bindings/java/pom.xml test
~~~

Provider checks:

~~~bash
cargo test -p of_adapters --features rithmic
cargo test -p of_adapters --features cqg
cargo test -p of_adapters --features "cqg cqg_proto"
cargo test -p of_adapters --features binance
python3 tools/provider_conformance.py --help
~~~

Tests should cover normal, empty, minimum, maximum, invalid, duplicate,
out-of-order, capacity, backpressure, disconnect, reconnect, timeout,
uncertain outcome, reset, shutdown, replay, and public error mapping.

Use bounded deadlines for worker and callback tests. Stop and join owned workers.
Recover poisoned test locks so the primary failure is not hidden.

## 17. CI and Feature Matrix

A new feature is incomplete until CI compiles and tests it.

Update Cargo feature wiring, documentation matrices, CI workflows, and packaging
where needed. The default feature set must remain intentionally small.

Test default features, no default features, each feature alone, supported
combinations, all features, and MSRV.

CI covers supply chain, MSRV, semver, adapter features, workspace tests, C ABI
manifests and exports, Python, Java, documentation coverage, and generated
inventories.

## 18. Security Rules

Treat provider messages, configuration, persisted files, dashboard input, and C
pointers as untrusted. Validate lengths, enum values, timestamps, sequences,
symbols, prices, quantities, capacities, encodings, and pagination.

Never commit credentials, API keys, tokens, private certificates, unredacted
provider transcripts, customer data, or restricted specifications.

Do not log secrets. Redact endpoints, URLs, query strings, paths, user names,
listen keys, and session tokens. Do not disable TLS certificate validation in
defaults or examples.

## 19. Commit Style

Use Conventional Commits:

~~~text
feat(crate): add additive capability
fix(crate): correct deterministic transition
docs: explain public lifecycle contract
test(crate): cover reconnect sequence gap
refactor(crate): simplify non-public implementation
chore: refresh generated inventories
ci: validate feature matrix
~~~

Keep commits atomic. One commit should represent one coherent implementation
and its directly related tests and docs. Do not mix unrelated formatting,
release publishing, generated churn, or refactors into a feature commit.

Before committing:

~~~bash
git diff --check
git status --short
git diff --stat
git diff -- <relevant files>
~~~

## 20. Pull Request Checklist

- [ ] Correct crate and ownership boundary selected.
- [ ] Source-of-truth files were updated.
- [ ] Existing Rust APIs remain compatible.
- [ ] Existing C symbols and layouts remain compatible.
- [ ] Existing Python and Java behavior remains compatible.
- [ ] Serialization and feature behavior remain compatible.
- [ ] Public items explain meaning and behavior.
- [ ] Error, ownership, blocking, allocation, and thread rules are documented.
- [ ] Hot-path consequences were reviewed.
- [ ] Normal and failure paths are tested.
- [ ] Recovery and uncertain outcomes are tested.
- [ ] Generated references are refreshed.
- [ ] Strict docs build passes.
- [ ] No secrets or local artifacts are included.
- [ ] Commits are atomic and conventional.
- [ ] Pull request explains compatibility and operational impact.

## 21. Definition of Done

A fresh contributor should be able to use this file and the handbook to:

1. locate the correct crate;
2. understand the owning contract;
3. implement a compatible change;
4. run focused and full validation;
5. update every affected document;
6. explain failure and recovery behavior;
7. produce a small, reviewable commit.

If a contributor must infer a public contract from implementation details, the
change or its documentation is not finished.
