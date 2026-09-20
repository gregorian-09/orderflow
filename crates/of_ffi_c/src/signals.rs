use super::*;

/// Writes current signal snapshot JSON into caller buffer.
#[no_mangle]
pub extern "C" fn of_get_signal_snapshot(
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
    let payload = match engine.inner.signal_snapshot(&symbol) {
        Some(snap) => {
            let state = match snap.state {
                SignalState::Neutral => "neutral",
                SignalState::LongBias => "long_bias",
                SignalState::ShortBias => "short_bias",
                SignalState::Blocked => "blocked",
            };
            format!(
                "{{\"module\":\"{}\",\"state\":\"{}\",\"confidence_bps\":{},\"quality_flags\":{},\"reason\":\"{}\"}}",
                escape_json(snap.module_id),
                state,
                snap.confidence_bps,
                snap.quality_flags,
                escape_json(&snap.reason)
            )
        }
        None => "{}".to_string(),
    };

    match write_json_to_c_buffer(&payload, out_buf, inout_len) {
        Ok(_) => of_error_t::OF_OK as i32,
        Err(e) => e as i32,
    }
}

/// Allocates and returns metrics JSON (`*out`) plus byte length (`*out_len`).
#[no_mangle]
pub extern "C" fn of_get_metrics_json(
    engine: *mut of_engine,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if engine.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let engine = unsafe { &mut *engine };
    let metrics = engine.inner.metrics_json();
    allocate_json_string(metrics, out_json, out_len)
}

/// Allocates production market-data persistence health JSON.
///
/// Release the returned pointer with [`of_string_free`].
#[no_mangle]
pub extern "C" fn of_get_market_data_persistence_health_json(
    engine: *mut of_engine,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if engine.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let engine = unsafe { &mut *engine };
    allocate_json_string(
        engine.inner.market_data_persistence_health_json(),
        out_json,
        out_len,
    )
}

/// Allocates and returns adapter inventory JSON.
#[no_mangle]
pub extern "C" fn of_get_adapter_inventory_json(
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let inventory = runtime_adapter_inventory_json();
    allocate_json_string(inventory, out_json, out_len)
}

/// Allocates and returns active adapter status JSON for `engine`.
#[no_mangle]
pub extern "C" fn of_get_active_adapter_status_json(
    engine: *mut of_engine,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if engine.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let engine = unsafe { &mut *engine };
    let status = engine.inner.active_adapter_status_json();
    allocate_json_string(status, out_json, out_len)
}

/// Allocates and returns built-in signal descriptor inventory JSON.
#[no_mangle]
pub extern "C" fn of_get_signal_descriptors_json(
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    allocate_json_string(signal_descriptor_inventory_json(), out_json, out_len)
}

/// Allocates and returns latest signal explanation JSON for `symbol`.
#[no_mangle]
pub extern "C" fn of_get_signal_explanation_json(
    engine: *mut of_engine,
    symbol: *const of_symbol_t,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if engine.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let (symbol, _) = match symbol_from_ffi(symbol) {
        Ok(v) => v,
        Err(e) => return e as i32,
    };

    let engine = unsafe { &mut *engine };
    let explanation = engine
        .inner
        .signal_explanation_json(&symbol)
        .unwrap_or_else(|| "{}".to_string());
    allocate_json_string(explanation, out_json, out_len)
}

/// Allocates and returns signal metrics JSON for `engine`.
#[no_mangle]
pub extern "C" fn of_get_signal_metrics_json(
    engine: *mut of_engine,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if engine.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }

    let engine = unsafe { &mut *engine };
    allocate_json_string(engine.inner.signal_metrics_json(), out_json, out_len)
}

/// Validates a built-in signal configuration and returns a JSON result.
///
/// A syntactically valid call returns `OF_OK` even when the configuration is
/// rejected; inspect the returned document's `valid` and `error` fields.
#[no_mangle]
pub extern "C" fn of_validate_signal_config_json(
    signal_id: *const c_char,
    parameters: *const of_signal_config_parameter_t,
    parameter_count: u32,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let Some(signal_id) = cstr_to_string(signal_id) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let owned_parameters = match signal_parameters_from_ffi(parameters, parameter_count) {
        Ok(parameters) => parameters,
        Err(()) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let borrowed_parameters = borrow_signal_parameters(&owned_parameters);
    let config = SignalConfig::with_parameters(&signal_id, &borrowed_parameters);
    allocate_json_string(
        SignalRegistry::with_built_ins().validate_config_json(&config),
        out_json,
        out_len,
    )
}

/// Constructs a built-in signal and validates it over ordered analytics events.
///
/// The returned library-owned JSON includes configuration, summary metrics,
/// optional retained samples, and structured timestamp/markout warnings. Free
/// it with [`of_string_free`]. Registry construction failures are represented
/// as `valid: false` JSON documents and still return `OF_OK`.
#[no_mangle]
pub extern "C" fn of_validate_signal_replay_json(
    signal_id: *const c_char,
    parameters: *const of_signal_config_parameter_t,
    parameter_count: u32,
    events: *const of_signal_validation_event_t,
    event_count: u32,
    validation_config: *const of_signal_validation_config_t,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    if validation_config.is_null() || out_json.is_null() || out_len.is_null() {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    }
    let Some(signal_id) = cstr_to_string(signal_id) else {
        return of_error_t::OF_ERR_INVALID_ARG as i32;
    };
    let owned_parameters = match signal_parameters_from_ffi(parameters, parameter_count) {
        Ok(parameters) => parameters,
        Err(()) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let ffi_events = match ffi_slice(events, event_count) {
        Ok(events) => events,
        Err(()) => return of_error_t::OF_ERR_INVALID_ARG as i32,
    };
    let borrowed_parameters = borrow_signal_parameters(&owned_parameters);
    let signal_config = SignalConfig::with_parameters(&signal_id, &borrowed_parameters);
    let registry = SignalRegistry::with_built_ins();
    if let Err(error) = registry.validate_config(&signal_config) {
        return allocate_json_string(
            signal_registry_error_json(&signal_id, &error.to_string()),
            out_json,
            out_len,
        );
    }
    let mut signal = match registry.create_signal(&signal_config) {
        Ok(signal) => signal,
        Err(error) => {
            return allocate_json_string(
                signal_registry_error_json(&signal_id, &error.to_string()),
                out_json,
                out_len,
            );
        }
    };

    let analytics = ffi_events
        .iter()
        .map(|event| AnalyticsSnapshot {
            delta: event.delta,
            cumulative_delta: event.cumulative_delta,
            buy_volume: event.buy_volume,
            sell_volume: event.sell_volume,
            last_price: event.last_price,
            point_of_control: event.point_of_control,
            value_area_low: event.value_area_low,
            value_area_high: event.value_area_high,
        })
        .collect::<Vec<_>>();
    let replay_events = analytics
        .iter()
        .zip(ffi_events)
        .map(|(analytics, event)| {
            if event.has_ts_exchange_ns == 0 {
                SignalReplayEvent::new(analytics)
            } else {
                SignalReplayEvent::with_ts_exchange_ns(analytics, event.ts_exchange_ns)
            }
        })
        .collect::<Vec<_>>();
    let validation_config = unsafe { *validation_config };
    let report = validate_signal_replay_events(
        &mut signal,
        &replay_events,
        SignalValidationConfig::new(validation_config.markout_horizon_events as usize)
            .with_flat_price_threshold(validation_config.flat_price_threshold)
            .with_min_confidence_bps(validation_config.min_confidence_bps)
            .with_store_samples(validation_config.store_samples != 0)
            .with_check_monotonic_timestamps(validation_config.check_monotonic_timestamps != 0),
    );
    allocate_json_string(report.json_report(), out_json, out_len)
}

#[derive(Debug)]
enum OwnedSignalConfigValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Text(String),
}

#[derive(Debug)]
struct OwnedSignalConfigParameter {
    name: String,
    value: OwnedSignalConfigValue,
}

fn signal_parameters_from_ffi(
    parameters: *const of_signal_config_parameter_t,
    parameter_count: u32,
) -> Result<Vec<OwnedSignalConfigParameter>, ()> {
    let parameters = ffi_slice(parameters, parameter_count)?;
    parameters
        .iter()
        .map(|parameter| {
            let name = cstr_to_string(parameter.name).ok_or(())?;
            let value = match parameter.kind {
                1 => OwnedSignalConfigValue::Integer(parameter.integer_value),
                2 if parameter.float_value.is_finite() => {
                    OwnedSignalConfigValue::Float(parameter.float_value)
                }
                3 if parameter.boolean_value <= 1 => {
                    OwnedSignalConfigValue::Boolean(parameter.boolean_value != 0)
                }
                4 => OwnedSignalConfigValue::Text(cstr_to_string(parameter.text_value).ok_or(())?),
                _ => return Err(()),
            };
            Ok(OwnedSignalConfigParameter { name, value })
        })
        .collect()
}

fn borrow_signal_parameters(
    parameters: &[OwnedSignalConfigParameter],
) -> Vec<SignalConfigParameter<'_>> {
    parameters
        .iter()
        .map(|parameter| {
            let value = match &parameter.value {
                OwnedSignalConfigValue::Integer(value) => SignalConfigValue::Integer(*value),
                OwnedSignalConfigValue::Float(value) => SignalConfigValue::Float(*value),
                OwnedSignalConfigValue::Boolean(value) => SignalConfigValue::Boolean(*value),
                OwnedSignalConfigValue::Text(value) => SignalConfigValue::Text(value),
            };
            SignalConfigParameter::new(&parameter.name, value)
        })
        .collect()
}

fn ffi_slice<'a, T>(ptr: *const T, len: u32) -> Result<&'a [T], ()> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(());
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr, len as usize) })
}

fn signal_registry_error_json(signal_id: &str, error: &str) -> String {
    format!(
        "{{\"schema_version\":1,\"signal_id\":\"{}\",\"valid\":false,\"error\":\"{}\"}}",
        escape_json(signal_id),
        escape_json(error)
    )
}

pub(crate) fn allocate_json_string(
    payload: String,
    out_json: *mut *const c_char,
    out_len: *mut u32,
) -> i32 {
    let c = match CString::new(payload) {
        Ok(c) => c,
        Err(_) => return of_error_t::OF_ERR_INTERNAL as i32,
    };

    let len = c.as_bytes().len() as u32;
    let ptr = c.into_raw();
    unsafe {
        *out_json = ptr;
        *out_len = len;
    }
    of_error_t::OF_OK as i32
}
