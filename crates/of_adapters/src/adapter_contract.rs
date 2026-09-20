use super::*;

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
