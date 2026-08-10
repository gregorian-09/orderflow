package com.orderflow.bindings;

/** Owned execution-algorithm child plan ready for OMS submission. */
public final class AlgoChildPlan {
    /** Child algorithm identifier. */ public final String childOrderId;
    /** Parent algorithm identifier. */ public final String parentOrderId;
    /** Planned release timestamp. */ public final long dueNs;
    /** Canonical OMS request. */ public final OrderRequest request;

    /** Creates an owned child plan. */
    public AlgoChildPlan(String childOrderId, String parentOrderId, long dueNs, OrderRequest request) {
        this.childOrderId = childOrderId;
        this.parentOrderId = parentOrderId;
        this.dueNs = dueNs;
        this.request = request;
    }
}
