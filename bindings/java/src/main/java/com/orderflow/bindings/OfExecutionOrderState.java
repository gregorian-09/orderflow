package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_order_state_t`. */
@Structure.FieldOrder({"client_order_id", "venue_order_id", "account_id", "route_id", "venue", "instrument", "status", "order_qty", "cumulative_qty", "leaves_qty", "average_price", "updated_ns"})
public class OfExecutionOrderState extends Structure {
    /** Client order id. */
    public byte[] client_order_id = new byte[41];
    /** Venue order id. */
    public byte[] venue_order_id = new byte[49];
    /** Account id. */
    public byte[] account_id = new byte[33];
    /** Route id. */
    public byte[] route_id = new byte[33];
    /** Venue id. */
    public byte[] venue = new byte[17];
    /** Instrument id. */
    public byte[] instrument = new byte[33];
    /** Order status. */
    public int status;
    /** Order quantity. */
    public long order_qty;
    /** Cumulative quantity. */
    public long cumulative_qty;
    /** Leaves quantity. */
    public long leaves_qty;
    /** Average price. */
    public long average_price;
    /** Updated timestamp ns. */
    public long updated_ns;
}

