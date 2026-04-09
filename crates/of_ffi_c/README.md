# of_ffi_c

`of_ffi_c` exposes a stable C ABI for embedding the Orderflow runtime in non-Rust environments.
It is the native interface used by Python (`ctypes`), Java (JNA), and any C-compatible host runtime.

## ABI Surface

- Engine lifecycle: `of_engine_create`, `of_engine_start`, `of_engine_stop`, `of_engine_destroy`
- Subscription: `of_subscribe`, `of_unsubscribe`, `of_unsubscribe_symbol`, `of_reset_symbol_session`
- External ingest and supervision: `of_ingest_trade`, `of_ingest_book`, `of_configure_external_feed`, `of_external_set_reconnecting`, `of_external_health_tick`
- Polling and snapshots: `of_engine_poll_once`, `of_get_book_snapshot`, `of_get_analytics_snapshot`, `of_get_derived_analytics_snapshot`, `of_get_session_candle_snapshot`, `of_get_interval_candle_snapshot`, `of_get_signal_snapshot`
- Metrics and memory management: `of_get_metrics_json`, `of_string_free`

## Public ABI Inventory

Public C structs/types:

- `of_engine_config_t`
- `of_symbol_t`
- `of_trade_t`
- `of_book_t`
- `of_external_feed_policy_t`
- `of_error_t`
- `of_engine`
- `of_subscription`
- `of_event_t`
- `of_event_cb`

Exported C functions:

- `of_api_version`
- `of_build_info`
- `of_engine_create`
- `of_engine_start`
- `of_engine_stop`
- `of_engine_destroy`
- `of_subscribe`
- `of_unsubscribe`
- `of_unsubscribe_symbol`
- `of_reset_symbol_session`
- `of_ingest_trade`
- `of_ingest_book`
- `of_configure_external_feed`
- `of_external_set_reconnecting`
- `of_external_health_tick`
- `of_get_book_snapshot`
- `of_get_analytics_snapshot`
- `of_get_derived_analytics_snapshot`
- `of_get_session_candle_snapshot`
- `of_get_interval_candle_snapshot`
- `of_get_signal_snapshot`
- `of_get_metrics_json`
- `of_string_free`
- `of_engine_poll_once`

`of_get_book_snapshot` returns a materialized JSON snapshot with:

- `venue`
- `symbol`
- `bids`
- `asks`
- `last_sequence`
- `ts_exchange_ns`
- `ts_recv_ns`

`of_get_derived_analytics_snapshot` returns additive session metrics with:

- `total_volume`
- `trade_count`
- `vwap`
- `average_trade_size`
- `imbalance_bps`

`of_get_session_candle_snapshot` returns candle-style session state with:

- `open`
- `high`
- `low`
- `close`
- `trade_count`
- `first_ts_exchange_ns`
- `last_ts_exchange_ns`

`of_get_interval_candle_snapshot` returns rolling-window candle state for a caller-supplied `window_ns` with:

- `window_ns`
- `open`
- `high`
- `low`
- `close`
- `trade_count`
- `total_volume`
- `vwap`
- `first_ts_exchange_ns`
- `last_ts_exchange_ns`

Subscription stream ids:

- `1`: `BOOK` raw book updates
- `2`: `TRADES` raw trade prints
- `3`: `ANALYTICS` snapshot callbacks
- `4`: `SIGNALS` snapshot callbacks
- `5`: `HEALTH` transition callbacks
- `6`: `BOOK_SNAPSHOT` materialized book snapshot callbacks after book changes
- `7`: `DERIVED_ANALYTICS` session-derived analytics callbacks after trade changes

## C Struct Reference

`of_engine_config_t`:

- `instance_id`: optional runtime instance id override
- `config_path`: optional `.toml` or `.json` runtime config path
- `log_level`: reserved for host integrations
- `enable_persistence`: non-zero enables persistence
- `audit_max_bytes`: audit rotation size
- `audit_max_files`: audit retention count
- `audit_redact_tokens_csv`: comma-separated audit redaction tokens
- `data_retention_max_bytes`: persistence byte cap
- `data_retention_max_age_secs`: persistence age cap in seconds

`of_symbol_t`:

- `venue`: venue/exchange name
- `symbol`: normalized symbol string
- `depth_levels`: requested book depth for subscribe calls

`of_trade_t`:

- `symbol`: embedded [`of_symbol_t`]
- `price`, `size`: integer-normalized trade values
- `aggressor_side`: one of `OF_SIDE_BID` or `OF_SIDE_ASK`
- `sequence`: venue sequence or `0` when unavailable
- `ts_exchange_ns`, `ts_recv_ns`: exchange and local timestamps

`of_book_t`:

- `symbol`: embedded [`of_symbol_t`]
- `side`: one of `OF_SIDE_BID` or `OF_SIDE_ASK`
- `level`: top-of-book-relative depth index
- `price`, `size`: integer-normalized book values
- `action`: one of `OF_BOOK_ACTION_UPSERT` or `OF_BOOK_ACTION_DELETE`
- `sequence`: venue sequence or `0` when unavailable
- `ts_exchange_ns`, `ts_recv_ns`: exchange and local timestamps

`of_external_feed_policy_t`:

- `stale_after_ms`: max allowed ingest silence before stale status
- `enforce_sequence`: non-zero enables sequence-gap/out-of-order checks

`of_event_t` callback envelope:

- `kind`: stream kind id
- `payload` / `payload_len`: UTF-8 JSON payload bytes
- `schema_id`: payload schema id, currently `1`
- `quality_flags`: `OF_DQ_*` bits associated with the event
- timestamps are copied from the underlying event when available

## Function Family Reference

Lifecycle:

- `of_engine_create`
- `of_engine_start`
- `of_engine_stop`
- `of_engine_destroy`

Subscription:

- `of_subscribe`
- `of_unsubscribe`
- `of_unsubscribe_symbol`
- `of_reset_symbol_session`

External ingest and supervision:

- `of_ingest_trade`
- `of_ingest_book`
- `of_configure_external_feed`
- `of_external_set_reconnecting`
- `of_external_health_tick`

Polling and snapshots:

- `of_engine_poll_once`
- `of_get_book_snapshot`
- `of_get_analytics_snapshot`
- `of_get_derived_analytics_snapshot`
- `of_get_session_candle_snapshot`
- `of_get_interval_candle_snapshot`
- `of_get_signal_snapshot`

Metadata and ownership helpers:

- `of_api_version`
- `of_build_info`
- `of_get_metrics_json`
- `of_string_free`

## Safety Contract

Callers must:

- pass valid non-null pointers for required pointer arguments
- pass UTF-8 `char*` values where strings are expected
- preserve pointer validity for the full duration of each call
- free owned strings returned by the API using `of_string_free`

Additional ownership rules:

- snapshot getters that write into caller buffers do not allocate for the caller
- functions returning owned `char*` require `of_string_free`
- callback payload pointers are only valid for the duration of the callback
- opaque `of_engine_t*` and `of_subscription_t*` handles must be destroyed/unsubscribed only through exported API calls

## Minimal C Example

```c
#include "orderflow.h"

int main(void) {
    of_engine_t* engine = NULL;
    of_engine_config_t cfg = {0};
    cfg.instance_id = "demo";

    int32_t rc = of_engine_create(&cfg, &engine);
    if (rc != OF_OK) return 1;

    rc = of_engine_start(engine);
    if (rc != OF_OK) {
        of_engine_destroy(engine);
        return 2;
    }

    of_engine_stop(engine);
    of_engine_destroy(engine);
    return 0;
}
```

## Error Semantics

Most functions return `int32_t` values mapped from [`of_error_t`]:

- `OF_OK` for success
- `OF_ERR_INVALID_ARG` for invalid pointers/inputs
- `OF_ERR_STATE` for lifecycle misuse or invalid runtime state
- `OF_ERR_IO`, `OF_ERR_DATA_QUALITY`, and other domain-specific failures

## Snapshot and Callback Payload Contracts

- `of_get_book_snapshot(...)` and `BOOK_SNAPSHOT` callbacks share the same JSON schema
- `of_get_derived_analytics_snapshot(...)` and `DERIVED_ANALYTICS` callbacks share the same JSON schema
- `of_get_session_candle_snapshot(...)` and `of_get_interval_candle_snapshot(...)` are additive snapshot families and do not alter the older analytics/signal contracts
- `inout_len` is both input capacity and output required size; if the buffer is too small, retry with the returned byte count
- payload field names are treated as stable once published; new fields are added additively

## Integration Notes

- Treat engine and subscription handles as opaque; do not cast or inspect internals.
- Keep ABI structs initialized (zero-init is recommended before setting fields).
- Prefer explicit timestamps and sequence numbers for external ingest to maximize quality checks.
- Snapshot functions write the required byte length back through `inout_len`; if the caller buffer is too small, retry with the returned size.
- `BOOK_SNAPSHOT` callbacks emit the same JSON shape as `of_get_book_snapshot(...)`, but only when book state changes for the subscribed symbol.
- `DERIVED_ANALYTICS` callbacks emit the same JSON shape as `of_get_derived_analytics_snapshot(...)`, but only when trade-driven analytics change for the subscribed symbol.
