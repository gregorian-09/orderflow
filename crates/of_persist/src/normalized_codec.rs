//! Versioned binary codec for canonical normalized market-data events.

use super::{MarketDataWalRecord, MarketDataWalRecordKind};
use of_core::{BookAction, BookUpdate, Side, SymbolId, TradePrint};
use std::error::Error;
use std::fmt;

const NORMALIZED_EVENT_MAGIC: [u8; 4] = *b"OFNE";
const NORMALIZED_EVENT_VERSION: u16 = 1;
const NORMALIZED_EVENT_HEADER_LEN: usize = 32;
const BOOK_KIND: u8 = 1;
const TRADE_KIND: u8 = 2;

/// Owned canonical event accepted by normalized persistence writers.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NormalizedMarketDataRecordInput {
    /// Canonical level-2 book update.
    Book(BookUpdate),
    /// Canonical trade print.
    Trade(TradePrint),
}

impl NormalizedMarketDataRecordInput {
    /// Creates an input from a canonical book update.
    pub fn book(event: BookUpdate) -> Self {
        Self::Book(event)
    }

    /// Creates an input from a canonical trade print.
    pub fn trade(event: TradePrint) -> Self {
        Self::Trade(event)
    }

    /// Returns the enclosing WAL record kind.
    pub const fn record_kind(&self) -> MarketDataWalRecordKind {
        match self {
            Self::Book(_) => MarketDataWalRecordKind::BookUpdate,
            Self::Trade(_) => MarketDataWalRecordKind::TradePrint,
        }
    }

    /// Returns provider/event sequence carried by the canonical event.
    pub const fn sequence(&self) -> u64 {
        match self {
            Self::Book(event) => event.sequence,
            Self::Trade(event) => event.sequence,
        }
    }

    /// Returns exchange timestamp in nanoseconds.
    pub const fn ts_exchange_ns(&self) -> u64 {
        match self {
            Self::Book(event) => event.ts_exchange_ns,
            Self::Trade(event) => event.ts_exchange_ns,
        }
    }

    /// Returns local receive timestamp in nanoseconds.
    pub const fn ts_recv_ns(&self) -> u64 {
        match self {
            Self::Book(event) => event.ts_recv_ns,
            Self::Trade(event) => event.ts_recv_ns,
        }
    }

    /// Returns exact encoded payload length.
    ///
    /// # Errors
    /// Returns an error when a venue or symbol exceeds the version-1 limit.
    pub fn encoded_len(&self) -> Result<usize, NormalizedMarketDataCodecError> {
        let symbol = self.symbol();
        validate_identifier_lengths(symbol)?;
        Ok(NORMALIZED_EVENT_HEADER_LEN + symbol.venue.len() + symbol.symbol.len())
    }

    /// Encodes into reusable caller-owned storage.
    ///
    /// # Errors
    /// Returns an error when a venue or symbol exceeds the version-1 limit.
    pub fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), NormalizedMarketDataCodecError> {
        let symbol = self.symbol();
        let encoded_len = self.encoded_len()?;
        out.clear();
        out.reserve(encoded_len);
        out.extend_from_slice(&NORMALIZED_EVENT_MAGIC);
        out.extend_from_slice(&NORMALIZED_EVENT_VERSION.to_le_bytes());
        let (kind, side, action, level, price, size) = match self {
            Self::Book(event) => (
                BOOK_KIND,
                encode_side(event.side),
                encode_action(event.action),
                event.level,
                event.price,
                event.size,
            ),
            Self::Trade(event) => (
                TRADE_KIND,
                encode_side(event.aggressor_side),
                0,
                0,
                event.price,
                event.size,
            ),
        };
        out.push(kind);
        out.push(0);
        out.extend_from_slice(&(symbol.venue.len() as u16).to_le_bytes());
        out.extend_from_slice(&(symbol.symbol.len() as u16).to_le_bytes());
        out.push(side);
        out.push(action);
        out.extend_from_slice(&level.to_le_bytes());
        out.extend_from_slice(&price.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(symbol.venue.as_bytes());
        out.extend_from_slice(symbol.symbol.as_bytes());
        debug_assert_eq!(out.len(), encoded_len);
        Ok(())
    }

    fn symbol(&self) -> &SymbolId {
        match self {
            Self::Book(event) => &event.symbol,
            Self::Trade(event) => &event.symbol,
        }
    }
}

/// Fail-closed normalized event codec error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NormalizedMarketDataCodecError {
    /// Venue identifier exceeds the version-1 `u16` byte-length limit.
    VenueTooLong,
    /// Symbol identifier exceeds the version-1 `u16` byte-length limit.
    SymbolTooLong,
    /// Payload is shorter than the fixed version-1 header.
    TruncatedHeader,
    /// Payload magic is not a normalized Orderflow event.
    InvalidMagic,
    /// Payload version is unsupported.
    UnsupportedVersion(u16),
    /// Reserved header byte is nonzero.
    NonzeroReservedByte,
    /// Payload kind disagrees with the enclosing WAL record kind.
    KindMismatch,
    /// Encoded side discriminant is invalid.
    InvalidSide(u8),
    /// Encoded book-action discriminant is invalid.
    InvalidAction(u8),
    /// Encoded payload length disagrees with identifier lengths.
    InvalidLength,
    /// Venue or symbol bytes are not valid UTF-8.
    InvalidUtf8,
}

impl fmt::Display for NormalizedMarketDataCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VenueTooLong => formatter.write_str("normalized venue identifier is too long"),
            Self::SymbolTooLong => formatter.write_str("normalized symbol identifier is too long"),
            Self::TruncatedHeader => formatter.write_str("normalized event header is truncated"),
            Self::InvalidMagic => formatter.write_str("normalized event magic is invalid"),
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "normalized event version {version} is unsupported"
                )
            }
            Self::NonzeroReservedByte => {
                formatter.write_str("normalized event reserved byte is nonzero")
            }
            Self::KindMismatch => {
                formatter.write_str("normalized event kind does not match WAL kind")
            }
            Self::InvalidSide(side) => write!(formatter, "normalized side {side} is invalid"),
            Self::InvalidAction(action) => {
                write!(formatter, "normalized book action {action} is invalid")
            }
            Self::InvalidLength => {
                formatter.write_str("normalized event payload length is invalid")
            }
            Self::InvalidUtf8 => formatter.write_str("normalized event identifier is not UTF-8"),
        }
    }
}

impl Error for NormalizedMarketDataCodecError {}

/// Decodes one canonical event from a validated WAL frame.
///
/// # Errors
/// Returns an error for malformed payloads or kind mismatches.
pub fn decode_normalized_market_data_record(
    record: &MarketDataWalRecord,
) -> Result<NormalizedMarketDataRecordInput, NormalizedMarketDataCodecError> {
    let payload = &record.payload;
    if payload.len() < NORMALIZED_EVENT_HEADER_LEN {
        return Err(NormalizedMarketDataCodecError::TruncatedHeader);
    }
    if payload[..4] != NORMALIZED_EVENT_MAGIC {
        return Err(NormalizedMarketDataCodecError::InvalidMagic);
    }
    let version = read_u16(&payload[4..6]);
    if version != NORMALIZED_EVENT_VERSION {
        return Err(NormalizedMarketDataCodecError::UnsupportedVersion(version));
    }
    if payload[7] != 0 {
        return Err(NormalizedMarketDataCodecError::NonzeroReservedByte);
    }
    let venue_len = read_u16(&payload[8..10]) as usize;
    let symbol_len = read_u16(&payload[10..12]) as usize;
    let expected_len = NORMALIZED_EVENT_HEADER_LEN
        .checked_add(venue_len)
        .and_then(|length| length.checked_add(symbol_len))
        .ok_or(NormalizedMarketDataCodecError::InvalidLength)?;
    if payload.len() != expected_len {
        return Err(NormalizedMarketDataCodecError::InvalidLength);
    }
    let venue_end = NORMALIZED_EVENT_HEADER_LEN + venue_len;
    let venue = std::str::from_utf8(&payload[NORMALIZED_EVENT_HEADER_LEN..venue_end])
        .map_err(|_| NormalizedMarketDataCodecError::InvalidUtf8)?;
    let symbol = std::str::from_utf8(&payload[venue_end..])
        .map_err(|_| NormalizedMarketDataCodecError::InvalidUtf8)?;
    let symbol = SymbolId {
        venue: venue.to_owned(),
        symbol: symbol.to_owned(),
    };
    let side = decode_side(payload[12])?;
    let action = payload[13];
    let level = read_u16(&payload[14..16]);
    let price = read_i64(&payload[16..24]);
    let size = read_i64(&payload[24..32]);

    match (record.kind, payload[6]) {
        (MarketDataWalRecordKind::BookUpdate, BOOK_KIND) => {
            let action = decode_action(action)?;
            Ok(NormalizedMarketDataRecordInput::Book(BookUpdate {
                symbol,
                side,
                level,
                price,
                size,
                action,
                sequence: record.event_sequence,
                ts_exchange_ns: record.ts_exchange_ns,
                ts_recv_ns: record.ts_recv_ns,
            }))
        }
        (MarketDataWalRecordKind::TradePrint, TRADE_KIND) if action == 0 && level == 0 => {
            Ok(NormalizedMarketDataRecordInput::Trade(TradePrint {
                symbol,
                price,
                size,
                aggressor_side: side,
                sequence: record.event_sequence,
                ts_exchange_ns: record.ts_exchange_ns,
                ts_recv_ns: record.ts_recv_ns,
            }))
        }
        _ => Err(NormalizedMarketDataCodecError::KindMismatch),
    }
}

fn validate_identifier_lengths(symbol: &SymbolId) -> Result<(), NormalizedMarketDataCodecError> {
    if symbol.venue.len() > u16::MAX as usize {
        return Err(NormalizedMarketDataCodecError::VenueTooLong);
    }
    if symbol.symbol.len() > u16::MAX as usize {
        return Err(NormalizedMarketDataCodecError::SymbolTooLong);
    }
    Ok(())
}

const fn encode_side(side: Side) -> u8 {
    match side {
        Side::Bid => 1,
        Side::Ask => 2,
    }
}

fn decode_side(value: u8) -> Result<Side, NormalizedMarketDataCodecError> {
    match value {
        1 => Ok(Side::Bid),
        2 => Ok(Side::Ask),
        _ => Err(NormalizedMarketDataCodecError::InvalidSide(value)),
    }
}

const fn encode_action(action: BookAction) -> u8 {
    match action {
        BookAction::Upsert => 1,
        BookAction::Delete => 2,
    }
}

fn decode_action(value: u8) -> Result<BookAction, NormalizedMarketDataCodecError> {
    match value {
        1 => Ok(BookAction::Upsert),
        2 => Ok(BookAction::Delete),
        _ => Err(NormalizedMarketDataCodecError::InvalidAction(value)),
    }
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().expect("validated normalized u16 slice"))
}

fn read_i64(bytes: &[u8]) -> i64 {
    i64::from_le_bytes(bytes.try_into().expect("validated normalized i64 slice"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MarketDataWalSequence;

    fn record(kind: MarketDataWalRecordKind, payload: Vec<u8>) -> MarketDataWalRecord {
        MarketDataWalRecord {
            sequence: MarketDataWalSequence(1),
            kind,
            provider_sequence: 99,
            event_sequence: 99,
            ts_exchange_ns: 101,
            ts_recv_ns: 103,
            payload,
        }
    }

    #[test]
    fn book_round_trip_is_exact() {
        let input = NormalizedMarketDataRecordInput::book(BookUpdate {
            symbol: SymbolId {
                venue: "CME".to_owned(),
                symbol: "ESM6".to_owned(),
            },
            side: Side::Bid,
            level: 3,
            price: 5_050_000,
            size: 17,
            action: BookAction::Upsert,
            sequence: 99,
            ts_exchange_ns: 101,
            ts_recv_ns: 103,
        });
        let mut payload = Vec::new();
        input.encode_into(&mut payload).expect("encode book");
        let decoded = decode_normalized_market_data_record(&record(
            MarketDataWalRecordKind::BookUpdate,
            payload,
        ))
        .expect("decode book");
        match decoded {
            NormalizedMarketDataRecordInput::Book(event) => {
                assert_eq!(event.symbol.venue, "CME");
                assert_eq!(event.symbol.symbol, "ESM6");
                assert_eq!(event.side, Side::Bid);
                assert_eq!(event.level, 3);
                assert_eq!(event.price, 5_050_000);
                assert_eq!(event.size, 17);
                assert_eq!(event.action, BookAction::Upsert);
                assert_eq!(event.sequence, 99);
            }
            _ => panic!("expected book"),
        }
    }

    #[test]
    fn trade_round_trip_is_exact() {
        let input = NormalizedMarketDataRecordInput::trade(TradePrint {
            symbol: SymbolId {
                venue: "BINANCE".to_owned(),
                symbol: "BTCUSDT".to_owned(),
            },
            price: 100_000,
            size: 4,
            aggressor_side: Side::Ask,
            sequence: 99,
            ts_exchange_ns: 101,
            ts_recv_ns: 103,
        });
        let mut payload = Vec::new();
        input.encode_into(&mut payload).expect("encode trade");
        let decoded = decode_normalized_market_data_record(&record(
            MarketDataWalRecordKind::TradePrint,
            payload,
        ))
        .expect("decode trade");
        match decoded {
            NormalizedMarketDataRecordInput::Trade(event) => {
                assert_eq!(event.symbol.venue, "BINANCE");
                assert_eq!(event.symbol.symbol, "BTCUSDT");
                assert_eq!(event.aggressor_side, Side::Ask);
                assert_eq!(event.price, 100_000);
                assert_eq!(event.size, 4);
                assert_eq!(event.sequence, 99);
            }
            _ => panic!("expected trade"),
        }
    }

    #[test]
    fn decoder_rejects_corruption_and_kind_mismatch() {
        let input = NormalizedMarketDataRecordInput::trade(TradePrint {
            symbol: SymbolId {
                venue: "X".to_owned(),
                symbol: "Y".to_owned(),
            },
            price: 1,
            size: 1,
            aggressor_side: Side::Bid,
            sequence: 1,
            ts_exchange_ns: 1,
            ts_recv_ns: 1,
        });
        let mut payload = Vec::new();
        input.encode_into(&mut payload).expect("encode trade");
        let error = decode_normalized_market_data_record(&record(
            MarketDataWalRecordKind::BookUpdate,
            payload.clone(),
        ))
        .expect_err("outer kind mismatch");
        assert_eq!(error, NormalizedMarketDataCodecError::KindMismatch);
        payload[7] = 1;
        let error = decode_normalized_market_data_record(&record(
            MarketDataWalRecordKind::TradePrint,
            payload,
        ))
        .expect_err("nonzero reserved byte");
        assert_eq!(error, NormalizedMarketDataCodecError::NonzeroReservedByte);
    }
}
