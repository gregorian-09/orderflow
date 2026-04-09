# of_persist

`of_persist` provides append-only JSONL persistence for normalized orderflow events, with optional retention pruning.
It is designed for replay, auditability, and post-trade research workflows.

## Main Types

- [`RollingStore`] - append-only store for `book` and `trades` streams.
- [`StoredBookEvent`] / [`StoredTradeEvent`] - typed readback records parsed from existing JSONL files.
- [`StoredEvent`] - merged replay-oriented enum for interleaved symbol reads.
- [`RetentionPolicy`] - bounded retention by total bytes and/or max file age.
- [`PersistError`] / [`PersistResult<T>`] - persistence error contract.

## New In 0.2.0

Relative to the `0.1.x` line, `of_persist` is no longer write-only. It now
includes:

- discovery APIs for venues, symbols, and streams
- typed readback APIs for books and trades
- merged event replay reads
- inclusive sequence-range filtering

That makes the crate useful for replay, incident analysis, and research instead
of only append-only storage.

## Public API Inventory

Public types:

- [`PersistError`]
- [`PersistResult<T>`]
- [`RetentionPolicy`]
- [`RollingStore`]
- [`StoredBookEvent`]
- [`StoredTradeEvent`]
- [`StoredEvent`]

Public methods:

- [`StoredEvent::sequence`]
- [`RollingStore::new`]
- [`RollingStore::with_retention`]
- [`RollingStore::append_book`]
- [`RollingStore::append_trade`]
- [`RollingStore::read_books`]
- [`RollingStore::read_books_in_range`]
- [`RollingStore::read_trades`]
- [`RollingStore::read_trades_in_range`]
- [`RollingStore::read_events`]
- [`RollingStore::read_events_in_range`]
- [`RollingStore::list_venues`]
- [`RollingStore::list_symbols`]
- [`RollingStore::list_streams`]

## Storage Layout

Events are written to:

`<root>/<venue>/<symbol>/(book|trades).jsonl`

This makes stream files easy to map into replay and analytics pipelines.

## Record Schema Reference

[`StoredBookEvent`] contains:

- `side`, `level`, `price`, `size`, `action`
- `sequence`
- `ts_exchange_ns`
- `ts_recv_ns`

[`StoredTradeEvent`] contains:

- `price`, `size`, `aggressor_side`
- `sequence`
- `ts_exchange_ns`
- `ts_recv_ns`

[`StoredEvent`] is the merged replay enum:

- `StoredEvent::Book(StoredBookEvent)`
- `StoredEvent::Trade(StoredTradeEvent)`

[`StoredEvent::sequence`] returns the merged event sequence regardless of variant.

## Readback API

`RollingStore` now supports additive typed readback over the same files it already writes:

- `list_venues()` enumerates discovered venue directories
- `list_symbols(venue)` enumerates discovered symbols for one venue
- `list_streams(venue, symbol)` enumerates discovered JSONL streams for one symbol
- `read_books(venue, symbol)` reads `book.jsonl` into [`StoredBookEvent`] values
- `read_books_in_range(venue, symbol, from_sequence, to_sequence)` applies inclusive sequence filtering to book reads
- `read_trades(venue, symbol)` reads `trades.jsonl` into [`StoredTradeEvent`] values
- `read_trades_in_range(venue, symbol, from_sequence, to_sequence)` applies inclusive sequence filtering to trade reads
- `read_events(venue, symbol)` merges both streams into [`StoredEvent`] values ordered by sequence
- `read_events_in_range(venue, symbol, from_sequence, to_sequence)` applies inclusive sequence filtering to merged reads
- missing streams return an empty vector
- malformed lines return `PersistError::Io` with `InvalidData`

## Ordering and Range Semantics

- append methods always write one JSON object per line
- `read_books*` and `read_trades*` preserve file order
- `read_events*` merges book and trade streams by ascending sequence
- `*_in_range` methods use inclusive `from_sequence` / `to_sequence` bounds
- `None` for a bound means it is open-ended on that side
- missing `book.jsonl` or `trades.jsonl` files are treated as empty streams, not hard errors

## RollingStore Contract

- [`RollingStore::new`] creates the persistence root if needed.
- [`RollingStore::with_retention`] returns an updated store handle with retention settings attached.
- [`RollingStore::append_book`] and [`RollingStore::append_trade`] write normalized events, not provider-native payloads.
- Discovery APIs operate on directory/file presence and do not require a separate index.
- Readback APIs parse the same JSONL files the writer produces, so replay stays aligned with persisted runtime output.

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
let venues = store.list_venues().expect("list venues");
let symbols = store.list_symbols("CME").expect("list symbols");
let streams = store.list_streams("CME", "ESM6").expect("list streams");
let trades = store
    .read_trades_in_range("CME", "ESM6", Some(10), Some(100))
    .expect("read trades");

println!("venues={venues:?} symbols={symbols:?} streams={streams:?}");
for trade in trades {
    println!("seq={} price={} size={}", trade.sequence, trade.price, trade.size);
}
```

## Replay Read Example

```rust
use of_persist::{RollingStore, StoredEvent};

let store = RollingStore::new("data").expect("store");
let events = store.read_events("CME", "ESM6").expect("read events");

for event in events {
    match event {
        StoredEvent::Book(book) => println!("book seq={} px={}", book.sequence, book.price),
        StoredEvent::Trade(trade) => println!("trade seq={} px={}", trade.sequence, trade.price),
    }
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

## Error Semantics

- [`PersistError::Io`] wraps filesystem and parse failures.
- directory creation happens eagerly on store creation, so path permission issues surface early.
- retention pruning is best-effort within normal append flows; it is not a separate daemon or background compactor.

## Real-World Use Cases

### 1. Incident review after a bad fill or missed signal

Read back the exact normalized book/trade stream that the runtime saw and
reconstruct the session around the problematic sequence range.

### 2. Research dataset generation

Persist normalized data during live or simulated sessions, then read back only
the venue/symbol windows needed for offline analysis.

### 3. Deterministic replay

Use `read_events(...)` or `read_events_in_range(...)` to feed ordered events
back into test or replay tooling.

## Detailed Example: Investigate A Sequence Window

```rust
use of_persist::{RollingStore, StoredEvent};

fn main() {
    let store = RollingStore::new("data").expect("store");
    let events = store
        .read_events_in_range("CME", "ESM6", Some(10_000), Some(10_150))
        .expect("events");

    for event in events {
        match event {
            StoredEvent::Book(book) => {
                println!(
                    "BOOK seq={} level={} px={} size={}",
                    book.sequence, book.level, book.price, book.size
                );
            }
            StoredEvent::Trade(trade) => {
                println!(
                    "TRADE seq={} px={} size={}",
                    trade.sequence, trade.price, trade.size
                );
            }
        }
    }
}
```

## Detailed Example: Discovery-First Replay Preparation

```rust
use of_persist::RollingStore;

fn main() {
    let store = RollingStore::new("data").expect("store");

    for venue in store.list_venues().expect("venues") {
        println!("venue={venue}");
        for symbol in store.list_symbols(&venue).expect("symbols") {
            let streams = store.list_streams(&venue, &symbol).expect("streams");
            println!("  symbol={symbol} streams={streams:?}");
        }
    }
}
```
