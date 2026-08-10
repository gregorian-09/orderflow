package com.orderflow.bindings;

import java.util.Optional;

import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;

/** Deterministic native TWAP planner with explicit release accounting. */
public final class TwapExecutionAlgo implements AutoCloseable {
    private final OrderflowNative nativeLib;
    private Pointer algo;

    /** Creates a validated native TWAP parent handle. */
    public TwapExecutionAlgo(String nativePath, TwapConfig config) {
        String libPath = nativePath == null || nativePath.isBlank() ? defaultLibraryPath() : nativePath;
        this.nativeLib = OrderflowNative.load(libPath);
        OfExecutionTwapConfig nativeConfig = toNative(config);
        nativeConfig.write();
        PointerByReference out = new PointerByReference();
        check(nativeLib.of_execution_twap_algo_create(nativeConfig, out), "of_execution_twap_algo_create");
        this.algo = out.getValue();
    }

    /** Plans one due child without advancing released quantity. */
    public Optional<AlgoChildPlan> plan(long nowNs, String childOrderId, String clientOrderId, long tsRecvNs) {
        requireOpen();
        OfExecutionAlgoChildPlan nativePlan = new OfExecutionAlgoChildPlan();
        check(
            nativeLib.of_execution_twap_algo_plan(
                algo, nowNs, childOrderId, clientOrderId, tsRecvNs, nativePlan
            ),
            "of_execution_twap_algo_plan"
        );
        nativePlan.read();
        if (nativePlan.has_plan == 0) {
            return Optional.empty();
        }
        OrderRequest request = new OrderRequest(
            cstr(nativePlan.client_order_id), cstr(nativePlan.account_id), cstr(nativePlan.route_id),
            cstr(nativePlan.strategy_id), cstr(nativePlan.venue), cstr(nativePlan.instrument),
            nativePlan.side, nativePlan.order_type, nativePlan.time_in_force, nativePlan.quantity,
            nativePlan.limit_price, nativePlan.stop_price, 0, nativePlan.ts_recv_ns
        );
        return Optional.of(new AlgoChildPlan(
            cstr(nativePlan.child_order_id), cstr(nativePlan.parent_order_id), nativePlan.due_ns, request
        ));
    }

    /** Commits the pending child after successful OMS submission. */
    public void commitPending() {
        requireOpen();
        check(nativeLib.of_execution_twap_algo_commit_pending(algo), "of_execution_twap_algo_commit_pending");
    }

    /** Discards the pending child when OMS submission did not occur. */
    public void discardPending() {
        requireOpen();
        check(nativeLib.of_execution_twap_algo_discard_pending(algo), "of_execution_twap_algo_discard_pending");
    }

    /** Folds a child fill/status update into parent progress. */
    public void recordExecution(long lastQty, long leavesQty, int orderStatus) {
        requireOpen();
        check(
            nativeLib.of_execution_twap_algo_record_execution(algo, lastQty, leavesQty, orderStatus),
            "of_execution_twap_algo_record_execution"
        );
    }

    /** Returns current parent progress. */
    public AlgoProgress progress() {
        requireOpen();
        OfExecutionAlgoProgress nativeProgress = new OfExecutionAlgoProgress();
        check(nativeLib.of_execution_twap_algo_progress(algo, nativeProgress), "of_execution_twap_algo_progress");
        nativeProgress.read();
        return new AlgoProgress(
            nativeProgress.target_qty, nativeProgress.released_qty, nativeProgress.completed_qty,
            nativeProgress.open_qty, nativeProgress.rejected_children, nativeProgress.terminal_children,
            nativeProgress.has_pending_plan != 0
        );
    }

    /** Destroys the native planner. */
    @Override
    public void close() {
        if (algo != null) {
            nativeLib.of_execution_twap_algo_destroy(algo);
            algo = null;
        }
    }

    private static OfExecutionTwapConfig toNative(TwapConfig config) {
        OfExecutionTwapConfig nativeConfig = new OfExecutionTwapConfig();
        nativeConfig.parent_order_id = config.parentOrderId;
        nativeConfig.account_id = config.accountId;
        nativeConfig.route_id = config.routeId;
        nativeConfig.strategy_id = config.strategyId;
        nativeConfig.venue = config.venue;
        nativeConfig.instrument = config.instrument;
        nativeConfig.side = config.side;
        nativeConfig.order_type = config.orderType;
        nativeConfig.time_in_force = config.timeInForce;
        nativeConfig.total_qty = config.totalQty;
        nativeConfig.limit_price = config.limitPrice;
        nativeConfig.stop_price = config.stopPrice;
        nativeConfig.start_ns = config.startNs;
        nativeConfig.end_ns = config.endNs;
        nativeConfig.min_clip = config.minClip;
        nativeConfig.max_clip = config.maxClip;
        nativeConfig.participation_cap_bps = (short) config.participationCapBps;
        nativeConfig.slice_interval_ns = config.sliceIntervalNs;
        return nativeConfig;
    }

    private static String cstr(byte[] value) {
        return Native.toString(value);
    }

    private static void check(int rc, String function) {
        if (rc == 0) {
            return;
        }
        if (rc == 1) {
            throw new OrderflowArgException(function + " failed with OF_ERR_INVALID_ARG");
        }
        if (rc == 2) {
            throw new OrderflowStateException(function + " failed with OF_ERR_STATE");
        }
        throw new OrderflowException(function + " failed with error code " + rc);
    }

    private void requireOpen() {
        if (algo == null) {
            throw new OrderflowStateException("TWAP algorithm is closed");
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
}
