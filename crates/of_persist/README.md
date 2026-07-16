# of_persist

`of_persist` provides append-only persistence for normalized orderflow events.
The stable [`RollingStore`] API writes human-readable JSONL for replay,
auditability, and post-trade research workflows. The additive [`MarketDataWal`]
API provides a binary normalized market-data WAL foundation for lower-latency
production capture paths.

## Main Types

- [`RollingStore`] - append-only store for `book` and `trades` streams.
- [`StoredBookEvent`] / [`StoredTradeEvent`] - typed readback records parsed from existing JSONL files.
- [`StoredEvent`] - merged replay-oriented enum for interleaved symbol reads.
- [`RetentionPolicy`] - bounded retention by total bytes and/or max file age.
- [`MarketDataWal`] - single-file binary WAL for normalized market-data frames.
- [`MarketDataWalConfig`] - WAL path and sync policy configuration.
- [`MarketDataWalRecordKind`] - fixed record kind vocabulary.
- [`MarketDataWalRecord`] - decoded WAL replay record.
- [`MarketDataWalSequence`] - monotonic writer sequence.
- [`MarketDataWalReplayResult`] - replay summary.
- [`MarketDataWalIntegrityReport`] - checksum and sequence validation summary.
- [`MarketDataWalMetrics`] - append/sync counters.
- [`MarketDataPersistenceMode`] - production writer mode vocabulary.
- [`MarketDataPersistenceFailureAction`] - host action when persistence degrades.
- [`MarketDataPersistencePolicy`] - configured persistence mode and failure action.
- [`MarketDataPersistenceHealth`] - health, lag, drop, and error snapshot.
- [`MarketDataRecordCriticality`] - relative record importance under pressure.
- [`MarketDataBackpressureDropPolicy`] - bounded writer drop strategy.
- [`MarketDataBackpressureReason`] - active pressure reason.
- [`MarketDataBackpressureAction`] - selected action for one candidate record.
- [`MarketDataBackpressurePolicy`] - queue/lag/byte pressure thresholds.
- [`MarketDataBackpressureDecision`] - evaluated action and flags.
- [`PersistError`] / [`PersistResult<T>`] - persistence error contract.

## New In 0.4.0

`0.4.0` keeps the existing market-data persistence API stable and documents how
persistence participates in the larger strategy lifecycle. `of_persist`
continues to store normalized book/trade event streams; execution command and
report journaling belongs to `of_execution`.

What changes for persistence users:

- market-data replay remains deterministic and sequence-bounded;
- strategy validation examples now pair market-data replay with simulated OMS
  execution so analytics and order decisions can be reviewed together;
- JSONL schema metadata remains backward-compatible for existing persisted
  streams;
- execution journals are intentionally separate from market-data stores so
  audit, retention, and recovery policies can differ;
- binary normalized market-data WAL helpers are available as additive building
  blocks for production capture without replacing JSONL workflows;
- production persistence policy and health helpers let hosts tie writer
  degradation into execution safety policy;
- market-data backpressure helpers make slow-consumer and bounded-writer
  behavior explicit without silently dropping records;
- production deployments should keep market-data replay files and execution
  command/event journals correlated by strategy id, session id, and timestamp.

Version policy:

- `of_persist` publishes as `0.4.0`;
- `of_execution` publishes as `0.1.0` and owns execution journaling helpers.

## Public API Inventory

Public types:

- [`PersistError`]
- [`PersistResult<T>`]
- [`RetentionPolicy`]
- [`RollingStore`]
- [`StoredBookEvent`]
- [`StoredTradeEvent`]
- [`StoredEvent`]
- [`MarketDataWalSequence`]
- [`MarketDataWalRecordKind`]
- [`MarketDataWalConfig`]
- [`MarketDataWalRecord`]
- [`MarketDataWalReplayResult`]
- [`MarketDataWalIntegrityReport`]
- [`MarketDataWalMetrics`]
- [`MarketDataPersistenceMode`]
- [`MarketDataPersistenceFailureAction`]
- [`MarketDataPersistencePolicy`]
- [`MarketDataPersistenceHealth`]
- [`MarketDataRecordCriticality`]
- [`MarketDataBackpressureDropPolicy`]
- [`MarketDataBackpressureReason`]
- [`MarketDataBackpressureAction`]
- [`MarketDataBackpressurePolicy`]
- [`MarketDataBackpressureDecision`]
- [`MarketDataWal`]

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
- [`MarketDataWalConfig::new`]
- [`MarketDataWalConfig::with_sync_on_write`]
- [`MarketDataWalConfig::path`]
- [`MarketDataWalConfig::sync_on_write`]
- [`MarketDataWal::open`]
- [`MarketDataWal::path`]
- [`MarketDataWal::next_sequence`]
- [`MarketDataWal::metrics`]
- [`MarketDataWal::append_record`]
- [`MarketDataWal::replay`]
- [`MarketDataWal::inspect_path`]
- [`MarketDataPersistencePolicy::disabled`]
- [`MarketDataPersistencePolicy::inline_strict`]
- [`MarketDataPersistencePolicy::bounded_async`]
- [`MarketDataPersistencePolicy::with_failure_action`]
- [`MarketDataPersistencePolicy::enabled`]
- [`MarketDataPersistenceHealth::from_wal_metrics`]
- [`MarketDataPersistenceHealth::with_lag`]
- [`MarketDataPersistenceHealth::with_dropped_records`]
- [`MarketDataPersistenceHealth::with_error`]
- [`MarketDataPersistenceHealth::is_healthy`]
- [`MarketDataBackpressurePolicy::reject_new`]
- [`MarketDataBackpressurePolicy::with_max_records_lag`]
- [`MarketDataBackpressurePolicy::with_max_lag_ns`]
- [`MarketDataBackpressurePolicy::with_max_bytes_pending`]
- [`MarketDataBackpressurePolicy::with_drop_policy`]
- [`MarketDataBackpressurePolicy::with_protected_criticality`]
- [`MarketDataBackpressurePolicy::with_failure_action`]
- [`MarketDataBackpressureDecision::is_stop`]
- [`evaluate_market_data_backpressure`]

## Storage Layout

Events are written to:

`<root>/<venue>/<symbol>/(book|trades).jsonl`

This makes stream files easy to map into replay and analytics pipelines.

## Record Schema Reference

Persisted JSONL records are additive and versioned with `"schema": 1`.
Newly-written records include event timestamps (`ts_exchange_ns`,
`ts_recv_ns`) alongside sequence and payload fields. The typed readback API
continues to accept legacy records that do not contain schema or timestamp
fields.

[`StoredBookEvent`] contains:

- `side`, `level`, `price`, `size`, `action`
- `sequence`

[`StoredTradeEvent`] contains:

- `price`, `size`, `aggressor_side`
- `sequence`

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

## MarketDataWal Contract

[`MarketDataWal`] is an additive binary WAL foundation for normalized
market-data capture. It does not replace [`RollingStore`]; JSONL remains the
compatibility, dashboard, and research-friendly format.

The WAL uses fixed-size headers with:

- magic and version,
- record kind,
- WAL sequence,
- provider and normalized event sequences,
- exchange and receive timestamps,
- payload length,
- checksum, and
- previous-record checksum link.

[`MarketDataWal::open`] validates existing bytes before appending. Corrupt
records, broken checksum links, invalid record kinds, and truncated tails fail
closed through [`PersistError::Io`] with `InvalidData`. [`MarketDataWal::replay`]
materializes decoded [`MarketDataWalRecord`] values, and
[`MarketDataWal::inspect_path`] validates a file without returning payloads.

Sync policy is intentionally small in this first foundation:

- `sync_on_write = false`: append to the OS page cache;
- `sync_on_write = true`: call `sync_data` after each append.

Segment rotation, async writer queues, raw provider capture, and cold-store
export remain higher-level integration work.

## Production Persistence Policy And Health

[`MarketDataPersistencePolicy`] gives hosts an explicit vocabulary for
production market-data persistence modes:

- [`MarketDataPersistenceMode::Disabled`]
- [`MarketDataPersistenceMode::InlineStrict`]
- [`MarketDataPersistenceMode::BoundedAsync`]
- [`MarketDataPersistenceMode::BestEffort`]

[`MarketDataPersistenceFailureAction`] records what the host should do when the
path degrades: mark degraded, stop market data, stop trading, fail the process,
or switch to memory-only retention. The policy does not start a worker or change
[`MarketDataWal`] behavior; it is stable configuration metadata for runtimes,
dashboards, and safety-policy integration.

[`MarketDataPersistenceHealth`] reports whether persistence is enabled,
degraded, lagging, dropping records, or surfacing write/sync errors. Hosts can
derive it from [`MarketDataWalMetrics`] with
[`MarketDataPersistenceHealth::from_wal_metrics`] and then attach queue-depth,
lag, pending bytes, drops, or last-error metadata from their writer.

## Backpressure Policy

[`MarketDataBackpressurePolicy`] evaluates writer queue depth, record lag,
nanosecond lag, pending bytes, and degraded persistence state for one candidate
record. It returns a [`MarketDataBackpressureDecision`] with a single action:
accept, reject, drop the current record, ask the host queue to drop an older or
lower-priority record, stop market data, stop trading, fail the process, or
switch to memory-only retention.

The helper is deterministic and allocation-free. Hosts still own the actual
queue, circuit breaker, adapter disconnect, and alerting behavior.

Useful policies:

- [`MarketDataBackpressureDropPolicy::RejectNew`] for strict capture paths,
- [`MarketDataBackpressureDropPolicy::DropNewest`] for best-effort diagnostics,
- [`MarketDataBackpressureDropPolicy::DropOldest`] for bounded rolling queues,
- [`MarketDataBackpressureDropPolicy::DropLowestPriority`] for priority queues,
- [`MarketDataBackpressureDropPolicy::PreserveTrades`] when trades should be
  retained before ordinary depth updates.

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

## Binary WAL Example

```rust,no_run
use of_persist::{MarketDataWal, MarketDataWalConfig, MarketDataWalRecordKind};

let path = "data/CME/ESM6/normalized.wal";
let mut wal = MarketDataWal::open(
    MarketDataWalConfig::new(path).with_sync_on_write(false),
)?;

let sequence = wal.append_record(
    MarketDataWalRecordKind::TradePrint,
    42,
    1001,
    1_700_000_000,
    1_700_000_050,
    b"encoded-normalized-trade",
)?;

let mut records = Vec::new();
let replay = wal.replay(&mut records)?;
println!("sequence={sequence:?} replayed={}", replay.records);
# Ok::<(), of_persist::PersistError>(())
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
