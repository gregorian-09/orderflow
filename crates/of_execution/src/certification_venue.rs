//! Deterministic execution-adapter certification venue.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

use of_execution_core::{
    AmendRequest, CancelRequest, ClientOrderId, ExecutionEvent, ExecutionId, ExecutionText,
    ExecutionType, OrderPrice, OrderQty, OrderRequest, OrderStatus, RiskRejectReason, VenueOrderId,
};

use crate::{
    ExecutionAdapter, ExecutionCapabilities, ExecutionError, ExecutionEventBuffer, ExecutionHealth,
    ExecutionResult,
};

const SCENARIO_KIND_COUNT: usize = 18;

/// A command expected by a scripted certification scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CertificationCommandKind {
    /// New-order submission.
    Submit,
    /// Order cancellation.
    Cancel,
    /// Cancel/replace request.
    Amend,
    /// Adapter polling/control-plane processing.
    Poll,
    /// Open-order recovery.
    Recover,
}

/// Outcome for the request side of a fill-versus-cancel/replace race.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CertificationRaceOutcome {
    /// The cancel or replace request is acknowledged after the fill.
    Ack,
    /// The cancel or replace request is rejected after the fill.
    Reject,
}

/// Stable classification for certification scenarios and coverage reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum CertificationScenarioKind {
    /// New order accepted without a fill.
    Accept = 0,
    /// New order rejected.
    Reject = 1,
    /// New order accepted and partially filled.
    PartialFill = 2,
    /// New order accepted and fully filled.
    FullFill = 3,
    /// Cancel acknowledged.
    CancelAck = 4,
    /// Cancel rejected.
    CancelReject = 5,
    /// Replace acknowledged.
    ReplaceAck = 6,
    /// Replace rejected.
    ReplaceReject = 7,
    /// Fill raced with cancel or replace processing.
    CancelReplaceRace = 8,
    /// A retained report was delivered more than once.
    DuplicateReports = 9,
    /// The latest two retained reports were delivered in reverse order.
    OutOfOrderReports = 10,
    /// Session disconnected.
    Disconnect = 11,
    /// Session reconnected.
    Reconnect = 12,
    /// Retained reports were resent from a sequence number.
    Resend = 13,
    /// Outbound report sequence was reset.
    SequenceReset = 14,
    /// Open orders were restated during recovery.
    RecoveryRestatement = 15,
    /// Report delivery was delayed by a deterministic poll count.
    SlowVenue = 16,
    /// Provider input failed canonical decoding or validation.
    MalformedProviderResponse = 17,
}

impl CertificationScenarioKind {
    /// Every scenario kind in stable discriminant order.
    pub const ALL: [Self; SCENARIO_KIND_COUNT] = [
        Self::Accept,
        Self::Reject,
        Self::PartialFill,
        Self::FullFill,
        Self::CancelAck,
        Self::CancelReject,
        Self::ReplaceAck,
        Self::ReplaceReject,
        Self::CancelReplaceRace,
        Self::DuplicateReports,
        Self::OutOfOrderReports,
        Self::Disconnect,
        Self::Reconnect,
        Self::Resend,
        Self::SequenceReset,
        Self::RecoveryRestatement,
        Self::SlowVenue,
        Self::MalformedProviderResponse,
    ];

    const fn index(self) -> usize {
        self as usize
    }

    const fn bit(self) -> u64 {
        1_u64 << self.index()
    }
}

/// One deterministic venue behavior in a certification script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CertificationScenario {
    /// Accepts a new order and leaves it working.
    Accept,
    /// Rejects a new order with structured diagnostics.
    Reject {
        /// Structured rejection reason.
        reason: RiskRejectReason,
        /// Bounded provider diagnostic.
        text: ExecutionText,
    },
    /// Accepts a new order and reports a partial fill.
    PartialFill {
        /// Fill quantity; must be less than the submitted quantity.
        quantity: OrderQty,
        /// Fill price.
        price: OrderPrice,
    },
    /// Accepts a new order and reports a complete fill.
    FullFill {
        /// Fill price.
        price: OrderPrice,
    },
    /// Acknowledges a cancel request.
    CancelAck,
    /// Rejects a cancel request while preserving the current order state.
    CancelReject {
        /// Bounded provider diagnostic.
        text: ExecutionText,
    },
    /// Acknowledges a cancel/replace request.
    ReplaceAck,
    /// Rejects a cancel/replace request while preserving current order state.
    ReplaceReject {
        /// Bounded provider diagnostic.
        text: ExecutionText,
    },
    /// Emits a fill before the result of an in-flight cancel or replace.
    CancelReplaceRace {
        /// Fill quantity applied before the request result.
        fill_quantity: OrderQty,
        /// Fill price.
        fill_price: OrderPrice,
        /// Request result delivered after the fill.
        outcome: CertificationRaceOutcome,
    },
    /// Re-delivers the latest retained report without assigning a new sequence.
    DuplicateReports {
        /// Number of duplicate copies to emit.
        copies: usize,
    },
    /// Re-delivers the latest two retained reports newest-first.
    OutOfOrderReports,
    /// Marks the venue disconnected.
    Disconnect,
    /// Marks the venue reconnected.
    Reconnect,
    /// Re-delivers retained reports from an inclusive report sequence.
    Resend {
        /// First retained sequence to resend.
        from_sequence: u64,
    },
    /// Sets the next newly generated report sequence.
    SequenceReset {
        /// Next report sequence; must be greater than zero.
        next_sequence: u64,
    },
    /// Restates every non-terminal order through `recover_open_orders`.
    RecoveryRestatement,
    /// Delays subsequently generated reports by this many `poll` calls.
    SlowVenue {
        /// Deterministic delivery delay in poll calls.
        polls: u64,
    },
    /// Simulates provider bytes that cannot become a canonical event.
    MalformedProviderResponse {
        /// Bounded decoding or validation diagnostic.
        text: ExecutionText,
    },
}

impl CertificationScenario {
    /// Returns the stable scenario classification.
    pub const fn kind(self) -> CertificationScenarioKind {
        match self {
            Self::Accept => CertificationScenarioKind::Accept,
            Self::Reject { .. } => CertificationScenarioKind::Reject,
            Self::PartialFill { .. } => CertificationScenarioKind::PartialFill,
            Self::FullFill { .. } => CertificationScenarioKind::FullFill,
            Self::CancelAck => CertificationScenarioKind::CancelAck,
            Self::CancelReject { .. } => CertificationScenarioKind::CancelReject,
            Self::ReplaceAck => CertificationScenarioKind::ReplaceAck,
            Self::ReplaceReject { .. } => CertificationScenarioKind::ReplaceReject,
            Self::CancelReplaceRace { .. } => CertificationScenarioKind::CancelReplaceRace,
            Self::DuplicateReports { .. } => CertificationScenarioKind::DuplicateReports,
            Self::OutOfOrderReports => CertificationScenarioKind::OutOfOrderReports,
            Self::Disconnect => CertificationScenarioKind::Disconnect,
            Self::Reconnect => CertificationScenarioKind::Reconnect,
            Self::Resend { .. } => CertificationScenarioKind::Resend,
            Self::SequenceReset { .. } => CertificationScenarioKind::SequenceReset,
            Self::RecoveryRestatement => CertificationScenarioKind::RecoveryRestatement,
            Self::SlowVenue { .. } => CertificationScenarioKind::SlowVenue,
            Self::MalformedProviderResponse { .. } => {
                CertificationScenarioKind::MalformedProviderResponse
            }
        }
    }

    /// Returns the adapter operation that consumes this scenario.
    pub const fn expected_command(self) -> CertificationCommandKind {
        match self {
            Self::Accept
            | Self::Reject { .. }
            | Self::PartialFill { .. }
            | Self::FullFill { .. } => CertificationCommandKind::Submit,
            Self::CancelAck | Self::CancelReject { .. } => CertificationCommandKind::Cancel,
            Self::ReplaceAck | Self::ReplaceReject { .. } => CertificationCommandKind::Amend,
            Self::CancelReplaceRace { .. } => CertificationCommandKind::Cancel,
            Self::RecoveryRestatement => CertificationCommandKind::Recover,
            Self::DuplicateReports { .. }
            | Self::OutOfOrderReports
            | Self::Disconnect
            | Self::Reconnect
            | Self::Resend { .. }
            | Self::SequenceReset { .. }
            | Self::SlowVenue { .. }
            | Self::MalformedProviderResponse { .. } => CertificationCommandKind::Poll,
        }
    }
}

/// Fixed resource bounds for a certification venue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertificationVenueConfig {
    scenario_capacity: usize,
    order_capacity: usize,
    pending_report_capacity: usize,
    retained_report_capacity: usize,
    transcript_capacity: usize,
}

impl CertificationVenueConfig {
    /// Creates a configuration with explicit fixed bounds.
    ///
    /// # Errors
    ///
    /// Returns [`CertificationVenueError::InvalidConfig`] if any bound is zero.
    pub const fn new(
        scenario_capacity: usize,
        order_capacity: usize,
        pending_report_capacity: usize,
        retained_report_capacity: usize,
        transcript_capacity: usize,
    ) -> Result<Self, CertificationVenueError> {
        if scenario_capacity == 0
            || order_capacity == 0
            || pending_report_capacity == 0
            || retained_report_capacity == 0
            || transcript_capacity == 0
        {
            return Err(CertificationVenueError::InvalidConfig);
        }
        Ok(Self {
            scenario_capacity,
            order_capacity,
            pending_report_capacity,
            retained_report_capacity,
            transcript_capacity,
        })
    }

    /// Returns the maximum scripted scenario count.
    pub const fn scenario_capacity(self) -> usize {
        self.scenario_capacity
    }

    /// Returns the maximum tracked order count.
    pub const fn order_capacity(self) -> usize {
        self.order_capacity
    }

    /// Returns the maximum delayed report count.
    pub const fn pending_report_capacity(self) -> usize {
        self.pending_report_capacity
    }

    /// Returns the retained report count used by duplicate/resend scenarios.
    pub const fn retained_report_capacity(self) -> usize {
        self.retained_report_capacity
    }

    /// Returns the retained transcript-entry count.
    pub const fn transcript_capacity(self) -> usize {
        self.transcript_capacity
    }
}

impl Default for CertificationVenueConfig {
    fn default() -> Self {
        Self {
            scenario_capacity: 256,
            order_capacity: 1024,
            pending_report_capacity: 2048,
            retained_report_capacity: 4096,
            transcript_capacity: 4096,
        }
    }
}

/// Typed certification-venue failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CertificationVenueError {
    /// One or more configured bounds are zero.
    InvalidConfig,
    /// The scenario queue reached its configured capacity.
    ScenarioBufferFull,
    /// The tracked-order table reached its configured capacity.
    OrderCapacityExceeded,
    /// The delayed-report queue reached its configured capacity.
    PendingReportBufferFull,
    /// Newly generated report sequence space is exhausted.
    ReportSequenceExhausted,
    /// No scripted scenario is available for the requested operation.
    ScriptExhausted {
        /// Operation that required a scenario.
        actual: CertificationCommandKind,
    },
    /// The next scenario expects a different operation.
    UnexpectedCommand {
        /// Operation expected by the next scenario.
        expected: CertificationCommandKind,
        /// Operation invoked by the adapter host.
        actual: CertificationCommandKind,
    },
    /// The requested order is unknown to this venue.
    UnknownOrder,
    /// Scenario quantity or sequence data is invalid for current state.
    InvalidScenario,
    /// Retained history does not contain enough reports for the scenario.
    InsufficientReportHistory,
}

impl fmt::Display for CertificationVenueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig => f.write_str("certification venue bounds must be non-zero"),
            Self::ScenarioBufferFull => f.write_str("certification scenario buffer is full"),
            Self::OrderCapacityExceeded => f.write_str("certification order capacity exceeded"),
            Self::PendingReportBufferFull => {
                f.write_str("certification pending-report buffer is full")
            }
            Self::ReportSequenceExhausted => {
                f.write_str("certification report sequence is exhausted")
            }
            Self::ScriptExhausted { actual } => {
                write!(f, "certification script exhausted before {actual:?}")
            }
            Self::UnexpectedCommand { expected, actual } => write!(
                f,
                "certification script expected {expected:?}, received {actual:?}"
            ),
            Self::UnknownOrder => f.write_str("certification venue order is unknown"),
            Self::InvalidScenario => {
                f.write_str("certification scenario is invalid for order state")
            }
            Self::InsufficientReportHistory => {
                f.write_str("certification report history is insufficient")
            }
        }
    }
}

impl Error for CertificationVenueError {}

/// One generated canonical report with its venue session sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertificationReport {
    sequence: u64,
    event: ExecutionEvent,
}

impl CertificationReport {
    /// Returns the venue report sequence.
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    /// Returns the canonical execution event.
    pub const fn event(self) -> ExecutionEvent {
        self.event
    }
}

/// Result recorded after consuming one scripted scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CertificationStepResult {
    /// Scenario completed and generated its expected effect.
    Applied,
    /// Scenario deliberately produced a provider failure.
    InjectedFailure,
}

/// Bounded transcript metadata for one consumed scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertificationTranscriptEntry {
    ordinal: u64,
    poll_index: u64,
    scenario: CertificationScenarioKind,
    command: CertificationCommandKind,
    result: CertificationStepResult,
    first_report_sequence: u64,
    report_count: usize,
}

impl CertificationTranscriptEntry {
    /// Returns the monotonic transcript ordinal.
    pub const fn ordinal(self) -> u64 {
        self.ordinal
    }

    /// Returns the poll index at which the scenario was consumed.
    pub const fn poll_index(self) -> u64 {
        self.poll_index
    }

    /// Returns the consumed scenario kind.
    pub const fn scenario(self) -> CertificationScenarioKind {
        self.scenario
    }

    /// Returns the operation that consumed the scenario.
    pub const fn command(self) -> CertificationCommandKind {
        self.command
    }

    /// Returns the scenario outcome classification.
    pub const fn result(self) -> CertificationStepResult {
        self.result
    }

    /// Returns the first newly assigned report sequence, or zero if none.
    pub const fn first_report_sequence(self) -> u64 {
        self.first_report_sequence
    }

    /// Returns the number of reports generated or replayed by the scenario.
    pub const fn report_count(self) -> usize {
        self.report_count
    }
}

/// Immutable scenario coverage report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertificationCoverage {
    mask: u64,
    counts: [u64; SCENARIO_KIND_COUNT],
}

impl CertificationCoverage {
    /// Returns true if the scenario has completed at least once.
    pub const fn contains(self, kind: CertificationScenarioKind) -> bool {
        self.mask & kind.bit() != 0
    }

    /// Returns the completed count for a scenario kind.
    pub const fn count(self, kind: CertificationScenarioKind) -> u64 {
        self.counts[kind.index()]
    }

    /// Returns true when every built-in scenario kind has completed.
    pub const fn is_complete(self) -> bool {
        self.mask == (1_u64 << SCENARIO_KIND_COUNT) - 1
    }

    /// Returns the raw stable coverage bit mask.
    pub const fn mask(self) -> u64 {
        self.mask
    }
}

/// Operational snapshot of a deterministic certification venue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CertificationVenueSnapshot {
    connected: bool,
    degraded: bool,
    health_seq: u64,
    next_report_sequence: u64,
    poll_index: u64,
    remaining_scenarios: usize,
    tracked_orders: usize,
    pending_reports: usize,
    retained_reports: usize,
    retained_report_evictions: u64,
    transcript_entries: usize,
    transcript_evictions: u64,
    delivery_delay_polls: u64,
    coverage: CertificationCoverage,
}

impl CertificationVenueSnapshot {
    /// Returns true when the simulated session is connected.
    pub const fn connected(self) -> bool {
        self.connected
    }
    /// Returns true when the simulated session is degraded.
    pub const fn degraded(self) -> bool {
        self.degraded
    }
    /// Returns the monotonic health sequence.
    pub const fn health_seq(self) -> u64 {
        self.health_seq
    }
    /// Returns the next sequence assigned to a newly generated report.
    pub const fn next_report_sequence(self) -> u64 {
        self.next_report_sequence
    }
    /// Returns the number of adapter polls performed.
    pub const fn poll_index(self) -> u64 {
        self.poll_index
    }
    /// Returns the number of unconsumed scripted scenarios.
    pub const fn remaining_scenarios(self) -> usize {
        self.remaining_scenarios
    }
    /// Returns the number of retained order records.
    pub const fn tracked_orders(self) -> usize {
        self.tracked_orders
    }
    /// Returns the number of delayed reports awaiting release.
    pub const fn pending_reports(self) -> usize {
        self.pending_reports
    }
    /// Returns the number of reports retained for replay.
    pub const fn retained_reports(self) -> usize {
        self.retained_reports
    }
    /// Returns the number of reports evicted from bounded replay history.
    pub const fn retained_report_evictions(self) -> u64 {
        self.retained_report_evictions
    }
    /// Returns the number of retained transcript entries.
    pub const fn transcript_entries(self) -> usize {
        self.transcript_entries
    }
    /// Returns the number of transcript entries evicted by the ring bound.
    pub const fn transcript_evictions(self) -> u64 {
        self.transcript_evictions
    }
    /// Returns the active deterministic report delay.
    pub const fn delivery_delay_polls(self) -> u64 {
        self.delivery_delay_polls
    }
    /// Returns scenario coverage and completion counts.
    pub const fn coverage(self) -> CertificationCoverage {
        self.coverage
    }
}

#[derive(Debug, Clone, Copy)]
struct TrackedOrder {
    request: OrderRequest,
    current_client_order_id: ClientOrderId,
    venue_order_id: VenueOrderId,
    cumulative_qty: OrderQty,
    leaves_qty: OrderQty,
    average_price: OrderPrice,
    status: OrderStatus,
}

impl TrackedOrder {
    fn new(request: OrderRequest, venue_order_id: VenueOrderId) -> Self {
        Self {
            request,
            current_client_order_id: request.client_order_id,
            venue_order_id,
            cumulative_qty: OrderQty(0),
            leaves_qty: request.quantity,
            average_price: OrderPrice(0),
            status: OrderStatus::New,
        }
    }

    fn is_open(self) -> bool {
        !self.status.is_terminal()
    }
}

#[derive(Debug, Clone, Copy)]
struct PendingReport {
    release_poll: u64,
    report: CertificationReport,
}

#[derive(Debug, Clone, Copy)]
struct StatusEventSpec {
    exec_type: ExecutionType,
    status: OrderStatus,
    client_order_id: ClientOrderId,
    orig_client_order_id: ClientOrderId,
    leaves_qty: OrderQty,
    text: ExecutionText,
    ts_recv_ns: u64,
}

/// Script-driven mock exchange for deterministic OMS and adapter certification.
///
/// Construction reserves all configured collections. Normal command and poll
/// paths stay within those fixed bounds; capacity exhaustion is explicit. This
/// adapter is deterministic and does not read wall-clock time or random state.
#[derive(Debug, Clone)]
pub struct CertificationVenue {
    config: CertificationVenueConfig,
    connected: bool,
    degraded: bool,
    health_seq: u64,
    last_error: Option<String>,
    scenarios: VecDeque<CertificationScenario>,
    orders: Vec<TrackedOrder>,
    pending_reports: VecDeque<PendingReport>,
    report_history: VecDeque<CertificationReport>,
    report_evictions: u64,
    transcript: VecDeque<CertificationTranscriptEntry>,
    transcript_evictions: u64,
    coverage: CertificationCoverage,
    transcript_ordinal: u64,
    next_report_sequence: u64,
    poll_index: u64,
    delivery_delay_polls: u64,
}

impl CertificationVenue {
    /// Creates an empty venue and reserves every configured bounded collection.
    ///
    /// # Errors
    ///
    /// Returns [`CertificationVenueError::InvalidConfig`] for zero bounds.
    pub fn new(config: CertificationVenueConfig) -> Result<Self, CertificationVenueError> {
        let config = CertificationVenueConfig::new(
            config.scenario_capacity,
            config.order_capacity,
            config.pending_report_capacity,
            config.retained_report_capacity,
            config.transcript_capacity,
        )?;
        Ok(Self {
            config,
            connected: false,
            degraded: false,
            health_seq: 0,
            last_error: None,
            scenarios: VecDeque::with_capacity(config.scenario_capacity),
            orders: Vec::with_capacity(config.order_capacity),
            pending_reports: VecDeque::with_capacity(config.pending_report_capacity),
            report_history: VecDeque::with_capacity(config.retained_report_capacity),
            report_evictions: 0,
            transcript: VecDeque::with_capacity(config.transcript_capacity),
            transcript_evictions: 0,
            coverage: CertificationCoverage {
                mask: 0,
                counts: [0; SCENARIO_KIND_COUNT],
            },
            transcript_ordinal: 0,
            next_report_sequence: 1,
            poll_index: 0,
            delivery_delay_polls: 0,
        })
    }

    /// Appends one scenario to the bounded script.
    ///
    /// # Errors
    ///
    /// Returns [`CertificationVenueError::ScenarioBufferFull`] at the bound.
    pub fn enqueue(
        &mut self,
        scenario: CertificationScenario,
    ) -> Result<(), CertificationVenueError> {
        if self.scenarios.len() >= self.config.scenario_capacity {
            return Err(CertificationVenueError::ScenarioBufferFull);
        }
        self.scenarios.push_back(scenario);
        Ok(())
    }

    /// Appends scenarios in iterator order.
    ///
    /// The operation is atomic: no scenario is appended unless the complete
    /// iterator fits the remaining fixed capacity.
    ///
    /// # Errors
    ///
    /// Returns [`CertificationVenueError::ScenarioBufferFull`] at the bound.
    pub fn enqueue_all<I>(&mut self, scenarios: I) -> Result<(), CertificationVenueError>
    where
        I: IntoIterator<Item = CertificationScenario>,
        I::IntoIter: ExactSizeIterator,
    {
        let scenarios = scenarios.into_iter();
        if scenarios.len()
            > self
                .config
                .scenario_capacity
                .saturating_sub(self.scenarios.len())
        {
            return Err(CertificationVenueError::ScenarioBufferFull);
        }
        self.scenarios.extend(scenarios);
        Ok(())
    }

    /// Removes all unconsumed scenarios without changing venue state.
    pub fn clear_script(&mut self) {
        self.scenarios.clear();
    }

    /// Returns the configured resource bounds.
    pub const fn config(&self) -> CertificationVenueConfig {
        self.config
    }

    /// Returns an operational and coverage snapshot.
    pub fn snapshot(&self) -> CertificationVenueSnapshot {
        CertificationVenueSnapshot {
            connected: self.connected,
            degraded: self.degraded,
            health_seq: self.health_seq,
            next_report_sequence: self.next_report_sequence,
            poll_index: self.poll_index,
            remaining_scenarios: self.scenarios.len(),
            tracked_orders: self.orders.len(),
            pending_reports: self.pending_reports.len(),
            retained_reports: self.report_history.len(),
            retained_report_evictions: self.report_evictions,
            transcript_entries: self.transcript.len(),
            transcript_evictions: self.transcript_evictions,
            delivery_delay_polls: self.delivery_delay_polls,
            coverage: self.coverage,
        }
    }

    /// Iterates retained report metadata from oldest to newest.
    pub fn retained_reports(&self) -> impl ExactSizeIterator<Item = &CertificationReport> {
        self.report_history.iter()
    }

    /// Iterates bounded transcript metadata from oldest to newest.
    pub fn transcript(&self) -> impl ExactSizeIterator<Item = &CertificationTranscriptEntry> {
        self.transcript.iter()
    }

    fn require_connected(&self) -> ExecutionResult<()> {
        if self.connected {
            Ok(())
        } else {
            Err(ExecutionError::Disconnected)
        }
    }

    fn scenario_for(
        &self,
        actual: CertificationCommandKind,
    ) -> Result<CertificationScenario, CertificationVenueError> {
        let scenario = self
            .scenarios
            .front()
            .copied()
            .ok_or(CertificationVenueError::ScriptExhausted { actual })?;
        let expected = scenario.expected_command();
        if expected != actual
            && !(matches!(scenario, CertificationScenario::CancelReplaceRace { .. })
                && actual == CertificationCommandKind::Amend)
        {
            return Err(CertificationVenueError::UnexpectedCommand { expected, actual });
        }
        Ok(scenario)
    }

    fn consume_scenario(&mut self) {
        self.scenarios
            .pop_front()
            .expect("scenario was validated from the queue front");
    }

    fn adapter_error(error: CertificationVenueError) -> ExecutionError {
        ExecutionError::Adapter(error.to_string())
    }

    fn order_index(&self, current_or_original: ClientOrderId) -> Option<usize> {
        self.orders.iter().position(|order| {
            order.current_client_order_id == current_or_original
                || order.request.client_order_id == current_or_original
        })
    }

    fn next_fixed_id<const N: usize>(
        prefix: &[u8],
        value: u64,
    ) -> of_execution_core::FixedAscii<N> {
        let mut bytes = [0_u8; N];
        let mut len = 0;
        for byte in prefix {
            if len < N {
                bytes[len] = *byte;
                len += 1;
            }
        }
        let mut digits = [0_u8; 20];
        let mut remaining = value;
        let mut digit_count = 0;
        loop {
            digits[digit_count] = b'0' + (remaining % 10) as u8;
            digit_count += 1;
            remaining /= 10;
            if remaining == 0 {
                break;
            }
        }
        for index in (0..digit_count).rev() {
            if len < N {
                bytes[len] = digits[index];
                len += 1;
            }
        }
        let text = std::str::from_utf8(&bytes[..len]).expect("stack id is ASCII");
        of_execution_core::FixedAscii::new(text).expect("stack id fits fixed capacity")
    }

    fn venue_order_id(&self) -> VenueOrderId {
        Self::next_fixed_id(b"CERT-", self.next_report_sequence)
    }

    fn execution_id(&self) -> ExecutionId {
        Self::next_fixed_id(b"CERTX-", self.next_report_sequence)
    }

    fn reserve_generated_reports(
        &self,
        count: usize,
        out: &ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        let count = u64::try_from(count)
            .map_err(|_| Self::adapter_error(CertificationVenueError::ReportSequenceExhausted))?;
        if self.next_report_sequence.checked_add(count).is_none() {
            return Err(Self::adapter_error(
                CertificationVenueError::ReportSequenceExhausted,
            ));
        }
        let count = count as usize;
        if self.delivery_delay_polls == 0 {
            if count > out.max_len().saturating_sub(out.len()) {
                return Err(ExecutionError::BufferFull);
            }
        } else if count
            > self
                .config
                .pending_report_capacity
                .saturating_sub(self.pending_reports.len())
        {
            return Err(Self::adapter_error(
                CertificationVenueError::PendingReportBufferFull,
            ));
        }
        Ok(())
    }

    fn record_report(&mut self, event: ExecutionEvent) -> CertificationReport {
        let report = CertificationReport {
            sequence: self.next_report_sequence,
            event,
        };
        self.next_report_sequence = self.next_report_sequence.saturating_add(1);
        if self.report_history.len() == self.config.retained_report_capacity {
            self.report_history.pop_front();
            self.report_evictions = self.report_evictions.saturating_add(1);
        }
        self.report_history.push_back(report);
        report
    }

    fn deliver_generated(
        &mut self,
        event: ExecutionEvent,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<u64> {
        let report = self.record_report(event);
        if self.delivery_delay_polls == 0 {
            out.push(report.event)?;
        } else {
            let release_poll = self.poll_index.saturating_add(self.delivery_delay_polls);
            let pending = PendingReport {
                release_poll,
                report,
            };
            if let Some(index) = self
                .pending_reports
                .iter()
                .position(|existing| existing.release_poll > release_poll)
            {
                self.pending_reports.insert(index, pending);
            } else {
                self.pending_reports.push_back(pending);
            }
        }
        Ok(report.sequence)
    }

    fn record_step(
        &mut self,
        scenario: CertificationScenarioKind,
        command: CertificationCommandKind,
        result: CertificationStepResult,
        first_report_sequence: u64,
        report_count: usize,
    ) {
        self.coverage.mask |= scenario.bit();
        self.coverage.counts[scenario.index()] =
            self.coverage.counts[scenario.index()].saturating_add(1);
        self.transcript_ordinal = self.transcript_ordinal.saturating_add(1);
        if self.transcript.len() == self.config.transcript_capacity {
            self.transcript.pop_front();
            self.transcript_evictions = self.transcript_evictions.saturating_add(1);
        }
        self.transcript.push_back(CertificationTranscriptEntry {
            ordinal: self.transcript_ordinal,
            poll_index: self.poll_index,
            scenario,
            command,
            result,
            first_report_sequence,
            report_count,
        });
    }

    fn fill_event(
        order: TrackedOrder,
        quantity: OrderQty,
        price: OrderPrice,
        execution_id: ExecutionId,
        ts_recv_ns: u64,
    ) -> ExecutionEvent {
        let cumulative_qty = OrderQty(order.cumulative_qty.0.saturating_add(quantity.0));
        let leaves_qty = OrderQty(order.leaves_qty.0.saturating_sub(quantity.0));
        let weighted = order
            .average_price
            .0
            .saturating_mul(order.cumulative_qty.0)
            .saturating_add(price.0.saturating_mul(quantity.0));
        let average_price = OrderPrice(weighted / cumulative_qty.0.max(1));
        ExecutionEvent {
            exec_type: ExecutionType::Trade,
            order_status: if leaves_qty.0 == 0 {
                OrderStatus::Filled
            } else {
                OrderStatus::PartiallyFilled
            },
            client_order_id: order.current_client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: order.venue_order_id,
            execution_id,
            account_id: order.request.account_id,
            route_id: order.request.route_id,
            symbol: order.request.symbol,
            last_qty: quantity,
            last_price: price,
            cumulative_qty,
            leaves_qty,
            average_price,
            ts_exchange_ns: order.request.ts_exchange_ns,
            ts_recv_ns,
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        }
    }

    fn apply_fill(order: &mut TrackedOrder, event: ExecutionEvent) {
        order.cumulative_qty = event.cumulative_qty;
        order.leaves_qty = event.leaves_qty;
        order.average_price = event.average_price;
        order.status = event.order_status;
    }

    fn status_event(order: TrackedOrder, spec: StatusEventSpec) -> ExecutionEvent {
        ExecutionEvent {
            exec_type: spec.exec_type,
            order_status: spec.status,
            client_order_id: spec.client_order_id,
            orig_client_order_id: spec.orig_client_order_id,
            venue_order_id: order.venue_order_id,
            execution_id: ExecutionId::empty(),
            account_id: order.request.account_id,
            route_id: order.request.route_id,
            symbol: order.request.symbol,
            last_qty: OrderQty(0),
            last_price: OrderPrice(0),
            cumulative_qty: order.cumulative_qty,
            leaves_qty: spec.leaves_qty,
            average_price: order.average_price,
            ts_exchange_ns: order.request.ts_exchange_ns,
            ts_recv_ns: spec.ts_recv_ns,
            reason: RiskRejectReason::None,
            text: spec.text,
        }
    }

    fn drain_ready(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        let ready = self
            .pending_reports
            .iter()
            .take_while(|pending| pending.release_poll <= self.poll_index)
            .count();
        if ready > out.max_len().saturating_sub(out.len()) {
            return Err(ExecutionError::BufferFull);
        }
        for _ in 0..ready {
            let report = self
                .pending_reports
                .pop_front()
                .expect("ready count came from queue");
            out.push(report.report.event)?;
        }
        Ok(ready)
    }

    fn replay_reports<I>(reports: I, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize>
    where
        I: IntoIterator<Item = CertificationReport>,
        I::IntoIter: ExactSizeIterator,
    {
        let reports = reports.into_iter();
        if reports.len() > out.max_len().saturating_sub(out.len()) {
            return Err(ExecutionError::BufferFull);
        }
        let mut count = 0;
        for report in reports {
            out.push(report.event)?;
            count += 1;
        }
        Ok(count)
    }

    fn replay_retained_from(
        &self,
        from_sequence: u64,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<usize> {
        let count = self
            .report_history
            .iter()
            .filter(|report| report.sequence >= from_sequence)
            .count();
        if count > out.max_len().saturating_sub(out.len()) {
            return Err(ExecutionError::BufferFull);
        }
        for report in self
            .report_history
            .iter()
            .filter(|report| report.sequence >= from_sequence)
        {
            out.push(report.event)?;
        }
        Ok(count)
    }

    fn process_control(
        &mut self,
        scenario: CertificationScenario,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<usize> {
        let kind = scenario.kind();
        let (result, first_report_sequence) = match scenario {
            CertificationScenario::DuplicateReports { copies } => {
                let report = self.report_history.back().copied().ok_or_else(|| {
                    Self::adapter_error(CertificationVenueError::InsufficientReportHistory)
                })?;
                if copies == 0 {
                    return Err(Self::adapter_error(
                        CertificationVenueError::InvalidScenario,
                    ));
                }
                (
                    Self::replay_reports(std::iter::repeat_n(report, copies), out)?,
                    report.sequence,
                )
            }
            CertificationScenario::OutOfOrderReports => {
                if self.report_history.len() < 2 {
                    return Err(Self::adapter_error(
                        CertificationVenueError::InsufficientReportHistory,
                    ));
                }
                let newest = self.report_history[self.report_history.len() - 1];
                let previous = self.report_history[self.report_history.len() - 2];
                (
                    Self::replay_reports([newest, previous], out)?,
                    newest.sequence,
                )
            }
            CertificationScenario::Disconnect => {
                self.connected = false;
                self.health_seq = self.health_seq.saturating_add(1);
                self.last_error = Some("scripted certification disconnect".to_string());
                (0, 0)
            }
            CertificationScenario::Reconnect => {
                self.connected = true;
                self.degraded = false;
                self.health_seq = self.health_seq.saturating_add(1);
                self.last_error = None;
                (0, 0)
            }
            CertificationScenario::Resend { from_sequence } => {
                let first = self
                    .report_history
                    .front()
                    .map(|report| report.sequence)
                    .ok_or_else(|| {
                        Self::adapter_error(CertificationVenueError::InsufficientReportHistory)
                    })?;
                let count = self
                    .report_history
                    .iter()
                    .filter(|report| report.sequence >= from_sequence)
                    .count();
                if from_sequence < first || count == 0 {
                    return Err(Self::adapter_error(
                        CertificationVenueError::InsufficientReportHistory,
                    ));
                }
                (
                    self.replay_retained_from(from_sequence, out)?,
                    from_sequence,
                )
            }
            CertificationScenario::SequenceReset { next_sequence } => {
                if next_sequence == 0 {
                    return Err(Self::adapter_error(
                        CertificationVenueError::InvalidScenario,
                    ));
                }
                self.next_report_sequence = next_sequence;
                (0, 0)
            }
            CertificationScenario::SlowVenue { polls } => {
                self.delivery_delay_polls = polls;
                (0, 0)
            }
            CertificationScenario::MalformedProviderResponse { text } => {
                self.degraded = true;
                self.health_seq = self.health_seq.saturating_add(1);
                self.last_error = Some(text.as_str().to_string());
                self.record_step(
                    kind,
                    CertificationCommandKind::Poll,
                    CertificationStepResult::InjectedFailure,
                    0,
                    0,
                );
                return Err(ExecutionError::Adapter(text.as_str().to_string()));
            }
            _ => {
                return Err(Self::adapter_error(
                    CertificationVenueError::UnexpectedCommand {
                        expected: scenario.expected_command(),
                        actual: CertificationCommandKind::Poll,
                    },
                ))
            }
        };
        self.record_step(
            kind,
            CertificationCommandKind::Poll,
            CertificationStepResult::Applied,
            first_report_sequence,
            result,
        );
        Ok(result)
    }

    fn validate_control(
        &self,
        scenario: CertificationScenario,
        out: &ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        let available = out.max_len().saturating_sub(out.len());
        match scenario {
            CertificationScenario::DuplicateReports { copies } => {
                if copies == 0 {
                    return Err(Self::adapter_error(
                        CertificationVenueError::InvalidScenario,
                    ));
                }
                if self.report_history.is_empty() {
                    return Err(Self::adapter_error(
                        CertificationVenueError::InsufficientReportHistory,
                    ));
                }
                if copies > available {
                    return Err(ExecutionError::BufferFull);
                }
            }
            CertificationScenario::OutOfOrderReports => {
                if self.report_history.len() < 2 {
                    return Err(Self::adapter_error(
                        CertificationVenueError::InsufficientReportHistory,
                    ));
                }
                if available < 2 {
                    return Err(ExecutionError::BufferFull);
                }
            }
            CertificationScenario::Resend { from_sequence } => {
                let first = self
                    .report_history
                    .front()
                    .map(|report| report.sequence)
                    .ok_or_else(|| {
                        Self::adapter_error(CertificationVenueError::InsufficientReportHistory)
                    })?;
                let count = self
                    .report_history
                    .iter()
                    .filter(|report| report.sequence >= from_sequence)
                    .count();
                if from_sequence < first || count == 0 {
                    return Err(Self::adapter_error(
                        CertificationVenueError::InsufficientReportHistory,
                    ));
                }
                if count > available {
                    return Err(ExecutionError::BufferFull);
                }
            }
            CertificationScenario::SequenceReset { next_sequence: 0 } => {
                return Err(Self::adapter_error(
                    CertificationVenueError::InvalidScenario,
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

impl Default for CertificationVenue {
    fn default() -> Self {
        Self::new(CertificationVenueConfig::default()).expect("default bounds are non-zero")
    }
}

impl ExecutionAdapter for CertificationVenue {
    fn connect(&mut self) -> ExecutionResult<()> {
        self.connected = true;
        self.degraded = false;
        self.health_seq = self.health_seq.saturating_add(1);
        self.last_error = None;
        Ok(())
    }

    fn submit(
        &mut self,
        req: &OrderRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        self.require_connected()?;
        let scenario = self
            .scenario_for(CertificationCommandKind::Submit)
            .map_err(Self::adapter_error)?;
        let report_count = match scenario {
            CertificationScenario::Reject { .. } => 1,
            CertificationScenario::Accept => 1,
            CertificationScenario::PartialFill { .. } | CertificationScenario::FullFill { .. } => 2,
            _ => {
                return Err(Self::adapter_error(
                    CertificationVenueError::InvalidScenario,
                ))
            }
        };
        self.reserve_generated_reports(report_count, out)?;
        if !matches!(scenario, CertificationScenario::Reject { .. })
            && self.orders.len() >= self.config.order_capacity
        {
            return Err(Self::adapter_error(
                CertificationVenueError::OrderCapacityExceeded,
            ));
        }
        match scenario {
            CertificationScenario::PartialFill { quantity, price }
                if quantity.0 <= 0 || quantity.0 >= req.quantity.0 || price.0 <= 0 =>
            {
                return Err(Self::adapter_error(
                    CertificationVenueError::InvalidScenario,
                ));
            }
            CertificationScenario::FullFill { price } if price.0 <= 0 => {
                return Err(Self::adapter_error(
                    CertificationVenueError::InvalidScenario,
                ));
            }
            _ => {}
        }
        self.consume_scenario();
        let first_sequence = self.next_report_sequence;
        match scenario {
            CertificationScenario::Reject { reason, text } => {
                self.deliver_generated(ExecutionEvent::rejected(req, reason, text), out)?;
            }
            CertificationScenario::Accept
            | CertificationScenario::PartialFill { .. }
            | CertificationScenario::FullFill { .. } => {
                let venue_order_id = self.venue_order_id();
                self.deliver_generated(ExecutionEvent::accepted(req, venue_order_id), out)?;
                let mut order = TrackedOrder::new(*req, venue_order_id);
                match scenario {
                    CertificationScenario::PartialFill { quantity, price } => {
                        let fill = Self::fill_event(
                            order,
                            quantity,
                            price,
                            self.execution_id(),
                            req.ts_recv_ns.saturating_add(1),
                        );
                        Self::apply_fill(&mut order, fill);
                        self.deliver_generated(fill, out)?;
                    }
                    CertificationScenario::FullFill { price } => {
                        let fill = Self::fill_event(
                            order,
                            req.quantity,
                            price,
                            self.execution_id(),
                            req.ts_recv_ns.saturating_add(1),
                        );
                        Self::apply_fill(&mut order, fill);
                        self.deliver_generated(fill, out)?;
                    }
                    CertificationScenario::Accept => {}
                    _ => unreachable!(),
                }
                self.orders.push(order);
            }
            _ => unreachable!(),
        }
        self.record_step(
            scenario.kind(),
            CertificationCommandKind::Submit,
            CertificationStepResult::Applied,
            first_sequence,
            report_count,
        );
        Ok(())
    }

    fn cancel(
        &mut self,
        req: &CancelRequest,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        self.require_connected()?;
        let scenario = self
            .scenario_for(CertificationCommandKind::Cancel)
            .map_err(Self::adapter_error)?;
        let report_count = if matches!(scenario, CertificationScenario::CancelReplaceRace { .. }) {
            2
        } else {
            1
        };
        self.reserve_generated_reports(report_count, out)?;
        let index = self
            .order_index(req.orig_client_order_id)
            .ok_or_else(|| Self::adapter_error(CertificationVenueError::UnknownOrder))?;
        if let CertificationScenario::CancelReplaceRace {
            fill_quantity,
            fill_price,
            ..
        } = scenario
        {
            let order = self.orders[index];
            if fill_quantity.0 <= 0 || fill_quantity.0 > order.leaves_qty.0 || fill_price.0 <= 0 {
                return Err(Self::adapter_error(
                    CertificationVenueError::InvalidScenario,
                ));
            }
        }
        self.consume_scenario();
        let first_sequence = self.next_report_sequence;
        if let CertificationScenario::CancelReplaceRace {
            fill_quantity,
            fill_price,
            outcome,
        } = scenario
        {
            let order = self.orders[index];
            let fill = Self::fill_event(
                order,
                fill_quantity,
                fill_price,
                self.execution_id(),
                req.ts_recv_ns,
            );
            Self::apply_fill(&mut self.orders[index], fill);
            self.deliver_generated(fill, out)?;
            let order = self.orders[index];
            let (exec_type, status, leaves, text) = match outcome {
                CertificationRaceOutcome::Ack => (
                    ExecutionType::CancelAck,
                    OrderStatus::Cancelled,
                    OrderQty(0),
                    ExecutionText::empty(),
                ),
                CertificationRaceOutcome::Reject => (
                    ExecutionType::CancelReject,
                    order.status,
                    order.leaves_qty,
                    ExecutionText::new("cancel lost race").expect("literal fits"),
                ),
            };
            let event = Self::status_event(
                order,
                StatusEventSpec {
                    exec_type,
                    status,
                    client_order_id: req.client_order_id,
                    orig_client_order_id: req.orig_client_order_id,
                    leaves_qty: leaves,
                    text,
                    ts_recv_ns: req.ts_recv_ns.saturating_add(1),
                },
            );
            if outcome == CertificationRaceOutcome::Ack {
                self.orders[index].status = OrderStatus::Cancelled;
                self.orders[index].leaves_qty = OrderQty(0);
            }
            self.deliver_generated(event, out)?;
        } else {
            let order = self.orders[index];
            let (exec_type, status, leaves, text) = match scenario {
                CertificationScenario::CancelAck => (
                    ExecutionType::CancelAck,
                    OrderStatus::Cancelled,
                    OrderQty(0),
                    ExecutionText::empty(),
                ),
                CertificationScenario::CancelReject { text } => (
                    ExecutionType::CancelReject,
                    order.status,
                    order.leaves_qty,
                    text,
                ),
                _ => unreachable!(),
            };
            let event = Self::status_event(
                order,
                StatusEventSpec {
                    exec_type,
                    status,
                    client_order_id: req.client_order_id,
                    orig_client_order_id: req.orig_client_order_id,
                    leaves_qty: leaves,
                    text,
                    ts_recv_ns: req.ts_recv_ns,
                },
            );
            if matches!(scenario, CertificationScenario::CancelAck) {
                self.orders[index].status = OrderStatus::Cancelled;
                self.orders[index].leaves_qty = OrderQty(0);
            }
            self.deliver_generated(event, out)?;
        }
        self.record_step(
            scenario.kind(),
            CertificationCommandKind::Cancel,
            CertificationStepResult::Applied,
            first_sequence,
            report_count,
        );
        Ok(())
    }

    fn amend(&mut self, req: &AmendRequest, out: &mut ExecutionEventBuffer) -> ExecutionResult<()> {
        self.require_connected()?;
        let scenario = self
            .scenario_for(CertificationCommandKind::Amend)
            .map_err(Self::adapter_error)?;
        let report_count = if matches!(scenario, CertificationScenario::CancelReplaceRace { .. }) {
            2
        } else {
            1
        };
        self.reserve_generated_reports(report_count, out)?;
        let index = self
            .order_index(req.orig_client_order_id)
            .ok_or_else(|| Self::adapter_error(CertificationVenueError::UnknownOrder))?;
        let order = self.orders[index];
        match scenario {
            CertificationScenario::ReplaceAck
                if req.quantity.0 < order.cumulative_qty.0
                    || req.quantity.0 <= 0
                    || req.limit_price.0 <= 0 =>
            {
                return Err(Self::adapter_error(
                    CertificationVenueError::InvalidScenario,
                ));
            }
            CertificationScenario::CancelReplaceRace {
                fill_quantity,
                fill_price,
                outcome,
            } => {
                let projected_cumulative = order.cumulative_qty.0.saturating_add(fill_quantity.0);
                if fill_quantity.0 <= 0
                    || fill_quantity.0 > order.leaves_qty.0
                    || fill_price.0 <= 0
                    || (outcome == CertificationRaceOutcome::Ack
                        && (req.quantity.0 < projected_cumulative
                            || req.quantity.0 <= 0
                            || req.limit_price.0 <= 0))
                {
                    return Err(Self::adapter_error(
                        CertificationVenueError::InvalidScenario,
                    ));
                }
            }
            _ => {}
        }
        self.consume_scenario();
        let first_sequence = self.next_report_sequence;
        if let CertificationScenario::CancelReplaceRace {
            fill_quantity,
            fill_price,
            outcome,
        } = scenario
        {
            let order = self.orders[index];
            let fill = Self::fill_event(
                order,
                fill_quantity,
                fill_price,
                self.execution_id(),
                req.ts_recv_ns,
            );
            Self::apply_fill(&mut self.orders[index], fill);
            self.deliver_generated(fill, out)?;
            self.finish_amend(
                index,
                req,
                outcome == CertificationRaceOutcome::Ack,
                ExecutionText::new("replace lost race").expect("literal fits"),
                out,
                req.ts_recv_ns.saturating_add(1),
            )?;
        } else {
            match scenario {
                CertificationScenario::ReplaceAck => {
                    self.finish_amend(
                        index,
                        req,
                        true,
                        ExecutionText::empty(),
                        out,
                        req.ts_recv_ns,
                    )?;
                }
                CertificationScenario::ReplaceReject { text } => {
                    self.finish_amend(index, req, false, text, out, req.ts_recv_ns)?;
                }
                _ => unreachable!(),
            }
        }
        self.record_step(
            scenario.kind(),
            CertificationCommandKind::Amend,
            CertificationStepResult::Applied,
            first_sequence,
            report_count,
        );
        Ok(())
    }

    fn poll(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        self.poll_index = self.poll_index.saturating_add(1);
        let mut count = self.drain_ready(out)?;
        if self
            .scenarios
            .front()
            .is_some_and(|scenario| scenario.expected_command() == CertificationCommandKind::Poll)
        {
            let scenario = self.scenarios.front().copied().expect("front was present");
            self.validate_control(scenario, out)?;
            self.consume_scenario();
            count = count.saturating_add(self.process_control(scenario, out)?);
        }
        Ok(count)
    }

    fn recover_open_orders(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        self.require_connected()?;
        let scenario = self
            .scenario_for(CertificationCommandKind::Recover)
            .map_err(Self::adapter_error)?;
        let open = self.orders.iter().filter(|order| order.is_open()).count();
        self.reserve_generated_reports(open, out)?;
        self.consume_scenario();
        let delivered = if self.delivery_delay_polls == 0 {
            open
        } else {
            0
        };
        let first = if open == 0 {
            0
        } else {
            self.next_report_sequence
        };
        for index in 0..self.orders.len() {
            let order = self.orders[index];
            if !order.is_open() {
                continue;
            }
            let event = Self::status_event(
                order,
                StatusEventSpec {
                    exec_type: ExecutionType::Restated,
                    status: order.status,
                    client_order_id: order.current_client_order_id,
                    orig_client_order_id: ClientOrderId::empty(),
                    leaves_qty: order.leaves_qty,
                    text: ExecutionText::empty(),
                    ts_recv_ns: self.poll_index,
                },
            );
            self.deliver_generated(event, out)?;
        }
        self.record_step(
            scenario.kind(),
            CertificationCommandKind::Recover,
            CertificationStepResult::Applied,
            first,
            open,
        );
        Ok(delivered)
    }

    fn capabilities(&self) -> ExecutionCapabilities {
        ExecutionCapabilities::simulated()
    }

    fn health(&self) -> ExecutionHealth {
        ExecutionHealth {
            connected: self.connected,
            degraded: self.degraded,
            health_seq: self.health_seq,
            last_error: self.last_error.clone(),
            protocol_info: Some("deterministic-certification-venue".to_string()),
        }
    }
}

impl CertificationVenue {
    fn finish_amend(
        &mut self,
        index: usize,
        req: &AmendRequest,
        accepted: bool,
        rejection_text: ExecutionText,
        out: &mut ExecutionEventBuffer,
        ts_recv_ns: u64,
    ) -> ExecutionResult<()> {
        let order = self.orders[index];
        if accepted
            && (req.quantity.0 < order.cumulative_qty.0
                || req.quantity.0 <= 0
                || req.limit_price.0 <= 0)
        {
            return Err(Self::adapter_error(
                CertificationVenueError::InvalidScenario,
            ));
        }
        let (exec_type, status, leaves, text) = if accepted {
            (
                ExecutionType::ReplaceAck,
                OrderStatus::Replaced,
                OrderQty(req.quantity.0 - order.cumulative_qty.0),
                ExecutionText::empty(),
            )
        } else {
            (
                ExecutionType::ReplaceReject,
                order.status,
                order.leaves_qty,
                rejection_text,
            )
        };
        let event = Self::status_event(
            order,
            StatusEventSpec {
                exec_type,
                status,
                client_order_id: req.client_order_id,
                orig_client_order_id: req.orig_client_order_id,
                leaves_qty: leaves,
                text,
                ts_recv_ns,
            },
        );
        if accepted {
            self.orders[index].current_client_order_id = req.client_order_id;
            self.orders[index].request.quantity = req.quantity;
            self.orders[index].request.limit_price = req.limit_price;
            self.orders[index].leaves_qty = leaves;
            self.orders[index].status = OrderStatus::Replaced;
        }
        self.deliver_generated(event, out)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use of_execution_core::{
        AccountId, ExecutionSymbol, OrderSide, OrderType, RouteId, StrategyId, TimeInForce,
    };

    fn request(id: &str, quantity: i64) -> OrderRequest {
        OrderRequest {
            client_order_id: ClientOrderId::new(id).unwrap(),
            account_id: AccountId::new("acct").unwrap(),
            route_id: RouteId::new("cert").unwrap(),
            strategy_id: StrategyId::new("test").unwrap(),
            symbol: ExecutionSymbol::new("CME", "ESM6").unwrap(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(quantity),
            limit_price: OrderPrice(100),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 10,
            ts_recv_ns: 11,
        }
    }

    fn cancel(id: &str, orig: &str) -> CancelRequest {
        CancelRequest {
            client_order_id: ClientOrderId::new(id).unwrap(),
            orig_client_order_id: ClientOrderId::new(orig).unwrap(),
            venue_order_id: VenueOrderId::empty(),
            account_id: AccountId::new("acct").unwrap(),
            route_id: RouteId::new("cert").unwrap(),
            symbol: ExecutionSymbol::new("CME", "ESM6").unwrap(),
            ts_recv_ns: 20,
        }
    }

    fn amend(id: &str, orig: &str, quantity: i64) -> AmendRequest {
        AmendRequest {
            client_order_id: ClientOrderId::new(id).unwrap(),
            orig_client_order_id: ClientOrderId::new(orig).unwrap(),
            venue_order_id: VenueOrderId::empty(),
            account_id: AccountId::new("acct").unwrap(),
            route_id: RouteId::new("cert").unwrap(),
            symbol: ExecutionSymbol::new("CME", "ESM6").unwrap(),
            quantity: OrderQty(quantity),
            limit_price: OrderPrice(101),
            ts_recv_ns: 21,
        }
    }

    #[test]
    fn scripted_submit_outcomes_are_deterministic() {
        let mut venue = CertificationVenue::default();
        venue
            .enqueue_all([
                CertificationScenario::Accept,
                CertificationScenario::PartialFill {
                    quantity: OrderQty(4),
                    price: OrderPrice(101),
                },
                CertificationScenario::FullFill {
                    price: OrderPrice(102),
                },
                CertificationScenario::Reject {
                    reason: RiskRejectReason::RouteDisabled,
                    text: ExecutionText::new("closed").unwrap(),
                },
            ])
            .unwrap();
        venue.connect().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        venue.submit(&request("A", 10), &mut out).unwrap();
        venue.submit(&request("B", 10), &mut out).unwrap();
        venue.submit(&request("C", 10), &mut out).unwrap();
        venue.submit(&request("D", 10), &mut out).unwrap();
        assert_eq!(out.len(), 6);
        assert_eq!(out.as_slice()[2].order_status, OrderStatus::PartiallyFilled);
        assert_eq!(out.as_slice()[4].order_status, OrderStatus::Filled);
        assert_eq!(out.as_slice()[5].exec_type, ExecutionType::Reject);
        assert_eq!(venue.snapshot().next_report_sequence(), 7);
    }

    #[test]
    fn cancel_replace_and_races_preserve_order_state() {
        let mut venue = CertificationVenue::default();
        venue
            .enqueue_all([
                CertificationScenario::Accept,
                CertificationScenario::ReplaceAck,
                CertificationScenario::ReplaceReject {
                    text: ExecutionText::new("no").unwrap(),
                },
                CertificationScenario::CancelReplaceRace {
                    fill_quantity: OrderQty(2),
                    fill_price: OrderPrice(102),
                    outcome: CertificationRaceOutcome::Reject,
                },
                CertificationScenario::CancelAck,
            ])
            .unwrap();
        venue.connect().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(16);
        venue.submit(&request("A", 10), &mut out).unwrap();
        venue.amend(&amend("A1", "A", 12), &mut out).unwrap();
        venue.amend(&amend("A2", "A1", 11), &mut out).unwrap();
        venue.cancel(&cancel("CX1", "A1"), &mut out).unwrap();
        venue.cancel(&cancel("CX2", "A1"), &mut out).unwrap();
        assert_eq!(out.as_slice()[3].exec_type, ExecutionType::Trade);
        assert_eq!(out.as_slice()[4].exec_type, ExecutionType::CancelReject);
        assert_eq!(out.as_slice()[5].order_status, OrderStatus::Cancelled);
    }

    #[test]
    fn control_scenarios_preserve_replay_sequence_identity() {
        let mut venue = CertificationVenue::default();
        venue
            .enqueue_all([
                CertificationScenario::Accept,
                CertificationScenario::DuplicateReports { copies: 2 },
                CertificationScenario::OutOfOrderReports,
                CertificationScenario::Resend { from_sequence: 1 },
                CertificationScenario::SequenceReset { next_sequence: 50 },
                CertificationScenario::FullFill {
                    price: OrderPrice(100),
                },
            ])
            .unwrap();
        venue.connect().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(16);
        venue.submit(&request("A", 10), &mut out).unwrap();
        assert_eq!(venue.poll(&mut out).unwrap(), 2);
        assert!(venue.poll(&mut out).is_err());
        venue.clear_script();
        venue
            .enqueue_all([
                CertificationScenario::Accept,
                CertificationScenario::OutOfOrderReports,
                CertificationScenario::Resend { from_sequence: 1 },
                CertificationScenario::SequenceReset { next_sequence: 50 },
                CertificationScenario::FullFill {
                    price: OrderPrice(100),
                },
            ])
            .unwrap();
        venue.submit(&request("B", 10), &mut out).unwrap();
        assert_eq!(venue.poll(&mut out).unwrap(), 2);
        assert_eq!(venue.poll(&mut out).unwrap(), 2);
        assert_eq!(venue.poll(&mut out).unwrap(), 0);
        venue.submit(&request("C", 10), &mut out).unwrap();
        let sequences: Vec<_> = venue
            .retained_reports()
            .map(|report| report.sequence())
            .collect();
        assert!(sequences.contains(&50));
    }

    #[test]
    fn delay_disconnect_reconnect_and_malformed_are_explicit() {
        let mut venue = CertificationVenue::default();
        venue
            .enqueue_all([
                CertificationScenario::SlowVenue { polls: 2 },
                CertificationScenario::Accept,
                CertificationScenario::Disconnect,
                CertificationScenario::Reconnect,
                CertificationScenario::MalformedProviderResponse {
                    text: ExecutionText::new("bad frame").unwrap(),
                },
            ])
            .unwrap();
        venue.connect().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        assert_eq!(venue.poll(&mut out).unwrap(), 0);
        venue.submit(&request("A", 10), &mut out).unwrap();
        assert!(out.is_empty());
        assert_eq!(venue.poll(&mut out).unwrap(), 0);
        assert!(!venue.health().connected);
        assert_eq!(venue.poll(&mut out).unwrap(), 1);
        assert!(venue.health().connected);
        assert!(venue.poll(&mut out).is_err());
        assert!(venue.health().degraded);
    }

    #[test]
    fn recovery_restates_only_open_orders() {
        let mut venue = CertificationVenue::default();
        venue
            .enqueue_all([
                CertificationScenario::Accept,
                CertificationScenario::FullFill {
                    price: OrderPrice(100),
                },
                CertificationScenario::RecoveryRestatement,
            ])
            .unwrap();
        venue.connect().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        venue.submit(&request("A", 10), &mut out).unwrap();
        venue.submit(&request("B", 10), &mut out).unwrap();
        assert_eq!(venue.recover_open_orders(&mut out).unwrap(), 1);
        assert_eq!(
            out.as_slice().last().unwrap().exec_type,
            ExecutionType::Restated
        );
    }

    #[test]
    fn script_order_and_bounds_fail_closed() {
        let config = CertificationVenueConfig::new(1, 1, 1, 1, 1).unwrap();
        let mut venue = CertificationVenue::new(config).unwrap();
        venue.enqueue(CertificationScenario::Accept).unwrap();
        assert_eq!(
            venue.enqueue(CertificationScenario::Accept),
            Err(CertificationVenueError::ScenarioBufferFull)
        );
        venue.connect().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(1);
        assert!(venue.cancel(&cancel("C", "A"), &mut out).is_err());
        venue.submit(&request("A", 1), &mut out).unwrap();
        assert!(venue.submit(&request("B", 1), &mut out).is_err());
    }

    #[test]
    fn complete_certification_script_covers_every_builtin_scenario() {
        let mut venue = CertificationVenue::default();
        venue
            .enqueue_all([
                CertificationScenario::Accept,
                CertificationScenario::CancelReject {
                    text: ExecutionText::new("working").unwrap(),
                },
                CertificationScenario::ReplaceAck,
                CertificationScenario::ReplaceReject {
                    text: ExecutionText::new("price band").unwrap(),
                },
                CertificationScenario::CancelReplaceRace {
                    fill_quantity: OrderQty(1),
                    fill_price: OrderPrice(101),
                    outcome: CertificationRaceOutcome::Reject,
                },
                CertificationScenario::CancelAck,
                CertificationScenario::PartialFill {
                    quantity: OrderQty(2),
                    price: OrderPrice(101),
                },
                CertificationScenario::FullFill {
                    price: OrderPrice(102),
                },
                CertificationScenario::Reject {
                    reason: RiskRejectReason::RouteDisabled,
                    text: ExecutionText::new("closed").unwrap(),
                },
                CertificationScenario::SlowVenue { polls: 1 },
                CertificationScenario::Accept,
                CertificationScenario::DuplicateReports { copies: 1 },
                CertificationScenario::OutOfOrderReports,
                CertificationScenario::Resend { from_sequence: 1 },
                CertificationScenario::SequenceReset { next_sequence: 100 },
                CertificationScenario::Disconnect,
                CertificationScenario::Reconnect,
                CertificationScenario::MalformedProviderResponse {
                    text: ExecutionText::new("invalid provider checksum").unwrap(),
                },
                CertificationScenario::RecoveryRestatement,
            ])
            .unwrap();
        venue.connect().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(128);

        venue.submit(&request("A", 10), &mut out).unwrap();
        venue.cancel(&cancel("CA", "A"), &mut out).unwrap();
        venue.amend(&amend("A1", "A", 12), &mut out).unwrap();
        venue.amend(&amend("A2", "A1", 12), &mut out).unwrap();
        venue.cancel(&cancel("CA2", "A1"), &mut out).unwrap();
        venue.cancel(&cancel("CA3", "A1"), &mut out).unwrap();
        venue.submit(&request("B", 10), &mut out).unwrap();
        venue.submit(&request("C", 10), &mut out).unwrap();
        venue.submit(&request("D", 10), &mut out).unwrap();
        venue.poll(&mut out).unwrap();
        venue.submit(&request("E", 10), &mut out).unwrap();
        venue.poll(&mut out).unwrap();
        venue.poll(&mut out).unwrap();
        venue.poll(&mut out).unwrap();
        venue.poll(&mut out).unwrap();
        venue.poll(&mut out).unwrap();
        venue.poll(&mut out).unwrap();
        assert!(venue.poll(&mut out).is_err());
        venue.recover_open_orders(&mut out).unwrap();

        let coverage = venue.snapshot().coverage();
        assert!(coverage.is_complete());
        for kind in CertificationScenarioKind::ALL {
            assert!(coverage.contains(kind));
            assert!(coverage.count(kind) >= 1);
        }
        assert_eq!(venue.snapshot().remaining_scenarios(), 0);
    }

    #[test]
    fn invalid_scenario_does_not_consume_script_or_emit_partial_reports() {
        let mut venue = CertificationVenue::default();
        venue
            .enqueue(CertificationScenario::PartialFill {
                quantity: OrderQty(10),
                price: OrderPrice(100),
            })
            .unwrap();
        venue.connect().unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(4);

        assert!(venue.submit(&request("A", 10), &mut out).is_err());
        assert!(out.is_empty());
        assert_eq!(venue.snapshot().remaining_scenarios(), 1);
        assert_eq!(venue.snapshot().next_report_sequence(), 1);
    }
}
