package com.orderflow.bindings;

/** Execution pre-trade risk limits. */
public final class RiskLimits {
    /** Kill switch flag. */
    public final boolean killSwitch;
    /** Maximum order quantity; zero disables. */
    public final long maxOrderQty;
    /** Maximum order notional; zero disables. */
    public final long maxOrderNotional;
    /** Maximum open orders; zero disables. */
    public final int maxOpenOrders;
    /** Maximum open notional; zero disables. */
    public final long maxOpenNotional;
    /** Price band in ticks; zero disables. */
    public final long priceBandTicks;

    /** Creates risk limits. */
    public RiskLimits(boolean killSwitch, long maxOrderQty, long maxOrderNotional, int maxOpenOrders, long maxOpenNotional, long priceBandTicks) {
        this.killSwitch = killSwitch;
        this.maxOrderQty = maxOrderQty;
        this.maxOrderNotional = maxOrderNotional;
        this.maxOpenOrders = maxOpenOrders;
        this.maxOpenNotional = maxOpenNotional;
        this.priceBandTicks = priceBandTicks;
    }

    /** Returns deny-by-default limits. */
    public static RiskLimits defaults() {
        return new RiskLimits(true, 0, 0, 0, 0, 0);
    }
}

