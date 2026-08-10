package com.orderflow.bindings;

/** One analytics observation consumed by offline signal validation. */
public final class SignalValidationEvent {
    /** Session delta. */ public final long delta;
    /** Cumulative session delta. */ public final long cumulativeDelta;
    /** Buy-side volume. */ public final long buyVolume;
    /** Sell-side volume. */ public final long sellVolume;
    /** Last traded price. */ public final long lastPrice;
    /** Session point of control. */ public final long pointOfControl;
    /** Session value-area low. */ public final long valueAreaLow;
    /** Session value-area high. */ public final long valueAreaHigh;
    /** Exchange timestamp, or null when unavailable. */ public final Long tsExchangeNs;

    /** Creates a complete analytics observation. */
    public SignalValidationEvent(
            long delta, long cumulativeDelta, long buyVolume, long sellVolume,
            long lastPrice, long pointOfControl, long valueAreaLow, long valueAreaHigh,
            Long tsExchangeNs) {
        this.delta = delta;
        this.cumulativeDelta = cumulativeDelta;
        this.buyVolume = buyVolume;
        this.sellVolume = sellVolume;
        this.lastPrice = lastPrice;
        this.pointOfControl = pointOfControl;
        this.valueAreaLow = valueAreaLow;
        this.valueAreaHigh = valueAreaHigh;
        this.tsExchangeNs = tsExchangeNs;
    }
}
