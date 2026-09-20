//! Low-allocation FIX tag-value codec primitives for Orderflow.
#![doc = include_str!("../README.md")]

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

mod session;

pub use session::*;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const SEQUENCE_SNAPSHOT_MAGIC: &[u8; 8] = b"OFIXSEQ\0";
const SEQUENCE_SNAPSHOT_VERSION: u16 = 1;
const SEQUENCE_SNAPSHOT_FILE: &str = "fix-sequence.snapshot";
const SEQUENCE_SNAPSHOT_TMP_FILE: &str = "fix-sequence.snapshot.tmp";
const DURABLE_RESEND_MAGIC: &[u8; 8] = b"OFIXRSD\0";
const DURABLE_RESEND_VERSION: u16 = 1;

mod codec;
mod errors;
mod foundation;
mod profile;
mod resend;
mod sequence;
mod transcript;

#[allow(unused_imports)]
pub(crate) use codec::{
    clamp_seq_no, decode_durable_resend_records, decode_sequence_snapshot,
    durable_resend_frame_checksum, durable_resend_io_error, durable_resend_kind_from_byte,
    durable_resend_kind_to_byte, encode_durable_resend_record, encode_message_parts,
    encode_message_parts_with_repeating_group, encode_sequence_snapshot, encode_session_message,
    find_tag_start, hash_bytes, hash_bytes_into, hash_u64, inspect_durable_resend_path, io_error,
    parse_checksum, parse_optional_fix_tag, parse_optional_reject_u64, parse_required_u64,
    parse_u32, parse_u64, parse_usize, patch_body_length, push_gap_fill, put_snapshot_bytes,
    put_snapshot_u16, put_snapshot_u64, read_durable_resend_records,
    sequence_snapshot_checksum_owned, update_transcript_hash, usize_to_u64,
    validate_repeating_group, validate_value, write_checksum, write_field, write_replay_header,
    write_u32, write_u32_digits, write_u64_digits, write_usize_digits, write_usize_padded,
    DurableResendRecord,
};

pub use codec::*;
pub use errors::*;
pub use foundation::*;
pub use profile::*;
pub use resend::*;
pub use sequence::*;
pub use transcript::*;

include!("tests.rs");
