package com.orderflow.bindings;

/** Segmented market-data WAL synchronization policy identifiers. */
public final class MarketDataWalSyncPolicy {
    /** Synchronize when a segment is sealed. */
    public static final int ON_SEGMENT_SEAL = 0;
    /** Rely on page cache until an explicit barrier. */
    public static final int NEVER = 1;
    /** Synchronize every record. */
    public static final int EVERY_RECORD = 2;
    /** Synchronize after the configured record cadence. */
    public static final int EVERY_RECORDS = 3;

    private MarketDataWalSyncPolicy() {}
}
