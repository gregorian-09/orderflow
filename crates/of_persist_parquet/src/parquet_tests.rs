#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use of_core::{Side, SymbolId, TradePrint};
    use of_persist::{
        MarketDataRetentionPolicy, MarketDataWalConfig, MarketDataWalRecordKind,
        MarketDataWalSyncPolicy, NormalizedMarketDataRecordInput, SegmentedMarketDataWalConfig,
    };

    use super::*;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "of-persist-parquet-{name}-{}-{nonce}",
                std::process::id()
            )))
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn trade_record(sequence: u64, quality_flags_bits: u32) -> MarketDataWalRecord {
        let input = NormalizedMarketDataRecordInput::trade_with_quality(
            TradePrint {
                symbol: SymbolId {
                    venue: "CME".to_owned(),
                    symbol: "ESM6".to_owned(),
                },
                price: 5_050_000 + sequence as i64,
                size: sequence as i64,
                aggressor_side: Side::Ask,
                sequence,
                ts_exchange_ns: sequence * 10,
                ts_recv_ns: sequence * 10 + 1,
            },
            quality_flags_bits,
        );
        let mut payload = Vec::new();
        input.encode_into(&mut payload).expect("encode");
        MarketDataWalRecord::new(
            MarketDataWalSequence(sequence),
            MarketDataWalRecordKind::TradePrint,
            sequence,
            sequence,
            sequence * 10,
            sequence * 10 + 1,
            payload,
        )
    }

    fn key() -> MarketDataParquetPartitionKey {
        MarketDataParquetPartitionKey::new("2026-08-12", "CME", "ESM6", "trades")
    }

    fn source() -> MarketDataParquetSourceMetadata {
        MarketDataParquetSourceMetadata::new("primary", "cqg", "session-1")
    }

    #[test]
    fn export_is_partitioned_verified_and_retention_safe() {
        let root = TestRoot::new("round-trip");
        let writer = MarketDataParquetWriter::open(
            MarketDataParquetExportConfig::new(&root.0)
                .with_batch_rows(1)
                .with_row_group_rows(1)
                .with_compression(MarketDataParquetCompression::Snappy),
        )
        .expect("writer");
        let records = vec![trade_record(1, 0), trade_record(2, 0x20)];
        let snapshots = [MarketDataDerivedSnapshotRef::new(
            MarketDataWalSequence(2),
            7,
            b"derived",
        )];
        let proof = writer
            .export_records(&key(), &source(), &records, &snapshots)
            .expect("export");

        assert!(proof.verified);
        assert_eq!(proof.partition.format, MarketDataColdExportFormat::Parquet);
        assert_eq!(proof.partition.records, 2);
        assert_eq!(
            proof.partition.first_sequence,
            Some(MarketDataWalSequence(1))
        );
        assert_eq!(
            proof.partition.last_sequence,
            Some(MarketDataWalSequence(2))
        );
        assert_eq!(proof.row_groups, 2);
        assert_eq!(proof.quality_flagged_records, 1);
        assert_eq!(proof.derived_snapshot_records, 1);
        assert_eq!(proof.sha256_hex.len(), 64);
        assert!(proof.partition.path.ends_with(
            "date=2026-08-12/venue=CME/symbol=ESM6/stream=trades/wal-00000000000000000001-00000000000000000002.parquet"
        ));
        writer.verify_export(&proof).expect("reverify");

        let retention = proof.retention_input(10, 100);
        assert!(retention.cold_export_verified);
        let policy = MarketDataRetentionPolicy::conservative()
            .with_hot_retention_ns(1)
            .with_min_checkpoints_retained(0);
        let decision = of_persist::plan_market_data_retention(policy, 20, &retention);
        assert!(decision
            .actions
            .contains(&of_persist::MarketDataRetentionAction::DeleteHotWal));
    }

    #[test]
    fn corruption_invalidates_existing_proof() {
        let root = TestRoot::new("corruption");
        let writer = MarketDataParquetWriter::open(
            MarketDataParquetExportConfig::new(&root.0)
                .with_batch_rows(1)
                .with_row_group_rows(1),
        )
        .expect("writer");
        let proof = writer
            .export_records(&key(), &source(), &[trade_record(1, 0)], &[])
            .expect("export");
        let file = fs::OpenOptions::new()
            .append(true)
            .open(&proof.partition.path)
            .expect("open append");
        file.set_len(proof.partition.bytes + 1).expect("corrupt");
        let error = writer.verify_export(&proof).expect_err("must reject");
        assert!(matches!(error, MarketDataParquetError::Verification(_)));
    }

    #[test]
    fn metadata_and_ordering_fail_before_publication() {
        let root = TestRoot::new("validation");
        let writer = MarketDataParquetWriter::open(MarketDataParquetExportConfig::new(&root.0))
            .expect("writer");
        let invalid_key =
            MarketDataParquetPartitionKey::new("2026-13-12", "../CME", "ESM6", "trades");
        assert!(matches!(
            writer.export_records(&invalid_key, &source(), &[trade_record(1, 0)], &[]),
            Err(MarketDataParquetError::InvalidMetadata(_))
        ));
        let impossible_date =
            MarketDataParquetPartitionKey::new("2026-02-29", "CME", "ESM6", "trades");
        assert!(matches!(
            writer.export_records(&impossible_date, &source(), &[trade_record(1, 0)], &[]),
            Err(MarketDataParquetError::InvalidMetadata(_))
        ));

        let records = vec![trade_record(1, 0), trade_record(2, 0)];
        let snapshots = [
            MarketDataDerivedSnapshotRef::new(MarketDataWalSequence(2), 1, b"two"),
            MarketDataDerivedSnapshotRef::new(MarketDataWalSequence(1), 1, b"one"),
        ];
        assert!(matches!(
            writer.export_records(&key(), &source(), &records, &snapshots),
            Err(MarketDataParquetError::InvalidDerivedSnapshots(_))
        ));

        let mut malformed = trade_record(3, 0);
        malformed.payload[0] ^= 0xff;
        assert!(matches!(
            writer.export_records(&key(), &source(), &[malformed], &[]),
            Err(MarketDataParquetError::Normalized(_))
        ));
        assert!(!walk_files(&root.0)
            .iter()
            .any(|path| path.extension().is_some_and(|extension| extension == "tmp")));
    }

    #[test]
    fn single_and_segmented_wal_export_paths_replay_filters() {
        let root = TestRoot::new("wal-bridges");
        let mut single = MarketDataWal::open(MarketDataWalConfig::new(root.0.join("single.ofmw")))
            .expect("single WAL");
        let record = trade_record(1, 0x10);
        single
            .append_record(
                record.kind,
                record.provider_sequence,
                record.event_sequence,
                record.ts_exchange_ns,
                record.ts_recv_ns,
                &record.payload,
            )
            .expect("single append");

        let writer = MarketDataParquetWriter::open(
            MarketDataParquetExportConfig::new(root.0.join("cold"))
                .with_batch_rows(1)
                .with_row_group_rows(1),
        )
        .expect("writer");
        let single_proof = writer
            .export_wal(
                &key(),
                &source(),
                &single,
                MarketDataWalReplayFilter::new().with_sequence_range(
                    Some(MarketDataWalSequence(1)),
                    Some(MarketDataWalSequence(1)),
                ),
                &[],
            )
            .expect("single export");
        assert_eq!(single_proof.partition.records, 1);

        let mut segmented = SegmentedMarketDataWal::open(
            SegmentedMarketDataWalConfig::new(root.0.join("segmented"))
                .with_sync_policy(MarketDataWalSyncPolicy::Never),
        )
        .expect("segmented WAL");
        segmented
            .append_record(
                record.kind,
                record.provider_sequence,
                record.event_sequence,
                record.ts_exchange_ns,
                record.ts_recv_ns,
                &record.payload,
            )
            .expect("segmented append");
        let segmented_key =
            MarketDataParquetPartitionKey::new("2026-08-12", "CME", "ESM6", "trades-segmented");
        let segmented_proof = writer
            .export_segmented_wal(
                &segmented_key,
                &source(),
                &segmented,
                MarketDataWalReplayFilter::new(),
                &[],
            )
            .expect("segmented export");
        assert_eq!(segmented_proof.partition.records, 1);
        assert_eq!(segmented_proof.quality_flagged_records, 1);
    }

    #[test]
    fn config_rejects_unbounded_or_inverted_batches() {
        let root = TestRoot::new("config");
        assert!(MarketDataParquetWriter::open(
            MarketDataParquetExportConfig::new(&root.0).with_batch_rows(0)
        )
        .is_err());
        assert!(MarketDataParquetWriter::open(
            MarketDataParquetExportConfig::new(&root.0)
                .with_batch_rows(2)
                .with_row_group_rows(1)
        )
        .is_err());
    }

    fn walk_files(root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let Ok(entries) = fs::read_dir(root) else {
            return files;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_files(&path));
            } else {
                files.push(path);
            }
        }
        files
    }
}
