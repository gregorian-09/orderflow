package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native {@code of_signal_validation_event_t}. */
@Structure.FieldOrder({
    "delta", "cumulative_delta", "buy_volume", "sell_volume", "last_price",
    "point_of_control", "value_area_low", "value_area_high", "ts_exchange_ns",
    "has_ts_exchange_ns"
})
public class OfSignalValidationEvent extends Structure {
    /** Session delta. */ public long delta;
    /** Cumulative session delta. */ public long cumulative_delta;
    /** Buy-side volume. */ public long buy_volume;
    /** Sell-side volume. */ public long sell_volume;
    /** Last traded price. */ public long last_price;
    /** Session point of control. */ public long point_of_control;
    /** Session value-area low. */ public long value_area_low;
    /** Session value-area high. */ public long value_area_high;
    /** Exchange timestamp. */ public long ts_exchange_ns;
    /** Timestamp-presence flag. */ public byte has_ts_exchange_ns;
}
