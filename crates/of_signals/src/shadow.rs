use super::*;

/// Runtime mode for a signal in production or validation hosts.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SignalRunMode {
    /// Output can be consumed by strategy and risk code.
    #[default]
    Active,
    /// Output is computed and recorded, but must not affect trading decisions.
    Shadow,
    /// Inputs/features are recorded, but the expensive signal may be skipped.
    RecordOnly,
    /// Signal is not evaluated or recorded.
    Disabled,
}

/// Evaluation and publication behavior implied by a run mode.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalRunModeDecision {
    /// Run mode that produced this decision.
    pub mode: SignalRunMode,
    /// Whether the signal should be evaluated.
    pub evaluate: bool,
    /// Whether signal output may affect trading decisions.
    pub publish_for_trading: bool,
    /// Whether input/features should be recorded.
    pub record_input: bool,
    /// Whether output should be recorded.
    pub record_output: bool,
}

impl SignalRunModeDecision {
    /// Creates a behavior decision from a run mode.
    pub const fn from_mode(mode: SignalRunMode) -> Self {
        match mode {
            SignalRunMode::Active => Self {
                mode,
                evaluate: true,
                publish_for_trading: true,
                record_input: true,
                record_output: true,
            },
            SignalRunMode::Shadow => Self {
                mode,
                evaluate: true,
                publish_for_trading: false,
                record_input: true,
                record_output: true,
            },
            SignalRunMode::RecordOnly => Self {
                mode,
                evaluate: false,
                publish_for_trading: false,
                record_input: true,
                record_output: false,
            },
            SignalRunMode::Disabled => Self {
                mode,
                evaluate: false,
                publish_for_trading: false,
                record_input: false,
                record_output: false,
            },
        }
    }
}

/// One production-versus-candidate shadow comparison sample.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SignalShadowSample {
    /// Event index where both signals were compared.
    pub event_index: usize,
    /// Optional exchange timestamp.
    pub ts_exchange_ns: Option<u64>,
    /// Production signal snapshot.
    pub production: SignalSnapshot,
    /// Candidate/shadow signal snapshot.
    pub candidate: SignalSnapshot,
    /// Production run mode.
    pub production_mode: SignalRunMode,
    /// Candidate run mode.
    pub candidate_mode: SignalRunMode,
    /// Whether production and candidate states differ.
    pub disagreement: bool,
    /// Candidate confidence minus production confidence.
    pub confidence_delta_bps: i32,
    /// Optional future markout label.
    pub markout_direction: Option<SignalMarkoutDirection>,
    /// Whether production matched the markout label.
    pub production_correct: Option<bool>,
    /// Whether candidate matched the markout label.
    pub candidate_correct: Option<bool>,
}

impl SignalShadowSample {
    /// Creates a shadow comparison sample.
    pub fn compare(
        event_index: usize,
        production: SignalSnapshot,
        candidate: SignalSnapshot,
    ) -> Self {
        let disagreement = production.state != candidate.state;
        let confidence_delta_bps =
            i32::from(candidate.confidence_bps) - i32::from(production.confidence_bps);
        Self {
            event_index,
            ts_exchange_ns: None,
            production,
            candidate,
            production_mode: SignalRunMode::Active,
            candidate_mode: SignalRunMode::Shadow,
            disagreement,
            confidence_delta_bps,
            markout_direction: None,
            production_correct: None,
            candidate_correct: None,
        }
    }

    /// Returns this sample with an exchange timestamp.
    pub const fn with_ts_exchange_ns(mut self, ts_exchange_ns: u64) -> Self {
        self.ts_exchange_ns = Some(ts_exchange_ns);
        self
    }

    /// Returns this sample with explicit run modes.
    pub const fn with_modes(
        mut self,
        production_mode: SignalRunMode,
        candidate_mode: SignalRunMode,
    ) -> Self {
        self.production_mode = production_mode;
        self.candidate_mode = candidate_mode;
        self
    }

    /// Returns this sample scored against a future markout label.
    pub fn with_markout(mut self, markout_direction: SignalMarkoutDirection) -> Self {
        self.markout_direction = Some(markout_direction);
        self.production_correct = score_direction(
            signal_state_direction(self.production.state),
            markout_direction,
        );
        self.candidate_correct = score_direction(
            signal_state_direction(self.candidate.state),
            markout_direction,
        );
        self
    }
}

/// Configuration for shadow comparison reports.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalShadowComparisonConfig {
    /// Whether retained samples should be included in the report.
    pub store_samples: bool,
}

impl SignalShadowComparisonConfig {
    /// Creates shadow comparison config.
    pub const fn new() -> Self {
        Self {
            store_samples: false,
        }
    }

    /// Returns config with sample retention changed.
    pub const fn with_store_samples(mut self, store_samples: bool) -> Self {
        self.store_samples = store_samples;
        self
    }
}

impl Default for SignalShadowComparisonConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Report comparing production and shadow/candidate signal output.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SignalShadowComparisonReport {
    /// Report configuration.
    pub config: SignalShadowComparisonConfig,
    /// Total samples inspected.
    pub total_samples: usize,
    /// Samples where both snapshots were compared.
    pub compared_samples: usize,
    /// Samples where production and candidate states matched.
    pub state_agreements: usize,
    /// Samples where production and candidate states differed.
    pub state_disagreements: usize,
    /// Production directional predictions.
    pub production_directional: usize,
    /// Candidate directional predictions.
    pub candidate_directional: usize,
    /// Samples where both sides were directional.
    pub both_directional: usize,
    /// Samples where production was correct.
    pub production_correct: usize,
    /// Samples where candidate was correct.
    pub candidate_correct: usize,
    /// Samples where production was correct and candidate was not.
    pub production_only_correct: usize,
    /// Samples where candidate was correct and production was not.
    pub candidate_only_correct: usize,
    /// Average candidate-minus-production confidence delta.
    pub average_confidence_delta_bps: i32,
    /// Retained samples.
    pub samples: Vec<SignalShadowSample>,
}

impl SignalShadowComparisonReport {
    /// Builds a comparison report from shadow samples.
    pub fn from_samples(
        samples: &[SignalShadowSample],
        config: SignalShadowComparisonConfig,
    ) -> Self {
        build_shadow_comparison_report(samples, config)
    }

    /// Returns state agreement rate in basis points.
    pub fn agreement_bps(&self) -> Option<u16> {
        ratio_bps(self.state_agreements, self.compared_samples)
    }

    /// Returns production directional accuracy in basis points.
    pub fn production_accuracy_bps(&self) -> Option<u16> {
        ratio_bps(self.production_correct, self.production_directional)
    }

    /// Returns candidate directional accuracy in basis points.
    pub fn candidate_accuracy_bps(&self) -> Option<u16> {
        ratio_bps(self.candidate_correct, self.candidate_directional)
    }

    /// Exports a compact JSON summary.
    pub fn json_summary(&self) -> String {
        let mut out = String::from("{");
        out.push_str("\"total_samples\":");
        out.push_str(&self.total_samples.to_string());
        out.push(',');
        out.push_str("\"compared_samples\":");
        out.push_str(&self.compared_samples.to_string());
        out.push(',');
        out.push_str("\"state_disagreements\":");
        out.push_str(&self.state_disagreements.to_string());
        out.push(',');
        out.push_str("\"agreement_bps\":");
        push_optional_u16_json(&mut out, self.agreement_bps());
        out.push(',');
        out.push_str("\"production_accuracy_bps\":");
        push_optional_u16_json(&mut out, self.production_accuracy_bps());
        out.push(',');
        out.push_str("\"candidate_accuracy_bps\":");
        push_optional_u16_json(&mut out, self.candidate_accuracy_bps());
        out.push(',');
        out.push_str("\"candidate_only_correct\":");
        out.push_str(&self.candidate_only_correct.to_string());
        out.push(',');
        out.push_str("\"production_only_correct\":");
        out.push_str(&self.production_only_correct.to_string());
        out.push(',');
        out.push_str("\"average_confidence_delta_bps\":");
        out.push_str(&self.average_confidence_delta_bps.to_string());
        out.push('}');
        out
    }
}

/// Incremental recorder for shadow-mode signal comparisons.
#[derive(Debug, Clone)]
pub struct SignalShadowRecorder {
    config: SignalShadowComparisonConfig,
    samples: Vec<SignalShadowSample>,
}

impl SignalShadowRecorder {
    /// Creates an empty shadow recorder.
    pub fn new(config: SignalShadowComparisonConfig) -> Self {
        Self {
            config,
            samples: Vec::new(),
        }
    }

    /// Returns recorder configuration.
    pub const fn config(&self) -> SignalShadowComparisonConfig {
        self.config
    }

    /// Returns recorded samples.
    pub fn samples(&self) -> &[SignalShadowSample] {
        &self.samples
    }

    /// Records one shadow comparison sample.
    pub fn record(&mut self, sample: SignalShadowSample) {
        self.samples.push(sample);
    }

    /// Builds a comparison report.
    pub fn report(&self) -> SignalShadowComparisonReport {
        SignalShadowComparisonReport::from_samples(&self.samples, self.config)
    }

    /// Clears recorded samples.
    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

impl Default for SignalShadowRecorder {
    fn default() -> Self {
        Self::new(SignalShadowComparisonConfig::default())
    }
}

fn build_shadow_comparison_report(
    samples: &[SignalShadowSample],
    config: SignalShadowComparisonConfig,
) -> SignalShadowComparisonReport {
    let mut report = SignalShadowComparisonReport {
        config,
        total_samples: samples.len(),
        compared_samples: 0,
        state_agreements: 0,
        state_disagreements: 0,
        production_directional: 0,
        candidate_directional: 0,
        both_directional: 0,
        production_correct: 0,
        candidate_correct: 0,
        production_only_correct: 0,
        candidate_only_correct: 0,
        average_confidence_delta_bps: 0,
        samples: Vec::new(),
    };

    let mut confidence_delta_sum = 0_i64;
    for sample in samples {
        report.compared_samples += 1;
        confidence_delta_sum += i64::from(sample.confidence_delta_bps);
        if sample.disagreement {
            report.state_disagreements += 1;
        } else {
            report.state_agreements += 1;
        }

        let production_directional = signal_state_direction(sample.production.state).is_some();
        let candidate_directional = signal_state_direction(sample.candidate.state).is_some();
        report.production_directional += usize::from(production_directional);
        report.candidate_directional += usize::from(candidate_directional);
        report.both_directional += usize::from(production_directional && candidate_directional);

        if sample.production_correct == Some(true) {
            report.production_correct += 1;
        }
        if sample.candidate_correct == Some(true) {
            report.candidate_correct += 1;
        }
        if sample.production_correct == Some(true) && sample.candidate_correct == Some(false) {
            report.production_only_correct += 1;
        }
        if sample.candidate_correct == Some(true) && sample.production_correct == Some(false) {
            report.candidate_only_correct += 1;
        }
    }

    if report.compared_samples > 0 {
        report.average_confidence_delta_bps =
            (confidence_delta_sum / report.compared_samples as i64) as i32;
    }
    if config.store_samples {
        report.samples = samples.to_vec();
    }

    report
}

fn signal_state_direction(state: SignalState) -> Option<SignalMarkoutDirection> {
    match state {
        SignalState::LongBias => Some(SignalMarkoutDirection::Up),
        SignalState::ShortBias => Some(SignalMarkoutDirection::Down),
        SignalState::Neutral | SignalState::Blocked => None,
    }
}
