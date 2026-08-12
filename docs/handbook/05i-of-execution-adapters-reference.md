# `of_execution_adapters` Reference

`of_execution_adapters` contains provider-specific execution adapter
infrastructure.
It is intentionally separate from `of_execution` so adapter implementations can
remain feature-gated and provider-specific dependencies do not leak into the
core execution crate.

Current FIX surfaces:

- transport-injected live FIX execution adapter;
- standard and customizable application profiles;
- standalone request/report bridges;
- original fail-closed compatibility shell.

## Feature Flags

| Feature | Purpose |
| --- | --- |
| `fix` | Enables live FIX execution infrastructure, `of_fix` session/codec integration, parser bridges, and compatibility shell |

The crate should stay dependency-light. New providers should be optional
features unless they are pure standard-library mappings.

## FIX API

The module contains:

- `FixSessionConfig`
- `FixExecutionReport`
- `FixOrderCancelReject`
- `FixReportParseConfig`
- `FixRequestEncodeConfig`
- `FixCancelEncodeContext`
- `FixAmendEncodeContext`
- `FixStopAmendEncodeContext`
- `FixReportParseError`
- `FixRequestEncodeError`
- `FixExecType`
- `FixOrdStatus`
- `FixCancelRejectResponseTo`
- `parse_execution_report`
- `parse_order_cancel_reject`
- `encode_order_request`
- `encode_cancel_request`
- `encode_amend_request`
- `encode_stop_amend_request`
- `map_execution_report`
- `map_order_cancel_reject`
- `FixExecutionAdapter`
- `FixTransportExecutionAdapter<T, C, P, J>`
- `FixLiveAdapterConfig`
- `FixFrameTransport` and `FixTransportPoll`
- `FixTimeSource` and `FixTimeSample`
- `FixExecutionProfile` and `StandardFixExecutionProfile`
- `FixWorkingOrderContext`
- `FixOutboundJournal`, `NoopFixOutboundJournal`, and
  `DurableFixOutboundJournal`
- `FixLiveAdapterMetrics`

### `FixSessionConfig`

| Field | Meaning |
| --- | --- |
| `begin_string` | FIX version such as `FIX.4.4` |
| `sender_comp_id` | SenderCompID |
| `target_comp_id` | TargetCompID |
| `heartbeat_secs` | Heartbeat interval |

All string fields use `FixedAscii` to keep the same identifier discipline as
the execution core.

### `FixExecutionReport`

This is the normalized result of parsing a FIX execution report. It is not the
raw tag map. A real FIX adapter can parse transport bytes with `of_fix`, convert
the validated message view with `parse_execution_report`, then call
`map_execution_report`.

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

### `FixReportParseConfig`

`FixReportParseConfig` supplies session context for converting a raw
`of_fix::FixMessageView` into a canonical report:

- default account id when `Account(1)` is absent;
- route id associated with the FIX session;
- venue id attached to `Symbol(55)`;
- quantity scale for integer-normalized `OrderQty`;
- price scale for integer-normalized `OrderPrice`.

The scale fields keep decimal policy explicit. A quantity scale of `100` maps
`LastQty(32)=1.25` to `OrderQty(125)`.

### Request Encoding

`FixRequestEncodeConfig` supplies quantity and price scales for converting
integer-normalized OMS request fields back into FIX decimal ASCII. Scales must
be powers of ten.

Outbound helpers:

- `encode_order_request(out, version, header, config, request, transact_time)`
  emits NewOrderSingle `<D>`.
- `encode_cancel_request(out, version, header, request, context)` emits
  OrderCancelRequest `<F>`.
- `encode_amend_request(out, version, header, config, request, context)` emits
  OrderCancelReplaceRequest `<G>`.
- `encode_stop_amend_request(out, version, header, config, request, context)`
  emits OrderCancelReplaceRequest `<G>` with explicit `StopPx(99)`.

The caller owns the `Vec<u8>` buffer and the `FixSessionHeader`, including
sequence and sending-time fields. `transact_time` is passed as FIX wire bytes
instead of being formatted by the standalone bridge, because venues differ in timestamp
precision and format policy.

Cancel and amend helpers require context because canonical `CancelRequest` and
`AmendRequest` intentionally do not carry every FIX-required field. The context
supplies original side for cancels, side/order-type/TIF for ordinary amends,
and side/order-type/TIF/stop price for stop amends.

The bridge encodes market, limit, stop, and stop-limit new orders. Ordinary
amend encoding supports market and limit replacements; stop and stop-limit
replacements use `encode_stop_amend_request` so replacement `StopPx(99)` is
explicit.

### `parse_execution_report`

`parse_execution_report(message, config, ts_recv_ns)` converts a validated
FIX `ExecutionReport(35=8)` into `FixExecutionReport`.

It fails closed when:

- `MsgType(35)` is not `8`;
- required identifiers are missing;
- `ExecType(150)` or `OrdStatus(39)` is unsupported;
- a fixed-size ASCII field is too long or non-ASCII;
- a decimal field cannot be represented with the configured scale.

### `parse_order_cancel_reject`

`parse_order_cancel_reject(message, config, ts_recv_ns)` converts a validated
FIX `OrderCancelReject(35=9)` into `FixOrderCancelReject`.

It fails closed when:

- `MsgType(35)` is not `9`;
- `ClOrdID(11)`, `OrigClOrdID(41)`, `OrdStatus(39)`, or
  `CxlRejResponseTo(434)` is missing;
- `OrdStatus(39)` or `CxlRejResponseTo(434)` is unsupported;
- a fixed-size ASCII field is too long or non-ASCII;
- numeric fields such as `CxlRejReason(102)` cannot be parsed.

`map_order_cancel_reject` emits `ExecutionType::CancelReject` for response
target `OrderCancelRequest` and `ExecutionType::ReplaceReject` for response
target `OrderCancelReplaceRequest`.

### Mapping Semantics

`map_execution_report` converts FIX-style `ExecType` and `OrdStatus` into the
canonical `ExecutionEvent` model.

Examples:

| FIX concept | Canonical event |
| --- | --- |
| New | `ExecutionType::Ack`, `OrderStatus::New` |
| Trade | `ExecutionType::Trade`, filled or partially filled status |
| Rejected | `ExecutionType::Reject`, `OrderStatus::Rejected` |
| PendingCancel | `ExecutionType::CancelPending` |
| Canceled | `ExecutionType::CancelAck`, `OrderStatus::Cancelled` |
| Replaced | `ExecutionType::ReplaceAck`, `OrderStatus::Replaced` |
| Restated | `ExecutionType::Restated` |

The standalone parser and mapper are deliberately narrow. The live adapter
composes transport/session/recovery behavior around them while keeping venue
rules in an injected profile.

## `FixTransportExecutionAdapter`

This is a synchronous, single-owner FIX execution runtime implementing the
existing `ExecutionAdapter` trait. It statically dispatches:

| Generic | Contract |
| --- | --- |
| `T` | non-blocking, complete-frame `FixFrameTransport` |
| `C` | coherent monotonic/UTC/FIX timestamp `FixTimeSource` |
| `P` | venue-specific `FixExecutionProfile` |
| `J` | pre-send durable `FixOutboundJournal`; defaults to no-op |

The adapter owns no socket/TLS policy, credentials, thread, executor, or async
runtime. A host chooses those components and calls `poll()` from its selected
owner thread/task. `poll()` drains held gap frames, accepts a configured number
of transport frames, maps canonical events, and advances one session timer.

### Protocol Behavior

- Connect emits Logon through the injected transport.
- Application commands are rejected until peer Logon establishes `Ready`.
- Begin string, component IDs, checksums, body length, and sequence numbers are
  validated before application mapping.
- Future inbound messages are held in a sorted bounded buffer while one
  ResendRequest recovers the missing range.
- Peer ResendRequests replay retained application messages with
  `PossDupFlag(43)=Y`; administrative/missing ranges use SequenceReset gap fill.
- SequenceReset discards now-obsolete held frames explicitly.
- Session Reject and BusinessMessageReject create bounded health diagnostics.
- ExecutionReport and OrderCancelReject map to canonical `ExecutionEvent`.
- Heartbeat, TestRequest, Logout, and liveness disconnect actions come from
  `of_fix::FixSessionEngine`.

### Bounds And Backpressure

`FixLiveAdapterConfig` bounds frame bytes, frames per poll, held gap frames,
working orders, peer resend sequence span, resend actions, and in-memory resend
retention. Output uses the existing bounded `ExecutionEventBuffer`. A full
buffer preserves one pending event for a later poll. Any further work stops;
nothing is silently dropped.

### Durability And Restart

Original frames follow this order:

```text
encode -> sequence -> durable journal -> resend retention -> transport send
```

The sequence is never rolled back after assignment. A send error is an
uncertain command and is resolved through OMS WAL/idempotency, retained FIX
frames, venue recovery, and reconciliation.

Production restart should validate `FileFixSequenceSnapshotStore`, load
`FileFixDurableResendStore` into a bounded `FixResendStore`, construct with
`with_journal_and_resend_store`, and restore OMS orders with
`restore_working_order`. `sequence_snapshot(trading_day)` exports the next
inbound/outbound counters for atomic checkpointing. New submissions remain
gated until Logon, open-order recovery, and OMS reconciliation are complete.

### Standard Profile

`StandardFixExecutionProfile` maps standard FIX 4.2/4.4 order entry and reports,
uses explicit price/quantity scales, and enforces configured capabilities before
sequence assignment. FIX 4.3/4.4 recovery uses OrderMassStatusRequest `<AF>`;
FIX 4.2 and proprietary recovery require a custom profile.

This runtime is production-capable infrastructure, not a certified venue
profile. Deployment still requires the counterparty specification, certified
custom fields/capabilities, transport/TLS/authentication, session schedule,
rate limits, cancel-on-disconnect policy, transcript evidence, and operations
approval. Raw transcript capture belongs in the transport using bounded
`FixTranscriptCapture`, where physical wire completion is observable.

### Metrics

`FixLiveAdapterMetrics` reports frame/byte/event totals, inbound/send errors,
held/duplicate/discarded gap frames, replay/gap-fill counts, rejected resend
work, recovery requests, protocol reject counts, clock-skew reports, and latest
and maximum exchange-to-local receive latency. `session_metrics()` returns the
allocation-free session counters from `of_fix`.

## `FixExecutionAdapter`

`FixExecutionAdapter` is a fail-closed shell. Its current behavior is:

- `connect()` returns an adapter error saying transport is not configured.
- `submit`, `cancel`, `amend`, `poll`, and `recover_open_orders` return
  disconnected errors.
- `capabilities()` reports ordinary FIX-style order support.
- `health()` reports degraded/disconnected state.

This compatibility API is intentionally unchanged. New integrations use
`FixTransportExecutionAdapter`; existing users do not receive new side effects
or source breaks.

## Implementing A Venue Profile

A venue integration should:

1. implement or reuse a complete-frame transport,
2. provide monotonic/UTC timestamp sampling,
3. implement custom application tags and report variants in a profile,
4. report only certified capabilities,
5. load sequence/resend/OMS state before connect,
6. use a durable outbound journal,
7. capture bounded certification transcripts in the transport,
8. pass venue certification and failure/recovery scenarios,
9. gate production activation on reconciliation and operational evidence.

For broader guidance, see [Provider Adapter Authoring](./12-provider-adapter-authoring.md).
