# of_runtime

`of_runtime` is the orchestration layer for live, replay, and external-ingest workflows.
It wires adapter events into analytics, applies quality-aware signal logic, and exposes health/metrics snapshots.

## Runtime Responsibilities

1. Connect and supervise a [`MarketDataAdapter`](of_adapters::MarketDataAdapter).
2. Process normalized [`RawEvent`](of_adapters::RawEvent) streams.
3. Update analytics using `of_core`.
4. Evaluate signal modules from `of_signals`.
5. Gate risk-sensitive output using [`DataQualityFlags`](of_core::DataQualityFlags).
6. Optionally persist event streams via `of_persist`.

## Main Types

- [`EngineConfig`] - runtime, adapter, audit, and retention config.
- [`Engine<A, S>`] - generic runtime over adapter and signal module.
- [`DefaultEngine`] - boxed adapter + default delta signal.
- [`RuntimeError`] - lifecycle/config/adapter/io errors.
- [`ExternalFeedPolicy`] - stale and sequence policy for non-adapter ingest mode.

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

## Health and Metrics Contracts

- [`Engine::metrics_json`] exposes runtime counters and adapter status.
- [`Engine::health_json`] exposes connectivity/degradation/reconnect state and active quality flags.
- [`Engine::health_seq`] increments on meaningful health transitions for cheap external polling.

## Config Loading

Use [`load_engine_config_from_path`] to load TOML config files and [`validate_startup_config`] to fail fast
on missing credentials or invalid startup settings before going live.

## Operational Guidance

- Call [`Engine::start`] before subscribe/poll/ingest operations.
- Use `configure_external_feed` + `external_health_tick` when ingesting from non-adapter bridges.
- For deterministic simulation, pair `MockAdapter` with replayed events and fixed timestamps.
