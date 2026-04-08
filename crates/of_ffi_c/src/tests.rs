#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;
    use std::sync::{Mutex, OnceLock};

    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
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
    fn analytics_snapshot_matches_golden_payload() {
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn session_candle_snapshot_matches_golden_payload() {
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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
            "{\"health_seq\":1,\"started\":true,\"connected\":true,\"degraded\":false,\"reconnect_state\":\"streaming\",\"quality_flags\":0,\"quality_flags_detail\":[],\"last_error\":null,\"protocol_info\":\"mock_adapter\",\"tracked_symbols\":0,\"processed_events\":0,\"external_feed_enabled\":false,\"external_feed_reconnecting\":false,\"external_sequence_enforced\":true,\"external_last_ingest_ns\":null}"
        );

        assert_eq!(of_unsubscribe(sub), of_error_t::OF_OK as i32);
        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn health_stream_emits_on_state_change_only() {
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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
        of_string_free(out);

        assert_eq!(of_engine_stop(engine), of_error_t::OF_OK as i32);
        of_engine_destroy(engine);
    }

    #[test]
    fn health_stream_stops_after_unsubscribe() {
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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
    fn ingest_book_rejects_invalid_side() {
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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
        let _guard = test_lock().lock().expect("lock");

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

    #[test]
    fn derived_analytics_stream_emits_session_snapshot_payload() {
        let _guard = test_lock().lock().expect("lock");

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
}
