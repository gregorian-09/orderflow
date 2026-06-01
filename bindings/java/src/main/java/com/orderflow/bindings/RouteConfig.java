package com.orderflow.bindings;

/** Execution route, account, symbol, and risk configuration. */
public final class RouteConfig {
    /** Route id. */
    public final String routeId;
    /** Account id. */
    public final String accountId;
    /** Venue id. */
    public final String venue;
    /** Instrument id. */
    public final String instrument;
    /** Enabled flag. */
    public final boolean enabled;
    /** Risk limits. */
    public final RiskLimits riskLimits;

    /** Creates a route config. */
    public RouteConfig(String routeId, String accountId, String venue, String instrument, boolean enabled, RiskLimits riskLimits) {
        this.routeId = routeId;
        this.accountId = accountId;
        this.venue = venue;
        this.instrument = instrument;
        this.enabled = enabled;
        this.riskLimits = riskLimits;
    }
}

