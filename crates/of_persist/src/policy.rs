use super::*;

/// Integrity report for a normalized market-data WAL file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataWalIntegrityReport {
    /// True when all complete records validate.
    pub valid: bool,
    /// Complete records inspected.
    pub records: u64,
    /// Bytes consumed by complete records.
    pub bytes: u64,
    /// Number of checksum failures.
    pub checksum_failures: u64,
    /// Number of sequence continuity failures.
    pub sequence_failures: u64,
    /// True when the file ends in an incomplete frame.
    pub truncated_tail: bool,
    /// Last valid sequence.
    pub last_sequence: Option<MarketDataWalSequence>,
}

/// Low-latency market-data WAL metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataWalMetrics {
    /// Number of records written successfully.
    pub records_written: u64,
    /// Number of bytes written successfully.
    pub bytes_written: u64,
    /// Number of successful sync operations.
    pub sync_count: u64,
    /// Number of append failures.
    pub write_failures: u64,
    /// Number of sync failures.
    pub sync_failures: u64,
}

/// Production market-data persistence writer mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataPersistenceMode {
    /// Production market-data persistence is disabled.
    #[default]
    Disabled,
    /// Writes occur on the caller path and errors are returned immediately.
    InlineStrict,
    /// Writes are expected to be queued to a bounded single-writer worker.
    BoundedAsync,
    /// Writes may be dropped according to policy, with every drop counted.
    BestEffort,
}

/// Host action when production market-data persistence is degraded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataPersistenceFailureAction {
    /// Continue processing and mark persistence degraded.
    #[default]
    MarkDegraded,
    /// Stop market-data processing.
    StopMarketData,
    /// Stop trading while allowing market-data processing to continue.
    StopTrading,
    /// Fail the process.
    FailProcess,
    /// Switch to memory-only retention.
    MemoryOnly,
}

/// Production market-data persistence policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataPersistencePolicy {
    /// Writer mode.
    pub mode: MarketDataPersistenceMode,
    /// Bounded queue depth for async writer modes. Zero means unspecified.
    pub max_queue_depth: u32,
    /// Failure action selected by the host.
    pub failure_action: MarketDataPersistenceFailureAction,
}

impl MarketDataPersistencePolicy {
    /// Creates a disabled persistence policy.
    pub const fn disabled() -> Self {
        Self {
            mode: MarketDataPersistenceMode::Disabled,
            max_queue_depth: 0,
            failure_action: MarketDataPersistenceFailureAction::MarkDegraded,
        }
    }

    /// Creates an inline strict persistence policy.
    pub const fn inline_strict() -> Self {
        Self {
            mode: MarketDataPersistenceMode::InlineStrict,
            max_queue_depth: 0,
            failure_action: MarketDataPersistenceFailureAction::StopTrading,
        }
    }

    /// Creates a bounded async persistence policy.
    pub const fn bounded_async(max_queue_depth: u32) -> Self {
        Self {
            mode: MarketDataPersistenceMode::BoundedAsync,
            max_queue_depth,
            failure_action: MarketDataPersistenceFailureAction::MarkDegraded,
        }
    }

    /// Sets the failure action.
    pub const fn with_failure_action(
        mut self,
        failure_action: MarketDataPersistenceFailureAction,
    ) -> Self {
        self.failure_action = failure_action;
        self
    }

    /// Returns true when production persistence is enabled.
    pub const fn enabled(self) -> bool {
        !matches!(self.mode, MarketDataPersistenceMode::Disabled)
    }
}

impl Default for MarketDataPersistencePolicy {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Production market-data persistence health snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataPersistenceHealth {
    /// Configured persistence mode.
    pub mode: MarketDataPersistenceMode,
    /// True when production persistence is enabled.
    pub enabled: bool,
    /// True when the persistence path is degraded.
    pub degraded: bool,
    /// Current writer queue depth.
    pub queue_depth: u32,
    /// Writer lag measured in records.
    pub records_lag: u64,
    /// Writer lag measured in nanoseconds.
    pub lag_ns: u64,
    /// Bytes waiting to be persisted.
    pub bytes_pending: u64,
    /// Number of dropped records.
    pub dropped_records: u64,
    /// Number of WAL write failures.
    pub write_failures: u64,
    /// Number of WAL sync failures.
    pub sync_failures: u64,
    /// Last persistence error text.
    pub last_error: Option<String>,
}

impl MarketDataPersistenceHealth {
    /// Creates a health snapshot from policy and WAL metrics.
    pub fn from_wal_metrics(
        policy: MarketDataPersistencePolicy,
        metrics: MarketDataWalMetrics,
    ) -> Self {
        let degraded = metrics.write_failures > 0 || metrics.sync_failures > 0;
        Self {
            mode: policy.mode,
            enabled: policy.enabled(),
            degraded,
            write_failures: metrics.write_failures,
            sync_failures: metrics.sync_failures,
            ..Self::default()
        }
    }

    /// Sets queue and lag fields.
    pub const fn with_lag(
        mut self,
        queue_depth: u32,
        records_lag: u64,
        lag_ns: u64,
        bytes_pending: u64,
    ) -> Self {
        self.queue_depth = queue_depth;
        self.records_lag = records_lag;
        self.lag_ns = lag_ns;
        self.bytes_pending = bytes_pending;
        self
    }

    /// Sets drop count and marks the path degraded when records were dropped.
    pub const fn with_dropped_records(mut self, dropped_records: u64) -> Self {
        self.dropped_records = dropped_records;
        self.degraded = self.degraded || dropped_records > 0;
        self
    }

    /// Sets the last error and marks the path degraded.
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.last_error = Some(error.into());
        self.degraded = true;
        self
    }

    /// Returns true when the configured persistence path is enabled and not degraded.
    pub const fn is_healthy(&self) -> bool {
        self.enabled && !self.degraded
    }
}

/// Relative importance of a market-data persistence record under backpressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataRecordCriticality {
    /// Low-priority diagnostic or redundant depth record.
    Low,
    /// Normal market-data record.
    #[default]
    Normal,
    /// High-priority state transition or quality marker.
    High,
    /// Critical record that should not be dropped by policy helpers.
    Critical,
}

/// Backpressure drop policy for bounded market-data persistence writers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataBackpressureDropPolicy {
    /// Reject new persistence records when limits are exceeded.
    #[default]
    RejectNew,
    /// Drop the candidate record.
    DropNewest,
    /// Ask the host queue to drop its oldest queued record.
    DropOldest,
    /// Ask the host queue to drop a queued low-priority record.
    DropLowestPriority,
    /// Preserve trade records and drop lower-priority non-trade records first.
    PreserveTrades,
}

/// Backpressure condition that triggered a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataBackpressureReason {
    /// No backpressure condition is active.
    #[default]
    None,
    /// Writer queue depth reached or exceeded the configured bound.
    QueueDepth,
    /// Writer record lag reached or exceeded the configured bound.
    RecordsLag,
    /// Writer lag in nanoseconds reached or exceeded the configured bound.
    TimeLag,
    /// Pending bytes reached or exceeded the configured bound.
    BytesPending,
    /// Persistence path is already degraded.
    Degraded,
}

/// Backpressure action for one candidate persistence record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarketDataBackpressureAction {
    /// Accept the candidate record.
    #[default]
    Accept,
    /// Reject the candidate record without selecting a queued record to drop.
    Reject,
    /// Drop the candidate record.
    DropCurrent,
    /// Ask the host queue to drop its oldest queued record before accepting.
    DropQueuedOldest,
    /// Ask the host queue to drop a queued low-priority record before accepting.
    DropQueuedLowestPriority,
    /// Stop market-data processing.
    StopMarketData,
    /// Stop trading while allowing market-data processing to continue.
    StopTrading,
    /// Fail the process.
    FailProcess,
    /// Switch the host to memory-only retention.
    MemoryOnly,
}

/// Bounded writer backpressure policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarketDataBackpressurePolicy {
    /// Maximum writer queue depth. Zero disables this bound.
    pub max_queue_depth: u32,
    /// Maximum writer lag in records. Zero disables this bound.
    pub max_records_lag: u64,
    /// Maximum writer lag in nanoseconds. Zero disables this bound.
    pub max_lag_ns: u64,
    /// Maximum pending bytes. Zero disables this bound.
    pub max_bytes_pending: u64,
    /// Drop policy used when a limit is exceeded.
    pub drop_policy: MarketDataBackpressureDropPolicy,
    /// Minimum criticality that cannot be dropped by policy helpers.
    pub protected_criticality: MarketDataRecordCriticality,
    /// Failure action used when the persistence path is already degraded.
    pub failure_action: MarketDataPersistenceFailureAction,
}

impl MarketDataBackpressurePolicy {
    /// Creates a reject-new policy with queue-depth bound.
    pub const fn reject_new(max_queue_depth: u32) -> Self {
        Self {
            max_queue_depth,
            max_records_lag: 0,
            max_lag_ns: 0,
            max_bytes_pending: 0,
            drop_policy: MarketDataBackpressureDropPolicy::RejectNew,
            protected_criticality: MarketDataRecordCriticality::Critical,
            failure_action: MarketDataPersistenceFailureAction::MarkDegraded,
        }
    }

    /// Sets record lag bound.
    pub const fn with_max_records_lag(mut self, max_records_lag: u64) -> Self {
        self.max_records_lag = max_records_lag;
        self
    }

    /// Sets nanosecond lag bound.
    pub const fn with_max_lag_ns(mut self, max_lag_ns: u64) -> Self {
        self.max_lag_ns = max_lag_ns;
        self
    }

    /// Sets pending-byte bound.
    pub const fn with_max_bytes_pending(mut self, max_bytes_pending: u64) -> Self {
        self.max_bytes_pending = max_bytes_pending;
        self
    }

    /// Sets drop policy.
    pub const fn with_drop_policy(mut self, drop_policy: MarketDataBackpressureDropPolicy) -> Self {
        self.drop_policy = drop_policy;
        self
    }

    /// Sets minimum protected criticality.
    pub const fn with_protected_criticality(
        mut self,
        protected_criticality: MarketDataRecordCriticality,
    ) -> Self {
        self.protected_criticality = protected_criticality;
        self
    }

    /// Sets failure action for degraded persistence.
    pub const fn with_failure_action(
        mut self,
        failure_action: MarketDataPersistenceFailureAction,
    ) -> Self {
        self.failure_action = failure_action;
        self
    }
}

impl Default for MarketDataBackpressurePolicy {
    fn default() -> Self {
        Self::reject_new(0)
    }
}

/// Backpressure decision for one candidate market-data persistence record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MarketDataBackpressureDecision {
    /// Selected action.
    pub action: MarketDataBackpressureAction,
    /// Triggering reason.
    pub reason: MarketDataBackpressureReason,
    /// True when any backpressure condition was active.
    pub backpressured: bool,
    /// True when the selected action allows the candidate to be persisted.
    pub accepts_current: bool,
    /// True when the selected action drops a record.
    pub drops_record: bool,
    /// True when the selected action preserves trade records over lower-priority records.
    pub preserves_trade: bool,
}

impl MarketDataBackpressureDecision {
    /// Returns true when the decision is a hard stop instead of a drop/reject.
    pub const fn is_stop(self) -> bool {
        matches!(
            self.action,
            MarketDataBackpressureAction::StopMarketData
                | MarketDataBackpressureAction::StopTrading
                | MarketDataBackpressureAction::FailProcess
        )
    }
}
