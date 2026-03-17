mod book;
mod config;
mod mapper;
mod metrics;
mod proto;
mod session;
mod transport;

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use book::{BookSequencer, SequenceStatus};
pub use config::CqgConfig;
use mapper::map_inbound_to_raw;
use metrics::CqgMetrics;
use proto::{
    decode_inbound, encode_inbound_for_test, encode_outbound, pb_schema_version, wire_mode,
    CqgInbound, CqgOutbound,
};
use session::{CqgSession, CqgSessionState};
use transport::{CqgTransport, MockTransport, WsProtobufTransport};

use crate::{
    AdapterConfig, AdapterError, AdapterHealth, AdapterResult, MarketDataAdapter, RawEvent,
    SubscribeReq,
};

/// CQG adapter implementation with session/reconnect/heartbeat supervision.
pub struct CqgAdapter {
    cfg: CqgConfig,
    session: CqgSession,
    transport: Box<dyn CqgTransport>,
    sequencer: BookSequencer,
    metrics: CqgMetrics,
    queue: VecDeque<RawEvent>,
    degraded: bool,
    last_error: Option<String>,
    reconnect_attempt: u32,
    next_reconnect_at: Option<Instant>,
    last_heartbeat_at: Option<Instant>,
    last_ping_at: Option<Instant>,
    healthy_since: Option<Instant>,
}

impl CqgAdapter {
    /// Creates a CQG adapter and validates runtime-safe configuration.
    pub fn from_config(cfg: &AdapterConfig) -> AdapterResult<Self> {
        let cfg = CqgConfig::from_adapter_config(cfg)?;
        cfg.validate_runtime()?;

        let transport: Box<dyn CqgTransport> = if cfg.endpoint.starts_with("mock://") {
            Box::new(MockTransport::default())
        } else {
            Box::new(WsProtobufTransport::new(cfg.endpoint.clone()))
        };

        Ok(Self {
            cfg,
            session: CqgSession::new(),
            transport,
            sequencer: BookSequencer::default(),
            metrics: CqgMetrics::default(),
            queue: VecDeque::new(),
            degraded: false,
            last_error: None,
            reconnect_attempt: 0,
            next_reconnect_at: None,
            last_heartbeat_at: None,
            last_ping_at: None,
            healthy_since: None,
        })
    }

    fn process_inbound(&mut self, msg: CqgInbound) {
        match msg {
            CqgInbound::LogonResult { success, message } => {
                if success {
                    self.metrics.logon_success += 1;
                    self.healthy_since = None;
                    if self.session.requested_depth.is_empty() {
                        self.session.set_state(CqgSessionState::Streaming);
                        self.healthy_since = Some(Instant::now());
                    } else {
                        self.session.set_state(CqgSessionState::ResolvingSymbols);
                        self.replay_subscriptions();
                    }
                } else {
                    self.metrics.logon_reject += 1;
                    self.degraded = true;
                    self.last_error = Some(message);
                    self.session.set_state(CqgSessionState::Degraded);
                }
            }
            CqgInbound::SymbolResolution {
                request_id,
                contract_id,
                ..
            } => {
                if let Some((_symbol, depth)) = self.session.on_symbol_resolved(request_id, contract_id) {
                    self.metrics.symbol_resolve_success += 1;
                    self.session.set_state(CqgSessionState::Subscribing);
                    if self
                        .send_market_data_subscription(&_symbol, contract_id, depth)
                        .is_err()
                    {
                        self.metrics.md_subscribe_fail += 1;
                        self.degraded = true;
                        self.last_error = Some("cqg subscribe send failure".to_string());
                    }
                } else {
                    self.metrics.symbol_resolve_fail += 1;
                }
            }
            CqgInbound::SubscriptionAck {
                request_id,
                contract_id,
                accepted,
            } => {
                if let Some((_symbol, expected_contract)) = self.session.on_subscription_ack(request_id) {
                    if accepted {
                        if expected_contract != contract_id {
                            self.metrics.md_subscribe_ack_mismatch += 1;
                            self.degraded = true;
                            self.healthy_since = None;
                            self.last_error =
                                Some("cqg subscription ack contract mismatch".to_string());
                            self.session.set_state(CqgSessionState::Degraded);
                            return;
                        }
                        self.metrics.md_subscribe_success += 1;
                        if !self.session.has_pending_work() {
                            self.session.set_state(CqgSessionState::Streaming);
                            self.healthy_since = Some(Instant::now());
                        }
                    } else {
                        self.metrics.md_subscribe_fail += 1;
                        self.degraded = true;
                        self.last_error = Some("cqg subscription rejected".to_string());
                        self.session.set_state(CqgSessionState::Degraded);
                    }
                } else {
                    self.metrics.decode_errors += 1;
                }
            }
            CqgInbound::MarketDataIncremental {
                contract_id,
                sequence,
                price,
                size,
                level,
                is_bid,
                is_delete,
                ..
            } => {
                match self.sequencer.apply_sequence(contract_id, sequence) {
                    SequenceStatus::Ok => {}
                    SequenceStatus::OutOfOrder => {
                        self.degraded = true;
                        self.healthy_since = None;
                        self.last_error = Some("cqg out-of-order update".to_string());
                    }
                    SequenceStatus::Gap { .. } => {
                        self.metrics.sequence_gaps += 1;
                        self.degraded = true;
                        self.healthy_since = None;
                        self.last_error = Some("cqg sequence gap detected".to_string());
                        self.session.set_state(CqgSessionState::Degraded);
                    }
                }
                if let Some(symbol) = self.symbol_for_contract(contract_id) {
                    if let Some(event) = map_inbound_to_raw(
                        &symbol,
                        &CqgInbound::MarketDataIncremental {
                            contract_id,
                            sequence,
                            price,
                            size,
                            level,
                            is_bid,
                            is_delete,
                        },
                    ) {
                        self.queue.push_back(event);
                    }
                }
            }
            CqgInbound::TradeUpdate { contract_id, .. } => {
                if let Some(symbol) = self.symbol_for_contract(contract_id) {
                    if let Some(event) = map_inbound_to_raw(&symbol, &msg) {
                        self.queue.push_back(event);
                    }
                }
            }
            CqgInbound::Reject { reason, .. } => {
                self.degraded = true;
                self.healthy_since = None;
                self.last_error = Some(reason);
                self.session.set_state(CqgSessionState::Degraded);
            }
            CqgInbound::Heartbeat => {
                self.last_heartbeat_at = Some(Instant::now());
            }
        }
    }

    fn drain_transport(&mut self) {
        loop {
            match self.transport.recv_next_frame() {
                Ok(Some(frame)) => match decode_inbound(&frame) {
                    Ok(msg) => self.process_inbound(msg),
                    Err(err) => {
                        self.metrics.decode_errors += 1;
                        self.degraded = true;
                        self.last_error = Some(format!("decode error: {err}"));
                    }
                },
                Ok(None) => break,
                Err(err) => {
                    self.degraded = true;
                    self.last_error = Some(err.to_string());
                    self.session.set_state(CqgSessionState::BackoffWait);
                    self.schedule_reconnect();
                    break;
                }
            }
        }
    }

    fn schedule_reconnect(&mut self) {
        self.reconnect_attempt = self.reconnect_attempt.saturating_add(1);
        self.metrics.reconnect_count = self.metrics.reconnect_count.saturating_add(1);
        let exp = self.reconnect_attempt.min(8);
        let base = self.cfg.reconnect_min_ms.max(1);
        let max = self.cfg.reconnect_max_ms.max(base);
        let delay_ms = (base.saturating_mul(1u64 << exp)).min(max);
        self.next_reconnect_at = Some(Instant::now() + Duration::from_millis(delay_ms));
    }

    fn should_reconnect_now(&self) -> bool {
        matches!(
            self.session.state(),
            CqgSessionState::BackoffWait | CqgSessionState::Disconnected
        ) && self
            .next_reconnect_at
            .map(|t| Instant::now() >= t)
            .unwrap_or(true)
    }

    fn reconnect_if_due(&mut self) {
        if !self.should_reconnect_now() {
            return;
        }
        self.session.clear_transient();
        self.session.set_state(CqgSessionState::Connecting);
        if self.transport.connect().is_ok() && self.transport.is_connected() {
            self.session.set_state(CqgSessionState::LogonPending);
            let _ = self.transport.send_frame(encode_outbound(&CqgOutbound::Logon));
            if self.is_mock_mode() {
                self.transport
                    .inject_test_frame(encode_inbound_for_test(&CqgInbound::LogonResult {
                        success: true,
                        message: "reconnected".to_string(),
                    }));
            }
            self.drain_transport();
            self.reconnect_attempt = 0;
            self.next_reconnect_at = None;
            self.degraded = true;
            self.last_error = Some("cqg reconnect warming".to_string());
            self.healthy_since = None;
            self.last_heartbeat_at = Some(Instant::now());
            self.last_ping_at = None;
        } else {
            self.metrics.ws_connect_failures += 1;
            self.session.set_state(CqgSessionState::BackoffWait);
            self.schedule_reconnect();
        }
    }

    fn symbol_for_contract(&self, contract_id: i64) -> Option<of_core::SymbolId> {
        self.session
            .symbol_to_contract
            .iter()
            .find_map(|(symbol, cid)| if *cid == contract_id { Some(symbol.clone()) } else { None })
    }

    fn is_mock_mode(&self) -> bool {
        self.cfg.endpoint.starts_with("mock://")
    }

    fn replay_subscriptions(&mut self) {
        let requested: Vec<(of_core::SymbolId, u16)> = self
            .session
            .requested_depth
            .iter()
            .map(|(s, d)| (s.clone(), *d))
            .collect();

        for (symbol, depth) in requested {
            let req_id = self.session.queue_symbol_resolution(symbol.clone(), depth);
            let _ = self
                .transport
                .send_frame(encode_outbound(&CqgOutbound::InformationRequest {
                    request_id: req_id,
                    symbol: symbol.symbol.clone(),
                }));

            if self.is_mock_mode() {
                let contract_id = req_id as i64 + 10_000;
                self.transport
                    .inject_test_frame(encode_inbound_for_test(&CqgInbound::SymbolResolution {
                        request_id: req_id,
                        contract_id,
                        symbol: symbol.symbol.clone(),
                    }));
            }
        }
    }

    fn send_market_data_subscription(
        &mut self,
        symbol: &of_core::SymbolId,
        contract_id: i64,
        depth: u16,
    ) -> AdapterResult<()> {
        let sub_req_id = self.session.next_request_id();
        self.transport
            .send_frame(encode_outbound(&CqgOutbound::MarketDataSubscription {
                request_id: sub_req_id,
                contract_id,
                level: depth,
            }))?;
        self.session
            .queue_subscription_ack(sub_req_id, symbol.clone(), contract_id);
        if self.is_mock_mode() {
            self.transport
                .inject_test_frame(encode_inbound_for_test(&CqgInbound::SubscriptionAck {
                    request_id: sub_req_id,
                    contract_id,
                    accepted: true,
                }));
        }
        Ok(())
    }

    fn maybe_send_ping(&mut self) {
        let now = Instant::now();
        let should_ping = self
            .last_ping_at
            .map(|t| now.duration_since(t) >= Duration::from_secs(self.cfg.ping_interval_secs))
            .unwrap_or(true);
        if should_ping {
            let _ = self.transport.send_frame(encode_outbound(&CqgOutbound::Ping));
            self.last_ping_at = Some(now);
        }
    }

    fn check_heartbeat_timeout(&mut self) {
        if self.session.state() != CqgSessionState::Streaming {
            return;
        }
        let now = Instant::now();
        let hb = self.last_heartbeat_at.unwrap_or(now);
        if now.duration_since(hb) > Duration::from_secs(self.cfg.heartbeat_timeout_secs) {
            self.degraded = true;
            self.healthy_since = None;
            self.last_error = Some("cqg heartbeat timeout".to_string());
            self.session.set_state(CqgSessionState::BackoffWait);
            let _ = self.transport.send_frame(encode_outbound(&CqgOutbound::Logoff));
            self.transport.force_disconnect();
            if self.next_reconnect_at.is_none() {
                self.schedule_reconnect();
            }
        }
    }

    fn maybe_clear_degraded(&mut self) {
        if !self.degraded || self.session.state() != CqgSessionState::Streaming {
            return;
        }
        let now = Instant::now();
        let since = self.healthy_since.get_or_insert(now);
        if now.duration_since(*since) >= Duration::from_secs(3) {
            self.degraded = false;
            self.last_error = None;
        }
    }
}

impl std::fmt::Debug for CqgAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CqgAdapter")
            .field("cfg", &self.cfg)
            .field("session_state", &self.session.state())
            .field("metrics", &self.metrics)
            .field("degraded", &self.degraded)
            .field("last_error", &self.last_error)
            .finish()
    }
}

impl MarketDataAdapter for CqgAdapter {
    fn connect(&mut self) -> AdapterResult<()> {
        self.metrics.ws_connect_attempts += 1;
        self.session.set_state(CqgSessionState::Connecting);
        self.transport.connect()?;

        if !self.transport.is_connected() {
            self.metrics.ws_connect_failures += 1;
            self.last_error = Some("cqg transport failed to connect".to_string());
            return Err(AdapterError::Disconnected);
        }

        self.session.set_state(CqgSessionState::LogonPending);
        self.transport.send_frame(encode_outbound(&CqgOutbound::Logon))?;

        // Mock transport path seeds deterministic handshake frame.
        if self.is_mock_mode() {
            self.transport
                .inject_test_frame(encode_inbound_for_test(&CqgInbound::LogonResult {
                    success: true,
                    message: "ok".to_string(),
                }));
        }
        self.drain_transport();
        self.reconnect_attempt = 0;
        self.next_reconnect_at = None;
        self.last_heartbeat_at = Some(Instant::now());
        self.last_ping_at = None;
        self.healthy_since = None;

        Ok(())
    }

    fn subscribe(&mut self, req: SubscribeReq) -> AdapterResult<()> {
        if !self.transport.is_connected() {
            return Err(AdapterError::Disconnected);
        }

        if req.depth_levels == 0 {
            if let Some(contract_id) = self.session.symbol_to_contract.get(&req.symbol).copied() {
                self.session.set_state(CqgSessionState::Subscribing);
                self.send_market_data_subscription(&req.symbol, contract_id, 0)?;
            }
            self.session.remove_symbol(&req.symbol);
            self.drain_transport();
            if !self.session.has_pending_work() {
                self.session.set_state(CqgSessionState::Streaming);
            }
            return Ok(());
        }

        self.session
            .upsert_requested_depth(req.symbol.clone(), req.depth_levels);
        if let Some(contract_id) = self.session.symbol_to_contract.get(&req.symbol).copied() {
            self.session.set_state(CqgSessionState::Subscribing);
            self.send_market_data_subscription(&req.symbol, contract_id, req.depth_levels)?;
            self.drain_transport();
            return Ok(());
        }

        self.session.set_state(CqgSessionState::ResolvingSymbols);
        let req_id = self
            .session
            .queue_symbol_resolution(req.symbol.clone(), req.depth_levels);
        self.transport
            .send_frame(encode_outbound(&CqgOutbound::InformationRequest {
                request_id: req_id,
                symbol: req.symbol.symbol.clone(),
            }))?;

        if self.is_mock_mode() {
            let contract_id = req_id as i64 + 10_000;
            self.transport
                .inject_test_frame(encode_inbound_for_test(&CqgInbound::SymbolResolution {
                    request_id: req_id,
                    contract_id,
                    symbol: req.symbol.symbol.clone(),
                }));
            // Seed one synthetic trade event to keep end-to-end path non-empty in scaffold mode.
            self.transport
                .inject_test_frame(encode_inbound_for_test(&CqgInbound::TradeUpdate {
                    contract_id,
                    sequence: 1,
                    price: 500_000,
                    size: 1,
                    aggressor_is_buy: true,
                }));
        }
        self.drain_transport();

        Ok(())
    }

    fn unsubscribe(&mut self, symbol: of_core::SymbolId) -> AdapterResult<()> {
        self.subscribe(SubscribeReq {
            symbol,
            depth_levels: 0,
        })
    }

    fn poll(&mut self, out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
        if !self.transport.is_connected() {
            self.degraded = true;
            self.last_error = Some("transport disconnected".to_string());
            self.session.set_state(CqgSessionState::BackoffWait);
            if self.next_reconnect_at.is_none() {
                self.schedule_reconnect();
            }
            self.reconnect_if_due();
            if !self.transport.is_connected() {
                return Err(AdapterError::Disconnected);
            }
        }

        self.reconnect_if_due();
        self.maybe_send_ping();
        self.check_heartbeat_timeout();
        self.reconnect_if_due();
        self.drain_transport();
        self.maybe_clear_degraded();

        let n = self.queue.len();
        out.extend(self.queue.drain(..));
        Ok(n)
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth {
            connected: self.transport.is_connected(),
            degraded: self.degraded,
            last_error: self.last_error.clone(),
            protocol_info: Some(format!(
                "provider=cqg;wire={};cqg_pb_schema_version={}",
                wire_mode(),
                pb_schema_version()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AdapterConfig, CredentialsRef, ProviderKind};

    fn cfg(endpoint: &str) -> AdapterConfig {
        std::env::set_var("CQG_TEST_USER", "demo_user");
        std::env::set_var("CQG_TEST_PASS", "demo_pass");
        AdapterConfig {
            provider: ProviderKind::Cqg,
            credentials: Some(CredentialsRef {
                key_id_env: "CQG_TEST_USER".to_string(),
                secret_env: "CQG_TEST_PASS".to_string(),
            }),
            endpoint: Some(endpoint.to_string()),
            app_name: Some("orderflow-tests".to_string()),
        }
    }

    #[test]
    fn connects_and_subscribes() {
        let mut adapter = CqgAdapter::from_config(&cfg("mock://cqg")).expect("cfg valid");
        adapter.connect().expect("connect");

        adapter
            .subscribe(SubscribeReq {
                symbol: of_core::SymbolId {
                    venue: "CME".to_string(),
                    symbol: "ESM6".to_string(),
                },
                depth_levels: 10,
            })
            .expect("subscribe");

        let mut out = Vec::new();
        let n = adapter.poll(&mut out).expect("poll ok");
        assert!(n > 0);
        assert!(adapter.health().connected);
    }

    #[test]
    fn live_boundary_connects_with_wss_endpoint() {
        let mut adapter = CqgAdapter::from_config(&cfg("wss://demoapi.cqg.com:443"))
            .expect("cfg valid");
        adapter.connect().expect("connect");
        assert!(adapter.health().connected);
    }

    #[test]
    fn reconnects_after_disconnect() {
        let mut adapter = CqgAdapter::from_config(&cfg("mock://cqg")).expect("cfg valid");
        adapter.connect().expect("connect");
        adapter
            .subscribe(SubscribeReq {
                symbol: of_core::SymbolId {
                    venue: "CME".to_string(),
                    symbol: "ESM6".to_string(),
                },
                depth_levels: 10,
            })
            .expect("subscribe");
        adapter.transport.force_disconnect();
        adapter.next_reconnect_at = Some(Instant::now());

        let mut out = Vec::new();
        let _ = adapter.poll(&mut out);
        assert!(adapter.health().connected);
        assert_eq!(adapter.session.state(), CqgSessionState::Streaming);
    }

    #[test]
    fn subscription_ack_drives_streaming() {
        let mut adapter = CqgAdapter::from_config(&cfg("mock://cqg")).expect("cfg valid");
        adapter.connect().expect("connect");
        adapter
            .subscribe(SubscribeReq {
                symbol: of_core::SymbolId {
                    venue: "CME".to_string(),
                    symbol: "NQM6".to_string(),
                },
                depth_levels: 10,
            })
            .expect("subscribe");
        let mut out = Vec::new();
        let _ = adapter.poll(&mut out).expect("poll");
        assert_eq!(adapter.session.state(), CqgSessionState::Streaming);
    }

    #[test]
    fn heartbeat_timeout_transitions_to_backoff() {
        let mut adapter = CqgAdapter::from_config(&cfg("mock://cqg")).expect("cfg valid");
        adapter.connect().expect("connect");
        adapter.last_heartbeat_at = Some(
            Instant::now() - Duration::from_secs(adapter.cfg.heartbeat_timeout_secs + 1),
        );

        let mut out = Vec::new();
        let _ = adapter.poll(&mut out);
        assert!(adapter.health().degraded);
    }

    #[test]
    fn health_exposes_wire_marker() {
        let adapter = CqgAdapter::from_config(&cfg("mock://cqg")).expect("cfg valid");
        let info = adapter
            .health()
            .protocol_info
            .expect("protocol info should be set");
        assert!(info.contains("provider=cqg"));
        assert!(info.contains("cqg_pb_schema_version="));
    }

    #[test]
    fn level_change_reuses_existing_contract() {
        let symbol = of_core::SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut adapter = CqgAdapter::from_config(&cfg("mock://cqg")).expect("cfg valid");
        adapter.connect().expect("connect");

        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 10,
            })
            .expect("subscribe 10");
        let resolved_once = adapter.metrics.symbol_resolve_success;

        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 5,
            })
            .expect("subscribe 5");

        assert_eq!(adapter.metrics.symbol_resolve_success, resolved_once);
        assert_eq!(adapter.session.requested_depth.get(&symbol).copied(), Some(5));
        assert!(adapter.metrics.md_subscribe_success >= 2);
    }

    #[test]
    fn unsubscribe_depth_zero_removes_symbol_state() {
        let symbol = of_core::SymbolId {
            venue: "CME".to_string(),
            symbol: "NQM6".to_string(),
        };
        let mut adapter = CqgAdapter::from_config(&cfg("mock://cqg")).expect("cfg valid");
        adapter.connect().expect("connect");
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 10,
            })
            .expect("subscribe 10");
        assert!(adapter.session.symbol_to_contract.contains_key(&symbol));

        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 0,
            })
            .expect("unsubscribe");

        assert!(!adapter.session.requested_depth.contains_key(&symbol));
        assert!(!adapter.session.symbol_to_contract.contains_key(&symbol));
    }

    #[test]
    fn ack_contract_mismatch_marks_degraded() {
        let symbol = of_core::SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };
        let mut adapter = CqgAdapter::from_config(&cfg("mock://cqg")).expect("cfg valid");
        adapter.connect().expect("connect");
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 10,
            })
            .expect("subscribe");

        let req_id = adapter.session.next_request_id();
        adapter
            .session
            .queue_subscription_ack(req_id, symbol, 12345);
        adapter
            .transport
            .inject_test_frame(encode_inbound_for_test(&CqgInbound::SubscriptionAck {
                request_id: req_id,
                contract_id: 99999,
                accepted: true,
            }));
        let mut out = Vec::new();
        let _ = adapter.poll(&mut out).expect("poll");
        assert!(adapter.health().degraded);
        assert_eq!(adapter.metrics.md_subscribe_ack_mismatch, 1);
    }

    #[test]
    fn reconnect_preserves_latest_depth_request() {
        let symbol = of_core::SymbolId {
            venue: "CME".to_string(),
            symbol: "NQM6".to_string(),
        };
        let mut adapter = CqgAdapter::from_config(&cfg("mock://cqg")).expect("cfg valid");
        adapter.connect().expect("connect");
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 10,
            })
            .expect("subscribe 10");
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 5,
            })
            .expect("subscribe 5");

        adapter.transport.force_disconnect();
        adapter.next_reconnect_at = Some(Instant::now());
        let mut out = Vec::new();
        let _ = adapter.poll(&mut out);

        assert_eq!(adapter.session.requested_depth.get(&symbol).copied(), Some(5));
        assert!(adapter.session.symbol_to_contract.contains_key(&symbol));
    }

    #[test]
    fn reconnect_does_not_restore_unsubscribed_symbol() {
        let symbol = of_core::SymbolId {
            venue: "CME".to_string(),
            symbol: "RTYM6".to_string(),
        };
        let mut adapter = CqgAdapter::from_config(&cfg("mock://cqg")).expect("cfg valid");
        adapter.connect().expect("connect");
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 10,
            })
            .expect("subscribe");
        adapter.unsubscribe(symbol.clone()).expect("unsubscribe");
        assert!(!adapter.session.requested_depth.contains_key(&symbol));

        adapter.transport.force_disconnect();
        adapter.next_reconnect_at = Some(Instant::now());
        let mut out = Vec::new();
        let _ = adapter.poll(&mut out);

        assert!(!adapter.session.requested_depth.contains_key(&symbol));
        assert!(!adapter.session.symbol_to_contract.contains_key(&symbol));
    }
}
