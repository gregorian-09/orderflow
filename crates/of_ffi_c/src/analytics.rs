use super::*;

macro_rules! symbol_json_c_abi {
    ($(#[$meta:meta])* $name:ident, $build:expr) => {
        $(#[$meta])*
        #[no_mangle]
        pub extern "C" fn $name(
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
            let payload = ($build)(engine, &symbol);
            match write_json_to_c_buffer(&payload, out_buf, inout_len) {
                Ok(_) => of_error_t::OF_OK as i32,
                Err(e) => e as i32,
            }
        }
    };
}

symbol_json_c_abi!(
    /// Writes current book snapshot JSON into caller buffer.
    of_get_book_snapshot,
    |engine: &mut of_engine, symbol: &SymbolId| match engine.inner.book_snapshot(symbol) {
        Some(snapshot) => format_book_snapshot(&snapshot),
        None => "{}".to_string(),
    }
);

symbol_json_c_abi!(
    /// Writes current book analytics snapshot JSON into caller buffer.
    ///
    /// Payload shape:
    /// ```json
    /// {"best_bid":...,"best_ask":...,"quoted_spread":...,"relative_spread_bps":...,
    ///  "microprice":...,"bid_depth":...,"ask_depth":...,"depth_imbalance_bps":...}
    /// ```
    of_get_book_analytics_snapshot,
    |engine: &mut of_engine, symbol: &SymbolId| {
        match engine.inner.book_analytics_snapshot(symbol) {
            Some(snapshot) => format_book_analytics_snapshot(&snapshot),
            None => "{}".to_string(),
        }
    }
);

/// Computes weighted average price for an order of `qty` and writes JSON result.
///
/// Payload: `{"price": N}` on success, `{}` if insufficient liquidity.
/// Positive qty = buy (walks asks), negative qty = sell (walks bids).
#[no_mangle]
pub extern "C" fn of_compute_weighted_average_price(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    qty: i64,
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
    let payload = match engine.inner.weighted_average_price(&symbol, qty) {
        Some(price) => format!("{{\"price\":{}}}", price),
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Computes depth slope for the first `levels` price levels and writes JSON result.
///
/// Payload: `{"slope": N.N}`. Returns `{"slope":0.0}` if book has fewer than 2 levels.
#[no_mangle]
pub extern "C" fn of_compute_depth_slope(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    levels: u32,
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
    let slope = engine.inner.depth_slope(&symbol, levels as usize);
    let payload = format!("{{\"slope\":{:.4}}}", slope);

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes mid price as JSON: `{"mid": N}`, or `{}` if no book data.
#[no_mangle]
pub extern "C" fn of_get_mid_price(
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
    let payload = match engine.inner.mid_price(&symbol) {
        Some(mid) => format!("{{\"mid\":{}}}", mid),
        None => "{}".to_string(),
    };
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes last effective spread in bps as JSON: `{"bps": N}`, or `{}`.
#[no_mangle]
pub extern "C" fn of_get_effective_spread_bps(
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
    let bps = engine.inner.effective_spread_bps(&symbol);
    let payload = format!("{{\"bps\":{}}}", bps);
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes average half-spread cost over `window` trades: `{"bps": N}`.
#[no_mangle]
pub extern "C" fn of_get_half_spread_cost_bps(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    window: u32,
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
    let bps = engine.inner.half_spread_cost_bps(&symbol, window as usize);
    let payload = format!("{{\"bps\":{}}}", bps);
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes realised spread over `hold_ticks` ticks ago: `{"bps": N}`.
#[no_mangle]
pub extern "C" fn of_get_realised_spread_bps(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    hold_ticks: u32,
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
    let bps = engine
        .inner
        .realised_spread_bps(&symbol, hold_ticks as usize);
    let payload = format!("{{\"bps\":{}}}", bps);
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Writes book-event analytics snapshot JSON over `window_ns`.
#[no_mangle]
pub extern "C" fn of_get_book_event_analytics(
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
    let snap = engine.inner.book_event_analytics(&symbol, window_ns);
    let payload = format_book_event_analytics_snapshot(&snap);
    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

symbol_json_c_abi!(
    /// Writes resiliency snapshot JSON.
    of_get_resiliency_snapshot,
    |engine: &mut of_engine, symbol: &SymbolId| {
        format_resiliency_snapshot(&engine.inner.resiliency_snapshot(symbol))
    }
);

symbol_json_c_abi!(
    /// Writes VPIN snapshot JSON.
    of_get_vpin_snapshot,
    |engine: &mut of_engine, symbol: &SymbolId| {
        format_vpin_snapshot(&engine.inner.vpin_snapshot(symbol))
    }
);

symbol_json_c_abi!(
    /// Writes Kyle's Lambda snapshot JSON.
    of_get_kyle_lambda_snapshot,
    |engine: &mut of_engine, symbol: &SymbolId| {
        format_kyle_lambda_snapshot(&engine.inner.kyle_lambda_snapshot(symbol))
    }
);

symbol_json_c_abi!(
    /// Writes Amihud illiquidity snapshot JSON.
    of_get_amihud_snapshot,
    |engine: &mut of_engine, symbol: &SymbolId| {
        format_amihud_snapshot(&engine.inner.amihud_snapshot(symbol))
    }
);

symbol_json_c_abi!(
    /// Writes CVD enhancement snapshot JSON.
    of_get_cvd_enhancement_snapshot,
    |engine: &mut of_engine, symbol: &SymbolId| {
        format_cvd_enhancement_snapshot(&engine.inner.cvd_enhancement_snapshot(symbol))
    }
);

symbol_json_c_abi!(
    /// Writes pattern detection snapshot JSON into caller buffer.
    of_get_pattern_snapshot,
    |engine: &mut of_engine, symbol: &SymbolId| {
        format_pattern_snapshot(&engine.inner.pattern_snapshot(symbol))
    }
);

macro_rules! snapshot_c_abi {
    ($name:ident, $format:ident, $method:ident) => {
        /// Writes an analytics snapshot JSON payload into the caller-provided buffer.
        #[no_mangle]
        pub extern "C" fn $name(
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
            let payload = $format(&engine.inner.$method(&symbol));
            match write_json_to_c_buffer(&payload, out_buf, inout_len) {
                Ok(_) => of_error_t::OF_OK as i32,
                Err(e) => e as i32,
            }
        }
    };
}

snapshot_c_abi!(
    of_get_volatility_snapshot,
    format_volatility_snapshot,
    volatility_snapshot
);
snapshot_c_abi!(of_get_noise_snapshot, format_noise_snapshot, noise_snapshot);
snapshot_c_abi!(
    of_get_hasbrouck_snapshot,
    format_hasbrouck_snapshot,
    hasbrouck_snapshot
);
snapshot_c_abi!(
    of_get_almgren_chriss_snapshot,
    format_almgren_chriss_snapshot,
    almgren_chriss_snapshot
);
snapshot_c_abi!(
    of_get_spread_decomp_snapshot,
    format_spread_decomp_snapshot,
    spread_decomp_snapshot
);
snapshot_c_abi!(of_get_acd_snapshot, format_acd_snapshot, acd_snapshot);
snapshot_c_abi!(
    of_get_regime_snapshot,
    format_regime_snapshot,
    regime_snapshot
);
snapshot_c_abi!(
    of_get_kinetic_energy_snapshot,
    format_kinetic_energy_snapshot,
    kinetic_energy_snapshot
);
snapshot_c_abi!(
    of_get_dark_pool_snapshot,
    format_dark_pool_snapshot,
    dark_pool_snapshot
);
snapshot_c_abi!(
    of_get_options_flow_snapshot,
    format_options_flow_snapshot,
    options_flow_snapshot
);
snapshot_c_abi!(
    of_get_futures_snapshot,
    format_futures_snapshot,
    futures_snapshot
);
snapshot_c_abi!(
    of_get_vol_signature_snapshot,
    format_vol_signature_snapshot,
    vol_signature_snapshot
);
snapshot_c_abi!(
    of_get_agent_type_snapshot,
    format_agent_type_snapshot,
    agent_type_snapshot
);
snapshot_c_abi!(
    of_get_dark_lit_correlation_snapshot,
    format_dark_lit_correlation_snapshot,
    dark_lit_correlation_snapshot
);
snapshot_c_abi!(
    of_get_institutional_flow_snapshot,
    format_institutional_flow_snapshot,
    institutional_flow_snapshot
);
snapshot_c_abi!(
    of_get_oi_analysis_snapshot,
    format_oi_analysis_snapshot,
    oi_analysis_snapshot
);

/// Computes LOB feature snapshot from engine book state and caller-provided flow metrics.
#[no_mangle]
pub extern "C" fn of_compute_lob_features(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    trade_imbalance: f64,
    cancel_rate: f64,
    arrival_rate: f64,
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
    let engine = unsafe { &*engine };
    let payload = format_lob_feature_snapshot(&engine.inner.lob_features(
        &symbol,
        trade_imbalance,
        cancel_rate,
        arrival_rate,
    ));
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

/// Sets the tickbar aggregation interval for new per-symbol accumulators.
///
/// A positive `interval_ns` enables tickbar aggregation at the given interval for
/// symbols whose accumulators are created after this call. Zero or negative values
/// disable tickbar aggregation for future accumulators. Existing accumulators
/// are not affected.
///
/// Requires the `tickbar` feature to be enabled at build time.
#[cfg(feature = "tickbar")]
#[no_mangle]
pub extern "C" fn of_engine_set_tickbar_interval(engine: *mut of_engine, interval_ns: i64) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    if interval_ns > 0 {
        engine.inner.set_tickbar_interval(Some(interval_ns));
    } else {
        engine.inner.set_tickbar_interval(None);
    }
    of_error_t::OF_OK as i32
}

/// Reports unsupported tickbar configuration when the native library is built without `tickbar`.
#[cfg(not(feature = "tickbar"))]
#[no_mangle]
pub extern "C" fn of_engine_set_tickbar_interval(engine: *mut of_engine, _interval_ns: i64) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    of_error_t::OF_ERR_STATE as i32
}

/// Writes completed bar series JSON array into caller buffer.
///
/// Requires the `tickbar` feature to be enabled at build time.
/// Returns `OF_ERR_STATE` when tickbar aggregation is not configured for the symbol.
#[cfg(feature = "tickbar")]
#[no_mangle]
pub extern "C" fn of_get_bar_series(
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
    let payload = match engine.inner.bar_series(&symbol) {
        Some(bars) => format_bar_series(&bars),
        None => "[]".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Reports unsupported tickbar bar retrieval when the native library is built without `tickbar`.
#[cfg(not(feature = "tickbar"))]
#[no_mangle]
pub extern "C" fn of_get_bar_series(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_buf: *mut c_void,
    inout_len: *mut u32,
) -> i32 {
    if engine.is_null() || symbol.is_null() || out_buf.is_null() || inout_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    of_error_t::OF_ERR_STATE as i32
}
