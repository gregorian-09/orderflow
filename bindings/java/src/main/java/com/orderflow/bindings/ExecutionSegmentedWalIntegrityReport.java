package com.orderflow.bindings;

/** Segmented execution WAL integrity report for offline diagnostics. */
public final class ExecutionSegmentedWalIntegrityReport {
    /** Segment file count. */
    public final long segments;
    /** Valid WAL frame count. */
    public final long records;
    /** Valid WAL bytes consumed. */
    public final long bytes;
    /** First WAL sequence, or null for an empty WAL directory. */
    public final Long firstSequence;
    /** Last WAL sequence, or null for an empty WAL directory. */
    public final Long lastSequence;
    /** Checksum failure count. */
    public final long checksumFailures;
    /** Strict sequence failure count. */
    public final long sequenceFailures;
    /** True when all inspected segments decoded cleanly. */
    public final boolean valid;

    /** Creates a segmented execution WAL integrity report. */
    public ExecutionSegmentedWalIntegrityReport(
        long segments,
        long records,
        long bytes,
        Long firstSequence,
        Long lastSequence,
        long checksumFailures,
        long sequenceFailures,
        boolean valid
    ) {
        this.segments = segments;
        this.records = records;
        this.bytes = bytes;
        this.firstSequence = firstSequence;
        this.lastSequence = lastSequence;
        this.checksumFailures = checksumFailures;
        this.sequenceFailures = sequenceFailures;
        this.valid = valid;
    }
}
