#![allow(non_camel_case_types)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![doc = include_str!("../README.md")]

mod support;

use std::ffi::{c_char, c_void, CString};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use of_adapters::{AdapterConfig, ProviderKind};
use of_core::{
    AnalyticsConfig, AnalyticsSnapshot, BookUpdate, DataQualityFlags, SignalState, SymbolId,
    TradePrint,
};
use of_execution::{
    recover_latest_checkpoint_from_segmented_wal_roots, simulated_engine_with_routes,
    AllowAllRiskGate, CheckpointStoreIntegrityReport, ConcurrentExecutionConfig,
    ConcurrentExecutionEngine, ConcurrentExecutionError, ExecutionCommand, ExecutionCommandKind,
    ExecutionCommandReport, ExecutionEngine, ExecutionError, ExecutionEventBuffer,
    FileExecutionCheckpointStore, InMemoryJournal, RouteConfig, SegmentedWalExecutionJournal,
    SimExecutionAdapter, WalSegmentIntegrityReport,
};
use of_execution_algos::{AlgoProgress, ChildOrderPlan, ParentOrder, TwapSlicePlanner};
use of_execution_core::{
    AmendRequest, CancelRequest, ExecutionEvent, ExecutionSymbol, ExecutionText, ExecutionType,
    FixedAscii, OrderPrice, OrderQty, OrderRequest, OrderSide, OrderState, OrderStatus, OrderType,
    RiskLimits, RiskRejectReason, StrategyId, TimeInForce, VenueOrderId, WalIntegrityReport,
};
use of_runtime::{
    adapter_inventory_json as runtime_adapter_inventory_json, build_default_engine,
    load_engine_config_from_path, signal_descriptor_inventory_json, DefaultEngine, EngineConfig,
    ExternalFeedPolicy, RuntimeError,
};
use of_signals::{
    validate_signal_replay_events, SignalConfig, SignalConfigParameter, SignalConfigValue,
    SignalRegistry, SignalReplayEvent, SignalValidationConfig,
};
#[cfg(feature = "tickbar")]
use support::format_bar_series;
use support::{
    action_from_ffi, cstr_to_string, dispatch_callbacks, dispatch_health_callbacks, escape_json,
    format_acd_snapshot, format_agent_type_snapshot, format_almgren_chriss_snapshot,
    format_amihud_snapshot, format_analytics_snapshot, format_book_analytics_snapshot,
    format_book_event_analytics_snapshot, format_book_snapshot, format_cvd_enhancement_snapshot,
    format_dark_lit_correlation_snapshot, format_dark_pool_snapshot,
    format_derived_analytics_snapshot, format_futures_snapshot, format_hasbrouck_snapshot,
    format_institutional_flow_snapshot, format_interval_candle_snapshot,
    format_kinetic_energy_snapshot, format_kyle_lambda_snapshot, format_lob_feature_snapshot,
    format_noise_snapshot, format_oi_analysis_snapshot, format_options_flow_snapshot,
    format_pattern_snapshot, format_regime_snapshot, format_resiliency_snapshot,
    format_session_candle_snapshot, format_spread_decomp_snapshot, format_vol_signature_snapshot,
    format_volatility_snapshot, format_vpin_snapshot, non_empty_string, parse_csv, side_from_ffi,
    symbol_from_ffi, symbol_from_ffi_ref, write_json_to_c_buffer,
};

const API_VERSION: u32 = 0x0001_0000;
const EXECUTION_API_VERSION: u32 = 0x0001_0000;
const BUILD_INFO: &[u8] = concat!("of_ffi_c/", env!("CARGO_PKG_VERSION"), "\0").as_bytes();
const FFI_EVENT_BUFFER_CAP: usize = 32;

/// Analytics configuration passed to [`of_engine_set_analytics_config`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct of_analytics_config_t {
    /// Trade-size threshold for agent classification.
    pub agent_small_trade_threshold: f64,
    /// Large-trade threshold for institutional-flow classification.
    pub institutional_trade_threshold: i64,
    /// Window for cancel/arrival-rate computation.
    pub cancel_arrival_window_ns: u64,
    /// Volume per VPIN bucket.
    pub vpin_volume_bucket: u32,
    /// Max VPIN buckets.
    pub vpin_max_buckets: u32,
    /// Kyle's Lambda rolling window.
    pub kyle_lambda_max_len: u32,
    /// CVD enhancement rolling window.
    pub cvd_max_len: u32,
    /// Volatility estimator rolling window.
    pub vol_estimator_max_len: u32,
    /// Microstructure noise rolling window.
    pub noise_max_len: u32,
    /// Hasbrouck VAR rolling window.
    pub hasbrouck_max_len: u32,
    /// Almgren-Chriss rolling window.
    pub almgren_chriss_max_len: u32,
    /// ACD model rolling window.
    pub acd_max_len: u32,
    /// Volatility signature rolling window.
    pub vol_signature_max_len: u32,
    /// Agent detector rolling window.
    pub agent_max_len: u32,
    /// Minimum samples for agent classification.
    pub agent_min_samples: u32,
    /// Institutional-flow rolling window.
    pub institutional_max_len: u32,
    /// Resiliency tracker rolling window.
    pub resiliency_max_len: u32,
    /// Spread-decomposition rolling window.
    pub spread_decomp_max_len: u32,
    /// Regime detector rolling window.
    pub regime_max_len: u32,
    /// Book-event tracker capacity.
    pub event_tracker_max_len: u32,
    /// Spread tracker capacity.
    pub spread_tracker_max_len: u32,
    /// Default rolling window for trackers not otherwise specified.
    pub default_max_len: u32,
}

impl From<of_analytics_config_t> for AnalyticsConfig {
    fn from(value: of_analytics_config_t) -> Self {
        Self {
            vpin_volume_bucket: i64::from(value.vpin_volume_bucket),
            vpin_max_buckets: value.vpin_max_buckets,
            kyle_lambda_max_len: value.kyle_lambda_max_len,
            cvd_max_len: value.cvd_max_len,
            vol_estimator_max_len: value.vol_estimator_max_len,
            noise_max_len: value.noise_max_len,
            hasbrouck_max_len: value.hasbrouck_max_len,
            almgren_chriss_max_len: value.almgren_chriss_max_len,
            acd_max_len: value.acd_max_len,
            vol_signature_max_len: value.vol_signature_max_len,
            agent_max_len: value.agent_max_len,
            agent_min_samples: value.agent_min_samples,
            agent_small_trade_threshold: value.agent_small_trade_threshold,
            institutional_trade_threshold: value.institutional_trade_threshold,
            institutional_max_len: value.institutional_max_len,
            resiliency_max_len: value.resiliency_max_len,
            spread_decomp_max_len: value.spread_decomp_max_len,
            regime_max_len: value.regime_max_len,
            cancel_arrival_window_ns: value.cancel_arrival_window_ns,
            event_tracker_max_len: value.event_tracker_max_len,
            spread_tracker_max_len: value.spread_tracker_max_len,
            default_max_len: value.default_max_len,
        }
    }
}

/// Engine configuration passed to [`of_engine_create`].
#[repr(C)]
pub struct of_engine_config_t {
    /// Optional runtime instance identifier.
    pub instance_id: *const c_char,
    /// Optional config file path loaded by the runtime.
    pub config_path: *const c_char,
    /// Reserved log-level field for host integrations.
    pub log_level: u32,
    /// Non-zero enables persistence.
    pub enable_persistence: u8,
    /// Audit log rotation size threshold in bytes.
    pub audit_max_bytes: u64,
    /// Number of rotated audit log files to retain.
    pub audit_max_files: u32,
    /// Comma-separated redaction token list.
    pub audit_redact_tokens_csv: *const c_char,
    /// Maximum retained persistence bytes (0 disables).
    pub data_retention_max_bytes: u64,
    /// Maximum retained persistence age seconds (0 disables).
    pub data_retention_max_age_secs: u64,
}

/// Symbol descriptor used by subscription and snapshot functions.
#[repr(C)]
pub struct of_symbol_t {
    /// Venue or exchange identifier.
    pub venue: *const c_char,
    /// Venue-native symbol identifier.
    pub symbol: *const c_char,
    /// Requested level-2 depth for subscriptions.
    pub depth_levels: u16,
}

/// External trade payload accepted by [`of_ingest_trade`].
#[repr(C)]
pub struct of_trade_t {
    /// Trade symbol descriptor.
    pub symbol: of_symbol_t,
    /// Trade price in integer units.
    pub price: i64,
    /// Trade quantity.
    pub size: i64,
    /// Aggressor side (`0=Bid`, `1=Ask`).
    pub aggressor_side: u32,
    /// Venue sequence number.
    pub sequence: u64,
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// External order-book payload accepted by [`of_ingest_book`].
#[repr(C)]
pub struct of_book_t {
    /// Book update symbol descriptor.
    pub symbol: of_symbol_t,
    /// Book side (`0=Bid`, `1=Ask`).
    pub side: u32,
    /// Price level index from top of book.
    pub level: u16,
    /// Level price in integer units.
    pub price: i64,
    /// Level quantity.
    pub size: i64,
    /// Mutation action (`0=Upsert`, `1=Delete`).
    pub action: u32,
    /// Venue sequence number.
    pub sequence: u64,
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// External-feed quality policy configured via [`of_configure_external_feed`].
#[repr(C)]
pub struct of_external_feed_policy_t {
    /// Stale threshold in milliseconds.
    pub stale_after_ms: u64,
    /// Non-zero enables sequence checks.
    pub enforce_sequence: u8,
}

/// Error codes returned by C ABI functions.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum of_error_t {
    /// Success.
    OF_OK = 0,
    /// Invalid argument.
    OF_ERR_INVALID_ARG = 1,
    /// Invalid runtime state.
    OF_ERR_STATE = 2,
    /// I/O failure.
    OF_ERR_IO = 3,
    /// Authentication failure.
    OF_ERR_AUTH = 4,
    /// Backpressure condition.
    OF_ERR_BACKPRESSURE = 5,
    /// Data-quality policy rejection.
    OF_ERR_DATA_QUALITY = 6,
    /// Pre-trade risk rejection.
    OF_ERR_RISK = 7,
    /// Internal/unknown failure.
    OF_ERR_INTERNAL = 255,
}

/// Execution route and risk configuration.
#[repr(C)]
pub struct of_execution_route_config_t {
    /// Route identifier.
    pub route_id: *const c_char,
    /// Account identifier.
    pub account_id: *const c_char,
    /// Venue identifier.
    pub venue: *const c_char,
    /// Instrument identifier.
    pub instrument: *const c_char,
    /// Non-zero enables the route.
    pub enabled: u8,
    /// Non-zero enables the kill switch.
    pub kill_switch: u8,
    /// Maximum order quantity; zero disables.
    pub max_order_qty: i64,
    /// Maximum order notional; zero disables.
    pub max_order_notional: i64,
    /// Maximum open orders; zero disables.
    pub max_open_orders: u32,
    /// Maximum open notional; zero disables.
    pub max_open_notional: i64,
    /// Maximum price distance from reference, in ticks; zero disables.
    pub price_band_ticks: i64,
}

/// Execution order request.
#[repr(C)]
pub struct of_execution_order_request_t {
    /// Client order id.
    pub client_order_id: *const c_char,
    /// Account id.
    pub account_id: *const c_char,
    /// Route id.
    pub route_id: *const c_char,
    /// Strategy id.
    pub strategy_id: *const c_char,
    /// Venue id.
    pub venue: *const c_char,
    /// Instrument id.
    pub instrument: *const c_char,
    /// Side (`1=Buy`, `2=Sell`).
    pub side: u32,
    /// Order type (`1=Market`, `2=Limit`, `3=Stop`, `4=StopLimit`).
    pub order_type: u32,
    /// Time-in-force (`1=Day`, `2=Gtc`, `3=Ioc`, `4=Fok`, `5=Gtd`).
    pub time_in_force: u32,
    /// Quantity in integer-normalized units.
    pub quantity: i64,
    /// Limit price in integer-normalized units, or zero.
    pub limit_price: i64,
    /// Stop price in integer-normalized units, or zero.
    pub stop_price: i64,
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive/create timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// Execution cancel request.
#[repr(C)]
pub struct of_execution_cancel_request_t {
    /// Client id for the cancel request.
    pub client_order_id: *const c_char,
    /// Last accepted client order id.
    pub orig_client_order_id: *const c_char,
    /// Venue order id, if known.
    pub venue_order_id: *const c_char,
    /// Account id.
    pub account_id: *const c_char,
    /// Route id.
    pub route_id: *const c_char,
    /// Venue id.
    pub venue: *const c_char,
    /// Instrument id.
    pub instrument: *const c_char,
    /// Local receive/create timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// Execution amend request.
#[repr(C)]
pub struct of_execution_amend_request_t {
    /// Client id for the replacement request.
    pub client_order_id: *const c_char,
    /// Last accepted client order id.
    pub orig_client_order_id: *const c_char,
    /// Venue order id, if known.
    pub venue_order_id: *const c_char,
    /// Account id.
    pub account_id: *const c_char,
    /// Route id.
    pub route_id: *const c_char,
    /// Venue id.
    pub venue: *const c_char,
    /// Instrument id.
    pub instrument: *const c_char,
    /// Replacement quantity.
    pub quantity: i64,
    /// Replacement limit price.
    pub limit_price: i64,
    /// Local receive/create timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// Execution event returned by execution C APIs.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct of_execution_event_t {
    /// Execution type.
    pub exec_type: u32,
    /// Current order status.
    pub order_status: u32,
    /// Client order id.
    pub client_order_id: [c_char; 41],
    /// Original client order id.
    pub orig_client_order_id: [c_char; 41],
    /// Venue order id.
    pub venue_order_id: [c_char; 49],
    /// Execution id.
    pub execution_id: [c_char; 49],
    /// Account id.
    pub account_id: [c_char; 33],
    /// Route id.
    pub route_id: [c_char; 33],
    /// Venue id.
    pub venue: [c_char; 17],
    /// Instrument id.
    pub instrument: [c_char; 33],
    /// Last fill quantity.
    pub last_qty: i64,
    /// Last fill price.
    pub last_price: i64,
    /// Cumulative quantity.
    pub cumulative_qty: i64,
    /// Leaves quantity.
    pub leaves_qty: i64,
    /// Average price.
    pub average_price: i64,
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
    /// Structured reason code.
    pub reason: u32,
    /// Bounded diagnostic text.
    pub text: [c_char; 129],
}

/// Execution order state returned by state query.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct of_execution_order_state_t {
    /// Client order id.
    pub client_order_id: [c_char; 41],
    /// Venue order id.
    pub venue_order_id: [c_char; 49],
    /// Account id.
    pub account_id: [c_char; 33],
    /// Route id.
    pub route_id: [c_char; 33],
    /// Venue id.
    pub venue: [c_char; 17],
    /// Instrument id.
    pub instrument: [c_char; 33],
    /// Order status.
    pub status: u32,
    /// Original order quantity.
    pub order_qty: i64,
    /// Cumulative quantity.
    pub cumulative_qty: i64,
    /// Leaves quantity.
    pub leaves_qty: i64,
    /// Average price.
    pub average_price: i64,
    /// Last update timestamp in nanoseconds.
    pub updated_ns: u64,
}

/// Execution health snapshot.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct of_execution_health_t {
    /// Non-zero when connected.
    pub connected: u8,
    /// Non-zero when degraded.
    pub degraded: u8,
    /// Monotonic health sequence.
    pub health_seq: u64,
}

/// Execution metrics snapshot.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct of_execution_metrics_t {
    /// Submitted orders accepted locally.
    pub submitted: u64,
    /// Cancel commands accepted locally.
    pub cancelled: u64,
    /// Amend commands accepted locally.
    pub amended: u64,
    /// Events applied.
    pub events_applied: u64,
    /// Risk rejections.
    pub risk_rejected: u64,
    /// Adapter errors.
    pub adapter_errors: u64,
    /// Recovery events applied.
    pub recovered: u64,
}

/// Execution WAL integrity report returned by
/// [`of_execution_wal_integrity_report`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct of_execution_wal_integrity_report_t {
    /// Number of valid WAL frames decoded before the first fatal frame error.
    pub records: u64,
    /// Number of encoded bytes consumed by valid records.
    pub bytes: u64,
    /// First decoded WAL sequence, valid when `has_first_sequence != 0`.
    pub first_sequence: u64,
    /// Last decoded WAL sequence, valid when `has_last_sequence != 0`.
    pub last_sequence: u64,
    /// Number of checksum failures encountered.
    pub checksum_failures: u64,
    /// Number of strict sequence failures encountered.
    pub sequence_failures: u64,
    /// Non-zero when `first_sequence` is meaningful.
    pub has_first_sequence: u8,
    /// Non-zero when `last_sequence` is meaningful.
    pub has_last_sequence: u8,
    /// Non-zero when the input ended with a partial frame.
    pub truncated_tail: u8,
    /// Non-zero when all bytes decoded cleanly.
    pub valid: u8,
}

/// Segmented execution WAL integrity report returned by
/// [`of_execution_segmented_wal_integrity_report`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct of_execution_segmented_wal_integrity_report_t {
    /// Number of segment files inspected.
    pub segments: u64,
    /// Number of valid WAL frames decoded before the first fatal frame error.
    pub records: u64,
    /// Number of encoded bytes consumed by valid records.
    pub bytes: u64,
    /// First decoded WAL sequence, valid when `has_first_sequence != 0`.
    pub first_sequence: u64,
    /// Last decoded WAL sequence, valid when `has_last_sequence != 0`.
    pub last_sequence: u64,
    /// Number of checksum failures encountered.
    pub checksum_failures: u64,
    /// Number of strict sequence failures encountered.
    pub sequence_failures: u64,
    /// Non-zero when `first_sequence` is meaningful.
    pub has_first_sequence: u8,
    /// Non-zero when `last_sequence` is meaningful.
    pub has_last_sequence: u8,
    /// Non-zero when all inspected segments decoded cleanly.
    pub valid: u8,
}

/// Execution checkpoint store integrity report returned by
/// [`of_execution_checkpoint_store_integrity_report`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct of_execution_checkpoint_store_integrity_report_t {
    /// Number of checkpoint files discovered.
    pub checkpoint_files: u64,
    /// Number of checkpoint files decoded and checksum-validated.
    pub valid_checkpoints: u64,
    /// Number of checkpoint files that failed validation.
    pub invalid_checkpoints: u64,
    /// Total bytes across discovered checkpoint files.
    pub bytes: u64,
    /// Latest valid checkpoint id, meaningful when `has_latest != 0`.
    pub latest_checkpoint_id: u64,
    /// Last WAL sequence covered by the latest valid checkpoint.
    pub latest_last_applied_sequence: u64,
    /// Creation timestamp for the latest valid checkpoint.
    pub latest_created_ns: u64,
    /// Non-zero when latest checkpoint fields are meaningful.
    pub has_latest: u8,
    /// Non-zero when all discovered checkpoints decoded cleanly.
    pub valid: u8,
}

/// Read-only execution recovery report configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct of_execution_recovery_config_t {
    /// Existing segmented execution WAL root.
    pub wal_root: *const c_char,
    /// Existing checkpoint root, or null/empty when checkpoint-free replay is
    /// allowed.
    pub checkpoint_root: *const c_char,
    /// Non-zero requires a valid checkpoint before replay.
    pub require_checkpoint: u8,
}

/// Concurrent execution worker configuration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct of_execution_concurrent_config_t {
    /// Bounded command queue capacity.
    pub command_capacity: u32,
    /// Bounded report queue capacity.
    pub report_capacity: u32,
    /// Per-command event buffer capacity.
    pub event_buffer_capacity: u32,
}

/// Concurrent execution command report.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct of_execution_command_report_t {
    /// Monotonic command sequence.
    pub sequence: u64,
    /// Command kind.
    pub kind: u32,
    /// Result code for the command.
    pub result_code: i32,
    /// Number of events copied to the caller event array.
    pub event_count: u32,
}

/// Parent-order configuration for a deterministic TWAP algorithm.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct of_execution_twap_config_t {
    /// Parent order identifier.
    pub parent_order_id: *const c_char,
    /// Trading account identifier.
    pub account_id: *const c_char,
    /// Default execution route identifier.
    pub route_id: *const c_char,
    /// Strategy attribution identifier.
    pub strategy_id: *const c_char,
    /// Venue identifier.
    pub venue: *const c_char,
    /// Instrument identifier.
    pub instrument: *const c_char,
    /// Canonical execution side.
    pub side: u32,
    /// Canonical order type.
    pub order_type: u32,
    /// Canonical time in force.
    pub time_in_force: u32,
    /// Total parent quantity.
    pub total_qty: i64,
    /// Child limit price, or zero where not applicable.
    pub limit_price: i64,
    /// Child stop price, or zero where not applicable.
    pub stop_price: i64,
    /// Parent schedule start in nanoseconds.
    pub start_ns: u64,
    /// Parent schedule end in nanoseconds.
    pub end_ns: u64,
    /// Minimum child clip.
    pub min_clip: i64,
    /// Maximum child clip.
    pub max_clip: i64,
    /// Optional participation cap in basis points.
    pub participation_cap_bps: u16,
    /// TWAP slice interval in nanoseconds.
    pub slice_interval_ns: u64,
}

/// Owned child-order plan produced by a deterministic execution algorithm.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct of_execution_algo_child_plan_t {
    /// Child algorithm identifier.
    pub child_order_id: [c_char; 41],
    /// Parent algorithm identifier.
    pub parent_order_id: [c_char; 41],
    /// Canonical OMS client order identifier.
    pub client_order_id: [c_char; 41],
    /// Trading account identifier.
    pub account_id: [c_char; 33],
    /// Execution route identifier.
    pub route_id: [c_char; 33],
    /// Strategy attribution identifier.
    pub strategy_id: [c_char; 33],
    /// Venue identifier.
    pub venue: [c_char; 17],
    /// Instrument identifier.
    pub instrument: [c_char; 33],
    /// Canonical execution side.
    pub side: u32,
    /// Canonical order type.
    pub order_type: u32,
    /// Canonical time in force.
    pub time_in_force: u32,
    /// Planned child quantity.
    pub quantity: i64,
    /// Planned child limit price.
    pub limit_price: i64,
    /// Planned child stop price.
    pub stop_price: i64,
    /// Planned release timestamp.
    pub due_ns: u64,
    /// OMS receive/create timestamp.
    pub ts_recv_ns: u64,
    /// Non-zero when a child is due; zero represents a successful no-op.
    pub has_plan: u8,
}

/// Aggregate progress snapshot for an execution algorithm.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct of_execution_algo_progress_t {
    /// Parent target quantity.
    pub target_qty: i64,
    /// Quantity committed as submitted child orders.
    pub released_qty: i64,
    /// Quantity filled by child orders.
    pub completed_qty: i64,
    /// Estimated currently open child quantity.
    pub open_qty: i64,
    /// Rejected terminal child count.
    pub rejected_children: u64,
    /// All terminal child count.
    pub terminal_children: u64,
    /// Non-zero when a planned child awaits commit/discard.
    pub has_pending_plan: u8,
}

/// Tagged signal configuration parameter used by registry-based binding calls.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct of_signal_config_parameter_t {
    /// Parameter name from the selected signal descriptor.
    pub name: *const c_char,
    /// Value kind: 1 integer, 2 floating point, 3 boolean, or 4 text.
    pub kind: u32,
    /// Integer payload when `kind` is 1.
    pub integer_value: i64,
    /// Floating-point payload when `kind` is 2.
    pub float_value: f64,
    /// Boolean payload when `kind` is 3; zero is false and one is true.
    pub boolean_value: u8,
    /// UTF-8 text payload when `kind` is 4.
    pub text_value: *const c_char,
}

/// Replay-validation policy passed to the signal validation facade.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct of_signal_validation_config_t {
    /// Number of future events used for each markout label.
    pub markout_horizon_events: u32,
    /// Absolute price change at or below which a markout is flat.
    pub flat_price_threshold: i64,
    /// Minimum directional confidence in basis points.
    pub min_confidence_bps: u16,
    /// Non-zero retains per-event samples in the returned JSON.
    pub store_samples: u8,
    /// Non-zero checks exchange timestamps for monotonic ordering.
    pub check_monotonic_timestamps: u8,
}

/// One analytics observation consumed by the signal replay validator.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct of_signal_validation_event_t {
    /// Session delta.
    pub delta: i64,
    /// Cumulative session delta.
    pub cumulative_delta: i64,
    /// Total buy-side volume.
    pub buy_volume: i64,
    /// Total sell-side volume.
    pub sell_volume: i64,
    /// Last traded price.
    pub last_price: i64,
    /// Session point of control.
    pub point_of_control: i64,
    /// Session value-area low.
    pub value_area_low: i64,
    /// Session value-area high.
    pub value_area_high: i64,
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Non-zero when `ts_exchange_ns` is present.
    pub has_ts_exchange_ns: u8,
}

/// Opaque engine handle.
pub struct of_engine {
    inner: DefaultEngine,
    subs: Vec<SubscriptionRecord>,
}

/// Opaque execution engine handle.
pub struct of_execution_engine {
    inner: ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal>,
}

/// Opaque concurrent execution engine handle.
pub struct of_execution_concurrent_engine {
    inner: ConcurrentExecutionEngine,
}

/// Opaque deterministic TWAP algorithm handle.
pub struct of_execution_twap_algo {
    parent: ParentOrder,
    progress: AlgoProgress,
    planner: TwapSlicePlanner,
    pending: Option<ChildOrderPlan>,
}

/// Opaque subscription token.
pub struct of_subscription {
    token: *mut SubscriptionToken,
}

/// Event envelope dispatched to subscription callbacks.
#[repr(C)]
pub struct of_event_t {
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
    /// Stream/event kind value.
    pub kind: u32,
    /// Pointer to UTF-8 payload bytes.
    pub payload: *const c_void,
    /// Payload byte length.
    pub payload_len: u32,
    /// Payload schema identifier.
    pub schema_id: u32,
    /// Quality flags bitset associated with this event.
    pub quality_flags: u32,
}

/// C callback signature for subscription delivery.
pub type of_event_cb = extern "C" fn(*const of_event_t, *mut c_void);

struct SubscriptionRecord {
    symbol: SymbolId,
    kind: u32,
    cb: of_event_cb,
    user_data: *mut c_void,
    active: Arc<AtomicBool>,
    last_health_seq: u64,
}

struct SubscriptionToken {
    active: Arc<AtomicBool>,
}

/// Returns ABI version (`major << 16 | minor` style encoding).
#[no_mangle]
pub extern "C" fn of_api_version() -> u32 {
    API_VERSION
}

/// Returns build/version info as a static NUL-terminated C string.
#[no_mangle]
pub extern "C" fn of_build_info() -> *const c_char {
    BUILD_INFO.as_ptr() as *const c_char
}

/// Returns execution ABI version (`major << 16 | minor` style encoding).
#[no_mangle]
pub extern "C" fn of_execution_api_version() -> u32 {
    EXECUTION_API_VERSION
}

/// Inspects an execution WAL file and writes a non-panicking integrity report.
#[no_mangle]
pub extern "C" fn of_execution_wal_integrity_report(
    path: *const c_char,
    out_report: *mut of_execution_wal_integrity_report_t,
) -> i32 {
    if path.is_null() || out_report.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let Some(path) = non_empty_string(path) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return of_error_t::OF_ERR_IO as i32,
    };
    let report = WalIntegrityReport::inspect(&bytes, true);
    unsafe {
        *out_report = wal_integrity_report_to_ffi(report);
    }
    of_error_t::OF_OK as i32
}

/// Inspects a segmented execution WAL root and writes an integrity report.
#[no_mangle]
pub extern "C" fn of_execution_segmented_wal_integrity_report(
    root: *const c_char,
    out_report: *mut of_execution_segmented_wal_integrity_report_t,
) -> i32 {
    if root.is_null() || out_report.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let Some(root) = non_empty_string(root) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let report = match SegmentedWalExecutionJournal::inspect_root(root) {
        Ok(report) => report,
        Err(_) => return of_error_t::OF_ERR_IO as i32,
    };
    unsafe {
        *out_report = segmented_wal_integrity_report_to_ffi(report);
    }
    of_error_t::OF_OK as i32
}

/// Inspects an execution checkpoint store root and writes an integrity report.
#[no_mangle]
pub extern "C" fn of_execution_checkpoint_store_integrity_report(
    root: *const c_char,
    out_report: *mut of_execution_checkpoint_store_integrity_report_t,
) -> i32 {
    if root.is_null() || out_report.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let Some(root) = non_empty_string(root) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let report = match FileExecutionCheckpointStore::inspect_root(root) {
        Ok(report) => report,
        Err(_) => return of_error_t::OF_ERR_IO as i32,
    };
    unsafe {
        *out_report = checkpoint_store_integrity_report_to_ffi(report);
    }
    of_error_t::OF_OK as i32
}

/// Recovers existing checkpoint/WAL roots read-only and allocates a bounded
/// operational JSON report.
///
/// The caller owns the returned string and must release it with
/// [`of_string_free`]. This function never creates roots, opens a WAL append
/// handle, mutates recovered state, reconciles with a venue, or enables order
/// submissions.
#[no_mangle]
pub extern "C" fn of_execution_recovery_report_json(
    config: *const of_execution_recovery_config_t,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if config.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let config = unsafe { &*config };
    if config.require_checkpoint > 1 {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let Some(wal_root) = non_empty_string(config.wal_root) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let checkpoint_root = non_empty_string(config.checkpoint_root);
    if config.require_checkpoint != 0 && checkpoint_root.is_none() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let result = match recover_latest_checkpoint_from_segmented_wal_roots(
        wal_root,
        checkpoint_root.as_deref().map(std::path::Path::new),
        config.require_checkpoint != 0,
    ) {
        Ok(result) => result,
        Err(error) => return map_execution_error(&error),
    };
    allocate_json_string(result.json_report(), out_json, out_len)
}

/// Creates a simulated execution engine and stores it in `out_engine`.
#[no_mangle]
pub extern "C" fn of_execution_engine_create(
    cfg: *const of_execution_route_config_t,
    out_engine: *mut *mut of_execution_engine,
) -> i32 {
    if cfg.is_null() || out_engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let cfg = unsafe { &*cfg };
    let route = match route_config_from_ffi(cfg) {
        Ok(route) => route,
        Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    create_execution_engine_from_routes(vec![route], out_engine)
}

/// Creates a simulated execution engine from multiple route configs.
#[no_mangle]
pub extern "C" fn of_execution_engine_create_multi(
    routes: *const of_execution_route_config_t,
    route_count: u32,
    out_engine: *mut *mut of_execution_engine,
) -> i32 {
    if routes.is_null() || route_count == 0 || out_engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let route_configs = match route_configs_from_ffi(routes, route_count) {
        Ok(routes) => routes,
        Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    create_execution_engine_from_routes(route_configs, out_engine)
}

fn create_execution_engine_from_routes(
    routes: Vec<RouteConfig>,
    out_engine: *mut *mut of_execution_engine,
) -> i32 {
    let engine = Box::new(of_execution_engine {
        inner: simulated_engine_with_routes(routes),
    });
    unsafe {
        *out_engine = Box::into_raw(engine);
    }
    of_error_t::OF_OK as i32
}

/// Creates and starts a concurrent simulated execution engine.
#[no_mangle]
pub extern "C" fn of_execution_concurrent_engine_create_multi(
    routes: *const of_execution_route_config_t,
    route_count: u32,
    config: *const of_execution_concurrent_config_t,
    out_engine: *mut *mut of_execution_concurrent_engine,
) -> i32 {
    if routes.is_null() || route_count == 0 || out_engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let route_configs = match route_configs_from_ffi(routes, route_count) {
        Ok(routes) => routes,
        Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let cfg = concurrent_config_from_ffi(config);
    let engine = simulated_engine_with_routes(route_configs);
    let inner = match ConcurrentExecutionEngine::spawn(engine, cfg) {
        Ok(engine) => engine,
        Err(err) => return map_concurrent_execution_error(&err),
    };
    let wrapped = Box::new(of_execution_concurrent_engine { inner });
    unsafe {
        *out_engine = Box::into_raw(wrapped);
    }
    of_error_t::OF_OK as i32
}

/// Destroys a concurrent execution engine.
#[no_mangle]
pub extern "C" fn of_execution_concurrent_engine_destroy(
    engine: *mut of_execution_concurrent_engine,
) {
    if engine.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(engine);
    }
}

/// Requests graceful concurrent execution worker stop.
#[no_mangle]
pub extern "C" fn of_execution_concurrent_stop(
    engine: *mut of_execution_concurrent_engine,
    out_sequence: *mut u64,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    match engine.inner.request_stop() {
        Ok(sequence) => {
            write_optional_u64(out_sequence, sequence);
            of_error_t::OF_OK as i32
        }
        Err(err) => map_concurrent_execution_error(&err),
    }
}

/// Sends a non-blocking submit command to a concurrent execution worker.
#[no_mangle]
pub extern "C" fn of_execution_concurrent_submit_order(
    engine: *mut of_execution_concurrent_engine,
    req: *const of_execution_order_request_t,
    out_sequence: *mut u64,
) -> i32 {
    if engine.is_null() || req.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let req = match order_request_from_ffi(unsafe { &*req }) {
        Ok(req) => req,
        Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let engine = unsafe { &mut *engine };
    send_concurrent_command(engine, ExecutionCommand::Submit(req), out_sequence)
}

/// Sends a non-blocking cancel command to a concurrent execution worker.
#[no_mangle]
pub extern "C" fn of_execution_concurrent_cancel_order(
    engine: *mut of_execution_concurrent_engine,
    req: *const of_execution_cancel_request_t,
    out_sequence: *mut u64,
) -> i32 {
    if engine.is_null() || req.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let req = match cancel_request_from_ffi(unsafe { &*req }) {
        Ok(req) => req,
        Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let engine = unsafe { &mut *engine };
    send_concurrent_command(engine, ExecutionCommand::Cancel(req), out_sequence)
}

/// Sends a non-blocking amend command to a concurrent execution worker.
#[no_mangle]
pub extern "C" fn of_execution_concurrent_amend_order(
    engine: *mut of_execution_concurrent_engine,
    req: *const of_execution_amend_request_t,
    out_sequence: *mut u64,
) -> i32 {
    if engine.is_null() || req.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let req = match amend_request_from_ffi(unsafe { &*req }) {
        Ok(req) => req,
        Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let engine = unsafe { &mut *engine };
    send_concurrent_command(engine, ExecutionCommand::Amend(req), out_sequence)
}

/// Sends a non-blocking poll command to a concurrent execution worker.
#[no_mangle]
pub extern "C" fn of_execution_concurrent_poll(
    engine: *mut of_execution_concurrent_engine,
    out_sequence: *mut u64,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    send_concurrent_command(engine, ExecutionCommand::Poll, out_sequence)
}

/// Attempts to receive one concurrent command report without blocking.
#[no_mangle]
pub extern "C" fn of_execution_concurrent_try_recv_report(
    engine: *mut of_execution_concurrent_engine,
    out_report: *mut of_execution_command_report_t,
    out_events: *mut of_execution_event_t,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() || out_report.is_null() || inout_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    let report = match engine.inner.try_recv_report() {
        Ok(report) => report,
        Err(err) => return map_concurrent_execution_error(&err),
    };
    write_concurrent_report(&report, out_report, out_events, inout_len)
}

/// Starts an execution engine.
#[no_mangle]
pub extern "C" fn of_execution_engine_start(engine: *mut of_execution_engine) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    map_execution_result(engine.inner.start())
}

/// Stops an execution engine.
#[no_mangle]
pub extern "C" fn of_execution_engine_stop(engine: *mut of_execution_engine) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    of_error_t::OF_OK as i32
}

/// Destroys an execution engine.
#[no_mangle]
pub extern "C" fn of_execution_engine_destroy(engine: *mut of_execution_engine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(engine);
    }
}

/// Creates a deterministic TWAP parent algorithm.
#[no_mangle]
pub extern "C" fn of_execution_twap_algo_create(
    config: *const of_execution_twap_config_t,
    out_algo: *mut *mut of_execution_twap_algo,
) -> i32 {
    if config.is_null() || out_algo.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    unsafe {
        *out_algo = std::ptr::null_mut();
    }
    let config = unsafe { &*config };
    let Ok((parent, planner)) = twap_config_from_ffi(config) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let algo = of_execution_twap_algo {
        progress: AlgoProgress::new(parent.id(), parent.total_qty()),
        parent,
        planner,
        pending: None,
    };
    unsafe {
        *out_algo = Box::into_raw(Box::new(algo));
    }
    of_error_t::OF_OK as i32
}

/// Plans the next due TWAP child without advancing parent progress.
#[no_mangle]
pub extern "C" fn of_execution_twap_algo_plan(
    algo: *mut of_execution_twap_algo,
    now_ns: u64,
    child_order_id: *const c_char,
    client_order_id: *const c_char,
    ts_recv_ns: u64,
    out_plan: *mut of_execution_algo_child_plan_t,
) -> i32 {
    if algo.is_null() || out_plan.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let Ok(child_order_id) = fixed_from_ptr::<40>(child_order_id) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let Ok(client_order_id) = fixed_from_ptr::<40>(client_order_id) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let algo = unsafe { &mut *algo };
    if let Some(pending) = algo.pending {
        if pending.child_id() != child_order_id
            || pending.request().client_order_id != client_order_id
        {
            return of_error_t::OF_ERR_STATE as i32;
        }
        unsafe {
            *out_plan = child_plan_to_ffi(Some(&pending));
        }
        return of_error_t::OF_OK as i32;
    }
    match algo.planner.plan_due_slice(
        &algo.parent,
        algo.progress,
        now_ns,
        child_order_id,
        client_order_id,
        ts_recv_ns,
    ) {
        Ok(plan) => {
            algo.pending = plan;
            unsafe {
                *out_plan = child_plan_to_ffi(algo.pending.as_ref());
            }
            of_error_t::OF_OK as i32
        }
        Err(_) => of_error_t::OF_ERR_INVALID_ARG as i32,
    }
}

/// Commits a pending child after successful OMS submission.
#[no_mangle]
pub extern "C" fn of_execution_twap_algo_commit_pending(algo: *mut of_execution_twap_algo) -> i32 {
    if algo.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let algo = unsafe { &mut *algo };
    let Some(plan) = algo.pending else {
        return of_error_t::OF_ERR_STATE as i32;
    };
    if algo.progress.on_child_released(&plan).is_err() {
        return of_error_t::OF_ERR_STATE as i32;
    }
    algo.pending = None;
    of_error_t::OF_OK as i32
}

/// Discards a pending child after failed or abandoned OMS submission.
#[no_mangle]
pub extern "C" fn of_execution_twap_algo_discard_pending(algo: *mut of_execution_twap_algo) -> i32 {
    if algo.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let algo = unsafe { &mut *algo };
    if algo.pending.take().is_none() {
        return of_error_t::OF_ERR_STATE as i32;
    }
    of_error_t::OF_OK as i32
}

/// Records child execution progress using canonical order-status values.
#[no_mangle]
pub extern "C" fn of_execution_twap_algo_record_execution(
    algo: *mut of_execution_twap_algo,
    last_qty: i64,
    leaves_qty: i64,
    order_status: u32,
) -> i32 {
    if algo.is_null() || last_qty < 0 || leaves_qty < 0 {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let Ok(order_status) = order_status_from_ffi(order_status) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let event = ExecutionEvent {
        exec_type: ExecutionType::Status,
        order_status,
        client_order_id: FixedAscii::empty(),
        orig_client_order_id: FixedAscii::empty(),
        venue_order_id: FixedAscii::empty(),
        execution_id: FixedAscii::empty(),
        account_id: FixedAscii::empty(),
        route_id: FixedAscii::empty(),
        symbol: ExecutionSymbol {
            venue: FixedAscii::empty(),
            instrument: FixedAscii::empty(),
        },
        last_qty: OrderQty(last_qty),
        last_price: OrderPrice(0),
        cumulative_qty: OrderQty(0),
        leaves_qty: OrderQty(leaves_qty),
        average_price: OrderPrice(0),
        ts_exchange_ns: 0,
        ts_recv_ns: 0,
        reason: RiskRejectReason::None,
        text: ExecutionText::empty(),
    };
    unsafe { &mut *algo }.progress.on_execution_event(&event);
    of_error_t::OF_OK as i32
}

/// Returns current aggregate parent progress.
#[no_mangle]
pub extern "C" fn of_execution_twap_algo_progress(
    algo: *const of_execution_twap_algo,
    out_progress: *mut of_execution_algo_progress_t,
) -> i32 {
    if algo.is_null() || out_progress.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let algo = unsafe { &*algo };
    unsafe {
        *out_progress = algo_progress_to_ffi(algo.progress, algo.pending.is_some());
    }
    of_error_t::OF_OK as i32
}

/// Destroys a deterministic TWAP algorithm handle.
#[no_mangle]
pub extern "C" fn of_execution_twap_algo_destroy(algo: *mut of_execution_twap_algo) {
    if !algo.is_null() {
        unsafe {
            drop(Box::from_raw(algo));
        }
    }
}

/// Submits an execution order.
#[no_mangle]
pub extern "C" fn of_execution_submit_order(
    engine: *mut of_execution_engine,
    req: *const of_execution_order_request_t,
    out_events: *mut of_execution_event_t,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() || req.is_null() || inout_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let req = match order_request_from_ffi(unsafe { &*req }) {
        Ok(req) => req,
        Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let engine = unsafe { &mut *engine };
    let mut events = ExecutionEventBuffer::with_capacity(FFI_EVENT_BUFFER_CAP);
    let rc = match engine.inner.submit(req, &mut events) {
        Ok(()) => of_error_t::OF_OK as i32,
        Err(err) => map_execution_error(&err),
    };
    let copy_rc = copy_execution_events(&events, out_events, inout_len);
    if copy_rc != of_error_t::OF_OK as i32 {
        copy_rc
    } else {
        rc
    }
}

/// Cancels an execution order.
#[no_mangle]
pub extern "C" fn of_execution_cancel_order(
    engine: *mut of_execution_engine,
    req: *const of_execution_cancel_request_t,
    out_events: *mut of_execution_event_t,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() || req.is_null() || inout_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let req = match cancel_request_from_ffi(unsafe { &*req }) {
        Ok(req) => req,
        Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let engine = unsafe { &mut *engine };
    let mut events = ExecutionEventBuffer::with_capacity(FFI_EVENT_BUFFER_CAP);
    let rc = match engine.inner.cancel(req, &mut events) {
        Ok(()) => of_error_t::OF_OK as i32,
        Err(err) => map_execution_error(&err),
    };
    let copy_rc = copy_execution_events(&events, out_events, inout_len);
    if copy_rc != of_error_t::OF_OK as i32 {
        copy_rc
    } else {
        rc
    }
}

/// Amends an execution order.
#[no_mangle]
pub extern "C" fn of_execution_amend_order(
    engine: *mut of_execution_engine,
    req: *const of_execution_amend_request_t,
    out_events: *mut of_execution_event_t,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() || req.is_null() || inout_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let req = match amend_request_from_ffi(unsafe { &*req }) {
        Ok(req) => req,
        Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let engine = unsafe { &mut *engine };
    let mut events = ExecutionEventBuffer::with_capacity(FFI_EVENT_BUFFER_CAP);
    let rc = match engine.inner.amend(req, &mut events) {
        Ok(()) => of_error_t::OF_OK as i32,
        Err(err) => map_execution_error(&err),
    };
    let copy_rc = copy_execution_events(&events, out_events, inout_len);
    if copy_rc != of_error_t::OF_OK as i32 {
        copy_rc
    } else {
        rc
    }
}

/// Polls execution events.
#[no_mangle]
pub extern "C" fn of_execution_poll(
    engine: *mut of_execution_engine,
    out_events: *mut of_execution_event_t,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() || inout_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    let mut events = ExecutionEventBuffer::with_capacity(FFI_EVENT_BUFFER_CAP);
    let rc = match engine.inner.poll(&mut events) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(err) => map_execution_error(&err),
    };
    let copy_rc = copy_execution_events(&events, out_events, inout_len);
    if copy_rc != of_error_t::OF_OK as i32 {
        copy_rc
    } else {
        rc
    }
}

/// Gets current order state for a client order id.
#[no_mangle]
pub extern "C" fn of_execution_get_order_state(
    engine: *const of_execution_engine,
    client_order_id: *const c_char,
    out_state: *mut of_execution_order_state_t,
) -> i32 {
    if engine.is_null() || client_order_id.is_null() || out_state.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let id = match fixed_from_ptr::<40>(client_order_id) {
        Ok(id) => id,
        Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let engine = unsafe { &*engine };
    let Some(state) = engine.inner.order_state(&id) else {
        return of_error_t::OF_ERR_STATE as i32;
    };
    unsafe {
        *out_state = order_state_to_ffi(&state);
    }
    of_error_t::OF_OK as i32
}

/// Gets execution health.
#[no_mangle]
pub extern "C" fn of_execution_health(
    engine: *const of_execution_engine,
    out_health: *mut of_execution_health_t,
) -> i32 {
    if engine.is_null() || out_health.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let health = unsafe { &*engine }.inner.health();
    unsafe {
        *out_health = of_execution_health_t {
            connected: u8::from(health.connected),
            degraded: u8::from(health.degraded),
            health_seq: health.health_seq,
        };
    }
    of_error_t::OF_OK as i32
}

/// Gets execution metrics.
#[no_mangle]
pub extern "C" fn of_execution_metrics(
    engine: *const of_execution_engine,
    out_metrics: *mut of_execution_metrics_t,
) -> i32 {
    if engine.is_null() || out_metrics.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let metrics = unsafe { &*engine }.inner.metrics();
    unsafe {
        *out_metrics = of_execution_metrics_t {
            submitted: metrics.submitted,
            cancelled: metrics.cancelled,
            amended: metrics.amended,
            events_applied: metrics.events_applied,
            risk_rejected: metrics.risk_rejected,
            adapter_errors: metrics.adapter_errors,
            recovered: metrics.recovered,
        };
    }
    of_error_t::OF_OK as i32
}

/// Creates a runtime engine and stores it in `out_engine`.
#[no_mangle]
pub extern "C" fn of_engine_create(
    cfg: *const of_engine_config_t,
    out_engine: *mut *mut of_engine,
) -> i32 {
    if cfg.is_null() || out_engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let cfg_ref = unsafe { &*cfg };
    let mut runtime_cfg = if let Some(path) = non_empty_string(cfg_ref.config_path) {
        match load_engine_config_from_path(&path) {
            Ok(v) => v,
            Err(_) => return of_error_t::OF_ERR_INVALID_ARG as i32,
        }
    } else {
        EngineConfig {
            instance_id: "default".to_string(),
            enable_persistence: false,
            data_root: "data".to_string(),
            audit_log_path: "audit/orderflow_audit.log".to_string(),
            audit_max_bytes: 10 * 1024 * 1024,
            audit_max_files: 5,
            audit_redact_tokens: vec![
                "secret".to_string(),
                "password".to_string(),
                "token".to_string(),
                "api_key".to_string(),
            ],
            data_retention_max_bytes: 10 * 1024 * 1024,
            data_retention_max_age_secs: 7 * 24 * 60 * 60,
            adapter: AdapterConfig {
                provider: ProviderKind::Mock,
                ..AdapterConfig::default()
            },
            signal_threshold: 100,
        }
    };

    if let Some(instance_id) = non_empty_string(cfg_ref.instance_id) {
        runtime_cfg.instance_id = instance_id;
    }
    runtime_cfg.enable_persistence = cfg_ref.enable_persistence != 0;
    if cfg_ref.audit_max_bytes > 0 {
        runtime_cfg.audit_max_bytes = cfg_ref.audit_max_bytes;
    }
    if cfg_ref.audit_max_files > 0 {
        runtime_cfg.audit_max_files = cfg_ref.audit_max_files;
    }
    if let Some(tokens) = parse_csv(cfg_ref.audit_redact_tokens_csv) {
        runtime_cfg.audit_redact_tokens = tokens;
    }
    if cfg_ref.data_retention_max_bytes > 0 {
        runtime_cfg.data_retention_max_bytes = cfg_ref.data_retention_max_bytes;
    }
    if cfg_ref.data_retention_max_age_secs > 0 {
        runtime_cfg.data_retention_max_age_secs = cfg_ref.data_retention_max_age_secs;
    }

    let engine = match build_default_engine(runtime_cfg) {
        Ok(v) => v,
        Err(_) => return of_error_t::OF_ERR_STATE as i32,
    };

    let wrapped = Box::new(of_engine {
        inner: engine,
        subs: Vec::new(),
    });
    unsafe {
        *out_engine = Box::into_raw(wrapped);
    }
    of_error_t::OF_OK as i32
}

/// Starts adapter polling/session for a created engine.
#[no_mangle]
pub extern "C" fn of_engine_start(engine: *mut of_engine) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let engine = unsafe { &mut *engine };
    match engine.inner.start() {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(_) => of_error_t::OF_ERR_STATE as i32,
    }
}

/// Stops adapter polling/session for an engine.
#[no_mangle]
pub extern "C" fn of_engine_stop(engine: *mut of_engine) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    engine.inner.stop();
    of_error_t::OF_OK as i32
}

/// Destroys an engine created by [`of_engine_create`].
#[no_mangle]
pub extern "C" fn of_engine_destroy(engine: *mut of_engine) {
    if !engine.is_null() {
        unsafe {
            drop(Box::from_raw(engine));
        }
    }
}

/// Subscribes to a symbol stream and returns a subscription token.
#[no_mangle]
pub extern "C" fn of_subscribe(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    _kind: u32,
    cb: Option<of_event_cb>,
    user_data: *mut c_void,
    out_sub: *mut *mut of_subscription,
) -> i32 {
    if engine.is_null() || symbol.is_null() || out_sub.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, depth_levels) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    if engine
        .inner
        .subscribe(symbol.clone(), depth_levels)
        .is_err()
    {
        return of_error_t::OF_ERR_STATE as i32;
    }

    let active = Arc::new(AtomicBool::new(true));
    if let Some(cb_fn) = cb {
        engine.subs.push(SubscriptionRecord {
            symbol: symbol.clone(),
            kind: _kind,
            cb: cb_fn,
            user_data,
            active: active.clone(),
            last_health_seq: 0,
        });
    }

    let token = Box::new(SubscriptionToken { active });
    let sub = Box::new(of_subscription {
        token: Box::into_raw(token),
    });
    unsafe {
        *out_sub = Box::into_raw(sub);
    }
    of_error_t::OF_OK as i32
}

/// Unsubscribes and destroys a subscription token.
#[no_mangle]
pub extern "C" fn of_unsubscribe(sub: *mut of_subscription) -> i32 {
    if sub.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    unsafe {
        let sub = Box::from_raw(sub);
        if !sub.token.is_null() {
            let token = Box::from_raw(sub.token);
            token.active.store(false, Ordering::Release);
        }
    }
    of_error_t::OF_OK as i32
}

/// Unsubscribes all active streams for a symbol on this engine.
#[no_mangle]
pub extern "C" fn of_unsubscribe_symbol(engine: *mut of_engine, symbol: *const of_symbol_t) -> i32 {
    if engine.is_null() || symbol.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    if engine.inner.unsubscribe(symbol.clone()).is_err() {
        return of_error_t::OF_ERR_STATE as i32;
    }

    for sub in &mut engine.subs {
        if sub.symbol == symbol {
            sub.active.store(false, Ordering::Release);
        }
    }
    engine.subs.retain(|s| s.active.load(Ordering::Acquire));
    of_error_t::OF_OK as i32
}

/// Resets per-symbol analytics session state.
#[no_mangle]
pub extern "C" fn of_reset_symbol_session(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
) -> i32 {
    if engine.is_null() || symbol.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    if engine.inner.reset_symbol_session(symbol).is_err() {
        return of_error_t::OF_ERR_STATE as i32;
    }
    of_error_t::OF_OK as i32
}

/// Injects one external trade event into runtime processing.
#[no_mangle]
pub extern "C" fn of_ingest_trade(
    engine: *mut of_engine,
    trade: *const of_trade_t,
    quality_flags: u32,
) -> i32 {
    if engine.is_null() || trade.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let trade = unsafe { &*trade };
    let (symbol, _) = match symbol_from_ffi_ref(&trade.symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let aggressor_side = match side_from_ffi(trade.aggressor_side) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let q = DataQualityFlags::from_bits_truncate(quality_flags);
    let event = TradePrint {
        symbol,
        price: trade.price,
        size: trade.size,
        aggressor_side,
        sequence: trade.sequence,
        ts_exchange_ns: trade.ts_exchange_ns,
        ts_recv_ns: trade.ts_recv_ns,
    };

    let engine = unsafe { &mut *engine };
    match engine.inner.ingest_trade(event, q) {
        Ok(_) => {
            dispatch_callbacks(engine, engine.inner.current_quality_flags_bits());
            of_error_t::OF_OK as i32
        }
        Err(_) => of_error_t::OF_ERR_STATE as i32,
    }
}

/// Injects one external book event into runtime processing.
#[no_mangle]
pub extern "C" fn of_ingest_book(
    engine: *mut of_engine,
    book: *const of_book_t,
    quality_flags: u32,
) -> i32 {
    if engine.is_null() || book.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let book = unsafe { &*book };
    let (symbol, _) = match symbol_from_ffi_ref(&book.symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let side = match side_from_ffi(book.side) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let action = match action_from_ffi(book.action) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let q = DataQualityFlags::from_bits_truncate(quality_flags);
    let event = BookUpdate {
        symbol,
        side,
        level: book.level,
        price: book.price,
        size: book.size,
        action,
        sequence: book.sequence,
        ts_exchange_ns: book.ts_exchange_ns,
        ts_recv_ns: book.ts_recv_ns,
    };

    let engine = unsafe { &mut *engine };
    match engine.inner.ingest_book(event, q) {
        Ok(_) => {
            dispatch_callbacks(engine, engine.inner.current_quality_flags_bits());
            of_error_t::OF_OK as i32
        }
        Err(_) => of_error_t::OF_ERR_STATE as i32,
    }
}

/// Configures stale/sequence policy for external ingest mode.
#[no_mangle]
pub extern "C" fn of_configure_external_feed(
    engine: *mut of_engine,
    policy: *const of_external_feed_policy_t,
) -> i32 {
    if engine.is_null() || policy.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    let policy = unsafe { &*policy };
    match engine.inner.configure_external_feed(ExternalFeedPolicy {
        stale_after_ms: policy.stale_after_ms,
        enforce_sequence: policy.enforce_sequence != 0,
    }) {
        Ok(_) => {
            dispatch_health_callbacks(engine, engine.inner.current_quality_flags_bits());
            of_error_t::OF_OK as i32
        }
        Err(_) => of_error_t::OF_ERR_STATE as i32,
    }
}

/// Marks external feed reconnecting state.
#[no_mangle]
pub extern "C" fn of_external_set_reconnecting(engine: *mut of_engine, reconnecting: u8) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    match engine.inner.set_external_reconnecting(reconnecting != 0) {
        Ok(_) => {
            dispatch_health_callbacks(engine, engine.inner.current_quality_flags_bits());
            of_error_t::OF_OK as i32
        }
        Err(_) => of_error_t::OF_ERR_STATE as i32,
    }
}

/// Re-evaluates external feed health without ingesting new events.
#[no_mangle]
pub extern "C" fn of_external_health_tick(engine: *mut of_engine) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    match engine.inner.external_health_tick() {
        Ok(_) => {
            dispatch_health_callbacks(engine, engine.inner.current_quality_flags_bits());
            of_error_t::OF_OK as i32
        }
        Err(_) => of_error_t::OF_ERR_STATE as i32,
    }
}

/// Writes current book snapshot JSON into caller buffer.
#[no_mangle]
pub extern "C" fn of_get_book_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let payload = match engine.inner.book_snapshot(&symbol) {
        Some(snapshot) => format_book_snapshot(&snapshot),
        None => "{}".to_string(),
    };
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes current book analytics snapshot JSON into caller buffer.
///
/// Payload shape:
/// ```json
/// {"best_bid":...,"best_ask":...,"quoted_spread":...,"relative_spread_bps":...,
///  "microprice":...,"bid_depth":...,"ask_depth":...,"depth_imbalance_bps":...}
/// ```
#[no_mangle]
pub extern "C" fn of_get_book_analytics_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let payload = match engine.inner.book_analytics_snapshot(&symbol) {
        Some(snap) => format_book_analytics_snapshot(&snap),
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Computes weighted average price for an order of `qty` and writes JSON result.
///
/// Payload: `{"price": N}` on success, `{}` if insufficient liquidity.
/// Positive qty = buy (walks asks), negative qty = sell (walks bids).
#[no_mangle]
pub extern "C" fn of_compute_weighted_average_price(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    qty: i64,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let payload = match engine.inner.weighted_average_price(&symbol, qty) {
        Some(price) => format!("{{\"price\":{}}}", price),
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Computes depth slope for the first `levels` price levels and writes JSON result.
///
/// Payload: `{"slope": N.N}`. Returns `{"slope":0.0}` if book has fewer than 2 levels.
#[no_mangle]
pub extern "C" fn of_compute_depth_slope(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    levels: u32,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let slope = engine.inner.depth_slope(&symbol, levels as usize);
    let payload = format!("{{\"slope\":{:.4}}}", slope);

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes mid price as JSON: `{"mid": N}`, or `{}` if no book data.
#[no_mangle]
pub extern "C" fn of_get_mid_price(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let payload = match engine.inner.mid_price(&symbol) {
        Some(mid) => format!("{{\"mid\":{}}}", mid),
        None => "{}".to_string(),
    };
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes last effective spread in bps as JSON: `{"bps": N}`, or `{}`.
#[no_mangle]
pub extern "C" fn of_get_effective_spread_bps(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let bps = engine.inner.effective_spread_bps(&symbol);
    let payload = format!("{{\"bps\":{}}}", bps);
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes average half-spread cost over `window` trades: `{"bps": N}`.
#[no_mangle]
pub extern "C" fn of_get_half_spread_cost_bps(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    window: u32,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let bps = engine.inner.half_spread_cost_bps(&symbol, window as usize);
    let payload = format!("{{\"bps\":{}}}", bps);
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes realised spread over `hold_ticks` ticks ago: `{"bps": N}`.
#[no_mangle]
pub extern "C" fn of_get_realised_spread_bps(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    hold_ticks: u32,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let bps = engine
        .inner
        .realised_spread_bps(&symbol, hold_ticks as usize);
    let payload = format!("{{\"bps\":{}}}", bps);
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes book-event analytics snapshot JSON over `window_ns`.
#[no_mangle]
pub extern "C" fn of_get_book_event_analytics(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    window_ns: u64,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let snap = engine.inner.book_event_analytics(&symbol, window_ns);
    let payload = format_book_event_analytics_snapshot(&snap);
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes resiliency snapshot JSON.
#[no_mangle]
pub extern "C" fn of_get_resiliency_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let snap = engine.inner.resiliency_snapshot(&symbol);
    let payload = format_resiliency_snapshot(&snap);
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes VPIN snapshot JSON.
#[no_mangle]
pub extern "C" fn of_get_vpin_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let payload = format_vpin_snapshot(&engine.inner.vpin_snapshot(&symbol));
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes Kyle's Lambda snapshot JSON.
#[no_mangle]
pub extern "C" fn of_get_kyle_lambda_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let payload = format_kyle_lambda_snapshot(&engine.inner.kyle_lambda_snapshot(&symbol));
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes Amihud illiquidity snapshot JSON.
#[no_mangle]
pub extern "C" fn of_get_amihud_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let payload = format_amihud_snapshot(&engine.inner.amihud_snapshot(&symbol));
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes CVD enhancement snapshot JSON.
#[no_mangle]
pub extern "C" fn of_get_cvd_enhancement_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let payload = format_cvd_enhancement_snapshot(&engine.inner.cvd_enhancement_snapshot(&symbol));
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes pattern detection snapshot JSON into caller buffer.
#[no_mangle]
pub extern "C" fn of_get_pattern_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &mut *engine };
    let payload = format_pattern_snapshot(&engine.inner.pattern_snapshot(&symbol));
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

macro_rules! snapshot_c_abi {
    ($name:ident, $format:ident, $method:ident) => {
        /// Writes an analytics snapshot JSON payload into the caller-provided buffer.
        #[no_mangle]
        pub extern "C" fn $name(
            engine: *mut of_engine,
            symbol: *const of_symbol_t,
            out_buf: *mut c_void,
            inout_len: *mut u32,
        ) -> i32 {
            if engine.is_null() {
                return of_error_t::OF_ERR_INVALID_ARG as i32;
            }
            let (symbol, _) = match symbol_from_ffi(symbol) {
                Ok(v) => v,
                Err(e) => return e as i32,
            };
            let engine = unsafe { &mut *engine };
            let payload = $format(&engine.inner.$method(&symbol));
            match write_json_to_c_buffer(&payload, out_buf, inout_len) {
                Ok(_) => of_error_t::OF_OK as i32,
                Err(e) => e as i32,
            }
        }
    };
}

snapshot_c_abi!(
    of_get_volatility_snapshot,
    format_volatility_snapshot,
    volatility_snapshot
);
snapshot_c_abi!(of_get_noise_snapshot, format_noise_snapshot, noise_snapshot);
snapshot_c_abi!(
    of_get_hasbrouck_snapshot,
    format_hasbrouck_snapshot,
    hasbrouck_snapshot
);
snapshot_c_abi!(
    of_get_almgren_chriss_snapshot,
    format_almgren_chriss_snapshot,
    almgren_chriss_snapshot
);
snapshot_c_abi!(
    of_get_spread_decomp_snapshot,
    format_spread_decomp_snapshot,
    spread_decomp_snapshot
);
snapshot_c_abi!(of_get_acd_snapshot, format_acd_snapshot, acd_snapshot);
snapshot_c_abi!(
    of_get_regime_snapshot,
    format_regime_snapshot,
    regime_snapshot
);
snapshot_c_abi!(
    of_get_kinetic_energy_snapshot,
    format_kinetic_energy_snapshot,
    kinetic_energy_snapshot
);
snapshot_c_abi!(
    of_get_dark_pool_snapshot,
    format_dark_pool_snapshot,
    dark_pool_snapshot
);
snapshot_c_abi!(
    of_get_options_flow_snapshot,
    format_options_flow_snapshot,
    options_flow_snapshot
);
snapshot_c_abi!(
    of_get_futures_snapshot,
    format_futures_snapshot,
    futures_snapshot
);
snapshot_c_abi!(
    of_get_vol_signature_snapshot,
    format_vol_signature_snapshot,
    vol_signature_snapshot
);
snapshot_c_abi!(
    of_get_agent_type_snapshot,
    format_agent_type_snapshot,
    agent_type_snapshot
);
snapshot_c_abi!(
    of_get_dark_lit_correlation_snapshot,
    format_dark_lit_correlation_snapshot,
    dark_lit_correlation_snapshot
);
snapshot_c_abi!(
    of_get_institutional_flow_snapshot,
    format_institutional_flow_snapshot,
    institutional_flow_snapshot
);
snapshot_c_abi!(
    of_get_oi_analysis_snapshot,
    format_oi_analysis_snapshot,
    oi_analysis_snapshot
);

/// Computes LOB feature snapshot from engine book state and caller-provided flow metrics.
#[no_mangle]
pub extern "C" fn of_compute_lob_features(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    trade_imbalance: f64,
    cancel_rate: f64,
    arrival_rate: f64,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };
    let engine = unsafe { &*engine };
    let payload = format_lob_feature_snapshot(&engine.inner.lob_features(
        &symbol,
        trade_imbalance,
        cancel_rate,
        arrival_rate,
    ));
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes current analytics snapshot JSON into caller buffer.
#[no_mangle]
pub extern "C" fn of_get_analytics_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let payload = match engine.inner.analytics_snapshot(&symbol) {
        Some(snap) => format_analytics_snapshot(&snap),
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes current derived analytics snapshot JSON into caller buffer.
#[no_mangle]
pub extern "C" fn of_get_derived_analytics_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let payload = match engine.inner.derived_analytics_snapshot(&symbol) {
        Some(snap) => format_derived_analytics_snapshot(&snap),
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes current session candle snapshot JSON into caller buffer.
#[no_mangle]
pub extern "C" fn of_get_session_candle_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let payload = match engine.inner.session_candle_snapshot(&symbol) {
        Some(snap) => format_session_candle_snapshot(&snap),
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes rolling interval candle snapshot JSON into caller buffer.
#[no_mangle]
pub extern "C" fn of_get_interval_candle_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    window_ns: u64,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let payload = match engine.inner.interval_candle_snapshot(&symbol, window_ns) {
        Some(snap) => format_interval_candle_snapshot(&snap),
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Sets the tickbar aggregation interval for new per-symbol accumulators.
///
/// A positive `interval_ns` enables tickbar aggregation at the given interval for
/// symbols whose accumulators are created after this call. Zero or negative values
/// disable tickbar aggregation for future accumulators. Existing accumulators
/// are not affected.
///
/// Requires the `tickbar` feature to be enabled at build time.
#[cfg(feature = "tickbar")]
#[no_mangle]
pub extern "C" fn of_engine_set_tickbar_interval(engine: *mut of_engine, interval_ns: i64) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    if interval_ns > 0 {
        engine.inner.set_tickbar_interval(Some(interval_ns));
    } else {
        engine.inner.set_tickbar_interval(None);
    }
    of_error_t::OF_OK as i32
}

/// Reports unsupported tickbar configuration when the native library is built without `tickbar`.
#[cfg(not(feature = "tickbar"))]
#[no_mangle]
pub extern "C" fn of_engine_set_tickbar_interval(engine: *mut of_engine, _interval_ns: i64) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    of_error_t::OF_ERR_STATE as i32
}

/// Writes completed bar series JSON array into caller buffer.
///
/// Requires the `tickbar` feature to be enabled at build time.
/// Returns `OF_ERR_STATE` when tickbar aggregation is not configured for the symbol.
#[cfg(feature = "tickbar")]
#[no_mangle]
pub extern "C" fn of_get_bar_series(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let payload = match engine.inner.bar_series(&symbol) {
        Some(bars) => format_bar_series(&bars),
        None => "[]".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Reports unsupported tickbar bar retrieval when the native library is built without `tickbar`.
#[cfg(not(feature = "tickbar"))]
#[no_mangle]
pub extern "C" fn of_get_bar_series(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() || symbol.is_null() || out_buf.is_null() || inout_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    of_error_t::OF_ERR_STATE as i32
}

/// Writes current signal snapshot JSON into caller buffer.
#[no_mangle]
pub extern "C" fn of_get_signal_snapshot(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let payload = match engine.inner.signal_snapshot(&symbol) {
        Some(snap) => {
            let state = match snap.state {
                SignalState::Neutral => "neutral",
                SignalState::LongBias => "long_bias",
                SignalState::ShortBias => "short_bias",
                SignalState::Blocked => "blocked",
            };
            format!(
                "{{\"module\":\"{}\",\"state\":\"{}\",\"confidence_bps\":{},\"quality_flags\":{},\"reason\":\"{}\"}}",
                escape_json(snap.module_id),
                state,
                snap.confidence_bps,
                snap.quality_flags,
                escape_json(&snap.reason)
            )
        }
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Allocates and returns metrics JSON (`*out`) plus byte length (`*out_len`).
#[no_mangle]
pub extern "C" fn of_get_metrics_json(
    engine: *mut of_engine,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if engine.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let engine = unsafe { &mut *engine };
    let metrics = engine.inner.metrics_json();
    allocate_json_string(metrics, out_json, out_len)
}

/// Allocates and returns adapter inventory JSON.
#[no_mangle]
pub extern "C" fn of_get_adapter_inventory_json(
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let inventory = runtime_adapter_inventory_json();
    allocate_json_string(inventory, out_json, out_len)
}

/// Allocates and returns active adapter status JSON for `engine`.
#[no_mangle]
pub extern "C" fn of_get_active_adapter_status_json(
    engine: *mut of_engine,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if engine.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let engine = unsafe { &mut *engine };
    let status = engine.inner.active_adapter_status_json();
    allocate_json_string(status, out_json, out_len)
}

/// Allocates and returns built-in signal descriptor inventory JSON.
#[no_mangle]
pub extern "C" fn of_get_signal_descriptors_json(
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    allocate_json_string(signal_descriptor_inventory_json(), out_json, out_len)
}

/// Allocates and returns latest signal explanation JSON for `symbol`.
#[no_mangle]
pub extern "C" fn of_get_signal_explanation_json(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if engine.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let explanation = engine
        .inner
        .signal_explanation_json(&symbol)
        .unwrap_or_else(|| "{}".to_string());
    allocate_json_string(explanation, out_json, out_len)
}

/// Allocates and returns signal metrics JSON for `engine`.
#[no_mangle]
pub extern "C" fn of_get_signal_metrics_json(
    engine: *mut of_engine,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if engine.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let engine = unsafe { &mut *engine };
    allocate_json_string(engine.inner.signal_metrics_json(), out_json, out_len)
}

/// Validates a built-in signal configuration and returns a JSON result.
///
/// A syntactically valid call returns `OF_OK` even when the configuration is
/// rejected; inspect the returned document's `valid` and `error` fields.
#[no_mangle]
pub extern "C" fn of_validate_signal_config_json(
    signal_id: *const c_char,
    parameters: *const of_signal_config_parameter_t,
    parameter_count: u32,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let Some(signal_id) = cstr_to_string(signal_id) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let owned_parameters = match signal_parameters_from_ffi(parameters, parameter_count) {
        Ok(parameters) => parameters,
        Err(()) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let borrowed_parameters = borrow_signal_parameters(&owned_parameters);
    let config = SignalConfig::with_parameters(&signal_id, &borrowed_parameters);
    allocate_json_string(
        SignalRegistry::with_built_ins().validate_config_json(&config),
        out_json,
        out_len,
    )
}

/// Constructs a built-in signal and validates it over ordered analytics events.
///
/// The returned library-owned JSON includes configuration, summary metrics,
/// optional retained samples, and structured timestamp/markout warnings. Free
/// it with [`of_string_free`]. Registry construction failures are represented
/// as `valid: false` JSON documents and still return `OF_OK`.
#[no_mangle]
pub extern "C" fn of_validate_signal_replay_json(
    signal_id: *const c_char,
    parameters: *const of_signal_config_parameter_t,
    parameter_count: u32,
    events: *const of_signal_validation_event_t,
    event_count: u32,
    validation_config: *const of_signal_validation_config_t,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if validation_config.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let Some(signal_id) = cstr_to_string(signal_id) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let owned_parameters = match signal_parameters_from_ffi(parameters, parameter_count) {
        Ok(parameters) => parameters,
        Err(()) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let ffi_events = match ffi_slice(events, event_count) {
        Ok(events) => events,
        Err(()) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let borrowed_parameters = borrow_signal_parameters(&owned_parameters);
    let signal_config = SignalConfig::with_parameters(&signal_id, &borrowed_parameters);
    let registry = SignalRegistry::with_built_ins();
    if let Err(error) = registry.validate_config(&signal_config) {
        return allocate_json_string(
            signal_registry_error_json(&signal_id, &error.to_string()),
            out_json,
            out_len,
        );
    }
    let mut signal = match registry.create_signal(&signal_config) {
        Ok(signal) => signal,
        Err(error) => {
            return allocate_json_string(
                signal_registry_error_json(&signal_id, &error.to_string()),
                out_json,
                out_len,
            );
        }
    };

    let analytics = ffi_events
        .iter()
        .map(|event| AnalyticsSnapshot {
            delta: event.delta,
            cumulative_delta: event.cumulative_delta,
            buy_volume: event.buy_volume,
            sell_volume: event.sell_volume,
            last_price: event.last_price,
            point_of_control: event.point_of_control,
            value_area_low: event.value_area_low,
            value_area_high: event.value_area_high,
        })
        .collect::<Vec<_>>();
    let replay_events = analytics
        .iter()
        .zip(ffi_events)
        .map(|(analytics, event)| {
            if event.has_ts_exchange_ns == 0 {
                SignalReplayEvent::new(analytics)
            } else {
                SignalReplayEvent::with_ts_exchange_ns(analytics, event.ts_exchange_ns)
            }
        })
        .collect::<Vec<_>>();
    let validation_config = unsafe { *validation_config };
    let report = validate_signal_replay_events(
        &mut signal,
        &replay_events,
        SignalValidationConfig::new(validation_config.markout_horizon_events as usize)
            .with_flat_price_threshold(validation_config.flat_price_threshold)
            .with_min_confidence_bps(validation_config.min_confidence_bps)
            .with_store_samples(validation_config.store_samples != 0)
            .with_check_monotonic_timestamps(validation_config.check_monotonic_timestamps != 0),
    );
    allocate_json_string(report.json_report(), out_json, out_len)
}

#[derive(Debug)]
enum OwnedSignalConfigValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Text(String),
}

#[derive(Debug)]
struct OwnedSignalConfigParameter {
    name: String,
    value: OwnedSignalConfigValue,
}

fn signal_parameters_from_ffi(
    parameters: *const of_signal_config_parameter_t,
    parameter_count: u32,
) -> Result<Vec<OwnedSignalConfigParameter>, ()> {
    let parameters = ffi_slice(parameters, parameter_count)?;
    parameters
        .iter()
        .map(|parameter| {
            let name = cstr_to_string(parameter.name).ok_or(())?;
            let value = match parameter.kind {
                1 => OwnedSignalConfigValue::Integer(parameter.integer_value),
                2 if parameter.float_value.is_finite() => {
                    OwnedSignalConfigValue::Float(parameter.float_value)
                }
                3 if parameter.boolean_value <= 1 => {
                    OwnedSignalConfigValue::Boolean(parameter.boolean_value != 0)
                }
                4 => OwnedSignalConfigValue::Text(cstr_to_string(parameter.text_value).ok_or(())?),
                _ => return Err(()),
            };
            Ok(OwnedSignalConfigParameter { name, value })
        })
        .collect()
}

fn borrow_signal_parameters(
    parameters: &[OwnedSignalConfigParameter],
) -> Vec<SignalConfigParameter<'_>> {
    parameters
        .iter()
        .map(|parameter| {
            let value = match &parameter.value {
                OwnedSignalConfigValue::Integer(value) => SignalConfigValue::Integer(*value),
                OwnedSignalConfigValue::Float(value) => SignalConfigValue::Float(*value),
                OwnedSignalConfigValue::Boolean(value) => SignalConfigValue::Boolean(*value),
                OwnedSignalConfigValue::Text(value) => SignalConfigValue::Text(value),
            };
            SignalConfigParameter::new(&parameter.name, value)
        })
        .collect()
}

fn ffi_slice<'a, T>(ptr: *const T, len: u32) -> Result<&'a [T], ()> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(());
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr, len as usize) })
}

fn signal_registry_error_json(signal_id: &str, error: &str) -> String {
    format!(
        "{{\"schema_version\":1,\"signal_id\":\"{}\",\"valid\":false,\"error\":\"{}\"}}",
        escape_json(signal_id),
        escape_json(error)
    )
}

fn allocate_json_string(payload: String, out_json: *mut *const c_char, out_len: *mut u32) -> i32 {
    let c = match CString::new(payload) {
        Ok(c) => c,
        Err(_) => return of_error_t::OF_ERR_INTERNAL as i32,
    };

    let len = c.as_bytes().len() as u32;
    let ptr = c.into_raw();
    unsafe {
        *out_json = ptr;
        *out_len = len;
    }
    of_error_t::OF_OK as i32
}

/// Frees a C string returned by this library.
#[no_mangle]
pub extern "C" fn of_string_free(p: *const c_char) {
    if p.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(p as *mut c_char);
    }
}

/// Polls adapter once and dispatches subscription callbacks.
#[no_mangle]
pub extern "C" fn of_engine_poll_once(engine: *mut of_engine, quality_flags: u32) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    let q = DataQualityFlags::from_bits_truncate(quality_flags);
    match engine.inner.poll_once(q) {
        Ok(_) => {
            dispatch_callbacks(engine, engine.inner.current_quality_flags_bits());
            of_error_t::OF_OK as i32
        }
        Err(err) => {
            let status = map_runtime_error(&err);
            if err.is_backpressure() {
                dispatch_callbacks(engine, engine.inner.current_quality_flags_bits());
            }
            status
        }
    }
}

/// Override analytics thresholds and buffer sizes at runtime.
/// Pass a pointer to a populated analytics config. Passing NULL resets to defaults.
#[no_mangle]
pub extern "C" fn of_engine_set_analytics_config(
    engine: *mut of_engine,
    config: *const of_analytics_config_t,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    if config.is_null() {
        engine
            .inner
            .set_analytics_config(AnalyticsConfig::default());
    } else {
        let cfg = unsafe { *config };
        engine.inner.set_analytics_config(cfg.into());
    }
    of_error_t::OF_OK as i32
}

fn map_runtime_error(err: &RuntimeError) -> i32 {
    if err.is_backpressure() {
        of_error_t::OF_ERR_BACKPRESSURE as i32
    } else {
        of_error_t::OF_ERR_STATE as i32
    }
}

fn map_execution_result(result: Result<(), ExecutionError>) -> i32 {
    match result {
        Ok(()) => of_error_t::OF_OK as i32,
        Err(err) => map_execution_error(&err),
    }
}

fn map_execution_error(err: &ExecutionError) -> i32 {
    match err {
        ExecutionError::RiskRejected(_) => of_error_t::OF_ERR_RISK as i32,
        ExecutionError::BufferFull => of_error_t::OF_ERR_BACKPRESSURE as i32,
        ExecutionError::Disconnected | ExecutionError::RouteNotFound => {
            of_error_t::OF_ERR_STATE as i32
        }
        ExecutionError::Core(_) => of_error_t::OF_ERR_INVALID_ARG as i32,
        ExecutionError::Adapter(_) | ExecutionError::Journal(_) => {
            of_error_t::OF_ERR_INTERNAL as i32
        }
    }
}

fn map_concurrent_execution_error(err: &ConcurrentExecutionError) -> i32 {
    match err {
        ConcurrentExecutionError::Backpressure => of_error_t::OF_ERR_BACKPRESSURE as i32,
        ConcurrentExecutionError::Stopped | ConcurrentExecutionError::WorkerPanic => {
            of_error_t::OF_ERR_STATE as i32
        }
        ConcurrentExecutionError::Execution(err) => map_execution_error(err),
    }
}

fn route_configs_from_ffi(
    routes: *const of_execution_route_config_t,
    route_count: u32,
) -> Result<Vec<RouteConfig>, ()> {
    let routes = unsafe { std::slice::from_raw_parts(routes, route_count as usize) };
    let mut route_configs = Vec::with_capacity(routes.len());
    for route in routes {
        route_configs.push(route_config_from_ffi(route)?);
    }
    Ok(route_configs)
}

fn concurrent_config_from_ffi(
    config: *const of_execution_concurrent_config_t,
) -> ConcurrentExecutionConfig {
    if config.is_null() {
        return ConcurrentExecutionConfig::default();
    }
    let config = unsafe { *config };
    ConcurrentExecutionConfig {
        command_capacity: nonzero_usize(config.command_capacity, 1024),
        report_capacity: nonzero_usize(config.report_capacity, 1024),
        event_buffer_capacity: nonzero_usize(config.event_buffer_capacity, FFI_EVENT_BUFFER_CAP),
    }
}

fn nonzero_usize(value: u32, default_value: usize) -> usize {
    if value == 0 {
        default_value
    } else {
        value as usize
    }
}

fn send_concurrent_command(
    engine: &mut of_execution_concurrent_engine,
    command: ExecutionCommand,
    out_sequence: *mut u64,
) -> i32 {
    match engine.inner.try_send(command) {
        Ok(sequence) => {
            write_optional_u64(out_sequence, sequence);
            of_error_t::OF_OK as i32
        }
        Err(err) => map_concurrent_execution_error(&err),
    }
}

fn write_concurrent_report(
    report: &ExecutionCommandReport,
    out_report: *mut of_execution_command_report_t,
    out_events: *mut of_execution_event_t,
    inout_len: *mut u32,
) -> i32 {
    let copy_rc = copy_execution_events(&report.events, out_events, inout_len);
    let event_count = unsafe { *inout_len };
    unsafe {
        *out_report = of_execution_command_report_t {
            sequence: report.sequence,
            kind: execution_command_kind_to_u32(report.kind),
            result_code: match &report.result {
                Ok(_) => of_error_t::OF_OK as i32,
                Err(err) => map_execution_error(err),
            },
            event_count,
        };
    }
    copy_rc
}

fn execution_command_kind_to_u32(kind: ExecutionCommandKind) -> u32 {
    match kind {
        ExecutionCommandKind::Submit => 1,
        ExecutionCommandKind::Cancel => 2,
        ExecutionCommandKind::Amend => 3,
        ExecutionCommandKind::Poll => 4,
        ExecutionCommandKind::RecoverOpenOrders => 5,
        ExecutionCommandKind::Stop => 6,
    }
}

fn write_optional_u64(ptr: *mut u64, value: u64) {
    if !ptr.is_null() {
        unsafe {
            *ptr = value;
        }
    }
}

fn wal_integrity_report_to_ffi(report: WalIntegrityReport) -> of_execution_wal_integrity_report_t {
    of_execution_wal_integrity_report_t {
        records: report.records,
        bytes: report.bytes,
        first_sequence: report.first_sequence.map_or(0, |sequence| sequence.0),
        last_sequence: report.last_sequence.map_or(0, |sequence| sequence.0),
        checksum_failures: report.checksum_failures,
        sequence_failures: report.sequence_failures,
        has_first_sequence: u8::from(report.first_sequence.is_some()),
        has_last_sequence: u8::from(report.last_sequence.is_some()),
        truncated_tail: u8::from(report.truncated_tail),
        valid: u8::from(report.valid),
    }
}

fn segmented_wal_integrity_report_to_ffi(
    report: WalSegmentIntegrityReport,
) -> of_execution_segmented_wal_integrity_report_t {
    of_execution_segmented_wal_integrity_report_t {
        segments: report.segments as u64,
        records: report.records,
        bytes: report.bytes,
        first_sequence: report.first_sequence.map_or(0, |sequence| sequence.0),
        last_sequence: report.last_sequence.map_or(0, |sequence| sequence.0),
        checksum_failures: report.checksum_failures,
        sequence_failures: report.sequence_failures,
        has_first_sequence: u8::from(report.first_sequence.is_some()),
        has_last_sequence: u8::from(report.last_sequence.is_some()),
        valid: u8::from(report.valid),
    }
}

fn checkpoint_store_integrity_report_to_ffi(
    report: CheckpointStoreIntegrityReport,
) -> of_execution_checkpoint_store_integrity_report_t {
    of_execution_checkpoint_store_integrity_report_t {
        checkpoint_files: report.checkpoint_files,
        valid_checkpoints: report.valid_checkpoints,
        invalid_checkpoints: report.invalid_checkpoints,
        bytes: report.bytes,
        latest_checkpoint_id: report.latest_checkpoint_id.unwrap_or(0),
        latest_last_applied_sequence: report
            .latest_last_applied_sequence
            .map_or(0, |sequence| sequence.0),
        latest_created_ns: report.latest_created_ns.unwrap_or(0),
        has_latest: u8::from(report.latest_checkpoint_id.is_some()),
        valid: u8::from(report.valid),
    }
}

fn fixed_from_ptr<const N: usize>(ptr: *const c_char) -> Result<FixedAscii<N>, ()> {
    let value = non_empty_string(ptr).ok_or(())?;
    FixedAscii::new(&value).map_err(|_| ())
}

fn twap_config_from_ffi(
    config: &of_execution_twap_config_t,
) -> Result<(ParentOrder, TwapSlicePlanner), ()> {
    let parent = ParentOrder::new(
        fixed_from_ptr::<40>(config.parent_order_id)?,
        fixed_from_ptr::<32>(config.account_id)?,
        fixed_from_ptr::<32>(config.route_id)?,
        fixed_from_ptr::<32>(config.strategy_id).unwrap_or_else(|_| StrategyId::empty()),
        ExecutionSymbol {
            venue: fixed_from_ptr::<16>(config.venue)?,
            instrument: fixed_from_ptr::<32>(config.instrument)?,
        },
        side_from_execution_ffi(config.side)?,
        order_type_from_ffi(config.order_type)?,
        tif_from_ffi(config.time_in_force)?,
        OrderQty(config.total_qty),
        OrderPrice(config.limit_price),
        OrderPrice(config.stop_price),
        config.start_ns,
        config.end_ns,
        OrderQty(config.min_clip),
        OrderQty(config.max_clip),
        config.participation_cap_bps,
    )
    .map_err(|_| ())?;
    let planner = TwapSlicePlanner::try_new(config.slice_interval_ns).map_err(|_| ())?;
    Ok((parent, planner))
}

fn child_plan_to_ffi(plan: Option<&ChildOrderPlan>) -> of_execution_algo_child_plan_t {
    let Some(plan) = plan else {
        return of_execution_algo_child_plan_t {
            child_order_id: [0; 41],
            parent_order_id: [0; 41],
            client_order_id: [0; 41],
            account_id: [0; 33],
            route_id: [0; 33],
            strategy_id: [0; 33],
            venue: [0; 17],
            instrument: [0; 33],
            side: 0,
            order_type: 0,
            time_in_force: 0,
            quantity: 0,
            limit_price: 0,
            stop_price: 0,
            due_ns: 0,
            ts_recv_ns: 0,
            has_plan: 0,
        };
    };
    let request = plan.request();
    of_execution_algo_child_plan_t {
        child_order_id: cstr_array(plan.child_id().as_str()),
        parent_order_id: cstr_array(plan.parent_id().as_str()),
        client_order_id: cstr_array(request.client_order_id.as_str()),
        account_id: cstr_array(request.account_id.as_str()),
        route_id: cstr_array(request.route_id.as_str()),
        strategy_id: cstr_array(request.strategy_id.as_str()),
        venue: cstr_array(request.symbol.venue.as_str()),
        instrument: cstr_array(request.symbol.instrument.as_str()),
        side: request.side as u32,
        order_type: request.order_type as u32,
        time_in_force: request.time_in_force as u32,
        quantity: request.quantity.0,
        limit_price: request.limit_price.0,
        stop_price: request.stop_price.0,
        due_ns: plan.due_ns(),
        ts_recv_ns: request.ts_recv_ns,
        has_plan: 1,
    }
}

const fn algo_progress_to_ffi(
    progress: AlgoProgress,
    has_pending_plan: bool,
) -> of_execution_algo_progress_t {
    of_execution_algo_progress_t {
        target_qty: progress.target_qty().0,
        released_qty: progress.released_qty().0,
        completed_qty: progress.completed_qty().0,
        open_qty: progress.open_qty().0,
        rejected_children: progress.rejected_children(),
        terminal_children: progress.terminal_children(),
        has_pending_plan: has_pending_plan as u8,
    }
}

fn route_config_from_ffi(cfg: &of_execution_route_config_t) -> Result<RouteConfig, ()> {
    Ok(RouteConfig {
        route_id: fixed_from_ptr::<32>(cfg.route_id)?,
        account_id: fixed_from_ptr::<32>(cfg.account_id)?,
        symbol: ExecutionSymbol {
            venue: fixed_from_ptr::<16>(cfg.venue)?,
            instrument: fixed_from_ptr::<32>(cfg.instrument)?,
        },
        enabled: cfg.enabled != 0,
        risk_limits: RiskLimits {
            kill_switch: cfg.kill_switch != 0,
            max_order_qty: cfg.max_order_qty,
            max_order_notional: i128::from(cfg.max_order_notional),
            max_open_orders: cfg.max_open_orders,
            max_open_notional: i128::from(cfg.max_open_notional),
            price_band_ticks: cfg.price_band_ticks,
        },
    })
}

fn order_request_from_ffi(req: &of_execution_order_request_t) -> Result<OrderRequest, ()> {
    Ok(OrderRequest {
        client_order_id: fixed_from_ptr::<40>(req.client_order_id)?,
        account_id: fixed_from_ptr::<32>(req.account_id)?,
        route_id: fixed_from_ptr::<32>(req.route_id)?,
        strategy_id: fixed_from_ptr::<32>(req.strategy_id).unwrap_or_else(|_| StrategyId::empty()),
        symbol: ExecutionSymbol {
            venue: fixed_from_ptr::<16>(req.venue)?,
            instrument: fixed_from_ptr::<32>(req.instrument)?,
        },
        side: side_from_execution_ffi(req.side)?,
        order_type: order_type_from_ffi(req.order_type)?,
        time_in_force: tif_from_ffi(req.time_in_force)?,
        quantity: OrderQty(req.quantity),
        limit_price: OrderPrice(req.limit_price),
        stop_price: OrderPrice(req.stop_price),
        ts_exchange_ns: req.ts_exchange_ns,
        ts_recv_ns: req.ts_recv_ns,
    })
}

fn cancel_request_from_ffi(req: &of_execution_cancel_request_t) -> Result<CancelRequest, ()> {
    Ok(CancelRequest {
        client_order_id: fixed_from_ptr::<40>(req.client_order_id)?,
        orig_client_order_id: fixed_from_ptr::<40>(req.orig_client_order_id)?,
        venue_order_id: fixed_from_ptr::<48>(req.venue_order_id)
            .unwrap_or_else(|_| VenueOrderId::empty()),
        account_id: fixed_from_ptr::<32>(req.account_id)?,
        route_id: fixed_from_ptr::<32>(req.route_id)?,
        symbol: ExecutionSymbol {
            venue: fixed_from_ptr::<16>(req.venue)?,
            instrument: fixed_from_ptr::<32>(req.instrument)?,
        },
        ts_recv_ns: req.ts_recv_ns,
    })
}

fn amend_request_from_ffi(req: &of_execution_amend_request_t) -> Result<AmendRequest, ()> {
    Ok(AmendRequest {
        client_order_id: fixed_from_ptr::<40>(req.client_order_id)?,
        orig_client_order_id: fixed_from_ptr::<40>(req.orig_client_order_id)?,
        venue_order_id: fixed_from_ptr::<48>(req.venue_order_id)
            .unwrap_or_else(|_| VenueOrderId::empty()),
        account_id: fixed_from_ptr::<32>(req.account_id)?,
        route_id: fixed_from_ptr::<32>(req.route_id)?,
        symbol: ExecutionSymbol {
            venue: fixed_from_ptr::<16>(req.venue)?,
            instrument: fixed_from_ptr::<32>(req.instrument)?,
        },
        quantity: OrderQty(req.quantity),
        limit_price: OrderPrice(req.limit_price),
        ts_recv_ns: req.ts_recv_ns,
    })
}

fn side_from_execution_ffi(value: u32) -> Result<OrderSide, ()> {
    match value {
        1 => Ok(OrderSide::Buy),
        2 => Ok(OrderSide::Sell),
        _ => Err(()),
    }
}

fn order_type_from_ffi(value: u32) -> Result<OrderType, ()> {
    match value {
        1 => Ok(OrderType::Market),
        2 => Ok(OrderType::Limit),
        3 => Ok(OrderType::Stop),
        4 => Ok(OrderType::StopLimit),
        _ => Err(()),
    }
}

fn tif_from_ffi(value: u32) -> Result<TimeInForce, ()> {
    match value {
        1 => Ok(TimeInForce::Day),
        2 => Ok(TimeInForce::Gtc),
        3 => Ok(TimeInForce::Ioc),
        4 => Ok(TimeInForce::Fok),
        5 => Ok(TimeInForce::Gtd),
        _ => Err(()),
    }
}

fn order_status_from_ffi(value: u32) -> Result<OrderStatus, ()> {
    match value {
        1 => Ok(OrderStatus::PendingNew),
        2 => Ok(OrderStatus::New),
        3 => Ok(OrderStatus::PartiallyFilled),
        4 => Ok(OrderStatus::Filled),
        5 => Ok(OrderStatus::PendingCancel),
        6 => Ok(OrderStatus::Cancelled),
        7 => Ok(OrderStatus::PendingReplace),
        8 => Ok(OrderStatus::Replaced),
        9 => Ok(OrderStatus::Rejected),
        10 => Ok(OrderStatus::Expired),
        11 => Ok(OrderStatus::Suspended),
        12 => Ok(OrderStatus::Unknown),
        _ => Err(()),
    }
}

fn copy_execution_events(
    events: &ExecutionEventBuffer,
    out_events: *mut of_execution_event_t,
    inout_len: *mut u32,
) -> i32 {
    if inout_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let capacity = unsafe { *inout_len as usize };
    let needed = events.len();
    unsafe {
        *inout_len = needed as u32;
    }
    if needed == 0 {
        return of_error_t::OF_OK as i32;
    }
    if out_events.is_null() {
        return of_error_t::OF_ERR_BACKPRESSURE as i32;
    }
    if capacity < needed {
        return of_error_t::OF_ERR_BACKPRESSURE as i32;
    }
    for (idx, event) in events.as_slice().iter().enumerate() {
        unsafe {
            *out_events.add(idx) = event_to_ffi(event);
        }
    }
    of_error_t::OF_OK as i32
}

fn event_to_ffi(event: &ExecutionEvent) -> of_execution_event_t {
    of_execution_event_t {
        exec_type: event.exec_type as u32,
        order_status: event.order_status as u32,
        client_order_id: cstr_array(event.client_order_id.as_str()),
        orig_client_order_id: cstr_array(event.orig_client_order_id.as_str()),
        venue_order_id: cstr_array(event.venue_order_id.as_str()),
        execution_id: cstr_array(event.execution_id.as_str()),
        account_id: cstr_array(event.account_id.as_str()),
        route_id: cstr_array(event.route_id.as_str()),
        venue: cstr_array(event.symbol.venue.as_str()),
        instrument: cstr_array(event.symbol.instrument.as_str()),
        last_qty: event.last_qty.0,
        last_price: event.last_price.0,
        cumulative_qty: event.cumulative_qty.0,
        leaves_qty: event.leaves_qty.0,
        average_price: event.average_price.0,
        ts_exchange_ns: event.ts_exchange_ns,
        ts_recv_ns: event.ts_recv_ns,
        reason: event.reason as u32,
        text: cstr_array(event.text.as_str()),
    }
}

fn order_state_to_ffi(state: &OrderState) -> of_execution_order_state_t {
    of_execution_order_state_t {
        client_order_id: cstr_array(state.client_order_id.as_str()),
        venue_order_id: cstr_array(state.venue_order_id.as_str()),
        account_id: cstr_array(state.account_id.as_str()),
        route_id: cstr_array(state.route_id.as_str()),
        venue: cstr_array(state.symbol.venue.as_str()),
        instrument: cstr_array(state.symbol.instrument.as_str()),
        status: state.status as u32,
        order_qty: state.order_qty.0,
        cumulative_qty: state.cumulative_qty.0,
        leaves_qty: state.leaves_qty.0,
        average_price: state.average_price.0,
        updated_ns: state.updated_ns,
    }
}

fn cstr_array<const N: usize>(value: &str) -> [c_char; N] {
    let mut out = [0 as c_char; N];
    if N == 0 {
        return out;
    }
    let bytes = value.as_bytes();
    let max = bytes.len().min(N - 1);
    for idx in 0..max {
        out[idx] = bytes[idx] as c_char;
    }
    out
}

#[cfg(test)]
include!("tests.rs");
