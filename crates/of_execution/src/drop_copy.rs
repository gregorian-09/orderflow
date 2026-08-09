//! Independent drop-copy ingestion and reconciliation primitives.

use std::collections::{HashMap, VecDeque};

use of_execution_core::{
    ClientOrderId, ExecutionCoreError, ExecutionEvent, ExecutionText, FixedAscii, OrderQty,
    OrderState, VenueOrderId,
};

use crate::{ExecutionError, ExecutionResult};

/// Maximum bytes stored in a drop-copy source identifier.
pub const DROP_COPY_SOURCE_ID_CAPACITY: usize = 32;
/// Maximum bytes stored in a provider report identifier.
pub const DROP_COPY_REPORT_ID_CAPACITY: usize = 64;

/// Stable identifier for an independent drop-copy source or session.
pub type DropCopySourceId = FixedAscii<DROP_COPY_SOURCE_ID_CAPACITY>;
/// Provider-assigned identifier used to deduplicate drop-copy reports.
pub type DropCopyReportId = FixedAscii<DROP_COPY_REPORT_ID_CAPACITY>;

/// Canonical report emitted by a drop-copy adapter.
///
/// A provider adapter maps its wire representation into the embedded canonical
/// [`ExecutionEvent`]. `source_sequence` should carry a monotonic session or
/// stream sequence when the provider exposes one. It is used as the duplicate
/// key when `report_id` is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropCopyReport {
    source_id: DropCopySourceId,
    report_id: DropCopyReportId,
    source_sequence: u64,
    received_ns: u64,
    event: ExecutionEvent,
}

impl DropCopyReport {
    /// Creates a canonical drop-copy report.
    pub const fn new(
        source_id: DropCopySourceId,
        report_id: DropCopyReportId,
        source_sequence: u64,
        received_ns: u64,
        event: ExecutionEvent,
    ) -> Self {
        Self {
            source_id,
            report_id,
            source_sequence,
            received_ns,
            event,
        }
    }

    /// Creates a report from ASCII source and report identifiers.
    ///
    /// # Errors
    ///
    /// Returns an identifier error when either value is non-ASCII or exceeds
    /// its fixed capacity.
    pub fn from_ids(
        source_id: &str,
        report_id: &str,
        source_sequence: u64,
        received_ns: u64,
        event: ExecutionEvent,
    ) -> Result<Self, ExecutionCoreError> {
        Ok(Self::new(
            DropCopySourceId::new(source_id)?,
            DropCopyReportId::new(report_id)?,
            source_sequence,
            received_ns,
            event,
        ))
    }

    /// Returns the source/session identifier.
    pub const fn source_id(&self) -> DropCopySourceId {
        self.source_id
    }

    /// Returns the provider report identifier.
    pub const fn report_id(&self) -> DropCopyReportId {
        self.report_id
    }

    /// Returns the provider session or stream sequence.
    pub const fn source_sequence(&self) -> u64 {
        self.source_sequence
    }

    /// Returns the local drop-copy receive timestamp in nanoseconds.
    pub const fn received_ns(&self) -> u64 {
        self.received_ns
    }

    /// Returns the canonical execution event.
    pub const fn event(&self) -> &ExecutionEvent {
        &self.event
    }

    fn duplicate_key(&self) -> Option<DropCopyDuplicateKey> {
        if !self.report_id.is_empty() {
            Some(DropCopyDuplicateKey::ReportId {
                source_id: self.source_id,
                report_id: self.report_id,
            })
        } else if self.source_sequence != 0 {
            Some(DropCopyDuplicateKey::SourceSequence {
                source_id: self.source_id,
                source_sequence: self.source_sequence,
            })
        } else {
            None
        }
    }
}

/// Caller-owned bounded buffer used by drop-copy adapters.
#[derive(Debug, Clone)]
pub struct DropCopyReportBuffer {
    reports: Vec<DropCopyReport>,
    max_len: usize,
}

impl DropCopyReportBuffer {
    /// Creates an empty report buffer with a hard maximum length.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            reports: Vec::with_capacity(capacity),
            max_len: capacity,
        }
    }

    /// Appends one report without growing beyond the configured capacity.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::BufferFull`] when the buffer is full.
    pub fn push(&mut self, report: DropCopyReport) -> ExecutionResult<()> {
        if self.reports.len() >= self.max_len {
            return Err(ExecutionError::BufferFull);
        }
        self.reports.push(report);
        Ok(())
    }

    /// Returns buffered reports.
    pub fn as_slice(&self) -> &[DropCopyReport] {
        &self.reports
    }

    /// Clears reports without releasing allocated storage.
    pub fn clear(&mut self) {
        self.reports.clear();
    }

    /// Returns the number of buffered reports.
    pub fn len(&self) -> usize {
        self.reports.len()
    }

    /// Returns true when no reports are buffered.
    pub fn is_empty(&self) -> bool {
        self.reports.is_empty()
    }

    /// Returns the configured maximum report count.
    pub const fn max_len(&self) -> usize {
        self.max_len
    }

    /// Returns the number of reports that can be appended without overflow.
    pub fn remaining_capacity(&self) -> usize {
        self.max_len.saturating_sub(self.reports.len())
    }
}

impl Default for DropCopyReportBuffer {
    fn default() -> Self {
        Self::with_capacity(64)
    }
}

/// Independent drop-copy transport/session state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DropCopySourceState {
    /// Transport is disconnected.
    #[default]
    Disconnected = 0,
    /// Transport connection or protocol logon is in progress.
    Connecting = 1,
    /// Source is ready to emit reports.
    Ready = 2,
    /// Source is connected but degraded.
    Degraded = 3,
    /// Source is recovering a sequence gap or replay.
    Recovering = 4,
    /// Source was stopped intentionally.
    Stopped = 5,
}

/// Health snapshot for one independent drop-copy source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropCopySourceHealth {
    source_id: DropCopySourceId,
    state: DropCopySourceState,
    health_sequence: u64,
    reports_received: u64,
    last_received_ns: u64,
    last_error: ExecutionText,
}

impl DropCopySourceHealth {
    /// Creates a disconnected source health snapshot.
    pub const fn new(source_id: DropCopySourceId) -> Self {
        Self {
            source_id,
            state: DropCopySourceState::Disconnected,
            health_sequence: 0,
            reports_received: 0,
            last_received_ns: 0,
            last_error: ExecutionText::empty(),
        }
    }

    /// Sets lifecycle state and the monotonic transition sequence.
    pub const fn with_state(mut self, state: DropCopySourceState, health_sequence: u64) -> Self {
        self.state = state;
        self.health_sequence = health_sequence;
        self
    }

    /// Sets report progress counters supplied by the adapter.
    pub const fn with_report_progress(
        mut self,
        reports_received: u64,
        last_received_ns: u64,
    ) -> Self {
        self.reports_received = reports_received;
        self.last_received_ns = last_received_ns;
        self
    }

    /// Sets bounded diagnostic text, or clears it with an empty value.
    pub const fn with_last_error(mut self, last_error: ExecutionText) -> Self {
        self.last_error = last_error;
        self
    }

    /// Returns the source identifier.
    pub const fn source_id(&self) -> DropCopySourceId {
        self.source_id
    }

    /// Returns the current source state.
    pub const fn state(&self) -> DropCopySourceState {
        self.state
    }

    /// Returns the monotonic health transition sequence.
    pub const fn health_sequence(&self) -> u64 {
        self.health_sequence
    }

    /// Returns the number of reports received by the adapter.
    pub const fn reports_received(&self) -> u64 {
        self.reports_received
    }

    /// Returns the latest local report-receive timestamp.
    pub const fn last_received_ns(&self) -> u64 {
        self.last_received_ns
    }

    /// Returns bounded diagnostic text, or an empty value when healthy.
    pub const fn last_error(&self) -> ExecutionText {
        self.last_error
    }

    /// Returns true when the source is ready to emit current reports.
    pub const fn is_ready(&self) -> bool {
        matches!(self.state, DropCopySourceState::Ready)
    }
}

/// Provider-neutral contract for an independent drop-copy session.
///
/// Implementations own transport, protocol sequencing, and provider-specific
/// decoding. They emit canonical reports into a caller-owned bounded buffer.
pub trait DropCopyAdapter: Send {
    /// Establishes transport and protocol/session state.
    fn connect(&mut self) -> ExecutionResult<()>;
    /// Stops the independent source session.
    fn disconnect(&mut self) -> ExecutionResult<()>;
    /// Drains currently available canonical reports.
    fn poll(&mut self, out: &mut DropCopyReportBuffer) -> ExecutionResult<usize>;
    /// Returns current source health.
    fn health(&self) -> DropCopySourceHealth;
}

impl DropCopyAdapter for Box<dyn DropCopyAdapter> {
    fn connect(&mut self) -> ExecutionResult<()> {
        self.as_mut().connect()
    }

    fn disconnect(&mut self) -> ExecutionResult<()> {
        self.as_mut().disconnect()
    }

    fn poll(&mut self, out: &mut DropCopyReportBuffer) -> ExecutionResult<usize> {
        self.as_mut().poll(out)
    }

    fn health(&self) -> DropCopySourceHealth {
        self.as_ref().health()
    }
}

/// Deterministic bounded drop-copy source for tests, replay, and bridges.
#[derive(Debug)]
pub struct InMemoryDropCopyAdapter {
    reports: VecDeque<DropCopyReport>,
    capacity: usize,
    health: DropCopySourceHealth,
}

impl InMemoryDropCopyAdapter {
    /// Creates an empty source with a bounded pending-report queue.
    pub fn new(source_id: DropCopySourceId, capacity: usize) -> Self {
        Self {
            reports: VecDeque::with_capacity(capacity),
            capacity,
            health: DropCopySourceHealth::new(source_id),
        }
    }

    /// Queues a canonical report without exceeding the configured bound.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::BufferFull`] when the source queue is full.
    pub fn enqueue(&mut self, report: DropCopyReport) -> ExecutionResult<()> {
        if report.source_id != self.health.source_id {
            return Err(ExecutionError::Adapter(
                "drop-copy report source does not match adapter source".to_string(),
            ));
        }
        if self.reports.len() >= self.capacity {
            return Err(ExecutionError::BufferFull);
        }
        self.health.reports_received = self.health.reports_received.saturating_add(1);
        self.health.last_received_ns = report.received_ns;
        self.reports.push_back(report);
        Ok(())
    }

    /// Returns the number of queued reports awaiting a poll.
    pub fn pending_reports(&self) -> usize {
        self.reports.len()
    }

    /// Transitions the source to a degraded state with bounded diagnostic text.
    pub fn mark_degraded(&mut self, error: ExecutionText) {
        self.transition(DropCopySourceState::Degraded, error);
    }

    fn transition(&mut self, state: DropCopySourceState, error: ExecutionText) {
        if self.health.state != state || self.health.last_error != error {
            self.health.health_sequence = self.health.health_sequence.saturating_add(1);
        }
        self.health.state = state;
        self.health.last_error = error;
    }
}

impl DropCopyAdapter for InMemoryDropCopyAdapter {
    fn connect(&mut self) -> ExecutionResult<()> {
        self.transition(DropCopySourceState::Ready, ExecutionText::empty());
        Ok(())
    }

    fn disconnect(&mut self) -> ExecutionResult<()> {
        self.transition(DropCopySourceState::Stopped, ExecutionText::empty());
        Ok(())
    }

    fn poll(&mut self, out: &mut DropCopyReportBuffer) -> ExecutionResult<usize> {
        if !self.health.is_ready() {
            return Err(ExecutionError::Disconnected);
        }
        let count = out.remaining_capacity().min(self.reports.len());
        for _ in 0..count {
            let report = self
                .reports
                .pop_front()
                .expect("count is bounded by pending reports");
            out.push(report)?;
        }
        Ok(count)
    }

    fn health(&self) -> DropCopySourceHealth {
        self.health
    }
}

/// Policy for reports that regress source time or cumulative fill quantity.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DropCopyLateReportPolicy {
    /// Keep the report eligible for reconciliation and flag it as late.
    AcceptAndFlag = 0,
    /// Retain the report for audit but do not treat it as current state.
    #[default]
    AuditOnly = 1,
    /// Reject the report from downstream reconciliation.
    Reject = 2,
}

/// Recommended handling after duplicate and late-report checks.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropCopyDisposition {
    /// Report is current and eligible for reconciliation.
    Current = 0,
    /// Report duplicates a previously observed source identity.
    Duplicate = 1,
    /// Late report remains eligible but carries explicit issue flags.
    LateAccepted = 2,
    /// Late report should be retained only as audit evidence.
    LateAuditOnly = 3,
    /// Late report should not be used for reconciliation.
    LateRejected = 4,
}

/// Correlation result between drop-copy evidence and local OMS state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropCopyCorrelation {
    /// Report state agrees with the correlated local order.
    Matched = 0,
    /// An order correlated but one or more state fields disagree.
    Mismatch = 1,
    /// No local order could be correlated by venue or client identifier.
    VenueOnly = 2,
    /// Duplicate reports are not reconciled again.
    NotEvaluated = 3,
}

/// Allocation-free bitset describing drop-copy reconciliation issues.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DropCopyIssueFlags(u32);

impl DropCopyIssueFlags {
    /// No issues were observed.
    pub const NONE: Self = Self(0);
    /// Neither report id nor source sequence was available for deduplication.
    pub const MISSING_DUPLICATE_KEY: Self = Self(1 << 0);
    /// Client-order identifiers disagree.
    pub const CLIENT_ORDER_ID_MISMATCH: Self = Self(1 << 1);
    /// Venue-order identifiers disagree.
    pub const VENUE_ORDER_ID_MISMATCH: Self = Self(1 << 2);
    /// Trading account differs.
    pub const ACCOUNT_MISMATCH: Self = Self(1 << 3);
    /// Route differs.
    pub const ROUTE_MISMATCH: Self = Self(1 << 4);
    /// Venue or instrument symbol differs.
    pub const SYMBOL_MISMATCH: Self = Self(1 << 5);
    /// Order status differs.
    pub const STATUS_MISMATCH: Self = Self(1 << 6);
    /// Cumulative fill quantity differs.
    pub const CUMULATIVE_QTY_MISMATCH: Self = Self(1 << 7);
    /// Remaining quantity differs.
    pub const LEAVES_QTY_MISMATCH: Self = Self(1 << 8);
    /// Average fill price differs.
    pub const AVERAGE_PRICE_MISMATCH: Self = Self(1 << 9);
    /// Trade cumulative and leaves quantities do not match local order size.
    pub const INVALID_FILL_TOTAL: Self = Self(1 << 10);
    /// Exchange timestamp regressed relative to this drop-copy order stream.
    pub const LATE_TIMESTAMP: Self = Self(1 << 11);
    /// Cumulative fill quantity regressed relative to prior drop-copy evidence.
    pub const REGRESSIVE_FILL: Self = Self(1 << 12);
    /// Bounded progress tracking was full for a new order identity.
    pub const TRACKING_CAPACITY_EXHAUSTED: Self = Self(1 << 13);

    /// Returns raw issue bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Returns true when all bits in `other` are present.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns true when no issue is present.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    fn without_operational(self) -> Self {
        Self(
            self.0
                & !(Self::MISSING_DUPLICATE_KEY.0
                    | Self::LATE_TIMESTAMP.0
                    | Self::REGRESSIVE_FILL.0
                    | Self::TRACKING_CAPACITY_EXHAUSTED.0),
        )
    }
}

impl std::ops::BitOr for DropCopyIssueFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for DropCopyIssueFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.insert(rhs);
    }
}

/// Result of observing one canonical drop-copy report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropCopyObservation {
    /// Recommended handling for the report.
    pub disposition: DropCopyDisposition,
    /// Local-state correlation classification.
    pub correlation: DropCopyCorrelation,
    /// Detailed mismatch and operational issue flags.
    pub issues: DropCopyIssueFlags,
    /// Correlated local client-order id, or empty for venue-only evidence.
    pub local_client_order_id: ClientOrderId,
    /// Receive-minus-exchange latency when both timestamps are usable.
    pub lag_ns: u64,
}

impl DropCopyObservation {
    /// Returns true when the report is unique and current enough to reconcile.
    pub const fn reconciliation_eligible(&self) -> bool {
        matches!(
            self.disposition,
            DropCopyDisposition::Current | DropCopyDisposition::LateAccepted
        )
    }

    /// Returns true when local and drop-copy order state disagree.
    pub const fn has_state_mismatch(&self) -> bool {
        matches!(self.correlation, DropCopyCorrelation::Mismatch)
    }
}

/// Allocation-free drop-copy ingestion and reconciliation metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct DropCopyMetricsSnapshot {
    /// All reports presented to the reconciler.
    pub reports_received: u64,
    /// Reports accepted as unique.
    pub unique_reports: u64,
    /// Reports suppressed as duplicates.
    pub duplicate_reports: u64,
    /// Reports with late timestamp or regressive fill evidence.
    pub late_reports: u64,
    /// Reports whose local state matched.
    pub matched_reports: u64,
    /// Reports whose local state differed.
    pub mismatched_reports: u64,
    /// Reports with no correlated local order.
    pub venue_only_reports: u64,
    /// Unique trade/fill reports observed.
    pub fill_reports: u64,
    /// Reports that could not be deduplicated.
    pub reports_without_duplicate_key: u64,
    /// New order identities omitted because progress capacity was full.
    pub tracking_capacity_exhaustions: u64,
    /// Most recent receive-minus-exchange latency.
    pub current_lag_ns: u64,
    /// Maximum receive-minus-exchange latency observed.
    pub max_lag_ns: u64,
    /// Latest caller-supplied local receive timestamp.
    pub last_received_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DropCopyDuplicateKey {
    ReportId {
        source_id: DropCopySourceId,
        report_id: DropCopyReportId,
    },
    SourceSequence {
        source_id: DropCopySourceId,
        source_sequence: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DropCopyOrderKey {
    Venue(VenueOrderId),
    Client(ClientOrderId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DropCopyProgress {
    exchange_ns: u64,
    cumulative_qty: OrderQty,
}

#[derive(Debug)]
struct BoundedDuplicateSet {
    keys: HashMap<DropCopyDuplicateKey, ()>,
    order: Vec<DropCopyDuplicateKey>,
    cursor: usize,
    capacity: usize,
}

impl BoundedDuplicateSet {
    fn new(capacity: usize) -> Self {
        Self {
            keys: HashMap::with_capacity(capacity),
            order: Vec::with_capacity(capacity),
            cursor: 0,
            capacity,
        }
    }

    fn observe(&mut self, key: DropCopyDuplicateKey) -> bool {
        if self.keys.contains_key(&key) {
            return true;
        }
        if self.capacity == 0 {
            return false;
        }
        if self.order.len() < self.capacity {
            self.order.push(key);
        } else {
            let evicted = std::mem::replace(&mut self.order[self.cursor], key);
            self.keys.remove(&evicted);
            self.cursor = (self.cursor + 1) % self.capacity;
        }
        self.keys.insert(key, ());
        false
    }

    fn clear(&mut self) {
        self.keys.clear();
        self.order.clear();
        self.cursor = 0;
    }
}

/// Bounded low-allocation drop-copy deduplicator and state reconciler.
///
/// `replace_local_orders` is a control-plane operation and may grow internal
/// indexes when the supplied order count exceeds `local_order_capacity`.
/// `observe` performs no intentional allocation while report, index, and
/// progress cardinality remain within the capacities supplied to [`Self::new`].
#[derive(Debug)]
pub struct DropCopyReconciler {
    duplicate_reports: BoundedDuplicateSet,
    local_states: Vec<OrderState>,
    by_client_id: HashMap<ClientOrderId, usize>,
    by_venue_id: HashMap<VenueOrderId, usize>,
    progress: HashMap<DropCopyOrderKey, DropCopyProgress>,
    progress_capacity: usize,
    late_policy: DropCopyLateReportPolicy,
    metrics: DropCopyMetricsSnapshot,
}

impl DropCopyReconciler {
    /// Creates an empty reconciler with explicit bounded hot-path capacities.
    pub fn new(
        duplicate_capacity: usize,
        local_order_capacity: usize,
        late_policy: DropCopyLateReportPolicy,
    ) -> Self {
        Self {
            duplicate_reports: BoundedDuplicateSet::new(duplicate_capacity),
            local_states: Vec::with_capacity(local_order_capacity),
            by_client_id: HashMap::with_capacity(local_order_capacity.saturating_mul(2)),
            by_venue_id: HashMap::with_capacity(local_order_capacity),
            progress: HashMap::with_capacity(local_order_capacity),
            progress_capacity: local_order_capacity,
            late_policy,
            metrics: DropCopyMetricsSnapshot::default(),
        }
    }

    /// Rebuilds local correlation indexes from an OMS state snapshot.
    ///
    /// Existing drop-copy duplicate and progress history is preserved. This
    /// allows local state to refresh without making replayed reports current.
    pub fn replace_local_orders(&mut self, states: &[OrderState]) {
        self.local_states.clear();
        self.by_client_id.clear();
        self.by_venue_id.clear();
        self.local_states.extend_from_slice(states);

        for (index, state) in self.local_states.iter().enumerate() {
            if !state.client_order_id.is_empty() {
                self.by_client_id.insert(state.client_order_id, index);
            }
            if !state.last_accepted_client_order_id.is_empty() {
                self.by_client_id
                    .insert(state.last_accepted_client_order_id, index);
            }
            if !state.venue_order_id.is_empty() {
                self.by_venue_id.insert(state.venue_order_id, index);
            }
        }
    }

    /// Observes, deduplicates, correlates, and reconciles one report.
    pub fn observe(&mut self, report: &DropCopyReport) -> DropCopyObservation {
        self.metrics.reports_received = self.metrics.reports_received.saturating_add(1);
        self.metrics.last_received_ns = report.received_ns;

        let lag_ns = if report.event.ts_exchange_ns == 0 {
            0
        } else {
            report
                .received_ns
                .saturating_sub(report.event.ts_exchange_ns)
        };
        self.metrics.current_lag_ns = lag_ns;
        self.metrics.max_lag_ns = self.metrics.max_lag_ns.max(lag_ns);

        let mut issues = DropCopyIssueFlags::NONE;
        if let Some(key) = report.duplicate_key() {
            if self.duplicate_reports.observe(key) {
                self.metrics.duplicate_reports = self.metrics.duplicate_reports.saturating_add(1);
                return DropCopyObservation {
                    disposition: DropCopyDisposition::Duplicate,
                    correlation: DropCopyCorrelation::NotEvaluated,
                    issues,
                    local_client_order_id: ClientOrderId::empty(),
                    lag_ns,
                };
            }
        } else {
            issues.insert(DropCopyIssueFlags::MISSING_DUPLICATE_KEY);
            self.metrics.reports_without_duplicate_key =
                self.metrics.reports_without_duplicate_key.saturating_add(1);
        }

        self.metrics.unique_reports = self.metrics.unique_reports.saturating_add(1);
        if matches!(
            report.event.exec_type,
            of_execution_core::ExecutionType::Trade
        ) {
            self.metrics.fill_reports = self.metrics.fill_reports.saturating_add(1);
        }

        let progress_key = order_key(&report.event);
        let is_late = if let Some(key) = progress_key {
            self.observe_progress(key, &report.event, &mut issues)
        } else {
            false
        };

        let disposition = if is_late {
            self.metrics.late_reports = self.metrics.late_reports.saturating_add(1);
            match self.late_policy {
                DropCopyLateReportPolicy::AcceptAndFlag => DropCopyDisposition::LateAccepted,
                DropCopyLateReportPolicy::AuditOnly => DropCopyDisposition::LateAuditOnly,
                DropCopyLateReportPolicy::Reject => DropCopyDisposition::LateRejected,
            }
        } else {
            DropCopyDisposition::Current
        };

        let local_index = self.correlate(&report.event);
        let (correlation, local_client_order_id) = if let Some(index) = local_index {
            let state = self.local_states[index];
            reconcile_state(&state, &report.event, &mut issues);
            let correlation = if issues.without_operational().is_empty() {
                self.metrics.matched_reports = self.metrics.matched_reports.saturating_add(1);
                DropCopyCorrelation::Matched
            } else {
                self.metrics.mismatched_reports = self.metrics.mismatched_reports.saturating_add(1);
                DropCopyCorrelation::Mismatch
            };
            (correlation, state.client_order_id)
        } else {
            self.metrics.venue_only_reports = self.metrics.venue_only_reports.saturating_add(1);
            (DropCopyCorrelation::VenueOnly, ClientOrderId::empty())
        };

        DropCopyObservation {
            disposition,
            correlation,
            issues,
            local_client_order_id,
            lag_ns,
        }
    }

    /// Returns allocation-free metrics for independent monitoring and SLOs.
    pub const fn metrics(&self) -> DropCopyMetricsSnapshot {
        self.metrics
    }

    /// Clears duplicate and progress history while preserving capacities and
    /// local correlation indexes.
    pub fn reset_stream_history(&mut self) {
        self.duplicate_reports.clear();
        self.progress.clear();
    }

    fn correlate(&self, event: &ExecutionEvent) -> Option<usize> {
        if !event.venue_order_id.is_empty() {
            if let Some(index) = self.by_venue_id.get(&event.venue_order_id) {
                return Some(*index);
            }
        }
        if !event.client_order_id.is_empty() {
            if let Some(index) = self.by_client_id.get(&event.client_order_id) {
                return Some(*index);
            }
        }
        if !event.orig_client_order_id.is_empty() {
            if let Some(index) = self.by_client_id.get(&event.orig_client_order_id) {
                return Some(*index);
            }
        }
        None
    }

    fn observe_progress(
        &mut self,
        key: DropCopyOrderKey,
        event: &ExecutionEvent,
        issues: &mut DropCopyIssueFlags,
    ) -> bool {
        if let Some(previous) = self.progress.get_mut(&key) {
            let late_timestamp = event.ts_exchange_ns != 0
                && previous.exchange_ns != 0
                && event.ts_exchange_ns < previous.exchange_ns;
            let regressive_fill = event.cumulative_qty.0 < previous.cumulative_qty.0;
            if late_timestamp {
                issues.insert(DropCopyIssueFlags::LATE_TIMESTAMP);
            }
            if regressive_fill {
                issues.insert(DropCopyIssueFlags::REGRESSIVE_FILL);
            }
            if !late_timestamp && !regressive_fill {
                previous.exchange_ns = previous.exchange_ns.max(event.ts_exchange_ns);
                previous.cumulative_qty = event.cumulative_qty;
            }
            return late_timestamp || regressive_fill;
        }

        if self.progress.len() >= self.progress_capacity {
            issues.insert(DropCopyIssueFlags::TRACKING_CAPACITY_EXHAUSTED);
            self.metrics.tracking_capacity_exhaustions =
                self.metrics.tracking_capacity_exhaustions.saturating_add(1);
            return false;
        }
        self.progress.insert(
            key,
            DropCopyProgress {
                exchange_ns: event.ts_exchange_ns,
                cumulative_qty: event.cumulative_qty,
            },
        );
        false
    }
}

fn order_key(event: &ExecutionEvent) -> Option<DropCopyOrderKey> {
    if !event.venue_order_id.is_empty() {
        Some(DropCopyOrderKey::Venue(event.venue_order_id))
    } else if !event.client_order_id.is_empty() {
        Some(DropCopyOrderKey::Client(event.client_order_id))
    } else {
        None
    }
}

fn reconcile_state(state: &OrderState, event: &ExecutionEvent, issues: &mut DropCopyIssueFlags) {
    let client_matches = event.client_order_id == state.client_order_id
        || event.client_order_id == state.last_accepted_client_order_id
        || event.orig_client_order_id == state.client_order_id
        || event.orig_client_order_id == state.last_accepted_client_order_id;
    if !client_matches {
        issues.insert(DropCopyIssueFlags::CLIENT_ORDER_ID_MISMATCH);
    }
    if !event.venue_order_id.is_empty()
        && !state.venue_order_id.is_empty()
        && event.venue_order_id != state.venue_order_id
    {
        issues.insert(DropCopyIssueFlags::VENUE_ORDER_ID_MISMATCH);
    }
    if event.account_id != state.account_id {
        issues.insert(DropCopyIssueFlags::ACCOUNT_MISMATCH);
    }
    if event.route_id != state.route_id {
        issues.insert(DropCopyIssueFlags::ROUTE_MISMATCH);
    }
    if event.symbol != state.symbol {
        issues.insert(DropCopyIssueFlags::SYMBOL_MISMATCH);
    }
    if event.order_status != state.status {
        issues.insert(DropCopyIssueFlags::STATUS_MISMATCH);
    }
    if event.cumulative_qty != state.cumulative_qty {
        issues.insert(DropCopyIssueFlags::CUMULATIVE_QTY_MISMATCH);
    }
    if event.leaves_qty != state.leaves_qty {
        issues.insert(DropCopyIssueFlags::LEAVES_QTY_MISMATCH);
    }
    if event.average_price != state.average_price {
        issues.insert(DropCopyIssueFlags::AVERAGE_PRICE_MISMATCH);
    }
    if matches!(event.exec_type, of_execution_core::ExecutionType::Trade)
        && event.cumulative_qty.0.saturating_add(event.leaves_qty.0) != state.order_qty.0
    {
        issues.insert(DropCopyIssueFlags::INVALID_FILL_TOTAL);
    }
}

#[cfg(test)]
mod tests {
    use of_execution_core::{
        ExecutionId, ExecutionSymbol, ExecutionType, OrderPrice, OrderSide, OrderStatus,
    };

    use super::*;

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn state() -> OrderState {
        OrderState {
            client_order_id: id("client-1"),
            last_accepted_client_order_id: id("client-1"),
            venue_order_id: id("venue-1"),
            account_id: id("account-1"),
            route_id: id("route-1"),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            side: OrderSide::Buy,
            status: OrderStatus::PartiallyFilled,
            order_qty: OrderQty(10),
            cumulative_qty: OrderQty(4),
            leaves_qty: OrderQty(6),
            average_price: OrderPrice(5_000),
            updated_ns: 200,
        }
    }

    fn event() -> ExecutionEvent {
        let state = state();
        ExecutionEvent {
            exec_type: ExecutionType::Trade,
            order_status: state.status,
            client_order_id: state.client_order_id,
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: state.venue_order_id,
            execution_id: ExecutionId::new("exec-1").unwrap(),
            account_id: state.account_id,
            route_id: state.route_id,
            symbol: state.symbol,
            last_qty: OrderQty(4),
            last_price: OrderPrice(5_000),
            cumulative_qty: state.cumulative_qty,
            leaves_qty: state.leaves_qty,
            average_price: state.average_price,
            ts_exchange_ns: 100,
            ts_recv_ns: 200,
            reason: of_execution_core::RiskRejectReason::None,
            text: ExecutionText::empty(),
        }
    }

    fn report(report_id: &str, sequence: u64, event: ExecutionEvent) -> DropCopyReport {
        DropCopyReport::from_ids("drop-a", report_id, sequence, 150, event).unwrap()
    }

    #[test]
    fn bounded_buffer_rejects_overflow() {
        let mut buffer = DropCopyReportBuffer::with_capacity(1);
        buffer.push(report("r1", 1, event())).unwrap();
        assert_eq!(
            buffer.push(report("r2", 2, event())),
            Err(ExecutionError::BufferFull)
        );
        assert_eq!(buffer.remaining_capacity(), 0);
        buffer.clear();
        assert_eq!(buffer.max_len(), 1);
    }

    #[test]
    fn in_memory_source_preserves_backpressure() {
        let mut adapter = InMemoryDropCopyAdapter::new(id("drop-a"), 2);
        adapter.enqueue(report("r1", 1, event())).unwrap();
        adapter.enqueue(report("r2", 2, event())).unwrap();
        adapter.connect().unwrap();

        let mut out = DropCopyReportBuffer::with_capacity(1);
        assert_eq!(adapter.poll(&mut out).unwrap(), 1);
        assert_eq!(adapter.pending_reports(), 1);
        assert_eq!(adapter.poll(&mut out).unwrap(), 0);
        assert_eq!(adapter.pending_reports(), 1);
        assert_eq!(adapter.health().reports_received(), 2);
    }

    #[test]
    fn in_memory_source_rejects_cross_source_reports() {
        let mut adapter = InMemoryDropCopyAdapter::new(id("drop-a"), 2);
        let cross_source = DropCopyReport::from_ids("drop-b", "r1", 1, 150, event()).unwrap();
        assert!(matches!(
            adapter.enqueue(cross_source),
            Err(ExecutionError::Adapter(_))
        ));
        assert_eq!(adapter.pending_reports(), 0);
    }

    #[test]
    fn reconciler_matches_canonical_state_and_tracks_lag() {
        let mut reconciler = DropCopyReconciler::new(16, 16, DropCopyLateReportPolicy::AuditOnly);
        reconciler.replace_local_orders(&[state()]);
        let observation = reconciler.observe(&report("r1", 1, event()));

        assert_eq!(observation.disposition, DropCopyDisposition::Current);
        assert_eq!(observation.correlation, DropCopyCorrelation::Matched);
        assert!(observation.issues.is_empty());
        assert_eq!(observation.lag_ns, 50);
        assert_eq!(reconciler.metrics().matched_reports, 1);
        assert_eq!(reconciler.metrics().fill_reports, 1);
    }

    #[test]
    fn duplicate_identity_is_scoped_to_source() {
        let mut reconciler = DropCopyReconciler::new(16, 16, DropCopyLateReportPolicy::AuditOnly);
        reconciler.replace_local_orders(&[state()]);
        let first = report("r1", 1, event());
        assert_eq!(
            reconciler.observe(&first).disposition,
            DropCopyDisposition::Current
        );
        assert_eq!(
            reconciler.observe(&first).disposition,
            DropCopyDisposition::Duplicate
        );

        let other_source = DropCopyReport::from_ids("drop-b", "r1", 1, 150, event()).unwrap();
        assert_eq!(
            reconciler.observe(&other_source).disposition,
            DropCopyDisposition::Current
        );
        assert_eq!(reconciler.metrics().duplicate_reports, 1);
    }

    #[test]
    fn duplicate_window_evicts_oldest_identity_deterministically() {
        let mut reconciler = DropCopyReconciler::new(1, 16, DropCopyLateReportPolicy::AuditOnly);
        assert_eq!(
            reconciler.observe(&report("r1", 1, event())).disposition,
            DropCopyDisposition::Current
        );
        assert_eq!(
            reconciler.observe(&report("r2", 2, event())).disposition,
            DropCopyDisposition::Current
        );
        assert_eq!(
            reconciler.observe(&report("r1", 1, event())).disposition,
            DropCopyDisposition::Current
        );
        assert_eq!(reconciler.metrics().duplicate_reports, 0);
    }

    #[test]
    fn source_sequence_deduplicates_when_report_id_is_absent() {
        let mut reconciler = DropCopyReconciler::new(16, 16, DropCopyLateReportPolicy::AuditOnly);
        let report = report("", 42, event());
        assert_eq!(
            reconciler.observe(&report).disposition,
            DropCopyDisposition::Current
        );
        assert_eq!(
            reconciler.observe(&report).disposition,
            DropCopyDisposition::Duplicate
        );
        assert_eq!(reconciler.metrics().reports_without_duplicate_key, 0);
    }

    #[test]
    fn client_fallback_exposes_venue_id_mismatch() {
        let mut reconciler = DropCopyReconciler::new(16, 16, DropCopyLateReportPolicy::AuditOnly);
        reconciler.replace_local_orders(&[state()]);
        let mut mismatched = event();
        mismatched.venue_order_id = VenueOrderId::new("unknown-venue").unwrap();
        let observation = reconciler.observe(&report("r1", 1, mismatched));

        assert_eq!(observation.correlation, DropCopyCorrelation::Mismatch);
        assert!(observation
            .issues
            .contains(DropCopyIssueFlags::VENUE_ORDER_ID_MISMATCH));
        assert_eq!(observation.local_client_order_id.as_str(), "client-1");
    }

    #[test]
    fn fill_reconciliation_reports_quantity_and_price_differences() {
        let mut reconciler = DropCopyReconciler::new(16, 16, DropCopyLateReportPolicy::AuditOnly);
        reconciler.replace_local_orders(&[state()]);
        let mut mismatched = event();
        mismatched.cumulative_qty = OrderQty(5);
        mismatched.leaves_qty = OrderQty(5);
        mismatched.average_price = OrderPrice(5_001);
        let observation = reconciler.observe(&report("r1", 1, mismatched));

        assert!(observation
            .issues
            .contains(DropCopyIssueFlags::CUMULATIVE_QTY_MISMATCH));
        assert!(observation
            .issues
            .contains(DropCopyIssueFlags::LEAVES_QTY_MISMATCH));
        assert!(observation
            .issues
            .contains(DropCopyIssueFlags::AVERAGE_PRICE_MISMATCH));
    }

    #[test]
    fn late_reports_follow_explicit_policy_without_replacing_progress() {
        let mut reconciler = DropCopyReconciler::new(16, 16, DropCopyLateReportPolicy::AuditOnly);
        reconciler.replace_local_orders(&[state()]);
        let mut current = event();
        current.ts_exchange_ns = 200;
        reconciler.observe(&report("r1", 1, current));

        let mut late = event();
        late.ts_exchange_ns = 100;
        late.cumulative_qty = OrderQty(3);
        late.leaves_qty = OrderQty(7);
        let observation = reconciler.observe(&report("r2", 2, late));
        assert_eq!(observation.disposition, DropCopyDisposition::LateAuditOnly);
        assert!(observation
            .issues
            .contains(DropCopyIssueFlags::LATE_TIMESTAMP));
        assert!(observation
            .issues
            .contains(DropCopyIssueFlags::REGRESSIVE_FILL));

        let mut next = event();
        next.ts_exchange_ns = 210;
        let observation = reconciler.observe(&report("r3", 3, next));
        assert_eq!(observation.disposition, DropCopyDisposition::Current);
        assert_eq!(reconciler.metrics().late_reports, 1);
    }

    #[test]
    fn late_report_policy_controls_reconciliation_eligibility() {
        let cases = [
            (
                DropCopyLateReportPolicy::AcceptAndFlag,
                DropCopyDisposition::LateAccepted,
                true,
            ),
            (
                DropCopyLateReportPolicy::AuditOnly,
                DropCopyDisposition::LateAuditOnly,
                false,
            ),
            (
                DropCopyLateReportPolicy::Reject,
                DropCopyDisposition::LateRejected,
                false,
            ),
        ];

        for (policy, expected, eligible) in cases {
            let mut reconciler = DropCopyReconciler::new(16, 16, policy);
            let mut current = event();
            current.ts_exchange_ns = 200;
            reconciler.observe(&report("current", 1, current));

            let mut late = event();
            late.ts_exchange_ns = 100;
            let observation = reconciler.observe(&report("late", 2, late));
            assert_eq!(observation.disposition, expected);
            assert_eq!(observation.reconciliation_eligible(), eligible);
        }
    }

    #[test]
    fn venue_only_and_progress_capacity_are_visible() {
        let mut reconciler = DropCopyReconciler::new(16, 0, DropCopyLateReportPolicy::Reject);
        let observation = reconciler.observe(&report("r1", 1, event()));
        assert_eq!(observation.correlation, DropCopyCorrelation::VenueOnly);
        assert!(observation
            .issues
            .contains(DropCopyIssueFlags::TRACKING_CAPACITY_EXHAUSTED));
        assert_eq!(reconciler.metrics().venue_only_reports, 1);
        assert_eq!(reconciler.metrics().tracking_capacity_exhaustions, 1);
    }

    #[test]
    fn missing_all_duplicate_identity_is_reported() {
        let mut reconciler = DropCopyReconciler::new(16, 16, DropCopyLateReportPolicy::AuditOnly);
        let report = report("", 0, event());
        let observation = reconciler.observe(&report);
        assert!(observation
            .issues
            .contains(DropCopyIssueFlags::MISSING_DUPLICATE_KEY));
        assert_eq!(reconciler.metrics().reports_without_duplicate_key, 1);
    }
}
