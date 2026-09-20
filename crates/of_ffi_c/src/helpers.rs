use super::*;

pub(crate) fn map_runtime_error(err: &RuntimeError) -> i32 {
    if err.is_backpressure() {
        of_error_t::OF_ERR_BACKPRESSURE as i32
    } else {
        of_error_t::OF_ERR_STATE as i32
    }
}

pub(crate) fn map_execution_result(result: Result<(), ExecutionError>) -> i32 {
    match result {
        Ok(()) => of_error_t::OF_OK as i32,
        Err(err) => map_execution_error(&err),
    }
}

pub(crate) fn map_execution_error(err: &ExecutionError) -> i32 {
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

pub(crate) fn map_concurrent_execution_error(err: &ConcurrentExecutionError) -> i32 {
    match err {
        ConcurrentExecutionError::Backpressure => of_error_t::OF_ERR_BACKPRESSURE as i32,
        ConcurrentExecutionError::Stopped | ConcurrentExecutionError::WorkerPanic => {
            of_error_t::OF_ERR_STATE as i32
        }
        ConcurrentExecutionError::Execution(err) => map_execution_error(err),
    }
}

pub(crate) fn route_configs_from_ffi(
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

pub(crate) fn concurrent_config_from_ffi(
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

pub(crate) fn nonzero_usize(value: u32, default_value: usize) -> usize {
    if value == 0 {
        default_value
    } else {
        value as usize
    }
}

pub(crate) fn send_concurrent_command(
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

pub(crate) fn write_concurrent_report(
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

pub(crate) fn execution_command_kind_to_u32(kind: ExecutionCommandKind) -> u32 {
    match kind {
        ExecutionCommandKind::Submit => 1,
        ExecutionCommandKind::Cancel => 2,
        ExecutionCommandKind::Amend => 3,
        ExecutionCommandKind::Poll => 4,
        ExecutionCommandKind::RecoverOpenOrders => 5,
        ExecutionCommandKind::Stop => 6,
    }
}

pub(crate) fn write_optional_u64(ptr: *mut u64, value: u64) {
    if !ptr.is_null() {
        unsafe {
            *ptr = value;
        }
    }
}

pub(crate) fn wal_integrity_report_to_ffi(
    report: WalIntegrityReport,
) -> of_execution_wal_integrity_report_t {
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

pub(crate) fn segmented_wal_integrity_report_to_ffi(
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

pub(crate) fn checkpoint_store_integrity_report_to_ffi(
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

pub(crate) fn fixed_from_ptr<const N: usize>(ptr: *const c_char) -> Result<FixedAscii<N>, ()> {
    let value = non_empty_string(ptr).ok_or(())?;
    FixedAscii::new(&value).map_err(|_| ())
}

pub(crate) fn twap_config_from_ffi(
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

pub(crate) fn child_plan_to_ffi(plan: Option<&ChildOrderPlan>) -> of_execution_algo_child_plan_t {
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

pub(crate) const fn algo_progress_to_ffi(
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

pub(crate) fn route_config_from_ffi(cfg: &of_execution_route_config_t) -> Result<RouteConfig, ()> {
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

pub(crate) fn order_request_from_ffi(
    req: &of_execution_order_request_t,
) -> Result<OrderRequest, ()> {
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

pub(crate) fn cancel_request_from_ffi(
    req: &of_execution_cancel_request_t,
) -> Result<CancelRequest, ()> {
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

pub(crate) fn amend_request_from_ffi(
    req: &of_execution_amend_request_t,
) -> Result<AmendRequest, ()> {
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

pub(crate) fn side_from_execution_ffi(value: u32) -> Result<OrderSide, ()> {
    match value {
        1 => Ok(OrderSide::Buy),
        2 => Ok(OrderSide::Sell),
        _ => Err(()),
    }
}

pub(crate) fn order_type_from_ffi(value: u32) -> Result<OrderType, ()> {
    match value {
        1 => Ok(OrderType::Market),
        2 => Ok(OrderType::Limit),
        3 => Ok(OrderType::Stop),
        4 => Ok(OrderType::StopLimit),
        _ => Err(()),
    }
}

pub(crate) fn tif_from_ffi(value: u32) -> Result<TimeInForce, ()> {
    match value {
        1 => Ok(TimeInForce::Day),
        2 => Ok(TimeInForce::Gtc),
        3 => Ok(TimeInForce::Ioc),
        4 => Ok(TimeInForce::Fok),
        5 => Ok(TimeInForce::Gtd),
        _ => Err(()),
    }
}

pub(crate) fn order_status_from_ffi(value: u32) -> Result<OrderStatus, ()> {
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

pub(crate) fn copy_execution_events(
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

pub(crate) fn event_to_ffi(event: &ExecutionEvent) -> of_execution_event_t {
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

pub(crate) fn order_state_to_ffi(state: &OrderState) -> of_execution_order_state_t {
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

pub(crate) fn cstr_array<const N: usize>(value: &str) -> [c_char; N] {
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
