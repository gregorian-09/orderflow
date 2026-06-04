package com.orderflow.bindings;

/** Current execution order state. */
public final class ExecutionOrderState {
    /** Client order id. */
    public final String clientOrderId;
    /** Venue order id. */
    public final String venueOrderId;
    /** Account id. */
    public final String accountId;
    /** Route id. */
    public final String routeId;
    /** Venue id. */
    public final String venue;
    /** Instrument id. */
    public final String instrument;
    /** Order status. */
    public final int status;
    /** Order quantity. */
    public final long orderQty;
    /** Cumulative quantity. */
    public final long cumulativeQty;
    /** Leaves quantity. */
    public final long leavesQty;
    /** Average price. */
    public final long averagePrice;
    /** Updated timestamp ns. */
    public final long updatedNs;

    /** Creates an order state. */
    public ExecutionOrderState(String clientOrderId, String venueOrderId, String accountId, String routeId, String venue, String instrument, int status, long orderQty, long cumulativeQty, long leavesQty, long averagePrice, long updatedNs) {
        this.clientOrderId = clientOrderId;
        this.venueOrderId = venueOrderId;
        this.accountId = accountId;
        this.routeId = routeId;
        this.venue = venue;
        this.instrument = instrument;
        this.status = status;
        this.orderQty = orderQty;
        this.cumulativeQty = cumulativeQty;
        this.leavesQty = leavesQty;
        this.averagePrice = averagePrice;
        this.updatedNs = updatedNs;
    }
}

