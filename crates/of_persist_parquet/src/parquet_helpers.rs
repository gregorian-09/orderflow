use super::*;

pub(crate) fn export_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("schema_version", DataType::UInt16, false),
        Field::new("partition_date", DataType::Utf8, false),
        Field::new("venue", DataType::Utf8, false),
        Field::new("symbol", DataType::Utf8, false),
        Field::new("stream", DataType::Utf8, false),
        Field::new("source_id", DataType::Utf8, false),
        Field::new("adapter_id", DataType::Utf8, false),
        Field::new("session_id", DataType::Utf8, false),
        Field::new("record_kind", DataType::UInt16, false),
        Field::new("record_kind_name", DataType::Utf8, false),
        Field::new("wal_sequence", DataType::UInt64, false),
        Field::new("provider_sequence", DataType::UInt64, false),
        Field::new("event_sequence", DataType::UInt64, false),
        Field::new("ts_exchange_ns", DataType::UInt64, false),
        Field::new("ts_recv_ns", DataType::UInt64, false),
        Field::new("quality_flags", DataType::UInt32, false),
        Field::new("payload", DataType::Binary, false),
        Field::new("payload_checksum", DataType::UInt32, false),
        Field::new("derived_schema_id", DataType::UInt32, true),
        Field::new("derived_payload", DataType::Binary, true),
    ]))
}

pub(crate) fn build_batch(
    schema: SchemaRef,
    key: &MarketDataParquetPartitionKey,
    source: &MarketDataParquetSourceMetadata,
    records: &[MarketDataWalRecord],
    derived_snapshots: &[MarketDataDerivedSnapshotRef<'_>],
    snapshot_index: &mut usize,
    quality_flagged_records: &mut u64,
) -> MarketDataParquetResult<RecordBatch> {
    let rows = records.len();
    let payload_bytes = records
        .iter()
        .map(|record| record.payload.len())
        .sum::<usize>();
    let mut schema_version = UInt16Builder::with_capacity(rows);
    let mut partition_date =
        StringBuilder::with_capacity(rows, rows.saturating_mul(key.date.len()));
    let mut venue = StringBuilder::with_capacity(rows, rows.saturating_mul(key.venue.len()));
    let mut symbol = StringBuilder::with_capacity(rows, rows.saturating_mul(key.symbol.len()));
    let mut stream = StringBuilder::with_capacity(rows, rows.saturating_mul(key.stream.len()));
    let mut source_id =
        StringBuilder::with_capacity(rows, rows.saturating_mul(source.source_id.len()));
    let mut adapter_id =
        StringBuilder::with_capacity(rows, rows.saturating_mul(source.adapter_id.len()));
    let mut session_id =
        StringBuilder::with_capacity(rows, rows.saturating_mul(source.session_id.len()));
    let mut record_kind = UInt16Builder::with_capacity(rows);
    let mut record_kind_name = StringBuilder::with_capacity(rows, rows.saturating_mul(24));
    let mut wal_sequence = UInt64Builder::with_capacity(rows);
    let mut provider_sequence = UInt64Builder::with_capacity(rows);
    let mut event_sequence = UInt64Builder::with_capacity(rows);
    let mut ts_exchange_ns = UInt64Builder::with_capacity(rows);
    let mut ts_recv_ns = UInt64Builder::with_capacity(rows);
    let mut quality_flags = UInt32Builder::with_capacity(rows);
    let mut payload = BinaryBuilder::with_capacity(rows, payload_bytes);
    let mut payload_checksum = UInt32Builder::with_capacity(rows);
    let mut derived_payload = BinaryBuilder::with_capacity(rows, 0);
    let mut derived_schema_id = UInt32Builder::with_capacity(rows);

    for record in records {
        let flags = record_quality_flags_for_partition(record, key)?;
        *quality_flagged_records = quality_flagged_records.saturating_add(u64::from(flags != 0));
        schema_version.append_value(EXPORT_SCHEMA_VERSION);
        partition_date.append_value(&key.date);
        venue.append_value(&key.venue);
        symbol.append_value(&key.symbol);
        stream.append_value(&key.stream);
        source_id.append_value(&source.source_id);
        adapter_id.append_value(&source.adapter_id);
        session_id.append_value(&source.session_id);
        record_kind.append_value(record.kind as u16);
        record_kind_name.append_value(record_kind_name_value(record.kind));
        wal_sequence.append_value(record.sequence.0);
        provider_sequence.append_value(record.provider_sequence);
        event_sequence.append_value(record.event_sequence);
        ts_exchange_ns.append_value(record.ts_exchange_ns);
        ts_recv_ns.append_value(record.ts_recv_ns);
        quality_flags.append_value(flags);
        payload.append_value(&record.payload);
        payload_checksum.append_value(fnv1a(&record.payload));
        if derived_snapshots
            .get(*snapshot_index)
            .is_some_and(|snapshot| snapshot.wal_sequence == record.sequence)
        {
            let snapshot = derived_snapshots[*snapshot_index];
            derived_payload.append_value(snapshot.payload);
            derived_schema_id.append_value(snapshot.schema_id);
            *snapshot_index += 1;
        } else {
            derived_payload.append_null();
            derived_schema_id.append_null();
        }
    }

    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(schema_version.finish()),
            Arc::new(partition_date.finish()),
            Arc::new(venue.finish()),
            Arc::new(symbol.finish()),
            Arc::new(stream.finish()),
            Arc::new(source_id.finish()),
            Arc::new(adapter_id.finish()),
            Arc::new(session_id.finish()),
            Arc::new(record_kind.finish()),
            Arc::new(record_kind_name.finish()),
            Arc::new(wal_sequence.finish()),
            Arc::new(provider_sequence.finish()),
            Arc::new(event_sequence.finish()),
            Arc::new(ts_exchange_ns.finish()),
            Arc::new(ts_recv_ns.finish()),
            Arc::new(quality_flags.finish()),
            Arc::new(payload.finish()),
            Arc::new(payload_checksum.finish()),
            Arc::new(derived_schema_id.finish()),
            Arc::new(derived_payload.finish()),
        ],
    )?)
}

pub(crate) fn validate_partition_key(
    key: &MarketDataParquetPartitionKey,
) -> MarketDataParquetResult<()> {
    if !is_iso_date(&key.date) {
        return Err(MarketDataParquetError::InvalidMetadata(
            "date must use YYYY-MM-DD".to_owned(),
        ));
    }
    validate_component("venue", &key.venue)?;
    validate_component("symbol", &key.symbol)?;
    validate_component("stream", &key.stream)
}

pub(crate) fn validate_source(
    source: &MarketDataParquetSourceMetadata,
) -> MarketDataParquetResult<()> {
    validate_text("source_id", &source.source_id)?;
    validate_text("adapter_id", &source.adapter_id)?;
    validate_text("session_id", &source.session_id)
}

pub(crate) fn validate_records(records: &[MarketDataWalRecord]) -> MarketDataParquetResult<()> {
    let mut previous = None;
    for record in records {
        if let Some(previous) = previous {
            if record.sequence.0 <= previous {
                return Err(MarketDataParquetError::InvalidMetadata(
                    "records must be strictly ordered by WAL sequence".to_owned(),
                ));
            }
        }
        previous = Some(record.sequence.0);
    }
    Ok(())
}

pub(crate) fn validate_derived_snapshots(
    records: &[MarketDataWalRecord],
    snapshots: &[MarketDataDerivedSnapshotRef<'_>],
) -> MarketDataParquetResult<()> {
    let range = records
        .first()
        .zip(records.last())
        .map(|(first, last)| (first.sequence.0, last.sequence.0));
    let mut previous = None;
    for snapshot in snapshots {
        if snapshot.schema_id == 0 {
            return Err(MarketDataParquetError::InvalidDerivedSnapshots(
                "schema_id must be non-zero".to_owned(),
            ));
        }
        if let Some(previous) = previous {
            if snapshot.wal_sequence.0 <= previous {
                return Err(MarketDataParquetError::InvalidDerivedSnapshots(
                    "snapshots must be strictly ordered by WAL sequence".to_owned(),
                ));
            }
        }
        if !range.is_some_and(|(first, last)| {
            snapshot.wal_sequence.0 >= first && snapshot.wal_sequence.0 <= last
        }) {
            return Err(MarketDataParquetError::InvalidDerivedSnapshots(
                "snapshot sequence is outside the export range".to_owned(),
            ));
        }
        previous = Some(snapshot.wal_sequence.0);
    }
    Ok(())
}

pub(crate) fn record_quality_flags_for_partition(
    record: &MarketDataWalRecord,
    key: &MarketDataParquetPartitionKey,
) -> MarketDataParquetResult<u32> {
    if matches!(
        record.kind,
        MarketDataWalRecordKind::BookUpdate | MarketDataWalRecordKind::TradePrint
    ) {
        let decoded = decode_normalized_market_data_record(record)?;
        let symbol = decoded.symbol();
        if symbol.venue != key.venue || symbol.symbol != key.symbol {
            return Err(MarketDataParquetError::InvalidMetadata(
                "normalized event does not match the target venue/symbol partition".to_owned(),
            ));
        }
        return Ok(decoded.quality_flags_bits());
    }
    Ok(0)
}

pub(crate) fn verify_payload_checksums(batch: &RecordBatch) -> MarketDataParquetResult<()> {
    let payloads = batch
        .column(16)
        .as_any()
        .downcast_ref::<BinaryArray>()
        .ok_or_else(|| column_type_error("payload"))?;
    let checksums = batch
        .column(17)
        .as_any()
        .downcast_ref::<UInt32Array>()
        .ok_or_else(|| column_type_error("payload_checksum"))?;
    if payloads.len() != checksums.len()
        || payloads
            .iter()
            .zip(checksums.iter())
            .any(|(payload, checksum)| match (payload, checksum) {
                (Some(payload), Some(checksum)) => fnv1a(payload) != checksum,
                _ => true,
            })
    {
        return Err(MarketDataParquetError::Verification(
            "payload checksum column is inconsistent".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn verify_constant_columns(
    batch: &RecordBatch,
    proof: &VerifiedMarketDataParquetExport,
) -> MarketDataParquetResult<()> {
    for (index, name, expected) in [
        (1, "partition_date", proof.partition_date.as_str()),
        (2, "venue", proof.partition.venue.as_str()),
        (3, "symbol", proof.partition.symbol.as_str()),
        (4, "stream", proof.partition.stream.as_str()),
        (5, "source_id", proof.source.source_id.as_str()),
        (6, "adapter_id", proof.source.adapter_id.as_str()),
        (7, "session_id", proof.source.session_id.as_str()),
    ] {
        let values = batch
            .column(index)
            .as_any()
            .downcast_ref::<arrow_array::StringArray>()
            .ok_or_else(|| column_type_error(name))?;
        if values.iter().flatten().any(|value| value != expected) {
            return Err(MarketDataParquetError::Verification(format!(
                "{name} column differs from proof"
            )));
        }
    }
    Ok(())
}

pub(crate) fn downcast_u64<'a>(
    batch: &'a RecordBatch,
    index: usize,
    name: &str,
) -> MarketDataParquetResult<&'a UInt64Array> {
    batch
        .column(index)
        .as_any()
        .downcast_ref::<UInt64Array>()
        .ok_or_else(|| column_type_error(name))
}

pub(crate) fn downcast_u16<'a>(
    batch: &'a RecordBatch,
    index: usize,
    name: &str,
) -> MarketDataParquetResult<&'a UInt16Array> {
    batch
        .column(index)
        .as_any()
        .downcast_ref::<UInt16Array>()
        .ok_or_else(|| column_type_error(name))
}

pub(crate) fn column_type_error(name: &str) -> MarketDataParquetError {
    MarketDataParquetError::Verification(format!("unexpected type for {name} column"))
}

pub(crate) fn validate_component(name: &str, value: &str) -> MarketDataParquetResult<()> {
    validate_text(name, value)?;
    if value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains('=')
        || value.ends_with(".tmp")
    {
        return Err(MarketDataParquetError::InvalidMetadata(format!(
            "{name} is not a safe partition component"
        )));
    }
    Ok(())
}

pub(crate) fn validate_text(name: &str, value: &str) -> MarketDataParquetResult<()> {
    if value.is_empty() || value.len() > 255 || value.chars().any(char::is_control) {
        return Err(MarketDataParquetError::InvalidMetadata(format!(
            "{name} must contain 1..=255 non-control bytes"
        )));
    }
    Ok(())
}

pub(crate) fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    let syntax_valid = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit());
    if !syntax_valid {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u16>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return false;
    };
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

pub(crate) fn export_file_name(records: &[MarketDataWalRecord]) -> String {
    match (records.first(), records.last()) {
        (Some(first), Some(last)) => format!(
            "wal-{:020}-{:020}.parquet",
            first.sequence.0, last.sequence.0
        ),
        _ => "wal-empty.parquet".to_owned(),
    }
}

pub(crate) fn parquet_compression(compression: MarketDataParquetCompression) -> Compression {
    match compression {
        MarketDataParquetCompression::Uncompressed => Compression::UNCOMPRESSED,
        MarketDataParquetCompression::Snappy => Compression::SNAPPY,
        MarketDataParquetCompression::Zstd => Compression::ZSTD(ZstdLevel::default()),
    }
}

pub(crate) fn record_kind_name_value(kind: MarketDataWalRecordKind) -> &'static str {
    match kind {
        MarketDataWalRecordKind::BookUpdate => "BookUpdate",
        MarketDataWalRecordKind::TradePrint => "TradePrint",
        MarketDataWalRecordKind::Heartbeat => "Heartbeat",
        MarketDataWalRecordKind::GapMarker => "GapMarker",
        MarketDataWalRecordKind::SegmentSeal => "SegmentSeal",
        MarketDataWalRecordKind::BookSnapshotMarker => "BookSnapshotMarker",
        MarketDataWalRecordKind::QualityFlag => "QualityFlag",
        MarketDataWalRecordKind::AdapterHealth => "AdapterHealth",
        MarketDataWalRecordKind::SubscriptionState => "SubscriptionState",
        MarketDataWalRecordKind::OutOfOrderMarker => "OutOfOrderMarker",
        MarketDataWalRecordKind::CheckpointMarker => "CheckpointMarker",
        MarketDataWalRecordKind::RawProviderMessage => "RawProviderMessage",
        _ => "Unknown",
    }
}

pub(crate) fn hash_file(path: &Path) -> MarketDataParquetResult<(String, u32)> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(HASH_BUFFER_BYTES, file);
    let mut sha256 = Sha256::new();
    let mut fnv = 0x811c9dc5_u32;
    let mut buffer = [0u8; HASH_BUFFER_BYTES];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        sha256.update(&buffer[..read]);
        fnv = update_fnv1a(fnv, &buffer[..read]);
    }
    let digest = sha256.finalize();
    Ok((lower_hex(&digest), fnv))
}

pub(crate) fn lower_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

pub(crate) fn fnv1a(bytes: &[u8]) -> u32 {
    update_fnv1a(0x811c9dc5_u32, bytes)
}

pub(crate) fn update_fnv1a(mut checksum: u32, bytes: &[u8]) -> u32 {
    for byte in bytes {
        checksum ^= u32::from(*byte);
        checksum = checksum.wrapping_mul(0x01000193);
    }
    checksum
}
