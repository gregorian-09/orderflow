use super::*;

/// Reference implementation: simple delta momentum threshold signal.
#[derive(Debug)]
pub struct DeltaMomentumSignal {
    latest: AnalyticsSnapshot,
    threshold: i64,
}

impl DeltaMomentumSignal {
    /// Creates a new signal with absolute delta threshold.
    pub fn new(threshold: i64) -> Self {
        Self {
            latest: AnalyticsSnapshot::default(),
            threshold,
        }
    }

    /// Returns static metadata for this signal type.
    pub const fn descriptor(&self) -> &'static SignalDescriptor {
        &DELTA_MOMENTUM_DESCRIPTOR
    }

    fn evaluate(&self) -> (SignalState, SignalReasonCode, &'static str) {
        if self.latest.delta >= self.threshold {
            (
                SignalState::LongBias,
                SignalReasonCode::DeltaMomentumPositive,
                "delta_above_threshold",
            )
        } else if self.latest.delta <= -self.threshold {
            (
                SignalState::ShortBias,
                SignalReasonCode::DeltaMomentumNegative,
                "delta_below_threshold",
            )
        } else {
            (
                SignalState::Neutral,
                SignalReasonCode::DeltaMomentumInsideBand,
                "delta_inside_band",
            )
        }
    }
}

impl Default for DeltaMomentumSignal {
    fn default() -> Self {
        Self::new(100)
    }
}

impl SignalModule for DeltaMomentumSignal {
    fn on_analytics(&mut self, ev: &AnalyticsSnapshot) {
        self.latest = ev.clone();
    }

    fn snapshot(&self) -> SignalSnapshot {
        let (state, _, reason) = self.evaluate();

        SignalSnapshot {
            module_id: "delta_momentum_v1",
            state,
            confidence_bps: 500,
            quality_flags: 0,
            reason: reason.to_string(),
        }
    }

    fn quality_gate(&self, q: DataQualityFlags) -> SignalGateDecision {
        default_quality_gate(q)
    }

    fn latest_explanation(&self) -> Option<SignalExplanation> {
        Some(<Self as ExplainableSignalModule>::explanation(self))
    }
}

impl ExplainableSignalModule for DeltaMomentumSignal {
    fn explanation(&self) -> SignalExplanation {
        let snapshot = self.snapshot();
        let (_, reason_code, _) = self.evaluate();
        SignalExplanation::from_snapshot(&snapshot, reason_code)
            .with_input(SignalInputValue::integer("delta", self.latest.delta))
            .with_threshold(SignalThreshold::integer("threshold", self.threshold))
            .with_confidence_component(SignalConfidenceComponent::new("base_confidence", 500))
    }
}

/// Volume imbalance signal based on buy/sell session totals.
#[derive(Debug)]
pub struct VolumeImbalanceSignal {
    latest: AnalyticsSnapshot,
    threshold: i64,
}

impl VolumeImbalanceSignal {
    /// Creates a new volume-imbalance signal with absolute imbalance threshold.
    pub fn new(threshold: i64) -> Self {
        Self {
            latest: AnalyticsSnapshot::default(),
            threshold,
        }
    }

    /// Returns static metadata for this signal type.
    pub const fn descriptor(&self) -> &'static SignalDescriptor {
        &VOLUME_IMBALANCE_DESCRIPTOR
    }

    fn imbalance(&self) -> i64 {
        self.latest.buy_volume - self.latest.sell_volume
    }

    fn evaluate(&self) -> (SignalState, SignalReasonCode, &'static str) {
        let imbalance = self.imbalance();
        if imbalance >= self.threshold {
            (
                SignalState::LongBias,
                SignalReasonCode::BuyVolumeImbalance,
                "buy_volume_above_threshold",
            )
        } else if imbalance <= -self.threshold {
            (
                SignalState::ShortBias,
                SignalReasonCode::SellVolumeImbalance,
                "sell_volume_above_threshold",
            )
        } else {
            (
                SignalState::Neutral,
                SignalReasonCode::VolumeInsideBand,
                "volume_inside_band",
            )
        }
    }
}

impl Default for VolumeImbalanceSignal {
    fn default() -> Self {
        Self::new(100)
    }
}

impl SignalModule for VolumeImbalanceSignal {
    fn on_analytics(&mut self, ev: &AnalyticsSnapshot) {
        self.latest = ev.clone();
    }

    fn snapshot(&self) -> SignalSnapshot {
        let (state, _, reason) = self.evaluate();

        SignalSnapshot {
            module_id: "volume_imbalance_v1",
            state,
            confidence_bps: 550,
            quality_flags: 0,
            reason: reason.to_string(),
        }
    }

    fn quality_gate(&self, q: DataQualityFlags) -> SignalGateDecision {
        default_quality_gate(q)
    }

    fn latest_explanation(&self) -> Option<SignalExplanation> {
        Some(<Self as ExplainableSignalModule>::explanation(self))
    }
}

impl ExplainableSignalModule for VolumeImbalanceSignal {
    fn explanation(&self) -> SignalExplanation {
        let snapshot = self.snapshot();
        let (_, reason_code, _) = self.evaluate();
        SignalExplanation::from_snapshot(&snapshot, reason_code)
            .with_input(SignalInputValue::integer(
                "buy_volume",
                self.latest.buy_volume,
            ))
            .with_input(SignalInputValue::integer(
                "sell_volume",
                self.latest.sell_volume,
            ))
            .with_input(SignalInputValue::integer("imbalance", self.imbalance()))
            .with_threshold(SignalThreshold::integer("threshold", self.threshold))
            .with_confidence_component(SignalConfidenceComponent::new("base_confidence", 550))
    }
}

/// Cumulative delta signal tuned for session-scale directional bias.
#[derive(Debug)]
pub struct CumulativeDeltaSignal {
    latest: AnalyticsSnapshot,
    threshold: i64,
}

impl CumulativeDeltaSignal {
    /// Creates a new cumulative-delta signal with absolute threshold.
    pub fn new(threshold: i64) -> Self {
        Self {
            latest: AnalyticsSnapshot::default(),
            threshold,
        }
    }

    /// Returns static metadata for this signal type.
    pub const fn descriptor(&self) -> &'static SignalDescriptor {
        &CUMULATIVE_DELTA_DESCRIPTOR
    }

    fn evaluate(&self) -> (SignalState, SignalReasonCode, &'static str) {
        if self.latest.cumulative_delta >= self.threshold {
            (
                SignalState::LongBias,
                SignalReasonCode::CumulativeDeltaPositive,
                "cumulative_delta_above_threshold",
            )
        } else if self.latest.cumulative_delta <= -self.threshold {
            (
                SignalState::ShortBias,
                SignalReasonCode::CumulativeDeltaNegative,
                "cumulative_delta_below_threshold",
            )
        } else {
            (
                SignalState::Neutral,
                SignalReasonCode::CumulativeDeltaInsideBand,
                "cumulative_delta_inside_band",
            )
        }
    }
}

impl Default for CumulativeDeltaSignal {
    fn default() -> Self {
        Self::new(250)
    }
}

impl SignalModule for CumulativeDeltaSignal {
    fn on_analytics(&mut self, ev: &AnalyticsSnapshot) {
        self.latest = ev.clone();
    }

    fn snapshot(&self) -> SignalSnapshot {
        let (state, _, reason) = self.evaluate();

        SignalSnapshot {
            module_id: "cumulative_delta_v1",
            state,
            confidence_bps: 600,
            quality_flags: 0,
            reason: reason.to_string(),
        }
    }

    fn quality_gate(&self, q: DataQualityFlags) -> SignalGateDecision {
        default_quality_gate(q)
    }

    fn latest_explanation(&self) -> Option<SignalExplanation> {
        Some(<Self as ExplainableSignalModule>::explanation(self))
    }
}

impl ExplainableSignalModule for CumulativeDeltaSignal {
    fn explanation(&self) -> SignalExplanation {
        let snapshot = self.snapshot();
        let (_, reason_code, _) = self.evaluate();
        SignalExplanation::from_snapshot(&snapshot, reason_code)
            .with_input(SignalInputValue::integer(
                "cumulative_delta",
                self.latest.cumulative_delta,
            ))
            .with_threshold(SignalThreshold::integer("threshold", self.threshold))
            .with_confidence_component(SignalConfidenceComponent::new("base_confidence", 600))
    }
}

/// Absorption signal that looks for strong directional flow failing to dislodge price from POC.
#[derive(Debug)]
pub struct AbsorptionSignal {
    latest: AnalyticsSnapshot,
    threshold: i64,
    price_band: i64,
}

impl AbsorptionSignal {
    /// Creates a new absorption signal using a delta threshold and price band around POC.
    pub fn new(threshold: i64, price_band: i64) -> Self {
        Self {
            latest: AnalyticsSnapshot::default(),
            threshold,
            price_band,
        }
    }

    /// Returns static metadata for this signal type.
    pub const fn descriptor(&self) -> &'static SignalDescriptor {
        &ABSORPTION_DESCRIPTOR
    }

    fn poc_distance(&self) -> i64 {
        (self.latest.last_price - self.latest.point_of_control).abs()
    }

    fn evaluate(&self) -> (SignalState, SignalReasonCode, &'static str) {
        let poc_distance = self.poc_distance();
        if poc_distance <= self.price_band && self.latest.delta <= -self.threshold {
            (
                SignalState::LongBias,
                SignalReasonCode::SellAbsorptionDetected,
                "sell_absorption_detected",
            )
        } else if poc_distance <= self.price_band && self.latest.delta >= self.threshold {
            (
                SignalState::ShortBias,
                SignalReasonCode::BuyAbsorptionDetected,
                "buy_absorption_detected",
            )
        } else {
            (
                SignalState::Neutral,
                SignalReasonCode::AbsorptionNotDetected,
                "absorption_not_detected",
            )
        }
    }
}

impl Default for AbsorptionSignal {
    fn default() -> Self {
        Self::new(150, 2)
    }
}

impl SignalModule for AbsorptionSignal {
    fn on_analytics(&mut self, ev: &AnalyticsSnapshot) {
        self.latest = ev.clone();
    }

    fn snapshot(&self) -> SignalSnapshot {
        let (state, _, reason) = self.evaluate();

        SignalSnapshot {
            module_id: "absorption_v1",
            state,
            confidence_bps: 575,
            quality_flags: 0,
            reason: reason.to_string(),
        }
    }

    fn quality_gate(&self, q: DataQualityFlags) -> SignalGateDecision {
        default_quality_gate(q)
    }

    fn latest_explanation(&self) -> Option<SignalExplanation> {
        Some(<Self as ExplainableSignalModule>::explanation(self))
    }
}

impl ExplainableSignalModule for AbsorptionSignal {
    fn explanation(&self) -> SignalExplanation {
        let snapshot = self.snapshot();
        let (_, reason_code, _) = self.evaluate();
        SignalExplanation::from_snapshot(&snapshot, reason_code)
            .with_input(SignalInputValue::integer("delta", self.latest.delta))
            .with_input(SignalInputValue::integer(
                "last_price",
                self.latest.last_price,
            ))
            .with_input(SignalInputValue::integer(
                "point_of_control",
                self.latest.point_of_control,
            ))
            .with_input(SignalInputValue::integer(
                "poc_distance",
                self.poc_distance(),
            ))
            .with_threshold(SignalThreshold::integer("threshold", self.threshold))
            .with_threshold(SignalThreshold::integer("price_band", self.price_band))
            .with_confidence_component(SignalConfidenceComponent::new("base_confidence", 575))
    }
}

/// Exhaustion signal that looks for strong directional flow stalling back near POC.
#[derive(Debug)]
pub struct ExhaustionSignal {
    latest: AnalyticsSnapshot,
    threshold: i64,
}

impl ExhaustionSignal {
    /// Creates a new exhaustion signal using a delta threshold.
    pub fn new(threshold: i64) -> Self {
        Self {
            latest: AnalyticsSnapshot::default(),
            threshold,
        }
    }

    /// Returns static metadata for this signal type.
    pub const fn descriptor(&self) -> &'static SignalDescriptor {
        &EXHAUSTION_DESCRIPTOR
    }

    fn evaluate(&self) -> (SignalState, SignalReasonCode, &'static str) {
        if self.latest.delta >= self.threshold
            && self.latest.last_price <= self.latest.point_of_control
        {
            (
                SignalState::ShortBias,
                SignalReasonCode::BuyExhaustionDetected,
                "buy_exhaustion_detected",
            )
        } else if self.latest.delta <= -self.threshold
            && self.latest.last_price >= self.latest.point_of_control
        {
            (
                SignalState::LongBias,
                SignalReasonCode::SellExhaustionDetected,
                "sell_exhaustion_detected",
            )
        } else {
            (
                SignalState::Neutral,
                SignalReasonCode::ExhaustionNotDetected,
                "exhaustion_not_detected",
            )
        }
    }
}

impl Default for ExhaustionSignal {
    fn default() -> Self {
        Self::new(150)
    }
}

impl SignalModule for ExhaustionSignal {
    fn on_analytics(&mut self, ev: &AnalyticsSnapshot) {
        self.latest = ev.clone();
    }

    fn snapshot(&self) -> SignalSnapshot {
        let (state, _, reason) = self.evaluate();

        SignalSnapshot {
            module_id: "exhaustion_v1",
            state,
            confidence_bps: 565,
            quality_flags: 0,
            reason: reason.to_string(),
        }
    }

    fn quality_gate(&self, q: DataQualityFlags) -> SignalGateDecision {
        default_quality_gate(q)
    }

    fn latest_explanation(&self) -> Option<SignalExplanation> {
        Some(<Self as ExplainableSignalModule>::explanation(self))
    }
}

impl ExplainableSignalModule for ExhaustionSignal {
    fn explanation(&self) -> SignalExplanation {
        let snapshot = self.snapshot();
        let (_, reason_code, _) = self.evaluate();
        SignalExplanation::from_snapshot(&snapshot, reason_code)
            .with_input(SignalInputValue::integer("delta", self.latest.delta))
            .with_input(SignalInputValue::integer(
                "last_price",
                self.latest.last_price,
            ))
            .with_input(SignalInputValue::integer(
                "point_of_control",
                self.latest.point_of_control,
            ))
            .with_threshold(SignalThreshold::integer("threshold", self.threshold))
            .with_confidence_component(SignalConfidenceComponent::new("base_confidence", 565))
    }
}

/// Sweep detection signal that looks for value-area breaks accompanied by directional flow.
#[derive(Debug)]
pub struct SweepDetectionSignal {
    latest: AnalyticsSnapshot,
    threshold: i64,
    breakout_ticks: i64,
}

impl SweepDetectionSignal {
    /// Creates a new sweep signal with delta threshold and breakout distance.
    pub fn new(threshold: i64, breakout_ticks: i64) -> Self {
        Self {
            latest: AnalyticsSnapshot::default(),
            threshold,
            breakout_ticks,
        }
    }

    /// Returns static metadata for this signal type.
    pub const fn descriptor(&self) -> &'static SignalDescriptor {
        &SWEEP_DETECTION_DESCRIPTOR
    }

    fn evaluate(&self) -> (SignalState, SignalReasonCode, &'static str) {
        if self.latest.delta >= self.threshold
            && self.latest.last_price >= self.latest.value_area_high + self.breakout_ticks
        {
            (
                SignalState::LongBias,
                SignalReasonCode::UpsideSweepDetected,
                "upside_sweep_detected",
            )
        } else if self.latest.delta <= -self.threshold
            && self.latest.last_price <= self.latest.value_area_low - self.breakout_ticks
        {
            (
                SignalState::ShortBias,
                SignalReasonCode::DownsideSweepDetected,
                "downside_sweep_detected",
            )
        } else {
            (
                SignalState::Neutral,
                SignalReasonCode::SweepNotDetected,
                "sweep_not_detected",
            )
        }
    }
}

impl Default for SweepDetectionSignal {
    fn default() -> Self {
        Self::new(150, 1)
    }
}

impl SignalModule for SweepDetectionSignal {
    fn on_analytics(&mut self, ev: &AnalyticsSnapshot) {
        self.latest = ev.clone();
    }

    fn snapshot(&self) -> SignalSnapshot {
        let (state, _, reason) = self.evaluate();

        SignalSnapshot {
            module_id: "sweep_detection_v1",
            state,
            confidence_bps: 625,
            quality_flags: 0,
            reason: reason.to_string(),
        }
    }

    fn quality_gate(&self, q: DataQualityFlags) -> SignalGateDecision {
        default_quality_gate(q)
    }

    fn latest_explanation(&self) -> Option<SignalExplanation> {
        Some(<Self as ExplainableSignalModule>::explanation(self))
    }
}

impl ExplainableSignalModule for SweepDetectionSignal {
    fn explanation(&self) -> SignalExplanation {
        let snapshot = self.snapshot();
        let (_, reason_code, _) = self.evaluate();
        SignalExplanation::from_snapshot(&snapshot, reason_code)
            .with_input(SignalInputValue::integer("delta", self.latest.delta))
            .with_input(SignalInputValue::integer(
                "last_price",
                self.latest.last_price,
            ))
            .with_input(SignalInputValue::integer(
                "value_area_high",
                self.latest.value_area_high,
            ))
            .with_input(SignalInputValue::integer(
                "value_area_low",
                self.latest.value_area_low,
            ))
            .with_threshold(SignalThreshold::integer("threshold", self.threshold))
            .with_threshold(SignalThreshold::integer(
                "breakout_ticks",
                self.breakout_ticks,
            ))
            .with_confidence_component(SignalConfidenceComponent::new("base_confidence", 625))
    }
}

/// Composite signal that aggregates child modules into one stable directional output.
pub struct CompositeSignal {
    modules: Vec<Box<dyn SignalModule>>,
}

impl CompositeSignal {
    /// Creates a composite signal from child modules.
    pub fn new(modules: Vec<Box<dyn SignalModule>>) -> Self {
        Self { modules }
    }

    /// Returns static metadata for this signal type.
    pub const fn descriptor(&self) -> &'static SignalDescriptor {
        &COMPOSITE_DESCRIPTOR
    }

    fn tally_votes(&self) -> (u16, u16, u32, Vec<&'static str>, Vec<&'static str>) {
        let mut long_votes = 0_u16;
        let mut short_votes = 0_u16;
        let mut confidence_sum = 0_u32;
        let mut long_modules = Vec::new();
        let mut short_modules = Vec::new();

        for module in &self.modules {
            let snapshot = module.snapshot();
            confidence_sum += u32::from(snapshot.confidence_bps);
            match snapshot.state {
                SignalState::LongBias => {
                    long_votes += 1;
                    long_modules.push(snapshot.module_id);
                }
                SignalState::ShortBias => {
                    short_votes += 1;
                    short_modules.push(snapshot.module_id);
                }
                SignalState::Neutral | SignalState::Blocked => {}
            }
        }

        (
            long_votes,
            short_votes,
            confidence_sum,
            long_modules,
            short_modules,
        )
    }
}

impl Default for CompositeSignal {
    fn default() -> Self {
        Self::new(vec![
            Box::new(DeltaMomentumSignal::default()),
            Box::new(VolumeImbalanceSignal::default()),
            Box::new(CumulativeDeltaSignal::default()),
        ])
    }
}

impl SignalModule for CompositeSignal {
    fn on_analytics(&mut self, ev: &AnalyticsSnapshot) {
        for module in &mut self.modules {
            module.on_analytics(ev);
        }
    }

    fn snapshot(&self) -> SignalSnapshot {
        if self.modules.is_empty() {
            return SignalSnapshot {
                module_id: "composite_v1",
                state: SignalState::Neutral,
                confidence_bps: 0,
                quality_flags: 0,
                reason: "no_child_modules".to_string(),
            };
        }

        let (long_votes, short_votes, confidence_sum, long_modules, short_modules) =
            self.tally_votes();

        let (state, reason) = if long_votes > short_votes && long_votes > 0 {
            (
                SignalState::LongBias,
                format!("composite_long:{}", long_modules.join(",")),
            )
        } else if short_votes > long_votes && short_votes > 0 {
            (
                SignalState::ShortBias,
                format!("composite_short:{}", short_modules.join(",")),
            )
        } else {
            (SignalState::Neutral, "composite_no_majority".to_string())
        };

        SignalSnapshot {
            module_id: "composite_v1",
            state,
            confidence_bps: (confidence_sum / self.modules.len() as u32) as u16,
            quality_flags: 0,
            reason,
        }
    }

    fn quality_gate(&self, q: DataQualityFlags) -> SignalGateDecision {
        if self
            .modules
            .iter()
            .any(|module| module.quality_gate(q) == SignalGateDecision::Block)
        {
            SignalGateDecision::Block
        } else {
            SignalGateDecision::Pass
        }
    }

    fn latest_explanation(&self) -> Option<SignalExplanation> {
        Some(<Self as ExplainableSignalModule>::explanation(self))
    }
}

impl ExplainableSignalModule for CompositeSignal {
    fn explanation(&self) -> SignalExplanation {
        let snapshot = self.snapshot();
        let (long_votes, short_votes, confidence_sum, _, _) = self.tally_votes();
        let reason_code = if self.modules.is_empty() {
            SignalReasonCode::NoChildModules
        } else {
            match snapshot.state {
                SignalState::LongBias => SignalReasonCode::CompositeLongMajority,
                SignalState::ShortBias => SignalReasonCode::CompositeShortMajority,
                SignalState::Neutral | SignalState::Blocked => {
                    SignalReasonCode::CompositeNoMajority
                }
            }
        };

        let average_confidence = if self.modules.is_empty() {
            0
        } else {
            (confidence_sum / self.modules.len() as u32) as u16
        };

        SignalExplanation::from_snapshot(&snapshot, reason_code)
            .with_input(SignalInputValue::integer(
                "module_count",
                self.modules.len() as i64,
            ))
            .with_input(SignalInputValue::integer(
                "long_votes",
                i64::from(long_votes),
            ))
            .with_input(SignalInputValue::integer(
                "short_votes",
                i64::from(short_votes),
            ))
            .with_confidence_component(SignalConfidenceComponent::new(
                "average_child_confidence",
                average_confidence,
            ))
    }
}

pub(crate) fn classify_transition(
    previous: SignalState,
    requested: SignalState,
) -> SignalTransitionKind {
    if previous == requested {
        return SignalTransitionKind::None;
    }

    match (is_directional(previous), is_directional(requested)) {
        (false, true) => SignalTransitionKind::Entry,
        (true, false) => SignalTransitionKind::Exit,
        (true, true) => SignalTransitionKind::Reversal,
        (false, false) => SignalTransitionKind::StateChange,
    }
}

pub(crate) fn is_directional(state: SignalState) -> bool {
    matches!(state, SignalState::LongBias | SignalState::ShortBias)
}

pub(crate) fn same_pending_state(left: &SignalSnapshot, right: &SignalSnapshot) -> bool {
    left.module_id == right.module_id && left.state == right.state
}

pub(crate) fn neutral_like(requested: &SignalSnapshot) -> SignalSnapshot {
    SignalSnapshot {
        module_id: requested.module_id,
        state: SignalState::Neutral,
        confidence_bps: requested.confidence_bps,
        quality_flags: requested.quality_flags,
        reason: "stabilizer_pending".to_string(),
    }
}

pub(crate) fn default_quality_gate(q: DataQualityFlags) -> SignalGateDecision {
    if q.intersects(
        DataQualityFlags::STALE_FEED
            | DataQualityFlags::SEQUENCE_GAP
            | DataQualityFlags::OUT_OF_ORDER
            | DataQualityFlags::ADAPTER_DEGRADED,
    ) {
        SignalGateDecision::Block
    } else {
        SignalGateDecision::Pass
    }
}

pub(crate) const THRESHOLD_PARAM_100: SignalParameterDescriptor = SignalParameterDescriptor {
    name: "threshold",
    description: "Absolute analytics threshold required before the signal emits directional bias.",
    kind: SignalParameterKind::Integer,
    default: Some(SignalParameterValue::Integer(100)),
    min: Some(SignalParameterValue::Integer(0)),
    max: None,
};

pub(crate) const THRESHOLD_PARAM_150: SignalParameterDescriptor = SignalParameterDescriptor {
    name: "threshold",
    description: "Absolute analytics threshold required before the signal emits directional bias.",
    kind: SignalParameterKind::Integer,
    default: Some(SignalParameterValue::Integer(150)),
    min: Some(SignalParameterValue::Integer(0)),
    max: None,
};

pub(crate) const THRESHOLD_PARAM_250: SignalParameterDescriptor = SignalParameterDescriptor {
    name: "threshold",
    description: "Absolute analytics threshold required before the signal emits directional bias.",
    kind: SignalParameterKind::Integer,
    default: Some(SignalParameterValue::Integer(250)),
    min: Some(SignalParameterValue::Integer(0)),
    max: None,
};

pub(crate) const PRICE_BAND_PARAM: SignalParameterDescriptor = SignalParameterDescriptor {
    name: "price_band",
    description: "Maximum integer price distance from point of control used by absorption logic.",
    kind: SignalParameterKind::Integer,
    default: Some(SignalParameterValue::Integer(2)),
    min: Some(SignalParameterValue::Integer(0)),
    max: None,
};

pub(crate) const BREAKOUT_TICKS_PARAM: SignalParameterDescriptor = SignalParameterDescriptor {
    name: "breakout_ticks",
    description: "Minimum integer price distance beyond value area required for sweep detection.",
    kind: SignalParameterKind::Integer,
    default: Some(SignalParameterValue::Integer(1)),
    min: Some(SignalParameterValue::Integer(0)),
    max: None,
};

pub(crate) const DELTA_MOMENTUM_PARAMS: &[SignalParameterDescriptor] = &[THRESHOLD_PARAM_100];
pub(crate) const VOLUME_IMBALANCE_PARAMS: &[SignalParameterDescriptor] = &[THRESHOLD_PARAM_100];
pub(crate) const CUMULATIVE_DELTA_PARAMS: &[SignalParameterDescriptor] = &[THRESHOLD_PARAM_250];
pub(crate) const ABSORPTION_PARAMS: &[SignalParameterDescriptor] =
    &[THRESHOLD_PARAM_150, PRICE_BAND_PARAM];
pub(crate) const EXHAUSTION_PARAMS: &[SignalParameterDescriptor] = &[THRESHOLD_PARAM_150];
pub(crate) const SWEEP_DETECTION_PARAMS: &[SignalParameterDescriptor] =
    &[THRESHOLD_PARAM_150, BREAKOUT_TICKS_PARAM];
pub(crate) const COMPOSITE_PARAMS: &[SignalParameterDescriptor] = &[];

pub(crate) const ANALYTICS_AND_QUALITY: SignalInputMask =
    SignalInputMask::ANALYTICS.union(SignalInputMask::DATA_QUALITY);

/// Static descriptor for `DeltaMomentumSignal`.
pub const DELTA_MOMENTUM_DESCRIPTOR: SignalDescriptor = SignalDescriptor {
    id: "delta_momentum_v1",
    name: "Delta Momentum",
    version: "1",
    description: "Directional bias from latest trade delta crossing an absolute threshold.",
    required_inputs: ANALYTICS_AND_QUALITY,
    warmup: SignalWarmupRequirement::Events(1),
    parameters: DELTA_MOMENTUM_PARAMS,
    output_semantics: SignalOutputSemantics::DirectionalBias,
    deterministic: true,
    checkpointable: false,
};

/// Static descriptor for `VolumeImbalanceSignal`.
pub const VOLUME_IMBALANCE_DESCRIPTOR: SignalDescriptor = SignalDescriptor {
    id: "volume_imbalance_v1",
    name: "Volume Imbalance",
    version: "1",
    description: "Directional bias from session buy-volume versus sell-volume imbalance.",
    required_inputs: ANALYTICS_AND_QUALITY,
    warmup: SignalWarmupRequirement::Events(1),
    parameters: VOLUME_IMBALANCE_PARAMS,
    output_semantics: SignalOutputSemantics::DirectionalBias,
    deterministic: true,
    checkpointable: false,
};

/// Static descriptor for `CumulativeDeltaSignal`.
pub const CUMULATIVE_DELTA_DESCRIPTOR: SignalDescriptor = SignalDescriptor {
    id: "cumulative_delta_v1",
    name: "Cumulative Delta",
    version: "1",
    description: "Session-scale directional bias from cumulative delta crossing a threshold.",
    required_inputs: ANALYTICS_AND_QUALITY,
    warmup: SignalWarmupRequirement::Events(1),
    parameters: CUMULATIVE_DELTA_PARAMS,
    output_semantics: SignalOutputSemantics::DirectionalBias,
    deterministic: true,
    checkpointable: false,
};

/// Static descriptor for `AbsorptionSignal`.
pub const ABSORPTION_DESCRIPTOR: SignalDescriptor = SignalDescriptor {
    id: "absorption_v1",
    name: "Absorption",
    version: "1",
    description: "Reversal bias when strong directional flow fails to move price away from POC.",
    required_inputs: ANALYTICS_AND_QUALITY,
    warmup: SignalWarmupRequirement::Events(1),
    parameters: ABSORPTION_PARAMS,
    output_semantics: SignalOutputSemantics::DirectionalBias,
    deterministic: true,
    checkpointable: false,
};

/// Static descriptor for `ExhaustionSignal`.
pub const EXHAUSTION_DESCRIPTOR: SignalDescriptor = SignalDescriptor {
    id: "exhaustion_v1",
    name: "Exhaustion",
    version: "1",
    description: "Reversal bias when directional flow stalls near the point of control.",
    required_inputs: ANALYTICS_AND_QUALITY,
    warmup: SignalWarmupRequirement::Events(1),
    parameters: EXHAUSTION_PARAMS,
    output_semantics: SignalOutputSemantics::DirectionalBias,
    deterministic: true,
    checkpointable: false,
};

/// Static descriptor for `SweepDetectionSignal`.
pub const SWEEP_DETECTION_DESCRIPTOR: SignalDescriptor = SignalDescriptor {
    id: "sweep_detection_v1",
    name: "Sweep Detection",
    version: "1",
    description: "Breakout bias from value-area breaks accompanied by directional flow.",
    required_inputs: ANALYTICS_AND_QUALITY,
    warmup: SignalWarmupRequirement::Events(1),
    parameters: SWEEP_DETECTION_PARAMS,
    output_semantics: SignalOutputSemantics::DirectionalBias,
    deterministic: true,
    checkpointable: false,
};

/// Static descriptor for `CompositeSignal`.
pub const COMPOSITE_DESCRIPTOR: SignalDescriptor = SignalDescriptor {
    id: "composite_v1",
    name: "Composite",
    version: "1",
    description: "Majority-vote aggregation over child signal modules.",
    required_inputs: ANALYTICS_AND_QUALITY,
    warmup: SignalWarmupRequirement::Events(1),
    parameters: COMPOSITE_PARAMS,
    output_semantics: SignalOutputSemantics::CompositeBias,
    deterministic: true,
    checkpointable: false,
};

pub(crate) const BUILT_IN_SIGNAL_DESCRIPTORS: [SignalDescriptor; 7] = [
    DELTA_MOMENTUM_DESCRIPTOR,
    VOLUME_IMBALANCE_DESCRIPTOR,
    CUMULATIVE_DELTA_DESCRIPTOR,
    ABSORPTION_DESCRIPTOR,
    EXHAUSTION_DESCRIPTOR,
    SWEEP_DETECTION_DESCRIPTOR,
    COMPOSITE_DESCRIPTOR,
];

pub(crate) const BUILT_IN_SIGNAL_REGISTRATIONS: [SignalRegistration; 7] = [
    SignalRegistration::new(
        &DELTA_MOMENTUM_DESCRIPTOR,
        Some(create_delta_momentum_signal),
    ),
    SignalRegistration::new(
        &VOLUME_IMBALANCE_DESCRIPTOR,
        Some(create_volume_imbalance_signal),
    ),
    SignalRegistration::new(
        &CUMULATIVE_DELTA_DESCRIPTOR,
        Some(create_cumulative_delta_signal),
    ),
    SignalRegistration::new(&ABSORPTION_DESCRIPTOR, Some(create_absorption_signal)),
    SignalRegistration::new(&EXHAUSTION_DESCRIPTOR, Some(create_exhaustion_signal)),
    SignalRegistration::new(
        &SWEEP_DETECTION_DESCRIPTOR,
        Some(create_sweep_detection_signal),
    ),
    SignalRegistration::new(&COMPOSITE_DESCRIPTOR, Some(create_composite_signal)),
];

/// Returns descriptors for all built-in signal modules.
pub fn built_in_signal_descriptors() -> &'static [SignalDescriptor] {
    &BUILT_IN_SIGNAL_DESCRIPTORS
}

/// Returns registrations for all built-in signal modules.
pub fn built_in_signal_registrations() -> &'static [SignalRegistration] {
    &BUILT_IN_SIGNAL_REGISTRATIONS
}

/// Finds a built-in signal descriptor by stable signal identifier.
pub fn describe_signal(id: &str) -> Option<&'static SignalDescriptor> {
    BUILT_IN_SIGNAL_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.id == id)
}

/// Exports built-in signal descriptors as compact JSON.
pub fn built_in_signal_descriptors_json() -> String {
    SignalRegistry::with_built_ins().descriptors_json()
}

pub(crate) fn create_delta_momentum_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(DeltaMomentumSignal::new(integer_parameter(
        config,
        &DELTA_MOMENTUM_DESCRIPTOR,
        "threshold",
    )?)))
}

pub(crate) fn create_volume_imbalance_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(VolumeImbalanceSignal::new(integer_parameter(
        config,
        &VOLUME_IMBALANCE_DESCRIPTOR,
        "threshold",
    )?)))
}

pub(crate) fn create_cumulative_delta_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(CumulativeDeltaSignal::new(integer_parameter(
        config,
        &CUMULATIVE_DELTA_DESCRIPTOR,
        "threshold",
    )?)))
}

pub(crate) fn create_absorption_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(AbsorptionSignal::new(
        integer_parameter(config, &ABSORPTION_DESCRIPTOR, "threshold")?,
        integer_parameter(config, &ABSORPTION_DESCRIPTOR, "price_band")?,
    )))
}

pub(crate) fn create_exhaustion_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(ExhaustionSignal::new(integer_parameter(
        config,
        &EXHAUSTION_DESCRIPTOR,
        "threshold",
    )?)))
}

pub(crate) fn create_sweep_detection_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(SweepDetectionSignal::new(
        integer_parameter(config, &SWEEP_DETECTION_DESCRIPTOR, "threshold")?,
        integer_parameter(config, &SWEEP_DETECTION_DESCRIPTOR, "breakout_ticks")?,
    )))
}

pub(crate) fn create_composite_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    validate_signal_config(&COMPOSITE_DESCRIPTOR, config)?;
    Ok(Box::new(CompositeSignal::default()))
}

pub(crate) fn integer_parameter(
    config: &SignalConfig<'_>,
    descriptor: &'static SignalDescriptor,
    name: &'static str,
) -> SignalRegistryResult<i64> {
    if let Some(value) = config.parameter(name) {
        return match value {
            SignalConfigValue::Integer(value) => Ok(value),
            other => Err(SignalRegistryError::InvalidParameterType {
                signal_id: descriptor.id,
                name,
                expected: SignalParameterKind::Integer,
                actual: other.kind(),
            }),
        };
    }

    descriptor
        .parameter(name)
        .and_then(|parameter| parameter.default)
        .and_then(|value| match value {
            SignalParameterValue::Integer(value) => Some(value),
            SignalParameterValue::Float(_)
            | SignalParameterValue::Boolean(_)
            | SignalParameterValue::Text(_) => None,
        })
        .ok_or(SignalRegistryError::UnknownParameter {
            signal_id: descriptor.id,
            name: name.to_string(),
        })
}

pub(crate) fn validate_signal_config(
    descriptor: &'static SignalDescriptor,
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<()> {
    for (index, parameter) in config.parameters.iter().enumerate() {
        if config.parameters[..index]
            .iter()
            .any(|previous| previous.name == parameter.name)
        {
            return Err(SignalRegistryError::DuplicateParameter {
                signal_id: descriptor.id,
                name: parameter.name.to_string(),
            });
        }

        let Some(parameter_descriptor) = descriptor.parameter(parameter.name) else {
            return Err(SignalRegistryError::UnknownParameter {
                signal_id: descriptor.id,
                name: parameter.name.to_string(),
            });
        };

        if parameter.value.kind() != parameter_descriptor.kind {
            return Err(SignalRegistryError::InvalidParameterType {
                signal_id: descriptor.id,
                name: parameter_descriptor.name,
                expected: parameter_descriptor.kind,
                actual: parameter.value.kind(),
            });
        }

        if let Some(min) = parameter_descriptor.min {
            if config_value_is_below(parameter.value, min) {
                return Err(SignalRegistryError::ParameterBelowMinimum {
                    signal_id: descriptor.id,
                    name: parameter_descriptor.name,
                    min,
                    actual: config_value_to_static(parameter.value),
                });
            }
        }

        if let Some(max) = parameter_descriptor.max {
            if config_value_is_above(parameter.value, max) {
                return Err(SignalRegistryError::ParameterAboveMaximum {
                    signal_id: descriptor.id,
                    name: parameter_descriptor.name,
                    max,
                    actual: config_value_to_static(parameter.value),
                });
            }
        }
    }

    Ok(())
}

pub(crate) fn config_value_is_below(
    actual: SignalConfigValue<'_>,
    min: SignalParameterValue,
) -> bool {
    match (actual, min) {
        (SignalConfigValue::Integer(actual), SignalParameterValue::Integer(min)) => actual < min,
        (SignalConfigValue::Float(actual), SignalParameterValue::Float(min)) => actual < min,
        _ => false,
    }
}

pub(crate) fn config_value_is_above(
    actual: SignalConfigValue<'_>,
    max: SignalParameterValue,
) -> bool {
    match (actual, max) {
        (SignalConfigValue::Integer(actual), SignalParameterValue::Integer(max)) => actual > max,
        (SignalConfigValue::Float(actual), SignalParameterValue::Float(max)) => actual > max,
        _ => false,
    }
}

pub(crate) fn config_value_to_static(value: SignalConfigValue<'_>) -> SignalConfigValue<'static> {
    match value {
        SignalConfigValue::Integer(value) => SignalConfigValue::Integer(value),
        SignalConfigValue::Float(value) => SignalConfigValue::Float(value),
        SignalConfigValue::Boolean(value) => SignalConfigValue::Boolean(value),
        SignalConfigValue::Text(_) => SignalConfigValue::Text("<text>"),
    }
}

pub(crate) fn push_descriptor_json(out: &mut String, descriptor: &SignalDescriptor) {
    out.push('{');
    push_json_field(out, "id", descriptor.id);
    out.push(',');
    push_json_field(out, "name", descriptor.name);
    out.push(',');
    push_json_field(out, "version", descriptor.version);
    out.push(',');
    push_json_field(out, "description", descriptor.description);
    out.push(',');
    out.push_str("\"required_inputs_bits\":");
    out.push_str(&descriptor.required_inputs.bits().to_string());
    out.push(',');
    out.push_str("\"required_inputs\":");
    push_input_mask_json(out, descriptor.required_inputs);
    out.push(',');
    out.push_str("\"warmup\":");
    push_warmup_json(out, descriptor.warmup);
    out.push(',');
    out.push_str("\"parameters\":");
    push_parameters_json(out, descriptor.parameters);
    out.push(',');
    push_json_field(
        out,
        "output_semantics",
        output_semantics_name(descriptor.output_semantics),
    );
    out.push(',');
    out.push_str("\"deterministic\":");
    out.push_str(if descriptor.deterministic {
        "true"
    } else {
        "false"
    });
    out.push(',');
    out.push_str("\"checkpointable\":");
    out.push_str(if descriptor.checkpointable {
        "true"
    } else {
        "false"
    });
    out.push('}');
}

pub(crate) fn push_explanation_json(out: &mut String, explanation: &SignalExplanation) {
    out.push('{');
    push_json_field(out, "module_id", explanation.module_id);
    out.push(',');
    push_json_field(out, "state", signal_state_name(explanation.state));
    out.push(',');
    out.push_str("\"confidence_bps\":");
    out.push_str(&explanation.confidence_bps.to_string());
    out.push(',');
    out.push_str("\"quality_flags\":");
    out.push_str(&explanation.quality_flags.to_string());
    out.push(',');
    push_json_field(out, "reason_code", explanation.reason_code.as_str());
    out.push(',');
    push_json_field(out, "reason", &explanation.reason);
    out.push(',');
    out.push_str("\"inputs\":");
    push_input_values_json(out, &explanation.inputs);
    out.push(',');
    out.push_str("\"thresholds\":");
    push_thresholds_json(out, &explanation.thresholds);
    out.push(',');
    out.push_str("\"confidence_components\":");
    push_confidence_components_json(out, &explanation.confidence_components);
    out.push('}');
}

pub(crate) fn push_json_field(out: &mut String, name: &str, value: &str) {
    push_json_string(out, name);
    out.push(':');
    push_json_string(out, value);
}

pub(crate) fn push_json_usize_field(out: &mut String, name: &str, value: usize) {
    push_json_string(out, name);
    out.push(':');
    out.push_str(&value.to_string());
}

pub(crate) fn push_json_i64_field(out: &mut String, name: &str, value: i64) {
    push_json_string(out, name);
    out.push(':');
    out.push_str(&value.to_string());
}

pub(crate) fn push_json_u16_field(out: &mut String, name: &str, value: u16) {
    push_json_string(out, name);
    out.push(':');
    out.push_str(&value.to_string());
}

pub(crate) fn push_json_bool_field(out: &mut String, name: &str, value: bool) {
    push_json_string(out, name);
    out.push(':');
    out.push_str(if value { "true" } else { "false" });
}

pub(crate) fn push_validation_sample_json(out: &mut String, sample: &SignalValidationSample) {
    out.push('{');
    push_json_usize_field(out, "event_index", sample.event_index);
    out.push(',');
    push_json_usize_field(out, "markout_event_index", sample.markout_event_index);
    out.push_str(",\"snapshot\":{");
    push_json_field(out, "module_id", sample.snapshot.module_id);
    out.push(',');
    push_json_field(out, "state", signal_state_name(sample.snapshot.state));
    out.push(',');
    push_json_u16_field(out, "confidence_bps", sample.snapshot.confidence_bps);
    out.push_str(",\"quality_flags\":");
    out.push_str(&sample.snapshot.quality_flags.to_string());
    out.push(',');
    push_json_field(out, "reason", &sample.snapshot.reason);
    out.push('}');
    out.push(',');
    push_json_i64_field(out, "entry_price", sample.entry_price);
    out.push(',');
    push_json_i64_field(out, "markout_price", sample.markout_price);
    out.push(',');
    push_json_i64_field(out, "price_change", sample.price_change);
    out.push(',');
    push_json_field(out, "markout_direction", sample.markout_direction.as_str());
    out.push_str(",\"predicted_direction\":");
    match sample.predicted_direction {
        Some(direction) => push_json_string(out, direction.as_str()),
        None => out.push_str("null"),
    }
    out.push_str(",\"correct\":");
    match sample.correct {
        Some(value) => out.push_str(if value { "true" } else { "false" }),
        None => out.push_str("null"),
    }
    out.push('}');
}

pub(crate) fn push_validation_warning_json(out: &mut String, warning: &SignalValidationWarning) {
    out.push('{');
    match warning {
        SignalValidationWarning::EmptyInput => {
            push_json_field(out, "code", "empty_input");
        }
        SignalValidationWarning::ZeroMarkoutHorizon => {
            push_json_field(out, "code", "zero_markout_horizon");
        }
        SignalValidationWarning::MissingMarkout {
            event_index,
            requested_horizon_events,
        } => {
            push_json_field(out, "code", "missing_markout");
            out.push(',');
            push_json_usize_field(out, "event_index", *event_index);
            out.push(',');
            push_json_usize_field(out, "requested_horizon_events", *requested_horizon_events);
        }
        SignalValidationWarning::NonMonotonicTimestamp {
            event_index,
            previous_ts_exchange_ns,
            current_ts_exchange_ns,
        } => {
            push_json_field(out, "code", "non_monotonic_timestamp");
            out.push(',');
            push_json_usize_field(out, "event_index", *event_index);
            out.push_str(",\"previous_ts_exchange_ns\":");
            out.push_str(&previous_ts_exchange_ns.to_string());
            out.push_str(",\"current_ts_exchange_ns\":");
            out.push_str(&current_ts_exchange_ns.to_string());
        }
    }
    out.push('}');
}

pub(crate) fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                out.push_str("\\u");
                out.push_str(&format!("{:04x}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

pub(crate) fn push_input_mask_json(out: &mut String, mask: SignalInputMask) {
    let inputs = [
        (SignalInputMask::ANALYTICS, "analytics"),
        (SignalInputMask::DATA_QUALITY, "data_quality"),
        (SignalInputMask::BOOK, "book"),
        (SignalInputMask::ADVANCED_ANALYTICS, "advanced_analytics"),
        (SignalInputMask::MARKET_REGIME, "market_regime"),
        (SignalInputMask::POSITION, "position"),
        (SignalInputMask::RISK, "risk"),
    ];
    out.push('[');
    let mut written = 0_usize;
    for (input, name) in inputs {
        if mask.contains(input) {
            if written > 0 {
                out.push(',');
            }
            push_json_string(out, name);
            written += 1;
        }
    }
    out.push(']');
}

pub(crate) fn push_warmup_json(out: &mut String, warmup: SignalWarmupRequirement) {
    match warmup {
        SignalWarmupRequirement::None => out.push_str("{\"kind\":\"none\"}"),
        SignalWarmupRequirement::Events(events) => {
            out.push_str("{\"kind\":\"events\",\"events\":");
            out.push_str(&events.to_string());
            out.push('}');
        }
        SignalWarmupRequirement::MarketTimeNs(market_time_ns) => {
            out.push_str("{\"kind\":\"market_time_ns\",\"market_time_ns\":");
            out.push_str(&market_time_ns.to_string());
            out.push('}');
        }
        SignalWarmupRequirement::CompletedBars(completed_bars) => {
            out.push_str("{\"kind\":\"completed_bars\",\"completed_bars\":");
            out.push_str(&completed_bars.to_string());
            out.push('}');
        }
        SignalWarmupRequirement::All(requirements) => {
            out.push_str("{\"kind\":\"all\",\"requirements\":[");
            for (index, requirement) in requirements.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_warmup_json(out, *requirement);
            }
            out.push_str("]}");
        }
    }
}

pub(crate) fn push_parameters_json(out: &mut String, parameters: &[SignalParameterDescriptor]) {
    out.push('[');
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_field(out, "name", parameter.name);
        out.push(',');
        push_json_field(out, "description", parameter.description);
        out.push(',');
        push_json_field(out, "kind", parameter_kind_name(parameter.kind));
        out.push(',');
        out.push_str("\"default\":");
        push_optional_parameter_value_json(out, parameter.default);
        out.push(',');
        out.push_str("\"min\":");
        push_optional_parameter_value_json(out, parameter.min);
        out.push(',');
        out.push_str("\"max\":");
        push_optional_parameter_value_json(out, parameter.max);
        out.push('}');
    }
    out.push(']');
}

pub(crate) fn push_input_values_json(out: &mut String, inputs: &[SignalInputValue]) {
    out.push('[');
    for (index, input) in inputs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_field(out, "name", input.name);
        out.push(',');
        out.push_str("\"value\":");
        push_parameter_value_json(out, input.value);
        out.push('}');
    }
    out.push(']');
}

pub(crate) fn push_thresholds_json(out: &mut String, thresholds: &[SignalThreshold]) {
    out.push('[');
    for (index, threshold) in thresholds.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_field(out, "name", threshold.name);
        out.push(',');
        out.push_str("\"value\":");
        push_parameter_value_json(out, threshold.value);
        out.push('}');
    }
    out.push(']');
}

pub(crate) fn push_confidence_components_json(
    out: &mut String,
    components: &[SignalConfidenceComponent],
) {
    out.push('[');
    for (index, component) in components.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_field(out, "name", component.name);
        out.push(',');
        out.push_str("\"value_bps\":");
        out.push_str(&component.value_bps.to_string());
        out.push('}');
    }
    out.push(']');
}

pub(crate) fn push_optional_parameter_value_json(
    out: &mut String,
    value: Option<SignalParameterValue>,
) {
    match value {
        Some(value) => push_parameter_value_json(out, value),
        None => out.push_str("null"),
    }
}

pub(crate) fn push_parameter_value_json(out: &mut String, value: SignalParameterValue) {
    match value {
        SignalParameterValue::Integer(value) => out.push_str(&value.to_string()),
        SignalParameterValue::Float(value) => out.push_str(&value.to_string()),
        SignalParameterValue::Boolean(value) => out.push_str(if value { "true" } else { "false" }),
        SignalParameterValue::Text(value) => push_json_string(out, value),
    }
}

pub(crate) fn signal_state_name(state: SignalState) -> &'static str {
    match state {
        SignalState::Neutral => "neutral",
        SignalState::LongBias => "long_bias",
        SignalState::ShortBias => "short_bias",
        SignalState::Blocked => "blocked",
    }
}

pub(crate) fn parameter_kind_name(kind: SignalParameterKind) -> &'static str {
    match kind {
        SignalParameterKind::Integer => "integer",
        SignalParameterKind::Float => "float",
        SignalParameterKind::Boolean => "boolean",
        SignalParameterKind::Text => "text",
    }
}

pub(crate) fn output_semantics_name(output_semantics: SignalOutputSemantics) -> &'static str {
    match output_semantics {
        SignalOutputSemantics::DirectionalBias => "directional_bias",
        SignalOutputSemantics::CompositeBias => "composite_bias",
        SignalOutputSemantics::Informational => "informational",
        SignalOutputSemantics::Veto => "veto",
    }
}
