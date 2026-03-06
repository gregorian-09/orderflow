use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use of_core::{Side, SymbolId, TradePrint};

use crate::{
    AdapterConfig, AdapterError, AdapterHealth, AdapterResult, MarketDataAdapter, RawEvent,
    SubscribeReq,
};

#[derive(Debug, Clone)]
pub struct RithmicConfig {
    endpoint: String,
    user: String,
    pass: String,
    app_name: String,
}

impl RithmicConfig {
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
    let v =
        std::env::var(name).map_err(|_| AdapterError::NotConfigured("required rithmic env var missing"))?;
    if v.trim().is_empty() {
        return Err(AdapterError::NotConfigured("required rithmic env var empty"));
    }
    Ok(v)
}

#[derive(Debug)]
pub struct RithmicAdapter {
    cfg: RithmicConfig,
    connected: bool,
    degraded: bool,
    last_error: Option<String>,
    requested_depth: HashMap<SymbolId, u16>,
    queue: VecDeque<RawEvent>,
    sequence: u64,
    connected_at: Option<Instant>,
}

impl RithmicAdapter {
    pub fn from_config(cfg: &AdapterConfig) -> AdapterResult<Self> {
        let cfg = RithmicConfig::from_adapter_config(cfg)?;
        Ok(Self {
            cfg,
            connected: false,
            degraded: false,
            last_error: None,
            requested_depth: HashMap::new(),
            queue: VecDeque::new(),
            sequence: 0,
            connected_at: None,
        })
    }

    fn is_mock_mode(&self) -> bool {
        self.cfg.endpoint.starts_with("mock://")
    }

    fn synth_trade(&mut self, symbol: &SymbolId) {
        self.sequence = self.sequence.saturating_add(1);
        self.queue.push_back(RawEvent::Trade(TradePrint {
            symbol: symbol.clone(),
            price: 500_000 + (self.sequence % 16) as i64,
            size: 1 + (self.sequence % 4) as i64,
            aggressor_side: if self.sequence % 2 == 0 {
                Side::Ask
            } else {
                Side::Bid
            },
            sequence: self.sequence,
            ts_exchange_ns: self.sequence,
            ts_recv_ns: self.sequence,
        }));
    }
}

impl MarketDataAdapter for RithmicAdapter {
    fn connect(&mut self) -> AdapterResult<()> {
        let _ = (&self.cfg.user, &self.cfg.pass, &self.cfg.app_name);
        self.connected = true;
        self.degraded = false;
        self.last_error = None;
        self.connected_at = Some(Instant::now());
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
            self.synth_trade(&req.symbol);
        }
        Ok(())
    }

    fn unsubscribe(&mut self, symbol: SymbolId) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        self.requested_depth.remove(&symbol);
        Ok(())
    }

    fn poll(&mut self, out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        if self.is_mock_mode() {
            let symbols: Vec<SymbolId> = self.requested_depth.keys().cloned().collect();
            for symbol in symbols {
                self.synth_trade(&symbol);
            }
        }

        let n = self.queue.len();
        out.extend(self.queue.drain(..));
        Ok(n)
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth {
            connected: self.connected,
            degraded: self.degraded,
            last_error: self.last_error.clone(),
            protocol_info: Some("provider=rithmic;wire=scaffold_v1".to_string()),
        }
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
    fn connects_subscribes_and_polls() {
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
}
