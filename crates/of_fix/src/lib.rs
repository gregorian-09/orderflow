//! Low-allocation FIX tag-value codec primitives for Orderflow.
#![doc = include_str!("../README.md")]

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
    /// `MsgType(35)`.
    pub const MSG_TYPE: Self = Self(35);
    /// `MsgSeqNum(34)`.
    pub const MSG_SEQ_NUM: Self = Self(34);
    /// `PossDupFlag(43)`.
    pub const POSS_DUP_FLAG: Self = Self(43);
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
    /// `CheckSum(10)`.
    pub const CHECK_SUM: Self = Self(10);
}

impl fmt::Display for FixTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
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

    /// Returns `MsgSeqNum(34)` parsed as `u64`.
    pub fn msg_seq_num(&self) -> Option<u64> {
        parse_u64(self.get(FixTag::MSG_SEQ_NUM)?).ok()
    }

    /// Returns true when `PossDupFlag(43)` is `Y`.
    pub fn poss_dup(&self) -> bool {
        self.get(FixTag::POSS_DUP_FLAG) == Some(b"Y".as_slice())
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
}

impl fmt::Display for FixEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueContainsSoh(tag) => write!(f, "FIX value for tag {tag} contains SOH"),
            Self::ReservedTag(tag) => write!(f, "FIX tag {tag} is owned by the encoder"),
        }
    }
}

impl Error for FixEncodeError {}

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
    validate_value(FixTag::BEGIN_STRING, begin_string)?;
    validate_value(FixTag::MSG_TYPE, msg_type)?;

    out.clear();
    write_field(out, FixTag::BEGIN_STRING, begin_string);
    write_field(out, FixTag::BODY_LENGTH, b"0000000000");
    let body_start = out.len();
    write_field(out, FixTag::MSG_TYPE, msg_type);
    for (tag, value) in fields {
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
}
