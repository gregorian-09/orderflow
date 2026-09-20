use super::*;

pub const EXECUTION_TEXT_CAP: usize = 128;
/// Magic value written at the start of every execution WAL frame.
pub const EXECUTION_WAL_MAGIC: u32 = 0x4c57_464f;
/// Binary execution WAL frame version.
pub const EXECUTION_WAL_VERSION: u16 = 1;
/// Encoded execution WAL header length in bytes.
pub const EXECUTION_WAL_HEADER_LEN: usize = 80;
/// Maximum payload bytes accepted by the execution WAL frame helpers.
pub const EXECUTION_WAL_MAX_PAYLOAD_LEN: usize = u32::MAX as usize;

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
