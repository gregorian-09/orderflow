package com.orderflow.bindings;

/** Callback event envelope delivered by subscription listeners. */
public final class OrderflowEvent {
    /** Exchange timestamp in nanoseconds. */
    public final long tsExchangeNs;
    /** Receive timestamp in nanoseconds. */
    public final long tsRecvNs;
    /** Stream kind id. */
    public final int kind;
    /** Payload schema id. */
    public final int schemaId;
    /** Data-quality flag bits. */
    public final int qualityFlags;
    /** UTF-8 JSON payload string. */
    public final String payloadJson;

    /** Creates immutable callback event object. */
    public OrderflowEvent(long tsExchangeNs, long tsRecvNs, int kind, int schemaId, int qualityFlags, String payloadJson) {
        this.tsExchangeNs = tsExchangeNs;
        this.tsRecvNs = tsRecvNs;
        this.kind = kind;
        this.schemaId = schemaId;
        this.qualityFlags = qualityFlags;
        this.payloadJson = payloadJson;
    }
}
