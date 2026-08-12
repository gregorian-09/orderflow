# Contributor Guide

This guide explains how to build, test, and extend the system.

## Repository Layout

- `crates/of_core`: canonical types and analytics math.
- `crates/of_analytics`: optional advanced analytics and feature extraction.
- `crates/of_signals`: signal trait and default signal module.
- `crates/of_adapters`: provider interface + mock/rithmic/cqg/binance adapters.
- `crates/of_persist`: JSONL, normalized/raw WAL, recovery, and retention.
- `crates/of_persist_parquet`: verified columnar cold export outside the hot path.
- `crates/of_runtime`: orchestration, quality supervision, metrics/health.
- `crates/of_execution_core`: canonical order and execution domain types.
- `crates/of_execution`: OMS engine, risk, recovery, journals, and concurrency.
- `crates/of_execution_algos`: deterministic parent/child execution planners.
- `crates/of_fix`: reusable FIX codec and session infrastructure.
- `crates/of_execution_adapters`: provider execution bridges and certification.
- `crates/of_ffi_c`: C ABI and callback bridge.
- `bindings/python`: ctypes binding.
- `bindings/java`: JNA binding.
- `dashboard`: live UI and backend state endpoints.
- `tools`: smoke/conformance/capture utilities.

## Build and Test

From workspace root:

```bash
cargo build
cargo test
```

C ABI only:

```bash
cargo build -p of_ffi_c
```

Python binding:

```bash
python -m pip install -e bindings/python
python bindings/python/examples/basic.py
```

Java binding:

```bash
mvn -q -f bindings/java/pom.xml package
mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.BasicExample
```

Dashboard smoke:

```bash
python3 tools/dashboard_smoke_test.py
```

Provider conformance:

```bash
python3 tools/provider_conformance.py --help
```

## Extension Pattern: Add a New Adapter

1. Add provider variant in `of_adapters::ProviderKind`.
2. Extend `AdapterConfig` handling and factory dispatch in `create_adapter`.
3. Implement `MarketDataAdapter`:
   - `connect`
   - `subscribe`
   - `unsubscribe`
   - `poll`
   - `health`
4. Normalize provider payloads into `RawEvent::{Book, Trade}`.
5. Add feature flag wiring in `crates/of_adapters/Cargo.toml` and `crates/of_ffi_c/Cargo.toml`.
6. Add config validation requirements in `of_runtime::validate_startup_config` if needed.
7. Add conformance tests and docs updates.

### Detailed Adapter Skeleton

```rust
use of_adapters::{
    AdapterError, AdapterHealth, AdapterResult, MarketDataAdapter, RawEvent, SubscribeReq,
};
use of_core::{BookAction, BookUpdate, Side, SymbolId, TradePrint};

#[derive(Default)]
struct BridgeAdapter {
    connected: bool,
    subscriptions: Vec<SubscribeReq>,
    pending: Vec<RawEvent>,
}

impl BridgeAdapter {
    fn push_trade_from_sdk(&mut self, symbol: SymbolId, price: i64, size: i64, seq: u64) {
        self.pending.push(RawEvent::Trade(TradePrint {
            symbol,
            price,
            size,
            aggressor_side: Side::Ask,
            sequence: seq,
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }));
    }

    fn push_book_from_sdk(&mut self, symbol: SymbolId, level: u16, price: i64, size: i64, seq: u64) {
        self.pending.push(RawEvent::Book(BookUpdate {
            symbol,
            side: Side::Bid,
            level,
            price,
            size,
            action: BookAction::Upsert,
            sequence: seq,
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }));
    }
}

impl MarketDataAdapter for BridgeAdapter {
    fn connect(&mut self) -> AdapterResult<()> {
        self.connected = true;
        Ok(())
    }

    fn subscribe(&mut self, req: SubscribeReq) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        self.subscriptions.push(req);
        Ok(())
    }

    fn unsubscribe(&mut self, symbol: SymbolId) -> AdapterResult<()> {
        self.subscriptions.retain(|req| req.symbol != symbol);
        Ok(())
    }

    fn poll(&mut self, out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        let n = self.pending.len();
        out.extend(self.pending.drain(..));
        Ok(n)
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth {
            connected: self.connected,
            degraded: false,
            last_error: None,
            protocol_info: Some(format!("subscriptions={}", self.subscriptions.len())),
        }
    }
}
```

### Adapter Authoring Rules

- normalize before leaving the adapter boundary
- preserve timestamps and sequences whenever the provider exposes them
- use `degraded` to reflect reconnecting or stale transport states
- keep provider-native error handling out of runtime and strategy modules
- test `poll()` behavior with empty, single-event, and burst-event cases

## Extension Pattern: Add a New Signal Module

1. Implement `SignalModule` in `of_signals`.
2. Define quality-gate policy for degraded feed states.
3. Add deterministic tests for:
   - normal signal transitions
   - blocked state under quality flags
4. Wire into runtime construction where required.

## Extension Pattern: Add Binding Features

General rule:

- Add functionality in Rust runtime + C ABI first.
- Then expose in Python and Java wrappers.

Binding checklist:

1. Update C header (`orderflow.h`).
2. Update `of_ffi_c` implementation and tests.
3. Update Python `_ffi.py` signatures + high-level API.
4. Update Java `OrderflowNative` signatures + `OrderflowEngine`.
5. Add/refresh examples and README snippets.

Native ABI test rules:

- serialize tests that touch shared native callback/runtime state through the
  crate-level FFI test guard;
- recover the guard from mutex poisoning so one failing test does not hide the
  original failure behind a cascade of lock errors;
- use bounded deadlines for worker-thread or channel-drain assertions instead
  of yield-only spins;
- keep timeout windows short enough for CI feedback but long enough to survive
  scheduler contention under parallel workspace tests.

## Code Quality Expectations

- Keep adapter normalization deterministic and explicit.
- Avoid hidden conversions of price/size units.
- Preserve sequence and timestamp metadata.
- Use quality flags to fail safe, not fail open.
- Prefer additive API evolution to avoid breaking bindings.

## Current Technical Notes

- Book snapshot API returns a materialized `bids`/`asks` snapshot once book updates have been processed.
- Analytics/signal snapshots are fully implemented.
- Health stream is edge-triggered (`health_seq` change required).
- External ingest supports continuous updates through repeated `ingest_trade`/`ingest_book`.
