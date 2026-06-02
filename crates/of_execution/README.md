# `of_execution`

`of_execution` is the execution routing and OMS crate for Orderflow. It builds
on `of_execution_core` by adding adapter contracts, route configuration,
bounded event buffers, simulated execution, journals, route-scoped risk,
concurrent command ownership, and reusable order-management helpers.

The crate is additive to the analytics runtime. It does not change market-data
adapter behavior and does not merge execution state into `of_runtime`.
Strategies can read analytics from `of_runtime` and send orders through
`of_execution`, while those two domains stay separate.

## Design Goals

- Keep the synchronous engine deterministic and single-owner.
- Preserve route/account/symbol scoped risk accounting.
- Use typed requests and reports on the hot path.
- Use caller-owned bounded event buffers.
- Surface backpressure explicitly instead of hiding it in unbounded queues.
- Keep adapter contracts provider-neutral.
- Provide a concurrent worker without forcing a Tokio/async runtime.
- Make journaling, recovery, and reconciliation additive and replaceable.

## Public API Inventory

Core result and error types:

- [`ExecutionResult<T>`]
- [`ExecutionError`]

Event buffers and adapter metadata:

- [`ExecutionEventBuffer`]
- [`LatencyClass`]
- [`ExecutionCapabilities`]
- [`ExecutionHealth`]
- [`ExecutionAdapter`]

Routing and risk:

- [`RouteConfig`]
- [`RouteKey`]
- [`AllowAllRiskGate`]

Journaling:

- [`JournalCommandKind`]
- [`JournalRecord`]
- [`ExecutionJournal`]
- [`InMemoryJournal`]

Engine and simulation:

- [`ExecutionEngine`]
- [`ExecutionMetrics`]
- [`SimExecutionAdapter`]
- [`simulated_engine`]
- [`simulated_engine_with_routes`]

Concurrent execution:

- [`ConcurrentExecutionConfig`]
- [`ExecutionCommandKind`]
- [`ExecutionCommand`]
- [`ExecutionCommandReport`]
- [`ConcurrentExecutionError`]
- [`ExecutionCommandSender`]
- [`ConcurrentExecutionEngine`]

OMS helpers:

- [`CommandId`]
- [`RequestId`]
- [`CommandIdGenerator`]
- [`CommandCorrelation`]
- [`ExecutionEventFanout`]
- [`ExecutionEventSubscriber`]
- [`ExecutionAdapterState`]
- [`ExecutionLifecycle`]
- [`ExecutionLifecycleSnapshot`]
- [`FileExecutionJournal`]
- [`ReconciliationAction`]
- [`ReconciliationItem`]
- [`ReconciliationReport`]
- [`reconcile_open_orders`]
- [`DisconnectPolicy`]
- [`RouteSafetyPolicy`]
- [`AdvancedRiskLimits`]
- [`AdvancedRiskGate`]
- [`Position`]
- [`PositionKey`]
- [`PositionLedger`]
- [`VenueOrderCapabilities`]
- [`NormalizedOrderType`]
- [`normalize_order_type`]
- [`ExecutionTelemetry`]
- [`ShardKey`]
- [`ShardRouter`]
- [`OrderThrottle`]
- [`ReplayDecision`]
- [`ReplayResult`]
- [`replay_simulated_oms`]
- [`ProviderAdapterContext`]
- [`ExecutionAdapterFactory`]
- [`ProviderAdapterSdk`]

## Layer Model

```mermaid
flowchart TD
  Strategy[Strategy / host language]
  Engine[ExecutionEngine<br/>or ConcurrentExecutionEngine]
  Route[RouteConfig / RouteKey]
  Risk[RiskLimits / RiskCheck]
  Journal[ExecutionJournal]
  State[OrderStateMachine]
  Adapter[ExecutionAdapter]
  Venue[Venue, broker, or simulator]

  Strategy --> Engine
  Engine --> Route
  Engine --> Risk
  Engine --> Journal
  Engine --> State
  Engine --> Adapter --> Venue
```

The synchronous [`ExecutionEngine`] is the canonical state owner. The
[`ConcurrentExecutionEngine`] is a wrapper that lets many producer threads
submit commands while one worker thread owns the synchronous engine.

## Adapter Contract

Implement [`ExecutionAdapter`] to connect a venue, broker, simulator, REST API,
WebSocket API, FIX session, or native SDK.

Required methods:

- `connect()`
- `submit(req, out)`
- `cancel(req, out)`
- `amend(req, out)`
- `poll(out)`
- `recover_open_orders(out)`
- `capabilities()`
- `health()`

Adapters write canonical `ExecutionEvent` values into caller-owned
[`ExecutionEventBuffer`] instances. They should not leak provider-specific
report structs past the adapter boundary.

## ExecutionEventBuffer

[`ExecutionEventBuffer`] is a bounded event vector used by adapters and engine
calls.

Important behavior:

- `with_capacity(capacity)` allocates bounded storage.
- `push(event)` fails with [`ExecutionError::BufferFull`] when full.
- `clear()` retains capacity for reuse.
- `as_slice()` and `as_mut_slice()` expose currently stored events.
- `drain_into(out)` moves events into another bounded buffer.
- `max_len()` returns configured capacity.

This buffer model matters for low latency and FFI. The caller controls memory
growth and the engine never silently drops order events.

## Route Configuration

[`RouteConfig`] binds:

- `route_id`
- `account_id`
- `symbol`
- `enabled`
- `risk_limits`

The engine indexes routes by [`RouteKey`]:

```mermaid
flowchart LR
  RouteId[route_id]
  AccountId[account_id]
  Symbol[execution symbol]
  Key[RouteKey]

  RouteId --> Key
  AccountId --> Key
  Symbol --> Key
```

Open-order count and open notional are calculated only within the matched
route/account/symbol. This allows one engine to handle multiple symbols without
cross-symbol contamination.

Example:

- ES route: max one open order.
- NQ route: max one open order.
- A second ES order is rejected.
- A first NQ order is accepted.

## Synchronous Engine

[`ExecutionEngine<A, R, J>`] owns:

- adapter `A`
- risk gate `R`
- journal `J`
- route table
- order-state machines
- open-order price cache
- metrics
- scratch event buffer

Lifecycle:

1. Build the engine with `ExecutionEngine::new(adapter, risk, journal, routes)`.
2. Call `start()`.
3. Call `submit`, `cancel`, `amend`, `poll`, or `recover_open_orders`.
4. Inspect `order_state`, `metrics`, `health`, `routes`, or `replay_journal`.

Submit path:

1. reject if the engine is not started,
2. validate the request shape,
3. find the configured route,
4. build route-scoped risk context,
5. check route risk limits,
6. check custom risk gate,
7. record the command in the journal,
8. create local pending state,
9. call the adapter,
10. apply returned events through the state machine,
11. record events in the journal,
12. copy events to the caller output buffer.

The adapter never receives a request that failed local validation or pre-trade
risk.

Cancel and amend paths verify the original client order id is known locally.
Amends also check replacement quantity/notional before routing.

## Simulated Execution

[`SimExecutionAdapter`] is deterministic and intended for:

- integration tests,
- binding smoke tests,
- strategy validation,
- replay examples,
- examples that should not touch a live broker.

Helpers:

- [`simulated_engine`] creates a single-route engine.
- [`simulated_engine_with_routes`] creates a multi-route engine.

The multi-route helper uses route-scoped limits and [`AllowAllRiskGate`],
because the engine already enforces each route's [`RiskLimits`].

```rust
use of_execution::{simulated_engine_with_routes, ExecutionEventBuffer, RouteConfig};
use of_execution_core::{
    AccountId, ClientOrderId, ExecutionSymbol, OrderPrice, OrderQty,
    OrderRequest, OrderSide, OrderType, RiskLimits, RouteId, StrategyId,
    TimeInForce,
};

let route = RouteConfig {
    route_id: RouteId::new("SIM")?,
    account_id: AccountId::new("ACC")?,
    symbol: ExecutionSymbol::new("SIM", "ES")?,
    enabled: true,
    risk_limits: RiskLimits {
        kill_switch: false,
        max_order_qty: 100,
        max_order_notional: 1_000_000,
        max_open_orders: 10,
        max_open_notional: 10_000_000,
        price_band_ticks: 0,
    },
};

let mut engine = simulated_engine_with_routes(vec![route]);
engine.start()?;

let req = OrderRequest {
    client_order_id: ClientOrderId::new("C1")?,
    account_id: AccountId::new("ACC")?,
    route_id: RouteId::new("SIM")?,
    strategy_id: StrategyId::new("STRAT")?,
    symbol: ExecutionSymbol::new("SIM", "ES")?,
    side: OrderSide::Buy,
    order_type: OrderType::Limit,
    time_in_force: TimeInForce::Day,
    quantity: OrderQty::new(1)?,
    limit_price: OrderPrice::new(5000)?,
    stop_price: OrderPrice(0),
    ts_exchange_ns: 0,
    ts_recv_ns: 1,
};

let mut events = ExecutionEventBuffer::with_capacity(8);
engine.submit(req, &mut events)?;
assert!(!events.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Journaling

[`ExecutionJournal`] records commands and events:

- `record_command`
- `record_event`
- `replay`

[`InMemoryJournal`] is useful for tests and embedded simulation.
[`FileExecutionJournal`] is an append-only durable implementation in the OMS
helper surface.

Journal records use [`JournalRecord`]:

- `Command`
- `Event`

Command kinds use [`JournalCommandKind`]:

- `Submit`
- `Cancel`
- `Amend`
- `Poll`
- `RecoverOpenOrders`

Production deployments can replace the journal with a WAL, mmap-backed writer,
database, or replicated log by implementing [`ExecutionJournal`].

## Concurrent Worker

[`ConcurrentExecutionEngine`] gives concurrent producer access while preserving
single-owner order-state mutation.

```mermaid
flowchart LR
  A[Producer A]
  B[Producer B]
  C[Producer C]
  CQ[Bounded command queue]
  Worker[Worker owns<br/>ExecutionEngine]
  RQ[Bounded report queue]

  A --> CQ
  B --> CQ
  C --> CQ
  CQ --> Worker
  Worker --> RQ
```

Properties:

- many producer handles can enqueue commands,
- command queue capacity is explicit,
- report queue capacity is explicit,
- the worker owns adapter and order state,
- reports include command sequence, command kind, result, and events,
- no Tokio runtime is required,
- order-state transitions remain serial and deterministic.

Use [`ExecutionCommandSender`] when producers should not own the worker handle.

Important methods:

- `ConcurrentExecutionEngine::spawn(engine, config)`
- `command_sender()`
- `send(command)`
- `try_send(command)`
- `recv_report()`
- `try_recv_report()`
- `recv_report_timeout(timeout)`
- `request_stop()`
- `join()`

The sender exposes convenience methods:

- `submit(req)`
- `try_submit(req)`
- `cancel(req)`
- `amend(req)`
- `poll()`
- `recover_open_orders()`
- `stop()`

[`ConcurrentExecutionError::Backpressure`] means the bounded command or report
path is full. Callers should retry, pause strategy intent, alert, or trip a
circuit breaker instead of assuming the command was accepted.

## OMS Helper Surface

### Command correlation

[`CommandId`], [`RequestId`], [`CommandIdGenerator`], and
[`CommandCorrelation`] let hosts associate strategy intent, submitted commands,
and reports without relying only on venue ids.

### Event fanout

[`ExecutionEventFanout`] publishes execution events to bounded
[`ExecutionEventSubscriber`] queues.

Full subscriber queues drop deliveries and increment `dropped_events()`.
This is suitable for telemetry or UI subscribers that must not block the main
execution path.

### Lifecycle

[`ExecutionLifecycle`] tracks [`ExecutionAdapterState`] transitions and returns
[`ExecutionLifecycleSnapshot`] values. Use it to expose adapter state changes
such as disconnected, connecting, ready, recovering, degraded, and stopped.

### Durable journal

[`FileExecutionJournal::open(path, sync_on_write)`] creates an append-only
journal. `sync_on_write = true` is safer but slower. `false` is faster but less
durable on sudden power loss.

### Reconciliation

[`reconcile_open_orders(local, venue)`] compares local open-order state against
venue open-order state and returns a [`ReconciliationReport`].

Actions:

- [`ReconciliationAction::Matched`]
- [`ReconciliationAction::VenueOnly`]
- [`ReconciliationAction::LocalOnly`]
- [`ReconciliationAction::RestateFromVenue`]

The function reports differences. It does not mutate state or cancel orders.

### Safety policies

[`DisconnectPolicy`] describes route behavior during disconnects:

- `Hold`
- `RejectNew`
- `CancelOpenOrders`
- `Freeze`

[`RouteSafetyPolicy`] combines disconnect policy, kill switch state, and whether
cancels are allowed while killed.

### Advanced risk

[`AdvancedRiskLimits`] and [`AdvancedRiskGate`] add helpers beyond
`RiskLimits`:

- message-rate limit,
- absolute position limit,
- gross notional limit,
- reduce-only mode,
- basic route limits.

### Position ledger

[`PositionLedger`] folds trade execution events into [`Position`] values keyed
by [`PositionKey`]. It tracks net quantity, buy quantity, sell quantity, gross
notional, and average price. It is an OMS-side exposure helper, not a complete
accounting system.

### Normalization

[`VenueOrderCapabilities`] and [`normalize_order_type`] validate canonical
order type and TIF choices against provider capabilities. Unsupported order
types and TIFs return structured [`RiskRejectReason`] values from
`of_execution_core`.

### Telemetry

[`ExecutionTelemetry`] tracks counts and latency totals. It is intentionally
small so deployments can export metrics to Prometheus, statsd, OpenTelemetry,
or internal systems without making this crate depend on any one telemetry
stack.

### Sharding

[`ShardRouter`] maps [`ShardKey`] values to deterministic shard indexes.
Sharding should preserve order lifecycle ordering within a route/account/symbol
scope while allowing independent scopes to run on separate workers.

### Throttling

[`OrderThrottle`] is a token-bucket style helper:

- `new(capacity, refill_per_sec)`
- `allow(now_ns)`
- `tokens()`

Use it before enqueueing commands when a venue or broker has strict message
limits.

### Replay simulation

[`ReplayDecision`], [`ReplayResult`], and [`replay_simulated_oms`] support
deterministic strategy decision replay using the simulated adapter.

### Provider adapter SDK helpers

[`ProviderAdapterContext`], [`ExecutionAdapterFactory`], and
[`ProviderAdapterSdk`] provide reusable scaffolding for adapter authors.

## Error Model

[`ExecutionError`] variants:

- `Disconnected`
- `BufferFull`
- `RouteNotFound`
- `RiskRejected`
- `Core`
- `Adapter`
- `Journal`

[`ConcurrentExecutionError`] variants:

- `Backpressure`
- `Disconnected`
- `Stopped`
- `WorkerPanic`
- `Execution`

Risk rejection is not an adapter failure. It is a structured local decision,
usually accompanied by a rejection event.

## Low-Latency Notes

- Use typed requests, not JSON, on the command path.
- Keep adapter output in caller-owned [`ExecutionEventBuffer`] values.
- Prefer bounded queues for worker and fanout paths.
- Apply pre-trade risk before provider I/O.
- Keep one owner for order-state mutation.
- Do not call strategy code while holding adapter locks.
- Export metrics out of band rather than formatting strings on the hot path.

## When To Use Which API

Use [`ExecutionEngine`] when:

- you are writing Rust,
- one owner thread already exists,
- deterministic synchronous control is desired,
- you are building tests, replay harnesses, or simulations.

Use [`ConcurrentExecutionEngine`] when:

- many producer threads submit commands,
- explicit queue backpressure matters,
- one worker should own adapter and order state,
- you do not want to depend on Tokio.

Use the C/Python/Java concurrent bindings when:

- host-language code needs non-blocking command queueing,
- host code should poll command reports,
- native code should preserve deterministic state internally.

## What This Crate Does Not Do

This crate does not:

- implement a live provider transport,
- parse FIX/REST/WebSocket messages,
- provide financial advice,
- guarantee venue-side execution behavior,
- replace broker-side risk controls,
- make the dashboard secure for remote production access.

Use `of_execution_adapters` for reusable provider scaffolds, and implement
custom [`ExecutionAdapter`] types for broker-specific behavior.

## Documentation

Additional project documentation:

- `docs/handbook/05h-of-execution-reference.md`
- `docs/handbook/09-oms-architecture.md`
- `docs/handbook/10-oms-cookbook.md`
- `docs/handbook/11-low-latency-design.md`
- `docs/handbook/13-recovery-and-operations.md`
