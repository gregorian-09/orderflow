# Orderflow Engine and Bindings

Orderflow is a multi-language market data and analytics engine that computes
**29 categories of microstructural analytics** across Rust, C, Python, and
Java — from spread metrics and VPIN toxicity through fingerprint patterns,
volatility signatures, Almgren-Chriss impact models, options flow, futures
basis, dark pool siphon detection, and machine-learning-ready LOB features.

## Documentation

Start here:

- **[Strategy Cookbook](./docs/handbook/08-strategy-cookbook.md)** — 30
  exhaustive strategy examples covering every analytics concept across all
  four API layers (Rust, C, Python, Java), plus a full multi-concept live
  trading loop, an API compatibility map, and the `AnalyticsConfig` tuning
  guide.
- [Handbook Home](./docs/handbook/README.md) — primer, architecture, API reference.
- [docs/README.md](./docs/README.md) — full navigation.
- [docs/bindings/README.md](./docs/bindings/README.md) — Python (ctypes) and Java (JNA) setup.

## Analytics at a Glance

| Tier | Concepts | Exposed |
|------|----------|---------|
| T0   | Spread, depth, book events, resiliency | Rust, C, Python, Java |
| T1   | Trade classification, VPIN, Kyle's λ, Amihud, CVD | Rust, C, Python, Java |
| T2   | 19 pattern flags (footprint, DOM, delta, session, volume profile) | Rust, C, Python, Java |
| T3   | Volatility (Parkinson/GK/YZ, signature), noise, Hasbrouck, Almgren-Chriss, spread decomp, ACD, regime | Rust, C, Python, Java |
| T4   | Kinetic energy, agent-type ID, LOB features (16 fields) | Rust, C, Python, Java |
| T5   | Dark pool, dark-lit correlation, institutional flow | Rust, C, Python, Java |
| T6   | Options flow, OI analysis | Rust, C, Python, Java |
| T7   | Futures basis, calendar spread, settlement | Rust, C, Python, Java |

Every analytics type is configurable via the 22-field `AnalyticsConfig` struct
and queryable through the same buffer-negotiation C ABI pattern.

## Quick Build

```bash
cargo build --all-features
cargo test --all-features
```

Build C ABI for bindings, then test FFI exports:

```bash
cargo build -p of_ffi_c --features tickbar
tools/check_ffi_exports.sh
```

## Bindings Quickstart

### Python
```python
from orderflow import Engine, EngineConfig, Symbol

with Engine(EngineConfig()) as engine:
    engine.start()
    engine.subscribe(Symbol("CME", "ESM6", 10))
    engine.poll_once()

    # Read any of the 29 analytics
    print(engine.analytics_snapshot(Symbol("CME", "ESM6", 10)))
    print(engine.pattern_snapshot(Symbol("CME", "ESM6", 10)))
    print(engine.lob_features(Symbol("CME", "ESM6", 10), 0.0, 0.0, 0.0))
```

### Java
```java
try (OrderflowEngine engine = new OrderflowEngine()) {
    engine.start();
    Symbol sym = new Symbol("CME", "ESM6", (short) 10);
    engine.subscribe(sym, StreamKind.ANALYTICS);

    System.out.println(engine.analyticsSnapshot(sym));
    System.out.println(engine.volatilitySnapshot(sym));
}
```

## Tooling

Replay utility:

```bash
cargo run -p replay_cli -- data              # list venues
cargo run -p replay_cli -- data CME          # list symbols
cargo run -p replay_cli -- data CME ESM6 100 200  # replay range
```
