package com.orderflow.bindings;

import com.sun.jna.Structure;

@Structure.FieldOrder({"stale_after_ms", "enforce_sequence"})
/** JNA mirror of native `of_external_feed_policy_t`. */
public class OfExternalFeedPolicy extends Structure {
    /** Stale threshold in milliseconds. */
    public long stale_after_ms;
    /** Sequence enforcement flag. */
    public byte enforce_sequence;
}
