use super::*;

/// Replay event consumed by an algorithm harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[allow(
    clippy::large_enum_variant,
    reason = "replay events remain Copy so caller-owned replay buffers avoid boxing"
)]
pub enum AlgoReplayEvent {
    /// Deterministic timer tick at `timestamp_ns`.
    Timer {
        /// Timer timestamp in nanoseconds.
        timestamp_ns: u64,
    },
    /// Canonical OMS execution event.
    Execution(ExecutionEvent),
    /// Parent lifecycle status update.
    ParentStatus {
        /// New parent status.
        status: ParentOrderStatus,
    },
}

/// Sequenced replay input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoReplayInput {
    sequence: u64,
    event: AlgoReplayEvent,
}

impl AlgoReplayInput {
    /// Creates a replay input.
    pub const fn new(sequence: u64, event: AlgoReplayEvent) -> Self {
        Self { sequence, event }
    }

    /// Returns the replay input sequence.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Returns the replay event.
    pub const fn event(&self) -> AlgoReplayEvent {
        self.event
    }
}

/// Deterministic child/client id generation prefixes for replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoReplayIdScheme {
    child_prefix: FixedAscii<24>,
    client_prefix: FixedAscii<24>,
}

impl AlgoReplayIdScheme {
    /// Creates replay id prefixes.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionCoreError`] when a prefix is non-ASCII or too long.
    pub fn new(child_prefix: &str, client_prefix: &str) -> Result<Self, ExecutionCoreError> {
        Ok(Self {
            child_prefix: FixedAscii::new(child_prefix)?,
            client_prefix: FixedAscii::new(client_prefix)?,
        })
    }

    /// Returns the child id prefix.
    pub const fn child_prefix(&self) -> FixedAscii<24> {
        self.child_prefix
    }

    /// Returns the client order id prefix.
    pub const fn client_prefix(&self) -> FixedAscii<24> {
        self.client_prefix
    }

    fn child_id(&self, index: u64) -> Result<ChildOrderId, AlgoError> {
        fixed_id_with_index(self.child_prefix.as_str(), index)
    }

    fn client_order_id(&self, index: u64) -> Result<ClientOrderId, AlgoError> {
        fixed_id_with_index(self.client_prefix.as_str(), index)
    }
}

impl Default for AlgoReplayIdScheme {
    fn default() -> Self {
        Self::new("child", "cl").expect("static replay id prefixes are valid")
    }
}

/// Replay step emitted for one input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoReplayStep<const N: usize = DEFAULT_ALGO_DECISION_CAPACITY> {
    input_sequence: u64,
    event: AlgoReplayEvent,
    progress_before: AlgoProgress,
    progress_after: AlgoProgress,
    decision: AlgoDecision<N>,
}

impl<const N: usize> AlgoReplayStep<N> {
    /// Returns the replay input sequence.
    pub const fn input_sequence(&self) -> u64 {
        self.input_sequence
    }

    /// Returns the replay event.
    pub const fn event(&self) -> AlgoReplayEvent {
        self.event
    }

    /// Returns progress before applying the event.
    pub const fn progress_before(&self) -> AlgoProgress {
        self.progress_before
    }

    /// Returns progress after applying the event and any generated decision.
    pub const fn progress_after(&self) -> AlgoProgress {
        self.progress_after
    }

    /// Returns the decision generated for this input.
    pub const fn decision(&self) -> &AlgoDecision<N> {
        &self.decision
    }
}

/// Summary returned by deterministic TWAP replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoReplaySummary {
    input_events: u64,
    decisions: u64,
    actions: u64,
    submitted_children: u64,
    final_progress: AlgoProgress,
    deterministic_hash: u64,
}

impl AlgoReplaySummary {
    /// Returns replay input event count.
    pub const fn input_events(&self) -> u64 {
        self.input_events
    }

    /// Returns emitted decision count.
    pub const fn decisions(&self) -> u64 {
        self.decisions
    }

    /// Returns total emitted action count.
    pub const fn actions(&self) -> u64 {
        self.actions
    }

    /// Returns submitted child action count.
    pub const fn submitted_children(&self) -> u64 {
        self.submitted_children
    }

    /// Returns final parent progress.
    pub const fn final_progress(&self) -> AlgoProgress {
        self.final_progress
    }

    /// Returns deterministic replay hash for regression checks.
    pub const fn deterministic_hash(&self) -> u64 {
        self.deterministic_hash
    }
}

/// Replays TWAP planning over explicit deterministic inputs.
///
/// `out` is caller-owned and cleared before replay. This keeps allocation
/// policy outside the harness and lets test/benchmark callers reuse capacity.
///
/// # Errors
///
/// Returns [`AlgoError`] when parent/progress state is invalid, a generated
/// child plan is invalid, or the fixed-capacity decision cannot hold a due
/// action.
pub fn replay_twap_into<const N: usize>(
    parent: ParentOrder,
    planner: TwapSlicePlanner,
    inputs: &[AlgoReplayInput],
    id_scheme: AlgoReplayIdScheme,
    out: &mut Vec<AlgoReplayStep<N>>,
) -> Result<AlgoReplaySummary, AlgoError> {
    parent.validate()?;
    out.clear();

    let mut active_parent = parent;
    let mut progress = AlgoProgress::new(parent.id(), parent.total_qty());
    let mut decisions = 0_u64;
    let mut actions = 0_u64;
    let mut submitted_children = 0_u64;
    let mut hash = FNV_OFFSET_BASIS;

    for input in inputs {
        let before = progress;
        let mut decision = AlgoDecision::<N>::new(decisions.saturating_add(1));
        match input.event() {
            AlgoReplayEvent::Timer { timestamp_ns } => {
                let next_child = submitted_children.saturating_add(1);
                if let Some(plan) = planner.plan_due_slice(
                    &active_parent,
                    progress,
                    timestamp_ns,
                    id_scheme.child_id(next_child)?,
                    id_scheme.client_order_id(next_child)?,
                    timestamp_ns,
                )? {
                    decision.push(AlgoAction::SubmitChild(plan))?;
                    progress.on_child_released(&plan)?;
                    submitted_children = submitted_children.saturating_add(1);
                }
            }
            AlgoReplayEvent::Execution(event) => {
                progress.on_execution_event(&event);
            }
            AlgoReplayEvent::ParentStatus { status } => {
                active_parent = active_parent.with_status(status);
            }
        }

        decisions = decisions.saturating_add(1);
        actions = actions.saturating_add(usize_to_u64(decision.len()));
        hash = hash_replay_step(hash, input.sequence(), input.event(), progress, &decision);
        out.push(AlgoReplayStep {
            input_sequence: input.sequence(),
            event: input.event(),
            progress_before: before,
            progress_after: progress,
            decision,
        });
    }

    Ok(AlgoReplaySummary {
        input_events: usize_to_u64(inputs.len()),
        decisions,
        actions,
        submitted_children,
        final_progress: progress,
        deterministic_hash: hash,
    })
}
