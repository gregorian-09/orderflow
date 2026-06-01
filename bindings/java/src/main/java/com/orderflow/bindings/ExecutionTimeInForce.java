package com.orderflow.bindings;

/** Execution time-in-force constants. */
public final class ExecutionTimeInForce {
    /** Day order. */
    public static final int DAY = 1;
    /** Good-till-cancelled order. */
    public static final int GTC = 2;
    /** Immediate-or-cancel order. */
    public static final int IOC = 3;
    /** Fill-or-kill order. */
    public static final int FOK = 4;
    /** Good-till-date order. */
    public static final int GTD = 5;

    private ExecutionTimeInForce() {}
}

