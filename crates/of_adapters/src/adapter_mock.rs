use super::*;

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
