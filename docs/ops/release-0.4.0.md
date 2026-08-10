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
- execution-quality/TCA primitives for implementation shortfall, arrival and
  decision slippage, adverse selection, trade-through, and fill-quality score
- liquidity/depth primitives for top-of-book depth, multi-level depth,
  proportional imbalance, order-flow imbalance, depth slope, depth convexity,
  book pressure, replenishment/depletion rates, drought detection, and
  sweepability
- market-impact primitives for Kyle-style lambda, Amihud-style illiquidity,
  calibrated expected impact, square-root impact, temporary/permanent impact,
  impact decay, and child-order attribution
- VPIN-style fixed-bucket toxicity primitives plus post-trade markout,
  adverse-selection scoring, quote-fade, toxic-burst, and informed-flow proxy
  signals
- fixed-window volatility/noise primitives with bipower variation, jump
  variation, OHLC range estimators, signature-plot points, and intraday
  seasonality buckets
- threshold-based regime classification plus composite trend/range/chop,
  liquidity, spread, session, hidden-liquidity, and transition-confidence
  labels
- feed-quality primitives for sequence gaps, out-of-order events, duplicates,
  stale events, locked/crossed books, timestamp skew, resets, health scoring,
  replay usability, primary issue selection, and operator review reports
- feature-vector primitives for stable feature ids, ordered schemas, schema
  hashes, missing-value policy, quality labels, reusable fixed-capacity
  writers, and extractor contracts
- resiliency primitives for threshold-based spread/depth shock detection,
  recovery timing, and liquidity resiliency scoring
- queue/fill primitives for passive queue position estimates, fill
  probability, expected time-to-fill, amend queue loss, top-level survival,
  cancel/replace cost, and maker/taker decision scoring
- pattern-risk primitives for spoofing/layering, quote-stuffing, stop-run,
  absorption, momentum-ignition, iceberg, hidden accumulation/distribution,
  stacked-imbalance, and failed-breakout risk indicators
- venue/route primitives for fill, reject, cancel, latency, route-health,
  venue-liquidity, toxicity, fill-quality, reliability, route-quality, drift,
  and degradation diagnostics
- cross-asset primitives for rolling correlation, beta, pair divergence,
  thresholded basis pressure, latency-adjusted correlation, cross-venue
  divergence, ETF/component imbalance, and relationship-degradation
  diagnostics
- derivatives primitives for put/call pressure, volume/open-interest anomaly,
  implied-volatility flow, gamma exposure, IV skew, term structure,
  implied-versus-realized richness, gamma pressure, futures basis, roll
  pressure, funding divergence, and aggregate derivatives stress
- feature profiles for future impact, toxicity, volatility, regime,
  data-quality, feature-vector, resiliency, queue-fill, cross-asset, patterns,
  derivatives, institutional, and ML feature modules
- borrowed `of_core::BookLevel` analysis paths that avoid copying book
  snapshots in hot analytics loops

Existing `of_core`, runtime, C ABI, Python, and Java analytics APIs remain
valid. The split is additive and provides a clean home for heavier models
without forcing all users to compile them.

### 1A. Market-data adapter SDK conformance

`of_adapters 0.4.0` keeps the existing `MarketDataAdapter` trait stable while
adding adapter-authoring and operator discovery helpers:

- expanded `AdapterQualityLevel` values for `Experimental`,
  `SimulatedCertified`, `PaperTrading`, `Certified`, and
  `ProductionObserved`
- optional certification and production-observed evidence fields on
  `AdapterDescriptor`
- `AdapterConformanceRequirement`, `AdapterConformanceFailure`, and
  `AdapterConformanceReport`
- `adapter_quality_requirements(...)`,
  `evaluate_adapter_conformance(...)`, and `adapter_conformance_report(...)`
- additive `AdapterOperationalStatus`, `AdapterRuntimeMode`, and
  `AdapterConnectionState` types plus a defaulted
  `MarketDataAdapter::operational_status()` method
- typed mode/session, reconnect, subscription, queue, loss, freshness, raw
  capture, and activity-age fields in active runtime status and metrics JSON
- centralized endpoint redaction that exposes scheme plus authority only and
  removes user information, paths, queries, and fragments

The conformance helpers are conservative. They report missing capability flags
or evidence for a requested quality target and do not construct adapters,
connect sockets, or upgrade any built-in provider claim automatically.

The operational-status method is defaulted, so existing third-party adapters
continue to compile. Status construction is queried explicitly and does not add
symbol sorting or string allocation to adapter polling. Existing C, Python, and
Java status functions receive the new fields through their existing JSON
payload and require no ABI or method-signature change.

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
- bounded deterministic certification venue with 18 scripted order/session
  scenarios, preserved report-sequence history, duplicate/out-of-order/resend
  injection, cancel/replace race handling, recovery restatement, degraded
  malformed-input behavior, transcript evidence, and completion coverage
- in-memory and file-backed execution journal helpers
- binary single-file and segmented WAL journals with versioned full submit,
  cancel, and amend payloads, backward-compatible legacy replay, deterministic
  no-checkpoint order reconstruction, and explicit pending cancel/replace
  recovery states
- recovery and open-order reconciliation primitives
- generalized recovery reconciliation across local OMS, WAL, checkpoint,
  adapter recovery, independent drop copy, broker positions, and position
  ledger evidence with verified watermarks, bounded machine-readable findings,
  deterministic mismatch classification, and explicit host policy actions
- recovery-readiness evaluation that gates resume decisions on WAL integrity,
  checkpoint integrity, recovery output, and reconciliation policy
- independent drop-copy adapter/session primitives with bounded canonical
  report buffering, source-scoped duplicate detection, venue/client order
  correlation, fill/state reconciliation, late-report handling, and
  allocation-free lag/health metrics
- bounded OMS command idempotency for submit/cancel/amend with scoped request
  IDs, semantic parameter mismatch rejection, stable command/client/provider ID
  mapping, strict lifecycle sequencing, fail-closed capacity, checksummed
  recovery checkpoints with canonical caller-buffer binary codecs,
  reconciliation-gated retries, and a checkpointed source-scoped
  execution-report duplicate horizon
- scoped fail-closed kill-switch primitives covering global, venue, route,
  account, strategy, symbol, order-type, and adapter-session controls, with
  bounded cancellation targets, reduce-only evaluation, complete actor/reason/
  timestamp/WAL audit metadata, idempotent cancel outcomes, and explicit forced
  clear evidence
- fixed-memory `ExecutionSloCollector` latency histograms with exact extrema
  and bounded p50/p95/p99 estimates, submit/cancel/replace reject rates,
  adapter/command/event queue gauges, WAL and checkpoint lag, recovery,
  reconciliation, route-health, and drop-copy indicators
- additive `ExecutionSloTargets` and machine-readable violation reports with
  explicit minimum-sample policy; exporter I/O and metric cardinality remain
  host-owned and outside the execution path
- `ExecutionOperatorController` with all runbook commands, fixed host-supplied
  permission bits, monotonic command identity, exact-retry idempotency,
  bounded receipt retention, requested/terminal audit phases, and fail-closed
  repair after post-effect audit failure
- direct global pause, route drain/degradation, scoped cancel, provider
  recovery, and bounded stuck-order inspection plus typed deployment services
  for reconciliation, incident export, segmented-WAL rotation, checkpointing,
  and kill-switch clear
- append-only `FileExecutionOperatorAudit` with complete typed action/scope/
  actor/reason/outcome frames, checksums, optional data sync, corruption and
  sequence validation, controller restore, and non-replaying restoration of
  local operator controls
- production `ExecutionAuditBundleExporter` with a fail-closed 12-class
  incident evidence profile, explicit custom bundles, bounded streamed file or
  in-memory artifacts, portable path and symlink validation, SHA-256 payload
  and manifest inventory, staged verification, unlisted-file rejection,
  immutable destinations, same-filesystem atomic publication, and independent
  post-export verification
- clear audit trust boundaries: source quiescence, redaction, destination
  permissions, signing/attestation, encryption, retention, legal hold, and
  chain-of-custody transfer remain deployment responsibilities and execute
  outside order/report paths
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
- `FixOwnedSequenceSnapshot` and `FileFixSequenceSnapshotStore` for atomic,
  checksum-validated latest sequence snapshot persistence across restarts
- bounded `FixResendStore` primitives for in-memory outbound frame retention,
  replay/gap-fill planning, and retention/drop/eviction metrics
- `FileFixDurableResendStore` for append-only, checksum-chained durable
  persistence of original outbound FIX frames and restart-time resend planner
  rebuilds
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

This is not a full FIX session engine. Transport, TCP/TLS-driven logon/logout,
automatic resend response transmission, venue certification, and
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
- additive `ProductionRiskEngine` policies across global, account, strategy,
  route, symbol, venue, and instrument-group scopes, with deterministic
  explainable decisions; bounded order/cancel rates; position, gross/net
  exposure, PnL, drawdown, session, price, and fat-finger controls; fail-closed
  health gates; and replaceable decision journaling
- additive `ProductionPositionLedger` with exact normalized open cost,
  realized/unrealized PnL, commission/fee/cash accounting, currency and
  multiplier metadata, scoped fill/adjustment idempotency, strict WAL-aligned
  sequences, corporate-action/manual hooks, versioned atomic checkpoints, and
  bounded broker/clearing position reconciliation
- additive `OrderIntentLifecycle` with immutable parent constraints, bounded
  native child ownership, fill/leaves aggregation, pause/resume, confirmed
  replacement lineage, deterministic cancel trees, late terminal-fill handling,
  strict WAL-aligned sequences, and recomputed recovery snapshots
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
- low-level plumbing: all manifest-exposed C functions now generate exact
  Python ctypes and Java JNA declarations from validated `orderflow.h` types in
  `bindings/api_manifest.toml` order
- deterministic TWAP bridge: C `of_execution_twap_algo_*`, Python
  `TwapExecutionAlgo`, and Java `TwapExecutionAlgo` expose owned child plans,
  explicit submit commit/discard, and execution progress without exposing Rust
  generic or borrowed planner internals
- signal research bridge: C `of_validate_signal_config_json` and
  `of_validate_signal_replay_json`, Python `SignalConfig` and
  `validate_signal_replay`, and Java `SignalConfig` and
  `OrderflowEngine.validateSignalReplay` construct descriptor-validated
  built-ins and return versioned reports with markout accuracy, coverage,
  retained samples, and replay-order warnings

Existing analytics handles and classes remain separate and unchanged.
High-level Python context managers/dataclasses and Java `AutoCloseable`/typed
wrappers remain manual. CI checks deterministic generated output, exact pointer
depth, callbacks, caller-owned buffers, output handles, JNA arrays, and
allocated-string mappings without changing any existing binding method.
The TWAP bridge does not submit directly: every child remains a canonical OMS
request and therefore still traverses configured risk, journaling, adapter,
kill-switch, and reconciliation paths.

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
