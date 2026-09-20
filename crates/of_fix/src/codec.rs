use super::*;

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

/// Encodes a FIX message with one flat repeating group appended to `fields`.
///
/// `definition` supplies the count tag, delimiter tag, and allowed entry tags.
/// The count field is emitted automatically, followed by each entry in the
/// order supplied by `groups`. This API intentionally handles one flat group;
/// nested groups remain a profile-specific extension above this low-level
/// codec.
///
/// # Errors
///
/// Returns [`FixEncodeError`] when a field value contains SOH, a reserved tag
/// is supplied, or an entry does not satisfy `definition`.
pub fn encode_message_with_repeating_group(
    out: &mut Vec<u8>,
    begin_string: &[u8],
    msg_type: &[u8],
    fields: &[(FixTag, &[u8])],
    definition: FixRepeatingGroupDefinition<'_>,
    groups: &[&[(FixTag, &[u8])]],
) -> Result<(), FixEncodeError> {
    encode_message_parts_with_repeating_group(
        out,
        begin_string,
        msg_type,
        &[],
        fields,
        Some(definition),
        groups,
    )
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

pub(crate) fn encode_session_message(
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

pub(crate) fn encode_message_parts(
    out: &mut Vec<u8>,
    begin_string: &[u8],
    msg_type: &[u8],
    header_fields: &[(FixTag, &[u8])],
    fields: &[(FixTag, &[u8])],
) -> Result<(), FixEncodeError> {
    encode_message_parts_with_repeating_group(
        out,
        begin_string,
        msg_type,
        header_fields,
        fields,
        None,
        &[],
    )
}

pub(crate) fn encode_message_parts_with_repeating_group(
    out: &mut Vec<u8>,
    begin_string: &[u8],
    msg_type: &[u8],
    header_fields: &[(FixTag, &[u8])],
    fields: &[(FixTag, &[u8])],
    definition: Option<FixRepeatingGroupDefinition<'_>>,
    groups: &[&[(FixTag, &[u8])]],
) -> Result<(), FixEncodeError> {
    validate_value(FixTag::BEGIN_STRING, begin_string)?;
    validate_value(FixTag::MSG_TYPE, msg_type)?;

    if let Some(definition) = definition {
        if matches!(
            definition.count_tag,
            FixTag::BEGIN_STRING | FixTag::BODY_LENGTH | FixTag::MSG_TYPE | FixTag::CHECK_SUM
        ) {
            return Err(FixEncodeError::ReservedTag(definition.count_tag));
        }
        if header_fields
            .iter()
            .chain(fields.iter())
            .any(|(tag, _)| *tag == definition.count_tag)
        {
            return Err(FixEncodeError::DuplicateRepeatingGroupCountTag(
                definition.count_tag,
            ));
        }
        validate_repeating_group(definition, groups)?;
    }

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
    if let Some(definition) = definition {
        let mut count = [0u8; 20];
        let count_len = write_u64_digits(&mut count, usize_to_u64(groups.len()));
        write_field(out, definition.count_tag, &count[..count_len]);
        for group in groups {
            for (tag, value) in group.iter().copied() {
                write_field(out, tag, value);
            }
        }
    }

    let body_len = out.len().saturating_sub(body_start);
    patch_body_length(out, body_len);
    let sum = checksum(out);
    write_checksum(out, sum);
    Ok(())
}

pub(crate) fn validate_repeating_group(
    definition: FixRepeatingGroupDefinition<'_>,
    groups: &[&[(FixTag, &[u8])]],
) -> Result<(), FixEncodeError> {
    for (group_index, group) in groups.iter().enumerate() {
        let first = group
            .first()
            .ok_or(FixEncodeError::MissingRepeatingGroupDelimiter {
                group: group_index,
                tag: definition.delimiter_tag,
            })?;
        if first.0 != definition.delimiter_tag {
            return Err(FixEncodeError::MissingRepeatingGroupDelimiter {
                group: group_index,
                tag: definition.delimiter_tag,
            });
        }
        for (field_index, &(tag, value)) in group.iter().enumerate() {
            if matches!(
                tag,
                FixTag::BEGIN_STRING | FixTag::BODY_LENGTH | FixTag::MSG_TYPE | FixTag::CHECK_SUM
            ) {
                return Err(FixEncodeError::ReservedTag(tag));
            }
            if !definition.allows(tag) {
                return Err(FixEncodeError::InvalidRepeatingGroupField {
                    group: group_index,
                    index: field_index,
                    tag,
                });
            }
            if field_index > 0 && tag == definition.delimiter_tag {
                return Err(FixEncodeError::RepeatedRepeatingGroupDelimiter {
                    group: group_index,
                    index: field_index,
                    tag,
                });
            }
            validate_value(tag, value)?;
        }
    }
    Ok(())
}

/// Computes a FIX modulo-256 checksum.
pub fn checksum(bytes: &[u8]) -> u8 {
    bytes
        .iter()
        .fold(0u32, |acc, byte| acc.wrapping_add(u32::from(*byte))) as u8
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub(crate) fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

pub(crate) fn hash_u64(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub(crate) fn hash_bytes_into(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub(crate) fn sequence_snapshot_checksum_owned(snapshot: &FixOwnedSequenceSnapshot) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    hash = hash_bytes_into(hash, snapshot.session_id.version().as_bytes());
    hash = hash_bytes_into(hash, snapshot.session_id.sender_comp_id());
    hash = hash_bytes_into(hash, snapshot.session_id.target_comp_id());
    hash = hash_bytes_into(hash, snapshot.session_id.qualifier());
    hash = hash_u64(hash, snapshot.next_inbound());
    hash = hash_u64(hash, snapshot.next_outbound());
    hash_bytes_into(hash, snapshot.trading_day())
}

pub(crate) fn encode_sequence_snapshot(
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

pub(crate) fn decode_sequence_snapshot(
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

pub(crate) fn put_snapshot_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_snapshot_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_snapshot_bytes(
    out: &mut Vec<u8>,
    value: &[u8],
) -> Result<(), FixSequenceStoreError> {
    let len = u16::try_from(value.len()).map_err(|_| FixSequenceStoreError::FieldTooLarge)?;
    put_snapshot_u16(out, len);
    out.extend_from_slice(value);
    Ok(())
}

pub(crate) fn io_error(err: std::io::Error) -> FixSequenceStoreError {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableResendRecord {
    pub(crate) seq_no: u64,
    pub(crate) kind: FixSentMessageKind,
    pub(crate) raw: Vec<u8>,
    pub(crate) encoded_bytes: u64,
    pub(crate) checksum: u64,
}

pub(crate) fn encode_durable_resend_record(
    out: &mut Vec<u8>,
    seq_no: u64,
    kind: FixSentMessageKind,
    raw: &[u8],
    previous_checksum: u64,
) -> Result<(), FixDurableResendStoreError> {
    let raw_len =
        u32::try_from(raw.len()).map_err(|_| FixDurableResendStoreError::FrameTooLarge)?;
    let raw_hash = hash_bytes(raw);
    let frame_checksum = durable_resend_frame_checksum(seq_no, kind, raw, previous_checksum);

    out.clear();
    out.reserve(DURABLE_RESEND_MAGIC.len() + 2 + 1 + 8 + 4 + 8 + 8 + 8 + raw.len());
    out.extend_from_slice(DURABLE_RESEND_MAGIC);
    put_snapshot_u16(out, DURABLE_RESEND_VERSION);
    out.push(durable_resend_kind_to_byte(kind));
    put_snapshot_u64(out, seq_no);
    out.extend_from_slice(&raw_len.to_le_bytes());
    put_snapshot_u64(out, raw_hash);
    put_snapshot_u64(out, previous_checksum);
    put_snapshot_u64(out, frame_checksum);
    out.extend_from_slice(raw);
    Ok(())
}

pub(crate) fn durable_resend_frame_checksum(
    seq_no: u64,
    kind: FixSentMessageKind,
    raw: &[u8],
    previous_checksum: u64,
) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    hash = hash_u64(hash, u64::from(DURABLE_RESEND_VERSION));
    hash ^= u64::from(durable_resend_kind_to_byte(kind));
    hash = hash.wrapping_mul(FNV_PRIME);
    hash = hash_u64(hash, seq_no);
    hash = hash_u64(hash, usize_to_u64(raw.len()));
    hash = hash_u64(hash, hash_bytes(raw));
    hash = hash_u64(hash, previous_checksum);
    hash_bytes_into(hash, raw)
}

pub(crate) fn durable_resend_kind_to_byte(kind: FixSentMessageKind) -> u8 {
    match kind {
        FixSentMessageKind::Application => 1,
        FixSentMessageKind::Administrative => 2,
        FixSentMessageKind::Reject => 3,
    }
}

pub(crate) fn durable_resend_kind_from_byte(
    kind: u8,
) -> Result<FixSentMessageKind, FixDurableResendStoreError> {
    match kind {
        1 => Ok(FixSentMessageKind::Application),
        2 => Ok(FixSentMessageKind::Administrative),
        3 => Ok(FixSentMessageKind::Reject),
        _ => Err(FixDurableResendStoreError::InvalidKind(kind)),
    }
}

pub(crate) fn inspect_durable_resend_path(
    path: &Path,
) -> Result<FixDurableResendReplayReport, FixDurableResendStoreError> {
    if !path.exists() {
        return Ok(FixDurableResendReplayReport::default());
    }
    let records = read_durable_resend_records(path)?;
    let mut report = FixDurableResendReplayReport::default();
    for record in records {
        report.records = report.records.saturating_add(1);
        report.bytes = report.bytes.saturating_add(record.encoded_bytes);
        report.first_seq_no.get_or_insert(record.seq_no);
        report.last_seq_no = Some(record.seq_no);
        report.checksum = record.checksum;
    }
    Ok(report)
}

pub(crate) fn read_durable_resend_records(
    path: &Path,
) -> Result<Vec<DurableResendRecord>, FixDurableResendStoreError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut file = File::open(path).map_err(durable_resend_io_error)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(durable_resend_io_error)?;
    decode_durable_resend_records(&bytes)
}

pub(crate) fn decode_durable_resend_records(
    bytes: &[u8],
) -> Result<Vec<DurableResendRecord>, FixDurableResendStoreError> {
    let mut cursor = DurableResendCursor::new(bytes);
    let mut records = Vec::new();
    let mut previous_checksum = 0_u64;
    let mut latest_seq_no = None;

    while !cursor.is_done() {
        let offset = cursor.offset;
        if cursor.read_exact(DURABLE_RESEND_MAGIC.len())? != DURABLE_RESEND_MAGIC {
            return Err(FixDurableResendStoreError::InvalidMagic);
        }
        let version = cursor.read_u16()?;
        if version != DURABLE_RESEND_VERSION {
            return Err(FixDurableResendStoreError::UnsupportedVersion(version));
        }
        let kind = durable_resend_kind_from_byte(cursor.read_u8()?)?;
        let seq_no = cursor.read_u64()?;
        if let Some(latest) = latest_seq_no {
            if seq_no <= latest {
                return Err(FixDurableResendStoreError::SequenceRegression {
                    latest,
                    received: seq_no,
                });
            }
        }
        let raw_len = usize::try_from(cursor.read_u32()?)
            .map_err(|_| FixDurableResendStoreError::FrameTooLarge)?;
        let expected_raw_hash = cursor.read_u64()?;
        let expected_previous_checksum = cursor.read_u64()?;
        if expected_previous_checksum != previous_checksum {
            return Err(FixDurableResendStoreError::PreviousChecksumMismatch {
                expected: expected_previous_checksum,
                actual: previous_checksum,
            });
        }
        let expected_frame_checksum = cursor.read_u64()?;
        let raw = cursor.read_exact(raw_len)?.to_vec();
        let actual_raw_hash = hash_bytes(&raw);
        if expected_raw_hash != actual_raw_hash {
            return Err(FixDurableResendStoreError::RawHashMismatch {
                expected: expected_raw_hash,
                actual: actual_raw_hash,
            });
        }
        let actual_frame_checksum =
            durable_resend_frame_checksum(seq_no, kind, &raw, previous_checksum);
        if expected_frame_checksum != actual_frame_checksum {
            return Err(FixDurableResendStoreError::FrameChecksumMismatch {
                expected: expected_frame_checksum,
                actual: actual_frame_checksum,
            });
        }
        previous_checksum = actual_frame_checksum;
        latest_seq_no = Some(seq_no);
        records.push(DurableResendRecord {
            seq_no,
            kind,
            raw,
            encoded_bytes: usize_to_u64(cursor.offset.saturating_sub(offset)),
            checksum: actual_frame_checksum,
        });
    }

    Ok(records)
}

pub(crate) fn durable_resend_io_error(err: std::io::Error) -> FixDurableResendStoreError {
    FixDurableResendStoreError::Io(err.to_string())
}

struct DurableResendCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> DurableResendCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], FixDurableResendStoreError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(FixDurableResendStoreError::Truncated)?;
        if end > self.bytes.len() {
            return Err(FixDurableResendStoreError::Truncated);
        }
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8, FixDurableResendStoreError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, FixDurableResendStoreError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, FixDurableResendStoreError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, FixDurableResendStoreError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    const fn is_done(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

pub(crate) fn update_transcript_hash(
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

pub(crate) fn write_field(out: &mut Vec<u8>, tag: FixTag, value: &[u8]) {
    write_u32(out, tag.0);
    out.push(b'=');
    out.extend_from_slice(value);
    out.push(SOH);
}

pub(crate) fn write_replay_header(
    out: &mut Vec<u8>,
    sending_time: &[u8],
    orig_sending_time: &[u8],
) {
    write_field(out, FixTag::POSS_DUP_FLAG, b"Y");
    write_field(out, FixTag::SENDING_TIME, sending_time);
    write_field(out, FixTag::ORIG_SENDING_TIME, orig_sending_time);
}

pub(crate) fn write_checksum(out: &mut Vec<u8>, sum: u8) {
    out.extend_from_slice(b"10=");
    out.push(b'0' + (sum / 100));
    out.push(b'0' + ((sum / 10) % 10));
    out.push(b'0' + (sum % 10));
    out.push(SOH);
}

pub(crate) fn patch_body_length(out: &mut [u8], body_len: usize) {
    let Some(tag_start) = find_tag_start(out, FixTag::BODY_LENGTH) else {
        return;
    };
    let value_start = tag_start + 2;
    let mut digits = [b'0'; 10];
    write_usize_padded(&mut digits, body_len);
    out[value_start..value_start + digits.len()].copy_from_slice(&digits);
}

pub(crate) fn find_tag_start(raw: &[u8], tag: FixTag) -> Option<usize> {
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

pub(crate) fn validate_value(tag: FixTag, value: &[u8]) -> Result<(), FixEncodeError> {
    if value.contains(&SOH) {
        Err(FixEncodeError::ValueContainsSoh(tag))
    } else {
        Ok(())
    }
}

pub(crate) fn parse_required_u64(
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

pub(crate) fn parse_optional_reject_u64(
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

pub(crate) fn parse_optional_fix_tag(
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

pub(crate) fn push_gap_fill<'a>(
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

pub(crate) const fn clamp_seq_no(value: u64) -> u64 {
    if value == 0 {
        1
    } else {
        value
    }
}

pub(crate) fn parse_checksum(value: &[u8]) -> Result<u8, FixParseError> {
    if value.len() != 3 || !value.iter().all(u8::is_ascii_digit) {
        return Err(FixParseError::InvalidChecksum);
    }
    Ok((value[0] - b'0') * 100 + (value[1] - b'0') * 10 + (value[2] - b'0'))
}

pub(crate) fn parse_u32(bytes: &[u8]) -> Result<u32, ()> {
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

pub(crate) fn parse_u64(bytes: &[u8]) -> Result<u64, ()> {
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

pub(crate) fn parse_usize(bytes: &[u8]) -> Result<usize, ()> {
    let parsed = parse_u64(bytes)?;
    usize::try_from(parsed).map_err(|_| ())
}

pub(crate) fn write_u32(out: &mut Vec<u8>, value: u32) {
    let mut digits = [0u8; 10];
    let len = write_u32_digits(&mut digits, value);
    out.extend_from_slice(&digits[..len]);
}

pub(crate) fn write_usize_padded(out: &mut [u8; 10], value: usize) {
    let mut digits = [0u8; 20];
    let len = write_usize_digits(&mut digits, value);
    let out_len = out.len();
    let copied_len = len.min(out_len);
    let start = out_len.saturating_sub(copied_len);
    let digit_start = len.saturating_sub(out_len);
    out[start..].copy_from_slice(&digits[digit_start..len]);
}

pub(crate) fn write_u32_digits(out: &mut [u8; 10], mut value: u32) -> usize {
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

pub(crate) fn write_u64_digits(out: &mut [u8; 20], mut value: u64) -> usize {
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

pub(crate) fn write_usize_digits(out: &mut [u8; 20], mut value: usize) -> usize {
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
