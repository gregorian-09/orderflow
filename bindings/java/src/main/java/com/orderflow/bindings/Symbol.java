package com.orderflow.bindings;

/** Symbol descriptor used by subscriptions, snapshots, and ingest APIs. */
public final class Symbol {
    /** Venue/exchange identifier. */
    public final String venue;
    /** Venue-native symbol value. */
    public final String symbol;
    /** Requested depth level count for book subscriptions. */
    public final int depthLevels;

    /**
     * Creates an immutable symbol descriptor.
     *
     * @param venue venue/exchange identifier (for example: CME, BINANCE)
     * @param symbol venue-native instrument symbol
     * @param depthLevels requested depth levels for book processing
     */
    public Symbol(String venue, String symbol, int depthLevels) {
        this.venue = venue;
        this.symbol = symbol;
        this.depthLevels = depthLevels;
    }
}
