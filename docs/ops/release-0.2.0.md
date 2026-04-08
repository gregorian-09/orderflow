# Release 0.2.0

Date: 2026-04-08

Orderflow `0.2.0` is the first hardening-focused feature release after the
initial `0.1.x` line. It keeps the public Rust API and C ABI stable while
adding the missing snapshot, replay, signal, and observability capabilities
that were previously identified as the main project weaknesses.

## Version Decision

This release is versioned as `0.2.0` because:

- it is materially larger than a patch release
- it adds multiple new public capabilities across Rust, C, Python, and Java
- it does not intentionally break the existing `0.1.x` public surface

Package versions for this release:

- Rust workspace / C ABI crates: `0.2.0`
- Python binding: `0.2.0`
- Java binding: `0.2.0`

## What Is New

### 1. Real book snapshots

The runtime now materializes full per-symbol book state instead of leaving the
snapshot path as a placeholder.

New practical outcomes:

- Rust: `Engine::book_snapshot(&SymbolId)`
- C ABI: `of_get_book_snapshot(...)`
- Python: `engine.book_snapshot(symbol)`
- Java: `engine.bookSnapshot(symbol)`
- C callbacks: `BOOK_SNAPSHOT` stream kind

### 2. Derived analytics and candle snapshots

The analytics surface is now much more useful out of the box.

New additive snapshot families:

- derived analytics:
  `total_volume`, `trade_count`, `vwap`, `average_trade_size`,
  `imbalance_bps`
- session candle:
  `open`, `high`, `low`, `close`, `trade_count`,
  `first_ts_exchange_ns`, `last_ts_exchange_ns`
- interval candle:
  rolling `window_ns` view with candle fields, `total_volume`, and `vwap`

### 3. Stronger built-in signals

The signal crate is no longer limited to the original delta-threshold module.

New built-in modules:

- `VolumeImbalanceSignal`
- `CumulativeDeltaSignal`
- `AbsorptionSignal`
- `ExhaustionSignal`
- `SweepDetectionSignal`
- `CompositeSignal`

### 4. Persistence readback and replay usability

Persistence is now useful for both write and read workflows.

New capabilities:

- venue discovery
- symbol discovery
- stream discovery
- typed trade/book readback
- merged event readback
- inclusive sequence range filtering
- replay CLI discovery flow and bounded replay

### 5. Adapter/live-path hardening

Live-path supervision is materially stronger than in `0.1.x`.

Improvements include:

- reconnect with backoff
- subscription replay after reconnect
- timeout/degraded-path handling
- richer `protocol_info` health metadata
- stronger live supervision in Rithmic and Binance paths

### 6. Config hardening without breaking old configs

Runtime config loading now:

- prefers strict typed TOML/JSON parsing
- still accepts legacy flat config shapes
- exposes compatibility reporting through
  `load_engine_config_report_from_path(...)`

### 7. Better verification and documentation

The project now has stronger compatibility guardrails:

- FFI export checks
- golden JSON payload checks
- Python binding smoke checks
- Java binding smoke checks
- 100% API docs coverage enforcement
- exhaustive public API inventories in package READMEs

## How 0.2.0 Differs From 0.1.x

### Compared to 0.1.0 / 0.1.1

`0.1.0` and `0.1.1` established the multi-language packaging surface, but the
engine still had clear maturity gaps:

- book snapshot support was incomplete
- persistence was mostly write-oriented
- the signal catalog was thin
- live adapter supervision was less robust
- config parsing was less strict and less transparent

`0.2.0` addresses those gaps directly.

### Compared to the later 0.1.x line

The later `0.1.x` work improved packaging and documentation quality, but
`0.2.0` is the release that materially upgrades runtime capability:

- real snapshot functionality instead of placeholder behavior
- replay/readback APIs instead of write-only persistence
- richer built-in signals
- better adapter/live supervision
- stronger CI compatibility guardrails
- cleaner internal runtime/FFI structure for maintainability

## What Existing Users Need To Do

### If you are already on `0.1.x`

Required migration:

- none

Recommended follow-up:

- update to the `0.2.0` native library if you use C/Python/Java bindings
- update the generated/packaged C header together with the native library
- run your existing integration tests against the new release

Optional adoption steps:

- start using `book_snapshot(...)` if you previously worked only from raw book
  updates
- use `derived_analytics_snapshot(...)` or the new candle snapshots instead of
  recomputing the same data outside the engine
- migrate old config files toward the nested typed `adapter` layout
- adopt persistence discovery/readback APIs for replay and incident analysis

### If you use Python or Java

Make sure the wrapper version and native library version match this release.

Recommended upgrade path:

1. upgrade the binding package to `0.2.0`
2. upgrade the native `libof_ffi_c` library to `0.2.0`
3. rerun any callback/snapshot integration tests

### If you use the C ABI directly

Pull the new header and library together, then consider adopting:

- `of_get_book_snapshot(...)`
- `of_get_derived_analytics_snapshot(...)`
- `of_get_session_candle_snapshot(...)`
- `of_get_interval_candle_snapshot(...)`

## Public APIs Added In 0.2.0

Rust/runtime:

- `Engine::book_snapshot`
- `Engine::derived_analytics_snapshot`
- `Engine::session_candle_snapshot`
- `Engine::interval_candle_snapshot`
- `load_engine_config_report_from_path`
- `ConfigLoadReport`
- `ConfigCompatibilityMode`

Rust/signals:

- `VolumeImbalanceSignal`
- `CumulativeDeltaSignal`
- `AbsorptionSignal`
- `ExhaustionSignal`
- `SweepDetectionSignal`
- `CompositeSignal`

Rust/persist:

- `list_venues`
- `list_symbols`
- `list_streams`
- `read_books_in_range`
- `read_trades_in_range`
- `read_events`
- `read_events_in_range`

C ABI:

- `of_get_book_snapshot`
- `of_get_derived_analytics_snapshot`
- `of_get_session_candle_snapshot`
- `of_get_interval_candle_snapshot`

Bindings:

- Python `book_snapshot`, `derived_analytics_snapshot`,
  `session_candle_snapshot`, `interval_candle_snapshot`
- Java `bookSnapshot`, `derivedAnalyticsSnapshot`,
  `sessionCandleSnapshot`, `intervalCandleSnapshot`

## Verification Status For This Release

Validated locally before finalizing the release notes:

- `cargo test -q`
- `cargo test -q -p of_runtime`
- `cargo test -q -p of_ffi_c`
- `cargo build -q -p of_ffi_c`
- `./tools/check_ffi_exports.sh target/debug/libof_ffi_c.so`
- `python3 tools/docs_coverage.py --enforce`
- Python binding smoke check
- Java binding smoke check

## Where To Read Next

- Changelog: [`CHANGELOG.md`](/home/gregorian-rayne/RustroverProjects/orderflow/CHANGELOG.md)
- Complete API reference: [docs/handbook/05-api-reference.md](/home/gregorian-rayne/RustroverProjects/orderflow/docs/handbook/05-api-reference.md)
- Release checklist: [docs/ops/release_checklist.md](/home/gregorian-rayne/RustroverProjects/orderflow/docs/ops/release_checklist.md)
