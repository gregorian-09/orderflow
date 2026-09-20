use super::*;

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

/// Production safety condition that may require fail-open or fail-closed action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SafetyCondition {
    /// Market data is stale for the relevant execution decision.
    MarketDataStale,
    /// Market-data persistence is degraded.
    MarketDataPersistenceDegraded,
    /// OMS WAL or journal persistence is degraded.
    OmsWalDegraded,
    /// Checkpoint creation or validation failed.
    CheckpointFailed,
    /// Execution adapter/session is disconnected.
    AdapterDisconnected,
    /// Drop-copy channel is disconnected.
    DropCopyDisconnected,
    /// Venue/local reconciliation found a mismatch.
    ReconciliationMismatch,
    /// Risk engine or risk dependency is unavailable.
    RiskUnavailable,
    /// Position ledger differs from the source of truth.
    PositionLedgerMismatch,
    /// Route health is degraded.
    RouteHealthDegraded,
}

/// Safety action applied when a condition is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum SafetyPolicyAction {
    /// Continue without degradation.
    Allow,
    /// Continue in a visible degraded/fail-open mode.
    AllowDegraded,
    /// Reject new orders while allowing cancels.
    #[default]
    RejectNew,
    /// Reject all order commands.
    RejectAll,
    /// Block new submissions until an operator approves recovery.
    RequireOperatorApproval,
}

impl SafetyPolicyAction {
    /// Returns true when the action allows new submissions.
    pub const fn allows_new_orders(self) -> bool {
        matches!(self, Self::Allow | Self::AllowDegraded)
    }

    /// Returns true when the action allows cancels.
    pub const fn allows_cancels(self) -> bool {
        !matches!(self, Self::RejectAll)
    }

    /// Returns true when the action represents an explicit degraded allowance.
    pub const fn is_fail_open(self) -> bool {
        matches!(self, Self::AllowDegraded)
    }
}

/// Boolean safety condition snapshot for one evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SafetyContext {
    /// Market data is stale.
    pub market_data_stale: bool,
    /// Market-data persistence is degraded.
    pub market_data_persistence_degraded: bool,
    /// OMS WAL or journal persistence is degraded.
    pub oms_wal_degraded: bool,
    /// Checkpoint creation or validation failed.
    pub checkpoint_failed: bool,
    /// Execution adapter/session is disconnected.
    pub adapter_disconnected: bool,
    /// Drop-copy channel is disconnected.
    pub drop_copy_disconnected: bool,
    /// Venue/local reconciliation found a mismatch.
    pub reconciliation_mismatch: bool,
    /// Risk engine or risk dependency is unavailable.
    pub risk_unavailable: bool,
    /// Position ledger differs from the source of truth.
    pub position_ledger_mismatch: bool,
    /// Route health is degraded.
    pub route_health_degraded: bool,
}

impl SafetyContext {
    /// Returns true when any safety condition is active.
    pub const fn any_active(self) -> bool {
        self.market_data_stale
            || self.market_data_persistence_degraded
            || self.oms_wal_degraded
            || self.checkpoint_failed
            || self.adapter_disconnected
            || self.drop_copy_disconnected
            || self.reconciliation_mismatch
            || self.risk_unavailable
            || self.position_ledger_mismatch
            || self.route_health_degraded
    }
}

/// Configurable safety policy matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SafetyPolicy {
    /// Action for stale market data.
    pub market_data_stale: SafetyPolicyAction,
    /// Action for degraded market-data persistence.
    pub market_data_persistence_degraded: SafetyPolicyAction,
    /// Action for degraded OMS WAL or journal.
    pub oms_wal_degraded: SafetyPolicyAction,
    /// Action for checkpoint failures.
    pub checkpoint_failed: SafetyPolicyAction,
    /// Action for disconnected execution adapters.
    pub adapter_disconnected: SafetyPolicyAction,
    /// Action for disconnected drop-copy.
    pub drop_copy_disconnected: SafetyPolicyAction,
    /// Action for reconciliation mismatch.
    pub reconciliation_mismatch: SafetyPolicyAction,
    /// Action for unavailable risk dependencies.
    pub risk_unavailable: SafetyPolicyAction,
    /// Action for position ledger mismatch.
    pub position_ledger_mismatch: SafetyPolicyAction,
    /// Action for degraded route health.
    pub route_health_degraded: SafetyPolicyAction,
}

impl SafetyPolicy {
    /// Creates a conservative fail-closed policy that rejects new orders for
    /// every active condition while still allowing cancels.
    pub const fn fail_closed() -> Self {
        Self {
            market_data_stale: SafetyPolicyAction::RejectNew,
            market_data_persistence_degraded: SafetyPolicyAction::RejectNew,
            oms_wal_degraded: SafetyPolicyAction::RejectNew,
            checkpoint_failed: SafetyPolicyAction::RejectNew,
            adapter_disconnected: SafetyPolicyAction::RejectNew,
            drop_copy_disconnected: SafetyPolicyAction::RejectNew,
            reconciliation_mismatch: SafetyPolicyAction::RejectNew,
            risk_unavailable: SafetyPolicyAction::RejectNew,
            position_ledger_mismatch: SafetyPolicyAction::RejectNew,
            route_health_degraded: SafetyPolicyAction::RejectNew,
        }
    }

    /// Creates a visible degraded policy that allows new orders for every
    /// active condition and records each allowance as fail-open.
    pub const fn fail_open_degraded() -> Self {
        Self {
            market_data_stale: SafetyPolicyAction::AllowDegraded,
            market_data_persistence_degraded: SafetyPolicyAction::AllowDegraded,
            oms_wal_degraded: SafetyPolicyAction::AllowDegraded,
            checkpoint_failed: SafetyPolicyAction::AllowDegraded,
            adapter_disconnected: SafetyPolicyAction::AllowDegraded,
            drop_copy_disconnected: SafetyPolicyAction::AllowDegraded,
            reconciliation_mismatch: SafetyPolicyAction::AllowDegraded,
            risk_unavailable: SafetyPolicyAction::AllowDegraded,
            position_ledger_mismatch: SafetyPolicyAction::AllowDegraded,
            route_health_degraded: SafetyPolicyAction::AllowDegraded,
        }
    }

    /// Sets the action for one condition.
    pub const fn with_action(
        mut self,
        condition: SafetyCondition,
        action: SafetyPolicyAction,
    ) -> Self {
        match condition {
            SafetyCondition::MarketDataStale => self.market_data_stale = action,
            SafetyCondition::MarketDataPersistenceDegraded => {
                self.market_data_persistence_degraded = action;
            }
            SafetyCondition::OmsWalDegraded => self.oms_wal_degraded = action,
            SafetyCondition::CheckpointFailed => self.checkpoint_failed = action,
            SafetyCondition::AdapterDisconnected => self.adapter_disconnected = action,
            SafetyCondition::DropCopyDisconnected => self.drop_copy_disconnected = action,
            SafetyCondition::ReconciliationMismatch => self.reconciliation_mismatch = action,
            SafetyCondition::RiskUnavailable => self.risk_unavailable = action,
            SafetyCondition::PositionLedgerMismatch => self.position_ledger_mismatch = action,
            SafetyCondition::RouteHealthDegraded => self.route_health_degraded = action,
        }
        self
    }

    /// Returns the configured action for a condition.
    pub const fn action_for(self, condition: SafetyCondition) -> SafetyPolicyAction {
        match condition {
            SafetyCondition::MarketDataStale => self.market_data_stale,
            SafetyCondition::MarketDataPersistenceDegraded => self.market_data_persistence_degraded,
            SafetyCondition::OmsWalDegraded => self.oms_wal_degraded,
            SafetyCondition::CheckpointFailed => self.checkpoint_failed,
            SafetyCondition::AdapterDisconnected => self.adapter_disconnected,
            SafetyCondition::DropCopyDisconnected => self.drop_copy_disconnected,
            SafetyCondition::ReconciliationMismatch => self.reconciliation_mismatch,
            SafetyCondition::RiskUnavailable => self.risk_unavailable,
            SafetyCondition::PositionLedgerMismatch => self.position_ledger_mismatch,
            SafetyCondition::RouteHealthDegraded => self.route_health_degraded,
        }
    }
}

impl Default for SafetyPolicy {
    fn default() -> Self {
        Self::fail_closed()
    }
}

/// One active safety policy decision item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SafetyPolicyDecisionItem {
    /// Active condition.
    pub condition: SafetyCondition,
    /// Action selected by the policy.
    pub action: SafetyPolicyAction,
}

/// Safety policy evaluation report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SafetyPolicyDecision {
    /// Active condition decisions.
    pub items: Vec<SafetyPolicyDecisionItem>,
    /// True when new submissions may continue.
    pub submissions_enabled: bool,
    /// True when cancel commands may continue.
    pub cancels_enabled: bool,
    /// True when operator approval is required before submissions resume.
    pub operator_approval_required: bool,
    /// True when at least one active condition is allowed in degraded mode.
    pub degraded: bool,
    /// Number of active conditions allowed in fail-open degraded mode.
    pub fail_open_count: u32,
    /// True when at least one active condition blocks new submissions or all
    /// commands.
    pub fail_closed: bool,
}

impl Default for SafetyPolicyDecision {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            submissions_enabled: true,
            cancels_enabled: true,
            operator_approval_required: false,
            degraded: false,
            fail_open_count: 0,
            fail_closed: false,
        }
    }
}

/// Evaluates a safety policy against current condition flags.
pub fn evaluate_safety_policy(
    context: SafetyContext,
    policy: SafetyPolicy,
) -> SafetyPolicyDecision {
    let mut decision = SafetyPolicyDecision::default();
    for condition in active_safety_conditions(context) {
        let action = policy.action_for(condition);
        decision.submissions_enabled &= action.allows_new_orders();
        decision.cancels_enabled &= action.allows_cancels();
        decision.operator_approval_required |=
            action == SafetyPolicyAction::RequireOperatorApproval;
        decision.degraded |= action.is_fail_open();
        decision.fail_open_count = decision
            .fail_open_count
            .saturating_add(u32::from(action.is_fail_open()));
        decision.fail_closed |= matches!(
            action,
            SafetyPolicyAction::RejectNew
                | SafetyPolicyAction::RejectAll
                | SafetyPolicyAction::RequireOperatorApproval
        );
        decision
            .items
            .push(SafetyPolicyDecisionItem { condition, action });
    }
    decision
}

pub(crate) fn active_safety_conditions(
    context: SafetyContext,
) -> impl Iterator<Item = SafetyCondition> {
    [
        (context.market_data_stale, SafetyCondition::MarketDataStale),
        (
            context.market_data_persistence_degraded,
            SafetyCondition::MarketDataPersistenceDegraded,
        ),
        (context.oms_wal_degraded, SafetyCondition::OmsWalDegraded),
        (context.checkpoint_failed, SafetyCondition::CheckpointFailed),
        (
            context.adapter_disconnected,
            SafetyCondition::AdapterDisconnected,
        ),
        (
            context.drop_copy_disconnected,
            SafetyCondition::DropCopyDisconnected,
        ),
        (
            context.reconciliation_mismatch,
            SafetyCondition::ReconciliationMismatch,
        ),
        (context.risk_unavailable, SafetyCondition::RiskUnavailable),
        (
            context.position_ledger_mismatch,
            SafetyCondition::PositionLedgerMismatch,
        ),
        (
            context.route_health_degraded,
            SafetyCondition::RouteHealthDegraded,
        ),
    ]
    .into_iter()
    .filter_map(|(active, condition)| active.then_some(condition))
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
pub(crate) struct MessageRateWindow {
    pub(crate) timestamps_ns: VecDeque<u64>,
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
    pub(crate) limits: AdvancedRiskLimits,
    pub(crate) window: Mutex<MessageRateWindow>,
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
    pub(crate) positions: HashMap<PositionKey, Position>,
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
