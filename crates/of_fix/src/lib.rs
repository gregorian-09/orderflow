//! Low-allocation FIX tag-value codec primitives for Orderflow.
#![doc = include_str!("../README.md")]

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

/// FIX field delimiter byte.
pub const SOH: u8 = 0x01;

/// Numeric FIX tag identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixTag(pub u32);

impl FixTag {
    /// `BeginString(8)`.
    pub const BEGIN_STRING: Self = Self(8);
    /// `BodyLength(9)`.
    pub const BODY_LENGTH: Self = Self(9);
    /// `BeginSeqNo(7)`.
    pub const BEGIN_SEQ_NO: Self = Self(7);
    /// `EndSeqNo(16)`.
    pub const END_SEQ_NO: Self = Self(16);
    /// `MsgType(35)`.
    pub const MSG_TYPE: Self = Self(35);
    /// `MsgSeqNum(34)`.
    pub const MSG_SEQ_NUM: Self = Self(34);
    /// `NewSeqNo(36)`.
    pub const NEW_SEQ_NO: Self = Self(36);
    /// `PossDupFlag(43)`.
    pub const POSS_DUP_FLAG: Self = Self(43);
    /// `RefSeqNum(45)`.
    pub const REF_SEQ_NUM: Self = Self(45);
    /// `SenderCompID(49)`.
    pub const SENDER_COMP_ID: Self = Self(49);
    /// `SendingTime(52)`.
    pub const SENDING_TIME: Self = Self(52);
    /// `TargetCompID(56)`.
    pub const TARGET_COMP_ID: Self = Self(56);
    /// `ClOrdID(11)`.
    pub const CL_ORD_ID: Self = Self(11);
    /// `OrigClOrdID(41)`.
    pub const ORIG_CL_ORD_ID: Self = Self(41);
    /// `OrderID(37)`.
    pub const ORDER_ID: Self = Self(37);
    /// `ExecID(17)`.
    pub const EXEC_ID: Self = Self(17);
    /// `ExecType(150)`.
    pub const EXEC_TYPE: Self = Self(150);
    /// `OrdStatus(39)`.
    pub const ORD_STATUS: Self = Self(39);
    /// `Symbol(55)`.
    pub const SYMBOL: Self = Self(55);
    /// `Side(54)`.
    pub const SIDE: Self = Self(54);
    /// `OrderQty(38)`.
    pub const ORDER_QTY: Self = Self(38);
    /// `OrdType(40)`.
    pub const ORD_TYPE: Self = Self(40);
    /// `Price(44)`.
    pub const PRICE: Self = Self(44);
    /// `TimeInForce(59)`.
    pub const TIME_IN_FORCE: Self = Self(59);
    /// `LastQty(32)`.
    pub const LAST_QTY: Self = Self(32);
    /// `LastPx(31)`.
    pub const LAST_PX: Self = Self(31);
    /// `CumQty(14)`.
    pub const CUM_QTY: Self = Self(14);
    /// `LeavesQty(151)`.
    pub const LEAVES_QTY: Self = Self(151);
    /// `AvgPx(6)`.
    pub const AVG_PX: Self = Self(6);
    /// `TransactTime(60)`.
    pub const TRANSACT_TIME: Self = Self(60);
    /// `Text(58)`.
    pub const TEXT: Self = Self(58);
    /// `EncryptMethod(98)`.
    pub const ENCRYPT_METHOD: Self = Self(98);
    /// `TestReqID(112)`.
    pub const TEST_REQ_ID: Self = Self(112);
    /// `OrigSendingTime(122)`.
    pub const ORIG_SENDING_TIME: Self = Self(122);
    /// `HeartBtInt(108)`.
    pub const HEART_BT_INT: Self = Self(108);
    /// `GapFillFlag(123)`.
    pub const GAP_FILL_FLAG: Self = Self(123);
    /// `ResetSeqNumFlag(141)`.
    pub const RESET_SEQ_NUM_FLAG: Self = Self(141);
    /// `RefTagID(371)`.
    pub const REF_TAG_ID: Self = Self(371);
    /// `RefMsgType(372)`.
    pub const REF_MSG_TYPE: Self = Self(372);
    /// `SessionRejectReason(373)`.
    pub const SESSION_REJECT_REASON: Self = Self(373);
    /// `BusinessRejectRefID(379)`.
    pub const BUSINESS_REJECT_REF_ID: Self = Self(379);
    /// `BusinessRejectReason(380)`.
    pub const BUSINESS_REJECT_REASON: Self = Self(380);
    /// `CheckSum(10)`.
    pub const CHECK_SUM: Self = Self(10);
}

impl fmt::Display for FixTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Known FIX begin-string versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixVersion {
    /// FIX 4.0.
    Fix40,
    /// FIX 4.1.
    Fix41,
    /// FIX 4.2.
    Fix42,
    /// FIX 4.3.
    Fix43,
    /// FIX 4.4.
    Fix44,
    /// FIXT 1.1 transport session version.
    FixT11,
}

impl FixVersion {
    /// Returns the wire begin-string bytes.
    pub const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Fix40 => b"FIX.4.0",
            Self::Fix41 => b"FIX.4.1",
            Self::Fix42 => b"FIX.4.2",
            Self::Fix43 => b"FIX.4.3",
            Self::Fix44 => b"FIX.4.4",
            Self::FixT11 => b"FIXT.1.1",
        }
    }

    /// Parses a known begin-string version.
    pub fn from_bytes(value: &[u8]) -> Option<Self> {
        match value {
            b"FIX.4.0" => Some(Self::Fix40),
            b"FIX.4.1" => Some(Self::Fix41),
            b"FIX.4.2" => Some(Self::Fix42),
            b"FIX.4.3" => Some(Self::Fix43),
            b"FIX.4.4" => Some(Self::Fix44),
            b"FIXT.1.1" => Some(Self::FixT11),
            _ => None,
        }
    }
}

impl fmt::Display for FixVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(std::str::from_utf8(self.as_bytes()).unwrap_or("<invalid>"))
    }
}

/// FIX `MsgType(35)` identifier.
///
/// This type is intentionally a borrowed static byte wrapper so known message
/// types can be compared without allocation while custom profile-specific
/// message types can still be represented through [`FixMsgType::from_static`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixMsgType(&'static [u8]);

impl FixMsgType {
    /// `Heartbeat(0)`.
    pub const HEARTBEAT: Self = Self(b"0");
    /// `TestRequest(1)`.
    pub const TEST_REQUEST: Self = Self(b"1");
    /// `ResendRequest(2)`.
    pub const RESEND_REQUEST: Self = Self(b"2");
    /// `Reject(3)`.
    pub const REJECT: Self = Self(b"3");
    /// `SequenceReset(4)`.
    pub const SEQUENCE_RESET: Self = Self(b"4");
    /// `Logout(5)`.
    pub const LOGOUT: Self = Self(b"5");
    /// `ExecutionReport(8)`.
    pub const EXECUTION_REPORT: Self = Self(b"8");
    /// `OrderCancelReject(9)`.
    pub const ORDER_CANCEL_REJECT: Self = Self(b"9");
    /// `Logon(A)`.
    pub const LOGON: Self = Self(b"A");
    /// `NewOrderSingle(D)`.
    pub const NEW_ORDER_SINGLE: Self = Self(b"D");
    /// `OrderCancelRequest(F)`.
    pub const ORDER_CANCEL_REQUEST: Self = Self(b"F");
    /// `OrderCancelReplaceRequest(G)`.
    pub const ORDER_CANCEL_REPLACE_REQUEST: Self = Self(b"G");
    /// `OrderStatusRequest(H)`.
    pub const ORDER_STATUS_REQUEST: Self = Self(b"H");
    /// `BusinessMessageReject(j)`.
    pub const BUSINESS_MESSAGE_REJECT: Self = Self(b"j");

    /// Creates a message type from a static byte slice.
    ///
    /// Use this for venue-defined or extension-pack message types when a
    /// dictionary/profile wants to validate them without allocating.
    pub const fn from_static(value: &'static [u8]) -> Self {
        Self(value)
    }

    /// Parses a known message type.
    pub fn from_bytes(value: &[u8]) -> Option<Self> {
        match value {
            b"0" => Some(Self::HEARTBEAT),
            b"1" => Some(Self::TEST_REQUEST),
            b"2" => Some(Self::RESEND_REQUEST),
            b"3" => Some(Self::REJECT),
            b"4" => Some(Self::SEQUENCE_RESET),
            b"5" => Some(Self::LOGOUT),
            b"8" => Some(Self::EXECUTION_REPORT),
            b"9" => Some(Self::ORDER_CANCEL_REJECT),
            b"A" => Some(Self::LOGON),
            b"D" => Some(Self::NEW_ORDER_SINGLE),
            b"F" => Some(Self::ORDER_CANCEL_REQUEST),
            b"G" => Some(Self::ORDER_CANCEL_REPLACE_REQUEST),
            b"H" => Some(Self::ORDER_STATUS_REQUEST),
            b"j" => Some(Self::BUSINESS_MESSAGE_REJECT),
            _ => None,
        }
    }

    /// Returns the wire message-type bytes.
    pub const fn as_bytes(self) -> &'static [u8] {
        self.0
    }

    /// Returns a human-readable message type name for diagnostics.
    pub fn name(self) -> &'static str {
        match self.0 {
            b"0" => "Heartbeat",
            b"1" => "TestRequest",
            b"2" => "ResendRequest",
            b"3" => "Reject",
            b"4" => "SequenceReset",
            b"5" => "Logout",
            b"8" => "ExecutionReport",
            b"9" => "OrderCancelReject",
            b"A" => "Logon",
            b"D" => "NewOrderSingle",
            b"F" => "OrderCancelRequest",
            b"G" => "OrderCancelReplaceRequest",
            b"H" => "OrderStatusRequest",
            b"j" => "BusinessMessageReject",
            _ => "Custom",
        }
    }
}

impl fmt::Display for FixMsgType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(std::str::from_utf8(self.0).unwrap_or("<invalid>"))
    }
}

/// Common FIX `Side(54)` values for order-entry builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixOrderSide {
    /// Buy (`1`).
    Buy,
    /// Sell (`2`).
    Sell,
    /// Sell short (`5`).
    SellShort,
}

impl FixOrderSide {
    /// Returns the wire value.
    pub const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Buy => b"1",
            Self::Sell => b"2",
            Self::SellShort => b"5",
        }
    }

    /// Parses a common side value.
    pub fn from_bytes(value: &[u8]) -> Option<Self> {
        match value {
            b"1" => Some(Self::Buy),
            b"2" => Some(Self::Sell),
            b"5" => Some(Self::SellShort),
            _ => None,
        }
    }
}

/// Common FIX `OrdType(40)` values for order-entry builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixOrdType {
    /// Market (`1`).
    Market,
    /// Limit (`2`).
    Limit,
    /// Stop (`3`).
    Stop,
    /// Stop limit (`4`).
    StopLimit,
}

impl FixOrdType {
    /// Returns the wire value.
    pub const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Market => b"1",
            Self::Limit => b"2",
            Self::Stop => b"3",
            Self::StopLimit => b"4",
        }
    }

    /// Parses a common order type.
    pub fn from_bytes(value: &[u8]) -> Option<Self> {
        match value {
            b"1" => Some(Self::Market),
            b"2" => Some(Self::Limit),
            b"3" => Some(Self::Stop),
            b"4" => Some(Self::StopLimit),
            _ => None,
        }
    }
}

/// Common FIX `TimeInForce(59)` values for order-entry builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixTimeInForce {
    /// Day (`0`).
    Day,
    /// Good till cancel (`1`).
    GoodTillCancel,
    /// Immediate or cancel (`3`).
    ImmediateOrCancel,
    /// Fill or kill (`4`).
    FillOrKill,
    /// Good till date (`6`).
    GoodTillDate,
}

impl FixTimeInForce {
    /// Returns the wire value.
    pub const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Day => b"0",
            Self::GoodTillCancel => b"1",
            Self::ImmediateOrCancel => b"3",
            Self::FillOrKill => b"4",
            Self::GoodTillDate => b"6",
        }
    }

    /// Parses a common time-in-force value.
    pub fn from_bytes(value: &[u8]) -> Option<Self> {
        match value {
            b"0" => Some(Self::Day),
            b"1" => Some(Self::GoodTillCancel),
            b"3" => Some(Self::ImmediateOrCancel),
            b"4" => Some(Self::FillOrKill),
            b"6" => Some(Self::GoodTillDate),
            _ => None,
        }
    }
}

/// Borrowed FIX tag-value field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixFieldView<'a> {
    /// Numeric FIX tag.
    pub tag: FixTag,
    /// Borrowed field value bytes.
    pub value: &'a [u8],
}

impl<'a> FixFieldView<'a> {
    /// Creates an empty field placeholder for scratch buffers.
    pub const fn empty() -> Self {
        Self {
            tag: FixTag(0),
            value: &[],
        }
    }
}

/// Borrowed view over a validated FIX message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixMessageView<'a> {
    raw: &'a [u8],
    fields: &'a [FixFieldView<'a>],
}

impl<'a> FixMessageView<'a> {
    /// Returns the raw FIX frame bytes.
    pub const fn raw(&self) -> &'a [u8] {
        self.raw
    }

    /// Returns parsed fields in wire order.
    pub const fn fields(&self) -> &'a [FixFieldView<'a>] {
        self.fields
    }

    /// Returns the first field value for `tag`.
    pub fn get(&self, tag: FixTag) -> Option<&'a [u8]> {
        self.fields
            .iter()
            .find(|field| field.tag == tag)
            .map(|field| field.value)
    }

    /// Returns `MsgType(35)`.
    pub fn msg_type(&self) -> Option<&'a [u8]> {
        self.get(FixTag::MSG_TYPE)
    }

    /// Returns `MsgType(35)` as a known typed message kind when recognized.
    pub fn typed_msg_type(&self) -> Option<FixMsgType> {
        FixMsgType::from_bytes(self.msg_type()?)
    }

    /// Returns `BeginString(8)`.
    pub fn begin_string(&self) -> Option<&'a [u8]> {
        self.get(FixTag::BEGIN_STRING)
    }

    /// Returns `BeginString(8)` as a known FIX version when recognized.
    pub fn version(&self) -> Option<FixVersion> {
        FixVersion::from_bytes(self.begin_string()?)
    }

    /// Returns `MsgSeqNum(34)` parsed as `u64`.
    pub fn msg_seq_num(&self) -> Option<u64> {
        parse_u64(self.get(FixTag::MSG_SEQ_NUM)?).ok()
    }

    /// Returns true when `PossDupFlag(43)` is `Y`.
    pub fn poss_dup(&self) -> bool {
        self.get(FixTag::POSS_DUP_FLAG) == Some(b"Y".as_slice())
    }

    /// Returns true when `GapFillFlag(123)` is `Y`.
    pub fn gap_fill(&self) -> bool {
        self.get(FixTag::GAP_FILL_FLAG) == Some(b"Y".as_slice())
    }

    /// Returns `NewSeqNo(36)` parsed as `u64`.
    pub fn new_seq_no(&self) -> Option<u64> {
        parse_u64(self.get(FixTag::NEW_SEQ_NO)?).ok()
    }

    /// Returns `BeginSeqNo(7)` parsed as `u64`.
    pub fn begin_seq_no(&self) -> Option<u64> {
        parse_u64(self.get(FixTag::BEGIN_SEQ_NO)?).ok()
    }

    /// Returns `EndSeqNo(16)` parsed as `u64`.
    pub fn end_seq_no(&self) -> Option<u64> {
        parse_u64(self.get(FixTag::END_SEQ_NO)?).ok()
    }

    /// Renders a debug string with `|` separators instead of SOH.
    ///
    /// This allocates and is intended for diagnostics, tests, and transcript
    /// output, not the live execution hot path.
    pub fn debug_render(&self) -> String {
        debug_render(self.raw)
    }
}

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
}

impl fmt::Display for FixEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueContainsSoh(tag) => write!(f, "FIX value for tag {tag} contains SOH"),
            Self::ReservedTag(tag) => write!(f, "FIX tag {tag} is owned by the encoder"),
            Self::MissingRequiredTag(tag) => write!(f, "FIX source message is missing tag {tag}"),
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
    ref_seq_num: u64,
    ref_tag_id: Option<FixTag>,
    ref_msg_type: Option<&'a [u8]>,
    session_reject_reason: Option<u64>,
    text: Option<&'a [u8]>,
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
    ref_seq_num: Option<u64>,
    ref_msg_type: &'a [u8],
    business_reject_ref_id: Option<&'a [u8]>,
    business_reject_reason: u64,
    text: Option<&'a [u8]>,
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

/// FIX session lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixSessionState {
    /// No active transport connection exists.
    Disconnected,
    /// Transport connection attempt is in progress.
    Connecting,
    /// Logon has been sent and the session is awaiting acceptance.
    LogonSent,
    /// Session is active and can process application flow.
    Ready,
    /// A resend request has been emitted and the session is waiting for gap
    /// recovery.
    ResendRequested,
    /// Session is applying recovery messages or sequence resets.
    Recovering,
    /// Logout has been sent and the session is draining.
    LogoutSent,
    /// Session has been intentionally stopped.
    Stopped,
    /// Session is alive but operating under a degraded policy.
    Degraded,
}

/// Resend range requested after an inbound sequence gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixResendRange {
    /// First missing sequence number.
    pub begin_seq_no: u64,
    /// Last missing sequence number.
    pub end_seq_no: u64,
}

/// Result of observing an inbound sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSequenceAction {
    /// The sequence number matched the expected inbound value and should be
    /// processed normally.
    Accept {
        /// Accepted inbound sequence number.
        seq_no: u64,
    },
    /// The sequence number is lower than expected and carries
    /// `PossDupFlag(43)=Y`, so the caller should ignore it if already applied.
    Duplicate {
        /// Duplicate inbound sequence number.
        seq_no: u64,
        /// Current expected inbound sequence number.
        expected: u64,
    },
    /// The sequence number is higher than expected and a resend request should
    /// be emitted for the missing range.
    Gap {
        /// Current expected inbound sequence number.
        expected: u64,
        /// Received inbound sequence number.
        received: u64,
        /// Missing range to request.
        resend: FixResendRange,
    },
    /// The sequence number is lower than expected without a duplicate marker.
    TooLow {
        /// Current expected inbound sequence number.
        expected: u64,
        /// Received inbound sequence number.
        received: u64,
    },
}

/// FIX sequence tracking errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSequenceError {
    /// `MsgSeqNum(34)` is missing.
    MissingMsgSeqNum,
    /// A sequence number was zero. FIX sequence numbers are one-based.
    ZeroSeqNo,
    /// A sequence reset attempted to lower the expected inbound sequence.
    SequenceResetWouldDecrease {
        /// Current expected inbound sequence number.
        current: u64,
        /// Requested new expected inbound sequence number.
        requested: u64,
    },
}

impl fmt::Display for FixSequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMsgSeqNum => write!(f, "FIX MsgSeqNum(34) is missing"),
            Self::ZeroSeqNo => write!(f, "FIX sequence number must be greater than zero"),
            Self::SequenceResetWouldDecrease { current, requested } => write!(
                f,
                "FIX sequence reset would decrease next inbound sequence: current {current}, requested {requested}"
            ),
        }
    }
}

impl Error for FixSequenceError {}

/// Borrowed FIX session identity.
///
/// A FIX session is commonly identified by begin string, sender, target, and an
/// optional qualifier used to disambiguate otherwise identical sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixSessionId<'a> {
    version: FixVersion,
    sender_comp_id: &'a [u8],
    target_comp_id: &'a [u8],
    qualifier: &'a [u8],
}

impl<'a> FixSessionId<'a> {
    /// Creates a session id without a qualifier.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when a value contains SOH.
    pub fn new(
        version: FixVersion,
        sender_comp_id: &'a [u8],
        target_comp_id: &'a [u8],
    ) -> Result<Self, FixEncodeError> {
        Self::with_qualifier(version, sender_comp_id, target_comp_id, b"")
    }

    /// Creates a session id with an optional qualifier.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when a value contains SOH.
    pub fn with_qualifier(
        version: FixVersion,
        sender_comp_id: &'a [u8],
        target_comp_id: &'a [u8],
        qualifier: &'a [u8],
    ) -> Result<Self, FixEncodeError> {
        validate_value(FixTag::SENDER_COMP_ID, sender_comp_id)?;
        validate_value(FixTag::TARGET_COMP_ID, target_comp_id)?;
        validate_value(FixTag::TEXT, qualifier)?;
        Ok(Self {
            version,
            sender_comp_id,
            target_comp_id,
            qualifier,
        })
    }

    /// Returns the FIX version.
    pub const fn version(&self) -> FixVersion {
        self.version
    }

    /// Returns `SenderCompID(49)`.
    pub const fn sender_comp_id(&self) -> &'a [u8] {
        self.sender_comp_id
    }

    /// Returns `TargetCompID(56)`.
    pub const fn target_comp_id(&self) -> &'a [u8] {
        self.target_comp_id
    }

    /// Returns the optional session qualifier bytes.
    pub const fn qualifier(&self) -> &'a [u8] {
        self.qualifier
    }
}

/// Borrowed persistable sequence-state snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixSequenceSnapshot<'a> {
    session_id: FixSessionId<'a>,
    next_inbound: u64,
    next_outbound: u64,
    trading_day: &'a [u8],
}

impl<'a> FixSequenceSnapshot<'a> {
    /// Creates a sequence snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when `trading_day` contains SOH.
    pub fn new(
        session_id: FixSessionId<'a>,
        next_inbound: u64,
        next_outbound: u64,
        trading_day: &'a [u8],
    ) -> Result<Self, FixEncodeError> {
        validate_value(FixTag::TEXT, trading_day)?;
        Ok(Self {
            session_id,
            next_inbound: clamp_seq_no(next_inbound),
            next_outbound: clamp_seq_no(next_outbound),
            trading_day,
        })
    }

    /// Returns the session id.
    pub const fn session_id(&self) -> FixSessionId<'a> {
        self.session_id
    }

    /// Returns the next inbound sequence number.
    pub const fn next_inbound(&self) -> u64 {
        self.next_inbound
    }

    /// Returns the next outbound sequence number.
    pub const fn next_outbound(&self) -> u64 {
        self.next_outbound
    }

    /// Returns the trading day or session date bytes.
    pub const fn trading_day(&self) -> &'a [u8] {
        self.trading_day
    }
}

/// Classification for outbound messages retained for resend handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixSentMessageKind {
    /// Application-level order or execution-flow message that may be replayed.
    Application,
    /// Session administrative message that should normally be gap-filled.
    Administrative,
    /// Session-level reject. FIX recovery rules allow reject messages to be
    /// replayed when a profile chooses to retain them.
    Reject,
}

impl FixSentMessageKind {
    /// Returns whether this message kind is replayable by default.
    pub const fn replayable(self) -> bool {
        matches!(self, Self::Application | Self::Reject)
    }
}

/// Bounded resend-store configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixResendStoreConfig {
    max_messages: usize,
    max_bytes: usize,
}

impl FixResendStoreConfig {
    /// Creates a bounded resend-store configuration.
    ///
    /// A zero `max_messages` or `max_bytes` disables retention while keeping
    /// counters observable through [`FixResendStore::metrics`].
    pub const fn new(max_messages: usize, max_bytes: usize) -> Self {
        Self {
            max_messages,
            max_bytes,
        }
    }

    /// Returns the maximum retained message count.
    pub const fn max_messages(&self) -> usize {
        self.max_messages
    }

    /// Returns the maximum retained raw-byte count.
    pub const fn max_bytes(&self) -> usize {
        self.max_bytes
    }
}

impl Default for FixResendStoreConfig {
    fn default() -> Self {
        Self {
            max_messages: 1024,
            max_bytes: 1024 * 1024,
        }
    }
}

/// Resend-store append errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixResendStoreError {
    /// FIX sequence numbers are one-based.
    ZeroSeqNo,
    /// A retained outbound message was recorded out of order or reused a
    /// sequence number already observed by the store.
    SequenceRegression {
        /// Latest retained or observed outbound sequence.
        latest: u64,
        /// Sequence number supplied by the caller.
        received: u64,
    },
}

impl fmt::Display for FixResendStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSeqNo => write!(
                f,
                "FIX resend store sequence number must be greater than zero"
            ),
            Self::SequenceRegression { latest, received } => write!(
                f,
                "FIX resend store sequence regression: latest {latest}, received {received}"
            ),
        }
    }
}

impl Error for FixResendStoreError {}

/// Retained outbound FIX frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixStoredMessage {
    seq_no: u64,
    kind: FixSentMessageKind,
    raw: Vec<u8>,
}

impl FixStoredMessage {
    /// Returns the outbound `MsgSeqNum(34)`.
    pub const fn seq_no(&self) -> u64 {
        self.seq_no
    }

    /// Returns the retained message kind.
    pub const fn kind(&self) -> FixSentMessageKind {
        self.kind
    }

    /// Returns the retained raw FIX frame.
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// Returns whether the message is replayable by default.
    pub const fn replayable(&self) -> bool {
        self.kind.replayable()
    }
}

/// Result of recording a sent message into a resend store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixResendRetention {
    retained: bool,
    evicted_messages: u64,
    evicted_bytes: u64,
}

impl FixResendRetention {
    /// Returns whether the message was retained.
    pub const fn retained(&self) -> bool {
        self.retained
    }

    /// Returns messages evicted to satisfy configured bounds.
    pub const fn evicted_messages(&self) -> u64 {
        self.evicted_messages
    }

    /// Returns bytes evicted to satisfy configured bounds.
    pub const fn evicted_bytes(&self) -> u64 {
        self.evicted_bytes
    }
}

/// Snapshot of resend-store counters and retained range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixResendStoreMetrics {
    retained_messages: u64,
    retained_bytes: u64,
    dropped_messages: u64,
    dropped_bytes: u64,
    evicted_messages: u64,
    evicted_bytes: u64,
    oldest_seq_no: Option<u64>,
    newest_seq_no: Option<u64>,
}

impl FixResendStoreMetrics {
    /// Returns the number of retained messages.
    pub const fn retained_messages(&self) -> u64 {
        self.retained_messages
    }

    /// Returns the number of retained raw bytes.
    pub const fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// Returns messages dropped because retention was disabled or the frame
    /// exceeded the byte budget.
    pub const fn dropped_messages(&self) -> u64 {
        self.dropped_messages
    }

    /// Returns bytes dropped because retention was disabled or the frame
    /// exceeded the byte budget.
    pub const fn dropped_bytes(&self) -> u64 {
        self.dropped_bytes
    }

    /// Returns messages evicted by bounded retention.
    pub const fn evicted_messages(&self) -> u64 {
        self.evicted_messages
    }

    /// Returns bytes evicted by bounded retention.
    pub const fn evicted_bytes(&self) -> u64 {
        self.evicted_bytes
    }

    /// Returns the oldest retained outbound sequence number.
    pub const fn oldest_seq_no(&self) -> Option<u64> {
        self.oldest_seq_no
    }

    /// Returns the newest observed outbound sequence number.
    pub const fn newest_seq_no(&self) -> Option<u64> {
        self.newest_seq_no
    }
}

/// One planned response for an outbound resend request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixResendAction<'a> {
    /// Replay a retained application or reject frame.
    Replay {
        /// Original outbound sequence number.
        seq_no: u64,
        /// Retained raw FIX frame.
        raw: &'a [u8],
    },
    /// Emit a SequenceReset `<4>` gap-fill for an inclusive sequence range.
    GapFill {
        /// First skipped sequence number.
        begin_seq_no: u64,
        /// Last skipped sequence number.
        end_seq_no: u64,
    },
}

/// Summary produced while planning a resend response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixResendPlanSummary {
    replay_messages: u64,
    gap_fill_messages: u64,
    gap_fill_sequences: u64,
}

impl FixResendPlanSummary {
    /// Returns replay actions produced by the planner.
    pub const fn replay_messages(&self) -> u64 {
        self.replay_messages
    }

    /// Returns gap-fill actions produced by the planner.
    pub const fn gap_fill_messages(&self) -> u64 {
        self.gap_fill_messages
    }

    /// Returns total skipped sequence numbers covered by gap fills.
    pub const fn gap_fill_sequences(&self) -> u64 {
        self.gap_fill_sequences
    }
}

/// Bounded in-memory FIX resend store.
///
/// The store is intentionally storage-neutral. It retains outbound raw frames
/// in memory for fast resend planning, while durable session stores can persist
/// the same frames separately according to their own latency and sync policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixResendStore {
    config: FixResendStoreConfig,
    messages: VecDeque<FixStoredMessage>,
    retained_bytes: usize,
    dropped_messages: u64,
    dropped_bytes: u64,
    evicted_messages: u64,
    evicted_bytes: u64,
    newest_seq_no: Option<u64>,
}

impl Default for FixResendStore {
    fn default() -> Self {
        Self::new(FixResendStoreConfig::default())
    }
}

impl FixResendStore {
    /// Creates an empty resend store.
    pub fn new(config: FixResendStoreConfig) -> Self {
        Self {
            config,
            messages: VecDeque::with_capacity(config.max_messages.min(64)),
            retained_bytes: 0,
            dropped_messages: 0,
            dropped_bytes: 0,
            evicted_messages: 0,
            evicted_bytes: 0,
            newest_seq_no: None,
        }
    }

    /// Returns the configured retention bounds.
    pub const fn config(&self) -> FixResendStoreConfig {
        self.config
    }

    /// Returns retained messages in sequence order.
    pub fn messages(&self) -> impl Iterator<Item = &FixStoredMessage> {
        self.messages.iter()
    }

    /// Returns a retained message by outbound sequence number.
    pub fn get(&self, seq_no: u64) -> Option<&FixStoredMessage> {
        self.messages
            .iter()
            .find(|message| message.seq_no == seq_no)
    }

    /// Records a sent outbound frame.
    ///
    /// Call this only for original outbound sends. Retransmitted `PossDupFlag`
    /// messages should not be recorded as new sends because their original
    /// sequence number may be lower than the latest observed sequence.
    ///
    /// # Errors
    ///
    /// Returns [`FixResendStoreError`] when `seq_no` is zero or lower than the
    /// latest sequence already observed by this store.
    pub fn record_sent(
        &mut self,
        seq_no: u64,
        kind: FixSentMessageKind,
        raw: &[u8],
    ) -> Result<FixResendRetention, FixResendStoreError> {
        if seq_no == 0 {
            return Err(FixResendStoreError::ZeroSeqNo);
        }
        if let Some(latest) = self.newest_seq_no {
            if seq_no <= latest {
                return Err(FixResendStoreError::SequenceRegression {
                    latest,
                    received: seq_no,
                });
            }
        }
        self.newest_seq_no = Some(seq_no);

        if self.config.max_messages == 0
            || self.config.max_bytes == 0
            || raw.len() > self.config.max_bytes
        {
            self.dropped_messages = self.dropped_messages.saturating_add(1);
            self.dropped_bytes = self.dropped_bytes.saturating_add(raw.len() as u64);
            return Ok(FixResendRetention {
                retained: false,
                evicted_messages: 0,
                evicted_bytes: 0,
            });
        }

        self.messages.push_back(FixStoredMessage {
            seq_no,
            kind,
            raw: raw.to_vec(),
        });
        self.retained_bytes = self.retained_bytes.saturating_add(raw.len());

        let mut evicted_messages = 0_u64;
        let mut evicted_bytes = 0_u64;
        while self.messages.len() > self.config.max_messages
            || self.retained_bytes > self.config.max_bytes
        {
            if let Some(evicted) = self.messages.pop_front() {
                let len = evicted.raw.len();
                self.retained_bytes = self.retained_bytes.saturating_sub(len);
                evicted_messages = evicted_messages.saturating_add(1);
                evicted_bytes = evicted_bytes.saturating_add(len as u64);
            } else {
                break;
            }
        }
        self.evicted_messages = self.evicted_messages.saturating_add(evicted_messages);
        self.evicted_bytes = self.evicted_bytes.saturating_add(evicted_bytes);

        Ok(FixResendRetention {
            retained: true,
            evicted_messages,
            evicted_bytes,
        })
    }

    /// Plans replay and gap-fill actions for an inclusive resend range.
    ///
    /// `EndSeqNo(16)=0` is interpreted as "through the newest observed outbound
    /// sequence" for the purpose of bounded planning. The `out` vector is
    /// cleared before actions are appended.
    pub fn plan_resend_range<'a>(
        &'a self,
        range: FixResendRange,
        out: &mut Vec<FixResendAction<'a>>,
    ) -> FixResendPlanSummary {
        out.clear();
        let Some(end_seq_no) = self.resolve_resend_end(range.end_seq_no) else {
            return FixResendPlanSummary {
                replay_messages: 0,
                gap_fill_messages: 0,
                gap_fill_sequences: 0,
            };
        };
        if range.begin_seq_no == 0 || range.begin_seq_no > end_seq_no {
            return FixResendPlanSummary {
                replay_messages: 0,
                gap_fill_messages: 0,
                gap_fill_sequences: 0,
            };
        }

        let mut cursor = range.begin_seq_no;
        let mut replay_messages = 0_u64;
        let mut gap_fill_messages = 0_u64;
        let mut gap_fill_sequences = 0_u64;

        for message in self
            .messages
            .iter()
            .filter(|message| message.seq_no >= range.begin_seq_no && message.seq_no <= end_seq_no)
        {
            if !message.replayable() {
                continue;
            }
            if cursor < message.seq_no {
                push_gap_fill(
                    out,
                    cursor,
                    message.seq_no.saturating_sub(1),
                    &mut gap_fill_messages,
                    &mut gap_fill_sequences,
                );
            }
            out.push(FixResendAction::Replay {
                seq_no: message.seq_no,
                raw: message.raw(),
            });
            replay_messages = replay_messages.saturating_add(1);
            cursor = message.seq_no.saturating_add(1);
        }

        if cursor <= end_seq_no {
            push_gap_fill(
                out,
                cursor,
                end_seq_no,
                &mut gap_fill_messages,
                &mut gap_fill_sequences,
            );
        }

        FixResendPlanSummary {
            replay_messages,
            gap_fill_messages,
            gap_fill_sequences,
        }
    }

    /// Returns resend-store metrics.
    pub fn metrics(&self) -> FixResendStoreMetrics {
        FixResendStoreMetrics {
            retained_messages: self.messages.len() as u64,
            retained_bytes: self.retained_bytes as u64,
            dropped_messages: self.dropped_messages,
            dropped_bytes: self.dropped_bytes,
            evicted_messages: self.evicted_messages,
            evicted_bytes: self.evicted_bytes,
            oldest_seq_no: self.messages.front().map(|message| message.seq_no),
            newest_seq_no: self.newest_seq_no,
        }
    }

    fn resolve_resend_end(&self, requested_end: u64) -> Option<u64> {
        if requested_end == 0 {
            self.newest_seq_no
        } else {
            Some(requested_end)
        }
    }
}

/// Deterministic inbound/outbound FIX sequence tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixSequenceTracker {
    next_inbound: u64,
    next_outbound: u64,
}

impl Default for FixSequenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl FixSequenceTracker {
    /// Creates a tracker with both inbound and outbound sequence numbers set to
    /// `1`.
    pub const fn new() -> Self {
        Self {
            next_inbound: 1,
            next_outbound: 1,
        }
    }

    /// Creates a tracker from persisted next inbound and outbound values.
    ///
    /// Values lower than `1` are clamped to `1` so restored state remains valid.
    pub const fn from_next(next_inbound: u64, next_outbound: u64) -> Self {
        Self {
            next_inbound: clamp_seq_no(next_inbound),
            next_outbound: clamp_seq_no(next_outbound),
        }
    }

    /// Returns the next inbound sequence number expected from the counterparty.
    pub const fn next_inbound(&self) -> u64 {
        self.next_inbound
    }

    /// Returns the next outbound sequence number to assign.
    pub const fn next_outbound(&self) -> u64 {
        self.next_outbound
    }

    /// Assigns and advances the next outbound sequence number.
    pub fn assign_outbound(&mut self) -> u64 {
        let seq_no = self.next_outbound;
        self.next_outbound = self.next_outbound.saturating_add(1);
        seq_no
    }

    /// Observes an inbound message and returns the sequence action.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceError`] when `MsgSeqNum(34)` is missing or zero.
    pub fn observe_message(
        &mut self,
        message: &FixMessageView<'_>,
    ) -> Result<FixSequenceAction, FixSequenceError> {
        let seq_no = message
            .msg_seq_num()
            .ok_or(FixSequenceError::MissingMsgSeqNum)?;
        self.observe_inbound(seq_no, message.poss_dup())
    }

    /// Observes an inbound sequence number.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceError::ZeroSeqNo`] when `seq_no` is zero.
    pub fn observe_inbound(
        &mut self,
        seq_no: u64,
        poss_dup: bool,
    ) -> Result<FixSequenceAction, FixSequenceError> {
        if seq_no == 0 {
            return Err(FixSequenceError::ZeroSeqNo);
        }

        let expected = self.next_inbound;
        if seq_no == expected {
            self.next_inbound = self.next_inbound.saturating_add(1);
            Ok(FixSequenceAction::Accept { seq_no })
        } else if seq_no > expected {
            Ok(FixSequenceAction::Gap {
                expected,
                received: seq_no,
                resend: FixResendRange {
                    begin_seq_no: expected,
                    end_seq_no: seq_no.saturating_sub(1),
                },
            })
        } else if poss_dup {
            Ok(FixSequenceAction::Duplicate { seq_no, expected })
        } else {
            Ok(FixSequenceAction::TooLow {
                expected,
                received: seq_no,
            })
        }
    }

    /// Applies `NewSeqNo(36)` as the next expected inbound sequence number.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceError::ZeroSeqNo`] for zero and
    /// [`FixSequenceError::SequenceResetWouldDecrease`] when the reset would
    /// lower the current expected sequence number.
    pub fn apply_sequence_reset(&mut self, new_seq_no: u64) -> Result<(), FixSequenceError> {
        if new_seq_no == 0 {
            return Err(FixSequenceError::ZeroSeqNo);
        }
        if new_seq_no < self.next_inbound {
            return Err(FixSequenceError::SequenceResetWouldDecrease {
                current: self.next_inbound,
                requested: new_seq_no,
            });
        }
        self.next_inbound = new_seq_no;
        Ok(())
    }

    /// Sets the next inbound sequence number from trusted persisted state.
    pub fn set_next_inbound(&mut self, next_inbound: u64) {
        self.next_inbound = clamp_seq_no(next_inbound);
    }

    /// Sets the next outbound sequence number from trusted persisted state.
    pub fn set_next_outbound(&mut self, next_outbound: u64) {
        self.next_outbound = clamp_seq_no(next_outbound);
    }

    /// Creates a persistable snapshot for this tracker.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when `trading_day` contains SOH.
    pub fn snapshot<'a>(
        &self,
        session_id: FixSessionId<'a>,
        trading_day: &'a [u8],
    ) -> Result<FixSequenceSnapshot<'a>, FixEncodeError> {
        FixSequenceSnapshot::new(
            session_id,
            self.next_inbound,
            self.next_outbound,
            trading_day,
        )
    }

    /// Restores tracker counters from a sequence snapshot.
    pub const fn from_snapshot(snapshot: &FixSequenceSnapshot<'_>) -> Self {
        Self::from_next(snapshot.next_inbound(), snapshot.next_outbound())
    }

    /// Resets both inbound and outbound counters to `1`.
    pub fn reset_to_one(&mut self) {
        self.next_inbound = 1;
        self.next_outbound = 1;
    }
}

/// Validation rule for one FIX message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixMessageRule<'a> {
    msg_type: FixMsgType,
    required_tags: &'a [FixTag],
    disallowed_tags: &'a [FixTag],
}

impl<'a> FixMessageRule<'a> {
    /// Creates a validation rule for a message type.
    pub const fn new(
        msg_type: FixMsgType,
        required_tags: &'a [FixTag],
        disallowed_tags: &'a [FixTag],
    ) -> Self {
        Self {
            msg_type,
            required_tags,
            disallowed_tags,
        }
    }

    /// Returns the message type this rule validates.
    pub const fn msg_type(&self) -> FixMsgType {
        self.msg_type
    }

    /// Returns tags required by this rule.
    pub const fn required_tags(&self) -> &'a [FixTag] {
        self.required_tags
    }

    /// Returns tags disallowed by this rule.
    pub const fn disallowed_tags(&self) -> &'a [FixTag] {
        self.disallowed_tags
    }

    /// Validates a parsed message against this rule.
    ///
    /// # Errors
    ///
    /// Returns [`FixProfileError`] when a required tag is missing or a
    /// disallowed tag is present.
    pub fn validate(&self, message: &FixMessageView<'_>) -> Result<(), FixProfileError> {
        for tag in self.required_tags {
            if message.get(*tag).is_none() {
                return Err(FixProfileError::MissingRequiredTag {
                    msg_type: self.msg_type,
                    tag: *tag,
                });
            }
        }
        for tag in self.disallowed_tags {
            if message.get(*tag).is_some() {
                return Err(FixProfileError::DisallowedTag {
                    msg_type: self.msg_type,
                    tag: *tag,
                });
            }
        }
        Ok(())
    }
}

/// Static FIX dictionary/profile used for message-level validation.
///
/// This type intentionally borrows rule slices so users can precompute
/// dictionaries once and share them without per-message allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixDictionary<'a> {
    version: FixVersion,
    rules: &'a [FixMessageRule<'a>],
}

impl<'a> FixDictionary<'a> {
    /// Creates a dictionary for `version` and static message rules.
    pub const fn new(version: FixVersion, rules: &'a [FixMessageRule<'a>]) -> Self {
        Self { version, rules }
    }

    /// Returns the FIX version this dictionary accepts.
    pub const fn version(&self) -> FixVersion {
        self.version
    }

    /// Returns all message rules.
    pub const fn rules(&self) -> &'a [FixMessageRule<'a>] {
        self.rules
    }

    /// Finds a rule by typed message type.
    pub fn rule_for(&self, msg_type: FixMsgType) -> Option<&'a FixMessageRule<'a>> {
        self.rules.iter().find(|rule| rule.msg_type == msg_type)
    }

    /// Finds a rule by raw `MsgType(35)` bytes.
    pub fn rule_for_bytes(&self, msg_type: &[u8]) -> Option<&'a FixMessageRule<'a>> {
        self.rules
            .iter()
            .find(|rule| rule.msg_type.as_bytes() == msg_type)
    }

    /// Validates a parsed message against the dictionary.
    ///
    /// # Errors
    ///
    /// Returns [`FixProfileError`] when the version, message type, or
    /// message-level rule validation fails.
    pub fn validate(&self, message: &FixMessageView<'_>) -> Result<(), FixProfileError> {
        let actual_version = message
            .begin_string()
            .ok_or(FixProfileError::MissingBeginString)
            .and_then(|value| {
                FixVersion::from_bytes(value).ok_or(FixProfileError::UnsupportedVersion)
            })?;
        if actual_version != self.version {
            return Err(FixProfileError::VersionMismatch {
                expected: self.version,
                actual: actual_version,
            });
        }

        let msg_type = message.msg_type().ok_or(FixProfileError::MissingMsgType)?;
        let rule = self
            .rule_for_bytes(msg_type)
            .ok_or(FixProfileError::UnsupportedMsgType)?;
        rule.validate(message)
    }
}

/// Stateless FIX decoder facade.
///
/// Use this when a component wants an explicit decoder object while still
/// keeping field storage caller-owned.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FixDecoder;

impl FixDecoder {
    /// Creates a decoder facade.
    pub const fn new() -> Self {
        Self
    }

    /// Parses and validates a FIX message into caller-provided scratch.
    ///
    /// # Errors
    ///
    /// Returns [`FixParseError`] for malformed frames, validation failures, or
    /// insufficient scratch capacity.
    pub fn parse<'a>(
        &self,
        raw: &'a [u8],
        scratch: &'a mut [FixFieldView<'a>],
    ) -> Result<FixMessageView<'a>, FixParseError> {
        parse_message(raw, scratch)
    }
}

/// Reusable FIX encoder with an owned output buffer.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FixEncoder {
    buffer: Vec<u8>,
}

impl FixEncoder {
    /// Creates an encoder with an empty buffer.
    pub const fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Creates an encoder with preallocated buffer capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Encodes into the reusable internal buffer and returns the encoded frame.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when a value contains SOH or a reserved tag is
    /// supplied by the caller.
    pub fn encode(
        &mut self,
        begin_string: &[u8],
        msg_type: &[u8],
        fields: &[(FixTag, &[u8])],
    ) -> Result<&[u8], FixEncodeError> {
        encode_message(&mut self.buffer, begin_string, msg_type, fields)?;
        Ok(&self.buffer)
    }

    /// Encodes a typed version and message type into the reusable buffer.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when a value contains SOH or a reserved tag is
    /// supplied by the caller.
    pub fn encode_typed(
        &mut self,
        version: FixVersion,
        msg_type: FixMsgType,
        fields: &[(FixTag, &[u8])],
    ) -> Result<&[u8], FixEncodeError> {
        self.encode(version.as_bytes(), msg_type.as_bytes(), fields)
    }

    /// Returns the current encoded buffer.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Clears the internal buffer without releasing capacity.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Consumes the encoder and returns the owned buffer.
    pub fn into_buffer(self) -> Vec<u8> {
        self.buffer
    }
}

/// Borrowed standard FIX session header fields used by admin builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixSessionHeader<'a> {
    sender_comp_id: &'a [u8],
    target_comp_id: &'a [u8],
    msg_seq_num: u64,
    sending_time: &'a [u8],
}

impl<'a> FixSessionHeader<'a> {
    /// Creates a standard session header.
    pub const fn new(
        sender_comp_id: &'a [u8],
        target_comp_id: &'a [u8],
        msg_seq_num: u64,
        sending_time: &'a [u8],
    ) -> Self {
        Self {
            sender_comp_id,
            target_comp_id,
            msg_seq_num,
            sending_time,
        }
    }

    /// Returns `SenderCompID(49)`.
    pub const fn sender_comp_id(&self) -> &'a [u8] {
        self.sender_comp_id
    }

    /// Returns `TargetCompID(56)`.
    pub const fn target_comp_id(&self) -> &'a [u8] {
        self.target_comp_id
    }

    /// Returns `MsgSeqNum(34)`.
    pub const fn msg_seq_num(&self) -> u64 {
        self.msg_seq_num
    }

    /// Returns `SendingTime(52)`.
    pub const fn sending_time(&self) -> &'a [u8] {
        self.sending_time
    }
}

/// Borrowed NewOrderSingle `<D>` request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixNewOrderSingle<'a> {
    cl_ord_id: &'a [u8],
    symbol: &'a [u8],
    side: FixOrderSide,
    transact_time: &'a [u8],
    order_qty: &'a [u8],
    ord_type: FixOrdType,
    price: Option<&'a [u8]>,
    time_in_force: Option<FixTimeInForce>,
}

impl<'a> FixNewOrderSingle<'a> {
    /// Creates a NewOrderSingle request.
    pub const fn new(
        cl_ord_id: &'a [u8],
        symbol: &'a [u8],
        side: FixOrderSide,
        transact_time: &'a [u8],
        order_qty: &'a [u8],
        ord_type: FixOrdType,
    ) -> Self {
        Self {
            cl_ord_id,
            symbol,
            side,
            transact_time,
            order_qty,
            ord_type,
            price: None,
            time_in_force: None,
        }
    }

    /// Adds `Price(44)`.
    pub const fn with_price(mut self, price: &'a [u8]) -> Self {
        self.price = Some(price);
        self
    }

    /// Adds `TimeInForce(59)`.
    pub const fn with_time_in_force(mut self, time_in_force: FixTimeInForce) -> Self {
        self.time_in_force = Some(time_in_force);
        self
    }
}

/// Borrowed OrderCancelRequest `<F>` request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixOrderCancelRequest<'a> {
    orig_cl_ord_id: &'a [u8],
    cl_ord_id: &'a [u8],
    symbol: &'a [u8],
    side: FixOrderSide,
    transact_time: &'a [u8],
}

impl<'a> FixOrderCancelRequest<'a> {
    /// Creates an OrderCancelRequest.
    pub const fn new(
        orig_cl_ord_id: &'a [u8],
        cl_ord_id: &'a [u8],
        symbol: &'a [u8],
        side: FixOrderSide,
        transact_time: &'a [u8],
    ) -> Self {
        Self {
            orig_cl_ord_id,
            cl_ord_id,
            symbol,
            side,
            transact_time,
        }
    }
}

/// Borrowed OrderCancelReplaceRequest `<G>` request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixOrderCancelReplaceRequest<'a> {
    orig_cl_ord_id: &'a [u8],
    cl_ord_id: &'a [u8],
    symbol: &'a [u8],
    side: FixOrderSide,
    transact_time: &'a [u8],
    order_qty: &'a [u8],
    ord_type: FixOrdType,
    price: Option<&'a [u8]>,
    time_in_force: Option<FixTimeInForce>,
}

impl<'a> FixOrderCancelReplaceRequest<'a> {
    /// Creates an OrderCancelReplaceRequest.
    pub const fn new(
        orig_cl_ord_id: &'a [u8],
        cl_ord_id: &'a [u8],
        symbol: &'a [u8],
        side: FixOrderSide,
        transact_time: &'a [u8],
        order_qty: &'a [u8],
        ord_type: FixOrdType,
    ) -> Self {
        Self {
            orig_cl_ord_id,
            cl_ord_id,
            symbol,
            side,
            transact_time,
            order_qty,
            ord_type,
            price: None,
            time_in_force: None,
        }
    }

    /// Adds `Price(44)`.
    pub const fn with_price(mut self, price: &'a [u8]) -> Self {
        self.price = Some(price);
        self
    }

    /// Adds `TimeInForce(59)`.
    pub const fn with_time_in_force(mut self, time_in_force: FixTimeInForce) -> Self {
        self.time_in_force = Some(time_in_force);
        self
    }
}

/// Parses and validates a FIX tag-value message into `scratch`.
///
/// The returned message borrows both `raw` and the initialized prefix of
/// `scratch`. No field values are allocated.
///
/// # Errors
///
/// Returns [`FixParseError`] if the message is malformed, missing required
/// tags, has invalid `BodyLength(9)` or `CheckSum(10)`, or `scratch` is too
/// small.
pub fn parse_message<'a>(
    raw: &'a [u8],
    scratch: &'a mut [FixFieldView<'a>],
) -> Result<FixMessageView<'a>, FixParseError> {
    if raw.is_empty() {
        return Err(FixParseError::Empty);
    }

    let mut count = 0usize;
    let mut body_start = None;
    let mut checksum_start = None;
    let mut declared_body_len = None;
    let mut declared_checksum = None;
    let mut pos = 0usize;

    while pos < raw.len() {
        let field_start = pos;
        let delimiter = raw[field_start..]
            .iter()
            .position(|b| *b == SOH)
            .map(|offset| field_start + offset)
            .unwrap_or(raw.len());
        if delimiter == field_start {
            return Err(FixParseError::MalformedField);
        }
        let field = &raw[field_start..delimiter];
        let eq = field
            .iter()
            .position(|b| *b == b'=')
            .ok_or(FixParseError::MalformedField)?;
        if eq == 0 {
            return Err(FixParseError::InvalidTag);
        }

        let tag = FixTag(parse_u32(&field[..eq]).map_err(|_| FixParseError::InvalidTag)?);
        let value = &field[eq + 1..];

        if count == scratch.len() {
            return Err(FixParseError::ScratchTooSmall {
                required: count.saturating_add(1),
                capacity: scratch.len(),
            });
        }
        scratch[count] = FixFieldView { tag, value };
        count += 1;

        if tag == FixTag::BODY_LENGTH {
            declared_body_len =
                Some(parse_usize(value).map_err(|_| FixParseError::InvalidBodyLength)?);
            body_start = Some(delimiter.saturating_add(1));
        } else if tag == FixTag::CHECK_SUM {
            checksum_start = Some(field_start);
            declared_checksum = Some(parse_checksum(value)?);
        }

        pos = delimiter.saturating_add(1);
    }

    let body_start = body_start.ok_or(FixParseError::MissingRequiredTag(FixTag::BODY_LENGTH))?;
    let checksum_start =
        checksum_start.ok_or(FixParseError::MissingRequiredTag(FixTag::CHECK_SUM))?;
    let expected_body_len =
        declared_body_len.ok_or(FixParseError::MissingRequiredTag(FixTag::BODY_LENGTH))?;
    let expected_checksum =
        declared_checksum.ok_or(FixParseError::MissingRequiredTag(FixTag::CHECK_SUM))?;

    if scratch[..count]
        .iter()
        .all(|field| field.tag != FixTag::BEGIN_STRING)
    {
        return Err(FixParseError::MissingRequiredTag(FixTag::BEGIN_STRING));
    }

    let actual_body_len = checksum_start.saturating_sub(body_start);
    if expected_body_len != actual_body_len {
        return Err(FixParseError::BodyLengthMismatch {
            expected: expected_body_len,
            actual: actual_body_len,
        });
    }

    let actual_checksum = checksum(&raw[..checksum_start]);
    if expected_checksum != actual_checksum {
        return Err(FixParseError::ChecksumMismatch {
            expected: expected_checksum,
            actual: actual_checksum,
        });
    }

    Ok(FixMessageView {
        raw,
        fields: &scratch[..count],
    })
}

/// Parses a validated Session Reject `<3>` message into a borrowed view.
///
/// # Errors
///
/// Returns [`FixRejectParseError`] when `MsgType(35)` is not `3`, required
/// `RefSeqNum(45)` is absent, or numeric reject fields are malformed.
pub fn parse_session_reject<'a>(
    message: &FixMessageView<'a>,
) -> Result<FixSessionRejectView<'a>, FixRejectParseError> {
    if message.msg_type() != Some(FixMsgType::REJECT.as_bytes()) {
        return Err(FixRejectParseError::InvalidMsgType);
    }
    Ok(FixSessionRejectView {
        ref_seq_num: parse_required_u64(message, FixTag::REF_SEQ_NUM)?,
        ref_tag_id: parse_optional_fix_tag(message, FixTag::REF_TAG_ID)?,
        ref_msg_type: message.get(FixTag::REF_MSG_TYPE),
        session_reject_reason: parse_optional_reject_u64(message, FixTag::SESSION_REJECT_REASON)?,
        text: message.get(FixTag::TEXT),
    })
}

/// Parses a validated BusinessMessageReject `<j>` message into a borrowed view.
///
/// # Errors
///
/// Returns [`FixRejectParseError`] when `MsgType(35)` is not `j`, required
/// fields are absent, or numeric reject fields are malformed.
pub fn parse_business_message_reject<'a>(
    message: &FixMessageView<'a>,
) -> Result<FixBusinessMessageRejectView<'a>, FixRejectParseError> {
    if message.msg_type() != Some(FixMsgType::BUSINESS_MESSAGE_REJECT.as_bytes()) {
        return Err(FixRejectParseError::InvalidMsgType);
    }
    let ref_msg_type = message
        .get(FixTag::REF_MSG_TYPE)
        .ok_or(FixRejectParseError::MissingTag(FixTag::REF_MSG_TYPE))?;
    Ok(FixBusinessMessageRejectView {
        ref_seq_num: parse_optional_reject_u64(message, FixTag::REF_SEQ_NUM)?,
        ref_msg_type,
        business_reject_ref_id: message.get(FixTag::BUSINESS_REJECT_REF_ID),
        business_reject_reason: parse_required_u64(message, FixTag::BUSINESS_REJECT_REASON)?,
        text: message.get(FixTag::TEXT),
    })
}

/// Encodes a FIX tag-value message into `out`.
///
/// `out` is cleared before encoding. Tags `8`, `9`, `35`, and `10` are owned by
/// this helper; pass application/session body fields through `fields`.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH or a reserved tag
/// is supplied in `fields`.
pub fn encode_message(
    out: &mut Vec<u8>,
    begin_string: &[u8],
    msg_type: &[u8],
    fields: &[(FixTag, &[u8])],
) -> Result<(), FixEncodeError> {
    encode_message_parts(out, begin_string, msg_type, &[], fields)
}

/// Encodes a retained source message as a possible-duplicate resend.
///
/// The source message must be a validated parsed message. This helper preserves
/// the original `MsgSeqNum(34)` and application fields, writes
/// `PossDupFlag(43)=Y`, replaces `SendingTime(52)` with `sending_time`, writes
/// `OrigSendingTime(122)` from the source `OrigSendingTime(122)` when present
/// or otherwise from the source `SendingTime(52)`, and recomputes
/// `BodyLength(9)`/`CheckSum(10)`.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when required source tags are missing or
/// `sending_time` contains SOH.
pub fn encode_poss_dup_replay(
    out: &mut Vec<u8>,
    source: &FixMessageView<'_>,
    sending_time: &[u8],
) -> Result<(), FixEncodeError> {
    let begin_string = source
        .begin_string()
        .ok_or(FixEncodeError::MissingRequiredTag(FixTag::BEGIN_STRING))?;
    let msg_type = source
        .msg_type()
        .ok_or(FixEncodeError::MissingRequiredTag(FixTag::MSG_TYPE))?;
    let orig_sending_time = source
        .get(FixTag::ORIG_SENDING_TIME)
        .or_else(|| source.get(FixTag::SENDING_TIME))
        .ok_or(FixEncodeError::MissingRequiredTag(FixTag::SENDING_TIME))?;

    validate_value(FixTag::BEGIN_STRING, begin_string)?;
    validate_value(FixTag::MSG_TYPE, msg_type)?;
    validate_value(FixTag::SENDING_TIME, sending_time)?;
    validate_value(FixTag::ORIG_SENDING_TIME, orig_sending_time)?;

    out.clear();
    write_field(out, FixTag::BEGIN_STRING, begin_string);
    write_field(out, FixTag::BODY_LENGTH, b"0000000000");
    let body_start = out.len();
    write_field(out, FixTag::MSG_TYPE, msg_type);

    let mut wrote_replay_header = false;
    for field in source.fields() {
        match field.tag {
            FixTag::BEGIN_STRING
            | FixTag::BODY_LENGTH
            | FixTag::MSG_TYPE
            | FixTag::CHECK_SUM
            | FixTag::POSS_DUP_FLAG
            | FixTag::ORIG_SENDING_TIME => {}
            FixTag::SENDING_TIME => {
                write_replay_header(out, sending_time, orig_sending_time);
                wrote_replay_header = true;
            }
            tag => {
                validate_value(tag, field.value)?;
                write_field(out, tag, field.value);
            }
        }
    }

    if !wrote_replay_header {
        return Err(FixEncodeError::MissingRequiredTag(FixTag::SENDING_TIME));
    }

    let body_len = out.len().saturating_sub(body_start);
    patch_body_length(out, body_len);
    let sum = checksum(out);
    write_checksum(out, sum);
    Ok(())
}

/// Encodes a Logon `<A>` admin message.
///
/// The builder writes standard header fields, `EncryptMethod(98)=0`,
/// `HeartBtInt(108)`, and optional `ResetSeqNumFlag(141)=Y`.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_logon(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    heartbeat_interval_secs: u64,
    reset_seq_num: bool,
) -> Result<(), FixEncodeError> {
    let mut heartbeat = [0u8; 20];
    let heartbeat_len = write_u64_digits(&mut heartbeat, heartbeat_interval_secs);
    let extra = [
        (FixTag::ENCRYPT_METHOD, b"0".as_slice()),
        (FixTag::HEART_BT_INT, &heartbeat[..heartbeat_len]),
        (FixTag::RESET_SEQ_NUM_FLAG, b"Y".as_slice()),
    ];
    let extra_len = if reset_seq_num { 3 } else { 2 };
    encode_session_message(out, version, FixMsgType::LOGON, header, &extra[..extra_len])
}

/// Encodes a Heartbeat `<0>` admin message.
///
/// `test_req_id` should be supplied when replying to a TestRequest `<1>`.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_heartbeat(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    test_req_id: Option<&[u8]>,
) -> Result<(), FixEncodeError> {
    let mut extra = [(FixTag::TEST_REQ_ID, b"".as_slice())];
    let extra_len = if let Some(test_req_id) = test_req_id {
        extra[0] = (FixTag::TEST_REQ_ID, test_req_id);
        1
    } else {
        0
    };
    encode_session_message(
        out,
        version,
        FixMsgType::HEARTBEAT,
        header,
        &extra[..extra_len],
    )
}

/// Encodes a TestRequest `<1>` admin message.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_test_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    test_req_id: &[u8],
) -> Result<(), FixEncodeError> {
    let extra = [(FixTag::TEST_REQ_ID, test_req_id)];
    encode_session_message(out, version, FixMsgType::TEST_REQUEST, header, &extra)
}

/// Encodes a ResendRequest `<2>` admin message.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_resend_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    range: FixResendRange,
) -> Result<(), FixEncodeError> {
    let mut begin = [0u8; 20];
    let begin_len = write_u64_digits(&mut begin, range.begin_seq_no);
    let mut end = [0u8; 20];
    let end_len = write_u64_digits(&mut end, range.end_seq_no);
    let extra = [
        (FixTag::BEGIN_SEQ_NO, &begin[..begin_len]),
        (FixTag::END_SEQ_NO, &end[..end_len]),
    ];
    encode_session_message(out, version, FixMsgType::RESEND_REQUEST, header, &extra)
}

/// Encodes a SequenceReset `<4>` gap-fill admin message.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_sequence_reset_gap_fill(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    new_seq_no: u64,
) -> Result<(), FixEncodeError> {
    let mut new_seq = [0u8; 20];
    let new_seq_len = write_u64_digits(&mut new_seq, new_seq_no);
    let extra = [
        (FixTag::GAP_FILL_FLAG, b"Y".as_slice()),
        (FixTag::NEW_SEQ_NO, &new_seq[..new_seq_len]),
    ];
    encode_session_message(out, version, FixMsgType::SEQUENCE_RESET, header, &extra)
}

/// Encodes a Logout `<5>` admin message.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_logout(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    text: Option<&[u8]>,
) -> Result<(), FixEncodeError> {
    let mut extra = [(FixTag::TEXT, b"".as_slice())];
    let extra_len = if let Some(text) = text {
        extra[0] = (FixTag::TEXT, text);
        1
    } else {
        0
    };
    encode_session_message(
        out,
        version,
        FixMsgType::LOGOUT,
        header,
        &extra[..extra_len],
    )
}

/// Encodes a NewOrderSingle `<D>` application message.
///
/// Quantities and prices are passed as borrowed wire-format bytes so venue
/// profiles can own decimal precision and tick-size policy.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_new_order_single(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    request: FixNewOrderSingle<'_>,
) -> Result<(), FixEncodeError> {
    let mut fields = [
        (FixTag::CL_ORD_ID, request.cl_ord_id),
        (FixTag::SYMBOL, request.symbol),
        (FixTag::SIDE, request.side.as_bytes()),
        (FixTag::TRANSACT_TIME, request.transact_time),
        (FixTag::ORDER_QTY, request.order_qty),
        (FixTag::ORD_TYPE, request.ord_type.as_bytes()),
        (FixTag::PRICE, b"".as_slice()),
        (FixTag::TIME_IN_FORCE, b"".as_slice()),
    ];
    let mut len = 6usize;
    if let Some(price) = request.price {
        fields[len] = (FixTag::PRICE, price);
        len += 1;
    }
    if let Some(time_in_force) = request.time_in_force {
        fields[len] = (FixTag::TIME_IN_FORCE, time_in_force.as_bytes());
        len += 1;
    }
    encode_session_message(
        out,
        version,
        FixMsgType::NEW_ORDER_SINGLE,
        header,
        &fields[..len],
    )
}

/// Encodes an OrderCancelRequest `<F>` application message.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_order_cancel_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    request: FixOrderCancelRequest<'_>,
) -> Result<(), FixEncodeError> {
    let fields = [
        (FixTag::ORIG_CL_ORD_ID, request.orig_cl_ord_id),
        (FixTag::CL_ORD_ID, request.cl_ord_id),
        (FixTag::SYMBOL, request.symbol),
        (FixTag::SIDE, request.side.as_bytes()),
        (FixTag::TRANSACT_TIME, request.transact_time),
    ];
    encode_session_message(
        out,
        version,
        FixMsgType::ORDER_CANCEL_REQUEST,
        header,
        &fields,
    )
}

/// Encodes an OrderCancelReplaceRequest `<G>` application message.
///
/// Quantities and prices are passed as borrowed wire-format bytes so venue
/// profiles can own decimal precision and tick-size policy.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_order_cancel_replace_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    request: FixOrderCancelReplaceRequest<'_>,
) -> Result<(), FixEncodeError> {
    let mut fields = [
        (FixTag::ORIG_CL_ORD_ID, request.orig_cl_ord_id),
        (FixTag::CL_ORD_ID, request.cl_ord_id),
        (FixTag::SYMBOL, request.symbol),
        (FixTag::SIDE, request.side.as_bytes()),
        (FixTag::TRANSACT_TIME, request.transact_time),
        (FixTag::ORDER_QTY, request.order_qty),
        (FixTag::ORD_TYPE, request.ord_type.as_bytes()),
        (FixTag::PRICE, b"".as_slice()),
        (FixTag::TIME_IN_FORCE, b"".as_slice()),
    ];
    let mut len = 7usize;
    if let Some(price) = request.price {
        fields[len] = (FixTag::PRICE, price);
        len += 1;
    }
    if let Some(time_in_force) = request.time_in_force {
        fields[len] = (FixTag::TIME_IN_FORCE, time_in_force.as_bytes());
        len += 1;
    }
    encode_session_message(
        out,
        version,
        FixMsgType::ORDER_CANCEL_REPLACE_REQUEST,
        header,
        &fields[..len],
    )
}

fn encode_session_message(
    out: &mut Vec<u8>,
    version: FixVersion,
    msg_type: FixMsgType,
    header: FixSessionHeader<'_>,
    fields: &[(FixTag, &[u8])],
) -> Result<(), FixEncodeError> {
    let mut seq_no = [0u8; 20];
    let seq_len = write_u64_digits(&mut seq_no, header.msg_seq_num());
    let header_fields = [
        (FixTag::SENDER_COMP_ID, header.sender_comp_id()),
        (FixTag::TARGET_COMP_ID, header.target_comp_id()),
        (FixTag::MSG_SEQ_NUM, &seq_no[..seq_len]),
        (FixTag::SENDING_TIME, header.sending_time()),
    ];
    encode_message_parts(
        out,
        version.as_bytes(),
        msg_type.as_bytes(),
        &header_fields,
        fields,
    )
}

fn encode_message_parts(
    out: &mut Vec<u8>,
    begin_string: &[u8],
    msg_type: &[u8],
    header_fields: &[(FixTag, &[u8])],
    fields: &[(FixTag, &[u8])],
) -> Result<(), FixEncodeError> {
    validate_value(FixTag::BEGIN_STRING, begin_string)?;
    validate_value(FixTag::MSG_TYPE, msg_type)?;

    out.clear();
    write_field(out, FixTag::BEGIN_STRING, begin_string);
    write_field(out, FixTag::BODY_LENGTH, b"0000000000");
    let body_start = out.len();
    write_field(out, FixTag::MSG_TYPE, msg_type);
    for (tag, value) in header_fields.iter().chain(fields.iter()) {
        if matches!(
            *tag,
            FixTag::BEGIN_STRING | FixTag::BODY_LENGTH | FixTag::MSG_TYPE | FixTag::CHECK_SUM
        ) {
            return Err(FixEncodeError::ReservedTag(*tag));
        }
        validate_value(*tag, value)?;
        write_field(out, *tag, value);
    }

    let body_len = out.len().saturating_sub(body_start);
    patch_body_length(out, body_len);
    let sum = checksum(out);
    write_checksum(out, sum);
    Ok(())
}

/// Computes a FIX modulo-256 checksum.
pub fn checksum(bytes: &[u8]) -> u8 {
    bytes
        .iter()
        .fold(0u32, |acc, byte| acc.wrapping_add(u32::from(*byte))) as u8
}

/// Renders a FIX frame with `|` in place of SOH.
///
/// This allocates and is intended for diagnostics rather than hot-path use.
pub fn debug_render(raw: &[u8]) -> String {
    raw.iter()
        .map(|b| if *b == SOH { '|' } else { *b as char })
        .collect()
}

fn write_field(out: &mut Vec<u8>, tag: FixTag, value: &[u8]) {
    write_u32(out, tag.0);
    out.push(b'=');
    out.extend_from_slice(value);
    out.push(SOH);
}

fn write_replay_header(out: &mut Vec<u8>, sending_time: &[u8], orig_sending_time: &[u8]) {
    write_field(out, FixTag::POSS_DUP_FLAG, b"Y");
    write_field(out, FixTag::SENDING_TIME, sending_time);
    write_field(out, FixTag::ORIG_SENDING_TIME, orig_sending_time);
}

fn write_checksum(out: &mut Vec<u8>, sum: u8) {
    out.extend_from_slice(b"10=");
    out.push(b'0' + (sum / 100));
    out.push(b'0' + ((sum / 10) % 10));
    out.push(b'0' + (sum % 10));
    out.push(SOH);
}

fn patch_body_length(out: &mut [u8], body_len: usize) {
    let Some(tag_start) = find_tag_start(out, FixTag::BODY_LENGTH) else {
        return;
    };
    let value_start = tag_start + 2;
    let mut digits = [b'0'; 10];
    write_usize_padded(&mut digits, body_len);
    out[value_start..value_start + digits.len()].copy_from_slice(&digits);
}

fn find_tag_start(raw: &[u8], tag: FixTag) -> Option<usize> {
    let mut pos = 0usize;
    while pos < raw.len() {
        let end = raw[pos..]
            .iter()
            .position(|b| *b == SOH)
            .map(|offset| pos + offset)
            .unwrap_or(raw.len());
        let field = &raw[pos..end];
        if let Some(eq) = field.iter().position(|b| *b == b'=') {
            if parse_u32(&field[..eq]).ok().map(FixTag) == Some(tag) {
                return Some(pos);
            }
        }
        pos = end.saturating_add(1);
    }
    None
}

fn validate_value(tag: FixTag, value: &[u8]) -> Result<(), FixEncodeError> {
    if value.contains(&SOH) {
        Err(FixEncodeError::ValueContainsSoh(tag))
    } else {
        Ok(())
    }
}

fn parse_required_u64(
    message: &FixMessageView<'_>,
    tag: FixTag,
) -> Result<u64, FixRejectParseError> {
    parse_u64(
        message
            .get(tag)
            .ok_or(FixRejectParseError::MissingTag(tag))?,
    )
    .map_err(|()| FixRejectParseError::InvalidNumber(tag))
}

fn parse_optional_reject_u64(
    message: &FixMessageView<'_>,
    tag: FixTag,
) -> Result<Option<u64>, FixRejectParseError> {
    if let Some(value) = message.get(tag) {
        parse_u64(value)
            .map(Some)
            .map_err(|()| FixRejectParseError::InvalidNumber(tag))
    } else {
        Ok(None)
    }
}

fn parse_optional_fix_tag(
    message: &FixMessageView<'_>,
    tag: FixTag,
) -> Result<Option<FixTag>, FixRejectParseError> {
    if let Some(value) = message.get(tag) {
        parse_u32(value)
            .map(|value| Some(FixTag(value)))
            .map_err(|()| FixRejectParseError::InvalidNumber(tag))
    } else {
        Ok(None)
    }
}

fn push_gap_fill<'a>(
    out: &mut Vec<FixResendAction<'a>>,
    begin_seq_no: u64,
    end_seq_no: u64,
    gap_fill_messages: &mut u64,
    gap_fill_sequences: &mut u64,
) {
    if begin_seq_no == 0 || begin_seq_no > end_seq_no {
        return;
    }
    out.push(FixResendAction::GapFill {
        begin_seq_no,
        end_seq_no,
    });
    *gap_fill_messages = gap_fill_messages.saturating_add(1);
    *gap_fill_sequences = gap_fill_sequences
        .saturating_add(end_seq_no.saturating_sub(begin_seq_no).saturating_add(1));
}

const fn clamp_seq_no(value: u64) -> u64 {
    if value == 0 {
        1
    } else {
        value
    }
}

fn parse_checksum(value: &[u8]) -> Result<u8, FixParseError> {
    if value.len() != 3 || !value.iter().all(u8::is_ascii_digit) {
        return Err(FixParseError::InvalidChecksum);
    }
    Ok((value[0] - b'0') * 100 + (value[1] - b'0') * 10 + (value[2] - b'0'))
}

fn parse_u32(bytes: &[u8]) -> Result<u32, ()> {
    let mut out = 0u32;
    if bytes.is_empty() {
        return Err(());
    }
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return Err(());
        }
        out = out
            .checked_mul(10)
            .and_then(|v| v.checked_add(u32::from(*byte - b'0')))
            .ok_or(())?;
    }
    Ok(out)
}

fn parse_u64(bytes: &[u8]) -> Result<u64, ()> {
    let mut out = 0u64;
    if bytes.is_empty() {
        return Err(());
    }
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return Err(());
        }
        out = out
            .checked_mul(10)
            .and_then(|v| v.checked_add(u64::from(*byte - b'0')))
            .ok_or(())?;
    }
    Ok(out)
}

fn parse_usize(bytes: &[u8]) -> Result<usize, ()> {
    let parsed = parse_u64(bytes)?;
    usize::try_from(parsed).map_err(|_| ())
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    let mut digits = [0u8; 10];
    let len = write_u32_digits(&mut digits, value);
    out.extend_from_slice(&digits[..len]);
}

fn write_usize_padded(out: &mut [u8; 10], value: usize) {
    let mut digits = [0u8; 20];
    let len = write_usize_digits(&mut digits, value);
    let out_len = out.len();
    let copied_len = len.min(out_len);
    let start = out_len.saturating_sub(copied_len);
    let digit_start = len.saturating_sub(out_len);
    out[start..].copy_from_slice(&digits[digit_start..len]);
}

fn write_u32_digits(out: &mut [u8; 10], mut value: u32) -> usize {
    if value == 0 {
        out[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 10];
    let mut len = 0usize;
    while value != 0 {
        tmp[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    for i in 0..len {
        out[i] = tmp[len - 1 - i];
    }
    len
}

fn write_u64_digits(out: &mut [u8; 20], mut value: u64) -> usize {
    if value == 0 {
        out[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 20];
    let mut len = 0usize;
    while value != 0 {
        tmp[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    for i in 0..len {
        out[i] = tmp[len - 1 - i];
    }
    len
}

fn write_usize_digits(out: &mut [u8; 20], mut value: usize) -> usize {
    if value == 0 {
        out[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 20];
    let mut len = 0usize;
    while value != 0 {
        tmp[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    for i in 0..len {
        out[i] = tmp[len - 1 - i];
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_parses_heartbeat() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"0",
            &[(FixTag::MSG_SEQ_NUM, b"1".as_slice())],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.get(FixTag::BEGIN_STRING),
            Some(b"FIX.4.4".as_slice())
        );
        assert_eq!(message.msg_type(), Some(b"0".as_slice()));
        assert_eq!(message.msg_seq_num(), Some(1));
        assert!(!message.poss_dup());
        assert!(message.debug_render().contains("35=0|"));
    }

    #[test]
    fn detects_body_length_mismatch() {
        let raw = b"8=FIX.4.4\x019=1\x0135=0\x0134=1\x0110=222\x01";
        let mut scratch = [FixFieldView::empty(); 16];
        let err = parse_message(raw, &mut scratch).expect_err("body length mismatch");
        assert!(matches!(err, FixParseError::BodyLengthMismatch { .. }));
    }

    #[test]
    fn detects_checksum_mismatch() {
        let mut raw = Vec::new();
        encode_message(&mut raw, b"FIX.4.4", b"0", &[]).expect("encode");
        let len = raw.len();
        raw[len - 3] = b'9';
        let mut scratch = [FixFieldView::empty(); 16];
        let err = parse_message(&raw, &mut scratch).expect_err("checksum mismatch");
        assert!(matches!(err, FixParseError::ChecksumMismatch { .. }));
    }

    #[test]
    fn rejects_too_small_scratch() {
        let mut raw = Vec::new();
        encode_message(&mut raw, b"FIX.4.4", b"0", &[]).expect("encode");
        let mut scratch = [FixFieldView::empty(); 2];
        let err = parse_message(&raw, &mut scratch).expect_err("scratch too small");
        assert!(matches!(err, FixParseError::ScratchTooSmall { .. }));
    }

    #[test]
    fn rejects_reserved_encode_tags() {
        let mut raw = Vec::new();
        let err = encode_message(
            &mut raw,
            b"FIX.4.4",
            b"D",
            &[(FixTag::CHECK_SUM, b"001".as_slice())],
        )
        .expect_err("reserved tag");
        assert_eq!(err, FixEncodeError::ReservedTag(FixTag::CHECK_SUM));
    }

    #[test]
    fn rejects_soh_in_values() {
        let mut raw = Vec::new();
        let err = encode_message(&mut raw, b"FIX.4.4", b"D\x01", &[]).expect_err("soh");
        assert_eq!(err, FixEncodeError::ValueContainsSoh(FixTag::MSG_TYPE));
    }

    #[test]
    fn typed_encoder_decoder_round_trip() {
        let mut encoder = FixEncoder::with_capacity(128);
        let raw = encoder
            .encode_typed(
                FixVersion::Fix44,
                FixMsgType::NEW_ORDER_SINGLE,
                &[
                    (FixTag::MSG_SEQ_NUM, b"7".as_slice()),
                    (FixTag::CL_ORD_ID, b"ORD-1".as_slice()),
                ],
            )
            .expect("encode");

        let decoder = FixDecoder::new();
        let mut scratch = [FixFieldView::empty(); 16];
        let message = decoder.parse(raw, &mut scratch).expect("parse");

        assert_eq!(message.version(), Some(FixVersion::Fix44));
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::NEW_ORDER_SINGLE));
        assert_eq!(message.msg_seq_num(), Some(7));
    }

    #[test]
    fn dictionary_validates_required_tags() {
        static REQUIRED: &[FixTag] = &[FixTag::CL_ORD_ID, FixTag::SYMBOL, FixTag::SIDE];
        static RULES: &[FixMessageRule<'static>] = &[FixMessageRule::new(
            FixMsgType::NEW_ORDER_SINGLE,
            REQUIRED,
            &[],
        )];
        let dictionary = FixDictionary::new(FixVersion::Fix44, RULES);

        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"D",
            &[
                (FixTag::CL_ORD_ID, b"ORD-1".as_slice()),
                (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
            ],
        )
        .expect("encode");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");

        let err = dictionary
            .validate(&message)
            .expect_err("missing side should fail");
        assert_eq!(
            err,
            FixProfileError::MissingRequiredTag {
                msg_type: FixMsgType::NEW_ORDER_SINGLE,
                tag: FixTag::SIDE,
            }
        );
    }

    #[test]
    fn dictionary_rejects_disallowed_tags() {
        static DISALLOWED: &[FixTag] = &[FixTag::TEXT];
        static RULES: &[FixMessageRule<'static>] =
            &[FixMessageRule::new(FixMsgType::HEARTBEAT, &[], DISALLOWED)];
        let dictionary = FixDictionary::new(FixVersion::Fix44, RULES);

        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"0",
            &[(FixTag::TEXT, b"no text here".as_slice())],
        )
        .expect("encode");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");

        let err = dictionary
            .validate(&message)
            .expect_err("disallowed text should fail");
        assert_eq!(
            err,
            FixProfileError::DisallowedTag {
                msg_type: FixMsgType::HEARTBEAT,
                tag: FixTag::TEXT,
            }
        );
    }

    #[test]
    fn dictionary_rejects_version_mismatch() {
        static RULES: &[FixMessageRule<'static>] =
            &[FixMessageRule::new(FixMsgType::HEARTBEAT, &[], &[])];
        let dictionary = FixDictionary::new(FixVersion::Fix42, RULES);

        let mut raw = Vec::new();
        encode_message(&mut raw, b"FIX.4.4", b"0", &[]).expect("encode");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");

        assert_eq!(
            dictionary.validate(&message),
            Err(FixProfileError::VersionMismatch {
                expected: FixVersion::Fix42,
                actual: FixVersion::Fix44,
            })
        );
    }

    #[test]
    fn sequence_tracker_accepts_expected_inbound() {
        let mut tracker = FixSequenceTracker::new();
        assert_eq!(
            tracker.observe_inbound(1, false),
            Ok(FixSequenceAction::Accept { seq_no: 1 })
        );
        assert_eq!(tracker.next_inbound(), 2);
    }

    #[test]
    fn sequence_tracker_detects_gap_without_advancing() {
        let mut tracker = FixSequenceTracker::new();
        assert_eq!(
            tracker.observe_inbound(3, false),
            Ok(FixSequenceAction::Gap {
                expected: 1,
                received: 3,
                resend: FixResendRange {
                    begin_seq_no: 1,
                    end_seq_no: 2,
                },
            })
        );
        assert_eq!(tracker.next_inbound(), 1);
    }

    #[test]
    fn sequence_tracker_marks_poss_dup_low_sequence_duplicate() {
        let mut tracker = FixSequenceTracker::from_next(5, 9);
        assert_eq!(
            tracker.observe_inbound(3, true),
            Ok(FixSequenceAction::Duplicate {
                seq_no: 3,
                expected: 5,
            })
        );
        assert_eq!(tracker.next_inbound(), 5);
    }

    #[test]
    fn sequence_tracker_marks_unflagged_low_sequence_too_low() {
        let mut tracker = FixSequenceTracker::from_next(5, 9);
        assert_eq!(
            tracker.observe_inbound(3, false),
            Ok(FixSequenceAction::TooLow {
                expected: 5,
                received: 3,
            })
        );
    }

    #[test]
    fn sequence_tracker_observes_parsed_message_sequence() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"0",
            &[(FixTag::MSG_SEQ_NUM, b"1".as_slice())],
        )
        .expect("encode");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");

        let mut tracker = FixSequenceTracker::new();
        assert_eq!(
            tracker.observe_message(&message),
            Ok(FixSequenceAction::Accept { seq_no: 1 })
        );
    }

    #[test]
    fn sequence_tracker_assigns_outbound_monotonically() {
        let mut tracker = FixSequenceTracker::from_next(1, 10);
        assert_eq!(tracker.assign_outbound(), 10);
        assert_eq!(tracker.assign_outbound(), 11);
        assert_eq!(tracker.next_outbound(), 12);
    }

    #[test]
    fn sequence_reset_advances_but_does_not_decrease() {
        let mut tracker = FixSequenceTracker::from_next(10, 1);
        tracker.apply_sequence_reset(15).expect("advance");
        assert_eq!(tracker.next_inbound(), 15);
        assert_eq!(
            tracker.apply_sequence_reset(14),
            Err(FixSequenceError::SequenceResetWouldDecrease {
                current: 15,
                requested: 14,
            })
        );
    }

    #[test]
    fn session_id_rejects_soh() {
        let err = FixSessionId::new(FixVersion::Fix44, b"CLIENT\x01", b"BROKER").expect_err("soh");
        assert_eq!(
            err,
            FixEncodeError::ValueContainsSoh(FixTag::SENDER_COMP_ID)
        );
    }

    #[test]
    fn sequence_snapshot_round_trips_tracker_state() {
        let session_id =
            FixSessionId::with_qualifier(FixVersion::Fix44, b"CLIENT", b"BROKER", b"A")
                .expect("session");
        let tracker = FixSequenceTracker::from_next(12, 34);
        let snapshot = tracker.snapshot(session_id, b"20260717").expect("snapshot");

        assert_eq!(snapshot.session_id(), session_id);
        assert_eq!(snapshot.trading_day(), b"20260717");
        assert_eq!(snapshot.next_inbound(), 12);
        assert_eq!(snapshot.next_outbound(), 34);

        let restored = FixSequenceTracker::from_snapshot(&snapshot);
        assert_eq!(restored.next_inbound(), 12);
        assert_eq!(restored.next_outbound(), 34);
    }

    #[test]
    fn sequence_snapshot_clamps_zero_counters() {
        let session_id =
            FixSessionId::new(FixVersion::Fix44, b"CLIENT", b"BROKER").expect("session");
        let snapshot = FixSequenceSnapshot::new(session_id, 0, 0, b"20260717").expect("snapshot");
        assert_eq!(snapshot.next_inbound(), 1);
        assert_eq!(snapshot.next_outbound(), 1);
    }

    #[test]
    fn sequence_tracker_resets_to_one() {
        let mut tracker = FixSequenceTracker::from_next(99, 100);
        tracker.reset_to_one();
        assert_eq!(tracker.next_inbound(), 1);
        assert_eq!(tracker.next_outbound(), 1);
    }

    #[test]
    fn resend_store_plans_replay_and_gap_fills() {
        let mut store = FixResendStore::new(FixResendStoreConfig::new(8, 1024));
        store
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("seq 1");
        store
            .record_sent(2, FixSentMessageKind::Administrative, b"admin-2")
            .expect("seq 2");
        store
            .record_sent(3, FixSentMessageKind::Reject, b"reject-3")
            .expect("seq 3");
        store
            .record_sent(5, FixSentMessageKind::Application, b"app-5")
            .expect("seq 5");

        let mut actions = Vec::new();
        let summary = store.plan_resend_range(
            FixResendRange {
                begin_seq_no: 1,
                end_seq_no: 5,
            },
            &mut actions,
        );

        assert_eq!(summary.replay_messages(), 3);
        assert_eq!(summary.gap_fill_messages(), 2);
        assert_eq!(summary.gap_fill_sequences(), 2);
        assert_eq!(
            actions,
            vec![
                FixResendAction::Replay {
                    seq_no: 1,
                    raw: b"app-1"
                },
                FixResendAction::GapFill {
                    begin_seq_no: 2,
                    end_seq_no: 2
                },
                FixResendAction::Replay {
                    seq_no: 3,
                    raw: b"reject-3"
                },
                FixResendAction::GapFill {
                    begin_seq_no: 4,
                    end_seq_no: 4
                },
                FixResendAction::Replay {
                    seq_no: 5,
                    raw: b"app-5"
                },
            ]
        );
    }

    #[test]
    fn resend_store_uses_newest_sequence_for_open_ended_range() {
        let mut store = FixResendStore::new(FixResendStoreConfig::new(8, 1024));
        store
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("seq 1");
        store
            .record_sent(2, FixSentMessageKind::Administrative, b"admin-2")
            .expect("seq 2");

        let mut actions = vec![FixResendAction::GapFill {
            begin_seq_no: 99,
            end_seq_no: 99,
        }];
        let summary = store.plan_resend_range(
            FixResendRange {
                begin_seq_no: 1,
                end_seq_no: 0,
            },
            &mut actions,
        );

        assert_eq!(summary.replay_messages(), 1);
        assert_eq!(summary.gap_fill_sequences(), 1);
        assert_eq!(
            actions,
            vec![
                FixResendAction::Replay {
                    seq_no: 1,
                    raw: b"app-1"
                },
                FixResendAction::GapFill {
                    begin_seq_no: 2,
                    end_seq_no: 2
                },
            ]
        );
    }

    #[test]
    fn resend_store_eviction_turns_old_sequences_into_gap_fill() {
        let mut store = FixResendStore::new(FixResendStoreConfig::new(2, 1024));
        store
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("seq 1");
        store
            .record_sent(2, FixSentMessageKind::Application, b"app-2")
            .expect("seq 2");
        let retention = store
            .record_sent(3, FixSentMessageKind::Application, b"app-3")
            .expect("seq 3");
        assert!(retention.retained());
        assert_eq!(retention.evicted_messages(), 1);

        let metrics = store.metrics();
        assert_eq!(metrics.retained_messages(), 2);
        assert_eq!(metrics.oldest_seq_no(), Some(2));
        assert_eq!(metrics.newest_seq_no(), Some(3));
        assert_eq!(metrics.evicted_messages(), 1);

        let mut actions = Vec::new();
        store.plan_resend_range(
            FixResendRange {
                begin_seq_no: 1,
                end_seq_no: 3,
            },
            &mut actions,
        );
        assert_eq!(
            actions,
            vec![
                FixResendAction::GapFill {
                    begin_seq_no: 1,
                    end_seq_no: 1
                },
                FixResendAction::Replay {
                    seq_no: 2,
                    raw: b"app-2"
                },
                FixResendAction::Replay {
                    seq_no: 3,
                    raw: b"app-3"
                },
            ]
        );
    }

    #[test]
    fn resend_store_reports_disabled_or_oversized_drops() {
        let mut disabled = FixResendStore::new(FixResendStoreConfig::new(0, 1024));
        let retention = disabled
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("disabled retention");
        assert!(!retention.retained());
        assert_eq!(disabled.metrics().dropped_messages(), 1);

        let mut bounded = FixResendStore::new(FixResendStoreConfig::new(4, 4));
        let retention = bounded
            .record_sent(1, FixSentMessageKind::Application, b"app-1")
            .expect("oversized retention");
        assert!(!retention.retained());
        assert_eq!(bounded.metrics().dropped_bytes(), 5);
    }

    #[test]
    fn resend_store_rejects_non_increasing_sequences() {
        let mut store = FixResendStore::default();
        store
            .record_sent(10, FixSentMessageKind::Application, b"app-10")
            .expect("seq 10");
        let err = store
            .record_sent(10, FixSentMessageKind::Application, b"app-10-again")
            .expect_err("same sequence");
        assert_eq!(
            err,
            FixResendStoreError::SequenceRegression {
                latest: 10,
                received: 10
            }
        );
    }

    #[test]
    fn encodes_poss_dup_replay_with_orig_sending_time() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:05.000");
        let order = FixNewOrderSingle::new(
            b"ORD-1",
            b"BTCUSDT",
            FixOrderSide::Buy,
            b"20260717-12:00:05.000",
            b"1.25",
            FixOrdType::Limit,
        )
        .with_price(b"65000.5");
        let mut original = Vec::new();
        encode_new_order_single(&mut original, FixVersion::Fix44, header, order)
            .expect("original order");
        let mut scratch = [FixFieldView::empty(); 32];
        let original_view = parse_message(&original, &mut scratch).expect("parse original");

        let mut replay = Vec::new();
        encode_poss_dup_replay(&mut replay, &original_view, b"20260717-12:00:06.000")
            .expect("replay");
        let mut replay_scratch = [FixFieldView::empty(); 40];
        let replay_view = parse_message(&replay, &mut replay_scratch).expect("parse replay");

        assert_eq!(replay_view.msg_seq_num(), Some(7));
        assert_eq!(
            replay_view.get(FixTag::POSS_DUP_FLAG),
            Some(b"Y".as_slice())
        );
        assert_eq!(
            replay_view.get(FixTag::SENDING_TIME),
            Some(b"20260717-12:00:06.000".as_slice())
        );
        assert_eq!(
            replay_view.get(FixTag::ORIG_SENDING_TIME),
            Some(b"20260717-12:00:05.000".as_slice())
        );
        assert_eq!(
            replay_view.get(FixTag::CL_ORD_ID),
            Some(b"ORD-1".as_slice())
        );
    }

    #[test]
    fn poss_dup_replay_preserves_existing_orig_sending_time() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:05.000");
        let mut original = Vec::new();
        encode_heartbeat(&mut original, FixVersion::Fix44, header, None).expect("heartbeat");
        let mut scratch = [FixFieldView::empty(); 16];
        let original_view = parse_message(&original, &mut scratch).expect("parse original");

        let mut first_replay = Vec::new();
        encode_poss_dup_replay(&mut first_replay, &original_view, b"20260717-12:00:06.000")
            .expect("first replay");
        let mut first_scratch = [FixFieldView::empty(); 20];
        let first_view = parse_message(&first_replay, &mut first_scratch).expect("parse first");

        let mut second_replay = Vec::new();
        encode_poss_dup_replay(&mut second_replay, &first_view, b"20260717-12:00:07.000")
            .expect("second replay");
        let mut second_scratch = [FixFieldView::empty(); 20];
        let second_view = parse_message(&second_replay, &mut second_scratch).expect("parse second");

        assert_eq!(
            second_view.get(FixTag::SENDING_TIME),
            Some(b"20260717-12:00:07.000".as_slice())
        );
        assert_eq!(
            second_view.get(FixTag::ORIG_SENDING_TIME),
            Some(b"20260717-12:00:05.000".as_slice())
        );
    }

    #[test]
    fn poss_dup_replay_requires_source_sending_time() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"0",
            &[
                (FixTag::SENDER_COMP_ID, b"CLIENT".as_slice()),
                (FixTag::TARGET_COMP_ID, b"BROKER".as_slice()),
                (FixTag::MSG_SEQ_NUM, b"1".as_slice()),
            ],
        )
        .expect("source without sending time");
        let mut scratch = [FixFieldView::empty(); 16];
        let view = parse_message(&raw, &mut scratch).expect("parse source");
        let err = encode_poss_dup_replay(&mut Vec::new(), &view, b"20260717-12:00:06.000")
            .expect_err("missing sending time");
        assert_eq!(
            err,
            FixEncodeError::MissingRequiredTag(FixTag::SENDING_TIME)
        );
    }

    #[test]
    fn parses_session_reject_view() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"3",
            &[
                (FixTag::REF_SEQ_NUM, b"12".as_slice()),
                (FixTag::REF_TAG_ID, b"55".as_slice()),
                (FixTag::REF_MSG_TYPE, b"D".as_slice()),
                (FixTag::SESSION_REJECT_REASON, b"1".as_slice()),
                (FixTag::TEXT, b"missing symbol".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let reject = parse_session_reject(&message).expect("reject");

        assert_eq!(reject.ref_seq_num(), 12);
        assert_eq!(reject.ref_tag_id(), Some(FixTag::SYMBOL));
        assert_eq!(reject.ref_msg_type(), Some(b"D".as_slice()));
        assert_eq!(reject.session_reject_reason(), Some(1));
        assert_eq!(reject.text(), Some(b"missing symbol".as_slice()));
    }

    #[test]
    fn session_reject_requires_ref_seq_num() {
        let mut raw = Vec::new();
        encode_message(&mut raw, b"FIX.4.4", b"3", &[]).expect("encode");

        let mut scratch = [FixFieldView::empty(); 8];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            parse_session_reject(&message),
            Err(FixRejectParseError::MissingTag(FixTag::REF_SEQ_NUM))
        );
    }

    #[test]
    fn parses_business_message_reject_view() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"j",
            &[
                (FixTag::REF_SEQ_NUM, b"21".as_slice()),
                (FixTag::REF_MSG_TYPE, b"D".as_slice()),
                (FixTag::BUSINESS_REJECT_REF_ID, b"ORD-1".as_slice()),
                (FixTag::BUSINESS_REJECT_REASON, b"3".as_slice()),
                (FixTag::TEXT, b"unsupported order".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let reject = parse_business_message_reject(&message).expect("reject");

        assert_eq!(reject.ref_seq_num(), Some(21));
        assert_eq!(reject.ref_msg_type(), b"D".as_slice());
        assert_eq!(reject.business_reject_ref_id(), Some(b"ORD-1".as_slice()));
        assert_eq!(reject.business_reject_reason(), 3);
        assert_eq!(reject.text(), Some(b"unsupported order".as_slice()));
    }

    #[test]
    fn business_message_reject_validates_required_numeric_reason() {
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"j",
            &[
                (FixTag::REF_MSG_TYPE, b"D".as_slice()),
                (FixTag::BUSINESS_REJECT_REASON, b"bad".as_slice()),
            ],
        )
        .expect("encode");

        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            parse_business_message_reject(&message),
            Err(FixRejectParseError::InvalidNumber(
                FixTag::BUSINESS_REJECT_REASON
            ))
        );
    }

    #[test]
    fn encodes_logon_with_required_admin_fields() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 1, b"20260717-12:00:00.000");
        let mut raw = Vec::new();
        encode_logon(&mut raw, FixVersion::Fix44, header, 30, true).expect("logon");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::LOGON));
        assert_eq!(
            message.get(FixTag::SENDER_COMP_ID),
            Some(b"CLIENT".as_slice())
        );
        assert_eq!(
            message.get(FixTag::TARGET_COMP_ID),
            Some(b"BROKER".as_slice())
        );
        assert_eq!(message.get(FixTag::ENCRYPT_METHOD), Some(b"0".as_slice()));
        assert_eq!(message.get(FixTag::HEART_BT_INT), Some(b"30".as_slice()));
        assert_eq!(
            message.get(FixTag::RESET_SEQ_NUM_FLAG),
            Some(b"Y".as_slice())
        );
    }

    #[test]
    fn encodes_heartbeat_with_test_request_id() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 2, b"20260717-12:00:01.000");
        let mut raw = Vec::new();
        encode_heartbeat(&mut raw, FixVersion::Fix44, header, Some(b"T1")).expect("heartbeat");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::HEARTBEAT));
        assert_eq!(message.get(FixTag::TEST_REQ_ID), Some(b"T1".as_slice()));
    }

    #[test]
    fn encodes_resend_request_range() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 3, b"20260717-12:00:02.000");
        let mut raw = Vec::new();
        encode_resend_request(
            &mut raw,
            FixVersion::Fix44,
            header,
            FixResendRange {
                begin_seq_no: 4,
                end_seq_no: 9,
            },
        )
        .expect("resend request");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::RESEND_REQUEST));
        assert_eq!(message.begin_seq_no(), Some(4));
        assert_eq!(message.end_seq_no(), Some(9));
    }

    #[test]
    fn encodes_sequence_reset_gap_fill() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 4, b"20260717-12:00:03.000");
        let mut raw = Vec::new();
        encode_sequence_reset_gap_fill(&mut raw, FixVersion::Fix44, header, 12).expect("gap fill");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::SEQUENCE_RESET));
        assert!(message.gap_fill());
        assert_eq!(message.new_seq_no(), Some(12));
    }

    #[test]
    fn logout_builder_rejects_soh_in_text() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 5, b"20260717-12:00:04.000");
        let mut raw = Vec::new();
        let err = encode_logout(
            &mut raw,
            FixVersion::Fix44,
            header,
            Some(b"bad\x01text".as_slice()),
        )
        .expect_err("soh should fail");
        assert_eq!(err, FixEncodeError::ValueContainsSoh(FixTag::TEXT));
    }

    #[test]
    fn encodes_new_order_single() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 6, b"20260717-12:00:05.000");
        let request = FixNewOrderSingle::new(
            b"ORD-1",
            b"BTCUSDT",
            FixOrderSide::Buy,
            b"20260717-12:00:05.000",
            b"1.25",
            FixOrdType::Limit,
        )
        .with_price(b"65000.5")
        .with_time_in_force(FixTimeInForce::Day);

        let mut raw = Vec::new();
        encode_new_order_single(&mut raw, FixVersion::Fix44, header, request).expect("new order");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::NEW_ORDER_SINGLE));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"ORD-1".as_slice()));
        assert_eq!(message.get(FixTag::SYMBOL), Some(b"BTCUSDT".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"1".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"1.25".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65000.5".as_slice()));
    }

    #[test]
    fn encodes_order_cancel_request() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 7, b"20260717-12:00:06.000");
        let request = FixOrderCancelRequest::new(
            b"ORD-1",
            b"ORD-1-CXL",
            b"BTCUSDT",
            FixOrderSide::Buy,
            b"20260717-12:00:06.000",
        );

        let mut raw = Vec::new();
        encode_order_cancel_request(&mut raw, FixVersion::Fix44, header, request).expect("cancel");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.typed_msg_type(),
            Some(FixMsgType::ORDER_CANCEL_REQUEST)
        );
        assert_eq!(
            message.get(FixTag::ORIG_CL_ORD_ID),
            Some(b"ORD-1".as_slice())
        );
        assert_eq!(
            message.get(FixTag::CL_ORD_ID),
            Some(b"ORD-1-CXL".as_slice())
        );
    }

    #[test]
    fn encodes_order_cancel_replace_request() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 8, b"20260717-12:00:07.000");
        let request = FixOrderCancelReplaceRequest::new(
            b"ORD-1",
            b"ORD-2",
            b"BTCUSDT",
            FixOrderSide::Buy,
            b"20260717-12:00:07.000",
            b"2.00",
            FixOrdType::Limit,
        )
        .with_price(b"65100")
        .with_time_in_force(FixTimeInForce::ImmediateOrCancel);

        let mut raw = Vec::new();
        encode_order_cancel_replace_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("replace");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.typed_msg_type(),
            Some(FixMsgType::ORDER_CANCEL_REPLACE_REQUEST)
        );
        assert_eq!(
            message.get(FixTag::ORIG_CL_ORD_ID),
            Some(b"ORD-1".as_slice())
        );
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"ORD-2".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"2.00".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"3".as_slice()));
    }

    #[test]
    fn order_builder_rejects_soh_in_symbol() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 9, b"20260717-12:00:08.000");
        let request = FixNewOrderSingle::new(
            b"ORD-1",
            b"BTC\x01USDT",
            FixOrderSide::Buy,
            b"20260717-12:00:08.000",
            b"1",
            FixOrdType::Market,
        );
        let mut raw = Vec::new();
        let err = encode_new_order_single(&mut raw, FixVersion::Fix44, header, request)
            .expect_err("soh should fail");
        assert_eq!(err, FixEncodeError::ValueContainsSoh(FixTag::SYMBOL));
    }
}
