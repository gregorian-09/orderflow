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
