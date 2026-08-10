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

/// Configuration value used when constructing a signal from a registry.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignalConfigValue<'a> {
    /// Signed integer configuration value.
    Integer(i64),
    /// Floating-point configuration value.
    Float(f64),
    /// Boolean configuration value.
    Boolean(bool),
    /// Borrowed text configuration value.
    Text(&'a str),
}

impl SignalConfigValue<'_> {
    /// Returns the parameter kind represented by this value.
    pub const fn kind(self) -> SignalParameterKind {
        match self {
            Self::Integer(_) => SignalParameterKind::Integer,
            Self::Float(_) => SignalParameterKind::Float,
            Self::Boolean(_) => SignalParameterKind::Boolean,
            Self::Text(_) => SignalParameterKind::Text,
        }
    }
}

/// One named parameter supplied in a signal configuration.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalConfigParameter<'a> {
    /// Stable parameter name.
    pub name: &'a str,
    /// Supplied parameter value.
    pub value: SignalConfigValue<'a>,
}

impl<'a> SignalConfigParameter<'a> {
    /// Creates a named signal configuration parameter.
    pub const fn new(name: &'a str, value: SignalConfigValue<'a>) -> Self {
        Self { name, value }
    }

    /// Creates an integer signal configuration parameter.
    pub const fn integer(name: &'a str, value: i64) -> Self {
        Self::new(name, SignalConfigValue::Integer(value))
    }

    /// Creates a floating-point signal configuration parameter.
    pub const fn float(name: &'a str, value: f64) -> Self {
        Self::new(name, SignalConfigValue::Float(value))
    }

    /// Creates a boolean signal configuration parameter.
    pub const fn boolean(name: &'a str, value: bool) -> Self {
        Self::new(name, SignalConfigValue::Boolean(value))
    }

    /// Creates a text signal configuration parameter.
    pub const fn text(name: &'a str, value: &'a str) -> Self {
        Self::new(name, SignalConfigValue::Text(value))
    }
}

/// Borrowed signal configuration for registry validation and construction.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalConfig<'a> {
    /// Stable signal id.
    pub id: &'a str,
    /// Supplied configuration parameters.
    pub parameters: &'a [SignalConfigParameter<'a>],
}

impl<'a> SignalConfig<'a> {
    /// Creates a signal configuration with no parameters.
    pub const fn new(id: &'a str) -> Self {
        Self {
            id,
            parameters: &[],
        }
    }

    /// Creates a signal configuration with explicit parameters.
    pub const fn with_parameters(id: &'a str, parameters: &'a [SignalConfigParameter<'a>]) -> Self {
        Self { id, parameters }
    }

    /// Finds a supplied parameter by name.
    pub fn parameter(&self, name: &str) -> Option<SignalConfigValue<'a>> {
        self.parameters
            .iter()
            .find(|parameter| parameter.name == name)
            .map(|parameter| parameter.value)
    }
}

/// Error returned by signal registry validation or construction.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum SignalRegistryError {
    /// The requested signal id is not registered.
    UnknownSignal {
        /// Requested signal id.
        id: String,
    },
    /// A signal id was registered more than once.
    DuplicateSignal {
        /// Duplicate signal id.
        id: &'static str,
    },
    /// A configuration supplied the same parameter more than once.
    DuplicateParameter {
        /// Signal id being validated.
        signal_id: &'static str,
        /// Duplicate parameter name.
        name: String,
    },
    /// A configuration supplied a parameter unknown to the signal descriptor.
    UnknownParameter {
        /// Signal id being validated.
        signal_id: &'static str,
        /// Unknown parameter name.
        name: String,
    },
    /// A configuration supplied a value with the wrong type.
    InvalidParameterType {
        /// Signal id being validated.
        signal_id: &'static str,
        /// Parameter name.
        name: &'static str,
        /// Expected kind.
        expected: SignalParameterKind,
        /// Actual kind.
        actual: SignalParameterKind,
    },
    /// A configuration supplied a value below the descriptor minimum.
    ParameterBelowMinimum {
        /// Signal id being validated.
        signal_id: &'static str,
        /// Parameter name.
        name: &'static str,
        /// Minimum allowed value.
        min: SignalParameterValue,
        /// Supplied value.
        actual: SignalConfigValue<'static>,
    },
    /// A configuration supplied a value above the descriptor maximum.
    ParameterAboveMaximum {
        /// Signal id being validated.
        signal_id: &'static str,
        /// Parameter name.
        name: &'static str,
        /// Maximum allowed value.
        max: SignalParameterValue,
        /// Supplied value.
        actual: SignalConfigValue<'static>,
    },
    /// The registered signal has no factory.
    MissingFactory {
        /// Signal id being constructed.
        signal_id: &'static str,
    },
}

impl std::fmt::Display for SignalRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSignal { id } => write!(f, "unknown signal id `{id}`"),
            Self::DuplicateSignal { id } => write!(f, "duplicate signal id `{id}`"),
            Self::DuplicateParameter { signal_id, name } => {
                write!(f, "duplicate parameter `{name}` for signal `{signal_id}`")
            }
            Self::UnknownParameter { signal_id, name } => {
                write!(f, "unknown parameter `{name}` for signal `{signal_id}`")
            }
            Self::InvalidParameterType {
                signal_id,
                name,
                expected,
                actual,
            } => write!(
                f,
                "invalid parameter type for `{signal_id}.{name}`: expected {expected:?}, got {actual:?}"
            ),
            Self::ParameterBelowMinimum {
                signal_id, name, ..
            } => write!(
                f,
                "parameter `{signal_id}.{name}` is below the descriptor minimum"
            ),
            Self::ParameterAboveMaximum {
                signal_id, name, ..
            } => write!(
                f,
                "parameter `{signal_id}.{name}` is above the descriptor maximum"
            ),
            Self::MissingFactory { signal_id } => {
                write!(f, "signal `{signal_id}` has no construction factory")
            }
        }
    }
}

impl std::error::Error for SignalRegistryError {}

/// Result returned by signal registry operations.
pub type SignalRegistryResult<T> = Result<T, SignalRegistryError>;

/// Factory function used by [`SignalRegistry`] to build a signal module.
pub type SignalFactory = fn(&SignalConfig<'_>) -> SignalRegistryResult<Box<dyn SignalModule>>;

/// One signal registration containing descriptor metadata and optional factory.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct SignalRegistration {
    /// Static signal descriptor.
    pub descriptor: &'static SignalDescriptor,
    /// Optional factory for constructing the signal from config.
    pub factory: Option<SignalFactory>,
}

impl SignalRegistration {
    /// Creates a signal registration.
    pub const fn new(
        descriptor: &'static SignalDescriptor,
        factory: Option<SignalFactory>,
    ) -> Self {
        Self {
            descriptor,
            factory,
        }
    }
}

/// Registry for discovering, validating, and constructing signal modules.
///
/// The registry is intended for startup/configuration paths, dashboards, and
/// bindings. Signal evaluation remains owned by concrete modules and the
/// `SignalModule` trait.
#[derive(Debug, Clone)]
pub struct SignalRegistry {
    registrations: Vec<SignalRegistration>,
}

impl SignalRegistry {
    /// Creates an empty signal registry.
    pub const fn new() -> Self {
        Self {
            registrations: Vec::new(),
        }
    }

    /// Creates a registry containing the built-in signal modules.
    pub fn with_built_ins() -> Self {
        Self {
            registrations: built_in_signal_registrations().to_vec(),
        }
    }

    /// Adds a signal registration.
    pub fn register(
        &mut self,
        registration: SignalRegistration,
    ) -> SignalRegistryResult<&mut Self> {
        if self
            .registrations
            .iter()
            .any(|existing| existing.descriptor.id == registration.descriptor.id)
        {
            return Err(SignalRegistryError::DuplicateSignal {
                id: registration.descriptor.id,
            });
        }
        self.registrations.push(registration);
        Ok(self)
    }

    /// Returns registered signal metadata.
    pub fn registrations(&self) -> &[SignalRegistration] {
        &self.registrations
    }

    /// Finds a registered signal descriptor by id.
    pub fn descriptor(&self, id: &str) -> Option<&'static SignalDescriptor> {
        self.registration(id)
            .map(|registration| registration.descriptor)
    }

    /// Returns descriptors whose required inputs are included in `available_inputs`.
    pub fn descriptors_matching_inputs(
        &self,
        available_inputs: SignalInputMask,
    ) -> Vec<&'static SignalDescriptor> {
        self.registrations
            .iter()
            .filter_map(|registration| {
                available_inputs
                    .contains(registration.descriptor.required_inputs)
                    .then_some(registration.descriptor)
            })
            .collect()
    }

    /// Validates a signal configuration without constructing the module.
    pub fn validate_config(&self, config: &SignalConfig<'_>) -> SignalRegistryResult<()> {
        let descriptor =
            self.descriptor(config.id)
                .ok_or_else(|| SignalRegistryError::UnknownSignal {
                    id: config.id.to_string(),
                })?;
        validate_signal_config(descriptor, config)
    }

    /// Validates a configuration and returns a stable JSON result for bindings.
    ///
    /// Registry validation failures are represented by `valid: false` in the
    /// returned document. The method itself does not panic or discard the
    /// diagnostic message.
    pub fn validate_config_json(&self, config: &SignalConfig<'_>) -> String {
        let result = self.validate_config(config);
        let mut out = String::from("{\"schema_version\":1,\"signal_id\":");
        push_json_string(&mut out, config.id);
        out.push_str(",\"valid\":");
        out.push_str(if result.is_ok() { "true" } else { "false" });
        out.push_str(",\"error\":");
        match result {
            Ok(()) => out.push_str("null"),
            Err(error) => push_json_string(&mut out, &error.to_string()),
        }
        out.push('}');
        out
    }

    /// Constructs a signal module from configuration.
    pub fn create_signal(
        &self,
        config: &SignalConfig<'_>,
    ) -> SignalRegistryResult<Box<dyn SignalModule>> {
        let registration =
            self.registration(config.id)
                .ok_or_else(|| SignalRegistryError::UnknownSignal {
                    id: config.id.to_string(),
                })?;
        self.validate_config(config)?;
        let Some(factory) = registration.factory else {
            return Err(SignalRegistryError::MissingFactory {
                signal_id: registration.descriptor.id,
            });
        };
        factory(config)
    }

    /// Exports registered descriptors as compact JSON for bindings and dashboards.
    pub fn descriptors_json(&self) -> String {
        let mut out = String::from("[");
        for (index, registration) in self.registrations.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_descriptor_json(&mut out, registration.descriptor);
        }
        out.push(']');
        out
    }

    fn registration(&self, id: &str) -> Option<&SignalRegistration> {
        self.registrations
            .iter()
            .find(|registration| registration.descriptor.id == id)
    }
}

impl Default for SignalRegistry {
    fn default() -> Self {
        Self::with_built_ins()
    }
}

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

fn score_direction(
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

fn ratio_bps(numerator: usize, denominator: usize) -> Option<u16> {
    let ratio = (numerator as u128)
        .saturating_mul(10_000)
        .checked_div(denominator as u128)?;
    Some(ratio.min(10_000) as u16)
}

fn average_bps(sum: u64, samples: usize) -> u16 {
    if samples == 0 {
        0
    } else {
        (sum / samples as u64) as u16
    }
}

fn abs_diff_bps(left: u16, right: u16) -> u16 {
    left.abs_diff(right)
}

fn push_optional_u16_json(out: &mut String, value: Option<u16>) {
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

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

/// Current schema version for signal checkpoints.
pub const SIGNAL_CHECKPOINT_SCHEMA_VERSION: u16 = 1;

/// Versioned signal checkpoint metadata and payload.
///
/// The core type is intentionally generic: the signal crate validates stable
/// identity/config metadata, while individual signal implementations own their
/// payload encoding.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SignalCheckpoint {
    /// Checkpoint schema version.
    pub schema_version: u16,
    /// Stable signal module id.
    pub module_id: String,
    /// Signal implementation or descriptor version.
    pub signal_version: String,
    /// Host-defined signal configuration hash.
    pub config_hash: u64,
    /// Optional symbol associated with the checkpoint.
    pub symbol: Option<SymbolId>,
    /// Last emitted state captured in the checkpoint.
    pub state: SignalState,
    /// Last emitted confidence in basis points.
    pub confidence_bps: u16,
    /// Last emitted quality flags.
    pub quality_flags: u32,
    /// Last emitted reason text.
    pub reason: String,
    /// Optional lifecycle state captured by the host.
    pub lifecycle_state: Option<SignalLifecycleState>,
    /// Optional calibration artifact id used by the signal.
    pub calibration_id: Option<u64>,
    /// Checkpoint creation timestamp in nanoseconds.
    pub created_at_ns: u64,
    /// Last signal update timestamp represented by this checkpoint.
    pub last_update_ns: u64,
    /// Opaque signal-owned payload bytes.
    pub payload: Vec<u8>,
}

impl SignalCheckpoint {
    /// Creates checkpoint metadata for a signal state.
    pub fn new(
        module_id: impl Into<String>,
        signal_version: impl Into<String>,
        state: SignalState,
    ) -> Self {
        Self {
            schema_version: SIGNAL_CHECKPOINT_SCHEMA_VERSION,
            module_id: module_id.into(),
            signal_version: signal_version.into(),
            config_hash: 0,
            symbol: None,
            state,
            confidence_bps: 0,
            quality_flags: 0,
            reason: String::new(),
            lifecycle_state: None,
            calibration_id: None,
            created_at_ns: 0,
            last_update_ns: 0,
            payload: Vec::new(),
        }
    }

    /// Creates checkpoint metadata from a signal snapshot.
    pub fn from_snapshot(snapshot: &SignalSnapshot, signal_version: impl Into<String>) -> Self {
        Self {
            schema_version: SIGNAL_CHECKPOINT_SCHEMA_VERSION,
            module_id: snapshot.module_id.to_string(),
            signal_version: signal_version.into(),
            config_hash: 0,
            symbol: None,
            state: snapshot.state,
            confidence_bps: snapshot.confidence_bps.min(10_000),
            quality_flags: snapshot.quality_flags,
            reason: snapshot.reason.clone(),
            lifecycle_state: None,
            calibration_id: None,
            created_at_ns: 0,
            last_update_ns: 0,
            payload: Vec::new(),
        }
    }

    /// Returns this checkpoint with a configuration hash.
    pub const fn with_config_hash(mut self, config_hash: u64) -> Self {
        self.config_hash = config_hash;
        self
    }

    /// Returns this checkpoint with a symbol.
    pub fn with_symbol(mut self, symbol: SymbolId) -> Self {
        self.symbol = Some(symbol);
        self
    }

    /// Returns this checkpoint with lifecycle state.
    pub const fn with_lifecycle_state(mut self, lifecycle_state: SignalLifecycleState) -> Self {
        self.lifecycle_state = Some(lifecycle_state);
        self
    }

    /// Returns this checkpoint with calibration id.
    pub const fn with_calibration_id(mut self, calibration_id: u64) -> Self {
        self.calibration_id = Some(calibration_id);
        self
    }

    /// Returns this checkpoint with creation and last-update timestamps.
    pub const fn with_timestamps(mut self, created_at_ns: u64, last_update_ns: u64) -> Self {
        self.created_at_ns = created_at_ns;
        self.last_update_ns = last_update_ns;
        self
    }

    /// Returns this checkpoint with opaque payload bytes.
    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self
    }
}

/// Restore-time validation policy for a signal checkpoint.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCheckpointRestorePolicy {
    /// Expected signal module id.
    pub expected_module_id: Option<String>,
    /// Expected signal implementation/descriptor version.
    pub expected_signal_version: Option<String>,
    /// Expected config hash.
    pub expected_config_hash: Option<u64>,
    /// Expected symbol.
    pub expected_symbol: Option<SymbolId>,
    /// Minimum accepted checkpoint schema version.
    pub min_schema_version: u16,
    /// Maximum accepted checkpoint schema version.
    pub max_schema_version: u16,
    /// Minimum accepted last-update timestamp.
    pub min_last_update_ns: Option<u64>,
    /// Expected calibration artifact id.
    pub expected_calibration_id: Option<u64>,
}

impl SignalCheckpointRestorePolicy {
    /// Creates a restore policy that accepts the current checkpoint schema.
    pub const fn new() -> Self {
        Self {
            expected_module_id: None,
            expected_signal_version: None,
            expected_config_hash: None,
            expected_symbol: None,
            min_schema_version: SIGNAL_CHECKPOINT_SCHEMA_VERSION,
            max_schema_version: SIGNAL_CHECKPOINT_SCHEMA_VERSION,
            min_last_update_ns: None,
            expected_calibration_id: None,
        }
    }

    /// Returns this policy with expected signal identity.
    pub fn with_signal(mut self, module_id: impl Into<String>, version: impl Into<String>) -> Self {
        self.expected_module_id = Some(module_id.into());
        self.expected_signal_version = Some(version.into());
        self
    }

    /// Returns this policy with an expected config hash.
    pub const fn with_config_hash(mut self, config_hash: u64) -> Self {
        self.expected_config_hash = Some(config_hash);
        self
    }

    /// Returns this policy with an expected symbol.
    pub fn with_symbol(mut self, symbol: SymbolId) -> Self {
        self.expected_symbol = Some(symbol);
        self
    }

    /// Returns this policy with an accepted schema version range.
    pub const fn with_schema_range(
        mut self,
        min_schema_version: u16,
        max_schema_version: u16,
    ) -> Self {
        self.min_schema_version = min_schema_version;
        self.max_schema_version = max_schema_version;
        self
    }

    /// Returns this policy with a minimum last-update timestamp.
    pub const fn with_min_last_update_ns(mut self, min_last_update_ns: u64) -> Self {
        self.min_last_update_ns = Some(min_last_update_ns);
        self
    }

    /// Returns this policy with an expected calibration id.
    pub const fn with_calibration_id(mut self, calibration_id: u64) -> Self {
        self.expected_calibration_id = Some(calibration_id);
        self
    }
}

impl Default for SignalCheckpointRestorePolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// One restore validation issue for a signal checkpoint.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalCheckpointValidationIssue {
    /// Checkpoint schema version was outside the accepted range.
    SchemaVersionOutOfRange {
        /// Minimum accepted schema version.
        min: u16,
        /// Maximum accepted schema version.
        max: u16,
        /// Actual checkpoint schema version.
        actual: u16,
    },
    /// Signal module id did not match.
    ModuleIdMismatch {
        /// Expected module id.
        expected: String,
        /// Actual module id.
        actual: String,
    },
    /// Signal version did not match.
    SignalVersionMismatch {
        /// Expected signal version.
        expected: String,
        /// Actual signal version.
        actual: String,
    },
    /// Signal config hash did not match.
    ConfigHashMismatch {
        /// Expected config hash.
        expected: u64,
        /// Actual config hash.
        actual: u64,
    },
    /// Symbol did not match.
    SymbolMismatch {
        /// Expected symbol.
        expected: SymbolId,
        /// Actual symbol.
        actual: Option<SymbolId>,
    },
    /// Calibration id did not match.
    CalibrationMismatch {
        /// Expected calibration id.
        expected: u64,
        /// Actual calibration id.
        actual: Option<u64>,
    },
    /// Checkpoint timestamp was older than allowed.
    NonMonotonicTimestamp {
        /// Minimum accepted last-update timestamp.
        min_last_update_ns: u64,
        /// Actual checkpoint last-update timestamp.
        checkpoint_last_update_ns: u64,
    },
}

/// Restore validation report for a signal checkpoint.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCheckpointValidationReport {
    /// Whether restore validation passed.
    pub valid: bool,
    /// Validation issues.
    pub issues: Vec<SignalCheckpointValidationIssue>,
}

impl SignalCheckpointValidationReport {
    /// Returns `true` when validation failed.
    pub const fn has_errors(&self) -> bool {
        !self.valid
    }
}

/// Error returned by checkpoint-aware signal restore operations.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalCheckpointRestoreError {
    /// Checkpoint metadata failed restore validation.
    InvalidCheckpoint(SignalCheckpointValidationReport),
    /// Checkpoint payload is not supported by this signal implementation.
    UnsupportedPayload,
}

/// Optional extension trait for signals that support checkpoint restore.
///
/// This is intentionally separate from [`SignalModule`] so existing downstream
/// signal implementations remain source-compatible.
pub trait CheckpointableSignal: SignalModule {
    /// Returns a checkpoint for the current signal state.
    fn checkpoint(&self) -> SignalCheckpoint;

    /// Restores signal state from a previously validated checkpoint.
    fn restore_checkpoint(
        &mut self,
        checkpoint: &SignalCheckpoint,
    ) -> Result<(), SignalCheckpointRestoreError>;
}

/// Validates checkpoint metadata against a restore policy.
pub fn validate_signal_checkpoint_restore(
    checkpoint: &SignalCheckpoint,
    policy: &SignalCheckpointRestorePolicy,
) -> SignalCheckpointValidationReport {
    let mut issues = Vec::new();

    if checkpoint.schema_version < policy.min_schema_version
        || checkpoint.schema_version > policy.max_schema_version
    {
        issues.push(SignalCheckpointValidationIssue::SchemaVersionOutOfRange {
            min: policy.min_schema_version,
            max: policy.max_schema_version,
            actual: checkpoint.schema_version,
        });
    }

    if let Some(expected) = &policy.expected_module_id {
        if checkpoint.module_id != *expected {
            issues.push(SignalCheckpointValidationIssue::ModuleIdMismatch {
                expected: expected.clone(),
                actual: checkpoint.module_id.clone(),
            });
        }
    }

    if let Some(expected) = &policy.expected_signal_version {
        if checkpoint.signal_version != *expected {
            issues.push(SignalCheckpointValidationIssue::SignalVersionMismatch {
                expected: expected.clone(),
                actual: checkpoint.signal_version.clone(),
            });
        }
    }

    if let Some(expected) = policy.expected_config_hash {
        if checkpoint.config_hash != expected {
            issues.push(SignalCheckpointValidationIssue::ConfigHashMismatch {
                expected,
                actual: checkpoint.config_hash,
            });
        }
    }

    if let Some(expected) = &policy.expected_symbol {
        if checkpoint.symbol.as_ref() != Some(expected) {
            issues.push(SignalCheckpointValidationIssue::SymbolMismatch {
                expected: expected.clone(),
                actual: checkpoint.symbol.clone(),
            });
        }
    }

    if let Some(expected) = policy.expected_calibration_id {
        if checkpoint.calibration_id != Some(expected) {
            issues.push(SignalCheckpointValidationIssue::CalibrationMismatch {
                expected,
                actual: checkpoint.calibration_id,
            });
        }
    }

    if let Some(min_last_update_ns) = policy.min_last_update_ns {
        if checkpoint.last_update_ns < min_last_update_ns {
            issues.push(SignalCheckpointValidationIssue::NonMonotonicTimestamp {
                min_last_update_ns,
                checkpoint_last_update_ns: checkpoint.last_update_ns,
            });
        }
    }

    SignalCheckpointValidationReport {
        valid: issues.is_empty(),
        issues,
    }
}

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

/// Quality flags attached to one feature value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FeatureQualityFlags(u32);

impl FeatureQualityFlags {
    /// No feature-quality issues.
    pub const NONE: Self = Self(0);
    /// Feature value is missing.
    pub const MISSING: Self = Self(1 << 0);
    /// Feature value is stale relative to the schema freshness policy.
    pub const STALE: Self = Self(1 << 1);
    /// Feature value is outside the descriptor range.
    pub const OUT_OF_RANGE: Self = Self(1 << 2);
    /// Feature value was imputed.
    pub const IMPUTED: Self = Self(1 << 3);
    /// Feature pipeline marked the value as degraded.
    pub const DEGRADED: Self = Self(1 << 4);

    /// Returns raw flag bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Creates flags from raw bits, ignoring unknown bits.
    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self(bits & Self::all_bits())
    }

    /// Returns true when all `other` flags are present.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns true when any `other` flag is present.
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    /// Returns the union of two flag sets.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    const fn all_bits() -> u32 {
        Self::MISSING.0 | Self::STALE.0 | Self::OUT_OF_RANGE.0 | Self::IMPUTED.0 | Self::DEGRADED.0
    }
}

impl BitOr for FeatureQualityFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl BitOrAssign for FeatureQualityFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

/// Semantic kind for one feature value.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FeatureValueKind {
    /// Generic floating-point feature.
    #[default]
    Float,
    /// Integer-valued feature encoded as `f64` in a vector.
    Integer,
    /// Boolean feature encoded as `0.0` or `1.0`.
    Boolean,
    /// Price-normalized feature.
    Price,
    /// Size/quantity feature.
    Size,
    /// Basis-point feature.
    BasisPoints,
}

/// Missing-value policy for a feature descriptor.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FeatureMissingPolicy {
    /// Reject vectors where this feature is missing.
    #[default]
    Reject,
    /// Treat missing values as zero.
    TreatAsZero,
    /// Use the configured default value.
    UseDefault(f64),
    /// Keep the value unavailable for downstream model code.
    MarkUnavailable,
}

/// One feature in a signal/model feature schema.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureDescriptor {
    /// Stable feature id.
    pub id: String,
    /// Feature value kind.
    pub value_kind: FeatureValueKind,
    /// Human-readable unit name.
    pub unit: String,
    /// Human-readable description.
    pub description: String,
    /// Missing-value handling policy.
    pub missing_policy: FeatureMissingPolicy,
    /// Optional minimum accepted value.
    pub min_value: Option<f64>,
    /// Optional maximum accepted value.
    pub max_value: Option<f64>,
    /// Optional freshness limit in nanoseconds.
    pub freshness_ns: Option<u64>,
}

impl FeatureDescriptor {
    /// Creates a feature descriptor with conservative defaults.
    pub fn new(id: impl Into<String>, value_kind: FeatureValueKind) -> Self {
        Self {
            id: id.into(),
            value_kind,
            unit: String::new(),
            description: String::new(),
            missing_policy: FeatureMissingPolicy::Reject,
            min_value: None,
            max_value: None,
            freshness_ns: None,
        }
    }

    /// Returns this descriptor with unit metadata.
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Returns this descriptor with a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Returns this descriptor with a missing-value policy.
    pub const fn with_missing_policy(mut self, missing_policy: FeatureMissingPolicy) -> Self {
        self.missing_policy = missing_policy;
        self
    }

    /// Returns this descriptor with an inclusive value range.
    pub const fn with_range(mut self, min_value: f64, max_value: f64) -> Self {
        self.min_value = Some(min_value);
        self.max_value = Some(max_value);
        self
    }

    /// Returns this descriptor with a freshness limit.
    pub const fn with_freshness_ns(mut self, freshness_ns: u64) -> Self {
        self.freshness_ns = Some(freshness_ns);
        self
    }
}

/// Stable feature schema used by feature-vector and model-backed signals.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureSchema {
    /// Stable schema id.
    pub id: String,
    /// Schema version.
    pub version: String,
    /// Host-defined config hash for the schema.
    pub config_hash: u64,
    /// Ordered feature descriptors.
    pub features: Vec<FeatureDescriptor>,
}

impl FeatureSchema {
    /// Creates an empty feature schema.
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            config_hash: 0,
            features: Vec::new(),
        }
    }

    /// Returns this schema with a config hash.
    pub const fn with_config_hash(mut self, config_hash: u64) -> Self {
        self.config_hash = config_hash;
        self
    }

    /// Returns this schema with an appended feature descriptor.
    pub fn with_feature(mut self, feature: FeatureDescriptor) -> Self {
        self.features.push(feature);
        self
    }

    /// Appends a feature descriptor.
    pub fn push_feature(&mut self, feature: FeatureDescriptor) {
        self.features.push(feature);
    }

    /// Returns the index for a feature id.
    pub fn feature_index(&self, id: &str) -> Option<usize> {
        self.features.iter().position(|feature| feature.id == id)
    }

    /// Returns a feature descriptor by id.
    pub fn feature(&self, id: &str) -> Option<&FeatureDescriptor> {
        self.feature_index(id)
            .and_then(|index| self.features.get(index))
    }
}

/// Borrowed feature vector plus schema and per-feature quality flags.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct FeatureVectorView<'a> {
    /// Schema describing value order and semantics.
    pub schema: &'a FeatureSchema,
    /// Feature values in schema order.
    pub values: &'a [f64],
    /// Per-feature quality flags in schema order.
    pub quality: &'a [FeatureQualityFlags],
    /// Event/inference timestamp in nanoseconds.
    pub timestamp_ns: u64,
}

impl<'a> FeatureVectorView<'a> {
    /// Creates a borrowed feature vector view.
    pub const fn new(
        schema: &'a FeatureSchema,
        values: &'a [f64],
        quality: &'a [FeatureQualityFlags],
        timestamp_ns: u64,
    ) -> Self {
        Self {
            schema,
            values,
            quality,
            timestamp_ns,
        }
    }

    /// Returns a feature value by id.
    pub fn value(&self, id: &str) -> Option<f64> {
        self.schema
            .feature_index(id)
            .and_then(|index| self.values.get(index).copied())
    }

    /// Returns feature quality flags by id.
    pub fn quality(&self, id: &str) -> Option<FeatureQualityFlags> {
        self.schema
            .feature_index(id)
            .and_then(|index| self.quality.get(index).copied())
    }

    /// Validates this feature vector against its schema.
    pub fn validate(&self, now_ns: Option<u64>) -> FeatureVectorValidationReport {
        validate_feature_vector(self, now_ns)
    }
}

/// One feature-vector validation issue.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureVectorValidationIssue {
    /// Value/quality slices do not match schema length.
    LengthMismatch {
        /// Number of descriptors in the schema.
        expected: usize,
        /// Number of values supplied.
        values: usize,
        /// Number of quality entries supplied.
        quality: usize,
    },
    /// A feature was missing while its descriptor requires a value.
    MissingFeature {
        /// Feature index.
        index: usize,
        /// Feature id.
        feature_id: String,
    },
    /// Feature vector timestamp exceeded the descriptor freshness limit.
    StaleFeature {
        /// Feature index.
        index: usize,
        /// Feature id.
        feature_id: String,
        /// Observed age in nanoseconds.
        age_ns: u64,
        /// Accepted freshness in nanoseconds.
        freshness_ns: u64,
    },
    /// Feature value was outside the accepted descriptor range.
    OutOfRange {
        /// Feature index.
        index: usize,
        /// Feature id.
        feature_id: String,
        /// Feature value.
        value: f64,
        /// Minimum accepted value.
        min: Option<f64>,
        /// Maximum accepted value.
        max: Option<f64>,
    },
}

/// Validation report for a feature vector.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureVectorValidationReport {
    /// Whether validation passed.
    pub valid: bool,
    /// Validation issues.
    pub issues: Vec<FeatureVectorValidationIssue>,
    /// Aggregate quality flags observed in the vector.
    pub aggregate_quality: FeatureQualityFlags,
}

impl FeatureVectorValidationReport {
    /// Returns `true` when validation found issues.
    pub const fn has_errors(&self) -> bool {
        !self.valid
    }
}

/// Validates a feature vector view against schema metadata.
pub fn validate_feature_vector(
    view: &FeatureVectorView<'_>,
    now_ns: Option<u64>,
) -> FeatureVectorValidationReport {
    let expected = view.schema.features.len();
    let mut issues = Vec::new();
    let mut aggregate_quality = FeatureQualityFlags::NONE;

    if view.values.len() != expected || view.quality.len() != expected {
        issues.push(FeatureVectorValidationIssue::LengthMismatch {
            expected,
            values: view.values.len(),
            quality: view.quality.len(),
        });
    }

    for (index, descriptor) in view.schema.features.iter().enumerate() {
        let quality = view
            .quality
            .get(index)
            .copied()
            .unwrap_or(FeatureQualityFlags::MISSING);
        aggregate_quality |= quality;

        if quality.contains(FeatureQualityFlags::MISSING)
            && descriptor.missing_policy == FeatureMissingPolicy::Reject
        {
            issues.push(FeatureVectorValidationIssue::MissingFeature {
                index,
                feature_id: descriptor.id.clone(),
            });
        }

        if let Some(value) = view.values.get(index).copied() {
            let below_min = descriptor.min_value.is_some_and(|min| value < min);
            let above_max = descriptor.max_value.is_some_and(|max| value > max);
            if below_min || above_max {
                issues.push(FeatureVectorValidationIssue::OutOfRange {
                    index,
                    feature_id: descriptor.id.clone(),
                    value,
                    min: descriptor.min_value,
                    max: descriptor.max_value,
                });
                aggregate_quality |= FeatureQualityFlags::OUT_OF_RANGE;
            }
        }

        if let (Some(now_ns), Some(freshness_ns)) = (now_ns, descriptor.freshness_ns) {
            let age_ns = now_ns.saturating_sub(view.timestamp_ns);
            if age_ns > freshness_ns {
                issues.push(FeatureVectorValidationIssue::StaleFeature {
                    index,
                    feature_id: descriptor.id.clone(),
                    age_ns,
                    freshness_ns,
                });
                aggregate_quality |= FeatureQualityFlags::STALE;
            }
        }
    }

    FeatureVectorValidationReport {
        valid: issues.is_empty(),
        issues,
        aggregate_quality,
    }
}

/// Supported model artifact/runtime family.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SignalModelKind {
    /// Model kind is not specified.
    #[default]
    Unknown,
    /// Host-provided native model implementation.
    Native,
    /// ONNX model artifact.
    Onnx,
    /// Linear model coefficients interpreted by the host.
    Linear,
    /// Tree or boosted-tree model artifact.
    TreeEnsemble,
    /// External service or process.
    External,
}

/// Model output semantics for model-backed signals.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SignalModelOutputKind {
    /// Model returns directional state and confidence directly.
    #[default]
    DirectionalState,
    /// Model returns up/down/flat probabilities.
    DirectionalProbabilities,
    /// Model returns a continuous score.
    Score,
}

/// Metadata describing a model-backed signal artifact.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct SignalModelMetadata {
    /// Stable model id.
    pub model_id: String,
    /// Model version.
    pub model_version: String,
    /// Model artifact/runtime kind.
    pub model_kind: SignalModelKind,
    /// Feature schema id expected by the model.
    pub feature_schema_id: String,
    /// Feature schema version expected by the model.
    pub feature_schema_version: String,
    /// Optional model artifact hash.
    pub artifact_hash: Option<String>,
    /// Optional training window start timestamp.
    pub training_start_ns: Option<u64>,
    /// Optional training window end timestamp.
    pub training_end_ns: Option<u64>,
    /// Optional calibration id.
    pub calibration_id: Option<u64>,
    /// Output semantics.
    pub output_kind: SignalModelOutputKind,
    /// Whether inference should be deterministic for identical input vectors.
    pub deterministic: bool,
}

impl SignalModelMetadata {
    /// Creates model metadata.
    pub fn new(
        model_id: impl Into<String>,
        model_version: impl Into<String>,
        feature_schema_id: impl Into<String>,
        feature_schema_version: impl Into<String>,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            model_version: model_version.into(),
            model_kind: SignalModelKind::Unknown,
            feature_schema_id: feature_schema_id.into(),
            feature_schema_version: feature_schema_version.into(),
            artifact_hash: None,
            training_start_ns: None,
            training_end_ns: None,
            calibration_id: None,
            output_kind: SignalModelOutputKind::DirectionalState,
            deterministic: true,
        }
    }

    /// Returns metadata with a model kind.
    pub const fn with_model_kind(mut self, model_kind: SignalModelKind) -> Self {
        self.model_kind = model_kind;
        self
    }

    /// Returns metadata with artifact hash.
    pub fn with_artifact_hash(mut self, artifact_hash: impl Into<String>) -> Self {
        self.artifact_hash = Some(artifact_hash.into());
        self
    }

    /// Returns metadata with training window timestamps.
    pub const fn with_training_window(mut self, start_ns: u64, end_ns: u64) -> Self {
        self.training_start_ns = Some(start_ns);
        self.training_end_ns = Some(end_ns);
        self
    }

    /// Returns metadata with calibration id.
    pub const fn with_calibration_id(mut self, calibration_id: u64) -> Self {
        self.calibration_id = Some(calibration_id);
        self
    }

    /// Returns metadata with output kind.
    pub const fn with_output_kind(mut self, output_kind: SignalModelOutputKind) -> Self {
        self.output_kind = output_kind;
        self
    }
}

/// Input binding between a model and a feature schema.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalModelInputBinding {
    /// Model input name.
    pub input_name: String,
    /// Ordered feature ids required by this input.
    pub feature_ids: Vec<String>,
}

impl SignalModelInputBinding {
    /// Creates a model input binding.
    pub fn new(input_name: impl Into<String>, feature_ids: Vec<String>) -> Self {
        Self {
            input_name: input_name.into(),
            feature_ids,
        }
    }

    /// Returns `true` when all bound feature ids exist in the schema.
    pub fn is_compatible_with(&self, schema: &FeatureSchema) -> bool {
        self.feature_ids
            .iter()
            .all(|feature_id| schema.feature_index(feature_id).is_some())
    }
}

/// Output returned by model-backed signal inference.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct SignalModelOutput {
    /// Signal state produced by the model.
    pub state: SignalState,
    /// Confidence in basis points.
    pub confidence_bps: u16,
    /// Optional continuous model score.
    pub score: Option<f64>,
    /// Human-readable model reason.
    pub reason: String,
}

impl SignalModelOutput {
    /// Creates a model output from state and confidence.
    pub fn new(state: SignalState, confidence_bps: u16) -> Self {
        Self {
            state,
            confidence_bps: confidence_bps.min(10_000),
            score: None,
            reason: String::new(),
        }
    }

    /// Returns this output with a score.
    pub const fn with_score(mut self, score: f64) -> Self {
        self.score = Some(score);
        self
    }

    /// Returns this output with reason text.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }
}

/// Optional extension trait for model-backed signal implementations.
///
/// This trait does not prescribe a runtime such as ONNX or TensorRT. Hosts and
/// optional crates can implement inference while the core `of_signals` crate
/// remains dependency-light.
pub trait ModelBackedSignal: SignalModule {
    /// Returns model artifact metadata.
    fn model_metadata(&self) -> &SignalModelMetadata;

    /// Returns the feature schema consumed by this model.
    fn feature_schema(&self) -> &FeatureSchema;

    /// Runs model inference over a validated feature vector.
    fn infer_features(&mut self, features: &FeatureVectorView<'_>) -> SignalModelOutput;
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

const BUILT_IN_SIGNAL_REGISTRATIONS: [SignalRegistration; 7] = [
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

fn create_delta_momentum_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(DeltaMomentumSignal::new(integer_parameter(
        config,
        &DELTA_MOMENTUM_DESCRIPTOR,
        "threshold",
    )?)))
}

fn create_volume_imbalance_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(VolumeImbalanceSignal::new(integer_parameter(
        config,
        &VOLUME_IMBALANCE_DESCRIPTOR,
        "threshold",
    )?)))
}

fn create_cumulative_delta_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(CumulativeDeltaSignal::new(integer_parameter(
        config,
        &CUMULATIVE_DELTA_DESCRIPTOR,
        "threshold",
    )?)))
}

fn create_absorption_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(AbsorptionSignal::new(
        integer_parameter(config, &ABSORPTION_DESCRIPTOR, "threshold")?,
        integer_parameter(config, &ABSORPTION_DESCRIPTOR, "price_band")?,
    )))
}

fn create_exhaustion_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(ExhaustionSignal::new(integer_parameter(
        config,
        &EXHAUSTION_DESCRIPTOR,
        "threshold",
    )?)))
}

fn create_sweep_detection_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    Ok(Box::new(SweepDetectionSignal::new(
        integer_parameter(config, &SWEEP_DETECTION_DESCRIPTOR, "threshold")?,
        integer_parameter(config, &SWEEP_DETECTION_DESCRIPTOR, "breakout_ticks")?,
    )))
}

fn create_composite_signal(
    config: &SignalConfig<'_>,
) -> SignalRegistryResult<Box<dyn SignalModule>> {
    validate_signal_config(&COMPOSITE_DESCRIPTOR, config)?;
    Ok(Box::new(CompositeSignal::default()))
}

fn integer_parameter(
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

fn validate_signal_config(
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

fn config_value_is_below(actual: SignalConfigValue<'_>, min: SignalParameterValue) -> bool {
    match (actual, min) {
        (SignalConfigValue::Integer(actual), SignalParameterValue::Integer(min)) => actual < min,
        (SignalConfigValue::Float(actual), SignalParameterValue::Float(min)) => actual < min,
        _ => false,
    }
}

fn config_value_is_above(actual: SignalConfigValue<'_>, max: SignalParameterValue) -> bool {
    match (actual, max) {
        (SignalConfigValue::Integer(actual), SignalParameterValue::Integer(max)) => actual > max,
        (SignalConfigValue::Float(actual), SignalParameterValue::Float(max)) => actual > max,
        _ => false,
    }
}

fn config_value_to_static(value: SignalConfigValue<'_>) -> SignalConfigValue<'static> {
    match value {
        SignalConfigValue::Integer(value) => SignalConfigValue::Integer(value),
        SignalConfigValue::Float(value) => SignalConfigValue::Float(value),
        SignalConfigValue::Boolean(value) => SignalConfigValue::Boolean(value),
        SignalConfigValue::Text(_) => SignalConfigValue::Text("<text>"),
    }
}

fn push_descriptor_json(out: &mut String, descriptor: &SignalDescriptor) {
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

fn push_explanation_json(out: &mut String, explanation: &SignalExplanation) {
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

fn push_json_field(out: &mut String, name: &str, value: &str) {
    push_json_string(out, name);
    out.push(':');
    push_json_string(out, value);
}

fn push_json_usize_field(out: &mut String, name: &str, value: usize) {
    push_json_string(out, name);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_json_i64_field(out: &mut String, name: &str, value: i64) {
    push_json_string(out, name);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_json_u16_field(out: &mut String, name: &str, value: u16) {
    push_json_string(out, name);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_json_bool_field(out: &mut String, name: &str, value: bool) {
    push_json_string(out, name);
    out.push(':');
    out.push_str(if value { "true" } else { "false" });
}

fn push_validation_sample_json(out: &mut String, sample: &SignalValidationSample) {
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

fn push_validation_warning_json(out: &mut String, warning: &SignalValidationWarning) {
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

fn push_json_string(out: &mut String, value: &str) {
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

fn push_input_mask_json(out: &mut String, mask: SignalInputMask) {
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

fn push_warmup_json(out: &mut String, warmup: SignalWarmupRequirement) {
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

fn push_parameters_json(out: &mut String, parameters: &[SignalParameterDescriptor]) {
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

fn push_input_values_json(out: &mut String, inputs: &[SignalInputValue]) {
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

fn push_thresholds_json(out: &mut String, thresholds: &[SignalThreshold]) {
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

fn push_confidence_components_json(out: &mut String, components: &[SignalConfidenceComponent]) {
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

fn push_optional_parameter_value_json(out: &mut String, value: Option<SignalParameterValue>) {
    match value {
        Some(value) => push_parameter_value_json(out, value),
        None => out.push_str("null"),
    }
}

fn push_parameter_value_json(out: &mut String, value: SignalParameterValue) {
    match value {
        SignalParameterValue::Integer(value) => out.push_str(&value.to_string()),
        SignalParameterValue::Float(value) => out.push_str(&value.to_string()),
        SignalParameterValue::Boolean(value) => out.push_str(if value { "true" } else { "false" }),
        SignalParameterValue::Text(value) => push_json_string(out, value),
    }
}

fn signal_state_name(state: SignalState) -> &'static str {
    match state {
        SignalState::Neutral => "neutral",
        SignalState::LongBias => "long_bias",
        SignalState::ShortBias => "short_bias",
        SignalState::Blocked => "blocked",
    }
}

fn parameter_kind_name(kind: SignalParameterKind) -> &'static str {
    match kind {
        SignalParameterKind::Integer => "integer",
        SignalParameterKind::Float => "float",
        SignalParameterKind::Boolean => "boolean",
        SignalParameterKind::Text => "text",
    }
}

fn output_semantics_name(output_semantics: SignalOutputSemantics) -> &'static str {
    match output_semantics {
        SignalOutputSemantics::DirectionalBias => "directional_bias",
        SignalOutputSemantics::CompositeBias => "composite_bias",
        SignalOutputSemantics::Informational => "informational",
        SignalOutputSemantics::Veto => "veto",
    }
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

    fn outcome_record(
        confidence_bps: u16,
        markout_direction: SignalMarkoutDirection,
        correct: Option<bool>,
    ) -> SignalOutcomeRecord {
        SignalOutcomeRecord {
            module_id: "test_signal_v1",
            state: SignalState::LongBias,
            confidence_bps,
            calibrated_confidence_bps: confidence_bps,
            predicted_direction: Some(SignalMarkoutDirection::Up),
            markout_direction,
            correct,
            regime: None,
        }
    }

    #[test]
    fn signal_explanation_json_exports_audit_payload() {
        let explanation = SignalExplanation::new(
            "test_signal_v1",
            SignalState::LongBias,
            7_500,
            DataQualityFlags::SEQUENCE_GAP.bits(),
            SignalReasonCode::DeltaMomentumPositive,
            "delta_above_threshold",
        )
        .with_input(SignalInputValue::integer("delta", 125))
        .with_threshold(SignalThreshold::integer("threshold", 100))
        .with_confidence_component(SignalConfidenceComponent::new("base", 7_500));

        let json = explanation.to_json();
        assert!(json.contains("\"module_id\":\"test_signal_v1\""));
        assert!(json.contains("\"state\":\"long_bias\""));
        assert!(json.contains("\"reason_code\":\"delta_momentum_positive\""));
        assert!(json.contains("\"inputs\":[{\"name\":\"delta\",\"value\":125}]"));
        assert!(json.contains("\"thresholds\":[{\"name\":\"threshold\",\"value\":100}]"));
        assert!(json.contains("\"confidence_components\":[{\"name\":\"base\",\"value_bps\":7500}]"));

        struct SnapshotOnlySignal;
        impl SignalModule for SnapshotOnlySignal {
            fn on_analytics(&mut self, _ev: &AnalyticsSnapshot) {}

            fn snapshot(&self) -> SignalSnapshot {
                snapshot(SignalState::Neutral, 0)
            }

            fn quality_gate(&self, _q: DataQualityFlags) -> SignalGateDecision {
                SignalGateDecision::Pass
            }
        }

        assert!(SnapshotOnlySignal.latest_explanation().is_none());
        assert!(DeltaMomentumSignal::default()
            .latest_explanation()
            .is_some());
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
    fn reason_codes_have_stable_string_values() {
        assert_eq!(
            SignalReasonCode::DeltaMomentumPositive.as_str(),
            "delta_momentum_positive"
        );
        assert_eq!(
            SignalReasonCode::BuyVolumeImbalance.as_str(),
            "buy_volume_imbalance"
        );
        assert_eq!(
            SignalReasonCode::from(SignalSuppressionReason::CooldownActive),
            SignalReasonCode::StabilizerCooldownActive
        );
    }

    #[test]
    fn delta_momentum_explanation_reports_inputs_and_threshold() {
        let mut signal = DeltaMomentumSignal::new(10);
        signal.on_analytics(&AnalyticsSnapshot {
            delta: 15,
            ..Default::default()
        });

        let explanation = signal.explanation();
        assert_eq!(explanation.module_id, "delta_momentum_v1");
        assert_eq!(explanation.state, SignalState::LongBias);
        assert_eq!(
            explanation.reason_code,
            SignalReasonCode::DeltaMomentumPositive
        );
        assert_eq!(
            explanation.inputs,
            vec![SignalInputValue::integer("delta", 15)]
        );
        assert_eq!(
            explanation.thresholds,
            vec![SignalThreshold::integer("threshold", 10)]
        );
    }

    #[test]
    fn absorption_explanation_reports_decision_context() {
        let mut signal = AbsorptionSignal::new(20, 2);
        signal.on_analytics(&AnalyticsSnapshot {
            delta: -25,
            last_price: 100,
            point_of_control: 101,
            ..Default::default()
        });

        let explanation = signal.explanation();
        assert_eq!(
            explanation.reason_code,
            SignalReasonCode::SellAbsorptionDetected
        );
        assert!(explanation
            .inputs
            .contains(&SignalInputValue::integer("poc_distance", 1)));
        assert!(explanation
            .thresholds
            .contains(&SignalThreshold::integer("price_band", 2)));
    }

    #[test]
    fn composite_explanation_reports_vote_counts() {
        let mut signal = CompositeSignal::new(vec![
            Box::new(DeltaMomentumSignal::new(10)),
            Box::new(VolumeImbalanceSignal::new(10)),
            Box::new(CumulativeDeltaSignal::new(100)),
        ]);
        signal.on_analytics(&AnalyticsSnapshot {
            delta: 20,
            buy_volume: 30,
            sell_volume: 10,
            cumulative_delta: 50,
            ..Default::default()
        });

        let explanation = signal.explanation();
        assert_eq!(
            explanation.reason_code,
            SignalReasonCode::CompositeLongMajority
        );
        assert!(explanation
            .inputs
            .contains(&SignalInputValue::integer("module_count", 3)));
        assert!(explanation
            .inputs
            .contains(&SignalInputValue::integer("long_votes", 2)));
        assert!(explanation
            .inputs
            .contains(&SignalInputValue::integer("short_votes", 0)));
    }

    #[test]
    fn transition_only_explanation_mode_emits_on_state_change() {
        let previous = snapshot(SignalState::Neutral, 500);
        let current_same = snapshot(SignalState::Neutral, 500);
        let current_changed = snapshot(SignalState::LongBias, 600);

        assert!(SignalExplanationMode::Always.should_emit_snapshot(Some(&previous), &current_same));
        assert!(!SignalExplanationMode::TransitionsOnly
            .should_emit_snapshot(Some(&previous), &current_same));
        assert!(SignalExplanationMode::TransitionsOnly
            .should_emit_snapshot(Some(&previous), &current_changed));
        assert!(SignalExplanationMode::TransitionsOnly.should_emit_snapshot(None, &current_same));
    }

    #[test]
    fn signal_registry_exposes_built_in_inventory() {
        let registry = SignalRegistry::with_built_ins();

        assert_eq!(registry.registrations().len(), 7);
        assert_eq!(
            registry.descriptor("delta_momentum_v1").unwrap().name,
            "Delta Momentum"
        );
        assert!(built_in_signal_registrations()
            .iter()
            .any(|registration| registration.descriptor.id == "sweep_detection_v1"));
    }

    #[test]
    fn signal_registry_filters_by_available_inputs() {
        let registry = SignalRegistry::with_built_ins();

        let none = registry.descriptors_matching_inputs(SignalInputMask::ANALYTICS);
        assert!(none.is_empty());

        let all = registry.descriptors_matching_inputs(
            SignalInputMask::ANALYTICS | SignalInputMask::DATA_QUALITY,
        );
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn signal_registry_validates_config_parameters() {
        let registry = SignalRegistry::with_built_ins();
        let params = [SignalConfigParameter::integer("threshold", 25)];
        let config = SignalConfig::with_parameters("delta_momentum_v1", &params);

        assert!(registry.validate_config(&config).is_ok());

        let bad_params = [SignalConfigParameter::integer("unknown", 1)];
        let bad_config = SignalConfig::with_parameters("delta_momentum_v1", &bad_params);
        assert!(matches!(
            registry.validate_config(&bad_config),
            Err(SignalRegistryError::UnknownParameter { .. })
        ));

        let wrong_type = [SignalConfigParameter::boolean("threshold", true)];
        let wrong_config = SignalConfig::with_parameters("delta_momentum_v1", &wrong_type);
        assert!(matches!(
            registry.validate_config(&wrong_config),
            Err(SignalRegistryError::InvalidParameterType { .. })
        ));

        let below_min = [SignalConfigParameter::integer("threshold", -1)];
        let below_config = SignalConfig::with_parameters("delta_momentum_v1", &below_min);
        assert!(matches!(
            registry.validate_config(&below_config),
            Err(SignalRegistryError::ParameterBelowMinimum { .. })
        ));

        let duplicate_params = [
            SignalConfigParameter::integer("threshold", 1),
            SignalConfigParameter::integer("threshold", 2),
        ];
        let duplicate_config =
            SignalConfig::with_parameters("delta_momentum_v1", &duplicate_params);
        assert!(matches!(
            registry.validate_config(&duplicate_config),
            Err(SignalRegistryError::DuplicateParameter { .. })
        ));
    }

    #[test]
    fn signal_registry_constructs_built_in_signal_from_config() {
        let registry = SignalRegistry::with_built_ins();
        let params = [SignalConfigParameter::integer("threshold", 25)];
        let config = SignalConfig::with_parameters("delta_momentum_v1", &params);
        let mut signal = registry.create_signal(&config).expect("signal constructed");

        signal.on_analytics(&AnalyticsSnapshot {
            delta: 30,
            ..Default::default()
        });

        let snapshot = signal.snapshot();
        assert_eq!(snapshot.module_id, "delta_momentum_v1");
        assert_eq!(snapshot.state, SignalState::LongBias);
    }

    #[test]
    fn signal_registry_rejects_duplicate_registration() {
        let mut registry = SignalRegistry::with_built_ins();
        let result = registry.register(SignalRegistration::new(
            &DELTA_MOMENTUM_DESCRIPTOR,
            Some(create_delta_momentum_signal),
        ));

        assert!(matches!(
            result,
            Err(SignalRegistryError::DuplicateSignal {
                id: "delta_momentum_v1"
            })
        ));
    }

    #[test]
    fn signal_descriptor_json_exports_binding_inventory() {
        let json = built_in_signal_descriptors_json();

        assert!(json.starts_with('['));
        assert!(json.contains("\"id\":\"delta_momentum_v1\""));
        assert!(json.contains("\"required_inputs_bits\":3"));
        assert!(json.contains("\"parameters\""));
        assert!(json.contains("\"output_semantics\":\"directional_bias\""));
    }

    #[test]
    fn signal_registry_validation_json_preserves_diagnostics() {
        let registry = SignalRegistry::with_built_ins();
        let invalid = [SignalConfigParameter::integer("threshold", -1)];
        let invalid_config = SignalConfig::with_parameters("delta_momentum_v1", &invalid);
        let invalid_json = registry.validate_config_json(&invalid_config);
        assert!(invalid_json.contains("\"valid\":false"));
        assert!(invalid_json.contains("below the descriptor minimum"));

        let valid_config = SignalConfig::new("delta_momentum_v1");
        let valid_json = registry.validate_config_json(&valid_config);
        assert!(valid_json.contains("\"valid\":true"));
        assert!(valid_json.contains("\"error\":null"));
    }

    #[test]
    fn signal_validation_scores_directional_markouts() {
        let mut signal = DeltaMomentumSignal::new(10);
        let events = vec![
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: -20,
                last_price: 90,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: -20,
                last_price: 80,
                ..Default::default()
            },
        ];
        let config = SignalValidationConfig::new(1).with_store_samples(true);

        let report = validate_signal_replay(&mut signal, &events, config);

        assert_eq!(report.module_id, Some("delta_momentum_v1"));
        assert_eq!(report.evaluated_events, 3);
        assert_eq!(report.labeled_events, 2);
        assert_eq!(report.missing_markouts, 1);
        assert_eq!(report.directional_predictions, 2);
        assert_eq!(report.correct_directional, 1);
        assert_eq!(report.incorrect_directional, 1);
        assert_eq!(report.directional_accuracy_bps(), Some(5_000));
        assert_eq!(report.label_coverage_bps(), Some(6_666));
        assert_eq!(report.samples.len(), 2);
        assert_eq!(
            report.samples[0].markout_direction,
            SignalMarkoutDirection::Down
        );
        assert_eq!(
            report.samples[0].predicted_direction,
            Some(SignalMarkoutDirection::Up)
        );
        assert_eq!(report.samples[0].correct, Some(false));
        assert!(report
            .warnings
            .iter()
            .any(|warning| matches!(warning, SignalValidationWarning::MissingMarkout { .. })));

        let json = report.json_report();
        assert!(json.contains("\"schema_version\":1"));
        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("\"long_predictions\":1"));
        assert!(json.contains("\"short_predictions\":2"));
        assert!(json.contains("\"directional_accuracy_bps\":5000"));
        assert!(json.contains("\"samples\":[{"));
        assert!(json.contains("\"code\":\"missing_markout\""));
    }

    #[test]
    fn signal_validation_confidence_filter_excludes_weak_predictions() {
        let mut signal = DeltaMomentumSignal::new(10);
        let events = vec![
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: 20,
                last_price: 110,
                ..Default::default()
            },
        ];
        let config = SignalValidationConfig::new(1).with_min_confidence_bps(600);

        let report = validate_signal_replay(&mut signal, &events, config);

        assert_eq!(report.directional_predictions, 0);
        assert_eq!(report.directional_accuracy_bps(), None);
        assert_eq!(report.long_predictions, 2);
    }

    #[test]
    fn signal_validation_warns_on_non_monotonic_timestamps() {
        let mut signal = DeltaMomentumSignal::new(10);
        let snapshots = [
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: 20,
                last_price: 110,
                ..Default::default()
            },
        ];
        let events = [
            SignalReplayEvent::with_ts_exchange_ns(&snapshots[0], 200),
            SignalReplayEvent::with_ts_exchange_ns(&snapshots[1], 100),
        ];

        let report =
            validate_signal_replay_events(&mut signal, &events, SignalValidationConfig::new(1));

        assert!(report.warnings.iter().any(|warning| matches!(
            warning,
            SignalValidationWarning::NonMonotonicTimestamp {
                event_index: 1,
                previous_ts_exchange_ns: 200,
                current_ts_exchange_ns: 100
            }
        )));
    }

    #[test]
    fn signal_validation_zero_horizon_is_normalized_and_reported() {
        let mut signal = DeltaMomentumSignal::new(10);
        let events = vec![
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: 20,
                last_price: 110,
                ..Default::default()
            },
        ];

        let report = validate_signal_replay(&mut signal, &events, SignalValidationConfig::new(0));

        assert_eq!(report.labeled_events, 1);
        assert!(report
            .warnings
            .contains(&SignalValidationWarning::ZeroMarkoutHorizon));
    }

    #[test]
    fn signal_validation_json_summary_is_python_friendly() {
        let mut signal = DeltaMomentumSignal::new(10);
        let events = vec![
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: 20,
                last_price: 110,
                ..Default::default()
            },
        ];

        let report = SignalValidationHarness::default().validate_signal(&mut signal, &events);
        let json = report.json_summary();

        assert!(json.contains("\"module_id\":\"delta_momentum_v1\""));
        assert!(json.contains("\"evaluated_events\":2"));
        assert!(json.contains("\"directional_accuracy_bps\":10000"));
    }

    #[test]
    fn signal_calibration_curve_interpolates_confidence() {
        let curve = SignalCalibrationCurve::new(vec![
            SignalCalibrationPoint::new(10_000, 9_000),
            SignalCalibrationPoint::new(0, 0),
            SignalCalibrationPoint::new(5_000, 4_000),
        ]);

        assert_eq!(
            IdentitySignalCalibrator.calibrate_confidence_bps(12_000),
            10_000
        );
        assert_eq!(curve.calibrate_confidence_bps(0), 0);
        assert_eq!(curve.calibrate_confidence_bps(2_500), 2_000);
        assert_eq!(curve.calibrate_confidence_bps(7_500), 6_500);
        assert_eq!(curve.calibrate_confidence_bps(10_000), 9_000);
    }

    #[test]
    fn signal_calibration_report_scores_validation_samples() {
        let mut signal = DeltaMomentumSignal::new(10);
        let events = vec![
            AnalyticsSnapshot {
                delta: 20,
                last_price: 100,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: -20,
                last_price: 90,
                ..Default::default()
            },
            AnalyticsSnapshot {
                delta: -20,
                last_price: 80,
                ..Default::default()
            },
        ];
        let validation_config = SignalValidationConfig::new(1).with_store_samples(true);
        let validation_report = validate_signal_replay(&mut signal, &events, validation_config);

        let calibration_report = SignalCalibrationReport::from_validation_report(
            &validation_report,
            SignalCalibrationConfig::new(1_000),
        );

        assert_eq!(calibration_report.total_records, 2);
        assert_eq!(calibration_report.scored_records, 2);
        assert_eq!(calibration_report.ignored_records, 0);
        assert_eq!(calibration_report.correct_records, 1);
        assert_eq!(calibration_report.accuracy_bps(), Some(5_000));
        assert_eq!(calibration_report.expected_calibration_error_bps, 4_500);

        let populated_bin = calibration_report
            .bins
            .iter()
            .find(|bin| bin.samples == 2)
            .expect("populated confidence bin");
        assert_eq!(populated_bin.lower_confidence_bps, 0);
        assert_eq!(populated_bin.upper_confidence_bps, 999);
        assert_eq!(populated_bin.average_confidence_bps, 500);
        assert_eq!(populated_bin.accuracy_bps, Some(5_000));
        assert_eq!(populated_bin.calibration_error_bps, Some(4_500));
    }

    #[test]
    fn signal_outcome_tracker_summarizes_regimes() {
        let mut tracker = SignalOutcomeTracker::new(SignalCalibrationConfig::new(2_000));
        tracker.record(
            outcome_record(8_000, SignalMarkoutDirection::Up, Some(true)).with_regime("trend"),
        );
        tracker.record(
            outcome_record(8_000, SignalMarkoutDirection::Down, Some(false)).with_regime("trend"),
        );
        tracker.record(
            outcome_record(6_000, SignalMarkoutDirection::Up, Some(true)).with_regime("range"),
        );
        tracker.record(outcome_record(4_000, SignalMarkoutDirection::Flat, None));

        let report = tracker.calibration_report();

        assert_eq!(tracker.records().len(), 4);
        assert_eq!(report.total_records, 4);
        assert_eq!(report.scored_records, 3);
        assert_eq!(report.ignored_records, 1);

        let trend = report
            .regimes
            .iter()
            .find(|regime| regime.regime == "trend")
            .expect("trend regime");
        assert_eq!(trend.samples, 2);
        assert_eq!(trend.correct, 1);
        assert_eq!(trend.average_confidence_bps, 8_000);
        assert_eq!(trend.accuracy_bps, Some(5_000));

        let range = report
            .regimes
            .iter()
            .find(|regime| regime.regime == "range")
            .expect("range regime");
        assert_eq!(range.samples, 1);
        assert_eq!(range.accuracy_bps, Some(10_000));
    }

    #[test]
    fn signal_calibration_drift_flags_ece_change() {
        let config = SignalCalibrationConfig::new(1_000).with_drift_alert_threshold_bps(500);
        let baseline = SignalCalibrationReport::from_records(
            &[
                outcome_record(9_000, SignalMarkoutDirection::Up, Some(true)),
                outcome_record(9_000, SignalMarkoutDirection::Up, Some(true)),
            ],
            config,
        );
        let current = SignalCalibrationReport::from_records(
            &[
                outcome_record(9_000, SignalMarkoutDirection::Down, Some(false)),
                outcome_record(9_000, SignalMarkoutDirection::Down, Some(false)),
            ],
            config,
        );

        let drift = SignalCalibrationDriftReport::compare(
            &baseline,
            &current,
            config.drift_alert_threshold_bps,
        );

        assert_eq!(baseline.expected_calibration_error_bps, 1_000);
        assert_eq!(current.expected_calibration_error_bps, 9_000);
        assert_eq!(drift.ece_delta_bps, 8_000);
        assert!(drift.significant);
        assert!(drift
            .bin_drifts
            .iter()
            .any(|bin| bin.accuracy_delta_bps == Some(-10_000)));
    }

    #[test]
    fn signal_calibration_json_summary_is_dependency_free() {
        let report = SignalCalibrationReport::from_records(
            &[outcome_record(
                7_000,
                SignalMarkoutDirection::Up,
                Some(true),
            )],
            SignalCalibrationConfig::default(),
        );

        let json = report.json_summary();

        assert!(json.contains("\"total_records\":1"));
        assert!(json.contains("\"scored_records\":1"));
        assert!(json.contains("\"accuracy_bps\":10000"));
        assert!(json.contains("\"expected_calibration_error_bps\":3000"));
    }

    #[test]
    fn signal_ensemble_majority_selects_directional_side() {
        let votes = [
            SignalEnsembleVote::new("delta_momentum_v1", SignalState::LongBias, 7_000),
            SignalEnsembleVote::new("volume_imbalance_v1", SignalState::ShortBias, 8_000),
            SignalEnsembleVote::new("cumulative_delta_v1", SignalState::LongBias, 6_000),
        ];

        let decision =
            evaluate_signal_ensemble("ensemble_v1", &votes, SignalEnsemblePolicy::majority());

        assert_eq!(decision.snapshot.module_id, "ensemble_v1");
        assert_eq!(decision.snapshot.state, SignalState::LongBias);
        assert_eq!(decision.snapshot.confidence_bps, 6_500);
        assert_eq!(decision.metrics.total_votes, 3);
        assert_eq!(decision.metrics.long_votes, 2);
        assert_eq!(decision.metrics.short_votes, 1);
        assert_eq!(decision.conflict, SignalEnsembleConflict::None);
    }

    #[test]
    fn signal_ensemble_quorum_requires_minimum_votes() {
        let votes = [
            SignalEnsembleVote::new("delta_momentum_v1", SignalState::LongBias, 7_000),
            SignalEnsembleVote::new("volume_imbalance_v1", SignalState::ShortBias, 9_000),
        ];
        let policy = SignalEnsemblePolicy::quorum(2)
            .with_conflict_policy(SignalEnsembleConflictPolicy::HighestConfidence);

        let decision = evaluate_signal_ensemble("ensemble_v1", &votes, policy);

        assert_eq!(decision.snapshot.state, SignalState::Neutral);
        assert_eq!(decision.conflict, SignalEnsembleConflict::None);
        assert_eq!(decision.metrics.eligible_votes, 2);
    }

    #[test]
    fn signal_ensemble_weighted_policy_can_override_vote_count() {
        let votes = [
            SignalEnsembleVote::new("fast_momentum", SignalState::LongBias, 6_000)
                .with_weight_bps(2_000),
            SignalEnsembleVote::new("slow_momentum", SignalState::LongBias, 6_000)
                .with_weight_bps(2_000),
            SignalEnsembleVote::new("risk_model", SignalState::ShortBias, 9_000)
                .with_weight_bps(10_000),
        ];
        let policy = SignalEnsemblePolicy::weighted(8_000);

        let decision = evaluate_signal_ensemble("ensemble_v1", &votes, policy);

        assert_eq!(decision.snapshot.state, SignalState::ShortBias);
        assert_eq!(decision.metrics.long_votes, 2);
        assert_eq!(decision.metrics.short_votes, 1);
        assert_eq!(decision.metrics.long_weighted_score_bps, 2_400);
        assert_eq!(decision.metrics.short_weighted_score_bps, 9_000);
    }

    #[test]
    fn signal_ensemble_veto_blocks_by_default() {
        let votes = [
            SignalEnsembleVote::new("delta_momentum_v1", SignalState::LongBias, 7_000),
            SignalEnsembleVote::new("risk_veto_v1", SignalState::Blocked, 0)
                .with_quality_flags(DataQualityFlags::STALE_FEED.bits()),
        ];

        let decision =
            evaluate_signal_ensemble("ensemble_v1", &votes, SignalEnsemblePolicy::default());

        assert_eq!(decision.snapshot.state, SignalState::Blocked);
        assert!(decision.veto_applied);
        assert_eq!(decision.conflict, SignalEnsembleConflict::Veto);
        assert_eq!(decision.metrics.veto_votes, 1);
        assert_eq!(
            decision.snapshot.quality_flags,
            DataQualityFlags::STALE_FEED.bits()
        );
    }

    #[test]
    fn signal_ensemble_explanation_aggregates_children() {
        let children = vec![
            SignalExplanation::new(
                "delta_momentum_v1",
                SignalState::LongBias,
                8_000,
                0,
                SignalReasonCode::DeltaMomentumPositive,
                "delta_above_threshold",
            ),
            SignalExplanation::new(
                "volume_imbalance_v1",
                SignalState::LongBias,
                7_000,
                0,
                SignalReasonCode::BuyVolumeImbalance,
                "buy_volume_imbalance",
            ),
        ];

        let ensemble = evaluate_signal_ensemble_explanations(
            "ensemble_v1",
            children,
            &[10_000, 5_000],
            SignalEnsemblePolicy::majority(),
        );
        let explanation = ensemble.explanation();

        assert_eq!(ensemble.children.len(), 2);
        assert_eq!(ensemble.decision.snapshot.state, SignalState::LongBias);
        assert_eq!(
            explanation.reason_code,
            SignalReasonCode::EnsembleLongSelected
        );
        assert!(explanation
            .inputs
            .contains(&SignalInputValue::integer("total_votes", 2)));
        assert!(explanation
            .confidence_components
            .contains(&SignalConfidenceComponent::new(
                "average_child_confidence",
                7_500
            )));
    }

    #[test]
    fn signal_checkpoint_restore_validation_accepts_matching_metadata() {
        let symbol = SymbolId {
            venue: "SIM".to_string(),
            symbol: "ES".to_string(),
        };
        let checkpoint =
            SignalCheckpoint::from_snapshot(&snapshot(SignalState::LongBias, 7_000), "1")
                .with_config_hash(42)
                .with_symbol(symbol.clone())
                .with_calibration_id(7)
                .with_timestamps(1_000, 2_000)
                .with_payload(vec![1, 2, 3]);
        let policy = SignalCheckpointRestorePolicy::new()
            .with_signal("test_signal_v1", "1")
            .with_config_hash(42)
            .with_symbol(symbol)
            .with_calibration_id(7)
            .with_min_last_update_ns(1_500);

        let report = validate_signal_checkpoint_restore(&checkpoint, &policy);

        assert!(report.valid);
        assert!(report.issues.is_empty());
        assert_eq!(checkpoint.payload, vec![1, 2, 3]);
    }

    #[test]
    fn signal_checkpoint_restore_validation_reports_mismatches() {
        let checkpoint = SignalCheckpoint::new("candidate_signal_v1", "2", SignalState::Neutral)
            .with_config_hash(10)
            .with_timestamps(0, 500);
        let policy = SignalCheckpointRestorePolicy::new()
            .with_signal("production_signal_v1", "1")
            .with_config_hash(11)
            .with_min_last_update_ns(1_000);

        let report = validate_signal_checkpoint_restore(&checkpoint, &policy);

        assert!(!report.valid);
        assert!(report.has_errors());
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            SignalCheckpointValidationIssue::ModuleIdMismatch { .. }
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            SignalCheckpointValidationIssue::SignalVersionMismatch { .. }
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            SignalCheckpointValidationIssue::ConfigHashMismatch { .. }
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            SignalCheckpointValidationIssue::NonMonotonicTimestamp { .. }
        )));
    }

    #[test]
    fn signal_run_modes_encode_shadow_safety_behavior() {
        let active = SignalRunModeDecision::from_mode(SignalRunMode::Active);
        let shadow = SignalRunModeDecision::from_mode(SignalRunMode::Shadow);
        let record_only = SignalRunModeDecision::from_mode(SignalRunMode::RecordOnly);
        let disabled = SignalRunModeDecision::from_mode(SignalRunMode::Disabled);

        assert!(active.evaluate);
        assert!(active.publish_for_trading);
        assert!(shadow.evaluate);
        assert!(!shadow.publish_for_trading);
        assert!(shadow.record_output);
        assert!(!record_only.evaluate);
        assert!(record_only.record_input);
        assert!(!record_only.record_output);
        assert!(!disabled.evaluate);
        assert!(!disabled.record_input);
    }

    #[test]
    fn shadow_comparison_report_scores_candidate_against_production() {
        let production = snapshot(SignalState::LongBias, 6_000);
        let candidate = snapshot(SignalState::ShortBias, 8_000);
        let samples = [
            SignalShadowSample::compare(0, production, candidate)
                .with_ts_exchange_ns(1_000)
                .with_markout(SignalMarkoutDirection::Down),
            SignalShadowSample::compare(
                1,
                snapshot(SignalState::Neutral, 3_000),
                snapshot(SignalState::LongBias, 7_000),
            )
            .with_markout(SignalMarkoutDirection::Up),
        ];

        let report = SignalShadowComparisonReport::from_samples(
            &samples,
            SignalShadowComparisonConfig::new().with_store_samples(true),
        );

        assert_eq!(report.total_samples, 2);
        assert_eq!(report.state_disagreements, 2);
        assert_eq!(report.production_directional, 1);
        assert_eq!(report.candidate_directional, 2);
        assert_eq!(report.production_correct, 0);
        assert_eq!(report.candidate_correct, 2);
        assert_eq!(report.candidate_only_correct, 1);
        assert_eq!(report.average_confidence_delta_bps, 3_000);
        assert_eq!(report.agreement_bps(), Some(0));
        assert_eq!(report.candidate_accuracy_bps(), Some(10_000));
        assert_eq!(report.samples.len(), 2);
        assert!(report
            .json_summary()
            .contains("\"candidate_accuracy_bps\":10000"));
    }

    #[test]
    fn shadow_recorder_builds_report_without_retaining_samples_by_default() {
        let mut recorder = SignalShadowRecorder::default();
        recorder.record(
            SignalShadowSample::compare(
                0,
                snapshot(SignalState::LongBias, 7_000),
                snapshot(SignalState::LongBias, 8_000),
            )
            .with_markout(SignalMarkoutDirection::Up),
        );

        let report = recorder.report();

        assert_eq!(recorder.samples().len(), 1);
        assert_eq!(report.compared_samples, 1);
        assert_eq!(report.state_agreements, 1);
        assert_eq!(report.samples.len(), 0);
        assert_eq!(report.production_accuracy_bps(), Some(10_000));
    }

    #[test]
    fn feature_vector_validation_accepts_clean_vector() {
        let schema = FeatureSchema::new("orderflow_features", "1")
            .with_feature(
                FeatureDescriptor::new("delta", FeatureValueKind::Integer)
                    .with_unit("contracts")
                    .with_range(-10_000.0, 10_000.0),
            )
            .with_feature(
                FeatureDescriptor::new("imbalance_bps", FeatureValueKind::BasisPoints)
                    .with_unit("bps")
                    .with_range(-10_000.0, 10_000.0)
                    .with_freshness_ns(1_000),
            );
        let values = [125.0, 2_500.0];
        let quality = [FeatureQualityFlags::NONE, FeatureQualityFlags::NONE];
        let view = FeatureVectorView::new(&schema, &values, &quality, 1_000);

        let report = view.validate(Some(1_500));

        assert!(report.valid);
        assert_eq!(view.value("delta"), Some(125.0));
        assert_eq!(
            view.quality("imbalance_bps"),
            Some(FeatureQualityFlags::NONE)
        );
        assert_eq!(report.aggregate_quality, FeatureQualityFlags::NONE);
    }

    #[test]
    fn feature_vector_validation_reports_schema_and_quality_issues() {
        let schema = FeatureSchema::new("orderflow_features", "1")
            .with_feature(FeatureDescriptor::new("delta", FeatureValueKind::Integer))
            .with_feature(
                FeatureDescriptor::new("vwap_distance", FeatureValueKind::Price)
                    .with_range(-10.0, 10.0)
                    .with_freshness_ns(100),
            );
        let values = [25.0, 25.0];
        let quality = [FeatureQualityFlags::MISSING];
        let view = FeatureVectorView::new(&schema, &values, &quality, 1_000);

        let report = validate_feature_vector(&view, Some(1_200));

        assert!(!report.valid);
        assert!(report.has_errors());
        assert!(report
            .aggregate_quality
            .intersects(FeatureQualityFlags::MISSING | FeatureQualityFlags::OUT_OF_RANGE));
        assert!(report
            .issues
            .iter()
            .any(|issue| matches!(issue, FeatureVectorValidationIssue::LengthMismatch { .. })));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            FeatureVectorValidationIssue::MissingFeature { feature_id, .. }
                if feature_id == "delta"
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            FeatureVectorValidationIssue::OutOfRange { feature_id, .. }
                if feature_id == "vwap_distance"
        )));
        assert!(report.issues.iter().any(|issue| matches!(
            issue,
            FeatureVectorValidationIssue::StaleFeature { feature_id, .. }
                if feature_id == "vwap_distance"
        )));
    }

    #[test]
    fn model_metadata_and_input_binding_validate_schema_compatibility() {
        let schema = FeatureSchema::new("orderflow_features", "1")
            .with_config_hash(42)
            .with_feature(FeatureDescriptor::new("delta", FeatureValueKind::Integer))
            .with_feature(FeatureDescriptor::new("vwap", FeatureValueKind::Price));
        let metadata = SignalModelMetadata::new("model_a", "2026-07-15", "orderflow_features", "1")
            .with_model_kind(SignalModelKind::Onnx)
            .with_artifact_hash("sha256:abc")
            .with_training_window(1_000, 2_000)
            .with_calibration_id(7)
            .with_output_kind(SignalModelOutputKind::DirectionalProbabilities);
        let binding = SignalModelInputBinding::new("features", vec!["delta".into(), "vwap".into()]);
        let missing =
            SignalModelInputBinding::new("features", vec!["delta".into(), "missing".into()]);

        assert_eq!(schema.feature_index("vwap"), Some(1));
        assert!(binding.is_compatible_with(&schema));
        assert!(!missing.is_compatible_with(&schema));
        assert_eq!(metadata.model_kind, SignalModelKind::Onnx);
        assert_eq!(metadata.calibration_id, Some(7));
    }

    #[test]
    fn model_backed_signal_trait_runs_over_feature_view() {
        struct TestModelSignal {
            metadata: SignalModelMetadata,
            schema: FeatureSchema,
            snapshot: SignalSnapshot,
        }

        impl SignalModule for TestModelSignal {
            fn on_analytics(&mut self, _ev: &AnalyticsSnapshot) {}

            fn snapshot(&self) -> SignalSnapshot {
                self.snapshot.clone()
            }

            fn quality_gate(&self, _q: DataQualityFlags) -> SignalGateDecision {
                SignalGateDecision::Pass
            }
        }

        impl ModelBackedSignal for TestModelSignal {
            fn model_metadata(&self) -> &SignalModelMetadata {
                &self.metadata
            }

            fn feature_schema(&self) -> &FeatureSchema {
                &self.schema
            }

            fn infer_features(&mut self, features: &FeatureVectorView<'_>) -> SignalModelOutput {
                let delta = features.value("delta").unwrap_or_default();
                let state = if delta >= 0.0 {
                    SignalState::LongBias
                } else {
                    SignalState::ShortBias
                };
                self.snapshot.state = state;
                self.snapshot.confidence_bps = 7_500;
                SignalModelOutput::new(state, 7_500)
                    .with_score(delta)
                    .with_reason("test_model")
            }
        }

        let schema = FeatureSchema::new("orderflow_features", "1")
            .with_feature(FeatureDescriptor::new("delta", FeatureValueKind::Integer));
        let metadata = SignalModelMetadata::new("test_model", "1", "orderflow_features", "1")
            .with_model_kind(SignalModelKind::Native);
        let values = [5.0];
        let quality = [FeatureQualityFlags::NONE];
        let view = FeatureVectorView::new(&schema, &values, &quality, 10);
        let mut signal = TestModelSignal {
            metadata,
            schema: schema.clone(),
            snapshot: snapshot(SignalState::Neutral, 0),
        };

        let output = signal.infer_features(&view);

        assert_eq!(signal.model_metadata().model_id, "test_model");
        assert_eq!(signal.feature_schema().id, "orderflow_features");
        assert_eq!(output.state, SignalState::LongBias);
        assert_eq!(output.confidence_bps, 7_500);
        assert_eq!(output.score, Some(5.0));
        assert_eq!(signal.snapshot().state, SignalState::LongBias);
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
