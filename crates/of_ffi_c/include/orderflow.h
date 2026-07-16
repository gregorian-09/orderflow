#ifndef ORDERFLOW_H
#define ORDERFLOW_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Opaque runtime engine handle. */
typedef struct of_engine of_engine_t;
/** Opaque execution engine handle. */
typedef struct of_execution_engine of_execution_engine_t;
/** Opaque concurrent execution engine handle. */
typedef struct of_execution_concurrent_engine of_execution_concurrent_engine_t;
/** Analytics configuration struct — mirrors of_core::AnalyticsConfig. */
typedef struct {
  double agent_small_trade_threshold;
  int64_t institutional_trade_threshold;
  uint64_t cancel_arrival_window_ns;
  uint32_t vpin_volume_bucket;
  uint32_t vpin_max_buckets;
  uint32_t kyle_lambda_max_len;
  uint32_t cvd_max_len;
  uint32_t vol_estimator_max_len;
  uint32_t noise_max_len;
  uint32_t hasbrouck_max_len;
  uint32_t almgren_chriss_max_len;
  uint32_t acd_max_len;
  uint32_t vol_signature_max_len;
  uint32_t agent_max_len;
  uint32_t agent_min_samples;
  uint32_t institutional_max_len;
  uint32_t resiliency_max_len;
  uint32_t spread_decomp_max_len;
  uint32_t regime_max_len;
  uint32_t event_tracker_max_len;
  uint32_t spread_tracker_max_len;
  uint32_t default_max_len;
} of_analytics_config_t;
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
  /** Materialized order-book snapshot stream emitted after book changes. */
  OF_STREAM_BOOK_SNAPSHOT = 6,
  /** Derived analytics snapshot stream emitted after trade-driven analytics changes. */
  OF_STREAM_DERIVED_ANALYTICS = 7,
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
  /** Stream/event kind (`BOOK`, `TRADES`, `ANALYTICS`, `SIGNALS`, `HEALTH`, `BOOK_SNAPSHOT`, `DERIVED_ANALYTICS`). */
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
  /** Pre-trade risk rejected an execution command. */
  OF_ERR_RISK = 7,
  /** Internal/unknown failure. */
  OF_ERR_INTERNAL = 255,
} of_error_t;

/** Execution route and risk configuration. */
typedef struct {
  /** Route identifier. */
  const char* route_id;
  /** Account identifier. */
  const char* account_id;
  /** Venue identifier. */
  const char* venue;
  /** Instrument identifier. */
  const char* instrument;
  /** Enables the route when non-zero. */
  uint8_t enabled;
  /** Enables the kill switch when non-zero. */
  uint8_t kill_switch;
  /** Maximum order quantity; zero disables. */
  int64_t max_order_qty;
  /** Maximum order notional; zero disables. */
  int64_t max_order_notional;
  /** Maximum open orders; zero disables. */
  uint32_t max_open_orders;
  /** Maximum open notional; zero disables. */
  int64_t max_open_notional;
  /** Maximum price distance from reference, in ticks; zero disables. */
  int64_t price_band_ticks;
} of_execution_route_config_t;

/** Execution order request. */
typedef struct {
  const char* client_order_id;
  const char* account_id;
  const char* route_id;
  const char* strategy_id;
  const char* venue;
  const char* instrument;
  uint32_t side;
  uint32_t order_type;
  uint32_t time_in_force;
  int64_t quantity;
  int64_t limit_price;
  int64_t stop_price;
  uint64_t ts_exchange_ns;
  uint64_t ts_recv_ns;
} of_execution_order_request_t;

/** Execution cancel request. */
typedef struct {
  const char* client_order_id;
  const char* orig_client_order_id;
  const char* venue_order_id;
  const char* account_id;
  const char* route_id;
  const char* venue;
  const char* instrument;
  uint64_t ts_recv_ns;
} of_execution_cancel_request_t;

/** Execution amend request. */
typedef struct {
  const char* client_order_id;
  const char* orig_client_order_id;
  const char* venue_order_id;
  const char* account_id;
  const char* route_id;
  const char* venue;
  const char* instrument;
  int64_t quantity;
  int64_t limit_price;
  uint64_t ts_recv_ns;
} of_execution_amend_request_t;

/** Execution event returned through caller-owned event arrays. */
typedef struct {
  uint32_t exec_type;
  uint32_t order_status;
  char client_order_id[41];
  char orig_client_order_id[41];
  char venue_order_id[49];
  char execution_id[49];
  char account_id[33];
  char route_id[33];
  char venue[17];
  char instrument[33];
  int64_t last_qty;
  int64_t last_price;
  int64_t cumulative_qty;
  int64_t leaves_qty;
  int64_t average_price;
  uint64_t ts_exchange_ns;
  uint64_t ts_recv_ns;
  uint32_t reason;
  char text[129];
} of_execution_event_t;

/** Execution order state. */
typedef struct {
  char client_order_id[41];
  char venue_order_id[49];
  char account_id[33];
  char route_id[33];
  char venue[17];
  char instrument[33];
  uint32_t status;
  int64_t order_qty;
  int64_t cumulative_qty;
  int64_t leaves_qty;
  int64_t average_price;
  uint64_t updated_ns;
} of_execution_order_state_t;

/** Execution health snapshot. */
typedef struct {
  uint8_t connected;
  uint8_t degraded;
  uint64_t health_seq;
} of_execution_health_t;

/** Execution metrics snapshot. */
typedef struct {
  uint64_t submitted;
  uint64_t cancelled;
  uint64_t amended;
  uint64_t events_applied;
  uint64_t risk_rejected;
  uint64_t adapter_errors;
  uint64_t recovered;
} of_execution_metrics_t;

/** Execution WAL integrity snapshot for operator diagnostics. */
typedef struct {
  /** Number of valid WAL frames decoded before the first fatal frame error. */
  uint64_t records;
  /** Number of encoded bytes consumed by valid records. */
  uint64_t bytes;
  /** First decoded WAL sequence; meaningful when has_first_sequence is non-zero. */
  uint64_t first_sequence;
  /** Last decoded WAL sequence; meaningful when has_last_sequence is non-zero. */
  uint64_t last_sequence;
  /** Number of checksum failures encountered. */
  uint64_t checksum_failures;
  /** Number of strict sequence failures encountered. */
  uint64_t sequence_failures;
  /** Non-zero when first_sequence is meaningful. */
  uint8_t has_first_sequence;
  /** Non-zero when last_sequence is meaningful. */
  uint8_t has_last_sequence;
  /** Non-zero when the file ends with a partial frame. */
  uint8_t truncated_tail;
  /** Non-zero when all bytes decoded cleanly. */
  uint8_t valid;
} of_execution_wal_integrity_report_t;

/** Concurrent execution worker configuration. */
typedef struct {
  /** Bounded command queue capacity; 0 uses default. */
  uint32_t command_capacity;
  /** Bounded report queue capacity; 0 uses default. */
  uint32_t report_capacity;
  /** Per-command event buffer capacity; 0 uses default. */
  uint32_t event_buffer_capacity;
} of_execution_concurrent_config_t;

/** Concurrent execution command report. */
typedef struct {
  /** Monotonic command sequence. */
  uint64_t sequence;
  /** Command kind (`1=Submit`, `2=Cancel`, `3=Amend`, `4=Poll`, `5=Recover`, `6=Stop`). */
  uint32_t kind;
  /** Result code for the command. */
  int32_t result_code;
  /** Number of events copied or required. */
  uint32_t event_count;
} of_execution_command_report_t;

/** Returns ABI version number. */
uint32_t of_api_version(void);
/** Returns static build info string. */
const char* of_build_info(void);
/** Returns execution ABI version number. */
uint32_t of_execution_api_version(void);
/** Inspects an execution WAL file and writes an integrity report. */
int32_t of_execution_wal_integrity_report(const char* path, of_execution_wal_integrity_report_t* out_report);

/** Creates a simulated execution engine instance. */
int32_t of_execution_engine_create(const of_execution_route_config_t* cfg, of_execution_engine_t** out_engine);
/** Creates a simulated execution engine instance with multiple route/account/symbol configs. */
int32_t of_execution_engine_create_multi(const of_execution_route_config_t* routes, uint32_t route_count, of_execution_engine_t** out_engine);
/** Starts an execution engine. */
int32_t of_execution_engine_start(of_execution_engine_t* engine);
/** Stops an execution engine. */
int32_t of_execution_engine_stop(of_execution_engine_t* engine);
/** Destroys an execution engine. */
void of_execution_engine_destroy(of_execution_engine_t* engine);
/** Submits an execution order and writes events into caller-owned array. */
int32_t of_execution_submit_order(of_execution_engine_t* engine, const of_execution_order_request_t* req, of_execution_event_t* out_events, uint32_t* inout_len);
/** Cancels an execution order and writes events into caller-owned array. */
int32_t of_execution_cancel_order(of_execution_engine_t* engine, const of_execution_cancel_request_t* req, of_execution_event_t* out_events, uint32_t* inout_len);
/** Amends an execution order and writes events into caller-owned array. */
int32_t of_execution_amend_order(of_execution_engine_t* engine, const of_execution_amend_request_t* req, of_execution_event_t* out_events, uint32_t* inout_len);
/** Polls execution events into caller-owned array. */
int32_t of_execution_poll(of_execution_engine_t* engine, of_execution_event_t* out_events, uint32_t* inout_len);
/** Gets current order state by client order id. */
int32_t of_execution_get_order_state(const of_execution_engine_t* engine, const char* client_order_id, of_execution_order_state_t* out_state);
/** Gets execution health. */
int32_t of_execution_health(const of_execution_engine_t* engine, of_execution_health_t* out_health);
/** Gets execution metrics. */
int32_t of_execution_metrics(const of_execution_engine_t* engine, of_execution_metrics_t* out_metrics);
/** Creates and starts a concurrent simulated execution engine instance. */
int32_t of_execution_concurrent_engine_create_multi(const of_execution_route_config_t* routes, uint32_t route_count, const of_execution_concurrent_config_t* config, of_execution_concurrent_engine_t** out_engine);
/** Destroys a concurrent execution engine. */
void of_execution_concurrent_engine_destroy(of_execution_concurrent_engine_t* engine);
/** Requests concurrent execution worker stop. */
int32_t of_execution_concurrent_stop(of_execution_concurrent_engine_t* engine, uint64_t* out_sequence);
/** Sends a non-blocking submit command. */
int32_t of_execution_concurrent_submit_order(of_execution_concurrent_engine_t* engine, const of_execution_order_request_t* req, uint64_t* out_sequence);
/** Sends a non-blocking cancel command. */
int32_t of_execution_concurrent_cancel_order(of_execution_concurrent_engine_t* engine, const of_execution_cancel_request_t* req, uint64_t* out_sequence);
/** Sends a non-blocking amend command. */
int32_t of_execution_concurrent_amend_order(of_execution_concurrent_engine_t* engine, const of_execution_amend_request_t* req, uint64_t* out_sequence);
/** Sends a non-blocking poll command. */
int32_t of_execution_concurrent_poll(of_execution_concurrent_engine_t* engine, uint64_t* out_sequence);
/** Attempts to receive one concurrent command report without blocking. */
int32_t of_execution_concurrent_try_recv_report(of_execution_concurrent_engine_t* engine, of_execution_command_report_t* out_report, of_execution_event_t* out_events, uint32_t* inout_len);

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

/**
 * Sets tickbar aggregation interval for new per-symbol accumulators.
 *
 * A positive `interval_ns` enables tickbar aggregation at the given interval
 * for symbols whose accumulators are created after this call. Zero or negative
 * values disable tickbar aggregation for future accumulators. Existing
 * accumulators are not affected.
 *
 * Requires the `tickbar` feature to be enabled at build time.
 */
int32_t of_engine_set_tickbar_interval(of_engine_t* engine, int64_t interval_ns);

/**
 * Returns current book snapshot JSON for `symbol`.
 *
 * Payload shape:
 * {
 *   "venue": "...",
 *   "symbol": "...",
 *   "bids": [{"level":0,"price":...,"size":...}, ...],
 *   "asks": [{"level":0,"price":...,"size":...}, ...],
 *   "last_sequence": ...,
 *   "ts_exchange_ns": ...,
 *   "ts_recv_ns": ...
 * }
 *
 * On success, `*inout_len` is set to the bytes written.
 * If `out_buf` is too small, the function returns `OF_ERR_INVALID_ARG`
 * and sets `*inout_len` to the required byte length.
 */
int32_t of_get_book_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);

/**
 * Returns current book analytics snapshot JSON for `symbol`.
 *
 * Payload shape:
 * {
 *   "best_bid": ..., "best_ask": ...,
 *   "quoted_spread": ..., "relative_spread_bps": ...,
 *   "microprice": ..., "bid_depth": ..., "ask_depth": ...,
 *   "depth_imbalance_bps": ...
 * }
 *
 * On success, `*inout_len` is set to the bytes written.
 * If `out_buf` is too small, returns `OF_ERR_INVALID_ARG`
 * and sets `*inout_len` to the required byte length.
 */
int32_t of_get_book_analytics_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);

/**
 * Computes weighted average price for an order of `qty` by walking the book.
 * Positive qty = buy (walks asks), negative qty = sell (walks bids).
 *
 * Payload: `{"price": N}` on success, `{}` if insufficient liquidity.
 *
 * On success, `*inout_len` is set to the bytes written.
 * If `out_buf` is too small, returns `OF_ERR_INVALID_ARG`
 * and sets `*inout_len` to the required byte length.
 */
int32_t of_compute_weighted_average_price(of_engine_t* engine, const of_symbol_t* symbol, int64_t qty, void* out_buf, uint32_t* inout_len);

/**
 * Computes depth slope (average volume decay per level) over first `levels` levels.
 *
 * Payload: `{"slope": N.N}`. Returns `{"slope":0.0}` if book has fewer than 2 levels.
 *
 * On success, `*inout_len` is set to the bytes written.
 * If `out_buf` is too small, returns `OF_ERR_INVALID_ARG`
 * and sets `*inout_len` to the required byte length.
 */
int32_t of_compute_depth_slope(of_engine_t* engine, const of_symbol_t* symbol, uint32_t levels, void* out_buf, uint32_t* inout_len);

/**
 * Returns mid price as JSON: `{"mid": N}`, or `{}` if no book data.
 */
int32_t of_get_mid_price(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);

/**
 * Returns last effective spread in bps: `{"bps": N}`.
 */
int32_t of_get_effective_spread_bps(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);

/**
 * Returns average half-spread cost in bps over `window` trades: `{"bps": N}`.
 */
int32_t of_get_half_spread_cost_bps(of_engine_t* engine, const of_symbol_t* symbol, uint32_t window, void* out_buf, uint32_t* inout_len);

/**
 * Returns realised spread in bps for trade `hold_ticks` ago: `{"bps": N}`.
 */
int32_t of_get_realised_spread_bps(of_engine_t* engine, const of_symbol_t* symbol, uint32_t hold_ticks, void* out_buf, uint32_t* inout_len);

/**
 * Returns book-event analytics snapshot JSON over `window_ns` window.
 *
 * Payload includes bid/ask arrival/cancel rates, change intensity, and event volumes.
 */
int32_t of_get_book_event_analytics(of_engine_t* engine, const of_symbol_t* symbol, uint64_t window_ns, void* out_buf, uint32_t* inout_len);

/**
 * Returns resiliency snapshot JSON (recovery time, depth elasticity).
 */
int32_t of_get_resiliency_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);

/** Returns VPIN snapshot JSON (vpin, z-score, mean, std, toxicity). */
int32_t of_get_vpin_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns Kyle's Lambda snapshot JSON (lambda, R², avg lambda). */
int32_t of_get_kyle_lambda_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns Amihud illiquidity snapshot JSON. */
int32_t of_get_amihud_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns CVD enhancement snapshot JSON (delta ratio, z-score, divergence). */
int32_t of_get_cvd_enhancement_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns pattern detection snapshot JSON (imbalance, iceberg, hidden accumulation/distribution, session type). */
int32_t of_get_pattern_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns volatility snapshot JSON (classic RV, Parkinson, Garman-Klass, Yang-Zhang). */
int32_t of_get_volatility_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns microstructure noise snapshot JSON. */
int32_t of_get_noise_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns Hasbrouck VAR snapshot JSON (permanent/temporary impact, information share). */
int32_t of_get_hasbrouck_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns Almgren-Chriss market impact snapshot JSON. */
int32_t of_get_almgren_chriss_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns spread decomposition snapshot JSON. */
int32_t of_get_spread_decomp_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns ACD trade duration model snapshot JSON. */
int32_t of_get_acd_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns market regime snapshot JSON. */
int32_t of_get_regime_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns kinetic energy snapshot JSON. */
int32_t of_get_kinetic_energy_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns dark pool analytics snapshot JSON. */
int32_t of_get_dark_pool_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns options flow snapshot JSON. */
int32_t of_get_options_flow_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns futures analytics snapshot JSON. */
int32_t of_get_futures_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);

/** Returns volatility signature snapshot JSON for `symbol`. */
int32_t of_get_vol_signature_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns agent type identification snapshot JSON for `symbol`. */
int32_t of_get_agent_type_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns dark-lit correlation snapshot JSON for `symbol`. */
int32_t of_get_dark_lit_correlation_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns institutional flow snapshot JSON for `symbol`. */
int32_t of_get_institutional_flow_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns OI analysis snapshot JSON for `symbol`. */
int32_t of_get_oi_analysis_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);

/** Returns current analytics snapshot JSON for `symbol`. */
int32_t of_get_analytics_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns current derived analytics snapshot JSON for `symbol`. */
int32_t of_get_derived_analytics_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns current session candle snapshot JSON for `symbol`. */
int32_t of_get_session_candle_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns rolling interval candle snapshot JSON for `symbol` over `window_ns`. */
int32_t of_get_interval_candle_snapshot(of_engine_t* engine, const of_symbol_t* symbol, uint64_t window_ns, void* out_buf, uint32_t* inout_len);
/**
 * Returns completed bar series JSON array for `symbol`.
 *
 * Requires the `tickbar` feature to be enabled at build time.
 * Returns an empty array `[]` when tickbar aggregation is not configured for the symbol.
 *
 * Each bar object:
 *   {"timestamp_ns":...,"open":...,"high":...,"low":...,"close":...,"volume":...,"tick_count":...,"vwap":...}
 *
 * On success, `*inout_len` is set to the bytes written.
 * If `out_buf` is too small, the function returns `OF_ERR_INVALID_ARG`
 * and sets `*inout_len` to the required byte length.
 */
int32_t of_get_bar_series(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);
/** Returns current signal snapshot JSON for `symbol`. */
int32_t of_get_signal_snapshot(of_engine_t* engine, const of_symbol_t* symbol, void* out_buf, uint32_t* inout_len);

/** Returns engine metrics JSON allocated by the library. */
int32_t of_get_metrics_json(of_engine_t* engine, const char** out_json, uint32_t* out_len);
/** Releases strings allocated by `of_get_metrics_json`. */
void of_string_free(const char* p);

/** Computes LOB feature snapshot from engine book state and flow metrics. */
int32_t of_compute_lob_features(of_engine_t* engine, const of_symbol_t* symbol, double trade_imbalance, double cancel_rate, double arrival_rate, void* out_buf, uint32_t* inout_len);

/** Override analytics thresholds and buffer sizes. Pass NULL to reset to defaults. */
int32_t of_engine_set_analytics_config(of_engine_t* engine, const of_analytics_config_t* config);

#ifdef __cplusplus
}
#endif

#endif
