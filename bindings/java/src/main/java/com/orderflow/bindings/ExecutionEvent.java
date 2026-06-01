package com.orderflow.bindings;

/** Execution event returned by the native execution engine. */
public final class ExecutionEvent {
    /** Execution type. */
    public final int execType;
    /** Order status. */
    public final int orderStatus;
    /** Client order id. */
    public final String clientOrderId;
    /** Original client order id. */
    public final String origClientOrderId;
    /** Venue order id. */
    public final String venueOrderId;
    /** Execution id. */
    public final String executionId;
    /** Account id. */
    public final String accountId;
    /** Route id. */
    public final String routeId;
    /** Venue id. */
    public final String venue;
    /** Instrument id. */
    public final String instrument;
    /** Last quantity. */
    public final long lastQty;
    /** Last price. */
    public final long lastPrice;
    /** Cumulative quantity. */
    public final long cumulativeQty;
    /** Leaves quantity. */
    public final long leavesQty;
    /** Average price. */
    public final long averagePrice;
    /** Exchange timestamp ns. */
    public final long tsExchangeNs;
    /** Receive timestamp ns. */
    public final long tsRecvNs;
    /** Reason code. */
    public final int reason;
    /** Diagnostic text. */
    public final String text;

    /** Creates an execution event. */
    public ExecutionEvent(int execType, int orderStatus, String clientOrderId, String origClientOrderId, String venueOrderId, String executionId, String accountId, String routeId, String venue, String instrument, long lastQty, long lastPrice, long cumulativeQty, long leavesQty, long averagePrice, long tsExchangeNs, long tsRecvNs, int reason, String text) {
        this.execType = execType;
        this.orderStatus = orderStatus;
        this.clientOrderId = clientOrderId;
        this.origClientOrderId = origClientOrderId;
        this.venueOrderId = venueOrderId;
        this.executionId = executionId;
        this.accountId = accountId;
        this.routeId = routeId;
        this.venue = venue;
        this.instrument = instrument;
        this.lastQty = lastQty;
        this.lastPrice = lastPrice;
        this.cumulativeQty = cumulativeQty;
        this.leavesQty = leavesQty;
        this.averagePrice = averagePrice;
        this.tsExchangeNs = tsExchangeNs;
        this.tsRecvNs = tsRecvNs;
        this.reason = reason;
        this.text = text;
    }
}

