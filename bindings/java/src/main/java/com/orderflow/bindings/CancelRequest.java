package com.orderflow.bindings;

/** Execution cancel request. */
public final class CancelRequest {
    /** Cancel request client id. */
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
    /** Local timestamp in nanoseconds. */
    public final long tsRecvNs;

    /** Creates a cancel request. */
    public CancelRequest(String clientOrderId, String origClientOrderId, String venueOrderId, String accountId, String routeId, String venue, String instrument, long tsRecvNs) {
        this.clientOrderId = clientOrderId;
        this.origClientOrderId = origClientOrderId;
        this.venueOrderId = venueOrderId;
        this.accountId = accountId;
        this.routeId = routeId;
        this.venue = venue;
        this.instrument = instrument;
        this.tsRecvNs = tsRecvNs;
    }
}

