use super::*;

/// Parses a validated FIX `ExecutionReport(35=8)` into a normalized report.
///
/// The mapper is deliberately profile-aware through `config`: quantity and
/// price fields are scaled into Orderflow's integer-normalized OMS types using
/// caller-provided scale factors.
///
/// # Errors
///
/// Returns [`FixReportParseError`] when required fields are absent, enum values
/// are unsupported, ASCII identifiers cannot fit their canonical bounds, or
/// decimal fields cannot be represented with the configured scale.
pub fn parse_execution_report(
    message: &FixMessageView<'_>,
    config: FixReportParseConfig,
    ts_recv_ns: u64,
) -> Result<FixExecutionReport, FixReportParseError> {
    if message.msg_type() != Some(FixMsgType::EXECUTION_REPORT.as_bytes()) {
        return Err(FixReportParseError::InvalidMsgType);
    }

    let exec_type = parse_exec_type(required(message, FixTag::EXEC_TYPE)?)?;
    let ord_status = parse_ord_status(required(message, FixTag::ORD_STATUS)?)?;
    let cl_ord_id = fixed_required(message, FixTag::CL_ORD_ID)?;
    let orig_cl_ord_id = fixed_optional(message, FixTag::ORIG_CL_ORD_ID)?;
    let order_id = fixed_required(message, FixTag::ORDER_ID)?;
    let exec_id = fixed_required(message, FixTag::EXEC_ID)?;
    let account_id = if let Some(account) = message.get(ACCOUNT_TAG) {
        fixed_from_bytes(ACCOUNT_TAG, account)?
    } else {
        config.account_id
    };
    let instrument: InstrumentId = fixed_required(message, FixTag::SYMBOL)?;

    Ok(FixExecutionReport {
        exec_type,
        ord_status,
        cl_ord_id,
        orig_cl_ord_id,
        order_id,
        exec_id,
        account_id,
        route_id: config.route_id,
        symbol: ExecutionSymbol {
            venue: config.venue,
            instrument,
        },
        last_qty: OrderQty(parse_optional_scaled(
            message,
            FixTag::LAST_QTY,
            config.quantity_scale,
        )?),
        last_price: OrderPrice(parse_optional_scaled(
            message,
            FixTag::LAST_PX,
            config.price_scale,
        )?),
        cumulative_qty: OrderQty(parse_optional_scaled(
            message,
            FixTag::CUM_QTY,
            config.quantity_scale,
        )?),
        leaves_qty: OrderQty(parse_optional_scaled(
            message,
            FixTag::LEAVES_QTY,
            config.quantity_scale,
        )?),
        average_price: OrderPrice(parse_optional_scaled(
            message,
            FixTag::AVG_PX,
            config.price_scale,
        )?),
        ts_exchange_ns: parse_optional_u64(message, FixTag::TRANSACT_TIME)?,
        ts_recv_ns,
        text: fixed_optional(message, FixTag::TEXT)?,
    })
}

/// Parses a validated FIX `OrderCancelReject(35=9)` into a normalized report.
///
/// # Errors
///
/// Returns [`FixReportParseError`] when required fields are absent, enum values
/// are unsupported, ASCII identifiers cannot fit their canonical bounds, or
/// numeric fields cannot be parsed.
pub fn parse_order_cancel_reject(
    message: &FixMessageView<'_>,
    config: FixReportParseConfig,
    ts_recv_ns: u64,
) -> Result<FixOrderCancelReject, FixReportParseError> {
    if message.msg_type() != Some(FixMsgType::ORDER_CANCEL_REJECT.as_bytes()) {
        return Err(FixReportParseError::InvalidMsgType);
    }

    let response_to = parse_cancel_reject_response_to(required(message, CXL_REJ_RESPONSE_TO_TAG)?)?;
    let ord_status = parse_ord_status(required(message, FixTag::ORD_STATUS)?)?;
    let cl_ord_id = fixed_required(message, FixTag::CL_ORD_ID)?;
    let orig_cl_ord_id = fixed_required(message, FixTag::ORIG_CL_ORD_ID)?;
    let account_id = if let Some(account) = message.get(ACCOUNT_TAG) {
        fixed_from_bytes(ACCOUNT_TAG, account)?
    } else {
        config.account_id
    };
    let instrument: InstrumentId = fixed_optional(message, FixTag::SYMBOL)?;

    Ok(FixOrderCancelReject {
        response_to,
        ord_status,
        cl_ord_id,
        orig_cl_ord_id,
        order_id: fixed_optional(message, FixTag::ORDER_ID)?,
        account_id,
        route_id: config.route_id,
        symbol: ExecutionSymbol {
            venue: config.venue,
            instrument,
        },
        cxl_rej_reason: parse_optional_u64(message, CXL_REJ_REASON_TAG)?,
        ts_exchange_ns: parse_optional_u64(message, FixTag::TRANSACT_TIME)?,
        ts_recv_ns,
        text: fixed_optional(message, FixTag::TEXT)?,
    })
}

/// Encodes a canonical new-order request as FIX NewOrderSingle `<D>`.
///
/// `transact_time` must already be in the venue/profile's accepted FIX wire
/// format. The bridge scales integer-normalized quantity and price fields into
/// decimal ASCII using `config`.
///
/// # Errors
///
/// Returns [`FixRequestEncodeError`] when scales are invalid, required
/// quantity/price fields are not positive, the order type needs unsupported
/// fields, or the underlying FIX encoder rejects a field value.
pub fn encode_order_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    config: FixRequestEncodeConfig,
    request: &OrderRequest,
    transact_time: &[u8],
) -> Result<(), FixRequestEncodeError> {
    let mut qty_buf = [0u8; 40];
    let qty = encode_scaled(
        &mut qty_buf,
        request.quantity.0,
        config.quantity_scale,
        ScaledField::Quantity,
    )?;
    let mut price_buf = [0u8; 40];
    let price = encode_order_price(
        &mut price_buf,
        request.order_type,
        request.limit_price,
        config,
    )?;
    let mut stop_px_buf = [0u8; 40];
    let stop_px = encode_order_stop_price(
        &mut stop_px_buf,
        request.order_type,
        request.stop_price,
        config,
    )?;
    let mut fix_request = FixNewOrderSingle::new(
        request.client_order_id.as_str().as_bytes(),
        request.symbol.instrument.as_str().as_bytes(),
        map_side_to_fix(request.side),
        transact_time,
        qty,
        map_order_type_to_fix(request.order_type)?,
    )
    .with_account(request.account_id.as_str().as_bytes())
    .with_time_in_force(map_tif_to_fix(request.time_in_force));
    if let Some(price) = price {
        fix_request = fix_request.with_price(price);
    }
    if let Some(stop_px) = stop_px {
        fix_request = fix_request.with_stop_px(stop_px);
    }
    encode_new_order_single(out, version, header, fix_request)?;
    Ok(())
}

/// Encodes a canonical cancel request as FIX OrderCancelRequest `<F>`.
///
/// Canonical cancel requests do not carry side, so callers must supply the side
/// from local order state or venue profile context.
///
/// # Errors
///
/// Returns [`FixRequestEncodeError`] when the underlying FIX encoder rejects a
/// field value.
pub fn encode_cancel_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    request: &CancelRequest,
    context: FixCancelEncodeContext<'_>,
) -> Result<(), FixRequestEncodeError> {
    let fix_request = FixOrderCancelRequest::new(
        request.orig_client_order_id.as_str().as_bytes(),
        request.client_order_id.as_str().as_bytes(),
        request.symbol.instrument.as_str().as_bytes(),
        map_side_to_fix(context.side),
        context.transact_time,
    )
    .with_account(request.account_id.as_str().as_bytes());
    encode_order_cancel_request(out, version, header, fix_request)?;
    Ok(())
}

/// Encodes a canonical amend request as FIX OrderCancelReplaceRequest `<G>`.
///
/// Canonical amend requests do not carry side, order type, or TIF, so callers
/// must supply them from local order state or venue profile context.
///
/// # Errors
///
/// Returns [`FixRequestEncodeError`] when scales are invalid, required
/// quantity/price fields are not positive, the order type needs unsupported
/// fields, or the underlying FIX encoder rejects a field value.
pub fn encode_amend_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    config: FixRequestEncodeConfig,
    request: &AmendRequest,
    context: FixAmendEncodeContext<'_>,
) -> Result<(), FixRequestEncodeError> {
    let mut qty_buf = [0u8; 40];
    let qty = encode_scaled(
        &mut qty_buf,
        request.quantity.0,
        config.quantity_scale,
        ScaledField::Quantity,
    )?;
    let mut price_buf = [0u8; 40];
    let price = encode_order_price(
        &mut price_buf,
        context.order_type,
        request.limit_price,
        config,
    )?;
    if matches!(context.order_type, OrderType::Stop | OrderType::StopLimit) {
        return Err(FixRequestEncodeError::UnsupportedOrderType);
    }
    let mut fix_request = FixOrderCancelReplaceRequest::new(
        request.orig_client_order_id.as_str().as_bytes(),
        request.client_order_id.as_str().as_bytes(),
        request.symbol.instrument.as_str().as_bytes(),
        map_side_to_fix(context.side),
        context.transact_time,
        qty,
        map_order_type_to_fix(context.order_type)?,
    )
    .with_account(request.account_id.as_str().as_bytes())
    .with_time_in_force(map_tif_to_fix(context.time_in_force));
    if let Some(price) = price {
        fix_request = fix_request.with_price(price);
    }
    encode_order_cancel_replace_request(out, version, header, fix_request)?;
    Ok(())
}

/// Encodes a stop/stop-limit amend request as FIX OrderCancelReplaceRequest `<G>`.
///
/// This helper is separate from [`encode_amend_request`] because canonical
/// [`AmendRequest`] carries one replacement limit price but no separate stop
/// price. Callers supply the stop price explicitly through `context`.
///
/// # Errors
///
/// Returns [`FixRequestEncodeError`] when scales are invalid, required
/// quantity/price fields are not positive, `context.order_type` is not stop or
/// stop-limit, or the underlying FIX encoder rejects a field value.
pub fn encode_stop_amend_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    config: FixRequestEncodeConfig,
    request: &AmendRequest,
    context: FixStopAmendEncodeContext<'_>,
) -> Result<(), FixRequestEncodeError> {
    if !matches!(context.order_type, OrderType::Stop | OrderType::StopLimit) {
        return Err(FixRequestEncodeError::UnsupportedOrderType);
    }
    let mut qty_buf = [0u8; 40];
    let qty = encode_scaled(
        &mut qty_buf,
        request.quantity.0,
        config.quantity_scale,
        ScaledField::Quantity,
    )?;
    let mut price_buf = [0u8; 40];
    let price = encode_order_price(
        &mut price_buf,
        context.order_type,
        request.limit_price,
        config,
    )?;
    let mut stop_px_buf = [0u8; 40];
    let stop_px = encode_order_stop_price(
        &mut stop_px_buf,
        context.order_type,
        context.stop_price,
        config,
    )?
    .ok_or(FixRequestEncodeError::InvalidPrice)?;
    let mut fix_request = FixOrderCancelReplaceRequest::new(
        request.orig_client_order_id.as_str().as_bytes(),
        request.client_order_id.as_str().as_bytes(),
        request.symbol.instrument.as_str().as_bytes(),
        map_side_to_fix(context.side),
        context.transact_time,
        qty,
        map_order_type_to_fix(context.order_type)?,
    )
    .with_account(request.account_id.as_str().as_bytes())
    .with_stop_px(stop_px)
    .with_time_in_force(map_tif_to_fix(context.time_in_force));
    if let Some(price) = price {
        fix_request = fix_request.with_price(price);
    }
    encode_order_cancel_replace_request(out, version, header, fix_request)?;
    Ok(())
}

/// Maps a parsed FIX execution report into a canonical execution event.
pub fn map_execution_report(report: &FixExecutionReport) -> ExecutionEvent {
    ExecutionEvent {
        exec_type: map_exec_type(report.exec_type),
        order_status: map_ord_status(report.ord_status),
        client_order_id: report.cl_ord_id,
        orig_client_order_id: report.orig_cl_ord_id,
        venue_order_id: report.order_id,
        execution_id: report.exec_id,
        account_id: report.account_id,
        route_id: report.route_id,
        symbol: report.symbol,
        last_qty: report.last_qty,
        last_price: report.last_price,
        cumulative_qty: report.cumulative_qty,
        leaves_qty: report.leaves_qty,
        average_price: report.average_price,
        ts_exchange_ns: report.ts_exchange_ns,
        ts_recv_ns: report.ts_recv_ns,
        reason: if report.exec_type == FixExecType::Rejected {
            RiskRejectReason::UnsupportedOrderType
        } else {
            RiskRejectReason::None
        },
        text: report.text,
    }
}

/// Maps a parsed FIX OrderCancelReject into a canonical execution event.
pub fn map_order_cancel_reject(report: &FixOrderCancelReject) -> ExecutionEvent {
    ExecutionEvent {
        exec_type: match report.response_to {
            FixCancelRejectResponseTo::OrderCancelRequest => ExecutionType::CancelReject,
            FixCancelRejectResponseTo::OrderCancelReplaceRequest => ExecutionType::ReplaceReject,
        },
        order_status: map_ord_status(report.ord_status),
        client_order_id: report.cl_ord_id,
        orig_client_order_id: report.orig_cl_ord_id,
        venue_order_id: report.order_id,
        execution_id: ExecutionId::empty(),
        account_id: report.account_id,
        route_id: report.route_id,
        symbol: report.symbol,
        last_qty: OrderQty(0),
        last_price: OrderPrice(0),
        cumulative_qty: OrderQty(0),
        leaves_qty: OrderQty(0),
        average_price: OrderPrice(0),
        ts_exchange_ns: report.ts_exchange_ns,
        ts_recv_ns: report.ts_recv_ns,
        reason: RiskRejectReason::None,
        text: report.text,
    }
}
