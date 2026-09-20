#[cfg(test)]
mod tests {
    use super::*;

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn symbol(instrument: &str) -> ExecutionSymbol {
        ExecutionSymbol {
            venue: id::<16>("SIM"),
            instrument: id::<32>(instrument),
        }
    }

    fn route() -> RouteConfig {
        RouteConfig {
            route_id: id("SIM"),
            account_id: id("ACC"),
            symbol: symbol("ES"),
            enabled: true,
            risk_limits: RiskLimits {
                kill_switch: false,
                max_order_qty: 100,
                max_order_notional: 1_000_000,
                max_open_orders: 10,
                max_open_notional: 10_000_000,
                price_band_ticks: 0,
            },
        }
    }

    fn order(client_order_id: &str) -> OrderRequest {
        OrderRequest {
            client_order_id: id(client_order_id),
            account_id: id("ACC"),
            route_id: id("SIM"),
            strategy_id: id("STRAT"),
            symbol: symbol("ES"),
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
    fn fanout_drops_when_subscriber_queue_is_full() {
        let fanout = ExecutionEventFanout::new(1);
        let sub = fanout.subscribe();
        let req = order("C1");
        fanout.publish(ExecutionEvent::accepted(&req, id("V1")));
        fanout.publish(ExecutionEvent::accepted(&req, id("V2")));

        assert!(sub.try_recv().is_some());
        assert_eq!(fanout.dropped_events(), 1);
    }

    #[test]
    fn file_journal_replays_commands_and_events() {
        let path =
            std::env::temp_dir().join(format!("orderflow-journal-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut journal = FileExecutionJournal::open(&path, false).unwrap();
        let req = order("C1");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req.client_order_id,
                req.ts_recv_ns,
            )
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req, id("V1")))
            .unwrap();
        drop(journal);

        let journal = FileExecutionJournal::open(&path, false).unwrap();
        let mut records = Vec::new();
        assert_eq!(journal.replay(&mut records).unwrap(), 2);
        assert!(matches!(records[0], JournalRecord::Command { .. }));
        assert!(matches!(records[1], JournalRecord::Event(_)));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wal_journal_replays_commands_and_events() {
        let path = std::env::temp_dir().join(format!("orderflow-wal-{}.ofwal", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut journal = WalExecutionJournal::open_path(&path, WalSyncPolicy::Never).unwrap();
        let req = order("C1");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req.client_order_id,
                req.ts_recv_ns,
            )
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req, id("V1")))
            .unwrap();
        assert_eq!(journal.next_sequence(), WalSequence(3));
        let metrics = journal.metrics();
        assert_eq!(metrics.records_written, 2);
        assert!(metrics.bytes_written > 0);
        assert_eq!(metrics.write_failures, 0);
        drop(journal);

        let journal = WalExecutionJournal::open_path(&path, WalSyncPolicy::Never).unwrap();
        assert_eq!(journal.next_sequence(), WalSequence(3));
        let report = journal.integrity_report().unwrap();
        assert!(report.valid);
        assert_eq!(report.records, 2);

        let mut records = Vec::new();
        assert_eq!(journal.replay(&mut records).unwrap(), 2);
        assert!(matches!(records[0], JournalRecord::Command { .. }));
        assert!(matches!(records[1], JournalRecord::Event(_)));

        let mut tail = Vec::new();
        let replay = journal.replay_from(WalSequence(2), &mut tail).unwrap();
        assert_eq!(replay.records, 1);
        assert_eq!(replay.first_sequence, Some(WalSequence(2)));
        assert!(matches!(tail[0], JournalRecord::Event(_)));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wal_journal_fails_closed_on_corruption() {
        let path = std::env::temp_dir().join(format!(
            "orderflow-wal-corrupt-{}.ofwal",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut journal = WalExecutionJournal::open_path(&path, WalSyncPolicy::Never).unwrap();
        let req = order("C2");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req.client_order_id,
                req.ts_recv_ns,
            )
            .unwrap();
        drop(journal);

        let mut bytes = std::fs::read(&path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x01;
        std::fs::write(&path, bytes).unwrap();

        assert!(WalExecutionJournal::open_path(&path, WalSyncPolicy::Never).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn segmented_wal_rotates_and_replays_across_segments() {
        let root =
            std::env::temp_dir().join(format!("orderflow-segmented-wal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root)
                .with_sync_policy(WalSyncPolicy::Never)
                .with_max_segment_records(2),
        )
        .unwrap();

        let req1 = order("C1");
        let req2 = order("C2");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req1.client_order_id,
                req1.ts_recv_ns,
            )
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req1, id("V1")))
            .unwrap();
        journal
            .record_command(
                JournalCommandKind::Submit,
                req2.client_order_id,
                req2.ts_recv_ns,
            )
            .unwrap();
        journal.sync().unwrap();

        assert_eq!(journal.next_sequence(), WalSequence(5));
        let metrics = journal.metrics();
        assert_eq!(metrics.records_written, 4);
        assert!(metrics.bytes_written > 0);
        assert_eq!(metrics.segment_rotations, 1);
        assert!(metrics.sync_count > 0);
        assert!(metrics.manifest_writes > 0);
        assert_eq!(journal.manifest().segments.len(), 2);
        assert!(journal.manifest().segments[0].sealed);
        assert_eq!(
            journal.manifest().segments[0].last_sequence,
            Some(WalSequence(3))
        );
        assert_eq!(
            journal.manifest().segments[1].first_sequence,
            Some(WalSequence(4))
        );

        let report = journal.integrity_report().unwrap();
        assert!(report.valid);
        assert_eq!(report.segments, 2);
        assert_eq!(report.records, 4);

        let mut replayed = Vec::new();
        assert_eq!(journal.replay(&mut replayed).unwrap(), 3);
        assert!(matches!(replayed[0], JournalRecord::Command { .. }));
        assert!(matches!(replayed[1], JournalRecord::Event(_)));
        assert!(matches!(replayed[2], JournalRecord::Command { .. }));

        let mut tail = Vec::new();
        let replay = journal.replay_from(WalSequence(4), &mut tail).unwrap();
        assert_eq!(replay.records, 1);
        assert_eq!(replay.first_sequence, Some(WalSequence(4)));
        assert!(matches!(tail[0], JournalRecord::Command { .. }));
        assert!(root.join("manifest").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn segmented_wal_reopens_and_continues_after_sealed_segment() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-segmented-wal-reopen-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut journal = SegmentedWalExecutionJournal::open(
                WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
            )
            .unwrap();
            let req = order("C1");
            journal
                .record_command(
                    JournalCommandKind::Submit,
                    req.client_order_id,
                    req.ts_recv_ns,
                )
                .unwrap();
            journal.rotate_segment().unwrap();
            journal.sync().unwrap();
            assert_eq!(journal.next_sequence(), WalSequence(3));
            assert_eq!(journal.manifest().segments.len(), 2);
        }

        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        assert_eq!(journal.next_sequence(), WalSequence(3));
        let req = order("C2");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req.client_order_id,
                req.ts_recv_ns,
            )
            .unwrap();

        let mut replayed = Vec::new();
        assert_eq!(journal.replay(&mut replayed).unwrap(), 2);
        assert_eq!(journal.manifest().segments.len(), 2);
        assert_eq!(
            journal.manifest().segments[1].first_sequence,
            Some(WalSequence(3))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn segmented_wal_fails_closed_on_corrupt_segment() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-segmented-wal-corrupt-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut journal = SegmentedWalExecutionJournal::open(
                WalSegmentConfig::new(&root)
                    .with_sync_policy(WalSyncPolicy::Never)
                    .with_max_segment_records(1),
            )
            .unwrap();
            let req = order("C1");
            journal
                .record_command(
                    JournalCommandKind::Submit,
                    req.client_order_id,
                    req.ts_recv_ns,
                )
                .unwrap();
            journal.sync().unwrap();
        }

        let segment_path = root.join("wal-000000000001.ofwal");
        let mut bytes = std::fs::read(&segment_path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x01;
        std::fs::write(&segment_path, bytes).unwrap();

        assert!(SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never)
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn segmented_wal_inspect_root_reports_valid_and_corrupt_segments() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-segmented-wal-inspect-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        {
            let mut journal = SegmentedWalExecutionJournal::open(
                WalSegmentConfig::new(&root)
                    .with_sync_policy(WalSyncPolicy::Never)
                    .with_max_segment_records(1),
            )
            .unwrap();
            let req = order("C1");
            journal
                .record_command(
                    JournalCommandKind::Submit,
                    req.client_order_id,
                    req.ts_recv_ns,
                )
                .unwrap();
            journal.sync().unwrap();
        }

        let report = SegmentedWalExecutionJournal::inspect_root(&root).unwrap();
        assert!(report.valid);
        assert_eq!(report.segments, 1);
        assert_eq!(report.records, 1);
        assert_eq!(report.first_sequence, Some(WalSequence(1)));
        assert_eq!(report.last_sequence, Some(WalSequence(1)));

        let segment_path = root.join("wal-000000000001.ofwal");
        let mut bytes = std::fs::read(&segment_path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x01;
        std::fs::write(&segment_path, bytes).unwrap();

        let report = SegmentedWalExecutionJournal::inspect_root(&root).unwrap();
        assert!(!report.valid);
        assert_eq!(report.segments, 1);
        assert_eq!(report.checksum_failures, 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn checkpoint_store_saves_loads_and_prunes() {
        let root =
            std::env::temp_dir().join(format!("orderflow-checkpoints-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut store = FileExecutionCheckpointStore::open(
            CheckpointConfig::new(&root)
                .with_sync_on_save(false)
                .with_max_retained(1),
        )
        .unwrap();
        let req = order("C3");
        let mut state = OrderState::pending_new(&req);
        state.venue_order_id = id("V3");
        let position = CheckpointPosition {
            key: PositionKey {
                account_id: req.account_id,
                strategy_id: req.strategy_id,
                symbol: req.symbol,
            },
            position: Position {
                net_qty: 10,
                buy_qty: 10,
                sell_qty: 0,
                gross_notional: 50_000,
                average_price: 5_000,
            },
        };

        let first = ExecutionCheckpoint::new(1, WalSequence(10), 100)
            .with_open_orders(vec![state])
            .with_positions(vec![position])
            .with_route_config_hash(7)
            .with_kill_switch(true);
        let second = ExecutionCheckpoint::new(2, WalSequence(20), 200);

        let manifest = store.save_checkpoint(&first).unwrap();
        assert_eq!(manifest.last_applied_sequence, WalSequence(10));
        store.save_checkpoint(&second).unwrap();

        let latest = store.load_latest().unwrap().unwrap();
        assert_eq!(latest.checkpoint_id, 2);
        assert_eq!(latest.last_applied_sequence, WalSequence(20));
        assert!(store.validate_checkpoint(&latest).unwrap());

        let checkpoints = store.list_checkpoints().unwrap();
        assert_eq!(checkpoints.len(), 2);
        assert_eq!(store.prune_old().unwrap(), 1);
        assert_eq!(store.list_checkpoints().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn checkpoint_store_rejects_corrupt_checkpoint() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-checkpoints-corrupt-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut store = FileExecutionCheckpointStore::open(
            CheckpointConfig::new(&root).with_sync_on_save(false),
        )
        .unwrap();
        let checkpoint = ExecutionCheckpoint::new(1, WalSequence(1), 1);
        let manifest = store.save_checkpoint(&checkpoint).unwrap();

        let mut bytes = std::fs::read(&manifest.path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x01;
        std::fs::write(&manifest.path, bytes).unwrap();

        assert!(store.load_latest().is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn checkpoint_store_inspect_root_reports_latest_and_corruption() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-checkpoints-inspect-{}",
            std::process::id()
        ));
        let missing = root.with_extension("missing");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&missing);
        assert!(FileExecutionCheckpointStore::inspect_root(&missing).is_err());

        let mut store = FileExecutionCheckpointStore::open(
            CheckpointConfig::new(&root).with_sync_on_save(false),
        )
        .unwrap();
        let first = ExecutionCheckpoint::new(1, WalSequence(10), 100);
        let second = ExecutionCheckpoint::new(2, WalSequence(20), 200);
        store.save_checkpoint(&first).unwrap();
        let manifest = store.save_checkpoint(&second).unwrap();

        let report = FileExecutionCheckpointStore::inspect_root(&root).unwrap();
        assert!(report.valid);
        assert_eq!(report.checkpoint_files, 2);
        assert_eq!(report.valid_checkpoints, 2);
        assert_eq!(report.invalid_checkpoints, 0);
        assert_eq!(report.latest_checkpoint_id, Some(2));
        assert_eq!(report.latest_last_applied_sequence, Some(WalSequence(20)));
        assert_eq!(report.latest_created_ns, Some(200));
        let latest = FileExecutionCheckpointStore::load_latest_from_root(&root)
            .unwrap()
            .unwrap();
        assert_eq!(latest.checkpoint_id, 2);

        let mut bytes = std::fs::read(&manifest.path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last ^= 0x01;
        std::fs::write(&manifest.path, bytes).unwrap();

        let report = FileExecutionCheckpointStore::inspect_root(&root).unwrap();
        assert!(!report.valid);
        assert_eq!(report.checkpoint_files, 2);
        assert_eq!(report.valid_checkpoints, 1);
        assert_eq!(report.invalid_checkpoints, 1);
        assert_eq!(report.latest_checkpoint_id, Some(1));
        assert_eq!(report.latest_last_applied_sequence, Some(WalSequence(10)));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recovery_replays_segmented_wal_after_latest_checkpoint() {
        let root =
            std::env::temp_dir().join(format!("orderflow-recovery-wal-{}", std::process::id()));
        let checkpoint_root = std::env::temp_dir().join(format!(
            "orderflow-recovery-checkpoints-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&checkpoint_root);

        let req = order("C1");
        let mut state = OrderState::pending_new(&req);
        state.status = OrderStatus::New;
        state.venue_order_id = id("V1");
        state.leaves_qty = req.quantity;
        state.updated_ns = 2;

        let mut store = FileExecutionCheckpointStore::open(
            CheckpointConfig::new(&checkpoint_root).with_sync_on_save(false),
        )
        .unwrap();
        store
            .save_checkpoint(
                &ExecutionCheckpoint::new(1, WalSequence(2), 100).with_open_orders(vec![state]),
            )
            .unwrap();

        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        journal
            .record_command(JournalCommandKind::Submit, req.client_order_id, 1)
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req, id("V1")))
            .unwrap();
        let mut cancel_ack = ExecutionEvent::accepted(&req, id("V1"));
        cancel_ack.exec_type = ExecutionType::CancelAck;
        cancel_ack.order_status = OrderStatus::Cancelled;
        cancel_ack.orig_client_order_id = req.client_order_id;
        cancel_ack.client_order_id = id("CXL1");
        cancel_ack.leaves_qty = OrderQty(0);
        cancel_ack.ts_recv_ns = 3;
        journal.record_event(&cancel_ack).unwrap();
        journal.sync().unwrap();

        let result = recover_latest_checkpoint_from_segmented_wal(&store, &journal).unwrap();
        assert_eq!(result.replay.first_sequence, Some(WalSequence(3)));
        assert_eq!(result.events_applied, 1);
        assert_eq!(result.commands_seen, 0);
        assert!(result.venue_reconciliation_required);
        assert!(!result.submissions_enabled);
        assert_eq!(result.state.orders().len(), 1);
        assert_eq!(result.state.orders()[0].status, OrderStatus::Cancelled);
        assert!(result.state.open_orders().is_empty());

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&checkpoint_root);
    }

    #[test]
    fn full_command_payload_round_trips_without_changing_legacy_replay() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-full-command-roundtrip-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let request = order("FULL1");
        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        journal.record_submit(&request).unwrap();

        let mut recovery_records = Vec::new();
        journal
            .replay_recovery_from(WalSequence(1), &mut recovery_records)
            .unwrap();
        assert_eq!(
            recovery_records,
            vec![DecodedRecoveryRecord::Command(Box::new(
                DecodedWalCommand::Submit(request)
            ))]
        );

        let mut legacy_records = Vec::new();
        journal.replay(&mut legacy_records).unwrap();
        assert_eq!(
            legacy_records,
            vec![JournalRecord::Command {
                kind: JournalCommandKind::Submit,
                client_order_id: request.client_order_id,
                ts_ns: request.ts_recv_ns,
            }]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn full_command_payload_recovers_without_checkpoint() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-full-command-recovery-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let request = order("FULL2");
        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        journal.record_submit(&request).unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&request, id("VENUE2")))
            .unwrap();

        let result =
            recover_oms_state_from_segmented_wal(RecoveryPlan::default(), None, &journal).unwrap();
        assert_eq!(result.commands_seen, 1);
        assert_eq!(result.events_applied, 1);
        assert_eq!(result.state.orders().len(), 1);
        assert_eq!(
            result.state.orders()[0].client_order_id,
            request.client_order_id
        );
        assert_eq!(result.state.orders()[0].order_qty, request.quantity);
        assert_eq!(result.state.orders()[0].status, OrderStatus::New);
        assert_eq!(result.state.orders()[0].venue_order_id, id("VENUE2"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn root_recovery_is_read_only_and_reports_marker_sequence() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-read-only-root-recovery-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let request = order("ROOT1");
        {
            let mut journal = SegmentedWalExecutionJournal::open(
                WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
            )
            .unwrap();
            journal.record_submit(&request).unwrap();
            journal
                .record_event(&ExecutionEvent::accepted(&request, id("ROOT-V")))
                .unwrap();
            journal.rotate_segment().unwrap();
        }
        let mut before = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        before.sort();

        let result =
            recover_latest_checkpoint_from_segmented_wal_roots(&root, None, false).unwrap();
        assert_eq!(result.replay.last_sequence, Some(WalSequence(3)));
        assert_eq!(result.replay.records, 2);
        assert_eq!(result.state.orders().len(), 1);
        assert_eq!(result.state.open_order_count(), 1);
        assert_eq!(
            result.json_report(),
            "{\"schema_version\":1,\"checkpoint_id\":null,\"route_config_hash\":0,\"kill_switch\":false,\"orders\":1,\"open_orders\":1,\"positions\":0,\"commands_seen\":1,\"events_applied\":1,\"replay\":{\"records\":2,\"bytes\":1450,\"first_sequence\":1,\"last_sequence\":3},\"venue_reconciliation_required\":true,\"submissions_enabled\":false}"
        );

        let mut after = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        after.sort();
        assert_eq!(before, after);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn root_recovery_requires_checkpoint_when_configured() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-required-checkpoint-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        drop(journal);
        let error =
            recover_latest_checkpoint_from_segmented_wal_roots(&root, None, true).unwrap_err();
        assert!(error.to_string().contains("requires a valid checkpoint"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn root_recovery_rejects_legacy_command_only_frames() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-legacy-root-recovery-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let request = order("LEGACY1");
        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        journal
            .record_command(
                JournalCommandKind::Submit,
                request.client_order_id,
                request.ts_recv_ns,
            )
            .unwrap();
        drop(journal);

        let error =
            recover_latest_checkpoint_from_segmented_wal_roots(&root, None, false).unwrap_err();
        assert!(error.to_string().contains("lacks the full payload"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn execution_engine_uses_full_payload_journal_hook() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-engine-full-command-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        let mut engine = ExecutionEngine::new(
            SimExecutionAdapter::default(),
            AllowAllRiskGate,
            journal,
            vec![route()],
        );
        engine.start().unwrap();
        let request = order("ENGINE-FULL");
        let mut events = ExecutionEventBuffer::with_capacity(4);
        engine.submit(request, &mut events).unwrap();
        drop(engine);

        let journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        let recovered =
            recover_oms_state_from_segmented_wal(RecoveryPlan::default(), None, &journal).unwrap();
        assert_eq!(recovered.commands_seen, 1);
        assert_eq!(recovered.events_applied, events.len());
        assert_eq!(recovered.state.orders().len(), 1);
        assert_eq!(recovered.state.orders()[0].status, OrderStatus::Filled);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn full_cancel_and_amend_payloads_recover_uncertain_pending_states() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-full-command-pending-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let cancel_order = order("CANCEL-BASE");
        let amend_order = order("AMEND-BASE");
        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        journal.record_submit(&cancel_order).unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&cancel_order, id("VC")))
            .unwrap();
        journal
            .record_cancel(&CancelRequest {
                client_order_id: id("CANCEL-REQ"),
                orig_client_order_id: cancel_order.client_order_id,
                venue_order_id: id("VC"),
                account_id: cancel_order.account_id,
                route_id: cancel_order.route_id,
                symbol: cancel_order.symbol,
                ts_recv_ns: 10,
            })
            .unwrap();
        journal.record_submit(&amend_order).unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&amend_order, id("VA")))
            .unwrap();
        journal
            .record_amend(&AmendRequest {
                client_order_id: id("AMEND-REQ"),
                orig_client_order_id: amend_order.client_order_id,
                venue_order_id: id("VA"),
                account_id: amend_order.account_id,
                route_id: amend_order.route_id,
                symbol: amend_order.symbol,
                quantity: OrderQty(8),
                limit_price: OrderPrice(4999),
                ts_recv_ns: 11,
            })
            .unwrap();

        let result =
            recover_oms_state_from_segmented_wal(RecoveryPlan::default(), None, &journal).unwrap();
        assert_eq!(result.commands_seen, 4);
        assert_eq!(result.events_applied, 2);
        assert_eq!(result.state.orders()[0].status, OrderStatus::PendingCancel);
        assert_eq!(result.state.orders()[0].updated_ns, 10);
        assert_eq!(result.state.orders()[1].status, OrderStatus::PendingReplace);
        assert_eq!(result.state.orders()[1].updated_ns, 11);
        assert!(result.venue_reconciliation_required);
        assert!(!result.submissions_enabled);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recovery_is_deterministic_for_same_checkpoint_and_wal() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-recovery-deterministic-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);

        let req = order("C1");
        let mut state = OrderState::pending_new(&req);
        state.status = OrderStatus::New;
        state.venue_order_id = id("V1");
        let checkpoint =
            ExecutionCheckpoint::new(1, WalSequence(2), 100).with_open_orders(vec![state]);

        let mut journal = SegmentedWalExecutionJournal::open(
            WalSegmentConfig::new(&root).with_sync_policy(WalSyncPolicy::Never),
        )
        .unwrap();
        journal
            .record_command(JournalCommandKind::Submit, req.client_order_id, 1)
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req, id("V1")))
            .unwrap();
        let mut fill = ExecutionEvent::accepted(&req, id("V1"));
        fill.exec_type = ExecutionType::Trade;
        fill.order_status = OrderStatus::Filled;
        fill.last_qty = req.quantity;
        fill.last_price = req.limit_price;
        fill.cumulative_qty = req.quantity;
        fill.leaves_qty = OrderQty(0);
        fill.average_price = req.limit_price;
        fill.ts_recv_ns = 3;
        journal.record_event(&fill).unwrap();

        let plan = RecoveryPlan::from_checkpoint(&checkpoint);
        let first = recover_oms_state_from_segmented_wal(plan.clone(), Some(&checkpoint), &journal)
            .unwrap();
        let second =
            recover_oms_state_from_segmented_wal(plan, Some(&checkpoint), &journal).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.state.orders()[0].status, OrderStatus::Filled);
        assert_eq!(first.state.orders()[0].average_price, req.limit_price);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recovery_fails_closed_on_unknown_order_event() {
        let req = order("C1");
        let record = JournalRecord::Event(Box::new(ExecutionEvent::accepted(&req, id("V1"))));
        let err =
            recover_oms_state_from_records(RecoveryPlan::default(), None, &[record]).unwrap_err();
        assert!(err.to_string().contains("unknown order"));
    }

    #[test]
    fn recovery_readiness_allows_clean_host_controlled_resume() {
        let plan = RecoveryPlan::new(WalSequence(1))
            .with_venue_policy(RecoveryVenuePolicy::HostControlled)
            .with_submissions_disabled(false);
        let recovery = recover_oms_state_from_records(plan, None, &[]).unwrap();
        let wal = WalSegmentIntegrityReport {
            valid: true,
            last_sequence: Some(WalSequence(10)),
            ..WalSegmentIntegrityReport::default()
        };
        let checkpoint_store = CheckpointStoreIntegrityReport {
            valid: true,
            latest_checkpoint_id: Some(3),
            latest_last_applied_sequence: Some(WalSequence(10)),
            ..CheckpointStoreIntegrityReport::default()
        };
        let reconciliation_report = reconcile_open_orders_detailed(&[], &[]);
        let reconciliation =
            evaluate_reconciliation_policy(&reconciliation_report, ReconciliationPolicy::default());

        let decision = evaluate_recovery_readiness(
            &recovery,
            &wal,
            &checkpoint_store,
            Some(&reconciliation),
            RecoveryReadinessConfig::strict(),
        );

        assert!(decision.is_ready());
        assert!(decision.submissions_enabled);
        assert!(!decision.fail_closed);
        assert_eq!(decision.latest_recovered_sequence, Some(WalSequence(10)));
        assert_eq!(decision.reconciliation_items, 0);
        assert!(decision.blockers.is_empty());
    }

    #[test]
    fn recovery_readiness_blocks_corrupt_unreconciled_restart() {
        let recovery = recover_oms_state_from_records(RecoveryPlan::default(), None, &[]).unwrap();
        let wal = WalSegmentIntegrityReport {
            checksum_failures: 1,
            valid: false,
            last_sequence: Some(WalSequence(7)),
            ..WalSegmentIntegrityReport::default()
        };
        let checkpoint_store = CheckpointStoreIntegrityReport {
            invalid_checkpoints: 1,
            valid: false,
            ..CheckpointStoreIntegrityReport::default()
        };

        let decision = evaluate_recovery_readiness(
            &recovery,
            &wal,
            &checkpoint_store,
            None,
            RecoveryReadinessConfig::strict(),
        );

        assert!(!decision.is_ready());
        assert!(decision.fail_closed);
        assert!(decision.operator_attention_required);
        assert!(decision.has_blocker(RecoveryReadinessBlocker::WalIntegrityFailed));
        assert!(decision.has_blocker(RecoveryReadinessBlocker::CheckpointMissing));
        assert!(decision.has_blocker(RecoveryReadinessBlocker::CheckpointIntegrityFailed));
        assert!(decision.has_blocker(RecoveryReadinessBlocker::RecoveryDidNotReachLatestWal));
        assert!(decision.has_blocker(RecoveryReadinessBlocker::RecoverySubmissionsDisabled));
        assert!(decision.has_blocker(RecoveryReadinessBlocker::VenueReconciliationMissing));
    }

    #[test]
    fn recovery_readiness_reports_reconciliation_actions() {
        let plan = RecoveryPlan::new(WalSequence(1))
            .with_venue_policy(RecoveryVenuePolicy::HostControlled)
            .with_submissions_disabled(false);
        let recovery = recover_oms_state_from_records(plan, None, &[]).unwrap();
        let wal = WalSegmentIntegrityReport {
            valid: true,
            last_sequence: Some(WalSequence(1)),
            ..WalSegmentIntegrityReport::default()
        };
        let checkpoint_store = CheckpointStoreIntegrityReport {
            valid: true,
            latest_checkpoint_id: Some(1),
            latest_last_applied_sequence: Some(WalSequence(1)),
            ..CheckpointStoreIntegrityReport::default()
        };
        let req = order("C1");
        let venue_state = OrderState::pending_new(&req);
        let reconciliation_report = reconcile_open_orders_detailed(&[], &[venue_state]);
        let reconciliation = evaluate_reconciliation_policy(
            &reconciliation_report,
            ReconciliationPolicy::fail_closed()
                .with_venue_only(ReconciliationPolicyAction::CancelVenueOrder),
        );

        let decision = evaluate_recovery_readiness(
            &recovery,
            &wal,
            &checkpoint_store,
            Some(&reconciliation),
            RecoveryReadinessConfig::strict(),
        );

        assert!(!decision.submissions_enabled);
        assert!(decision.has_blocker(RecoveryReadinessBlocker::ReconciliationPolicyBlocks));
        assert!(decision.has_blocker(RecoveryReadinessBlocker::VenueCancelsRequired));
        assert_eq!(decision.reconciliation_items, 1);
    }

    #[test]
    fn reconciliation_detects_venue_only_state() {
        let req = order("C1");
        let state = OrderState::pending_new(&req);
        let report = reconcile_open_orders(&[], &[state]);
        assert_eq!(report.items.len(), 1);
        assert_eq!(report.items[0].action, ReconciliationAction::VenueOnly);
    }

    #[test]
    fn detailed_reconciliation_classifies_mismatches() {
        let req = order("C1");
        let local = OrderState::pending_new(&req);
        let mut venue = local;
        venue.status = OrderStatus::New;

        let report = reconcile_open_orders_detailed(&[local], &[venue]);
        assert_eq!(report.details.len(), 1);
        assert_eq!(
            report.details[0].issue,
            ReconciliationIssueKind::StatusMismatch
        );
        assert!(report.has_discrepancies());

        let mut venue = local;
        venue.leaves_qty = OrderQty(5);
        let report = reconcile_open_orders_detailed(&[local], &[venue]);
        assert_eq!(
            report.details[0].issue,
            ReconciliationIssueKind::QuantityMismatch
        );

        let mut venue = local;
        venue.average_price = OrderPrice(5000);
        let report = reconcile_open_orders_detailed(&[local], &[venue]);
        assert_eq!(
            report.details[0].issue,
            ReconciliationIssueKind::PriceMismatch
        );
    }

    #[test]
    fn reconciliation_policy_blocks_until_host_action_completes() {
        let req = order("C1");
        let state = OrderState::pending_new(&req);
        let report = reconcile_open_orders_detailed(&[], &[state]);
        let policy = ReconciliationPolicy::fail_closed()
            .with_venue_only(ReconciliationPolicyAction::CancelVenueOrder);

        let decision = evaluate_reconciliation_policy(&report, policy);
        assert!(!decision.submissions_enabled);
        assert!(decision.venue_cancels_required);
        assert!(!decision.fail_closed);
        assert_eq!(
            decision.items[0].action,
            ReconciliationPolicyAction::CancelVenueOrder
        );

        let approval = evaluate_reconciliation_policy(
            &report,
            ReconciliationPolicy::require_operator_approval(),
        );
        assert!(approval.operator_approval_required);
        assert!(!approval.submissions_enabled);
    }

    #[test]
    fn proportional_allocation_balances_with_largest_remainder() {
        let group = AllocationGroup::new(
            AllocationMethod::Proportional,
            vec![
                AllocationLeg::proportional(id("A1"), id("R1"), id("S1"), 1),
                AllocationLeg::proportional(id("A2"), id("R1"), id("S1"), 1),
                AllocationLeg::proportional(id("A3"), id("R1"), id("S1"), 1),
            ],
        );
        let report = allocate_block_fill(&group, OrderQty(10), OrderPrice(5000)).unwrap();

        assert!(report.balanced);
        assert_eq!(report.allocated_qty(), OrderQty(10));
        assert_eq!(report.fills[0].quantity, OrderQty(4));
        assert_eq!(report.fills[1].quantity, OrderQty(3));
        assert_eq!(report.fills[2].quantity, OrderQty(3));
        assert_eq!(report.fills[0].average_price, OrderPrice(5000));
    }

    #[test]
    fn priority_allocation_respects_targets() {
        let group = AllocationGroup::new(
            AllocationMethod::Priority,
            vec![
                AllocationLeg::priority(id("A1"), id("R1"), id("S1"), OrderQty(3), 2),
                AllocationLeg::priority(id("A2"), id("R1"), id("S1"), OrderQty(5), 1),
                AllocationLeg::priority(id("A3"), id("R1"), id("S1"), OrderQty(0), 3),
            ],
        );
        let report = allocate_block_fill(&group, OrderQty(10), OrderPrice(5000)).unwrap();

        assert!(report.balanced);
        assert_eq!(report.fills[0].quantity, OrderQty(3));
        assert_eq!(report.fills[1].quantity, OrderQty(5));
        assert_eq!(report.fills[2].quantity, OrderQty(2));
    }

    #[test]
    fn allocation_rejects_invalid_groups() {
        let empty = AllocationGroup::new(AllocationMethod::Proportional, Vec::new());
        assert_eq!(
            allocate_block_fill(&empty, OrderQty(1), OrderPrice(1)).unwrap_err(),
            AllocationError::EmptyGroup
        );

        let zero_weight = AllocationGroup::new(
            AllocationMethod::Proportional,
            vec![AllocationLeg::proportional(id("A1"), id("R1"), id("S1"), 0)],
        );
        assert_eq!(
            allocate_block_fill(&zero_weight, OrderQty(1), OrderPrice(1)).unwrap_err(),
            AllocationError::ZeroTotalWeight
        );
    }

    #[test]
    fn allocation_reconciliation_classifies_mismatches() {
        let expected = AllocationFill {
            account_id: id("A1"),
            route_id: id("R1"),
            strategy_id: id("S1"),
            quantity: OrderQty(10),
            average_price: OrderPrice(5000),
        };
        let actual = AllocationFill {
            quantity: OrderQty(9),
            ..expected
        };

        let report = reconcile_allocations(&[expected], &[actual]);
        assert!(!report.is_clean());
        assert_eq!(
            report.details[0].issue,
            AllocationReconciliationIssue::QuantityMismatch
        );

        let report = reconcile_allocations(&[expected], &[]);
        assert_eq!(
            report.details[0].issue,
            AllocationReconciliationIssue::MissingActual
        );
    }

    #[test]
    fn timestamp_trace_attributes_latency() {
        let trace = ExecutionTimestampTrace::new()
            .with_strategy_decision(100, TimestampSource::MonotonicClock)
            .with_oms_receive(110, TimestampSource::SystemClock)
            .with_wal_append(120, TimestampSource::Journal)
            .with_adapter_send(140, TimestampSource::SystemClock)
            .with_exchange(170, TimestampSource::Venue)
            .with_oms_receive_report(210, TimestampSource::SystemClock)
            .with_drop_copy_receive(230, TimestampSource::DropCopy)
            .with_checkpoint(300, TimestampSource::Checkpoint);

        let attribution = trace.latency_attribution();
        assert_eq!(attribution.strategy_to_oms_ns, Some(10));
        assert_eq!(attribution.oms_to_wal_append_ns, Some(10));
        assert_eq!(attribution.wal_to_adapter_send_ns, Some(20));
        assert_eq!(attribution.adapter_to_exchange_ns, Some(30));
        assert_eq!(attribution.exchange_to_oms_report_ns, Some(40));
        assert_eq!(attribution.oms_report_to_drop_copy_ns, Some(20));
        assert_eq!(attribution.report_to_checkpoint_ns, Some(90));
        assert_eq!(attribution.end_to_end_ns, Some(200));
    }

    #[test]
    fn timestamp_trace_detects_non_monotonic_internal_time() {
        let trace = ExecutionTimestampTrace::new()
            .with_strategy_decision(100, TimestampSource::MonotonicClock)
            .with_oms_receive(90, TimestampSource::SystemClock);

        let report = trace.validate(TimestampDisciplineConfig::default());
        assert!(!report.is_clean());
        assert_eq!(report.issue_count, 1);
        assert_eq!(
            report.issues[0].kind,
            TimestampDisciplineIssueKind::NonMonotonic
        );
    }

    #[test]
    fn timestamp_trace_detects_exchange_skew() {
        let trace = ExecutionTimestampTrace::new()
            .with_exchange(1_000, TimestampSource::Venue)
            .with_oms_receive_report(5_000, TimestampSource::SystemClock);
        let report = trace.validate(TimestampDisciplineConfig {
            max_clock_skew_ns: 100,
            ..TimestampDisciplineConfig::default()
        });

        assert_eq!(report.max_clock_skew_ns, 4_000);
        assert_eq!(
            report.issues[0].kind,
            TimestampDisciplineIssueKind::ClockSkewExceeded
        );
    }

    #[test]
    fn timestamp_trace_observes_execution_event() {
        let req = order("C1");
        let mut event = ExecutionEvent::accepted(&req, id("V1"));
        event.ts_exchange_ns = 50;
        event.ts_recv_ns = 75;

        let trace = ExecutionTimestampTrace::from_order_request(&req).observe_event(&event);

        assert_eq!(trace.exchange_ns, 50);
        assert_eq!(trace.oms_receive_report_ns, 75);
        assert_eq!(trace.sources.exchange, TimestampSource::Venue);
        assert_eq!(
            trace.sources.oms_receive_report,
            TimestampSource::SystemClock
        );
    }

    #[test]
    fn safety_policy_defaults_to_fail_closed_for_new_orders() {
        let context = SafetyContext {
            risk_unavailable: true,
            ..SafetyContext::default()
        };

        let decision = evaluate_safety_policy(context, SafetyPolicy::default());

        assert!(!decision.submissions_enabled);
        assert!(decision.cancels_enabled);
        assert!(decision.fail_closed);
        assert!(!decision.degraded);
        assert_eq!(decision.items.len(), 1);
        assert_eq!(
            decision.items[0].condition,
            SafetyCondition::RiskUnavailable
        );
        assert_eq!(decision.items[0].action, SafetyPolicyAction::RejectNew);
    }

    #[test]
    fn safety_policy_reports_fail_open_degradation() {
        let context = SafetyContext {
            market_data_stale: true,
            route_health_degraded: true,
            ..SafetyContext::default()
        };

        let decision = evaluate_safety_policy(context, SafetyPolicy::fail_open_degraded());

        assert!(decision.submissions_enabled);
        assert!(decision.cancels_enabled);
        assert!(!decision.fail_closed);
        assert!(decision.degraded);
        assert_eq!(decision.fail_open_count, 2);
    }

    #[test]
    fn safety_policy_reject_all_blocks_cancels() {
        let context = SafetyContext {
            oms_wal_degraded: true,
            ..SafetyContext::default()
        };
        let policy = SafetyPolicy::default().with_action(
            SafetyCondition::OmsWalDegraded,
            SafetyPolicyAction::RejectAll,
        );

        let decision = evaluate_safety_policy(context, policy);

        assert!(!decision.submissions_enabled);
        assert!(!decision.cancels_enabled);
        assert!(decision.fail_closed);
        assert_eq!(decision.items[0].action, SafetyPolicyAction::RejectAll);
    }

    #[test]
    fn throttle_refills_over_time() {
        let mut throttle = OrderThrottle::new(1, 1);
        assert!(throttle.allow(1));
        assert!(!throttle.allow(2));
        assert!(throttle.allow(1_000_000_002));
    }

    #[test]
    fn replay_simulation_is_deterministic() {
        let decisions = [ReplayDecision {
            ts_recv_ns: 2,
            command: ExecutionCommand::Submit(order("C1")),
        }];
        let result = replay_simulated_oms(vec![route()], &decisions).unwrap();
        assert_eq!(result.reports.len(), 1);
        assert_eq!(result.reports[0].events.len(), 2);
        assert_eq!(result.metrics.submitted, 1);
    }

    #[test]
    fn normalize_rejects_unsupported_tif() {
        let caps = VenueOrderCapabilities {
            market: true,
            limit: true,
            stop: false,
            stop_limit: false,
            tif_day: true,
            tif_gtc: false,
            tif_ioc: false,
            tif_fok: false,
            tif_gtd: false,
        };
        assert_eq!(
            normalize_order_type(OrderType::Limit, TimeInForce::Gtc, caps).unwrap_err(),
            RiskRejectReason::UnsupportedTimeInForce
        );
    }
}
