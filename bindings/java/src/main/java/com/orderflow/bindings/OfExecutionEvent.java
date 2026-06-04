package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_event_t`. */
@Structure.FieldOrder({"exec_type", "order_status", "client_order_id", "orig_client_order_id", "venue_order_id", "execution_id", "account_id", "route_id", "venue", "instrument", "last_qty", "last_price", "cumulative_qty", "leaves_qty", "average_price", "ts_exchange_ns", "ts_recv_ns", "reason", "text"})
public class OfExecutionEvent extends Structure {
    /** Execution type. */
    public int exec_type;
    /** Order status. */
    public int order_status;
    /** Client order id. */
    public byte[] client_order_id = new byte[41];
    /** Original client order id. */
    public byte[] orig_client_order_id = new byte[41];
    /** Venue order id. */
    public byte[] venue_order_id = new byte[49];
    /** Execution id. */
    public byte[] execution_id = new byte[49];
    /** Account id. */
    public byte[] account_id = new byte[33];
    /** Route id. */
    public byte[] route_id = new byte[33];
    /** Venue id. */
    public byte[] venue = new byte[17];
    /** Instrument id. */
    public byte[] instrument = new byte[33];
    /** Last quantity. */
    public long last_qty;
    /** Last price. */
    public long last_price;
    /** Cumulative quantity. */
    public long cumulative_qty;
    /** Leaves quantity. */
    public long leaves_qty;
    /** Average price. */
    public long average_price;
    /** Exchange timestamp ns. */
    public long ts_exchange_ns;
    /** Receive timestamp ns. */
    public long ts_recv_ns;
    /** Reason code. */
    public int reason;
    /** Diagnostic text. */
    public byte[] text = new byte[129];
}

