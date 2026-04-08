# API Reference (Rust, C, Python, Java)

This page is the complete public API map for the current codebase.

## Compatibility Layers

- **Rust crates** are the implementation and extension surface.
- **C ABI** (`crates/of_ffi_c/include/orderflow.h`) is the stable cross-language boundary.
- **Python** wraps C ABI with `ctypes`.
- **Java** wraps C ABI with JNA.

---

## Rust API

### `of_core`

Public types:

- `SymbolId { venue, symbol }`
- `Side` (`Bid`, `Ask`)
- `BookAction` (`Upsert`, `Delete`)
- `BookUpdate`
- `TradePrint`
- `AnalyticsSnapshot`
- `DerivedAnalyticsSnapshot`
- `SignalState` (`Neutral`, `LongBias`, `ShortBias`, `Blocked`)
- `SignalSnapshot`
- `DataQualityFlags`
- `AnalyticsAccumulator`

Public `DataQualityFlags` constants:

- `NONE`
- `STALE_FEED`
- `SEQUENCE_GAP`
- `CLOCK_SKEW`
- `DEPTH_TRUNCATED`
- `OUT_OF_ORDER`
- `ADAPTER_DEGRADED`

Public methods:

- `DataQualityFlags::bits() -> u32`
- `DataQualityFlags::from_bits_truncate(u32) -> DataQualityFlags`
- `DataQualityFlags::intersects(DataQualityFlags) -> bool`
- `AnalyticsAccumulator::on_trade(&TradePrint)`
- `AnalyticsAccumulator::reset_session_delta()`
- `AnalyticsAccumulator::reset_session()`
- `AnalyticsAccumulator::snapshot() -> AnalyticsSnapshot`
- `AnalyticsAccumulator::derived_snapshot() -> DerivedAnalyticsSnapshot`

### `of_adapters`

Public types:

- `SubscribeReq { symbol, depth_levels }`
- `AdapterHealth { connected, degraded, last_error, protocol_info }`
- `RawEvent` (`Book(BookUpdate)`, `Trade(TradePrint)`)
- `AdapterError`
- `AdapterResult<T>`
- `MarketDataAdapter` trait
- `ProviderKind` (`Mock`, `Rithmic`, `Cqg`, `Binance`)
- `AdapterConfig`
- `CredentialsRef`
- `MockAdapter`

Public functions/methods:

- `create_adapter(&AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>>`
- `MockAdapter::push_event(RawEvent)`

`MarketDataAdapter` trait methods:

- `connect()`
- `subscribe(SubscribeReq)`
- `unsubscribe(SymbolId)`
- `poll(&mut Vec<RawEvent>)`
- `health() -> AdapterHealth`

### `of_signals`

Public types:

- `SignalGateDecision` (`Pass`, `Block`)
- `SignalModule` trait
- `DeltaMomentumSignal`
- `VolumeImbalanceSignal`
- `CumulativeDeltaSignal`

Public methods:

- `DeltaMomentumSignal::new(threshold: i64) -> Self`
- `VolumeImbalanceSignal::new(threshold: i64) -> Self`
- `CumulativeDeltaSignal::new(threshold: i64) -> Self`

`SignalModule` trait methods:

- `on_analytics(&AnalyticsSnapshot)`
- `snapshot() -> SignalSnapshot`
- `quality_gate(DataQualityFlags) -> SignalGateDecision`

### `of_persist`

Public types:

- `PersistError`
- `PersistResult<T>`
- `RetentionPolicy { max_total_bytes, max_age_secs }`
- `RollingStore`
- `StoredBookEvent`
- `StoredTradeEvent`
- `StoredEvent`

Public methods:

- `RollingStore::new(root) -> PersistResult<RollingStore>`
- `RollingStore::with_retention(Option<RetentionPolicy>) -> RollingStore`
- `RollingStore::append_book(&BookUpdate) -> PersistResult<()>`
- `RollingStore::append_trade(&TradePrint) -> PersistResult<()>`
- `RollingStore::list_venues() -> PersistResult<Vec<String>>`
- `RollingStore::list_symbols(venue) -> PersistResult<Vec<String>>`
- `RollingStore::read_books(venue, symbol) -> PersistResult<Vec<StoredBookEvent>>`
- `RollingStore::read_trades(venue, symbol) -> PersistResult<Vec<StoredTradeEvent>>`
- `RollingStore::read_events(venue, symbol) -> PersistResult<Vec<StoredEvent>>`

### `of_runtime`

Public types:

- `EngineConfig`
- `RuntimeError`
- `ExternalFeedPolicy`
- `Engine<A, S>`
- `DefaultEngine` type alias

Public constructor/build/config functions:

- `Engine::new(cfg, adapter, signal_module) -> Engine<A, S>`
- `build_default_engine(cfg: EngineConfig) -> Result<DefaultEngine, RuntimeError>`
- `load_engine_config_from_path(path: &str) -> Result<EngineConfig, RuntimeError>`
  - preferred input shape: typed TOML/JSON with nested `adapter` / `adapter.credentials`
  - compatibility fallback: legacy flat config files remain accepted
- `validate_startup_config(cfg: &EngineConfig) -> Result<(), RuntimeError>`

Public runtime methods:

- `with_persistence(Option<RollingStore>)`
- `start()`
- `stop()`
- `subscribe(SymbolId, depth_levels)`
- `unsubscribe(SymbolId)`
- `reset_symbol_session(SymbolId)`
- `configure_external_feed(ExternalFeedPolicy)`
- `set_external_reconnecting(bool)`
- `external_health_tick()`
- `ingest_trade(TradePrint, DataQualityFlags)`
- `ingest_book(BookUpdate, DataQualityFlags)`
- `poll_once(DataQualityFlags)`
- `analytics_snapshot(&SymbolId)`
- `derived_analytics_snapshot(&SymbolId)`
- `signal_snapshot(&SymbolId)`
- `metrics_json() -> String`
- `health_seq() -> u64`
- `health_json() -> String`
- `last_events() -> &[RawEvent]`
- `current_quality_flags_bits() -> u32`

---

## C API (`orderflow.h`)

### Opaque Handles

- `of_engine_t`
- `of_subscription_t`

### Data Structures

- `of_engine_config_t`
- `of_symbol_t`
- `of_trade_t`
- `of_book_t`
- `of_external_feed_policy_t`
- `of_event_t`

### Enums and constants

- `of_side_t`: `OF_SIDE_BID`, `OF_SIDE_ASK`
- `of_book_action_t`: `OF_BOOK_ACTION_UPSERT`, `OF_BOOK_ACTION_DELETE`
- `of_error_t`: `OF_OK`, `OF_ERR_INVALID_ARG`, `OF_ERR_STATE`, `OF_ERR_IO`, `OF_ERR_AUTH`, `OF_ERR_BACKPRESSURE`, `OF_ERR_DATA_QUALITY`, `OF_ERR_INTERNAL`

### Functions

Lifecycle:

- `of_api_version()`
- `of_build_info()`
- `of_engine_create(...)`
- `of_engine_start(...)`
- `of_engine_stop(...)`
- `of_engine_destroy(...)`

Subscription and processing:

- `of_subscribe(...)`
- `of_unsubscribe(...)`
- `of_unsubscribe_symbol(...)`
- `of_reset_symbol_session(...)`
- `of_engine_poll_once(...)`

External ingest and quality supervision:

- `of_ingest_trade(...)`
- `of_ingest_book(...)`
- `of_configure_external_feed(...)`
- `of_external_set_reconnecting(...)`
- `of_external_health_tick(...)`

Snapshots and metrics:

- `of_get_book_snapshot(...)`
- `of_get_analytics_snapshot(...)`
- `of_get_derived_analytics_snapshot(...)`
- `of_get_signal_snapshot(...)`
- `of_get_metrics_json(...)`
- `of_string_free(...)`

### Stream Kind IDs

Used in `of_subscribe(..., kind, ...)` and callback payloads:

- `1`: BOOK
- `2`: TRADES
- `3`: ANALYTICS
- `4`: SIGNALS
- `5`: HEALTH
- `6`: BOOK_SNAPSHOT
- `7`: DERIVED_ANALYTICS

### C API Notes

- `of_get_book_snapshot(...)` returns populated JSON when book updates exist for the symbol.
- `BOOK_SNAPSHOT` callback payloads use the same JSON contract as `of_get_book_snapshot(...)`.
- `DERIVED_ANALYTICS` callback payloads use the same JSON contract as `of_get_derived_analytics_snapshot(...)`.
- Book snapshot JSON includes:
  - `venue`
  - `symbol`
  - `bids`
  - `asks`
  - `last_sequence`
  - `ts_exchange_ns`
  - `ts_recv_ns`
- `of_get_analytics_snapshot(...)`, `of_get_derived_analytics_snapshot(...)`, and `of_get_signal_snapshot(...)` return populated JSON when data exists.
- `of_get_metrics_json(...)` allocates output string; caller must free via `of_string_free(...)`.
- Snapshot functions report the required byte size via `inout_len`; callers should retry with a larger buffer when they receive `OF_ERR_INVALID_ARG`.

---

## Python Binding API (`bindings/python/orderflow/api.py`)

### Public classes/constants

- `StreamKind` (`BOOK`, `TRADES`, `ANALYTICS`, `SIGNALS`, `HEALTH`, `BOOK_SNAPSHOT`, `DERIVED_ANALYTICS`)
- `Side` (`BID`, `ASK`)
- `BookAction` (`UPSERT`, `DELETE`)
- `DataQualityFlags` constants
- `OrderflowError`, `OrderflowStateError`, `OrderflowArgError`
- `Symbol`
- `EngineConfig`
- `ExternalFeedPolicy`
- `Engine`

### `Engine` public methods/properties

- `api_version` (property)
- `build_info` (property)
- `start()`
- `stop()`
- `close()`
- `subscribe(symbol, stream_kind=..., callback=None)`
- `poll_once(quality_flags=DataQualityFlags.NONE)`
- `unsubscribe(symbol)`
- `reset_symbol_session(symbol)`
- `configure_external_feed(policy)`
- `set_external_reconnecting(reconnecting)`
- `external_health_tick()`
- `ingest_trade(symbol, price, size, aggressor_side, sequence=0, ts_exchange_ns=0, ts_recv_ns=0, quality_flags=...)`
- `ingest_book(symbol, side, level, price, size, action=..., sequence=0, ts_exchange_ns=0, ts_recv_ns=0, quality_flags=...)`
- `book_snapshot(symbol) -> dict`
- `analytics_snapshot(symbol) -> dict`
- `derived_analytics_snapshot(symbol) -> dict`
- `signal_snapshot(symbol) -> dict`
- `metrics() -> dict`

Context manager support:

- `with Engine(config) as eng: ...`

---

## Java Binding API (`bindings/java/src/main/java/com/orderflow/bindings`)

### Public user-facing classes

- `OrderflowEngine` (`AutoCloseable`)
- `EngineConfig`
- `Symbol`
- `StreamKind`
- `DataQualityFlags`
- `Side`
- `BookAction`
- `OrderflowEvent`
- `EventListener`
- `OrderflowException`
- `OrderflowArgException`
- `OrderflowStateException`

### `OrderflowEngine` public methods

- `apiVersion()`
- `buildInfo()`
- `start()`
- `stop()`
- `subscribe(Symbol, int)`
- `subscribe(Symbol, int, EventListener)`
- `pollOnce(int qualityFlags)`
- `unsubscribe(Symbol)`
- `resetSymbolSession(Symbol)`
- `configureExternalFeed(long staleAfterMs, boolean enforceSequence)`
- `setExternalReconnecting(boolean reconnecting)`
- `externalHealthTick()`
- `ingestTrade(Symbol, long price, long size, int aggressorSide)`
- `ingestTrade(Symbol, long price, long size, int aggressorSide, long sequence, long tsExchangeNs, long tsRecvNs, int qualityFlags)`
- `ingestBook(Symbol, int side, int level, long price, long size)`
- `ingestBook(Symbol, int side, int level, long price, long size, int action, long sequence, long tsExchangeNs, long tsRecvNs, int qualityFlags)`
- `bookSnapshot(Symbol)`
- `analyticsSnapshot(Symbol)`
- `derivedAnalyticsSnapshot(Symbol)`
- `signalSnapshot(Symbol)`
- `metricsJson()`
- `close()`

---

## JSON Payload Contracts

### BOOK event payload (`StreamKind=1`)

```json
{
  "venue": "CME",
  "symbol": "ESM6",
  "side": "Bid|Ask",
  "level": 0,
  "price": 504900,
  "size": 20,
  "action": "Upsert|Delete",
  "sequence": 1,
  "ts_exchange_ns": 1000,
  "ts_recv_ns": 1100
}
```

### TRADES event payload (`StreamKind=2`)

```json
{
  "venue": "CME",
  "symbol": "ESM6",
  "price": 505000,
  "size": 7,
  "aggressor": "Bid|Ask",
  "sequence": 2,
  "ts_exchange_ns": 1200,
  "ts_recv_ns": 1300
}
```

### ANALYTICS snapshot/payload (`StreamKind=3`)

```json
{
  "delta": 7,
  "cumulative_delta": 21,
  "buy_volume": 55,
  "sell_volume": 48,
  "last_price": 505000,
  "point_of_control": 504900,
  "value_area_low": 504700,
  "value_area_high": 505100
}
```

### Derived analytics snapshot (`of_get_derived_analytics_snapshot`)

```json
{
  "total_volume": 15,
  "trade_count": 2,
  "vwap": 504966,
  "average_trade_size": 7,
  "imbalance_bps": 3333
}
```

### SIGNAL snapshot/payload (`StreamKind=4`)

```json
{
  "module": "delta_momentum_v1",
  "state": "neutral|long_bias|short_bias|blocked",
  "confidence_bps": 500,
  "quality_flags": 0,
  "reason": "delta_inside_band"
}
```

### HEALTH payload (`StreamKind=5`)

```json
{
  "health_seq": 3,
  "started": true,
  "connected": true,
  "degraded": false,
  "reconnect_state": "streaming|degraded|disconnected",
  "quality_flags": 0,
  "last_error": null,
  "protocol_info": "mock_adapter"
}
```

### BOOK_SNAPSHOT payload (`StreamKind=6`)

```json
{
  "venue": "CME",
  "symbol": "ESM6",
  "bids": [{"level": 0, "price": 504900, "size": 20}],
  "asks": [{"level": 0, "price": 505000, "size": 18}],
  "last_sequence": 8,
  "ts_exchange_ns": 1400,
  "ts_recv_ns": 1500
}
```

### DERIVED_ANALYTICS payload (`StreamKind=7`)

```json
{
  "total_volume": 15,
  "trade_count": 2,
  "vwap": 504966,
  "average_trade_size": 7,
  "imbalance_bps": 3333
}
```

### Metrics payload

```json
{
  "instance_id": "example",
  "started": true,
  "processed_events": 120,
  "symbols": 1,
  "persistence": false,
  "adapter_connected": true,
  "adapter_degraded": false,
  "adapter_last_error": null,
  "adapter_protocol_info": "mock_adapter",
  "external_feed_enabled": true,
  "external_feed_reconnecting": false,
  "external_stale_after_ms": 15000
}
```

---

## Error Mapping

C error codes:

- `0`: success
- `1`: invalid argument
- `2`: invalid state
- `3`: I/O
- `4`: auth
- `5`: backpressure
- `6`: data quality
- `255`: internal

Binding behavior:

- Python maps non-zero codes to `Orderflow*Error`.
- Java maps `1` to `OrderflowArgException`, `2` to `OrderflowStateException`, others to `OrderflowException`.
