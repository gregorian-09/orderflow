#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use of_adapters::{AdapterConfig, ProviderKind, RawEvent};
use of_core::{BookAction, BookUpdate, DataQualityFlags, Side, SignalState, SymbolId, TradePrint};
use of_runtime::{
    build_default_engine, load_engine_config_from_path, DefaultEngine, EngineConfig,
    ExternalFeedPolicy,
};

const API_VERSION: u32 = 0x0001_0000;
const BUILD_INFO: &[u8] = concat!("of_ffi_c/", env!("CARGO_PKG_VERSION"), "\0").as_bytes();

#[repr(C)]
pub struct of_engine_config_t {
    pub instance_id: *const c_char,
    pub config_path: *const c_char,
    pub log_level: u32,
    pub enable_persistence: u8,
    pub audit_max_bytes: u64,
    pub audit_max_files: u32,
    pub audit_redact_tokens_csv: *const c_char,
    pub data_retention_max_bytes: u64,
    pub data_retention_max_age_secs: u64,
}

#[repr(C)]
pub struct of_symbol_t {
    pub venue: *const c_char,
    pub symbol: *const c_char,
    pub depth_levels: u16,
}

#[repr(C)]
pub struct of_trade_t {
    pub symbol: of_symbol_t,
    pub price: i64,
    pub size: i64,
    pub aggressor_side: u32,
    pub sequence: u64,
    pub ts_exchange_ns: u64,
    pub ts_recv_ns: u64,
}

#[repr(C)]
pub struct of_book_t {
    pub symbol: of_symbol_t,
    pub side: u32,
    pub level: u16,
    pub price: i64,
    pub size: i64,
    pub action: u32,
    pub sequence: u64,
    pub ts_exchange_ns: u64,
    pub ts_recv_ns: u64,
}

#[repr(C)]
pub struct of_external_feed_policy_t {
    pub stale_after_ms: u64,
    pub enforce_sequence: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum of_error_t {
    OF_OK = 0,
    OF_ERR_INVALID_ARG = 1,
    OF_ERR_STATE = 2,
    OF_ERR_IO = 3,
    OF_ERR_AUTH = 4,
    OF_ERR_BACKPRESSURE = 5,
    OF_ERR_DATA_QUALITY = 6,
    OF_ERR_INTERNAL = 255,
}

pub struct of_engine {
    inner: DefaultEngine,
    subs: Vec<SubscriptionRecord>,
}

pub struct of_subscription {
    token: *mut SubscriptionToken,
}

#[repr(C)]
pub struct of_event_t {
    pub ts_exchange_ns: u64,
    pub ts_recv_ns: u64,
    pub kind: u32,
    pub payload: *const c_void,
    pub payload_len: u32,
    pub schema_id: u32,
    pub quality_flags: u32,
}

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

fn cstr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let s = unsafe { CStr::from_ptr(ptr) };
    s.to_str().ok().map(|v| v.to_string())
}

fn non_empty_string(ptr: *const c_char) -> Option<String> {
    let v = cstr_to_string(ptr)?;
    if v.trim().is_empty() {
        None
    } else {
        Some(v)
    }
}

fn parse_csv(ptr: *const c_char) -> Option<Vec<String>> {
    let raw = non_empty_string(ptr)?;
    let values = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn symbol_from_ffi(sym: *const of_symbol_t) -> Result<(SymbolId, u16), of_error_t> {
    if sym.is_null() {
        return Err(of_error_t::OF_ERR_INVALID_ARG);
    }
    symbol_from_ffi_ref(unsafe { &*sym })
}

fn symbol_from_ffi_ref(sym: &of_symbol_t) -> Result<(SymbolId, u16), of_error_t> {
    let venue = cstr_to_string(sym.venue).ok_or(of_error_t::OF_ERR_INVALID_ARG)?;
    let symbol = cstr_to_string(sym.symbol).ok_or(of_error_t::OF_ERR_INVALID_ARG)?;
    Ok((SymbolId { venue, symbol }, sym.depth_levels))
}

fn side_from_ffi(value: u32) -> Result<Side, of_error_t> {
    match value {
        0 => Ok(Side::Bid),
        1 => Ok(Side::Ask),
        _ => Err(of_error_t::OF_ERR_INVALID_ARG),
    }
}

fn action_from_ffi(value: u32) -> Result<BookAction, of_error_t> {
    match value {
        0 => Ok(BookAction::Upsert),
        1 => Ok(BookAction::Delete),
        _ => Err(of_error_t::OF_ERR_INVALID_ARG),
    }
}

#[no_mangle]
pub extern "C" fn of_api_version() -> u32 {
    API_VERSION
}

#[no_mangle]
pub extern "C" fn of_build_info() -> *const c_char {
    BUILD_INFO.as_ptr() as *const c_char
}

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

#[no_mangle]
pub extern "C" fn of_engine_stop(engine: *mut of_engine) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    engine.inner.stop();
    of_error_t::OF_OK as i32
}

#[no_mangle]
pub extern "C" fn of_engine_destroy(engine: *mut of_engine) {
    if !engine.is_null() {
        unsafe {
            drop(Box::from_raw(engine));
        }
    }
}

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

fn write_json_to_c_buffer(
    value: &str,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> Result<(), of_error_t> {
    if out_buf.is_null() || inout_len.is_null() {
        return Err(of_error_t::OF_ERR_INVALID_ARG);
    }

    let needed = value.len() as u32;
    let cap = unsafe { *inout_len };
    unsafe {
        *inout_len = needed;
    }
    if cap < needed {
        return Err(of_error_t::OF_ERR_INVALID_ARG);
    }

    unsafe {
        ptr::copy_nonoverlapping(value.as_ptr(), out_buf as *mut u8, needed as usize);
    }
    Ok(())
}

#[no_mangle]
pub extern "C" fn of_get_book_snapshot(
    _engine: *mut of_engine,
    _symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    let payload = "{}";
    match write_json_to_c_buffer(payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

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
        Some(snap) => format!(
            "{{\"delta\":{},\"cumulative_delta\":{},\"buy_volume\":{},\"sell_volume\":{},\"last_price\":{},\"point_of_control\":{},\"value_area_low\":{},\"value_area_high\":{}}}",
            snap.delta,
            snap.cumulative_delta,
            snap.buy_volume,
            snap.sell_volume,
            snap.last_price,
            snap.point_of_control,
            snap.value_area_low,
            snap.value_area_high
        ),
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

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
                snap.module_id, state, snap.confidence_bps, snap.quality_flags, snap.reason
            )
        }
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

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

#[no_mangle]
pub extern "C" fn of_string_free(p: *const c_char) {
    if p.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(p as *mut c_char);
    }
}

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

fn dispatch_callbacks(engine: &mut of_engine, quality_flags: u32) {
    engine.subs.retain(|s| s.active.load(Ordering::Acquire));
    for sub in &mut engine.subs {
        if !sub.active.load(Ordering::Acquire) {
            continue;
        }

        if sub.kind == 1 || sub.kind == 2 {
            for event in engine.inner.last_events() {
                let payload = match event {
                    RawEvent::Book(book) if sub.kind == 1 && book.symbol == sub.symbol => {
                        Some(format_book_event(book))
                    }
                    RawEvent::Trade(trade) if sub.kind == 2 && trade.symbol == sub.symbol => {
                        Some(format_trade_event(trade))
                    }
                    _ => None,
                };
                let Some(payload) = payload else {
                    continue;
                };
                let (ts_exchange_ns, ts_recv_ns) = match event {
                    RawEvent::Book(book) => (book.ts_exchange_ns, book.ts_recv_ns),
                    RawEvent::Trade(trade) => (trade.ts_exchange_ns, trade.ts_recv_ns),
                };
                let event = of_event_t {
                    ts_exchange_ns,
                    ts_recv_ns,
                    kind: sub.kind,
                    payload: payload.as_ptr() as *const c_void,
                    payload_len: payload.len() as u32,
                    schema_id: 1,
                    quality_flags,
                };
                (sub.cb)(&event as *const of_event_t, sub.user_data);
            }
            continue;
        }

        if sub.kind == 5 {
            let seq = engine.inner.health_seq();
            if seq == sub.last_health_seq {
                continue;
            }
            sub.last_health_seq = seq;
        }

        let payload = match sub.kind {
            3 => {
                // analytics
                match engine.inner.analytics_snapshot(&sub.symbol) {
                    Some(s) => format!(
                        "{{\"delta\":{},\"cumulative_delta\":{},\"buy_volume\":{},\"sell_volume\":{},\"last_price\":{},\"point_of_control\":{},\"value_area_low\":{},\"value_area_high\":{}}}",
                        s.delta,
                        s.cumulative_delta,
                        s.buy_volume,
                        s.sell_volume,
                        s.last_price,
                        s.point_of_control,
                        s.value_area_low,
                        s.value_area_high
                    ),
                    None => "{}".to_string(),
                }
            }
            4 => {
                // signal
                match engine.inner.signal_snapshot(&sub.symbol) {
                    Some(s) => {
                        let state = match s.state {
                            SignalState::Neutral => "neutral",
                            SignalState::LongBias => "long_bias",
                            SignalState::ShortBias => "short_bias",
                            SignalState::Blocked => "blocked",
                        };
                        format!(
                            "{{\"module\":\"{}\",\"state\":\"{}\",\"confidence_bps\":{},\"quality_flags\":{},\"reason\":\"{}\"}}",
                            s.module_id, state, s.confidence_bps, s.quality_flags, s.reason
                        )
                    }
                    None => "{}".to_string(),
                }
            }
            5 => engine.inner.health_json(),
            _ => "{}".to_string(),
        };

        let event = of_event_t {
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
            kind: sub.kind,
            payload: payload.as_ptr() as *const c_void,
            payload_len: payload.len() as u32,
            schema_id: 1,
            quality_flags,
        };

        (sub.cb)(&event as *const of_event_t, sub.user_data);
    }
}

fn dispatch_health_callbacks(engine: &mut of_engine, quality_flags: u32) {
    engine.subs.retain(|s| s.active.load(Ordering::Acquire));
    for sub in &mut engine.subs {
        if !sub.active.load(Ordering::Acquire) || sub.kind != 5 {
            continue;
        }
        let seq = engine.inner.health_seq();
        if seq == sub.last_health_seq {
            continue;
        }
        sub.last_health_seq = seq;
        let payload = engine.inner.health_json();
        let event = of_event_t {
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
            kind: 5,
            payload: payload.as_ptr() as *const c_void,
            payload_len: payload.len() as u32,
            schema_id: 1,
            quality_flags,
        };
        (sub.cb)(&event as *const of_event_t, sub.user_data);
    }
}

fn format_trade_event(trade: &of_core::TradePrint) -> String {
    let aggressor = match trade.aggressor_side {
        Side::Bid => "Bid",
        Side::Ask => "Ask",
    };
    format!(
        "{{\"venue\":\"{}\",\"symbol\":\"{}\",\"price\":{},\"size\":{},\"aggressor\":\"{}\",\"sequence\":{},\"ts_exchange_ns\":{},\"ts_recv_ns\":{}}}",
        escape_json(&trade.symbol.venue),
        escape_json(&trade.symbol.symbol),
        trade.price,
        trade.size,
        aggressor,
        trade.sequence,
        trade.ts_exchange_ns,
        trade.ts_recv_ns
    )
}

fn format_book_event(book: &of_core::BookUpdate) -> String {
    let side = match book.side {
        Side::Bid => "Bid",
        Side::Ask => "Ask",
    };
    let action = match book.action {
        BookAction::Upsert => "Upsert",
        BookAction::Delete => "Delete",
    };
    format!(
        "{{\"venue\":\"{}\",\"symbol\":\"{}\",\"side\":\"{}\",\"level\":{},\"price\":{},\"size\":{},\"action\":\"{}\",\"sequence\":{},\"ts_exchange_ns\":{},\"ts_recv_ns\":{}}}",
        escape_json(&book.symbol.venue),
        escape_json(&book.symbol.symbol),
        side,
        book.level,
        book.price,
        book.size,
        action,
        book.sequence,
        book.ts_exchange_ns,
        book.ts_recv_ns
    )
}

fn escape_json(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::sync::{Mutex, OnceLock};

    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[derive(Default)]
    struct CallbackSink {
        payloads: Vec<String>,
        kinds: Vec<u32>,
        quality_flags: Vec<u32>,
    }

    extern "C" fn capture_event(ev: *const of_event_t, user_data: *mut c_void) {
        if ev.is_null() || user_data.is_null() {
            return;
        }

        let ev = unsafe { &*ev };
        let sink = unsafe { &mut *(user_data as *mut CallbackSink) };
        let payload = if !ev.payload.is_null() && ev.payload_len > 0 {
            let bytes =
                unsafe { std::slice::from_raw_parts(ev.payload as *const u8, ev.payload_len as usize) };
            String::from_utf8_lossy(bytes).to_string()
        } else {
            "{}".to_string()
        };
        sink.payloads.push(payload);
        sink.kinds.push(ev.kind);
        sink.quality_flags.push(ev.quality_flags);
    }

    fn analytics_json(engine: *mut of_engine, symbol: &of_symbol_t) -> String {
        let mut buf = vec![0u8; 1024];
        let mut len = buf.len() as u32;
        assert_eq!(
            of_get_analytics_snapshot(
                engine,
                symbol as *const of_symbol_t,
                buf.as_mut_ptr().cast::<c_void>(),
                &mut len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        String::from_utf8_lossy(&buf[..len as usize]).to_string()
    }

    #[test]
    fn health_stream_emits_on_state_change_only() {
        let _guard = test_lock().lock().expect("lock");

        let instance_id = CString::new("ffi-health-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                5,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert!(!sub.is_null());

        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);
        assert_eq!(
            of_engine_poll_once(engine, DataQualityFlags::ADAPTER_DEGRADED.bits()),
            of_error_t::OF_OK as i32
        );
        assert_eq!(
            of_engine_poll_once(engine, DataQualityFlags::ADAPTER_DEGRADED.bits()),
            of_error_t::OF_OK as i32
        );
        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);

        assert_eq!(sink.payloads.len(), 3);
        for kind in &sink.kinds {
            assert_eq!(*kind, 5);
        }
        assert_eq!(sink.quality_flags, vec![0, DataQualityFlags::ADAPTER_DEGRADED.bits(), 0]);

        assert!(sink.payloads[0].contains("\"health_seq\""));
        assert!(sink.payloads[0].contains("\"reconnect_state\""));
        assert!(sink.payloads[0].contains("\"protocol_info\""));

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn health_stream_stops_after_unsubscribe() {
        let _guard = test_lock().lock().expect("lock");

        let instance_id = CString::new("ffi-health-unsub-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                5,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert!(!sub.is_null());

        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);
        assert_eq!(sink.payloads.len(), 1);

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);

        assert_eq!(
            of_engine_poll_once(engine, DataQualityFlags::ADAPTER_DEGRADED.bits()),
            of_error_t::OF_OK as i32
        );
        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);

        // After unsubscribe no further events should arrive, even on health transitions.
        assert_eq!(sink.payloads.len(), 1);

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn unsubscribe_symbol_deactivates_matching_callbacks() {
        let _guard = test_lock().lock().expect("lock");

        let instance_id = CString::new("ffi-unsub-symbol-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let sym_a = CString::new("ESM6").expect("cstring");
        let sym_b = CString::new("NQM6").expect("cstring");
        let ffi_symbol_a = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: sym_a.as_ptr(),
            depth_levels: 10,
        };
        let ffi_symbol_b = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: sym_b.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub_a: *mut of_subscription = ptr::null_mut();
        let mut sub_b: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol_a as *const of_symbol_t,
                5,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub_a as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol_b as *const of_symbol_t,
                5,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub_b as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );

        let engine_ref = unsafe { &mut *engine };
        assert_eq!(engine_ref.subs.len(), 2);

        assert_eq!(
            of_unsubscribe_symbol(engine, &ffi_symbol_a as *const of_symbol_t),
            of_error_t::OF_OK as i32
        );
        assert_eq!(engine_ref.subs.len(), 1);
        assert_eq!(engine_ref.subs[0].symbol.symbol, "NQM6");

        assert_eq!(of_unsubscribe(sub_a), of_error_t::OF_OK as i32);
        assert_eq!(of_unsubscribe(sub_b), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn ingest_trade_updates_analytics_and_emits_callbacks() {
        let _guard = test_lock().lock().expect("lock");

        let instance_id = CString::new("ffi-ingest-trade-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                3,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert!(!sub.is_null());

        let trade = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505000,
            size: 9,
            aggressor_side: 1,
            sequence: 1,
            ts_exchange_ns: 10,
            ts_recv_ns: 11,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        let analytics = analytics_json(engine, &ffi_symbol);
        assert!(analytics.contains("\"delta\":9"));
        assert_eq!(sink.payloads.len(), 1);
        assert_eq!(sink.kinds, vec![3]);

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn ingest_book_rejects_invalid_side() {
        let _guard = test_lock().lock().expect("lock");

        let instance_id = CString::new("ffi-ingest-book-invalid-side").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let book = of_book_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            side: 99,
            level: 0,
            price: 505000,
            size: 1,
            action: 0,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };

        assert_eq!(
            of_ingest_book(engine, &book as *const of_book_t, 0),
            of_error_t::OF_ERR_INVALID_ARG as i32
        );

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn external_supervisor_sequence_gap_is_propagated_to_callbacks() {
        let _guard = test_lock().lock().expect("lock");

        let instance_id = CString::new("ffi-external-seq-gap").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let policy = of_external_feed_policy_t {
            stale_after_ms: 0,
            enforce_sequence: 1,
        };
        assert_eq!(
            of_configure_external_feed(engine, &policy as *const of_external_feed_policy_t),
            of_error_t::OF_OK as i32
        );

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                3,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );

        let trade1 = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505000,
            size: 1,
            aggressor_side: 1,
            sequence: 1,
            ts_exchange_ns: 1,
            ts_recv_ns: 1,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade1 as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        let trade_gap = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505010,
            size: 1,
            aggressor_side: 1,
            sequence: 3,
            ts_exchange_ns: 2,
            ts_recv_ns: 2,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade_gap as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        let last_flag = *sink.quality_flags.last().expect("quality flag");
        assert!(last_flag & DataQualityFlags::SEQUENCE_GAP.bits() != 0);

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }
}
