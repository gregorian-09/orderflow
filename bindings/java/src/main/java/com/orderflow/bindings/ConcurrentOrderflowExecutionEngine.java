package com.orderflow.bindings;

import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.Optional;

import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.LongByReference;
import com.sun.jna.ptr.PointerByReference;

/** High-level Java wrapper around the concurrent Orderflow execution C ABI. */
public final class ConcurrentOrderflowExecutionEngine implements AutoCloseable {
    private static final int EVENT_CAPACITY = 32;

    private final OrderflowNative nativeLib;
    private Pointer engine;

    /** Creates a concurrent execution worker with default queue configuration. */
    public ConcurrentOrderflowExecutionEngine(String nativePath, List<RouteConfig> routes) {
        this(nativePath, routes, new ConcurrentExecutionConfig());
    }

    /** Creates a concurrent execution worker. */
    public ConcurrentOrderflowExecutionEngine(String nativePath, List<RouteConfig> routes, ConcurrentExecutionConfig config) {
        String libPath = nativePath == null || nativePath.isBlank() ? defaultLibraryPath() : nativePath;
        this.nativeLib = OrderflowNative.load(libPath);
        OfExecutionRouteConfig[] cfgs = toNativeRoutes(routes);
        OfExecutionConcurrentConfig nativeConfig = new OfExecutionConcurrentConfig();
        nativeConfig.command_capacity = config.commandCapacity;
        nativeConfig.report_capacity = config.reportCapacity;
        nativeConfig.event_buffer_capacity = config.eventBufferCapacity;
        nativeConfig.write();
        PointerByReference out = new PointerByReference();
        check(nativeLib.of_execution_concurrent_engine_create_multi(cfgs, cfgs.length, nativeConfig, out), "of_execution_concurrent_engine_create_multi", List.of());
        this.engine = out.getValue();
    }

    /** Queues a submit command and returns its command sequence. */
    public long submitOrder(OrderRequest request) {
        requireEngine();
        OfExecutionOrderRequest req = toNative(request);
        req.write();
        LongByReference sequence = new LongByReference();
        check(nativeLib.of_execution_concurrent_submit_order(engine, req, sequence), "of_execution_concurrent_submit_order", List.of());
        return sequence.getValue();
    }

    /** Queues a cancel command and returns its command sequence. */
    public long cancelOrder(CancelRequest request) {
        requireEngine();
        OfExecutionCancelRequest req = toNative(request);
        req.write();
        LongByReference sequence = new LongByReference();
        check(nativeLib.of_execution_concurrent_cancel_order(engine, req, sequence), "of_execution_concurrent_cancel_order", List.of());
        return sequence.getValue();
    }

    /** Queues an amend command and returns its command sequence. */
    public long amendOrder(AmendRequest request) {
        requireEngine();
        OfExecutionAmendRequest req = toNative(request);
        req.write();
        LongByReference sequence = new LongByReference();
        check(nativeLib.of_execution_concurrent_amend_order(engine, req, sequence), "of_execution_concurrent_amend_order", List.of());
        return sequence.getValue();
    }

    /** Queues a poll command and returns its command sequence. */
    public long pollExecution() {
        requireEngine();
        LongByReference sequence = new LongByReference();
        check(nativeLib.of_execution_concurrent_poll(engine, sequence), "of_execution_concurrent_poll", List.of());
        return sequence.getValue();
    }

    /** Queues a stop command and returns its command sequence. */
    public long stop() {
        requireEngine();
        LongByReference sequence = new LongByReference();
        check(nativeLib.of_execution_concurrent_stop(engine, sequence), "of_execution_concurrent_stop", List.of());
        return sequence.getValue();
    }

    /** Attempts to receive one command report without blocking. */
    public Optional<ExecutionCommandReport> tryRecvReport() {
        requireEngine();
        OfExecutionCommandReport report = new OfExecutionCommandReport();
        EventCall call = new EventCall();
        int rc = nativeLib.of_execution_concurrent_try_recv_report(engine, report, call.events, call.len);
        if (rc == 5) {
            return Optional.empty();
        }
        List<ExecutionEvent> decoded = decode(call.events, call.len.getValue());
        check(rc, "of_execution_concurrent_try_recv_report", decoded);
        report.read();
        return Optional.of(new ExecutionCommandReport(
            report.sequence,
            report.kind,
            report.result_code,
            report.event_count,
            decoded
        ));
    }

    /** Destroys native concurrent execution worker. */
    @Override
    public void close() {
        if (engine != null) {
            nativeLib.of_execution_concurrent_engine_destroy(engine);
            engine = null;
        }
    }

    private void requireEngine() {
        if (engine == null) {
            throw new OrderflowStateException("concurrent execution engine is closed");
        }
    }

    private static OfExecutionRouteConfig[] toNativeRoutes(List<RouteConfig> routes) {
        Objects.requireNonNull(routes, "routes");
        if (routes.isEmpty()) {
            throw new OrderflowArgException("at least one execution route is required");
        }
        OfExecutionRouteConfig[] cfgs =
            (OfExecutionRouteConfig[]) new OfExecutionRouteConfig().toArray(routes.size());
        for (int idx = 0; idx < routes.size(); idx++) {
            RouteConfig route = Objects.requireNonNull(routes.get(idx), "route");
            cfgs[idx].route_id = route.routeId;
            cfgs[idx].account_id = route.accountId;
            cfgs[idx].venue = route.venue;
            cfgs[idx].instrument = route.instrument;
            cfgs[idx].enabled = (byte) (route.enabled ? 1 : 0);
            cfgs[idx].kill_switch = (byte) (route.riskLimits.killSwitch ? 1 : 0);
            cfgs[idx].max_order_qty = route.riskLimits.maxOrderQty;
            cfgs[idx].max_order_notional = route.riskLimits.maxOrderNotional;
            cfgs[idx].max_open_orders = route.riskLimits.maxOpenOrders;
            cfgs[idx].max_open_notional = route.riskLimits.maxOpenNotional;
            cfgs[idx].price_band_ticks = route.riskLimits.priceBandTicks;
            cfgs[idx].write();
        }
        return cfgs;
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
        if (rc == 5) {
            throw new OrderflowStateException(fn + " failed with OF_ERR_BACKPRESSURE");
        }
        throw new OrderflowException(fn + " failed with error code " + rc);
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
