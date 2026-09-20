#[cfg(test)]
mod tests {
    use super::*;
    use of_fix::{encode_message, parse_message, FixFieldView};

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn parse_config() -> FixReportParseConfig {
        FixReportParseConfig::new(id("ACC"), id("FIX"), id("BINANCE"))
            .with_quantity_scale(100)
            .with_price_scale(10)
    }

    #[test]
    fn maps_trade_report_to_execution_event() {
        let report = FixExecutionReport {
            exec_type: FixExecType::Trade,
            ord_status: FixOrdStatus::PartiallyFilled,
            cl_ord_id: id("C1"),
            orig_cl_ord_id: ClientOrderId::empty(),
            order_id: id("V1"),
            exec_id: id("E1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            symbol: ExecutionSymbol::new("CME", "ESM6").unwrap(),
            last_qty: OrderQty(5),
            last_price: OrderPrice(5000),
            cumulative_qty: OrderQty(5),
            leaves_qty: OrderQty(5),
            average_price: OrderPrice(5000),
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
            text: ExecutionText::empty(),
        };

        let event = map_execution_report(&report);
        assert_eq!(event.exec_type, ExecutionType::Trade);
        assert_eq!(event.order_status, OrderStatus::PartiallyFilled);
        assert_eq!(event.cumulative_qty, OrderQty(5));
    }

    #[test]
    fn parses_fix_execution_report_from_message_view() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"8",
            &[
                (FixTag::EXEC_TYPE, b"1".as_slice()),
                (FixTag::ORD_STATUS, b"1".as_slice()),
                (FixTag::CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORDER_ID, b"V1".as_slice()),
                (FixTag::EXEC_ID, b"E1".as_slice()),
                (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
                (FixTag::LAST_QTY, b"1.25".as_slice()),
                (FixTag::LAST_PX, b"65000.5".as_slice()),
                (FixTag::CUM_QTY, b"1.25".as_slice()),
                (FixTag::LEAVES_QTY, b"0.75".as_slice()),
                (FixTag::AVG_PX, b"65000.5".as_slice()),
                (FixTag::TRANSACT_TIME, b"1784275200000000000".as_slice()),
                (FixTag::TEXT, b"partial".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let report =
            parse_execution_report(&message, parse_config(), 1784275200000000100).expect("map");

        assert_eq!(report.exec_type, FixExecType::Trade);
        assert_eq!(report.ord_status, FixOrdStatus::PartiallyFilled);
        assert_eq!(report.cl_ord_id, id("C1"));
        assert_eq!(report.order_id, id("V1"));
        assert_eq!(report.exec_id, id("E1"));
        assert_eq!(report.account_id, id("ACC"));
        assert_eq!(report.route_id, id("FIX"));
        assert_eq!(report.symbol.venue, id("BINANCE"));
        assert_eq!(report.symbol.instrument, id("BTCUSDT"));
        assert_eq!(report.last_qty, OrderQty(125));
        assert_eq!(report.last_price, OrderPrice(650005));
        assert_eq!(report.cumulative_qty, OrderQty(125));
        assert_eq!(report.leaves_qty, OrderQty(75));
        assert_eq!(report.average_price, OrderPrice(650005));
        assert_eq!(report.ts_exchange_ns, 1_784_275_200_000_000_000);
        assert_eq!(report.ts_recv_ns, 1_784_275_200_000_000_100);
        assert_eq!(report.text, id("partial"));
    }

    #[test]
    fn parses_account_from_fix_when_present() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"8",
            &[
                (FixTag::EXEC_TYPE, b"0".as_slice()),
                (FixTag::ORD_STATUS, b"0".as_slice()),
                (FixTag::CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORDER_ID, b"V1".as_slice()),
                (FixTag::EXEC_ID, b"E1".as_slice()),
                (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
                (FixTag(1), b"SUBACC".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let report = parse_execution_report(&message, parse_config(), 10).expect("map");
        assert_eq!(report.account_id, id("SUBACC"));
        assert_eq!(report.exec_type, FixExecType::New);
        assert_eq!(report.ord_status, FixOrdStatus::New);
    }

    #[test]
    fn parses_order_cancel_reject_from_message_view() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"9",
            &[
                (FixTag::ORDER_ID, b"V1".as_slice()),
                (FixTag::CL_ORD_ID, b"CANCEL-1".as_slice()),
                (FixTag::ORIG_CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORD_STATUS, b"0".as_slice()),
                (CXL_REJ_RESPONSE_TO_TAG, b"1".as_slice()),
                (CXL_REJ_REASON_TAG, b"1".as_slice()),
                (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
                (FixTag::TRANSACT_TIME, b"1784275200000000000".as_slice()),
                (FixTag::TEXT, b"too late".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let report = parse_order_cancel_reject(&message, parse_config(), 1_784_275_200_000_000_100)
            .expect("cancel reject");

        assert_eq!(
            report.response_to,
            FixCancelRejectResponseTo::OrderCancelRequest
        );
        assert_eq!(report.ord_status, FixOrdStatus::New);
        assert_eq!(report.cl_ord_id, id("CANCEL-1"));
        assert_eq!(report.orig_cl_ord_id, id("C1"));
        assert_eq!(report.order_id, id("V1"));
        assert_eq!(report.account_id, id("ACC"));
        assert_eq!(report.route_id, id("FIX"));
        assert_eq!(report.symbol.venue, id("BINANCE"));
        assert_eq!(report.symbol.instrument, id("BTCUSDT"));
        assert_eq!(report.cxl_rej_reason, 1);
        assert_eq!(report.ts_exchange_ns, 1_784_275_200_000_000_000);
        assert_eq!(report.ts_recv_ns, 1_784_275_200_000_000_100);
        assert_eq!(report.text, id("too late"));

        let event = map_order_cancel_reject(&report);
        assert_eq!(event.exec_type, ExecutionType::CancelReject);
        assert_eq!(event.order_status, OrderStatus::New);
        assert_eq!(event.client_order_id, id("CANCEL-1"));
        assert_eq!(event.orig_client_order_id, id("C1"));
        assert_eq!(event.reason, RiskRejectReason::None);
    }

    #[test]
    fn maps_order_cancel_replace_reject() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"9",
            &[
                (FixTag::CL_ORD_ID, b"REPLACE-1".as_slice()),
                (FixTag::ORIG_CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORD_STATUS, b"0".as_slice()),
                (CXL_REJ_RESPONSE_TO_TAG, b"2".as_slice()),
                (ACCOUNT_TAG, b"SUBACC".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 24];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let report = parse_order_cancel_reject(&message, parse_config(), 10).expect("map");
        let event = map_order_cancel_reject(&report);

        assert_eq!(
            report.response_to,
            FixCancelRejectResponseTo::OrderCancelReplaceRequest
        );
        assert_eq!(report.account_id, id("SUBACC"));
        assert_eq!(event.exec_type, ExecutionType::ReplaceReject);
        assert_eq!(event.venue_order_id, VenueOrderId::empty());
        assert_eq!(event.symbol.instrument, InstrumentId::empty());
    }

    #[test]
    fn rejects_invalid_cancel_reject_response_to() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"9",
            &[
                (FixTag::CL_ORD_ID, b"CANCEL-1".as_slice()),
                (FixTag::ORIG_CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORD_STATUS, b"0".as_slice()),
                (CXL_REJ_RESPONSE_TO_TAG, b"9".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 24];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            parse_order_cancel_reject(&message, parse_config(), 0),
            Err(FixReportParseError::InvalidCancelRejectResponseTo)
        );
    }

    #[test]
    fn rejects_missing_required_report_tag() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"8",
            &[
                (FixTag::EXEC_TYPE, b"0".as_slice()),
                (FixTag::ORD_STATUS, b"0".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            parse_execution_report(&message, parse_config(), 0),
            Err(FixReportParseError::MissingTag(FixTag::CL_ORD_ID))
        );
    }

    #[test]
    fn rejects_unrepresentable_decimal_scale() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"8",
            &[
                (FixTag::EXEC_TYPE, b"1".as_slice()),
                (FixTag::ORD_STATUS, b"1".as_slice()),
                (FixTag::CL_ORD_ID, b"C1".as_slice()),
                (FixTag::ORDER_ID, b"V1".as_slice()),
                (FixTag::EXEC_ID, b"E1".as_slice()),
                (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
                (FixTag::LAST_QTY, b"1.234".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            parse_execution_report(&message, parse_config(), 0),
            Err(FixReportParseError::InvalidNumber(FixTag::LAST_QTY))
        );
    }

    #[test]
    fn encodes_order_request_to_fix_new_order_single() {
        let request = OrderRequest {
            client_order_id: id("C1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            strategy_id: id("S1"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(125),
            limit_price: OrderPrice(650005),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 0,
            ts_recv_ns: 10,
        };
        let config = FixRequestEncodeConfig::new()
            .with_quantity_scale(100)
            .with_price_scale(10);
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:00.000");
        let mut raw = Vec::new();
        encode_order_request(
            &mut raw,
            FixVersion::Fix44,
            header,
            config,
            &request,
            b"20260717-12:00:00.000",
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.msg_type(), Some(b"D".as_slice()));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"C1".as_slice()));
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::SYMBOL), Some(b"BTCUSDT".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"1".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"1.25".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65000.5".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"0".as_slice()));
    }

    #[test]
    fn encodes_stop_limit_order_request_to_fix_new_order_single() {
        let request = OrderRequest {
            client_order_id: id("C2"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            strategy_id: id("S1"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            side: OrderSide::Sell,
            order_type: OrderType::StopLimit,
            time_in_force: TimeInForce::Gtc,
            quantity: OrderQty(100),
            limit_price: OrderPrice(650005),
            stop_price: OrderPrice(649505),
            ts_exchange_ns: 0,
            ts_recv_ns: 10,
        };
        let config = FixRequestEncodeConfig::new()
            .with_quantity_scale(100)
            .with_price_scale(10);
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:00.000");
        let mut raw = Vec::new();
        encode_order_request(
            &mut raw,
            FixVersion::Fix44,
            header,
            config,
            &request,
            b"20260717-12:00:00.000",
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.msg_type(), Some(b"D".as_slice()));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"C2".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"2".as_slice()));
        assert_eq!(message.get(FixTag::ORD_TYPE), Some(b"4".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"1".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65000.5".as_slice()));
        assert_eq!(message.get(FixTag::STOP_PX), Some(b"64950.5".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"1".as_slice()));
    }

    #[test]
    fn encodes_cancel_request_with_explicit_side_context() {
        let request = CancelRequest {
            client_order_id: id("CXL-1"),
            orig_client_order_id: id("C1"),
            venue_order_id: id("V1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            ts_recv_ns: 10,
        };
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 8, b"20260717-12:00:01.000");
        let context = FixCancelEncodeContext::new(OrderSide::Sell, b"20260717-12:00:01.000");
        let mut raw = Vec::new();
        encode_cancel_request(&mut raw, FixVersion::Fix44, header, &request, context)
            .expect("encode");

        let mut scratch = [FixFieldView::empty(); 24];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.msg_type(), Some(b"F".as_slice()));
        assert_eq!(message.get(FixTag::ORIG_CL_ORD_ID), Some(b"C1".as_slice()));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"CXL-1".as_slice()));
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"2".as_slice()));
    }

    #[test]
    fn encodes_amend_request_with_explicit_order_context() {
        let request = AmendRequest {
            client_order_id: id("RPL-1"),
            orig_client_order_id: id("C1"),
            venue_order_id: id("V1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            quantity: OrderQty(200),
            limit_price: OrderPrice(650100),
            ts_recv_ns: 10,
        };
        let config = FixRequestEncodeConfig::new()
            .with_quantity_scale(100)
            .with_price_scale(10);
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 9, b"20260717-12:00:02.000");
        let context = FixAmendEncodeContext::new(
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::Gtc,
            b"20260717-12:00:02.000",
        );
        let mut raw = Vec::new();
        encode_amend_request(
            &mut raw,
            FixVersion::Fix44,
            header,
            config,
            &request,
            context,
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.msg_type(), Some(b"G".as_slice()));
        assert_eq!(message.get(FixTag::ORIG_CL_ORD_ID), Some(b"C1".as_slice()));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"RPL-1".as_slice()));
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"2".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65010".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"1".as_slice()));
    }

    #[test]
    fn encodes_stop_amend_request_with_explicit_stop_context() {
        let request = AmendRequest {
            client_order_id: id("RPL-STP"),
            orig_client_order_id: id("C1"),
            venue_order_id: id("V1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            quantity: OrderQty(200),
            limit_price: OrderPrice(650100),
            ts_recv_ns: 10,
        };
        let config = FixRequestEncodeConfig::new()
            .with_quantity_scale(100)
            .with_price_scale(10);
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 9, b"20260717-12:00:02.000");
        let context = FixStopAmendEncodeContext::new(
            OrderSide::Buy,
            OrderType::StopLimit,
            TimeInForce::Gtc,
            OrderPrice(649900),
            b"20260717-12:00:02.000",
        );
        let mut raw = Vec::new();
        encode_stop_amend_request(
            &mut raw,
            FixVersion::Fix44,
            header,
            config,
            &request,
            context,
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.msg_type(), Some(b"G".as_slice()));
        assert_eq!(message.get(FixTag::ORD_TYPE), Some(b"4".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65010".as_slice()));
        assert_eq!(message.get(FixTag::STOP_PX), Some(b"64990".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"1".as_slice()));
    }

    #[test]
    fn request_encoder_rejects_invalid_shapes() {
        let mut request = OrderRequest {
            client_order_id: id("C1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            strategy_id: id("S1"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            side: OrderSide::Buy,
            order_type: OrderType::Stop,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(125),
            limit_price: OrderPrice(0),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 0,
            ts_recv_ns: 10,
        };
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:00.000");
        let mut raw = Vec::new();
        assert_eq!(
            encode_order_request(
                &mut raw,
                FixVersion::Fix44,
                header,
                FixRequestEncodeConfig::new().with_quantity_scale(100),
                &request,
                b"20260717-12:00:00.000",
            ),
            Err(FixRequestEncodeError::InvalidPrice)
        );

        request.order_type = OrderType::Limit;
        request.limit_price = OrderPrice(650005);
        assert_eq!(
            encode_order_request(
                &mut raw,
                FixVersion::Fix44,
                header,
                FixRequestEncodeConfig {
                    quantity_scale: 3,
                    price_scale: 10,
                },
                &request,
                b"20260717-12:00:00.000",
            ),
            Err(FixRequestEncodeError::InvalidScale)
        );

        let amend = AmendRequest {
            client_order_id: id("RPL-STOP"),
            orig_client_order_id: id("C1"),
            venue_order_id: id("V1"),
            account_id: id("ACC"),
            route_id: id("FIX"),
            symbol: ExecutionSymbol::new("BINANCE", "BTCUSDT").unwrap(),
            quantity: OrderQty(125),
            limit_price: OrderPrice(650005),
            ts_recv_ns: 10,
        };
        let context = FixAmendEncodeContext::new(
            OrderSide::Buy,
            OrderType::StopLimit,
            TimeInForce::Day,
            b"20260717-12:00:00.000",
        );
        assert_eq!(
            encode_amend_request(
                &mut raw,
                FixVersion::Fix44,
                header,
                FixRequestEncodeConfig::new()
                    .with_quantity_scale(100)
                    .with_price_scale(10),
                &amend,
                context,
            ),
            Err(FixRequestEncodeError::UnsupportedOrderType)
        );

        let non_stop_context = FixStopAmendEncodeContext::new(
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::Day,
            OrderPrice(650000),
            b"20260717-12:00:00.000",
        );
        assert_eq!(
            encode_stop_amend_request(
                &mut raw,
                FixVersion::Fix44,
                header,
                FixRequestEncodeConfig::new()
                    .with_quantity_scale(100)
                    .with_price_scale(10),
                &amend,
                non_stop_context,
            ),
            Err(FixRequestEncodeError::UnsupportedOrderType)
        );
    }

    #[test]
    fn fix_adapter_fails_closed_without_transport() {
        let cfg = FixSessionConfig::new("FIX.4.4", "SENDER", "TARGET", 30).unwrap();
        let mut adapter = FixExecutionAdapter::new(cfg);
        assert!(adapter.connect().is_err());
        assert_eq!(
            adapter.capabilities().latency_class,
            LatencyClass::NativeFix
        );
        assert!(adapter.health().degraded);
    }
}
