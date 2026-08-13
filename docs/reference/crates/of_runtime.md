# `of_runtime` Reference

> Generated from `crates/of_runtime/Cargo.toml`, `rust-surface.md`, and `rust-values.md`.

**Version:** `0.5.0`  
**Description:** Runtime orchestration and health supervision for the Orderflow engine  
**Source:** [`crates/of_runtime/src`](https://github.com/gregorian-09/orderflow/tree/main/crates/of_runtime/src)  
**Generated Rustdoc:** [open `of_runtime` Rustdoc](https://docs.rs/of_runtime/0.5.0/of_runtime/)

This page is the crate-level index. The source links and generated
Rustdoc are authoritative for exact signatures, conditional compilation,
multiline declarations, and implementation-specific detail.

## Features

- `default`: empty feature
- `tickbar`: `of_core/tickbar`

## Local Dependencies

- [`of_adapters`](./of_adapters.md)
- [`of_core`](./of_core.md)
- [`of_persist`](./of_persist.md)
- [`of_signals`](./of_signals.md)

## Public Declaration Index

| Kind | Name | Summary | Source | Docs marker |
| --- | --- | --- | --- | --- |
| `enum` | `ConfigCompatibilityMode` | Indicates how a runtime config file was accepted | [`crates/of_runtime/src/config.rs:13`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L13) | `present` |
| `struct` | `ConfigLoadReport` | Detailed result for config-file loading | [`crates/of_runtime/src/config.rs:22`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L22) | `present` |
| `fn` | `used_legacy_fallback` | Returns `true` when the legacy flat-key compatibility parser was required | [`crates/of_runtime/src/config.rs:35`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L35) | `present` |
| `fn` | `load_engine_config_from_path` | Loads engine config from ` | [`crates/of_runtime/src/config.rs:41`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L41) | `present` |
| `fn` | `load_engine_config_report_from_path` | Loads engine config and reports whether legacy compatibility fallback was required | [`crates/of_runtime/src/config.rs:46`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L46) | `present` |
| `fn` | `validate_startup_config` | Validates startup configuration and environment prerequisites | [`crates/of_runtime/src/config.rs:60`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L60) | `present` |
| `struct` | `EngineConfig` | Runtime engine configuration | [`crates/of_runtime/src/engine.rs:54`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L54) | `present` |
| `enum` | `RuntimeError` | Runtime errors surfaced by engine lifecycle and processing | [`crates/of_runtime/src/engine.rs:104`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L104) | `present` |
| `fn` | `is_backpressure` | Returns true when this error represents an opt-in runtime backpressure condition | [`crates/of_runtime/src/engine.rs:130`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L130) | `present` |
| `fn` | `is_circuit_open` | Returns true when this error represents an open adapter circuit breaker | [`crates/of_runtime/src/engine.rs:135`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L135) | `present` |
| `struct` | `ExternalFeedPolicy` | Policy controlling quality constraints for externally-ingested feeds | [`crates/of_runtime/src/engine.rs:142`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L142) | `present` |
| `struct` | `RuntimeAdapterStatus` | Runtime status for the active market-data adapter | [`crates/of_runtime/src/engine.rs:152`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L152) | `present` |
| `struct` | `Engine` | Runtime engine over a market-data adapter and signal module | [`crates/of_runtime/src/engine.rs:528`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L528) | `present` |
| `type` | `DefaultEngine` | Default engine type used by C ABI and high-level bindings | [`crates/of_runtime/src/engine.rs:608`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L608) | `present` |
| `fn` | `adapter_inventory_json` | Returns all known adapter descriptors as compact JSON | [`crates/of_runtime/src/engine.rs:611`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L611) | `present` |
| `fn` | `signal_descriptor_inventory_json` | Returns built-in signal descriptors as compact JSON | [`crates/of_runtime/src/engine.rs:616`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L616) | `present` |
| `fn` | `new` | Creates an engine with explicit adapter and signal module | [`crates/of_runtime/src/engine.rs:623`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L623) | `present` |
| `fn` | `set_analytics_config` | Override analytics thresholds and buffer sizes | [`crates/of_runtime/src/engine.rs:678`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L678) | `present` |
| `fn` | `with_persistence` | Injects optional persistence backend | [`crates/of_runtime/src/engine.rs:683`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L683) | `present` |
| `fn` | `with_market_data_wal_producer` | Attaches an additive bounded normalized market-data WAL producer | [`crates/of_runtime/src/engine.rs:694`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L694) | `present` |
| `fn` | `set_market_data_wal_producer` | Replaces or clears the bounded normalized market-data WAL producer | [`crates/of_runtime/src/engine.rs:704`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L704) | `present` |
| `fn` | `configure_market_data_wal` | Starts and owns a bounded normalized market-data WAL writer | [`crates/of_runtime/src/engine.rs:732`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L732) | `present` |
| `fn` | `flush_market_data_persistence` | Flushes an engine-owned market-data WAL through a durability barrier | [`crates/of_runtime/src/engine.rs:764`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L764) | `present` |
| `fn` | `shutdown_market_data_persistence` | Stops an engine-owned writer and disables production persistence | [`crates/of_runtime/src/engine.rs:786`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L786) | `present` |
| `fn` | `market_data_writer_metrics` | Returns bounded normalized market-data writer metrics when configured | [`crates/of_runtime/src/engine.rs:805`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L805) | `present` |
| `fn` | `market_data_persistence_health` | Returns production normalized market-data persistence health | [`crates/of_runtime/src/engine.rs:812`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L812) | `present` |
| `fn` | `market_data_persistence_policy` | Returns configured production persistence policy | [`crates/of_runtime/src/engine.rs:820`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L820) | `present` |
| `fn` | `market_data_persistence_blocks_trading` | Returns whether persistence safety policy currently blocks trading | [`crates/of_runtime/src/engine.rs:828`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L828) | `present` |
| `fn` | `market_data_persistence_health_json` | Returns production persistence health as stable compact JSON | [`crates/of_runtime/src/engine.rs:835`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L835) | `present` |
| `fn` | `with_max_events_per_poll` | Sets an optional per-poll event drain limit | [`crates/of_runtime/src/engine.rs:850`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L850) | `present` |
| `fn` | `with_circuit_breaker` | Sets adapter circuit-breaker policy for repeated poll failures | [`crates/of_runtime/src/engine.rs:861`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L861) | `present` |
| `fn` | `start` | Connects adapter and marks runtime as started | [`crates/of_runtime/src/engine.rs:867`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L867) | `present` |
| `fn` | `stop` | Stops runtime state and emits health transition | [`crates/of_runtime/src/engine.rs:885`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L885) | `present` |
| `fn` | `shutdown_gracefully` | Graceful shutdown with signal handler | [`crates/of_runtime/src/engine.rs:903`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L903) | `present` |
| `fn` | `subscribe` | Subscribes to symbol stream through adapter | [`crates/of_runtime/src/engine.rs:910`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L910) | `present` |
| `fn` | `unsubscribe` | Unsubscribes symbol from adapter stream | [`crates/of_runtime/src/engine.rs:931`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L931) | `present` |
| `fn` | `reset_symbol_session` | Resets per-symbol analytics/session state | [`crates/of_runtime/src/engine.rs:948`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L948) | `present` |
| `fn` | `configure_external_feed` | Configures external-feed quality supervisor policy | [`crates/of_runtime/src/engine.rs:969`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L969) | `present` |
| `fn` | `set_external_reconnecting` | Marks external feed reconnecting/degraded state | [`crates/of_runtime/src/engine.rs:990`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L990) | `present` |
| `fn` | `external_health_tick` | Re-evaluates health for external-feed stale policy without ingesting data | [`crates/of_runtime/src/engine.rs:1005`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1005) | `present` |
| `fn` | `ingest_trade` | Ingests a single external trade event | [`crates/of_runtime/src/engine.rs:1015`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1015) | `present` |
| `fn` | `ingest_book` | Ingests a single external book event | [`crates/of_runtime/src/engine.rs:1037`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1037) | `present` |
| `fn` | `poll_once` | Polls adapter once and processes all returned events | [`crates/of_runtime/src/engine.rs:1059`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1059) | `present` |
| `fn` | `analytics_snapshot` | Returns analytics snapshot for symbol if available | [`crates/of_runtime/src/engine.rs:1133`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1133) | `present` |
| `fn` | `derived_analytics_snapshot` | Returns additive derived analytics snapshot for symbol if available | [`crates/of_runtime/src/engine.rs:1140`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1140) | `present` |
| `fn` | `session_candle_snapshot` | Returns session candle snapshot for symbol if available | [`crates/of_runtime/src/engine.rs:1150`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1150) | `present` |
| `fn` | `interval_candle_snapshot` | Returns rolling interval candle snapshot for symbol if available | [`crates/of_runtime/src/engine.rs:1157`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1157) | `present` |
| `fn` | `set_tickbar_interval` | Sets the tickbar aggregation interval | [`crates/of_runtime/src/engine.rs:1170`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1170) | `present` |
| `fn` | `tickbar_interval` | Returns the configured tickbar interval, if any | [`crates/of_runtime/src/engine.rs:1176`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1176) | `present` |
| `fn` | `bar_series` | Returns completed tickbar series for symbol if a tickbar aggregator is configured | [`crates/of_runtime/src/engine.rs:1182`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1182) | `present` |
| `fn` | `book_snapshot` | Returns the current materialized book snapshot for symbol if available | [`crates/of_runtime/src/engine.rs:1189`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1189) | `present` |
| `fn` | `book_analytics_snapshot` | Returns book-derived analytics (spread, depth, imbalance, microprice) for symbol if available | [`crates/of_runtime/src/engine.rs:1194`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1194) | `present` |
| `fn` | `weighted_average_price` | Returns the weighted average price for an order of `qty` shares by walking the book | [`crates/of_runtime/src/engine.rs:1204`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1204) | `present` |
| `fn` | `depth_slope` | Returns average volume decay per level for this symbol's book | [`crates/of_runtime/src/engine.rs:1214`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1214) | `present` |
| `fn` | `mid_price` | Returns mid price for symbol if the book has both sides | [`crates/of_runtime/src/engine.rs:1222`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1222) | `present` |
| `fn` | `effective_spread_bps` | Returns the effective spread in bps for the most recent trade | [`crates/of_runtime/src/engine.rs:1229`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1229) | `present` |
| `fn` | `half_spread_cost_bps` | Returns average half-spread cost in bps over the last `window` trades | [`crates/of_runtime/src/engine.rs:1237`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1237) | `present` |
| `fn` | `realised_spread_bps` | Returns realised spread in bps for the trade `hold_ticks` ago | [`crates/of_runtime/src/engine.rs:1245`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1245) | `present` |
| `fn` | `book_event_analytics` | Returns book-event analytics snapshot for symbol over `window_ns` | [`crates/of_runtime/src/engine.rs:1253`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1253) | `present` |
| `fn` | `resiliency_snapshot` | Returns resiliency snapshot for symbol | [`crates/of_runtime/src/engine.rs:1286`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1286) | `present` |
| `fn` | `vpin_snapshot` | Returns the VPIN snapshot for symbol | [`crates/of_runtime/src/engine.rs:1297`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1297) | `present` |
| `fn` | `kyle_lambda_snapshot` | Returns the Kyle's Lambda snapshot for symbol | [`crates/of_runtime/src/engine.rs:1305`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1305) | `present` |
| `fn` | `amihud_snapshot` | Returns the Amihud illiquidity snapshot for symbol | [`crates/of_runtime/src/engine.rs:1313`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1313) | `present` |
| `fn` | `cvd_enhancement_snapshot` | Returns the CVD enhancement snapshot for symbol | [`crates/of_runtime/src/engine.rs:1321`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1321) | `present` |
| `fn` | `pattern_snapshot` | Returns the pattern detection snapshot for symbol | [`crates/of_runtime/src/engine.rs:1329`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1329) | `present` |
| `fn` | `volatility_snapshot` | Returns volatility snapshot for symbol | [`crates/of_runtime/src/engine.rs:1352`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1352) | `present` |
| `fn` | `noise_snapshot` | Returns microstructure noise snapshot for symbol | [`crates/of_runtime/src/engine.rs:1360`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1360) | `present` |
| `fn` | `hasbrouck_snapshot` | Returns Hasbrouck VAR snapshot for symbol | [`crates/of_runtime/src/engine.rs:1368`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1368) | `present` |
| `fn` | `almgren_chriss_snapshot` | Returns Almgren-Chriss snapshot for symbol | [`crates/of_runtime/src/engine.rs:1376`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1376) | `present` |
| `fn` | `spread_decomp_snapshot` | Returns spread decomposition snapshot for symbol | [`crates/of_runtime/src/engine.rs:1384`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1384) | `present` |
| `fn` | `acd_snapshot` | Returns ACD snapshot for symbol | [`crates/of_runtime/src/engine.rs:1392`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1392) | `present` |
| `fn` | `regime_snapshot` | Returns regime snapshot for symbol | [`crates/of_runtime/src/engine.rs:1400`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1400) | `present` |
| `fn` | `kinetic_energy_snapshot` | Returns kinetic-energy snapshot for symbol | [`crates/of_runtime/src/engine.rs:1408`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1408) | `present` |
| `fn` | `dark_pool_snapshot` | Returns dark-pool analytics snapshot for symbol | [`crates/of_runtime/src/engine.rs:1416`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1416) | `present` |
| `fn` | `options_flow_snapshot` | Returns options-flow analytics snapshot for symbol | [`crates/of_runtime/src/engine.rs:1424`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1424) | `present` |
| `fn` | `futures_snapshot` | Returns futures basis and roll snapshot for symbol | [`crates/of_runtime/src/engine.rs:1432`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1432) | `present` |
| `fn` | `vol_signature_snapshot` | Returns volatility signature snapshot for symbol | [`crates/of_runtime/src/engine.rs:1440`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1440) | `present` |
| `fn` | `agent_type_snapshot` | Returns agent-type snapshot for symbol | [`crates/of_runtime/src/engine.rs:1448`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1448) | `present` |
| `fn` | `dark_lit_correlation_snapshot` | Returns dark-lit correlation snapshot for symbol | [`crates/of_runtime/src/engine.rs:1456`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1456) | `present` |
| `fn` | `institutional_flow_snapshot` | Returns institutional flow snapshot for symbol | [`crates/of_runtime/src/engine.rs:1464`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1464) | `present` |
| `fn` | `oi_analysis_snapshot` | Returns OI analysis snapshot for symbol | [`crates/of_runtime/src/engine.rs:1472`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1472) | `present` |
| `fn` | `lob_features` | Computes LOB feature snapshot from internal book state for a symbol | [`crates/of_runtime/src/engine.rs:1480`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1480) | `present` |
| `fn` | `last_classification` | Returns the last classification vote for symbol | [`crates/of_runtime/src/engine.rs:1497`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1497) | `present` |
| `fn` | `signal_snapshot` | Returns latest signal snapshot for symbol if available | [`crates/of_runtime/src/engine.rs:1505`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1505) | `present` |
| `fn` | `signal_explanation_json` | Returns latest signal explanation JSON for symbol if available | [`crates/of_runtime/src/engine.rs:1510`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1510) | `present` |
| `fn` | `adapter_descriptor` | Returns static descriptor for the configured adapter provider | [`crates/of_runtime/src/engine.rs:1515`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1515) | `present` |
| `fn` | `adapter_status` | Returns latest active-adapter status | [`crates/of_runtime/src/engine.rs:1520`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1520) | `present` |
| `fn` | `adapter_inventory_json` | Returns adapter inventory as compact JSON | [`crates/of_runtime/src/engine.rs:1572`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1572) | `present` |
| `fn` | `active_adapter_status_json` | Returns active adapter status as compact JSON | [`crates/of_runtime/src/engine.rs:1577`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1577) | `present` |
| `fn` | `signal_descriptor_inventory_json` | Returns built-in signal descriptors as compact JSON | [`crates/of_runtime/src/engine.rs:1582`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1582) | `present` |
| `fn` | `signal_metrics_json` | Returns signal metrics as compact JSON payload | [`crates/of_runtime/src/engine.rs:1587`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1587) | `present` |
| `fn` | `metrics_json` | Returns runtime metrics as compact JSON payload | [`crates/of_runtime/src/engine.rs:1632`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1632) | `present` |
| `fn` | `health_seq` | Returns monotonic health sequence number | [`crates/of_runtime/src/engine.rs:1712`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1712) | `present` |
| `fn` | `health_json` | Returns health snapshot as compact JSON payload | [`crates/of_runtime/src/engine.rs:1717`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1717) | `present` |
| `fn` | `last_events` | Returns events processed in the last poll/ingest cycle | [`crates/of_runtime/src/engine.rs:1787`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1787) | `present` |
| `fn` | `replay_normalized_wal_record` | Replays one versioned normalized WAL record without writing it again | [`crates/of_runtime/src/engine.rs:1796`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1796) | `present` |
| `fn` | `current_quality_flags_bits` | Returns currently-active quality flags as raw bits | [`crates/of_runtime/src/engine.rs:1834`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L1834) | `present` |
| `fn` | `build_default_engine` | Builds the default runtime engine using configured provider and signal module | [`crates/of_runtime/src/engine.rs:2629`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L2629) | `present` |

## Constants, Aliases, Fields, and Variants

| Kind | Owner | Name | Declared type/value | Source |
| --- | --- | --- | --- | --- |
| `variant` | `ConfigCompatibilityMode` | `Strict` | `Strict` | [`crates/of_runtime/src/config.rs:15`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L15) |
| `variant` | `ConfigCompatibilityMode` | `LegacyFallback` | `LegacyFallback` | [`crates/of_runtime/src/config.rs:17`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L17) |
| `field` | `ConfigLoadReport` | `config` | `: EngineConfig` | [`crates/of_runtime/src/config.rs:24`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L24) |
| `field` | `ConfigLoadReport` | `format` | `: &'static str` | [`crates/of_runtime/src/config.rs:26`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L26) |
| `field` | `ConfigLoadReport` | `compatibility_mode` | `: ConfigCompatibilityMode` | [`crates/of_runtime/src/config.rs:28`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L28) |
| `field` | `ConfigLoadReport` | `warning` | `: Option<String>` | [`crates/of_runtime/src/config.rs:30`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/config.rs#L30) |
| `field` | `EngineConfig` | `instance_id` | `: String` | [`crates/of_runtime/src/engine.rs:56`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L56) |
| `field` | `EngineConfig` | `enable_persistence` | `: bool` | [`crates/of_runtime/src/engine.rs:58`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L58) |
| `field` | `EngineConfig` | `data_root` | `: String` | [`crates/of_runtime/src/engine.rs:60`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L60) |
| `field` | `EngineConfig` | `audit_log_path` | `: String` | [`crates/of_runtime/src/engine.rs:62`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L62) |
| `field` | `EngineConfig` | `audit_max_bytes` | `: u64` | [`crates/of_runtime/src/engine.rs:64`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L64) |
| `field` | `EngineConfig` | `audit_max_files` | `: u32` | [`crates/of_runtime/src/engine.rs:66`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L66) |
| `field` | `EngineConfig` | `audit_redact_tokens` | `: Vec<String>` | [`crates/of_runtime/src/engine.rs:68`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L68) |
| `field` | `EngineConfig` | `data_retention_max_bytes` | `: u64` | [`crates/of_runtime/src/engine.rs:70`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L70) |
| `field` | `EngineConfig` | `data_retention_max_age_secs` | `: u64` | [`crates/of_runtime/src/engine.rs:72`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L72) |
| `field` | `EngineConfig` | `adapter` | `: AdapterConfig` | [`crates/of_runtime/src/engine.rs:74`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L74) |
| `field` | `EngineConfig` | `signal_threshold` | `: i64` | [`crates/of_runtime/src/engine.rs:76`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L76) |
| `variant` | `RuntimeError` | `Adapter` | `Adapter(String)` | [`crates/of_runtime/src/engine.rs:106`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L106) |
| `variant` | `RuntimeError` | `Config` | `Config(String)` | [`crates/of_runtime/src/engine.rs:108`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L108) |
| `variant` | `RuntimeError` | `Io` | `Io(String)` | [`crates/of_runtime/src/engine.rs:110`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L110) |
| `variant` | `RuntimeError` | `NotStarted` | `NotStarted` | [`crates/of_runtime/src/engine.rs:112`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L112) |
| `field` | `ExternalFeedPolicy` | `stale_after_ms` | `: u64` | [`crates/of_runtime/src/engine.rs:144`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L144) |
| `field` | `ExternalFeedPolicy` | `enforce_sequence` | `: bool` | [`crates/of_runtime/src/engine.rs:146`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L146) |
| `field` | `RuntimeAdapterStatus` | `descriptor` | `: AdapterDescriptor` | [`crates/of_runtime/src/engine.rs:154`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L154) |
| `field` | `RuntimeAdapterStatus` | `health` | `: AdapterHealth` | [`crates/of_runtime/src/engine.rs:156`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L156) |
| `field` | `RuntimeAdapterStatus` | `operational` | `: AdapterOperationalStatus` | [`crates/of_runtime/src/engine.rs:158`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L158) |
| `field` | `RuntimeAdapterStatus` | `health_seq` | `: u64` | [`crates/of_runtime/src/engine.rs:160`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L160) |
| `field` | `RuntimeAdapterStatus` | `started` | `: bool` | [`crates/of_runtime/src/engine.rs:162`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L162) |
| `field` | `RuntimeAdapterStatus` | `circuit_breaker_open` | `: bool` | [`crates/of_runtime/src/engine.rs:164`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L164) |
| `type` | `-` | `DefaultEngine` | `= Engine<Box<dyn MarketDataAdapter>, of_signals::DeltaMomentumSignal>` | [`crates/of_runtime/src/engine.rs:608`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_runtime/src/engine.rs#L608) |

## Audit Requirements

The semantic review for this crate must additionally document every
public item's purpose, invariants, defaults, errors, ownership,
thread-safety, allocation/blocking behavior, persistence implications,
feature availability, introduction version, and tested usage.

- [Rust public surface audit](../rust-surface.md)
- [Rust values and layout audit](../rust-values.md)
- [Package and feature matrix](../package-matrix.md)
