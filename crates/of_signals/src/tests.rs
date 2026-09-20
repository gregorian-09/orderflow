#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(state: SignalState, confidence_bps: u16) -> SignalSnapshot {
        SignalSnapshot {
            module_id: "test_signal_v1",
            state,
            confidence_bps,
            quality_flags: 0,
            reason: "test".to_string(),
        }
    }

    fn outcome_record(
        confidence_bps: u16,
        markout_direction: SignalMarkoutDirection,
        correct: Option<bool>,
    ) -> SignalOutcomeRecord {
        SignalOutcomeRecord {
            module_id: "test_signal_v1",
            state: SignalState::LongBias,
            confidence_bps,
            calibrated_confidence_bps: confidence_bps,
            predicted_direction: Some(SignalMarkoutDirection::Up),
            markout_direction,
            correct,
            regime: None,
        }
    }

    #[test]
    fn signal_explanation_json_exports_audit_payload() {
        let explanation = SignalExplanation::new(
            "test_signal_v1",
            SignalState::LongBias,
            7_500,
            DataQualityFlags::SEQUENCE_GAP.bits(),
            SignalReasonCode::DeltaMomentumPositive,
            "delta_above_threshold",
        )
        .with_input(SignalInputValue::integer("delta", 125))
        .with_threshold(SignalThreshold::integer("threshold", 100))
        .with_confidence_component(SignalConfidenceComponent::new("base", 7_500));

        let json = explanation.to_json();
        assert!(json.contains("\"module_id\":\"test_signal_v1\""));
        assert!(json.contains("\"state\":\"long_bias\""));
        assert!(json.contains("\"reason_code\":\"delta_momentum_positive\""));
        assert!(json.contains("\"inputs\":[{\"name\":\"delta\",\"value\":125}]"));
        assert!(json.contains("\"thresholds\":[{\"name\":\"threshold\",\"value\":100}]"));
        assert!(json.contains("\"confidence_components\":[{\"name\":\"base\",\"value_bps\":7500}]"));

        struct SnapshotOnlySignal;
        impl SignalModule for SnapshotOnlySignal {
            fn on_analytics(&mut self, _ev: &AnalyticsSnapshot) {}

            fn snapshot(&self) -> SignalSnapshot {
                snapshot(SignalState::Neutral, 0)
            }

            fn quality_gate(&self, _q: DataQualityFlags) -> SignalGateDecision {
                SignalGateDecision::Pass
            }
        }

        assert!(SnapshotOnlySignal.latest_explanation().is_none());
        assert!(DeltaMomentumSignal::default()
            .latest_explanation()
            .is_some());
    }

    #[test]
    fn blocks_on_quality_issues() {
        let s = DeltaMomentumSignal::default();
        let decision = s.quality_gate(DataQualityFlags::SEQUENCE_GAP);
        assert_eq!(decision, SignalGateDecision::Block);
    }

    #[test]
    fn volume_imbalance_signal_uses_session_totals() {
        let mut s = VolumeImbalanceSignal::new(10);
        s.on_analytics(&AnalyticsSnapshot {
            buy_volume: 30,
            sell_volume: 15,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "volume_imbalance_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
    }

    #[test]
    fn cumulative_delta_signal_uses_cumulative_threshold() {
        let mut s = CumulativeDeltaSignal::new(20);
        s.on_analytics(&AnalyticsSnapshot {
            cumulative_delta: -25,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "cumulative_delta_v1");
        assert_eq!(snapshot.state, SignalState::ShortBias);
    }

    #[test]
    fn absorption_signal_detects_failed_sell_push() {
        let mut s = AbsorptionSignal::new(20, 1);
        s.on_analytics(&AnalyticsSnapshot {
            delta: -25,
            last_price: 100,
            point_of_control: 100,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "absorption_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
    }

    #[test]
    fn exhaustion_signal_detects_failed_buy_follow_through() {
        let mut s = ExhaustionSignal::new(20);
        s.on_analytics(&AnalyticsSnapshot {
            delta: 25,
            last_price: 100,
            point_of_control: 101,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "exhaustion_v1");
        assert_eq!(snapshot.state, SignalState::ShortBias);
    }

    #[test]
    fn sweep_signal_detects_value_area_break() {
        let mut s = SweepDetectionSignal::new(20, 1);
        s.on_analytics(&AnalyticsSnapshot {
            delta: 30,
            last_price: 106,
            value_area_high: 104,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "sweep_detection_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
    }

    #[test]
    fn composite_signal_aggregates_child_votes() {
        let mut s = CompositeSignal::new(vec![
            Box::new(DeltaMomentumSignal::new(10)),
            Box::new(VolumeImbalanceSignal::new(10)),
            Box::new(CumulativeDeltaSignal::new(10)),
        ]);
        s.on_analytics(&AnalyticsSnapshot {
            delta: 15,
            cumulative_delta: 20,
            buy_volume: 30,
            sell_volume: 10,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "composite_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
    }

    #[test]
    fn reason_codes_have_stable_string_values() {
        assert_eq!(
            SignalReasonCode::DeltaMomentumPositive.as_str(),
            "delta_momentum_positive"
        );
        assert_eq!(
            SignalReasonCode::BuyVolumeImbalance.as_str(),
            "buy_volume_imbalance"
        );
        assert_eq!(
            SignalReasonCode::from(SignalSuppressionReason::CooldownActive),
            SignalReasonCode::StabilizerCooldownActive
        );
    }

    #[test]
    fn delta_momentum_explanation_reports_inputs_and_threshold() {
        let mut signal = DeltaMomentumSignal::new(10);
        signal.on_analytics(&AnalyticsSnapshot {
            delta: 15,
            ..Default::default()
        });

        let explanation = signal.explanation();
        assert_eq!(explanation.module_id, "delta_momentum_v1");
        assert_eq!(explanation.state, SignalState::LongBias);
        assert_eq!(
            explanation.reason_code,
            SignalReasonCode::DeltaMomentumPositive
        );
        assert_eq!(
            explanation.inputs,
            vec![SignalInputValue::integer("delta", 15)]
        );
        assert_eq!(
            explanation.thresholds,
            vec![SignalThreshold::integer("threshold", 10)]
        );
    }

    #[test]
    fn absorption_explanation_reports_decision_context() {
        let mut signal = AbsorptionSignal::new(20, 2);
        signal.on_analytics(&AnalyticsSnapshot {
            delta: -25,
            last_price: 100,
            point_of_control: 101,
            ..Default::default()
        });

        let explanation = signal.explanation();
        assert_eq!(
            explanation.reason_code,
            SignalReasonCode::SellAbsorptionDetected
        );
        assert!(explanation
            .inputs
            .contains(&SignalInputValue::integer("poc_distance", 1)));
        assert!(explanation
            .thresholds
            .contains(&SignalThreshold::integer("price_band", 2)));
    }

    #[test]
    fn composite_explanation_reports_vote_counts() {
        let mut signal = CompositeSignal::new(vec![
            Box::new(DeltaMomentumSignal::new(10)),
            Box::new(VolumeImbalanceSignal::new(10)),
            Box::new(CumulativeDeltaSignal::new(100)),
        ]);
        signal.on_analytics(&AnalyticsSnapshot {
            delta: 20,
            buy_volume: 30,
            sell_volume: 10,
            cumulative_delta: 50,
            ..Default::default()
        });

        let explanation = signal.explanation();
        assert_eq!(
            explanation.reason_code,
            SignalReasonCode::CompositeLongMajority
        );
        assert!(explanation
            .inputs
            .contains(&SignalInputValue::integer("module_count", 3)));
        assert!(explanation
            .inputs
            .contains(&SignalInputValue::integer("long_votes", 2)));
        assert!(explanation
            .inputs
            .contains(&SignalInputValue::integer("short_votes", 0)));
    }

    #[test]
    fn transition_only_explanation_mode_emits_on_state_change() {
        let previous = snapshot(SignalState::Neutral, 500);
        let current_same = snapshot(SignalState::Neutral, 500);
        let current_changed = snapshot(SignalState::LongBias, 600);

        assert!(SignalExplanationMode::Always.should_emit_snapshot(Some(&previous), &current_same));
        assert!(!SignalExplanationMode::TransitionsOnly
            .should_emit_snapshot(Some(&previous), &current_same));
        assert!(SignalExplanationMode::TransitionsOnly
            .should_emit_snapshot(Some(&previous), &current_changed));
        assert!(SignalExplanationMode::TransitionsOnly.should_emit_snapshot(None, &current_same));
    }

    #[test]
    fn signal_registry_exposes_built_in_inventory() {
        let registry = SignalRegistry::with_built_ins();

        assert_eq!(registry.registrations().len(), 7);
        assert_eq!(
            registry.descriptor("delta_momentum_v1").unwrap().name,
            "Delta Momentum"
        );
        assert!(built_in_signal_registrations()
            .iter()
            .any(|registration| registration.descriptor.id == "sweep_detection_v1"));
    }

    #[test]
    fn signal_registry_filters_by_available_inputs() {
        let registry = SignalRegistry::with_built_ins();

        let none = registry.descriptors_matching_inputs(SignalInputMask::ANALYTICS);
        assert!(none.is_empty());

        let all = registry.descriptors_matching_inputs(
            SignalInputMask::ANALYTICS | SignalInputMask::DATA_QUALITY,
        );
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn signal_registry_validates_config_parameters() {
        let registry = SignalRegistry::with_built_ins();
        let params = [SignalConfigParameter::integer("threshold", 25)];
        let config = SignalConfig::with_parameters("delta_momentum_v1", &params);

        assert!(registry.validate_config(&config).is_ok());

        let bad_params = [SignalConfigParameter::integer("unknown", 1)];
        let bad_config = SignalConfig::with_parameters("delta_momentum_v1", &bad_params);
        assert!(matches!(
            registry.validate_config(&bad_config),
            Err(SignalRegistryError::UnknownParameter { .. })
        ));

        let wrong_type = [SignalConfigParameter::boolean("threshold", true)];
        let wrong_config = SignalConfig::with_parameters("delta_momentum_v1", &wrong_type);
        assert!(matches!(
            registry.validate_config(&wrong_config),
            Err(SignalRegistryError::InvalidParameterType { .. })
        ));

        let below_min = [SignalConfigParameter::integer("threshold", -1)];
        let below_config = SignalConfig::with_parameters("delta_momentum_v1", &below_min);
        assert!(matches!(
            registry.validate_config(&below_config),
            Err(SignalRegistryError::ParameterBelowMinimum { .. })
        ));

        let duplicate_params = [
            SignalConfigParameter::integer("threshold", 1),
            SignalConfigParameter::integer("threshold", 2),
        ];
        let duplicate_config =
            SignalConfig::with_parameters("delta_momentum_v1", &duplicate_params);
        assert!(matches!(
            registry.validate_config(&duplicate_config),
            Err(SignalRegistryError::DuplicateParameter { .. })
        ));
    }

    #[test]
    fn signal_registry_constructs_built_in_signal_from_config() {
        let registry = SignalRegistry::with_built_ins();
        let params = [SignalConfigParameter::integer("threshold", 25)];
        let config = SignalConfig::with_parameters("delta_momentum_v1", &params);
        let mut signal = registry.create_signal(&config).expect("signal constructed");

        signal.on_analytics(&AnalyticsSnapshot {
            delta: 30,
            ..Default::default()
        });

        let snapshot = signal.snapshot();
        assert_eq!(snapshot.module_id, "delta_momentum_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
    }

    #[test]
    fn signal_registry_rejects_duplicate_registration() {
        let mut registry = SignalRegistry::with_built_ins();
        let result = registry.register(SignalRegistration::new(
            &DELTA_MOMENTUM_DESCRIPTOR,
            Some(create_delta_momentum_signal),
        ));

        assert!(matches!(
            result,
            Err(SignalRegistryError::DuplicateSignal {
                id: "delta_momentum_v1"
            })
        ));
    }

    #[test]
    fn signal_descriptor_json_exports_binding_inventory() {
        let json = built_in_signal_descriptors_json();

        assert!(json.starts_with('['));
        assert!(json.contains("\"id\":\"delta_momentum_v1\""));
        assert!(json.contains("\"required_inputs_bits\":3"));
        assert!(json.contains("\"parameters\""));
        assert!(json.contains("\"output_semantics\":\"directional_bias\""));
    }

    #[test]
    fn signal_registry_validation_json_preserves_diagnostics() {
        let registry = SignalRegistry::with_built_ins();
        let invalid = [SignalConfigParameter::integer("threshold", -1)];
        let invalid_config = SignalConfig::with_parameters("delta_momentum_v1", &invalid);
        let invalid_json = registry.validate_config_json(&invalid_config);
        assert!(invalid_json.contains("\"valid\":false"));
        assert!(invalid_json.contains("below the descriptor minimum"));

        let valid_config = SignalConfig::new("delta_momentum_v1");
        let valid_json = registry.validate_config_json(&valid_config);
        assert!(valid_json.contains("\"valid\":true"));
        assert!(valid_json.contains("\"error\":null"));
    }

    #[test]
    fn signal_validation_scores_directional_markouts() {
        let mut signal = DeltaMomentumSignal::new(10);
        let events = vec![
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: -20,
                last_price: 90,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: -20,
                last_price: 80,
                ..Default::default()
            },
        ];
        let config = SignalValidationConfig::new(1).with_store_samples(true);

        let report = validate_signal_replay(&mut signal, &events, config);

        assert_eq!(report.module_id, Some("delta_momentum_v1"));
        assert_eq!(report.evaluated_events, 3);
        assert_eq!(report.labeled_events, 2);
        assert_eq!(report.missing_markouts, 1);
        assert_eq!(report.directional_predictions, 2);
        assert_eq!(report.correct_directional, 1);
        assert_eq!(report.incorrect_directional, 1);
        assert_eq!(report.directional_accuracy_bps(), Some(5_000));
        assert_eq!(report.label_coverage_bps(), Some(6_666));
        assert_eq!(report.samples.len(), 2);
        assert_eq!(
            report.samples[0].markout_direction,
            SignalMarkoutDirection::Down
        );
        assert_eq!(
            report.samples[0].predicted_direction,
            Some(SignalMarkoutDirection::Up)
        );
        assert_eq!(report.samples[0].correct, Some(false));
        assert!(report
            .warnings
            .iter()
            .any(|warning| matches!(warning, SignalValidationWarning::MissingMarkout { .. })));

        let json = report.json_report();
        assert!(json.contains("\"schema_version\":1"));
        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("\"long_predictions\":1"));
        assert!(json.contains("\"short_predictions\":2"));
        assert!(json.contains("\"directional_accuracy_bps\":5000"));
        assert!(json.contains("\"samples\":[{"));
        assert!(json.contains("\"code\":\"missing_markout\""));
    }

    #[test]
    fn signal_validation_confidence_filter_excludes_weak_predictions() {
        let mut signal = DeltaMomentumSignal::new(10);
        let events = vec![
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: 20,
                last_price: 110,
                ..Default::default()
            },
        ];
        let config = SignalValidationConfig::new(1).with_min_confidence_bps(600);

        let report = validate_signal_replay(&mut signal, &events, config);

        assert_eq!(report.directional_predictions, 0);
        assert_eq!(report.directional_accuracy_bps(), None);
        assert_eq!(report.long_predictions, 2);
    }

    #[test]
    fn signal_validation_warns_on_non_monotonic_timestamps() {
        let mut signal = DeltaMomentumSignal::new(10);
        let snapshots = [
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: 20,
                last_price: 110,
                ..Default::default()
            },
        ];
        let events = [
            SignalReplayEvent::with_ts_exchange_ns(&snapshots[0], 200),
            SignalReplayEvent::with_ts_exchange_ns(&snapshots[1], 100),
        ];

        let report =
            validate_signal_replay_events(&mut signal, &events, SignalValidationConfig::new(1));

        assert!(report.warnings.iter().any(|warning| matches!(
            warning,
            SignalValidationWarning::NonMonotonicTimestamp {
                event_index: 1,
                previous_ts_exchange_ns: 200,
                current_ts_exchange_ns: 100
            }
        )));
    }

    #[test]
    fn signal_validation_zero_horizon_is_normalized_and_reported() {
        let mut signal = DeltaMomentumSignal::new(10);
        let events = vec![
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: 20,
                last_price: 110,
                ..Default::default()
            },
        ];

        let report = validate_signal_replay(&mut signal, &events, SignalValidationConfig::new(0));

        assert_eq!(report.labeled_events, 1);
        assert!(report
            .warnings
            .contains(&SignalValidationWarning::ZeroMarkoutHorizon));
    }

    #[test]
    fn signal_validation_json_summary_is_python_friendly() {
        let mut signal = DeltaMomentumSignal::new(10);
        let events = vec![
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: 20,
                last_price: 110,
                ..Default::default()
            },
        ];

        let report = SignalValidationHarness::default().validate_signal(&mut signal, &events);
        let json = report.json_summary();

        assert!(json.contains("\"module_id\":\"delta_momentum_v1\""));
        assert!(json.contains("\"evaluated_events\":2"));
        assert!(json.contains("\"directional_accuracy_bps\":10000"));
    }

    #[test]
    fn signal_calibration_curve_interpolates_confidence() {
        let curve = SignalCalibrationCurve::new(vec![
            SignalCalibrationPoint::new(10_000, 9_000),
            SignalCalibrationPoint::new(0, 0),
            SignalCalibrationPoint::new(5_000, 4_000),
        ]);

        assert_eq!(
            IdentitySignalCalibrator.calibrate_confidence_bps(12_000),
            10_000
        );
        assert_eq!(curve.calibrate_confidence_bps(0), 0);
        assert_eq!(curve.calibrate_confidence_bps(2_500), 2_000);
        assert_eq!(curve.calibrate_confidence_bps(7_500), 6_500);
        assert_eq!(curve.calibrate_confidence_bps(10_000), 9_000);
    }

    #[test]
    fn signal_calibration_report_scores_validation_samples() {
        let mut signal = DeltaMomentumSignal::new(10);
        let events = vec![
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: -20,
                last_price: 90,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: -20,
                last_price: 80,
                ..Default::default()
            },
        ];
        let validation_config = SignalValidationConfig::new(1).with_store_samples(true);
        let validation_report = validate_signal_replay(&mut signal, &events, validation_config);

        let calibration_report = SignalCalibrationReport::from_validation_report(
            &validation_report,
            SignalCalibrationConfig::new(1_000),
        );

        assert_eq!(calibration_report.total_records, 2);
        assert_eq!(calibration_report.scored_records, 2);
        assert_eq!(calibration_report.ignored_records, 0);
        assert_eq!(calibration_report.correct_records, 1);
        assert_eq!(calibration_report.accuracy_bps(), Some(5_000));
        assert_eq!(calibration_report.expected_calibration_error_bps, 4_500);

        let populated_bin = calibration_report
            .bins
            .iter()
            .find(|bin| bin.samples == 2)
            .expect("populated confidence bin");
        assert_eq!(populated_bin.lower_confidence_bps, 0);
        assert_eq!(populated_bin.upper_confidence_bps, 999);
        assert_eq!(populated_bin.average_confidence_bps, 500);
        assert_eq!(populated_bin.accuracy_bps, Some(5_000));
        assert_eq!(populated_bin.calibration_error_bps, Some(4_500));
    }

    #[test]
    fn signal_outcome_tracker_summarizes_regimes() {
        let mut tracker = SignalOutcomeTracker::new(SignalCalibrationConfig::new(2_000));
        tracker.record(
            outcome_record(8_000, SignalMarkoutDirection::Up, Some(true)).with_regime("trend"),
        );
        tracker.record(
            outcome_record(8_000, SignalMarkoutDirection::Down, Some(false)).with_regime("trend"),
        );
        tracker.record(
            outcome_record(6_000, SignalMarkoutDirection::Up, Some(true)).with_regime("range"),
        );
        tracker.record(outcome_record(4_000, SignalMarkoutDirection::Flat, None));

        let report = tracker.calibration_report();

        assert_eq!(tracker.records().len(), 4);
        assert_eq!(report.total_records, 4);
        assert_eq!(report.scored_records, 3);
        assert_eq!(report.ignored_records, 1);

        let trend = report
            .regimes
            .iter()
            .find(|regime| regime.regime == "trend")
            .expect("trend regime");
        assert_eq!(trend.samples, 2);
        assert_eq!(trend.correct, 1);
        assert_eq!(trend.average_confidence_bps, 8_000);
        assert_eq!(trend.accuracy_bps, Some(5_000));

        let range = report
            .regimes
            .iter()
            .find(|regime| regime.regime == "range")
            .expect("range regime");
        assert_eq!(range.samples, 1);
        assert_eq!(range.accuracy_bps, Some(10_000));
    }

    #[test]
    fn signal_calibration_drift_flags_ece_change() {
        let config = SignalCalibrationConfig::new(1_000).with_drift_alert_threshold_bps(500);
        let baseline = SignalCalibrationReport::from_records(
            &[
                outcome_record(9_000, SignalMarkoutDirection::Up, Some(true)),
                outcome_record(9_000, SignalMarkoutDirection::Up, Some(true)),
            ],
            config,
        );
        let current = SignalCalibrationReport::from_records(
            &[
                outcome_record(9_000, SignalMarkoutDirection::Down, Some(false)),
                outcome_record(9_000, SignalMarkoutDirection::Down, Some(false)),
            ],
            config,
        );

        let drift = SignalCalibrationDriftReport::compare(
            &baseline,
            &current,
            config.drift_alert_threshold_bps,
        );

        assert_eq!(baseline.expected_calibration_error_bps, 1_000);
        assert_eq!(current.expected_calibration_error_bps, 9_000);
        assert_eq!(drift.ece_delta_bps, 8_000);
        assert!(drift.significant);
        assert!(drift
            .bin_drifts
            .iter()
            .any(|bin| bin.accuracy_delta_bps == Some(-10_000)));
    }

    #[test]
    fn signal_calibration_json_summary_is_dependency_free() {
        let report = SignalCalibrationReport::from_records(
            &[outcome_record(
                7_000,
                SignalMarkoutDirection::Up,
                Some(true),
            )],
            SignalCalibrationConfig::default(),
        );

        let json = report.json_summary();

        assert!(json.contains("\"total_records\":1"));
        assert!(json.contains("\"scored_records\":1"));
        assert!(json.contains("\"accuracy_bps\":10000"));
        assert!(json.contains("\"expected_calibration_error_bps\":3000"));
    }

    #[test]
    fn signal_ensemble_majority_selects_directional_side() {
        let votes = [
            SignalEnsembleVote::new("delta_momentum_v1", SignalState::LongBias, 7_000),
            SignalEnsembleVote::new("volume_imbalance_v1", SignalState::ShortBias, 8_000),
            SignalEnsembleVote::new("cumulative_delta_v1", SignalState::LongBias, 6_000),
        ];

        let decision =
            evaluate_signal_ensemble("ensemble_v1", &votes, SignalEnsemblePolicy::majority());

        assert_eq!(decision.snapshot.module_id, "ensemble_v1");
        assert_eq!(decision.snapshot.state, SignalState::LongBias);
        assert_eq!(decision.snapshot.confidence_bps, 6_500);
        assert_eq!(decision.metrics.total_votes, 3);
        assert_eq!(decision.metrics.long_votes, 2);
        assert_eq!(decision.metrics.short_votes, 1);
        assert_eq!(decision.conflict, SignalEnsembleConflict::None);
    }

    #[test]
    fn signal_ensemble_quorum_requires_minimum_votes() {
        let votes = [
            SignalEnsembleVote::new("delta_momentum_v1", SignalState::LongBias, 7_000),
            SignalEnsembleVote::new("volume_imbalance_v1", SignalState::ShortBias, 9_000),
        ];
        let policy = SignalEnsemblePolicy::quorum(2)
            .with_conflict_policy(SignalEnsembleConflictPolicy::HighestConfidence);

        let decision = evaluate_signal_ensemble("ensemble_v1", &votes, policy);

        assert_eq!(decision.snapshot.state, SignalState::Neutral);
        assert_eq!(decision.conflict, SignalEnsembleConflict::None);
        assert_eq!(decision.metrics.eligible_votes, 2);
    }

    #[test]
    fn signal_ensemble_weighted_policy_can_override_vote_count() {
        let votes = [
            SignalEnsembleVote::new("fast_momentum", SignalState::LongBias, 6_000)
                .with_weight_bps(2_000),
            SignalEnsembleVote::new("slow_momentum", SignalState::LongBias, 6_000)
                .with_weight_bps(2_000),
            SignalEnsembleVote::new("risk_model", SignalState::ShortBias, 9_000)
                .with_weight_bps(10_000),
        ];
        let policy = SignalEnsemblePolicy::weighted(8_000);

        let decision = evaluate_signal_ensemble("ensemble_v1", &votes, policy);

        assert_eq!(decision.snapshot.state, SignalState::ShortBias);
        assert_eq!(decision.metrics.long_votes, 2);
        assert_eq!(decision.metrics.short_votes, 1);
        assert_eq!(decision.metrics.long_weighted_score_bps, 2_400);
        assert_eq!(decision.metrics.short_weighted_score_bps, 9_000);
    }

    #[test]
    fn signal_ensemble_veto_blocks_by_default() {
        let votes = [
            SignalEnsembleVote::new("delta_momentum_v1", SignalState::LongBias, 7_000),
            SignalEnsembleVote::new("risk_veto_v1", SignalState::Blocked, 0)
                .with_quality_flags(DataQualityFlags::STALE_FEED.bits()),
        ];

        let decision =
            evaluate_signal_ensemble("ensemble_v1", &votes, SignalEnsemblePolicy::default());

        assert_eq!(decision.snapshot.state, SignalState::Blocked);
        assert!(decision.veto_applied);
        assert_eq!(decision.conflict, SignalEnsembleConflict::Veto);
        assert_eq!(decision.metrics.veto_votes, 1);
        assert_eq!(
            decision.snapshot.quality_flags,
            DataQualityFlags::STALE_FEED.bits()
        );
    }

    #[test]
    fn signal_ensemble_explanation_aggregates_children() {
        let children = vec![
            SignalExplanation::new(
                "delta_momentum_v1",
                SignalState::LongBias,
                8_000,
                0,
                SignalReasonCode::DeltaMomentumPositive,
                "delta_above_threshold",
            ),
            SignalExplanation::new(
                "volume_imbalance_v1",
                SignalState::LongBias,
                7_000,
                0,
                SignalReasonCode::BuyVolumeImbalance,
                "buy_volume_imbalance",
            ),
        ];

        let ensemble = evaluate_signal_ensemble_explanations(
            "ensemble_v1",
            children,
            &[10_000, 5_000],
            SignalEnsemblePolicy::majority(),
        );
        let explanation = ensemble.explanation();

        assert_eq!(ensemble.children.len(), 2);
        assert_eq!(ensemble.decision.snapshot.state, SignalState::LongBias);
        assert_eq!(
            explanation.reason_code,
            SignalReasonCode::EnsembleLongSelected
        );
        assert!(explanation
            .inputs
            .contains(&SignalInputValue::integer("total_votes", 2)));
        assert!(explanation
            .confidence_components
            .contains(&SignalConfidenceComponent::new(
                "average_child_confidence",
                7_500
            )));
    }

    #[test]
    fn signal_checkpoint_restore_validation_accepts_matching_metadata() {
        let symbol = SymbolId {
            venue: "SIM".to_string(),
            symbol: "ES".to_string(),
        };
        let checkpoint =
            SignalCheckpoint::from_snapshot(&snapshot(SignalState::LongBias, 7_000), "1")
                .with_config_hash(42)
                .with_symbol(symbol.clone())
                .with_calibration_id(7)
                .with_timestamps(1_000, 2_000)
                .with_payload(vec![1, 2, 3]);
        let policy = SignalCheckpointRestorePolicy::new()
            .with_signal("test_signal_v1", "1")
            .with_config_hash(42)
            .with_symbol(symbol)
            .with_calibration_id(7)
            .with_min_last_update_ns(1_500);

        let report = validate_signal_checkpoint_restore(&checkpoint, &policy);

        assert!(report.valid);
        assert!(report.issues.is_empty());
        assert_eq!(checkpoint.payload, vec![1, 2, 3]);
    }

    #[test]
    fn signal_checkpoint_restore_validation_reports_mismatches() {
        let checkpoint = SignalCheckpoint::new("candidate_signal_v1", "2", SignalState::Neutral)
            .with_config_hash(10)
            .with_timestamps(0, 500);
        let policy = SignalCheckpointRestorePolicy::new()
            .with_signal("production_signal_v1", "1")
            .with_config_hash(11)
            .with_min_last_update_ns(1_000);

        let report = validate_signal_checkpoint_restore(&checkpoint, &policy);

        assert!(!report.valid);
        assert!(report.has_errors());
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            SignalCheckpointValidationIssue::ModuleIdMismatch { .. }
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            SignalCheckpointValidationIssue::SignalVersionMismatch { .. }
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            SignalCheckpointValidationIssue::ConfigHashMismatch { .. }
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            SignalCheckpointValidationIssue::NonMonotonicTimestamp { .. }
        )));
    }

    #[test]
    fn signal_run_modes_encode_shadow_safety_behavior() {
        let active = SignalRunModeDecision::from_mode(SignalRunMode::Active);
        let shadow = SignalRunModeDecision::from_mode(SignalRunMode::Shadow);
        let record_only = SignalRunModeDecision::from_mode(SignalRunMode::RecordOnly);
        let disabled = SignalRunModeDecision::from_mode(SignalRunMode::Disabled);

        assert!(active.evaluate);
        assert!(active.publish_for_trading);
        assert!(shadow.evaluate);
        assert!(!shadow.publish_for_trading);
        assert!(shadow.record_output);
        assert!(!record_only.evaluate);
        assert!(record_only.record_input);
        assert!(!record_only.record_output);
        assert!(!disabled.evaluate);
        assert!(!disabled.record_input);
    }

    #[test]
    fn shadow_comparison_report_scores_candidate_against_production() {
        let production = snapshot(SignalState::LongBias, 6_000);
        let candidate = snapshot(SignalState::ShortBias, 8_000);
        let samples = [
            SignalShadowSample::compare(0, production, candidate)
                .with_ts_exchange_ns(1_000)
                .with_markout(SignalMarkoutDirection::Down),
            SignalShadowSample::compare(
                1,
                snapshot(SignalState::Neutral, 3_000),
                snapshot(SignalState::LongBias, 7_000),
            )
            .with_markout(SignalMarkoutDirection::Up),
        ];

        let report = SignalShadowComparisonReport::from_samples(
            &samples,
            SignalShadowComparisonConfig::new().with_store_samples(true),
        );

        assert_eq!(report.total_samples, 2);
        assert_eq!(report.state_disagreements, 2);
        assert_eq!(report.production_directional, 1);
        assert_eq!(report.candidate_directional, 2);
        assert_eq!(report.production_correct, 0);
        assert_eq!(report.candidate_correct, 2);
        assert_eq!(report.candidate_only_correct, 1);
        assert_eq!(report.average_confidence_delta_bps, 3_000);
        assert_eq!(report.agreement_bps(), Some(0));
        assert_eq!(report.candidate_accuracy_bps(), Some(10_000));
        assert_eq!(report.samples.len(), 2);
        assert!(report
            .json_summary()
            .contains("\"candidate_accuracy_bps\":10000"));
    }

    #[test]
    fn shadow_recorder_builds_report_without_retaining_samples_by_default() {
        let mut recorder = SignalShadowRecorder::default();
        recorder.record(
            SignalShadowSample::compare(
                0,
                snapshot(SignalState::LongBias, 7_000),
                snapshot(SignalState::LongBias, 8_000),
            )
            .with_markout(SignalMarkoutDirection::Up),
        );

        let report = recorder.report();

        assert_eq!(recorder.samples().len(), 1);
        assert_eq!(report.compared_samples, 1);
        assert_eq!(report.state_agreements, 1);
        assert_eq!(report.samples.len(), 0);
        assert_eq!(report.production_accuracy_bps(), Some(10_000));
    }

    #[test]
    fn feature_vector_validation_accepts_clean_vector() {
        let schema = FeatureSchema::new("orderflow_features", "1")
            .with_feature(
                FeatureDescriptor::new("delta", FeatureValueKind::Integer)
                    .with_unit("contracts")
                    .with_range(-10_000.0, 10_000.0),
            )
            .with_feature(
                FeatureDescriptor::new("imbalance_bps", FeatureValueKind::BasisPoints)
                    .with_unit("bps")
                    .with_range(-10_000.0, 10_000.0)
                    .with_freshness_ns(1_000),
            );
        let values = [125.0, 2_500.0];
        let quality = [FeatureQualityFlags::NONE, FeatureQualityFlags::NONE];
        let view = FeatureVectorView::new(&schema, &values, &quality, 1_000);

        let report = view.validate(Some(1_500));

        assert!(report.valid);
        assert_eq!(view.value("delta"), Some(125.0));
        assert_eq!(
            view.quality("imbalance_bps"),
            Some(FeatureQualityFlags::NONE)
        );
        assert_eq!(report.aggregate_quality, FeatureQualityFlags::NONE);
    }

    #[test]
    fn feature_vector_validation_reports_schema_and_quality_issues() {
        let schema = FeatureSchema::new("orderflow_features", "1")
            .with_feature(FeatureDescriptor::new("delta", FeatureValueKind::Integer))
            .with_feature(
                FeatureDescriptor::new("vwap_distance", FeatureValueKind::Price)
                    .with_range(-10.0, 10.0)
                    .with_freshness_ns(100),
            );
        let values = [25.0, 25.0];
        let quality = [FeatureQualityFlags::MISSING];
        let view = FeatureVectorView::new(&schema, &values, &quality, 1_000);

        let report = validate_feature_vector(&view, Some(1_200));

        assert!(!report.valid);
        assert!(report.has_errors());
        assert!(report
            .aggregate_quality
            .intersects(FeatureQualityFlags::MISSING | FeatureQualityFlags::OUT_OF_RANGE));
        assert!(report
            .issues
            .iter()
            .any(|issue| matches!(issue, FeatureVectorValidationIssue::LengthMismatch { .. })));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            FeatureVectorValidationIssue::MissingFeature { feature_id, .. }
                if feature_id == "delta"
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            FeatureVectorValidationIssue::OutOfRange { feature_id, .. }
                if feature_id == "vwap_distance"
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            FeatureVectorValidationIssue::StaleFeature { feature_id, .. }
                if feature_id == "vwap_distance"
        )));
    }

    #[test]
    fn model_metadata_and_input_binding_validate_schema_compatibility() {
        let schema = FeatureSchema::new("orderflow_features", "1")
            .with_config_hash(42)
            .with_feature(FeatureDescriptor::new("delta", FeatureValueKind::Integer))
            .with_feature(FeatureDescriptor::new("vwap", FeatureValueKind::Price));
        let metadata = SignalModelMetadata::new("model_a", "2026-07-15", "orderflow_features", "1")
            .with_model_kind(SignalModelKind::Onnx)
            .with_artifact_hash("sha256:abc")
            .with_training_window(1_000, 2_000)
            .with_calibration_id(7)
            .with_output_kind(SignalModelOutputKind::DirectionalProbabilities);
        let binding = SignalModelInputBinding::new("features", vec!["delta".into(), "vwap".into()]);
        let missing =
            SignalModelInputBinding::new("features", vec!["delta".into(), "missing".into()]);

        assert_eq!(schema.feature_index("vwap"), Some(1));
        assert!(binding.is_compatible_with(&schema));
        assert!(!missing.is_compatible_with(&schema));
        assert_eq!(metadata.model_kind, SignalModelKind::Onnx);
        assert_eq!(metadata.calibration_id, Some(7));
    }

    #[test]
    fn model_backed_signal_trait_runs_over_feature_view() {
        struct TestModelSignal {
            metadata: SignalModelMetadata,
            schema: FeatureSchema,
            snapshot: SignalSnapshot,
        }

        impl SignalModule for TestModelSignal {
            fn on_analytics(&mut self, _ev: &AnalyticsSnapshot) {}

            fn snapshot(&self) -> SignalSnapshot {
                self.snapshot.clone()
            }

            fn quality_gate(&self, _q: DataQualityFlags) -> SignalGateDecision {
                SignalGateDecision::Pass
            }
        }

        impl ModelBackedSignal for TestModelSignal {
            fn model_metadata(&self) -> &SignalModelMetadata {
                &self.metadata
            }

            fn feature_schema(&self) -> &FeatureSchema {
                &self.schema
            }

            fn infer_features(&mut self, features: &FeatureVectorView<'_>) -> SignalModelOutput {
                let delta = features.value("delta").unwrap_or_default();
                let state = if delta >= 0.0 {
                    SignalState::LongBias
                } else {
                    SignalState::ShortBias
                };
                self.snapshot.state = state;
                self.snapshot.confidence_bps = 7_500;
                SignalModelOutput::new(state, 7_500)
                    .with_score(delta)
                    .with_reason("test_model")
            }
        }

        let schema = FeatureSchema::new("orderflow_features", "1")
            .with_feature(FeatureDescriptor::new("delta", FeatureValueKind::Integer));
        let metadata = SignalModelMetadata::new("test_model", "1", "orderflow_features", "1")
            .with_model_kind(SignalModelKind::Native);
        let values = [5.0];
        let quality = [FeatureQualityFlags::NONE];
        let view = FeatureVectorView::new(&schema, &values, &quality, 10);
        let mut signal = TestModelSignal {
            metadata,
            schema: schema.clone(),
            snapshot: snapshot(SignalState::Neutral, 0),
        };

        let output = signal.infer_features(&view);

        assert_eq!(signal.model_metadata().model_id, "test_model");
        assert_eq!(signal.feature_schema().id, "orderflow_features");
        assert_eq!(output.state, SignalState::LongBias);
        assert_eq!(output.confidence_bps, 7_500);
        assert_eq!(output.score, Some(5.0));
        assert_eq!(signal.snapshot().state, SignalState::LongBias);
    }

    #[test]
    fn built_in_descriptors_cover_all_default_modules() {
        let descriptors = built_in_signal_descriptors();
        assert_eq!(descriptors.len(), 7);

        let ids: Vec<&str> = descriptors.iter().map(|descriptor| descriptor.id).collect();
        assert!(ids.contains(&"delta_momentum_v1"));
        assert!(ids.contains(&"volume_imbalance_v1"));
        assert!(ids.contains(&"cumulative_delta_v1"));
        assert!(ids.contains(&"absorption_v1"));
        assert!(ids.contains(&"exhaustion_v1"));
        assert!(ids.contains(&"sweep_detection_v1"));
        assert!(ids.contains(&"composite_v1"));

        for descriptor in descriptors {
            assert!(descriptor.requires_input(SignalInputMask::ANALYTICS));
            assert!(descriptor.requires_input(SignalInputMask::DATA_QUALITY));
            assert!(descriptor.deterministic);
        }
    }

    #[test]
    fn describe_signal_finds_parameter_metadata() {
        let descriptor = describe_signal("absorption_v1").expect("descriptor exists");
        assert_eq!(descriptor.name, "Absorption");

        let threshold = descriptor
            .parameter("threshold")
            .expect("threshold parameter exists");
        assert_eq!(threshold.kind, SignalParameterKind::Integer);
        assert_eq!(threshold.default, Some(SignalParameterValue::Integer(150)));

        let price_band = descriptor
            .parameter("price_band")
            .expect("price_band parameter exists");
        assert_eq!(price_band.default, Some(SignalParameterValue::Integer(2)));
        assert!(descriptor.parameter("unknown").is_none());
        assert!(describe_signal("unknown").is_none());
    }

    #[test]
    fn signal_lifecycle_activates_after_event_warmup() {
        let mut lifecycle = SignalLifecycle::new(SignalWarmupRequirement::Events(2));
        assert_eq!(lifecycle.state(), SignalLifecycleState::WarmingUp);
        assert!(!lifecycle.is_active());

        lifecycle.record_event();
        assert_eq!(lifecycle.state(), SignalLifecycleState::WarmingUp);

        lifecycle.record_event();
        assert_eq!(lifecycle.state(), SignalLifecycleState::Active);
        assert!(lifecycle.is_active());
        assert_eq!(lifecycle.progress().events, 2);
    }

    #[test]
    fn signal_lifecycle_supports_composite_warmup_requirements() {
        const REQUIREMENTS: &[SignalWarmupRequirement] = &[
            SignalWarmupRequirement::Events(2),
            SignalWarmupRequirement::CompletedBars(1),
            SignalWarmupRequirement::MarketTimeNs(1_000),
        ];

        let mut lifecycle = SignalLifecycle::new(SignalWarmupRequirement::All(REQUIREMENTS));
        lifecycle.record_event();
        lifecycle.record_completed_bar();
        lifecycle.set_market_time_ns(1_000);
        assert_eq!(lifecycle.state(), SignalLifecycleState::WarmingUp);

        lifecycle.record_event();
        assert_eq!(lifecycle.state(), SignalLifecycleState::Active);
    }

    #[test]
    fn disabled_lifecycle_does_not_degrade_or_block() {
        let mut lifecycle = SignalLifecycle::new(SignalWarmupRequirement::None);
        lifecycle.disable();
        lifecycle.block();
        assert_eq!(lifecycle.state(), SignalLifecycleState::Disabled);
        lifecycle.degrade();
        assert_eq!(lifecycle.state(), SignalLifecycleState::Disabled);
    }

    #[test]
    fn custom_descriptor_constructors_support_external_signals() {
        const PARAMS: &[SignalParameterDescriptor] = &[SignalParameterDescriptor::integer(
            "lookback_events",
            "Number of events used by the custom signal.",
            Some(32),
            Some(1),
            Some(10_000),
        )];

        let descriptor = SignalDescriptor::new(
            "custom_signal_v1",
            "Custom Signal",
            "1",
            "Example custom signal descriptor.",
        )
        .with_required_inputs(SignalInputMask::ANALYTICS | SignalInputMask::DATA_QUALITY)
        .with_warmup(SignalWarmupRequirement::Events(32))
        .with_parameters(PARAMS)
        .with_output_semantics(SignalOutputSemantics::DirectionalBias)
        .with_deterministic(true)
        .with_checkpointable(true);

        assert_eq!(descriptor.id, "custom_signal_v1");
        assert!(descriptor.requires_input(SignalInputMask::ANALYTICS));
        assert!(descriptor.checkpointable);
        assert_eq!(
            descriptor.parameter("lookback_events").unwrap().default,
            Some(SignalParameterValue::Integer(32))
        );
    }

    #[test]
    fn signal_context_builder_attaches_optional_inputs() {
        let analytics = AnalyticsSnapshot {
            delta: 10,
            ..Default::default()
        };
        let symbol = SymbolId {
            venue: "SIM".to_string(),
            symbol: "ES".to_string(),
        };
        let book = BookSnapshot {
            symbol: symbol.clone(),
            bids: Vec::new(),
            asks: Vec::new(),
            last_sequence: 42,
            ts_exchange_ns: 100,
            ts_recv_ns: 110,
        };
        let tags = [("profile", "research")];

        let ctx = SignalContext::new(&analytics, DataQualityFlags::NONE)
            .with_symbol(&symbol)
            .with_book(&book)
            .with_timestamps(Some(100), Some(110))
            .with_lifecycle_state(SignalLifecycleState::Active)
            .with_extension_tags(&tags);

        assert_eq!(ctx.analytics.delta, 10);
        assert_eq!(ctx.symbol.unwrap().symbol, "ES");
        assert_eq!(ctx.book.unwrap().last_sequence, 42);
        assert_eq!(ctx.ts_exchange_ns, Some(100));
        assert_eq!(ctx.ts_recv_ns, Some(110));
        assert_eq!(ctx.lifecycle_state, Some(SignalLifecycleState::Active));
        assert_eq!(ctx.extension_tags, &[("profile", "research")]);
    }

    #[test]
    fn legacy_adapter_forwards_context_to_signal_module() {
        let mut signal = LegacySignalAdapter::with_descriptor(
            DeltaMomentumSignal::new(10),
            &DELTA_MOMENTUM_DESCRIPTOR,
        );
        let analytics = AnalyticsSnapshot {
            delta: 15,
            ..Default::default()
        };
        let ctx = SignalContext::new(&analytics, DataQualityFlags::NONE);

        assert_eq!(
            signal.lifecycle_state(),
            Some(SignalLifecycleState::WarmingUp)
        );
        signal.on_context(&ctx);

        let snapshot = signal.snapshot();
        assert_eq!(snapshot.module_id, "delta_momentum_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
        assert_eq!(signal.descriptor().unwrap().id, "delta_momentum_v1");
        assert_eq!(signal.lifecycle_state(), Some(SignalLifecycleState::Active));
    }

    #[test]
    fn legacy_adapter_uses_wrapped_quality_gate() {
        let signal = LegacySignalAdapter::new(DeltaMomentumSignal::default());
        let analytics = AnalyticsSnapshot::default();
        let ctx = SignalContext::new(&analytics, DataQualityFlags::SEQUENCE_GAP);

        assert_eq!(signal.quality_gate(&ctx), SignalGateDecision::Block);
    }

    #[test]
    fn legacy_adapter_can_return_inner_signal() {
        let mut adapter = LegacySignalAdapter::new(DeltaMomentumSignal::new(5));
        adapter.reset_lifecycle();
        assert_eq!(
            adapter.lifecycle_state(),
            Some(SignalLifecycleState::WarmingUp)
        );

        let inner = adapter.into_inner();
        let mut signal = inner;
        signal.on_analytics(&AnalyticsSnapshot {
            delta: -10,
            ..Default::default()
        });

        assert_eq!(signal.snapshot().state, SignalState::ShortBias);
    }

    #[test]
    fn stabilizer_accepts_immediately_when_policies_are_disabled() {
        let mut stabilizer = SignalStabilizer::new();
        let decision = stabilizer.stabilize(snapshot(SignalState::LongBias, 1), 1);

        assert!(decision.accepted);
        assert_eq!(decision.suppression_reason, SignalSuppressionReason::None);
        assert_eq!(decision.transition, SignalTransitionKind::Entry);
        assert_eq!(decision.emitted.state, SignalState::LongBias);
    }

    #[test]
    fn stabilizer_hysteresis_blocks_weak_entry() {
        let mut stabilizer = SignalStabilizer::with_policies(
            HysteresisPolicy::new(700, 0, 0),
            DebouncePolicy::default(),
            CooldownPolicy::default(),
        );

        let weak = stabilizer.stabilize(snapshot(SignalState::LongBias, 600), 1);
        assert!(!weak.accepted);
        assert_eq!(weak.suppression_reason, SignalSuppressionReason::Hysteresis);
        assert_eq!(weak.emitted.state, SignalState::Neutral);

        let strong = stabilizer.stabilize(snapshot(SignalState::LongBias, 700), 2);
        assert!(strong.accepted);
        assert_eq!(strong.emitted.state, SignalState::LongBias);
    }

    #[test]
    fn stabilizer_debounce_requires_repeated_confirmation() {
        let mut stabilizer = SignalStabilizer::with_policies(
            HysteresisPolicy::default(),
            DebouncePolicy::new(2, 0),
            CooldownPolicy::default(),
        );

        let first = stabilizer.stabilize(snapshot(SignalState::LongBias, 900), 1);
        assert!(!first.accepted);
        assert_eq!(
            first.suppression_reason,
            SignalSuppressionReason::DebouncePending
        );

        let second = stabilizer.stabilize(snapshot(SignalState::LongBias, 900), 2);
        assert!(second.accepted);
        assert_eq!(second.emitted.state, SignalState::LongBias);
    }

    #[test]
    fn stabilizer_debounce_requires_time_confirmation() {
        let mut stabilizer = SignalStabilizer::with_policies(
            HysteresisPolicy::default(),
            DebouncePolicy::new(1, 10),
            CooldownPolicy::default(),
        );

        let first = stabilizer.stabilize(snapshot(SignalState::ShortBias, 900), 100);
        assert!(!first.accepted);

        let early = stabilizer.stabilize(snapshot(SignalState::ShortBias, 900), 105);
        assert!(!early.accepted);

        let ready = stabilizer.stabilize(snapshot(SignalState::ShortBias, 900), 110);
        assert!(ready.accepted);
        assert_eq!(ready.emitted.state, SignalState::ShortBias);
    }

    #[test]
    fn stabilizer_cooldown_suppresses_reversal_after_entry() {
        let mut stabilizer = SignalStabilizer::with_policies(
            HysteresisPolicy::default(),
            DebouncePolicy::default(),
            CooldownPolicy::new(100, 0, 0),
        );

        let entry = stabilizer.stabilize(snapshot(SignalState::LongBias, 900), 1_000);
        assert!(entry.accepted);

        let reversal = stabilizer.stabilize(snapshot(SignalState::ShortBias, 900), 1_050);
        assert!(!reversal.accepted);
        assert_eq!(
            reversal.suppression_reason,
            SignalSuppressionReason::CooldownActive
        );
        assert_eq!(reversal.emitted.state, SignalState::LongBias);

        let ready = stabilizer.stabilize(snapshot(SignalState::ShortBias, 900), 1_100);
        assert!(ready.accepted);
        assert_eq!(ready.transition, SignalTransitionKind::Reversal);
    }

    #[test]
    fn stabilizer_accepts_blocked_state_without_suppression() {
        let mut stabilizer = SignalStabilizer::with_policies(
            HysteresisPolicy::new(900, 900, 900),
            DebouncePolicy::new(10, 1_000),
            CooldownPolicy::new(1_000, 1_000, 1_000),
        );

        let blocked = stabilizer.stabilize(snapshot(SignalState::Blocked, 0), 1);
        assert!(blocked.accepted);
        assert_eq!(blocked.emitted.state, SignalState::Blocked);
        assert_eq!(blocked.suppression_reason, SignalSuppressionReason::None);
    }
}
