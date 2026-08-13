# `of_persist_parquet` Reference

> Generated from `crates/of_persist_parquet/Cargo.toml`, `rust-surface.md`, and `rust-values.md`.

**Version:** `0.1.0`<br>
**Description:** Verified Apache Parquet cold export for Orderflow market-data WALs<br>
**Source:** [`crates/of_persist_parquet/src`](https://github.com/gregorian-09/orderflow/tree/main/crates/of_persist_parquet/src)<br>
**Generated Rustdoc:** [open `of_persist_parquet` Rustdoc](https://docs.rs/of_persist_parquet/0.1.0/of_persist_parquet/)

This page is the crate-level index. The source links and generated
Rustdoc are authoritative for exact signatures, conditional compilation,
multiline declarations, and implementation-specific detail.

## Features

- No crate-defined features.

## Local Dependencies

- [`of_core`](./of_core.md)
- [`of_persist`](./of_persist.md)

## Public Declaration Index

| Kind | Name | Summary | Source | Docs marker |
| --- | --- | --- | --- | --- |
| `type` | `MarketDataParquetResult` | Result type for verified Parquet export operations | [`crates/of_persist_parquet/src/lib.rs:36`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L36) | `present` |
| `enum` | `MarketDataParquetError` | Verified Parquet export failure | [`crates/of_persist_parquet/src/lib.rs:41`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L41) | `present` |
| `enum` | `MarketDataParquetCompression` | Compression used for a Parquet export | [`crates/of_persist_parquet/src/lib.rs:129`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L129) | `present` |
| `struct` | `MarketDataParquetExportConfig` | Configuration for [`MarketDataParquetWriter`] | [`crates/of_persist_parquet/src/lib.rs:141`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L141) | `present` |
| `fn` | `new` | Creates export configuration rooted at `root` | [`crates/of_persist_parquet/src/lib.rs:151`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L151) | `present` |
| `fn` | `with_compression` | Sets Parquet column compression | [`crates/of_persist_parquet/src/lib.rs:162`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L162) | `present` |
| `fn` | `with_batch_rows` | Sets the maximum records materialized in one Arrow batch | [`crates/of_persist_parquet/src/lib.rs:168`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L168) | `present` |
| `fn` | `with_row_group_rows` | Sets the maximum rows per Parquet row group | [`crates/of_persist_parquet/src/lib.rs:174`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L174) | `present` |
| `fn` | `with_sync_on_write` | Sets whether the completed temporary file is synchronized before rename | [`crates/of_persist_parquet/src/lib.rs:180`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L180) | `present` |
| `fn` | `root` | Returns the cold-export root | [`crates/of_persist_parquet/src/lib.rs:186`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L186) | `present` |
| `fn` | `compression` | Returns configured compression | [`crates/of_persist_parquet/src/lib.rs:191`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L191) | `present` |
| `fn` | `batch_rows` | Returns the Arrow batch row bound | [`crates/of_persist_parquet/src/lib.rs:196`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L196) | `present` |
| `fn` | `row_group_rows` | Returns the Parquet row-group row bound | [`crates/of_persist_parquet/src/lib.rs:201`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L201) | `present` |
| `fn` | `sync_on_write` | Returns whether completed temporary files are synchronized | [`crates/of_persist_parquet/src/lib.rs:206`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L206) | `present` |
| `struct` | `MarketDataParquetPartitionKey` | Hive-style cold-export partition identity | [`crates/of_persist_parquet/src/lib.rs:213`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L213) | `present` |
| `fn` | `new` | Creates a partition key | [`crates/of_persist_parquet/src/lib.rs:226`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L226) | `present` |
| `struct` | `MarketDataParquetSourceMetadata` | Source provenance repeated in each exported row | [`crates/of_persist_parquet/src/lib.rs:243`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L243) | `present` |
| `fn` | `new` | Creates source provenance metadata | [`crates/of_persist_parquet/src/lib.rs:254`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L254) | `present` |
| `struct` | `MarketDataDerivedSnapshotRef` | Optional derived analytics snapshot joined to one WAL sequence | [`crates/of_persist_parquet/src/lib.rs:269`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L269) | `present` |
| `fn` | `new` | Creates one borrowed derived snapshot row | [`crates/of_persist_parquet/src/lib.rs:280`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L280) | `present` |
| `struct` | `VerifiedMarketDataParquetExport` | Proof produced only after a Parquet file passes full post-write verification | [`crates/of_persist_parquet/src/lib.rs:296`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L296) | `present` |
| `fn` | `retention_input` | Creates conservative retention input backed by this verified proof | [`crates/of_persist_parquet/src/lib.rs:319`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L319) | `present` |
| `struct` | `MarketDataParquetWriter` | Synchronous verified Parquet cold-export writer | [`crates/of_persist_parquet/src/lib.rs:332`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L332) | `present` |
| `fn` | `open` | Opens an export root after validating memory and row-group bounds | [`crates/of_persist_parquet/src/lib.rs:342`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L342) | `present` |
| `fn` | `config` | Returns export configuration | [`crates/of_persist_parquet/src/lib.rs:366`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L366) | `present` |
| `fn` | `schema` | Returns the stable Arrow schema written by this crate version | [`crates/of_persist_parquet/src/lib.rs:371`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L371) | `present` |
| `fn` | `export_records` | Exports caller-provided decoded WAL records and verifies the result | [`crates/of_persist_parquet/src/lib.rs:380`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L380) | `present` |
| `fn` | `export_wal` | Replays matching records from a single-file WAL and exports them | [`crates/of_persist_parquet/src/lib.rs:483`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L483) | `present` |
| `fn` | `export_segmented_wal` | Replays matching records from a segmented WAL and exports them | [`crates/of_persist_parquet/src/lib.rs:500`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L500) | `present` |
| `fn` | `verify_export` | Reopens and fully verifies a previously produced export proof | [`crates/of_persist_parquet/src/lib.rs:518`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L518) | `present` |

## Constants, Aliases, Fields, and Variants

| Kind | Owner | Name | Declared type/value | Source |
| --- | --- | --- | --- | --- |
| `variant` | `MarketDataParquetError` | `Io` | `Io(io::Error)` | [`crates/of_persist_parquet/src/lib.rs:43`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L43) |
| `variant` | `MarketDataParquetError` | `Arrow` | `Arrow(arrow_schema::ArrowError)` | [`crates/of_persist_parquet/src/lib.rs:45`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L45) |
| `variant` | `MarketDataParquetError` | `Parquet` | `Parquet(ParquetError)` | [`crates/of_persist_parquet/src/lib.rs:47`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L47) |
| `variant` | `MarketDataParquetError` | `Persist` | `Persist(of_persist::PersistError)` | [`crates/of_persist_parquet/src/lib.rs:49`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L49) |
| `variant` | `MarketDataParquetError` | `Normalized` | `Normalized(NormalizedMarketDataCodecError)` | [`crates/of_persist_parquet/src/lib.rs:51`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L51) |
| `variant` | `MarketDataParquetError` | `InvalidConfig` | `InvalidConfig(String)` | [`crates/of_persist_parquet/src/lib.rs:53`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L53) |
| `variant` | `MarketDataParquetError` | `InvalidMetadata` | `InvalidMetadata(String)` | [`crates/of_persist_parquet/src/lib.rs:55`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L55) |
| `variant` | `MarketDataParquetError` | `InvalidDerivedSnapshots` | `InvalidDerivedSnapshots(String)` | [`crates/of_persist_parquet/src/lib.rs:57`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L57) |
| `variant` | `MarketDataParquetError` | `Verification` | `Verification(String)` | [`crates/of_persist_parquet/src/lib.rs:59`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L59) |
| `variant` | `MarketDataParquetCompression` | `Uncompressed` | `Uncompressed` | [`crates/of_persist_parquet/src/lib.rs:131`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L131) |
| `variant` | `MarketDataParquetCompression` | `Snappy` | `Snappy` | [`crates/of_persist_parquet/src/lib.rs:133`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L133) |
| `variant` | `MarketDataParquetCompression` | `Zstd` | `Zstd` | [`crates/of_persist_parquet/src/lib.rs:136`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L136) |
| `field` | `MarketDataParquetPartitionKey` | `date` | `: String` | [`crates/of_persist_parquet/src/lib.rs:215`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L215) |
| `field` | `MarketDataParquetPartitionKey` | `venue` | `: String` | [`crates/of_persist_parquet/src/lib.rs:217`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L217) |
| `field` | `MarketDataParquetPartitionKey` | `symbol` | `: String` | [`crates/of_persist_parquet/src/lib.rs:219`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L219) |
| `field` | `MarketDataParquetPartitionKey` | `stream` | `: String` | [`crates/of_persist_parquet/src/lib.rs:221`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L221) |
| `field` | `MarketDataParquetSourceMetadata` | `source_id` | `: String` | [`crates/of_persist_parquet/src/lib.rs:245`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L245) |
| `field` | `MarketDataParquetSourceMetadata` | `adapter_id` | `: String` | [`crates/of_persist_parquet/src/lib.rs:247`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L247) |
| `field` | `MarketDataParquetSourceMetadata` | `session_id` | `: String` | [`crates/of_persist_parquet/src/lib.rs:249`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L249) |
| `field` | `MarketDataDerivedSnapshotRef` | `wal_sequence` | `: MarketDataWalSequence` | [`crates/of_persist_parquet/src/lib.rs:271`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L271) |
| `field` | `MarketDataDerivedSnapshotRef` | `schema_id` | `: u32` | [`crates/of_persist_parquet/src/lib.rs:273`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L273) |
| `field` | `MarketDataDerivedSnapshotRef` | `payload` | `: &'a [u8]` | [`crates/of_persist_parquet/src/lib.rs:275`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L275) |
| `field` | `VerifiedMarketDataParquetExport` | `partition` | `: MarketDataColdExportPartition` | [`crates/of_persist_parquet/src/lib.rs:298`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L298) |
| `field` | `VerifiedMarketDataParquetExport` | `partition_date` | `: String` | [`crates/of_persist_parquet/src/lib.rs:300`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L300) |
| `field` | `VerifiedMarketDataParquetExport` | `schema_version` | `: u16` | [`crates/of_persist_parquet/src/lib.rs:302`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L302) |
| `field` | `VerifiedMarketDataParquetExport` | `sha256_hex` | `: String` | [`crates/of_persist_parquet/src/lib.rs:304`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L304) |
| `field` | `VerifiedMarketDataParquetExport` | `row_groups` | `: u64` | [`crates/of_persist_parquet/src/lib.rs:306`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L306) |
| `field` | `VerifiedMarketDataParquetExport` | `quality_flagged_records` | `: u64` | [`crates/of_persist_parquet/src/lib.rs:308`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L308) |
| `field` | `VerifiedMarketDataParquetExport` | `derived_snapshot_records` | `: u64` | [`crates/of_persist_parquet/src/lib.rs:310`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L310) |
| `field` | `VerifiedMarketDataParquetExport` | `source` | `: MarketDataParquetSourceMetadata` | [`crates/of_persist_parquet/src/lib.rs:312`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L312) |
| `field` | `VerifiedMarketDataParquetExport` | `verified` | `: bool` | [`crates/of_persist_parquet/src/lib.rs:314`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_persist_parquet/src/lib.rs#L314) |

## Audit Requirements

The semantic review for this crate must additionally document every
public item's purpose, invariants, defaults, errors, ownership,
thread-safety, allocation/blocking behavior, persistence implications,
feature availability, introduction version, and tested usage.

- [Rust public surface audit](../rust-surface.md)
- [Rust values and layout audit](../rust-values.md)
- [Package and feature matrix](../package-matrix.md)
