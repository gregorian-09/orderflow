use super::*;

/// Configuration for confidence calibration reports.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalCalibrationConfig {
    /// Width of each confidence bin in basis points.
    pub bin_width_bps: u16,
    /// Minimum samples required before a bin contributes to ECE.
    pub min_samples_per_bin: usize,
    /// Absolute ECE delta that marks drift as significant.
    pub drift_alert_threshold_bps: u16,
}

impl SignalCalibrationConfig {
    /// Creates calibration config from a bin width in basis points.
    pub const fn new(bin_width_bps: u16) -> Self {
        Self {
            bin_width_bps,
            min_samples_per_bin: 1,
            drift_alert_threshold_bps: 500,
        }
    }

    /// Returns config with a different minimum sample count per bin.
    pub const fn with_min_samples_per_bin(mut self, min_samples_per_bin: usize) -> Self {
        self.min_samples_per_bin = min_samples_per_bin;
        self
    }

    /// Returns config with a different drift alert threshold.
    pub const fn with_drift_alert_threshold_bps(mut self, drift_alert_threshold_bps: u16) -> Self {
        self.drift_alert_threshold_bps = drift_alert_threshold_bps;
        self
    }

    fn normalized_bin_width(self) -> u16 {
        self.bin_width_bps.clamp(1, 10_000)
    }
}

impl Default for SignalCalibrationConfig {
    fn default() -> Self {
        Self::new(1_000)
    }
}

/// Maps raw signal confidence into calibrated confidence.
pub trait SignalConfidenceCalibrator {
    /// Returns calibrated confidence in basis points.
    fn calibrate_confidence_bps(&self, raw_confidence_bps: u16) -> u16;
}

/// Identity confidence calibrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IdentitySignalCalibrator;

impl SignalConfidenceCalibrator for IdentitySignalCalibrator {
    fn calibrate_confidence_bps(&self, raw_confidence_bps: u16) -> u16 {
        raw_confidence_bps.min(10_000)
    }
}

/// One point in a piecewise-linear confidence calibration curve.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalCalibrationPoint {
    /// Raw confidence in basis points.
    pub raw_confidence_bps: u16,
    /// Calibrated confidence in basis points.
    pub calibrated_confidence_bps: u16,
}

impl SignalCalibrationPoint {
    /// Creates a calibration point.
    pub const fn new(raw_confidence_bps: u16, calibrated_confidence_bps: u16) -> Self {
        Self {
            raw_confidence_bps,
            calibrated_confidence_bps,
        }
    }
}

/// Piecewise-linear confidence calibration curve.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SignalCalibrationCurve {
    points: Vec<SignalCalibrationPoint>,
}

impl SignalCalibrationCurve {
    /// Creates a curve from calibration points sorted by raw confidence.
    pub fn new(mut points: Vec<SignalCalibrationPoint>) -> Self {
        for point in &mut points {
            point.raw_confidence_bps = point.raw_confidence_bps.min(10_000);
            point.calibrated_confidence_bps = point.calibrated_confidence_bps.min(10_000);
        }
        points.sort_by_key(|point| point.raw_confidence_bps);
        Self { points }
    }

    /// Returns calibration points.
    pub fn points(&self) -> &[SignalCalibrationPoint] {
        &self.points
    }
}

impl SignalConfidenceCalibrator for SignalCalibrationCurve {
    fn calibrate_confidence_bps(&self, raw_confidence_bps: u16) -> u16 {
        let raw = raw_confidence_bps.min(10_000);
        let Some(first) = self.points.first() else {
            return raw;
        };
        if raw <= first.raw_confidence_bps {
            return first.calibrated_confidence_bps;
        }

        for pair in self.points.windows(2) {
            let left = pair[0];
            let right = pair[1];
            if raw <= right.raw_confidence_bps {
                let span = right
                    .raw_confidence_bps
                    .saturating_sub(left.raw_confidence_bps);
                if span == 0 {
                    return right.calibrated_confidence_bps;
                }
                let offset = raw.saturating_sub(left.raw_confidence_bps);
                let calibrated_span = i32::from(right.calibrated_confidence_bps)
                    - i32::from(left.calibrated_confidence_bps);
                let interpolated = i32::from(left.calibrated_confidence_bps)
                    + (calibrated_span * i32::from(offset)) / i32::from(span);
                return interpolated.clamp(0, 10_000) as u16;
            }
        }

        self.points
            .last()
            .map_or(raw, |point| point.calibrated_confidence_bps)
    }
}

/// One realized signal outcome used for calibration and drift reports.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct SignalOutcomeRecord {
    /// Stable signal module id.
    pub module_id: &'static str,
    /// Signal state that produced the prediction.
    pub state: SignalState,
    /// Raw signal confidence in basis points.
    pub confidence_bps: u16,
    /// Calibrated confidence in basis points.
    pub calibrated_confidence_bps: u16,
    /// Direction implied by the signal, if scored.
    pub predicted_direction: Option<SignalMarkoutDirection>,
    /// Realized future markout direction.
    pub markout_direction: SignalMarkoutDirection,
    /// Whether the scored directional prediction was correct.
    pub correct: Option<bool>,
    /// Optional market-regime label.
    pub regime: Option<String>,
}

impl SignalOutcomeRecord {
    /// Creates a signal outcome record.
    ///
    /// The raw confidence is also used as the calibrated confidence initially.
    /// Apply [`SignalOutcomeRecord::with_calibrated_confidence_bps`] when a
    /// host has a calibrated value.
    pub fn new(
        module_id: &'static str,
        state: SignalState,
        confidence_bps: u16,
        predicted_direction: Option<SignalMarkoutDirection>,
        markout_direction: SignalMarkoutDirection,
        correct: Option<bool>,
    ) -> Self {
        let confidence_bps = confidence_bps.min(10_000);
        Self {
            module_id,
            state,
            confidence_bps,
            calibrated_confidence_bps: confidence_bps,
            predicted_direction,
            markout_direction,
            correct,
            regime: None,
        }
    }

    /// Creates an outcome record from a validation sample.
    pub fn from_validation_sample(sample: &SignalValidationSample) -> Self {
        Self::new(
            sample.snapshot.module_id,
            sample.snapshot.state,
            sample.snapshot.confidence_bps,
            sample.predicted_direction,
            sample.markout_direction,
            sample.correct,
        )
    }

    /// Returns this record with a calibrated confidence value.
    pub fn with_calibrated_confidence_bps(mut self, calibrated_confidence_bps: u16) -> Self {
        self.calibrated_confidence_bps = calibrated_confidence_bps.min(10_000);
        self
    }

    /// Returns this record with a market-regime label.
    pub fn with_regime(mut self, regime: impl Into<String>) -> Self {
        self.regime = Some(regime.into());
        self
    }
}

/// One confidence bin in a calibration report.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCalibrationBin {
    /// Inclusive lower confidence bound.
    pub lower_confidence_bps: u16,
    /// Inclusive upper confidence bound.
    pub upper_confidence_bps: u16,
    /// Number of scored records in the bin.
    pub samples: usize,
    /// Number of correct scored records in the bin.
    pub correct: usize,
    /// Average calibrated confidence in the bin.
    pub average_confidence_bps: u16,
    /// Empirical accuracy in the bin.
    pub accuracy_bps: Option<u16>,
    /// Absolute calibration gap in basis points.
    pub calibration_error_bps: Option<u16>,
}

/// Per-regime confidence and accuracy summary.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalRegimeSummary {
    /// Regime label.
    pub regime: String,
    /// Number of scored records in this regime.
    pub samples: usize,
    /// Number of correct scored records in this regime.
    pub correct: usize,
    /// Average calibrated confidence in this regime.
    pub average_confidence_bps: u16,
    /// Empirical accuracy in this regime.
    pub accuracy_bps: Option<u16>,
}

/// Calibration report for realized signal outcomes.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCalibrationReport {
    /// Calibration configuration used for the report.
    pub config: SignalCalibrationConfig,
    /// Number of records inspected.
    pub total_records: usize,
    /// Number of records with scored directional outcomes.
    pub scored_records: usize,
    /// Number of records not included in calibration scoring.
    pub ignored_records: usize,
    /// Number of correct scored records.
    pub correct_records: usize,
    /// Expected calibration error in basis points.
    pub expected_calibration_error_bps: u16,
    /// Confidence-bin summaries.
    pub bins: Vec<SignalCalibrationBin>,
    /// Per-regime summaries.
    pub regimes: Vec<SignalRegimeSummary>,
}

impl SignalCalibrationReport {
    /// Builds a calibration report from outcome records.
    pub fn from_records(records: &[SignalOutcomeRecord], config: SignalCalibrationConfig) -> Self {
        build_calibration_report(records, config)
    }

    /// Builds a calibration report from retained validation samples.
    pub fn from_validation_report(
        report: &SignalValidationReport,
        config: SignalCalibrationConfig,
    ) -> Self {
        let records: Vec<_> = report
            .samples
            .iter()
            .map(SignalOutcomeRecord::from_validation_sample)
            .collect();
        Self::from_records(&records, config)
    }

    /// Returns scored accuracy in basis points.
    pub fn accuracy_bps(&self) -> Option<u16> {
        ratio_bps(self.correct_records, self.scored_records)
    }

    /// Exports a compact JSON summary.
    pub fn json_summary(&self) -> String {
        let mut out = String::from("{");
        out.push_str("\"total_records\":");
        out.push_str(&self.total_records.to_string());
        out.push(',');
        out.push_str("\"scored_records\":");
        out.push_str(&self.scored_records.to_string());
        out.push(',');
        out.push_str("\"ignored_records\":");
        out.push_str(&self.ignored_records.to_string());
        out.push(',');
        out.push_str("\"accuracy_bps\":");
        push_optional_u16_json(&mut out, self.accuracy_bps());
        out.push(',');
        out.push_str("\"expected_calibration_error_bps\":");
        out.push_str(&self.expected_calibration_error_bps.to_string());
        out.push(',');
        out.push_str("\"bins\":");
        out.push_str(&self.bins.len().to_string());
        out.push(',');
        out.push_str("\"regimes\":");
        out.push_str(&self.regimes.len().to_string());
        out.push('}');
        out
    }
}

/// One confidence-bin drift comparison.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCalibrationBinDrift {
    /// Inclusive lower confidence bound.
    pub lower_confidence_bps: u16,
    /// Inclusive upper confidence bound.
    pub upper_confidence_bps: u16,
    /// Baseline bin sample count.
    pub baseline_samples: usize,
    /// Current bin sample count.
    pub current_samples: usize,
    /// Baseline bin accuracy.
    pub baseline_accuracy_bps: Option<u16>,
    /// Current bin accuracy.
    pub current_accuracy_bps: Option<u16>,
    /// Current minus baseline accuracy, when both exist.
    pub accuracy_delta_bps: Option<i32>,
}

/// Drift report comparing current calibration with a baseline.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCalibrationDriftReport {
    /// Baseline expected calibration error in basis points.
    pub baseline_ece_bps: u16,
    /// Current expected calibration error in basis points.
    pub current_ece_bps: u16,
    /// Current minus baseline ECE.
    pub ece_delta_bps: i32,
    /// Whether absolute ECE drift exceeded the configured threshold.
    pub significant: bool,
    /// Per-bin drift summaries.
    pub bin_drifts: Vec<SignalCalibrationBinDrift>,
}

impl SignalCalibrationDriftReport {
    /// Builds a drift report from baseline and current calibration reports.
    pub fn compare(
        baseline: &SignalCalibrationReport,
        current: &SignalCalibrationReport,
        drift_alert_threshold_bps: u16,
    ) -> Self {
        let ece_delta_bps = i32::from(current.expected_calibration_error_bps)
            - i32::from(baseline.expected_calibration_error_bps);
        let significant = ece_delta_bps.unsigned_abs() >= u32::from(drift_alert_threshold_bps);
        let mut bin_drifts = Vec::new();

        for current_bin in &current.bins {
            let baseline_bin = baseline.bins.iter().find(|bin| {
                bin.lower_confidence_bps == current_bin.lower_confidence_bps
                    && bin.upper_confidence_bps == current_bin.upper_confidence_bps
            });
            let baseline_accuracy_bps = baseline_bin.and_then(|bin| bin.accuracy_bps);
            let current_accuracy_bps = current_bin.accuracy_bps;
            let accuracy_delta_bps = match (baseline_accuracy_bps, current_accuracy_bps) {
                (Some(baseline), Some(current)) => Some(i32::from(current) - i32::from(baseline)),
                _ => None,
            };

            bin_drifts.push(SignalCalibrationBinDrift {
                lower_confidence_bps: current_bin.lower_confidence_bps,
                upper_confidence_bps: current_bin.upper_confidence_bps,
                baseline_samples: baseline_bin.map_or(0, |bin| bin.samples),
                current_samples: current_bin.samples,
                baseline_accuracy_bps,
                current_accuracy_bps,
                accuracy_delta_bps,
            });
        }

        Self {
            baseline_ece_bps: baseline.expected_calibration_error_bps,
            current_ece_bps: current.expected_calibration_error_bps,
            ece_delta_bps,
            significant,
            bin_drifts,
        }
    }
}

/// Incremental outcome tracker for calibration and drift reporting.
#[derive(Debug, Clone)]
pub struct SignalOutcomeTracker {
    config: SignalCalibrationConfig,
    records: Vec<SignalOutcomeRecord>,
}

impl SignalOutcomeTracker {
    /// Creates an empty outcome tracker.
    pub fn new(config: SignalCalibrationConfig) -> Self {
        Self {
            config,
            records: Vec::new(),
        }
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> SignalCalibrationConfig {
        self.config
    }

    /// Returns tracked outcome records.
    pub fn records(&self) -> &[SignalOutcomeRecord] {
        &self.records
    }

    /// Records one realized signal outcome.
    pub fn record(&mut self, record: SignalOutcomeRecord) {
        self.records.push(record);
    }

    /// Records retained samples from a validation report.
    pub fn extend_validation_report(&mut self, report: &SignalValidationReport) {
        self.records.extend(
            report
                .samples
                .iter()
                .map(SignalOutcomeRecord::from_validation_sample),
        );
    }

    /// Builds a calibration report from tracked outcomes.
    pub fn calibration_report(&self) -> SignalCalibrationReport {
        SignalCalibrationReport::from_records(&self.records, self.config)
    }

    /// Compares tracked outcomes against a baseline report.
    pub fn drift_report(&self, baseline: &SignalCalibrationReport) -> SignalCalibrationDriftReport {
        let current = self.calibration_report();
        SignalCalibrationDriftReport::compare(
            baseline,
            &current,
            self.config.drift_alert_threshold_bps,
        )
    }

    /// Clears all tracked outcomes.
    pub fn clear(&mut self) {
        self.records.clear();
    }
}

impl Default for SignalOutcomeTracker {
    fn default() -> Self {
        Self::new(SignalCalibrationConfig::default())
    }
}

#[derive(Debug, Clone)]
struct CalibrationBinAccumulator {
    lower_confidence_bps: u16,
    upper_confidence_bps: u16,
    samples: usize,
    correct: usize,
    confidence_sum: u64,
}

#[derive(Debug, Clone)]
struct RegimeAccumulator {
    regime: String,
    samples: usize,
    correct: usize,
    confidence_sum: u64,
}

fn build_calibration_report(
    records: &[SignalOutcomeRecord],
    config: SignalCalibrationConfig,
) -> SignalCalibrationReport {
    let bin_width = config.normalized_bin_width();
    let mut bins = build_bin_accumulators(bin_width);
    let mut regimes: Vec<RegimeAccumulator> = Vec::new();
    let mut scored_records = 0_usize;
    let mut correct_records = 0_usize;

    for record in records {
        let Some(correct) = record.correct else {
            continue;
        };
        scored_records += 1;
        if correct {
            correct_records += 1;
        }

        let confidence = record.calibrated_confidence_bps.min(10_000);
        let bin_index = (confidence / bin_width) as usize;
        let bin_index = bin_index.min(bins.len().saturating_sub(1));
        let bin = &mut bins[bin_index];
        bin.samples += 1;
        bin.correct += usize::from(correct);
        bin.confidence_sum += u64::from(confidence);

        let regime = record.regime.as_deref().unwrap_or("default");
        record_regime(&mut regimes, regime, confidence, correct);
    }

    let public_bins: Vec<_> = bins
        .into_iter()
        .map(|bin| {
            let average_confidence_bps = average_bps(bin.confidence_sum, bin.samples);
            let accuracy_bps = ratio_bps(bin.correct, bin.samples);
            let calibration_error_bps =
                accuracy_bps.map(|accuracy| abs_diff_bps(average_confidence_bps, accuracy));
            SignalCalibrationBin {
                lower_confidence_bps: bin.lower_confidence_bps,
                upper_confidence_bps: bin.upper_confidence_bps,
                samples: bin.samples,
                correct: bin.correct,
                average_confidence_bps,
                accuracy_bps,
                calibration_error_bps,
            }
        })
        .collect();

    let expected_calibration_error_bps =
        expected_calibration_error_bps(&public_bins, scored_records, config.min_samples_per_bin);
    let regime_summaries = regimes
        .into_iter()
        .map(|regime| SignalRegimeSummary {
            regime: regime.regime,
            samples: regime.samples,
            correct: regime.correct,
            average_confidence_bps: average_bps(regime.confidence_sum, regime.samples),
            accuracy_bps: ratio_bps(regime.correct, regime.samples),
        })
        .collect();

    SignalCalibrationReport {
        config,
        total_records: records.len(),
        scored_records,
        ignored_records: records.len().saturating_sub(scored_records),
        correct_records,
        expected_calibration_error_bps,
        bins: public_bins,
        regimes: regime_summaries,
    }
}

fn build_bin_accumulators(bin_width: u16) -> Vec<CalibrationBinAccumulator> {
    let mut bins = Vec::new();
    let mut lower = 0_u16;
    loop {
        let upper = lower
            .saturating_add(bin_width.saturating_sub(1))
            .min(10_000);
        bins.push(CalibrationBinAccumulator {
            lower_confidence_bps: lower,
            upper_confidence_bps: upper,
            samples: 0,
            correct: 0,
            confidence_sum: 0,
        });
        if upper >= 10_000 {
            break;
        }
        lower = upper.saturating_add(1);
    }
    bins
}

fn record_regime(
    regimes: &mut Vec<RegimeAccumulator>,
    regime: &str,
    confidence_bps: u16,
    correct: bool,
) {
    if let Some(existing) = regimes
        .iter_mut()
        .find(|existing| existing.regime == regime)
    {
        existing.samples += 1;
        existing.correct += usize::from(correct);
        existing.confidence_sum += u64::from(confidence_bps);
        return;
    }

    regimes.push(RegimeAccumulator {
        regime: regime.to_string(),
        samples: 1,
        correct: usize::from(correct),
        confidence_sum: u64::from(confidence_bps),
    });
}

fn expected_calibration_error_bps(
    bins: &[SignalCalibrationBin],
    scored_records: usize,
    min_samples_per_bin: usize,
) -> u16 {
    if scored_records == 0 {
        return 0;
    }

    let mut weighted_error = 0_u64;
    let mut weighted_samples = 0_usize;
    for bin in bins {
        if bin.samples < min_samples_per_bin {
            continue;
        }
        let Some(error) = bin.calibration_error_bps else {
            continue;
        };
        weighted_error += u64::from(error) * bin.samples as u64;
        weighted_samples += bin.samples;
    }

    if weighted_samples == 0 {
        0
    } else {
        (weighted_error / weighted_samples as u64) as u16
    }
}

pub(crate) fn ratio_bps(numerator: usize, denominator: usize) -> Option<u16> {
    let ratio = (numerator as u128)
        .saturating_mul(10_000)
        .checked_div(denominator as u128)?;
    Some(ratio.min(10_000) as u16)
}

pub(crate) fn average_bps(sum: u64, samples: usize) -> u16 {
    if samples == 0 {
        0
    } else {
        (sum / samples as u64) as u16
    }
}

fn abs_diff_bps(left: u16, right: u16) -> u16 {
    left.abs_diff(right)
}

pub(crate) fn push_optional_u16_json(out: &mut String, value: Option<u16>) {
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}
