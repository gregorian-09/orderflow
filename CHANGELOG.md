# Changelog
All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]
### Added
- Additive `of_signals` metadata and lifecycle APIs:
  `SignalDescriptor`, `SignalInputMask`, `SignalWarmupRequirement`,
  `SignalWarmupProgress`, `SignalLifecycleState`, `SignalLifecycle`,
  signal parameter metadata, output semantics, built-in descriptor constants,
  `built_in_signal_descriptors()`, and `describe_signal()`. Existing
  `SignalModule` implementations and built-in signal behavior are unchanged.
- Additive `of_signals` contextual API with `SignalContext`,
  `ContextualSignalModule`, and `LegacySignalAdapter`, allowing richer signal
  hosts to pass analytics, data quality, symbol/book references, timestamps,
  lifecycle state, and extension tags without changing `SignalModule`.
- Additive `of_signals` stabilization policies with `HysteresisPolicy`,
  `DebouncePolicy`, `CooldownPolicy`, `SignalStabilizer`, and
  `StabilizedSignal` so hosts can opt into deterministic anti-flapping behavior
  without changing built-in signal outputs.
- Additive `of_signals` explainability APIs with `SignalReasonCode`,
  `SignalExplanation`, `SignalInputValue`, `SignalThreshold`,
  `SignalConfidenceComponent`, `SignalExplanationMode`, and
  `ExplainableSignalModule`. Built-in signals now expose structured diagnostic
  explanations while existing `SignalModule` and `SignalSnapshot` contracts are
  unchanged.
- Additive `of_signals` registry/config APIs with `SignalRegistry`,
  `SignalRegistration`, `SignalConfig`, typed config parameters,
  `SignalRegistryError`, built-in registrations, config validation,
  built-in construction, input filtering, and dependency-free descriptor JSON
  export for bindings and dashboards.
- Additive `of_signals` replay validation harness with `SignalValidationConfig`,
  `SignalValidationHarness`, `SignalValidationReport`, markout labels,
  optional timestamp-order warnings, confidence filtering, retained samples, and
  dependency-free JSON summaries for notebook/Python-style workflows.

### Fixed
- Hardened `of_ffi_c` native ABI tests so a failed test does not poison the
  shared FFI test lock and cascade misleading `PoisonError` failures through
  unrelated tests.

## [0.4.0] - 2026-06-04
### Added
- Broad additive analytics expansion across Rust, C, Python, and Java:
  spread/book-event/resiliency helpers; VPIN, Kyle's Lambda, Amihud, and CVD
  enhancement snapshots; practitioner pattern detection; volatility, noise,
  Hasbrouck, Almgren-Chriss, spread decomposition, ACD, regime, kinetic-energy,
  agent-type, dark-pool, options-flow, futures, dark-lit correlation,
  institutional-flow, OI-analysis, and LOB-feature APIs.
- `AnalyticsConfig` tuning surface across Rust, C, and Python for rolling
  window lengths, thresholds, and event-rate windows.
- Tickbar integration: fixed-interval OHLCV bar aggregation using the `tickbar`
  crate. New `CompletedBar` domain type, `AnalyticsAccumulator::with_tickbar()`,
  `bar_series()`, and `reset_tickbar()` methods. Exposed through C ABI
  (`of_engine_set_tickbar_interval`, `of_get_bar_series`), Python binding
  (`set_tickbar_interval()`, `bar_series()`), and Java binding
  (`setTickbarInterval()`, `barSeries()`). Feature-gated behind `tickbar`
  (off by default).
- Strategy cookbook expanded to 30 strategy examples with payload-key-accurate
  examples and API compatibility mapping across Rust, C, Python, and Java.
- Additive execution-core foundation for order-management workflows:
  `of_execution_core`, `of_execution`, and `of_execution_adapters` introduce
  fixed-size execution identifiers, typed order/cancel/amend requests,
  FIX-style order-state transitions, structured pre-trade risk rejection,
  bounded execution event buffers, simulated execution, journal/recovery hooks,
  and a FIX execution-report mapping scaffold.
- Additive execution C ABI, Python, and Java APIs using separate execution
  handles (`of_execution_engine_t`, `ExecutionEngine`,
  `OrderflowExecutionEngine`). Existing analytics/runtime APIs are unchanged.
- Multi-route execution engines for multi-symbol order flow. Rust execution now
  indexes route/account/symbol configs, applies route-scoped open-order and
  notional risk accounting, and exposes additive C (`of_execution_engine_create_multi`),
  Python (`ExecutionEngine([routes...])`), and Java
  (`new OrderflowExecutionEngine(path, List<RouteConfig>)`) entry points.
- Additive Rust `ConcurrentExecutionEngine` worker wrapper for concurrent order
  producers. The wrapper keeps the synchronous execution engine owned by one
  dedicated thread, uses bounded command/report channels, and preserves serial
  deterministic order-state transitions.
- Full additive OMS support primitives: command/request correlation, bounded
  execution-event fanout, adapter lifecycle snapshots, file-backed durable
  execution journal, open-order reconciliation, disconnect/kill-switch safety
  policies, advanced risk gate helpers, position/fill ledger, order-type/TIF
  normalization, execution telemetry, deterministic sharding, token-bucket
  throttling, replayable OMS simulation, and provider adapter SDK helpers.
- Concurrent execution runtime access across C, Python, and Java through
  additive handles/classes (`of_execution_concurrent_engine_t`,
  `ConcurrentExecutionEngine`, and `ConcurrentOrderflowExecutionEngine`).
- Expanded handbook coverage for the execution and OMS subsystem, including
  crate references, OMS architecture, multi-symbol cookbook workflows,
  low-latency design guidance, provider adapter authoring, and recovery
  operations.

### Changed
- Existing analytics/runtime/native/binding package line advances to `0.4.0`.
- New Rust execution crates publish as `0.1.0`:
  `of_execution_core`, `of_execution`, and `of_execution_adapters`.
- Root README, crate READMEs, Python README, Java README, strategy handbook,
  and OMS cookbook now focus on the `0.4.0` analytics-to-execution workflow.

### Upgrade Notes
- Existing analytics/runtime APIs are intended to remain source-compatible.
- Update Python/Java packages and the native `of_ffi_c` runtime/header together
  to `0.4.0`.
- If you build native execution providers, depend on the execution crates at
  compatible `0.1.x` versions and follow adapter-authoring guidance.
- Treat simulated execution as a deterministic test and integration tool, not a
  broker-certified live OMS.

## [0.3.0] - 2026-05-09
This is a non-breaking operational hardening release after `0.2.0`. For the
complete user-facing release guide, see
[`docs/ops/release-0.3.0.md`](docs/ops/release-0.3.0.md).

### Added
- Optional dashboard token authentication through `OF_DASH_TOKEN`, disabled by
  default for local development compatibility.
- Dashboard Prometheus `/metrics` endpoint with runtime counters, quality flags,
  adapter status, backpressure counters, aggregate health, and circuit-breaker
  state.
- Runtime opt-in backpressure with `Engine::with_max_events_per_poll(Some(n))`
  and `OF_RUNTIME_MAX_EVENTS_PER_POLL`.
- Runtime aggregate adapter health fields in health/metrics JSON:
  `adapter_total_count`, `adapter_healthy_count`, and
  `runtime_health_status`.
- Runtime opt-in adapter circuit breaker with
  `Engine::with_circuit_breaker(...)`,
  `OF_RUNTIME_CIRCUIT_BREAKER_FAILURES`, and
  `OF_RUNTIME_CIRCUIT_BREAKER_COOLDOWN_MS`.
- Additive circuit-breaker health/metrics fields:
  `circuit_breaker_enabled`, `circuit_breaker_open`,
  `circuit_breaker_consecutive_failures`, `circuit_breaker_opened_count`, and
  `circuit_breaker_cooldown_ms`.
- Additive JSONL persistence record metadata: `"schema": 1`,
  `ts_exchange_ns`, and `ts_recv_ns`.
- Runtime end-to-end persistence replay parity regression covering ingest,
  persistence, readback, replay, analytics, signals, and materialized book
  state.
- Python PEP 561 marker (`py.typed`) for type-checker support.
- Python bundled native library lookup under `orderflow/native/`.
- GitHub Actions workflow for platform-tagged Python binary wheels.

### Changed
- Package versions are aligned at `0.3.0` for Rust, C, Python, and Java.
- Python publish workflow now builds an sdist plus a Linux platform wheel with
  the native `of_ffi_c` runtime staged inside the package.
- Cargo lockfile dependency selections were kept compatible with the project's
  minimum supported Rust/Cargo toolchain.

### Upgrade Notes
- This release is additive and non-breaking for existing `0.2.0` integrations.
- Existing Rust, C, Python, and Java APIs continue to work; no required
  rename/removal migration is needed.
- If you use direct C or native bindings, update the native library and header
  together.
- If you use Python or Java, keep the binding package and native library on the
  same release version.
- Consider enabling `OF_DASH_TOKEN` before exposing the dashboard outside a
  trusted local environment.
- Consider enabling backpressure and circuit breaking for live deployments that
  need explicit failure containment.

## [0.2.0] - 2026-04-08
This is the first hardening-focused feature release after the initial `0.1.x`
line. For the complete user-facing release guide, see
[`docs/ops/release-0.2.0.md`](/home/gregorian-rayne/RustroverProjects/orderflow/docs/ops/release-0.2.0.md).

### Added
- Rust crate front-page documentation (`//!`) for `of_core`, `of_signals`,
  `of_persist`, `of_adapters`, `of_runtime`, and `of_ffi_c`, including
  purpose, architecture notes, and quick-start examples.
- C ABI API documentation comments for exported `of_ffi_c` symbols and FFI
  structs to improve docs.rs discoverability for non-Rust integrators.
- Java package-level JavaDoc (`package-info.java`) for
  `com.orderflow.bindings` and `com.orderflow.examples`.
- Richer Python module-level API documentation for `orderflow.api`,
  `orderflow._ffi`, and package root `orderflow`.
- JavaDoc overview page (`bindings/java/src/main/javadoc/overview.html`) to
  provide a richer published API landing page for Maven consumers.
- C SDK distribution packaging in `.github/workflows/release-native-artifacts.yml`,
  now publishing versioned platform archives with header, libraries, pkg-config
  metadata, and SDK README.
- C API header constants for stream kinds and data-quality flags
  (`of_stream_kind_t`, `of_data_quality_flags_t`) for first-class C developer
  ergonomics.
- C usage example: `examples/c/basic.c`.
- Official vcpkg submission scaffold for C consumers:
  `packaging/vcpkg/official/ports/orderflow-c` with portfile, manifest,
  and usage docs.
- Release helper script:
  `tools/release/sync_vcpkg_registry_baseline.py` to auto-sync the published
  `orderflow-vcpkg-registry` baseline SHA into tracked docs/config examples.
- Ops release runbook:
  `docs/ops/release_checklist.md` with binding/version sync, vcpkg baseline
  sync, docs coverage checks, and pre-publish validation commands.
- Runtime book snapshots across Rust, C, Python, and Java, including
  materialized snapshot queries and `BOOK_SNAPSHOT` callback delivery.
- Additive derived analytics snapshot APIs (`total_volume`, `trade_count`,
  `vwap`, `average_trade_size`, `imbalance_bps`) across Rust/C/Python/Java,
  plus `DERIVED_ANALYTICS` callback delivery.
- Additive candle-style analytics snapshots:
  `session_candle_snapshot` and rolling `interval_candle_snapshot(window_ns)`
  across Rust/C/Python/Java.
- Persistence readback/discovery APIs for venues, symbols, streams, typed
  book/trade reads, merged event reads, and inclusive sequence-range filtering.
- Replay CLI discovery-first flow for listing venues/symbols/streams and
  replaying merged events with optional sequence bounds.
- New built-in signal modules:
  `VolumeImbalanceSignal`, `CumulativeDeltaSignal`, `AbsorptionSignal`,
  `ExhaustionSignal`, `SweepDetectionSignal`, and `CompositeSignal`.
- Config compatibility reporting through
  `load_engine_config_report_from_path(...)`.
- Binding smoke checks in CI for Python and Java against the real native ABI.
- Expanded package README inventories for each public Rust crate API surface.

### Changed
- Rust crate publishing metadata now includes `repository`, `homepage`, and
  crate-level `documentation` links for better crates.io/docs.rs presentation.
- Workspace and binding author metadata updated to:
  Gregorian Rayne `<gregorianrayne09@gmail.com>`.
- Python PyPI metadata now includes project URLs, classifiers, and keywords.
- Java Maven metadata now includes organization, issue tracker URL,
  inception year, and developer id/email.
- Binding release versions are now managed centrally in
  `bindings/versions.toml` and synchronized via
  `tools/release/sync_binding_versions.py` across Python, Java, and Rust/C
  package version surfaces.
- Python and Java binding package descriptions were upgraded with richer
  packaging-facing docs (badges, API map, operations notes, and direct doc links)
  to improve PyPI and Maven discoverability.
- Python and Java binding distribution docs were expanded to include full
  public API reference tables, signature-level usage guidance, ingest/polling
  workflows, and troubleshooting sections for PyPI and Maven users.
- Binding and Rust/C package versions are now aligned for this release cycle
  at `0.2.0`.
- Added root `LICENSE` file (MIT) to satisfy package distribution and
  registry compliance requirements.
- Live adapters were hardened with reconnect/backoff supervision, subscription
  replay after reconnect, richer protocol health metadata, and stronger
  timeout/degraded-path handling.
- Runtime config loading now prefers typed TOML/JSON parsing with compatibility
  fallback for older flat config shapes.
- Runtime and FFI internals were modularized without changing the public Rust
  API or exported C symbols.

### Fixed
- Restored missing `#[no_mangle]` export attributes during the FFI
  modularization pass before release finalization.

### Upgrade Notes
- This release is additive and non-breaking for existing `0.1.x` integrations.
- Existing Rust, C, Python, and Java APIs continue to work; no required
  rename/removal migration is needed.
- Package versions are aligned at `0.2.0` for Rust, C, Python, and Java.
- If you use direct C or native bindings, update the native library and header
  together so the new snapshot symbols and stream constants stay in sync.
- If you use Python or Java, upgrade the binding package and the native
  `libof_ffi_c` library together.
- If you use config files, your existing flat config files still load, but new
  deployments should prefer the typed nested `adapter` and
  `adapter.credentials` shape.
- If you want to adopt the new functionality, start with:
  `book_snapshot`, `derived_analytics_snapshot`,
  `session_candle_snapshot`, `interval_candle_snapshot`,
  persistence readback APIs, and richer signal modules.

## [0.1.1] - 2026-03-16
### Fixed
- Python binding: `Engine.subscribe(..., callback=None)` now works correctly by
  passing a typed null callback pointer to the C ABI, instead of raising a
  `ctypes.ArgumentError`.

## [0.1.0] - 2026-03-09
### Added
- Initial public release of Rust crates (`of_core`, `of_signals`, `of_persist`,
  `of_adapters`, `of_runtime`, `of_ffi_c`), Java binding
  (`io.github.gregorian-09:orderflow-java-binding`), and Python binding
  (`orderflow-gregorian09`).
