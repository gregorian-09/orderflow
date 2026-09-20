use super::*;

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

/// Configures an engine-owned bounded segmented market-data WAL.
///
/// This is a blocking control-plane operation. Existing engine creation and
/// JSONL persistence behavior remain unchanged until this function is called.
#[no_mangle]
pub extern "C" fn of_configure_market_data_wal(
    engine: *mut of_engine,
    cfg: *const of_market_data_wal_config_t,
) -> i32 {
    if engine.is_null() || cfg.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    let cfg = unsafe { &*cfg };
    let Some(root_path) = non_empty_string(cfg.root_path) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let mut wal_config = SegmentedMarketDataWalConfig::new(root_path);
    if cfg.max_segment_bytes > 0 {
        wal_config = wal_config.with_max_segment_bytes(cfg.max_segment_bytes);
    }
    if cfg.max_payload_bytes > 0 {
        let Ok(max_payload_bytes) = usize::try_from(cfg.max_payload_bytes) else {
            return of_error_t::OF_ERR_INVALID_ARG as i32;
        };
        wal_config = wal_config.with_max_payload_bytes(max_payload_bytes);
    }
    let sync_policy = match cfg.sync_policy {
        0 => MarketDataWalSyncPolicy::OnSegmentSeal,
        1 => MarketDataWalSyncPolicy::Never,
        2 => MarketDataWalSyncPolicy::EveryRecord,
        3 if cfg.sync_every_records > 0 => {
            MarketDataWalSyncPolicy::EveryRecords(cfg.sync_every_records)
        }
        _ => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    wal_config = wal_config
        .with_sync_policy(sync_policy)
        .with_sync_manifest(cfg.sync_manifest != 0);

    let mut writer_config = BoundedMarketDataWriterConfig::new();
    if cfg.queue_capacity > 0 {
        writer_config = writer_config.with_queue_capacity(cfg.queue_capacity as usize);
    }
    if cfg.max_queued_payload_bytes > 0 {
        let Ok(max_queued_payload_bytes) = usize::try_from(cfg.max_queued_payload_bytes) else {
            return of_error_t::OF_ERR_INVALID_ARG as i32;
        };
        writer_config = writer_config.with_max_queued_payload_bytes(max_queued_payload_bytes);
    }
    if let Some(thread_name) = non_empty_string(cfg.writer_thread_name) {
        writer_config = writer_config.with_thread_name(thread_name);
    }
    let failure_action = match cfg.failure_action {
        0 => MarketDataPersistenceFailureAction::MarkDegraded,
        1 => MarketDataPersistenceFailureAction::StopMarketData,
        2 => MarketDataPersistenceFailureAction::StopTrading,
        3 => MarketDataPersistenceFailureAction::FailProcess,
        4 => MarketDataPersistenceFailureAction::MemoryOnly,
        _ => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };

    match engine
        .inner
        .configure_market_data_wal(wal_config, writer_config, failure_action)
    {
        Ok(()) => of_error_t::OF_OK as i32,
        Err(RuntimeError::Config(_)) => of_error_t::OF_ERR_STATE as i32,
        Err(_) => of_error_t::OF_ERR_IO as i32,
    }
}

/// Flushes an engine-owned market-data WAL through a durability barrier.
#[no_mangle]
pub extern "C" fn of_flush_market_data_wal(engine: *mut of_engine) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    match engine.inner.flush_market_data_persistence() {
        Ok(()) => of_error_t::OF_OK as i32,
        Err(RuntimeError::Config(_)) => of_error_t::OF_ERR_STATE as i32,
        Err(_) => of_error_t::OF_ERR_IO as i32,
    }
}

/// Drains, synchronizes, and shuts down engine-owned market-data persistence.
#[no_mangle]
pub extern "C" fn of_shutdown_market_data_wal(engine: *mut of_engine) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    match engine.inner.shutdown_market_data_persistence() {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(_) => of_error_t::OF_ERR_IO as i32,
    }
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
