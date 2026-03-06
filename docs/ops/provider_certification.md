# Provider Certification Runbook

Use this runbook to validate live-provider readiness in a real vendor environment.

## Scope

- CQG and Rithmic connectivity/callback conformance at runtime/binding layer.
- Health + analytics stream behavior under live credentials.

## Prerequisites

- Valid provider credentials in environment variables referenced by config.
- Reachable provider endpoint in config file (`.toml` or `.json`).
- Built shared library + Python binding import path available.

### Credential handling standard

Production standard:

- Keep secrets out of repo and config files.
- Store credentials in a secret manager (for example: Vault, AWS Secrets Manager, Kubernetes Secrets, CI secret store).
- Inject credentials as process environment variables at runtime.
- In config, reference only env var names (`credentials_key_id_env`, `credentials_secret_env`).

Local/dev certification:

- A `.env` file is acceptable for local runs only.
- Keep `.env` uncommitted.
- Use [`docs/ops/examples/.env.example`](./examples/.env.example) as the template.

Load local `.env` into your shell before running the harness:

```bash
set -a
source docs/ops/examples/.env
set +a
```

Build the C ABI with provider features before running harness:

```bash
cargo build -p of_ffi_c --features "binance rithmic cqg"
```

For Binance-only testing:

```bash
cargo build -p of_ffi_c --features "binance"
```

## Conformance harness

Run:

```bash
python3 tools/provider_conformance.py \
  --provider cqg \
  --config-path /path/to/live_config.toml \
  --venue CME \
  --symbol ESM6 \
  --duration 30
```

For Rithmic:

```bash
python3 tools/provider_conformance.py \
  --provider rithmic \
  --config-path /path/to/live_config.toml \
  --venue CME \
  --symbol ESM6 \
  --duration 30
```

For Binance (crypto):

```bash
python3 tools/provider_conformance.py \
  --provider binance \
  --config-path docs/ops/examples/binance_conformance.toml \
  --venue BINANCE \
  --symbol BTCUSDT \
  --duration 30
```

For live Binance WebSocket execution:

```bash
python3 tools/provider_conformance.py \
  --provider binance \
  --config-path docs/ops/examples/binance_live.toml \
  --venue BINANCE \
  --symbol BTCUSDT \
  --duration 30
```

Live note:
- The Binance adapter now opens a real websocket session to `wss://stream.binance.com:9443/ws`,
  sends `SUBSCRIBE`/`UNSUBSCRIBE` commands for `@aggTrade` and `@depth@100ms`,
  handles ping/pong, and emits both trade and depth raw events.

## Pass criteria

- `ok=true` in report output.
- `health_events > 0`
- `analytics_events > 0`
- `adapter_connected=true` in metrics point.
- No persistent `degraded=true`/`last_error` patterns.

## Notes

- A failure here is usually environment-level (credentials, endpoint ACL, vendor account permissions).
- Keep output JSON as certification evidence.
