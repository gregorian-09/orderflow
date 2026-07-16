package com.orderflow.bindings;

import com.sun.jna.Structure;

@Structure.FieldOrder({
    "segments",
    "records",
    "bytes",
    "first_sequence",
    "last_sequence",
    "checksum_failures",
    "sequence_failures",
    "has_first_sequence",
    "has_last_sequence",
    "valid"
})
/** JNA mirror of native `of_execution_segmented_wal_integrity_report_t`. */
public class OfExecutionSegmentedWalIntegrityReport extends Structure {
    /** Segment file count. */
    public long segments;
    /** Valid WAL frame count. */
    public long records;
    /** Valid WAL bytes consumed. */
    public long bytes;
    /** First WAL sequence when has_first_sequence is non-zero. */
    public long first_sequence;
    /** Last WAL sequence when has_last_sequence is non-zero. */
    public long last_sequence;
    /** Checksum failure count. */
    public long checksum_failures;
    /** Strict sequence failure count. */
    public long sequence_failures;
    /** Non-zero when first_sequence is present. */
    public byte has_first_sequence;
    /** Non-zero when last_sequence is present. */
    public byte has_last_sequence;
    /** Non-zero when all inspected segments decoded cleanly. */
    public byte valid;
}
