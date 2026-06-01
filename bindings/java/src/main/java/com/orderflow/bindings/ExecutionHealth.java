package com.orderflow.bindings;

/** Execution health snapshot. */
public final class ExecutionHealth {
    /** Connected flag. */
    public final boolean connected;
    /** Degraded flag. */
    public final boolean degraded;
    /** Health sequence. */
    public final long healthSeq;

    /** Creates execution health. */
    public ExecutionHealth(boolean connected, boolean degraded, long healthSeq) {
        this.connected = connected;
        this.degraded = degraded;
        this.healthSeq = healthSeq;
    }
}

