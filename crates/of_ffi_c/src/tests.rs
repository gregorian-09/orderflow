#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::time::{Duration, Instant};

    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn test_guard() -> MutexGuard<'static, ()> {
        test_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[derive(Default)]
    struct CallbackSink {
        payloads: Vec<String>,
        kinds: Vec<u32>,
        quality_flags: Vec<u32>,
    }

    extern "C" fn capture_event(ev: *const of_event_t, user_data: *mut c_void) {
        if ev.is_null() || user_data.is_null() {
            return;
        }

        let ev = unsafe { &*ev };
        let sink = unsafe { &mut *(user_data as *mut CallbackSink) };
        let payload = if !ev.payload.is_null() && ev.payload_len > 0 {
            let bytes =
                unsafe { std::slice::from_raw_parts(ev.payload as *const u8, ev.payload_len as usize) };
            String::from_utf8_lossy(bytes).to_string()
        } else {
            "{}".to_string()
        };
        sink.payloads.push(payload);
        sink.kinds.push(ev.kind);
        sink.quality_flags.push(ev.quality_flags);
    }

    fn analytics_json(engine: *mut of_engine, symbol: &of_symbol_t) -> String {
        let mut buf = vec![0u8; 1024];
        let mut len = buf.len() as u32;
        assert_eq!(
            of_get_analytics_snapshot(
                engine,
                symbol as *const of_symbol_t,
                buf.as_mut_ptr().cast::<c_void>(),
                &mut len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        String::from_utf8_lossy(&buf[..len as usize]).to_string()
    }

    fn book_json(engine: *mut of_engine, symbol: &of_symbol_t) -> String {
        let mut buf = vec![0u8; 2048];
        let mut len = buf.len() as u32;
        assert_eq!(
            of_get_book_snapshot(
                engine,
                symbol as *const of_symbol_t,
                buf.as_mut_ptr().cast::<c_void>(),
                &mut len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        String::from_utf8_lossy(&buf[..len as usize]).to_string()
    }

    fn signal_json(engine: *mut of_engine, symbol: &of_symbol_t) -> String {
        let mut buf = vec![0u8; 1024];
        let mut len = buf.len() as u32;
        assert_eq!(
            of_get_signal_snapshot(
                engine,
                symbol as *const of_symbol_t,
                buf.as_mut_ptr().cast::<c_void>(),
                &mut len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        String::from_utf8_lossy(&buf[..len as usize]).to_string()
    }

    fn session_candle_json(engine: *mut of_engine, symbol: &of_symbol_t) -> String {
        let mut buf = vec![0u8; 1024];
        let mut len = buf.len() as u32;
        assert_eq!(
            of_get_session_candle_snapshot(
                engine,
                symbol as *const of_symbol_t,
                buf.as_mut_ptr().cast::<c_void>(),
                &mut len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        String::from_utf8_lossy(&buf[..len as usize]).to_string()
    }

    fn interval_candle_json(engine: *mut of_engine, symbol: &of_symbol_t, window_ns: u64) -> String {
        let mut buf = vec![0u8; 1024];
        let mut len = buf.len() as u32;
        assert_eq!(
            of_get_interval_candle_snapshot(
                engine,
                symbol as *const of_symbol_t,
                window_ns,
                buf.as_mut_ptr().cast::<c_void>(),
                &mut len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        String::from_utf8_lossy(&buf[..len as usize]).to_string()
    }

    #[test]
    fn maps_runtime_backpressure_to_c_status() {
        assert_eq!(
            map_runtime_error(&RuntimeError::Adapter(
                "backpressure: drained_events=3 processed_events=2 dropped_events=1 max_events_per_poll=2"
                    .to_string()
            )),
            of_error_t::OF_ERR_BACKPRESSURE as i32
        );
    }

    #[test]
    fn analytics_snapshot_matches_golden_payload() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-analytics-golden").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                3,
                None,
                ptr::null_mut(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert!(!sub.is_null());

        let trade = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505000,
            size: 9,
            aggressor_side: 1,
            sequence: 1,
            ts_exchange_ns: 10,
            ts_recv_ns: 11,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        let analytics = analytics_json(engine, &ffi_symbol);
        assert_eq!(
            analytics,
            "{\"delta\":9,\"cumulative_delta\":9,\"buy_volume\":9,\"sell_volume\":0,\"last_price\":505000,\"point_of_control\":505000,\"value_area_low\":505000,\"value_area_high\":505000}"
        );

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn signal_snapshot_matches_golden_payload() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-signal-golden").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                4,
                None,
                ptr::null_mut(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert!(!sub.is_null());

        let trade = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505000,
            size: 9,
            aggressor_side: 1,
            sequence: 1,
            ts_exchange_ns: 10,
            ts_recv_ns: 11,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        let signal = signal_json(engine, &ffi_symbol);
        assert_eq!(
            signal,
            "{\"module\":\"delta_momentum_v1\",\"state\":\"neutral\",\"confidence_bps\":500,\"quality_flags\":0,\"reason\":\"delta_inside_band\"}"
        );

        let mut explanation_out: *const c_char = ptr::null();
        let mut explanation_len = 0u32;
        assert_eq!(
            of_get_signal_explanation_json(
                engine,
                &ffi_symbol as *const of_symbol_t,
                &mut explanation_out as *mut *const c_char,
                &mut explanation_len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        let explanation = unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(
                explanation_out.cast::<u8>(),
                explanation_len as usize,
            ))
            .to_string()
        };
        assert!(explanation.contains("\"module_id\":\"delta_momentum_v1\""));
        assert!(explanation.contains("\"state\":\"neutral\""));
        assert!(explanation.contains("\"reason_code\":\"delta_momentum_inside_band\""));
        assert!(explanation.contains("\"inputs\":[{\"name\":\"delta\",\"value\":9}]"));
        of_string_free(explanation_out);

        let mut metrics_out: *const c_char = ptr::null();
        let mut metrics_len = 0u32;
        assert_eq!(
            of_get_signal_metrics_json(
                engine,
                &mut metrics_out as *mut *const c_char,
                &mut metrics_len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        let signal_metrics = unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(
                metrics_out.cast::<u8>(),
                metrics_len as usize,
            ))
            .to_string()
        };
        assert!(signal_metrics.contains("\"schema_version\":1"));
        assert!(signal_metrics.contains("\"signal_symbols\":1"));
        assert!(signal_metrics.contains("\"explanation_symbols\":1"));
        assert!(signal_metrics.contains("\"neutral\":1"));
        assert!(signal_metrics.contains("\"average_confidence_bps\":500"));
        of_string_free(metrics_out);

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn session_candle_snapshot_matches_golden_payload() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-session-candle-golden").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                3,
                None,
                ptr::null_mut(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert!(!sub.is_null());

        for (price, size, side, seq, ts) in [
            (505000, 9, 1u32, 1u64, 10u64),
            (504900, 4, 0u32, 2u64, 20u64),
        ] {
            let trade = of_trade_t {
                symbol: of_symbol_t {
                    venue: venue.as_ptr(),
                    symbol: symbol.as_ptr(),
                    depth_levels: 10,
                },
                price,
                size,
                aggressor_side: side,
                sequence: seq,
                ts_exchange_ns: ts,
                ts_recv_ns: ts + 1,
            };
            assert_eq!(
                of_ingest_trade(engine, &trade as *const of_trade_t, 0),
                of_error_t::OF_OK as i32
            );
        }

        let candle = session_candle_json(engine, &ffi_symbol);
        assert_eq!(
            candle,
            "{\"open\":505000,\"high\":505000,\"low\":504900,\"close\":504900,\"trade_count\":2,\"first_ts_exchange_ns\":10,\"last_ts_exchange_ns\":20}"
        );

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn interval_candle_snapshot_matches_golden_payload() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-interval-candle-golden").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        for (price, size, side, seq, ts) in [
            (505000, 9, 1u32, 1u64, 10u64),
            (504900, 4, 0u32, 2u64, 40u64),
            (505100, 8, 1u32, 3u64, 100u64),
        ] {
            let trade = of_trade_t {
                symbol: of_symbol_t {
                    venue: venue.as_ptr(),
                    symbol: symbol.as_ptr(),
                    depth_levels: 10,
                },
                price,
                size,
                aggressor_side: side,
                sequence: seq,
                ts_exchange_ns: ts,
                ts_recv_ns: ts + 1,
            };
            assert_eq!(
                of_ingest_trade(engine, &trade as *const of_trade_t, 0),
                of_error_t::OF_OK as i32
            );
        }

        let candle = interval_candle_json(engine, &ffi_symbol, 70);
        assert_eq!(
            candle,
            "{\"window_ns\":70,\"open\":504900,\"high\":505100,\"low\":504900,\"close\":505100,\"trade_count\":2,\"total_volume\":12,\"vwap\":505033,\"first_ts_exchange_ns\":40,\"last_ts_exchange_ns\":100}"
        );

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn health_stream_matches_golden_payload() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-health-golden").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                5,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert!(!sub.is_null());

        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);
        assert_eq!(sink.payloads.len(), 1);
        assert_eq!(
            sink.payloads[0],
            "{\"health_seq\":1,\"started\":true,\"connected\":true,\"degraded\":false,\"reconnect_state\":\"streaming\",\"quality_flags\":0,\"quality_flags_detail\":[],\"last_error\":null,\"protocol_info\":\"mock_adapter\",\"tracked_symbols\":0,\"processed_events\":0,\"adapter_total_count\":1,\"adapter_healthy_count\":1,\"runtime_health_status\":\"healthy\",\"external_feed_enabled\":false,\"external_feed_reconnecting\":false,\"external_sequence_enforced\":true,\"external_last_ingest_ns\":null,\"max_events_per_poll\":null,\"backpressure_dropped_events\":0,\"circuit_breaker_enabled\":false,\"circuit_breaker_open\":false,\"circuit_breaker_consecutive_failures\":0,\"circuit_breaker_opened_count\":0,\"circuit_breaker_cooldown_ms\":1000}"
        );

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn health_stream_emits_on_state_change_only() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-health-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                5,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert!(!sub.is_null());

        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);
        assert_eq!(
            of_engine_poll_once(engine, DataQualityFlags::ADAPTER_DEGRADED.bits()),
            of_error_t::OF_OK as i32
        );
        assert_eq!(
            of_engine_poll_once(engine, DataQualityFlags::ADAPTER_DEGRADED.bits()),
            of_error_t::OF_OK as i32
        );
        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);

        assert_eq!(sink.payloads.len(), 3);
        for kind in &sink.kinds {
            assert_eq!(*kind, 5);
        }
        assert_eq!(sink.quality_flags, vec![0, DataQualityFlags::ADAPTER_DEGRADED.bits(), 0]);

        assert!(sink.payloads[0].contains("\"health_seq\""));
        assert!(sink.payloads[0].contains("\"reconnect_state\""));
        assert!(sink.payloads[0].contains("\"protocol_info\""));
        assert!(sink.payloads[0].contains("\"quality_flags_detail\""));
        assert!(sink.payloads[0].contains("\"tracked_symbols\""));

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn metrics_json_includes_additive_observability_fields() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-metrics-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let mut out: *const c_char = ptr::null();
        let mut out_len = 0u32;
        assert_eq!(
            of_get_metrics_json(engine, &mut out as *mut *const c_char, &mut out_len as *mut u32),
            of_error_t::OF_OK as i32
        );
        let metrics = unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(out.cast::<u8>(), out_len as usize))
                .to_string()
        };
        assert!(metrics.contains("\"health_seq\":"));
        assert!(metrics.contains("\"quality_flags_detail\":"));
        assert!(metrics.contains("\"book_symbols\":"));
        assert!(metrics.contains("\"external_last_ingest_ns\":"));
        assert!(metrics.contains("\"adapter_total_count\":1"));
        assert!(metrics.contains("\"adapter_healthy_count\":1"));
        assert!(metrics.contains("\"runtime_health_status\":\"healthy\""));
        assert!(metrics.contains("\"circuit_breaker_enabled\":false"));
        assert!(metrics.contains("\"circuit_breaker_open\":false"));
        of_string_free(out);

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn health_stream_stops_after_unsubscribe() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-health-unsub-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                5,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert!(!sub.is_null());

        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);
        assert_eq!(sink.payloads.len(), 1);

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);

        assert_eq!(
            of_engine_poll_once(engine, DataQualityFlags::ADAPTER_DEGRADED.bits()),
            of_error_t::OF_OK as i32
        );
        assert_eq!(of_engine_poll_once(engine, 0), of_error_t::OF_OK as i32);

        // After unsubscribe no further events should arrive, even on health transitions.
        assert_eq!(sink.payloads.len(), 1);

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn unsubscribe_symbol_deactivates_matching_callbacks() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-unsub-symbol-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let sym_a = CString::new("ESM6").expect("cstring");
        let sym_b = CString::new("NQM6").expect("cstring");
        let ffi_symbol_a = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: sym_a.as_ptr(),
            depth_levels: 10,
        };
        let ffi_symbol_b = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: sym_b.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub_a: *mut of_subscription = ptr::null_mut();
        let mut sub_b: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol_a as *const of_symbol_t,
                5,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub_a as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol_b as *const of_symbol_t,
                5,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub_b as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );

        let engine_ref = unsafe { &mut *engine };
        assert_eq!(engine_ref.subs.len(), 2);

        assert_eq!(
            of_unsubscribe_symbol(engine, &ffi_symbol_a as *const of_symbol_t),
            of_error_t::OF_OK as i32
        );
        assert_eq!(engine_ref.subs.len(), 1);
        assert_eq!(engine_ref.subs[0].symbol.symbol, "NQM6");

        assert_eq!(of_unsubscribe(sub_a), of_error_t::OF_OK as i32);
        assert_eq!(of_unsubscribe(sub_b), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn ingest_trade_updates_analytics_and_emits_callbacks() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-ingest-trade-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                3,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );
        assert!(!sub.is_null());

        let trade = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505000,
            size: 9,
            aggressor_side: 1,
            sequence: 1,
            ts_exchange_ns: 10,
            ts_recv_ns: 11,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        let analytics = analytics_json(engine, &ffi_symbol);
        assert!(analytics.contains("\"delta\":9"));
        assert_eq!(sink.payloads.len(), 1);
        assert_eq!(sink.kinds, vec![3]);

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn book_snapshot_returns_materialized_levels() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-book-snapshot-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let ask = of_book_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            side: 1,
            level: 0,
            price: 505100,
            size: 9,
            action: 0,
            sequence: 7,
            ts_exchange_ns: 22,
            ts_recv_ns: 23,
        };
        assert_eq!(
            of_ingest_book(engine, &ask as *const of_book_t, 0),
            of_error_t::OF_OK as i32
        );

        let json = book_json(engine, &ffi_symbol);
        assert!(json.contains("\"venue\":\"CME\""));
        assert!(json.contains("\"symbol\":\"ESM6\""));
        assert!(json.contains("\"bids\":[]"));
        assert!(json.contains("\"asks\":[{\"level\":0,\"price\":505100,\"size\":9}]"));
        assert!(json.contains("\"last_sequence\":7"));
        assert!(json.contains("\"ts_exchange_ns\":22"));
        assert!(json.contains("\"ts_recv_ns\":23"));

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn book_snapshot_reports_required_buffer_size() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-book-buffer-size-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let ask = of_book_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            side: 1,
            level: 0,
            price: 505100,
            size: 9,
            action: 0,
            sequence: 7,
            ts_exchange_ns: 22,
            ts_recv_ns: 23,
        };
        assert_eq!(
            of_ingest_book(engine, &ask as *const of_book_t, 0),
            of_error_t::OF_OK as i32
        );

        let mut buf = [0u8; 8];
        let mut len = buf.len() as u32;
        assert_eq!(
            of_get_book_snapshot(
                engine,
                &ffi_symbol as *const of_symbol_t,
                buf.as_mut_ptr().cast::<c_void>(),
                &mut len as *mut u32,
            ),
            of_error_t::OF_ERR_INVALID_ARG as i32
        );
        assert!(len > buf.len() as u32);

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn derived_analytics_snapshot_returns_session_stats() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-derived-analytics-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let trade_1 = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505000,
            size: 10,
            aggressor_side: 1,
            sequence: 1,
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        };
        let trade_2 = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 504900,
            size: 5,
            aggressor_side: 0,
            sequence: 2,
            ts_exchange_ns: 3,
            ts_recv_ns: 4,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade_1 as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );
        assert_eq!(
            of_ingest_trade(engine, &trade_2 as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        let mut buf = vec![0u8; 1024];
        let mut len = buf.len() as u32;
        assert_eq!(
            of_get_derived_analytics_snapshot(
                engine,
                &ffi_symbol as *const of_symbol_t,
                buf.as_mut_ptr().cast::<c_void>(),
                &mut len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        let json = String::from_utf8_lossy(&buf[..len as usize]).to_string();
        assert!(json.contains("\"total_volume\":15"));
        assert!(json.contains("\"trade_count\":2"));
        assert!(json.contains("\"vwap\":504966"));
        assert!(json.contains("\"average_trade_size\":7"));
        assert!(json.contains("\"imbalance_bps\":3333"));

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn inventory_status_and_signal_descriptor_json_are_allocated() {
        let _guard = test_guard();

        let mut inventory_out: *const c_char = ptr::null();
        let mut inventory_len = 0u32;
        assert_eq!(
            of_get_adapter_inventory_json(
                &mut inventory_out as *mut *const c_char,
                &mut inventory_len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        let inventory = unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(
                inventory_out.cast::<u8>(),
                inventory_len as usize,
            ))
            .to_string()
        };
        assert!(inventory.contains("\"schema_version\":1"));
        assert!(inventory.contains("\"provider_id\":\"mock\""));
        assert!(inventory.contains("\"total_count\":4"));
        of_string_free(inventory_out);

        let mut signals_out: *const c_char = ptr::null();
        let mut signals_len = 0u32;
        assert_eq!(
            of_get_signal_descriptors_json(
                &mut signals_out as *mut *const c_char,
                &mut signals_len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        let signals = unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(
                signals_out.cast::<u8>(),
                signals_len as usize,
            ))
            .to_string()
        };
        assert!(signals.contains("\"schema_version\":1"));
        assert!(signals.contains("\"signals\":["));
        assert!(signals.contains("\"id\":\"delta_momentum_v1\""));
        of_string_free(signals_out);

        let cfg = of_engine_config_t {
            instance_id: ptr::null(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let mut status_out: *const c_char = ptr::null();
        let mut status_len = 0u32;
        assert_eq!(
            of_get_active_adapter_status_json(
                engine,
                &mut status_out as *mut *const c_char,
                &mut status_len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        let status = unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(
                status_out.cast::<u8>(),
                status_len as usize,
            ))
            .to_string()
        };
        assert!(status.contains("\"provider_id\":\"mock\""));
        assert!(status.contains("\"started\":true"));
        assert!(status.contains("\"connected\":true"));
        assert!(status.contains("\"healthy\":true"));
        assert!(status.contains("\"capabilities\":{"));
        of_string_free(status_out);

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn ingest_book_rejects_invalid_side() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-ingest-book-invalid-side").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let book = of_book_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            side: 99,
            level: 0,
            price: 505000,
            size: 1,
            action: 0,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };

        assert_eq!(
            of_ingest_book(engine, &book as *const of_book_t, 0),
            of_error_t::OF_ERR_INVALID_ARG as i32
        );

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn external_supervisor_sequence_gap_is_propagated_to_callbacks() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-external-seq-gap").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let policy = of_external_feed_policy_t {
            stale_after_ms: 0,
            enforce_sequence: 1,
        };
        assert_eq!(
            of_configure_external_feed(engine, &policy as *const of_external_feed_policy_t),
            of_error_t::OF_OK as i32
        );

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        let mut sink = Box::new(CallbackSink::default());
        let mut sub: *mut of_subscription = ptr::null_mut();
        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                3,
                Some(capture_event),
                (&mut *sink as *mut CallbackSink).cast::<c_void>(),
                &mut sub as *mut *mut of_subscription,
            ),
            of_error_t::OF_OK as i32
        );

        let trade1 = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505000,
            size: 1,
            aggressor_side: 1,
            sequence: 1,
            ts_exchange_ns: 1,
            ts_recv_ns: 1,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade1 as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        let trade_gap = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505010,
            size: 1,
            aggressor_side: 1,
            sequence: 3,
            ts_exchange_ns: 2,
            ts_recv_ns: 2,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade_gap as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        let last_flag = *sink.quality_flags.last().expect("quality flag");
        assert!(last_flag & DataQualityFlags::SEQUENCE_GAP.bits() != 0);

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn book_snapshot_stream_emits_materialized_snapshot_payload() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-book-snapshot-stream").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg as *const of_engine_config_t, &mut engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };
        let mut sub: *mut of_subscription = ptr::null_mut();
        let payloads = Arc::new(Mutex::new(Vec::<String>::new()));
        let payloads_ptr = Arc::as_ptr(&payloads) as *mut c_void;

        extern "C" fn on_book_snapshot(ev: *const of_event_t, user: *mut c_void) {
            if ev.is_null() || user.is_null() {
                return;
            }
            unsafe {
                let ev = &*ev;
                let payload =
                    std::slice::from_raw_parts(ev.payload as *const u8, ev.payload_len as usize);
                let payload = String::from_utf8_lossy(payload).to_string();
                let sink = &*(user as *const Mutex<Vec<String>>);
                sink.lock().expect("lock").push(payload);
            }
        }

        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                6,
                Some(on_book_snapshot),
                payloads_ptr,
                &mut sub,
            ),
            of_error_t::OF_OK as i32
        );
        assert_eq!(
            of_ingest_book(
                engine,
                &of_book_t {
                    symbol: of_symbol_t {
                        venue: venue.as_ptr(),
                        symbol: symbol.as_ptr(),
                        depth_levels: 10,
                    },
                    side: 0,
                    level: 0,
                    price: 505000,
                    size: 8,
                    action: 0,
                    sequence: 77,
                    ts_exchange_ns: 1001,
                    ts_recv_ns: 1002,
                },
                0,
            ),
            of_error_t::OF_OK as i32
        );

        let payloads = payloads.lock().expect("lock");
        assert_eq!(payloads.len(), 1);
        assert!(payloads[0].contains("\"bids\":[{\"level\":0,\"price\":505000,\"size\":8}]"));
        assert!(payloads[0].contains("\"last_sequence\":77"));

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[cfg(feature = "tickbar")]
    #[test]
    fn bar_series_returns_completed_bars_for_tickbar_interval() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-bar-series-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        // Enable tickbar with 1000ns interval BEFORE ingesting trades
        assert_eq!(
            of_engine_set_tickbar_interval(engine, 1000),
            of_error_t::OF_OK as i32
        );

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        // Trade at T=0ns → bar [0, 1000)
        let trade1 = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505000,
            size: 9,
            aggressor_side: 1,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade1 as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        // Trade at T=500ns → same bar [0, 1000)
        let trade2 = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 504900,
            size: 4,
            aggressor_side: 0,
            sequence: 2,
            ts_exchange_ns: 500,
            ts_recv_ns: 501,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade2 as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        // Trade at T=1500ns → bar [1000, 2000)
        let trade3 = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505100,
            size: 8,
            aggressor_side: 1,
            sequence: 3,
            ts_exchange_ns: 1500,
            ts_recv_ns: 1501,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade3 as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        // Query bar series
        let mut buf = vec![0u8; 2048];
        let mut len = buf.len() as u32;
        assert_eq!(
            of_get_bar_series(
                engine,
                &ffi_symbol as *const of_symbol_t,
                buf.as_mut_ptr().cast::<c_void>(),
                &mut len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        let json = String::from_utf8_lossy(&buf[..len as usize]).to_string();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("\"timestamp_ns\":0"));
        assert!(json.contains("\"open\":505000"));
        assert!(json.contains("\"close\":504900"));
        assert!(json.contains("\"timestamp_ns\":1000"));
        assert!(json.contains("\"open\":505100"));

        // Should have 2 bars: [0,1000) from trades at 0 and 500, and [1000,2000) from trade at 1500
        assert!(
            json.matches("open").count() >= 2,
            "expected at least 2 bars, got JSON: {json}"
        );

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[cfg(feature = "tickbar")]
    #[test]
    fn bar_series_returns_empty_array_when_tickbar_not_configured() {
        // Test that of_get_bar_series returns "[]" when no tickbar interval was set.
        let _guard = test_guard();

        let instance_id = CString::new("ffi-bar-series-empty-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg, &mut engine as *mut *mut of_engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };

        // Ingest a trade (no tickbar interval set)
        let trade = of_trade_t {
            symbol: of_symbol_t {
                venue: venue.as_ptr(),
                symbol: symbol.as_ptr(),
                depth_levels: 10,
            },
            price: 505000,
            size: 9,
            aggressor_side: 1,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        };
        assert_eq!(
            of_ingest_trade(engine, &trade as *const of_trade_t, 0),
            of_error_t::OF_OK as i32
        );

        let mut buf = vec![0u8; 1024];
        let mut len = buf.len() as u32;
        assert_eq!(
            of_get_bar_series(
                engine,
                &ffi_symbol as *const of_symbol_t,
                buf.as_mut_ptr().cast::<c_void>(),
                &mut len as *mut u32,
            ),
            of_error_t::OF_OK as i32
        );
        let json = String::from_utf8_lossy(&buf[..len as usize]).to_string();
        assert_eq!(json, "[]");

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[cfg(feature = "tickbar")]
    #[test]
    fn bar_series_rejects_null_engine() {
        assert_eq!(
            of_get_bar_series(ptr::null_mut(), ptr::null(), ptr::null_mut(), ptr::null_mut()),
            of_error_t::OF_ERR_INVALID_ARG as i32
        );
    }

    #[test]
    fn derived_analytics_stream_emits_session_snapshot_payload() {
        let _guard = test_guard();

        let instance_id = CString::new("ffi-derived-stream-test").expect("cstring");
        let cfg = of_engine_config_t {
            instance_id: instance_id.as_ptr(),
            config_path: ptr::null(),
            log_level: 0,
            enable_persistence: 0,
            audit_max_bytes: 0,
            audit_max_files: 0,
            audit_redact_tokens_csv: ptr::null(),
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
        };

        let mut engine: *mut of_engine = ptr::null_mut();
        assert_eq!(
            of_engine_create(&cfg as *const of_engine_config_t, &mut engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(of_engine_start(engine), of_error_t::OF_OK as i32);

        let venue = CString::new("CME").expect("cstring");
        let symbol = CString::new("ESM6").expect("cstring");
        let ffi_symbol = of_symbol_t {
            venue: venue.as_ptr(),
            symbol: symbol.as_ptr(),
            depth_levels: 10,
        };
        let mut sub: *mut of_subscription = ptr::null_mut();
        let payloads = Arc::new(Mutex::new(Vec::<String>::new()));
        let payloads_ptr = Arc::as_ptr(&payloads) as *mut c_void;

        extern "C" fn on_derived(ev: *const of_event_t, user: *mut c_void) {
            if ev.is_null() || user.is_null() {
                return;
            }
            unsafe {
                let ev = &*ev;
                let payload =
                    std::slice::from_raw_parts(ev.payload as *const u8, ev.payload_len as usize);
                let payload = String::from_utf8_lossy(payload).to_string();
                let sink = &*(user as *const Mutex<Vec<String>>);
                sink.lock().expect("lock").push(payload);
            }
        }

        assert_eq!(
            of_subscribe(
                engine,
                &ffi_symbol as *const of_symbol_t,
                7,
                Some(on_derived),
                payloads_ptr,
                &mut sub,
            ),
            of_error_t::OF_OK as i32
        );
        assert_eq!(
            of_ingest_trade(
                engine,
                &of_trade_t {
                    symbol: of_symbol_t {
                        venue: venue.as_ptr(),
                        symbol: symbol.as_ptr(),
                        depth_levels: 10,
                    },
                    price: 505000,
                    size: 8,
                    aggressor_side: 1,
                    sequence: 10,
                    ts_exchange_ns: 100,
                    ts_recv_ns: 101,
                },
                0,
            ),
            of_error_t::OF_OK as i32
        );

        let payloads = payloads.lock().expect("lock");
        assert_eq!(payloads.len(), 1);
        assert!(payloads[0].contains("\"total_volume\":8"));
        assert!(payloads[0].contains("\"trade_count\":1"));
        assert!(payloads[0].contains("\"imbalance_bps\":10000"));

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn execution_abi_submits_simulated_order() {
        let _guard = test_guard();
        let route = CString::new("SIM").expect("cstring");
        let account = CString::new("ACC").expect("cstring");
        let venue = CString::new("SIM").expect("cstring");
        let instrument = CString::new("ES").expect("cstring");
        let cfg = of_execution_route_config_t {
            route_id: route.as_ptr(),
            account_id: account.as_ptr(),
            venue: venue.as_ptr(),
            instrument: instrument.as_ptr(),
            enabled: 1,
            kill_switch: 0,
            max_order_qty: 100,
            max_order_notional: 1_000_000,
            max_open_orders: 10,
            max_open_notional: 10_000_000,
            price_band_ticks: 0,
        };

        let mut engine: *mut of_execution_engine = ptr::null_mut();
        assert_eq!(
            of_execution_engine_create(&cfg, &mut engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(
            of_execution_engine_start(engine),
            of_error_t::OF_OK as i32
        );

        let client_order_id = CString::new("C1").expect("cstring");
        let strategy = CString::new("STRAT").expect("cstring");
        let req = of_execution_order_request_t {
            client_order_id: client_order_id.as_ptr(),
            account_id: account.as_ptr(),
            route_id: route.as_ptr(),
            strategy_id: strategy.as_ptr(),
            venue: venue.as_ptr(),
            instrument: instrument.as_ptr(),
            side: 1,
            order_type: 2,
            time_in_force: 1,
            quantity: 10,
            limit_price: 5000,
            stop_price: 0,
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        };
        let mut events = [of_execution_event_t {
            exec_type: 0,
            order_status: 0,
            client_order_id: [0; 41],
            orig_client_order_id: [0; 41],
            venue_order_id: [0; 49],
            execution_id: [0; 49],
            account_id: [0; 33],
            route_id: [0; 33],
            venue: [0; 17],
            instrument: [0; 33],
            last_qty: 0,
            last_price: 0,
            cumulative_qty: 0,
            leaves_qty: 0,
            average_price: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
            reason: 0,
            text: [0; 129],
        }; 4];
        let mut len = events.len() as u32;
        assert_eq!(
            of_execution_submit_order(engine, &req, events.as_mut_ptr(), &mut len),
            of_error_t::OF_OK as i32
        );
        assert_eq!(len, 2);
        assert_eq!(events[0].exec_type, 1);
        assert_eq!(events[1].exec_type, 3);
        assert_eq!(events[1].order_status, 4);

        let mut state = of_execution_order_state_t {
            client_order_id: [0; 41],
            venue_order_id: [0; 49],
            account_id: [0; 33],
            route_id: [0; 33],
            venue: [0; 17],
            instrument: [0; 33],
            status: 0,
            order_qty: 0,
            cumulative_qty: 0,
            leaves_qty: 0,
            average_price: 0,
            updated_ns: 0,
        };
        assert_eq!(
            of_execution_get_order_state(engine, client_order_id.as_ptr(), &mut state),
            of_error_t::OF_OK as i32
        );
        assert_eq!(state.status, 4);
        assert_eq!(state.cumulative_qty, 10);

        let mut metrics = of_execution_metrics_t {
            submitted: 0,
            cancelled: 0,
            amended: 0,
            events_applied: 0,
            risk_rejected: 0,
            adapter_errors: 0,
            recovered: 0,
        };
        assert_eq!(
            of_execution_metrics(engine, &mut metrics),
            of_error_t::OF_OK as i32
        );
        assert_eq!(metrics.submitted, 1);
        assert_eq!(metrics.events_applied, 2);

        of_execution_engine_destroy(engine);
    }

    #[test]
    fn execution_wal_integrity_report_handles_empty_and_corrupt_files() {
        let _guard = test_guard();
        let empty_path =
            std::env::temp_dir().join(format!("orderflow-ffi-empty-wal-{}.ofwal", std::process::id()));
        let corrupt_path =
            std::env::temp_dir().join(format!("orderflow-ffi-corrupt-wal-{}.ofwal", std::process::id()));
        let missing_path =
            std::env::temp_dir().join(format!("orderflow-ffi-missing-wal-{}.ofwal", std::process::id()));
        let _ = std::fs::remove_file(&empty_path);
        let _ = std::fs::remove_file(&corrupt_path);
        let _ = std::fs::remove_file(&missing_path);

        let mut report = of_execution_wal_integrity_report_t {
            records: 99,
            bytes: 99,
            first_sequence: 99,
            last_sequence: 99,
            checksum_failures: 99,
            sequence_failures: 99,
            has_first_sequence: 1,
            has_last_sequence: 1,
            truncated_tail: 1,
            valid: 0,
        };
        assert_eq!(
            of_execution_wal_integrity_report(ptr::null(), &mut report),
            of_error_t::OF_ERR_INVALID_ARG as i32
        );

        let missing = CString::new(missing_path.to_string_lossy().as_bytes()).expect("cstring");
        assert_eq!(
            of_execution_wal_integrity_report(missing.as_ptr(), &mut report),
            of_error_t::OF_ERR_IO as i32
        );

        std::fs::write(&empty_path, []).expect("write empty wal");
        let empty = CString::new(empty_path.to_string_lossy().as_bytes()).expect("cstring");
        assert_eq!(
            of_execution_wal_integrity_report(empty.as_ptr(), &mut report),
            of_error_t::OF_OK as i32
        );
        assert_eq!(report.records, 0);
        assert_eq!(report.bytes, 0);
        assert_eq!(report.has_first_sequence, 0);
        assert_eq!(report.has_last_sequence, 0);
        assert_eq!(report.truncated_tail, 0);
        assert_eq!(report.valid, 1);

        std::fs::write(&corrupt_path, [1_u8, 2, 3]).expect("write corrupt wal");
        let corrupt = CString::new(corrupt_path.to_string_lossy().as_bytes()).expect("cstring");
        assert_eq!(
            of_execution_wal_integrity_report(corrupt.as_ptr(), &mut report),
            of_error_t::OF_OK as i32
        );
        assert_eq!(report.records, 0);
        assert_eq!(report.valid, 0);
        assert_eq!(report.truncated_tail, 1);

        let _ = std::fs::remove_file(&empty_path);
        let _ = std::fs::remove_file(&corrupt_path);
    }

    #[test]
    fn execution_segmented_wal_integrity_report_scans_directory() {
        let _guard = test_guard();
        let root = std::env::temp_dir().join(format!(
            "orderflow-ffi-segmented-wal-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut journal = of_execution::SegmentedWalExecutionJournal::open(
                of_execution::WalSegmentConfig::new(&root)
                    .with_sync_policy(of_execution_core::WalSyncPolicy::Never)
                    .with_max_segment_records(1),
            )
            .expect("segmented wal");
            let client_order_id = of_execution_core::ClientOrderId::new("C1").expect("client id");
            of_execution::ExecutionJournal::record_command(
                &mut journal,
                of_execution::JournalCommandKind::Submit,
                client_order_id,
                1,
            )
                .expect("record command");
            journal.sync().expect("sync");
        }

        let root_c = CString::new(root.to_string_lossy().as_bytes()).expect("cstring");
        let mut report = of_execution_segmented_wal_integrity_report_t {
            segments: 0,
            records: 0,
            bytes: 0,
            first_sequence: 0,
            last_sequence: 0,
            checksum_failures: 0,
            sequence_failures: 0,
            has_first_sequence: 0,
            has_last_sequence: 0,
            valid: 0,
        };
        assert_eq!(
            of_execution_segmented_wal_integrity_report(root_c.as_ptr(), &mut report),
            of_error_t::OF_OK as i32
        );
        assert_eq!(report.valid, 1);
        assert_eq!(report.segments, 1);
        assert_eq!(report.records, 1);
        assert_eq!(report.first_sequence, 1);
        assert_eq!(report.last_sequence, 1);
        assert_eq!(report.has_first_sequence, 1);
        assert_eq!(report.has_last_sequence, 1);

        let segment_path = root.join("wal-000000000001.ofwal");
        let mut bytes = std::fs::read(&segment_path).expect("read segment");
        let last = bytes.last_mut().expect("segment byte");
        *last ^= 0x01;
        std::fs::write(&segment_path, bytes).expect("write corrupt segment");

        assert_eq!(
            of_execution_segmented_wal_integrity_report(root_c.as_ptr(), &mut report),
            of_error_t::OF_OK as i32
        );
        assert_eq!(report.valid, 0);
        assert_eq!(report.checksum_failures, 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn execution_checkpoint_store_integrity_report_scans_directory() {
        let _guard = test_guard();
        let root = std::env::temp_dir().join(format!(
            "orderflow-ffi-checkpoints-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut store = of_execution::FileExecutionCheckpointStore::open(
                of_execution::CheckpointConfig::new(&root).with_sync_on_save(false),
            )
            .expect("checkpoint store");
            let first = of_execution::ExecutionCheckpoint::new(
                1,
                of_execution_core::WalSequence(10),
                100,
            );
            let second = of_execution::ExecutionCheckpoint::new(
                2,
                of_execution_core::WalSequence(20),
                200,
            );
            of_execution::ExecutionCheckpointStore::save_checkpoint(&mut store, &first)
                .expect("first checkpoint");
            let manifest =
                of_execution::ExecutionCheckpointStore::save_checkpoint(&mut store, &second)
                    .expect("second checkpoint");

            let root_c = CString::new(root.to_string_lossy().as_bytes()).expect("cstring");
            let mut report = of_execution_checkpoint_store_integrity_report_t {
                checkpoint_files: 0,
                valid_checkpoints: 0,
                invalid_checkpoints: 0,
                bytes: 0,
                latest_checkpoint_id: 0,
                latest_last_applied_sequence: 0,
                latest_created_ns: 0,
                has_latest: 0,
                valid: 0,
            };
            assert_eq!(
                of_execution_checkpoint_store_integrity_report(root_c.as_ptr(), &mut report),
                of_error_t::OF_OK as i32
            );
            assert_eq!(report.valid, 1);
            assert_eq!(report.checkpoint_files, 2);
            assert_eq!(report.valid_checkpoints, 2);
            assert_eq!(report.invalid_checkpoints, 0);
            assert_eq!(report.latest_checkpoint_id, 2);
            assert_eq!(report.latest_last_applied_sequence, 20);
            assert_eq!(report.has_latest, 1);

            let mut bytes = std::fs::read(&manifest.path).expect("read checkpoint");
            let last = bytes.last_mut().expect("checkpoint byte");
            *last ^= 0x01;
            std::fs::write(&manifest.path, bytes).expect("write corrupt checkpoint");

            assert_eq!(
                of_execution_checkpoint_store_integrity_report(root_c.as_ptr(), &mut report),
                of_error_t::OF_OK as i32
            );
            assert_eq!(report.valid, 0);
            assert_eq!(report.checkpoint_files, 2);
            assert_eq!(report.valid_checkpoints, 1);
            assert_eq!(report.invalid_checkpoints, 1);
            assert_eq!(report.latest_checkpoint_id, 1);
            assert_eq!(report.latest_last_applied_sequence, 10);
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn execution_abi_create_multi_routes_submits_multiple_symbols() {
        let _guard = test_guard();
        let route = CString::new("SIM").expect("cstring");
        let account = CString::new("ACC").expect("cstring");
        let venue = CString::new("SIM").expect("cstring");
        let es = CString::new("ES").expect("cstring");
        let nq = CString::new("NQ").expect("cstring");
        let cfgs = [
            of_execution_route_config_t {
                route_id: route.as_ptr(),
                account_id: account.as_ptr(),
                venue: venue.as_ptr(),
                instrument: es.as_ptr(),
                enabled: 1,
                kill_switch: 0,
                max_order_qty: 100,
                max_order_notional: 1_000_000,
                max_open_orders: 10,
                max_open_notional: 10_000_000,
                price_band_ticks: 0,
            },
            of_execution_route_config_t {
                route_id: route.as_ptr(),
                account_id: account.as_ptr(),
                venue: venue.as_ptr(),
                instrument: nq.as_ptr(),
                enabled: 1,
                kill_switch: 0,
                max_order_qty: 100,
                max_order_notional: 1_000_000,
                max_open_orders: 10,
                max_open_notional: 10_000_000,
                price_band_ticks: 0,
            },
        ];

        let mut engine: *mut of_execution_engine = ptr::null_mut();
        assert_eq!(
            of_execution_engine_create_multi(cfgs.as_ptr(), cfgs.len() as u32, &mut engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());
        assert_eq!(
            of_execution_engine_start(engine),
            of_error_t::OF_OK as i32
        );

        let strategy = CString::new("STRAT").expect("cstring");
        let es_id = CString::new("ES-1").expect("cstring");
        let nq_id = CString::new("NQ-1").expect("cstring");
        let es_req = of_execution_order_request_t {
            client_order_id: es_id.as_ptr(),
            account_id: account.as_ptr(),
            route_id: route.as_ptr(),
            strategy_id: strategy.as_ptr(),
            venue: venue.as_ptr(),
            instrument: es.as_ptr(),
            side: 1,
            order_type: 2,
            time_in_force: 1,
            quantity: 10,
            limit_price: 5000,
            stop_price: 0,
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        };
        let nq_req = of_execution_order_request_t {
            client_order_id: nq_id.as_ptr(),
            account_id: account.as_ptr(),
            route_id: route.as_ptr(),
            strategy_id: strategy.as_ptr(),
            venue: venue.as_ptr(),
            instrument: nq.as_ptr(),
            side: 1,
            order_type: 2,
            time_in_force: 1,
            quantity: 10,
            limit_price: 5000,
            stop_price: 0,
            ts_exchange_ns: 1,
            ts_recv_ns: 3,
        };
        let mut events = [of_execution_event_t {
            exec_type: 0,
            order_status: 0,
            client_order_id: [0; 41],
            orig_client_order_id: [0; 41],
            venue_order_id: [0; 49],
            execution_id: [0; 49],
            account_id: [0; 33],
            route_id: [0; 33],
            venue: [0; 17],
            instrument: [0; 33],
            last_qty: 0,
            last_price: 0,
            cumulative_qty: 0,
            leaves_qty: 0,
            average_price: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
            reason: 0,
            text: [0; 129],
        }; 4];
        let mut len = events.len() as u32;
        assert_eq!(
            of_execution_submit_order(engine, &es_req, events.as_mut_ptr(), &mut len),
            of_error_t::OF_OK as i32
        );
        assert_eq!(len, 2);
        len = events.len() as u32;
        assert_eq!(
            of_execution_submit_order(engine, &nq_req, events.as_mut_ptr(), &mut len),
            of_error_t::OF_OK as i32
        );
        assert_eq!(len, 2);

        let mut metrics = of_execution_metrics_t {
            submitted: 0,
            cancelled: 0,
            amended: 0,
            events_applied: 0,
            risk_rejected: 0,
            adapter_errors: 0,
            recovered: 0,
        };
        assert_eq!(
            of_execution_metrics(engine, &mut metrics),
            of_error_t::OF_OK as i32
        );
        assert_eq!(metrics.submitted, 2);
        assert_eq!(metrics.events_applied, 4);

        of_execution_engine_destroy(engine);
    }

    #[test]
    fn execution_concurrent_abi_submits_and_reports() {
        let _guard = test_guard();
        let route = CString::new("SIM").expect("cstring");
        let account = CString::new("ACC").expect("cstring");
        let venue = CString::new("SIM").expect("cstring");
        let instrument = CString::new("ES").expect("cstring");
        let cfg = of_execution_route_config_t {
            route_id: route.as_ptr(),
            account_id: account.as_ptr(),
            venue: venue.as_ptr(),
            instrument: instrument.as_ptr(),
            enabled: 1,
            kill_switch: 0,
            max_order_qty: 100,
            max_order_notional: 1_000_000,
            max_open_orders: 10,
            max_open_notional: 10_000_000,
            price_band_ticks: 0,
        };
        let worker_cfg = of_execution_concurrent_config_t {
            command_capacity: 8,
            report_capacity: 8,
            event_buffer_capacity: 8,
        };

        let mut engine: *mut of_execution_concurrent_engine = ptr::null_mut();
        assert_eq!(
            of_execution_concurrent_engine_create_multi(&cfg, 1, &worker_cfg, &mut engine),
            of_error_t::OF_OK as i32
        );
        assert!(!engine.is_null());

        let client_order_id = CString::new("C1").expect("cstring");
        let strategy = CString::new("STRAT").expect("cstring");
        let req = of_execution_order_request_t {
            client_order_id: client_order_id.as_ptr(),
            account_id: account.as_ptr(),
            route_id: route.as_ptr(),
            strategy_id: strategy.as_ptr(),
            venue: venue.as_ptr(),
            instrument: instrument.as_ptr(),
            side: 1,
            order_type: 2,
            time_in_force: 1,
            quantity: 10,
            limit_price: 5000,
            stop_price: 0,
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        };
        let mut sequence = 0_u64;
        assert_eq!(
            of_execution_concurrent_submit_order(engine, &req, &mut sequence),
            of_error_t::OF_OK as i32
        );
        assert_eq!(sequence, 1);

        let mut report = of_execution_command_report_t {
            sequence: 0,
            kind: 0,
            result_code: 0,
            event_count: 0,
        };
        let mut events = [of_execution_event_t {
            exec_type: 0,
            order_status: 0,
            client_order_id: [0; 41],
            orig_client_order_id: [0; 41],
            venue_order_id: [0; 49],
            execution_id: [0; 49],
            account_id: [0; 33],
            route_id: [0; 33],
            venue: [0; 17],
            instrument: [0; 33],
            last_qty: 0,
            last_price: 0,
            cumulative_qty: 0,
            leaves_qty: 0,
            average_price: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
            reason: 0,
            text: [0; 129],
        }; 4];
        let mut len;
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut rc;
        loop {
            len = events.len() as u32;
            rc = of_execution_concurrent_try_recv_report(
                engine,
                &mut report,
                events.as_mut_ptr(),
                &mut len,
            );
            if rc == of_error_t::OF_OK as i32 || Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(rc, of_error_t::OF_OK as i32);
        assert_eq!(report.sequence, 1);
        assert_eq!(report.kind, 1);
        assert_eq!(report.result_code, of_error_t::OF_OK as i32);
        assert_eq!(report.event_count, 2);
        assert_eq!(len, 2);
        assert_eq!(events[0].exec_type, 1);
        assert_eq!(events[1].exec_type, 3);

        assert_eq!(
            of_execution_concurrent_stop(engine, &mut sequence),
            of_error_t::OF_OK as i32
        );
        of_execution_concurrent_engine_destroy(engine);
    }
}
