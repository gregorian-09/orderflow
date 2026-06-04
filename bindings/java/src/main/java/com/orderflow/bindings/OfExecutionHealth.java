package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_health_t`. */
@Structure.FieldOrder({"connected", "degraded", "health_seq"})
public class OfExecutionHealth extends Structure {
    /** Connected flag. */
    public byte connected;
    /** Degraded flag. */
    public byte degraded;
    /** Health sequence. */
    public long health_seq;
}

