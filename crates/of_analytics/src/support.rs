use super::*;

pub(crate) fn validate_levels(levels: &[BookLevel]) -> Result<(), AnalyticsError> {
    for level in levels {
        if level.price <= 0 || level.size < 0 {
            return Err(AnalyticsError::InvalidDepth);
        }
    }
    Ok(())
}

pub(crate) fn depth_sum(levels: &[BookLevel], max_levels: usize) -> i64 {
    levels
        .iter()
        .take(max_levels)
        .fold(0_i64, |sum, level| sum.saturating_add(level.size))
}

pub(crate) fn sweep_qty(levels: &[BookLevel], target_qty: i64) -> i64 {
    let mut remaining = target_qty;
    let mut swept = 0_i64;
    for level in levels {
        if remaining <= 0 {
            break;
        }
        let take = level.size.min(remaining);
        swept = swept.saturating_add(take);
        remaining = remaining.saturating_sub(take);
    }
    swept
}

pub(crate) fn depth_slope_bps(bids: &[BookLevel], asks: &[BookLevel], max_levels: usize) -> i32 {
    let used = max_levels.min(bids.len()).min(asks.len());
    if used <= 1 {
        return 0;
    }
    let top = bids[0].size.saturating_add(asks[0].size);
    let outer = bids[used - 1].size.saturating_add(asks[used - 1].size);
    if top <= 0 {
        return 0;
    }
    i32::try_from((i128::from(outer.saturating_sub(top)) * 10_000) / i128::from(top)).unwrap_or(0)
}

pub(crate) fn depth_convexity_bps(
    bids: &[BookLevel],
    asks: &[BookLevel],
    max_levels: usize,
) -> i32 {
    let used = max_levels.min(bids.len()).min(asks.len());
    if used < 3 {
        return 0;
    }
    let first = bids[0].size.saturating_add(asks[0].size);
    let mid_index = used / 2;
    let mid = bids[mid_index].size.saturating_add(asks[mid_index].size);
    let outer = bids[used - 1].size.saturating_add(asks[used - 1].size);
    if mid <= 0 {
        return 0;
    }
    let first_slope = mid.saturating_sub(first);
    let second_slope = outer.saturating_sub(mid);
    i32::try_from((i128::from(second_slope.saturating_sub(first_slope)) * 10_000) / i128::from(mid))
        .unwrap_or(0)
}

pub(crate) fn book_pressure_bps(bids: &[BookLevel], asks: &[BookLevel], max_levels: usize) -> i32 {
    let used = max_levels.min(bids.len()).min(asks.len());
    if used == 0 {
        return 0;
    }
    let mut signed = 0_i128;
    let mut total = 0_i128;
    for index in 0..used {
        let weight = i128::try_from(used.saturating_sub(index)).unwrap_or(i128::MAX);
        let bid = i128::from(bids[index].size);
        let ask = i128::from(asks[index].size);
        signed = signed.saturating_add(weight.saturating_mul(bid.saturating_sub(ask)));
        total = total.saturating_add(weight.saturating_mul(bid.saturating_add(ask)));
    }
    if total <= 0 {
        return 0;
    }
    i32::try_from((signed * 10_000) / total).unwrap_or(0)
}

pub(crate) fn sweepability_bps(sweepable_qty: i64, target_qty: i64) -> u16 {
    if target_qty <= 0 {
        return 10_000;
    }
    u16::try_from(((i128::from(sweepable_qty) * 10_000) / i128::from(target_qty)).min(10_000))
        .unwrap_or(10_000)
}

pub(crate) fn qty_rate_per_sec(qty: i64, elapsed_ns: u64) -> i64 {
    if elapsed_ns == 0 {
        return 0;
    }
    i64::try_from((i128::from(qty) * 1_000_000_000_i128) / i128::from(elapsed_ns))
        .unwrap_or(i64::MAX)
}

pub(crate) fn ratio_bps_i64(value: i64, total: i64) -> u16 {
    if value <= 0 || total <= 0 {
        return 0;
    }
    u16::try_from(((i128::from(value) * 10_000) / i128::from(total)).min(10_000)).unwrap_or(10_000)
}

pub(crate) fn coefficient_impact_bps(participation_bps: u16, coefficient_bps: u16) -> i32 {
    i32::try_from((u128::from(participation_bps) * u128::from(coefficient_bps)) / 10_000)
        .unwrap_or(i32::MAX)
}

pub(crate) fn decay_remaining_bps(horizon_ns: u64, half_life_ns: u64) -> u16 {
    if half_life_ns == 0 {
        return 0;
    }
    let denominator = u128::from(half_life_ns).saturating_add(u128::from(horizon_ns));
    u16::try_from((u128::from(half_life_ns) * 10_000) / denominator).unwrap_or(0)
}

pub(crate) fn integer_sqrt_u128(value: u128) -> u128 {
    if value <= 1 {
        return value;
    }
    let mut low = 1_u128;
    let mut high = value;
    let mut answer = 1_u128;
    while low <= high {
        let mid = low + ((high - low) / 2);
        let square = mid.saturating_mul(mid);
        if square <= value {
            answer = mid;
            low = mid.saturating_add(1);
        } else {
            high = mid.saturating_sub(1);
        }
    }
    answer
}

pub(crate) fn bps_to_price_delta(price: i64, bps: i32) -> i64 {
    if price <= 0 || bps <= 0 {
        return 0;
    }
    i64::try_from((i128::from(price) * i128::from(bps)) / 10_000).unwrap_or(i64::MAX)
}

pub(crate) fn side_aware_price_move_bps(
    side: Side,
    reference_price: i64,
    observed_price: i64,
) -> i32 {
    let distance = match side {
        Side::Ask => observed_price.saturating_sub(reference_price),
        Side::Bid => reference_price.saturating_sub(observed_price),
    };
    price_to_bps(distance, reference_price)
}

pub(crate) fn quote_fade_bps(input: ToxicityInput) -> i32 {
    let (pre_qty, post_qty) = match input.trade().aggressor_side() {
        Side::Ask => (input.pre_ask_qty(), input.post_ask_qty()),
        Side::Bid => (input.pre_bid_qty(), input.post_bid_qty()),
    };
    if pre_qty <= 0 {
        return 0;
    }
    i32::try_from((i128::from(pre_qty.saturating_sub(post_qty)) * 10_000) / i128::from(pre_qty))
        .unwrap_or(0)
}

pub(crate) fn positive_bps(value: i32) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(0)
}

pub(crate) fn average_bps4(a: u16, b: u16, c: u16, d: u16) -> u16 {
    u16::try_from(
        (u32::from(a)
            .saturating_add(u32::from(b))
            .saturating_add(u32::from(c))
            .saturating_add(u32::from(d))
            / 4)
        .min(10_000),
    )
    .unwrap_or(10_000)
}

pub(crate) fn return_bps(to_price: i64, from_price: i64) -> i32 {
    if from_price <= 0 {
        return 0;
    }
    i32::try_from(
        ((i128::from(to_price) - i128::from(from_price)) * 10_000) / i128::from(from_price),
    )
    .unwrap_or(0)
}

pub(crate) fn abs_return_bps(to_price: i64, from_price: i64) -> u32 {
    return_bps(to_price, from_price).unsigned_abs()
}

pub(crate) fn volatility_and_noise(returns_bps: &[i32]) -> (u32, u16) {
    let mut sum_sq = 0_u128;
    let mut sign_flips = 0_u32;
    let mut prev_sign = 0_i32;
    for ret in returns_bps {
        let abs = ret.unsigned_abs();
        sum_sq = sum_sq.saturating_add(u128::from(abs).saturating_mul(u128::from(abs)));
        let sign = ret.signum();
        if prev_sign != 0 && sign != 0 && sign != prev_sign {
            sign_flips = sign_flips.saturating_add(1);
        }
        if sign != 0 {
            prev_sign = sign;
        }
    }
    let len = u128::try_from(returns_bps.len()).unwrap_or(1);
    let realized = u32::try_from(isqrt_u128(sum_sq / len)).unwrap_or(u32::MAX);
    let noise = if returns_bps.len() <= 1 {
        0
    } else {
        u16::try_from(
            (u128::from(sign_flips) * 10_000) / u128::try_from(returns_bps.len() - 1).unwrap_or(1),
        )
        .unwrap_or(10_000)
    };
    (realized, noise)
}

pub(crate) fn transition_confidence_bps(
    input: CompositeRegimeInput,
    config: CompositeRegimeConfig,
    trend: TrendRegimeKind,
    liquidity: LiquidityRegimeKind,
    spread: SpreadRegimeKind,
    session: SessionRegimeKind,
) -> u16 {
    let trend_margin = match trend {
        TrendRegimeKind::Trend => margin_bps_u32(
            u32::from(input.trend_strength_bps()),
            u32::from(config.trend_threshold_bps()),
        ),
        TrendRegimeKind::Chop => margin_bps_u32(
            u32::from(input.chop_score_bps()),
            u32::from(config.chop_threshold_bps()),
        ),
        TrendRegimeKind::Range => {
            let trend_gap = u32::from(
                config
                    .trend_threshold_bps()
                    .saturating_sub(input.trend_strength_bps()),
            );
            let chop_gap = u32::from(
                config
                    .chop_threshold_bps()
                    .saturating_sub(input.chop_score_bps()),
            );
            u16::try_from(trend_gap.min(chop_gap).min(10_000)).unwrap_or(10_000)
        }
    };
    let liquidity_margin = match liquidity {
        LiquidityRegimeKind::Hidden => margin_bps_u32(
            u32::from(input.hidden_liquidity_proxy_bps()),
            u32::from(config.hidden_liquidity_threshold_bps()),
        ),
        LiquidityRegimeKind::Thin => {
            i64_distance_to_bps(config.thin_depth(), input.displayed_depth())
        }
        LiquidityRegimeKind::Deep => {
            i64_distance_to_bps(input.displayed_depth(), config.deep_depth())
        }
        LiquidityRegimeKind::Normal => {
            let above_thin = input.displayed_depth().saturating_sub(config.thin_depth());
            let below_deep = config.deep_depth().saturating_sub(input.displayed_depth());
            i64_distance_to_bps(above_thin.min(below_deep), config.deep_depth().max(1))
        }
    };
    let spread_margin = match spread {
        SpreadRegimeKind::Wide => margin_bps_u32(input.spread_bps(), config.wide_spread_bps()),
        SpreadRegimeKind::Tight => margin_bps_u32(config.tight_spread_bps(), input.spread_bps()),
        SpreadRegimeKind::Normal => u16::try_from(
            input
                .spread_bps()
                .saturating_sub(config.tight_spread_bps())
                .min(config.wide_spread_bps().saturating_sub(input.spread_bps()))
                .min(10_000),
        )
        .unwrap_or(10_000),
    };
    let session_margin = match session {
        SessionRegimeKind::NewsShock => margin_bps_u32(
            u32::from(input.news_intensity_bps()),
            u32::from(config.news_shock_threshold_bps()),
        ),
        SessionRegimeKind::Auction => 10_000,
        SessionRegimeKind::Open => {
            time_margin_bps(config.open_window_ns(), input.elapsed_since_open_ns())
        }
        SessionRegimeKind::Close => {
            time_margin_bps(config.close_window_ns(), input.remaining_to_close_ns())
        }
        SessionRegimeKind::Continuous => {
            let from_open = input
                .elapsed_since_open_ns()
                .saturating_sub(config.open_window_ns());
            let from_close = input
                .remaining_to_close_ns()
                .saturating_sub(config.close_window_ns());
            u16::try_from((from_open.min(from_close) / 1_000_000_000).min(10_000)).unwrap_or(10_000)
        }
    };
    trend_margin
        .min(liquidity_margin)
        .min(spread_margin)
        .min(session_margin)
}

pub(crate) fn margin_bps_u32(value: u32, threshold: u32) -> u16 {
    if threshold == 0 {
        return 10_000;
    }
    u16::try_from(
        ((u128::from(value.saturating_sub(threshold)) * 10_000) / u128::from(threshold))
            .min(10_000),
    )
    .unwrap_or(10_000)
}

pub(crate) fn i64_distance_to_bps(distance: i64, reference: i64) -> u16 {
    if reference <= 0 {
        return 10_000;
    }
    u16::try_from(((i128::from(distance.max(0)) * 10_000) / i128::from(reference)).min(10_000))
        .unwrap_or(10_000)
}

pub(crate) fn time_margin_bps(window_ns: u64, value_ns: u64) -> u16 {
    if window_ns == 0 {
        return 10_000;
    }
    u16::try_from(
        ((u128::from(window_ns.saturating_sub(value_ns)) * 10_000) / u128::from(window_ns))
            .min(10_000),
    )
    .unwrap_or(10_000)
}

pub(crate) fn price_to_bps(value: i64, reference: i64) -> i32 {
    if reference <= 0 {
        return 0;
    }
    let bps = (i128::from(value) * 10_000) / i128::from(reference);
    i32::try_from(bps.clamp(i128::from(i32::MIN), i128::from(i32::MAX))).unwrap_or(0)
}

pub(crate) fn side_aware_slippage_bps(trade: TradeContext, benchmark_price: i64) -> i32 {
    let distance = match trade.aggressor_side() {
        Side::Ask => trade.price().saturating_sub(benchmark_price),
        Side::Bid => benchmark_price.saturating_sub(trade.price()),
    };
    price_to_bps(distance, benchmark_price)
}

pub(crate) fn rate_bps(count: u64, total: u64) -> u16 {
    if total == 0 {
        return 0;
    }
    u16::try_from(((u128::from(count) * 10_000) / u128::from(total)).min(10_000)).unwrap_or(10_000)
}

pub(crate) fn max_u16(values: &[u16]) -> u16 {
    values.iter().copied().max().unwrap_or(0)
}

pub(crate) fn primary_feed_quality_issue(
    sequence_gap_rate: u16,
    out_of_order_rate: u16,
    duplicate_rate: u16,
    stale_rate: u16,
    bad_book_rate: u16,
    timestamp_skew_rate: u16,
    sequence_reset_rate: u16,
) -> FeedQualityFlags {
    let weighted = [
        (
            u32::from(sequence_gap_rate).saturating_mul(10),
            FeedQualityFlags::SEQUENCE_GAP,
        ),
        (
            u32::from(out_of_order_rate).saturating_mul(15),
            FeedQualityFlags::OUT_OF_ORDER,
        ),
        (
            u32::from(duplicate_rate).saturating_mul(5),
            FeedQualityFlags::DUPLICATE,
        ),
        (
            u32::from(stale_rate).saturating_mul(8),
            FeedQualityFlags::STALE,
        ),
        (
            u32::from(bad_book_rate).saturating_mul(20),
            FeedQualityFlags::CROSSED_BOOK,
        ),
        (
            u32::from(timestamp_skew_rate).saturating_mul(10),
            FeedQualityFlags::TIMESTAMP_SKEW,
        ),
        (
            u32::from(sequence_reset_rate).saturating_mul(15),
            FeedQualityFlags::SEQUENCE_RESET,
        ),
    ];
    let mut primary = FeedQualityFlags::OK;
    let mut primary_score = 0_u32;
    for (score, flag) in weighted {
        if score > primary_score {
            primary_score = score;
            primary = flag;
        }
    }
    primary
}

pub(crate) fn rate_scaled(count: u64, total: u64) -> u32 {
    if total == 0 {
        return 0;
    }
    u32::try_from(((u128::from(count) * 10_000) / u128::from(total)).min(u128::from(u32::MAX)))
        .unwrap_or(u32::MAX)
}

pub(crate) fn score_ratio(value: u32, threshold: u32) -> u16 {
    if threshold == 0 {
        return 0;
    }
    u16::try_from(((u128::from(value) * 10_000) / u128::from(threshold)).min(10_000))
        .unwrap_or(10_000)
}

pub(crate) fn score_ratio_u64(value: u64, threshold: u64) -> u16 {
    if threshold == 0 {
        return 0;
    }
    u16::try_from(((u128::from(value) * 10_000) / u128::from(threshold)).min(10_000))
        .unwrap_or(10_000)
}

pub(crate) fn score_ratio_i64(value: i64, threshold: i64) -> u16 {
    if threshold <= 0 {
        return 0;
    }
    u16::try_from(((value.max(0) as u128 * 10_000) / threshold as u128).min(10_000))
        .unwrap_or(10_000)
}

pub(crate) fn wait_penalty_bps(
    expected_time_to_fill_ns: u64,
    max_wait_ns: u64,
    urgency_bps: u16,
) -> u16 {
    if max_wait_ns == 0 || urgency_bps == 0 {
        return 0;
    }
    let wait_ratio = if expected_time_to_fill_ns == u64::MAX {
        10_000
    } else {
        u16::try_from(
            ((u128::from(expected_time_to_fill_ns) * 10_000) / u128::from(max_wait_ns)).min(10_000),
        )
        .unwrap_or(10_000)
    };
    u16::try_from((u32::from(wait_ratio) * u32::from(urgency_bps)) / 10_000).unwrap_or(10_000)
}

pub(crate) fn queue_priority_loss_bps(replace_queue_loss_qty: i64, total_queue_qty: i64) -> u16 {
    if replace_queue_loss_qty <= 0 || total_queue_qty <= 0 {
        return 0;
    }
    u16::try_from(
        ((i128::from(replace_queue_loss_qty) * 10_000) / i128::from(total_queue_qty)).min(10_000),
    )
    .unwrap_or(10_000)
}

pub(crate) fn maker_taker_score_bps(
    passive_edge_bps: i32,
    aggressive_cost_bps: u32,
    fill_probability_bps: u16,
    top_level_survival_bps: u16,
    urgency_bps: u16,
) -> u16 {
    let delta = passive_edge_bps
        .saturating_sub(i32::try_from(aggressive_cost_bps).unwrap_or(i32::MAX))
        .saturating_add((i32::from(fill_probability_bps) - 5_000) / 2)
        .saturating_add((i32::from(top_level_survival_bps) - 5_000) / 2)
        .saturating_sub(i32::from(urgency_bps) / 2);
    u16::try_from(5_000_i32.saturating_add(delta).clamp(0, 10_000)).unwrap_or(0)
}

pub(crate) fn latency_quality_bps(latency_ns: u64, max_latency_ns: u64) -> u16 {
    if max_latency_ns == 0 {
        return 0;
    }
    if latency_ns == 0 {
        return 10_000;
    }
    let used =
        u16::try_from(((u128::from(latency_ns) * 10_000) / u128::from(max_latency_ns)).min(10_000))
            .unwrap_or(10_000);
    10_000_u16.saturating_sub(used)
}

pub(crate) fn threshold_penalty_bps(value_bps: u16, max_bps: u16) -> u16 {
    if value_bps == 0 {
        return 0;
    }
    if max_bps == 0 {
        return 10_000;
    }
    score_ratio(u32::from(value_bps), u32::from(max_bps))
}

pub(crate) fn route_reliability_bps(
    snapshot: VenueRouteSnapshot,
    config: VenueRouteQualityConfig,
) -> u16 {
    let fill_component = score_ratio(
        u32::from(snapshot.fill_rate_bps()),
        u32::from(config.target_fill_rate_bps()),
    );
    let reject_penalty =
        threshold_penalty_bps(snapshot.reject_rate_bps(), config.max_reject_rate_bps());
    let cancel_penalty =
        threshold_penalty_bps(snapshot.cancel_rate_bps(), config.max_cancel_rate_bps());
    fill_component
        .saturating_sub(reject_penalty / 2)
        .saturating_sub(cancel_penalty / 4)
}

pub(crate) fn abs_diff_u64(left: u64, right: u64) -> u64 {
    left.abs_diff(right)
}

pub(crate) fn bounded_abs_bps(value: i32) -> u16 {
    u16::try_from(value.unsigned_abs().min(10_000)).unwrap_or(10_000)
}

pub(crate) fn signed_scaled_bps(value_bps: i32, scale_bps: u16) -> i32 {
    i32::try_from((i128::from(value_bps) * i128::from(scale_bps)) / 10_000).unwrap_or_else(|_| {
        if value_bps.is_negative() {
            i32::MIN
        } else {
            i32::MAX
        }
    })
}

pub(crate) fn bounded_u32_bps(value: u32) -> u16 {
    u16::try_from(value.min(10_000)).unwrap_or(10_000)
}

pub(crate) fn signed_u32_diff_to_i32(left: u32, right: u32) -> i32 {
    i32::try_from(i128::from(left) - i128::from(right)).unwrap_or({
        if left < right {
            i32::MIN
        } else {
            i32::MAX
        }
    })
}

pub(crate) fn signed_i128_ratio_to_i32(numerator: i128, denominator: i128) -> i32 {
    if denominator <= 0 {
        return 0;
    }
    let scaled = (numerator.saturating_mul(10_000)) / denominator;
    i32::try_from(scaled.clamp(i128::from(i32::MIN), i128::from(i32::MAX))).unwrap_or(0)
}

pub(crate) fn ratio_i64_to_u32(numerator: i64, denominator: i64) -> u32 {
    if denominator <= 0 {
        return 0;
    }
    u32::try_from(
        ((numerator.max(0) as u128 * 10_000) / denominator.max(1) as u128)
            .min(u128::from(u32::MAX)),
    )
    .unwrap_or(u32::MAX)
}

pub(crate) fn ratio_i128_to_u32(numerator: i128, denominator: i128) -> u32 {
    if denominator <= 0 {
        return 0;
    }
    u32::try_from(
        ((numerator.max(0) as u128 * 10_000) / denominator.max(1) as u128)
            .min(u128::from(u32::MAX)),
    )
    .unwrap_or(u32::MAX)
}

pub(crate) fn signed_pressure_bps(left: i64, right: i64) -> i32 {
    let total = left.saturating_add(right);
    if total <= 0 {
        return 0;
    }
    i32::try_from((i128::from(left.saturating_sub(right)) * 10_000) / i128::from(total))
        .unwrap_or(0)
}

pub(crate) fn average_score(scores: &[u16]) -> u16 {
    if scores.is_empty() {
        return 0;
    }
    let sum = scores
        .iter()
        .fold(0_u32, |acc, score| acc.saturating_add(u32::from(*score)));
    u16::try_from(sum / u32::try_from(scores.len()).unwrap_or(1)).unwrap_or(10_000)
}

pub(crate) fn avg_u128(sum: u128, count: u64) -> u64 {
    if count == 0 {
        return 0;
    }
    u64::try_from(sum / u128::from(count)).unwrap_or(u64::MAX)
}

pub(crate) fn fnv1a64(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub(crate) fn hash_feature_definition(mut hash: u64, definition: FeatureDefinition) -> u64 {
    hash = fnv1a64(hash, &definition.id().raw().to_le_bytes());
    hash = fnv1a64(hash, definition.name().as_bytes());
    hash = fnv1a64(hash, &[definition.unit().code()]);
    hash = fnv1a64(hash, &definition.scale().to_le_bytes());
    hash = fnv1a64(hash, &[definition.missing_policy().code()]);
    fnv1a64(
        hash,
        &definition.missing_policy().fill_value().to_le_bytes(),
    )
}

pub(crate) fn isqrt_u128(value: u128) -> u128 {
    if value <= 1 {
        return value;
    }
    let mut x0 = value / 2;
    let mut x1 = (x0 + value / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + value / x0) / 2;
    }
    x0
}
