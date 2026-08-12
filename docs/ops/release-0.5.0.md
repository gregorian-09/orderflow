# Release 0.5.0

Status: release candidate  
Date: TBD

Orderflow `0.5.0` is a non-breaking production-hardening release over the
published `0.4.0` line. It completes the planned OMS, FIX, execution-algorithm,
signal, adapter, and market-data persistence foundations while keeping
existing analytics, runtime, C ABI, Python, and Java call sites valid.

The project remains infrastructure for developers. Provider connectivity,
counterparty certification, credentials, deployment topology, operational
permissions, and live-capital approval remain host responsibilities.

## Version Model

| Package family | Version | Compatibility rule |
| --- | ---: | --- |
| `of_core`, `of_adapters`, `of_signals`, `of_persist`, `of_runtime`, `of_ffi_c` | `0.5.0` | Established line; additive APIs over `0.4.0` |
| Python `orderflow-gregorian09` | `0.5.0` | Install with matching native `of_ffi_c` |
| Java `orderflow-java-binding` | `0.5.0` | Install with matching native `of_ffi_c` |
| `of_analytics`, `of_execution_core`, `of_fix`, `of_execution`, `of_execution_algos`, `of_execution_adapters`, `of_persist_parquet` | `0.1.0` | Independent new public surfaces; pin compatible `0.1.x` versions |

The established crates must move to `0.5.0` because crates.io versions are
immutable and the new standalone crates require APIs added after the `0.4.0`
publication. Reusing `0.4.0` would make packaged path dependencies resolve to
older registry crates that do not contain those APIs.

The C ABI major returned by `of_api_version()` remains unchanged. `0.5.0` adds
symbols and opaque handle families; it does not reorder existing `repr(C)`
structures, remove exports, or alter established function signatures.

## Production Market-Data Persistence

`of_persist 0.5.0` adds a complete additive capture and recovery foundation:

- checksum-linked single-file and segmented normalized WALs;
- global sequence/checksum continuity across rotations;
- explicit segment seals and atomic rebuildable manifests;
- configurable record, cadence, and seal synchronization policies;
- cloneable nonblocking producers feeding one bounded writer owner;
- independent record-count, payload-byte, and payload-size bounds;
- ownership-preserving admission failures for pooled-buffer reuse;
- versioned `OFNE` canonical trade/book encoding with effective quality flags;
- event-time backlog and durability watermarks without admission-time clocks;
- provider-native raw evidence envelopes before normalization;
- bounded raw-capture writer, metadata, flags, timestamps, and replay;
- checkpoint storage with sequence anchors, checksums, and retention;
- deterministic recovery planning for clean, degraded, snapshot-gated, and
  fail-closed startup outcomes;
- replay filters by WAL/provider/event sequence, event/receive time, and kind;
- dependency-free JSONL cold export and conservative retention planning.

`of_runtime 0.5.0` can own or borrow the normalized WAL writer. Configuration,
flush, shutdown, health, failure policy, and trading-readiness gates are
explicit. The ingest hot path performs bounded ownership transfer; encoding,
filesystem I/O, and sync happen on the writer thread. Replay preserves the
quality state observed live and does not recursively journal replayed events.

C, Python, and Java expose the same lifecycle additively through configure,
flush, shutdown, and allocated health JSON APIs.

## Verified Parquet Cold Storage

New `of_persist_parquet 0.1.0` keeps Arrow and Parquet outside the capture hot
crate while adding:

- bounded Arrow batches and Parquet row groups;
- Zstandard, Snappy, and uncompressed output;
- date/venue/symbol/stream partitioning;
- source, adapter, session, sequence, timestamp, quality, kind, and original
  normalized payload columns;
- optional caller-versioned derived snapshots joined linearly by WAL sequence;
- strict normalized payload and partition identity validation;
- create-new temporary output and no-clobber same-directory publication;
- full reopen checks for SHA-256, file checksum, schema, rows, ranges, row
  groups, constant metadata, quality/derived counts, and payload checksums;
- retention evidence available only from a verified proof.

Parquet export is synchronous maintenance work. Compact sealed,
sequence-bounded WAL ranges on a dedicated worker, verify bytes after object
storage transfer, then ask the conservative retention planner whether the hot
range can be deleted.

## OMS And Execution Safety

`of_execution 0.1.0` now supplies production-oriented additive foundations for:

- deterministic multi-route and multi-symbol ownership;
- bounded concurrent command/report workers with one state owner;
- full-payload command WAL and backward-compatible legacy replay;
- checkpointed and checkpoint-free order reconstruction;
- command idempotency and source-scoped report deduplication;
- order intent plus bounded native parent/child lifecycle;
- scoped kill switches and deterministic cancellation targets;
- scoped production risk with position, exposure, PnL, price, rate, health,
  reduce-only, and fat-finger controls;
- authoritative position/PnL, fees, commissions, cash, corporate actions, and
  rational FX accounting;
- independent drop-copy ingest and reconciliation;
- generalized local/WAL/checkpoint/venue/drop-copy/position reconciliation;
- allocation, timestamp discipline, and safety-policy helpers;
- deterministic 18-scenario OMS certification venue;
- fixed-memory execution SLIs and explicit SLO evaluation;
- permission-ready idempotent operator runbook commands;
- bounded, checksummed, atomically published incident audit bundles.

All recovery modes remain fail closed until required evidence is valid and
venue reconciliation is complete. Simulation and certification evidence do
not imply broker or exchange approval.

## FIX Infrastructure And Execution Adapter

`of_fix 0.1.0` provides reusable transport-independent infrastructure:

- borrowed tag-value parsing and caller-owned encoding;
- strict `BodyLength(9)` and `CheckSum(10)` validation;
- FIX 4.2/4.4 message and dictionary/profile validation;
- Logon, Logout, Heartbeat, TestRequest, ResendRequest, SequenceReset, Reject,
  and BusinessMessageReject handling;
- deterministic inbound/outbound sequence state and snapshots;
- bounded in-memory and durable resend stores;
- possible-duplicate replay with preserved original sending time;
- typed new, cancel, replace, status, mass-cancel, and mass-status builders;
- bounded raw transcript evidence and diagnostics.

`of_execution_adapters 0.1.0` maps this infrastructure into canonical OMS
requests/reports through a synchronous, transport-injected FIX adapter. The
host owns sockets, TLS, credentials, scheduling, clocks, and certification.
The adapter owns bounded session state, resend/gap recovery, liveness,
working-order context, durable outbound intent, report mapping, and fixed-memory
metrics. A deterministic 13-scenario wire certification harness verifies exact
transcripts, capability evidence, recovery, backpressure, and failure behavior.

## Execution Algorithms

`of_execution_algos 0.1.0` provides fixed-capacity, deterministic planners for:

- TWAP and replay-stable TWAP validation;
- POV participation;
- historical-curve VWAP;
- synthetic iceberg replenishment;
- implementation shortfall;
- passive queue placement and optional late crossing;
- smart order routing;
- liquidity seeking;
- sweep/aggressive take;
- basket and spread execution;
- pairs/spread execution;
- market-making quote planning.

Planner output is a canonical child-order plan, not an exchange send. Hosts
must submit it through OMS risk, idempotency, journaling, and adapters, then
commit planner progress only after accepted submission.

## Signals And Advanced Analytics

`of_signals 0.5.0` retains the original `SignalModule` contract and adds
opt-in production surfaces:

- descriptors, parameter schemas, registries, and config validation;
- contextual inputs and a legacy-module adapter;
- warmup/lifecycle and run-mode state;
- hysteresis, debounce, cooldown, and transition policies;
- structured reason codes and explanations;
- outcome tracking, calibration reports, drift, and confidence calibration;
- majority, quorum, weighted, and veto ensembles;
- checkpoints, restore validation, shadow recording, and comparison;
- feature schema/model metadata binding;
- replay validation through Rust, C, Python, and Java.

`of_analytics 0.1.0` remains an optional heavy analytics home covering market
quality/TCA, liquidity/depth, impact, toxicity, volatility/noise, regime,
quality, resiliency, queue/fill, pattern risk, venue/route, cross-asset,
derivatives, and stable feature-vector extraction. This split prevents every
`of_core` user from compiling advanced modules.

## Market-Data Adapter Operations

`of_adapters 0.5.0` retains the `MarketDataAdapter` trait and adds:

- conservative quality/certification/production-observed descriptors;
- SDK conformance requirements and reports;
- defaulted typed operational status for third-party source compatibility;
- runtime mode, connection, session, queue, loss, freshness, raw capture, and
  subscription diagnostics;
- endpoint redaction that removes user info, path, query, and fragment;
- Binance update-id continuity, duplicate suppression, gap/snapshot state,
  queue bounds, raw capture, fixture replay, parse/normalization latency, and
  jittered reconnect backoff.

Built-in provider maturity labels remain evidence claims, not marketing labels.
Hosts should use descriptor and conformance output before enabling live modes.

## Binding And ABI Quality

The binding surface now uses one machine-readable API manifest to validate the
C header and native exports and to generate all low-level Python ctypes and Java
JNA signatures. High-level ownership, lifecycle, exceptions, buffer growth,
dataclasses, and Java records remain manually designed where generation would
reduce ergonomics or safety.

Additive binding features include:

- separate synchronous and concurrent execution handles;
- multi-route/multi-symbol construction;
- deterministic TWAP parent handles;
- offline signal config/replay validation;
- adapter inventory and active status;
- signal descriptor, explanation, and metrics JSON;
- market-data WAL lifecycle and persistence health;
- offline WAL/checkpoint recovery diagnostics.

Existing analytics `Engine`/`OrderflowEngine` methods retain their names and
signatures. Bindings and native libraries must be upgraded together.

## Toolchain And Supply Chain

- Declared all-feature MSRV: Rust `1.88.0`.
- The optional `tickbar` dependency now disables its Python default feature,
  removing unused PyO3 and macro/build dependencies.
- RustSec fixes pin `crossbeam-epoch >=0.9.20` and `memmap2 >=0.9.11` in the
  release lockfile.
- SHA-256 users share `sha2 0.11`, reducing duplicate crypto dependencies.
- `deny.toml` enforces reviewed licenses, crates.io-only registry sources, no
  git dependencies, no wildcard requirements, and no ignored advisories.
- CI tests the all-feature graph on Rust `1.88.0` and runs `cargo-deny`.

## Rust Publication Order

Crates are published in dependency order:

1. `of_core 0.5.0`
2. `of_analytics 0.1.0`
3. `of_execution_core 0.1.0`
4. `of_fix 0.1.0`
5. `of_signals 0.5.0`
6. `of_persist 0.5.0`
7. `of_persist_parquet 0.1.0`
8. `of_adapters 0.5.0`
9. `of_runtime 0.5.0`
10. `of_execution 0.1.0`
11. `of_execution_algos 0.1.0`
12. `of_execution_adapters 0.1.0`
13. `of_ffi_c 0.5.0`

The workflow waits until each successful publication is visible in the
crates.io index before publishing a dependent crate. CI performs locked
compilation and package-content inspection for every crate, plus full package
verification for the dependency root. Cargo performs full downstream package
verification during ordered publication once each new dependency version
exists in the registry.

## Upgrade Checklist

1. Upgrade established Rust crates together to `0.5.0`.
2. Upgrade Python/Java packages with the matching `0.5.0` native library.
3. Keep `of_api_version()` compatibility checks in host startup.
4. Explicitly select optional `0.1.x` crates; they are not added to established
   default feature sets.
5. Configure bounded WAL limits, failure action, sync policy, and health gates
   before enabling persistence in a live runtime.
6. Replay and reconcile persisted sessions before enabling execution after a
   restart.
7. Treat Parquet proof as local evidence only until uploaded bytes are
   independently verified.
8. Run provider/FIX/OMS certification suites against the actual venue profile
   and transport before paper or live deployment.
9. Run `cargo test --workspace --all-features`, the no-default matrix, Clippy,
   rustdoc, binding smoke tests, ABI export checks, semver checks, `cargo audit`,
   and `cargo deny check` before tagging.

## Explicit Non-Goals

`0.5.0` does not provide credentials, a universal exchange socket/TLS stack,
counterparty certification, smart production defaults, capital approval,
distributed leader election, object-store credentials, or a guarantee of
profitability. It provides deterministic, bounded, testable building blocks so
developers can implement those deployment-specific responsibilities without
forking core domain logic.
