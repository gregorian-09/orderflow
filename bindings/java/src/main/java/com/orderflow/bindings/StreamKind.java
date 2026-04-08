package com.orderflow.bindings;

/** Stream kind identifiers used for subscription requests. */
public final class StreamKind {
    private StreamKind() {}

    /** Order book updates. */
    public static final int BOOK = 1;
    /** Trade prints. */
    public static final int TRADES = 2;
    /** Analytics snapshots. */
    public static final int ANALYTICS = 3;
    /** Signal snapshots. */
    public static final int SIGNALS = 4;
    /** Health transitions. */
    public static final int HEALTH = 5;
    /** Materialized order-book snapshots emitted after book changes. */
    public static final int BOOK_SNAPSHOT = 6;
    /** Derived analytics snapshots emitted after trade-driven analytics changes. */
    public static final int DERIVED_ANALYTICS = 7;
}
