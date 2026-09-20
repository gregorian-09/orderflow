use super::*;

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

/// Engine-owned segmented market-data WAL configuration.
#[repr(C)]
pub struct of_market_data_wal_config_t {
    /// Required WAL directory path.
    pub root_path: *const c_char,
    /// Soft segment size in bytes, or zero for the library default.
    pub max_segment_bytes: u64,
    /// Maximum encoded payload bytes, or zero for the library default.
    pub max_payload_bytes: u64,
    /// Sync policy (`0=segment seal`, `1=never`, `2=every record`, `3=every N`).
    pub sync_policy: u32,
    /// Record cadence used when `sync_policy` is `3`.
    pub sync_every_records: u64,
    /// Non-zero synchronizes manifest snapshots before atomic rename.
    pub sync_manifest: u8,
    /// Bounded queue record capacity, or zero for the library default.
    pub queue_capacity: u32,
    /// Bounded aggregate queued payload bytes, or zero for the library default.
    pub max_queued_payload_bytes: u64,
    /// Failure action (`0=degrade`, `1=stop data`, `2=stop trading`, `3=fail`, `4=memory`).
    pub failure_action: u32,
    /// Optional native writer thread name.
    pub writer_thread_name: *const c_char,
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
    pub(crate) inner: DefaultEngine,
    pub(crate) subs: Vec<SubscriptionRecord>,
}

/// Opaque execution engine handle.
pub struct of_execution_engine {
    pub(crate) inner: ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal>,
}

/// Opaque concurrent execution engine handle.
pub struct of_execution_concurrent_engine {
    pub(crate) inner: ConcurrentExecutionEngine,
}

/// Opaque deterministic TWAP algorithm handle.
pub struct of_execution_twap_algo {
    pub(crate) parent: ParentOrder,
    pub(crate) progress: AlgoProgress,
    pub(crate) planner: TwapSlicePlanner,
    pub(crate) pending: Option<ChildOrderPlan>,
}

/// Opaque subscription token.
pub struct of_subscription {
    pub(crate) token: *mut SubscriptionToken,
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

pub(crate) struct SubscriptionRecord {
    pub(crate) symbol: SymbolId,
    pub(crate) kind: u32,
    pub(crate) cb: of_event_cb,
    pub(crate) user_data: *mut c_void,
    pub(crate) active: Arc<AtomicBool>,
    pub(crate) last_health_seq: u64,
}

pub(crate) struct SubscriptionToken {
    pub(crate) active: Arc<AtomicBool>,
}
