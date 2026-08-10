# `of_runtime` Reference

`of_runtime` is the orchestration layer that connects adapters, analytics,
signals, book state, persistence, health reporting, and external ingest flows.

## Public API Map

| Item | Kind | Purpose |
| --- | --- | --- |
| `EngineConfig` | struct | Runtime control-plane configuration |
| `RuntimeError` | enum | Runtime error contract |
| `ExternalFeedPolicy` | struct | External ingest supervision rules |
| `RuntimeAdapterStatus` | struct | Descriptor, compatibility health, and typed operational status |
| `Engine<A, S>` | struct | Generic runtime |
| `DefaultEngine` | type alias | Runtime with boxed adapter and default signal |
| `ConfigCompatibilityMode` | enum | Config loader compatibility state |
| `ConfigLoadReport` | struct | Detailed config loading result |
| `build_default_engine` | fn | Convenience constructor |
| `load_engine_config_from_path` | fn | Loads config only |
| `load_engine_config_report_from_path` | fn | Loads config plus compatibility report |
| `validate_startup_config` | fn | Validates config and env prerequisites |
| `adapter_inventory_json` | fn | Lists known adapter descriptors for this build |

## Configuration Types

### `EngineConfig`

| Field | Type | Meaning |
| --- | --- | --- |
| `instance_id` | `String` | Logical runtime name for logs/metrics |
| `enable_persistence` | `bool` | Enables JSONL persistence |
| `data_root` | `String` | Persistence root |
| `audit_log_path` | `String` | Audit log file path |
| `audit_max_bytes` | `u64` | Audit rotation size |
| `audit_max_files` | `u32` | Max rotated audit files retained |
| `audit_redact_tokens` | `Vec<String>` | Tokens redacted from audit details |
| `data_retention_max_bytes` | `u64` | Persistence byte cap |
| `data_retention_max_age_secs` | `u64` | Persistence age cap |
| `adapter` | `AdapterConfig` | Adapter/provider config |
| `signal_threshold` | `i64` | Default threshold used by `build_default_engine` |

### `ExternalFeedPolicy`

| Field | Type | Meaning |
| --- | --- | --- |
| `stale_after_ms` | `u64` | Max ingest silence before stale status |
| `enforce_sequence` | `bool` | Enables sequence-gap/out-of-order checks |

### `RuntimeError`

| Variant | Meaning |
| --- | --- |
| `Adapter(String)` | Adapter/provider failure |
| `Config(String)` | Invalid config or missing prerequisite |
| `Io(String)` | Filesystem or I/O failure |
| `NotStarted` | Operation requires a started engine |

## Config Compatibility Types

### `ConfigCompatibilityMode`

| Variant | Meaning |
| --- | --- |
| `Strict` | Typed TOML/JSON parsing succeeded directly |
| `LegacyFallback` | Older flat-key config shape was accepted through compatibility loader |

### `ConfigLoadReport`

| Field | Type | Meaning |
| --- | --- | --- |
| `config` | `EngineConfig` | Loaded runtime config |
| `format` | `&'static str` | Source format, currently `json` or `toml` |
| `compatibility_mode` | `ConfigCompatibilityMode` | Strict or fallback mode |
| `warning` | `Option<String>` | Optional migration warning for callers |

#### Method

| Method | Returns | Meaning |
| --- | --- | --- |
| `used_legacy_fallback()` | `bool` | True when compatibility parsing was required |

## Engine Constructors and Top-Level Functions

| Function | Returns | Meaning |
| --- | --- | --- |
| `build_default_engine(cfg)` | `Result<DefaultEngine, RuntimeError>` | Builds runtime with factory adapter and default signal |
| `load_engine_config_from_path(path)` | `Result<EngineConfig, RuntimeError>` | Loads config file only |
| `load_engine_config_report_from_path(path)` | `Result<ConfigLoadReport, RuntimeError>` | Loads config plus compatibility diagnostics |
| `validate_startup_config(cfg)` | `Result<(), RuntimeError>` | Validates startup config and env vars |
| `adapter_inventory_json()` | `String` | Lists known adapter descriptors for this build |
| `signal_descriptor_inventory_json()` | `String` | Lists built-in signal descriptors for this build |

## `Engine<A, S>`

### Constructors and setup

| Method | Returns | Meaning |
| --- | --- | --- |
| `new(cfg, adapter, signal_module)` | `Engine<A, S>` | Creates engine with explicit adapter and signal |
| `with_persistence(persistence)` | `Engine<A, S>` | Attaches optional `RollingStore` |

### Lifecycle methods

| Method | Returns | Meaning |
| --- | --- | --- |
| `start()` | `Result<(), RuntimeError>` | Validates config and starts adapter/session |
| `stop()` | `()` | Stops runtime and adapter/session |
| `subscribe(symbol, depth_levels)` | `Result<(), RuntimeError>` | Adds or refreshes one symbol |
| `unsubscribe(symbol)` | `Result<(), RuntimeError>` | Removes one symbol |
| `reset_symbol_session(symbol)` | `Result<(), RuntimeError>` | Clears per-symbol session analytics |

### External ingest supervision

| Method | Returns | Meaning |
| --- | --- | --- |
| `configure_external_feed(policy)` | `Result<(), RuntimeError>` | Enables supervision for external ingest mode |
| `set_external_reconnecting(reconnecting)` | `Result<(), RuntimeError>` | Marks external bridge degraded/reconnecting state |
| `external_health_tick()` | `Result<(), RuntimeError>` | Re-evaluates stale/degraded status without ingest |

### Event processing

| Method | Returns | Meaning |
| --- | --- | --- |
| `ingest_trade(trade, quality_flags)` | `Result<(), RuntimeError>` | Processes one external trade |
| `ingest_book(book, quality_flags)` | `Result<(), RuntimeError>` | Processes one external book update |
| `poll_once(quality_flags)` | `Result<usize, RuntimeError>` | Polls adapter once and processes any ready events |

### Snapshot getters

| Method | Returns | Meaning |
| --- | --- | --- |
| `analytics_snapshot(symbol)` | `Option<AnalyticsSnapshot>` | Base analytics snapshot |
| `derived_analytics_snapshot(symbol)` | `Option<DerivedAnalyticsSnapshot>` | Additive totals snapshot |
| `session_candle_snapshot(symbol)` | `Option<SessionCandleSnapshot>` | Session candle snapshot |
| `interval_candle_snapshot(symbol, window_ns)` | `Option<IntervalCandleSnapshot>` | Rolling-window candle snapshot |
| `book_snapshot(symbol)` | `Option<BookSnapshot>` | Materialized book snapshot |
| `signal_snapshot(symbol)` | `Option<SignalSnapshot>` | Current signal snapshot |
| `signal_explanation_json(symbol)` | `Option<String>` | Latest signal explanation JSON |
| `signal_metrics_json()` | `String` | Signal state and explanation coverage metrics |

### Health, metrics, and diagnostics

| Method | Returns | Meaning |
| --- | --- | --- |
| `adapter_descriptor()` | `AdapterDescriptor` | Static descriptor for configured provider |
| `adapter_status()` | `RuntimeAdapterStatus` | Active adapter descriptor and health |
| `adapter_inventory_json()` | `String` | Adapter descriptor inventory with active marker |
| `active_adapter_status_json()` | `String` | Active adapter status JSON |
| `signal_descriptor_inventory_json()` | `String` | Built-in signal descriptor inventory JSON |
| `metrics_json()` | `String` | Counter-oriented metrics JSON |
| `health_seq()` | `u64` | Monotonic health-change sequence |
| `health_json()` | `String` | Operational health JSON |
| `last_events()` | `&[RawEvent]` | Last processed raw event batch |
| `current_quality_flags_bits()` | `u32` | Current runtime quality bitset |
| `with_max_events_per_poll(max)` | `Engine` | Enables or disables an optional per-poll drain limit |
| `with_circuit_breaker(failure_threshold, cooldown_ms)` | `Engine` | Enables or disables adapter poll circuit breaking |

## Lifecycle Rules

1. Build the engine.
2. Optionally attach persistence.
3. Call `start()`.
4. Use either adapter polling or external ingest.
5. Read snapshots, health, and metrics.
6. Call `stop()` when done.

Important rules:

- `subscribe`, `unsubscribe`, `poll_once`, and external-ingest calls require a
  started engine.
- Snapshot getters return `None` until enough data has been observed for the
  requested symbol.
- `reset_symbol_session` clears session analytics without removing the symbol
  from runtime tracking.

## Snapshot Semantics

- `book_snapshot` appears only after book updates have been seen.
- `derived_analytics_snapshot`, `session_candle_snapshot`, and
  `interval_candle_snapshot` are additive APIs and do not alter the older
  `analytics_snapshot` contract.
- Snapshot getters are side-effect free and return cloned state suitable for
  callers to retain.

## Health and Metrics Contracts

- `health_json()` is the user-facing operational snapshot.
- `metrics_json()` is the counter-focused operational snapshot. It keeps the
  existing flat adapter counters and also includes an additive `adapters` array
  with low-cardinality descriptor and health metadata for dashboards.
- `adapter_inventory_json()` is the build/provider catalog used by dashboards,
  bindings, and CLIs before attempting provider-specific configuration.
- `active_adapter_status_json()` combines static descriptor fields with current
  adapter health, typed operational status, `health_seq`, and circuit-breaker
  state.
- `signal_descriptor_inventory_json()` is the built-in signal catalog used by
  dashboards, config tools, and binding discovery helpers.
- `signal_explanation_json(symbol)` is the latest per-symbol signal
  explanation cache for audit and dashboard paths.
- `signal_metrics_json()` is the aggregate signal diagnostics summary for
  dashboards and bindings.
- JSON field names are treated as stable once published.
- New fields are added additively rather than replacing existing fields.
- Aggregate fields such as `adapter_total_count`, `adapter_healthy_count`, and
  `runtime_health_status` summarize runtime health without replacing the
  adapter-specific fields.
- Circuit-breaker state is exposed as additive `circuit_breaker_*` fields.
- `max_events_per_poll` and `backpressure_dropped_events` are included when
  inspecting runtime health/metrics payloads.

### Active adapter operational fields

The status object and each `metrics_json().adapters[]` entry retain the original
descriptor/health fields and add this stable structured surface:

| Field | Meaning |
| --- | --- |
| `mode` | `mock`, `live`, `replay`, `bridge`, or `unknown` |
| `connection_state` | Disconnected/connecting/streaming/reconnecting/backoff/replay state |
| `endpoint_redacted` | Scheme and authority only, or `null` |
| `app_name` | Non-secret provider application name, or `null` |
| `reconnect_attempt` | Current consecutive reconnect attempt |
| `subscription_count` | Unique active symbol count |
| `subscribed_symbols` | Sorted `{venue, symbol}` records |
| `queue_depth`, `queue_capacity` | Adapter-side buffering utilization |
| `dropped_events`, `gap_count` | Loss/integrity counters |
| `stale` | Provider-specific freshness failure |
| `raw_capture_enabled`, `raw_capture_depth`, `raw_capture_capacity` | Bounded raw-message capture state |
| `last_message_age_ms`, `last_market_data_age_ms` | Optional freshness ages |

Status collection is a control-plane operation. It may clone and sort symbol
ids, but it is never executed by the event hot path. Endpoint values remove
userinfo, path, query, and fragment components to avoid exposing provider
credentials, listen keys, and private stream identifiers. Unknown fields remain
zero or `null`; hosts should use adapter descriptors to distinguish unsupported
capabilities from observed zero activity.

## Backpressure Rules

- Backpressure is disabled by default.
- Set `OF_RUNTIME_MAX_EVENTS_PER_POLL` for default engines, or call
  `with_max_events_per_poll(Some(n))` when constructing an engine directly.
- If an adapter poll drains more than the limit, the runtime processes up to the
  limit, drops the remainder from that drain, sets the `ADAPTER_DEGRADED`
  quality flag, and returns a backpressure error.
- C ABI callers receive `OF_ERR_BACKPRESSURE` for that condition.

## Circuit Breaker Rules

- Circuit breaking is disabled by default.
- Set `OF_RUNTIME_CIRCUIT_BREAKER_FAILURES` for default engines, or call
  `with_circuit_breaker(failure_threshold, cooldown_ms)` when constructing an
  engine directly.
- `OF_RUNTIME_CIRCUIT_BREAKER_COOLDOWN_MS` overrides the default 1000 ms
  cooldown for default engines.
- When consecutive adapter poll failures reach the threshold, later polls during
  the cooldown return a `circuit_open` adapter error and mark the runtime
  degraded.

## Config Loading Rules

- Preferred config shape is typed TOML/JSON with nested `adapter` and
  `adapter.credentials` sections.
- Legacy flat-key config shapes are still accepted.
- Call `load_engine_config_report_from_path` in user-facing CLIs or services if
  you want to surface compatibility warnings.
- `validate_startup_config` enforces endpoint rules, required auth env vars, and
  persistence-retention sanity before startup.

## Persistence Integration

When enabled, the runtime persists normalized events through `of_persist`.

- Persistence does not change runtime snapshot semantics.
- The runtime stores normalized `book` and `trade` streams, not provider-native
  wire payloads.
- Readback and replay consumers should use `of_persist` and `examples/replay_cli`.
