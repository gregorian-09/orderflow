# of_adapters

`of_adapters` defines the provider boundary between venue-specific feeds and the normalized Orderflow runtime.
It standardizes lifecycle, subscription, polling, and health reporting while keeping provider protocol details isolated.

## Core API

- Trait: [`MarketDataAdapter`]
- Factory: [`create_adapter`]
- Events: [`RawEvent`] (`Book`, `Trade`)
- Config: [`AdapterConfig`], [`ProviderKind`], [`CredentialsRef`]
- Health: [`AdapterHealth`]

## Public API Inventory

Public types:

- [`SubscribeReq`]
- [`AdapterHealth`]
- [`RawEvent`]
- [`AdapterError`]
- [`AdapterResult<T>`]
- [`ProviderKind`]
- [`AdapterConfig`]
- [`CredentialsRef`]
- [`MockAdapter`]

Public functions and methods:

- [`create_adapter`]
- [`MockAdapter::push_event`]

[`MarketDataAdapter`] trait methods:

- `connect()`
- `subscribe(SubscribeReq)`
- `unsubscribe(SymbolId)`
- `poll(&mut Vec<RawEvent>)`
- `health() -> AdapterHealth`

## Provider Strategy

The crate is built around a feature-gated provider model:

- Always available: `Mock` provider
- Optional: `Rithmic`, `CQG`, `Binance` (enable via Cargo features)

This keeps the default build deterministic while allowing production adapters where needed.

Current provider notes:

- `Rithmic`:
  - mock mode emits deterministic book and trade events for end-to-end testing
  - live `ws://` / `wss://` mode now performs websocket reachability validation before reporting connected
  - live mode tracks heartbeat/message activity, schedules reconnect with backoff, and replays subscriptions after reconnect
  - live mode accepts normalized JSON `book` / `trade` / `heartbeat` payloads from bridge processes
  - health metadata includes mode, endpoint, app name, uptime, reconnect attempt, subscription count, and activity ages
- `CQG`:
  - reconnect/resubscribe and sequencing logic are implemented
- `Binance`:
  - live websocket transport parses trade and depth events
  - live mode schedules reconnect with backoff on disconnect or market-data timeout
  - reconnect replays active subscriptions automatically
  - health metadata includes reconnect attempt, subscription count, and message/data ages

## Trait Contract

[`MarketDataAdapter`] is intentionally small, but each method has a specific contract:

- `connect()` establishes transport/session state and should be idempotent where practical.
- `subscribe(SubscribeReq)` starts or updates delivery for one symbol and depth.
- `unsubscribe(SymbolId)` stops delivery for that symbol.
- `poll(&mut Vec<RawEvent>)` appends zero or more normalized events into the caller-owned buffer and returns the number appended.
- `health()` returns the latest transport/supervision state without mutating adapter state.

Normalization rules:

- adapters emit only [`RawEvent::Book`] and [`RawEvent::Trade`]
- provider-native protocol details stay inside the adapter implementation
- all emitted symbols, sequences, timestamps, and sides should already be normalized for runtime consumption

## AdapterConfig Reference

- `provider`: selects `Mock`, `Rithmic`, `Cqg`, or `Binance`.
- `credentials`: optional env-var references for providers that need authenticated bootstrap.
- `endpoint`: websocket or provider endpoint URI for live adapters.
- `app_name`: optional client or bridge identifier used in health metadata where supported.

[`CredentialsRef`] contains env-var names, not secret values:

- `key_id_env`: env var that stores the provider user/key id
- `secret_env`: env var that stores the secret/password/token

## Health Semantics

[`AdapterHealth`] is the bridge between provider supervision and runtime quality decisions.

- `connected = true` means the transport/session is considered up
- `degraded = true` means the adapter is reconnecting, stale, or otherwise unhealthy enough for runtime quality gating
- `last_error` is the latest human-readable adapter failure if known
- `protocol_info` is provider-specific diagnostic text intended for logging and dashboards

## Factory Behavior

[`create_adapter`] is feature-gated.

- `ProviderKind::Mock` is always available.
- live providers require their Cargo feature to be enabled.
- requesting a provider without its feature returns [`AdapterError::FeatureDisabled`].
- missing endpoint or credential references return [`AdapterError::NotConfigured`].

## Choosing an Adapter

- Use [`MockAdapter`] for deterministic tests, replay, and CI.
- Use `Rithmic` or `CQG` when an authenticated futures feed is needed.
- Use `Binance` when a public crypto depth/trade feed is sufficient.
- Keep provider-specific auth, transport, and reconnect tuning out of runtime code and inside the adapter layer.

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

## Subscription Semantics

- [`SubscribeReq::depth_levels`] is advisory and provider-dependent; mock and depth-aware providers use it directly.
- repeated subscribe calls for the same symbol should be treated as update-or-refresh, not as a duplicate stream request.
- unsubscribe should remove future delivery for that symbol, but does not retroactively clear already-polled events from the caller buffer.
