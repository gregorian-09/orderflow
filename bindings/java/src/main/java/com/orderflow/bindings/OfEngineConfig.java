package com.orderflow.bindings;

import com.sun.jna.Structure;

@Structure.FieldOrder({
    "instance_id",
    "config_path",
    "log_level",
    "enable_persistence",
    "audit_max_bytes",
    "audit_max_files",
    "audit_redact_tokens_csv",
    "data_retention_max_bytes",
    "data_retention_max_age_secs"
})
/** JNA mirror of native `of_engine_config_t`. */
public class OfEngineConfig extends Structure {
    /** Instance id pointer content. */
    public String instance_id;
    /** Config path pointer content. */
    public String config_path;
    /** Log level value. */
    public int log_level;
    /** Persistence enable flag. */
    public byte enable_persistence;
    /** Audit max bytes. */
    public long audit_max_bytes;
    /** Audit max files. */
    public int audit_max_files;
    /** Audit redact token csv. */
    public String audit_redact_tokens_csv;
    /** Persistence max bytes. */
    public long data_retention_max_bytes;
    /** Persistence max age seconds. */
    public long data_retention_max_age_secs;
}
