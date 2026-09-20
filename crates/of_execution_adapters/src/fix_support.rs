use super::*;

pub(crate) fn map_exec_type(value: FixExecType) -> ExecutionType {
    match value {
        FixExecType::New => ExecutionType::Ack,
        FixExecType::Rejected => ExecutionType::Reject,
        FixExecType::Trade => ExecutionType::Trade,
        FixExecType::PendingCancel => ExecutionType::CancelPending,
        FixExecType::Canceled => ExecutionType::CancelAck,
        FixExecType::PendingReplace => ExecutionType::ReplacePending,
        FixExecType::Replaced => ExecutionType::ReplaceAck,
        FixExecType::Expired => ExecutionType::Expire,
        FixExecType::Restated => ExecutionType::Restated,
    }
}

pub(crate) fn map_ord_status(value: FixOrdStatus) -> OrderStatus {
    match value {
        FixOrdStatus::New => OrderStatus::New,
        FixOrdStatus::PartiallyFilled => OrderStatus::PartiallyFilled,
        FixOrdStatus::Filled => OrderStatus::Filled,
        FixOrdStatus::DoneForDay => OrderStatus::Suspended,
        FixOrdStatus::Canceled => OrderStatus::Cancelled,
        FixOrdStatus::Replaced => OrderStatus::Replaced,
        FixOrdStatus::PendingCancel => OrderStatus::PendingCancel,
        FixOrdStatus::Stopped => OrderStatus::Suspended,
        FixOrdStatus::Rejected => OrderStatus::Rejected,
        FixOrdStatus::Suspended => OrderStatus::Suspended,
        FixOrdStatus::PendingNew => OrderStatus::PendingNew,
        FixOrdStatus::Expired => OrderStatus::Expired,
        FixOrdStatus::PendingReplace => OrderStatus::PendingReplace,
    }
}

pub(crate) fn parse_exec_type(value: &[u8]) -> Result<FixExecType, FixReportParseError> {
    match value {
        b"0" => Ok(FixExecType::New),
        b"1" | b"2" | b"F" => Ok(FixExecType::Trade),
        b"4" => Ok(FixExecType::Canceled),
        b"5" => Ok(FixExecType::Replaced),
        b"6" => Ok(FixExecType::PendingCancel),
        b"8" => Ok(FixExecType::Rejected),
        b"C" => Ok(FixExecType::Expired),
        b"D" | b"I" => Ok(FixExecType::Restated),
        b"E" => Ok(FixExecType::PendingReplace),
        _ => Err(FixReportParseError::InvalidExecType),
    }
}

pub(crate) fn parse_ord_status(value: &[u8]) -> Result<FixOrdStatus, FixReportParseError> {
    match value {
        b"0" => Ok(FixOrdStatus::New),
        b"1" => Ok(FixOrdStatus::PartiallyFilled),
        b"2" => Ok(FixOrdStatus::Filled),
        b"3" => Ok(FixOrdStatus::DoneForDay),
        b"4" => Ok(FixOrdStatus::Canceled),
        b"5" => Ok(FixOrdStatus::Replaced),
        b"6" => Ok(FixOrdStatus::PendingCancel),
        b"7" => Ok(FixOrdStatus::Stopped),
        b"8" => Ok(FixOrdStatus::Rejected),
        b"9" => Ok(FixOrdStatus::Suspended),
        b"A" => Ok(FixOrdStatus::PendingNew),
        b"C" => Ok(FixOrdStatus::Expired),
        b"E" => Ok(FixOrdStatus::PendingReplace),
        _ => Err(FixReportParseError::InvalidOrdStatus),
    }
}

pub(crate) fn parse_cancel_reject_response_to(
    value: &[u8],
) -> Result<FixCancelRejectResponseTo, FixReportParseError> {
    match value {
        b"1" => Ok(FixCancelRejectResponseTo::OrderCancelRequest),
        b"2" => Ok(FixCancelRejectResponseTo::OrderCancelReplaceRequest),
        _ => Err(FixReportParseError::InvalidCancelRejectResponseTo),
    }
}

pub(crate) fn map_side_to_fix(value: OrderSide) -> FixOrderSide {
    match value {
        OrderSide::Buy => FixOrderSide::Buy,
        OrderSide::Sell => FixOrderSide::Sell,
    }
}

pub(crate) fn map_order_type_to_fix(value: OrderType) -> Result<FixOrdType, FixRequestEncodeError> {
    match value {
        OrderType::Market => Ok(FixOrdType::Market),
        OrderType::Limit => Ok(FixOrdType::Limit),
        OrderType::Stop => Ok(FixOrdType::Stop),
        OrderType::StopLimit => Ok(FixOrdType::StopLimit),
    }
}

pub(crate) fn map_tif_to_fix(value: TimeInForce) -> FixTimeInForce {
    match value {
        TimeInForce::Day => FixTimeInForce::Day,
        TimeInForce::Gtc => FixTimeInForce::GoodTillCancel,
        TimeInForce::Ioc => FixTimeInForce::ImmediateOrCancel,
        TimeInForce::Fok => FixTimeInForce::FillOrKill,
        TimeInForce::Gtd => FixTimeInForce::GoodTillDate,
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScaledField {
    Quantity,
    Price,
}

pub(crate) fn encode_order_price(
    buf: &mut [u8; 40],
    order_type: OrderType,
    price: OrderPrice,
    config: FixRequestEncodeConfig,
) -> Result<Option<&[u8]>, FixRequestEncodeError> {
    match order_type {
        OrderType::Market => Ok(None),
        OrderType::Limit => {
            encode_scaled(buf, price.0, config.price_scale, ScaledField::Price).map(Some)
        }
        OrderType::Stop => Ok(None),
        OrderType::StopLimit => {
            encode_scaled(buf, price.0, config.price_scale, ScaledField::Price).map(Some)
        }
    }
}

pub(crate) fn encode_order_stop_price(
    buf: &mut [u8; 40],
    order_type: OrderType,
    stop_price: OrderPrice,
    config: FixRequestEncodeConfig,
) -> Result<Option<&[u8]>, FixRequestEncodeError> {
    match order_type {
        OrderType::Market | OrderType::Limit => Ok(None),
        OrderType::Stop | OrderType::StopLimit => {
            encode_scaled(buf, stop_price.0, config.price_scale, ScaledField::Price).map(Some)
        }
    }
}

pub(crate) fn encode_scaled(
    buf: &mut [u8; 40],
    value: i64,
    scale: i64,
    field: ScaledField,
) -> Result<&[u8], FixRequestEncodeError> {
    if value <= 0 {
        return Err(match field {
            ScaledField::Quantity => FixRequestEncodeError::InvalidQuantity,
            ScaledField::Price => FixRequestEncodeError::InvalidPrice,
        });
    }
    let places = decimal_places(scale)?;
    let value = u64::try_from(value).map_err(|_| match field {
        ScaledField::Quantity => FixRequestEncodeError::InvalidQuantity,
        ScaledField::Price => FixRequestEncodeError::InvalidPrice,
    })?;
    let scale = u64::try_from(scale).map_err(|_| FixRequestEncodeError::InvalidScale)?;
    let whole = value / scale;
    let mut pos = write_u64_ascii(buf, whole);
    let rem = value % scale;
    if rem == 0 {
        return Ok(&buf[..pos]);
    }
    buf[pos] = b'.';
    pos += 1;
    let frac_start = pos;
    pos += write_padded_u64_ascii(&mut buf[pos..], rem, places);
    while pos > frac_start && buf[pos - 1] == b'0' {
        pos -= 1;
    }
    Ok(&buf[..pos])
}

pub(crate) fn decimal_places(scale: i64) -> Result<usize, FixRequestEncodeError> {
    if scale < 1 {
        return Err(FixRequestEncodeError::InvalidScale);
    }
    let mut scale = scale;
    let mut places = 0usize;
    while scale > 1 {
        if scale % 10 != 0 {
            return Err(FixRequestEncodeError::InvalidScale);
        }
        scale /= 10;
        places += 1;
    }
    Ok(places)
}

pub(crate) fn write_u64_ascii(buf: &mut [u8], value: u64) -> usize {
    if value == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 20];
    let mut len = 0usize;
    let mut n = value;
    while n > 0 {
        tmp[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    for idx in 0..len {
        buf[idx] = tmp[len - idx - 1];
    }
    len
}

pub(crate) fn write_padded_u64_ascii(buf: &mut [u8], value: u64, width: usize) -> usize {
    let mut tmp = [b'0'; 20];
    let mut len = 0usize;
    let mut n = value;
    while n > 0 {
        tmp[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    let padding = width.saturating_sub(len);
    for slot in buf.iter_mut().take(padding) {
        *slot = b'0';
    }
    for idx in 0..len {
        buf[padding + idx] = tmp[len - idx - 1];
    }
    padding + len
}

pub(crate) fn required<'a>(
    message: &FixMessageView<'a>,
    tag: FixTag,
) -> Result<&'a [u8], FixReportParseError> {
    message.get(tag).ok_or(FixReportParseError::MissingTag(tag))
}

pub(crate) fn fixed_required<const N: usize>(
    message: &FixMessageView<'_>,
    tag: FixTag,
) -> Result<FixedAscii<N>, FixReportParseError> {
    fixed_from_bytes(tag, required(message, tag)?)
}

pub(crate) fn fixed_optional<const N: usize>(
    message: &FixMessageView<'_>,
    tag: FixTag,
) -> Result<FixedAscii<N>, FixReportParseError> {
    if let Some(value) = message.get(tag) {
        fixed_from_bytes(tag, value)
    } else {
        Ok(FixedAscii::empty())
    }
}

pub(crate) fn fixed_from_bytes<const N: usize>(
    tag: FixTag,
    value: &[u8],
) -> Result<FixedAscii<N>, FixReportParseError> {
    let value = std::str::from_utf8(value).map_err(|_| FixReportParseError::InvalidAscii {
        tag,
        source: ExecutionCoreError::NonAsciiIdentifier,
    })?;
    FixedAscii::new(value).map_err(|source| FixReportParseError::InvalidAscii { tag, source })
}

pub(crate) fn parse_optional_scaled(
    message: &FixMessageView<'_>,
    tag: FixTag,
    scale: i64,
) -> Result<i64, FixReportParseError> {
    if let Some(value) = message.get(tag) {
        parse_scaled(value, scale).ok_or(FixReportParseError::InvalidNumber(tag))
    } else {
        Ok(0)
    }
}

pub(crate) fn parse_optional_u64(
    message: &FixMessageView<'_>,
    tag: FixTag,
) -> Result<u64, FixReportParseError> {
    if let Some(value) = message.get(tag) {
        if tag == FixTag::TRANSACT_TIME {
            parse_u64_digits(value)
                .or_else(|| parse_fix_utc_timestamp_ns(value))
                .ok_or(FixReportParseError::InvalidNumber(tag))
        } else {
            parse_u64_digits(value).ok_or(FixReportParseError::InvalidNumber(tag))
        }
    } else {
        Ok(0)
    }
}

pub(crate) fn parse_fix_utc_timestamp_ns(value: &[u8]) -> Option<u64> {
    if value.len() < 17
        || value.get(8) != Some(&b'-')
        || value.get(11) != Some(&b':')
        || value.get(14) != Some(&b':')
    {
        return None;
    }
    let year = parse_fixed_digits(value.get(0..4)?)? as i64;
    let month = parse_fixed_digits(value.get(4..6)?)? as u32;
    let day = parse_fixed_digits(value.get(6..8)?)? as u32;
    let hour = parse_fixed_digits(value.get(9..11)?)? as u32;
    let minute = parse_fixed_digits(value.get(12..14)?)? as u32;
    let second = parse_fixed_digits(value.get(15..17)?)? as u32;
    if year < 1970
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let fractional_ns = match value.get(17..) {
        Some([]) | None => 0,
        Some(fraction) if fraction.first() == Some(&b'.') => {
            let digits = &fraction[1..];
            if digits.is_empty() || digits.len() > 9 || !digits.iter().all(u8::is_ascii_digit) {
                return None;
            }
            let fraction = parse_fixed_digits(digits)?;
            fraction.checked_mul(10u64.checked_pow(9u32.saturating_sub(digits.len() as u32))?)?
        }
        _ => return None,
    };

    let days = days_from_civil(year, month, day)?;
    let seconds = u64::try_from(days)
        .ok()?
        .checked_mul(86_400)?
        .checked_add(u64::from(hour) * 3_600)?
        .checked_add(u64::from(minute) * 60)?
        .checked_add(u64::from(second))?;
    seconds
        .checked_mul(1_000_000_000)?
        .checked_add(fractional_ns)
}

pub(crate) fn parse_fixed_digits(value: &[u8]) -> Option<u64> {
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return None;
    }
    value.iter().try_fold(0u64, |current, byte| {
        current
            .checked_mul(10)?
            .checked_add(u64::from(*byte - b'0'))
    })
}

pub(crate) fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

pub(crate) fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    let adjusted_year = year.checked_sub(i64::from(month <= 2))?;
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year.checked_sub(era.checked_mul(400)?)?;
    let adjusted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153i64.checked_mul(adjusted_month)?.checked_add(2)? / 5)
        .checked_add(i64::from(day).checked_sub(1)?)?;
    let day_of_era = year_of_era
        .checked_mul(365)?
        .checked_add(year_of_era / 4)?
        .checked_sub(year_of_era / 100)?
        .checked_add(day_of_year)?;
    era.checked_mul(146_097)?
        .checked_add(day_of_era)?
        .checked_sub(719_468)
}

pub(crate) fn parse_scaled(value: &[u8], scale: i64) -> Option<i64> {
    if value.is_empty() || scale < 1 {
        return None;
    }

    let mut int = 0i64;
    let mut frac = 0i64;
    let mut frac_divisor = 1i64;
    let mut seen_dot = false;

    for byte in value {
        if *byte == b'.' && !seen_dot {
            seen_dot = true;
            continue;
        }
        if !byte.is_ascii_digit() {
            return None;
        }
        let digit = i64::from(*byte - b'0');
        if seen_dot {
            frac_divisor = frac_divisor.checked_mul(10)?;
            frac = frac.checked_mul(10)?.checked_add(digit)?;
        } else {
            int = int.checked_mul(10)?.checked_add(digit)?;
        }
    }

    let scaled_int = int.checked_mul(scale)?;
    if frac_divisor == 1 {
        return Some(scaled_int);
    }
    if scale % frac_divisor != 0 {
        return None;
    }
    scaled_int.checked_add(frac.checked_mul(scale / frac_divisor)?)
}

pub(crate) fn parse_u64_digits(value: &[u8]) -> Option<u64> {
    if value.is_empty() {
        return None;
    }
    let mut out = 0u64;
    for byte in value {
        if !byte.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add(u64::from(*byte - b'0'))?;
    }
    Some(out)
}
