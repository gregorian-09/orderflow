# `of_runtime` Reference

`of_runtime` is the orchestration layer that connects adapters, analytics,
signals, book state, persistence, health reporting, and external ingest flows.

## Public API Map

| Item | Kind | Purpose |
| --- | --- | --- |
| `EngineConfig` | struct | Runtime control-plane configuration |
| `RuntimeError` | enum | Runtime error contract |
| `ExternalFeedPolicy` | struct | External ingest supervision rules |
| `Engine<A, S>` | struct | Generic runtime |
| `DefaultEngine` | type alias | Runtime with boxed adapter and default signal |
| `ConfigCompatibilityMode` | enum | Config loader compatibility state |
| `ConfigLoadReport` | struct | Detailed config loading result |
| `build_default_engine` | fn | Convenience constructor |
| `load_engine_config_from_path` | fn | Loads config only |
| `load_engine_config_report_from_path` | fn | Loads config plus compatibility report |
| `validate_startup_config` | fn | Validates config and env prerequisites |

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

### Health, metrics, and diagnostics

| Method | Returns | Meaning |
| --- | --- | --- |
| `metrics_json()` | `String` | Counter-oriented metrics JSON |
| `health_seq()` | `u64` | Monotonic health-change sequence |
| `health_json()` | `String` | Operational health JSON |
| `last_events()` | `&[RawEvent]` | Last processed raw event batch |
| `current_quality_flags_bits()` | `u32` | Current runtime quality bitset |

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
- `metrics_json()` is the counter-focused operational snapshot.
- JSON field names are treated as stable once published.
- New fields are added additively rather than replacing existing fields.

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
