use super::*;

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

/// Describes one flat FIX repeating group.
///
/// `count_tag` identifies the field containing the number of entries,
/// `delimiter_tag` identifies the first field of every entry, and `field_tags`
/// lists every tag allowed inside an entry. The slices are borrowed so a venue
/// profile can define them as static data and reuse the definition across
/// messages without allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixRepeatingGroupDefinition<'a> {
    pub(crate) count_tag: FixTag,
    pub(crate) delimiter_tag: FixTag,
    pub(crate) field_tags: &'a [FixTag],
}

impl<'a> FixRepeatingGroupDefinition<'a> {
    /// Creates a flat repeating-group definition.
    pub const fn new(count_tag: FixTag, delimiter_tag: FixTag, field_tags: &'a [FixTag]) -> Self {
        Self {
            count_tag,
            delimiter_tag,
            field_tags,
        }
    }

    /// Returns the group count tag.
    pub const fn count_tag(self) -> FixTag {
        self.count_tag
    }

    /// Returns the first-field delimiter tag.
    pub const fn delimiter_tag(self) -> FixTag {
        self.delimiter_tag
    }

    /// Returns all tags allowed inside each group entry.
    pub const fn field_tags(self) -> &'a [FixTag] {
        self.field_tags
    }

    pub(crate) fn allows(self, tag: FixTag) -> bool {
        self.field_tags.contains(&tag)
    }
}

/// Caller-owned boundary storage for one parsed repeating-group entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixRepeatingGroupEntry {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl FixRepeatingGroupEntry {
    /// Creates an empty boundary slot for caller-provided scratch storage.
    pub const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    /// Returns the field range occupied by this entry in the parsed message.
    pub const fn field_range(self) -> (usize, usize) {
        (self.start, self.end)
    }
}

/// Borrowed fields for one repeating-group entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixRepeatingGroup<'a> {
    pub(crate) fields: &'a [FixFieldView<'a>],
}

impl<'a> FixRepeatingGroup<'a> {
    /// Returns the entry fields in wire order.
    pub const fn fields(self) -> &'a [FixFieldView<'a>] {
        self.fields
    }

    /// Returns the first value for `tag` in this entry.
    pub fn get(self, tag: FixTag) -> Option<&'a [u8]> {
        self.fields
            .iter()
            .find(|field| field.tag == tag)
            .map(|field| field.value)
    }
}

/// Borrowed view over the entries of one flat repeating group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixRepeatingGroupView<'a, 'scratch> {
    pub(crate) fields: &'a [FixFieldView<'a>],
    pub(crate) entries: &'scratch [FixRepeatingGroupEntry],
}

impl<'a, 'scratch> FixRepeatingGroupView<'a, 'scratch> {
    /// Returns the number of parsed group entries.
    pub const fn len(self) -> usize {
        self.entries.len()
    }

    /// Returns true when no entries were declared.
    pub const fn is_empty(self) -> bool {
        self.entries.is_empty()
    }

    /// Returns one entry by zero-based index.
    pub fn get(self, index: usize) -> Option<FixRepeatingGroup<'a>> {
        let entry = self.entries.get(index)?;
        Some(FixRepeatingGroup {
            fields: &self.fields[entry.start..entry.end],
        })
    }

    /// Iterates over entries in wire order without allocating.
    pub fn iter(&self) -> impl Iterator<Item = FixRepeatingGroup<'a>> + '_ {
        self.entries.iter().map(move |entry| FixRepeatingGroup {
            fields: &self.fields[entry.start..entry.end],
        })
    }
}

/// Borrowed view over a validated FIX message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixMessageView<'a> {
    pub(crate) raw: &'a [u8],
    pub(crate) fields: &'a [FixFieldView<'a>],
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

    /// Parses one flat repeating group using caller-provided boundary scratch.
    ///
    /// The group count, delimiter, and allowed fields come from `definition`.
    /// The returned entries borrow the original parsed fields; only their
    /// start/end boundaries are written to `scratch`. Nested groups are not
    /// interpreted by this API.
    ///
    /// # Errors
    ///
    /// Returns [`FixGroupError`] when the count is missing or malformed, the
    /// scratch capacity is insufficient, an entry delimiter is missing, or an
    /// entry contains a tag outside the definition.
    pub fn repeating_group<'scratch>(
        &self,
        definition: FixRepeatingGroupDefinition<'_>,
        scratch: &'scratch mut [FixRepeatingGroupEntry],
    ) -> Result<FixRepeatingGroupView<'a, 'scratch>, FixGroupError> {
        let count_index = self
            .fields
            .iter()
            .position(|field| field.tag == definition.count_tag)
            .ok_or(FixGroupError::MissingCountTag(definition.count_tag))?;
        let count_value = self.fields[count_index].value;
        let count = parse_usize(count_value)
            .map_err(|_| FixGroupError::InvalidCount(definition.count_tag))?;
        if count > scratch.len() {
            return Err(FixGroupError::ScratchTooSmall {
                required: count,
                capacity: scratch.len(),
            });
        }

        let mut cursor = count_index.saturating_add(1);
        for (index, entry) in scratch.iter_mut().take(count).enumerate() {
            if cursor >= self.fields.len() || self.fields[cursor].tag != definition.delimiter_tag {
                return Err(FixGroupError::MissingDelimiter {
                    index,
                    tag: definition.delimiter_tag,
                });
            }
            let start = cursor;
            cursor = cursor.saturating_add(1);
            while cursor < self.fields.len() {
                let field = self.fields[cursor];
                if field.tag == definition.delimiter_tag {
                    break;
                }
                if !definition.allows(field.tag) {
                    break;
                }
                cursor = cursor.saturating_add(1);
            }
            *entry = FixRepeatingGroupEntry { start, end: cursor };
        }

        for (index, field) in self.fields.iter().enumerate().skip(cursor) {
            if definition.allows(field.tag) {
                return Err(FixGroupError::UnexpectedField {
                    index,
                    tag: field.tag,
                });
            }
        }

        Ok(FixRepeatingGroupView {
            fields: self.fields,
            entries: &scratch[..count],
        })
    }
}
