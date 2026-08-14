# Provider Certification Runbook

Use this runbook to validate live-provider readiness in a real vendor environment.

## Scope

- CQG and Rithmic connectivity/callback conformance at runtime/binding layer.
- Health + analytics stream behavior under live credentials.

## Prerequisites

- Valid provider credentials in environment variables referenced by config.
- Reachable provider endpoint in config file (`.toml` or `.json`).
- Built shared library + Python binding import path available.
- For private trust roots or mTLS, readable PEM files and the TLS environment
  variables described below.

### Credential handling standard

Production standard:

- Keep secrets out of repo and config files.
- Store credentials in a secret manager (for example: Vault, AWS Secrets Manager, Kubernetes Secrets, CI secret store).
- Inject credentials as process environment variables at runtime.
- In config, reference only env var names (`credentials_key_id_env`, `credentials_secret_env`).

Local/dev certification:

- A `.env` file is acceptable for local runs only.
- Keep `.env` uncommitted.
- Use the [`.env.example`](https://github.com/gregorian-09/orderflow/blob/main/docs/ops/examples/.env.example) template.

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

### Custom trust roots and mTLS

The live WebSocket adapters use OpenSSL for the connection boundary. They
verify the server certificate against the system trust store by default and
verify the endpoint hostname. `ORDERFLOW_*_TLS_CA_FILE` selects a supplied PEM
trust bundle when a venue uses a private CA. A venue that requires a client
certificate can provide mTLS files without putting secret material in runtime
configuration:

```bash
export ORDERFLOW_CQG_TLS_CA_FILE=/run/secrets/cqg-ca.pem
export ORDERFLOW_CQG_TLS_CLIENT_CERT_FILE=/run/secrets/cqg-client.pem
export ORDERFLOW_CQG_TLS_CLIENT_CHAIN_FILE=/run/secrets/cqg-chain.pem
export ORDERFLOW_CQG_TLS_CLIENT_KEY_FILE=/run/secrets/cqg-client-key.pem
export ORDERFLOW_CQG_TLS_CLIENT_KEY_PASSWORD_ENV=CQG_TLS_KEY_PASSWORD
export CQG_TLS_KEY_PASSWORD='provided-by-your-secret-manager'
```

Use `BINANCE_TLS` or `RITHMIC_TLS` for those providers. The generic
`ORDERFLOW_TLS_*` names are fallback settings; provider-specific names take
precedence and are preferable when one process owns multiple venue sessions.
`CLIENT_CERT_FILE` and `CLIENT_KEY_FILE` must be supplied together. All paths
must point to readable regular files; OpenSSL performs PEM and chain
validation after the path checks. The key password is looked up by name and is
passed to OpenSSL through an environment reference, so it is not exposed in
process arguments or logs.

This configuration is applied during connection setup only. It does not add
per-event work to the market-data polling path. Do not disable hostname
verification, use `-verify` bypasses, or replace a venue-specific CA with an
unreviewed system-wide bundle.

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

Rithmic live note:
- The adapter now validates websocket reachability during `connect()`.
- Mock mode emits deterministic book and trade events for local integration tests.
- Live certification still requires vendor-specific endpoint and credential validation in a real environment.

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
