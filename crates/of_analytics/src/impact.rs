use super::*;

/// Market-impact sample over a measurement interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactSample {
    start_midpoint: i64,
    end_midpoint: i64,
    signed_qty: i64,
    notional: i128,
}

impl ImpactSample {
    /// Creates an impact sample.
    ///
    /// Positive signed quantity represents buyer-initiated flow; negative
    /// signed quantity represents seller-initiated flow.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when prices, quantity, or
    /// notional are invalid.
    pub const fn new(
        start_midpoint: i64,
        end_midpoint: i64,
        signed_qty: i64,
        notional: i128,
    ) -> Result<Self, AnalyticsError> {
        if start_midpoint <= 0 || end_midpoint <= 0 || signed_qty == 0 || notional <= 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            start_midpoint,
            end_midpoint,
            signed_qty,
            notional,
        })
    }

    /// Returns starting midpoint.
    pub const fn start_midpoint(&self) -> i64 {
        self.start_midpoint
    }

    /// Returns ending midpoint.
    pub const fn end_midpoint(&self) -> i64 {
        self.end_midpoint
    }

    /// Returns signed quantity.
    pub const fn signed_qty(&self) -> i64 {
        self.signed_qty
    }

    /// Returns traded notional over the interval.
    pub const fn notional(&self) -> i128 {
        self.notional
    }

    /// Returns signed price change aligned to flow direction.
    pub const fn signed_price_change(&self) -> i64 {
        if self.signed_qty > 0 {
            self.end_midpoint - self.start_midpoint
        } else {
            self.start_midpoint - self.end_midpoint
        }
    }
}

/// Cumulative market-impact snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactSnapshot {
    samples: u64,
    signed_volume: i64,
    absolute_volume: i64,
    signed_price_change: i64,
    kyle_lambda_ppm: i64,
    amihud_illiquidity_ppm: i64,
}

impl ImpactSnapshot {
    /// Returns sample count.
    pub const fn samples(&self) -> u64 {
        self.samples
    }

    /// Returns cumulative signed volume.
    pub const fn signed_volume(&self) -> i64 {
        self.signed_volume
    }

    /// Returns cumulative absolute volume.
    pub const fn absolute_volume(&self) -> i64 {
        self.absolute_volume
    }

    /// Returns cumulative signed price change.
    pub const fn signed_price_change(&self) -> i64 {
        self.signed_price_change
    }

    /// Returns Kyle-style price impact per unit signed volume, scaled by
    /// 1,000,000.
    pub const fn kyle_lambda_ppm(&self) -> i64 {
        self.kyle_lambda_ppm
    }

    /// Returns Amihud-style absolute return per notional, scaled by 1,000,000.
    pub const fn amihud_illiquidity_ppm(&self) -> i64 {
        self.amihud_illiquidity_ppm
    }
}

/// Allocation-free cumulative impact tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactTracker {
    samples: u64,
    signed_volume: i64,
    absolute_volume: i64,
    signed_price_change: i64,
    absolute_return_ppm_sum: i128,
    notional_sum: i128,
}

impl ImpactTracker {
    /// Creates an empty impact tracker.
    pub const fn new() -> Self {
        Self {
            samples: 0,
            signed_volume: 0,
            absolute_volume: 0,
            signed_price_change: 0,
            absolute_return_ppm_sum: 0,
            notional_sum: 0,
        }
    }

    /// Records one impact sample.
    pub fn on_sample(&mut self, sample: ImpactSample) {
        self.samples = self.samples.saturating_add(1);
        self.signed_volume = self.signed_volume.saturating_add(sample.signed_qty());
        self.absolute_volume = self
            .absolute_volume
            .saturating_add(sample.signed_qty().abs());
        self.signed_price_change = self
            .signed_price_change
            .saturating_add(sample.signed_price_change());
        self.absolute_return_ppm_sum = self.absolute_return_ppm_sum.saturating_add(
            i128::from(sample.end_midpoint().abs_diff(sample.start_midpoint()) as i64)
                .saturating_mul(1_000_000)
                / i128::from(sample.start_midpoint()),
        );
        self.notional_sum = self.notional_sum.saturating_add(sample.notional());
    }

    /// Returns current impact snapshot.
    pub fn snapshot(&self) -> ImpactSnapshot {
        ImpactSnapshot {
            samples: self.samples,
            signed_volume: self.signed_volume,
            absolute_volume: self.absolute_volume,
            signed_price_change: self.signed_price_change,
            kyle_lambda_ppm: if self.signed_volume == 0 {
                0
            } else {
                i64::try_from(
                    (i128::from(self.signed_price_change) * 1_000_000)
                        / i128::from(self.signed_volume),
                )
                .unwrap_or(0)
            },
            amihud_illiquidity_ppm: if self.notional_sum <= 0 {
                0
            } else {
                i64::try_from((self.absolute_return_ppm_sum * 1_000_000) / self.notional_sum)
                    .unwrap_or(0)
            },
        }
    }
}

impl Default for ImpactTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Calibrated market-impact parameters for pre-trade estimates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactCalibration {
    daily_volume: i64,
    volatility_bps: u16,
    square_root_coefficient_bps: u16,
    temporary_impact_coefficient_bps: u16,
    permanent_impact_coefficient_bps: u16,
    decay_half_life_ns: u64,
}

impl ImpactCalibration {
    /// Creates calibrated impact parameters.
    ///
    /// Coefficients are basis-point scaled. A `square_root_coefficient_bps` of
    /// `10_000` means one volatility unit times the square-root participation
    /// estimate.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when daily volume is
    /// non-positive.
    pub const fn new(
        daily_volume: i64,
        volatility_bps: u16,
        square_root_coefficient_bps: u16,
        temporary_impact_coefficient_bps: u16,
        permanent_impact_coefficient_bps: u16,
        decay_half_life_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        if daily_volume <= 0 {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            daily_volume,
            volatility_bps,
            square_root_coefficient_bps,
            temporary_impact_coefficient_bps,
            permanent_impact_coefficient_bps,
            decay_half_life_ns,
        })
    }

    /// Returns expected daily volume.
    pub const fn daily_volume(&self) -> i64 {
        self.daily_volume
    }

    /// Returns volatility in basis points.
    pub const fn volatility_bps(&self) -> u16 {
        self.volatility_bps
    }

    /// Returns square-root impact coefficient in basis points.
    pub const fn square_root_coefficient_bps(&self) -> u16 {
        self.square_root_coefficient_bps
    }

    /// Returns temporary impact coefficient in basis points.
    pub const fn temporary_impact_coefficient_bps(&self) -> u16 {
        self.temporary_impact_coefficient_bps
    }

    /// Returns permanent impact coefficient in basis points.
    pub const fn permanent_impact_coefficient_bps(&self) -> u16 {
        self.permanent_impact_coefficient_bps
    }

    /// Returns impact decay half-life in nanoseconds.
    pub const fn decay_half_life_ns(&self) -> u64 {
        self.decay_half_life_ns
    }
}

/// Pre-trade impact estimate input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedImpactInput {
    side: Side,
    order_qty: i64,
    expected_interval_volume: i64,
    arrival_midpoint: i64,
    horizon_ns: u64,
    calibration: ImpactCalibration,
}

impl ExpectedImpactInput {
    /// Creates pre-trade impact estimate input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when quantities, price, or
    /// horizon are invalid.
    pub const fn new(
        side: Side,
        order_qty: i64,
        expected_interval_volume: i64,
        arrival_midpoint: i64,
        horizon_ns: u64,
        calibration: ImpactCalibration,
    ) -> Result<Self, AnalyticsError> {
        if order_qty <= 0
            || expected_interval_volume <= 0
            || arrival_midpoint <= 0
            || horizon_ns == 0
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            side,
            order_qty,
            expected_interval_volume,
            arrival_midpoint,
            horizon_ns,
            calibration,
        })
    }

    /// Returns execution side.
    pub const fn side(&self) -> Side {
        self.side
    }

    /// Returns proposed order quantity.
    pub const fn order_qty(&self) -> i64 {
        self.order_qty
    }

    /// Returns expected interval volume.
    pub const fn expected_interval_volume(&self) -> i64 {
        self.expected_interval_volume
    }

    /// Returns arrival midpoint.
    pub const fn arrival_midpoint(&self) -> i64 {
        self.arrival_midpoint
    }

    /// Returns execution horizon in nanoseconds.
    pub const fn horizon_ns(&self) -> u64 {
        self.horizon_ns
    }

    /// Returns impact calibration.
    pub const fn calibration(&self) -> ImpactCalibration {
        self.calibration
    }
}

/// Pre-trade market-impact estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedImpactSnapshot {
    participation_bps: u16,
    daily_participation_bps: u16,
    square_root_impact_bps: i32,
    temporary_impact_bps: i32,
    permanent_impact_bps: i32,
    instantaneous_impact_bps: i32,
    decay_remaining_bps: u16,
    expected_total_impact_bps: i32,
    expected_signed_price_move: i64,
}

impl ExpectedImpactSnapshot {
    /// Returns interval participation in basis points.
    pub const fn participation_bps(&self) -> u16 {
        self.participation_bps
    }

    /// Returns daily participation in basis points.
    pub const fn daily_participation_bps(&self) -> u16 {
        self.daily_participation_bps
    }

    /// Returns square-root impact estimate in basis points.
    pub const fn square_root_impact_bps(&self) -> i32 {
        self.square_root_impact_bps
    }

    /// Returns temporary impact estimate in basis points.
    pub const fn temporary_impact_bps(&self) -> i32 {
        self.temporary_impact_bps
    }

    /// Returns permanent impact estimate in basis points.
    pub const fn permanent_impact_bps(&self) -> i32 {
        self.permanent_impact_bps
    }

    /// Returns instantaneous impact estimate in basis points.
    pub const fn instantaneous_impact_bps(&self) -> i32 {
        self.instantaneous_impact_bps
    }

    /// Returns remaining temporary impact after the horizon in basis points.
    pub const fn decay_remaining_bps(&self) -> u16 {
        self.decay_remaining_bps
    }

    /// Returns expected total impact cost in basis points.
    pub const fn expected_total_impact_bps(&self) -> i32 {
        self.expected_total_impact_bps
    }

    /// Returns expected signed midpoint move in normalized price units.
    pub const fn expected_signed_price_move(&self) -> i64 {
        self.expected_signed_price_move
    }
}

/// Deterministic pre-trade market-impact estimator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedImpactEstimator;

impl ExpectedImpactEstimator {
    /// Estimates pre-trade market impact from explicit calibration.
    pub fn estimate(input: ExpectedImpactInput) -> ExpectedImpactSnapshot {
        let participation_bps = ratio_bps_i64(input.order_qty(), input.expected_interval_volume());
        let daily_participation_bps =
            ratio_bps_i64(input.order_qty(), input.calibration().daily_volume());
        let sqrt_participation_bps =
            integer_sqrt_u128(u128::from(daily_participation_bps) * 10_000);
        let square_root_impact_bps = i32::try_from(
            (u128::from(input.calibration().volatility_bps())
                * u128::from(input.calibration().square_root_coefficient_bps())
                * sqrt_participation_bps)
                / 100_000_000,
        )
        .unwrap_or(i32::MAX);
        let temporary_impact_bps = coefficient_impact_bps(
            participation_bps,
            input.calibration().temporary_impact_coefficient_bps(),
        );
        let permanent_impact_bps = coefficient_impact_bps(
            participation_bps,
            input.calibration().permanent_impact_coefficient_bps(),
        );
        let instantaneous_impact_bps = temporary_impact_bps.saturating_add(permanent_impact_bps);
        let decay_remaining_bps =
            decay_remaining_bps(input.horizon_ns(), input.calibration().decay_half_life_ns());
        let decayed_temporary_bps = i32::try_from(
            (i128::from(temporary_impact_bps) * i128::from(decay_remaining_bps)) / 10_000,
        )
        .unwrap_or(0);
        let expected_total_impact_bps = square_root_impact_bps
            .saturating_add(permanent_impact_bps)
            .saturating_add(decayed_temporary_bps);
        let price_move =
            bps_to_price_delta(input.arrival_midpoint(), expected_total_impact_bps.abs());
        let expected_signed_price_move = match input.side() {
            Side::Ask => price_move,
            Side::Bid => price_move.saturating_neg(),
        };
        ExpectedImpactSnapshot {
            participation_bps,
            daily_participation_bps,
            square_root_impact_bps,
            temporary_impact_bps,
            permanent_impact_bps,
            instantaneous_impact_bps,
            decay_remaining_bps,
            expected_total_impact_bps,
            expected_signed_price_move,
        }
    }
}

/// Child-order impact attribution context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildOrderImpactContext {
    side: Side,
    parent_qty: i64,
    child_qty: i64,
    arrival_midpoint: i64,
    child_fill_price: i64,
    post_child_midpoint: i64,
    final_midpoint: i64,
}

impl ChildOrderImpactContext {
    /// Creates child-order impact attribution context.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when quantities or prices are
    /// invalid.
    pub const fn new(
        side: Side,
        parent_qty: i64,
        child_qty: i64,
        arrival_midpoint: i64,
        child_fill_price: i64,
        post_child_midpoint: i64,
        final_midpoint: i64,
    ) -> Result<Self, AnalyticsError> {
        if parent_qty <= 0
            || child_qty <= 0
            || child_qty > parent_qty
            || arrival_midpoint <= 0
            || child_fill_price <= 0
            || post_child_midpoint <= 0
            || final_midpoint <= 0
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            side,
            parent_qty,
            child_qty,
            arrival_midpoint,
            child_fill_price,
            post_child_midpoint,
            final_midpoint,
        })
    }

    /// Returns execution side.
    pub const fn side(&self) -> Side {
        self.side
    }

    /// Returns parent order quantity.
    pub const fn parent_qty(&self) -> i64 {
        self.parent_qty
    }

    /// Returns child order quantity.
    pub const fn child_qty(&self) -> i64 {
        self.child_qty
    }

    /// Returns arrival midpoint.
    pub const fn arrival_midpoint(&self) -> i64 {
        self.arrival_midpoint
    }

    /// Returns child fill price.
    pub const fn child_fill_price(&self) -> i64 {
        self.child_fill_price
    }

    /// Returns midpoint immediately after the child order.
    pub const fn post_child_midpoint(&self) -> i64 {
        self.post_child_midpoint
    }

    /// Returns final midpoint after the attribution horizon.
    pub const fn final_midpoint(&self) -> i64 {
        self.final_midpoint
    }
}

/// Child-order impact attribution snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildOrderImpactSnapshot {
    child_participation_bps: u16,
    child_slippage_bps: i32,
    instantaneous_impact_bps: i32,
    permanent_impact_bps: i32,
    temporary_impact_bps: i32,
    impact_decay_bps: i32,
    attributed_impact_bps: i32,
}

impl ChildOrderImpactSnapshot {
    /// Returns child share of parent quantity in basis points.
    pub const fn child_participation_bps(&self) -> u16 {
        self.child_participation_bps
    }

    /// Returns child fill slippage versus arrival in basis points.
    pub const fn child_slippage_bps(&self) -> i32 {
        self.child_slippage_bps
    }

    /// Returns immediate post-child impact in basis points.
    pub const fn instantaneous_impact_bps(&self) -> i32 {
        self.instantaneous_impact_bps
    }

    /// Returns permanent impact at the attribution horizon in basis points.
    pub const fn permanent_impact_bps(&self) -> i32 {
        self.permanent_impact_bps
    }

    /// Returns temporary impact component in basis points.
    pub const fn temporary_impact_bps(&self) -> i32 {
        self.temporary_impact_bps
    }

    /// Returns impact decay from immediate to final mark in basis points.
    pub const fn impact_decay_bps(&self) -> i32 {
        self.impact_decay_bps
    }

    /// Returns parent-weighted child attribution in basis points.
    pub const fn attributed_impact_bps(&self) -> i32 {
        self.attributed_impact_bps
    }
}

/// Deterministic child-order impact attribution analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildOrderImpactAnalyzer;

impl ChildOrderImpactAnalyzer {
    /// Evaluates child-order impact attribution.
    pub fn evaluate(context: ChildOrderImpactContext) -> ChildOrderImpactSnapshot {
        let child_participation_bps = ratio_bps_i64(context.child_qty(), context.parent_qty());
        let child_slippage_bps = side_aware_price_move_bps(
            context.side(),
            context.arrival_midpoint(),
            context.child_fill_price(),
        );
        let instantaneous_impact_bps = side_aware_price_move_bps(
            context.side(),
            context.arrival_midpoint(),
            context.post_child_midpoint(),
        );
        let permanent_impact_bps = side_aware_price_move_bps(
            context.side(),
            context.arrival_midpoint(),
            context.final_midpoint(),
        );
        let temporary_impact_bps = child_slippage_bps.saturating_sub(permanent_impact_bps);
        let impact_decay_bps = instantaneous_impact_bps.saturating_sub(permanent_impact_bps);
        let attributed_impact_bps = i32::try_from(
            (i128::from(child_slippage_bps) * i128::from(child_participation_bps)) / 10_000,
        )
        .unwrap_or(0);
        ChildOrderImpactSnapshot {
            child_participation_bps,
            child_slippage_bps,
            instantaneous_impact_bps,
            permanent_impact_bps,
            temporary_impact_bps,
            impact_decay_bps,
            attributed_impact_bps,
        }
    }
}
