#![doc = include_str!("../README.md")]

use std::fmt;
use std::ops::BitOr;
use std::collections::HashMap;

/// Canonical market symbol identifier used across venues.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId {
    /// Venue/exchange identifier, e.g. `CME` or `BINANCE`.
    pub venue: String,
    /// Instrument symbol in venue format.
    pub symbol: String,
}

/// Trade or book side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Bid/buy side.
    Bid,
    /// Ask/sell side.
    Ask,
}

/// Book mutation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookAction {
    /// Insert or update a price level.
    Upsert,
    /// Remove a price level.
    Delete,
}

/// Level-2 order book update.
#[derive(Debug, Clone)]
pub struct BookUpdate {
    /// Symbol that produced the update.
    pub symbol: SymbolId,
    /// Side being mutated.
    pub side: Side,
    /// Level index from top of book.
    pub level: u16,
    /// Price in integer ticks or price units.
    pub price: i64,
    /// Quantity/size at level.
    pub size: i64,
    /// Mutation operation.
    pub action: BookAction,
    /// Venue sequence number when available.
    pub sequence: u64,
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// Last-trade print/tick.
#[derive(Debug, Clone)]
pub struct TradePrint {
    /// Symbol that traded.
    pub symbol: SymbolId,
    /// Trade price.
    pub price: i64,
    /// Trade size.
    pub size: i64,
    /// Aggressor side for the print.
    pub aggressor_side: Side,
    /// Venue sequence number when available.
    pub sequence: u64,
    /// Exchange timestamp in nanoseconds.
    pub ts_exchange_ns: u64,
    /// Local receive timestamp in nanoseconds.
    pub ts_recv_ns: u64,
}

/// Aggregated analytics for a symbol/session.
#[derive(Debug, Clone, Default)]
pub struct AnalyticsSnapshot {
    /// Session delta (buy minus sell).
    pub delta: i64,
    /// Cumulative delta across session.
    pub cumulative_delta: i64,
    /// Total buy-side volume.
    pub buy_volume: i64,
    /// Total sell-side volume.
    pub sell_volume: i64,
    /// Last traded price.
    pub last_price: i64,
    /// Point of control (highest volume price).
    pub point_of_control: i64,
    /// Lower bound of value area.
    pub value_area_low: i64,
    /// Upper bound of value area.
    pub value_area_high: i64,
}

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

#[derive(Debug, Default)]
pub struct AnalyticsAccumulator {
    snapshot: AnalyticsSnapshot,
    volume_profile: HashMap<i64, i64>,
}

impl AnalyticsAccumulator {
    /// Applies a trade print to analytics and recomputes profile levels.
    pub fn on_trade(&mut self, trade: &TradePrint) {
        self.snapshot.last_price = trade.price;
        *self.volume_profile.entry(trade.price).or_insert(0) += trade.size;
        match trade.aggressor_side {
            Side::Bid => {
                self.snapshot.sell_volume += trade.size;
                self.snapshot.delta -= trade.size;
                self.snapshot.cumulative_delta -= trade.size;
            }
            Side::Ask => {
                self.snapshot.buy_volume += trade.size;
                self.snapshot.delta += trade.size;
                self.snapshot.cumulative_delta += trade.size;
            }
        }
        self.recompute_profile_levels();
    }

    /// Resets session delta and directional volume, keeps cumulative profile.
    pub fn reset_session_delta(&mut self) {
        self.snapshot.delta = 0;
        self.snapshot.buy_volume = 0;
        self.snapshot.sell_volume = 0;
    }

    /// Resets all session analytics and volume-profile state.
    pub fn reset_session(&mut self) {
        self.snapshot = AnalyticsSnapshot::default();
        self.volume_profile.clear();
    }

    /// Returns a copy of current analytics state.
    pub fn snapshot(&self) -> AnalyticsSnapshot {
        self.snapshot.clone()
    }

    fn recompute_profile_levels(&mut self) {
        if self.volume_profile.is_empty() {
            return;
        }

        let mut prices: Vec<i64> = self.volume_profile.keys().copied().collect();
        prices.sort_unstable();
        let total_volume: i64 = self.volume_profile.values().sum();
        if total_volume <= 0 {
            return;
        }

        let mut poc_price = prices[0];
        let mut poc_volume = self.volume_profile[&poc_price];
        for p in &prices {
            let v = self.volume_profile[p];
            if v > poc_volume || (v == poc_volume && *p > poc_price) {
                poc_price = *p;
                poc_volume = v;
            }
        }
        self.snapshot.point_of_control = poc_price;

        let target = ((total_volume as f64) * 0.70).ceil() as i64;
        let mut covered = poc_volume;
        let mut low = poc_price;
        let mut high = poc_price;

        let poc_idx = prices.iter().position(|p| *p == poc_price).unwrap_or(0);
        let mut left: isize = poc_idx as isize - 1;
        let mut right: usize = poc_idx + 1;

        while covered < target && (left >= 0 || right < prices.len()) {
            let left_vol = if left >= 0 {
                self.volume_profile[&prices[left as usize]]
            } else {
                -1
            };
            let right_vol = if right < prices.len() {
                self.volume_profile[&prices[right]]
            } else {
                -1
            };

            if right_vol > left_vol {
                covered += right_vol.max(0);
                high = prices[right];
                right += 1;
            } else {
                covered += left_vol.max(0);
                low = prices[left as usize];
                left -= 1;
            }
        }

        self.snapshot.value_area_low = low;
        self.snapshot.value_area_high = high;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol() -> SymbolId {
        SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        }
    }

    #[test]
    fn tracks_delta_and_cumulative_delta() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 100,
            size: 5,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 99,
            size: 2,
            aggressor_side: Side::Bid,
            sequence: 2,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });

        let snap = acc.snapshot();
        assert_eq!(snap.delta, 3);
        assert_eq!(snap.cumulative_delta, 3);
        assert_eq!(snap.buy_volume, 5);
        assert_eq!(snap.sell_volume, 2);
        assert_eq!(snap.last_price, 99);
        assert_eq!(snap.point_of_control, 100);
        assert_eq!(snap.value_area_low, 100);
        assert_eq!(snap.value_area_high, 100);

        acc.reset_session_delta();
        let reset = acc.snapshot();
        assert_eq!(reset.delta, 0);
        assert_eq!(reset.buy_volume, 0);
        assert_eq!(reset.sell_volume, 0);
        assert_eq!(reset.cumulative_delta, 3);
    }

    #[test]
    fn tracks_poc_and_value_area() {
        let mut acc = AnalyticsAccumulator::default();
        let s = symbol();
        let prints = [
            (100, 5, Side::Ask),
            (101, 7, Side::Ask),
            (99, 3, Side::Bid),
            (102, 2, Side::Ask),
            (101, 5, Side::Bid),
        ];
        for (i, (price, size, side)) in prints.iter().enumerate() {
            acc.on_trade(&TradePrint {
                symbol: s.clone(),
                price: *price,
                size: *size,
                aggressor_side: *side,
                sequence: i as u64 + 1,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            });
        }
        let snap = acc.snapshot();
        assert_eq!(snap.point_of_control, 101);
        assert!(snap.value_area_low <= snap.point_of_control);
        assert!(snap.value_area_high >= snap.point_of_control);
    }

    #[test]
    fn full_session_reset_clears_profile_and_cumulative() {
        let mut acc = AnalyticsAccumulator::default();
        acc.on_trade(&TradePrint {
            symbol: symbol(),
            price: 101,
            size: 4,
            aggressor_side: Side::Ask,
            sequence: 1,
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
        });
        acc.reset_session();
        let snap = acc.snapshot();
        assert_eq!(snap.delta, 0);
        assert_eq!(snap.cumulative_delta, 0);
        assert_eq!(snap.buy_volume, 0);
        assert_eq!(snap.sell_volume, 0);
        assert_eq!(snap.point_of_control, 0);
        assert_eq!(snap.value_area_low, 0);
        assert_eq!(snap.value_area_high, 0);
    }
}
