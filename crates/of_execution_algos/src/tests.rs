#[cfg(test)]
mod tests {
    use super::*;
    use of_execution_core::{ExecutionEvent, ExecutionId, RiskRejectReason, VenueOrderId};

    fn parent() -> ParentOrder {
        ParentOrder::new(
            ParentOrderId::new("parent-1").expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("sim").expect("route"),
            StrategyId::new("twap").expect("strategy"),
            ExecutionSymbol::new("SIM", "ESZ6").expect("symbol"),
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::Day,
            OrderQty(100),
            OrderPrice(500_000),
            OrderPrice(0),
            1_000,
            11_000,
            OrderQty(10),
            OrderQty(25),
            0,
        )
        .expect("parent")
    }

    fn sell_parent(
        id: &str,
        total_qty: OrderQty,
        min_clip: OrderQty,
        max_clip: OrderQty,
    ) -> ParentOrder {
        ParentOrder::new(
            ParentOrderId::new(id).expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("sim").expect("route"),
            StrategyId::new("spread").expect("strategy"),
            ExecutionSymbol::new("SIM", "NQZ6").expect("symbol"),
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::Day,
            total_qty,
            OrderPrice(1_000_000),
            OrderPrice(0),
            1_000,
            11_000,
            min_clip,
            max_clip,
            0,
        )
        .expect("sell parent")
    }

    #[test]
    fn twap_plans_due_slices_without_over_release() {
        let parent = parent();
        let planner = TwapSlicePlanner::new(1_000);
        let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let first = planner
            .plan_due_slice(
                &parent,
                progress,
                1_000,
                ChildOrderId::new("child-1").expect("child"),
                ClientOrderId::new("cl-1").expect("client"),
                1_000,
            )
            .expect("plan")
            .expect("due");
        assert_eq!(first.request().quantity, OrderQty(10));
        progress.on_child_released(&first).expect("release");

        let second_same_bucket = planner
            .plan_due_slice(
                &parent,
                progress,
                1_500,
                ChildOrderId::new("child-2").expect("child"),
                ClientOrderId::new("cl-2").expect("client"),
                1_500,
            )
            .expect("plan");
        assert!(second_same_bucket.is_none());

        let later = planner
            .plan_due_slice(
                &parent,
                progress,
                3_000,
                ChildOrderId::new("child-3").expect("child"),
                ClientOrderId::new("cl-3").expect("client"),
                3_000,
            )
            .expect("plan")
            .expect("due");
        assert_eq!(later.request().quantity, OrderQty(20));
    }

    #[test]
    fn twap_respects_max_clip_and_final_leaves() {
        let parent = parent();
        let planner = TwapSlicePlanner::new(1_000);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let late = planner
            .plan_due_slice(
                &parent,
                progress,
                10_500,
                ChildOrderId::new("child-late").expect("child"),
                ClientOrderId::new("cl-late").expect("client"),
                10_500,
            )
            .expect("plan")
            .expect("due");
        assert_eq!(late.request().quantity, parent.max_clip());
    }

    #[test]
    fn decision_buffer_is_bounded() {
        let mut decision = AlgoDecision::<1>::new(7);
        let plan = ChildOrderPlan::new(
            ChildOrderId::new("child-1").expect("child"),
            parent().id(),
            parent().build_order_request(
                ClientOrderId::new("cl-1").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("plan");

        decision
            .push(AlgoAction::SubmitChild(plan))
            .expect("first action");
        assert_eq!(decision.len(), 1);
        assert_eq!(
            decision.push(AlgoAction::CompleteParent {
                parent_id: parent().id()
            }),
            Err(AlgoError::DecisionFull { capacity: 1 })
        );
        assert_eq!(decision.actions().count(), 1);
    }

    #[test]
    fn risk_policy_allows_child_inside_limits() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("risk-ok").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("risk-ok-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let limits = AlgoRiskLimits::new(
            OrderQty(200),
            OrderQty(25),
            10_000_000,
            1_500,
            100,
            OrderQty(50),
            2,
            10,
        )
        .expect("limits");
        let context = AlgoRiskContext::new(OrderPrice(500_000))
            .expect("context")
            .with_observed_market_volume(OrderQty(1_000));

        let report = AlgoRiskPolicy::new(limits)
            .evaluate_child::<DEFAULT_ALGO_RISK_VIOLATION_CAPACITY>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                &child,
                context,
            )
            .expect("report");

        assert!(report.is_allowed());
        assert!(report.is_empty());
    }

    #[test]
    fn risk_policy_blocks_limit_breaches() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("risk-block").expect("child"),
            parent.id(),
            parent.build_order_request_at_price(
                ClientOrderId::new("risk-block-cl").expect("client"),
                OrderQty(25),
                OrderPrice(510_000),
                1,
            ),
            1,
        )
        .expect("child");
        let limits = AlgoRiskLimits::new(
            OrderQty(100),
            OrderQty(10),
            5_000_000,
            1_000,
            100,
            OrderQty(50),
            2,
            10,
        )
        .expect("limits");
        let context = AlgoRiskContext::new(OrderPrice(500_000))
            .expect("context")
            .with_observed_market_volume(OrderQty(100));

        let report = AlgoRiskPolicy::new(limits)
            .evaluate_child::<DEFAULT_ALGO_RISK_VIOLATION_CAPACITY>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                &child,
                context,
            )
            .expect("report");

        assert_eq!(report.outcome(), AlgoRiskOutcome::Block);
        assert!(report
            .violations()
            .any(|violation| violation.kind() == AlgoRiskViolationKind::ChildQuantityExceeded));
        assert!(report
            .violations()
            .any(|violation| violation.kind() == AlgoRiskViolationKind::PriceCollarExceeded));
        assert!(report
            .violations()
            .any(|violation| violation.kind() == AlgoRiskViolationKind::ParticipationExceeded));
    }

    #[test]
    fn risk_policy_kill_switch_halts_submission() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("risk-kill").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("risk-kill-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let context = AlgoRiskContext::new(OrderPrice(500_000))
            .expect("context")
            .with_kill_switch_active(true);

        let report = AlgoRiskPolicy::default()
            .evaluate_child::<DEFAULT_ALGO_RISK_VIOLATION_CAPACITY>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                &child,
                context,
            )
            .expect("report");

        assert_eq!(report.outcome(), AlgoRiskOutcome::KillSwitch);
        assert_eq!(
            report.first_violation().expect("violation").kind(),
            AlgoRiskViolationKind::KillSwitchActive
        );
    }

    #[test]
    fn risk_policy_checks_decision_level_limits() {
        let parent = parent();
        let mut decision = AlgoDecision::<2>::new(1);
        for index in 1..=2 {
            let suffix = index.to_string();
            let child = ChildOrderPlan::new(
                ChildOrderId::new(&format!("risk-d-{suffix}")).expect("child"),
                parent.id(),
                parent.build_order_request(
                    ClientOrderId::new(&format!("risk-d-cl-{suffix}")).expect("client"),
                    OrderQty(10),
                    index,
                ),
                index,
            )
            .expect("child");
            decision.push(AlgoAction::SubmitChild(child)).expect("push");
        }
        let limits = AlgoRiskLimits::new(OrderQty(0), OrderQty(0), 0, 0, 0, OrderQty(15), 1, 1)
            .expect("limits");
        let context = AlgoRiskContext::new(OrderPrice(500_000))
            .expect("context")
            .with_child_orders_in_window(0);

        let report = AlgoRiskPolicy::new(limits)
            .evaluate_decision::<DEFAULT_ALGO_RISK_VIOLATION_CAPACITY, 2>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                &decision,
                context,
            )
            .expect("report");

        assert_eq!(report.outcome(), AlgoRiskOutcome::Block);
        assert!(report.violations().any(
            |violation| violation.kind() == AlgoRiskViolationKind::ChildrenPerDecisionExceeded
        ));
        assert!(report
            .violations()
            .any(|violation| violation.kind() == AlgoRiskViolationKind::OpenQuantityExceeded));
    }

    #[test]
    fn algo_checkpoint_records_replay_cursors() {
        let parent = parent();
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let checkpoint = AlgoCheckpoint::new(parent, progress, 7, 42).expect("checkpoint");

        assert_eq!(checkpoint.schema_version(), ALGO_CHECKPOINT_SCHEMA_VERSION);
        assert_eq!(checkpoint.parent(), parent);
        assert_eq!(checkpoint.progress(), progress);
        assert_eq!(checkpoint.next_decision_seq(), 7);
        assert_eq!(checkpoint.last_input_sequence(), 42);
        assert_eq!(
            AlgoCheckpoint::new(parent, progress, 0, 42),
            Err(AlgoError::InvalidRecoveryState)
        );
    }

    #[test]
    fn recovery_plan_pauses_by_default() {
        let parent = parent();
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let checkpoint = AlgoCheckpoint::new(parent, progress, 7, 42).expect("checkpoint");
        let plan = AlgoRecoveryPlan::new(checkpoint, AlgoRecoveryPolicy::default()).expect("plan");

        assert_eq!(plan.action(), AlgoRecoveryAction::Pause);
        assert_eq!(plan.replay_from_sequence(), 43);
        assert_eq!(plan.next_decision_seq(), 7);
        assert!(plan.reconciliation_required());
    }

    #[test]
    fn recovery_plan_can_resume_when_policy_allows() {
        let parent = parent();
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let checkpoint = AlgoCheckpoint::new(parent, progress, 3, 9).expect("checkpoint");
        let policy = AlgoRecoveryPolicy::default()
            .with_pause_on_recovery(false)
            .with_require_reconciliation(false);
        let plan = AlgoRecoveryPlan::new(checkpoint, policy).expect("plan");

        assert_eq!(plan.action(), AlgoRecoveryAction::Resume);
        assert!(!plan.reconciliation_required());
    }

    #[test]
    fn recovery_plan_completes_finished_progress() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("rec-child").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("rec-cl").expect("client"),
                parent.total_qty(),
                1,
            ),
            1,
        )
        .expect("child");
        let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());
        progress.on_child_released(&child).expect("release");
        progress.on_execution_event(&ExecutionEvent {
            exec_type: of_execution_core::ExecutionType::Trade,
            order_status: OrderStatus::Filled,
            client_order_id: child.request().client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: VenueOrderId::new("rec-venue").expect("venue"),
            execution_id: ExecutionId::new("rec-exec").expect("exec"),
            account_id: parent.account_id(),
            route_id: parent.route_id(),
            symbol: parent.symbol(),
            last_qty: parent.total_qty(),
            last_price: parent.limit_price(),
            cumulative_qty: parent.total_qty(),
            leaves_qty: OrderQty(0),
            average_price: parent.limit_price(),
            ts_exchange_ns: 2,
            ts_recv_ns: 3,
            reason: RiskRejectReason::None,
            text: of_execution_core::ExecutionText::empty(),
        });
        let checkpoint = AlgoCheckpoint::new(parent, progress, 10, 99).expect("checkpoint");
        let plan = AlgoRecoveryPlan::new(checkpoint, AlgoRecoveryPolicy::default()).expect("plan");

        assert_eq!(plan.action(), AlgoRecoveryAction::CompleteParent);
    }

    #[test]
    fn recovery_plan_escalates_terminal_parent() {
        let parent = parent().with_status(ParentOrderStatus::Failed);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let checkpoint = AlgoCheckpoint::new(parent, progress, 10, 99).expect("checkpoint");
        let policy = AlgoRecoveryPolicy::default()
            .with_pause_on_recovery(false)
            .with_require_reconciliation(false);
        let plan = AlgoRecoveryPlan::new(checkpoint, policy).expect("plan");

        assert_eq!(plan.action(), AlgoRecoveryAction::EscalateRisk);
    }

    #[test]
    fn simulator_fills_child_and_updates_progress() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("sim-fill").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("sim-fill-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let simulator = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(10), OrderPrice(500_025), false, false, 25)
                .expect("market"),
        );
        let step = simulator.simulate_child(&child, 1).expect("step");
        let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());
        progress.on_child_released(&child).expect("release");
        progress.on_execution_event(&step.event());

        assert_eq!(step.outcome(), AlgoSimOutcome::Filled);
        assert_eq!(step.filled_qty(), OrderQty(10));
        assert_eq!(step.event().order_status, OrderStatus::Filled);
        assert_eq!(step.event().ts_recv_ns, 26);
        assert_eq!(progress.completed_qty(), OrderQty(10));
        assert_eq!(progress.open_qty(), OrderQty(0));
    }

    #[test]
    fn simulator_partially_fills_child() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("sim-partial").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("sim-partial-cl").expect("client"),
                OrderQty(25),
                1,
            ),
            1,
        )
        .expect("child");
        let simulator = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(5), OrderPrice(500_000), false, false, 0).expect("market"),
        );
        let step = simulator.simulate_child(&child, 7).expect("step");

        assert_eq!(step.outcome(), AlgoSimOutcome::PartiallyFilled);
        assert_eq!(step.filled_qty(), OrderQty(5));
        assert_eq!(step.leaves_qty(), OrderQty(20));
        assert_eq!(step.event().order_status, OrderStatus::PartiallyFilled);
    }

    #[test]
    fn simulator_rejects_child() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("sim-reject").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("sim-reject-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let simulator = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(0), OrderPrice(0), true, false, 0).expect("market"),
        );
        let step = simulator.simulate_child(&child, 2).expect("step");

        assert_eq!(step.outcome(), AlgoSimOutcome::Rejected);
        assert_eq!(step.event().exec_type, ExecutionType::Reject);
        assert_eq!(step.event().order_status, OrderStatus::Rejected);
        assert_eq!(step.event().leaves_qty, OrderQty(10));
    }

    #[test]
    fn simulator_reports_decision_totals() {
        let parent = parent();
        let mut decision = AlgoDecision::<2>::new(1);
        for index in 1..=2 {
            let child = ChildOrderPlan::new(
                ChildOrderId::new(&format!("sim-d-{index}")).expect("child"),
                parent.id(),
                parent.build_order_request(
                    ClientOrderId::new(&format!("sim-d-cl-{index}")).expect("client"),
                    OrderQty(10),
                    index,
                ),
                index,
            )
            .expect("child");
            decision.push(AlgoAction::SubmitChild(child)).expect("push");
        }
        let simulator = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(10), OrderPrice(500_000), false, true, 0).expect("market"),
        );
        let report = simulator
            .simulate_decision::<1, 2>(&decision, 10)
            .expect("report");

        assert!(report.truncated());
        assert_eq!(report.len(), 1);
        assert_eq!(report.total_filled_qty(), OrderQty(20));
        assert_eq!(report.cancelled_children(), 0);
    }

    #[test]
    fn metrics_accumulate_fill_quality() {
        let parent = parent();
        let child = ChildOrderPlan::new(
            ChildOrderId::new("met-fill").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("met-fill-cl").expect("client"),
                OrderQty(10),
                1_000,
            ),
            1_000,
        )
        .expect("child");
        let mut metrics = AlgoMetricsAccumulator::new(
            &parent,
            AlgoTcaBenchmark::new(
                OrderPrice(500_000),
                OrderPrice(502_500),
                OrderPrice(501_000),
            )
            .expect("benchmark"),
        )
        .expect("metrics");
        metrics.on_child_submitted(&child);
        let step = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(10), OrderPrice(505_000), false, false, 25)
                .expect("market"),
        )
        .simulate_child(&child, 1)
        .expect("step");
        metrics.on_execution_event(&step.event());
        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.submitted_children(), 1);
        assert_eq!(snapshot.filled_children(), 1);
        assert_eq!(snapshot.completed_qty(), OrderQty(10));
        assert_eq!(snapshot.completion_bps(), 1_000);
        assert_eq!(snapshot.average_price(), OrderPrice(505_000));
        assert_eq!(snapshot.arrival_slippage_bps(), 100);
        assert_eq!(snapshot.average_latency_ns(), 25);
    }

    #[test]
    fn metrics_record_rejects_and_cancels() {
        let parent = parent();
        let rejected_child = ChildOrderPlan::new(
            ChildOrderId::new("met-reject").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("met-reject-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let cancelled_child = ChildOrderPlan::new(
            ChildOrderId::new("met-cancel").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("met-cancel-cl").expect("client"),
                OrderQty(10),
                2,
            ),
            2,
        )
        .expect("child");
        let mut metrics = AlgoMetricsAccumulator::new(
            &parent,
            AlgoTcaBenchmark::new(OrderPrice(500_000), OrderPrice(0), OrderPrice(0))
                .expect("benchmark"),
        )
        .expect("metrics");
        metrics.on_child_submitted(&rejected_child);
        metrics.on_child_submitted(&cancelled_child);
        let reject = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(0), OrderPrice(0), true, false, 0).expect("market"),
        )
        .simulate_child(&rejected_child, 1)
        .expect("reject");
        let cancel = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(0), OrderPrice(500_000), false, true, 0).expect("market"),
        )
        .simulate_child(&cancelled_child, 2)
        .expect("cancel");
        metrics.on_execution_event(&reject.event());
        metrics.on_execution_event(&cancel.event());
        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.submitted_children(), 2);
        assert_eq!(snapshot.rejected_children(), 1);
        assert_eq!(snapshot.cancelled_children(), 1);
        assert_eq!(snapshot.completed_qty(), OrderQty(0));
    }

    #[test]
    fn metrics_preserve_favorable_sell_slippage() {
        let parent = sell_parent("met-sell", OrderQty(100), OrderQty(10), OrderQty(25));
        let child = ChildOrderPlan::new(
            ChildOrderId::new("met-sell-child").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("met-sell-cl").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("child");
        let mut metrics = AlgoMetricsAccumulator::new(
            &parent,
            AlgoTcaBenchmark::new(OrderPrice(500_000), OrderPrice(0), OrderPrice(0))
                .expect("benchmark"),
        )
        .expect("metrics");
        metrics.on_child_submitted(&child);
        let step = AlgoSimulator::new(
            AlgoSimMarket::new(OrderQty(10), OrderPrice(505_000), false, false, 0).expect("market"),
        )
        .simulate_child(&child, 1)
        .expect("step");
        metrics.on_execution_event(&step.event());

        assert_eq!(metrics.snapshot().arrival_slippage_bps(), -100);
    }

    #[test]
    fn algo_config_builds_parent_and_policies() {
        let parent = parent();
        let config_parent = AlgoParentConfig::from_parent(parent);
        let risk_limits = AlgoRiskLimits::new(
            OrderQty(100),
            OrderQty(25),
            10_000_000,
            1_500,
            100,
            OrderQty(50),
            2,
            10,
        )
        .expect("risk");
        let recovery = AlgoRecoveryPolicy::default()
            .with_pause_on_recovery(false)
            .with_require_reconciliation(false);
        let config = AlgoConfig::new(AlgoKind::Twap, config_parent)
            .with_risk_limits(risk_limits)
            .with_recovery_policy(recovery);

        assert_eq!(config.kind(), AlgoKind::Twap);
        assert_eq!(config.to_parent_order().expect("parent"), parent);
        assert_eq!(config.to_risk_policy().limits(), risk_limits);
        assert_eq!(config.recovery_policy(), recovery);
    }

    #[test]
    fn algo_parent_config_rejects_invalid_schedule() {
        assert_eq!(
            AlgoParentConfig::new(
                ParentOrderId::new("cfg-bad").expect("id"),
                AccountId::new("acct").expect("account"),
                RouteId::new("sim").expect("route"),
                StrategyId::new("cfg").expect("strategy"),
                ExecutionSymbol::new("SIM", "ESZ6").expect("symbol"),
                OrderSide::Buy,
                OrderType::Limit,
                TimeInForce::Day,
                OrderQty(100),
                OrderPrice(500_000),
                OrderPrice(0),
                1_000,
                1_000,
                OrderQty(10),
                OrderQty(25),
                0,
            ),
            Err(AlgoError::InvalidTimeWindow)
        );
    }

    #[test]
    fn algo_config_supports_custom_kind() {
        let config = AlgoConfig::new(AlgoKind::Custom, AlgoParentConfig::from_parent(parent()));

        assert_eq!(config.kind(), AlgoKind::Custom);
        assert_eq!(
            config.recovery_policy(),
            AlgoRecoveryPolicy::new(true, true, true)
        );
        assert_eq!(config.risk_limits(), AlgoRiskLimits::unbounded());
    }

    #[test]
    fn progress_folds_fill_events() {
        let parent = parent();
        let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let plan = ChildOrderPlan::new(
            ChildOrderId::new("child-1").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("cl-1").expect("client"),
                OrderQty(10),
                1,
            ),
            1,
        )
        .expect("plan");
        progress.on_child_released(&plan).expect("release");

        progress.on_execution_event(&ExecutionEvent {
            exec_type: of_execution_core::ExecutionType::Trade,
            order_status: OrderStatus::Filled,
            client_order_id: plan.request().client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: VenueOrderId::new("venue-1").expect("venue"),
            execution_id: ExecutionId::new("exec-1").expect("exec"),
            account_id: parent.account_id(),
            route_id: parent.route_id(),
            symbol: parent.symbol(),
            last_qty: OrderQty(10),
            last_price: OrderPrice(500_000),
            cumulative_qty: OrderQty(10),
            leaves_qty: OrderQty(0),
            average_price: OrderPrice(500_000),
            ts_exchange_ns: 2,
            ts_recv_ns: 3,
            reason: RiskRejectReason::None,
            text: of_execution_core::ExecutionText::empty(),
        });

        assert_eq!(progress.completed_qty(), OrderQty(10));
        assert_eq!(progress.open_qty(), OrderQty(0));
        assert_eq!(progress.terminal_children(), 1);
    }

    #[test]
    fn invalid_parent_schedule_is_rejected() {
        assert_eq!(
            ParentOrder::new(
                ParentOrderId::new("parent-1").expect("id"),
                AccountId::new("acct").expect("account"),
                RouteId::new("sim").expect("route"),
                StrategyId::new("twap").expect("strategy"),
                ExecutionSymbol::new("SIM", "ESZ6").expect("symbol"),
                OrderSide::Buy,
                OrderType::Limit,
                TimeInForce::Day,
                OrderQty(100),
                OrderPrice(500_000),
                OrderPrice(0),
                10,
                10,
                OrderQty(10),
                OrderQty(25),
                0,
            ),
            Err(AlgoError::InvalidTimeWindow)
        );
    }

    #[test]
    fn replay_twap_is_deterministic() {
        let parent = parent();
        let planner = TwapSlicePlanner::new(1_000);
        let inputs = [
            AlgoReplayInput::new(
                1,
                AlgoReplayEvent::Timer {
                    timestamp_ns: 1_000,
                },
            ),
            AlgoReplayInput::new(
                2,
                AlgoReplayEvent::Timer {
                    timestamp_ns: 2_000,
                },
            ),
            AlgoReplayInput::new(
                3,
                AlgoReplayEvent::Timer {
                    timestamp_ns: 3_000,
                },
            ),
        ];
        let mut first_steps = Vec::new();
        let mut second_steps = Vec::new();
        let first = replay_twap_into::<DEFAULT_ALGO_DECISION_CAPACITY>(
            parent,
            planner,
            &inputs,
            AlgoReplayIdScheme::default(),
            &mut first_steps,
        )
        .expect("first replay");
        let second = replay_twap_into::<DEFAULT_ALGO_DECISION_CAPACITY>(
            parent,
            planner,
            &inputs,
            AlgoReplayIdScheme::default(),
            &mut second_steps,
        )
        .expect("second replay");

        assert_eq!(first, second);
        assert_eq!(first_steps, second_steps);
        assert_eq!(first.input_events(), 3);
        assert_eq!(first.submitted_children(), 3);
        assert_eq!(first.final_progress().released_qty(), OrderQty(30));
    }

    #[test]
    fn replay_folds_execution_events() {
        let parent = parent();
        let planner = TwapSlicePlanner::new(1_000);
        let fill = ExecutionEvent {
            exec_type: of_execution_core::ExecutionType::Trade,
            order_status: OrderStatus::Filled,
            client_order_id: ClientOrderId::new("cl-1").expect("client"),
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: VenueOrderId::new("venue-1").expect("venue"),
            execution_id: ExecutionId::new("exec-1").expect("exec"),
            account_id: parent.account_id(),
            route_id: parent.route_id(),
            symbol: parent.symbol(),
            last_qty: OrderQty(10),
            last_price: OrderPrice(500_000),
            cumulative_qty: OrderQty(10),
            leaves_qty: OrderQty(0),
            average_price: OrderPrice(500_000),
            ts_exchange_ns: 2,
            ts_recv_ns: 3,
            reason: RiskRejectReason::None,
            text: of_execution_core::ExecutionText::empty(),
        };
        let inputs = [
            AlgoReplayInput::new(
                1,
                AlgoReplayEvent::Timer {
                    timestamp_ns: 1_000,
                },
            ),
            AlgoReplayInput::new(2, AlgoReplayEvent::Execution(fill)),
        ];
        let mut steps = Vec::new();
        let summary = replay_twap_into::<DEFAULT_ALGO_DECISION_CAPACITY>(
            parent,
            planner,
            &inputs,
            AlgoReplayIdScheme::default(),
            &mut steps,
        )
        .expect("replay");

        assert_eq!(summary.final_progress().released_qty(), OrderQty(10));
        assert_eq!(summary.final_progress().completed_qty(), OrderQty(10));
        assert_eq!(summary.final_progress().open_qty(), OrderQty(0));
        assert_eq!(steps[0].decision().len(), 1);
        assert!(steps[1].decision().is_empty());
    }

    #[test]
    fn replay_reports_decision_capacity_errors() {
        let parent = parent();
        let inputs = [AlgoReplayInput::new(
            1,
            AlgoReplayEvent::Timer {
                timestamp_ns: 1_000,
            },
        )];
        let mut steps = Vec::new();
        assert_eq!(
            replay_twap_into::<0>(
                parent,
                TwapSlicePlanner::new(1_000),
                &inputs,
                AlgoReplayIdScheme::default(),
                &mut steps,
            ),
            Err(AlgoError::DecisionFull { capacity: 0 })
        );
        assert!(steps.is_empty());
    }

    #[test]
    fn pov_plans_from_observed_volume() {
        let parent = parent();
        let planner = PovSlicePlanner::new(1_000, 1_500);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_volume_slice(
                &parent,
                progress,
                OrderQty(1_000),
                2_000,
                ChildOrderId::new("child-pov").expect("child"),
                ClientOrderId::new("cl-pov").expect("client"),
                2_000,
            )
            .expect("plan")
            .expect("due");

        assert_eq!(child.request().quantity, parent.max_clip());
    }

    #[test]
    fn pov_waits_when_due_quantity_is_below_min_clip() {
        let parent = parent();
        let planner = PovSlicePlanner::new(1_000, 1_500);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_volume_slice(
                &parent,
                progress,
                OrderQty(50),
                2_000,
                ChildOrderId::new("child-small").expect("child"),
                ClientOrderId::new("cl-small").expect("client"),
                2_000,
            )
            .expect("plan");

        assert!(child.is_none());
    }

    #[test]
    fn pov_rejects_parent_cap_below_target() {
        let capped_parent = parent().with_status(ParentOrderStatus::Active);
        let capped_parent = ParentOrder::new(
            capped_parent.id(),
            capped_parent.account_id(),
            capped_parent.route_id(),
            capped_parent.strategy_id(),
            capped_parent.symbol(),
            capped_parent.side(),
            capped_parent.order_type(),
            capped_parent.time_in_force(),
            capped_parent.total_qty(),
            capped_parent.limit_price(),
            capped_parent.stop_price(),
            capped_parent.start_ns(),
            capped_parent.end_ns(),
            capped_parent.min_clip(),
            capped_parent.max_clip(),
            500,
        )
        .expect("capped parent");
        let planner = PovSlicePlanner::new(1_000, 1_500);
        let progress = AlgoProgress::new(capped_parent.id(), capped_parent.total_qty());

        assert_eq!(
            planner.plan_volume_slice(
                &capped_parent,
                progress,
                OrderQty(1_000),
                2_000,
                ChildOrderId::new("child-cap").expect("child"),
                ClientOrderId::new("cl-cap").expect("client"),
                2_000,
            ),
            Err(AlgoError::InvalidParticipationRate)
        );
    }

    #[test]
    fn vwap_plans_from_cumulative_curve() {
        let parent = parent();
        let curve = VwapVolumeCurve::new(1_000, 1_000, &[10, 30, 60, 100]).expect("curve");
        let planner = VwapSlicePlanner::new(curve);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_curve_slice(
                &parent,
                progress,
                2_000,
                ChildOrderId::new("child-vwap").expect("child"),
                ClientOrderId::new("cl-vwap").expect("client"),
                2_000,
            )
            .expect("plan")
            .expect("due");

        assert_eq!(child.request().quantity, OrderQty(25));
    }

    #[test]
    fn vwap_waits_for_min_clip() {
        let parent = parent();
        let curve = VwapVolumeCurve::new(1_000, 1_000, &[1, 2, 100]).expect("curve");
        let planner = VwapSlicePlanner::new(curve);
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_curve_slice(
                &parent,
                progress,
                1_000,
                ChildOrderId::new("child-small").expect("child"),
                ClientOrderId::new("cl-small").expect("client"),
                1_000,
            )
            .expect("plan");

        assert!(child.is_none());
    }

    #[test]
    fn vwap_rejects_invalid_curve() {
        assert_eq!(
            VwapVolumeCurve::new(1, 1, &[10, 9]),
            Err(AlgoError::InvalidVolumeProfile)
        );
        assert_eq!(
            VwapVolumeCurve::new(1, 0, &[10]),
            Err(AlgoError::InvalidVolumeProfile)
        );
    }

    #[test]
    fn iceberg_replenishes_when_open_quantity_is_at_threshold() {
        let parent = parent();
        let planner = IcebergSlicePlanner::new(OrderQty(20), OrderQty(0));
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_replenishment(
                &parent,
                progress,
                1_000,
                ChildOrderId::new("child-ice").expect("child"),
                ClientOrderId::new("cl-ice").expect("client"),
                1_000,
            )
            .expect("plan")
            .expect("due");

        assert_eq!(child.request().quantity, OrderQty(20));
    }

    #[test]
    fn iceberg_waits_while_display_quantity_is_working() {
        let parent = parent();
        let planner = IcebergSlicePlanner::new(OrderQty(20), OrderQty(0));
        let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let working = ChildOrderPlan::new(
            ChildOrderId::new("child-ice").expect("child"),
            parent.id(),
            parent.build_order_request(
                ClientOrderId::new("cl-ice").expect("client"),
                OrderQty(20),
                1,
            ),
            1,
        )
        .expect("child");
        progress.on_child_released(&working).expect("release");

        let child = planner
            .plan_replenishment(
                &parent,
                progress,
                2_000,
                ChildOrderId::new("child-next").expect("child"),
                ClientOrderId::new("cl-next").expect("client"),
                2_000,
            )
            .expect("plan");

        assert!(child.is_none());
    }

    #[test]
    fn iceberg_rejects_invalid_display_settings() {
        assert_eq!(
            IcebergSlicePlanner::try_new(OrderQty(0), OrderQty(0)),
            Err(AlgoError::InvalidDisplayQuantity)
        );
        assert_eq!(
            IcebergSlicePlanner::try_new(OrderQty(10), OrderQty(11)),
            Err(AlgoError::InvalidDisplayQuantity)
        );
    }

    #[test]
    fn passive_queue_joins_when_fill_probability_is_good() {
        let parent = parent();
        let planner = PassiveQueuePlanner::new(
            PassiveQueueConfig::new(PassivePegMode::SameSide, OrderPrice(25)).expect("config"),
        );
        let context = PassiveQueueContext::new(
            OrderPrice(499_975),
            OrderPrice(500_025),
            OrderQty(25),
            OrderQty(100),
            10,
        )
        .expect("context");
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let decision = planner
            .plan_passive_slice(
                &parent,
                progress,
                2_000,
                context,
                ChildOrderId::new("child-pq").expect("child"),
                ClientOrderId::new("cl-pq").expect("client"),
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.action(), PassiveQueueAction::JoinQueue);
        let child = decision.child().expect("child");
        assert_eq!(child.request().limit_price, context.best_bid());
        assert_eq!(child.request().quantity, parent.max_clip());
        assert!(decision.estimate().fill_probability_bps() >= 2_500);
    }

    #[test]
    fn passive_queue_improves_when_queue_is_unlikely_to_fill() {
        let parent = parent();
        let config = PassiveQueueConfig::new(PassivePegMode::SameSide, OrderPrice(25))
            .expect("config")
            .with_thresholds(2_500, 250, 1_500)
            .expect("thresholds")
            .with_max_improvement_ticks(1);
        let planner = PassiveQueuePlanner::new(config);
        let context = PassiveQueueContext::new(
            OrderPrice(499_950),
            OrderPrice(500_050),
            OrderQty(10_000),
            OrderQty(10),
            10,
        )
        .expect("context");
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let decision = planner
            .plan_passive_slice(
                &parent,
                progress,
                2_000,
                context,
                ChildOrderId::new("child-imp").expect("child"),
                ClientOrderId::new("cl-imp").expect("client"),
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.action(), PassiveQueueAction::ImprovePrice);
        let child = decision.child().expect("child");
        assert_eq!(child.request().limit_price, OrderPrice(499_975));
        assert!(child.request().limit_price.0 < context.best_ask().0);
    }

    #[test]
    fn passive_queue_waits_when_adverse_selection_is_high() {
        let parent = parent();
        let planner = PassiveQueuePlanner::new(
            PassiveQueueConfig::new(PassivePegMode::SameSide, OrderPrice(25)).expect("config"),
        );
        let context = PassiveQueueContext::new(
            OrderPrice(499_975),
            OrderPrice(500_025),
            OrderQty(0),
            OrderQty(100),
            1_000,
        )
        .expect("context");

        let decision = planner
            .plan_passive_slice(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                context,
                ChildOrderId::new("child-wait").expect("child"),
                ClientOrderId::new("cl-wait").expect("client"),
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.action(), PassiveQueueAction::Wait);
        assert!(decision.child().is_none());
    }

    #[test]
    fn passive_queue_can_cross_when_allowed_and_late() {
        let parent = parent();
        let config = PassiveQueueConfig::new(PassivePegMode::SameSide, OrderPrice(25))
            .expect("config")
            .with_crossing(true, 8_000)
            .expect("crossing");
        let planner = PassiveQueuePlanner::new(config);
        let context = PassiveQueueContext::new(
            OrderPrice(499_975),
            OrderPrice(500_025),
            OrderQty(10_000),
            OrderQty(0),
            10,
        )
        .expect("context");

        let decision = planner
            .plan_passive_slice(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                10_000,
                context,
                ChildOrderId::new("child-cross").expect("child"),
                ClientOrderId::new("cl-cross").expect("client"),
                10_000,
            )
            .expect("decision");

        assert_eq!(decision.action(), PassiveQueueAction::CrossSpread);
        assert_eq!(
            decision.child().expect("child").request().limit_price,
            context.best_ask()
        );
    }

    #[test]
    fn passive_queue_rejects_invalid_context() {
        assert_eq!(
            PassiveQueueConfig::new(PassivePegMode::SameSide, OrderPrice(0)),
            Err(AlgoError::InvalidPassiveQueueParameters)
        );
        assert_eq!(
            PassiveQueueContext::new(OrderPrice(10), OrderPrice(10), OrderQty(0), OrderQty(0), 0),
            Err(AlgoError::InvalidPassiveQueueParameters)
        );
    }

    #[test]
    fn sor_prefers_better_scored_route() {
        let parent = parent();
        let planner = SorPlanner::new(SorConfig::default());
        let candidates = [
            SorRouteCandidate::new(
                RouteId::new("slow").expect("route"),
                OrderPrice(500_050),
                OrderQty(25),
            )
            .expect("candidate")
            .with_metrics(SorRouteMetrics::new(0, 2_000, 200, 4_000, 100, 9_000).expect("metrics")),
            SorRouteCandidate::new(
                RouteId::new("fast").expect("route"),
                OrderPrice(499_975),
                OrderQty(25),
            )
            .expect("candidate")
            .with_metrics(SorRouteMetrics::new(0, 100, 0, 9_000, 10, 10_000).expect("metrics")),
        ];
        let child_ids = [
            ChildOrderId::new("child-sor-1").expect("child"),
            ChildOrderId::new("child-sor-2").expect("child"),
        ];
        let client_ids = [
            ClientOrderId::new("cl-sor-1").expect("client"),
            ClientOrderId::new("cl-sor-2").expect("client"),
        ];

        let decision = planner
            .plan_routes::<2>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        let first = decision.allocations().next().expect("allocation");
        assert_eq!(
            first.plan().request().route_id,
            RouteId::new("fast").expect("route")
        );
        assert_eq!(first.plan().request().limit_price, OrderPrice(499_975));
    }

    #[test]
    fn sor_splits_across_routes_until_max_clip_is_reached() {
        let parent = parent();
        let planner = SorPlanner::new(SorConfig::new(3, SorScoreWeights::default()).expect("cfg"));
        let candidates = [
            SorRouteCandidate::new(
                RouteId::new("r1").expect("route"),
                OrderPrice(499_975),
                OrderQty(10),
            )
            .expect("candidate")
            .with_metrics(SorRouteMetrics::new(0, 0, 0, 10_000, 0, 10_000).expect("metrics")),
            SorRouteCandidate::new(
                RouteId::new("r2").expect("route"),
                OrderPrice(499_980),
                OrderQty(30),
            )
            .expect("candidate")
            .with_metrics(SorRouteMetrics::new(0, 0, 0, 1_000, 0, 10_000).expect("metrics")),
        ];
        let child_ids = [
            ChildOrderId::new("child-r1").expect("child"),
            ChildOrderId::new("child-r2").expect("child"),
            ChildOrderId::new("child-r3").expect("child"),
        ];
        let client_ids = [
            ClientOrderId::new("cl-r1").expect("client"),
            ClientOrderId::new("cl-r2").expect("client"),
            ClientOrderId::new("cl-r3").expect("client"),
        ];

        let decision = planner
            .plan_routes::<3>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        let qty: i64 = decision
            .allocations()
            .map(|allocation| allocation.plan().request().quantity.0)
            .sum();
        assert_eq!(decision.len(), 2);
        assert_eq!(qty, parent.max_clip().0);
    }

    #[test]
    fn sor_skips_blocked_and_unsupported_routes() {
        let parent = parent();
        let planner = SorPlanner::new(SorConfig::default());
        let candidates = [
            SorRouteCandidate::new(
                RouteId::new("blocked").expect("route"),
                OrderPrice(499_900),
                OrderQty(25),
            )
            .expect("candidate")
            .with_status(SorRouteStatus::Blocked),
            SorRouteCandidate::new(
                RouteId::new("market-only").expect("route"),
                OrderPrice(499_925),
                OrderQty(25),
            )
            .expect("candidate")
            .with_capability(SorRouteCapability::new(false, true)),
            SorRouteCandidate::new(
                RouteId::new("limit-ok").expect("route"),
                OrderPrice(500_000),
                OrderQty(25),
            )
            .expect("candidate"),
        ];
        let child_ids = [ChildOrderId::new("child-ok").expect("child")];
        let client_ids = [ClientOrderId::new("cl-ok").expect("client")];

        let decision = planner
            .plan_routes::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        let allocation = decision.allocations().next().expect("allocation");
        assert_eq!(
            allocation.plan().request().route_id,
            RouteId::new("limit-ok").expect("route")
        );
    }

    #[test]
    fn sor_requires_enough_caller_owned_ids() {
        let parent = parent();
        let planner = SorPlanner::new(SorConfig::default());
        let candidates = [SorRouteCandidate::new(
            RouteId::new("route").expect("route"),
            OrderPrice(500_000),
            OrderQty(25),
        )
        .expect("candidate")];

        assert_eq!(
            planner.plan_routes::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &[],
                &[],
                2_000,
            ),
            Err(AlgoError::InvalidSorParameters)
        );
    }

    #[test]
    fn liquidity_seeker_takes_high_fill_route() {
        let parent = parent();
        let config =
            LiquiditySeekingConfig::new(2, OrderQty(5), 0, 7_500, 1_500, 3, 4).expect("config");
        let planner = LiquiditySeekingPlanner::new(config, SorConfig::default());
        let route = SorRouteCandidate::new(
            RouteId::new("lit").expect("route"),
            OrderPrice(499_975),
            OrderQty(25),
        )
        .expect("route")
        .with_metrics(SorRouteMetrics::new(0, 100, 0, 9_000, 100, 10_000).expect("metrics"));
        let candidates =
            [LiquiditySeekingCandidate::new(route, 500, 25, OrderQty(0)).expect("candidate")];
        let child_ids = [ChildOrderId::new("liq-child").expect("child")];
        let client_ids = [ClientOrderId::new("liq-cl").expect("client")];

        let decision = planner
            .plan_liquidity::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        let allocation = decision.allocations().next().expect("allocation");
        assert_eq!(allocation.action(), LiquiditySeekingAction::Take);
        assert_eq!(allocation.plan().request().quantity, parent.max_clip());
        assert_eq!(allocation.plan().request().route_id, route.route_id());
    }

    #[test]
    fn liquidity_seeker_probes_lower_fill_hidden_route() {
        let parent = parent();
        let config =
            LiquiditySeekingConfig::new(1, OrderQty(10), 0, 7_500, 1_500, 3, 4).expect("config");
        let planner = LiquiditySeekingPlanner::new(config, SorConfig::default());
        let route = SorRouteCandidate::new(
            RouteId::new("dark").expect("route"),
            OrderPrice(500_000),
            OrderQty(100),
        )
        .expect("route")
        .with_metrics(SorRouteMetrics::new(0, 200, 0, 4_000, 100, 9_000).expect("metrics"));
        let candidates =
            [LiquiditySeekingCandidate::new(route, 2_500, 50, OrderQty(10)).expect("candidate")];
        let child_ids = [ChildOrderId::new("probe-child").expect("child")];
        let client_ids = [ClientOrderId::new("probe-cl").expect("client")];

        let decision = planner
            .plan_liquidity::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        let allocation = decision.allocations().next().expect("allocation");
        assert_eq!(allocation.action(), LiquiditySeekingAction::Probe);
        assert_eq!(allocation.plan().request().quantity, OrderQty(10));
    }

    #[test]
    fn liquidity_seeker_skips_toxic_routes() {
        let parent = parent();
        let planner = LiquiditySeekingPlanner::new(
            LiquiditySeekingConfig::new(1, OrderQty(10), 0, 7_500, 500, 3, 4).expect("config"),
            SorConfig::default(),
        );
        let route = SorRouteCandidate::new(
            RouteId::new("toxic").expect("route"),
            OrderPrice(499_975),
            OrderQty(100),
        )
        .expect("route")
        .with_metrics(SorRouteMetrics::new(0, 100, 0, 9_000, 1_000, 10_000).expect("metrics"));
        let candidates =
            [LiquiditySeekingCandidate::new(route, 0, 0, OrderQty(0)).expect("candidate")];
        let child_ids = [ChildOrderId::new("skip-child").expect("child")];
        let client_ids = [ClientOrderId::new("skip-cl").expect("client")];

        let decision = planner
            .plan_liquidity::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        assert!(decision.is_empty());
        assert_eq!(decision.skipped_routes(), 1);
    }

    #[test]
    fn liquidity_seeker_rejects_invalid_inputs() {
        assert_eq!(
            LiquiditySeekingConfig::new(0, OrderQty(1), 0, 0, 0, 0, 0),
            Err(AlgoError::InvalidLiquiditySeekingParameters)
        );
        let route = SorRouteCandidate::new(
            RouteId::new("route").expect("route"),
            OrderPrice(500_000),
            OrderQty(1),
        )
        .expect("route");
        assert_eq!(
            LiquiditySeekingCandidate::new(route, 10_001, 0, OrderQty(0)),
            Err(AlgoError::InvalidLiquiditySeekingParameters)
        );
    }

    #[test]
    fn sweep_walks_buy_levels_until_clip_or_collar() {
        let parent = parent();
        let planner =
            SweepPlanner::new(SweepConfig::new(3, OrderPrice(500_025), OrderQty(0)).expect("cfg"));
        let candidates = [
            SorRouteCandidate::new(
                RouteId::new("r2").expect("route"),
                OrderPrice(500_000),
                OrderQty(10),
            )
            .expect("candidate"),
            SorRouteCandidate::new(
                RouteId::new("r1").expect("route"),
                OrderPrice(499_975),
                OrderQty(10),
            )
            .expect("candidate"),
            SorRouteCandidate::new(
                RouteId::new("r3").expect("route"),
                OrderPrice(500_050),
                OrderQty(10),
            )
            .expect("candidate"),
        ];
        let child_ids = [
            ChildOrderId::new("sw-1").expect("child"),
            ChildOrderId::new("sw-2").expect("child"),
            ChildOrderId::new("sw-3").expect("child"),
        ];
        let client_ids = [
            ClientOrderId::new("sw-cl-1").expect("client"),
            ClientOrderId::new("sw-cl-2").expect("client"),
            ClientOrderId::new("sw-cl-3").expect("client"),
        ];

        let decision = planner
            .plan_sweep::<3>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.len(), 2);
        assert_eq!(decision.total_qty(), OrderQty(20));
        assert!(decision.collar_reached());
        let prices: Vec<i64> = decision
            .allocations()
            .map(|allocation| allocation.plan().request().limit_price.0)
            .collect();
        assert_eq!(prices, vec![499_975, 500_000]);
    }

    #[test]
    fn sweep_respects_sell_side_collar() {
        let sell_parent = ParentOrder::new(
            ParentOrderId::new("parent-sweep-sell").expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("sim").expect("route"),
            StrategyId::new("sweep").expect("strategy"),
            ExecutionSymbol::new("SIM", "ESZ6").expect("symbol"),
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::Day,
            OrderQty(100),
            OrderPrice(500_000),
            OrderPrice(0),
            1_000,
            11_000,
            OrderQty(10),
            OrderQty(25),
            0,
        )
        .expect("parent");
        let planner =
            SweepPlanner::new(SweepConfig::new(2, OrderPrice(499_975), OrderQty(0)).expect("cfg"));
        let candidates = [
            SorRouteCandidate::new(
                RouteId::new("sell-good").expect("route"),
                OrderPrice(500_025),
                OrderQty(25),
            )
            .expect("candidate"),
            SorRouteCandidate::new(
                RouteId::new("sell-bad").expect("route"),
                OrderPrice(499_950),
                OrderQty(25),
            )
            .expect("candidate"),
        ];
        let child_ids = [
            ChildOrderId::new("sell-sw-1").expect("child"),
            ChildOrderId::new("sell-sw-2").expect("child"),
        ];
        let client_ids = [
            ClientOrderId::new("sell-cl-1").expect("client"),
            ClientOrderId::new("sell-cl-2").expect("client"),
        ];

        let decision = planner
            .plan_sweep::<2>(
                &sell_parent,
                AlgoProgress::new(sell_parent.id(), sell_parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.len(), 1);
        assert_eq!(
            decision
                .allocations()
                .next()
                .expect("allocation")
                .plan()
                .request()
                .limit_price,
            OrderPrice(500_025)
        );
        assert!(decision.collar_reached());
    }

    #[test]
    fn sweep_suppresses_decision_below_min_fill() {
        let parent = parent();
        let planner =
            SweepPlanner::new(SweepConfig::new(1, OrderPrice(500_000), OrderQty(20)).expect("cfg"));
        let candidates = [SorRouteCandidate::new(
            RouteId::new("small").expect("route"),
            OrderPrice(499_975),
            OrderQty(10),
        )
        .expect("candidate")];
        let child_ids = [ChildOrderId::new("small-sw").expect("child")];
        let client_ids = [ClientOrderId::new("small-cl").expect("client")];

        let decision = planner
            .plan_sweep::<1>(
                &parent,
                AlgoProgress::new(parent.id(), parent.total_qty()),
                2_000,
                &candidates,
                &child_ids,
                &client_ids,
                2_000,
            )
            .expect("decision");

        assert!(decision.is_empty());
    }

    #[test]
    fn sweep_rejects_invalid_config() {
        assert_eq!(
            SweepConfig::new(0, OrderPrice(1), OrderQty(0)),
            Err(AlgoError::InvalidSweepParameters)
        );
        assert_eq!(
            SweepConfig::new(1, OrderPrice(0), OrderQty(0)),
            Err(AlgoError::InvalidSweepParameters)
        );
    }

    #[test]
    fn basket_plans_synchronized_due_legs() {
        let first = BasketLeg::new(parent(), BasketLegRole::Primary, 10_000).expect("leg");
        let second_parent = ParentOrder::new(
            ParentOrderId::new("parent-2").expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("hedge").expect("route"),
            StrategyId::new("basket").expect("strategy"),
            ExecutionSymbol::new("SIM", "NQZ6").expect("symbol"),
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::Day,
            OrderQty(50),
            OrderPrice(1_800_000),
            OrderPrice(0),
            1_000,
            11_000,
            OrderQty(5),
            OrderQty(20),
            0,
        )
        .expect("parent");
        let second = BasketLeg::new(second_parent, BasketLegRole::Hedge, -5_000).expect("leg");
        let legs = [first, second];
        let progresses = [
            AlgoProgress::new(first.parent().id(), first.parent().total_qty()),
            AlgoProgress::new(second.parent().id(), second.parent().total_qty()),
        ];
        let child_ids = [
            ChildOrderId::new("child-b1").expect("child"),
            ChildOrderId::new("child-b2").expect("child"),
        ];
        let client_ids = [
            ClientOrderId::new("cl-b1").expect("client"),
            ClientOrderId::new("cl-b2").expect("client"),
        ];

        let decision = BasketPlanner::new()
            .plan_synchronized_slice::<2>(&legs, &progresses, 6_000, &child_ids, &client_ids, 6_000)
            .expect("decision");

        assert_eq!(decision.len(), 2);
        let quantities: Vec<i64> = decision
            .allocations()
            .map(|allocation| allocation.plan().request().quantity.0)
            .collect();
        assert_eq!(quantities, vec![25, 20]);
    }

    #[test]
    fn basket_blocks_terminal_legs_without_releasing() {
        let terminal_parent = parent().with_status(ParentOrderStatus::Cancelled);
        let leg = BasketLeg::new(terminal_parent, BasketLegRole::Primary, 10_000).expect("leg");
        let progress = AlgoProgress::new(terminal_parent.id(), terminal_parent.total_qty());
        let child_ids = [ChildOrderId::new("child-b").expect("child")];
        let client_ids = [ClientOrderId::new("cl-b").expect("client")];

        let decision = BasketPlanner::new()
            .plan_synchronized_slice::<1>(
                &[leg],
                &[progress],
                6_000,
                &child_ids,
                &client_ids,
                6_000,
            )
            .expect("decision");

        assert!(decision.is_empty());
        assert_eq!(decision.blocked_legs(), 1);
    }

    #[test]
    fn basket_rejects_mismatched_inputs() {
        let leg = BasketLeg::new(parent(), BasketLegRole::Primary, 10_000).expect("leg");
        assert_eq!(
            BasketPlanner::new().plan_synchronized_slice::<1>(&[leg], &[], 6_000, &[], &[], 6_000),
            Err(AlgoError::InvalidBasketParameters)
        );
        assert_eq!(
            BasketLeg::new(parent(), BasketLegRole::Primary, 0),
            Err(AlgoError::InvalidBasketParameters)
        );
    }

    #[test]
    fn spread_plans_when_edge_meets_limit() {
        let buy_parent = parent();
        let sell_parent = sell_parent("spread-sell-1", OrderQty(100), OrderQty(10), OrderQty(25));
        let planner = SpreadPlanner::new(SpreadConfig::new(10_000, 50).expect("config"));
        let decision = planner
            .plan_spread(
                &buy_parent,
                AlgoProgress::new(buy_parent.id(), buy_parent.total_qty()),
                &sell_parent,
                AlgoProgress::new(sell_parent.id(), sell_parent.total_qty()),
                2_000,
                SpreadQuote::new(OrderPrice(500_000), OrderPrice(505_000)).expect("quote"),
                ChildOrderId::new("sp-buy").expect("child"),
                ClientOrderId::new("sp-buy-cl").expect("client"),
                ChildOrderId::new("sp-sell").expect("child"),
                ClientOrderId::new("sp-sell-cl").expect("client"),
                2_000,
            )
            .expect("decision");

        assert!(decision.estimate().executable());
        assert_eq!(decision.estimate().edge_bps(), 100);
        let buy = decision.buy().expect("buy");
        let sell = decision.sell().expect("sell");
        assert_eq!(buy.request().side, OrderSide::Buy);
        assert_eq!(sell.request().side, OrderSide::Sell);
        assert_eq!(buy.request().quantity, OrderQty(25));
        assert_eq!(sell.request().quantity, OrderQty(25));
        assert_eq!(buy.request().limit_price, OrderPrice(500_000));
        assert_eq!(sell.request().limit_price, OrderPrice(505_000));
    }

    #[test]
    fn spread_waits_when_edge_is_below_limit() {
        let buy_parent = parent();
        let sell_parent = sell_parent("spread-sell-2", OrderQty(100), OrderQty(10), OrderQty(25));
        let planner = SpreadPlanner::new(SpreadConfig::new(10_000, 50).expect("config"));
        let decision = planner
            .plan_spread(
                &buy_parent,
                AlgoProgress::new(buy_parent.id(), buy_parent.total_qty()),
                &sell_parent,
                AlgoProgress::new(sell_parent.id(), sell_parent.total_qty()),
                2_000,
                SpreadQuote::new(OrderPrice(500_000), OrderPrice(500_500)).expect("quote"),
                ChildOrderId::new("sp-wait-buy").expect("child"),
                ClientOrderId::new("sp-wait-buy-cl").expect("client"),
                ChildOrderId::new("sp-wait-sell").expect("child"),
                ClientOrderId::new("sp-wait-sell-cl").expect("client"),
                2_000,
            )
            .expect("decision");

        assert!(!decision.estimate().executable());
        assert!(decision.buy().is_none());
        assert!(decision.sell().is_none());
    }

    #[test]
    fn spread_sizes_by_ratio_and_available_leaves() {
        let buy_parent = parent();
        let sell_parent = sell_parent("spread-sell-3", OrderQty(8), OrderQty(1), OrderQty(8));
        let planner = SpreadPlanner::new(SpreadConfig::new(5_000, 100).expect("config"));
        let decision = planner
            .plan_spread(
                &buy_parent,
                AlgoProgress::new(buy_parent.id(), buy_parent.total_qty()),
                &sell_parent,
                AlgoProgress::new(sell_parent.id(), sell_parent.total_qty()),
                2_000,
                SpreadQuote::new(OrderPrice(500_000), OrderPrice(1_100_000)).expect("quote"),
                ChildOrderId::new("sp-ratio-buy").expect("child"),
                ClientOrderId::new("sp-ratio-buy-cl").expect("client"),
                ChildOrderId::new("sp-ratio-sell").expect("child"),
                ClientOrderId::new("sp-ratio-sell-cl").expect("client"),
                2_000,
            )
            .expect("decision");

        assert_eq!(decision.estimate().buy_qty(), OrderQty(16));
        assert_eq!(decision.estimate().sell_qty(), OrderQty(8));
        assert_eq!(
            decision.buy().expect("buy").request().quantity,
            OrderQty(16)
        );
        assert_eq!(
            decision.sell().expect("sell").request().quantity,
            OrderQty(8)
        );
    }

    #[test]
    fn spread_rejects_invalid_inputs() {
        assert_eq!(
            SpreadConfig::new(0, 0),
            Err(AlgoError::InvalidSpreadParameters)
        );
        assert_eq!(
            SpreadQuote::new(OrderPrice(0), OrderPrice(1)),
            Err(AlgoError::InvalidSpreadParameters)
        );
        let buy_parent = parent();
        let wrong_side_parent = ParentOrder::new(
            ParentOrderId::new("spread-wrong").expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("sim").expect("route"),
            StrategyId::new("spread").expect("strategy"),
            ExecutionSymbol::new("SIM", "NQZ6").expect("symbol"),
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::Day,
            OrderQty(100),
            OrderPrice(1_000_000),
            OrderPrice(0),
            1_000,
            11_000,
            OrderQty(10),
            OrderQty(25),
            0,
        )
        .expect("wrong side");

        assert_eq!(
            SpreadPlanner::new(SpreadConfig::new(10_000, 0).expect("config")).estimate(
                &buy_parent,
                AlgoProgress::new(buy_parent.id(), buy_parent.total_qty()),
                &wrong_side_parent,
                AlgoProgress::new(wrong_side_parent.id(), wrong_side_parent.total_qty()),
                SpreadQuote::new(OrderPrice(500_000), OrderPrice(505_000)).expect("quote"),
            ),
            Err(AlgoError::InvalidSpreadParameters)
        );
    }

    #[test]
    fn market_maker_quotes_both_sides_around_fair_value() {
        let template = parent();
        let config = MarketMakerConfig::new(
            OrderPrice(25),
            OrderQty(5),
            20,
            2,
            1_000,
            1_000,
            5_000,
            5_000,
        )
        .expect("config");
        let planner = MarketMakerPlanner::new(config);
        let context = MarketMakerContext::new(
            OrderPrice(500_000),
            OrderPrice(499_975),
            OrderPrice(500_025),
            OrderQty(0),
            OrderQty(100),
            10,
            10,
        )
        .expect("context");

        let decision = planner
            .plan_quotes(
                &template,
                2_000,
                context,
                ChildOrderId::new("mm-bid").expect("child"),
                ClientOrderId::new("mm-bid-cl").expect("client"),
                ChildOrderId::new("mm-ask").expect("child"),
                ClientOrderId::new("mm-ask-cl").expect("client"),
                2_000,
            )
            .expect("decision");

        let bid = decision.bid().expect("bid");
        let ask = decision.ask().expect("ask");
        assert_eq!(bid.request().side, OrderSide::Buy);
        assert_eq!(ask.request().side, OrderSide::Sell);
        assert!(bid.request().limit_price.0 < ask.request().limit_price.0);
        assert_eq!(bid.request().quantity, OrderQty(5));
        assert_eq!(ask.request().quantity, OrderQty(5));
    }

    #[test]
    fn market_maker_suppresses_bid_at_long_inventory_limit() {
        let template = parent();
        let planner = MarketMakerPlanner::new(MarketMakerConfig::default());
        let context = MarketMakerContext::new(
            OrderPrice(500_000),
            OrderPrice(499_975),
            OrderPrice(500_025),
            OrderQty(100),
            OrderQty(100),
            0,
            0,
        )
        .expect("context");

        let decision = planner
            .plan_quotes(
                &template,
                2_000,
                context,
                ChildOrderId::new("mm-bid").expect("child"),
                ClientOrderId::new("mm-bid-cl").expect("client"),
                ChildOrderId::new("mm-ask").expect("child"),
                ClientOrderId::new("mm-ask-cl").expect("client"),
                2_000,
            )
            .expect("decision");

        assert!(decision.bid().is_none());
        assert!(decision.ask().is_some());
        assert!(decision.estimate().adjusted_fair_value().0 < context.fair_value().0);
    }

    #[test]
    fn market_maker_rejects_invalid_parameters() {
        assert_eq!(
            MarketMakerConfig::new(OrderPrice(0), OrderQty(1), 10, 1, 100, 0, 0, 0),
            Err(AlgoError::InvalidMarketMakingParameters)
        );
        assert_eq!(
            MarketMakerContext::new(
                OrderPrice(1),
                OrderPrice(10),
                OrderPrice(9),
                OrderQty(0),
                OrderQty(1),
                0,
                0
            ),
            Err(AlgoError::InvalidMarketMakingParameters)
        );
    }

    #[test]
    fn shortfall_front_loads_on_adverse_buy_move() {
        let parent = parent();
        let planner = ImplementationShortfallPlanner::new(ImplementationShortfallConfig::default());
        let context = ImplementationShortfallContext::new(
            OrderPrice(500_000),
            OrderPrice(510_000),
            200,
            20,
            10,
        )
        .expect("context");
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());
        let estimate = planner
            .estimate(&parent, progress, 1_000, context)
            .expect("estimate");

        assert!(estimate.adverse_move_bps() > 0);
        assert!(estimate.urgency_bps() > 1_000);
        assert!(estimate.target_release_qty().0 > 0);
        let child = planner
            .plan_shortfall_slice(
                &parent,
                progress,
                1_000,
                context,
                ChildOrderId::new("child-is").expect("child"),
                ClientOrderId::new("cl-is").expect("client"),
                1_000,
            )
            .expect("plan")
            .expect("due");
        assert!(child.request().quantity.0 >= parent.min_clip().0);
        assert!(child.request().quantity.0 <= parent.max_clip().0);
    }

    #[test]
    fn shortfall_high_impact_can_wait_below_min_clip() {
        let parent = parent();
        let config =
            ImplementationShortfallConfig::new(100, 1_000, 0, 0, 0, 10_000).expect("config");
        let planner = ImplementationShortfallPlanner::new(config);
        let context = ImplementationShortfallContext::new(
            OrderPrice(500_000),
            OrderPrice(500_000),
            0,
            0,
            500,
        )
        .expect("context");
        let progress = AlgoProgress::new(parent.id(), parent.total_qty());

        let child = planner
            .plan_shortfall_slice(
                &parent,
                progress,
                1_000,
                context,
                ChildOrderId::new("child-wait").expect("child"),
                ClientOrderId::new("cl-wait").expect("client"),
                1_000,
            )
            .expect("plan");

        assert!(child.is_none());
    }

    #[test]
    fn shortfall_detects_adverse_sell_move() {
        let sell_parent = ParentOrder::new(
            ParentOrderId::new("parent-sell").expect("id"),
            AccountId::new("acct").expect("account"),
            RouteId::new("sim").expect("route"),
            StrategyId::new("is").expect("strategy"),
            ExecutionSymbol::new("SIM", "ESZ6").expect("symbol"),
            OrderSide::Sell,
            OrderType::Limit,
            TimeInForce::Day,
            OrderQty(100),
            OrderPrice(500_000),
            OrderPrice(0),
            1_000,
            11_000,
            OrderQty(10),
            OrderQty(25),
            0,
        )
        .expect("parent");
        let planner = ImplementationShortfallPlanner::new(ImplementationShortfallConfig::default());
        let context = ImplementationShortfallContext::new(
            OrderPrice(500_000),
            OrderPrice(490_000),
            50,
            10,
            0,
        )
        .expect("context");
        let estimate = planner
            .estimate(
                &sell_parent,
                AlgoProgress::new(sell_parent.id(), sell_parent.total_qty()),
                2_000,
                context,
            )
            .expect("estimate");

        assert!(estimate.adverse_move_bps() > 0);
        assert!(estimate.target_release_qty().0 > 10);
    }

    #[test]
    fn shortfall_rejects_invalid_parameters() {
        assert_eq!(
            ImplementationShortfallConfig::new(8_000, 7_000, 0, 0, 0, 0),
            Err(AlgoError::InvalidShortfallParameters)
        );
        assert_eq!(
            ImplementationShortfallContext::new(OrderPrice(0), OrderPrice(1), 0, 0, 0),
            Err(AlgoError::InvalidShortfallParameters)
        );
    }
}
