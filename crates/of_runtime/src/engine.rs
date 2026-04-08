use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::fs::{self, create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use of_adapters::{create_adapter, AdapterConfig, MarketDataAdapter, RawEvent, SubscribeReq};
use of_core::{
    AnalyticsAccumulator, AnalyticsSnapshot, BookLevel, BookSnapshot, BookUpdate,
    DataQualityFlags, DerivedAnalyticsSnapshot, IntervalCandleSnapshot, SessionCandleSnapshot,
    SignalSnapshot, SignalState, SymbolId, TradePrint,
};
use of_persist::{RetentionPolicy, RollingStore};
use of_signals::{SignalGateDecision, SignalModule};

use crate::config::config_hash;
use crate::validate_startup_config;

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
            books: HashMap::new(),
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

    /// Returns the current materialized book snapshot for symbol if available.
    pub fn book_snapshot(&self, symbol: &SymbolId) -> Option<BookSnapshot> {
        self.books.get(symbol).map(|book| book.snapshot(symbol))
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
        let quality_flags_detail = quality_flags_detail_json(self.last_quality_flags_bits);
        let external_last_ingest = optional_u64_json(self.external.last_ingest_ns);

        format!(
            "{{\"instance_id\":\"{}\",\"started\":{},\"processed_events\":{},\"symbols\":{},\"book_symbols\":{},\"analytics_symbols\":{},\"signal_symbols\":{},\"persistence\":{},\"health_seq\":{},\"quality_flags\":{},\"quality_flags_detail\":{},\"adapter_connected\":{},\"adapter_degraded\":{},\"adapter_last_error\":{},\"adapter_protocol_info\":{},\"external_feed_enabled\":{},\"external_feed_reconnecting\":{},\"external_sequence_enforced\":{},\"external_stale_after_ms\":{},\"external_last_ingest_ns\":{},\"external_trade_sequence_symbols\":{},\"external_book_sequence_symbols\":{}}}",
            escape_json(&self.cfg.instance_id),
            self.started,
            self.processed_events,
            self.tracked_symbol_count(),
            self.books.len(),
            self.analytics.len(),
            self.latest_signals.len(),
            self.persistence.is_some(),
            self.health_seq,
            self.last_quality_flags_bits,
            quality_flags_detail,
            adapter_health.connected,
            adapter_health.degraded,
            last_error_json,
            protocol_info_json,
            self.external.enabled,
            self.external.reconnecting,
            self.external.policy.enforce_sequence,
            self.external.policy.stale_after_ms,
            external_last_ingest,
            self.external.trade_seq.len(),
            self.external.book_seq.len()
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
        let quality_flags_detail = quality_flags_detail_json(self.last_quality_flags_bits);
        let external_last_ingest = optional_u64_json(self.external.last_ingest_ns);
        format!(
            "{{\"health_seq\":{},\"started\":{},\"connected\":{},\"degraded\":{},\"reconnect_state\":\"{}\",\"quality_flags\":{},\"quality_flags_detail\":{},\"last_error\":{},\"protocol_info\":{},\"tracked_symbols\":{},\"processed_events\":{},\"external_feed_enabled\":{},\"external_feed_reconnecting\":{},\"external_sequence_enforced\":{},\"external_last_ingest_ns\":{}}}",
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
            self.external.enabled,
            self.external.reconnecting,
            self.external.policy.enforce_sequence,
            external_last_ingest
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
                self.books
                    .entry(book.symbol.clone())
                    .or_default()
                    .on_book(&book);
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

fn optional_u64_json(value: Option<u64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string())
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
