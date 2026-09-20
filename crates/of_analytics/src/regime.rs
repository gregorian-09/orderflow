use super::*;

/// Market regime classification.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RegimeKind {
    /// Quiet conditions.
    Quiet = 1,
    /// Normal conditions.
    Normal = 2,
    /// Volatility is elevated.
    Volatile = 3,
    /// Toxic flow or adverse-selection risk is elevated.
    Toxic = 4,
    /// Spread/imbalance suggests illiquidity.
    Illiquid = 5,
}

/// Regime classifier input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegimeInput {
    spread_bps: u32,
    volatility_bps: u32,
    toxicity_bps: u16,
    imbalance_abs_bps: u16,
}

impl RegimeInput {
    /// Creates regime input.
    pub const fn new(
        spread_bps: u32,
        volatility_bps: u32,
        toxicity_bps: u16,
        imbalance_abs_bps: u16,
    ) -> Self {
        Self {
            spread_bps,
            volatility_bps,
            toxicity_bps,
            imbalance_abs_bps,
        }
    }

    /// Returns spread in basis points.
    pub const fn spread_bps(&self) -> u32 {
        self.spread_bps
    }

    /// Returns volatility in basis points.
    pub const fn volatility_bps(&self) -> u32 {
        self.volatility_bps
    }

    /// Returns toxicity in basis points.
    pub const fn toxicity_bps(&self) -> u16 {
        self.toxicity_bps
    }

    /// Returns absolute imbalance in basis points.
    pub const fn imbalance_abs_bps(&self) -> u16 {
        self.imbalance_abs_bps
    }
}

/// Regime snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegimeSnapshot {
    kind: RegimeKind,
    stress_bps: u16,
}

impl RegimeSnapshot {
    /// Returns regime kind.
    pub const fn kind(&self) -> RegimeKind {
        self.kind
    }

    /// Returns aggregate stress score in basis points.
    pub const fn stress_bps(&self) -> u16 {
        self.stress_bps
    }
}

/// Threshold-based market regime classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegimeClassifier {
    quiet_vol_bps: u32,
    volatile_vol_bps: u32,
    wide_spread_bps: u32,
    toxic_bps: u16,
    imbalance_bps: u16,
}

impl RegimeClassifier {
    /// Creates regime classifier thresholds.
    pub const fn new(
        quiet_vol_bps: u32,
        volatile_vol_bps: u32,
        wide_spread_bps: u32,
        toxic_bps: u16,
        imbalance_bps: u16,
    ) -> Self {
        Self {
            quiet_vol_bps,
            volatile_vol_bps,
            wide_spread_bps,
            toxic_bps,
            imbalance_bps,
        }
    }

    /// Classifies a regime input.
    pub fn classify(&self, input: RegimeInput) -> RegimeSnapshot {
        let kind = if input.toxicity_bps() >= self.toxic_bps {
            RegimeKind::Toxic
        } else if input.spread_bps() >= self.wide_spread_bps
            || input.imbalance_abs_bps() >= self.imbalance_bps
        {
            RegimeKind::Illiquid
        } else if input.volatility_bps() >= self.volatile_vol_bps {
            RegimeKind::Volatile
        } else if input.volatility_bps() <= self.quiet_vol_bps && input.spread_bps() == 0 {
            RegimeKind::Quiet
        } else {
            RegimeKind::Normal
        };
        let stress = input
            .volatility_bps()
            .min(10_000)
            .max(u32::from(input.toxicity_bps()))
            .max(u32::from(input.imbalance_abs_bps()))
            .max(input.spread_bps().min(10_000));
        RegimeSnapshot {
            kind,
            stress_bps: u16::try_from(stress).unwrap_or(10_000),
        }
    }
}

impl Default for RegimeClassifier {
    fn default() -> Self {
        Self::new(5, 50, 25, 7_500, 7_500)
    }
}

/// Trend/range/chop regime label.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TrendRegimeKind {
    /// Directional trend conditions.
    Trend = 1,
    /// Range-bound conditions.
    Range = 2,
    /// Choppy, reversal-prone conditions.
    Chop = 3,
}

/// Liquidity regime label.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LiquidityRegimeKind {
    /// Deep displayed liquidity.
    Deep = 1,
    /// Normal displayed liquidity.
    Normal = 2,
    /// Thin displayed liquidity.
    Thin = 3,
    /// Hidden-liquidity proxy is elevated.
    Hidden = 4,
}

/// Spread regime label.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpreadRegimeKind {
    /// Tight spread conditions.
    Tight = 1,
    /// Normal spread conditions.
    Normal = 2,
    /// Wide spread conditions.
    Wide = 3,
}

/// Session phase regime label.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SessionRegimeKind {
    /// Continuous trading phase.
    Continuous = 1,
    /// Auction or uncrossing phase.
    Auction = 2,
    /// Opening window.
    Open = 3,
    /// Closing window.
    Close = 4,
    /// News-shock or event-risk window.
    NewsShock = 5,
}

/// Composite regime classifier configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeRegimeConfig {
    trend_threshold_bps: u16,
    chop_threshold_bps: u16,
    wide_spread_bps: u32,
    tight_spread_bps: u32,
    volatile_threshold_bps: u32,
    thin_depth: i64,
    deep_depth: i64,
    open_window_ns: u64,
    close_window_ns: u64,
    news_shock_threshold_bps: u16,
    hidden_liquidity_threshold_bps: u16,
}

impl CompositeRegimeConfig {
    /// Creates composite regime configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when thresholds are
    /// inconsistent or basis-point values exceed 10,000.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        trend_threshold_bps: u16,
        chop_threshold_bps: u16,
        wide_spread_bps: u32,
        tight_spread_bps: u32,
        volatile_threshold_bps: u32,
        thin_depth: i64,
        deep_depth: i64,
        open_window_ns: u64,
        close_window_ns: u64,
        news_shock_threshold_bps: u16,
        hidden_liquidity_threshold_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if trend_threshold_bps > 10_000
            || chop_threshold_bps > 10_000
            || news_shock_threshold_bps > 10_000
            || hidden_liquidity_threshold_bps > 10_000
            || thin_depth < 0
            || deep_depth < thin_depth
            || tight_spread_bps > wide_spread_bps
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            trend_threshold_bps,
            chop_threshold_bps,
            wide_spread_bps,
            tight_spread_bps,
            volatile_threshold_bps,
            thin_depth,
            deep_depth,
            open_window_ns,
            close_window_ns,
            news_shock_threshold_bps,
            hidden_liquidity_threshold_bps,
        })
    }

    /// Returns trend threshold in basis points.
    pub const fn trend_threshold_bps(&self) -> u16 {
        self.trend_threshold_bps
    }

    /// Returns chop threshold in basis points.
    pub const fn chop_threshold_bps(&self) -> u16 {
        self.chop_threshold_bps
    }

    /// Returns wide-spread threshold in basis points.
    pub const fn wide_spread_bps(&self) -> u32 {
        self.wide_spread_bps
    }

    /// Returns tight-spread threshold in basis points.
    pub const fn tight_spread_bps(&self) -> u32 {
        self.tight_spread_bps
    }

    /// Returns volatile threshold in basis points.
    pub const fn volatile_threshold_bps(&self) -> u32 {
        self.volatile_threshold_bps
    }

    /// Returns thin-depth threshold.
    pub const fn thin_depth(&self) -> i64 {
        self.thin_depth
    }

    /// Returns deep-depth threshold.
    pub const fn deep_depth(&self) -> i64 {
        self.deep_depth
    }

    /// Returns open-window duration in nanoseconds.
    pub const fn open_window_ns(&self) -> u64 {
        self.open_window_ns
    }

    /// Returns close-window duration in nanoseconds.
    pub const fn close_window_ns(&self) -> u64 {
        self.close_window_ns
    }

    /// Returns news-shock threshold in basis points.
    pub const fn news_shock_threshold_bps(&self) -> u16 {
        self.news_shock_threshold_bps
    }

    /// Returns hidden-liquidity threshold in basis points.
    pub const fn hidden_liquidity_threshold_bps(&self) -> u16 {
        self.hidden_liquidity_threshold_bps
    }
}

impl Default for CompositeRegimeConfig {
    fn default() -> Self {
        Self {
            trend_threshold_bps: 6_000,
            chop_threshold_bps: 6_000,
            wide_spread_bps: 25,
            tight_spread_bps: 5,
            volatile_threshold_bps: 75,
            thin_depth: 100,
            deep_depth: 1_000,
            open_window_ns: 15 * 60 * 1_000_000_000,
            close_window_ns: 15 * 60 * 1_000_000_000,
            news_shock_threshold_bps: 7_000,
            hidden_liquidity_threshold_bps: 7_000,
        }
    }
}

/// Composite regime classifier input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeRegimeInput {
    trend_strength_bps: u16,
    chop_score_bps: u16,
    spread_bps: u32,
    volatility_bps: u32,
    displayed_depth: i64,
    elapsed_since_open_ns: u64,
    remaining_to_close_ns: u64,
    auction: bool,
    news_intensity_bps: u16,
    hidden_liquidity_proxy_bps: u16,
}

impl CompositeRegimeInput {
    /// Creates composite regime input.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidTrade`] when basis-point fields exceed
    /// 10,000 or displayed depth is negative.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        trend_strength_bps: u16,
        chop_score_bps: u16,
        spread_bps: u32,
        volatility_bps: u32,
        displayed_depth: i64,
        elapsed_since_open_ns: u64,
        remaining_to_close_ns: u64,
        auction: bool,
        news_intensity_bps: u16,
        hidden_liquidity_proxy_bps: u16,
    ) -> Result<Self, AnalyticsError> {
        if trend_strength_bps > 10_000
            || chop_score_bps > 10_000
            || displayed_depth < 0
            || news_intensity_bps > 10_000
            || hidden_liquidity_proxy_bps > 10_000
        {
            return Err(AnalyticsError::InvalidTrade);
        }
        Ok(Self {
            trend_strength_bps,
            chop_score_bps,
            spread_bps,
            volatility_bps,
            displayed_depth,
            elapsed_since_open_ns,
            remaining_to_close_ns,
            auction,
            news_intensity_bps,
            hidden_liquidity_proxy_bps,
        })
    }

    /// Returns trend-strength score in basis points.
    pub const fn trend_strength_bps(&self) -> u16 {
        self.trend_strength_bps
    }

    /// Returns chop score in basis points.
    pub const fn chop_score_bps(&self) -> u16 {
        self.chop_score_bps
    }

    /// Returns spread in basis points.
    pub const fn spread_bps(&self) -> u32 {
        self.spread_bps
    }

    /// Returns volatility in basis points.
    pub const fn volatility_bps(&self) -> u32 {
        self.volatility_bps
    }

    /// Returns displayed depth.
    pub const fn displayed_depth(&self) -> i64 {
        self.displayed_depth
    }

    /// Returns elapsed time since open in nanoseconds.
    pub const fn elapsed_since_open_ns(&self) -> u64 {
        self.elapsed_since_open_ns
    }

    /// Returns remaining time to close in nanoseconds.
    pub const fn remaining_to_close_ns(&self) -> u64 {
        self.remaining_to_close_ns
    }

    /// Returns whether the instrument is in an auction phase.
    pub const fn auction(&self) -> bool {
        self.auction
    }

    /// Returns news intensity in basis points.
    pub const fn news_intensity_bps(&self) -> u16 {
        self.news_intensity_bps
    }

    /// Returns hidden-liquidity proxy in basis points.
    pub const fn hidden_liquidity_proxy_bps(&self) -> u16 {
        self.hidden_liquidity_proxy_bps
    }
}

/// Composite regime snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeRegimeSnapshot {
    trend: TrendRegimeKind,
    liquidity: LiquidityRegimeKind,
    spread: SpreadRegimeKind,
    session: SessionRegimeKind,
    volatile: bool,
    hidden_liquidity_proxy_bps: u16,
    transition_confidence_bps: u16,
}

impl CompositeRegimeSnapshot {
    /// Returns trend/range/chop label.
    pub const fn trend(&self) -> TrendRegimeKind {
        self.trend
    }

    /// Returns liquidity label.
    pub const fn liquidity(&self) -> LiquidityRegimeKind {
        self.liquidity
    }

    /// Returns spread label.
    pub const fn spread(&self) -> SpreadRegimeKind {
        self.spread
    }

    /// Returns session phase label.
    pub const fn session(&self) -> SessionRegimeKind {
        self.session
    }

    /// Returns whether volatility is elevated.
    pub const fn volatile(&self) -> bool {
        self.volatile
    }

    /// Returns hidden-liquidity proxy in basis points.
    pub const fn hidden_liquidity_proxy_bps(&self) -> u16 {
        self.hidden_liquidity_proxy_bps
    }

    /// Returns transition confidence in basis points.
    pub const fn transition_confidence_bps(&self) -> u16 {
        self.transition_confidence_bps
    }
}

/// Deterministic composite regime classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeRegimeClassifier {
    config: CompositeRegimeConfig,
}

impl CompositeRegimeClassifier {
    /// Creates a composite regime classifier.
    pub const fn new(config: CompositeRegimeConfig) -> Self {
        Self { config }
    }

    /// Returns classifier configuration.
    pub const fn config(&self) -> CompositeRegimeConfig {
        self.config
    }

    /// Classifies composite market regime.
    pub fn classify(&self, input: CompositeRegimeInput) -> CompositeRegimeSnapshot {
        let trend = if input.chop_score_bps() >= self.config.chop_threshold_bps() {
            TrendRegimeKind::Chop
        } else if input.trend_strength_bps() >= self.config.trend_threshold_bps() {
            TrendRegimeKind::Trend
        } else {
            TrendRegimeKind::Range
        };
        let liquidity =
            if input.hidden_liquidity_proxy_bps() >= self.config.hidden_liquidity_threshold_bps() {
                LiquidityRegimeKind::Hidden
            } else if input.displayed_depth() <= self.config.thin_depth() {
                LiquidityRegimeKind::Thin
            } else if input.displayed_depth() >= self.config.deep_depth() {
                LiquidityRegimeKind::Deep
            } else {
                LiquidityRegimeKind::Normal
            };
        let spread = if input.spread_bps() >= self.config.wide_spread_bps() {
            SpreadRegimeKind::Wide
        } else if input.spread_bps() <= self.config.tight_spread_bps() {
            SpreadRegimeKind::Tight
        } else {
            SpreadRegimeKind::Normal
        };
        let session = if input.news_intensity_bps() >= self.config.news_shock_threshold_bps() {
            SessionRegimeKind::NewsShock
        } else if input.auction() {
            SessionRegimeKind::Auction
        } else if input.elapsed_since_open_ns() <= self.config.open_window_ns() {
            SessionRegimeKind::Open
        } else if input.remaining_to_close_ns() <= self.config.close_window_ns() {
            SessionRegimeKind::Close
        } else {
            SessionRegimeKind::Continuous
        };
        let transition_confidence_bps =
            transition_confidence_bps(input, self.config, trend, liquidity, spread, session);
        CompositeRegimeSnapshot {
            trend,
            liquidity,
            spread,
            session,
            volatile: input.volatility_bps() >= self.config.volatile_threshold_bps(),
            hidden_liquidity_proxy_bps: input.hidden_liquidity_proxy_bps(),
            transition_confidence_bps,
        }
    }
}

impl Default for CompositeRegimeClassifier {
    fn default() -> Self {
        Self::new(CompositeRegimeConfig::default())
    }
}
