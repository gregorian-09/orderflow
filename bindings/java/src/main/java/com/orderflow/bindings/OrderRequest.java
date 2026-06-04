package com.orderflow.bindings;

/** Execution new-order request. */
public final class OrderRequest {
    /** Client order id. */
    public final String clientOrderId;
    /** Account id. */
    public final String accountId;
    /** Route id. */
    public final String routeId;
    /** Strategy id. */
    public final String strategyId;
    /** Venue id. */
    public final String venue;
    /** Instrument id. */
    public final String instrument;
    /** Side constant. */
    public final int side;
    /** Order type constant. */
    public final int orderType;
    /** Time-in-force constant. */
    public final int timeInForce;
    /** Quantity. */
    public final long quantity;
    /** Limit price, or zero. */
    public final long limitPrice;
    /** Stop price, or zero. */
    public final long stopPrice;
    /** Exchange timestamp in nanoseconds. */
    public final long tsExchangeNs;
    /** Local timestamp in nanoseconds. */
    public final long tsRecvNs;

    /** Creates a new-order request. */
    public OrderRequest(String clientOrderId, String accountId, String routeId, String strategyId, String venue, String instrument, int side, int orderType, int timeInForce, long quantity, long limitPrice, long stopPrice, long tsExchangeNs, long tsRecvNs) {
        this.clientOrderId = clientOrderId;
        this.accountId = accountId;
        this.routeId = routeId;
        this.strategyId = strategyId;
        this.venue = venue;
        this.instrument = instrument;
        this.side = side;
        this.orderType = orderType;
        this.timeInForce = timeInForce;
        this.quantity = quantity;
        this.limitPrice = limitPrice;
        this.stopPrice = stopPrice;
        this.tsExchangeNs = tsExchangeNs;
        this.tsRecvNs = tsRecvNs;
    }
}

