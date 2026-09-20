use super::*;

/// Optional stabilizer that applies hysteresis, debounce, and cooldown policies.
#[derive(Debug, Clone)]
pub struct SignalStabilizer {
    hysteresis: HysteresisPolicy,
    debounce: DebouncePolicy,
    cooldown: CooldownPolicy,
    emitted: Option<SignalSnapshot>,
    pending: Option<PendingSignal>,
    last_transition_ns: Option<u64>,
    last_transition: SignalTransitionKind,
}

impl SignalStabilizer {
    /// Creates a stabilizer with all policies disabled.
    pub fn new() -> Self {
        Self::with_policies(
            HysteresisPolicy::default(),
            DebouncePolicy::default(),
            CooldownPolicy::default(),
        )
    }

    /// Creates a stabilizer from explicit policies.
    pub fn with_policies(
        hysteresis: HysteresisPolicy,
        debounce: DebouncePolicy,
        cooldown: CooldownPolicy,
    ) -> Self {
        Self {
            hysteresis,
            debounce,
            cooldown,
            emitted: None,
            pending: None,
            last_transition_ns: None,
            last_transition: SignalTransitionKind::None,
        }
    }

    /// Returns the configured hysteresis policy.
    pub const fn hysteresis(&self) -> HysteresisPolicy {
        self.hysteresis
    }

    /// Returns the configured debounce policy.
    pub const fn debounce(&self) -> DebouncePolicy {
        self.debounce
    }

    /// Returns the configured cooldown policy.
    pub const fn cooldown(&self) -> CooldownPolicy {
        self.cooldown
    }

    /// Returns the last emitted signal, if any.
    pub fn emitted(&self) -> Option<&SignalSnapshot> {
        self.emitted.as_ref()
    }

    /// Clears emitted and pending stabilization state.
    pub fn reset(&mut self) {
        self.emitted = None;
        self.pending = None;
        self.last_transition_ns = None;
        self.last_transition = SignalTransitionKind::None;
    }

    /// Applies stabilization policies to a requested signal snapshot.
    pub fn stabilize(&mut self, requested: SignalSnapshot, now_ns: u64) -> StabilizedSignal {
        let previous_state = self
            .emitted
            .as_ref()
            .map_or(SignalState::Neutral, |snapshot| snapshot.state);
        let transition = classify_transition(previous_state, requested.state);

        if requested.state == SignalState::Blocked {
            return self.accept(requested, now_ns, transition);
        }

        if let Some(reason) = self.hysteresis_suppression(&requested, transition) {
            self.pending = None;
            return self.suppress(requested, transition, reason);
        }

        if self.cooldown_active(now_ns, transition) {
            self.pending = None;
            return self.suppress(
                requested,
                transition,
                SignalSuppressionReason::CooldownActive,
            );
        }

        if !self.debounce_satisfied(&requested, now_ns, transition) {
            return self.suppress(
                requested,
                transition,
                SignalSuppressionReason::DebouncePending,
            );
        }

        self.pending = None;
        self.accept(requested, now_ns, transition)
    }

    fn accept(
        &mut self,
        requested: SignalSnapshot,
        now_ns: u64,
        transition: SignalTransitionKind,
    ) -> StabilizedSignal {
        if transition != SignalTransitionKind::None {
            self.last_transition_ns = Some(now_ns);
            self.last_transition = transition;
        }
        self.emitted = Some(requested.clone());
        StabilizedSignal {
            requested: requested.clone(),
            emitted: requested,
            accepted: true,
            suppression_reason: SignalSuppressionReason::None,
            transition,
        }
    }

    fn suppress(
        &self,
        requested: SignalSnapshot,
        transition: SignalTransitionKind,
        suppression_reason: SignalSuppressionReason,
    ) -> StabilizedSignal {
        let emitted = self
            .emitted
            .clone()
            .unwrap_or_else(|| neutral_like(&requested));
        StabilizedSignal {
            requested,
            emitted,
            accepted: false,
            suppression_reason,
            transition,
        }
    }

    fn hysteresis_suppression(
        &self,
        requested: &SignalSnapshot,
        transition: SignalTransitionKind,
    ) -> Option<SignalSuppressionReason> {
        let required = match transition {
            SignalTransitionKind::None => 0,
            SignalTransitionKind::Entry => self.hysteresis.min_entry_confidence_bps,
            SignalTransitionKind::Exit => self.hysteresis.min_exit_confidence_bps,
            SignalTransitionKind::Reversal => self.hysteresis.min_reversal_confidence_bps,
            SignalTransitionKind::StateChange => self.hysteresis.min_entry_confidence_bps,
        };
        if requested.confidence_bps < required {
            Some(SignalSuppressionReason::Hysteresis)
        } else {
            None
        }
    }

    fn cooldown_active(&self, now_ns: u64, transition: SignalTransitionKind) -> bool {
        if transition == SignalTransitionKind::None || self.cooldown.is_disabled() {
            return false;
        }
        let Some(last_transition_ns) = self.last_transition_ns else {
            return false;
        };
        let cooldown_ns = match self.last_transition {
            SignalTransitionKind::Entry => self.cooldown.after_entry_ns,
            SignalTransitionKind::Exit => self.cooldown.after_exit_ns,
            SignalTransitionKind::Reversal => self.cooldown.after_reversal_ns,
            SignalTransitionKind::None | SignalTransitionKind::StateChange => 0,
        };
        now_ns.saturating_sub(last_transition_ns) < cooldown_ns
    }

    fn debounce_satisfied(
        &mut self,
        requested: &SignalSnapshot,
        now_ns: u64,
        transition: SignalTransitionKind,
    ) -> bool {
        if transition == SignalTransitionKind::None || self.debounce.is_disabled() {
            self.pending = None;
            return true;
        }

        let pending = match self.pending.as_mut() {
            Some(pending) if same_pending_state(&pending.snapshot, requested) => {
                pending.confirming_events = pending.confirming_events.saturating_add(1);
                pending.snapshot = requested.clone();
                pending
            }
            _ => {
                self.pending = Some(PendingSignal {
                    snapshot: requested.clone(),
                    first_seen_ns: now_ns,
                    confirming_events: 1,
                });
                self.pending.as_mut().expect("pending just inserted")
            }
        };

        pending.confirming_events >= self.debounce.confirming_events
            && now_ns.saturating_sub(pending.first_seen_ns) >= self.debounce.confirming_time_ns
    }
}

impl Default for SignalStabilizer {
    fn default() -> Self {
        Self::new()
    }
}
