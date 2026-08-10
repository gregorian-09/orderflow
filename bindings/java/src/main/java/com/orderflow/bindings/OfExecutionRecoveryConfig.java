package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native {@code of_execution_recovery_config_t}. */
@Structure.FieldOrder({"wal_root", "checkpoint_root", "require_checkpoint"})
public class OfExecutionRecoveryConfig extends Structure {
    /** Existing segmented execution WAL root. */
    public String wal_root;
    /** Existing checkpoint root, or null for checkpoint-free replay. */
    public String checkpoint_root;
    /** Non-zero requires a valid checkpoint before replay. */
    public byte require_checkpoint;
}
