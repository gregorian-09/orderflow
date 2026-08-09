//! Authoritative, checkpointable position and PnL ledger primitives.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use of_execution_core::{
    AccountId, ClientOrderId, ExecutionEvent, ExecutionId, ExecutionSymbol, ExecutionType,
    FixedAscii, OrderPrice, OrderQty, OrderSide, RouteId, StrategyId, VenueOrderId,
};

/// Maximum bytes stored in a settlement or reporting currency code.
pub const LEDGER_CURRENCY_CAPACITY: usize = 12;
/// Maximum bytes stored in a ledger adjustment identifier.
pub const LEDGER_ADJUSTMENT_ID_CAPACITY: usize = 48;
/// Current binary position-ledger checkpoint schema.
pub const POSITION_LEDGER_CHECKPOINT_VERSION: u16 = 1;

const CHECKPOINT_MAGIC: u32 = 0x4744_4c50;
const CHECKPOINT_EXTENSION: &str = "ofplchk";
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Settlement or reporting currency identifier, such as `USD` or `USDT`.
pub type LedgerCurrency = FixedAscii<LEDGER_CURRENCY_CAPACITY>;
/// Stable identifier for a manual, corporate-action, or correction mutation.
pub type LedgerAdjustmentId = FixedAscii<LEDGER_ADJUSTMENT_ID_CAPACITY>;

/// Position ownership and valuation key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ProductionPositionKey {
    /// Trading account.
    pub account_id: AccountId,
    /// Strategy attribution.
    pub strategy_id: StrategyId,
    /// Venue-native symbol.
    pub symbol: ExecutionSymbol,
    /// Settlement/reporting currency for all money fields.
    pub currency: LedgerCurrency,
}

impl ProductionPositionKey {
    /// Creates a position key.
    pub const fn new(
        account_id: AccountId,
        strategy_id: StrategyId,
        symbol: ExecutionSymbol,
        currency: LedgerCurrency,
    ) -> Self {
        Self {
            account_id,
            strategy_id,
            symbol,
            currency,
        }
    }
}

/// Provider/session-scoped execution identity used for fill deduplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct LedgerExecutionIdentity {
    /// Execution route/session namespace.
    pub route_id: RouteId,
    /// Trading account namespace.
    pub account_id: AccountId,
    /// Venue-native symbol namespace.
    pub symbol: ExecutionSymbol,
    /// Provider execution id.
    pub execution_id: ExecutionId,
}

impl LedgerExecutionIdentity {
    /// Creates a scoped execution identity from a ledger fill.
    pub const fn from_fill(fill: &LedgerFill) -> Self {
        Self {
            route_id: fill.route_id,
            account_id: fill.key.account_id,
            symbol: fill.key.symbol,
            execution_id: fill.execution_id,
        }
    }
}

/// Position-scoped adjustment identity used for adjustment deduplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct LedgerScopedAdjustmentId {
    /// Position key receiving the adjustment.
    pub key: ProductionPositionKey,
    /// Host adjustment id.
    pub adjustment_id: LedgerAdjustmentId,
}

/// Positive rational conversion from local money units to base money units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LedgerFxRate {
    numerator: i128,
    denominator: i128,
}

impl LedgerFxRate {
    /// Creates a positive rational FX rate.
    ///
    /// # Errors
    ///
    /// Returns [`PositionLedgerError::InvalidFxRate`] for non-positive values.
    pub const fn new(numerator: i128, denominator: i128) -> Result<Self, PositionLedgerError> {
        if numerator <= 0 || denominator <= 0 {
            return Err(PositionLedgerError::InvalidFxRate);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// Returns the rate numerator.
    pub const fn numerator(self) -> i128 {
        self.numerator
    }

    /// Returns the rate denominator.
    pub const fn denominator(self) -> i128 {
        self.denominator
    }

    /// Converts local money units to base money units with truncation toward zero.
    pub const fn convert(self, local: i128) -> i128 {
        local.saturating_mul(self.numerator) / self.denominator
    }
}

/// Validated canonical fill consumed by [`ProductionPositionLedger`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LedgerFill {
    /// Globally ordered host/WAL mutation sequence.
    pub sequence: u64,
    /// Venue execution id used for duplicate suppression.
    pub execution_id: ExecutionId,
    /// Current client order id.
    pub client_order_id: ClientOrderId,
    /// Venue order id when known.
    pub venue_order_id: VenueOrderId,
    /// Route producing the fill.
    pub route_id: RouteId,
    /// Position key.
    pub key: ProductionPositionKey,
    /// Fill side.
    pub side: OrderSide,
    /// Positive last-fill quantity.
    pub quantity: OrderQty,
    /// Positive last-fill price in normalized ticks/money units.
    pub price: OrderPrice,
    /// Positive contract multiplier; use `1` for spot/equity units.
    pub contract_multiplier: i64,
    /// Non-negative commission in the key currency.
    pub commission: i128,
    /// Non-negative exchange, clearing, regulatory, and other fees.
    pub fees: i128,
    /// Exchange timestamp when available.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp.
    pub ts_recv_ns: u64,
}

impl LedgerFill {
    /// Maps a trade execution event into a ledger fill with host attribution.
    ///
    /// # Errors
    ///
    /// Returns a validation error unless the event is a trade with non-empty
    /// execution id, positive quantity/price/multiplier, and non-negative costs.
    #[allow(clippy::too_many_arguments, reason = "fill economics stay explicit")]
    pub fn from_execution_event(
        event: &ExecutionEvent,
        sequence: u64,
        strategy_id: StrategyId,
        side: OrderSide,
        currency: LedgerCurrency,
        contract_multiplier: i64,
        commission: i128,
        fees: i128,
    ) -> Result<Self, PositionLedgerError> {
        let fill = Self {
            sequence,
            execution_id: event.execution_id,
            client_order_id: event.client_order_id,
            venue_order_id: event.venue_order_id,
            route_id: event.route_id,
            key: ProductionPositionKey::new(event.account_id, strategy_id, event.symbol, currency),
            side,
            quantity: event.last_qty,
            price: event.last_price,
            contract_multiplier,
            commission,
            fees,
            ts_exchange_ns: event.ts_exchange_ns,
            ts_recv_ns: event.ts_recv_ns,
        };
        if event.exec_type != ExecutionType::Trade {
            return Err(PositionLedgerError::NotTradeEvent);
        }
        fill.validate()?;
        Ok(fill)
    }

    fn validate(self) -> Result<(), PositionLedgerError> {
        if self.sequence == 0 {
            return Err(PositionLedgerError::InvalidSequence);
        }
        if self.execution_id.is_empty() {
            return Err(PositionLedgerError::MissingMutationId);
        }
        if self.key.currency.is_empty() {
            return Err(PositionLedgerError::MissingCurrency);
        }
        if self.quantity.0 <= 0 {
            return Err(PositionLedgerError::InvalidQuantity);
        }
        if self.price.0 <= 0 {
            return Err(PositionLedgerError::InvalidPrice);
        }
        if self.contract_multiplier <= 0 {
            return Err(PositionLedgerError::InvalidContractMultiplier);
        }
        if self.commission < 0 || self.fees < 0 {
            return Err(PositionLedgerError::InvalidCosts);
        }
        Ok(())
    }
}

/// Fill attribution retained in the bounded recent-mutation window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LedgerFillAttribution {
    /// Host/WAL sequence.
    pub sequence: u64,
    /// Venue execution identity.
    pub execution_id: ExecutionId,
    /// Current client order identity.
    pub client_order_id: ClientOrderId,
    /// Venue order identity.
    pub venue_order_id: VenueOrderId,
    /// Execution route.
    pub route_id: RouteId,
    /// Position key.
    pub key: ProductionPositionKey,
    /// Fill side.
    pub side: OrderSide,
    /// Fill quantity.
    pub quantity: OrderQty,
    /// Fill price.
    pub price: OrderPrice,
    /// Gross realized PnL produced by this fill.
    pub realized_pnl_delta: i128,
    /// Commission charged by this fill.
    pub commission: i128,
    /// Other fees charged by this fill.
    pub fees: i128,
    /// Local receive timestamp.
    pub ts_recv_ns: u64,
}

/// Auditable non-fill mutation kind.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LedgerAdjustmentKind {
    /// Opening balance imported from an authoritative source.
    OpeningBalance = 0,
    /// Broker/venue trade correction or bust effect.
    TradeCorrection = 1,
    /// Corporate action such as split, merger, or distribution.
    CorporateAction = 2,
    /// Explicitly authorized manual adjustment.
    Manual = 3,
    /// Cash-only dividend, funding, interest, or settlement adjustment.
    Cash = 4,
}

/// Explicit position/PnL adjustment supplied by an authorized host path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LedgerAdjustment {
    /// Globally ordered host/WAL mutation sequence.
    pub sequence: u64,
    /// Stable adjustment id used for duplicate suppression.
    pub adjustment_id: LedgerAdjustmentId,
    /// Adjustment classification.
    pub kind: LedgerAdjustmentKind,
    /// Position key.
    pub key: ProductionPositionKey,
    /// Signed quantity delta.
    pub quantity_delta: i64,
    /// Optional positive average-price replacement after quantity adjustment.
    pub average_price_override: Option<OrderPrice>,
    /// Gross realized PnL delta.
    pub realized_pnl_delta: i128,
    /// Commission delta; negative values permit audited corrections.
    pub commission_delta: i128,
    /// Fee delta; negative values permit audited corrections.
    pub fee_delta: i128,
    /// Cash-balance delta in key currency.
    pub cash_delta: i128,
    /// Positive multiplier replacement, or zero to retain current value.
    pub contract_multiplier_override: i64,
    /// Host timestamp.
    pub timestamp_ns: u64,
}

impl LedgerAdjustment {
    /// Creates a zero-delta adjustment for explicit builder configuration.
    pub const fn new(
        sequence: u64,
        adjustment_id: LedgerAdjustmentId,
        kind: LedgerAdjustmentKind,
        key: ProductionPositionKey,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            sequence,
            adjustment_id,
            kind,
            key,
            quantity_delta: 0,
            average_price_override: None,
            realized_pnl_delta: 0,
            commission_delta: 0,
            fee_delta: 0,
            cash_delta: 0,
            contract_multiplier_override: 0,
            timestamp_ns,
        }
    }

    /// Sets signed quantity delta and optional resulting average price.
    pub const fn with_position(
        mut self,
        quantity_delta: i64,
        average_price_override: Option<OrderPrice>,
    ) -> Self {
        self.quantity_delta = quantity_delta;
        self.average_price_override = average_price_override;
        self
    }

    /// Sets gross realized PnL delta.
    pub const fn with_realized_pnl(mut self, realized_pnl_delta: i128) -> Self {
        self.realized_pnl_delta = realized_pnl_delta;
        self
    }

    /// Sets commission and other-fee deltas.
    pub const fn with_costs(mut self, commission_delta: i128, fee_delta: i128) -> Self {
        self.commission_delta = commission_delta;
        self.fee_delta = fee_delta;
        self
    }

    /// Sets cash-balance delta.
    pub const fn with_cash(mut self, cash_delta: i128) -> Self {
        self.cash_delta = cash_delta;
        self
    }

    /// Sets a positive resulting contract multiplier.
    pub const fn with_contract_multiplier(mut self, contract_multiplier: i64) -> Self {
        self.contract_multiplier_override = contract_multiplier;
        self
    }
}

/// Position mark used for unrealized PnL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LedgerMark {
    /// Globally ordered host/WAL mutation sequence.
    pub sequence: u64,
    /// Position key.
    pub key: ProductionPositionKey,
    /// Positive mark price.
    pub price: OrderPrice,
    /// Mark timestamp.
    pub timestamp_ns: u64,
}

impl LedgerMark {
    /// Creates a mark-to-market mutation.
    pub const fn new(
        sequence: u64,
        key: ProductionPositionKey,
        price: OrderPrice,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            sequence,
            key,
            price,
            timestamp_ns,
        }
    }
}

/// Authoritative local position/PnL state in one settlement currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ProductionPosition {
    /// Net signed quantity.
    pub net_qty: i64,
    /// Cumulative bought quantity.
    pub buy_qty: i64,
    /// Cumulative sold quantity.
    pub sell_qty: i64,
    /// Average cost of the open net position.
    pub average_price: i64,
    /// Exact absolute normalized cost basis of the open position.
    pub open_cost: i128,
    /// Last mark price, or zero when unavailable.
    pub mark_price: i64,
    /// Positive contract multiplier.
    pub contract_multiplier: i64,
    /// Gross realized PnL before commission and fees.
    pub realized_pnl: i128,
    /// Mark-to-market unrealized PnL.
    pub unrealized_pnl: i128,
    /// Cumulative commissions.
    pub commissions: i128,
    /// Cumulative non-commission fees.
    pub fees: i128,
    /// Cash adjustments such as dividends, funding, or interest.
    pub cash_balance: i128,
    /// Cumulative gross traded notional.
    pub gross_traded_notional: i128,
    /// Last applied global ledger sequence.
    pub last_sequence: u64,
    /// Last fill/adjustment/mark timestamp.
    pub updated_ns: u64,
}

impl ProductionPosition {
    /// Returns absolute open notional at average cost.
    pub fn open_notional(self) -> i128 {
        self.open_cost
    }

    /// Returns realized PnL net of commissions and fees, including cash adjustments.
    pub fn net_realized_pnl(self) -> i128 {
        self.realized_pnl
            .saturating_sub(self.commissions)
            .saturating_sub(self.fees)
            .saturating_add(self.cash_balance)
    }

    /// Returns current total PnL in local currency.
    pub fn total_pnl(self) -> i128 {
        self.net_realized_pnl().saturating_add(self.unrealized_pnl)
    }

    /// Returns current total PnL converted through a caller-supplied FX rate.
    pub fn total_pnl_in_base(self, rate: LedgerFxRate) -> i128 {
        rate.convert(self.total_pnl())
    }
}

/// Bounded ledger sizing and duplicate-retention configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProductionPositionLedgerConfig {
    /// Maximum distinct position keys.
    pub position_capacity: usize,
    /// Maximum fill/adjustment identities retained for session deduplication.
    pub mutation_identity_capacity: usize,
    /// Number of recent fill attribution records retained for inspection.
    pub attribution_capacity: usize,
}

impl ProductionPositionLedgerConfig {
    /// Creates explicit bounded ledger configuration.
    pub const fn new(
        position_capacity: usize,
        mutation_identity_capacity: usize,
        attribution_capacity: usize,
    ) -> Self {
        Self {
            position_capacity,
            mutation_identity_capacity,
            attribution_capacity,
        }
    }
}

impl Default for ProductionPositionLedgerConfig {
    fn default() -> Self {
        Self::new(4_096, 65_536, 16_384)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MutationIdentity {
    Fill(LedgerExecutionIdentity),
    Adjustment(LedgerScopedAdjustmentId),
}

/// Result classification for one ledger mutation.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LedgerApplyStatus {
    /// Mutation changed authoritative state.
    Applied = 0,
    /// Mutation id was already retained and state was not changed.
    Duplicate = 1,
}

/// Explainable result of applying one fill or adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LedgerApplyResult {
    /// Apply/duplicate classification.
    pub status: LedgerApplyStatus,
    /// Position key.
    pub key: ProductionPositionKey,
    /// Mutation sequence.
    pub sequence: u64,
    /// Gross realized PnL delta.
    pub realized_pnl_delta: i128,
    /// Position after the mutation, or unchanged position for a duplicate.
    pub position: ProductionPosition,
}

/// Position-ledger validation, capacity, ordering, and persistence error.
#[derive(Debug)]
#[non_exhaustive]
pub enum PositionLedgerError {
    /// Input execution event is not a trade.
    NotTradeEvent,
    /// Quantity is not positive where required.
    InvalidQuantity,
    /// Price is not positive where required.
    InvalidPrice,
    /// Contract multiplier is not positive where required.
    InvalidContractMultiplier,
    /// Commission or fee input is invalid.
    InvalidCosts,
    /// Currency identifier is empty.
    MissingCurrency,
    /// Fill or adjustment id is empty.
    MissingMutationId,
    /// Mutation sequence is zero.
    InvalidSequence,
    /// Mutation sequence did not advance the authoritative ledger.
    SequenceRegression {
        /// Last successfully applied sequence.
        previous: u64,
        /// Regressing or repeated sequence received.
        received: u64,
    },
    /// Position-key capacity is exhausted.
    PositionCapacityExceeded,
    /// Mutation-identity capacity is exhausted; applying would lose idempotency.
    MutationIdentityCapacityExceeded,
    /// Requested position key does not exist.
    PositionNotFound,
    /// Existing position uses a different contract multiplier.
    ContractMultiplierMismatch,
    /// Adjustment would cross through zero without an average-price override.
    AdjustmentPriceRequired,
    /// Adjustment would make accumulated commission or fees negative.
    NegativeAccumulatedCosts,
    /// FX numerator or denominator is not positive.
    InvalidFxRate,
    /// Checkpoint schema, checksum, or payload is invalid.
    InvalidCheckpoint,
    /// Requested checkpoint does not match ledger capacity constraints.
    CheckpointCapacityExceeded,
    /// Caller-owned reconciliation output is full.
    ReconciliationBufferFull,
    /// File-backed checkpoint I/O failed.
    Io(std::io::Error),
}

impl fmt::Display for PositionLedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotTradeEvent => write!(f, "execution event is not a trade"),
            Self::InvalidQuantity => write!(f, "ledger quantity must be positive"),
            Self::InvalidPrice => write!(f, "ledger price must be positive"),
            Self::InvalidContractMultiplier => write!(f, "contract multiplier must be positive"),
            Self::InvalidCosts => write!(f, "ledger commission and fees must be non-negative"),
            Self::MissingCurrency => write!(f, "ledger currency is empty"),
            Self::MissingMutationId => write!(f, "ledger mutation id is empty"),
            Self::InvalidSequence => write!(f, "ledger sequence must be nonzero"),
            Self::SequenceRegression { previous, received } => {
                write!(f, "ledger sequence regressed from {previous} to {received}")
            }
            Self::PositionCapacityExceeded => write!(f, "ledger position capacity exceeded"),
            Self::MutationIdentityCapacityExceeded => {
                write!(f, "ledger mutation identity capacity exceeded")
            }
            Self::PositionNotFound => write!(f, "ledger position was not found"),
            Self::ContractMultiplierMismatch => write!(f, "ledger contract multiplier mismatch"),
            Self::AdjustmentPriceRequired => write!(f, "adjustment requires an average price"),
            Self::NegativeAccumulatedCosts => write!(f, "adjustment makes costs negative"),
            Self::InvalidFxRate => write!(f, "ledger FX rate must be positive"),
            Self::InvalidCheckpoint => write!(f, "position ledger checkpoint is invalid"),
            Self::CheckpointCapacityExceeded => write!(f, "checkpoint exceeds ledger capacity"),
            Self::ReconciliationBufferFull => write!(f, "ledger reconciliation buffer is full"),
            Self::Io(err) => write!(f, "position ledger checkpoint I/O failed: {err}"),
        }
    }
}

impl Error for PositionLedgerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PositionLedgerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Bounded authoritative average-cost position and PnL ledger.
#[derive(Debug, Clone)]
pub struct ProductionPositionLedger {
    config: ProductionPositionLedgerConfig,
    positions: HashMap<ProductionPositionKey, ProductionPosition>,
    recent_identities: HashSet<MutationIdentity>,
    identity_order: VecDeque<MutationIdentity>,
    attributions: VecDeque<LedgerFillAttribution>,
    last_sequence: u64,
}

impl ProductionPositionLedger {
    /// Creates an empty ledger with pre-sized bounded state.
    pub fn new(config: ProductionPositionLedgerConfig) -> Self {
        Self {
            positions: HashMap::with_capacity(config.position_capacity),
            recent_identities: HashSet::with_capacity(config.mutation_identity_capacity),
            identity_order: VecDeque::with_capacity(config.mutation_identity_capacity),
            attributions: VecDeque::with_capacity(config.attribution_capacity),
            config,
            last_sequence: 0,
        }
    }

    /// Returns ledger sizing configuration.
    pub const fn config(&self) -> ProductionPositionLedgerConfig {
        self.config
    }

    /// Returns the last applied authoritative mutation sequence.
    pub const fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    /// Returns distinct position count.
    pub fn position_count(&self) -> usize {
        self.positions.len()
    }

    /// Returns one position snapshot.
    pub fn position(&self, key: &ProductionPositionKey) -> Option<ProductionPosition> {
        self.positions.get(key).copied()
    }

    /// Returns all positions in unspecified map order.
    pub fn positions(
        &self,
    ) -> impl ExactSizeIterator<Item = (&ProductionPositionKey, &ProductionPosition)> {
        self.positions.iter()
    }

    /// Returns retained recent fill attribution in oldest-to-newest order.
    pub fn recent_attributions(&self) -> impl ExactSizeIterator<Item = &LedgerFillAttribution> {
        self.attributions.iter()
    }

    /// Applies a validated fill using average-cost realization.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed economics, regressing sequence,
    /// multiplier mismatch, or exhausted position capacity.
    pub fn apply_fill(
        &mut self,
        fill: LedgerFill,
    ) -> Result<LedgerApplyResult, PositionLedgerError> {
        fill.validate()?;
        let identity = MutationIdentity::Fill(LedgerExecutionIdentity::from_fill(&fill));
        if self.recent_identities.contains(&identity) {
            return Ok(LedgerApplyResult {
                status: LedgerApplyStatus::Duplicate,
                key: fill.key,
                sequence: fill.sequence,
                realized_pnl_delta: 0,
                position: self.positions.get(&fill.key).copied().unwrap_or_default(),
            });
        }
        self.ensure_identity_capacity()?;
        self.validate_sequence(fill.sequence)?;
        self.ensure_position_capacity(fill.key)?;

        let mut position = self.positions.get(&fill.key).copied().unwrap_or_default();
        if position.contract_multiplier == 0 {
            position.contract_multiplier = fill.contract_multiplier;
        } else if position.contract_multiplier != fill.contract_multiplier {
            return Err(PositionLedgerError::ContractMultiplierMismatch);
        }
        let realized = apply_average_cost(&mut position, fill);
        position.commissions = position.commissions.saturating_add(fill.commission);
        position.fees = position.fees.saturating_add(fill.fees);
        position.gross_traded_notional = position.gross_traded_notional.saturating_add(
            i128::from(fill.quantity.0)
                .saturating_mul(i128::from(fill.price.0))
                .saturating_mul(i128::from(fill.contract_multiplier)),
        );
        position.last_sequence = fill.sequence;
        position.updated_ns = fill.ts_recv_ns;
        recompute_unrealized(&mut position);
        let snapshot = position;
        self.positions.insert(fill.key, position);
        self.last_sequence = fill.sequence;
        self.retain_identity(identity);
        self.retain_attribution(LedgerFillAttribution {
            sequence: fill.sequence,
            execution_id: fill.execution_id,
            client_order_id: fill.client_order_id,
            venue_order_id: fill.venue_order_id,
            route_id: fill.route_id,
            key: fill.key,
            side: fill.side,
            quantity: fill.quantity,
            price: fill.price,
            realized_pnl_delta: realized,
            commission: fill.commission,
            fees: fill.fees,
            ts_recv_ns: fill.ts_recv_ns,
        });
        Ok(LedgerApplyResult {
            status: LedgerApplyStatus::Applied,
            key: fill.key,
            sequence: fill.sequence,
            realized_pnl_delta: realized,
            position: snapshot,
        })
    }

    /// Applies an authorized manual/corporate-action/correction mutation.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty id, invalid ordering, capacity exhaustion,
    /// invalid multiplier/price transition, or negative accumulated costs.
    pub fn apply_adjustment(
        &mut self,
        adjustment: LedgerAdjustment,
    ) -> Result<LedgerApplyResult, PositionLedgerError> {
        validate_adjustment(adjustment)?;
        let identity = MutationIdentity::Adjustment(LedgerScopedAdjustmentId {
            key: adjustment.key,
            adjustment_id: adjustment.adjustment_id,
        });
        if self.recent_identities.contains(&identity) {
            return Ok(LedgerApplyResult {
                status: LedgerApplyStatus::Duplicate,
                key: adjustment.key,
                sequence: adjustment.sequence,
                realized_pnl_delta: 0,
                position: self
                    .positions
                    .get(&adjustment.key)
                    .copied()
                    .unwrap_or_default(),
            });
        }
        self.ensure_identity_capacity()?;
        self.validate_sequence(adjustment.sequence)?;
        self.ensure_position_capacity(adjustment.key)?;

        let mut position = self
            .positions
            .get(&adjustment.key)
            .copied()
            .unwrap_or_default();
        let projected_commissions = position
            .commissions
            .saturating_add(adjustment.commission_delta);
        let projected_fees = position.fees.saturating_add(adjustment.fee_delta);
        if projected_commissions < 0 || projected_fees < 0 {
            return Err(PositionLedgerError::NegativeAccumulatedCosts);
        }
        let projected_qty = position.net_qty.saturating_add(adjustment.quantity_delta);
        let crosses = crosses_zero(position.net_qty, projected_qty);
        if (crosses || (position.net_qty == 0 && projected_qty != 0))
            && adjustment.average_price_override.is_none()
        {
            return Err(PositionLedgerError::AdjustmentPriceRequired);
        }
        if let Some(price) = adjustment.average_price_override {
            if price.0 <= 0 && projected_qty != 0 {
                return Err(PositionLedgerError::InvalidPrice);
            }
            position.average_price = if projected_qty == 0 { 0 } else { price.0 };
        } else if projected_qty == 0 {
            position.average_price = 0;
        }
        if adjustment.contract_multiplier_override > 0 {
            position.contract_multiplier = adjustment.contract_multiplier_override;
        } else if position.contract_multiplier == 0 && projected_qty != 0 {
            return Err(PositionLedgerError::InvalidContractMultiplier);
        }
        position.net_qty = projected_qty;
        if projected_qty == 0 {
            position.open_cost = 0;
        } else {
            position.open_cost = i128::from(projected_qty.saturating_abs())
                .saturating_mul(i128::from(position.average_price))
                .saturating_mul(i128::from(position.contract_multiplier));
        }
        position.realized_pnl = position
            .realized_pnl
            .saturating_add(adjustment.realized_pnl_delta);
        position.commissions = projected_commissions;
        position.fees = projected_fees;
        position.cash_balance = position.cash_balance.saturating_add(adjustment.cash_delta);
        position.last_sequence = adjustment.sequence;
        position.updated_ns = adjustment.timestamp_ns;
        recompute_unrealized(&mut position);
        let snapshot = position;
        self.positions.insert(adjustment.key, position);
        self.last_sequence = adjustment.sequence;
        self.retain_identity(identity);
        Ok(LedgerApplyResult {
            status: LedgerApplyStatus::Applied,
            key: adjustment.key,
            sequence: adjustment.sequence,
            realized_pnl_delta: adjustment.realized_pnl_delta,
            position: snapshot,
        })
    }

    /// Applies a mark and recomputes unrealized PnL.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid price/sequence, unknown position, or
    /// regressing global sequence.
    pub fn apply_mark(
        &mut self,
        mark: LedgerMark,
    ) -> Result<ProductionPosition, PositionLedgerError> {
        if mark.sequence == 0 {
            return Err(PositionLedgerError::InvalidSequence);
        }
        if mark.price.0 <= 0 {
            return Err(PositionLedgerError::InvalidPrice);
        }
        self.validate_sequence(mark.sequence)?;
        let position = self
            .positions
            .get_mut(&mark.key)
            .ok_or(PositionLedgerError::PositionNotFound)?;
        position.mark_price = mark.price.0;
        position.last_sequence = mark.sequence;
        position.updated_ns = mark.timestamp_ns;
        recompute_unrealized(position);
        self.last_sequence = mark.sequence;
        Ok(*position)
    }

    fn validate_sequence(&self, sequence: u64) -> Result<(), PositionLedgerError> {
        if sequence == 0 {
            return Err(PositionLedgerError::InvalidSequence);
        }
        if sequence <= self.last_sequence {
            return Err(PositionLedgerError::SequenceRegression {
                previous: self.last_sequence,
                received: sequence,
            });
        }
        Ok(())
    }

    fn ensure_position_capacity(
        &self,
        key: ProductionPositionKey,
    ) -> Result<(), PositionLedgerError> {
        if !self.positions.contains_key(&key)
            && self.positions.len() >= self.config.position_capacity
        {
            return Err(PositionLedgerError::PositionCapacityExceeded);
        }
        Ok(())
    }

    fn ensure_identity_capacity(&self) -> Result<(), PositionLedgerError> {
        if self.recent_identities.len() >= self.config.mutation_identity_capacity {
            return Err(PositionLedgerError::MutationIdentityCapacityExceeded);
        }
        Ok(())
    }

    fn retain_identity(&mut self, identity: MutationIdentity) {
        self.identity_order.push_back(identity);
        self.recent_identities.insert(identity);
    }

    fn retain_attribution(&mut self, attribution: LedgerFillAttribution) {
        if self.config.attribution_capacity == 0 {
            return;
        }
        if self.attributions.len() == self.config.attribution_capacity {
            self.attributions.pop_front();
        }
        self.attributions.push_back(attribution);
    }
}

fn apply_average_cost(position: &mut ProductionPosition, fill: LedgerFill) -> i128 {
    let current_qty = position.net_qty;
    let fill_signed = match fill.side {
        OrderSide::Buy => fill.quantity.0,
        OrderSide::Sell => fill.quantity.0.saturating_neg(),
    };
    if current_qty == 0 || current_qty.signum() == fill_signed.signum() {
        position.net_qty = current_qty.saturating_add(fill_signed);
        position.open_cost = position.open_cost.saturating_add(fill_notional(fill));
    } else {
        let close_qty = current_qty.saturating_abs().min(fill.quantity.0);
        let current_abs = current_qty.saturating_abs();
        let allocated_cost = if close_qty == current_abs {
            position.open_cost
        } else {
            position.open_cost.saturating_mul(i128::from(close_qty)) / i128::from(current_abs)
        };
        let close_value = i128::from(fill.price.0)
            .saturating_mul(i128::from(close_qty))
            .saturating_mul(i128::from(fill.contract_multiplier));
        let realized = if current_qty > 0 {
            close_value.saturating_sub(allocated_cost)
        } else {
            allocated_cost.saturating_sub(close_value)
        };
        position.realized_pnl = position.realized_pnl.saturating_add(realized);
        position.net_qty = current_qty.saturating_add(fill_signed);
        if position.net_qty == 0 {
            position.open_cost = 0;
            position.average_price = 0;
        } else if position.net_qty.signum() != current_qty.signum() {
            let opening_qty = fill.quantity.0.saturating_sub(close_qty);
            position.open_cost = i128::from(opening_qty)
                .saturating_mul(i128::from(fill.price.0))
                .saturating_mul(i128::from(fill.contract_multiplier));
        } else {
            position.open_cost = position.open_cost.saturating_sub(allocated_cost);
        }
        refresh_average_price(position);
        update_side_totals(position, fill);
        return realized;
    }
    refresh_average_price(position);
    update_side_totals(position, fill);
    0
}

fn fill_notional(fill: LedgerFill) -> i128 {
    i128::from(fill.quantity.0)
        .saturating_mul(i128::from(fill.price.0))
        .saturating_mul(i128::from(fill.contract_multiplier))
}

fn refresh_average_price(position: &mut ProductionPosition) {
    if position.net_qty == 0 || position.contract_multiplier <= 0 {
        position.average_price = 0;
        return;
    }
    let divisor = i128::from(position.net_qty.saturating_abs())
        .saturating_mul(i128::from(position.contract_multiplier));
    position.average_price = i64::try_from(position.open_cost / divisor).unwrap_or(i64::MAX);
}

fn update_side_totals(position: &mut ProductionPosition, fill: LedgerFill) {
    match fill.side {
        OrderSide::Buy => position.buy_qty = position.buy_qty.saturating_add(fill.quantity.0),
        OrderSide::Sell => position.sell_qty = position.sell_qty.saturating_add(fill.quantity.0),
    }
}

fn recompute_unrealized(position: &mut ProductionPosition) {
    if position.net_qty == 0 || position.mark_price <= 0 || position.contract_multiplier <= 0 {
        position.unrealized_pnl = 0;
        return;
    }
    let market_value = i128::from(position.mark_price)
        .saturating_mul(i128::from(position.net_qty.saturating_abs()))
        .saturating_mul(i128::from(position.contract_multiplier));
    position.unrealized_pnl = if position.net_qty > 0 {
        market_value.saturating_sub(position.open_cost)
    } else {
        position.open_cost.saturating_sub(market_value)
    };
}

fn validate_adjustment(adjustment: LedgerAdjustment) -> Result<(), PositionLedgerError> {
    if adjustment.sequence == 0 {
        return Err(PositionLedgerError::InvalidSequence);
    }
    if adjustment.adjustment_id.is_empty() {
        return Err(PositionLedgerError::MissingMutationId);
    }
    if adjustment.key.currency.is_empty() {
        return Err(PositionLedgerError::MissingCurrency);
    }
    if adjustment.contract_multiplier_override < 0 {
        return Err(PositionLedgerError::InvalidContractMultiplier);
    }
    Ok(())
}

fn crosses_zero(current: i64, projected: i64) -> bool {
    (current < 0 && projected > 0) || (current > 0 && projected < 0)
}

/// Persisted recent mutation identity used by checkpoint recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LedgerCheckpointIdentity {
    /// Route/account/symbol-scoped venue execution/fill id.
    Fill(LedgerExecutionIdentity),
    /// Position-scoped manual/corporate-action/correction id.
    Adjustment(LedgerScopedAdjustmentId),
}

impl From<MutationIdentity> for LedgerCheckpointIdentity {
    fn from(value: MutationIdentity) -> Self {
        match value {
            MutationIdentity::Fill(id) => Self::Fill(id),
            MutationIdentity::Adjustment(id) => Self::Adjustment(id),
        }
    }
}

impl From<LedgerCheckpointIdentity> for MutationIdentity {
    fn from(value: LedgerCheckpointIdentity) -> Self {
        match value {
            LedgerCheckpointIdentity::Fill(id) => Self::Fill(id),
            LedgerCheckpointIdentity::Adjustment(id) => Self::Adjustment(id),
        }
    }
}

/// Position row stored in a ledger checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LedgerCheckpointPosition {
    /// Position key.
    pub key: ProductionPositionKey,
    /// Authoritative position state.
    pub position: ProductionPosition,
}

/// Versioned, checksummed position-ledger checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PositionLedgerCheckpoint {
    /// Binary schema version.
    pub schema_version: u16,
    /// Caller-assigned checkpoint id.
    pub checkpoint_id: u64,
    /// Checkpoint creation timestamp.
    pub created_ns: u64,
    /// Last fully applied global mutation sequence.
    pub last_sequence: u64,
    /// Deterministically ordered position rows.
    pub positions: Vec<LedgerCheckpointPosition>,
    /// Oldest-to-newest retained duplicate identities.
    pub mutation_identities: Vec<LedgerCheckpointIdentity>,
    /// Oldest-to-newest retained fill attribution.
    pub recent_attributions: Vec<LedgerFillAttribution>,
    /// FNV-1a checksum over the canonical payload.
    pub checksum: u64,
}

impl PositionLedgerCheckpoint {
    /// Recomputes and stores the checkpoint checksum.
    pub fn refresh_checksum(&mut self) {
        self.checksum = checkpoint_checksum(self);
    }

    /// Returns true when schema and checksum are valid.
    pub fn validate(&self) -> bool {
        self.schema_version == POSITION_LEDGER_CHECKPOINT_VERSION
            && self.checksum == checkpoint_checksum(self)
    }

    /// Encodes the checkpoint into a deterministic binary payload.
    pub fn encode(&self) -> Vec<u8> {
        encode_checkpoint(self, true)
    }

    /// Decodes and validates a deterministic binary checkpoint payload.
    ///
    /// # Errors
    ///
    /// Returns [`PositionLedgerError::InvalidCheckpoint`] for malformed,
    /// unsupported, trailing, or checksum-invalid bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, PositionLedgerError> {
        decode_checkpoint(bytes)
    }
}

impl ProductionPositionLedger {
    /// Captures deterministic position, duplicate, and attribution state.
    pub fn checkpoint(&self, checkpoint_id: u64, created_ns: u64) -> PositionLedgerCheckpoint {
        let mut positions = self
            .positions
            .iter()
            .map(|(key, position)| LedgerCheckpointPosition {
                key: *key,
                position: *position,
            })
            .collect::<Vec<_>>();
        positions.sort_by(|left, right| compare_position_keys(&left.key, &right.key));
        let mut checkpoint = PositionLedgerCheckpoint {
            schema_version: POSITION_LEDGER_CHECKPOINT_VERSION,
            checkpoint_id,
            created_ns,
            last_sequence: self.last_sequence,
            positions,
            mutation_identities: self
                .identity_order
                .iter()
                .copied()
                .map(Into::into)
                .collect(),
            recent_attributions: self.attributions.iter().copied().collect(),
            checksum: 0,
        };
        checkpoint.refresh_checksum();
        checkpoint
    }

    /// Restores an authoritative ledger after validating the entire checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid schema/checksum, duplicate keys/ids,
    /// inconsistent position economics, or configured capacity exhaustion.
    pub fn restore(
        config: ProductionPositionLedgerConfig,
        checkpoint: &PositionLedgerCheckpoint,
    ) -> Result<Self, PositionLedgerError> {
        if !checkpoint.validate() {
            return Err(PositionLedgerError::InvalidCheckpoint);
        }
        if checkpoint.positions.len() > config.position_capacity
            || checkpoint.mutation_identities.len() > config.mutation_identity_capacity
            || checkpoint.recent_attributions.len() > config.attribution_capacity
        {
            return Err(PositionLedgerError::CheckpointCapacityExceeded);
        }
        let mut ledger = Self::new(config);
        for row in &checkpoint.positions {
            validate_checkpoint_position(*row, checkpoint.last_sequence)?;
            if ledger.positions.insert(row.key, row.position).is_some() {
                return Err(PositionLedgerError::InvalidCheckpoint);
            }
        }
        for identity in &checkpoint.mutation_identities {
            let identity: MutationIdentity = (*identity).into();
            if !ledger.recent_identities.insert(identity) {
                return Err(PositionLedgerError::InvalidCheckpoint);
            }
            ledger.identity_order.push_back(identity);
        }
        for attribution in &checkpoint.recent_attributions {
            if attribution.sequence == 0 || attribution.sequence > checkpoint.last_sequence {
                return Err(PositionLedgerError::InvalidCheckpoint);
            }
            ledger.attributions.push_back(*attribution);
        }
        ledger.last_sequence = checkpoint.last_sequence;
        Ok(ledger)
    }
}

fn compare_position_keys(
    left: &ProductionPositionKey,
    right: &ProductionPositionKey,
) -> std::cmp::Ordering {
    left.account_id
        .as_str()
        .cmp(right.account_id.as_str())
        .then_with(|| left.strategy_id.as_str().cmp(right.strategy_id.as_str()))
        .then_with(|| left.symbol.venue.as_str().cmp(right.symbol.venue.as_str()))
        .then_with(|| {
            left.symbol
                .instrument
                .as_str()
                .cmp(right.symbol.instrument.as_str())
        })
        .then_with(|| left.currency.as_str().cmp(right.currency.as_str()))
}

fn validate_checkpoint_position(
    row: LedgerCheckpointPosition,
    checkpoint_sequence: u64,
) -> Result<(), PositionLedgerError> {
    let position = row.position;
    if row.key.currency.is_empty()
        || position.net_qty != 0
            && (position.average_price <= 0
                || position.contract_multiplier <= 0
                || position.open_cost <= 0)
        || position.net_qty == 0 && (position.average_price != 0 || position.open_cost != 0)
        || position.buy_qty < 0
        || position.sell_qty < 0
        || position.commissions < 0
        || position.fees < 0
        || position.last_sequence > checkpoint_sequence
    {
        return Err(PositionLedgerError::InvalidCheckpoint);
    }
    let mut expected = position;
    refresh_average_price(&mut expected);
    recompute_unrealized(&mut expected);
    if expected.average_price != position.average_price
        || expected.unrealized_pnl != position.unrealized_pnl
    {
        return Err(PositionLedgerError::InvalidCheckpoint);
    }
    Ok(())
}

/// File-backed checkpoint-store configuration.
#[derive(Debug, Clone)]
pub struct PositionLedgerCheckpointConfig {
    root: PathBuf,
    sync_on_save: bool,
    max_retained: usize,
    max_checkpoint_bytes: usize,
}

impl PositionLedgerCheckpointConfig {
    /// Creates conservative file-store configuration rooted at `root`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            sync_on_save: true,
            max_retained: 8,
            max_checkpoint_bytes: 512 * 1024 * 1024,
        }
    }

    /// Sets whether file and parent directory are synced before success.
    pub fn with_sync_on_save(mut self, sync_on_save: bool) -> Self {
        self.sync_on_save = sync_on_save;
        self
    }

    /// Sets maximum retained checkpoints. Zero disables automatic retention.
    pub fn with_max_retained(mut self, max_retained: usize) -> Self {
        self.max_retained = max_retained;
        self
    }

    /// Sets maximum accepted encoded checkpoint bytes.
    pub fn with_max_checkpoint_bytes(mut self, max_checkpoint_bytes: usize) -> Self {
        self.max_checkpoint_bytes = max_checkpoint_bytes;
        self
    }

    /// Returns checkpoint root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns whether durable sync is enabled.
    pub const fn sync_on_save(&self) -> bool {
        self.sync_on_save
    }

    /// Returns retained checkpoint limit.
    pub const fn max_retained(&self) -> usize {
        self.max_retained
    }

    /// Returns maximum encoded checkpoint bytes.
    pub const fn max_checkpoint_bytes(&self) -> usize {
        self.max_checkpoint_bytes
    }
}

/// Installed checkpoint file metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PositionLedgerCheckpointManifest {
    /// Checkpoint id.
    pub checkpoint_id: u64,
    /// Last covered ledger sequence.
    pub last_sequence: u64,
    /// Checkpoint creation timestamp.
    pub created_ns: u64,
    /// Installed path.
    pub path: PathBuf,
    /// Encoded byte count.
    pub bytes: u64,
    /// Payload checksum.
    pub checksum: u64,
}

/// Replaceable checkpoint-store contract.
pub trait PositionLedgerCheckpointStore: Send {
    /// Saves one validated checkpoint.
    fn save(
        &mut self,
        checkpoint: &PositionLedgerCheckpoint,
    ) -> Result<PositionLedgerCheckpointManifest, PositionLedgerError>;

    /// Loads the latest valid checkpoint.
    fn load_latest(&self) -> Result<Option<PositionLedgerCheckpoint>, PositionLedgerError>;

    /// Lists valid installed checkpoint manifests in id order.
    fn list(&self) -> Result<Vec<PositionLedgerCheckpointManifest>, PositionLedgerError>;

    /// Prunes oldest checkpoints according to configured retention.
    fn prune(&mut self) -> Result<usize, PositionLedgerError>;
}

/// Atomic file-backed production position-ledger checkpoint store.
#[derive(Debug, Clone)]
pub struct FilePositionLedgerCheckpointStore {
    config: PositionLedgerCheckpointConfig,
}

impl FilePositionLedgerCheckpointStore {
    /// Opens or creates a checkpoint directory.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the root cannot be created.
    pub fn open(config: PositionLedgerCheckpointConfig) -> Result<Self, PositionLedgerError> {
        fs::create_dir_all(config.root())?;
        Ok(Self { config })
    }

    /// Returns store configuration.
    pub const fn config(&self) -> &PositionLedgerCheckpointConfig {
        &self.config
    }

    fn checkpoint_path(&self, checkpoint_id: u64) -> PathBuf {
        self.config.root.join(format!(
            "position-ledger-{checkpoint_id:020}.{CHECKPOINT_EXTENSION}"
        ))
    }
}

impl PositionLedgerCheckpointStore for FilePositionLedgerCheckpointStore {
    fn save(
        &mut self,
        checkpoint: &PositionLedgerCheckpoint,
    ) -> Result<PositionLedgerCheckpointManifest, PositionLedgerError> {
        if !checkpoint.validate() {
            return Err(PositionLedgerError::InvalidCheckpoint);
        }
        let bytes = checkpoint.encode();
        if bytes.len() > self.config.max_checkpoint_bytes {
            return Err(PositionLedgerError::CheckpointCapacityExceeded);
        }
        let path = self.checkpoint_path(checkpoint.checkpoint_id);
        if path.exists() {
            return Err(PositionLedgerError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "position ledger checkpoint id already exists",
            )));
        }
        let temp = self
            .config
            .root
            .join(format!(".position-ledger-{}.tmp", checkpoint.checkpoint_id));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let install = (|| -> Result<(), PositionLedgerError> {
            file.write_all(&bytes)?;
            if self.config.sync_on_save {
                file.sync_all()?;
            }
            drop(file);
            fs::rename(&temp, &path)?;
            if self.config.sync_on_save {
                sync_directory(&self.config.root)?;
            }
            Ok(())
        })();
        if install.is_err() {
            let _ = fs::remove_file(&temp);
        }
        install?;
        if self.config.max_retained > 0 {
            self.prune()?;
        }
        Ok(PositionLedgerCheckpointManifest {
            checkpoint_id: checkpoint.checkpoint_id,
            last_sequence: checkpoint.last_sequence,
            created_ns: checkpoint.created_ns,
            path,
            bytes: bytes.len() as u64,
            checksum: checkpoint.checksum,
        })
    }

    fn load_latest(&self) -> Result<Option<PositionLedgerCheckpoint>, PositionLedgerError> {
        let Some(manifest) = self.list()?.pop() else {
            return Ok(None);
        };
        let metadata = fs::metadata(&manifest.path)?;
        if metadata.len() > self.config.max_checkpoint_bytes as u64 {
            return Err(PositionLedgerError::CheckpointCapacityExceeded);
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        File::open(manifest.path)?.read_to_end(&mut bytes)?;
        Ok(Some(PositionLedgerCheckpoint::decode(&bytes)?))
    }

    fn list(&self) -> Result<Vec<PositionLedgerCheckpointManifest>, PositionLedgerError> {
        let mut manifests = Vec::new();
        for entry in fs::read_dir(&self.config.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some(CHECKPOINT_EXTENSION) {
                continue;
            }
            let metadata = entry.metadata()?;
            if metadata.len() > self.config.max_checkpoint_bytes as u64 {
                return Err(PositionLedgerError::CheckpointCapacityExceeded);
            }
            let mut bytes = Vec::with_capacity(metadata.len() as usize);
            File::open(&path)?.read_to_end(&mut bytes)?;
            let checkpoint = PositionLedgerCheckpoint::decode(&bytes)?;
            manifests.push(PositionLedgerCheckpointManifest {
                checkpoint_id: checkpoint.checkpoint_id,
                last_sequence: checkpoint.last_sequence,
                created_ns: checkpoint.created_ns,
                path,
                bytes: metadata.len(),
                checksum: checkpoint.checksum,
            });
        }
        manifests.sort_by_key(|item| item.checkpoint_id);
        Ok(manifests)
    }

    fn prune(&mut self) -> Result<usize, PositionLedgerError> {
        if self.config.max_retained == 0 {
            return Ok(0);
        }
        let manifests = self.list()?;
        let remove_count = manifests.len().saturating_sub(self.config.max_retained);
        for manifest in manifests.iter().take(remove_count) {
            fs::remove_file(&manifest.path)?;
        }
        if remove_count > 0 && self.config.sync_on_save {
            sync_directory(&self.config.root)?;
        }
        Ok(remove_count)
    }
}

fn checkpoint_checksum(checkpoint: &PositionLedgerCheckpoint) -> u64 {
    fnv1a(&encode_checkpoint(checkpoint, false))
}

fn encode_checkpoint(checkpoint: &PositionLedgerCheckpoint, include_checksum: bool) -> Vec<u8> {
    let mut out = Vec::new();
    put_u32(&mut out, CHECKPOINT_MAGIC);
    put_u16(&mut out, checkpoint.schema_version);
    put_u64(&mut out, checkpoint.checkpoint_id);
    put_u64(&mut out, checkpoint.created_ns);
    put_u64(&mut out, checkpoint.last_sequence);
    put_u32(&mut out, checkpoint.positions.len() as u32);
    for row in &checkpoint.positions {
        encode_key(&mut out, row.key);
        encode_position(&mut out, row.position);
    }
    put_u32(&mut out, checkpoint.mutation_identities.len() as u32);
    for identity in &checkpoint.mutation_identities {
        match identity {
            LedgerCheckpointIdentity::Fill(id) => {
                out.push(1);
                put_fixed(&mut out, &id.route_id);
                put_fixed(&mut out, &id.account_id);
                put_fixed(&mut out, &id.symbol.venue);
                put_fixed(&mut out, &id.symbol.instrument);
                put_fixed(&mut out, &id.execution_id);
            }
            LedgerCheckpointIdentity::Adjustment(id) => {
                out.push(2);
                encode_key(&mut out, id.key);
                put_fixed(&mut out, &id.adjustment_id);
            }
        }
    }
    put_u32(&mut out, checkpoint.recent_attributions.len() as u32);
    for attribution in &checkpoint.recent_attributions {
        encode_attribution(&mut out, *attribution);
    }
    if include_checksum {
        put_u64(&mut out, checkpoint.checksum);
    }
    out
}

fn decode_checkpoint(bytes: &[u8]) -> Result<PositionLedgerCheckpoint, PositionLedgerError> {
    let mut reader = LedgerReader::new(bytes);
    if reader.read_u32()? != CHECKPOINT_MAGIC {
        return Err(PositionLedgerError::InvalidCheckpoint);
    }
    let schema_version = reader.read_u16()?;
    if schema_version != POSITION_LEDGER_CHECKPOINT_VERSION {
        return Err(PositionLedgerError::InvalidCheckpoint);
    }
    let checkpoint_id = reader.read_u64()?;
    let created_ns = reader.read_u64()?;
    let last_sequence = reader.read_u64()?;
    let position_count = reader.read_count()?;
    let mut positions = Vec::with_capacity(position_count);
    for _ in 0..position_count {
        positions.push(LedgerCheckpointPosition {
            key: decode_key(&mut reader)?,
            position: decode_position(&mut reader)?,
        });
    }
    let identity_count = reader.read_count()?;
    let mut mutation_identities = Vec::with_capacity(identity_count);
    for _ in 0..identity_count {
        mutation_identities.push(match reader.read_u8()? {
            1 => LedgerCheckpointIdentity::Fill(LedgerExecutionIdentity {
                route_id: reader.read_fixed()?,
                account_id: reader.read_fixed()?,
                symbol: ExecutionSymbol {
                    venue: reader.read_fixed()?,
                    instrument: reader.read_fixed()?,
                },
                execution_id: reader.read_fixed()?,
            }),
            2 => LedgerCheckpointIdentity::Adjustment(LedgerScopedAdjustmentId {
                key: decode_key(&mut reader)?,
                adjustment_id: reader.read_fixed()?,
            }),
            _ => return Err(PositionLedgerError::InvalidCheckpoint),
        });
    }
    let attribution_count = reader.read_count()?;
    let mut recent_attributions = Vec::with_capacity(attribution_count);
    for _ in 0..attribution_count {
        recent_attributions.push(decode_attribution(&mut reader)?);
    }
    let checksum = reader.read_u64()?;
    if !reader.finished() {
        return Err(PositionLedgerError::InvalidCheckpoint);
    }
    let checkpoint = PositionLedgerCheckpoint {
        schema_version,
        checkpoint_id,
        created_ns,
        last_sequence,
        positions,
        mutation_identities,
        recent_attributions,
        checksum,
    };
    if !checkpoint.validate() {
        return Err(PositionLedgerError::InvalidCheckpoint);
    }
    Ok(checkpoint)
}

fn encode_key(out: &mut Vec<u8>, key: ProductionPositionKey) {
    put_fixed(out, &key.account_id);
    put_fixed(out, &key.strategy_id);
    put_fixed(out, &key.symbol.venue);
    put_fixed(out, &key.symbol.instrument);
    put_fixed(out, &key.currency);
}

fn decode_key(reader: &mut LedgerReader<'_>) -> Result<ProductionPositionKey, PositionLedgerError> {
    Ok(ProductionPositionKey::new(
        reader.read_fixed()?,
        reader.read_fixed()?,
        ExecutionSymbol {
            venue: reader.read_fixed()?,
            instrument: reader.read_fixed()?,
        },
        reader.read_fixed()?,
    ))
}

fn encode_position(out: &mut Vec<u8>, value: ProductionPosition) {
    put_i64(out, value.net_qty);
    put_i64(out, value.buy_qty);
    put_i64(out, value.sell_qty);
    put_i64(out, value.average_price);
    put_i128(out, value.open_cost);
    put_i64(out, value.mark_price);
    put_i64(out, value.contract_multiplier);
    put_i128(out, value.realized_pnl);
    put_i128(out, value.unrealized_pnl);
    put_i128(out, value.commissions);
    put_i128(out, value.fees);
    put_i128(out, value.cash_balance);
    put_i128(out, value.gross_traded_notional);
    put_u64(out, value.last_sequence);
    put_u64(out, value.updated_ns);
}

fn decode_position(
    reader: &mut LedgerReader<'_>,
) -> Result<ProductionPosition, PositionLedgerError> {
    Ok(ProductionPosition {
        net_qty: reader.read_i64()?,
        buy_qty: reader.read_i64()?,
        sell_qty: reader.read_i64()?,
        average_price: reader.read_i64()?,
        open_cost: reader.read_i128()?,
        mark_price: reader.read_i64()?,
        contract_multiplier: reader.read_i64()?,
        realized_pnl: reader.read_i128()?,
        unrealized_pnl: reader.read_i128()?,
        commissions: reader.read_i128()?,
        fees: reader.read_i128()?,
        cash_balance: reader.read_i128()?,
        gross_traded_notional: reader.read_i128()?,
        last_sequence: reader.read_u64()?,
        updated_ns: reader.read_u64()?,
    })
}

fn encode_attribution(out: &mut Vec<u8>, value: LedgerFillAttribution) {
    put_u64(out, value.sequence);
    put_fixed(out, &value.execution_id);
    put_fixed(out, &value.client_order_id);
    put_fixed(out, &value.venue_order_id);
    put_fixed(out, &value.route_id);
    encode_key(out, value.key);
    out.push(match value.side {
        OrderSide::Buy => 1,
        OrderSide::Sell => 2,
    });
    put_i64(out, value.quantity.0);
    put_i64(out, value.price.0);
    put_i128(out, value.realized_pnl_delta);
    put_i128(out, value.commission);
    put_i128(out, value.fees);
    put_u64(out, value.ts_recv_ns);
}

fn decode_attribution(
    reader: &mut LedgerReader<'_>,
) -> Result<LedgerFillAttribution, PositionLedgerError> {
    let sequence = reader.read_u64()?;
    let execution_id = reader.read_fixed()?;
    let client_order_id = reader.read_fixed()?;
    let venue_order_id = reader.read_fixed()?;
    let route_id = reader.read_fixed()?;
    let key = decode_key(reader)?;
    let side = match reader.read_u8()? {
        1 => OrderSide::Buy,
        2 => OrderSide::Sell,
        _ => return Err(PositionLedgerError::InvalidCheckpoint),
    };
    Ok(LedgerFillAttribution {
        sequence,
        execution_id,
        client_order_id,
        venue_order_id,
        route_id,
        key,
        side,
        quantity: OrderQty(reader.read_i64()?),
        price: OrderPrice(reader.read_i64()?),
        realized_pnl_delta: reader.read_i128()?,
        commission: reader.read_i128()?,
        fees: reader.read_i128()?,
        ts_recv_ns: reader.read_u64()?,
    })
}

struct LedgerReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> LedgerReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_count(&mut self) -> Result<usize, PositionLedgerError> {
        let count = self.read_u32()? as usize;
        if count > self.bytes.len() {
            return Err(PositionLedgerError::InvalidCheckpoint);
        }
        Ok(count)
    }

    fn read_u8(&mut self) -> Result<u8, PositionLedgerError> {
        Ok(self.take(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, PositionLedgerError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn read_u32(&mut self) -> Result<u32, PositionLedgerError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn read_u64(&mut self) -> Result<u64, PositionLedgerError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn read_i64(&mut self) -> Result<i64, PositionLedgerError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn read_i128(&mut self) -> Result<i128, PositionLedgerError> {
        Ok(i128::from_le_bytes(self.take(16)?.try_into().unwrap()))
    }

    fn read_fixed<const N: usize>(&mut self) -> Result<FixedAscii<N>, PositionLedgerError> {
        let len = self.read_u8()? as usize;
        if len > N {
            return Err(PositionLedgerError::InvalidCheckpoint);
        }
        let bytes = self.take(len)?;
        let value =
            std::str::from_utf8(bytes).map_err(|_| PositionLedgerError::InvalidCheckpoint)?;
        FixedAscii::new(value).map_err(|_| PositionLedgerError::InvalidCheckpoint)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], PositionLedgerError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(PositionLedgerError::InvalidCheckpoint)?;
        if end > self.bytes.len() {
            return Err(PositionLedgerError::InvalidCheckpoint);
        }
        let result = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(result)
    }

    fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

fn put_fixed<const N: usize>(out: &mut Vec<u8>, value: &FixedAscii<N>) {
    out.push(value.as_str().len() as u8);
    out.extend_from_slice(value.as_str().as_bytes());
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_i128(out: &mut Vec<u8>, value: i128) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

fn sync_directory(path: &Path) -> Result<(), PositionLedgerError> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    Ok(())
}

/// Authoritative broker, clearing, venue, or drop-copy position snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExternalPositionSnapshot {
    /// Position key.
    pub key: ProductionPositionKey,
    /// Signed net quantity.
    pub net_qty: i64,
    /// Average open price.
    pub average_price: i64,
    /// Positive contract multiplier.
    pub contract_multiplier: i64,
    /// Gross realized PnL when supplied by the source.
    pub realized_pnl: i128,
    /// Cumulative commissions when supplied by the source.
    pub commissions: i128,
    /// Cumulative other fees when supplied by the source.
    pub fees: i128,
    /// Source as-of timestamp.
    pub as_of_ns: u64,
}

impl ExternalPositionSnapshot {
    /// Creates an external position with zero reported PnL and costs.
    pub const fn new(
        key: ProductionPositionKey,
        net_qty: i64,
        average_price: i64,
        contract_multiplier: i64,
        as_of_ns: u64,
    ) -> Self {
        Self {
            key,
            net_qty,
            average_price,
            contract_multiplier,
            realized_pnl: 0,
            commissions: 0,
            fees: 0,
            as_of_ns,
        }
    }

    /// Sets externally reported realized PnL, commission, and other fees.
    pub const fn with_financials(
        mut self,
        realized_pnl: i128,
        commissions: i128,
        fees: i128,
    ) -> Self {
        self.realized_pnl = realized_pnl;
        self.commissions = commissions;
        self.fees = fees;
        self
    }
}

/// Absolute comparison tolerances for external position reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PositionReconciliationTolerance {
    /// Allowed average-price difference.
    pub average_price: i64,
    /// Allowed realized-PnL difference.
    pub realized_pnl: i128,
    /// Allowed commission difference.
    pub commissions: i128,
    /// Allowed fee difference.
    pub fees: i128,
}

impl PositionReconciliationTolerance {
    /// Creates explicit absolute comparison tolerances.
    pub const fn new(
        average_price: i64,
        realized_pnl: i128,
        commissions: i128,
        fees: i128,
    ) -> Self {
        Self {
            average_price,
            realized_pnl,
            commissions,
            fees,
        }
    }
}

/// Compact reconciliation issue bitset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PositionReconciliationIssueFlags(u32);

impl PositionReconciliationIssueFlags {
    /// Local position has no external counterpart.
    pub const LOCAL_ONLY: Self = Self(1 << 0);
    /// External position has no local counterpart.
    pub const EXTERNAL_ONLY: Self = Self(1 << 1);
    /// Signed net quantity differs.
    pub const NET_QUANTITY: Self = Self(1 << 2);
    /// Average open price differs beyond tolerance.
    pub const AVERAGE_PRICE: Self = Self(1 << 3);
    /// Contract multiplier differs.
    pub const CONTRACT_MULTIPLIER: Self = Self(1 << 4);
    /// Gross realized PnL differs beyond tolerance.
    pub const REALIZED_PNL: Self = Self(1 << 5);
    /// Commission differs beyond tolerance.
    pub const COMMISSIONS: Self = Self(1 << 6);
    /// Other fees differ beyond tolerance.
    pub const FEES: Self = Self(1 << 7);
    /// External input repeats a position key.
    pub const DUPLICATE_EXTERNAL_KEY: Self = Self(1 << 8);

    /// Returns an empty issue set.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Returns raw issue bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Returns true when no issues are set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns true when every bit in `other` is present.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// One local-to-external reconciliation comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PositionReconciliationItem {
    /// Compared key.
    pub key: ProductionPositionKey,
    /// Local position when present.
    pub local: Option<ProductionPosition>,
    /// External position when present.
    pub external: Option<ExternalPositionSnapshot>,
    /// Compact issue classification.
    pub issues: PositionReconciliationIssueFlags,
}

/// Caller-owned bounded reconciliation output.
#[derive(Debug, Clone)]
pub struct PositionReconciliationBuffer {
    items: Vec<PositionReconciliationItem>,
    capacity: usize,
}

impl PositionReconciliationBuffer {
    /// Creates an empty bounded output buffer.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Clears rows without releasing allocation.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Returns compared rows.
    pub fn as_slice(&self) -> &[PositionReconciliationItem] {
        &self.items
    }

    /// Returns configured maximum row count.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    fn push(&mut self, item: PositionReconciliationItem) -> Result<(), PositionLedgerError> {
        if self.items.len() >= self.capacity {
            return Err(PositionLedgerError::ReconciliationBufferFull);
        }
        self.items.push(item);
        Ok(())
    }
}

/// Aggregate position reconciliation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PositionReconciliationReport {
    /// Compared output rows, including explicit duplicate-source rows.
    pub compared: u32,
    /// Keys with no issue.
    pub matched: u32,
    /// Keys with one or more issues.
    pub mismatched: u32,
    /// Local-only keys.
    pub local_only: u32,
    /// External-only keys.
    pub external_only: u32,
    /// Duplicate external input rows.
    pub duplicate_external_keys: u32,
    /// Latest external as-of timestamp observed.
    pub external_as_of_ns: u64,
}

/// Reconciles local ledger state against external authoritative snapshots.
///
/// The function is non-mutating and writes one row per unique key plus one
/// explicit row for each duplicate external key. Callers should keep new order
/// flow blocked until mismatches are resolved under an operator policy.
///
/// # Errors
///
/// Returns [`PositionLedgerError::ReconciliationBufferFull`] without silently
/// truncating output when caller capacity is insufficient.
pub fn reconcile_production_positions(
    ledger: &ProductionPositionLedger,
    external: &[ExternalPositionSnapshot],
    tolerance: PositionReconciliationTolerance,
    out: &mut PositionReconciliationBuffer,
) -> Result<PositionReconciliationReport, PositionLedgerError> {
    out.clear();
    let mut report = PositionReconciliationReport::default();
    let mut first_external = HashMap::with_capacity(external.len());
    let mut duplicate_external = HashSet::with_capacity(external.len());
    for (index, item) in external.iter().enumerate() {
        if first_external.insert(item.key, index).is_some() {
            duplicate_external.insert(index);
        }
    }
    // Retain the first occurrence as authoritative for comparison.
    first_external.clear();
    for (index, item) in external.iter().enumerate() {
        first_external.entry(item.key).or_insert(index);
    }
    let mut matched_external = HashSet::with_capacity(external.len());

    for (key, local) in ledger.positions() {
        let Some(first_index) = first_external.get(key).copied() else {
            let issues = PositionReconciliationIssueFlags::LOCAL_ONLY;
            out.push(PositionReconciliationItem {
                key: *key,
                local: Some(*local),
                external: None,
                issues,
            })?;
            report.compared = report.compared.saturating_add(1);
            report.mismatched = report.mismatched.saturating_add(1);
            report.local_only = report.local_only.saturating_add(1);
            continue;
        };
        let first = external[first_index];
        matched_external.insert(first_index);
        report.external_as_of_ns = report.external_as_of_ns.max(first.as_of_ns);
        let issues = compare_external(*local, first, tolerance);
        out.push(PositionReconciliationItem {
            key: *key,
            local: Some(*local),
            external: Some(first),
            issues,
        })?;
        report.compared = report.compared.saturating_add(1);
        if issues.is_empty() {
            report.matched = report.matched.saturating_add(1);
        } else {
            report.mismatched = report.mismatched.saturating_add(1);
        }
    }

    for (index, item) in external.iter().enumerate() {
        if duplicate_external.contains(&index) {
            report.external_as_of_ns = report.external_as_of_ns.max(item.as_of_ns);
            out.push(PositionReconciliationItem {
                key: item.key,
                local: ledger.position(&item.key),
                external: Some(*item),
                issues: PositionReconciliationIssueFlags::DUPLICATE_EXTERNAL_KEY,
            })?;
            report.compared = report.compared.saturating_add(1);
            report.mismatched = report.mismatched.saturating_add(1);
            report.duplicate_external_keys = report.duplicate_external_keys.saturating_add(1);
            continue;
        }
        if matched_external.contains(&index) {
            continue;
        }
        report.external_as_of_ns = report.external_as_of_ns.max(item.as_of_ns);
        out.push(PositionReconciliationItem {
            key: item.key,
            local: None,
            external: Some(*item),
            issues: PositionReconciliationIssueFlags::EXTERNAL_ONLY,
        })?;
        report.compared = report.compared.saturating_add(1);
        report.mismatched = report.mismatched.saturating_add(1);
        report.external_only = report.external_only.saturating_add(1);
    }
    Ok(report)
}

fn compare_external(
    local: ProductionPosition,
    external: ExternalPositionSnapshot,
    tolerance: PositionReconciliationTolerance,
) -> PositionReconciliationIssueFlags {
    let mut issues = PositionReconciliationIssueFlags::empty();
    if local.net_qty != external.net_qty {
        issues.insert(PositionReconciliationIssueFlags::NET_QUANTITY);
    }
    if abs_diff_i64(local.average_price, external.average_price)
        > tolerance.average_price.max(0) as u64
    {
        issues.insert(PositionReconciliationIssueFlags::AVERAGE_PRICE);
    }
    if local.contract_multiplier != external.contract_multiplier {
        issues.insert(PositionReconciliationIssueFlags::CONTRACT_MULTIPLIER);
    }
    if abs_diff_i128(local.realized_pnl, external.realized_pnl)
        > tolerance.realized_pnl.max(0) as u128
    {
        issues.insert(PositionReconciliationIssueFlags::REALIZED_PNL);
    }
    if abs_diff_i128(local.commissions, external.commissions) > tolerance.commissions.max(0) as u128
    {
        issues.insert(PositionReconciliationIssueFlags::COMMISSIONS);
    }
    if abs_diff_i128(local.fees, external.fees) > tolerance.fees.max(0) as u128 {
        issues.insert(PositionReconciliationIssueFlags::FEES);
    }
    issues
}

fn abs_diff_i64(left: i64, right: i64) -> u64 {
    left.abs_diff(right)
}

fn abs_diff_i128(left: i128, right: i128) -> u128 {
    left.abs_diff(right)
}

#[cfg(test)]
mod tests {
    use of_execution_core::{ExecutionText, OrderStatus, RiskRejectReason};

    use super::*;

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn key(instrument: &str) -> ProductionPositionKey {
        ProductionPositionKey::new(
            id("account-a"),
            id("strategy-a"),
            ExecutionSymbol::new("XCME", instrument).unwrap(),
            id("USD"),
        )
    }

    fn fill(
        sequence: u64,
        execution_id: &str,
        side: OrderSide,
        quantity: i64,
        price: i64,
    ) -> LedgerFill {
        LedgerFill {
            sequence,
            execution_id: id(execution_id),
            client_order_id: id("client-a"),
            venue_order_id: id("venue-a"),
            route_id: id("route-a"),
            key: key("ESM6"),
            side,
            quantity: OrderQty(quantity),
            price: OrderPrice(price),
            contract_multiplier: 10,
            commission: 2,
            fees: 1,
            ts_exchange_ns: sequence * 10,
            ts_recv_ns: sequence * 10 + 1,
        }
    }

    fn ledger() -> ProductionPositionLedger {
        ProductionPositionLedger::new(ProductionPositionLedgerConfig::new(8, 32, 8))
    }

    #[test]
    fn average_cost_fill_close_reversal_and_mark_are_correct() {
        let mut ledger = ledger();
        ledger
            .apply_fill(fill(1, "exec-1", OrderSide::Buy, 10, 100))
            .unwrap();
        ledger
            .apply_fill(fill(2, "exec-2", OrderSide::Buy, 10, 110))
            .unwrap();
        let partial = ledger
            .apply_fill(fill(3, "exec-3", OrderSide::Sell, 5, 120))
            .unwrap();
        assert_eq!(partial.realized_pnl_delta, 750);
        assert_eq!(partial.position.net_qty, 15);
        assert_eq!(partial.position.average_price, 105);
        assert_eq!(partial.position.realized_pnl, 750);
        assert_eq!(partial.position.commissions, 6);
        assert_eq!(partial.position.fees, 3);

        let marked = ledger
            .apply_mark(LedgerMark {
                sequence: 4,
                key: key("ESM6"),
                price: OrderPrice(115),
                timestamp_ns: 40,
            })
            .unwrap();
        assert_eq!(marked.unrealized_pnl, 1_500);
        assert_eq!(marked.total_pnl(), 2_241);
        assert_eq!(
            marked.total_pnl_in_base(LedgerFxRate::new(3, 2).unwrap()),
            3_361
        );

        let reversal = ledger
            .apply_fill(fill(5, "exec-4", OrderSide::Sell, 20, 90))
            .unwrap();
        assert_eq!(reversal.realized_pnl_delta, -2_250);
        assert_eq!(reversal.position.net_qty, -5);
        assert_eq!(reversal.position.average_price, 90);
        assert_eq!(reversal.position.unrealized_pnl, -1_250);
    }

    #[test]
    fn short_position_realizes_on_buy_to_cover() {
        let mut ledger = ledger();
        ledger
            .apply_fill(fill(1, "short", OrderSide::Sell, 10, 100))
            .unwrap();
        let result = ledger
            .apply_fill(fill(2, "cover", OrderSide::Buy, 4, 90))
            .unwrap();
        assert_eq!(result.position.net_qty, -6);
        assert_eq!(result.position.average_price, 100);
        assert_eq!(result.realized_pnl_delta, 400);
    }

    #[test]
    fn exact_open_cost_prevents_average_price_rounding_drift() {
        let mut first = fill(1, "fraction-1", OrderSide::Buy, 1, 100);
        first.contract_multiplier = 1;
        first.commission = 0;
        first.fees = 0;
        let mut second = fill(2, "fraction-2", OrderSide::Buy, 2, 101);
        second.contract_multiplier = 1;
        second.commission = 0;
        second.fees = 0;
        let mut close = fill(3, "fraction-3", OrderSide::Sell, 3, 101);
        close.contract_multiplier = 1;
        close.commission = 0;
        close.fees = 0;
        let mut ledger = ledger();
        ledger.apply_fill(first).unwrap();
        let open = ledger.apply_fill(second).unwrap().position;
        assert_eq!(open.open_cost, 302);
        assert_eq!(open.average_price, 100);
        let closed = ledger.apply_fill(close).unwrap();
        assert_eq!(closed.realized_pnl_delta, 1);
        assert_eq!(closed.position.open_cost, 0);
    }

    #[test]
    fn duplicate_is_idempotent_and_identity_exhaustion_fails_closed() {
        let mut ledger =
            ProductionPositionLedger::new(ProductionPositionLedgerConfig::new(1, 1, 1));
        let first = fill(1, "same", OrderSide::Buy, 1, 100);
        assert_eq!(
            ledger.apply_fill(first).unwrap().status,
            LedgerApplyStatus::Applied
        );
        let mut retry = first;
        retry.sequence = 2;
        assert_eq!(
            ledger.apply_fill(retry).unwrap().status,
            LedgerApplyStatus::Duplicate
        );
        assert_eq!(ledger.last_sequence(), 1);
        assert_eq!(
            ledger
                .apply_fill(fill(2, "new", OrderSide::Buy, 1, 100))
                .unwrap_err()
                .to_string(),
            PositionLedgerError::MutationIdentityCapacityExceeded.to_string()
        );
        assert_eq!(ledger.position(&key("ESM6")).unwrap().net_qty, 1);
    }

    #[test]
    fn execution_and_adjustment_ids_are_scoped_to_their_owners() {
        let mut ledger =
            ProductionPositionLedger::new(ProductionPositionLedgerConfig::new(4, 8, 0));
        ledger
            .apply_fill(fill(1, "provider-1", OrderSide::Buy, 1, 100))
            .unwrap();
        let mut other_route = fill(2, "provider-1", OrderSide::Buy, 1, 100);
        other_route.route_id = id("route-b");
        other_route.key = key("NQM6");
        assert_eq!(
            ledger.apply_fill(other_route).unwrap().status,
            LedgerApplyStatus::Applied
        );

        for (sequence, instrument) in [(3, "ESM6"), (4, "NQM6")] {
            let adjustment = LedgerAdjustment {
                sequence,
                adjustment_id: id("same-host-id"),
                kind: LedgerAdjustmentKind::Cash,
                key: key(instrument),
                quantity_delta: 0,
                average_price_override: None,
                realized_pnl_delta: 0,
                commission_delta: 0,
                fee_delta: 0,
                cash_delta: 1,
                contract_multiplier_override: 0,
                timestamp_ns: sequence,
            };
            assert_eq!(
                ledger.apply_adjustment(adjustment).unwrap().status,
                LedgerApplyStatus::Applied
            );
        }
    }

    #[test]
    fn sequence_capacity_and_multiplier_failures_are_atomic() {
        let mut ledger =
            ProductionPositionLedger::new(ProductionPositionLedgerConfig::new(1, 8, 0));
        ledger
            .apply_fill(fill(10, "exec-1", OrderSide::Buy, 1, 100))
            .unwrap();
        assert!(matches!(
            ledger.apply_fill(fill(9, "exec-2", OrderSide::Buy, 1, 100)),
            Err(PositionLedgerError::SequenceRegression { .. })
        ));
        let mut mismatch = fill(11, "exec-3", OrderSide::Buy, 1, 100);
        mismatch.contract_multiplier = 5;
        assert!(matches!(
            ledger.apply_fill(mismatch),
            Err(PositionLedgerError::ContractMultiplierMismatch)
        ));
        let mut other = fill(11, "exec-4", OrderSide::Buy, 1, 100);
        other.key = key("NQM6");
        assert!(matches!(
            ledger.apply_fill(other),
            Err(PositionLedgerError::PositionCapacityExceeded)
        ));
        assert_eq!(ledger.last_sequence(), 10);
        assert_eq!(ledger.position_count(), 1);
    }

    #[test]
    fn corporate_action_and_cash_adjustments_are_auditable() {
        let mut ledger = ledger();
        let opening = LedgerAdjustment {
            sequence: 1,
            adjustment_id: id("opening"),
            kind: LedgerAdjustmentKind::OpeningBalance,
            key: key("ESM6"),
            quantity_delta: 10,
            average_price_override: Some(OrderPrice(100)),
            realized_pnl_delta: 0,
            commission_delta: 0,
            fee_delta: 0,
            cash_delta: 0,
            contract_multiplier_override: 1,
            timestamp_ns: 1,
        };
        ledger.apply_adjustment(opening).unwrap();
        let split = LedgerAdjustment {
            sequence: 2,
            adjustment_id: id("split-2-for-1"),
            kind: LedgerAdjustmentKind::CorporateAction,
            key: key("ESM6"),
            quantity_delta: 10,
            average_price_override: Some(OrderPrice(50)),
            realized_pnl_delta: 0,
            commission_delta: 0,
            fee_delta: 0,
            cash_delta: 25,
            contract_multiplier_override: 0,
            timestamp_ns: 2,
        };
        let result = ledger.apply_adjustment(split).unwrap();
        assert_eq!(result.position.net_qty, 20);
        assert_eq!(result.position.average_price, 50);
        assert_eq!(result.position.cash_balance, 25);
        assert_eq!(result.position.buy_qty, 0);
        assert_eq!(result.position.sell_qty, 0);

        let mut invalid = split;
        invalid.sequence = 3;
        invalid.adjustment_id = id("invalid-cost");
        invalid.commission_delta = -1;
        assert!(matches!(
            ledger.apply_adjustment(invalid),
            Err(PositionLedgerError::NegativeAccumulatedCosts)
        ));
        assert_eq!(ledger.last_sequence(), 2);
    }

    #[test]
    fn execution_event_mapping_validates_trade_economics() {
        let mut event = ExecutionEvent {
            exec_type: ExecutionType::Trade,
            order_status: OrderStatus::PartiallyFilled,
            client_order_id: id("client"),
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: id("venue"),
            execution_id: id("exec"),
            account_id: id("account-a"),
            route_id: id("route-a"),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            last_qty: OrderQty(2),
            last_price: OrderPrice(100),
            cumulative_qty: OrderQty(2),
            leaves_qty: OrderQty(8),
            average_price: OrderPrice(100),
            ts_exchange_ns: 1,
            ts_recv_ns: 2,
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        };
        assert!(LedgerFill::from_execution_event(
            &event,
            1,
            id("strategy-a"),
            OrderSide::Buy,
            id("USD"),
            1,
            0,
            0,
        )
        .is_ok());
        event.exec_type = ExecutionType::Ack;
        assert!(matches!(
            LedgerFill::from_execution_event(
                &event,
                1,
                id("strategy-a"),
                OrderSide::Buy,
                id("USD"),
                1,
                0,
                0,
            ),
            Err(PositionLedgerError::NotTradeEvent)
        ));
    }

    #[test]
    fn checkpoint_round_trip_restores_deduplication_and_attribution() {
        let mut ledger = ledger();
        ledger
            .apply_fill(fill(1, "exec-1", OrderSide::Buy, 2, 100))
            .unwrap();
        ledger
            .apply_mark(LedgerMark {
                sequence: 2,
                key: key("ESM6"),
                price: OrderPrice(110),
                timestamp_ns: 2,
            })
            .unwrap();
        let checkpoint = ledger.checkpoint(7, 99);
        assert!(checkpoint.validate());
        let encoded = checkpoint.encode();
        let decoded = PositionLedgerCheckpoint::decode(&encoded).unwrap();
        assert_eq!(decoded, checkpoint);
        let mut restored = ProductionPositionLedger::restore(ledger.config(), &decoded).unwrap();
        assert_eq!(
            restored.position(&key("ESM6")),
            ledger.position(&key("ESM6"))
        );
        assert_eq!(restored.recent_attributions().len(), 1);
        let mut duplicate = fill(3, "exec-1", OrderSide::Buy, 2, 100);
        duplicate.ts_recv_ns = 3;
        assert_eq!(
            restored.apply_fill(duplicate).unwrap().status,
            LedgerApplyStatus::Duplicate
        );

        let mut corrupt = encoded;
        let last = corrupt.last_mut().unwrap();
        *last ^= 1;
        assert!(matches!(
            PositionLedgerCheckpoint::decode(&corrupt),
            Err(PositionLedgerError::InvalidCheckpoint)
        ));
    }

    #[test]
    fn file_checkpoint_store_installs_loads_and_prunes_atomically() {
        let root = std::env::temp_dir().join(format!(
            "orderflow-production-ledger-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let mut store = FilePositionLedgerCheckpointStore::open(
            PositionLedgerCheckpointConfig::new(&root)
                .with_sync_on_save(false)
                .with_max_retained(1),
        )
        .unwrap();
        let mut ledger = ledger();
        ledger
            .apply_fill(fill(1, "exec-1", OrderSide::Buy, 1, 100))
            .unwrap();
        store.save(&ledger.checkpoint(1, 10)).unwrap();
        store.save(&ledger.checkpoint(2, 20)).unwrap();
        let manifests = store.list().unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].checkpoint_id, 2);
        let latest = store.load_latest().unwrap().unwrap();
        assert_eq!(latest.checkpoint_id, 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reconciliation_reports_all_mismatch_and_duplicate_classes() {
        let mut ledger = ledger();
        ledger
            .apply_fill(fill(1, "exec-1", OrderSide::Buy, 2, 100))
            .unwrap();
        let local = ledger.position(&key("ESM6")).unwrap();
        let matching = ExternalPositionSnapshot {
            key: key("ESM6"),
            net_qty: local.net_qty,
            average_price: local.average_price,
            contract_multiplier: local.contract_multiplier,
            realized_pnl: local.realized_pnl,
            commissions: local.commissions,
            fees: local.fees,
            as_of_ns: 10,
        };
        let mut mismatch = matching;
        mismatch.net_qty = 3;
        mismatch.average_price = 105;
        let external_only = ExternalPositionSnapshot {
            key: key("NQM6"),
            ..matching
        };
        let mut out = PositionReconciliationBuffer::with_capacity(4);
        let report = reconcile_production_positions(
            &ledger,
            &[mismatch, matching, external_only],
            PositionReconciliationTolerance::default(),
            &mut out,
        )
        .unwrap();
        assert_eq!(report.compared, 3);
        assert_eq!(report.mismatched, 3);
        assert_eq!(report.duplicate_external_keys, 1);
        assert_eq!(report.external_only, 1);
        assert!(out.as_slice()[0]
            .issues
            .contains(PositionReconciliationIssueFlags::NET_QUANTITY));
        assert!(out.as_slice()[0]
            .issues
            .contains(PositionReconciliationIssueFlags::AVERAGE_PRICE));
        assert!(out.as_slice()[1]
            .issues
            .contains(PositionReconciliationIssueFlags::DUPLICATE_EXTERNAL_KEY));
    }

    #[test]
    fn reconciliation_buffer_never_silently_truncates() {
        let mut ledger = ledger();
        ledger
            .apply_fill(fill(1, "exec-1", OrderSide::Buy, 1, 100))
            .unwrap();
        let mut out = PositionReconciliationBuffer::with_capacity(0);
        assert!(matches!(
            reconcile_production_positions(
                &ledger,
                &[],
                PositionReconciliationTolerance::default(),
                &mut out,
            ),
            Err(PositionLedgerError::ReconciliationBufferFull)
        ));
    }
}
