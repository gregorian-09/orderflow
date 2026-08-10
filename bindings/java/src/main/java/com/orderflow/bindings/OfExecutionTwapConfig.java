package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_twap_config_t`. */
@Structure.FieldOrder({
    "parent_order_id", "account_id", "route_id", "strategy_id", "venue", "instrument",
    "side", "order_type", "time_in_force", "total_qty", "limit_price", "stop_price",
    "start_ns", "end_ns", "min_clip", "max_clip", "participation_cap_bps", "slice_interval_ns"
})
/** Native TWAP configuration structure. */
public class OfExecutionTwapConfig extends Structure {
    /** Parent order identifier. */
    public String parent_order_id;
    /** Trading account identifier. */
    public String account_id;
    /** Default route identifier. */
    public String route_id;
    /** Strategy attribution identifier. */
    public String strategy_id;
    /** Venue identifier. */
    public String venue;
    /** Instrument identifier. */
    public String instrument;
    /** Canonical side. */
    public int side;
    /** Canonical order type. */
    public int order_type;
    /** Canonical time in force. */
    public int time_in_force;
    /** Total parent quantity. */
    public long total_qty;
    /** Child limit price. */
    public long limit_price;
    /** Child stop price. */
    public long stop_price;
    /** Parent start timestamp. */
    public long start_ns;
    /** Parent end timestamp. */
    public long end_ns;
    /** Minimum child clip. */
    public long min_clip;
    /** Maximum child clip. */
    public long max_clip;
    /** Optional participation cap in basis points. */
    public short participation_cap_bps;
    /** TWAP slice interval. */
    public long slice_interval_ns;
}
