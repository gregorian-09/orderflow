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

- `FixSessionConfig` — FIX sender/target configuration.
- `FixExecutionReport` — Minimal FIX execution-report payload after transport parsing.
- `FixOrderCancelReject` — Minimal FIX OrderCancelReject payload after transport parsing.
- `FixReportParseConfig` — Context required to map raw FIX execution reports into canonical OMS fields.
- `FixRequestEncodeConfig` — Context required to encode canonical OMS requests as FIX order-entry frames.
- `FixCancelEncodeContext` — Extra fields required to encode a canonical cancel request as FIX.
- `FixAmendEncodeContext` — Extra fields required to encode a canonical amend request as FIX.
- `FixStopAmendEncodeContext` — Extra fields required to encode a stop/stop-limit amend request as FIX.
- `FixReportParseError` — Errors returned while converting a parsed FIX execution report.
- `FixRequestEncodeError` — Errors returned while encoding canonical OMS requests as FIX frames.
- `FixExecType` — FIX ExecType values normalized for mapping.
- `FixOrdStatus` — FIX OrdStatus values normalized for mapping.
- `FixCancelRejectResponseTo` — FIX CxlRejResponseTo values normalized for mapping.
- `parse_execution_report` — Parses a validated FIX `ExecutionReport(35=8)` into a normalized report.
- `parse_order_cancel_reject` — Parses a validated FIX `OrderCancelReject(35=9)` into a normalized report.
- `encode_order_request` — Encodes a canonical new-order request as FIX NewOrderSingle `<D>`.
- `encode_cancel_request` — Encodes a canonical cancel request as FIX OrderCancelRequest `<F>`.
- `encode_amend_request` — Encodes a canonical amend request as FIX OrderCancelReplaceRequest `<G>`.
- `encode_stop_amend_request` — Encodes a stop/stop-limit amend request as FIX OrderCancelReplaceRequest `<G>`.
- `map_execution_report` — Maps a parsed FIX execution report into a canonical execution event.
- `map_order_cancel_reject` — Maps a parsed FIX OrderCancelReject into a canonical execution event.
- `FixExecutionAdapter` — FIX execution adapter shell.
- `FixTransportExecutionAdapter<T, C, P, J>` — Generic synchronous adapter that composes transport, clock, profile, and journal implementations.
- `FixLiveAdapterConfig` — Bounded configuration for a transport-injected FIX execution adapter.
- `FixFrameTransport` — Transport boundary that sends and receives already-framed FIX messages.
- `FixTransportPoll` — Result returned by a transport poll, describing received frames or transport actions.
- `FixTimeSource` — Clock boundary used to obtain FIX timestamps without coupling the adapter to wall-clock APIs.
- `FixTimeSample` — Timestamp sample supplied by the configured FIX time source.
- `FixExecutionProfile` — Venue-specific mapping of canonical execution concepts to FIX fields and values.
- `StandardFixExecutionProfile` — Built-in FIX profile for the standard execution mappings supported by the adapter.
- `FixWorkingOrderContext` — Original-order context needed by FIX cancel and replace messages.
- `FixOutboundJournal` — Journal boundary for recording outbound FIX messages and their replay identity.
- `NoopFixOutboundJournal` — Deliberately disabled outbound journal for hosts that provide persistence elsewhere.
- `DurableFixOutboundJournal` — Outbound journal implementation that retains messages for recovery and audit.
- `FixLiveAdapterMetrics` — Allocation-free operational counters for the live FIX adapter.
- `FixCertificationScenario` — One deterministic certification case for exercising a FIX behavior.
- `FixCertificationCapability` — Capability result identifying which certification behavior a profile supports.
- `FixCertificationConfig` — Inputs controlling certification scenario execution.
- `FixCertificationHarness` — Deterministic runner for scripted FIX certification scenarios.
- `FixCertificationReport` — Outcome and diagnostics produced by certification execution.
- `FixFrameExpectation` — Expected properties of one outbound or inbound FIX frame.
- `FixExpectedField` — Expected tag/value assertion within a certification frame.
- `FixScriptedTransportConfig` — Configuration for an in-memory scripted FIX transport.
- `FixScriptedTransport` — Deterministic transport used by certification and adapter tests.
- `FixCertificationClock` — Deterministic coherent clock for FIX adapter certification.

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

### Certification Harness

`FixScriptedTransport` is a bounded `FixFrameTransport` implementation for
counterparty simulation. It queues complete inbound frames, retains bounded
outbound frames, injects deterministic connect/send/receive/disconnect
failures, and records both directions through `FixTranscriptCapture`.
`FixCertificationClock` makes lifecycle and timeout scripts repeatable.

`FixCertificationHarness` records the complete FIX scenario inventory and can
assert exact ordered `FixFrameExpectation` values for direction, `MsgType(35)`,
`MsgSeqNum(34)`, and arbitrary exact fields. Rich event/state assertions are
recorded as scenario failures. Details and transcripts are bounded; overflow,
raw evidence loss, or record eviction fails certification rather than silently
weakening it.

The report separately exposes missing scenarios, retained failures,
unsupported required capabilities, unexercised advertised capabilities,
latency evidence, allocator-profiler evidence, and transcript counters/hash.
Optional maximum latency/allocation thresholds are evaluated at report time.
The default full-suite configuration requires all 13 scenarios, latency
samples, allocation evidence, and a complete transcript.

Certification instrumentation is outside the live hot path. A deployment must
still retain counterparty approval and specification/profile/config evidence;
a local passing report is not a venue certification claim.

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
