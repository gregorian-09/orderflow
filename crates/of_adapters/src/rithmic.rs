use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use of_core::{BookAction, BookUpdate, Side, SymbolId, TradePrint};

use crate::{
    redact_adapter_endpoint, AdapterConfig, AdapterConnectionState, AdapterError, AdapterHealth,
    AdapterOperationalStatus, AdapterResult, AdapterRuntimeMode, MarketDataAdapter, RawEvent,
    SubscribeReq, TextWebSocket,
};

/// Resolved runtime configuration for the feature-gated Rithmic adapter.
#[derive(Debug, Clone)]
pub struct RithmicConfig {
    endpoint: String,
    user: String,
    pass: String,
    app_name: String,
}

impl RithmicConfig {
    /// Builds a validated Rithmic config from generic adapter config input.
    pub fn from_adapter_config(cfg: &AdapterConfig) -> AdapterResult<Self> {
        let endpoint = cfg
            .endpoint
            .clone()
            .ok_or(AdapterError::NotConfigured("missing rithmic endpoint"))?;
        if !endpoint.starts_with("wss://")
            && !endpoint.starts_with("ws://")
            && !endpoint.starts_with("mock://")
        {
            return Err(AdapterError::NotConfigured(
                "rithmic endpoint must use wss://, ws://, or mock://",
            ));
        }

        let creds = cfg.credentials.as_ref().ok_or(AdapterError::NotConfigured(
            "missing rithmic credentials reference",
        ))?;
        let user = read_env(&creds.key_id_env)?;
        let pass = read_env(&creds.secret_env)?;
        let app_name = cfg
            .app_name
            .clone()
            .unwrap_or_else(|| "orderflow".to_string());

        Ok(Self {
            endpoint,
            user,
            pass,
            app_name,
        })
    }
}

fn read_env(name: &str) -> AdapterResult<String> {
    if name.trim().is_empty() {
        return Err(AdapterError::NotConfigured("empty env reference"));
    }
    let v = std::env::var(name)
        .map_err(|_| AdapterError::NotConfigured("required rithmic env var missing"))?;
    if v.trim().is_empty() {
        return Err(AdapterError::NotConfigured(
            "required rithmic env var empty",
        ));
    }
    Ok(v)
}

#[derive(Debug)]
enum RithmicTransport {
    Mock,
    Live(TextWebSocket),
}

/// Rithmic adapter implementing the common market-data adapter trait.
#[derive(Debug)]
pub struct RithmicAdapter {
    cfg: RithmicConfig,
    transport: RithmicTransport,
    connected: bool,
    degraded: bool,
    last_error: Option<String>,
    requested_depth: HashMap<SymbolId, u16>,
    queue: VecDeque<RawEvent>,
    sequence: u64,
    connected_at: Option<Instant>,
    last_poll_at: Option<Instant>,
    reconnect_attempt: u32,
    next_reconnect_at: Option<Instant>,
    last_message_at: Option<Instant>,
    last_heartbeat_at: Option<Instant>,
    healthy_since: Option<Instant>,
}

impl RithmicAdapter {
    /// Creates a Rithmic adapter instance from generic adapter configuration.
    pub fn from_config(cfg: &AdapterConfig) -> AdapterResult<Self> {
        let cfg = RithmicConfig::from_adapter_config(cfg)?;
        let transport = if cfg.endpoint.starts_with("mock://") {
            RithmicTransport::Mock
        } else {
            RithmicTransport::Live(TextWebSocket::new(cfg.endpoint.clone()))
        };
        Ok(Self {
            cfg,
            transport,
            connected: false,
            degraded: false,
            last_error: None,
            requested_depth: HashMap::new(),
            queue: VecDeque::new(),
            sequence: 0,
            connected_at: None,
            last_poll_at: None,
            reconnect_attempt: 0,
            next_reconnect_at: None,
            last_message_at: None,
            last_heartbeat_at: None,
            healthy_since: None,
        })
    }

    fn is_mock_mode(&self) -> bool {
        matches!(self.transport, RithmicTransport::Mock)
    }

    fn now_ns() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence = self.sequence.saturating_add(1);
        self.sequence
    }

    fn synth_book_and_trade(&mut self, symbol: &SymbolId, depth_levels: u16) {
        let depth = depth_levels.clamp(1, 10);
        let ts_recv_ns = Self::now_ns();
        let ts_exchange_ns = ts_recv_ns.saturating_sub(500_000);
        let base_price = if symbol.symbol.to_ascii_uppercase().contains("NQ") {
            1_780_000
        } else {
            505_000
        };

        for level in 0..depth.min(2) {
            let seq = self.next_sequence();
            self.queue.push_back(RawEvent::Book(BookUpdate {
                symbol: symbol.clone(),
                side: Side::Bid,
                level,
                price: base_price - (level as i64 * 25),
                size: 5 + level as i64,
                action: BookAction::Upsert,
                sequence: seq,
                ts_exchange_ns,
                ts_recv_ns,
            }));
        }
        for level in 0..depth.min(2) {
            let seq = self.next_sequence();
            self.queue.push_back(RawEvent::Book(BookUpdate {
                symbol: symbol.clone(),
                side: Side::Ask,
                level,
                price: base_price + 25 + (level as i64 * 25),
                size: 4 + level as i64,
                action: BookAction::Upsert,
                sequence: seq,
                ts_exchange_ns,
                ts_recv_ns,
            }));
        }

        let seq = self.next_sequence();
        self.queue.push_back(RawEvent::Trade(TradePrint {
            symbol: symbol.clone(),
            price: base_price + (seq % 4) as i64 * 25,
            size: 1 + (seq % 4) as i64,
            aggressor_side: if seq.is_multiple_of(2) {
                Side::Ask
            } else {
                Side::Bid
            },
            sequence: seq,
            ts_exchange_ns,
            ts_recv_ns,
        }));
    }

    fn bootstrap_live_session(&mut self) -> AdapterResult<()> {
        let payload = format!(
            "{{\"type\":\"login_probe\",\"user\":\"{}\",\"app_name\":\"{}\"}}",
            escape_json(&self.cfg.user),
            escape_json(&self.cfg.app_name)
        );
        match &mut self.transport {
            RithmicTransport::Live(ws) => ws.send_text("rithmic", payload),
            RithmicTransport::Mock => Ok(()),
        }
    }

    fn send_subscribe_wire(&mut self, symbol: &SymbolId, depth_levels: u16) -> AdapterResult<()> {
        let payload = format!(
            "{{\"type\":\"subscribe\",\"venue\":\"{}\",\"symbol\":\"{}\",\"depth_levels\":{}}}",
            escape_json(&symbol.venue),
            escape_json(&symbol.symbol),
            depth_levels
        );
        match &mut self.transport {
            RithmicTransport::Live(ws) => ws.send_text("rithmic", payload),
            RithmicTransport::Mock => Ok(()),
        }
    }

    fn send_unsubscribe_wire(&mut self, symbol: &SymbolId) -> AdapterResult<()> {
        let payload = format!(
            "{{\"type\":\"unsubscribe\",\"venue\":\"{}\",\"symbol\":\"{}\"}}",
            escape_json(&symbol.venue),
            escape_json(&symbol.symbol)
        );
        match &mut self.transport {
            RithmicTransport::Live(ws) => ws.send_text("rithmic", payload),
            RithmicTransport::Mock => Ok(()),
        }
    }

    fn replay_subscriptions(&mut self) -> AdapterResult<()> {
        let requested: Vec<(SymbolId, u16)> = self
            .requested_depth
            .iter()
            .map(|(symbol, depth)| (symbol.clone(), *depth))
            .collect();
        for (symbol, depth) in requested {
            self.send_subscribe_wire(&symbol, depth)?;
        }
        Ok(())
    }

    fn schedule_reconnect(&mut self) {
        self.reconnect_attempt = self.reconnect_attempt.saturating_add(1);
        let base_ms = 250u64;
        let max_ms = 5_000u64;
        let delay_ms = (base_ms.saturating_mul(1u64 << self.reconnect_attempt.min(5))).min(max_ms);
        self.next_reconnect_at = Some(Instant::now() + Duration::from_millis(delay_ms));
    }

    fn reconnect_if_due(&mut self) -> AdapterResult<()> {
        if self.is_mock_mode() {
            return Ok(());
        }
        let due = self
            .next_reconnect_at
            .map(|t| Instant::now() >= t)
            .unwrap_or(false);
        if !due {
            return Ok(());
        }

        match &mut self.transport {
            RithmicTransport::Live(ws) => ws.connect("rithmic")?,
            RithmicTransport::Mock => return Ok(()),
        }
        self.connected = true;
        self.bootstrap_live_session()?;
        self.replay_subscriptions()?;
        self.next_reconnect_at = None;
        self.last_message_at = Some(Instant::now());
        self.last_heartbeat_at = Some(Instant::now());
        self.healthy_since = None;
        self.degraded = true;
        self.last_error = Some("rithmic reconnect warming".to_string());
        Ok(())
    }

    fn maybe_send_heartbeat_probe(&mut self) {
        if self.is_mock_mode() || !self.connected {
            return;
        }
        let now = Instant::now();
        let should_ping = self
            .last_message_at
            .map(|t| now.duration_since(t) >= Duration::from_secs(5))
            .unwrap_or(true);
        if should_ping {
            let payload = "{\"type\":\"heartbeat_probe\"}".to_string();
            if let RithmicTransport::Live(ws) = &mut self.transport {
                let _ = ws.send_text("rithmic", payload);
            }
        }
    }

    fn check_heartbeat_timeout(&mut self) {
        if self.is_mock_mode() || !self.connected {
            return;
        }
        let now = Instant::now();
        let heartbeat = self
            .last_heartbeat_at
            .or(self.last_message_at)
            .unwrap_or(now);
        if now.duration_since(heartbeat) > Duration::from_secs(15) {
            self.connected = false;
            self.degraded = true;
            self.healthy_since = None;
            self.last_error = Some("rithmic heartbeat timeout".to_string());
            if self.next_reconnect_at.is_none() {
                self.schedule_reconnect();
            }
        }
    }

    fn maybe_clear_degraded(&mut self) {
        if !self.degraded || !self.connected {
            return;
        }
        let now = Instant::now();
        let since = self.healthy_since.get_or_insert(now);
        if now.duration_since(*since) >= Duration::from_secs(2) {
            self.degraded = false;
            self.last_error = None;
        }
    }

    fn parse_live_message(&mut self, msg: &str) {
        self.last_message_at = Some(Instant::now());

        match extract_string_field(msg, "type") {
            Some("heartbeat") | Some("heartbeat_ack") | Some("subscribed") => {
                self.last_heartbeat_at = Some(Instant::now());
                self.healthy_since.get_or_insert_with(Instant::now);
            }
            Some("error") => {
                self.degraded = true;
                self.healthy_since = None;
                self.last_error = Some(
                    extract_string_field(msg, "message")
                        .unwrap_or("rithmic live error")
                        .to_string(),
                );
                if extract_bool_field(msg, "reconnect").unwrap_or(false) {
                    self.connected = false;
                    if self.next_reconnect_at.is_none() {
                        self.schedule_reconnect();
                    }
                }
            }
            Some("book") => {
                let sequence =
                    extract_u64_field(msg, "sequence").unwrap_or_else(|| self.next_sequence());
                let ts_exchange_ns =
                    extract_u64_field(msg, "ts_exchange_ns").unwrap_or_else(Self::now_ns);
                let ts_recv_ns = extract_u64_field(msg, "ts_recv_ns").unwrap_or_else(Self::now_ns);
                let symbol = SymbolId {
                    venue: extract_string_field(msg, "venue")
                        .unwrap_or("RITHMIC")
                        .to_string(),
                    symbol: match extract_string_field(msg, "symbol") {
                        Some(v) => v.to_string(),
                        None => return,
                    },
                };
                let side = match extract_string_field(msg, "side").unwrap_or("bid") {
                    "ask" | "ASK" => Side::Ask,
                    _ => Side::Bid,
                };
                let action = match extract_string_field(msg, "action").unwrap_or("upsert") {
                    "delete" | "DELETE" => BookAction::Delete,
                    _ => BookAction::Upsert,
                };
                self.queue.push_back(RawEvent::Book(BookUpdate {
                    symbol,
                    side,
                    level: extract_u16_field(msg, "level").unwrap_or(0),
                    price: match extract_i64_field(msg, "price") {
                        Some(v) => v,
                        None => return,
                    },
                    size: extract_i64_field(msg, "size").unwrap_or(0),
                    action,
                    sequence,
                    ts_exchange_ns,
                    ts_recv_ns,
                }));
                self.last_heartbeat_at = Some(Instant::now());
                self.healthy_since.get_or_insert_with(Instant::now);
            }
            Some("trade") => {
                let sequence =
                    extract_u64_field(msg, "sequence").unwrap_or_else(|| self.next_sequence());
                let ts_exchange_ns =
                    extract_u64_field(msg, "ts_exchange_ns").unwrap_or_else(Self::now_ns);
                let ts_recv_ns = extract_u64_field(msg, "ts_recv_ns").unwrap_or_else(Self::now_ns);
                let symbol = SymbolId {
                    venue: extract_string_field(msg, "venue")
                        .unwrap_or("RITHMIC")
                        .to_string(),
                    symbol: match extract_string_field(msg, "symbol") {
                        Some(v) => v.to_string(),
                        None => return,
                    },
                };
                let aggressor_side =
                    match extract_string_field(msg, "aggressor_side").unwrap_or("bid") {
                        "ask" | "ASK" => Side::Ask,
                        _ => Side::Bid,
                    };
                self.queue.push_back(RawEvent::Trade(TradePrint {
                    symbol,
                    price: match extract_i64_field(msg, "price") {
                        Some(v) => v,
                        None => return,
                    },
                    size: extract_i64_field(msg, "size").unwrap_or(0),
                    aggressor_side,
                    sequence,
                    ts_exchange_ns,
                    ts_recv_ns,
                }));
                self.last_heartbeat_at = Some(Instant::now());
                self.healthy_since.get_or_insert_with(Instant::now);
            }
            _ => {}
        }
    }
}

impl MarketDataAdapter for RithmicAdapter {
    fn connect(&mut self) -> AdapterResult<()> {
        let _ = &self.cfg.pass;
        self.degraded = false;
        self.last_error = None;
        self.next_reconnect_at = None;
        self.reconnect_attempt = 0;
        let mut needs_bootstrap = false;
        match &mut self.transport {
            RithmicTransport::Mock => {
                self.connected = true;
            }
            RithmicTransport::Live(ws) => {
                if let Err(err) = ws.connect("rithmic") {
                    self.connected = false;
                    self.degraded = true;
                    self.last_error = Some(err.to_string());
                    return Err(err);
                }
                self.connected = true;
                needs_bootstrap = true;
            }
        }
        if needs_bootstrap {
            if let Err(err) = self.bootstrap_live_session() {
                self.connected = false;
                self.degraded = true;
                self.last_error = Some(err.to_string());
                return Err(err);
            }
        }
        self.connected_at = Some(Instant::now());
        self.last_message_at = Some(Instant::now());
        self.last_heartbeat_at = Some(Instant::now());
        self.healthy_since = None;
        Ok(())
    }

    fn subscribe(&mut self, req: SubscribeReq) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        if req.depth_levels == 0 {
            self.requested_depth.remove(&req.symbol);
            return Ok(());
        }

        self.requested_depth
            .insert(req.symbol.clone(), req.depth_levels);
        if self.is_mock_mode() {
            self.synth_book_and_trade(&req.symbol, req.depth_levels);
        } else {
            self.send_subscribe_wire(&req.symbol, req.depth_levels)?;
        }
        Ok(())
    }

    fn unsubscribe(&mut self, symbol: SymbolId) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        self.requested_depth.remove(&symbol);
        if !self.is_mock_mode() {
            self.send_unsubscribe_wire(&symbol)?;
        }
        Ok(())
    }

    fn poll(&mut self, out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
        if !self.connected {
            self.reconnect_if_due()?;
            if !self.connected {
                return Err(AdapterError::Disconnected);
            }
        }
        self.last_poll_at = Some(Instant::now());

        if self.is_mock_mode() {
            let symbols: Vec<(SymbolId, u16)> = self
                .requested_depth
                .iter()
                .map(|(symbol, depth)| (symbol.clone(), *depth))
                .collect();
            for (symbol, depth) in symbols {
                self.synth_book_and_trade(&symbol, depth);
            }
        } else {
            self.maybe_send_heartbeat_probe();
            self.check_heartbeat_timeout();
            if !self.connected {
                self.reconnect_if_due()?;
                if !self.connected {
                    return Err(AdapterError::Disconnected);
                }
            }
            let mut inbound = Vec::new();
            if let RithmicTransport::Live(ws) = &mut self.transport {
                loop {
                    match ws.recv_text() {
                        Ok(Some(msg)) => inbound.push(msg),
                        Ok(None) => break,
                        Err(err) => {
                            self.connected = false;
                            self.degraded = true;
                            self.healthy_since = None;
                            self.last_error = Some(err.to_string());
                            if self.next_reconnect_at.is_none() {
                                self.schedule_reconnect();
                            }
                            return Err(err);
                        }
                    }
                }
            }
            for msg in inbound {
                self.parse_live_message(&msg);
            }
            self.maybe_clear_degraded();
        }

        let n = self.queue.len();
        out.extend(self.queue.drain(..));
        Ok(n)
    }

    fn health(&self) -> AdapterHealth {
        let mode = if self.is_mock_mode() {
            "mock"
        } else {
            "live_ws"
        };
        let connected = self.connected
            && match &self.transport {
                RithmicTransport::Mock => true,
                RithmicTransport::Live(ws) => ws.is_connected(),
            };
        let uptime_ms = self
            .connected_at
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let last_message_age_ms = self
            .last_message_at
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let last_heartbeat_age_ms = self
            .last_heartbeat_at
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let endpoint =
            redact_adapter_endpoint(&self.cfg.endpoint).unwrap_or_else(|| "<redacted>".to_string());
        AdapterHealth {
            connected,
            degraded: self.degraded,
            last_error: self.last_error.clone(),
            protocol_info: Some(format!(
                "provider=rithmic;wire=ws_probe_v1;mode={mode};endpoint={};app_name={};uptime_ms={uptime_ms};reconnect_attempt={};subscribed={};last_message_age_ms={last_message_age_ms};last_heartbeat_age_ms={last_heartbeat_age_ms}",
                endpoint,
                self.cfg.app_name,
                self.reconnect_attempt,
                self.requested_depth.len()
            )),
        }
    }

    fn operational_status(&self) -> AdapterOperationalStatus {
        let connected = self.connected
            && match &self.transport {
                RithmicTransport::Mock => true,
                RithmicTransport::Live(ws) => ws.is_connected(),
            };
        let state = if connected && !self.degraded {
            AdapterConnectionState::Streaming
        } else if connected {
            AdapterConnectionState::Reconnecting
        } else if self.next_reconnect_at.is_some() {
            AdapterConnectionState::Backoff
        } else {
            AdapterConnectionState::Disconnected
        };
        let mode = if self.is_mock_mode() {
            AdapterRuntimeMode::Mock
        } else {
            AdapterRuntimeMode::Live
        };
        let last_message_age_ms = self
            .last_message_at
            .map(|at| at.elapsed().as_millis() as u64);
        let last_heartbeat_age_ms = self
            .last_heartbeat_at
            .map(|at| at.elapsed().as_millis() as u64);
        let stale = !self.is_mock_mode()
            && connected
            && !self.requested_depth.is_empty()
            && last_heartbeat_age_ms
                .or(last_message_age_ms)
                .is_some_and(|age| age > 15_000);

        AdapterOperationalStatus::new(mode, state)
            .with_endpoint(Some(&self.cfg.endpoint))
            .with_app_name(Some(&self.cfg.app_name))
            .with_reconnect_attempt(self.reconnect_attempt)
            .with_subscribed_symbols(self.requested_depth.keys().cloned())
            .with_queue(self.queue.len(), None)
            .with_stale(stale)
            .with_activity_ages(last_message_age_ms, last_heartbeat_age_ms)
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

fn extract_field_value<'a>(raw: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\"");
    let key_pos = raw.find(&pat)?;
    let after_key = &raw[key_pos + pat.len()..];
    let colon = after_key.find(':')?;
    let mut v = after_key[colon + 1..].trim_start();
    if let Some(stripped) = v.strip_prefix('"') {
        let end = stripped.find('"')?;
        return Some(&stripped[..end]);
    }
    let end = v
        .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
        .unwrap_or(v.len());
    v = &v[..end];
    Some(v.trim())
}

fn extract_string_field<'a>(raw: &'a str, key: &str) -> Option<&'a str> {
    extract_field_value(raw, key)
}

fn extract_u64_field(raw: &str, key: &str) -> Option<u64> {
    extract_field_value(raw, key)?.parse::<u64>().ok()
}

fn extract_i64_field(raw: &str, key: &str) -> Option<i64> {
    extract_field_value(raw, key)?.parse::<i64>().ok()
}

fn extract_u16_field(raw: &str, key: &str) -> Option<u16> {
    extract_field_value(raw, key)?.parse::<u16>().ok()
}

fn extract_bool_field(raw: &str, key: &str) -> Option<bool> {
    match extract_field_value(raw, key)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AdapterConfig, CredentialsRef, ProviderKind};

    fn cfg(endpoint: &str) -> AdapterConfig {
        std::env::set_var("RITHMIC_TEST_USER", "demo_user");
        std::env::set_var("RITHMIC_TEST_PASS", "demo_pass");
        AdapterConfig {
            provider: ProviderKind::Rithmic,
            credentials: Some(CredentialsRef {
                key_id_env: "RITHMIC_TEST_USER".to_string(),
                secret_env: "RITHMIC_TEST_PASS".to_string(),
            }),
            endpoint: Some(endpoint.to_string()),
            app_name: Some("orderflow-tests".to_string()),
        }
    }

    #[test]
    fn connects_subscribes_and_polls_mock_flow() {
        let mut adapter = RithmicAdapter::from_config(&cfg("mock://rithmic")).expect("cfg");
        adapter.connect().expect("connect");
        adapter
            .subscribe(SubscribeReq {
                symbol: SymbolId {
                    venue: "CME".to_string(),
                    symbol: "ESM6".to_string(),
                },
                depth_levels: 10,
            })
            .expect("sub");
        let mut out = Vec::new();
        let n = adapter.poll(&mut out).expect("poll");
        assert!(n > 0);
        assert!(out.iter().any(|ev| matches!(ev, RawEvent::Book(_))));
        assert!(out.iter().any(|ev| matches!(ev, RawEvent::Trade(_))));
    }

    #[test]
    fn explicit_unsubscribe_removes_symbol() {
        let mut adapter = RithmicAdapter::from_config(&cfg("mock://rithmic")).expect("cfg");
        adapter.connect().expect("connect");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "NQM6".to_string(),
        };
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 10,
            })
            .expect("sub");
        adapter.unsubscribe(symbol.clone()).expect("unsub");
        assert!(!adapter.requested_depth.contains_key(&symbol));
    }

    #[test]
    fn zero_depth_aliases_unsubscribe() {
        let mut adapter = RithmicAdapter::from_config(&cfg("mock://rithmic")).expect("cfg");
        adapter.connect().expect("connect");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "RTYM6".to_string(),
        };
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 10,
            })
            .expect("sub");
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 0,
            })
            .expect("zero");
        assert!(!adapter.requested_depth.contains_key(&symbol));
    }

    #[test]
    fn health_reports_mode_and_endpoint() {
        let mut adapter = RithmicAdapter::from_config(&cfg("mock://rithmic")).expect("cfg");
        adapter.connect().expect("connect");
        let health = adapter.health();
        assert!(health.connected);
        let protocol_info = health.protocol_info.expect("protocol info");
        assert!(protocol_info.contains("provider=rithmic"));
        assert!(protocol_info.contains("mode=mock"));
        assert!(protocol_info.contains("endpoint=mock://rithmic"));
    }

    #[test]
    fn health_and_operational_status_redact_endpoint_secrets() {
        let endpoint = "wss://user:password@example.test/private/path?token=secret";
        let adapter = RithmicAdapter::from_config(&cfg(endpoint)).expect("cfg");
        let health = adapter.health().protocol_info.expect("protocol info");
        let status = adapter.operational_status();

        assert!(health.contains("endpoint=wss://example.test"));
        assert!(!health.contains("password"));
        assert!(!health.contains("token"));
        assert_eq!(
            status.endpoint_redacted.as_deref(),
            Some("wss://example.test")
        );
        assert_eq!(status.mode, AdapterRuntimeMode::Live);
        assert_eq!(status.app_name.as_deref(), Some("orderflow-tests"));
    }

    #[test]
    fn live_connect_attempt_returns_error_for_unreachable_endpoint() {
        let mut adapter =
            RithmicAdapter::from_config(&cfg("ws://127.0.0.1:1/rithmic")).expect("cfg");
        let err = adapter.connect().expect_err("connect should fail");
        assert!(format!("{err}").contains("connect failed"));
        let health = adapter.health();
        assert!(!health.connected);
        assert!(health.degraded);
        assert!(health.last_error.is_some());
    }

    #[test]
    fn live_mode_parses_normalized_trade_and_book_messages() {
        let mut adapter = RithmicAdapter::from_config(&cfg("ws://test.live/rithmic")).expect("cfg");
        adapter.connect().expect("connect");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 5,
            })
            .expect("sub");

        if let RithmicTransport::Live(ws) = &mut adapter.transport {
            ws.inject_text(r#"{"type":"heartbeat"}"#);
            ws.inject_text(r#"{"type":"book","venue":"CME","symbol":"ESM6","side":"bid","level":0,"price":505000,"size":8,"action":"upsert","sequence":77,"ts_exchange_ns":10,"ts_recv_ns":11}"#);
            ws.inject_text(r#"{"type":"trade","venue":"CME","symbol":"ESM6","price":505025,"size":3,"aggressor_side":"ask","sequence":78,"ts_exchange_ns":12,"ts_recv_ns":13}"#);
        }

        let mut out = Vec::new();
        let n = adapter.poll(&mut out).expect("poll");
        assert_eq!(n, 2);
        assert!(out.iter().any(|ev| matches!(ev, RawEvent::Book(_))));
        assert!(out.iter().any(|ev| matches!(ev, RawEvent::Trade(_))));
        assert!(adapter
            .health()
            .protocol_info
            .unwrap_or_default()
            .contains("subscribed=1"));
    }

    #[test]
    fn heartbeat_timeout_marks_live_path_degraded() {
        let mut adapter = RithmicAdapter::from_config(&cfg("ws://test.live/rithmic")).expect("cfg");
        adapter.connect().expect("connect");
        adapter.last_heartbeat_at = Some(Instant::now() - Duration::from_secs(20));
        adapter.last_message_at = adapter.last_heartbeat_at;

        let mut out = Vec::new();
        let err = adapter
            .poll(&mut out)
            .expect_err("timeout should disconnect");
        assert!(matches!(err, AdapterError::Disconnected));
        assert!(adapter.health().degraded);
        assert!(adapter.next_reconnect_at.is_some());
    }

    #[test]
    fn live_disconnect_schedules_and_recovers_with_reconnect() {
        let mut adapter = RithmicAdapter::from_config(&cfg("ws://test.live/rithmic")).expect("cfg");
        adapter.connect().expect("connect");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "NQM6".to_string(),
        };
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 5,
            })
            .expect("sub");

        if let RithmicTransport::Live(ws) = &mut adapter.transport {
            ws.force_disconnect();
        }
        let mut out = Vec::new();
        let err = adapter
            .poll(&mut out)
            .expect_err("disconnect should surface");
        assert!(matches!(err, AdapterError::Disconnected));
        assert!(adapter.next_reconnect_at.is_some());

        adapter.next_reconnect_at = Some(Instant::now());
        adapter.poll(&mut out).expect("reconnect poll");
        assert!(adapter.health().connected);
        assert!(adapter.health().degraded);

        if let RithmicTransport::Live(ws) = &mut adapter.transport {
            ws.inject_text(r#"{"type":"heartbeat"}"#);
            ws.inject_text(r#"{"type":"trade","venue":"CME","symbol":"NQM6","price":1780025,"size":2,"aggressor_side":"bid","sequence":91,"ts_exchange_ns":20,"ts_recv_ns":21}"#);
        }
        let mut recovered = Vec::new();
        let _ = adapter.poll(&mut recovered).expect("post reconnect poll");
        assert!(recovered.iter().any(|ev| matches!(ev, RawEvent::Trade(_))));
    }
}
