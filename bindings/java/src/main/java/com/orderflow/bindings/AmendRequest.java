package com.orderflow.bindings;

/** Execution amend/cancel-replace request. */
public final class AmendRequest {
    /** Replacement request client id. */
    public final String clientOrderId;
    /** Last accepted original client id. */
    public final String origClientOrderId;
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
    /** Replacement quantity. */
    public final long quantity;
    /** Replacement limit price. */
    public final long limitPrice;
    /** Local timestamp in nanoseconds. */
    public final long tsRecvNs;

    /** Creates an amend request. */
    public AmendRequest(String clientOrderId, String origClientOrderId, String venueOrderId, String accountId, String routeId, String venue, String instrument, long quantity, long limitPrice, long tsRecvNs) {
        this.clientOrderId = clientOrderId;
        this.origClientOrderId = origClientOrderId;
        this.venueOrderId = venueOrderId;
        this.accountId = accountId;
        this.routeId = routeId;
        this.venue = venue;
        this.instrument = instrument;
        this.quantity = quantity;
        this.limitPrice = limitPrice;
        this.tsRecvNs = tsRecvNs;
    }
}
