package com.orderflow.bindings;

import java.util.List;

/** Concurrent execution command report. */
public final class ExecutionCommandReport {
    /** Monotonic command sequence. */
    public final long sequence;
    /** Command kind. */
    public final int kind;
    /** Native result code. */
    public final int resultCode;
    /** Event count copied or required. */
    public final int eventCount;
    /** Decoded execution events. */
    public final List<ExecutionEvent> events;

    /** Creates a command report. */
    public ExecutionCommandReport(long sequence, int kind, int resultCode, int eventCount, List<ExecutionEvent> events) {
        this.sequence = sequence;
        this.kind = kind;
        this.resultCode = resultCode;
        this.eventCount = eventCount;
        this.events = List.copyOf(events);
    }
}
