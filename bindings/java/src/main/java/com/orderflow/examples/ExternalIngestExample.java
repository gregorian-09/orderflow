package com.orderflow.examples;

import com.orderflow.bindings.BookAction;
import com.orderflow.bindings.EngineConfig;
import com.orderflow.bindings.OrderflowEngine;
import com.orderflow.bindings.OrderflowEvent;
import com.orderflow.bindings.Side;
import com.orderflow.bindings.StreamKind;
import com.orderflow.bindings.Symbol;

public final class ExternalIngestExample {
    private ExternalIngestExample() {}

    public static void main(String[] args) {
        EngineConfig cfg = EngineConfig.defaults();
        try (OrderflowEngine engine = new OrderflowEngine(null, cfg)) {
            engine.start();
            Symbol sym = new Symbol("CME", "ESM6", 10);
            engine.configureExternalFeed(2_000, true);

            engine.subscribe(sym, StreamKind.ANALYTICS, ExternalIngestExample::onEvent);

            engine.ingestBook(sym, Side.BID, 0, 504900, 20, BookAction.UPSERT, 1, 1_000, 1_100, 0);
            engine.ingestTrade(sym, 505000, 7, Side.ASK, 2, 1_200, 1_300, 0);

            System.out.println("analytics=" + engine.analyticsSnapshot(sym));
            System.out.println("signal=" + engine.signalSnapshot(sym));
            engine.stop();
        }
    }

    private static void onEvent(OrderflowEvent event) {
        System.out.println("event kind=" + event.kind + " payload=" + event.payloadJson);
    }
}
