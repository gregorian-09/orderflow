use super::*;

/// Synchronous verified Parquet cold-export writer.
#[derive(Debug, Clone)]
pub struct MarketDataParquetWriter {
    config: MarketDataParquetExportConfig,
    schema: SchemaRef,
}

impl MarketDataParquetWriter {
    /// Opens an export root after validating memory and row-group bounds.
    ///
    /// # Errors
    /// Returns an error for zero/inverted bounds or root creation failure.
    pub fn open(config: MarketDataParquetExportConfig) -> MarketDataParquetResult<Self> {
        if config.batch_rows == 0 {
            return Err(MarketDataParquetError::InvalidConfig(
                "batch_rows must be greater than zero".to_owned(),
            ));
        }
        if config.row_group_rows == 0 {
            return Err(MarketDataParquetError::InvalidConfig(
                "row_group_rows must be greater than zero".to_owned(),
            ));
        }
        if config.batch_rows > config.row_group_rows {
            return Err(MarketDataParquetError::InvalidConfig(
                "batch_rows cannot exceed row_group_rows".to_owned(),
            ));
        }
        fs::create_dir_all(&config.root)?;
        Ok(Self {
            config,
            schema: export_schema(),
        })
    }

    /// Returns export configuration.
    pub const fn config(&self) -> &MarketDataParquetExportConfig {
        &self.config
    }

    /// Returns the stable Arrow schema written by this crate version.
    pub fn schema(&self) -> &SchemaRef {
        &self.schema
    }

    /// Exports caller-provided decoded WAL records and verifies the result.
    ///
    /// # Errors
    /// Returns an error for invalid metadata, malformed normalized events,
    /// existing destinations, encoding failures, or failed post-write checks.
    pub fn export_records(
        &self,
        key: &MarketDataParquetPartitionKey,
        source: &MarketDataParquetSourceMetadata,
        records: &[MarketDataWalRecord],
        derived_snapshots: &[MarketDataDerivedSnapshotRef<'_>],
    ) -> MarketDataParquetResult<VerifiedMarketDataParquetExport> {
        validate_partition_key(key)?;
        validate_source(source)?;
        validate_records(records)?;
        validate_derived_snapshots(records, derived_snapshots)?;

        let dir = self
            .config
            .root
            .join(format!("date={}", key.date))
            .join(format!("venue={}", key.venue))
            .join(format!("symbol={}", key.symbol))
            .join(format!("stream={}", key.stream));
        fs::create_dir_all(&dir)?;
        let file_name = export_file_name(records);
        let path = dir.join(file_name);
        let temp_path = path.with_extension("parquet.tmp");
        if path.exists() || temp_path.exists() {
            return Err(MarketDataParquetError::InvalidMetadata(format!(
                "export destination already exists: {}",
                path.display()
            )));
        }

        let properties = WriterProperties::builder()
            .set_compression(parquet_compression(self.config.compression))
            .set_max_row_group_row_count(Some(self.config.row_group_rows))
            .set_created_by(format!(
                "of_persist_parquet/{} schema={EXPORT_SCHEMA_VERSION}",
                env!("CARGO_PKG_VERSION")
            ))
            .build();
        let (mut temp_guard, file) = TemporaryExport::create(&temp_path)?;
        let mut writer = ArrowWriter::try_new(file, Arc::clone(&self.schema), Some(properties))?;
        let mut snapshot_index = 0usize;
        let mut quality_flagged_records = 0u64;
        for records_batch in records.chunks(self.config.batch_rows) {
            let batch = build_batch(
                Arc::clone(&self.schema),
                key,
                source,
                records_batch,
                derived_snapshots,
                &mut snapshot_index,
                &mut quality_flagged_records,
            )?;
            writer.write(&batch)?;
        }
        let metadata = writer.close()?;
        if snapshot_index != derived_snapshots.len() {
            return Err(MarketDataParquetError::InvalidDerivedSnapshots(
                "one or more snapshots did not match an exported record".to_owned(),
            ));
        }
        if self.config.sync_on_write {
            File::open(temp_guard.path())?.sync_all()?;
        }
        temp_guard.link_to(&path)?;

        let bytes = fs::metadata(&path)?.len();
        let (sha256_hex, file_checksum) = hash_file(&path)?;
        let partition = MarketDataColdExportPartition::new(
            MarketDataColdExportFormat::Parquet,
            &key.venue,
            &key.symbol,
            &key.stream,
            path,
        )
        .with_summary(
            records.len() as u64,
            bytes,
            records.first().map(|record| record.sequence),
            records.last().map(|record| record.sequence),
            records.first().map(|record| record.ts_exchange_ns),
            records.last().map(|record| record.ts_exchange_ns),
            file_checksum,
        );
        let proof = VerifiedMarketDataParquetExport {
            partition,
            partition_date: key.date.clone(),
            schema_version: EXPORT_SCHEMA_VERSION,
            sha256_hex,
            row_groups: metadata.num_row_groups() as u64,
            quality_flagged_records,
            derived_snapshot_records: derived_snapshots.len() as u64,
            source: source.clone(),
            verified: true,
        };
        self.verify_export(&proof)?;
        temp_guard.publish();
        Ok(proof)
    }

    /// Replays matching records from a single-file WAL and exports them.
    ///
    /// # Errors
    /// Returns replay, validation, write, or verification failure.
    pub fn export_wal(
        &self,
        key: &MarketDataParquetPartitionKey,
        source: &MarketDataParquetSourceMetadata,
        wal: &MarketDataWal,
        filter: MarketDataWalReplayFilter,
        derived_snapshots: &[MarketDataDerivedSnapshotRef<'_>],
    ) -> MarketDataParquetResult<VerifiedMarketDataParquetExport> {
        let mut records = Vec::new();
        wal.replay_filtered(filter, &mut records)?;
        self.export_records(key, source, &records, derived_snapshots)
    }

    /// Replays matching records from a segmented WAL and exports them.
    ///
    /// # Errors
    /// Returns replay, validation, write, or verification failure.
    pub fn export_segmented_wal(
        &self,
        key: &MarketDataParquetPartitionKey,
        source: &MarketDataParquetSourceMetadata,
        wal: &SegmentedMarketDataWal,
        filter: MarketDataWalReplayFilter,
        derived_snapshots: &[MarketDataDerivedSnapshotRef<'_>],
    ) -> MarketDataParquetResult<VerifiedMarketDataParquetExport> {
        let mut records = Vec::new();
        wal.replay_filtered(filter, &mut records)?;
        self.export_records(key, source, &records, derived_snapshots)
    }

    /// Reopens and fully verifies a previously produced export proof.
    ///
    /// # Errors
    /// Returns an error when bytes, schema, rows, sequence range, row groups,
    /// source metadata, or derived/quality counts differ from the proof.
    pub fn verify_export(
        &self,
        proof: &VerifiedMarketDataParquetExport,
    ) -> MarketDataParquetResult<()> {
        if !proof.verified
            || proof.schema_version != EXPORT_SCHEMA_VERSION
            || proof.partition.format != MarketDataColdExportFormat::Parquet
        {
            return Err(MarketDataParquetError::Verification(
                "proof is not a supported verified schema".to_owned(),
            ));
        }
        let bytes = fs::metadata(&proof.partition.path)?.len();
        if bytes != proof.partition.bytes {
            return Err(MarketDataParquetError::Verification(format!(
                "file byte count changed: expected {}, observed {bytes}",
                proof.partition.bytes
            )));
        }
        let (sha256_hex, checksum) = hash_file(&proof.partition.path)?;
        if sha256_hex != proof.sha256_hex || checksum != proof.partition.checksum {
            return Err(MarketDataParquetError::Verification(
                "file digest changed".to_owned(),
            ));
        }

        let file = File::open(&proof.partition.path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let row_groups = builder.metadata().num_row_groups() as u64;
        if row_groups != proof.row_groups {
            return Err(MarketDataParquetError::Verification(format!(
                "row-group count changed: expected {}, observed {row_groups}",
                proof.row_groups
            )));
        }
        if builder.schema().as_ref() != self.schema.as_ref() {
            return Err(MarketDataParquetError::Verification(
                "Arrow schema changed".to_owned(),
            ));
        }
        let mut reader = builder.with_batch_size(self.config.batch_rows).build()?;
        let mut rows = 0u64;
        let mut first_sequence = None;
        let mut last_sequence = None;
        let mut quality_flagged_records = 0u64;
        let mut derived_snapshot_records = 0u64;
        for batch in &mut reader {
            let batch = batch?;
            verify_constant_columns(&batch, proof)?;
            let sequence = downcast_u64(&batch, 10, "wal_sequence")?;
            let schema_version = downcast_u16(&batch, 0, "schema_version")?;
            if schema_version
                .iter()
                .flatten()
                .any(|value| value != EXPORT_SCHEMA_VERSION)
            {
                return Err(MarketDataParquetError::Verification(
                    "row schema version changed".to_owned(),
                ));
            }
            if let Some(value) = sequence.iter().flatten().next() {
                first_sequence.get_or_insert(MarketDataWalSequence(value));
            }
            if let Some(value) = sequence.iter().flatten().last() {
                last_sequence = Some(MarketDataWalSequence(value));
            }
            let quality = batch
                .column(15)
                .as_any()
                .downcast_ref::<arrow_array::UInt32Array>()
                .ok_or_else(|| column_type_error("quality_flags"))?;
            quality_flagged_records =
                quality_flagged_records.saturating_add(
                    quality.iter().flatten().filter(|value| *value != 0).count() as u64,
                );
            let derived = batch
                .column(19)
                .as_any()
                .downcast_ref::<BinaryArray>()
                .ok_or_else(|| column_type_error("derived_payload"))?;
            derived_snapshot_records = derived_snapshot_records
                .saturating_add((derived.len() - derived.null_count()) as u64);
            verify_payload_checksums(&batch)?;
            rows = rows.saturating_add(batch.num_rows() as u64);
        }
        if rows != proof.partition.records
            || first_sequence != proof.partition.first_sequence
            || last_sequence != proof.partition.last_sequence
            || quality_flagged_records != proof.quality_flagged_records
            || derived_snapshot_records != proof.derived_snapshot_records
        {
            return Err(MarketDataParquetError::Verification(
                "decoded row summary differs from proof".to_owned(),
            ));
        }
        Ok(())
    }
}
