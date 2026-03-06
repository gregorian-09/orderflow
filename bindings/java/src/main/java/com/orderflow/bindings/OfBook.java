package com.orderflow.bindings;

import com.sun.jna.Structure;

@Structure.FieldOrder({
    "symbol",
    "side",
    "level",
    "price",
    "size",
    "action",
    "sequence",
    "ts_exchange_ns",
    "ts_recv_ns"
})
/** JNA mirror of native `of_book_t`. */
public class OfBook extends Structure {
    /** Symbol descriptor. */
    public OfSymbol symbol;
    /** Side id. */
    public int side;
    /** Book level index. */
    public short level;
    /** Level price. */
    public long price;
    /** Level size. */
    public long size;
    /** Action id. */
    public int action;
    /** Sequence number. */
    public long sequence;
    /** Exchange timestamp ns. */
    public long ts_exchange_ns;
    /** Receive timestamp ns. */
    public long ts_recv_ns;
}
