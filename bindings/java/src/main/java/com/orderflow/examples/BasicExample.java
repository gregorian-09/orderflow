package com.orderflow.examples;

import com.orderflow.bindings.EngineConfig;
import com.orderflow.bindings.OrderflowEngine;
import com.orderflow.bindings.OrderflowEvent;
import com.orderflow.bindings.StreamKind;
import com.orderflow.bindings.Symbol;

public final class BasicExample {
    private BasicExample() {}

    public static void main(String[] args) {
        EngineConfig cfg = EngineConfig.defaults();
        try (OrderflowEngine engine = new OrderflowEngine(null, cfg)) {
            engine.start();
            Symbol sym = new Symbol("CME", "ESM6", 10);
            engine.subscribe(sym, StreamKind.ANALYTICS, BasicExample::onEvent);
            engine.subscribe(sym, StreamKind.HEALTH, BasicExample::onHealthEvent);
            engine.pollOnce(0);
            System.out.println("api_version=" + engine.apiVersion());
            System.out.println("build=" + engine.buildInfo());
            System.out.println("analytics=" + engine.analyticsSnapshot(sym));
            System.out.println("signal=" + engine.signalSnapshot(sym));
            System.out.println("metrics=" + engine.metricsJson());
            engine.stop();
        }
    }

    private static void onEvent(OrderflowEvent event) {
        System.out.println("event kind=" + event.kind + " payload=" + event.payloadJson);
    }

    private static void onHealthEvent(OrderflowEvent event) {
        System.out.println("health payload=" + event.payloadJson);
    }
}
