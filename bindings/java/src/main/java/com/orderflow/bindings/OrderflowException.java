package com.orderflow.bindings;

/** Base runtime exception raised by Java binding operations. */
public class OrderflowException extends RuntimeException {
    /** Creates exception with descriptive message. */
    public OrderflowException(String message) {
        super(message);
    }
}
