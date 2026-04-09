# `of_adapters` Reference

`of_adapters` is the normalized provider boundary. It hides provider-specific
transport and protocol details behind a small polling interface that emits only
normalized `BookUpdate` and `TradePrint` events.

## Public API Map

| Item | Kind | Purpose |
| --- | --- | --- |
| `SubscribeReq` | struct | Subscription request forwarded to an adapter |
| `AdapterHealth` | struct | Adapter transport/supervision health snapshot |
| `RawEvent` | enum | Normalized output stream from adapters |
| `AdapterError` | enum | Adapter-layer failure contract |
| `AdapterResult<T>` | type alias | `Result<T, AdapterError>` |
| `MarketDataAdapter` | trait | Common provider interface |
| `ProviderKind` | enum | Adapter factory selector |
| `AdapterConfig` | struct | Adapter factory configuration |
| `CredentialsRef` | struct | Environment-variable references for secrets |
| `MockAdapter` | struct | Deterministic in-memory adapter |
| `create_adapter` | fn | Provider factory |

## Core Types

### `SubscribeReq`

| Field | Type | Meaning |
| --- | --- | --- |
| `symbol` | `SymbolId` | Target symbol |
| `depth_levels` | `u16` | Requested book depth |

`depth_levels` is advisory. Some providers honor it directly, while simpler
providers may provide a fixed depth.

### `AdapterHealth`

| Field | Type | Meaning |
| --- | --- | --- |
| `connected` | `bool` | Transport/session is currently up |
| `degraded` | `bool` | Feed is reconnecting, stale, or otherwise unhealthy |
| `last_error` | `Option<String>` | Latest human-readable failure, if known |
| `protocol_info` | `Option<String>` | Provider-specific diagnostic text |

### `RawEvent`

| Variant | Payload | Meaning |
| --- | --- | --- |
| `Book` | `BookUpdate` | Incremental order-book mutation |
| `Trade` | `TradePrint` | Trade print |

Adapters do not emit provider-native payloads across the public boundary.

### `AdapterError`

| Variant | Meaning |
| --- | --- |
| `Disconnected` | Operation requires a connected adapter |
| `NotConfigured(&'static str)` | Required config value is missing |
| `FeatureDisabled(&'static str)` | Provider feature not compiled in |
| `Other(String)` | Provider-specific or unexpected failure |

### `ProviderKind`

| Variant | Meaning |
| --- | --- |
| `Mock` | Deterministic in-memory adapter |
| `Rithmic` | Rithmic provider |
| `Cqg` | CQG provider |
| `Binance` | Binance provider |

## Configuration Types

### `AdapterConfig`

| Field | Type | Meaning |
| --- | --- | --- |
| `provider` | `ProviderKind` | Factory selector |
| `credentials` | `Option<CredentialsRef>` | Env-var references for auth |
| `endpoint` | `Option<String>` | Provider endpoint URI |
| `app_name` | `Option<String>` | Optional client/bridge identifier |

### `CredentialsRef`

| Field | Type | Meaning |
| --- | --- | --- |
| `key_id_env` | `String` | Env var containing key id or username |
| `secret_env` | `String` | Env var containing secret/password/token |

These fields hold environment variable names, not raw secret values.

## `MarketDataAdapter` Trait Contract

| Method | Returns | Meaning |
| --- | --- | --- |
| `connect()` | `AdapterResult<()>` | Establishes provider session/transport |
| `subscribe(req)` | `AdapterResult<()>` | Starts or refreshes symbol delivery |
| `unsubscribe(symbol)` | `AdapterResult<()>` | Stops symbol delivery |
| `poll(out)` | `AdapterResult<usize>` | Appends ready events into caller-owned buffer |
| `health()` | `AdapterHealth` | Returns current supervision snapshot |

### Behavioral Rules

- `poll(out)` appends into `out`; callers should clear the buffer themselves if
  they do not want accumulated results.
- `connect()` should be safe to call only at startup or controlled reconnect
  points; runtime code treats connection as adapter-owned.
- `subscribe()` should behave as update-or-refresh for repeated calls on the
  same symbol.
- `health()` must not mutate adapter state.

## Factory Function

### `create_adapter(&AdapterConfig)`

Returns a boxed adapter for the selected provider.

Factory behavior:

- `ProviderKind::Mock` is always available.
- `Rithmic`, `Cqg`, and `Binance` require their Cargo features.
- If a feature is not enabled, `FeatureDisabled` is returned.
- If required settings such as endpoint or credentials are missing, the factory
  returns `NotConfigured`.

## `MockAdapter`

`MockAdapter` is the deterministic adapter used in tests, replay flows, and
offline examples.

### Public method

| Method | Returns | Meaning |
| --- | --- | --- |
| `push_event(event)` | `()` | Queues a normalized event for later `poll()` |

## Provider Notes

### Rithmic

- Supports deterministic mock mode for testing.
- Live mode supervises websocket/bridge activity.
- Reconnect backoff, subscription replay, and health metadata are exposed
  through `AdapterHealth`.

### CQG

- Supports reconnect/resubscribe and sequencing-aware polling behavior.

### Binance

- Parses public trade and depth events.
- Supervises live activity timeout and reconnects with backoff.

## Choosing `of_adapters`

- Use this crate when adding a new provider or testing runtime behavior with a
  custom adapter implementation.
- Use `MockAdapter` when you need deterministic integration tests.
- Use `of_runtime` when you want orchestration, health, persistence, and
  snapshot production on top of adapters.
