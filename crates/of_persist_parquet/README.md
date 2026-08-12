# of_persist_parquet

`of_persist_parquet` is the opt-in columnar cold-storage companion to
[`of_persist`](https://docs.rs/of_persist). It exports validated normalized
market-data WAL records to Apache Parquet without adding Arrow, compression,
or hashing dependencies to the hot persistence crate.

The crate is intentionally a control-plane and research path. Do not call an
export method from an adapter polling or execution hot thread.

## Guarantees

- Hive-style `date/venue/symbol/stream` partition directories.
- Stable schema version `1` with sequence, timestamp, quality, source, payload,
  and optional derived-snapshot columns.
- Configurable bounded record-batch and row-group sizes.
- Snappy, Zstandard, or uncompressed output.
- Temporary-file write followed by atomic, no-clobber same-directory hard-link
  publication.
- Optional file synchronization before publication.
- Full post-write reopen, schema, row-count, sequence-range, and SHA-256
  verification before a retention proof is returned.
- Quality flags are decoded from versioned normalized `OFNE` records; malformed
  normalized payloads fail export instead of being silently downgraded.
- Existing `of_persist` JSONL, WAL, checkpoint, and retention APIs are unchanged.

## Quick Start

```rust,no_run
use of_persist::{MarketDataWal, MarketDataWalConfig, MarketDataWalReplayFilter};
use of_persist_parquet::{
    MarketDataParquetCompression, MarketDataParquetExportConfig,
    MarketDataParquetPartitionKey, MarketDataParquetSourceMetadata,
    MarketDataParquetWriter,
};

let wal = MarketDataWal::open(MarketDataWalConfig::new("data/normalized.ofmw"))?;
let writer = MarketDataParquetWriter::open(
    MarketDataParquetExportConfig::new("data/cold")
        .with_compression(MarketDataParquetCompression::Zstd)
        .with_batch_rows(32_768)
        .with_row_group_rows(131_072),
)?;
let key = MarketDataParquetPartitionKey::new(
    "2026-08-12", "CME", "ESM6", "market-data",
);
let source = MarketDataParquetSourceMetadata::new(
    "cqg-primary", "cqg-webapi", "session-20260812",
);
let proof = writer.export_wal(
    &key,
    &source,
    &wal,
    MarketDataWalReplayFilter::new(),
    &[],
)?;
assert!(proof.verified);
let retention = proof.retention_input(0, 256 * 1024 * 1024);
assert!(retention.cold_export_verified);
# Ok::<(), of_persist_parquet::MarketDataParquetError>(())
```

## Partition And Schema Contract

The caller supplies an ISO `YYYY-MM-DD` partition date. This avoids making UTC
session-boundary assumptions inside the library. Components reject empty
values, path separators, traversal markers, control characters, and reserved
temporary suffixes.

Schema version `1` contains:

| Column | Arrow type | Meaning |
|---|---|---|
| `schema_version` | `UInt16` | Export schema version |
| `partition_date` | `Utf8` | Caller-selected UTC/session date |
| `venue`, `symbol`, `stream` | `Utf8` | Partition identity |
| `source_id`, `adapter_id`, `session_id` | `Utf8` | Capture provenance |
| `record_kind`, `record_kind_name` | `UInt16`, `Utf8` | Stable WAL kind |
| `wal_sequence` | `UInt64` | Global persisted sequence |
| `provider_sequence`, `event_sequence` | `UInt64` | Source/canonical sequences |
| `ts_exchange_ns`, `ts_recv_ns` | `UInt64` | Exchange and receive times |
| `quality_flags` | `UInt32` | Effective live quality flags |
| `payload` | `Binary` | Original decoded WAL payload |
| `payload_checksum` | `UInt32` | Per-payload FNV-1a checksum |
| `derived_schema_id` | nullable `UInt32` | Optional derived snapshot schema |
| `derived_payload` | nullable `Binary` | Optional derived snapshot bytes |

Derived snapshots must be strictly ordered by WAL sequence and are joined in a
single linear pass. Callers choose their serialization and schema id; this
crate does not couple cold export to one analytics snapshot version.

## Verification And Retention

`export_records`, `export_wal`, and `export_segmented_wal` return
`VerifiedMarketDataParquetExport` only after reopening the final file and
validating the exact schema, row count, first/last sequence, Parquet metadata,
and SHA-256 digest. `verify_export` can revalidate a proof later.

`retention_input(created_ns, hot_bytes)` creates an
`of_persist::MarketDataRetentionInput` with `cold_export_verified = true`.
Only a verified proof has this method, preventing an unverified manifest from
accidentally authorizing hot-WAL deletion.

## Operational Rules

- Keep writer options consistent across files in one dataset.
- Choose batch and row-group bounds from measured memory and query workloads.
- Publish files to object storage only after local verification succeeds.
- Reverify the object-store checksum before deleting dependent hot WAL.
- Keep incident windows and checkpoint dependencies under the conservative
  `of_persist` retention planner.
- Treat source/session identifiers as operational metadata, not credentials.
- Export is synchronous by design; schedule it on a dedicated maintenance
  worker or external compactor.
- Replay bridges materialize the selected WAL range before encoding. Rotate and
  export sealed, sequence-bounded segments so memory remains operationally
  bounded for large histories.
- A failed post-write verification removes the newly published local file. A
  verified file is never overwritten by a later export of the same range.

## Compatibility

This crate begins at `0.1.0`. It depends on the established `of_persist 0.4`
record model but does not modify it. Applications adopt the columnar path by
adding this crate explicitly; existing APIs, serialized JSONL files, native ABI,
Python binding, and Java binding remain valid.
