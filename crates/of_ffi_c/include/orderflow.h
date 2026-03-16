#ifndef ORDERFLOW_H
#define ORDERFLOW_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Opaque runtime engine handle. */
typedef struct of_engine of_engine_t;
/** Opaque subscription handle returned by `of_subscribe`. */
typedef struct of_subscription of_subscription_t;

/** Engine creation configuration. */
typedef struct {
  /** Optional instance identifier override. */
  const char* instance_id;
  /** Optional runtime config file path (.toml/.json). */
  const char* config_path;
  /** Reserved log level field for host integration. */
  uint32_t log_level;
  /** Enables persistence when non-zero. */
  uint8_t enable_persistence;
  /** Max audit log file bytes before rotation. */
  uint64_t audit_max_bytes;
  /** Number of rotated audit files retained. */
  uint32_t audit_max_files;
  /** Comma-separated tokens redacted from audit logs. */
  const char* audit_redact_tokens_csv;
  /** Max retained persisted bytes (0 disables). */
  uint64_t data_retention_max_bytes;
  /** Max retained persisted age in seconds (0 disables). */
  uint64_t data_retention_max_age_secs;
} of_engine_config_t;

/** Symbol descriptor used for subscriptions and snapshots. */
typedef struct {
  /** Venue/exchange identifier (e.g. CME, BINANCE). */
  const char* venue;
  /** Venue-native symbol string. */
  const char* symbol;
  /** Requested depth levels for book subscriptions. */
  uint16_t depth_levels;
} of_symbol_t;

/** Stream kind identifiers used by subscribe/callback APIs. */
typedef enum {
  /** Order-book updates stream. */
  OF_STREAM_BOOK = 1,
  /** Trade prints stream. */
  OF_STREAM_TRADES = 2,
  /** Analytics snapshot stream. */
  OF_STREAM_ANALYTICS = 3,
  /** Signal snapshot stream. */
  OF_STREAM_SIGNALS = 4,
  /** Health transition stream. */
  OF_STREAM_HEALTH = 5,
} of_stream_kind_t;

/** Side constants used by trade/book payloads. */
typedef enum {
  /** Bid/buy side. */
  OF_SIDE_BID = 0,
  /** Ask/sell side. */
  OF_SIDE_ASK = 1,
} of_side_t;

/** Book action constants used by book payloads. */
typedef enum {
  /** Insert or update a book level. */
  OF_BOOK_ACTION_UPSERT = 0,
  /** Delete a book level. */
  OF_BOOK_ACTION_DELETE = 1,
} of_book_action_t;

/** Data-quality bit flags used by ingest, poll, and callbacks. */
typedef enum {
  /** No quality issues. */
  OF_DQ_NONE = 0,
  /** Feed is stale beyond configured threshold. */
  OF_DQ_STALE_FEED = 1u << 0,
  /** Sequence gap detected. */
  OF_DQ_SEQUENCE_GAP = 1u << 1,
  /** Clock skew detected. */
  OF_DQ_CLOCK_SKEW = 1u << 2,
  /** Depth updates were truncated. */
  OF_DQ_DEPTH_TRUNCATED = 1u << 3,
  /** Out-of-order sequence observed. */
  OF_DQ_OUT_OF_ORDER = 1u << 4,
  /** Adapter or external feed is degraded. */
  OF_DQ_ADAPTER_DEGRADED = 1u << 5,
} of_data_quality_flags_t;

/** External trade payload for `of_ingest_trade`. */
typedef struct {
  /** Trade symbol. */
  of_symbol_t symbol;
  /** Trade price. */
  int64_t price;
  /** Trade size/quantity. */
  int64_t size;
  /** Aggressor side (`of_side_t`). */
  uint32_t aggressor_side;
  /** Venue sequence number (0 when unavailable). */
  uint64_t sequence;
  /** Exchange timestamp in nanoseconds. */
  uint64_t ts_exchange_ns;
  /** Local receive timestamp in nanoseconds. */
  uint64_t ts_recv_ns;
} of_trade_t;

/** External book payload for `of_ingest_book`. */
typedef struct {
  /** Book symbol. */
  of_symbol_t symbol;
  /** Side (`of_side_t`). */
  uint32_t side;
  /** Price level index from top of book. */
  uint16_t level;
  /** Level price. */
  int64_t price;
  /** Level size/quantity. */
  int64_t size;
  /** Mutation action (`of_book_action_t`). */
  uint32_t action;
  /** Venue sequence number (0 when unavailable). */
  uint64_t sequence;
  /** Exchange timestamp in nanoseconds. */
  uint64_t ts_exchange_ns;
  /** Local receive timestamp in nanoseconds. */
  uint64_t ts_recv_ns;
} of_book_t;

/** External-feed supervision policy. */
typedef struct {
  /** Maximum allowed ingest silence before stale flag, in milliseconds. */
  uint64_t stale_after_ms;
  /** Non-zero enables sequence-gap/out-of-order checks. */
  uint8_t enforce_sequence;
} of_external_feed_policy_t;

/** Generic callback event envelope. */
typedef struct {
  /** Exchange timestamp in nanoseconds (0 for synthetic snapshots). */
  uint64_t ts_exchange_ns;
  /** Local receive timestamp in nanoseconds (0 for synthetic snapshots). */
  uint64_t ts_recv_ns;
  /** Stream/event kind (`BOOK`, `TRADES`, `ANALYTICS`, `SIGNALS`, `HEALTH`). */
  uint32_t kind;
  /** Pointer to UTF-8 JSON payload bytes. */
  const void* payload;
  /** Payload length in bytes. */
  uint32_t payload_len;
  /** Payload schema id (currently 1). */
  uint32_t schema_id;
  /** Data-quality flags associated with this callback. */
  uint32_t quality_flags;
} of_event_t;

/** Event callback signature. */
typedef void (*of_event_cb)(const of_event_t* ev, void* user_data);

/** Error/status codes returned by C ABI calls. */
typedef enum {
  /** Operation completed successfully. */
  OF_OK = 0,
  /** Invalid argument was supplied. */
  OF_ERR_INVALID_ARG = 1,
  /** Invalid engine/subscription state for operation. */
  OF_ERR_STATE = 2,
  /** I/O failure occurred. */
  OF_ERR_IO = 3,
  /** Authentication/authorization failed. */
  OF_ERR_AUTH = 4,
  /** Backpressure condition detected. */
  OF_ERR_BACKPRESSURE = 5,
  /** Data-quality policy rejected operation. */
  OF_ERR_DATA_QUALITY = 6,
  /** Internal/unknown failure. */
  OF_ERR_INTERNAL = 255,
} of_error_t;

/** Returns ABI version number. */
uint32_t of_api_version(void);
/** Returns static build info string. */
const char* of_build_info(void);

/** Creates a runtime engine instance. */
int32_t of_engine_create(const of_engine_config_t* cfg, of_engine_t** out_engine);
/** Starts engine adapter/session. */
int32_t of_engine_start(of_engine_t* engine);
/** Stops engine adapter/session. */
int32_t of_engine_stop(of_engine_t* engine);
/** Destroys engine and releases owned resources. */
void of_engine_destroy(of_engine_t* engine);

/** Subscribes to a stream kind for a symbol and optional callback delivery. */
int32_t of_subscribe(
  of_engine_t* engine,
  const of_symbol_t* symbol,
  uint32_t kind,
  of_event_cb cb,
  void* user_data,
  of_subscription_t** out_sub);

/** Deactivates a subscription token returned by `of_subscribe`. */
int32_t of_unsubscribe(of_subscription_t* sub);
/** Unsubscribes all streams for a symbol at engine level. */
int32_t of_unsubscribe_symbol(of_engine_t* engine, const of_symbol_t* symbol);
/** Resets per-symbol analytics/session context. */
int32_t of_reset_symbol_session(of_engine_t* engine, const of_symbol_t* symbol);
/** Injects an external trade event into runtime processing. */
int32_t of_ingest_trade(of_engine_t* engine, const of_trade_t* trade, uint32_t quality_flags);
/** Injects an external book event into runtime processing. */
int32_t of_ingest_book(of_engine_t* engine, const of_book_t* book, uint32_t quality_flags);
/** Configures external-feed supervision policy. */
int32_t of_configure_external_feed(of_engine_t* engine, const of_external_feed_policy_t* policy);
/** Marks external feed reconnecting/degraded state. */
int32_t of_external_set_reconnecting(of_engine_t* engine, uint8_t reconnecting);
/** Triggers health reevaluation for stale external-feed detection. */
int32_t of_external_health_tick(of_engine_t* engine);
/** Polls adapter once and dispatches callbacks/snapshots. */
int32_t of_engine_poll_once(of_engine_t* engine, uint32_t quality_flags);

/** Returns current book snapshot JSON for `symbol`. */
int32_t of_get_book_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns current analytics snapshot JSON for `symbol`. */
int32_t of_get_analytics_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns current signal snapshot JSON for `symbol`. */
int32_t of_get_signal_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);

/** Returns engine metrics JSON allocated by the library. */
int32_t of_get_metrics_json(of_engine_t* engine, const char** out_json, uint32_t* out_len);
/** Releases strings allocated by `of_get_metrics_json`. */
void of_string_free(const char* p);

#ifdef __cplusplus
}
#endif

#endif
