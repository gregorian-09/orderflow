package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_checkpoint_store_integrity_report_t`. */
@Structure.FieldOrder({
    "checkpoint_files",
    "valid_checkpoints",
    "invalid_checkpoints",
    "bytes",
    "latest_checkpoint_id",
    "latest_last_applied_sequence",
    "latest_created_ns",
    "has_latest",
    "valid"
})
public class OfExecutionCheckpointStoreIntegrityReport extends Structure {
    /** Checkpoint file count. */
    public long checkpoint_files;
    /** Valid checkpoint count. */
    public long valid_checkpoints;
    /** Invalid checkpoint count. */
    public long invalid_checkpoints;
    /** Total bytes across checkpoint files. */
    public long bytes;
    /** Latest valid checkpoint id when has_latest is non-zero. */
    public long latest_checkpoint_id;
    /** Last WAL sequence covered by latest valid checkpoint. */
    public long latest_last_applied_sequence;
    /** Creation timestamp for latest valid checkpoint. */
    public long latest_created_ns;
    /** Non-zero when latest checkpoint fields are meaningful. */
    public byte has_latest;
    /** Non-zero when all discovered checkpoint files decoded cleanly. */
    public byte valid;
}
