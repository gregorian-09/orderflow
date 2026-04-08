# of_ffi_c

`of_ffi_c` exposes a stable C ABI for embedding the Orderflow runtime in non-Rust environments.
It is the native interface used by Python (`ctypes`), Java (JNA), and any C-compatible host runtime.

## ABI Surface

- Engine lifecycle: `of_engine_create`, `of_engine_start`, `of_engine_stop`, `of_engine_destroy`
- Subscription: `of_subscribe`, `of_unsubscribe`, `of_unsubscribe_symbol`
- Ingestion and polling: `of_ingest_trade`, `of_ingest_book`, `of_engine_poll_once`
- Snapshots: `of_get_book_snapshot`, `of_get_analytics_snapshot`, `of_get_derived_analytics_snapshot`, `of_get_session_candle_snapshot`, `of_get_signal_snapshot`
- Health/metrics: `of_get_metrics_json`, `of_get_health_json`, `of_get_health_seq`

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

Subscription stream ids:

- `1`: `BOOK` raw book updates
- `2`: `TRADES` raw trade prints
- `3`: `ANALYTICS` snapshot callbacks
- `4`: `SIGNALS` snapshot callbacks
- `5`: `HEALTH` transition callbacks
- `6`: `BOOK_SNAPSHOT` materialized book snapshot callbacks after book changes
- `7`: `DERIVED_ANALYTICS` session-derived analytics callbacks after trade changes

## Safety Contract

Callers must:

- pass valid non-null pointers for required pointer arguments
- pass UTF-8 `char*` values where strings are expected
- preserve pointer validity for the full duration of each call
- free owned strings returned by the API using `of_string_free`

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

## Integration Notes

- Treat engine and subscription handles as opaque; do not cast or inspect internals.
- Keep ABI structs initialized (zero-init is recommended before setting fields).
- Prefer explicit timestamps and sequence numbers for external ingest to maximize quality checks.
- Snapshot functions write the required byte length back through `inout_len`; if the caller buffer is too small, retry with the returned size.
- `BOOK_SNAPSHOT` callbacks emit the same JSON shape as `of_get_book_snapshot(...)`, but only when book state changes for the subscribed symbol.
- `DERIVED_ANALYTICS` callbacks emit the same JSON shape as `of_get_derived_analytics_snapshot(...)`, but only when trade-driven analytics change for the subscribed symbol.
