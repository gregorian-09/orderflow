use super::*;

/// Transition class used by signal stabilization policies.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalTransitionKind {
    /// No state transition occurred.
    None,
    /// Transition from neutral/blocked into long or short bias.
    Entry,
    /// Transition from long or short bias into neutral/blocked.
    Exit,
    /// Direct transition between long and short bias.
    Reversal,
    /// Other state transition.
    StateChange,
}

/// Reason a requested signal transition was suppressed.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalSuppressionReason {
    /// The requested output was accepted.
    None,
    /// The requested confidence did not satisfy hysteresis thresholds.
    Hysteresis,
    /// The requested transition is waiting for repeated/time confirmation.
    DebouncePending,
    /// The requested transition occurred during a cooldown window.
    CooldownActive,
}

/// Result returned by [`SignalStabilizer`].
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct StabilizedSignal {
    /// Snapshot requested by the underlying signal.
    pub requested: SignalSnapshot,
    /// Snapshot emitted after stabilization.
    pub emitted: SignalSnapshot,
    /// Whether the requested snapshot became the emitted snapshot.
    pub accepted: bool,
    /// Reason the requested snapshot was suppressed.
    pub suppression_reason: SignalSuppressionReason,
    /// Transition kind represented by the request.
    pub transition: SignalTransitionKind,
}

/// Stable machine-readable reason for a signal output.
///
/// These codes are intended for audit logs, dashboards, replay review, and
/// downstream language bindings. They complement the human-readable
/// `SignalSnapshot::reason` string without changing that shared snapshot
/// contract.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalReasonCode {
    /// No specific reason code is available.
    Unknown,
    /// Latest trade delta crossed the positive momentum threshold.
    DeltaMomentumPositive,
    /// Latest trade delta crossed the negative momentum threshold.
    DeltaMomentumNegative,
    /// Latest trade delta remained inside the configured band.
    DeltaMomentumInsideBand,
    /// Session buy volume exceeded sell volume by the configured threshold.
    BuyVolumeImbalance,
    /// Session sell volume exceeded buy volume by the configured threshold.
    SellVolumeImbalance,
    /// Session buy/sell volume imbalance remained inside the configured band.
    VolumeInsideBand,
    /// Session cumulative delta crossed the positive threshold.
    CumulativeDeltaPositive,
    /// Session cumulative delta crossed the negative threshold.
    CumulativeDeltaNegative,
    /// Session cumulative delta remained inside the configured band.
    CumulativeDeltaInsideBand,
    /// Selling pressure was absorbed near the point of control.
    SellAbsorptionDetected,
    /// Buying pressure was absorbed near the point of control.
    BuyAbsorptionDetected,
    /// Absorption criteria were not met.
    AbsorptionNotDetected,
    /// Buying pressure exhausted near the point of control.
    BuyExhaustionDetected,
    /// Selling pressure exhausted near the point of control.
    SellExhaustionDetected,
    /// Exhaustion criteria were not met.
    ExhaustionNotDetected,
    /// Upside value-area sweep criteria were met.
    UpsideSweepDetected,
    /// Downside value-area sweep criteria were met.
    DownsideSweepDetected,
    /// Sweep criteria were not met.
    SweepNotDetected,
    /// Composite children voted for a long bias.
    CompositeLongMajority,
    /// Composite children voted for a short bias.
    CompositeShortMajority,
    /// Composite children did not produce a directional majority.
    CompositeNoMajority,
    /// Composite signal has no child modules.
    NoChildModules,
    /// Ensemble policy selected a long bias.
    EnsembleLongSelected,
    /// Ensemble policy selected a short bias.
    EnsembleShortSelected,
    /// Ensemble policy did not select a directional output.
    EnsembleNoSelection,
    /// Ensemble policy applied a child veto.
    EnsembleVetoApplied,
    /// Signal output was blocked by a data-quality or risk gate.
    QualityBlocked,
    /// Stabilization suppressed a transition due to hysteresis.
    StabilizerHysteresis,
    /// Stabilization is waiting for debounce confirmation.
    StabilizerDebouncePending,
    /// Stabilization suppressed a transition during cooldown.
    StabilizerCooldownActive,
}

impl SignalReasonCode {
    /// Returns the stable string representation of this reason code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::DeltaMomentumPositive => "delta_momentum_positive",
            Self::DeltaMomentumNegative => "delta_momentum_negative",
            Self::DeltaMomentumInsideBand => "delta_momentum_inside_band",
            Self::BuyVolumeImbalance => "buy_volume_imbalance",
            Self::SellVolumeImbalance => "sell_volume_imbalance",
            Self::VolumeInsideBand => "volume_inside_band",
            Self::CumulativeDeltaPositive => "cumulative_delta_positive",
            Self::CumulativeDeltaNegative => "cumulative_delta_negative",
            Self::CumulativeDeltaInsideBand => "cumulative_delta_inside_band",
            Self::SellAbsorptionDetected => "sell_absorption_detected",
            Self::BuyAbsorptionDetected => "buy_absorption_detected",
            Self::AbsorptionNotDetected => "absorption_not_detected",
            Self::BuyExhaustionDetected => "buy_exhaustion_detected",
            Self::SellExhaustionDetected => "sell_exhaustion_detected",
            Self::ExhaustionNotDetected => "exhaustion_not_detected",
            Self::UpsideSweepDetected => "upside_sweep_detected",
            Self::DownsideSweepDetected => "downside_sweep_detected",
            Self::SweepNotDetected => "sweep_not_detected",
            Self::CompositeLongMajority => "composite_long_majority",
            Self::CompositeShortMajority => "composite_short_majority",
            Self::CompositeNoMajority => "composite_no_majority",
            Self::NoChildModules => "no_child_modules",
            Self::EnsembleLongSelected => "ensemble_long_selected",
            Self::EnsembleShortSelected => "ensemble_short_selected",
            Self::EnsembleNoSelection => "ensemble_no_selection",
            Self::EnsembleVetoApplied => "ensemble_veto_applied",
            Self::QualityBlocked => "quality_blocked",
            Self::StabilizerHysteresis => "stabilizer_hysteresis",
            Self::StabilizerDebouncePending => "stabilizer_debounce_pending",
            Self::StabilizerCooldownActive => "stabilizer_cooldown_active",
        }
    }
}

impl From<SignalSuppressionReason> for SignalReasonCode {
    fn from(reason: SignalSuppressionReason) -> Self {
        match reason {
            SignalSuppressionReason::None => Self::Unknown,
            SignalSuppressionReason::Hysteresis => Self::StabilizerHysteresis,
            SignalSuppressionReason::DebouncePending => Self::StabilizerDebouncePending,
            SignalSuppressionReason::CooldownActive => Self::StabilizerCooldownActive,
        }
    }
}

/// One observed input value included in a signal explanation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalInputValue {
    /// Stable input name.
    pub name: &'static str,
    /// Observed input value.
    pub value: SignalParameterValue,
}

impl SignalInputValue {
    /// Creates an input value from a stable name and parameter-compatible value.
    pub const fn new(name: &'static str, value: SignalParameterValue) -> Self {
        Self { name, value }
    }

    /// Creates an integer input value.
    pub const fn integer(name: &'static str, value: i64) -> Self {
        Self::new(name, SignalParameterValue::Integer(value))
    }

    /// Creates a floating-point input value.
    pub const fn float(name: &'static str, value: f64) -> Self {
        Self::new(name, SignalParameterValue::Float(value))
    }

    /// Creates a boolean input value.
    pub const fn boolean(name: &'static str, value: bool) -> Self {
        Self::new(name, SignalParameterValue::Boolean(value))
    }

    /// Creates a static text input value.
    pub const fn text(name: &'static str, value: &'static str) -> Self {
        Self::new(name, SignalParameterValue::Text(value))
    }
}

/// One configured threshold included in a signal explanation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalThreshold {
    /// Stable threshold name.
    pub name: &'static str,
    /// Configured threshold value.
    pub value: SignalParameterValue,
}

impl SignalThreshold {
    /// Creates a threshold from a stable name and parameter-compatible value.
    pub const fn new(name: &'static str, value: SignalParameterValue) -> Self {
        Self { name, value }
    }

    /// Creates an integer threshold.
    pub const fn integer(name: &'static str, value: i64) -> Self {
        Self::new(name, SignalParameterValue::Integer(value))
    }

    /// Creates a floating-point threshold.
    pub const fn float(name: &'static str, value: f64) -> Self {
        Self::new(name, SignalParameterValue::Float(value))
    }

    /// Creates a boolean threshold.
    pub const fn boolean(name: &'static str, value: bool) -> Self {
        Self::new(name, SignalParameterValue::Boolean(value))
    }

    /// Creates a static text threshold.
    pub const fn text(name: &'static str, value: &'static str) -> Self {
        Self::new(name, SignalParameterValue::Text(value))
    }
}

/// One confidence contributor included in a signal explanation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalConfidenceComponent {
    /// Stable contributor name.
    pub name: &'static str,
    /// Contributor value in basis points.
    pub value_bps: u16,
}

impl SignalConfidenceComponent {
    /// Creates a confidence contributor.
    pub const fn new(name: &'static str, value_bps: u16) -> Self {
        Self { name, value_bps }
    }
}

/// Structured diagnostic explanation for a signal snapshot.
///
/// Explanations are intended for audit/replay and UI paths. They can allocate
/// for vectors and reason text; keep using `SignalSnapshot` on the tight signal
/// hot path when only state is needed.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct SignalExplanation {
    /// Stable signal module id.
    pub module_id: &'static str,
    /// Signal state explained by this payload.
    pub state: SignalState,
    /// Confidence in basis points.
    pub confidence_bps: u16,
    /// Quality flags attached to the explained snapshot.
    pub quality_flags: u32,
    /// Machine-readable reason code.
    pub reason_code: SignalReasonCode,
    /// Human-readable reason string.
    pub reason: String,
    /// Observed input values used by the decision.
    pub inputs: Vec<SignalInputValue>,
    /// Configured thresholds used by the decision.
    pub thresholds: Vec<SignalThreshold>,
    /// Confidence contributors used by the decision.
    pub confidence_components: Vec<SignalConfidenceComponent>,
}

impl SignalExplanation {
    /// Creates a structured explanation.
    pub fn new(
        module_id: &'static str,
        state: SignalState,
        confidence_bps: u16,
        quality_flags: u32,
        reason_code: SignalReasonCode,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            module_id,
            state,
            confidence_bps,
            quality_flags,
            reason_code,
            reason: reason.into(),
            inputs: Vec::new(),
            thresholds: Vec::new(),
            confidence_components: Vec::new(),
        }
    }

    /// Creates an explanation from an existing signal snapshot.
    pub fn from_snapshot(snapshot: &SignalSnapshot, reason_code: SignalReasonCode) -> Self {
        Self::new(
            snapshot.module_id,
            snapshot.state,
            snapshot.confidence_bps,
            snapshot.quality_flags,
            reason_code,
            snapshot.reason.clone(),
        )
    }

    /// Returns this explanation with one observed input appended.
    pub fn with_input(mut self, input: SignalInputValue) -> Self {
        self.inputs.push(input);
        self
    }

    /// Returns this explanation with one configured threshold appended.
    pub fn with_threshold(mut self, threshold: SignalThreshold) -> Self {
        self.thresholds.push(threshold);
        self
    }

    /// Returns this explanation with one confidence contributor appended.
    pub fn with_confidence_component(mut self, component: SignalConfidenceComponent) -> Self {
        self.confidence_components.push(component);
        self
    }

    /// Serializes this explanation as compact dependency-free JSON.
    ///
    /// The JSON payload is intended for bindings, dashboards, and audit logs.
    /// Field additions are additive; existing field names are stable once
    /// published.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        push_explanation_json(&mut out, self);
        out
    }
}

/// Controls whether explanations should be emitted for every evaluation or only transitions.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SignalExplanationMode {
    /// Emit an explanation for every evaluated snapshot.
    #[default]
    Always,
    /// Emit only when the signal state differs from the previous state.
    TransitionsOnly,
}

impl SignalExplanationMode {
    /// Returns `true` when an explanation should be emitted for these states.
    pub fn should_emit(
        self,
        previous_state: Option<SignalState>,
        current_state: SignalState,
    ) -> bool {
        match self {
            Self::Always => true,
            Self::TransitionsOnly => previous_state != Some(current_state),
        }
    }

    /// Returns `true` when an explanation should be emitted for these snapshots.
    pub fn should_emit_snapshot(
        self,
        previous: Option<&SignalSnapshot>,
        current: &SignalSnapshot,
    ) -> bool {
        self.should_emit(previous.map(|snapshot| snapshot.state), current.state)
    }
}

/// Optional extension trait for modules that expose structured explanations.
///
/// This is intentionally separate from [`SignalModule`] so existing downstream
/// implementations do not need to add a new required method.
pub trait ExplainableSignalModule: SignalModule {
    /// Returns a structured explanation for the current snapshot.
    fn explanation(&self) -> SignalExplanation;
}

#[derive(Debug, Clone)]
pub(crate) struct PendingSignal {
    pub(crate) snapshot: SignalSnapshot,
    pub(crate) first_seen_ns: u64,
    pub(crate) confirming_events: u32,
}
