use of_core::{AnalyticsSnapshot, DataQualityFlags, SignalSnapshot, SignalState};

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_on_quality_issues() {
        let s = DeltaMomentumSignal::default();
        let decision = s.quality_gate(DataQualityFlags::SEQUENCE_GAP);
        assert_eq!(decision, SignalGateDecision::Block);
    }
}
