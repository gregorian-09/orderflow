use super::*;

/// Structured risk rejection reason.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiskRejectReason {
    /// No rejection.
    None = 0,
    /// Kill switch is active.
    KillSwitch = 1,
    /// Account is not enabled.
    AccountDisabled = 2,
    /// Route is not enabled.
    RouteDisabled = 3,
    /// Symbol is not enabled.
    SymbolDisabled = 4,
    /// Quantity exceeds configured max.
    MaxOrderQty = 5,
    /// Notional exceeds configured max.
    MaxOrderNotional = 6,
    /// Open order count exceeds configured max.
    MaxOpenOrders = 7,
    /// Open notional exceeds configured max.
    MaxOpenNotional = 8,
    /// Price is outside configured band.
    PriceBand = 9,
    /// Client order id is already in use.
    DuplicateClientOrderId = 10,
    /// Order type is unsupported on the route.
    UnsupportedOrderType = 11,
    /// Time-in-force is unsupported on the route.
    UnsupportedTimeInForce = 12,
}

/// Risk decision.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskDecision {
    /// True when the request is allowed to route.
    pub allowed: bool,
    /// Structured reject reason.
    pub reason: RiskRejectReason,
    /// Bounded diagnostic text.
    pub text: ExecutionText,
}

impl RiskDecision {
    /// Creates an allow decision.
    pub const fn allow() -> Self {
        Self {
            allowed: true,
            reason: RiskRejectReason::None,
            text: ExecutionText::empty(),
        }
    }

    /// Creates a reject decision.
    pub fn reject(reason: RiskRejectReason, text: ExecutionText) -> Self {
        Self {
            allowed: false,
            reason,
            text,
        }
    }
}

/// Static risk limits for one route/account scope.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskLimits {
    /// Kill switch flag.
    pub kill_switch: bool,
    /// Maximum quantity per order. Zero disables the check.
    pub max_order_qty: i64,
    /// Maximum notional per order. Zero disables the check.
    pub max_order_notional: i128,
    /// Maximum open orders. Zero disables the check.
    pub max_open_orders: u32,
    /// Maximum open notional. Zero disables the check.
    pub max_open_notional: i128,
    /// Allowed absolute distance from reference price. Zero disables the check.
    pub price_band_ticks: i64,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            kill_switch: true,
            max_order_qty: 0,
            max_order_notional: 0,
            max_open_orders: 0,
            max_open_notional: 0,
            price_band_ticks: 0,
        }
    }
}

/// Runtime risk context supplied by the execution engine.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskContext {
    /// Current open order count for the account/route scope.
    pub open_orders: u32,
    /// Current open notional for the account/route scope.
    pub open_notional: i128,
    /// Reference price used for price-band checks. Zero disables price-band checks.
    pub reference_price: OrderPrice,
    /// True when the client order id is already known.
    pub duplicate_client_order_id: bool,
    /// True when account is enabled.
    pub account_enabled: bool,
    /// True when route is enabled.
    pub route_enabled: bool,
    /// True when symbol is enabled.
    pub symbol_enabled: bool,
    /// True when order type is supported.
    pub order_type_supported: bool,
    /// True when time-in-force is supported.
    pub tif_supported: bool,
}

impl Default for RiskContext {
    fn default() -> Self {
        Self {
            open_orders: 0,
            open_notional: 0,
            reference_price: OrderPrice(0),
            duplicate_client_order_id: false,
            account_enabled: false,
            route_enabled: false,
            symbol_enabled: false,
            order_type_supported: false,
            tif_supported: false,
        }
    }
}

/// Pre-trade risk-check contract.
pub trait RiskCheck: Send + Sync {
    /// Checks a new order request.
    fn check_new(&self, req: &OrderRequest, ctx: &RiskContext) -> RiskDecision;
    /// Checks an amend request.
    fn check_amend(&self, req: &AmendRequest, ctx: &RiskContext) -> RiskDecision;
    /// Checks a cancel request.
    fn check_cancel(&self, req: &CancelRequest, ctx: &RiskContext) -> RiskDecision;
}

/// Deterministic pre-trade risk gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasicRiskGate {
    limits: RiskLimits,
}

impl BasicRiskGate {
    /// Creates a risk gate from static limits.
    pub const fn new(limits: RiskLimits) -> Self {
        Self { limits }
    }

    fn check_common(&self, ctx: &RiskContext) -> RiskDecision {
        if self.limits.kill_switch {
            return reject(RiskRejectReason::KillSwitch, "kill switch active");
        }
        if !ctx.account_enabled {
            return reject(RiskRejectReason::AccountDisabled, "account disabled");
        }
        if !ctx.route_enabled {
            return reject(RiskRejectReason::RouteDisabled, "route disabled");
        }
        if !ctx.symbol_enabled {
            return reject(RiskRejectReason::SymbolDisabled, "symbol disabled");
        }
        if ctx.duplicate_client_order_id {
            return reject(
                RiskRejectReason::DuplicateClientOrderId,
                "duplicate client order id",
            );
        }
        if !ctx.order_type_supported {
            return reject(
                RiskRejectReason::UnsupportedOrderType,
                "unsupported order type",
            );
        }
        if !ctx.tif_supported {
            return reject(
                RiskRejectReason::UnsupportedTimeInForce,
                "unsupported time in force",
            );
        }
        if self.limits.max_open_orders > 0 && ctx.open_orders >= self.limits.max_open_orders {
            return reject(RiskRejectReason::MaxOpenOrders, "max open orders exceeded");
        }
        if self.limits.max_open_notional > 0 && ctx.open_notional >= self.limits.max_open_notional {
            return reject(
                RiskRejectReason::MaxOpenNotional,
                "max open notional exceeded",
            );
        }
        RiskDecision::allow()
    }

    fn check_size_price(
        &self,
        qty: OrderQty,
        price: OrderPrice,
        ctx: &RiskContext,
    ) -> RiskDecision {
        if self.limits.max_order_qty > 0 && qty.0 > self.limits.max_order_qty {
            return reject(RiskRejectReason::MaxOrderQty, "max order quantity exceeded");
        }
        if self.limits.max_order_notional > 0 {
            let notional = i128::from(qty.0).saturating_mul(i128::from(price.0));
            if notional > self.limits.max_order_notional {
                return reject(
                    RiskRejectReason::MaxOrderNotional,
                    "max order notional exceeded",
                );
            }
        }
        if self.limits.price_band_ticks > 0 && ctx.reference_price.0 > 0 && price.0 > 0 {
            let distance = price.0.saturating_sub(ctx.reference_price.0).abs();
            if distance > self.limits.price_band_ticks {
                return reject(RiskRejectReason::PriceBand, "price outside risk band");
            }
        }
        RiskDecision::allow()
    }
}

impl RiskCheck for BasicRiskGate {
    fn check_new(&self, req: &OrderRequest, ctx: &RiskContext) -> RiskDecision {
        let common = self.check_common(ctx);
        if !common.allowed {
            return common;
        }
        self.check_size_price(req.quantity, req.limit_price, ctx)
    }

    fn check_amend(&self, req: &AmendRequest, ctx: &RiskContext) -> RiskDecision {
        let common = self.check_common(ctx);
        if !common.allowed {
            return common;
        }
        self.check_size_price(req.quantity, req.limit_price, ctx)
    }

    fn check_cancel(&self, _req: &CancelRequest, ctx: &RiskContext) -> RiskDecision {
        if self.limits.kill_switch {
            return reject(RiskRejectReason::KillSwitch, "kill switch active");
        }
        if !ctx.account_enabled {
            return reject(RiskRejectReason::AccountDisabled, "account disabled");
        }
        if !ctx.route_enabled {
            return reject(RiskRejectReason::RouteDisabled, "route disabled");
        }
        RiskDecision::allow()
    }
}

fn reject(reason: RiskRejectReason, text: &str) -> RiskDecision {
    let text = ExecutionText::new(text).unwrap_or_else(|_| ExecutionText::empty());
    RiskDecision::reject(reason, text)
}
