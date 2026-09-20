use super::*;

/// Single-file binary WAL for normalized market-data events.
#[derive(Debug)]
pub struct MarketDataWal {
    pub(crate) path: PathBuf,
    pub(crate) file: File,
    pub(crate) sync_on_write: bool,
    pub(crate) next_sequence: MarketDataWalSequence,
    pub(crate) previous_checksum: u32,
    pub(crate) metrics: MarketDataWalMetrics,
    pub(crate) frame_scratch: Vec<u8>,
}

impl MarketDataWal {
    /// Opens or creates a market-data WAL.
    ///
    /// Existing bytes are validated before append state is initialized.
    pub fn open(config: MarketDataWalConfig) -> PersistResult<Self> {
        if let Some(parent) = config.path.parent() {
            create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&config.path)?;
        let scan = scan_market_data_wal(&config.path, false)?;
        if !scan.report.valid {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "market-data WAL failed integrity validation",
            )
            .into());
        }
        Ok(Self {
            path: config.path,
            file,
            sync_on_write: config.sync_on_write,
            next_sequence: MarketDataWalSequence(
                scan.report
                    .last_sequence
                    .map_or(1, |sequence| sequence.0 + 1),
            ),
            previous_checksum: scan.previous_checksum,
            metrics: MarketDataWalMetrics::default(),
            frame_scratch: Vec::new(),
        })
    }

    /// Returns WAL path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns next sequence that will be assigned.
    pub const fn next_sequence(&self) -> MarketDataWalSequence {
        self.next_sequence
    }

    /// Returns metrics.
    pub const fn metrics(&self) -> MarketDataWalMetrics {
        self.metrics
    }

    /// Appends one encoded WAL record and returns its assigned sequence.
    pub fn append_record(
        &mut self,
        kind: MarketDataWalRecordKind,
        provider_sequence: u64,
        event_sequence: u64,
        ts_exchange_ns: u64,
        ts_recv_ns: u64,
        payload: &[u8],
    ) -> PersistResult<MarketDataWalSequence> {
        let sequence = self.next_sequence;
        if sequence.0 == u64::MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "market-data WAL sequence space is exhausted",
            )
            .into());
        }
        encode_market_data_wal_frame_into(
            &mut self.frame_scratch,
            MarketDataWalFrameInput {
                sequence,
                kind,
                provider_sequence,
                event_sequence,
                ts_exchange_ns,
                ts_recv_ns,
                payload,
                previous_checksum: self.previous_checksum,
            },
        )?;
        if let Err(err) = self.file.write_all(&self.frame_scratch) {
            self.metrics.write_failures = self.metrics.write_failures.saturating_add(1);
            return Err(err.into());
        }
        if self.sync_on_write {
            if let Err(err) = self.file.sync_data() {
                self.metrics.sync_failures = self.metrics.sync_failures.saturating_add(1);
                return Err(err.into());
            }
            self.metrics.sync_count = self.metrics.sync_count.saturating_add(1);
        }
        self.metrics.records_written = self.metrics.records_written.saturating_add(1);
        self.metrics.bytes_written = self
            .metrics
            .bytes_written
            .saturating_add(self.frame_scratch.len() as u64);
        self.previous_checksum = read_u32(&self.frame_scratch[52..56]);
        self.next_sequence = MarketDataWalSequence(self.next_sequence.0 + 1);
        Ok(sequence)
    }

    /// Replays all records into `out`.
    pub fn replay(
        &self,
        out: &mut Vec<MarketDataWalRecord>,
    ) -> PersistResult<MarketDataWalReplayResult> {
        replay_market_data_wal(&self.path, out)
    }

    /// Replays records matching `filter` into `out`.
    pub fn replay_filtered(
        &self,
        filter: MarketDataWalReplayFilter,
        out: &mut Vec<MarketDataWalRecord>,
    ) -> PersistResult<MarketDataWalReplayResult> {
        replay_market_data_wal_filtered(&self.path, filter, out)
    }

    /// Inspects a WAL path for integrity without materializing payloads.
    pub fn inspect_path(path: impl AsRef<Path>) -> PersistResult<MarketDataWalIntegrityReport> {
        Ok(scan_market_data_wal(path.as_ref(), false)?.report)
    }
}

#[derive(Debug, Default)]
pub(crate) struct MarketDataWalScan {
    pub(crate) report: MarketDataWalIntegrityReport,
    pub(crate) previous_checksum: u32,
    pub(crate) last_kind: Option<MarketDataWalRecordKind>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MarketDataWalRecordHeader {
    pub(crate) sequence: MarketDataWalSequence,
    pub(crate) kind: MarketDataWalRecordKind,
    pub(crate) provider_sequence: u64,
    pub(crate) event_sequence: u64,
    pub(crate) ts_exchange_ns: u64,
    pub(crate) ts_recv_ns: u64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MarketDataWalFrameInput<'a> {
    pub(crate) sequence: MarketDataWalSequence,
    pub(crate) kind: MarketDataWalRecordKind,
    pub(crate) provider_sequence: u64,
    pub(crate) event_sequence: u64,
    pub(crate) ts_exchange_ns: u64,
    pub(crate) ts_recv_ns: u64,
    pub(crate) payload: &'a [u8],
    pub(crate) previous_checksum: u32,
}

pub(crate) fn encode_market_data_wal_frame_into(
    frame: &mut Vec<u8>,
    input: MarketDataWalFrameInput<'_>,
) -> PersistResult<()> {
    let payload = input.payload;
    if payload.len() > u32::MAX as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "market-data WAL payload is too large",
        )
        .into());
    }
    frame.clear();
    frame.resize(MARKET_DATA_WAL_HEADER_LEN + payload.len(), 0);
    frame[0..4].copy_from_slice(&MARKET_DATA_WAL_MAGIC);
    write_u16(&mut frame[4..6], MARKET_DATA_WAL_VERSION);
    write_u16(&mut frame[6..8], input.kind as u16);
    write_u64(&mut frame[8..16], input.sequence.0);
    write_u64(&mut frame[16..24], input.provider_sequence);
    write_u64(&mut frame[24..32], input.event_sequence);
    write_u64(&mut frame[32..40], input.ts_exchange_ns);
    write_u64(&mut frame[40..48], input.ts_recv_ns);
    write_u32(&mut frame[48..52], payload.len() as u32);
    write_u32(&mut frame[56..60], input.previous_checksum);
    frame[MARKET_DATA_WAL_HEADER_LEN..].copy_from_slice(payload);
    let checksum = market_data_wal_checksum(frame);
    write_u32(&mut frame[52..56], checksum);
    Ok(())
}

pub(crate) fn replay_market_data_wal(
    path: &Path,
    out: &mut Vec<MarketDataWalRecord>,
) -> PersistResult<MarketDataWalReplayResult> {
    replay_market_data_wal_filtered(path, MarketDataWalReplayFilter::new(), out)
}

pub(crate) fn replay_market_data_wal_filtered(
    path: &Path,
    filter: MarketDataWalReplayFilter,
    out: &mut Vec<MarketDataWalRecord>,
) -> PersistResult<MarketDataWalReplayResult> {
    let before = out.len();
    let scan = scan_market_data_wal_into(path, Some((out, filter)))?;
    let records = out.len().saturating_sub(before);
    Ok(MarketDataWalReplayResult {
        records,
        bytes: scan.report.bytes,
        first_sequence: out.get(before).map(|record| record.sequence),
        last_sequence: records
            .checked_sub(1)
            .and_then(|offset| out.get(before + offset))
            .map(|record| record.sequence),
    })
}

pub(crate) fn scan_market_data_wal(
    path: &Path,
    materialize: bool,
) -> PersistResult<MarketDataWalScan> {
    if materialize {
        let mut records = Vec::new();
        scan_market_data_wal_into(path, Some((&mut records, MarketDataWalReplayFilter::new())))
    } else {
        scan_market_data_wal_into(path, None)
    }
}

pub(crate) fn scan_market_data_wal_into(
    path: &Path,
    out: Option<(&mut Vec<MarketDataWalRecord>, MarketDataWalReplayFilter)>,
) -> PersistResult<MarketDataWalScan> {
    scan_market_data_wal_into_from(path, MarketDataWalSequence(1), 0, out)
}

pub(crate) fn scan_market_data_wal_into_from(
    path: &Path,
    initial_sequence: MarketDataWalSequence,
    initial_previous_checksum: u32,
    mut out: Option<(&mut Vec<MarketDataWalRecord>, MarketDataWalReplayFilter)>,
) -> PersistResult<MarketDataWalScan> {
    if !path.exists() {
        return Ok(MarketDataWalScan {
            report: MarketDataWalIntegrityReport {
                valid: true,
                ..MarketDataWalIntegrityReport::default()
            },
            previous_checksum: initial_previous_checksum,
            last_kind: None,
        });
    }
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(0))?;
    let mut scan = MarketDataWalScan {
        report: MarketDataWalIntegrityReport {
            valid: true,
            ..MarketDataWalIntegrityReport::default()
        },
        previous_checksum: initial_previous_checksum,
        last_kind: None,
    };
    let mut expected_sequence = initial_sequence.0;
    loop {
        let mut header = [0_u8; MARKET_DATA_WAL_HEADER_LEN];
        let read = read_exact_or_tail(&mut file, &mut header)?;
        if read == 0 {
            break;
        }
        if read < MARKET_DATA_WAL_HEADER_LEN {
            scan.report.valid = false;
            scan.report.truncated_tail = true;
            break;
        }
        if header[0..4] != MARKET_DATA_WAL_MAGIC
            || read_u16(&header[4..6]) != MARKET_DATA_WAL_VERSION
        {
            scan.report.valid = false;
            scan.report.checksum_failures = scan.report.checksum_failures.saturating_add(1);
            break;
        }
        let kind = MarketDataWalRecordKind::from_u16(read_u16(&header[6..8])).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid market-data WAL record kind",
            )
        })?;
        let record_header = MarketDataWalRecordHeader {
            sequence: MarketDataWalSequence(read_u64(&header[8..16])),
            kind,
            provider_sequence: read_u64(&header[16..24]),
            event_sequence: read_u64(&header[24..32]),
            ts_exchange_ns: read_u64(&header[32..40]),
            ts_recv_ns: read_u64(&header[40..48]),
        };
        if record_header.sequence.0 != expected_sequence {
            scan.report.valid = false;
            scan.report.sequence_failures = scan.report.sequence_failures.saturating_add(1);
        }
        let payload_len = read_u32(&header[48..52]) as usize;
        let expected_checksum = read_u32(&header[52..56]);
        let previous_checksum = read_u32(&header[56..60]);
        if previous_checksum != scan.previous_checksum {
            scan.report.valid = false;
            scan.report.checksum_failures = scan.report.checksum_failures.saturating_add(1);
        }
        let mut payload = vec![0_u8; payload_len];
        let payload_read = read_exact_or_tail(&mut file, &mut payload)?;
        if payload_read < payload_len {
            scan.report.valid = false;
            scan.report.truncated_tail = true;
            break;
        }
        let mut frame = Vec::with_capacity(MARKET_DATA_WAL_HEADER_LEN + payload_len);
        frame.extend_from_slice(&header);
        frame.extend_from_slice(&payload);
        let actual_checksum = market_data_wal_checksum(&frame);
        if actual_checksum != expected_checksum {
            scan.report.valid = false;
            scan.report.checksum_failures = scan.report.checksum_failures.saturating_add(1);
        }
        if let Some((records, filter)) = out.as_mut() {
            if filter.matches(&record_header) {
                (*records).push(MarketDataWalRecord {
                    sequence: record_header.sequence,
                    kind: record_header.kind,
                    provider_sequence: record_header.provider_sequence,
                    event_sequence: record_header.event_sequence,
                    ts_exchange_ns: record_header.ts_exchange_ns,
                    ts_recv_ns: record_header.ts_recv_ns,
                    payload,
                });
            }
        }
        scan.report.records = scan.report.records.saturating_add(1);
        scan.report.bytes = scan
            .report
            .bytes
            .saturating_add((MARKET_DATA_WAL_HEADER_LEN + payload_len) as u64);
        scan.report.last_sequence = Some(record_header.sequence);
        scan.previous_checksum = expected_checksum;
        scan.last_kind = Some(record_header.kind);
        expected_sequence = record_header.sequence.0.saturating_add(1);
    }
    Ok(scan)
}

pub(crate) fn read_exact_or_tail(file: &mut File, buf: &mut [u8]) -> PersistResult<usize> {
    let mut offset = 0;
    while offset < buf.len() {
        match file.read(&mut buf[offset..]) {
            Ok(0) => break,
            Ok(n) => offset += n,
            Err(err) => return Err(err.into()),
        }
    }
    Ok(offset)
}

pub(crate) fn market_data_wal_checksum(frame: &[u8]) -> u32 {
    let mut hash = 0x811c9dc5_u32;
    for (idx, byte) in frame.iter().enumerate() {
        if (52..56).contains(&idx) {
            hash ^= 0;
        } else {
            hash ^= u32::from(*byte);
        }
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

pub(crate) fn range_contains<T: Ord + Copy>(from: Option<T>, to: Option<T>, value: T) -> bool {
    from.is_none_or(|from| value >= from) && to.is_none_or(|to| value <= to)
}

pub(crate) fn update_fnv1a(mut hash: u32, bytes: &[u8]) -> u32 {
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

pub(crate) fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

pub(crate) fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub(crate) fn read_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

pub(crate) fn write_u16(out: &mut [u8], value: u16) {
    out.copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn write_u32(out: &mut [u8], value: u32) {
    out.copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn write_u64(out: &mut [u8], value: u64) {
    out.copy_from_slice(&value.to_le_bytes());
}
