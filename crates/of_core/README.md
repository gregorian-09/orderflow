# of_core

`of_core` defines the canonical data model and analytics primitives used across the Orderflow stack.
It is provider-agnostic and intentionally lightweight so every binding (C, Python, Java) can rely on
the same normalized semantics.

## What This Crate Contains

- Market identity: [`SymbolId`]
- Event model: [`TradePrint`], [`BookUpdate`], [`Side`], [`BookAction`]
- Quality flags: [`DataQualityFlags`]
- Runtime outputs: [`AnalyticsSnapshot`], [`SignalSnapshot`], [`SignalState`]
- Deterministic analytics engine: [`AnalyticsAccumulator`]

## Design Principles

- Deterministic arithmetic: prices and sizes are integer values, avoiding float drift in replay/backtests.
- Stable schema: types are designed for cross-language transport and long-lived storage.
- Minimal dependencies: this crate stays small so it can be embedded broadly.

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

For full orchestration and adapter integration, see `of_runtime`.
