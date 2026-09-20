//! Additive OMS building blocks for execution integrations.

use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use of_execution_core::{
    execution_wal_checksum, AccountId, AmendRequest, CancelRequest, ClientOrderId,
    ExecutionCoreError, ExecutionEvent, ExecutionSymbol, ExecutionText, ExecutionType, FixedAscii,
    OrderPrice, OrderQty, OrderRequest, OrderSide, OrderState, OrderStatus, OrderType, RiskCheck,
    RiskContext, RiskDecision, RiskLimits, RiskRejectReason, RouteId, StrategyId, TimeInForce,
    WalIntegrityReport, WalRecordKind, WalRecordView, WalReplayCursor, WalSegmentId, WalSequence,
    WalSyncPolicy,
};

use crate::{
    AllowAllRiskGate, ExecutionAdapter, ExecutionCapabilities, ExecutionCommand,
    ExecutionCommandKind, ExecutionCommandReport, ExecutionEngine, ExecutionError,
    ExecutionEventBuffer, ExecutionJournal, ExecutionMetrics, ExecutionResult, InMemoryJournal,
    JournalCommandKind, JournalRecord, RouteConfig, RouteKey, SimExecutionAdapter,
};

#[path = "oms_allocation.rs"]
mod oms_allocation;
#[path = "oms_foundation.rs"]
mod oms_foundation;
#[path = "oms_journal.rs"]
mod oms_journal;
#[path = "oms_reconciliation.rs"]
mod oms_reconciliation;
#[path = "oms_recovery.rs"]
mod oms_recovery;
#[path = "oms_routing.rs"]
mod oms_routing;
#[path = "oms_safety.rs"]
mod oms_safety;
#[path = "oms_telemetry.rs"]
mod oms_telemetry;

#[allow(unused_imports)]
pub(crate) use oms_foundation::FanoutInner;

#[allow(unused_imports)]
pub(crate) use oms_journal::{
    command_wal_kind, decode_command_payload, decode_event_payload, decode_recovery_wal_payload,
    decode_wal_command, decode_wal_payload, encode_amend_payload, encode_cancel_payload,
    encode_command_payload, encode_event_payload, encode_submit_payload, event_wal_kind,
    inspect_segmented_wal_root, is_risk_boundary_wal_kind, list_segment_ids, load_segment_manifest,
    now_ns, parse_segment_id, put_fixed, put_payload_i64, put_payload_u16, put_payload_u64,
    put_payload_u8, replay_segmented_recovery_records, replay_wal_bytes, scan_segment_file,
    scan_wal_file, segment_path, validate_wal_link, validate_wal_sequence, wal_error,
    write_segment_manifest, DecodedRecoveryRecord, DecodedWalCommand, PayloadReader, SegmentScan,
};

#[allow(unused_imports)]
pub(crate) use oms_recovery::{
    apply_recovered_command, apply_recovered_event, apply_recovered_state_transition,
    checkpoint_checksum, decode_checkpoint, decode_checkpoint_position, decode_order_state,
    encode_checkpoint, encode_checkpoint_position, encode_order_state, finish_decoded_recovery,
    inspect_checkpoint_store_root, list_checkpoint_manifests, load_checkpoint_file,
    order_side_from_u8, order_type_from_u8, put_payload_i128, put_payload_u32,
    recover_oms_state_from_decoded_records, recovered_order_mut, recovery_event_key,
    recovery_result, sync_directory, time_in_force_from_u8, write_optional_json_u64,
};

#[allow(unused_imports)]
pub(crate) use oms_allocation::{
    classify_allocation_issue, classify_reconciliation_issue, priority_quantities,
    proportional_quantities,
};

#[allow(unused_imports)]
pub(crate) use oms_safety::{active_safety_conditions, MessageRateWindow};

#[allow(unused_imports)]
pub(crate) use oms_telemetry::{
    diff_if_ordered, first_last_latency, internal_timestamp_points, timestamp_points,
    validate_timestamp_trace,
};

#[allow(unused_imports)]
pub(crate) use oms_routing::{
    command_kind_from_u8, command_kind_u8, event_to_journal_line, execution_type_from_u8, fixed,
    order_status_from_u8, parse_event_parts, parse_i64, parse_journal_line, parse_u64, parse_u8,
    reject, risk_reason_from_u8, sanitize_field, StableHasher,
};

pub use oms_allocation::*;
pub use oms_foundation::*;
pub use oms_journal::*;
pub use oms_reconciliation::*;
pub use oms_recovery::*;
pub use oms_routing::*;
pub use oms_safety::*;
pub use oms_telemetry::*;

include!("oms_tests.rs");
