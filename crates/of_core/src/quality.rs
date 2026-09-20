use super::*;

/// Output state emitted by signal modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalState {
    /// No directional bias.
    Neutral,
    /// Long/buy bias.
    LongBias,
    /// Short/sell bias.
    ShortBias,
    /// Blocked due to data-quality gating.
    Blocked,
}

/// Snapshot of a signal module evaluation.
#[derive(Debug, Clone)]
pub struct SignalSnapshot {
    /// Stable signal module identifier.
    pub module_id: &'static str,
    /// Current state.
    pub state: SignalState,
    /// Confidence in basis points.
    pub confidence_bps: u16,
    /// Active quality flags bits.
    pub quality_flags: u32,
    /// Human-readable reason for current state.
    pub reason: String,
}

/// Bitset wrapper for feed-quality flags.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataQualityFlags(u32);

impl DataQualityFlags {
    /// No quality issues detected.
    pub const NONE: Self = Self(0);
    /// Feed is stale beyond policy threshold.
    pub const STALE_FEED: Self = Self(1 << 0);
    /// A sequence number gap was detected.
    pub const SEQUENCE_GAP: Self = Self(1 << 1);
    /// Clock skew detected between source and consumer.
    pub const CLOCK_SKEW: Self = Self(1 << 2);
    /// Book depth was truncated.
    pub const DEPTH_TRUNCATED: Self = Self(1 << 3);
    /// Event arrived out-of-order.
    pub const OUT_OF_ORDER: Self = Self(1 << 4);
    /// Adapter/external feed is degraded or reconnecting.
    pub const ADAPTER_DEGRADED: Self = Self(1 << 5);

    /// Returns raw bit representation.
    pub fn bits(self) -> u32 {
        self.0
    }

    /// Builds flags from raw bits, preserving unknown bits.
    pub fn from_bits_truncate(bits: u32) -> Self {
        Self(bits)
    }

    /// Returns true when any flag in `other` is set in `self`.
    pub fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

impl fmt::Debug for DataQualityFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataQualityFlags({:#x})", self.0)
    }
}

impl BitOr for DataQualityFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
