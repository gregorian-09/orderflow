#[cfg(test)]
mod tests {
    use super::*;
    use of_execution_core::{ExecutionSymbol, FixedAscii, OrderSide};

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn route() -> RouteConfig {
        route_for("ES", 10)
    }

    fn route_for(instrument: &str, max_open_orders: u32) -> RouteConfig {
        RouteConfig {
            route_id: id("SIM"),
            account_id: id("ACC"),
            symbol: ExecutionSymbol::new("SIM", instrument).unwrap(),
            enabled: true,
            risk_limits: RiskLimits {
                kill_switch: false,
                max_order_qty: 100,
                max_order_notional: 1_000_000,
                max_open_orders,
                max_open_notional: 10_000_000,
                price_band_ticks: 0,
            },
        }
    }

    fn order() -> OrderRequest {
        order_for("C1", "ES")
    }

    fn order_for(client_order_id: &str, instrument: &str) -> OrderRequest {
        OrderRequest {
            client_order_id: id(client_order_id),
            account_id: id("ACC"),
            route_id: id("SIM"),
            strategy_id: id("STRAT"),
            symbol: ExecutionSymbol::new("SIM", instrument).unwrap(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(10),
            limit_price: OrderPrice(5000),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }
    }

    #[test]
    fn simulated_engine_submits_and_fills() {
        let mut engine = simulated_engine(route());
        let stopped = engine.runbook_snapshot();
        assert!(!stopped.started);
        assert!(stopped.new_submissions_blocked);
        assert!(stopped.operator_attention_required);

        engine.start().unwrap();
        let started = engine.runbook_snapshot();
        assert!(started.started);
        assert!(started.connected);
        assert_eq!(started.route_count, 1);
        assert_eq!(started.enabled_route_count, 1);
        assert_eq!(started.kill_switch_route_count, 0);
        assert!(started.can_submit_new_orders());
        assert!(!started.operator_attention_required);

        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order(), &mut out).unwrap();

        assert_eq!(out.len(), 2);
        assert_eq!(out.as_slice()[0].exec_type, ExecutionType::Ack);
        assert_eq!(out.as_slice()[1].exec_type, ExecutionType::Trade);
        assert_eq!(
            engine.order_state(&id("C1")).unwrap().status,
            OrderStatus::Filled
        );
        assert_eq!(engine.metrics().submitted, 1);
        assert_eq!(engine.metrics().events_applied, 2);
        let after_fill = engine.runbook_snapshot();
        assert_eq!(after_fill.submitted, 1);
        assert_eq!(after_fill.events_applied, 2);
        assert_eq!(after_fill.open_order_count, 0);
        assert_eq!(after_fill.terminal_order_count, 1);
    }

    #[test]
    fn runbook_snapshot_marks_killed_routes_without_mutating_engine() {
        let mut killed = route();
        killed.risk_limits.kill_switch = true;
        let mut engine = simulated_engine(killed);
        engine.start().unwrap();

        let snapshot = engine.runbook_snapshot();
        assert!(snapshot.started);
        assert!(snapshot.connected);
        assert_eq!(snapshot.kill_switch_route_count, 1);
        assert!(snapshot.new_submissions_blocked);
        assert!(!snapshot.can_submit_new_orders());
        assert!(snapshot.operator_attention_required);
    }

    #[test]
    fn audit_bundle_manifest_counts_journal_and_runbook_inputs() {
        let mut engine = simulated_engine(route());
        engine.start().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order(), &mut out).unwrap();

        let manifest = engine.audit_bundle_manifest_at(123).unwrap();
        assert_eq!(
            manifest.schema_version,
            ExecutionAuditBundleManifest::SCHEMA_VERSION
        );
        assert_eq!(manifest.generated_ns, 123);
        assert_eq!(manifest.route_count, 1);
        assert_eq!(manifest.enabled_route_count, 1);
        assert_eq!(manifest.open_order_count, 0);
        assert_eq!(manifest.terminal_order_count, 1);
        assert_eq!(manifest.journal_record_count, 3);
        assert_eq!(manifest.journal_command_count, 1);
        assert_eq!(manifest.journal_event_count, 2);
        assert_eq!(manifest.metrics.submitted, 1);
        assert_eq!(manifest.runbook.submitted, 1);
        assert!(manifest.submissions_enabled);
        assert!(!manifest.requires_operator_review());
    }

    #[test]
    fn risk_rejects_duplicate_client_order_id() {
        let mut engine = simulated_engine(route());
        engine.start().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order(), &mut out).unwrap();
        out.clear();

        let err = engine.submit(order(), &mut out).unwrap_err();
        assert!(matches!(err, ExecutionError::RiskRejected(_)));
        assert_eq!(
            out.as_slice()[0].reason,
            RiskRejectReason::DuplicateClientOrderId
        );
    }

    #[test]
    fn buffer_bound_is_enforced() {
        let mut buffer = ExecutionEventBuffer::with_capacity(0);
        let req = order();
        let event = ExecutionEvent::accepted(&req, id("V1"));
        assert_eq!(buffer.push(event).unwrap_err(), ExecutionError::BufferFull);
    }

    #[test]
    fn journal_records_commands_and_events() {
        let mut engine = simulated_engine(route());
        engine.start().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order(), &mut out).unwrap();

        let mut records = Vec::new();
        let count = engine.replay_journal(&mut records).unwrap();
        assert_eq!(count, 3);
        assert!(matches!(
            records[0],
            JournalRecord::Command {
                kind: JournalCommandKind::Submit,
                ..
            }
        ));
    }

    #[test]
    fn multi_route_engine_submits_multiple_symbols() {
        let routes = vec![route_for("ES", 10), route_for("NQ", 10)];
        let mut engine = simulated_engine_with_routes(routes);
        engine.start().unwrap();

        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order_for("ES-1", "ES"), &mut out).unwrap();
        out.clear();
        engine.submit(order_for("NQ-1", "NQ"), &mut out).unwrap();

        assert_eq!(
            engine.order_state(&id("ES-1")).unwrap().status,
            OrderStatus::Filled
        );
        assert_eq!(
            engine.order_state(&id("NQ-1")).unwrap().status,
            OrderStatus::Filled
        );
        assert_eq!(engine.metrics().submitted, 2);
    }

    #[test]
    fn route_limits_are_scoped_by_symbol() {
        let routes = vec![route_for("ES", 1), route_for("NQ", 1)];
        let mut engine = ExecutionEngine::new(
            SimExecutionAdapter::default().with_partial_fill(true),
            AllowAllRiskGate,
            InMemoryJournal::default(),
            routes,
        );
        engine.start().unwrap();

        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order_for("ES-1", "ES"), &mut out).unwrap();
        out.clear();
        engine.submit(order_for("NQ-1", "NQ"), &mut out).unwrap();
        out.clear();

        let err = engine
            .submit(order_for("ES-2", "ES"), &mut out)
            .unwrap_err();
        assert!(matches!(
            err,
            ExecutionError::RiskRejected(RiskRejectReason::MaxOpenOrders)
        ));
        assert_eq!(out.as_slice()[0].reason, RiskRejectReason::MaxOpenOrders);
    }

    #[test]
    fn engine_evaluates_venue_reconciliation_policy() {
        let mut engine = ExecutionEngine::new(
            SimExecutionAdapter::default().with_partial_fill(true),
            AllowAllRiskGate,
            InMemoryJournal::default(),
            vec![route()],
        );
        engine.start().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        engine.submit(order(), &mut out).unwrap();

        let mut venue = engine.open_order_states();
        assert_eq!(venue.len(), 1);
        venue[0].status = OrderStatus::Cancelled;

        let report = engine.reconcile_open_orders_with(&venue);
        assert_eq!(report.details.len(), 1);
        assert_eq!(
            report.details[0].issue,
            ReconciliationIssueKind::StatusMismatch
        );

        let decision = engine.evaluate_reconciliation(
            &venue,
            ReconciliationPolicy::fail_closed()
                .with_status_mismatch(ReconciliationPolicyAction::AcceptVenueTruth),
        );
        assert!(!decision.submissions_enabled);
        assert!(decision.local_restates_required);
        assert_eq!(
            decision.items[0].action,
            ReconciliationPolicyAction::AcceptVenueTruth
        );
    }

    #[test]
    fn concurrent_worker_processes_multiple_producers_serially() {
        let routes = vec![route_for("ES", 10), route_for("NQ", 10)];
        let engine = simulated_engine_with_routes(routes);
        let mut concurrent =
            ConcurrentExecutionEngine::spawn(engine, ConcurrentExecutionConfig::default()).unwrap();
        let es_sender = concurrent.command_sender();
        let nq_sender = concurrent.command_sender();

        let es_thread =
            std::thread::spawn(move || es_sender.submit(order_for("ES-C", "ES")).unwrap());
        let nq_thread =
            std::thread::spawn(move || nq_sender.submit(order_for("NQ-C", "NQ")).unwrap());
        let es_seq = es_thread.join().unwrap();
        let nq_seq = nq_thread.join().unwrap();

        let first = concurrent.recv_report().unwrap();
        let second = concurrent.recv_report().unwrap();
        assert!(first.result.is_ok());
        assert!(second.result.is_ok());
        assert_eq!(first.events.len(), 2);
        assert_eq!(second.events.len(), 2);
        assert!(first.sequence == es_seq || second.sequence == es_seq);
        assert!(first.sequence == nq_seq || second.sequence == nq_seq);

        concurrent.request_stop().unwrap();
        let stop = concurrent.recv_report().unwrap();
        assert_eq!(stop.kind, ExecutionCommandKind::Stop);
        assert!(stop.result.is_ok());
        concurrent.join().unwrap();
    }

    #[test]
    fn concurrent_worker_reports_risk_rejection() {
        let routes = vec![route_for("ES", 1)];
        let engine = ExecutionEngine::new(
            SimExecutionAdapter::default().with_partial_fill(true),
            AllowAllRiskGate,
            InMemoryJournal::default(),
            routes,
        );
        let mut concurrent =
            ConcurrentExecutionEngine::spawn(engine, ConcurrentExecutionConfig::default()).unwrap();

        concurrent
            .send(ExecutionCommand::Submit(order_for("ES-OPEN", "ES")))
            .unwrap();
        assert!(concurrent.recv_report().unwrap().result.is_ok());
        concurrent
            .send(ExecutionCommand::Submit(order_for("ES-REJECT", "ES")))
            .unwrap();
        let report = concurrent.recv_report().unwrap();

        assert!(matches!(
            report.result,
            Err(ExecutionError::RiskRejected(
                RiskRejectReason::MaxOpenOrders
            ))
        ));
        assert_eq!(report.events.len(), 1);
        assert_eq!(
            report.events.as_slice()[0].reason,
            RiskRejectReason::MaxOpenOrders
        );

        concurrent.request_stop().unwrap();
        let _ = concurrent.recv_report().unwrap();
        concurrent.join().unwrap();
    }
}
