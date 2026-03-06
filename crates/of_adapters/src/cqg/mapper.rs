use of_core::{BookAction, BookUpdate, Side, SymbolId, TradePrint};

use super::proto::CqgInbound;
use crate::RawEvent;

pub fn map_inbound_to_raw(symbol: &SymbolId, msg: &CqgInbound) -> Option<RawEvent> {
    match msg {
        CqgInbound::MarketDataIncremental {
            sequence,
            price,
            size,
            level,
            is_bid,
            is_delete,
            ..
        } => Some(RawEvent::Book(BookUpdate {
            symbol: symbol.clone(),
            side: if *is_bid { Side::Bid } else { Side::Ask },
            level: *level,
            price: *price,
            size: *size,
            action: if *is_delete {
                BookAction::Delete
            } else {
                BookAction::Upsert
            },
            sequence: *sequence,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        })),
        CqgInbound::TradeUpdate {
            sequence,
            price,
            size,
            aggressor_is_buy,
            ..
        } => Some(RawEvent::Trade(TradePrint {
            symbol: symbol.clone(),
            price: *price,
            size: *size,
            aggressor_side: if *aggressor_is_buy {
                Side::Ask
            } else {
                Side::Bid
            },
            sequence: *sequence,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        })),
        _ => None,
    }
}
