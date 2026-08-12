use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::fs::{self, create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use of_adapters::{
    adapter_descriptors, create_adapter, describe_adapter, AdapterConfig, AdapterConnectionState,
    AdapterDescriptor, AdapterHealth, AdapterOperationalStatus, AdapterRuntimeMode,
    MarketDataAdapter, ProviderKind, RawEvent, SubscribeReq,
};
#[cfg(feature = "tickbar")]
use of_core::CompletedBar;
use of_core::{
    compute_book_analytics, compute_depth_slope, compute_effective_spread_bps,
    compute_lob_features, compute_mid_price, compute_weighted_average_price, ACDModel, ACDSnapshot,
    AgentTypeDetector, AgentTypeSnapshot, AlmgrenChriss, AlmgrenChrissSnapshot, AmihudSnapshot,
    AmihudTracker, AnalyticsAccumulator, AnalyticsConfig, AnalyticsSnapshot, BookAnalyticsSnapshot,
    BookEventAnalyticsSnapshot, BookEventTracker, BookLevel, BookSnapshot, BookUpdate,
    ClassificationVote, CvdEnhancementSnapshot, CvdEnhancements, DarkLitCorrelationSnapshot,
    DarkLitCorrelator, DarkPoolSnapshot, DarkPoolTracker, DataQualityFlags,
    DerivedAnalyticsSnapshot, FuturesSnapshot, FuturesTracker, HasbrouckSnapshot, HasbrouckVAR,
    InstitutionalFlowSnapshot, InstitutionalFlowTracker, IntervalCandleSnapshot,
    KineticEnergySnapshot, KineticEnergyTracker, KyleLambdaSnapshot, KyleLambdaTracker,
    LOBFeatureSnapshot, MicrostructureNoise, NoiseSnapshot, OIAnalysisSnapshot, OIAnalyzer,
    OptionsFlowSnapshot, OptionsFlowTracker, PatternDetector, PatternSnapshot, RegimeDetector,
    RegimeSnapshot, ResiliencySnapshot, ResiliencyTracker, SessionCandleSnapshot, SignalSnapshot,
    SignalState, SpreadDecomposition, SpreadDecompositionSnapshot, SpreadTracker, SymbolId,
    TradeClassifier, TradePrint, VolatilityEstimator, VolatilitySignature,
    VolatilitySignatureSnapshot, VolatilitySnapshot, VpinSnapshot, VpinTracker,
};
use of_persist::{
    decode_normalized_market_data_record, BoundedMarketDataWalWriter,
    BoundedMarketDataWriterConfig, BoundedMarketDataWriterMetrics,
    MarketDataPersistenceFailureAction, MarketDataPersistenceHealth, MarketDataPersistenceMode,
    MarketDataPersistencePolicy, MarketDataWalProducer, MarketDataWalRecord,
    NormalizedMarketDataRecordInput, RetentionPolicy, RollingStore, SegmentedMarketDataWalConfig,
};
use of_signals::{SignalExplanation, SignalGateDecision, SignalModule, SignalReasonCode};

use crate::config::config_hash;
use crate::validate_startup_config;

const MAX_EVENTS_PER_POLL_ENV: &str = "OF_RUNTIME_MAX_EVENTS_PER_POLL";
const CIRCUIT_BREAKER_FAILURES_ENV: &str = "OF_RUNTIME_CIRCUIT_BREAKER_FAILURES";
const CIRCUIT_BREAKER_COOLDOWN_MS_ENV: &str = "OF_RUNTIME_CIRCUIT_BREAKER_COOLDOWN_MS";
const DEFAULT_CIRCUIT_BREAKER_COOLDOWN_MS: u64 = 1_000;
const MAX_ANALYTICS_WINDOW_LEN: u32 = 1_000_000;
const MAX_EVENT_TRACKER_LEN: u32 = 1_000_000;

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
struct ExternalFeedState {
    enabled: bool,
    reconnecting: bool,
    policy: ExternalFeedPolicy,
    last_ingest_ns: Option<u64>,
    trade_seq: HashMap<SymbolId, u64>,
    book_seq: HashMap<SymbolId, u64>,
}

#[derive(Debug, Clone, Default)]
struct CircuitBreakerState {
    failure_threshold: u32,
    cooldown_ms: u64,
    consecutive_failures: u32,
    open_until_ns: Option<u64>,
    opened_count: u64,
}

#[derive(Debug)]
struct RuntimeMarketDataPersistence {
    producer: MarketDataWalProducer,
    owned_writer: Option<BoundedMarketDataWalWriter>,
    failure_action: MarketDataPersistenceFailureAction,
    rejected_records: u64,
    last_error: Option<String>,
    memory_only: bool,
}

impl Drop for RuntimeMarketDataPersistence {
    fn drop(&mut self) {
        if let Some(writer) = self.owned_writer.take() {
            let _ = writer.shutdown();
        }
    }
}

impl RuntimeMarketDataPersistence {
    fn policy(&self) -> MarketDataPersistencePolicy {
        MarketDataPersistencePolicy::bounded_async(
            self.producer.queue_capacity().min(u32::MAX as usize) as u32,
        )
        .with_failure_action(self.failure_action)
    }

    fn health(&self) -> MarketDataPersistenceHealth {
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

    fn has_failure(&self) -> bool {
        let metrics = self.producer.metrics();
        self.rejected_records > 0
            || self.memory_only
            || metrics.degraded
            || metrics.stopped
            || metrics.abandoned_records > 0
    }

    fn blocks_market_data(&self) -> bool {
        self.has_failure()
            && matches!(
                self.failure_action,
                MarketDataPersistenceFailureAction::StopMarketData
                    | MarketDataPersistenceFailureAction::FailProcess
            )
    }

    fn blocks_trading(&self) -> bool {
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
    fn from_env() -> Self {
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

    fn configured(failure_threshold: u32, cooldown_ms: u64) -> Self {
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

    fn enabled(&self) -> bool {
        self.failure_threshold > 0
    }

    fn is_open_at(&self, now_ns: u64) -> bool {
        self.open_until_ns
            .map(|open_until| now_ns < open_until)
            .unwrap_or(false)
    }

    fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.open_until_ns = None;
    }

    fn record_failure(&mut self, now_ns: u64) {
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
struct BookState {
    bids: BTreeMap<u16, BookLevel>,
    asks: BTreeMap<u16, BookLevel>,
    last_sequence: u64,
    ts_exchange_ns: u64,
    ts_recv_ns: u64,
}

impl BookState {
    fn on_book(&mut self, book: &BookUpdate) {
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

    fn snapshot(&self, symbol: &SymbolId) -> BookSnapshot {
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
struct AuditLog {
    path: PathBuf,
    max_bytes: u64,
    max_files: u32,
    redact_tokens: Vec<String>,
}

impl AuditLog {
    fn new(
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

    fn append(&self, event: &str, details: &str) -> Result<(), RuntimeError> {
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

    fn rotate_if_needed(&self, incoming_len: u64) -> Result<(), RuntimeError> {
        if self.max_bytes == 0 {
            return Ok(());
        }
        let current_size = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        if current_size + incoming_len <= self.max_bytes {
            return Ok(());
        }
        self.rotate_files()
    }

    fn rotate_files(&self) -> Result<(), RuntimeError> {
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

fn redact_tokens(input: &str, tokens: &[String]) -> String {
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

fn unix_ts_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn unix_ts_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn max_events_per_poll_from_env() -> Option<usize> {
    std::env::var(MAX_EVENTS_PER_POLL_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}

fn combine_quality_flags(lhs: DataQualityFlags, rhs: DataQualityFlags) -> DataQualityFlags {
    DataQualityFlags::from_bits_truncate(lhs.bits() | rhs.bits())
}

/// Runtime engine over a market-data adapter and signal module.
pub struct Engine<A: MarketDataAdapter, S: SignalModule> {
    cfg: EngineConfig,
    adapter: A,
    signal_module: S,
    started: bool,
    books: HashMap<SymbolId, BookState>,
    analytics: HashMap<SymbolId, AnalyticsAccumulator>,
    latest_signals: HashMap<SymbolId, SignalSnapshot>,
    latest_signal_explanations: HashMap<SymbolId, String>,
    processed_events: u64,
    persistence: Option<RollingStore>,
    market_data_persistence: Option<RuntimeMarketDataPersistence>,
    audit: Option<AuditLog>,
    health_seq: u64,
    last_health_fingerprint: String,
    last_quality_flags_bits: u32,
    last_events: Vec<RawEvent>,
    external: ExternalFeedState,
    max_events_per_poll: Option<usize>,
    backpressure_dropped_events: u64,
    circuit_breaker: CircuitBreakerState,
    #[cfg(feature = "tickbar")]
    tickbar_interval_ns: Option<i64>,
    /// Per-symbol spread trackers for effective/realised spread.
    spread_trackers: HashMap<SymbolId, SpreadTracker>,
    /// Per-symbol book event trackers for arrival/cancel rates.
    event_trackers: HashMap<SymbolId, BookEventTracker>,
    /// Per-symbol resiliency trackers for depth recovery.
    resiliency_trackers: HashMap<SymbolId, ResiliencyTracker>,
    /// Per-symbol trade classifiers.
    classifiers: HashMap<SymbolId, TradeClassifier>,
    /// Per-symbol VPIN trackers.
    vpin_trackers: HashMap<SymbolId, VpinTracker>,
    /// Per-symbol Kyle's Lambda trackers.
    kyle_lambda_trackers: HashMap<SymbolId, KyleLambdaTracker>,
    /// Per-symbol Amihud trackers.
    amihud_trackers: HashMap<SymbolId, AmihudTracker>,
    /// Per-symbol CVD enhancement trackers.
    cvd_enhancements: HashMap<SymbolId, CvdEnhancements>,
    /// Per-symbol pattern detectors.
    pattern_detectors: HashMap<SymbolId, PatternDetector>,
    /// Per-symbol volatility estimators.
    volatility_estimators: HashMap<SymbolId, VolatilityEstimator>,
    /// Per-symbol microstructure noise trackers.
    noise_trackers: HashMap<SymbolId, MicrostructureNoise>,
    /// Per-symbol Hasbrouck VAR trackers.
    hasbrouck_vars: HashMap<SymbolId, HasbrouckVAR>,
    /// Per-symbol Almgren-Chriss trackers.
    almgren_chriss: HashMap<SymbolId, AlmgrenChriss>,
    /// Per-symbol spread decomposition trackers.
    spread_decomps: HashMap<SymbolId, SpreadDecomposition>,
    /// Per-symbol ACD model trackers.
    acd_models: HashMap<SymbolId, ACDModel>,
    /// Previous trade timestamps for ACD duration calculation.
    prev_trade_ts: HashMap<SymbolId, u64>,
    /// Per-symbol regime detectors.
    regime_detectors: HashMap<SymbolId, RegimeDetector>,
    /// Per-symbol kinetic energy trackers.
    kinetic_trackers: HashMap<SymbolId, KineticEnergyTracker>,
    /// Per-symbol dark pool trackers.
    dark_pool_trackers: HashMap<SymbolId, DarkPoolTracker>,
    /// Per-symbol options flow trackers.
    options_trackers: HashMap<SymbolId, OptionsFlowTracker>,
    /// Per-symbol futures trackers.
    futures_trackers: HashMap<SymbolId, FuturesTracker>,
    /// Per-symbol volatility signature trackers.
    vol_signature_trackers: HashMap<SymbolId, VolatilitySignature>,
    /// Per-symbol agent-type detectors.
    agent_type_detectors: HashMap<SymbolId, AgentTypeDetector>,
    /// Per-symbol dark-lit correlators.
    dark_lit_correlators: HashMap<SymbolId, DarkLitCorrelator>,
    /// Per-symbol institutional flow trackers.
    institutional_flow: HashMap<SymbolId, InstitutionalFlowTracker>,
    /// Per-symbol OI analyzers.
    oi_analyzers: HashMap<SymbolId, OIAnalyzer>,
    /// Analytics thresholds and buffer sizes.
    analytics_config: AnalyticsConfig,
}

/// Default engine type used by C ABI and high-level bindings.
pub type DefaultEngine = Engine<Box<dyn MarketDataAdapter>, of_signals::DeltaMomentumSignal>;

/// Returns all known adapter descriptors as compact JSON.
pub fn adapter_inventory_json() -> String {
    format_adapter_inventory_json(None)
}

/// Returns built-in signal descriptors as compact JSON.
pub fn signal_descriptor_inventory_json() -> String {
    let signals = of_signals::built_in_signal_descriptors_json();
    format!("{{\"schema_version\":1,\"signals\":{signals}}}")
}

impl<A: MarketDataAdapter, S: SignalModule> Engine<A, S> {
    /// Creates an engine with explicit adapter and signal module.
    pub fn new(cfg: EngineConfig, adapter: A, signal_module: S) -> Self {
        Self {
            cfg,
            adapter,
            signal_module,
            started: false,
            books: HashMap::new(),
            analytics: HashMap::new(),
            latest_signals: HashMap::new(),
            latest_signal_explanations: HashMap::new(),
            processed_events: 0,
            persistence: None,
            market_data_persistence: None,
            audit: None,
            health_seq: 0,
            last_health_fingerprint: String::new(),
            last_quality_flags_bits: 0,
            last_events: Vec::new(),
            external: ExternalFeedState::default(),
            max_events_per_poll: max_events_per_poll_from_env(),
            backpressure_dropped_events: 0,
            circuit_breaker: CircuitBreakerState::from_env(),
            #[cfg(feature = "tickbar")]
            tickbar_interval_ns: None,
            spread_trackers: HashMap::new(),
            event_trackers: HashMap::new(),
            resiliency_trackers: HashMap::new(),
            classifiers: HashMap::new(),
            vpin_trackers: HashMap::new(),
            kyle_lambda_trackers: HashMap::new(),
            amihud_trackers: HashMap::new(),
            cvd_enhancements: HashMap::new(),
            pattern_detectors: HashMap::new(),
            volatility_estimators: HashMap::new(),
            noise_trackers: HashMap::new(),
            hasbrouck_vars: HashMap::new(),
            almgren_chriss: HashMap::new(),
            spread_decomps: HashMap::new(),
            acd_models: HashMap::new(),
            prev_trade_ts: HashMap::new(),
            regime_detectors: HashMap::new(),
            kinetic_trackers: HashMap::new(),
            dark_pool_trackers: HashMap::new(),
            options_trackers: HashMap::new(),
            futures_trackers: HashMap::new(),
            vol_signature_trackers: HashMap::new(),
            agent_type_detectors: HashMap::new(),
            dark_lit_correlators: HashMap::new(),
            institutional_flow: HashMap::new(),
            oi_analyzers: HashMap::new(),
            analytics_config: sanitize_analytics_config(AnalyticsConfig::default()),
        }
    }

    /// Override analytics thresholds and buffer sizes.
    pub fn set_analytics_config(&mut self, config: AnalyticsConfig) {
        self.analytics_config = sanitize_analytics_config(config);
    }

    /// Injects optional persistence backend.
    pub fn with_persistence(mut self, persistence: Option<RollingStore>) -> Self {
        self.persistence = persistence;
        self
    }

    /// Attaches an additive bounded normalized market-data WAL producer.
    ///
    /// The caller retains the writer handle and owns explicit flush/shutdown.
    /// Existing JSONL persistence and engine defaults are unchanged. Accepted
    /// canonical events are encoded on the writer thread rather than this
    /// runtime thread.
    pub fn with_market_data_wal_producer(
        mut self,
        producer: MarketDataWalProducer,
        failure_action: MarketDataPersistenceFailureAction,
    ) -> Self {
        self.set_market_data_wal_producer(Some(producer), failure_action);
        self
    }

    /// Replaces or clears the bounded normalized market-data WAL producer.
    pub fn set_market_data_wal_producer(
        &mut self,
        producer: Option<MarketDataWalProducer>,
        failure_action: MarketDataPersistenceFailureAction,
    ) {
        self.market_data_persistence = producer.map(|producer| RuntimeMarketDataPersistence {
            producer,
            owned_writer: None,
            failure_action,
            rejected_records: 0,
            last_error: None,
            memory_only: false,
        });
        self.update_health_state(DataQualityFlags::from_bits_truncate(
            self.last_quality_flags_bits,
        ));
    }

    /// Starts and owns a bounded normalized market-data WAL writer.
    ///
    /// This is a blocking control-plane operation because it validates and
    /// opens the WAL before returning. The engine shuts the writer down during
    /// drop, while [`Self::shutdown_market_data_persistence`] provides an
    /// explicit durability barrier and final metrics.
    ///
    /// # Errors
    /// Returns an error when persistence is already configured or the WAL
    /// cannot be opened.
    pub fn configure_market_data_wal(
        &mut self,
        wal_config: SegmentedMarketDataWalConfig,
        writer_config: BoundedMarketDataWriterConfig,
        failure_action: MarketDataPersistenceFailureAction,
    ) -> Result<(), RuntimeError> {
        if self.market_data_persistence.is_some() {
            return Err(RuntimeError::Config(
                "market-data persistence is already configured".to_owned(),
            ));
        }
        let writer = BoundedMarketDataWalWriter::start(wal_config, writer_config)
            .map_err(|error| RuntimeError::Io(error.to_string()))?;
        self.market_data_persistence = Some(RuntimeMarketDataPersistence {
            producer: writer.producer(),
            owned_writer: Some(writer),
            failure_action,
            rejected_records: 0,
            last_error: None,
            memory_only: false,
        });
        self.update_health_state(DataQualityFlags::from_bits_truncate(
            self.last_quality_flags_bits,
        ));
        Ok(())
    }

    /// Flushes an engine-owned market-data WAL through a durability barrier.
    ///
    /// # Errors
    /// Returns an error when persistence is disabled, the producer is
    /// externally owned, or the writer fails its barrier.
    pub fn flush_market_data_persistence(&self) -> Result<(), RuntimeError> {
        let persistence = self.market_data_persistence.as_ref().ok_or_else(|| {
            RuntimeError::Config("market-data persistence is not configured".to_owned())
        })?;
        let writer = persistence.owned_writer.as_ref().ok_or_else(|| {
            RuntimeError::Config(
                "market-data WAL producer is externally owned; flush it through its writer"
                    .to_owned(),
            )
        })?;
        writer
            .flush()
            .map_err(|error| RuntimeError::Io(error.to_string()))
    }

    /// Stops an engine-owned writer and disables production persistence.
    ///
    /// For an externally owned producer this only detaches the producer; its
    /// caller remains responsible for shutdown.
    ///
    /// # Errors
    /// Returns an error when the owned writer cannot drain, sync, or join.
    pub fn shutdown_market_data_persistence(
        &mut self,
    ) -> Result<Option<BoundedMarketDataWriterMetrics>, RuntimeError> {
        let Some(mut persistence) = self.market_data_persistence.take() else {
            return Ok(None);
        };
        let metrics = match persistence.owned_writer.take() {
            Some(writer) => writer
                .shutdown()
                .map_err(|error| RuntimeError::Io(error.to_string()))?,
            None => persistence.producer.metrics(),
        };
        self.update_health_state(DataQualityFlags::from_bits_truncate(
            self.last_quality_flags_bits,
        ));
        Ok(Some(metrics))
    }

    /// Returns bounded normalized market-data writer metrics when configured.
    pub fn market_data_writer_metrics(&self) -> Option<BoundedMarketDataWriterMetrics> {
        self.market_data_persistence
            .as_ref()
            .map(|persistence| persistence.producer.metrics())
    }

    /// Returns production normalized market-data persistence health.
    pub fn market_data_persistence_health(&self) -> MarketDataPersistenceHealth {
        self.market_data_persistence
            .as_ref()
            .map(RuntimeMarketDataPersistence::health)
            .unwrap_or_default()
    }

    /// Returns configured production persistence policy.
    pub fn market_data_persistence_policy(&self) -> MarketDataPersistencePolicy {
        self.market_data_persistence
            .as_ref()
            .map(RuntimeMarketDataPersistence::policy)
            .unwrap_or_default()
    }

    /// Returns whether persistence safety policy currently blocks trading.
    pub fn market_data_persistence_blocks_trading(&self) -> bool {
        self.market_data_persistence
            .as_ref()
            .is_some_and(RuntimeMarketDataPersistence::blocks_trading)
    }

    /// Returns production persistence health as stable compact JSON.
    pub fn market_data_persistence_health_json(&self) -> String {
        format_runtime_market_data_persistence_json(self.market_data_persistence.as_ref())
    }

    fn with_audit(mut self, audit: Option<AuditLog>) -> Self {
        self.audit = audit;
        self
    }

    /// Sets an optional per-poll event drain limit.
    ///
    /// `None` preserves the default unbounded drain behavior. `Some(0)` is
    /// treated as disabled. When a poll exceeds the configured limit, the
    /// runtime processes up to the limit, drops the remainder from that drain,
    /// sets the `ADAPTER_DEGRADED` quality flag, and returns a backpressure error.
    pub fn with_max_events_per_poll(mut self, max: Option<usize>) -> Self {
        self.max_events_per_poll = max.filter(|value| *value > 0);
        self
    }

    /// Sets adapter circuit-breaker policy for repeated poll failures.
    ///
    /// A `failure_threshold` of `0` disables the breaker and preserves legacy
    /// adapter polling behavior. When enabled, consecutive adapter poll errors
    /// open the circuit for `cooldown_ms` and surface a `circuit_open` adapter
    /// error on polls attempted during that window.
    pub fn with_circuit_breaker(mut self, failure_threshold: u32, cooldown_ms: u64) -> Self {
        self.circuit_breaker = CircuitBreakerState::configured(failure_threshold, cooldown_ms);
        self
    }

    /// Connects adapter and marks runtime as started.
    pub fn start(&mut self) -> Result<(), RuntimeError> {
        self.adapter
            .connect()
            .map_err(|e| RuntimeError::Adapter(e.to_string()))?;
        self.started = true;
        self.update_health_state(DataQualityFlags::NONE);
        self.audit_event(
            "engine_started",
            &format!(
                "{{\"instance_id\":\"{}\",\"config_hash\":\"{}\"}}",
                self.cfg.instance_id,
                config_hash(&self.cfg)
            ),
        )?;
        Ok(())
    }

    /// Stops runtime state and emits health transition.
    pub fn stop(&mut self) {
        self.started = false;
        let _ = self.audit_event("engine_stopping", "{\"draining\":true}");
        // Drain remaining adapter events
        let mut drain_buf = Vec::new();
        let _ = self.adapter.poll(&mut drain_buf);
        for event in drain_buf {
            let _ = match event {
                RawEvent::Trade(t) => self.ingest_trade(t, DataQualityFlags::NONE),
                RawEvent::Book(b) => self.ingest_book(b, DataQualityFlags::NONE),
            };
        }
        self.update_health_state(DataQualityFlags::NONE);
        let details = format!("{{\"processed_events\":{}}}", self.processed_events);
        let _ = self.audit_event("engine_stopped", &details);
    }

    /// Graceful shutdown with signal handler.
    pub fn shutdown_gracefully(&mut self) {
        let _ = self.audit_event("shutdown_initiated", "{}");
        self.stop();
        let _ = self.audit_event("shutdown_complete", "{}");
    }

    /// Subscribes to symbol stream through adapter.
    pub fn subscribe(&mut self, symbol: SymbolId, depth_levels: u16) -> Result<(), RuntimeError> {
        self.adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels,
            })
            .map_err(|e| RuntimeError::Adapter(e.to_string()))?;

        self.audit_event(
            "subscription_added",
            &format!(
                "{{\"venue\":\"{}\",\"symbol\":\"{}\",\"depth_levels\":{}}}",
                symbol.venue, symbol.symbol, depth_levels
            ),
        )?;
        self.update_health_state(DataQualityFlags::NONE);

        Ok(())
    }

    /// Unsubscribes symbol from adapter stream.
    pub fn unsubscribe(&mut self, symbol: SymbolId) -> Result<(), RuntimeError> {
        self.adapter
            .unsubscribe(symbol.clone())
            .map_err(|e| RuntimeError::Adapter(e.to_string()))?;

        self.audit_event(
            "subscription_removed",
            &format!(
                "{{\"venue\":\"{}\",\"symbol\":\"{}\"}}",
                symbol.venue, symbol.symbol
            ),
        )?;
        self.update_health_state(DataQualityFlags::NONE);
        Ok(())
    }

    /// Resets per-symbol analytics/session state.
    pub fn reset_symbol_session(&mut self, symbol: SymbolId) -> Result<(), RuntimeError> {
        if let Some(acc) = self.analytics.get_mut(&symbol) {
            acc.reset_session();
            let snap = acc.snapshot();
            self.signal_module.on_analytics(&snap);
            self.latest_signals
                .insert(symbol.clone(), self.signal_module.snapshot());
            self.cache_signal_explanation(&symbol, None);
        }
        self.audit_event(
            "session_reset",
            &format!(
                "{{\"venue\":\"{}\",\"symbol\":\"{}\"}}",
                symbol.venue, symbol.symbol
            ),
        )?;
        self.update_health_state(DataQualityFlags::NONE);
        Ok(())
    }

    /// Configures external-feed quality supervisor policy.
    pub fn configure_external_feed(
        &mut self,
        policy: ExternalFeedPolicy,
    ) -> Result<(), RuntimeError> {
        if !self.started {
            return Err(RuntimeError::NotStarted);
        }
        self.external.enabled = true;
        self.external.policy = policy;
        self.audit_event(
            "external_feed_configured",
            &format!(
                "{{\"stale_after_ms\":{},\"enforce_sequence\":{}}}",
                self.external.policy.stale_after_ms, self.external.policy.enforce_sequence
            ),
        )?;
        self.update_health_state(self.external_quality_flags());
        Ok(())
    }

    /// Marks external feed reconnecting/degraded state.
    pub fn set_external_reconnecting(&mut self, reconnecting: bool) -> Result<(), RuntimeError> {
        if !self.started {
            return Err(RuntimeError::NotStarted);
        }
        self.external.enabled = true;
        self.external.reconnecting = reconnecting;
        self.audit_event(
            "external_feed_reconnecting",
            &format!("{{\"reconnecting\":{reconnecting}}}"),
        )?;
        self.update_health_state(self.external_quality_flags());
        Ok(())
    }

    /// Re-evaluates health for external-feed stale policy without ingesting data.
    pub fn external_health_tick(&mut self) -> Result<(), RuntimeError> {
        if !self.started {
            return Err(RuntimeError::NotStarted);
        }
        self.external.enabled = true;
        self.update_health_state(self.external_quality_flags());
        Ok(())
    }

    /// Ingests a single external trade event.
    pub fn ingest_trade(
        &mut self,
        trade: TradePrint,
        quality_flags: DataQualityFlags,
    ) -> Result<(), RuntimeError> {
        if !self.started {
            return Err(RuntimeError::NotStarted);
        }
        self.external.enabled = true;
        let mut effective_quality =
            combine_quality_flags(quality_flags, self.external_quality_flags());
        let seq_flags = self.external_sequence_flags(&trade.symbol, trade.sequence, true);
        effective_quality = combine_quality_flags(effective_quality, seq_flags);
        self.external.last_ingest_ns = Some(unix_ts_nanos());
        let event = RawEvent::Trade(trade);
        self.last_events = vec![event.clone()];
        self.process_event(event, effective_quality, true)?;
        self.update_health_state(effective_quality);
        Ok(())
    }

    /// Ingests a single external book event.
    pub fn ingest_book(
        &mut self,
        book: BookUpdate,
        quality_flags: DataQualityFlags,
    ) -> Result<(), RuntimeError> {
        if !self.started {
            return Err(RuntimeError::NotStarted);
        }
        self.external.enabled = true;
        let mut effective_quality =
            combine_quality_flags(quality_flags, self.external_quality_flags());
        let seq_flags = self.external_sequence_flags(&book.symbol, book.sequence, false);
        effective_quality = combine_quality_flags(effective_quality, seq_flags);
        self.external.last_ingest_ns = Some(unix_ts_nanos());
        let event = RawEvent::Book(book);
        self.last_events = vec![event.clone()];
        self.process_event(event, effective_quality, true)?;
        self.update_health_state(effective_quality);
        Ok(())
    }

    /// Polls adapter once and processes all returned events.
    pub fn poll_once(&mut self, quality_flags: DataQualityFlags) -> Result<usize, RuntimeError> {
        if !self.started {
            return Err(RuntimeError::NotStarted);
        }

        let now_ns = unix_ts_nanos();
        if self.circuit_breaker.is_open_at(now_ns) {
            let effective_quality =
                combine_quality_flags(quality_flags, DataQualityFlags::ADAPTER_DEGRADED);
            self.update_health_state(effective_quality);
            return Err(RuntimeError::Adapter(format!(
                "circuit_open: cooldown_ms={} consecutive_failures={}",
                self.circuit_breaker.cooldown_ms, self.circuit_breaker.consecutive_failures
            )));
        }

        let mut events = Vec::new();
        if let Err(err) = self.adapter.poll(&mut events) {
            self.circuit_breaker.record_failure(now_ns);
            let effective_quality =
                combine_quality_flags(quality_flags, DataQualityFlags::ADAPTER_DEGRADED);
            self.update_health_state(effective_quality);
            return Err(RuntimeError::Adapter(err.to_string()));
        }
        self.circuit_breaker.record_success();

        let drained_events = events.len();
        let mut effective_quality = quality_flags;
        let backpressure = self
            .max_events_per_poll
            .filter(|max| drained_events > *max)
            .map(|max| {
                let dropped = drained_events.saturating_sub(max);
                self.backpressure_dropped_events = self
                    .backpressure_dropped_events
                    .saturating_add(dropped as u64);
                events.truncate(max);
                dropped
            });

        if backpressure.is_some() {
            effective_quality =
                combine_quality_flags(effective_quality, DataQualityFlags::ADAPTER_DEGRADED);
        }

        self.last_events = events.clone();

        for event in events {
            self.process_event(event, effective_quality, true)?;
        }
        self.update_health_state(effective_quality);

        if let Some(dropped) = backpressure {
            self.audit_event(
                "backpressure",
                &format!(
                    "{{\"drained_events\":{},\"processed_events\":{},\"dropped_events\":{},\"max_events_per_poll\":{}}}",
                    drained_events,
                    self.last_events.len(),
                    dropped,
                    self.max_events_per_poll.unwrap_or(0)
                ),
            )?;
            return Err(RuntimeError::Adapter(format!(
                "backpressure: drained_events={drained_events} processed_events={} dropped_events={dropped} max_events_per_poll={}",
                self.last_events.len(),
                self.max_events_per_poll.unwrap_or(0)
            )));
        }

        Ok(self.last_events.len())
    }

    /// Returns analytics snapshot for symbol if available.
    pub fn analytics_snapshot(&self, symbol: &SymbolId) -> Option<AnalyticsSnapshot> {
        self.analytics
            .get(symbol)
            .map(AnalyticsAccumulator::snapshot)
    }

    /// Returns additive derived analytics snapshot for symbol if available.
    pub fn derived_analytics_snapshot(
        &self,
        symbol: &SymbolId,
    ) -> Option<DerivedAnalyticsSnapshot> {
        self.analytics
            .get(symbol)
            .map(AnalyticsAccumulator::derived_snapshot)
    }

    /// Returns session candle snapshot for symbol if available.
    pub fn session_candle_snapshot(&self, symbol: &SymbolId) -> Option<SessionCandleSnapshot> {
        self.analytics
            .get(symbol)
            .map(AnalyticsAccumulator::session_candle_snapshot)
    }

    /// Returns rolling interval candle snapshot for symbol if available.
    pub fn interval_candle_snapshot(
        &self,
        symbol: &SymbolId,
        window_ns: u64,
    ) -> Option<IntervalCandleSnapshot> {
        self.analytics
            .get(symbol)
            .map(|acc| acc.interval_candle_snapshot(window_ns))
    }

    /// Sets the tickbar aggregation interval. New per-symbol accumulators will use this interval.
    /// Existing accumulators are not affected. Pass `None` to disable tickbar for new symbols.
    #[cfg(feature = "tickbar")]
    pub fn set_tickbar_interval(&mut self, interval_ns: Option<i64>) {
        self.tickbar_interval_ns = interval_ns;
    }

    /// Returns the configured tickbar interval, if any.
    #[cfg(feature = "tickbar")]
    pub fn tickbar_interval(&self) -> Option<i64> {
        self.tickbar_interval_ns
    }

    /// Returns completed tickbar series for symbol if a tickbar aggregator is configured.
    #[cfg(feature = "tickbar")]
    pub fn bar_series(&mut self, symbol: &SymbolId) -> Option<Vec<CompletedBar>> {
        self.analytics
            .get_mut(symbol)
            .and_then(AnalyticsAccumulator::bar_series)
    }

    /// Returns the current materialized book snapshot for symbol if available.
    pub fn book_snapshot(&self, symbol: &SymbolId) -> Option<BookSnapshot> {
        self.books.get(symbol).map(|book| book.snapshot(symbol))
    }

    /// Returns book-derived analytics (spread, depth, imbalance, microprice) for symbol if available.
    pub fn book_analytics_snapshot(&self, symbol: &SymbolId) -> Option<BookAnalyticsSnapshot> {
        self.books
            .get(symbol)
            .map(|book| compute_book_analytics(&book.snapshot(symbol)))
    }

    /// Returns the weighted average price for an order of `qty` shares by walking the book.
    ///
    /// Positive `qty` walks asks (buy), negative walks bids (sell).
    /// Returns `None` if the symbol has no book or liquidity is insufficient.
    pub fn weighted_average_price(&self, symbol: &SymbolId, qty: i64) -> Option<i64> {
        self.books
            .get(symbol)
            .and_then(|book| compute_weighted_average_price(&book.snapshot(symbol), qty))
    }

    /// Returns average volume decay per level for this symbol's book.
    ///
    /// Positive value indicates liquidity decreases with depth (typical).
    /// Returns `0.0` if the book has fewer than 2 levels or no data.
    pub fn depth_slope(&self, symbol: &SymbolId, levels: usize) -> f64 {
        self.books
            .get(symbol)
            .map(|book| compute_depth_slope(&book.snapshot(symbol), levels))
            .unwrap_or(0.0)
    }

    /// Returns mid price for symbol if the book has both sides.
    pub fn mid_price(&self, symbol: &SymbolId) -> Option<i64> {
        self.books
            .get(symbol)
            .and_then(|book| compute_mid_price(&book.snapshot(symbol)))
    }

    /// Returns the effective spread in bps for the most recent trade.
    pub fn effective_spread_bps(&self, symbol: &SymbolId) -> i64 {
        self.spread_trackers
            .get(symbol)
            .map(|st| st.last_effective_spread_bps())
            .unwrap_or(0)
    }

    /// Returns average half-spread cost in bps over the last `window` trades.
    pub fn half_spread_cost_bps(&self, symbol: &SymbolId, window: usize) -> i64 {
        self.spread_trackers
            .get(symbol)
            .map(|st| st.average_half_spread_cost_bps(window))
            .unwrap_or(0)
    }

    /// Returns realised spread in bps for the trade `hold_ticks` ago.
    pub fn realised_spread_bps(&self, symbol: &SymbolId, hold_ticks: usize) -> i64 {
        self.spread_trackers
            .get(symbol)
            .map(|st| st.realised_spread_bps(hold_ticks))
            .unwrap_or(0)
    }

    /// Returns book-event analytics snapshot for symbol over `window_ns`.
    pub fn book_event_analytics(
        &self,
        symbol: &SymbolId,
        window_ns: u64,
    ) -> BookEventAnalyticsSnapshot {
        self.event_trackers
            .get(symbol)
            .map(|bet| {
                let (bid_arr, ask_arr) = bet.arrival_rate_per_sec(window_ns);
                let (bid_can, ask_can) = bet.cancel_rate_per_sec(window_ns);
                let (bid_vol, ask_vol) = bet.event_volume_in_window(window_ns);
                let (bid_count, ask_count) = bet.event_count_in_window(window_ns, None);
                let total_count = bid_count + ask_count;
                let secs = (window_ns as f64) / 1_000_000_000.0;
                let change_intensity = if secs > 0.0 {
                    total_count as f64 / secs
                } else {
                    0.0
                };
                BookEventAnalyticsSnapshot {
                    bid_arrival_rate: bid_arr,
                    ask_arrival_rate: ask_arr,
                    bid_cancel_rate: bid_can,
                    ask_cancel_rate: ask_can,
                    change_intensity,
                    bid_event_volume: bid_vol,
                    ask_event_volume: ask_vol,
                }
            })
            .unwrap_or_default()
    }

    /// Returns resiliency snapshot for symbol.
    pub fn resiliency_snapshot(&self, symbol: &SymbolId) -> ResiliencySnapshot {
        self.resiliency_trackers
            .get(symbol)
            .map(|rt| ResiliencySnapshot {
                recovery_time_ms: rt.latest_recovery_time_ms().unwrap_or(0.0),
                depth_elasticity: rt.latest_depth_elasticity().unwrap_or(0.0),
            })
            .unwrap_or_default()
    }

    /// Returns the VPIN snapshot for symbol.
    pub fn vpin_snapshot(&self, symbol: &SymbolId) -> VpinSnapshot {
        self.vpin_trackers
            .get(symbol)
            .map(|v| v.snapshot())
            .unwrap_or_default()
    }

    /// Returns the Kyle's Lambda snapshot for symbol.
    pub fn kyle_lambda_snapshot(&self, symbol: &SymbolId) -> KyleLambdaSnapshot {
        self.kyle_lambda_trackers
            .get(symbol)
            .map(|k| k.snapshot())
            .unwrap_or_default()
    }

    /// Returns the Amihud illiquidity snapshot for symbol.
    pub fn amihud_snapshot(&self, symbol: &SymbolId) -> AmihudSnapshot {
        self.amihud_trackers
            .get(symbol)
            .map(|a| a.snapshot())
            .unwrap_or_default()
    }

    /// Returns the CVD enhancement snapshot for symbol.
    pub fn cvd_enhancement_snapshot(&self, symbol: &SymbolId) -> CvdEnhancementSnapshot {
        self.cvd_enhancements
            .get(symbol)
            .map(|c| c.snapshot())
            .unwrap_or_default()
    }

    /// Returns the pattern detection snapshot for symbol.
    pub fn pattern_snapshot(&self, symbol: &SymbolId) -> PatternSnapshot {
        self.pattern_detectors
            .get(symbol)
            .map(|pd| {
                let empty_book = BookSnapshot {
                    symbol: symbol.clone(),
                    bids: vec![],
                    asks: vec![],
                    last_sequence: 0,
                    ts_exchange_ns: 0,
                    ts_recv_ns: 0,
                };
                let book = self
                    .books
                    .get(symbol)
                    .map(|b| b.snapshot(symbol))
                    .unwrap_or(empty_book);
                pd.snapshot(&book, 0, 0.0, 0.0)
            })
            .unwrap_or_default()
    }

    /// Returns volatility snapshot for symbol.
    pub fn volatility_snapshot(&self, symbol: &SymbolId) -> VolatilitySnapshot {
        self.volatility_estimators
            .get(symbol)
            .map(|v| v.snapshot())
            .unwrap_or_default()
    }

    /// Returns microstructure noise snapshot for symbol.
    pub fn noise_snapshot(&self, symbol: &SymbolId) -> NoiseSnapshot {
        self.noise_trackers
            .get(symbol)
            .map(|n| n.snapshot())
            .unwrap_or_default()
    }

    /// Returns Hasbrouck VAR snapshot for symbol.
    pub fn hasbrouck_snapshot(&self, symbol: &SymbolId) -> HasbrouckSnapshot {
        self.hasbrouck_vars
            .get(symbol)
            .map(|h| h.snapshot())
            .unwrap_or_default()
    }

    /// Returns Almgren-Chriss snapshot for symbol.
    pub fn almgren_chriss_snapshot(&self, symbol: &SymbolId) -> AlmgrenChrissSnapshot {
        self.almgren_chriss
            .get(symbol)
            .map(|a| a.snapshot())
            .unwrap_or_default()
    }

    /// Returns spread decomposition snapshot for symbol.
    pub fn spread_decomp_snapshot(&self, symbol: &SymbolId) -> SpreadDecompositionSnapshot {
        self.spread_decomps
            .get(symbol)
            .map(|s| s.snapshot())
            .unwrap_or_default()
    }

    /// Returns ACD snapshot for symbol.
    pub fn acd_snapshot(&self, symbol: &SymbolId) -> ACDSnapshot {
        self.acd_models
            .get(symbol)
            .map(|a| a.snapshot())
            .unwrap_or_default()
    }

    /// Returns regime snapshot for symbol.
    pub fn regime_snapshot(&self, symbol: &SymbolId) -> RegimeSnapshot {
        self.regime_detectors
            .get(symbol)
            .map(|r| r.snapshot())
            .unwrap_or_default()
    }

    /// Returns kinetic-energy snapshot for symbol.
    pub fn kinetic_energy_snapshot(&self, symbol: &SymbolId) -> KineticEnergySnapshot {
        self.kinetic_trackers
            .get(symbol)
            .map(|k| k.snapshot())
            .unwrap_or_default()
    }

    /// Returns dark-pool analytics snapshot for symbol.
    pub fn dark_pool_snapshot(&self, symbol: &SymbolId) -> DarkPoolSnapshot {
        self.dark_pool_trackers
            .get(symbol)
            .map(|d| d.snapshot())
            .unwrap_or_default()
    }

    /// Returns options-flow analytics snapshot for symbol.
    pub fn options_flow_snapshot(&self, symbol: &SymbolId) -> OptionsFlowSnapshot {
        self.options_trackers
            .get(symbol)
            .map(|o| o.snapshot())
            .unwrap_or_default()
    }

    /// Returns futures basis and roll snapshot for symbol.
    pub fn futures_snapshot(&self, symbol: &SymbolId) -> FuturesSnapshot {
        self.futures_trackers
            .get(symbol)
            .map(|f| f.snapshot())
            .unwrap_or_default()
    }

    /// Returns volatility signature snapshot for symbol.
    pub fn vol_signature_snapshot(&self, symbol: &SymbolId) -> VolatilitySignatureSnapshot {
        self.vol_signature_trackers
            .get(symbol)
            .map(|v| v.snapshot())
            .unwrap_or_default()
    }

    /// Returns agent-type snapshot for symbol.
    pub fn agent_type_snapshot(&self, symbol: &SymbolId) -> AgentTypeSnapshot {
        self.agent_type_detectors
            .get(symbol)
            .map(|a| a.snapshot())
            .unwrap_or_default()
    }

    /// Returns dark-lit correlation snapshot for symbol.
    pub fn dark_lit_correlation_snapshot(&self, symbol: &SymbolId) -> DarkLitCorrelationSnapshot {
        self.dark_lit_correlators
            .get(symbol)
            .map(|d| d.snapshot())
            .unwrap_or_default()
    }

    /// Returns institutional flow snapshot for symbol.
    pub fn institutional_flow_snapshot(&self, symbol: &SymbolId) -> InstitutionalFlowSnapshot {
        self.institutional_flow
            .get(symbol)
            .map(|i| i.snapshot())
            .unwrap_or_default()
    }

    /// Returns OI analysis snapshot for symbol.
    pub fn oi_analysis_snapshot(&self, symbol: &SymbolId) -> OIAnalysisSnapshot {
        self.oi_analyzers
            .get(symbol)
            .map(|o| o.snapshot())
            .unwrap_or_default()
    }

    /// Computes LOB feature snapshot from internal book state for a symbol.
    pub fn lob_features(
        &self,
        symbol: &SymbolId,
        trade_imbalance: f64,
        cancel_rate: f64,
        arrival_rate: f64,
    ) -> LOBFeatureSnapshot {
        match self.books.get(symbol) {
            Some(book) => {
                let snap = book.snapshot(symbol);
                compute_lob_features(&snap, trade_imbalance, cancel_rate, arrival_rate)
            }
            None => LOBFeatureSnapshot::default(),
        }
    }

    /// Returns the last classification vote for symbol.
    pub fn last_classification(&self, symbol: &SymbolId) -> Option<ClassificationVote> {
        self.classifiers.get(symbol).map(|c| {
            let votes = c.last_votes();
            votes[0] // Return consensus (first element after classify())
        })
    }

    /// Returns latest signal snapshot for symbol if available.
    pub fn signal_snapshot(&self, symbol: &SymbolId) -> Option<SignalSnapshot> {
        self.latest_signals.get(symbol).cloned()
    }

    /// Returns latest signal explanation JSON for symbol if available.
    pub fn signal_explanation_json(&self, symbol: &SymbolId) -> Option<String> {
        self.latest_signal_explanations.get(symbol).cloned()
    }

    /// Returns static descriptor for the configured adapter provider.
    pub fn adapter_descriptor(&self) -> AdapterDescriptor {
        describe_adapter(self.cfg.adapter.provider.clone())
    }

    /// Returns latest active-adapter status.
    pub fn adapter_status(&self) -> RuntimeAdapterStatus {
        let health = self.adapter.health();
        RuntimeAdapterStatus {
            descriptor: self.adapter_descriptor(),
            operational: self.resolved_adapter_operational_status(&health),
            health,
            health_seq: self.health_seq,
            started: self.started,
            circuit_breaker_open: self.circuit_breaker.is_open_at(unix_ts_nanos()),
        }
    }

    fn resolved_adapter_operational_status(
        &self,
        health: &AdapterHealth,
    ) -> AdapterOperationalStatus {
        let mut operational = self.adapter.operational_status();
        if operational.mode == AdapterRuntimeMode::Unknown {
            operational.mode = if self.cfg.adapter.provider == ProviderKind::Mock
                || self
                    .cfg
                    .adapter
                    .endpoint
                    .as_deref()
                    .is_some_and(|endpoint| endpoint.starts_with("mock://"))
            {
                AdapterRuntimeMode::Mock
            } else if self.cfg.adapter.endpoint.is_some() {
                AdapterRuntimeMode::Live
            } else {
                AdapterRuntimeMode::Unknown
            };
        }
        if operational.connection_state == AdapterConnectionState::Unknown {
            operational.connection_state = if health.connected && !health.degraded {
                AdapterConnectionState::Streaming
            } else if health.connected {
                AdapterConnectionState::Reconnecting
            } else {
                AdapterConnectionState::Disconnected
            };
        }
        if operational.endpoint_redacted.is_none() {
            operational = operational.with_endpoint(self.cfg.adapter.endpoint.as_deref());
        }
        if operational.app_name.is_none() {
            operational = operational.with_app_name(self.cfg.adapter.app_name.as_deref());
        }
        operational
    }

    /// Returns adapter inventory as compact JSON.
    pub fn adapter_inventory_json(&self) -> String {
        format_adapter_inventory_json(Some(&self.cfg.adapter.provider))
    }

    /// Returns active adapter status as compact JSON.
    pub fn active_adapter_status_json(&self) -> String {
        format_adapter_status_json(&self.adapter_status())
    }

    /// Returns built-in signal descriptors as compact JSON.
    pub fn signal_descriptor_inventory_json(&self) -> String {
        signal_descriptor_inventory_json()
    }

    /// Returns signal metrics as compact JSON payload.
    pub fn signal_metrics_json(&self) -> String {
        let mut neutral = 0_u64;
        let mut long_bias = 0_u64;
        let mut short_bias = 0_u64;
        let mut blocked = 0_u64;
        let mut quality_flagged = 0_u64;
        let mut confidence_sum = 0_u64;

        for snapshot in self.latest_signals.values() {
            match snapshot.state {
                SignalState::Neutral => neutral += 1,
                SignalState::LongBias => long_bias += 1,
                SignalState::ShortBias => short_bias += 1,
                SignalState::Blocked => blocked += 1,
            }
            if snapshot.quality_flags != 0 {
                quality_flagged += 1;
            }
            confidence_sum += u64::from(snapshot.confidence_bps);
        }

        let total = self.latest_signals.len() as u64;
        let average_confidence_bps = confidence_sum.checked_div(total).unwrap_or(0);
        let latest_explanation_coverage_bps = (self.latest_signal_explanations.len() as u64)
            .saturating_mul(10_000)
            .checked_div(total)
            .unwrap_or(0);

        format!(
            "{{\"schema_version\":1,\"signal_symbols\":{},\"explanation_symbols\":{},\"neutral\":{},\"long_bias\":{},\"short_bias\":{},\"blocked\":{},\"directional\":{},\"quality_flagged\":{},\"average_confidence_bps\":{},\"last_quality_flags\":{},\"latest_explanation_coverage_bps\":{}}}",
            total,
            self.latest_signal_explanations.len(),
            neutral,
            long_bias,
            short_bias,
            blocked,
            long_bias + short_bias,
            quality_flagged,
            average_confidence_bps,
            self.last_quality_flags_bits,
            latest_explanation_coverage_bps
        )
    }

    /// Returns runtime metrics as compact JSON payload.
    pub fn metrics_json(&self) -> String {
        let adapter_health = self.adapter.health();
        let last_error_json = adapter_health
            .last_error
            .as_ref()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .unwrap_or_else(|| "null".to_string());
        let protocol_info_json = adapter_health
            .protocol_info
            .as_ref()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .unwrap_or_else(|| "null".to_string());
        let quality_flags_detail = quality_flags_detail_json(self.last_quality_flags_bits);
        let external_last_ingest = optional_u64_json(self.external.last_ingest_ns);
        let max_events_per_poll = optional_usize_json(self.max_events_per_poll);
        let circuit_open = self.circuit_breaker.is_open_at(unix_ts_nanos());
        let adapter_healthy =
            self.started && adapter_health.connected && !adapter_health.degraded && !circuit_open;
        let adapter_status_json = format_adapter_status_json(&RuntimeAdapterStatus {
            descriptor: self.adapter_descriptor(),
            operational: self.resolved_adapter_operational_status(&adapter_health),
            health: adapter_health.clone(),
            health_seq: self.health_seq,
            started: self.started,
            circuit_breaker_open: circuit_open,
        });
        let market_data_persistence = self.market_data_persistence_health();
        let market_data_persistence_json = self.market_data_persistence_health_json();
        let runtime_health_status = if !self.started || !adapter_health.connected {
            "disconnected"
        } else if adapter_health.degraded
            || circuit_open
            || self.external.reconnecting
            || self.last_quality_flags_bits != 0
            || market_data_persistence.degraded
        {
            "degraded"
        } else {
            "healthy"
        };

        format!(
            "{{\"instance_id\":\"{}\",\"started\":{},\"processed_events\":{},\"symbols\":{},\"book_symbols\":{},\"analytics_symbols\":{},\"signal_symbols\":{},\"persistence\":{},\"market_data_persistence\":{},\"health_seq\":{},\"quality_flags\":{},\"quality_flags_detail\":{},\"adapter_connected\":{},\"adapter_degraded\":{},\"adapter_last_error\":{},\"adapter_protocol_info\":{},\"adapter_total_count\":1,\"adapter_healthy_count\":{},\"runtime_health_status\":\"{}\",\"external_feed_enabled\":{},\"external_feed_reconnecting\":{},\"external_sequence_enforced\":{},\"external_stale_after_ms\":{},\"external_last_ingest_ns\":{},\"external_trade_sequence_symbols\":{},\"external_book_sequence_symbols\":{},\"max_events_per_poll\":{},\"backpressure_dropped_events\":{},\"circuit_breaker_enabled\":{},\"circuit_breaker_open\":{},\"circuit_breaker_consecutive_failures\":{},\"circuit_breaker_opened_count\":{},\"circuit_breaker_cooldown_ms\":{},\"adapters\":[{}]}}",
            escape_json(&self.cfg.instance_id),
            self.started,
            self.processed_events,
            self.tracked_symbol_count(),
            self.books.len(),
            self.analytics.len(),
            self.latest_signals.len(),
            self.persistence.is_some(),
            market_data_persistence_json,
            self.health_seq,
            self.last_quality_flags_bits,
            quality_flags_detail,
            adapter_health.connected,
            adapter_health.degraded,
            last_error_json,
            protocol_info_json,
            if adapter_healthy { 1 } else { 0 },
            runtime_health_status,
            self.external.enabled,
            self.external.reconnecting,
            self.external.policy.enforce_sequence,
            self.external.policy.stale_after_ms,
            external_last_ingest,
            self.external.trade_seq.len(),
            self.external.book_seq.len(),
            max_events_per_poll,
            self.backpressure_dropped_events,
            self.circuit_breaker.enabled(),
            circuit_open,
            self.circuit_breaker.consecutive_failures,
            self.circuit_breaker.opened_count,
            self.circuit_breaker.cooldown_ms,
            adapter_status_json
        )
    }

    /// Returns monotonic health sequence number.
    pub fn health_seq(&self) -> u64 {
        self.health_seq
    }

    /// Returns health snapshot as compact JSON payload.
    pub fn health_json(&self) -> String {
        let adapter_health = self.adapter.health();
        let circuit_open = self.circuit_breaker.is_open_at(unix_ts_nanos());
        let reconnect_state = if !adapter_health.connected {
            "disconnected"
        } else if adapter_health.degraded || self.external.reconnecting || circuit_open {
            "degraded"
        } else {
            "streaming"
        };
        let last_error_json = adapter_health
            .last_error
            .as_ref()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .unwrap_or_else(|| "null".to_string());
        let protocol_info_json = adapter_health
            .protocol_info
            .as_ref()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .unwrap_or_else(|| "null".to_string());
        let quality_flags_detail = quality_flags_detail_json(self.last_quality_flags_bits);
        let external_last_ingest = optional_u64_json(self.external.last_ingest_ns);
        let max_events_per_poll = optional_usize_json(self.max_events_per_poll);
        let adapter_healthy =
            self.started && adapter_health.connected && !adapter_health.degraded && !circuit_open;
        let market_data_persistence = self.market_data_persistence_health();
        let market_data_persistence_json = self.market_data_persistence_health_json();
        let runtime_health_status = if !self.started || !adapter_health.connected {
            "disconnected"
        } else if adapter_health.degraded
            || circuit_open
            || self.external.reconnecting
            || self.last_quality_flags_bits != 0
            || market_data_persistence.degraded
        {
            "degraded"
        } else {
            "healthy"
        };
        format!(
            "{{\"health_seq\":{},\"started\":{},\"connected\":{},\"degraded\":{},\"reconnect_state\":\"{}\",\"quality_flags\":{},\"quality_flags_detail\":{},\"last_error\":{},\"protocol_info\":{},\"tracked_symbols\":{},\"processed_events\":{},\"adapter_total_count\":1,\"adapter_healthy_count\":{},\"runtime_health_status\":\"{}\",\"external_feed_enabled\":{},\"external_feed_reconnecting\":{},\"external_sequence_enforced\":{},\"external_last_ingest_ns\":{},\"max_events_per_poll\":{},\"backpressure_dropped_events\":{},\"circuit_breaker_enabled\":{},\"circuit_breaker_open\":{},\"circuit_breaker_consecutive_failures\":{},\"circuit_breaker_opened_count\":{},\"circuit_breaker_cooldown_ms\":{},\"market_data_persistence\":{}}}",
            self.health_seq,
            self.started,
            adapter_health.connected,
            adapter_health.degraded,
            reconnect_state,
            self.last_quality_flags_bits,
            quality_flags_detail,
            last_error_json,
            protocol_info_json,
            self.tracked_symbol_count(),
            self.processed_events,
            if adapter_healthy { 1 } else { 0 },
            runtime_health_status,
            self.external.enabled,
            self.external.reconnecting,
            self.external.policy.enforce_sequence,
            external_last_ingest,
            max_events_per_poll,
            self.backpressure_dropped_events,
            self.circuit_breaker.enabled(),
            circuit_open,
            self.circuit_breaker.consecutive_failures,
            self.circuit_breaker.opened_count,
            self.circuit_breaker.cooldown_ms,
            market_data_persistence_json
        )
    }

    /// Returns events processed in the last poll/ingest cycle.
    pub fn last_events(&self) -> &[RawEvent] {
        &self.last_events
    }

    /// Replays one versioned normalized WAL record without writing it again.
    ///
    /// The event's persisted quality flags are applied to signal gating. The
    /// engine must be started. Non-book/trade records and malformed payloads
    /// fail closed.
    pub fn replay_normalized_wal_record(
        &mut self,
        record: &MarketDataWalRecord,
    ) -> Result<(), RuntimeError> {
        if !self.started {
            return Err(RuntimeError::NotStarted);
        }
        let input = decode_normalized_market_data_record(record)
            .map_err(|error| RuntimeError::Io(format!("normalized WAL decode failed: {error}")))?;
        let (event, quality_flags) = match input {
            NormalizedMarketDataRecordInput::Book {
                event,
                quality_flags_bits,
            } => (
                RawEvent::Book(event),
                DataQualityFlags::from_bits_truncate(quality_flags_bits),
            ),
            NormalizedMarketDataRecordInput::Trade {
                event,
                quality_flags_bits,
            } => (
                RawEvent::Trade(event),
                DataQualityFlags::from_bits_truncate(quality_flags_bits),
            ),
            _ => {
                return Err(RuntimeError::Io(
                    "normalized WAL record kind is unsupported by this runtime".to_owned(),
                ));
            }
        };
        self.last_events.clear();
        self.last_events.push(event.clone());
        self.process_event(event, quality_flags, false)?;
        self.update_health_state(quality_flags);
        Ok(())
    }

    /// Returns currently-active quality flags as raw bits.
    pub fn current_quality_flags_bits(&self) -> u32 {
        self.last_quality_flags_bits
    }

    fn tracked_symbol_count(&self) -> usize {
        let mut symbols = HashSet::new();
        symbols.extend(self.books.keys().cloned());
        symbols.extend(self.analytics.keys().cloned());
        symbols.extend(self.latest_signals.keys().cloned());
        symbols.len()
    }

    fn external_quality_flags(&self) -> DataQualityFlags {
        if !self.external.enabled {
            return DataQualityFlags::NONE;
        }

        let mut flags = DataQualityFlags::NONE;
        if self.external.reconnecting {
            flags = combine_quality_flags(flags, DataQualityFlags::ADAPTER_DEGRADED);
        }

        if self.external.policy.stale_after_ms > 0 {
            if let Some(last_ingest) = self.external.last_ingest_ns {
                let stale_after_ns = self
                    .external
                    .policy
                    .stale_after_ms
                    .saturating_mul(1_000_000);
                let age_ns = unix_ts_nanos().saturating_sub(last_ingest);
                if age_ns > stale_after_ns {
                    flags = combine_quality_flags(flags, DataQualityFlags::STALE_FEED);
                }
            }
        }

        flags
    }

    fn external_sequence_flags(
        &mut self,
        symbol: &SymbolId,
        sequence: u64,
        is_trade: bool,
    ) -> DataQualityFlags {
        if !self.external.policy.enforce_sequence || sequence == 0 {
            return DataQualityFlags::NONE;
        }

        let cache = if is_trade {
            &mut self.external.trade_seq
        } else {
            &mut self.external.book_seq
        };
        let mut flags = DataQualityFlags::NONE;

        if let Some(last) = cache.get(symbol).copied() {
            if sequence <= last {
                flags = combine_quality_flags(flags, DataQualityFlags::OUT_OF_ORDER);
            } else if sequence > last.saturating_add(1) {
                flags = combine_quality_flags(flags, DataQualityFlags::SEQUENCE_GAP);
            }
            if sequence > last {
                cache.insert(symbol.clone(), sequence);
            }
        } else {
            cache.insert(symbol.clone(), sequence);
        }

        flags
    }

    fn process_event(
        &mut self,
        event: RawEvent,
        quality_flags: DataQualityFlags,
        persist_to_wal: bool,
    ) -> Result<(), RuntimeError> {
        if persist_to_wal
            && self
                .market_data_persistence
                .as_ref()
                .is_some_and(RuntimeMarketDataPersistence::blocks_market_data)
        {
            return Err(RuntimeError::Io(
                "market-data persistence safety policy blocks event processing".to_owned(),
            ));
        }
        match event {
            RawEvent::Book(book) => {
                self.books
                    .entry(book.symbol.clone())
                    .or_default()
                    .on_book(&book);
                if let Some(store) = &self.persistence {
                    let _ = store.append_book(&book);
                }
                self.event_trackers
                    .entry(book.symbol.clone())
                    .or_insert_with(|| {
                        BookEventTracker::new(self.analytics_config.event_tracker_max_len as usize)
                    })
                    .on_book_update(book.side, book.action, book.size, book.ts_exchange_ns);
                if let Some(rt) = self.resiliency_trackers.get_mut(&book.symbol) {
                    let snapshot = self
                        .books
                        .get(&book.symbol)
                        .map(|b| b.snapshot(&book.symbol));
                    if let Some(snap) = snapshot {
                        let bid_depth: i64 = snap.bids.iter().map(|l| l.size).sum();
                        let ask_depth: i64 = snap.asks.iter().map(|l| l.size).sum();
                        rt.on_trade_post(bid_depth, ask_depth, book.ts_exchange_ns);
                    }
                }
                if let Some(pd) = self.pattern_detectors.get_mut(&book.symbol) {
                    pd.on_book_update(book.side, book.price, book.size);
                }
                self.processed_events += 1;
                if persist_to_wal {
                    self.persist_normalized_event(
                        NormalizedMarketDataRecordInput::book_with_quality(
                            book,
                            quality_flags.bits(),
                        ),
                    )?;
                }
            }
            RawEvent::Trade(trade) => {
                if let Some(store) = &self.persistence {
                    let _ = store.append_trade(&trade);
                }

                let symbol = trade.symbol.clone();

                // Feed spread tracker with mid price at trade time
                if let Some(book_state) = self.books.get(&symbol) {
                    let snapshot = book_state.snapshot(&symbol);
                    let bid = snapshot.bids.first().map(|l| l.price).unwrap_or(0);
                    let ask = snapshot.asks.first().map(|l| l.price).unwrap_or(0);
                    let mid = if bid > 0 && ask > 0 {
                        Some((bid + ask) / 2)
                    } else {
                        None
                    };
                    if let Some(mid) = mid {
                        self.spread_trackers
                            .entry(symbol.clone())
                            .or_insert_with(|| {
                                SpreadTracker::new(
                                    self.analytics_config.spread_tracker_max_len as usize,
                                )
                            })
                            .on_trade(trade.price, mid, trade.ts_exchange_ns);
                    }

                    // Classify trade and feed VPIN, Kyle's Lambda, CVD
                    let classifier = self.classifiers.entry(symbol.clone()).or_default();
                    let classification = classifier.classify(trade.price, trade.size, bid, ask);

                    // Determine classified buy/sell volume
                    let (buy_vol, sell_vol) = match classification {
                        ClassificationVote::Buy => (trade.size, 0),
                        ClassificationVote::Sell => (0, trade.size),
                        ClassificationVote::Neutral => {
                            // Split evenly at mid
                            (trade.size / 2, trade.size / 2)
                        }
                    };

                    // Feed VPIN
                    self.vpin_trackers
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            VpinTracker::new(
                                self.analytics_config.vpin_volume_bucket,
                                self.analytics_config.vpin_max_buckets as usize,
                            )
                        })
                        .on_trade(buy_vol, sell_vol);

                    // Feed Kyle's Lambda with signed volume and price change
                    let prev_price = self
                        .analytics
                        .get(&symbol)
                        .map(|a| a.snapshot().last_price)
                        .unwrap_or(trade.price);
                    let signed_vol = if buy_vol > 0 { trade.size } else { -trade.size };
                    let price_change = trade.price - prev_price;
                    self.kyle_lambda_trackers
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            KyleLambdaTracker::new(
                                self.analytics_config.kyle_lambda_max_len as usize,
                            )
                        })
                        .on_trade(signed_vol, price_change);

                    // Feed CVD enhancements with per-trade delta
                    let net_delta = buy_vol - sell_vol;
                    self.cvd_enhancements
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            CvdEnhancements::new(self.analytics_config.cvd_max_len as usize)
                        })
                        .on_bar(net_delta, trade.size, trade.price);

                    // Feed pattern detector (compute cumulative delta from prior state)
                    let prior_cumulative_delta = self
                        .analytics
                        .get(&symbol)
                        .map(|a| a.snapshot().cumulative_delta)
                        .unwrap_or(0);
                    let new_cumulative_delta = prior_cumulative_delta + net_delta;
                    self.pattern_detectors
                        .entry(symbol.clone())
                        .or_default()
                        .on_trade(
                            trade.price,
                            trade.size,
                            trade.aggressor_side,
                            trade.ts_exchange_ns,
                            new_cumulative_delta,
                            buy_vol,
                            sell_vol,
                        );

                    // Volatility estimator: per-trade OHLC
                    let prev_price_vol = self
                        .analytics
                        .get(&symbol)
                        .map(|a| a.snapshot().last_price)
                        .unwrap_or(trade.price);
                    let vol_est = self
                        .volatility_estimators
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            VolatilityEstimator::new(
                                self.analytics_config.vol_estimator_max_len as usize,
                            )
                        });
                    let high = trade.price.max(prev_price_vol) as f64;
                    let low = trade.price.min(prev_price_vol) as f64;
                    let close = trade.price as f64;
                    let open = prev_price_vol as f64;
                    vol_est.on_bar(open, high, low, close);

                    // Microstructure noise
                    self.noise_trackers
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            MicrostructureNoise::new(self.analytics_config.noise_max_len as usize)
                        })
                        .on_trade(trade.price, trade.size);

                    // Hasbrouck VAR
                    let ret = (trade.price as f64 / prev_price_vol.max(1) as f64).ln();
                    let signed_vol_f = if buy_vol > 0 {
                        trade.size as f64
                    } else {
                        -(trade.size as f64)
                    };
                    self.hasbrouck_vars
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            HasbrouckVAR::new(self.analytics_config.hasbrouck_max_len as usize)
                        })
                        .on_trade(ret, signed_vol_f);

                    // Almgren-Chriss
                    let pc = (trade.price - prev_price_vol) as f64;
                    self.almgren_chriss
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            AlmgrenChriss::new(
                                self.analytics_config.almgren_chriss_max_len as usize,
                            )
                        })
                        .on_trade(pc, signed_vol_f);

                    // ACD model (trade duration)
                    let prev_ts_acd = self.prev_trade_ts.get(&symbol).copied().unwrap_or(0);
                    self.acd_models
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            ACDModel::new(self.analytics_config.acd_max_len as usize)
                        })
                        .on_trade(trade.ts_exchange_ns, prev_ts_acd);
                    self.prev_trade_ts
                        .insert(symbol.clone(), trade.ts_exchange_ns);

                    // Volatility signature
                    self.vol_signature_trackers
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            VolatilitySignature::new(
                                self.analytics_config.vol_signature_max_len as usize,
                            )
                        })
                        .on_return(ret);

                    // Agent-type identification
                    let cr = self
                        .event_trackers
                        .get(&symbol)
                        .map(|et| {
                            et.cancel_rate_per_sec(self.analytics_config.cancel_arrival_window_ns)
                                .0
                        })
                        .unwrap_or(0.0);
                    let ar = self
                        .event_trackers
                        .get(&symbol)
                        .map(|et| {
                            et.arrival_rate_per_sec(self.analytics_config.cancel_arrival_window_ns)
                                .0
                        })
                        .unwrap_or(0.0);
                    self.agent_type_detectors
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            AgentTypeDetector::new(self.analytics_config.agent_max_len as usize)
                        })
                        .on_event(trade.size, cr, ar);

                    // Institutional flow (large trades only)
                    if trade.size > self.analytics_config.institutional_trade_threshold {
                        let is_buy = buy_vol > sell_vol;
                        self.institutional_flow
                            .entry(symbol.clone())
                            .or_insert_with(|| {
                                InstitutionalFlowTracker::new(
                                    self.analytics_config.institutional_max_len as usize,
                                )
                            })
                            .on_trade(trade.size, is_buy);
                    }

                    // Feed resiliency tracker with pre-trade depth
                    let bid_depth: i64 = snapshot.bids.iter().map(|l| l.size).sum();
                    let ask_depth: i64 = snapshot.asks.iter().map(|l| l.size).sum();
                    self.resiliency_trackers
                        .entry(symbol.clone())
                        .or_insert_with(|| {
                            ResiliencyTracker::new(
                                self.analytics_config.resiliency_max_len as usize,
                            )
                        })
                        .on_trade_pre(bid_depth, ask_depth);
                }
                #[cfg(feature = "tickbar")]
                let tickbar_ns = self.tickbar_interval_ns;
                let acc = self.analytics.entry(symbol.clone()).or_insert_with(|| {
                    #[cfg(feature = "tickbar")]
                    if let Some(ns) = tickbar_ns {
                        return AnalyticsAccumulator::with_tickbar(ns);
                    }
                    AnalyticsAccumulator::default()
                });
                acc.on_trade(&trade);
                let snap = acc.snapshot();
                self.signal_module.on_analytics(&snap);

                let mut signal = self.signal_module.snapshot();
                if self.signal_module.quality_gate(quality_flags) == SignalGateDecision::Block {
                    signal.state = SignalState::Blocked;
                    signal.quality_flags = quality_flags.bits();
                    signal.reason = "blocked_by_quality_gate".to_string();
                    self.audit_event(
                        "signal_blocked",
                        &format!(
                            "{{\"venue\":\"{}\",\"symbol\":\"{}\",\"quality_flags\":{}}}",
                            symbol.venue,
                            symbol.symbol,
                            quality_flags.bits()
                        ),
                    )?;
                }
                self.cache_signal_explanation(&symbol, Some(&signal));
                self.latest_signals.insert(symbol.clone(), signal);

                // Spread decomposition
                if let Some(st) = self.spread_trackers.get(&symbol) {
                    let eff = st.last_effective_spread_bps() as f64;
                    let real = st.realised_spread_bps(10) as f64;
                    if eff > 0.0 || real > 0.0 {
                        let quoted = if let Some(book_state) = self.books.get(&symbol) {
                            let bs = book_state.snapshot(&symbol);
                            let bid = bs.bids.first().map(|l| l.price).unwrap_or(0);
                            let ask = bs.asks.first().map(|l| l.price).unwrap_or(0);
                            if bid > 0 && ask > 0 {
                                compute_effective_spread_bps(ask, bid) as f64
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        };
                        self.spread_decomps
                            .entry(symbol.clone())
                            .or_insert_with(|| {
                                SpreadDecomposition::new(
                                    self.analytics_config.spread_decomp_max_len as usize,
                                )
                            })
                            .on_spread(eff, real, quoted);
                    }
                }

                // Regime detector
                let vpin_s = self.vpin_trackers.get(&symbol).map(|v| v.snapshot());
                let vol_s = self
                    .volatility_estimators
                    .get(&symbol)
                    .map(|v| v.snapshot());
                let spread_val = self
                    .spread_trackers
                    .get(&symbol)
                    .map(|s| s.last_effective_spread_bps() as f64)
                    .unwrap_or(0.0);
                let vol_val = vol_s.map(|v| v.classic_rv).unwrap_or(0.0);
                let vpin_val = vpin_s.map(|v| v.vpin).unwrap_or(0.0);
                self.regime_detectors
                    .entry(symbol.clone())
                    .or_insert_with(|| {
                        RegimeDetector::new(self.analytics_config.regime_max_len as usize)
                    })
                    .on_metrics(spread_val, vol_val, vpin_val);

                self.processed_events += 1;
                if persist_to_wal {
                    self.persist_normalized_event(
                        NormalizedMarketDataRecordInput::trade_with_quality(
                            trade,
                            quality_flags.bits(),
                        ),
                    )?;
                }
            }
        }

        Ok(())
    }

    fn persist_normalized_event(
        &mut self,
        input: NormalizedMarketDataRecordInput,
    ) -> Result<(), RuntimeError> {
        let Some(persistence) = self.market_data_persistence.as_mut() else {
            return Ok(());
        };
        if persistence.memory_only {
            return Ok(());
        }
        if let Err(error) = persistence.producer.try_append_normalized_owned(input) {
            persistence.rejected_records = persistence.rejected_records.saturating_add(1);
            persistence.last_error = Some(error.to_string());
            match persistence.failure_action {
                MarketDataPersistenceFailureAction::MarkDegraded
                | MarketDataPersistenceFailureAction::StopTrading => return Ok(()),
                MarketDataPersistenceFailureAction::MemoryOnly => {
                    persistence.memory_only = true;
                    return Ok(());
                }
                MarketDataPersistenceFailureAction::StopMarketData => {
                    return Err(RuntimeError::Io(format!(
                        "market-data persistence stopped event processing: {error}"
                    )));
                }
                MarketDataPersistenceFailureAction::FailProcess => {
                    return Err(RuntimeError::Io(format!(
                        "market-data persistence fail-closed policy triggered: {error}"
                    )));
                }
                _ => {
                    return Err(RuntimeError::Io(format!(
                        "unknown market-data persistence failure action triggered: {error}"
                    )));
                }
            }
        }
        Ok(())
    }

    fn cache_signal_explanation(&mut self, symbol: &SymbolId, emitted: Option<&SignalSnapshot>) {
        let explanation = match emitted {
            Some(snapshot) if snapshot.state == SignalState::Blocked => Some(
                SignalExplanation::from_snapshot(snapshot, SignalReasonCode::QualityBlocked),
            ),
            _ => self.signal_module.latest_explanation(),
        };

        match explanation {
            Some(explanation) => {
                self.latest_signal_explanations
                    .insert(symbol.clone(), explanation.to_json());
            }
            None => {
                self.latest_signal_explanations.remove(symbol);
            }
        }
    }

    fn audit_event(&self, event: &str, details_json: &str) -> Result<(), RuntimeError> {
        if let Some(audit) = &self.audit {
            audit.append(event, details_json)?;
        }
        Ok(())
    }

    fn update_health_state(&mut self, quality_flags: DataQualityFlags) {
        self.last_quality_flags_bits = quality_flags.bits();
        let adapter_health = self.adapter.health();
        let persistence_health = self.market_data_persistence_health();
        let fingerprint = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.started,
            adapter_health.connected,
            adapter_health.degraded,
            self.last_quality_flags_bits,
            adapter_health.last_error.as_deref().unwrap_or(""),
            adapter_health.protocol_info.as_deref().unwrap_or(""),
            self.backpressure_dropped_events,
            self.circuit_breaker.is_open_at(unix_ts_nanos()),
            self.circuit_breaker.consecutive_failures,
            self.circuit_breaker.opened_count,
            persistence_health.enabled,
            persistence_health.degraded,
            persistence_health.dropped_records,
            persistence_health.last_error.as_deref().unwrap_or("")
        );
        if fingerprint != self.last_health_fingerprint {
            self.health_seq = self.health_seq.saturating_add(1);
            self.last_health_fingerprint = fingerprint;
        }
    }
}

fn escape_json(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn optional_u64_json(value: Option<u64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn optional_usize_json(value: Option<usize>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn optional_str_json(value: Option<&str>) -> String {
    value
        .map(|v| format!("\"{}\"", escape_json(v)))
        .unwrap_or_else(|| "null".to_string())
}

fn format_runtime_market_data_persistence_json(
    persistence: Option<&RuntimeMarketDataPersistence>,
) -> String {
    let Some(persistence) = persistence else {
        return "{\"schema_version\":1,\"mode\":\"disabled\",\"enabled\":false,\"degraded\":false,\"memory_only\":false,\"failure_action\":\"mark_degraded\",\"blocks_trading\":false,\"accepted_records\":0,\"written_records\":0,\"abandoned_records\":0,\"records_lag\":0,\"event_time_lag_ns\":0,\"latest_accepted_ts_recv_ns\":0,\"latest_written_ts_recv_ns\":0,\"queue_depth\":0,\"queue_capacity\":0,\"queue_high_watermark\":0,\"queued_payload_bytes\":0,\"queued_payload_capacity\":0,\"queued_payload_high_watermark\":0,\"rejected_records\":0,\"write_failures\":0,\"sync_failures\":0,\"last_written_sequence\":null,\"last_durable_sequence\":null,\"last_error\":null}".to_owned();
    };
    let metrics = persistence.producer.metrics();
    let health = persistence.health();
    format!(
        "{{\"schema_version\":1,\"mode\":\"bounded_async\",\"enabled\":{},\"degraded\":{},\"memory_only\":{},\"failure_action\":\"{}\",\"blocks_trading\":{},\"accepted_records\":{},\"written_records\":{},\"abandoned_records\":{},\"records_lag\":{},\"event_time_lag_ns\":{},\"latest_accepted_ts_recv_ns\":{},\"latest_written_ts_recv_ns\":{},\"queue_depth\":{},\"queue_capacity\":{},\"queue_high_watermark\":{},\"queued_payload_bytes\":{},\"queued_payload_capacity\":{},\"queued_payload_high_watermark\":{},\"rejected_records\":{},\"write_failures\":{},\"sync_failures\":{},\"last_written_sequence\":{},\"last_durable_sequence\":{},\"last_error\":{}}}",
        health.enabled,
        health.degraded,
        persistence.memory_only,
        persistence_failure_action_id(persistence.failure_action),
        persistence.blocks_trading(),
        metrics.accepted_records,
        metrics.written_records,
        metrics.abandoned_records,
        health.records_lag,
        metrics.event_time_lag_ns,
        metrics.latest_accepted_ts_recv_ns,
        metrics.latest_written_ts_recv_ns,
        metrics.queue_depth,
        persistence.producer.queue_capacity(),
        metrics.queue_high_watermark,
        metrics.queued_payload_bytes,
        persistence.producer.max_queued_payload_bytes(),
        metrics.queued_payload_high_watermark,
        persistence.rejected_records,
        metrics.write_failures,
        metrics.sync_failures,
        optional_wal_sequence_json(metrics.last_written_sequence),
        optional_wal_sequence_json(metrics.last_synced_sequence),
        optional_str_json(health.last_error.as_deref())
    )
}

fn persistence_failure_action_id(action: MarketDataPersistenceFailureAction) -> &'static str {
    match action {
        MarketDataPersistenceFailureAction::MarkDegraded => "mark_degraded",
        MarketDataPersistenceFailureAction::StopMarketData => "stop_market_data",
        MarketDataPersistenceFailureAction::StopTrading => "stop_trading",
        MarketDataPersistenceFailureAction::FailProcess => "fail_process",
        MarketDataPersistenceFailureAction::MemoryOnly => "memory_only",
        _ => "unknown",
    }
}

fn optional_wal_sequence_json(sequence: Option<of_persist::MarketDataWalSequence>) -> String {
    sequence
        .map(|sequence| sequence.0.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn format_adapter_descriptor_json(
    descriptor: &AdapterDescriptor,
    active_provider: Option<&ProviderKind>,
) -> String {
    let active = active_provider
        .map(|provider| provider == &descriptor.provider)
        .unwrap_or(false);
    format!(
        "{{\"provider\":\"{}\",\"provider_id\":\"{}\",\"display_name\":\"{}\",\"feature\":{},\"compiled\":{},\"quality\":\"{}\",\"supports_live\":{},\"supports_replay\":{},\"supports_trades\":{},\"supports_order_book\":{},\"supports_level2\":{},\"supports_reconnect\":{},\"supports_gap_recovery\":{},\"supports_backpressure\":{},\"supports_raw_capture\":{},\"supports_fixture_replay\":{},\"supports_stale_detection\":{},\"supports_latency_metrics\":{},\"supports_polling\":{},\"active\":{},\"notes\":\"{}\"}}",
        escape_json(descriptor.provider.id()),
        escape_json(descriptor.provider_id),
        escape_json(descriptor.display_name),
        optional_str_json(descriptor.feature),
        descriptor.compiled,
        escape_json(descriptor.quality.id()),
        descriptor.supports_live,
        descriptor.supports_replay,
        descriptor.supports_trades,
        descriptor.supports_order_book,
        descriptor.supports_level2,
        descriptor.supports_reconnect,
        descriptor.supports_gap_recovery,
        descriptor.supports_backpressure,
        descriptor.supports_raw_capture,
        descriptor.supports_fixture_replay,
        descriptor.supports_stale_detection,
        descriptor.supports_latency_metrics,
        descriptor.supports_polling,
        active,
        escape_json(descriptor.notes)
    )
}

fn format_adapter_inventory_json(active_provider: Option<&ProviderKind>) -> String {
    let items = adapter_descriptors()
        .iter()
        .map(|descriptor| format_adapter_descriptor_json(descriptor, active_provider))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema_version\":1,\"adapters\":[{}],\"compiled_count\":{},\"total_count\":{}}}",
        items,
        adapter_descriptors()
            .iter()
            .filter(|descriptor| descriptor.compiled)
            .count(),
        adapter_descriptors().len()
    )
}

fn format_adapter_status_json(status: &RuntimeAdapterStatus) -> String {
    let last_error_json = status
        .health
        .last_error
        .as_ref()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .unwrap_or_else(|| "null".to_string());
    let protocol_info_json = status
        .health
        .protocol_info
        .as_ref()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .unwrap_or_else(|| "null".to_string());
    let healthy = status.started
        && status.health.connected
        && !status.health.degraded
        && !status.circuit_breaker_open;
    let subscribed_symbols_json = status
        .operational
        .subscribed_symbols
        .iter()
        .map(|symbol| {
            format!(
                "{{\"venue\":\"{}\",\"symbol\":\"{}\"}}",
                escape_json(&symbol.venue),
                escape_json(&symbol.symbol)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema_version\":1,\"provider\":\"{}\",\"provider_id\":\"{}\",\"display_name\":\"{}\",\"feature\":{},\"compiled\":{},\"quality\":\"{}\",\"started\":{},\"connected\":{},\"degraded\":{},\"healthy\":{},\"last_error\":{},\"protocol_info\":{},\"health_seq\":{},\"circuit_breaker_open\":{},\"mode\":\"{}\",\"connection_state\":\"{}\",\"endpoint_redacted\":{},\"app_name\":{},\"reconnect_attempt\":{},\"subscription_count\":{},\"subscribed_symbols\":[{}],\"queue_depth\":{},\"queue_capacity\":{},\"dropped_events\":{},\"gap_count\":{},\"stale\":{},\"raw_capture_enabled\":{},\"raw_capture_depth\":{},\"raw_capture_capacity\":{},\"last_message_age_ms\":{},\"last_market_data_age_ms\":{},\"capabilities\":{}}}",
        escape_json(status.descriptor.provider.id()),
        escape_json(status.descriptor.provider_id),
        escape_json(status.descriptor.display_name),
        optional_str_json(status.descriptor.feature),
        status.descriptor.compiled,
        escape_json(status.descriptor.quality.id()),
        status.started,
        status.health.connected,
        status.health.degraded,
        healthy,
        last_error_json,
        protocol_info_json,
        status.health_seq,
        status.circuit_breaker_open,
        status.operational.mode.id(),
        status.operational.connection_state.id(),
        optional_str_json(status.operational.endpoint_redacted.as_deref()),
        optional_str_json(status.operational.app_name.as_deref()),
        status.operational.reconnect_attempt,
        status.operational.subscription_count,
        subscribed_symbols_json,
        status.operational.queue_depth,
        optional_usize_json(status.operational.queue_capacity),
        status.operational.dropped_events,
        status.operational.gap_count,
        status.operational.stale,
        status.operational.raw_capture_enabled,
        status.operational.raw_capture_depth,
        status.operational.raw_capture_capacity,
        optional_u64_json(status.operational.last_message_age_ms),
        optional_u64_json(status.operational.last_market_data_age_ms),
        format_adapter_descriptor_json(&status.descriptor, Some(&status.descriptor.provider))
    )
}

fn quality_flag_names(bits: u32) -> Vec<&'static str> {
    let mut names = Vec::new();
    if bits & DataQualityFlags::STALE_FEED.bits() != 0 {
        names.push("STALE_FEED");
    }
    if bits & DataQualityFlags::SEQUENCE_GAP.bits() != 0 {
        names.push("SEQUENCE_GAP");
    }
    if bits & DataQualityFlags::CLOCK_SKEW.bits() != 0 {
        names.push("CLOCK_SKEW");
    }
    if bits & DataQualityFlags::DEPTH_TRUNCATED.bits() != 0 {
        names.push("DEPTH_TRUNCATED");
    }
    if bits & DataQualityFlags::OUT_OF_ORDER.bits() != 0 {
        names.push("OUT_OF_ORDER");
    }
    if bits & DataQualityFlags::ADAPTER_DEGRADED.bits() != 0 {
        names.push("ADAPTER_DEGRADED");
    }
    names
}

fn quality_flags_detail_json(bits: u32) -> String {
    let names = quality_flag_names(bits);
    if names.is_empty() {
        return "[]".to_string();
    }

    let items = names
        .into_iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn sanitize_analytics_config(mut config: AnalyticsConfig) -> AnalyticsConfig {
    config.vpin_volume_bucket = config.vpin_volume_bucket.max(1);
    config.vpin_max_buckets = config.vpin_max_buckets.min(MAX_ANALYTICS_WINDOW_LEN);
    config.kyle_lambda_max_len = config.kyle_lambda_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.cvd_max_len = config.cvd_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.vol_estimator_max_len = config.vol_estimator_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.noise_max_len = config.noise_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.hasbrouck_max_len = config.hasbrouck_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.almgren_chriss_max_len = config.almgren_chriss_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.acd_max_len = config.acd_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.vol_signature_max_len = config.vol_signature_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.agent_max_len = config.agent_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.institutional_max_len = config.institutional_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.resiliency_max_len = config.resiliency_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.spread_decomp_max_len = config.spread_decomp_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.regime_max_len = config.regime_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.event_tracker_max_len = config.event_tracker_max_len.min(MAX_EVENT_TRACKER_LEN);
    config.spread_tracker_max_len = config.spread_tracker_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.default_max_len = config.default_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config
}

/// Builds the default runtime engine using configured provider and signal module.
pub fn build_default_engine(cfg: EngineConfig) -> Result<DefaultEngine, RuntimeError> {
    validate_startup_config(&cfg)?;
    let signal_threshold = cfg.signal_threshold;
    let persistence = if cfg.enable_persistence {
        Some(
            RollingStore::new(&cfg.data_root)
                .map_err(|e| RuntimeError::Io(format!("{e:?}")))?
                .with_retention(Some(RetentionPolicy {
                    max_total_bytes: cfg.data_retention_max_bytes,
                    max_age_secs: cfg.data_retention_max_age_secs,
                })),
        )
    } else {
        None
    };

    let audit = Some(AuditLog::new(
        &cfg.audit_log_path,
        cfg.audit_max_bytes,
        cfg.audit_max_files,
        cfg.audit_redact_tokens.clone(),
    )?);

    let adapter = create_adapter(&cfg.adapter).map_err(|e| RuntimeError::Adapter(e.to_string()))?;
    Ok(Engine::new(
        cfg,
        adapter,
        of_signals::DeltaMomentumSignal::new(signal_threshold),
    )
    .with_persistence(persistence)
    .with_audit(audit))
}
