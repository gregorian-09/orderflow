#![doc = include_str!("../README.md")]

mod checkpoint;
mod common;
mod market_data_segmented;
mod market_data_writer;
mod normalized_codec;
mod policy;
mod raw_capture;
mod retention;
mod rolling;
mod wal;

pub use checkpoint::*;
pub use common::*;
pub use market_data_segmented::*;
pub use market_data_writer::*;
pub use normalized_codec::*;
pub use policy::*;
pub use raw_capture::*;
pub use retention::*;
pub use rolling::*;
pub use wal::*;

use std::collections::BTreeSet;
use std::fs::{self, create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use of_core::{BookAction, BookUpdate, Side, TradePrint};
use serde::Deserialize;

const JSONL_SCHEMA_VERSION: u32 = 1;
const MARKET_DATA_WAL_MAGIC: [u8; 4] = *b"OFMW";
const MARKET_DATA_WAL_VERSION: u16 = 1;
const MARKET_DATA_WAL_HEADER_LEN: usize = 64;
const MARKET_DATA_CHECKPOINT_MAGIC: [u8; 4] = *b"OFMC";
const MARKET_DATA_CHECKPOINT_VERSION: u16 = 1;
const MARKET_DATA_CHECKPOINT_HEADER_LEN: usize = 64;

#[allow(unused_imports)]
pub(crate) use checkpoint::{
    clean_recovery_plan, degraded_recovery_plan, impossible_recovery_plan,
};

#[allow(unused_imports)]
pub(crate) use retention::{
    active_backpressure_reason, bytes_to_hex, checkpoint_ids, checkpoint_path,
    checkpoint_temp_path, cold_export_file_name, current_unix_nanos,
    decode_market_data_checkpoint_file, decode_market_data_checkpoint_header,
    drop_policy_to_action, encode_market_data_checkpoint_frame, escape_json,
    failure_action_to_backpressure_action, format_cold_export_record,
    market_data_checkpoint_checksum, market_data_wal_record_kind_name, path_component,
    read_market_data_checkpoint_manifest, validate_market_data_checkpoint_file,
    MarketDataCheckpointFrameInput,
};

#[allow(unused_imports)]
pub(crate) use wal::{
    encode_market_data_wal_frame_into, market_data_wal_checksum, range_contains,
    read_exact_or_tail, read_u16, read_u32, read_u64, replay_market_data_wal,
    replay_market_data_wal_filtered, scan_market_data_wal, scan_market_data_wal_into,
    scan_market_data_wal_into_from, update_fnv1a, write_u16, write_u32, write_u64,
    MarketDataWalFrameInput, MarketDataWalRecordHeader, MarketDataWalScan,
};

#[allow(unused_imports)]
pub(crate) use rolling::{
    collect_files, filter_by_sequence_range, invalid_data, parse_book_action, parse_book_line,
    parse_side, parse_trade_line, read_dir_if_exists, read_jsonl_stream, stored_event_kind_rank,
    FileMeta, StoredBookEventWire, StoredTradeEventWire,
};

include!("tests.rs");
