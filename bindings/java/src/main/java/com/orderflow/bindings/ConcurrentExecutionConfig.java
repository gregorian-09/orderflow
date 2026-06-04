package com.orderflow.bindings;

/** Concurrent execution worker queue configuration. */
public final class ConcurrentExecutionConfig {
    /** Bounded command queue capacity. */
    public final int commandCapacity;
    /** Bounded report queue capacity. */
    public final int reportCapacity;
    /** Per-command event buffer capacity. */
    public final int eventBufferCapacity;

    /** Creates default concurrent execution configuration. */
    public ConcurrentExecutionConfig() {
        this(1024, 1024, 64);
    }

    /** Creates concurrent execution configuration. */
    public ConcurrentExecutionConfig(int commandCapacity, int reportCapacity, int eventBufferCapacity) {
        this.commandCapacity = commandCapacity;
        this.reportCapacity = reportCapacity;
        this.eventBufferCapacity = eventBufferCapacity;
    }
}
