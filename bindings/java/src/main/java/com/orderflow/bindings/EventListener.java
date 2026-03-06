package com.orderflow.bindings;

/** Functional callback for receiving stream events. */
public interface EventListener {
    /** Handles one runtime callback event. */
    void onEvent(OrderflowEvent event);
}
