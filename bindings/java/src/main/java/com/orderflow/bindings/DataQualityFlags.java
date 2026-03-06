package com.orderflow.bindings;

/** Data-quality bit flags returned by runtime callbacks and snapshots. */
public final class DataQualityFlags {
    private DataQualityFlags() {}

    /** No quality issues. */
    public static final int NONE = 0;
    /** Feed is stale beyond configured threshold. */
    public static final int STALE_FEED = 1 << 0;
    /** Sequence gap detected. */
    public static final int SEQUENCE_GAP = 1 << 1;
    /** Clock skew detected. */
    public static final int CLOCK_SKEW = 1 << 2;
    /** Book depth was truncated. */
    public static final int DEPTH_TRUNCATED = 1 << 3;
    /** Out-of-order sequence detected. */
    public static final int OUT_OF_ORDER = 1 << 4;
    /** Adapter/external feed is degraded. */
    public static final int ADAPTER_DEGRADED = 1 << 5;
}
