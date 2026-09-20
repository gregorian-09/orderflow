#[cfg(test)]
mod tests {
    use super::*;

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn order_request() -> OrderRequest {
        OrderRequest {
            client_order_id: id("C1"),
            account_id: id("A1"),
            route_id: id("R1"),
            strategy_id: id("S1"),
            symbol: ExecutionSymbol::new("CME", "ESM6").unwrap(),
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

    fn live_ctx() -> RiskContext {
        RiskContext {
            open_orders: 0,
            open_notional: 0,
            reference_price: OrderPrice(5000),
            duplicate_client_order_id: false,
            account_enabled: true,
            route_enabled: true,
            symbol_enabled: true,
            order_type_supported: true,
            tif_supported: true,
        }
    }

    #[test]
    fn fixed_ascii_rejects_invalid_input() {
        assert_eq!(
            ClientOrderId::new("abcdefghijklmnopqrstuvwxyz1234567890ABCDE").unwrap_err(),
            ExecutionCoreError::IdentifierTooLong {
                capacity: 40,
                actual: 41
            }
        );
        assert_eq!(
            ClientOrderId::new("ordé").unwrap_err(),
            ExecutionCoreError::NonAsciiIdentifier
        );
    }

    #[test]
    fn order_validation_requires_limit_price() {
        let mut req = order_request();
        req.limit_price = OrderPrice(0);
        assert_eq!(req.validate(), Err(ExecutionCoreError::InvalidPrice));
    }

    #[test]
    fn state_machine_accepts_and_fills_order() {
        let req = order_request();
        let mut sm = OrderStateMachine::new(&req);
        let ack = ExecutionEvent::accepted(&req, id("V1"));
        sm.apply(&ack).unwrap();
        assert_eq!(sm.state().status, OrderStatus::New);

        let mut fill = ack;
        fill.exec_type = ExecutionType::Trade;
        fill.order_status = OrderStatus::Filled;
        fill.execution_id = id("E1");
        fill.last_qty = OrderQty(10);
        fill.last_price = OrderPrice(5001);
        fill.cumulative_qty = OrderQty(10);
        fill.leaves_qty = OrderQty(0);
        fill.average_price = OrderPrice(5001);
        fill.ts_recv_ns = 3;
        sm.apply(&fill).unwrap();

        assert_eq!(sm.state().status, OrderStatus::Filled);
        assert_eq!(sm.state().cumulative_qty, OrderQty(10));
        assert!(sm.apply(&fill).is_err());
    }

    #[test]
    fn state_machine_handles_cancel_reject_as_status() {
        let req = order_request();
        let mut sm = OrderStateMachine::new(&req);
        sm.apply(&ExecutionEvent::accepted(&req, id("V1"))).unwrap();

        let mut pending_cancel = ExecutionEvent::accepted(&req, id("V1"));
        pending_cancel.exec_type = ExecutionType::CancelPending;
        pending_cancel.order_status = OrderStatus::PendingCancel;
        pending_cancel.ts_recv_ns = 4;
        sm.apply(&pending_cancel).unwrap();

        let mut reject = pending_cancel;
        reject.exec_type = ExecutionType::CancelReject;
        reject.order_status = OrderStatus::New;
        reject.ts_recv_ns = 5;
        sm.apply(&reject).unwrap();

        assert_eq!(sm.state().status, OrderStatus::New);
    }

    #[test]
    fn risk_gate_denies_by_default() {
        let req = order_request();
        let gate = BasicRiskGate::new(RiskLimits::default());
        let decision = gate.check_new(&req, &RiskContext::default());
        assert!(!decision.allowed);
        assert_eq!(decision.reason, RiskRejectReason::KillSwitch);
    }

    #[test]
    fn risk_gate_allows_configured_order() {
        let req = order_request();
        let gate = BasicRiskGate::new(RiskLimits {
            kill_switch: false,
            max_order_qty: 100,
            max_order_notional: 1_000_000,
            max_open_orders: 10,
            max_open_notional: 10_000_000,
            price_band_ticks: 10,
        });
        let decision = gate.check_new(&req, &live_ctx());
        assert!(decision.allowed);
    }

    #[test]
    fn risk_gate_rejects_price_band() {
        let mut req = order_request();
        req.limit_price = OrderPrice(5020);
        let gate = BasicRiskGate::new(RiskLimits {
            kill_switch: false,
            max_order_qty: 100,
            max_order_notional: 1_000_000,
            max_open_orders: 10,
            max_open_notional: 10_000_000,
            price_band_ticks: 10,
        });
        let decision = gate.check_new(&req, &live_ctx());
        assert!(!decision.allowed);
        assert_eq!(decision.reason, RiskRejectReason::PriceBand);
    }

    #[test]
    fn wal_record_round_trips_borrowed_payload() {
        let payload = b"submit:C1";
        let record =
            WalRecordView::new(WalRecordKind::CommandSubmit, WalSequence(1), 123, payload).unwrap();

        let mut encoded = vec![0; record.encoded_len()];
        assert_eq!(record.encode_into(&mut encoded).unwrap(), encoded.len());

        let (decoded, consumed) = WalRecordView::decode(&encoded).unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded.header.kind, WalRecordKind::CommandSubmit);
        assert_eq!(decoded.header.sequence, WalSequence(1));
        assert_eq!(decoded.header.timestamp_ns, 123);
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn wal_record_detects_payload_corruption() {
        let record =
            WalRecordView::new(WalRecordKind::ExecutionEvent, WalSequence(2), 456, b"fill")
                .unwrap();
        let mut encoded = Vec::new();
        record.append_to(&mut encoded);
        let last = encoded.last_mut().unwrap();
        *last ^= 0x01;

        let error = WalRecordView::decode(&encoded).unwrap_err();
        assert!(matches!(
            error,
            ExecutionWalError::ChecksumMismatch {
                field: WalChecksumField::Payload,
                ..
            }
        ));
    }

    #[test]
    fn wal_cursor_detects_strict_sequence_gap() {
        let first = WalRecordView::new(WalRecordKind::Heartbeat, WalSequence(1), 1, b"").unwrap();
        let second = WalRecordView::new(WalRecordKind::Heartbeat, WalSequence(3), 2, b"").unwrap();

        let mut encoded = Vec::new();
        first.append_to(&mut encoded);
        second.append_to(&mut encoded);

        let mut cursor = WalReplayCursor::new(&encoded);
        assert_eq!(
            cursor.next_record().unwrap().unwrap().header.sequence,
            WalSequence(1)
        );
        assert!(matches!(
            cursor.next_record().unwrap_err(),
            ExecutionWalError::SequenceGap {
                expected: WalSequence(2),
                actual: WalSequence(3)
            }
        ));
    }

    #[test]
    fn wal_integrity_report_summarizes_valid_bytes() {
        let first =
            WalRecordView::new(WalRecordKind::Heartbeat, WalSequence(10), 1, b"one").unwrap();
        let second =
            WalRecordView::new(WalRecordKind::Heartbeat, WalSequence(11), 2, b"two").unwrap();

        let mut encoded = Vec::new();
        first.append_to(&mut encoded);
        second.append_to(&mut encoded);

        let report = WalIntegrityReport::inspect(&encoded, true);
        assert!(report.valid);
        assert_eq!(report.records, 2);
        assert_eq!(report.bytes, encoded.len() as u64);
        assert_eq!(report.first_sequence, Some(WalSequence(10)));
        assert_eq!(report.last_sequence, Some(WalSequence(11)));
    }

    #[test]
    fn wal_integrity_report_detects_truncated_tail() {
        let record =
            WalRecordView::new(WalRecordKind::CheckpointMarker, WalSequence(1), 1, b"chk").unwrap();
        let mut encoded = Vec::new();
        record.append_to(&mut encoded);
        encoded.truncate(encoded.len() - 1);

        let report = WalIntegrityReport::inspect(&encoded, true);
        assert!(!report.valid);
        assert!(report.truncated_tail);
        assert_eq!(report.records, 0);
    }
}
