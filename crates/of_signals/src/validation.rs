use super::*;

/// Directional markout label used by signal validation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalMarkoutDirection {
    /// Future price moved up beyond the configured flat threshold.
    Up,
    /// Future price moved down beyond the configured flat threshold.
    Down,
    /// Future price change stayed inside the configured flat threshold.
    Flat,
}

impl SignalMarkoutDirection {
    /// Creates a markout direction from an integer price change.
    pub fn from_price_change(price_change: i64, flat_price_threshold: i64) -> Self {
        let threshold = flat_price_threshold.max(0);
        if price_change > threshold {
            Self::Up
        } else if price_change < -threshold {
            Self::Down
        } else {
            Self::Flat
        }
    }

    /// Returns the stable string representation of this markout direction.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Flat => "flat",
        }
    }
}

/// Borrowed analytics event used when validation needs timestamp checks.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct SignalReplayEvent<'a> {
    /// Analytics snapshot to feed into the signal.
    pub analytics: &'a AnalyticsSnapshot,
    /// Optional exchange timestamp for monotonic replay checks.
    pub ts_exchange_ns: Option<u64>,
}

impl<'a> SignalReplayEvent<'a> {
    /// Creates a replay event without timestamp metadata.
    pub const fn new(analytics: &'a AnalyticsSnapshot) -> Self {
        Self {
            analytics,
            ts_exchange_ns: None,
        }
    }

    /// Creates a replay event with exchange timestamp metadata.
    pub const fn with_ts_exchange_ns(
        analytics: &'a AnalyticsSnapshot,
        ts_exchange_ns: u64,
    ) -> Self {
        Self {
            analytics,
            ts_exchange_ns: Some(ts_exchange_ns),
        }
    }
}

/// Configuration for replay-based signal validation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalValidationConfig {
    /// Number of future events used for each markout label.
    pub markout_horizon_events: usize,
    /// Absolute price-change threshold below which a markout is `Flat`.
    pub flat_price_threshold: i64,
    /// Minimum confidence required before a directional prediction is scored.
    pub min_confidence_bps: u16,
    /// Whether individual validation samples should be retained in the report.
    pub store_samples: bool,
    /// Whether exchange timestamps should be checked for monotonic ordering.
    pub check_monotonic_timestamps: bool,
}

impl SignalValidationConfig {
    /// Creates validation config from an event horizon.
    pub const fn new(markout_horizon_events: usize) -> Self {
        Self {
            markout_horizon_events,
            flat_price_threshold: 0,
            min_confidence_bps: 0,
            store_samples: false,
            check_monotonic_timestamps: true,
        }
    }

    /// Returns config with a different flat price threshold.
    pub const fn with_flat_price_threshold(mut self, flat_price_threshold: i64) -> Self {
        self.flat_price_threshold = flat_price_threshold;
        self
    }

    /// Returns config with a minimum confidence threshold.
    pub const fn with_min_confidence_bps(mut self, min_confidence_bps: u16) -> Self {
        self.min_confidence_bps = min_confidence_bps;
        self
    }

    /// Returns config with sample retention changed.
    pub const fn with_store_samples(mut self, store_samples: bool) -> Self {
        self.store_samples = store_samples;
        self
    }

    /// Returns config with timestamp checking changed.
    pub const fn with_check_monotonic_timestamps(
        mut self,
        check_monotonic_timestamps: bool,
    ) -> Self {
        self.check_monotonic_timestamps = check_monotonic_timestamps;
        self
    }

    fn normalized_horizon(self) -> usize {
        self.markout_horizon_events.max(1)
    }
}

impl Default for SignalValidationConfig {
    fn default() -> Self {
        Self::new(1)
    }
}

/// Warning emitted by the replay validation harness.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalValidationWarning {
    /// Replay input was empty.
    EmptyInput,
    /// Configured markout horizon was zero and was treated as one event.
    ZeroMarkoutHorizon,
    /// A future markout could not be computed for this event.
    MissingMarkout {
        /// Event index without enough future observations.
        event_index: usize,
        /// Requested future horizon in events.
        requested_horizon_events: usize,
    },
    /// Exchange timestamps moved backward during replay.
    NonMonotonicTimestamp {
        /// Event index where non-monotonic time was detected.
        event_index: usize,
        /// Previous exchange timestamp.
        previous_ts_exchange_ns: u64,
        /// Current exchange timestamp.
        current_ts_exchange_ns: u64,
    },
}

/// One scored replay sample.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SignalValidationSample {
    /// Event index where the signal was evaluated.
    pub event_index: usize,
    /// Future event index used for the markout label.
    pub markout_event_index: usize,
    /// Snapshot emitted at `event_index`.
    pub snapshot: SignalSnapshot,
    /// Price observed at `event_index`.
    pub entry_price: i64,
    /// Price observed at `markout_event_index`.
    pub markout_price: i64,
    /// `markout_price - entry_price`.
    pub price_change: i64,
    /// Future markout label.
    pub markout_direction: SignalMarkoutDirection,
    /// Direction implied by the signal snapshot, if directional and confident enough.
    pub predicted_direction: Option<SignalMarkoutDirection>,
    /// Whether the directional prediction matched the markout label.
    pub correct: Option<bool>,
}

/// Summary report produced by replay-based signal validation.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SignalValidationReport {
    /// Module id observed from the first snapshot, when available.
    pub module_id: Option<&'static str>,
    /// Validation configuration used for this run.
    pub config: SignalValidationConfig,
    /// Number of analytics events evaluated.
    pub evaluated_events: usize,
    /// Number of events with markout labels.
    pub labeled_events: usize,
    /// Number of events without enough future data for a markout.
    pub missing_markouts: usize,
    /// Number of directional predictions scored.
    pub directional_predictions: usize,
    /// Number of long-bias predictions.
    pub long_predictions: usize,
    /// Number of short-bias predictions.
    pub short_predictions: usize,
    /// Number of neutral snapshots.
    pub neutral_predictions: usize,
    /// Number of blocked snapshots.
    pub blocked_predictions: usize,
    /// Number of directional predictions matching the markout label.
    pub correct_directional: usize,
    /// Number of directional predictions not matching a non-flat markout label.
    pub incorrect_directional: usize,
    /// Number of labeled events whose markout was flat.
    pub flat_markouts: usize,
    /// Average confidence across evaluated snapshots.
    pub average_confidence_bps: u16,
    /// Retained samples, when enabled by config.
    pub samples: Vec<SignalValidationSample>,
    /// Validation warnings.
    pub warnings: Vec<SignalValidationWarning>,
}

impl SignalValidationReport {
    /// Returns directional accuracy in basis points.
    pub fn directional_accuracy_bps(&self) -> Option<u16> {
        if self.directional_predictions == 0 {
            return None;
        }
        Some(((self.correct_directional * 10_000) / self.directional_predictions) as u16)
    }

    /// Returns labeled-event coverage in basis points.
    pub fn label_coverage_bps(&self) -> Option<u16> {
        if self.evaluated_events == 0 {
            return None;
        }
        Some(((self.labeled_events * 10_000) / self.evaluated_events) as u16)
    }

    /// Returns `true` when the report contains warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Exports a compact JSON summary for Python and notebook workflows.
    pub fn json_summary(&self) -> String {
        let mut out = String::from("{");
        out.push_str("\"module_id\":");
        match self.module_id {
            Some(module_id) => push_json_string(&mut out, module_id),
            None => out.push_str("null"),
        }
        out.push(',');
        out.push_str("\"evaluated_events\":");
        out.push_str(&self.evaluated_events.to_string());
        out.push(',');
        out.push_str("\"labeled_events\":");
        out.push_str(&self.labeled_events.to_string());
        out.push(',');
        out.push_str("\"missing_markouts\":");
        out.push_str(&self.missing_markouts.to_string());
        out.push(',');
        out.push_str("\"directional_predictions\":");
        out.push_str(&self.directional_predictions.to_string());
        out.push(',');
        out.push_str("\"correct_directional\":");
        out.push_str(&self.correct_directional.to_string());
        out.push(',');
        out.push_str("\"incorrect_directional\":");
        out.push_str(&self.incorrect_directional.to_string());
        out.push(',');
        out.push_str("\"flat_markouts\":");
        out.push_str(&self.flat_markouts.to_string());
        out.push(',');
        out.push_str("\"average_confidence_bps\":");
        out.push_str(&self.average_confidence_bps.to_string());
        out.push(',');
        out.push_str("\"directional_accuracy_bps\":");
        match self.directional_accuracy_bps() {
            Some(value) => out.push_str(&value.to_string()),
            None => out.push_str("null"),
        }
        out.push(',');
        out.push_str("\"label_coverage_bps\":");
        match self.label_coverage_bps() {
            Some(value) => out.push_str(&value.to_string()),
            None => out.push_str("null"),
        }
        out.push(',');
        out.push_str("\"warnings\":");
        out.push_str(&self.warnings.len().to_string());
        out.push('}');
        out
    }

    /// Exports the complete replay-validation report as dependency-free JSON.
    ///
    /// This schema is intended for C, Python, Java, notebooks, and dashboards.
    /// It is separate from [`Self::json_summary`] so existing compact-summary
    /// consumers retain their exact serialized shape.
    pub fn json_report(&self) -> String {
        let mut out = String::from("{\"schema_version\":1,\"valid\":true,\"module_id\":");
        match self.module_id {
            Some(module_id) => push_json_string(&mut out, module_id),
            None => out.push_str("null"),
        }
        out.push_str(",\"config\":{");
        push_json_usize_field(
            &mut out,
            "markout_horizon_events",
            self.config.markout_horizon_events,
        );
        out.push(',');
        push_json_i64_field(
            &mut out,
            "flat_price_threshold",
            self.config.flat_price_threshold,
        );
        out.push(',');
        push_json_u16_field(
            &mut out,
            "min_confidence_bps",
            self.config.min_confidence_bps,
        );
        out.push(',');
        push_json_bool_field(&mut out, "store_samples", self.config.store_samples);
        out.push(',');
        push_json_bool_field(
            &mut out,
            "check_monotonic_timestamps",
            self.config.check_monotonic_timestamps,
        );
        out.push('}');
        out.push(',');
        push_json_usize_field(&mut out, "evaluated_events", self.evaluated_events);
        out.push(',');
        push_json_usize_field(&mut out, "labeled_events", self.labeled_events);
        out.push(',');
        push_json_usize_field(&mut out, "missing_markouts", self.missing_markouts);
        out.push(',');
        push_json_usize_field(
            &mut out,
            "directional_predictions",
            self.directional_predictions,
        );
        out.push(',');
        push_json_usize_field(&mut out, "long_predictions", self.long_predictions);
        out.push(',');
        push_json_usize_field(&mut out, "short_predictions", self.short_predictions);
        out.push(',');
        push_json_usize_field(&mut out, "neutral_predictions", self.neutral_predictions);
        out.push(',');
        push_json_usize_field(&mut out, "blocked_predictions", self.blocked_predictions);
        out.push(',');
        push_json_usize_field(&mut out, "correct_directional", self.correct_directional);
        out.push(',');
        push_json_usize_field(
            &mut out,
            "incorrect_directional",
            self.incorrect_directional,
        );
        out.push(',');
        push_json_usize_field(&mut out, "flat_markouts", self.flat_markouts);
        out.push(',');
        push_json_u16_field(
            &mut out,
            "average_confidence_bps",
            self.average_confidence_bps,
        );
        out.push_str(",\"directional_accuracy_bps\":");
        push_optional_u16_json(&mut out, self.directional_accuracy_bps());
        out.push_str(",\"label_coverage_bps\":");
        push_optional_u16_json(&mut out, self.label_coverage_bps());
        out.push(',');
        push_json_usize_field(&mut out, "warning_count", self.warnings.len());
        out.push_str(",\"samples\":[");
        for (index, sample) in self.samples.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_validation_sample_json(&mut out, sample);
        }
        out.push_str("],\"warnings\":[");
        for (index, warning) in self.warnings.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_validation_warning_json(&mut out, warning);
        }
        out.push_str("]}");
        out
    }
}

/// Replay validation harness for signal modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalValidationHarness {
    config: SignalValidationConfig,
}

impl SignalValidationHarness {
    /// Creates a validation harness with explicit config.
    pub const fn new(config: SignalValidationConfig) -> Self {
        Self { config }
    }

    /// Creates a validation harness with default config.
    pub const fn default_config() -> Self {
        Self::new(SignalValidationConfig::new(1))
    }

    /// Returns the harness configuration.
    pub const fn config(&self) -> SignalValidationConfig {
        self.config
    }

    /// Validates a signal over ordered analytics snapshots.
    pub fn validate_signal<S: SignalModule>(
        &self,
        signal: &mut S,
        events: &[AnalyticsSnapshot],
    ) -> SignalValidationReport {
        validate_signal_replay(signal, events, self.config)
    }

    /// Validates a signal over ordered replay events with optional timestamps.
    pub fn validate_events<S: SignalModule>(
        &self,
        signal: &mut S,
        events: &[SignalReplayEvent<'_>],
    ) -> SignalValidationReport {
        validate_signal_replay_events(signal, events, self.config)
    }
}

impl Default for SignalValidationHarness {
    fn default() -> Self {
        Self::new(SignalValidationConfig::default())
    }
}

/// Validates a signal by replaying ordered analytics snapshots.
///
/// The harness feeds each current snapshot into the signal, captures the signal
/// output, and only then computes the future markout label for scoring. This
/// sequencing keeps label generation separate from signal evaluation and helps
/// avoid lookahead bias in validation code.
pub fn validate_signal_replay<S: SignalModule>(
    signal: &mut S,
    events: &[AnalyticsSnapshot],
    config: SignalValidationConfig,
) -> SignalValidationReport {
    let mut builder = SignalValidationReportBuilder::new(config);
    builder.validate(signal, events);
    builder.finish()
}

/// Validates a signal by replaying ordered analytics events with optional timestamps.
pub fn validate_signal_replay_events<S: SignalModule>(
    signal: &mut S,
    events: &[SignalReplayEvent<'_>],
    config: SignalValidationConfig,
) -> SignalValidationReport {
    let mut builder = SignalValidationReportBuilder::new(config);
    builder.validate_events(signal, events);
    builder.finish()
}

#[derive(Debug)]
struct SignalValidationReportBuilder {
    report: SignalValidationReport,
    confidence_sum: u64,
}

impl SignalValidationReportBuilder {
    fn new(config: SignalValidationConfig) -> Self {
        let mut warnings = Vec::new();
        if config.markout_horizon_events == 0 {
            warnings.push(SignalValidationWarning::ZeroMarkoutHorizon);
        }

        Self {
            report: SignalValidationReport {
                module_id: None,
                config,
                evaluated_events: 0,
                labeled_events: 0,
                missing_markouts: 0,
                directional_predictions: 0,
                long_predictions: 0,
                short_predictions: 0,
                neutral_predictions: 0,
                blocked_predictions: 0,
                correct_directional: 0,
                incorrect_directional: 0,
                flat_markouts: 0,
                average_confidence_bps: 0,
                samples: Vec::new(),
                warnings,
            },
            confidence_sum: 0,
        }
    }

    fn validate<S: SignalModule>(&mut self, signal: &mut S, events: &[AnalyticsSnapshot]) {
        if events.is_empty() {
            self.report
                .warnings
                .push(SignalValidationWarning::EmptyInput);
            return;
        }

        let horizon = self.report.config.normalized_horizon();
        let mut previous_ts = None;

        for (event_index, event) in events.iter().enumerate() {
            self.check_timestamp(event_index, None, &mut previous_ts);

            signal.on_analytics(event);
            let snapshot = signal.snapshot();
            self.record_snapshot_counts(&snapshot);

            let Some(markout_event_index) = event_index.checked_add(horizon) else {
                self.record_missing_markout(event_index, horizon);
                continue;
            };
            let Some(markout_event) = events.get(markout_event_index) else {
                self.record_missing_markout(event_index, horizon);
                continue;
            };

            let sample = self.score_sample(
                event_index,
                markout_event_index,
                event,
                markout_event,
                snapshot,
            );
            if self.report.config.store_samples {
                self.report.samples.push(sample);
            }
        }
    }

    fn validate_events<S: SignalModule>(
        &mut self,
        signal: &mut S,
        events: &[SignalReplayEvent<'_>],
    ) {
        if events.is_empty() {
            self.report
                .warnings
                .push(SignalValidationWarning::EmptyInput);
            return;
        }

        let horizon = self.report.config.normalized_horizon();
        let mut previous_ts = None;

        for (event_index, event) in events.iter().enumerate() {
            self.check_timestamp(event_index, event.ts_exchange_ns, &mut previous_ts);

            signal.on_analytics(event.analytics);
            let snapshot = signal.snapshot();
            self.record_snapshot_counts(&snapshot);

            let Some(markout_event_index) = event_index.checked_add(horizon) else {
                self.record_missing_markout(event_index, horizon);
                continue;
            };
            let Some(markout_event) = events.get(markout_event_index) else {
                self.record_missing_markout(event_index, horizon);
                continue;
            };

            let sample = self.score_sample(
                event_index,
                markout_event_index,
                event.analytics,
                markout_event.analytics,
                snapshot,
            );
            if self.report.config.store_samples {
                self.report.samples.push(sample);
            }
        }
    }

    fn check_timestamp(
        &mut self,
        event_index: usize,
        ts_exchange_ns: Option<u64>,
        previous_ts: &mut Option<u64>,
    ) {
        if !self.report.config.check_monotonic_timestamps {
            return;
        }
        let Some(ts_exchange_ns) = ts_exchange_ns else {
            return;
        };
        if let Some(previous) = *previous_ts {
            if ts_exchange_ns < previous {
                self.report
                    .warnings
                    .push(SignalValidationWarning::NonMonotonicTimestamp {
                        event_index,
                        previous_ts_exchange_ns: previous,
                        current_ts_exchange_ns: ts_exchange_ns,
                    });
            }
        }
        *previous_ts = Some(ts_exchange_ns);
    }

    fn record_snapshot_counts(&mut self, snapshot: &SignalSnapshot) {
        if self.report.module_id.is_none() {
            self.report.module_id = Some(snapshot.module_id);
        }
        self.report.evaluated_events += 1;
        self.confidence_sum += u64::from(snapshot.confidence_bps);

        match snapshot.state {
            SignalState::LongBias => self.report.long_predictions += 1,
            SignalState::ShortBias => self.report.short_predictions += 1,
            SignalState::Neutral => self.report.neutral_predictions += 1,
            SignalState::Blocked => self.report.blocked_predictions += 1,
        }
    }

    fn record_missing_markout(&mut self, event_index: usize, horizon: usize) {
        self.report.missing_markouts += 1;
        self.report
            .warnings
            .push(SignalValidationWarning::MissingMarkout {
                event_index,
                requested_horizon_events: horizon,
            });
    }

    fn score_sample(
        &mut self,
        event_index: usize,
        markout_event_index: usize,
        event: &AnalyticsSnapshot,
        markout_event: &AnalyticsSnapshot,
        snapshot: SignalSnapshot,
    ) -> SignalValidationSample {
        let price_change = markout_event.last_price - event.last_price;
        let markout_direction = SignalMarkoutDirection::from_price_change(
            price_change,
            self.report.config.flat_price_threshold,
        );
        let predicted_direction = predicted_direction(&snapshot, self.report.config);
        let correct = score_direction(predicted_direction, markout_direction);

        self.report.labeled_events += 1;
        if markout_direction == SignalMarkoutDirection::Flat {
            self.report.flat_markouts += 1;
        }

        if predicted_direction.is_some() {
            self.report.directional_predictions += 1;
            match correct {
                Some(true) => self.report.correct_directional += 1,
                Some(false) => self.report.incorrect_directional += 1,
                None => {}
            }
        }

        SignalValidationSample {
            event_index,
            markout_event_index,
            snapshot,
            entry_price: event.last_price,
            markout_price: markout_event.last_price,
            price_change,
            markout_direction,
            predicted_direction,
            correct,
        }
    }

    fn finish(mut self) -> SignalValidationReport {
        if self.report.evaluated_events > 0 {
            self.report.average_confidence_bps =
                (self.confidence_sum / self.report.evaluated_events as u64) as u16;
        }
        self.report
    }
}

fn predicted_direction(
    snapshot: &SignalSnapshot,
    config: SignalValidationConfig,
) -> Option<SignalMarkoutDirection> {
    if snapshot.confidence_bps < config.min_confidence_bps {
        return None;
    }

    match snapshot.state {
        SignalState::LongBias => Some(SignalMarkoutDirection::Up),
        SignalState::ShortBias => Some(SignalMarkoutDirection::Down),
        SignalState::Neutral | SignalState::Blocked => None,
    }
}

pub(crate) fn score_direction(
    predicted: Option<SignalMarkoutDirection>,
    markout: SignalMarkoutDirection,
) -> Option<bool> {
    let predicted = predicted?;
    if markout == SignalMarkoutDirection::Flat {
        None
    } else {
        Some(predicted == markout)
    }
}
