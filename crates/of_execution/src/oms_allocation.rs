use super::*;

/// Allocation method for splitting a block fill across accounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum AllocationMethod {
    /// Allocate proportionally by each leg's configured weight.
    #[default]
    Proportional,
    /// Allocate sequentially by leg priority until each target quantity is met.
    Priority,
}

/// One account/route target in an allocation group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AllocationLeg {
    /// Destination account.
    pub account_id: AccountId,
    /// Destination route.
    pub route_id: RouteId,
    /// Destination strategy.
    pub strategy_id: StrategyId,
    /// Positive proportional weight.
    pub weight: u32,
    /// Optional target quantity for priority allocation. Zero means unlimited.
    pub target_qty: OrderQty,
    /// Lower values receive priority allocation first.
    pub priority: u16,
}

impl AllocationLeg {
    /// Creates a proportional allocation leg.
    pub const fn proportional(
        account_id: AccountId,
        route_id: RouteId,
        strategy_id: StrategyId,
        weight: u32,
    ) -> Self {
        Self {
            account_id,
            route_id,
            strategy_id,
            weight,
            target_qty: OrderQty(0),
            priority: 0,
        }
    }

    /// Creates a priority allocation leg.
    pub const fn priority(
        account_id: AccountId,
        route_id: RouteId,
        strategy_id: StrategyId,
        target_qty: OrderQty,
        priority: u16,
    ) -> Self {
        Self {
            account_id,
            route_id,
            strategy_id,
            weight: 1,
            target_qty,
            priority,
        }
    }
}

/// Allocation group definition.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct AllocationGroup {
    /// Allocation method.
    pub method: AllocationMethod,
    /// Allocation legs.
    pub legs: Vec<AllocationLeg>,
}

impl AllocationGroup {
    /// Creates an allocation group.
    pub fn new(method: AllocationMethod, legs: Vec<AllocationLeg>) -> Self {
        Self { method, legs }
    }
}

/// One allocated fill destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AllocationFill {
    /// Destination account.
    pub account_id: AccountId,
    /// Destination route.
    pub route_id: RouteId,
    /// Destination strategy.
    pub strategy_id: StrategyId,
    /// Allocated quantity.
    pub quantity: OrderQty,
    /// Average allocation price.
    pub average_price: OrderPrice,
}

/// Allocation report for one block fill.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct AllocationReport {
    /// Allocation method used.
    pub method: AllocationMethod,
    /// Original block quantity.
    pub block_qty: OrderQty,
    /// Average block price.
    pub average_price: OrderPrice,
    /// Allocation fills.
    pub fills: Vec<AllocationFill>,
    /// True when allocated quantity equals block quantity.
    pub balanced: bool,
    /// Quantity that could not be allocated.
    pub unallocated_qty: OrderQty,
}

impl AllocationReport {
    /// Returns allocated quantity.
    pub fn allocated_qty(&self) -> OrderQty {
        OrderQty(self.fills.iter().map(|fill| fill.quantity.0).sum())
    }
}

/// Allocation validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AllocationError {
    /// Allocation group has no legs.
    EmptyGroup,
    /// Block quantity is zero or negative.
    EmptyBlock,
    /// A proportional group has no positive weight.
    ZeroTotalWeight,
}

impl std::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyGroup => write!(f, "allocation group has no legs"),
            Self::EmptyBlock => write!(f, "allocation block quantity must be positive"),
            Self::ZeroTotalWeight => write!(f, "allocation group has no positive weights"),
        }
    }
}

impl std::error::Error for AllocationError {}

/// Allocates a block fill across an allocation group.
pub fn allocate_block_fill(
    group: &AllocationGroup,
    block_qty: OrderQty,
    average_price: OrderPrice,
) -> Result<AllocationReport, AllocationError> {
    if group.legs.is_empty() {
        return Err(AllocationError::EmptyGroup);
    }
    if block_qty.0 <= 0 {
        return Err(AllocationError::EmptyBlock);
    }
    let block_qty_value = block_qty.0;

    let quantities = match group.method {
        AllocationMethod::Proportional => proportional_quantities(&group.legs, block_qty_value)?,
        AllocationMethod::Priority => priority_quantities(&group.legs, block_qty_value),
    };
    let fills = group
        .legs
        .iter()
        .zip(quantities)
        .filter_map(|(leg, quantity)| {
            (quantity > 0).then_some(AllocationFill {
                account_id: leg.account_id,
                route_id: leg.route_id,
                strategy_id: leg.strategy_id,
                quantity: OrderQty(quantity),
                average_price,
            })
        })
        .collect::<Vec<_>>();
    let allocated = fills.iter().map(|fill| fill.quantity.0).sum::<i64>();
    Ok(AllocationReport {
        method: group.method,
        block_qty,
        average_price,
        fills,
        balanced: allocated == block_qty_value,
        unallocated_qty: OrderQty(block_qty_value.saturating_sub(allocated)),
    })
}

pub(crate) fn proportional_quantities(
    legs: &[AllocationLeg],
    block_qty: i64,
) -> Result<Vec<i64>, AllocationError> {
    let total_weight = legs.iter().map(|leg| u64::from(leg.weight)).sum::<u64>();
    if total_weight == 0 {
        return Err(AllocationError::ZeroTotalWeight);
    }

    let block_qty = block_qty as u64;
    let mut allocated = vec![0_i64; legs.len()];
    let mut remainders = Vec::with_capacity(legs.len());
    let mut total_allocated = 0_u64;
    for (index, leg) in legs.iter().enumerate() {
        let weighted = u128::from(block_qty).saturating_mul(u128::from(leg.weight));
        let base = (weighted / u128::from(total_weight)) as u64;
        let remainder = (weighted % u128::from(total_weight)) as u64;
        allocated[index] = base as i64;
        total_allocated = total_allocated.saturating_add(base);
        remainders.push((index, remainder, leg.priority));
    }

    remainders.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.0.cmp(&right.0))
    });
    let mut remainder_qty = block_qty.saturating_sub(total_allocated);
    for (index, _, _) in remainders {
        if remainder_qty == 0 {
            break;
        }
        allocated[index] = allocated[index].saturating_add(1);
        remainder_qty -= 1;
    }
    Ok(allocated)
}

pub(crate) fn priority_quantities(legs: &[AllocationLeg], block_qty: i64) -> Vec<i64> {
    let mut order = legs
        .iter()
        .enumerate()
        .map(|(index, leg)| (index, leg.priority))
        .collect::<Vec<_>>();
    order.sort_by_key(|(index, priority)| (*priority, *index));

    let mut remaining = block_qty as u64;
    let mut allocated = vec![0_i64; legs.len()];
    for (index, _) in order {
        if remaining == 0 {
            break;
        }
        let cap = if legs[index].target_qty.0 == 0 {
            remaining
        } else {
            legs[index].target_qty.0.max(0) as u64
        };
        let quantity = remaining.min(cap);
        allocated[index] = quantity as i64;
        remaining -= quantity;
    }
    allocated
}

/// Allocation reconciliation issue classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AllocationReconciliationIssue {
    /// Expected and actual allocation match.
    Matched,
    /// Expected allocation is missing from actual allocations.
    MissingActual,
    /// Actual allocation was not expected.
    UnexpectedActual,
    /// Allocated quantity differs.
    QuantityMismatch,
    /// Average allocation price differs.
    PriceMismatch,
}

/// One allocation reconciliation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AllocationReconciliationDetail {
    /// Destination account.
    pub account_id: AccountId,
    /// Destination route.
    pub route_id: RouteId,
    /// Destination strategy.
    pub strategy_id: StrategyId,
    /// Issue found.
    pub issue: AllocationReconciliationIssue,
    /// Expected fill when present.
    pub expected: Option<AllocationFill>,
    /// Actual fill when present.
    pub actual: Option<AllocationFill>,
}

/// Allocation reconciliation report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct AllocationReconciliationReport {
    /// Per-destination reconciliation findings.
    pub details: Vec<AllocationReconciliationDetail>,
}

impl AllocationReconciliationReport {
    /// Returns true when all allocation destinations match.
    pub fn is_clean(&self) -> bool {
        self.details
            .iter()
            .all(|detail| detail.issue == AllocationReconciliationIssue::Matched)
    }
}

/// Compares expected allocation fills to actual allocation fills.
pub fn reconcile_allocations(
    expected: &[AllocationFill],
    actual: &[AllocationFill],
) -> AllocationReconciliationReport {
    let mut actual_by_key: HashMap<(AccountId, RouteId, StrategyId), AllocationFill> =
        HashMap::with_capacity(actual.len());
    for fill in actual {
        actual_by_key.insert((fill.account_id, fill.route_id, fill.strategy_id), *fill);
    }

    let mut report = AllocationReconciliationReport::default();
    for expected_fill in expected {
        let key = (
            expected_fill.account_id,
            expected_fill.route_id,
            expected_fill.strategy_id,
        );
        match actual_by_key.remove(&key) {
            Some(actual_fill) => report.details.push(AllocationReconciliationDetail {
                account_id: expected_fill.account_id,
                route_id: expected_fill.route_id,
                strategy_id: expected_fill.strategy_id,
                issue: classify_allocation_issue(expected_fill, &actual_fill),
                expected: Some(*expected_fill),
                actual: Some(actual_fill),
            }),
            None => report.details.push(AllocationReconciliationDetail {
                account_id: expected_fill.account_id,
                route_id: expected_fill.route_id,
                strategy_id: expected_fill.strategy_id,
                issue: AllocationReconciliationIssue::MissingActual,
                expected: Some(*expected_fill),
                actual: None,
            }),
        }
    }

    for actual_fill in actual_by_key.into_values() {
        report.details.push(AllocationReconciliationDetail {
            account_id: actual_fill.account_id,
            route_id: actual_fill.route_id,
            strategy_id: actual_fill.strategy_id,
            issue: AllocationReconciliationIssue::UnexpectedActual,
            expected: None,
            actual: Some(actual_fill),
        });
    }
    report
}

pub(crate) fn classify_allocation_issue(
    expected: &AllocationFill,
    actual: &AllocationFill,
) -> AllocationReconciliationIssue {
    if expected == actual {
        AllocationReconciliationIssue::Matched
    } else if expected.quantity != actual.quantity {
        AllocationReconciliationIssue::QuantityMismatch
    } else if expected.average_price != actual.average_price {
        AllocationReconciliationIssue::PriceMismatch
    } else {
        AllocationReconciliationIssue::Matched
    }
}

pub(crate) fn classify_reconciliation_issue(
    local: &OrderState,
    venue: &OrderState,
) -> ReconciliationIssueKind {
    if local == venue {
        return ReconciliationIssueKind::Matched;
    }
    if local.account_id != venue.account_id
        || local.route_id != venue.route_id
        || local.symbol != venue.symbol
        || local.side != venue.side
    {
        return ReconciliationIssueKind::Unknown;
    }
    if local.status != venue.status {
        return ReconciliationIssueKind::StatusMismatch;
    }
    if local.order_qty != venue.order_qty
        || local.cumulative_qty != venue.cumulative_qty
        || local.leaves_qty != venue.leaves_qty
    {
        return ReconciliationIssueKind::QuantityMismatch;
    }
    if local.average_price != venue.average_price {
        return ReconciliationIssueKind::PriceMismatch;
    }
    ReconciliationIssueKind::Unknown
}
