package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_metrics_t`. */
@Structure.FieldOrder({"submitted", "cancelled", "amended", "events_applied", "risk_rejected", "adapter_errors", "recovered"})
public class OfExecutionMetrics extends Structure {
    /** Submitted count. */
    public long submitted;
    /** Cancelled command count. */
    public long cancelled;
    /** Amended command count. */
    public long amended;
    /** Events applied count. */
    public long events_applied;
    /** Risk rejected count. */
    public long risk_rejected;
    /** Adapter error count. */
    public long adapter_errors;
    /** Recovered count. */
    public long recovered;
}
