# `of_execution_adapters`

[![Crates.io](https://img.shields.io/crates/v/of_execution_adapters.svg)](https://crates.io/crates/of_execution_adapters)
[![Docs.rs](https://docs.rs/of_execution_adapters/badge.svg)](https://docs.rs/of_execution_adapters)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](https://opensource.org/license/mit)

`of_execution_adapters` contains optional execution adapter scaffolds for
Orderflow venues, brokers, and execution protocols.

This crate is separate from `of_execution` so provider-specific dependencies
can remain feature-gated and so the core execution engine does not depend on a
particular broker SDK, FIX stack, REST client, WebSocket implementation, or
native transport.

Current scaffold:

- `fix` feature: FIX-style execution-report mapping and fail-closed adapter
  shell.

## First Release: 0.1.0

`of_execution_adapters` publishes as `0.1.0` inside the broader Orderflow
`0.4.0` release. It is a new adapter-scaffold family, not a mature production
provider suite.

Versioning rules:

- `of_execution_adapters` depends on `of_execution = 0.1` and
  `of_execution_core = 0.1`;
- provider-specific scaffolds stay feature-gated and opt-in;
- fail-closed scaffolds are allowed in `0.1.x` as long as README and docs state
  the production boundary clearly;
- production adapters should document certification status, recovery behavior,
  rate limits, duplicate handling, and latency assumptions.

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
| `fix` | no | Enables `of_execution_adapters::fix` and its `of_fix` parser bridge |

The crate has `default = []`. Consumers opt in to provider scaffolds explicitly:

```toml
[dependencies]
of_execution_adapters = { version = "0.1.0", features = ["fix"] }
```

## Public API Inventory

With the `fix` feature enabled:

- [`fix::FixSessionConfig`]
- [`fix::FixExecutionReport`]
- [`fix::FixOrderCancelReject`]
- [`fix::FixReportParseConfig`]
- [`fix::FixRequestEncodeConfig`]
- [`fix::FixCancelEncodeContext`]
- [`fix::FixAmendEncodeContext`]
- [`fix::FixStopAmendEncodeContext`]
- [`fix::FixReportParseError`]
- [`fix::FixRequestEncodeError`]
- [`fix::FixExecType`]
- [`fix::FixOrdStatus`]
- [`fix::FixCancelRejectResponseTo`]
- [`fix::parse_execution_report`]
- [`fix::parse_order_cancel_reject`]
- [`fix::encode_order_request`]
- [`fix::encode_cancel_request`]
- [`fix::encode_amend_request`]
- [`fix::encode_stop_amend_request`]
- [`fix::map_execution_report`]
- [`fix::map_order_cancel_reject`]
- [`fix::FixExecutionAdapter`]

## FIX Scaffold

The FIX module is not a full FIX engine. It does not implement TCP/TLS,
logon/logout, heartbeats, resend requests, sequence reset, session stores, or
exchange-specific certification behavior.

It provides the reusable pieces that are safe to share at this layer:

- session configuration shape,
- normalized execution-report struct,
- normalized order-cancel-reject struct,
- validated `of_fix::FixMessageView` to `FixExecutionReport` conversion,
- validated `of_fix::FixMessageView` to `FixOrderCancelReject` conversion,
- canonical OMS request to FIX NewOrderSingle, OrderCancelRequest, and
  OrderCancelReplaceRequest encoding helpers,
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
# #[cfg(feature = "fix")]
# {
use of_execution_adapters::fix::FixSessionConfig;

let cfg = FixSessionConfig::new("FIX.4.4", "BUY_SIDE", "BROKER", 30)?;
assert_eq!(cfg.heartbeat_secs, 30);
# }
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

A real FIX adapter can parse bytes with `of_fix::parse_message`, call
[`fix::parse_execution_report`] to produce this struct, then call
[`fix::map_execution_report`] to produce a canonical `ExecutionEvent`.

## FixReportParseConfig

[`fix::FixReportParseConfig`] supplies the venue/session context that raw FIX
execution reports do not fully encode in a canonical Orderflow form:

- default `account_id` when `Account(1)` is absent;
- `route_id` associated with the session;
- `venue` assigned to parsed `Symbol(55)` values;
- quantity scale for integer-normalized `OrderQty`;
- price scale for integer-normalized `OrderPrice`.

This keeps decimal/tick-size assumptions out of the parser. For example, a
quantity scale of `100` maps `LastQty(32)=1.25` to `OrderQty(125)`.

## FixRequestEncodeConfig

[`fix::FixRequestEncodeConfig`] supplies the inverse scaling policy for
canonical OMS requests:

- quantity scale for integer-normalized `OrderQty`;
- price scale for integer-normalized `OrderPrice`.

The encode helpers require scales to be powers of ten. This keeps decimal
rendering deterministic and avoids silently converting fixed-point quantities
into ambiguous FIX decimals.

## parse_execution_report

[`fix::parse_execution_report`] converts a validated `of_fix::FixMessageView`
with `MsgType(35)=8` into [`fix::FixExecutionReport`].

It requires common report identifiers such as `ExecType(150)`, `OrdStatus(39)`,
`ClOrdID(11)`, `OrderID(37)`, `ExecID(17)`, and `Symbol(55)`. Optional fill
fields default to zero when absent. Decimal fields must be representable with
the configured scale; otherwise the parser fails closed with
[`fix::FixReportParseError`].

## parse_order_cancel_reject

[`fix::parse_order_cancel_reject`] converts a validated
`of_fix::FixMessageView` with `MsgType(35)=9` into
[`fix::FixOrderCancelReject`].

It requires `ClOrdID(11)`, `OrigClOrdID(41)`, `OrdStatus(39)`, and
`CxlRejResponseTo(434)`. Optional `OrderID(37)`, `Account(1)`, `Symbol(55)`,
`CxlRejReason(102)`, `TransactTime(60)`, and `Text(58)` are copied when
present. The mapper emits `ExecutionType::CancelReject` for rejected cancel
requests and `ExecutionType::ReplaceReject` for rejected cancel/replace
requests.

## Request Encoding

The outbound bridge converts canonical OMS requests into low-allocation FIX
wire frames through `of_fix` builders:

- [`fix::encode_order_request`] -> NewOrderSingle `<D>`
- [`fix::encode_cancel_request`] -> OrderCancelRequest `<F>`
- [`fix::encode_amend_request`] -> OrderCancelReplaceRequest `<G>`
- [`fix::encode_stop_amend_request`] -> OrderCancelReplaceRequest `<G>` with
  explicit `StopPx(99)`

The helpers encode into caller-owned `Vec<u8>` buffers. The caller supplies a
`FixSessionHeader` with sequence and sending-time fields, and supplies
wire-format `TransactTime(60)` bytes explicitly so venue profiles can choose
the exact timestamp representation.

Cancel and amend encoding use explicit context structs because the canonical
`CancelRequest` and `AmendRequest` do not carry every FIX-required field:

- [`fix::FixCancelEncodeContext`] supplies original side and transact time.
- [`fix::FixAmendEncodeContext`] supplies original side, replacement order
  type, replacement TIF, and transact time.
- [`fix::FixStopAmendEncodeContext`] supplies original side, stop/stop-limit
  order type, replacement TIF, explicit stop price, and transact time.

The bridge encodes market, limit, stop, and stop-limit new orders. Plain amend
encoding supports market and limit replacements; stop and stop-limit
replacements use [`fix::encode_stop_amend_request`] so the replacement
`StopPx(99)` is explicit.

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
