package com.orderflow.bindings;

import com.sun.jna.Structure;

@Structure.FieldOrder({"venue", "symbol", "depth_levels"})
/** JNA mirror of native `of_symbol_t`. */
public class OfSymbol extends Structure {
    /** Venue string. */
    public String venue;
    /** Symbol string. */
    public String symbol;
    /** Depth levels. */
    public short depth_levels;
}
