package com.orderflow.bindings;

/** Engine configuration passed to the native runtime. */
public final class EngineConfig {
    /** Runtime instance identifier. */
    public final String instanceId;
    /** Optional config file path consumed by Rust runtime. */
    public final String configPath;
    /** Reserved log level field. */
    public final int logLevel;
    /** Enables persistence when true. */
    public final boolean enablePersistence;
    /** Max audit bytes before rotation. */
    public final long auditMaxBytes;
    /** Max number of rotated audit files. */
    public final int auditMaxFiles;
    /** Comma-separated redaction tokens. */
    public final String auditRedactTokensCsv;
    /** Max persisted bytes to retain. */
    public final long dataRetentionMaxBytes;
    /** Max persisted age in seconds. */
    public final long dataRetentionMaxAgeSecs;

    /** Creates an immutable engine configuration. */
    public EngineConfig(
            String instanceId,
            String configPath,
            int logLevel,
            boolean enablePersistence,
            long auditMaxBytes,
            int auditMaxFiles,
            String auditRedactTokensCsv,
            long dataRetentionMaxBytes,
            long dataRetentionMaxAgeSecs) {
        this.instanceId = instanceId;
        this.configPath = configPath;
        this.logLevel = logLevel;
        this.enablePersistence = enablePersistence;
        this.auditMaxBytes = auditMaxBytes;
        this.auditMaxFiles = auditMaxFiles;
        this.auditRedactTokensCsv = auditRedactTokensCsv;
        this.dataRetentionMaxBytes = dataRetentionMaxBytes;
        this.dataRetentionMaxAgeSecs = dataRetentionMaxAgeSecs;
    }

    /** Returns sane default configuration values. */
    public static EngineConfig defaults() {
        return new EngineConfig(
                "java",
                "",
                0,
                false,
                10L * 1024L * 1024L,
                5,
                "secret,password,token,api_key",
                10L * 1024L * 1024L,
                7L * 24L * 60L * 60L);
    }
}
