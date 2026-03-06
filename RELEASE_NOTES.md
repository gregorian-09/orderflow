# Release Notes

Date: 2026-02-27
Version: v1 stabilization pass

## Highlights

- Rust core/runtime + C ABI + Python + Java binding stack is fully wired.
- Added explicit unsubscribe path end-to-end:
  - Rust: `MarketDataAdapter::unsubscribe(SymbolId)`
  - Runtime: `Engine::unsubscribe(SymbolId)`
  - C ABI: `of_unsubscribe_symbol(...)`
  - Python: `Engine.unsubscribe(Symbol)`
  - Java: `OrderflowEngine.unsubscribe(Symbol)`
- Health stream is now state-change driven with sequence tracking (`health_seq`).
- CQG adapter improved with:
  - reconnect/resubscribe flows
  - level updates
  - unsubscribe semantics
  - ack correlation hardening (`contract_id` mismatch detection)
  - optional protobuf codec mode (`cqg_proto`)
- Rithmic adapter moved from stub to functional scaffold module with:
  - env/config validation
  - subscribe/unsubscribe lifecycle
  - polling + mock event synthesis
  - health reporting
- Added Binance crypto adapter module with:
  - endpoint validation
  - subscribe/unsubscribe lifecycle
  - polling + mock event synthesis
  - health reporting
- CI matrix added for adapter feature lanes:
  - `rithmic`
  - `cqg`
  - `cqg cqg_proto`
  - `rithmic cqg`

## Validation baseline

```bash
cargo test -q
cargo test -q -p of_adapters --features rithmic
cargo test -q -p of_adapters --features cqg
cargo test -q -p of_adapters --features "cqg cqg_proto"
python3 -m py_compile bindings/python/orderflow/_ffi.py bindings/python/orderflow/api.py
mvn -q -f bindings/java/pom.xml -Dmaven.repo.local=.m2 -DskipTests compile
```

## Notes

- CQG/Rithmic implementations in this repo are functional for current architecture/tests, but production vendor-complete transport/protocol parity still depends on official provider artifacts and environment-specific certification.
