# Provider Adapter Authoring

This chapter is the authoritative engineering manual for adding, reviewing, and
operating provider adapters in Orderflow. It is intentionally detailed because
an adapter is the boundary where an external provider's timing, identifiers,
wire protocol, errors, capabilities, rate limits, and recovery behavior become
part of a deterministic library contract.

A provider adapter is not merely a wrapper around an SDK. It is a translator,
lifecycle owner, quality boundary, and evidence-producing component. A correct
adapter makes provider behavior explicit to the rest of the system. An
incomplete adapter hides uncertainty, and hidden uncertainty is dangerous in
market-data and execution systems.

This chapter covers:

- market-data adapters in `of_adapters`;
- execution adapters in `of_execution_adapters`;
- the shared execution adapter contract in `of_execution`;
- FIX infrastructure in `of_fix`;
- provider factories and convenience SDK helpers;
- transport, timing, buffering, health, recovery, and rate limiting;
- testing, certification, observability, security, and release evidence.

The guide distinguishes market-data and execution adapters throughout. They
share engineering principles, but they do not share the same domain contract.

## 1. What an Adapter Is

An adapter translates between a provider boundary and a canonical Orderflow
boundary.

For market data, the direction is:

~~~mermaid
flowchart LR
    Provider[Provider wire data or SDK] --> Transport[Provider transport]
    Transport --> Decode[Provider decode and validation]
    Decode --> Normalize[Canonical TradePrint or BookUpdate]
    Normalize --> Quality[Sequence, timestamp, and quality policy]
    Quality --> Runtime[of_runtime]
    Runtime --> Analytics[Analytics and signals]
    Runtime --> Persist[Persistence and replay]
~~~

For execution, the direction is bidirectional:

~~~mermaid
flowchart LR
    Command[Canonical order command] --> Guard[Identity and risk guards]
    Guard --> Encode[Provider request encoding]
    Encode --> Submit[Provider transport]
    Submit --> Report[Provider acknowledgement or fill]
    Report --> Decode[Provider report decoding]
    Decode --> Map[Canonical ExecutionEvent]
    Map --> State[OMS state and ledger]
~~~

The adapter owns the translation and provider-facing lifecycle. It does not own
strategy logic, analytics policy, canonical order state, application UI, or
language binding convenience behavior.

### 1.1 Adapter responsibilities

A production adapter should:

1. establish and close its provider transport;
2. authenticate or perform provider logon where required;
3. expose the provider's real capabilities;
4. translate provider messages into canonical types;
5. preserve provider identifiers, timestamps, and sequences;
6. apply explicit validation and quality policy;
7. bound all queues, buffers, retained frames, and transcripts;
8. report lifecycle and health transitions;
9. recover from disconnects according to a documented state machine;
10. prevent unsafe commands while not ready;
11. expose enough telemetry to diagnose latency and loss;
12. provide deterministic tests that do not require private credentials;
13. document limitations, unsupported features, and operational assumptions.

### 1.2 What an adapter must not do

An adapter must not:

- implement trading strategy or signal logic;
- mutate canonical OMS state outside the execution engine;
- silently coerce unsupported order types or time-in-force values;
- replace missing timestamps without recording the loss of meaning;
- reorder events without an explicit contract and test;
- claim a capability because the provider might support it in some account;
- hide a provider reject as a transport failure;
- allocate unbounded memory from provider-controlled input;
- log credentials, API keys, session secrets, or private order data;
- use synthetic events on a live path unless the mode is explicitly mock;
- perform blocking file or network work from a method documented as non-blocking;
- retry an uncertain execution command without reconciliation;
- let provider SDK types escape into reusable domain crates.

## 2. Choose the Correct Adapter Family

Orderflow has two adapter families.

### 2.1 Market-data adapters: `of_adapters`

Market-data adapters provide trades and book updates to `of_runtime`. The
canonical trait and normalized event types are in `of_adapters` and
`of_core`.

The market-data boundary includes:

- `SubscribeReq` for symbol and depth requests;
- `RawEvent::Trade` for normalized trade prints;
- `RawEvent::Book` for normalized book updates;
- `AdapterHealth` for basic transport health;
- `AdapterRuntimeMode` for mock, live, replay, bridge, or unknown mode;
- `AdapterConnectionState` for disconnected, connecting, streaming,
  reconnecting, backoff, replay, or unknown state;
- `AdapterOperationalStatus` for typed operational metrics and redacted
  provider state.

Market-data adapters are consumed by the poll-driven runtime. They should make
event ordering, freshness, sequence gaps, duplicate handling, and book-depth
semantics explicit.

### 2.2 Execution adapters: `of_execution_adapters`

Execution adapters implement the shared `of_execution::ExecutionAdapter`
contract. They receive canonical order commands and emit canonical
`ExecutionEvent` values.

The execution boundary includes:

- `OrderRequest`, `CancelRequest`, and `AmendRequest`;
- `ExecutionEventBuffer`, a caller-owned bounded output buffer;
- `ExecutionCapabilities`, which describes supported operations;
- `ExecutionHealth`, which describes session and transport state;
- `ExecutionError`, which separates disconnection, bounds, core, adapter,
  and journal failures;
- `ExecutionAdapterFactory` and `ProviderAdapterSdk` for reusable
  route construction and validation.

Execution adapters must not own the authoritative order state machine. They
translate commands and reports; `of_execution` applies canonical events
to state, journal, idempotency, reconciliation, and the position ledger.

### 2.3 FIX adapters

FIX execution integrations are intentionally split:

- `of_fix` owns generic FIX tags, framing, parsing, encoding, session
  sequencing, timers, resend behavior, and reusable profiles;
- `of_execution_adapters::fix` owns execution mapping, live adapter
  composition, venue configuration, and certification;
- `of_execution` owns canonical OMS behavior.

Do not place a second FIX parser inside a venue adapter. Do not place OMS
semantics inside generic FIX session code.

## 3. Repository Orientation

Before implementing an adapter, read these sources in order:

1. `crates/of_execution/src/lib.rs` for the execution trait, capabilities,
   health, errors, routes, buffers, and engine integration;
2. `crates/of_execution/src/oms.rs` for factories, lifecycle, throttling,
   and convenience helpers;
3. `crates/of_adapters/src/lib.rs` for market-data contracts and status;
4. the closest existing provider module under `crates/of_adapters/src/`;
5. `crates/of_fix/src/lib.rs` and `session.rs` for FIX behavior;
6. `crates/of_execution_adapters/src/fix.rs` and its `fix/`
   modules for the live FIX composition;
7. the relevant crate README and handbook reference page;
8. provider conformance and certification tests;
9. CI feature matrices and packaging metadata.

Useful commands:

~~~bash
rg -n "trait ExecutionAdapter|struct ExecutionCapabilities|struct ExecutionHealth" \
  crates/of_execution crates/of_execution_adapters

rg -n "trait MarketDataAdapter|RawEvent|AdapterHealth|AdapterOperationalStatus" \
  crates/of_adapters crates/of_runtime

rg -n "FixFrameTransport|FixTimeSource|FixOutboundJournal|FixCertification" \
  crates/of_fix crates/of_execution_adapters

cargo tree -p of_execution_adapters
cargo tree -p of_adapters
~~~

Do not begin by copying a provider implementation wholesale. First identify
which behavior is generic and which behavior is provider-specific.

## 4. Provider Research Before Coding

A provider adapter should be designed from provider evidence, not from the
shape of a convenient SDK.

### 4.1 Required provider facts

Record the following before implementation:

| Area | Questions to answer |
| --- | --- |
| Identity | What identifies an account, session, venue, symbol, order, execution, and request? |
| Transport | Is the connection TCP, TLS, WebSocket, REST, FIX, native SDK, or hybrid? |
| Authentication | How are credentials supplied, rotated, refreshed, and revoked? |
| Timing | Which timestamps are exchange, provider, client-send, and client-receive time? |
| Ordering | Are sequence numbers global, per session, per symbol, or absent? |
| Replay | How are gaps detected and recovered? |
| Commands | Which submit, cancel, replace, mass-cancel, and status operations exist? |
| Reports | Which acknowledgement, reject, fill, cancel, replace, and restatement reports exist? |
| Capabilities | Which order types, TIFs, symbols, accounts, and routes are supported? |
| Limits | What are request, message, connection, and order-rate limits? |
| Recovery | How are open orders and uncertain requests reconciled after reconnect? |
| Errors | Which errors are transport, validation, throttle, session, and business rejects? |
| Environment | What differs between simulation, certification, and production? |
| Compliance | What audit, retention, certification, and deployment requirements apply? |

Write answers in the adapter README and configuration documentation. If a
fact is unknown, represent it as an unknown or unsupported capability. Do not
turn an assumption into a default.

### 4.2 Provider evidence

Prefer, in order:

1. official protocol specifications and provider API documentation;
2. provider certification or onboarding material;
3. official SDK source and type definitions;
4. controlled sandbox observations;
5. captured, redacted, and approved transcripts;
6. only then, community examples or inferred behavior.

Every non-obvious mapping should have a source note in the adapter documentation
or test name. Private provider documents must not be committed unless their
license explicitly permits redistribution.

### 4.3 Provider capability matrix

Create a capability matrix before coding:

| Capability | Provider behavior | Adapter representation | Evidence |
| --- | --- | --- | --- |
| Market order | Supported, restricted, or absent | `ExecutionCapabilities::market` | Provider reference |
| Stop order | Supported with provider condition | `stop` plus validation | Sandbox test |
| FOK | Account or venue dependent | `tif_fok` | Capability probe |
| Amend | Native replace or cancel/new fallback | `amend` | Protocol specification |
| Client id | Preserved, normalized, or generated | `native_client_order_id` | Report transcript |
| Open-order recovery | Mass status, REST query, or unavailable | `recover_open_orders` | Recovery test |
| Duplicate reports | Possible after reconnect | Deduplicated by OMS | Reconnect test |

Capabilities are not documentation decoration. The runtime and risk layer use
them to reject unsupported operations before provider I/O.

## 5. Data and Identity Model

### 5.1 Canonical identity

The canonical identity must remain stable across the full path.

For market data:

~~~text
provider symbol -> validated SymbolId -> accumulator key -> persisted symbol
~~~

For execution:

~~~text
application request id
    -> canonical client order id
    -> provider client order id
    -> provider order id
    -> provider execution id
    -> canonical ExecutionEvent
~~~

Preserve every available identity. If an identity is unavailable, document the
substitution and ensure it cannot create collisions.

### 5.2 Symbols

Provider symbols often differ from canonical symbols because of:

- separators such as `BTC-USD` versus `BTCUSD`;
- contract month or expiry encoding;
- exchange prefixes;
- venue-specific aliases;
- case sensitivity;
- currency or product suffixes;
- synthetic spread symbols;
- option strike and put/call encoding.

Define a reversible symbol mapping where possible. A mapping should answer:

1. Which canonical symbol maps to the provider symbol?
2. Can a provider symbol map to more than one canonical symbol?
3. Is the mapping stable across sessions?
4. Is the mapping account, market, or environment dependent?
5. Where is tick size, lot size, multiplier, and currency metadata stored?
6. How are unknown symbols rejected?
7. How are symbols shown in health and audit output?

Do not derive tick size or quantity precision from a display string.

### 5.3 Client order identifiers

Client order identifiers need:

- a maximum provider length;
- an allowed character set;
- uniqueness scope;
- retry and restart behavior;
- correlation to the canonical request;
- collision detection;
- preservation or mapping rules.

If the provider truncates identifiers, truncation must be deterministic and
collision-safe. Never use a simple prefix truncation for identifiers whose
uniqueness depends on the discarded suffix.

### 5.4 Numeric normalization

Orderflow uses integer-normalized price and quantity values. The adapter must
know the provider's:

- price tick or decimal scale;
- quantity lot or decimal scale;
- contract multiplier;
- notional currency;
- minimum and maximum quantities;
- minimum price increment;
- allowed decimal precision;
- rounding direction for order submission.

The adapter must not silently round an order. Return a structured validation
error when a canonical value cannot be represented exactly or safely.

For a conversion, document:

~~~text
canonical price -> provider integer/decimal -> provider wire field
provider fill price -> canonical integer price
~~~

Test round-trip conversion for zero, one tick, maximum supported value, and
values near a rounding boundary.

## 6. The Market-Data Adapter Contract

Market-data adapters are polled by the runtime. The adapter should keep provider
I/O and normalization inside the adapter and emit only canonical events.

### 6.1 Subscription request

`SubscribeReq` contains a canonical symbol and requested depth. A provider
may support less depth than requested. The adapter must choose and document one
policy:

- reject the subscription;
- clamp to the provider maximum and mark degradation;
- subscribe to the maximum and report actual depth;
- emulate depth from a lower-level feed;
- accept the request but provide only trades.

Never claim requested depth was delivered when it was truncated.

### 6.2 Event mapping

A trade mapping must define:

- symbol;
- normalized price;
- normalized size;
- aggressor side;
- provider sequence;
- exchange timestamp;
- receive timestamp;
- provider quality state.

A book mapping must define:

- symbol;
- bid or ask side;
- level index;
- normalized price;
- normalized size;
- snapshot, insert, update, delete, or reset action;
- provider sequence;
- exchange timestamp;
- receive timestamp;
- depth truncation policy.

### 6.3 Ordering policy

Choose one of these policies explicitly:

| Policy | Meaning | Appropriate when |
| --- | --- | --- |
| Preserve | Emit provider order exactly | Provider order is authoritative |
| Reject | Refuse events that move backward | Corruption is unsafe |
| Flag | Emit events and set out-of-order quality | Research or tolerant consumers |
| Reorder | Buffer and sort before emit | Provider gives bounded sequence disorder |

Reordering adds latency and memory. If used, define the maximum holdback,
timeout, memory bound, and behavior when the bound is exceeded.

### 6.4 Duplicate policy

Duplicates may arise from reconnect replay, provider resend, polling overlap,
or SDK callbacks. Define the deduplication key:

- provider sequence;
- provider event id;
- symbol plus sequence;
- timestamp and payload hash only as a last resort.

A timestamp-only key is unsafe for high-rate feeds. Count suppressed duplicates
in operational status or metrics.

### 6.5 Book state policy

If the provider supplies snapshots and deltas:

1. obtain a snapshot;
2. record its sequence;
3. buffer deltas received during snapshot acquisition;
4. discard deltas older than the snapshot;
5. apply contiguous deltas;
6. mark a gap and resynchronize when continuity fails.

Do not apply a delta to an uninitialized book unless the provider contract
explicitly guarantees that the stream begins with a complete snapshot.

## 7. Market-Data Lifecycle and Health

### 7.1 Runtime modes

`AdapterRuntimeMode` communicates how the adapter is operating:

| Mode | Meaning | Safety consequence |
| --- | --- | --- |
| `Mock` | Deterministic local or synthetic data | Never treat as live evidence |
| `Live` | Provider transport is active | Credentials and production limits apply |
| `Replay` | Persisted events are being replayed | Deterministic time/order rules apply |
| `Bridge` | External caller injects events | Caller owns feed validity |
| `Unknown` | Adapter did not report a mode | Conservative diagnostics |

### 7.2 Connection states

`AdapterConnectionState` distinguishes transport lifecycle:

- `Disconnected`: no provider transport exists;
- `Connecting`: initial connection or logon is in progress;
- `Streaming`: provider events can be consumed normally;
- `Reconnecting`: a prior connection is being re-established;
- `Backoff`: reconnect is delayed by bounded policy;
- `Replay`: persisted data is being consumed;
- `Unknown`: no more specific state is available.

A state transition must be observable and deterministic. Do not toggle state on
every event; change it only when the semantic lifecycle state changes.

### 7.3 Health sequence

The health sequence is an edge detector. Increment it when meaningful health
information changes, such as:

- connection state;
- degraded flag;
- last error category;
- protocol/session identifier;
- reconnect attempt state;
- stale-feed state.

Do not increment it for ordinary event throughput. Consumers use it to avoid
reprocessing unchanged health snapshots.

### 7.4 Operational status

`AdapterOperationalStatus` is a control-plane snapshot. It may sort
subscriptions, redact endpoints, and allocate strings because it is queried
outside the event hot path.

It can describe:

- mode and connection state;
- redacted endpoint and non-secret application name;
- reconnect attempt;
- active subscriptions;
- queue depth and capacity;
- dropped events and sequence gaps;
- stale state;
- bounded raw-capture utilization;
- message age and normalized-event age.

Endpoint redaction must remove credentials, paths, queries, fragments, tokens,
listen keys, and user information. The status surface is not a secret store.

## 8. Execution Adapter Trait Contract

The shared execution trait is:

~~~rust
pub trait ExecutionAdapter: Send {
    fn connect(&mut self) -> ExecutionResult<()>;
    fn submit(
        &mut self,
        req: &OrderRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()>;
    fn cancel(
        &mut self,
        req: &CancelRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()>;
    fn amend(
        &mut self,
        req: &AmendRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()>;
    fn poll(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize>;
    fn recover_open_orders(
        &mut self,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<usize>;
    fn capabilities(&self) -> ExecutionCapabilities;
    fn health(&self) -> ExecutionHealth;
}
~~~

The trait is synchronous and mutable. A caller owns the adapter and invokes
methods in a defined order. A concurrent owner may serialize calls on a worker,
but that does not make the adapter internally thread-safe for arbitrary direct
sharing.

### 8.1 `connect`

`connect` establishes the provider session or transport. It should:

- validate static configuration before network I/O;
- establish transport;
- authenticate or log on;
- initialize sequence and session state;
- initialize provider subscriptions or account context;
- transition health to ready only after the provider is usable;
- return a structured error when connection is not established.

Do not report `connected = true` after a TCP connection if provider
logon, authentication, or session synchronization is still pending.

Repeated connect behavior must be documented. Prefer idempotence for an already
ready session, or return a clear state error. Do not open duplicate sessions.

### 8.2 `submit`

`submit` receives a canonical `OrderRequest`. It should:

1. validate adapter readiness;
2. validate route, account, symbol, and provider capability;
3. validate provider-specific precision and bounds;
4. allocate or derive a provider client id deterministically;
5. encode the request;
6. journal the original command if the adapter owns that hook;
7. send the provider request;
8. emit immediate canonical events only when the provider actually reports them.

A successful send is not necessarily an acknowledgement. Do not emit an
acknowledgement merely because bytes were written. If the provider has no
synchronous acknowledgement, return success for transmission and let `poll`
deliver the authoritative report, according to the adapter contract.

### 8.3 `cancel`

Cancellation requires the provider order identity and often the original
client-order identity. Define:

- whether cancellation is by canonical client id, provider order id, or both;
- whether cancel is valid before acknowledgement;
- how a cancel race with a fill is represented;
- how a provider cancel reject is mapped;
- whether cancellation is idempotent.

A cancel request that is sent successfully must not be treated as proof that
the order is cancelled. Only an authoritative report transitions the order.

### 8.4 `amend`

Amend may be native cancel/replace or a provider-specific sequence. Define:

- which fields may change;
- whether quantity means total quantity or remaining quantity;
- required original order fields;
- whether a new provider order id is created;
- client-order-id chaining;
- behavior when the order fills during the replace race;
- rejection when the provider has no native amend.

If a provider lacks native amend, do not silently implement cancel/new unless
the public capability and risk semantics explicitly permit that policy.

### 8.5 `poll`

`poll` drains provider reports into a caller-owned bounded buffer. It must:

- avoid blocking when documented as a polling operation;
- return the number of events appended;
- preserve provider report order unless documented otherwise;
- stop before overflowing `ExecutionEventBuffer`;
- leave unread reports available when the output buffer is full;
- surface disconnects and provider errors without losing already decoded events;
- keep per-call work bounded by configured frame/event limits.

If one provider frame can yield multiple canonical events, define atomicity.
Either emit all events only when the buffer can hold them or emit a documented
prefix and retain the rest. Never silently drop the suffix.

### 8.6 `recover_open_orders`

Recovery retrieves provider-authoritative open-order state after reconnect or
restart. It should emit `ExecutionType::Restated` events where that is the
canonical recovery representation.

Recovery must define:

- query mechanism;
- pagination and maximum page size;
- snapshot sequence or watermark;
- handling of orders created during the query;
- handling of orders missing from provider response;
- provider order status mapping;
- duplicate suppression;
- interaction with local checkpoints;
- behavior when the provider cannot provide open orders.

Never mark a local order cancelled merely because one incomplete provider query
did not return it.

## 9. Canonical Request Mapping

Map every field explicitly.

| Canonical field | Typical provider mapping | Questions |
| --- | --- | --- |
| `client_order_id` | `ClOrdID`, `clientOrderId`, provider token | Is it preserved and what is its length? |
| `account_id` | account, subaccount, clearing account | Is it required per message or session? |
| `route_id` | venue, session, destination, connection | Can one adapter serve multiple routes? |
| `symbol` | provider symbol or contract key | Is mapping reversible and versioned? |
| `side` | Buy/Sell enum or numeric value | Are short-sale or position flags required? |
| `order_type` | market, limit, stop, stop-limit | Are provider conditions required? |
| `time_in_force` | Day, GTC, IOC, FOK, GTD | Is expiry timestamp required? |
| `quantity` | quantity, leaves, contracts, lots | What precision and multiplier apply? |
| `limit_price` | limit price | What tick and rounding rules apply? |
| `stop_price` | trigger/stop price | Is the trigger side or type required? |

### 9.1 Capability-before-mapping rule

Check capability before encoding. A provider rejection is later and less useful
than a local capability error. The adapter may still receive a provider reject
because capabilities can be account or session dependent; that reject must then
be mapped as an execution event rather than hidden.

### 9.2 Unsupported values

When a provider cannot represent a canonical value:

1. reject before provider I/O when the limitation is known;
2. return a stable adapter or core error;
3. include the unsupported field in diagnostics without leaking secrets;
4. do not substitute another order type silently;
5. test that the canonical state remains unchanged.

### 9.3 Provider profiles

Use a profile when the same transport supports multiple venue or account
mappings. A profile should own:

- provider message types;
- tag or field mappings;
- enum conversions;
- required fields;
- capability values;
- precision and symbol rules;
- reject and report mappings;
- certification expectations.

A profile should not own sockets, global runtime state, or OMS persistence.

## 10. Canonical Report Mapping

Every authoritative provider report should become a canonical
`ExecutionEvent`, even when the report represents rejection, cancellation,
replacement, expiry, restatement, or adapter degradation.

Map:

| Provider value | Canonical field |
| --- | --- |
| provider order id | `venue_order_id` |
| provider execution id | `execution_id` |
| provider client id | `client_order_id` |
| original client id | `orig_client_order_id` |
| account | `account_id` |
| route/venue | `route_id` and symbol |
| last fill quantity | `last_qty` |
| last fill price | `last_price` |
| cumulative quantity | `cumulative_qty` |
| remaining quantity | `leaves_qty` |
| average fill price | `average_price` |
| exchange timestamp | `ts_exchange_ns` |
| receive timestamp | `ts_recv_ns` |
| provider execution type | `ExecutionType` |
| provider order status | `OrderStatus` |
| provider reason/text | canonical reason and bounded text |

If a provider omits a field, use the canonical empty or zero representation
only when that representation is defined as “not supplied.” Document the loss
of information. Do not fabricate a provider order id or execution id.

### 10.1 Report classification

At minimum, distinguish:

- acknowledgement;
- new-order reject;
- trade or fill;
- partial fill;
- cancel acknowledgement;
- cancel reject;
- replace acknowledgement;
- replace reject;
- expiry;
- restatement;
- status-only report;
- session or adapter degradation report where the canonical model supports it.

### 10.2 Monotonic report validation

Track provider sequence when available. Validate:

- sequence does not move backward;
- duplicate reports are handled;
- cumulative quantity does not decrease without a documented correction;
- leaves quantity is coherent with cumulative and original quantity;
- average price is valid for the quantity;
- terminal statuses do not accept illegal later transitions;
- execution ids are not reused across unrelated orders.

Some venues issue corrections or restatements. Support them explicitly; do not
apply generic monotonic rules that reject a documented correction path.

## 11. Bounded Buffers and Backpressure

Orderflow uses caller-owned bounded event buffers because unbounded buffering
turns provider bursts into memory risk.

`ExecutionEventBuffer::with_capacity(n)` reserves an event bound. Its
`push` method returns `ExecutionError::BufferFull` rather than
growing beyond the configured maximum.

### 11.1 Buffer ownership

The caller owns the output buffer. The adapter may append events but must not:

- retain a reference to the buffer;
- clear caller events without an explicit contract;
- assume capacity beyond `max_len`;
- allocate a second unbounded event queue as an escape hatch;
- discard events when `push` returns `BufferFull`.

### 11.2 Buffer sizing

Size the buffer for the maximum expected output of one call:

- one command may produce an immediate reject plus diagnostics;
- one provider frame may produce multiple execution events;
- cancel-all or recovery may produce many events;
- a market-data poll may produce a burst;
- a FIX session action may produce a reject or gap-fill event.

If the provider can return more than the caller can accept, implement a
bounded internal queue with visible occupancy and a documented overflow policy.
Possible policies are stop-reading, backpressure, drop-with-quality, or
connection degradation. The policy must be explicit and tested.

### 11.3 Partial drain semantics

When draining internal events into the caller buffer:

1. inspect available output capacity;
2. transfer only events that fit;
3. preserve remaining events in order;
4. return the number transferred;
5. surface a bounded condition if the internal queue is full.

Never drain a collection destructively before confirming that the destination
can accept the event.

### 11.4 Provider-controlled sizes

Bound:

- frame size;
- number of frames per poll;
- number of events per frame;
- pending sequence-gap frames;
- working-order records;
- resend records;
- transcript records;
- raw-message capture;
- diagnostic text;
- reconnect attempts and backoff state.

All bounds need an error, metric, or degraded-health consequence.

## 12. Transport Design

The transport should expose the minimum operations the adapter needs.

### 12.1 Non-blocking transport

A non-blocking poll should distinguish:

- no complete message currently available;
- one complete frame available;
- transport disconnected;
- transport error.

For FIX, `FixTransportPoll` provides `Idle`, `Frame`, and
`Disconnected`. It writes complete frames into adapter-owned memory.

### 12.2 Transport ownership

The transport owns:

- socket or SDK connection;
- TLS state;
- OS handles;
- receive and send buffers;
- provider-specific connection operations.

The adapter owns:

- canonical command mapping;
- provider session semantics;
- report mapping;
- lifecycle state;
- capability and health reporting.

### 12.3 Transport errors

Classify errors without losing detail:

| Error class | Meaning | Canonical response |
| --- | --- | --- |
| Connect | Physical connection failed | health transition plus adapter error |
| Authenticate | Credentials/logon rejected | degraded or disconnected, no blind retry |
| Send | Request could not be written | adapter error; command outcome may be uncertain |
| Receive | Frame could not be read | health transition and recovery |
| Decode | Bytes are malformed | reject/quarantine according to protocol |
| Timeout | Expected response absent | recovery or uncertain state |
| Throttle | Provider rate limit | bounded backoff and telemetry |

A send failure after partial transmission may leave command outcome uncertain.
Execution adapters must reconcile before retrying.

### 12.4 TLS and security

Do not disable certificate validation to simplify local testing. Make endpoint,
certificate, trust-store, and hostname behavior configurable without exposing
secrets in status output. Keep credentials outside source and examples.

## 13. Time and Latency

Time has multiple meanings. Do not collapse them.

### 13.1 Market-data time

Preserve:

- provider event time;
- exchange event time when supplied;
- adapter receive time;
- normalized emission time if separately measured.

Mark clock skew or missing time using existing quality policy.

### 13.2 Execution time

Execution adapters may need:

- monotonic send/receive time for latency;
- UTC protocol time for wire fields;
- exchange timestamp from report;
- journal append time;
- state-application time.

Use monotonic clocks for elapsed durations. Use UTC or provider-required
formats only for protocol and audit fields.

### 13.3 FIX time source

`FixTimeSource` writes `SendingTime(52)` into a caller-owned buffer
and returns `FixTimeSample`. A sample combines:

- monotonic nanoseconds for liveness and latency;
- Unix nanoseconds for receive and audit timestamps;
- initialized FIX timestamp length.

This lets a low-latency host provide a cached or vDSO-backed clock while keeping
protocol formatting explicit.

### 13.4 Latency classification

`ExecutionCapabilities.latency_class` should describe the adapter's
transport class:

- `NativeFix`;
- `NativeBinary`;
- `StreamingWebSocket`;
- `RestConvenience`;
- `Simulated`.

It is a classification, not a performance guarantee. Benchmark real deployment
paths and document allocation, queue, and scheduling behavior.

## 14. Reconnect and Recovery State Machine

A robust adapter has an explicit state machine.

~~~mermaid
stateDiagram-v2
    [*] --> Disconnected
    Disconnected --> Connecting: connect
    Connecting --> LogonPending: transport ready
    LogonPending --> Ready: provider logon accepted
    LogonPending --> Degraded: provider logon rejected
    Ready --> Recovering: gap or reconnect
    Ready --> Degraded: stale or protocol fault
    Recovering --> Ready: state reconciled
    Recovering --> Degraded: recovery incomplete
    Degraded --> Connecting: bounded retry
    Degraded --> Stopped: operator stop
    Ready --> Stopped: operator stop
    Stopped --> [*]
~~~

### 14.1 Reconnect steps

A reconnect sequence should:

1. record the transition and reason;
2. stop accepting unsafe commands;
3. close or reset the old transport;
4. wait according to bounded backoff;
5. establish a fresh transport;
6. authenticate or log on;
7. restore sequence/session state;
8. resubscribe or request open orders;
9. reconcile pending and uncertain state;
10. emit recovery events;
11. mark ready only after the required recovery gate passes.

### 14.2 Backoff

Backoff should define:

- initial delay;
- multiplier;
- maximum delay;
- jitter policy;
- maximum attempts or operator intervention;
- reset condition after stable readiness;
- interaction with provider rate limits.

Do not create a reconnect storm. Do not use unbounded exponential arithmetic.
Use checked duration calculations and test overflow boundaries.

### 14.3 Recovery and uncertain commands

A command is uncertain when local transmission may have occurred but no
authoritative response was observed. On uncertainty:

- preserve the original client id;
- record the command and uncertainty state;
- query provider order state or use drop copy;
- match by client id, provider id, or deterministic evidence;
- retry only after the provider confirms absence;
- emit a recovery or reconciliation result.

Never generate a new order id and submit again simply because a response timed
out.

## 15. Rate Limits and Throttling

Provider limits can apply separately to:

- new orders;
- cancels;
- replaces;
- mass status;
- open-order queries;
- session messages;
- WebSocket subscriptions;
- REST requests;
- connection attempts.

`OrderThrottle` is a reusable token-bucket-style helper for command
throttling. It does not replace provider-specific policy. Configure and test
limits per route and account where required.

### 15.1 Throttle behavior

Define whether a limit causes:

- immediate `BufferFull` or adapter error;
- bounded queueing;
- delayed send;
- provider reconnect;
- command rejection;
- health degradation.

A blocking wait in a hot path is usually unacceptable. Prefer an explicit
not-ready or bounded queue result.

### 15.2 Cancellation priority

Many providers require cancel requests to remain responsive during a new-order
burst. If queues are separated, define priority and fairness. Do not starve
cancels behind an unbounded submit queue.

### 15.3 Server responses

A server throttle response should update telemetry and backoff. Do not treat
it as a permanent capability failure. Do not immediately retry at the same rate.

## 16. Error Model

Execution adapters use `ExecutionResult<T>` and `ExecutionError`.

| Error | Meaning | Adapter behavior |
| --- | --- | --- |
| `Disconnected` | Transport/session unavailable | reject unsafe operation and update health |
| `BufferFull` | Caller or bounded internal event capacity reached | preserve unread events and surface pressure |
| `RouteNotFound` | Route/account/symbol not configured | do not send provider command |
| `RiskRejected` | Core risk rejected request | do not send provider command |
| `Core` | Canonical model or state error | preserve state and report exact cause |
| `Adapter(String)` | Provider-specific failure | include safe diagnostic context |
| `Journal(String)` | Journal durability or encoding failure | do not claim durable send |

A provider business reject is not automatically `ExecutionError::Adapter`.
If the provider accepted and rejected the order at the business level, emit a
canonical reject event. Use an adapter error when the adapter cannot produce a
valid canonical outcome.

### 16.1 Error text

Error text should be:

- bounded;
- safe to log;
- free of credentials and full private payloads;
- stable enough for diagnostics;
- not used as a machine-readable discriminator.

Use typed variants or structured fields for programmatic handling.

### 16.2 Panic and unwind

The C ABI must not allow Rust unwinding across FFI. Adapter code should avoid
panics on provider-controlled input. Check indexing, length arithmetic, enum
conversion, capacity, and UTF-8 boundaries.

## 17. FIX Adapter Architecture

FIX integrations should compose generic infrastructure.

~~~mermaid
flowchart TD
    Config[FixLiveAdapterConfig] --> Adapter[FixTransportExecutionAdapter]
    Transport[FixFrameTransport] --> Adapter
    Clock[FixTimeSource] --> Adapter
    Profile[FixExecutionProfile] --> Adapter
    Journal[FixOutboundJournal] --> Adapter
    Adapter --> Session[FixSessionEngine]
    Session --> Codec[of_fix parser and encoder]
    Adapter --> OMS[Canonical ExecutionEvent]
    Scripted[FixScriptedTransport] --> Adapter
    Harness[FixCertificationHarness] --> Scripted
~~~

### 17.1 `FixFrameTransport`

The transport must:

- establish physical connection;
- send one complete frame;
- poll one complete received frame without blocking;
- disconnect;
- preserve complete frame boundaries;
- return transport-specific errors;
- avoid retaining borrowed frame slices after the call.

It may be TCP, TLS, WebSocket, leased line, native SDK, or deterministic test
transport. Protocol parsing belongs above it.

### 17.2 `FixSessionEngine`

The session engine owns generic protocol behavior such as:

- logon and logout;
- heartbeat and test request;
- incoming and outgoing sequence numbers;
- resend requests;
- gap fills and sequence resets;
- duplicate flags;
- session state;
- bounded pending gaps;
- session metrics.

The venue adapter supplies profile and transport behavior; it should not copy
session sequencing logic.

### 17.3 `FixOutboundJournal`

The outbound journal records original messages by sequence before transmission
when the configured durability contract requires it. Replay frames are excluded
from the original-message journal.

`NoopFixOutboundJournal` is suitable only when the host intentionally
accepts no durable resend guarantee. `DurableFixOutboundJournal` adapts a
durable resend store. Document what “recorded” means: memory accepted, queued
to a writer, fsynced, replicated, or otherwise durable.

### 17.4 FIX message validation

Validate:

- BeginString;
- BodyLength;
- CheckSum;
- MsgType;
- required session fields;
- sequence number;
- duplicate flag;
- timestamp;
- required business fields;
- enum values;
- bounded tag values.

Malformed frames should not panic or corrupt session state. The response may be
reject, disconnect, quarantine, or certification failure depending on protocol
and configuration.

## 18. FIX Request and Report Mapping

Use `of_fix` helpers for borrowed parsing and caller-owned encoding.

Common fields include:

- `MsgType(35)`;
- `MsgSeqNum(34)`;
- `PossDupFlag(43)`;
- `ClOrdID(11)`;
- `OrigClOrdID(41)`;
- `OrderID(37)`;
- `ExecID(17)`;
- `ExecType(150)`;
- `OrdStatus(39)`;
- `LastQty(32)`;
- `LastPx(31)`;
- `CumQty(14)`;
- `LeavesQty(151)`;
- `AvgPx(6)`.

Do not assume a tag's presence or value type. Profile validation determines
required fields. A successful parse is not automatically a valid business
report; map and validate business meaning separately.

### 18.1 New order

A new order mapping should define:

- message type, normally `NewOrderSingle <D>`;
- sender and target identifiers;
- client order id;
- account and route;
- symbol;
- side;
- order type;
- quantity;
- price fields;
- time in force;
- transact time;
- provider-specific tags;
- checksum/body length generation.

### 18.2 Cancel

A cancel mapping should define:

- `OrderCancelRequest <F>`;
- new client order id;
- original client order id;
- provider order id;
- side and symbol requirements;
- quantity semantics;
- account and route;
- reject mapping for invalid cancellation.

### 18.3 Replace

A replace mapping should define:

- `OrderCancelReplaceRequest <G>`;
- new and original client ids;
- provider order id;
- new quantity and price;
- original order context;
- stop price when relevant;
- time-in-force behavior;
- replace reject mapping.

### 18.4 Execution report

An `ExecutionReport <8>` mapping should define:

- execution type;
- order status;
- identifiers;
- quantities and prices;
- leaves and cumulative semantics;
- reject reason;
- text;
- exchange and receive timestamps;
- restatement and correction rules.

## 19. Provider Factory and Convenience SDK

`ProviderAdapterContext` contains a provider name, route configurations,
and an execution lifecycle snapshot. It is a construction-time context, not a
mutable runtime event bus.

`ExecutionAdapterFactory` builds an adapter from that context. A factory
should:

- validate required provider configuration;
- validate route count and route identity;
- reject conflicting routes;
- construct provider-specific resources;
- return a typed adapter;
- avoid opening unexpected network connections during mere configuration
  validation unless explicitly documented.

`ProviderAdapterSdk` provides reusable helpers such as simulated
capabilities and route validation. Use it to avoid duplicating trivial policy,
but keep provider-specific validation in the provider factory.

### 19.1 Multi-route adapters

A single adapter may serve multiple routes, accounts, or symbols only when the
provider session model allows it. Define:

- route lookup key;
- account isolation;
- symbol capability;
- per-route health;
- per-route rate limits;
- command correlation;
- reconnect scope;
- recovery scope;
- shutdown behavior.

If one route disconnects, do not mark unrelated healthy routes down unless they
share a transport or session.

### 19.2 Factory lifecycle

Document whether factory-built resources are:

- owned by the adapter;
- shared by several adapters;
- borrowed from a host;
- closed on adapter drop;
- closed only through an explicit method.

Avoid hidden global singletons for credentials, sessions, or mutable provider
state.

## 20. Configuration Design

Configuration is part of the public API.

### 20.1 Configuration fields

For each field document:

- type and unit;
- default;
- valid range;
- whether required;
- whether it can change after construction;
- secret handling;
- effect on hot path;
- compatibility and serialization behavior.

### 20.2 Secret fields

Credentials should be supplied through secure runtime configuration, secret
stores, or environment integration controlled by the host. They must not be:

- embedded in examples;
- included in Debug output;
- included in health or operational status;
- written to normal logs;
- included in error strings;
- persisted in raw transcripts.

If a configuration type derives `Debug`, implement redaction or store secret
material outside the debug-visible type.

### 20.3 Endpoint validation

Validate scheme, authority, host, port, and allowed environment before connect.
Store a redacted endpoint for diagnostics. Do not accept arbitrary endpoint
overrides in production without an explicit policy.

## 21. Provider Session Semantics

### 21.1 Logon

Define:

- credentials and authentication order;
- application or sender name;
- session identifier;
- requested heartbeat;
- sequence number initialization;
- timezone and sending time;
- provider logon response;
- retry and disable behavior.

A transport-connected but unauthenticated session is not ready.

### 21.2 Heartbeats

Track:

- last inbound message;
- last outbound message;
- last heartbeat;
- test request id;
- heartbeat deadline;
- disconnect threshold.

Heartbeat timers must use monotonic time. Document whether the adapter emits
heartbeats itself or delegates them to `FixSessionEngine` or provider SDK.

### 21.3 Session close

Close should:

1. stop accepting new commands;
2. attempt provider logout when safe;
3. flush required journal state;
4. close transport;
5. update health;
6. release resources;
7. leave no worker thread running.

Timeout and forced-close behavior must be documented.

### 21.4 Session schedule

For scheduled venues, define:

- session open and close;
- maintenance window;
- timezone;
- holiday handling;
- reconnect before open;
- command behavior outside session;
- status during scheduled closure.

Do not use local machine timezone implicitly.

## 22. Testing Strategy

Adapter tests must prove translation, lifecycle, limits, failure, and recovery.
A successful unit test that only checks one happy-path request is insufficient.

### 22.1 Test layers

1. pure mapping unit tests;
2. transport tests with a deterministic fake;
3. adapter lifecycle tests;
4. buffer and bound tests;
5. provider conformance tests;
6. certification scenarios;
7. replay and recovery tests;
8. end-to-end OMS or runtime tests;
9. optional sandbox tests with credentials;
10. performance tests with representative bursts.

### 22.2 Mapping tests

For every supported request and report:

- valid minimum;
- valid maximum;
- each enum variant;
- missing required provider field;
- unsupported canonical value;
- invalid precision;
- invalid identifier;
- malformed bytes;
- provider reject;
- duplicate;
- out-of-order;
- terminal-state conflict.

Assertions should include every mapped field, not merely message type.

### 22.3 Market-data adapter tests

Test:

- connect and health;
- disconnect;
- initial subscription;
- duplicate subscription;
- unsubscribe;
- empty poll;
- one event;
- burst event;
- bounded queue;
- sequence gap;
- duplicate event;
- out-of-order event;
- stale event;
- snapshot plus delta ordering;
- reconnect and resubscribe;
- provider error;
- malformed payload;
- endpoint redaction;
- deterministic subscription ordering;
- operational status counters.

### 22.4 Execution adapter tests

Test:

- command before connect;
- connect twice;
- submit;
- cancel;
- amend;
- poll;
- provider acknowledgement;
- full fill;
- partial fill;
- cancel acknowledgement;
- cancel reject;
- replace acknowledgement;
- replace reject;
- venue reject;
- duplicate report;
- out-of-order report;
- report after terminal state;
- recovery;
- uncertain command;
- bounded output;
- capabilities;
- health sequence;
- throttling;
- shutdown.

### 22.5 Conformance test structure

A conformance test should call the public adapter contract, not private
implementation methods. It should be usable by every provider implementation
with a provider-specific fixture or deterministic transport.

A useful fixture supplies:

- adapter construction;
- known valid route;
- known symbol;
- deterministic clock;
- fake provider response;
- bounded output buffer;
- expected canonical event.

### 22.6 Certification tests

FIX certification uses `FixScriptedTransport` and
`FixCertificationHarness`. The harness should cover:

- session lifecycle and liveness;
- both resend directions;
- gap fill and sequence reset;
- duplicate report;
- partial fill;
- cancel/replace races;
- disconnect recovery;
- malformed message;
- protocol reject;
- buffer bounds;
- complete transcript;
- capability-specific scenarios.

Require only capabilities approved by the counterparty. A test that enables every
possible order type is not evidence for a venue that supports only a subset.

### 22.7 Failure injection

Use deterministic failure points for:

- connect;
- send;
- receive;
- disconnect;
- inbound capacity;
- outbound capacity;
- transcript capacity;
- journal write;
- parse;
- provider reject;
- heartbeat timeout;
- sequence gap;
- recovery query.

Assert health transition, retained events, error category, and recovery action.

## 23. FIX Scripted Transport

`FixScriptedTransportConfig` bounds inbound frames, outbound frames,
working transport state, and transcript capture. A zero capacity must be
rejected where the contract requires a usable queue.

`FixScriptedTransport` is not a network simulator with unlimited memory.
It is a deterministic counterparty transport that:

- accepts scripted inbound frames;
- records outbound frames;
- can inject failures;
- preserves bounded transcript behavior;
- implements `FixFrameTransport`;
- supports certification and adapter tests without credentials.

Test the following:

- zero queue capacity;
- inbound capacity;
- outbound capacity;
- transcript capacity;
- connect failure;
- send failure;
- receive failure;
- disconnect failure;
- receive buffer too small;
- successful frame send and receive;
- transcript hash or complete transcript behavior.

## 24. Durable FIX Resend and Restart

A FIX adapter may need original outbound frames for resend. The durable store
must distinguish original messages from replay frames.

Before sending a newly sequenced original frame:

1. encode the complete frame;
2. assign the outgoing sequence;
3. record the original frame according to the journal durability contract;
4. send the frame;
5. expose send failure without pretending the provider accepted it.

On resend:

- retrieve original frames by sequence;
- mark replay/poss-dup semantics;
- do not record replay frames as new originals;
- preserve the original application message;
- apply bounded resend action limits;
- fail closed if the required original frame is unavailable.

After restart, restore sequence and resend state from validated durable data.
Document what happens if the store is truncated, corrupt, from the wrong session,
or from a different configuration hash.

## 25. Recovery Reconciliation

Recovery is not just reconnecting a socket. It re-establishes agreement among:

- provider session;
- adapter working-order cache;
- OMS order state;
- durable command journal;
- report deduplicator;
- position ledger;
- drop-copy or independent evidence;
- application checkpoint.

### 25.1 Recovery evidence order

Prefer authoritative evidence in this order:

1. provider open-order and execution query;
2. independent drop copy;
3. durable provider report sequence;
4. local journal;
5. local in-memory state.

Local memory alone cannot prove provider state after a process restart.

### 25.2 Open-order recovery

When recovering open orders:

1. request provider state;
2. capture a provider watermark if available;
3. map each record to a restated canonical event;
4. deduplicate by stable execution/order identity;
5. reconcile local non-terminal orders;
6. identify local orders missing from provider state;
7. preserve uncertainty until policy resolves it;
8. resume command flow only after the configured recovery gate.

### 25.3 Recovery failure

If recovery cannot establish authoritative state:

- keep the adapter degraded;
- reject or hold unsafe new commands according to policy;
- expose the reason;
- retain the evidence needed for operator review;
- do not silently declare all orders cancelled;
- do not automatically retry uncertain commands.

## 26. Observability

Every adapter needs enough information to answer “what happened?” without
turning the hot path into a logging system.

### 26.1 Required counters

At minimum consider:

- messages received;
- messages decoded;
- events emitted;
- events dropped;
- duplicate messages;
- sequence gaps;
- malformed messages;
- provider rejects;
- transport errors;
- reconnect attempts;
- successful reconnects;
- recovery queries;
- recovery mismatches;
- queue high-water mark;
- throttle decisions;
- journal failures;
- p50/p95/p99 latency where measured.

### 26.2 Health versus metrics

Health describes current readiness and meaningful state transitions. Metrics
describe counts and distributions. Do not increment health sequence for every
counter update.

### 26.3 Raw capture

Raw capture is useful for certification and diagnosis but is sensitive and must
be bounded. Define:

- enablement;
- maximum bytes or messages;
- redaction;
- retention;
- access;
- transcript hash;
- behavior when full;
- whether capture affects the hot path.

Never capture credentials or private session material by default.

### 26.4 Protocol diagnostics

Protocol information should be safe, concise, and useful:

- protocol version;
- session id without secret;
- provider route;
- message mode;
- reconnect count;
- last safe error category.

Do not put complete raw messages in health strings.

## 27. Security

Provider adapters process credentials and externally controlled bytes.

### 27.1 Input validation

Validate:

- frame and payload length;
- tag count;
- string length;
- UTF-8 or ASCII constraints;
- numeric ranges;
- enum discriminants;
- timestamps;
- sequence numbers;
- symbol and account identifiers;
- queue capacity;
- pagination limits.

### 27.2 Credential handling

Credentials must:

- come from secure runtime configuration;
- never appear in source examples;
- never appear in Debug output;
- never be included in errors or health;
- never be committed in fixtures;
- be redacted from transcripts;
- be rotatable without source changes where possible.

### 27.3 TLS and certificates

Validate certificates and hostnames. If custom trust roots are supported,
document the exact policy. Do not add an insecure “skip verification” default.

### 27.4 Command safety

Before sending an order:

- ensure the adapter is ready;
- ensure route/account/symbol are allowed;
- ensure capability and precision are valid;
- ensure risk checks already passed;
- ensure the command identity is unique;
- ensure journal policy is satisfied;
- ensure throttling allows the operation.

The adapter should not bypass the OMS risk boundary.

## 28. Performance and Allocation Discipline

A provider adapter should make its performance costs visible.

### 28.1 Hot-path review

For each method, document:

| Method | May allocate? | May block? | May perform I/O? | Bound |
| --- | --- | --- | --- | --- |
| connect | construction/config dependent | usually yes at lifecycle boundary | yes | timeout/retry policy |
| submit | should be bounded | should not wait indefinitely | provider send | encoded frame size |
| cancel | should be bounded | should not wait indefinitely | provider send | encoded frame size |
| amend | should be bounded | should not wait indefinitely | provider send | encoded frame size |
| poll | preferably no allocation | non-blocking contract | receive poll | frames/events per call |
| recover_open_orders | may use control-plane buffers | may wait on provider | yes | pages/orders |
| health | control-plane allocation allowed | no network I/O | no | status fields |

The exact contract must follow the implementation and documentation. Do not
claim zero allocation if a method formats a String or grows a Vec.

### 28.2 Preallocation

Preallocate:

- receive frame storage;
- encode scratch;
- event buffer;
- pending gap frames;
- working-order map;
- resend action storage;
- transcript storage.

Preallocation is not permission to allocate based on untrusted provider sizes.
Apply validated maximums first.

### 28.3 Benchmark design

A useful benchmark identifies:

- provider message shape;
- frame size;
- event count;
- order count;
- route count;
- queue capacity;
- build profile;
- CPU and OS;
- allocator;
- clock source;
- warm-up;
- sample count;
- p50, p95, p99, and maximum;
- allocation count.

Compare before and after. Do not report a single best-case timing.

### 28.4 Performance regression policy

If a change adds:

- a lock;
- a heap allocation;
- a string conversion;
- a clone of a large frame;
- an unbounded collection;
- a blocking wait;
- a second parse;
- a dynamic dispatch in a measured loop;

explain why it is acceptable, isolate it from the hot path, or provide measured
evidence.

## 29. Examples of Adapter Shapes

### 29.1 REST convenience adapter

A REST execution adapter is convenient but usually request/response based.

It should:

- use a separate polling or report stream for authoritative execution events;
- define idempotency keys;
- bound HTTP response size;
- enforce request limits;
- separate send acknowledgement from execution report;
- handle timeout uncertainty through query/reconciliation;
- classify it as `RestConvenience`.

Do not assume a successful HTTP response means a fill.

### 29.2 Streaming WebSocket adapter

A WebSocket adapter should define:

- authentication and listen-key renewal;
- ping/pong;
- message ordering;
- reconnect and subscription restoration;
- sequence gaps;
- duplicate events;
- bounded receive queue;
- provider error frames;
- graceful close.

Do not treat a WebSocket connection as ready until authentication and
subscription acknowledgement are complete.

### 29.3 Native SDK adapter

A native SDK adapter should isolate:

- SDK thread callbacks;
- SDK lifetime and shutdown;
- callback-to-poll queue;
- SDK error codes;
- thread handoff;
- provider object ownership;
- callback reentrancy.

Never call user or engine code directly from an SDK callback unless the thread
and reentrancy contract explicitly permits it. Prefer copying validated data into
a bounded adapter-owned queue.

### 29.4 FIX adapter

A FIX adapter should use the generic session and codec path described above.
The venue-specific module should primarily provide:

- profile;
- configuration;
- mapping;
- transport composition;
- certification scenarios;
- capability declaration.

## 30. Worked Market-Data Skeleton

The following is a design skeleton. Names must be adjusted to the exact
market-data trait in the current crate source; it illustrates ownership and
validation, not a copy-paste provider implementation.

~~~rust
struct ProviderMarketDataAdapter {
    mode: AdapterRuntimeMode,
    state: AdapterConnectionState,
    transport: ProviderTransport,
    subscriptions: Vec<SubscribeReq>,
    pending: Vec<RawEvent>,
    next_sequence: Option<u64>,
    dropped_events: u64,
    gap_count: u64,
}

impl ProviderMarketDataAdapter {
    fn normalize_trade(&mut self, message: ProviderTrade) -> Result<RawEvent, AdapterError> {
        let symbol = self.map_symbol(&message.symbol)?;
        let price = self.normalize_price(message.price)?;
        let size = self.normalize_size(message.size)?;
        self.validate_sequence(message.sequence)?;
        Ok(RawEvent::Trade(TradePrint {
            symbol,
            price,
            size,
            aggressor_side: self.map_side(message.side)?,
            sequence: message.sequence,
            ts_exchange_ns: message.exchange_time_ns,
            ts_recv_ns: self.receive_time_ns(),
        }))
    }

    fn normalize_book(&mut self, message: ProviderBook) -> Result<RawEvent, AdapterError> {
        let symbol = self.map_symbol(&message.symbol)?;
        let price = self.normalize_price(message.price)?;
        let size = self.normalize_size(message.size)?;
        self.validate_sequence(message.sequence)?;
        Ok(RawEvent::Book(BookUpdate {
            symbol,
            side: self.map_side(message.side)?,
            level: message.level,
            price,
            size,
            action: self.map_action(message.action)?,
            sequence: message.sequence,
            ts_exchange_ns: message.exchange_time_ns,
            ts_recv_ns: self.receive_time_ns(),
        }))
    }
}
~~~

A real implementation must add bounded queues, lifecycle transitions, error
mapping, reconnect policy, subscription recovery, tests, and documentation.

## 31. Worked Execution Skeleton

The following skeleton shows the boundaries an execution adapter should keep
separate.

~~~rust
struct ProviderExecutionAdapter<T> {
    transport: T,
    capabilities: ExecutionCapabilities,
    health: ExecutionHealth,
    working_orders: BoundedWorkingOrders,
    throttle: OrderThrottle,
}

impl<T: ProviderTransport> ExecutionAdapter for ProviderExecutionAdapter<T> {
    fn connect(&mut self) -> ExecutionResult<()> {
        self.transport.connect()
            .map_err(|err| ExecutionError::Adapter(err.safe_text()))?;
        self.authenticate()?;
        self.health.connected = true;
        self.health.degraded = false;
        self.health.health_seq = self.health.health_seq.saturating_add(1);
        Ok(())
    }

    fn submit(
        &mut self,
        req: &OrderRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        self.ensure_ready()?;
        self.validate_request(req)?;
        self.throttle.allow_submit()?;
        let command = self.encode_submit(req)?;
        self.transport.send(&command)
            .map_err(|err| ExecutionError::Adapter(err.safe_text()))?;
        Ok(())
    }

    fn poll(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        let mut count = 0;
        while count < out.max_len() {
            match self.transport.poll_report()? {
                None => break,
                Some(report) => {
                    let event = self.map_report(report)?;
                    out.push(event)?;
                    count += 1;
                }
            }
        }
        Ok(count)
    }
}
~~~

The skeleton omits provider details and should not be copied without verifying
the actual trait, error types, and buffer semantics. In particular, the
implementation must define what happens when `map_report` fails after
the provider report has been consumed.

## 32. Documentation Requirements for a New Adapter

Add or update all of the following:

### 32.1 Crate and module documentation

Document:

- provider identity and supported environments;
- feature flag;
- transport;
- authentication assumptions;
- capabilities;
- symbol and numeric mapping;
- lifecycle;
- reconnect and recovery;
- rate limits;
- health fields;
- error mapping;
- allocation and blocking;
- certification status;
- unsupported behavior.

### 32.2 Public API documentation

Every public adapter type, method, field, enum, variant, constant, and feature
must define what it means and what it does. Include:

- units;
- defaults;
- valid ranges;
- ownership;
- thread behavior;
- failure behavior;
- compatibility notes;
- example construction where meaningful.

### 32.3 Operational documentation

Document:

- credentials and secret setup;
- endpoint configuration;
- clock requirements;
- provider permissions;
- session schedule;
- deployment topology;
- reconnect alerting;
- queue and gap alerts;
- recovery runbook;
- raw capture policy;
- data retention.

### 32.4 Certification evidence

For a certified provider integration, record:

- provider and environment;
- protocol/profile revision;
- adapter version;
- configuration hash;
- scenario inventory;
- transcript hash;
- performance evidence;
- known limitations;
- approval or certification date;
- raw artifact retention location.

Do not publish private credentials or restricted provider documents.

## 33. Testing Commands

Run focused tests first:

~~~bash
cargo test -p of_execution
cargo test -p of_execution_adapters
cargo test -p of_fix
cargo test -p of_adapters
~~~

Run provider feature tests:

~~~bash
cargo test -p of_adapters --features rithmic
cargo test -p of_adapters --features cqg
cargo test -p of_adapters --features "cqg cqg_proto"
cargo test -p of_adapters --features binance
~~~

Run static and public-surface checks:

~~~bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 tools/provider_conformance.py --help
python3 tools/check_api_manifest.py
python3 tools/check_binding_parity.py
~~~

Run the broad suite:

~~~bash
cargo test --workspace --all-features
cargo test --workspace --no-default-features
~~~

Run documentation checks:

~~~bash
python3 tools/docs_coverage.py --enforce
python3 tools/generate_docs_inventory.py --check
python3 tools/generate_rust_surface.py --check
python3 tools/generate_rust_values.py --check
python3 tools/generate_binding_surface.py --check
python3 tools/generate_package_matrix.py --check
python3 tools/generate_crate_pages.py --check
python3 tools/enrich_api_reference.py --check
python3 tools/enrich_handbook_public_lists.py --check
.venv/bin/mkdocs build --strict --site-dir /tmp/orderflow-docs-check
~~~

## 34. CI and Feature-Matrix Integration

A new provider feature is incomplete until CI can compile and test it.

Update:

- the provider crate `Cargo.toml`;
- workspace dependency declarations if applicable;
- feature forwarding;
- documentation feature matrix;
- provider conformance invocation;
- CI feature matrix;
- packaging or native artifact workflow when applicable;
- release notes and README.

The default feature set should remain intentionally small. Optional provider SDK
dependencies must not become mandatory for users who only need core analytics
or unrelated adapters.

Test at least:

- default features;
- no default features;
- provider feature alone;
- provider feature combinations that are supported;
- `--all-features`;
- MSRV where the dependency supports it.

## 35. Review Checklist: Design

- [ ] The adapter family is correct.
- [ ] Provider evidence is recorded.
- [ ] Ownership boundaries are explicit.
- [ ] Canonical identities are preserved.
- [ ] Symbol mapping is deterministic.
- [ ] Numeric normalization is exact or rejects safely.
- [ ] Capability flags are honest.
- [ ] Unsupported operations fail before provider I/O.
- [ ] Transport and protocol responsibilities are separated.
- [ ] Lifecycle states are explicit.
- [ ] Reconnect and backoff are bounded.
- [ ] Recovery handles uncertain commands.
- [ ] Every queue and capture buffer has a bound.
- [ ] Backpressure behavior is documented.
- [ ] Provider errors and business rejects are distinguished.
- [ ] Timestamps and sequences are preserved.
- [ ] Secrets are excluded from diagnostics.
- [ ] Hot-path allocation and blocking are reviewed.

## 36. Review Checklist: Implementation

- [ ] Public types and methods have complete documentation.
- [ ] No provider SDK type leaks through reusable domain APIs.
- [ ] No hidden global mutable state was added.
- [ ] No unsafe conversion lacks validation.
- [ ] No provider-controlled length is used without a bound.
- [ ] No panic path is reachable from malformed provider data.
- [ ] No old public signature or ABI layout changed.
- [ ] No existing error code or enum value was reordered.
- [ ] Health sequence changes only on meaningful health transitions.
- [ ] Poll returns accurate event counts.
- [ ] Buffer-full behavior retains unread events.
- [ ] Disconnect does not silently become an empty poll.
- [ ] Recovery does not retry uncertain orders blindly.
- [ ] Shutdown closes transport and joins owned workers.
- [ ] Logs and status output redact secrets.

## 37. Review Checklist: Tests

- [ ] Construction and configuration validation.
- [ ] Connect, reconnect, and stop.
- [ ] Empty and burst input.
- [ ] Capability rejection.
- [ ] Numeric precision and bounds.
- [ ] Identifier mapping and collisions.
- [ ] Sequence gaps and duplicates.
- [ ] Out-of-order behavior.
- [ ] Timestamp and freshness behavior.
- [ ] Buffer capacity and overflow.
- [ ] Throttle and retry behavior.
- [ ] Malformed provider input.
- [ ] Provider business rejects.
- [ ] Acknowledgement, fill, partial fill, cancel, amend, and recovery.
- [ ] Journal failure and uncertain send.
- [ ] Health transitions and operational status.
- [ ] Certification transcript and failure injection.
- [ ] Deterministic replay.
- [ ] Performance evidence where hot-path code changed.

## 38. Review Checklist: Documentation and Release

- [ ] Crate README explains end-to-end setup.
- [ ] Handbook reference explains concepts and behavior.
- [ ] Every listed public item has a definition.
- [ ] Feature flag and default behavior are documented.
- [ ] Configuration fields and secret handling are documented.
- [ ] Capability matrix is current.
- [ ] Recovery and runbook are documented.
- [ ] Examples do not use credentials.
- [ ] Generated inventories are refreshed.
- [ ] Strict docs build passes.
- [ ] Changelog/release notes explain user-visible behavior.
- [ ] Adapter certification status is honest.
- [ ] Commit is atomic and uses Conventional Commit format.

## 39. Troubleshooting

### 39.1 Adapter remains disconnected

Check:

1. endpoint and environment;
2. credentials and permissions;
3. transport TLS validation;
4. provider session schedule;
5. logon or authentication response;
6. health state and last error;
7. reconnect attempt and backoff;
8. whether the adapter was explicitly connected.

A TCP connection alone does not establish readiness.

### 39.2 Adapter is connected but degraded

Inspect:

- sequence gaps;
- stale message age;
- queue depth;
- dropped event count;
- malformed frames;
- provider throttle responses;
- heartbeat/test-request state;
- recovery completion;
- capability or account status.

Do not clear degraded state manually without satisfying the recovery gate.

### 39.3 Orders are rejected locally

Determine whether the rejection is:

- route not found;
- capability mismatch;
- precision or quantity validation;
- risk rejection;
- disconnected state;
- throttle;
- provider profile validation.

A local rejection should not produce provider I/O. Inspect canonical request
fields before inspecting provider logs.

### 39.4 Orders are accepted by transport but missing

Treat this as uncertain, not rejected. Preserve client id and journal evidence,
query provider state, inspect drop copy, reconcile, and retry only after absence
is authoritative.

### 39.5 Reports duplicate after reconnect

Check provider resend semantics, duplicate flags, execution-id scope, report
deduplication horizon, and checkpoint restoration. Do not solve duplicate
reports by dropping all reports with the same timestamp.

### 39.6 Book has a crossed or stale state

Check snapshot/delta ordering, sequence continuity, symbol mapping, delete
semantics, level indexing, provider reset events, and receive-time freshness.
Resynchronize rather than hiding a gap.

### 39.7 Poll returns zero unexpectedly

Distinguish idle from disconnected. Inspect transport state, queue depth,
subscription acknowledgement, provider heartbeat, and mode. Zero events is not
the same as a healthy stream.

### 39.8 FIX resend fails

Check sequence restoration, original-message journal durability, session
identity, resend range bounds, missing frames, gap-fill rules, and whether replay
frames were incorrectly recorded as new originals.

### 39.9 Certification transcript differs

Compare provider profile, FIX version, session identifiers, clock formatting,
sequence state, configuration hash, frame bytes, failure injection, and
transcript bounds. Normalize only fields explicitly marked volatile.

### 39.10 Tests hang

Use bounded deadlines. Ensure:

- transport is disconnected;
- worker receives stop;
- worker is joined;
- queues are drained or intentionally abandoned;
- no test waits for an event that the fake provider never scripts;
- shared locks recover from poisoning.

## 40. Adapter Completion Definition

An adapter is complete only when all of the following are true:

1. its provider scope and evidence are documented;
2. its canonical mappings are explicit;
3. capabilities are honest and tested;
4. transport and lifecycle states are bounded;
5. errors distinguish transport, validation, business reject, and journal failure;
6. sequence, timestamp, duplicate, and gap policy is defined;
7. reconnect and recovery are deterministic;
8. all buffers and queues have visible bounds;
9. security and secret handling are reviewed;
10. focused and workspace tests pass;
11. feature-matrix CI passes;
12. documentation and generated references are current;
13. certification status is accurate;
14. performance evidence exists for claimed low-latency paths;
15. an operator can diagnose and recover the adapter without reading source.

The adapter is a public boundary. Treat every provider assumption as a contract,
every external message as untrusted input, every uncertain command as a recovery
case, and every undocumented behavior as unfinished work.
