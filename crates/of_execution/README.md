# `of_execution`

[![Crates.io](https://img.shields.io/crates/v/of_execution.svg)](https://crates.io/crates/of_execution)
[![Docs.rs](https://docs.rs/of_execution/badge.svg)](https://docs.rs/of_execution)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](https://opensource.org/license/mit)

`of_execution` is the execution routing and OMS crate for Orderflow. It builds
on `of_execution_core` by adding adapter contracts, route configuration,
bounded event buffers, simulated execution, journals, route-scoped risk,
concurrent command ownership, and reusable order-management helpers.

The crate is additive to the analytics runtime. It does not change market-data
adapter behavior and does not merge execution state into `of_runtime`.
Strategies can read analytics from `of_runtime` and send orders through
`of_execution`, while those two domains stay separate.

## First Release: 0.1.0

`of_execution` publishes as `0.1.0` inside the Orderflow `0.4.0` release. This
is intentional. The market-data runtime and bindings are on the established
`0.4.0` line; execution routing, adapter traits, journals, and OMS helpers are
new public crate surfaces and should carry their own compatibility signal.

Versioning rules:

- `of_execution` depends on `of_execution_core = 0.1`;
- existing analytics crates do not depend on `of_execution`;
- `of_ffi_c 0.4.0`, Python `0.4.0`, and Java `0.4.0` expose execution through
  additive handles/classes backed by this crate;
- future `0.1.x` releases should prefer additive changes and clear migration
  notes for adapter authors.

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

Independent drop copy:

- [`DropCopySourceId`]
- [`DropCopyReportId`]
- [`DropCopyReport`]
- [`DropCopyReportBuffer`]
- [`DropCopySourceState`]
- [`DropCopySourceHealth`]
- [`DropCopyAdapter`]
- [`InMemoryDropCopyAdapter`]
- [`DropCopyLateReportPolicy`]
- [`DropCopyDisposition`]
- [`DropCopyCorrelation`]
- [`DropCopyIssueFlags`]
- [`DropCopyObservation`]
- [`DropCopyMetricsSnapshot`]
- [`DropCopyReconciler`]

Scoped kill switches:

- [`KillSwitchId`]
- [`KillSwitchSessionId`]
- [`KillSwitchActorId`]
- [`KillSwitchSourceKind`]
- [`KillSwitchSource`]
- [`KillSwitchScope`]
- [`KillSwitchMode`]
- [`KillSwitchReasonCode`]
- [`KillSwitchStateCertainty`]
- [`KillSwitchActivation`]
- [`KillSwitchCancelResult`]
- [`KillSwitchClear`]
- [`KillSwitchEventKind`]
- [`KillSwitchCancelOutcome`]
- [`KillSwitchEvent`]
- [`KillSwitchOrderContext`]
- [`KillSwitchAffectedOrder`]
- [`KillSwitchAffectedOrderBuffer`]
- [`ActiveKillSwitch`]
- [`KillSwitchDecisionReason`]
- [`KillSwitchDecision`]
- [`KillSwitchRegistry`]
- [`KillSwitchError`]

Production risk:

- [`ProductionRiskPolicyId`]
- [`RiskInstrumentGroupId`]
- [`ProductionRiskScope`]
- [`RiskTradingWindow`]
- [`ProductionRiskLimits`]
- [`ProductionRiskPolicy`]
- [`ProductionRiskCommandKind`]
- [`ProductionRiskCommand`]
- [`ProductionRiskContext`]
- [`ProductionRiskReason`]
- [`ProductionRiskJournalStatus`]
- [`ProductionRiskDecision`]
- [`ProductionRiskError`]
- [`ProductionRiskJournalError`]
- [`ProductionRiskDecisionJournal`]
- [`InMemoryProductionRiskJournal`]
- [`ProductionRiskEngine`]

Routing and risk:

- [`RouteConfig`]
- [`RouteKey`]
- [`AllowAllRiskGate`]

Journaling:

- [`JournalCommandKind`]
- [`JournalRecord`]
- [`ExecutionJournal`]
- [`InMemoryJournal`]
- [`WalJournalConfig`]
- [`WalReplayResult`]
- [`WalJournalMetrics`]
- [`WalExecutionJournal`]
- [`WalSegmentConfig`]
- [`WalSegmentMetadata`]
- [`WalSegmentManifest`]
- [`WalSegmentIntegrityReport`]
- [`SegmentedWalExecutionJournal`]
- [`CheckpointPosition`]
- [`ExecutionCheckpoint`]
- [`CheckpointPolicy`]
- [`CheckpointConfig`]
- [`CheckpointManifest`]
- [`ExecutionCheckpointStore`]
- [`FileExecutionCheckpointStore`]
- [`RecoveryCorruptionPolicy`]
- [`RecoveryVenuePolicy`]
- [`RecoveryPlan`]
- [`RecoveredOmsState`]
- [`RecoveryResult`]
- [`RecoveryReadinessConfig`]
- [`RecoveryReadinessBlocker`]
- [`RecoveryReadinessDecision`]
- [`evaluate_recovery_readiness`]
- [`recover_oms_state_from_records`]
- [`recover_oms_state_from_segmented_wal`]
- [`recover_latest_checkpoint_from_segmented_wal`]

Engine and simulation:

- [`ExecutionEngine`]
- [`ExecutionMetrics`]
- [`ExecutionRunbookSnapshot`]
- [`ExecutionAuditBundleManifest`]
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
- [`ReconciliationIssueKind`]
- [`ReconciliationDetail`]
- [`VenueReconciliationReport`]
- [`ReconciliationPolicyAction`]
- [`ReconciliationPolicy`]
- [`ReconciliationPolicyItem`]
- [`ReconciliationPolicyDecision`]
- [`reconcile_open_orders`]
- [`reconcile_open_orders_detailed`]
- [`evaluate_reconciliation_policy`]
- [`AllocationMethod`]
- [`AllocationLeg`]
- [`AllocationGroup`]
- [`AllocationFill`]
- [`AllocationReport`]
- [`AllocationError`]
- [`AllocationReconciliationIssue`]
- [`AllocationReconciliationDetail`]
- [`AllocationReconciliationReport`]
- [`allocate_block_fill`]
- [`reconcile_allocations`]
- [`DisconnectPolicy`]
- [`RouteSafetyPolicy`]
- [`SafetyCondition`]
- [`SafetyPolicyAction`]
- [`SafetyContext`]
- [`SafetyPolicy`]
- [`SafetyPolicyDecisionItem`]
- [`SafetyPolicyDecision`]
- [`evaluate_safety_policy`]
- [`AdvancedRiskLimits`]
- [`AdvancedRiskGate`]
- [`Position`]
- [`PositionKey`]
- [`PositionLedger`]
- [`VenueOrderCapabilities`]
- [`NormalizedOrderType`]
- [`normalize_order_type`]
- [`ExecutionTelemetry`]
- [`TimestampSource`]
- [`TimestampPointKind`]
- [`ExecutionTimestampSources`]
- [`ExecutionTimestampTrace`]
- [`ExecutionLatencyAttribution`]
- [`TimestampDisciplineConfig`]
- [`TimestampDisciplineIssueKind`]
- [`TimestampDisciplineIssue`]
- [`TimestampDisciplineReport`]
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

[`ExecutionEngine::runbook_snapshot`] returns a read-only
[`ExecutionRunbookSnapshot`] for operator dashboards and incident checklists. It
summarizes adapter connectivity, route enablement, route-level kill switches,
open versus terminal local orders, core execution counters, whether every
new-order route is blocked, and whether the engine deserves operator attention.
The snapshot is factual only: hosts still own permissions, escalation, and
operator command execution.

[`ExecutionEngine::audit_bundle_manifest`] returns an
[`ExecutionAuditBundleManifest`] for incident export workflows. It combines the
runbook snapshot with journal command/event counts and execution metrics, so a
host can package WAL segments, checkpoints, configs, reconciliation reports, and
adapter health consistently without adding export work to the order path. Use
[`ExecutionEngine::audit_bundle_manifest_at`] in deterministic replay tests when
the bundle timestamp must be fixed.

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
helper surface. [`WalExecutionJournal`] is an additive binary WAL-backed
implementation that uses `of_execution_core` WAL frames, sequence numbers,
checksums, and configurable sync policy.

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

### Binary WAL journal

[`WalExecutionJournal`] records the same [`JournalRecord`] model as
[`FileExecutionJournal`], but avoids text formatting on the journal path. It
owns WAL sequence assignment, validates existing bytes before accepting new
records, and replays into the existing journal output type.

```rust
use of_execution::{
    ExecutionJournal, JournalCommandKind, WalExecutionJournal, WalJournalConfig,
};
use of_execution_core::{ClientOrderId, WalSequence, WalSyncPolicy};

let path = std::env::temp_dir().join("orders.ofwal");
let mut journal = WalExecutionJournal::open(
    WalJournalConfig::new(&path).with_sync_policy(WalSyncPolicy::EveryNRecords(32)),
)?;

journal.record_command(
    JournalCommandKind::Submit,
    ClientOrderId::new("C1")?,
    1_000,
)?;
journal.sync()?;

let report = journal.integrity_report()?;
assert!(report.valid);

let metrics = journal.metrics();
assert_eq!(metrics.records_written, 1);

let mut replayed = Vec::new();
let replay = journal.replay_from(WalSequence(1), &mut replayed)?;
assert_eq!(replay.records, replayed.len());
# let _ = std::fs::remove_file(path);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Supported sync policies come from `of_execution_core::WalSyncPolicy`:

- `Never`
- `EveryRecord`
- `EveryNRecords(n)`
- `EveryDurationNs(ns)`
- `Manual`
- `OnRiskBoundary`

[`WalJournalMetrics`] records successful WAL frames/bytes, write latency, sync
latency, sync failures, write failures, segment rotations, and manifest writes.
The snapshot is copyable and allocation-free so hosts can export it out of band
to Prometheus, statsd, or tracing systems without formatting on the order path.

### Segmented WAL journal

[`SegmentedWalExecutionJournal`] uses the same binary WAL frames and
[`ExecutionJournal`] model as [`WalExecutionJournal`], but stores records in an
ordered segment directory:

```text
execution-wal/
  manifest
  wal-000000000001.ofwal
  wal-000000000002.ofwal
```

The journal rotates before appending a normal command/event record when either
`WalSegmentConfig::max_segment_bytes()` or
`WalSegmentConfig::max_segment_records()` would be exceeded. Rotation appends a
`SegmentSeal` WAL frame to the old segment, opens the next segment file, and
updates the manifest. Seal frames consume WAL sequence numbers so checksum and
sequence validation remain continuous across files, but replay skips them and
returns only [`JournalRecord`] values.

```rust
use of_execution::{
    ExecutionJournal, JournalCommandKind, SegmentedWalExecutionJournal,
    WalSegmentConfig,
};
use of_execution_core::{ClientOrderId, WalSequence, WalSyncPolicy};

let root = std::env::temp_dir().join(format!("execution-wal-{}", std::process::id()));
let _ = std::fs::remove_dir_all(&root);
let mut journal = SegmentedWalExecutionJournal::open(
    WalSegmentConfig::new(&root)
        .with_sync_policy(WalSyncPolicy::EveryNRecords(64))
        .with_max_segment_records(1_000_000)
        .with_max_segment_bytes(64 * 1024 * 1024),
)?;

journal.record_command(
    JournalCommandKind::Submit,
    ClientOrderId::new("C1")?,
    1_000,
)?;
journal.sync()?;

let manifest = journal.manifest();
assert!(manifest.active_segment().is_some());
assert_eq!(journal.metrics().records_written, 1);

let mut replayed = Vec::new();
let replay = journal.replay_from(WalSequence(1), &mut replayed)?;
assert_eq!(replay.records, replayed.len());
# let _ = std::fs::remove_dir_all(root);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Recovery does not trust the manifest as source of truth. Opening the journal
scans `wal-*.ofwal` files in segment-id order, validates checksums and
sequences across segment boundaries, reconstructs the manifest, and fails
closed on corrupt or non-contiguous data. The manifest is an operator inventory
and discovery aid.

Use `SegmentedWalExecutionJournal::inspect_root(root)` when an operator or
binding needs a read-only integrity report without creating a new active
segment. The report returns segment count, valid record/byte counts, optional
sequence range, checksum failures, sequence failures, and a `valid` flag. It is
intended for recovery drills, restart checks, and archival validation outside
the order-submission path.

Checkpoint marker records remain a separate additive feature so the
compatibility boundary stays narrow.

### Checkpoint store

[`FileExecutionCheckpointStore`] stores versioned OMS checkpoints outside the
hot order path. Checkpoints are written to a temporary file, flushed, optionally
synced, and atomically renamed into place. Each checkpoint carries a checksum
over its payload, and corrupt checkpoints are rejected on load.

The first checkpoint format captures:

- last fully applied WAL sequence;
- route configuration hash selected by the host;
- open order states;
- position snapshots;
- kill-switch state.

```rust
use of_execution::{
    CheckpointConfig, ExecutionCheckpoint, ExecutionCheckpointStore,
    FileExecutionCheckpointStore,
};
use of_execution_core::WalSequence;

let root = std::env::temp_dir().join(format!("orderflow-checkpoints-{}", std::process::id()));
let _ = std::fs::remove_dir_all(&root);
let mut store = FileExecutionCheckpointStore::open(
    CheckpointConfig::new(&root).with_sync_on_save(false),
)?;

let checkpoint = ExecutionCheckpoint::new(1, WalSequence(42), 1_000)
    .with_route_config_hash(7);

let manifest = store.save_checkpoint(&checkpoint)?;
assert_eq!(manifest.last_applied_sequence, WalSequence(42));

let latest = store.load_latest()?.expect("checkpoint");
assert_eq!(latest.checkpoint_id, 1);
assert!(store.validate_checkpoint(&latest)?);
# let _ = std::fs::remove_dir_all(root);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`CheckpointPolicy` is metadata for hosts and future checkpoint schedulers. This
store does not start background writers or block the OMS worker automatically;
callers decide when to construct and save snapshots.

Use `FileExecutionCheckpointStore::inspect_root(root)` for read-only startup and
operator diagnostics. The report counts discovered, valid, and invalid
checkpoint files, totals checkpoint bytes, and identifies the latest valid
checkpoint id, covered WAL sequence, and creation timestamp. It does not create
the store directory, save checkpoints, prune files, or mutate the checkpoint
root.

### Recovery plan

[`RecoveryPlan`] describes a deterministic replay before recovery starts:

- first WAL sequence to replay;
- optional latest sequence expected by the caller;
- fail-closed corruption policy;
- venue reconciliation policy;
- whether strategy submissions remain disabled after recovery.

[`recover_latest_checkpoint_from_segmented_wal`] loads the newest valid
checkpoint from an [`ExecutionCheckpointStore`], builds a plan from
`last_applied_sequence + 1`, replays the segmented WAL tail, and returns a
[`RecoveryResult`] with [`RecoveredOmsState`].

```rust
use of_execution::{
    recover_latest_checkpoint_from_segmented_wal, CheckpointConfig,
    FileExecutionCheckpointStore, SegmentedWalExecutionJournal, WalSegmentConfig,
};
use of_execution_core::WalSyncPolicy;

let checkpoints = std::env::temp_dir().join(format!(
    "orderflow-checkpoints-recovery-{}",
    std::process::id()
));
let wal = std::env::temp_dir().join(format!("execution-wal-recovery-{}", std::process::id()));
let _ = std::fs::remove_dir_all(&checkpoints);
let _ = std::fs::remove_dir_all(&wal);

let store = FileExecutionCheckpointStore::open(
    CheckpointConfig::new(&checkpoints).with_sync_on_save(false),
)?;
let journal = SegmentedWalExecutionJournal::open(
    WalSegmentConfig::new(&wal).with_sync_policy(WalSyncPolicy::Never),
)?;

let result = recover_latest_checkpoint_from_segmented_wal(&store, &journal)?;
assert!(result.venue_reconciliation_required);
# let _ = std::fs::remove_dir_all(checkpoints);
# let _ = std::fs::remove_dir_all(wal);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Recovery intentionally fails closed when the WAL tail contains an execution
event for an order that was not present in the selected checkpoint. The current
command WAL payload records command kind, id, and timestamp, not the full
`OrderRequest`, so the recovery layer refuses to invent side, strategy, price,
or quantity. Production hosts should checkpoint frequently, require venue
reconciliation, and add full command-payload journaling before relying on
checkpoint-only recovery for long uncheckpointed windows.

### Recovery readiness gate

[`evaluate_recovery_readiness`] combines the independent restart evidence into
one typed, fail-closed resume decision:

- segmented WAL integrity;
- checkpoint-store integrity;
- latest recovered WAL sequence;
- whether recovery still disables submissions;
- required venue reconciliation;
- reconciliation policy actions such as venue cancels, local restates, and
  operator approval.

The gate does not perform I/O, replay WAL bytes, mutate checkpoints, call
venues, or enable trading. It is a deterministic policy evaluator over reports
the host already produced during startup. That keeps the low-latency restart
path explicit and lets production hosts replace the policy without replacing
the WAL or checkpoint implementations.

```rust
use of_execution::{
    evaluate_reconciliation_policy, evaluate_recovery_readiness,
    recover_oms_state_from_records, CheckpointStoreIntegrityReport,
    ReconciliationPolicy, RecoveryPlan, RecoveryReadinessConfig,
    VenueReconciliationReport,
    RecoveryVenuePolicy, WalSegmentIntegrityReport,
};
use of_execution_core::WalSequence;

let recovery = recover_oms_state_from_records(
    RecoveryPlan::new(WalSequence(1))
        .with_venue_policy(RecoveryVenuePolicy::HostControlled)
        .with_submissions_disabled(false),
    None,
    &[],
)?;
let mut wal = WalSegmentIntegrityReport::default();
wal.valid = true;
wal.last_sequence = Some(WalSequence(10));
let mut checkpoints = CheckpointStoreIntegrityReport::default();
checkpoints.valid = true;
checkpoints.latest_checkpoint_id = Some(3);
checkpoints.latest_last_applied_sequence = Some(WalSequence(10));
let reconciliation_report = VenueReconciliationReport::default();
let reconciliation =
    evaluate_reconciliation_policy(&reconciliation_report, ReconciliationPolicy::default());

let decision = evaluate_recovery_readiness(
    &recovery,
    &wal,
    &checkpoints,
    Some(&reconciliation),
    RecoveryReadinessConfig::strict(),
);
assert!(decision.is_ready());
# Ok::<(), Box<dyn std::error::Error>>(())
```

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

Use [`reconcile_open_orders_detailed`] when recovery or live monitoring needs a
more precise discrepancy type:

- [`ReconciliationIssueKind::Matched`]
- [`ReconciliationIssueKind::VenueOnly`]
- [`ReconciliationIssueKind::LocalOnly`]
- [`ReconciliationIssueKind::QuantityMismatch`]
- [`ReconciliationIssueKind::StatusMismatch`]
- [`ReconciliationIssueKind::PriceMismatch`]
- [`ReconciliationIssueKind::Unknown`]

[`ReconciliationPolicy`] maps those issues to host actions such as fail closed,
accept venue truth, cancel venue-only orders, restate venue-only orders, or
require operator approval. [`evaluate_reconciliation_policy`] returns a
[`ReconciliationPolicyDecision`] that keeps submissions disabled until required
host action completes.

[`ExecutionEngine::reconcile_open_orders_with`] and
[`ExecutionEngine::evaluate_reconciliation`] apply the same logic to the
engine's current local non-terminal orders and a venue open-order snapshot.

### Independent drop copy

[`DropCopyAdapter`] is deliberately separate from [`ExecutionAdapter`]. A
provider can therefore run order entry and independent execution evidence on
different transports, credentials, sessions, and recovery sequences without
changing existing execution adapter implementations.

```mermaid
flowchart LR
    Primary[Primary order-entry session] --> OMS[ExecutionEngine]
    Primary --> Venue[Venue matching engine]
    Venue --> PrimaryReports[Primary execution reports]
    PrimaryReports --> OMS
    Venue --> DropSession[Independent drop-copy session]
    DropSession --> Mapper[Provider mapping]
    Mapper --> Report[DropCopyReport]
    OMS --> Local[Local OrderState snapshot]
    Report --> Reconciler[DropCopyReconciler]
    Local --> Reconciler
    Reconciler --> Observation[DropCopyObservation]
    Reconciler --> Metrics[DropCopyMetricsSnapshot]
```

Provider adapters decode their wire message into an [`ExecutionEvent`] and
place it in [`DropCopyReport`]. The envelope adds the independent source id,
provider report id, source sequence, and local receive timestamp needed for
deduplication and audit. Report identity is scoped by source. When a provider
does not expose a report id, a nonzero source sequence is the fallback key;
when neither exists, [`DropCopyIssueFlags::MISSING_DUPLICATE_KEY`] makes the
loss of protection explicit.

[`DropCopyReconciler`] correlates venue order id first, then current, previous,
or original client order id. It compares account, route, symbol, status,
cumulative quantity, leaves quantity, average price, and trade quantity
invariants. It reports differences as a compact [`DropCopyIssueFlags`] bitset
without mutating the engine. Venue-only evidence remains visible instead of
being silently inserted into local state.

```rust
use of_execution::{
    DropCopyLateReportPolicy, DropCopyReconciler, DropCopyReport,
};
use of_execution_core::OrderState;

fn inspect_drop_copy(report: &DropCopyReport, local_orders: &[OrderState]) {
    let mut reconciler = DropCopyReconciler::new(
        16_384, // retained report identities
        8_192,  // local/progress order identities
        DropCopyLateReportPolicy::AuditOnly,
    );
    reconciler.replace_local_orders(local_orders);

    let observation = reconciler.observe(report);
    if observation.reconciliation_eligible() && observation.has_state_mismatch() {
        // Keep submissions fail-closed and send the issue bitset to the
        // operator/reconciliation control plane.
    }
    let metrics = reconciler.metrics();
    assert_eq!(metrics.reports_received, 1);
}
```

Late handling is explicit. A lower exchange timestamp or cumulative fill than
previous independent evidence is classified as late. `AcceptAndFlag` keeps it
eligible, `AuditOnly` retains evidence without treating it as current, and
`Reject` excludes it from reconciliation. Duplicate reports are always marked
`Duplicate` and are not reconciled twice.

Capacity is part of the operational contract:

- [`DropCopyReportBuffer`] and [`InMemoryDropCopyAdapter`] never grow beyond
  their configured queue bounds;
- duplicate retention uses a fixed-capacity FIFO window;
- progress tracking does not grow on the report path after its configured
  capacity is reached;
- [`DropCopyIssueFlags::TRACKING_CAPACITY_EXHAUSTED`] and
  [`DropCopyMetricsSnapshot::tracking_capacity_exhaustions`] expose undersized
  deployments;
- caller-supplied timestamps keep clock reads outside the reconciliation path;
- JSON, logging, metric formatting, alerting, and policy actions stay in the
  host control plane.

Refreshing local orders is a control-plane operation and can allocate if the
new snapshot exceeds the initial capacity. Size the reconciler from expected
peak open-order cardinality, and alert on duplicate-window turnover,
`venue_only_reports`, `mismatched_reports`, `late_reports`, source state, and
drop-copy lag.

### Scoped kill switches

The existing `RiskLimits::kill_switch` route flag remains valid. The additive
[`KillSwitchRegistry`] handles production workflows that need several active,
independently auditable scopes at once:

- global;
- venue;
- route;
- account;
- strategy;
- symbol;
- order type; and
- adapter/session.

```mermaid
stateDiagram-v2
    [*] --> Uncertain
    Uncertain --> Confirmed: restore and confirm state
    Confirmed --> Active: activate switch
    Active --> Cancelling: CancelAll / CancelScope / HardStopAdapter
    Active --> Cleared: RejectNew / ReduceOnly / PauseStrategy clear
    Cancelling --> Cleared: all cancels succeeded
    Cancelling --> ForcedClear: explicit operator override
    Active --> Uncertain: recovery evidence lost
    Cancelling --> Uncertain: recovery evidence lost
    Cleared --> Confirmed
    ForcedClear --> Confirmed
```

[`KillSwitchRegistry::new`] starts in
[`KillSwitchStateCertainty::Uncertain`], and all new orders fail closed until
the host restores durable state and calls `confirm_state`. Use
[`KillSwitchRegistry::confirmed_empty`] only when the host has authoritative
evidence that a new session has no active switches. Mark state uncertain again
whenever WAL, checkpoint, or operator-control recovery is incomplete.

Evaluate a request before normal route risk and before provider I/O:

```rust
use of_execution::{KillSwitchRegistry, KillSwitchSessionId};
use of_execution_core::OrderRequest;

fn can_submit(
    registry: &KillSwitchRegistry,
    request: &OrderRequest,
    signed_position: i64,
) -> bool {
    let session = KillSwitchSessionId::new("fix-order-entry-a").unwrap();
    registry
        .evaluate_request(request, signed_position, session)
        .allow_new_order
}
```

Modes have distinct operational meaning:

- `RejectNew` blocks matching submissions but keeps cancels available;
- `CancelAll` blocks every submission and selects every supplied open order,
  regardless of the scope value;
- `CancelScope` blocks and selects only matching orders;
- `ReduceOnly` permits an order only when side and quantity strictly reduce the
  supplied signed position without crossing through zero;
- `PauseStrategy` blocks matching strategy commands while preserving cancels;
- `HardStopAdapter` selects matching open orders, blocks submissions, marks
  cancel flow unavailable after shutdown, and tells the host to stop the
  adapter/session.

Activation accepts current open-order contexts and writes cancellation targets
to a caller-owned [`KillSwitchAffectedOrderBuffer`]. The returned
[`KillSwitchEvent`] contains the full scope, mode, actor/system source, reason,
timestamp, WAL sequence, affected/captured counts, truncation status, cancel
counts, aggregate outcome, state certainty, and forced-clear status. Journal
the event and each later cancel-progress/clear event before exposing them to
operator tooling.

Cancellation targets and results are bounded and preallocated. A result is
accepted once, only for an order captured at activation. If target output or
internal result capacity is too small, the activation event remains visibly
truncated and cannot reach `AllSucceeded`; clearing then requires a deliberate
forced override. This prevents a partial cancellation list from being mistaken
for a clean kill.

The registry does not call adapters, write a WAL, authenticate operators, or
invent timestamps. Those actions belong to the host because venue APIs,
permissions, durability policies, and adapter shutdown mechanics differ. The
typed outputs make those actions explicit while the matching and decision path
stays allocation-free after construction.

Operational references:

- [CME Globex Kill Switch](https://www.cmegroup.com/tools-information/webhelp/globex-credit-controls/Content/Kill-Switch.html)
- [CFTC automated-trading risk-control discussion](https://www.cftc.gov/LawRegulation/FederalRegister/finalrules/2013-22185.html)

### Production risk engine

[`ProductionRiskEngine`] is an additive, scoped pre-trade policy engine for
deployments that need controls beyond [`RiskLimits`] and
[`AdvancedRiskLimits`]. Existing engines, risk traits, request layouts, and
bindings are unchanged. A host opts in by evaluating a canonical
[`ProductionRiskCommand`] before submitting to [`ExecutionEngine`].

Policies compose across global, account, strategy, route, symbol, venue, and
host-defined instrument-group scopes. They evaluate in stable `(priority,
policy_id)` order. Every matching policy is checked so its independent rate
window advances, while the first ordered rejection remains the primary
explanation. An empty policy set or a command matching no policy fails closed.

```mermaid
flowchart LR
  Command[Submit / amend / cancel] --> Context[Authoritative host context]
  Context --> Match[Match ordered scopes]
  Match --> Rates[Update bounded rate windows]
  Rates --> Controls[Operational, PnL, size,<br/>exposure, price, session checks]
  Controls --> Decision[Explainable decision]
  Decision --> Journal{Decision retained?}
  Journal -- No --> Reject[Fail closed]
  Journal -- Yes, allow --> OMS[ExecutionEngine]
  Journal -- Yes, reject --> Audit[Operator / audit path]
```

The limit model covers:

- order quantity and normalized notional;
- projected position, gross exposure, net exposure, and open-order count;
- independent trailing one-second submit/amend and cancel rates;
- reference-price collars and typical-quantity fat-finger limits;
- duplicate client ids, restricted scopes, and host self-trade checks;
- UTC day, overnight, and full-day trading windows;
- strict reduce-only behavior that cannot cross through flat;
- loss and peak-to-current daily drawdown limits; and
- fail-closed market-data, adapter, persistence, and risk-state health gates.

The host supplies [`ProductionRiskContext`] from its authoritative position,
PnL, market-data, duplicate-id, self-trade, and health stores. The default
context reports unavailable risk state. [`ProductionRiskLimits::conservative`]
enables operational blocks; [`ProductionRiskLimits::permissive`] is an
explicit simulation/test profile.

```rust
use of_execution::{
    InMemoryProductionRiskJournal, ProductionRiskCommand,
    ProductionRiskContext, ProductionRiskEngine, ProductionRiskLimits,
    ProductionRiskPolicy, ProductionRiskScope,
};
use of_execution_core::{
    AccountId, ClientOrderId, ExecutionSymbol, OrderPrice, OrderQty,
    OrderRequest, OrderSide, OrderType, RouteId, StrategyId, TimeInForce,
};

let mut limits = ProductionRiskLimits::conservative();
limits.max_order_qty = 50;
limits.max_order_notional = 50_000_000;
limits.max_position_abs = 250;
limits.max_order_rate_per_sec = 1_000;
limits.price_collar_ticks = 20;

let mut risk = ProductionRiskEngine::with_capacity(16);
risk.add_policy(ProductionRiskPolicy::from_id(
    "global-default",
    ProductionRiskScope::Global,
    100,
    limits,
)?)?;

let request = OrderRequest {
    client_order_id: ClientOrderId::new("strategy-a-42")?,
    account_id: AccountId::new("account-a")?,
    route_id: RouteId::new("fix-a")?,
    strategy_id: StrategyId::new("strategy-a")?,
    symbol: ExecutionSymbol::new("XCME", "ESM6")?,
    side: OrderSide::Buy,
    order_type: OrderType::Limit,
    time_in_force: TimeInForce::Day,
    quantity: OrderQty(2),
    limit_price: OrderPrice(5_000),
    stop_price: OrderPrice(0),
    ts_exchange_ns: 0,
    ts_recv_ns: 1_000_000_000,
};
let group = of_execution::RiskInstrumentGroupId::new("equity-index")?;
let command = ProductionRiskCommand::submit(&request, group);
let mut context = ProductionRiskContext::available();
context.reference_price = OrderPrice(4_995);
let mut journal = InMemoryProductionRiskJournal::with_capacity(1_024);
let decision = risk.evaluate_and_record(command, context, &mut journal);
if decision.allowed {
    // Submit `request` through the existing ExecutionEngine.
} else {
    // Persist and expose decision.reason, policy_id, observed, and limit.
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

Construction reserves policy and rate-window storage. Evaluation uses
fixed-size identifiers, mutates only preallocated rate queues, performs no I/O,
and does not read clocks. The caller supplies timestamps and owns
synchronization; one worker should own one mutable risk engine.
[`ProductionRiskEngine::evaluate_and_record`] records before routing, and
journal failure converts even an allowed result to
[`ProductionRiskReason::DecisionJournalUnavailable`]. Replace the bounded
in-memory journal with a durable implementation in production.

Policy installation rejects empty policy ids, negative signed limits, duplicate
ids, exhausted policy capacity, and rate limits above the documented maximum.
For amend evaluation, current gross and net context must include the order being
replaced; the engine subtracts that existing exposure before adding the
replacement.

Cancels intentionally bypass exposure, market-data, and PnL blocks so
operators can reduce risk during degradation, while their independent
cancel-rate and timestamp controls remain active. Amend context includes the
replaced quantity and price so projected position and exposure are computed
from the delta.

Regulatory design references include the
[SEC Market Access Rule](https://www.sec.gov/rules-regulations/2010/11/risk-management-controls-brokers-dealers-market-access)
and [FINRA market-access guidance](https://www.finra.org/rules-guidance/guidance/reports/2021-finras-examination-and-risk-monitoring-program/market-access-rule).

### Multi-account allocation

[`AllocationGroup`] defines optional post-trade allocation across account,
route, and strategy legs. [`allocate_block_fill`] supports deterministic
average-price block allocation with:

- [`AllocationMethod::Proportional`] using largest-remainder balancing, and
- [`AllocationMethod::Priority`] using priority and target quantities.

[`AllocationReport`] records the balanced allocated quantity, average price, and
per-leg [`AllocationFill`] values. [`reconcile_allocations`] compares expected
and actual fills with allocation-native mismatch classes such as missing actual
fills, unexpected actual fills, quantity mismatch, and price mismatch. These
helpers do not submit child orders or mutate OMS state; hosts decide when to
emit allocations to middle-office, FIX allocation, file export, or accounting
systems.

### Safety policies

[`DisconnectPolicy`] describes route behavior during disconnects:

- `Hold`
- `RejectNew`
- `CancelOpenOrders`
- `Freeze`

[`RouteSafetyPolicy`] combines disconnect policy, kill switch state, and whether
cancels are allowed while killed.

[`SafetyPolicy`] provides a configurable fail-open/fail-closed matrix across
production conditions such as stale market data, degraded persistence, degraded
OMS WAL, checkpoint failure, adapter disconnect, drop-copy disconnect,
reconciliation mismatch, unavailable risk, position ledger mismatch, and route
health degradation.

`SafetyPolicy::fail_closed()` is the default. It rejects new submissions for
active conditions while preserving cancel flow. `SafetyPolicy::fail_open_degraded()`
must be selected explicitly and records every active allowance in
[`SafetyPolicyDecision::fail_open_count`]. [`evaluate_safety_policy`] returns
whether submissions and cancels remain enabled, whether operator approval is
required, and whether the decision is degraded.

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

### Clock and timestamp discipline

[`ExecutionTimestampTrace`] is an optional, host-owned timestamp envelope for
production execution workflows. It tracks:

- strategy decision time,
- OMS receive time,
- WAL append time,
- adapter send time,
- exchange/venue time,
- OMS report receive time,
- drop-copy receive time, and
- checkpoint time.

[`TimestampSource`] and [`ExecutionTimestampSources`] record where those values
came from, such as host software clocks, monotonic clocks, hardware clocks,
venues, journals, drop copy, or checkpoint stores. The trace can produce
[`ExecutionLatencyAttribution`] without forcing the engine to read clocks on
every command.

[`TimestampDisciplineConfig`] and [`TimestampDisciplineReport`] validate known
internal timestamps for monotonic order and compare venue timestamps with OMS
report receive time for clock-skew checks. The helpers are additive and do not
change existing `repr(C)` request or event layouts.

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
