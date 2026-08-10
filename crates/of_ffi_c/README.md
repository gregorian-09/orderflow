# of_ffi_c

`of_ffi_c` exposes a stable C ABI for embedding the Orderflow runtime in non-Rust environments.
It is the native interface used by Python (`ctypes`), Java (JNA), and any C-compatible host runtime.

## ABI Surface

- Engine lifecycle: `of_engine_create`, `of_engine_start`, `of_engine_stop`, `of_engine_destroy`
- Subscription: `of_subscribe`, `of_unsubscribe`, `of_unsubscribe_symbol`, `of_reset_symbol_session`
- External ingest and supervision: `of_ingest_trade`, `of_ingest_book`, `of_configure_external_feed`, `of_external_set_reconnecting`, `of_external_health_tick`
- Polling and snapshots: `of_engine_poll_once`, `of_get_book_snapshot`, `of_get_analytics_snapshot`, `of_get_derived_analytics_snapshot`, `of_get_session_candle_snapshot`, `of_get_interval_candle_snapshot`, `of_get_signal_snapshot`
- Metrics, adapter discovery, and memory management:
  `of_get_metrics_json`, `of_get_adapter_inventory_json`,
  `of_get_active_adapter_status_json`, `of_get_signal_descriptors_json`,
  `of_get_signal_explanation_json`, `of_get_signal_metrics_json`,
  `of_string_free`

The machine-readable ABI inventory lives in `bindings/api_manifest.toml`.
Maintainers should update it whenever they add a C ABI symbol. The manifest is
validated against `include/orderflow.h` by `tools/check_api_manifest.py`, and
`tools/check_ffi_exports.sh` reads the same manifest when checking the compiled
shared library. `tools/generate_binding_signatures.py` then emits exact Python
`ctypes` and Java JNA declarations in manifest order from the validated header.
CI runs it with `--check`, so the header, inventory, exports, and generated
declarations cannot drift silently.

## New In 0.4.0

`0.4.0` keeps all existing analytics/runtime C ABI symbols valid and adds a
separate execution ABI family. Existing hosts that only call
`of_engine_create`, subscribe, poll, ingest, and read snapshots do not need to
change those call sites.

New execution ABI concepts:

- `of_execution_engine_t`: synchronous simulated execution handle
- `of_execution_engine_create` and `of_execution_engine_create_multi`:
  single-route and multi-route construction
- `of_execution_submit_order`, `of_execution_cancel_order`,
  `of_execution_amend_order`, and `of_execution_poll`
- `of_execution_order_state`, `of_execution_health`, and
  `of_execution_metrics`
- `of_execution_wal_integrity_report`: offline operator diagnostic for a
  single binary execution WAL file
- `of_execution_segmented_wal_integrity_report`: offline operator diagnostic
  for a rotated execution WAL directory
- `of_execution_checkpoint_store_integrity_report`: offline operator
  diagnostic for an execution checkpoint directory
- `of_execution_concurrent_engine_t`: bounded worker for many command
  producers and one deterministic execution owner
- `of_execution_command_report_t`: typed command completion report with a
  caller-owned event buffer
- `bindings/api_manifest.toml`: the first machine-readable C ABI manifest for
  export validation and binding parity checks
- generated Python and Java low-level signatures, with manual ergonomic
  wrappers retained above the mechanical FFI layer
- `of_get_signal_descriptors_json`: allocated JSON inventory for built-in
  signal descriptors, requirements, parameters, and output semantics
- `of_get_signal_explanation_json`: allocated JSON explanation for the latest
  signal emitted for one symbol
- `of_get_signal_metrics_json`: allocated JSON summary of signal state counts,
  confidence, quality flags, and explanation coverage

The execution ABI is additive and intentionally separate from the market-data
engine ABI. That separation lets C, Python, Java, and other FFI users adopt OMS
workflows without destabilizing existing analytics deployments.

Version policy:

- `of_ffi_c` publishes as `0.4.0` with the established native library line;
- the Rust execution crates behind the ABI publish as `0.1.0`;
- native headers and libraries should still be upgraded together.

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
- `of_get_adapter_inventory_json`
- `of_get_active_adapter_status_json`
- `of_get_signal_descriptors_json`
- `of_get_signal_explanation_json`
- `of_get_signal_metrics_json`
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

When the runtime backpressure limit is enabled through
`OF_RUNTIME_MAX_EVENTS_PER_POLL`, `of_engine_poll_once` returns
`OF_ERR_BACKPRESSURE` if a poll drains more events than the configured limit.

Metadata and ownership helpers:

- `of_api_version`
- `of_build_info`
- `of_get_adapter_inventory_json`
- `of_get_active_adapter_status_json`
- `of_execution_wal_integrity_report`
- `of_execution_segmented_wal_integrity_report`
- `of_execution_checkpoint_store_integrity_report`
- `of_get_metrics_json`
- `of_string_free`

Execution recovery diagnostics:

- `of_execution_wal_integrity_report(path, out_report)` reads a single WAL file
  and fills `of_execution_wal_integrity_report_t`.
- `of_execution_segmented_wal_integrity_report(root, out_report)` reads
  `wal-*.ofwal` files under a segmented WAL root and fills
  `of_execution_segmented_wal_integrity_report_t`.
- `of_execution_checkpoint_store_integrity_report(root, out_report)` reads
  `.ofchk` checkpoint files under a checkpoint root and fills
  `of_execution_checkpoint_store_integrity_report_t`.
- It is an offline/operator path, not an order-submission hot-path function.
- Empty WAL files or directories are valid and report no first/last sequence.
- Empty checkpoint directories are valid and report no latest checkpoint.
- Corrupt or truncated bytes return `OF_OK` with `valid = 0` so operators can
  inspect the report; missing/unreadable files or directories return
  `OF_ERR_IO`.
- Checkpoint diagnostics report discovered, valid, and invalid checkpoint file
  counts, total checkpoint bytes, and the latest valid checkpoint id, covered
  WAL sequence, and creation timestamp when one exists.

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

## Real-World Use Cases

### 1. Embed the runtime in a C or C++ trading host

Use the lifecycle, subscription, and polling APIs directly from a native host
process that already owns process supervision and deployment.

### 2. Drive Python or Java bindings from the same native ABI

The Python and Java packages both rely on this ABI, so host-side operators can
reason about one native contract instead of three unrelated APIs.

### 3. Build a custom host-side event pump

Use callbacks for snapshot/event delivery and poll-driven control for host-side
scheduling.

## Detailed Example: Poll And Read Snapshots

```c
#include "orderflow.h"
#include <stdint.h>
#include <stdio.h>

int main(void) {
  of_engine_t* engine = NULL;
  of_engine_config_t cfg = {0};
  cfg.instance_id = "native-demo";

  if (of_engine_create(&cfg, &engine) != OF_OK) return 1;
  if (of_engine_start(engine) != OF_OK) return 2;

  of_symbol_t symbol = {0};
  symbol.venue = "SIM";
  symbol.symbol = "ESM6";
  symbol.depth_levels = 10;

  if (of_subscribe(engine, &symbol, OF_STREAM_ANALYTICS, NULL, NULL, NULL) != OF_OK) return 3;

  if (of_engine_poll_once(engine, OF_DQ_NONE) != OF_OK) return 4;

  char buf[2048];
  uint32_t len = (uint32_t)sizeof(buf);
  if (of_get_analytics_snapshot(engine, &symbol, buf, &len) == OF_OK) {
    printf("analytics: %.*s\n", (int)len, buf);
  }

  len = (uint32_t)sizeof(buf);
  if (of_get_book_snapshot(engine, &symbol, buf, &len) == OF_OK) {
    printf("book: %.*s\n", (int)len, buf);
  }

  of_engine_stop(engine);
  of_engine_destroy(engine);
  return 0;
}
```
- Prefer explicit timestamps and sequence numbers for external ingest to maximize quality checks.
- Snapshot functions write the required byte length back through `inout_len`; if the caller buffer is too small, retry with the returned size.
- `BOOK_SNAPSHOT` callbacks emit the same JSON shape as `of_get_book_snapshot(...)`, but only when book state changes for the subscribed symbol.
- `DERIVED_ANALYTICS` callbacks emit the same JSON shape as `of_get_derived_analytics_snapshot(...)`, but only when trade-driven analytics change for the subscribed symbol.
