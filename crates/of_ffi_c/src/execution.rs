use super::*;

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
