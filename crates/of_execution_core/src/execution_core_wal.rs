use super::*;

const EXECUTION_WAL_CHECKSUM_OFFSET: usize = 72;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Execution WAL durability policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WalSyncPolicy {
    /// Never call durable sync from the WAL writer.
    Never,
    /// Sync after every record.
    EveryRecord,
    /// Sync after every configured number of records.
    EveryNRecords(u32),
    /// Sync after the configured elapsed nanoseconds budget.
    EveryDurationNs(u64),
    /// Caller performs explicit sync operations.
    Manual,
    /// Sync at risk-sensitive boundaries such as accepted orders and fills.
    OnRiskBoundary,
}

/// Fixed-size execution WAL record header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct WalRecordHeader {
    /// WAL frame version.
    pub version: u16,
    /// Record kind.
    pub kind: WalRecordKind,
    /// Writer-defined flags.
    pub flags: u16,
    /// Payload length in bytes.
    pub payload_len: u32,
    /// Monotonic WAL sequence.
    pub sequence: WalSequence,
    /// Event or write timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Optional route hash for sharding and diagnostics.
    pub route_hash: u64,
    /// Optional account hash for sharding and diagnostics.
    pub account_hash: u64,
    /// Optional symbol hash for sharding and diagnostics.
    pub symbol_hash: u64,
    /// Previous record checksum or sequence link.
    pub previous_checksum: u64,
    /// Payload checksum.
    pub payload_checksum: u64,
    /// Header checksum.
    pub header_checksum: u64,
}

impl WalRecordHeader {
    /// Creates a header for `payload`.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionWalError::PayloadTooLarge`] when the payload cannot
    /// be represented in the binary frame.
    pub fn new(
        kind: WalRecordKind,
        sequence: WalSequence,
        timestamp_ns: u64,
        payload: &[u8],
    ) -> Result<Self, ExecutionWalError> {
        if payload.len() > EXECUTION_WAL_MAX_PAYLOAD_LEN {
            return Err(ExecutionWalError::PayloadTooLarge {
                max: EXECUTION_WAL_MAX_PAYLOAD_LEN,
                actual: payload.len(),
            });
        }
        let mut header = Self {
            version: EXECUTION_WAL_VERSION,
            kind,
            flags: 0,
            payload_len: payload.len() as u32,
            sequence,
            timestamp_ns,
            route_hash: 0,
            account_hash: 0,
            symbol_hash: 0,
            previous_checksum: 0,
            payload_checksum: execution_wal_checksum(payload),
            header_checksum: 0,
        };
        header.refresh_header_checksum();
        Ok(header)
    }

    /// Sets writer-defined flags and refreshes the header checksum.
    pub fn with_flags(mut self, flags: u16) -> Self {
        self.flags = flags;
        self.refresh_header_checksum();
        self
    }

    /// Sets route/account/symbol hashes and refreshes the header checksum.
    pub fn with_hashes(mut self, route_hash: u64, account_hash: u64, symbol_hash: u64) -> Self {
        self.route_hash = route_hash;
        self.account_hash = account_hash;
        self.symbol_hash = symbol_hash;
        self.refresh_header_checksum();
        self
    }

    /// Sets the previous checksum link and refreshes the header checksum.
    pub fn with_previous_checksum(mut self, previous_checksum: u64) -> Self {
        self.previous_checksum = previous_checksum;
        self.refresh_header_checksum();
        self
    }

    /// Returns total encoded frame length for this header.
    pub const fn frame_len(&self) -> usize {
        EXECUTION_WAL_HEADER_LEN + self.payload_len as usize
    }

    fn refresh_header_checksum(&mut self) {
        self.header_checksum = 0;
        self.header_checksum = self.compute_header_checksum();
    }

    fn compute_header_checksum(&self) -> u64 {
        let mut bytes = [0_u8; EXECUTION_WAL_HEADER_LEN];
        encode_header(self, &mut bytes, false);
        execution_wal_checksum(&bytes[..EXECUTION_WAL_CHECKSUM_OFFSET])
    }

    fn validate_checksums(&self, payload: &[u8]) -> Result<(), ExecutionWalError> {
        let actual_payload = execution_wal_checksum(payload);
        if self.payload_checksum != actual_payload {
            return Err(ExecutionWalError::ChecksumMismatch {
                field: WalChecksumField::Payload,
                expected: self.payload_checksum,
                actual: actual_payload,
            });
        }

        let actual_header = self.compute_header_checksum();
        if self.header_checksum != actual_header {
            return Err(ExecutionWalError::ChecksumMismatch {
                field: WalChecksumField::Header,
                expected: self.header_checksum,
                actual: actual_header,
            });
        }
        Ok(())
    }
}

/// Borrowed execution WAL record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct WalRecordView<'a> {
    /// Decoded WAL header.
    pub header: WalRecordHeader,
    /// Borrowed payload bytes.
    pub payload: &'a [u8],
}

impl<'a> WalRecordView<'a> {
    /// Creates a borrowed WAL record and computes its checksums.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionWalError::PayloadTooLarge`] when the payload cannot
    /// be represented in the binary frame.
    pub fn new(
        kind: WalRecordKind,
        sequence: WalSequence,
        timestamp_ns: u64,
        payload: &'a [u8],
    ) -> Result<Self, ExecutionWalError> {
        Ok(Self {
            header: WalRecordHeader::new(kind, sequence, timestamp_ns, payload)?,
            payload,
        })
    }

    /// Creates a borrowed WAL record from an existing header and payload.
    ///
    /// # Errors
    ///
    /// Returns checksum or length errors when the header does not describe the
    /// payload exactly.
    pub fn from_header(
        header: WalRecordHeader,
        payload: &'a [u8],
    ) -> Result<Self, ExecutionWalError> {
        if header.payload_len as usize != payload.len() {
            return Err(ExecutionWalError::TruncatedFrame {
                required: header.frame_len(),
                actual: EXECUTION_WAL_HEADER_LEN + payload.len(),
            });
        }
        header.validate_checksums(payload)?;
        Ok(Self { header, payload })
    }

    /// Returns total encoded frame length.
    pub const fn encoded_len(&self) -> usize {
        self.header.frame_len()
    }

    /// Encodes this record into `out`.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionWalError::BufferTooSmall`] when `out` cannot hold
    /// the complete frame.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, ExecutionWalError> {
        let required = self.encoded_len();
        if out.len() < required {
            return Err(ExecutionWalError::BufferTooSmall {
                required,
                actual: out.len(),
            });
        }
        encode_header(&self.header, &mut out[..EXECUTION_WAL_HEADER_LEN], true);
        out[EXECUTION_WAL_HEADER_LEN..required].copy_from_slice(self.payload);
        Ok(required)
    }

    /// Appends this record to `out`.
    pub fn append_to(&self, out: &mut Vec<u8>) {
        let start = out.len();
        out.resize(start + self.encoded_len(), 0);
        self.encode_into(&mut out[start..])
            .expect("resized output has exact encoded capacity");
    }

    /// Decodes one record from the beginning of `bytes`.
    ///
    /// # Errors
    ///
    /// Returns a frame, checksum, version, or kind error when bytes do not
    /// contain a valid WAL frame.
    pub fn decode(bytes: &'a [u8]) -> Result<(Self, usize), ExecutionWalError> {
        if bytes.len() < EXECUTION_WAL_HEADER_LEN {
            return Err(ExecutionWalError::TruncatedFrame {
                required: EXECUTION_WAL_HEADER_LEN,
                actual: bytes.len(),
            });
        }

        let header = decode_header(&bytes[..EXECUTION_WAL_HEADER_LEN])?;
        let required = header.frame_len();
        if bytes.len() < required {
            return Err(ExecutionWalError::TruncatedFrame {
                required,
                actual: bytes.len(),
            });
        }

        let payload = &bytes[EXECUTION_WAL_HEADER_LEN..required];
        header.validate_checksums(payload)?;
        Ok((Self { header, payload }, required))
    }
}

/// Sequential borrowed replay cursor for execution WAL bytes.
#[derive(Debug, Clone)]
pub struct WalReplayCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
    previous_sequence: Option<WalSequence>,
    strict_sequence: bool,
}

impl<'a> WalReplayCursor<'a> {
    /// Creates a cursor over encoded WAL bytes.
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            previous_sequence: None,
            strict_sequence: true,
        }
    }

    /// Enables or disables contiguous sequence validation.
    pub const fn with_strict_sequence(mut self, strict_sequence: bool) -> Self {
        self.strict_sequence = strict_sequence;
        self
    }

    /// Returns the current byte offset.
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the number of unread bytes.
    pub const fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    /// Decodes the next record.
    ///
    /// Returns `Ok(None)` when the cursor is at end of input.
    ///
    /// # Errors
    ///
    /// Returns frame, checksum, or strict sequence validation errors.
    pub fn next_record(&mut self) -> Result<Option<WalRecordView<'a>>, ExecutionWalError> {
        if self.offset == self.bytes.len() {
            return Ok(None);
        }

        let (record, consumed) = WalRecordView::decode(&self.bytes[self.offset..])?;
        if self.strict_sequence {
            if let Some(previous) = self.previous_sequence {
                let expected = previous.next();
                if record.header.sequence <= previous {
                    return Err(ExecutionWalError::SequenceRegression {
                        previous,
                        next: record.header.sequence,
                    });
                }
                if record.header.sequence != expected {
                    return Err(ExecutionWalError::SequenceGap {
                        expected,
                        actual: record.header.sequence,
                    });
                }
            }
        }
        self.previous_sequence = Some(record.header.sequence);
        self.offset += consumed;
        Ok(Some(record))
    }
}

impl<'a> Iterator for WalReplayCursor<'a> {
    type Item = Result<WalRecordView<'a>, ExecutionWalError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_record() {
            Ok(Some(record)) => Some(Ok(record)),
            Ok(None) => None,
            Err(error) => {
                self.offset = self.bytes.len();
                Some(Err(error))
            }
        }
    }
}

/// Integrity summary for encoded execution WAL bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct WalIntegrityReport {
    /// Number of valid records decoded before the first fatal frame error.
    pub records: u64,
    /// Number of bytes consumed by valid records.
    pub bytes: u64,
    /// First decoded WAL sequence.
    pub first_sequence: Option<WalSequence>,
    /// Last decoded WAL sequence.
    pub last_sequence: Option<WalSequence>,
    /// Number of checksum mismatches encountered.
    pub checksum_failures: u64,
    /// Number of strict sequence gaps or regressions encountered.
    pub sequence_failures: u64,
    /// True when the input ended with a partial frame.
    pub truncated_tail: bool,
    /// True when all provided bytes decoded cleanly.
    pub valid: bool,
}

impl WalIntegrityReport {
    /// Inspects encoded WAL bytes and returns a non-panicking integrity report.
    pub fn inspect(bytes: &[u8], strict_sequence: bool) -> Self {
        let mut report = Self {
            valid: true,
            ..Self::default()
        };
        let mut cursor = WalReplayCursor::new(bytes).with_strict_sequence(strict_sequence);

        loop {
            match cursor.next_record() {
                Ok(Some(record)) => {
                    report.records = report.records.saturating_add(1);
                    report.bytes = cursor.offset() as u64;
                    report.first_sequence.get_or_insert(record.header.sequence);
                    report.last_sequence = Some(record.header.sequence);
                }
                Ok(None) => break,
                Err(error) => {
                    report.valid = false;
                    match error {
                        ExecutionWalError::ChecksumMismatch { .. } => {
                            report.checksum_failures = report.checksum_failures.saturating_add(1);
                        }
                        ExecutionWalError::SequenceGap { .. }
                        | ExecutionWalError::SequenceRegression { .. } => {
                            report.sequence_failures = report.sequence_failures.saturating_add(1);
                        }
                        ExecutionWalError::TruncatedFrame { .. } => {
                            report.truncated_tail = true;
                        }
                        ExecutionWalError::BufferTooSmall { .. }
                        | ExecutionWalError::PayloadTooLarge { .. }
                        | ExecutionWalError::InvalidMagic { .. }
                        | ExecutionWalError::UnsupportedVersion { .. }
                        | ExecutionWalError::InvalidHeaderLength { .. }
                        | ExecutionWalError::UnknownRecordKind { .. } => {}
                    }
                    break;
                }
            }
        }
        report
    }
}

/// Returns the deterministic non-cryptographic checksum used by WAL frames.
pub fn execution_wal_checksum(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn encode_header(header: &WalRecordHeader, out: &mut [u8], include_header_checksum: bool) {
    put_u32(out, 0, EXECUTION_WAL_MAGIC);
    put_u16(out, 4, header.version);
    put_u16(out, 6, header.kind as u16);
    put_u16(out, 8, EXECUTION_WAL_HEADER_LEN as u16);
    put_u16(out, 10, header.flags);
    put_u32(out, 12, header.payload_len);
    put_u64(out, 16, header.sequence.0);
    put_u64(out, 24, header.timestamp_ns);
    put_u64(out, 32, header.route_hash);
    put_u64(out, 40, header.account_hash);
    put_u64(out, 48, header.symbol_hash);
    put_u64(out, 56, header.previous_checksum);
    put_u64(out, 64, header.payload_checksum);
    put_u64(
        out,
        EXECUTION_WAL_CHECKSUM_OFFSET,
        if include_header_checksum {
            header.header_checksum
        } else {
            0
        },
    );
}

fn decode_header(bytes: &[u8]) -> Result<WalRecordHeader, ExecutionWalError> {
    let magic = get_u32(bytes, 0);
    if magic != EXECUTION_WAL_MAGIC {
        return Err(ExecutionWalError::InvalidMagic { actual: magic });
    }

    let version = get_u16(bytes, 4);
    if version != EXECUTION_WAL_VERSION {
        return Err(ExecutionWalError::UnsupportedVersion {
            expected: EXECUTION_WAL_VERSION,
            actual: version,
        });
    }

    let header_len = get_u16(bytes, 8) as usize;
    if header_len != EXECUTION_WAL_HEADER_LEN {
        return Err(ExecutionWalError::InvalidHeaderLength {
            expected: EXECUTION_WAL_HEADER_LEN,
            actual: header_len,
        });
    }

    let header = WalRecordHeader {
        version,
        kind: WalRecordKind::try_from(get_u16(bytes, 6))?,
        flags: get_u16(bytes, 10),
        payload_len: get_u32(bytes, 12),
        sequence: WalSequence(get_u64(bytes, 16)),
        timestamp_ns: get_u64(bytes, 24),
        route_hash: get_u64(bytes, 32),
        account_hash: get_u64(bytes, 40),
        symbol_hash: get_u64(bytes, 48),
        previous_checksum: get_u64(bytes, 56),
        payload_checksum: get_u64(bytes, 64),
        header_checksum: get_u64(bytes, EXECUTION_WAL_CHECKSUM_OFFSET),
    };
    Ok(header)
}

fn put_u16(out: &mut [u8], offset: usize, value: u16) {
    out[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut [u8], offset: usize, value: u64) {
    out[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn get_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("u16 frame field"),
    )
}

fn get_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("u32 frame field"),
    )
}

fn get_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("u64 frame field"),
    )
}
