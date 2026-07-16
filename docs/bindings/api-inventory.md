# Binding API Inventory

This file is generated from `bindings/api_manifest.toml`.
Run `python3 tools/generate_api_inventory.py` after changing the C ABI manifest.

The inventory tracks the stable C ABI symbols that low-level bindings and
release checks use as their source of truth. Human-facing Python and Java
wrappers remain documented in their binding-specific README files.

## Summary

- Exported symbols: `87`
- Families: `5`

## Binding Compatibility Matrix

The matrix reports whether each exported C ABI symbol is declared in the
low-level Python ctypes layer and Java JNA layer. `yes` means the symbol
has both Python `argtypes` and `restype` registrations, and a Java native
interface declaration.

| Function | Family | C ABI | Python ctypes | Java JNA | Exposure |
| --- | --- | --- | --- | --- | --- |
| `of_api_version` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_build_info` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_api_version` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_wal_integrity_report` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_segmented_wal_integrity_report` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_checkpoint_store_integrity_report` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_engine_create` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_engine_create_multi` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_engine_start` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_engine_stop` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_submit_order` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_cancel_order` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_amend_order` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_poll` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_get_order_state` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_health` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_metrics` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_concurrent_engine_create_multi` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_concurrent_stop` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_concurrent_submit_order` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_concurrent_cancel_order` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_concurrent_amend_order` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_concurrent_poll` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_concurrent_try_recv_report` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_engine_destroy` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_execution_concurrent_engine_destroy` | `execution` | yes | yes | yes | `LowLevelGenerated` |
| `of_engine_create` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_engine_start` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_engine_stop` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_subscribe` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_unsubscribe` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_unsubscribe_symbol` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_reset_symbol_session` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_ingest_trade` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_ingest_book` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_configure_external_feed` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_external_set_reconnecting` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_external_health_tick` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_engine_poll_once` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_engine_set_tickbar_interval` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_engine_destroy` | `runtime` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_book_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_book_analytics_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_compute_weighted_average_price` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_compute_depth_slope` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_mid_price` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_effective_spread_bps` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_half_spread_cost_bps` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_realised_spread_bps` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_book_event_analytics` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_resiliency_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_vpin_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_kyle_lambda_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_amihud_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_cvd_enhancement_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_pattern_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_volatility_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_noise_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_hasbrouck_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_almgren_chriss_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_spread_decomp_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_acd_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_regime_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_kinetic_energy_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_dark_pool_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_options_flow_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_futures_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_vol_signature_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_agent_type_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_dark_lit_correlation_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_institutional_flow_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_oi_analysis_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_analytics_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_derived_analytics_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_session_candle_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_interval_candle_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_bar_series` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_signal_snapshot` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_compute_lob_features` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_engine_set_analytics_config` | `analytics` | yes | yes | yes | `LowLevelGenerated` |
| `of_get_metrics_json` | `runtime` | yes | yes | yes | `JsonFacade` |
| `of_get_adapter_inventory_json` | `adapters` | yes | yes | yes | `JsonFacade` |
| `of_get_active_adapter_status_json` | `adapters` | yes | yes | yes | `JsonFacade` |
| `of_get_signal_descriptors_json` | `signals` | yes | yes | yes | `JsonFacade` |
| `of_get_signal_explanation_json` | `signals` | yes | yes | yes | `JsonFacade` |
| `of_get_signal_metrics_json` | `signals` | yes | yes | yes | `JsonFacade` |
| `of_string_free` | `runtime` | yes | yes | yes | `LowLevelGenerated` |

## Adapters

Symbols: `2`

| Function | Return | Ownership | Introduced | Binding Exposure |
| --- | --- | --- | --- | --- |
| `of_get_adapter_inventory_json` | `int32_t` | `library_allocated_string` | `0.4.0` | `JsonFacade` |
| `of_get_active_adapter_status_json` | `int32_t` | `library_allocated_string` | `0.4.0` | `JsonFacade` |

## Analytics

Symbols: `39`

| Function | Return | Ownership | Introduced | Binding Exposure |
| --- | --- | --- | --- | --- |
| `of_get_book_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_book_analytics_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_compute_weighted_average_price` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_compute_depth_slope` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_mid_price` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_effective_spread_bps` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_half_spread_cost_bps` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_realised_spread_bps` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_book_event_analytics` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_resiliency_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_vpin_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_kyle_lambda_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_amihud_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_cvd_enhancement_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_pattern_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_volatility_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_noise_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_hasbrouck_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_almgren_chriss_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_spread_decomp_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_acd_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_regime_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_kinetic_energy_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_dark_pool_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_options_flow_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_futures_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_vol_signature_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_agent_type_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_dark_lit_correlation_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_institutional_flow_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_oi_analysis_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_analytics_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_derived_analytics_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_session_candle_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_interval_candle_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_bar_series` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_signal_snapshot` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_compute_lob_features` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |
| `of_engine_set_analytics_config` | `int32_t` | `caller_owned_buffer` | `pre-manifest` | `LowLevelGenerated` |

## Execution

Symbols: `24`

| Function | Return | Ownership | Introduced | Binding Exposure |
| --- | --- | --- | --- | --- |
| `of_execution_api_version` | `uint32_t` | `value` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_wal_integrity_report` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_segmented_wal_integrity_report` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_checkpoint_store_integrity_report` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_engine_create` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_engine_create_multi` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_engine_start` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_engine_stop` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_submit_order` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_cancel_order` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_amend_order` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_poll` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_get_order_state` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_health` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_metrics` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_concurrent_engine_create_multi` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_concurrent_stop` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_concurrent_submit_order` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_concurrent_cancel_order` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_concurrent_amend_order` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_concurrent_poll` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_concurrent_try_recv_report` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_engine_destroy` | `void` | `releases_handle` | `pre-manifest` | `LowLevelGenerated` |
| `of_execution_concurrent_engine_destroy` | `void` | `releases_handle` | `pre-manifest` | `LowLevelGenerated` |

## Runtime

Symbols: `19`

| Function | Return | Ownership | Introduced | Binding Exposure |
| --- | --- | --- | --- | --- |
| `of_api_version` | `uint32_t` | `value` | `pre-manifest` | `LowLevelGenerated` |
| `of_build_info` | `const char*` | `static_string` | `pre-manifest` | `LowLevelGenerated` |
| `of_engine_create` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_engine_start` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_engine_stop` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_subscribe` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_unsubscribe` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_unsubscribe_symbol` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_reset_symbol_session` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_ingest_trade` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_ingest_book` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_configure_external_feed` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_external_set_reconnecting` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_external_health_tick` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_engine_poll_once` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_engine_set_tickbar_interval` | `int32_t` | `caller_owned_output` | `pre-manifest` | `LowLevelGenerated` |
| `of_engine_destroy` | `void` | `releases_handle` | `pre-manifest` | `LowLevelGenerated` |
| `of_get_metrics_json` | `int32_t` | `library_allocated_string` | `pre-manifest` | `JsonFacade` |
| `of_string_free` | `void` | `releases_library_allocated_string` | `pre-manifest` | `LowLevelGenerated` |

## Signals

Symbols: `3`

| Function | Return | Ownership | Introduced | Binding Exposure |
| --- | --- | --- | --- | --- |
| `of_get_signal_descriptors_json` | `int32_t` | `library_allocated_string` | `0.4.0` | `JsonFacade` |
| `of_get_signal_explanation_json` | `int32_t` | `library_allocated_string` | `0.4.0` | `JsonFacade` |
| `of_get_signal_metrics_json` | `int32_t` | `library_allocated_string` | `0.4.0` | `JsonFacade` |
