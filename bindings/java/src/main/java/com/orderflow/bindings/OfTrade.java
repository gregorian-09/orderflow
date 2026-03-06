package com.orderflow.bindings;

import com.sun.jna.Structure;

@Structure.FieldOrder({
    "symbol",
    "price",
    "size",
    "aggressor_side",
    "sequence",
    "ts_exchange_ns",
    "ts_recv_ns"
})
/** JNA mirror of native `of_trade_t`. */
public class OfTrade extends Structure {
    /** Symbol descriptor. */
    public OfSymbol symbol;
    /** Trade price. */
    public long price;
    /** Trade size. */
    public long size;
    /** Aggressor side id. */
    public int aggressor_side;
    /** Sequence number. */
    public long sequence;
    /** Exchange timestamp ns. */
    public long ts_exchange_ns;
    /** Receive timestamp ns. */
    public long ts_recv_ns;
}
