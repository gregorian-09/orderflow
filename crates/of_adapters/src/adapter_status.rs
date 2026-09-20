use super::*;

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
