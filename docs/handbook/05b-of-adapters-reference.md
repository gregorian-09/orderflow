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
| `AdapterQualityLevel` | enum | Conservative provider maturity label |
| `AdapterDescriptor` | struct | Static provider capability descriptor |
| `AdapterConfig` | struct | Adapter factory configuration |
| `CredentialsRef` | struct | Environment-variable references for secrets |
| `MockAdapter` | struct | Deterministic in-memory adapter |
| `create_adapter` | fn | Provider factory |
| `adapter_descriptors` | fn | Returns all known provider descriptors |
| `compiled_adapter_descriptors` | fn | Returns descriptors enabled in this build |
| `describe_adapter` | fn | Returns descriptor for one provider |
| `adapter_feature_enabled` | fn | Returns whether provider can be constructed |

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

### `AdapterQualityLevel`

| Variant | Meaning |
| --- | --- |
| `Simulation` | Local deterministic adapter for tests, demos, and replay |
| `Scaffold` | Build-time integration scaffold, not live-production complete |
| `Functional` | Live-capable adapter that still requires operator validation |
| `ProductionCandidate` | Candidate for production use with runbook, recovery, and metrics |

Quality levels are descriptive and conservative. They are not a claim that an
adapter is certified for a venue or suitable for live capital without user
validation.

### `AdapterDescriptor`

| Field | Meaning |
| --- | --- |
| `provider` | `ProviderKind` used by `AdapterConfig` |
| `provider_id` | Stable lowercase id used in JSON and diagnostics |
| `display_name` | Human-readable provider name |
| `feature` | Required Cargo feature, or `None` for always-available providers |
| `compiled` | Whether this binary has the provider feature enabled |
| `quality` | `AdapterQualityLevel` |
| `supports_live` | Can connect to a live provider endpoint |
| `supports_replay` | Suitable for deterministic local/replay flows |
| `supports_trades` | Emits normalized trade events |
| `supports_order_book` | Emits normalized book/depth events |
| `supports_level2` | Supports level-2 depth updates |
| `supports_reconnect` | Has reconnect behavior |
| `supports_gap_recovery` | Has gap detection or recovery semantics |
| `supports_polling` | Driven through the runtime poll contract |
| `notes` | Operator-facing maturity or usage note |

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

## Discovery Functions

Use discovery before constructing adapters in dashboards, CLIs, bindings, or
plugin hosts:

| Function | Meaning |
| --- | --- |
| `adapter_descriptors()` | Lists all known providers, including feature-disabled providers |
| `compiled_adapter_descriptors()` | Lists providers compiled into the current binary |
| `describe_adapter(provider)` | Returns one descriptor |
| `adapter_feature_enabled(provider)` | Checks whether `create_adapter` can attempt that provider |

Discovery functions do not open sockets, validate credentials, create adapters,
or mutate runtime state.

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
- Tracks depth update-id continuity per symbol using Binance `U`, `u`, and
  `pu` fields.
- Drops duplicate depth updates before normalization.
- Marks the adapter degraded and schedules rebuild/reconnect handling on depth
  gaps.
- Supports an opt-in bounded pending event queue on direct `BinanceAdapter`
  instances with `with_max_queue_depth(...)` or `set_max_queue_depth(...)`.
  The default value `0` is unbounded for compatibility.
- Sheds the candidate normalized event when the configured queue bound is full,
  increments dropped/backpressure counters, and marks health degraded.
- Supports opt-in bounded raw inbound message capture on direct
  `BinanceAdapter` instances with `with_raw_capture_capacity(...)` or
  `set_raw_capture_capacity(...)`; the default capacity `0` disables capture.
- Drops oldest captured raw messages when capture capacity is full and reports
  raw capture depth, capacity, and drop counters in health metadata.
- Supervises live activity timeout and reconnects with backoff.
- Exposes Binance health metadata for messages received, normalized events,
  parse errors, dropped events, backpressure events, duplicate depth updates,
  gap count, snapshot rebuild count, queue depth, and max queue depth.
- Reports parse latency and normalization latency as aggregate sample,
  average-nanosecond, and max-nanosecond health fields.
- Reports per-symbol last depth update ids and redacts endpoint userinfo,
  query strings, and fragments before health metadata is emitted.

## Choosing `of_adapters`

- Use this crate when adding a new provider or testing runtime behavior with a
  custom adapter implementation.
- Use `MockAdapter` when you need deterministic integration tests.
- Use `of_runtime` when you want orchestration, health, persistence, and
  snapshot production on top of adapters.
