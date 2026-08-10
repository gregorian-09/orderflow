# Changelog
All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]
### Added
- Added read-only OMS state reconstruction across Rust, C, Python, and Java:
  root-based recovery never creates or mutates storage, checkpoint-required
  defaults fail closed, bounded identifier-free reports expose replay evidence,
  and submissions remain disabled pending venue reconciliation.
- Added backward-compatible full-payload OMS WAL command frames and
  request-aware `ExecutionJournal` hooks. Binary journals now retain complete
  submit, cancel, and amend requests, recovery can reconstruct orders without a
  checkpoint, and crash-boundary cancel/replace intent remains explicitly
  pending until venue reconciliation. Legacy command frames and the public
  `JournalRecord` replay shape remain supported unchanged.
- Added config-driven signal validation across Rust, C, Python, and Java:
  `SignalValidationReport::json_report`, registry validation diagnostics,
  `of_validate_signal_config_json`, `of_validate_signal_replay_json`, Python
  `SignalConfig`/`validate_signal_replay`, and Java
  `OrderflowEngine.validateSignalReplay`. The facade constructs built-ins from
  descriptor-checked parameters, preserves optional timestamp-order warnings
  and retained samples, and remains outside the live engine hot path.
- Added an additive deterministic TWAP binding facade across C, Python, and
  Java with an opaque native parent handle, allocation-free plan calls, owned
  child requests, retry-stable pending plans, explicit commit/discard after OMS
  submission, execution-event progress folding, and typed progress snapshots.
  Existing OMS/risk/journal and analytics APIs are unchanged.
- Added deterministic manifest/header-driven generation for all 97 low-level
  Python ctypes and Java JNA function signatures, including exact pointer
  depth, callbacks, output handles, caller buffers, JNA arrays, and allocated
  string ownership. CI checks committed output, unit-tests contextual mappings,
  and keeps the existing high-level Python/Java APIs manually designed and
  unchanged.
- Added typed market-data adapter operational status for transport mode,
  connection state, redacted endpoint, application name, reconnect attempts,
  sorted subscriptions, queue utilization, drop/gap counters, stale state,
  raw-capture utilization, and activity ages. The adapter trait method is
  defaulted for source compatibility, status work stays off the event hot path,
  and existing runtime/C/Python/Java status JSON gains fields additively.
- Hardened built-in adapter diagnostics so endpoint user information, path,
  query, and fragment components cannot enter status output; this also closes
  the previous Rithmic `protocol_info` endpoint disclosure.
- Added production OMS incident bundle export with a fail-closed 12-class
  evidence profile, explicit custom profile, bounded streaming collection,
  portable path and symlink defenses, SHA-256 payload/manifest inventory,
  staged self-verification, unlisted-file detection, atomic publication,
  immutable destinations, and independent post-transfer verification while
  keeping redaction, signing, encryption, retention, and custody host-owned.
- Added a complete additive OMS operator runbook controller for pause/resume,
  route drain/degradation, cancel-all/scoped cancel, recovery, reconciliation,
  audit export, stuck-order inspection, WAL rotation, checkpointing, and
  kill-switch clear, with host-supplied permissions, idempotent receipts,
  intent-before-effect journaling, checksummed file audit, and restart restore.
- Added fixed-memory execution SLI collection and explicit SLO evaluation for
  submit/send/ack/cancel/replace/fill/recovery/drop-copy latency distributions,
  submit/cancel/replace reject rates, queue depth, WAL/checkpoint lag,
  reconciliation mismatches, and route health, with atomic observation
  validation and host-owned low-cardinality export.
- Added a bounded deterministic OMS certification venue implementing the
  existing execution-adapter contract, with 18 scripted accept/fill/reject,
  cancel/replace race, duplicate/out-of-order/resend, sequence reset,
  disconnect/reconnect, recovery, slow-delivery, and malformed-provider
  scenarios plus sequenced report history, transcript evidence, and complete
  coverage reporting.
- Added a generalized, additive OMS reconciliation coordinator across local,
  WAL, checkpoint, adapter recovery, drop-copy, broker-position, and position-
  ledger evidence with sequence/time/count watermarks, deterministic scoped
  order comparison, all mismatch classes, bounded findings, and explicit
  fail-closed/venue-truth/cancel/restate/operator policy actions.
- Added bounded, recovery-safe OMS command idempotency for submit, cancel, and
  amend requests with scoped request keys, semantic parameter matching, stable
  OMS/provider ID correlation, fail-closed capacity, strict sequencing,
  canonical caller-buffer binary checkpoints, reconciliation-gated retries,
  and a separately checkpointed source-scoped execution-report duplicate
  window.
- Added an additive bounded OMS order-intent and native parent/child lifecycle
  with constrained child allocation, canonical fill aggregation, pause/resume,
  replace lineage, atomic cancel trees, late-fill handling, strict sequencing,
  and independently validated recovery snapshots.
- Added an authoritative, additive production position/PnL ledger with exact
  average-cost basis, long/short/reversal realization, marks, fees,
  commissions, cash and corporate-action adjustments, rational FX conversion,
  scoped fill attribution/deduplication, strict mutation sequencing,
  checksummed atomic checkpoints, and bounded external reconciliation.
- Added an additive scoped production risk engine with deterministic policy
  ordering, bounded independent order/cancel rate windows, projected position
  and exposure limits, PnL/session/price/fat-finger controls, fail-closed runtime
  health gates, explainable decisions, and a replaceable decision journal.
- Added scoped, fail-closed OMS kill switches for global, venue, route,
  account, strategy, symbol, order-type, and adapter-session controls, with
  bounded cancellation targets, reduce-only evaluation, typed actor/reason/WAL
  audit events, idempotent cancel results, and explicit forced-clear evidence.
- Added independent OMS drop-copy primitives with a separate adapter/session
  contract, canonical bounded report buffers, deterministic simulation source,
  source-scoped duplicate suppression, venue/client order correlation,
  fill/state reconciliation flags, explicit late-report policy, and
  allocation-free health/lag metrics.
- Added `of_adapters` adapter SDK conformance helpers with an expanded
  production adoption quality ladder, descriptor evidence fields, target
  quality requirements, and fail-closed conformance reports for adapter authors
  and operator tooling.
- Added new `of_analytics` crate foundation with dependency-light
  market-quality/TCA primitives, liquidity/depth primitives, feature profiles
  for future advanced analytics modules, and borrowed `of_core::BookLevel`
  analysis paths that avoid copying book snapshots.
- Added `of_analytics` execution-quality/TCA primitives for implementation
  shortfall, arrival and decision slippage, adverse selection, trade-through,
  and fill-quality scoring.
- Hardened `of_analytics` liquidity/depth analytics with depth convexity,
  book-pressure, target sweepability scores, order-flow imbalance,
  replenishment/depletion rates, and liquidity-drought detection.
- Hardened `of_analytics` market-impact analytics with calibrated expected
  impact, square-root impact, temporary/permanent impact, impact decay, and
  child-order attribution primitives.
- Hardened `of_analytics` toxicity analytics with post-trade markout,
  adverse-selection scoring, quote-fade measurement, informed-flow proxy, and
  toxic-flow burst detection.
- Hardened `of_analytics` volatility analytics with bipower variation, jump
  variation, OHLC range estimators, volatility signature points, and intraday
  seasonality buckets.
- Hardened `of_analytics` regime analytics with composite trend/range/chop,
  liquidity, spread, session, hidden-liquidity, volatility, and transition
  confidence labels.
- Hardened `of_analytics` pattern-risk analytics with iceberg/hidden-refresh,
  hidden accumulation/distribution, stacked-imbalance, absorption-strength, and
  failed-breakout diagnostics.
- Hardened `of_analytics` queue/fill analytics with cancel/replace cost,
  priority-loss, wait-penalty, passive-edge, aggressive-cost, and maker/taker
  decision primitives.
- Added `of_analytics` market-impact and VPIN-style toxicity primitives with
  Kyle-style lambda, Amihud-style illiquidity, fixed-capacity VPIN buckets, and
  deterministic integer-scaled snapshots.
- Added `of_analytics` fixed-window volatility/noise tracking and
  threshold-based regime classification with quiet, normal, volatile, toxic,
  and illiquid regimes.
- Added `of_analytics` feed-quality analytics for sequence gaps,
  out-of-order events, duplicates, stale events, locked/crossed books,
  timestamp skew, sequence resets, rates, flags, health scoring, replay
  usability, primary issue selection, and operator review reports.
- Added `of_analytics` feature-vector APIs for stable feature ids, schemas,
  schema hashes, missing-value policies, per-feature quality labels, reusable
  fixed-capacity writers, and extractor contracts.
- Added `of_analytics` resiliency analytics for threshold-based spread/depth
  shock detection, recovery timing, and liquidity resiliency scoring.
- Added `of_analytics` queue/fill analytics for passive queue position,
  fill probability, expected time-to-fill, amend queue loss, and maker/taker
  scoring.
- Added `of_analytics` pattern-risk analytics for spoofing/layering,
  quote-stuffing, stop-run, absorption, and momentum-ignition indicators.
- Added `of_analytics` venue/route analytics for fill, reject, cancel,
  latency, route-health, venue-liquidity, toxicity, fill-quality, reliability,
  route-quality, drift, and degradation diagnostics.
- Added `of_analytics` cross-asset analytics for rolling correlation, beta,
  pair divergence, thresholded basis pressure, latency-adjusted correlation,
  cross-venue divergence, ETF/component imbalance, and
  relationship-degradation diagnostics.
- Added `of_analytics` derivatives analytics for put/call pressure,
  volume/open-interest anomaly, implied-volatility flow, gamma exposure,
  IV skew, term structure, implied-versus-realized richness, gamma pressure,
  futures basis, roll pressure, funding divergence, and aggregate derivatives
  stress.
- Added `tools/check_binding_parity.py` and CI coverage to validate that
  manifest-exposed C ABI symbols have matching Python ctypes registrations and
  Java JNA declarations before release.
- Added generated binding API inventory documentation from
  `bindings/api_manifest.toml`, with CI coverage to keep the symbol tables in
  sync with the manifest.
- Expanded the generated binding inventory with a per-symbol C ABI, Python
  ctypes, and Java JNA compatibility matrix.
- Added structured active adapter metadata to `Engine::metrics_json()` under an
  additive `adapters` array while preserving existing flat metrics fields.
- Added `ExecutionRunbookSnapshot` and `ExecutionEngine::runbook_snapshot()` for
  read-only OMS operator dashboards and incident runbooks.
- Added `ExecutionAuditBundleManifest` plus deterministic engine manifest
  capture as the state input to production incident bundle export.
- Added optional multi-account allocation primitives with proportional and
  priority average-price allocation reports plus allocation reconciliation.
- Added optional execution timestamp discipline helpers for source-tagged
  workflow traces, latency attribution, monotonic validation, and exchange/OMS
  clock-skew checks.
- Added configurable execution safety policy helpers with conservative
  fail-closed defaults, explicit degraded fail-open actions, operator approval
  gating, and per-condition decision reports.
- Added production deployment templates covering simulation, paper trading,
  shadow mode, live trading, capture, replay, active/passive recovery, and
  disaster recovery drills.
- Added additive `of_persist` binary normalized market-data WAL primitives with
  fixed-frame append, replay, checksum/sequence integrity inspection, sync
  policy, and write metrics while preserving existing JSONL APIs.
- Added additive `of_persist` production market-data persistence policy and
  health snapshots for writer mode, failure action, lag, drops, and error
  reporting.
- Added additive `of_persist` market-data backpressure policy helpers for
  queue-depth, lag, pending-byte, degraded-state, drop-policy, trade-preserve,
  and critical-record decisions.
- Added additive `of_persist` market-data checkpoint store helpers with opaque
  payloads, WAL sequence anchors, validation, latest-checkpoint loading, and
  retention pruning for bounded production recovery.
- Added additive `of_persist` market-data recovery planner helpers that classify
  checkpoint/WAL restore states into clean, degraded, snapshot-gated, or
  fail-closed host actions.
- Added additive `of_persist` WAL replay filters for deterministic replay by
  WAL sequence, provider sequence, normalized event sequence, timestamps, and
  record kind.
- Added additive `of_persist` JSONL cold-export helpers with partition
  manifests, payload hex preservation, WAL replay filtering, and dependency-free
  research export files.
- Added additive `of_persist` retention/tiering planner helpers for hot WAL,
  verified cold export, checkpoint dependency, and incident-window decisions.
- Added Binance adapter depth update-id continuity tracking with duplicate
  depth-update suppression, gap degradation, snapshot rebuild counters, and
  expanded health metadata.
- Added Binance adapter health hardening with redacted endpoint metadata and
  deterministic per-symbol last depth update-id reporting.
- Added opt-in Binance adapter pending event queue bounds with dropped-event and
  backpressure health counters while preserving the default unbounded queue
  behavior.
- Added Binance adapter parse and normalization latency health metrics with
  aggregate sample, average, and max nanosecond fields.
- Added opt-in bounded Binance raw inbound message capture for incident
  analysis and fixture generation, with capture depth, capacity, and drop
  health counters.
- Added Binance raw fixture replay through the live parser/normalizer path with
  deterministic receive timestamps for repeatable adapter tests.
- Added adapter descriptor capability flags and inventory JSON fields for
  backpressure, raw capture, fixture replay, stale detection, and latency
  metrics.
- Added deterministic jitter to Binance reconnect backoff and exposed the last
  scheduled reconnect delay in health metadata.
- Added new `of_fix` crate foundation with borrowed FIX tag-value parsing,
  `BodyLength(9)` and `CheckSum(10)` validation, common tag helpers,
  caller-owned encoding buffers, and diagnostic rendering.
- Added `of_fix` typed `FixVersion`/`FixMsgType` helpers, reusable
  `FixEncoder`/`FixDecoder` facades, and static `FixDictionary` profile
  validation for required and disallowed message tags.
- Added `of_fix` borrowed Reject `<3>` and BusinessMessageReject `<j>` parsers
  for low-allocation counterparty/session diagnostics.
- Added `of_fix` session-state and sequence-tracking primitives with
  deterministic accept/gap/duplicate/too-low outcomes, resend ranges, monotonic
  outbound assignment, and guarded sequence-reset advancement.
- Added `of_fix` borrowed session identity and sequence snapshot primitives for
  storage-neutral persistence of next inbound/outbound sequence counters.
- Added `of_fix` owned sequence snapshots and
  `FileFixSequenceSnapshotStore` for atomic, checksum-validated persistence of
  latest FIX session sequence state across restart/reconnect workflows.
- Added `of_fix` bounded resend-store primitives for replay/gap-fill planning
  with explicit message/byte retention metrics and sequence guardrails.
- Added `of_fix` `FileFixDurableResendStore` for append-only,
  checksum-chained durable persistence of original outbound FIX frames and
  restart-time rebuilds of the bounded resend planner.
- Added `of_fix` bounded transcript-capture primitives for certification/audit
  evidence with optional raw retention, metadata-only records, and rolling hash
  metrics.
- Added `of_fix::encode_poss_dup_replay` for possible-duplicate resend frame
  encoding with current `SendingTime(52)` and preserved `OrigSendingTime(122)`.
- Added `of_fix` session/admin builders for Logon, Heartbeat, TestRequest,
  ResendRequest, SequenceReset gap fill, and Logout using caller-owned buffers.
- Added `of_fix` typed order-entry builders for NewOrderSingle,
  OrderCancelRequest, and OrderCancelReplaceRequest with borrowed wire-format
  quantity and price fields.
- Added optional `Account(1)` support to `of_fix` NewOrderSingle,
  OrderCancelRequest, and OrderCancelReplaceRequest builders.
- Added optional `StopPx(99)` support to `of_fix` NewOrderSingle and
  OrderCancelReplaceRequest builders.
- Added `of_fix` OrderStatusRequest `<H>` builder with required `ClOrdID(11)`
  and optional `OrderID(37)`.
- Added `of_fix` OrderMassCancelRequest `<q>` builder with typed
  `MassCancelRequestType(530)` scopes and optional qualifier fields.
- Added `of_fix` OrderMassStatusRequest `<AF>` builder with typed
  `MassStatusReqType(585)` scopes and optional account/session/symbol/side
  qualifiers.
- Added `of_execution_adapters::fix::parse_execution_report`, parse config, and
  parse errors to convert validated `of_fix::FixMessageView` execution reports
  into canonical FIX execution reports with explicit quantity/price scales.
- Added `of_execution_adapters::fix::parse_order_cancel_reject` and
  `map_order_cancel_reject` to convert FIX `OrderCancelReject(35=9)` messages
  into canonical cancel/replace reject execution events.
- Added `of_execution_adapters::fix` outbound request encoding bridge for
  canonical OMS order, cancel, and amend requests using `of_fix` builders,
  explicit decimal scales, caller-provided FIX timestamps, and `Account(1)`
  propagation.
- Added stop and stop-limit new-order encoding support to the
  `of_execution_adapters::fix` outbound request bridge.
- Added `of_execution_adapters::fix::encode_stop_amend_request` and
  `FixStopAmendEncodeContext` for explicit stop/stop-limit amend encoding.
- Added new `of_execution_algos` crate with parent/child execution-algorithm
  identifiers, lifecycle statuses, child-order plans, fixed-capacity decisions,
  execution-event progress folding, and deterministic TWAP slice planning.
- Added deterministic TWAP replay primitives in `of_execution_algos` with
  explicit timer/execution/status inputs, caller-owned replay-step output,
  generated replay ids, and summary hashes for regression checks.
- Added deterministic POV participation planning in `of_execution_algos` with
  target/max participation rates, parent participation caps, observed volume
  inputs, and explicit clip handling.
- Added deterministic VWAP curve planning in `of_execution_algos` with borrowed
  cumulative volume profiles, O(1) bucket lookup, and explicit clip handling.
- Added deterministic synthetic iceberg replenishment planning in
  `of_execution_algos` with display quantity, replenish threshold, and
  remaining-parent leaves handling.
- Added deterministic implementation-shortfall planning in
  `of_execution_algos` with arrival-price context, adverse-move detection,
  urgency weights, impact patience, and auditable target-release estimates.
- Added deterministic passive queue planning in `of_execution_algos` with
  host-owned best bid/ask, queue-ahead quantity, expected contra volume,
  adverse-selection estimates, passive improvement, and optional late crossing.
- Added deterministic smart-order-routing planning in `of_execution_algos`
  with route status, order-type capability, route metrics, configurable score
  weights, fixed-capacity allocations, and OMS-safe child plans.
- Added deterministic liquidity-seeking planning in `of_execution_algos` with
  SOR candidate reuse, hidden-liquidity and price-improvement scoring,
  probe/take decisions, toxicity filtering, minimum quantity checks, and
  OMS-safe route-specific child plans.
- Added deterministic sweep/aggressive-take planning in `of_execution_algos`
  with side-aware price collars, minimum fill quantity suppression,
  route/level capacity, average planned price, and OMS-safe child plans.
- Added deterministic basket planning in `of_execution_algos` with leg
  roles, hedge-ratio metadata, synchronized per-leg release, fixed-capacity
  decisions, and explicit non-atomic multi-leg semantics.
- Added deterministic pairs/spread planning in `of_execution_algos` with
  hedge-ratio sizing, executable spread-edge gating, synchronized buy/sell
  child plans, ratio-aware quantity clipping, and explicit legging-risk
  boundaries.
- Added additive algorithm risk controls in `of_execution_algos` with typed
  limits, host-owned risk context, fixed-capacity explainable violation
  reports, kill-switch/operator-pause outcomes, price collars, participation,
  notional, child quantity, open quantity, child-count, stale-data,
  route-degradation, and persistence-degradation checks.
- Added additive algorithm checkpoint and recovery planning in
  `of_execution_algos` with schema-versioned parent/progress snapshots, replay
  cursors, decision sequence restoration, pause/resume/complete/escalate
  recovery actions, and explicit separation from OMS WAL ownership.
- Added deterministic child-order simulation in `of_execution_algos` with
  explicit fill model inputs, canonical `ExecutionEvent` output, fixed-capacity
  simulation reports, fill/reject/cancel/resting outcomes, simulated latency,
  deterministic venue/execution id generation, and direct progress folding.
- Added allocation-free algorithm metrics and TCA accumulation in
  `of_execution_algos` with child submission counts, fill/reject/cancel counts,
  completion, average execution price, side-aware arrival/VWAP/TWAP slippage,
  first/last timestamps, and average event latency.
- Added typed algorithm configuration in `of_execution_algos` with
  `AlgoKind`, `AlgoParentConfig`, and `AlgoConfig`, allowing hosts to build
  existing `ParentOrder`, risk policy, and recovery policy values without
  free-form maps or a forced serialization dependency.
- Added deterministic market-making quote planning in `of_execution_algos`
  with fair-value quoting, inventory skew, volatility/adverse-selection spread
  widening, inventory-limit side suppression, and OMS-safe bid/ask child plans.
- Additive `of_signals` metadata and lifecycle APIs:
  `SignalDescriptor`, `SignalInputMask`, `SignalWarmupRequirement`,
  `SignalWarmupProgress`, `SignalLifecycleState`, `SignalLifecycle`,
  signal parameter metadata, output semantics, built-in descriptor constants,
  `built_in_signal_descriptors()`, and `describe_signal()`. Existing
  `SignalModule` implementations and built-in signal behavior are unchanged.
- Additive `of_signals` contextual API with `SignalContext`,
  `ContextualSignalModule`, and `LegacySignalAdapter`, allowing richer signal
  hosts to pass analytics, data quality, symbol/book references, timestamps,
  lifecycle state, and extension tags without changing `SignalModule`.
- Additive `of_signals` stabilization policies with `HysteresisPolicy`,
  `DebouncePolicy`, `CooldownPolicy`, `SignalStabilizer`, and
  `StabilizedSignal` so hosts can opt into deterministic anti-flapping behavior
  without changing built-in signal outputs.
- Additive `of_signals` explainability APIs with `SignalReasonCode`,
  `SignalExplanation`, `SignalInputValue`, `SignalThreshold`,
  `SignalConfidenceComponent`, `SignalExplanationMode`, and
  `ExplainableSignalModule`. Built-in signals now expose structured diagnostic
  explanations while existing `SignalModule` and `SignalSnapshot` contracts are
  unchanged.
- Additive `of_signals` registry/config APIs with `SignalRegistry`,
  `SignalRegistration`, `SignalConfig`, typed config parameters,
  `SignalRegistryError`, built-in registrations, config validation,
  built-in construction, input filtering, and dependency-free descriptor JSON
  export for bindings and dashboards.
- Additive `of_signals` replay validation harness with `SignalValidationConfig`,
  `SignalValidationHarness`, `SignalValidationReport`, markout labels,
  optional timestamp-order warnings, confidence filtering, retained samples, and
  dependency-free JSON summaries for notebook/Python-style workflows.
- Additive `of_signals` calibration and outcome-tracking APIs with
  `SignalConfidenceCalibrator`, `SignalCalibrationCurve`,
  `SignalOutcomeTracker`, `SignalCalibrationReport`, per-regime summaries, and
  drift reports so replay/live outcomes can be checked for confidence
  calibration without changing existing signal modules.
- Additive `of_signals` ensemble framework with `SignalEnsemblePolicy`,
  `SignalEnsembleVote`, reusable majority/quorum/weighted rules, veto handling,
  conflict resolution, metrics, and child-explanation aggregation. Existing
  `CompositeSignal` behavior is unchanged.
- Additive `of_signals` checkpoint and shadow-mode primitives with
  `SignalCheckpoint`, restore validation policies/reports,
  `CheckpointableSignal`, `SignalRunMode`, `SignalShadowRecorder`, and
  production-versus-candidate comparison reports for safe live validation
  without changing existing signal modules.
- Additive `of_signals` feature-vector and model metadata APIs with
  `FeatureSchema`, `FeatureVectorView`, feature-quality validation,
  `SignalModelMetadata`, `SignalModelInputBinding`, `SignalModelOutput`, and
  optional `ModelBackedSignal` support while keeping heavy model runtimes out of
  default builds.
- Additive `of_execution_core` binary WAL frame primitives with
  `WalSequence`, `WalSegmentId`, `WalRecordKind`, `WalSyncPolicy`,
  `WalRecordHeader`, `WalRecordView`, `WalReplayCursor`,
  `WalIntegrityReport`, checksum validation, strict sequence checks, and
  borrowed replay support for future low-latency OMS persistence.
- Additive `of_execution` `WalExecutionJournal` with `WalJournalConfig`,
  `WalReplayResult`, `WalJournalMetrics`, binary WAL frame persistence,
  replay-from-sequence, integrity reporting, configurable sync policy, write
  and sync latency counters, and fail-closed validation of existing WAL bytes
  while preserving the existing `ExecutionJournal` trait.
- Additive `of_execution` checkpoint store APIs with `ExecutionCheckpoint`,
  `CheckpointPosition`, `CheckpointConfig`, `CheckpointPolicy`,
  `CheckpointManifest`, `ExecutionCheckpointStore`, and
  `FileExecutionCheckpointStore` for checksum-validated, atomic file-backed OMS
  snapshots keyed by last applied WAL sequence.
- Additive `of_execution` segmented WAL APIs with `WalSegmentConfig`,
  `WalSegmentMetadata`, `WalSegmentManifest`, `WalSegmentIntegrityReport`, and
  `SegmentedWalExecutionJournal` for rotated binary OMS WAL directories with
  segment seals, manifest inventory, cross-segment checksum validation, and
  replay through the existing `ExecutionJournal` model.
- Additive `of_execution` recovery APIs with `RecoveryPlan`,
  `RecoveryCorruptionPolicy`, `RecoveryVenuePolicy`, `RecoveredOmsState`,
  `RecoveryResult`, and segmented-WAL recovery helpers that load the latest
  checkpoint, replay the WAL tail, fail closed on unsafe unknown orders, and
  require venue reconciliation by default.
- Additive `of_execution` recovery-readiness gate with
  `RecoveryReadinessConfig`, `RecoveryReadinessBlocker`,
  `RecoveryReadinessDecision`, and `evaluate_recovery_readiness` so production
  hosts can combine WAL integrity, checkpoint integrity, recovery output, and
  reconciliation policy into one fail-closed resume decision.
- Additive `of_execution` venue reconciliation policy APIs with
  `ReconciliationIssueKind`, `VenueReconciliationReport`,
  `ReconciliationPolicy`, `ReconciliationPolicyDecision`, detailed mismatch
  classification, and `ExecutionEngine` helpers for evaluating venue
  open-order snapshots without mutating local state or sending cancels.
- Additive execution WAL integrity diagnostics across the C ABI, Python, and
  Java bindings: `of_execution_wal_integrity_report`,
  `of_execution_segmented_wal_integrity_report`, `inspect_execution_wal`,
  `inspect_execution_segmented_wal`, `OrderflowExecutionEngine.inspectWal`,
  and `OrderflowExecutionEngine.inspectSegmentedWal` expose offline WAL scans
  without changing existing execution handles or order-path APIs.
- Additive checkpoint-store integrity diagnostics across the C ABI, Python,
  and Java bindings: `of_execution_checkpoint_store_integrity_report`,
  `inspect_execution_checkpoint_store`, and
  `OrderflowExecutionEngine.inspectCheckpointStore` expose offline checkpoint
  validation and latest-valid-checkpoint metadata without mutating checkpoint
  stores.
- Additive market-data adapter discovery APIs across Rust, C ABI, Python, and
  Java: `AdapterDescriptor`, `adapter_descriptors()`,
  `compiled_adapter_descriptors()`, runtime adapter inventory/status JSON,
  `of_get_adapter_inventory_json`, `of_get_active_adapter_status_json`,
  `available_adapters()`, `adapter_inventory()`, and Java
  `OrderflowEngine.adapterInventory()` / `adapterStatus()` expose provider
  capabilities, feature gates, quality level, and active health without
  changing adapter polling or ingest behavior.
- Additive binding API manifest tooling with `bindings/api_manifest.toml`,
  `tools/check_api_manifest.py`, and manifest-driven C ABI export checks so
  exported native symbols and `orderflow.h` declarations stay synchronized
  before low-level Python/JNA generation is expanded.
- Additive signal descriptor discovery bridge across runtime, C ABI, Python,
  and Java: `signal_descriptor_inventory_json()`,
  `of_get_signal_descriptors_json`, Python `signal_descriptors()`, and Java
  `OrderflowEngine.signalDescriptors()` expose built-in signal metadata without
  changing signal evaluation or OMS routing behavior.
- Additive signal explanation bridge with `SignalModule::latest_explanation()`
  default hook, `SignalExplanation::to_json()`, per-symbol runtime explanation
  cache, `of_get_signal_explanation_json`, Python
  `Engine.signal_explanation(symbol)`, and Java
  `OrderflowEngine.signalExplanation(symbol)` for audit/dashboard diagnostics
  without changing existing signal snapshots.
- Additive signal metrics bridge with `Engine::signal_metrics_json`,
  `of_get_signal_metrics_json`, Python `Engine.signal_metrics()`, and Java
  `OrderflowEngine.signalMetrics()` for state counts, confidence summaries,
  quality-flagged signal counts, and explanation coverage diagnostics.

### Fixed
- Fixed release version synchronization for mixed-version workspace crates.
  Internal path dependencies are now checked against the effective version of
  their target crate manifest, so established `0.4.0` crates and new `0.1.0`
  analytics/execution/FIX crates can advance independently without hard-coded
  override drift.
- Hardened `of_ffi_c` native ABI tests so a failed test does not poison the
  shared FFI test lock and cascade misleading `PoisonError` failures through
  unrelated tests.

## [0.4.0] - 2026-06-04
### Added
- Broad additive analytics expansion across Rust, C, Python, and Java:
  spread/book-event/resiliency helpers; VPIN, Kyle's Lambda, Amihud, and CVD
  enhancement snapshots; practitioner pattern detection; volatility, noise,
  Hasbrouck, Almgren-Chriss, spread decomposition, ACD, regime, kinetic-energy,
  agent-type, dark-pool, options-flow, futures, dark-lit correlation,
  institutional-flow, OI-analysis, and LOB-feature APIs.
- `AnalyticsConfig` tuning surface across Rust, C, and Python for rolling
  window lengths, thresholds, and event-rate windows.
- Tickbar integration: fixed-interval OHLCV bar aggregation using the `tickbar`
  crate. New `CompletedBar` domain type, `AnalyticsAccumulator::with_tickbar()`,
  `bar_series()`, and `reset_tickbar()` methods. Exposed through C ABI
  (`of_engine_set_tickbar_interval`, `of_get_bar_series`), Python binding
  (`set_tickbar_interval()`, `bar_series()`), and Java binding
  (`setTickbarInterval()`, `barSeries()`). Feature-gated behind `tickbar`
  (off by default).
- Strategy cookbook expanded to 30 strategy examples with payload-key-accurate
  examples and API compatibility mapping across Rust, C, Python, and Java.
- Additive execution-core foundation for order-management workflows:
  `of_execution_core`, `of_execution`, and `of_execution_adapters` introduce
  fixed-size execution identifiers, typed order/cancel/amend requests,
  FIX-style order-state transitions, structured pre-trade risk rejection,
  bounded execution event buffers, simulated execution, journal/recovery hooks,
  and a FIX execution-report mapping scaffold.
- Additive execution C ABI, Python, and Java APIs using separate execution
  handles (`of_execution_engine_t`, `ExecutionEngine`,
  `OrderflowExecutionEngine`). Existing analytics/runtime APIs are unchanged.
- Multi-route execution engines for multi-symbol order flow. Rust execution now
  indexes route/account/symbol configs, applies route-scoped open-order and
  notional risk accounting, and exposes additive C (`of_execution_engine_create_multi`),
  Python (`ExecutionEngine([routes...])`), and Java
  (`new OrderflowExecutionEngine(path, List<RouteConfig>)`) entry points.
- Additive Rust `ConcurrentExecutionEngine` worker wrapper for concurrent order
  producers. The wrapper keeps the synchronous execution engine owned by one
  dedicated thread, uses bounded command/report channels, and preserves serial
  deterministic order-state transitions.
- Full additive OMS support primitives: command/request correlation, bounded
  execution-event fanout, adapter lifecycle snapshots, file-backed durable
  execution journal, open-order reconciliation, disconnect/kill-switch safety
  policies, advanced risk gate helpers, position/fill ledger, order-type/TIF
  normalization, execution telemetry, deterministic sharding, token-bucket
  throttling, replayable OMS simulation, and provider adapter SDK helpers.
- Concurrent execution runtime access across C, Python, and Java through
  additive handles/classes (`of_execution_concurrent_engine_t`,
  `ConcurrentExecutionEngine`, and `ConcurrentOrderflowExecutionEngine`).
- Expanded handbook coverage for the execution and OMS subsystem, including
  crate references, OMS architecture, multi-symbol cookbook workflows,
  low-latency design guidance, provider adapter authoring, and recovery
  operations.

### Changed
- Existing analytics/runtime/native/binding package line advances to `0.4.0`.
- New Rust execution crates publish as `0.1.0`:
  `of_execution_core`, `of_execution`, and `of_execution_adapters`.
- Root README, crate READMEs, Python README, Java README, strategy handbook,
  and OMS cookbook now focus on the `0.4.0` analytics-to-execution workflow.

### Upgrade Notes
- Existing analytics/runtime APIs are intended to remain source-compatible.
- Update Python/Java packages and the native `of_ffi_c` runtime/header together
  to `0.4.0`.
- If you build native execution providers, depend on the execution crates at
  compatible `0.1.x` versions and follow adapter-authoring guidance.
- Treat simulated execution as a deterministic test and integration tool, not a
  broker-certified live OMS.

## [0.3.0] - 2026-05-09
This is a non-breaking operational hardening release after `0.2.0`. For the
complete user-facing release guide, see
[`docs/ops/release-0.3.0.md`](docs/ops/release-0.3.0.md).

### Added
- Optional dashboard token authentication through `OF_DASH_TOKEN`, disabled by
  default for local development compatibility.
- Dashboard Prometheus `/metrics` endpoint with runtime counters, quality flags,
  adapter status, backpressure counters, aggregate health, and circuit-breaker
  state.
- Runtime opt-in backpressure with `Engine::with_max_events_per_poll(Some(n))`
  and `OF_RUNTIME_MAX_EVENTS_PER_POLL`.
- Runtime aggregate adapter health fields in health/metrics JSON:
  `adapter_total_count`, `adapter_healthy_count`, and
  `runtime_health_status`.
- Runtime opt-in adapter circuit breaker with
  `Engine::with_circuit_breaker(...)`,
  `OF_RUNTIME_CIRCUIT_BREAKER_FAILURES`, and
  `OF_RUNTIME_CIRCUIT_BREAKER_COOLDOWN_MS`.
- Additive circuit-breaker health/metrics fields:
  `circuit_breaker_enabled`, `circuit_breaker_open`,
  `circuit_breaker_consecutive_failures`, `circuit_breaker_opened_count`, and
  `circuit_breaker_cooldown_ms`.
- Additive JSONL persistence record metadata: `"schema": 1`,
  `ts_exchange_ns`, and `ts_recv_ns`.
- Runtime end-to-end persistence replay parity regression covering ingest,
  persistence, readback, replay, analytics, signals, and materialized book
  state.
- Python PEP 561 marker (`py.typed`) for type-checker support.
- Python bundled native library lookup under `orderflow/native/`.
- GitHub Actions workflow for platform-tagged Python binary wheels.

### Changed
- Package versions are aligned at `0.3.0` for Rust, C, Python, and Java.
- Python publish workflow now builds an sdist plus a Linux platform wheel with
  the native `of_ffi_c` runtime staged inside the package.
- Cargo lockfile dependency selections were kept compatible with the project's
  minimum supported Rust/Cargo toolchain.

### Upgrade Notes
- This release is additive and non-breaking for existing `0.2.0` integrations.
- Existing Rust, C, Python, and Java APIs continue to work; no required
  rename/removal migration is needed.
- If you use direct C or native bindings, update the native library and header
  together.
- If you use Python or Java, keep the binding package and native library on the
  same release version.
- Consider enabling `OF_DASH_TOKEN` before exposing the dashboard outside a
  trusted local environment.
- Consider enabling backpressure and circuit breaking for live deployments that
  need explicit failure containment.

## [0.2.0] - 2026-04-08
This is the first hardening-focused feature release after the initial `0.1.x`
line. For the complete user-facing release guide, see
[`docs/ops/release-0.2.0.md`](/home/gregorian-rayne/RustroverProjects/orderflow/docs/ops/release-0.2.0.md).

### Added
- Rust crate front-page documentation (`//!`) for `of_core`, `of_signals`,
  `of_persist`, `of_adapters`, `of_runtime`, and `of_ffi_c`, including
  purpose, architecture notes, and quick-start examples.
- C ABI API documentation comments for exported `of_ffi_c` symbols and FFI
  structs to improve docs.rs discoverability for non-Rust integrators.
- Java package-level JavaDoc (`package-info.java`) for
  `com.orderflow.bindings` and `com.orderflow.examples`.
- Richer Python module-level API documentation for `orderflow.api`,
  `orderflow._ffi`, and package root `orderflow`.
- JavaDoc overview page (`bindings/java/src/main/javadoc/overview.html`) to
  provide a richer published API landing page for Maven consumers.
- C SDK distribution packaging in `.github/workflows/release-native-artifacts.yml`,
  now publishing versioned platform archives with header, libraries, pkg-config
  metadata, and SDK README.
- C API header constants for stream kinds and data-quality flags
  (`of_stream_kind_t`, `of_data_quality_flags_t`) for first-class C developer
  ergonomics.
- C usage example: `examples/c/basic.c`.
- Official vcpkg submission scaffold for C consumers:
  `packaging/vcpkg/official/ports/orderflow-c` with portfile, manifest,
  and usage docs.
- Release helper script:
  `tools/release/sync_vcpkg_registry_baseline.py` to auto-sync the published
  `orderflow-vcpkg-registry` baseline SHA into tracked docs/config examples.
- Ops release runbook:
  `docs/ops/release_checklist.md` with binding/version sync, vcpkg baseline
  sync, docs coverage checks, and pre-publish validation commands.
- Runtime book snapshots across Rust, C, Python, and Java, including
  materialized snapshot queries and `BOOK_SNAPSHOT` callback delivery.
- Additive derived analytics snapshot APIs (`total_volume`, `trade_count`,
  `vwap`, `average_trade_size`, `imbalance_bps`) across Rust/C/Python/Java,
  plus `DERIVED_ANALYTICS` callback delivery.
- Additive candle-style analytics snapshots:
  `session_candle_snapshot` and rolling `interval_candle_snapshot(window_ns)`
  across Rust/C/Python/Java.
- Persistence readback/discovery APIs for venues, symbols, streams, typed
  book/trade reads, merged event reads, and inclusive sequence-range filtering.
- Replay CLI discovery-first flow for listing venues/symbols/streams and
  replaying merged events with optional sequence bounds.
- New built-in signal modules:
  `VolumeImbalanceSignal`, `CumulativeDeltaSignal`, `AbsorptionSignal`,
  `ExhaustionSignal`, `SweepDetectionSignal`, and `CompositeSignal`.
- Config compatibility reporting through
  `load_engine_config_report_from_path(...)`.
- Binding smoke checks in CI for Python and Java against the real native ABI.
- Expanded package README inventories for each public Rust crate API surface.

### Changed
- Rust crate publishing metadata now includes `repository`, `homepage`, and
  crate-level `documentation` links for better crates.io/docs.rs presentation.
- Workspace and binding author metadata updated to:
  Gregorian Rayne `<gregorianrayne09@gmail.com>`.
- Python PyPI metadata now includes project URLs, classifiers, and keywords.
- Java Maven metadata now includes organization, issue tracker URL,
  inception year, and developer id/email.
- Binding release versions are now managed centrally in
  `bindings/versions.toml` and synchronized via
  `tools/release/sync_binding_versions.py` across Python, Java, and Rust/C
  package version surfaces.
- Python and Java binding package descriptions were upgraded with richer
  packaging-facing docs (badges, API map, operations notes, and direct doc links)
  to improve PyPI and Maven discoverability.
- Python and Java binding distribution docs were expanded to include full
  public API reference tables, signature-level usage guidance, ingest/polling
  workflows, and troubleshooting sections for PyPI and Maven users.
- Binding and Rust/C package versions are now aligned for this release cycle
  at `0.2.0`.
- Added root `LICENSE` file (MIT) to satisfy package distribution and
  registry compliance requirements.
- Live adapters were hardened with reconnect/backoff supervision, subscription
  replay after reconnect, richer protocol health metadata, and stronger
  timeout/degraded-path handling.
- Runtime config loading now prefers typed TOML/JSON parsing with compatibility
  fallback for older flat config shapes.
- Runtime and FFI internals were modularized without changing the public Rust
  API or exported C symbols.

### Fixed
- Restored missing `#[no_mangle]` export attributes during the FFI
  modularization pass before release finalization.

### Upgrade Notes
- This release is additive and non-breaking for existing `0.1.x` integrations.
- Existing Rust, C, Python, and Java APIs continue to work; no required
  rename/removal migration is needed.
- Package versions are aligned at `0.2.0` for Rust, C, Python, and Java.
- If you use direct C or native bindings, update the native library and header
  together so the new snapshot symbols and stream constants stay in sync.
- If you use Python or Java, upgrade the binding package and the native
  `libof_ffi_c` library together.
- If you use config files, your existing flat config files still load, but new
  deployments should prefer the typed nested `adapter` and
  `adapter.credentials` shape.
- If you want to adopt the new functionality, start with:
  `book_snapshot`, `derived_analytics_snapshot`,
  `session_candle_snapshot`, `interval_candle_snapshot`,
  persistence readback APIs, and richer signal modules.

## [0.1.1] - 2026-03-16
### Fixed
- Python binding: `Engine.subscribe(..., callback=None)` now works correctly by
  passing a typed null callback pointer to the C ABI, instead of raising a
  `ctypes.ArgumentError`.

## [0.1.0] - 2026-03-09
### Added
- Initial public release of Rust crates (`of_core`, `of_signals`, `of_persist`,
  `of_adapters`, `of_runtime`, `of_ffi_c`), Java binding
  (`io.github.gregorian-09:orderflow-java-binding`), and Python binding
  (`orderflow-gregorian09`).
