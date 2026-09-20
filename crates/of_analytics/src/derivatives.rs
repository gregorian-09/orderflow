use super::*;

/// Option contract kind.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OptionKind {
    /// Call option.
    Call = 1,
    /// Put option.
    Put = 2,
}

/// Option flow sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionFlowSample {
    kind: OptionKind,
    volume: i64,
    open_interest: i64,
    premium: i128,
    implied_vol_bps: u32,
    gamma_exposure: i128,
}

impl OptionFlowSample {
    /// Creates an option flow sample.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDerivative`] when quantity, premium,
    /// or open interest is invalid.
    pub const fn new(
        kind: OptionKind,
        volume: i64,
        open_interest: i64,
        premium: i128,
        implied_vol_bps: u32,
        gamma_exposure: i128,
    ) -> Result<Self, AnalyticsError> {
        if volume < 0 || open_interest < 0 || premium < 0 {
            return Err(AnalyticsError::InvalidDerivative);
        }
        Ok(Self {
            kind,
            volume,
            open_interest,
            premium,
            implied_vol_bps,
            gamma_exposure,
        })
    }

    /// Returns option kind.
    pub const fn kind(&self) -> OptionKind {
        self.kind
    }

    /// Returns contract volume.
    pub const fn volume(&self) -> i64 {
        self.volume
    }

    /// Returns open interest.
    pub const fn open_interest(&self) -> i64 {
        self.open_interest
    }

    /// Returns premium/notional contribution.
    pub const fn premium(&self) -> i128 {
        self.premium
    }

    /// Returns implied volatility in basis points.
    pub const fn implied_vol_bps(&self) -> u32 {
        self.implied_vol_bps
    }

    /// Returns caller-supplied gamma exposure contribution.
    pub const fn gamma_exposure(&self) -> i128 {
        self.gamma_exposure
    }
}

/// Option flow snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionFlowSnapshot {
    call_volume: i64,
    put_volume: i64,
    call_open_interest: i64,
    put_open_interest: i64,
    call_premium: i128,
    put_premium: i128,
    put_call_volume_ratio_bps: u32,
    put_call_open_interest_ratio_bps: u32,
    put_call_premium_ratio_bps: u32,
    volume_open_interest_anomaly_bps: u32,
    implied_vol_flow_bps: u32,
    net_gamma_exposure: i128,
    put_call_pressure_bps: i32,
}

impl OptionFlowSnapshot {
    /// Returns call volume.
    pub const fn call_volume(&self) -> i64 {
        self.call_volume
    }

    /// Returns put volume.
    pub const fn put_volume(&self) -> i64 {
        self.put_volume
    }

    /// Returns call open interest.
    pub const fn call_open_interest(&self) -> i64 {
        self.call_open_interest
    }

    /// Returns put open interest.
    pub const fn put_open_interest(&self) -> i64 {
        self.put_open_interest
    }

    /// Returns call premium.
    pub const fn call_premium(&self) -> i128 {
        self.call_premium
    }

    /// Returns put premium.
    pub const fn put_premium(&self) -> i128 {
        self.put_premium
    }

    /// Returns put/call volume ratio scaled by 10,000.
    pub const fn put_call_volume_ratio_bps(&self) -> u32 {
        self.put_call_volume_ratio_bps
    }

    /// Returns put/call open-interest ratio scaled by 10,000.
    pub const fn put_call_open_interest_ratio_bps(&self) -> u32 {
        self.put_call_open_interest_ratio_bps
    }

    /// Returns put/call premium ratio scaled by 10,000.
    pub const fn put_call_premium_ratio_bps(&self) -> u32 {
        self.put_call_premium_ratio_bps
    }

    /// Returns aggregate volume/open-interest anomaly ratio.
    pub const fn volume_open_interest_anomaly_bps(&self) -> u32 {
        self.volume_open_interest_anomaly_bps
    }

    /// Returns premium-weighted implied-volatility flow in basis points.
    pub const fn implied_vol_flow_bps(&self) -> u32 {
        self.implied_vol_flow_bps
    }

    /// Returns net gamma exposure.
    pub const fn net_gamma_exposure(&self) -> i128 {
        self.net_gamma_exposure
    }

    /// Returns put-minus-call directional pressure in basis points.
    pub const fn put_call_pressure_bps(&self) -> i32 {
        self.put_call_pressure_bps
    }
}

/// Cumulative option flow tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionFlowTracker {
    call_volume: i64,
    put_volume: i64,
    call_open_interest: i64,
    put_open_interest: i64,
    call_premium: i128,
    put_premium: i128,
    implied_vol_premium_sum: u128,
    premium_sum: u128,
    net_gamma_exposure: i128,
}

impl OptionFlowTracker {
    /// Creates an empty option flow tracker.
    pub const fn new() -> Self {
        Self {
            call_volume: 0,
            put_volume: 0,
            call_open_interest: 0,
            put_open_interest: 0,
            call_premium: 0,
            put_premium: 0,
            implied_vol_premium_sum: 0,
            premium_sum: 0,
            net_gamma_exposure: 0,
        }
    }

    /// Records one option flow sample.
    pub fn on_sample(&mut self, sample: OptionFlowSample) {
        match sample.kind() {
            OptionKind::Call => {
                self.call_volume = self.call_volume.saturating_add(sample.volume());
                self.call_open_interest = self
                    .call_open_interest
                    .saturating_add(sample.open_interest());
                self.call_premium = self.call_premium.saturating_add(sample.premium());
            }
            OptionKind::Put => {
                self.put_volume = self.put_volume.saturating_add(sample.volume());
                self.put_open_interest = self
                    .put_open_interest
                    .saturating_add(sample.open_interest());
                self.put_premium = self.put_premium.saturating_add(sample.premium());
            }
        }
        self.implied_vol_premium_sum = self.implied_vol_premium_sum.saturating_add(
            u128::from(sample.implied_vol_bps()).saturating_mul(sample.premium() as u128),
        );
        self.premium_sum = self.premium_sum.saturating_add(sample.premium() as u128);
        self.net_gamma_exposure = self
            .net_gamma_exposure
            .saturating_add(sample.gamma_exposure());
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> OptionFlowSnapshot {
        let total_volume = self.call_volume.saturating_add(self.put_volume);
        let total_oi = self
            .call_open_interest
            .saturating_add(self.put_open_interest);
        OptionFlowSnapshot {
            call_volume: self.call_volume,
            put_volume: self.put_volume,
            call_open_interest: self.call_open_interest,
            put_open_interest: self.put_open_interest,
            call_premium: self.call_premium,
            put_premium: self.put_premium,
            put_call_volume_ratio_bps: ratio_i64_to_u32(self.put_volume, self.call_volume),
            put_call_open_interest_ratio_bps: ratio_i64_to_u32(
                self.put_open_interest,
                self.call_open_interest,
            ),
            put_call_premium_ratio_bps: ratio_i128_to_u32(self.put_premium, self.call_premium),
            volume_open_interest_anomaly_bps: ratio_i64_to_u32(total_volume, total_oi),
            implied_vol_flow_bps: self
                .implied_vol_premium_sum
                .checked_div(self.premium_sum)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(0),
            net_gamma_exposure: self.net_gamma_exposure,
            put_call_pressure_bps: signed_pressure_bps(self.put_volume, self.call_volume),
        }
    }

    /// Clears accumulated option flow state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for OptionFlowTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Futures basis input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuturesBasisInput {
    spot_price: i64,
    futures_price: i64,
    fair_value_price: i64,
    near_contract_price: i64,
    far_contract_price: i64,
    funding_rate_bps: i32,
}

impl FuturesBasisInput {
    /// Creates futures basis input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDerivative`] when prices are
    /// non-positive.
    pub const fn new(
        spot_price: i64,
        futures_price: i64,
        fair_value_price: i64,
        near_contract_price: i64,
        far_contract_price: i64,
        funding_rate_bps: i32,
    ) -> Result<Self, AnalyticsError> {
        if spot_price <= 0
            || futures_price <= 0
            || fair_value_price <= 0
            || near_contract_price <= 0
            || far_contract_price <= 0
        {
            return Err(AnalyticsError::InvalidDerivative);
        }
        Ok(Self {
            spot_price,
            futures_price,
            fair_value_price,
            near_contract_price,
            far_contract_price,
            funding_rate_bps,
        })
    }

    /// Returns spot price.
    pub const fn spot_price(&self) -> i64 {
        self.spot_price
    }

    /// Returns futures price.
    pub const fn futures_price(&self) -> i64 {
        self.futures_price
    }

    /// Returns fair-value futures price.
    pub const fn fair_value_price(&self) -> i64 {
        self.fair_value_price
    }

    /// Returns near contract price.
    pub const fn near_contract_price(&self) -> i64 {
        self.near_contract_price
    }

    /// Returns far contract price.
    pub const fn far_contract_price(&self) -> i64 {
        self.far_contract_price
    }

    /// Returns funding rate in basis points.
    pub const fn funding_rate_bps(&self) -> i32 {
        self.funding_rate_bps
    }
}

/// Futures basis snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuturesBasisSnapshot {
    basis_bps: i32,
    fair_value_gap_bps: i32,
    calendar_spread_bps: i32,
    roll_pressure_bps: i32,
    funding_basis_divergence_bps: i32,
}

impl FuturesBasisSnapshot {
    /// Returns futures-minus-spot basis in basis points.
    pub const fn basis_bps(&self) -> i32 {
        self.basis_bps
    }

    /// Returns futures-minus-fair-value gap in basis points.
    pub const fn fair_value_gap_bps(&self) -> i32 {
        self.fair_value_gap_bps
    }

    /// Returns far-minus-near calendar spread in basis points.
    pub const fn calendar_spread_bps(&self) -> i32 {
        self.calendar_spread_bps
    }

    /// Returns calendar spread minus basis as roll-pressure proxy.
    pub const fn roll_pressure_bps(&self) -> i32 {
        self.roll_pressure_bps
    }

    /// Returns basis minus funding-rate divergence in basis points.
    pub const fn funding_basis_divergence_bps(&self) -> i32 {
        self.funding_basis_divergence_bps
    }
}

/// Futures basis analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuturesBasisAnalyzer;

impl FuturesBasisAnalyzer {
    /// Analyzes futures basis input.
    pub fn analyze(input: FuturesBasisInput) -> FuturesBasisSnapshot {
        let basis_bps = price_to_bps(
            input.futures_price().saturating_sub(input.spot_price()),
            input.spot_price(),
        );
        let fair_value_gap_bps = price_to_bps(
            input
                .futures_price()
                .saturating_sub(input.fair_value_price()),
            input.fair_value_price(),
        );
        let calendar_spread_bps = price_to_bps(
            input
                .far_contract_price()
                .saturating_sub(input.near_contract_price()),
            input.near_contract_price(),
        );
        FuturesBasisSnapshot {
            basis_bps,
            fair_value_gap_bps,
            calendar_spread_bps,
            roll_pressure_bps: calendar_spread_bps.saturating_sub(basis_bps),
            funding_basis_divergence_bps: basis_bps.saturating_sub(input.funding_rate_bps()),
        }
    }
}

/// Derivatives diagnostic thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivativesDiagnosticConfig {
    skew_threshold_bps: u32,
    term_structure_threshold_bps: u32,
    iv_richness_threshold_bps: u32,
    gamma_pressure_threshold_bps: u16,
    basis_stress_threshold_bps: u32,
    funding_stress_threshold_bps: u32,
    stress_threshold_bps: u16,
}

impl DerivativesDiagnosticConfig {
    /// Creates derivatives diagnostic thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDerivative`] when basis-point
    /// thresholds exceed 10,000 where the field is capped as a score.
    pub const fn new(
        skew_threshold_bps: u32,
        term_structure_threshold_bps: u32,
        iv_richness_threshold_bps: u32,
        gamma_pressure_threshold_bps: u16,
        basis_stress_threshold_bps: u32,
        funding_stress_threshold_bps: u32,
        stress_threshold_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if gamma_pressure_threshold_bps > 10_000 || stress_threshold_bps > 10_000 {
            return Err(AnalyticsError::InvalidDerivative);
        }
        Ok(Self {
            skew_threshold_bps,
            term_structure_threshold_bps,
            iv_richness_threshold_bps,
            gamma_pressure_threshold_bps,
            basis_stress_threshold_bps,
            funding_stress_threshold_bps,
            stress_threshold_bps,
        })
    }

    /// Returns volatility-skew stress threshold in basis points.
    pub const fn skew_threshold_bps(&self) -> u32 {
        self.skew_threshold_bps
    }

    /// Returns term-structure stress threshold in basis points.
    pub const fn term_structure_threshold_bps(&self) -> u32 {
        self.term_structure_threshold_bps
    }

    /// Returns implied-versus-realized richness threshold in basis points.
    pub const fn iv_richness_threshold_bps(&self) -> u32 {
        self.iv_richness_threshold_bps
    }

    /// Returns gamma-pressure threshold in basis points.
    pub const fn gamma_pressure_threshold_bps(&self) -> u16 {
        self.gamma_pressure_threshold_bps
    }

    /// Returns basis-stress threshold in basis points.
    pub const fn basis_stress_threshold_bps(&self) -> u32 {
        self.basis_stress_threshold_bps
    }

    /// Returns funding/basis stress threshold in basis points.
    pub const fn funding_stress_threshold_bps(&self) -> u32 {
        self.funding_stress_threshold_bps
    }

    /// Returns aggregate derivatives stress threshold in basis points.
    pub const fn stress_threshold_bps(&self) -> u16 {
        self.stress_threshold_bps
    }
}

impl Default for DerivativesDiagnosticConfig {
    fn default() -> Self {
        Self {
            skew_threshold_bps: 500,
            term_structure_threshold_bps: 500,
            iv_richness_threshold_bps: 1_000,
            gamma_pressure_threshold_bps: 1_000,
            basis_stress_threshold_bps: 100,
            funding_stress_threshold_bps: 100,
            stress_threshold_bps: 2_500,
        }
    }
}

/// Caller-supplied derivatives volatility surface summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivativesVolatilitySurface {
    atm_iv_bps: u32,
    put_wing_iv_bps: u32,
    call_wing_iv_bps: u32,
    front_expiry_iv_bps: u32,
    back_expiry_iv_bps: u32,
    realized_vol_bps: u32,
}

impl DerivativesVolatilitySurface {
    /// Creates a derivatives volatility surface summary.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidDerivative`] when required implied
    /// volatility inputs are zero.
    pub const fn new(
        atm_iv_bps: u32,
        put_wing_iv_bps: u32,
        call_wing_iv_bps: u32,
        front_expiry_iv_bps: u32,
        back_expiry_iv_bps: u32,
        realized_vol_bps: u32,
    ) -> Result<Self, AnalyticsError> {
        if atm_iv_bps == 0
            || put_wing_iv_bps == 0
            || call_wing_iv_bps == 0
            || front_expiry_iv_bps == 0
            || back_expiry_iv_bps == 0
        {
            return Err(AnalyticsError::InvalidDerivative);
        }
        Ok(Self {
            atm_iv_bps,
            put_wing_iv_bps,
            call_wing_iv_bps,
            front_expiry_iv_bps,
            back_expiry_iv_bps,
            realized_vol_bps,
        })
    }

    /// Returns at-the-money implied volatility in basis points.
    pub const fn atm_iv_bps(&self) -> u32 {
        self.atm_iv_bps
    }

    /// Returns put-wing implied volatility in basis points.
    pub const fn put_wing_iv_bps(&self) -> u32 {
        self.put_wing_iv_bps
    }

    /// Returns call-wing implied volatility in basis points.
    pub const fn call_wing_iv_bps(&self) -> u32 {
        self.call_wing_iv_bps
    }

    /// Returns front-expiry implied volatility in basis points.
    pub const fn front_expiry_iv_bps(&self) -> u32 {
        self.front_expiry_iv_bps
    }

    /// Returns back-expiry implied volatility in basis points.
    pub const fn back_expiry_iv_bps(&self) -> u32 {
        self.back_expiry_iv_bps
    }

    /// Returns realized volatility in basis points.
    pub const fn realized_vol_bps(&self) -> u32 {
        self.realized_vol_bps
    }
}

/// Derivatives diagnostic input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivativesDiagnosticInput {
    option_snapshot: OptionFlowSnapshot,
    futures_snapshot: FuturesBasisSnapshot,
    volatility_surface: DerivativesVolatilitySurface,
}

impl DerivativesDiagnosticInput {
    /// Creates derivatives diagnostic input.
    pub const fn new(
        option_snapshot: OptionFlowSnapshot,
        futures_snapshot: FuturesBasisSnapshot,
        volatility_surface: DerivativesVolatilitySurface,
    ) -> Self {
        Self {
            option_snapshot,
            futures_snapshot,
            volatility_surface,
        }
    }

    /// Returns option-flow snapshot.
    pub const fn option_snapshot(&self) -> OptionFlowSnapshot {
        self.option_snapshot
    }

    /// Returns futures-basis snapshot.
    pub const fn futures_snapshot(&self) -> FuturesBasisSnapshot {
        self.futures_snapshot
    }

    /// Returns volatility surface summary.
    pub const fn volatility_surface(&self) -> DerivativesVolatilitySurface {
        self.volatility_surface
    }
}

/// Derivatives diagnostic snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivativesDiagnosticSnapshot {
    skew_bps: i32,
    term_structure_bps: i32,
    iv_richness_bps: i32,
    gamma_pressure_bps: i32,
    options_risk_score_bps: u16,
    futures_stress_score_bps: u16,
    derivatives_stress_score_bps: u16,
    skew_stress: bool,
    term_structure_stress: bool,
    iv_richness_stress: bool,
    gamma_pressure_stress: bool,
    basis_stress: bool,
    funding_basis_stress: bool,
    derivatives_stressed: bool,
}

impl DerivativesDiagnosticSnapshot {
    /// Returns put-wing minus call-wing implied-volatility skew.
    pub const fn skew_bps(&self) -> i32 {
        self.skew_bps
    }

    /// Returns back-expiry minus front-expiry implied-volatility term structure.
    pub const fn term_structure_bps(&self) -> i32 {
        self.term_structure_bps
    }

    /// Returns at-the-money implied volatility minus realized volatility.
    pub const fn iv_richness_bps(&self) -> i32 {
        self.iv_richness_bps
    }

    /// Returns net gamma exposure normalized by option premium.
    pub const fn gamma_pressure_bps(&self) -> i32 {
        self.gamma_pressure_bps
    }

    /// Returns aggregate option-flow and volatility-surface risk score.
    pub const fn options_risk_score_bps(&self) -> u16 {
        self.options_risk_score_bps
    }

    /// Returns aggregate futures basis/funding stress score.
    pub const fn futures_stress_score_bps(&self) -> u16 {
        self.futures_stress_score_bps
    }

    /// Returns aggregate derivatives stress score.
    pub const fn derivatives_stress_score_bps(&self) -> u16 {
        self.derivatives_stress_score_bps
    }

    /// Returns true when volatility skew exceeds threshold.
    pub const fn skew_stress(&self) -> bool {
        self.skew_stress
    }

    /// Returns true when volatility term structure exceeds threshold.
    pub const fn term_structure_stress(&self) -> bool {
        self.term_structure_stress
    }

    /// Returns true when implied volatility is rich versus realized volatility.
    pub const fn iv_richness_stress(&self) -> bool {
        self.iv_richness_stress
    }

    /// Returns true when normalized gamma pressure exceeds threshold.
    pub const fn gamma_pressure_stress(&self) -> bool {
        self.gamma_pressure_stress
    }

    /// Returns true when basis/fair-value/roll stress exceeds threshold.
    pub const fn basis_stress(&self) -> bool {
        self.basis_stress
    }

    /// Returns true when funding/basis divergence exceeds threshold.
    pub const fn funding_basis_stress(&self) -> bool {
        self.funding_basis_stress
    }

    /// Returns true when aggregate derivatives stress exceeds threshold.
    pub const fn derivatives_stressed(&self) -> bool {
        self.derivatives_stressed
    }
}

/// Deterministic derivatives diagnostic analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivativesDiagnosticAnalyzer {
    config: DerivativesDiagnosticConfig,
}

impl DerivativesDiagnosticAnalyzer {
    /// Creates a derivatives diagnostic analyzer.
    pub const fn new(config: DerivativesDiagnosticConfig) -> Self {
        Self { config }
    }

    /// Returns analyzer configuration.
    pub const fn config(&self) -> DerivativesDiagnosticConfig {
        self.config
    }

    /// Evaluates option-flow, volatility-surface, and futures-basis stress.
    pub fn evaluate(&self, input: DerivativesDiagnosticInput) -> DerivativesDiagnosticSnapshot {
        let options = input.option_snapshot();
        let futures = input.futures_snapshot();
        let surface = input.volatility_surface();
        let skew = signed_u32_diff_to_i32(surface.put_wing_iv_bps(), surface.call_wing_iv_bps());
        let term_structure =
            signed_u32_diff_to_i32(surface.back_expiry_iv_bps(), surface.front_expiry_iv_bps());
        let iv_richness = signed_u32_diff_to_i32(surface.atm_iv_bps(), surface.realized_vol_bps());
        let gamma_pressure = signed_i128_ratio_to_i32(
            options.net_gamma_exposure(),
            options.call_premium().saturating_add(options.put_premium()),
        );
        let options_risk = average_score(&[
            bounded_abs_bps(options.put_call_pressure_bps()),
            bounded_u32_bps(options.volume_open_interest_anomaly_bps()),
            bounded_abs_bps(skew),
            bounded_abs_bps(iv_richness),
            bounded_abs_bps(gamma_pressure),
        ]);
        let futures_stress = average_score(&[
            bounded_abs_bps(futures.basis_bps()),
            bounded_abs_bps(futures.fair_value_gap_bps()),
            bounded_abs_bps(futures.roll_pressure_bps()),
            bounded_abs_bps(futures.funding_basis_divergence_bps()),
        ]);
        let derivatives_stress = average_score(&[options_risk, futures_stress]);
        let skew_stress = skew.unsigned_abs() >= self.config.skew_threshold_bps();
        let term_structure_stress =
            term_structure.unsigned_abs() >= self.config.term_structure_threshold_bps();
        let iv_richness_stress =
            iv_richness.unsigned_abs() >= self.config.iv_richness_threshold_bps();
        let gamma_pressure_stress =
            gamma_pressure.unsigned_abs() >= u32::from(self.config.gamma_pressure_threshold_bps());
        let basis_stress = futures.basis_bps().unsigned_abs()
            >= self.config.basis_stress_threshold_bps()
            || futures.fair_value_gap_bps().unsigned_abs()
                >= self.config.basis_stress_threshold_bps()
            || futures.roll_pressure_bps().unsigned_abs()
                >= self.config.basis_stress_threshold_bps();
        let funding_basis_stress = futures.funding_basis_divergence_bps().unsigned_abs()
            >= self.config.funding_stress_threshold_bps();
        DerivativesDiagnosticSnapshot {
            skew_bps: skew,
            term_structure_bps: term_structure,
            iv_richness_bps: iv_richness,
            gamma_pressure_bps: gamma_pressure,
            options_risk_score_bps: options_risk,
            futures_stress_score_bps: futures_stress,
            derivatives_stress_score_bps: derivatives_stress,
            skew_stress,
            term_structure_stress,
            iv_richness_stress,
            gamma_pressure_stress,
            basis_stress,
            funding_basis_stress,
            derivatives_stressed: derivatives_stress >= self.config.stress_threshold_bps()
                || skew_stress
                || term_structure_stress
                || iv_richness_stress
                || gamma_pressure_stress
                || basis_stress
                || funding_basis_stress,
        }
    }
}

impl Default for DerivativesDiagnosticAnalyzer {
    fn default() -> Self {
        Self::new(DerivativesDiagnosticConfig::default())
    }
}
