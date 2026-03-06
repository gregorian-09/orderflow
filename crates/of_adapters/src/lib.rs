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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct CredentialsRef {
    /// Environment variable name for key id/user id.
    pub key_id_env: String,
    /// Environment variable name for secret/password.
    pub secret_env: String,
}

/// Creates a provider adapter from configuration.
pub fn create_adapter(cfg: &AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>> {
    match cfg.provider {
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
        return Ok(Box::new(adapter));
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
        return Ok(Box::new(adapter));
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
        return Ok(Box::new(adapter));
    }

    #[cfg(not(feature = "binance"))]
    {
        let _ = cfg;
        Err(AdapterError::FeatureDisabled(
            "compile with --features binance to enable",
        ))
    }
}

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
        out.extend(self.queue.drain(..));
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
