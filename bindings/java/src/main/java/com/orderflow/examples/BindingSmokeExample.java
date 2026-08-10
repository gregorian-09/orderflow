package com.orderflow.examples;

import com.orderflow.bindings.EngineConfig;
import com.orderflow.bindings.ExecutionOrderType;
import com.orderflow.bindings.ExecutionSide;
import com.orderflow.bindings.ExecutionTimeInForce;
import com.orderflow.bindings.OrderRequest;
import com.orderflow.bindings.OrderflowEngine;
import com.orderflow.bindings.OrderflowExecutionEngine;
import com.orderflow.bindings.RiskLimits;
import com.orderflow.bindings.RouteConfig;
import com.orderflow.bindings.Side;
import com.orderflow.bindings.SignalConfig;
import com.orderflow.bindings.SignalConfigParameter;
import com.orderflow.bindings.SignalValidationConfig;
import com.orderflow.bindings.SignalValidationEvent;
import com.orderflow.bindings.StreamKind;
import com.orderflow.bindings.Symbol;
import com.orderflow.bindings.TwapConfig;
import com.orderflow.bindings.TwapExecutionAlgo;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;

/** Minimal end-to-end smoke check for the Java binding. */
public final class BindingSmokeExample {
    private BindingSmokeExample() {}

    public static void main(String[] args) {
        AtomicInteger callbackCount = new AtomicInteger();
        EngineConfig cfg = EngineConfig.defaults();
        String nativePath = resolveNativeLibraryPath();

        try (OrderflowEngine engine = new OrderflowEngine(nativePath, cfg)) {
            engine.start();
            Symbol sym = new Symbol("CME", "ESM6", 10);
            engine.configureExternalFeed(2_000, true);
            engine.subscribe(sym, StreamKind.ANALYTICS, ev -> callbackCount.incrementAndGet());
            engine.ingestTrade(sym, 505000L, 2L, Side.ASK, 1L, 10L, 11L, 0);

            String analytics = engine.analyticsSnapshot(sym);
            String interval = engine.intervalCandleSnapshot(sym, 60L);
            String signal = engine.signalSnapshot(sym);
            String metrics = engine.metricsJson();

            require(analytics.contains("\"delta\""), "analytics snapshot missing delta");
            require(analytics.contains("\"delta\":2"), "analytics snapshot delta mismatch");
            require(interval.contains("\"window_ns\":60"), "interval candle snapshot window mismatch");
            require(interval.contains("\"trade_count\":1"), "interval candle snapshot trade count mismatch");
            require(signal.contains("\"state\""), "signal snapshot missing state");
            require(metrics.contains("\"started\":true"), "metrics missing started=true");
            require(callbackCount.get() > 0, "no callbacks observed in smoke run");

            engine.stop();
        }

        RiskLimits limits = new RiskLimits(false, 100L, 1_000_000L, 10, 10_000_000L, 0L);
        RouteConfig route = new RouteConfig("SIM", "ACC", "SIM", "ES", true, limits);
        try (OrderflowExecutionEngine execution = new OrderflowExecutionEngine(nativePath, route)) {
            execution.start();
            OrderRequest request = new OrderRequest(
                "SMOKE-1", "ACC", "SIM", "SMOKE", "SIM", "ES",
                ExecutionSide.BUY, ExecutionOrderType.LIMIT, ExecutionTimeInForce.DAY,
                1L, 5_000L, 0L, 0L, 1L
            );
            require(!execution.submitOrder(request).isEmpty(), "execution binding returned no events");
            require(
                "SMOKE-1".equals(execution.orderState("SMOKE-1").clientOrderId),
                "execution binding order-state mismatch"
            );

            TwapConfig twapConfig = new TwapConfig(
                "TWAP-PARENT", "ACC", "SIM", "TWAP", "SIM", "ES",
                ExecutionSide.BUY, ExecutionOrderType.LIMIT, ExecutionTimeInForce.DAY,
                100L, 5_000L, 0L, 1_000L, 11_000L, 10L, 25L, 0, 2_000L
            );
            try (TwapExecutionAlgo twap = new TwapExecutionAlgo(nativePath, twapConfig)) {
                var child = twap.plan(1_000L, "TWAP-CHILD-1", "TWAP-ORDER-1", 1_001L)
                    .orElseThrow(() -> new IllegalStateException("TWAP binding returned no due child"));
                var childEvents = execution.submitOrder(child.request);
                twap.commitPending();
                childEvents.forEach(event ->
                    twap.recordExecution(event.lastQty, event.leavesQty, event.orderStatus)
                );
                require(twap.progress().releasedQty == 20L, "TWAP released quantity mismatch");
                require(twap.progress().completedQty == 20L, "TWAP completed quantity mismatch");
            }
            execution.stop();
        }

        SignalConfig signalConfig = new SignalConfig(
            "delta_momentum_v1",
            List.of(SignalConfigParameter.integer("threshold", 10L))
        );
        require(
            OrderflowEngine.validateSignalConfig(signalConfig, nativePath).valid,
            "signal config validation failed"
        );
        var validationReport = OrderflowEngine.validateSignalReplay(
            signalConfig,
            List.of(
                new SignalValidationEvent(20L, 20L, 20L, 0L, 100L, 100L, 99L, 101L, 1L),
                new SignalValidationEvent(-20L, 0L, 20L, 20L, 90L, 95L, 89L, 101L, 2L),
                new SignalValidationEvent(-20L, -20L, 20L, 40L, 80L, 90L, 79L, 101L, 3L)
            ),
            new SignalValidationConfig(1L, 0L, 0, true, true),
            nativePath
        );
        require(validationReport.evaluatedEvents == 3L, "signal validation event count mismatch");
        require(
            Integer.valueOf(5_000).equals(validationReport.directionalAccuracyBps),
            "signal validation accuracy mismatch"
        );

        Path recoveryRoot;
        try {
            recoveryRoot = Files.createTempDirectory("orderflow-recovery-smoke-");
        } catch (java.io.IOException error) {
            throw new IllegalStateException("failed to create recovery smoke root", error);
        }
        try {
            var recovery = OrderflowExecutionEngine.inspectRecovery(
                nativePath,
                recoveryRoot.toString(),
                null,
                false
            );
            require(recovery.schemaVersion == 1, "recovery report schema mismatch");
            require(recovery.orders == 0L, "empty recovery root returned orders");
            require(
                recovery.venueReconciliationRequired,
                "recovery report did not preserve reconciliation gate"
            );
            require(!recovery.submissionsEnabled, "recovery report enabled submissions");
        } finally {
            try {
                Files.deleteIfExists(recoveryRoot);
            } catch (java.io.IOException error) {
                throw new IllegalStateException("failed to remove recovery smoke root", error);
            }
        }

        System.out.println("java binding smoke: PASS");
    }

    private static void require(boolean condition, String message) {
        if (!condition) {
            throw new IllegalStateException(message);
        }
    }

    private static String resolveNativeLibraryPath() {
        String mapped = System.mapLibraryName("of_ffi_c");
        Path repoRootBuild = Path.of("..", "..", "target", "debug", mapped).normalize();
        if (Files.exists(repoRootBuild)) {
            return repoRootBuild.toString();
        }
        Path localBuild = Path.of("target", "debug", mapped).normalize();
        if (Files.exists(localBuild)) {
            return localBuild.toString();
        }
        throw new IllegalStateException("native library not found for smoke check");
    }
}
