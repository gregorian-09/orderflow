package com.orderflow.bindings;

/** Runtime action identifiers for market-data persistence failures. */
public final class MarketDataPersistenceFailureAction {
    /** Continue processing while reporting degraded health. */
    public static final int MARK_DEGRADED = 0;
    /** Reject subsequent market-data processing. */
    public static final int STOP_MARKET_DATA = 1;
    /** Continue analytics but block trading readiness. */
    public static final int STOP_TRADING = 2;
    /** Surface failure and block subsequent processing. */
    public static final int FAIL_PROCESS = 3;
    /** Continue in memory after persistence failure. */
    public static final int MEMORY_ONLY = 4;

    private MarketDataPersistenceFailureAction() {}
}
