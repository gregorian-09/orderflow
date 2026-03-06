package com.orderflow.bindings;

/** Exception for invalid runtime state failures (`OF_ERR_STATE`). */
public final class OrderflowStateException extends OrderflowException {
    /** Creates state exception. */
    public OrderflowStateException(String message) {
        super(message);
    }
}
