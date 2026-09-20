#![allow(non_camel_case_types)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![doc = include_str!("../README.md")]

mod analytics;
mod execution;
mod helpers;
mod lifecycle;
mod runtime;
mod signals;
mod support;
mod types;

use std::ffi::{c_char, c_void, CString};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use of_adapters::{AdapterConfig, ProviderKind};
use of_core::{
    AnalyticsConfig, AnalyticsSnapshot, BookUpdate, DataQualityFlags, SignalState, SymbolId,
    TradePrint,
};
use of_execution::{
    recover_latest_checkpoint_from_segmented_wal_roots, simulated_engine_with_routes,
    AllowAllRiskGate, CheckpointStoreIntegrityReport, ConcurrentExecutionConfig,
    ConcurrentExecutionEngine, ConcurrentExecutionError, ExecutionCommand, ExecutionCommandKind,
    ExecutionCommandReport, ExecutionEngine, ExecutionError, ExecutionEventBuffer,
    FileExecutionCheckpointStore, InMemoryJournal, RouteConfig, SegmentedWalExecutionJournal,
    SimExecutionAdapter, WalSegmentIntegrityReport,
};
use of_execution_algos::{AlgoProgress, ChildOrderPlan, ParentOrder, TwapSlicePlanner};
use of_execution_core::{
    AmendRequest, CancelRequest, ExecutionEvent, ExecutionSymbol, ExecutionText, ExecutionType,
    FixedAscii, OrderPrice, OrderQty, OrderRequest, OrderSide, OrderState, OrderStatus, OrderType,
    RiskLimits, RiskRejectReason, StrategyId, TimeInForce, VenueOrderId, WalIntegrityReport,
};
use of_persist::{
    BoundedMarketDataWriterConfig, MarketDataPersistenceFailureAction, MarketDataWalSyncPolicy,
    SegmentedMarketDataWalConfig,
};
use of_runtime::{
    adapter_inventory_json as runtime_adapter_inventory_json, build_default_engine,
    load_engine_config_from_path, signal_descriptor_inventory_json, DefaultEngine, EngineConfig,
    ExternalFeedPolicy, RuntimeError,
};
use of_signals::{
    validate_signal_replay_events, SignalConfig, SignalConfigParameter, SignalConfigValue,
    SignalRegistry, SignalReplayEvent, SignalValidationConfig,
};
#[cfg(feature = "tickbar")]
use support::format_bar_series;
use support::{
    action_from_ffi, cstr_to_string, dispatch_callbacks, dispatch_health_callbacks, escape_json,
    format_acd_snapshot, format_agent_type_snapshot, format_almgren_chriss_snapshot,
    format_amihud_snapshot, format_analytics_snapshot, format_book_analytics_snapshot,
    format_book_event_analytics_snapshot, format_book_snapshot, format_cvd_enhancement_snapshot,
    format_dark_lit_correlation_snapshot, format_dark_pool_snapshot,
    format_derived_analytics_snapshot, format_futures_snapshot, format_hasbrouck_snapshot,
    format_institutional_flow_snapshot, format_interval_candle_snapshot,
    format_kinetic_energy_snapshot, format_kyle_lambda_snapshot, format_lob_feature_snapshot,
    format_noise_snapshot, format_oi_analysis_snapshot, format_options_flow_snapshot,
    format_pattern_snapshot, format_regime_snapshot, format_resiliency_snapshot,
    format_session_candle_snapshot, format_spread_decomp_snapshot, format_vol_signature_snapshot,
    format_volatility_snapshot, format_vpin_snapshot, non_empty_string, parse_csv, side_from_ffi,
    symbol_from_ffi, symbol_from_ffi_ref, write_json_to_c_buffer,
};

const API_VERSION: u32 = 0x0001_0000;
const EXECUTION_API_VERSION: u32 = 0x0001_0000;
const BUILD_INFO: &[u8] = concat!("of_ffi_c/", env!("CARGO_PKG_VERSION"), "\0").as_bytes();
const FFI_EVENT_BUFFER_CAP: usize = 32;

pub use analytics::*;
pub use execution::*;
pub(crate) use helpers::*;
pub use lifecycle::*;
pub use runtime::*;
pub(crate) use signals::allocate_json_string;
pub use signals::*;
pub use types::*;

#[cfg(test)]
include!("tests.rs");
