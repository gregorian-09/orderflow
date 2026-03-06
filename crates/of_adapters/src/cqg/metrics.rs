#[derive(Debug, Clone, Default)]
pub struct CqgMetrics {
    pub ws_connect_attempts: u64,
    pub ws_connect_failures: u64,
    pub logon_success: u64,
    pub logon_reject: u64,
    pub symbol_resolve_success: u64,
    pub symbol_resolve_fail: u64,
    pub md_subscribe_success: u64,
    pub md_subscribe_fail: u64,
    pub md_subscribe_ack_mismatch: u64,
    pub decode_errors: u64,
    pub sequence_gaps: u64,
    pub reconnect_count: u64,
}
