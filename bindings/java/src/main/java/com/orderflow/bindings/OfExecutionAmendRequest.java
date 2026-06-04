package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_amend_request_t`. */
@Structure.FieldOrder({"client_order_id", "orig_client_order_id", "venue_order_id", "account_id", "route_id", "venue", "instrument", "quantity", "limit_price", "ts_recv_ns"})
public class OfExecutionAmendRequest extends Structure {
    /** Replacement client id. */
    public String client_order_id;
    /** Original client id. */
    public String orig_client_order_id;
    /** Venue order id. */
    public String venue_order_id;
    /** Account id. */
    public String account_id;
    /** Route id. */
    public String route_id;
    /** Venue id. */
    public String venue;
    /** Instrument id. */
    public String instrument;
    /** Replacement quantity. */
    public long quantity;
    /** Replacement limit price. */
    public long limit_price;
    /** Receive timestamp ns. */
    public long ts_recv_ns;
}

