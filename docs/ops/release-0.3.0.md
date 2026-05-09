# Release 0.3.0

Date: 2026-05-09

Orderflow `0.3.0` is a non-breaking operational hardening release. It keeps
the existing poll-driven developer API, Rust crate layering, C ABI, and
Python/Java binding surface stable while closing production-readiness gaps from
the broader project analysis.

## Version Decision

This release is versioned as `0.3.0` because:

- it adds multiple backward-compatible runtime and packaging capabilities
- it is materially larger than a patch release
- it does not intentionally remove or rename existing public APIs

Package versions for this release:

- Rust workspace / C ABI crates: `0.3.0`
- Python binding: `0.3.0`
- Java binding: `0.3.0`

## What Is New

### 1. Dashboard auth and Prometheus metrics

The dashboard now supports optional token authentication through
`OF_DASH_TOKEN`. Auth remains disabled by default for local development, so
existing dashboard workflows continue to work.

The dashboard also exposes a Prometheus-compatible `/metrics` endpoint with
runtime counters, quality flags, adapter status, backpressure counters, and
circuit-breaker state.

### 2. Runtime backpressure

Hosts that need explicit event-drain limits can now opt in with:

- Rust: `Engine::with_max_events_per_poll(Some(n))`
- Default engines: `OF_RUNTIME_MAX_EVENTS_PER_POLL=n`

Backpressure is disabled by default. When enabled and a poll drains more than
the configured limit, the runtime processes up to the limit, drops the excess
from that drain, marks the runtime degraded, and surfaces a backpressure error.

### 3. Adapter health aggregation and circuit breaking

The runtime now exposes additive aggregate health fields:

- `adapter_total_count`
- `adapter_healthy_count`
- `runtime_health_status`
- `circuit_breaker_enabled`
- `circuit_breaker_open`
- `circuit_breaker_consecutive_failures`
- `circuit_breaker_opened_count`
- `circuit_breaker_cooldown_ms`

Circuit breaking is opt-in:

- Rust: `Engine::with_circuit_breaker(failure_threshold, cooldown_ms)`
- Default engines: `OF_RUNTIME_CIRCUIT_BREAKER_FAILURES`
- Optional cooldown override: `OF_RUNTIME_CIRCUIT_BREAKER_COOLDOWN_MS`

This gives live hosts a way to stop reconnect or poll-failure storms without
changing the adapter trait contract.

### 4. Additive persistence schema metadata

New JSONL records include `"schema": 1`, `ts_exchange_ns`, and `ts_recv_ns`.
Legacy persisted records remain readable. This gives replay and migration tools
a stable place to reason about persisted record format without breaking older
data.

### 5. End-to-end replay parity regression

The runtime test suite now covers:

`runtime ingest -> persistence -> readback -> replay -> analytics/signal/book parity`

This validates that persisted book/trade streams replay into matching runtime
analytics, signal state, and materialized book state.

### 6. Python packaging improvements

The Python package now includes:

- a `py.typed` marker for PEP 561 type-checker support
- package data support for bundled native libraries under `orderflow/native/`
- native library lookup that checks bundled wheel libraries before local debug
  build paths
- CI workflow support for building platform-tagged binary wheels

Source installs and explicit `library_path=` / `ORDERFLOW_LIBRARY_PATH` loading
continue to work.

### 7. MSRV-compatible lockfile cleanup

The lockfile and dependency selections were kept compatible with the project's
minimum Rust toolchain expectations so offline CI and contributor builds remain
stable.

## How 0.3.0 Differs From 0.2.0

`0.2.0` focused on feature completeness: snapshots, readback, replay
discovery, signals, config hardening, and binding parity.

`0.3.0` focuses on operational readiness:

- dashboard protection and metrics
- runtime backpressure
- aggregate health reporting
- adapter circuit breaker policy
- persisted record schema metadata
- deterministic replay parity coverage
- Python typing and binary-wheel readiness

## What Existing Users Need To Do

Required migration:

- none

Recommended follow-up:

- update Rust/C/Python/Java packages and native libraries together to `0.3.0`
- enable `OF_DASH_TOKEN` if exposing the dashboard beyond a trusted local host
- scrape `/metrics` if you already run Prometheus or compatible monitoring
- consider `OF_RUNTIME_MAX_EVENTS_PER_POLL` for hosts that need bounded poll
  drains
- consider `OF_RUNTIME_CIRCUIT_BREAKER_FAILURES` for live adapter deployments

## Public APIs Added In 0.3.0

Rust/runtime:

- `RuntimeError::is_backpressure`
- `RuntimeError::is_circuit_open`
- `Engine::with_max_events_per_poll`
- `Engine::with_circuit_breaker`

Operational JSON fields:

- `adapter_total_count`
- `adapter_healthy_count`
- `runtime_health_status`
- `max_events_per_poll`
- `backpressure_dropped_events`
- `circuit_breaker_enabled`
- `circuit_breaker_open`
- `circuit_breaker_consecutive_failures`
- `circuit_breaker_opened_count`
- `circuit_breaker_cooldown_ms`

Python packaging:

- PEP 561 `py.typed`
- bundled native library search path: `orderflow/native/`

## Verification Status For This Release

Validated locally before finalizing the release notes:

- `cargo test -q --workspace --offline`
- `python3 -m py_compile dashboard/server.py tools/dashboard_smoke_test.py bindings/python/orderflow/_ffi.py bindings/python/orderflow/api.py`
- `python3 tools/docs_coverage.py --enforce`
- `git diff --check`
- `python3 tools/dashboard_smoke_test.py`

## Deferred Work

A true Tokio/async runtime migration remains intentionally deferred. Forcing it
into this release would risk breaking the current developer-facing poll API,
C ABI assumptions, and binding behavior. The safer migration path is a separate
major design branch with an additive async facade or worker before any adapter
trait redesign.

## Where To Read Next

- Changelog: [`CHANGELOG.md`](../../CHANGELOG.md)
- Runtime reference: [`docs/handbook/05e-of-runtime-reference.md`](../handbook/05e-of-runtime-reference.md)
- Release checklist: [`docs/ops/release_checklist.md`](release_checklist.md)
