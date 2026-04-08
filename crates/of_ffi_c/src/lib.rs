#![allow(non_camel_case_types)]
#![doc = include_str!("../README.md")]

mod support;

use std::ffi::{c_char, c_void, CString};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use of_adapters::{AdapterConfig, ProviderKind};
use of_core::{
    BookUpdate, DataQualityFlags, SignalState, SymbolId, TradePrint,
};
use of_runtime::{
    build_default_engine, load_engine_config_from_path, DefaultEngine, EngineConfig,
    ExternalFeedPolicy,
};
use support::{
    action_from_ffi, dispatch_callbacks, dispatch_health_callbacks, escape_json,
    format_analytics_snapshot, format_book_snapshot, format_derived_analytics_snapshot,
    format_interval_candle_snapshot, format_session_candle_snapshot, non_empty_string, parse_csv,
    side_from_ffi, symbol_from_ffi, symbol_from_ffi_ref, write_json_to_c_buffer,
};


const API_VERSION: u32 = 0x0001_0000;
const BUILD_INFO: &[u8] = concat!("of_ffi_c/", env!("CARGO_PKG_VERSION"), "\0").as_bytes();

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
    /// Internal/unknown failure.
    OF_ERR_INTERNAL = 255,
}

/// Opaque engine handle.
pub struct of_engine {
    inner: DefaultEngine,
    subs: Vec<SubscriptionRecord>,
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
    if engine.inner.subscribe(symbol.clone(), depth_levels).is_err() {
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
pub extern "C" fn of_unsubscribe_symbol(
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
    let c = match CString::new(metrics) {
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
        Err(_) => of_error_t::OF_ERR_STATE as i32,
    }
}


#[cfg(test)]
include!("tests.rs");
