# of_adapters

`of_adapters` defines the provider boundary between venue-specific feeds and the normalized Orderflow runtime.
It standardizes lifecycle, subscription, polling, and health reporting while keeping provider protocol details isolated.

## Core API

- Trait: [`MarketDataAdapter`]
- Factory: [`create_adapter`]
- Events: [`RawEvent`] (`Book`, `Trade`)
- Config: [`AdapterConfig`], [`ProviderKind`], [`CredentialsRef`]
- Health: [`AdapterHealth`]

## Provider Strategy

The crate is built around a feature-gated provider model:

- Always available: `Mock` provider
- Optional: `Rithmic`, `CQG`, `Binance` (enable via Cargo features)

This keeps the default build deterministic while allowing production adapters where needed.

## Create an Adapter

```rust
use of_adapters::{create_adapter, AdapterConfig, ProviderKind};

let cfg = AdapterConfig {
    provider: ProviderKind::Mock,
    ..Default::default()
};

let mut adapter = create_adapter(&cfg).expect("adapter");
adapter.connect().expect("connect");
assert!(adapter.health().connected);
```

## Mock Adapter for Tests and Replays

[`MockAdapter`] is useful for deterministic tests and simulation pipelines:

```rust
use of_adapters::{MarketDataAdapter, MockAdapter, RawEvent, SubscribeReq};
use of_core::{Side, SymbolId, TradePrint};

let symbol = SymbolId {
    venue: "SIM".to_string(),
    symbol: "TEST".to_string(),
};

let mut adapter = MockAdapter::default();
adapter.connect().expect("connect");
adapter.subscribe(SubscribeReq {
    symbol: symbol.clone(),
    depth_levels: 10,
}).expect("subscribe");

adapter.push_event(RawEvent::Trade(TradePrint {
    symbol,
    price: 100,
    size: 1,
    aggressor_side: Side::Ask,
    sequence: 1,
    ts_exchange_ns: 1,
    ts_recv_ns: 2,
}));

let mut out = Vec::new();
let n = adapter.poll(&mut out).expect("poll");
assert_eq!(n, 1);
assert_eq!(out.len(), 1);
```

## Error Handling

All adapter operations return [`AdapterResult<T>`] with [`AdapterError`], covering:

- disconnected state
- missing configuration
- provider feature not enabled at build time
- provider-specific operational errors
