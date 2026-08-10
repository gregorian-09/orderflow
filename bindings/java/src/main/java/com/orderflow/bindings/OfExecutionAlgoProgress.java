package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native `of_execution_algo_progress_t`. */
@Structure.FieldOrder({
    "target_qty", "released_qty", "completed_qty", "open_qty", "rejected_children",
    "terminal_children", "has_pending_plan"
})
/** Native execution-algorithm progress structure. */
public class OfExecutionAlgoProgress extends Structure {
    /** Parent target quantity. */
    public long target_qty;
    /** Quantity committed as child submissions. */
    public long released_qty;
    /** Filled quantity. */
    public long completed_qty;
    /** Estimated open child quantity. */
    public long open_qty;
    /** Rejected child count. */
    public long rejected_children;
    /** Terminal child count. */
    public long terminal_children;
    /** Non-zero when a child plan awaits commit/discard. */
    public byte has_pending_plan;
}
