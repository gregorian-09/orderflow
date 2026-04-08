# of_persist

`of_persist` provides append-only JSONL persistence for normalized orderflow events, with optional retention pruning.
It is designed for replay, auditability, and post-trade research workflows.

## Main Types

- [`RollingStore`] - append-only store for `book` and `trades` streams.
- [`StoredBookEvent`] / [`StoredTradeEvent`] - typed readback records parsed from existing JSONL files.
- [`RetentionPolicy`] - bounded retention by total bytes and/or max file age.
- [`PersistError`] / [`PersistResult<T>`] - persistence error contract.

## Storage Layout

Events are written to:

`<root>/<venue>/<symbol>/(book|trades).jsonl`

This makes stream files easy to map into replay and analytics pipelines.

## Readback API

`RollingStore` now supports additive typed readback over the same files it already writes:

- `read_books(venue, symbol)` reads `book.jsonl` into [`StoredBookEvent`] values
- `read_trades(venue, symbol)` reads `trades.jsonl` into [`StoredTradeEvent`] values
- missing streams return an empty vector
- malformed lines return `PersistError::Io` with `InvalidData`

## Quick Example

```rust
use of_core::{Side, SymbolId, TradePrint};
use of_persist::RollingStore;

let store = RollingStore::new("data").expect("store");

store.append_trade(&TradePrint {
    symbol: SymbolId {
        venue: "CME".to_string(),
        symbol: "ESM6".to_string(),
    },
    price: 505_000,
    size: 2,
    aggressor_side: Side::Ask,
    sequence: 1,
    ts_exchange_ns: 1,
    ts_recv_ns: 2,
}).expect("append");
```

## Readback Example

```rust
use of_persist::RollingStore;

let store = RollingStore::new("data").expect("store");
let trades = store.read_trades("CME", "ESM6").expect("read trades");

for trade in trades {
    println!("seq={} price={} size={}", trade.sequence, trade.price, trade.size);
}
```

## Retention Example

```rust,no_run
use of_persist::{RetentionPolicy, RollingStore};

let store = RollingStore::new("data")?
    .with_retention(Some(RetentionPolicy {
        max_total_bytes: 2 * 1024 * 1024 * 1024,
        max_age_secs: 7 * 24 * 60 * 60,
    }));

let _ = store;
# Ok::<(), of_persist::PersistError>(())
```

## Retention Behavior

- `max_age_secs > 0`: files older than threshold are pruned.
- `max_total_bytes > 0`: oldest files are pruned until under limit.
- `0` means that limit is disabled.
