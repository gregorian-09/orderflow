package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_concurrent_config_t`. */
@Structure.FieldOrder({"command_capacity", "report_capacity", "event_buffer_capacity"})
public class OfExecutionConcurrentConfig extends Structure {
    /** Bounded command queue capacity. */
    public int command_capacity;
    /** Bounded report queue capacity. */
    public int report_capacity;
    /** Per-command event buffer capacity. */
    public int event_buffer_capacity;
}
