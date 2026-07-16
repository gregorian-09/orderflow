# `of_persist` Reference

`of_persist` provides append-only storage for normalized orderflow data.
`RollingStore` remains the stable JSONL store for auditability, replay, and
post-trade research. `MarketDataWal` adds a binary normalized market-data WAL
foundation for lower-latency production capture paths.

## Public API Map

| Item | Kind | Purpose |
| --- | --- | --- |
| `PersistError` | enum | Persistence error contract |
| `PersistResult<T>` | type alias | `Result<T, PersistError>` |
| `RetentionPolicy` | struct | Retention settings |
| `RollingStore` | struct | Main persistence handle |
| `StoredBookEvent` | struct | Typed book readback record |
| `StoredTradeEvent` | struct | Typed trade readback record |
| `StoredEvent` | enum | Merged replay-oriented event |
| `MarketDataWalSequence` | newtype | Monotonic binary WAL writer sequence |
| `MarketDataWalRecordKind` | enum | Binary WAL record kind vocabulary |
| `MarketDataWalConfig` | struct | Binary WAL path and sync configuration |
| `MarketDataWalRecord` | struct | Decoded binary WAL replay record |
| `MarketDataWalReplayResult` | struct | Binary WAL replay summary |
| `MarketDataWalIntegrityReport` | struct | Checksum/sequence integrity report |
| `MarketDataWalMetrics` | struct | Binary WAL append/sync counters |
| `MarketDataCheckpointId` | newtype | Monotonic checkpoint identifier |
| `MarketDataCheckpointKind` | enum | Opaque checkpoint payload category |
| `MarketDataCheckpointConfig` | struct | Checkpoint root, retention, and sync configuration |
| `MarketDataCheckpoint` | struct | Opaque checkpoint payload with sequence anchors |
| `MarketDataCheckpointManifest` | struct | Checkpoint metadata without payload bytes |
| `MarketDataCheckpointValidation` | struct | Checkpoint integrity report |
| `FileMarketDataCheckpointStore` | struct | File-backed checkpoint store |
| `MarketDataPersistenceMode` | enum | Production writer mode vocabulary |
| `MarketDataPersistenceFailureAction` | enum | Host action when persistence degrades |
| `MarketDataPersistencePolicy` | struct | Production persistence policy |
| `MarketDataPersistenceHealth` | struct | Production persistence health snapshot |
| `MarketDataRecordCriticality` | enum | Relative record importance under pressure |
| `MarketDataBackpressureDropPolicy` | enum | Bounded writer drop strategy |
| `MarketDataBackpressureReason` | enum | Active pressure reason |
| `MarketDataBackpressureAction` | enum | Selected action for one candidate record |
| `MarketDataBackpressurePolicy` | struct | Queue/lag/byte pressure thresholds |
| `MarketDataBackpressureDecision` | struct | Evaluated action and flags |
| `evaluate_market_data_backpressure` | function | Evaluates one candidate persistence record |
| `MarketDataWal` | struct | Single-file binary normalized market-data WAL |

## Storage Layout

Files are stored as:

`<root>/<venue>/<symbol>/book.jsonl`

`<root>/<venue>/<symbol>/trades.jsonl`

Each line is one JSON object representing one normalized event.

Newly-written JSONL records include additive metadata:

- `schema`: record schema version, currently `1`
- `ts_exchange_ns`: exchange timestamp from the normalized event
- `ts_recv_ns`: receive timestamp from the normalized event

Legacy records without these metadata fields remain readable.

## Configuration Type

### `RetentionPolicy`

| Field | Type | Meaning |
| --- | --- | --- |
| `max_total_bytes` | `u64` | Max retained bytes under the persistence root |
| `max_age_secs` | `u64` | Max allowed file age in seconds |

Rules:

- `0` disables that limit.
- If both limits are `0`, retention is effectively disabled.

## Readback Record Types

### `StoredBookEvent`

| Field | Type | Meaning |
| --- | --- | --- |
| `side` | `Side` | Bid or ask |
| `level` | `u16` | Depth index |
| `price` | `i64` | Integer-normalized price |
| `size` | `i64` | Integer-normalized size |
| `action` | `BookAction` | Upsert or delete |
| `sequence` | `u64` | Event sequence |

### `StoredTradeEvent`

| Field | Type | Meaning |
| --- | --- | --- |
| `price` | `i64` | Integer-normalized price |
| `size` | `i64` | Integer-normalized size |
| `aggressor_side` | `Side` | Trade direction |
| `sequence` | `u64` | Event sequence |

### `StoredEvent`

| Variant | Payload | Meaning |
| --- | --- | --- |
| `Book` | `StoredBookEvent` | One stored book mutation |
| `Trade` | `StoredTradeEvent` | One stored trade |

#### Method

| Method | Returns | Meaning |
| --- | --- | --- |
| `sequence()` | `u64` | Sequence number regardless of variant |

## `RollingStore`

### Constructors and configuration

| Method | Returns | Meaning |
| --- | --- | --- |
| `new(root)` | `PersistResult<RollingStore>` | Creates or opens a persistence root |
| `with_retention(retention)` | `RollingStore` | Returns a store handle with retention settings attached |

### Append methods

| Method | Returns | Meaning |
| --- | --- | --- |
| `append_book(&BookUpdate)` | `PersistResult<()>` | Appends one normalized book event |
| `append_trade(&TradePrint)` | `PersistResult<()>` | Appends one normalized trade event |

### Discovery methods

| Method | Returns | Meaning |
| --- | --- | --- |
| `list_venues()` | `PersistResult<Vec<String>>` | Discovers venue directories |
| `list_symbols(venue)` | `PersistResult<Vec<String>>` | Discovers symbols under one venue |
| `list_streams(venue, symbol)` | `PersistResult<Vec<String>>` | Discovers stream files under one symbol |

### Readback methods

| Method | Returns | Meaning |
| --- | --- | --- |
| `read_books(venue, symbol)` | `PersistResult<Vec<StoredBookEvent>>` | Reads all stored book events |
| `read_books_in_range(venue, symbol, from, to)` | `PersistResult<Vec<StoredBookEvent>>` | Reads book events within inclusive sequence bounds |
| `read_trades(venue, symbol)` | `PersistResult<Vec<StoredTradeEvent>>` | Reads all stored trade events |
| `read_trades_in_range(venue, symbol, from, to)` | `PersistResult<Vec<StoredTradeEvent>>` | Reads trade events within inclusive sequence bounds |
| `read_events(venue, symbol)` | `PersistResult<Vec<StoredEvent>>` | Merges stored book and trade events by sequence |
| `read_events_in_range(venue, symbol, from, to)` | `PersistResult<Vec<StoredEvent>>` | Merged read within inclusive sequence bounds |

## `MarketDataWal`

`MarketDataWal` is a binary append-only WAL for normalized market-data frames.
It is additive and does not change `RollingStore` JSONL semantics.

### Configuration

| Method | Returns | Meaning |
| --- | --- | --- |
| `MarketDataWalConfig::new(path)` | `MarketDataWalConfig` | Creates WAL config for a single file path |
| `with_sync_on_write(bool)` | `MarketDataWalConfig` | Enables or disables `sync_data` after each append |
| `path()` | `&Path` | Returns the configured WAL path |
| `sync_on_write()` | `bool` | Returns the sync policy |

### Writer and replay

| Method | Returns | Meaning |
| --- | --- | --- |
| `MarketDataWal::open(config)` | `PersistResult<MarketDataWal>` | Opens a WAL and validates existing bytes before appending |
| `path()` | `&Path` | Returns the WAL path |
| `next_sequence()` | `MarketDataWalSequence` | Returns the next sequence to be assigned |
| `metrics()` | `MarketDataWalMetrics` | Returns append/sync counters |
| `append_record(...)` | `PersistResult<MarketDataWalSequence>` | Appends one binary frame |
| `replay(out)` | `PersistResult<MarketDataWalReplayResult>` | Replays decoded records into `out` |
| `inspect_path(path)` | `PersistResult<MarketDataWalIntegrityReport>` | Validates a WAL file without materializing payloads |

### Frame contract

Each frame carries magic/version, record kind, WAL sequence, provider sequence,
normalized event sequence, exchange timestamp, receive timestamp, payload
length, checksum, and previous-record checksum link. Existing bytes must
validate before append state is initialized.

The first implementation is intentionally single-file. Segment rotation,
bounded async writer queues, raw provider-message capture, and cold research
export remain separate integration layers.

## `FileMarketDataCheckpointStore`

`FileMarketDataCheckpointStore` persists opaque binary checkpoints for
market-data recovery. It is additive and codec-neutral: callers decide how to
serialize order books, analytics accumulators, signal state, sequence caches, or
runtime baselines.

### Configuration

| Method | Returns | Meaning |
| --- | --- | --- |
| `MarketDataCheckpointConfig::new(root)` | `MarketDataCheckpointConfig` | Creates checkpoint config rooted at `root` |
| `with_retain_last(usize)` | `MarketDataCheckpointConfig` | Keeps only the newest N checkpoints per venue/symbol after save; `0` disables pruning |
| `with_sync_on_save(bool)` | `MarketDataCheckpointConfig` | Enables or disables `sync_data` before rename |
| `root()` | `&Path` | Returns the configured checkpoint root |
| `retain_last()` | `usize` | Returns the automatic retention count |
| `sync_on_save()` | `bool` | Returns the sync policy |

### Checkpoint payload

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `MarketDataCheckpointId` | Checkpoint id; zero lets the file store assign the next id |
| `kind` | `MarketDataCheckpointKind` | Payload category |
| `venue` | `String` | Venue path component |
| `symbol` | `String` | Symbol path component |
| `wal_sequence` | `MarketDataWalSequence` | Last applied market-data WAL sequence |
| `provider_sequence` | `u64` | Last provider-native sequence when known |
| `event_sequence` | `u64` | Last normalized event sequence when known |
| `created_ns` | `u64` | Creation timestamp in nanoseconds since Unix epoch |
| `payload_version` | `u32` | Caller-owned payload schema/version tag |
| `payload` | `Vec<u8>` | Opaque encoded checkpoint bytes |

### Store methods

| Method | Returns | Meaning |
| --- | --- | --- |
| `FileMarketDataCheckpointStore::open(config)` | `PersistResult<FileMarketDataCheckpointStore>` | Creates or opens the checkpoint root |
| `config()` | `&MarketDataCheckpointConfig` | Returns the store configuration |
| `save_checkpoint(checkpoint)` | `PersistResult<MarketDataCheckpointManifest>` | Writes a temp file and renames it into place |
| `load_checkpoint(venue, symbol, id)` | `PersistResult<MarketDataCheckpoint>` | Loads and validates one checkpoint payload |
| `load_latest(venue, symbol, kind)` | `PersistResult<Option<MarketDataCheckpoint>>` | Returns the newest valid checkpoint, optionally filtered by kind |
| `list_checkpoints(venue, symbol)` | `PersistResult<Vec<MarketDataCheckpointManifest>>` | Lists checkpoint metadata ordered by id |
| `validate_checkpoint(venue, symbol, id)` | `PersistResult<MarketDataCheckpointValidation>` | Validates checksum and payload length |
| `prune_old(venue, symbol, retain_last)` | `PersistResult<usize>` | Removes older checkpoint files and returns the count |

### Layout and recovery contract

Checkpoint files are stored as:

`<root>/<venue>/<symbol>/checkpoints/<checkpoint-id>.ofmc`

Each file carries magic/version, checkpoint kind, checkpoint id, WAL sequence,
provider sequence, normalized event sequence, creation timestamp, payload
version, payload length, and checksum. Existing ids are not overwritten, and
explicit ids must be greater than all existing ids for the venue/symbol.

A production recovery flow should:

1. call `load_latest` for the venue/symbol and desired kind,
2. restore the opaque payload into the host-owned book/analytics/signal state,
3. replay WAL records after `checkpoint.wal_sequence`,
4. validate sequence continuity before allowing strategy submission.

## Production Persistence Policy And Health

`MarketDataPersistencePolicy` describes the host's production writer mode:
disabled, inline strict, bounded async, or best effort. It also records the
failure action to take when persistence degrades.

`MarketDataPersistenceHealth` reports enabled/degraded state, queue depth,
record lag, nanosecond lag, pending bytes, dropped records, WAL write/sync
failures, and last error text. These fields are intentionally host-owned so a
runtime can wire persistence degradation into OMS safety policy without forcing
an async writer into `of_persist`.

## Backpressure Policy

`MarketDataBackpressurePolicy` evaluates writer queue depth, record lag,
nanosecond lag, pending bytes, and degraded persistence state for one candidate
record. `evaluate_market_data_backpressure` returns a deterministic decision
that tells the host whether to accept, reject, drop current, drop queued oldest,
drop queued lowest-priority, stop market data, stop trading, fail the process,
or switch to memory-only retention.

The helper does not own a queue or spawn a writer. It is a stable policy surface
for runtimes that need explicit slow-consumer handling without silently losing
market-data evidence.

## Ordering and Range Rules

- Append methods preserve append order inside each file.
- `read_books*` and `read_trades*` preserve the stored file order.
- `read_events*` merges both streams by ascending sequence.
- Range bounds are inclusive.
- `None` as a bound means open-ended on that side.
- Missing stream files return an empty vector instead of an error.

## Error Semantics

### `PersistError`

| Variant | Meaning |
| --- | --- |
| `Io(std::io::Error)` | Filesystem or parse failure |

Malformed JSONL lines are surfaced as `Io` with `InvalidData`.

## Retention Behavior

- Retention is enforced during normal append flows.
- Oldest files are pruned first when `max_total_bytes` is exceeded.
- Files older than `max_age_secs` are pruned when age retention is enabled.
- The crate does not run a background compactor or daemon.
- Binary WAL sync is configured per writer with `sync_on_write`.

## When To Use `of_persist`

- Use it from the runtime when you want normalized event persistence.
- Use it directly when building replay, audit, or research tools.
- Use `examples/replay_cli` when you want a ready-made discovery-and-replay
  command-line workflow.
