#![doc = include_str!("../README.md")]

use std::env;
use std::error::Error;
use std::fmt;
use std::path::Path;

use of_core::{BookUpdate, SymbolId, TradePrint};

/// Subscription request forwarded to adapters.
#[derive(Debug, Clone)]
pub struct SubscribeReq {
    /// Symbol to subscribe.
    pub symbol: SymbolId,
    /// Requested book depth levels.
    pub depth_levels: u16,
}

/// Adapter connection and quality health snapshot.
#[derive(Debug, Clone, Default)]
pub struct AdapterHealth {
    /// True when underlying stream is connected.
    pub connected: bool,
    /// True when feed is degraded/reconnecting.
    pub degraded: bool,
    /// Last adapter error if known.
    pub last_error: Option<String>,
    /// Provider/protocol metadata.
    pub protocol_info: Option<String>,
}

/// Transport mode used by an active adapter instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum AdapterRuntimeMode {
    /// Deterministic local or synthetic transport.
    Mock,
    /// Live provider transport.
    Live,
    /// Deterministic persisted-data replay transport.
    Replay,
    /// Externally driven bridge transport.
    Bridge,
    /// Adapter did not report its transport mode.
    #[default]
    Unknown,
}

impl AdapterRuntimeMode {
    /// Returns the stable lowercase identifier used in status payloads.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Live => "live",
            Self::Replay => "replay",
            Self::Bridge => "bridge",
            Self::Unknown => "unknown",
        }
    }
}

/// Provider connection state reported by an active adapter instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum AdapterConnectionState {
    /// No provider transport is connected.
    Disconnected,
    /// Initial connection or provider logon is in progress.
    Connecting,
    /// Provider events can be consumed normally.
    Streaming,
    /// A previously connected transport is reconnecting.
    Reconnecting,
    /// Reconnection is delayed by a bounded backoff policy.
    Backoff,
    /// Persisted events are being replayed.
    Replay,
    /// Adapter did not report a more specific state.
    #[default]
    Unknown,
}

impl AdapterConnectionState {
    /// Returns the stable lowercase identifier used in status payloads.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::Connecting => "connecting",
            Self::Streaming => "streaming",
            Self::Reconnecting => "reconnecting",
            Self::Backoff => "backoff",
            Self::Replay => "replay",
            Self::Unknown => "unknown",
        }
    }
}

/// Typed operational status for a market-data adapter.
///
/// Adapters build this snapshot only when queried; event polling does not pay
/// for symbol sorting, endpoint redaction, or string allocation. Endpoint
/// values contain only a validated URI scheme and authority. User information,
/// paths, queries, and fragments are deliberately omitted because providers
/// can carry credentials or listen keys in any of those components.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct AdapterOperationalStatus {
    /// Active transport mode.
    pub mode: AdapterRuntimeMode,
    /// Current provider connection state.
    pub connection_state: AdapterConnectionState,
    /// Redacted endpoint containing only URI scheme and authority.
    pub endpoint_redacted: Option<String>,
    /// Non-secret application name supplied to the provider.
    pub app_name: Option<String>,
    /// Consecutive reconnect attempt number.
    pub reconnect_attempt: u32,
    /// Number of active symbol subscriptions.
    pub subscription_count: usize,
    /// Deterministically sorted active symbol subscriptions.
    pub subscribed_symbols: Vec<SymbolId>,
    /// Number of provider events waiting in the adapter queue.
    pub queue_depth: usize,
    /// Configured queue bound, or `None` when unbounded or unknown.
    pub queue_capacity: Option<usize>,
    /// Number of provider events dropped by bounded buffering.
    pub dropped_events: u64,
    /// Number of detected provider sequence gaps.
    pub gap_count: u64,
    /// True when adapter-specific freshness policy considers the feed stale.
    pub stale: bool,
    /// True when bounded raw-message capture is enabled.
    pub raw_capture_enabled: bool,
    /// Number of raw messages currently retained.
    pub raw_capture_depth: usize,
    /// Configured raw-message capture bound.
    pub raw_capture_capacity: usize,
    /// Milliseconds since the last provider message, when observed.
    pub last_message_age_ms: Option<u64>,
    /// Milliseconds since the last normalized market-data event, when observed.
    pub last_market_data_age_ms: Option<u64>,
}

impl AdapterOperationalStatus {
    /// Creates an operational snapshot with the supplied mode and state.
    pub fn new(mode: AdapterRuntimeMode, connection_state: AdapterConnectionState) -> Self {
        Self {
            mode,
            connection_state,
            ..Self::default()
        }
    }

    /// Sets the transport mode.
    pub fn with_mode(mut self, mode: AdapterRuntimeMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the provider connection state.
    pub fn with_connection_state(mut self, state: AdapterConnectionState) -> Self {
        self.connection_state = state;
        self
    }

    /// Redacts and sets a configured endpoint.
    pub fn with_endpoint(mut self, endpoint: Option<&str>) -> Self {
        self.endpoint_redacted = endpoint.and_then(redact_adapter_endpoint);
        self
    }

    /// Sets a non-secret provider application name.
    pub fn with_app_name(mut self, app_name: Option<&str>) -> Self {
        self.app_name = app_name.map(str::to_owned);
        self
    }

    /// Sets the current reconnect attempt number.
    pub fn with_reconnect_attempt(mut self, reconnect_attempt: u32) -> Self {
        self.reconnect_attempt = reconnect_attempt;
        self
    }

    /// Sets and deterministically orders active subscriptions.
    pub fn with_subscribed_symbols<I>(mut self, symbols: I) -> Self
    where
        I: IntoIterator<Item = SymbolId>,
    {
        self.subscribed_symbols = symbols.into_iter().collect();
        self.subscribed_symbols.sort_unstable_by(|left, right| {
            (&left.venue, &left.symbol).cmp(&(&right.venue, &right.symbol))
        });
        self.subscribed_symbols.dedup();
        self.subscription_count = self.subscribed_symbols.len();
        self
    }

    /// Sets provider-event queue utilization.
    pub fn with_queue(mut self, depth: usize, capacity: Option<usize>) -> Self {
        self.queue_depth = depth;
        self.queue_capacity = capacity;
        self
    }

    /// Sets drop and sequence-gap counters.
    pub fn with_loss_counters(mut self, dropped_events: u64, gap_count: u64) -> Self {
        self.dropped_events = dropped_events;
        self.gap_count = gap_count;
        self
    }

    /// Sets the adapter-specific stale-feed state.
    pub fn with_stale(mut self, stale: bool) -> Self {
        self.stale = stale;
        self
    }

    /// Sets bounded raw-message capture utilization.
    pub fn with_raw_capture(mut self, depth: usize, capacity: usize) -> Self {
        self.raw_capture_enabled = capacity > 0;
        self.raw_capture_depth = depth;
        self.raw_capture_capacity = capacity;
        self
    }

    /// Sets provider-message and normalized-event ages.
    pub fn with_activity_ages(
        mut self,
        last_message_age_ms: Option<u64>,
        last_market_data_age_ms: Option<u64>,
    ) -> Self {
        self.last_message_age_ms = last_message_age_ms;
        self.last_market_data_age_ms = last_market_data_age_ms;
        self
    }
}

/// Returns a diagnostics-safe endpoint containing only URI scheme and authority.
///
/// Returns `None` for malformed endpoints. The result never contains user
/// information, a path, query parameters, or a fragment.
pub fn redact_adapter_endpoint(endpoint: &str) -> Option<String> {
    let endpoint = endpoint.trim();
    let (scheme, remainder) = endpoint.split_once("://")?;
    let mut scheme_chars = scheme.chars();
    if !scheme_chars
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic())
        || !scheme_chars
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '+' | '-' | '.'))
    {
        return None;
    }

    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let authority = &remainder[..authority_end];
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, value)| value);
    if authority.is_empty()
        || authority
            .chars()
            .any(|value| value.is_ascii_whitespace() || value.is_ascii_control())
    {
        return None;
    }

    Some(format!("{}://{}", scheme.to_ascii_lowercase(), authority))
}

/// Raw adapter event stream.
#[derive(Debug, Clone)]
pub enum RawEvent {
    /// Book update event.
    Book(BookUpdate),
    /// Trade print event.
    Trade(TradePrint),
}

/// Adapter-level error variants.
#[derive(Debug, Clone)]
pub enum AdapterError {
    /// Adapter is disconnected.
    Disconnected,
    /// Required configuration is missing.
    NotConfigured(&'static str),
    /// Build-time feature was not enabled for this provider.
    FeatureDisabled(&'static str),
    /// Provider-specific error message.
    Other(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterError::Disconnected => write!(f, "adapter disconnected"),
            AdapterError::NotConfigured(msg) => write!(f, "adapter misconfigured: {msg}"),
            AdapterError::FeatureDisabled(msg) => write!(f, "adapter feature disabled: {msg}"),
            AdapterError::Other(msg) => write!(f, "adapter error: {msg}"),
        }
    }
}

impl Error for AdapterError {}

/// Result type alias used by adapter interfaces.
pub type AdapterResult<T> = Result<T, AdapterError>;

/// Common market-data adapter interface used by runtime.
pub trait MarketDataAdapter: Send {
    /// Establishes provider connection/session.
    fn connect(&mut self) -> AdapterResult<()>;
    /// Starts or updates a symbol subscription.
    fn subscribe(&mut self, req: SubscribeReq) -> AdapterResult<()>;
    /// Stops a symbol subscription.
    fn unsubscribe(&mut self, symbol: SymbolId) -> AdapterResult<()>;
    /// Drains ready events into `out` and returns number appended.
    fn poll(&mut self, out: &mut Vec<RawEvent>) -> AdapterResult<usize>;
    /// Returns latest adapter health snapshot.
    fn health(&self) -> AdapterHealth;
    /// Returns typed operational status for diagnostics and supervision.
    ///
    /// The default preserves source compatibility for third-party adapters and
    /// reports unknown values until an implementation opts into richer status.
    fn operational_status(&self) -> AdapterOperationalStatus {
        AdapterOperationalStatus::default()
    }
}

impl MarketDataAdapter for Box<dyn MarketDataAdapter> {
    fn connect(&mut self) -> AdapterResult<()> {
        self.as_mut().connect()
    }

    fn subscribe(&mut self, req: SubscribeReq) -> AdapterResult<()> {
        self.as_mut().subscribe(req)
    }

    fn unsubscribe(&mut self, symbol: SymbolId) -> AdapterResult<()> {
        self.as_mut().unsubscribe(symbol)
    }

    fn poll(&mut self, out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
        self.as_mut().poll(out)
    }

    fn health(&self) -> AdapterHealth {
        self.as_ref().health()
    }

    fn operational_status(&self) -> AdapterOperationalStatus {
        self.as_ref().operational_status()
    }
}

/// Provider selection used by adapter factory configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    /// In-memory deterministic test provider.
    Mock,
    /// Rithmic adapter provider.
    Rithmic,
    /// CQG adapter provider.
    Cqg,
    /// Binance adapter provider.
    Binance,
}

impl ProviderKind {
    /// Returns the stable lowercase provider id used in diagnostics.
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Rithmic => "rithmic",
            Self::Cqg => "cqg",
            Self::Binance => "binance",
        }
    }
}

/// Adapter maturity level advertised by the discovery registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdapterQualityLevel {
    /// Early adapter implementation with no conformance claim.
    Experimental,
    /// Deterministic local adapter intended for tests, examples, and replay.
    Simulation,
    /// Build-time integration scaffold that is not live-production complete.
    Scaffold,
    /// Adapter passed deterministic simulator/conformance scenarios.
    SimulatedCertified,
    /// Live-capable adapter that still requires operator validation.
    Functional,
    /// Adapter has been exercised against a broker or exchange paper/sandbox environment.
    PaperTrading,
    /// Candidate for production use with recovery, runbook, and metrics.
    ProductionCandidate,
    /// Adapter has passed provider certification for a documented profile.
    Certified,
    /// Adapter has documented production-observed behavior for a specific profile.
    ProductionObserved,
}

impl AdapterQualityLevel {
    /// Returns the stable lowercase quality id used in diagnostics.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Simulation => "simulation",
            Self::Scaffold => "scaffold",
            Self::SimulatedCertified => "simulated_certified",
            Self::Functional => "functional",
            Self::PaperTrading => "paper_trading",
            Self::ProductionCandidate => "production_candidate",
            Self::Certified => "certified",
            Self::ProductionObserved => "production_observed",
        }
    }

    /// Returns the conservative ordering used by conformance reports.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Experimental => 0,
            Self::Simulation => 10,
            Self::Scaffold => 20,
            Self::SimulatedCertified => 30,
            Self::Functional => 40,
            Self::PaperTrading => 45,
            Self::ProductionCandidate => 50,
            Self::Certified => 60,
            Self::ProductionObserved => 70,
        }
    }

    /// Returns true when this level is at least as mature as `target`.
    pub const fn meets(self, target: Self) -> bool {
        self.rank() >= target.rank()
    }
}

/// Static capability description for one market-data adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AdapterDescriptor {
    /// Provider enum used by [`AdapterConfig`].
    pub provider: ProviderKind,
    /// Stable lowercase provider id.
    pub provider_id: &'static str,
    /// Human-readable provider name.
    pub display_name: &'static str,
    /// Cargo feature required for the adapter, or `None` when always present.
    pub feature: Option<&'static str>,
    /// True when this binary was compiled with the required adapter feature.
    pub compiled: bool,
    /// Public maturity level for this adapter.
    pub quality: AdapterQualityLevel,
    /// True when adapter can connect to a live provider endpoint.
    pub supports_live: bool,
    /// True when adapter can support deterministic local/replay flows.
    pub supports_replay: bool,
    /// True when adapter emits trade events.
    pub supports_trades: bool,
    /// True when adapter emits book/depth events.
    pub supports_order_book: bool,
    /// True when adapter supports level-2/depth updates beyond top-of-book.
    pub supports_level2: bool,
    /// True when adapter has reconnect behavior.
    pub supports_reconnect: bool,
    /// True when adapter has gap detection or recovery semantics.
    pub supports_gap_recovery: bool,
    /// True when adapter exposes bounded backpressure behavior or counters.
    pub supports_backpressure: bool,
    /// True when adapter can capture raw provider messages.
    pub supports_raw_capture: bool,
    /// True when adapter can replay provider-specific raw fixtures.
    pub supports_fixture_replay: bool,
    /// True when adapter has stale-feed detection.
    pub supports_stale_detection: bool,
    /// True when adapter reports parser or normalization latency metrics.
    pub supports_latency_metrics: bool,
    /// True when adapter is driven through the poll-based runtime contract.
    pub supports_polling: bool,
    /// Public certification evidence URL, document id, or profile id when known.
    pub certification_evidence: Option<&'static str>,
    /// Public production-observed evidence URL, document id, or profile id when known.
    pub production_evidence: Option<&'static str>,
    /// Short operator-facing note.
    pub notes: &'static str,
}

/// Adapter conformance requirement checked for a target quality level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdapterConformanceRequirement {
    /// Adapter advertised quality must meet the requested target quality.
    AdvertisedQuality,
    /// Adapter must be compiled into the current binary.
    Compiled,
    /// Adapter must support a live provider endpoint.
    LiveEndpoint,
    /// Adapter must support deterministic replay or simulator-driven workflows.
    ReplayOrSimulation,
    /// Adapter must emit normalized trades or book/depth events.
    MarketDataEvents,
    /// Adapter must use the poll-based runtime contract.
    PollingContract,
    /// Adapter must provide reconnect behavior.
    Reconnect,
    /// Adapter must provide sequence-gap detection or recovery semantics.
    GapRecovery,
    /// Adapter must expose bounded backpressure behavior or counters.
    Backpressure,
    /// Adapter must provide stale-feed detection.
    StaleDetection,
    /// Adapter must expose latency metrics.
    LatencyMetrics,
    /// Adapter must be able to capture raw provider messages for incidents.
    RawCapture,
    /// Adapter must be able to replay provider-specific raw fixtures.
    FixtureReplay,
    /// Adapter must carry public or operator-verifiable certification evidence.
    CertificationEvidence,
    /// Adapter must carry public or operator-verifiable production evidence.
    ProductionEvidence,
}

impl AdapterConformanceRequirement {
    /// Returns the stable lowercase requirement id used in reports.
    pub const fn id(self) -> &'static str {
        match self {
            Self::AdvertisedQuality => "advertised_quality",
            Self::Compiled => "compiled",
            Self::LiveEndpoint => "live_endpoint",
            Self::ReplayOrSimulation => "replay_or_simulation",
            Self::MarketDataEvents => "market_data_events",
            Self::PollingContract => "polling_contract",
            Self::Reconnect => "reconnect",
            Self::GapRecovery => "gap_recovery",
            Self::Backpressure => "backpressure",
            Self::StaleDetection => "stale_detection",
            Self::LatencyMetrics => "latency_metrics",
            Self::RawCapture => "raw_capture",
            Self::FixtureReplay => "fixture_replay",
            Self::CertificationEvidence => "certification_evidence",
            Self::ProductionEvidence => "production_evidence",
        }
    }
}

/// One failed adapter conformance requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterConformanceFailure {
    /// Requirement that failed.
    pub requirement: AdapterConformanceRequirement,
    /// Stable operator-facing explanation.
    pub message: &'static str,
}

/// Adapter conformance report for one descriptor and target quality level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterConformanceReport {
    /// Provider enum associated with the evaluated descriptor.
    pub provider: ProviderKind,
    /// Stable lowercase provider id.
    pub provider_id: &'static str,
    /// Quality level advertised by the descriptor.
    pub advertised_quality: AdapterQualityLevel,
    /// Quality level requested by the host or certification gate.
    pub target_quality: AdapterQualityLevel,
    /// Number of requirements evaluated.
    pub checked_requirements: usize,
    /// Failed requirements. An empty vector means the report passed.
    pub failures: Vec<AdapterConformanceFailure>,
}

impl AdapterConformanceReport {
    /// Returns true when every checked requirement passed.
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

const EXPERIMENTAL_REQUIREMENTS: &[AdapterConformanceRequirement] = &[
    AdapterConformanceRequirement::Compiled,
    AdapterConformanceRequirement::MarketDataEvents,
];
const SIMULATION_REQUIREMENTS: &[AdapterConformanceRequirement] = &[
    AdapterConformanceRequirement::Compiled,
    AdapterConformanceRequirement::ReplayOrSimulation,
    AdapterConformanceRequirement::MarketDataEvents,
    AdapterConformanceRequirement::PollingContract,
];
const SIMULATED_CERTIFIED_REQUIREMENTS: &[AdapterConformanceRequirement] = &[
    AdapterConformanceRequirement::Compiled,
    AdapterConformanceRequirement::ReplayOrSimulation,
    AdapterConformanceRequirement::MarketDataEvents,
    AdapterConformanceRequirement::PollingContract,
    AdapterConformanceRequirement::FixtureReplay,
];
const FUNCTIONAL_REQUIREMENTS: &[AdapterConformanceRequirement] = &[
    AdapterConformanceRequirement::Compiled,
    AdapterConformanceRequirement::LiveEndpoint,
    AdapterConformanceRequirement::MarketDataEvents,
    AdapterConformanceRequirement::PollingContract,
];
const PRODUCTION_CANDIDATE_REQUIREMENTS: &[AdapterConformanceRequirement] = &[
    AdapterConformanceRequirement::Compiled,
    AdapterConformanceRequirement::LiveEndpoint,
    AdapterConformanceRequirement::MarketDataEvents,
    AdapterConformanceRequirement::PollingContract,
    AdapterConformanceRequirement::Reconnect,
    AdapterConformanceRequirement::GapRecovery,
    AdapterConformanceRequirement::Backpressure,
    AdapterConformanceRequirement::StaleDetection,
    AdapterConformanceRequirement::LatencyMetrics,
    AdapterConformanceRequirement::RawCapture,
    AdapterConformanceRequirement::FixtureReplay,
];
const CERTIFIED_REQUIREMENTS: &[AdapterConformanceRequirement] = &[
    AdapterConformanceRequirement::Compiled,
    AdapterConformanceRequirement::LiveEndpoint,
    AdapterConformanceRequirement::MarketDataEvents,
    AdapterConformanceRequirement::PollingContract,
    AdapterConformanceRequirement::Reconnect,
    AdapterConformanceRequirement::GapRecovery,
    AdapterConformanceRequirement::Backpressure,
    AdapterConformanceRequirement::StaleDetection,
    AdapterConformanceRequirement::LatencyMetrics,
    AdapterConformanceRequirement::RawCapture,
    AdapterConformanceRequirement::FixtureReplay,
    AdapterConformanceRequirement::CertificationEvidence,
];
const PRODUCTION_OBSERVED_REQUIREMENTS: &[AdapterConformanceRequirement] = &[
    AdapterConformanceRequirement::Compiled,
    AdapterConformanceRequirement::LiveEndpoint,
    AdapterConformanceRequirement::MarketDataEvents,
    AdapterConformanceRequirement::PollingContract,
    AdapterConformanceRequirement::Reconnect,
    AdapterConformanceRequirement::GapRecovery,
    AdapterConformanceRequirement::Backpressure,
    AdapterConformanceRequirement::StaleDetection,
    AdapterConformanceRequirement::LatencyMetrics,
    AdapterConformanceRequirement::RawCapture,
    AdapterConformanceRequirement::FixtureReplay,
    AdapterConformanceRequirement::CertificationEvidence,
    AdapterConformanceRequirement::ProductionEvidence,
];

/// Returns the conformance requirements for a target adapter quality level.
pub const fn adapter_quality_requirements(
    target: AdapterQualityLevel,
) -> &'static [AdapterConformanceRequirement] {
    match target {
        AdapterQualityLevel::Experimental | AdapterQualityLevel::Scaffold => {
            EXPERIMENTAL_REQUIREMENTS
        }
        AdapterQualityLevel::Simulation => SIMULATION_REQUIREMENTS,
        AdapterQualityLevel::SimulatedCertified => SIMULATED_CERTIFIED_REQUIREMENTS,
        AdapterQualityLevel::Functional | AdapterQualityLevel::PaperTrading => {
            FUNCTIONAL_REQUIREMENTS
        }
        AdapterQualityLevel::ProductionCandidate => PRODUCTION_CANDIDATE_REQUIREMENTS,
        AdapterQualityLevel::Certified => CERTIFIED_REQUIREMENTS,
        AdapterQualityLevel::ProductionObserved => PRODUCTION_OBSERVED_REQUIREMENTS,
    }
}

const ADAPTER_DESCRIPTORS: [AdapterDescriptor; 4] = [
    AdapterDescriptor {
        provider: ProviderKind::Mock,
        provider_id: "mock",
        display_name: "Mock",
        feature: None,
        compiled: true,
        quality: AdapterQualityLevel::Simulation,
        supports_live: false,
        supports_replay: true,
        supports_trades: true,
        supports_order_book: true,
        supports_level2: true,
        supports_reconnect: false,
        supports_gap_recovery: false,
        supports_backpressure: false,
        supports_raw_capture: false,
        supports_fixture_replay: false,
        supports_stale_detection: false,
        supports_latency_metrics: false,
        supports_polling: true,
        certification_evidence: None,
        production_evidence: None,
        notes: "deterministic in-memory adapter for tests, demos, and replay harnesses",
    },
    AdapterDescriptor {
        provider: ProviderKind::Rithmic,
        provider_id: "rithmic",
        display_name: "Rithmic",
        feature: Some("rithmic"),
        compiled: cfg!(feature = "rithmic"),
        quality: AdapterQualityLevel::Scaffold,
        supports_live: cfg!(feature = "rithmic"),
        supports_replay: false,
        supports_trades: true,
        supports_order_book: true,
        supports_level2: true,
        supports_reconnect: cfg!(feature = "rithmic"),
        supports_gap_recovery: false,
        supports_backpressure: false,
        supports_raw_capture: false,
        supports_fixture_replay: false,
        supports_stale_detection: cfg!(feature = "rithmic"),
        supports_latency_metrics: false,
        supports_polling: true,
        certification_evidence: None,
        production_evidence: None,
        notes: "feature-gated futures adapter scaffold; validate venue behavior before live capital use",
    },
    AdapterDescriptor {
        provider: ProviderKind::Cqg,
        provider_id: "cqg",
        display_name: "CQG",
        feature: Some("cqg"),
        compiled: cfg!(feature = "cqg"),
        quality: AdapterQualityLevel::Functional,
        supports_live: cfg!(feature = "cqg"),
        supports_replay: false,
        supports_trades: true,
        supports_order_book: true,
        supports_level2: true,
        supports_reconnect: cfg!(feature = "cqg"),
        supports_gap_recovery: false,
        supports_backpressure: false,
        supports_raw_capture: false,
        supports_fixture_replay: false,
        supports_stale_detection: false,
        supports_latency_metrics: false,
        supports_polling: true,
        certification_evidence: None,
        production_evidence: None,
        notes: "feature-gated CQG adapter with reconnect/resubscribe hardening",
    },
    AdapterDescriptor {
        provider: ProviderKind::Binance,
        provider_id: "binance",
        display_name: "Binance",
        feature: Some("binance"),
        compiled: cfg!(feature = "binance"),
        quality: AdapterQualityLevel::Scaffold,
        supports_live: cfg!(feature = "binance"),
        supports_replay: false,
        supports_trades: true,
        supports_order_book: true,
        supports_level2: true,
        supports_reconnect: cfg!(feature = "binance"),
        supports_gap_recovery: cfg!(feature = "binance"),
        supports_backpressure: cfg!(feature = "binance"),
        supports_raw_capture: cfg!(feature = "binance"),
        supports_fixture_replay: cfg!(feature = "binance"),
        supports_stale_detection: cfg!(feature = "binance"),
        supports_latency_metrics: cfg!(feature = "binance"),
        supports_polling: true,
        certification_evidence: None,
        production_evidence: None,
        notes: "feature-gated crypto adapter with websocket trade/depth parsing, reconnects, gap detection, bounded backpressure, raw capture, and fixture replay",
    },
];

/// Returns static descriptors for all known adapter providers.
pub fn adapter_descriptors() -> &'static [AdapterDescriptor] {
    &ADAPTER_DESCRIPTORS
}

/// Returns descriptors for providers compiled into the current binary.
pub fn compiled_adapter_descriptors() -> Vec<AdapterDescriptor> {
    adapter_descriptors()
        .iter()
        .filter(|descriptor| descriptor.compiled)
        .cloned()
        .collect()
}

/// Returns the descriptor for `provider`.
pub fn describe_adapter(provider: ProviderKind) -> AdapterDescriptor {
    adapter_descriptors()
        .iter()
        .find(|descriptor| descriptor.provider == provider)
        .cloned()
        .expect("all ProviderKind variants have descriptors")
}

/// Returns true when the current binary can construct `provider`.
pub fn adapter_feature_enabled(provider: ProviderKind) -> bool {
    describe_adapter(provider).compiled
}

/// Evaluates whether a descriptor satisfies a target adapter quality level.
///
/// This is a control-plane helper for dashboards, CLIs, binding generators, and
/// certification scripts. It does not construct or connect an adapter.
pub fn evaluate_adapter_conformance(
    descriptor: &AdapterDescriptor,
    target_quality: AdapterQualityLevel,
) -> AdapterConformanceReport {
    let requirements = adapter_quality_requirements(target_quality);
    let mut failures = Vec::new();

    for requirement in requirements {
        if let Some(message) = adapter_conformance_failure_message(descriptor, *requirement) {
            failures.push(AdapterConformanceFailure {
                requirement: *requirement,
                message,
            });
        }
    }

    if !descriptor.quality.meets(target_quality) {
        failures.push(AdapterConformanceFailure {
            requirement: AdapterConformanceRequirement::AdvertisedQuality,
            message: "advertised quality is below the requested target quality",
        });
    }

    AdapterConformanceReport {
        provider: descriptor.provider.clone(),
        provider_id: descriptor.provider_id,
        advertised_quality: descriptor.quality,
        target_quality,
        checked_requirements: requirements.len(),
        failures,
    }
}

/// Evaluates a known provider against a target adapter quality level.
pub fn adapter_conformance_report(
    provider: ProviderKind,
    target_quality: AdapterQualityLevel,
) -> AdapterConformanceReport {
    let descriptor = describe_adapter(provider);
    evaluate_adapter_conformance(&descriptor, target_quality)
}

fn adapter_conformance_failure_message(
    descriptor: &AdapterDescriptor,
    requirement: AdapterConformanceRequirement,
) -> Option<&'static str> {
    match requirement {
        AdapterConformanceRequirement::Compiled if !descriptor.compiled => {
            Some("adapter feature is not compiled into this binary")
        }
        AdapterConformanceRequirement::LiveEndpoint if !descriptor.supports_live => {
            Some("adapter does not advertise live endpoint support")
        }
        AdapterConformanceRequirement::ReplayOrSimulation if !descriptor.supports_replay => {
            Some("adapter does not advertise deterministic replay or simulation support")
        }
        AdapterConformanceRequirement::MarketDataEvents
            if !(descriptor.supports_trades || descriptor.supports_order_book) =>
        {
            Some("adapter does not advertise normalized trade or book events")
        }
        AdapterConformanceRequirement::PollingContract if !descriptor.supports_polling => {
            Some("adapter does not advertise the poll-based runtime contract")
        }
        AdapterConformanceRequirement::Reconnect if !descriptor.supports_reconnect => {
            Some("adapter does not advertise reconnect behavior")
        }
        AdapterConformanceRequirement::GapRecovery if !descriptor.supports_gap_recovery => {
            Some("adapter does not advertise sequence-gap detection or recovery")
        }
        AdapterConformanceRequirement::Backpressure if !descriptor.supports_backpressure => {
            Some("adapter does not advertise bounded backpressure behavior or counters")
        }
        AdapterConformanceRequirement::StaleDetection if !descriptor.supports_stale_detection => {
            Some("adapter does not advertise stale-feed detection")
        }
        AdapterConformanceRequirement::LatencyMetrics if !descriptor.supports_latency_metrics => {
            Some("adapter does not advertise latency metrics")
        }
        AdapterConformanceRequirement::RawCapture if !descriptor.supports_raw_capture => {
            Some("adapter does not advertise raw provider-message capture")
        }
        AdapterConformanceRequirement::FixtureReplay if !descriptor.supports_fixture_replay => {
            Some("adapter does not advertise provider fixture replay")
        }
        AdapterConformanceRequirement::CertificationEvidence
            if descriptor.certification_evidence.is_none() =>
        {
            Some("adapter does not provide certification evidence")
        }
        AdapterConformanceRequirement::ProductionEvidence
            if descriptor.production_evidence.is_none() =>
        {
            Some("adapter does not provide production-observed evidence")
        }
        _ => None,
    }
}

/// Generic adapter factory configuration.
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    /// Provider selection.
    pub provider: ProviderKind,
    /// Optional credentials env-key references.
    pub credentials: Option<CredentialsRef>,
    /// Provider endpoint URI.
    pub endpoint: Option<String>,
    /// Optional client/app name.
    pub app_name: Option<String>,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Mock,
            credentials: None,
            endpoint: None,
            app_name: None,
        }
    }
}

/// Credential environment-variable references for adapter auth bootstrap.
#[derive(Debug, Clone)]
pub struct CredentialsRef {
    /// Environment variable name for key id/user id.
    pub key_id_env: String,
    /// Environment variable name for secret/password.
    pub secret_env: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TlsFileConfig {
    ca_file: Option<String>,
    client_cert_file: Option<String>,
    client_chain_file: Option<String>,
    client_key_file: Option<String>,
    client_key_password_env: Option<String>,
}

impl TlsFileConfig {
    fn from_env(provider: &str) -> AdapterResult<Self> {
        let ca_file = tls_env(provider, "CA_FILE");
        let client_cert_file = tls_env(provider, "CLIENT_CERT_FILE");
        let client_chain_file = tls_env(provider, "CLIENT_CHAIN_FILE");
        let client_key_file = tls_env(provider, "CLIENT_KEY_FILE");
        let client_key_password_env = tls_env(provider, "CLIENT_KEY_PASSWORD_ENV");

        if client_cert_file.is_some() != client_key_file.is_some() {
            return Err(AdapterError::NotConfigured(
                "TLS client certificate and private key must be configured together",
            ));
        }
        if let Some(password_env) = &client_key_password_env {
            if env::var_os(password_env).is_none() {
                return Err(AdapterError::Other(format!(
                    "TLS private-key password env var is not set: {password_env}"
                )));
            }
        }

        for (name, path) in [
            ("CA file", ca_file.as_deref()),
            ("client certificate file", client_cert_file.as_deref()),
            (
                "client certificate chain file",
                client_chain_file.as_deref(),
            ),
            ("client private key file", client_key_file.as_deref()),
        ] {
            if let Some(path) = path {
                if !Path::new(path).is_file() {
                    return Err(AdapterError::Other(format!(
                        "TLS {name} does not exist or is not a file: {path}"
                    )));
                }
            }
        }

        Ok(Self {
            ca_file,
            client_cert_file,
            client_chain_file,
            client_key_file,
            client_key_password_env,
        })
    }

    fn openssl_args(&self, host: &str, port: u16) -> Vec<String> {
        let mut args = vec![
            "s_client".to_string(),
            "-quiet".to_string(),
            "-verify_return_error".to_string(),
            "-verify_hostname".to_string(),
            host.to_string(),
            "-connect".to_string(),
            format!("{host}:{port}"),
            "-servername".to_string(),
            host.to_string(),
        ];
        if let Some(path) = &self.ca_file {
            args.extend(["-CAfile".to_string(), path.clone()]);
        }
        if let Some(path) = &self.client_cert_file {
            args.extend(["-cert".to_string(), path.clone()]);
        }
        if let Some(path) = &self.client_chain_file {
            args.extend(["-cert_chain".to_string(), path.clone()]);
        }
        if let Some(path) = &self.client_key_file {
            args.extend(["-key".to_string(), path.clone()]);
        }
        if let Some(password_env) = &self.client_key_password_env {
            args.extend(["-passin".to_string(), format!("env:{password_env}")]);
        }
        args
    }
}

pub(crate) fn openssl_s_client_args(
    provider: &str,
    host: &str,
    port: u16,
) -> AdapterResult<Vec<String>> {
    Ok(TlsFileConfig::from_env(provider)?.openssl_args(host, port))
}

fn tls_env(provider: &str, suffix: &str) -> Option<String> {
    let provider_name = provider.to_ascii_uppercase();
    let scoped = format!("ORDERFLOW_{provider_name}_TLS_{suffix}");
    env::var(&scoped)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var(format!("ORDERFLOW_TLS_{suffix}"))
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
}

/// Creates a provider adapter from configuration.
pub fn create_adapter(cfg: &AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>> {
    match &cfg.provider {
        ProviderKind::Mock => Ok(Box::new(MockAdapter::default())),
        ProviderKind::Rithmic => create_rithmic_adapter(cfg),
        ProviderKind::Cqg => create_cqg_adapter(cfg),
        ProviderKind::Binance => create_binance_adapter(cfg),
    }
}

fn create_rithmic_adapter(cfg: &AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>> {
    #[cfg(feature = "rithmic")]
    {
        let adapter = rithmic::RithmicAdapter::from_config(cfg)?;
        Ok(Box::new(adapter))
    }

    #[cfg(not(feature = "rithmic"))]
    {
        let _ = cfg;
        Err(AdapterError::FeatureDisabled(
            "compile with --features rithmic to enable",
        ))
    }
}

fn create_cqg_adapter(cfg: &AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>> {
    #[cfg(feature = "cqg")]
    {
        let adapter = cqg::CqgAdapter::from_config(cfg)?;
        Ok(Box::new(adapter))
    }

    #[cfg(not(feature = "cqg"))]
    {
        let _ = cfg;
        Err(AdapterError::FeatureDisabled(
            "compile with --features cqg to enable",
        ))
    }
}

fn create_binance_adapter(cfg: &AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>> {
    #[cfg(feature = "binance")]
    {
        let adapter = binance::BinanceAdapter::from_config(cfg)?;
        Ok(Box::new(adapter))
    }

    #[cfg(not(feature = "binance"))]
    {
        let _ = cfg;
        Err(AdapterError::FeatureDisabled(
            "compile with --features binance to enable",
        ))
    }
}

/// Deterministic in-memory adapter for tests, demos, and replay harnesses.
#[derive(Debug, Default)]
pub struct MockAdapter {
    /// Connection state flag.
    pub connected: bool,
    /// Subscribed symbols for tests.
    pub subscribed: Vec<SubscribeReq>,
    queue: Vec<RawEvent>,
}

impl MockAdapter {
    /// Pushes an event into mock queue, drained by `poll`.
    pub fn push_event(&mut self, event: RawEvent) {
        self.queue.push(event);
    }
}

impl MarketDataAdapter for MockAdapter {
    fn connect(&mut self) -> AdapterResult<()> {
        self.connected = true;
        Ok(())
    }

    fn subscribe(&mut self, req: SubscribeReq) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        self.subscribed.push(req);
        Ok(())
    }

    fn poll(&mut self, out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        let n = self.queue.len();
        out.append(&mut self.queue);
        Ok(n)
    }

    fn unsubscribe(&mut self, symbol: SymbolId) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        self.subscribed.retain(|s| s.symbol != symbol);
        Ok(())
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth {
            connected: self.connected,
            degraded: false,
            last_error: None,
            protocol_info: Some("mock_adapter".to_string()),
        }
    }

    fn operational_status(&self) -> AdapterOperationalStatus {
        let state = if self.connected {
            AdapterConnectionState::Streaming
        } else {
            AdapterConnectionState::Disconnected
        };
        AdapterOperationalStatus::new(AdapterRuntimeMode::Mock, state)
            .with_subscribed_symbols(self.subscribed.iter().map(|req| req.symbol.clone()))
            .with_queue(self.queue.len(), None)
    }
}

#[cfg(feature = "rithmic")]
/// Rithmic adapter implementation (feature-gated).
pub mod rithmic;

#[cfg(feature = "cqg")]
/// CQG adapter implementation (feature-gated).
pub mod cqg;

#[cfg(feature = "binance")]
/// Binance adapter implementation (feature-gated).
pub mod binance;

#[cfg(test)]
mod tests {
    use super::*;

    struct LegacyStyleAdapter;

    impl MarketDataAdapter for LegacyStyleAdapter {
        fn connect(&mut self) -> AdapterResult<()> {
            Ok(())
        }

        fn subscribe(&mut self, _req: SubscribeReq) -> AdapterResult<()> {
            Ok(())
        }

        fn unsubscribe(&mut self, _symbol: SymbolId) -> AdapterResult<()> {
            Ok(())
        }

        fn poll(&mut self, _out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
            Ok(0)
        }

        fn health(&self) -> AdapterHealth {
            AdapterHealth::default()
        }
    }

    #[test]
    fn factory_returns_mock_by_default() {
        let cfg = AdapterConfig::default();
        let mut adapter = create_adapter(&cfg).expect("adapter should be created");
        adapter.connect().expect("connect should work");
        assert!(adapter.health().connected);
    }

    #[test]
    fn default_operational_status_preserves_existing_adapter_implementations() {
        let status = LegacyStyleAdapter.operational_status();
        assert_eq!(status.mode, AdapterRuntimeMode::Unknown);
        assert_eq!(status.connection_state, AdapterConnectionState::Unknown);
    }

    #[test]
    fn endpoint_redaction_omits_every_potential_secret_component() {
        assert_eq!(
            redact_adapter_endpoint(
                "WSS://user:secret@stream.example:9443/ws/listen-key?token=private#fragment"
            ),
            Some("wss://stream.example:9443".to_string())
        );
        assert_eq!(
            redact_adapter_endpoint("mock://local-provider/path"),
            Some("mock://local-provider".to_string())
        );
        assert_eq!(redact_adapter_endpoint("missing-scheme.example"), None);
        assert_eq!(redact_adapter_endpoint("1bad://example"), None);
        assert_eq!(redact_adapter_endpoint("wss://user:secret@/path"), None);
    }

    #[test]
    fn mock_operational_status_sorts_and_deduplicates_symbols() {
        let mut adapter = MockAdapter::default();
        adapter.connect().expect("connect");
        for (venue, symbol) in [("CME", "NQM6"), ("CME", "ESM6"), ("CME", "ESM6")] {
            adapter
                .subscribe(SubscribeReq {
                    symbol: SymbolId {
                        venue: venue.to_string(),
                        symbol: symbol.to_string(),
                    },
                    depth_levels: 10,
                })
                .expect("subscribe");
        }

        let status = adapter.operational_status();
        assert_eq!(status.subscription_count, 2);
        assert_eq!(status.subscribed_symbols[0].symbol, "ESM6");
        assert_eq!(status.subscribed_symbols[1].symbol, "NQM6");
        assert_eq!(status.connection_state, AdapterConnectionState::Streaming);
    }

    #[test]
    fn descriptors_cover_all_known_providers() {
        let descriptors = adapter_descriptors();
        assert_eq!(descriptors.len(), 4);
        assert_eq!(describe_adapter(ProviderKind::Mock).provider_id, "mock");
        assert_eq!(
            describe_adapter(ProviderKind::Rithmic).provider_id,
            "rithmic"
        );
        assert_eq!(describe_adapter(ProviderKind::Cqg).provider_id, "cqg");
        assert_eq!(
            describe_adapter(ProviderKind::Binance).provider_id,
            "binance"
        );
        let binance = describe_adapter(ProviderKind::Binance);
        assert_eq!(binance.supports_raw_capture, cfg!(feature = "binance"));
        assert_eq!(binance.supports_fixture_replay, cfg!(feature = "binance"));
        assert_eq!(binance.supports_backpressure, cfg!(feature = "binance"));
        assert_eq!(binance.supports_stale_detection, cfg!(feature = "binance"));
        assert_eq!(binance.supports_latency_metrics, cfg!(feature = "binance"));
    }

    #[test]
    fn compiled_descriptors_match_feature_enabled_helper() {
        for descriptor in adapter_descriptors() {
            assert_eq!(
                descriptor.compiled,
                adapter_feature_enabled(descriptor.provider.clone())
            );
        }
        assert!(adapter_feature_enabled(ProviderKind::Mock));
        assert!(compiled_adapter_descriptors()
            .iter()
            .any(|descriptor| descriptor.provider == ProviderKind::Mock));
    }

    #[test]
    fn adapter_quality_ladder_preserves_production_distinctions() {
        assert!(AdapterQualityLevel::ProductionObserved.meets(AdapterQualityLevel::Certified));
        assert!(AdapterQualityLevel::Certified.meets(AdapterQualityLevel::ProductionCandidate));
        assert!(AdapterQualityLevel::PaperTrading.meets(AdapterQualityLevel::Functional));
        assert!(!AdapterQualityLevel::Functional.meets(AdapterQualityLevel::PaperTrading));
        assert!(!AdapterQualityLevel::Simulation.meets(AdapterQualityLevel::Scaffold));
    }

    #[test]
    fn conformance_report_accepts_mock_simulation_target() {
        let report =
            adapter_conformance_report(ProviderKind::Mock, AdapterQualityLevel::Simulation);
        assert!(report.passed(), "{report:?}");
        assert_eq!(report.checked_requirements, 4);
        assert_eq!(report.provider_id, "mock");
    }

    #[test]
    fn conformance_report_rejects_uncertified_production_claims() {
        let report =
            adapter_conformance_report(ProviderKind::Binance, AdapterQualityLevel::Certified);
        assert!(!report.passed());
        assert!(report.failures.iter().any(|failure| {
            failure.requirement == AdapterConformanceRequirement::CertificationEvidence
        }));
        assert!(report.failures.iter().any(|failure| {
            failure.requirement == AdapterConformanceRequirement::AdvertisedQuality
        }));
    }

    #[test]
    fn conformance_report_identifies_disabled_live_provider() {
        let report =
            adapter_conformance_report(ProviderKind::Rithmic, AdapterQualityLevel::Functional);
        if cfg!(feature = "rithmic") {
            assert!(report
                .failures
                .iter()
                .all(|failure| { failure.requirement != AdapterConformanceRequirement::Compiled }));
        } else {
            assert!(report
                .failures
                .iter()
                .any(|failure| { failure.requirement == AdapterConformanceRequirement::Compiled }));
        }
    }

    #[test]
    fn tls_arguments_enforce_hostname_verification_without_file_configuration() {
        let args = TlsFileConfig::default().openssl_args("feed.example.test", 443);

        assert!(args.windows(2).any(|pair| {
            pair == [
                "-verify_hostname".to_string(),
                "feed.example.test".to_string(),
            ]
        }));
        assert!(args
            .windows(2)
            .any(|pair| { pair == ["-connect".to_string(), "feed.example.test:443".to_string()] }));
        assert!(args.contains(&"-verify_return_error".to_string()));
        assert!(!args.iter().any(|arg| arg == "-CAfile"));
        assert!(!args.iter().any(|arg| arg == "-cert"));
        assert!(!args.iter().any(|arg| arg == "-key"));
    }

    #[test]
    fn tls_arguments_keep_credentials_as_paths_or_environment_references() {
        let config = TlsFileConfig {
            ca_file: Some("/run/secrets/venue-ca.pem".to_string()),
            client_cert_file: Some("/run/secrets/client.pem".to_string()),
            client_chain_file: Some("/run/secrets/chain.pem".to_string()),
            client_key_file: Some("/run/secrets/client-key.pem".to_string()),
            client_key_password_env: Some("CLIENT_KEY_PASSWORD".to_string()),
        };

        let args = config.openssl_args("feed.example.test", 443);

        assert!(args.windows(2).any(|pair| {
            pair == [
                "-CAfile".to_string(),
                "/run/secrets/venue-ca.pem".to_string(),
            ]
        }));
        assert!(args
            .windows(2)
            .any(|pair| { pair == ["-cert".to_string(), "/run/secrets/client.pem".to_string()] }));
        assert!(args.windows(2).any(|pair| {
            pair == [
                "-cert_chain".to_string(),
                "/run/secrets/chain.pem".to_string(),
            ]
        }));
        assert!(args.windows(2).any(|pair| {
            pair == [
                "-key".to_string(),
                "/run/secrets/client-key.pem".to_string(),
            ]
        }));
        assert!(args.windows(2).any(|pair| {
            pair == ["-passin".to_string(), "env:CLIENT_KEY_PASSWORD".to_string()]
        }));
        assert!(!args.iter().any(|arg| arg == "actual-secret-value"));
    }

    #[cfg(not(feature = "rithmic"))]
    #[test]
    fn factory_rejects_disabled_provider_features() {
        let cfg = AdapterConfig {
            provider: ProviderKind::Rithmic,
            ..AdapterConfig::default()
        };
        match create_adapter(&cfg) {
            Err(AdapterError::FeatureDisabled(_)) => {}
            Err(other) => panic!("unexpected error variant: {other}"),
            Ok(_) => panic!("expected feature-disabled error"),
        }
    }
}
