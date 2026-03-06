package com.orderflow.examples;

import com.orderflow.bindings.DataQualityFlags;
import com.orderflow.bindings.EngineConfig;
import com.orderflow.bindings.OrderflowEngine;
import com.orderflow.bindings.OrderflowEvent;
import com.orderflow.bindings.StreamKind;
import com.orderflow.bindings.Symbol;

public final class HealthExample {
    private HealthExample() {}

    public static void main(String[] args) {
        EngineConfig defaults = EngineConfig.defaults();
        EngineConfig cfg = new EngineConfig(
                "java-health-example",
                defaults.configPath,
                defaults.logLevel,
                defaults.enablePersistence,
                defaults.auditMaxBytes,
                defaults.auditMaxFiles,
                defaults.auditRedactTokensCsv,
                defaults.dataRetentionMaxBytes,
                defaults.dataRetentionMaxAgeSecs);

        try (OrderflowEngine engine = new OrderflowEngine(null, cfg)) {
            engine.start();
            Symbol sym = new Symbol("CME", "ESM6", 10);
            engine.subscribe(sym, StreamKind.HEALTH, HealthExample::onHealthEvent);
            engine.subscribe(sym, StreamKind.ANALYTICS);

            engine.pollOnce(DataQualityFlags.NONE);
            engine.pollOnce(DataQualityFlags.ADAPTER_DEGRADED);
            engine.pollOnce(DataQualityFlags.NONE);
            engine.unsubscribe(sym);

            System.out.println("metrics=" + engine.metricsJson());
            engine.stop();
        }
    }

    private static void onHealthEvent(OrderflowEvent event) {
        System.out.println("health payload=" + event.payloadJson);
    }
}
