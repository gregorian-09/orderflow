use super::*;

/// Frees a C string returned by this library.
#[no_mangle]
pub extern "C" fn of_string_free(p: *const c_char) {
    if p.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(p as *mut c_char);
    }
}

/// Polls adapter once and dispatches subscription callbacks.
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
        Err(err) => {
            let status = map_runtime_error(&err);
            if err.is_backpressure() {
                dispatch_callbacks(engine, engine.inner.current_quality_flags_bits());
            }
            status
        }
    }
}

/// Override analytics thresholds and buffer sizes at runtime.
/// Pass a pointer to a populated analytics config. Passing NULL resets to defaults.
#[no_mangle]
pub extern "C" fn of_engine_set_analytics_config(
    engine: *mut of_engine,
    config: *const of_analytics_config_t,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    if config.is_null() {
        engine
            .inner
            .set_analytics_config(AnalyticsConfig::default());
    } else {
        let cfg = unsafe { *config };
        engine.inner.set_analytics_config(cfg.into());
    }
    of_error_t::OF_OK as i32
}
