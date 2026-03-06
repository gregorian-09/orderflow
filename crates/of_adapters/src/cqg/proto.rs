#[derive(Debug, Clone)]
pub enum CqgOutbound {
    Logon,
    InformationRequest { request_id: u64, symbol: String },
    MarketDataSubscription {
        request_id: u64,
        contract_id: i64,
        level: u16,
    },
    Ping,
    Logoff,
}

#[derive(Debug, Clone)]
pub enum CqgInbound {
    LogonResult { success: bool, message: String },
    SymbolResolution {
        request_id: u64,
        contract_id: i64,
        symbol: String,
    },
    MarketDataIncremental {
        contract_id: i64,
        sequence: u64,
        price: i64,
        size: i64,
        level: u16,
        is_bid: bool,
        is_delete: bool,
    },
    TradeUpdate {
        contract_id: i64,
        sequence: u64,
        price: i64,
        size: i64,
        aggressor_is_buy: bool,
    },
    SubscriptionAck {
        request_id: u64,
        contract_id: i64,
        accepted: bool,
    },
    Reject { request_id: u64, reason: String },
    Heartbeat,
}

#[cfg(feature = "cqg_proto")]
mod protobuf_codec {
    use super::{CqgInbound, CqgOutbound};

    pub(super) const PB_SCHEMA_VERSION: u32 = 1;

    const MSG_OUT_LOGON: u64 = 1;
    const MSG_OUT_INFO: u64 = 2;
    const MSG_OUT_SUB: u64 = 3;
    const MSG_OUT_PING: u64 = 4;
    const MSG_OUT_LOGOFF: u64 = 5;

    const MSG_IN_LOGON_RESULT: u64 = 100;
    const MSG_IN_RESOLVE: u64 = 101;
    const MSG_IN_BOOK: u64 = 102;
    const MSG_IN_TRADE: u64 = 103;
    const MSG_IN_REJECT: u64 = 104;
    const MSG_IN_HEARTBEAT: u64 = 105;
    const MSG_IN_SUB_ACK: u64 = 106;

    pub fn encode_outbound(msg: &CqgOutbound) -> Vec<u8> {
        match msg {
            CqgOutbound::Logon => wrap(MSG_OUT_LOGON, Vec::new()),
            CqgOutbound::InformationRequest { request_id, symbol } => {
                let mut payload = Vec::new();
                encode_varint_field(&mut payload, 1, *request_id);
                encode_bytes_field(&mut payload, 2, symbol.as_bytes());
                wrap(MSG_OUT_INFO, payload)
            }
            CqgOutbound::MarketDataSubscription {
                request_id,
                contract_id,
                level,
            } => {
                let mut payload = Vec::new();
                encode_varint_field(&mut payload, 1, *request_id);
                encode_sint64_field(&mut payload, 2, *contract_id);
                encode_varint_field(&mut payload, 3, *level as u64);
                wrap(MSG_OUT_SUB, payload)
            }
            CqgOutbound::Ping => wrap(MSG_OUT_PING, Vec::new()),
            CqgOutbound::Logoff => wrap(MSG_OUT_LOGOFF, Vec::new()),
        }
    }

    pub fn encode_inbound_for_test(msg: &CqgInbound) -> Vec<u8> {
        match msg {
            CqgInbound::LogonResult { success, message } => {
                let mut payload = Vec::new();
                encode_bool_field(&mut payload, 1, *success);
                encode_bytes_field(&mut payload, 2, message.as_bytes());
                wrap(MSG_IN_LOGON_RESULT, payload)
            }
            CqgInbound::SymbolResolution {
                request_id,
                contract_id,
                symbol,
            } => {
                let mut payload = Vec::new();
                encode_varint_field(&mut payload, 1, *request_id);
                encode_sint64_field(&mut payload, 2, *contract_id);
                encode_bytes_field(&mut payload, 3, symbol.as_bytes());
                wrap(MSG_IN_RESOLVE, payload)
            }
            CqgInbound::MarketDataIncremental {
                contract_id,
                sequence,
                price,
                size,
                level,
                is_bid,
                is_delete,
            } => {
                let mut payload = Vec::new();
                encode_sint64_field(&mut payload, 1, *contract_id);
                encode_varint_field(&mut payload, 2, *sequence);
                encode_sint64_field(&mut payload, 3, *price);
                encode_sint64_field(&mut payload, 4, *size);
                encode_varint_field(&mut payload, 5, *level as u64);
                encode_bool_field(&mut payload, 6, *is_bid);
                encode_bool_field(&mut payload, 7, *is_delete);
                wrap(MSG_IN_BOOK, payload)
            }
            CqgInbound::TradeUpdate {
                contract_id,
                sequence,
                price,
                size,
                aggressor_is_buy,
            } => {
                let mut payload = Vec::new();
                encode_sint64_field(&mut payload, 1, *contract_id);
                encode_varint_field(&mut payload, 2, *sequence);
                encode_sint64_field(&mut payload, 3, *price);
                encode_sint64_field(&mut payload, 4, *size);
                encode_bool_field(&mut payload, 5, *aggressor_is_buy);
                wrap(MSG_IN_TRADE, payload)
            }
            CqgInbound::SubscriptionAck {
                request_id,
                contract_id,
                accepted,
            } => {
                let mut payload = Vec::new();
                encode_varint_field(&mut payload, 1, *request_id);
                encode_sint64_field(&mut payload, 2, *contract_id);
                encode_bool_field(&mut payload, 3, *accepted);
                wrap(MSG_IN_SUB_ACK, payload)
            }
            CqgInbound::Reject { request_id, reason } => {
                let mut payload = Vec::new();
                encode_varint_field(&mut payload, 1, *request_id);
                encode_bytes_field(&mut payload, 2, reason.as_bytes());
                wrap(MSG_IN_REJECT, payload)
            }
            CqgInbound::Heartbeat => wrap(MSG_IN_HEARTBEAT, Vec::new()),
        }
    }

    pub fn decode_inbound(frame: &[u8]) -> Result<CqgInbound, String> {
        let (msg_type, payload) = decode_envelope(frame)?;
        match msg_type {
            MSG_IN_LOGON_RESULT => {
                let success = decode_bool_field(payload, 1)?.unwrap_or(false);
                let message = decode_bytes_field(payload, 2)?
                    .map(|v| String::from_utf8_lossy(&v).to_string())
                    .unwrap_or_default();
                Ok(CqgInbound::LogonResult { success, message })
            }
            MSG_IN_RESOLVE => {
                let request_id =
                    decode_varint_field(payload, 1)?.ok_or("missing request_id".to_string())?;
                let contract_id =
                    decode_sint64_field(payload, 2)?.ok_or("missing contract_id".to_string())?;
                let symbol = decode_bytes_field(payload, 3)?
                    .map(|v| String::from_utf8_lossy(&v).to_string())
                    .ok_or("missing symbol".to_string())?;
                Ok(CqgInbound::SymbolResolution {
                    request_id,
                    contract_id,
                    symbol,
                })
            }
            MSG_IN_BOOK => {
                let contract_id =
                    decode_sint64_field(payload, 1)?.ok_or("missing contract_id".to_string())?;
                let sequence =
                    decode_varint_field(payload, 2)?.ok_or("missing sequence".to_string())?;
                let price =
                    decode_sint64_field(payload, 3)?.ok_or("missing price".to_string())?;
                let size = decode_sint64_field(payload, 4)?.ok_or("missing size".to_string())?;
                let level = decode_varint_field(payload, 5)?.ok_or("missing level".to_string())?;
                let is_bid = decode_bool_field(payload, 6)?.unwrap_or(false);
                let is_delete = decode_bool_field(payload, 7)?.unwrap_or(false);
                Ok(CqgInbound::MarketDataIncremental {
                    contract_id,
                    sequence,
                    price,
                    size,
                    level: level as u16,
                    is_bid,
                    is_delete,
                })
            }
            MSG_IN_TRADE => {
                let contract_id =
                    decode_sint64_field(payload, 1)?.ok_or("missing contract_id".to_string())?;
                let sequence =
                    decode_varint_field(payload, 2)?.ok_or("missing sequence".to_string())?;
                let price =
                    decode_sint64_field(payload, 3)?.ok_or("missing price".to_string())?;
                let size = decode_sint64_field(payload, 4)?.ok_or("missing size".to_string())?;
                let aggressor_is_buy = decode_bool_field(payload, 5)?.unwrap_or(false);
                Ok(CqgInbound::TradeUpdate {
                    contract_id,
                    sequence,
                    price,
                    size,
                    aggressor_is_buy,
                })
            }
            MSG_IN_SUB_ACK => {
                let request_id =
                    decode_varint_field(payload, 1)?.ok_or("missing request_id".to_string())?;
                let contract_id =
                    decode_sint64_field(payload, 2)?.ok_or("missing contract_id".to_string())?;
                let accepted = decode_bool_field(payload, 3)?.unwrap_or(false);
                Ok(CqgInbound::SubscriptionAck {
                    request_id,
                    contract_id,
                    accepted,
                })
            }
            MSG_IN_REJECT => {
                let request_id =
                    decode_varint_field(payload, 1)?.ok_or("missing request_id".to_string())?;
                let reason = decode_bytes_field(payload, 2)?
                    .map(|v| String::from_utf8_lossy(&v).to_string())
                    .unwrap_or_else(|| "reject".to_string());
                Ok(CqgInbound::Reject { request_id, reason })
            }
            MSG_IN_HEARTBEAT => Ok(CqgInbound::Heartbeat),
            _ => Err("unknown inbound type".to_string()),
        }
    }

    pub fn is_ping_outbound_frame(frame: &[u8]) -> bool {
        decode_envelope(frame)
            .map(|(msg_type, _)| msg_type == MSG_OUT_PING)
            .unwrap_or(false)
    }

    fn wrap(msg_type: u64, payload: Vec<u8>) -> Vec<u8> {
        let mut out = Vec::new();
        encode_key(&mut out, 1, 0);
        encode_varint(&mut out, msg_type);
        encode_key(&mut out, 2, 2);
        encode_varint(&mut out, payload.len() as u64);
        out.extend(payload);
        out
    }

    fn decode_envelope(input: &[u8]) -> Result<(u64, &[u8]), String> {
        let mut i = 0usize;
        let mut msg_type = None;
        let mut payload = None;

        while i < input.len() {
            let key = decode_varint(input, &mut i)?;
            let field = key >> 3;
            let wire = key & 0x07;
            match (field, wire) {
                (1, 0) => msg_type = Some(decode_varint(input, &mut i)?),
                (2, 2) => {
                    let len = decode_varint(input, &mut i)? as usize;
                    if i + len > input.len() {
                        return Err("truncated envelope payload".to_string());
                    }
                    payload = Some(&input[i..i + len]);
                    i += len;
                }
                _ => skip_field(input, &mut i, wire)?,
            }
        }

        Ok((
            msg_type.ok_or("missing envelope msg_type".to_string())?,
            payload.ok_or("missing envelope payload".to_string())?,
        ))
    }

    fn encode_varint_field(out: &mut Vec<u8>, field: u32, value: u64) {
        encode_key(out, field, 0);
        encode_varint(out, value);
    }

    fn encode_bool_field(out: &mut Vec<u8>, field: u32, value: bool) {
        encode_varint_field(out, field, if value { 1 } else { 0 });
    }

    fn encode_sint64_field(out: &mut Vec<u8>, field: u32, value: i64) {
        encode_key(out, field, 0);
        encode_varint(out, zigzag_encode(value));
    }

    fn encode_bytes_field(out: &mut Vec<u8>, field: u32, value: &[u8]) {
        encode_key(out, field, 2);
        encode_varint(out, value.len() as u64);
        out.extend_from_slice(value);
    }

    fn decode_varint_field(input: &[u8], target_field: u32) -> Result<Option<u64>, String> {
        let mut i = 0usize;
        let mut out = None;
        while i < input.len() {
            let key = decode_varint(input, &mut i)?;
            let field = (key >> 3) as u32;
            let wire = key & 0x07;
            if field == target_field && wire == 0 {
                out = Some(decode_varint(input, &mut i)?);
            } else {
                skip_field(input, &mut i, wire)?;
            }
        }
        Ok(out)
    }

    fn decode_bool_field(input: &[u8], target_field: u32) -> Result<Option<bool>, String> {
        Ok(decode_varint_field(input, target_field)?.map(|v| v != 0))
    }

    fn decode_sint64_field(input: &[u8], target_field: u32) -> Result<Option<i64>, String> {
        Ok(decode_varint_field(input, target_field)?.map(zigzag_decode))
    }

    fn decode_bytes_field(input: &[u8], target_field: u32) -> Result<Option<Vec<u8>>, String> {
        let mut i = 0usize;
        let mut out = None;
        while i < input.len() {
            let key = decode_varint(input, &mut i)?;
            let field = (key >> 3) as u32;
            let wire = key & 0x07;
            if field == target_field && wire == 2 {
                let len = decode_varint(input, &mut i)? as usize;
                if i + len > input.len() {
                    return Err("truncated bytes field".to_string());
                }
                out = Some(input[i..i + len].to_vec());
                i += len;
            } else {
                skip_field(input, &mut i, wire)?;
            }
        }
        Ok(out)
    }

    fn skip_field(input: &[u8], index: &mut usize, wire: u64) -> Result<(), String> {
        match wire {
            0 => {
                let _ = decode_varint(input, index)?;
                Ok(())
            }
            1 => {
                if *index + 8 > input.len() {
                    return Err("truncated fixed64".to_string());
                }
                *index += 8;
                Ok(())
            }
            2 => {
                let len = decode_varint(input, index)? as usize;
                if *index + len > input.len() {
                    return Err("truncated length-delimited field".to_string());
                }
                *index += len;
                Ok(())
            }
            5 => {
                if *index + 4 > input.len() {
                    return Err("truncated fixed32".to_string());
                }
                *index += 4;
                Ok(())
            }
            _ => Err("unsupported protobuf wire type".to_string()),
        }
    }

    fn encode_key(out: &mut Vec<u8>, field: u32, wire: u32) {
        encode_varint(out, ((field as u64) << 3) | wire as u64);
    }

    fn encode_varint(out: &mut Vec<u8>, mut v: u64) {
        loop {
            let mut b = (v & 0x7f) as u8;
            v >>= 7;
            if v != 0 {
                b |= 0x80;
            }
            out.push(b);
            if v == 0 {
                break;
            }
        }
    }

    fn decode_varint(input: &[u8], index: &mut usize) -> Result<u64, String> {
        let mut shift = 0u32;
        let mut out = 0u64;
        for _ in 0..10 {
            if *index >= input.len() {
                return Err("truncated varint".to_string());
            }
            let b = input[*index];
            *index += 1;
            out |= ((b & 0x7f) as u64) << shift;
            if (b & 0x80) == 0 {
                return Ok(out);
            }
            shift += 7;
        }
        Err("varint too long".to_string())
    }

    fn zigzag_encode(v: i64) -> u64 {
        ((v << 1) ^ (v >> 63)) as u64
    }

    fn zigzag_decode(v: u64) -> i64 {
        ((v >> 1) as i64) ^ (-((v & 1) as i64))
    }
}

#[cfg(feature = "cqg_proto")]
pub fn encode_outbound(msg: &CqgOutbound) -> Vec<u8> {
    protobuf_codec::encode_outbound(msg)
}

#[cfg(feature = "cqg_proto")]
pub fn pb_schema_version() -> u32 {
    protobuf_codec::PB_SCHEMA_VERSION
}

#[cfg(not(feature = "cqg_proto"))]
pub fn pb_schema_version() -> u32 {
    0
}

#[cfg(not(feature = "cqg_proto"))]
pub fn encode_outbound(msg: &CqgOutbound) -> Vec<u8> {
    let wire = match msg {
        CqgOutbound::Logon => "OUT|LOGON".to_string(),
        CqgOutbound::InformationRequest { request_id, symbol } => {
            format!("OUT|INFO|{}|{}", request_id, symbol)
        }
        CqgOutbound::MarketDataSubscription {
            request_id,
            contract_id,
            level,
        } => format!("OUT|SUB|{}|{}|{}", request_id, contract_id, level),
        CqgOutbound::Ping => "OUT|PING".to_string(),
        CqgOutbound::Logoff => "OUT|LOGOFF".to_string(),
    };
    wire.into_bytes()
}

#[cfg(feature = "cqg_proto")]
pub fn encode_inbound_for_test(msg: &CqgInbound) -> Vec<u8> {
    protobuf_codec::encode_inbound_for_test(msg)
}

#[cfg(not(feature = "cqg_proto"))]
pub fn encode_inbound_for_test(msg: &CqgInbound) -> Vec<u8> {
    let wire = match msg {
        CqgInbound::LogonResult { success, message } => {
            format!("IN|LOGON|{}|{}", if *success { 1 } else { 0 }, message)
        }
        CqgInbound::SymbolResolution {
            request_id,
            contract_id,
            symbol,
        } => format!("IN|RESOLVE|{}|{}|{}", request_id, contract_id, symbol),
        CqgInbound::MarketDataIncremental {
            contract_id,
            sequence,
            price,
            size,
            level,
            is_bid,
            is_delete,
        } => format!(
            "IN|BOOK|{}|{}|{}|{}|{}|{}|{}",
            contract_id,
            sequence,
            price,
            size,
            level,
            if *is_bid { 1 } else { 0 },
            if *is_delete { 1 } else { 0 }
        ),
        CqgInbound::TradeUpdate {
            contract_id,
            sequence,
            price,
            size,
            aggressor_is_buy,
        } => format!(
            "IN|TRADE|{}|{}|{}|{}|{}",
            contract_id,
            sequence,
            price,
            size,
            if *aggressor_is_buy { 1 } else { 0 }
        ),
        CqgInbound::SubscriptionAck {
            request_id,
            contract_id,
            accepted,
        } => format!(
            "IN|SUBACK|{}|{}|{}",
            request_id,
            contract_id,
            if *accepted { 1 } else { 0 }
        ),
        CqgInbound::Reject { request_id, reason } => {
            format!("IN|REJECT|{}|{}", request_id, reason)
        }
        CqgInbound::Heartbeat => "IN|HEARTBEAT".to_string(),
    };
    wire.into_bytes()
}

#[cfg(feature = "cqg_proto")]
pub fn decode_inbound(frame: &[u8]) -> Result<CqgInbound, String> {
    protobuf_codec::decode_inbound(frame)
}

#[cfg(not(feature = "cqg_proto"))]
pub fn decode_inbound(frame: &[u8]) -> Result<CqgInbound, String> {
    let text = std::str::from_utf8(frame).map_err(|_| "invalid utf8 frame".to_string())?;
    let parts: Vec<&str> = text.split('|').collect();
    if parts.len() < 2 || parts[0] != "IN" {
        return Err("invalid frame prefix".to_string());
    }

    match parts[1] {
        "LOGON" => {
            if parts.len() < 4 {
                return Err("invalid logon frame".to_string());
            }
            Ok(CqgInbound::LogonResult {
                success: parts[2] == "1",
                message: parts[3].to_string(),
            })
        }
        "RESOLVE" => {
            if parts.len() < 5 {
                return Err("invalid resolve frame".to_string());
            }
            Ok(CqgInbound::SymbolResolution {
                request_id: parts[2].parse().map_err(|_| "bad request_id")?,
                contract_id: parts[3].parse().map_err(|_| "bad contract_id")?,
                symbol: parts[4].to_string(),
            })
        }
        "BOOK" => {
            if parts.len() < 9 {
                return Err("invalid book frame".to_string());
            }
            Ok(CqgInbound::MarketDataIncremental {
                contract_id: parts[2].parse().map_err(|_| "bad contract_id")?,
                sequence: parts[3].parse().map_err(|_| "bad sequence")?,
                price: parts[4].parse().map_err(|_| "bad price")?,
                size: parts[5].parse().map_err(|_| "bad size")?,
                level: parts[6].parse().map_err(|_| "bad level")?,
                is_bid: parts[7] == "1",
                is_delete: parts[8] == "1",
            })
        }
        "TRADE" => {
            if parts.len() < 7 {
                return Err("invalid trade frame".to_string());
            }
            Ok(CqgInbound::TradeUpdate {
                contract_id: parts[2].parse().map_err(|_| "bad contract_id")?,
                sequence: parts[3].parse().map_err(|_| "bad sequence")?,
                price: parts[4].parse().map_err(|_| "bad price")?,
                size: parts[5].parse().map_err(|_| "bad size")?,
                aggressor_is_buy: parts[6] == "1",
            })
        }
        "SUBACK" => {
            if parts.len() < 5 {
                return Err("invalid suback frame".to_string());
            }
            Ok(CqgInbound::SubscriptionAck {
                request_id: parts[2].parse().map_err(|_| "bad request_id")?,
                contract_id: parts[3].parse().map_err(|_| "bad contract_id")?,
                accepted: parts[4] == "1",
            })
        }
        "REJECT" => {
            if parts.len() < 4 {
                return Err("invalid reject frame".to_string());
            }
            Ok(CqgInbound::Reject {
                request_id: parts[2].parse().map_err(|_| "bad request_id")?,
                reason: parts[3].to_string(),
            })
        }
        "HEARTBEAT" => Ok(CqgInbound::Heartbeat),
        _ => Err("unknown inbound type".to_string()),
    }
}

#[cfg(feature = "cqg_proto")]
pub fn is_ping_outbound_frame(frame: &[u8]) -> bool {
    protobuf_codec::is_ping_outbound_frame(frame)
}

#[cfg(not(feature = "cqg_proto"))]
pub fn is_ping_outbound_frame(frame: &[u8]) -> bool {
    frame.starts_with(b"OUT|PING")
}

#[cfg(feature = "cqg_proto")]
pub fn wire_mode() -> &'static str {
    "protobuf"
}

#[cfg(not(feature = "cqg_proto"))]
pub fn wire_mode() -> &'static str {
    "text"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_detection_matches_encoded_ping() {
        let frame = encode_outbound(&CqgOutbound::Ping);
        assert!(is_ping_outbound_frame(&frame));
    }

    #[test]
    fn inbound_roundtrip_suback() {
        let raw = encode_inbound_for_test(&CqgInbound::SubscriptionAck {
            request_id: 7,
            contract_id: 1007,
            accepted: true,
        });
        match decode_inbound(&raw).expect("decode") {
            CqgInbound::SubscriptionAck {
                request_id,
                contract_id,
                accepted,
            } => {
                assert_eq!(request_id, 7);
                assert_eq!(contract_id, 1007);
                assert!(accepted);
            }
            _ => panic!("unexpected decoded variant"),
        }
    }
}
