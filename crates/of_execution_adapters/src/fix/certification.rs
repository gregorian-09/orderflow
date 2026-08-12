//! Deterministic FIX adapter certification infrastructure.

use super::{FixFrameTransport, FixTimeSample, FixTimeSource};
use of_execution::ExecutionCapabilities;
use of_fix::{
    parse_message, FixFieldView, FixTag, FixTranscriptCapture, FixTranscriptConfig,
    FixTranscriptDirection, FixTranscriptMetrics, FixTranscriptRecord, FixVersion,
};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

const CERTIFICATION_PARSE_FIELDS: usize = 192;
const DEFAULT_MAX_FAILURES: usize = 256;

/// Required deterministic FIX certification scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixCertificationScenario {
    /// Connect, Logon, Ready, Logout, and physical disconnect lifecycle.
    SessionLifecycle,
    /// Heartbeat, TestRequest, correlated Heartbeat, and liveness timeout.
    HeartbeatLiveness,
    /// Future inbound sequence detection and ordered gap recovery.
    SequenceGapRecovery,
    /// Peer ResendRequest handling with replay and SequenceReset gap fills.
    PeerResend,
    /// Possible-duplicate execution report suppression and identity behavior.
    DuplicateExecutionReport,
    /// New, partial-fill, and full-fill execution progression.
    PartialFill,
    /// Cancel acknowledgement/rejection and cancel-versus-fill races.
    CancelRace,
    /// Replace acknowledgement/rejection and replace-versus-fill races.
    ReplaceRace,
    /// Abnormal disconnect, reconnect, sequence restore, and open-order recovery.
    DisconnectReconnect,
    /// Invalid framing, checksum, body length, required fields, and values.
    MalformedMessage,
    /// Session Reject `<3>` mapping and safe degradation.
    SessionReject,
    /// BusinessMessageReject `<j>` mapping and safe degradation.
    BusinessReject,
    /// Frame, queue, retained-gap, resend-work, and output backpressure bounds.
    BackpressureAndBounds,
}

impl FixCertificationScenario {
    /// Complete required scenario inventory in stable report order.
    pub const ALL: [Self; 13] = [
        Self::SessionLifecycle,
        Self::HeartbeatLiveness,
        Self::SequenceGapRecovery,
        Self::PeerResend,
        Self::DuplicateExecutionReport,
        Self::PartialFill,
        Self::CancelRace,
        Self::ReplaceRace,
        Self::DisconnectReconnect,
        Self::MalformedMessage,
        Self::SessionReject,
        Self::BusinessReject,
        Self::BackpressureAndBounds,
    ];

    /// Stable scenario identifier for reports and CI output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionLifecycle => "session_lifecycle",
            Self::HeartbeatLiveness => "heartbeat_liveness",
            Self::SequenceGapRecovery => "sequence_gap_recovery",
            Self::PeerResend => "peer_resend",
            Self::DuplicateExecutionReport => "duplicate_execution_report",
            Self::PartialFill => "partial_fill",
            Self::CancelRace => "cancel_race",
            Self::ReplaceRace => "replace_race",
            Self::DisconnectReconnect => "disconnect_reconnect",
            Self::MalformedMessage => "malformed_message",
            Self::SessionReject => "session_reject",
            Self::BusinessReject => "business_reject",
            Self::BackpressureAndBounds => "backpressure_and_bounds",
        }
    }
}

/// FIX application capability that certification can require and exercise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixCertificationCapability {
    /// Market order submission.
    Market,
    /// Limit order submission.
    Limit,
    /// Stop order submission.
    Stop,
    /// Stop-limit order submission.
    StopLimit,
    /// Day time-in-force.
    Day,
    /// Good-till-cancel time-in-force.
    GoodTillCancel,
    /// Immediate-or-cancel time-in-force.
    ImmediateOrCancel,
    /// Fill-or-kill time-in-force.
    FillOrKill,
    /// Good-till-date time-in-force.
    GoodTillDate,
    /// Cancel/replace support.
    Amend,
    /// Native preservation of caller client-order ids.
    NativeClientOrderId,
}

impl FixCertificationCapability {
    /// Complete capability inventory in stable report order.
    pub const ALL: [Self; 11] = [
        Self::Market,
        Self::Limit,
        Self::Stop,
        Self::StopLimit,
        Self::Day,
        Self::GoodTillCancel,
        Self::ImmediateOrCancel,
        Self::FillOrKill,
        Self::GoodTillDate,
        Self::Amend,
        Self::NativeClientOrderId,
    ];

    const fn bit(self) -> u16 {
        match self {
            Self::Market => 1 << 0,
            Self::Limit => 1 << 1,
            Self::Stop => 1 << 2,
            Self::StopLimit => 1 << 3,
            Self::Day => 1 << 4,
            Self::GoodTillCancel => 1 << 5,
            Self::ImmediateOrCancel => 1 << 6,
            Self::FillOrKill => 1 << 7,
            Self::GoodTillDate => 1 << 8,
            Self::Amend => 1 << 9,
            Self::NativeClientOrderId => 1 << 10,
        }
    }

    /// Stable capability identifier for reports and CI output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Market => "market",
            Self::Limit => "limit",
            Self::Stop => "stop",
            Self::StopLimit => "stop_limit",
            Self::Day => "tif_day",
            Self::GoodTillCancel => "tif_gtc",
            Self::ImmediateOrCancel => "tif_ioc",
            Self::FillOrKill => "tif_fok",
            Self::GoodTillDate => "tif_gtd",
            Self::Amend => "amend",
            Self::NativeClientOrderId => "native_client_order_id",
        }
    }

    fn advertised(self, capabilities: ExecutionCapabilities) -> bool {
        match self {
            Self::Market => capabilities.market,
            Self::Limit => capabilities.limit,
            Self::Stop => capabilities.stop,
            Self::StopLimit => capabilities.stop_limit,
            Self::Day => capabilities.tif_day,
            Self::GoodTillCancel => capabilities.tif_gtc,
            Self::ImmediateOrCancel => capabilities.tif_ioc,
            Self::FillOrKill => capabilities.tif_fok,
            Self::GoodTillDate => capabilities.tif_gtd,
            Self::Amend => capabilities.amend,
            Self::NativeClientOrderId => capabilities.native_client_order_id,
        }
    }
}

/// Certification assertion failure category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixCertificationFailureKind {
    /// An expected frame was absent.
    MissingFrame,
    /// More frames were observed than the exact transcript allowed.
    UnexpectedFrame,
    /// Frame direction did not match.
    Direction,
    /// Message type did not match.
    MessageType,
    /// Message sequence number did not match.
    Sequence,
    /// A required FIX tag was absent.
    MissingTag,
    /// A FIX tag value did not match.
    TagValue,
    /// A frame could not be parsed and validated.
    MalformedFrame,
    /// Adapter behavior or returned event did not match the scenario.
    Behavior,
    /// A configured capacity was not enforced.
    BoundNotEnforced,
    /// A required advertised capability was missing.
    CapabilityNotAdvertised,
    /// An advertised capability was not exercised.
    CapabilityNotExercised,
    /// Required latency evidence was absent.
    LatencyEvidenceMissing,
    /// Required allocation evidence was absent.
    AllocationEvidenceMissing,
    /// Observed latency exceeded the configured certification threshold.
    LatencyLimitExceeded,
    /// Observed allocation count or bytes exceeded a configured threshold.
    AllocationLimitExceeded,
    /// Transcript records/raw bytes were dropped or evicted.
    TranscriptIncomplete,
    /// Harness failure supplied by a profile-specific scenario.
    Custom,
}

/// One bounded certification failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixCertificationFailure {
    scenario: Option<FixCertificationScenario>,
    kind: FixCertificationFailureKind,
    detail: String,
}

impl FixCertificationFailure {
    /// Creates one failure with bounded diagnostic text.
    pub fn new(
        scenario: Option<FixCertificationScenario>,
        kind: FixCertificationFailureKind,
        detail: impl Into<String>,
    ) -> Self {
        let mut detail = detail.into();
        detail.truncate(512);
        Self {
            scenario,
            kind,
            detail,
        }
    }

    /// Returns the scenario, or `None` for suite-level failure.
    pub const fn scenario(&self) -> Option<FixCertificationScenario> {
        self.scenario
    }

    /// Returns the failure category.
    pub const fn kind(&self) -> FixCertificationFailureKind {
        self.kind
    }

    /// Returns bounded diagnostic detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// One expected FIX tag/value pair in a transcript frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixExpectedField {
    tag: u32,
    value: Vec<u8>,
}

impl FixExpectedField {
    /// Creates an exact field expectation.
    pub fn new(tag: u32, value: impl Into<Vec<u8>>) -> Self {
        Self {
            tag,
            value: value.into(),
        }
    }

    /// Returns the expected tag.
    pub const fn tag(&self) -> u32 {
        self.tag
    }

    /// Returns the expected wire value.
    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

/// Exact metadata and field expectations for one transcript frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixFrameExpectation {
    direction: FixTranscriptDirection,
    message_type: Vec<u8>,
    sequence: Option<u64>,
    fields: Vec<FixExpectedField>,
}

impl FixFrameExpectation {
    /// Creates an expectation for a direction and message type.
    pub fn new(direction: FixTranscriptDirection, message_type: impl Into<Vec<u8>>) -> Self {
        Self {
            direction,
            message_type: message_type.into(),
            sequence: None,
            fields: Vec::new(),
        }
    }

    /// Requires an exact `MsgSeqNum(34)`.
    pub const fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = Some(sequence);
        self
    }

    /// Adds an exact required field.
    pub fn with_field(mut self, field: FixExpectedField) -> Self {
        self.fields.push(field);
        self
    }

    /// Returns expected direction.
    pub const fn direction(&self) -> FixTranscriptDirection {
        self.direction
    }

    /// Returns expected message type bytes.
    pub fn message_type(&self) -> &[u8] {
        &self.message_type
    }

    /// Returns expected sequence, when constrained.
    pub const fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    /// Returns exact required fields.
    pub fn fields(&self) -> &[FixExpectedField] {
        &self.fields
    }
}

/// Aggregate latency evidence collected outside the adapter hot path.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FixCertificationLatencyEvidence {
    samples: u64,
    total_ns: u128,
    min_ns: u64,
    max_ns: u64,
}

impl FixCertificationLatencyEvidence {
    /// Records one non-negative latency sample.
    pub fn record(&mut self, latency_ns: u64) {
        if self.samples == 0 {
            self.min_ns = latency_ns;
        } else {
            self.min_ns = self.min_ns.min(latency_ns);
        }
        self.max_ns = self.max_ns.max(latency_ns);
        self.total_ns = self.total_ns.saturating_add(u128::from(latency_ns));
        self.samples = self.samples.saturating_add(1);
    }

    /// Returns sample count.
    pub const fn samples(self) -> u64 {
        self.samples
    }

    /// Returns minimum observed latency.
    pub const fn min_ns(self) -> u64 {
        self.min_ns
    }

    /// Returns maximum observed latency.
    pub const fn max_ns(self) -> u64 {
        self.max_ns
    }

    /// Returns integer average latency.
    pub fn average_ns(self) -> u64 {
        if self.samples == 0 {
            return 0;
        }
        let average = self.total_ns / u128::from(self.samples);
        u64::try_from(average).unwrap_or(u64::MAX)
    }
}

/// Allocation evidence measured by a host-provided allocator/profiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixCertificationAllocationEvidence {
    measured_messages: u64,
    hot_path_allocations: u64,
    hot_path_allocated_bytes: u64,
}

impl FixCertificationAllocationEvidence {
    /// Creates externally measured allocation evidence.
    pub const fn new(
        measured_messages: u64,
        hot_path_allocations: u64,
        hot_path_allocated_bytes: u64,
    ) -> Self {
        Self {
            measured_messages,
            hot_path_allocations,
            hot_path_allocated_bytes,
        }
    }

    /// Returns measured message count.
    pub const fn measured_messages(self) -> u64 {
        self.measured_messages
    }

    /// Returns measured hot-path allocation count.
    pub const fn hot_path_allocations(self) -> u64 {
        self.hot_path_allocations
    }

    /// Returns measured allocated bytes.
    pub const fn hot_path_allocated_bytes(self) -> u64 {
        self.hot_path_allocated_bytes
    }
}

/// Bounded certification harness configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixCertificationConfig {
    required_scenarios: Vec<FixCertificationScenario>,
    required_capabilities: u16,
    max_failures: usize,
    require_latency_evidence: bool,
    require_allocation_evidence: bool,
    require_complete_transcript: bool,
    max_observed_latency_ns: Option<u64>,
    max_hot_path_allocations: Option<u64>,
    max_hot_path_allocated_bytes: Option<u64>,
}

impl Default for FixCertificationConfig {
    fn default() -> Self {
        Self {
            required_scenarios: FixCertificationScenario::ALL.to_vec(),
            required_capabilities: 0,
            max_failures: DEFAULT_MAX_FAILURES,
            require_latency_evidence: true,
            require_allocation_evidence: true,
            require_complete_transcript: true,
            max_observed_latency_ns: None,
            max_hot_path_allocations: None,
            max_hot_path_allocated_bytes: None,
        }
    }
}

impl FixCertificationConfig {
    /// Creates a full-suite configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces required scenarios, sorting and deduplicating them.
    pub fn with_required_scenarios(
        mut self,
        scenarios: impl IntoIterator<Item = FixCertificationScenario>,
    ) -> Self {
        self.required_scenarios = scenarios.into_iter().collect();
        self.required_scenarios.sort_unstable();
        self.required_scenarios.dedup();
        self
    }

    /// Requires evidence for an advertised application capability.
    pub const fn require_capability(mut self, capability: FixCertificationCapability) -> Self {
        self.required_capabilities |= capability.bit();
        self
    }

    /// Sets the retained failure bound.
    pub const fn with_max_failures(mut self, max_failures: usize) -> Self {
        self.max_failures = max_failures;
        self
    }

    /// Configures whether at least one latency sample is mandatory.
    pub const fn with_latency_evidence_required(mut self, required: bool) -> Self {
        self.require_latency_evidence = required;
        self
    }

    /// Configures whether allocation evidence is mandatory.
    pub const fn with_allocation_evidence_required(mut self, required: bool) -> Self {
        self.require_allocation_evidence = required;
        self
    }

    /// Configures whether dropped/evicted transcript evidence fails the suite.
    pub const fn with_complete_transcript_required(mut self, required: bool) -> Self {
        self.require_complete_transcript = required;
        self
    }

    /// Sets a maximum observed adapter latency threshold.
    pub const fn with_max_observed_latency_ns(mut self, maximum: Option<u64>) -> Self {
        self.max_observed_latency_ns = maximum;
        self
    }

    /// Sets the maximum total allocations accepted in measured hot-path work.
    pub const fn with_max_hot_path_allocations(mut self, maximum: Option<u64>) -> Self {
        self.max_hot_path_allocations = maximum;
        self
    }

    /// Sets the maximum total allocated bytes accepted in measured hot-path work.
    pub const fn with_max_hot_path_allocated_bytes(mut self, maximum: Option<u64>) -> Self {
        self.max_hot_path_allocated_bytes = maximum;
        self
    }

    /// Returns required scenarios.
    pub fn required_scenarios(&self) -> &[FixCertificationScenario] {
        &self.required_scenarios
    }

    /// Returns the retained failure-detail bound.
    pub const fn max_failures(&self) -> usize {
        self.max_failures
    }
}

/// Result for one certification scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixCertificationScenarioResult {
    scenario: FixCertificationScenario,
    passed: bool,
    assertions: u64,
    transcript_frames: u64,
    failure_count: u64,
}

impl FixCertificationScenarioResult {
    /// Returns scenario identity.
    pub const fn scenario(&self) -> FixCertificationScenario {
        self.scenario
    }

    /// Returns whether every scenario assertion passed.
    pub const fn passed(&self) -> bool {
        self.passed
    }

    /// Returns assertion count.
    pub const fn assertions(&self) -> u64 {
        self.assertions
    }

    /// Returns frames covered by transcript assertions.
    pub const fn transcript_frames(&self) -> u64 {
        self.transcript_frames
    }

    /// Returns failure count, including failures beyond retained detail bounds.
    pub const fn failure_count(&self) -> u64 {
        self.failure_count
    }
}

/// Immutable FIX certification conformance report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixCertificationReport {
    profile_name: String,
    version: FixVersion,
    passed: bool,
    scenarios: Vec<FixCertificationScenarioResult>,
    missing_scenarios: Vec<FixCertificationScenario>,
    failures: Vec<FixCertificationFailure>,
    dropped_failure_details: u64,
    unsupported_capabilities: Vec<FixCertificationCapability>,
    unexercised_capabilities: Vec<FixCertificationCapability>,
    latency: FixCertificationLatencyEvidence,
    allocation: Option<FixCertificationAllocationEvidence>,
    transcript: FixTranscriptMetrics,
}

impl FixCertificationReport {
    /// Returns profile/report name.
    pub fn profile_name(&self) -> &str {
        &self.profile_name
    }

    /// Returns FIX version under test.
    pub const fn version(&self) -> FixVersion {
        self.version
    }

    /// Returns true only when scenarios, capabilities, and required evidence pass.
    pub const fn passed(&self) -> bool {
        self.passed
    }

    /// Returns scenario results in stable order.
    pub fn scenarios(&self) -> &[FixCertificationScenarioResult] {
        &self.scenarios
    }

    /// Returns required scenarios that were never recorded.
    pub fn missing_scenarios(&self) -> &[FixCertificationScenario] {
        &self.missing_scenarios
    }

    /// Returns bounded failure details.
    pub fn failures(&self) -> &[FixCertificationFailure] {
        &self.failures
    }

    /// Returns failures omitted after the configured detail bound.
    pub const fn dropped_failure_details(&self) -> u64 {
        self.dropped_failure_details
    }

    /// Returns required capabilities not advertised by the profile.
    pub fn unsupported_capabilities(&self) -> &[FixCertificationCapability] {
        &self.unsupported_capabilities
    }

    /// Returns required advertised capabilities not exercised by the suite.
    pub fn unexercised_capabilities(&self) -> &[FixCertificationCapability] {
        &self.unexercised_capabilities
    }

    /// Returns latency evidence.
    pub const fn latency(&self) -> FixCertificationLatencyEvidence {
        self.latency
    }

    /// Returns external allocation evidence.
    pub const fn allocation(&self) -> Option<FixCertificationAllocationEvidence> {
        self.allocation
    }

    /// Returns transcript archive metrics/hash.
    pub const fn transcript(&self) -> FixTranscriptMetrics {
        self.transcript
    }
}

/// Stateful bounded certification report builder.
#[derive(Debug, Clone)]
pub struct FixCertificationHarness {
    profile_name: String,
    version: FixVersion,
    config: FixCertificationConfig,
    scenarios: Vec<FixCertificationScenarioResult>,
    failures: Vec<FixCertificationFailure>,
    dropped_failure_details: u64,
    exercised_capabilities: u16,
    latency: FixCertificationLatencyEvidence,
    allocation: Option<FixCertificationAllocationEvidence>,
}

impl FixCertificationHarness {
    /// Creates an empty harness.
    ///
    /// # Errors
    ///
    /// Returns [`FixCertificationHarnessError`] for an empty profile name, no
    /// required scenarios, or a zero failure bound.
    pub fn new(
        profile_name: impl Into<String>,
        version: FixVersion,
        config: FixCertificationConfig,
    ) -> Result<Self, FixCertificationHarnessError> {
        let profile_name = profile_name.into();
        if profile_name.trim().is_empty() {
            return Err(FixCertificationHarnessError::EmptyProfileName);
        }
        if config.required_scenarios.is_empty() {
            return Err(FixCertificationHarnessError::NoRequiredScenarios);
        }
        if config.max_failures == 0 {
            return Err(FixCertificationHarnessError::ZeroFailureCapacity);
        }
        Ok(Self {
            profile_name,
            version,
            failures: Vec::with_capacity(config.max_failures.min(32)),
            scenarios: Vec::with_capacity(config.required_scenarios.len()),
            config,
            dropped_failure_details: 0,
            exercised_capabilities: 0,
            latency: FixCertificationLatencyEvidence::default(),
            allocation: None,
        })
    }

    /// Records a manually evaluated scenario.
    ///
    /// # Errors
    ///
    /// Returns [`FixCertificationHarnessError::DuplicateScenario`] when the
    /// scenario already has a result.
    pub fn record_scenario(
        &mut self,
        scenario: FixCertificationScenario,
        assertions: u64,
        failures: impl IntoIterator<Item = FixCertificationFailure>,
    ) -> Result<(), FixCertificationHarnessError> {
        self.ensure_new_scenario(scenario)?;
        let mut failure_count = 0u64;
        for mut failure in failures {
            failure.scenario = Some(scenario);
            failure_count = failure_count.saturating_add(1);
            self.retain_failure(failure);
        }
        self.scenarios.push(FixCertificationScenarioResult {
            scenario,
            passed: assertions > 0 && failure_count == 0,
            assertions,
            transcript_frames: 0,
            failure_count,
        });
        Ok(())
    }

    /// Compares a transcript slice with exact ordered frame expectations.
    ///
    /// When `exact_length` is true, extra records fail the scenario. Parsing
    /// uses caller-local fixed scratch storage and never mutates the archive.
    ///
    /// # Errors
    ///
    /// Returns [`FixCertificationHarnessError::DuplicateScenario`] when the
    /// scenario already has a result.
    pub fn assert_transcript(
        &mut self,
        scenario: FixCertificationScenario,
        records: &[&FixTranscriptRecord],
        expectations: &[FixFrameExpectation],
        exact_length: bool,
    ) -> Result<(), FixCertificationHarnessError> {
        self.ensure_new_scenario(scenario)?;
        let initial_failures = self.total_failure_count();
        for (index, expectation) in expectations.iter().enumerate() {
            let Some(record) = records.get(index) else {
                self.retain_failure(FixCertificationFailure::new(
                    Some(scenario),
                    FixCertificationFailureKind::MissingFrame,
                    format!("missing transcript frame at index {index}"),
                ));
                continue;
            };
            self.assert_frame(scenario, index, record, expectation);
        }
        if exact_length && records.len() > expectations.len() {
            self.retain_failure(FixCertificationFailure::new(
                Some(scenario),
                FixCertificationFailureKind::UnexpectedFrame,
                format!(
                    "observed {} transcript frames, expected {}",
                    records.len(),
                    expectations.len()
                ),
            ));
        }
        let failures = self.total_failure_count().saturating_sub(initial_failures);
        self.scenarios.push(FixCertificationScenarioResult {
            scenario,
            passed: !expectations.is_empty() && failures == 0,
            assertions: expectations.len() as u64,
            transcript_frames: records.len() as u64,
            failure_count: failures,
        });
        Ok(())
    }

    /// Marks one advertised capability as exercised by a passing scenario.
    pub const fn exercise_capability(&mut self, capability: FixCertificationCapability) {
        self.exercised_capabilities |= capability.bit();
    }

    /// Records one externally measured adapter latency sample.
    pub fn record_latency_ns(&mut self, latency_ns: u64) {
        self.latency.record(latency_ns);
    }

    /// Installs host-profiler allocation evidence.
    pub const fn set_allocation_evidence(&mut self, evidence: FixCertificationAllocationEvidence) {
        self.allocation = Some(evidence);
    }

    /// Builds an immutable conformance report.
    pub fn report(
        &self,
        capabilities: ExecutionCapabilities,
        transcript: FixTranscriptMetrics,
    ) -> FixCertificationReport {
        let mut scenarios = self.scenarios.clone();
        scenarios.sort_unstable_by_key(|result| result.scenario);
        let missing_scenarios = self
            .config
            .required_scenarios
            .iter()
            .copied()
            .filter(|required| !scenarios.iter().any(|result| result.scenario == *required))
            .collect::<Vec<_>>();
        let mut unsupported_capabilities = Vec::new();
        let mut unexercised_capabilities = Vec::new();
        for capability in FixCertificationCapability::ALL {
            if self.config.required_capabilities & capability.bit() == 0 {
                continue;
            }
            if !capability.advertised(capabilities) {
                unsupported_capabilities.push(capability);
            } else if self.exercised_capabilities & capability.bit() == 0 {
                unexercised_capabilities.push(capability);
            }
        }
        let scenario_failure = scenarios.iter().any(|result| !result.passed);
        let allocation_missing = self.config.require_allocation_evidence
            && match self.allocation {
                Some(evidence) => evidence.measured_messages == 0,
                None => true,
            };
        let latency_exceeded = self
            .config
            .max_observed_latency_ns
            .is_some_and(|maximum| self.latency.max_ns > maximum);
        let allocation_exceeded = self.allocation.is_some_and(|evidence| {
            self.config
                .max_hot_path_allocations
                .is_some_and(|maximum| evidence.hot_path_allocations > maximum)
                || self
                    .config
                    .max_hot_path_allocated_bytes
                    .is_some_and(|maximum| evidence.hot_path_allocated_bytes > maximum)
        });
        let transcript_incomplete = self.config.require_complete_transcript
            && (transcript.dropped_records() > 0
                || transcript.dropped_raw_bytes() > 0
                || transcript.evicted_records() > 0
                || transcript.evicted_raw_bytes() > 0);
        let evidence_failure = (self.config.require_latency_evidence && self.latency.samples == 0)
            || allocation_missing
            || latency_exceeded
            || allocation_exceeded
            || transcript_incomplete;
        let passed = !scenario_failure
            && missing_scenarios.is_empty()
            && self.failures.is_empty()
            && self.dropped_failure_details == 0
            && unsupported_capabilities.is_empty()
            && unexercised_capabilities.is_empty()
            && !evidence_failure;
        let mut failures = self.failures.clone();
        let mut dropped_failure_details = self.dropped_failure_details;
        if self.config.require_latency_evidence && self.latency.samples == 0 {
            push_report_failure(
                &mut failures,
                &mut dropped_failure_details,
                self.config.max_failures,
                FixCertificationFailure::new(
                    None,
                    FixCertificationFailureKind::LatencyEvidenceMissing,
                    "latency evidence is required",
                ),
            );
        }
        if allocation_missing {
            push_report_failure(
                &mut failures,
                &mut dropped_failure_details,
                self.config.max_failures,
                FixCertificationFailure::new(
                    None,
                    FixCertificationFailureKind::AllocationEvidenceMissing,
                    "allocation evidence with at least one measured message is required",
                ),
            );
        }
        if latency_exceeded {
            push_report_failure(
                &mut failures,
                &mut dropped_failure_details,
                self.config.max_failures,
                FixCertificationFailure::new(
                    None,
                    FixCertificationFailureKind::LatencyLimitExceeded,
                    format!(
                        "maximum observed latency {} ns exceeds configured {} ns",
                        self.latency.max_ns,
                        self.config.max_observed_latency_ns.unwrap_or(u64::MAX)
                    ),
                ),
            );
        }
        if allocation_exceeded {
            push_report_failure(
                &mut failures,
                &mut dropped_failure_details,
                self.config.max_failures,
                FixCertificationFailure::new(
                    None,
                    FixCertificationFailureKind::AllocationLimitExceeded,
                    "hot-path allocation evidence exceeds configured limits",
                ),
            );
        }
        if transcript_incomplete {
            push_report_failure(
                &mut failures,
                &mut dropped_failure_details,
                self.config.max_failures,
                FixCertificationFailure::new(
                    None,
                    FixCertificationFailureKind::TranscriptIncomplete,
                    "certification transcript dropped or evicted evidence",
                ),
            );
        }
        FixCertificationReport {
            profile_name: self.profile_name.clone(),
            version: self.version,
            passed,
            scenarios,
            missing_scenarios,
            failures,
            dropped_failure_details,
            unsupported_capabilities,
            unexercised_capabilities,
            latency: self.latency,
            allocation: self.allocation,
            transcript,
        }
    }

    fn ensure_new_scenario(
        &self,
        scenario: FixCertificationScenario,
    ) -> Result<(), FixCertificationHarnessError> {
        if self
            .scenarios
            .iter()
            .any(|result| result.scenario == scenario)
        {
            return Err(FixCertificationHarnessError::DuplicateScenario(scenario));
        }
        Ok(())
    }

    fn assert_frame(
        &mut self,
        scenario: FixCertificationScenario,
        index: usize,
        record: &FixTranscriptRecord,
        expectation: &FixFrameExpectation,
    ) {
        if record.direction() != expectation.direction {
            self.retain_failure(FixCertificationFailure::new(
                Some(scenario),
                FixCertificationFailureKind::Direction,
                format!("frame {index} direction mismatch"),
            ));
        }
        if record.msg_type() != expectation.message_type {
            self.retain_failure(FixCertificationFailure::new(
                Some(scenario),
                FixCertificationFailureKind::MessageType,
                format!("frame {index} message type mismatch"),
            ));
        }
        if expectation
            .sequence
            .is_some_and(|expected| record.seq_no() != Some(expected))
        {
            self.retain_failure(FixCertificationFailure::new(
                Some(scenario),
                FixCertificationFailureKind::Sequence,
                format!("frame {index} sequence mismatch"),
            ));
        }
        if expectation.fields.is_empty() {
            return;
        }
        if !record.raw_retained() {
            self.retain_failure(FixCertificationFailure::new(
                Some(scenario),
                FixCertificationFailureKind::MissingFrame,
                format!("frame {index} raw bytes were not retained"),
            ));
            return;
        }
        let mut scratch = [FixFieldView::empty(); CERTIFICATION_PARSE_FIELDS];
        let Ok(message) = parse_message(record.raw(), &mut scratch) else {
            self.retain_failure(FixCertificationFailure::new(
                Some(scenario),
                FixCertificationFailureKind::MalformedFrame,
                format!("frame {index} failed strict FIX parsing"),
            ));
            return;
        };
        for expected in &expectation.fields {
            match message.get(FixTag(expected.tag)) {
                None => self.retain_failure(FixCertificationFailure::new(
                    Some(scenario),
                    FixCertificationFailureKind::MissingTag,
                    format!("frame {index} missing tag {}", expected.tag),
                )),
                Some(actual) if actual != expected.value.as_slice() => {
                    self.retain_failure(FixCertificationFailure::new(
                        Some(scenario),
                        FixCertificationFailureKind::TagValue,
                        format!("frame {index} tag {} value mismatch", expected.tag),
                    ));
                }
                Some(_) => {}
            }
        }
    }

    fn retain_failure(&mut self, failure: FixCertificationFailure) {
        if self.failures.len() < self.config.max_failures {
            self.failures.push(failure);
        } else {
            self.dropped_failure_details = self.dropped_failure_details.saturating_add(1);
        }
    }

    fn total_failure_count(&self) -> u64 {
        (self.failures.len() as u64).saturating_add(self.dropped_failure_details)
    }
}

/// Certification harness configuration/state error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixCertificationHarnessError {
    /// Profile name was empty.
    EmptyProfileName,
    /// Required scenario inventory was empty.
    NoRequiredScenarios,
    /// Failure detail capacity was zero.
    ZeroFailureCapacity,
    /// A scenario was recorded more than once.
    DuplicateScenario(FixCertificationScenario),
}

impl fmt::Display for FixCertificationHarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProfileName => f.write_str("FIX certification profile name is empty"),
            Self::NoRequiredScenarios => f.write_str("FIX certification has no required scenarios"),
            Self::ZeroFailureCapacity => f.write_str("FIX certification failure capacity is zero"),
            Self::DuplicateScenario(scenario) => {
                write!(
                    f,
                    "FIX certification scenario {} is duplicated",
                    scenario.as_str()
                )
            }
        }
    }
}

impl Error for FixCertificationHarnessError {}

/// Bounded scripted counterparty transport configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixScriptedTransportConfig {
    max_inbound_frames: usize,
    max_inbound_bytes: usize,
    max_outbound_frames: usize,
    max_outbound_bytes: usize,
    transcript: FixTranscriptConfig,
}

impl FixScriptedTransportConfig {
    /// Creates transport bounds.
    pub const fn new(
        max_inbound_frames: usize,
        max_inbound_bytes: usize,
        max_outbound_frames: usize,
        max_outbound_bytes: usize,
        transcript: FixTranscriptConfig,
    ) -> Self {
        Self {
            max_inbound_frames,
            max_inbound_bytes,
            max_outbound_frames,
            max_outbound_bytes,
            transcript,
        }
    }
}

impl Default for FixScriptedTransportConfig {
    fn default() -> Self {
        Self::new(
            1024,
            8 * 1024 * 1024,
            1024,
            8 * 1024 * 1024,
            FixTranscriptConfig::default(),
        )
    }
}

/// Injected scripted-transport failure point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixScriptedTransportFailure {
    /// Fail the next connect call.
    Connect,
    /// Fail the next send call.
    Send,
    /// Fail the next receive poll.
    Receive,
    /// Fail the next disconnect call.
    Disconnect,
}

/// Scripted transport error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixScriptedTransportError {
    /// Configuration has a zero capacity.
    ZeroCapacity,
    /// Operation requires an established transport.
    Disconnected,
    /// Inbound queue limits were exceeded.
    InboundCapacity,
    /// Outbound archive limits were exceeded.
    OutboundCapacity,
    /// Destination receive buffer is smaller than the next frame.
    ReceiveBufferTooSmall {
        /// Required frame bytes.
        required: usize,
        /// Supplied destination bytes.
        available: usize,
    },
    /// Requested deterministic failure was injected.
    Injected(FixScriptedTransportFailure),
    /// Transcript capture rejected frame metadata.
    Transcript,
}

impl fmt::Display for FixScriptedTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCapacity => f.write_str("scripted FIX transport capacity is zero"),
            Self::Disconnected => f.write_str("scripted FIX transport is disconnected"),
            Self::InboundCapacity => f.write_str("scripted FIX inbound capacity exceeded"),
            Self::OutboundCapacity => f.write_str("scripted FIX outbound capacity exceeded"),
            Self::ReceiveBufferTooSmall {
                required,
                available,
            } => write!(
                f,
                "scripted FIX receive frame requires {required} bytes, {available} available"
            ),
            Self::Injected(point) => write!(f, "scripted FIX transport failure at {point:?}"),
            Self::Transcript => f.write_str("scripted FIX transcript capture failed"),
        }
    }
}

impl Error for FixScriptedTransportError {}

/// Bounded deterministic counterparty transport for adapter certification.
#[derive(Debug, Clone)]
pub struct FixScriptedTransport {
    config: FixScriptedTransportConfig,
    inbound: VecDeque<Vec<u8>>,
    inbound_bytes: usize,
    outbound: Vec<Vec<u8>>,
    outbound_bytes: usize,
    transcript: FixTranscriptCapture,
    connected: bool,
    timestamp_ns: u64,
    failure: Option<FixScriptedTransportFailure>,
}

impl FixScriptedTransport {
    /// Creates an empty scripted transport.
    ///
    /// # Errors
    ///
    /// Returns [`FixScriptedTransportError::ZeroCapacity`] when any queue or
    /// byte bound is zero.
    pub fn new(config: FixScriptedTransportConfig) -> Result<Self, FixScriptedTransportError> {
        if config.max_inbound_frames == 0
            || config.max_inbound_bytes == 0
            || config.max_outbound_frames == 0
            || config.max_outbound_bytes == 0
        {
            return Err(FixScriptedTransportError::ZeroCapacity);
        }
        Ok(Self {
            inbound: VecDeque::with_capacity(config.max_inbound_frames.min(64)),
            outbound: Vec::with_capacity(config.max_outbound_frames.min(64)),
            transcript: FixTranscriptCapture::new(config.transcript),
            config,
            inbound_bytes: 0,
            outbound_bytes: 0,
            connected: false,
            timestamp_ns: 0,
            failure: None,
        })
    }

    /// Enqueues one complete inbound counterparty frame.
    ///
    /// # Errors
    ///
    /// Returns [`FixScriptedTransportError::InboundCapacity`] without mutation
    /// when frame or byte bounds would be exceeded.
    pub fn enqueue_inbound(
        &mut self,
        frame: impl Into<Vec<u8>>,
    ) -> Result<(), FixScriptedTransportError> {
        let frame = frame.into();
        if self.inbound.len() >= self.config.max_inbound_frames
            || self.inbound_bytes.saturating_add(frame.len()) > self.config.max_inbound_bytes
        {
            return Err(FixScriptedTransportError::InboundCapacity);
        }
        self.inbound_bytes = self.inbound_bytes.saturating_add(frame.len());
        self.inbound.push_back(frame);
        Ok(())
    }

    /// Injects one deterministic failure at the next matching operation.
    pub const fn fail_next(&mut self, failure: FixScriptedTransportFailure) {
        self.failure = Some(failure);
    }

    /// Advances transcript capture time.
    pub const fn set_timestamp_ns(&mut self, timestamp_ns: u64) {
        self.timestamp_ns = timestamp_ns;
    }

    /// Returns whether the physical test transport is connected.
    pub const fn is_connected(&self) -> bool {
        self.connected
    }

    /// Returns queued inbound frame count.
    pub fn inbound_frames(&self) -> usize {
        self.inbound.len()
    }

    /// Returns retained outbound frames in send order.
    pub fn outbound_frames(&self) -> &[Vec<u8>] {
        &self.outbound
    }

    /// Returns transcript archive.
    pub const fn transcript(&self) -> &FixTranscriptCapture {
        &self.transcript
    }

    /// Clears sent frame retention while preserving transcript evidence.
    pub fn clear_outbound_frames(&mut self) {
        self.outbound.clear();
        self.outbound_bytes = 0;
    }

    fn take_failure(
        &mut self,
        point: FixScriptedTransportFailure,
    ) -> Result<(), FixScriptedTransportError> {
        if self.failure == Some(point) {
            self.failure = None;
            return Err(FixScriptedTransportError::Injected(point));
        }
        Ok(())
    }

    fn capture(
        &mut self,
        direction: FixTranscriptDirection,
        raw: &[u8],
    ) -> Result<(), FixScriptedTransportError> {
        let mut scratch = [FixFieldView::empty(); CERTIFICATION_PARSE_FIELDS];
        if let Ok(message) = parse_message(raw, &mut scratch) {
            self.transcript
                .record_message(direction, self.timestamp_ns, &message)
                .map_err(|_| FixScriptedTransportError::Transcript)?;
        } else {
            self.transcript
                .record_frame(direction, self.timestamp_ns, None, &[], raw)
                .map_err(|_| FixScriptedTransportError::Transcript)?;
        }
        Ok(())
    }
}

impl FixFrameTransport for FixScriptedTransport {
    type Error = FixScriptedTransportError;

    fn connect(&mut self) -> Result<(), Self::Error> {
        self.take_failure(FixScriptedTransportFailure::Connect)?;
        self.connected = true;
        Ok(())
    }

    fn send(&mut self, frame: &[u8]) -> Result<(), Self::Error> {
        if !self.connected {
            return Err(FixScriptedTransportError::Disconnected);
        }
        self.take_failure(FixScriptedTransportFailure::Send)?;
        if self.outbound.len() >= self.config.max_outbound_frames
            || self.outbound_bytes.saturating_add(frame.len()) > self.config.max_outbound_bytes
        {
            return Err(FixScriptedTransportError::OutboundCapacity);
        }
        self.capture(FixTranscriptDirection::Outbound, frame)?;
        self.outbound_bytes = self.outbound_bytes.saturating_add(frame.len());
        self.outbound.push(frame.to_vec());
        Ok(())
    }

    fn poll_receive(&mut self, out: &mut [u8]) -> Result<super::FixTransportPoll, Self::Error> {
        if !self.connected {
            return Err(FixScriptedTransportError::Disconnected);
        }
        self.take_failure(FixScriptedTransportFailure::Receive)?;
        let Some(frame) = self.inbound.front() else {
            return Ok(super::FixTransportPoll::Idle);
        };
        if frame.len() > out.len() {
            return Err(FixScriptedTransportError::ReceiveBufferTooSmall {
                required: frame.len(),
                available: out.len(),
            });
        }
        let frame = self.inbound.pop_front().expect("front checked");
        self.inbound_bytes = self.inbound_bytes.saturating_sub(frame.len());
        out[..frame.len()].copy_from_slice(&frame);
        self.capture(FixTranscriptDirection::Inbound, &frame)?;
        Ok(super::FixTransportPoll::Frame { len: frame.len() })
    }

    fn disconnect(&mut self) -> Result<(), Self::Error> {
        self.take_failure(FixScriptedTransportFailure::Disconnect)?;
        self.connected = false;
        Ok(())
    }
}

/// Deterministic coherent clock for FIX adapter certification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixCertificationClock {
    monotonic_ns: u64,
    unix_ns: u64,
    step_ns: u64,
    sending_time: Vec<u8>,
}

impl FixCertificationClock {
    /// Creates a deterministic clock and fixed valid FIX UTC timestamp.
    pub fn new(
        monotonic_ns: u64,
        unix_ns: u64,
        step_ns: u64,
        sending_time: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            monotonic_ns,
            unix_ns,
            step_ns,
            sending_time: sending_time.into(),
        }
    }

    /// Advances both clocks explicitly.
    pub fn advance_ns(&mut self, delta_ns: u64) {
        self.monotonic_ns = self.monotonic_ns.saturating_add(delta_ns);
        self.unix_ns = self.unix_ns.saturating_add(delta_ns);
    }

    /// Replaces FIX `SendingTime(52)` bytes.
    pub fn set_sending_time(&mut self, sending_time: impl Into<Vec<u8>>) {
        self.sending_time = sending_time.into();
    }
}

/// Deterministic certification clock error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixCertificationClockError;

impl fmt::Display for FixCertificationClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FIX certification timestamp does not fit destination buffer")
    }
}

impl Error for FixCertificationClockError {}

impl FixTimeSource for FixCertificationClock {
    type Error = FixCertificationClockError;

    fn sample(&mut self, out: &mut [u8]) -> Result<FixTimeSample, Self::Error> {
        if self.sending_time.is_empty() || self.sending_time.len() > out.len() {
            return Err(FixCertificationClockError);
        }
        out[..self.sending_time.len()].copy_from_slice(&self.sending_time);
        let sample = FixTimeSample::new(self.monotonic_ns, self.unix_ns, self.sending_time.len());
        self.advance_ns(self.step_ns);
        Ok(sample)
    }
}

fn push_report_failure(
    failures: &mut Vec<FixCertificationFailure>,
    dropped: &mut u64,
    capacity: usize,
    failure: FixCertificationFailure,
) {
    if failures.len() < capacity {
        failures.push(failure);
    } else {
        *dropped = dropped.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fix::{
        FixLiveAdapterConfig, FixReportParseConfig, FixRequestEncodeConfig, FixSessionConfig,
        FixTransportExecutionAdapter, StandardFixExecutionProfile,
    };
    use of_execution::{ExecutionAdapter, ExecutionEventBuffer};
    use of_execution_core::{AccountId, RouteId, VenueId};
    use of_fix::{encode_logon, FixSessionHeader};

    fn capabilities() -> ExecutionCapabilities {
        ExecutionCapabilities::simulated()
    }

    fn logon(sequence: u64) -> Vec<u8> {
        let mut raw = Vec::new();
        encode_logon(
            &mut raw,
            FixVersion::Fix44,
            FixSessionHeader::new(
                b"TARGET",
                b"SENDER",
                sequence,
                b"20260812-12:34:56.123456789",
            ),
            30,
            false,
        )
        .expect("logon");
        raw
    }

    fn live_adapter() -> FixTransportExecutionAdapter<
        FixScriptedTransport,
        FixCertificationClock,
        StandardFixExecutionProfile,
    > {
        let session = FixSessionConfig::new("FIX.4.4", "SENDER", "TARGET", 30).expect("session");
        let config = FixLiveAdapterConfig::new(session).expect("config");
        let transport =
            FixScriptedTransport::new(FixScriptedTransportConfig::default()).expect("transport");
        let clock = FixCertificationClock::new(
            1,
            1_786_538_096_123_456_789,
            1,
            b"20260812-12:34:56.123456789".to_vec(),
        );
        let profile = StandardFixExecutionProfile::new(
            FixVersion::Fix44,
            FixRequestEncodeConfig::new()
                .with_quantity_scale(1)
                .with_price_scale(100),
            FixReportParseConfig::new(
                AccountId::new("ACC1").expect("account"),
                RouteId::new("FIX-A").expect("route"),
                VenueId::new("XNAS").expect("venue"),
            )
            .with_quantity_scale(1)
            .with_price_scale(100),
        );
        FixTransportExecutionAdapter::new(config, transport, clock, profile).expect("adapter")
    }

    #[test]
    fn scripted_transport_drives_real_adapter_and_exact_transcript() {
        let mut adapter = live_adapter();
        let mut events = ExecutionEventBuffer::with_capacity(4);
        adapter.connect().expect("connect");
        adapter
            .transport_mut()
            .enqueue_inbound(logon(1))
            .expect("enqueue");
        adapter.poll(&mut events).expect("poll");

        let records = adapter
            .transport()
            .transcript()
            .records()
            .collect::<Vec<_>>();
        let mut harness = FixCertificationHarness::new(
            "standard-fix44",
            FixVersion::Fix44,
            FixCertificationConfig::new()
                .with_required_scenarios([FixCertificationScenario::SessionLifecycle])
                .with_latency_evidence_required(false)
                .with_allocation_evidence_required(false),
        )
        .expect("harness");
        harness
            .assert_transcript(
                FixCertificationScenario::SessionLifecycle,
                &records,
                &[
                    FixFrameExpectation::new(FixTranscriptDirection::Outbound, b"A".to_vec())
                        .with_sequence(1)
                        .with_field(FixExpectedField::new(49, b"SENDER".to_vec())),
                    FixFrameExpectation::new(FixTranscriptDirection::Inbound, b"A".to_vec())
                        .with_sequence(1)
                        .with_field(FixExpectedField::new(49, b"TARGET".to_vec())),
                ],
                true,
            )
            .expect("assert transcript");
        let report = harness.report(capabilities(), adapter.transport().transcript().metrics());
        assert!(report.passed());
        assert_eq!(report.scenarios()[0].transcript_frames(), 2);
        assert_ne!(report.transcript().rolling_hash(), 0);
    }

    #[test]
    fn full_suite_requires_every_scenario_capability_and_measurement() {
        let config = FixCertificationConfig::new()
            .require_capability(FixCertificationCapability::Limit)
            .require_capability(FixCertificationCapability::Amend);
        let mut harness =
            FixCertificationHarness::new("venue-a", FixVersion::Fix44, config).expect("harness");
        for scenario in FixCertificationScenario::ALL {
            harness
                .record_scenario(scenario, 1, std::iter::empty())
                .expect("scenario");
        }
        harness.exercise_capability(FixCertificationCapability::Limit);
        harness.exercise_capability(FixCertificationCapability::Amend);
        harness.record_latency_ns(90);
        harness.record_latency_ns(110);
        harness.set_allocation_evidence(FixCertificationAllocationEvidence::new(10_000, 0, 0));

        let report = harness.report(capabilities(), FixTranscriptCapture::default().metrics());
        assert!(report.passed());
        assert_eq!(
            report.scenarios().len(),
            FixCertificationScenario::ALL.len()
        );
        assert_eq!(report.latency().average_ns(), 100);
        assert_eq!(
            report
                .allocation()
                .expect("allocation")
                .hot_path_allocations(),
            0
        );
    }

    #[test]
    fn report_fails_closed_on_missing_scenario_capability_and_evidence() {
        let config = FixCertificationConfig::new()
            .with_required_scenarios([
                FixCertificationScenario::SessionLifecycle,
                FixCertificationScenario::MalformedMessage,
            ])
            .require_capability(FixCertificationCapability::Stop);
        let mut harness =
            FixCertificationHarness::new("venue-b", FixVersion::Fix42, config).expect("harness");
        harness
            .record_scenario(
                FixCertificationScenario::SessionLifecycle,
                1,
                std::iter::empty(),
            )
            .expect("scenario");
        let mut advertised = capabilities();
        advertised.stop = false;
        let report = harness.report(advertised, FixTranscriptCapture::default().metrics());
        assert!(!report.passed());
        assert_eq!(
            report.missing_scenarios(),
            &[FixCertificationScenario::MalformedMessage]
        );
        assert_eq!(
            report.unsupported_capabilities(),
            &[FixCertificationCapability::Stop]
        );
        assert!(report
            .failures()
            .iter()
            .any(|failure| failure.kind() == FixCertificationFailureKind::LatencyEvidenceMissing));
        assert!(report.failures().iter().any(|failure| {
            failure.kind() == FixCertificationFailureKind::AllocationEvidenceMissing
        }));
    }

    #[test]
    fn transcript_assertion_reports_field_and_length_mismatches() {
        let mut transport =
            FixScriptedTransport::new(FixScriptedTransportConfig::default()).expect("transport");
        transport.connect().expect("connect");
        transport.send(&logon(1)).expect("send");
        let records = transport.transcript().records().collect::<Vec<_>>();
        let mut harness = FixCertificationHarness::new(
            "bad-expectation",
            FixVersion::Fix44,
            FixCertificationConfig::new()
                .with_required_scenarios([FixCertificationScenario::SessionLifecycle])
                .with_latency_evidence_required(false)
                .with_allocation_evidence_required(false),
        )
        .expect("harness");
        harness
            .assert_transcript(
                FixCertificationScenario::SessionLifecycle,
                &records,
                &[
                    FixFrameExpectation::new(FixTranscriptDirection::Inbound, b"0".to_vec())
                        .with_sequence(9)
                        .with_field(FixExpectedField::new(49, b"WRONG".to_vec())),
                    FixFrameExpectation::new(FixTranscriptDirection::Outbound, b"5".to_vec()),
                ],
                true,
            )
            .expect("assert transcript");
        let report = harness.report(capabilities(), transport.transcript().metrics());
        assert!(!report.passed());
        assert!(report.failures().len() >= 5);
    }

    #[test]
    fn scripted_transport_enforces_bounds_without_partial_mutation() {
        let config =
            FixScriptedTransportConfig::new(1, 32, 1, 32, FixTranscriptConfig::new(4, 128, true));
        let mut transport = FixScriptedTransport::new(config).expect("transport");
        transport.enqueue_inbound(vec![1; 16]).expect("first");
        assert_eq!(
            transport.enqueue_inbound(vec![2; 8]),
            Err(FixScriptedTransportError::InboundCapacity)
        );
        assert_eq!(transport.inbound_frames(), 1);
        transport.connect().expect("connect");
        transport.send(&[3; 16]).expect("send");
        assert_eq!(
            transport.send(&[4; 8]),
            Err(FixScriptedTransportError::OutboundCapacity)
        );
        assert_eq!(transport.outbound_frames().len(), 1);
    }

    #[test]
    fn failure_details_are_bounded_and_duplicate_scenarios_rejected() {
        let config = FixCertificationConfig::new()
            .with_required_scenarios([FixCertificationScenario::MalformedMessage])
            .with_max_failures(1)
            .with_latency_evidence_required(false)
            .with_allocation_evidence_required(false);
        let mut harness =
            FixCertificationHarness::new("bounded", FixVersion::Fix44, config).expect("harness");
        harness
            .record_scenario(
                FixCertificationScenario::MalformedMessage,
                2,
                [
                    FixCertificationFailure::new(
                        None,
                        FixCertificationFailureKind::MalformedFrame,
                        "first",
                    ),
                    FixCertificationFailure::new(
                        None,
                        FixCertificationFailureKind::Behavior,
                        "second",
                    ),
                ],
            )
            .expect("scenario");
        assert_eq!(
            harness.record_scenario(
                FixCertificationScenario::MalformedMessage,
                1,
                std::iter::empty(),
            ),
            Err(FixCertificationHarnessError::DuplicateScenario(
                FixCertificationScenario::MalformedMessage
            ))
        );
        let report = harness.report(capabilities(), FixTranscriptCapture::default().metrics());
        assert_eq!(report.failures().len(), 1);
        assert_eq!(report.dropped_failure_details(), 1);
    }

    #[test]
    fn performance_limits_and_incomplete_transcript_fail_closed() {
        let config = FixCertificationConfig::new()
            .with_required_scenarios([FixCertificationScenario::SessionLifecycle])
            .with_max_observed_latency_ns(Some(50))
            .with_max_hot_path_allocations(Some(0))
            .with_max_hot_path_allocated_bytes(Some(0));
        let mut harness =
            FixCertificationHarness::new("limits", FixVersion::Fix44, config).expect("harness");
        harness
            .record_scenario(
                FixCertificationScenario::SessionLifecycle,
                1,
                std::iter::empty(),
            )
            .expect("scenario");
        harness.record_latency_ns(51);
        harness.set_allocation_evidence(FixCertificationAllocationEvidence::new(1, 1, 8));

        let mut capture = FixTranscriptCapture::new(FixTranscriptConfig::new(1, 8, true));
        capture
            .record_frame(FixTranscriptDirection::Outbound, 1, Some(1), b"A", &[1; 16])
            .expect("first");
        capture
            .record_frame(FixTranscriptDirection::Inbound, 2, Some(1), b"A", &[2; 16])
            .expect("second");
        let report = harness.report(capabilities(), capture.metrics());
        assert!(!report.passed());
        for kind in [
            FixCertificationFailureKind::LatencyLimitExceeded,
            FixCertificationFailureKind::AllocationLimitExceeded,
            FixCertificationFailureKind::TranscriptIncomplete,
        ] {
            assert!(report
                .failures()
                .iter()
                .any(|failure| failure.kind() == kind));
        }
    }

    #[test]
    fn zero_message_allocation_evidence_is_not_certification_evidence() {
        let config = FixCertificationConfig::new()
            .with_required_scenarios([FixCertificationScenario::SessionLifecycle])
            .with_latency_evidence_required(false);
        let mut harness =
            FixCertificationHarness::new("zero", FixVersion::Fix44, config).expect("harness");
        harness
            .record_scenario(
                FixCertificationScenario::SessionLifecycle,
                1,
                std::iter::empty(),
            )
            .expect("scenario");
        harness.set_allocation_evidence(FixCertificationAllocationEvidence::new(0, 0, 0));
        let report = harness.report(capabilities(), FixTranscriptCapture::default().metrics());
        assert!(!report.passed());
        assert!(report.failures().iter().any(|failure| {
            failure.kind() == FixCertificationFailureKind::AllocationEvidenceMissing
        }));
    }
}
