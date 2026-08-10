package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native {@code of_signal_validation_config_t}. */
@Structure.FieldOrder({
    "markout_horizon_events", "flat_price_threshold", "min_confidence_bps",
    "store_samples", "check_monotonic_timestamps"
})
public class OfSignalValidationConfig extends Structure {
    /** Future event horizon. */ public int markout_horizon_events;
    /** Flat markout threshold. */ public long flat_price_threshold;
    /** Minimum confidence in basis points. */ public short min_confidence_bps;
    /** Sample-retention flag. */ public byte store_samples;
    /** Timestamp-order check flag. */ public byte check_monotonic_timestamps;
}
