#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub(crate) use std::error::Error;
pub(crate) use std::fmt;
pub(crate) use std::fs::{self, File};
pub(crate) use std::io::{self, BufReader, Read};
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::sync::Arc;

pub(crate) use arrow_array::builder::{
    BinaryBuilder, StringBuilder, UInt16Builder, UInt32Builder, UInt64Builder,
};
pub(crate) use arrow_array::{
    Array, BinaryArray, RecordBatch, UInt16Array, UInt32Array, UInt64Array,
};
pub(crate) use arrow_schema::{DataType, Field, Schema, SchemaRef};
pub(crate) use of_persist::{
    decode_normalized_market_data_record, MarketDataColdExportFormat,
    MarketDataColdExportPartition, MarketDataRetentionInput, MarketDataWal, MarketDataWalRecord,
    MarketDataWalRecordKind, MarketDataWalReplayFilter, MarketDataWalSequence,
    NormalizedMarketDataCodecError, SegmentedMarketDataWal,
};
pub(crate) use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
pub(crate) use parquet::arrow::ArrowWriter;
pub(crate) use parquet::basic::{Compression, ZstdLevel};
pub(crate) use parquet::errors::ParquetError;
pub(crate) use parquet::file::properties::WriterProperties;
pub(crate) use sha2::{Digest, Sha256};

pub(crate) const EXPORT_SCHEMA_VERSION: u16 = 1;
pub(crate) const DEFAULT_BATCH_ROWS: usize = 32_768;
pub(crate) const DEFAULT_ROW_GROUP_ROWS: usize = 131_072;
pub(crate) const HASH_BUFFER_BYTES: usize = 64 * 1024;

#[path = "parquet_helpers.rs"]
mod helpers;
#[path = "parquet_temp.rs"]
mod temp;
#[path = "parquet_types.rs"]
mod types;
#[path = "parquet_writer.rs"]
mod writer;

pub(crate) use helpers::*;
pub(crate) use temp::TemporaryExport;
pub use types::*;
pub use writer::*;

include!("parquet_tests.rs");
