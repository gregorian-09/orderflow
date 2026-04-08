#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use of_adapters::{AdapterConfig, CredentialsRef, MockAdapter, ProviderKind, RawEvent};
    use of_core::{BookAction, BookUpdate, DataQualityFlags, Side, SignalState, SymbolId, TradePrint};
    use of_signals::DeltaMomentumSignal;

    use super::*;

    #[test]
    fn engine_processes_trade_and_updates_snapshots() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        let mut adapter = MockAdapter::default();
        adapter.push_event(RawEvent::Trade(TradePrint {
            symbol: symbol.clone(),
            price: 505000,
            size: 10,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }));

        let mut engine = Engine::new(
            EngineConfig::default(),
            adapter,
            DeltaMomentumSignal::new(5),
        );

        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine.poll_once(DataQualityFlags::NONE).expect("poll failed");

        let analytics = engine.analytics_snapshot(&symbol).expect("analytics missing");
        assert_eq!(analytics.delta, 10);

        let signal = engine.signal_snapshot(&symbol).expect("signal missing");
        assert_eq!(signal.state, SignalState::LongBias);
    }

    #[test]
    fn engine_ingests_external_events_and_updates_snapshots() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );

        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine
            .ingest_book(
                BookUpdate {
                    symbol: symbol.clone(),
                    side: Side::Bid,
                    level: 0,
                    price: 504900,
                    size: 20,
                    action: BookAction::Upsert,
                    sequence: 1,
                    ts_exchange_ns: 10,
                    ts_recv_ns: 11,
                },
                DataQualityFlags::NONE,
            )
            .expect("book ingest failed");
        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505000,
                    size: 7,
                    aggressor_side: Side::Ask,
                    sequence: 2,
                    ts_exchange_ns: 12,
                    ts_recv_ns: 13,
                },
                DataQualityFlags::ADAPTER_DEGRADED,
            )
            .expect("trade ingest failed");

        let analytics = engine.analytics_snapshot(&symbol).expect("analytics missing");
        assert_eq!(analytics.delta, 7);
        let signal = engine.signal_snapshot(&symbol).expect("signal missing");
        assert_eq!(signal.state, SignalState::Blocked);
        assert_eq!(signal.quality_flags, DataQualityFlags::ADAPTER_DEGRADED.bits());
        assert_eq!(signal.reason, "blocked_by_quality_gate");
        assert_eq!(engine.last_events().len(), 1);
    }

    #[test]
    fn engine_materializes_book_snapshot_from_updates() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );

        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine
            .ingest_book(
                BookUpdate {
                    symbol: symbol.clone(),
                    side: Side::Bid,
                    level: 0,
                    price: 504900,
                    size: 20,
                    action: BookAction::Upsert,
                    sequence: 1,
                    ts_exchange_ns: 10,
                    ts_recv_ns: 11,
                },
                DataQualityFlags::NONE,
            )
            .expect("bid ingest failed");
        engine
            .ingest_book(
                BookUpdate {
                    symbol: symbol.clone(),
                    side: Side::Ask,
                    level: 1,
                    price: 505100,
                    size: 12,
                    action: BookAction::Upsert,
                    sequence: 2,
                    ts_exchange_ns: 12,
                    ts_recv_ns: 13,
                },
                DataQualityFlags::NONE,
            )
            .expect("ask ingest failed");
        engine
            .ingest_book(
                BookUpdate {
                    symbol: symbol.clone(),
                    side: Side::Bid,
                    level: 0,
                    price: 0,
                    size: 0,
                    action: BookAction::Delete,
                    sequence: 3,
                    ts_exchange_ns: 14,
                    ts_recv_ns: 15,
                },
                DataQualityFlags::NONE,
            )
            .expect("delete ingest failed");

        let snapshot = engine.book_snapshot(&symbol).expect("book snapshot missing");
        assert!(snapshot.bids.is_empty());
        assert_eq!(snapshot.asks.len(), 1);
        assert_eq!(snapshot.asks[0].level, 1);
        assert_eq!(snapshot.asks[0].price, 505100);
        assert_eq!(snapshot.asks[0].size, 12);
        assert_eq!(snapshot.last_sequence, 3);
        assert_eq!(snapshot.ts_exchange_ns, 14);
        assert_eq!(snapshot.ts_recv_ns, 15);
        assert!(engine.metrics_json().contains("\"symbols\":1"));
    }

    #[test]
    fn engine_exposes_derived_analytics_snapshot() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );

        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505000,
                    size: 10,
                    aggressor_side: Side::Ask,
                    sequence: 1,
                    ts_exchange_ns: 1,
                    ts_recv_ns: 2,
                },
                DataQualityFlags::NONE,
            )
            .expect("trade 1");
        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 504900,
                    size: 5,
                    aggressor_side: Side::Bid,
                    sequence: 2,
                    ts_exchange_ns: 3,
                    ts_recv_ns: 4,
                },
                DataQualityFlags::NONE,
            )
            .expect("trade 2");

        let derived = engine
            .derived_analytics_snapshot(&symbol)
            .expect("derived analytics missing");
        assert_eq!(derived.total_volume, 15);
        assert_eq!(derived.trade_count, 2);
        assert_eq!(derived.average_trade_size, 7);
        assert_eq!(derived.vwap, 504966);
        assert_eq!(derived.imbalance_bps, 3333);
    }

    #[test]
    fn engine_exposes_session_candle_snapshot() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );

        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505000,
                    size: 10,
                    aggressor_side: Side::Ask,
                    sequence: 1,
                    ts_exchange_ns: 10,
                    ts_recv_ns: 11,
                },
                DataQualityFlags::NONE,
            )
            .expect("trade 1");
        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 504900,
                    size: 5,
                    aggressor_side: Side::Bid,
                    sequence: 2,
                    ts_exchange_ns: 20,
                    ts_recv_ns: 21,
                },
                DataQualityFlags::NONE,
            )
            .expect("trade 2");

        let candle = engine
            .session_candle_snapshot(&symbol)
            .expect("session candle missing");
        assert_eq!(candle.open, 505000);
        assert_eq!(candle.high, 505000);
        assert_eq!(candle.low, 504900);
        assert_eq!(candle.close, 504900);
        assert_eq!(candle.trade_count, 2);
        assert_eq!(candle.first_ts_exchange_ns, 10);
        assert_eq!(candle.last_ts_exchange_ns, 20);
    }

    #[test]
    fn engine_exposes_interval_candle_snapshot() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );

        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        for (price, size, side, seq, ts) in [
            (505000, 10, Side::Ask, 1u64, 10u64),
            (504900, 5, Side::Bid, 2u64, 40u64),
            (505100, 8, Side::Ask, 3u64, 100u64),
        ] {
            engine
                .ingest_trade(
                    TradePrint {
                        symbol: symbol.clone(),
                        price,
                        size,
                        aggressor_side: side,
                        sequence: seq,
                        ts_exchange_ns: ts,
                        ts_recv_ns: ts + 1,
                    },
                    DataQualityFlags::NONE,
                )
                .expect("trade ingest");
        }

        let candle = engine
            .interval_candle_snapshot(&symbol, 70)
            .expect("interval candle missing");
        assert_eq!(candle.window_ns, 70);
        assert_eq!(candle.open, 504900);
        assert_eq!(candle.high, 505100);
        assert_eq!(candle.low, 504900);
        assert_eq!(candle.close, 505100);
        assert_eq!(candle.trade_count, 2);
        assert_eq!(candle.total_volume, 13);
        assert_eq!(candle.vwap, 505023);
        assert_eq!(candle.first_ts_exchange_ns, 40);
        assert_eq!(candle.last_ts_exchange_ns, 100);
    }

    #[test]
    fn external_supervisor_sets_sequence_and_order_flags() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );
        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine
            .configure_external_feed(ExternalFeedPolicy {
                stale_after_ms: 0,
                enforce_sequence: true,
            })
            .expect("configure external feed");

        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505000,
                    size: 1,
                    aggressor_side: Side::Ask,
                    sequence: 1,
                    ts_exchange_ns: 1,
                    ts_recv_ns: 1,
                },
                DataQualityFlags::NONE,
            )
            .expect("ingest seq1");
        let s1 = engine.signal_snapshot(&symbol).expect("signal 1");
        assert_eq!(s1.quality_flags, 0);

        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505001,
                    size: 1,
                    aggressor_side: Side::Ask,
                    sequence: 3,
                    ts_exchange_ns: 2,
                    ts_recv_ns: 2,
                },
                DataQualityFlags::NONE,
            )
            .expect("ingest seq3");
        let s2 = engine.signal_snapshot(&symbol).expect("signal 2");
        assert!(s2.quality_flags & DataQualityFlags::SEQUENCE_GAP.bits() != 0);

        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505002,
                    size: 1,
                    aggressor_side: Side::Ask,
                    sequence: 2,
                    ts_exchange_ns: 3,
                    ts_recv_ns: 3,
                },
                DataQualityFlags::NONE,
            )
            .expect("ingest seq2");
        let s3 = engine.signal_snapshot(&symbol).expect("signal 3");
        assert!(s3.quality_flags & DataQualityFlags::OUT_OF_ORDER.bits() != 0);
    }

    #[test]
    fn external_supervisor_reconnecting_and_stale_flags_affect_health() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );
        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine
            .configure_external_feed(ExternalFeedPolicy {
                stale_after_ms: 1,
                enforce_sequence: true,
            })
            .expect("configure external feed");

        engine
            .set_external_reconnecting(true)
            .expect("set reconnecting true");
        let degraded = engine.health_json();
        assert!(degraded.contains(&format!(
            "\"quality_flags\":{}",
            DataQualityFlags::ADAPTER_DEGRADED.bits()
        )));
        assert!(degraded.contains("\"quality_flags_detail\":[\"ADAPTER_DEGRADED\"]"));
        assert!(degraded.contains("\"external_feed_reconnecting\":true"));

        engine
            .set_external_reconnecting(false)
            .expect("set reconnecting false");
        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505000,
                    size: 1,
                    aggressor_side: Side::Ask,
                    sequence: 1,
                    ts_exchange_ns: 1,
                    ts_recv_ns: 1,
                },
                DataQualityFlags::NONE,
            )
            .expect("ingest");
        std::thread::sleep(std::time::Duration::from_millis(3));
        engine.external_health_tick().expect("health tick");
        let stale = engine.health_json();
        assert!(stale.contains(&format!(
            "\"quality_flags\":{}",
            DataQualityFlags::STALE_FEED.bits()
        )));
        assert!(stale.contains("\"quality_flags_detail\":[\"STALE_FEED\"]"));
        assert!(stale.contains("\"external_last_ingest_ns\":"));
    }

    #[test]
    fn default_builder_wires_mock_provider() {
        let cfg = EngineConfig {
            adapter: AdapterConfig {
                provider: ProviderKind::Mock,
                ..AdapterConfig::default()
            },
            ..EngineConfig::default()
        };
        let mut engine = build_default_engine(cfg).expect("build should work");
        engine.start().expect("start should work");
        let metrics = engine.metrics_json();
        assert!(metrics.contains("\"started\":true"));
        assert!(metrics.contains("\"adapter_protocol_info\""));
        assert!(metrics.contains("\"health_seq\":"));
        assert!(metrics.contains("\"quality_flags_detail\":"));
    }

    #[test]
    fn health_and_metrics_include_additive_observability_fields() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );

        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine
            .configure_external_feed(ExternalFeedPolicy {
                stale_after_ms: 15_000,
                enforce_sequence: true,
            })
            .expect("configure external feed");
        engine
            .ingest_trade(
                TradePrint {
                    symbol,
                    price: 505000,
                    size: 2,
                    aggressor_side: Side::Ask,
                    sequence: 1,
                    ts_exchange_ns: 11,
                    ts_recv_ns: 12,
                },
                DataQualityFlags::NONE,
            )
            .expect("ingest trade");

        let health = engine.health_json();
        assert!(health.contains("\"tracked_symbols\":1"));
        assert!(health.contains("\"processed_events\":1"));
        assert!(health.contains("\"external_feed_enabled\":true"));
        assert!(health.contains("\"external_sequence_enforced\":true"));
        assert!(health.contains("\"quality_flags_detail\":[]"));

        let metrics = engine.metrics_json();
        assert!(metrics.contains("\"book_symbols\":0"));
        assert!(metrics.contains("\"analytics_symbols\":1"));
        assert!(metrics.contains("\"signal_symbols\":1"));
        assert!(metrics.contains("\"external_trade_sequence_symbols\":1"));
        assert!(metrics.contains("\"external_book_sequence_symbols\":0"));
        assert!(metrics.contains("\"external_last_ingest_ns\":"));
    }

    #[test]
    fn parses_toml_file_config() {
        let path = write_temp_file(
            "runtime_cfg.toml",
            r#"
instance_id = "from_toml"
enable_persistence = true
signal_threshold = 250
provider = "mock"
data_root = "data_local"
audit_log_path = "audit/local.log"
"#,
        );

        let cfg = load_engine_config_from_path(path.to_str().expect("valid path"))
            .expect("toml parse should work");
        assert_eq!(cfg.instance_id, "from_toml");
        assert!(cfg.enable_persistence);
        assert_eq!(cfg.signal_threshold, 250);
        assert_eq!(cfg.data_root, "data_local");
        assert_eq!(cfg.audit_log_path, "audit/local.log");
        assert!(matches!(cfg.adapter.provider, ProviderKind::Mock));
    }

    #[test]
    fn validates_non_mock_requires_env_refs() {
        let cfg = EngineConfig {
            adapter: AdapterConfig {
                provider: ProviderKind::Cqg,
                endpoint: Some("cqg://example".to_string()),
                credentials: Some(CredentialsRef {
                    key_id_env: "OF_TEST_MISSING_KEY".to_string(),
                    secret_env: "OF_TEST_MISSING_SECRET".to_string(),
                }),
                ..AdapterConfig::default()
            },
            ..EngineConfig::default()
        };

        let err = validate_startup_config(&cfg).expect_err("missing env vars should fail");
        assert!(format!("{err}").contains("missing required env var"));
    }

    #[test]
    fn parses_binance_provider_without_credentials() {
        let path = write_temp_file(
            "runtime_cfg_binance.toml",
            r#"
instance_id = "from_toml_binance"
provider = "binance"
endpoint = "mock://binance"
"#,
        );
        let cfg = load_engine_config_from_path(path.to_str().expect("valid path"))
            .expect("toml parse should work");
        assert!(matches!(cfg.adapter.provider, ProviderKind::Binance));
        validate_startup_config(&cfg).expect("binance should not require creds");
    }

    #[test]
    fn parses_nested_toml_config_strictly() {
        let path = write_temp_file(
            "runtime_cfg_nested.toml",
            r#"
instance_id = "strict_toml"
enable_persistence = true
signal_threshold = 300
data_root = "strict_data"
audit_log_path = "audit/strict.log"
audit_redact_tokens = ["secret", "token"]

[adapter]
provider = "cqg"
endpoint = "wss://demoapi.cqg.com/feed"
app_name = "strict-runtime"

[adapter.credentials]
key_id_env = "OF_STRICT_KEY"
secret_env = "OF_STRICT_SECRET"
"#,
        );

        let cfg = load_engine_config_from_path(path.to_str().expect("valid path"))
            .expect("strict toml parse should work");
        assert_eq!(cfg.instance_id, "strict_toml");
        assert!(cfg.enable_persistence);
        assert_eq!(cfg.signal_threshold, 300);
        assert_eq!(cfg.audit_redact_tokens, vec!["secret", "token"]);
        assert!(matches!(cfg.adapter.provider, ProviderKind::Cqg));
        assert_eq!(
            cfg.adapter.endpoint.as_deref(),
            Some("wss://demoapi.cqg.com/feed")
        );
        assert_eq!(cfg.adapter.app_name.as_deref(), Some("strict-runtime"));
        let creds = cfg.adapter.credentials.expect("credentials");
        assert_eq!(creds.key_id_env, "OF_STRICT_KEY");
        assert_eq!(creds.secret_env, "OF_STRICT_SECRET");

        let report = load_engine_config_report_from_path(path.to_str().expect("valid path"))
            .expect("strict report should work");
        assert_eq!(report.format, "toml");
        assert_eq!(report.compatibility_mode, ConfigCompatibilityMode::Strict);
        assert!(!report.used_legacy_fallback());
        assert!(report.warning.is_none());
    }

    #[test]
    fn parses_nested_json_config_strictly() {
        let path = write_temp_file(
            "runtime_cfg_nested.json",
            r#"{
  "instance_id": "strict_json",
  "signal_threshold": 175,
  "audit_redact_tokens": ["secret", "password"],
  "adapter": {
    "provider": "binance",
    "endpoint": "wss://stream.binance.com:9443/ws",
    "app_name": "strict-json-runtime"
  }
}"#,
        );

        let cfg = load_engine_config_from_path(path.to_str().expect("valid path"))
            .expect("strict json parse should work");
        assert_eq!(cfg.instance_id, "strict_json");
        assert_eq!(cfg.signal_threshold, 175);
        assert_eq!(cfg.audit_redact_tokens, vec!["secret", "password"]);
        assert!(matches!(cfg.adapter.provider, ProviderKind::Binance));
        assert_eq!(
            cfg.adapter.endpoint.as_deref(),
            Some("wss://stream.binance.com:9443/ws")
        );
        assert_eq!(cfg.adapter.app_name.as_deref(), Some("strict-json-runtime"));

        let report = load_engine_config_report_from_path(path.to_str().expect("valid path"))
            .expect("strict report should work");
        assert_eq!(report.format, "json");
        assert_eq!(report.compatibility_mode, ConfigCompatibilityMode::Strict);
        assert!(!report.used_legacy_fallback());
        assert!(report.warning.is_none());
    }

    #[test]
    fn legacy_fallback_still_accepts_flat_json_like_shape() {
        let path = write_temp_file(
            "runtime_cfg_legacy.json",
            "{\n\"instance_id\": \"legacy_json\",\n\"provider\": \"mock\",\n\"signal_threshold\": \"250\",\n\"audit_redact_tokens\": \"secret,token\"\n}\n",
        );

        let cfg = load_engine_config_from_path(path.to_str().expect("valid path"))
            .expect("legacy fallback should still work");
        assert_eq!(cfg.instance_id, "legacy_json");
        assert!(matches!(cfg.adapter.provider, ProviderKind::Mock));
        assert_eq!(cfg.signal_threshold, 250);
        assert_eq!(cfg.audit_redact_tokens, vec!["secret", "token"]);

        let report = load_engine_config_report_from_path(path.to_str().expect("valid path"))
            .expect("legacy report should still work");
        assert_eq!(report.format, "json");
        assert_eq!(report.compatibility_mode, ConfigCompatibilityMode::LegacyFallback);
        assert!(report.used_legacy_fallback());
        assert!(report.warning.as_deref().unwrap_or("").contains("legacy json fallback"));
    }

    #[test]
    fn audit_log_rotates_and_redacts() {
        let base = temp_dir("audit_rotate");
        let audit_path = base.join("audit.log");
        let data_root = base.join("data");

        let mut engine = build_default_engine(EngineConfig {
            instance_id: "audit-test".to_string(),
            enable_persistence: false,
            data_root: data_root.to_string_lossy().to_string(),
            audit_log_path: audit_path.to_string_lossy().to_string(),
            audit_max_bytes: 180,
            audit_max_files: 2,
            audit_redact_tokens: vec!["token".to_string()],
            data_retention_max_bytes: 1024,
            data_retention_max_age_secs: 60,
            adapter: AdapterConfig::default(),
            signal_threshold: 100,
        })
        .expect("engine build should work");

        engine.start().expect("start should work");
        for i in 0..6 {
            engine
                .subscribe(
                    SymbolId {
                        venue: "CME".to_string(),
                        symbol: format!("ES_token_{i}"),
                    },
                    10,
                )
                .expect("subscribe should work");
        }
        engine.stop();

        let current = fs::read_to_string(&audit_path).expect("current audit must exist");
        assert!(current.contains("[REDACTED]"));
        assert!(rotated_path(&audit_path, 1).exists());
    }

    #[test]
    fn reset_symbol_session_clears_analytics() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut adapter = MockAdapter::default();
        adapter.push_event(RawEvent::Trade(TradePrint {
            symbol: symbol.clone(),
            price: 505000,
            size: 10,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }));

        let mut engine = Engine::new(
            EngineConfig::default(),
            adapter,
            DeltaMomentumSignal::new(5),
        );
        engine.start().expect("start");
        engine.subscribe(symbol.clone(), 10).expect("subscribe");
        engine.poll_once(DataQualityFlags::NONE).expect("poll");
        let pre = engine.analytics_snapshot(&symbol).expect("pre");
        assert!(pre.cumulative_delta > 0);

        engine
            .reset_symbol_session(symbol.clone())
            .expect("reset session");
        let post = engine.analytics_snapshot(&symbol).expect("post");
        assert_eq!(post.delta, 0);
        assert_eq!(post.cumulative_delta, 0);
        assert_eq!(post.point_of_control, 0);
    }

    fn write_temp_file(name: &str, content: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nonce = format!(
            "{}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock ok")
                .as_nanos(),
            name
        );
        path.push(nonce);
        fs::write(&path, content).expect("temp file write should work");
        path
    }

    fn temp_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{}_{}_{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock ok")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("temp dir create should work");
        path
    }
}
