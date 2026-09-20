use super::*;

/// Feed-quality degradation flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeedQualityFlags {
    bits: u32,
}

impl FeedQualityFlags {
    /// No feed-quality issues.
    pub const OK: Self = Self { bits: 0 };
    /// Sequence gap was detected.
    pub const SEQUENCE_GAP: Self = Self { bits: 1 << 0 };
    /// Sequence or event timestamp moved backward.
    pub const OUT_OF_ORDER: Self = Self { bits: 1 << 1 };
    /// Consecutive duplicate sequence was observed.
    pub const DUPLICATE: Self = Self { bits: 1 << 2 };
    /// Event was older than the configured freshness window.
    pub const STALE: Self = Self { bits: 1 << 3 };
    /// Best bid equals best ask.
    pub const LOCKED_BOOK: Self = Self { bits: 1 << 4 };
    /// Best bid is greater than best ask.
    pub const CROSSED_BOOK: Self = Self { bits: 1 << 5 };
    /// Event and receive timestamps exceeded the configured skew limit.
    pub const TIMESTAMP_SKEW: Self = Self { bits: 1 << 6 };
    /// Sequence moved backward in a way that looks like a feed reset.
    pub const SEQUENCE_RESET: Self = Self { bits: 1 << 7 };

    /// Creates flags from raw bits.
    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    /// Returns raw flag bits.
    pub const fn bits(&self) -> u32 {
        self.bits
    }

    /// Returns true when no flags are set.
    pub const fn is_ok(&self) -> bool {
        self.bits == 0
    }

    /// Returns true when `other` is present in this set.
    pub const fn contains(&self, other: Self) -> bool {
        self.bits & other.bits == other.bits
    }

    /// Adds `other` flags into this set.
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

impl Default for FeedQualityFlags {
    fn default() -> Self {
        Self::OK
    }
}

/// Feed-quality tracker configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedQualityConfig {
    stale_after_ns: u64,
    max_timestamp_skew_ns: u64,
    expected_sequence_step: u64,
}

impl FeedQualityConfig {
    /// Creates feed-quality configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQuality`] when sequence step is zero.
    pub const fn new(
        stale_after_ns: u64,
        max_timestamp_skew_ns: u64,
        expected_sequence_step: u64,
    ) -> Result<Self, AnalyticsError> {
        if expected_sequence_step == 0 {
            return Err(AnalyticsError::InvalidQuality);
        }
        Ok(Self {
            stale_after_ns,
            max_timestamp_skew_ns,
            expected_sequence_step,
        })
    }

    /// Returns freshness threshold in nanoseconds.
    pub const fn stale_after_ns(&self) -> u64 {
        self.stale_after_ns
    }

    /// Returns maximum event/receive timestamp skew in nanoseconds.
    pub const fn max_timestamp_skew_ns(&self) -> u64 {
        self.max_timestamp_skew_ns
    }

    /// Returns expected sequence increment.
    pub const fn expected_sequence_step(&self) -> u64 {
        self.expected_sequence_step
    }
}

impl Default for FeedQualityConfig {
    fn default() -> Self {
        Self {
            stale_after_ns: 1_000_000_000,
            max_timestamp_skew_ns: 100_000_000,
            expected_sequence_step: 1,
        }
    }
}

/// Market-data event context used by feed-quality analytics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedQualityEvent {
    sequence: Option<u64>,
    event_ts_ns: u64,
    receive_ts_ns: u64,
    bid_price: Option<i64>,
    ask_price: Option<i64>,
}

impl FeedQualityEvent {
    /// Creates feed-quality event context.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQuality`] when timestamps are zero or
    /// supplied quote prices are non-positive.
    pub const fn new(
        sequence: Option<u64>,
        event_ts_ns: u64,
        receive_ts_ns: u64,
        bid_price: Option<i64>,
        ask_price: Option<i64>,
    ) -> Result<Self, AnalyticsError> {
        if event_ts_ns == 0 || receive_ts_ns == 0 {
            return Err(AnalyticsError::InvalidQuality);
        }
        if let Some(price) = bid_price {
            if price <= 0 {
                return Err(AnalyticsError::InvalidQuality);
            }
        }
        if let Some(price) = ask_price {
            if price <= 0 {
                return Err(AnalyticsError::InvalidQuality);
            }
        }
        Ok(Self {
            sequence,
            event_ts_ns,
            receive_ts_ns,
            bid_price,
            ask_price,
        })
    }

    /// Creates feed-quality context from a quote.
    pub const fn from_quote(
        sequence: Option<u64>,
        quote: QuoteContext,
        receive_ts_ns: u64,
    ) -> Result<Self, AnalyticsError> {
        Self::new(
            sequence,
            quote.ts_ns(),
            receive_ts_ns,
            Some(quote.bid_price()),
            Some(quote.ask_price()),
        )
    }

    /// Returns optional venue/provider sequence.
    pub const fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    /// Returns event timestamp.
    pub const fn event_ts_ns(&self) -> u64 {
        self.event_ts_ns
    }

    /// Returns local receive timestamp.
    pub const fn receive_ts_ns(&self) -> u64 {
        self.receive_ts_ns
    }

    /// Returns optional best bid price.
    pub const fn bid_price(&self) -> Option<i64> {
        self.bid_price
    }

    /// Returns optional best ask price.
    pub const fn ask_price(&self) -> Option<i64> {
        self.ask_price
    }
}

/// Cumulative feed-quality snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedQualitySnapshot {
    events: u64,
    sequence_gap_events: u64,
    sequence_gap_units: u64,
    out_of_order_events: u64,
    duplicate_events: u64,
    stale_events: u64,
    locked_book_events: u64,
    crossed_book_events: u64,
    timestamp_skew_events: u64,
    sequence_reset_events: u64,
    last_sequence: Option<u64>,
    last_event_ts_ns: u64,
    flags: FeedQualityFlags,
    health_score_bps: u16,
}

impl FeedQualitySnapshot {
    /// Returns observed event count.
    pub const fn events(&self) -> u64 {
        self.events
    }

    /// Returns sequence gap event count.
    pub const fn sequence_gap_events(&self) -> u64 {
        self.sequence_gap_events
    }

    /// Returns total missing sequence units.
    pub const fn sequence_gap_units(&self) -> u64 {
        self.sequence_gap_units
    }

    /// Returns out-of-order event count.
    pub const fn out_of_order_events(&self) -> u64 {
        self.out_of_order_events
    }

    /// Returns duplicate event count.
    pub const fn duplicate_events(&self) -> u64 {
        self.duplicate_events
    }

    /// Returns stale event count.
    pub const fn stale_events(&self) -> u64 {
        self.stale_events
    }

    /// Returns locked-book event count.
    pub const fn locked_book_events(&self) -> u64 {
        self.locked_book_events
    }

    /// Returns crossed-book event count.
    pub const fn crossed_book_events(&self) -> u64 {
        self.crossed_book_events
    }

    /// Returns timestamp-skew event count.
    pub const fn timestamp_skew_events(&self) -> u64 {
        self.timestamp_skew_events
    }

    /// Returns sequence-reset event count.
    pub const fn sequence_reset_events(&self) -> u64 {
        self.sequence_reset_events
    }

    /// Returns latest accepted sequence.
    pub const fn last_sequence(&self) -> Option<u64> {
        self.last_sequence
    }

    /// Returns latest event timestamp.
    pub const fn last_event_ts_ns(&self) -> u64 {
        self.last_event_ts_ns
    }

    /// Returns cumulative degradation flags.
    pub const fn flags(&self) -> FeedQualityFlags {
        self.flags
    }

    /// Returns aggregate health score in basis points, where 10,000 is best.
    pub const fn health_score_bps(&self) -> u16 {
        self.health_score_bps
    }

    /// Returns sequence gap event rate in basis points.
    pub fn sequence_gap_rate_bps(&self) -> u16 {
        rate_bps(self.sequence_gap_events, self.events)
    }

    /// Returns out-of-order event rate in basis points.
    pub fn out_of_order_rate_bps(&self) -> u16 {
        rate_bps(self.out_of_order_events, self.events)
    }

    /// Returns duplicate event rate in basis points.
    pub fn duplicate_rate_bps(&self) -> u16 {
        rate_bps(self.duplicate_events, self.events)
    }

    /// Returns stale event rate in basis points.
    pub fn stale_rate_bps(&self) -> u16 {
        rate_bps(self.stale_events, self.events)
    }

    /// Returns bad top-of-book rate in basis points.
    pub fn bad_book_rate_bps(&self) -> u16 {
        rate_bps(
            self.locked_book_events
                .saturating_add(self.crossed_book_events),
            self.events,
        )
    }
}

/// Allocation-free feed-quality tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedQualityTracker {
    config: FeedQualityConfig,
    events: u64,
    sequence_gap_events: u64,
    sequence_gap_units: u64,
    out_of_order_events: u64,
    duplicate_events: u64,
    stale_events: u64,
    locked_book_events: u64,
    crossed_book_events: u64,
    timestamp_skew_events: u64,
    sequence_reset_events: u64,
    last_sequence: Option<u64>,
    last_event_ts_ns: u64,
    flags: FeedQualityFlags,
}

impl FeedQualityTracker {
    /// Creates a feed-quality tracker.
    pub const fn new(config: FeedQualityConfig) -> Self {
        Self {
            config,
            events: 0,
            sequence_gap_events: 0,
            sequence_gap_units: 0,
            out_of_order_events: 0,
            duplicate_events: 0,
            stale_events: 0,
            locked_book_events: 0,
            crossed_book_events: 0,
            timestamp_skew_events: 0,
            sequence_reset_events: 0,
            last_sequence: None,
            last_event_ts_ns: 0,
            flags: FeedQualityFlags::OK,
        }
    }

    /// Returns tracker configuration.
    pub const fn config(&self) -> FeedQualityConfig {
        self.config
    }

    /// Records one market-data event and returns flags for that event.
    pub fn on_event(&mut self, event: FeedQualityEvent) -> FeedQualityFlags {
        self.events = self.events.saturating_add(1);
        let mut event_flags = FeedQualityFlags::OK;
        event_flags = self.observe_sequence(event.sequence(), event_flags);
        event_flags =
            self.observe_timestamps(event.event_ts_ns(), event.receive_ts_ns(), event_flags);
        event_flags = self.observe_book(event.bid_price(), event.ask_price(), event_flags);
        self.flags = self.flags.union(event_flags);
        event_flags
    }

    /// Returns current snapshot.
    pub fn snapshot(&self) -> FeedQualitySnapshot {
        FeedQualitySnapshot {
            events: self.events,
            sequence_gap_events: self.sequence_gap_events,
            sequence_gap_units: self.sequence_gap_units,
            out_of_order_events: self.out_of_order_events,
            duplicate_events: self.duplicate_events,
            stale_events: self.stale_events,
            locked_book_events: self.locked_book_events,
            crossed_book_events: self.crossed_book_events,
            timestamp_skew_events: self.timestamp_skew_events,
            sequence_reset_events: self.sequence_reset_events,
            last_sequence: self.last_sequence,
            last_event_ts_ns: self.last_event_ts_ns,
            flags: self.flags,
            health_score_bps: self.health_score_bps(),
        }
    }

    /// Clears accumulated counters and last-seen state.
    pub fn reset(&mut self) {
        self.events = 0;
        self.sequence_gap_events = 0;
        self.sequence_gap_units = 0;
        self.out_of_order_events = 0;
        self.duplicate_events = 0;
        self.stale_events = 0;
        self.locked_book_events = 0;
        self.crossed_book_events = 0;
        self.timestamp_skew_events = 0;
        self.sequence_reset_events = 0;
        self.last_sequence = None;
        self.last_event_ts_ns = 0;
        self.flags = FeedQualityFlags::OK;
    }

    fn observe_sequence(
        &mut self,
        sequence: Option<u64>,
        mut flags: FeedQualityFlags,
    ) -> FeedQualityFlags {
        if let Some(sequence) = sequence {
            match self.last_sequence {
                Some(last) if sequence == last => {
                    self.duplicate_events = self.duplicate_events.saturating_add(1);
                    flags = flags.union(FeedQualityFlags::DUPLICATE);
                }
                Some(last) if sequence < last => {
                    if sequence <= self.config.expected_sequence_step() {
                        self.sequence_reset_events = self.sequence_reset_events.saturating_add(1);
                        flags = flags.union(FeedQualityFlags::SEQUENCE_RESET);
                        self.last_sequence = Some(sequence);
                    } else {
                        self.out_of_order_events = self.out_of_order_events.saturating_add(1);
                        flags = flags.union(FeedQualityFlags::OUT_OF_ORDER);
                    }
                }
                Some(last) => {
                    let expected = last.saturating_add(self.config.expected_sequence_step());
                    if sequence > expected {
                        self.sequence_gap_events = self.sequence_gap_events.saturating_add(1);
                        self.sequence_gap_units = self
                            .sequence_gap_units
                            .saturating_add(sequence.saturating_sub(expected));
                        flags = flags.union(FeedQualityFlags::SEQUENCE_GAP);
                    }
                    self.last_sequence = Some(sequence);
                }
                None => self.last_sequence = Some(sequence),
            }
        }
        flags
    }

    fn observe_timestamps(
        &mut self,
        event_ts_ns: u64,
        receive_ts_ns: u64,
        mut flags: FeedQualityFlags,
    ) -> FeedQualityFlags {
        if self.last_event_ts_ns != 0 && event_ts_ns < self.last_event_ts_ns {
            self.out_of_order_events = self.out_of_order_events.saturating_add(1);
            flags = flags.union(FeedQualityFlags::OUT_OF_ORDER);
        }
        if receive_ts_ns.saturating_sub(event_ts_ns) > self.config.stale_after_ns() {
            self.stale_events = self.stale_events.saturating_add(1);
            flags = flags.union(FeedQualityFlags::STALE);
        }
        if receive_ts_ns.abs_diff(event_ts_ns) > self.config.max_timestamp_skew_ns() {
            self.timestamp_skew_events = self.timestamp_skew_events.saturating_add(1);
            flags = flags.union(FeedQualityFlags::TIMESTAMP_SKEW);
        }
        self.last_event_ts_ns = self.last_event_ts_ns.max(event_ts_ns);
        flags
    }

    fn observe_book(
        &mut self,
        bid_price: Option<i64>,
        ask_price: Option<i64>,
        mut flags: FeedQualityFlags,
    ) -> FeedQualityFlags {
        if let (Some(bid), Some(ask)) = (bid_price, ask_price) {
            if bid > ask {
                self.crossed_book_events = self.crossed_book_events.saturating_add(1);
                flags = flags.union(FeedQualityFlags::CROSSED_BOOK);
            } else if bid == ask {
                self.locked_book_events = self.locked_book_events.saturating_add(1);
                flags = flags.union(FeedQualityFlags::LOCKED_BOOK);
            }
        }
        flags
    }

    fn health_score_bps(&self) -> u16 {
        if self.events == 0 {
            return 10_000;
        }
        let weighted_penalty = self
            .sequence_gap_events
            .saturating_mul(1_000)
            .saturating_add(self.out_of_order_events.saturating_mul(1_500))
            .saturating_add(self.duplicate_events.saturating_mul(500))
            .saturating_add(self.stale_events.saturating_mul(800))
            .saturating_add(self.locked_book_events.saturating_mul(600))
            .saturating_add(self.crossed_book_events.saturating_mul(2_000))
            .saturating_add(self.timestamp_skew_events.saturating_mul(1_000))
            .saturating_add(self.sequence_reset_events.saturating_mul(1_500));
        let penalty_bps = weighted_penalty / self.events;
        u16::try_from(10_000_u64.saturating_sub(penalty_bps.min(10_000))).unwrap_or(0)
    }
}

impl Default for FeedQualityTracker {
    fn default() -> Self {
        Self::new(FeedQualityConfig::default())
    }
}

/// Replay-quality report thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayQualityConfig {
    min_events: u64,
    min_health_score_bps: u16,
    max_issue_rate_bps: u16,
    max_sequence_gap_units: u64,
}

impl ReplayQualityConfig {
    /// Creates replay-quality report thresholds.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidQuality`] when basis-point thresholds
    /// exceed 10,000.
    pub const fn new(
        min_events: u64,
        min_health_score_bps: u16,
        max_issue_rate_bps: u16,
        max_sequence_gap_units: u64,
    ) -> Result<Self, AnalyticsError> {
        if min_health_score_bps > 10_000 || max_issue_rate_bps > 10_000 {
            return Err(AnalyticsError::InvalidQuality);
        }
        Ok(Self {
            min_events,
            min_health_score_bps,
            max_issue_rate_bps,
            max_sequence_gap_units,
        })
    }

    /// Returns minimum event count required for replay usability.
    pub const fn min_events(&self) -> u64 {
        self.min_events
    }

    /// Returns minimum health score in basis points.
    pub const fn min_health_score_bps(&self) -> u16 {
        self.min_health_score_bps
    }

    /// Returns maximum acceptable per-issue rate in basis points.
    pub const fn max_issue_rate_bps(&self) -> u16 {
        self.max_issue_rate_bps
    }

    /// Returns maximum acceptable missing sequence units.
    pub const fn max_sequence_gap_units(&self) -> u64 {
        self.max_sequence_gap_units
    }
}

impl Default for ReplayQualityConfig {
    fn default() -> Self {
        Self {
            min_events: 1,
            min_health_score_bps: 9_000,
            max_issue_rate_bps: 100,
            max_sequence_gap_units: 0,
        }
    }
}

/// Replay-quality report derived from feed-quality counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayQualityReport {
    events: u64,
    min_events_met: bool,
    health_score_bps: u16,
    sequence_gap_rate_bps: u16,
    out_of_order_rate_bps: u16,
    duplicate_rate_bps: u16,
    stale_rate_bps: u16,
    bad_book_rate_bps: u16,
    timestamp_skew_rate_bps: u16,
    sequence_reset_rate_bps: u16,
    worst_issue_rate_bps: u16,
    sequence_gap_units: u64,
    sequence_complete: bool,
    primary_issue: FeedQualityFlags,
    flags: FeedQualityFlags,
    replay_usable: bool,
    operator_review_required: bool,
}

impl ReplayQualityReport {
    /// Returns observed event count.
    pub const fn events(&self) -> u64 {
        self.events
    }

    /// Returns true when the report has enough events for the configured gate.
    pub const fn min_events_met(&self) -> bool {
        self.min_events_met
    }

    /// Returns aggregate health score in basis points.
    pub const fn health_score_bps(&self) -> u16 {
        self.health_score_bps
    }

    /// Returns sequence gap event rate in basis points.
    pub const fn sequence_gap_rate_bps(&self) -> u16 {
        self.sequence_gap_rate_bps
    }

    /// Returns out-of-order event rate in basis points.
    pub const fn out_of_order_rate_bps(&self) -> u16 {
        self.out_of_order_rate_bps
    }

    /// Returns duplicate event rate in basis points.
    pub const fn duplicate_rate_bps(&self) -> u16 {
        self.duplicate_rate_bps
    }

    /// Returns stale event rate in basis points.
    pub const fn stale_rate_bps(&self) -> u16 {
        self.stale_rate_bps
    }

    /// Returns locked/crossed top-of-book event rate in basis points.
    pub const fn bad_book_rate_bps(&self) -> u16 {
        self.bad_book_rate_bps
    }

    /// Returns timestamp-skew event rate in basis points.
    pub const fn timestamp_skew_rate_bps(&self) -> u16 {
        self.timestamp_skew_rate_bps
    }

    /// Returns sequence-reset event rate in basis points.
    pub const fn sequence_reset_rate_bps(&self) -> u16 {
        self.sequence_reset_rate_bps
    }

    /// Returns highest individual issue rate in basis points.
    pub const fn worst_issue_rate_bps(&self) -> u16 {
        self.worst_issue_rate_bps
    }

    /// Returns total missing sequence units.
    pub const fn sequence_gap_units(&self) -> u64 {
        self.sequence_gap_units
    }

    /// Returns true when missing sequence units stay within threshold.
    pub const fn sequence_complete(&self) -> bool {
        self.sequence_complete
    }

    /// Returns the primary issue flag selected by severity-weighted rate.
    pub const fn primary_issue(&self) -> FeedQualityFlags {
        self.primary_issue
    }

    /// Returns cumulative feed-quality flags.
    pub const fn flags(&self) -> FeedQualityFlags {
        self.flags
    }

    /// Returns true when replay quality passes configured gates.
    pub const fn replay_usable(&self) -> bool {
        self.replay_usable
    }

    /// Returns true when an operator should review replay quality.
    pub const fn operator_review_required(&self) -> bool {
        self.operator_review_required
    }
}

/// Deterministic replay-quality report analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayQualityAnalyzer {
    config: ReplayQualityConfig,
}

impl ReplayQualityAnalyzer {
    /// Creates a replay-quality analyzer.
    pub const fn new(config: ReplayQualityConfig) -> Self {
        Self { config }
    }

    /// Returns analyzer configuration.
    pub const fn config(&self) -> ReplayQualityConfig {
        self.config
    }

    /// Builds a replay-quality report from a feed-quality snapshot.
    pub fn evaluate(&self, snapshot: FeedQualitySnapshot) -> ReplayQualityReport {
        let sequence_gap_rate = snapshot.sequence_gap_rate_bps();
        let out_of_order_rate = snapshot.out_of_order_rate_bps();
        let duplicate_rate = snapshot.duplicate_rate_bps();
        let stale_rate = snapshot.stale_rate_bps();
        let bad_book_rate = snapshot.bad_book_rate_bps();
        let timestamp_skew_rate = rate_bps(snapshot.timestamp_skew_events(), snapshot.events());
        let sequence_reset_rate = rate_bps(snapshot.sequence_reset_events(), snapshot.events());
        let worst_issue_rate = max_u16(&[
            sequence_gap_rate,
            out_of_order_rate,
            duplicate_rate,
            stale_rate,
            bad_book_rate,
            timestamp_skew_rate,
            sequence_reset_rate,
        ]);
        let min_events_met = snapshot.events() >= self.config.min_events();
        let sequence_complete =
            snapshot.sequence_gap_units() <= self.config.max_sequence_gap_units();
        let replay_usable = min_events_met
            && sequence_complete
            && snapshot.health_score_bps() >= self.config.min_health_score_bps()
            && worst_issue_rate <= self.config.max_issue_rate_bps();
        ReplayQualityReport {
            events: snapshot.events(),
            min_events_met,
            health_score_bps: snapshot.health_score_bps(),
            sequence_gap_rate_bps: sequence_gap_rate,
            out_of_order_rate_bps: out_of_order_rate,
            duplicate_rate_bps: duplicate_rate,
            stale_rate_bps: stale_rate,
            bad_book_rate_bps: bad_book_rate,
            timestamp_skew_rate_bps: timestamp_skew_rate,
            sequence_reset_rate_bps: sequence_reset_rate,
            worst_issue_rate_bps: worst_issue_rate,
            sequence_gap_units: snapshot.sequence_gap_units(),
            sequence_complete,
            primary_issue: primary_feed_quality_issue(
                sequence_gap_rate,
                out_of_order_rate,
                duplicate_rate,
                stale_rate,
                bad_book_rate,
                timestamp_skew_rate,
                sequence_reset_rate,
            ),
            flags: snapshot.flags(),
            replay_usable,
            operator_review_required: !replay_usable || !snapshot.flags().is_ok(),
        }
    }
}

impl Default for ReplayQualityAnalyzer {
    fn default() -> Self {
        Self::new(ReplayQualityConfig::default())
    }
}
