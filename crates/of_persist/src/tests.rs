#[cfg(test)]
mod tests {
    use std::fs;

    use of_core::{BookAction, BookUpdate, Side, SymbolId};

    use super::*;

    #[test]
    fn prunes_by_total_size() {
        let root = temp_dir("persist_prune_size");
        let store = RollingStore::new(&root)
            .expect("store")
            .with_retention(Some(RetentionPolicy {
                max_total_bytes: 150,
                max_age_secs: 0,
            }));

        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        for seq in 0..20 {
            store
                .append_book(&BookUpdate {
                    symbol: symbol.clone(),
                    side: Side::Bid,
                    level: 0,
                    price: 100,
                    size: 1,
                    action: BookAction::Upsert,
                    sequence: seq,
                    ts_exchange_ns: 0,
                    ts_recv_ns: 0,
                })
                .expect("append");
        }

        let mut files = Vec::new();
        collect_files(&root, &mut files).expect("collect");
        let total: u64 = files.iter().map(|f| f.len).sum();
        assert!(total <= 150);
    }

    #[test]
    fn prunes_by_age() {
        let root = temp_dir("persist_prune_age");
        let old_path = root.join("old.jsonl");
        fs::write(&old_path, b"old").expect("write old");
        std::thread::sleep(std::time::Duration::from_millis(2200));

        let store = RollingStore::new(&root)
            .expect("store")
            .with_retention(Some(RetentionPolicy {
                max_total_bytes: 0,
                max_age_secs: 1,
            }));

        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_book(&BookUpdate {
                symbol,
                side: Side::Bid,
                level: 0,
                price: 100,
                size: 1,
                action: BookAction::Upsert,
                sequence: 1,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append");

        assert!(!old_path.exists());
    }

    #[test]
    fn reads_back_appended_book_and_trade_streams() {
        let root = temp_dir("persist_readback");
        let store = RollingStore::new(&root).expect("store");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_book(&BookUpdate {
                symbol: symbol.clone(),
                side: Side::Bid,
                level: 1,
                price: 505_000,
                size: 7,
                action: BookAction::Upsert,
                sequence: 10,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append book");
        store
            .append_trade(&TradePrint {
                symbol: symbol.clone(),
                price: 505_025,
                size: 3,
                aggressor_side: Side::Ask,
                sequence: 11,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append trade");

        let books = store
            .read_books(&symbol.venue, &symbol.symbol)
            .expect("read books");
        let trades = store
            .read_trades(&symbol.venue, &symbol.symbol)
            .expect("read trades");

        assert_eq!(
            books,
            vec![StoredBookEvent {
                sequence: 10,
                side: Side::Bid,
                level: 1,
                price: 505_000,
                size: 7,
                action: BookAction::Upsert,
            }]
        );
        assert_eq!(
            trades,
            vec![StoredTradeEvent {
                sequence: 11,
                price: 505_025,
                size: 3,
                aggressor_side: Side::Ask,
            }]
        );
    }

    #[test]
    fn writes_schema_metadata_without_breaking_readback() {
        let root = temp_dir("persist_schema_metadata");
        let store = RollingStore::new(&root).expect("store");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_trade(&TradePrint {
                symbol: symbol.clone(),
                price: 505_025,
                size: 3,
                aggressor_side: Side::Ask,
                sequence: 11,
                ts_exchange_ns: 123,
                ts_recv_ns: 456,
            })
            .expect("append trade");

        let raw = fs::read_to_string(root.join("CME").join("ESM6").join("trades.jsonl"))
            .expect("read raw stream");
        assert!(raw.contains("\"schema\":1"));
        assert!(raw.contains("\"ts_exchange_ns\":123"));
        assert!(raw.contains("\"ts_recv_ns\":456"));

        let trades = store
            .read_trades(&symbol.venue, &symbol.symbol)
            .expect("read trades");
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].sequence, 11);
    }

    #[test]
    fn reads_legacy_records_without_schema_metadata() {
        let root = temp_dir("persist_legacy_schema");
        let stream_dir = root.join("CME").join("ESM6");
        fs::create_dir_all(&stream_dir).expect("create dir");
        fs::write(
            stream_dir.join("trades.jsonl"),
            b"{\"seq\":11,\"price\":505025,\"size\":3,\"aggressor\":\"Ask\"}\n",
        )
        .expect("write legacy trade");

        let store = RollingStore::new(&root).expect("store");
        let trades = store
            .read_trades("CME", "ESM6")
            .expect("read legacy trades");

        assert_eq!(
            trades,
            vec![StoredTradeEvent {
                sequence: 11,
                price: 505_025,
                size: 3,
                aggressor_side: Side::Ask,
            }]
        );
    }

    #[test]
    fn missing_stream_reads_back_as_empty() {
        let root = temp_dir("persist_missing_stream");
        let store = RollingStore::new(&root).expect("store");

        let books = store.read_books("CME", "ESM6").expect("read books");
        let trades = store.read_trades("CME", "ESM6").expect("read trades");

        assert!(books.is_empty());
        assert!(trades.is_empty());
    }

    #[test]
    fn invalid_stream_data_returns_invalid_data_error() {
        let root = temp_dir("persist_invalid_stream");
        let stream_dir = root.join("CME").join("ESM6");
        fs::create_dir_all(&stream_dir).expect("create dir");
        fs::write(
            stream_dir.join("book.jsonl"),
            b"{\"seq\":1,\"side\":\"Middle\",\"level\":0,\"price\":1,\"size\":1,\"action\":\"Upsert\"}\n",
        )
        .expect("write");

        let store = RollingStore::new(&root).expect("store");
        let err = store.read_books("CME", "ESM6").expect_err("invalid data");

        match err {
            PersistError::Io(inner) => assert_eq!(inner.kind(), std::io::ErrorKind::InvalidData),
        }
    }

    #[test]
    fn reads_merged_symbol_events_in_sequence_order() {
        let root = temp_dir("persist_merged_readback");
        let store = RollingStore::new(&root).expect("store");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_trade(&TradePrint {
                symbol: symbol.clone(),
                price: 505_050,
                size: 2,
                aggressor_side: Side::Ask,
                sequence: 12,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append trade");
        store
            .append_book(&BookUpdate {
                symbol: symbol.clone(),
                side: Side::Bid,
                level: 0,
                price: 505_000,
                size: 10,
                action: BookAction::Upsert,
                sequence: 10,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append book");
        store
            .append_book(&BookUpdate {
                symbol: symbol.clone(),
                side: Side::Ask,
                level: 0,
                price: 505_075,
                size: 9,
                action: BookAction::Upsert,
                sequence: 12,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append book");

        let events = store
            .read_events(&symbol.venue, &symbol.symbol)
            .expect("read events");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].sequence(), 10);
        assert_eq!(events[1].sequence(), 12);
        assert_eq!(events[2].sequence(), 12);
        assert!(matches!(events[1], StoredEvent::Book(_)));
        assert!(matches!(events[2], StoredEvent::Trade(_)));
    }

    #[test]
    fn lists_venues_and_symbols_in_sorted_order() {
        let root = temp_dir("persist_discovery");
        fs::create_dir_all(root.join("BINANCE").join("BTCUSDT")).expect("btc dir");
        fs::create_dir_all(root.join("CME").join("NQM6")).expect("nq dir");
        fs::create_dir_all(root.join("CME").join("ESM6")).expect("es dir");

        let store = RollingStore::new(&root).expect("store");

        let venues = store.list_venues().expect("venues");
        let symbols = store.list_symbols("CME").expect("symbols");
        let missing = store.list_symbols("ICE").expect("missing");

        assert_eq!(venues, vec!["BINANCE".to_string(), "CME".to_string()]);
        assert_eq!(symbols, vec!["ESM6".to_string(), "NQM6".to_string()]);
        assert!(missing.is_empty());
    }

    #[test]
    fn lists_symbol_streams_without_suffixes() {
        let root = temp_dir("persist_stream_discovery");
        let stream_dir = root.join("CME").join("ESM6");
        fs::create_dir_all(&stream_dir).expect("stream dir");
        fs::write(stream_dir.join("book.jsonl"), b"{}\n").expect("write book");
        fs::write(stream_dir.join("trades.jsonl"), b"{}\n").expect("write trades");
        fs::write(stream_dir.join("notes.txt"), b"ignore").expect("write notes");

        let store = RollingStore::new(&root).expect("store");
        let streams = store.list_streams("CME", "ESM6").expect("streams");
        let missing = store.list_streams("CME", "NQM6").expect("missing");

        assert_eq!(streams, vec!["book".to_string(), "trades".to_string()]);
        assert!(missing.is_empty());
    }

    #[test]
    fn reads_range_filtered_events_inclusively() {
        let root = temp_dir("persist_range_filter");
        let store = RollingStore::new(&root).expect("store");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        for sequence in [10_u64, 11, 12] {
            store
                .append_trade(&TradePrint {
                    symbol: symbol.clone(),
                    price: 505000 + (sequence as i64),
                    size: 1,
                    aggressor_side: Side::Ask,
                    sequence,
                    ts_exchange_ns: 0,
                    ts_recv_ns: 0,
                })
                .expect("append trade");
        }

        let trades = store
            .read_trades_in_range(&symbol.venue, &symbol.symbol, Some(11), Some(12))
            .expect("trades in range");
        let events = store
            .read_events_in_range(&symbol.venue, &symbol.symbol, Some(10), Some(11))
            .expect("events in range");

        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].sequence, 11);
        assert_eq!(trades[1].sequence, 12);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence(), 10);
        assert_eq!(events[1].sequence(), 11);
    }

    #[test]
    fn market_data_wal_appends_and_replays_records() {
        let root = temp_dir("persist_market_data_wal");
        let path = root.join("normalized.wal");
        let mut wal = MarketDataWal::open(MarketDataWalConfig::new(&path)).expect("open wal");

        let first = wal
            .append_record(
                MarketDataWalRecordKind::TradePrint,
                10,
                20,
                30,
                40,
                b"trade",
            )
            .expect("append first");
        let second = wal
            .append_record(MarketDataWalRecordKind::BookUpdate, 11, 21, 31, 41, b"book")
            .expect("append second");

        assert_eq!(first, MarketDataWalSequence(1));
        assert_eq!(second, MarketDataWalSequence(2));
        assert_eq!(wal.metrics().records_written, 2);

        let mut records = Vec::new();
        let replay = wal.replay(&mut records).expect("replay");
        assert_eq!(replay.records, 2);
        assert_eq!(replay.first_sequence, Some(MarketDataWalSequence(1)));
        assert_eq!(replay.last_sequence, Some(MarketDataWalSequence(2)));
        assert_eq!(records[0].kind, MarketDataWalRecordKind::TradePrint);
        assert_eq!(records[0].payload, b"trade");
        assert_eq!(records[1].kind, MarketDataWalRecordKind::BookUpdate);
        assert_eq!(records[1].payload, b"book");
    }

    #[test]
    fn market_data_wal_replays_filtered_records() {
        let root = temp_dir("persist_market_data_wal_filtered");
        let path = root.join("normalized.wal");
        let mut wal = MarketDataWal::open(MarketDataWalConfig::new(&path)).expect("open wal");
        wal.append_record(
            MarketDataWalRecordKind::TradePrint,
            10,
            100,
            1_000,
            2_000,
            b"t1",
        )
        .expect("append first");
        wal.append_record(
            MarketDataWalRecordKind::BookUpdate,
            11,
            101,
            1_100,
            2_100,
            b"b1",
        )
        .expect("append second");
        wal.append_record(
            MarketDataWalRecordKind::TradePrint,
            12,
            102,
            1_200,
            2_200,
            b"t2",
        )
        .expect("append third");

        let filter = MarketDataWalReplayFilter::new()
            .with_sequence_range(
                Some(MarketDataWalSequence(2)),
                Some(MarketDataWalSequence(3)),
            )
            .with_kind(Some(MarketDataWalRecordKind::TradePrint));
        let mut records = Vec::new();
        let replay = wal
            .replay_filtered(filter, &mut records)
            .expect("filtered replay");

        assert_eq!(replay.records, 1);
        assert_eq!(records[0].sequence, MarketDataWalSequence(3));
        assert_eq!(records[0].payload, b"t2");
    }

    #[test]
    fn market_data_wal_replay_filter_matches_provider_and_time_ranges() {
        let root = temp_dir("persist_market_data_wal_filtered_ranges");
        let path = root.join("normalized.wal");
        let mut wal = MarketDataWal::open(MarketDataWalConfig::new(&path)).expect("open wal");
        wal.append_record(
            MarketDataWalRecordKind::TradePrint,
            20,
            200,
            5_000,
            6_000,
            b"t1",
        )
        .expect("append first");
        wal.append_record(
            MarketDataWalRecordKind::TradePrint,
            21,
            201,
            5_100,
            6_100,
            b"t2",
        )
        .expect("append second");
        wal.append_record(
            MarketDataWalRecordKind::TradePrint,
            22,
            202,
            5_200,
            6_200,
            b"t3",
        )
        .expect("append third");

        let filter = MarketDataWalReplayFilter::new()
            .with_provider_sequence_range(Some(21), Some(22))
            .with_event_sequence_range(Some(201), Some(202))
            .with_exchange_time_range(Some(5_100), Some(5_200))
            .with_receive_time_range(Some(6_100), Some(6_100));
        let mut records = Vec::new();
        let replay = wal
            .replay_filtered(filter, &mut records)
            .expect("filtered replay");

        assert_eq!(replay.records, 1);
        assert_eq!(records[0].provider_sequence, 21);
        assert_eq!(records[0].ts_recv_ns, 6_100);
    }

    #[test]
    fn market_data_jsonl_export_writer_exports_records() {
        let root = temp_dir("persist_market_data_jsonl_export");
        let writer = FileMarketDataJsonlExportWriter::open(MarketDataJsonlExportConfig::new(&root))
            .expect("open export writer");
        let records = vec![
            MarketDataWalRecord {
                sequence: MarketDataWalSequence(1),
                kind: MarketDataWalRecordKind::TradePrint,
                provider_sequence: 10,
                event_sequence: 100,
                ts_exchange_ns: 1_000,
                ts_recv_ns: 2_000,
                payload: b"trade".to_vec(),
            },
            MarketDataWalRecord {
                sequence: MarketDataWalSequence(2),
                kind: MarketDataWalRecordKind::BookUpdate,
                provider_sequence: 11,
                event_sequence: 101,
                ts_exchange_ns: 1_100,
                ts_recv_ns: 2_100,
                payload: b"book".to_vec(),
            },
        ];

        let partition = writer
            .export_records("CME", "ESZ6", "market-data", &records)
            .expect("export records");
        let manifest = MarketDataColdExportManifest::from_partitions(
            MarketDataColdExportFormat::JsonLines,
            vec![partition.clone()],
        );
        let exported = fs::read_to_string(&partition.path).expect("read export");

        assert_eq!(partition.records, 2);
        assert_eq!(partition.first_sequence, Some(MarketDataWalSequence(1)));
        assert_eq!(partition.last_sequence, Some(MarketDataWalSequence(2)));
        assert_eq!(partition.first_ts_exchange_ns, Some(1_000));
        assert_eq!(partition.last_ts_exchange_ns, Some(1_100));
        assert!(partition.bytes > 0);
        assert_eq!(manifest.total_records, 2);
        assert_eq!(manifest.total_bytes, partition.bytes);
        assert!(exported.contains("\"kind\":\"TradePrint\""));
        assert!(exported.contains("\"payload_hex\":\"7472616465\""));
        assert!(exported.contains("\"payload_hex\":\"626f6f6b\""));
    }

    #[test]
    fn market_data_jsonl_export_writer_exports_filtered_wal() {
        let root = temp_dir("persist_market_data_jsonl_export_wal");
        let wal_path = root.join("normalized.wal");
        let export_root = root.join("cold");
        let mut wal = MarketDataWal::open(MarketDataWalConfig::new(&wal_path)).expect("open wal");
        wal.append_record(MarketDataWalRecordKind::TradePrint, 1, 10, 100, 200, b"t1")
            .expect("append first");
        wal.append_record(MarketDataWalRecordKind::BookUpdate, 2, 11, 101, 201, b"b1")
            .expect("append second");

        let writer =
            FileMarketDataJsonlExportWriter::open(MarketDataJsonlExportConfig::new(&export_root))
                .expect("open export writer");
        let partition = writer
            .export_wal(
                "CME",
                "NQZ6",
                "trades",
                &wal,
                MarketDataWalReplayFilter::new()
                    .with_kind(Some(MarketDataWalRecordKind::TradePrint)),
            )
            .expect("export wal");
        let exported = fs::read_to_string(&partition.path).expect("read export");

        assert_eq!(partition.records, 1);
        assert_eq!(partition.first_sequence, Some(MarketDataWalSequence(1)));
        assert!(exported.contains("\"kind\":\"TradePrint\""));
        assert!(!exported.contains("\"kind\":\"BookUpdate\""));
    }

    #[test]
    fn market_data_retention_preserves_incident_window() {
        let input = MarketDataRetentionInput::new(
            Some(MarketDataWalSequence(1)),
            Some(MarketDataWalSequence(10)),
            0,
            10_000,
        )
        .with_incident_window(true)
        .with_cold_export_verified(true);

        let decision =
            plan_market_data_retention(MarketDataRetentionPolicy::conservative(), 10_000, &input);

        assert!(!decision.may_delete_hot_wal);
        assert!(decision
            .actions
            .contains(&MarketDataRetentionAction::PreserveIncidentWindow));
        assert!(decision
            .reasons
            .contains(&MarketDataRetentionReason::IncidentWindow));
    }

    #[test]
    fn market_data_retention_keeps_wal_inside_hot_window() {
        let policy = MarketDataRetentionPolicy::conservative().with_hot_retention_ns(1_000);
        let input = MarketDataRetentionInput::new(
            Some(MarketDataWalSequence(1)),
            Some(MarketDataWalSequence(10)),
            9_500,
            10,
        );

        let decision = plan_market_data_retention(policy, 10_000, &input);

        assert_eq!(
            decision.actions,
            vec![MarketDataRetentionAction::RetainHotWal]
        );
        assert_eq!(
            decision.reasons,
            vec![MarketDataRetentionReason::WithinHotWindow]
        );
    }

    #[test]
    fn market_data_retention_exports_before_deleting_unverified_wal() {
        let policy = MarketDataRetentionPolicy::conservative().with_hot_retention_ns(1_000);
        let input = MarketDataRetentionInput::new(
            Some(MarketDataWalSequence(1)),
            Some(MarketDataWalSequence(10)),
            0,
            10,
        );

        let decision = plan_market_data_retention(policy, 10_000, &input);

        assert!(decision.should_export_cold);
        assert!(!decision.may_delete_hot_wal);
        assert_eq!(
            decision.actions,
            vec![
                MarketDataRetentionAction::ExportCold,
                MarketDataRetentionAction::RetainHotWal,
            ]
        );
    }

    #[test]
    fn market_data_retention_keeps_wal_with_checkpoint_dependency() {
        let policy = MarketDataRetentionPolicy::conservative().with_max_hot_bytes(1);
        let input = MarketDataRetentionInput::new(
            Some(MarketDataWalSequence(1)),
            Some(MarketDataWalSequence(10)),
            0,
            10,
        )
        .with_cold_export_verified(true)
        .with_dependent_checkpoint_sequence(Some(MarketDataWalSequence(10)));

        let decision = plan_market_data_retention(policy, 10_000, &input);

        assert!(!decision.may_delete_hot_wal);
        assert!(decision
            .actions
            .contains(&MarketDataRetentionAction::RetainCheckpoint));
        assert!(decision
            .reasons
            .contains(&MarketDataRetentionReason::CheckpointDependsOnWal));
    }

    #[test]
    fn market_data_retention_deletes_verified_exported_wal() {
        let policy = MarketDataRetentionPolicy::conservative()
            .with_hot_retention_ns(1_000)
            .with_min_checkpoints_retained(2);
        let input = MarketDataRetentionInput::new(
            Some(MarketDataWalSequence(1)),
            Some(MarketDataWalSequence(10)),
            0,
            10,
        )
        .with_cold_export_verified(true)
        .with_retained_checkpoints(3);

        let decision = plan_market_data_retention(policy, 10_000, &input);

        assert!(decision.may_delete_hot_wal);
        assert_eq!(
            decision.actions,
            vec![
                MarketDataRetentionAction::DeleteHotWal,
                MarketDataRetentionAction::DeleteCheckpoint,
            ]
        );
        assert!(decision
            .reasons
            .contains(&MarketDataRetentionReason::VerifiedColdExport));
    }

    #[test]
    fn market_data_wal_reopens_after_valid_existing_records() {
        let root = temp_dir("persist_market_data_wal_reopen");
        let path = root.join("normalized.wal");
        {
            let mut wal = MarketDataWal::open(MarketDataWalConfig::new(&path)).expect("open wal");
            wal.append_record(MarketDataWalRecordKind::Heartbeat, 0, 0, 1, 2, b"")
                .expect("append");
        }

        let wal = MarketDataWal::open(MarketDataWalConfig::new(&path)).expect("reopen wal");
        assert_eq!(wal.next_sequence(), MarketDataWalSequence(2));
        let report = MarketDataWal::inspect_path(&path).expect("inspect");
        assert!(report.valid);
        assert_eq!(report.records, 1);
        assert_eq!(report.last_sequence, Some(MarketDataWalSequence(1)));
    }

    #[test]
    fn market_data_wal_detects_corruption() {
        let root = temp_dir("persist_market_data_wal_corrupt");
        let path = root.join("normalized.wal");
        let mut wal = MarketDataWal::open(MarketDataWalConfig::new(&path)).expect("open wal");
        wal.append_record(MarketDataWalRecordKind::GapMarker, 0, 9, 1, 2, b"gap")
            .expect("append");

        let mut bytes = fs::read(&path).expect("read wal");
        let last = bytes.last_mut().expect("last byte");
        *last ^= 0x01;
        fs::write(&path, bytes).expect("corrupt wal");

        let report = MarketDataWal::inspect_path(&path).expect("inspect");
        assert!(!report.valid);
        assert_eq!(report.checksum_failures, 1);
        assert!(MarketDataWal::open(MarketDataWalConfig::new(&path)).is_err());
    }

    #[test]
    fn market_data_checkpoint_store_saves_and_loads_payload() {
        let root = temp_dir("persist_market_data_checkpoint");
        let store = FileMarketDataCheckpointStore::open(MarketDataCheckpointConfig::new(&root))
            .expect("open checkpoint store");
        let checkpoint = MarketDataCheckpoint::new(
            MarketDataCheckpointKind::BookAndAnalytics,
            "CME",
            "ESZ6",
            MarketDataWalSequence(42),
            b"checkpoint-bytes".to_vec(),
        )
        .with_provider_sequence(100)
        .with_event_sequence(200)
        .with_created_ns(300)
        .with_payload_version(7);

        let manifest = store.save_checkpoint(&checkpoint).expect("save checkpoint");
        let loaded = store
            .load_checkpoint("CME", "ESZ6", manifest.id)
            .expect("load checkpoint");
        let validation = store
            .validate_checkpoint("CME", "ESZ6", manifest.id)
            .expect("validate checkpoint");

        assert_eq!(manifest.id, MarketDataCheckpointId(1));
        assert_eq!(manifest.kind, MarketDataCheckpointKind::BookAndAnalytics);
        assert_eq!(manifest.wal_sequence, MarketDataWalSequence(42));
        assert_eq!(manifest.payload_bytes, b"checkpoint-bytes".len() as u64);
        assert_eq!(loaded.payload, b"checkpoint-bytes");
        assert_eq!(loaded.provider_sequence, 100);
        assert_eq!(loaded.event_sequence, 200);
        assert!(validation.valid);
        assert_eq!(
            validation.manifest.as_ref().map(|manifest| manifest.id),
            Some(MarketDataCheckpointId(1))
        );
    }

    #[test]
    fn market_data_checkpoint_store_loads_latest_valid_by_kind() {
        let root = temp_dir("persist_market_data_checkpoint_latest");
        let store = FileMarketDataCheckpointStore::open(MarketDataCheckpointConfig::new(&root))
            .expect("open checkpoint store");
        store
            .save_checkpoint(&MarketDataCheckpoint::new(
                MarketDataCheckpointKind::Book,
                "CME",
                "NQZ6",
                MarketDataWalSequence(1),
                b"book-1".to_vec(),
            ))
            .expect("save book");
        store
            .save_checkpoint(&MarketDataCheckpoint::new(
                MarketDataCheckpointKind::Analytics,
                "CME",
                "NQZ6",
                MarketDataWalSequence(2),
                b"analytics-1".to_vec(),
            ))
            .expect("save analytics");
        store
            .save_checkpoint(&MarketDataCheckpoint::new(
                MarketDataCheckpointKind::Book,
                "CME",
                "NQZ6",
                MarketDataWalSequence(3),
                b"book-2".to_vec(),
            ))
            .expect("save second book");

        let latest_any = store
            .load_latest("CME", "NQZ6", None)
            .expect("latest any")
            .expect("checkpoint exists");
        let latest_analytics = store
            .load_latest("CME", "NQZ6", Some(MarketDataCheckpointKind::Analytics))
            .expect("latest analytics")
            .expect("analytics checkpoint exists");

        assert_eq!(latest_any.id, MarketDataCheckpointId(3));
        assert_eq!(latest_any.payload, b"book-2");
        assert_eq!(latest_analytics.id, MarketDataCheckpointId(2));
        assert_eq!(latest_analytics.payload, b"analytics-1");
    }

    #[test]
    fn market_data_checkpoint_validation_detects_corruption() {
        let root = temp_dir("persist_market_data_checkpoint_corrupt");
        let store = FileMarketDataCheckpointStore::open(MarketDataCheckpointConfig::new(&root))
            .expect("open checkpoint store");
        let manifest = store
            .save_checkpoint(&MarketDataCheckpoint::new(
                MarketDataCheckpointKind::SequenceState,
                "CME",
                "YMZ6",
                MarketDataWalSequence(9),
                b"sequence-state".to_vec(),
            ))
            .expect("save checkpoint");
        let mut bytes = fs::read(&manifest.path).expect("read checkpoint");
        let last = bytes.last_mut().expect("last byte");
        *last ^= 0x01;
        fs::write(&manifest.path, bytes).expect("corrupt checkpoint");

        let validation = store
            .validate_checkpoint("CME", "YMZ6", manifest.id)
            .expect("validate checkpoint");

        assert!(!validation.valid);
        assert_eq!(validation.checksum_failures, 1);
        assert!(store.load_checkpoint("CME", "YMZ6", manifest.id).is_err());
    }

    #[test]
    fn market_data_checkpoint_store_prunes_old_checkpoints() {
        let root = temp_dir("persist_market_data_checkpoint_prune");
        let store = FileMarketDataCheckpointStore::open(
            MarketDataCheckpointConfig::new(&root).with_retain_last(2),
        )
        .expect("open checkpoint store");
        for sequence in 1..=4 {
            store
                .save_checkpoint(&MarketDataCheckpoint::new(
                    MarketDataCheckpointKind::RuntimeState,
                    "CME",
                    "RTYZ6",
                    MarketDataWalSequence(sequence),
                    vec![sequence as u8],
                ))
                .expect("save checkpoint");
        }

        let manifests = store
            .list_checkpoints("CME", "RTYZ6")
            .expect("list checkpoints");

        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests[0].id, MarketDataCheckpointId(3));
        assert_eq!(manifests[1].id, MarketDataCheckpointId(4));
    }

    #[test]
    fn market_data_checkpoint_store_rejects_non_monotonic_explicit_id() {
        let root = temp_dir("persist_market_data_checkpoint_monotonic");
        let store = FileMarketDataCheckpointStore::open(
            MarketDataCheckpointConfig::new(&root).with_retain_last(1),
        )
        .expect("open checkpoint store");
        store
            .save_checkpoint(&MarketDataCheckpoint::new(
                MarketDataCheckpointKind::RuntimeState,
                "CME",
                "MNQZ6",
                MarketDataWalSequence(1),
                b"one".to_vec(),
            ))
            .expect("save checkpoint");
        let err = store
            .save_checkpoint(
                &MarketDataCheckpoint::new(
                    MarketDataCheckpointKind::RuntimeState,
                    "CME",
                    "MNQZ6",
                    MarketDataWalSequence(2),
                    b"older-id".to_vec(),
                )
                .with_id(MarketDataCheckpointId(1)),
            )
            .expect_err("reject non-monotonic id");

        match err {
            PersistError::Io(err) => assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput),
        }
    }

    #[test]
    fn market_data_recovery_plan_replays_from_checkpoint() {
        let checkpoint = test_checkpoint_manifest(MarketDataWalSequence(10));
        let input = MarketDataRecoveryInput::new(
            Some(checkpoint),
            test_wal_report(true, Some(MarketDataWalSequence(15))),
        );

        let plan = plan_market_data_recovery(MarketDataRecoveryPolicy::fail_closed(), &input);

        assert_eq!(plan.status, MarketDataRecoveryStatus::CleanReplay);
        assert_eq!(plan.replay_from_sequence, Some(MarketDataWalSequence(11)));
        assert_eq!(plan.replay_to_sequence, Some(MarketDataWalSequence(15)));
        assert!(plan.trading_enabled);
        assert_eq!(
            plan.actions,
            vec![
                MarketDataRecoveryAction::RestoreCheckpoint,
                MarketDataRecoveryAction::ReplayWalTail,
                MarketDataRecoveryAction::ResumeMarketData,
            ]
        );
    }

    #[test]
    fn market_data_recovery_plan_requires_checkpoint_by_default() {
        let input = MarketDataRecoveryInput::new(
            None,
            test_wal_report(true, Some(MarketDataWalSequence(2))),
        );

        let plan = plan_market_data_recovery(MarketDataRecoveryPolicy::fail_closed(), &input);

        assert_eq!(plan.status, MarketDataRecoveryStatus::NoCheckpoint);
        assert!(plan.is_impossible());
        assert!(!plan.trading_enabled);
    }

    #[test]
    fn market_data_recovery_plan_rebuilds_from_wal_start_when_allowed() {
        let input = MarketDataRecoveryInput::new(
            None,
            test_wal_report(true, Some(MarketDataWalSequence(3))),
        );

        let plan =
            plan_market_data_recovery(MarketDataRecoveryPolicy::replay_from_wal_start(), &input);

        assert_eq!(plan.status, MarketDataRecoveryStatus::CleanReplay);
        assert_eq!(plan.replay_from_sequence, Some(MarketDataWalSequence(1)));
        assert!(plan.trading_enabled);
        assert_eq!(
            plan.actions,
            vec![
                MarketDataRecoveryAction::ReplayWalTail,
                MarketDataRecoveryAction::ResumeMarketData,
            ]
        );
    }

    #[test]
    fn market_data_recovery_plan_requests_snapshot_on_allowed_gap() {
        let mut report = test_wal_report(false, Some(MarketDataWalSequence(20)));
        report.sequence_failures = 1;
        let input = MarketDataRecoveryInput::new(
            Some(test_checkpoint_manifest(MarketDataWalSequence(10))),
            report,
        );
        let policy = MarketDataRecoveryPolicy::fail_closed().with_allow_sequence_gaps(true);

        let plan = plan_market_data_recovery(policy, &input);

        assert_eq!(plan.status, MarketDataRecoveryStatus::NeedsFreshSnapshot);
        assert!(plan.requires_fresh_snapshot);
        assert!(!plan.trading_enabled);
        assert!(plan
            .actions
            .contains(&MarketDataRecoveryAction::RequestFreshSnapshot));
        assert!(!plan.is_impossible());
    }

    #[test]
    fn market_data_recovery_plan_allows_truncated_tail_by_policy() {
        let mut report = test_wal_report(false, Some(MarketDataWalSequence(4)));
        report.truncated_tail = true;
        let input = MarketDataRecoveryInput::new(
            Some(test_checkpoint_manifest(MarketDataWalSequence(2))),
            report,
        );
        let policy = MarketDataRecoveryPolicy::fail_closed().with_allow_truncated_tail(true);

        let plan = plan_market_data_recovery(policy, &input);

        assert_eq!(plan.status, MarketDataRecoveryStatus::TruncatedWalTail);
        assert!(!plan.is_impossible());
        assert!(!plan.trading_enabled);
        assert!(plan
            .actions
            .contains(&MarketDataRecoveryAction::MarkDegraded));
    }

    #[test]
    fn market_data_recovery_plan_aborts_on_checksum_corruption() {
        let mut report = test_wal_report(false, Some(MarketDataWalSequence(4)));
        report.checksum_failures = 1;
        let input = MarketDataRecoveryInput::new(
            Some(test_checkpoint_manifest(MarketDataWalSequence(2))),
            report,
        );

        let plan = plan_market_data_recovery(MarketDataRecoveryPolicy::fail_closed(), &input);

        assert_eq!(plan.status, MarketDataRecoveryStatus::CorruptWal);
        assert!(plan.is_impossible());
        assert_eq!(
            plan.actions,
            vec![
                MarketDataRecoveryAction::DisableTrading,
                MarketDataRecoveryAction::AbortRecovery,
            ]
        );
    }

    #[test]
    fn market_data_persistence_policy_reports_enabled_modes() {
        let disabled = MarketDataPersistencePolicy::default();
        let strict = MarketDataPersistencePolicy::inline_strict();
        let async_policy = MarketDataPersistencePolicy::bounded_async(1024)
            .with_failure_action(MarketDataPersistenceFailureAction::StopTrading);

        assert!(!disabled.enabled());
        assert!(strict.enabled());
        assert_eq!(strict.mode, MarketDataPersistenceMode::InlineStrict);
        assert!(async_policy.enabled());
        assert_eq!(async_policy.max_queue_depth, 1024);
        assert_eq!(
            async_policy.failure_action,
            MarketDataPersistenceFailureAction::StopTrading
        );
    }

    #[test]
    fn market_data_persistence_health_marks_failures_and_drops_degraded() {
        let policy = MarketDataPersistencePolicy::bounded_async(64);
        let metrics = MarketDataWalMetrics {
            write_failures: 1,
            ..MarketDataWalMetrics::default()
        };

        let health = MarketDataPersistenceHealth::from_wal_metrics(policy, metrics)
            .with_lag(3, 9, 100, 4096)
            .with_dropped_records(2)
            .with_error("disk full");

        assert!(health.enabled);
        assert!(health.degraded);
        assert!(!health.is_healthy());
        assert_eq!(health.queue_depth, 3);
        assert_eq!(health.records_lag, 9);
        assert_eq!(health.bytes_pending, 4096);
        assert_eq!(health.dropped_records, 2);
        assert_eq!(health.write_failures, 1);
        assert_eq!(health.last_error.as_deref(), Some("disk full"));
    }

    #[test]
    fn market_data_backpressure_accepts_when_under_limits() {
        let policy = MarketDataBackpressurePolicy::reject_new(8);
        let health = MarketDataPersistenceHealth {
            enabled: true,
            queue_depth: 4,
            ..MarketDataPersistenceHealth::default()
        };

        let decision = evaluate_market_data_backpressure(
            policy,
            &health,
            MarketDataWalRecordKind::BookUpdate,
            MarketDataRecordCriticality::Normal,
        );

        assert_eq!(decision.action, MarketDataBackpressureAction::Accept);
        assert!(!decision.backpressured);
        assert!(decision.accepts_current);
        assert!(!decision.drops_record);
    }

    #[test]
    fn market_data_backpressure_preserves_trades_under_queue_pressure() {
        let policy = MarketDataBackpressurePolicy::reject_new(8)
            .with_drop_policy(MarketDataBackpressureDropPolicy::PreserveTrades);
        let health = MarketDataPersistenceHealth {
            enabled: true,
            queue_depth: 8,
            ..MarketDataPersistenceHealth::default()
        };

        let trade_decision = evaluate_market_data_backpressure(
            policy,
            &health,
            MarketDataWalRecordKind::TradePrint,
            MarketDataRecordCriticality::Normal,
        );
        let book_decision = evaluate_market_data_backpressure(
            policy,
            &health,
            MarketDataWalRecordKind::BookUpdate,
            MarketDataRecordCriticality::Normal,
        );

        assert_eq!(
            trade_decision.action,
            MarketDataBackpressureAction::DropQueuedLowestPriority
        );
        assert!(trade_decision.preserves_trade);
        assert!(trade_decision.accepts_current);
        assert_eq!(
            book_decision.action,
            MarketDataBackpressureAction::DropCurrent
        );
        assert!(!book_decision.accepts_current);
    }

    #[test]
    fn market_data_backpressure_protects_critical_records() {
        let policy = MarketDataBackpressurePolicy::reject_new(1)
            .with_drop_policy(MarketDataBackpressureDropPolicy::DropNewest)
            .with_protected_criticality(MarketDataRecordCriticality::High);
        let health = MarketDataPersistenceHealth {
            enabled: true,
            queue_depth: 1,
            ..MarketDataPersistenceHealth::default()
        };

        let decision = evaluate_market_data_backpressure(
            policy,
            &health,
            MarketDataWalRecordKind::GapMarker,
            MarketDataRecordCriticality::High,
        );

        assert_eq!(decision.action, MarketDataBackpressureAction::Reject);
        assert!(decision.backpressured);
        assert!(!decision.drops_record);
    }

    #[test]
    fn market_data_backpressure_maps_degraded_failure_action() {
        let policy = MarketDataBackpressurePolicy::reject_new(8)
            .with_failure_action(MarketDataPersistenceFailureAction::StopTrading);
        let health = MarketDataPersistenceHealth {
            enabled: true,
            degraded: true,
            ..MarketDataPersistenceHealth::default()
        };

        let decision = evaluate_market_data_backpressure(
            policy,
            &health,
            MarketDataWalRecordKind::TradePrint,
            MarketDataRecordCriticality::Normal,
        );

        assert_eq!(decision.reason, MarketDataBackpressureReason::Degraded);
        assert_eq!(decision.action, MarketDataBackpressureAction::StopTrading);
        assert!(decision.is_stop());
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
        fs::create_dir_all(&path).expect("temp dir");
        path
    }

    fn test_checkpoint_manifest(
        wal_sequence: MarketDataWalSequence,
    ) -> MarketDataCheckpointManifest {
        MarketDataCheckpointManifest {
            id: MarketDataCheckpointId(1),
            kind: MarketDataCheckpointKind::BookAndAnalytics,
            venue: "CME".to_owned(),
            symbol: "ESZ6".to_owned(),
            wal_sequence,
            provider_sequence: 0,
            event_sequence: 0,
            created_ns: 0,
            payload_version: 1,
            payload_bytes: 0,
            checksum: 0,
            path: PathBuf::from("checkpoint.ofmc"),
        }
    }

    fn test_wal_report(
        valid: bool,
        last_sequence: Option<MarketDataWalSequence>,
    ) -> MarketDataWalIntegrityReport {
        MarketDataWalIntegrityReport {
            valid,
            last_sequence,
            ..MarketDataWalIntegrityReport::default()
        }
    }
}
