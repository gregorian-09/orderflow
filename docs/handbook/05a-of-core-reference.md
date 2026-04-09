# `of_core` Reference

`of_core` is the canonical normalized data model for the project. It contains
provider-agnostic event types, analytics output types, signal output types, and
the deterministic trade-driven analytics accumulator used by the runtime and
bindings.

## Type Overview

| Type | Kind | Purpose |
| --- | --- | --- |
| `SymbolId` | struct | Canonical venue/symbol identity key |
| `Side` | enum | Bid/ask direction for book and trade semantics |
| `BookAction` | enum | Order-book mutation kind |
| `BookUpdate` | struct | One normalized incremental book mutation |
| `BookLevel` | struct | One materialized depth level in a snapshot |
| `BookSnapshot` | struct | Full reconstructed book state for one symbol |
| `TradePrint` | struct | One normalized trade event |
| `AnalyticsSnapshot` | struct | Base session analytics payload |
| `DerivedAnalyticsSnapshot` | struct | Additive session totals and ratios |
| `SessionCandleSnapshot` | struct | Session-wide candle view |
| `IntervalCandleSnapshot` | struct | Rolling-window candle view |
| `SignalState` | enum | Stable directional signal state |
| `SignalSnapshot` | struct | Signal output payload |
| `DataQualityFlags` | newtype bitset | Feed-quality flags |
| `AnalyticsAccumulator` | struct | Deterministic trade-driven analytics engine |

## Identity and Direction Types

### `SymbolId`

| Field | Type | Meaning |
| --- | --- | --- |
| `venue` | `String` | Normalized venue or exchange identifier |
| `symbol` | `String` | Venue-native or normalized instrument identifier |

Use `SymbolId` as the stable lookup key everywhere in the project. Runtime
state, persistence paths, subscriptions, and snapshots all key off this type.

### `Side`

| Variant | Meaning |
| --- | --- |
| `Bid` | Bid side or buy aggressor |
| `Ask` | Ask side or sell aggressor |

### `BookAction`

| Variant | Meaning |
| --- | --- |
| `Upsert` | Insert-or-update a level |
| `Delete` | Remove a level |

## Book Types

### `BookUpdate`

| Field | Type | Meaning |
| --- | --- | --- |
| `symbol` | `SymbolId` | Target symbol |
| `side` | `Side` | Bid or ask side |
| `level` | `u16` | Depth index from top of book |
| `price` | `i64` | Integer-normalized price |
| `size` | `i64` | Integer-normalized quantity |
| `action` | `BookAction` | Mutation type |
| `sequence` | `u64` | Venue sequence number |
| `ts_exchange_ns` | `u64` | Exchange timestamp |
| `ts_recv_ns` | `u64` | Local receive timestamp |

`BookUpdate` is the incremental event form used by adapters, persistence, and
external ingest. The runtime folds these updates into `BookSnapshot`.

### `BookLevel`

| Field | Type | Meaning |
| --- | --- | --- |
| `level` | `u16` | Depth index from top of book |
| `price` | `i64` | Materialized price at that depth |
| `size` | `i64` | Materialized size at that depth |

### `BookSnapshot`

| Field | Type | Meaning |
| --- | --- | --- |
| `symbol` | `SymbolId` | Snapshot symbol |
| `bids` | `Vec<BookLevel>` | Materialized bid ladder ordered by `level` |
| `asks` | `Vec<BookLevel>` | Materialized ask ladder ordered by `level` |
| `last_sequence` | `u64` | Sequence of the last applied book event |
| `ts_exchange_ns` | `u64` | Exchange timestamp of last applied event |
| `ts_recv_ns` | `u64` | Receive timestamp of last applied event |

## Trade Type

### `TradePrint`

| Field | Type | Meaning |
| --- | --- | --- |
| `symbol` | `SymbolId` | Target symbol |
| `price` | `i64` | Integer-normalized trade price |
| `size` | `i64` | Integer-normalized trade quantity |
| `aggressor_side` | `Side` | Direction of aggressive liquidity taking |
| `sequence` | `u64` | Venue sequence number, or `0` when unavailable |
| `ts_exchange_ns` | `u64` | Exchange timestamp |
| `ts_recv_ns` | `u64` | Local receive timestamp |

`AnalyticsAccumulator` is driven from `TradePrint`, not from book updates.

## Analytics Output Types

### `AnalyticsSnapshot`

| Field | Type | Meaning |
| --- | --- | --- |
| `buy_volume` | `i64` | Session buy-side aggressive volume |
| `sell_volume` | `i64` | Session sell-side aggressive volume |
| `delta` | `i64` | `buy_volume - sell_volume` for current session state |
| `cumulative_delta` | `i64` | Long-running directional accumulation |
| `point_of_control` | `i64` | Price with highest traded volume |
| `value_area_low` | `i64` | Lower bound of volume-based value area |
| `value_area_high` | `i64` | Upper bound of volume-based value area |
| `quality_flags` | `u32` | Active `DataQualityFlags` bitset |

### `DerivedAnalyticsSnapshot`

| Field | Type | Meaning |
| --- | --- | --- |
| `total_volume` | `i64` | Total traded volume seen in session |
| `trade_count` | `u64` | Number of trades processed |
| `vwap` | `f64` | Volume-weighted average price |
| `average_trade_size` | `f64` | Mean trade size |
| `imbalance_bps` | `f64` | Directional imbalance in basis points |

These fields are additive and intentionally separate from the original
`AnalyticsSnapshot` contract.

### `SessionCandleSnapshot`

| Field | Type | Meaning |
| --- | --- | --- |
| `open` | `i64` | First trade price in current session |
| `high` | `i64` | Highest trade price in current session |
| `low` | `i64` | Lowest trade price in current session |
| `close` | `i64` | Most recent trade price in current session |
| `trade_count` | `u64` | Trades seen in current session |
| `first_ts_exchange_ns` | `u64` | First exchange timestamp in session |
| `last_ts_exchange_ns` | `u64` | Most recent exchange timestamp in session |

### `IntervalCandleSnapshot`

| Field | Type | Meaning |
| --- | --- | --- |
| `window_ns` | `u64` | Requested rolling lookback window |
| `open` | `i64` | First trade price inside the active interval |
| `high` | `i64` | Highest trade price inside the interval |
| `low` | `i64` | Lowest trade price inside the interval |
| `close` | `i64` | Most recent trade price inside the interval |
| `trade_count` | `u64` | Number of trades inside the interval |
| `total_volume` | `i64` | Total volume inside the interval |
| `vwap` | `f64` | Interval VWAP |
| `first_ts_exchange_ns` | `u64` | First exchange timestamp in interval |
| `last_ts_exchange_ns` | `u64` | Last exchange timestamp in interval |

## Signal Output Types

### `SignalState`

| Variant | Meaning |
| --- | --- |
| `Neutral` | No directional bias |
| `LongBias` | Bullish directional bias |
| `ShortBias` | Bearish directional bias |
| `Blocked` | Signal intentionally blocked by quality gating |

### `SignalSnapshot`

| Field | Type | Meaning |
| --- | --- | --- |
| `state` | `SignalState` | Current directional state |
| `confidence` | `u8` | Module-defined confidence score |
| `reason` | `String` | Short human-readable rationale |
| `quality_flags` | `u32` | Quality bitset associated with signal state |

## `DataQualityFlags`

| Constant | Bit | Meaning |
| --- | --- | --- |
| `NONE` | `0` | No quality issue |
| `STALE_FEED` | `1 << 0` | Feed has gone stale |
| `SEQUENCE_GAP` | `1 << 1` | Sequence discontinuity was observed |
| `CLOCK_SKEW` | `1 << 2` | Timestamp skew was observed |
| `DEPTH_TRUNCATED` | `1 << 3` | Full intended depth was not available |
| `OUT_OF_ORDER` | `1 << 4` | Out-of-order sequence was observed |
| `ADAPTER_DEGRADED` | `1 << 5` | Adapter or bridge is degraded/reconnecting |

### Methods

| Method | Returns | Meaning |
| --- | --- | --- |
| `bits()` | `u32` | Raw bitset |
| `from_bits_truncate(bits)` | `DataQualityFlags` | Converts arbitrary bits into known flags |
| `intersects(other)` | `bool` | Tests overlap with another flag set |

## `AnalyticsAccumulator`

`AnalyticsAccumulator` is the deterministic session analytics engine.

### Methods

| Method | Returns | Meaning |
| --- | --- | --- |
| `on_trade(&TradePrint)` | `()` | Updates analytics state from one trade |
| `reset_session_delta()` | `()` | Clears directional delta-related state |
| `reset_session()` | `()` | Clears full session state |
| `snapshot()` | `AnalyticsSnapshot` | Returns base analytics state |
| `derived_snapshot()` | `DerivedAnalyticsSnapshot` | Returns additive totals |
| `session_candle_snapshot()` | `SessionCandleSnapshot` | Returns session candle |
| `interval_candle_snapshot(window_ns)` | `IntervalCandleSnapshot` | Returns rolling-window candle |

### Behavior Notes

- Prices and sizes are integer-normalized to avoid drift in replay and storage.
- `point_of_control` and value area are trade-volume derived, not quote derived.
- `interval_candle_snapshot(window_ns)` is query-time windowing over recent
  trades and does not reset or mutate session analytics.
- `reset_session()` should be used when the caller wants a new session boundary.

## When To Use `of_core`

- Use it directly for deterministic analytics tests and offline research.
- Use it as the shared schema contract when writing adapters, signals, or
  persistence tooling.
- Use the runtime crate when you need orchestration, subscription management,
  book reconstruction, health, or persistence integration.
