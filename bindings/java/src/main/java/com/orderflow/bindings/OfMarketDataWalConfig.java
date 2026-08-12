package com.orderflow.bindings;

import com.sun.jna.Structure;

@Structure.FieldOrder({
    "root_path",
    "max_segment_bytes",
    "max_payload_bytes",
    "sync_policy",
    "sync_every_records",
    "sync_manifest",
    "queue_capacity",
    "max_queued_payload_bytes",
    "failure_action",
    "writer_thread_name"
})
/** JNA mirror of native {@code of_market_data_wal_config_t}. */
public class OfMarketDataWalConfig extends Structure {
    /** Required WAL directory. */
    public String root_path;
    /** Soft segment byte target. */
    public long max_segment_bytes;
    /** Maximum encoded record payload. */
    public long max_payload_bytes;
    /** Native sync-policy identifier. */
    public int sync_policy;
    /** Sync cadence when policy is every N records. */
    public long sync_every_records;
    /** Manifest synchronization flag. */
    public byte sync_manifest;
    /** Bounded queue record capacity. */
    public int queue_capacity;
    /** Bounded aggregate queued payload bytes. */
    public long max_queued_payload_bytes;
    /** Native persistence-failure action identifier. */
    public int failure_action;
    /** Optional native writer thread name. */
    public String writer_thread_name;
}
