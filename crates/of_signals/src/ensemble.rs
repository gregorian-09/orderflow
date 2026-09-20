use super::*;

/// Rule used to select an ensemble signal state from child votes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SignalEnsembleDecisionRule {
    /// Select the side with more directional votes.
    #[default]
    Majority,
    /// Select a side only when it reaches the configured directional vote count.
    Quorum {
        /// Minimum directional votes required to select a side.
        min_votes: usize,
    },
    /// Select a side only when weighted confidence reaches the configured score.
    Weighted {
        /// Minimum weighted score in basis-point units.
        min_score_bps: u32,
    },
}

/// Policy used when long and short ensemble evidence conflicts.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SignalEnsembleConflictPolicy {
    /// Emit neutral on equal long/short evidence.
    #[default]
    NeutralOnTie,
    /// Prefer the side with the highest single child confidence.
    HighestConfidence,
    /// Prefer the side with the highest weighted confidence score.
    HighestWeightedScore,
}

/// Policy used when an ensemble child marks itself as a veto.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SignalEnsembleVetoPolicy {
    /// Ignore veto flags when evaluating the ensemble.
    Ignore,
    /// Emit neutral when any child veto is present.
    NeutralOnVeto,
    /// Emit blocked when any child veto is present.
    #[default]
    BlockOnVeto,
}

/// Configuration for evaluating an ensemble of signal snapshots.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalEnsemblePolicy {
    /// Rule used to select a directional state.
    pub rule: SignalEnsembleDecisionRule,
    /// Tie/conflict resolution policy.
    pub conflict_policy: SignalEnsembleConflictPolicy,
    /// Veto handling policy.
    pub veto_policy: SignalEnsembleVetoPolicy,
    /// Minimum child confidence before a vote contributes to direction.
    pub min_confidence_bps: u16,
}

impl SignalEnsemblePolicy {
    /// Creates a majority-vote ensemble policy.
    pub const fn majority() -> Self {
        Self {
            rule: SignalEnsembleDecisionRule::Majority,
            conflict_policy: SignalEnsembleConflictPolicy::NeutralOnTie,
            veto_policy: SignalEnsembleVetoPolicy::BlockOnVeto,
            min_confidence_bps: 0,
        }
    }

    /// Creates a quorum ensemble policy.
    pub const fn quorum(min_votes: usize) -> Self {
        Self {
            rule: SignalEnsembleDecisionRule::Quorum { min_votes },
            ..Self::majority()
        }
    }

    /// Creates a weighted-score ensemble policy.
    pub const fn weighted(min_score_bps: u32) -> Self {
        Self {
            rule: SignalEnsembleDecisionRule::Weighted { min_score_bps },
            ..Self::majority()
        }
    }

    /// Returns this policy with a different conflict policy.
    pub const fn with_conflict_policy(
        mut self,
        conflict_policy: SignalEnsembleConflictPolicy,
    ) -> Self {
        self.conflict_policy = conflict_policy;
        self
    }

    /// Returns this policy with a different veto policy.
    pub const fn with_veto_policy(mut self, veto_policy: SignalEnsembleVetoPolicy) -> Self {
        self.veto_policy = veto_policy;
        self
    }

    /// Returns this policy with a minimum child confidence.
    pub const fn with_min_confidence_bps(mut self, min_confidence_bps: u16) -> Self {
        self.min_confidence_bps = min_confidence_bps;
        self
    }
}

impl Default for SignalEnsemblePolicy {
    fn default() -> Self {
        Self::majority()
    }
}

/// One child vote supplied to the ensemble evaluator.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalEnsembleVote {
    /// Stable child signal module id.
    pub module_id: &'static str,
    /// Child signal state.
    pub state: SignalState,
    /// Child confidence in basis points.
    pub confidence_bps: u16,
    /// Child weight in basis points.
    pub weight_bps: u16,
    /// Child quality flags.
    pub quality_flags: u32,
    /// Whether this child should veto the ensemble under veto-aware policies.
    pub veto: bool,
}

impl SignalEnsembleVote {
    /// Creates a child ensemble vote.
    pub const fn new(module_id: &'static str, state: SignalState, confidence_bps: u16) -> Self {
        Self {
            module_id,
            state,
            confidence_bps,
            weight_bps: 10_000,
            quality_flags: 0,
            veto: matches!(state, SignalState::Blocked),
        }
    }

    /// Creates an ensemble vote from a signal snapshot.
    pub fn from_snapshot(snapshot: &SignalSnapshot) -> Self {
        Self {
            module_id: snapshot.module_id,
            state: snapshot.state,
            confidence_bps: snapshot.confidence_bps.min(10_000),
            weight_bps: 10_000,
            quality_flags: snapshot.quality_flags,
            veto: snapshot.state == SignalState::Blocked,
        }
    }

    /// Creates an ensemble vote from a signal explanation.
    pub fn from_explanation(explanation: &SignalExplanation) -> Self {
        Self {
            module_id: explanation.module_id,
            state: explanation.state,
            confidence_bps: explanation.confidence_bps.min(10_000),
            weight_bps: 10_000,
            quality_flags: explanation.quality_flags,
            veto: explanation.state == SignalState::Blocked,
        }
    }

    /// Returns this vote with a different child weight.
    pub const fn with_weight_bps(mut self, weight_bps: u16) -> Self {
        self.weight_bps = weight_bps;
        self
    }

    /// Returns this vote with a different quality flag set.
    pub const fn with_quality_flags(mut self, quality_flags: u32) -> Self {
        self.quality_flags = quality_flags;
        self
    }

    /// Returns this vote with explicit veto behavior.
    pub const fn with_veto(mut self, veto: bool) -> Self {
        self.veto = veto;
        self
    }
}

/// Conflict observed while evaluating an ensemble.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SignalEnsembleConflict {
    /// No conflict was observed.
    #[default]
    None,
    /// Long and short evidence tied and was not resolved directionally.
    Tie,
    /// A veto controlled the final state.
    Veto,
}

/// Aggregate metrics produced by ensemble evaluation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SignalEnsembleMetrics {
    /// Total child votes inspected.
    pub total_votes: usize,
    /// Votes that met the confidence filter and were not blocked.
    pub eligible_votes: usize,
    /// Eligible long votes.
    pub long_votes: usize,
    /// Eligible short votes.
    pub short_votes: usize,
    /// Eligible neutral votes.
    pub neutral_votes: usize,
    /// Blocked child votes.
    pub blocked_votes: usize,
    /// Child votes marked as vetoes.
    pub veto_votes: usize,
    /// Aggregated child quality flags.
    pub aggregate_quality_flags: u32,
    /// Weighted long confidence score.
    pub long_weighted_score_bps: u64,
    /// Weighted short confidence score.
    pub short_weighted_score_bps: u64,
    /// Weighted neutral confidence score.
    pub neutral_weighted_score_bps: u64,
    /// Average confidence across eligible votes.
    pub average_confidence_bps: u16,
}

/// Result of evaluating a signal ensemble.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SignalEnsembleDecision {
    /// Ensemble signal snapshot.
    pub snapshot: SignalSnapshot,
    /// Metrics calculated during evaluation.
    pub metrics: SignalEnsembleMetrics,
    /// Conflict classification.
    pub conflict: SignalEnsembleConflict,
    /// Whether a child veto controlled the final state.
    pub veto_applied: bool,
}

/// Aggregated explanation for an ensemble decision.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SignalEnsembleExplanation {
    /// Ensemble decision.
    pub decision: SignalEnsembleDecision,
    /// Child explanations that contributed to the decision.
    pub children: Vec<SignalExplanation>,
}

impl SignalEnsembleExplanation {
    /// Creates an ensemble explanation from a decision and child explanations.
    pub fn new(decision: SignalEnsembleDecision, children: Vec<SignalExplanation>) -> Self {
        Self { decision, children }
    }

    /// Returns a compact top-level signal explanation for the ensemble.
    pub fn explanation(&self) -> SignalExplanation {
        let reason_code = match self.decision.snapshot.state {
            SignalState::LongBias => SignalReasonCode::EnsembleLongSelected,
            SignalState::ShortBias => SignalReasonCode::EnsembleShortSelected,
            SignalState::Blocked => SignalReasonCode::EnsembleVetoApplied,
            SignalState::Neutral => SignalReasonCode::EnsembleNoSelection,
        };

        SignalExplanation::from_snapshot(&self.decision.snapshot, reason_code)
            .with_input(SignalInputValue::integer(
                "total_votes",
                self.decision.metrics.total_votes as i64,
            ))
            .with_input(SignalInputValue::integer(
                "eligible_votes",
                self.decision.metrics.eligible_votes as i64,
            ))
            .with_input(SignalInputValue::integer(
                "long_votes",
                self.decision.metrics.long_votes as i64,
            ))
            .with_input(SignalInputValue::integer(
                "short_votes",
                self.decision.metrics.short_votes as i64,
            ))
            .with_input(SignalInputValue::integer(
                "veto_votes",
                self.decision.metrics.veto_votes as i64,
            ))
            .with_confidence_component(SignalConfidenceComponent::new(
                "average_child_confidence",
                self.decision.metrics.average_confidence_bps,
            ))
    }
}

/// Evaluates child votes into one ensemble signal decision.
pub fn evaluate_signal_ensemble(
    module_id: &'static str,
    votes: &[SignalEnsembleVote],
    policy: SignalEnsemblePolicy,
) -> SignalEnsembleDecision {
    let (metrics, accumulator) = collect_ensemble_metrics(votes, policy);

    if votes.is_empty() {
        return ensemble_decision(
            ensemble_snapshot(
                module_id,
                SignalState::Neutral,
                0,
                0,
                "ensemble_no_votes".to_string(),
            ),
            metrics,
            SignalEnsembleConflict::None,
            false,
        );
    }

    if policy.veto_policy != SignalEnsembleVetoPolicy::Ignore && metrics.veto_votes > 0 {
        let state = match policy.veto_policy {
            SignalEnsembleVetoPolicy::Ignore => SignalState::Neutral,
            SignalEnsembleVetoPolicy::NeutralOnVeto => SignalState::Neutral,
            SignalEnsembleVetoPolicy::BlockOnVeto => SignalState::Blocked,
        };
        return ensemble_decision(
            ensemble_snapshot(
                module_id,
                state,
                0,
                metrics.aggregate_quality_flags,
                "ensemble_veto_applied".to_string(),
            ),
            metrics,
            SignalEnsembleConflict::Veto,
            true,
        );
    }

    let (state, conflict) = select_ensemble_state(policy, metrics, accumulator);
    let confidence_bps = confidence_for_state(state, accumulator, metrics);
    let reason = ensemble_reason(policy.rule, state, metrics, conflict);

    ensemble_decision(
        ensemble_snapshot(
            module_id,
            state,
            confidence_bps,
            metrics.aggregate_quality_flags,
            reason,
        ),
        metrics,
        conflict,
        false,
    )
}

/// Evaluates child explanations and returns an aggregated ensemble explanation.
pub fn evaluate_signal_ensemble_explanations(
    module_id: &'static str,
    child_explanations: Vec<SignalExplanation>,
    weights_bps: &[u16],
    policy: SignalEnsemblePolicy,
) -> SignalEnsembleExplanation {
    let votes: Vec<_> = child_explanations
        .iter()
        .enumerate()
        .map(|(index, explanation)| {
            SignalEnsembleVote::from_explanation(explanation)
                .with_weight_bps(*weights_bps.get(index).unwrap_or(&10_000))
        })
        .collect();
    let decision = evaluate_signal_ensemble(module_id, &votes, policy);
    SignalEnsembleExplanation::new(decision, child_explanations)
}

#[derive(Debug, Clone, Copy, Default)]
struct SignalEnsembleAccumulator {
    long_confidence_sum: u64,
    short_confidence_sum: u64,
    neutral_confidence_sum: u64,
    eligible_confidence_sum: u64,
    max_long_confidence_bps: u16,
    max_short_confidence_bps: u16,
}

fn collect_ensemble_metrics(
    votes: &[SignalEnsembleVote],
    policy: SignalEnsemblePolicy,
) -> (SignalEnsembleMetrics, SignalEnsembleAccumulator) {
    let mut metrics = SignalEnsembleMetrics {
        total_votes: votes.len(),
        ..SignalEnsembleMetrics::default()
    };
    let mut accumulator = SignalEnsembleAccumulator::default();

    for vote in votes {
        let confidence = vote.confidence_bps.min(10_000);
        let weight = vote.weight_bps.min(10_000);
        metrics.aggregate_quality_flags |= vote.quality_flags;
        if vote.veto {
            metrics.veto_votes += 1;
        }
        if vote.state == SignalState::Blocked {
            metrics.blocked_votes += 1;
            continue;
        }
        if confidence < policy.min_confidence_bps.min(10_000) {
            continue;
        }

        metrics.eligible_votes += 1;
        accumulator.eligible_confidence_sum += u64::from(confidence);
        let weighted_score = weighted_confidence_score_bps(confidence, weight);

        match vote.state {
            SignalState::LongBias => {
                metrics.long_votes += 1;
                metrics.long_weighted_score_bps += weighted_score;
                accumulator.long_confidence_sum += u64::from(confidence);
                accumulator.max_long_confidence_bps =
                    accumulator.max_long_confidence_bps.max(confidence);
            }
            SignalState::ShortBias => {
                metrics.short_votes += 1;
                metrics.short_weighted_score_bps += weighted_score;
                accumulator.short_confidence_sum += u64::from(confidence);
                accumulator.max_short_confidence_bps =
                    accumulator.max_short_confidence_bps.max(confidence);
            }
            SignalState::Neutral => {
                metrics.neutral_votes += 1;
                metrics.neutral_weighted_score_bps += weighted_score;
                accumulator.neutral_confidence_sum += u64::from(confidence);
            }
            SignalState::Blocked => {}
        }
    }

    metrics.average_confidence_bps =
        average_bps(accumulator.eligible_confidence_sum, metrics.eligible_votes);
    (metrics, accumulator)
}

fn weighted_confidence_score_bps(confidence_bps: u16, weight_bps: u16) -> u64 {
    (u64::from(confidence_bps) * u64::from(weight_bps)) / 10_000
}

fn select_ensemble_state(
    policy: SignalEnsemblePolicy,
    metrics: SignalEnsembleMetrics,
    accumulator: SignalEnsembleAccumulator,
) -> (SignalState, SignalEnsembleConflict) {
    match policy.rule {
        SignalEnsembleDecisionRule::Majority => select_by_counts(
            metrics.long_votes,
            metrics.short_votes,
            policy.conflict_policy,
            metrics,
            accumulator,
        ),
        SignalEnsembleDecisionRule::Quorum { min_votes } => {
            let long_met = metrics.long_votes >= min_votes && metrics.long_votes > 0;
            let short_met = metrics.short_votes >= min_votes && metrics.short_votes > 0;
            match (long_met, short_met) {
                (true, false) => (SignalState::LongBias, SignalEnsembleConflict::None),
                (false, true) => (SignalState::ShortBias, SignalEnsembleConflict::None),
                (true, true) => select_by_counts(
                    metrics.long_votes,
                    metrics.short_votes,
                    policy.conflict_policy,
                    metrics,
                    accumulator,
                ),
                (false, false) => (SignalState::Neutral, SignalEnsembleConflict::None),
            }
        }
        SignalEnsembleDecisionRule::Weighted { min_score_bps } => {
            let min_score = u64::from(min_score_bps);
            let long_met =
                metrics.long_weighted_score_bps >= min_score && metrics.long_weighted_score_bps > 0;
            let short_met = metrics.short_weighted_score_bps >= min_score
                && metrics.short_weighted_score_bps > 0;
            match (long_met, short_met) {
                (true, false) => (SignalState::LongBias, SignalEnsembleConflict::None),
                (false, true) => (SignalState::ShortBias, SignalEnsembleConflict::None),
                (true, true) => {
                    select_by_weighted_score(policy.conflict_policy, metrics, accumulator)
                }
                (false, false) => (SignalState::Neutral, SignalEnsembleConflict::None),
            }
        }
    }
}

fn select_by_counts(
    long_votes: usize,
    short_votes: usize,
    conflict_policy: SignalEnsembleConflictPolicy,
    metrics: SignalEnsembleMetrics,
    accumulator: SignalEnsembleAccumulator,
) -> (SignalState, SignalEnsembleConflict) {
    if long_votes > short_votes {
        (SignalState::LongBias, SignalEnsembleConflict::None)
    } else if short_votes > long_votes {
        (SignalState::ShortBias, SignalEnsembleConflict::None)
    } else if long_votes == 0 {
        (SignalState::Neutral, SignalEnsembleConflict::None)
    } else {
        resolve_ensemble_conflict(conflict_policy, metrics, accumulator)
    }
}

fn select_by_weighted_score(
    conflict_policy: SignalEnsembleConflictPolicy,
    metrics: SignalEnsembleMetrics,
    accumulator: SignalEnsembleAccumulator,
) -> (SignalState, SignalEnsembleConflict) {
    if metrics.long_weighted_score_bps > metrics.short_weighted_score_bps {
        (SignalState::LongBias, SignalEnsembleConflict::None)
    } else if metrics.short_weighted_score_bps > metrics.long_weighted_score_bps {
        (SignalState::ShortBias, SignalEnsembleConflict::None)
    } else {
        resolve_ensemble_conflict(conflict_policy, metrics, accumulator)
    }
}

fn resolve_ensemble_conflict(
    conflict_policy: SignalEnsembleConflictPolicy,
    metrics: SignalEnsembleMetrics,
    accumulator: SignalEnsembleAccumulator,
) -> (SignalState, SignalEnsembleConflict) {
    match conflict_policy {
        SignalEnsembleConflictPolicy::NeutralOnTie => {
            (SignalState::Neutral, SignalEnsembleConflict::Tie)
        }
        SignalEnsembleConflictPolicy::HighestConfidence => {
            if accumulator.max_long_confidence_bps > accumulator.max_short_confidence_bps {
                (SignalState::LongBias, SignalEnsembleConflict::None)
            } else if accumulator.max_short_confidence_bps > accumulator.max_long_confidence_bps {
                (SignalState::ShortBias, SignalEnsembleConflict::None)
            } else {
                (SignalState::Neutral, SignalEnsembleConflict::Tie)
            }
        }
        SignalEnsembleConflictPolicy::HighestWeightedScore => {
            if metrics.long_weighted_score_bps > metrics.short_weighted_score_bps {
                (SignalState::LongBias, SignalEnsembleConflict::None)
            } else if metrics.short_weighted_score_bps > metrics.long_weighted_score_bps {
                (SignalState::ShortBias, SignalEnsembleConflict::None)
            } else {
                (SignalState::Neutral, SignalEnsembleConflict::Tie)
            }
        }
    }
}

fn confidence_for_state(
    state: SignalState,
    accumulator: SignalEnsembleAccumulator,
    metrics: SignalEnsembleMetrics,
) -> u16 {
    match state {
        SignalState::LongBias => average_bps(accumulator.long_confidence_sum, metrics.long_votes),
        SignalState::ShortBias => {
            average_bps(accumulator.short_confidence_sum, metrics.short_votes)
        }
        SignalState::Neutral => {
            if metrics.neutral_votes > 0 {
                average_bps(accumulator.neutral_confidence_sum, metrics.neutral_votes)
            } else {
                metrics.average_confidence_bps
            }
        }
        SignalState::Blocked => 0,
    }
}

fn ensemble_reason(
    rule: SignalEnsembleDecisionRule,
    state: SignalState,
    metrics: SignalEnsembleMetrics,
    conflict: SignalEnsembleConflict,
) -> String {
    let selected = match state {
        SignalState::LongBias => "long",
        SignalState::ShortBias => "short",
        SignalState::Neutral => "neutral",
        SignalState::Blocked => "blocked",
    };
    format!(
        "ensemble_{selected}:rule={}:long_votes={}:short_votes={}:long_score={}:short_score={}:conflict={}",
        ensemble_rule_name(rule),
        metrics.long_votes,
        metrics.short_votes,
        metrics.long_weighted_score_bps,
        metrics.short_weighted_score_bps,
        ensemble_conflict_name(conflict)
    )
}

fn ensemble_rule_name(rule: SignalEnsembleDecisionRule) -> &'static str {
    match rule {
        SignalEnsembleDecisionRule::Majority => "majority",
        SignalEnsembleDecisionRule::Quorum { .. } => "quorum",
        SignalEnsembleDecisionRule::Weighted { .. } => "weighted",
    }
}

fn ensemble_conflict_name(conflict: SignalEnsembleConflict) -> &'static str {
    match conflict {
        SignalEnsembleConflict::None => "none",
        SignalEnsembleConflict::Tie => "tie",
        SignalEnsembleConflict::Veto => "veto",
    }
}

fn ensemble_snapshot(
    module_id: &'static str,
    state: SignalState,
    confidence_bps: u16,
    quality_flags: u32,
    reason: String,
) -> SignalSnapshot {
    SignalSnapshot {
        module_id,
        state,
        confidence_bps,
        quality_flags,
        reason,
    }
}

fn ensemble_decision(
    snapshot: SignalSnapshot,
    metrics: SignalEnsembleMetrics,
    conflict: SignalEnsembleConflict,
    veto_applied: bool,
) -> SignalEnsembleDecision {
    SignalEnsembleDecision {
        snapshot,
        metrics,
        conflict,
        veto_applied,
    }
}
