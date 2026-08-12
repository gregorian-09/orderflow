package com.orderflow.bindings;

/** Immutable engine-owned bounded segmented market-data WAL configuration. */
public final class MarketDataWalConfig {
    /** Required WAL directory. */
    public final String rootPath;
    /** Soft segment byte target, or zero for native default. */
    public final long maxSegmentBytes;
    /** Maximum encoded payload bytes, or zero for native default. */
    public final long maxPayloadBytes;
    /** Sync-policy identifier from {@link MarketDataWalSyncPolicy}. */
    public final int syncPolicy;
    /** Sync cadence when policy is {@code EVERY_RECORDS}. */
    public final long syncEveryRecords;
    /** Whether manifest snapshots are synchronized before rename. */
    public final boolean syncManifest;
    /** Queue record capacity, or zero for native default. */
    public final int queueCapacity;
    /** Aggregate queued payload byte bound, or zero for native default. */
    public final long maxQueuedPayloadBytes;
    /** Failure action from {@link MarketDataPersistenceFailureAction}. */
    public final int failureAction;
    /** Optional native writer thread name. */
    public final String writerThreadName;

    /** Creates a complete immutable WAL configuration. */
    public MarketDataWalConfig(
            String rootPath,
            long maxSegmentBytes,
            long maxPayloadBytes,
            int syncPolicy,
            long syncEveryRecords,
            boolean syncManifest,
            int queueCapacity,
            long maxQueuedPayloadBytes,
            int failureAction,
            String writerThreadName) {
        this.rootPath = rootPath;
        this.maxSegmentBytes = maxSegmentBytes;
        this.maxPayloadBytes = maxPayloadBytes;
        this.syncPolicy = syncPolicy;
        this.syncEveryRecords = syncEveryRecords;
        this.syncManifest = syncManifest;
        this.queueCapacity = queueCapacity;
        this.maxQueuedPayloadBytes = maxQueuedPayloadBytes;
        this.failureAction = failureAction;
        this.writerThreadName = writerThreadName;
    }

    /** Returns conservative production defaults rooted at {@code rootPath}. */
    public static MarketDataWalConfig defaults(String rootPath) {
        return new MarketDataWalConfig(
                rootPath,
                0,
                0,
                MarketDataWalSyncPolicy.ON_SEGMENT_SEAL,
                0,
                true,
                0,
                0,
                MarketDataPersistenceFailureAction.STOP_TRADING,
                "");
    }
}
