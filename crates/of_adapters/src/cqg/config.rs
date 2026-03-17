use crate::{AdapterConfig, AdapterError, AdapterResult};

/// Resolved CQG adapter runtime configuration.
#[derive(Debug, Clone)]
pub struct CqgConfig {
    /// CQG websocket endpoint (`wss://`, `ws://`, or `mock://`).
    pub endpoint: String,
    /// CQG private label/account namespace.
    pub private_label: String,
    /// CQG client identifier (typically application id).
    pub client_id: String,
    /// CQG username resolved from credential environment reference.
    pub username: String,
    /// CQG password resolved from credential environment reference.
    pub password: String,
    /// Ping interval used for keepalive checks.
    pub ping_interval_secs: u64,
    /// Max heartbeat silence before degraded detection.
    pub heartbeat_timeout_secs: u64,
    /// Minimum reconnect backoff in milliseconds.
    pub reconnect_min_ms: u64,
    /// Maximum reconnect backoff in milliseconds.
    pub reconnect_max_ms: u64,
    /// Maximum concurrently in-flight protocol requests.
    pub max_inflight_requests: u32,
}

impl CqgConfig {
    /// Builds CQG config from generic adapter configuration plus environment vars.
    pub fn from_adapter_config(cfg: &AdapterConfig) -> AdapterResult<Self> {
        let endpoint = cfg
            .endpoint
            .clone()
            .ok_or(AdapterError::NotConfigured("missing cqg endpoint"))?;
        if !endpoint.starts_with("wss://")
            && !endpoint.starts_with("ws://")
            && !endpoint.starts_with("mock://")
        {
            return Err(AdapterError::NotConfigured(
                "cqg endpoint must use wss://, ws://, or mock://",
            ));
        }

        let creds = cfg.credentials.as_ref().ok_or(AdapterError::NotConfigured(
            "missing cqg credentials reference",
        ))?;
        let username = read_env(&creds.key_id_env)?;
        let password = read_env(&creds.secret_env)?;

        let client_id = cfg
            .app_name
            .clone()
            .unwrap_or_else(|| "orderflow".to_string());

        Ok(Self {
            endpoint,
            private_label: "WebAPITest".to_string(),
            client_id,
            username,
            password,
            ping_interval_secs: 15,
            heartbeat_timeout_secs: 45,
            reconnect_min_ms: 250,
            reconnect_max_ms: 10_000,
            max_inflight_requests: 1024,
        })
    }

    /// Validates runtime invariants for reconnect and heartbeat policies.
    pub fn validate_runtime(&self) -> AdapterResult<()> {
        if self.reconnect_min_ms > self.reconnect_max_ms {
            return Err(AdapterError::NotConfigured(
                "reconnect_min_ms must be <= reconnect_max_ms",
            ));
        }
        if self.max_inflight_requests == 0 {
            return Err(AdapterError::NotConfigured(
                "max_inflight_requests must be > 0",
            ));
        }
        if self.heartbeat_timeout_secs < self.ping_interval_secs {
            return Err(AdapterError::NotConfigured(
                "heartbeat_timeout_secs must be >= ping_interval_secs",
            ));
        }
        Ok(())
    }
}

fn read_env(refs: &str) -> AdapterResult<String> {
    if refs.trim().is_empty() {
        return Err(AdapterError::NotConfigured("empty env reference"));
    }
    let value = std::env::var(refs)
        .map_err(|_| AdapterError::NotConfigured("required cqg env var missing"))?;
    if value.trim().is_empty() {
        return Err(AdapterError::NotConfigured("required cqg env var empty"));
    }
    Ok(value)
}
