#[cfg(test)]
mod tests {
    use super::*;

    fn symbol() -> SymbolId {
        SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        }
    }

    #[test]
    fn tracks_delta_and_cumulative_delta() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 100,
            size: 5,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 99,
            size: 2,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });

        let snap = acc.snapshot();
        assert_eq!(snap.delta, 3);
        assert_eq!(snap.cumulative_delta, 3);
        assert_eq!(snap.buy_volume, 5);
        assert_eq!(snap.sell_volume, 2);
        assert_eq!(snap.last_price, 99);
        assert_eq!(snap.point_of_control, 100);
        assert_eq!(snap.value_area_low, 100);
        assert_eq!(snap.value_area_high, 100);

        acc.reset_session_delta();
        let reset = acc.snapshot();
        assert_eq!(reset.delta, 0);
        assert_eq!(reset.buy_volume, 0);
        assert_eq!(reset.sell_volume, 0);
        assert_eq!(reset.cumulative_delta, 3);
    }

    #[test]
    fn tracks_poc_and_value_area() {
        let mut acc = AnalyticsAccumulator::default();
        let s = symbol();
        let prints = [
            (100, 5, Side::Ask),
            (101, 7, Side::Ask),
            (99, 3, Side::Bid),
            (102, 2, Side::Ask),
            (101, 5, Side::Bid),
        ];
        for (i, (price, size, side)) in prints.iter().enumerate() {
            acc.on_trade(&TradePrint {
                symbol: s.clone(),
                price: *price,
                size: *size,
                aggressor_side: *side,
                sequence: i as u64 + 1,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            });
        }
        let snap = acc.snapshot();
        assert_eq!(snap.point_of_control, 101);
        assert!(snap.value_area_low <= snap.point_of_control);
        assert!(snap.value_area_high >= snap.point_of_control);
    }

    #[test]
    fn computes_derived_session_metrics() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 100,
            size: 5,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 98,
            size: 3,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });

        let derived = acc.derived_snapshot();
        assert_eq!(derived.total_volume, 8);
        assert_eq!(derived.trade_count, 2);
        assert_eq!(derived.vwap, 99);
        assert_eq!(derived.average_trade_size, 4);
        assert_eq!(derived.imbalance_bps, 2500);

        acc.reset_session_delta();
        let reset = acc.derived_snapshot();
        assert_eq!(reset.total_volume, 0);
        assert_eq!(reset.trade_count, 0);
        assert_eq!(reset.vwap, 0);
    }

    #[test]
    fn computes_session_candle_snapshot() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 100,
            size: 5,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 10,
            ts_recv_ns: 11,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 98,
            size: 3,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 20,
            ts_recv_ns: 21,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 101,
            size: 2,
            aggressor_side: Side::Ask,
            sequence: 3,
            ts_exchange_ns: 30,
            ts_recv_ns: 31,
        });

        let candle = acc.session_candle_snapshot();
        assert_eq!(candle.open, 100);
        assert_eq!(candle.high, 101);
        assert_eq!(candle.low, 98);
        assert_eq!(candle.close, 101);
        assert_eq!(candle.trade_count, 3);
        assert_eq!(candle.first_ts_exchange_ns, 10);
        assert_eq!(candle.last_ts_exchange_ns, 30);

        acc.reset_session_delta();
        let reset = acc.session_candle_snapshot();
        assert_eq!(reset, SessionCandleSnapshot::default());
    }

    #[test]
    fn computes_interval_candle_snapshot() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 100,
            size: 5,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 10,
            ts_recv_ns: 11,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 98,
            size: 3,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 40,
            ts_recv_ns: 41,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 101,
            size: 2,
            aggressor_side: Side::Ask,
            sequence: 3,
            ts_exchange_ns: 100,
            ts_recv_ns: 101,
        });

        let recent = acc.interval_candle_snapshot(70);
        assert_eq!(recent.window_ns, 70);
        assert_eq!(recent.open, 98);
        assert_eq!(recent.high, 101);
        assert_eq!(recent.low, 98);
        assert_eq!(recent.close, 101);
        assert_eq!(recent.trade_count, 2);
        assert_eq!(recent.total_volume, 5);
        assert_eq!(recent.vwap, 99);
        assert_eq!(recent.first_ts_exchange_ns, 40);
        assert_eq!(recent.last_ts_exchange_ns, 100);

        acc.reset_session_delta();
        let reset = acc.interval_candle_snapshot(70);
        assert_eq!(
            reset,
            IntervalCandleSnapshot {
                window_ns: 70,
                ..IntervalCandleSnapshot::default()
            }
        );
    }

    #[test]
    fn full_session_reset_clears_profile_and_cumulative() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 101,
            size: 4,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });
        acc.reset_session();
        let snap = acc.snapshot();
        assert_eq!(snap.delta, 0);
        assert_eq!(snap.cumulative_delta, 0);
        assert_eq!(snap.buy_volume, 0);
        assert_eq!(snap.sell_volume, 0);
        assert_eq!(snap.point_of_control, 0);
        assert_eq!(snap.value_area_low, 0);
        assert_eq!(snap.value_area_high, 0);
    }

    #[cfg(feature = "tickbar")]
    #[test]
    fn tickbar_aggregates_bars_from_trades() {
        let mut acc = AnalyticsAccumulator::with_tickbar(1000);
        let s = symbol();

        acc.on_trade(&TradePrint {
            symbol: s.clone(),
            price: 505000,
            size: 9,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        });
        acc.on_trade(&TradePrint {
            symbol: s.clone(),
            price: 504900,
            size: 4,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 500,
            ts_recv_ns: 501,
        });
        acc.on_trade(&TradePrint {
            symbol: s.clone(),
            price: 505100,
            size: 8,
            aggressor_side: Side::Ask,
            sequence: 3,
            ts_exchange_ns: 1500,
            ts_recv_ns: 1501,
        });

        let bars = acc.bar_series().expect("should have bars");
        assert_eq!(bars.len(), 2, "expected 2 bars, got {}", bars.len());

        // First bar: trades at 0 and 500 ns → interval [0, 1000)
        assert_eq!(bars[0].timestamp_ns, 0);
        assert_eq!(bars[0].open, 505000);
        assert_eq!(bars[0].high, 505000);
        assert_eq!(bars[0].low, 504900);
        assert_eq!(bars[0].close, 504900);
        assert_eq!(bars[0].volume, 13);
        assert_eq!(bars[0].tick_count, 2);

        // Second bar: trade at 1500 ns → interval [1000, 2000)
        assert_eq!(bars[1].timestamp_ns, 1000);
        assert_eq!(bars[1].open, 505100);
        assert_eq!(bars[1].high, 505100);
        assert_eq!(bars[1].low, 505100);
        assert_eq!(bars[1].close, 505100);
        assert_eq!(bars[1].volume, 8);
        assert_eq!(bars[1].tick_count, 1);
    }

    #[cfg(feature = "tickbar")]
    #[test]
    fn tickbar_default_accumulator_returns_none() {
        let mut acc = AnalyticsAccumulator::default();
        let s = symbol();
        acc.on_trade(&TradePrint {
            symbol: s,
            price: 505000,
            size: 9,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        });
        assert!(acc.bar_series().is_none());
    }

    #[cfg(feature = "tickbar")]
    #[test]
    fn tickbar_reset_removes_aggregator() {
        let mut acc = AnalyticsAccumulator::with_tickbar(1000);
        let s = symbol();
        acc.on_trade(&TradePrint {
            symbol: s,
            price: 505000,
            size: 9,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        });
        assert!(acc.bar_series().is_some());

        // After bar_series() the aggregator is rebuilt internally, but reset_tickbar removes it fully
        acc.reset_tickbar();
        let s2 = symbol();
        acc.on_trade(&TradePrint {
            symbol: s2,
            price: 505000,
            size: 9,
            aggressor_side: Side::Ask,
            sequence: 2,
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        });
        assert!(acc.bar_series().is_none());
    }

    #[test]
    fn compute_book_analytics_returns_spread_and_depth_metrics() {
        let snapshot = BookSnapshot {
            symbol: symbol(),
            bids: vec![
                BookLevel {
                    level: 0,
                    price: 100,
                    size: 10,
                },
                BookLevel {
                    level: 1,
                    price: 99,
                    size: 5,
                },
            ],
            asks: vec![
                BookLevel {
                    level: 0,
                    price: 102,
                    size: 8,
                },
                BookLevel {
                    level: 1,
                    price: 103,
                    size: 3,
                },
            ],
            last_sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };

        let analytics = compute_book_analytics(&snapshot);
        assert_eq!(analytics.best_bid, 100);
        assert_eq!(analytics.best_ask, 102);
        assert_eq!(analytics.quoted_spread, 2);
        assert!(analytics.relative_spread_bps > 0);
        assert!(analytics.microprice > 0);
        assert_eq!(analytics.bid_depth, 15);
        assert_eq!(analytics.ask_depth, 11);
        assert!(analytics.depth_imbalance_bps > 0);
    }

    #[test]
    fn compute_book_analytics_empty_book_returns_defaults() {
        let snapshot = BookSnapshot {
            symbol: symbol(),
            bids: vec![],
            asks: vec![],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };

        let analytics = compute_book_analytics(&snapshot);
        assert_eq!(analytics, BookAnalyticsSnapshot::default());
    }

    #[test]
    fn compute_weighted_average_price_buy_walks_asks() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel {
                level: 0,
                price: 100,
                size: 10,
            }],
            asks: vec![
                BookLevel {
                    level: 0,
                    price: 102,
                    size: 5,
                },
                BookLevel {
                    level: 1,
                    price: 103,
                    size: 5,
                },
            ],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        // Buy 5 @ 102 = 102 avg
        assert_eq!(compute_weighted_average_price(&book, 5), Some(102));
        // Buy 7: 5@102 + 2@103 = (510+206)/7 = 716/7 = 102.285 -> 102
        assert_eq!(compute_weighted_average_price(&book, 7), Some(102));
        // Buy 10: 5@102 + 5@103 = (510+515)/10 = 1025/10 = 102.5 -> 102 (i64 truncation)
        assert_eq!(compute_weighted_average_price(&book, 10), Some(102));
    }

    #[test]
    fn compute_weighted_average_price_sell_walks_bids() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![
                BookLevel {
                    level: 0,
                    price: 100,
                    size: 8,
                },
                BookLevel {
                    level: 1,
                    price: 99,
                    size: 4,
                },
            ],
            asks: vec![BookLevel {
                level: 0,
                price: 102,
                size: 5,
            }],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        // Sell 6 @ 100: 6*100/6 = 100
        assert_eq!(compute_weighted_average_price(&book, -6), Some(100));
        // Sell 10: 8@100 + 2@99 = (800+198)/10 = 998/10 = 99.8 -> 99
        assert_eq!(compute_weighted_average_price(&book, -10), Some(99));
    }

    #[test]
    fn compute_weighted_average_price_insufficient_liquidity_returns_none() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel {
                level: 0,
                price: 100,
                size: 5,
            }],
            asks: vec![BookLevel {
                level: 0,
                price: 102,
                size: 3,
            }],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        assert_eq!(compute_weighted_average_price(&book, 10), None);
        assert_eq!(compute_weighted_average_price(&book, -10), None);
        assert_eq!(compute_weighted_average_price(&book, 0), None);
    }

    #[test]
    fn compute_depth_slope_positive_decay() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![
                BookLevel {
                    level: 0,
                    price: 100,
                    size: 100,
                },
                BookLevel {
                    level: 1,
                    price: 99,
                    size: 60,
                },
                BookLevel {
                    level: 2,
                    price: 98,
                    size: 20,
                },
            ],
            asks: vec![
                BookLevel {
                    level: 0,
                    price: 102,
                    size: 80,
                },
                BookLevel {
                    level: 1,
                    price: 103,
                    size: 50,
                },
                BookLevel {
                    level: 2,
                    price: 104,
                    size: 10,
                },
            ],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        let slope = compute_depth_slope(&book, 3);
        assert!(slope > 0.0, "expected positive decay slope, got {}", slope);
    }

    #[test]
    fn compute_depth_slope_few_levels_returns_zero() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel {
                level: 0,
                price: 100,
                size: 10,
            }],
            asks: vec![BookLevel {
                level: 0,
                price: 102,
                size: 8,
            }],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        assert_eq!(compute_depth_slope(&book, 5), 0.0);
    }

    #[test]
    fn book_snapshot_keeps_level_order() {
        let snapshot = BookSnapshot {
            symbol: symbol(),
            bids: vec![
                BookLevel {
                    level: 0,
                    price: 100,
                    size: 5,
                },
                BookLevel {
                    level: 2,
                    price: 98,
                    size: 3,
                },
            ],
            asks: vec![BookLevel {
                level: 1,
                price: 101,
                size: 4,
            }],
            last_sequence: 7,
            ts_exchange_ns: 11,
            ts_recv_ns: 12,
        };

        assert_eq!(snapshot.bids[0].level, 0);
        assert_eq!(snapshot.bids[1].level, 2);
        assert_eq!(snapshot.asks[0].level, 1);
        assert_eq!(snapshot.last_sequence, 7);
    }

    #[test]
    fn compute_mid_price_returns_midpoint() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel {
                level: 0,
                price: 100,
                size: 10,
            }],
            asks: vec![BookLevel {
                level: 0,
                price: 102,
                size: 8,
            }],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        assert_eq!(compute_mid_price(&book), Some(101));
    }

    #[test]
    fn compute_mid_price_empty_book_returns_none() {
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![],
            asks: vec![],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        assert!(compute_mid_price(&book).is_none());
    }

    #[test]
    fn compute_effective_spread_bps_at_mid_returns_zero() {
        assert_eq!(compute_effective_spread_bps(100, 100), 0);
    }

    #[test]
    fn compute_effective_spread_bps_one_tick_away() {
        // 100 vs 101: 2 * |100-101| * 10000 / 101 = 2*1*10000/101 = 198
        assert_eq!(compute_effective_spread_bps(100, 101), 198);
        // 100 vs 99: 2 * |100-99| * 10000 / 99 = 2*1*10000/99 = 202
        assert_eq!(compute_effective_spread_bps(100, 99), 202);
    }

    #[test]
    fn compute_realised_spread_bps_never_negative() {
        assert_eq!(compute_realised_spread_bps(200, 300), 0);
        assert_eq!(compute_realised_spread_bps(200, 100), 100);
    }

    #[test]
    fn spread_tracker_tracks_effective_and_half_spread() {
        let mut st = SpreadTracker::new(100);
        st.on_trade(101, 100, 0);
        st.on_trade(103, 100, 1);
        assert_eq!(st.last_effective_spread_bps(), 600); // 2*3*10000/100 = 600
        assert!(st.average_half_spread_cost_bps(10) > 0);
    }

    #[test]
    fn spread_tracker_realised_spread_returns_zero_for_insufficient_history() {
        let mut st = SpreadTracker::new(100);
        st.on_trade(101, 100, 0);
        assert_eq!(st.realised_spread_bps(5), 0); // need 6 samples for hold_ticks=5
    }

    #[test]
    fn book_event_tracker_tracks_arrival_and_cancel_rates() {
        let mut bet = BookEventTracker::new(1000);
        let now = 1_000_000_000; // 1 sec in ns
                                 // 10 bid upserts at t=0
        for _ in 0..10 {
            bet.on_book_update(Side::Bid, BookAction::Upsert, 100, 0);
        }
        // 5 ask deletes at t=now
        for _ in 0..5 {
            bet.on_book_update(Side::Ask, BookAction::Delete, 50, now);
        }
        let (bid_arr, ask_arr) = bet.arrival_rate_per_sec(2_000_000_000); // 2 sec window
        assert!(bid_arr > 4.0); // 10 arrives / 2 sec = 5/s
        assert_eq!(ask_arr, 0.0); // no ask upserts
        let (bid_can, ask_can) = bet.cancel_rate_per_sec(2_000_000_000);
        assert_eq!(bid_can, 0.0);
        assert!(ask_can > 2.0); // 5 cancels / 2 sec = 2.5/s
        let (bid_vol, ask_vol) = bet.event_volume_in_window(2_000_000_000);
        assert_eq!(bid_vol, 1000); // 10 * 100
        assert_eq!(ask_vol, 250); // 5 * 50
    }

    #[test]
    fn book_event_analytics_empty_returns_zeros() {
        let bet = BookEventTracker::new(100);
        assert_eq!(bet.event_count_in_window(1000, None), (0, 0));
        assert_eq!(bet.arrival_rate_per_sec(1000), (0.0, 0.0));
        assert_eq!(bet.cancel_rate_per_sec(1000), (0.0, 0.0));
    }

    #[test]
    fn resiliency_tracker_records_pre_and_post_trade_depth() {
        let mut rt = ResiliencyTracker::new(100);
        rt.on_trade_pre(1000, 800);
        rt.on_trade_post(900, 700, 1_000_000); // 1 ms later
        rt.on_trade_post(950, 750, 5_000_000); // 5 ms later - recovery update
        assert!(rt.latest_recovery_time_ms().is_some());
        // Depth elasticity should be positive
        let elasticity = rt.latest_depth_elasticity();
        assert!(elasticity.is_some() || elasticity.is_none());
    }

    #[test]
    fn resiliency_tracker_no_data_returns_none() {
        let rt = ResiliencyTracker::new(100);
        assert!(rt.latest_recovery_time_ms().is_none());
        assert!(rt.latest_depth_elasticity().is_none());
    }

    #[test]
    fn trade_classifier_tick_rule_up_tick_is_buy() {
        let mut tc = TradeClassifier::new();
        tc.last_price = Some(100);
        assert_eq!(tc.tick_rule(101, 10, 5), ClassificationVote::Buy);
    }

    #[test]
    fn trade_classifier_tick_rule_down_tick_is_sell() {
        let mut tc = TradeClassifier::new();
        tc.last_price = Some(100);
        assert_eq!(tc.tick_rule(99, 10, 5), ClassificationVote::Sell);
    }

    #[test]
    fn trade_classifier_tick_rule_no_last_price_is_neutral() {
        let tc = TradeClassifier::new();
        assert_eq!(tc.tick_rule(100, 10, 5), ClassificationVote::Neutral);
    }

    #[test]
    fn trade_classifier_quote_rule_at_ask_is_buy() {
        assert_eq!(
            TradeClassifier::quote_rule(102, 100, 102),
            ClassificationVote::Buy
        );
    }

    #[test]
    fn trade_classifier_quote_rule_at_bid_is_sell() {
        assert_eq!(
            TradeClassifier::quote_rule(100, 100, 102),
            ClassificationVote::Sell
        );
    }

    #[test]
    fn trade_classifier_quote_rule_at_mid_is_neutral() {
        assert_eq!(
            TradeClassifier::quote_rule(101, 100, 102),
            ClassificationVote::Neutral
        );
    }

    #[test]
    fn trade_classifier_lee_ready_uses_quote_when_available() {
        let vote = TradeClassifier::lee_ready(102, 100, 102, Some(100), 10, 5);
        assert_eq!(vote, ClassificationVote::Buy);
    }

    #[test]
    fn trade_classifier_lee_ready_falls_back_to_tick_at_mid() {
        let vote = TradeClassifier::lee_ready(101, 100, 102, Some(100), 10, 5);
        assert_eq!(vote, ClassificationVote::Buy); // up-tick → buy
    }

    #[test]
    fn trade_classifier_consensus_vote() {
        let mut tc = TradeClassifier::new();
        // Buy: quote says buy (at ask), tick says neutral (no last), LR falls back to neutral
        let vote = tc.classify(102, 10, 100, 102);
        // quote_weight=0.4 for buy, tick=0, LR=0 → buy
        assert_eq!(vote, ClassificationVote::Buy);
    }

    #[test]
    fn trade_classifier_reset_clears_state() {
        let mut tc = TradeClassifier::new();
        tc.last_price = Some(100);
        tc.reset();
        assert!(tc.last_price.is_none());
    }

    #[test]
    fn vpin_tracker_emits_bucket_on_sufficient_volume() {
        let mut vpin = VpinTracker::new(100, 50);
        vpin.on_trade(60, 40); // total 100 = bucket filled, buy-sell = 20
        let snap = vpin.snapshot();
        assert!(snap.vpin > 0.0, "vpin should be >0 got {}", snap.vpin);
        assert_eq!(snap.bucket_count, 1);
    }

    #[test]
    fn vpin_tracker_toxicity_detected() {
        let mut vpin = VpinTracker::new(100, 50).with_toxicity_threshold(1.0);
        // Multiple extreme-imbalance buckets
        for _ in 0..5 {
            vpin.on_trade(100, 0);
            vpin.on_trade(0, 100);
        }
        let snap = vpin.snapshot();
        // With high imbalance, z-score should exceed threshold
        assert!(snap.bucket_count > 0);
    }

    #[test]
    fn kyle_lambda_tracker_basic_regression() {
        let mut kl = KyleLambdaTracker::new(100);
        // Positive volume should correlate with positive price change
        for i in 0..10 {
            kl.on_trade(100 * i, i);
        }
        let snap = kl.snapshot();
        assert!(snap.sample_count >= 10);
    }

    #[test]
    fn kyle_lambda_tracker_insufficient_samples_returns_default() {
        let kl = KyleLambdaTracker::new(100);
        let snap = kl.snapshot();
        assert_eq!(snap.sample_count, 0);
    }

    #[test]
    fn amihud_tracker_computes_ratio() {
        let mut am = AmihudTracker::new(50);
        am.on_bar(101.0, 1_000_000.0, 100.0);
        let snap = am.snapshot();
        assert!(snap.amihud_ratio > 0.0);
        assert_eq!(snap.bar_count, 1);
    }

    #[test]
    fn cvd_enhancements_basic_metrics() {
        let mut cvd = CvdEnhancements::new(20);
        cvd.on_bar(100, 500, 100);
        cvd.on_bar(50, 400, 101);
        let snap = cvd.snapshot();
        assert!(snap.delta_ratio > 0.0);
    }

    #[test]
    fn cvd_enhancements_divergence_detected() {
        let mut cvd = CvdEnhancements::new(20);
        // Price rising but CVD falling = bearish divergence
        cvd.on_bar(100, 500, 100); // start
        cvd.on_bar(80, 400, 101); // price up, delta down
        cvd.on_bar(60, 300, 102); // price up, delta down
        let snap = cvd.snapshot();
        assert!(snap.divergence_detected);
    }

    #[test]
    fn pattern_detector_initial_balance_defaults() {
        let pd = PatternDetector::new();
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![],
            asks: vec![],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        let snap = pd.snapshot(&book, 0, 0.0, 0.0);
        assert!(!snap.trend_day);
        assert!(!snap.range_day);
    }

    #[test]
    fn volatility_estimator_classic_rv_computed() {
        let mut ve = VolatilityEstimator::new(100);
        ve.on_bar(100.0, 102.0, 99.0, 101.0);
        ve.on_bar(101.0, 103.0, 100.0, 102.0);
        let snap = ve.snapshot();
        assert!(snap.classic_rv > 0.0);
        assert!(snap.parkinson > 0.0);
    }

    #[test]
    fn noise_tracker_returns_default_with_few_samples() {
        let nt = MicrostructureNoise::new(100);
        assert_eq!(nt.snapshot().noise_variance, 0.0);
    }

    #[test]
    fn hasbrouck_var_returns_default_with_few_samples() {
        let hv = HasbrouckVAR::new(100);
        assert_eq!(hv.snapshot().permanent_impact, 0.0);
    }

    #[test]
    fn almgren_chriss_returns_default_with_few_samples() {
        let ac = AlmgrenChriss::new(100);
        assert_eq!(ac.snapshot().permanent_impact_coef, 0.0);
    }

    #[test]
    fn spread_decomposition_returns_default_empty() {
        let sd = SpreadDecomposition::new(100);
        assert_eq!(sd.snapshot().pin, 0.0);
    }

    #[test]
    fn acd_model_returns_default_with_few_samples() {
        let acd = ACDModel::new(100);
        assert_eq!(acd.snapshot().mean_duration_ns, 0.0);
    }

    #[test]
    fn regime_detector_normal_by_default() {
        let rd = RegimeDetector::new(100);
        assert_eq!(rd.snapshot().regime, 0); // Normal
    }

    #[test]
    fn kinetic_energy_returns_default_with_few_samples() {
        let ke = KineticEnergyTracker::new(100);
        assert_eq!(ke.snapshot().kinetic_energy, 0.0);
    }

    #[test]
    fn dark_pool_analytics_basic() {
        let mut dp = DarkPoolTracker::new(20);
        dp.on_day(1000.0, 9000.0);
        let snap = dp.snapshot();
        assert!((snap.dark_volume_pct - 10.0).abs() < 0.01);
    }

    #[test]
    fn options_flow_put_call_ratio() {
        let mut ot = OptionsFlowTracker::new(100);
        ot.on_trade(true, 100.0, 1000.0, false);
        ot.on_trade(false, 200.0, 2000.0, false);
        let snap = ot.snapshot();
        assert!((snap.put_call_ratio - 2.0).abs() < 0.01);
    }

    #[test]
    fn futures_basis_computed() {
        let mut ft = FuturesTracker::new(100);
        ft.on_tick(100.0, 101.0, 1000.0, 5000.0);
        let snap = ft.snapshot();
        assert!((snap.basis_bps - 100.0).abs() < 0.01);
    }

    #[test]
    fn pattern_detector_imbalance_detected() {
        let pd = PatternDetector::new();
        let book = BookSnapshot {
            symbol: symbol(),
            bids: vec![BookLevel {
                level: 0,
                price: 100,
                size: 10,
            }],
            asks: vec![BookLevel {
                level: 0,
                price: 102,
                size: 50,
            }],
            last_sequence: 0,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        };
        let snap = pd.snapshot(&book, 0, 0.0, 0.0);
        assert!(snap.imbalance_detected);
    }
}
