# `of_execution` Reference

`of_execution` is the execution routing and OMS crate. It builds on
`of_execution_core` by adding adapter contracts, route configuration, risk
context construction, bounded event buffers, simulated execution, journaling,
concurrent worker ownership, and reusable OMS primitives.

The crate has two execution layers:

- `ExecutionEngine`: synchronous deterministic engine.
- `ConcurrentExecutionEngine`: worker-thread owner around `ExecutionEngine`.

The synchronous engine is the canonical state machine. The concurrent engine is
an additive producer/worker wrapper.

## Core Traits

### `ExecutionAdapter`

Adapters implement:

| Method | Purpose |
| --- | --- |
| `connect()` | Establish transport/session |
| `submit(req, out)` | Send new order and write generated events |
| `cancel(req, out)` | Send cancel and write generated events |
| `amend(req, out)` | Send replace and write generated events |
| `poll(out)` | Drain async venue reports |
| `recover_open_orders(out)` | Emit recovery/restatement reports |
| `capabilities()` | Report order-type/TIF support |
| `health()` | Report adapter health |

The adapter writes events into caller-owned `ExecutionEventBuffer`. It should
not allocate per report on the hot path unless the provider protocol requires
it.

### `ExecutionJournal`

Journals implement:

- `record_command`
- `record_event`
- `replay`

`InMemoryJournal` is used for tests and embedded simulation. `FileExecutionJournal`
in the OMS helper layer provides an append-only text file implementation.
`WalExecutionJournal` provides an additive binary WAL implementation backed by
`of_execution_core` WAL frames, sequence numbers, checksums, and configurable
sync policy.

| Journal | Use |
| --- | --- |
| `InMemoryJournal` | Tests, simulation, volatile embedded hosts |
| `FileExecutionJournal` | Human-readable append-only text journal |
| `WalExecutionJournal` | Binary append-only WAL with integrity validation |

`WalExecutionJournal::open(WalJournalConfig::new(path))` validates existing WAL
bytes before accepting new records. It fails closed on corrupt frames or
non-contiguous sequences. `replay_from(WalSequence, out)` supports bounded
startup replay once checkpoints are added later.

## Route Configuration

`RouteConfig` binds:

- `route_id`
- `account_id`
- `symbol`
- `enabled`
- `risk_limits`

The engine builds an internal `HashMap<RouteKey, usize>` for constant-time
lookup. The key is `(route_id, account_id, symbol)`.

This is how one engine can handle many symbols while keeping risk limits scoped
to the exact route/account/symbol.

## `ExecutionEventBuffer`

`ExecutionEventBuffer` is a bounded vector used by adapters and engine calls.
It enforces capacity with `ExecutionError::BufferFull`.

Important behaviors:

- `clear()` retains allocation.
- `push()` fails when capacity is reached.
- `drain_into()` moves events into another bounded buffer.
- FFI uses the same bounded-copy semantics through caller-owned arrays.

## Synchronous Engine

`ExecutionEngine<A, R, J>` owns:

- adapter `A`,
- risk gate `R`,
- journal `J`,
- route table,
- order state machines,
- order price cache for open-notional accounting,
- metrics,
- scratch event buffer.

### Lifecycle

1. Construct with `ExecutionEngine::new(adapter, risk, journal, routes)`.
2. Call `start()`.
3. Call `submit`, `cancel`, `amend`, `poll`, or `recover_open_orders`.
4. Read `order_state`, `metrics`, `health`, or `replay_journal`.

### Submit Path

Submit does the following in order:

1. rejects if not started,
2. validates the request shape,
3. finds the configured route,
4. builds route-scoped `RiskContext`,
5. checks route risk limits,
6. checks custom risk gate,
7. records command in journal,
8. creates local pending state,
9. calls adapter,
10. applies returned events,
11. records events in journal,
12. copies events to caller buffer.

This order is important. The adapter never sees a request that failed local
validation or pre-trade risk.

### Cancel and Amend Paths

Cancel and amend require known local state for `orig_client_order_id`. Amend
also performs route-scoped size/notional checks and then custom risk checks.

### Route-Scoped Risk

Open order count and open notional are computed only for the matched
route/account/symbol. A partially filled ES order does not consume the NQ open
order budget unless both orders share the exact same execution symbol.

## Simulated Execution

`SimExecutionAdapter` is deterministic and intended for integration testing,
binding smoke tests, strategy validation, and examples.

Helpers:

- `simulated_engine(route)`
- `simulated_engine_with_routes(routes)`

The single-route helper preserves the original return type and risk behavior.
The multi-route helper uses route-scoped limits and `AllowAllRiskGate`, because
route limits are enforced directly by the engine.

## Concurrent Worker

`ConcurrentExecutionEngine` owns a synchronous engine on one worker thread.
It exposes:

- cloneable `ExecutionCommandSender`,
- bounded command queue,
- bounded report queue,
- non-blocking `try_send`,
- blocking `send`,
- report receive methods,
- graceful stop and join.

Command kinds:

- `Submit`
- `Cancel`
- `Amend`
- `Poll`
- `RecoverOpenOrders`
- `Stop`

The worker preserves deterministic order-state mutation. Producers can be
concurrent, but the engine remains single-owner.

## OMS Helper Surface

The `oms` module is re-exported from `of_execution`.

| Area | Types / Functions |
| --- | --- |
| Correlation | `CommandId`, `RequestId`, `CommandIdGenerator`, `CommandCorrelation` |
| Event fanout | `ExecutionEventFanout`, `ExecutionEventSubscriber` |
| Lifecycle | `ExecutionLifecycle`, `ExecutionAdapterState`, `ExecutionLifecycleSnapshot` |
| Durable journal | `FileExecutionJournal` |
| Reconciliation | `reconcile_open_orders`, `ReconciliationReport` |
| Safety policies | `DisconnectPolicy`, `RouteSafetyPolicy` |
| Advanced risk | `AdvancedRiskLimits`, `AdvancedRiskGate` |
| Ledger | `PositionLedger`, `Position`, `PositionKey` |
| Normalization | `VenueOrderCapabilities`, `normalize_order_type` |
| Telemetry | `ExecutionTelemetry` |
| Sharding | `ShardKey`, `ShardRouter` |
| Throttling | `OrderThrottle` |
| Replay | `ReplayDecision`, `ReplayResult`, `replay_simulated_oms` |
| Adapter SDK | `ProviderAdapterContext`, `ExecutionAdapterFactory`, `ProviderAdapterSdk` |

## Error Model

`ExecutionError` includes:

- `Disconnected`
- `BufferFull`
- `RouteNotFound`
- `RiskRejected`
- `Core`
- `Adapter`
- `Journal`

Concurrent wrapper errors are separated as `ConcurrentExecutionError`, mapping
queue backpressure, stopped workers, worker panic, and underlying execution
errors.

## When To Use Which API

Use `ExecutionEngine` when:

- you are in Rust,
- one owner thread already exists,
- deterministic synchronous control is desired,
- you are writing tests or a replay harness.

Use `ConcurrentExecutionEngine` when:

- multiple producer threads submit commands,
- you need explicit queue backpressure,
- you want one thread to own the adapter and state machine,
- you do not want a Tokio dependency.

Use C/Python/Java concurrent bindings when:

- host languages need non-blocking command queueing,
- host code should poll command reports,
- the native worker should preserve deterministic state internally.
