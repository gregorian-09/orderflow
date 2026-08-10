package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_algo_child_plan_t`. */
@Structure.FieldOrder({
    "child_order_id", "parent_order_id", "client_order_id", "account_id", "route_id",
    "strategy_id", "venue", "instrument", "side", "order_type", "time_in_force",
    "quantity", "limit_price", "stop_price", "due_ns", "ts_recv_ns", "has_plan"
})
/** Native owned child-plan structure. */
public class OfExecutionAlgoChildPlan extends Structure {
    /** Child algorithm identifier. */
    public byte[] child_order_id = new byte[41];
    /** Parent algorithm identifier. */
    public byte[] parent_order_id = new byte[41];
    /** Canonical client order identifier. */
    public byte[] client_order_id = new byte[41];
    /** Trading account identifier. */
    public byte[] account_id = new byte[33];
    /** Route identifier. */
    public byte[] route_id = new byte[33];
    /** Strategy identifier. */
    public byte[] strategy_id = new byte[33];
    /** Venue identifier. */
    public byte[] venue = new byte[17];
    /** Instrument identifier. */
    public byte[] instrument = new byte[33];
    /** Canonical side. */
    public int side;
    /** Canonical order type. */
    public int order_type;
    /** Canonical time in force. */
    public int time_in_force;
    /** Planned quantity. */
    public long quantity;
    /** Planned limit price. */
    public long limit_price;
    /** Planned stop price. */
    public long stop_price;
    /** Planned release timestamp. */
    public long due_ns;
    /** OMS receive/create timestamp. */
    public long ts_recv_ns;
    /** Non-zero when a plan is present. */
    public byte has_plan;
}
