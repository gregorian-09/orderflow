# OMS Architecture

This chapter explains the execution and order-management subsystem as a system,
not just as individual API types.

The OMS is additive to the market-data runtime. It does not replace
`of_runtime`, and it does not mix order execution state with analytics state.
Strategy code can consume analytics from `of_runtime` and send orders through
the execution APIs, but those two domains remain separate.

## Layer Map

```mermaid
flowchart TD
  Strategy[Strategy / Host language]
  Api[C / Python / Java / Rust execution API]
  Engine[of_execution::ExecutionEngine<br/>or ConcurrentExecutionEngine]
  Risk[RiskCheck<br/>route-scoped limits]
  Journal[ExecutionJournal]
  State[OrderStateMachine]
  Adapter[ExecutionAdapter]
  Venue[Venue / broker / simulator]

  Strategy --> Api --> Engine
  Engine --> Risk
  Engine --> Journal
  Engine --> State
  Engine --> Adapter --> Venue
```

## Core Separation

| Layer | Responsibility |
| --- | --- |
| `of_execution_core` | Canonical typed execution model |
| `of_execution` | Routing, risk context, state, journals, workers, OMS helpers |
| `of_execution_adapters` | Provider-specific adapter scaffolds |
| `of_ffi_c` | Stable native ABI for execution handles |
| Python/Java bindings | Ergonomic wrappers over the C ABI |

This split is deliberate. The execution core should not know about sockets,
worker threads, provider APIs, or host-language wrappers. The engine should not
know how a specific broker encodes a request. Bindings should not duplicate
order-state logic.

## Certification Boundary

The deterministic certification venue sits behind the unchanged
`ExecutionAdapter` boundary. It validates OMS behavior before a provider
adapter is connected to official certification infrastructure.

```mermaid
flowchart LR
  Script[Bounded certification script]
  Cert[CertificationVenue]
  History[Bounded sequenced report history]
  AdapterTrait[ExecutionAdapter contract]
  OMS[ExecutionEngine / recovery harness]
  Evidence[Transcript + 18-kind coverage]
  Official[Provider certification environment]

  Script --> Cert
  Cert --> History
  History --> Cert
  Cert --> AdapterTrait --> OMS
  Cert --> Evidence
  Evidence --> Official
```

This division matters: deterministic local coverage catches state-machine,
backpressure, duplicate, ordering, race, and recovery defects; official venue
certification proves the actual transport and counterparty profile. Neither
substitutes for the other.

## Synchronous Engine

`ExecutionEngine` is the deterministic core. It owns mutable order state and is
driven by method calls:

- `start`
- `submit`
- `cancel`
- `amend`
- `poll`
- `recover_open_orders`

It requires `&mut self`, which means callers cannot mutate the engine from two
Rust threads at the same time without adding an owner. That is a feature, not a
limitation. OMS state should not be mutated concurrently without explicit
ordering.

## Concurrent Worker

`ConcurrentExecutionEngine` solves concurrent producer access without making
the state machine concurrent.

```mermaid
flowchart LR
  A[Producer thread A]
  B[Producer thread B]
  C[Producer thread C]
  CQ[Bounded command queue]
  Worker[Worker thread<br/>owns ExecutionEngine]
  RQ[Bounded report queue]

  A --> CQ
  B --> CQ
  C --> CQ
  CQ --> Worker
  Worker --> RQ
```

Properties:

- many producer handles can enqueue commands,
- command queue is bounded,
- report queue is bounded,
- one worker owns adapter and order state,
- reports include command sequence and events,
- no Tokio runtime is required,
- no order-state mutation happens in parallel.

This model keeps latency predictable and makes replay/audit reasoning simpler.

## Route Scoping

Execution routes are keyed by:

```mermaid
flowchart LR
  RouteId[route_id]
  AccountId[account_id]
  Venue[venue]
  Instrument[instrument]
  Key[RouteKey]

  RouteId --> Key
  AccountId --> Key
  Venue --> Key
  Instrument --> Key
```

The engine indexes these keys internally. Risk calculations such as open order
count and open notional are scoped to the matching route. This avoids accidental
cross-symbol contamination.

Example:

- ES route has one open order and max open orders of one.
- NQ route has no open orders and max open orders of one.
- A second ES order is rejected.
- A first NQ order is allowed.

## Command Flow

New order flow:

```mermaid
sequenceDiagram
  participant Strategy
  participant Engine
  participant Risk
  participant Journal
  participant State as OrderStateMachine
  participant Adapter

  Strategy->>Engine: OrderRequest
  Engine->>Engine: Validate request shape
  Engine->>Engine: Find route
  Engine->>Risk: Build context and check limits
  Risk-->>Engine: Allow or reject
  Engine->>Journal: Record command
  Engine->>State: Create local pending state
  Engine->>Adapter: Submit request
  Adapter-->>Engine: ExecutionEvent values
  Engine->>State: Apply events
  Engine->>Journal: Record events
  Engine-->>Strategy: Events or command report
```

Cancel/replace flow is similar, but starts by verifying the original client
order id is known locally.

## Event Flow

Execution events can come from:

- synchronous adapter responses,
- adapter `poll`,
- recovery/restatement,
- local risk rejection,
- local lifecycle/degradation handling.

Every event should be canonical `ExecutionEvent`. Adapters should not leak
provider-specific report structs past the adapter boundary.

## Journaling and Recovery

The journal records commands and events. A durable journal is the foundation for
crash recovery:

1. replay prior commands/events,
2. rebuild local state,
3. reconnect adapter,
4. request venue open orders,
5. reconcile local vs venue state,
6. emit restatements for differences.

`FileExecutionJournal` is the current append-only durable implementation. It is
small and deterministic; production deployments may replace it with an fsync
policy, WAL, or database-backed journal through the `ExecutionJournal` trait.

## Reconciliation

`reconcile_open_orders(local, venue)` compares two sets of `OrderState` values
and preserves the original simple classification:

- `Matched`
- `VenueOnly`
- `LocalOnly`
- `RestateFromVenue`

The function does not mutate state. It reports differences so the caller can
decide whether to restate, cancel, alert, or halt trading.

`reconcile_open_orders_detailed(local, venue)` adds production-oriented issue
classification:

- `QuantityMismatch`
- `StatusMismatch`
- `PriceMismatch`
- `Unknown`

`ReconciliationPolicy` maps each issue to a host action. The default is fail
closed. Other actions let the host accept venue truth, cancel venue-only
orders, restate venue-only orders locally, or require operator approval. The
policy evaluator returns `ReconciliationPolicyDecision`, which keeps
submissions disabled while any host action is still required.

`ExecutionEngine::reconcile_open_orders_with(venue)` and
`ExecutionEngine::evaluate_reconciliation(venue, policy)` apply those helpers
to the engine's current local non-terminal order states.

## Independent Drop Copy

The primary order-entry session is not the only source of execution truth.
`DropCopyAdapter` models a separate provider session that maps independent
reports into canonical `DropCopyReport` values. Keeping it separate avoids
coupling order submission availability to the credentials, sequence state, or
recovery state of the independent evidence channel.

```mermaid
sequenceDiagram
    participant Strategy
    participant OMS as Primary OMS session
    participant Venue
    participant Drop as Drop-copy session
    participant Reconciler
    Strategy->>OMS: OrderRequest
    OMS->>Venue: Provider order
    Venue-->>OMS: Primary ExecutionEvent
    Venue-->>Drop: Independent report
    Drop->>Reconciler: DropCopyReport
    OMS->>Reconciler: Local OrderState snapshot
    Reconciler-->>OMS: DropCopyObservation + metrics
```

`DropCopyReconciler` provides four deterministic stages:

1. deduplicate by source plus report id, or source plus sequence;
2. classify timestamp and cumulative-fill regressions under an explicit late
   policy;
3. correlate venue order id first and client-order aliases second; and
4. compare identity, routing, status, fill quantities, leaves, and average
   price without mutating OMS state.

Duplicate identity retention and report/progress queues are bounded. Capacity
exhaustion is observable, not silently expanded. Report timestamps are supplied
by the adapter or host, so the hot path does not read clocks. Policy decisions,
WAL writes, alert delivery, JSON export, and operator actions remain outside the
reconciler and can use the existing safety and recovery gates.

## Scoped Kill-Switch Control Plane

`KillSwitchRegistry` is a separate control-plane state machine. It complements
the route-level risk flag without changing current `ExecutionEngine`
construction or adapter trait requirements.

```mermaid
flowchart TD
    Recovery[Restore WAL/checkpoint state] --> Certainty{State confirmed?}
    Certainty -- No --> FailClosed[Reject new orders]
    Certainty -- Yes --> Match[Match active scopes]
    Request[OrderRequest + position + session] --> Match
    Match --> Decision[KillSwitchDecision]
    Decision --> Risk[Normal pre-trade risk]
    Decision --> Pause[Pause strategy]
    Decision --> Stop[Stop adapter/session]
    Activation[KillSwitchActivation] --> Select[Select open orders]
    Open[Open-order contexts] --> Select
    Select --> Targets[Bounded affected-order buffer]
    Targets --> Cancel[Host sends cancels]
    Cancel --> Progress[Cancel result events]
    Progress --> Clear{Clean completion?}
    Clear -- Yes --> Cleared[Clear switch]
    Clear -- No --> Override[Explicit forced clear or remain blocked]
```

The decision path is synchronous and allocation-free after registry
construction. Scope matching uses fixed-size identifiers. Emergency actions do
not invoke user callbacks or provider code while registry state is borrowed.
The host journals each typed event, dispatches selected cancels, records unique
results, and performs adapter shutdown outside the registry.

Recovery is conservative. Unknown switch state blocks new orders while keeping
cancel flow available. A switch requiring cancellation cannot be cleared
normally until every captured affected order succeeds. Truncated target output,
failed cancels, or incomplete attempts remain visible and require an explicit
forced-clear event rather than silently reopening flow.

## Production Risk Policy Layer

The additive production-risk engine composes policies across organizational and
market scopes without changing the existing engine constructor.

```mermaid
sequenceDiagram
    participant Strategy
    participant Host as OMS host worker
    participant Ledger as Position/PnL and health state
    participant Risk as ProductionRiskEngine
    participant Audit as Decision journal
    participant OMS as ExecutionEngine
    Strategy->>Host: Submit / amend / cancel
    Host->>Ledger: Read authoritative context
    Ledger-->>Host: Exposure, PnL, reference, health
    Host->>Risk: ProductionRiskCommand + context
    Risk->>Risk: Match ordered scopes and update rate windows
    Risk-->>Host: Explainable decision
    Host->>Audit: Record decision
    alt journal failed or decision rejected
        Host-->>Strategy: Reject and expose reason
    else allowed and retained
        Host->>OMS: Existing typed request
    end
```

Global and narrow policies are cumulative. A host can use a global operational
baseline, an account credit limit, a strategy loss limit, a route message-rate
limit, a venue restriction, and symbol or product-group collars at the same
time. Stable priority and id ordering makes the primary rejection deterministic
under replay.

The engine accepts host-owned state rather than borrowing the OMS internals.
That boundary avoids locks and lets deployments source exposure from a richer
ledger, independent drop copy, or broker reconciliation. It also makes
availability explicit: missing authoritative state is a risk input, not an
implicit zero.

Policy installation is a control-plane operation and may allocate bounded rate
storage. Evaluation uses fixed-size ids and preallocated queues, does not read
clocks or perform I/O, and belongs on the single-owner command worker. Sharded
deployments should route a command and every policy state affecting it to one
deterministic owner or evaluate global limits in a separate serialized tier.

## Safety Policies

`RouteSafetyPolicy` and `DisconnectPolicy` describe what should happen when
the adapter disconnects or a kill switch is active.

Possible policies:

- hold working orders,
- reject new orders,
- cancel open orders,
- freeze all commands,
- allow cancels while killed.

Policies are separate from adapter code so users can apply them consistently
across FIX, REST, WebSocket, or broker SDK adapters.

## Order Intent And Child Ownership

Execution algorithms decide when and where to release liquidity; the OMS owns
the resulting order tree. `OrderIntentLifecycle` is that ownership boundary.

```mermaid
stateDiagram-v2
    [*] --> Pending
    Pending --> Active: activate after risk acceptance
    Pending --> Rejected
    Active --> Paused
    Paused --> Active
    Active --> PendingCancel: cancel tree
    Paused --> PendingCancel: cancel tree
    Failed --> PendingCancel: emergency cancel tree
    Active --> Completed: aggregate fill reaches target
    PendingCancel --> Completed: final fill reaches target
    PendingCancel --> Cancelled: every child terminal
    Active --> Failed: uncertain state
    Paused --> Failed: uncertain state
    Recovering --> Active: reconciliation approved
```

```mermaid
sequenceDiagram
    participant Planner as Strategy / algo planner
    participant Tree as OrderIntentLifecycle
    participant Controls as Kill switch / risk / WAL
    participant OMS as ExecutionEngine
    participant Venue
    Planner->>Tree: Plan bounded child
    Tree-->>Planner: OmsChildOrder
    Planner->>Controls: Submit child request
    Controls->>OMS: Allowed and journaled request
    OMS->>Venue: Provider order
    Venue-->>OMS: Execution report
    OMS-->>Tree: Canonical ExecutionEvent
    Tree->>Tree: Validate and aggregate fills/leaves
```

This separation prevents planners from becoming a second order-state machine.
The algorithm crate may produce TWAP, VWAP, POV, SOR, iceberg, or other plans,
but each plan becomes an OMS child and remains subject to normal controls.
Provider parent tags are optional metadata; canonical child client ids remain
the execution-correlation authority.

The tree is single-owner and sequence-driven. Child maps, indexes, and cancel
scratch are reserved at construction. Recovery snapshot creation allocates and
belongs to the control plane; planning, state transition, report folding, and
cancel selection do not grow storage after configured capacity is reached.

## Idempotency Boundary

Command queue delivery, process restart, network timeout, and FIX resend can
all create retries. The OMS must distinguish a retry of the same intent from a
new intent without assuming that a missing response means the venue did
nothing.

```mermaid
flowchart TB
    Request[Strategy request ID + command ID + client order ID]
    Guard[Bounded IdempotencyRegistry]
    WAL[Checksummed OMS WAL]
    Send[Adapter send + provider command ID]
    Venue[Venue / broker]
    Reports[Primary and drop-copy reports]
    Dedup[ExecutionReportDeduplicator]
    State[Order tree and position ledger]
    Recovery[Checkpoint + reconciliation]

    Request --> Guard
    Guard -->|new exact intent| WAL --> Send --> Venue
    Guard -->|matching retry| Request
    Guard -->|parameter mismatch| Recovery
    Venue --> Reports --> Dedup --> State
    Guard --> Recovery
    Dedup --> Recovery
    Recovery -->|authoritative absent result| Guard
    Recovery -->|authoritative present result| State
```

`IdempotencyRegistry` owns command admission only. The host owns durability and
side effects and advances state after each successful boundary:

1. reserve the scoped request and complete semantic command;
2. append the command/correlation to the WAL;
3. mark it journaled;
4. send using a stable adapter/FIX identity;
5. mark it sent;
6. complete it from authoritative evidence.

A disconnect after step 4 is not a retry instruction. Mark the command
`RecoveryPending`, query/reconcile venue and drop-copy state, and only then
either complete the original record or release its exact retained command for
retry. Restoring a checkpoint converts every non-terminal command to that same
fail-closed recovery state.

The registry does not evict command identities automatically. Capacity
exhaustion blocks new admission, because silent eviction could turn a delayed
retry into a second live order. Terminal retirement is an explicit operator or
retention-policy action after durable archival and expiry of every upstream
retry window.

Execution reports have a different boundedness policy. Their source-scoped
identity window is FIFO and exposes eviction metrics. Capacity must exceed the
maximum provider replay, FIX resend, and delayed drop-copy horizon. Its
checkpoint is part of the same recovery generation as order and position
state, preventing a valid state checkpoint from being paired with an older
deduplication horizon.

## Position Ledger

`PositionLedger` folds trade events into position state. It is intentionally
small:

- net quantity,
- buy quantity,
- sell quantity,
- gross notional,
- average price.

The ledger is not a complete accounting system. It is the OMS-side building
block for exposure checks, strategy attribution, and reconciliation with broker
positions.

For production risk and recovery, use the additive
`ProductionPositionLedger`. It preserves exact normalized open cost and keeps
settlement currency, contract multiplier, realized/unrealized PnL, commissions,
fees, cash effects, mutation sequence, and fill attribution explicit.

```mermaid
sequenceDiagram
    participant WAL as OMS WAL sequence
    participant Ledger as ProductionPositionLedger
    participant Risk as ProductionRiskEngine
    participant Store as Checkpoint store
    participant External as Broker / clearing report
    participant Reconcile as Reconciliation control plane
    WAL->>Ledger: Scoped fill or authorized adjustment
    Ledger->>Ledger: Deduplicate and fold exact cost
    Ledger-->>Risk: Position, exposure, PnL context
    Ledger->>Store: Versioned checksummed checkpoint
    Store-->>Ledger: Validate and restore
    External->>Reconcile: Position snapshots
    Ledger->>Reconcile: Local snapshots
    Reconcile-->>Risk: Match or fail-closed mismatch condition
```

The mutation sequence must share an ordering authority with the execution WAL.
A checkpoint covers every mutation through `last_sequence`; recovery restores
it and replays only later records. The checkpoint also carries the complete
retained identity set so replay and reconnect retries cannot double-count a
fill.

Broker/clearing snapshots are evidence, not direct mutations. Reconciliation
first reports differences under explicit tolerances. Host policy then chooses
whether to block, investigate, apply an authorized correction, or accept a new
opening balance. This keeps external surprises from silently rewriting risk
state.

## Generalized Recovery Reconciliation

`OmsReconciliationCoordinator` is the final fail-closed gate after component
recovery. It requires deployment-selected evidence from local state, WAL,
checkpoint, adapter recovery, independent drop copy, broker positions, and the
local position ledger. Every source supplies integrity status, sequence, as-of
time, and claimed row counts.

```mermaid
sequenceDiagram
    participant Host
    participant Sources as Recovery evidence sources
    participant Cycle as OmsReconciliationCoordinator
    participant Policy
    participant Operator
    Host->>Cycle: begin_cycle(expected WAL sequence)
    Sources->>Cycle: integrity watermarks
    Sources->>Cycle: order snapshots / position report
    Cycle->>Cycle: classify missing, stale, corrupt, duplicate, and state mismatch
    Cycle->>Policy: bounded machine-readable findings
    alt clean
        Policy-->>Host: submissions_enabled
    else automated action configured
        Policy-->>Host: cancel observed-only / restate local / accept truth
        Host->>Cycle: start new verification cycle
    else approval required
        Policy-->>Operator: evidence and required action
        Operator-->>Host: approve or remain blocked
    end
```

The coordinator never edits state. A policy action is an obligation that must
complete before a fresh cycle proves convergence. This separates evidence,
decision, mutation, and verification and prevents partial recovery from being
mistaken for readiness.

## Sharding

`ShardRouter` maps route/account/symbol keys to shard indexes. A sharded OMS can
run one worker per shard:

- deterministic within a shard,
- parallel across independent shards,
- useful for many symbols or venues.

The current helper gives deterministic routing. A full sharded runtime can be
built additively around it.

## Binding Model

The C ABI exposes separate handles for:

- synchronous execution engine,
- concurrent execution engine.

Python and Java mirror this separation:

- Python: `ExecutionEngine`, `ConcurrentExecutionEngine`
- Java: `OrderflowExecutionEngine`, `ConcurrentOrderflowExecutionEngine`

Existing synchronous APIs are not changed.
