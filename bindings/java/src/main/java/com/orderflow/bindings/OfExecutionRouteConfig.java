package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_route_config_t`. */
@Structure.FieldOrder({"route_id", "account_id", "venue", "instrument", "enabled", "kill_switch", "max_order_qty", "max_order_notional", "max_open_orders", "max_open_notional", "price_band_ticks"})
public class OfExecutionRouteConfig extends Structure {
    /** Route id. */
    public String route_id;
    /** Account id. */
    public String account_id;
    /** Venue id. */
    public String venue;
    /** Instrument id. */
    public String instrument;
    /** Enabled flag. */
    public byte enabled;
    /** Kill switch flag. */
    public byte kill_switch;
    /** Maximum order quantity. */
    public long max_order_qty;
    /** Maximum order notional. */
    public long max_order_notional;
    /** Maximum open orders. */
    public int max_open_orders;
    /** Maximum open notional. */
    public long max_open_notional;
    /** Price band in ticks. */
    public long price_band_ticks;
}

