package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_cancel_request_t`. */
@Structure.FieldOrder({"client_order_id", "orig_client_order_id", "venue_order_id", "account_id", "route_id", "venue", "instrument", "ts_recv_ns"})
public class OfExecutionCancelRequest extends Structure {
    /** Cancel client id. */
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
    /** Receive timestamp ns. */
    public long ts_recv_ns;
}

