# Release 0.4.0

Date: 2026-06-04

Orderflow `0.4.0` is a non-breaking analytics-to-execution expansion release.
It keeps existing market-data analytics APIs stable while adding a separate
execution and OMS foundation for developer-built order-management workflows.

## Version Decision

This release uses a split version model:

| Package family | Version |
| --- | ---: |
| Existing Rust crates: `of_core`, `of_adapters`, `of_signals`, `of_persist`, `of_runtime`, `of_ffi_c` | `0.4.0` |
| Python binding: `orderflow-gregorian09` | `0.4.0` |
| Java binding: `orderflow-java-binding` | `0.4.0` |
| New Rust execution/FIX crates: `of_execution_core`, `of_fix`, `of_execution`, `of_execution_adapters` | `0.1.0` |

The split is intentional. The analytics/runtime/binding stack is the existing
public package family. The execution/FIX crates are new public Rust surfaces and
should start at `0.1.0` so their traits, codec APIs, and adapter contracts can mature
honestly.

## What Is New

### 1. Execution core

`of_execution_core 0.1.0` adds:

- fixed-size ASCII identifiers for client, venue, execution, account, route,
  strategy, venue, instrument, and bounded text fields
- typed `OrderRequest`, `CancelRequest`, and `AmendRequest`
- integer-normalized quantity and price wrappers
- canonical side, order type, time-in-force, execution type, and order status
- strict order-state machine with validated transitions
- basic route-scoped risk limits and structured risk rejection reasons

### 2. Execution engine and OMS helpers

`of_execution 0.1.0` adds:

- `ExecutionAdapter` provider-neutral adapter trait
- `ExecutionEngine` synchronous deterministic owner
- `ConcurrentExecutionEngine` bounded worker for many producers and one native
  order-state owner
- route/account/symbol configuration and open-order risk accounting
- bounded `ExecutionEventBuffer`
- deterministic simulated execution adapter
- in-memory and file-backed execution journal helpers
- recovery and open-order reconciliation primitives
- lifecycle, fanout, command correlation, throttling, sharding, telemetry,
  position ledger, safety policy, and replay helpers

### 3. FIX codec foundation

`of_fix 0.1.0` adds a reusable low-allocation FIX tag-value codec foundation:

- borrowed `FixFieldView` and `FixMessageView` parsing from raw bytes
- caller-provided parse scratch buffers
- strict `BodyLength(9)` and `CheckSum(10)` validation
- common FIX tag constants and extraction helpers
- caller-owned encoding buffers with computed body length and checksum
- diagnostic rendering with `|` separators outside hot paths

This is not a full FIX session engine. Transport, logon/logout, resend, sequence
reset, persistence, venue profiles, and certification tooling remain separate
future layers built on top of the codec.

### 4. Execution adapter scaffolding

`of_execution_adapters 0.1.0` adds a feature-gated FIX scaffold:

- FIX-style session config
- normalized execution-report struct
- FIX exec type/status mapping
- canonical `ExecutionEvent` conversion
- fail-closed adapter shell

This is not a production FIX engine. It is a reusable mapping and adapter
authoring scaffold.

### 5. C, Python, and Java execution APIs

`of_ffi_c 0.4.0`, Python `0.4.0`, and Java `0.4.0` expose the execution layer
through additive handles/classes:

- C: `of_execution_engine_t`, `of_execution_engine_create_multi`,
  `of_execution_submit_order`, `of_execution_cancel_order`,
  `of_execution_amend_order`, `of_execution_concurrent_*`
- Python: `ExecutionEngine`, `ConcurrentExecutionEngine`, `OrderRequest`,
  `CancelRequest`, `AmendRequest`, `RiskLimits`, `RouteConfig`
- Java: `OrderflowExecutionEngine`, `ConcurrentOrderflowExecutionEngine`,
  `OrderRequest`, `CancelRequest`, `AmendRequest`, `RiskLimits`, `RouteConfig`

Existing analytics handles and classes remain separate and unchanged.

### 6. Documentation expansion

The documentation now teaches the full workflow:

- market-data ingest or replay
- analytics and signal snapshots
- data-quality gating
- route-scoped risk checks
- simulated execution
- concurrent execution worker usage
- order event handling
- journaling, recovery, reconciliation, and replay review
- provider adapter authoring boundaries
- low-latency design constraints

## Upgrade Notes

Required migration for existing analytics users:

- none expected

Recommended upgrade steps:

- update `of_core`, `of_adapters`, `of_signals`, `of_persist`, `of_runtime`,
  and `of_ffi_c` together to `0.4.0`
- update Python/Java packages and native `of_ffi_c` runtime/header together to
  `0.4.0`
- pin new execution crates to compatible `0.1.x` versions if building Rust
  execution providers
- keep market-data runtime and execution engine ownership explicit in your
  application architecture
- treat simulated execution as a deterministic development/test adapter, not as
  broker-certified live connectivity

## What Existing APIs Are Not Changed

The release does not intentionally remove or rename:

- existing market-data adapter trait methods
- runtime lifecycle, subscribe, poll, ingest, and snapshot methods
- existing C analytics/runtime symbols
- existing Python `Engine` analytics methods
- existing Java `OrderflowEngine` analytics methods
- existing persistence readback APIs
- existing signal trait and constructors

## Where To Read Next

- Root README: [`README.md`](../../README.md)
- Strategy design: [`docs/handbook/02-strategy-design.md`](../handbook/02-strategy-design.md)
- Strategy cookbook: [`docs/handbook/08-strategy-cookbook.md`](../handbook/08-strategy-cookbook.md)
- OMS architecture: [`docs/handbook/09-oms-architecture.md`](../handbook/09-oms-architecture.md)
- OMS cookbook: [`docs/handbook/10-oms-cookbook.md`](../handbook/10-oms-cookbook.md)
- Low-latency design: [`docs/handbook/11-low-latency-design.md`](../handbook/11-low-latency-design.md)
- Provider adapter authoring: [`docs/handbook/12-provider-adapter-authoring.md`](../handbook/12-provider-adapter-authoring.md)
- Changelog: [`CHANGELOG.md`](../../CHANGELOG.md)
