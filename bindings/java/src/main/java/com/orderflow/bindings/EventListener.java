package com.orderflow.bindings;

/** Functional callback for receiving stream events. */
public interface EventListener {
    /**
     * Handles one runtime callback event.
     *
     * @param event immutable callback envelope
     */
    void onEvent(OrderflowEvent event);
}
