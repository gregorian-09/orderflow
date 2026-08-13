# `of_execution_adapters`

[![Crates.io](https://img.shields.io/crates/v/of_execution_adapters.svg)](https://crates.io/crates/of_execution_adapters)
[![Docs.rs](https://docs.rs/of_execution_adapters/badge.svg)](https://docs.rs/of_execution_adapters)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](https://opensource.org/license/mit)

`of_execution_adapters` contains optional execution adapter infrastructure for
Orderflow venues, brokers, and execution protocols.

This crate is separate from `of_execution` so provider-specific dependencies
can remain feature-gated and so the core execution engine does not depend on a
particular broker SDK, FIX stack, REST client, WebSocket implementation, or
native transport.

Current FIX surfaces:

- `FixTransportExecutionAdapter`: a live, transport-injected FIX 4.2/4.4
  execution runtime built on `of_fix`;
- `StandardFixExecutionProfile`: standard order-entry/report mapping that can
  be replaced with a venue-certified profile;
- `FixExecutionAdapter`: the original fail-closed compatibility shell.

## What's New in 0.2.0

`of_execution_adapters 0.1.0` was published with Orderflow `0.4.0`. The
current `0.2.0` release adds the transport-injected FIX execution runtime,
standard profile, report/request mapping, durable resend hooks, recovery
context, health/metrics, and deterministic certification harness. These are
additive public capabilities after `0.1.0`; this is still an adapter-
infrastructure family, not a mature suite of counterparty-certified providers.

Versioning rules:

- `of_execution_adapters 0.2.0` depends on `of_execution = 0.2` and
  `of_execution_core = 0.2`;
- provider-specific integrations stay feature-gated and opt-in;
- compatibility shells remain fail closed unless every required live boundary
  is configured;
- production adapters should document certification status, recovery behavior,
  rate limits, duplicate handling, and latency assumptions.

## Design Goals

- Keep provider-specific code outside the execution core.
- Keep optional transports behind feature flags.
- Map provider reports into canonical `ExecutionEvent` values.
- Fail closed until a transport, clock, profile, and recovery policy are wired.
- Expose honest capabilities and health.
- Preserve low-latency typed request/report boundaries.
- Avoid claiming venue production readiness without certification evidence.

## Feature Flags

| Feature | Default | Purpose |
| --- | --- | --- |
| `fix` | no | Enables `of_execution_adapters::fix` and its `of_fix` parser bridge |

The crate has `default = []`. Consumers opt in to provider integrations explicitly:

```toml
[dependencies]
of_execution_adapters = { version = "0.2.0", features = ["fix"] }
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
- [`fix::FixTransportExecutionAdapter`]
- [`fix::FixLiveAdapterConfig`]
- [`fix::FixFrameTransport`]
- [`fix::FixTransportPoll`]
- [`fix::FixTimeSource`]
- [`fix::FixTimeSample`]
- [`fix::FixExecutionProfile`]
- [`fix::StandardFixExecutionProfile`]
- [`fix::FixWorkingOrderContext`]
- [`fix::FixOutboundJournal`]
- [`fix::NoopFixOutboundJournal`]
- [`fix::DurableFixOutboundJournal`]
- [`fix::FixLiveAdapterMetrics`]
- [`fix::FixCertificationHarness`]
- [`fix::FixCertificationConfig`]
- [`fix::FixCertificationReport`]
- [`fix::FixCertificationScenario`]
- [`fix::FixScriptedTransport`]
- [`fix::FixCertificationClock`]
- [`fix::FixFrameExpectation`]

### What the public FIX adapter items mean

The parse/config types define how FIX fields are interpreted and which optional
fields a venue accepts. `parse_execution_report` and
`parse_order_cancel_reject` borrow fields from a validated FIX message and
return typed provider observations; mapping functions convert those
observations into canonical execution events. Encoding functions perform the
inverse operation for order, cancel, amend, and stop-amend commands.

`FixTransportExecutionAdapter` is the generic live session runtime. The host
owns transport I/O, time, credentials, and scheduling through
`FixFrameTransport`, `FixTimeSource`, and the drive/poll contract. A
`FixExecutionProfile` owns venue-specific fields and capability rules;
`StandardFixExecutionProfile` is protocol-capable but is not evidence of venue
certification. `FixExecutionAdapter` remains the compatibility shell.

Outbound journals, working-order context, metrics, and health make uncertain
sends and restart recovery explicit. Certification types are deterministic test
fixtures and transcript expectations, not a substitute for counterparty
certification. All transport and certification abstractions are additive and
must keep malformed frames, duplicate reports, sequence gaps, and uncertain
outcomes fail-closed.

## FIX Module Boundaries

The module has three deliberately separate surfaces: the live generic runtime,
the original compatibility shell, and certification/control-plane tooling.

`FixTransportExecutionAdapter` implements the reusable protocol runtime:

- Logon/Logout, Heartbeat/TestRequest, and liveness timeout;
- strict begin-string/component-id/session sequence validation;
- bounded out-of-order frame retention;
- ResendRequest, possible-duplicate replay, and SequenceReset gap-fill;
- order submit/cancel/replace mapping;
- execution-report and order-cancel-reject mapping;
- session/business reject diagnostics;
- asynchronous open-order recovery requests;
- restored sequence, resend, and working-order context;
- durable pre-send outbound journal hooks;
- health and fixed-size operational counters.

It deliberately does not open sockets, configure TLS, read the system clock,
spawn threads/tasks, hold credentials, or claim venue certification. Those are
injected through `FixFrameTransport`, `FixTimeSource`, and
`FixExecutionProfile`.

The original `FixExecutionAdapter` remains an unchanged fail-closed shell for
source compatibility. Shared bridge pieces remain available independently:

- session configuration shape,
- normalized execution-report struct,
- normalized order-cancel-reject struct,
- validated `of_fix::FixMessageView` to `FixExecutionReport` conversion,
- validated `of_fix::FixMessageView` to `FixOrderCancelReject` conversion,
- canonical OMS request to FIX NewOrderSingle, OrderCancelRequest, and
  OrderCancelReplaceRequest encoding helpers,
- FIX-style exec type/status enums,
- mapping into the canonical execution model,
- standard and venue-specific adapter construction.

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

The standalone mapper does not handle transport ordering, sequence recovery,
or certification. `FixTransportExecutionAdapter` composes those protocol
responsibilities around the mapper; venue-specific rules stay in the profile.

## Live Transport-Injected Adapter

[`fix::FixTransportExecutionAdapter<T, C, P, J>`] implements
`of_execution::ExecutionAdapter` with static dispatch over four host-owned
boundaries:

| Parameter | Responsibility | Hot-path contract |
| --- | --- | --- |
| `T: FixFrameTransport` | TCP/TLS/WebSocket/native transport and complete frame extraction | non-blocking receive into caller-provided memory; complete sends |
| `C: FixTimeSource` | monotonic liveness time, UTC epoch time, FIX `SendingTime(52)` formatting | one coherent, allocation-free sample |
| `P: FixExecutionProfile` | venue fields, capabilities, request encoding, report mapping, recovery message | statically dispatched; no trait object required |
| `J: FixOutboundJournal` | durable original-frame retention before network transmission | return success only at configured durability point |

`J` defaults to `NoopFixOutboundJournal`, preserving a simple four-argument
constructor. Production hosts can inject
`DurableFixOutboundJournal<FileFixDurableResendStore>` or a bounded
asynchronous journal implementation.

### Drive Model

The adapter is synchronous and single-owner. `connect()` opens the injected
transport and sends Logon. The host calls `poll()` from its selected thread,
event loop, or pinned worker. Each poll:

1. emits a previously backpressured event if capacity is available;
2. drains now-contiguous held gap frames;
3. processes at most `max_frames_per_poll` transport frames;
4. performs one heartbeat/TestRequest/Logout timer action;
5. returns canonical events through the bounded `ExecutionEventBuffer`.

There is no internal Tokio runtime, mutex, callback thread, sleep, or socket.
An async application can place this owner on one dedicated task/thread while
many strategies submit through the existing bounded concurrent OMS layer.

### Configuration And Bounds

`FixLiveAdapterConfig` derives the session engine from `FixSessionConfig` and
has explicit bounds for:

- complete frame bytes;
- frames processed per poll;
- held out-of-order frames;
- locally tracked working orders;
- sequences served by one peer resend request;
- replay/gap-fill actions generated by one request;
- in-memory resend messages/bytes through `FixResendStoreConfig`.

Zero capacities are rejected. Frames, held-gap state, working-order context,
resend planning, and output events cannot grow without a configured bound.
Reject text rendered into health diagnostics is truncated. Bound exhaustion
returns `ExecutionError::BufferFull` or an adapter error and leaves health
degraded rather than dropping state silently.

### Send Ordering And Uncertain Outcomes

For every newly sequenced original message the adapter performs:

```text
validate request -> encode -> assign sequence -> durable journal
                 -> in-memory resend retention -> transport send
```

Sequence numbers are never rolled back. A journal or send failure therefore
represents an uncertain command with a consumed sequence, exactly as a real FIX
session requires. The OMS command WAL and idempotency layer must retain the
canonical request before calling the adapter. Recovery then uses the outbound
journal/resend store, OMS WAL, venue status, and reconciliation policy instead
of issuing a new client-order id blindly.

### Receive And Gap Recovery

Inbound frames are checksum/body-length validated by `of_fix`, then session
identity and sequence are checked before profile mapping. A future sequence
causes one ResendRequest and the triggering frame is retained in a bounded,
sorted buffer. Missing replay messages are accepted in order. SequenceReset
advancement discards held frames made obsolete by the gap fill and records the
count. Duplicate sequence numbers are suppressed only when FIX duplicate flags
permit it; unflagged regressions fail closed.

A peer ResendRequest is capped by `max_resend_sequences` and
`max_resend_actions`. Retained application/reject messages are re-encoded with
`PossDupFlag(43)=Y` and `OrigSendingTime(122)`. Administrative/missing messages
become coalesced SequenceReset gap fills. Replays keep their original sequence
and are not re-journaled as new sends.

### Persistence And Restart

Before reconnecting a production session:

1. load and checksum-validate the latest `FixOwnedSequenceSnapshot`;
2. load the durable outbound log into a bounded `FixResendStore`;
3. build `FixSequenceTracker` from the snapshot;
4. construct with `with_journal_and_resend_store(...)`;
5. restore each OMS open order with `restore_working_order(...)`;
6. connect and complete FIX Logon;
7. call `recover_open_orders(...)` and reconcile venue truth;
8. enable new submissions only after the OMS recovery-readiness gate passes.

`sequence_snapshot(trading_day)` exports an owned checksummed snapshot for
`FileFixSequenceSnapshotStore`. `restore_working_order` is idempotent for exact
context and fails on conflicting context, so post-restart cancel/replace has
the original side/order-type/TIF/stop-price fields FIX requires.

For FIX 4.3/4.4, `StandardFixExecutionProfile` sends one idempotent
OrderMassStatusRequest `<AF>` when `recover_open_orders` is first called and
then continues draining reports on later calls. FIX 4.2 venues require a custom
profile because the recovery workflow is counterparty-specific.

### Profiles And Certification

`StandardFixExecutionProfile` supports standard FIX 4.2/4.4 NewOrderSingle,
OrderCancelRequest, OrderCancelReplaceRequest, ExecutionReport, and
OrderCancelReject fields. It accepts an explicit capability set and decimal
quantity/price scales. The adapter checks order type/TIF/amend capability before
assigning a sequence.

Implement `FixExecutionProfile` for counterparty requirements such as:

- SecurityID/SecurityIDSource instead of Symbol;
- custom Logon/application tags;
- account/party and self-trade-prevention fields;
- trading-session/destination fields;
- venue-specific reject and execution variants;
- cancel/replace restrictions;
- proprietary open-order status workflows;
- certified order-type/TIF capability limits.

`StandardFixExecutionProfile` is protocol-capable, not venue-certified. A live
deployment still needs the counterparty specification, TLS/authentication,
session schedule, certificates, certification transcript, rate policy,
cancel-on-disconnect policy, and operational evidence.

### Deterministic Certification Harness

`FixCertificationHarness` and `FixScriptedTransport` provide the reusable
wire-level certification layer around the live adapter. This is deliberately
test/control-plane code: it may retain owned frames and diagnostics, while the
production adapter hot path remains bounded and single-owner.

The stable required scenario inventory covers:

- session lifecycle and heartbeat/TestRequest liveness;
- inbound gaps and peer resend replay/gap fills;
- duplicate reports and partial fills;
- cancel/fill and replace/fill races;
- disconnect/reconnect and restart recovery;
- malformed frames, session rejects, and business rejects;
- frame, queue, retained-gap, resend-work, and event-buffer bounds.

`FixScriptedTransport` implements `FixFrameTransport`, so the same
`FixTransportExecutionAdapter` used by a host can be driven with queued inbound
frames and deterministic send/receive failures. It enforces separate inbound
and outbound frame/byte bounds and archives both directions through
`FixTranscriptCapture`. `FixCertificationClock` supplies repeatable coherent
monotonic/UTC samples without system-clock reads.

Use `FixFrameExpectation` for ordered direction, message-type, sequence, and
exact tag assertions. Use `record_scenario` for event/state/race assertions that
are richer than a wire predicate. A report passes only when:

1. every configured scenario has a non-empty passing result;
2. every required capability is advertised and explicitly exercised;
3. required latency and allocator-profiler evidence has real samples;
4. configured latency/allocation thresholds are satisfied;
5. transcript records/raw bytes were not dropped or evicted;
6. no failure detail overflow occurred.

The harness never installs a global allocator or clock and therefore cannot
distort the measurements it is evaluating. The host records physical-wire
latency and supplies `FixCertificationAllocationEvidence` from its selected
profiler. CI can inspect typed `FixCertificationReport` fields, retained raw
transcript records, and the deterministic transcript rolling hash.

A passing local report means the implementation passed the configured suite.
It does not mean a broker/exchange certified the profile. Archive the exact
counterparty specification revision, environment, endpoint/TLS policy,
profile/config hash, harness report, raw transcript, performance evidence, and
counterparty approval together before changing a public adapter quality claim.

### Health And Metrics

`health()` reports readiness only when transport and FIX session are ready. It
marks sequence recovery, protocol rejects, liveness failure, disconnects, and
bound failures as degraded. `FixLiveAdapterMetrics` exposes frame/byte counts,
events, parse/send errors, held/duplicate/discarded gap frames, replay/gap-fill
counts, rejected resend work, recovery requests, reject counts, clock skew,
and latest/maximum exchange-to-local receive latency. Metrics snapshots do not
allocate; `health()` allocates only when explicitly queried.

Transport implementations are the correct place for raw inbound/outbound
capture and wire-level latency because they see physical read/write completion.
Use bounded `of_fix::FixTranscriptCapture` there; never place unbounded capture
or synchronous logging on the adapter owner thread.

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

This compatibility type is intentionally unchanged. New integrations should
use `FixTransportExecutionAdapter`; existing users retain the same constructor
and fail-closed behavior with no source or runtime semantic break.

## Implementing A Venue Adapter

A counterparty adapter built from this infrastructure should:

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

The FIX runtime deliberately does not provide:

- a concrete TCP/TLS, leased-line, or WebSocket transport;
- REST or WebSocket execution adapters;
- credential or certificate management;
- a counterparty-certified venue profile;
- a process-wide scheduler or async runtime;
- OMS command-WAL ownership or venue-truth reconciliation policy;
- order throttling by itself.

Session sequence recovery, bounded in-memory resend retention, durable resend
store integration, and SequenceReset handling are provided. Their concrete
storage paths, sync policy, transport wiring, and certification evidence remain
deployment decisions.

Use `of_execution` for the execution engine, simulated execution, journals,
reconciliation, safety policies, throttling helpers, and provider SDK helpers.

## Documentation

Additional project documentation:

- `docs/handbook/05i-of-execution-adapters-reference.md`
- `docs/handbook/12-provider-adapter-authoring.md`
- `docs/handbook/11-low-latency-design.md`
