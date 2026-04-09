# `of_ffi_c` Reference

`of_ffi_c` exposes the stable C ABI used by C hosts, the Python `ctypes`
binding, and the Java JNA binding.

## Public ABI Map

| Item | Kind | Purpose |
| --- | --- | --- |
| `of_engine_t` | opaque handle | Runtime engine handle |
| `of_subscription_t` | opaque handle | Subscription token |
| `of_engine_config_t` | struct | Engine creation config |
| `of_symbol_t` | struct | Symbol descriptor |
| `of_trade_t` | struct | External trade payload |
| `of_book_t` | struct | External book payload |
| `of_external_feed_policy_t` | struct | External supervision config |
| `of_event_t` | struct | Callback event envelope |
| `of_event_cb` | function pointer | Callback signature |
| `of_error_t` | enum | Error/status codes |
| `of_stream_kind_t` | enum | Subscription stream ids |
| `of_side_t` | enum | Side constants |
| `of_book_action_t` | enum | Book action constants |
| `of_data_quality_flags_t` | enum | Data-quality bit flags |

## Opaque Handles

| Handle | Meaning |
| --- | --- |
| `of_engine_t*` | Native runtime engine instance |
| `of_subscription_t*` | Active subscription token returned by `of_subscribe` |

Callers must treat these as opaque and only manage them through exported ABI
functions.

## Struct Reference

### `of_engine_config_t`

| Field | Type | Meaning |
| --- | --- | --- |
| `instance_id` | `const char*` | Optional runtime instance id |
| `config_path` | `const char*` | Optional `.toml` or `.json` runtime config path |
| `log_level` | `uint32_t` | Reserved log level field |
| `enable_persistence` | `uint8_t` | Non-zero enables persistence |
| `audit_max_bytes` | `uint64_t` | Audit rotation size |
| `audit_max_files` | `uint32_t` | Max rotated audit files retained |
| `audit_redact_tokens_csv` | `const char*` | Comma-separated audit redaction tokens |
| `data_retention_max_bytes` | `uint64_t` | Persistence byte cap |
| `data_retention_max_age_secs` | `uint64_t` | Persistence age cap in seconds |

### `of_symbol_t`

| Field | Type | Meaning |
| --- | --- | --- |
| `venue` | `const char*` | Venue/exchange name |
| `symbol` | `const char*` | Symbol name |
| `depth_levels` | `uint16_t` | Requested depth for book subscriptions |

### `of_trade_t`

| Field | Type | Meaning |
| --- | --- | --- |
| `symbol` | `of_symbol_t` | Target symbol |
| `price` | `int64_t` | Integer-normalized trade price |
| `size` | `int64_t` | Integer-normalized trade size |
| `aggressor_side` | `uint32_t` | `OF_SIDE_BID` or `OF_SIDE_ASK` |
| `sequence` | `uint64_t` | Venue sequence, or `0` if unavailable |
| `ts_exchange_ns` | `uint64_t` | Exchange timestamp |
| `ts_recv_ns` | `uint64_t` | Local receive timestamp |

### `of_book_t`

| Field | Type | Meaning |
| --- | --- | --- |
| `symbol` | `of_symbol_t` | Target symbol |
| `side` | `uint32_t` | `OF_SIDE_BID` or `OF_SIDE_ASK` |
| `level` | `uint16_t` | Depth index from top of book |
| `price` | `int64_t` | Integer-normalized price |
| `size` | `int64_t` | Integer-normalized size |
| `action` | `uint32_t` | `OF_BOOK_ACTION_UPSERT` or `OF_BOOK_ACTION_DELETE` |
| `sequence` | `uint64_t` | Venue sequence, or `0` if unavailable |
| `ts_exchange_ns` | `uint64_t` | Exchange timestamp |
| `ts_recv_ns` | `uint64_t` | Local receive timestamp |

### `of_external_feed_policy_t`

| Field | Type | Meaning |
| --- | --- | --- |
| `stale_after_ms` | `uint64_t` | Max allowed ingest silence before stale status |
| `enforce_sequence` | `uint8_t` | Non-zero enables sequence checks |

### `of_event_t`

| Field | Type | Meaning |
| --- | --- | --- |
| `ts_exchange_ns` | `uint64_t` | Exchange timestamp, or `0` for synthetic snapshots |
| `ts_recv_ns` | `uint64_t` | Receive timestamp, or `0` for synthetic snapshots |
| `kind` | `uint32_t` | Stream kind id |
| `payload` | `const void*` | UTF-8 JSON payload |
| `payload_len` | `uint32_t` | Payload size in bytes |
| `schema_id` | `uint32_t` | Payload schema id, currently `1` |
| `quality_flags` | `uint32_t` | `OF_DQ_*` bitset |

## Enums and Constants

### `of_stream_kind_t`

| Constant | Value | Meaning |
| --- | --- | --- |
| `OF_STREAM_BOOK` | `1` | Raw book update stream |
| `OF_STREAM_TRADES` | `2` | Raw trade stream |
| `OF_STREAM_ANALYTICS` | `3` | Analytics snapshot stream |
| `OF_STREAM_SIGNALS` | `4` | Signal snapshot stream |
| `OF_STREAM_HEALTH` | `5` | Health transition stream |
| `OF_STREAM_BOOK_SNAPSHOT` | `6` | Materialized book snapshot stream |
| `OF_STREAM_DERIVED_ANALYTICS` | `7` | Derived analytics snapshot stream |

### `of_side_t`

| Constant | Value | Meaning |
| --- | --- | --- |
| `OF_SIDE_BID` | `0` | Bid side |
| `OF_SIDE_ASK` | `1` | Ask side |

### `of_book_action_t`

| Constant | Value | Meaning |
| --- | --- | --- |
| `OF_BOOK_ACTION_UPSERT` | `0` | Insert or update |
| `OF_BOOK_ACTION_DELETE` | `1` | Delete |

### `of_data_quality_flags_t`

| Constant | Value | Meaning |
| --- | --- | --- |
| `OF_DQ_NONE` | `0` | No quality issue |
| `OF_DQ_STALE_FEED` | `1 << 0` | Feed stale |
| `OF_DQ_SEQUENCE_GAP` | `1 << 1` | Sequence gap |
| `OF_DQ_CLOCK_SKEW` | `1 << 2` | Clock skew |
| `OF_DQ_DEPTH_TRUNCATED` | `1 << 3` | Depth truncated |
| `OF_DQ_OUT_OF_ORDER` | `1 << 4` | Out-of-order data |
| `OF_DQ_ADAPTER_DEGRADED` | `1 << 5` | Adapter/bridge degraded |

### `of_error_t`

| Constant | Value | Meaning |
| --- | --- | --- |
| `OF_OK` | `0` | Success |
| `OF_ERR_INVALID_ARG` | `1` | Invalid input or insufficient buffer |
| `OF_ERR_STATE` | `2` | Invalid lifecycle or runtime state |
| `OF_ERR_IO` | `3` | I/O failure |
| `OF_ERR_AUTH` | `4` | Authentication/authorization failure |
| `OF_ERR_BACKPRESSURE` | `5` | Backpressure condition |
| `OF_ERR_DATA_QUALITY` | `6` | Quality policy rejected operation |
| `OF_ERR_INTERNAL` | `255` | Internal or unknown failure |

## Function Reference

### Metadata

| Function | Returns | Meaning |
| --- | --- | --- |
| `of_api_version()` | `uint32_t` | ABI version number |
| `of_build_info()` | `const char*` | Static build descriptor string |

### Engine lifecycle

| Function | Returns | Meaning |
| --- | --- | --- |
| `of_engine_create(cfg, out_engine)` | `int32_t` | Creates engine handle |
| `of_engine_start(engine)` | `int32_t` | Starts runtime |
| `of_engine_stop(engine)` | `int32_t` | Stops runtime |
| `of_engine_destroy(engine)` | `void` | Releases engine handle |

### Subscription and polling

| Function | Returns | Meaning |
| --- | --- | --- |
| `of_subscribe(engine, symbol, kind, cb, user_data, out_sub)` | `int32_t` | Subscribes one stream kind for one symbol |
| `of_unsubscribe(sub)` | `int32_t` | Deactivates a subscription token |
| `of_unsubscribe_symbol(engine, symbol)` | `int32_t` | Removes all streams for one symbol |
| `of_reset_symbol_session(engine, symbol)` | `int32_t` | Resets per-symbol session analytics |
| `of_engine_poll_once(engine, quality_flags)` | `int32_t` | Polls adapter once and dispatches callbacks |

### External ingest and supervision

| Function | Returns | Meaning |
| --- | --- | --- |
| `of_ingest_trade(engine, trade, quality_flags)` | `int32_t` | Processes one external trade |
| `of_ingest_book(engine, book, quality_flags)` | `int32_t` | Processes one external book update |
| `of_configure_external_feed(engine, policy)` | `int32_t` | Configures stale/sequence supervision |
| `of_external_set_reconnecting(engine, reconnecting)` | `int32_t` | Marks bridge reconnecting state |
| `of_external_health_tick(engine)` | `int32_t` | Re-evaluates stale/degraded health |

### Snapshot getters

| Function | Returns | Meaning |
| --- | --- | --- |
| `of_get_book_snapshot(engine, symbol, out_buf, inout_len)` | `int32_t` | Writes book snapshot JSON into caller buffer |
| `of_get_analytics_snapshot(engine, symbol, out_buf, inout_len)` | `int32_t` | Writes analytics snapshot JSON |
| `of_get_derived_analytics_snapshot(engine, symbol, out_buf, inout_len)` | `int32_t` | Writes derived analytics snapshot JSON |
| `of_get_session_candle_snapshot(engine, symbol, out_buf, inout_len)` | `int32_t` | Writes session candle snapshot JSON |
| `of_get_interval_candle_snapshot(engine, symbol, window_ns, out_buf, inout_len)` | `int32_t` | Writes interval candle snapshot JSON |
| `of_get_signal_snapshot(engine, symbol, out_buf, inout_len)` | `int32_t` | Writes signal snapshot JSON |

### Metrics and ownership helpers

| Function | Returns | Meaning |
| --- | --- | --- |
| `of_get_metrics_json(engine, out_json, out_len)` | `int32_t` | Allocates and returns metrics JSON |
| `of_string_free(p)` | `void` | Releases strings owned by library |

## Buffer and Ownership Rules

- Snapshot getters use caller-provided buffers.
- `inout_len` is both input capacity and output required size.
- If a snapshot buffer is too small, the function returns `OF_ERR_INVALID_ARG`
  and writes the required byte size back into `inout_len`.
- `of_get_metrics_json` allocates the returned string; callers must free it with
  `of_string_free`.
- Callback payload pointers are valid only for the duration of the callback.

## Payload Compatibility Rules

- `OF_STREAM_BOOK_SNAPSHOT` callbacks use the same JSON shape as
  `of_get_book_snapshot`.
- `OF_STREAM_DERIVED_ANALYTICS` callbacks use the same JSON shape as
  `of_get_derived_analytics_snapshot`.
- Snapshot payload field names are treated as stable once published.
- New fields are added additively rather than replacing existing fields.
