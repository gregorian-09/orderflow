use super::*;

/// Validation rule for one FIX message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixMessageRule<'a> {
    pub(crate) msg_type: FixMsgType,
    pub(crate) required_tags: &'a [FixTag],
    pub(crate) disallowed_tags: &'a [FixTag],
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
    pub(crate) version: FixVersion,
    pub(crate) rules: &'a [FixMessageRule<'a>],
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
    pub(crate) buffer: Vec<u8>,
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

    /// Encodes a message with one flat repeating group appended to `fields`.
    ///
    /// The group count is generated from `groups.len()`. Each entry must begin
    /// with the definition delimiter and contain only tags listed by the
    /// definition. The internal buffer is reused and no group-owned heap
    /// allocation is performed by the encoder.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when a value contains SOH, a reserved tag is
    /// supplied, or a group entry violates `definition`.
    pub fn encode_with_repeating_group(
        &mut self,
        begin_string: &[u8],
        msg_type: &[u8],
        fields: &[(FixTag, &[u8])],
        definition: FixRepeatingGroupDefinition<'_>,
        groups: &[&[(FixTag, &[u8])]],
    ) -> Result<&[u8], FixEncodeError> {
        encode_message_with_repeating_group(
            &mut self.buffer,
            begin_string,
            msg_type,
            fields,
            definition,
            groups,
        )?;
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
    pub(crate) sender_comp_id: &'a [u8],
    pub(crate) target_comp_id: &'a [u8],
    pub(crate) msg_seq_num: u64,
    pub(crate) sending_time: &'a [u8],
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
    pub(crate) cl_ord_id: &'a [u8],
    pub(crate) account: Option<&'a [u8]>,
    pub(crate) symbol: &'a [u8],
    pub(crate) side: FixOrderSide,
    pub(crate) transact_time: &'a [u8],
    pub(crate) order_qty: &'a [u8],
    pub(crate) ord_type: FixOrdType,
    pub(crate) price: Option<&'a [u8]>,
    pub(crate) stop_px: Option<&'a [u8]>,
    pub(crate) time_in_force: Option<FixTimeInForce>,
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
    pub(crate) orig_cl_ord_id: &'a [u8],
    pub(crate) cl_ord_id: &'a [u8],
    pub(crate) account: Option<&'a [u8]>,
    pub(crate) symbol: &'a [u8],
    pub(crate) side: FixOrderSide,
    pub(crate) transact_time: &'a [u8],
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
    pub(crate) orig_cl_ord_id: &'a [u8],
    pub(crate) cl_ord_id: &'a [u8],
    pub(crate) account: Option<&'a [u8]>,
    pub(crate) symbol: &'a [u8],
    pub(crate) side: FixOrderSide,
    pub(crate) transact_time: &'a [u8],
    pub(crate) order_qty: &'a [u8],
    pub(crate) ord_type: FixOrdType,
    pub(crate) price: Option<&'a [u8]>,
    pub(crate) stop_px: Option<&'a [u8]>,
    pub(crate) time_in_force: Option<FixTimeInForce>,
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
    pub(crate) cl_ord_id: &'a [u8],
    pub(crate) order_id: Option<&'a [u8]>,
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
    pub(crate) cl_ord_id: &'a [u8],
    pub(crate) mass_cancel_request_type: FixMassCancelRequestType,
    pub(crate) transact_time: &'a [u8],
    pub(crate) secondary_cl_ord_id: Option<&'a [u8]>,
    pub(crate) trading_session_id: Option<&'a [u8]>,
    pub(crate) trading_session_sub_id: Option<&'a [u8]>,
    pub(crate) symbol: Option<&'a [u8]>,
    pub(crate) side: Option<FixOrderSide>,
    pub(crate) text: Option<&'a [u8]>,
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
    pub(crate) mass_status_req_id: &'a [u8],
    pub(crate) mass_status_req_type: FixMassStatusReqType,
    pub(crate) account: Option<&'a [u8]>,
    pub(crate) acct_id_source: Option<&'a [u8]>,
    pub(crate) trading_session_id: Option<&'a [u8]>,
    pub(crate) trading_session_sub_id: Option<&'a [u8]>,
    pub(crate) symbol: Option<&'a [u8]>,
    pub(crate) side: Option<FixOrderSide>,
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
