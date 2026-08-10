package com.orderflow.bindings;

/** Aggregate execution-algorithm progress snapshot. */
public final class AlgoProgress {
    /** Parent target quantity. */ public final long targetQty;
    /** Quantity committed as child submissions. */ public final long releasedQty;
    /** Filled quantity. */ public final long completedQty;
    /** Estimated open child quantity. */ public final long openQty;
    /** Rejected child count. */ public final long rejectedChildren;
    /** Terminal child count. */ public final long terminalChildren;
    /** Whether a child plan awaits commit/discard. */ public final boolean hasPendingPlan;

    /** Creates a progress snapshot. */
    public AlgoProgress(
        long targetQty, long releasedQty, long completedQty, long openQty,
        long rejectedChildren, long terminalChildren, boolean hasPendingPlan
    ) {
        this.targetQty = targetQty;
        this.releasedQty = releasedQty;
        this.completedQty = completedQty;
        this.openQty = openQty;
        this.rejectedChildren = rejectedChildren;
        this.terminalChildren = terminalChildren;
        this.hasPendingPlan = hasPendingPlan;
    }
}
