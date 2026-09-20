use super::*;

/// Market context for implementation-shortfall planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplementationShortfallContext {
    arrival_price: OrderPrice,
    reference_price: OrderPrice,
    volatility_bps: u16,
    spread_bps: u16,
    temporary_impact_bps: u16,
}

impl ImplementationShortfallContext {
    /// Creates implementation-shortfall market context.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidShortfallParameters`] when either price is
    /// not positive.
    pub const fn new(
        arrival_price: OrderPrice,
        reference_price: OrderPrice,
        volatility_bps: u16,
        spread_bps: u16,
        temporary_impact_bps: u16,
    ) -> Result<Self, AlgoError> {
        if arrival_price.0 <= 0 || reference_price.0 <= 0 {
            return Err(AlgoError::InvalidShortfallParameters);
        }
        Ok(Self {
            arrival_price,
            reference_price,
            volatility_bps,
            spread_bps,
            temporary_impact_bps,
        })
    }

    /// Returns the arrival benchmark price.
    pub const fn arrival_price(&self) -> OrderPrice {
        self.arrival_price
    }

    /// Returns the current decision/reference price.
    pub const fn reference_price(&self) -> OrderPrice {
        self.reference_price
    }

    /// Returns estimated short-horizon volatility in basis points.
    pub const fn volatility_bps(&self) -> u16 {
        self.volatility_bps
    }

    /// Returns current spread in basis points.
    pub const fn spread_bps(&self) -> u16 {
        self.spread_bps
    }

    /// Returns temporary market-impact estimate in basis points.
    pub const fn temporary_impact_bps(&self) -> u16 {
        self.temporary_impact_bps
    }
}

/// Configuration for implementation-shortfall planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplementationShortfallConfig {
    base_urgency_bps: u16,
    max_urgency_bps: u16,
    volatility_weight_bps: u16,
    spread_weight_bps: u16,
    adverse_move_weight_bps: u16,
    impact_weight_bps: u16,
}

impl ImplementationShortfallConfig {
    /// Creates implementation-shortfall planner configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidShortfallParameters`] when urgency values
    /// exceed 10,000 bps or base urgency exceeds the cap.
    pub const fn new(
        base_urgency_bps: u16,
        max_urgency_bps: u16,
        volatility_weight_bps: u16,
        spread_weight_bps: u16,
        adverse_move_weight_bps: u16,
        impact_weight_bps: u16,
    ) -> Result<Self, AlgoError> {
        if base_urgency_bps > 10_000
            || max_urgency_bps > 10_000
            || base_urgency_bps > max_urgency_bps
        {
            return Err(AlgoError::InvalidShortfallParameters);
        }
        Ok(Self {
            base_urgency_bps,
            max_urgency_bps,
            volatility_weight_bps,
            spread_weight_bps,
            adverse_move_weight_bps,
            impact_weight_bps,
        })
    }

    /// Returns base urgency in basis points.
    pub const fn base_urgency_bps(&self) -> u16 {
        self.base_urgency_bps
    }

    /// Returns maximum urgency in basis points.
    pub const fn max_urgency_bps(&self) -> u16 {
        self.max_urgency_bps
    }

    /// Returns volatility urgency weight in basis points.
    pub const fn volatility_weight_bps(&self) -> u16 {
        self.volatility_weight_bps
    }

    /// Returns spread urgency weight in basis points.
    pub const fn spread_weight_bps(&self) -> u16 {
        self.spread_weight_bps
    }

    /// Returns adverse move urgency weight in basis points.
    pub const fn adverse_move_weight_bps(&self) -> u16 {
        self.adverse_move_weight_bps
    }

    /// Returns temporary impact patience weight in basis points.
    pub const fn impact_weight_bps(&self) -> u16 {
        self.impact_weight_bps
    }
}

impl Default for ImplementationShortfallConfig {
    fn default() -> Self {
        Self {
            base_urgency_bps: 1_000,
            max_urgency_bps: 7_500,
            volatility_weight_bps: 5_000,
            spread_weight_bps: 2_500,
            adverse_move_weight_bps: 7_500,
            impact_weight_bps: 3_000,
        }
    }
}

/// Implementation-shortfall planning estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplementationShortfallEstimate {
    elapsed_bps: u16,
    adverse_move_bps: u16,
    urgency_bps: u16,
    target_release_qty: OrderQty,
}

impl ImplementationShortfallEstimate {
    /// Returns elapsed parent interval in basis points.
    pub const fn elapsed_bps(&self) -> u16 {
        self.elapsed_bps
    }

    /// Returns adverse move from arrival price in basis points.
    pub const fn adverse_move_bps(&self) -> u16 {
        self.adverse_move_bps
    }

    /// Returns urgency used for the decision in basis points.
    pub const fn urgency_bps(&self) -> u16 {
        self.urgency_bps
    }

    /// Returns target cumulative released quantity.
    pub const fn target_release_qty(&self) -> OrderQty {
        self.target_release_qty
    }
}

/// Deterministic implementation-shortfall planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplementationShortfallPlanner {
    config: ImplementationShortfallConfig,
}

impl ImplementationShortfallPlanner {
    /// Creates an implementation-shortfall planner.
    pub const fn new(config: ImplementationShortfallConfig) -> Self {
        Self { config }
    }

    /// Returns planner configuration.
    pub const fn config(&self) -> ImplementationShortfallConfig {
        self.config
    }

    /// Estimates the current implementation-shortfall target.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/context state is invalid.
    pub fn estimate(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        context: ImplementationShortfallContext,
    ) -> Result<ImplementationShortfallEstimate, AlgoError> {
        parent.validate()?;
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if context.arrival_price().0 <= 0 || context.reference_price().0 <= 0 {
            return Err(AlgoError::InvalidShortfallParameters);
        }
        let elapsed_bps = elapsed_bps(parent.start_ns(), parent.end_ns(), now_ns);
        let adverse_move_bps = adverse_move_bps(
            parent.side(),
            context.arrival_price(),
            context.reference_price(),
        );
        let urgency_bps = self.urgency_bps(context, adverse_move_bps);
        let remaining_time_bps = 10_000_u16.saturating_sub(elapsed_bps);
        let target_bps = u32::from(elapsed_bps).saturating_add(scale_bps_u32(
            u32::from(remaining_time_bps),
            u32::from(urgency_bps),
        ));
        let target_release_qty = scale_qty_bps(parent.total_qty().0, target_bps.min(10_000));
        Ok(ImplementationShortfallEstimate {
            elapsed_bps,
            adverse_move_bps,
            urgency_bps,
            target_release_qty: OrderQty(target_release_qty),
        })
    }

    /// Plans one implementation-shortfall child slice.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress/context state is invalid or
    /// the generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_shortfall_slice(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        context: ImplementationShortfallContext,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<Option<ChildOrderPlan>, AlgoError> {
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if now_ns < parent.start_ns() || progress.is_complete() {
            return Ok(None);
        }

        let estimate = self.estimate(parent, progress, now_ns, context)?;
        let due_qty = estimate
            .target_release_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        if due_qty <= 0 {
            return Ok(None);
        }
        let leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        let mut child_qty = due_qty.min(parent.max_clip().0).min(leaves);
        let final_slice = progress.released_qty().0.saturating_add(child_qty)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if child_qty < parent.min_clip().0 && !final_slice {
            return Ok(None);
        }
        if child_qty <= 0 {
            return Ok(None);
        }
        child_qty = child_qty.min(
            parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0),
        );
        let request = parent.build_order_request(client_order_id, OrderQty(child_qty), ts_recv_ns);
        Ok(Some(ChildOrderPlan::new(
            child_id,
            parent.id(),
            request,
            now_ns,
        )?))
    }

    fn urgency_bps(&self, context: ImplementationShortfallContext, adverse_move_bps: u16) -> u16 {
        let mut urgency = i64::from(self.config.base_urgency_bps());
        urgency = urgency.saturating_add(i64::from(scale_bps_u32(
            u32::from(context.volatility_bps()),
            u32::from(self.config.volatility_weight_bps()),
        )));
        urgency = urgency.saturating_add(i64::from(scale_bps_u32(
            u32::from(context.spread_bps()),
            u32::from(self.config.spread_weight_bps()),
        )));
        urgency = urgency.saturating_add(i64::from(scale_bps_u32(
            u32::from(adverse_move_bps),
            u32::from(self.config.adverse_move_weight_bps()),
        )));
        urgency = urgency.saturating_sub(i64::from(scale_bps_u32(
            u32::from(context.temporary_impact_bps()),
            u32::from(self.config.impact_weight_bps()),
        )));
        let bounded = urgency.clamp(0, i64::from(self.config.max_urgency_bps()));
        u16::try_from(bounded).unwrap_or(self.config.max_urgency_bps())
    }
}
