package com.orderflow.examples;

import com.orderflow.bindings.EngineConfig;
import com.orderflow.bindings.OrderflowEngine;
import com.orderflow.bindings.Side;
import com.orderflow.bindings.StreamKind;
import com.orderflow.bindings.Symbol;
import java.nio.file.Files;
import java.nio.file.Path;
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
            String signal = engine.signalSnapshot(sym);
            String metrics = engine.metricsJson();

            require(analytics.contains("\"delta\""), "analytics snapshot missing delta");
            require(analytics.contains("\"delta\":2"), "analytics snapshot delta mismatch");
            require(signal.contains("\"state\""), "signal snapshot missing state");
            require(metrics.contains("\"started\":true"), "metrics missing started=true");
            require(callbackCount.get() > 0, "no callbacks observed in smoke run");

            engine.stop();
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
