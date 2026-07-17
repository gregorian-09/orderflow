# API Reference (Rust, C, Python, Java)

This page is the API index for the current codebase. The detailed Rust crate
reference has been split into dedicated handbook chapters so each public
surface can carry field-level and method-level documentation without collapsing
into one oversized page.

## Compatibility Layers

- **Rust crates** are the implementation and extension surface.
- **C ABI** (`crates/of_ffi_c/include/orderflow.h`) is the stable cross-language boundary.
- **Python** wraps C ABI with `ctypes`.
- **Java** wraps C ABI with JNA.

The C ABI also has a machine-readable inventory at
`bindings/api_manifest.toml`. `tools/check_api_manifest.py` validates that the
manifest and `orderflow.h` expose the same functions and return types, while
`tools/check_ffi_exports.sh` uses the manifest as the expected native symbol
list. The manifest is for low-level parity and future generated declarations;
the Python and Java user-facing APIs remain manually designed.

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

Binding-specific docs:

- [Python binding handbook](../bindings/python.md)
- [Java binding handbook](../bindings/java.md)
- [C ABI header](../../crates/of_ffi_c/include/orderflow.h)

Execution and OMS workflow docs:

- [OMS architecture](./09-oms-architecture.md)
- [OMS cookbook](./10-oms-cookbook.md)
- [Low-latency design](./11-low-latency-design.md)
- [Provider adapter authoring](./12-provider-adapter-authoring.md)
- [OMS recovery and operations](./13-recovery-and-operations.md)

---

## Rust API

### `of_core`

Public types:

- `SymbolId { venue, symbol }`
- `Side` (`Bid`, `Ask`)
- `BookAction` (`Upsert`, `Delete`)
- `BookUpdate`
- `TradePrint`
- `AnalyticsSnapshot`
- `DerivedAnalyticsSnapshot`
- `IntervalCandleSnapshot`
- `SignalState` (`Neutral`, `LongBias`, `ShortBias`, `Blocked`)
- `SignalSnapshot`
- `DataQualityFlags`
- `AnalyticsAccumulator`

Public `DataQualityFlags` constants:

- `NONE`
- `STALE_FEED`
- `SEQUENCE_GAP`
- `CLOCK_SKEW`
- `DEPTH_TRUNCATED`
- `OUT_OF_ORDER`
- `ADAPTER_DEGRADED`

Public methods:

- `DataQualityFlags::bits() -> u32`
- `DataQualityFlags::from_bits_truncate(u32) -> DataQualityFlags`
- `DataQualityFlags::intersects(DataQualityFlags) -> bool`
- `AnalyticsAccumulator::on_trade(&TradePrint)`
- `AnalyticsAccumulator::reset_session_delta()`
- `AnalyticsAccumulator::reset_session()`
- `AnalyticsAccumulator::snapshot() -> AnalyticsSnapshot`
- `AnalyticsAccumulator::derived_snapshot() -> DerivedAnalyticsSnapshot`
- `AnalyticsAccumulator::session_candle_snapshot() -> SessionCandleSnapshot`
- `AnalyticsAccumulator::interval_candle_snapshot(window_ns: u64) -> IntervalCandleSnapshot`

### `of_adapters`

Public types:

- `SubscribeReq { symbol, depth_levels }`
- `AdapterHealth { connected, degraded, last_error, protocol_info }`
- `RawEvent` (`Book(BookUpdate)`, `Trade(TradePrint)`)
- `AdapterError`
- `AdapterResult<T>`
- `MarketDataAdapter` trait
- `ProviderKind` (`Mock`, `Rithmic`, `Cqg`, `Binance`)
- `AdapterQualityLevel`
- `AdapterDescriptor`
- `AdapterConfig`
- `CredentialsRef`
- `MockAdapter`

Public functions/methods:

- `adapter_descriptors() -> &'static [AdapterDescriptor]`
- `compiled_adapter_descriptors() -> Vec<AdapterDescriptor>`
- `describe_adapter(ProviderKind) -> AdapterDescriptor`
- `adapter_feature_enabled(ProviderKind) -> bool`
- `create_adapter(&AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>>`
- `MockAdapter::push_event(RawEvent)`

`MarketDataAdapter` trait methods:

- `connect()`
- `subscribe(SubscribeReq)`
- `unsubscribe(SymbolId)`
- `poll(&mut Vec<RawEvent>)`
- `health() -> AdapterHealth`

### `of_signals`

Public types:

- `SignalGateDecision` (`Pass`, `Block`)
- `SignalModule` trait
- `ExplainableSignalModule` trait
- `SignalExplanation`
- `SignalReasonCode`
- `SignalExplanationMode`
- `SignalConfig`
- `SignalConfigParameter`
- `SignalConfigValue`
- `SignalRegistration`
- `SignalRegistry`
- `SignalRegistryError`
- `SignalMarkoutDirection`
- `SignalReplayEvent`
- `SignalValidationConfig`
- `SignalValidationWarning`
- `SignalValidationSample`
- `SignalValidationReport`
- `SignalValidationHarness`
- `SignalCalibrationConfig`
- `SignalConfidenceCalibrator`
- `IdentitySignalCalibrator`
- `SignalCalibrationPoint`
- `SignalCalibrationCurve`
- `SignalOutcomeRecord`
- `SignalCalibrationBin`
- `SignalRegimeSummary`
- `SignalCalibrationReport`
- `SignalCalibrationBinDrift`
- `SignalCalibrationDriftReport`
- `SignalOutcomeTracker`
- `SignalEnsembleDecisionRule`
- `SignalEnsembleConflictPolicy`
- `SignalEnsembleVetoPolicy`
- `SignalEnsemblePolicy`
- `SignalEnsembleVote`
- `SignalEnsembleConflict`
- `SignalEnsembleMetrics`
- `SignalEnsembleDecision`
- `SignalEnsembleExplanation`
- `SignalCheckpoint`
- `SignalCheckpointRestorePolicy`
- `SignalCheckpointValidationIssue`
- `SignalCheckpointValidationReport`
- `SignalCheckpointRestoreError`
- `CheckpointableSignal`
- `SignalRunMode`
- `SignalRunModeDecision`
- `SignalShadowSample`
- `SignalShadowComparisonConfig`
- `SignalShadowComparisonReport`
- `SignalShadowRecorder`
- `FeatureQualityFlags`
- `FeatureValueKind`
- `FeatureMissingPolicy`
- `FeatureDescriptor`
- `FeatureSchema`
- `FeatureVectorView`
- `FeatureVectorValidationIssue`
- `FeatureVectorValidationReport`
- `SignalModelKind`
- `SignalModelOutputKind`
- `SignalModelMetadata`
- `SignalModelInputBinding`
- `SignalModelOutput`
- `ModelBackedSignal`
- `DeltaMomentumSignal`
- `VolumeImbalanceSignal`
- `CumulativeDeltaSignal`
- `AbsorptionSignal`
- `ExhaustionSignal`
- `SweepDetectionSignal`
- `CompositeSignal`

Public methods:

- `DeltaMomentumSignal::new(threshold: i64) -> Self`
- `VolumeImbalanceSignal::new(threshold: i64) -> Self`
- `CumulativeDeltaSignal::new(threshold: i64) -> Self`
- `AbsorptionSignal::new(threshold: i64, price_band: i64) -> Self`
- `ExhaustionSignal::new(threshold: i64) -> Self`
- `SweepDetectionSignal::new(threshold: i64, breakout_ticks: i64) -> Self`
- `CompositeSignal::new(modules: Vec<Box<dyn SignalModule>>) -> Self`

`SignalModule` trait methods:

- `on_analytics(&AnalyticsSnapshot)`
- `snapshot() -> SignalSnapshot`
- `quality_gate(DataQualityFlags) -> SignalGateDecision`

`ExplainableSignalModule` trait methods:

- `explanation() -> SignalExplanation`

Registry functions and methods:

- `built_in_signal_registrations() -> &'static [SignalRegistration]`
- `built_in_signal_descriptors_json() -> String`
- `SignalRegistry::with_built_ins() -> Self`
- `SignalRegistry::validate_config(&SignalConfig) -> SignalRegistryResult<()>`
- `SignalRegistry::create_signal(&SignalConfig) -> SignalRegistryResult<Box<dyn SignalModule>>`
- `SignalRegistry::descriptors_json() -> String`

Validation functions and methods:

- `validate_signal_replay(&mut impl SignalModule, &[AnalyticsSnapshot], SignalValidationConfig) -> SignalValidationReport`
- `validate_signal_replay_events(&mut impl SignalModule, &[SignalReplayEvent], SignalValidationConfig) -> SignalValidationReport`
- `SignalValidationHarness::validate_signal(...) -> SignalValidationReport`
- `SignalValidationReport::json_summary() -> String`

Calibration functions and methods:

- `SignalCalibrationConfig::new(bin_width_bps) -> SignalCalibrationConfig`
- `SignalCalibrationPoint::new(raw_confidence_bps, calibrated_confidence_bps) -> SignalCalibrationPoint`
- `SignalCalibrationCurve::new(points) -> SignalCalibrationCurve`
- `SignalConfidenceCalibrator::calibrate_confidence_bps(raw_confidence_bps) -> u16`
- `SignalOutcomeRecord::new(module_id, state, confidence_bps, predicted_direction, markout_direction, correct) -> SignalOutcomeRecord`
- `SignalOutcomeRecord::from_validation_sample(&SignalValidationSample) -> SignalOutcomeRecord`
- `SignalCalibrationReport::from_records(&[SignalOutcomeRecord], SignalCalibrationConfig) -> SignalCalibrationReport`
- `SignalCalibrationReport::from_validation_report(&SignalValidationReport, SignalCalibrationConfig) -> SignalCalibrationReport`
- `SignalCalibrationReport::accuracy_bps() -> Option<u16>`
- `SignalCalibrationReport::json_summary() -> String`
- `SignalCalibrationDriftReport::compare(&baseline, &current, threshold_bps) -> SignalCalibrationDriftReport`
- `SignalOutcomeTracker::record(SignalOutcomeRecord)`
- `SignalOutcomeTracker::extend_validation_report(&SignalValidationReport)`
- `SignalOutcomeTracker::calibration_report() -> SignalCalibrationReport`
- `SignalOutcomeTracker::drift_report(&SignalCalibrationReport) -> SignalCalibrationDriftReport`

Ensemble functions and methods:

- `SignalEnsemblePolicy::majority() -> SignalEnsemblePolicy`
- `SignalEnsemblePolicy::quorum(min_votes) -> SignalEnsemblePolicy`
- `SignalEnsemblePolicy::weighted(min_score_bps) -> SignalEnsemblePolicy`
- `SignalEnsemblePolicy::with_conflict_policy(policy) -> SignalEnsemblePolicy`
- `SignalEnsemblePolicy::with_veto_policy(policy) -> SignalEnsemblePolicy`
- `SignalEnsemblePolicy::with_min_confidence_bps(min_confidence_bps) -> SignalEnsemblePolicy`
- `SignalEnsembleVote::new(module_id, state, confidence_bps) -> SignalEnsembleVote`
- `SignalEnsembleVote::from_snapshot(&SignalSnapshot) -> SignalEnsembleVote`
- `SignalEnsembleVote::from_explanation(&SignalExplanation) -> SignalEnsembleVote`
- `SignalEnsembleVote::with_weight_bps(weight_bps) -> SignalEnsembleVote`
- `SignalEnsembleVote::with_veto(veto) -> SignalEnsembleVote`
- `evaluate_signal_ensemble(module_id, votes, policy) -> SignalEnsembleDecision`
- `evaluate_signal_ensemble_explanations(module_id, child_explanations, weights_bps, policy) -> SignalEnsembleExplanation`
- `SignalEnsembleExplanation::explanation() -> SignalExplanation`

Checkpoint and shadow-mode functions and methods:

- `SignalCheckpoint::new(module_id, signal_version, state) -> SignalCheckpoint`
- `SignalCheckpoint::from_snapshot(&SignalSnapshot, signal_version) -> SignalCheckpoint`
- `SignalCheckpointRestorePolicy::new() -> SignalCheckpointRestorePolicy`
- `validate_signal_checkpoint_restore(&SignalCheckpoint, &SignalCheckpointRestorePolicy) -> SignalCheckpointValidationReport`
- `CheckpointableSignal::checkpoint() -> SignalCheckpoint`
- `CheckpointableSignal::restore_checkpoint(&SignalCheckpoint) -> Result<(), SignalCheckpointRestoreError>`
- `SignalRunModeDecision::from_mode(SignalRunMode) -> SignalRunModeDecision`
- `SignalShadowSample::compare(event_index, production, candidate) -> SignalShadowSample`
- `SignalShadowSample::with_markout(markout_direction) -> SignalShadowSample`
- `SignalShadowComparisonReport::from_samples(samples, config) -> SignalShadowComparisonReport`
- `SignalShadowComparisonReport::agreement_bps() -> Option<u16>`
- `SignalShadowComparisonReport::production_accuracy_bps() -> Option<u16>`
- `SignalShadowComparisonReport::candidate_accuracy_bps() -> Option<u16>`
- `SignalShadowComparisonReport::json_summary() -> String`
- `SignalShadowRecorder::record(SignalShadowSample)`
- `SignalShadowRecorder::report() -> SignalShadowComparisonReport`

Feature vector and model-support functions and methods:

- `FeatureQualityFlags::bits() -> u32`
- `FeatureQualityFlags::from_bits_truncate(bits) -> FeatureQualityFlags`
- `FeatureDescriptor::new(id, value_kind) -> FeatureDescriptor`
- `FeatureSchema::new(id, version) -> FeatureSchema`
- `FeatureSchema::with_feature(feature) -> FeatureSchema`
- `FeatureSchema::feature_index(id) -> Option<usize>`
- `FeatureSchema::feature(id) -> Option<&FeatureDescriptor>`
- `FeatureVectorView::new(schema, values, quality, timestamp_ns) -> FeatureVectorView`
- `FeatureVectorView::value(id) -> Option<f64>`
- `FeatureVectorView::quality(id) -> Option<FeatureQualityFlags>`
- `FeatureVectorView::validate(now_ns) -> FeatureVectorValidationReport`
- `validate_feature_vector(&FeatureVectorView, now_ns) -> FeatureVectorValidationReport`
- `SignalModelMetadata::new(model_id, model_version, feature_schema_id, feature_schema_version) -> SignalModelMetadata`
- `SignalModelInputBinding::new(input_name, feature_ids) -> SignalModelInputBinding`
- `SignalModelInputBinding::is_compatible_with(&FeatureSchema) -> bool`
- `SignalModelOutput::new(state, confidence_bps) -> SignalModelOutput`
- `ModelBackedSignal::model_metadata() -> &SignalModelMetadata`
- `ModelBackedSignal::feature_schema() -> &FeatureSchema`
- `ModelBackedSignal::infer_features(&FeatureVectorView) -> SignalModelOutput`

### `of_persist`

Public types:

- `PersistError`
- `PersistResult<T>`
- `RetentionPolicy { max_total_bytes, max_age_secs }`
- `RollingStore`
- `StoredBookEvent`
- `StoredTradeEvent`
- `StoredEvent`

Public methods:

- `RollingStore::new(root) -> PersistResult<RollingStore>`
- `RollingStore::with_retention(Option<RetentionPolicy>) -> RollingStore`
- `RollingStore::append_book(&BookUpdate) -> PersistResult<()>`
- `RollingStore::append_trade(&TradePrint) -> PersistResult<()>`
- `RollingStore::list_venues() -> PersistResult<Vec<String>>`
- `RollingStore::list_symbols(venue) -> PersistResult<Vec<String>>`
- `RollingStore::list_streams(venue, symbol) -> PersistResult<Vec<String>>`
- `RollingStore::read_books(venue, symbol) -> PersistResult<Vec<StoredBookEvent>>`
- `RollingStore::read_books_in_range(venue, symbol, from_sequence, to_sequence) -> PersistResult<Vec<StoredBookEvent>>`
- `RollingStore::read_trades(venue, symbol) -> PersistResult<Vec<StoredTradeEvent>>`
- `RollingStore::read_trades_in_range(venue, symbol, from_sequence, to_sequence) -> PersistResult<Vec<StoredTradeEvent>>`
- `RollingStore::read_events(venue, symbol) -> PersistResult<Vec<StoredEvent>>`
- `RollingStore::read_events_in_range(venue, symbol, from_sequence, to_sequence) -> PersistResult<Vec<StoredEvent>>`

### `of_runtime`

Public types:

- `EngineConfig`
- `RuntimeError`
- `ConfigCompatibilityMode`
- `ConfigLoadReport`
- `ExternalFeedPolicy`
- `Engine<A, S>`
- `DefaultEngine` type alias

Public constructor/build/config functions:

- `Engine::new(cfg, adapter, signal_module) -> Engine<A, S>`
- `build_default_engine(cfg: EngineConfig) -> Result<DefaultEngine, RuntimeError>`
- `load_engine_config_from_path(path: &str) -> Result<EngineConfig, RuntimeError>`
  - preferred input shape: typed TOML/JSON with nested `adapter` / `adapter.credentials`
  - compatibility fallback: legacy flat config files remain accepted
- `load_engine_config_report_from_path(path: &str) -> Result<ConfigLoadReport, RuntimeError>`
  - reports `format`
  - reports `compatibility_mode`
  - surfaces a warning when legacy fallback was required
- `validate_startup_config(cfg: &EngineConfig) -> Result<(), RuntimeError>`

Public runtime methods:

- `with_persistence(Option<RollingStore>)`
- `start()`
- `stop()`
- `subscribe(SymbolId, depth_levels)`
- `unsubscribe(SymbolId)`
- `reset_symbol_session(SymbolId)`
- `configure_external_feed(ExternalFeedPolicy)`
- `set_external_reconnecting(bool)`
- `external_health_tick()`
- `ingest_trade(TradePrint, DataQualityFlags)`
- `ingest_book(BookUpdate, DataQualityFlags)`
- `poll_once(DataQualityFlags)`
- `analytics_snapshot(&SymbolId)`
- `derived_analytics_snapshot(&SymbolId)`
- `session_candle_snapshot(&SymbolId)`
- `interval_candle_snapshot(&SymbolId, window_ns: u64)`
- `signal_snapshot(&SymbolId)`
- `signal_explanation_json(&SymbolId) -> Option<String>`
- `signal_metrics_json() -> String`
- `adapter_descriptor() -> AdapterDescriptor`
- `adapter_status() -> RuntimeAdapterStatus`
- `adapter_inventory_json() -> String`
- `active_adapter_status_json() -> String`
- `signal_descriptor_inventory_json() -> String`
- `metrics_json() -> String`
- `health_seq() -> u64`
- `health_json() -> String`
- `last_events() -> &[RawEvent]`
- `current_quality_flags_bits() -> u32`

### `of_execution_core`

Public identifier types:

- `FixedAscii<N>`
- `ClientOrderId`
- `VenueOrderId`
- `ExecutionId`
- `AccountId`
- `RouteId`
- `StrategyId`
- `VenueId`
- `InstrumentId`
- `ExecutionText`

Public execution model types:

- `ExecutionSymbol`
- `OrderQty`
- `OrderPrice`
- `OrderSide`
- `OrderType`
- `TimeInForce`
- `OrderStatus`
- `ExecutionType`
- `OrderRequest`
- `CancelRequest`
- `AmendRequest`
- `ExecutionEvent`
- `OrderState`
- `OrderStateMachine`

Public risk types:

- `RiskRejectReason`
- `RiskDecision`
- `RiskLimits`
- `RiskContext`
- `RiskCheck` trait
- `BasicRiskGate`

Public execution WAL frame primitives:

- `EXECUTION_WAL_MAGIC`
- `EXECUTION_WAL_VERSION`
- `EXECUTION_WAL_HEADER_LEN`
- `EXECUTION_WAL_MAX_PAYLOAD_LEN`
- `ExecutionWalError`
- `WalChecksumField`
- `WalSequence`
- `WalSegmentId`
- `WalRecordKind`
- `WalSyncPolicy`
- `WalRecordHeader`
- `WalRecordView`
- `WalReplayCursor`
- `WalIntegrityReport`
- `execution_wal_checksum`

For field-level semantics and transition rules, see
[`of_execution_core` reference](./05g-of-execution-core-reference.md).

### `of_execution`

Public adapter and engine types:

- `ExecutionError`
- `ExecutionResult<T>`
- `ExecutionEventBuffer`
- `LatencyClass`
- `ExecutionCapabilities`
- `ExecutionHealth`
- `ExecutionAdapter` trait
- `RouteConfig`
- `RouteKey`
- `AllowAllRiskGate`
- `ExecutionEngine`
- `SimExecutionAdapter`

Public journal types:

- `JournalCommandKind`
- `JournalRecord`
- `ExecutionJournal` trait
- `InMemoryJournal`
- `WalJournalConfig`
- `WalReplayResult`
- `WalJournalMetrics`
- `WalExecutionJournal`
- `WalSegmentConfig`
- `WalSegmentMetadata`
- `WalSegmentManifest`
- `WalSegmentIntegrityReport`
- `SegmentedWalExecutionJournal`
- `CheckpointPosition`
- `ExecutionCheckpoint`
- `CheckpointPolicy`
- `CheckpointConfig`
- `CheckpointManifest`
- `ExecutionCheckpointStore` trait
- `FileExecutionCheckpointStore`
- `RecoveryCorruptionPolicy`
- `RecoveryVenuePolicy`
- `RecoveryPlan`
- `RecoveredOmsState`
- `RecoveryResult`
- `recover_oms_state_from_records`
- `recover_oms_state_from_segmented_wal`
- `recover_latest_checkpoint_from_segmented_wal`

Public concurrent execution types:

- `ConcurrentExecutionConfig`
- `ExecutionCommandKind`
- `ExecutionCommand`
- `ExecutionCommandReport`
- `ConcurrentExecutionError`
- `ExecutionCommandSender`
- `ConcurrentExecutionEngine`

Public OMS helper types:

- `CommandId`
- `RequestId`
- `CommandIdGenerator`
- `CommandCorrelation`
- `ExecutionEventFanout`
- `ExecutionEventSubscriber`
- `ExecutionAdapterState`
- `ExecutionLifecycle`
- `ExecutionLifecycleSnapshot`
- `FileExecutionJournal`
- `ReconciliationAction`
- `ReconciliationItem`
- `ReconciliationReport`
- `ReconciliationIssueKind`
- `ReconciliationDetail`
- `VenueReconciliationReport`
- `ReconciliationPolicyAction`
- `ReconciliationPolicy`
- `ReconciliationPolicyItem`
- `ReconciliationPolicyDecision`
- `DisconnectPolicy`
- `RouteSafetyPolicy`
- `AdvancedRiskLimits`
- `AdvancedRiskGate`
- `Position`
- `PositionKey`
- `PositionLedger`
- `VenueOrderCapabilities`
- `NormalizedOrderType`
- `ExecutionTelemetry`
- `ShardKey`
- `ShardRouter`
- `OrderThrottle`
- `ReplayDecision`
- `ReplayResult`
- `ProviderAdapterContext`
- `ExecutionAdapterFactory`
- `ProviderAdapterSdk`

Public helper functions:

- `simulated_engine(route) -> ExecutionEngine<SimExecutionAdapter, BasicRiskGate, InMemoryJournal>`
- `simulated_engine_with_routes(routes) -> ExecutionEngine<SimExecutionAdapter, AllowAllRiskGate, InMemoryJournal>`
- `reconcile_open_orders(local, venue) -> ReconciliationReport`
- `reconcile_open_orders_detailed(local, venue) -> VenueReconciliationReport`
- `evaluate_reconciliation_policy(report, policy) -> ReconciliationPolicyDecision`
- `normalize_order_type(order_type, tif, capabilities) -> NormalizedOrderType`
- `replay_simulated_oms(routes, decisions) -> ExecutionResult<ReplayResult>`

For lifecycle, routing, concurrency, and OMS helper details, see
[`of_execution` reference](./05h-of-execution-reference.md).

### `of_fix`

Public FIX codec types:

- `FixTag`
- `FixVersion`
- `FixMsgType`
- `FixFieldView`
- `FixMessageView`
- `FixParseError`
- `FixEncodeError`
- `FixProfileError`
- `FixMessageRule`
- `FixDictionary`
- `FixDecoder`
- `FixEncoder`
- `FixSessionState`
- `FixSequenceTracker`
- `FixSequenceAction`
- `FixSequenceError`
- `FixResendRange`
- `FixSessionId`
- `FixSequenceSnapshot`
- `FixSentMessageKind`
- `FixResendStoreConfig`
- `FixResendStore`
- `FixStoredMessage`
- `FixResendRetention`
- `FixResendStoreMetrics`
- `FixResendStoreError`
- `FixResendAction`
- `FixResendPlanSummary`
- `FixSessionHeader`
- `FixOrderSide`
- `FixOrdType`
- `FixTimeInForce`
- `FixNewOrderSingle`
- `FixOrderCancelRequest`
- `FixOrderCancelReplaceRequest`

Public FIX codec constants:

- `SOH`

Public FIX codec functions:

- `parse_message(raw, scratch) -> FixMessageView`
- `encode_message(out, begin_string, msg_type, fields)`
- `checksum(bytes) -> u8`
- `debug_render(raw) -> String`
- `encode_logon(out, version, header, heartbeat_interval_secs, reset_seq_num)`
- `encode_heartbeat(out, version, header, test_req_id)`
- `encode_test_request(out, version, header, test_req_id)`
- `encode_resend_request(out, version, header, range)`
- `encode_sequence_reset_gap_fill(out, version, header, new_seq_no)`
- `encode_logout(out, version, header, text)`
- `encode_new_order_single(out, version, header, request)`
- `encode_order_cancel_request(out, version, header, request)`
- `encode_order_cancel_replace_request(out, version, header, request)`

For low-allocation FIX parsing, validation, and encoding details, see
[`of_fix` reference](./05j-of-fix-reference.md).

### `of_execution_adapters`

Feature-gated public FIX scaffold types under `of_execution_adapters::fix`:

- `FixSessionConfig`
- `FixExecutionReport`
- `FixReportParseConfig`
- `FixReportParseError`
- `FixExecType`
- `FixOrdStatus`
- `FixExecutionAdapter`

Feature-gated public FIX helper functions under `of_execution_adapters::fix`:

- `parse_execution_report(message, config, ts_recv_ns) -> FixExecutionReport`
- `map_execution_report(report) -> ExecutionEvent`

For mapping rules and adapter implementation guidance, see
[`of_execution_adapters` reference](./05i-of-execution-adapters-reference.md)
and [Provider Adapter Authoring](./12-provider-adapter-authoring.md).

---

## C API (`orderflow.h`)

### Opaque Handles

- `of_engine_t`
- `of_subscription_t`

### Data Structures

- `of_engine_config_t`
- `of_symbol_t`
- `of_trade_t`
- `of_book_t`
- `of_external_feed_policy_t`
- `of_event_t`

### Enums and constants

- `of_side_t`: `OF_SIDE_BID`, `OF_SIDE_ASK`
- `of_book_action_t`: `OF_BOOK_ACTION_UPSERT`, `OF_BOOK_ACTION_DELETE`
- `of_error_t`: `OF_OK`, `OF_ERR_INVALID_ARG`, `OF_ERR_STATE`, `OF_ERR_IO`, `OF_ERR_AUTH`, `OF_ERR_BACKPRESSURE`, `OF_ERR_DATA_QUALITY`, `OF_ERR_INTERNAL`

### Functions

Lifecycle:

- `of_api_version()`
- `of_build_info()`
- `of_engine_create(...)`
- `of_engine_start(...)`
- `of_engine_stop(...)`
- `of_engine_destroy(...)`

Subscription and processing:

- `of_subscribe(...)`
- `of_unsubscribe(...)`
- `of_unsubscribe_symbol(...)`
- `of_reset_symbol_session(...)`
- `of_engine_poll_once(...)`

External ingest and quality supervision:

- `of_ingest_trade(...)`
- `of_ingest_book(...)`
- `of_configure_external_feed(...)`
- `of_external_set_reconnecting(...)`
- `of_external_health_tick(...)`

Snapshots and metrics:

- `of_get_book_snapshot(...)`
- `of_get_analytics_snapshot(...)`
- `of_get_derived_analytics_snapshot(...)`
- `of_get_session_candle_snapshot(...)`
- `of_get_interval_candle_snapshot(...)`
- `of_get_signal_snapshot(...)`
- `of_get_metrics_json(...)`
- `of_get_adapter_inventory_json(...)`
- `of_get_active_adapter_status_json(...)`
- `of_get_signal_descriptors_json(...)`
- `of_get_signal_explanation_json(...)`
- `of_get_signal_metrics_json(...)`
- `of_string_free(...)`

### Stream Kind IDs

Used in `of_subscribe(..., kind, ...)` and callback payloads:

- `1`: BOOK
- `2`: TRADES
- `3`: ANALYTICS
- `4`: SIGNALS
- `5`: HEALTH
- `6`: BOOK_SNAPSHOT
- `7`: DERIVED_ANALYTICS

### C API Notes

- `of_get_book_snapshot(...)` returns populated JSON when book updates exist for the symbol.
- `BOOK_SNAPSHOT` callback payloads use the same JSON contract as `of_get_book_snapshot(...)`.
- `DERIVED_ANALYTICS` callback payloads use the same JSON contract as `of_get_derived_analytics_snapshot(...)`.
- Book snapshot JSON includes:
  - `venue`
  - `symbol`
  - `bids`
  - `asks`
  - `last_sequence`
  - `ts_exchange_ns`
  - `ts_recv_ns`
- `of_get_analytics_snapshot(...)`, `of_get_derived_analytics_snapshot(...)`, `of_get_session_candle_snapshot(...)`, `of_get_interval_candle_snapshot(...)`, and `of_get_signal_snapshot(...)` return populated JSON when data exists.
- `of_get_metrics_json(...)`, `of_get_adapter_inventory_json(...)`,
  `of_get_active_adapter_status_json(...)`, and
  `of_get_signal_descriptors_json(...)`, and
  `of_get_signal_explanation_json(...)`, and `of_get_signal_metrics_json(...)`
  allocate output strings; callers must free them via `of_string_free(...)`.
- Snapshot functions report the required byte size via `inout_len`; callers should retry with a larger buffer when they receive `OF_ERR_INVALID_ARG`.

---

## Python Binding API (`bindings/python/orderflow/api.py`)

### Public classes/constants

- `StreamKind` (`BOOK`, `TRADES`, `ANALYTICS`, `SIGNALS`, `HEALTH`, `BOOK_SNAPSHOT`, `DERIVED_ANALYTICS`)
- `Side` (`BID`, `ASK`)
- `BookAction` (`UPSERT`, `DELETE`)
- `DataQualityFlags` constants
- `OrderflowError`, `OrderflowStateError`, `OrderflowArgError`
- `Symbol`
- `EngineConfig`
- `ExternalFeedPolicy`
- `Engine`

### `Engine` public methods/properties

- `api_version` (property)
- `build_info` (property)
- `start()`
- `stop()`
- `close()`
- `subscribe(symbol, stream_kind=..., callback=None)`
- `poll_once(quality_flags=DataQualityFlags.NONE)`
- `unsubscribe(symbol)`
- `reset_symbol_session(symbol)`
- `configure_external_feed(policy)`
- `set_external_reconnecting(reconnecting)`
- `external_health_tick()`
- `ingest_trade(symbol, price, size, aggressor_side, sequence=0, ts_exchange_ns=0, ts_recv_ns=0, quality_flags=...)`
- `ingest_book(symbol, side, level, price, size, action=..., sequence=0, ts_exchange_ns=0, ts_recv_ns=0, quality_flags=...)`
- `book_snapshot(symbol) -> dict`
- `analytics_snapshot(symbol) -> dict`
- `derived_analytics_snapshot(symbol) -> dict`
- `session_candle_snapshot(symbol) -> dict`
- `interval_candle_snapshot(symbol, window_ns) -> dict`
- `signal_snapshot(symbol) -> dict`
- `signal_explanation(symbol) -> dict`
- `signal_metrics() -> dict`
- `metrics() -> dict`
- `adapter_inventory(library_path=None) -> dict`
- `available_adapters(library_path=None) -> list[dict]`
- `signal_descriptors(library_path=None) -> dict`
- `Engine.adapter_inventory() -> dict`
- `Engine.adapter_status() -> dict`
- `Engine.signal_descriptors() -> dict`

Context manager support:

- `with Engine(config) as eng: ...`

---

## Java Binding API (`bindings/java/src/main/java/com/orderflow/bindings`)

### Public user-facing classes

- `OrderflowEngine` (`AutoCloseable`)
- `EngineConfig`
- `Symbol`
- `StreamKind`
- `DataQualityFlags`
- `Side`
- `BookAction`
- `OrderflowEvent`
- `EventListener`
- `OrderflowException`
- `OrderflowArgException`
- `OrderflowStateException`

### `OrderflowEngine` public methods

- `apiVersion()`
- `buildInfo()`
- `start()`
- `stop()`
- `subscribe(Symbol, int)`
- `subscribe(Symbol, int, EventListener)`
- `pollOnce(int qualityFlags)`
- `unsubscribe(Symbol)`
- `resetSymbolSession(Symbol)`
- `configureExternalFeed(long staleAfterMs, boolean enforceSequence)`
- `setExternalReconnecting(boolean reconnecting)`
- `externalHealthTick()`
- `ingestTrade(Symbol, long price, long size, int aggressorSide)`
- `ingestTrade(Symbol, long price, long size, int aggressorSide, long sequence, long tsExchangeNs, long tsRecvNs, int qualityFlags)`
- `ingestBook(Symbol, int side, int level, long price, long size)`
- `ingestBook(Symbol, int side, int level, long price, long size, int action, long sequence, long tsExchangeNs, long tsRecvNs, int qualityFlags)`
- `bookSnapshot(Symbol)`
- `analyticsSnapshot(Symbol)`
- `derivedAnalyticsSnapshot(Symbol)`
- `sessionCandleSnapshot(Symbol)`
- `intervalCandleSnapshot(Symbol, long windowNs)`
- `signalSnapshot(Symbol)`
- `signalExplanation(Symbol)`
- `signalMetrics()`
- `metricsJson()`
- `adapterInventory(String nativePath)`
- `adapterInventory()`
- `adapterStatus()`
- `signalDescriptors(String nativePath)`
- `signalDescriptors()`
- `close()`

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

- `of_event_t.schema_id` is currently `1` for all shipped stream payloads.
- Within `schema_id = 1`, payload evolution is additive-only:
  - existing field names are retained
  - existing field meanings are retained
  - new fields may be appended
- Removing or repurposing fields requires a future schema-id change.

---

## Error Mapping

C error codes:

- `0`: success
- `1`: invalid argument
- `2`: invalid state
- `3`: I/O
- `4`: auth
- `5`: backpressure
- `6`: data quality
- `255`: internal

Binding behavior:

- Python maps non-zero codes to `Orderflow*Error`.
- Java maps `1` to `OrderflowArgException`, `2` to `OrderflowStateException`, others to `OrderflowException`.
