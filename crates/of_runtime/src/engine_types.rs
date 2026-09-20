use super::*;

pub(crate) const MAX_EVENTS_PER_POLL_ENV: &str = "OF_RUNTIME_MAX_EVENTS_PER_POLL";
pub(crate) const CIRCUIT_BREAKER_FAILURES_ENV: &str = "OF_RUNTIME_CIRCUIT_BREAKER_FAILURES";
pub(crate) const CIRCUIT_BREAKER_COOLDOWN_MS_ENV: &str = "OF_RUNTIME_CIRCUIT_BREAKER_COOLDOWN_MS";
pub(crate) const DEFAULT_CIRCUIT_BREAKER_COOLDOWN_MS: u64 = 1_000;
pub(crate) const MAX_ANALYTICS_WINDOW_LEN: u32 = 1_000_000;
pub(crate) const MAX_EVENT_TRACKER_LEN: u32 = 1_000_000;

/// Runtime engine configuration.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Logical runtime instance identifier.
    pub instance_id: String,
    /// Enables JSONL persistence via [`RollingStore`].
    pub enable_persistence: bool,
    /// Root directory for persisted data.
    pub data_root: String,
    /// Audit log file path.
    pub audit_log_path: String,
    /// Maximum bytes before audit log rotation.
    pub audit_max_bytes: u64,
    /// Number of rotated audit files to retain.
    pub audit_max_files: u32,
    /// Tokens to redact from audit details.
    pub audit_redact_tokens: Vec<String>,
    /// Max retained persisted bytes (0 disables).
    pub data_retention_max_bytes: u64,
    /// Max retained persisted age seconds (0 disables).
    pub data_retention_max_age_secs: u64,
    /// Adapter/provider configuration.
    pub adapter: AdapterConfig,
    /// Absolute delta threshold for default signal module.
    pub signal_threshold: i64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            instance_id: "default".to_string(),
            enable_persistence: false,
            data_root: "data".to_string(),
            audit_log_path: "audit/orderflow_audit.log".to_string(),
            audit_max_bytes: 10 * 1024 * 1024,
            audit_max_files: 5,
            audit_redact_tokens: vec![
                "secret".to_string(),
                "password".to_string(),
                "token".to_string(),
                "api_key".to_string(),
            ],
            data_retention_max_bytes: 0,
            data_retention_max_age_secs: 0,
            adapter: AdapterConfig::default(),
            signal_threshold: 100,
        }
    }
}

/// Runtime errors surfaced by engine lifecycle and processing.
#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// Adapter/provider error.
    Adapter(String),
    /// Configuration validation error.
    Config(String),
    /// Filesystem/I/O error.
    Io(String),
    /// Operation requires a started engine.
    NotStarted,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Adapter(v) => write!(f, "adapter error: {v}"),
            RuntimeError::Config(v) => write!(f, "config error: {v}"),
            RuntimeError::Io(v) => write!(f, "io error: {v}"),
            RuntimeError::NotStarted => write!(f, "engine not started"),
        }
    }
}

impl Error for RuntimeError {}

impl RuntimeError {
    /// Returns true when this error represents an opt-in runtime backpressure condition.
    pub fn is_backpressure(&self) -> bool {
        matches!(self, Self::Adapter(message) if message.starts_with("backpressure:"))
    }

    /// Returns true when this error represents an open adapter circuit breaker.
    pub fn is_circuit_open(&self) -> bool {
        matches!(self, Self::Adapter(message) if message.starts_with("circuit_open:"))
    }
}

/// Policy controlling quality constraints for externally-ingested feeds.
#[derive(Debug, Clone)]
pub struct ExternalFeedPolicy {
    /// Max allowed ingest silence before marking feed stale.
    pub stale_after_ms: u64,
    /// Enables sequence-gap/out-of-order checks.
    pub enforce_sequence: bool,
}

/// Runtime status for the active market-data adapter.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RuntimeAdapterStatus {
    /// Static adapter descriptor for the configured provider.
    pub descriptor: AdapterDescriptor,
    /// Latest health snapshot returned by the adapter.
    pub health: AdapterHealth,
    /// Typed provider operational state returned by the adapter.
    pub operational: AdapterOperationalStatus,
    /// Runtime health sequence at the time this status was read.
    pub health_seq: u64,
    /// True when the runtime has been started.
    pub started: bool,
    /// True when the adapter circuit breaker is currently open.
    pub circuit_breaker_open: bool,
}

impl Default for ExternalFeedPolicy {
    fn default() -> Self {
        Self {
            stale_after_ms: 15_000,
            enforce_sequence: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ExternalFeedState {
    pub(crate) enabled: bool,
    pub(crate) reconnecting: bool,
    pub(crate) policy: ExternalFeedPolicy,
    pub(crate) last_ingest_ns: Option<u64>,
    pub(crate) trade_seq: HashMap<SymbolId, u64>,
    pub(crate) book_seq: HashMap<SymbolId, u64>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CircuitBreakerState {
    pub(crate) failure_threshold: u32,
    pub(crate) cooldown_ms: u64,
    pub(crate) consecutive_failures: u32,
    pub(crate) open_until_ns: Option<u64>,
    pub(crate) opened_count: u64,
}

#[derive(Debug)]
pub(crate) struct RuntimeMarketDataPersistence {
    pub(crate) producer: MarketDataWalProducer,
    pub(crate) owned_writer: Option<BoundedMarketDataWalWriter>,
    pub(crate) failure_action: MarketDataPersistenceFailureAction,
    pub(crate) rejected_records: u64,
    pub(crate) last_error: Option<String>,
    pub(crate) memory_only: bool,
}

impl Drop for RuntimeMarketDataPersistence {
    fn drop(&mut self) {
        if let Some(writer) = self.owned_writer.take() {
            let _ = writer.shutdown();
        }
    }
}

// Admission reservations use drop guards, poisoned diagnostic locks are
// recovered, and the worker contains panics before exposing shared state. The
// channel implementation's conservative auto-trait result therefore does not
// invalidate the engine's historical unwind-safety contract.
impl std::panic::UnwindSafe for RuntimeMarketDataPersistence {}
impl std::panic::RefUnwindSafe for RuntimeMarketDataPersistence {}

impl RuntimeMarketDataPersistence {
    pub(crate) fn policy(&self) -> MarketDataPersistencePolicy {
        MarketDataPersistencePolicy::bounded_async(
            self.producer.queue_capacity().min(u32::MAX as usize) as u32,
        )
        .with_failure_action(self.failure_action)
    }

    pub(crate) fn health(&self) -> MarketDataPersistenceHealth {
        let metrics = self.producer.metrics();
        let records_lag = metrics
            .accepted_records
            .saturating_sub(metrics.written_records)
            .saturating_sub(metrics.abandoned_records);
        let dropped_records = self
            .rejected_records
            .saturating_add(metrics.abandoned_records);
        let mut health = MarketDataPersistenceHealth::default();
        health.mode = MarketDataPersistenceMode::BoundedAsync;
        health.enabled = !self.memory_only;
        health.degraded =
            self.memory_only || metrics.degraded || metrics.stopped || dropped_records > 0;
        health.queue_depth = metrics.queue_depth.min(u32::MAX as usize) as u32;
        health.records_lag = records_lag;
        health.lag_ns = metrics.event_time_lag_ns;
        health.bytes_pending = metrics.queued_payload_bytes as u64;
        health.dropped_records = dropped_records;
        health.write_failures = metrics.write_failures;
        health.sync_failures = metrics.sync_failures;
        health.last_error = self
            .last_error
            .clone()
            .or_else(|| self.producer.last_error());
        if self.memory_only {
            health.last_error.get_or_insert_with(|| {
                "market-data persistence switched to memory-only retention".to_owned()
            });
        }
        health
    }

    pub(crate) fn has_failure(&self) -> bool {
        let metrics = self.producer.metrics();
        self.rejected_records > 0
            || self.memory_only
            || metrics.degraded
            || metrics.stopped
            || metrics.abandoned_records > 0
    }

    pub(crate) fn blocks_market_data(&self) -> bool {
        self.has_failure()
            && matches!(
                self.failure_action,
                MarketDataPersistenceFailureAction::StopMarketData
                    | MarketDataPersistenceFailureAction::FailProcess
            )
    }

    pub(crate) fn blocks_trading(&self) -> bool {
        self.has_failure()
            && matches!(
                self.failure_action,
                MarketDataPersistenceFailureAction::StopTrading
                    | MarketDataPersistenceFailureAction::StopMarketData
                    | MarketDataPersistenceFailureAction::FailProcess
            )
    }
}

impl CircuitBreakerState {
    pub(crate) fn from_env() -> Self {
        let failure_threshold = std::env::var(CIRCUIT_BREAKER_FAILURES_ENV)
            .ok()
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let cooldown_ms = std::env::var(CIRCUIT_BREAKER_COOLDOWN_MS_ENV)
            .ok()
            .and_then(|raw| raw.trim().parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_CIRCUIT_BREAKER_COOLDOWN_MS);
        Self {
            failure_threshold,
            cooldown_ms,
            ..Self::default()
        }
    }

    pub(crate) fn configured(failure_threshold: u32, cooldown_ms: u64) -> Self {
        Self {
            failure_threshold,
            cooldown_ms: if cooldown_ms == 0 {
                DEFAULT_CIRCUIT_BREAKER_COOLDOWN_MS
            } else {
                cooldown_ms
            },
            ..Self::default()
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.failure_threshold > 0
    }

    pub(crate) fn is_open_at(&self, now_ns: u64) -> bool {
        self.open_until_ns
            .map(|open_until| now_ns < open_until)
            .unwrap_or(false)
    }

    pub(crate) fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.open_until_ns = None;
    }

    pub(crate) fn record_failure(&mut self, now_ns: u64) {
        if !self.enabled() {
            return;
        }
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= self.failure_threshold {
            self.open_until_ns =
                Some(now_ns.saturating_add(self.cooldown_ms.saturating_mul(1_000_000)));
            self.opened_count = self.opened_count.saturating_add(1);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct BookState {
    pub(crate) bids: BTreeMap<u16, BookLevel>,
    pub(crate) asks: BTreeMap<u16, BookLevel>,
    pub(crate) last_sequence: u64,
    pub(crate) ts_exchange_ns: u64,
    pub(crate) ts_recv_ns: u64,
}

impl BookState {
    pub(crate) fn on_book(&mut self, book: &BookUpdate) {
        let levels = match book.side {
            of_core::Side::Bid => &mut self.bids,
            of_core::Side::Ask => &mut self.asks,
        };

        match book.action {
            of_core::BookAction::Upsert => {
                levels.insert(
                    book.level,
                    BookLevel {
                        level: book.level,
                        price: book.price,
                        size: book.size,
                    },
                );
            }
            of_core::BookAction::Delete => {
                levels.remove(&book.level);
            }
        }

        self.last_sequence = book.sequence;
        self.ts_exchange_ns = book.ts_exchange_ns;
        self.ts_recv_ns = book.ts_recv_ns;
    }

    pub(crate) fn snapshot(&self, symbol: &SymbolId) -> BookSnapshot {
        BookSnapshot {
            symbol: symbol.clone(),
            bids: self.bids.values().cloned().collect(),
            asks: self.asks.values().cloned().collect(),
            last_sequence: self.last_sequence,
            ts_exchange_ns: self.ts_exchange_ns,
            ts_recv_ns: self.ts_recv_ns,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AuditLog {
    pub(crate) path: PathBuf,
    pub(crate) max_bytes: u64,
    pub(crate) max_files: u32,
    pub(crate) redact_tokens: Vec<String>,
}

impl AuditLog {
    pub(crate) fn new(
        path: impl AsRef<Path>,
        max_bytes: u64,
        max_files: u32,
        redact_tokens: Vec<String>,
    ) -> Result<Self, RuntimeError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            create_dir_all(parent).map_err(|e| RuntimeError::Io(e.to_string()))?;
        }
        Ok(Self {
            path,
            max_bytes,
            max_files,
            redact_tokens,
        })
    }

    pub(crate) fn append(&self, event: &str, details: &str) -> Result<(), RuntimeError> {
        let sanitized_details = redact_tokens(details, &self.redact_tokens);
        let line = format!(
            "{{\"event\":\"{}\",\"details\":{},\"ts\":{}}}\n",
            event,
            sanitized_details,
            unix_ts_secs()
        );
        self.rotate_if_needed(line.len() as u64)?;

        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| RuntimeError::Io(e.to_string()))?;
        f.write_all(line.as_bytes())
            .map_err(|e| RuntimeError::Io(e.to_string()))
    }

    pub(crate) fn rotate_if_needed(&self, incoming_len: u64) -> Result<(), RuntimeError> {
        if self.max_bytes == 0 {
            return Ok(());
        }
        let current_size = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        if current_size + incoming_len <= self.max_bytes {
            return Ok(());
        }
        self.rotate_files()
    }

    pub(crate) fn rotate_files(&self) -> Result<(), RuntimeError> {
        if self.max_files == 0 {
            if self.path.exists() {
                fs::remove_file(&self.path).map_err(|e| RuntimeError::Io(e.to_string()))?;
            }
            return Ok(());
        }

        let oldest = rotated_path(&self.path, self.max_files);
        if oldest.exists() {
            fs::remove_file(&oldest).map_err(|e| RuntimeError::Io(e.to_string()))?;
        }

        for idx in (1..self.max_files).rev() {
            let src = rotated_path(&self.path, idx);
            let dst = rotated_path(&self.path, idx + 1);
            if src.exists() {
                fs::rename(&src, &dst).map_err(|e| RuntimeError::Io(e.to_string()))?;
            }
        }

        if self.path.exists() {
            fs::rename(&self.path, rotated_path(&self.path, 1))
                .map_err(|e| RuntimeError::Io(e.to_string()))?;
        }
        Ok(())
    }
}

pub(crate) fn rotated_path(base: &Path, idx: u32) -> PathBuf {
    let mut p = base.as_os_str().to_os_string();
    p.push(format!(".{idx}"));
    PathBuf::from(p)
}

pub(crate) fn redact_tokens(input: &str, tokens: &[String]) -> String {
    let mut out = input.to_string();
    for token in tokens {
        if token.is_empty() {
            continue;
        }
        out = out.replace(token, "[REDACTED]");
        out = out.replace(&token.to_ascii_lowercase(), "[REDACTED]");
        out = out.replace(&token.to_ascii_uppercase(), "[REDACTED]");
    }
    out
}

pub(crate) fn unix_ts_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(crate) fn unix_ts_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

pub(crate) fn max_events_per_poll_from_env() -> Option<usize> {
    std::env::var(MAX_EVENTS_PER_POLL_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}

pub(crate) fn combine_quality_flags(
    lhs: DataQualityFlags,
    rhs: DataQualityFlags,
) -> DataQualityFlags {
    DataQualityFlags::from_bits_truncate(lhs.bits() | rhs.bits())
}
