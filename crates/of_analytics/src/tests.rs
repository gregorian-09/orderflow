#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_quality_computes_spreads_and_improvement() {
        let mut tracker = MarketQualityTracker::new(1_000);
        tracker.on_quote(QuoteContext::new(499_000, 501_000, 100, 120, 1).expect("quote"));

        let snapshot = tracker
            .evaluate_trade(
                TradeContext::new(500_000, 10, Side::Ask, 500).expect("trade"),
                Some(500_010),
            )
            .expect("snapshot");

        assert_eq!(snapshot.quoted_spread(), 2_000);
        assert_eq!(snapshot.quoted_spread_bps(), 40);
        assert_eq!(snapshot.effective_spread_bps(), 0);
        assert!(snapshot.price_improvement_bps() > 0);
        assert!(!snapshot.stale_quote());
    }

    #[test]
    fn market_quality_flags_stale_quote() {
        let mut tracker = MarketQualityTracker::new(10);
        tracker.on_quote(QuoteContext::new(499_975, 500_025, 100, 120, 1).expect("quote"));

        let snapshot = tracker
            .evaluate_trade(
                TradeContext::new(500_025, 10, Side::Ask, 100).expect("trade"),
                None,
            )
            .expect("snapshot");

        assert!(snapshot.stale_quote());
    }

    #[test]
    fn execution_quality_scores_buy_fill_against_benchmarks() {
        let trade = TradeContext::new(101_000, 10, Side::Ask, 1).expect("trade");
        let benchmark = ExecutionBenchmark::new(100_000, 100_500, 99_950, 100_050, Some(100_750))
            .expect("benchmark");

        let snapshot = ExecutionQualityAnalyzer::evaluate(trade, benchmark);

        assert_eq!(snapshot.arrival_slippage_bps(), 100);
        assert_eq!(snapshot.decision_slippage_bps(), 49);
        assert_eq!(snapshot.implementation_shortfall_bps(), 49);
        assert_eq!(snapshot.adverse_selection_bps(), 24);
        assert!(snapshot.trade_through());
        assert!(snapshot.fill_quality_score_bps() < 10_000);
    }

    #[test]
    fn execution_quality_scores_sell_price_improvement() {
        let trade = TradeContext::new(100_500, 10, Side::Bid, 1).expect("trade");
        let benchmark = ExecutionBenchmark::new(100_000, 100_000, 99_950, 100_050, Some(100_250))
            .expect("benchmark");

        let snapshot = ExecutionQualityAnalyzer::evaluate(trade, benchmark);

        assert!(snapshot.arrival_slippage_bps() < 0);
        assert!(snapshot.decision_slippage_bps() < 0);
        assert!(snapshot.adverse_selection_bps() < 0);
        assert!(!snapshot.trade_through());
        assert_eq!(snapshot.fill_quality_score_bps(), 10_000);
    }

    #[test]
    fn liquidity_depth_uses_borrowed_levels() {
        let bids = [
            BookLevel {
                level: 0,
                price: 499_975,
                size: 100,
            },
            BookLevel {
                level: 1,
                price: 499_950,
                size: 80,
            },
        ];
        let asks = [
            BookLevel {
                level: 0,
                price: 500_025,
                size: 120,
            },
            BookLevel {
                level: 1,
                price: 500_050,
                size: 90,
            },
        ];

        let snapshot = LiquidityDepthAnalyzer::new(2)
            .analyze(&bids, &asks, 150)
            .expect("snapshot");

        assert_eq!(snapshot.levels_used(), 2);
        assert_eq!(snapshot.bid_depth(), 180);
        assert_eq!(snapshot.ask_depth(), 210);
        assert_eq!(snapshot.sweepable_buy_qty(), 150);
        assert_eq!(snapshot.sweepable_sell_qty(), 150);
        assert_eq!(snapshot.buy_sweepability_bps(), 10_000);
        assert_eq!(snapshot.sell_sweepability_bps(), 10_000);
        assert_eq!(snapshot.sweepability_score_bps(), 10_000);
        assert!(snapshot.proportional_imbalance_bps() < 0);
    }

    #[test]
    fn liquidity_depth_reports_shape_pressure_and_partial_sweepability() {
        let bids = [
            BookLevel {
                level: 0,
                price: 500_000,
                size: 300,
            },
            BookLevel {
                level: 1,
                price: 499_975,
                size: 200,
            },
            BookLevel {
                level: 2,
                price: 499_950,
                size: 100,
            },
        ];
        let asks = [
            BookLevel {
                level: 0,
                price: 500_025,
                size: 50,
            },
            BookLevel {
                level: 1,
                price: 500_050,
                size: 100,
            },
            BookLevel {
                level: 2,
                price: 500_075,
                size: 150,
            },
        ];

        let snapshot = LiquidityDepthAnalyzer::new(3)
            .analyze(&bids, &asks, 500)
            .expect("snapshot");

        assert_eq!(snapshot.depth_slope_bps(), -2_857);
        assert_eq!(snapshot.depth_convexity_bps(), 0);
        assert!(snapshot.book_pressure_bps() > 0);
        assert_eq!(snapshot.buy_sweepability_bps(), 6_000);
        assert_eq!(snapshot.sell_sweepability_bps(), 10_000);
        assert_eq!(snapshot.sweepability_score_bps(), 6_000);
    }

    #[test]
    fn liquidity_depth_rejects_invalid_levels() {
        let bids = [BookLevel {
            level: 0,
            price: 0,
            size: 100,
        }];
        let asks = [BookLevel {
            level: 0,
            price: 500_025,
            size: 100,
        }];

        assert_eq!(
            LiquidityDepthAnalyzer::new(1).analyze(&bids, &asks, 10),
            Err(AnalyticsError::InvalidDepth)
        );
    }

    #[test]
    fn liquidity_flow_tracks_imbalance_rates_and_drought() {
        let config = LiquidityFlowConfig::new(1_000_000_000, 2_500, 100).unwrap();
        let mut tracker = LiquidityFlowTracker::new(config);

        tracker.on_event(LiquidityFlowEvent::new(Side::Bid, 100, 0, 0, 1).unwrap());
        tracker.on_event(LiquidityFlowEvent::new(Side::Ask, 0, 250, 50, 500_000_001).unwrap());

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.events(), 2);
        assert_eq!(snapshot.bid_added_qty(), 100);
        assert_eq!(snapshot.ask_depleted_qty(), 250);
        assert_eq!(snapshot.ask_traded_qty(), 50);
        assert_eq!(snapshot.order_flow_imbalance_bps(), 10_000);
        assert_eq!(snapshot.replenishment_rate_per_sec(), 200);
        assert_eq!(snapshot.depletion_rate_per_sec(), 600);
        assert!(!snapshot.liquidity_drought());

        tracker.on_event(LiquidityFlowEvent::new(Side::Bid, 0, 600, 0, 1_000_000_001).unwrap());
        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.depletion_rate_per_sec(), 900);
        assert!(snapshot.liquidity_drought());

        tracker.reset();
        assert_eq!(tracker.snapshot().events(), 0);
    }

    #[test]
    fn liquidity_flow_rejects_empty_or_invalid_events() {
        assert_eq!(
            LiquidityFlowEvent::new(Side::Bid, 0, 0, 0, 1),
            Err(AnalyticsError::InvalidDepth)
        );
        assert_eq!(
            LiquidityFlowEvent::new(Side::Ask, 1, 0, 0, 0),
            Err(AnalyticsError::InvalidDepth)
        );
        assert_eq!(
            LiquidityFlowConfig::new(0, 0, 0),
            Err(AnalyticsError::InvalidDepth)
        );
    }

    #[test]
    fn impact_tracker_computes_kyle_lambda() {
        let mut tracker = ImpactTracker::new();
        tracker.on_sample(ImpactSample::new(500_000, 501_000, 100, 50_000_000).expect("sample"));
        tracker.on_sample(ImpactSample::new(501_000, 500_500, -50, 25_000_000).expect("sample"));

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 2);
        assert_eq!(snapshot.signed_volume(), 50);
        assert_eq!(snapshot.absolute_volume(), 150);
        assert_eq!(snapshot.signed_price_change(), 1_500);
        assert!(snapshot.kyle_lambda_ppm() > 0);
    }

    #[test]
    fn expected_impact_estimator_combines_calibrated_components() {
        let calibration = ImpactCalibration::new(1_000_000, 200, 10_000, 500, 250, 1_000_000_000)
            .expect("calibration");
        let input = ExpectedImpactInput::new(
            Side::Ask,
            1_000,
            10_000,
            100_000,
            1_000_000_000,
            calibration,
        )
        .expect("input");

        let snapshot = ExpectedImpactEstimator::estimate(input);

        assert_eq!(snapshot.participation_bps(), 1_000);
        assert_eq!(snapshot.daily_participation_bps(), 10);
        assert_eq!(snapshot.square_root_impact_bps(), 6);
        assert_eq!(snapshot.temporary_impact_bps(), 50);
        assert_eq!(snapshot.permanent_impact_bps(), 25);
        assert_eq!(snapshot.instantaneous_impact_bps(), 75);
        assert_eq!(snapshot.decay_remaining_bps(), 5_000);
        assert_eq!(snapshot.expected_total_impact_bps(), 56);
        assert_eq!(snapshot.expected_signed_price_move(), 560);
    }

    #[test]
    fn child_order_impact_attribution_is_side_aware() {
        let buy =
            ChildOrderImpactContext::new(Side::Ask, 1_000, 100, 100_000, 100_500, 100_800, 100_200)
                .expect("buy context");
        let buy_snapshot = ChildOrderImpactAnalyzer::evaluate(buy);

        assert_eq!(buy_snapshot.child_participation_bps(), 1_000);
        assert_eq!(buy_snapshot.child_slippage_bps(), 50);
        assert_eq!(buy_snapshot.instantaneous_impact_bps(), 80);
        assert_eq!(buy_snapshot.permanent_impact_bps(), 20);
        assert_eq!(buy_snapshot.temporary_impact_bps(), 30);
        assert_eq!(buy_snapshot.impact_decay_bps(), 60);
        assert_eq!(buy_snapshot.attributed_impact_bps(), 5);

        let sell =
            ChildOrderImpactContext::new(Side::Bid, 1_000, 100, 100_000, 99_500, 99_200, 99_800)
                .expect("sell context");
        assert_eq!(
            ChildOrderImpactAnalyzer::evaluate(sell).child_slippage_bps(),
            50
        );
    }

    #[test]
    fn impact_primitives_reject_invalid_inputs() {
        assert_eq!(
            ImpactCalibration::new(0, 0, 0, 0, 0, 0),
            Err(AnalyticsError::InvalidTrade)
        );
        let calibration = ImpactCalibration::new(1_000, 100, 10_000, 100, 100, 1).unwrap();
        assert_eq!(
            ExpectedImpactInput::new(Side::Ask, 0, 1, 1, 1, calibration),
            Err(AnalyticsError::InvalidTrade)
        );
        assert_eq!(
            ChildOrderImpactContext::new(Side::Ask, 100, 101, 1, 1, 1, 1),
            Err(AnalyticsError::InvalidTrade)
        );
    }

    #[test]
    fn vpin_tracker_closes_fixed_buckets() {
        let mut tracker = VpinTracker::<2>::new(100).expect("tracker");
        tracker.on_trade(TradeContext::new(500_000, 80, Side::Ask, 1).expect("trade"));
        tracker.on_trade(TradeContext::new(500_000, 20, Side::Bid, 2).expect("trade"));
        tracker.on_trade(TradeContext::new(500_000, 100, Side::Bid, 3).expect("trade"));

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.bucket_count(), 2);
        assert_eq!(snapshot.current_bucket_volume(), 0);
        assert_eq!(snapshot.vpin_bps(), 8_000);
        assert_eq!(snapshot.toxicity_bps(), snapshot.vpin_bps());
    }

    #[test]
    fn toxicity_analyzer_detects_adverse_burst_and_quote_fade() {
        let analyzer = ToxicityAnalyzer::new(ToxicityConfig::new(20, 3_000, 7_000, 6_000).unwrap());
        let input = ToxicityInput::new(
            TradeContext::new(100_000, 10, Side::Ask, 1).unwrap(),
            100_500,
            1_000,
            1_000,
            1_000,
            500,
            8_000,
            7_000,
        )
        .unwrap();

        let snapshot = analyzer.evaluate(input);

        assert_eq!(snapshot.post_trade_markout_bps(), 50);
        assert_eq!(snapshot.quote_fade_bps(), 5_000);
        assert_eq!(snapshot.adverse_selection_score_bps(), 10_000);
        assert!(snapshot.informed_flow_proxy_bps() > 8_000);
        assert!(snapshot.toxic_flow_burst());
        assert!(snapshot.toxicity_score_bps() >= snapshot.informed_flow_proxy_bps());
    }

    #[test]
    fn toxicity_analyzer_handles_favorable_markout() {
        let input = ToxicityInput::new(
            TradeContext::new(100_000, 10, Side::Bid, 1).unwrap(),
            100_500,
            1_000,
            1_000,
            700,
            1_000,
            1_000,
            1_000,
        )
        .unwrap();

        let snapshot = ToxicityAnalyzer::default().evaluate(input);

        assert!(snapshot.post_trade_markout_bps() < 0);
        assert_eq!(snapshot.adverse_selection_score_bps(), 0);
        assert_eq!(snapshot.quote_fade_bps(), 3_000);
        assert!(!snapshot.toxic_flow_burst());
    }

    #[test]
    fn toxicity_primitives_reject_invalid_inputs() {
        assert_eq!(
            ToxicityConfig::new(0, 1, 1, 1),
            Err(AnalyticsError::InvalidTrade)
        );
        assert_eq!(
            ToxicityInput::new(
                TradeContext::new(1, 1, Side::Ask, 1).unwrap(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ),
            Err(AnalyticsError::InvalidTrade)
        );
    }

    #[test]
    fn volatility_tracker_computes_realized_noise() {
        let mut tracker = VolatilityTracker::<4>::new().expect("tracker");
        tracker.on_price(100_000).expect("price");
        tracker.on_price(101_000).expect("price");
        tracker.on_price(100_000).expect("price");
        tracker.on_price(101_000).expect("price");

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 3);
        assert_eq!(snapshot.last_price(), 101_000);
        assert!(snapshot.realized_vol_bps() > 0);
        assert!(snapshot.mean_abs_return_bps() > 0);
        assert!(snapshot.bipower_vol_bps() > 0);
        assert!(snapshot.jump_variation_bps() <= snapshot.realized_vol_bps());
        assert!(snapshot.noise_ratio_bps() > 0);
    }

    #[test]
    fn volatility_tracker_preserves_ring_order_for_noise() {
        let mut tracker = VolatilityTracker::<3>::new().expect("tracker");
        for price in [100_000, 101_000, 102_000, 101_000, 100_000] {
            tracker.on_price(price).expect("price");
        }

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 3);
        assert_eq!(snapshot.noise_ratio_bps(), 5_000);
    }

    #[test]
    fn ohlc_volatility_estimator_reports_range_estimators() {
        let input = OhlcVolatilityInput::new(100_000, 102_000, 99_000, 101_000, Some(99_500))
            .expect("ohlc");
        let snapshot = OhlcVolatilityEstimator::estimate(input);

        assert_eq!(snapshot.close_to_close_vol_bps(), 150);
        assert_eq!(snapshot.jump_gap_bps(), 50);
        assert!(snapshot.parkinson_vol_bps() > 0);
        assert!(snapshot.garman_klass_vol_bps() > 0);
        assert!(snapshot.rogers_satchell_vol_bps() > 0);
        assert_eq!(
            OhlcVolatilityInput::new(100_000, 99_000, 100_000, 100_000, None),
            Err(AnalyticsError::InvalidTrade)
        );
    }

    #[test]
    fn volatility_signature_estimator_reports_noise() {
        let snapshot = VolatilitySignatureEstimator::estimate(1_000_000, &[10, -10, 20, -20])
            .expect("signature");

        assert_eq!(snapshot.sampling_interval_ns(), 1_000_000);
        assert_eq!(snapshot.samples(), 4);
        assert!(snapshot.realized_vol_bps() > 0);
        assert_eq!(snapshot.noise_ratio_bps(), 10_000);
        assert_eq!(
            VolatilitySignatureEstimator::estimate(0, &[1]),
            Err(AnalyticsError::InvalidTrade)
        );
    }

    #[test]
    fn volatility_seasonality_tracker_accumulates_buckets() {
        let mut tracker = VolatilitySeasonalityTracker::<2>::new(25).expect("tracker");
        tracker.on_return(0, 10).expect("return");
        tracker.on_return(0, -30).expect("return");
        tracker.on_return(1, 5).expect("return");

        let bucket = tracker.snapshot(0).expect("bucket");

        assert_eq!(bucket.bucket(), 0);
        assert_eq!(bucket.samples(), 2);
        assert_eq!(bucket.mean_abs_return_bps(), 20);
        assert_eq!(bucket.jump_count(), 1);
        assert!(bucket.realized_vol_bps() > 0);
        assert_eq!(tracker.on_return(2, 1), Err(AnalyticsError::InvalidTrade));

        tracker.reset();
        assert_eq!(tracker.snapshot(0).unwrap().samples(), 0);
    }

    #[test]
    fn regime_classifier_prioritizes_toxicity_and_illiquidity() {
        let classifier = RegimeClassifier::default();

        assert_eq!(
            classifier
                .classify(RegimeInput::new(1, 10, 8_000, 0))
                .kind(),
            RegimeKind::Toxic
        );
        assert_eq!(
            classifier.classify(RegimeInput::new(50, 10, 0, 0)).kind(),
            RegimeKind::Illiquid
        );
        assert_eq!(
            classifier.classify(RegimeInput::new(1, 100, 0, 0)).kind(),
            RegimeKind::Volatile
        );
    }

    #[test]
    fn composite_regime_classifier_detects_news_hidden_chop() {
        let classifier = CompositeRegimeClassifier::default();
        let snapshot = classifier.classify(
            CompositeRegimeInput::new(
                2_000,
                8_000,
                50,
                100,
                50,
                1_000_000_000,
                3_600_000_000_000,
                false,
                8_000,
                8_000,
            )
            .expect("input"),
        );

        assert_eq!(snapshot.trend(), TrendRegimeKind::Chop);
        assert_eq!(snapshot.liquidity(), LiquidityRegimeKind::Hidden);
        assert_eq!(snapshot.spread(), SpreadRegimeKind::Wide);
        assert_eq!(snapshot.session(), SessionRegimeKind::NewsShock);
        assert!(snapshot.volatile());
        assert_eq!(snapshot.hidden_liquidity_proxy_bps(), 8_000);
        assert!(snapshot.transition_confidence_bps() > 0);
    }

    #[test]
    fn composite_regime_classifier_detects_continuous_trend() {
        let config = CompositeRegimeConfig::default();
        let classifier = CompositeRegimeClassifier::new(config);
        let snapshot = classifier.classify(
            CompositeRegimeInput::new(
                8_000,
                1_000,
                1,
                20,
                2_000,
                config.open_window_ns().saturating_add(60_000_000_000),
                config.close_window_ns().saturating_add(60_000_000_000),
                false,
                0,
                0,
            )
            .expect("input"),
        );

        assert_eq!(snapshot.trend(), TrendRegimeKind::Trend);
        assert_eq!(snapshot.liquidity(), LiquidityRegimeKind::Deep);
        assert_eq!(snapshot.spread(), SpreadRegimeKind::Tight);
        assert_eq!(snapshot.session(), SessionRegimeKind::Continuous);
        assert!(!snapshot.volatile());
    }

    #[test]
    fn composite_regime_primitives_reject_invalid_inputs() {
        assert_eq!(
            CompositeRegimeConfig::new(0, 0, 10, 20, 0, 0, 0, 0, 0, 0, 0),
            Err(AnalyticsError::InvalidTrade)
        );
        assert_eq!(
            CompositeRegimeInput::new(10_001, 0, 0, 0, 0, 0, 0, false, 0, 0),
            Err(AnalyticsError::InvalidTrade)
        );
    }

    #[test]
    fn feed_quality_tracks_sequence_and_book_degradation() {
        let config = FeedQualityConfig::new(10, 20, 1).expect("config");
        let mut tracker = FeedQualityTracker::new(config);

        assert!(tracker
            .on_event(FeedQualityEvent::new(Some(10), 100, 105, Some(99), Some(101)).unwrap())
            .is_ok());
        let gap = tracker
            .on_event(FeedQualityEvent::new(Some(12), 110, 115, Some(100), Some(100)).unwrap());
        assert!(gap.contains(FeedQualityFlags::SEQUENCE_GAP));
        assert!(gap.contains(FeedQualityFlags::LOCKED_BOOK));
        let crossed = tracker
            .on_event(FeedQualityEvent::new(Some(12), 111, 200, Some(102), Some(101)).unwrap());
        assert!(crossed.contains(FeedQualityFlags::DUPLICATE));
        assert!(crossed.contains(FeedQualityFlags::STALE));
        assert!(crossed.contains(FeedQualityFlags::TIMESTAMP_SKEW));
        assert!(crossed.contains(FeedQualityFlags::CROSSED_BOOK));

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.events(), 3);
        assert_eq!(snapshot.sequence_gap_events(), 1);
        assert_eq!(snapshot.sequence_gap_units(), 1);
        assert_eq!(snapshot.duplicate_events(), 1);
        assert_eq!(snapshot.locked_book_events(), 1);
        assert_eq!(snapshot.crossed_book_events(), 1);
        assert_eq!(snapshot.stale_events(), 1);
        assert_eq!(snapshot.timestamp_skew_events(), 1);
        assert_eq!(snapshot.last_sequence(), Some(12));
        assert!(snapshot.health_score_bps() < 10_000);
        assert_eq!(snapshot.sequence_gap_rate_bps(), 3_333);
    }

    #[test]
    fn feed_quality_tracks_out_of_order_and_resets() {
        let mut tracker = FeedQualityTracker::default();
        tracker.on_event(FeedQualityEvent::new(Some(100), 100, 100, None, None).unwrap());

        let old = tracker.on_event(FeedQualityEvent::new(Some(90), 90, 100, None, None).unwrap());
        assert!(old.contains(FeedQualityFlags::OUT_OF_ORDER));

        let reset = tracker.on_event(FeedQualityEvent::new(Some(1), 101, 101, None, None).unwrap());
        assert!(reset.contains(FeedQualityFlags::SEQUENCE_RESET));

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.out_of_order_events(), 2);
        assert_eq!(snapshot.sequence_reset_events(), 1);
        assert_eq!(snapshot.last_sequence(), Some(1));
        assert_eq!(snapshot.last_event_ts_ns(), 101);
    }

    #[test]
    fn replay_quality_report_accepts_clean_replay() {
        let mut tracker = FeedQualityTracker::default();
        tracker.on_event(FeedQualityEvent::new(Some(1), 100, 100, Some(99), Some(101)).unwrap());
        tracker.on_event(FeedQualityEvent::new(Some(2), 101, 101, Some(100), Some(102)).unwrap());

        let report = ReplayQualityAnalyzer::default().evaluate(tracker.snapshot());

        assert_eq!(report.events(), 2);
        assert!(report.min_events_met());
        assert_eq!(report.health_score_bps(), 10_000);
        assert_eq!(report.worst_issue_rate_bps(), 0);
        assert!(report.sequence_complete());
        assert_eq!(report.primary_issue(), FeedQualityFlags::OK);
        assert!(report.replay_usable());
        assert!(!report.operator_review_required());
    }

    #[test]
    fn replay_quality_report_flags_degraded_replay() {
        let config = FeedQualityConfig::new(10, 20, 1).expect("config");
        let mut tracker = FeedQualityTracker::new(config);
        tracker.on_event(FeedQualityEvent::new(Some(1), 100, 100, Some(99), Some(101)).unwrap());
        tracker.on_event(FeedQualityEvent::new(Some(4), 101, 150, Some(100), Some(100)).unwrap());
        tracker.on_event(FeedQualityEvent::new(Some(3), 90, 200, Some(103), Some(102)).unwrap());

        let analyzer =
            ReplayQualityAnalyzer::new(ReplayQualityConfig::new(3, 9_000, 100, 0).unwrap());
        let report = analyzer.evaluate(tracker.snapshot());

        assert!(report.min_events_met());
        assert!(!report.sequence_complete());
        assert!(report.sequence_gap_rate_bps() > 0);
        assert!(report.out_of_order_rate_bps() > 0);
        assert!(report.bad_book_rate_bps() > 0);
        assert!(report.timestamp_skew_rate_bps() > 0);
        assert!(report.worst_issue_rate_bps() > 100);
        assert!(report.flags().contains(FeedQualityFlags::SEQUENCE_GAP));
        assert!(report
            .primary_issue()
            .contains(FeedQualityFlags::CROSSED_BOOK));
        assert!(!report.replay_usable());
        assert!(report.operator_review_required());
    }

    #[test]
    fn replay_quality_config_rejects_invalid_thresholds() {
        assert_eq!(
            ReplayQualityConfig::new(0, 10_001, 0, 0),
            Err(AnalyticsError::InvalidQuality)
        );
        assert_eq!(
            ReplayQualityConfig::new(0, 0, 10_001, 0),
            Err(AnalyticsError::InvalidQuality)
        );
    }

    #[test]
    fn feature_schema_registers_stable_order_and_hash() {
        let mut schema = FeatureSchema::<4>::new().expect("schema");
        let spread = FeatureDefinition::new(
            FeatureId::new(1).unwrap(),
            "spread_bps",
            FeatureUnit::BasisPoints,
            1,
            MissingValuePolicy::Sentinel(i64::MIN),
        )
        .unwrap();
        let quality = FeatureDefinition::new(
            FeatureId::new(2).unwrap(),
            "quality_bps",
            FeatureUnit::ScoreBasisPoints,
            1,
            MissingValuePolicy::Zero,
        )
        .unwrap();

        assert_eq!(schema.register(spread), Ok(0));
        assert_eq!(schema.register(quality), Ok(1));
        assert_eq!(schema.register(spread), Err(AnalyticsError::InvalidFeature));

        assert_eq!(schema.len(), 2);
        assert_eq!(schema.index_of(FeatureId::new(2).unwrap()), Some(1));
        assert_eq!(schema.definition(0).unwrap().name(), "spread_bps");
        assert_ne!(schema.schema_hash(), 0);
    }

    #[test]
    fn feature_vector_writer_reuses_schema_defaults() {
        let mut schema = FeatureSchema::<2>::new().expect("schema");
        schema
            .register(
                FeatureDefinition::new(
                    FeatureId::new(1).unwrap(),
                    "spread_bps",
                    FeatureUnit::BasisPoints,
                    1,
                    MissingValuePolicy::Sentinel(-1),
                )
                .unwrap(),
            )
            .unwrap();
        schema
            .register(
                FeatureDefinition::new(
                    FeatureId::new(2).unwrap(),
                    "is_stale",
                    FeatureUnit::Boolean,
                    1,
                    MissingValuePolicy::Zero,
                )
                .unwrap(),
            )
            .unwrap();

        let mut writer = FeatureVectorWriter::new(&schema);
        assert_eq!(writer.finish().values(), &[-1, 0]);
        writer.set(0, 25, FeatureQuality::Good).unwrap();
        assert_eq!(
            writer.set(2, 1, FeatureQuality::Good),
            Err(AnalyticsError::InvalidFeature)
        );

        let vector = writer.finish();

        assert_eq!(vector.len(), 2);
        assert_eq!(vector.value(0), Some(25));
        assert_eq!(vector.value(1), Some(0));
        assert_eq!(vector.quality(0), Some(FeatureQuality::Good));
        assert_eq!(vector.quality(1), Some(FeatureQuality::Missing));
        assert_eq!(vector.schema_hash(), schema.schema_hash());
    }

    #[test]
    fn resiliency_tracker_measures_spread_recovery() {
        let config = ResiliencyConfig::new(5, 1_000, 25, 5_000, 8, 9_000).expect("config");
        let mut tracker = ResiliencyTracker::new(config);

        let quiet = tracker.on_sample(ResiliencySample::new(100, 5, 500, 500).unwrap());
        assert!(!quiet.active_shock());
        assert_eq!(quiet.score_bps(), 10_000);

        let shock = tracker.on_sample(ResiliencySample::new(200, 30, 400, 400).unwrap());
        assert!(shock.active_shock());
        assert_eq!(shock.shock_count(), 1);
        assert_eq!(shock.last_shock_ts_ns(), 200);
        assert_eq!(shock.max_spread_bps(), 30);

        let recovery = tracker.on_sample(ResiliencySample::new(1_000_200, 7, 500, 500).unwrap());
        assert!(!recovery.active_shock());
        assert_eq!(recovery.recovery_count(), 1);
        assert_eq!(recovery.last_recovery_time_ns(), 1_000_000);
        assert!(recovery.score_bps() < 10_000);
    }

    #[test]
    fn resiliency_tracker_detects_depth_depletion() {
        let config = ResiliencyConfig::new(5, 1_000, 25, 5_000, 8, 9_000).expect("config");
        let mut tracker = ResiliencyTracker::new(config);

        let shock = tracker.on_sample(ResiliencySample::new(100, 5, 200, 200).unwrap());
        assert!(shock.active_shock());
        assert_eq!(shock.min_depth(), 400);

        tracker.reset();
        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 0);
        assert!(!snapshot.active_shock());
        assert_eq!(snapshot.min_depth(), 0);
        assert_eq!(snapshot.score_bps(), 10_000);
    }

    #[test]
    fn queue_fill_tracker_updates_on_trade_and_cancel() {
        let config = QueueFillConfig::new(5_000, 100, 1_000_000_000).expect("config");
        let estimate = QueuePositionEstimate::new(100, 50, 200, 1).expect("estimate");
        let mut tracker = QueueFillTracker::new(config, estimate);

        let after_trade = tracker
            .on_update(QueueFillUpdate::new(QueueUpdateKind::Trade, 40, 160, 2).expect("update"));
        assert_eq!(after_trade.qty_ahead(), 60);
        assert_eq!(after_trade.own_qty_remaining(), 50);
        assert!(after_trade.fill_probability_bps() > 0);
        assert_eq!(after_trade.expected_time_to_fill_ns(), 1_100_000_000);

        let after_cancel = tracker
            .on_update(QueueFillUpdate::new(QueueUpdateKind::Cancel, 20, 140, 3).expect("update"));
        assert_eq!(after_cancel.qty_ahead(), 50);
        assert!(after_cancel.top_level_survival_bps() > 0);
    }

    #[test]
    fn queue_fill_tracker_tracks_amend_queue_loss() {
        let config = QueueFillConfig::new(0, 100, 1_000_000_000).expect("config");
        let estimate = QueuePositionEstimate::new(10, 10, 20, 1).expect("estimate");
        let mut tracker = QueueFillTracker::new(config, estimate);

        let snapshot = tracker
            .on_update(QueueFillUpdate::new(QueueUpdateKind::Amend, 0, 100, 2).expect("update"));

        assert_eq!(snapshot.qty_ahead(), 100);
        assert_eq!(snapshot.queue_loss_after_amend(), 100);
        assert_eq!(snapshot.last_update_ts_ns(), 2);
        assert!(snapshot.maker_taker_score_bps() < 10_000);
    }

    #[test]
    fn queue_decision_prefers_passive_when_queue_is_healthy() {
        let fill_config = QueueFillConfig::new(5_000, 100, 1_000_000_000).expect("config");
        let estimate = QueuePositionEstimate::new(10, 10, 100, 1).expect("estimate");
        let snapshot = QueueFillTracker::new(fill_config, estimate).snapshot();
        let analyzer = QueueDecisionAnalyzer::new(
            QueueDecisionConfig::new(5_000, 5_000, 10_000_000_000).unwrap(),
        );

        let decision = analyzer.evaluate(
            QueueDecisionInput::new(snapshot, 10, 5, 1, 2, 1, 1_000, 0, 0).expect("input"),
        );

        assert_eq!(decision.expected_wait_penalty_bps(), 20);
        assert_eq!(decision.queue_priority_loss_bps(), 0);
        assert!(decision.passive_edge_bps() < 10);
        assert_eq!(decision.aggressive_cost_bps(), 7);
        assert!(decision.maker_taker_decision_score_bps() > 5_000);
        assert!(decision.prefer_passive());
        assert!(!decision.prefer_replace());
    }

    #[test]
    fn queue_decision_blocks_passive_when_fill_is_weak() {
        let fill_config = QueueFillConfig::new(0, 10, 1_000_000_000).expect("config");
        let estimate = QueuePositionEstimate::new(1_000, 100, 1_100, 1).expect("estimate");
        let snapshot = QueueFillTracker::new(fill_config, estimate).snapshot();
        let decision = QueueDecisionAnalyzer::default().evaluate(
            QueueDecisionInput::new(snapshot, 1, 0, 0, 1, 100, 10_000, 0, 0).expect("input"),
        );

        assert!(decision.expected_wait_penalty_bps() > 0);
        assert!(decision.maker_taker_decision_score_bps() < 5_000);
        assert!(!decision.prefer_passive());
    }

    #[test]
    fn queue_decision_prices_cancel_replace_priority_loss() {
        let fill_config = QueueFillConfig::new(5_000, 100, 1_000_000_000).expect("config");
        let estimate = QueuePositionEstimate::new(10, 10, 100, 1).expect("estimate");
        let snapshot = QueueFillTracker::new(fill_config, estimate).snapshot();
        let analyzer =
            QueueDecisionAnalyzer::new(QueueDecisionConfig::new(0, 0, 10_000_000_000).unwrap());

        let decision = analyzer.evaluate(
            QueueDecisionInput::new(snapshot, 1, 0, 0, 1, 0, 1_000, 2_000, 10).expect("input"),
        );

        assert_eq!(decision.queue_priority_loss_bps(), 1_000);
        assert!(decision.cancel_replace_cost_bps() < 0);
        assert!(decision.prefer_replace());
    }

    #[test]
    fn queue_decision_primitives_reject_invalid_inputs() {
        let snapshot = QueueFillTracker::new(
            QueueFillConfig::new(0, 1, 1).unwrap(),
            QueuePositionEstimate::new(0, 1, 1, 1).unwrap(),
        )
        .snapshot();

        assert_eq!(
            QueueDecisionConfig::new(10_001, 0, 1),
            Err(AnalyticsError::InvalidQueue)
        );
        assert_eq!(
            QueueDecisionInput::new(snapshot, 0, 0, 0, 0, 0, 10_001, 0, 0),
            Err(AnalyticsError::InvalidQueue)
        );
        assert_eq!(
            QueueDecisionInput::new(snapshot, 0, 0, 0, 0, 0, 0, 0, -1),
            Err(AnalyticsError::InvalidQueue)
        );
    }

    #[test]
    fn pattern_risk_flags_layering_and_quote_stuffing() {
        let classifier = PatternRiskClassifier::default();
        let snapshot = classifier.classify(
            PatternRiskInput::new(
                800,
                900,
                10,
                8_000,
                5,
                PatternRiskLiquidity::new(10, 1_000).unwrap(),
                1_000_000,
            )
            .unwrap(),
        );

        assert!(snapshot.spoofing_layering_risk_bps() > 5_000);
        assert_eq!(snapshot.quote_stuffing_risk_bps(), 10_000);
        assert_eq!(
            snapshot.overall_risk_bps(),
            snapshot.quote_stuffing_risk_bps()
        );
    }

    #[test]
    fn pattern_risk_scores_stop_run_and_absorption() {
        let classifier = PatternRiskClassifier::default();
        let stop_run = classifier.classify(
            PatternRiskInput::new(
                10,
                10,
                100,
                1_000,
                50,
                PatternRiskLiquidity::new(2_000, 1_000).unwrap(),
                1_000_000,
            )
            .unwrap(),
        );
        let absorption = classifier.classify(
            PatternRiskInput::new(
                10,
                10,
                100,
                1_000,
                1,
                PatternRiskLiquidity::new(2_000, 1_000).unwrap(),
                1_000_000,
            )
            .unwrap(),
        );

        assert!(stop_run.stop_run_risk_bps() > absorption.stop_run_risk_bps());
        assert!(absorption.absorption_risk_bps() > stop_run.absorption_risk_bps());
    }

    #[test]
    fn pattern_detail_analyzer_detects_iceberg_and_failed_breakout() {
        let analyzer = PatternDetailAnalyzer::default();
        let snapshot = analyzer.evaluate(
            PatternDetailInput::new(8, 4, 1_000, 100, 4, 8_000, 500, 2, 30, 35, 1_000)
                .expect("input"),
        );

        assert!(snapshot.iceberg_risk_bps() > 0);
        assert_eq!(snapshot.hidden_distribution_bps(), 0);
        assert!(snapshot.hidden_accumulation_bps() > 0);
        assert_eq!(snapshot.stacked_imbalance_risk_bps(), 10_000);
        assert!(snapshot.absorption_strength_bps() > 0);
        assert_eq!(snapshot.failed_breakout_risk_bps(), 10_000);
        assert_eq!(snapshot.overall_risk_bps(), 10_000);
    }

    #[test]
    fn pattern_detail_analyzer_detects_hidden_distribution() {
        let snapshot = PatternDetailAnalyzer::default()
            .evaluate(PatternDetailInput::new(3, 3, 500, 100, 0, 0, 0, 0, 0, 0, -500).unwrap());

        assert_eq!(snapshot.hidden_accumulation_bps(), 0);
        assert!(snapshot.hidden_distribution_bps() > 0);
    }

    #[test]
    fn pattern_detail_primitives_reject_invalid_inputs() {
        assert_eq!(
            PatternDetailConfig::new(0, 10_001, 0, 0),
            Err(AnalyticsError::InvalidPattern)
        );
        assert_eq!(
            PatternDetailInput::new(0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0),
            Err(AnalyticsError::InvalidPattern)
        );
    }

    #[test]
    fn venue_route_tracker_computes_rates_and_latency() {
        let mut tracker = VenueRouteTracker::new();
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Sent, 100, 0, 10).unwrap());
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Fill, 60, 100, 20).unwrap());
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Reject, 0, 0, 30).unwrap());
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Cancel, 40, 0, 40).unwrap());

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.sent(), 1);
        assert_eq!(snapshot.fills(), 1);
        assert_eq!(snapshot.rejects(), 1);
        assert_eq!(snapshot.cancels(), 1);
        assert_eq!(snapshot.sent_qty(), 100);
        assert_eq!(snapshot.filled_qty(), 60);
        assert_eq!(snapshot.fill_rate_bps(), 3_333);
        assert_eq!(snapshot.avg_quote_to_fill_latency_ns(), 100);
        assert_eq!(snapshot.max_market_data_to_order_latency_ns(), 40);
        assert!(snapshot.route_health_bps() < snapshot.fill_rate_bps());
    }

    #[test]
    fn venue_route_tracker_resets_state() {
        let mut tracker = VenueRouteTracker::new();
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Sent, 100, 0, 10).unwrap());

        tracker.reset();

        assert_eq!(tracker.snapshot().sent(), 0);
        assert_eq!(tracker.snapshot().route_health_bps(), 0);
    }

    #[test]
    fn venue_route_quality_scores_healthy_route() {
        let mut tracker = VenueRouteTracker::new();
        tracker.on_event(VenueRouteEvent::new(VenueRouteEventKind::Sent, 100, 0, 100_000).unwrap());
        tracker.on_event(
            VenueRouteEvent::new(VenueRouteEventKind::Fill, 100, 100_000, 100_000).unwrap(),
        );

        let input =
            VenueRouteQualityInput::new(tracker.snapshot(), 9_000, 500, 9_500, 9_500).unwrap();
        let snapshot = VenueRouteQualityAnalyzer::default().evaluate(input);

        assert!(snapshot.latency_score_bps() > 8_000);
        assert!(snapshot.reliability_score_bps() > 8_000);
        assert!(snapshot.route_quality_score_bps() > 8_000);
        assert_eq!(snapshot.route_drift_bps(), 0);
        assert!(!snapshot.route_degraded());
    }

    #[test]
    fn venue_route_quality_detects_degraded_route() {
        let mut tracker = VenueRouteTracker::new();
        tracker
            .on_event(VenueRouteEvent::new(VenueRouteEventKind::Sent, 100, 0, 2_000_000).unwrap());
        tracker
            .on_event(VenueRouteEvent::new(VenueRouteEventKind::Reject, 0, 0, 2_000_000).unwrap());
        tracker.on_event(
            VenueRouteEvent::new(VenueRouteEventKind::Cancel, 100, 0, 2_000_000).unwrap(),
        );

        let input =
            VenueRouteQualityInput::new(tracker.snapshot(), 2_000, 9_000, 1_000, 9_000).unwrap();
        let snapshot = VenueRouteQualityAnalyzer::default().evaluate(input);

        assert!(snapshot.route_degraded());
        assert!(snapshot.route_drift_bps() >= 9_000);
        assert!(snapshot.route_quality_score_bps() < 5_000);
    }

    #[test]
    fn venue_route_quality_rejects_invalid_inputs() {
        assert_eq!(
            VenueRouteQualityConfig::new(0, 1_000, 5_000, 1, 1, 5_000, 2_000),
            Err(AnalyticsError::InvalidRoute)
        );
        assert_eq!(
            VenueRouteQualityConfig::new(7_500, 10_001, 5_000, 1, 1, 5_000, 2_000),
            Err(AnalyticsError::InvalidRoute)
        );
        assert_eq!(
            VenueRouteQualityConfig::new(7_500, 1_000, 5_000, 0, 1, 5_000, 2_000),
            Err(AnalyticsError::InvalidRoute)
        );
        assert_eq!(
            VenueRouteQualityInput::new(VenueRouteTracker::new().snapshot(), 10_001, 0, 0, 0),
            Err(AnalyticsError::InvalidRoute)
        );
    }

    #[test]
    fn cross_asset_tracker_computes_correlation_beta_and_basis() {
        let mut tracker =
            CrossAssetTracker::<4>::new(CrossAssetConfig::default()).expect("tracker");
        tracker.on_sample(CrossAssetSample::new(100_000, 200_000, 1).unwrap());
        tracker.on_sample(CrossAssetSample::new(101_000, 202_000, 2).unwrap());
        tracker.on_sample(CrossAssetSample::new(102_000, 204_000, 3).unwrap());

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 2);
        assert_eq!(snapshot.last_ts_ns(), 3);
        assert!(snapshot.correlation_bps() > 9_000);
        assert!(snapshot.beta_bps() > 9_000);
        assert!(!snapshot.correlation_breakdown());
        assert!(snapshot.basis_pressure_bps() < 0);
        assert!(snapshot.basis_pressure());
        assert!(snapshot.lead_lag_score_bps() > 9_000);
    }

    #[test]
    fn cross_asset_tracker_flags_divergence_and_resets() {
        let config = CrossAssetConfig::new(50, 25, 2_000).expect("config");
        let mut tracker = CrossAssetTracker::<3>::new(config).expect("tracker");
        tracker.on_sample(CrossAssetSample::new(100_000, 100_000, 1).unwrap());
        tracker.on_sample(CrossAssetSample::new(101_000, 99_000, 2).unwrap());
        tracker.on_sample(CrossAssetSample::new(102_000, 98_000, 3).unwrap());
        tracker.on_sample(CrossAssetSample::new(103_000, 97_000, 4).unwrap());

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.samples(), 3);
        assert!(snapshot.correlation_bps() < 0);
        assert!(snapshot.pair_divergence_bps() < 0);
        assert!(snapshot.pair_divergence());

        tracker.reset();

        let reset = tracker.snapshot();
        assert_eq!(reset.samples(), 0);
        assert_eq!(reset.last_ts_ns(), 0);
        assert_eq!(reset.correlation_bps(), 0);
    }

    #[test]
    fn cross_asset_diagnostics_score_synchronized_relationship() {
        let mut tracker =
            CrossAssetTracker::<4>::new(CrossAssetConfig::default()).expect("tracker");
        tracker.on_sample(CrossAssetSample::new(100_000, 100_100, 1).unwrap());
        tracker.on_sample(CrossAssetSample::new(101_000, 101_100, 2).unwrap());
        tracker.on_sample(CrossAssetSample::new(102_000, 102_100, 3).unwrap());

        let diagnostic = CrossAssetDiagnosticAnalyzer::default().evaluate(
            CrossAssetDiagnosticInput::new(tracker.snapshot(), 1_000_000, 1_100_000, 5, 100)
                .expect("input"),
        );

        assert_eq!(diagnostic.sample_skew_ns(), 100_000);
        assert!(diagnostic.synchronization_quality_bps() > 8_000);
        assert!(diagnostic.latency_adjusted_correlation_bps() > 8_000);
        assert!(!diagnostic.cross_venue_divergence());
        assert!(!diagnostic.component_imbalance());
        assert!(!diagnostic.relationship_degraded());
    }

    #[test]
    fn cross_asset_diagnostics_detect_degraded_relationship() {
        let config = CrossAssetConfig::new(50, 25, 2_000).expect("config");
        let mut tracker = CrossAssetTracker::<4>::new(config).expect("tracker");
        tracker.on_sample(CrossAssetSample::new(100_000, 100_000, 1).unwrap());
        tracker.on_sample(CrossAssetSample::new(101_000, 99_000, 2).unwrap());
        tracker.on_sample(CrossAssetSample::new(102_000, 98_000, 3).unwrap());

        let diagnostic = CrossAssetDiagnosticAnalyzer::default().evaluate(
            CrossAssetDiagnosticInput::new(tracker.snapshot(), 1_000_000, 3_000_000, 100, 2_000)
                .expect("input"),
        );

        assert_eq!(diagnostic.synchronization_quality_bps(), 0);
        assert_eq!(diagnostic.latency_adjusted_correlation_bps(), 0);
        assert!(diagnostic.cross_venue_divergence());
        assert!(diagnostic.component_imbalance());
        assert!(diagnostic.aggregate_divergence_score_bps() >= 250);
        assert!(diagnostic.relationship_degraded());
    }

    #[test]
    fn cross_asset_diagnostics_reject_invalid_inputs() {
        let snapshot = CrossAssetTracker::<2>::new(CrossAssetConfig::default())
            .expect("tracker")
            .snapshot();

        assert_eq!(
            CrossAssetDiagnosticConfig::new(10_001, 250, 25, 1_000, 1, 5_000),
            Err(AnalyticsError::InvalidCrossAsset)
        );
        assert_eq!(
            CrossAssetDiagnosticConfig::new(2_000, 250, 25, 1_000, 0, 5_000),
            Err(AnalyticsError::InvalidCrossAsset)
        );
        assert_eq!(
            CrossAssetDiagnosticInput::new(snapshot, 0, 1, 0, 0),
            Err(AnalyticsError::InvalidCrossAsset)
        );
    }

    #[test]
    fn option_flow_tracker_computes_pressure_iv_and_gamma() {
        let mut tracker = OptionFlowTracker::new();
        tracker.on_sample(
            OptionFlowSample::new(OptionKind::Call, 100, 1_000, 50_000, 2_000, 1_000)
                .expect("call"),
        );
        tracker.on_sample(
            OptionFlowSample::new(OptionKind::Put, 200, 1_500, 150_000, 3_000, -2_000)
                .expect("put"),
        );

        let snapshot = tracker.snapshot();

        assert_eq!(snapshot.call_volume(), 100);
        assert_eq!(snapshot.put_volume(), 200);
        assert_eq!(snapshot.put_call_volume_ratio_bps(), 20_000);
        assert_eq!(snapshot.put_call_open_interest_ratio_bps(), 15_000);
        assert_eq!(snapshot.put_call_premium_ratio_bps(), 30_000);
        assert_eq!(snapshot.implied_vol_flow_bps(), 2_750);
        assert_eq!(snapshot.net_gamma_exposure(), -1_000);
        assert!(snapshot.put_call_pressure_bps() > 0);

        tracker.reset();
        assert_eq!(tracker.snapshot().call_volume(), 0);
    }

    #[test]
    fn futures_basis_analyzer_computes_roll_and_funding_divergence() {
        let snapshot = FuturesBasisAnalyzer::analyze(
            FuturesBasisInput::new(100_000, 101_000, 100_500, 101_000, 102_000, 25).expect("basis"),
        );

        assert_eq!(snapshot.basis_bps(), 100);
        assert_eq!(snapshot.fair_value_gap_bps(), 49);
        assert_eq!(snapshot.calendar_spread_bps(), 99);
        assert_eq!(snapshot.roll_pressure_bps(), -1);
        assert_eq!(snapshot.funding_basis_divergence_bps(), 75);
    }

    #[test]
    fn derivatives_diagnostics_score_balanced_surface() {
        let mut options = OptionFlowTracker::new();
        options.on_sample(
            OptionFlowSample::new(OptionKind::Call, 100, 1_000, 100_000, 2_000, 500).expect("call"),
        );
        options.on_sample(
            OptionFlowSample::new(OptionKind::Put, 100, 1_000, 100_000, 2_100, -500).expect("put"),
        );
        let basis = FuturesBasisAnalyzer::analyze(
            FuturesBasisInput::new(100_000, 100_100, 100_100, 100_100, 100_150, 5).expect("basis"),
        );

        let diagnostic =
            DerivativesDiagnosticAnalyzer::default().evaluate(DerivativesDiagnosticInput::new(
                options.snapshot(),
                basis,
                DerivativesVolatilitySurface::new(2_000, 2_100, 2_000, 2_000, 2_050, 1_900)
                    .expect("surface"),
            ));

        assert_eq!(diagnostic.skew_bps(), 100);
        assert_eq!(diagnostic.term_structure_bps(), 50);
        assert_eq!(diagnostic.iv_richness_bps(), 100);
        assert_eq!(diagnostic.gamma_pressure_bps(), 0);
        assert!(!diagnostic.skew_stress());
        assert!(!diagnostic.term_structure_stress());
        assert!(!diagnostic.iv_richness_stress());
        assert!(!diagnostic.gamma_pressure_stress());
        assert!(!diagnostic.derivatives_stressed());
    }

    #[test]
    fn derivatives_diagnostics_detect_stressed_surface_and_basis() {
        let mut options = OptionFlowTracker::new();
        options.on_sample(
            OptionFlowSample::new(OptionKind::Call, 100, 1_000, 50_000, 2_000, 10_000)
                .expect("call"),
        );
        options.on_sample(
            OptionFlowSample::new(OptionKind::Put, 400, 1_000, 200_000, 4_500, 30_000)
                .expect("put"),
        );
        let basis = FuturesBasisAnalyzer::analyze(
            FuturesBasisInput::new(100_000, 102_000, 100_500, 100_000, 103_000, -500)
                .expect("basis"),
        );

        let diagnostic =
            DerivativesDiagnosticAnalyzer::default().evaluate(DerivativesDiagnosticInput::new(
                options.snapshot(),
                basis,
                DerivativesVolatilitySurface::new(4_000, 5_000, 2_000, 3_000, 5_000, 1_000)
                    .expect("surface"),
            ));

        assert!(diagnostic.skew_stress());
        assert!(diagnostic.term_structure_stress());
        assert!(diagnostic.iv_richness_stress());
        assert!(diagnostic.gamma_pressure_stress());
        assert!(diagnostic.basis_stress());
        assert!(diagnostic.funding_basis_stress());
        assert!(diagnostic.derivatives_stressed());
    }

    #[test]
    fn derivatives_diagnostics_reject_invalid_inputs() {
        let options = OptionFlowTracker::new().snapshot();
        let basis = FuturesBasisAnalyzer::analyze(
            FuturesBasisInput::new(100_000, 100_100, 100_100, 100_100, 100_150, 5).expect("basis"),
        );

        assert_eq!(
            DerivativesDiagnosticConfig::new(0, 0, 0, 10_001, 0, 0, 0),
            Err(AnalyticsError::InvalidDerivative)
        );
        assert_eq!(
            DerivativesVolatilitySurface::new(0, 1, 1, 1, 1, 0),
            Err(AnalyticsError::InvalidDerivative)
        );
        let surface = DerivativesVolatilitySurface::new(1, 1, 1, 1, 1, 0).expect("surface");
        let input = DerivativesDiagnosticInput::new(options, basis, surface);
        assert_eq!(input.volatility_surface().realized_vol_bps(), 0);
    }
}
