package com.orderflow.bindings;

import java.util.ArrayList;
import java.util.List;

import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.PointerByReference;

/** High-level Java wrapper around the Orderflow execution C ABI. */
public final class OrderflowExecutionEngine implements AutoCloseable {
    private static final int EVENT_CAPACITY = 32;

    private final OrderflowNative nativeLib;
    private Pointer engine;

    /**
     * Creates an execution engine using route configuration and optional native path.
     *
     * @param nativePath native library path, or null/blank for default debug path
     * @param route route and risk configuration
     */
    public OrderflowExecutionEngine(String nativePath, RouteConfig route) {
        String libPath = nativePath == null || nativePath.isBlank() ? defaultLibraryPath() : nativePath;
        this.nativeLib = OrderflowNative.load(libPath);
        OfExecutionRouteConfig cfg = toNative(route);
        cfg.write();
        PointerByReference out = new PointerByReference();
        check(nativeLib.of_execution_engine_create(cfg, out), "of_execution_engine_create", List.of());
        this.engine = out.getValue();
    }

    /** Returns execution ABI version. */
    public int apiVersion() {
        return nativeLib.of_execution_api_version();
    }

    /** Starts execution adapter/session. */
    public void start() {
        requireEngine();
        check(nativeLib.of_execution_engine_start(engine), "of_execution_engine_start", List.of());
    }

    /** Stops execution adapter/session. */
    public void stop() {
        if (engine != null) {
            check(nativeLib.of_execution_engine_stop(engine), "of_execution_engine_stop", List.of());
        }
    }

    /** Destroys native execution engine. */
    @Override
    public void close() {
        if (engine != null) {
            nativeLib.of_execution_engine_destroy(engine);
            engine = null;
        }
    }

    /** Submits an order and returns generated execution events. */
    public List<ExecutionEvent> submitOrder(OrderRequest request) {
        requireEngine();
        OfExecutionOrderRequest req = toNative(request);
        req.write();
        EventCall call = new EventCall();
        int rc = nativeLib.of_execution_submit_order(engine, req, call.events, call.len);
        List<ExecutionEvent> decoded = decode(call.events, call.len.getValue());
        check(rc, "of_execution_submit_order", decoded);
        return decoded;
    }

    /** Cancels an order and returns generated execution events. */
    public List<ExecutionEvent> cancelOrder(CancelRequest request) {
        requireEngine();
        OfExecutionCancelRequest req = toNative(request);
        req.write();
        EventCall call = new EventCall();
        int rc = nativeLib.of_execution_cancel_order(engine, req, call.events, call.len);
        List<ExecutionEvent> decoded = decode(call.events, call.len.getValue());
        check(rc, "of_execution_cancel_order", decoded);
        return decoded;
    }

    /** Amends an order and returns generated execution events. */
    public List<ExecutionEvent> amendOrder(AmendRequest request) {
        requireEngine();
        OfExecutionAmendRequest req = toNative(request);
        req.write();
        EventCall call = new EventCall();
        int rc = nativeLib.of_execution_amend_order(engine, req, call.events, call.len);
        List<ExecutionEvent> decoded = decode(call.events, call.len.getValue());
        check(rc, "of_execution_amend_order", decoded);
        return decoded;
    }

    /** Polls execution events. */
    public List<ExecutionEvent> pollExecution() {
        requireEngine();
        EventCall call = new EventCall();
        int rc = nativeLib.of_execution_poll(engine, call.events, call.len);
        List<ExecutionEvent> decoded = decode(call.events, call.len.getValue());
        check(rc, "of_execution_poll", decoded);
        return decoded;
    }

    /** Returns current order state. */
    public ExecutionOrderState orderState(String clientOrderId) {
        requireEngine();
        OfExecutionOrderState state = new OfExecutionOrderState();
        int rc = nativeLib.of_execution_get_order_state(engine, clientOrderId, state);
        check(rc, "of_execution_get_order_state", List.of());
        state.read();
        return new ExecutionOrderState(
            cstr(state.client_order_id),
            cstr(state.venue_order_id),
            cstr(state.account_id),
            cstr(state.route_id),
            cstr(state.venue),
            cstr(state.instrument),
            state.status,
            state.order_qty,
            state.cumulative_qty,
            state.leaves_qty,
            state.average_price,
            state.updated_ns
        );
    }

    /** Returns execution health. */
    public ExecutionHealth executionHealth() {
        requireEngine();
        OfExecutionHealth health = new OfExecutionHealth();
        int rc = nativeLib.of_execution_health(engine, health);
        check(rc, "of_execution_health", List.of());
        health.read();
        return new ExecutionHealth(health.connected != 0, health.degraded != 0, health.health_seq);
    }

    /** Returns execution metrics. */
    public ExecutionMetrics executionMetrics() {
        requireEngine();
        OfExecutionMetrics metrics = new OfExecutionMetrics();
        int rc = nativeLib.of_execution_metrics(engine, metrics);
        check(rc, "of_execution_metrics", List.of());
        metrics.read();
        return new ExecutionMetrics(
            metrics.submitted,
            metrics.cancelled,
            metrics.amended,
            metrics.events_applied,
            metrics.risk_rejected,
            metrics.adapter_errors,
            metrics.recovered
        );
    }

    private static OfExecutionRouteConfig toNative(RouteConfig route) {
        OfExecutionRouteConfig cfg = new OfExecutionRouteConfig();
        cfg.route_id = route.routeId;
        cfg.account_id = route.accountId;
        cfg.venue = route.venue;
        cfg.instrument = route.instrument;
        cfg.enabled = (byte) (route.enabled ? 1 : 0);
        cfg.kill_switch = (byte) (route.riskLimits.killSwitch ? 1 : 0);
        cfg.max_order_qty = route.riskLimits.maxOrderQty;
        cfg.max_order_notional = route.riskLimits.maxOrderNotional;
        cfg.max_open_orders = route.riskLimits.maxOpenOrders;
        cfg.max_open_notional = route.riskLimits.maxOpenNotional;
        cfg.price_band_ticks = route.riskLimits.priceBandTicks;
        return cfg;
    }

    private static OfExecutionOrderRequest toNative(OrderRequest request) {
        OfExecutionOrderRequest req = new OfExecutionOrderRequest();
        req.client_order_id = request.clientOrderId;
        req.account_id = request.accountId;
        req.route_id = request.routeId;
        req.strategy_id = request.strategyId;
        req.venue = request.venue;
        req.instrument = request.instrument;
        req.side = request.side;
        req.order_type = request.orderType;
        req.time_in_force = request.timeInForce;
        req.quantity = request.quantity;
        req.limit_price = request.limitPrice;
        req.stop_price = request.stopPrice;
        req.ts_exchange_ns = request.tsExchangeNs;
        req.ts_recv_ns = request.tsRecvNs;
        return req;
    }

    private static OfExecutionCancelRequest toNative(CancelRequest request) {
        OfExecutionCancelRequest req = new OfExecutionCancelRequest();
        req.client_order_id = request.clientOrderId;
        req.orig_client_order_id = request.origClientOrderId;
        req.venue_order_id = request.venueOrderId;
        req.account_id = request.accountId;
        req.route_id = request.routeId;
        req.venue = request.venue;
        req.instrument = request.instrument;
        req.ts_recv_ns = request.tsRecvNs;
        return req;
    }

    private static OfExecutionAmendRequest toNative(AmendRequest request) {
        OfExecutionAmendRequest req = new OfExecutionAmendRequest();
        req.client_order_id = request.clientOrderId;
        req.orig_client_order_id = request.origClientOrderId;
        req.venue_order_id = request.venueOrderId;
        req.account_id = request.accountId;
        req.route_id = request.routeId;
        req.venue = request.venue;
        req.instrument = request.instrument;
        req.quantity = request.quantity;
        req.limit_price = request.limitPrice;
        req.ts_recv_ns = request.tsRecvNs;
        return req;
    }

    private static List<ExecutionEvent> decode(OfExecutionEvent[] events, int count) {
        List<ExecutionEvent> out = new ArrayList<>(Math.max(0, count));
        int limit = Math.min(count, events.length);
        for (int idx = 0; idx < limit; idx++) {
            events[idx].read();
            OfExecutionEvent ev = events[idx];
            out.add(new ExecutionEvent(
                ev.exec_type,
                ev.order_status,
                cstr(ev.client_order_id),
                cstr(ev.orig_client_order_id),
                cstr(ev.venue_order_id),
                cstr(ev.execution_id),
                cstr(ev.account_id),
                cstr(ev.route_id),
                cstr(ev.venue),
                cstr(ev.instrument),
                ev.last_qty,
                ev.last_price,
                ev.cumulative_qty,
                ev.leaves_qty,
                ev.average_price,
                ev.ts_exchange_ns,
                ev.ts_recv_ns,
                ev.reason,
                cstr(ev.text)
            ));
        }
        return out;
    }

    private static String cstr(byte[] bytes) {
        return Native.toString(bytes);
    }

    private static void check(int rc, String fn, List<ExecutionEvent> events) {
        if (rc == 0) {
            return;
        }
        if (rc == 7) {
            throw new OrderflowRiskException(fn + " failed with OF_ERR_RISK", events);
        }
        if (rc == 1) {
            throw new OrderflowArgException(fn + " failed with OF_ERR_INVALID_ARG");
        }
        if (rc == 2) {
            throw new OrderflowStateException(fn + " failed with OF_ERR_STATE");
        }
        throw new OrderflowException(fn + " failed with error code " + rc);
    }

    private void requireEngine() {
        if (engine == null) {
            throw new OrderflowStateException("execution engine is closed");
        }
    }

    private static String defaultLibraryPath() {
        String os = System.getProperty("os.name").toLowerCase();
        if (os.contains("win")) {
            return "target/debug/of_ffi_c.dll";
        }
        if (os.contains("mac")) {
            return "target/debug/libof_ffi_c.dylib";
        }
        return "target/debug/libof_ffi_c.so";
    }

    private static final class EventCall {
        private final OfExecutionEvent[] events;
        private final IntByReference len;

        private EventCall() {
            this.events = (OfExecutionEvent[]) new OfExecutionEvent().toArray(EVENT_CAPACITY);
            this.len = new IntByReference(EVENT_CAPACITY);
        }
    }
}
