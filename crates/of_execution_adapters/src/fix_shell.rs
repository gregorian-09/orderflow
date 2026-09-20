use super::*;

/// FIX execution adapter shell.
#[derive(Debug, Clone)]
pub struct FixExecutionAdapter {
    config: FixSessionConfig,
    connected: bool,
    health_seq: u64,
}

impl FixExecutionAdapter {
    /// Creates a FIX adapter shell.
    pub const fn new(config: FixSessionConfig) -> Self {
        Self {
            config,
            connected: false,
            health_seq: 0,
        }
    }

    /// Returns FIX session config.
    pub const fn config(&self) -> FixSessionConfig {
        self.config
    }
}

impl ExecutionAdapter for FixExecutionAdapter {
    fn connect(&mut self) -> ExecutionResult<()> {
        self.connected = false;
        self.health_seq = self.health_seq.saturating_add(1);
        Err(ExecutionError::Adapter(
            "FIX transport is not configured".to_string(),
        ))
    }

    fn submit(
        &mut self,
        _req: &OrderRequest,
        _out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        Err(ExecutionError::Disconnected)
    }

    fn cancel(
        &mut self,
        _req: &CancelRequest,
        _out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        Err(ExecutionError::Disconnected)
    }

    fn amend(
        &mut self,
        _req: &AmendRequest,
        _out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        Err(ExecutionError::Disconnected)
    }

    fn poll(&mut self, _out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        Err(ExecutionError::Disconnected)
    }

    fn recover_open_orders(&mut self, _out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        Err(ExecutionError::Disconnected)
    }

    fn capabilities(&self) -> ExecutionCapabilities {
        ExecutionCapabilities {
            latency_class: LatencyClass::NativeFix,
            market: true,
            limit: true,
            stop: true,
            stop_limit: true,
            tif_day: true,
            tif_gtc: true,
            tif_ioc: true,
            tif_fok: true,
            tif_gtd: true,
            amend: true,
            native_client_order_id: true,
        }
    }

    fn health(&self) -> ExecutionHealth {
        ExecutionHealth {
            connected: self.connected,
            degraded: !self.connected,
            health_seq: self.health_seq,
            last_error: Some("FIX transport is not configured".to_string()),
            protocol_info: Some(format!(
                "{}:{}->{}",
                self.config.begin_string, self.config.sender_comp_id, self.config.target_comp_id
            )),
        }
    }
}
