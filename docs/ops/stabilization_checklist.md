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
  - `quality_flags_detail`
  - `last_error`
  - `protocol_info`
  - `tracked_symbols`
  - `processed_events`
  - external supervision fields

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
- Mock mode emits deterministic book + trade flows for end-to-end tests.
- Live mode validates websocket reachability before reporting connected.
- Health reporting integrated into runtime metrics/health stream.
- `protocol_info` includes mode, endpoint, app name, and uptime metadata.

## Binance guarantees

- Crypto market adapter path implemented.
- Subscribe/unsubscribe lifecycle implemented.
- Mock/live endpoint modes supported (`mock://`, `ws://`, `wss://`).
- Health reporting integrated into runtime metrics/health stream.

## CI baseline

- Rust API compatibility is checked in CI for all published crates using `cargo-semver-checks` with patch-level compatibility rules:
  - `of_core`
  - `of_adapters`
  - `of_signals`
  - `of_persist`
  - `of_runtime`
  - `of_ffi_c`
- GitHub Actions matrix added in `.github/workflows/ci.yml` for CQG feature lanes:
  - `cqg`
  - `cqg cqg_proto`
- Python binding syntax validation runs in CI:
  - `python3 -m py_compile bindings/python/orderflow/_ffi.py bindings/python/orderflow/api.py`
- Java binding compile validation runs in CI:
  - `mvn -q -f bindings/java/pom.xml -DskipTests compile`
- Documentation coverage is enforced in CI.

## C ABI export guarantees

- CI verifies that the shared library still exports the documented baseline symbols:
  - `of_api_version`
  - `of_build_info`
  - `of_engine_create`
  - `of_engine_start`
  - `of_engine_stop`
  - `of_engine_destroy`
  - `of_subscribe`
  - `of_unsubscribe`
  - `of_unsubscribe_symbol`
  - `of_reset_symbol_session`
  - `of_ingest_trade`
  - `of_ingest_book`
  - `of_configure_external_feed`
  - `of_external_set_reconnecting`
  - `of_external_health_tick`
  - `of_get_book_snapshot`
  - `of_get_analytics_snapshot`
  - `of_get_derived_analytics_snapshot`
  - `of_get_session_candle_snapshot`
  - `of_get_signal_snapshot`
  - `of_get_metrics_json`
  - `of_string_free`
  - `of_engine_poll_once`
- Export validation runs through `tools/check_ffi_exports.sh` against the built shared library artifact.

## Snapshot compatibility guarantees

- `of_get_book_snapshot(...)` returns materialized JSON once book updates exist for the symbol.
- `OF_STREAM_BOOK_SNAPSHOT` emits the same materialized JSON contract after book changes for the subscribed symbol.
- If a caller buffer is too small for a snapshot payload:
  - the function returns `OF_ERR_INVALID_ARG`
  - `inout_len` is updated with the required byte size
- Python and Java bindings retry with a larger buffer automatically for snapshot retrieval.

## Callback schema guarantees

- `of_event_t.schema_id` remains `1` for all currently shipped payloads.
- Within schema `1`, payload changes are additive-only:
  - existing field names are retained
  - existing field semantics are retained
  - new fields may be appended
- Golden tests pin the current analytics, signal, and health JSON payload contracts at the FFI layer.

## Validation commands

```bash
cargo test -q
cargo test -q -p of_adapters --features rithmic
cargo test -q -p of_adapters --features binance
cargo test -q -p of_adapters --features cqg
cargo test -q -p of_adapters --features "cqg cqg_proto"
cargo build -q -p of_ffi_c
./tools/check_ffi_exports.sh target/debug/libof_ffi_c.so
python3 -m py_compile bindings/python/orderflow/_ffi.py bindings/python/orderflow/api.py
mvn -q -f bindings/java/pom.xml -Dmaven.repo.local=.m2 -DskipTests compile
```
