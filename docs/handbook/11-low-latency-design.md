# Low-Latency Design

This chapter explains what the project optimizes for and where the boundaries
are. Low latency is not just "make it async." In an OMS, predictable ordering
and failure behavior matter as much as raw speed.

## Hot Path Rules

The execution hot path is:

```mermaid
flowchart LR
  Command[submit / cancel / amend]
  Risk[Risk check]
  Adapter[ExecutionAdapter]
  Buffer[ExecutionEventBuffer]
  State[OrderStateMachine]

  Command --> Risk --> Adapter --> Buffer --> State
```

Rules:

- use typed structs, not JSON,
- use integer-normalized prices and quantities,
- use fixed-size IDs,
- use caller-owned buffers,
- keep queues bounded,
- reject instead of silently dropping,
- avoid background mutation of order state,
- preserve deterministic command order.

## What Is Not Hot Path

These are important but not hot-path operations:

- dashboard rendering,
- JSON snapshot formatting,
- docs and API examples,
- release packaging,
- long-form audit export,
- historical analysis over stored files.

Do not optimize these at the expense of execution correctness.

## Synchronous Core

The synchronous `ExecutionEngine` is intentionally single-owner. It avoids:

- lock contention inside order state,
- async scheduling jitter,
- hidden task cancellation,
- unordered report application,
- accidental concurrent state mutation.

If a caller wants concurrency, use the concurrent worker wrapper.

## Concurrent Worker

The concurrent worker uses bounded standard-library channels. It gives:

- concurrent command producers,
- explicit backpressure,
- one owner thread,
- deterministic event application,
- simple crash/replay reasoning.

It does not use Tokio. That is intentional. A Tokio-based adapter can still
exist outside the engine and bridge into typed events, but the state machine
should remain deterministic.

## Bounded Queues

Unbounded queues hide risk. A venue outage plus unbounded submit queue can turn
into a delayed burst of stale commands.

Bounded queues force the caller to decide:

- retry,
- drop strategy intent,
- pause strategy,
- trip a circuit breaker,
- alert an operator.

## Fixed-Size IDs

`FixedAscii<N>` avoids allocation and enforces FFI-safe identity fields. It also
forces each provider adapter to decide how much identifier capacity it needs.

Do not replace fixed IDs with `String` in hot-path structs unless you are
creating a separate non-hot-path representation.

## Integer Normalization

Prices and quantities are integers. Symbol metadata is responsible for
denormalization.

Benefits:

- replay determinism,
- no floating-point drift,
- stable persistence,
- easier cross-language comparison.

## Risk Before Adapter

Pre-trade risk should happen before the adapter sees a request. This avoids:

- sending orders that are locally known to be invalid,
- venue-side rejects that could have been prevented,
- inconsistent local/journal state.

## Journaling

Journaling is part of reliability, not just compliance. The minimum production
flow is:

1. record accepted local command,
2. send to adapter,
3. record reports,
4. replay after restart,
5. reconcile with venue.

`FileExecutionJournal` is an additive baseline. Higher-performance deployments
can implement `ExecutionJournal` with a WAL, mmap file, or database.

## Metrics To Watch

Important low-latency metrics:

- command queue depth,
- report queue depth,
- submit-to-ack latency,
- cancel latency,
- worker loop lag,
- adapter reconnect count,
- risk rejects by reason,
- report fanout drops,
- journal write latency,
- venue sequence gap count.

`ExecutionTelemetry` is the additive in-crate helper. Production deployments
should export these metrics to their own telemetry system.

## Sharding

Sharding should preserve deterministic ordering within each route/account/symbol
scope. Good sharding keys:

- `(route_id, account_id, symbol)`,
- `(venue, account_id)`,
- provider session id.

Avoid sharding by random command id. That breaks order lifecycle ordering.

## Adapter Guidance

Adapters should:

- parse provider messages into canonical events quickly,
- use bounded internal queues,
- surface lifecycle state,
- expose precise capabilities,
- never call back into strategy code while holding adapter state locks,
- avoid string formatting on the command path,
- map provider rejects into structured reasons where possible.
