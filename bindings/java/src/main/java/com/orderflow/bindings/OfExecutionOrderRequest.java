package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_order_request_t`. */
@Structure.FieldOrder({"client_order_id", "account_id", "route_id", "strategy_id", "venue", "instrument", "side", "order_type", "time_in_force", "quantity", "limit_price", "stop_price", "ts_exchange_ns", "ts_recv_ns"})
public class OfExecutionOrderRequest extends Structure {
    /** Client order id. */
    public String client_order_id;
    /** Account id. */
    public String account_id;
    /** Route id. */
    public String route_id;
    /** Strategy id. */
    public String strategy_id;
    /** Venue id. */
    public String venue;
    /** Instrument id. */
    public String instrument;
    /** Side constant. */
    public int side;
    /** Order type constant. */
    public int order_type;
    /** Time-in-force constant. */
    public int time_in_force;
    /** Quantity. */
    public long quantity;
    /** Limit price. */
    public long limit_price;
    /** Stop price. */
    public long stop_price;
    /** Exchange timestamp ns. */
    public long ts_exchange_ns;
    /** Receive timestamp ns. */
    public long ts_recv_ns;
}

