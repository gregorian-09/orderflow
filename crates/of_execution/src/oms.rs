//! Additive OMS building blocks for execution integrations.

use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

use of_execution_core::{
    AccountId, AmendRequest, CancelRequest, ClientOrderId, ExecutionEvent, ExecutionSymbol,
    ExecutionText, ExecutionType, FixedAscii, OrderPrice, OrderQty, OrderRequest, OrderSide,
    OrderState, OrderStatus, OrderType, RiskCheck, RiskContext, RiskDecision, RiskLimits,
    RiskRejectReason, RouteId, StrategyId, TimeInForce,
};

use crate::{
    AllowAllRiskGate, ExecutionAdapter, ExecutionCapabilities, ExecutionCommand,
    ExecutionCommandKind, ExecutionCommandReport, ExecutionEngine, ExecutionError,
    ExecutionEventBuffer, ExecutionJournal, ExecutionMetrics, ExecutionResult, InMemoryJournal,
    JournalCommandKind, JournalRecord, RouteConfig, RouteKey, SimExecutionAdapter,
};

/// Monotonic command identifier assigned before a command enters an OMS queue.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CommandId(pub u64);

/// Request identifier used to correlate strategy intent, command queue entry,
/// and downstream execution reports.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RequestId(pub FixedAscii<40>);

impl RequestId {
    /// Creates a request id from ASCII text.
    ///
    /// # Errors
    ///
    /// Returns an error when the id exceeds capacity or is not ASCII.
    pub fn new(value: &str) -> Result<Self, of_execution_core::ExecutionCoreError> {
        Ok(Self(FixedAscii::new(value)?))
    }

    /// Returns the request id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Lock-free monotonic command id generator.
#[derive(Debug, Default)]
pub struct CommandIdGenerator {
    next: AtomicU64,
}

impl CommandIdGenerator {
    /// Creates a generator starting at `first`.
    pub const fn new(first: u64) -> Self {
        Self {
            next: AtomicU64::new(first),
        }
    }

    /// Returns the next command id.
    pub fn next(&self) -> CommandId {
        CommandId(self.next.fetch_add(1, Ordering::Relaxed))
    }
}

/// Correlation envelope for an execution command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandCorrelation {
    /// Monotonic command id.
    pub command_id: CommandId,
    /// Optional strategy/request id.
    pub request_id: RequestId,
    /// Client order id associated with the command.
    pub client_order_id: ClientOrderId,
    /// Command kind.
    pub kind: ExecutionCommandKind,
}

impl CommandCorrelation {
    /// Creates a command correlation envelope.
    pub const fn new(
        command_id: CommandId,
        request_id: RequestId,
        client_order_id: ClientOrderId,
        kind: ExecutionCommandKind,
    ) -> Self {
        Self {
            command_id,
            request_id,
            client_order_id,
            kind,
        }
    }
}

/// Event subscriber for execution fanout.
#[derive(Debug)]
pub struct ExecutionEventSubscriber {
    receiver: Receiver<ExecutionEvent>,
}

impl ExecutionEventSubscriber {
    /// Receives the next execution event.
    ///
    /// # Errors
    ///
    /// Returns an error when the fanout source has been dropped.
    pub fn recv(&self) -> Result<ExecutionEvent, ExecutionError> {
        self.receiver
            .recv()
            .map_err(|_| ExecutionError::Adapter("execution event fanout closed".to_string()))
    }

    /// Attempts to receive one execution event without blocking.
    pub fn try_recv(&self) -> Option<ExecutionEvent> {
        self.receiver.try_recv().ok()
    }
}

#[derive(Debug)]
struct FanoutInner {
    subscribers: Vec<SyncSender<ExecutionEvent>>,
    dropped_events: u64,
}

/// Bounded execution-event fanout for multiple consumers.
#[derive(Debug, Clone)]
pub struct ExecutionEventFanout {
    inner: Arc<Mutex<FanoutInner>>,
    subscriber_capacity: usize,
}

impl ExecutionEventFanout {
    /// Creates an empty fanout with per-subscriber queue capacity.
    pub fn new(subscriber_capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(FanoutInner {
                subscribers: Vec::new(),
                dropped_events: 0,
            })),
            subscriber_capacity,
        }
    }

    /// Adds a subscriber.
    pub fn subscribe(&self) -> ExecutionEventSubscriber {
        let (tx, receiver) = mpsc::sync_channel(self.subscriber_capacity);
        let mut inner = self.inner.lock().expect("fanout mutex");
        inner.subscribers.push(tx);
        ExecutionEventSubscriber { receiver }
    }

    /// Publishes an event to all active subscribers.
    pub fn publish(&self, event: ExecutionEvent) {
        let mut inner = self.inner.lock().expect("fanout mutex");
        let mut dropped = 0_u64;
        inner
            .subscribers
            .retain(|subscriber| match subscriber.try_send(event) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) => {
                    dropped = dropped.saturating_add(1);
                    true
                }
                Err(TrySendError::Disconnected(_)) => false,
            });
        inner.dropped_events = inner.dropped_events.saturating_add(dropped);
    }

    /// Publishes all events in `events`.
    pub fn publish_buffer(&self, events: &ExecutionEventBuffer) {
        for event in events.as_slice() {
            self.publish(*event);
        }
    }

    /// Returns the number of event deliveries dropped because a subscriber
    /// queue was full.
    pub fn dropped_events(&self) -> u64 {
        self.inner.lock().expect("fanout mutex").dropped_events
    }

    /// Returns current active subscriber count.
    pub fn subscriber_count(&self) -> usize {
        self.inner.lock().expect("fanout mutex").subscribers.len()
    }
}

/// Venue adapter/session lifecycle state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExecutionAdapterState {
    /// Transport is disconnected.
    #[default]
    Disconnected = 0,
    /// Transport connect is in progress.
    Connecting = 1,
    /// Protocol logon/authentication is in progress.
    LogonPending = 2,
    /// Session is ready for order flow.
    Ready = 3,
    /// Session is recovering state after reconnect.
    Recovering = 4,
    /// Session is connected but degraded.
    Degraded = 5,
    /// Session is stopped intentionally.
    Stopped = 6,
}

/// Execution lifecycle snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecutionLifecycleSnapshot {
    /// Current adapter/session state.
    pub state: ExecutionAdapterState,
    /// Monotonic lifecycle sequence.
    pub sequence: u64,
    /// Last transition timestamp in nanoseconds.
    pub updated_ns: u64,
    /// Last lifecycle error.
    pub last_error: Option<String>,
}

/// Mutable lifecycle tracker for adapters and supervisors.
#[derive(Debug, Clone, Default)]
pub struct ExecutionLifecycle {
    snapshot: ExecutionLifecycleSnapshot,
}

impl ExecutionLifecycle {
    /// Creates a disconnected lifecycle tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Transitions to `state`.
    pub fn transition(
        &mut self,
        state: ExecutionAdapterState,
        updated_ns: u64,
        last_error: Option<String>,
    ) -> ExecutionLifecycleSnapshot {
        self.snapshot.state = state;
        self.snapshot.sequence = self.snapshot.sequence.saturating_add(1);
        self.snapshot.updated_ns = updated_ns;
        self.snapshot.last_error = last_error;
        self.snapshot.clone()
    }

    /// Returns current lifecycle snapshot.
    pub fn snapshot(&self) -> ExecutionLifecycleSnapshot {
        self.snapshot.clone()
    }
}

/// Durable append-only execution journal.
#[derive(Debug)]
pub struct FileExecutionJournal {
    path: PathBuf,
    file: File,
    sync_on_write: bool,
}

impl FileExecutionJournal {
    /// Opens or creates a file-backed execution journal.
    ///
    /// # Errors
    ///
    /// Returns an execution journal error when the file cannot be opened.
    pub fn open(path: impl AsRef<Path>, sync_on_write: bool) -> ExecutionResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        Ok(Self {
            path,
            file,
            sync_on_write,
        })
    }

    /// Returns the journal path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn write_line(&mut self, line: &str) -> ExecutionResult<()> {
        self.file
            .write_all(line.as_bytes())
            .and_then(|()| self.file.write_all(b"\n"))
            .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        if self.sync_on_write {
            self.file
                .sync_data()
                .map_err(|err| ExecutionError::Journal(err.to_string()))?;
        }
        Ok(())
    }
}

impl ExecutionJournal for FileExecutionJournal {
    fn record_command(
        &mut self,
        kind: JournalCommandKind,
        id: ClientOrderId,
        ts_ns: u64,
    ) -> ExecutionResult<()> {
        self.write_line(&format!("C|{}|{}|{}", command_kind_u8(kind), id, ts_ns))
    }

    fn record_event(&mut self, event: &ExecutionEvent) -> ExecutionResult<()> {
        self.write_line(&event_to_journal_line(event))
    }

    fn replay(&self, out: &mut Vec<JournalRecord>) -> ExecutionResult<usize> {
        let file =
            File::open(&self.path).map_err(|err| ExecutionError::Journal(err.to_string()))?;
        let reader = BufReader::new(file);
        let start = out.len();
        for line in reader.lines() {
            let line = line.map_err(|err| ExecutionError::Journal(err.to_string()))?;
            if line.is_empty() {
                continue;
            }
            if let Some(record) = parse_journal_line(&line)? {
                out.push(record);
            }
        }
        Ok(out.len().saturating_sub(start))
    }
}

/// Open-order reconciliation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationAction {
    /// Local and venue state match.
    Matched,
    /// Venue has an order not present locally.
    VenueOnly,
    /// Local has an order not present at the venue.
    LocalOnly,
    /// Local state should be restated from venue state.
    RestateFromVenue,
}

/// One reconciliation difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconciliationItem {
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Reconciliation action.
    pub action: ReconciliationAction,
    /// Local state when present.
    pub local: Option<OrderState>,
    /// Venue state when present.
    pub venue: Option<OrderState>,
}

/// Open-order reconciliation report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReconciliationReport {
    /// Reconciliation items.
    pub items: Vec<ReconciliationItem>,
}

impl ReconciliationReport {
    /// Returns true when no local/venue differences were found.
    pub fn is_clean(&self) -> bool {
        self.items
            .iter()
            .all(|item| item.action == ReconciliationAction::Matched)
    }
}

/// Reconciles local open-order state against venue state.
pub fn reconcile_open_orders(local: &[OrderState], venue: &[OrderState]) -> ReconciliationReport {
    let mut report = ReconciliationReport::default();
    let mut venue_by_id: HashMap<ClientOrderId, OrderState> = HashMap::with_capacity(venue.len());
    for state in venue {
        venue_by_id.insert(state.client_order_id, *state);
    }

    for local_state in local {
        match venue_by_id.remove(&local_state.client_order_id) {
            Some(venue_state) if venue_state == *local_state => {
                report.items.push(ReconciliationItem {
                    client_order_id: local_state.client_order_id,
                    action: ReconciliationAction::Matched,
                    local: Some(*local_state),
                    venue: Some(venue_state),
                })
            }
            Some(venue_state) => report.items.push(ReconciliationItem {
                client_order_id: local_state.client_order_id,
                action: ReconciliationAction::RestateFromVenue,
                local: Some(*local_state),
                venue: Some(venue_state),
            }),
            None => report.items.push(ReconciliationItem {
                client_order_id: local_state.client_order_id,
                action: ReconciliationAction::LocalOnly,
                local: Some(*local_state),
                venue: None,
            }),
        }
    }

    for venue_state in venue_by_id.into_values() {
        report.items.push(ReconciliationItem {
            client_order_id: venue_state.client_order_id,
            action: ReconciliationAction::VenueOnly,
            local: None,
            venue: Some(venue_state),
        });
    }
    report
}

/// Route safety behavior during disconnects and kill switches.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisconnectPolicy {
    /// Leave working orders untouched.
    Hold = 0,
    /// Reject new orders while allowing cancels.
    RejectNew = 1,
    /// Cancel open orders on disconnect.
    CancelOpenOrders = 2,
    /// Reject all order commands.
    Freeze = 3,
}

/// Safety policy for one route scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteSafetyPolicy {
    /// Route key.
    pub route: RouteKey,
    /// Disconnect behavior.
    pub disconnect_policy: DisconnectPolicy,
    /// Non-zero rejects new order flow.
    pub kill_switch: bool,
    /// Non-zero allows cancel commands while killed/frozen.
    pub allow_cancels_when_killed: bool,
}

impl RouteSafetyPolicy {
    /// Returns true when a new order should be rejected.
    pub const fn reject_new(self, disconnected: bool) -> bool {
        self.kill_switch
            || matches!(self.disconnect_policy, DisconnectPolicy::Freeze)
            || (disconnected
                && matches!(
                    self.disconnect_policy,
                    DisconnectPolicy::RejectNew | DisconnectPolicy::CancelOpenOrders
                ))
    }

    /// Returns true when a cancel command should be allowed.
    pub const fn allow_cancel(self) -> bool {
        !self.kill_switch
            || self.allow_cancels_when_killed
            || matches!(self.disconnect_policy, DisconnectPolicy::RejectNew)
    }
}

/// Advanced additive risk limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AdvancedRiskLimits {
    /// Basic route limits.
    pub basic: RiskLimits,
    /// Maximum messages per one-second window. Zero disables.
    pub max_message_rate_per_sec: u32,
    /// Maximum absolute net position. Zero disables.
    pub max_position_abs: i64,
    /// Maximum gross notional. Zero disables.
    pub max_gross_notional: i128,
    /// Reduce-only mode rejects orders that increase exposure.
    pub reduce_only: bool,
}

#[derive(Debug, Default)]
struct MessageRateWindow {
    timestamps_ns: VecDeque<u64>,
}

impl MessageRateWindow {
    fn allow(&mut self, ts_recv_ns: u64, max_rate: u32) -> bool {
        if max_rate == 0 {
            return true;
        }
        let cutoff = ts_recv_ns.saturating_sub(1_000_000_000);
        while self
            .timestamps_ns
            .front()
            .is_some_and(|timestamp| *timestamp <= cutoff)
        {
            self.timestamps_ns.pop_front();
        }
        if self.timestamps_ns.len() >= max_rate as usize {
            return false;
        }
        self.timestamps_ns.push_back(ts_recv_ns);
        true
    }
}

/// Advanced risk gate with basic limits plus message-rate checks.
#[derive(Debug)]
pub struct AdvancedRiskGate {
    limits: AdvancedRiskLimits,
    window: Mutex<MessageRateWindow>,
}

impl AdvancedRiskGate {
    /// Creates an advanced risk gate.
    pub fn new(limits: AdvancedRiskLimits) -> Self {
        Self {
            limits,
            window: Mutex::new(MessageRateWindow {
                timestamps_ns: VecDeque::new(),
            }),
        }
    }

    fn check_common(&self, ctx: &RiskContext, ts_recv_ns: u64) -> RiskDecision {
        if self.limits.basic.kill_switch {
            return reject(RiskRejectReason::KillSwitch, "kill switch active");
        }
        if ctx.duplicate_client_order_id {
            return reject(
                RiskRejectReason::DuplicateClientOrderId,
                "duplicate client order id",
            );
        }
        if !self
            .window
            .lock()
            .expect("risk window mutex")
            .allow(ts_recv_ns, self.limits.max_message_rate_per_sec)
        {
            return reject(RiskRejectReason::MaxOpenOrders, "message rate exceeded");
        }
        RiskDecision::allow()
    }
}

impl RiskCheck for AdvancedRiskGate {
    fn check_new(&self, req: &OrderRequest, ctx: &RiskContext) -> RiskDecision {
        let common = self.check_common(ctx, req.ts_recv_ns);
        if !common.allowed {
            return common;
        }
        if self.limits.basic.max_order_qty > 0 && req.quantity.0 > self.limits.basic.max_order_qty {
            return reject(RiskRejectReason::MaxOrderQty, "max order quantity exceeded");
        }
        let notional = i128::from(req.quantity.0).saturating_mul(i128::from(req.limit_price.0));
        if self.limits.basic.max_order_notional > 0
            && notional > self.limits.basic.max_order_notional
        {
            return reject(
                RiskRejectReason::MaxOrderNotional,
                "max order notional exceeded",
            );
        }
        if self.limits.max_gross_notional > 0
            && ctx.open_notional.saturating_add(notional) > self.limits.max_gross_notional
        {
            return reject(
                RiskRejectReason::MaxOpenNotional,
                "max gross notional exceeded",
            );
        }
        RiskDecision::allow()
    }

    fn check_amend(&self, req: &AmendRequest, ctx: &RiskContext) -> RiskDecision {
        let common = self.check_common(ctx, req.ts_recv_ns);
        if !common.allowed {
            return common;
        }
        if self.limits.basic.max_order_qty > 0 && req.quantity.0 > self.limits.basic.max_order_qty {
            return reject(RiskRejectReason::MaxOrderQty, "max order quantity exceeded");
        }
        RiskDecision::allow()
    }

    fn check_cancel(&self, req: &CancelRequest, ctx: &RiskContext) -> RiskDecision {
        self.check_common(ctx, req.ts_recv_ns)
    }
}

/// Position for one account/strategy/symbol scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    /// Net signed quantity.
    pub net_qty: i64,
    /// Buy quantity.
    pub buy_qty: i64,
    /// Sell quantity.
    pub sell_qty: i64,
    /// Gross traded notional.
    pub gross_notional: i128,
    /// Average price of the current net position.
    pub average_price: i64,
}

/// Position key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionKey {
    /// Account id.
    pub account_id: AccountId,
    /// Strategy id.
    pub strategy_id: StrategyId,
    /// Symbol.
    pub symbol: ExecutionSymbol,
}

/// Fill and position ledger.
#[derive(Debug, Default, Clone)]
pub struct PositionLedger {
    positions: HashMap<PositionKey, Position>,
}

impl PositionLedger {
    /// Creates an empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies a trade execution report to the ledger.
    pub fn apply_fill(&mut self, event: &ExecutionEvent, strategy_id: StrategyId, side: OrderSide) {
        if event.exec_type != ExecutionType::Trade || event.last_qty.0 <= 0 {
            return;
        }
        let key = PositionKey {
            account_id: event.account_id,
            strategy_id,
            symbol: event.symbol,
        };
        let position = self.positions.entry(key).or_default();
        let signed = match side {
            OrderSide::Buy => event.last_qty.0,
            OrderSide::Sell => -event.last_qty.0,
        };
        position.net_qty = position.net_qty.saturating_add(signed);
        if side == OrderSide::Buy {
            position.buy_qty = position.buy_qty.saturating_add(event.last_qty.0);
        } else {
            position.sell_qty = position.sell_qty.saturating_add(event.last_qty.0);
        }
        let fill_notional =
            i128::from(event.last_qty.0).saturating_mul(i128::from(event.last_price.0));
        position.gross_notional = position.gross_notional.saturating_add(fill_notional);
        if position.net_qty != 0 {
            position.average_price =
                (position.gross_notional / i128::from(position.net_qty.abs())) as i64;
        } else {
            position.average_price = 0;
        }
    }

    /// Returns a position by key.
    pub fn position(&self, key: &PositionKey) -> Option<Position> {
        self.positions.get(key).copied()
    }

    /// Returns all positions.
    pub fn positions(&self) -> &HashMap<PositionKey, Position> {
        &self.positions
    }
}

/// Venue-specific order type and TIF capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueOrderCapabilities {
    /// Market orders are supported.
    pub market: bool,
    /// Limit orders are supported.
    pub limit: bool,
    /// Stop orders are supported.
    pub stop: bool,
    /// Stop-limit orders are supported.
    pub stop_limit: bool,
    /// Day orders are supported.
    pub tif_day: bool,
    /// GTC orders are supported.
    pub tif_gtc: bool,
    /// IOC orders are supported.
    pub tif_ioc: bool,
    /// FOK orders are supported.
    pub tif_fok: bool,
    /// GTD orders are supported.
    pub tif_gtd: bool,
}

impl From<ExecutionCapabilities> for VenueOrderCapabilities {
    fn from(value: ExecutionCapabilities) -> Self {
        Self {
            market: value.market,
            limit: value.limit,
            stop: value.stop,
            stop_limit: value.stop_limit,
            tif_day: value.tif_day,
            tif_gtc: value.tif_gtc,
            tif_ioc: value.tif_ioc,
            tif_fok: value.tif_fok,
            tif_gtd: value.tif_gtd,
        }
    }
}

/// Normalized venue order encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizedOrderType {
    /// Canonical order type.
    pub order_type: OrderType,
    /// Canonical time-in-force.
    pub time_in_force: TimeInForce,
}

/// Validates and normalizes order type/TIF against venue capabilities.
pub fn normalize_order_type(
    order_type: OrderType,
    time_in_force: TimeInForce,
    capabilities: VenueOrderCapabilities,
) -> Result<NormalizedOrderType, RiskRejectReason> {
    let order_supported = match order_type {
        OrderType::Market => capabilities.market,
        OrderType::Limit => capabilities.limit,
        OrderType::Stop => capabilities.stop,
        OrderType::StopLimit => capabilities.stop_limit,
    };
    if !order_supported {
        return Err(RiskRejectReason::UnsupportedOrderType);
    }
    let tif_supported = match time_in_force {
        TimeInForce::Day => capabilities.tif_day,
        TimeInForce::Gtc => capabilities.tif_gtc,
        TimeInForce::Ioc => capabilities.tif_ioc,
        TimeInForce::Fok => capabilities.tif_fok,
        TimeInForce::Gtd => capabilities.tif_gtd,
    };
    if !tif_supported {
        return Err(RiskRejectReason::UnsupportedTimeInForce);
    }
    Ok(NormalizedOrderType {
        order_type,
        time_in_force,
    })
}

/// Additive execution telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExecutionTelemetry {
    /// Last observed command queue depth.
    pub command_queue_depth: u32,
    /// Last observed report queue depth.
    pub report_queue_depth: u32,
    /// Submit-to-report latency sample count.
    pub latency_samples: u64,
    /// Minimum latency in nanoseconds.
    pub min_latency_ns: u64,
    /// Maximum latency in nanoseconds.
    pub max_latency_ns: u64,
    /// Sum of latency samples in nanoseconds.
    pub total_latency_ns: u128,
}

impl ExecutionTelemetry {
    /// Records one latency sample.
    pub fn record_latency(&mut self, latency_ns: u64) {
        self.latency_samples = self.latency_samples.saturating_add(1);
        self.min_latency_ns = if self.min_latency_ns == 0 {
            latency_ns
        } else {
            self.min_latency_ns.min(latency_ns)
        };
        self.max_latency_ns = self.max_latency_ns.max(latency_ns);
        self.total_latency_ns = self.total_latency_ns.saturating_add(u128::from(latency_ns));
    }

    /// Returns average latency in nanoseconds.
    pub fn average_latency_ns(&self) -> u64 {
        if self.latency_samples == 0 {
            0
        } else {
            (self.total_latency_ns / u128::from(self.latency_samples)) as u64
        }
    }
}

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
struct StableHasher(u64);

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
    capacity: u32,
    refill_per_sec: u32,
    tokens: u32,
    last_refill_ns: u64,
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

fn command_kind_u8(kind: JournalCommandKind) -> u8 {
    match kind {
        JournalCommandKind::Submit => 1,
        JournalCommandKind::Cancel => 2,
        JournalCommandKind::Amend => 3,
    }
}

fn command_kind_from_u8(value: u8) -> Option<JournalCommandKind> {
    match value {
        1 => Some(JournalCommandKind::Submit),
        2 => Some(JournalCommandKind::Cancel),
        3 => Some(JournalCommandKind::Amend),
        _ => None,
    }
}

fn event_to_journal_line(event: &ExecutionEvent) -> String {
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

fn parse_journal_line(line: &str) -> ExecutionResult<Option<JournalRecord>> {
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

fn parse_event_parts(parts: &[&str]) -> ExecutionResult<ExecutionEvent> {
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

fn fixed<const N: usize>(value: &str) -> ExecutionResult<FixedAscii<N>> {
    FixedAscii::new(value).map_err(|err| ExecutionError::Journal(err.to_string()))
}

fn parse_u8(value: &str) -> ExecutionResult<u8> {
    value
        .parse::<u8>()
        .map_err(|err| ExecutionError::Journal(err.to_string()))
}

fn parse_i64(value: &str) -> ExecutionResult<i64> {
    value
        .parse::<i64>()
        .map_err(|err| ExecutionError::Journal(err.to_string()))
}

fn parse_u64(value: &str) -> ExecutionResult<u64> {
    value
        .parse::<u64>()
        .map_err(|err| ExecutionError::Journal(err.to_string()))
}

fn execution_type_from_u8(value: u8) -> ExecutionResult<ExecutionType> {
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

fn order_status_from_u8(value: u8) -> ExecutionResult<OrderStatus> {
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

fn risk_reason_from_u8(value: u8) -> ExecutionResult<RiskRejectReason> {
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

fn sanitize_field(value: &str) -> String {
    value.replace('|', " ")
}

fn reject(reason: RiskRejectReason, text: &str) -> RiskDecision {
    let text = ExecutionText::new(text).unwrap_or_else(|_| ExecutionText::empty());
    RiskDecision::reject(reason, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn symbol(instrument: &str) -> ExecutionSymbol {
        ExecutionSymbol {
            venue: id::<16>("SIM"),
            instrument: id::<32>(instrument),
        }
    }

    fn route() -> RouteConfig {
        RouteConfig {
            route_id: id("SIM"),
            account_id: id("ACC"),
            symbol: symbol("ES"),
            enabled: true,
            risk_limits: RiskLimits {
                kill_switch: false,
                max_order_qty: 100,
                max_order_notional: 1_000_000,
                max_open_orders: 10,
                max_open_notional: 10_000_000,
                price_band_ticks: 0,
            },
        }
    }

    fn order(client_order_id: &str) -> OrderRequest {
        OrderRequest {
            client_order_id: id(client_order_id),
            account_id: id("ACC"),
            route_id: id("SIM"),
            strategy_id: id("STRAT"),
            symbol: symbol("ES"),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(10),
            limit_price: OrderPrice(5000),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
        }
    }

    #[test]
    fn fanout_drops_when_subscriber_queue_is_full() {
        let fanout = ExecutionEventFanout::new(1);
        let sub = fanout.subscribe();
        let req = order("C1");
        fanout.publish(ExecutionEvent::accepted(&req, id("V1")));
        fanout.publish(ExecutionEvent::accepted(&req, id("V2")));

        assert!(sub.try_recv().is_some());
        assert_eq!(fanout.dropped_events(), 1);
    }

    #[test]
    fn file_journal_replays_commands_and_events() {
        let path =
            std::env::temp_dir().join(format!("orderflow-journal-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut journal = FileExecutionJournal::open(&path, false).unwrap();
        let req = order("C1");
        journal
            .record_command(
                JournalCommandKind::Submit,
                req.client_order_id,
                req.ts_recv_ns,
            )
            .unwrap();
        journal
            .record_event(&ExecutionEvent::accepted(&req, id("V1")))
            .unwrap();
        drop(journal);

        let journal = FileExecutionJournal::open(&path, false).unwrap();
        let mut records = Vec::new();
        assert_eq!(journal.replay(&mut records).unwrap(), 2);
        assert!(matches!(records[0], JournalRecord::Command { .. }));
        assert!(matches!(records[1], JournalRecord::Event(_)));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn reconciliation_detects_venue_only_state() {
        let req = order("C1");
        let state = OrderState::pending_new(&req);
        let report = reconcile_open_orders(&[], &[state]);
        assert_eq!(report.items.len(), 1);
        assert_eq!(report.items[0].action, ReconciliationAction::VenueOnly);
    }

    #[test]
    fn throttle_refills_over_time() {
        let mut throttle = OrderThrottle::new(1, 1);
        assert!(throttle.allow(1));
        assert!(!throttle.allow(2));
        assert!(throttle.allow(1_000_000_002));
    }

    #[test]
    fn replay_simulation_is_deterministic() {
        let decisions = [ReplayDecision {
            ts_recv_ns: 2,
            command: ExecutionCommand::Submit(order("C1")),
        }];
        let result = replay_simulated_oms(vec![route()], &decisions).unwrap();
        assert_eq!(result.reports.len(), 1);
        assert_eq!(result.reports[0].events.len(), 2);
        assert_eq!(result.metrics.submitted, 1);
    }

    #[test]
    fn normalize_rejects_unsupported_tif() {
        let caps = VenueOrderCapabilities {
            market: true,
            limit: true,
            stop: false,
            stop_limit: false,
            tif_day: true,
            tif_gtc: false,
            tif_ioc: false,
            tif_fok: false,
            tif_gtd: false,
        };
        assert_eq!(
            normalize_order_type(OrderType::Limit, TimeInForce::Gtc, caps).unwrap_err(),
            RiskRejectReason::UnsupportedTimeInForce
        );
    }
}
