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
}
