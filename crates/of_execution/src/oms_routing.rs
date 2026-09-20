use super::*;

/// Route sharding key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShardKey {
    /// Route id.
    pub route_id: RouteId,
    /// Account id.
    pub account_id: AccountId,
    /// Symbol.
    pub symbol: ExecutionSymbol,
}

impl From<RouteKey> for ShardKey {
    fn from(value: RouteKey) -> Self {
        Self {
            route_id: value.route_id,
            account_id: value.account_id,
            symbol: value.symbol,
        }
    }
}

/// Deterministic sharding helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardRouter {
    /// Number of configured shards.
    pub shard_count: usize,
}

impl ShardRouter {
    /// Creates a sharding helper.
    pub const fn new(shard_count: usize) -> Self {
        Self { shard_count }
    }

    /// Returns the shard index for `key`.
    pub fn shard_for(&self, key: ShardKey) -> usize {
        if self.shard_count == 0 {
            return 0;
        }
        let mut hasher = StableHasher::default();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shard_count
    }
}

#[derive(Debug, Default)]
pub(crate) struct StableHasher(u64);

impl Hasher for StableHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Token-bucket style order throttler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderThrottle {
    pub(crate) capacity: u32,
    pub(crate) refill_per_sec: u32,
    pub(crate) tokens: u32,
    pub(crate) last_refill_ns: u64,
}

impl OrderThrottle {
    /// Creates a throttler.
    pub const fn new(capacity: u32, refill_per_sec: u32) -> Self {
        Self {
            capacity,
            refill_per_sec,
            tokens: capacity,
            last_refill_ns: 0,
        }
    }

    /// Attempts to consume one token at `now_ns`.
    pub fn allow(&mut self, now_ns: u64) -> bool {
        self.refill(now_ns);
        if self.tokens == 0 {
            return false;
        }
        self.tokens -= 1;
        true
    }

    /// Returns currently available tokens.
    pub const fn tokens(&self) -> u32 {
        self.tokens
    }

    fn refill(&mut self, now_ns: u64) {
        if self.last_refill_ns == 0 {
            self.last_refill_ns = now_ns;
            return;
        }
        let elapsed_ns = now_ns.saturating_sub(self.last_refill_ns);
        let add = elapsed_ns.saturating_mul(u64::from(self.refill_per_sec)) / 1_000_000_000;
        if add > 0 {
            self.tokens = self.capacity.min(self.tokens.saturating_add(add as u32));
            self.last_refill_ns = now_ns;
        }
    }
}

/// Replay decision used by the OMS simulation harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayDecision {
    /// Decision timestamp.
    pub ts_recv_ns: u64,
    /// Command to execute.
    pub command: ExecutionCommand,
}

/// Replay result for deterministic OMS simulation.
#[derive(Debug, Clone)]
pub struct ReplayResult {
    /// Command reports in replay order.
    pub reports: Vec<ExecutionCommandReport>,
    /// Final execution metrics.
    pub metrics: ExecutionMetrics,
}

/// Runs a deterministic simulated OMS replay.
pub fn replay_simulated_oms(
    routes: Vec<RouteConfig>,
    decisions: &[ReplayDecision],
) -> ExecutionResult<ReplayResult> {
    let mut engine = ExecutionEngine::new(
        SimExecutionAdapter::default(),
        AllowAllRiskGate,
        InMemoryJournal::default(),
        routes,
    );
    engine.start()?;
    let mut reports = Vec::with_capacity(decisions.len());
    let mut events = ExecutionEventBuffer::with_capacity(64);
    for (idx, decision) in decisions.iter().enumerate() {
        events.clear();
        let kind = decision.command.kind();
        let result = match decision.command {
            ExecutionCommand::Submit(req) => engine.submit(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Cancel(req) => engine.cancel(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Amend(req) => engine.amend(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Poll => engine.poll(&mut events),
            ExecutionCommand::RecoverOpenOrders => engine.recover_open_orders(&mut events),
            ExecutionCommand::Stop => Ok(0),
        };
        reports.push(ExecutionCommandReport {
            sequence: (idx + 1) as u64,
            kind,
            result,
            events: events.clone(),
        });
    }
    let metrics = engine.metrics();
    Ok(ReplayResult { reports, metrics })
}

/// Provider adapter context supplied to convenience adapter builders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAdapterContext {
    /// Adapter name.
    pub name: String,
    /// Route configs handled by the adapter.
    pub routes: Vec<RouteConfig>,
    /// Lifecycle state.
    pub lifecycle: ExecutionLifecycleSnapshot,
}

/// Factory trait for provider-specific execution adapters.
pub trait ExecutionAdapterFactory {
    /// Adapter type produced by the factory.
    type Adapter: ExecutionAdapter;

    /// Builds an adapter for `context`.
    ///
    /// # Errors
    ///
    /// Returns an execution error when required provider configuration is
    /// missing or invalid.
    fn build(&self, context: &ProviderAdapterContext) -> ExecutionResult<Self::Adapter>;
}

/// Convenience SDK helpers for provider adapters.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProviderAdapterSdk;

impl ProviderAdapterSdk {
    /// Returns default simulated capabilities for adapter tests.
    pub const fn simulated_capabilities() -> ExecutionCapabilities {
        ExecutionCapabilities::simulated()
    }

    /// Validates route configs for a provider adapter.
    ///
    /// # Errors
    ///
    /// Returns an error when no routes are configured.
    pub fn validate_routes(routes: &[RouteConfig]) -> ExecutionResult<()> {
        if routes.is_empty() {
            return Err(ExecutionError::RouteNotFound);
        }
        Ok(())
    }
}

pub(crate) fn command_kind_u8(kind: JournalCommandKind) -> u8 {
    match kind {
        JournalCommandKind::Submit => 1,
        JournalCommandKind::Cancel => 2,
        JournalCommandKind::Amend => 3,
    }
}

pub(crate) fn command_kind_from_u8(value: u8) -> Option<JournalCommandKind> {
    match value {
        1 => Some(JournalCommandKind::Submit),
        2 => Some(JournalCommandKind::Cancel),
        3 => Some(JournalCommandKind::Amend),
        _ => None,
    }
}

pub(crate) fn event_to_journal_line(event: &ExecutionEvent) -> String {
    format!(
        "E|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        event.exec_type as u8,
        event.order_status as u8,
        event.client_order_id,
        event.orig_client_order_id,
        event.venue_order_id,
        event.execution_id,
        event.account_id,
        event.route_id,
        event.symbol.venue,
        event.symbol.instrument,
        event.last_qty.0,
        event.last_price.0,
        event.cumulative_qty.0,
        event.leaves_qty.0,
        event.average_price.0,
        event.ts_exchange_ns,
        event.ts_recv_ns,
        event.reason as u8,
        sanitize_field(event.text.as_str())
    )
}

pub(crate) fn parse_journal_line(line: &str) -> ExecutionResult<Option<JournalRecord>> {
    let parts: Vec<&str> = line.split('|').collect();
    match parts.first().copied() {
        Some("C") if parts.len() == 4 => {
            let kind = parts[1]
                .parse::<u8>()
                .ok()
                .and_then(command_kind_from_u8)
                .ok_or_else(|| ExecutionError::Journal("invalid command kind".to_string()))?;
            let client_order_id = ClientOrderId::new(parts[2])
                .map_err(|err| ExecutionError::Journal(err.to_string()))?;
            let ts_ns = parts[3]
                .parse::<u64>()
                .map_err(|err| ExecutionError::Journal(err.to_string()))?;
            Ok(Some(JournalRecord::Command {
                kind,
                client_order_id,
                ts_ns,
            }))
        }
        Some("E") if parts.len() == 20 => Ok(Some(JournalRecord::Event(Box::new(
            parse_event_parts(&parts)?,
        )))),
        Some(_) => Err(ExecutionError::Journal("invalid journal line".to_string())),
        None => Ok(None),
    }
}

pub(crate) fn parse_event_parts(parts: &[&str]) -> ExecutionResult<ExecutionEvent> {
    Ok(ExecutionEvent {
        exec_type: execution_type_from_u8(parse_u8(parts[1])?)?,
        order_status: order_status_from_u8(parse_u8(parts[2])?)?,
        client_order_id: fixed(parts[3])?,
        orig_client_order_id: fixed(parts[4])?,
        venue_order_id: fixed(parts[5])?,
        execution_id: fixed(parts[6])?,
        account_id: fixed(parts[7])?,
        route_id: fixed(parts[8])?,
        symbol: ExecutionSymbol {
            venue: fixed(parts[9])?,
            instrument: fixed(parts[10])?,
        },
        last_qty: OrderQty(parse_i64(parts[11])?),
        last_price: OrderPrice(parse_i64(parts[12])?),
        cumulative_qty: OrderQty(parse_i64(parts[13])?),
        leaves_qty: OrderQty(parse_i64(parts[14])?),
        average_price: OrderPrice(parse_i64(parts[15])?),
        ts_exchange_ns: parse_u64(parts[16])?,
        ts_recv_ns: parse_u64(parts[17])?,
        reason: risk_reason_from_u8(parse_u8(parts[18])?)?,
        text: fixed(parts[19])?,
    })
}

pub(crate) fn fixed<const N: usize>(value: &str) -> ExecutionResult<FixedAscii<N>> {
    FixedAscii::new(value).map_err(|err| ExecutionError::Journal(err.to_string()))
}

pub(crate) fn parse_u8(value: &str) -> ExecutionResult<u8> {
    value
        .parse::<u8>()
        .map_err(|err| ExecutionError::Journal(err.to_string()))
}

pub(crate) fn parse_i64(value: &str) -> ExecutionResult<i64> {
    value
        .parse::<i64>()
        .map_err(|err| ExecutionError::Journal(err.to_string()))
}

pub(crate) fn parse_u64(value: &str) -> ExecutionResult<u64> {
    value
        .parse::<u64>()
        .map_err(|err| ExecutionError::Journal(err.to_string()))
}

pub(crate) fn execution_type_from_u8(value: u8) -> ExecutionResult<ExecutionType> {
    match value {
        1 => Ok(ExecutionType::Ack),
        2 => Ok(ExecutionType::Reject),
        3 => Ok(ExecutionType::Trade),
        4 => Ok(ExecutionType::CancelPending),
        5 => Ok(ExecutionType::CancelAck),
        6 => Ok(ExecutionType::CancelReject),
        7 => Ok(ExecutionType::ReplacePending),
        8 => Ok(ExecutionType::ReplaceAck),
        9 => Ok(ExecutionType::ReplaceReject),
        10 => Ok(ExecutionType::Expire),
        11 => Ok(ExecutionType::Status),
        12 => Ok(ExecutionType::Restated),
        13 => Ok(ExecutionType::AdapterDegraded),
        _ => Err(ExecutionError::Journal(
            "invalid execution type".to_string(),
        )),
    }
}

pub(crate) fn order_status_from_u8(value: u8) -> ExecutionResult<OrderStatus> {
    match value {
        1 => Ok(OrderStatus::PendingNew),
        2 => Ok(OrderStatus::New),
        3 => Ok(OrderStatus::PartiallyFilled),
        4 => Ok(OrderStatus::Filled),
        5 => Ok(OrderStatus::PendingCancel),
        6 => Ok(OrderStatus::Cancelled),
        7 => Ok(OrderStatus::PendingReplace),
        8 => Ok(OrderStatus::Replaced),
        9 => Ok(OrderStatus::Rejected),
        10 => Ok(OrderStatus::Expired),
        11 => Ok(OrderStatus::Suspended),
        12 => Ok(OrderStatus::Unknown),
        _ => Err(ExecutionError::Journal("invalid order status".to_string())),
    }
}

pub(crate) fn risk_reason_from_u8(value: u8) -> ExecutionResult<RiskRejectReason> {
    match value {
        0 => Ok(RiskRejectReason::None),
        1 => Ok(RiskRejectReason::KillSwitch),
        2 => Ok(RiskRejectReason::AccountDisabled),
        3 => Ok(RiskRejectReason::RouteDisabled),
        4 => Ok(RiskRejectReason::SymbolDisabled),
        5 => Ok(RiskRejectReason::MaxOrderQty),
        6 => Ok(RiskRejectReason::MaxOrderNotional),
        7 => Ok(RiskRejectReason::MaxOpenOrders),
        8 => Ok(RiskRejectReason::MaxOpenNotional),
        9 => Ok(RiskRejectReason::PriceBand),
        10 => Ok(RiskRejectReason::DuplicateClientOrderId),
        11 => Ok(RiskRejectReason::UnsupportedOrderType),
        12 => Ok(RiskRejectReason::UnsupportedTimeInForce),
        _ => Err(ExecutionError::Journal("invalid risk reason".to_string())),
    }
}

pub(crate) fn sanitize_field(value: &str) -> String {
    value.replace('|', " ")
}

pub(crate) fn reject(reason: RiskRejectReason, text: &str) -> RiskDecision {
    let text = ExecutionText::new(text).unwrap_or_else(|_| ExecutionText::empty());
    RiskDecision::reject(reason, text)
}
