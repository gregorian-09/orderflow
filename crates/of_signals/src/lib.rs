#![doc = include_str!("../README.md")]

use std::ops::{BitOr, BitOrAssign};

use of_core::{
    AnalyticsSnapshot, BookSnapshot, DataQualityFlags, SignalSnapshot, SignalState, SymbolId,
};

/// Result of running quality-gate checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalGateDecision {
    /// Signal may be emitted.
    Pass,
    /// Signal must be blocked due to quality policy.
    Block,
}

/// Trait implemented by signal modules consumed by the runtime.
pub trait SignalModule: Send + Sync {
    /// Updates internal module state using latest analytics.
    fn on_analytics(&mut self, ev: &AnalyticsSnapshot);
    /// Returns the current signal snapshot.
    fn snapshot(&self) -> SignalSnapshot;
    /// Applies module-specific data-quality gate.
    fn quality_gate(&self, q: DataQualityFlags) -> SignalGateDecision;
}

/// Context passed to contextual signal modules.
///
/// This type is intentionally borrowed so hosts can compose analytics, book,
/// symbol, and lifecycle metadata without cloning hot-path state.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct SignalContext<'a> {
    /// Latest analytics snapshot.
    pub analytics: &'a AnalyticsSnapshot,
    /// Data-quality flags active for this evaluation.
    pub data_quality: DataQualityFlags,
    /// Optional symbol identity for multi-symbol hosts.
    pub symbol: Option<&'a SymbolId>,
    /// Optional materialized order-book snapshot.
    pub book: Option<&'a BookSnapshot>,
    /// Exchange timestamp associated with this evaluation, when known.
    pub ts_exchange_ns: Option<u64>,
    /// Local receive/evaluation timestamp associated with this evaluation, when known.
    pub ts_recv_ns: Option<u64>,
    /// Lifecycle state supplied by the host, when known.
    pub lifecycle_state: Option<SignalLifecycleState>,
    /// Optional opaque extension tags for host-specific context.
    pub extension_tags: &'a [(&'a str, &'a str)],
}

impl<'a> SignalContext<'a> {
    /// Creates a context from analytics and data-quality state.
    pub const fn new(analytics: &'a AnalyticsSnapshot, data_quality: DataQualityFlags) -> Self {
        Self {
            analytics,
            data_quality,
            symbol: None,
            book: None,
            ts_exchange_ns: None,
            ts_recv_ns: None,
            lifecycle_state: None,
            extension_tags: &[],
        }
    }

    /// Returns a context with symbol identity attached.
    pub const fn with_symbol(mut self, symbol: &'a SymbolId) -> Self {
        self.symbol = Some(symbol);
        self
    }

    /// Returns a context with a materialized book snapshot attached.
    pub const fn with_book(mut self, book: &'a BookSnapshot) -> Self {
        self.book = Some(book);
        self
    }

    /// Returns a context with exchange and receive timestamps attached.
    pub const fn with_timestamps(
        mut self,
        ts_exchange_ns: Option<u64>,
        ts_recv_ns: Option<u64>,
    ) -> Self {
        self.ts_exchange_ns = ts_exchange_ns;
        self.ts_recv_ns = ts_recv_ns;
        self
    }

    /// Returns a context with host lifecycle state attached.
    pub const fn with_lifecycle_state(mut self, lifecycle_state: SignalLifecycleState) -> Self {
        self.lifecycle_state = Some(lifecycle_state);
        self
    }

    /// Returns a context with opaque extension tags attached.
    pub const fn with_extension_tags(mut self, extension_tags: &'a [(&'a str, &'a str)]) -> Self {
        self.extension_tags = extension_tags;
        self
    }
}

/// Trait for signal modules that consume richer evaluation context.
///
/// This is additive beside [`SignalModule`]. Existing signal modules can be
/// adapted with [`LegacySignalAdapter`] instead of being rewritten.
pub trait ContextualSignalModule: Send + Sync {
    /// Updates internal state from the latest signal context.
    fn on_context(&mut self, ctx: &SignalContext<'_>);

    /// Returns the current signal snapshot.
    fn snapshot(&self) -> SignalSnapshot;

    /// Applies contextual data-quality gating.
    fn quality_gate(&self, ctx: &SignalContext<'_>) -> SignalGateDecision {
        default_quality_gate(ctx.data_quality)
    }

    /// Returns static descriptor metadata when available.
    fn descriptor(&self) -> Option<&'static SignalDescriptor> {
        None
    }

    /// Returns lifecycle state when the module or wrapper tracks it.
    fn lifecycle_state(&self) -> Option<SignalLifecycleState> {
        None
    }
}

/// Adapter that lets an existing [`SignalModule`] consume [`SignalContext`].
#[derive(Debug)]
pub struct LegacySignalAdapter<S> {
    inner: S,
    descriptor: Option<&'static SignalDescriptor>,
    lifecycle: SignalLifecycle,
}

impl<S> LegacySignalAdapter<S> {
    /// Wraps a legacy signal module with no descriptor metadata.
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            descriptor: None,
            lifecycle: SignalLifecycle::new(SignalWarmupRequirement::Events(1)),
        }
    }

    /// Wraps a legacy signal module with descriptor metadata.
    pub fn with_descriptor(inner: S, descriptor: &'static SignalDescriptor) -> Self {
        Self {
            inner,
            descriptor: Some(descriptor),
            lifecycle: SignalLifecycle::new(descriptor.warmup),
        }
    }

    /// Returns the wrapped signal module by reference.
    pub const fn inner(&self) -> &S {
        &self.inner
    }

    /// Returns the wrapped signal module by mutable reference.
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Consumes the adapter and returns the wrapped signal module.
    pub fn into_inner(self) -> S {
        self.inner
    }

    /// Returns the adapter lifecycle helper.
    pub const fn lifecycle(&self) -> SignalLifecycle {
        self.lifecycle
    }

    /// Resets adapter warmup progress.
    pub fn reset_lifecycle(&mut self) {
        self.lifecycle.reset_warmup();
    }
}

impl<S: SignalModule> ContextualSignalModule for LegacySignalAdapter<S> {
    fn on_context(&mut self, ctx: &SignalContext<'_>) {
        self.lifecycle.record_event();
        self.inner.on_analytics(ctx.analytics);
    }

    fn snapshot(&self) -> SignalSnapshot {
        self.inner.snapshot()
    }

    fn quality_gate(&self, ctx: &SignalContext<'_>) -> SignalGateDecision {
        self.inner.quality_gate(ctx.data_quality)
    }

    fn descriptor(&self) -> Option<&'static SignalDescriptor> {
        self.descriptor
    }

    fn lifecycle_state(&self) -> Option<SignalLifecycleState> {
        Some(self.lifecycle.state())
    }
}

/// Bitset describing which inputs a signal needs to evaluate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalInputMask(u32);

impl SignalInputMask {
    /// No declared inputs.
    pub const NONE: Self = Self(0);
    /// The signal consumes `of_core::AnalyticsSnapshot`.
    pub const ANALYTICS: Self = Self(1 << 0);
    /// The signal evaluates `of_core::DataQualityFlags`.
    pub const DATA_QUALITY: Self = Self(1 << 1);
    /// The signal needs reconstructed book state.
    pub const BOOK: Self = Self(1 << 2);
    /// The signal needs advanced analytics or feature vectors.
    pub const ADVANCED_ANALYTICS: Self = Self(1 << 3);
    /// The signal needs market-regime context.
    pub const MARKET_REGIME: Self = Self(1 << 4);
    /// The signal needs current position context.
    pub const POSITION: Self = Self(1 << 5);
    /// The signal needs risk or OMS gating context.
    pub const RISK: Self = Self(1 << 6);

    /// Returns the raw bit representation.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Builds an input mask from raw bits, preserving unknown future bits.
    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self(bits)
    }

    /// Returns `true` when all bits in `other` are present.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns `true` when at least one bit overlaps.
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    /// Returns a mask containing bits from both masks.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Default for SignalInputMask {
    fn default() -> Self {
        Self::NONE
    }
}

impl BitOr for SignalInputMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl BitOrAssign for SignalInputMask {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

/// Lifecycle state for production signal evaluation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalLifecycleState {
    /// The signal object exists but is not ready to evaluate.
    Initializing,
    /// The signal is receiving data but has not met its warmup requirement.
    WarmingUp,
    /// The signal is ready for normal consumption.
    Active,
    /// The signal can emit output, but consumers should treat it cautiously.
    Degraded,
    /// The signal output must not be used for trading decisions.
    Blocked,
    /// The signal is suppressing rapid transitions after a state change.
    CoolingDown,
    /// The signal is configured off.
    Disabled,
}

/// Progress available to evaluate a signal warmup requirement.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SignalWarmupProgress {
    /// Number of analytics or context updates observed.
    pub events: u64,
    /// Amount of market time observed by the signal.
    pub market_time_ns: i64,
    /// Number of completed bars observed by the signal.
    pub completed_bars: u32,
}

impl SignalWarmupProgress {
    /// Creates warmup progress from explicit counters.
    pub const fn new(events: u64, market_time_ns: i64, completed_bars: u32) -> Self {
        Self {
            events,
            market_time_ns,
            completed_bars,
        }
    }

    /// Creates warmup progress from an event count.
    pub const fn from_events(events: u64) -> Self {
        Self {
            events,
            market_time_ns: 0,
            completed_bars: 0,
        }
    }
}

/// Warmup requirement that must be satisfied before a signal is active.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SignalWarmupRequirement {
    /// No warmup is required.
    #[default]
    None,
    /// Require at least this many input events.
    Events(u64),
    /// Require at least this much market time.
    MarketTimeNs(i64),
    /// Require at least this many completed bars.
    CompletedBars(u32),
    /// Require every child requirement to be satisfied.
    All(&'static [SignalWarmupRequirement]),
}

impl SignalWarmupRequirement {
    /// Returns `true` when `progress` satisfies this requirement.
    pub fn is_satisfied_by(self, progress: SignalWarmupProgress) -> bool {
        match self {
            Self::None => true,
            Self::Events(required) => progress.events >= required,
            Self::MarketTimeNs(required) => progress.market_time_ns >= required,
            Self::CompletedBars(required) => progress.completed_bars >= required,
            Self::All(requirements) => requirements
                .iter()
                .all(|requirement| requirement.is_satisfied_by(progress)),
        }
    }
}

/// Small lifecycle helper for warmup-aware signal wrappers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalLifecycle {
    warmup: SignalWarmupRequirement,
    progress: SignalWarmupProgress,
    state: SignalLifecycleState,
}

impl SignalLifecycle {
    /// Creates a lifecycle initialized from a warmup requirement.
    pub fn new(warmup: SignalWarmupRequirement) -> Self {
        let progress = SignalWarmupProgress::default();
        let state = if warmup.is_satisfied_by(progress) {
            SignalLifecycleState::Active
        } else {
            SignalLifecycleState::WarmingUp
        };

        Self {
            warmup,
            progress,
            state,
        }
    }

    /// Returns the configured warmup requirement.
    pub const fn warmup(&self) -> SignalWarmupRequirement {
        self.warmup
    }

    /// Returns current warmup progress.
    pub const fn progress(&self) -> SignalWarmupProgress {
        self.progress
    }

    /// Returns the current lifecycle state.
    pub const fn state(&self) -> SignalLifecycleState {
        self.state
    }

    /// Returns `true` when the lifecycle is active.
    pub const fn is_active(&self) -> bool {
        matches!(self.state, SignalLifecycleState::Active)
    }

    /// Records one input event and activates the lifecycle if warmup is done.
    pub fn record_event(&mut self) {
        self.progress.events = self.progress.events.saturating_add(1);
        self.activate_if_ready();
    }

    /// Records one completed bar and activates the lifecycle if warmup is done.
    pub fn record_completed_bar(&mut self) {
        self.progress.completed_bars = self.progress.completed_bars.saturating_add(1);
        self.activate_if_ready();
    }

    /// Sets observed market time and activates the lifecycle if warmup is done.
    pub fn set_market_time_ns(&mut self, market_time_ns: i64) {
        self.progress.market_time_ns = market_time_ns.max(0);
        self.activate_if_ready();
    }

    /// Replaces all progress counters and activates the lifecycle if warmup is done.
    pub fn update_progress(&mut self, progress: SignalWarmupProgress) {
        self.progress = progress;
        self.activate_if_ready();
    }

    /// Marks the signal as degraded unless it is disabled.
    pub fn degrade(&mut self) {
        if self.state != SignalLifecycleState::Disabled {
            self.state = SignalLifecycleState::Degraded;
        }
    }

    /// Blocks the signal unless it is disabled.
    pub fn block(&mut self) {
        if self.state != SignalLifecycleState::Disabled {
            self.state = SignalLifecycleState::Blocked;
        }
    }

    /// Puts the signal into cooldown unless it is disabled.
    pub fn cool_down(&mut self) {
        if self.state != SignalLifecycleState::Disabled {
            self.state = SignalLifecycleState::CoolingDown;
        }
    }

    /// Disables the signal.
    pub fn disable(&mut self) {
        self.state = SignalLifecycleState::Disabled;
    }

    /// Resets progress and returns to the warmup state implied by the requirement.
    pub fn reset_warmup(&mut self) {
        self.progress = SignalWarmupProgress::default();
        self.state = if self.warmup.is_satisfied_by(self.progress) {
            SignalLifecycleState::Active
        } else {
            SignalLifecycleState::WarmingUp
        };
    }

    /// Activates the signal if warmup has completed and the current state can transition.
    pub fn activate_if_ready(&mut self) {
        if matches!(
            self.state,
            SignalLifecycleState::Initializing | SignalLifecycleState::WarmingUp
        ) && self.warmup.is_satisfied_by(self.progress)
        {
            self.state = SignalLifecycleState::Active;
        }
    }
}

/// Confidence thresholds used to avoid weak signal transitions.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HysteresisPolicy {
    /// Minimum confidence required to enter a directional state from neutral.
    pub min_entry_confidence_bps: u16,
    /// Minimum confidence required to exit a directional state.
    pub min_exit_confidence_bps: u16,
    /// Minimum confidence required to reverse directly between long and short.
    pub min_reversal_confidence_bps: u16,
}

impl HysteresisPolicy {
    /// Creates a hysteresis policy from confidence thresholds.
    pub const fn new(
        min_entry_confidence_bps: u16,
        min_exit_confidence_bps: u16,
        min_reversal_confidence_bps: u16,
    ) -> Self {
        Self {
            min_entry_confidence_bps,
            min_exit_confidence_bps,
            min_reversal_confidence_bps,
        }
    }

    /// Creates a policy that accepts every transition.
    pub const fn disabled() -> Self {
        Self::new(0, 0, 0)
    }

    /// Returns a policy with a different entry threshold.
    pub const fn with_entry_confidence(mut self, confidence_bps: u16) -> Self {
        self.min_entry_confidence_bps = confidence_bps;
        self
    }

    /// Returns a policy with a different exit threshold.
    pub const fn with_exit_confidence(mut self, confidence_bps: u16) -> Self {
        self.min_exit_confidence_bps = confidence_bps;
        self
    }

    /// Returns a policy with a different reversal threshold.
    pub const fn with_reversal_confidence(mut self, confidence_bps: u16) -> Self {
        self.min_reversal_confidence_bps = confidence_bps;
        self
    }
}

impl Default for HysteresisPolicy {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Confirmation policy used to prevent one-tick signal flapping.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebouncePolicy {
    /// Number of repeated candidate states required before transition.
    pub confirming_events: u32,
    /// Market/evaluation time the candidate must remain stable before transition.
    pub confirming_time_ns: u64,
}

impl DebouncePolicy {
    /// Creates a debounce policy.
    pub const fn new(confirming_events: u32, confirming_time_ns: u64) -> Self {
        Self {
            confirming_events,
            confirming_time_ns,
        }
    }

    /// Creates a policy that accepts transitions immediately.
    pub const fn disabled() -> Self {
        Self::new(1, 0)
    }

    /// Returns `true` when this policy accepts transitions immediately.
    pub const fn is_disabled(&self) -> bool {
        self.confirming_events <= 1 && self.confirming_time_ns == 0
    }
}

impl Default for DebouncePolicy {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Time-based suppression policy after accepted transitions.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CooldownPolicy {
    /// Cooldown after entering a directional state.
    pub after_entry_ns: u64,
    /// Cooldown after exiting a directional state.
    pub after_exit_ns: u64,
    /// Cooldown after reversing directly between long and short.
    pub after_reversal_ns: u64,
}

impl CooldownPolicy {
    /// Creates a cooldown policy from explicit durations.
    pub const fn new(after_entry_ns: u64, after_exit_ns: u64, after_reversal_ns: u64) -> Self {
        Self {
            after_entry_ns,
            after_exit_ns,
            after_reversal_ns,
        }
    }

    /// Creates a policy that never suppresses transitions by time.
    pub const fn disabled() -> Self {
        Self::new(0, 0, 0)
    }

    /// Returns `true` when this policy never suppresses transitions.
    pub const fn is_disabled(&self) -> bool {
        self.after_entry_ns == 0 && self.after_exit_ns == 0 && self.after_reversal_ns == 0
    }
}

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

#[derive(Debug, Clone)]
struct PendingSignal {
    snapshot: SignalSnapshot,
    first_seen_ns: u64,
    confirming_events: u32,
}

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

/// Shape of output produced by a signal module.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalOutputSemantics {
    /// Emits directional bias such as long, short, neutral, or blocked.
    DirectionalBias,
    /// Aggregates child signals into a combined output.
    CompositeBias,
    /// Emits an informational state that should not be treated as direction.
    Informational,
    /// Emits a veto or gate over another signal or strategy.
    Veto,
}

/// Parameter value used in signal metadata.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignalParameterValue {
    /// Signed integer parameter.
    Integer(i64),
    /// Floating-point parameter.
    Float(f64),
    /// Boolean parameter.
    Boolean(bool),
    /// Static text parameter.
    Text(&'static str),
}

/// Parameter type advertised by a signal descriptor.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalParameterKind {
    /// Signed integer value.
    Integer,
    /// Floating-point value.
    Float,
    /// Boolean value.
    Boolean,
    /// Static text value.
    Text,
}

/// Metadata for one configurable signal parameter.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalParameterDescriptor {
    /// Stable parameter name.
    pub name: &'static str,
    /// Human-readable parameter description.
    pub description: &'static str,
    /// Expected parameter value type.
    pub kind: SignalParameterKind,
    /// Default value used by the built-in implementation.
    pub default: Option<SignalParameterValue>,
    /// Inclusive minimum value when the parameter is range-bound.
    pub min: Option<SignalParameterValue>,
    /// Inclusive maximum value when the parameter is range-bound.
    pub max: Option<SignalParameterValue>,
}

impl SignalParameterDescriptor {
    /// Creates metadata for a signal parameter.
    pub const fn new(
        name: &'static str,
        description: &'static str,
        kind: SignalParameterKind,
        default: Option<SignalParameterValue>,
        min: Option<SignalParameterValue>,
        max: Option<SignalParameterValue>,
    ) -> Self {
        Self {
            name,
            description,
            kind,
            default,
            min,
            max,
        }
    }

    /// Creates metadata for an integer signal parameter.
    pub const fn integer(
        name: &'static str,
        description: &'static str,
        default: Option<i64>,
        min: Option<i64>,
        max: Option<i64>,
    ) -> Self {
        Self::new(
            name,
            description,
            SignalParameterKind::Integer,
            match default {
                Some(value) => Some(SignalParameterValue::Integer(value)),
                None => None,
            },
            match min {
                Some(value) => Some(SignalParameterValue::Integer(value)),
                None => None,
            },
            match max {
                Some(value) => Some(SignalParameterValue::Integer(value)),
                None => None,
            },
        )
    }
}

/// Static metadata describing a signal module.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalDescriptor {
    /// Stable signal identifier. This should match `SignalSnapshot::module_id`.
    pub id: &'static str,
    /// Human-readable signal name.
    pub name: &'static str,
    /// Descriptor/schema version for this signal definition.
    pub version: &'static str,
    /// Human-readable signal description.
    pub description: &'static str,
    /// Inputs required by the signal.
    pub required_inputs: SignalInputMask,
    /// Warmup needed before production use.
    pub warmup: SignalWarmupRequirement,
    /// Public parameter metadata.
    pub parameters: &'static [SignalParameterDescriptor],
    /// Output semantics for consumers and dashboards.
    pub output_semantics: SignalOutputSemantics,
    /// Whether the signal is deterministic for the same ordered input stream.
    pub deterministic: bool,
    /// Whether the current implementation exposes checkpointable state.
    pub checkpointable: bool,
}

impl SignalDescriptor {
    /// Creates static metadata for a signal module with conservative defaults.
    ///
    /// Use the `with_*` methods to attach inputs, warmup policy, parameters,
    /// output semantics, and capability flags.
    pub const fn new(
        id: &'static str,
        name: &'static str,
        version: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            id,
            name,
            version,
            description,
            required_inputs: SignalInputMask::NONE,
            warmup: SignalWarmupRequirement::None,
            parameters: &[],
            output_semantics: SignalOutputSemantics::DirectionalBias,
            deterministic: true,
            checkpointable: false,
        }
    }

    /// Returns a descriptor with required input metadata changed.
    pub const fn with_required_inputs(mut self, required_inputs: SignalInputMask) -> Self {
        self.required_inputs = required_inputs;
        self
    }

    /// Returns a descriptor with warmup metadata changed.
    pub const fn with_warmup(mut self, warmup: SignalWarmupRequirement) -> Self {
        self.warmup = warmup;
        self
    }

    /// Returns a descriptor with parameter metadata changed.
    pub const fn with_parameters(
        mut self,
        parameters: &'static [SignalParameterDescriptor],
    ) -> Self {
        self.parameters = parameters;
        self
    }

    /// Returns a descriptor with output semantics metadata changed.
    pub const fn with_output_semantics(mut self, output_semantics: SignalOutputSemantics) -> Self {
        self.output_semantics = output_semantics;
        self
    }

    /// Returns a descriptor with the deterministic flag changed.
    pub const fn with_deterministic(mut self, deterministic: bool) -> Self {
        self.deterministic = deterministic;
        self
    }

    /// Returns a descriptor with the checkpointable flag changed.
    pub const fn with_checkpointable(mut self, checkpointable: bool) -> Self {
        self.checkpointable = checkpointable;
        self
    }

    /// Returns `true` when the descriptor requires `input`.
    pub const fn requires_input(&self, input: SignalInputMask) -> bool {
        self.required_inputs.contains(input)
    }

    /// Finds a parameter descriptor by stable name.
    pub fn parameter(&self, name: &str) -> Option<&'static SignalParameterDescriptor> {
        self.parameters
            .iter()
            .find(|parameter| parameter.name == name)
    }
}

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
        let (state, reason) = if self.latest.delta >= self.threshold {
            (SignalState::LongBias, "delta_above_threshold")
        } else if self.latest.delta <= -self.threshold {
            (SignalState::ShortBias, "delta_below_threshold")
        } else {
            (SignalState::Neutral, "delta_inside_band")
        };

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
        let imbalance = self.latest.buy_volume - self.latest.sell_volume;
        let (state, reason) = if imbalance >= self.threshold {
            (SignalState::LongBias, "buy_volume_above_threshold")
        } else if imbalance <= -self.threshold {
            (SignalState::ShortBias, "sell_volume_above_threshold")
        } else {
            (SignalState::Neutral, "volume_inside_band")
        };

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
        let (state, reason) = if self.latest.cumulative_delta >= self.threshold {
            (SignalState::LongBias, "cumulative_delta_above_threshold")
        } else if self.latest.cumulative_delta <= -self.threshold {
            (SignalState::ShortBias, "cumulative_delta_below_threshold")
        } else {
            (SignalState::Neutral, "cumulative_delta_inside_band")
        };

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
        let poc_distance = (self.latest.last_price - self.latest.point_of_control).abs();
        let (state, reason) =
            if poc_distance <= self.price_band && self.latest.delta <= -self.threshold {
                (SignalState::LongBias, "sell_absorption_detected")
            } else if poc_distance <= self.price_band && self.latest.delta >= self.threshold {
                (SignalState::ShortBias, "buy_absorption_detected")
            } else {
                (SignalState::Neutral, "absorption_not_detected")
            };

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
        let (state, reason) = if self.latest.delta >= self.threshold
            && self.latest.last_price <= self.latest.point_of_control
        {
            (SignalState::ShortBias, "buy_exhaustion_detected")
        } else if self.latest.delta <= -self.threshold
            && self.latest.last_price >= self.latest.point_of_control
        {
            (SignalState::LongBias, "sell_exhaustion_detected")
        } else {
            (SignalState::Neutral, "exhaustion_not_detected")
        };

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
        let (state, reason) = if self.latest.delta >= self.threshold
            && self.latest.last_price >= self.latest.value_area_high + self.breakout_ticks
        {
            (SignalState::LongBias, "upside_sweep_detected")
        } else if self.latest.delta <= -self.threshold
            && self.latest.last_price <= self.latest.value_area_low - self.breakout_ticks
        {
            (SignalState::ShortBias, "downside_sweep_detected")
        } else {
            (SignalState::Neutral, "sweep_not_detected")
        };

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

        let mut long_votes = 0_u16;
        let mut short_votes = 0_u16;
        let mut confidence_sum = 0_u32;
        let mut long_modules = Vec::new();
        let mut short_modules = Vec::new();

        for module in &self.modules {
            let snapshot = module.snapshot();
            confidence_sum += snapshot.confidence_bps as u32;
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
}

fn classify_transition(previous: SignalState, requested: SignalState) -> SignalTransitionKind {
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

fn is_directional(state: SignalState) -> bool {
    matches!(state, SignalState::LongBias | SignalState::ShortBias)
}

fn same_pending_state(left: &SignalSnapshot, right: &SignalSnapshot) -> bool {
    left.module_id == right.module_id && left.state == right.state
}

fn neutral_like(requested: &SignalSnapshot) -> SignalSnapshot {
    SignalSnapshot {
        module_id: requested.module_id,
        state: SignalState::Neutral,
        confidence_bps: requested.confidence_bps,
        quality_flags: requested.quality_flags,
        reason: "stabilizer_pending".to_string(),
    }
}

fn default_quality_gate(q: DataQualityFlags) -> SignalGateDecision {
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

const THRESHOLD_PARAM_100: SignalParameterDescriptor = SignalParameterDescriptor {
    name: "threshold",
    description: "Absolute analytics threshold required before the signal emits directional bias.",
    kind: SignalParameterKind::Integer,
    default: Some(SignalParameterValue::Integer(100)),
    min: Some(SignalParameterValue::Integer(0)),
    max: None,
};

const THRESHOLD_PARAM_150: SignalParameterDescriptor = SignalParameterDescriptor {
    name: "threshold",
    description: "Absolute analytics threshold required before the signal emits directional bias.",
    kind: SignalParameterKind::Integer,
    default: Some(SignalParameterValue::Integer(150)),
    min: Some(SignalParameterValue::Integer(0)),
    max: None,
};

const THRESHOLD_PARAM_250: SignalParameterDescriptor = SignalParameterDescriptor {
    name: "threshold",
    description: "Absolute analytics threshold required before the signal emits directional bias.",
    kind: SignalParameterKind::Integer,
    default: Some(SignalParameterValue::Integer(250)),
    min: Some(SignalParameterValue::Integer(0)),
    max: None,
};

const PRICE_BAND_PARAM: SignalParameterDescriptor = SignalParameterDescriptor {
    name: "price_band",
    description: "Maximum integer price distance from point of control used by absorption logic.",
    kind: SignalParameterKind::Integer,
    default: Some(SignalParameterValue::Integer(2)),
    min: Some(SignalParameterValue::Integer(0)),
    max: None,
};

const BREAKOUT_TICKS_PARAM: SignalParameterDescriptor = SignalParameterDescriptor {
    name: "breakout_ticks",
    description: "Minimum integer price distance beyond value area required for sweep detection.",
    kind: SignalParameterKind::Integer,
    default: Some(SignalParameterValue::Integer(1)),
    min: Some(SignalParameterValue::Integer(0)),
    max: None,
};

const DELTA_MOMENTUM_PARAMS: &[SignalParameterDescriptor] = &[THRESHOLD_PARAM_100];
const VOLUME_IMBALANCE_PARAMS: &[SignalParameterDescriptor] = &[THRESHOLD_PARAM_100];
const CUMULATIVE_DELTA_PARAMS: &[SignalParameterDescriptor] = &[THRESHOLD_PARAM_250];
const ABSORPTION_PARAMS: &[SignalParameterDescriptor] = &[THRESHOLD_PARAM_150, PRICE_BAND_PARAM];
const EXHAUSTION_PARAMS: &[SignalParameterDescriptor] = &[THRESHOLD_PARAM_150];
const SWEEP_DETECTION_PARAMS: &[SignalParameterDescriptor] =
    &[THRESHOLD_PARAM_150, BREAKOUT_TICKS_PARAM];
const COMPOSITE_PARAMS: &[SignalParameterDescriptor] = &[];

const ANALYTICS_AND_QUALITY: SignalInputMask =
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

const BUILT_IN_SIGNAL_DESCRIPTORS: [SignalDescriptor; 7] = [
    DELTA_MOMENTUM_DESCRIPTOR,
    VOLUME_IMBALANCE_DESCRIPTOR,
    CUMULATIVE_DELTA_DESCRIPTOR,
    ABSORPTION_DESCRIPTOR,
    EXHAUSTION_DESCRIPTOR,
    SWEEP_DETECTION_DESCRIPTOR,
    COMPOSITE_DESCRIPTOR,
];

/// Returns descriptors for all built-in signal modules.
pub fn built_in_signal_descriptors() -> &'static [SignalDescriptor] {
    &BUILT_IN_SIGNAL_DESCRIPTORS
}

/// Finds a built-in signal descriptor by stable signal identifier.
pub fn describe_signal(id: &str) -> Option<&'static SignalDescriptor> {
    BUILT_IN_SIGNAL_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(state: SignalState, confidence_bps: u16) -> SignalSnapshot {
        SignalSnapshot {
            module_id: "test_signal_v1",
            state,
            confidence_bps,
            quality_flags: 0,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn blocks_on_quality_issues() {
        let s = DeltaMomentumSignal::default();
        let decision = s.quality_gate(DataQualityFlags::SEQUENCE_GAP);
        assert_eq!(decision, SignalGateDecision::Block);
    }

    #[test]
    fn volume_imbalance_signal_uses_session_totals() {
        let mut s = VolumeImbalanceSignal::new(10);
        s.on_analytics(&AnalyticsSnapshot {
            buy_volume: 30,
            sell_volume: 15,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "volume_imbalance_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
    }

    #[test]
    fn cumulative_delta_signal_uses_cumulative_threshold() {
        let mut s = CumulativeDeltaSignal::new(20);
        s.on_analytics(&AnalyticsSnapshot {
            cumulative_delta: -25,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "cumulative_delta_v1");
        assert_eq!(snapshot.state, SignalState::ShortBias);
    }

    #[test]
    fn absorption_signal_detects_failed_sell_push() {
        let mut s = AbsorptionSignal::new(20, 1);
        s.on_analytics(&AnalyticsSnapshot {
            delta: -25,
            last_price: 100,
            point_of_control: 100,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "absorption_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
    }

    #[test]
    fn exhaustion_signal_detects_failed_buy_follow_through() {
        let mut s = ExhaustionSignal::new(20);
        s.on_analytics(&AnalyticsSnapshot {
            delta: 25,
            last_price: 100,
            point_of_control: 101,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "exhaustion_v1");
        assert_eq!(snapshot.state, SignalState::ShortBias);
    }

    #[test]
    fn sweep_signal_detects_value_area_break() {
        let mut s = SweepDetectionSignal::new(20, 1);
        s.on_analytics(&AnalyticsSnapshot {
            delta: 30,
            last_price: 106,
            value_area_high: 104,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "sweep_detection_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
    }

    #[test]
    fn composite_signal_aggregates_child_votes() {
        let mut s = CompositeSignal::new(vec![
            Box::new(DeltaMomentumSignal::new(10)),
            Box::new(VolumeImbalanceSignal::new(10)),
            Box::new(CumulativeDeltaSignal::new(10)),
        ]);
        s.on_analytics(&AnalyticsSnapshot {
            delta: 15,
            cumulative_delta: 20,
            buy_volume: 30,
            sell_volume: 10,
            ..Default::default()
        });
        let snapshot = s.snapshot();
        assert_eq!(snapshot.module_id, "composite_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
    }

    #[test]
    fn built_in_descriptors_cover_all_default_modules() {
        let descriptors = built_in_signal_descriptors();
        assert_eq!(descriptors.len(), 7);

        let ids: Vec<&str> = descriptors.iter().map(|descriptor| descriptor.id).collect();
        assert!(ids.contains(&"delta_momentum_v1"));
        assert!(ids.contains(&"volume_imbalance_v1"));
        assert!(ids.contains(&"cumulative_delta_v1"));
        assert!(ids.contains(&"absorption_v1"));
        assert!(ids.contains(&"exhaustion_v1"));
        assert!(ids.contains(&"sweep_detection_v1"));
        assert!(ids.contains(&"composite_v1"));

        for descriptor in descriptors {
            assert!(descriptor.requires_input(SignalInputMask::ANALYTICS));
            assert!(descriptor.requires_input(SignalInputMask::DATA_QUALITY));
            assert!(descriptor.deterministic);
        }
    }

    #[test]
    fn describe_signal_finds_parameter_metadata() {
        let descriptor = describe_signal("absorption_v1").expect("descriptor exists");
        assert_eq!(descriptor.name, "Absorption");

        let threshold = descriptor
            .parameter("threshold")
            .expect("threshold parameter exists");
        assert_eq!(threshold.kind, SignalParameterKind::Integer);
        assert_eq!(threshold.default, Some(SignalParameterValue::Integer(150)));

        let price_band = descriptor
            .parameter("price_band")
            .expect("price_band parameter exists");
        assert_eq!(price_band.default, Some(SignalParameterValue::Integer(2)));
        assert!(descriptor.parameter("unknown").is_none());
        assert!(describe_signal("unknown").is_none());
    }

    #[test]
    fn signal_lifecycle_activates_after_event_warmup() {
        let mut lifecycle = SignalLifecycle::new(SignalWarmupRequirement::Events(2));
        assert_eq!(lifecycle.state(), SignalLifecycleState::WarmingUp);
        assert!(!lifecycle.is_active());

        lifecycle.record_event();
        assert_eq!(lifecycle.state(), SignalLifecycleState::WarmingUp);

        lifecycle.record_event();
        assert_eq!(lifecycle.state(), SignalLifecycleState::Active);
        assert!(lifecycle.is_active());
        assert_eq!(lifecycle.progress().events, 2);
    }

    #[test]
    fn signal_lifecycle_supports_composite_warmup_requirements() {
        const REQUIREMENTS: &[SignalWarmupRequirement] = &[
            SignalWarmupRequirement::Events(2),
            SignalWarmupRequirement::CompletedBars(1),
            SignalWarmupRequirement::MarketTimeNs(1_000),
        ];

        let mut lifecycle = SignalLifecycle::new(SignalWarmupRequirement::All(REQUIREMENTS));
        lifecycle.record_event();
        lifecycle.record_completed_bar();
        lifecycle.set_market_time_ns(1_000);
        assert_eq!(lifecycle.state(), SignalLifecycleState::WarmingUp);

        lifecycle.record_event();
        assert_eq!(lifecycle.state(), SignalLifecycleState::Active);
    }

    #[test]
    fn disabled_lifecycle_does_not_degrade_or_block() {
        let mut lifecycle = SignalLifecycle::new(SignalWarmupRequirement::None);
        lifecycle.disable();
        lifecycle.block();
        assert_eq!(lifecycle.state(), SignalLifecycleState::Disabled);
        lifecycle.degrade();
        assert_eq!(lifecycle.state(), SignalLifecycleState::Disabled);
    }

    #[test]
    fn custom_descriptor_constructors_support_external_signals() {
        const PARAMS: &[SignalParameterDescriptor] = &[SignalParameterDescriptor::integer(
            "lookback_events",
            "Number of events used by the custom signal.",
            Some(32),
            Some(1),
            Some(10_000),
        )];

        let descriptor = SignalDescriptor::new(
            "custom_signal_v1",
            "Custom Signal",
            "1",
            "Example custom signal descriptor.",
        )
        .with_required_inputs(SignalInputMask::ANALYTICS | SignalInputMask::DATA_QUALITY)
        .with_warmup(SignalWarmupRequirement::Events(32))
        .with_parameters(PARAMS)
        .with_output_semantics(SignalOutputSemantics::DirectionalBias)
        .with_deterministic(true)
        .with_checkpointable(true);

        assert_eq!(descriptor.id, "custom_signal_v1");
        assert!(descriptor.requires_input(SignalInputMask::ANALYTICS));
        assert!(descriptor.checkpointable);
        assert_eq!(
            descriptor.parameter("lookback_events").unwrap().default,
            Some(SignalParameterValue::Integer(32))
        );
    }

    #[test]
    fn signal_context_builder_attaches_optional_inputs() {
        let analytics = AnalyticsSnapshot {
            delta: 10,
            ..Default::default()
        };
        let symbol = SymbolId {
            venue: "SIM".to_string(),
            symbol: "ES".to_string(),
        };
        let book = BookSnapshot {
            symbol: symbol.clone(),
            bids: Vec::new(),
            asks: Vec::new(),
            last_sequence: 42,
            ts_exchange_ns: 100,
            ts_recv_ns: 110,
        };
        let tags = [("profile", "research")];

        let ctx = SignalContext::new(&analytics, DataQualityFlags::NONE)
            .with_symbol(&symbol)
            .with_book(&book)
            .with_timestamps(Some(100), Some(110))
            .with_lifecycle_state(SignalLifecycleState::Active)
            .with_extension_tags(&tags);

        assert_eq!(ctx.analytics.delta, 10);
        assert_eq!(ctx.symbol.unwrap().symbol, "ES");
        assert_eq!(ctx.book.unwrap().last_sequence, 42);
        assert_eq!(ctx.ts_exchange_ns, Some(100));
        assert_eq!(ctx.ts_recv_ns, Some(110));
        assert_eq!(ctx.lifecycle_state, Some(SignalLifecycleState::Active));
        assert_eq!(ctx.extension_tags, &[("profile", "research")]);
    }

    #[test]
    fn legacy_adapter_forwards_context_to_signal_module() {
        let mut signal = LegacySignalAdapter::with_descriptor(
            DeltaMomentumSignal::new(10),
            &DELTA_MOMENTUM_DESCRIPTOR,
        );
        let analytics = AnalyticsSnapshot {
            delta: 15,
            ..Default::default()
        };
        let ctx = SignalContext::new(&analytics, DataQualityFlags::NONE);

        assert_eq!(
            signal.lifecycle_state(),
            Some(SignalLifecycleState::WarmingUp)
        );
        signal.on_context(&ctx);

        let snapshot = signal.snapshot();
        assert_eq!(snapshot.module_id, "delta_momentum_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
        assert_eq!(signal.descriptor().unwrap().id, "delta_momentum_v1");
        assert_eq!(signal.lifecycle_state(), Some(SignalLifecycleState::Active));
    }

    #[test]
    fn legacy_adapter_uses_wrapped_quality_gate() {
        let signal = LegacySignalAdapter::new(DeltaMomentumSignal::default());
        let analytics = AnalyticsSnapshot::default();
        let ctx = SignalContext::new(&analytics, DataQualityFlags::SEQUENCE_GAP);

        assert_eq!(signal.quality_gate(&ctx), SignalGateDecision::Block);
    }

    #[test]
    fn legacy_adapter_can_return_inner_signal() {
        let mut adapter = LegacySignalAdapter::new(DeltaMomentumSignal::new(5));
        adapter.reset_lifecycle();
        assert_eq!(
            adapter.lifecycle_state(),
            Some(SignalLifecycleState::WarmingUp)
        );

        let inner = adapter.into_inner();
        let mut signal = inner;
        signal.on_analytics(&AnalyticsSnapshot {
            delta: -10,
            ..Default::default()
        });

        assert_eq!(signal.snapshot().state, SignalState::ShortBias);
    }

    #[test]
    fn stabilizer_accepts_immediately_when_policies_are_disabled() {
        let mut stabilizer = SignalStabilizer::new();
        let decision = stabilizer.stabilize(snapshot(SignalState::LongBias, 1), 1);

        assert!(decision.accepted);
        assert_eq!(decision.suppression_reason, SignalSuppressionReason::None);
        assert_eq!(decision.transition, SignalTransitionKind::Entry);
        assert_eq!(decision.emitted.state, SignalState::LongBias);
    }

    #[test]
    fn stabilizer_hysteresis_blocks_weak_entry() {
        let mut stabilizer = SignalStabilizer::with_policies(
            HysteresisPolicy::new(700, 0, 0),
            DebouncePolicy::default(),
            CooldownPolicy::default(),
        );

        let weak = stabilizer.stabilize(snapshot(SignalState::LongBias, 600), 1);
        assert!(!weak.accepted);
        assert_eq!(weak.suppression_reason, SignalSuppressionReason::Hysteresis);
        assert_eq!(weak.emitted.state, SignalState::Neutral);

        let strong = stabilizer.stabilize(snapshot(SignalState::LongBias, 700), 2);
        assert!(strong.accepted);
        assert_eq!(strong.emitted.state, SignalState::LongBias);
    }

    #[test]
    fn stabilizer_debounce_requires_repeated_confirmation() {
        let mut stabilizer = SignalStabilizer::with_policies(
            HysteresisPolicy::default(),
            DebouncePolicy::new(2, 0),
            CooldownPolicy::default(),
        );

        let first = stabilizer.stabilize(snapshot(SignalState::LongBias, 900), 1);
        assert!(!first.accepted);
        assert_eq!(
            first.suppression_reason,
            SignalSuppressionReason::DebouncePending
        );

        let second = stabilizer.stabilize(snapshot(SignalState::LongBias, 900), 2);
        assert!(second.accepted);
        assert_eq!(second.emitted.state, SignalState::LongBias);
    }

    #[test]
    fn stabilizer_debounce_requires_time_confirmation() {
        let mut stabilizer = SignalStabilizer::with_policies(
            HysteresisPolicy::default(),
            DebouncePolicy::new(1, 10),
            CooldownPolicy::default(),
        );

        let first = stabilizer.stabilize(snapshot(SignalState::ShortBias, 900), 100);
        assert!(!first.accepted);

        let early = stabilizer.stabilize(snapshot(SignalState::ShortBias, 900), 105);
        assert!(!early.accepted);

        let ready = stabilizer.stabilize(snapshot(SignalState::ShortBias, 900), 110);
        assert!(ready.accepted);
        assert_eq!(ready.emitted.state, SignalState::ShortBias);
    }

    #[test]
    fn stabilizer_cooldown_suppresses_reversal_after_entry() {
        let mut stabilizer = SignalStabilizer::with_policies(
            HysteresisPolicy::default(),
            DebouncePolicy::default(),
            CooldownPolicy::new(100, 0, 0),
        );

        let entry = stabilizer.stabilize(snapshot(SignalState::LongBias, 900), 1_000);
        assert!(entry.accepted);

        let reversal = stabilizer.stabilize(snapshot(SignalState::ShortBias, 900), 1_050);
        assert!(!reversal.accepted);
        assert_eq!(
            reversal.suppression_reason,
            SignalSuppressionReason::CooldownActive
        );
        assert_eq!(reversal.emitted.state, SignalState::LongBias);

        let ready = stabilizer.stabilize(snapshot(SignalState::ShortBias, 900), 1_100);
        assert!(ready.accepted);
        assert_eq!(ready.transition, SignalTransitionKind::Reversal);
    }

    #[test]
    fn stabilizer_accepts_blocked_state_without_suppression() {
        let mut stabilizer = SignalStabilizer::with_policies(
            HysteresisPolicy::new(900, 900, 900),
            DebouncePolicy::new(10, 1_000),
            CooldownPolicy::new(1_000, 1_000, 1_000),
        );

        let blocked = stabilizer.stabilize(snapshot(SignalState::Blocked, 0), 1);
        assert!(blocked.accepted);
        assert_eq!(blocked.emitted.state, SignalState::Blocked);
        assert_eq!(blocked.suppression_reason, SignalSuppressionReason::None);
    }
}
