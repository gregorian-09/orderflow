//! Scoped, explainable, low-allocation production risk controls.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

use of_execution_core::{
    AccountId, AmendRequest, CancelRequest, ClientOrderId, ExecutionCoreError, ExecutionSymbol,
    FixedAscii, OrderPrice, OrderQty, OrderRequest, OrderSide, OrderType, RouteId, StrategyId,
    VenueId,
};

/// Maximum bytes stored in a production risk policy identifier.
pub const PRODUCTION_RISK_POLICY_ID_CAPACITY: usize = 32;
/// Maximum bytes stored in an instrument-group identifier.
pub const RISK_INSTRUMENT_GROUP_ID_CAPACITY: usize = 32;
/// Largest rate-window capacity accepted by one policy.
pub const MAX_PRODUCTION_RISK_RATE_PER_SEC: u32 = 1_000_000;

const NANOS_PER_DAY: u64 = 86_400_000_000_000;
const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Stable identifier for one risk policy.
pub type ProductionRiskPolicyId = FixedAscii<PRODUCTION_RISK_POLICY_ID_CAPACITY>;
/// Host-defined instrument or product group identifier.
pub type RiskInstrumentGroupId = FixedAscii<RISK_INSTRUMENT_GROUP_ID_CAPACITY>;

/// Scope matched by a production risk policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProductionRiskScope {
    /// Every command evaluated by the engine.
    Global,
    /// One trading account.
    Account(AccountId),
    /// One strategy.
    Strategy(StrategyId),
    /// One execution route.
    Route(RouteId),
    /// One venue-native symbol.
    Symbol(ExecutionSymbol),
    /// One venue or exchange.
    Venue(VenueId),
    /// One host-defined instrument group.
    InstrumentGroup(RiskInstrumentGroupId),
}

/// UTC nanosecond-of-day trading window.
///
/// Equal start and end values represent a full-day window. A start greater
/// than end represents an overnight window crossing UTC midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RiskTradingWindow {
    start_ns_of_day: u64,
    end_ns_of_day: u64,
}

impl RiskTradingWindow {
    /// Creates a validated UTC trading window.
    ///
    /// # Errors
    ///
    /// Returns [`ProductionRiskError::InvalidTradingWindow`] when either value
    /// is outside one UTC day.
    pub const fn new(
        start_ns_of_day: u64,
        end_ns_of_day: u64,
    ) -> Result<Self, ProductionRiskError> {
        if start_ns_of_day >= NANOS_PER_DAY || end_ns_of_day >= NANOS_PER_DAY {
            return Err(ProductionRiskError::InvalidTradingWindow);
        }
        Ok(Self {
            start_ns_of_day,
            end_ns_of_day,
        })
    }

    /// Returns the UTC start nanosecond of day.
    pub const fn start_ns_of_day(self) -> u64 {
        self.start_ns_of_day
    }

    /// Returns the UTC end nanosecond of day.
    pub const fn end_ns_of_day(self) -> u64 {
        self.end_ns_of_day
    }

    /// Returns true when `timestamp_ns` falls inside the configured window.
    pub const fn contains(self, timestamp_ns: u64) -> bool {
        if self.start_ns_of_day == self.end_ns_of_day {
            return true;
        }
        let value = timestamp_ns % NANOS_PER_DAY;
        if self.start_ns_of_day < self.end_ns_of_day {
            value >= self.start_ns_of_day && value < self.end_ns_of_day
        } else {
            value >= self.start_ns_of_day || value < self.end_ns_of_day
        }
    }
}

/// Limits and safety conditions attached to one scoped policy.
///
/// Numeric zero values disable their check. Operational block flags default
/// to conservative fail-closed behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProductionRiskLimits {
    /// Maximum quantity per submit or amend.
    pub max_order_qty: i64,
    /// Maximum normalized notional per submit or amend.
    pub max_order_notional: i128,
    /// Maximum projected absolute position.
    pub max_position_abs: i64,
    /// Maximum projected gross exposure.
    pub max_gross_exposure: i128,
    /// Maximum projected absolute net exposure.
    pub max_net_exposure_abs: i128,
    /// Maximum projected open-order count.
    pub max_open_orders: u32,
    /// Maximum submit/amend messages in a trailing one-second window.
    pub max_order_rate_per_sec: u32,
    /// Maximum cancel messages in a trailing one-second window.
    pub max_cancel_rate_per_sec: u32,
    /// Maximum absolute limit-price distance from reference price.
    pub price_collar_ticks: i64,
    /// Maximum quantity as basis points of typical quantity.
    pub max_typical_qty_multiple_bps: u32,
    /// Reject every new/amend command matching this policy.
    pub restricted: bool,
    /// Reject when the host self-trade hook reports a collision.
    pub block_self_trade: bool,
    /// Optional UTC trading window.
    pub trading_window: Option<RiskTradingWindow>,
    /// Permit only strict exposure-reducing commands.
    pub reduce_only: bool,
    /// Maximum absolute daily loss in normalized money units.
    pub max_loss: i128,
    /// Maximum decline from peak daily PnL.
    pub max_daily_drawdown: i128,
    /// Block new/amend commands on stale market data.
    pub block_stale_market_data: bool,
    /// Block new/amend commands on degraded adapter health.
    pub block_degraded_adapter: bool,
    /// Block new/amend commands on degraded required persistence.
    pub block_degraded_persistence: bool,
    /// Block new/amend commands when risk/ledger state is unavailable.
    pub block_unavailable_risk_state: bool,
}

impl ProductionRiskLimits {
    /// Creates limits with numeric checks disabled and operational safety
    /// conditions enabled.
    pub const fn conservative() -> Self {
        Self {
            max_order_qty: 0,
            max_order_notional: 0,
            max_position_abs: 0,
            max_gross_exposure: 0,
            max_net_exposure_abs: 0,
            max_open_orders: 0,
            max_order_rate_per_sec: 0,
            max_cancel_rate_per_sec: 0,
            price_collar_ticks: 0,
            max_typical_qty_multiple_bps: 0,
            restricted: false,
            block_self_trade: true,
            trading_window: None,
            reduce_only: false,
            max_loss: 0,
            max_daily_drawdown: 0,
            block_stale_market_data: true,
            block_degraded_adapter: true,
            block_degraded_persistence: true,
            block_unavailable_risk_state: true,
        }
    }

    /// Creates fully permissive limits for explicit simulation/test use.
    pub const fn permissive() -> Self {
        Self {
            block_self_trade: false,
            block_stale_market_data: false,
            block_degraded_adapter: false,
            block_degraded_persistence: false,
            block_unavailable_risk_state: false,
            ..Self::conservative()
        }
    }
}

impl Default for ProductionRiskLimits {
    fn default() -> Self {
        Self::conservative()
    }
}

/// One ordered scoped production risk policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProductionRiskPolicy {
    /// Stable policy identifier included in every matching decision.
    pub policy_id: ProductionRiskPolicyId,
    /// Scope matched by this policy.
    pub scope: ProductionRiskScope,
    /// Lower values evaluate first and determine the primary rejection.
    pub priority: u16,
    /// Configured limits and safety controls.
    pub limits: ProductionRiskLimits,
}

impl ProductionRiskPolicy {
    /// Creates a policy from fixed-size identifiers and limits.
    pub const fn new(
        policy_id: ProductionRiskPolicyId,
        scope: ProductionRiskScope,
        priority: u16,
        limits: ProductionRiskLimits,
    ) -> Self {
        Self {
            policy_id,
            scope,
            priority,
            limits,
        }
    }

    /// Creates a policy from an ASCII policy id.
    ///
    /// # Errors
    ///
    /// Returns an identifier error when the id is non-ASCII or too long.
    pub fn from_id(
        policy_id: &str,
        scope: ProductionRiskScope,
        priority: u16,
        limits: ProductionRiskLimits,
    ) -> Result<Self, ExecutionCoreError> {
        Ok(Self::new(
            ProductionRiskPolicyId::new(policy_id)?,
            scope,
            priority,
            limits,
        ))
    }
}

/// Command classification used by production risk evaluation.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProductionRiskCommandKind {
    /// New order submission.
    Submit = 0,
    /// Cancel/replace amendment.
    Amend = 1,
    /// Order cancellation.
    Cancel = 2,
}

/// Canonical command view consumed by [`ProductionRiskEngine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProductionRiskCommand {
    /// Command kind.
    pub kind: ProductionRiskCommandKind,
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Trading account.
    pub account_id: AccountId,
    /// Execution route.
    pub route_id: RouteId,
    /// Strategy attribution.
    pub strategy_id: StrategyId,
    /// Venue-native symbol.
    pub symbol: ExecutionSymbol,
    /// Host-defined instrument group.
    pub instrument_group: RiskInstrumentGroupId,
    /// Side for submit/amend; ignored for cancel.
    pub side: OrderSide,
    /// Order type for submit/amend; ignored for cancel.
    pub order_type: OrderType,
    /// New quantity for submit/amend; zero for cancel.
    pub quantity: OrderQty,
    /// New limit/referenceable price; zero when not supplied.
    pub price: OrderPrice,
    /// Existing quantity replaced by an amend; zero otherwise.
    pub existing_quantity: OrderQty,
    /// Existing price replaced by an amend; zero otherwise.
    pub existing_price: OrderPrice,
    /// Caller-supplied receive timestamp in nanoseconds.
    pub timestamp_ns: u64,
}

impl ProductionRiskCommand {
    /// Creates a submit command view.
    pub const fn submit(request: &OrderRequest, instrument_group: RiskInstrumentGroupId) -> Self {
        Self {
            kind: ProductionRiskCommandKind::Submit,
            client_order_id: request.client_order_id,
            account_id: request.account_id,
            route_id: request.route_id,
            strategy_id: request.strategy_id,
            symbol: request.symbol,
            instrument_group,
            side: request.side,
            order_type: request.order_type,
            quantity: request.quantity,
            price: request.limit_price,
            existing_quantity: OrderQty(0),
            existing_price: OrderPrice(0),
            timestamp_ns: request.ts_recv_ns,
        }
    }

    /// Creates an amend command view with existing-order metadata supplied by
    /// the OMS host.
    #[allow(clippy::too_many_arguments, reason = "amend metadata is explicit")]
    pub const fn amend(
        request: &AmendRequest,
        strategy_id: StrategyId,
        side: OrderSide,
        order_type: OrderType,
        instrument_group: RiskInstrumentGroupId,
        existing_quantity: OrderQty,
        existing_price: OrderPrice,
    ) -> Self {
        Self {
            kind: ProductionRiskCommandKind::Amend,
            client_order_id: request.client_order_id,
            account_id: request.account_id,
            route_id: request.route_id,
            strategy_id,
            symbol: request.symbol,
            instrument_group,
            side,
            order_type,
            quantity: request.quantity,
            price: request.limit_price,
            existing_quantity,
            existing_price,
            timestamp_ns: request.ts_recv_ns,
        }
    }

    /// Creates a cancel command view.
    pub const fn cancel(
        request: &CancelRequest,
        strategy_id: StrategyId,
        instrument_group: RiskInstrumentGroupId,
    ) -> Self {
        Self {
            kind: ProductionRiskCommandKind::Cancel,
            client_order_id: request.client_order_id,
            account_id: request.account_id,
            route_id: request.route_id,
            strategy_id,
            symbol: request.symbol,
            instrument_group,
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            quantity: OrderQty(0),
            price: OrderPrice(0),
            existing_quantity: OrderQty(0),
            existing_price: OrderPrice(0),
            timestamp_ns: request.ts_recv_ns,
        }
    }
}

/// Caller-supplied state used for production risk checks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProductionRiskContext {
    /// True when position, exposure, PnL, and open-order state is authoritative.
    pub risk_state_available: bool,
    /// True when client order id already exists in the relevant lifecycle.
    pub duplicate_client_order_id: bool,
    /// Result from the host self-trade prevention hook.
    pub self_trade_risk: bool,
    /// Current open-order count for this scope.
    pub open_orders: u32,
    /// Current signed position.
    pub current_position: i64,
    /// Current gross exposure, including the existing order on amend.
    pub current_gross_exposure: i128,
    /// Current signed net exposure, including the existing order on amend.
    pub current_net_exposure: i128,
    /// Current market/reference price.
    pub reference_price: OrderPrice,
    /// Typical order quantity used by fat-finger multiple checks.
    pub typical_order_qty: OrderQty,
    /// Current daily realized plus marked PnL.
    pub daily_pnl: i128,
    /// Highest daily PnL observed before this decision.
    pub peak_daily_pnl: i128,
    /// True when required market data is stale.
    pub market_data_stale: bool,
    /// True when the selected execution adapter is degraded.
    pub adapter_degraded: bool,
    /// True when required command/event persistence is degraded.
    pub persistence_degraded: bool,
}

impl ProductionRiskContext {
    /// Creates an available zero-exposure context for tests and explicit new
    /// sessions.
    pub const fn available() -> Self {
        Self {
            risk_state_available: true,
            duplicate_client_order_id: false,
            self_trade_risk: false,
            open_orders: 0,
            current_position: 0,
            current_gross_exposure: 0,
            current_net_exposure: 0,
            reference_price: OrderPrice(0),
            typical_order_qty: OrderQty(0),
            daily_pnl: 0,
            peak_daily_pnl: 0,
            market_data_stale: false,
            adapter_degraded: false,
            persistence_degraded: false,
        }
    }
}

/// Detailed production risk decision reason.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProductionRiskReason {
    /// Every matching policy allowed the command.
    Allowed = 0,
    /// No policy matched, so the engine failed closed.
    NoMatchingPolicy = 1,
    /// Risk or ledger state is unavailable.
    RiskStateUnavailable = 2,
    /// Policy marks the scope restricted.
    RestrictedScope = 3,
    /// Command falls outside the configured trading window.
    OutsideTradingWindow = 4,
    /// Client order id is duplicated.
    DuplicateClientOrderId = 5,
    /// Host self-trade hook found a collision.
    SelfTradeRisk = 6,
    /// Order quantity exceeds the configured maximum.
    MaxOrderQty = 7,
    /// Order notional exceeds the configured maximum.
    MaxOrderNotional = 8,
    /// Projected absolute position exceeds the maximum.
    MaxPosition = 9,
    /// Projected gross exposure exceeds the maximum.
    MaxGrossExposure = 10,
    /// Projected absolute net exposure exceeds the maximum.
    MaxNetExposure = 11,
    /// Projected open-order count exceeds the maximum.
    MaxOpenOrders = 12,
    /// Submit/amend message rate exceeds the one-second limit.
    MaxOrderRate = 13,
    /// Cancel message rate exceeds the one-second limit.
    MaxCancelRate = 14,
    /// Price lies outside the configured collar.
    PriceCollar = 15,
    /// Quantity exceeds the configured typical-size multiple.
    FatFinger = 16,
    /// Command does not strictly reduce exposure.
    ReduceOnly = 17,
    /// Daily loss limit is reached.
    MaxLoss = 18,
    /// Daily drawdown limit is reached.
    MaxDailyDrawdown = 19,
    /// Required market data is stale.
    MarketDataStale = 20,
    /// Execution adapter is degraded.
    AdapterDegraded = 21,
    /// Required persistence is degraded.
    PersistenceDegraded = 22,
    /// Reference price is required but unavailable.
    ReferencePriceUnavailable = 23,
    /// Rate-window timestamp regressed.
    TimestampRegression = 24,
    /// Decision journal rejected or could not retain the decision.
    DecisionJournalUnavailable = 25,
}

/// Decision-journal state returned to the caller.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProductionRiskJournalStatus {
    /// Caller used evaluation without a journal.
    NotRequested = 0,
    /// Decision was accepted by the configured journal.
    Recorded = 1,
    /// Journal failed and the returned decision was forced to reject.
    Failed = 2,
}

/// Explainable allocation-free production risk decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProductionRiskDecision {
    /// True when every matching policy allowed the command.
    pub allowed: bool,
    /// Primary allow/reject reason.
    pub reason: ProductionRiskReason,
    /// Policy producing the primary rejection, or empty for engine-level failure.
    pub policy_id: ProductionRiskPolicyId,
    /// Matching policy scope when available.
    pub scope: Option<ProductionRiskScope>,
    /// Command kind.
    pub command_kind: ProductionRiskCommandKind,
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Observed value associated with the primary reason.
    pub observed: i128,
    /// Configured limit associated with the primary reason.
    pub limit: i128,
    /// Caller-supplied command timestamp.
    pub timestamp_ns: u64,
    /// Risk policy-set version.
    pub policy_version: u64,
    /// Number of matching policies evaluated.
    pub checked_policies: u32,
    /// Journal outcome for this decision.
    pub journal_status: ProductionRiskJournalStatus,
}

impl ProductionRiskDecision {
    fn allowed(command: ProductionRiskCommand, policy_version: u64) -> Self {
        Self {
            allowed: true,
            reason: ProductionRiskReason::Allowed,
            policy_id: ProductionRiskPolicyId::empty(),
            scope: None,
            command_kind: command.kind,
            client_order_id: command.client_order_id,
            observed: 0,
            limit: 0,
            timestamp_ns: command.timestamp_ns,
            policy_version,
            checked_policies: 0,
            journal_status: ProductionRiskJournalStatus::NotRequested,
        }
    }

    fn reject_engine(
        command: ProductionRiskCommand,
        policy_version: u64,
        reason: ProductionRiskReason,
    ) -> Self {
        Self {
            allowed: false,
            reason,
            ..Self::allowed(command, policy_version)
        }
    }

    fn reject_policy(
        command: ProductionRiskCommand,
        policy_version: u64,
        policy: ProductionRiskPolicy,
        reason: ProductionRiskReason,
        observed: i128,
        limit: i128,
    ) -> Self {
        Self {
            allowed: false,
            reason,
            policy_id: policy.policy_id,
            scope: Some(policy.scope),
            command_kind: command.kind,
            client_order_id: command.client_order_id,
            observed,
            limit,
            timestamp_ns: command.timestamp_ns,
            policy_version,
            checked_policies: 0,
            journal_status: ProductionRiskJournalStatus::NotRequested,
        }
    }
}

/// Configuration and capacity errors for production risk state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProductionRiskError {
    /// Trading window value lies outside one UTC day.
    InvalidTradingWindow,
    /// Policy rate exceeds [`MAX_PRODUCTION_RISK_RATE_PER_SEC`].
    RateLimitTooLarge,
    /// Policy identifier is empty and cannot distinguish audit decisions.
    EmptyPolicyId,
    /// One or more signed numeric limits are negative.
    InvalidLimit,
    /// Policy capacity is exhausted.
    PolicyCapacityExceeded,
    /// Policy id already exists.
    DuplicatePolicyId,
    /// Requested policy id does not exist.
    PolicyNotFound,
}

impl fmt::Display for ProductionRiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTradingWindow => write!(f, "risk trading window is outside one UTC day"),
            Self::RateLimitTooLarge => write!(f, "production risk rate limit is too large"),
            Self::EmptyPolicyId => write!(f, "production risk policy id is empty"),
            Self::InvalidLimit => write!(f, "production risk limit cannot be negative"),
            Self::PolicyCapacityExceeded => write!(f, "production risk policy capacity exceeded"),
            Self::DuplicatePolicyId => write!(f, "production risk policy id already exists"),
            Self::PolicyNotFound => write!(f, "production risk policy id was not found"),
        }
    }
}

impl Error for ProductionRiskError {}

/// Bounded decision-journal error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionRiskJournalError {
    /// Journal capacity is exhausted.
    Full,
    /// Journal is unavailable or degraded.
    Unavailable,
}

impl fmt::Display for ProductionRiskJournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "production risk decision journal is full"),
            Self::Unavailable => write!(f, "production risk decision journal is unavailable"),
        }
    }
}

impl Error for ProductionRiskJournalError {}

/// Journal contract for explainable production risk decisions.
pub trait ProductionRiskDecisionJournal: Send {
    /// Records one decision before an allowed command routes.
    ///
    /// # Errors
    ///
    /// Returns a bounded or durability error. Callers must fail closed.
    fn record(
        &mut self,
        decision: &ProductionRiskDecision,
    ) -> Result<(), ProductionRiskJournalError>;
}

/// Bounded in-memory decision journal for tests and low-latency handoff.
#[derive(Debug, Clone)]
pub struct InMemoryProductionRiskJournal {
    decisions: Vec<ProductionRiskDecision>,
    capacity: usize,
    available: bool,
}

impl InMemoryProductionRiskJournal {
    /// Creates an available bounded journal.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            decisions: Vec::with_capacity(capacity),
            capacity,
            available: true,
        }
    }

    /// Marks the journal available or unavailable.
    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }

    /// Returns recorded decisions.
    pub fn decisions(&self) -> &[ProductionRiskDecision] {
        &self.decisions
    }

    /// Clears retained decisions without releasing capacity.
    pub fn clear(&mut self) {
        self.decisions.clear();
    }
}

impl ProductionRiskDecisionJournal for InMemoryProductionRiskJournal {
    fn record(
        &mut self,
        decision: &ProductionRiskDecision,
    ) -> Result<(), ProductionRiskJournalError> {
        if !self.available {
            return Err(ProductionRiskJournalError::Unavailable);
        }
        if self.decisions.len() >= self.capacity {
            return Err(ProductionRiskJournalError::Full);
        }
        self.decisions.push(*decision);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RateObservation {
    Allowed,
    Exceeded(u32),
    TimestampRegression(u64),
}

#[derive(Debug)]
struct RateWindow {
    timestamps_ns: VecDeque<u64>,
    last_timestamp_ns: u64,
}

impl RateWindow {
    fn new(capacity: u32) -> Self {
        Self {
            timestamps_ns: VecDeque::with_capacity(capacity as usize),
            last_timestamp_ns: 0,
        }
    }

    fn observe(&mut self, timestamp_ns: u64, limit: u32) -> RateObservation {
        if limit == 0 {
            return RateObservation::Allowed;
        }
        if self.last_timestamp_ns != 0 && timestamp_ns < self.last_timestamp_ns {
            return RateObservation::TimestampRegression(self.last_timestamp_ns);
        }
        self.last_timestamp_ns = timestamp_ns;
        let cutoff = timestamp_ns.saturating_sub(NANOS_PER_SECOND);
        while self
            .timestamps_ns
            .front()
            .is_some_and(|value| *value <= cutoff)
        {
            self.timestamps_ns.pop_front();
        }
        let observed = self.timestamps_ns.len().saturating_add(1);
        if observed > limit as usize {
            return RateObservation::Exceeded(observed.min(u32::MAX as usize) as u32);
        }
        self.timestamps_ns.push_back(timestamp_ns);
        RateObservation::Allowed
    }
}

#[derive(Debug)]
struct ProductionRiskPolicyState {
    policy: ProductionRiskPolicy,
    order_rate: RateWindow,
    cancel_rate: RateWindow,
}

impl ProductionRiskPolicyState {
    fn new(policy: ProductionRiskPolicy) -> Self {
        Self {
            order_rate: RateWindow::new(policy.limits.max_order_rate_per_sec),
            cancel_rate: RateWindow::new(policy.limits.max_cancel_rate_per_sec),
            policy,
        }
    }
}

/// Ordered bounded engine for scoped production risk controls.
///
/// The engine is mutable and single-owner so per-policy rate windows need no
/// locks. It does not perform I/O unless [`Self::evaluate_and_record`] is used
/// with an explicit decision journal.
#[derive(Debug)]
pub struct ProductionRiskEngine {
    policies: Vec<ProductionRiskPolicyState>,
    capacity: usize,
    version: u64,
}

impl ProductionRiskEngine {
    /// Creates an empty fail-closed engine with bounded policy capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            policies: Vec::with_capacity(capacity),
            capacity,
            version: 0,
        }
    }

    /// Adds a validated policy in stable priority/id order.
    ///
    /// # Errors
    ///
    /// Returns an error for duplicates, capacity exhaustion, or unsafe rate
    /// allocations.
    pub fn add_policy(&mut self, policy: ProductionRiskPolicy) -> Result<(), ProductionRiskError> {
        validate_policy(policy)?;
        if self
            .policies
            .iter()
            .any(|state| state.policy.policy_id == policy.policy_id)
        {
            return Err(ProductionRiskError::DuplicatePolicyId);
        }
        if self.policies.len() >= self.capacity {
            return Err(ProductionRiskError::PolicyCapacityExceeded);
        }
        let key = (policy.priority, policy.policy_id.as_str());
        let index = self
            .policies
            .iter()
            .position(|state| (state.policy.priority, state.policy.policy_id.as_str()) > key)
            .unwrap_or(self.policies.len());
        self.policies
            .insert(index, ProductionRiskPolicyState::new(policy));
        self.version = self.version.saturating_add(1);
        Ok(())
    }

    /// Removes a policy and its rate-window state.
    ///
    /// # Errors
    ///
    /// Returns [`ProductionRiskError::PolicyNotFound`] when absent.
    pub fn remove_policy(
        &mut self,
        policy_id: ProductionRiskPolicyId,
    ) -> Result<ProductionRiskPolicy, ProductionRiskError> {
        let index = self
            .policies
            .iter()
            .position(|state| state.policy.policy_id == policy_id)
            .ok_or(ProductionRiskError::PolicyNotFound)?;
        self.version = self.version.saturating_add(1);
        Ok(self.policies.remove(index).policy)
    }

    /// Returns policy-set version incremented by every add/remove operation.
    pub const fn version(&self) -> u64 {
        self.version
    }

    /// Returns configured policy count.
    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    /// Returns policies in deterministic evaluation order.
    pub fn policies(&self) -> impl ExactSizeIterator<Item = &ProductionRiskPolicy> {
        self.policies.iter().map(|state| &state.policy)
    }

    /// Evaluates a command and updates every matching rate window.
    pub fn evaluate(
        &mut self,
        command: ProductionRiskCommand,
        context: ProductionRiskContext,
    ) -> ProductionRiskDecision {
        let mut decision = ProductionRiskDecision::allowed(command, self.version);
        let mut matched = 0_u32;

        for state in &mut self.policies {
            if !scope_matches(state.policy.scope, &command) {
                continue;
            }
            matched = matched.saturating_add(1);
            let candidate = evaluate_policy(state, command, context, self.version);
            if decision.allowed && !candidate.allowed {
                decision = candidate;
            }
        }

        if matched == 0 {
            return ProductionRiskDecision::reject_engine(
                command,
                self.version,
                ProductionRiskReason::NoMatchingPolicy,
            );
        }
        decision.checked_policies = matched;
        decision
    }

    /// Evaluates and records a decision. Journal failure converts the returned
    /// result to a fail-closed `DecisionJournalUnavailable` rejection.
    pub fn evaluate_and_record<J: ProductionRiskDecisionJournal>(
        &mut self,
        command: ProductionRiskCommand,
        context: ProductionRiskContext,
        journal: &mut J,
    ) -> ProductionRiskDecision {
        let mut decision = self.evaluate(command, context);
        decision.journal_status = ProductionRiskJournalStatus::Recorded;
        if journal.record(&decision).is_err() {
            decision.allowed = false;
            decision.reason = ProductionRiskReason::DecisionJournalUnavailable;
            decision.journal_status = ProductionRiskJournalStatus::Failed;
        }
        decision
    }
}

fn validate_policy(policy: ProductionRiskPolicy) -> Result<(), ProductionRiskError> {
    if policy.policy_id.is_empty() {
        return Err(ProductionRiskError::EmptyPolicyId);
    }
    if policy.limits.max_order_rate_per_sec > MAX_PRODUCTION_RISK_RATE_PER_SEC
        || policy.limits.max_cancel_rate_per_sec > MAX_PRODUCTION_RISK_RATE_PER_SEC
    {
        return Err(ProductionRiskError::RateLimitTooLarge);
    }
    let limits = policy.limits;
    if limits.max_order_qty < 0
        || limits.max_order_notional < 0
        || limits.max_position_abs < 0
        || limits.max_gross_exposure < 0
        || limits.max_net_exposure_abs < 0
        || limits.price_collar_ticks < 0
        || limits.max_loss < 0
        || limits.max_daily_drawdown < 0
    {
        return Err(ProductionRiskError::InvalidLimit);
    }
    Ok(())
}

fn scope_matches(scope: ProductionRiskScope, command: &ProductionRiskCommand) -> bool {
    match scope {
        ProductionRiskScope::Global => true,
        ProductionRiskScope::Account(account) => command.account_id == account,
        ProductionRiskScope::Strategy(strategy) => command.strategy_id == strategy,
        ProductionRiskScope::Route(route) => command.route_id == route,
        ProductionRiskScope::Symbol(symbol) => command.symbol == symbol,
        ProductionRiskScope::Venue(venue) => command.symbol.venue == venue,
        ProductionRiskScope::InstrumentGroup(group) => command.instrument_group == group,
    }
}

fn evaluate_policy(
    state: &mut ProductionRiskPolicyState,
    command: ProductionRiskCommand,
    context: ProductionRiskContext,
    version: u64,
) -> ProductionRiskDecision {
    let policy = state.policy;
    let limits = policy.limits;
    let rate = match command.kind {
        ProductionRiskCommandKind::Cancel => state
            .cancel_rate
            .observe(command.timestamp_ns, limits.max_cancel_rate_per_sec),
        ProductionRiskCommandKind::Submit | ProductionRiskCommandKind::Amend => state
            .order_rate
            .observe(command.timestamp_ns, limits.max_order_rate_per_sec),
    };
    if let RateObservation::TimestampRegression(previous) = rate {
        return reject(
            command,
            version,
            policy,
            ProductionRiskReason::TimestampRegression,
            i128::from(command.timestamp_ns),
            i128::from(previous),
        );
    }
    if let RateObservation::Exceeded(observed) = rate {
        let (reason, limit) = match command.kind {
            ProductionRiskCommandKind::Cancel => (
                ProductionRiskReason::MaxCancelRate,
                limits.max_cancel_rate_per_sec,
            ),
            ProductionRiskCommandKind::Submit | ProductionRiskCommandKind::Amend => (
                ProductionRiskReason::MaxOrderRate,
                limits.max_order_rate_per_sec,
            ),
        };
        return reject(
            command,
            version,
            policy,
            reason,
            i128::from(observed),
            i128::from(limit),
        );
    }

    if command.kind == ProductionRiskCommandKind::Cancel {
        return ProductionRiskDecision::allowed(command, version);
    }
    if limits.block_unavailable_risk_state && !context.risk_state_available {
        return reject_zero(
            command,
            version,
            policy,
            ProductionRiskReason::RiskStateUnavailable,
        );
    }
    if limits.restricted {
        return reject_zero(
            command,
            version,
            policy,
            ProductionRiskReason::RestrictedScope,
        );
    }
    if let Some(window) = limits.trading_window {
        if !window.contains(command.timestamp_ns) {
            return reject_zero(
                command,
                version,
                policy,
                ProductionRiskReason::OutsideTradingWindow,
            );
        }
    }
    if context.duplicate_client_order_id {
        return reject_zero(
            command,
            version,
            policy,
            ProductionRiskReason::DuplicateClientOrderId,
        );
    }
    if limits.block_self_trade && context.self_trade_risk {
        return reject_zero(
            command,
            version,
            policy,
            ProductionRiskReason::SelfTradeRisk,
        );
    }
    if limits.block_stale_market_data && context.market_data_stale {
        return reject_zero(
            command,
            version,
            policy,
            ProductionRiskReason::MarketDataStale,
        );
    }
    if limits.block_degraded_adapter && context.adapter_degraded {
        return reject_zero(
            command,
            version,
            policy,
            ProductionRiskReason::AdapterDegraded,
        );
    }
    if limits.block_degraded_persistence && context.persistence_degraded {
        return reject_zero(
            command,
            version,
            policy,
            ProductionRiskReason::PersistenceDegraded,
        );
    }
    if limits.max_loss > 0 && context.daily_pnl <= -limits.max_loss {
        return reject(
            command,
            version,
            policy,
            ProductionRiskReason::MaxLoss,
            context.daily_pnl.saturating_neg(),
            limits.max_loss,
        );
    }
    let drawdown = context.peak_daily_pnl.saturating_sub(context.daily_pnl);
    if limits.max_daily_drawdown > 0 && drawdown >= limits.max_daily_drawdown {
        return reject(
            command,
            version,
            policy,
            ProductionRiskReason::MaxDailyDrawdown,
            drawdown,
            limits.max_daily_drawdown,
        );
    }
    if limits.max_order_qty > 0 && command.quantity.0 > limits.max_order_qty {
        return reject(
            command,
            version,
            policy,
            ProductionRiskReason::MaxOrderQty,
            i128::from(command.quantity.0),
            i128::from(limits.max_order_qty),
        );
    }

    let needs_reference = limits.max_order_notional > 0
        || limits.max_gross_exposure > 0
        || limits.max_net_exposure_abs > 0
        || limits.price_collar_ticks > 0;
    let price = if command.price.0 > 0 {
        command.price
    } else {
        context.reference_price
    };
    if needs_reference && price.0 <= 0 {
        return reject_zero(
            command,
            version,
            policy,
            ProductionRiskReason::ReferencePriceUnavailable,
        );
    }
    let notional = normalized_notional(command.quantity, price);
    if limits.max_order_notional > 0 && notional > limits.max_order_notional {
        return reject(
            command,
            version,
            policy,
            ProductionRiskReason::MaxOrderNotional,
            notional,
            limits.max_order_notional,
        );
    }
    if limits.price_collar_ticks > 0 {
        if context.reference_price.0 <= 0 {
            return reject_zero(
                command,
                version,
                policy,
                ProductionRiskReason::ReferencePriceUnavailable,
            );
        }
        if command.price.0 > 0 {
            let distance = command
                .price
                .0
                .saturating_sub(context.reference_price.0)
                .abs();
            if distance > limits.price_collar_ticks {
                return reject(
                    command,
                    version,
                    policy,
                    ProductionRiskReason::PriceCollar,
                    i128::from(distance),
                    i128::from(limits.price_collar_ticks),
                );
            }
        }
    }
    if limits.max_typical_qty_multiple_bps > 0 {
        if context.typical_order_qty.0 <= 0 {
            return reject_zero(
                command,
                version,
                policy,
                ProductionRiskReason::RiskStateUnavailable,
            );
        }
        let observed = i128::from(command.quantity.0).saturating_mul(10_000);
        let allowed = i128::from(context.typical_order_qty.0)
            .saturating_mul(i128::from(limits.max_typical_qty_multiple_bps));
        if observed > allowed {
            return reject(
                command,
                version,
                policy,
                ProductionRiskReason::FatFinger,
                observed,
                allowed,
            );
        }
    }

    let signed_delta_qty = signed_quantity_delta(command);
    let projected_position = context.current_position.saturating_add(signed_delta_qty);
    if limits.reduce_only
        && (projected_position.saturating_abs() >= context.current_position.saturating_abs()
            || crosses_zero(context.current_position, projected_position))
    {
        return reject(
            command,
            version,
            policy,
            ProductionRiskReason::ReduceOnly,
            i128::from(projected_position),
            i128::from(context.current_position),
        );
    }
    if limits.max_position_abs > 0 && projected_position.saturating_abs() > limits.max_position_abs
    {
        return reject(
            command,
            version,
            policy,
            ProductionRiskReason::MaxPosition,
            i128::from(projected_position.saturating_abs()),
            i128::from(limits.max_position_abs),
        );
    }

    let existing_notional = normalized_notional(command.existing_quantity, command.existing_price);
    let projected_gross = context
        .current_gross_exposure
        .saturating_sub(existing_notional)
        .max(0)
        .saturating_add(notional);
    if limits.max_gross_exposure > 0 && projected_gross > limits.max_gross_exposure {
        return reject(
            command,
            version,
            policy,
            ProductionRiskReason::MaxGrossExposure,
            projected_gross,
            limits.max_gross_exposure,
        );
    }

    let signed_notional_delta =
        signed_notional(command.side, notional.saturating_sub(existing_notional));
    let projected_net = context
        .current_net_exposure
        .saturating_add(signed_notional_delta);
    if limits.max_net_exposure_abs > 0
        && projected_net.saturating_abs() > limits.max_net_exposure_abs
    {
        return reject(
            command,
            version,
            policy,
            ProductionRiskReason::MaxNetExposure,
            projected_net.saturating_abs(),
            limits.max_net_exposure_abs,
        );
    }

    let projected_open_orders = context.open_orders.saturating_add(u32::from(matches!(
        command.kind,
        ProductionRiskCommandKind::Submit
    )));
    if limits.max_open_orders > 0 && projected_open_orders > limits.max_open_orders {
        return reject(
            command,
            version,
            policy,
            ProductionRiskReason::MaxOpenOrders,
            i128::from(projected_open_orders),
            i128::from(limits.max_open_orders),
        );
    }

    ProductionRiskDecision::allowed(command, version)
}

fn normalized_notional(quantity: OrderQty, price: OrderPrice) -> i128 {
    i128::from(quantity.0.max(0)).saturating_mul(i128::from(price.0.max(0)))
}

fn signed_quantity_delta(command: ProductionRiskCommand) -> i64 {
    let delta = command
        .quantity
        .0
        .saturating_sub(command.existing_quantity.0);
    match command.side {
        OrderSide::Buy => delta,
        OrderSide::Sell => delta.saturating_neg(),
    }
}

fn signed_notional(side: OrderSide, value: i128) -> i128 {
    match side {
        OrderSide::Buy => value,
        OrderSide::Sell => value.saturating_neg(),
    }
}

fn crosses_zero(current: i64, projected: i64) -> bool {
    (current < 0 && projected > 0) || (current > 0 && projected < 0)
}

fn reject_zero(
    command: ProductionRiskCommand,
    version: u64,
    policy: ProductionRiskPolicy,
    reason: ProductionRiskReason,
) -> ProductionRiskDecision {
    reject(command, version, policy, reason, 0, 0)
}

fn reject(
    command: ProductionRiskCommand,
    version: u64,
    policy: ProductionRiskPolicy,
    reason: ProductionRiskReason,
    observed: i128,
    limit: i128,
) -> ProductionRiskDecision {
    ProductionRiskDecision::reject_policy(command, version, policy, reason, observed, limit)
}

#[cfg(test)]
mod tests {
    use of_execution_core::{ExecutionSymbol, TimeInForce};

    use super::*;

    fn fixed<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn request(id: &str, side: OrderSide, qty: i64, price: i64, timestamp_ns: u64) -> OrderRequest {
        OrderRequest {
            client_order_id: fixed(id),
            account_id: fixed("account-a"),
            route_id: fixed("route-a"),
            strategy_id: fixed("strategy-a"),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            side,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(qty),
            limit_price: OrderPrice(price),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 0,
            ts_recv_ns: timestamp_ns,
        }
    }

    fn command(id: &str) -> ProductionRiskCommand {
        ProductionRiskCommand::submit(
            &request(id, OrderSide::Buy, 2, 5_000, NANOS_PER_SECOND),
            fixed("equity-index"),
        )
    }

    fn policy(id: &str, priority: u16, limits: ProductionRiskLimits) -> ProductionRiskPolicy {
        ProductionRiskPolicy::from_id(id, ProductionRiskScope::Global, priority, limits).unwrap()
    }

    fn engine(limits: ProductionRiskLimits) -> ProductionRiskEngine {
        let mut engine = ProductionRiskEngine::with_capacity(8);
        engine.add_policy(policy("global", 100, limits)).unwrap();
        engine
    }

    #[test]
    fn empty_engine_fails_closed() {
        let mut engine = ProductionRiskEngine::with_capacity(1);
        let decision = engine.evaluate(command("c1"), ProductionRiskContext::available());
        assert!(!decision.allowed);
        assert_eq!(decision.reason, ProductionRiskReason::NoMatchingPolicy);
    }

    #[test]
    fn scope_policies_evaluate_in_priority_then_id_order() {
        let mut engine = ProductionRiskEngine::with_capacity(8);
        let scopes = [
            ProductionRiskScope::Global,
            ProductionRiskScope::Account(fixed("account-a")),
            ProductionRiskScope::Strategy(fixed("strategy-a")),
            ProductionRiskScope::Route(fixed("route-a")),
            ProductionRiskScope::Symbol(ExecutionSymbol::new("XCME", "ESM6").unwrap()),
            ProductionRiskScope::Venue(fixed("XCME")),
            ProductionRiskScope::InstrumentGroup(fixed("equity-index")),
        ];
        for (index, scope) in scopes.into_iter().enumerate() {
            engine
                .add_policy(
                    ProductionRiskPolicy::from_id(
                        &format!("p-{index}"),
                        scope,
                        index as u16,
                        ProductionRiskLimits::permissive(),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        let decision = engine.evaluate(command("c1"), ProductionRiskContext::available());
        assert!(decision.allowed);
        assert_eq!(decision.checked_policies, 7);
        let priorities = engine
            .policies()
            .map(|item| item.priority)
            .collect::<Vec<_>>();
        assert_eq!(priorities, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn operational_guards_fail_closed() {
        let limits = ProductionRiskLimits::conservative();
        let cases = [
            (
                ProductionRiskContext::default(),
                ProductionRiskReason::RiskStateUnavailable,
            ),
            (
                ProductionRiskContext {
                    market_data_stale: true,
                    ..ProductionRiskContext::available()
                },
                ProductionRiskReason::MarketDataStale,
            ),
            (
                ProductionRiskContext {
                    adapter_degraded: true,
                    ..ProductionRiskContext::available()
                },
                ProductionRiskReason::AdapterDegraded,
            ),
            (
                ProductionRiskContext {
                    persistence_degraded: true,
                    ..ProductionRiskContext::available()
                },
                ProductionRiskReason::PersistenceDegraded,
            ),
        ];
        for (context, expected) in cases {
            assert_eq!(
                engine(limits).evaluate(command("c1"), context).reason,
                expected
            );
        }
    }

    #[test]
    fn restricted_duplicate_and_self_trade_checks_are_explainable() {
        let mut restricted = ProductionRiskLimits::permissive();
        restricted.restricted = true;
        assert_eq!(
            engine(restricted)
                .evaluate(command("c1"), ProductionRiskContext::available())
                .reason,
            ProductionRiskReason::RestrictedScope
        );

        let duplicate = ProductionRiskContext {
            duplicate_client_order_id: true,
            ..ProductionRiskContext::available()
        };
        assert_eq!(
            engine(ProductionRiskLimits::permissive())
                .evaluate(command("c1"), duplicate)
                .reason,
            ProductionRiskReason::DuplicateClientOrderId
        );

        let mut self_trade_limits = ProductionRiskLimits::permissive();
        self_trade_limits.block_self_trade = true;
        let self_trade = ProductionRiskContext {
            self_trade_risk: true,
            ..ProductionRiskContext::available()
        };
        assert_eq!(
            engine(self_trade_limits)
                .evaluate(command("c1"), self_trade)
                .reason,
            ProductionRiskReason::SelfTradeRisk
        );
    }

    #[test]
    fn order_size_notional_and_price_collar_reject() {
        let mut limits = ProductionRiskLimits::permissive();
        limits.max_order_qty = 1;
        let decision = engine(limits).evaluate(command("c1"), ProductionRiskContext::available());
        assert_eq!(decision.reason, ProductionRiskReason::MaxOrderQty);
        assert_eq!(decision.observed, 2);

        limits.max_order_qty = 0;
        limits.max_order_notional = 9_000;
        let decision = engine(limits).evaluate(command("c1"), ProductionRiskContext::available());
        assert_eq!(decision.reason, ProductionRiskReason::MaxOrderNotional);

        limits.max_order_notional = 0;
        limits.price_collar_ticks = 10;
        let context = ProductionRiskContext {
            reference_price: OrderPrice(4_980),
            ..ProductionRiskContext::available()
        };
        assert_eq!(
            engine(limits).evaluate(command("c1"), context).reason,
            ProductionRiskReason::PriceCollar
        );
    }

    #[test]
    fn position_gross_net_and_open_order_limits_use_projected_state() {
        let mut limits = ProductionRiskLimits::permissive();
        limits.max_position_abs = 5;
        let context = ProductionRiskContext {
            current_position: 4,
            ..ProductionRiskContext::available()
        };
        assert_eq!(
            engine(limits).evaluate(command("c1"), context).reason,
            ProductionRiskReason::MaxPosition
        );

        limits.max_position_abs = 0;
        limits.max_gross_exposure = 15_000;
        let context = ProductionRiskContext {
            current_gross_exposure: 6_000,
            ..ProductionRiskContext::available()
        };
        assert_eq!(
            engine(limits).evaluate(command("c1"), context).reason,
            ProductionRiskReason::MaxGrossExposure
        );

        limits.max_gross_exposure = 0;
        limits.max_net_exposure_abs = 15_000;
        let context = ProductionRiskContext {
            current_net_exposure: 6_000,
            ..ProductionRiskContext::available()
        };
        assert_eq!(
            engine(limits).evaluate(command("c1"), context).reason,
            ProductionRiskReason::MaxNetExposure
        );

        limits.max_net_exposure_abs = 0;
        limits.max_open_orders = 2;
        let context = ProductionRiskContext {
            open_orders: 2,
            ..ProductionRiskContext::available()
        };
        assert_eq!(
            engine(limits).evaluate(command("c1"), context).reason,
            ProductionRiskReason::MaxOpenOrders
        );
    }

    #[test]
    fn reducing_amend_releases_existing_gross_and_net_exposure() {
        let request = AmendRequest {
            client_order_id: fixed("replace-1"),
            orig_client_order_id: fixed("c1"),
            venue_order_id: fixed("venue-1"),
            account_id: fixed("account-a"),
            route_id: fixed("route-a"),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            quantity: OrderQty(1),
            limit_price: OrderPrice(5_000),
            ts_recv_ns: NANOS_PER_SECOND,
        };
        let command = ProductionRiskCommand::amend(
            &request,
            fixed("strategy-a"),
            OrderSide::Buy,
            OrderType::Limit,
            fixed("equity-index"),
            OrderQty(2),
            OrderPrice(5_000),
        );
        let mut limits = ProductionRiskLimits::permissive();
        limits.max_gross_exposure = 7_500;
        limits.max_net_exposure_abs = 7_500;
        let context = ProductionRiskContext {
            current_gross_exposure: 10_000,
            current_net_exposure: 10_000,
            ..ProductionRiskContext::available()
        };
        assert!(engine(limits).evaluate(command, context).allowed);
    }

    #[test]
    fn fat_finger_and_reduce_only_use_caller_state() {
        let mut limits = ProductionRiskLimits::permissive();
        limits.max_typical_qty_multiple_bps = 15_000;
        let context = ProductionRiskContext {
            typical_order_qty: OrderQty(1),
            ..ProductionRiskContext::available()
        };
        assert_eq!(
            engine(limits).evaluate(command("c1"), context).reason,
            ProductionRiskReason::FatFinger
        );

        limits.max_typical_qty_multiple_bps = 0;
        limits.reduce_only = true;
        let sell = ProductionRiskCommand::submit(
            &request("sell", OrderSide::Sell, 2, 5_000, NANOS_PER_SECOND),
            fixed("equity-index"),
        );
        let context = ProductionRiskContext {
            current_position: 5,
            ..ProductionRiskContext::available()
        };
        assert!(engine(limits).evaluate(sell, context).allowed);
        assert_eq!(
            engine(limits).evaluate(command("buy"), context).reason,
            ProductionRiskReason::ReduceOnly
        );
    }

    #[test]
    fn pnl_and_session_limits_reject_at_boundary() {
        let mut limits = ProductionRiskLimits::permissive();
        limits.max_loss = 100;
        let context = ProductionRiskContext {
            daily_pnl: -100,
            ..ProductionRiskContext::available()
        };
        assert_eq!(
            engine(limits).evaluate(command("c1"), context).reason,
            ProductionRiskReason::MaxLoss
        );

        limits.max_loss = 0;
        limits.max_daily_drawdown = 50;
        let context = ProductionRiskContext {
            daily_pnl: 25,
            peak_daily_pnl: 75,
            ..ProductionRiskContext::available()
        };
        assert_eq!(
            engine(limits).evaluate(command("c1"), context).reason,
            ProductionRiskReason::MaxDailyDrawdown
        );

        limits.max_daily_drawdown = 0;
        limits.trading_window = Some(RiskTradingWindow::new(2_000, 3_000).unwrap());
        assert_eq!(
            engine(limits)
                .evaluate(command("c1"), ProductionRiskContext::available())
                .reason,
            ProductionRiskReason::OutsideTradingWindow
        );
    }

    #[test]
    fn trading_windows_support_day_overnight_and_full_day_sessions() {
        let day = RiskTradingWindow::new(1_000, 2_000).unwrap();
        assert!(day.contains(1_000));
        assert!(day.contains(1_999));
        assert!(!day.contains(2_000));

        let overnight = RiskTradingWindow::new(NANOS_PER_DAY - 1_000, 1_000).unwrap();
        assert!(overnight.contains(NANOS_PER_DAY - 1));
        assert!(overnight.contains(999));
        assert!(!overnight.contains(1_000));

        let full_day = RiskTradingWindow::new(10_000, 10_000).unwrap();
        assert!(full_day.contains(0));
        assert!(full_day.contains(NANOS_PER_DAY - 1));
        assert_eq!(
            RiskTradingWindow::new(NANOS_PER_DAY, 0),
            Err(ProductionRiskError::InvalidTradingWindow)
        );
    }

    #[test]
    fn order_and_cancel_rates_are_independent_and_timestamp_safe() {
        let mut limits = ProductionRiskLimits::permissive();
        limits.max_order_rate_per_sec = 1;
        limits.max_cancel_rate_per_sec = 1;
        let mut engine = engine(limits);
        assert!(
            engine
                .evaluate(command("c1"), ProductionRiskContext::available())
                .allowed
        );
        let mut second = command("c2");
        second.timestamp_ns += 1;
        assert_eq!(
            engine
                .evaluate(second, ProductionRiskContext::available())
                .reason,
            ProductionRiskReason::MaxOrderRate
        );

        let cancel_request = CancelRequest {
            client_order_id: fixed("cancel-1"),
            orig_client_order_id: fixed("c1"),
            venue_order_id: fixed("venue-1"),
            account_id: fixed("account-a"),
            route_id: fixed("route-a"),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            ts_recv_ns: NANOS_PER_SECOND,
        };
        let cancel = ProductionRiskCommand::cancel(
            &cancel_request,
            fixed("strategy-a"),
            fixed("equity-index"),
        );
        assert!(
            engine
                .evaluate(cancel, ProductionRiskContext::default())
                .allowed
        );
        let mut second_cancel = cancel;
        second_cancel.client_order_id = fixed("cancel-2");
        second_cancel.timestamp_ns += 1;
        assert_eq!(
            engine
                .evaluate(second_cancel, ProductionRiskContext::default())
                .reason,
            ProductionRiskReason::MaxCancelRate
        );

        let mut regressing = command("c3");
        regressing.timestamp_ns = NANOS_PER_SECOND - 1;
        let decision = engine.evaluate(regressing, ProductionRiskContext::available());
        assert_eq!(decision.reason, ProductionRiskReason::TimestampRegression);
        assert_eq!(decision.observed, i128::from(NANOS_PER_SECOND - 1));
        assert_eq!(decision.limit, i128::from(NANOS_PER_SECOND + 1));
    }

    #[test]
    fn journal_failure_forces_rejection() {
        let mut engine = engine(ProductionRiskLimits::permissive());
        let mut journal = InMemoryProductionRiskJournal::with_capacity(1);
        let decision = engine.evaluate_and_record(
            command("c1"),
            ProductionRiskContext::available(),
            &mut journal,
        );
        assert!(decision.allowed);
        assert_eq!(
            decision.journal_status,
            ProductionRiskJournalStatus::Recorded
        );
        assert_eq!(journal.decisions().len(), 1);

        let decision = engine.evaluate_and_record(
            command("c2"),
            ProductionRiskContext::available(),
            &mut journal,
        );
        assert!(!decision.allowed);
        assert_eq!(
            decision.reason,
            ProductionRiskReason::DecisionJournalUnavailable
        );
        assert_eq!(decision.journal_status, ProductionRiskJournalStatus::Failed);
    }

    #[test]
    fn policy_configuration_is_bounded_and_versioned() {
        let mut engine = ProductionRiskEngine::with_capacity(1);
        let global_policy = policy("global", 10, ProductionRiskLimits::permissive());
        engine.add_policy(global_policy).unwrap();
        assert_eq!(engine.version(), 1);
        assert_eq!(
            engine.add_policy(global_policy),
            Err(ProductionRiskError::DuplicatePolicyId)
        );
        assert_eq!(
            engine.add_policy(policy("other", 20, ProductionRiskLimits::permissive())),
            Err(ProductionRiskError::PolicyCapacityExceeded)
        );
        assert_eq!(
            engine.remove_policy(global_policy.policy_id).unwrap(),
            global_policy
        );
        assert_eq!(engine.version(), 2);
    }

    #[test]
    fn policy_configuration_rejects_ambiguous_or_negative_limits() {
        let mut engine = ProductionRiskEngine::with_capacity(2);
        let empty = ProductionRiskPolicy::new(
            ProductionRiskPolicyId::empty(),
            ProductionRiskScope::Global,
            0,
            ProductionRiskLimits::permissive(),
        );
        assert_eq!(
            engine.add_policy(empty),
            Err(ProductionRiskError::EmptyPolicyId)
        );

        let mut limits = ProductionRiskLimits::permissive();
        limits.max_loss = -1;
        assert_eq!(
            engine.add_policy(policy("invalid", 0, limits)),
            Err(ProductionRiskError::InvalidLimit)
        );

        limits.max_loss = 0;
        limits.max_order_rate_per_sec = MAX_PRODUCTION_RISK_RATE_PER_SEC + 1;
        assert_eq!(
            engine.add_policy(policy("oversized-rate", 0, limits)),
            Err(ProductionRiskError::RateLimitTooLarge)
        );
    }
}
