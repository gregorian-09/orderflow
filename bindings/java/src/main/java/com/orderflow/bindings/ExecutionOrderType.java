package com.orderflow.bindings;

/** Execution order type constants. */
public final class ExecutionOrderType {
    /** Market order. */
    public static final int MARKET = 1;
    /** Limit order. */
    public static final int LIMIT = 2;
    /** Stop order. */
    public static final int STOP = 3;
    /** Stop-limit order. */
    public static final int STOP_LIMIT = 4;

    private ExecutionOrderType() {}
}

