# `of_execution_adapters`

`of_execution_adapters` contains optional execution adapter scaffolds for
Orderflow venues, brokers, and execution protocols.

This crate is separate from `of_execution` so provider-specific dependencies
can remain feature-gated and so the core execution engine does not depend on a
particular broker SDK, FIX stack, REST client, WebSocket implementation, or
native transport.

Current scaffold:

- `fix` feature: FIX-style execution-report mapping and fail-closed adapter
  shell.

## Design Goals

- Keep provider-specific code outside the execution core.
- Keep optional transports behind feature flags.
- Map provider reports into canonical `ExecutionEvent` values.
- Fail closed until a real transport is wired.
- Expose honest capabilities and health.
- Preserve low-latency typed request/report boundaries.
- Avoid pretending that a scaffold is a production live adapter.

## Feature Flags

| Feature | Default | Purpose |
| --- | --- | --- |
| `fix` | no | Enables `of_execution_adapters::fix` |

The crate has `default = []`. Consumers opt in to provider scaffolds explicitly:

```toml
[dependencies]
of_execution_adapters = { version = "0.3.0", features = ["fix"] }
```

## Public API Inventory

With the `fix` feature enabled:

- [`fix::FixSessionConfig`]
- [`fix::FixExecutionReport`]
- [`fix::FixExecType`]
- [`fix::FixOrdStatus`]
- [`fix::map_execution_report`]
- [`fix::FixExecutionAdapter`]

## FIX Scaffold

The FIX module is not a full FIX engine. It does not implement TCP/TLS,
logon/logout, heartbeats, resend requests, sequence reset, session stores,
message parsing, or exchange-specific certification behavior.

It provides the reusable pieces that are safe to share at this layer:

- session configuration shape,
- normalized execution-report struct,
- FIX-style exec type/status enums,
- mapping into the canonical execution model,
- adapter shell implementing the `ExecutionAdapter` trait,
- fail-closed behavior until transport is configured.

## FixSessionConfig

[`fix::FixSessionConfig`] describes a FIX session identity:

- `begin_string`: FIX version such as `FIX.4.4`
- `sender_comp_id`: SenderCompID
- `target_comp_id`: TargetCompID
- `heartbeat_secs`: negotiated heartbeat interval

The string fields use fixed ASCII identifiers from `of_execution_core`. This
keeps the same low-allocation and FFI-safe identity discipline as the rest of
the execution model.

```rust
use of_execution_adapters::fix::FixSessionConfig;

let cfg = FixSessionConfig::new("FIX.4.4", "BUY_SIDE", "BROKER", 30)?;
assert_eq!(cfg.heartbeat_secs, 30);
# Ok::<(), of_execution_core::ExecutionCoreError>(())
```

## FixExecutionReport

[`fix::FixExecutionReport`] is the normalized result of parsing a provider FIX
execution report. It is not a raw tag map.

Important fields:

- `exec_type`
- `ord_status`
- `cl_ord_id`
- `orig_cl_ord_id`
- `order_id`
- `exec_id`
- `account_id`
- `route_id`
- `symbol`
- `last_qty`
- `last_price`
- `cumulative_qty`
- `leaves_qty`
- `average_price`
- `ts_exchange_ns`
- `ts_recv_ns`
- `text`

A real FIX adapter should parse bytes or tag maps into this struct, then call
[`fix::map_execution_report`] to produce a canonical `ExecutionEvent`.

## FIX Exec Type And Status

[`fix::FixExecType`] represents FIX-style execution report reasons:

- `New`
- `Canceled`
- `Replaced`
- `PendingCancel`
- `Rejected`
- `Trade`
- `Expired`
- `PendingReplace`
- `Restated`
- `Status`

[`fix::FixOrdStatus`] represents FIX-style order status:

- `New`
- `PartiallyFilled`
- `Filled`
- `DoneForDay`
- `Canceled`
- `Replaced`
- `PendingCancel`
- `Stopped`
- `Rejected`
- `Suspended`
- `PendingNew`
- `Calculated`
- `Expired`
- `AcceptedForBidding`
- `PendingReplace`

The mapper converts these into canonical `ExecutionType` and `OrderStatus`
values from `of_execution_core`.

## Mapping Semantics

[`fix::map_execution_report`] maps normalized FIX reports into canonical
execution events.

Examples:

| FIX concept | Canonical event |
| --- | --- |
| New | `ExecutionType::Ack`, `OrderStatus::New` |
| Trade | `ExecutionType::Trade`, filled or partially filled status |
| Rejected | `ExecutionType::Reject`, `OrderStatus::Rejected` |
| PendingCancel | `ExecutionType::CancelPending` |
| Canceled | `ExecutionType::CancelAck`, `OrderStatus::Cancelled` |
| PendingReplace | `ExecutionType::ReplacePending` |
| Replaced | `ExecutionType::ReplaceAck`, `OrderStatus::Replaced` |
| Expired | `ExecutionType::Expired`, `OrderStatus::Expired` |
| Restated | `ExecutionType::Restated` |
| Status | `ExecutionType::Status` |

The mapper preserves:

- client order id,
- original client order id,
- venue order id,
- execution id,
- account and route,
- symbol,
- fill quantities and prices,
- cumulative and leaves quantities,
- average price,
- exchange and receive timestamps,
- bounded text.

It does not handle transport ordering, FIX session sequence numbers, resend
logic, duplicate suppression, or broker-specific certification rules. Those
belong in a real adapter implementation.

## FixExecutionAdapter

[`fix::FixExecutionAdapter`] implements `of_execution::ExecutionAdapter`, but
it is a fail-closed shell.

Current behavior:

- `connect()` returns an adapter error saying transport is not configured.
- `submit()`, `cancel()`, `amend()`, `poll()`, and `recover_open_orders()`
  return disconnected errors.
- `capabilities()` reports ordinary FIX-style order support.
- `health()` reports disconnected/degraded state with protocol info.
- `config()` returns the stored [`fix::FixSessionConfig`].

This is intentional. It gives adapter authors a compilable shape and shared
mapping contract without implying that a production FIX session is present.

## Implementing A Real Adapter

A production adapter built from this scaffold should:

1. Own the provider session or transport.
2. Validate lifecycle before accepting commands.
3. Translate `OrderRequest`, `CancelRequest`, and `AmendRequest` into provider
   messages.
4. Parse provider reports into `FixExecutionReport` or an equivalent internal
   normalized report.
5. Convert reports into `ExecutionEvent` values.
6. Preserve client-order-id semantics where the venue supports them.
7. Implement `recover_open_orders` with restatement reports.
8. Expose accurate `ExecutionCapabilities`.
9. Expose meaningful `ExecutionHealth`.
10. Keep command and report queues bounded.
11. Handle duplicate or out-of-order provider messages explicitly.
12. Keep credentials and secrets out of logs and diagnostic text.

## Request Mapping Guidance

Canonical fields should map explicitly:

| Canonical field | FIX-style mapping |
| --- | --- |
| `client_order_id` | `ClOrdID` |
| `orig_client_order_id` | `OrigClOrdID` |
| `account_id` | `Account` |
| `route_id` | session, destination, or broker route |
| `symbol` | `Symbol` or provider-native instrument fields |
| `side` | `Side` |
| `order_type` | `OrdType` |
| `time_in_force` | `TimeInForce` |
| `quantity` | `OrderQty` |
| `limit_price` | `Price` |
| `stop_price` | `StopPx` |

Do not silently coerce unsupported order types or TIFs. Return a structured
error or let capability checks reject the command before routing.

## Report Mapping Guidance

Provider reports should become canonical `ExecutionEvent` values:

- provider order id -> `venue_order_id`
- provider execution id -> `execution_id`
- last fill quantity/price -> `last_qty` and `last_price`
- cumulative quantity -> `cumulative_qty`
- leaves quantity -> `leaves_qty`
- average price -> `average_price`
- status and exec reason -> `exec_type` and `order_status`
- provider text -> bounded `text`

If a provider omits a field, use the canonical empty or zero value and document
that behavior in the adapter.

## Capabilities And Health

`ExecutionCapabilities` should be honest:

- set unsupported order types to `false`,
- set unsupported TIFs to `false`,
- set `native_client_order_id` according to provider behavior,
- choose a realistic `LatencyClass`.

`ExecutionHealth` should report:

- `connected`,
- `degraded`,
- `health_seq`,
- `last_error`,
- `protocol_info`.

Increment health sequence when meaningful lifecycle state changes.

## Low-Latency Guidance

- Parse provider messages into canonical reports quickly.
- Use bounded queues for inbound and outbound reports.
- Avoid string formatting on command and report hot paths.
- Do not call strategy callbacks while holding adapter locks.
- Apply venue throttles before sending.
- Prefer fixed-size IDs and integer-normalized prices/quantities.
- Surface backpressure, disconnects, and degraded states explicitly.

## Testing Checklist

Every real adapter should test:

- connect and health reporting,
- command rejection before connect,
- new-order ack mapping,
- venue reject mapping,
- partial fill mapping,
- full fill mapping,
- cancel pending and cancel ack mapping,
- cancel reject mapping,
- replace pending and replace ack mapping,
- replace reject mapping,
- expired/restated/status reports,
- duplicate provider reports,
- out-of-order provider reports,
- bounded event buffer pressure,
- recovery/open-order restatement,
- capability reporting,
- reconnect lifecycle transitions.

## What This Crate Does Not Do

This crate does not currently provide:

- a production FIX transport,
- a REST execution adapter,
- a WebSocket execution adapter,
- broker authentication,
- exchange certification logic,
- a session message store,
- resend/sequence-reset implementation,
- order throttling by itself.

Use `of_execution` for the execution engine, simulated execution, journals,
reconciliation, safety policies, throttling helpers, and provider SDK helpers.

## Documentation

Additional project documentation:

- `docs/handbook/05i-of-execution-adapters-reference.md`
- `docs/handbook/12-provider-adapter-authoring.md`
- `docs/handbook/11-low-latency-design.md`
