# Changelog
All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]
### Added
- Placeholder for the next release cycle.

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
