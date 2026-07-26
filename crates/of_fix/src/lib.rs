//! Low-allocation FIX tag-value codec primitives for Orderflow.
#![doc = include_str!("../README.md")]

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const SEQUENCE_SNAPSHOT_MAGIC: &[u8; 8] = b"OFIXSEQ\0";
const SEQUENCE_SNAPSHOT_VERSION: u16 = 1;
const SEQUENCE_SNAPSHOT_FILE: &str = "fix-sequence.snapshot";
const SEQUENCE_SNAPSHOT_TMP_FILE: &str = "fix-sequence.snapshot.tmp";

/// FIX field delimiter byte.
pub const SOH: u8 = 0x01;

/// Numeric FIX tag identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixTag(pub u32);

impl FixTag {
    /// `BeginString(8)`.
    pub const BEGIN_STRING: Self = Self(8);
    /// `Account(1)`.
    pub const ACCOUNT: Self = Self(1);
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
    /// `TradingSessionID(336)`.
    pub const TRADING_SESSION_ID: Self = Self(336);
    /// `EncodedTextLen(354)`.
    pub const ENCODED_TEXT_LEN: Self = Self(354);
    /// `OrderQty(38)`.
    pub const ORDER_QTY: Self = Self(38);
    /// `OrdType(40)`.
    pub const ORD_TYPE: Self = Self(40);
    /// `Price(44)`.
    pub const PRICE: Self = Self(44);
    /// `TimeInForce(59)`.
    pub const TIME_IN_FORCE: Self = Self(59);
    /// `StopPx(99)`.
    pub const STOP_PX: Self = Self(99);
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
    /// `SecondaryClOrdID(526)`.
    pub const SECONDARY_CL_ORD_ID: Self = Self(526);
    /// `MassCancelRequestType(530)`.
    pub const MASS_CANCEL_REQUEST_TYPE: Self = Self(530);
    /// `MassStatusReqID(584)`.
    pub const MASS_STATUS_REQ_ID: Self = Self(584);
    /// `MassStatusReqType(585)`.
    pub const MASS_STATUS_REQ_TYPE: Self = Self(585);
    /// `TradingSessionSubID(625)`.
    pub const TRADING_SESSION_SUB_ID: Self = Self(625);
    /// `AcctIDSource(660)`.
    pub const ACCT_ID_SOURCE: Self = Self(660);
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
    /// `OrderMassCancelRequest(q)`.
    pub const ORDER_MASS_CANCEL_REQUEST: Self = Self(b"q");
    /// `OrderMassStatusRequest(AF)`.
    pub const ORDER_MASS_STATUS_REQUEST: Self = Self(b"AF");

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
            b"AF" => Some(Self::ORDER_MASS_STATUS_REQUEST),
            b"j" => Some(Self::BUSINESS_MESSAGE_REJECT),
            b"q" => Some(Self::ORDER_MASS_CANCEL_REQUEST),
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
            b"AF" => "OrderMassStatusRequest",
            b"j" => "BusinessMessageReject",
            b"q" => "OrderMassCancelRequest",
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

/// Common FIX `MassCancelRequestType(530)` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixMassCancelRequestType {
    /// Cancel orders for a security (`1`).
    Security,
    /// Cancel orders for an underlying security (`2`).
    UnderlyingSecurity,
    /// Cancel orders for a product (`3`).
    Product,
    /// Cancel orders for a CFICode (`4`).
    CfiCode,
    /// Cancel orders for a security type (`5`).
    SecurityType,
    /// Cancel orders for a trading session (`6`).
    TradingSession,
    /// Cancel all orders (`7`).
    AllOrders,
}

impl FixMassCancelRequestType {
    /// Returns the wire value.
    pub const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Security => b"1",
            Self::UnderlyingSecurity => b"2",
            Self::Product => b"3",
            Self::CfiCode => b"4",
            Self::SecurityType => b"5",
            Self::TradingSession => b"6",
            Self::AllOrders => b"7",
        }
    }
}

/// Common FIX `MassStatusReqType(585)` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixMassStatusReqType {
    /// Status for orders for a security (`1`).
    Security,
    /// Status for orders for an underlying security (`2`).
    UnderlyingSecurity,
    /// Status for orders for a product (`3`).
    Product,
    /// Status for orders for a CFICode (`4`).
    CfiCode,
    /// Status for orders for a security type (`5`).
    SecurityType,
    /// Status for orders for a trading session (`6`).
    TradingSession,
    /// Status for all orders (`7`).
    AllOrders,
    /// Status for orders for a PartyID (`8`).
    PartyId,
}

impl FixMassStatusReqType {
    /// Returns the wire value.
    pub const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Security => b"1",
            Self::UnderlyingSecurity => b"2",
            Self::Product => b"3",
            Self::CfiCode => b"4",
            Self::SecurityType => b"5",
            Self::TradingSession => b"6",
            Self::AllOrders => b"7",
            Self::PartyId => b"8",
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

/// Owned FIX session identity loaded from durable storage.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixOwnedSessionId {
    version: FixVersion,
    sender_comp_id: Vec<u8>,
    target_comp_id: Vec<u8>,
    qualifier: Vec<u8>,
}

impl FixOwnedSessionId {
    /// Creates an owned session id.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when an identifier contains SOH.
    pub fn new(
        version: FixVersion,
        sender_comp_id: impl Into<Vec<u8>>,
        target_comp_id: impl Into<Vec<u8>>,
    ) -> Result<Self, FixEncodeError> {
        Self::with_qualifier(version, sender_comp_id, target_comp_id, Vec::new())
    }

    /// Creates an owned session id with a qualifier.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when an identifier contains SOH.
    pub fn with_qualifier(
        version: FixVersion,
        sender_comp_id: impl Into<Vec<u8>>,
        target_comp_id: impl Into<Vec<u8>>,
        qualifier: impl Into<Vec<u8>>,
    ) -> Result<Self, FixEncodeError> {
        let sender_comp_id = sender_comp_id.into();
        let target_comp_id = target_comp_id.into();
        let qualifier = qualifier.into();
        validate_value(FixTag::SENDER_COMP_ID, &sender_comp_id)?;
        validate_value(FixTag::TARGET_COMP_ID, &target_comp_id)?;
        validate_value(FixTag::TEXT, &qualifier)?;
        Ok(Self {
            version,
            sender_comp_id,
            target_comp_id,
            qualifier,
        })
    }

    /// Creates an owned id from a borrowed session id.
    pub fn from_borrowed(session_id: FixSessionId<'_>) -> Self {
        Self {
            version: session_id.version(),
            sender_comp_id: session_id.sender_comp_id().to_vec(),
            target_comp_id: session_id.target_comp_id().to_vec(),
            qualifier: session_id.qualifier().to_vec(),
        }
    }

    /// Returns a borrowed session id view.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] if stored bytes are invalid.
    pub fn as_borrowed(&self) -> Result<FixSessionId<'_>, FixEncodeError> {
        FixSessionId::with_qualifier(
            self.version,
            &self.sender_comp_id,
            &self.target_comp_id,
            &self.qualifier,
        )
    }

    /// Returns the FIX version.
    pub const fn version(&self) -> FixVersion {
        self.version
    }

    /// Returns `SenderCompID(49)`.
    pub fn sender_comp_id(&self) -> &[u8] {
        &self.sender_comp_id
    }

    /// Returns `TargetCompID(56)`.
    pub fn target_comp_id(&self) -> &[u8] {
        &self.target_comp_id
    }

    /// Returns the optional session qualifier.
    pub fn qualifier(&self) -> &[u8] {
        &self.qualifier
    }
}

/// Owned persistable sequence-state snapshot loaded from storage.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixOwnedSequenceSnapshot {
    session_id: FixOwnedSessionId,
    next_inbound: u64,
    next_outbound: u64,
    trading_day: Vec<u8>,
    checksum: u64,
}

impl FixOwnedSequenceSnapshot {
    /// Creates an owned sequence snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when `trading_day` contains SOH.
    pub fn new(
        session_id: FixOwnedSessionId,
        next_inbound: u64,
        next_outbound: u64,
        trading_day: impl Into<Vec<u8>>,
    ) -> Result<Self, FixEncodeError> {
        let trading_day = trading_day.into();
        validate_value(FixTag::TEXT, &trading_day)?;
        let mut snapshot = Self {
            session_id,
            next_inbound: clamp_seq_no(next_inbound),
            next_outbound: clamp_seq_no(next_outbound),
            trading_day,
            checksum: 0,
        };
        snapshot.checksum = sequence_snapshot_checksum_owned(&snapshot);
        Ok(snapshot)
    }

    /// Creates an owned snapshot from a borrowed sequence snapshot.
    pub fn from_borrowed(snapshot: &FixSequenceSnapshot<'_>) -> Self {
        let mut owned = Self {
            session_id: FixOwnedSessionId::from_borrowed(snapshot.session_id()),
            next_inbound: snapshot.next_inbound(),
            next_outbound: snapshot.next_outbound(),
            trading_day: snapshot.trading_day().to_vec(),
            checksum: 0,
        };
        owned.checksum = sequence_snapshot_checksum_owned(&owned);
        owned
    }

    /// Returns a borrowed snapshot view.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] if stored identity or trading-day bytes are invalid.
    pub fn as_borrowed(&self) -> Result<FixSequenceSnapshot<'_>, FixEncodeError> {
        FixSequenceSnapshot::new(
            self.session_id.as_borrowed()?,
            self.next_inbound,
            self.next_outbound,
            &self.trading_day,
        )
    }

    /// Returns the owned session id.
    pub const fn session_id(&self) -> &FixOwnedSessionId {
        &self.session_id
    }

    /// Returns the next inbound sequence number.
    pub const fn next_inbound(&self) -> u64 {
        self.next_inbound
    }

    /// Returns the next outbound sequence number.
    pub const fn next_outbound(&self) -> u64 {
        self.next_outbound
    }

    /// Returns the trading day bytes.
    pub fn trading_day(&self) -> &[u8] {
        &self.trading_day
    }

    /// Returns the stored snapshot checksum.
    pub const fn checksum(&self) -> u64 {
        self.checksum
    }

    /// Returns true when the stored checksum matches the snapshot payload.
    pub fn validate_checksum(&self) -> bool {
        self.checksum == sequence_snapshot_checksum_owned(self)
    }
}

/// File-backed FIX sequence snapshot store configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixSequenceStoreConfig {
    root: PathBuf,
    sync_on_save: bool,
}

impl FixSequenceStoreConfig {
    /// Creates a sequence store config rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            sync_on_save: true,
        }
    }

    /// Sets whether snapshot files are synced before atomic rename.
    pub const fn with_sync_on_save(mut self, sync_on_save: bool) -> Self {
        self.sync_on_save = sync_on_save;
        self
    }

    /// Returns the sequence snapshot root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns whether save operations sync snapshot bytes.
    pub const fn sync_on_save(&self) -> bool {
        self.sync_on_save
    }
}

/// Metadata for an installed FIX sequence snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixSequenceSnapshotManifest {
    /// Snapshot file path.
    pub path: PathBuf,
    /// Encoded snapshot bytes.
    pub bytes: u64,
    /// Snapshot checksum.
    pub checksum: u64,
    /// Next inbound sequence number.
    pub next_inbound: u64,
    /// Next outbound sequence number.
    pub next_outbound: u64,
}

/// Error returned by FIX sequence snapshot persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSequenceStoreError {
    /// Filesystem operation failed.
    Io(String),
    /// Snapshot value validation failed.
    Encode(FixEncodeError),
    /// Snapshot file magic does not match the expected format.
    InvalidMagic,
    /// Snapshot file version is not supported.
    UnsupportedVersion(u16),
    /// Snapshot file ended before a complete field could be decoded.
    Truncated,
    /// Snapshot payload has an invalid known FIX begin string.
    InvalidVersion,
    /// Encoded field length exceeds the supported snapshot format.
    FieldTooLarge,
    /// Snapshot checksum does not match the encoded payload.
    ChecksumMismatch {
        /// Checksum stored in the snapshot file.
        expected: u64,
        /// Checksum recomputed from the decoded snapshot payload.
        actual: u64,
    },
}

impl fmt::Display for FixSequenceStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "FIX sequence store I/O error: {err}"),
            Self::Encode(err) => write!(f, "FIX sequence store encode error: {err}"),
            Self::InvalidMagic => write!(f, "invalid FIX sequence snapshot magic"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported FIX sequence snapshot version {version}")
            }
            Self::Truncated => write!(f, "truncated FIX sequence snapshot"),
            Self::InvalidVersion => write!(f, "invalid FIX begin string in sequence snapshot"),
            Self::FieldTooLarge => write!(f, "FIX sequence snapshot field is too large"),
            Self::ChecksumMismatch { expected, actual } => write!(
                f,
                "FIX sequence snapshot checksum mismatch: expected {expected}, actual {actual}"
            ),
        }
    }
}

impl Error for FixSequenceStoreError {}

impl From<FixEncodeError> for FixSequenceStoreError {
    fn from(value: FixEncodeError) -> Self {
        Self::Encode(value)
    }
}

/// FIX sequence snapshot persistence contract.
pub trait FixSequenceSnapshotStore {
    /// Saves a sequence snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceStoreError`] when validation or storage fails.
    fn save_snapshot(
        &mut self,
        snapshot: &FixSequenceSnapshot<'_>,
    ) -> Result<FixSequenceSnapshotManifest, FixSequenceStoreError>;

    /// Loads the latest sequence snapshot, if present.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceStoreError`] when the snapshot cannot be decoded or
    /// its checksum does not validate.
    fn load_latest(&self) -> Result<Option<FixOwnedSequenceSnapshot>, FixSequenceStoreError>;
}

/// Atomic file-backed FIX sequence snapshot store.
#[derive(Debug, Clone)]
pub struct FileFixSequenceSnapshotStore {
    config: FixSequenceStoreConfig,
}

impl FileFixSequenceSnapshotStore {
    /// Opens or creates a file-backed sequence snapshot store.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceStoreError`] when the root cannot be created.
    pub fn open(config: FixSequenceStoreConfig) -> Result<Self, FixSequenceStoreError> {
        fs::create_dir_all(config.root()).map_err(io_error)?;
        Ok(Self { config })
    }

    /// Returns the store configuration.
    pub const fn config(&self) -> &FixSequenceStoreConfig {
        &self.config
    }

    /// Returns the latest snapshot path.
    pub fn snapshot_path(&self) -> PathBuf {
        self.config.root().join(SEQUENCE_SNAPSHOT_FILE)
    }

    fn temp_path(&self) -> PathBuf {
        self.config.root().join(SEQUENCE_SNAPSHOT_TMP_FILE)
    }
}

impl FixSequenceSnapshotStore for FileFixSequenceSnapshotStore {
    fn save_snapshot(
        &mut self,
        snapshot: &FixSequenceSnapshot<'_>,
    ) -> Result<FixSequenceSnapshotManifest, FixSequenceStoreError> {
        let owned = FixOwnedSequenceSnapshot::from_borrowed(snapshot);
        let bytes = encode_sequence_snapshot(&owned)?;
        let checksum = owned.checksum();
        let final_path = self.snapshot_path();
        let tmp_path = self.temp_path();

        {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&tmp_path)
                .map_err(io_error)?;
            file.write_all(&bytes).map_err(io_error)?;
            file.flush().map_err(io_error)?;
            if self.config.sync_on_save() {
                file.sync_all().map_err(io_error)?;
            }
        }

        fs::rename(&tmp_path, &final_path).map_err(io_error)?;

        Ok(FixSequenceSnapshotManifest {
            path: final_path,
            bytes: usize_to_u64(bytes.len()),
            checksum,
            next_inbound: owned.next_inbound(),
            next_outbound: owned.next_outbound(),
        })
    }

    fn load_latest(&self) -> Result<Option<FixOwnedSequenceSnapshot>, FixSequenceStoreError> {
        let path = self.snapshot_path();
        if !path.exists() {
            return Ok(None);
        }
        let mut file = File::open(path).map_err(io_error)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(io_error)?;
        decode_sequence_snapshot(&bytes).map(Some)
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

/// Direction of a captured FIX transcript frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixTranscriptDirection {
    /// Frame received from the counterparty.
    Inbound,
    /// Frame sent to the counterparty.
    Outbound,
}

impl FixTranscriptDirection {
    const fn as_byte(self) -> u8 {
        match self {
            Self::Inbound => b'I',
            Self::Outbound => b'O',
        }
    }
}

/// Fixed-size transcript message-type copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixTranscriptMsgType {
    len: u8,
    bytes: [u8; 8],
}

impl FixTranscriptMsgType {
    /// Creates an empty message-type marker.
    pub const fn empty() -> Self {
        Self {
            len: 0,
            bytes: [0; 8],
        }
    }

    /// Creates a transcript message type from wire bytes.
    ///
    /// # Errors
    ///
    /// Returns [`FixTranscriptError::MsgTypeTooLong`] when the message type
    /// exceeds the fixed transcript capacity.
    pub fn new(value: &[u8]) -> Result<Self, FixTranscriptError> {
        if value.len() > 8 {
            return Err(FixTranscriptError::MsgTypeTooLong {
                capacity: 8,
                actual: value.len(),
            });
        }
        let mut bytes = [0u8; 8];
        bytes[..value.len()].copy_from_slice(value);
        Ok(Self {
            len: value.len() as u8,
            bytes,
        })
    }

    /// Returns the copied message-type bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    /// Returns true when no message type was available.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for FixTranscriptMsgType {
    fn default() -> Self {
        Self::empty()
    }
}

/// Bounded transcript capture configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixTranscriptConfig {
    max_records: usize,
    max_raw_bytes: usize,
    retain_raw: bool,
}

impl FixTranscriptConfig {
    /// Creates a transcript capture configuration.
    ///
    /// A zero `max_records` disables record retention while counters and the
    /// rolling hash still advance. `max_raw_bytes` bounds retained raw frame
    /// bytes when `retain_raw` is true.
    pub const fn new(max_records: usize, max_raw_bytes: usize, retain_raw: bool) -> Self {
        Self {
            max_records,
            max_raw_bytes,
            retain_raw,
        }
    }

    /// Returns the maximum retained record count.
    pub const fn max_records(&self) -> usize {
        self.max_records
    }

    /// Returns the maximum retained raw byte count.
    pub const fn max_raw_bytes(&self) -> usize {
        self.max_raw_bytes
    }

    /// Returns whether raw FIX frames are retained when they fit.
    pub const fn retain_raw(&self) -> bool {
        self.retain_raw
    }
}

impl Default for FixTranscriptConfig {
    fn default() -> Self {
        Self {
            max_records: 4096,
            max_raw_bytes: 4 * 1024 * 1024,
            retain_raw: true,
        }
    }
}

/// Transcript capture errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixTranscriptError {
    /// Message type exceeded the fixed transcript capacity.
    MsgTypeTooLong {
        /// Configured capacity in bytes.
        capacity: usize,
        /// Actual supplied byte length.
        actual: usize,
    },
}

impl fmt::Display for FixTranscriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MsgTypeTooLong { capacity, actual } => write!(
                f,
                "FIX transcript message type length {actual} exceeds capacity {capacity}"
            ),
        }
    }
}

impl Error for FixTranscriptError {}

/// Retained transcript frame metadata and optional raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixTranscriptRecord {
    ordinal: u64,
    timestamp_ns: u64,
    direction: FixTranscriptDirection,
    seq_no: Option<u64>,
    msg_type: FixTranscriptMsgType,
    raw_len: usize,
    raw_checksum: u8,
    raw_hash: u64,
    raw_retained: bool,
    raw: Vec<u8>,
}

impl FixTranscriptRecord {
    /// Returns the one-based capture ordinal.
    pub const fn ordinal(&self) -> u64 {
        self.ordinal
    }

    /// Returns the caller-provided capture timestamp in nanoseconds.
    pub const fn timestamp_ns(&self) -> u64 {
        self.timestamp_ns
    }

    /// Returns the capture direction.
    pub const fn direction(&self) -> FixTranscriptDirection {
        self.direction
    }

    /// Returns `MsgSeqNum(34)` when known.
    pub const fn seq_no(&self) -> Option<u64> {
        self.seq_no
    }

    /// Returns the copied `MsgType(35)` bytes when known.
    pub fn msg_type(&self) -> &[u8] {
        self.msg_type.as_bytes()
    }

    /// Returns the original raw frame length.
    pub const fn raw_len(&self) -> usize {
        self.raw_len
    }

    /// Returns the FIX modulo-256 checksum over the raw frame bytes.
    pub const fn raw_checksum(&self) -> u8 {
        self.raw_checksum
    }

    /// Returns the FNV-1a hash over the raw frame bytes.
    pub const fn raw_hash(&self) -> u64 {
        self.raw_hash
    }

    /// Returns whether raw frame bytes were retained.
    pub const fn raw_retained(&self) -> bool {
        self.raw_retained
    }

    /// Returns retained raw frame bytes, or an empty slice when raw retention
    /// was disabled, oversized, or evicted with the record.
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }
}

/// Result of recording a transcript frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixTranscriptRetention {
    retained: bool,
    raw_retained: bool,
    evicted_records: u64,
    evicted_raw_bytes: u64,
}

impl FixTranscriptRetention {
    /// Returns whether the transcript record metadata was retained.
    pub const fn retained(&self) -> bool {
        self.retained
    }

    /// Returns whether raw frame bytes were retained.
    pub const fn raw_retained(&self) -> bool {
        self.raw_retained
    }

    /// Returns records evicted to satisfy configured bounds.
    pub const fn evicted_records(&self) -> u64 {
        self.evicted_records
    }

    /// Returns retained raw bytes evicted to satisfy configured bounds.
    pub const fn evicted_raw_bytes(&self) -> u64 {
        self.evicted_raw_bytes
    }
}

/// Snapshot of transcript capture counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixTranscriptMetrics {
    captured_records: u64,
    retained_records: u64,
    retained_raw_bytes: u64,
    dropped_records: u64,
    dropped_raw_bytes: u64,
    evicted_records: u64,
    evicted_raw_bytes: u64,
    oldest_ordinal: Option<u64>,
    newest_ordinal: Option<u64>,
    rolling_hash: u64,
}

impl FixTranscriptMetrics {
    /// Returns the total number of frames observed by the capture.
    pub const fn captured_records(&self) -> u64 {
        self.captured_records
    }

    /// Returns the number of retained transcript records.
    pub const fn retained_records(&self) -> u64 {
        self.retained_records
    }

    /// Returns retained raw frame bytes.
    pub const fn retained_raw_bytes(&self) -> u64 {
        self.retained_raw_bytes
    }

    /// Returns records dropped because record retention was disabled.
    pub const fn dropped_records(&self) -> u64 {
        self.dropped_records
    }

    /// Returns raw bytes not retained because raw retention was disabled,
    /// oversized, or record retention was disabled.
    pub const fn dropped_raw_bytes(&self) -> u64 {
        self.dropped_raw_bytes
    }

    /// Returns records evicted by bounded retention.
    pub const fn evicted_records(&self) -> u64 {
        self.evicted_records
    }

    /// Returns raw bytes evicted by bounded retention.
    pub const fn evicted_raw_bytes(&self) -> u64 {
        self.evicted_raw_bytes
    }

    /// Returns the oldest retained transcript ordinal.
    pub const fn oldest_ordinal(&self) -> Option<u64> {
        self.oldest_ordinal
    }

    /// Returns the newest captured transcript ordinal.
    pub const fn newest_ordinal(&self) -> Option<u64> {
        self.newest_ordinal
    }

    /// Returns the deterministic rolling hash over captured metadata and raw
    /// bytes.
    pub const fn rolling_hash(&self) -> u64 {
        self.rolling_hash
    }
}

/// Bounded in-memory FIX transcript capture.
///
/// The capture is intentionally transport- and storage-neutral. It records
/// frame metadata and optional raw bytes for certification/audit workflows,
/// while durable transcript archives can persist records according to their own
/// sync and redaction policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixTranscriptCapture {
    config: FixTranscriptConfig,
    records: VecDeque<FixTranscriptRecord>,
    retained_raw_bytes: usize,
    captured_records: u64,
    dropped_records: u64,
    dropped_raw_bytes: u64,
    evicted_records: u64,
    evicted_raw_bytes: u64,
    rolling_hash: u64,
}

impl Default for FixTranscriptCapture {
    fn default() -> Self {
        Self::new(FixTranscriptConfig::default())
    }
}

impl FixTranscriptCapture {
    /// Creates an empty transcript capture.
    pub fn new(config: FixTranscriptConfig) -> Self {
        Self {
            config,
            records: VecDeque::with_capacity(config.max_records.min(64)),
            retained_raw_bytes: 0,
            captured_records: 0,
            dropped_records: 0,
            dropped_raw_bytes: 0,
            evicted_records: 0,
            evicted_raw_bytes: 0,
            rolling_hash: FNV_OFFSET_BASIS,
        }
    }

    /// Returns the configured capture bounds.
    pub const fn config(&self) -> FixTranscriptConfig {
        self.config
    }

    /// Returns retained transcript records in capture order.
    pub fn records(&self) -> impl Iterator<Item = &FixTranscriptRecord> {
        self.records.iter()
    }

    /// Records a parsed validated FIX message.
    ///
    /// # Errors
    ///
    /// Returns [`FixTranscriptError`] when the parsed message type exceeds the
    /// fixed transcript capacity.
    pub fn record_message(
        &mut self,
        direction: FixTranscriptDirection,
        timestamp_ns: u64,
        message: &FixMessageView<'_>,
    ) -> Result<FixTranscriptRetention, FixTranscriptError> {
        self.record_frame(
            direction,
            timestamp_ns,
            message.msg_seq_num(),
            message.msg_type().unwrap_or(&[]),
            message.raw(),
        )
    }

    /// Records a raw FIX frame with caller-provided sequence and message type
    /// metadata.
    ///
    /// # Errors
    ///
    /// Returns [`FixTranscriptError`] when `msg_type` exceeds the fixed
    /// transcript capacity.
    pub fn record_frame(
        &mut self,
        direction: FixTranscriptDirection,
        timestamp_ns: u64,
        seq_no: Option<u64>,
        msg_type: &[u8],
        raw: &[u8],
    ) -> Result<FixTranscriptRetention, FixTranscriptError> {
        let msg_type = FixTranscriptMsgType::new(msg_type)?;
        self.captured_records = self.captured_records.saturating_add(1);
        let ordinal = self.captured_records;
        let raw_hash = hash_bytes(raw);
        let raw_checksum = checksum(raw);
        self.rolling_hash = update_transcript_hash(
            self.rolling_hash,
            ordinal,
            timestamp_ns,
            direction,
            seq_no,
            msg_type.as_bytes(),
            raw,
        );

        if self.config.max_records == 0 {
            self.dropped_records = self.dropped_records.saturating_add(1);
            self.dropped_raw_bytes = self
                .dropped_raw_bytes
                .saturating_add(usize_to_u64(raw.len()));
            return Ok(FixTranscriptRetention {
                retained: false,
                raw_retained: false,
                evicted_records: 0,
                evicted_raw_bytes: 0,
            });
        }

        let retain_raw = self.config.retain_raw
            && self.config.max_raw_bytes > 0
            && raw.len() <= self.config.max_raw_bytes;
        let raw_vec = if retain_raw { raw.to_vec() } else { Vec::new() };
        if !retain_raw && !raw.is_empty() {
            self.dropped_raw_bytes = self
                .dropped_raw_bytes
                .saturating_add(usize_to_u64(raw.len()));
        }
        let retained_raw_len = raw_vec.len();
        self.retained_raw_bytes = self.retained_raw_bytes.saturating_add(retained_raw_len);
        self.records.push_back(FixTranscriptRecord {
            ordinal,
            timestamp_ns,
            direction,
            seq_no,
            msg_type,
            raw_len: raw.len(),
            raw_checksum,
            raw_hash,
            raw_retained: retain_raw,
            raw: raw_vec,
        });

        let (evicted_records, evicted_raw_bytes) = self.evict_to_bounds();
        Ok(FixTranscriptRetention {
            retained: true,
            raw_retained: retain_raw,
            evicted_records,
            evicted_raw_bytes,
        })
    }

    /// Clears retained records and byte counters without resetting cumulative
    /// capture/drop/eviction counters or the rolling hash.
    pub fn clear_retained(&mut self) {
        self.records.clear();
        self.retained_raw_bytes = 0;
    }

    /// Returns transcript capture metrics.
    pub fn metrics(&self) -> FixTranscriptMetrics {
        FixTranscriptMetrics {
            captured_records: self.captured_records,
            retained_records: usize_to_u64(self.records.len()),
            retained_raw_bytes: usize_to_u64(self.retained_raw_bytes),
            dropped_records: self.dropped_records,
            dropped_raw_bytes: self.dropped_raw_bytes,
            evicted_records: self.evicted_records,
            evicted_raw_bytes: self.evicted_raw_bytes,
            oldest_ordinal: self.records.front().map(FixTranscriptRecord::ordinal),
            newest_ordinal: if self.captured_records == 0 {
                None
            } else {
                Some(self.captured_records)
            },
            rolling_hash: self.rolling_hash,
        }
    }

    fn evict_to_bounds(&mut self) -> (u64, u64) {
        let mut evicted_records = 0u64;
        let mut evicted_raw_bytes = 0u64;
        while self.records.len() > self.config.max_records
            || self.retained_raw_bytes > self.config.max_raw_bytes
        {
            let Some(record) = self.records.pop_front() else {
                break;
            };
            evicted_records = evicted_records.saturating_add(1);
            let raw_len = if record.raw_retained {
                record.raw.len()
            } else {
                0
            };
            self.retained_raw_bytes = self.retained_raw_bytes.saturating_sub(raw_len);
            evicted_raw_bytes = evicted_raw_bytes.saturating_add(usize_to_u64(raw_len));
        }
        self.evicted_records = self.evicted_records.saturating_add(evicted_records);
        self.evicted_raw_bytes = self.evicted_raw_bytes.saturating_add(evicted_raw_bytes);
        (evicted_records, evicted_raw_bytes)
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
    account: Option<&'a [u8]>,
    symbol: &'a [u8],
    side: FixOrderSide,
    transact_time: &'a [u8],
    order_qty: &'a [u8],
    ord_type: FixOrdType,
    price: Option<&'a [u8]>,
    stop_px: Option<&'a [u8]>,
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
            account: None,
            symbol,
            side,
            transact_time,
            order_qty,
            ord_type,
            price: None,
            stop_px: None,
            time_in_force: None,
        }
    }

    /// Adds `Account(1)`.
    pub const fn with_account(mut self, account: &'a [u8]) -> Self {
        self.account = Some(account);
        self
    }

    /// Adds `Price(44)`.
    pub const fn with_price(mut self, price: &'a [u8]) -> Self {
        self.price = Some(price);
        self
    }

    /// Adds `StopPx(99)`.
    pub const fn with_stop_px(mut self, stop_px: &'a [u8]) -> Self {
        self.stop_px = Some(stop_px);
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
    account: Option<&'a [u8]>,
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
            account: None,
            symbol,
            side,
            transact_time,
        }
    }

    /// Adds `Account(1)`.
    pub const fn with_account(mut self, account: &'a [u8]) -> Self {
        self.account = Some(account);
        self
    }
}

/// Borrowed OrderCancelReplaceRequest `<G>` request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixOrderCancelReplaceRequest<'a> {
    orig_cl_ord_id: &'a [u8],
    cl_ord_id: &'a [u8],
    account: Option<&'a [u8]>,
    symbol: &'a [u8],
    side: FixOrderSide,
    transact_time: &'a [u8],
    order_qty: &'a [u8],
    ord_type: FixOrdType,
    price: Option<&'a [u8]>,
    stop_px: Option<&'a [u8]>,
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
            account: None,
            symbol,
            side,
            transact_time,
            order_qty,
            ord_type,
            price: None,
            stop_px: None,
            time_in_force: None,
        }
    }

    /// Adds `Account(1)`.
    pub const fn with_account(mut self, account: &'a [u8]) -> Self {
        self.account = Some(account);
        self
    }

    /// Adds `Price(44)`.
    pub const fn with_price(mut self, price: &'a [u8]) -> Self {
        self.price = Some(price);
        self
    }

    /// Adds `StopPx(99)`.
    pub const fn with_stop_px(mut self, stop_px: &'a [u8]) -> Self {
        self.stop_px = Some(stop_px);
        self
    }

    /// Adds `TimeInForce(59)`.
    pub const fn with_time_in_force(mut self, time_in_force: FixTimeInForce) -> Self {
        self.time_in_force = Some(time_in_force);
        self
    }
}

/// Borrowed OrderStatusRequest `<H>` request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixOrderStatusRequest<'a> {
    cl_ord_id: &'a [u8],
    order_id: Option<&'a [u8]>,
}

impl<'a> FixOrderStatusRequest<'a> {
    /// Creates an OrderStatusRequest.
    pub const fn new(cl_ord_id: &'a [u8]) -> Self {
        Self {
            cl_ord_id,
            order_id: None,
        }
    }

    /// Adds `OrderID(37)` when known.
    pub const fn with_order_id(mut self, order_id: &'a [u8]) -> Self {
        self.order_id = Some(order_id);
        self
    }
}

/// Borrowed OrderMassCancelRequest `<q>` request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixOrderMassCancelRequest<'a> {
    cl_ord_id: &'a [u8],
    mass_cancel_request_type: FixMassCancelRequestType,
    transact_time: &'a [u8],
    secondary_cl_ord_id: Option<&'a [u8]>,
    trading_session_id: Option<&'a [u8]>,
    trading_session_sub_id: Option<&'a [u8]>,
    symbol: Option<&'a [u8]>,
    side: Option<FixOrderSide>,
    text: Option<&'a [u8]>,
}

impl<'a> FixOrderMassCancelRequest<'a> {
    /// Creates an OrderMassCancelRequest.
    pub const fn new(
        cl_ord_id: &'a [u8],
        mass_cancel_request_type: FixMassCancelRequestType,
        transact_time: &'a [u8],
    ) -> Self {
        Self {
            cl_ord_id,
            mass_cancel_request_type,
            transact_time,
            secondary_cl_ord_id: None,
            trading_session_id: None,
            trading_session_sub_id: None,
            symbol: None,
            side: None,
            text: None,
        }
    }

    /// Adds `SecondaryClOrdID(526)`.
    pub const fn with_secondary_cl_ord_id(mut self, secondary_cl_ord_id: &'a [u8]) -> Self {
        self.secondary_cl_ord_id = Some(secondary_cl_ord_id);
        self
    }

    /// Adds `TradingSessionID(336)`.
    pub const fn with_trading_session_id(mut self, trading_session_id: &'a [u8]) -> Self {
        self.trading_session_id = Some(trading_session_id);
        self
    }

    /// Adds `TradingSessionSubID(625)`.
    pub const fn with_trading_session_sub_id(mut self, trading_session_sub_id: &'a [u8]) -> Self {
        self.trading_session_sub_id = Some(trading_session_sub_id);
        self
    }

    /// Adds `Symbol(55)`.
    pub const fn with_symbol(mut self, symbol: &'a [u8]) -> Self {
        self.symbol = Some(symbol);
        self
    }

    /// Adds `Side(54)`.
    pub const fn with_side(mut self, side: FixOrderSide) -> Self {
        self.side = Some(side);
        self
    }

    /// Adds `Text(58)`.
    pub const fn with_text(mut self, text: &'a [u8]) -> Self {
        self.text = Some(text);
        self
    }
}

/// Borrowed OrderMassStatusRequest `<AF>` request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixOrderMassStatusRequest<'a> {
    mass_status_req_id: &'a [u8],
    mass_status_req_type: FixMassStatusReqType,
    account: Option<&'a [u8]>,
    acct_id_source: Option<&'a [u8]>,
    trading_session_id: Option<&'a [u8]>,
    trading_session_sub_id: Option<&'a [u8]>,
    symbol: Option<&'a [u8]>,
    side: Option<FixOrderSide>,
}

impl<'a> FixOrderMassStatusRequest<'a> {
    /// Creates an OrderMassStatusRequest.
    pub const fn new(
        mass_status_req_id: &'a [u8],
        mass_status_req_type: FixMassStatusReqType,
    ) -> Self {
        Self {
            mass_status_req_id,
            mass_status_req_type,
            account: None,
            acct_id_source: None,
            trading_session_id: None,
            trading_session_sub_id: None,
            symbol: None,
            side: None,
        }
    }

    /// Adds `Account(1)`.
    pub const fn with_account(mut self, account: &'a [u8]) -> Self {
        self.account = Some(account);
        self
    }

    /// Adds `AcctIDSource(660)`.
    pub const fn with_acct_id_source(mut self, acct_id_source: &'a [u8]) -> Self {
        self.acct_id_source = Some(acct_id_source);
        self
    }

    /// Adds `TradingSessionID(336)`.
    pub const fn with_trading_session_id(mut self, trading_session_id: &'a [u8]) -> Self {
        self.trading_session_id = Some(trading_session_id);
        self
    }

    /// Adds `TradingSessionSubID(625)`.
    pub const fn with_trading_session_sub_id(mut self, trading_session_sub_id: &'a [u8]) -> Self {
        self.trading_session_sub_id = Some(trading_session_sub_id);
        self
    }

    /// Adds `Symbol(55)`.
    pub const fn with_symbol(mut self, symbol: &'a [u8]) -> Self {
        self.symbol = Some(symbol);
        self
    }

    /// Adds `Side(54)`.
    pub const fn with_side(mut self, side: FixOrderSide) -> Self {
        self.side = Some(side);
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
        (FixTag::ACCOUNT, b"".as_slice()),
        (FixTag::SYMBOL, request.symbol),
        (FixTag::SIDE, request.side.as_bytes()),
        (FixTag::TRANSACT_TIME, request.transact_time),
        (FixTag::ORDER_QTY, request.order_qty),
        (FixTag::ORD_TYPE, request.ord_type.as_bytes()),
        (FixTag::PRICE, b"".as_slice()),
        (FixTag::STOP_PX, b"".as_slice()),
        (FixTag::TIME_IN_FORCE, b"".as_slice()),
    ];
    let mut len = 1usize;
    if let Some(account) = request.account {
        fields[len] = (FixTag::ACCOUNT, account);
        len += 1;
    }
    fields[len] = (FixTag::SYMBOL, request.symbol);
    len += 1;
    fields[len] = (FixTag::SIDE, request.side.as_bytes());
    len += 1;
    fields[len] = (FixTag::TRANSACT_TIME, request.transact_time);
    len += 1;
    fields[len] = (FixTag::ORDER_QTY, request.order_qty);
    len += 1;
    fields[len] = (FixTag::ORD_TYPE, request.ord_type.as_bytes());
    len += 1;
    if let Some(price) = request.price {
        fields[len] = (FixTag::PRICE, price);
        len += 1;
    }
    if let Some(stop_px) = request.stop_px {
        fields[len] = (FixTag::STOP_PX, stop_px);
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
    let mut fields = [
        (FixTag::ORIG_CL_ORD_ID, request.orig_cl_ord_id),
        (FixTag::CL_ORD_ID, request.cl_ord_id),
        (FixTag::ACCOUNT, b"".as_slice()),
        (FixTag::SYMBOL, request.symbol),
        (FixTag::SIDE, request.side.as_bytes()),
        (FixTag::TRANSACT_TIME, request.transact_time),
    ];
    let mut len = 2usize;
    if let Some(account) = request.account {
        fields[len] = (FixTag::ACCOUNT, account);
        len += 1;
    }
    fields[len] = (FixTag::SYMBOL, request.symbol);
    len += 1;
    fields[len] = (FixTag::SIDE, request.side.as_bytes());
    len += 1;
    fields[len] = (FixTag::TRANSACT_TIME, request.transact_time);
    len += 1;
    encode_session_message(
        out,
        version,
        FixMsgType::ORDER_CANCEL_REQUEST,
        header,
        &fields[..len],
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
        (FixTag::ACCOUNT, b"".as_slice()),
        (FixTag::SYMBOL, request.symbol),
        (FixTag::SIDE, request.side.as_bytes()),
        (FixTag::TRANSACT_TIME, request.transact_time),
        (FixTag::ORDER_QTY, request.order_qty),
        (FixTag::ORD_TYPE, request.ord_type.as_bytes()),
        (FixTag::PRICE, b"".as_slice()),
        (FixTag::STOP_PX, b"".as_slice()),
        (FixTag::TIME_IN_FORCE, b"".as_slice()),
    ];
    let mut len = 2usize;
    if let Some(account) = request.account {
        fields[len] = (FixTag::ACCOUNT, account);
        len += 1;
    }
    fields[len] = (FixTag::SYMBOL, request.symbol);
    len += 1;
    fields[len] = (FixTag::SIDE, request.side.as_bytes());
    len += 1;
    fields[len] = (FixTag::TRANSACT_TIME, request.transact_time);
    len += 1;
    fields[len] = (FixTag::ORDER_QTY, request.order_qty);
    len += 1;
    fields[len] = (FixTag::ORD_TYPE, request.ord_type.as_bytes());
    len += 1;
    if let Some(price) = request.price {
        fields[len] = (FixTag::PRICE, price);
        len += 1;
    }
    if let Some(stop_px) = request.stop_px {
        fields[len] = (FixTag::STOP_PX, stop_px);
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

/// Encodes an OrderStatusRequest `<H>` application message.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_order_status_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    request: FixOrderStatusRequest<'_>,
) -> Result<(), FixEncodeError> {
    let mut fields = [
        (FixTag::CL_ORD_ID, request.cl_ord_id),
        (FixTag::ORDER_ID, b"".as_slice()),
    ];
    let mut len = 1usize;
    if let Some(order_id) = request.order_id {
        fields[len] = (FixTag::ORDER_ID, order_id);
        len += 1;
    }
    encode_session_message(
        out,
        version,
        FixMsgType::ORDER_STATUS_REQUEST,
        header,
        &fields[..len],
    )
}

/// Encodes an OrderMassCancelRequest `<q>` application message.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_order_mass_cancel_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    request: FixOrderMassCancelRequest<'_>,
) -> Result<(), FixEncodeError> {
    let mut fields = [
        (FixTag::CL_ORD_ID, request.cl_ord_id),
        (
            FixTag::MASS_CANCEL_REQUEST_TYPE,
            request.mass_cancel_request_type.as_bytes(),
        ),
        (FixTag::TRANSACT_TIME, request.transact_time),
        (FixTag::SECONDARY_CL_ORD_ID, b"".as_slice()),
        (FixTag::TRADING_SESSION_ID, b"".as_slice()),
        (FixTag::TRADING_SESSION_SUB_ID, b"".as_slice()),
        (FixTag::SYMBOL, b"".as_slice()),
        (FixTag::SIDE, b"".as_slice()),
        (FixTag::TEXT, b"".as_slice()),
    ];
    let mut len = 3usize;
    if let Some(secondary_cl_ord_id) = request.secondary_cl_ord_id {
        fields[len] = (FixTag::SECONDARY_CL_ORD_ID, secondary_cl_ord_id);
        len += 1;
    }
    if let Some(trading_session_id) = request.trading_session_id {
        fields[len] = (FixTag::TRADING_SESSION_ID, trading_session_id);
        len += 1;
    }
    if let Some(trading_session_sub_id) = request.trading_session_sub_id {
        fields[len] = (FixTag::TRADING_SESSION_SUB_ID, trading_session_sub_id);
        len += 1;
    }
    if let Some(symbol) = request.symbol {
        fields[len] = (FixTag::SYMBOL, symbol);
        len += 1;
    }
    if let Some(side) = request.side {
        fields[len] = (FixTag::SIDE, side.as_bytes());
        len += 1;
    }
    if let Some(text) = request.text {
        fields[len] = (FixTag::TEXT, text);
        len += 1;
    }
    encode_session_message(
        out,
        version,
        FixMsgType::ORDER_MASS_CANCEL_REQUEST,
        header,
        &fields[..len],
    )
}

/// Encodes an OrderMassStatusRequest `<AF>` application message.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH.
pub fn encode_order_mass_status_request(
    out: &mut Vec<u8>,
    version: FixVersion,
    header: FixSessionHeader<'_>,
    request: FixOrderMassStatusRequest<'_>,
) -> Result<(), FixEncodeError> {
    let mut fields = [
        (FixTag::MASS_STATUS_REQ_ID, request.mass_status_req_id),
        (
            FixTag::MASS_STATUS_REQ_TYPE,
            request.mass_status_req_type.as_bytes(),
        ),
        (FixTag::ACCOUNT, b"".as_slice()),
        (FixTag::ACCT_ID_SOURCE, b"".as_slice()),
        (FixTag::TRADING_SESSION_ID, b"".as_slice()),
        (FixTag::TRADING_SESSION_SUB_ID, b"".as_slice()),
        (FixTag::SYMBOL, b"".as_slice()),
        (FixTag::SIDE, b"".as_slice()),
    ];
    let mut len = 2usize;
    if let Some(account) = request.account {
        fields[len] = (FixTag::ACCOUNT, account);
        len += 1;
    }
    if let Some(acct_id_source) = request.acct_id_source {
        fields[len] = (FixTag::ACCT_ID_SOURCE, acct_id_source);
        len += 1;
    }
    if let Some(trading_session_id) = request.trading_session_id {
        fields[len] = (FixTag::TRADING_SESSION_ID, trading_session_id);
        len += 1;
    }
    if let Some(trading_session_sub_id) = request.trading_session_sub_id {
        fields[len] = (FixTag::TRADING_SESSION_SUB_ID, trading_session_sub_id);
        len += 1;
    }
    if let Some(symbol) = request.symbol {
        fields[len] = (FixTag::SYMBOL, symbol);
        len += 1;
    }
    if let Some(side) = request.side {
        fields[len] = (FixTag::SIDE, side.as_bytes());
        len += 1;
    }
    encode_session_message(
        out,
        version,
        FixMsgType::ORDER_MASS_STATUS_REQUEST,
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

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn hash_u64(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn hash_bytes_into(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn sequence_snapshot_checksum_owned(snapshot: &FixOwnedSequenceSnapshot) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    hash = hash_bytes_into(hash, snapshot.session_id.version().as_bytes());
    hash = hash_bytes_into(hash, snapshot.session_id.sender_comp_id());
    hash = hash_bytes_into(hash, snapshot.session_id.target_comp_id());
    hash = hash_bytes_into(hash, snapshot.session_id.qualifier());
    hash = hash_u64(hash, snapshot.next_inbound());
    hash = hash_u64(hash, snapshot.next_outbound());
    hash_bytes_into(hash, snapshot.trading_day())
}

fn encode_sequence_snapshot(
    snapshot: &FixOwnedSequenceSnapshot,
) -> Result<Vec<u8>, FixSequenceStoreError> {
    let version = snapshot.session_id.version().as_bytes();
    let sender = snapshot.session_id.sender_comp_id();
    let target = snapshot.session_id.target_comp_id();
    let qualifier = snapshot.session_id.qualifier();
    let trading_day = snapshot.trading_day();
    let capacity = SEQUENCE_SNAPSHOT_MAGIC.len()
        + 2
        + 8
        + 8
        + 8
        + 5 * 2
        + version.len()
        + sender.len()
        + target.len()
        + qualifier.len()
        + trading_day.len();
    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(SEQUENCE_SNAPSHOT_MAGIC);
    put_snapshot_u16(&mut out, SEQUENCE_SNAPSHOT_VERSION);
    put_snapshot_u64(&mut out, snapshot.next_inbound());
    put_snapshot_u64(&mut out, snapshot.next_outbound());
    put_snapshot_bytes(&mut out, version)?;
    put_snapshot_bytes(&mut out, sender)?;
    put_snapshot_bytes(&mut out, target)?;
    put_snapshot_bytes(&mut out, qualifier)?;
    put_snapshot_bytes(&mut out, trading_day)?;
    put_snapshot_u64(&mut out, snapshot.checksum());
    Ok(out)
}

fn decode_sequence_snapshot(
    bytes: &[u8],
) -> Result<FixOwnedSequenceSnapshot, FixSequenceStoreError> {
    let mut cursor = SnapshotCursor::new(bytes);
    if cursor.read_exact(SEQUENCE_SNAPSHOT_MAGIC.len())? != SEQUENCE_SNAPSHOT_MAGIC {
        return Err(FixSequenceStoreError::InvalidMagic);
    }
    let version = cursor.read_u16()?;
    if version != SEQUENCE_SNAPSHOT_VERSION {
        return Err(FixSequenceStoreError::UnsupportedVersion(version));
    }
    let next_inbound = cursor.read_u64()?;
    let next_outbound = cursor.read_u64()?;
    let begin_string = cursor.read_vec()?;
    let sender_comp_id = cursor.read_vec()?;
    let target_comp_id = cursor.read_vec()?;
    let qualifier = cursor.read_vec()?;
    let trading_day = cursor.read_vec()?;
    let expected_checksum = cursor.read_u64()?;
    if !cursor.is_done() {
        return Err(FixSequenceStoreError::Truncated);
    }
    let version =
        FixVersion::from_bytes(&begin_string).ok_or(FixSequenceStoreError::InvalidVersion)?;
    let mut snapshot = FixOwnedSequenceSnapshot::new(
        FixOwnedSessionId::with_qualifier(version, sender_comp_id, target_comp_id, qualifier)?,
        next_inbound,
        next_outbound,
        trading_day,
    )?;
    let actual = snapshot.checksum();
    if expected_checksum != actual {
        return Err(FixSequenceStoreError::ChecksumMismatch {
            expected: expected_checksum,
            actual,
        });
    }
    snapshot.checksum = expected_checksum;
    Ok(snapshot)
}

fn put_snapshot_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_snapshot_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_snapshot_bytes(out: &mut Vec<u8>, value: &[u8]) -> Result<(), FixSequenceStoreError> {
    let len = u16::try_from(value.len()).map_err(|_| FixSequenceStoreError::FieldTooLarge)?;
    put_snapshot_u16(out, len);
    out.extend_from_slice(value);
    Ok(())
}

fn io_error(err: std::io::Error) -> FixSequenceStoreError {
    FixSequenceStoreError::Io(err.to_string())
}

struct SnapshotCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> SnapshotCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], FixSequenceStoreError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(FixSequenceStoreError::Truncated)?;
        if end > self.bytes.len() {
            return Err(FixSequenceStoreError::Truncated);
        }
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn read_u16(&mut self) -> Result<u16, FixSequenceStoreError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u64(&mut self) -> Result<u64, FixSequenceStoreError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_vec(&mut self) -> Result<Vec<u8>, FixSequenceStoreError> {
        let len = usize::from(self.read_u16()?);
        Ok(self.read_exact(len)?.to_vec())
    }

    const fn is_done(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

fn update_transcript_hash(
    mut hash: u64,
    ordinal: u64,
    timestamp_ns: u64,
    direction: FixTranscriptDirection,
    seq_no: Option<u64>,
    msg_type: &[u8],
    raw: &[u8],
) -> u64 {
    hash = hash_u64(hash, ordinal);
    hash = hash_u64(hash, timestamp_ns);
    hash ^= u64::from(direction.as_byte());
    hash = hash.wrapping_mul(FNV_PRIME);
    hash = hash_u64(hash, seq_no.unwrap_or(0));
    hash = hash_bytes_into(hash, msg_type);
    hash_bytes_into(hash, raw)
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
    fn file_sequence_snapshot_store_saves_and_loads_latest() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-fix-sequence-store-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);

        let session_id =
            FixSessionId::with_qualifier(FixVersion::Fix44, b"CLIENT", b"BROKER", b"PRIMARY")
                .expect("session");
        let snapshot = FixSequenceSnapshot::new(session_id, 42, 77, b"20260726").expect("snapshot");
        let mut store = FileFixSequenceSnapshotStore::open(
            FixSequenceStoreConfig::new(&root).with_sync_on_save(false),
        )
        .expect("store");

        let manifest = store.save_snapshot(&snapshot).expect("save");
        assert_eq!(manifest.next_inbound, 42);
        assert_eq!(manifest.next_outbound, 77);
        assert!(manifest.bytes > 0);

        let loaded = store.load_latest().expect("load").expect("snapshot");
        assert!(loaded.validate_checksum());
        assert_eq!(loaded.session_id().version(), FixVersion::Fix44);
        assert_eq!(loaded.session_id().sender_comp_id(), b"CLIENT");
        assert_eq!(loaded.session_id().target_comp_id(), b"BROKER");
        assert_eq!(loaded.session_id().qualifier(), b"PRIMARY");
        assert_eq!(loaded.trading_day(), b"20260726");

        let borrowed = loaded.as_borrowed().expect("borrowed");
        let restored = FixSequenceTracker::from_snapshot(&borrowed);
        assert_eq!(restored.next_inbound(), 42);
        assert_eq!(restored.next_outbound(), 77);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn file_sequence_snapshot_store_returns_none_when_empty() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-fix-sequence-store-empty-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = FileFixSequenceSnapshotStore::open(
            FixSequenceStoreConfig::new(&root).with_sync_on_save(false),
        )
        .expect("store");

        assert!(store.load_latest().expect("load").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn file_sequence_snapshot_store_rejects_corrupt_checksum() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-fix-sequence-store-corrupt-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);

        let session_id =
            FixSessionId::new(FixVersion::Fix42, b"CLIENT", b"BROKER").expect("session");
        let snapshot = FixSequenceSnapshot::new(session_id, 12, 21, b"20260726").expect("snapshot");
        let mut store = FileFixSequenceSnapshotStore::open(
            FixSequenceStoreConfig::new(&root).with_sync_on_save(false),
        )
        .expect("store");
        store.save_snapshot(&snapshot).expect("save");

        let path = store.snapshot_path();
        let mut bytes = fs::read(&path).expect("read");
        let last = bytes.last_mut().expect("byte");
        *last ^= 0x01;
        fs::write(&path, bytes).expect("write");

        let err = store.load_latest().expect_err("checksum");
        assert!(matches!(
            err,
            FixSequenceStoreError::ChecksumMismatch { .. }
        ));
        let _ = fs::remove_dir_all(&root);
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
    fn transcript_capture_records_parsed_messages() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 2, b"20260717-12:00:01.000");
        let mut raw = Vec::new();
        encode_heartbeat(&mut raw, FixVersion::Fix44, header, Some(b"T1")).expect("heartbeat");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        let mut capture = FixTranscriptCapture::new(FixTranscriptConfig::new(4, 1024, true));
        let retention = capture
            .record_message(
                FixTranscriptDirection::Outbound,
                1_784_275_200_000_000_000,
                &message,
            )
            .expect("record");

        assert!(retention.retained());
        assert!(retention.raw_retained());
        let metrics = capture.metrics();
        assert_eq!(metrics.captured_records(), 1);
        assert_eq!(metrics.retained_records(), 1);
        assert_eq!(metrics.retained_raw_bytes(), raw.len() as u64);
        assert_ne!(metrics.rolling_hash(), FNV_OFFSET_BASIS);
        let record = capture.records().next().expect("record");
        assert_eq!(record.ordinal(), 1);
        assert_eq!(record.direction(), FixTranscriptDirection::Outbound);
        assert_eq!(record.seq_no(), Some(2));
        assert_eq!(record.msg_type(), FixMsgType::HEARTBEAT.as_bytes());
        assert_eq!(record.raw_checksum(), checksum(&raw));
        assert_eq!(record.raw(), raw.as_slice());
    }

    #[test]
    fn transcript_capture_evicts_to_bounds() {
        let mut capture = FixTranscriptCapture::new(FixTranscriptConfig::new(2, 64, true));
        let first = capture
            .record_frame(
                FixTranscriptDirection::Inbound,
                1,
                Some(1),
                b"0",
                b"8=FIX.4.4\x0135=0\x01",
            )
            .expect("record first");
        assert!(first.retained());
        let second = capture
            .record_frame(
                FixTranscriptDirection::Outbound,
                2,
                Some(2),
                b"1",
                b"8=FIX.4.4\x0135=1\x01",
            )
            .expect("record second");
        assert!(second.retained());
        let third = capture
            .record_frame(
                FixTranscriptDirection::Inbound,
                3,
                Some(3),
                b"2",
                b"8=FIX.4.4\x0135=2\x01",
            )
            .expect("record third");

        assert_eq!(third.evicted_records(), 1);
        let metrics = capture.metrics();
        assert_eq!(metrics.captured_records(), 3);
        assert_eq!(metrics.retained_records(), 2);
        assert_eq!(metrics.evicted_records(), 1);
        assert_eq!(metrics.oldest_ordinal(), Some(2));
        assert_eq!(metrics.newest_ordinal(), Some(3));
    }

    #[test]
    fn transcript_capture_can_keep_metadata_without_raw() {
        let mut capture = FixTranscriptCapture::new(FixTranscriptConfig::new(4, 8, true));
        let retention = capture
            .record_frame(
                FixTranscriptDirection::Inbound,
                1,
                Some(1),
                b"D",
                b"this raw frame is intentionally too large",
            )
            .expect("record");

        assert!(retention.retained());
        assert!(!retention.raw_retained());
        let metrics = capture.metrics();
        assert_eq!(metrics.retained_records(), 1);
        assert_eq!(metrics.retained_raw_bytes(), 0);
        assert_eq!(
            metrics.dropped_raw_bytes(),
            "this raw frame is intentionally too large".len() as u64
        );
        let record = capture.records().next().expect("record");
        assert_eq!(
            record.raw_len(),
            "this raw frame is intentionally too large".len()
        );
        assert!(!record.raw_retained());
        assert!(record.raw().is_empty());
    }

    #[test]
    fn transcript_capture_respects_disabled_record_retention() {
        let mut capture = FixTranscriptCapture::new(FixTranscriptConfig::new(0, 1024, true));
        let retention = capture
            .record_frame(FixTranscriptDirection::Inbound, 1, None, b"0", b"raw")
            .expect("record");

        assert!(!retention.retained());
        assert!(!retention.raw_retained());
        let metrics = capture.metrics();
        assert_eq!(metrics.captured_records(), 1);
        assert_eq!(metrics.retained_records(), 0);
        assert_eq!(metrics.dropped_records(), 1);
        assert_eq!(metrics.dropped_raw_bytes(), 3);
        assert_ne!(metrics.rolling_hash(), FNV_OFFSET_BASIS);
    }

    #[test]
    fn transcript_message_type_is_bounded() {
        let mut capture = FixTranscriptCapture::default();
        assert_eq!(
            capture.record_frame(
                FixTranscriptDirection::Inbound,
                1,
                None,
                b"TOO-LONG-MSG-TYPE",
                b"raw",
            ),
            Err(FixTranscriptError::MsgTypeTooLong {
                capacity: 8,
                actual: 17,
            })
        );
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
        .with_account(b"ACC")
        .with_price(b"65000.5")
        .with_stop_px(b"64950")
        .with_time_in_force(FixTimeInForce::Day);

        let mut raw = Vec::new();
        encode_new_order_single(&mut raw, FixVersion::Fix44, header, request).expect("new order");

        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.typed_msg_type(), Some(FixMsgType::NEW_ORDER_SINGLE));
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"ORD-1".as_slice()));
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::SYMBOL), Some(b"BTCUSDT".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"1".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"1.25".as_slice()));
        assert_eq!(message.get(FixTag::PRICE), Some(b"65000.5".as_slice()));
        assert_eq!(message.get(FixTag::STOP_PX), Some(b"64950".as_slice()));
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
        )
        .with_account(b"ACC");

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
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
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
        .with_account(b"ACC")
        .with_price(b"65100")
        .with_stop_px(b"65000")
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
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_QTY), Some(b"2.00".as_slice()));
        assert_eq!(message.get(FixTag::STOP_PX), Some(b"65000".as_slice()));
        assert_eq!(message.get(FixTag::TIME_IN_FORCE), Some(b"3".as_slice()));
    }

    #[test]
    fn encodes_order_status_request() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 10, b"20260717-12:00:08.000");
        let request = FixOrderStatusRequest::new(b"ORD-1").with_order_id(b"VENUE-1");
        let mut raw = Vec::new();
        encode_order_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("status request");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.msg_type(),
            Some(FixMsgType::ORDER_STATUS_REQUEST.as_bytes())
        );
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"ORD-1".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_ID), Some(b"VENUE-1".as_slice()));
    }

    #[test]
    fn order_status_request_allows_minimal_required_shape() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 10, b"20260717-12:00:08.000");
        let request = FixOrderStatusRequest::new(b"ORD-1");
        let mut raw = Vec::new();
        encode_order_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("status request");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"ORD-1".as_slice()));
        assert_eq!(message.get(FixTag::ORDER_ID), None);
    }

    #[test]
    fn order_status_request_rejects_soh() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 10, b"20260717-12:00:08.000");
        let request = FixOrderStatusRequest::new(b"ORD\x01");
        let mut raw = Vec::new();
        let err = encode_order_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect_err("soh");
        assert_eq!(err, FixEncodeError::ValueContainsSoh(FixTag::CL_ORD_ID));
    }

    #[test]
    fn encodes_order_mass_cancel_request() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 11, b"20260717-12:00:09.000");
        let request = FixOrderMassCancelRequest::new(
            b"MASS-1",
            FixMassCancelRequestType::Security,
            b"20260717-12:00:09.000",
        )
        .with_secondary_cl_ord_id(b"ALT-1")
        .with_trading_session_id(b"REG")
        .with_trading_session_sub_id(b"AM")
        .with_symbol(b"BTCUSDT")
        .with_side(FixOrderSide::Buy)
        .with_text(b"cancel symbol");
        let mut raw = Vec::new();
        encode_order_mass_cancel_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("mass cancel");
        let mut scratch = [FixFieldView::empty(); 24];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.msg_type(),
            Some(FixMsgType::ORDER_MASS_CANCEL_REQUEST.as_bytes())
        );
        assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"MASS-1".as_slice()));
        assert_eq!(
            message.get(FixTag::MASS_CANCEL_REQUEST_TYPE),
            Some(b"1".as_slice())
        );
        assert_eq!(
            message.get(FixTag::TRANSACT_TIME),
            Some(b"20260717-12:00:09.000".as_slice())
        );
        assert_eq!(message.get(FixTag::SYMBOL), Some(b"BTCUSDT".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"1".as_slice()));
        assert_eq!(message.get(FixTag::TEXT), Some(b"cancel symbol".as_slice()));
    }

    #[test]
    fn order_mass_cancel_request_allows_minimal_required_shape() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 11, b"20260717-12:00:09.000");
        let request = FixOrderMassCancelRequest::new(
            b"MASS-1",
            FixMassCancelRequestType::AllOrders,
            b"20260717-12:00:09.000",
        );
        let mut raw = Vec::new();
        encode_order_mass_cancel_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("mass cancel");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.get(FixTag::MASS_CANCEL_REQUEST_TYPE),
            Some(b"7".as_slice())
        );
        assert_eq!(message.get(FixTag::SYMBOL), None);
    }

    #[test]
    fn order_mass_cancel_request_rejects_soh() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 11, b"20260717-12:00:09.000");
        let request = FixOrderMassCancelRequest::new(
            b"MASS-1",
            FixMassCancelRequestType::Security,
            b"20260717-12:00:09.000",
        )
        .with_text(b"bad\x01text");
        let mut raw = Vec::new();
        let err = encode_order_mass_cancel_request(&mut raw, FixVersion::Fix44, header, request)
            .expect_err("soh");
        assert_eq!(err, FixEncodeError::ValueContainsSoh(FixTag::TEXT));
    }

    #[test]
    fn encodes_order_mass_status_request() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 12, b"20260717-12:00:10.000");
        let request = FixOrderMassStatusRequest::new(b"MS-1", FixMassStatusReqType::Security)
            .with_account(b"ACC")
            .with_acct_id_source(b"1")
            .with_trading_session_id(b"REG")
            .with_trading_session_sub_id(b"AM")
            .with_symbol(b"BTCUSDT")
            .with_side(FixOrderSide::Sell);
        let mut raw = Vec::new();
        encode_order_mass_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("mass status");
        let mut scratch = [FixFieldView::empty(); 24];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.msg_type(),
            Some(FixMsgType::ORDER_MASS_STATUS_REQUEST.as_bytes())
        );
        assert_eq!(
            message.get(FixTag::MASS_STATUS_REQ_ID),
            Some(b"MS-1".as_slice())
        );
        assert_eq!(
            message.get(FixTag::MASS_STATUS_REQ_TYPE),
            Some(b"1".as_slice())
        );
        assert_eq!(message.get(FixTag::ACCOUNT), Some(b"ACC".as_slice()));
        assert_eq!(message.get(FixTag::SYMBOL), Some(b"BTCUSDT".as_slice()));
        assert_eq!(message.get(FixTag::SIDE), Some(b"2".as_slice()));
    }

    #[test]
    fn order_mass_status_request_allows_minimal_required_shape() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 12, b"20260717-12:00:10.000");
        let request = FixOrderMassStatusRequest::new(b"MS-1", FixMassStatusReqType::AllOrders);
        let mut raw = Vec::new();
        encode_order_mass_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect("mass status");
        let mut scratch = [FixFieldView::empty(); 16];
        let message = parse_message(&raw, &mut scratch).expect("parse");
        assert_eq!(
            message.get(FixTag::MASS_STATUS_REQ_TYPE),
            Some(b"7".as_slice())
        );
        assert_eq!(message.get(FixTag::ACCOUNT), None);
    }

    #[test]
    fn order_mass_status_request_rejects_soh() {
        let header = FixSessionHeader::new(b"CLIENT", b"BROKER", 12, b"20260717-12:00:10.000");
        let request = FixOrderMassStatusRequest::new(b"MS\x01", FixMassStatusReqType::AllOrders);
        let mut raw = Vec::new();
        let err = encode_order_mass_status_request(&mut raw, FixVersion::Fix44, header, request)
            .expect_err("soh");
        assert_eq!(
            err,
            FixEncodeError::ValueContainsSoh(FixTag::MASS_STATUS_REQ_ID)
        );
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
