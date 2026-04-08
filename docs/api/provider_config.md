# Provider Config Contracts

This document defines the runtime adapter selection contract used by `of_runtime::build_default_engine`.

## Rust types

- `of_adapters::ProviderKind`
  - `Mock`
  - `Rithmic`
  - `Cqg`
- `of_adapters::CredentialsRef`
  - `key_id_env`: environment variable name for key/user id
  - `secret_env`: environment variable name for secret/password
- `of_adapters::AdapterConfig`
  - `provider`: provider selector
  - `credentials`: optional env-var reference pair
  - `endpoint`: provider endpoint/cluster
  - `app_name`: optional client app id/name

## Runtime wiring

`of_runtime::EngineConfig` includes:

- `instance_id: String`
- `enable_persistence: bool`
- `data_root: String`
- `audit_log_path: String`
- `audit_max_bytes: u64`
- `audit_max_files: u32`
- `audit_redact_tokens: Vec<String>`
- `data_retention_max_bytes: u64`
- `data_retention_max_age_secs: u64`
- `signal_threshold: i64`
- `adapter: AdapterConfig`

`build_default_engine(config)` performs:

1. Startup config validation (`validate_startup_config`)
2. Provider dispatch via `create_adapter(&config.adapter)`
3. Dynamic adapter boxing (`Box<dyn MarketDataAdapter>`)
4. Engine construction with `DeltaMomentumSignal::new(config.signal_threshold)`

## Config loading

Use `of_runtime::load_engine_config_from_path(path)` with `.toml` or `.json` extensions.

Preferred shape:

- top-level runtime keys
- nested `adapter`
- nested `adapter.credentials`

Legacy compatibility:

- older flat keys and section-qualified keys are still accepted
- this compatibility path exists to avoid breaking existing users

Supported keys (modern or legacy):

- `instance_id`
- `enable_persistence`
- `signal_threshold`
- `data_root`
- `audit_log_path`
- `audit_max_bytes`
- `audit_max_files`
- `audit_redact_tokens` (comma-separated)
- `data_retention_max_bytes`
- `data_retention_max_age_secs`
- `provider` or `adapter.provider`
- `endpoint` or `adapter.endpoint`
- `app_name` or `adapter.app_name`
- `credentials_key_id_env` or `credentials.key_id_env` or `adapter.credentials.key_id_env`
- `credentials_secret_env` or `credentials.secret_env` or `adapter.credentials.secret_env`

### TOML example

```toml
instance_id = "of-prod"
enable_persistence = true
signal_threshold = 150
data_root = "data"
audit_log_path = "audit/orderflow_audit.log"
audit_max_bytes = 10485760
audit_max_files = 5
audit_redact_tokens = ["secret", "password", "token", "api_key"]
data_retention_max_bytes = 10485760
data_retention_max_age_secs = 604800

[adapter]
provider = "mock"
```

### JSON example

```json
{
  "instance_id": "of-prod",
  "enable_persistence": true,
  "signal_threshold": 150,
  "provider": "mock",
  "data_root": "data",
  "audit_log_path": "audit/orderflow_audit.log",
  "audit_max_bytes": 10485760,
  "audit_max_files": 5,
  "audit_redact_tokens": ["secret", "password", "token", "api_key"],
  "data_retention_max_bytes": 10485760,
  "data_retention_max_age_secs": 604800,
  "adapter": {
    "provider": "mock"
  }
}
```

### Nested provider example

```toml
instance_id = "of-live"
signal_threshold = 200

[adapter]
provider = "cqg"
endpoint = "wss://demoapi.cqg.com/feed"
app_name = "orderflow"

[adapter.credentials]
key_id_env = "CQG_USER"
secret_env = "CQG_PASS"
```

## Startup validation

For non-mock providers:

- `adapter.endpoint` must be set and non-empty.
- Credential env-var references must be provided (`key_id_env`, `secret_env`).
- Referenced environment variables must exist and be non-empty.

General validation:

- `instance_id` must be non-empty.
- `signal_threshold` must be greater than zero.
- `audit_log_path` must be non-empty.
- `audit_max_bytes` must be greater than zero.
- `audit_max_files` must be less than or equal to 1000.
- If `enable_persistence=true`, `data_root` must be non-empty.
- If `enable_persistence=true`, set at least one of:
  - `data_retention_max_bytes > 0`
  - `data_retention_max_age_secs > 0`

## C API behavior

`of_engine_create` now reads `config_path` when provided.

- If `config_path` is set, runtime config is loaded from file.
- If `instance_id` is also set, it overrides the file value.
- `enable_persistence` from C config overrides file value.
- If non-zero/set, these C fields also override file/default values:
  - `audit_max_bytes`
  - `audit_max_files`
  - `audit_redact_tokens_csv`
  - `data_retention_max_bytes`
  - `data_retention_max_age_secs`
- If no `config_path` is set, defaults are used (`mock` provider).

## Feature flags

Provider implementations are compile-time gated in `of_adapters`:

- `rithmic`
- `cqg`
- `binance`

Behavior:

- If a provider is selected but feature is not enabled, returns `AdapterError::FeatureDisabled`.
- If feature is enabled but required fields are missing, returns `AdapterError::NotConfigured`.

## Current status

- Mock provider: functional.
- CQG provider: implemented adapter flow with reconnect/resubscribe, level updates, explicit unsubscribe, and optional `cqg_proto` codec mode.
- Rithmic provider: implemented adapter flow with credential validation, mock/live endpoint modes (`mock://`, `ws://`, `wss://`), subscribe/unsubscribe, websocket reachability checks for live connect, deterministic mock book/trade emission, and richer health reporting.
- Binance provider (crypto): implemented adapter flow with endpoint validation, real websocket execution (`ws://`, `wss://`) including subscribe/unsubscribe command flow, ping/pong handling, live `aggTrade` + `depthUpdate` parsing, mock mode (`mock://`), and health reporting.
- Audit logging: enabled by default; writes lifecycle/subscription/quality-block events.
- Audit rotation: rotates at `audit_max_bytes`, keeps up to `audit_max_files` archives (`.1`, `.2`, ...).
- Audit redaction: replaces configured sensitive tokens in log details with `[REDACTED]`.
- Persistence: optional rolling store for normalized book/trade events.
- Persistence retention: prunes by age and/or total-byte budget after appends.
