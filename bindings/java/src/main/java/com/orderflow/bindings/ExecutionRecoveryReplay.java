package com.orderflow.bindings;

/** Validated WAL range consumed by a read-only recovery report. */
public final class ExecutionRecoveryReplay {
    /** Number of state-bearing command/event records replayed. */
    public final long records;
    /** Encoded WAL bytes scanned. */
    public final long bytes;
    /** First WAL sequence inspected, or null for an empty root. */
    public final Long firstSequence;
    /** Last WAL sequence inspected, including marker records. */
    public final Long lastSequence;

    /** Creates a typed recovery replay summary. */
    public ExecutionRecoveryReplay(long records, long bytes, Long firstSequence, Long lastSequence) {
        this.records = records;
        this.bytes = bytes;
        this.firstSequence = firstSequence;
        this.lastSequence = lastSequence;
    }
}
