//! Low-latency execution-domain primitives for Orderflow.
#![doc = include_str!("../README.md")]

use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Maximum bytes stored in an execution diagnostic text field.
pub const EXECUTION_TEXT_CAP: usize = 128;
/// Magic value written at the start of every execution WAL frame.
pub const EXECUTION_WAL_MAGIC: u32 = 0x4c57_464f;
/// Binary execution WAL frame version.
pub const EXECUTION_WAL_VERSION: u16 = 1;
/// Encoded execution WAL header length in bytes.
pub const EXECUTION_WAL_HEADER_LEN: usize = 80;
/// Maximum payload bytes accepted by the execution WAL frame helpers.
pub const EXECUTION_WAL_MAX_PAYLOAD_LEN: usize = u32::MAX as usize;

const EXECUTION_WAL_CHECKSUM_OFFSET: usize = 72;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Fixed-size ASCII field used for low-allocation identifiers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FixedAscii<const N: usize> {
    len: u8,
    bytes: [u8; N],
}

impl<const N: usize> FixedAscii<N> {
    /// Creates an empty fixed ASCII value.
    pub const fn empty() -> Self {
        Self {
            len: 0,
            bytes: [0; N],
        }
    }

    /// Creates a fixed ASCII value from `value`.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionCoreError::IdentifierTooLong`] when `value` exceeds
    /// the fixed capacity, or [`ExecutionCoreError::NonAsciiIdentifier`] when
    /// it contains non-ASCII bytes.
    pub fn new(value: &str) -> Result<Self, ExecutionCoreError> {
        if value.len() > N {
            return Err(ExecutionCoreError::IdentifierTooLong {
                capacity: N,
                actual: value.len(),
            });
        }
        if !value.is_ascii() {
            return Err(ExecutionCoreError::NonAsciiIdentifier);
        }

        let mut bytes = [0; N];
        bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(Self {
            len: value.len() as u8,
            bytes,
        })
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len as usize])
            .expect("FixedAscii stores only validated ASCII")
    }

    /// Returns the fixed field capacity in bytes.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Returns true when the identifier is empty.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> Default for FixedAscii<N> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<const N: usize> fmt::Debug for FixedAscii<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FixedAscii").field(&self.as_str()).finish()
    }
}

impl<const N: usize> fmt::Display for FixedAscii<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<const N: usize> PartialEq for FixedAscii<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<const N: usize> Eq for FixedAscii<N> {}

impl<const N: usize> Hash for FixedAscii<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

/// Client-assigned order identifier.
pub type ClientOrderId = FixedAscii<40>;
/// Venue-assigned order identifier.
pub type VenueOrderId = FixedAscii<48>;
/// Venue execution/fill identifier.
pub type ExecutionId = FixedAscii<48>;
/// Trading account identifier.
pub type AccountId = FixedAscii<32>;
/// Execution route identifier.
pub type RouteId = FixedAscii<32>;
/// Strategy identifier used for attribution.
pub type StrategyId = FixedAscii<32>;
/// Venue identifier used by execution routing.
pub type VenueId = FixedAscii<16>;
/// Instrument identifier in venue/native format.
pub type InstrumentId = FixedAscii<32>;
/// Bounded diagnostic text.
pub type ExecutionText = FixedAscii<EXECUTION_TEXT_CAP>;

/// Execution-core error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionCoreError {
    /// Fixed-size identifier capacity was exceeded.
    IdentifierTooLong {
        /// Configured capacity in bytes.
        capacity: usize,
        /// Actual input length in bytes.
        actual: usize,
    },
    /// Identifier contained a non-ASCII byte.
    NonAsciiIdentifier,
    /// Order quantity must be positive for the requested operation.
    InvalidQuantity,
    /// Price must be positive for price-bearing orders.
    InvalidPrice,
    /// State transition is not valid for the current order state.
    InvalidTransition,
}

impl fmt::Display for ExecutionCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentifierTooLong { capacity, actual } => {
                write!(f, "identifier length {actual} exceeds capacity {capacity}")
            }
            Self::NonAsciiIdentifier => write!(f, "identifier must be ASCII"),
            Self::InvalidQuantity => write!(f, "quantity must be positive"),
            Self::InvalidPrice => write!(f, "price must be positive"),
            Self::InvalidTransition => write!(f, "invalid order state transition"),
        }
    }
}

impl Error for ExecutionCoreError {}

/// Execution WAL record checksum category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WalChecksumField {
    /// Header checksum mismatch.
    Header,
    /// Payload checksum mismatch.
    Payload,
}

/// Error returned by execution WAL frame helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExecutionWalError {
    /// Output buffer was too small.
    BufferTooSmall {
        /// Required byte count.
        required: usize,
        /// Provided byte count.
        actual: usize,
    },
    /// Payload exceeds the WAL frame encoding limit.
    PayloadTooLarge {
        /// Maximum supported payload bytes.
        max: usize,
        /// Actual payload bytes.
        actual: usize,
    },
    /// Frame magic did not match [`EXECUTION_WAL_MAGIC`].
    InvalidMagic {
        /// Magic value found in the frame.
        actual: u32,
    },
    /// Frame version is not supported by this crate.
    UnsupportedVersion {
        /// Expected version.
        expected: u16,
        /// Actual version.
        actual: u16,
    },
    /// Encoded header length is not supported.
    InvalidHeaderLength {
        /// Expected header length.
        expected: usize,
        /// Actual header length.
        actual: usize,
    },
    /// Record kind discriminant is unknown.
    UnknownRecordKind {
        /// Raw record-kind value.
        raw: u16,
    },
    /// Header or payload checksum did not match.
    ChecksumMismatch {
        /// Field that failed validation.
        field: WalChecksumField,
        /// Checksum stored in the frame.
        expected: u64,
        /// Checksum calculated from bytes.
        actual: u64,
    },
    /// Frame ended before the declared record length.
    TruncatedFrame {
        /// Required bytes to decode the frame.
        required: usize,
        /// Available bytes.
        actual: usize,
    },
    /// WAL sequence moved backward or repeated during strict replay.
    SequenceRegression {
        /// Previous accepted sequence.
        previous: WalSequence,
        /// Next decoded sequence.
        next: WalSequence,
    },
    /// WAL sequence skipped a value during strict replay.
    SequenceGap {
        /// Expected sequence.
        expected: WalSequence,
        /// Actual decoded sequence.
        actual: WalSequence,
    },
}

impl fmt::Display for ExecutionWalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall { required, actual } => {
                write!(f, "buffer too small: required {required}, actual {actual}")
            }
            Self::PayloadTooLarge { max, actual } => {
                write!(f, "WAL payload length {actual} exceeds limit {max}")
            }
            Self::InvalidMagic { actual } => write!(f, "invalid WAL magic {actual:#x}"),
            Self::UnsupportedVersion { expected, actual } => {
                write!(f, "unsupported WAL version {actual}; expected {expected}")
            }
            Self::InvalidHeaderLength { expected, actual } => {
                write!(f, "invalid WAL header length {actual}; expected {expected}")
            }
            Self::UnknownRecordKind { raw } => write!(f, "unknown WAL record kind {raw}"),
            Self::ChecksumMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "{field:?} checksum mismatch: expected {expected:#x}, actual {actual:#x}"
            ),
            Self::TruncatedFrame { required, actual } => {
                write!(
                    f,
                    "truncated WAL frame: required {required}, actual {actual}"
                )
            }
            Self::SequenceRegression { previous, next } => {
                write!(
                    f,
                    "WAL sequence regressed from {} to {}",
                    previous.0, next.0
                )
            }
            Self::SequenceGap { expected, actual } => {
                write!(
                    f,
                    "WAL sequence gap: expected {}, actual {}",
                    expected.0, actual.0
                )
            }
        }
    }
}

impl Error for ExecutionWalError {}

/// Monotonic execution WAL sequence number.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WalSequence(pub u64);

impl WalSequence {
    /// Returns the next sequence using saturating arithmetic.
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Execution WAL segment identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WalSegmentId(pub u64);

/// Execution WAL record kind.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WalRecordKind {
    /// New-order command payload.
    CommandSubmit = 1,
    /// Cancel command payload.
    CommandCancel = 2,
    /// Amend/cancel-replace command payload.
    CommandAmend = 3,
    /// Venue or local execution-event payload.
    ExecutionEvent = 4,
    /// Local risk rejection payload.
    RiskReject = 5,
    /// Recovery or venue-restatement payload.
    RecoveryEvent = 6,
    /// Marker tying the WAL to a durable checkpoint.
    CheckpointMarker = 7,
    /// Marker sealing a segment.
    SegmentSeal = 8,
    /// Liveness record with no state transition.
    Heartbeat = 9,
}

impl TryFrom<u16> for WalRecordKind {
    type Error = ExecutionWalError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::CommandSubmit),
            2 => Ok(Self::CommandCancel),
            3 => Ok(Self::CommandAmend),
            4 => Ok(Self::ExecutionEvent),
            5 => Ok(Self::RiskReject),
            6 => Ok(Self::RecoveryEvent),
            7 => Ok(Self::CheckpointMarker),
            8 => Ok(Self::SegmentSeal),
            9 => Ok(Self::Heartbeat),
            raw => Err(ExecutionWalError::UnknownRecordKind { raw }),
        }
    }
}

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

/// Execution symbol in venue-native format.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ExecutionSymbol {
    /// Venue/exchange identifier.
    pub venue: VenueId,
    /// Venue-native instrument symbol.
    pub instrument: InstrumentId,
}

impl ExecutionSymbol {
    /// Creates a symbol from ASCII venue and instrument identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error when either identifier is non-ASCII or too long.
    pub fn new(venue: &str, instrument: &str) -> Result<Self, ExecutionCoreError> {
        Ok(Self {
            venue: VenueId::new(venue)?,
            instrument: InstrumentId::new(instrument)?,
        })
    }
}

/// Integer-normalized order quantity.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct OrderQty(pub i64);

impl OrderQty {
    /// Creates a positive order quantity.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionCoreError::InvalidQuantity`] if `value <= 0`.
    pub fn new(value: i64) -> Result<Self, ExecutionCoreError> {
        if value <= 0 {
            return Err(ExecutionCoreError::InvalidQuantity);
        }
        Ok(Self(value))
    }
}

/// Integer-normalized order price.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct OrderPrice(pub i64);

impl OrderPrice {
    /// Creates a positive order price.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionCoreError::InvalidPrice`] if `value <= 0`.
    pub fn new(value: i64) -> Result<Self, ExecutionCoreError> {
        if value <= 0 {
            return Err(ExecutionCoreError::InvalidPrice);
        }
        Ok(Self(value))
    }
}

/// Buy/sell order side.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderSide {
    /// Buy order.
    Buy = 1,
    /// Sell order.
    Sell = 2,
}

/// Supported canonical order types.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderType {
    /// Market order.
    Market = 1,
    /// Limit order.
    Limit = 2,
    /// Stop order.
    Stop = 3,
    /// Stop-limit order.
    StopLimit = 4,
}

/// Time-in-force policy.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeInForce {
    /// Day order.
    Day = 1,
    /// Good-till-cancelled order.
    Gtc = 2,
    /// Immediate-or-cancel order.
    Ioc = 3,
    /// Fill-or-kill order.
    Fok = 4,
    /// Good-till-date order.
    Gtd = 5,
}

/// FIX-style canonical order status.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    /// Local request is pending submission/acknowledgement.
    PendingNew = 1,
    /// Venue accepted the live order.
    New = 2,
    /// Order has at least one fill and remaining leaves quantity.
    PartiallyFilled = 3,
    /// Order is fully filled.
    Filled = 4,
    /// Cancel request is pending.
    PendingCancel = 5,
    /// Order is cancelled.
    Cancelled = 6,
    /// Cancel/replace request is pending.
    PendingReplace = 7,
    /// Order was replaced.
    Replaced = 8,
    /// Order was rejected.
    Rejected = 9,
    /// Order expired.
    Expired = 10,
    /// Order is suspended.
    Suspended = 11,
    /// Order state could not be reconciled.
    Unknown = 12,
}

impl OrderStatus {
    /// Returns true when no further venue activity is expected.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Filled | Self::Cancelled | Self::Rejected | Self::Expired
        )
    }
}

/// Canonical execution report purpose.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionType {
    /// Order accepted.
    Ack = 1,
    /// Order rejected.
    Reject = 2,
    /// Trade/fill report.
    Trade = 3,
    /// Cancel request accepted or pending.
    CancelPending = 4,
    /// Cancel completed.
    CancelAck = 5,
    /// Cancel request rejected.
    CancelReject = 6,
    /// Replace request accepted or pending.
    ReplacePending = 7,
    /// Replace completed.
    ReplaceAck = 8,
    /// Replace request rejected.
    ReplaceReject = 9,
    /// Order expired.
    Expire = 10,
    /// Status-only report.
    Status = 11,
    /// Recovered or restated state.
    Restated = 12,
    /// Adapter degradation report.
    AdapterDegraded = 13,
}

/// New order request.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderRequest {
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Trading account.
    pub account_id: AccountId,
    /// Execution route.
    pub route_id: RouteId,
    /// Strategy attribution id.
    pub strategy_id: StrategyId,
    /// Target symbol.
    pub symbol: ExecutionSymbol,
    /// Order side.
    pub side: OrderSide,
    /// Order type.
    pub order_type: OrderType,
    /// Time-in-force.
    pub time_in_force: TimeInForce,
    /// Requested quantity.
    pub quantity: OrderQty,
    /// Limit price, or zero for orders without limit price.
    pub limit_price: OrderPrice,
    /// Stop price, or zero for orders without stop price.
    pub stop_price: OrderPrice,
    /// Exchange/session timestamp in nanoseconds when known.
    pub ts_exchange_ns: u64,
    /// Local receive/create timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

impl OrderRequest {
    /// Validates basic order shape.
    ///
    /// # Errors
    ///
    /// Returns invalid quantity/price errors when required fields are not
    /// positive for the selected order type.
    pub fn validate(&self) -> Result<(), ExecutionCoreError> {
        if self.quantity.0 <= 0 {
            return Err(ExecutionCoreError::InvalidQuantity);
        }
        match self.order_type {
            OrderType::Limit | OrderType::StopLimit if self.limit_price.0 <= 0 => {
                Err(ExecutionCoreError::InvalidPrice)
            }
            OrderType::Stop | OrderType::StopLimit if self.stop_price.0 <= 0 => {
                Err(ExecutionCoreError::InvalidPrice)
            }
            _ => Ok(()),
        }
    }
}

/// Cancel request.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelRequest {
    /// New client id for the cancel request.
    pub client_order_id: ClientOrderId,
    /// Last accepted client order id being cancelled.
    pub orig_client_order_id: ClientOrderId,
    /// Venue order id when known.
    pub venue_order_id: VenueOrderId,
    /// Trading account.
    pub account_id: AccountId,
    /// Execution route.
    pub route_id: RouteId,
    /// Target symbol.
    pub symbol: ExecutionSymbol,
    /// Local request timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// Amend/cancel-replace request.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmendRequest {
    /// New client id for the replacement request.
    pub client_order_id: ClientOrderId,
    /// Last accepted client order id being replaced.
    pub orig_client_order_id: ClientOrderId,
    /// Venue order id when known.
    pub venue_order_id: VenueOrderId,
    /// Trading account.
    pub account_id: AccountId,
    /// Execution route.
    pub route_id: RouteId,
    /// Target symbol.
    pub symbol: ExecutionSymbol,
    /// Replacement quantity.
    pub quantity: OrderQty,
    /// Replacement limit price.
    pub limit_price: OrderPrice,
    /// Local request timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// Canonical execution event.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionEvent {
    /// Execution report type.
    pub exec_type: ExecutionType,
    /// Current order status after applying the event.
    pub order_status: OrderStatus,
    /// Current client order id.
    pub client_order_id: ClientOrderId,
    /// Original client order id for cancel/replace flows.
    pub orig_client_order_id: ClientOrderId,
    /// Venue order id.
    pub venue_order_id: VenueOrderId,
    /// Execution/fill id.
    pub execution_id: ExecutionId,
    /// Account id.
    pub account_id: AccountId,
    /// Route id.
    pub route_id: RouteId,
    /// Symbol.
    pub symbol: ExecutionSymbol,
    /// Last fill quantity.
    pub last_qty: OrderQty,
    /// Last fill price.
    pub last_price: OrderPrice,
    /// Cumulative filled quantity.
    pub cumulative_qty: OrderQty,
    /// Remaining quantity.
    pub leaves_qty: OrderQty,
    /// Average fill price.
    pub average_price: OrderPrice,
    /// Exchange/session timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
    /// Structured rejection/degradation reason.
    pub reason: RiskRejectReason,
    /// Bounded diagnostic text.
    pub text: ExecutionText,
}

impl ExecutionEvent {
    /// Creates an accepted event from a new order request.
    pub fn accepted(req: &OrderRequest, venue_order_id: VenueOrderId) -> Self {
        Self {
            exec_type: ExecutionType::Ack,
            order_status: OrderStatus::New,
            client_order_id: req.client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id,
            execution_id: ExecutionId::empty(),
            account_id: req.account_id,
            route_id: req.route_id,
            symbol: req.symbol,
            last_qty: OrderQty(0),
            last_price: OrderPrice(0),
            cumulative_qty: OrderQty(0),
            leaves_qty: req.quantity,
            average_price: OrderPrice(0),
            ts_exchange_ns: req.ts_exchange_ns,
            ts_recv_ns: req.ts_recv_ns,
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        }
    }

    /// Creates a structured local rejection event from a request.
    pub fn rejected(req: &OrderRequest, reason: RiskRejectReason, text: ExecutionText) -> Self {
        Self {
            exec_type: ExecutionType::Reject,
            order_status: OrderStatus::Rejected,
            client_order_id: req.client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: VenueOrderId::empty(),
            execution_id: ExecutionId::empty(),
            account_id: req.account_id,
            route_id: req.route_id,
            symbol: req.symbol,
            last_qty: OrderQty(0),
            last_price: OrderPrice(0),
            cumulative_qty: OrderQty(0),
            leaves_qty: req.quantity,
            average_price: OrderPrice(0),
            ts_exchange_ns: req.ts_exchange_ns,
            ts_recv_ns: req.ts_recv_ns,
            reason,
            text,
        }
    }
}

/// Current order state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderState {
    /// Current client order id.
    pub client_order_id: ClientOrderId,
    /// Last accepted client order id.
    pub last_accepted_client_order_id: ClientOrderId,
    /// Venue order id.
    pub venue_order_id: VenueOrderId,
    /// Account id.
    pub account_id: AccountId,
    /// Route id.
    pub route_id: RouteId,
    /// Symbol.
    pub symbol: ExecutionSymbol,
    /// Side.
    pub side: OrderSide,
    /// Current order status.
    pub status: OrderStatus,
    /// Original order quantity.
    pub order_qty: OrderQty,
    /// Cumulative filled quantity.
    pub cumulative_qty: OrderQty,
    /// Remaining quantity.
    pub leaves_qty: OrderQty,
    /// Average fill price.
    pub average_price: OrderPrice,
    /// Last state update timestamp in nanoseconds.
    pub updated_ns: u64,
}

impl OrderState {
    /// Creates local pending-new state from a request.
    pub fn pending_new(req: &OrderRequest) -> Self {
        Self {
            client_order_id: req.client_order_id,
            last_accepted_client_order_id: req.client_order_id,
            venue_order_id: VenueOrderId::empty(),
            account_id: req.account_id,
            route_id: req.route_id,
            symbol: req.symbol,
            side: req.side,
            status: OrderStatus::PendingNew,
            order_qty: req.quantity,
            cumulative_qty: OrderQty(0),
            leaves_qty: req.quantity,
            average_price: OrderPrice(0),
            updated_ns: req.ts_recv_ns,
        }
    }
}

/// Deterministic order state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderStateMachine {
    state: OrderState,
}

impl OrderStateMachine {
    /// Creates a state machine from an order request.
    pub fn new(req: &OrderRequest) -> Self {
        Self {
            state: OrderState::pending_new(req),
        }
    }

    /// Returns the current order state.
    pub const fn state(&self) -> &OrderState {
        &self.state
    }

    /// Applies an execution event.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionCoreError::InvalidTransition`] when the event cannot
    /// legally apply to the current state.
    pub fn apply(&mut self, event: &ExecutionEvent) -> Result<(), ExecutionCoreError> {
        if self.state.status.is_terminal() && event.exec_type != ExecutionType::Status {
            return Err(ExecutionCoreError::InvalidTransition);
        }

        match event.exec_type {
            ExecutionType::Ack => self.apply_ack(event),
            ExecutionType::Reject => self.apply_terminal(event, OrderStatus::Rejected),
            ExecutionType::Trade => self.apply_trade(event),
            ExecutionType::CancelPending => self.apply_pending(event, OrderStatus::PendingCancel),
            ExecutionType::CancelAck => self.apply_terminal(event, OrderStatus::Cancelled),
            ExecutionType::CancelReject => self.apply_status(event),
            ExecutionType::ReplacePending => self.apply_pending(event, OrderStatus::PendingReplace),
            ExecutionType::ReplaceAck => self.apply_replace(event),
            ExecutionType::ReplaceReject => self.apply_status(event),
            ExecutionType::Expire => self.apply_terminal(event, OrderStatus::Expired),
            ExecutionType::Status | ExecutionType::Restated | ExecutionType::AdapterDegraded => {
                self.apply_status(event)
            }
        }
    }

    fn apply_ack(&mut self, event: &ExecutionEvent) -> Result<(), ExecutionCoreError> {
        if self.state.status != OrderStatus::PendingNew {
            return Err(ExecutionCoreError::InvalidTransition);
        }
        self.state.status = OrderStatus::New;
        self.state.venue_order_id = event.venue_order_id;
        self.state.leaves_qty = event.leaves_qty;
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }

    fn apply_trade(&mut self, event: &ExecutionEvent) -> Result<(), ExecutionCoreError> {
        if event.cumulative_qty.0 > self.state.order_qty.0 {
            return Err(ExecutionCoreError::InvalidTransition);
        }
        self.state.cumulative_qty = event.cumulative_qty;
        self.state.leaves_qty = event.leaves_qty;
        self.state.average_price = event.average_price;
        self.state.status = if event.leaves_qty.0 == 0 {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }

    fn apply_pending(
        &mut self,
        event: &ExecutionEvent,
        status: OrderStatus,
    ) -> Result<(), ExecutionCoreError> {
        if matches!(
            self.state.status,
            OrderStatus::PendingNew | OrderStatus::PendingCancel | OrderStatus::PendingReplace
        ) {
            return Err(ExecutionCoreError::InvalidTransition);
        }
        self.state.status = status;
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }

    fn apply_replace(&mut self, event: &ExecutionEvent) -> Result<(), ExecutionCoreError> {
        if self.state.status != OrderStatus::PendingReplace {
            return Err(ExecutionCoreError::InvalidTransition);
        }
        self.state.client_order_id = event.client_order_id;
        self.state.last_accepted_client_order_id = event.client_order_id;
        self.state.status = OrderStatus::Replaced;
        self.state.order_qty = OrderQty(event.cumulative_qty.0 + event.leaves_qty.0);
        self.state.cumulative_qty = event.cumulative_qty;
        self.state.leaves_qty = event.leaves_qty;
        self.state.average_price = event.average_price;
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }

    fn apply_terminal(
        &mut self,
        event: &ExecutionEvent,
        status: OrderStatus,
    ) -> Result<(), ExecutionCoreError> {
        self.state.status = status;
        self.state.cumulative_qty = event.cumulative_qty;
        self.state.leaves_qty = event.leaves_qty;
        self.state.average_price = event.average_price;
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }

    fn apply_status(&mut self, event: &ExecutionEvent) -> Result<(), ExecutionCoreError> {
        if event.order_status == OrderStatus::Unknown {
            self.state.status = OrderStatus::Unknown;
        } else if event.order_status == OrderStatus::Suspended {
            self.state.status = OrderStatus::Suspended;
        } else if matches!(
            event.exec_type,
            ExecutionType::CancelReject | ExecutionType::ReplaceReject
        ) {
            self.state.status = event.order_status;
        }
        self.state.updated_ns = event.ts_recv_ns;
        Ok(())
    }
}

/// Structured risk rejection reason.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiskRejectReason {
    /// No rejection.
    None = 0,
    /// Kill switch is active.
    KillSwitch = 1,
    /// Account is not enabled.
    AccountDisabled = 2,
    /// Route is not enabled.
    RouteDisabled = 3,
    /// Symbol is not enabled.
    SymbolDisabled = 4,
    /// Quantity exceeds configured max.
    MaxOrderQty = 5,
    /// Notional exceeds configured max.
    MaxOrderNotional = 6,
    /// Open order count exceeds configured max.
    MaxOpenOrders = 7,
    /// Open notional exceeds configured max.
    MaxOpenNotional = 8,
    /// Price is outside configured band.
    PriceBand = 9,
    /// Client order id is already in use.
    DuplicateClientOrderId = 10,
    /// Order type is unsupported on the route.
    UnsupportedOrderType = 11,
    /// Time-in-force is unsupported on the route.
    UnsupportedTimeInForce = 12,
}

/// Risk decision.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskDecision {
    /// True when the request is allowed to route.
    pub allowed: bool,
    /// Structured reject reason.
    pub reason: RiskRejectReason,
    /// Bounded diagnostic text.
    pub text: ExecutionText,
}

impl RiskDecision {
    /// Creates an allow decision.
    pub const fn allow() -> Self {
        Self {
            allowed: true,
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        }
    }

    /// Creates a reject decision.
    pub fn reject(reason: RiskRejectReason, text: ExecutionText) -> Self {
        Self {
            allowed: false,
            reason,
            text,
        }
    }
}

/// Static risk limits for one route/account scope.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskLimits {
    /// Kill switch flag.
    pub kill_switch: bool,
    /// Maximum quantity per order. Zero disables the check.
    pub max_order_qty: i64,
    /// Maximum notional per order. Zero disables the check.
    pub max_order_notional: i128,
    /// Maximum open orders. Zero disables the check.
    pub max_open_orders: u32,
    /// Maximum open notional. Zero disables the check.
    pub max_open_notional: i128,
    /// Allowed absolute distance from reference price. Zero disables the check.
    pub price_band_ticks: i64,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            kill_switch: true,
            max_order_qty: 0,
            max_order_notional: 0,
            max_open_orders: 0,
            max_open_notional: 0,
            price_band_ticks: 0,
        }
    }
}

/// Runtime risk context supplied by the execution engine.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskContext {
    /// Current open order count for the account/route scope.
    pub open_orders: u32,
    /// Current open notional for the account/route scope.
    pub open_notional: i128,
    /// Reference price used for price-band checks. Zero disables price-band checks.
    pub reference_price: OrderPrice,
    /// True when the client order id is already known.
    pub duplicate_client_order_id: bool,
    /// True when account is enabled.
    pub account_enabled: bool,
    /// True when route is enabled.
    pub route_enabled: bool,
    /// True when symbol is enabled.
    pub symbol_enabled: bool,
    /// True when order type is supported.
    pub order_type_supported: bool,
    /// True when time-in-force is supported.
    pub tif_supported: bool,
}

impl Default for RiskContext {
    fn default() -> Self {
        Self {
            open_orders: 0,
            open_notional: 0,
            reference_price: OrderPrice(0),
            duplicate_client_order_id: false,
            account_enabled: false,
            route_enabled: false,
            symbol_enabled: false,
            order_type_supported: false,
            tif_supported: false,
        }
    }
}

/// Pre-trade risk-check contract.
pub trait RiskCheck: Send + Sync {
    /// Checks a new order request.
    fn check_new(&self, req: &OrderRequest, ctx: &RiskContext) -> RiskDecision;
    /// Checks an amend request.
    fn check_amend(&self, req: &AmendRequest, ctx: &RiskContext) -> RiskDecision;
    /// Checks a cancel request.
    fn check_cancel(&self, req: &CancelRequest, ctx: &RiskContext) -> RiskDecision;
}

/// Deterministic pre-trade risk gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasicRiskGate {
    limits: RiskLimits,
}

impl BasicRiskGate {
    /// Creates a risk gate from static limits.
    pub const fn new(limits: RiskLimits) -> Self {
        Self { limits }
    }

    fn check_common(&self, ctx: &RiskContext) -> RiskDecision {
        if self.limits.kill_switch {
            return reject(RiskRejectReason::KillSwitch, "kill switch active");
        }
        if !ctx.account_enabled {
            return reject(RiskRejectReason::AccountDisabled, "account disabled");
        }
        if !ctx.route_enabled {
            return reject(RiskRejectReason::RouteDisabled, "route disabled");
        }
        if !ctx.symbol_enabled {
            return reject(RiskRejectReason::SymbolDisabled, "symbol disabled");
        }
        if ctx.duplicate_client_order_id {
            return reject(
                RiskRejectReason::DuplicateClientOrderId,
                "duplicate client order id",
            );
        }
        if !ctx.order_type_supported {
            return reject(
                RiskRejectReason::UnsupportedOrderType,
                "unsupported order type",
            );
        }
        if !ctx.tif_supported {
            return reject(
                RiskRejectReason::UnsupportedTimeInForce,
                "unsupported time in force",
            );
        }
        if self.limits.max_open_orders > 0 && ctx.open_orders >= self.limits.max_open_orders {
            return reject(RiskRejectReason::MaxOpenOrders, "max open orders exceeded");
        }
        if self.limits.max_open_notional > 0 && ctx.open_notional >= self.limits.max_open_notional {
            return reject(
                RiskRejectReason::MaxOpenNotional,
                "max open notional exceeded",
            );
        }
        RiskDecision::allow()
    }

    fn check_size_price(
        &self,
        qty: OrderQty,
        price: OrderPrice,
        ctx: &RiskContext,
    ) -> RiskDecision {
        if self.limits.max_order_qty > 0 && qty.0 > self.limits.max_order_qty {
            return reject(RiskRejectReason::MaxOrderQty, "max order quantity exceeded");
        }
        if self.limits.max_order_notional > 0 {
            let notional = i128::from(qty.0).saturating_mul(i128::from(price.0));
            if notional > self.limits.max_order_notional {
                return reject(
                    RiskRejectReason::MaxOrderNotional,
                    "max order notional exceeded",
                );
            }
        }
        if self.limits.price_band_ticks > 0 && ctx.reference_price.0 > 0 && price.0 > 0 {
            let distance = price.0.saturating_sub(ctx.reference_price.0).abs();
            if distance > self.limits.price_band_ticks {
                return reject(RiskRejectReason::PriceBand, "price outside risk band");
            }
        }
        RiskDecision::allow()
    }
}

impl RiskCheck for BasicRiskGate {
    fn check_new(&self, req: &OrderRequest, ctx: &RiskContext) -> RiskDecision {
        let common = self.check_common(ctx);
        if !common.allowed {
            return common;
        }
        self.check_size_price(req.quantity, req.limit_price, ctx)
    }

    fn check_amend(&self, req: &AmendRequest, ctx: &RiskContext) -> RiskDecision {
        let common = self.check_common(ctx);
        if !common.allowed {
            return common;
        }
        self.check_size_price(req.quantity, req.limit_price, ctx)
    }

    fn check_cancel(&self, _req: &CancelRequest, ctx: &RiskContext) -> RiskDecision {
        if self.limits.kill_switch {
            return reject(RiskRejectReason::KillSwitch, "kill switch active");
        }
        if !ctx.account_enabled {
            return reject(RiskRejectReason::AccountDisabled, "account disabled");
        }
        if !ctx.route_enabled {
            return reject(RiskRejectReason::RouteDisabled, "route disabled");
        }
        RiskDecision::allow()
    }
}

fn reject(reason: RiskRejectReason, text: &str) -> RiskDecision {
    let text = ExecutionText::new(text).unwrap_or_else(|_| ExecutionText::empty());
    RiskDecision::reject(reason, text)
}

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
