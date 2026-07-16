#![doc = include_str!("../README.md")]

use std::error::Error;
use std::fmt;

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
    /// Deterministic local adapter intended for tests, examples, and replay.
    Simulation,
    /// Build-time integration scaffold that is not live-production complete.
    Scaffold,
    /// Live-capable adapter that still requires operator validation.
    Functional,
    /// Candidate for production use with recovery, runbook, and metrics.
    ProductionCandidate,
}

impl AdapterQualityLevel {
    /// Returns the stable lowercase quality id used in diagnostics.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Simulation => "simulation",
            Self::Scaffold => "scaffold",
            Self::Functional => "functional",
            Self::ProductionCandidate => "production_candidate",
        }
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
    /// True when adapter is driven through the poll-based runtime contract.
    pub supports_polling: bool,
    /// Short operator-facing note.
    pub notes: &'static str,
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
        supports_polling: true,
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
        supports_polling: true,
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
        supports_polling: true,
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
        supports_gap_recovery: false,
        supports_polling: true,
        notes: "feature-gated crypto adapter scaffold; streaming production path is still maturing",
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

    #[test]
    fn factory_returns_mock_by_default() {
        let cfg = AdapterConfig::default();
        let mut adapter = create_adapter(&cfg).expect("adapter should be created");
        adapter.connect().expect("connect should work");
        assert!(adapter.health().connected);
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
