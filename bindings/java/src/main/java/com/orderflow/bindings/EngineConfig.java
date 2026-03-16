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

    /**
     * Creates an immutable engine configuration.
     *
     * @param instanceId runtime instance identifier
     * @param configPath optional engine TOML config path
     * @param logLevel reserved log level field
     * @param enablePersistence enables local persistence when true
     * @param auditMaxBytes max bytes per audit file before rotation
     * @param auditMaxFiles max number of rotated audit files retained
     * @param auditRedactTokensCsv comma-separated sensitive token list
     * @param dataRetentionMaxBytes max persisted bytes retained
     * @param dataRetentionMaxAgeSecs max retained data age in seconds
     */
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

    /**
     * Returns sane default configuration values for local development.
     *
     * @return default immutable config
     */
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
