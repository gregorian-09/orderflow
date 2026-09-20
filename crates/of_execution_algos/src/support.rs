use super::*;

pub(crate) fn div_ceil_u64(lhs: u64, rhs: u64) -> u64 {
    lhs / rhs + u64::from(!lhs.is_multiple_of(rhs))
}

pub(crate) fn div_ceil_i128(lhs: i128, rhs: i128) -> i128 {
    lhs / rhs + i128::from(lhs % rhs != 0)
}

pub(crate) fn participation_qty(volume: i64, bps: u16) -> i64 {
    let value = i128::from(volume) * i128::from(bps);
    i64::try_from(value / 10_000).unwrap_or(i64::MAX)
}

pub(crate) fn vwap_target_qty(parent_qty: i64, cumulative_weight: u64, total_weight: u64) -> i64 {
    let value = i128::from(parent_qty) * i128::from(cumulative_weight);
    i64::try_from(value / i128::from(total_weight)).unwrap_or(i64::MAX)
}

pub(crate) fn elapsed_bps(start_ns: u64, end_ns: u64, now_ns: u64) -> u16 {
    if now_ns <= start_ns {
        return 0;
    }
    if now_ns >= end_ns {
        return 10_000;
    }
    let elapsed = now_ns.saturating_sub(start_ns);
    let total = end_ns.saturating_sub(start_ns).max(1);
    u16::try_from((u128::from(elapsed) * 10_000) / u128::from(total)).unwrap_or(10_000)
}

pub(crate) fn scale_bps_u32(value: u32, bps: u32) -> u32 {
    let scaled = (u128::from(value) * u128::from(bps)) / 10_000;
    u32::try_from(scaled).unwrap_or(u32::MAX)
}

pub(crate) fn scale_qty_bps(quantity: i64, bps: u32) -> i64 {
    let scaled = (i128::from(quantity) * i128::from(bps)) / 10_000;
    i64::try_from(scaled).unwrap_or(i64::MAX)
}

pub(crate) fn participation_bps(child_qty: i64, market_volume: i64) -> u32 {
    if child_qty <= 0 || market_volume <= 0 {
        return 0;
    }
    let bps = (i128::from(child_qty) * 10_000) / i128::from(market_volume);
    u32::try_from(bps.clamp(0, i128::from(u32::MAX))).unwrap_or(u32::MAX)
}

pub(crate) fn price_distance_bps(price: OrderPrice, reference: OrderPrice) -> u32 {
    if price.0 <= 0 || reference.0 <= 0 {
        return u32::MAX;
    }
    let distance = price.0.abs_diff(reference.0);
    let bps = (u128::from(distance) * 10_000) / i64_to_u128(reference.0);
    u32::try_from(bps.min(u128::from(u32::MAX))).unwrap_or(u32::MAX)
}

pub(crate) fn child_notional(child: &ChildOrderPlan) -> u128 {
    i64_to_u128(child.request().quantity.0)
        .saturating_mul(i64_to_u128(child.request().limit_price.0))
}

pub(crate) fn completion_bps(completed_qty: i64, target_qty: i64) -> u16 {
    if completed_qty <= 0 || target_qty <= 0 {
        return 0;
    }
    let bps = (i128::from(completed_qty) * 10_000) / i128::from(target_qty);
    u16::try_from(bps.clamp(0, 10_000)).unwrap_or(10_000)
}

pub(crate) fn average_price_from_notional(notional: u128, quantity: OrderQty) -> OrderPrice {
    if notional == 0 || quantity.0 <= 0 {
        return OrderPrice(0);
    }
    let avg = notional / i64_to_u128(quantity.0);
    OrderPrice(i64::try_from(avg).unwrap_or(i64::MAX))
}

pub(crate) fn side_slippage_bps(
    side: OrderSide,
    average_price: OrderPrice,
    benchmark: OrderPrice,
) -> i32 {
    if average_price.0 <= 0 || benchmark.0 <= 0 {
        return 0;
    }
    let raw = match side {
        OrderSide::Buy => i128::from(average_price.0) - i128::from(benchmark.0),
        OrderSide::Sell => i128::from(benchmark.0) - i128::from(average_price.0),
    };
    let bps = (raw * 10_000) / i128::from(benchmark.0);
    i32::try_from(bps.clamp(i128::from(i32::MIN), i128::from(i32::MAX))).unwrap_or(0)
}

pub(crate) fn optional_slippage_bps(
    side: OrderSide,
    average_price: OrderPrice,
    benchmark: OrderPrice,
) -> i32 {
    if benchmark.0 <= 0 {
        0
    } else {
        side_slippage_bps(side, average_price, benchmark)
    }
}

pub(crate) fn average_latency_ns(total_latency_ns: u128, samples: u64) -> u64 {
    if samples == 0 {
        return 0;
    }
    u64::try_from(total_latency_ns / u128::from(samples)).unwrap_or(u64::MAX)
}

pub(crate) fn i64_to_u128(value: i64) -> u128 {
    u128::try_from(value.max(0)).unwrap_or(0)
}

pub(crate) fn stronger_risk_outcome(
    current: AlgoRiskOutcome,
    kind: AlgoRiskViolationKind,
) -> AlgoRiskOutcome {
    if matches!(
        kind,
        AlgoRiskViolationKind::KillSwitchActive | AlgoRiskViolationKind::OperatorPaused
    ) {
        return AlgoRiskOutcome::KillSwitch;
    }
    if matches!(current, AlgoRiskOutcome::KillSwitch) {
        AlgoRiskOutcome::KillSwitch
    } else {
        AlgoRiskOutcome::Block
    }
}

pub(crate) fn scale_qty_inverse_bps(quantity: i64, bps: u32) -> i64 {
    if bps == 0 {
        return 0;
    }
    let scaled = (i128::from(quantity) * 10_000) / i128::from(bps);
    i64::try_from(scaled).unwrap_or(i64::MAX)
}

pub(crate) fn spread_edge_bps(
    buy_price: OrderPrice,
    sell_price: OrderPrice,
    ratio_bps: u32,
) -> i32 {
    if buy_price.0 <= 0 || sell_price.0 <= 0 || ratio_bps == 0 {
        return i32::MIN;
    }
    let scaled_sell = (i128::from(sell_price.0) * i128::from(ratio_bps)) / 10_000;
    let edge = ((scaled_sell - i128::from(buy_price.0)) * 10_000) / i128::from(buy_price.0);
    i32::try_from(edge.clamp(i128::from(i32::MIN), i128::from(i32::MAX))).unwrap_or(0)
}

pub(crate) fn passive_candidate_qty(
    parent: &ParentOrder,
    progress: AlgoProgress,
    now_ns: u64,
) -> OrderQty {
    let leaves = parent
        .total_qty()
        .0
        .saturating_sub(progress.released_qty().0);
    if leaves <= 0 {
        return OrderQty(0);
    }
    let mut qty = leaves.min(parent.max_clip().0);
    let final_slice = progress.released_qty().0.saturating_add(qty) >= parent.total_qty().0
        || now_ns >= parent.end_ns();
    if qty < parent.min_clip().0 && final_slice {
        qty = leaves;
    }
    OrderQty(qty.min(leaves))
}

pub(crate) fn queue_fill_probability_bps(
    queue_ahead_qty: i64,
    child_qty: i64,
    expected_take_qty: i64,
) -> u16 {
    if child_qty <= 0 || expected_take_qty <= 0 {
        return 0;
    }
    let required = queue_ahead_qty.max(0).saturating_add(child_qty);
    if required <= 0 {
        return 10_000;
    }
    let bps = (i128::from(expected_take_qty) * 10_000) / i128::from(required);
    u16::try_from(bps.clamp(0, 10_000)).unwrap_or(10_000)
}

pub(crate) fn midpoint_price(
    side: OrderSide,
    context: PassiveQueueContext,
    tick_size: OrderPrice,
) -> OrderPrice {
    let bid = context.best_bid().0;
    let ask = context.best_ask().0;
    let mid = bid.saturating_add(ask.saturating_sub(bid) / 2);
    match side {
        OrderSide::Buy => {
            let ticks_from_bid = mid.saturating_sub(bid) / tick_size.0;
            OrderPrice(bid.saturating_add(ticks_from_bid.saturating_mul(tick_size.0)))
        }
        OrderSide::Sell => {
            let ticks_from_ask = ask.saturating_sub(mid) / tick_size.0;
            OrderPrice(ask.saturating_sub(ticks_from_ask.saturating_mul(tick_size.0)))
        }
    }
}

pub(crate) fn route_liquidity_bps(available_qty: i64, target_qty: i64) -> u16 {
    if available_qty <= 0 || target_qty <= 0 {
        return 0;
    }
    let bps = (i128::from(available_qty) * 10_000) / i128::from(target_qty);
    u16::try_from(bps.clamp(0, 10_000)).unwrap_or(10_000)
}

pub(crate) fn route_price_penalty_bps(
    side: OrderSide,
    price: OrderPrice,
    best_price: OrderPrice,
) -> u16 {
    if price.0 <= 0 || best_price.0 <= 0 {
        return 10_000;
    }
    let penalty_ticks = match side {
        OrderSide::Buy => price.0.saturating_sub(best_price.0),
        OrderSide::Sell => best_price.0.saturating_sub(price.0),
    };
    if penalty_ticks <= 0 {
        return 0;
    }
    let bps = (i128::from(penalty_ticks) * 10_000) / i128::from(best_price.0);
    u16::try_from(bps.clamp(0, 10_000)).unwrap_or(10_000)
}

pub(crate) fn best_sor_price(
    side: OrderSide,
    candidates: &[SorRouteCandidate],
) -> Option<OrderPrice> {
    let mut best: Option<OrderPrice> = None;
    for candidate in candidates {
        if !candidate.status().is_routable()
            || candidate.price().0 <= 0
            || candidate.available_qty().0 <= 0
        {
            continue;
        }
        best = Some(match (side, best) {
            (_, None) => candidate.price(),
            (OrderSide::Buy, Some(current)) => OrderPrice(current.0.min(candidate.price().0)),
            (OrderSide::Sell, Some(current)) => OrderPrice(current.0.max(candidate.price().0)),
        });
    }
    best
}

pub(crate) fn decision_has_route<const N: usize>(
    decision: &SorDecision<N>,
    route_id: RouteId,
) -> bool {
    decision
        .allocations()
        .any(|allocation| allocation.plan().request().route_id == route_id)
}

pub(crate) fn liquidity_decision_has_route<const N: usize>(
    decision: &LiquiditySeekingDecision<N>,
    route_id: RouteId,
) -> bool {
    decision
        .allocations()
        .any(|allocation| allocation.plan().request().route_id == route_id)
}

pub(crate) fn sweep_decision_has_level<const N: usize>(
    decision: &SweepDecision<N>,
    route_id: RouteId,
    price: OrderPrice,
) -> bool {
    decision.allocations().any(|allocation| {
        allocation.plan().request().route_id == route_id
            && allocation.plan().request().limit_price == price
    })
}

pub(crate) fn price_inside_collar(side: OrderSide, price: OrderPrice, collar: OrderPrice) -> bool {
    match side {
        OrderSide::Buy => price.0 <= collar.0,
        OrderSide::Sell => price.0 >= collar.0,
    }
}

pub(crate) fn best_liquidity_price(
    side: OrderSide,
    candidates: &[LiquiditySeekingCandidate],
) -> Option<OrderPrice> {
    let mut best: Option<OrderPrice> = None;
    for candidate in candidates {
        let route = candidate.route();
        if !route.status().is_routable() || route.price().0 <= 0 || route.available_qty().0 <= 0 {
            continue;
        }
        best = Some(match (side, best) {
            (_, None) => route.price(),
            (OrderSide::Buy, Some(current)) => OrderPrice(current.0.min(route.price().0)),
            (OrderSide::Sell, Some(current)) => OrderPrice(current.0.max(route.price().0)),
        });
    }
    best
}

pub(crate) fn signed_inventory_ratio_bps(inventory_qty: i64, max_inventory_qty: i64) -> i32 {
    if max_inventory_qty <= 0 {
        return 0;
    }
    let ratio = (i128::from(inventory_qty) * 10_000) / i128::from(max_inventory_qty);
    i32::try_from(ratio.clamp(-10_000, 10_000)).unwrap_or(0)
}

pub(crate) fn scale_signed_bps(value_bps: i32, weight_bps: i32) -> i32 {
    let scaled = (i64::from(value_bps) * i64::from(weight_bps)) / 10_000;
    i32::try_from(scaled.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(0)
}

pub(crate) fn apply_price_bps(price: OrderPrice, adjustment_bps: i32) -> OrderPrice {
    let adjustment = (i128::from(price.0) * i128::from(adjustment_bps)) / 10_000;
    let adjusted = i128::from(price.0).saturating_add(adjustment);
    OrderPrice(i64::try_from(adjusted.max(1)).unwrap_or(i64::MAX))
}

pub(crate) fn price_bps_to_ticks(price: OrderPrice, bps: u16) -> i64 {
    let value = (i128::from(price.0) * i128::from(bps)) / 10_000;
    i64::try_from(value.max(1)).unwrap_or(i64::MAX)
}

pub(crate) fn snap_down_to_tick(price: i64, tick_size: i64) -> i64 {
    if tick_size <= 0 {
        return price;
    }
    price - price.rem_euclid(tick_size)
}

pub(crate) fn snap_up_to_tick(price: i64, tick_size: i64) -> i64 {
    if tick_size <= 0 {
        return price;
    }
    let rem = price.rem_euclid(tick_size);
    if rem == 0 {
        price
    } else {
        price.saturating_add(tick_size.saturating_sub(rem))
    }
}

pub(crate) fn adverse_move_bps(side: OrderSide, arrival: OrderPrice, reference: OrderPrice) -> u16 {
    let adverse_ticks = match side {
        OrderSide::Buy => reference.0.saturating_sub(arrival.0),
        OrderSide::Sell => arrival.0.saturating_sub(reference.0),
    };
    if adverse_ticks <= 0 || arrival.0 <= 0 {
        return 0;
    }
    let bps = (i128::from(adverse_ticks) * 10_000) / i128::from(arrival.0);
    u16::try_from(bps.min(i128::from(u16::MAX))).unwrap_or(u16::MAX)
}

pub(crate) fn fixed_id_with_index<const N: usize>(
    prefix: &str,
    index: u64,
) -> Result<FixedAscii<N>, AlgoError> {
    let mut bytes = [0_u8; N];
    let prefix_bytes = prefix.as_bytes();
    if prefix_bytes.len().saturating_add(1) > N {
        return Err(AlgoError::GeneratedIdentifierTooLong);
    }
    bytes[..prefix_bytes.len()].copy_from_slice(prefix_bytes);
    let mut len = prefix_bytes.len();
    bytes[len] = b'-';
    len += 1;

    let mut digits = [0_u8; 20];
    let mut value = index;
    let mut digit_len = 0_usize;
    loop {
        digits[digit_len] = b'0' + u8::try_from(value % 10).unwrap_or(0);
        digit_len += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    if len.saturating_add(digit_len) > N {
        return Err(AlgoError::GeneratedIdentifierTooLong);
    }
    for digit in digits[..digit_len].iter().rev() {
        bytes[len] = *digit;
        len += 1;
    }
    let value =
        std::str::from_utf8(&bytes[..len]).expect("prefix is ASCII and generated digits are ASCII");
    Ok(FixedAscii::new(value)?)
}

pub(crate) fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

pub(crate) fn hash_u64(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub(crate) fn hash_i64(hash: u64, value: i64) -> u64 {
    hash_u64(hash, value as u64)
}

pub(crate) fn hash_replay_step<const N: usize>(
    mut hash: u64,
    input_sequence: u64,
    event: AlgoReplayEvent,
    progress: AlgoProgress,
    decision: &AlgoDecision<N>,
) -> u64 {
    hash = hash_u64(hash, input_sequence);
    hash = match event {
        AlgoReplayEvent::Timer { timestamp_ns } => hash_u64(hash_u64(hash, 1), timestamp_ns),
        AlgoReplayEvent::Execution(event) => hash_i64(
            hash_u64(hash_u64(hash, 2), u64::from(event.order_status as u8)),
            event.last_qty.0,
        ),
        AlgoReplayEvent::ParentStatus { status } => hash_u64(hash_u64(hash, 3), status as u64),
    };
    hash = hash_i64(hash, progress.released_qty().0);
    hash = hash_i64(hash, progress.completed_qty().0);
    hash = hash_i64(hash, progress.open_qty().0);
    hash_u64(hash, usize_to_u64(decision.len()))
}
