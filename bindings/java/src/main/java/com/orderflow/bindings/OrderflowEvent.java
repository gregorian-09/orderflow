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

    /**
     * Creates immutable callback event object.
     *
     * @param tsExchangeNs exchange timestamp in nanoseconds
     * @param tsRecvNs receive timestamp in nanoseconds
     * @param kind stream kind id
     * @param schemaId payload schema id
     * @param qualityFlags quality flag bits
     * @param payloadJson UTF-8 JSON payload
     */
    public OrderflowEvent(long tsExchangeNs, long tsRecvNs, int kind, int schemaId, int qualityFlags, String payloadJson) {
        this.tsExchangeNs = tsExchangeNs;
        this.tsRecvNs = tsRecvNs;
        this.kind = kind;
        this.schemaId = schemaId;
        this.qualityFlags = qualityFlags;
        this.payloadJson = payloadJson;
    }
}
