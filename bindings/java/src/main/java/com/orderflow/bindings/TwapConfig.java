package com.orderflow.bindings;

/** Parent-order and schedule inputs for deterministic native TWAP planning. */
public final class TwapConfig {
    /** Parent order identifier. */ public final String parentOrderId;
    /** Trading account identifier. */ public final String accountId;
    /** Default route identifier. */ public final String routeId;
    /** Strategy attribution identifier. */ public final String strategyId;
    /** Venue identifier. */ public final String venue;
    /** Instrument identifier. */ public final String instrument;
    /** Canonical side. */ public final int side;
    /** Canonical order type. */ public final int orderType;
    /** Canonical time in force. */ public final int timeInForce;
    /** Total parent quantity. */ public final long totalQty;
    /** Child limit price. */ public final long limitPrice;
    /** Child stop price. */ public final long stopPrice;
    /** Parent start timestamp. */ public final long startNs;
    /** Parent end timestamp. */ public final long endNs;
    /** Minimum child clip. */ public final long minClip;
    /** Maximum child clip. */ public final long maxClip;
    /** Optional participation cap in basis points. */ public final int participationCapBps;
    /** TWAP slice interval. */ public final long sliceIntervalNs;

    /** Creates a complete TWAP configuration. */
    public TwapConfig(
        String parentOrderId, String accountId, String routeId, String strategyId,
        String venue, String instrument, int side, int orderType, int timeInForce,
        long totalQty, long limitPrice, long stopPrice, long startNs, long endNs,
        long minClip, long maxClip, int participationCapBps, long sliceIntervalNs
    ) {
        this.parentOrderId = parentOrderId;
        this.accountId = accountId;
        this.routeId = routeId;
        this.strategyId = strategyId;
        this.venue = venue;
        this.instrument = instrument;
        this.side = side;
        this.orderType = orderType;
        this.timeInForce = timeInForce;
        this.totalQty = totalQty;
        this.limitPrice = limitPrice;
        this.stopPrice = stopPrice;
        this.startNs = startNs;
        this.endNs = endNs;
        this.minClip = minClip;
        this.maxClip = maxClip;
        this.participationCapBps = participationCapBps;
        this.sliceIntervalNs = sliceIntervalNs;
    }
}
