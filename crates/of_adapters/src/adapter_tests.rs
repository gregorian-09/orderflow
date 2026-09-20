#[cfg(test)]
mod tests {
    use super::*;

    struct LegacyStyleAdapter;

    impl MarketDataAdapter for LegacyStyleAdapter {
        fn connect(&mut self) -> AdapterResult<()> {
            Ok(())
        }

        fn subscribe(&mut self, _req: SubscribeReq) -> AdapterResult<()> {
            Ok(())
        }

        fn unsubscribe(&mut self, _symbol: SymbolId) -> AdapterResult<()> {
            Ok(())
        }

        fn poll(&mut self, _out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
            Ok(0)
        }

        fn health(&self) -> AdapterHealth {
            AdapterHealth::default()
        }
    }

    #[test]
    fn factory_returns_mock_by_default() {
        let cfg = AdapterConfig::default();
        let mut adapter = create_adapter(&cfg).expect("adapter should be created");
        adapter.connect().expect("connect should work");
        assert!(adapter.health().connected);
    }

    #[test]
    fn default_operational_status_preserves_existing_adapter_implementations() {
        let status = LegacyStyleAdapter.operational_status();
        assert_eq!(status.mode, AdapterRuntimeMode::Unknown);
        assert_eq!(status.connection_state, AdapterConnectionState::Unknown);
    }

    #[test]
    fn endpoint_redaction_omits_every_potential_secret_component() {
        assert_eq!(
            redact_adapter_endpoint(
                "WSS://user:secret@stream.example:9443/ws/listen-key?token=private#fragment"
            ),
            Some("wss://stream.example:9443".to_string())
        );
        assert_eq!(
            redact_adapter_endpoint("mock://local-provider/path"),
            Some("mock://local-provider".to_string())
        );
        assert_eq!(redact_adapter_endpoint("missing-scheme.example"), None);
        assert_eq!(redact_adapter_endpoint("1bad://example"), None);
        assert_eq!(redact_adapter_endpoint("wss://user:secret@/path"), None);
    }

    #[test]
    fn mock_operational_status_sorts_and_deduplicates_symbols() {
        let mut adapter = MockAdapter::default();
        adapter.connect().expect("connect");
        for (venue, symbol) in [("CME", "NQM6"), ("CME", "ESM6"), ("CME", "ESM6")] {
            adapter
                .subscribe(SubscribeReq {
                    symbol: SymbolId {
                        venue: venue.to_string(),
                        symbol: symbol.to_string(),
                    },
                    depth_levels: 10,
                })
                .expect("subscribe");
        }

        let status = adapter.operational_status();
        assert_eq!(status.subscription_count, 2);
        assert_eq!(status.subscribed_symbols[0].symbol, "ESM6");
        assert_eq!(status.subscribed_symbols[1].symbol, "NQM6");
        assert_eq!(status.connection_state, AdapterConnectionState::Streaming);
    }

    #[test]
    fn descriptors_cover_all_known_providers() {
        let descriptors = adapter_descriptors();
        assert_eq!(descriptors.len(), 4);
        assert_eq!(describe_adapter(ProviderKind::Mock).provider_id, "mock");
        assert_eq!(
            describe_adapter(ProviderKind::Rithmic).provider_id,
            "rithmic"
        );
        assert_eq!(describe_adapter(ProviderKind::Cqg).provider_id, "cqg");
        assert_eq!(
            describe_adapter(ProviderKind::Binance).provider_id,
            "binance"
        );
        let binance = describe_adapter(ProviderKind::Binance);
        assert_eq!(binance.supports_raw_capture, cfg!(feature = "binance"));
        assert_eq!(binance.supports_fixture_replay, cfg!(feature = "binance"));
        assert_eq!(binance.supports_backpressure, cfg!(feature = "binance"));
        assert_eq!(binance.supports_stale_detection, cfg!(feature = "binance"));
        assert_eq!(binance.supports_latency_metrics, cfg!(feature = "binance"));
    }

    #[test]
    fn compiled_descriptors_match_feature_enabled_helper() {
        for descriptor in adapter_descriptors() {
            assert_eq!(
                descriptor.compiled,
                adapter_feature_enabled(descriptor.provider.clone())
            );
        }
        assert!(adapter_feature_enabled(ProviderKind::Mock));
        assert!(compiled_adapter_descriptors()
            .iter()
            .any(|descriptor| descriptor.provider == ProviderKind::Mock));
    }

    #[test]
    fn adapter_quality_ladder_preserves_production_distinctions() {
        assert!(AdapterQualityLevel::ProductionObserved.meets(AdapterQualityLevel::Certified));
        assert!(AdapterQualityLevel::Certified.meets(AdapterQualityLevel::ProductionCandidate));
        assert!(AdapterQualityLevel::PaperTrading.meets(AdapterQualityLevel::Functional));
        assert!(!AdapterQualityLevel::Functional.meets(AdapterQualityLevel::PaperTrading));
        assert!(!AdapterQualityLevel::Simulation.meets(AdapterQualityLevel::Scaffold));
    }

    #[test]
    fn conformance_report_accepts_mock_simulation_target() {
        let report =
            adapter_conformance_report(ProviderKind::Mock, AdapterQualityLevel::Simulation);
        assert!(report.passed(), "{report:?}");
        assert_eq!(report.checked_requirements, 4);
        assert_eq!(report.provider_id, "mock");
    }

    #[test]
    fn conformance_report_rejects_uncertified_production_claims() {
        let report =
            adapter_conformance_report(ProviderKind::Binance, AdapterQualityLevel::Certified);
        assert!(!report.passed());
        assert!(report.failures.iter().any(|failure| {
            failure.requirement == AdapterConformanceRequirement::CertificationEvidence
        }));
        assert!(report.failures.iter().any(|failure| {
            failure.requirement == AdapterConformanceRequirement::AdvertisedQuality
        }));
    }

    #[test]
    fn conformance_report_identifies_disabled_live_provider() {
        let report =
            adapter_conformance_report(ProviderKind::Rithmic, AdapterQualityLevel::Functional);
        if cfg!(feature = "rithmic") {
            assert!(report
                .failures
                .iter()
                .all(|failure| { failure.requirement != AdapterConformanceRequirement::Compiled }));
        } else {
            assert!(report
                .failures
                .iter()
                .any(|failure| { failure.requirement == AdapterConformanceRequirement::Compiled }));
        }
    }

    #[cfg(any(feature = "binance", feature = "cqg", feature = "rithmic"))]
    #[test]
    fn tls_arguments_enforce_hostname_verification_without_file_configuration() {
        let args = TlsFileConfig::default().openssl_args("feed.example.test", 443);

        assert!(args.windows(2).any(|pair| {
            pair == [
                "-verify_hostname".to_string(),
                "feed.example.test".to_string(),
            ]
        }));
        assert!(args
            .windows(2)
            .any(|pair| { pair == ["-connect".to_string(), "feed.example.test:443".to_string()] }));
        assert!(args.contains(&"-verify_return_error".to_string()));
        assert!(!args.iter().any(|arg| arg == "-CAfile"));
        assert!(!args.iter().any(|arg| arg == "-cert"));
        assert!(!args.iter().any(|arg| arg == "-key"));
    }

    #[cfg(any(feature = "binance", feature = "cqg", feature = "rithmic"))]
    #[test]
    fn tls_arguments_keep_credentials_as_paths_or_environment_references() {
        let config = TlsFileConfig {
            ca_file: Some("/run/secrets/venue-ca.pem".to_string()),
            client_cert_file: Some("/run/secrets/client.pem".to_string()),
            client_chain_file: Some("/run/secrets/chain.pem".to_string()),
            client_key_file: Some("/run/secrets/client-key.pem".to_string()),
            client_key_password_env: Some("CLIENT_KEY_PASSWORD".to_string()),
        };

        let args = config.openssl_args("feed.example.test", 443);

        assert!(args.windows(2).any(|pair| {
            pair == [
                "-CAfile".to_string(),
                "/run/secrets/venue-ca.pem".to_string(),
            ]
        }));
        assert!(args
            .windows(2)
            .any(|pair| { pair == ["-cert".to_string(), "/run/secrets/client.pem".to_string()] }));
        assert!(args.windows(2).any(|pair| {
            pair == [
                "-cert_chain".to_string(),
                "/run/secrets/chain.pem".to_string(),
            ]
        }));
        assert!(args.windows(2).any(|pair| {
            pair == [
                "-key".to_string(),
                "/run/secrets/client-key.pem".to_string(),
            ]
        }));
        assert!(args.windows(2).any(|pair| {
            pair == ["-passin".to_string(), "env:CLIENT_KEY_PASSWORD".to_string()]
        }));
        assert!(!args.iter().any(|arg| arg == "actual-secret-value"));
    }

    #[cfg(not(feature = "rithmic"))]
    #[test]
    fn factory_rejects_disabled_provider_features() {
        let cfg = AdapterConfig {
            provider: ProviderKind::Rithmic,
            ..AdapterConfig::default()
        };
        match create_adapter(&cfg) {
            Err(AdapterError::FeatureDisabled(_)) => {}
            Err(other) => panic!("unexpected error variant: {other}"),
            Ok(_) => panic!("expected feature-disabled error"),
        }
    }
}
