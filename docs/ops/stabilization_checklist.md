# Stabilization Checklist

Last updated: 2026-04-08

## API freeze scope (current)

- Rust runtime + adapters:
  - `MarketDataAdapter::subscribe(...)`
  - `MarketDataAdapter::unsubscribe(SymbolId)`
  - `Engine::subscribe(...)`
  - `Engine::unsubscribe(...)`
- C ABI:
  - `of_subscribe(...)`
  - `of_unsubscribe(...)`
  - `of_unsubscribe_symbol(...)`
  - `of_engine_poll_once(...)`
  - `of_get_metrics_json(...)`
- Bindings:
  - Python `Engine.subscribe(...)`, `Engine.unsubscribe(...)`
  - Java `OrderflowEngine.subscribe(...)`, `OrderflowEngine.unsubscribe(...)`

## Health stream guarantees

- `OF_STREAM_HEALTH` is emitted on health-state transitions only.
- Health payload includes:
  - `health_seq`
  - `started`, `connected`, `degraded`
  - `reconnect_state`
  - `quality_flags`
  - `last_error`
  - `protocol_info`

## CQG guarantees (scaffold)

- Reconnect + resubscribe flow implemented.
- Depth level change supported without re-resolution when contract is known.
- Unsubscribe semantics supported via depth level `0` and explicit runtime unsubscribe path.
- Subscription ack correlation validates expected `contract_id`; mismatches mark adapter degraded and increment mismatch metrics.
- Feature parity test lanes:
  - `--features cqg`
  - `--features "cqg cqg_proto"`

## Rithmic guarantees

- Adapter config validation + env credential resolution implemented.
- Subscribe/unsubscribe lifecycle implemented.
- Mock/live endpoint modes supported at config boundary (`mock://`, `ws://`, `wss://`).
- Health reporting integrated into runtime metrics/health stream.

## Binance guarantees

- Crypto market adapter path implemented.
- Subscribe/unsubscribe lifecycle implemented.
- Mock/live endpoint modes supported (`mock://`, `ws://`, `wss://`).
- Health reporting integrated into runtime metrics/health stream.

## CI baseline

- GitHub Actions matrix added in `.github/workflows/ci.yml` for CQG feature lanes:
  - `cqg`
  - `cqg cqg_proto`
- Python binding syntax validation runs in CI:
  - `python3 -m py_compile bindings/python/orderflow/_ffi.py bindings/python/orderflow/api.py`
- Java binding compile validation runs in CI:
  - `mvn -q -f bindings/java/pom.xml -DskipTests compile`
- Documentation coverage is enforced in CI.

## Snapshot compatibility guarantees

- `of_get_book_snapshot(...)` returns materialized JSON once book updates exist for the symbol.
- If a caller buffer is too small for a snapshot payload:
  - the function returns `OF_ERR_INVALID_ARG`
  - `inout_len` is updated with the required byte size
- Python and Java bindings retry with a larger buffer automatically for snapshot retrieval.

## Validation commands

```bash
cargo test -q
cargo test -q -p of_adapters --features rithmic
cargo test -q -p of_adapters --features binance
cargo test -q -p of_adapters --features cqg
cargo test -q -p of_adapters --features "cqg cqg_proto"
python3 -m py_compile bindings/python/orderflow/_ffi.py bindings/python/orderflow/api.py
mvn -q -f bindings/java/pom.xml -Dmaven.repo.local=.m2 -DskipTests compile
```
