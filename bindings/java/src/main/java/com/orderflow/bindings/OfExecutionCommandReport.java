package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_command_report_t`. */
@Structure.FieldOrder({"sequence", "kind", "result_code", "event_count"})
public class OfExecutionCommandReport extends Structure {
    /** Monotonic command sequence. */
    public long sequence;
    /** Command kind. */
    public int kind;
    /** Result code. */
    public int result_code;
    /** Event count copied or required. */
    public int event_count;
}
