use super::*;

/// Market-making context supplied by the host quote model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketMakerContext {
    fair_value: OrderPrice,
    best_bid: OrderPrice,
    best_ask: OrderPrice,
    inventory_qty: OrderQty,
    max_inventory_qty: OrderQty,
    volatility_bps: u16,
    adverse_selection_bps: u16,
}

impl MarketMakerContext {
    /// Creates market-making quote context.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidMarketMakingParameters`] when prices are
    /// non-positive/crossed or maximum inventory is not positive.
    #[allow(
        clippy::too_many_arguments,
        reason = "flat context mirrors one hot-path quote snapshot"
    )]
    pub const fn new(
        fair_value: OrderPrice,
        best_bid: OrderPrice,
        best_ask: OrderPrice,
        inventory_qty: OrderQty,
        max_inventory_qty: OrderQty,
        volatility_bps: u16,
        adverse_selection_bps: u16,
    ) -> Result<Self, AlgoError> {
        if fair_value.0 <= 0
            || best_bid.0 <= 0
            || best_ask.0 <= 0
            || best_bid.0 >= best_ask.0
            || max_inventory_qty.0 <= 0
        {
            return Err(AlgoError::InvalidMarketMakingParameters);
        }
        Ok(Self {
            fair_value,
            best_bid,
            best_ask,
            inventory_qty,
            max_inventory_qty,
            volatility_bps,
            adverse_selection_bps,
        })
    }

    /// Returns model fair value.
    pub const fn fair_value(&self) -> OrderPrice {
        self.fair_value
    }

    /// Returns current best bid.
    pub const fn best_bid(&self) -> OrderPrice {
        self.best_bid
    }

    /// Returns current best ask.
    pub const fn best_ask(&self) -> OrderPrice {
        self.best_ask
    }

    /// Returns signed inventory quantity.
    pub const fn inventory_qty(&self) -> OrderQty {
        self.inventory_qty
    }

    /// Returns absolute maximum inventory quantity.
    pub const fn max_inventory_qty(&self) -> OrderQty {
        self.max_inventory_qty
    }

    /// Returns volatility estimate in basis points.
    pub const fn volatility_bps(&self) -> u16 {
        self.volatility_bps
    }

    /// Returns adverse-selection estimate in basis points.
    pub const fn adverse_selection_bps(&self) -> u16 {
        self.adverse_selection_bps
    }
}

/// Market-making quote configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketMakerConfig {
    tick_size: OrderPrice,
    quote_qty: OrderQty,
    base_spread_bps: u16,
    min_spread_ticks: u16,
    max_spread_bps: u16,
    inventory_skew_bps: u16,
    volatility_weight_bps: u16,
    adverse_selection_weight_bps: u16,
}

impl MarketMakerConfig {
    /// Creates market-making configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidMarketMakingParameters`] when tick size,
    /// quote quantity, or spread bounds are invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "flat constructor keeps quote controls explicit"
    )]
    pub const fn new(
        tick_size: OrderPrice,
        quote_qty: OrderQty,
        base_spread_bps: u16,
        min_spread_ticks: u16,
        max_spread_bps: u16,
        inventory_skew_bps: u16,
        volatility_weight_bps: u16,
        adverse_selection_weight_bps: u16,
    ) -> Result<Self, AlgoError> {
        if tick_size.0 <= 0
            || quote_qty.0 <= 0
            || min_spread_ticks == 0
            || base_spread_bps > 10_000
            || max_spread_bps > 10_000
            || base_spread_bps > max_spread_bps
        {
            return Err(AlgoError::InvalidMarketMakingParameters);
        }
        Ok(Self {
            tick_size,
            quote_qty,
            base_spread_bps,
            min_spread_ticks,
            max_spread_bps,
            inventory_skew_bps,
            volatility_weight_bps,
            adverse_selection_weight_bps,
        })
    }

    /// Returns tick size in normalized price units.
    pub const fn tick_size(&self) -> OrderPrice {
        self.tick_size
    }

    /// Returns quote quantity.
    pub const fn quote_qty(&self) -> OrderQty {
        self.quote_qty
    }

    /// Returns base spread in basis points.
    pub const fn base_spread_bps(&self) -> u16 {
        self.base_spread_bps
    }

    /// Returns minimum spread in ticks.
    pub const fn min_spread_ticks(&self) -> u16 {
        self.min_spread_ticks
    }

    /// Returns maximum spread in basis points.
    pub const fn max_spread_bps(&self) -> u16 {
        self.max_spread_bps
    }

    /// Returns inventory skew strength in basis points.
    pub const fn inventory_skew_bps(&self) -> u16 {
        self.inventory_skew_bps
    }

    /// Returns volatility spread weight in basis points.
    pub const fn volatility_weight_bps(&self) -> u16 {
        self.volatility_weight_bps
    }

    /// Returns adverse-selection spread weight in basis points.
    pub const fn adverse_selection_weight_bps(&self) -> u16 {
        self.adverse_selection_weight_bps
    }
}

impl Default for MarketMakerConfig {
    fn default() -> Self {
        Self::new(
            OrderPrice(1),
            OrderQty(1),
            10,
            1,
            1_000,
            1_000,
            5_000,
            5_000,
        )
        .expect("static market-making defaults are valid")
    }
}

/// Market-making quote estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketMakerQuoteEstimate {
    adjusted_fair_value: OrderPrice,
    bid_price: OrderPrice,
    ask_price: OrderPrice,
    spread_bps: u16,
    quote_bid: bool,
    quote_ask: bool,
}

impl MarketMakerQuoteEstimate {
    /// Returns inventory-skewed fair value.
    pub const fn adjusted_fair_value(&self) -> OrderPrice {
        self.adjusted_fair_value
    }

    /// Returns bid quote price.
    pub const fn bid_price(&self) -> OrderPrice {
        self.bid_price
    }

    /// Returns ask quote price.
    pub const fn ask_price(&self) -> OrderPrice {
        self.ask_price
    }

    /// Returns selected spread in basis points.
    pub const fn spread_bps(&self) -> u16 {
        self.spread_bps
    }

    /// Returns true when a bid quote is allowed.
    pub const fn quote_bid(&self) -> bool {
        self.quote_bid
    }

    /// Returns true when an ask quote is allowed.
    pub const fn quote_ask(&self) -> bool {
        self.quote_ask
    }
}

/// Market-making quote decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketMakerQuoteDecision {
    estimate: MarketMakerQuoteEstimate,
    bid: Option<ChildOrderPlan>,
    ask: Option<ChildOrderPlan>,
}

impl MarketMakerQuoteDecision {
    /// Returns the quote estimate.
    pub const fn estimate(&self) -> MarketMakerQuoteEstimate {
        self.estimate
    }

    /// Returns optional bid child order plan.
    pub const fn bid(&self) -> Option<ChildOrderPlan> {
        self.bid
    }

    /// Returns optional ask child order plan.
    pub const fn ask(&self) -> Option<ChildOrderPlan> {
        self.ask
    }
}

/// Deterministic market-making quote planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketMakerPlanner {
    config: MarketMakerConfig,
}

impl MarketMakerPlanner {
    /// Creates a market-making planner.
    pub const fn new(config: MarketMakerConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> MarketMakerConfig {
        self.config
    }

    /// Estimates bid/ask quote prices and inventory-side quote suppression.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when template or context is invalid.
    pub fn estimate(
        &self,
        template: &ParentOrder,
        context: MarketMakerContext,
    ) -> Result<MarketMakerQuoteEstimate, AlgoError> {
        template.validate()?;
        self.validate_context(context)?;
        let inventory_ratio_bps =
            signed_inventory_ratio_bps(context.inventory_qty().0, context.max_inventory_qty().0);
        let skew_bps = scale_signed_bps(
            inventory_ratio_bps,
            i32::from(self.config.inventory_skew_bps()),
        );
        let adjusted_fair_value = apply_price_bps(context.fair_value(), -skew_bps);

        let mut spread_bps = self.config.base_spread_bps();
        spread_bps = spread_bps.saturating_add(
            u16::try_from(scale_bps_u32(
                u32::from(context.volatility_bps()),
                u32::from(self.config.volatility_weight_bps()),
            ))
            .unwrap_or(u16::MAX),
        );
        spread_bps = spread_bps.saturating_add(
            u16::try_from(scale_bps_u32(
                u32::from(context.adverse_selection_bps()),
                u32::from(self.config.adverse_selection_weight_bps()),
            ))
            .unwrap_or(u16::MAX),
        );
        spread_bps = spread_bps.min(self.config.max_spread_bps());

        let min_spread = self
            .config
            .tick_size()
            .0
            .saturating_mul(i64::from(self.config.min_spread_ticks()));
        let spread_price = price_bps_to_ticks(adjusted_fair_value, spread_bps).max(min_spread);
        let half = (spread_price / 2).max(self.config.tick_size().0);
        let bid_price = snap_down_to_tick(
            adjusted_fair_value.0.saturating_sub(half),
            self.config.tick_size().0,
        );
        let ask_price = snap_up_to_tick(
            adjusted_fair_value.0.saturating_add(half),
            self.config.tick_size().0,
        );
        let quote_bid = context.inventory_qty().0 < context.max_inventory_qty().0;
        let quote_ask = context.inventory_qty().0 > -context.max_inventory_qty().0;

        Ok(MarketMakerQuoteEstimate {
            adjusted_fair_value,
            bid_price: OrderPrice(bid_price.max(self.config.tick_size().0)),
            ask_price: OrderPrice(
                ask_price.max(bid_price.saturating_add(self.config.tick_size().0)),
            ),
            spread_bps,
            quote_bid,
            quote_ask,
        })
    }

    /// Plans bid and ask child quote orders.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when inputs are invalid or a generated quote order
    /// fails OMS request validation.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns quote identifiers and timestamps"
    )]
    pub fn plan_quotes(
        &self,
        template: &ParentOrder,
        now_ns: u64,
        context: MarketMakerContext,
        bid_child_id: ChildOrderId,
        bid_client_order_id: ClientOrderId,
        ask_child_id: ChildOrderId,
        ask_client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<MarketMakerQuoteDecision, AlgoError> {
        if template.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        let estimate = self.estimate(template, context)?;
        let bid = if estimate.quote_bid() {
            let request = template.build_order_request_for_side_at_price(
                OrderSide::Buy,
                bid_client_order_id,
                self.config.quote_qty(),
                estimate.bid_price(),
                ts_recv_ns,
            );
            Some(ChildOrderPlan::new(
                bid_child_id,
                template.id(),
                request,
                now_ns,
            )?)
        } else {
            None
        };
        let ask = if estimate.quote_ask() {
            let request = template.build_order_request_for_side_at_price(
                OrderSide::Sell,
                ask_client_order_id,
                self.config.quote_qty(),
                estimate.ask_price(),
                ts_recv_ns,
            );
            Some(ChildOrderPlan::new(
                ask_child_id,
                template.id(),
                request,
                now_ns,
            )?)
        } else {
            None
        };
        Ok(MarketMakerQuoteDecision { estimate, bid, ask })
    }

    fn validate_context(&self, context: MarketMakerContext) -> Result<(), AlgoError> {
        if self.config.tick_size().0 <= 0
            || self.config.quote_qty().0 <= 0
            || self.config.base_spread_bps() > self.config.max_spread_bps()
            || context.fair_value().0 <= 0
            || context.best_bid().0 <= 0
            || context.best_ask().0 <= 0
            || context.best_bid().0 >= context.best_ask().0
            || context.max_inventory_qty().0 <= 0
        {
            return Err(AlgoError::InvalidMarketMakingParameters);
        }
        Ok(())
    }
}
