use super::*;

/// Venue-specific order type and TIF capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueOrderCapabilities {
    /// Market orders are supported.
    pub market: bool,
    /// Limit orders are supported.
    pub limit: bool,
    /// Stop orders are supported.
    pub stop: bool,
    /// Stop-limit orders are supported.
    pub stop_limit: bool,
    /// Day orders are supported.
    pub tif_day: bool,
    /// GTC orders are supported.
    pub tif_gtc: bool,
    /// IOC orders are supported.
    pub tif_ioc: bool,
    /// FOK orders are supported.
    pub tif_fok: bool,
    /// GTD orders are supported.
    pub tif_gtd: bool,
}

impl From<ExecutionCapabilities> for VenueOrderCapabilities {
    fn from(value: ExecutionCapabilities) -> Self {
        Self {
            market: value.market,
            limit: value.limit,
            stop: value.stop,
            stop_limit: value.stop_limit,
            tif_day: value.tif_day,
            tif_gtc: value.tif_gtc,
            tif_ioc: value.tif_ioc,
            tif_fok: value.tif_fok,
            tif_gtd: value.tif_gtd,
        }
    }
}

/// Normalized venue order encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizedOrderType {
    /// Canonical order type.
    pub order_type: OrderType,
    /// Canonical time-in-force.
    pub time_in_force: TimeInForce,
}

/// Validates and normalizes order type/TIF against venue capabilities.
pub fn normalize_order_type(
    order_type: OrderType,
    time_in_force: TimeInForce,
    capabilities: VenueOrderCapabilities,
) -> Result<NormalizedOrderType, RiskRejectReason> {
    let order_supported = match order_type {
        OrderType::Market => capabilities.market,
        OrderType::Limit => capabilities.limit,
        OrderType::Stop => capabilities.stop,
        OrderType::StopLimit => capabilities.stop_limit,
    };
    if !order_supported {
        return Err(RiskRejectReason::UnsupportedOrderType);
    }
    let tif_supported = match time_in_force {
        TimeInForce::Day => capabilities.tif_day,
        TimeInForce::Gtc => capabilities.tif_gtc,
        TimeInForce::Ioc => capabilities.tif_ioc,
        TimeInForce::Fok => capabilities.tif_fok,
        TimeInForce::Gtd => capabilities.tif_gtd,
    };
    if !tif_supported {
        return Err(RiskRejectReason::UnsupportedTimeInForce);
    }
    Ok(NormalizedOrderType {
        order_type,
        time_in_force,
    })
}

/// Additive execution telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExecutionTelemetry {
    /// Last observed command queue depth.
    pub command_queue_depth: u32,
    /// Last observed report queue depth.
    pub report_queue_depth: u32,
    /// Submit-to-report latency sample count.
    pub latency_samples: u64,
    /// Minimum latency in nanoseconds.
    pub min_latency_ns: u64,
    /// Maximum latency in nanoseconds.
    pub max_latency_ns: u64,
    /// Sum of latency samples in nanoseconds.
    pub total_latency_ns: u128,
}

impl ExecutionTelemetry {
    /// Records one latency sample.
    pub fn record_latency(&mut self, latency_ns: u64) {
        self.latency_samples = self.latency_samples.saturating_add(1);
        self.min_latency_ns = if self.min_latency_ns == 0 {
            latency_ns
        } else {
            self.min_latency_ns.min(latency_ns)
        };
        self.max_latency_ns = self.max_latency_ns.max(latency_ns);
        self.total_latency_ns = self.total_latency_ns.saturating_add(u128::from(latency_ns));
    }

    /// Returns average latency in nanoseconds.
    pub fn average_latency_ns(&self) -> u64 {
        if self.latency_samples == 0 {
            0
        } else {
            (self.total_latency_ns / u128::from(self.latency_samples)) as u64
        }
    }
}

/// Timestamp source metadata used for audit and latency attribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TimestampSource {
    /// Source is unknown or was not supplied by the host.
    #[default]
    Unknown,
    /// Host software wall clock, typically Unix nanoseconds.
    SystemClock,
    /// Host monotonic clock converted into the trace's nanosecond domain.
    MonotonicClock,
    /// Hardware timestamp from a NIC, switch, capture card, or feed appliance.
    HardwareClock,
    /// Venue/exchange supplied timestamp.
    Venue,
    /// Drop-copy or post-trade channel timestamp.
    DropCopy,
    /// WAL or journal append timestamp.
    Journal,
    /// Checkpoint creation timestamp.
    Checkpoint,
    /// External host-specific timestamp source.
    External,
}

/// Timestamp point kind in a production execution workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TimestampPointKind {
    /// Strategy decision timestamp.
    StrategyDecision,
    /// OMS command receive/create timestamp.
    OmsReceive,
    /// WAL append timestamp.
    WalAppend,
    /// Adapter send timestamp.
    AdapterSend,
    /// Exchange or venue timestamp.
    Exchange,
    /// OMS execution-report receive timestamp.
    OmsReceiveReport,
    /// Drop-copy receive timestamp.
    DropCopyReceive,
    /// Checkpoint creation timestamp.
    Checkpoint,
}

/// Source metadata for every timestamp in an execution trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ExecutionTimestampSources {
    /// Source for [`ExecutionTimestampTrace::strategy_decision_ns`].
    pub strategy_decision: TimestampSource,
    /// Source for [`ExecutionTimestampTrace::oms_receive_ns`].
    pub oms_receive: TimestampSource,
    /// Source for [`ExecutionTimestampTrace::wal_append_ns`].
    pub wal_append: TimestampSource,
    /// Source for [`ExecutionTimestampTrace::adapter_send_ns`].
    pub adapter_send: TimestampSource,
    /// Source for [`ExecutionTimestampTrace::exchange_ns`].
    pub exchange: TimestampSource,
    /// Source for [`ExecutionTimestampTrace::oms_receive_report_ns`].
    pub oms_receive_report: TimestampSource,
    /// Source for [`ExecutionTimestampTrace::drop_copy_receive_ns`].
    pub drop_copy_receive: TimestampSource,
    /// Source for [`ExecutionTimestampTrace::checkpoint_ns`].
    pub checkpoint: TimestampSource,
}

/// Explicit timestamp trace for one execution workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ExecutionTimestampTrace {
    /// Strategy decision timestamp in nanoseconds.
    pub strategy_decision_ns: u64,
    /// OMS command receive/create timestamp in nanoseconds.
    pub oms_receive_ns: u64,
    /// WAL append timestamp in nanoseconds.
    pub wal_append_ns: u64,
    /// Adapter send timestamp in nanoseconds.
    pub adapter_send_ns: u64,
    /// Exchange or venue timestamp in nanoseconds.
    pub exchange_ns: u64,
    /// OMS execution-report receive timestamp in nanoseconds.
    pub oms_receive_report_ns: u64,
    /// Drop-copy receive timestamp in nanoseconds.
    pub drop_copy_receive_ns: u64,
    /// Checkpoint creation timestamp in nanoseconds.
    pub checkpoint_ns: u64,
    /// Source metadata for every timestamp.
    pub sources: ExecutionTimestampSources,
}

impl ExecutionTimestampTrace {
    /// Creates an empty timestamp trace.
    pub const fn new() -> Self {
        Self {
            strategy_decision_ns: 0,
            oms_receive_ns: 0,
            wal_append_ns: 0,
            adapter_send_ns: 0,
            exchange_ns: 0,
            oms_receive_report_ns: 0,
            drop_copy_receive_ns: 0,
            checkpoint_ns: 0,
            sources: ExecutionTimestampSources {
                strategy_decision: TimestampSource::Unknown,
                oms_receive: TimestampSource::Unknown,
                wal_append: TimestampSource::Unknown,
                adapter_send: TimestampSource::Unknown,
                exchange: TimestampSource::Unknown,
                oms_receive_report: TimestampSource::Unknown,
                drop_copy_receive: TimestampSource::Unknown,
                checkpoint: TimestampSource::Unknown,
            },
        }
    }

    /// Creates a trace from a new-order request.
    pub fn from_order_request(req: &OrderRequest) -> Self {
        Self::new()
            .with_oms_receive(req.ts_recv_ns, TimestampSource::SystemClock)
            .with_exchange(req.ts_exchange_ns, TimestampSource::Venue)
    }

    /// Adds strategy decision time.
    pub const fn with_strategy_decision(
        mut self,
        timestamp_ns: u64,
        source: TimestampSource,
    ) -> Self {
        self.strategy_decision_ns = timestamp_ns;
        self.sources.strategy_decision = source;
        self
    }

    /// Adds OMS receive time.
    pub const fn with_oms_receive(mut self, timestamp_ns: u64, source: TimestampSource) -> Self {
        self.oms_receive_ns = timestamp_ns;
        self.sources.oms_receive = source;
        self
    }

    /// Adds WAL append time.
    pub const fn with_wal_append(mut self, timestamp_ns: u64, source: TimestampSource) -> Self {
        self.wal_append_ns = timestamp_ns;
        self.sources.wal_append = source;
        self
    }

    /// Adds adapter send time.
    pub const fn with_adapter_send(mut self, timestamp_ns: u64, source: TimestampSource) -> Self {
        self.adapter_send_ns = timestamp_ns;
        self.sources.adapter_send = source;
        self
    }

    /// Adds exchange or venue time.
    pub const fn with_exchange(mut self, timestamp_ns: u64, source: TimestampSource) -> Self {
        self.exchange_ns = timestamp_ns;
        self.sources.exchange = source;
        self
    }

    /// Adds OMS execution-report receive time.
    pub const fn with_oms_receive_report(
        mut self,
        timestamp_ns: u64,
        source: TimestampSource,
    ) -> Self {
        self.oms_receive_report_ns = timestamp_ns;
        self.sources.oms_receive_report = source;
        self
    }

    /// Adds drop-copy receive time.
    pub const fn with_drop_copy_receive(
        mut self,
        timestamp_ns: u64,
        source: TimestampSource,
    ) -> Self {
        self.drop_copy_receive_ns = timestamp_ns;
        self.sources.drop_copy_receive = source;
        self
    }

    /// Adds checkpoint creation time.
    pub const fn with_checkpoint(mut self, timestamp_ns: u64, source: TimestampSource) -> Self {
        self.checkpoint_ns = timestamp_ns;
        self.sources.checkpoint = source;
        self
    }

    /// Adds exchange and OMS report receive times from an execution event.
    pub fn observe_event(mut self, event: &ExecutionEvent) -> Self {
        if event.ts_exchange_ns != 0 {
            self.exchange_ns = event.ts_exchange_ns;
            self.sources.exchange = TimestampSource::Venue;
        }
        self.oms_receive_report_ns = event.ts_recv_ns;
        self.sources.oms_receive_report = TimestampSource::SystemClock;
        self
    }

    /// Computes latency attribution from the trace.
    pub fn latency_attribution(&self) -> ExecutionLatencyAttribution {
        ExecutionLatencyAttribution {
            strategy_to_oms_ns: diff_if_ordered(self.strategy_decision_ns, self.oms_receive_ns),
            oms_to_wal_append_ns: diff_if_ordered(self.oms_receive_ns, self.wal_append_ns),
            wal_to_adapter_send_ns: diff_if_ordered(self.wal_append_ns, self.adapter_send_ns),
            adapter_to_exchange_ns: diff_if_ordered(self.adapter_send_ns, self.exchange_ns),
            exchange_to_oms_report_ns: diff_if_ordered(
                self.exchange_ns,
                self.oms_receive_report_ns,
            ),
            oms_report_to_drop_copy_ns: diff_if_ordered(
                self.oms_receive_report_ns,
                self.drop_copy_receive_ns,
            ),
            report_to_checkpoint_ns: diff_if_ordered(
                self.oms_receive_report_ns,
                self.checkpoint_ns,
            ),
            end_to_end_ns: first_last_latency(self),
        }
    }

    /// Validates timestamp ordering and exchange clock skew.
    pub fn validate(&self, config: TimestampDisciplineConfig) -> TimestampDisciplineReport {
        validate_timestamp_trace(self, config)
    }
}

/// Latency attribution derived from [`ExecutionTimestampTrace`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ExecutionLatencyAttribution {
    /// Strategy decision to OMS receive latency.
    pub strategy_to_oms_ns: Option<u64>,
    /// OMS receive to WAL append latency.
    pub oms_to_wal_append_ns: Option<u64>,
    /// WAL append to adapter send latency.
    pub wal_to_adapter_send_ns: Option<u64>,
    /// Adapter send to exchange timestamp latency.
    pub adapter_to_exchange_ns: Option<u64>,
    /// Exchange timestamp to OMS report receive latency.
    pub exchange_to_oms_report_ns: Option<u64>,
    /// OMS report receive to drop-copy receive latency.
    pub oms_report_to_drop_copy_ns: Option<u64>,
    /// OMS report receive to checkpoint latency.
    pub report_to_checkpoint_ns: Option<u64>,
    /// First observed timestamp to last observed timestamp latency.
    pub end_to_end_ns: Option<u64>,
}

/// Timestamp validation configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TimestampDisciplineConfig {
    /// Maximum tolerated exchange-vs-OMS clock skew in nanoseconds.
    pub max_clock_skew_ns: u64,
    /// Validate monotonic order for known internal timestamps.
    pub require_internal_monotonic: bool,
    /// Require a non-zero exchange timestamp when validating events.
    pub require_exchange_timestamp: bool,
}

impl Default for TimestampDisciplineConfig {
    fn default() -> Self {
        Self {
            max_clock_skew_ns: 1_000_000,
            require_internal_monotonic: true,
            require_exchange_timestamp: false,
        }
    }
}

/// Timestamp validation issue kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TimestampDisciplineIssueKind {
    /// A required timestamp is missing.
    MissingTimestamp,
    /// A later workflow point has an earlier timestamp.
    NonMonotonic,
    /// Exchange and OMS report timestamps exceed the configured skew budget.
    ClockSkewExceeded,
}

/// One timestamp validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TimestampDisciplineIssue {
    /// Issue kind.
    pub kind: TimestampDisciplineIssueKind,
    /// Earlier point in the comparison, or the missing point.
    pub first: TimestampPointKind,
    /// Later point in the comparison, or the missing point.
    pub second: TimestampPointKind,
    /// First timestamp value.
    pub first_ns: u64,
    /// Second timestamp value.
    pub second_ns: u64,
    /// Observed absolute delta in nanoseconds.
    pub delta_ns: u64,
}

/// Timestamp validation summary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct TimestampDisciplineReport {
    /// Number of validation issues.
    pub issue_count: u32,
    /// Maximum observed exchange-vs-OMS clock skew in nanoseconds.
    pub max_clock_skew_ns: u64,
    /// Per-issue detail.
    pub issues: Vec<TimestampDisciplineIssue>,
}

impl TimestampDisciplineReport {
    /// Returns true when no timestamp issues were found.
    pub const fn is_clean(&self) -> bool {
        self.issue_count == 0
    }
}

pub(crate) fn diff_if_ordered(start_ns: u64, end_ns: u64) -> Option<u64> {
    if start_ns == 0 || end_ns == 0 || end_ns < start_ns {
        None
    } else {
        Some(end_ns - start_ns)
    }
}

pub(crate) fn first_last_latency(trace: &ExecutionTimestampTrace) -> Option<u64> {
    let points = timestamp_points(trace);
    let first = points.iter().find(|(_, timestamp_ns)| *timestamp_ns != 0)?;
    let last = points
        .iter()
        .rev()
        .find(|(_, timestamp_ns)| *timestamp_ns != 0)?;
    diff_if_ordered(first.1, last.1)
}

pub(crate) fn timestamp_points(trace: &ExecutionTimestampTrace) -> [(TimestampPointKind, u64); 8] {
    [
        (
            TimestampPointKind::StrategyDecision,
            trace.strategy_decision_ns,
        ),
        (TimestampPointKind::OmsReceive, trace.oms_receive_ns),
        (TimestampPointKind::WalAppend, trace.wal_append_ns),
        (TimestampPointKind::AdapterSend, trace.adapter_send_ns),
        (TimestampPointKind::Exchange, trace.exchange_ns),
        (
            TimestampPointKind::OmsReceiveReport,
            trace.oms_receive_report_ns,
        ),
        (
            TimestampPointKind::DropCopyReceive,
            trace.drop_copy_receive_ns,
        ),
        (TimestampPointKind::Checkpoint, trace.checkpoint_ns),
    ]
}

pub(crate) fn internal_timestamp_points(
    trace: &ExecutionTimestampTrace,
) -> [(TimestampPointKind, u64); 7] {
    [
        (
            TimestampPointKind::StrategyDecision,
            trace.strategy_decision_ns,
        ),
        (TimestampPointKind::OmsReceive, trace.oms_receive_ns),
        (TimestampPointKind::WalAppend, trace.wal_append_ns),
        (TimestampPointKind::AdapterSend, trace.adapter_send_ns),
        (
            TimestampPointKind::OmsReceiveReport,
            trace.oms_receive_report_ns,
        ),
        (
            TimestampPointKind::DropCopyReceive,
            trace.drop_copy_receive_ns,
        ),
        (TimestampPointKind::Checkpoint, trace.checkpoint_ns),
    ]
}

pub(crate) fn validate_timestamp_trace(
    trace: &ExecutionTimestampTrace,
    config: TimestampDisciplineConfig,
) -> TimestampDisciplineReport {
    let mut report = TimestampDisciplineReport::default();
    if config.require_exchange_timestamp && trace.exchange_ns == 0 {
        report.issues.push(TimestampDisciplineIssue {
            kind: TimestampDisciplineIssueKind::MissingTimestamp,
            first: TimestampPointKind::Exchange,
            second: TimestampPointKind::Exchange,
            first_ns: 0,
            second_ns: 0,
            delta_ns: 0,
        });
    }

    if config.require_internal_monotonic {
        let mut previous: Option<(TimestampPointKind, u64)> = None;
        for (kind, timestamp_ns) in internal_timestamp_points(trace) {
            if timestamp_ns == 0 {
                continue;
            }
            if let Some((previous_kind, previous_ns)) = previous {
                if timestamp_ns < previous_ns {
                    report.issues.push(TimestampDisciplineIssue {
                        kind: TimestampDisciplineIssueKind::NonMonotonic,
                        first: previous_kind,
                        second: kind,
                        first_ns: previous_ns,
                        second_ns: timestamp_ns,
                        delta_ns: previous_ns - timestamp_ns,
                    });
                }
            }
            previous = Some((kind, timestamp_ns));
        }
    }

    if trace.exchange_ns != 0 && trace.oms_receive_report_ns != 0 {
        let skew = trace.exchange_ns.abs_diff(trace.oms_receive_report_ns);
        report.max_clock_skew_ns = skew;
        if skew > config.max_clock_skew_ns {
            report.issues.push(TimestampDisciplineIssue {
                kind: TimestampDisciplineIssueKind::ClockSkewExceeded,
                first: TimestampPointKind::Exchange,
                second: TimestampPointKind::OmsReceiveReport,
                first_ns: trace.exchange_ns,
                second_ns: trace.oms_receive_report_ns,
                delta_ns: skew,
            });
        }
    }

    report.issue_count = report.issues.len() as u32;
    report
}
