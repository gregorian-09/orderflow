package com.orderflow.bindings;

/** Exception for invalid argument failures (`OF_ERR_INVALID_ARG`). */
public final class OrderflowArgException extends OrderflowException {
    /** Creates argument exception. */
    public OrderflowArgException(String message) {
        super(message);
    }
}
