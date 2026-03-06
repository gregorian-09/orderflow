use std::collections::HashMap;

use of_core::SymbolId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CqgSessionState {
    Disconnected,
    Connecting,
    LogonPending,
    ResolvingSymbols,
    Subscribing,
    Streaming,
    Degraded,
    BackoffWait,
}

#[derive(Debug)]
pub struct CqgSession {
    state: CqgSessionState,
    next_request_id: u64,
    pub symbol_to_contract: HashMap<SymbolId, i64>,
    pending_symbol_resolution: HashMap<u64, SymbolId>,
    pending_subscription_ack: HashMap<u64, (SymbolId, i64)>,
    pub requested_depth: HashMap<SymbolId, u16>,
}

impl CqgSession {
    pub fn new() -> Self {
        Self {
            state: CqgSessionState::Disconnected,
            next_request_id: 1,
            symbol_to_contract: HashMap::new(),
            pending_symbol_resolution: HashMap::new(),
            pending_subscription_ack: HashMap::new(),
            requested_depth: HashMap::new(),
        }
    }

    pub fn state(&self) -> CqgSessionState {
        self.state
    }

    pub fn set_state(&mut self, state: CqgSessionState) {
        self.state = state;
    }

    pub fn next_request_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        id
    }

    pub fn queue_symbol_resolution(&mut self, symbol: SymbolId, depth: u16) -> u64 {
        let req_id = self.next_request_id();
        self.pending_symbol_resolution.insert(req_id, symbol.clone());
        self.requested_depth.insert(symbol, depth);
        req_id
    }

    pub fn on_symbol_resolved(&mut self, request_id: u64, contract_id: i64) -> Option<(SymbolId, u16)> {
        let symbol = self.pending_symbol_resolution.remove(&request_id)?;
        let depth = self.requested_depth.get(&symbol).copied().unwrap_or(1);
        self.symbol_to_contract.insert(symbol.clone(), contract_id);
        Some((symbol, depth))
    }

    pub fn queue_subscription_ack(&mut self, request_id: u64, symbol: SymbolId, contract_id: i64) {
        self.pending_subscription_ack
            .insert(request_id, (symbol, contract_id));
    }

    pub fn on_subscription_ack(&mut self, request_id: u64) -> Option<(SymbolId, i64)> {
        self.pending_subscription_ack.remove(&request_id)
    }

    pub fn has_pending_work(&self) -> bool {
        !self.pending_symbol_resolution.is_empty() || !self.pending_subscription_ack.is_empty()
    }

    pub fn clear_transient(&mut self) {
        self.pending_symbol_resolution.clear();
        self.pending_subscription_ack.clear();
    }

    pub fn upsert_requested_depth(&mut self, symbol: SymbolId, depth: u16) {
        self.requested_depth.insert(symbol, depth);
    }

    pub fn remove_symbol(&mut self, symbol: &SymbolId) {
        self.requested_depth.remove(symbol);
        self.symbol_to_contract.remove(symbol);
        self.pending_symbol_resolution.retain(|_, s| s != symbol);
        self.pending_subscription_ack
            .retain(|_, (s, _)| s != symbol);
    }
}
