use std::ffi::{c_char, c_void, CStr};
use std::ptr;
use std::sync::atomic::Ordering;

use of_adapters::RawEvent;
use of_core::{
    ACDSnapshot, AgentTypeSnapshot, AlmgrenChrissSnapshot, BookAction, BookAnalyticsSnapshot,
    BookEventAnalyticsSnapshot, BookSnapshot, CvdEnhancementSnapshot, DarkLitCorrelationSnapshot,
    DarkPoolSnapshot, DerivedAnalyticsSnapshot, FuturesSnapshot, HasbrouckSnapshot,
    InstitutionalFlowSnapshot, IntervalCandleSnapshot, KineticEnergySnapshot, KyleLambdaSnapshot,
    LOBFeatureSnapshot, NoiseSnapshot, OIAnalysisSnapshot, OptionsFlowSnapshot, PatternSnapshot,
    RegimeSnapshot, ResiliencySnapshot, SessionCandleSnapshot, Side, SignalState,
    SpreadDecompositionSnapshot, SymbolId, VolatilitySignatureSnapshot, VolatilitySnapshot, VpinSnapshot,
};
#[cfg(feature = "tickbar")]
use of_core::CompletedBar;

use crate::{of_engine, of_error_t, of_event_t, of_symbol_t};

pub(crate) fn cstr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let s = unsafe { CStr::from_ptr(ptr) };
    s.to_str().ok().map(|v| v.to_string())
}

pub(crate) fn non_empty_string(ptr: *const c_char) -> Option<String> {
    let v = cstr_to_string(ptr)?;
    if v.trim().is_empty() {
        None
    } else {
        Some(v)
    }
}

pub(crate) fn parse_csv(ptr: *const c_char) -> Option<Vec<String>> {
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

pub(crate) fn symbol_from_ffi(sym: *const of_symbol_t) -> Result<(SymbolId, u16), of_error_t> {
    if sym.is_null() {
        return Err(of_error_t::OF_ERR_INVALID_ARG);
    }
    symbol_from_ffi_ref(unsafe { &*sym })
}

pub(crate) fn symbol_from_ffi_ref(sym: &of_symbol_t) -> Result<(SymbolId, u16), of_error_t> {
    let venue = cstr_to_string(sym.venue).ok_or(of_error_t::OF_ERR_INVALID_ARG)?;
    let symbol = cstr_to_string(sym.symbol).ok_or(of_error_t::OF_ERR_INVALID_ARG)?;
    Ok((SymbolId { venue, symbol }, sym.depth_levels))
}

pub(crate) fn side_from_ffi(value: u32) -> Result<Side, of_error_t> {
    match value {
        0 => Ok(Side::Bid),
        1 => Ok(Side::Ask),
        _ => Err(of_error_t::OF_ERR_INVALID_ARG),
    }
}

pub(crate) fn action_from_ffi(value: u32) -> Result<BookAction, of_error_t> {
    match value {
        0 => Ok(BookAction::Upsert),
        1 => Ok(BookAction::Delete),
        _ => Err(of_error_t::OF_ERR_INVALID_ARG),
    }
}

pub(crate) fn write_json_to_c_buffer(
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

pub(crate) fn dispatch_callbacks(engine: &mut of_engine, quality_flags: u32) {
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

        if sub.kind == 6 {
            let mut latest_ts_exchange_ns = 0;
            let mut latest_ts_recv_ns = 0;
            let mut saw_book_update = false;
            for event in engine.inner.last_events() {
                let RawEvent::Book(book) = event else {
                    continue;
                };
                if book.symbol != sub.symbol {
                    continue;
                }
                saw_book_update = true;
                latest_ts_exchange_ns = book.ts_exchange_ns;
                latest_ts_recv_ns = book.ts_recv_ns;
            }
            if !saw_book_update {
                continue;
            }

            let payload = match engine.inner.book_snapshot(&sub.symbol) {
                Some(snapshot) => format_book_snapshot(&snapshot),
                None => "{}".to_string(),
            };
            let event = of_event_t {
                ts_exchange_ns: latest_ts_exchange_ns,
                ts_recv_ns: latest_ts_recv_ns,
                kind: sub.kind,
                payload: payload.as_ptr() as *const c_void,
                payload_len: payload.len() as u32,
                schema_id: 1,
                quality_flags,
            };
            (sub.cb)(&event as *const of_event_t, sub.user_data);
            continue;
        }

        if sub.kind == 7 {
            let mut latest_ts_exchange_ns = 0;
            let mut latest_ts_recv_ns = 0;
            let mut saw_trade_update = false;
            for event in engine.inner.last_events() {
                let RawEvent::Trade(trade) = event else {
                    continue;
                };
                if trade.symbol != sub.symbol {
                    continue;
                }
                saw_trade_update = true;
                latest_ts_exchange_ns = trade.ts_exchange_ns;
                latest_ts_recv_ns = trade.ts_recv_ns;
            }
            if !saw_trade_update {
                continue;
            }

            let payload = match engine.inner.derived_analytics_snapshot(&sub.symbol) {
                Some(snapshot) => format_derived_analytics_snapshot(&snapshot),
                None => "{}".to_string(),
            };
            let event = of_event_t {
                ts_exchange_ns: latest_ts_exchange_ns,
                ts_recv_ns: latest_ts_recv_ns,
                kind: sub.kind,
                payload: payload.as_ptr() as *const c_void,
                payload_len: payload.len() as u32,
                schema_id: 1,
                quality_flags,
            };
            (sub.cb)(&event as *const of_event_t, sub.user_data);
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
                    Some(s) => format_analytics_snapshot(&s),
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
                            escape_json(s.module_id),
                            state,
                            s.confidence_bps,
                            s.quality_flags,
                            escape_json(&s.reason)
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

pub(crate) fn dispatch_health_callbacks(engine: &mut of_engine, quality_flags: u32) {
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

pub(crate) fn format_trade_event(trade: &of_core::TradePrint) -> String {
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

pub(crate) fn format_book_snapshot(snapshot: &BookSnapshot) -> String {
    format!(
        "{{\"venue\":\"{}\",\"symbol\":\"{}\",\"bids\":[{}],\"asks\":[{}],\"last_sequence\":{},\"ts_exchange_ns\":{},\"ts_recv_ns\":{}}}",
        escape_json(&snapshot.symbol.venue),
        escape_json(&snapshot.symbol.symbol),
        format_book_levels(&snapshot.bids),
        format_book_levels(&snapshot.asks),
        snapshot.last_sequence,
        snapshot.ts_exchange_ns,
        snapshot.ts_recv_ns
    )
}

pub(crate) fn format_book_analytics_snapshot(snap: &BookAnalyticsSnapshot) -> String {
    format!(
        "{{\"best_bid\":{},\"best_ask\":{},\"quoted_spread\":{},\"relative_spread_bps\":{},\"microprice\":{},\"bid_depth\":{},\"ask_depth\":{},\"depth_imbalance_bps\":{}}}",
        snap.best_bid,
        snap.best_ask,
        snap.quoted_spread,
        snap.relative_spread_bps,
        snap.microprice,
        snap.bid_depth,
        snap.ask_depth,
        snap.depth_imbalance_bps
    )
}

pub(crate) fn format_book_event_analytics_snapshot(snap: &BookEventAnalyticsSnapshot) -> String {
    format!(
        "{{\"bid_arrival_rate\":{:.4},\"ask_arrival_rate\":{:.4},\"bid_cancel_rate\":{:.4},\"ask_cancel_rate\":{:.4},\"change_intensity\":{:.4},\"bid_event_volume\":{},\"ask_event_volume\":{}}}",
        snap.bid_arrival_rate,
        snap.ask_arrival_rate,
        snap.bid_cancel_rate,
        snap.ask_cancel_rate,
        snap.change_intensity,
        snap.bid_event_volume,
        snap.ask_event_volume,
    )
}

pub(crate) fn format_resiliency_snapshot(snap: &ResiliencySnapshot) -> String {
    format!(
        "{{\"recovery_time_ms\":{:.4},\"depth_elasticity\":{:.4}}}",
        snap.recovery_time_ms, snap.depth_elasticity,
    )
}

pub(crate) fn format_vpin_snapshot(snap: &VpinSnapshot) -> String {
    format!(
        "{{\"vpin\":{:.6},\"vpin_zscore\":{:.4},\"vpin_mean\":{:.6},\"vpin_std\":{:.6},\"is_toxic\":{},\"bucket_count\":{}}}",
        snap.vpin,
        snap.vpin_zscore,
        snap.vpin_mean,
        snap.vpin_std,
        if snap.is_toxic { "true" } else { "false" },
        snap.bucket_count,
    )
}

pub(crate) fn format_kyle_lambda_snapshot(snap: &KyleLambdaSnapshot) -> String {
    format!(
        "{{\"lambda_bps\":{:.4},\"r_squared\":{:.4},\"average_lambda_bps\":{:.4},\"sample_count\":{}}}",
        snap.lambda_bps, snap.r_squared, snap.average_lambda_bps, snap.sample_count,
    )
}

pub(crate) fn format_amihud_snapshot(snap: &of_core::AmihudSnapshot) -> String {
    format!(
        "{{\"amihud_ratio\":{:.10},\"average_illiquidity\":{:.10},\"bar_count\":{}}}",
        snap.amihud_ratio, snap.average_illiquidity, snap.bar_count,
    )
}

pub(crate) fn format_cvd_enhancement_snapshot(snap: &CvdEnhancementSnapshot) -> String {
    format!(
        "{{\"delta_ratio\":{:.4},\"delta_zscore\":{:.4},\"divergence_detected\":{}}}",
        snap.delta_ratio,
        snap.delta_zscore,
        if snap.divergence_detected { "true" } else { "false" },
    )
}

pub(crate) fn format_pattern_snapshot(snap: &PatternSnapshot) -> String {
    format!(
        "{{\"imbalance_detected\":{},\"stacked_imbalance_detected\":{},\"absorption_detected\":{},\"exhaustion_detected\":{},\"initiation_detected\":{},\"tailing_detected\":{},\"iceberg_detected\":{},\"spoofing_detected\":{},\"flip_detected\":{},\"liquidity_gap_detected\":{},\"stop_hunt_detected\":{},\"hidden_accumulation\":{},\"hidden_distribution\":{},\"trapped_traders_detected\":{},\"delta_clock_ns\":{},\"trend_day\":{},\"range_day\":{},\"reversal_day\":{},\"session_type_score\":{:.4},\"volume_entropy\":{:.6},\"volume_skew\":{:.6},\"initial_balance_high\":{},\"initial_balance_low\":{},\"hvn_count\":{},\"lvn_count\":{},\"composite_hvn\":{},\"composite_lvn\":{},\"vwap_per_bin_json\":{}}}",
        bool_str(snap.imbalance_detected),
        bool_str(snap.stacked_imbalance_detected),
        bool_str(snap.absorption_detected),
        bool_str(snap.exhaustion_detected),
        bool_str(snap.initiation_detected),
        bool_str(snap.tailing_detected),
        bool_str(snap.iceberg_detected),
        bool_str(snap.spoofing_detected),
        bool_str(snap.flip_detected),
        bool_str(snap.liquidity_gap_detected),
        bool_str(snap.stop_hunt_detected),
        bool_str(snap.hidden_accumulation),
        bool_str(snap.hidden_distribution),
        bool_str(snap.trapped_traders_detected),
        snap.delta_clock_ns,
        bool_str(snap.trend_day),
        bool_str(snap.range_day),
        bool_str(snap.reversal_day),
        snap.session_type_score,
        snap.volume_entropy,
        snap.volume_skew,
        snap.initial_balance_high,
        snap.initial_balance_low,
        snap.hvn_count,
        snap.lvn_count,
        snap.composite_hvn,
        snap.composite_lvn,
        json_from_bytes(&snap.vwap_per_bin_json),
    )
}

fn json_from_bytes(buf: &[u8; 512]) -> &str {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(0);
    if end == 0 { "{}" } else { std::str::from_utf8(&buf[..end]).unwrap_or("{}") }
}

fn bool_str(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

pub(crate) fn format_analytics_snapshot(snap: &of_core::AnalyticsSnapshot) -> String {
    format!(
        "{{\"delta\":{},\"cumulative_delta\":{},\"buy_volume\":{},\"sell_volume\":{},\"last_price\":{},\"point_of_control\":{},\"value_area_low\":{},\"value_area_high\":{}}}",
        snap.delta,
        snap.cumulative_delta,
        snap.buy_volume,
        snap.sell_volume,
        snap.last_price,
        snap.point_of_control,
        snap.value_area_low,
        snap.value_area_high
    )
}

pub(crate) fn format_derived_analytics_snapshot(snap: &DerivedAnalyticsSnapshot) -> String {
    format!(
        "{{\"total_volume\":{},\"trade_count\":{},\"vwap\":{},\"average_trade_size\":{},\"imbalance_bps\":{}}}",
        snap.total_volume,
        snap.trade_count,
        snap.vwap,
        snap.average_trade_size,
        snap.imbalance_bps
    )
}

pub(crate) fn format_session_candle_snapshot(snap: &SessionCandleSnapshot) -> String {
    format!(
        "{{\"open\":{},\"high\":{},\"low\":{},\"close\":{},\"trade_count\":{},\"first_ts_exchange_ns\":{},\"last_ts_exchange_ns\":{}}}",
        snap.open,
        snap.high,
        snap.low,
        snap.close,
        snap.trade_count,
        snap.first_ts_exchange_ns,
        snap.last_ts_exchange_ns
    )
}

#[cfg(feature = "tickbar")]
pub(crate) fn format_bar_series(bars: &[CompletedBar]) -> String {
    let items: Vec<String> = bars
        .iter()
        .map(|b| {
            format!(
                "{{\"timestamp_ns\":{},\"open\":{},\"high\":{},\"low\":{},\"close\":{},\"volume\":{},\"tick_count\":{},\"vwap\":{}}}",
                b.timestamp_ns,
                b.open,
                b.high,
                b.low,
                b.close,
                b.volume,
                b.tick_count,
                b.vwap
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

pub(crate) fn format_interval_candle_snapshot(snap: &IntervalCandleSnapshot) -> String {
    format!(
        "{{\"window_ns\":{},\"open\":{},\"high\":{},\"low\":{},\"close\":{},\"trade_count\":{},\"total_volume\":{},\"vwap\":{},\"first_ts_exchange_ns\":{},\"last_ts_exchange_ns\":{}}}",
        snap.window_ns,
        snap.open,
        snap.high,
        snap.low,
        snap.close,
        snap.trade_count,
        snap.total_volume,
        snap.vwap,
        snap.first_ts_exchange_ns,
        snap.last_ts_exchange_ns
    )
}

fn format_book_levels(levels: &[of_core::BookLevel]) -> String {
    levels
        .iter()
        .map(|level| {
            format!(
                "{{\"level\":{},\"price\":{},\"size\":{}}}",
                level.level, level.price, level.size
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn escape_json(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

pub(crate) fn format_kinetic_energy_snapshot(snap: &KineticEnergySnapshot) -> String {
    format!("{{\"kinetic_energy\":{:.8},\"order_flow_momentum\":{:.8},\"energy_change\":{:.8}}}",
        snap.kinetic_energy, snap.order_flow_momentum, snap.energy_change)
}

pub(crate) fn format_dark_pool_snapshot(snap: &DarkPoolSnapshot) -> String {
    format!("{{\"dark_volume_pct\":{:.4},\"dark_zscore\":{:.4},\"dark_lit_divergence\":{}}}",
        snap.dark_volume_pct, snap.dark_zscore, bool_str(snap.dark_lit_divergence))
}

pub(crate) fn format_options_flow_snapshot(snap: &OptionsFlowSnapshot) -> String {
    format!("{{\"sweep_detected\":{},\"put_call_ratio\":{:.4},\"delta_notional\":{:.0},\"gamma_positioning\":{:.6}}}",
        bool_str(snap.sweep_detected), snap.put_call_ratio, snap.delta_notional, snap.gamma_positioning)
}

pub(crate) fn format_futures_snapshot(snap: &FuturesSnapshot) -> String {
    format!("{{\"basis_bps\":{:.4},\"calendar_spread\":{:.4},\"settlement_pressure\":{:.4},\"roll_progress\":{:.4}}}",
        snap.basis_bps, snap.calendar_spread, snap.settlement_pressure, snap.roll_progress)
}

pub(crate) fn format_volatility_snapshot(snap: &VolatilitySnapshot) -> String {
    format!("{{\"classic_rv\":{:.8},\"parkinson\":{:.8},\"garman_klass\":{:.8},\"yang_zhang\":{:.8}}}",
        snap.classic_rv, snap.parkinson, snap.garman_klass, snap.yang_zhang)
}

pub(crate) fn format_noise_snapshot(snap: &NoiseSnapshot) -> String {
    format!("{{\"noise_variance\":{:.8},\"signal_to_noise\":{:.4}}}",
        snap.noise_variance, snap.signal_to_noise)
}

pub(crate) fn format_hasbrouck_snapshot(snap: &HasbrouckSnapshot) -> String {
    format!("{{\"permanent_impact\":{:.6},\"temporary_impact\":{:.6},\"information_share\":{:.4}}}",
        snap.permanent_impact, snap.temporary_impact, snap.information_share)
}

pub(crate) fn format_almgren_chriss_snapshot(snap: &AlmgrenChrissSnapshot) -> String {
    format!("{{\"permanent_impact_coef\":{:.6},\"temporary_impact_coef\":{:.6}}}",
        snap.permanent_impact_coef, snap.temporary_impact_coef)
}

pub(crate) fn format_spread_decomp_snapshot(snap: &SpreadDecompositionSnapshot) -> String {
    format!("{{\"adverse_selection\":{:.6},\"order_processing_cost\":{:.6},\"inventory_component\":{:.6},\"pin\":{:.4}}}",
        snap.adverse_selection, snap.order_processing_cost, snap.inventory_component, snap.pin)
}

pub(crate) fn format_acd_snapshot(snap: &ACDSnapshot) -> String {
    format!("{{\"mean_duration_ns\":{:.0},\"intensity\":{:.6},\"alpha\":{:.4},\"beta\":{:.4}}}",
        snap.mean_duration_ns, snap.intensity, snap.alpha, snap.beta)
}

pub(crate) fn format_regime_snapshot(snap: &RegimeSnapshot) -> String {
    format!("{{\"regime\":{},\"spread_z\":{:.4},\"vol_z\":{:.4},\"vpin_z\":{:.4}}}",
        snap.regime, snap.spread_z, snap.vol_z, snap.vpin_z)
}

pub(crate) fn format_vol_signature_snapshot(snap: &VolatilitySignatureSnapshot) -> String {
    let points_str: Vec<String> = snap.points[..snap.point_count as usize].iter().map(|p| {
        format!("{{\"lag\":{},\"rv\":{:.8}}}", p.lag, p.rv)
    }).collect();
    format!("{{\"points\":[{}],\"optimal_lag\":{}}}",
        points_str.join(","), snap.optimal_lag)
}

pub(crate) fn format_agent_type_snapshot(snap: &AgentTypeSnapshot) -> String {
    format!("{{\"irp\":{:.4},\"ipin\":{:.6},\"ivpin\":{:.6},\"hft_reflexivity\":{:.4}}}",
        snap.irp, snap.ipin, snap.ivpin, snap.hft_reflexivity)
}

pub(crate) fn format_dark_lit_correlation_snapshot(snap: &DarkLitCorrelationSnapshot) -> String {
    format!("{{\"correlation\":{:.4},\"siphon_active\":{}}}",
        snap.correlation, if snap.siphon_active { "true" } else { "false" })
}

pub(crate) fn format_institutional_flow_snapshot(snap: &InstitutionalFlowSnapshot) -> String {
    format!("{{\"institutional_buy_ratio\":{:.4},\"crowding_score\":{:.4}}}",
        snap.institutional_buy_ratio, snap.crowding_score)
}

pub(crate) fn format_oi_analysis_snapshot(snap: &OIAnalysisSnapshot) -> String {
    format!("{{\"oi_divergence\":{},\"oi_build_rate\":{:.6},\"max_pain_distance_bps\":{:.2}}}",
        if snap.oi_divergence { "true" } else { "false" },
        snap.oi_build_rate, snap.max_pain_distance_bps)
}

pub(crate) fn format_lob_feature_snapshot(snap: &LOBFeatureSnapshot) -> String {
    format!("{{\"spread_bps\":{:.4},\"depth_imbalance\":{:.4},\"microprice\":{:.4},\"depth_slope\":{:.4},\"order_intensity\":{:.4},\"price_pressure_1\":{:.4},\"price_pressure_5\":{:.4},\"price_pressure_10\":{:.4},\"bid_ask_ratio_1\":{:.4},\"bid_ask_ratio_5\":{:.4},\"bid_ask_ratio_10\":{:.4},\"weighted_spread\":{:.4},\"volume_concentration\":{:.4},\"cancel_intensity\":{:.4},\"arrival_intensity\":{:.4},\"trade_flow_imbalance\":{:.4}}}",
        snap.spread_bps, snap.depth_imbalance, snap.microprice, snap.depth_slope,
        snap.order_intensity, snap.price_pressure_1, snap.price_pressure_5, snap.price_pressure_10,
        snap.bid_ask_ratio_1, snap.bid_ask_ratio_5, snap.bid_ask_ratio_10, snap.weighted_spread,
        snap.volume_concentration, snap.cancel_intensity, snap.arrival_intensity, snap.trade_flow_imbalance)
}
