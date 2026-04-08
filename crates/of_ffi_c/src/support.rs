use std::ffi::{c_char, c_void, CStr};
use std::ptr;
use std::sync::atomic::Ordering;

use of_adapters::RawEvent;
use of_core::{BookAction, BookSnapshot, DerivedAnalyticsSnapshot, IntervalCandleSnapshot, SessionCandleSnapshot, Side, SignalState, SymbolId};

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
