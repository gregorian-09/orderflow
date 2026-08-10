package com.orderflow.bindings;

/** Replay markout, confidence, sample-retention, and timestamp policy. */
public final class SignalValidationConfig {
    /** Number of future events used for markout labels. */ public final long markoutHorizonEvents;
    /** Absolute price change treated as flat. */ public final long flatPriceThreshold;
    /** Minimum directional confidence in basis points. */ public final int minConfidenceBps;
    /** Whether per-event samples are retained. */ public final boolean storeSamples;
    /** Whether exchange timestamps must be monotonic. */ public final boolean checkMonotonicTimestamps;

    /** Creates an explicit replay-validation policy. */
    public SignalValidationConfig(
            long markoutHorizonEvents, long flatPriceThreshold, int minConfidenceBps,
            boolean storeSamples, boolean checkMonotonicTimestamps) {
        if (markoutHorizonEvents < 0 || markoutHorizonEvents > 0xffff_ffffL) {
            throw new IllegalArgumentException("markoutHorizonEvents must fit uint32");
        }
        if (minConfidenceBps < 0 || minConfidenceBps > 0xffff) {
            throw new IllegalArgumentException("minConfidenceBps must fit uint16");
        }
        this.markoutHorizonEvents = markoutHorizonEvents;
        this.flatPriceThreshold = flatPriceThreshold;
        this.minConfidenceBps = minConfidenceBps;
        this.storeSamples = storeSamples;
        this.checkMonotonicTimestamps = checkMonotonicTimestamps;
    }

    /** Returns the native validator defaults. */
    public static SignalValidationConfig defaults() {
        return new SignalValidationConfig(1, 0, 0, false, true);
    }
}
