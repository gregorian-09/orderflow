# API Reference (Rust, C, Python, Java)

This page is the semantic map of the public API. The lists below are exact
lookup aids, but they are preceded by explanations of what each surface owns,
how calls change state, what success means, and what the caller must do next.
The dedicated crate chapters then provide field-level and method-level detail.

Read an API item in this order: identify the layer that owns the invariant,
understand the lifecycle around the call, inspect the exact signature and
values, then read errors, ownership, concurrency, latency, and persistence
implications. A function name alone is never the complete contract.

## Compatibility Layers

- **Rust crates** are the implementation and extension surface.
- **C ABI** (`crates/of_ffi_c/include/orderflow.h`) is the stable cross-language boundary.
- **Python** wraps C ABI with `ctypes`.
- **Java** wraps C ABI with JNA.

The C ABI also has a machine-readable inventory at
`bindings/api_manifest.toml`. `tools/check_api_manifest.py` validates that the
manifest and `orderflow.h` expose the same functions and return types, while
`tools/check_ffi_exports.sh` uses the manifest as the expected native symbol
list. `tools/generate_binding_signatures.py` emits exact Python ctypes and Java
JNA declarations in manifest order from the validated header. The mechanical
native declarations are generated; Python and Java user-facing lifecycle,
ownership, error, buffer, and naming APIs remain manually designed.

## Detailed Handbook Chapters

Rust crate chapters:

- [`of_core` reference](./05a-of-core-reference.md)
- [`of_adapters` reference](./05b-of-adapters-reference.md)
- [`of_signals` reference](./05c-of-signals-reference.md)
- [`of_persist` reference](./05d-of-persist-reference.md)
- [`of_runtime` reference](./05e-of-runtime-reference.md)
- [`of_ffi_c` reference](./05f-of-ffi-c-reference.md)
- [`of_execution_core` reference](./05g-of-execution-core-reference.md)
- [`of_execution` reference](./05h-of-execution-reference.md)
- [`of_execution_adapters` reference](./05i-of-execution-adapters-reference.md)
- [`of_fix` reference](./05j-of-fix-reference.md)
- [`of_execution_algos` reference](./05k-of-execution-algos-reference.md)
- [`of_analytics` reference](./05l-of-analytics-reference.md)
- [`of_persist_parquet` reference](./05m-of-persist-parquet-reference.md)

Binding-specific docs:

- [Python binding handbook](../bindings/python.md)
- [Java binding handbook](../bindings/java.md)
- [C ABI header](https://github.com/gregorian-09/orderflow/blob/main/crates/of_ffi_c/include/orderflow.h)

Execution and OMS workflow docs:

- [OMS architecture](./09-oms-architecture.md)
- [OMS cookbook](./10-oms-cookbook.md)
- [Low-latency design](./11-low-latency-design.md)
- [Provider adapter authoring](./12-provider-adapter-authoring.md)
- [OMS recovery and operations](./13-recovery-and-operations.md)

---

## Rust API

Rust is the semantic source of truth. Its public methods are grouped by the
state they operate on: normalized market state, provider sessions, signal
interpretation, durable history, runtime orchestration, or execution state.
The caller should not skip layers merely because a lower-level type is visible.

### `of_core`

`of_core` defines the vocabulary shared by the market-data plane. A
`TradePrint` says that a match occurred; a `BookUpdate` changes a materialized
view of resting liquidity; `AnalyticsAccumulator` folds trades into
deterministic session state. These types contain no provider socket, file
writer, strategy policy, or order submission behavior.

The normal flow is to construct provider-neutral events, apply them in accepted
sequence order, inspect snapshots, and reset explicitly at a session boundary.
All price and quantity integers require instrument metadata for presentation.
Quality flags must be checked before interpreting a snapshot as actionable.

Public types:

- `SymbolId { venue, symbol }` — Canonical market symbol identifier used across venues.
- `Side` (`Bid`, `Ask`) — Trade or book side.
- `BookAction` (`Upsert`, `Delete`) — Book mutation kind.
- `BookUpdate` — Level-2 order book update.
- `TradePrint` — Last-trade print/tick.
- `AnalyticsSnapshot` — Aggregated analytics for a symbol/session.
- `DerivedAnalyticsSnapshot` — Additive derived analytics computed from the current session accumulator state.
- `IntervalCandleSnapshot` — Rolling interval candle-style summary derived from recent session trades.
- `SignalState` (`Neutral`, `LongBias`, `ShortBias`, `Blocked`) — Output state emitted by signal modules.
- `SignalSnapshot` — Snapshot of a signal module evaluation.
- `DataQualityFlags` — Bitset wrapper for feed-quality flags.
- `AnalyticsAccumulator` — In-memory accumulator that updates analytics state from normalized trades.

Public `DataQualityFlags` constants:

- `NONE` — No quality issues detected.
- `STALE_FEED` — Feed is stale beyond policy threshold.
- `SEQUENCE_GAP` — A sequence number gap was detected.
- `CLOCK_SKEW` — Clock skew detected between source and consumer.
- `DEPTH_TRUNCATED` — Book depth was truncated.
- `OUT_OF_ORDER` — Event arrived out-of-order.
- `ADAPTER_DEGRADED` — Adapter/external feed is degraded or reconnecting.

Public methods:

- `DataQualityFlags::bits() -> u32` — Returns raw bit representation.
- `DataQualityFlags::from_bits_truncate(u32) -> DataQualityFlags` — Builds flags from raw bits, preserving unknown bits.
- `DataQualityFlags::intersects(DataQualityFlags) -> bool` — Returns true when any flag in `other` is set in `self`.
- `AnalyticsAccumulator::on_trade(&TradePrint)` — Applies a trade print to analytics and recomputes profile levels.
- `AnalyticsAccumulator::reset_session_delta()` — Resets session delta and directional volume, keeps cumulative profile.
- `AnalyticsAccumulator::reset_session()` — Resets all session analytics and volume-profile state.
- `AnalyticsAccumulator::snapshot() -> AnalyticsSnapshot` — Returns a copy of current analytics state.
- `AnalyticsAccumulator::derived_snapshot() -> DerivedAnalyticsSnapshot` — Returns additive derived analytics for the current session accumulator state.
- `AnalyticsAccumulator::session_candle_snapshot() -> SessionCandleSnapshot` — Returns candle-style session summary for the current analytics session.
- `AnalyticsAccumulator::interval_candle_snapshot(window_ns: u64) -> IntervalCandleSnapshot` — Returns candle-style summary for trades observed inside a rolling interval.

### `of_adapters`

`of_adapters` is the translation boundary between provider messages and
`of_core`. `connect` establishes a provider session, `subscribe` declares the
desired stream, `poll` performs a bounded unit of work, and `health` reports
whether the resulting stream is usable. The adapter must validate correlation,
sequence, timestamps, and payload meaning before emitting `RawEvent`.

An adapter error is not automatically a process failure. The runtime may mark
the affected stream degraded and reconnect, but it must not silently claim
continuity or retain subscriptions that the provider did not restore.

Public types:

- `SubscribeReq { symbol, depth_levels }` — Subscription request forwarded to adapters.
- `AdapterHealth { connected, degraded, last_error, protocol_info }` — Adapter connection and quality health snapshot.
- `RawEvent` (`Book(BookUpdate)`, `Trade(TradePrint)`) — Raw adapter event stream.
- `AdapterError` — Adapter-level error variants.
- `AdapterResult<T>` — Result type alias used by adapter interfaces.
- `MarketDataAdapter` trait — Common market-data adapter interface used by runtime.
- `ProviderKind` (`Mock`, `Rithmic`, `Cqg`, `Binance`) — Provider selection used by adapter factory configuration.
- `AdapterQualityLevel` — Adapter maturity level advertised by the discovery registry.
- `AdapterDescriptor` — Static capability description for one market-data adapter.
- `AdapterConformanceRequirement` — Adapter conformance requirement checked for a target quality level.
- `AdapterConformanceFailure` — One failed adapter conformance requirement.
- `AdapterConformanceReport` — Adapter conformance report for one descriptor and target quality level.
- `AdapterConfig` — Generic adapter factory configuration.
- `CredentialsRef` — Credential environment-variable references for adapter auth bootstrap.
- `MockAdapter` — Deterministic in-memory adapter for tests, demos, and replay harnesses.

Public functions/methods:

- `adapter_descriptors() -> &'static [AdapterDescriptor]` — Returns static descriptors for all known adapter providers.
- `compiled_adapter_descriptors() -> Vec<AdapterDescriptor>` — Returns descriptors for providers compiled into the current binary.
- `describe_adapter(ProviderKind) -> AdapterDescriptor` — Returns the descriptor for `provider`.
- `adapter_feature_enabled(ProviderKind) -> bool` — Returns true when the current binary can construct `provider`.
- `adapter_quality_requirements(AdapterQualityLevel) -> &'static [AdapterConformanceRequirement]` — Returns the conformance requirements for a target adapter quality level.
- `evaluate_adapter_conformance(&AdapterDescriptor, AdapterQualityLevel) -> AdapterConformanceReport` — Evaluates whether a descriptor satisfies a target adapter quality level.
- `adapter_conformance_report(ProviderKind, AdapterQualityLevel) -> AdapterConformanceReport` — Evaluates a known provider against a target adapter quality level.
- `create_adapter(&AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>>` — Creates a provider adapter from configuration.
- `MockAdapter::push_event(RawEvent)` — Pushes an event into mock queue, drained by `poll`.

`MarketDataAdapter` trait methods:

- `connect()` — Opens or establishes the provider/session connection.
- `subscribe(SubscribeReq)` — Registers a symbol or stream and records the requested subscription state.
- `unsubscribe(SymbolId)` — Removes a symbol or stream subscription and releases its active state.
- `poll(&mut Vec<RawEvent>)` — Processes one bounded unit of provider work and emits normalized events.
- `health() -> AdapterHealth` — Returns the component's connection, freshness, and degradation state.

### `of_signals`

`of_signals` interprets analytics; it does not create execution truth. Signal
modules consume snapshots and context, pass through warm-up and quality gates,
then return direction, confidence, lifecycle state, and (where supported) an
explanation. `Unknown` or blocked is a meaningful result: callers must not
coerce it to `Flat` and continue trading.

Registry, validation, calibration, ensemble, checkpoint, and shadow APIs exist
to make promotion observable. A signal should be replayed and validated before
its output is translated by the host into an order intent.

Public types:

- `SignalGateDecision` (`Pass`, `Block`) — Result of running quality-gate checks.
- `SignalModule` trait — Trait implemented by signal modules consumed by the runtime.
- `ExplainableSignalModule` trait — Optional extension trait for modules that expose structured explanations.
- `SignalExplanation` — Structured diagnostic explanation for a signal snapshot.
- `SignalReasonCode` — Stable machine-readable reason for a signal output.
- `SignalExplanationMode` — Controls whether explanations should be emitted for every evaluation or only transitions.
- `SignalConfig` — Borrowed signal configuration for registry validation and construction.
- `SignalConfigParameter` — One named parameter supplied in a signal configuration.
- `SignalConfigValue` — Configuration value used when constructing a signal from a registry.
- `SignalRegistration` — One signal registration containing descriptor metadata and optional factory.
- `SignalRegistry` — Registry for discovering, validating, and constructing signal modules.
- `SignalRegistryError` — Error returned by signal registry validation or construction.
- `SignalMarkoutDirection` — Directional markout label used by signal validation.
- `SignalReplayEvent` — Borrowed analytics event used when validation needs timestamp checks.
- `SignalValidationConfig` — Configuration for replay-based signal validation.
- `SignalValidationWarning` — Warning emitted by the replay validation harness.
- `SignalValidationSample` — One scored replay sample.
- `SignalValidationReport` — Summary report produced by replay-based signal validation.
- `SignalValidationHarness` — Replay validation harness for signal modules.
- `SignalCalibrationConfig` — Configuration for confidence calibration reports.
- `SignalConfidenceCalibrator` — Maps raw signal confidence into calibrated confidence.
- `IdentitySignalCalibrator` — Identity confidence calibrator.
- `SignalCalibrationPoint` — One point in a piecewise-linear confidence calibration curve.
- `SignalCalibrationCurve` — Piecewise-linear confidence calibration curve.
- `SignalOutcomeRecord` — One realized signal outcome used for calibration and drift reports.
- `SignalCalibrationBin` — One confidence bin in a calibration report.
- `SignalRegimeSummary` — Per-regime confidence and accuracy summary.
- `SignalCalibrationReport` — Calibration report for realized signal outcomes.
- `SignalCalibrationBinDrift` — One confidence-bin drift comparison.
- `SignalCalibrationDriftReport` — Drift report comparing current calibration with a baseline.
- `SignalOutcomeTracker` — Incremental outcome tracker for calibration and drift reporting.
- `SignalEnsembleDecisionRule` — Rule used to select an ensemble signal state from child votes.
- `SignalEnsembleConflictPolicy` — Policy used when long and short ensemble evidence conflicts.
- `SignalEnsembleVetoPolicy` — Policy used when an ensemble child marks itself as a veto.
- `SignalEnsemblePolicy` — Configuration for evaluating an ensemble of signal snapshots.
- `SignalEnsembleVote` — One child vote supplied to the ensemble evaluator.
- `SignalEnsembleConflict` — Conflict observed while evaluating an ensemble.
- `SignalEnsembleMetrics` — Aggregate metrics produced by ensemble evaluation.
- `SignalEnsembleDecision` — Result of evaluating a signal ensemble.
- `SignalEnsembleExplanation` — Aggregated explanation for an ensemble decision.
- `SignalCheckpoint` — Versioned signal checkpoint metadata and payload.
- `SignalCheckpointRestorePolicy` — Restore-time validation policy for a signal checkpoint.
- `SignalCheckpointValidationIssue` — One restore validation issue for a signal checkpoint.
- `SignalCheckpointValidationReport` — Restore validation report for a signal checkpoint.
- `SignalCheckpointRestoreError` — Error returned by checkpoint-aware signal restore operations.
- `CheckpointableSignal` — Optional extension trait for signals that support checkpoint restore.
- `SignalRunMode` — Runtime mode for a signal in production or validation hosts.
- `SignalRunModeDecision` — Evaluation and publication behavior implied by a run mode.
- `SignalShadowSample` — One production-versus-candidate shadow comparison sample.
- `SignalShadowComparisonConfig` — Configuration for shadow comparison reports.
- `SignalShadowComparisonReport` — Report comparing production and shadow/candidate signal output.
- `SignalShadowRecorder` — Incremental recorder for shadow-mode signal comparisons.
- `FeatureQualityFlags` — Quality flags attached to one feature value.
- `FeatureValueKind` — Semantic kind for one feature value.
- `FeatureMissingPolicy` — Missing-value policy for a feature descriptor.
- `FeatureDescriptor` — One feature in a signal/model feature schema.
- `FeatureSchema` — Stable feature schema used by feature-vector and model-backed signals.
- `FeatureVectorView` — Borrowed feature vector plus schema and per-feature quality flags.
- `FeatureVectorValidationIssue` — One feature-vector validation issue.
- `FeatureVectorValidationReport` — Validation report for a feature vector.
- `SignalModelKind` — Supported model artifact/runtime family.
- `SignalModelOutputKind` — Model output semantics for model-backed signals.
- `SignalModelMetadata` — Metadata describing a model-backed signal artifact.
- `SignalModelInputBinding` — Input binding between a model and a feature schema.
- `SignalModelOutput` — Output returned by model-backed signal inference.
- `ModelBackedSignal` — Optional extension trait for model-backed signal implementations.
- `DeltaMomentumSignal` — Reference implementation: simple delta momentum threshold signal.
- `VolumeImbalanceSignal` — Volume imbalance signal based on buy/sell session totals.
- `CumulativeDeltaSignal` — Cumulative delta signal tuned for session-scale directional bias.
- `AbsorptionSignal` — Absorption signal that looks for strong directional flow failing to dislodge price from POC.
- `ExhaustionSignal` — Exhaustion signal that looks for strong directional flow stalling back near POC.
- `SweepDetectionSignal` — Sweep detection signal that looks for value-area breaks accompanied by directional flow.
- `CompositeSignal` — Composite signal that aggregates child modules into one stable directional output.

Public methods:

- `DeltaMomentumSignal::new(threshold: i64) -> Self` — Creates a new signal with absolute delta threshold.
- `VolumeImbalanceSignal::new(threshold: i64) -> Self` — Creates a new volume-imbalance signal with absolute imbalance threshold.
- `CumulativeDeltaSignal::new(threshold: i64) -> Self` — Creates a new cumulative-delta signal with absolute threshold.
- `AbsorptionSignal::new(threshold: i64, price_band: i64) -> Self` — Creates a new absorption signal using a delta threshold and price band around POC.
- `ExhaustionSignal::new(threshold: i64) -> Self` — Creates a new exhaustion signal using a delta threshold.
- `SweepDetectionSignal::new(threshold: i64, breakout_ticks: i64) -> Self` — Creates a new sweep signal with delta threshold and breakout distance.
- `CompositeSignal::new(modules: Vec<Box<dyn SignalModule>>) -> Self` — Creates a composite signal from child modules.

`SignalModule` trait methods:

- `on_analytics(&AnalyticsSnapshot)` — Feeds an analytics snapshot into the signal's decision state.
- `snapshot() -> SignalSnapshot` — Returns a read-only view of the component's current state.
- `quality_gate(DataQualityFlags) -> SignalGateDecision` — Evaluates whether the supplied data-quality flags permit signal output.

`ExplainableSignalModule` trait methods:

- `explanation() -> SignalExplanation` — Returns a compact top-level signal explanation for the ensemble.

Registry functions and methods:

- `built_in_signal_registrations() -> &'static [SignalRegistration]` — Returns registrations for all built-in signal modules.
- `built_in_signal_descriptors_json() -> String` — Exports built-in signal descriptors as compact JSON.
- `SignalRegistry::with_built_ins() -> Self` — Creates a registry containing the built-in signal modules.
- `SignalRegistry::validate_config(&SignalConfig) -> SignalRegistryResult<()>` — Validates a signal configuration without constructing the module.
- `SignalRegistry::create_signal(&SignalConfig) -> SignalRegistryResult<Box<dyn SignalModule>>` — Constructs a signal module from configuration.
- `SignalRegistry::descriptors_json() -> String` — Exports registered descriptors as compact JSON for bindings and dashboards.

Validation functions and methods:

- `validate_signal_replay(&mut impl SignalModule, &[AnalyticsSnapshot], SignalValidationConfig) -> SignalValidationReport` — Validates a signal by replaying ordered analytics snapshots.
- `validate_signal_replay_events(&mut impl SignalModule, &[SignalReplayEvent], SignalValidationConfig) -> SignalValidationReport` — Validates a signal by replaying ordered analytics events with optional timestamps.
- `SignalValidationHarness::validate_signal(...) -> SignalValidationReport` — Validates a signal over ordered analytics snapshots.
- `SignalValidationReport::json_summary() -> String` — Exports a compact JSON summary for Python and notebook workflows.

Calibration functions and methods:

- `SignalCalibrationConfig::new(bin_width_bps) -> SignalCalibrationConfig` — Creates a context from analytics and data-quality state.
- `SignalCalibrationPoint::new(raw_confidence_bps, calibrated_confidence_bps) -> SignalCalibrationPoint` — Creates a context from analytics and data-quality state.
- `SignalCalibrationCurve::new(points) -> SignalCalibrationCurve` — Creates a curve from calibration points sorted by raw confidence.
- `SignalConfidenceCalibrator::calibrate_confidence_bps(raw_confidence_bps) -> u16` — Maps a raw confidence value through the configured calibration curve.
- `SignalOutcomeRecord::new(module_id, state, confidence_bps, predicted_direction, markout_direction, correct) -> SignalOutcomeRecord` — Creates a signal outcome record.
- `SignalOutcomeRecord::from_validation_sample(&SignalValidationSample) -> SignalOutcomeRecord` — Creates an outcome record from a validation sample.
- `SignalCalibrationReport::from_records(&[SignalOutcomeRecord], SignalCalibrationConfig) -> SignalCalibrationReport` — Builds a calibration report from outcome records.
- `SignalCalibrationReport::from_validation_report(&SignalValidationReport, SignalCalibrationConfig) -> SignalCalibrationReport` — Builds a calibration report from retained validation samples.
- `SignalCalibrationReport::accuracy_bps() -> Option<u16>` — Returns scored accuracy in basis points.
- `SignalCalibrationReport::json_summary() -> String` — Exports a compact JSON summary.
- `SignalCalibrationDriftReport::compare(&baseline, &current, threshold_bps) -> SignalCalibrationDriftReport` — Builds a drift report from baseline and current calibration reports.
- `SignalOutcomeTracker::record(SignalOutcomeRecord)` — Records one realized signal outcome.
- `SignalOutcomeTracker::extend_validation_report(&SignalValidationReport)` — Records retained samples from a validation report.
- `SignalOutcomeTracker::calibration_report() -> SignalCalibrationReport` — Builds a calibration report from tracked outcomes.
- `SignalOutcomeTracker::drift_report(&SignalCalibrationReport) -> SignalCalibrationDriftReport` — Compares tracked outcomes against a baseline report.

Ensemble functions and methods:

- `SignalEnsemblePolicy::majority() -> SignalEnsemblePolicy` — Creates a majority-vote ensemble policy.
- `SignalEnsemblePolicy::quorum(min_votes) -> SignalEnsemblePolicy` — Creates a quorum ensemble policy.
- `SignalEnsemblePolicy::weighted(min_score_bps) -> SignalEnsemblePolicy` — Creates a weighted-score ensemble policy.
- `SignalEnsemblePolicy::with_conflict_policy(policy) -> SignalEnsemblePolicy` — Returns this policy with a different conflict policy.
- `SignalEnsemblePolicy::with_veto_policy(policy) -> SignalEnsemblePolicy` — Returns this policy with a different veto policy.
- `SignalEnsemblePolicy::with_min_confidence_bps(min_confidence_bps) -> SignalEnsemblePolicy` — Returns config with a minimum confidence threshold.
- `SignalEnsembleVote::new(module_id, state, confidence_bps) -> SignalEnsembleVote` — Creates a context from analytics and data-quality state.
- `SignalEnsembleVote::from_snapshot(&SignalSnapshot) -> SignalEnsembleVote` — Creates an ensemble vote from a signal snapshot.
- `SignalEnsembleVote::from_explanation(&SignalExplanation) -> SignalEnsembleVote` — Creates an ensemble vote from a signal explanation.
- `SignalEnsembleVote::with_weight_bps(weight_bps) -> SignalEnsembleVote` — Returns this vote with a different child weight.
- `SignalEnsembleVote::with_veto(veto) -> SignalEnsembleVote` — Returns this vote with explicit veto behavior.
- `evaluate_signal_ensemble(module_id, votes, policy) -> SignalEnsembleDecision` — Evaluates child votes into one ensemble signal decision.
- `evaluate_signal_ensemble_explanations(module_id, child_explanations, weights_bps, policy) -> SignalEnsembleExplanation` — Evaluates child explanations and returns an aggregated ensemble explanation.
- `SignalEnsembleExplanation::explanation() -> SignalExplanation` — Returns a compact top-level signal explanation for the ensemble.

Checkpoint and shadow-mode functions and methods:

- `SignalCheckpoint::new(module_id, signal_version, state) -> SignalCheckpoint` — Creates checkpoint metadata for a signal state.
- `SignalCheckpoint::from_snapshot(&SignalSnapshot, signal_version) -> SignalCheckpoint` — Creates checkpoint metadata from a signal snapshot.
- `SignalCheckpointRestorePolicy::new() -> SignalCheckpointRestorePolicy` — Creates a context from analytics and data-quality state.
- `validate_signal_checkpoint_restore(&SignalCheckpoint, &SignalCheckpointRestorePolicy) -> SignalCheckpointValidationReport` — Validates checkpoint metadata against a restore policy.
- `CheckpointableSignal::checkpoint() -> SignalCheckpoint` — Captures the component state needed for deterministic restart.
- `CheckpointableSignal::restore_checkpoint(&SignalCheckpoint) -> Result<(), SignalCheckpointRestoreError>` — Validates and restores a previously captured component state.
- `SignalRunModeDecision::from_mode(SignalRunMode) -> SignalRunModeDecision` — Creates a behavior decision from a run mode.
- `SignalShadowSample::compare(event_index, production, candidate) -> SignalShadowSample` — Creates a shadow comparison sample.
- `SignalShadowSample::with_markout(markout_direction) -> SignalShadowSample` — Returns this sample scored against a future markout label.
- `SignalShadowComparisonReport::from_samples(samples, config) -> SignalShadowComparisonReport` — Builds a comparison report from shadow samples.
- `SignalShadowComparisonReport::agreement_bps() -> Option<u16>` — Returns state agreement rate in basis points.
- `SignalShadowComparisonReport::production_accuracy_bps() -> Option<u16>` — Returns production directional accuracy in basis points.
- `SignalShadowComparisonReport::candidate_accuracy_bps() -> Option<u16>` — Returns candidate directional accuracy in basis points.
- `SignalShadowComparisonReport::json_summary() -> String` — Exports a compact JSON summary.
- `SignalShadowRecorder::record(SignalShadowSample)` — Records one shadow comparison sample.
- `SignalShadowRecorder::report() -> SignalShadowComparisonReport` — Builds a comparison report.

Feature vector and model-support functions and methods:

- `FeatureQualityFlags::bits() -> u32` — Returns the raw bit representation.
- `FeatureQualityFlags::from_bits_truncate(bits) -> FeatureQualityFlags` — Builds an input mask from raw bits, preserving unknown future bits.
- `FeatureDescriptor::new(id, value_kind) -> FeatureDescriptor` — Creates a feature descriptor with conservative defaults.
- `FeatureSchema::new(id, version) -> FeatureSchema` — Creates an empty feature schema.
- `FeatureSchema::with_feature(feature) -> FeatureSchema` — Returns this schema with an appended feature descriptor.
- `FeatureSchema::feature_index(id) -> Option<usize>` — Returns the index for a feature id.
- `FeatureSchema::feature(id) -> Option<&FeatureDescriptor>` — Returns a feature descriptor by id.
- `FeatureVectorView::new(schema, values, quality, timestamp_ns) -> FeatureVectorView` — Creates a context from analytics and data-quality state.
- `FeatureVectorView::value(id) -> Option<f64>` — Returns a feature value by id.
- `FeatureVectorView::quality(id) -> Option<FeatureQualityFlags>` — Returns feature quality flags by id.
- `FeatureVectorView::validate(now_ns) -> FeatureVectorValidationReport` — Validates this feature vector against its schema.
- `validate_feature_vector(&FeatureVectorView, now_ns) -> FeatureVectorValidationReport` — Validates a feature vector view against schema metadata.
- `SignalModelMetadata::new(model_id, model_version, feature_schema_id, feature_schema_version) -> SignalModelMetadata` — Creates model metadata.
- `SignalModelInputBinding::new(input_name, feature_ids) -> SignalModelInputBinding` — Creates a model input binding.
- `SignalModelInputBinding::is_compatible_with(&FeatureSchema) -> bool` — Returns `true` when all bound feature ids exist in the schema.
- `SignalModelOutput::new(state, confidence_bps) -> SignalModelOutput` — Creates a model output from state and confidence.
- `ModelBackedSignal::model_metadata() -> &SignalModelMetadata` — Returns the model identity and compatibility metadata.
- `ModelBackedSignal::feature_schema() -> &FeatureSchema` — Returns the feature schema required by the model.
- `ModelBackedSignal::infer_features(&FeatureVectorView) -> SignalModelOutput` — Evaluates the model against a validated feature vector.

### `of_persist`

`of_persist` owns durable market-data history. Its WAL admission, write, sync,
seal, checkpoint, replay, and retention operations are different durability
boundaries. A successful admission does not prove that bytes survived power
loss; a valid replay does not repair a provider gap. Failure policies must be
selected explicitly because dropping a record changes recoverability.

Public types:

- `PersistError` — Persistence-layer errors.
- `PersistResult<T>` — Result type alias used by persistence APIs.
- `RetentionPolicy { max_total_bytes, max_age_secs }` — Retention policy used by [`RollingStore`].
- `RollingStore` — JSONL rolling store for book/trade stream persistence.
- `StoredBookEvent` — Parsed book event read back from persisted JSONL storage.
- `StoredTradeEvent` — Parsed trade event read back from persisted JSONL storage.
- `StoredEvent` — Merged persisted event used for replay-oriented symbol reads.

Public methods:

- `RollingStore::new(root) -> PersistResult<RollingStore>` — Creates a store rooted at `root`, creating directories as needed.
- `RollingStore::with_retention(Option<RetentionPolicy>) -> RollingStore` — Sets optional retention policy used after each append.
- `RollingStore::append_book(&BookUpdate) -> PersistResult<()>` — Appends a single book event as JSON line.
- `RollingStore::append_trade(&TradePrint) -> PersistResult<()>` — Appends a single trade event as JSON line.
- `RollingStore::list_venues() -> PersistResult<Vec<String>>` — Lists venue directories currently present under the store root.
- `RollingStore::list_symbols(venue) -> PersistResult<Vec<String>>` — Lists symbol directories for a given venue currently present under the store root.
- `RollingStore::list_streams(venue, symbol) -> PersistResult<Vec<String>>` — Lists stream files currently present for a given venue and symbol.
- `RollingStore::read_books(venue, symbol) -> PersistResult<Vec<StoredBookEvent>>` — Reads persisted book events for the given venue and symbol.
- `RollingStore::read_books_in_range(venue, symbol, from_sequence, to_sequence) -> PersistResult<Vec<StoredBookEvent>>` — Reads persisted book events filtered by an inclusive sequence range.
- `RollingStore::read_trades(venue, symbol) -> PersistResult<Vec<StoredTradeEvent>>` — Reads persisted trade events for the given venue and symbol.
- `RollingStore::read_trades_in_range(venue, symbol, from_sequence, to_sequence) -> PersistResult<Vec<StoredTradeEvent>>` — Reads persisted trade events filtered by an inclusive sequence range.
- `RollingStore::read_events(venue, symbol) -> PersistResult<Vec<StoredEvent>>` — Reads and merges persisted book and trade events for the given venue and symbol.
- `RollingStore::read_events_in_range(venue, symbol, from_sequence, to_sequence) -> PersistResult<Vec<StoredEvent>>` — Reads merged persisted events filtered by an inclusive sequence range.

### `of_runtime`

`of_runtime` composes adapters, core state, signals, persistence, and health
under a host-controlled synchronous lifecycle. `start`/`stop` govern activity,
`poll_once` advances a bounded processing step, ingest methods accept events
from a host-owned feed, and snapshot methods return read models. A running
engine can still be unready when its feed is stale, degraded, incomplete, or
awaiting persistence/reconciliation policy.

Public types:

- `EngineConfig` — Runtime engine configuration.
- `RuntimeError` — Runtime errors surfaced by engine lifecycle and processing.
- `ConfigCompatibilityMode` — Indicates how a runtime config file was accepted.
- `ConfigLoadReport` — Detailed result for config-file loading.
- `ExternalFeedPolicy` — Policy controlling quality constraints for externally-ingested feeds.
- `Engine<A, S>` — Runtime engine over a market-data adapter and signal module.
- `DefaultEngine` type alias — Default engine type used by C ABI and high-level bindings.

Public constructor/build/config functions:

- `Engine::new(cfg, adapter, signal_module) -> Engine<A, S>` — Creates an engine with explicit adapter and signal module.
- `build_default_engine(cfg: EngineConfig) -> Result<DefaultEngine, RuntimeError>` — Builds the default runtime engine using configured provider and signal module.
- `load_engine_config_from_path(path: &str) -> Result<EngineConfig, RuntimeError>` — Loads engine config from `.
  - preferred input shape: typed TOML/JSON with nested `adapter` / `adapter.credentials`
  - compatibility fallback: legacy flat config files remain accepted
- `load_engine_config_report_from_path(path: &str) -> Result<ConfigLoadReport, RuntimeError>` — Loads engine config and reports whether legacy compatibility fallback was required.
  - reports `format`
  - reports `compatibility_mode`
  - surfaces a warning when legacy fallback was required
- `validate_startup_config(cfg: &EngineConfig) -> Result<(), RuntimeError>` — Validates startup configuration and environment prerequisites.

Public runtime methods:

- `with_persistence(Option<RollingStore>)` — Injects optional persistence backend.
- `start()` — Connects adapter and marks runtime as started.
- `stop()` — Stops runtime state and emits health transition.
- `subscribe(SymbolId, depth_levels)` — Subscribes to symbol stream through adapter.
- `unsubscribe(SymbolId)` — Unsubscribes symbol from adapter stream.
- `reset_symbol_session(SymbolId)` — Resets per-symbol analytics/session state.
- `configure_external_feed(ExternalFeedPolicy)` — Configures external-feed quality supervisor policy.
- `set_external_reconnecting(bool)` — Marks external feed reconnecting/degraded state.
- `external_health_tick()` — Re-evaluates health for external-feed stale policy without ingesting data.
- `ingest_trade(TradePrint, DataQualityFlags)` — Ingests a single external trade event.
- `ingest_book(BookUpdate, DataQualityFlags)` — Ingests a single external book event.
- `poll_once(DataQualityFlags)` — Polls adapter once and processes all returned events.
- `analytics_snapshot(&SymbolId)` — Returns analytics snapshot for symbol if available.
- `derived_analytics_snapshot(&SymbolId)` — Returns additive derived analytics snapshot for symbol if available.
- `session_candle_snapshot(&SymbolId)` — Returns session candle snapshot for symbol if available.
- `interval_candle_snapshot(&SymbolId, window_ns: u64)` — Returns rolling interval candle snapshot for symbol if available.
- `signal_snapshot(&SymbolId)` — Returns latest signal snapshot for symbol if available.
- `signal_explanation_json(&SymbolId) -> Option<String>` — Returns latest signal explanation JSON for symbol if available.
- `signal_metrics_json() -> String` — Returns signal metrics as compact JSON payload.
- `adapter_descriptor() -> AdapterDescriptor` — Returns static descriptor for the configured adapter provider.
- `adapter_status() -> RuntimeAdapterStatus` — Returns latest active-adapter status.
- `adapter_inventory_json() -> String` — Returns all known adapter descriptors as compact JSON.
- `active_adapter_status_json() -> String` — Returns active adapter status as compact JSON.
- `signal_descriptor_inventory_json() -> String` — Returns built-in signal descriptors as compact JSON.
- `metrics_json() -> String` — Returns runtime metrics as compact JSON payload.
- `health_seq() -> u64` — Returns monotonic health sequence number.
- `health_json() -> String` — Returns health snapshot as compact JSON payload.
- `last_events() -> &[RawEvent]` — Returns events processed in the last poll/ingest cycle.
- `current_quality_flags_bits() -> u32` — Returns currently-active quality flags as raw bits.

### `of_execution_core`

`of_execution_core` is the small canonical vocabulary for order requests,
reports, identifiers, state transitions, risk primitives, and durable records.
It does not connect to a venue. Its purpose is to ensure every adapter and OMS
implementation agrees on identity, quantity, side, timestamp, and transition
meaning before transport-specific behavior is added.

Public identifier types:

- `FixedAscii<N>` — Fixed-size ASCII field used for low-allocation identifiers.
- `ClientOrderId` — Client-assigned order identifier.
- `VenueOrderId` — Venue-assigned order identifier.
- `ExecutionId` — Venue execution/fill identifier.
- `AccountId` — Trading account identifier.
- `RouteId` — Execution route identifier.
- `StrategyId` — Strategy identifier used for attribution.
- `VenueId` — Venue identifier used by execution routing.
- `InstrumentId` — Instrument identifier in venue/native format.
- `ExecutionText` — Bounded diagnostic text.

Public execution model types:

- `ExecutionSymbol` — Execution symbol in venue-native format.
- `OrderQty` — Integer-normalized order quantity.
- `OrderPrice` — Integer-normalized order price.
- `OrderSide` — Buy/sell order side.
- `OrderType` — Supported canonical order types.
- `TimeInForce` — Time-in-force policy.
- `OrderStatus` — FIX-style canonical order status.
- `ExecutionType` — Canonical execution report purpose.
- `OrderRequest` — New order request.
- `CancelRequest` — Cancel request.
- `AmendRequest` — Amend/cancel-replace request.
- `ExecutionEvent` — Canonical execution event.
- `OrderState` — Current order state.
- `OrderStateMachine` — Deterministic order state machine.

Public risk types:

- `RiskRejectReason` — Structured risk rejection reason.
- `RiskDecision` — Risk decision.
- `RiskLimits` — Static risk limits for one route/account scope.
- `RiskContext` — Runtime risk context supplied by the execution engine.
- `RiskCheck` trait — Pre-trade risk-check contract.
- `BasicRiskGate` — Deterministic pre-trade risk gate.

Public execution WAL frame primitives:

- `EXECUTION_WAL_MAGIC` — Magic value written at the start of every execution WAL frame.
- `EXECUTION_WAL_VERSION` — Binary execution WAL frame version.
- `EXECUTION_WAL_HEADER_LEN` — Encoded execution WAL header length in bytes.
- `EXECUTION_WAL_MAX_PAYLOAD_LEN` — Maximum payload bytes accepted by the execution WAL frame helpers.
- `ExecutionWalError` — Error returned by execution WAL frame helpers.
- `WalChecksumField` — Execution WAL record checksum category.
- `WalSequence` — Monotonic execution WAL sequence number.
- `WalSegmentId` — Execution WAL segment identifier.
- `WalRecordKind` — Execution WAL record kind.
- `WalSyncPolicy` — Execution WAL durability policy.
- `WalRecordHeader` — Fixed-size execution WAL record header.
- `WalRecordView` — Borrowed execution WAL record.
- `WalReplayCursor` — Sequential borrowed replay cursor for execution WAL bytes.
- `WalIntegrityReport` — Integrity summary for encoded execution WAL bytes.
- `execution_wal_checksum` — Returns the deterministic non-cryptographic checksum used by WAL frames.

For field-level semantics and transition rules, see
[`of_execution_core` reference](./05g-of-execution-core-reference.md).

### `of_execution`

`of_execution` is the OMS control plane. A submit call validates a host-owned
intent, checks idempotency and risk, journals an accepted command, routes it,
and later folds authoritative reports into canonical state. A transport
acknowledgement is not a fill. A timeout after submission is uncertain state
and must be reconciled using the original identity rather than retried as a
new order.

For multiple symbols, accounts, and routes, identity isolates each lifecycle;
coordinated baskets still contain independent leg truth. Checkpoint and WAL
recovery must finish before the engine reports readiness for new risk.

Public adapter and engine types:

- `ExecutionError` — Execution-layer error.
- `ExecutionResult<T>` — Execution result alias.
- `ExecutionEventBuffer` — Caller-owned event buffer used by execution adapters.
- `LatencyClass` — Adapter latency classification.
- `ExecutionCapabilities` — Execution adapter capabilities.
- `ExecutionHealth` — Execution adapter health snapshot.
- `ExecutionAdapter` trait — Common execution adapter interface.
- `RouteConfig` — Execution route configuration.
- `RouteKey` — Stable lookup key for a configured execution route.
- `AllowAllRiskGate` — Pass-through risk hook for engines that rely on route-scoped limits.
- `ExecutionEngine` — Execution engine for one adapter and one route set.
- `SimExecutionAdapter` — Deterministic simulated execution adapter.

Public journal types:

- `JournalCommandKind` — Journal command kind.
- `JournalRecord` — Execution journal record.
- `ExecutionJournal` trait — Execution journal hook.
- `InMemoryJournal` — In-memory execution journal for tests and embedded hosts.
- `WalJournalConfig` — Configuration for [`WalExecutionJournal`].
- `WalReplayResult` — Replay summary returned by WAL replay helpers.
- `WalJournalMetrics` — Low-latency execution WAL metrics snapshot.
- `WalExecutionJournal` — Binary append-only execution WAL journal.
- `WalSegmentConfig` — Configuration for [`SegmentedWalExecutionJournal`].
- `WalSegmentMetadata` — Metadata for one execution WAL segment file.
- `WalSegmentManifest` — Manifest inventory for a segmented execution WAL.
- `WalSegmentIntegrityReport` — Integrity summary for a segmented execution WAL.
- `SegmentedWalExecutionJournal` — Segmented binary execution WAL journal.
- `CheckpointPosition` — Snapshot of one position included in an execution checkpoint.
- `ExecutionCheckpoint` — Versioned OMS checkpoint payload.
- `CheckpointPolicy` — Checkpoint creation policy vocabulary.
- `CheckpointConfig` — File-backed checkpoint store configuration.
- `CheckpointManifest` — Metadata for one checkpoint file.
- `ExecutionCheckpointStore` trait — Execution checkpoint store contract.
- `FileExecutionCheckpointStore` — Atomic file-backed execution checkpoint store.
- `RecoveryCorruptionPolicy` — Recovery behavior when WAL replay encounters unusable data.
- `RecoveryVenuePolicy` — Venue reconciliation requirement selected for a recovery run.
- `RecoveryPlan` — Deterministic OMS recovery plan.
- `RecoveredOmsState` — Recovered OMS state reconstructed from a checkpoint and WAL replay.
- `RecoveryResult` — Summary of one deterministic recovery run.
- `RecoveryReadinessConfig` — Policy for evaluating whether recovered OMS state may resume submissions.
- `RecoveryReadinessBlocker` — Fail-closed reason emitted by recovery-readiness evaluation.
- `RecoveryReadinessDecision` — Aggregate recovery-readiness decision for restart workflows.
- `evaluate_recovery_readiness` — Evaluates WAL, checkpoint, recovery, and reconciliation evidence before live submissions resume.
- `recover_oms_state_from_records` — Recovers OMS state from already decoded journal records.
- `recover_oms_state_from_segmented_wal` — Recovers OMS state from a segmented WAL and an optional checkpoint.
- `recover_oms_state_from_segmented_wal_root` — Recovers OMS state from an existing segmented WAL root without creating an append handle or modifying files.
- `recover_latest_checkpoint_from_segmented_wal` — Loads the latest checkpoint and recovers state from a segmented WAL.
- `recover_latest_checkpoint_from_segmented_wal_roots` — Loads an optional latest checkpoint and recovers an existing segmented WAL root without creating or modifying either root.

Public concurrent execution types:

- `ConcurrentExecutionConfig` — Configuration for the concurrent execution worker.
- `ExecutionCommandKind` — Command kind sent to a concurrent execution worker.
- `ExecutionCommand` — Command payload sent to a concurrent execution worker.
- `ExecutionCommandReport` — Result report emitted by a concurrent execution worker.
- `ConcurrentExecutionError` — Error returned by the concurrent execution wrapper.
- `ExecutionCommandSender` — Cloneable concurrent command handle for an execution worker.
- `ConcurrentExecutionEngine` — Concurrent owner for a synchronous execution engine.

Public independent drop-copy types:

- `DropCopySourceId` — Stable identifier for an independent drop-copy source or session.
- `DropCopyReportId` — Provider-assigned identifier used to deduplicate drop-copy reports.
- `DropCopyReport` — Canonical report emitted by a drop-copy adapter.
- `DropCopyReportBuffer` — Caller-owned bounded buffer used by drop-copy adapters.
- `DropCopySourceState` — Independent drop-copy transport/session state.
- `DropCopySourceHealth` — Health snapshot for one independent drop-copy source.
- `DropCopyAdapter` trait — Provider-neutral contract for an independent drop-copy session.
- `InMemoryDropCopyAdapter` — Deterministic bounded drop-copy source for tests, replay, and bridges.
- `DropCopyLateReportPolicy` — Policy for reports that regress source time or cumulative fill quantity.
- `DropCopyDisposition` — Recommended handling after duplicate and late-report checks.
- `DropCopyCorrelation` — Correlation result between drop-copy evidence and local OMS state.
- `DropCopyIssueFlags` — Allocation-free bitset describing drop-copy reconciliation issues.
- `DropCopyObservation` — Result of observing one canonical drop-copy report.
- `DropCopyMetricsSnapshot` — Allocation-free drop-copy ingestion and reconciliation metrics.
- `DropCopyReconciler` — Bounded low-allocation drop-copy deduplicator and state reconciler.

Public scoped kill-switch types:

- `KillSwitchId` — Stable identifier for one kill-switch activation lifecycle.
- `KillSwitchSessionId` — Adapter or protocol-session identity used by session-scoped switches.
- `KillSwitchActorId` — Human or system identity recorded with kill-switch operations.
- `KillSwitchSourceKind` — Source category responsible for a kill-switch operation.
- `KillSwitchSource` — Actor responsible for activating, updating, or clearing a switch.
- `KillSwitchScope` — Scope selected by a kill switch.
- `KillSwitchMode` — Operational behavior selected by an active switch.
- `KillSwitchReasonCode` — Structured reason for a kill-switch operation.
- `KillSwitchStateCertainty` — Registry certainty used to enforce fail-closed startup and recovery.
- `KillSwitchActivation` — Command that activates a scoped kill switch.
- `KillSwitchCancelResult` — Command that records one attempted cancellation for an active switch.
- `KillSwitchClear` — Command that clears one active switch.
- `KillSwitchEventKind` — Kind of auditable kill-switch event.
- `KillSwitchCancelOutcome` — Aggregate cancellation state for one switch activation.
- `KillSwitchEvent` — Immutable audit event emitted for kill-switch state transitions.
- `KillSwitchOrderContext` — Order metadata needed for scope matching and reduce-only evaluation.
- `KillSwitchAffectedOrder` — One affected open order emitted into a caller-owned cancellation buffer.
- `KillSwitchAffectedOrderBuffer` — Caller-owned bounded output for affected open orders.
- `ActiveKillSwitch` — Read-only active switch entry retained by [`KillSwitchRegistry`].
- `KillSwitchDecisionReason` — Reason selected by a kill-switch order decision.
- `KillSwitchDecision` — Allocation-free decision for one prospective order.
- `KillSwitchRegistry` — Bounded registry for scoped kill-switch state and cancellation progress.
- `KillSwitchError` — Errors returned by bounded kill-switch state management.

Public production-risk types:

- `ProductionRiskPolicyId` — Stable identifier for one risk policy.
- `RiskInstrumentGroupId` — Host-defined instrument or product group identifier.
- `ProductionRiskScope` — Scope matched by a production risk policy.
- `RiskTradingWindow` — UTC nanosecond-of-day trading window.
- `ProductionRiskLimits` — Limits and safety conditions attached to one scoped policy.
- `ProductionRiskPolicy` — One ordered scoped production risk policy.
- `ProductionRiskCommandKind` — Command classification used by production risk evaluation.
- `ProductionRiskCommand` — Canonical command view consumed by [`ProductionRiskEngine`].
- `ProductionRiskContext` — Caller-supplied state used for production risk checks.
- `ProductionRiskReason` — Detailed production risk decision reason.
- `ProductionRiskJournalStatus` — Decision-journal state returned to the caller.
- `ProductionRiskDecision` — Explainable allocation-free production risk decision.
- `ProductionRiskError` — Configuration and capacity errors for production risk state.
- `ProductionRiskJournalError` — Bounded decision-journal error.
- `ProductionRiskDecisionJournal` trait — Journal contract for explainable production risk decisions.
- `InMemoryProductionRiskJournal` — Bounded in-memory decision journal for tests and low-latency handoff.
- `ProductionRiskEngine` — Ordered bounded engine for scoped production risk controls.

Public order-intent and parent/child types:

- `OrderIntentId` — Stable strategy intent identifier.
- `OmsParentOrderId` — Stable OMS parent-order identifier.
- `OmsChildOrderId` — Stable OMS child-order identifier.
- `OrderIntentState` — Parent intent lifecycle state.
- `OmsChildOrderState` — OMS-owned child lifecycle state.
- `ExecutionInstruction` — Routing and venue-order instructions for one child.
- `OrderIntent` — Immutable strategy intent and parent-level constraints.
- `OmsChildOrder` — OMS child order with replacement lineage and aggregate fill state.
- `OrderIntentSnapshot` — Read-only parent aggregate snapshot.
- `OrderIntentError` — Parent/child lifecycle validation and capacity error.
- `OmsChildCancelTarget` — Child cancellation target selected by parent cancel-tree processing.
- `OmsChildCancelBuffer` — Caller-owned bounded cancel-tree output.
- `OrderIntentRecoverySnapshot` — Recovery payload for one complete parent/child tree.
- `OrderIntentLifecycle` — Bounded single-owner OMS parent/child lifecycle.

Public OMS helper types:

- `CommandId` — Monotonic command identifier assigned before a command enters an OMS queue.
- `RequestId` — Request identifier used to correlate strategy intent, command queue entry, and downstream execution reports.
- `CommandIdGenerator` — Lock-free monotonic command id generator.
- `CommandCorrelation` — Correlation envelope for an execution command.
- `IdempotencyScopeId` — Caller-defined tenant, strategy gateway, or session scope for request IDs.
- `AdapterCommandId` — Provider-specific identifier attached to an outbound command.
- `IdempotencyKey` — Scope plus caller request ID forming one idempotency key.
- `IdempotentExecutionCommand` — Mutating execution command protected by an idempotency key.
- `IdempotencyState` — Durable lifecycle state for one idempotent command.
- `IdempotencyCompletion` — Definitive outcome supplied after local or venue processing.
- `IdempotencyRecord` — Stored command correlation and retry state.
- `IdempotencyDecision` — Result of reserving a command key.
- `IdempotencyMetrics` — Bounded idempotency registry metrics.
- `IdempotencyError` — Idempotency validation or lifecycle error.
- `IdempotencyCheckpoint` — Checksummed control-plane snapshot for recovery-safe command retries.
- `IdempotencyRegistry` — Bounded, allocation-free-after-construction command idempotency registry.
- `ExecutionReportSourceId` — Source/session identity used to scope execution-report identities.
- `ExecutionReportKey` — Canonical execution-report identity scoped to one adapter/session source.
- `ExecutionReportDisposition` — Fresh/duplicate result from the report window.
- `ExecutionReportDedupMetrics` — Execution-report duplicate-window metrics.
- `ExecutionReportDedupError` — Execution-report duplicate-window error.
- `ExecutionReportDedupCheckpoint` — Checksummed oldest-to-newest duplicate-window checkpoint.
- `ExecutionReportDeduplicator` — Fixed-capacity FIFO duplicate window for normal, replay, and drop-copy events.
- `ExecutionEventFanout` — Bounded execution-event fanout for multiple consumers.
- `ExecutionEventSubscriber` — Event subscriber for execution fanout.
- `ExecutionAdapterState` — Venue adapter/session lifecycle state.
- `ExecutionLifecycle` — Mutable lifecycle tracker for adapters and supervisors.
- `ExecutionLifecycleSnapshot` — Execution lifecycle snapshot.
- `FileExecutionJournal` — Durable append-only execution journal.
- `ReconciliationAction` — Open-order reconciliation action.
- `ReconciliationItem` — One reconciliation difference.
- `ReconciliationReport` — Open-order reconciliation report.
- `ReconciliationIssueKind` — Fine-grained reconciliation issue classification.
- `ReconciliationDetail` — One detailed reconciliation finding.
- `VenueReconciliationReport` — Detailed venue reconciliation report.
- `ReconciliationPolicyAction` — Host action selected for a reconciliation issue.
- `ReconciliationPolicy` — Policy for mapping reconciliation issues to host actions.
- `ReconciliationPolicyItem` — Policy decision for one reconciliation finding.
- `ReconciliationPolicyDecision` — Aggregate policy decision for a reconciliation report.
- `OmsReconciliationSource` — Evidence source participating in one OMS reconciliation cycle.
- `OmsReconciliationSourceSet` — Compact required/observed source set.
- `OmsEvidenceStatus` — Integrity/availability state supplied for one evidence source.
- `OmsEvidenceWatermark` — Source watermark and integrity evidence.
- `OmsReconciliationIssue` — Fine-grained generalized reconciliation classification.
- `OmsReconciliationAction` — Host action selected for a generalized finding.
- `OmsReconciliationPolicy` — Complete issue-to-action policy.
- `OmsReconciliationConfig` — Reconciliation cycle bounds and required evidence policy.
- `OmsReconciliationEntity` — Entity represented by a reconciliation finding.
- `OmsReconciliationFinding` — One machine-readable generalized reconciliation finding.
- `OmsReconciliationBuffer` — Caller-owned bounded generalized reconciliation output.
- `OmsReconciliationSummary` — Aggregate result for one completed reconciliation cycle.
- `OmsReconciliationError` — Generalized reconciliation lifecycle/capacity error.
- `OmsReconciliationCoordinator` — Single-owner generalized reconciliation cycle coordinator.
- `DisconnectPolicy` — Route safety behavior during disconnects and kill switches.
- `RouteSafetyPolicy` — Safety policy for one route scope.
- `AdvancedRiskLimits` — Advanced additive risk limits.
- `AdvancedRiskGate` — Advanced risk gate with basic limits plus message-rate checks.
- `Position` — Position for one account/strategy/symbol scope.
- `PositionKey` — Position key.
- `PositionLedger` — Fill and position ledger.
- `LedgerCurrency` — Settlement or reporting currency identifier, such as `USD` or `USDT`.
- `LedgerAdjustmentId` — Stable identifier for a manual, corporate-action, or correction mutation.
- `ProductionPositionKey` — Position ownership and valuation key.
- `LedgerExecutionIdentity` — Provider/session-scoped execution identity used for fill deduplication.
- `LedgerScopedAdjustmentId` — Position-scoped adjustment identity used for adjustment deduplication.
- `LedgerFxRate` — Positive rational conversion from local money units to base money units.
- `LedgerFill` — Validated canonical fill consumed by [`ProductionPositionLedger`].
- `LedgerFillAttribution` — Fill attribution retained in the bounded recent-mutation window.
- `LedgerAdjustmentKind` — Auditable non-fill mutation kind.
- `LedgerAdjustment` — Explicit position/PnL adjustment supplied by an authorized host path.
- `LedgerMark` — Position mark used for unrealized PnL.
- `ProductionPosition` — Authoritative local position/PnL state in one settlement currency.
- `ProductionPositionLedgerConfig` — Bounded ledger sizing and duplicate-retention configuration.
- `LedgerApplyStatus` — Result classification for one ledger mutation.
- `LedgerApplyResult` — Explainable result of applying one fill or adjustment.
- `PositionLedgerError` — Position-ledger validation, capacity, ordering, and persistence error.
- `ProductionPositionLedger` — Bounded authoritative average-cost position and PnL ledger.
- `LedgerCheckpointIdentity` — Persisted recent mutation identity used by checkpoint recovery.
- `LedgerCheckpointPosition` — Position row stored in a ledger checkpoint.
- `PositionLedgerCheckpoint` — Versioned, checksummed position-ledger checkpoint.
- `PositionLedgerCheckpointConfig` — File-backed checkpoint-store configuration.
- `PositionLedgerCheckpointManifest` — Installed checkpoint file metadata.
- `PositionLedgerCheckpointStore` trait — Replaceable checkpoint-store contract.
- `FilePositionLedgerCheckpointStore` — Atomic file-backed production position-ledger checkpoint store.
- `ExternalPositionSnapshot` — Authoritative broker, clearing, venue, or drop-copy position snapshot.
- `PositionReconciliationTolerance` — Absolute comparison tolerances for external position reconciliation.
- `PositionReconciliationIssueFlags` — Compact reconciliation issue bitset.
- `PositionReconciliationItem` — One local-to-external reconciliation comparison.
- `PositionReconciliationBuffer` — Caller-owned bounded reconciliation output.
- `PositionReconciliationReport` — Aggregate position reconciliation result.
- `VenueOrderCapabilities` — Venue-specific order type and TIF capabilities.
- `NormalizedOrderType` — Normalized venue order encoding.
- `ExecutionTelemetry` — Additive execution telemetry.
- `ShardKey` — Route sharding key.
- `ShardRouter` — Deterministic sharding helper.
- `OrderThrottle` — Token-bucket style order throttler.
- `ReplayDecision` — Replay decision used by the OMS simulation harness.
- `ReplayResult` — Replay result for deterministic OMS simulation.
- `ProviderAdapterContext` — Provider adapter context supplied to convenience adapter builders.
- `ExecutionAdapterFactory` — Factory trait for provider-specific execution adapters.
- `ProviderAdapterSdk` — Convenience SDK helpers for provider adapters.

Public helper functions:

- `simulated_engine(route) -> ExecutionEngine<SimExecutionAdapter, BasicRiskGate, InMemoryJournal>` — Creates a one-route simulated execution engine.
- `simulated_engine_with_routes(routes) -> ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal>` — Creates a simulated execution engine for multiple configured routes.
- `reconcile_open_orders(local, venue) -> ReconciliationReport` — Reconciles local open-order state against venue state.
- `reconcile_open_orders_detailed(local, venue) -> VenueReconciliationReport` — Compares local open-order state against venue open-order state with fine-grained discrepancy classification.
- `evaluate_reconciliation_policy(report, policy) -> ReconciliationPolicyDecision` — Evaluates a detailed reconciliation report against a host policy.
- `normalize_order_type(order_type, tif, capabilities) -> NormalizedOrderType` — Validates and normalizes order type/TIF against venue capabilities.
- `replay_simulated_oms(routes, decisions) -> ExecutionResult<ReplayResult>` — Runs a deterministic simulated OMS replay.
- `reconcile_production_positions(ledger, external, tolerance, out) -> Result<PositionReconciliationReport, PositionLedgerError>` — Reconciles local ledger state against external authoritative snapshots.

For lifecycle, routing, concurrency, and OMS helper details, see
[`of_execution` reference](./05h-of-execution-reference.md).

### `of_execution_algos`

`of_execution_algos` produces bounded child-order plans from parent progress
and supplied market context. It never bypasses the OMS's risk, idempotency,
route capability, or kill-switch checks. Planning is deterministic when parent
state, configuration, observations, and elapsed-time inputs are the same.
Rounding, remainder allocation, min/max clips, zero-output meaning, rejects,
partial fills, and uncertain submissions are part of each algorithm's contract.

Public execution-algorithm identifier types:

- `ParentOrderId` — Algorithm parent-order identifier.
- `ChildOrderId` — Algorithm child-order identifier.
- `AlgoIntentId` — Strategy intent identifier.
- `AlgoInstanceId` — Running algorithm instance identifier.

Public parent/child and decision types:

- `ParentOrderStatus` — Execution-algorithm status for a parent order.
- `ChildOrderStatus` — Execution-algorithm status for a child order.
- `AlgoError` — Execution-algorithm error.
- `ParentOrder` — Parent order controlled by an execution algorithm.
- `ChildOrderPlan` — Planned child order generated by an execution algorithm.
- `AlgoProgress` — Aggregate parent execution progress.
- `AlgoAction` — Execution-algorithm action.
- `AlgoDecision` — Fixed-capacity algorithm decision.
- `AlgoRiskOutcome` — Algorithm risk-policy outcome.
- `AlgoRiskViolationKind` — Algorithm risk violation category.
- `AlgoRiskViolation` — One algorithm risk violation retained in a report.
- `AlgoRiskLimits` — Algorithm risk limits.
- `AlgoRiskContext` — Host-supplied risk context for one algorithm decision.
- `AlgoRiskReport` — Fixed-capacity risk report.
- `AlgoRiskPolicy` — Additive algorithm risk policy for validating child plans before OMS submit.
- `AlgoRecoveryAction` — Recovery action recommended for an algorithm instance.
- `AlgoCheckpoint` — Deterministic checkpoint for one algorithm parent instance.
- `AlgoRecoveryPolicy` — Algorithm recovery policy.
- `AlgoRecoveryPlan` — Deterministic recovery plan derived from a checkpoint and policy.
- `AlgoSimOutcome` — Deterministic child-order simulation outcome.
- `AlgoSimMarket` — Deterministic market/fill model for one simulation pass.
- `AlgoSimStep` — One simulated child-order result.
- `AlgoSimReport` — Fixed-capacity algorithm simulation report.
- `AlgoSimulator` — Deterministic simulator for generated child plans.
- `AlgoTcaBenchmark` — Optional TCA benchmark prices for an algorithm parent.
- `AlgoMetricsSnapshot` — Snapshot of algorithm execution metrics and TCA fields.
- `AlgoMetricsAccumulator` — Allocation-free accumulator for algo execution metrics and TCA.
- `AlgoKind` — Execution algorithm category for typed configuration.
- `AlgoParentConfig` — Typed parent-order configuration for algorithm construction.
- `AlgoConfig` — Typed top-level algorithm configuration.
- `TwapSlicePlanner` — Deterministic TWAP slice planner.
- `AlgoReplayEvent` — Replay event consumed by an algorithm harness.
- `AlgoReplayInput` — Sequenced replay input.
- `AlgoReplayIdScheme` — Deterministic child/client id generation prefixes for replay.
- `AlgoReplayStep` — Replay step emitted for one input event.
- `AlgoReplaySummary` — Summary returned by deterministic TWAP replay.
- `replay_twap_into` — Replays TWAP planning over explicit deterministic inputs.
- `PovSlicePlanner` — Deterministic percentage-of-volume child slice planner.
- `VwapVolumeCurve` — Borrowed cumulative volume curve for VWAP planning.
- `VwapSlicePlanner` — Deterministic VWAP child slice planner.
- `IcebergSlicePlanner` — Deterministic synthetic iceberg replenishment planner.
- `PassivePegMode` — Passive peg reference used by [`PassiveQueuePlanner`].
- `PassiveQueueAction` — Passive queue action selected by [`PassiveQueuePlanner`].
- `PassiveQueueContext` — Market context for passive queue planning.
- `PassiveQueueConfig` — Configuration for passive queue planning.
- `PassiveQueueEstimate` — Passive queue planning estimate.
- `PassiveQueueDecision` — Passive queue decision with an optional child order plan.
- `PassiveQueuePlanner` — Deterministic passive peg and queue-position planner.
- `SorRouteStatus` — Route availability state used by [`SorPlanner`].
- `SorRouteCapability` — Order-type capability advertised by a SOR route.
- `SorRouteMetrics` — Route quality metrics used for smart-order-router scoring.
- `SorRouteCandidate` — Routable liquidity candidate consumed by [`SorPlanner`].
- `SorScoreWeights` — Integer score weights for [`SorPlanner`].
- `SorConfig` — Smart-order-router configuration.
- `SorChildAllocation` — Scored child allocation produced by [`SorPlanner`].
- `SorDecision` — Fixed-capacity SOR decision.
- `SorPlanner` — Deterministic smart-order-router planner.
- `LiquiditySeekingAction` — Liquidity-seeking action selected for one route.
- `LiquiditySeekingCandidate` — Liquidity-seeking candidate derived from a routable venue.
- `LiquiditySeekingConfig` — Configuration for liquidity-seeking route selection.
- `LiquiditySeekingAllocation` — One liquidity-seeking allocation.
- `LiquiditySeekingDecision` — Fixed-capacity liquidity-seeking decision.
- `LiquiditySeekingPlanner` — Deterministic liquidity-seeking planner.
- `SweepConfig` — Aggressive sweep configuration.
- `SweepAllocation` — One aggressive sweep allocation.
- `SweepDecision` — Fixed-capacity aggressive sweep decision.
- `SweepPlanner` — Deterministic aggressive sweep planner.
- `BasketLegRole` — Basket or spread leg side in the portfolio objective.
- `BasketLeg` — One parent order participating in a basket or spread execution.
- `BasketChildAllocation` — Planned child allocation for one basket leg.
- `BasketDecision` — Fixed-capacity basket decision.
- `BasketPlanner` — Deterministic synchronized basket/spread planner.
- `SpreadConfig` — Two-leg spread execution configuration.
- `SpreadQuote` — Current executable two-leg spread prices.
- `SpreadEstimate` — Spread estimate used by [`SpreadPlanner`].
- `SpreadDecision` — Two-leg spread decision.
- `SpreadPlanner` — Deterministic two-leg pairs/spread planner.
- `MarketMakerContext` — Market-making context supplied by the host quote model.
- `MarketMakerConfig` — Market-making quote configuration.
- `MarketMakerQuoteEstimate` — Market-making quote estimate.
- `MarketMakerQuoteDecision` — Market-making quote decision.
- `MarketMakerPlanner` — Deterministic market-making quote planner.
- `ImplementationShortfallContext` — Market context for implementation-shortfall planning.
- `ImplementationShortfallConfig` — Configuration for implementation-shortfall planning.
- `ImplementationShortfallEstimate` — Implementation-shortfall planning estimate.
- `ImplementationShortfallPlanner` — Deterministic implementation-shortfall planner.

Public constants:

- `DEFAULT_ALGO_DECISION_CAPACITY` — Default maximum number of actions retained in an [`AlgoDecision`].
- `DEFAULT_ALGO_RISK_VIOLATION_CAPACITY` — Default maximum number of retained violations in an [`AlgoRiskReport`].
- `ALGO_CHECKPOINT_SCHEMA_VERSION` — Current algorithm checkpoint schema version.

For parent/child and TWAP planning details, see
[`of_execution_algos` reference](./05k-of-execution-algos-reference.md).

### `of_analytics`

`of_analytics` contains reusable advanced measurements that are intentionally
separate from the smallest core accumulator. Trackers have explicit warm-up,
bounded history, units, reset, and insufficient-sample behavior. A default or
zero snapshot may mean “not enough evidence,” not “the measured phenomenon is
zero.” Feed it validated events or feature views and preserve its schema when
serializing derived data.

Public market-quality and liquidity types:

- `AnalyticsError` — Advanced analytics error.
- `QuoteContext` — Best bid/ask context used by market-quality analytics.
- `TradeContext` — Trade context aligned to a quote.
- `MarketQualitySnapshot` — Market-quality and transaction-cost snapshot for one trade/quote pair.
- `MarketQualityTracker` — Market-quality tracker retaining the latest quote.
- `ExecutionBenchmark` — Execution-quality benchmark context.
- `ExecutionQualitySnapshot` — Execution-quality/TCA snapshot.
- `ExecutionQualityAnalyzer` — Execution-quality/TCA analyzer.
- `LiquidityDepthSnapshot` — Liquidity/depth snapshot over borrowed book levels.
- `LiquidityDepthAnalyzer` — Borrowed depth analyzer.
- `LiquidityFlowEvent` — Liquidity-flow event over a book observation interval.
- `LiquidityFlowConfig` — Liquidity-flow tracker configuration.
- `LiquidityFlowSnapshot` — Liquidity-flow snapshot over accumulated book events.
- `LiquidityFlowTracker` — Allocation-free liquidity-flow tracker.
- `ImpactSample` — Market-impact sample over a measurement interval.
- `ImpactSnapshot` — Cumulative market-impact snapshot.
- `ImpactTracker` — Allocation-free cumulative impact tracker.
- `ImpactCalibration` — Calibrated market-impact parameters for pre-trade estimates.
- `ExpectedImpactInput` — Pre-trade impact estimate input.
- `ExpectedImpactSnapshot` — Pre-trade market-impact estimate.
- `ExpectedImpactEstimator` — Deterministic pre-trade market-impact estimator.
- `ChildOrderImpactContext` — Child-order impact attribution context.
- `ChildOrderImpactSnapshot` — Child-order impact attribution snapshot.
- `ChildOrderImpactAnalyzer` — Deterministic child-order impact attribution analyzer.
- `VpinSnapshot` — VPIN-style toxicity snapshot.
- `VpinTracker` — Fixed-capacity VPIN-style bucket tracker.
- `ToxicityConfig` — Toxicity/adverse-selection thresholds.
- `ToxicityInput` — Toxicity/adverse-selection observation.
- `ToxicitySnapshot` — Toxicity/adverse-selection risk snapshot.
- `ToxicityAnalyzer` — Deterministic toxicity/adverse-selection analyzer.
- `VolatilitySnapshot` — Rolling volatility/noise snapshot.
- `VolatilityTracker` — Fixed-window volatility tracker.
- `OhlcVolatilityInput` — OHLC volatility estimator input.
- `OhlcVolatilitySnapshot` — OHLC volatility estimator snapshot.
- `OhlcVolatilityEstimator` — Deterministic OHLC volatility estimator.
- `VolatilitySignatureSnapshot` — Volatility signature point over borrowed returns.
- `VolatilitySignatureEstimator` — Borrowed volatility signature estimator.
- `VolatilitySeasonalitySnapshot` — Intraday volatility seasonality bucket snapshot.
- `VolatilitySeasonalityTracker` — Fixed-bucket intraday volatility seasonality tracker.
- `RegimeKind` — Market regime classification.
- `RegimeInput` — Regime classifier input.
- `RegimeSnapshot` — Regime snapshot.
- `RegimeClassifier` — Threshold-based market regime classifier.
- `TrendRegimeKind` — Trend/range/chop regime label.
- `LiquidityRegimeKind` — Liquidity regime label.
- `SpreadRegimeKind` — Spread regime label.
- `SessionRegimeKind` — Session phase regime label.
- `CompositeRegimeConfig` — Composite regime classifier configuration.
- `CompositeRegimeInput` — Composite regime classifier input.
- `CompositeRegimeSnapshot` — Composite regime snapshot.
- `CompositeRegimeClassifier` — Deterministic composite regime classifier.
- `FeedQualityFlags` — Feed-quality degradation flags.
- `FeedQualityConfig` — Feed-quality tracker configuration.
- `FeedQualityEvent` — Market-data event context used by feed-quality analytics.
- `FeedQualitySnapshot` — Cumulative feed-quality snapshot.
- `FeedQualityTracker` — Allocation-free feed-quality tracker.
- `ReplayQualityConfig` — Replay-quality report thresholds.
- `ReplayQualityReport` — Replay-quality report derived from feed-quality counters.
- `ReplayQualityAnalyzer` — Deterministic replay-quality report analyzer.
- `FeatureId` — Stable feature identifier.
- `FeatureUnit` — Feature value unit.
- `FeatureQuality` — Per-feature extraction quality.
- `MissingValuePolicy` — Missing-feature fill policy.
- `FeatureDefinition` — Feature definition inside a stable schema.
- `FeatureSchema` — Fixed-capacity feature schema.
- `FeatureRegistry` — Fixed-capacity feature registry alias.
- `FeatureVector` — Completed fixed-capacity feature vector.
- `FeatureVectorWriter` — Reusable fixed-capacity feature-vector writer.
- `FeatureExtractor` — Feature extractor contract.
- `ResiliencySample` — Liquidity resiliency sample.
- `ResiliencyConfig` — Liquidity resiliency thresholds.
- `ResiliencySnapshot` — Liquidity resiliency snapshot.
- `ResiliencyTracker` — Threshold-based liquidity resiliency tracker.
- `QueueUpdateKind` — Queue update kind.
- `QueuePositionEstimate` — Passive order queue estimate.
- `QueueFillConfig` — Queue/fill probability configuration.
- `QueueFillUpdate` — Queue update at the local order price.
- `QueueFillSnapshot` — Passive fill probability snapshot.
- `QueueFillTracker` — Queue/fill probability tracker.
- `QueueDecisionConfig` — Queue decision thresholds.
- `QueueDecisionInput` — Queue decision economics and replacement context.
- `QueueDecisionSnapshot` — Queue decision snapshot.
- `QueueDecisionAnalyzer` — Deterministic queue decision analyzer.
- `PatternRiskInput` — Pattern-risk input over a bounded observation window.
- `PatternRiskLiquidity` — Pattern-risk liquidity summary.
- `PatternRiskConfig` — Pattern-risk classifier thresholds.
- `PatternRiskSnapshot` — Pattern-risk snapshot.
- `PatternRiskClassifier` — Deterministic pattern-risk classifier.
- `PatternDetailConfig` — Detailed pattern-risk configuration.
- `PatternDetailInput` — Detailed pattern-risk input over a bounded observation window.
- `PatternDetailSnapshot` — Detailed pattern-risk snapshot.
- `PatternDetailAnalyzer` — Deterministic detailed pattern-risk analyzer.
- `VenueRouteEventKind` — Venue route event kind.
- `VenueRouteEvent` — Venue route analytics event.
- `VenueRouteSnapshot` — Venue route analytics snapshot.
- `VenueRouteTracker` — Venue route analytics tracker.
- `VenueRouteQualityConfig` — Venue route quality thresholds.
- `VenueRouteQualityInput` — Venue route quality input.
- `VenueRouteQualitySnapshot` — Venue route quality snapshot.
- `VenueRouteQualityAnalyzer` — Deterministic venue route quality analyzer.
- `CrossAssetSample` — Cross-asset paired price sample.
- `CrossAssetConfig` — Cross-asset analytics configuration.
- `CrossAssetSnapshot` — Cross-asset analytics snapshot.
- `CrossAssetTracker` — Fixed-window cross-asset lead/lag tracker.
- `CrossAssetDiagnosticConfig` — Cross-asset diagnostic thresholds.
- `CrossAssetDiagnosticInput` — Cross-asset diagnostic input.
- `CrossAssetDiagnosticSnapshot` — Cross-asset diagnostic snapshot.
- `CrossAssetDiagnosticAnalyzer` — Deterministic cross-asset diagnostic analyzer.
- `OptionKind` — Option contract kind.
- `OptionFlowSample` — Option flow sample.
- `OptionFlowSnapshot` — Option flow snapshot.
- `OptionFlowTracker` — Cumulative option flow tracker.
- `FuturesBasisInput` — Futures basis input.
- `FuturesBasisSnapshot` — Futures basis snapshot.
- `FuturesBasisAnalyzer` — Futures basis analyzer.
- `DerivativesVolatilitySurface` — Caller-supplied derivatives volatility surface summary.
- `DerivativesDiagnosticConfig` — Derivatives diagnostic thresholds.
- `DerivativesDiagnosticInput` — Derivatives diagnostic input.
- `DerivativesDiagnosticSnapshot` — Derivatives diagnostic snapshot.
- `DerivativesDiagnosticAnalyzer` — Deterministic derivatives diagnostic analyzer.

For advanced analytics crate details, see
[`of_analytics` reference](./05l-of-analytics-reference.md).

### `of_fix`

`of_fix` separates FIX protocol correctness from venue business policy. Its
codec validates bounded frames and fields; its session layer handles sequence,
heartbeat, resend, reset, duplicate, and recovery rules; execution adapters
map accepted business messages into canonical OMS events. It does not own a
socket or infer that a syntactically valid message is an accepted order.

Public FIX codec types:

- `FixTag` — Numeric FIX tag identifier.
- `FixVersion` — Known FIX begin-string versions.
- `FixMsgType` — FIX `MsgType(35)` identifier.
- `FixFieldView` — Borrowed FIX tag-value field.
- `FixMessageView` — Borrowed view over a validated FIX message.
- `FixParseError` — FIX parse and validation errors.
- `FixEncodeError` — FIX encode errors.
- `FixProfileError` — FIX dictionary/profile validation errors.
- `FixRejectParseError` — FIX reject-message parse errors.
- `FixSessionRejectView` — Borrowed Session Reject `<3>` view.
- `FixBusinessMessageRejectView` — Borrowed BusinessMessageReject `<j>` view.
- `FixMessageRule` — Validation rule for one FIX message type.
- `FixDictionary` — Static FIX dictionary/profile used for message-level validation.
- `FixDecoder` — Stateless FIX decoder facade.
- `FixEncoder` — Reusable FIX encoder with an owned output buffer.
- `FixSessionState` — FIX session lifecycle state.
- `FixSessionEngineConfig` — Configuration for a deterministic FIX session engine.
- `FixSessionConfigError` — Invalid FIX session-engine configuration.
- `FixSessionEngine` — Single-owner deterministic FIX session state machine.
- `FixSessionAction` — Deterministic action produced by a FIX session-engine call.
- `FixSessionSendKind` — Administrative message kind emitted into the caller-owned output buffer.
- `FixSessionDisconnectReason` — Reason the session asks its host to close the transport.
- `FixSessionMetrics` — Allocation-free FIX session counters and timing snapshot.
- `FixSessionError` — FIX session protocol and state-machine errors.
- `FixSequenceTracker` — Deterministic inbound/outbound FIX sequence tracker.
- `FixSequenceAction` — Result of observing an inbound sequence number.
- `FixSequenceError` — FIX sequence tracking errors.
- `FixResendRange` — Resend range requested after an inbound sequence gap.
- `FixSessionId` — Borrowed FIX session identity.
- `FixSequenceSnapshot` — Borrowed persistable sequence-state snapshot.
- `FixOwnedSessionId` — Owned FIX session identity loaded from durable storage.
- `FixOwnedSequenceSnapshot` — Owned persistable sequence-state snapshot loaded from storage.
- `FixSequenceStoreConfig` — File-backed FIX sequence snapshot store configuration.
- `FixSequenceSnapshotManifest` — Metadata for an installed FIX sequence snapshot.
- `FixSequenceStoreError` — Error returned by FIX sequence snapshot persistence.
- `FixSequenceSnapshotStore` trait — FIX sequence snapshot persistence contract.
- `FileFixSequenceSnapshotStore` — Atomic file-backed FIX sequence snapshot store.
- `FixSentMessageKind` — Classification for outbound messages retained for resend handling.
- `FixResendStoreConfig` — Bounded resend-store configuration.
- `FixResendStore` — Bounded in-memory FIX resend store.
- `FixStoredMessage` — Retained outbound FIX frame.
- `FixResendRetention` — Result of recording a sent message into a resend store.
- `FixResendStoreMetrics` — Snapshot of resend-store counters and retained range.
- `FixResendStoreError` — Resend-store append errors.
- `FixDurableResendStoreConfig` — File-backed durable resend-message store configuration.
- `FixDurableResendAppend` — Metadata for one durable resend append.
- `FixDurableResendReplayReport` — Summary produced by replaying durable resend frames.
- `FixDurableResendStoreError` — Error returned by durable FIX resend-message persistence.
- `FixDurableResendMessageStore` trait — Durable resend-message persistence contract.
- `FileFixDurableResendStore` — Append-only file-backed durable FIX resend-message store.
- `FixResendAction` — One planned response for an outbound resend request.
- `FixResendPlanSummary` — Summary produced while planning a resend response.
- `FixTranscriptDirection` — Direction of a captured FIX transcript frame.
- `FixTranscriptMsgType` — Fixed-size transcript message-type copy.
- `FixTranscriptConfig` — Bounded transcript capture configuration.
- `FixTranscriptError` — Transcript capture errors.
- `FixTranscriptRecord` — Retained transcript frame metadata and optional raw bytes.
- `FixTranscriptRetention` — Result of recording a transcript frame.
- `FixTranscriptMetrics` — Snapshot of transcript capture counters.
- `FixTranscriptCapture` — Bounded in-memory FIX transcript capture.
- `FixSessionHeader` — Borrowed standard FIX session header fields used by admin builders.
- `FixOrderSide` — Common FIX `Side(54)` values for order-entry builders.
- `FixOrdType` — Common FIX `OrdType(40)` values for order-entry builders.
- `FixTimeInForce` — Common FIX `TimeInForce(59)` values for order-entry builders.
- `FixMassCancelRequestType` — Common FIX `MassCancelRequestType(530)` values.
- `FixMassStatusReqType` — Common FIX `MassStatusReqType(585)` values.
- `FixNewOrderSingle` — Borrowed NewOrderSingle `<D>` request fields.
- `FixOrderCancelRequest` — Borrowed OrderCancelRequest `<F>` request fields.
- `FixOrderCancelReplaceRequest` — Borrowed OrderCancelReplaceRequest `<G>` request fields.
- `FixOrderStatusRequest` — Borrowed OrderStatusRequest `<H>` request fields.
- `FixOrderMassCancelRequest` — Borrowed OrderMassCancelRequest `<q>` request fields.
- `FixOrderMassStatusRequest` — Borrowed OrderMassStatusRequest `<AF>` request fields.

Public FIX codec constants:

- `SOH` — FIX field delimiter byte.

Public FIX codec functions:

- `parse_message(raw, scratch) -> FixMessageView` — Parses and validates a FIX tag-value message into `scratch`.
- `parse_session_reject(message) -> FixSessionRejectView` — Parses a validated Session Reject `<3>` message into a borrowed view.
- `parse_business_message_reject(message) -> FixBusinessMessageRejectView` — Parses a validated BusinessMessageReject `<j>` message into a borrowed view.
- `encode_message(out, begin_string, msg_type, fields)` — Encodes a FIX tag-value message into `out`.
- `encode_poss_dup_replay(out, source, sending_time)` — Encodes a retained source message as a possible-duplicate resend.
- `checksum(bytes) -> u8` — Returns the stored snapshot checksum.
- `debug_render(raw) -> String` — Renders a debug string with `\|` separators instead of SOH.
- `encode_logon(out, version, header, heartbeat_interval_secs, reset_seq_num)` — Encodes a Logon `<A>` admin message.
- `encode_heartbeat(out, version, header, test_req_id)` — Encodes a Heartbeat `<0>` admin message.
- `encode_test_request(out, version, header, test_req_id)` — Encodes a TestRequest `<1>` admin message.
- `encode_resend_request(out, version, header, range)` — Encodes a ResendRequest `<2>` admin message.
- `encode_sequence_reset_gap_fill(out, version, header, new_seq_no)` — Encodes a SequenceReset `<4>` gap-fill admin message.
- `encode_logout(out, version, header, text)` — Encodes a Logout `<5>` admin message.
- `encode_new_order_single(out, version, header, request)` — Encodes a NewOrderSingle `<D>` application message.
- `encode_order_cancel_request(out, version, header, request)` — Encodes an OrderCancelRequest `<F>` application message.
- `encode_order_cancel_replace_request(out, version, header, request)` — Encodes an OrderCancelReplaceRequest `<G>` application message.
- `encode_order_status_request(out, version, header, request)` — Encodes an OrderStatusRequest `<H>` application message.
- `encode_order_mass_cancel_request(out, version, header, request)` — Encodes an OrderMassCancelRequest `<q>` application message.
- `encode_order_mass_status_request(out, version, header, request)` — Encodes an OrderMassStatusRequest `<AF>` application message.

For low-allocation FIX parsing, validation, and encoding details, see
[`of_fix` reference](./05j-of-fix-reference.md).

### `of_execution_adapters`

Execution adapters translate canonical OMS commands into venue/protocol
messages and map venue reports back without losing correlation or identity.
They own capability profiles, session integration, report mapping, and
certification behavior. They do not duplicate canonical order-state logic or
turn transport success into execution success.

Feature-gated public FIX execution types under `of_execution_adapters::fix`:

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
- `FixExecutionAdapter` — FIX execution adapter shell.
- `FixTransportPoll` — Result of one non-blocking transport receive attempt.
- `FixFrameTransport` — Frame-oriented transport contract for a live FIX session.
- `FixTimeSample` — One monotonic/wall-clock sample for FIX protocol work.
- `FixTimeSource` — Injected monotonic clock and FIX UTC timestamp formatter.
- `FixOutboundJournal` — Durable original-message journal used before transport transmission.
- `NoopFixOutboundJournal` — Outbound journal that performs no durable I/O.
- `InfallibleFixJournalError` — Uninhabited error used by [`NoopFixOutboundJournal`].
- `DurableFixOutboundJournal` — Adapts an `of_fix` durable resend store to the live-adapter journal hook.
- `FixWorkingOrderContext` — Original-order context needed by FIX cancel and replace messages.
- `FixExecutionProfile` — Venue/profile policy used by the transport adapter.
- `StandardFixProfileError` — Error from the standard FIX execution profile.
- `StandardFixExecutionProfile` — Standard FIX 4.
- `FixLiveAdapterConfigError` — Invalid live FIX adapter configuration.
- `FixLiveAdapterConfig` — Bounded configuration for a transport-injected FIX execution adapter.
- `FixLiveAdapterMetrics` — Allocation-free operational counters for the live FIX adapter.
- `FixTransportExecutionAdapter` — Synchronous, single-owner FIX execution adapter over injected infrastructure.
- `FixCertificationScenario` — Required deterministic FIX certification scenarios.
- `FixCertificationCapability` — FIX application capability that certification can require and exercise.
- `FixCertificationFailureKind` — Certification assertion failure category.
- `FixCertificationFailure` — One bounded certification failure.
- `FixExpectedField` — One expected FIX tag/value pair in a transcript frame.
- `FixFrameExpectation` — Exact metadata and field expectations for one transcript frame.
- `FixCertificationLatencyEvidence` — Aggregate latency evidence collected outside the adapter hot path.
- `FixCertificationAllocationEvidence` — Allocation evidence measured by a host-provided allocator/profiler.
- `FixCertificationConfig` — Bounded certification harness configuration.
- `FixCertificationScenarioResult` — Result for one certification scenario.
- `FixCertificationReport` — Immutable FIX certification conformance report.
- `FixCertificationHarness` — Stateful bounded certification report builder.
- `FixCertificationHarnessError` — Certification harness configuration/state error.
- `FixScriptedTransportConfig` — Bounded scripted counterparty transport configuration.
- `FixScriptedTransportFailure` — Injected scripted-transport failure point.
- `FixScriptedTransportError` — Scripted transport error.
- `FixScriptedTransport` — Bounded deterministic counterparty transport for adapter certification.
- `FixCertificationClock` — Deterministic coherent clock for FIX adapter certification.
- `FixCertificationClockError` — Deterministic certification clock error.

Feature-gated public FIX helper functions under `of_execution_adapters::fix`:

- `parse_execution_report(message, config, ts_recv_ns) -> FixExecutionReport` — Parses a validated FIX `ExecutionReport(35=8)` into a normalized report.
- `parse_order_cancel_reject(message, config, ts_recv_ns) -> FixOrderCancelReject` — Parses a validated FIX `OrderCancelReject(35=9)` into a normalized report.
- `encode_order_request(out, version, header, config, request, transact_time)` — Encodes a canonical new-order request as FIX NewOrderSingle `<D>`.
- `encode_cancel_request(out, version, header, request, context)` — Encodes a canonical cancel request as FIX OrderCancelRequest `<F>`.
- `encode_amend_request(out, version, header, config, request, context)` — Encodes a canonical amend request as FIX OrderCancelReplaceRequest `<G>`.
- `encode_stop_amend_request(out, version, header, config, request, context)` — Encodes a stop/stop-limit amend request as FIX OrderCancelReplaceRequest `<G>`.
- `map_execution_report(report) -> ExecutionEvent` — Maps a parsed FIX execution report into a canonical execution event.
- `map_order_cancel_reject(report) -> ExecutionEvent` — Maps a parsed FIX OrderCancelReject into a canonical execution event.

For mapping rules and adapter implementation guidance, see
[`of_execution_adapters` reference](./05i-of-execution-adapters-reference.md)
and [Provider Adapter Authoring](./12-provider-adapter-authoring.md).

---

## C API (`orderflow.h`)

The C API is a stable ABI, not a direct projection of Rust ownership. The
caller creates an opaque handle, supplies validated input structs or caller-owned
buffers, checks the returned error code, and destroys the handle exactly once.
Existing `repr(C)` layouts and function signatures are compatibility surfaces;
new functionality is added through new symbols and additive payload fields.

Every C call should be read through three questions: who owns this pointer,
does this call mutate engine state or only read a snapshot, and does success
prove durable/venue state or only local acceptance? Snapshot functions use
capacity negotiation rather than truncation. Allocated strings have an
explicit `of_string_free` owner path.

### Opaque Handles

- `of_engine_t` — Opaque C ABI handle whose lifetime is controlled by the matching create/destroy functions.
- `of_subscription_t` — Opaque C ABI handle whose lifetime is controlled by the matching create/destroy functions.

### Data Structures

- `of_engine_config_t` — C ABI configuration structure copied and validated by the native engine.
- `of_symbol_t` — C ABI data structure carrying the documented value or event fields supplied by the caller or native library.
- `of_trade_t` — C ABI data structure carrying the documented value or event fields supplied by the caller or native library.
- `of_book_t` — C ABI data structure carrying the documented value or event fields supplied by the caller or native library.
- `of_external_feed_policy_t` — C ABI data structure carrying the documented value or event fields supplied by the caller or native library.
- `of_event_t` — C ABI data structure carrying the documented value or event fields supplied by the caller or native library.

### Enums and constants

- `of_side_t`: `OF_SIDE_BID`, `OF_SIDE_ASK` — C ABI enumeration selecting the direction or book mutation represented by the event.
- `of_book_action_t`: `OF_BOOK_ACTION_UPSERT`, `OF_BOOK_ACTION_DELETE` — C ABI enumeration selecting the direction or book mutation represented by the event.
- `of_error_t`: `OF_OK`, `OF_ERR_INVALID_ARG`, `OF_ERR_STATE`, `OF_ERR_IO`, `OF_ERR_AUTH`, `OF_ERR_BACKPRESSURE`, `OF_ERR_DATA_QUALITY`, `OF_ERR_INTERNAL` — C ABI error enumeration that classifies success, invalid input, state, I/O, backpressure, and quality outcomes.

### Functions

Lifecycle:

- `of_api_version()` — Returns native compatibility/build metadata without mutating engine state.
- `of_build_info()` — Returns native compatibility/build metadata without mutating engine state.
- `of_engine_create(...)` — Allocates and initializes an opaque native engine handle.
- `of_engine_start(...)` — Starts the native engine after configuration validation.
- `of_engine_stop(...)` — Stops native processing without releasing the engine handle.
- `of_engine_destroy(...)` — Releases the native engine handle and all owned state.

Subscription and processing:

- `of_subscribe(...)` — Registers a symbol stream and callback subscription in the native engine.
- `of_unsubscribe(...)` — Removes a subscription while preserving the engine handle.
- `of_unsubscribe_symbol(...)` — Removes all active streams for one symbol.
- `of_reset_symbol_session(...)` — Clears the selected symbol's session analytics state.
- `of_engine_poll_once(...)` — Advances one bounded native processing cycle.

External ingest and quality supervision:

- `of_ingest_trade(...)` — Validates and applies one caller-supplied trade event.
- `of_ingest_book(...)` — Validates and applies one caller-supplied book update.
- `of_configure_external_feed(...)` — Configures stale-feed and sequence policy for a host-owned feed.
- `of_external_set_reconnecting(...)` — Marks the host-owned feed as reconnecting or restored.
- `of_external_health_tick(...)` — Advances stale-feed supervision for the host-owned feed.

Snapshots and metrics:

- `of_get_book_snapshot(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_analytics_snapshot(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_derived_analytics_snapshot(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_session_candle_snapshot(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_interval_candle_snapshot(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_signal_snapshot(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_metrics_json(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_adapter_inventory_json(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_active_adapter_status_json(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_signal_descriptors_json(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_signal_explanation_json(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_signal_metrics_json(...)` — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_string_free(...)` — Releases a string allocated by the native library.

### Stream Kind IDs

Used in `of_subscribe(..., kind, ...)` and callback payloads:

- `1`: BOOK — Stream identifier used when subscribing and dispatching callbacks.
- `2`: TRADES — Stream identifier used when subscribing and dispatching callbacks.
- `3`: ANALYTICS — Stream identifier used when subscribing and dispatching callbacks.
- `4`: SIGNALS — Stream identifier used when subscribing and dispatching callbacks.
- `5`: HEALTH — Stream identifier used when subscribing and dispatching callbacks.
- `6`: BOOK_SNAPSHOT — Stream identifier used when subscribing and dispatching callbacks.
- `7`: DERIVED_ANALYTICS — Stream identifier used when subscribing and dispatching callbacks.

### C API Notes

The lifecycle is intentionally explicit:

```text
of_engine_create -> of_engine_start -> subscribe/configure
  -> poll or ingest -> snapshot/metrics/callbacks
  -> stop -> destroy
```

Calling a method after destroy is invalid. `OF_ERR_BACKPRESSURE` means the
bounded runtime could not accept work; it is not equivalent to successful
ingest. `OF_ERR_DATA_QUALITY` means the event or state failed the configured
quality boundary; callers must not convert it into a normal analytics result.

- `of_get_book_snapshot(...)` returns populated JSON when book updates exist for the symbol. — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `BOOK_SNAPSHOT` callback payloads use the same JSON contract as `of_get_book_snapshot(...)`. — Named constant used by the public compatibility contract.
- `DERIVED_ANALYTICS` callback payloads use the same JSON contract as `of_get_derived_analytics_snapshot(...)`. — Named constant used by the public compatibility contract.
- Book snapshot JSON includes:
  - `venue` — JSON field carrying the corresponding identity, materialized level, sequence, or timestamp value.
  - `symbol` — JSON field carrying the corresponding identity, materialized level, sequence, or timestamp value.
  - `bids` — JSON field carrying the corresponding identity, materialized level, sequence, or timestamp value.
  - `asks` — JSON field carrying the corresponding identity, materialized level, sequence, or timestamp value.
  - `last_sequence` — JSON field carrying the corresponding identity, materialized level, sequence, or timestamp value.
  - `ts_exchange_ns` — JSON field carrying the corresponding identity, materialized level, sequence, or timestamp value.
  - `ts_recv_ns` — JSON field carrying the corresponding identity, materialized level, sequence, or timestamp value.
- `of_get_analytics_snapshot(...)`, `of_get_derived_analytics_snapshot(...)`, `of_get_session_candle_snapshot(...)`, `of_get_interval_candle_snapshot(...)`, and `of_get_signal_snapshot(...)` return populated JSON when data exists. — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
- `of_get_metrics_json(...)`, `of_get_adapter_inventory_json(...)`, — Serializes the requested read-only snapshot or diagnostic into the caller's buffer.
  `of_get_active_adapter_status_json(...)`, and
  `of_get_signal_descriptors_json(...)`, and
  `of_get_signal_explanation_json(...)`, and `of_get_signal_metrics_json(...)`
  allocate output strings; callers must free them via `of_string_free(...)`.
- Snapshot functions report the required byte size via `inout_len`; callers should retry with a larger buffer when they receive `OF_ERR_INVALID_ARG`.

---

## Python Binding API (`bindings/python/orderflow/api.py`)

The Python package is an ergonomic wrapper over the C ABI. It translates
native error codes into Python exceptions, converts JSON payloads into Python
values, and provides context-manager cleanup. It does not change the runtime
state model or make callbacks reentrant.

### Public classes/constants

- `StreamKind` (`BOOK`, `TRADES`, `ANALYTICS`, `SIGNALS`, `HEALTH`, `BOOK_SNAPSHOT`, `DERIVED_ANALYTICS`) — Selects the event or snapshot stream delivered by the engine.
- `Side` (`BID`, `ASK`) — Identifies bid/ask direction using the binding's stable numeric mapping.
- `BookAction` (`UPSERT`, `DELETE`) — Selects whether a book level is inserted/replaced or deleted.
- `DataQualityFlags` constants — Carries feed freshness, ordering, sequence, depth, and adapter-quality conditions.
- `OrderflowError`, `OrderflowStateError`, `OrderflowArgError` — Base exception for an operation rejected by the native library.
- `Symbol` — Identifies one venue-native instrument and its binding-side metadata.
- `EngineConfig` — Controls engine identity, provider selection, persistence, and bounded runtime policy.
- `ExternalFeedPolicy` — Controls stale-feed detection and external sequence enforcement.
- `Engine` — Owns the binding lifecycle for the native market-data runtime.

### `Engine` public methods/properties

Use `Engine` as a scoped resource. Start/configure it, subscribe or ingest
bounded work, inspect health before acting on snapshots, then stop and close
even when the application raises. Convenience snapshot methods retry the C
capacity negotiation, but applications should impose a memory ceiling when
payload size can be influenced by depth or diagnostics.

- `api_version` (property) — Returns the native ABI version used for compatibility checks.
- `build_info` (property) — Returns build and feature information for diagnostics.
- `start()` — Starts processing after configuration and startup validation.
- `stop()` — Stops processing and begins the explicit shutdown barrier.
- `close()` — Releases the owned native handle and makes further use invalid.
- `subscribe(symbol, stream_kind=..., callback=None)` — Registers a symbol or stream and records the requested subscription state.
- `poll_once(quality_flags=DataQualityFlags.NONE)` — Advances one bounded host-controlled processing cycle.
- `unsubscribe(symbol)` — Removes a symbol or stream subscription and releases its active state.
- `reset_symbol_session(symbol)` — Changes or releases the associated public lifecycle state according to its arguments.
- `configure_external_feed(policy)` — Changes or releases the associated public lifecycle state according to its arguments.
- `set_external_reconnecting(reconnecting)` — Marks an externally managed feed as reconnecting or restored.
- `external_health_tick()` — Advances stale-feed supervision when the host owns the external feed loop.
- `ingest_trade(symbol, price, size, aggressor_side, sequence=0, ts_exchange_ns=0, ts_recv_ns=0, quality_flags=...)` — Validates and applies one externally supplied normalized trade.
- `ingest_book(symbol, side, level, price, size, action=..., sequence=0, ts_exchange_ns=0, ts_recv_ns=0, quality_flags=...)` — Validates and applies one externally supplied normalized book update.
- `book_snapshot(symbol) -> dict` — Returns the materialized book read model for one symbol.
- `analytics_snapshot(symbol) -> dict` — Returns the current session analytics read model for one symbol.
- `derived_analytics_snapshot(symbol) -> dict` — Returns additive derived analytics for one symbol.
- `session_candle_snapshot(symbol) -> dict` — Returns the current session OHLCV-style summary.
- `interval_candle_snapshot(symbol, window_ns) -> dict` — Returns the analytics summary for the requested rolling window.
- `signal_snapshot(symbol) -> dict` — Returns the current signal state, confidence, and gating result.
- `signal_explanation(symbol) -> dict` — Returns the signal's structured reason and decision context.
- `signal_metrics() -> dict` — Returns signal lifecycle, transition, and evaluation metrics.
- `metrics() -> dict` — Returns adapter counters.
- `adapter_inventory(library_path=None) -> dict` — Returns descriptors for adapters compiled into or discoverable by the native runtime.
- `available_adapters(library_path=None) -> list[dict]` — Lists adapter descriptors that the binding can discover from the selected native library.
- `signal_descriptors(library_path=None) -> dict` — Returns discoverable signal names, versions, inputs, and configuration metadata.
- `Engine.adapter_inventory() -> dict` — Returns descriptors for adapters compiled into or discoverable by the native runtime.
- `Engine.adapter_status() -> dict` — Returns the active adapter's redacted operational status.
- `Engine.signal_descriptors() -> dict` — Returns discoverable signal names, versions, inputs, and configuration metadata.

Context manager support:

- `with Engine(config) as eng: ...` — Scopes native engine ownership so cleanup runs when the block exits.

---

## Java Binding API (`bindings/java/src/main/java/com/orderflow/bindings`)

The Java binding applies the same native contract through JNA. `OrderflowEngine`
is `AutoCloseable`, native errors become typed Java exceptions, and snapshot
buffers are retried using platform-sized `size_t` references. Callback objects
must remain strongly reachable until unregistration and may be invoked from a
native thread; callback work should be handed to an application executor.

### Public user-facing classes

- `OrderflowEngine` (`AutoCloseable`) — Owns the Java lifecycle for the native market-data and execution surface.
- `EngineConfig` — Controls engine identity, provider selection, persistence, and bounded runtime policy.
- `Symbol` — Identifies one venue-native instrument and its binding-side metadata.
- `StreamKind` — Selects the event or snapshot stream delivered by the engine.
- `DataQualityFlags` — Carries feed freshness, ordering, sequence, depth, and adapter-quality conditions.
- `Side` — Identifies bid/ask direction using the binding's stable numeric mapping.
- `BookAction` — Selects whether a book level is inserted/replaced or deleted.
- `OrderflowEvent` — Represents one decoded callback event delivered from the native stream.
- `EventListener` — Receives native stream events and must remain lightweight and non-reentrant.
- `OrderflowException` — Error type describing invalid input, lifecycle state, or failed external work.
- `OrderflowArgException` — Error type describing invalid input, lifecycle state, or failed external work.
- `OrderflowStateException` — Error type describing invalid input, lifecycle state, or failed external work.

### `OrderflowEngine` public methods

- `apiVersion()` — Returns the native ABI version used for compatibility checks.
- `buildInfo()` — Returns native build and feature information for diagnostics.
- `start()` — Starts processing after configuration and startup validation.
- `stop()` — Stops processing and begins the explicit shutdown barrier.
- `subscribe(Symbol, int)` — Registers a symbol or stream and records the requested subscription state.
- `subscribe(Symbol, int, EventListener)` — Registers a symbol or stream and records the requested subscription state.
- `pollOnce(int qualityFlags)` — Advances one bounded host-controlled processing cycle.
- `unsubscribe(Symbol)` — Removes a symbol or stream subscription and releases its active state.
- `resetSymbolSession(Symbol)` — Changes or releases the associated public lifecycle state according to its arguments.
- `configureExternalFeed(long staleAfterMs, boolean enforceSequence)` — Changes or releases the associated public lifecycle state according to its arguments.
- `setExternalReconnecting(boolean reconnecting)` — Marks an externally managed feed as reconnecting or restored.
- `externalHealthTick()` — Advances stale-feed supervision for an externally managed feed.
- `ingestTrade(Symbol, long price, long size, int aggressorSide)` — Validates and applies one externally supplied normalized trade.
- `ingestTrade(Symbol, long price, long size, int aggressorSide, long sequence, long tsExchangeNs, long tsRecvNs, int qualityFlags)` — Validates and applies one externally supplied normalized trade.
- `ingestBook(Symbol, int side, int level, long price, long size)` — Validates and applies one externally supplied normalized book update.
- `ingestBook(Symbol, int side, int level, long price, long size, int action, long sequence, long tsExchangeNs, long tsRecvNs, int qualityFlags)` — Validates and applies one externally supplied normalized book update.
- `bookSnapshot(Symbol)` — Read-only result describing the operation's observed state and diagnostics.
- `analyticsSnapshot(Symbol)` — Read-only result describing the operation's observed state and diagnostics.
- `derivedAnalyticsSnapshot(Symbol)` — Read-only result describing the operation's observed state and diagnostics.
- `sessionCandleSnapshot(Symbol)` — Read-only result describing the operation's observed state and diagnostics.
- `intervalCandleSnapshot(Symbol, long windowNs)` — Read-only result describing the operation's observed state and diagnostics.
- `signalSnapshot(Symbol)` — Read-only result describing the operation's observed state and diagnostics.
- `signalExplanation(Symbol)` — Returns the signal's structured reason and decision context.
- `signalMetrics()` — Read-only result describing the operation's observed state and diagnostics.
- `metricsJson()` — Reads the current public state or diagnostic value without changing ownership.
- `adapterInventory(String nativePath)` — Returns descriptors for adapters compiled into or discoverable by the native runtime.
- `adapterInventory()` — Returns descriptors for adapters compiled into or discoverable by the native runtime.
- `adapterStatus()` — Returns the active adapter's redacted operational status.
- `signalDescriptors(String nativePath)` — Returns discoverable signal names, versions, inputs, and configuration metadata.
- `signalDescriptors()` — Returns discoverable signal names, versions, inputs, and configuration metadata.
- `close()` — Releases the owned native handle and makes further use invalid.

---

## JSON Payload Contracts

### BOOK event payload (`StreamKind=1`)

```json
{
  "venue": "CME",
  "symbol": "ESM6",
  "side": "Bid|Ask",
  "level": 0,
  "price": 504900,
  "size": 20,
  "action": "Upsert|Delete",
  "sequence": 1,
  "ts_exchange_ns": 1000,
  "ts_recv_ns": 1100
}
```

### TRADES event payload (`StreamKind=2`)

```json
{
  "venue": "CME",
  "symbol": "ESM6",
  "price": 505000,
  "size": 7,
  "aggressor": "Bid|Ask",
  "sequence": 2,
  "ts_exchange_ns": 1200,
  "ts_recv_ns": 1300
}
```

### ANALYTICS snapshot/payload (`StreamKind=3`)

```json
{
  "delta": 7,
  "cumulative_delta": 21,
  "buy_volume": 55,
  "sell_volume": 48,
  "last_price": 505000,
  "point_of_control": 504900,
  "value_area_low": 504700,
  "value_area_high": 505100
}
```

### Derived analytics snapshot (`of_get_derived_analytics_snapshot`)

```json
{
  "total_volume": 15,
  "trade_count": 2,
  "vwap": 504966,
  "average_trade_size": 7,
  "imbalance_bps": 3333
}
```

### Session candle snapshot (`of_get_session_candle_snapshot`)

```json
{
  "open": 505000,
  "high": 505000,
  "low": 504900,
  "close": 504900,
  "trade_count": 2,
  "first_ts_exchange_ns": 10,
  "last_ts_exchange_ns": 20
}
```

### Interval candle snapshot (`of_get_interval_candle_snapshot`)

Rolling interval candle snapshot for a caller-supplied `window_ns`.

```json
{
  "window_ns": 70000000000,
  "open": 504900,
  "high": 505100,
  "low": 504900,
  "close": 505100,
  "trade_count": 2,
  "total_volume": 12,
  "vwap": 505033,
  "first_ts_exchange_ns": 40,
  "last_ts_exchange_ns": 100
}
```

### SIGNAL snapshot/payload (`StreamKind=4`)

```json
{
  "module": "delta_momentum_v1",
  "state": "neutral|long_bias|short_bias|blocked",
  "confidence_bps": 500,
  "quality_flags": 0,
  "reason": "delta_inside_band"
}
```

### HEALTH payload (`StreamKind=5`)

```json
{
  "health_seq": 3,
  "started": true,
  "connected": true,
  "degraded": false,
  "reconnect_state": "streaming|degraded|disconnected",
  "quality_flags": 0,
  "quality_flags_detail": [],
  "last_error": null,
  "protocol_info": "mock_adapter",
  "tracked_symbols": 1,
  "processed_events": 120,
  "external_feed_enabled": true,
  "external_feed_reconnecting": false,
  "external_sequence_enforced": true,
  "external_last_ingest_ns": 1712500000123456789
}
```

### BOOK_SNAPSHOT payload (`StreamKind=6`)

```json
{
  "venue": "CME",
  "symbol": "ESM6",
  "bids": [{"level": 0, "price": 504900, "size": 20}],
  "asks": [{"level": 0, "price": 505000, "size": 18}],
  "last_sequence": 8,
  "ts_exchange_ns": 1400,
  "ts_recv_ns": 1500
}
```

### DERIVED_ANALYTICS payload (`StreamKind=7`)

```json
{
  "total_volume": 15,
  "trade_count": 2,
  "vwap": 504966,
  "average_trade_size": 7,
  "imbalance_bps": 3333
}
```

### Metrics payload

```json
{
  "instance_id": "example",
  "started": true,
  "processed_events": 120,
  "symbols": 1,
  "book_symbols": 1,
  "analytics_symbols": 1,
  "signal_symbols": 1,
  "persistence": false,
  "health_seq": 3,
  "quality_flags": 0,
  "quality_flags_detail": [],
  "adapter_connected": true,
  "adapter_degraded": false,
  "adapter_last_error": null,
  "adapter_protocol_info": "mock_adapter",
  "external_feed_enabled": true,
  "external_feed_reconnecting": false,
  "external_sequence_enforced": true,
  "external_stale_after_ms": 15000,
  "external_last_ingest_ns": 1712500000123456789,
  "external_trade_sequence_symbols": 1,
  "external_book_sequence_symbols": 1
}
```

### Callback schema guarantees

- `of_event_t.schema_id` is currently `1` for all shipped stream payloads. — JSON field carrying the corresponding identity, materialized level, sequence, or timestamp value.
- Within `schema_id = 1`, payload evolution is additive-only:
  - existing field names are retained
  - existing field meanings are retained
  - new fields may be appended
- Removing or repurposing fields requires a future schema-id change.

---

## Error Mapping

C error codes:

- `0`: success — Native error value returned to classify the operation outcome.
- `1`: invalid argument — Native error value returned to classify the operation outcome.
- `2`: invalid state — Native error value returned to classify the operation outcome.
- `3`: I/O — Native error value returned to classify the operation outcome.
- `4`: auth — Native error value returned to classify the operation outcome.
- `5`: backpressure — Native error value returned to classify the operation outcome.
- `6`: data quality — Native error value returned to classify the operation outcome.
- `255`: internal — Native error value returned to classify the operation outcome.

Binding behavior:

- Python maps non-zero codes to `Orderflow*Error`.
- Java maps `1` to `OrderflowArgException`, `2` to `OrderflowStateException`, others to `OrderflowException`.
