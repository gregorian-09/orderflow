package com.orderflow.bindings;

/** Execution metrics snapshot. */
public final class ExecutionMetrics {
    /** Submitted count. */
    public final long submitted;
    /** Cancel command count. */
    public final long cancelled;
    /** Amend command count. */
    public final long amended;
    /** Events applied count. */
    public final long eventsApplied;
    /** Risk rejection count. */
    public final long riskRejected;
    /** Adapter error count. */
    public final long adapterErrors;
    /** Recovery event count. */
    public final long recovered;

    /** Creates execution metrics. */
    public ExecutionMetrics(long submitted, long cancelled, long amended, long eventsApplied, long riskRejected, long adapterErrors, long recovered) {
        this.submitted = submitted;
        this.cancelled = cancelled;
        this.amended = amended;
        this.eventsApplied = eventsApplied;
        this.riskRejected = riskRejected;
        this.adapterErrors = adapterErrors;
        this.recovered = recovered;
    }
}

