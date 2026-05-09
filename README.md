# Orderflow Engine and Bindings

Orderflow is a multi-language market data and analytics engine with:

- Rust core/runtime crates
- C ABI
- Python binding (`ctypes`)
- Java binding (JNA)
- Dashboard and tooling for live/replay workflows

## Documentation

Start with:

- [Release 0.3.0 notes](./docs/ops/release-0.3.0.md)
- [docs/README.md](./docs/README.md)
- [docs/handbook/README.md](./docs/handbook/README.md)
- [docs/bindings/README.md](./docs/bindings/README.md)

## What's New In 0.3.0

`0.3.0` is a non-breaking operational hardening release:

- optional dashboard auth with `OF_DASH_TOKEN`
- Prometheus `/metrics` for dashboard/runtime monitoring
- opt-in runtime backpressure and adapter circuit breaker policy
- additive aggregate health fields in runtime JSON
- versioned JSONL persistence records with event timestamps
- persist -> replay -> signal/book parity regression coverage
- Python `py.typed` marker and bundled native-wheel loading support

## Quick Build

```bash
cargo build
cargo test
```

Build C ABI for bindings:

```bash
cargo build -p of_ffi_c
```

## Tooling

Replay utility example:

```bash
cargo run -p replay_cli -- data
cargo run -p replay_cli -- data CME
cargo run -p replay_cli -- data CME ESM6 100 200
```

The replay CLI now supports discovery-first workflows:

- list venues under a persistence root
- list symbols for a venue
- inspect available streams for a symbol
- print merged replay events with optional inclusive sequence bounds
