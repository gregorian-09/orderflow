package com.orderflow.bindings;

/** Execution WAL integrity report for offline operator diagnostics. */
public final class ExecutionWalIntegrityReport {
    /** Valid WAL frame count. */
    public final long records;
    /** Valid WAL bytes consumed. */
    public final long bytes;
    /** First WAL sequence, or null for an empty WAL. */
    public final Long firstSequence;
    /** Last WAL sequence, or null for an empty WAL. */
    public final Long lastSequence;
    /** Checksum failure count. */
    public final long checksumFailures;
    /** Strict sequence failure count. */
    public final long sequenceFailures;
    /** True when the WAL ends with a partial frame. */
    public final boolean truncatedTail;
    /** True when all bytes decoded cleanly. */
    public final boolean valid;

    /** Creates an execution WAL integrity report. */
    public ExecutionWalIntegrityReport(
        long records,
        long bytes,
        Long firstSequence,
        Long lastSequence,
        long checksumFailures,
        long sequenceFailures,
        boolean truncatedTail,
        boolean valid
    ) {
        this.records = records;
        this.bytes = bytes;
        this.firstSequence = firstSequence;
        this.lastSequence = lastSequence;
        this.checksumFailures = checksumFailures;
        this.sequenceFailures = sequenceFailures;
        this.truncatedTail = truncatedTail;
        this.valid = valid;
    }
}
