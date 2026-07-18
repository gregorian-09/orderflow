# Release 0.4.0

Date: 2026-06-04

Orderflow `0.4.0` is a non-breaking analytics-to-execution expansion release.
It keeps existing market-data analytics APIs stable while adding a separate
execution and OMS foundation for developer-built order-management workflows.

## Version Decision

This release uses a split version model:

| Package family | Version |
| --- | ---: |
| Existing Rust crates: `of_core`, `of_adapters`, `of_signals`, `of_persist`, `of_runtime`, `of_ffi_c` | `0.4.0` |
| Python binding: `orderflow-gregorian09` | `0.4.0` |
| Java binding: `orderflow-java-binding` | `0.4.0` |
| New Rust advanced analytics/execution/FIX/algo crates: `of_analytics`, `of_execution_core`, `of_fix`, `of_execution`, `of_execution_adapters`, `of_execution_algos` | `0.1.0` |

The split is intentional. The analytics/runtime/binding stack is the existing
public package family. The advanced analytics/execution/FIX crates are new
public Rust surfaces and should start at `0.1.0` so their traits, codec APIs,
and adapter contracts can mature honestly.

Rust publish order for the new crate family:

1. `of_analytics`
2. `of_execution_core`
3. `of_fix`
4. `of_execution`
5. `of_execution_adapters`
6. `of_execution_algos`

`of_execution_adapters`' `fix` feature depends on `of_fix`, so local
`cargo package -p of_execution_adapters` verification can only resolve the
registry dependency after `of_fix 0.1.0` is published.

## What Is New

### 1. Advanced analytics crate split

`of_analytics 0.1.0` adds a dependency-light advanced analytics home:

- market-quality/TCA primitives for quoted spread, effective spread, realized
  spread, price improvement, quote freshness, and side-aware slippage
- liquidity/depth primitives for top-of-book depth, multi-level depth,
  proportional imbalance, depth slope, and sweepability
- market-impact primitives for Kyle-style lambda and Amihud-style illiquidity
- VPIN-style fixed-bucket toxicity primitives
- fixed-window volatility/noise primitives and threshold-based regime
  classification
- feed-quality primitives for sequence gaps, out-of-order events, duplicates,
  stale events, locked/crossed books, timestamp skew, resets, and health
  scoring
- feature-vector primitives for stable feature ids, ordered schemas, schema
  hashes, missing-value policy, quality labels, reusable fixed-capacity
  writers, and extractor contracts
- resiliency primitives for threshold-based spread/depth shock detection,
  recovery timing, and liquidity resiliency scoring
- queue/fill primitives for passive queue position estimates, fill
  probability, expected time-to-fill, amend queue loss, and maker/taker scoring
- pattern-risk primitives for spoofing/layering, quote-stuffing, stop-run,
  absorption, and momentum-ignition risk indicators
- feature profiles for future impact, toxicity, volatility, regime,
  data-quality, feature-vector, resiliency, queue-fill, patterns, derivatives,
  institutional, and ML feature modules
- borrowed `of_core::BookLevel` analysis paths that avoid copying book
  snapshots in hot analytics loops

Existing `of_core`, runtime, C ABI, Python, and Java analytics APIs remain
valid. The split is additive and provides a clean home for heavier models
without forcing all users to compile them.

### 2. Execution core

`of_execution_core 0.1.0` adds:

- fixed-size ASCII identifiers for client, venue, execution, account, route,
  strategy, venue, instrument, and bounded text fields
- typed `OrderRequest`, `CancelRequest`, and `AmendRequest`
- integer-normalized quantity and price wrappers
- canonical side, order type, time-in-force, execution type, and order status
- strict order-state machine with validated transitions
- basic route-scoped risk limits and structured risk rejection reasons

### 3. Execution engine and OMS helpers

`of_execution 0.1.0` adds:

- `ExecutionAdapter` provider-neutral adapter trait
- `ExecutionEngine` synchronous deterministic owner
- `ConcurrentExecutionEngine` bounded worker for many producers and one native
  order-state owner
- route/account/symbol configuration and open-order risk accounting
- bounded `ExecutionEventBuffer`
- deterministic simulated execution adapter
- in-memory and file-backed execution journal helpers
- recovery and open-order reconciliation primitives
- lifecycle, fanout, command correlation, throttling, sharding, telemetry,
  position ledger, safety policy, and replay helpers

### 4. FIX codec foundation

`of_fix 0.1.0` adds a reusable low-allocation FIX tag-value codec foundation:

- borrowed `FixFieldView` and `FixMessageView` parsing from raw bytes
- caller-provided parse scratch buffers
- strict `BodyLength(9)` and `CheckSum(10)` validation
- typed `FixVersion` and common `FixMsgType` helpers
- static `FixDictionary`/`FixMessageRule` profile validation for required and
  disallowed tags
- borrowed Reject `<3>` and BusinessMessageReject `<j>` diagnostic parsers
- reusable `FixDecoder` and `FixEncoder` facades
- `FixSessionState`, `FixSequenceTracker`, `FixSequenceAction`, and
  `FixResendRange` primitives for deterministic session sequence handling
- `FixSessionId` and `FixSequenceSnapshot` primitives for storage-neutral
  session sequence persistence
- bounded `FixResendStore` primitives for in-memory outbound frame retention,
  replay/gap-fill planning, and retention/drop/eviction metrics
- bounded `FixTranscriptCapture` primitives for certification/audit transcript
  evidence with optional raw retention, metadata-only capture, and rolling hash
- `encode_poss_dup_replay` for possible-duplicate replay encoding with
  current `SendingTime(52)`, preserved `OrigSendingTime(122)`, and recomputed
  body length/checksum
- typed session/admin builders for Logon, Heartbeat, TestRequest,
  ResendRequest, SequenceReset gap fill, and Logout
- typed order-entry builders for NewOrderSingle, OrderCancelRequest,
  OrderCancelReplaceRequest, OrderStatusRequest, OrderMassCancelRequest, and
  OrderMassStatusRequest
- optional `Account(1)` support on NewOrderSingle, OrderCancelRequest, and
  OrderCancelReplaceRequest builders
- optional `StopPx(99)` support on NewOrderSingle and
  OrderCancelReplaceRequest builders
- common FIX tag constants and extraction helpers
- caller-owned encoding buffers with computed body length and checksum
- diagnostic rendering with `|` separators outside hot paths

This is not a full FIX session engine. Transport, logon/logout, resend, sequence
message replay, gap-fill generation, persistence, venue certification, and
counterparty-specific business rules remain separate future layers built on top
of the codec.

### 5. Execution adapter scaffolding

`of_execution_adapters 0.1.0` adds a feature-gated FIX scaffold:

- FIX-style session config
- normalized execution-report struct
- validated `of_fix::FixMessageView` execution-report parser bridge
- validated `of_fix::FixMessageView` order-cancel-reject parser bridge
- canonical OMS request to FIX NewOrderSingle, OrderCancelRequest, and
  OrderCancelReplaceRequest encoding bridge with explicit decimal scales,
  caller-provided FIX timestamps, and `Account(1)` propagation
- stop and stop-limit new-order encoding through the FIX request bridge
- explicit stop and stop-limit amend encoding through
  `encode_stop_amend_request`

### 6. Execution algorithm foundation

`of_execution_algos 0.1.0` adds an optional parent/child execution-algorithm
foundation:

- fixed-size parent, child, intent, and algo-instance identifiers
- parent lifecycle status and child lifecycle status vocabularies
- `ParentOrder` metadata that mirrors OMS order ticket fields without changing
  existing `of_execution` APIs
- `ChildOrderPlan` values that convert algorithm decisions into canonical OMS
  `OrderRequest` submissions
- `AlgoProgress` folding from canonical `ExecutionEvent` reports
- fixed-capacity `AlgoDecision` buffers for allocation-aware live decision paths
- deterministic `TwapSlicePlanner` with explicit time window, clip bounds,
  caller-supplied ids, and caller-supplied timestamps
- deterministic TWAP replay primitives with explicit timer/execution/status
  inputs, caller-owned step output, generated replay ids, and summary hashes
  for regression checks
- deterministic `PovSlicePlanner` for volume-responsive participation planning
  from observed market volume, target/max participation bps, parent caps, and
  explicit clip limits
- deterministic `VwapVolumeCurve` and `VwapSlicePlanner` for historical
  volume-curve execution using borrowed cumulative weights and O(1) bucket
  lookup on the planning path
- deterministic `IcebergSlicePlanner` for synthetic displayed-quantity
  replenishment based on remaining parent leaves and open displayed quantity
- deterministic `ImplementationShortfallPlanner` with explicit market context,
  urgency weights, adverse-move detection from arrival price, and auditable
  target-release estimates
- deterministic `PassiveQueuePlanner` with host-owned best bid/ask, queue
  depth, expected contra volume, adverse-selection estimates, passive
  improvement, and optional late crossing
- deterministic `SorPlanner` with route status, order-type capability, price,
  available quantity, fees/rebates, latency, reject rate, fill probability,
  toxicity, data-quality scoring, fixed-capacity allocations, and OMS-safe
  child plans
- deterministic `LiquiditySeekingPlanner` with SOR candidate reuse,
  hidden-liquidity and price-improvement scoring, probe/take decisions,
  toxicity filtering, minimum quantity checks, and OMS-safe route-specific
  child plans
- deterministic `SweepPlanner` for aggressive liquidity taking over route
  candidates with side-aware price collars, minimum fill quantity suppression,
  route/level capacity, average planned price, and OMS-safe child plans
- deterministic `BasketPlanner` with leg roles, hedge-ratio metadata,
  synchronized per-leg release, fixed-capacity decisions, and explicit
  non-atomic multi-leg semantics
- deterministic `SpreadPlanner` with hedge-ratio sizing, executable
  spread-edge gating, synchronized buy/sell child plans, ratio-aware quantity
  clipping, and explicit legging-risk boundaries
- additive `AlgoRiskPolicy` controls with typed limits, host-owned risk
  context, fixed-capacity explainable violation reports, kill switch and
  operator pause outcomes, price collars, participation, notional, child
  quantity, open quantity, child-count, stale-data, route-degradation, and
  persistence-degradation checks
- additive `AlgoCheckpoint` and `AlgoRecoveryPlan` primitives with
  schema-versioned parent/progress snapshots, replay cursors, decision sequence
  restoration, pause/resume/complete/escalate recovery actions, and explicit
  separation from OMS WAL ownership
- deterministic `AlgoSimulator` child-order simulation with explicit fill
  model inputs, canonical `ExecutionEvent` output, fixed-capacity simulation
  reports, fill/reject/cancel/resting outcomes, simulated latency,
  deterministic venue/execution id generation, and direct progress folding
- allocation-free `AlgoMetricsAccumulator` and TCA snapshots with child
  submission counts, fill/reject/cancel counts, completion, average execution
  price, side-aware arrival/VWAP/TWAP slippage, first/last timestamps, and
  average event latency
- typed `AlgoKind`, `AlgoParentConfig`, and `AlgoConfig` configuration
  primitives that build existing parent-order, risk-policy, and recovery-policy
  values without free-form maps or a forced serialization dependency
- deterministic `MarketMakerPlanner` with fair-value quoting, inventory skew,
  volatility/adverse-selection spread widening, inventory-limit side
  suppression, and OMS-safe bid/ask child plans

The crate does not bypass OMS risk, journaling, adapter capability checks, kill
switches, or reconciliation. Hosts should submit child orders through
`of_execution` and feed resulting execution events back into algo progress.
- FIX exec type/status mapping
- canonical `ExecutionEvent` conversion
- fail-closed adapter shell

This is not a production FIX engine. It is a reusable mapping and adapter
authoring scaffold.

### 7. C, Python, and Java execution APIs

`of_ffi_c 0.4.0`, Python `0.4.0`, and Java `0.4.0` expose the execution layer
through additive handles/classes:

- C: `of_execution_engine_t`, `of_execution_engine_create_multi`,
  `of_execution_submit_order`, `of_execution_cancel_order`,
  `of_execution_amend_order`, `of_execution_concurrent_*`
- Python: `ExecutionEngine`, `ConcurrentExecutionEngine`, `OrderRequest`,
  `CancelRequest`, `AmendRequest`, `RiskLimits`, `RouteConfig`
- Java: `OrderflowExecutionEngine`, `ConcurrentOrderflowExecutionEngine`,
  `OrderRequest`, `CancelRequest`, `AmendRequest`, `RiskLimits`, `RouteConfig`

Existing analytics handles and classes remain separate and unchanged.

### 8. Documentation expansion

The documentation now teaches the full workflow:

- market-data ingest or replay
- analytics and signal snapshots
- data-quality gating
- route-scoped risk checks
- simulated execution
- concurrent execution worker usage
- order event handling
- journaling, recovery, reconciliation, and replay review
- provider adapter authoring boundaries
- low-latency design constraints

## Upgrade Notes

Required migration for existing analytics users:

- none expected

Recommended upgrade steps:

- update `of_core`, `of_adapters`, `of_signals`, `of_persist`, `of_runtime`,
  and `of_ffi_c` together to `0.4.0`
- update Python/Java packages and native `of_ffi_c` runtime/header together to
  `0.4.0`
- pin new execution crates to compatible `0.1.x` versions if building Rust
  execution providers
- keep market-data runtime and execution engine ownership explicit in your
  application architecture
- treat simulated execution as a deterministic development/test adapter, not as
  broker-certified live connectivity

## What Existing APIs Are Not Changed

The release does not intentionally remove or rename:

- existing market-data adapter trait methods
- runtime lifecycle, subscribe, poll, ingest, and snapshot methods
- existing C analytics/runtime symbols
- existing Python `Engine` analytics methods
- existing Java `OrderflowEngine` analytics methods
- existing persistence readback APIs
- existing signal trait and constructors

## Where To Read Next

- Root README: [`README.md`](../../README.md)
- Strategy design: [`docs/handbook/02-strategy-design.md`](../handbook/02-strategy-design.md)
- Strategy cookbook: [`docs/handbook/08-strategy-cookbook.md`](../handbook/08-strategy-cookbook.md)
- OMS architecture: [`docs/handbook/09-oms-architecture.md`](../handbook/09-oms-architecture.md)
- OMS cookbook: [`docs/handbook/10-oms-cookbook.md`](../handbook/10-oms-cookbook.md)
- Low-latency design: [`docs/handbook/11-low-latency-design.md`](../handbook/11-low-latency-design.md)
- Provider adapter authoring: [`docs/handbook/12-provider-adapter-authoring.md`](../handbook/12-provider-adapter-authoring.md)
- Changelog: [`CHANGELOG.md`](../../CHANGELOG.md)
