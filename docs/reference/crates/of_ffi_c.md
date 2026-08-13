# `of_ffi_c` Reference

> Generated from `crates/of_ffi_c/Cargo.toml`, `rust-surface.md`, and `rust-values.md`.

**Version:** `0.5.0`  
**Description:** C ABI facade for the Orderflow runtime  
**Source:** [`crates/of_ffi_c/src`](https://github.com/gregorian-09/orderflow/tree/main/crates/of_ffi_c/src)  
**Generated Rustdoc:** [open `of_ffi_c` Rustdoc](https://docs.rs/of_ffi_c/0.5.0/of_ffi_c/)

This page is the crate-level index. The source links and generated
Rustdoc are authoritative for exact signatures, conditional compilation,
multiline declarations, and implementation-specific detail.

## Features

- `default`: empty feature
- `rithmic`: `of_adapters/rithmic`
- `cqg`: `of_adapters/cqg`
- `cqg_proto`: `of_adapters/cqg_proto`
- `binance`: `of_adapters/binance`
- `tickbar`: `of_core/tickbar`, `of_runtime/tickbar`

## Local Dependencies

- [`of_adapters`](./of_adapters.md)
- [`of_core`](./of_core.md)
- [`of_execution`](./of_execution.md)
- [`of_execution_algos`](./of_execution_algos.md)
- [`of_execution_core`](./of_execution_core.md)
- [`of_persist`](./of_persist.md)
- [`of_runtime`](./of_runtime.md)
- [`of_signals`](./of_signals.md)

## Public Declaration Index

| Kind | Name | Summary | Source | Docs marker |
| --- | --- | --- | --- | --- |
| `struct` | `of_analytics_config_t` | Analytics configuration passed to [`of_engine_set_analytics_config`] | [`crates/of_ffi_c/src/lib.rs:71`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L71) | `present` |
| `struct` | `of_engine_config_t` | Engine configuration passed to [`of_engine_create`] | [`crates/of_ffi_c/src/lib.rs:149`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L149) | `present` |
| `struct` | `of_market_data_wal_config_t` | Engine-owned segmented market-data WAL configuration | [`crates/of_ffi_c/src/lib.rs:172`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L172) | `present` |
| `struct` | `of_symbol_t` | Symbol descriptor used by subscription and snapshot functions | [`crates/of_ffi_c/src/lib.rs:197`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L197) | `present` |
| `struct` | `of_trade_t` | External trade payload accepted by [`of_ingest_trade`] | [`crates/of_ffi_c/src/lib.rs:208`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L208) | `present` |
| `struct` | `of_book_t` | External order-book payload accepted by [`of_ingest_book`] | [`crates/of_ffi_c/src/lib.rs:227`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L227) | `present` |
| `struct` | `of_external_feed_policy_t` | External-feed quality policy configured via [`of_configure_external_feed`] | [`crates/of_ffi_c/src/lib.rs:250`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L250) | `present` |
| `enum` | `of_error_t` | Error codes returned by C ABI functions | [`crates/of_ffi_c/src/lib.rs:260`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L260) | `present` |
| `struct` | `of_execution_route_config_t` | Execution route and risk configuration | [`crates/of_ffi_c/src/lib.rs:283`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L283) | `present` |
| `struct` | `of_execution_order_request_t` | Execution order request | [`crates/of_ffi_c/src/lib.rs:310`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L310) | `present` |
| `struct` | `of_execution_cancel_request_t` | Execution cancel request | [`crates/of_ffi_c/src/lib.rs:343`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L343) | `present` |
| `struct` | `of_execution_amend_request_t` | Execution amend request | [`crates/of_ffi_c/src/lib.rs:364`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L364) | `present` |
| `struct` | `of_execution_event_t` | Execution event returned by execution C APIs | [`crates/of_ffi_c/src/lib.rs:390`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L390) | `present` |
| `struct` | `of_execution_order_state_t` | Execution order state returned by state query | [`crates/of_ffi_c/src/lib.rs:434`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L434) | `present` |
| `struct` | `of_execution_health_t` | Execution health snapshot | [`crates/of_ffi_c/src/lib.rs:464`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L464) | `present` |
| `struct` | `of_execution_metrics_t` | Execution metrics snapshot | [`crates/of_ffi_c/src/lib.rs:476`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L476) | `present` |
| `struct` | `of_execution_wal_integrity_report_t` | Execution WAL integrity report returned by [`of_execution_wal_integrity_report`] | [`crates/of_ffi_c/src/lib.rs:497`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L497) | `present` |
| `struct` | `of_execution_segmented_wal_integrity_report_t` | Segmented execution WAL integrity report returned by [`of_execution_segmented_wal_integrity_report`] | [`crates/of_ffi_c/src/lib.rs:524`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L524) | `present` |
| `struct` | `of_execution_checkpoint_store_integrity_report_t` | Execution checkpoint store integrity report returned by [`of_execution_checkpoint_store_integrity_report`] | [`crates/of_ffi_c/src/lib.rs:551`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L551) | `present` |
| `struct` | `of_execution_recovery_config_t` | Read-only execution recovery report configuration | [`crates/of_ffi_c/src/lib.rs:575`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L575) | `present` |
| `struct` | `of_execution_concurrent_config_t` | Concurrent execution worker configuration | [`crates/of_ffi_c/src/lib.rs:588`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L588) | `present` |
| `struct` | `of_execution_command_report_t` | Concurrent execution command report | [`crates/of_ffi_c/src/lib.rs:600`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L600) | `present` |
| `struct` | `of_execution_twap_config_t` | Parent-order configuration for a deterministic TWAP algorithm | [`crates/of_ffi_c/src/lib.rs:614`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L614) | `present` |
| `struct` | `of_execution_algo_child_plan_t` | Owned child-order plan produced by a deterministic execution algorithm | [`crates/of_ffi_c/src/lib.rs:656`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L656) | `present` |
| `struct` | `of_execution_algo_progress_t` | Aggregate progress snapshot for an execution algorithm | [`crates/of_ffi_c/src/lib.rs:696`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L696) | `present` |
| `struct` | `of_signal_config_parameter_t` | Tagged signal configuration parameter used by registry-based binding calls | [`crates/of_ffi_c/src/lib.rs:716`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L716) | `present` |
| `struct` | `of_signal_validation_config_t` | Replay-validation policy passed to the signal validation facade | [`crates/of_ffi_c/src/lib.rs:734`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L734) | `present` |
| `struct` | `of_signal_validation_event_t` | One analytics observation consumed by the signal replay validator | [`crates/of_ffi_c/src/lib.rs:750`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L750) | `present` |
| `struct` | `of_engine` | Opaque engine handle | [`crates/of_ffi_c/src/lib.rs:774`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L774) | `present` |
| `struct` | `of_execution_engine` | Opaque execution engine handle | [`crates/of_ffi_c/src/lib.rs:780`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L780) | `present` |
| `struct` | `of_execution_concurrent_engine` | Opaque concurrent execution engine handle | [`crates/of_ffi_c/src/lib.rs:785`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L785) | `present` |
| `struct` | `of_execution_twap_algo` | Opaque deterministic TWAP algorithm handle | [`crates/of_ffi_c/src/lib.rs:790`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L790) | `present` |
| `struct` | `of_subscription` | Opaque subscription token | [`crates/of_ffi_c/src/lib.rs:798`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L798) | `present` |
| `struct` | `of_event_t` | Event envelope dispatched to subscription callbacks | [`crates/of_ffi_c/src/lib.rs:804`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L804) | `present` |
| `type` | `of_event_cb` | C callback signature for subscription delivery | [`crates/of_ffi_c/src/lib.rs:822`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L822) | `present` |

## Constants, Aliases, Fields, and Variants

| Kind | Owner | Name | Declared type/value | Source |
| --- | --- | --- | --- | --- |
| `field` | `of_analytics_config_t` | `agent_small_trade_threshold` | `: f64` | [`crates/of_ffi_c/src/lib.rs:73`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L73) |
| `field` | `of_analytics_config_t` | `institutional_trade_threshold` | `: i64` | [`crates/of_ffi_c/src/lib.rs:75`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L75) |
| `field` | `of_analytics_config_t` | `cancel_arrival_window_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:77`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L77) |
| `field` | `of_analytics_config_t` | `vpin_volume_bucket` | `: u32` | [`crates/of_ffi_c/src/lib.rs:79`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L79) |
| `field` | `of_analytics_config_t` | `vpin_max_buckets` | `: u32` | [`crates/of_ffi_c/src/lib.rs:81`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L81) |
| `field` | `of_analytics_config_t` | `kyle_lambda_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:83`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L83) |
| `field` | `of_analytics_config_t` | `cvd_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:85`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L85) |
| `field` | `of_analytics_config_t` | `vol_estimator_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:87`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L87) |
| `field` | `of_analytics_config_t` | `noise_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:89`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L89) |
| `field` | `of_analytics_config_t` | `hasbrouck_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:91`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L91) |
| `field` | `of_analytics_config_t` | `almgren_chriss_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:93`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L93) |
| `field` | `of_analytics_config_t` | `acd_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:95`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L95) |
| `field` | `of_analytics_config_t` | `vol_signature_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:97`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L97) |
| `field` | `of_analytics_config_t` | `agent_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:99`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L99) |
| `field` | `of_analytics_config_t` | `agent_min_samples` | `: u32` | [`crates/of_ffi_c/src/lib.rs:101`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L101) |
| `field` | `of_analytics_config_t` | `institutional_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:103`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L103) |
| `field` | `of_analytics_config_t` | `resiliency_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:105`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L105) |
| `field` | `of_analytics_config_t` | `spread_decomp_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:107`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L107) |
| `field` | `of_analytics_config_t` | `regime_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:109`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L109) |
| `field` | `of_analytics_config_t` | `event_tracker_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:111`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L111) |
| `field` | `of_analytics_config_t` | `spread_tracker_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:113`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L113) |
| `field` | `of_analytics_config_t` | `default_max_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:115`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L115) |
| `field` | `of_engine_config_t` | `instance_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:151`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L151) |
| `field` | `of_engine_config_t` | `config_path` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:153`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L153) |
| `field` | `of_engine_config_t` | `log_level` | `: u32` | [`crates/of_ffi_c/src/lib.rs:155`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L155) |
| `field` | `of_engine_config_t` | `enable_persistence` | `: u8` | [`crates/of_ffi_c/src/lib.rs:157`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L157) |
| `field` | `of_engine_config_t` | `audit_max_bytes` | `: u64` | [`crates/of_ffi_c/src/lib.rs:159`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L159) |
| `field` | `of_engine_config_t` | `audit_max_files` | `: u32` | [`crates/of_ffi_c/src/lib.rs:161`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L161) |
| `field` | `of_engine_config_t` | `audit_redact_tokens_csv` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:163`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L163) |
| `field` | `of_engine_config_t` | `data_retention_max_bytes` | `: u64` | [`crates/of_ffi_c/src/lib.rs:165`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L165) |
| `field` | `of_engine_config_t` | `data_retention_max_age_secs` | `: u64` | [`crates/of_ffi_c/src/lib.rs:167`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L167) |
| `field` | `of_market_data_wal_config_t` | `root_path` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:174`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L174) |
| `field` | `of_market_data_wal_config_t` | `max_segment_bytes` | `: u64` | [`crates/of_ffi_c/src/lib.rs:176`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L176) |
| `field` | `of_market_data_wal_config_t` | `max_payload_bytes` | `: u64` | [`crates/of_ffi_c/src/lib.rs:178`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L178) |
| `field` | `of_market_data_wal_config_t` | `sync_policy` | `: u32` | [`crates/of_ffi_c/src/lib.rs:180`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L180) |
| `field` | `of_market_data_wal_config_t` | `sync_every_records` | `: u64` | [`crates/of_ffi_c/src/lib.rs:182`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L182) |
| `field` | `of_market_data_wal_config_t` | `sync_manifest` | `: u8` | [`crates/of_ffi_c/src/lib.rs:184`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L184) |
| `field` | `of_market_data_wal_config_t` | `queue_capacity` | `: u32` | [`crates/of_ffi_c/src/lib.rs:186`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L186) |
| `field` | `of_market_data_wal_config_t` | `max_queued_payload_bytes` | `: u64` | [`crates/of_ffi_c/src/lib.rs:188`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L188) |
| `field` | `of_market_data_wal_config_t` | `failure_action` | `: u32` | [`crates/of_ffi_c/src/lib.rs:190`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L190) |
| `field` | `of_market_data_wal_config_t` | `writer_thread_name` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:192`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L192) |
| `field` | `of_symbol_t` | `venue` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:199`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L199) |
| `field` | `of_symbol_t` | `symbol` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:201`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L201) |
| `field` | `of_symbol_t` | `depth_levels` | `: u16` | [`crates/of_ffi_c/src/lib.rs:203`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L203) |
| `field` | `of_trade_t` | `symbol` | `: of_symbol_t` | [`crates/of_ffi_c/src/lib.rs:210`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L210) |
| `field` | `of_trade_t` | `price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:212`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L212) |
| `field` | `of_trade_t` | `size` | `: i64` | [`crates/of_ffi_c/src/lib.rs:214`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L214) |
| `field` | `of_trade_t` | `aggressor_side` | `: u32` | [`crates/of_ffi_c/src/lib.rs:216`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L216) |
| `field` | `of_trade_t` | `sequence` | `: u64` | [`crates/of_ffi_c/src/lib.rs:218`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L218) |
| `field` | `of_trade_t` | `ts_exchange_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:220`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L220) |
| `field` | `of_trade_t` | `ts_recv_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:222`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L222) |
| `field` | `of_book_t` | `symbol` | `: of_symbol_t` | [`crates/of_ffi_c/src/lib.rs:229`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L229) |
| `field` | `of_book_t` | `side` | `: u32` | [`crates/of_ffi_c/src/lib.rs:231`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L231) |
| `field` | `of_book_t` | `level` | `: u16` | [`crates/of_ffi_c/src/lib.rs:233`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L233) |
| `field` | `of_book_t` | `price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:235`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L235) |
| `field` | `of_book_t` | `size` | `: i64` | [`crates/of_ffi_c/src/lib.rs:237`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L237) |
| `field` | `of_book_t` | `action` | `: u32` | [`crates/of_ffi_c/src/lib.rs:239`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L239) |
| `field` | `of_book_t` | `sequence` | `: u64` | [`crates/of_ffi_c/src/lib.rs:241`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L241) |
| `field` | `of_book_t` | `ts_exchange_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:243`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L243) |
| `field` | `of_book_t` | `ts_recv_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:245`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L245) |
| `field` | `of_external_feed_policy_t` | `stale_after_ms` | `: u64` | [`crates/of_ffi_c/src/lib.rs:252`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L252) |
| `field` | `of_external_feed_policy_t` | `enforce_sequence` | `: u8` | [`crates/of_ffi_c/src/lib.rs:254`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L254) |
| `variant` | `of_error_t` | `OF_OK` | `OF_OK = 0` | [`crates/of_ffi_c/src/lib.rs:262`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L262) |
| `variant` | `of_error_t` | `OF_ERR_INVALID_ARG` | `OF_ERR_INVALID_ARG = 1` | [`crates/of_ffi_c/src/lib.rs:264`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L264) |
| `variant` | `of_error_t` | `OF_ERR_STATE` | `OF_ERR_STATE = 2` | [`crates/of_ffi_c/src/lib.rs:266`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L266) |
| `variant` | `of_error_t` | `OF_ERR_IO` | `OF_ERR_IO = 3` | [`crates/of_ffi_c/src/lib.rs:268`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L268) |
| `variant` | `of_error_t` | `OF_ERR_AUTH` | `OF_ERR_AUTH = 4` | [`crates/of_ffi_c/src/lib.rs:270`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L270) |
| `variant` | `of_error_t` | `OF_ERR_BACKPRESSURE` | `OF_ERR_BACKPRESSURE = 5` | [`crates/of_ffi_c/src/lib.rs:272`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L272) |
| `variant` | `of_error_t` | `OF_ERR_DATA_QUALITY` | `OF_ERR_DATA_QUALITY = 6` | [`crates/of_ffi_c/src/lib.rs:274`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L274) |
| `variant` | `of_error_t` | `OF_ERR_RISK` | `OF_ERR_RISK = 7` | [`crates/of_ffi_c/src/lib.rs:276`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L276) |
| `variant` | `of_error_t` | `OF_ERR_INTERNAL` | `OF_ERR_INTERNAL = 255` | [`crates/of_ffi_c/src/lib.rs:278`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L278) |
| `field` | `of_execution_route_config_t` | `route_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:285`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L285) |
| `field` | `of_execution_route_config_t` | `account_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:287`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L287) |
| `field` | `of_execution_route_config_t` | `venue` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:289`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L289) |
| `field` | `of_execution_route_config_t` | `instrument` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:291`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L291) |
| `field` | `of_execution_route_config_t` | `enabled` | `: u8` | [`crates/of_ffi_c/src/lib.rs:293`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L293) |
| `field` | `of_execution_route_config_t` | `kill_switch` | `: u8` | [`crates/of_ffi_c/src/lib.rs:295`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L295) |
| `field` | `of_execution_route_config_t` | `max_order_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:297`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L297) |
| `field` | `of_execution_route_config_t` | `max_order_notional` | `: i64` | [`crates/of_ffi_c/src/lib.rs:299`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L299) |
| `field` | `of_execution_route_config_t` | `max_open_orders` | `: u32` | [`crates/of_ffi_c/src/lib.rs:301`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L301) |
| `field` | `of_execution_route_config_t` | `max_open_notional` | `: i64` | [`crates/of_ffi_c/src/lib.rs:303`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L303) |
| `field` | `of_execution_route_config_t` | `price_band_ticks` | `: i64` | [`crates/of_ffi_c/src/lib.rs:305`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L305) |
| `field` | `of_execution_order_request_t` | `client_order_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:312`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L312) |
| `field` | `of_execution_order_request_t` | `account_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:314`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L314) |
| `field` | `of_execution_order_request_t` | `route_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:316`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L316) |
| `field` | `of_execution_order_request_t` | `strategy_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:318`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L318) |
| `field` | `of_execution_order_request_t` | `venue` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:320`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L320) |
| `field` | `of_execution_order_request_t` | `instrument` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:322`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L322) |
| `field` | `of_execution_order_request_t` | `side` | `: u32` | [`crates/of_ffi_c/src/lib.rs:324`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L324) |
| `field` | `of_execution_order_request_t` | `order_type` | `: u32` | [`crates/of_ffi_c/src/lib.rs:326`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L326) |
| `field` | `of_execution_order_request_t` | `time_in_force` | `: u32` | [`crates/of_ffi_c/src/lib.rs:328`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L328) |
| `field` | `of_execution_order_request_t` | `quantity` | `: i64` | [`crates/of_ffi_c/src/lib.rs:330`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L330) |
| `field` | `of_execution_order_request_t` | `limit_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:332`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L332) |
| `field` | `of_execution_order_request_t` | `stop_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:334`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L334) |
| `field` | `of_execution_order_request_t` | `ts_exchange_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:336`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L336) |
| `field` | `of_execution_order_request_t` | `ts_recv_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:338`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L338) |
| `field` | `of_execution_cancel_request_t` | `client_order_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:345`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L345) |
| `field` | `of_execution_cancel_request_t` | `orig_client_order_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:347`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L347) |
| `field` | `of_execution_cancel_request_t` | `venue_order_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:349`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L349) |
| `field` | `of_execution_cancel_request_t` | `account_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:351`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L351) |
| `field` | `of_execution_cancel_request_t` | `route_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:353`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L353) |
| `field` | `of_execution_cancel_request_t` | `venue` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:355`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L355) |
| `field` | `of_execution_cancel_request_t` | `instrument` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:357`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L357) |
| `field` | `of_execution_cancel_request_t` | `ts_recv_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:359`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L359) |
| `field` | `of_execution_amend_request_t` | `client_order_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:366`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L366) |
| `field` | `of_execution_amend_request_t` | `orig_client_order_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:368`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L368) |
| `field` | `of_execution_amend_request_t` | `venue_order_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:370`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L370) |
| `field` | `of_execution_amend_request_t` | `account_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:372`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L372) |
| `field` | `of_execution_amend_request_t` | `route_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:374`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L374) |
| `field` | `of_execution_amend_request_t` | `venue` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:376`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L376) |
| `field` | `of_execution_amend_request_t` | `instrument` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:378`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L378) |
| `field` | `of_execution_amend_request_t` | `quantity` | `: i64` | [`crates/of_ffi_c/src/lib.rs:380`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L380) |
| `field` | `of_execution_amend_request_t` | `limit_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:382`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L382) |
| `field` | `of_execution_amend_request_t` | `ts_recv_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:384`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L384) |
| `field` | `of_execution_event_t` | `exec_type` | `: u32` | [`crates/of_ffi_c/src/lib.rs:392`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L392) |
| `field` | `of_execution_event_t` | `order_status` | `: u32` | [`crates/of_ffi_c/src/lib.rs:394`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L394) |
| `field` | `of_execution_event_t` | `client_order_id` | `: [c_char; 41]` | [`crates/of_ffi_c/src/lib.rs:396`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L396) |
| `field` | `of_execution_event_t` | `orig_client_order_id` | `: [c_char; 41]` | [`crates/of_ffi_c/src/lib.rs:398`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L398) |
| `field` | `of_execution_event_t` | `venue_order_id` | `: [c_char; 49]` | [`crates/of_ffi_c/src/lib.rs:400`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L400) |
| `field` | `of_execution_event_t` | `execution_id` | `: [c_char; 49]` | [`crates/of_ffi_c/src/lib.rs:402`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L402) |
| `field` | `of_execution_event_t` | `account_id` | `: [c_char; 33]` | [`crates/of_ffi_c/src/lib.rs:404`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L404) |
| `field` | `of_execution_event_t` | `route_id` | `: [c_char; 33]` | [`crates/of_ffi_c/src/lib.rs:406`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L406) |
| `field` | `of_execution_event_t` | `venue` | `: [c_char; 17]` | [`crates/of_ffi_c/src/lib.rs:408`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L408) |
| `field` | `of_execution_event_t` | `instrument` | `: [c_char; 33]` | [`crates/of_ffi_c/src/lib.rs:410`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L410) |
| `field` | `of_execution_event_t` | `last_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:412`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L412) |
| `field` | `of_execution_event_t` | `last_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:414`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L414) |
| `field` | `of_execution_event_t` | `cumulative_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:416`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L416) |
| `field` | `of_execution_event_t` | `leaves_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:418`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L418) |
| `field` | `of_execution_event_t` | `average_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:420`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L420) |
| `field` | `of_execution_event_t` | `ts_exchange_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:422`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L422) |
| `field` | `of_execution_event_t` | `ts_recv_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:424`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L424) |
| `field` | `of_execution_event_t` | `reason` | `: u32` | [`crates/of_ffi_c/src/lib.rs:426`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L426) |
| `field` | `of_execution_event_t` | `text` | `: [c_char; 129]` | [`crates/of_ffi_c/src/lib.rs:428`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L428) |
| `field` | `of_execution_order_state_t` | `client_order_id` | `: [c_char; 41]` | [`crates/of_ffi_c/src/lib.rs:436`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L436) |
| `field` | `of_execution_order_state_t` | `venue_order_id` | `: [c_char; 49]` | [`crates/of_ffi_c/src/lib.rs:438`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L438) |
| `field` | `of_execution_order_state_t` | `account_id` | `: [c_char; 33]` | [`crates/of_ffi_c/src/lib.rs:440`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L440) |
| `field` | `of_execution_order_state_t` | `route_id` | `: [c_char; 33]` | [`crates/of_ffi_c/src/lib.rs:442`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L442) |
| `field` | `of_execution_order_state_t` | `venue` | `: [c_char; 17]` | [`crates/of_ffi_c/src/lib.rs:444`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L444) |
| `field` | `of_execution_order_state_t` | `instrument` | `: [c_char; 33]` | [`crates/of_ffi_c/src/lib.rs:446`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L446) |
| `field` | `of_execution_order_state_t` | `status` | `: u32` | [`crates/of_ffi_c/src/lib.rs:448`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L448) |
| `field` | `of_execution_order_state_t` | `order_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:450`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L450) |
| `field` | `of_execution_order_state_t` | `cumulative_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:452`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L452) |
| `field` | `of_execution_order_state_t` | `leaves_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:454`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L454) |
| `field` | `of_execution_order_state_t` | `average_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:456`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L456) |
| `field` | `of_execution_order_state_t` | `updated_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:458`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L458) |
| `field` | `of_execution_health_t` | `connected` | `: u8` | [`crates/of_ffi_c/src/lib.rs:466`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L466) |
| `field` | `of_execution_health_t` | `degraded` | `: u8` | [`crates/of_ffi_c/src/lib.rs:468`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L468) |
| `field` | `of_execution_health_t` | `health_seq` | `: u64` | [`crates/of_ffi_c/src/lib.rs:470`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L470) |
| `field` | `of_execution_metrics_t` | `submitted` | `: u64` | [`crates/of_ffi_c/src/lib.rs:478`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L478) |
| `field` | `of_execution_metrics_t` | `cancelled` | `: u64` | [`crates/of_ffi_c/src/lib.rs:480`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L480) |
| `field` | `of_execution_metrics_t` | `amended` | `: u64` | [`crates/of_ffi_c/src/lib.rs:482`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L482) |
| `field` | `of_execution_metrics_t` | `events_applied` | `: u64` | [`crates/of_ffi_c/src/lib.rs:484`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L484) |
| `field` | `of_execution_metrics_t` | `risk_rejected` | `: u64` | [`crates/of_ffi_c/src/lib.rs:486`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L486) |
| `field` | `of_execution_metrics_t` | `adapter_errors` | `: u64` | [`crates/of_ffi_c/src/lib.rs:488`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L488) |
| `field` | `of_execution_metrics_t` | `recovered` | `: u64` | [`crates/of_ffi_c/src/lib.rs:490`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L490) |
| `field` | `of_execution_wal_integrity_report_t` | `records` | `: u64` | [`crates/of_ffi_c/src/lib.rs:499`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L499) |
| `field` | `of_execution_wal_integrity_report_t` | `bytes` | `: u64` | [`crates/of_ffi_c/src/lib.rs:501`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L501) |
| `field` | `of_execution_wal_integrity_report_t` | `first_sequence` | `: u64` | [`crates/of_ffi_c/src/lib.rs:503`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L503) |
| `field` | `of_execution_wal_integrity_report_t` | `last_sequence` | `: u64` | [`crates/of_ffi_c/src/lib.rs:505`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L505) |
| `field` | `of_execution_wal_integrity_report_t` | `checksum_failures` | `: u64` | [`crates/of_ffi_c/src/lib.rs:507`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L507) |
| `field` | `of_execution_wal_integrity_report_t` | `sequence_failures` | `: u64` | [`crates/of_ffi_c/src/lib.rs:509`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L509) |
| `field` | `of_execution_wal_integrity_report_t` | `has_first_sequence` | `: u8` | [`crates/of_ffi_c/src/lib.rs:511`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L511) |
| `field` | `of_execution_wal_integrity_report_t` | `has_last_sequence` | `: u8` | [`crates/of_ffi_c/src/lib.rs:513`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L513) |
| `field` | `of_execution_wal_integrity_report_t` | `truncated_tail` | `: u8` | [`crates/of_ffi_c/src/lib.rs:515`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L515) |
| `field` | `of_execution_wal_integrity_report_t` | `valid` | `: u8` | [`crates/of_ffi_c/src/lib.rs:517`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L517) |
| `field` | `of_execution_segmented_wal_integrity_report_t` | `segments` | `: u64` | [`crates/of_ffi_c/src/lib.rs:526`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L526) |
| `field` | `of_execution_segmented_wal_integrity_report_t` | `records` | `: u64` | [`crates/of_ffi_c/src/lib.rs:528`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L528) |
| `field` | `of_execution_segmented_wal_integrity_report_t` | `bytes` | `: u64` | [`crates/of_ffi_c/src/lib.rs:530`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L530) |
| `field` | `of_execution_segmented_wal_integrity_report_t` | `first_sequence` | `: u64` | [`crates/of_ffi_c/src/lib.rs:532`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L532) |
| `field` | `of_execution_segmented_wal_integrity_report_t` | `last_sequence` | `: u64` | [`crates/of_ffi_c/src/lib.rs:534`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L534) |
| `field` | `of_execution_segmented_wal_integrity_report_t` | `checksum_failures` | `: u64` | [`crates/of_ffi_c/src/lib.rs:536`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L536) |
| `field` | `of_execution_segmented_wal_integrity_report_t` | `sequence_failures` | `: u64` | [`crates/of_ffi_c/src/lib.rs:538`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L538) |
| `field` | `of_execution_segmented_wal_integrity_report_t` | `has_first_sequence` | `: u8` | [`crates/of_ffi_c/src/lib.rs:540`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L540) |
| `field` | `of_execution_segmented_wal_integrity_report_t` | `has_last_sequence` | `: u8` | [`crates/of_ffi_c/src/lib.rs:542`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L542) |
| `field` | `of_execution_segmented_wal_integrity_report_t` | `valid` | `: u8` | [`crates/of_ffi_c/src/lib.rs:544`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L544) |
| `field` | `of_execution_checkpoint_store_integrity_report_t` | `checkpoint_files` | `: u64` | [`crates/of_ffi_c/src/lib.rs:553`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L553) |
| `field` | `of_execution_checkpoint_store_integrity_report_t` | `valid_checkpoints` | `: u64` | [`crates/of_ffi_c/src/lib.rs:555`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L555) |
| `field` | `of_execution_checkpoint_store_integrity_report_t` | `invalid_checkpoints` | `: u64` | [`crates/of_ffi_c/src/lib.rs:557`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L557) |
| `field` | `of_execution_checkpoint_store_integrity_report_t` | `bytes` | `: u64` | [`crates/of_ffi_c/src/lib.rs:559`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L559) |
| `field` | `of_execution_checkpoint_store_integrity_report_t` | `latest_checkpoint_id` | `: u64` | [`crates/of_ffi_c/src/lib.rs:561`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L561) |
| `field` | `of_execution_checkpoint_store_integrity_report_t` | `latest_last_applied_sequence` | `: u64` | [`crates/of_ffi_c/src/lib.rs:563`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L563) |
| `field` | `of_execution_checkpoint_store_integrity_report_t` | `latest_created_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:565`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L565) |
| `field` | `of_execution_checkpoint_store_integrity_report_t` | `has_latest` | `: u8` | [`crates/of_ffi_c/src/lib.rs:567`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L567) |
| `field` | `of_execution_checkpoint_store_integrity_report_t` | `valid` | `: u8` | [`crates/of_ffi_c/src/lib.rs:569`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L569) |
| `field` | `of_execution_recovery_config_t` | `wal_root` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:577`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L577) |
| `field` | `of_execution_recovery_config_t` | `checkpoint_root` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:580`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L580) |
| `field` | `of_execution_recovery_config_t` | `require_checkpoint` | `: u8` | [`crates/of_ffi_c/src/lib.rs:582`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L582) |
| `field` | `of_execution_concurrent_config_t` | `command_capacity` | `: u32` | [`crates/of_ffi_c/src/lib.rs:590`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L590) |
| `field` | `of_execution_concurrent_config_t` | `report_capacity` | `: u32` | [`crates/of_ffi_c/src/lib.rs:592`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L592) |
| `field` | `of_execution_concurrent_config_t` | `event_buffer_capacity` | `: u32` | [`crates/of_ffi_c/src/lib.rs:594`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L594) |
| `field` | `of_execution_command_report_t` | `sequence` | `: u64` | [`crates/of_ffi_c/src/lib.rs:602`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L602) |
| `field` | `of_execution_command_report_t` | `kind` | `: u32` | [`crates/of_ffi_c/src/lib.rs:604`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L604) |
| `field` | `of_execution_command_report_t` | `result_code` | `: i32` | [`crates/of_ffi_c/src/lib.rs:606`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L606) |
| `field` | `of_execution_command_report_t` | `event_count` | `: u32` | [`crates/of_ffi_c/src/lib.rs:608`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L608) |
| `field` | `of_execution_twap_config_t` | `parent_order_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:616`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L616) |
| `field` | `of_execution_twap_config_t` | `account_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:618`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L618) |
| `field` | `of_execution_twap_config_t` | `route_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:620`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L620) |
| `field` | `of_execution_twap_config_t` | `strategy_id` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:622`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L622) |
| `field` | `of_execution_twap_config_t` | `venue` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:624`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L624) |
| `field` | `of_execution_twap_config_t` | `instrument` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:626`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L626) |
| `field` | `of_execution_twap_config_t` | `side` | `: u32` | [`crates/of_ffi_c/src/lib.rs:628`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L628) |
| `field` | `of_execution_twap_config_t` | `order_type` | `: u32` | [`crates/of_ffi_c/src/lib.rs:630`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L630) |
| `field` | `of_execution_twap_config_t` | `time_in_force` | `: u32` | [`crates/of_ffi_c/src/lib.rs:632`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L632) |
| `field` | `of_execution_twap_config_t` | `total_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:634`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L634) |
| `field` | `of_execution_twap_config_t` | `limit_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:636`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L636) |
| `field` | `of_execution_twap_config_t` | `stop_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:638`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L638) |
| `field` | `of_execution_twap_config_t` | `start_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:640`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L640) |
| `field` | `of_execution_twap_config_t` | `end_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:642`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L642) |
| `field` | `of_execution_twap_config_t` | `min_clip` | `: i64` | [`crates/of_ffi_c/src/lib.rs:644`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L644) |
| `field` | `of_execution_twap_config_t` | `max_clip` | `: i64` | [`crates/of_ffi_c/src/lib.rs:646`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L646) |
| `field` | `of_execution_twap_config_t` | `participation_cap_bps` | `: u16` | [`crates/of_ffi_c/src/lib.rs:648`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L648) |
| `field` | `of_execution_twap_config_t` | `slice_interval_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:650`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L650) |
| `field` | `of_execution_algo_child_plan_t` | `child_order_id` | `: [c_char; 41]` | [`crates/of_ffi_c/src/lib.rs:658`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L658) |
| `field` | `of_execution_algo_child_plan_t` | `parent_order_id` | `: [c_char; 41]` | [`crates/of_ffi_c/src/lib.rs:660`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L660) |
| `field` | `of_execution_algo_child_plan_t` | `client_order_id` | `: [c_char; 41]` | [`crates/of_ffi_c/src/lib.rs:662`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L662) |
| `field` | `of_execution_algo_child_plan_t` | `account_id` | `: [c_char; 33]` | [`crates/of_ffi_c/src/lib.rs:664`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L664) |
| `field` | `of_execution_algo_child_plan_t` | `route_id` | `: [c_char; 33]` | [`crates/of_ffi_c/src/lib.rs:666`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L666) |
| `field` | `of_execution_algo_child_plan_t` | `strategy_id` | `: [c_char; 33]` | [`crates/of_ffi_c/src/lib.rs:668`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L668) |
| `field` | `of_execution_algo_child_plan_t` | `venue` | `: [c_char; 17]` | [`crates/of_ffi_c/src/lib.rs:670`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L670) |
| `field` | `of_execution_algo_child_plan_t` | `instrument` | `: [c_char; 33]` | [`crates/of_ffi_c/src/lib.rs:672`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L672) |
| `field` | `of_execution_algo_child_plan_t` | `side` | `: u32` | [`crates/of_ffi_c/src/lib.rs:674`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L674) |
| `field` | `of_execution_algo_child_plan_t` | `order_type` | `: u32` | [`crates/of_ffi_c/src/lib.rs:676`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L676) |
| `field` | `of_execution_algo_child_plan_t` | `time_in_force` | `: u32` | [`crates/of_ffi_c/src/lib.rs:678`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L678) |
| `field` | `of_execution_algo_child_plan_t` | `quantity` | `: i64` | [`crates/of_ffi_c/src/lib.rs:680`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L680) |
| `field` | `of_execution_algo_child_plan_t` | `limit_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:682`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L682) |
| `field` | `of_execution_algo_child_plan_t` | `stop_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:684`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L684) |
| `field` | `of_execution_algo_child_plan_t` | `due_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:686`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L686) |
| `field` | `of_execution_algo_child_plan_t` | `ts_recv_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:688`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L688) |
| `field` | `of_execution_algo_child_plan_t` | `has_plan` | `: u8` | [`crates/of_ffi_c/src/lib.rs:690`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L690) |
| `field` | `of_execution_algo_progress_t` | `target_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:698`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L698) |
| `field` | `of_execution_algo_progress_t` | `released_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:700`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L700) |
| `field` | `of_execution_algo_progress_t` | `completed_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:702`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L702) |
| `field` | `of_execution_algo_progress_t` | `open_qty` | `: i64` | [`crates/of_ffi_c/src/lib.rs:704`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L704) |
| `field` | `of_execution_algo_progress_t` | `rejected_children` | `: u64` | [`crates/of_ffi_c/src/lib.rs:706`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L706) |
| `field` | `of_execution_algo_progress_t` | `terminal_children` | `: u64` | [`crates/of_ffi_c/src/lib.rs:708`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L708) |
| `field` | `of_execution_algo_progress_t` | `has_pending_plan` | `: u8` | [`crates/of_ffi_c/src/lib.rs:710`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L710) |
| `field` | `of_signal_config_parameter_t` | `name` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:718`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L718) |
| `field` | `of_signal_config_parameter_t` | `kind` | `: u32` | [`crates/of_ffi_c/src/lib.rs:720`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L720) |
| `field` | `of_signal_config_parameter_t` | `integer_value` | `: i64` | [`crates/of_ffi_c/src/lib.rs:722`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L722) |
| `field` | `of_signal_config_parameter_t` | `float_value` | `: f64` | [`crates/of_ffi_c/src/lib.rs:724`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L724) |
| `field` | `of_signal_config_parameter_t` | `boolean_value` | `: u8` | [`crates/of_ffi_c/src/lib.rs:726`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L726) |
| `field` | `of_signal_config_parameter_t` | `text_value` | `: *const c_char` | [`crates/of_ffi_c/src/lib.rs:728`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L728) |
| `field` | `of_signal_validation_config_t` | `markout_horizon_events` | `: u32` | [`crates/of_ffi_c/src/lib.rs:736`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L736) |
| `field` | `of_signal_validation_config_t` | `flat_price_threshold` | `: i64` | [`crates/of_ffi_c/src/lib.rs:738`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L738) |
| `field` | `of_signal_validation_config_t` | `min_confidence_bps` | `: u16` | [`crates/of_ffi_c/src/lib.rs:740`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L740) |
| `field` | `of_signal_validation_config_t` | `store_samples` | `: u8` | [`crates/of_ffi_c/src/lib.rs:742`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L742) |
| `field` | `of_signal_validation_config_t` | `check_monotonic_timestamps` | `: u8` | [`crates/of_ffi_c/src/lib.rs:744`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L744) |
| `field` | `of_signal_validation_event_t` | `delta` | `: i64` | [`crates/of_ffi_c/src/lib.rs:752`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L752) |
| `field` | `of_signal_validation_event_t` | `cumulative_delta` | `: i64` | [`crates/of_ffi_c/src/lib.rs:754`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L754) |
| `field` | `of_signal_validation_event_t` | `buy_volume` | `: i64` | [`crates/of_ffi_c/src/lib.rs:756`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L756) |
| `field` | `of_signal_validation_event_t` | `sell_volume` | `: i64` | [`crates/of_ffi_c/src/lib.rs:758`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L758) |
| `field` | `of_signal_validation_event_t` | `last_price` | `: i64` | [`crates/of_ffi_c/src/lib.rs:760`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L760) |
| `field` | `of_signal_validation_event_t` | `point_of_control` | `: i64` | [`crates/of_ffi_c/src/lib.rs:762`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L762) |
| `field` | `of_signal_validation_event_t` | `value_area_low` | `: i64` | [`crates/of_ffi_c/src/lib.rs:764`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L764) |
| `field` | `of_signal_validation_event_t` | `value_area_high` | `: i64` | [`crates/of_ffi_c/src/lib.rs:766`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L766) |
| `field` | `of_signal_validation_event_t` | `ts_exchange_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:768`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L768) |
| `field` | `of_signal_validation_event_t` | `has_ts_exchange_ns` | `: u8` | [`crates/of_ffi_c/src/lib.rs:770`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L770) |
| `field` | `of_event_t` | `ts_exchange_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:806`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L806) |
| `field` | `of_event_t` | `ts_recv_ns` | `: u64` | [`crates/of_ffi_c/src/lib.rs:808`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L808) |
| `field` | `of_event_t` | `kind` | `: u32` | [`crates/of_ffi_c/src/lib.rs:810`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L810) |
| `field` | `of_event_t` | `payload` | `: *const c_void` | [`crates/of_ffi_c/src/lib.rs:812`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L812) |
| `field` | `of_event_t` | `payload_len` | `: u32` | [`crates/of_ffi_c/src/lib.rs:814`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L814) |
| `field` | `of_event_t` | `schema_id` | `: u32` | [`crates/of_ffi_c/src/lib.rs:816`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L816) |
| `field` | `of_event_t` | `quality_flags` | `: u32` | [`crates/of_ffi_c/src/lib.rs:818`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L818) |
| `type` | `-` | `of_event_cb` | `= extern "C" fn(*const of_event_t, *mut c_void)` | [`crates/of_ffi_c/src/lib.rs:822`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/src/lib.rs#L822) |

## Audit Requirements

The semantic review for this crate must additionally document every
public item's purpose, invariants, defaults, errors, ownership,
thread-safety, allocation/blocking behavior, persistence implications,
feature availability, introduction version, and tested usage.

- [Rust public surface audit](../rust-surface.md)
- [Rust values and layout audit](../rust-values.md)
- [Package and feature matrix](../package-matrix.md)
