//! Allocation-free execution metrics and service-level objective evaluation.

use std::error::Error;
use std::fmt;

const LATENCY_SUB_BUCKETS: usize = 4;
const LATENCY_BUCKET_COUNT: usize = 1 + 64 * LATENCY_SUB_BUCKETS;
const PPM_SCALE: u64 = 1_000_000;

/// Execution latency population tracked by [`ExecutionSloCollector`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum ExecutionLatencyKind {
    /// Local submit receipt to adapter send.
    SubmitToSend = 0,
    /// Adapter send to venue acknowledgement or rejection.
    SendToAck = 1,
    /// Local submit receipt to venue acknowledgement or rejection.
    SubmitToAck = 2,
    /// Cancel request to cancel acknowledgement or rejection.
    CancelToAck = 3,
    /// Replace request to replace acknowledgement or rejection.
    ReplaceToAck = 4,
    /// Local submit receipt to a fill report.
    Fill = 5,
    /// One recovery cycle duration.
    Recovery = 6,
    /// Exchange/drop-copy timestamp to local drop-copy receipt.
    DropCopyLag = 7,
}

impl ExecutionLatencyKind {
    /// Every latency kind in stable discriminant order.
    pub const ALL: [Self; 8] = [
        Self::SubmitToSend,
        Self::SendToAck,
        Self::SubmitToAck,
        Self::CancelToAck,
        Self::ReplaceToAck,
        Self::Fill,
        Self::Recovery,
        Self::DropCopyLag,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

/// Queue population tracked by [`ExecutionSloCollector`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum ExecutionQueueKind {
    /// Provider adapter's outbound/inbound work queue.
    Adapter = 0,
    /// Concurrent OMS command queue.
    Command = 1,
    /// Canonical execution-event/report queue.
    Event = 2,
}

impl ExecutionQueueKind {
    /// Every queue kind in stable discriminant order.
    pub const ALL: [Self; 3] = [Self::Adapter, Self::Command, Self::Event];

    const fn index(self) -> usize {
        self as usize
    }
}

/// Route/session health classification used by execution SLO evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
#[non_exhaustive]
pub enum ExecutionRouteHealth {
    /// No authoritative health sample has been supplied.
    #[default]
    Unknown = 0,
    /// Route and execution session are healthy.
    Healthy = 1,
    /// Route remains connected but is degraded.
    Degraded = 2,
    /// Route or execution session is disconnected.
    Disconnected = 3,
}

/// Order acknowledgement outcome used for reject-rate accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExecutionSubmitOutcome {
    /// Venue accepted the new order.
    Ack,
    /// Venue or adapter rejected the new order.
    Reject,
}

/// Cancel acknowledgement outcome used for cancel-reject accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExecutionCancelOutcome {
    /// Venue accepted the cancellation.
    Ack,
    /// Venue or adapter rejected the cancellation.
    Reject,
}

/// Replace acknowledgement outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExecutionReplaceOutcome {
    /// Venue accepted the replacement.
    Ack,
    /// Venue or adapter rejected the replacement.
    Reject,
}

/// Complete timestamps for one new-order acknowledgement and optional fill.
///
/// Every timestamp must use the same host-monotonic clock domain. Exchange
/// timestamps require separate clock-skew handling before attribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionSubmitObservation {
    /// Local OMS receipt/create timestamp in nanoseconds.
    pub submit_ns: u64,
    /// Adapter send/ownership timestamp in nanoseconds.
    pub send_ns: u64,
    /// Venue acknowledgement or rejection receipt timestamp in nanoseconds.
    pub ack_ns: u64,
    /// Fill receipt timestamp in nanoseconds when a fill accompanies the acknowledgement.
    pub fill_ns: Option<u64>,
    /// Venue acknowledgement outcome.
    pub outcome: ExecutionSubmitOutcome,
}

impl ExecutionSubmitObservation {
    /// Creates an acknowledgement observation without an accompanying fill.
    pub const fn new(
        submit_ns: u64,
        send_ns: u64,
        ack_ns: u64,
        outcome: ExecutionSubmitOutcome,
    ) -> Self {
        Self {
            submit_ns,
            send_ns,
            ack_ns,
            fill_ns: None,
            outcome,
        }
    }

    /// Attaches a fill received with the acknowledgement.
    pub const fn with_fill_ns(mut self, fill_ns: u64) -> Self {
        self.fill_ns = Some(fill_ns);
        self
    }
}

/// Complete timestamps for one cancel acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionCancelObservation {
    /// Cancel request timestamp in nanoseconds.
    pub request_ns: u64,
    /// Cancel acknowledgement/rejection receipt timestamp in nanoseconds.
    pub ack_ns: u64,
    /// Venue acknowledgement outcome.
    pub outcome: ExecutionCancelOutcome,
}

impl ExecutionCancelObservation {
    /// Creates a cancel acknowledgement observation.
    pub const fn new(request_ns: u64, ack_ns: u64, outcome: ExecutionCancelOutcome) -> Self {
        Self {
            request_ns,
            ack_ns,
            outcome,
        }
    }
}

/// Complete timestamps for one replace acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionReplaceObservation {
    /// Replace request timestamp in nanoseconds.
    pub request_ns: u64,
    /// Replace acknowledgement/rejection receipt timestamp in nanoseconds.
    pub ack_ns: u64,
    /// Venue acknowledgement outcome.
    pub outcome: ExecutionReplaceOutcome,
}

impl ExecutionReplaceObservation {
    /// Creates a replace acknowledgement observation.
    pub const fn new(request_ns: u64, ack_ns: u64, outcome: ExecutionReplaceOutcome) -> Self {
        Self {
            request_ns,
            ack_ns,
            outcome,
        }
    }
}

/// Host-supplied operational state sampled at one timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionOperationalObservation {
    /// Observation timestamp in nanoseconds.
    pub now_ns: u64,
    /// Adapter work queue depth.
    pub adapter_queue_depth: u64,
    /// OMS command queue depth.
    pub command_queue_depth: u64,
    /// Canonical execution-event queue depth.
    pub event_queue_depth: u64,
    /// Latest appended WAL sequence.
    pub wal_head_sequence: u64,
    /// Latest durably synchronized WAL sequence.
    pub wal_durable_sequence: u64,
    /// Timestamp of the latest durable WAL progress when known.
    pub wal_durable_ns: Option<u64>,
    /// Timestamp of the latest validated checkpoint when known.
    pub checkpoint_ns: Option<u64>,
    /// Most recent recovery-cycle duration when sampled.
    pub recovery_duration_ns: Option<u64>,
    /// Current reconciliation mismatch count.
    pub reconciliation_mismatch_count: u64,
    /// Current route/session health.
    pub route_health: ExecutionRouteHealth,
    /// Current receive-minus-exchange drop-copy lag when known.
    pub drop_copy_lag_ns: Option<u64>,
}

impl ExecutionOperationalObservation {
    /// Creates an operational sample with zero depths and unknown optional state.
    pub const fn new(now_ns: u64) -> Self {
        Self {
            now_ns,
            adapter_queue_depth: 0,
            command_queue_depth: 0,
            event_queue_depth: 0,
            wal_head_sequence: 0,
            wal_durable_sequence: 0,
            wal_durable_ns: None,
            checkpoint_ns: None,
            recovery_duration_ns: None,
            reconciliation_mismatch_count: 0,
            route_health: ExecutionRouteHealth::Unknown,
            drop_copy_lag_ns: None,
        }
    }

    /// Sets adapter, command, and canonical event queue depths.
    pub const fn with_queue_depths(mut self, adapter: u64, command: u64, event: u64) -> Self {
        self.adapter_queue_depth = adapter;
        self.command_queue_depth = command;
        self.event_queue_depth = event;
        self
    }

    /// Sets WAL head/durable progress and optional durable-progress timestamp.
    pub const fn with_wal_progress(
        mut self,
        head_sequence: u64,
        durable_sequence: u64,
        durable_ns: Option<u64>,
    ) -> Self {
        self.wal_head_sequence = head_sequence;
        self.wal_durable_sequence = durable_sequence;
        self.wal_durable_ns = durable_ns;
        self
    }

    /// Sets the latest validated checkpoint timestamp.
    pub const fn with_checkpoint_ns(mut self, checkpoint_ns: u64) -> Self {
        self.checkpoint_ns = Some(checkpoint_ns);
        self
    }

    /// Sets the latest recovery-cycle duration.
    pub const fn with_recovery_duration_ns(mut self, duration_ns: u64) -> Self {
        self.recovery_duration_ns = Some(duration_ns);
        self
    }

    /// Sets the current reconciliation mismatch count.
    pub const fn with_reconciliation_mismatches(mut self, count: u64) -> Self {
        self.reconciliation_mismatch_count = count;
        self
    }

    /// Sets current route/session health.
    pub const fn with_route_health(mut self, health: ExecutionRouteHealth) -> Self {
        self.route_health = health;
        self
    }

    /// Sets the current receive-minus-exchange drop-copy lag.
    pub const fn with_drop_copy_lag_ns(mut self, lag_ns: u64) -> Self {
        self.drop_copy_lag_ns = Some(lag_ns);
        self
    }
}

/// Error returned while validating an execution metric observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExecutionMetricsError {
    /// A required timestamp is zero.
    MissingTimestamp,
    /// A later timestamp precedes an earlier timestamp.
    NonMonotonicTimestamp,
    /// Durable WAL progress exceeds the appended WAL head.
    WalSequenceAhead,
    /// Observation fields describe an impossible semantic outcome.
    InvalidObservation,
}

impl fmt::Display for ExecutionMetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTimestamp => f.write_str("required execution metric timestamp is missing"),
            Self::NonMonotonicTimestamp => {
                f.write_str("execution metric timestamps are not monotonic")
            }
            Self::WalSequenceAhead => {
                f.write_str("durable WAL sequence exceeds appended WAL sequence")
            }
            Self::InvalidObservation => f.write_str("execution metric observation is invalid"),
        }
    }
}

impl Error for ExecutionMetricsError {}

/// Quantile and aggregate snapshot from a fixed logarithmic histogram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ExecutionLatencySnapshot {
    /// Recorded sample count.
    pub count: u64,
    /// Minimum observed value in nanoseconds.
    pub min_ns: u64,
    /// Approximate 50th percentile upper bound in nanoseconds.
    pub p50_ns: u64,
    /// Approximate 95th percentile upper bound in nanoseconds.
    pub p95_ns: u64,
    /// Approximate 99th percentile upper bound in nanoseconds.
    pub p99_ns: u64,
    /// Maximum observed value in nanoseconds.
    pub max_ns: u64,
    /// Arithmetic mean in nanoseconds.
    pub mean_ns: u64,
}

/// Fixed-memory logarithmic latency histogram.
///
/// Four sub-buckets per power-of-two range provide bounded relative precision
/// without sample retention, heap allocation, floating point, or locks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionLatencyHistogram {
    buckets: [u64; LATENCY_BUCKET_COUNT],
    count: u64,
    sum_ns: u128,
    min_ns: u64,
    max_ns: u64,
}

impl ExecutionLatencyHistogram {
    /// Creates an empty fixed-memory histogram.
    pub const fn new() -> Self {
        Self {
            buckets: [0; LATENCY_BUCKET_COUNT],
            count: 0,
            sum_ns: 0,
            min_ns: 0,
            max_ns: 0,
        }
    }

    /// Records one non-negative nanosecond value without allocation.
    pub fn record(&mut self, value_ns: u64) {
        let index = latency_bucket_index(value_ns);
        self.buckets[index] = self.buckets[index].saturating_add(1);
        self.count = self.count.saturating_add(1);
        self.sum_ns = self.sum_ns.saturating_add(u128::from(value_ns));
        self.min_ns = if self.count == 1 {
            value_ns
        } else {
            self.min_ns.min(value_ns)
        };
        self.max_ns = self.max_ns.max(value_ns);
    }

    /// Returns the number of recorded samples.
    pub const fn len(&self) -> u64 {
        self.count
    }

    /// Returns true when no sample has been recorded.
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Clears all samples while retaining fixed storage.
    pub fn clear(&mut self) {
        self.buckets.fill(0);
        self.count = 0;
        self.sum_ns = 0;
        self.min_ns = 0;
        self.max_ns = 0;
    }

    /// Returns aggregate and p50/p95/p99 upper-bound estimates.
    pub fn snapshot(&self) -> ExecutionLatencySnapshot {
        if self.count == 0 {
            return ExecutionLatencySnapshot::default();
        }
        ExecutionLatencySnapshot {
            count: self.count,
            min_ns: self.min_ns,
            p50_ns: self.quantile(50, 100),
            p95_ns: self.quantile(95, 100),
            p99_ns: self.quantile(99, 100),
            max_ns: self.max_ns,
            mean_ns: u64::try_from(self.sum_ns / u128::from(self.count)).unwrap_or(u64::MAX),
        }
    }

    fn quantile(&self, numerator: u64, denominator: u64) -> u64 {
        let rank = (u128::from(self.count) * u128::from(numerator))
            .div_ceil(u128::from(denominator))
            .max(1);
        let mut cumulative = 0_u128;
        for (index, count) in self.buckets.iter().enumerate() {
            cumulative = cumulative.saturating_add(u128::from(*count));
            if cumulative >= rank {
                return latency_bucket_upper_bound(index);
            }
        }
        self.max_ns
    }
}

impl Default for ExecutionLatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Current and maximum value for one sampled gauge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ExecutionGaugeSnapshot {
    /// Latest observed value.
    pub current: u64,
    /// Maximum observed value since reset.
    pub max: u64,
    /// Number of observations.
    pub observations: u64,
}

impl ExecutionGaugeSnapshot {
    fn observe(&mut self, value: u64) {
        self.current = value;
        self.max = self.max.max(value);
        self.observations = self.observations.saturating_add(1);
    }
}

/// Numerator/denominator rate represented in parts per million.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ExecutionRateSnapshot {
    /// Events matching the rate numerator.
    pub numerator: u64,
    /// Total eligible events.
    pub denominator: u64,
    /// Saturating rate in parts per million.
    pub parts_per_million: u64,
}

impl ExecutionRateSnapshot {
    fn new(numerator: u64, denominator: u64) -> Self {
        let parts_per_million = if denominator == 0 {
            0
        } else {
            u64::try_from((u128::from(numerator) * u128::from(PPM_SCALE)) / u128::from(denominator))
                .unwrap_or(u64::MAX)
        };
        Self {
            numerator,
            denominator,
            parts_per_million,
        }
    }
}

/// Route-health counters and latest state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ExecutionRouteHealthSnapshot {
    /// Latest route health.
    pub current: ExecutionRouteHealth,
    /// Number of supplied samples.
    pub observations: u64,
    /// Number of state changes after the first sample.
    pub transitions: u64,
    /// Number of degraded samples.
    pub degraded_observations: u64,
    /// Number of disconnected samples.
    pub disconnected_observations: u64,
}

/// Immutable execution SLI snapshot suitable for host exporters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionSloSnapshot {
    latencies: [ExecutionLatencySnapshot; 8],
    queue_depths: [ExecutionGaugeSnapshot; 3],
    reject_rate: ExecutionRateSnapshot,
    cancel_reject_rate: ExecutionRateSnapshot,
    replace_reject_rate: ExecutionRateSnapshot,
    wal_lag_records: ExecutionGaugeSnapshot,
    wal_lag_ns: ExecutionGaugeSnapshot,
    checkpoint_age_ns: ExecutionGaugeSnapshot,
    reconciliation_mismatches: ExecutionGaugeSnapshot,
    route_health: ExecutionRouteHealthSnapshot,
    operational_observations: u64,
}

impl ExecutionSloSnapshot {
    /// Returns one latency population.
    pub const fn latency(self, kind: ExecutionLatencyKind) -> ExecutionLatencySnapshot {
        self.latencies[kind.index()]
    }

    /// Returns one queue-depth gauge.
    pub const fn queue_depth(self, kind: ExecutionQueueKind) -> ExecutionGaugeSnapshot {
        self.queue_depths[kind.index()]
    }

    /// Returns the submit reject rate.
    pub const fn reject_rate(self) -> ExecutionRateSnapshot {
        self.reject_rate
    }

    /// Returns the cancel reject rate.
    pub const fn cancel_reject_rate(self) -> ExecutionRateSnapshot {
        self.cancel_reject_rate
    }

    /// Returns the replace reject rate.
    pub const fn replace_reject_rate(self) -> ExecutionRateSnapshot {
        self.replace_reject_rate
    }

    /// Returns WAL sequence lag (`head - durable`).
    pub const fn wal_lag_records(self) -> ExecutionGaugeSnapshot {
        self.wal_lag_records
    }

    /// Returns age of the latest durable WAL progress.
    pub const fn wal_lag_ns(self) -> ExecutionGaugeSnapshot {
        self.wal_lag_ns
    }

    /// Returns latest validated checkpoint age.
    pub const fn checkpoint_age_ns(self) -> ExecutionGaugeSnapshot {
        self.checkpoint_age_ns
    }

    /// Returns reconciliation mismatch count.
    pub const fn reconciliation_mismatches(self) -> ExecutionGaugeSnapshot {
        self.reconciliation_mismatches
    }

    /// Returns route-health state and counters.
    pub const fn route_health(self) -> ExecutionRouteHealthSnapshot {
        self.route_health
    }

    /// Returns the number of valid operational observations.
    pub const fn operational_observations(self) -> u64 {
        self.operational_observations
    }
}

/// Single-owner fixed-memory execution SLI collector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionSloCollector {
    latencies: [ExecutionLatencyHistogram; 8],
    queue_depths: [ExecutionGaugeSnapshot; 3],
    submit_outcomes: u64,
    submit_rejects: u64,
    cancel_outcomes: u64,
    cancel_rejects: u64,
    replace_outcomes: u64,
    replace_rejects: u64,
    wal_lag_records: ExecutionGaugeSnapshot,
    wal_lag_ns: ExecutionGaugeSnapshot,
    checkpoint_age_ns: ExecutionGaugeSnapshot,
    reconciliation_mismatches: ExecutionGaugeSnapshot,
    route_health: ExecutionRouteHealthSnapshot,
    operational_observations: u64,
}

impl ExecutionSloCollector {
    /// Creates an empty fixed-memory collector.
    pub fn new() -> Self {
        Self {
            latencies: std::array::from_fn(|_| ExecutionLatencyHistogram::new()),
            queue_depths: [ExecutionGaugeSnapshot {
                current: 0,
                max: 0,
                observations: 0,
            }; 3],
            submit_outcomes: 0,
            submit_rejects: 0,
            cancel_outcomes: 0,
            cancel_rejects: 0,
            replace_outcomes: 0,
            replace_rejects: 0,
            wal_lag_records: ExecutionGaugeSnapshot {
                current: 0,
                max: 0,
                observations: 0,
            },
            wal_lag_ns: ExecutionGaugeSnapshot {
                current: 0,
                max: 0,
                observations: 0,
            },
            checkpoint_age_ns: ExecutionGaugeSnapshot {
                current: 0,
                max: 0,
                observations: 0,
            },
            reconciliation_mismatches: ExecutionGaugeSnapshot {
                current: 0,
                max: 0,
                observations: 0,
            },
            route_health: ExecutionRouteHealthSnapshot {
                current: ExecutionRouteHealth::Unknown,
                observations: 0,
                transitions: 0,
                degraded_observations: 0,
                disconnected_observations: 0,
            },
            operational_observations: 0,
        }
    }

    /// Records a validated submit acknowledgement and optional fill.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionMetricsError`] for missing or regressing timestamps.
    /// No metric changes when validation fails.
    pub fn observe_submit(
        &mut self,
        observation: ExecutionSubmitObservation,
    ) -> Result<(), ExecutionMetricsError> {
        require_timestamp(observation.submit_ns)?;
        require_timestamp(observation.send_ns)?;
        require_timestamp(observation.ack_ns)?;
        let submit_to_send = ordered_diff(observation.submit_ns, observation.send_ns)?;
        let send_to_ack = ordered_diff(observation.send_ns, observation.ack_ns)?;
        let submit_to_ack = ordered_diff(observation.submit_ns, observation.ack_ns)?;
        let fill = if let Some(fill_ns) = observation.fill_ns {
            if observation.outcome == ExecutionSubmitOutcome::Reject {
                return Err(ExecutionMetricsError::InvalidObservation);
            }
            let _ = ordered_diff(observation.send_ns, fill_ns)?;
            Some(ordered_diff(observation.submit_ns, fill_ns)?)
        } else {
            None
        };

        self.record_latency(ExecutionLatencyKind::SubmitToSend, submit_to_send);
        self.record_latency(ExecutionLatencyKind::SendToAck, send_to_ack);
        self.record_latency(ExecutionLatencyKind::SubmitToAck, submit_to_ack);
        if let Some(fill) = fill {
            self.record_latency(ExecutionLatencyKind::Fill, fill);
        }
        self.submit_outcomes = self.submit_outcomes.saturating_add(1);
        if observation.outcome == ExecutionSubmitOutcome::Reject {
            self.submit_rejects = self.submit_rejects.saturating_add(1);
        }
        Ok(())
    }

    /// Records a validated cancel acknowledgement.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionMetricsError`] for missing or regressing timestamps.
    pub fn observe_cancel(
        &mut self,
        observation: ExecutionCancelObservation,
    ) -> Result<(), ExecutionMetricsError> {
        require_timestamp(observation.request_ns)?;
        require_timestamp(observation.ack_ns)?;
        let latency = ordered_diff(observation.request_ns, observation.ack_ns)?;
        self.record_latency(ExecutionLatencyKind::CancelToAck, latency);
        self.cancel_outcomes = self.cancel_outcomes.saturating_add(1);
        if observation.outcome == ExecutionCancelOutcome::Reject {
            self.cancel_rejects = self.cancel_rejects.saturating_add(1);
        }
        Ok(())
    }

    /// Records a validated replace acknowledgement.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionMetricsError`] for missing or regressing timestamps.
    pub fn observe_replace(
        &mut self,
        observation: ExecutionReplaceObservation,
    ) -> Result<(), ExecutionMetricsError> {
        require_timestamp(observation.request_ns)?;
        require_timestamp(observation.ack_ns)?;
        let latency = ordered_diff(observation.request_ns, observation.ack_ns)?;
        self.record_latency(ExecutionLatencyKind::ReplaceToAck, latency);
        self.replace_outcomes = self.replace_outcomes.saturating_add(1);
        if observation.outcome == ExecutionReplaceOutcome::Reject {
            self.replace_rejects = self.replace_rejects.saturating_add(1);
        }
        Ok(())
    }

    /// Records one fill latency from submit receipt to fill receipt.
    ///
    /// Use this for additional partial fills after the acknowledgement sample.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionMetricsError`] for missing or regressing timestamps.
    pub fn observe_fill(
        &mut self,
        submit_ns: u64,
        fill_ns: u64,
    ) -> Result<(), ExecutionMetricsError> {
        require_timestamp(submit_ns)?;
        require_timestamp(fill_ns)?;
        let latency = ordered_diff(submit_ns, fill_ns)?;
        self.record_latency(ExecutionLatencyKind::Fill, latency);
        Ok(())
    }

    /// Records one caller-derived latency value.
    pub fn record_latency(&mut self, kind: ExecutionLatencyKind, latency_ns: u64) {
        self.latencies[kind.index()].record(latency_ns);
    }

    /// Records one atomic operational sample.
    ///
    /// `now_ns`, WAL/checkpoint timestamps, and queue depths are supplied by
    /// the host. The collector performs no clock or I/O operation.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionMetricsError`] for invalid WAL sequence or timestamp
    /// ordering. No metric changes when validation fails.
    pub fn observe_operational(
        &mut self,
        observation: ExecutionOperationalObservation,
    ) -> Result<(), ExecutionMetricsError> {
        require_timestamp(observation.now_ns)?;
        if observation.wal_durable_sequence > observation.wal_head_sequence {
            return Err(ExecutionMetricsError::WalSequenceAhead);
        }
        let wal_age = optional_age(observation.wal_durable_ns, observation.now_ns)?;
        let checkpoint_age = optional_age(observation.checkpoint_ns, observation.now_ns)?;

        self.queue_depths[ExecutionQueueKind::Adapter.index()]
            .observe(observation.adapter_queue_depth);
        self.queue_depths[ExecutionQueueKind::Command.index()]
            .observe(observation.command_queue_depth);
        self.queue_depths[ExecutionQueueKind::Event.index()].observe(observation.event_queue_depth);
        self.wal_lag_records.observe(
            observation
                .wal_head_sequence
                .saturating_sub(observation.wal_durable_sequence),
        );
        if let Some(wal_age) = wal_age {
            self.wal_lag_ns.observe(wal_age);
        }
        if let Some(checkpoint_age) = checkpoint_age {
            self.checkpoint_age_ns.observe(checkpoint_age);
        }
        if let Some(recovery_duration_ns) = observation.recovery_duration_ns {
            self.record_latency(ExecutionLatencyKind::Recovery, recovery_duration_ns);
        }
        self.reconciliation_mismatches
            .observe(observation.reconciliation_mismatch_count);
        self.observe_route_health(observation.route_health);
        if let Some(drop_copy_lag_ns) = observation.drop_copy_lag_ns {
            self.record_latency(ExecutionLatencyKind::DropCopyLag, drop_copy_lag_ns);
        }
        self.operational_observations = self.operational_observations.saturating_add(1);
        Ok(())
    }

    /// Returns an immutable typed SLI snapshot.
    pub fn snapshot(&self) -> ExecutionSloSnapshot {
        let mut latencies = [ExecutionLatencySnapshot::default(); 8];
        for kind in ExecutionLatencyKind::ALL {
            latencies[kind.index()] = self.latencies[kind.index()].snapshot();
        }
        ExecutionSloSnapshot {
            latencies,
            queue_depths: self.queue_depths,
            reject_rate: ExecutionRateSnapshot::new(self.submit_rejects, self.submit_outcomes),
            cancel_reject_rate: ExecutionRateSnapshot::new(
                self.cancel_rejects,
                self.cancel_outcomes,
            ),
            replace_reject_rate: ExecutionRateSnapshot::new(
                self.replace_rejects,
                self.replace_outcomes,
            ),
            wal_lag_records: self.wal_lag_records,
            wal_lag_ns: self.wal_lag_ns,
            checkpoint_age_ns: self.checkpoint_age_ns,
            reconciliation_mismatches: self.reconciliation_mismatches,
            route_health: self.route_health,
            operational_observations: self.operational_observations,
        }
    }

    /// Clears every metric without reallocating.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    fn observe_route_health(&mut self, health: ExecutionRouteHealth) {
        if self.route_health.observations != 0 && self.route_health.current != health {
            self.route_health.transitions = self.route_health.transitions.saturating_add(1);
        }
        self.route_health.current = health;
        self.route_health.observations = self.route_health.observations.saturating_add(1);
        if health == ExecutionRouteHealth::Degraded {
            self.route_health.degraded_observations =
                self.route_health.degraded_observations.saturating_add(1);
        }
        if health == ExecutionRouteHealth::Disconnected {
            self.route_health.disconnected_observations = self
                .route_health
                .disconnected_observations
                .saturating_add(1);
        }
    }
}

impl Default for ExecutionSloCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Optional thresholds used to evaluate an [`ExecutionSloSnapshot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionSloTargets {
    /// Maximum submit-to-send p99 latency.
    submit_to_send_p99_ns: Option<u64>,
    /// Maximum send-to-ack p99 latency.
    send_to_ack_p99_ns: Option<u64>,
    /// Maximum submit-to-ack p99 latency.
    submit_to_ack_p99_ns: Option<u64>,
    /// Maximum cancel-to-ack p99 latency.
    cancel_to_ack_p99_ns: Option<u64>,
    /// Maximum replace-to-ack p99 latency.
    replace_to_ack_p99_ns: Option<u64>,
    /// Maximum fill p99 latency.
    fill_p99_ns: Option<u64>,
    /// Maximum submit reject rate in parts per million.
    reject_rate_ppm: Option<u64>,
    /// Maximum cancel reject rate in parts per million.
    cancel_reject_rate_ppm: Option<u64>,
    /// Maximum replace reject rate in parts per million.
    replace_reject_rate_ppm: Option<u64>,
    /// Maximum adapter queue depth.
    adapter_queue_depth: Option<u64>,
    /// Maximum command queue depth.
    command_queue_depth: Option<u64>,
    /// Maximum event queue depth.
    event_queue_depth: Option<u64>,
    /// Maximum WAL sequence lag.
    wal_lag_records: Option<u64>,
    /// Maximum durable WAL age.
    wal_lag_ns: Option<u64>,
    /// Maximum validated checkpoint age.
    checkpoint_age_ns: Option<u64>,
    /// Maximum recovery p99 duration.
    recovery_p99_ns: Option<u64>,
    /// Maximum current reconciliation mismatch count.
    reconciliation_mismatch_count: Option<u64>,
    /// Require the latest route health to be healthy.
    require_healthy_route: bool,
    /// Maximum drop-copy p99 lag.
    drop_copy_lag_p99_ns: Option<u64>,
    /// Minimum samples required for each enabled latency/rate objective.
    minimum_samples: u64,
    /// Treat insufficient or missing samples as a violation.
    fail_on_insufficient_samples: bool,
}

impl ExecutionSloTargets {
    /// Creates conservative sample handling with every objective disabled.
    pub const fn new() -> Self {
        Self {
            submit_to_send_p99_ns: None,
            send_to_ack_p99_ns: None,
            submit_to_ack_p99_ns: None,
            cancel_to_ack_p99_ns: None,
            replace_to_ack_p99_ns: None,
            fill_p99_ns: None,
            reject_rate_ppm: None,
            cancel_reject_rate_ppm: None,
            replace_reject_rate_ppm: None,
            adapter_queue_depth: None,
            command_queue_depth: None,
            event_queue_depth: None,
            wal_lag_records: None,
            wal_lag_ns: None,
            checkpoint_age_ns: None,
            recovery_p99_ns: None,
            reconciliation_mismatch_count: None,
            require_healthy_route: false,
            drop_copy_lag_p99_ns: None,
            minimum_samples: 1,
            fail_on_insufficient_samples: true,
        }
    }

    /// Enables or replaces one p99 latency objective.
    pub const fn with_latency_p99_ns(mut self, kind: ExecutionLatencyKind, target_ns: u64) -> Self {
        match kind {
            ExecutionLatencyKind::SubmitToSend => self.submit_to_send_p99_ns = Some(target_ns),
            ExecutionLatencyKind::SendToAck => self.send_to_ack_p99_ns = Some(target_ns),
            ExecutionLatencyKind::SubmitToAck => self.submit_to_ack_p99_ns = Some(target_ns),
            ExecutionLatencyKind::CancelToAck => self.cancel_to_ack_p99_ns = Some(target_ns),
            ExecutionLatencyKind::ReplaceToAck => self.replace_to_ack_p99_ns = Some(target_ns),
            ExecutionLatencyKind::Fill => self.fill_p99_ns = Some(target_ns),
            ExecutionLatencyKind::Recovery => self.recovery_p99_ns = Some(target_ns),
            ExecutionLatencyKind::DropCopyLag => self.drop_copy_lag_p99_ns = Some(target_ns),
        }
        self
    }

    /// Returns the configured p99 latency objective for `kind`.
    pub const fn latency_p99_ns(self, kind: ExecutionLatencyKind) -> Option<u64> {
        match kind {
            ExecutionLatencyKind::SubmitToSend => self.submit_to_send_p99_ns,
            ExecutionLatencyKind::SendToAck => self.send_to_ack_p99_ns,
            ExecutionLatencyKind::SubmitToAck => self.submit_to_ack_p99_ns,
            ExecutionLatencyKind::CancelToAck => self.cancel_to_ack_p99_ns,
            ExecutionLatencyKind::ReplaceToAck => self.replace_to_ack_p99_ns,
            ExecutionLatencyKind::Fill => self.fill_p99_ns,
            ExecutionLatencyKind::Recovery => self.recovery_p99_ns,
            ExecutionLatencyKind::DropCopyLag => self.drop_copy_lag_p99_ns,
        }
    }

    /// Enables or replaces the submit reject-rate objective in parts per million.
    pub const fn with_reject_rate_ppm(mut self, target: u64) -> Self {
        self.reject_rate_ppm = Some(if target > PPM_SCALE {
            PPM_SCALE
        } else {
            target
        });
        self
    }

    /// Enables or replaces the cancel reject-rate objective in parts per million.
    pub const fn with_cancel_reject_rate_ppm(mut self, target: u64) -> Self {
        self.cancel_reject_rate_ppm = Some(if target > PPM_SCALE {
            PPM_SCALE
        } else {
            target
        });
        self
    }

    /// Enables or replaces the replace reject-rate objective in parts per million.
    pub const fn with_replace_reject_rate_ppm(mut self, target: u64) -> Self {
        self.replace_reject_rate_ppm = Some(if target > PPM_SCALE {
            PPM_SCALE
        } else {
            target
        });
        self
    }

    /// Enables or replaces one current queue-depth objective.
    pub const fn with_queue_depth(mut self, kind: ExecutionQueueKind, target: u64) -> Self {
        match kind {
            ExecutionQueueKind::Adapter => self.adapter_queue_depth = Some(target),
            ExecutionQueueKind::Command => self.command_queue_depth = Some(target),
            ExecutionQueueKind::Event => self.event_queue_depth = Some(target),
        }
        self
    }

    /// Enables or replaces the WAL sequence-lag objective.
    pub const fn with_wal_lag_records(mut self, target: u64) -> Self {
        self.wal_lag_records = Some(target);
        self
    }

    /// Enables or replaces the durable-WAL age objective.
    pub const fn with_wal_lag_ns(mut self, target: u64) -> Self {
        self.wal_lag_ns = Some(target);
        self
    }

    /// Enables or replaces the checkpoint-age objective.
    pub const fn with_checkpoint_age_ns(mut self, target: u64) -> Self {
        self.checkpoint_age_ns = Some(target);
        self
    }

    /// Enables or replaces the reconciliation mismatch-count objective.
    pub const fn with_reconciliation_mismatch_count(mut self, target: u64) -> Self {
        self.reconciliation_mismatch_count = Some(target);
        self
    }

    /// Requires the latest observed route state to be healthy when `required`.
    pub const fn with_healthy_route_required(mut self, required: bool) -> Self {
        self.require_healthy_route = required;
        self
    }

    /// Sets the minimum population required by each latency and rate objective.
    ///
    /// A value of zero is normalized to one.
    pub const fn with_minimum_samples(mut self, samples: u64) -> Self {
        self.minimum_samples = if samples == 0 { 1 } else { samples };
        self
    }

    /// Selects whether missing or undersized populations violate enabled objectives.
    pub const fn with_fail_on_insufficient_samples(mut self, fail: bool) -> Self {
        self.fail_on_insufficient_samples = fail;
        self
    }
}

impl Default for ExecutionSloTargets {
    fn default() -> Self {
        Self::new()
    }
}

/// Machine-readable execution SLO violation classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum ExecutionSloViolationKind {
    /// An enabled objective lacks its minimum sample population.
    InsufficientSamples = 0,
    /// Submit-to-send p99 exceeded its target.
    SubmitToSendLatency = 1,
    /// Send-to-ack p99 exceeded its target.
    SendToAckLatency = 2,
    /// Submit-to-ack p99 exceeded its target.
    SubmitToAckLatency = 3,
    /// Cancel-to-ack p99 exceeded its target.
    CancelToAckLatency = 4,
    /// Replace-to-ack p99 exceeded its target.
    ReplaceToAckLatency = 5,
    /// Fill p99 exceeded its target.
    FillLatency = 6,
    /// Submit reject rate exceeded its target.
    RejectRate = 7,
    /// Cancel reject rate exceeded its target.
    CancelRejectRate = 8,
    /// Adapter queue depth exceeded its target.
    AdapterQueueDepth = 9,
    /// Command queue depth exceeded its target.
    CommandQueueDepth = 10,
    /// Event queue depth exceeded its target.
    EventQueueDepth = 11,
    /// WAL sequence lag exceeded its target.
    WalLagRecords = 12,
    /// Durable WAL age exceeded its target.
    WalLagTime = 13,
    /// Checkpoint age exceeded its target.
    CheckpointAge = 14,
    /// Recovery p99 exceeded its target.
    RecoveryDuration = 15,
    /// Reconciliation mismatch count exceeded its target.
    ReconciliationMismatch = 16,
    /// Route health is not healthy.
    RouteHealth = 17,
    /// Drop-copy p99 lag exceeded its target.
    DropCopyLag = 18,
    /// Replace reject rate exceeded its target.
    ReplaceRejectRate = 19,
}

impl ExecutionSloViolationKind {
    const fn bit(self) -> u64 {
        1_u64 << self as u8
    }
}

/// Allocation-free bitset of execution SLO violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ExecutionSloViolationFlags(u64);

impl ExecutionSloViolationFlags {
    /// Returns true when a violation kind is present.
    pub const fn contains(self, kind: ExecutionSloViolationKind) -> bool {
        self.0 & kind.bit() != 0
    }

    /// Returns true when no objective was violated.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns the stable raw bit mask.
    pub const fn bits(self) -> u64 {
        self.0
    }

    fn insert(&mut self, kind: ExecutionSloViolationKind) {
        self.0 |= kind.bit();
    }
}

/// Result of evaluating one SLI snapshot against configured objectives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ExecutionSloReport {
    /// Number of enabled objectives evaluated.
    pub objectives_evaluated: u32,
    /// Number of enabled objectives that failed.
    pub objectives_violated: u32,
    /// Machine-readable violation flags.
    pub violations: ExecutionSloViolationFlags,
}

impl ExecutionSloReport {
    /// Returns true when every enabled objective passed.
    pub const fn is_compliant(self) -> bool {
        self.violations.is_empty()
    }
}

impl ExecutionSloSnapshot {
    /// Evaluates this snapshot against host-selected objectives.
    pub fn evaluate(self, targets: ExecutionSloTargets) -> ExecutionSloReport {
        let mut report = ExecutionSloReport::default();
        evaluate_latency(
            &mut report,
            self.latency(ExecutionLatencyKind::SubmitToSend),
            targets.submit_to_send_p99_ns,
            targets,
            ExecutionSloViolationKind::SubmitToSendLatency,
        );
        evaluate_latency(
            &mut report,
            self.latency(ExecutionLatencyKind::SendToAck),
            targets.send_to_ack_p99_ns,
            targets,
            ExecutionSloViolationKind::SendToAckLatency,
        );
        evaluate_latency(
            &mut report,
            self.latency(ExecutionLatencyKind::SubmitToAck),
            targets.submit_to_ack_p99_ns,
            targets,
            ExecutionSloViolationKind::SubmitToAckLatency,
        );
        evaluate_latency(
            &mut report,
            self.latency(ExecutionLatencyKind::CancelToAck),
            targets.cancel_to_ack_p99_ns,
            targets,
            ExecutionSloViolationKind::CancelToAckLatency,
        );
        evaluate_latency(
            &mut report,
            self.latency(ExecutionLatencyKind::ReplaceToAck),
            targets.replace_to_ack_p99_ns,
            targets,
            ExecutionSloViolationKind::ReplaceToAckLatency,
        );
        evaluate_latency(
            &mut report,
            self.latency(ExecutionLatencyKind::Fill),
            targets.fill_p99_ns,
            targets,
            ExecutionSloViolationKind::FillLatency,
        );
        evaluate_rate(
            &mut report,
            self.reject_rate,
            targets.reject_rate_ppm,
            targets,
            ExecutionSloViolationKind::RejectRate,
        );
        evaluate_rate(
            &mut report,
            self.cancel_reject_rate,
            targets.cancel_reject_rate_ppm,
            targets,
            ExecutionSloViolationKind::CancelRejectRate,
        );
        evaluate_rate(
            &mut report,
            self.replace_reject_rate,
            targets.replace_reject_rate_ppm,
            targets,
            ExecutionSloViolationKind::ReplaceRejectRate,
        );
        evaluate_gauge(
            &mut report,
            self.queue_depth(ExecutionQueueKind::Adapter),
            targets.adapter_queue_depth,
            targets,
            ExecutionSloViolationKind::AdapterQueueDepth,
        );
        evaluate_gauge(
            &mut report,
            self.queue_depth(ExecutionQueueKind::Command),
            targets.command_queue_depth,
            targets,
            ExecutionSloViolationKind::CommandQueueDepth,
        );
        evaluate_gauge(
            &mut report,
            self.queue_depth(ExecutionQueueKind::Event),
            targets.event_queue_depth,
            targets,
            ExecutionSloViolationKind::EventQueueDepth,
        );
        evaluate_gauge(
            &mut report,
            self.wal_lag_records,
            targets.wal_lag_records,
            targets,
            ExecutionSloViolationKind::WalLagRecords,
        );
        evaluate_gauge(
            &mut report,
            self.wal_lag_ns,
            targets.wal_lag_ns,
            targets,
            ExecutionSloViolationKind::WalLagTime,
        );
        evaluate_gauge(
            &mut report,
            self.checkpoint_age_ns,
            targets.checkpoint_age_ns,
            targets,
            ExecutionSloViolationKind::CheckpointAge,
        );
        evaluate_latency(
            &mut report,
            self.latency(ExecutionLatencyKind::Recovery),
            targets.recovery_p99_ns,
            targets,
            ExecutionSloViolationKind::RecoveryDuration,
        );
        evaluate_gauge(
            &mut report,
            self.reconciliation_mismatches,
            targets.reconciliation_mismatch_count,
            targets,
            ExecutionSloViolationKind::ReconciliationMismatch,
        );
        if targets.require_healthy_route {
            report.objectives_evaluated = report.objectives_evaluated.saturating_add(1);
            if self.route_health.observations == 0 {
                mark_insufficient(&mut report, targets);
            } else if self.route_health.current != ExecutionRouteHealth::Healthy {
                mark_violation(&mut report, ExecutionSloViolationKind::RouteHealth);
            }
        }
        evaluate_latency(
            &mut report,
            self.latency(ExecutionLatencyKind::DropCopyLag),
            targets.drop_copy_lag_p99_ns,
            targets,
            ExecutionSloViolationKind::DropCopyLag,
        );
        report
    }
}

fn evaluate_latency(
    report: &mut ExecutionSloReport,
    value: ExecutionLatencySnapshot,
    target: Option<u64>,
    config: ExecutionSloTargets,
    kind: ExecutionSloViolationKind,
) {
    let Some(target) = target else { return };
    report.objectives_evaluated = report.objectives_evaluated.saturating_add(1);
    if value.count < config.minimum_samples.max(1) {
        mark_insufficient(report, config);
    } else if value.p99_ns > target {
        mark_violation(report, kind);
    }
}

fn evaluate_rate(
    report: &mut ExecutionSloReport,
    value: ExecutionRateSnapshot,
    target: Option<u64>,
    config: ExecutionSloTargets,
    kind: ExecutionSloViolationKind,
) {
    let Some(target) = target else { return };
    report.objectives_evaluated = report.objectives_evaluated.saturating_add(1);
    if value.denominator < config.minimum_samples.max(1) {
        mark_insufficient(report, config);
    } else if value.parts_per_million > target {
        mark_violation(report, kind);
    }
}

fn evaluate_gauge(
    report: &mut ExecutionSloReport,
    value: ExecutionGaugeSnapshot,
    target: Option<u64>,
    config: ExecutionSloTargets,
    kind: ExecutionSloViolationKind,
) {
    let Some(target) = target else { return };
    report.objectives_evaluated = report.objectives_evaluated.saturating_add(1);
    if value.observations == 0 {
        mark_insufficient(report, config);
    } else if value.current > target {
        mark_violation(report, kind);
    }
}

fn mark_insufficient(report: &mut ExecutionSloReport, config: ExecutionSloTargets) {
    if config.fail_on_insufficient_samples {
        report.objectives_violated = report.objectives_violated.saturating_add(1);
        report
            .violations
            .insert(ExecutionSloViolationKind::InsufficientSamples);
    }
}

fn mark_violation(report: &mut ExecutionSloReport, kind: ExecutionSloViolationKind) {
    if !report.violations.contains(kind) {
        report.objectives_violated = report.objectives_violated.saturating_add(1);
        report.violations.insert(kind);
    }
}

fn require_timestamp(timestamp_ns: u64) -> Result<(), ExecutionMetricsError> {
    if timestamp_ns == 0 {
        Err(ExecutionMetricsError::MissingTimestamp)
    } else {
        Ok(())
    }
}

fn ordered_diff(start_ns: u64, end_ns: u64) -> Result<u64, ExecutionMetricsError> {
    end_ns
        .checked_sub(start_ns)
        .ok_or(ExecutionMetricsError::NonMonotonicTimestamp)
}

fn optional_age(
    timestamp_ns: Option<u64>,
    now_ns: u64,
) -> Result<Option<u64>, ExecutionMetricsError> {
    timestamp_ns
        .map(|timestamp_ns| {
            require_timestamp(timestamp_ns)?;
            ordered_diff(timestamp_ns, now_ns)
        })
        .transpose()
}

fn latency_bucket_index(value: u64) -> usize {
    if value == 0 {
        return 0;
    }
    let exponent = (u64::BITS - 1 - value.leading_zeros()) as usize;
    let base = 1_u64 << exponent;
    let offset = usize::try_from(
        (u128::from(value - base) * LATENCY_SUB_BUCKETS as u128) / u128::from(base),
    )
    .unwrap_or(LATENCY_SUB_BUCKETS - 1)
    .min(LATENCY_SUB_BUCKETS - 1);
    1 + exponent * LATENCY_SUB_BUCKETS + offset
}

fn latency_bucket_upper_bound(index: usize) -> u64 {
    if index == 0 {
        return 0;
    }
    let zero_based = index - 1;
    let exponent = zero_based / LATENCY_SUB_BUCKETS;
    let sub_bucket = zero_based % LATENCY_SUB_BUCKETS;
    let base = 1_u128 << exponent;
    let width = (base * (sub_bucket + 1) as u128).div_ceil(LATENCY_SUB_BUCKETS as u128);
    u64::try_from(base + width - 1).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_reports_bounded_quantiles_and_exact_extrema() {
        let mut histogram = ExecutionLatencyHistogram::new();
        for value in 1..=100 {
            histogram.record(value);
        }
        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count, 100);
        assert_eq!(snapshot.min_ns, 1);
        assert_eq!(snapshot.max_ns, 100);
        assert!(snapshot.p50_ns >= 50);
        assert!(snapshot.p50_ns <= 63);
        assert!(snapshot.p95_ns >= 95);
        assert!(snapshot.p99_ns >= 99);
        assert_eq!(snapshot.mean_ns, 50);
    }

    #[test]
    fn submit_cancel_replace_and_fill_metrics_are_attributed() {
        let mut collector = ExecutionSloCollector::new();
        collector
            .observe_submit(ExecutionSubmitObservation {
                submit_ns: 100,
                send_ns: 110,
                ack_ns: 150,
                fill_ns: None,
                outcome: ExecutionSubmitOutcome::Reject,
            })
            .unwrap();
        collector.observe_fill(100, 200).unwrap();
        collector
            .observe_cancel(ExecutionCancelObservation {
                request_ns: 200,
                ack_ns: 230,
                outcome: ExecutionCancelOutcome::Reject,
            })
            .unwrap();
        collector
            .observe_replace(ExecutionReplaceObservation {
                request_ns: 300,
                ack_ns: 325,
                outcome: ExecutionReplaceOutcome::Reject,
            })
            .unwrap();

        let snapshot = collector.snapshot();
        assert_eq!(
            snapshot.latency(ExecutionLatencyKind::SubmitToSend).max_ns,
            10
        );
        assert_eq!(snapshot.latency(ExecutionLatencyKind::SendToAck).max_ns, 40);
        assert_eq!(
            snapshot.latency(ExecutionLatencyKind::SubmitToAck).max_ns,
            50
        );
        assert_eq!(snapshot.latency(ExecutionLatencyKind::Fill).max_ns, 100);
        assert_eq!(
            snapshot.latency(ExecutionLatencyKind::CancelToAck).max_ns,
            30
        );
        assert_eq!(
            snapshot.latency(ExecutionLatencyKind::ReplaceToAck).max_ns,
            25
        );
        assert_eq!(snapshot.reject_rate().parts_per_million, PPM_SCALE);
        assert_eq!(snapshot.cancel_reject_rate().parts_per_million, PPM_SCALE);
        assert_eq!(snapshot.replace_reject_rate().parts_per_million, PPM_SCALE);
    }

    #[test]
    fn invalid_timestamps_do_not_partially_mutate_collector() {
        let mut collector = ExecutionSloCollector::new();
        let before = collector.clone();
        let error = collector.observe_submit(ExecutionSubmitObservation {
            submit_ns: 100,
            send_ns: 90,
            ack_ns: 120,
            fill_ns: None,
            outcome: ExecutionSubmitOutcome::Ack,
        });
        assert_eq!(error, Err(ExecutionMetricsError::NonMonotonicTimestamp));
        assert_eq!(collector, before);
    }

    #[test]
    fn operational_metrics_track_current_max_and_route_transitions() {
        let mut collector = ExecutionSloCollector::new();
        collector
            .observe_operational(ExecutionOperationalObservation {
                now_ns: 1_000,
                adapter_queue_depth: 2,
                command_queue_depth: 3,
                event_queue_depth: 4,
                wal_head_sequence: 100,
                wal_durable_sequence: 95,
                wal_durable_ns: Some(900),
                checkpoint_ns: Some(800),
                recovery_duration_ns: Some(50),
                reconciliation_mismatch_count: 2,
                route_health: ExecutionRouteHealth::Healthy,
                drop_copy_lag_ns: Some(25),
            })
            .unwrap();
        collector
            .observe_operational(ExecutionOperationalObservation {
                now_ns: 1_100,
                adapter_queue_depth: 1,
                command_queue_depth: 8,
                event_queue_depth: 2,
                wal_head_sequence: 110,
                wal_durable_sequence: 109,
                wal_durable_ns: Some(1_050),
                checkpoint_ns: Some(1_000),
                recovery_duration_ns: None,
                reconciliation_mismatch_count: 0,
                route_health: ExecutionRouteHealth::Degraded,
                drop_copy_lag_ns: Some(30),
            })
            .unwrap();
        let snapshot = collector.snapshot();
        assert_eq!(snapshot.queue_depth(ExecutionQueueKind::Command).current, 8);
        assert_eq!(snapshot.queue_depth(ExecutionQueueKind::Event).max, 4);
        assert_eq!(snapshot.wal_lag_records().current, 1);
        assert_eq!(snapshot.wal_lag_records().max, 5);
        assert_eq!(snapshot.checkpoint_age_ns().current, 100);
        assert_eq!(snapshot.route_health().transitions, 1);
        assert_eq!(snapshot.latency(ExecutionLatencyKind::Recovery).count, 1);
        assert_eq!(snapshot.latency(ExecutionLatencyKind::DropCopyLag).count, 2);
    }

    #[test]
    fn operational_validation_is_atomic() {
        let mut collector = ExecutionSloCollector::new();
        let before = collector.clone();
        let result = collector.observe_operational(ExecutionOperationalObservation {
            now_ns: 100,
            adapter_queue_depth: 1,
            command_queue_depth: 1,
            event_queue_depth: 1,
            wal_head_sequence: 9,
            wal_durable_sequence: 10,
            wal_durable_ns: Some(90),
            checkpoint_ns: Some(80),
            recovery_duration_ns: None,
            reconciliation_mismatch_count: 0,
            route_health: ExecutionRouteHealth::Healthy,
            drop_copy_lag_ns: None,
        });
        assert_eq!(result, Err(ExecutionMetricsError::WalSequenceAhead));
        assert_eq!(collector, before);
    }

    #[test]
    fn slo_evaluation_reports_latency_rate_queue_and_health_failures() {
        let mut collector = ExecutionSloCollector::new();
        collector
            .observe_submit(ExecutionSubmitObservation {
                submit_ns: 100,
                send_ns: 120,
                ack_ns: 200,
                fill_ns: None,
                outcome: ExecutionSubmitOutcome::Reject,
            })
            .unwrap();
        collector
            .observe_operational(ExecutionOperationalObservation {
                now_ns: 1_000,
                adapter_queue_depth: 20,
                command_queue_depth: 2,
                event_queue_depth: 1,
                wal_head_sequence: 100,
                wal_durable_sequence: 99,
                wal_durable_ns: Some(990),
                checkpoint_ns: Some(900),
                recovery_duration_ns: None,
                reconciliation_mismatch_count: 0,
                route_health: ExecutionRouteHealth::Degraded,
                drop_copy_lag_ns: None,
            })
            .unwrap();
        let report = collector.snapshot().evaluate(
            ExecutionSloTargets::new()
                .with_latency_p99_ns(ExecutionLatencyKind::SubmitToAck, 50)
                .with_reject_rate_ppm(100_000)
                .with_queue_depth(ExecutionQueueKind::Adapter, 10)
                .with_healthy_route_required(true),
        );
        assert!(!report.is_compliant());
        assert!(report
            .violations
            .contains(ExecutionSloViolationKind::SubmitToAckLatency));
        assert!(report
            .violations
            .contains(ExecutionSloViolationKind::RejectRate));
        assert!(report
            .violations
            .contains(ExecutionSloViolationKind::AdapterQueueDepth));
        assert!(report
            .violations
            .contains(ExecutionSloViolationKind::RouteHealth));
        assert_eq!(report.objectives_evaluated, 4);
        assert_eq!(report.objectives_violated, 4);
    }

    #[test]
    fn missing_samples_follow_explicit_policy() {
        let snapshot = ExecutionSloCollector::new().snapshot();
        let targets = ExecutionSloTargets::new()
            .with_latency_p99_ns(ExecutionLatencyKind::SubmitToAck, 100)
            .with_minimum_samples(10);
        let report = snapshot.evaluate(targets);
        assert!(report
            .violations
            .contains(ExecutionSloViolationKind::InsufficientSamples));
        assert_eq!(report.objectives_evaluated, 1);
        assert_eq!(report.objectives_violated, 1);
    }

    #[test]
    fn rejected_submit_with_fill_is_rejected_atomically() {
        let mut collector = ExecutionSloCollector::new();
        let before = collector.clone();
        let result = collector.observe_submit(ExecutionSubmitObservation {
            submit_ns: 100,
            send_ns: 110,
            ack_ns: 120,
            fill_ns: Some(130),
            outcome: ExecutionSubmitOutcome::Reject,
        });
        assert_eq!(result, Err(ExecutionMetricsError::InvalidObservation));
        assert_eq!(collector, before);
    }

    #[test]
    fn insufficient_objectives_are_counted_independently() {
        let report = ExecutionSloCollector::new().snapshot().evaluate(
            ExecutionSloTargets::new()
                .with_latency_p99_ns(ExecutionLatencyKind::SubmitToAck, 10)
                .with_latency_p99_ns(ExecutionLatencyKind::Fill, 10),
        );
        assert_eq!(report.objectives_evaluated, 2);
        assert_eq!(report.objectives_violated, 2);
        assert!(report
            .violations
            .contains(ExecutionSloViolationKind::InsufficientSamples));
    }
}
