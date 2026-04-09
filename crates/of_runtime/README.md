# of_runtime

`of_runtime` is the orchestration layer for live, replay, and external-ingest workflows.
It wires adapter events into book state and analytics, applies quality-aware signal logic, and exposes snapshots plus health/metrics payloads.

## Runtime Responsibilities

1. Connect and supervise a [`MarketDataAdapter`](of_adapters::MarketDataAdapter).
2. Process normalized [`RawEvent`](of_adapters::RawEvent) streams.
3. Materialize book snapshots from normalized book updates.
4. Update analytics using `of_core`.
5. Evaluate signal modules from `of_signals`.
6. Gate risk-sensitive output using [`DataQualityFlags`](of_core::DataQualityFlags).
7. Optionally persist event streams via `of_persist`.

## New In 0.2.0

Relative to the `0.1.x` line, `of_runtime` now adds or hardens:

- real [`Engine::book_snapshot`] support
- additive derived analytics, session candle, and interval candle snapshots
- config compatibility reporting with [`ConfigLoadReport`]
- stronger live supervision and richer health/metrics payloads
- cleaner internal modularization without changing the public API

## Main Types

- [`EngineConfig`] - runtime, adapter, audit, and retention config.
- [`Engine<A, S>`] - generic runtime over adapter and signal module.
- [`DefaultEngine`] - boxed adapter + default delta signal.
- [`RuntimeError`] - lifecycle/config/adapter/io errors.
- [`ExternalFeedPolicy`] - stale and sequence policy for non-adapter ingest mode.
- Snapshot accessors:
  - [`Engine::book_snapshot`]
  - [`Engine::analytics_snapshot`]
  - [`Engine::derived_analytics_snapshot`]
  - [`Engine::session_candle_snapshot`]
  - [`Engine::interval_candle_snapshot`]
  - [`Engine::signal_snapshot`]

## Public API Inventory

Public types:

- [`EngineConfig`]
- [`RuntimeError`]
- [`ExternalFeedPolicy`]
- [`Engine<A, S>`]
- [`DefaultEngine`]
- [`ConfigCompatibilityMode`]
- [`ConfigLoadReport`]

Public top-level functions:

- [`build_default_engine`]
- [`load_engine_config_from_path`]
- [`load_engine_config_report_from_path`]
- [`validate_startup_config`]

Public `ConfigLoadReport` method:

- [`ConfigLoadReport::used_legacy_fallback`]

Public `Engine<A, S>` methods:

- [`Engine::new`]
- [`Engine::with_persistence`]
- [`Engine::start`]
- [`Engine::stop`]
- [`Engine::subscribe`]
- [`Engine::unsubscribe`]
- [`Engine::reset_symbol_session`]
- [`Engine::configure_external_feed`]
- [`Engine::set_external_reconnecting`]
- [`Engine::external_health_tick`]
- [`Engine::ingest_trade`]
- [`Engine::ingest_book`]
- [`Engine::poll_once`]
- [`Engine::analytics_snapshot`]
- [`Engine::derived_analytics_snapshot`]
- [`Engine::session_candle_snapshot`]
- [`Engine::interval_candle_snapshot`]
- [`Engine::book_snapshot`]
- [`Engine::signal_snapshot`]
- [`Engine::metrics_json`]
- [`Engine::health_seq`]
- [`Engine::health_json`]
- [`Engine::last_events`]
- [`Engine::current_quality_flags_bits`]

## EngineConfig Field Reference

[`EngineConfig`] is the runtime control plane.

- `instance_id`: logical engine name used in audit/metrics output.
- `enable_persistence`: enables JSONL persistence through [`RollingStore`](of_persist::RollingStore).
- `data_root`: persistence root directory when persistence is enabled.
- `audit_log_path`: audit log file path used by runtime audit output.
- `audit_max_bytes`: max bytes before audit rotation.
- `audit_max_files`: max rotated audit files retained.
- `audit_redact_tokens`: case-insensitive token list scrubbed from audit text.
- `data_retention_max_bytes`: persisted-data byte cap; `0` disables the limit.
- `data_retention_max_age_secs`: persisted-data age cap in seconds; `0` disables the limit.
- `adapter`: provider configuration forwarded to [`of_adapters`].
- `signal_threshold`: default threshold used by [`build_default_engine`].

## Lifecycle Contract

The runtime has a simple state machine:

1. Build an [`Engine`] with [`Engine::new`] or [`build_default_engine`].
2. Optionally attach persistence with [`Engine::with_persistence`].
3. Call [`Engine::start`] before subscribe, poll, or external-ingest operations.
4. Use either adapter polling or external ingest.
5. Read snapshots and health/metrics as needed.
6. Call [`Engine::stop`] when done.

Operational rules:

- [`Engine::start`] validates config and connects the adapter.
- [`Engine::subscribe`] and [`Engine::unsubscribe`] require a started engine.
- [`Engine::poll_once`] is for adapter-driven mode.
- [`Engine::ingest_trade`] and [`Engine::ingest_book`] are for externally-fed mode.
- Snapshot getters return `Option<_>` and remain `None` until enough symbol data has been observed.
- [`Engine::reset_symbol_session`] clears session analytics for one symbol without dropping the subscription itself.

## External Ingest Contract

Use external ingest when a separate bridge, broker API, or custom parser already owns transport.

- [`Engine::configure_external_feed`] enables stale/sequence supervision rules.
- [`Engine::set_external_reconnecting`] lets a bridge tell the runtime it is currently degraded/reconnecting.
- [`Engine::external_health_tick`] advances stale-feed supervision when no data has arrived recently.
- [`Engine::ingest_trade`] and [`Engine::ingest_book`] accept caller-supplied [`DataQualityFlags`](of_core::DataQualityFlags) so upstream bridges can forward their own quality judgments.

The runtime still applies its own sequence and stale checks on top of caller-supplied flags.

## Snapshot Semantics

All snapshot getters are additive and side-effect free.

- [`Engine::analytics_snapshot`] returns the base analytics payload for one symbol.
- [`Engine::derived_analytics_snapshot`] returns the additive totals view for one symbol.
- [`Engine::session_candle_snapshot`] returns one session candle for one symbol.
- [`Engine::interval_candle_snapshot`] computes one rolling-window candle for one symbol using the provided `window_ns`.
- [`Engine::book_snapshot`] returns the materialized book when book updates have been seen.
- [`Engine::signal_snapshot`] returns the latest evaluated signal state for one symbol.

Return behavior:

- `None` means the runtime has not yet observed enough data to build that snapshot for the requested symbol.
- Returned snapshots are clones of runtime state; callers can keep them without affecting engine internals.

## Health and Metrics Reference

- [`Engine::health_seq`] is the cheap monotonic change counter for external polling loops.
- [`Engine::health_json`] is the user-facing operational snapshot and includes connectivity, degradation, quality flags, supervision metadata, and tracked symbol counts.
- [`Engine::metrics_json`] is the counter-oriented metrics payload and includes processed event counts, quality flag detail, and subsystem counts.
- [`Engine::current_quality_flags_bits`] exposes the current runtime quality bitset directly for low-allocation callers.
- [`Engine::last_events`] exposes the last processed raw event batch for inspection/testing.

Compatibility rule:

- field names in JSON payloads are treated as stable once published
- new metrics/health fields are added additively rather than replacing old fields

## Config Loading and Validation Reference

- [`load_engine_config_from_path`] returns only the parsed [`EngineConfig`].
- [`load_engine_config_report_from_path`] also returns file format, compatibility mode, and optional warning text.
- [`validate_startup_config`] checks required env vars, retention settings, adapter endpoint rules, and signal/audit sanity before going live.
- [`ConfigCompatibilityMode::Strict`] means typed TOML/JSON parsing succeeded directly.
- [`ConfigCompatibilityMode::LegacyFallback`] means an older flat-key config was accepted through the compatibility loader.
- [`ConfigLoadReport::used_legacy_fallback`] is the simplest check for surfacing upgrade guidance in hosts or CLIs.

## Persistence Integration

When persistence is enabled, the runtime writes normalized `book` and `trade` streams through [`RollingStore`](of_persist::RollingStore).

- Persistence is optional and does not change runtime snapshot semantics.
- Retention limits are enforced through [`RetentionPolicy`](of_persist::RetentionPolicy).
- The runtime persists normalized events, not provider-native wire payloads.
- Readback and replay are handled by `of_persist` and `examples/replay_cli`.

## End-to-End Example (Adapter Polling)

```rust,no_run
use of_adapters::MockAdapter;
use of_core::{DataQualityFlags, SymbolId};
use of_runtime::{Engine, EngineConfig};
use of_signals::DeltaMomentumSignal;

let adapter = MockAdapter::default();
let signal = DeltaMomentumSignal::new(100);
let mut engine = Engine::new(EngineConfig::default(), adapter, signal);

engine.start()?;
engine.subscribe(SymbolId {
    venue: "SIM".to_string(),
    symbol: "ESM6".to_string(),
}, 10)?;

let _processed = engine.poll_once(DataQualityFlags::NONE)?;
engine.stop();
# Ok::<(), of_runtime::RuntimeError>(())
```

## External Ingest Example (Broker Bridge)

```rust,no_run
use of_adapters::MockAdapter;
use of_core::{DataQualityFlags, Side, SymbolId, TradePrint};
use of_runtime::{Engine, EngineConfig, ExternalFeedPolicy};
use of_signals::DeltaMomentumSignal;

let mut engine = Engine::new(
    EngineConfig::default(),
    MockAdapter::default(),
    DeltaMomentumSignal::default(),
);

engine.start()?;
engine.configure_external_feed(ExternalFeedPolicy {
    stale_after_ms: 15_000,
    enforce_sequence: true,
})?;

engine.ingest_trade(TradePrint {
    symbol: SymbolId { venue: "BINANCE".into(), symbol: "BTCUSDT".into() },
    price: 62_500_00,
    size: 100,
    aggressor_side: Side::Ask,
    sequence: 1,
    ts_exchange_ns: 1,
    ts_recv_ns: 2,
}, DataQualityFlags::NONE)?;

let health = engine.health_json();
assert!(health.contains("\"started\":true"));
# Ok::<(), of_runtime::RuntimeError>(())
```

## Snapshot Contracts

- [`Engine::book_snapshot`] returns the materialized book state for a symbol when book updates have been observed.
- Book snapshots contain `bids` and `asks` arrays, each ordered by `level`.
- [`Engine::analytics_snapshot`] and [`Engine::signal_snapshot`] retain their current payload semantics.
- [`Engine::derived_analytics_snapshot`] exposes additive session metrics without changing the original analytics snapshot contract.
- [`Engine::session_candle_snapshot`] exposes additive candle-style session state with `open`, `high`, `low`,
  `close`, `trade_count`, and first/last exchange timestamps.
- [`Engine::interval_candle_snapshot`] exposes a rolling-window candle view for a caller-supplied `window_ns`
  with `open`, `high`, `low`, `close`, `trade_count`, `total_volume`, `vwap`, and first/last exchange timestamps.

## Health and Metrics Contracts

- [`Engine::metrics_json`] exposes runtime counters and adapter status.
- [`Engine::health_json`] exposes connectivity/degradation/reconnect state and active quality flags.
- [`Engine::health_seq`] increments on meaningful health transitions for cheap external polling.
- Existing JSON field names remain stable; new observability fields are added additively.
- [`Engine::health_json`] also includes `quality_flags_detail`, `tracked_symbols`,
  `processed_events`, and external supervision fields such as `external_last_ingest_ns`.
- [`Engine::metrics_json`] also includes `health_seq`, per-subsystem symbol counts,
  `quality_flags_detail`, and external sequence-cache counts for live diagnostics.

## Config Loading

Use [`load_engine_config_from_path`] to load TOML config files, [`load_engine_config_report_from_path`]
when you also want compatibility diagnostics, and [`validate_startup_config`] to fail fast on
missing credentials or invalid startup settings before going live.

Preferred config files use typed TOML/JSON shapes with nested `adapter` and `adapter.credentials`
sections. Legacy flat config shapes are still accepted through a compatibility fallback so older
deployments continue to load without source changes. `ConfigLoadReport` tells you whether strict
parsing succeeded or the legacy fallback path was required.

## Operational Guidance

- Call [`Engine::start`] before subscribe/poll/ingest operations.
- Use `configure_external_feed` + `external_health_tick` when ingesting from non-adapter bridges.
- For deterministic simulation, pair `MockAdapter` with replayed events and fixed timestamps.
- Prefer [`load_engine_config_report_from_path`] in user-facing CLIs or services so you can warn when a config only loaded through compatibility fallback.

## Real-World Use Cases

### 1. Live runtime with adapter-managed transport

Use the engine as the orchestration layer when the adapter owns connectivity and
you want signal generation, health, persistence, and snapshots in one place.

### 2. Broker bridge or gateway ingestion

If another process already owns the transport, use `configure_external_feed`,
`ingest_trade`, and `ingest_book` to keep quality supervision and analytics
inside the runtime.

### 3. Deterministic replay and regression testing

Pair `MockAdapter` or external ingest with fixed event sequences and compare
snapshots, health transitions, and signal outputs.

## Detailed Example: Bridge-Driven Runtime

```rust,no_run
use of_adapters::MockAdapter;
use of_core::{BookAction, BookUpdate, DataQualityFlags, Side, SymbolId, TradePrint};
use of_runtime::{Engine, EngineConfig, ExternalFeedPolicy};
use of_signals::CompositeSignal;

let symbol = SymbolId {
    venue: "BINANCE".into(),
    symbol: "BTCUSDT".into(),
};

let mut engine = Engine::new(
    EngineConfig::default(),
    MockAdapter::default(),
    CompositeSignal::default(),
);

engine.start()?;
engine.configure_external_feed(ExternalFeedPolicy {
    stale_after_ms: 5_000,
    enforce_sequence: true,
})?;

engine.ingest_book(
    BookUpdate {
        symbol: symbol.clone(),
        side: Side::Bid,
        level: 0,
        price: 6_250_000,
        size: 10,
        action: BookAction::Upsert,
        sequence: 1,
        ts_exchange_ns: 1_000,
        ts_recv_ns: 1_200,
    },
    DataQualityFlags::NONE,
)?;

engine.ingest_trade(
    TradePrint {
        symbol: symbol.clone(),
        price: 6_250_100,
        size: 2,
        aggressor_side: Side::Ask,
        sequence: 2,
        ts_exchange_ns: 2_000,
        ts_recv_ns: 2_100,
    },
    DataQualityFlags::NONE,
)?;

if let Some(book) = engine.book_snapshot(&symbol) {
    println!("best bid levels={}", book.bids.len());
}
if let Some(derived) = engine.derived_analytics_snapshot(&symbol) {
    println!("trade_count={} vwap={}", derived.trade_count, derived.vwap);
}
# Ok::<(), of_runtime::RuntimeError>(())
```

## Strategy Construction Pattern With `of_runtime`

A practical runtime-backed strategy flow is:

1. subscribe one or more symbols
2. ingest or poll normalized events
3. read `analytics_snapshot` and `derived_analytics_snapshot` for context
4. use `signal_snapshot` as the strategy-facing decision surface
5. block action whenever `current_quality_flags_bits()` or `signal_snapshot.quality_flags` indicates degradation
6. persist the session so the same sequence can be replayed later
