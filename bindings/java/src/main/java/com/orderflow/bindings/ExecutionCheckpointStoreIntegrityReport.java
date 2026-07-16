package com.orderflow.bindings;

/** Execution checkpoint store integrity report for offline diagnostics. */
public final class ExecutionCheckpointStoreIntegrityReport {
    /** Checkpoint file count. */
    public final long checkpointFiles;
    /** Valid checkpoint count. */
    public final long validCheckpoints;
    /** Invalid checkpoint count. */
    public final long invalidCheckpoints;
    /** Total bytes across checkpoint files. */
    public final long bytes;
    /** Latest valid checkpoint id, or null when no valid checkpoint exists. */
    public final Long latestCheckpointId;
    /** Last WAL sequence covered by latest valid checkpoint. */
    public final Long latestLastAppliedSequence;
    /** Creation timestamp for latest valid checkpoint. */
    public final Long latestCreatedNs;
    /** True when all discovered checkpoint files decoded cleanly. */
    public final boolean valid;

    /** Creates an execution checkpoint store integrity report. */
    public ExecutionCheckpointStoreIntegrityReport(
        long checkpointFiles,
        long validCheckpoints,
        long invalidCheckpoints,
        long bytes,
        Long latestCheckpointId,
        Long latestLastAppliedSequence,
        Long latestCreatedNs,
        boolean valid
    ) {
        this.checkpointFiles = checkpointFiles;
        this.validCheckpoints = validCheckpoints;
        this.invalidCheckpoints = invalidCheckpoints;
        this.bytes = bytes;
        this.latestCheckpointId = latestCheckpointId;
        this.latestLastAppliedSequence = latestLastAppliedSequence;
        this.latestCreatedNs = latestCreatedNs;
        this.valid = valid;
    }
}
