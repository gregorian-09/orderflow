# of_core

`of_core` defines the canonical data model and analytics primitives used across the Orderflow stack.
It is provider-agnostic and intentionally lightweight so every binding (C, Python, Java) can rely on
the same normalized semantics.

## What This Crate Contains

- Market identity: [`SymbolId`]
- Event model: [`TradePrint`], [`BookUpdate`], [`BookLevel`], [`BookSnapshot`], [`Side`], [`BookAction`]
- Quality flags: [`DataQualityFlags`]
- Runtime outputs: [`AnalyticsSnapshot`], [`DerivedAnalyticsSnapshot`], [`SessionCandleSnapshot`], [`IntervalCandleSnapshot`], [`SignalSnapshot`], [`SignalState`]
- Deterministic analytics engine: [`AnalyticsAccumulator`]

## New In 0.2.0

Relative to the `0.1.x` line, `of_core` now includes:

- [`BookSnapshot`] as a first-class materialized depth model
- [`DerivedAnalyticsSnapshot`] for additive totals such as `vwap` and `trade_count`
- [`SessionCandleSnapshot`] for session-wide OHLC state
- [`IntervalCandleSnapshot`] for rolling-window OHLC state

These are additive data-model extensions. They do not change the older
`AnalyticsSnapshot` contract.

## Public API Inventory

Public types:

- [`SymbolId`]
- [`Side`]
- [`BookAction`]
- [`BookUpdate`]
- [`BookLevel`]
- [`BookSnapshot`]
- [`TradePrint`]
- [`AnalyticsSnapshot`]
- [`DerivedAnalyticsSnapshot`]
- [`SessionCandleSnapshot`]
- [`IntervalCandleSnapshot`]
- [`SignalState`]
- [`SignalSnapshot`]
- [`DataQualityFlags`]
- [`AnalyticsAccumulator`]

Public `DataQualityFlags` methods:

- `NONE`
- `STALE_FEED`
- `SEQUENCE_GAP`
- `CLOCK_SKEW`
- `DEPTH_TRUNCATED`
- `OUT_OF_ORDER`
- `ADAPTER_DEGRADED`
- [`DataQualityFlags::bits`]
- [`DataQualityFlags::from_bits_truncate`]
- [`DataQualityFlags::intersects`]

Public `AnalyticsAccumulator` methods:

- [`AnalyticsAccumulator::on_trade`]
- [`AnalyticsAccumulator::reset_session_delta`]
- [`AnalyticsAccumulator::reset_session`]
- [`AnalyticsAccumulator::snapshot`]
- [`AnalyticsAccumulator::derived_snapshot`]
- [`AnalyticsAccumulator::session_candle_snapshot`]
- [`AnalyticsAccumulator::interval_candle_snapshot`]

## Design Principles

- Deterministic arithmetic: prices and sizes are integer values, avoiding float drift in replay/backtests.
- Stable schema: types are designed for cross-language transport and long-lived storage.
- Minimal dependencies: this crate stays small so it can be embedded broadly.

## Type Semantics Reference

### Market identity and direction

- [`SymbolId`] is the canonical identity key used everywhere in the project.
  `venue` should be the normalized venue name and `symbol` should remain stable for the life of a stream.
- [`Side`] uses `Bid` and `Ask` for both book updates and trade aggressor direction.
- [`BookAction`] uses `Upsert` for insert-or-replace semantics and `Delete` for level removal.

### Book event types

- [`BookUpdate`] represents a single level mutation.
  `level` is the depth index from top of book.
  `price` and `size` are integer-normalized values.
  `sequence`, `ts_exchange_ns`, and `ts_recv_ns` preserve replay ordering and latency analysis.
- [`BookLevel`] is the materialized view of one price level after runtime consolidation.
- [`BookSnapshot`] is the full reconstructed book for one symbol at one point in time.
  `bids` and `asks` are level-ordered arrays.
  `last_sequence` is the sequence of the last applied update.
  `ts_exchange_ns` and `ts_recv_ns` are copied from that last applied update.

### Trade event types

- [`TradePrint`] represents one normalized trade.
  `price` and `size` are integer-normalized.
  `aggressor_side` is the trade direction used by the analytics engine.
  `sequence` may be zero when a source does not provide a venue sequence.

### Analytics and signal output types

- [`AnalyticsSnapshot`] is the base session analytics payload.
  It includes directional volume, delta, cumulative delta, POC, value area, and the active quality flags.
- [`DerivedAnalyticsSnapshot`] adds additive session totals that were intentionally kept out of the original analytics payload so older consumers would not break.
- [`SessionCandleSnapshot`] is a session-wide candle view derived from ingested trades.
- [`IntervalCandleSnapshot`] is a rolling-window candle view computed on demand from recent trades for a caller-supplied `window_ns`.
- [`SignalState`] is the stable directional state machine used across the runtime and bindings.
- [`SignalSnapshot`] packages state, confidence, reason text, and quality flags for downstream consumers.

## AnalyticsAccumulator Contract

[`AnalyticsAccumulator`] is session-oriented.

- [`AnalyticsAccumulator::on_trade`] mutates all session analytics state from one trade.
- [`AnalyticsAccumulator::reset_session_delta`] clears directional volume and delta state but keeps longer-lived session context that is not explicitly reset.
- [`AnalyticsAccumulator::reset_session`] clears the full session state, including candle and derived totals.
- [`AnalyticsAccumulator::snapshot`] returns the base analytics payload.
- [`AnalyticsAccumulator::derived_snapshot`] returns additive totals such as `vwap` and `average_trade_size`.
- [`AnalyticsAccumulator::session_candle_snapshot`] returns the session candle built from all trades seen since the last session reset.
- [`AnalyticsAccumulator::interval_candle_snapshot`] computes a rolling candle over the recent trade window without mutating session state.

Important behavior:

- No book data is required for `AnalyticsAccumulator`; it is trade-driven.
- All price and size arithmetic uses integer math at ingest time and converts only where a derived floating result is needed, such as `vwap`.
- `point_of_control` is volume-based, not quote-based.
- Value area fields are derived from traded-volume distribution, not full order-book depth.

## Quick Start

```rust
use of_core::{AnalyticsAccumulator, Side, SymbolId, TradePrint};

let symbol = SymbolId {
    venue: "CME".to_string(),
    symbol: "ESM6".to_string(),
};

let mut acc = AnalyticsAccumulator::default();
acc.on_trade(&TradePrint {
    symbol,
    price: 505_000,
    size: 10,
    aggressor_side: Side::Ask,
    sequence: 1,
    ts_exchange_ns: 1,
    ts_recv_ns: 2,
});

let snap = acc.snapshot();
assert_eq!(snap.buy_volume, 10);
assert_eq!(snap.delta, 10);
```

## Quality Flags

[`DataQualityFlags`] is a bitset used to express data-health issues such as stale feed, sequence gaps,
and out-of-order events. Signals and runtime gating can use these flags to block unsafe decisions.

```rust
use of_core::DataQualityFlags;

let q = DataQualityFlags::STALE_FEED | DataQualityFlags::SEQUENCE_GAP;
assert!(q.intersects(DataQualityFlags::STALE_FEED));
assert_eq!(q.bits() & DataQualityFlags::SEQUENCE_GAP.bits(), DataQualityFlags::SEQUENCE_GAP.bits());
```

## Analytics Model Notes

- `delta` tracks current session directional imbalance.
- `cumulative_delta` retains directional accumulation over time.
- `point_of_control` is computed as highest-volume price level.
- `value_area_low` / `value_area_high` approximate the high-volume range around POC.
- [`DerivedAnalyticsSnapshot`] adds session totals such as `total_volume`, `trade_count`, `vwap`,
  `average_trade_size`, and `imbalance_bps` without changing the original analytics payload.
- [`SessionCandleSnapshot`] adds a candle-style session view with `open`, `high`, `low`, `close`,
  `trade_count`, and first/last exchange timestamps.
- [`IntervalCandleSnapshot`] adds a parameterized rolling-window candle view with `window_ns`,
  `open`, `high`, `low`, `close`, `trade_count`, `total_volume`, `vwap`, and first/last exchange timestamps.

For full orchestration and adapter integration, see `of_runtime`.

## Book Snapshot Model

[`BookSnapshot`] materializes the latest known order book for one symbol:

- `bids`: bid-side levels ordered by `level`
- `asks`: ask-side levels ordered by `level`
- `last_sequence`: sequence number of the most recent applied book event
- `ts_exchange_ns` / `ts_recv_ns`: timestamps from the most recent applied book event

This snapshot model is used by the runtime and exposed through the FFI and bindings.

## Choosing the Right Snapshot Type

- Use [`AnalyticsSnapshot`] when you want the original compact analytics contract.
- Use [`DerivedAnalyticsSnapshot`] when you need totals such as `trade_count` or `vwap`.
- Use [`SessionCandleSnapshot`] when you want one candle for the entire active session.
- Use [`IntervalCandleSnapshot`] when you want a rolling lookback window.
- Use [`BookSnapshot`] when you need reconstructed depth instead of raw incremental updates.

## Real-World Use Cases

### 1. Offline research and replay analytics

Use [`AnalyticsAccumulator`] directly when you already have normalized trade
data and want deterministic session analytics without the full runtime.

Typical use cases:

- replaying one session from a CSV/JSONL converter
- validating a strategy hypothesis before wiring live adapters
- generating session summaries for dashboards or reports

### 2. Shared schema between components

Use `of_core` as the common contract when writing:

- custom adapters that emit normalized [`BookUpdate`] and [`TradePrint`]
- custom signal modules that consume [`AnalyticsSnapshot`]
- persistence or replay tools that must stay aligned with runtime semantics

### 3. Strategy prototyping before runtime integration

For early-stage ideas, it is often faster to work only with [`TradePrint`],
[`AnalyticsAccumulator`], and the output snapshot types before integrating with
`of_runtime`.

## Detailed Example: Build Session Analytics From Trades

```rust
use of_core::{AnalyticsAccumulator, Side, SymbolId, TradePrint};

fn main() {
    let symbol = SymbolId {
        venue: "CME".to_string(),
        symbol: "ESM6".to_string(),
    };

    let trades = vec![
        TradePrint {
            symbol: symbol.clone(),
            price: 505_000,
            size: 8,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 1_000,
            ts_recv_ns: 1_100,
        },
        TradePrint {
            symbol: symbol.clone(),
            price: 505_025,
            size: 4,
            aggressor_side: Side::Ask,
            sequence: 2,
            ts_exchange_ns: 2_000,
            ts_recv_ns: 2_100,
        },
        TradePrint {
            symbol,
            price: 505_000,
            size: 6,
            aggressor_side: Side::Bid,
            sequence: 3,
            ts_exchange_ns: 3_000,
            ts_recv_ns: 3_100,
        },
    ];

    let mut acc = AnalyticsAccumulator::default();
    for trade in &trades {
        acc.on_trade(trade);
    }

    let analytics = acc.snapshot();
    let derived = acc.derived_snapshot();
    let session_candle = acc.session_candle_snapshot();
    let interval_candle = acc.interval_candle_snapshot(5_000);

    println!(
        "delta={} poc={} total_volume={} vwap={:.2}",
        analytics.delta,
        analytics.point_of_control,
        derived.total_volume,
        derived.vwap
    );
    println!(
        "session ohlc=({}, {}, {}, {}) trades={}",
        session_candle.open,
        session_candle.high,
        session_candle.low,
        session_candle.close,
        session_candle.trade_count
    );
    println!(
        "interval close={} interval_vwap={:.2}",
        interval_candle.close,
        interval_candle.vwap
    );
}
```

## Strategy-Prototyping Pattern

A common progression is:

1. use [`TradePrint`] and [`AnalyticsAccumulator`] to compute deterministic features
2. test threshold logic over [`AnalyticsSnapshot`] and [`DerivedAnalyticsSnapshot`]
3. once stable, move the logic into an `of_signals::SignalModule`
4. finally run it inside `of_runtime` for live or replay orchestration
