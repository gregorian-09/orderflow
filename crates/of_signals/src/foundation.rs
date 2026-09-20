use super::*;

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
    /// Returns a structured explanation when the module supports one.
    ///
    /// The default keeps existing downstream implementations source-compatible.
    /// Hosts should keep using [`SignalModule::snapshot`] on hot paths and call
    /// this only for audit, replay, dashboard, or diagnostic flows.
    fn latest_explanation(&self) -> Option<SignalExplanation> {
        None
    }
}

impl<T: SignalModule + ?Sized> SignalModule for Box<T> {
    fn on_analytics(&mut self, ev: &AnalyticsSnapshot) {
        (**self).on_analytics(ev);
    }

    fn snapshot(&self) -> SignalSnapshot {
        (**self).snapshot()
    }

    fn quality_gate(&self, q: DataQualityFlags) -> SignalGateDecision {
        (**self).quality_gate(q)
    }

    fn latest_explanation(&self) -> Option<SignalExplanation> {
        (**self).latest_explanation()
    }
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
