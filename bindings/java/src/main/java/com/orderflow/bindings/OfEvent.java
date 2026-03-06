package com.orderflow.bindings;

import com.sun.jna.Pointer;
import com.sun.jna.Structure;

@Structure.FieldOrder({"ts_exchange_ns", "ts_recv_ns", "kind", "payload", "payload_len", "schema_id", "quality_flags"})
/** JNA mirror of native `of_event_t`. */
public class OfEvent extends Structure {
    /** Exchange timestamp ns. */
    public long ts_exchange_ns;
    /** Receive timestamp ns. */
    public long ts_recv_ns;
    /** Stream kind id. */
    public int kind;
    /** Payload pointer. */
    public Pointer payload;
    /** Payload byte length. */
    public int payload_len;
    /** Schema id. */
    public int schema_id;
    /** Quality flag bits. */
    public int quality_flags;

    /** Default constructor for JNA allocation. */
    public OfEvent() {}

    /** Constructor that reads fields from existing native pointer. */
    public OfEvent(Pointer p) {
        super(p);
        read();
    }
}
