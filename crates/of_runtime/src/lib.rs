#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, create_dir_all, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};

use of_adapters::{
    create_adapter, AdapterConfig, CredentialsRef, MarketDataAdapter, ProviderKind, RawEvent,
    SubscribeReq,
};
use of_core::{
    AnalyticsAccumulator, AnalyticsSnapshot, BookUpdate, DataQualityFlags, SignalSnapshot,
    SignalState, SymbolId, TradePrint,
};
use of_persist::{RetentionPolicy, RollingStore};
use of_signals::{SignalGateDecision, SignalModule};

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

/// Policy controlling quality constraints for externally-ingested feeds.
#[derive(Debug, Clone)]
pub struct ExternalFeedPolicy {
    /// Max allowed ingest silence before marking feed stale.
    pub stale_after_ms: u64,
    /// Enables sequence-gap/out-of-order checks.
    pub enforce_sequence: bool,
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

fn rotated_path(base: &Path, idx: u32) -> PathBuf {
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

fn combine_quality_flags(lhs: DataQualityFlags, rhs: DataQualityFlags) -> DataQualityFlags {
    DataQualityFlags::from_bits_truncate(lhs.bits() | rhs.bits())
}

/// Runtime engine over a market-data adapter and signal module.
pub struct Engine<A: MarketDataAdapter, S: SignalModule> {
    cfg: EngineConfig,
    adapter: A,
    signal_module: S,
    started: bool,
    analytics: HashMap<SymbolId, AnalyticsAccumulator>,
    latest_signals: HashMap<SymbolId, SignalSnapshot>,
    processed_events: u64,
    persistence: Option<RollingStore>,
    audit: Option<AuditLog>,
    health_seq: u64,
    last_health_fingerprint: String,
    last_quality_flags_bits: u32,
    last_events: Vec<RawEvent>,
    external: ExternalFeedState,
}

/// Default engine type used by C ABI and high-level bindings.
pub type DefaultEngine = Engine<Box<dyn MarketDataAdapter>, of_signals::DeltaMomentumSignal>;

impl<A: MarketDataAdapter, S: SignalModule> Engine<A, S> {
    /// Creates an engine with explicit adapter and signal module.
    pub fn new(cfg: EngineConfig, adapter: A, signal_module: S) -> Self {
        Self {
            cfg,
            adapter,
            signal_module,
            started: false,
            analytics: HashMap::new(),
            latest_signals: HashMap::new(),
            processed_events: 0,
            persistence: None,
            audit: None,
            health_seq: 0,
            last_health_fingerprint: String::new(),
            last_quality_flags_bits: 0,
            last_events: Vec::new(),
            external: ExternalFeedState::default(),
        }
    }

    /// Injects optional persistence backend.
    pub fn with_persistence(mut self, persistence: Option<RollingStore>) -> Self {
        self.persistence = persistence;
        self
    }

    fn with_audit(mut self, audit: Option<AuditLog>) -> Self {
        self.audit = audit;
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
        self.update_health_state(DataQualityFlags::NONE);
        let _ = self.audit_event("engine_stopped", "{}");
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
            self.latest_signals.insert(symbol.clone(), self.signal_module.snapshot());
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
    pub fn configure_external_feed(&mut self, policy: ExternalFeedPolicy) -> Result<(), RuntimeError> {
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
        let mut effective_quality = combine_quality_flags(quality_flags, self.external_quality_flags());
        let seq_flags = self.external_sequence_flags(&trade.symbol, trade.sequence, true);
        effective_quality = combine_quality_flags(effective_quality, seq_flags);
        self.external.last_ingest_ns = Some(unix_ts_nanos());
        let event = RawEvent::Trade(trade);
        self.last_events = vec![event.clone()];
        self.process_event(event, effective_quality)?;
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
        let mut effective_quality = combine_quality_flags(quality_flags, self.external_quality_flags());
        let seq_flags = self.external_sequence_flags(&book.symbol, book.sequence, false);
        effective_quality = combine_quality_flags(effective_quality, seq_flags);
        self.external.last_ingest_ns = Some(unix_ts_nanos());
        let event = RawEvent::Book(book);
        self.last_events = vec![event.clone()];
        self.process_event(event, effective_quality)?;
        self.update_health_state(effective_quality);
        Ok(())
    }

    /// Polls adapter once and processes all returned events.
    pub fn poll_once(&mut self, quality_flags: DataQualityFlags) -> Result<usize, RuntimeError> {
        if !self.started {
            return Err(RuntimeError::NotStarted);
        }

        let mut events = Vec::new();
        self.adapter
            .poll(&mut events)
            .map_err(|e| RuntimeError::Adapter(e.to_string()))?;
        self.last_events = events.clone();

        for event in events {
            self.process_event(event, quality_flags)?;
        }
        self.update_health_state(quality_flags);

        Ok(self.last_events.len())
    }

    /// Returns analytics snapshot for symbol if available.
    pub fn analytics_snapshot(&self, symbol: &SymbolId) -> Option<AnalyticsSnapshot> {
        self.analytics.get(symbol).map(AnalyticsAccumulator::snapshot)
    }

    /// Returns latest signal snapshot for symbol if available.
    pub fn signal_snapshot(&self, symbol: &SymbolId) -> Option<SignalSnapshot> {
        self.latest_signals.get(symbol).cloned()
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

        format!(
            "{{\"instance_id\":\"{}\",\"started\":{},\"processed_events\":{},\"symbols\":{},\"persistence\":{},\"adapter_connected\":{},\"adapter_degraded\":{},\"adapter_last_error\":{},\"adapter_protocol_info\":{},\"external_feed_enabled\":{},\"external_feed_reconnecting\":{},\"external_stale_after_ms\":{}}}",
            escape_json(&self.cfg.instance_id),
            self.started,
            self.processed_events,
            self.analytics.len(),
            self.persistence.is_some(),
            adapter_health.connected,
            adapter_health.degraded,
            last_error_json,
            protocol_info_json,
            self.external.enabled,
            self.external.reconnecting,
            self.external.policy.stale_after_ms
        )
    }

    /// Returns monotonic health sequence number.
    pub fn health_seq(&self) -> u64 {
        self.health_seq
    }

    /// Returns health snapshot as compact JSON payload.
    pub fn health_json(&self) -> String {
        let adapter_health = self.adapter.health();
        let reconnect_state = if !adapter_health.connected {
            "disconnected"
        } else if adapter_health.degraded || self.external.reconnecting {
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
        format!(
            "{{\"health_seq\":{},\"started\":{},\"connected\":{},\"degraded\":{},\"reconnect_state\":\"{}\",\"quality_flags\":{},\"last_error\":{},\"protocol_info\":{}}}",
            self.health_seq,
            self.started,
            adapter_health.connected,
            adapter_health.degraded,
            reconnect_state,
            self.last_quality_flags_bits,
            last_error_json,
            protocol_info_json
        )
    }

    /// Returns events processed in the last poll/ingest cycle.
    pub fn last_events(&self) -> &[RawEvent] {
        &self.last_events
    }

    /// Returns currently-active quality flags as raw bits.
    pub fn current_quality_flags_bits(&self) -> u32 {
        self.last_quality_flags_bits
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
                let stale_after_ns = self.external.policy.stale_after_ms.saturating_mul(1_000_000);
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
    ) -> Result<(), RuntimeError> {
        match event {
            RawEvent::Book(book) => {
                if let Some(store) = &self.persistence {
                    let _ = store.append_book(&book);
                }
                self.processed_events += 1;
            }
            RawEvent::Trade(trade) => {
                if let Some(store) = &self.persistence {
                    let _ = store.append_trade(&trade);
                }

                let symbol = trade.symbol.clone();
                let acc = self.analytics.entry(symbol.clone()).or_default();
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
                self.latest_signals.insert(symbol, signal);
                self.processed_events += 1;
            }
        }

        Ok(())
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
        let fingerprint = format!(
            "{}|{}|{}|{}|{}|{}",
            self.started,
            adapter_health.connected,
            adapter_health.degraded,
            self.last_quality_flags_bits,
            adapter_health.last_error.as_deref().unwrap_or(""),
            adapter_health.protocol_info.as_deref().unwrap_or("")
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
    Ok(
        Engine::new(cfg, adapter, of_signals::DeltaMomentumSignal::new(signal_threshold))
            .with_persistence(persistence)
            .with_audit(audit),
    )
}

/// Loads engine config from `.toml` or `.json`-like config file.
pub fn load_engine_config_from_path(path: &str) -> Result<EngineConfig, RuntimeError> {
    let raw = fs::read_to_string(path).map_err(|e| RuntimeError::Io(e.to_string()))?;
    let mut kv = HashMap::new();

    if path.ends_with(".json") {
        parse_json_like(&raw, &mut kv)?;
    } else if path.ends_with(".toml") {
        parse_toml_like(&raw, &mut kv)?;
    } else {
        return Err(RuntimeError::Config(
            "unsupported config format; use .json or .toml".to_string(),
        ));
    }

    config_from_map(&kv)
}

/// Validates startup configuration and environment prerequisites.
pub fn validate_startup_config(cfg: &EngineConfig) -> Result<(), RuntimeError> {
    if cfg.instance_id.trim().is_empty() {
        return Err(RuntimeError::Config("instance_id must not be empty".to_string()));
    }

    if cfg.signal_threshold <= 0 {
        return Err(RuntimeError::Config(
            "signal_threshold must be > 0".to_string(),
        ));
    }

    if cfg.audit_log_path.trim().is_empty() {
        return Err(RuntimeError::Config(
            "audit_log_path must not be empty".to_string(),
        ));
    }
    if cfg.audit_max_bytes == 0 {
        return Err(RuntimeError::Config(
            "audit_max_bytes must be > 0".to_string(),
        ));
    }
    if cfg.audit_max_files > 1000 {
        return Err(RuntimeError::Config(
            "audit_max_files must be <= 1000".to_string(),
        ));
    }

    if cfg.enable_persistence && cfg.data_root.trim().is_empty() {
        return Err(RuntimeError::Config(
            "data_root must not be empty when persistence is enabled".to_string(),
        ));
    }
    if cfg.enable_persistence && cfg.data_retention_max_bytes == 0 && cfg.data_retention_max_age_secs == 0 {
        return Err(RuntimeError::Config(
            "set at least one of data_retention_max_bytes or data_retention_max_age_secs when persistence is enabled".to_string(),
        ));
    }

    match cfg.adapter.provider {
        ProviderKind::Mock => Ok(()),
        ProviderKind::Rithmic | ProviderKind::Cqg | ProviderKind::Binance => {
            if cfg
                .adapter
                .endpoint
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err(RuntimeError::Config(
                    "non-mock providers require adapter.endpoint".to_string(),
                ));
            }

            if matches!(cfg.adapter.provider, ProviderKind::Rithmic | ProviderKind::Cqg) {
                let creds = cfg.adapter.credentials.as_ref().ok_or_else(|| {
                    RuntimeError::Config(
                        "rithmic/cqg providers require adapter.credentials references".to_string(),
                    )
                })?;

                validate_env_var(&creds.key_id_env)?;
                validate_env_var(&creds.secret_env)?;
            }
            Ok(())
        }
    }
}

fn validate_env_var(name: &str) -> Result<(), RuntimeError> {
    let value = std::env::var(name)
        .map_err(|_| RuntimeError::Config(format!("missing required env var: {name}")))?;
    if value.trim().is_empty() {
        return Err(RuntimeError::Config(format!(
            "required env var is empty: {name}"
        )));
    }
    Ok(())
}

fn config_hash(cfg: &EngineConfig) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    cfg.instance_id.hash(&mut hasher);
    cfg.enable_persistence.hash(&mut hasher);
    cfg.data_root.hash(&mut hasher);
    cfg.audit_log_path.hash(&mut hasher);
    cfg.audit_max_bytes.hash(&mut hasher);
    cfg.audit_max_files.hash(&mut hasher);
    cfg.data_retention_max_bytes.hash(&mut hasher);
    cfg.data_retention_max_age_secs.hash(&mut hasher);
    cfg.signal_threshold.hash(&mut hasher);
    let provider = match cfg.adapter.provider {
        ProviderKind::Mock => 0u8,
        ProviderKind::Rithmic => 1u8,
        ProviderKind::Cqg => 2u8,
        ProviderKind::Binance => 3u8,
    };
    provider.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn config_from_map(map: &HashMap<String, String>) -> Result<EngineConfig, RuntimeError> {
    let mut cfg = EngineConfig::default();

    if let Some(v) = map.get("instance_id") {
        cfg.instance_id = v.to_string();
    }

    if let Some(v) = map.get("enable_persistence") {
        cfg.enable_persistence = parse_bool(v, "enable_persistence")?;
    }

    if let Some(v) = map.get("signal_threshold") {
        cfg.signal_threshold = parse_i64(v, "signal_threshold")?;
    }

    if let Some(v) = map.get("data_root") {
        cfg.data_root = v.to_string();
    }
    if let Some(v) = map.get("audit_log_path") {
        cfg.audit_log_path = v.to_string();
    }
    if let Some(v) = map.get("audit_max_bytes") {
        cfg.audit_max_bytes = parse_u64(v, "audit_max_bytes")?;
    }
    if let Some(v) = map.get("audit_max_files") {
        cfg.audit_max_files = parse_u32(v, "audit_max_files")?;
    }
    if let Some(v) = map.get("audit_redact_tokens") {
        cfg.audit_redact_tokens = parse_csv(v);
    }
    if let Some(v) = map.get("data_retention_max_bytes") {
        cfg.data_retention_max_bytes = parse_u64(v, "data_retention_max_bytes")?;
    }
    if let Some(v) = map.get("data_retention_max_age_secs") {
        cfg.data_retention_max_age_secs = parse_u64(v, "data_retention_max_age_secs")?;
    }

    if let Some(v) = map.get("adapter.provider").or_else(|| map.get("provider")) {
        cfg.adapter.provider = parse_provider(v)?;
    }

    if let Some(v) = map.get("adapter.endpoint").or_else(|| map.get("endpoint")) {
        cfg.adapter.endpoint = Some(v.to_string());
    }

    if let Some(v) = map.get("adapter.app_name").or_else(|| map.get("app_name")) {
        cfg.adapter.app_name = Some(v.to_string());
    }

    let key_ref = map
        .get("adapter.credentials.key_id_env")
        .or_else(|| map.get("credentials.key_id_env"))
        .or_else(|| map.get("credentials_key_id_env"));
    let secret_ref = map
        .get("adapter.credentials.secret_env")
        .or_else(|| map.get("credentials.secret_env"))
        .or_else(|| map.get("credentials_secret_env"));

    match (key_ref, secret_ref) {
        (Some(k), Some(s)) => {
            cfg.adapter.credentials = Some(CredentialsRef {
                key_id_env: k.to_string(),
                secret_env: s.to_string(),
            });
        }
        (None, None) => {}
        _ => {
            return Err(RuntimeError::Config(
                "credentials require both key_id_env and secret_env".to_string(),
            ));
        }
    }

    Ok(cfg)
}

fn parse_provider(v: &str) -> Result<ProviderKind, RuntimeError> {
    match v.trim().to_ascii_lowercase().as_str() {
        "mock" => Ok(ProviderKind::Mock),
        "rithmic" => Ok(ProviderKind::Rithmic),
        "cqg" => Ok(ProviderKind::Cqg),
        "binance" | "binance_spot" | "crypto_binance" => Ok(ProviderKind::Binance),
        _ => Err(RuntimeError::Config(format!("unknown provider: {v}"))),
    }
}

fn parse_bool(v: &str, key: &str) -> Result<bool, RuntimeError> {
    match v.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(RuntimeError::Config(format!("invalid bool for {key}: {v}"))),
    }
}

fn parse_i64(v: &str, key: &str) -> Result<i64, RuntimeError> {
    v.trim()
        .parse::<i64>()
        .map_err(|_| RuntimeError::Config(format!("invalid i64 for {key}: {v}")))
}

fn parse_u64(v: &str, key: &str) -> Result<u64, RuntimeError> {
    v.trim()
        .parse::<u64>()
        .map_err(|_| RuntimeError::Config(format!("invalid u64 for {key}: {v}")))
}

fn parse_u32(v: &str, key: &str) -> Result<u32, RuntimeError> {
    v.trim()
        .parse::<u32>()
        .map_err(|_| RuntimeError::Config(format!("invalid u32 for {key}: {v}")))
}

fn parse_csv(v: &str) -> Vec<String> {
    v.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn parse_json_like(raw: &str, out: &mut HashMap<String, String>) -> Result<(), RuntimeError> {
    for line in raw.lines() {
        let mut s = line.trim();
        if s.is_empty() || s == "{" || s == "}" {
            continue;
        }
        if s.ends_with(',') {
            s = &s[..s.len() - 1];
        }
        if s.ends_with('{') {
            continue;
        }

        let (k, v) = match s.split_once(':') {
            Some(parts) => parts,
            None => continue,
        };

        let key = trim_quotes(k.trim());
        let value = trim_quotes(v.trim());
        if !key.is_empty() {
            out.insert(key.to_string(), value.to_string());
        }
    }

    Ok(())
}

fn parse_toml_like(raw: &str, out: &mut HashMap<String, String>) -> Result<(), RuntimeError> {
    let mut section = String::new();

    for line in raw.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }

        let (k, v) = line
            .split_once('=')
            .ok_or_else(|| RuntimeError::Config("invalid toml line".to_string()))?;
        let key = k.trim();
        let value = trim_quotes(v.trim());
        let full_key = if section.is_empty() {
            key.to_string()
        } else {
            format!("{section}.{key}")
        };
        out.insert(full_key, value.to_string());
    }

    Ok(())
}

fn trim_quotes(v: &str) -> &str {
    let t = v.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        &t[1..t.len() - 1]
    } else {
        t
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use of_adapters::{MockAdapter, ProviderKind, RawEvent};
    use of_core::{BookAction, BookUpdate, Side, TradePrint};
    use of_signals::DeltaMomentumSignal;

    use super::*;

    #[test]
    fn engine_processes_trade_and_updates_snapshots() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        let mut adapter = MockAdapter::default();
        adapter.push_event(RawEvent::Trade(TradePrint {
            symbol: symbol.clone(),
            price: 505000,
            size: 10,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }));

        let mut engine = Engine::new(
            EngineConfig::default(),
            adapter,
            DeltaMomentumSignal::new(5),
        );

        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine.poll_once(DataQualityFlags::NONE).expect("poll failed");

        let analytics = engine.analytics_snapshot(&symbol).expect("analytics missing");
        assert_eq!(analytics.delta, 10);

        let signal = engine.signal_snapshot(&symbol).expect("signal missing");
        assert_eq!(signal.state, SignalState::LongBias);
    }

    #[test]
    fn engine_ingests_external_events_and_updates_snapshots() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );

        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine
            .ingest_book(
                BookUpdate {
                    symbol: symbol.clone(),
                    side: Side::Bid,
                    level: 0,
                    price: 504900,
                    size: 20,
                    action: BookAction::Upsert,
                    sequence: 1,
                    ts_exchange_ns: 10,
                    ts_recv_ns: 11,
                },
                DataQualityFlags::NONE,
            )
            .expect("book ingest failed");
        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505000,
                    size: 7,
                    aggressor_side: Side::Ask,
                    sequence: 2,
                    ts_exchange_ns: 12,
                    ts_recv_ns: 13,
                },
                DataQualityFlags::ADAPTER_DEGRADED,
            )
            .expect("trade ingest failed");

        let analytics = engine.analytics_snapshot(&symbol).expect("analytics missing");
        assert_eq!(analytics.delta, 7);
        let signal = engine.signal_snapshot(&symbol).expect("signal missing");
        assert_eq!(signal.state, SignalState::Blocked);
        assert_eq!(signal.quality_flags, DataQualityFlags::ADAPTER_DEGRADED.bits());
        assert_eq!(signal.reason, "blocked_by_quality_gate");
        assert_eq!(engine.last_events().len(), 1);
    }

    #[test]
    fn external_supervisor_sets_sequence_and_order_flags() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );
        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine
            .configure_external_feed(ExternalFeedPolicy {
                stale_after_ms: 0,
                enforce_sequence: true,
            })
            .expect("configure external feed");

        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505000,
                    size: 1,
                    aggressor_side: Side::Ask,
                    sequence: 1,
                    ts_exchange_ns: 1,
                    ts_recv_ns: 1,
                },
                DataQualityFlags::NONE,
            )
            .expect("ingest seq1");
        let s1 = engine.signal_snapshot(&symbol).expect("signal 1");
        assert_eq!(s1.quality_flags, 0);

        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505001,
                    size: 1,
                    aggressor_side: Side::Ask,
                    sequence: 3,
                    ts_exchange_ns: 2,
                    ts_recv_ns: 2,
                },
                DataQualityFlags::NONE,
            )
            .expect("ingest seq3");
        let s2 = engine.signal_snapshot(&symbol).expect("signal 2");
        assert!(s2.quality_flags & DataQualityFlags::SEQUENCE_GAP.bits() != 0);

        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505002,
                    size: 1,
                    aggressor_side: Side::Ask,
                    sequence: 2,
                    ts_exchange_ns: 3,
                    ts_recv_ns: 3,
                },
                DataQualityFlags::NONE,
            )
            .expect("ingest seq2");
        let s3 = engine.signal_snapshot(&symbol).expect("signal 3");
        assert!(s3.quality_flags & DataQualityFlags::OUT_OF_ORDER.bits() != 0);
    }

    #[test]
    fn external_supervisor_reconnecting_and_stale_flags_affect_health() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut engine = Engine::new(
            EngineConfig::default(),
            MockAdapter::default(),
            DeltaMomentumSignal::new(5),
        );
        engine.start().expect("start failed");
        engine.subscribe(symbol.clone(), 10).expect("sub failed");
        engine
            .configure_external_feed(ExternalFeedPolicy {
                stale_after_ms: 1,
                enforce_sequence: true,
            })
            .expect("configure external feed");

        engine
            .set_external_reconnecting(true)
            .expect("set reconnecting true");
        let degraded = engine.health_json();
        assert!(degraded.contains(&format!(
            "\"quality_flags\":{}",
            DataQualityFlags::ADAPTER_DEGRADED.bits()
        )));

        engine
            .set_external_reconnecting(false)
            .expect("set reconnecting false");
        engine
            .ingest_trade(
                TradePrint {
                    symbol: symbol.clone(),
                    price: 505000,
                    size: 1,
                    aggressor_side: Side::Ask,
                    sequence: 1,
                    ts_exchange_ns: 1,
                    ts_recv_ns: 1,
                },
                DataQualityFlags::NONE,
            )
            .expect("ingest");
        std::thread::sleep(std::time::Duration::from_millis(3));
        engine.external_health_tick().expect("health tick");
        let stale = engine.health_json();
        assert!(stale.contains(&format!(
            "\"quality_flags\":{}",
            DataQualityFlags::STALE_FEED.bits()
        )));
    }

    #[test]
    fn default_builder_wires_mock_provider() {
        let cfg = EngineConfig {
            adapter: AdapterConfig {
                provider: ProviderKind::Mock,
                ..AdapterConfig::default()
            },
            ..EngineConfig::default()
        };
        let mut engine = build_default_engine(cfg).expect("build should work");
        engine.start().expect("start should work");
        let metrics = engine.metrics_json();
        assert!(metrics.contains("\"started\":true"));
        assert!(metrics.contains("\"adapter_protocol_info\""));
    }

    #[test]
    fn parses_toml_file_config() {
        let path = write_temp_file(
            "runtime_cfg.toml",
            r#"
instance_id = "from_toml"
enable_persistence = true
signal_threshold = 250
provider = "mock"
data_root = "data_local"
audit_log_path = "audit/local.log"
"#,
        );

        let cfg = load_engine_config_from_path(path.to_str().expect("valid path"))
            .expect("toml parse should work");
        assert_eq!(cfg.instance_id, "from_toml");
        assert!(cfg.enable_persistence);
        assert_eq!(cfg.signal_threshold, 250);
        assert_eq!(cfg.data_root, "data_local");
        assert_eq!(cfg.audit_log_path, "audit/local.log");
        assert!(matches!(cfg.adapter.provider, ProviderKind::Mock));
    }

    #[test]
    fn validates_non_mock_requires_env_refs() {
        let cfg = EngineConfig {
            adapter: AdapterConfig {
                provider: ProviderKind::Cqg,
                endpoint: Some("cqg://example".to_string()),
                credentials: Some(CredentialsRef {
                    key_id_env: "OF_TEST_MISSING_KEY".to_string(),
                    secret_env: "OF_TEST_MISSING_SECRET".to_string(),
                }),
                ..AdapterConfig::default()
            },
            ..EngineConfig::default()
        };

        let err = validate_startup_config(&cfg).expect_err("missing env vars should fail");
        assert!(format!("{err}").contains("missing required env var"));
    }

    #[test]
    fn parses_binance_provider_without_credentials() {
        let path = write_temp_file(
            "runtime_cfg_binance.toml",
            r#"
instance_id = "from_toml_binance"
provider = "binance"
endpoint = "mock://binance"
"#,
        );
        let cfg = load_engine_config_from_path(path.to_str().expect("valid path"))
            .expect("toml parse should work");
        assert!(matches!(cfg.adapter.provider, ProviderKind::Binance));
        validate_startup_config(&cfg).expect("binance should not require creds");
    }

    #[test]
    fn audit_log_rotates_and_redacts() {
        let base = temp_dir("audit_rotate");
        let audit_path = base.join("audit.log");
        let data_root = base.join("data");

        let mut engine = build_default_engine(EngineConfig {
            instance_id: "audit-test".to_string(),
            enable_persistence: false,
            data_root: data_root.to_string_lossy().to_string(),
            audit_log_path: audit_path.to_string_lossy().to_string(),
            audit_max_bytes: 180,
            audit_max_files: 2,
            audit_redact_tokens: vec!["token".to_string()],
            data_retention_max_bytes: 1024,
            data_retention_max_age_secs: 60,
            adapter: AdapterConfig::default(),
            signal_threshold: 100,
        })
        .expect("engine build should work");

        engine.start().expect("start should work");
        for i in 0..6 {
            engine
                .subscribe(
                    SymbolId {
                        venue: "CME".to_string(),
                        symbol: format!("ES_token_{i}"),
                    },
                    10,
                )
                .expect("subscribe should work");
        }
        engine.stop();

        let current = fs::read_to_string(&audit_path).expect("current audit must exist");
        assert!(current.contains("[REDACTED]"));
        assert!(rotated_path(&audit_path, 1).exists());
    }

    #[test]
    fn reset_symbol_session_clears_analytics() {
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut adapter = MockAdapter::default();
        adapter.push_event(RawEvent::Trade(TradePrint {
            symbol: symbol.clone(),
            price: 505000,
            size: 10,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }));

        let mut engine = Engine::new(
            EngineConfig::default(),
            adapter,
            DeltaMomentumSignal::new(5),
        );
        engine.start().expect("start");
        engine.subscribe(symbol.clone(), 10).expect("subscribe");
        engine.poll_once(DataQualityFlags::NONE).expect("poll");
        let pre = engine.analytics_snapshot(&symbol).expect("pre");
        assert!(pre.cumulative_delta > 0);

        engine
            .reset_symbol_session(symbol.clone())
            .expect("reset session");
        let post = engine.analytics_snapshot(&symbol).expect("post");
        assert_eq!(post.delta, 0);
        assert_eq!(post.cumulative_delta, 0);
        assert_eq!(post.point_of_control, 0);
    }

    fn write_temp_file(name: &str, content: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nonce = format!(
            "{}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock ok")
                .as_nanos(),
            name
        );
        path.push(nonce);
        fs::write(&path, content).expect("temp file write should work");
        path
    }

    fn temp_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{}_{}_{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock ok")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("temp dir create should work");
        path
    }
}
