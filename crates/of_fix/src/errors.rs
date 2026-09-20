use super::*;

/// Errors returned while interpreting a flat FIX repeating group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixGroupError {
    /// The group count tag is absent.
    MissingCountTag(FixTag),
    /// The group count is not an unsigned integer.
    InvalidCount(FixTag),
    /// The caller-provided entry boundary scratch is too small.
    ScratchTooSmall {
        /// Number of entries declared by the message.
        required: usize,
        /// Number of boundaries supplied by the caller.
        capacity: usize,
    },
    /// An entry does not begin with the configured delimiter tag.
    MissingDelimiter {
        /// Zero-based entry index.
        index: usize,
        /// Expected delimiter tag.
        tag: FixTag,
    },
    /// A tag that belongs to the group appears after the declared entries.
    UnexpectedField {
        /// Field index in the parsed message.
        index: usize,
        /// Unexpected tag.
        tag: FixTag,
    },
}

impl fmt::Display for FixGroupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCountTag(tag) => {
                write!(f, "FIX repeating-group count tag {tag} is missing")
            }
            Self::InvalidCount(tag) => write!(f, "FIX repeating-group count tag {tag} is invalid"),
            Self::ScratchTooSmall { required, capacity } => write!(
                f,
                "FIX repeating-group scratch too small: required {required}, capacity {capacity}"
            ),
            Self::MissingDelimiter { index, tag } => {
                write!(
                    f,
                    "FIX repeating-group entry {index} is missing delimiter tag {tag}"
                )
            }
            Self::UnexpectedField { index, tag } => {
                write!(
                    f,
                    "FIX repeating-group field {index} has unexpected tag {tag}"
                )
            }
        }
    }
}

impl Error for FixGroupError {}

/// FIX parse and validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixParseError {
    /// The frame is empty.
    Empty,
    /// A field does not contain `=`.
    MalformedField,
    /// A field tag is not numeric.
    InvalidTag,
    /// A required field is missing.
    MissingRequiredTag(FixTag),
    /// The provided scratch buffer cannot hold all fields.
    ScratchTooSmall {
        /// Required number of fields.
        required: usize,
        /// Provided scratch capacity.
        capacity: usize,
    },
    /// `BodyLength(9)` is not numeric.
    InvalidBodyLength,
    /// `BodyLength(9)` does not match the raw frame.
    BodyLengthMismatch {
        /// Length declared by tag `9`.
        expected: usize,
        /// Length computed from raw frame bytes.
        actual: usize,
    },
    /// `CheckSum(10)` is malformed.
    InvalidChecksum,
    /// `CheckSum(10)` does not match the raw frame.
    ChecksumMismatch {
        /// Checksum declared by tag `10`.
        expected: u8,
        /// Checksum computed from raw frame bytes.
        actual: u8,
    },
}

impl fmt::Display for FixParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "FIX frame is empty"),
            Self::MalformedField => write!(f, "FIX field is malformed"),
            Self::InvalidTag => write!(f, "FIX field tag is invalid"),
            Self::MissingRequiredTag(tag) => write!(f, "required FIX tag {tag} is missing"),
            Self::ScratchTooSmall { required, capacity } => write!(
                f,
                "FIX parse scratch too small: required {required}, capacity {capacity}"
            ),
            Self::InvalidBodyLength => write!(f, "FIX BodyLength(9) is invalid"),
            Self::BodyLengthMismatch { expected, actual } => write!(
                f,
                "FIX BodyLength(9) mismatch: expected {expected}, actual {actual}"
            ),
            Self::InvalidChecksum => write!(f, "FIX CheckSum(10) is invalid"),
            Self::ChecksumMismatch { expected, actual } => write!(
                f,
                "FIX CheckSum(10) mismatch: expected {expected:03}, actual {actual:03}"
            ),
        }
    }
}

impl Error for FixParseError {}

/// FIX encode errors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixEncodeError {
    /// A field value contains SOH, which would corrupt framing.
    ValueContainsSoh(FixTag),
    /// Caller attempted to pass a header/trailer tag that the encoder owns.
    ReservedTag(FixTag),
    /// A source message is missing a required tag for the requested encoding.
    MissingRequiredTag(FixTag),
    /// The generated repeating-group count tag was supplied in ordinary fields.
    DuplicateRepeatingGroupCountTag(FixTag),
    /// A repeating-group entry does not begin with its configured delimiter.
    MissingRepeatingGroupDelimiter {
        /// Zero-based entry index.
        group: usize,
        /// Expected delimiter tag.
        tag: FixTag,
    },
    /// A repeating-group entry contains a tag outside its definition.
    InvalidRepeatingGroupField {
        /// Zero-based entry index.
        group: usize,
        /// Zero-based field index within the entry.
        index: usize,
        /// Invalid tag.
        tag: FixTag,
    },
    /// A repeating-group delimiter appears after the first field of an entry.
    RepeatedRepeatingGroupDelimiter {
        /// Zero-based entry index.
        group: usize,
        /// Zero-based field index within the entry.
        index: usize,
        /// Repeated delimiter tag.
        tag: FixTag,
    },
}

impl fmt::Display for FixEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueContainsSoh(tag) => write!(f, "FIX value for tag {tag} contains SOH"),
            Self::ReservedTag(tag) => write!(f, "FIX tag {tag} is owned by the encoder"),
            Self::MissingRequiredTag(tag) => write!(f, "FIX source message is missing tag {tag}"),
            Self::DuplicateRepeatingGroupCountTag(tag) => {
                write!(
                    f,
                    "FIX repeating-group count tag {tag} was supplied more than once"
                )
            }
            Self::MissingRepeatingGroupDelimiter { group, tag } => write!(
                f,
                "FIX repeating-group entry {group} is missing delimiter tag {tag}"
            ),
            Self::InvalidRepeatingGroupField { group, index, tag } => write!(
                f,
                "FIX repeating-group entry {group} field {index} has invalid tag {tag}"
            ),
            Self::RepeatedRepeatingGroupDelimiter { group, index, tag } => write!(
                f,
                "FIX repeating-group entry {group} field {index} repeats delimiter tag {tag}"
            ),
        }
    }
}

impl Error for FixEncodeError {}

/// FIX dictionary/profile validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixProfileError {
    /// `BeginString(8)` is missing from the parsed message.
    MissingBeginString,
    /// `BeginString(8)` is not one of the known versions represented by
    /// [`FixVersion`].
    UnsupportedVersion,
    /// The message version does not match the dictionary version.
    VersionMismatch {
        /// Version expected by the dictionary.
        expected: FixVersion,
        /// Version declared by the message.
        actual: FixVersion,
    },
    /// `MsgType(35)` is missing.
    MissingMsgType,
    /// No rule exists for the message type in this dictionary.
    UnsupportedMsgType,
    /// A required tag is missing for the message type.
    MissingRequiredTag {
        /// Message type being validated.
        msg_type: FixMsgType,
        /// Required tag that was not present.
        tag: FixTag,
    },
    /// A tag explicitly disallowed by the profile is present.
    DisallowedTag {
        /// Message type being validated.
        msg_type: FixMsgType,
        /// Disallowed tag that was present.
        tag: FixTag,
    },
}

impl fmt::Display for FixProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBeginString => write!(f, "FIX BeginString(8) is missing"),
            Self::UnsupportedVersion => write!(f, "FIX BeginString(8) is unsupported"),
            Self::VersionMismatch { expected, actual } => {
                write!(
                    f,
                    "FIX version mismatch: expected {expected}, actual {actual}"
                )
            }
            Self::MissingMsgType => write!(f, "FIX MsgType(35) is missing"),
            Self::UnsupportedMsgType => write!(f, "FIX MsgType(35) is unsupported"),
            Self::MissingRequiredTag { msg_type, tag } => {
                write!(f, "FIX message {msg_type} is missing required tag {tag}")
            }
            Self::DisallowedTag { msg_type, tag } => {
                write!(f, "FIX message {msg_type} contains disallowed tag {tag}")
            }
        }
    }
}

impl Error for FixProfileError {}

/// FIX reject-message parse errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixRejectParseError {
    /// Message type does not match the requested reject parser.
    InvalidMsgType,
    /// A required reject tag is missing.
    MissingTag(FixTag),
    /// A numeric reject field is malformed or overflows.
    InvalidNumber(FixTag),
}

impl fmt::Display for FixRejectParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMsgType => write!(f, "FIX message type is not a reject message"),
            Self::MissingTag(tag) => write!(f, "FIX reject message is missing tag {tag}"),
            Self::InvalidNumber(tag) => write!(f, "FIX reject numeric tag {tag} is invalid"),
        }
    }
}

impl Error for FixRejectParseError {}

/// Borrowed Session Reject `<3>` view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixSessionRejectView<'a> {
    pub(crate) ref_seq_num: u64,
    pub(crate) ref_tag_id: Option<FixTag>,
    pub(crate) ref_msg_type: Option<&'a [u8]>,
    pub(crate) session_reject_reason: Option<u64>,
    pub(crate) text: Option<&'a [u8]>,
}

impl<'a> FixSessionRejectView<'a> {
    /// Returns `RefSeqNum(45)`.
    pub const fn ref_seq_num(&self) -> u64 {
        self.ref_seq_num
    }

    /// Returns `RefTagID(371)` when present.
    pub const fn ref_tag_id(&self) -> Option<FixTag> {
        self.ref_tag_id
    }

    /// Returns `RefMsgType(372)` when present.
    pub const fn ref_msg_type(&self) -> Option<&'a [u8]> {
        self.ref_msg_type
    }

    /// Returns `SessionRejectReason(373)` when present.
    pub const fn session_reject_reason(&self) -> Option<u64> {
        self.session_reject_reason
    }

    /// Returns `Text(58)` when present.
    pub const fn text(&self) -> Option<&'a [u8]> {
        self.text
    }
}

/// Borrowed BusinessMessageReject `<j>` view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixBusinessMessageRejectView<'a> {
    pub(crate) ref_seq_num: Option<u64>,
    pub(crate) ref_msg_type: &'a [u8],
    pub(crate) business_reject_ref_id: Option<&'a [u8]>,
    pub(crate) business_reject_reason: u64,
    pub(crate) text: Option<&'a [u8]>,
}

impl<'a> FixBusinessMessageRejectView<'a> {
    /// Returns `RefSeqNum(45)` when present.
    pub const fn ref_seq_num(&self) -> Option<u64> {
        self.ref_seq_num
    }

    /// Returns required `RefMsgType(372)`.
    pub const fn ref_msg_type(&self) -> &'a [u8] {
        self.ref_msg_type
    }

    /// Returns `BusinessRejectRefID(379)` when present.
    pub const fn business_reject_ref_id(&self) -> Option<&'a [u8]> {
        self.business_reject_ref_id
    }

    /// Returns required `BusinessRejectReason(380)`.
    pub const fn business_reject_reason(&self) -> u64 {
        self.business_reject_reason
    }

    /// Returns `Text(58)` when present.
    pub const fn text(&self) -> Option<&'a [u8]> {
        self.text
    }
}
