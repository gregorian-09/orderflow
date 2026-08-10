"""Low-level ``ctypes`` bridge to the Orderflow C ABI.

This module defines:
- Python ``ctypes.Structure`` mirrors of exported C structs.
- shared-library lookup behavior and loader.
- function signatures for all supported ABI calls.

Most users should import from :mod:`orderflow.api` instead of using this module
directly.
"""

from __future__ import annotations

import ctypes
import os
import sys
from ctypes import c_char_p, c_double, c_int32, c_int64, c_uint16, c_uint32, c_uint64, c_uint8, c_void_p
from pathlib import Path
from typing import Optional

from ._generated_signatures import _bind_symbols as _bind_generated_symbols


class OfEngineConfig(ctypes.Structure):
    """ctypes mirror of `of_engine_config_t`."""

    _fields_ = [
        ("instance_id", c_char_p),
        ("config_path", c_char_p),
        ("log_level", c_uint32),
        ("enable_persistence", c_uint8),
        ("audit_max_bytes", c_uint64),
        ("audit_max_files", c_uint32),
        ("audit_redact_tokens_csv", c_char_p),
        ("data_retention_max_bytes", c_uint64),
        ("data_retention_max_age_secs", c_uint64),
    ]


class OfAnalyticsConfig(ctypes.Structure):
    """ctypes mirror of `of_analytics_config_t`."""

    _fields_ = [
        ("agent_small_trade_threshold", c_double),
        ("institutional_trade_threshold", c_int64),
        ("cancel_arrival_window_ns", c_uint64),
        ("vpin_volume_bucket", c_uint32),
        ("vpin_max_buckets", c_uint32),
        ("kyle_lambda_max_len", c_uint32),
        ("cvd_max_len", c_uint32),
        ("vol_estimator_max_len", c_uint32),
        ("noise_max_len", c_uint32),
        ("hasbrouck_max_len", c_uint32),
        ("almgren_chriss_max_len", c_uint32),
        ("acd_max_len", c_uint32),
        ("vol_signature_max_len", c_uint32),
        ("agent_max_len", c_uint32),
        ("agent_min_samples", c_uint32),
        ("institutional_max_len", c_uint32),
        ("resiliency_max_len", c_uint32),
        ("spread_decomp_max_len", c_uint32),
        ("regime_max_len", c_uint32),
        ("event_tracker_max_len", c_uint32),
        ("spread_tracker_max_len", c_uint32),
        ("default_max_len", c_uint32),
    ]

    @staticmethod
    def defaults() -> "OfAnalyticsConfig":
        """Return native analytics configuration defaults."""
        return OfAnalyticsConfig(
            agent_small_trade_threshold=100.0,
            institutional_trade_threshold=5000,
            cancel_arrival_window_ns=1_000_000_000,
            vpin_volume_bucket=5000,
            vpin_max_buckets=50,
            kyle_lambda_max_len=100,
            cvd_max_len=50,
            vol_estimator_max_len=100,
            noise_max_len=100,
            hasbrouck_max_len=100,
            almgren_chriss_max_len=100,
            acd_max_len=100,
            vol_signature_max_len=200,
            agent_max_len=100,
            agent_min_samples=5,
            institutional_max_len=100,
            resiliency_max_len=1024,
            spread_decomp_max_len=100,
            regime_max_len=100,
            event_tracker_max_len=65536,
            spread_tracker_max_len=1024,
            default_max_len=100,
        )


class OfSymbol(ctypes.Structure):
    """ctypes mirror of `of_symbol_t`."""

    _fields_ = [
        ("venue", c_char_p),
        ("symbol", c_char_p),
        ("depth_levels", c_uint16),
    ]


class OfTrade(ctypes.Structure):
    """ctypes mirror of `of_trade_t`."""

    _fields_ = [
        ("symbol", OfSymbol),
        ("price", c_int64),
        ("size", c_int64),
        ("aggressor_side", c_uint32),
        ("sequence", c_uint64),
        ("ts_exchange_ns", c_uint64),
        ("ts_recv_ns", c_uint64),
    ]


class OfBook(ctypes.Structure):
    """ctypes mirror of `of_book_t`."""

    _fields_ = [
        ("symbol", OfSymbol),
        ("side", c_uint32),
        ("level", c_uint16),
        ("price", c_int64),
        ("size", c_int64),
        ("action", c_uint32),
        ("sequence", c_uint64),
        ("ts_exchange_ns", c_uint64),
        ("ts_recv_ns", c_uint64),
    ]


class OfExternalFeedPolicy(ctypes.Structure):
    """ctypes mirror of `of_external_feed_policy_t`."""

    _fields_ = [
        ("stale_after_ms", c_uint64),
        ("enforce_sequence", c_uint8),
    ]


class OfEvent(ctypes.Structure):
    """ctypes mirror of `of_event_t` callback envelope."""

    _fields_ = [
        ("ts_exchange_ns", ctypes.c_uint64),
        ("ts_recv_ns", ctypes.c_uint64),
        ("kind", ctypes.c_uint32),
        ("payload", c_void_p),
        ("payload_len", ctypes.c_uint32),
        ("schema_id", ctypes.c_uint32),
        ("quality_flags", ctypes.c_uint32),
    ]

OfEventCallback = ctypes.CFUNCTYPE(None, ctypes.POINTER(OfEvent), c_void_p)


class OfExecutionRouteConfig(ctypes.Structure):
    """ctypes mirror of `of_execution_route_config_t`."""

    _fields_ = [
        ("route_id", c_char_p),
        ("account_id", c_char_p),
        ("venue", c_char_p),
        ("instrument", c_char_p),
        ("enabled", c_uint8),
        ("kill_switch", c_uint8),
        ("max_order_qty", c_int64),
        ("max_order_notional", c_int64),
        ("max_open_orders", c_uint32),
        ("max_open_notional", c_int64),
        ("price_band_ticks", c_int64),
    ]


class OfExecutionOrderRequest(ctypes.Structure):
    """ctypes mirror of `of_execution_order_request_t`."""

    _fields_ = [
        ("client_order_id", c_char_p),
        ("account_id", c_char_p),
        ("route_id", c_char_p),
        ("strategy_id", c_char_p),
        ("venue", c_char_p),
        ("instrument", c_char_p),
        ("side", c_uint32),
        ("order_type", c_uint32),
        ("time_in_force", c_uint32),
        ("quantity", c_int64),
        ("limit_price", c_int64),
        ("stop_price", c_int64),
        ("ts_exchange_ns", c_uint64),
        ("ts_recv_ns", c_uint64),
    ]


class OfExecutionCancelRequest(ctypes.Structure):
    """ctypes mirror of `of_execution_cancel_request_t`."""

    _fields_ = [
        ("client_order_id", c_char_p),
        ("orig_client_order_id", c_char_p),
        ("venue_order_id", c_char_p),
        ("account_id", c_char_p),
        ("route_id", c_char_p),
        ("venue", c_char_p),
        ("instrument", c_char_p),
        ("ts_recv_ns", c_uint64),
    ]


class OfExecutionAmendRequest(ctypes.Structure):
    """ctypes mirror of `of_execution_amend_request_t`."""

    _fields_ = [
        ("client_order_id", c_char_p),
        ("orig_client_order_id", c_char_p),
        ("venue_order_id", c_char_p),
        ("account_id", c_char_p),
        ("route_id", c_char_p),
        ("venue", c_char_p),
        ("instrument", c_char_p),
        ("quantity", c_int64),
        ("limit_price", c_int64),
        ("ts_recv_ns", c_uint64),
    ]


class OfExecutionEvent(ctypes.Structure):
    """ctypes mirror of `of_execution_event_t`."""

    _fields_ = [
        ("exec_type", c_uint32),
        ("order_status", c_uint32),
        ("client_order_id", ctypes.c_char * 41),
        ("orig_client_order_id", ctypes.c_char * 41),
        ("venue_order_id", ctypes.c_char * 49),
        ("execution_id", ctypes.c_char * 49),
        ("account_id", ctypes.c_char * 33),
        ("route_id", ctypes.c_char * 33),
        ("venue", ctypes.c_char * 17),
        ("instrument", ctypes.c_char * 33),
        ("last_qty", c_int64),
        ("last_price", c_int64),
        ("cumulative_qty", c_int64),
        ("leaves_qty", c_int64),
        ("average_price", c_int64),
        ("ts_exchange_ns", c_uint64),
        ("ts_recv_ns", c_uint64),
        ("reason", c_uint32),
        ("text", ctypes.c_char * 129),
    ]


class OfExecutionOrderState(ctypes.Structure):
    """ctypes mirror of `of_execution_order_state_t`."""

    _fields_ = [
        ("client_order_id", ctypes.c_char * 41),
        ("venue_order_id", ctypes.c_char * 49),
        ("account_id", ctypes.c_char * 33),
        ("route_id", ctypes.c_char * 33),
        ("venue", ctypes.c_char * 17),
        ("instrument", ctypes.c_char * 33),
        ("status", c_uint32),
        ("order_qty", c_int64),
        ("cumulative_qty", c_int64),
        ("leaves_qty", c_int64),
        ("average_price", c_int64),
        ("updated_ns", c_uint64),
    ]


class OfExecutionHealth(ctypes.Structure):
    """ctypes mirror of `of_execution_health_t`."""

    _fields_ = [
        ("connected", c_uint8),
        ("degraded", c_uint8),
        ("health_seq", c_uint64),
    ]


class OfExecutionMetrics(ctypes.Structure):
    """ctypes mirror of `of_execution_metrics_t`."""

    _fields_ = [
        ("submitted", c_uint64),
        ("cancelled", c_uint64),
        ("amended", c_uint64),
        ("events_applied", c_uint64),
        ("risk_rejected", c_uint64),
        ("adapter_errors", c_uint64),
        ("recovered", c_uint64),
    ]


class OfExecutionWalIntegrityReport(ctypes.Structure):
    """ctypes mirror of `of_execution_wal_integrity_report_t`."""

    _fields_ = [
        ("records", c_uint64),
        ("bytes", c_uint64),
        ("first_sequence", c_uint64),
        ("last_sequence", c_uint64),
        ("checksum_failures", c_uint64),
        ("sequence_failures", c_uint64),
        ("has_first_sequence", c_uint8),
        ("has_last_sequence", c_uint8),
        ("truncated_tail", c_uint8),
        ("valid", c_uint8),
    ]


class OfExecutionSegmentedWalIntegrityReport(ctypes.Structure):
    """ctypes mirror of `of_execution_segmented_wal_integrity_report_t`."""

    _fields_ = [
        ("segments", c_uint64),
        ("records", c_uint64),
        ("bytes", c_uint64),
        ("first_sequence", c_uint64),
        ("last_sequence", c_uint64),
        ("checksum_failures", c_uint64),
        ("sequence_failures", c_uint64),
        ("has_first_sequence", c_uint8),
        ("has_last_sequence", c_uint8),
        ("valid", c_uint8),
    ]


class OfExecutionCheckpointStoreIntegrityReport(ctypes.Structure):
    """ctypes mirror of `of_execution_checkpoint_store_integrity_report_t`."""

    _fields_ = [
        ("checkpoint_files", c_uint64),
        ("valid_checkpoints", c_uint64),
        ("invalid_checkpoints", c_uint64),
        ("bytes", c_uint64),
        ("latest_checkpoint_id", c_uint64),
        ("latest_last_applied_sequence", c_uint64),
        ("latest_created_ns", c_uint64),
        ("has_latest", c_uint8),
        ("valid", c_uint8),
    ]


class OfExecutionRecoveryConfig(ctypes.Structure):
    """ctypes mirror of `of_execution_recovery_config_t`."""

    _fields_ = [
        ("wal_root", c_char_p),
        ("checkpoint_root", c_char_p),
        ("require_checkpoint", c_uint8),
    ]


class OfExecutionConcurrentConfig(ctypes.Structure):
    """ctypes mirror of `of_execution_concurrent_config_t`."""

    _fields_ = [
        ("command_capacity", c_uint32),
        ("report_capacity", c_uint32),
        ("event_buffer_capacity", c_uint32),
    ]


class OfExecutionCommandReport(ctypes.Structure):
    """ctypes mirror of `of_execution_command_report_t`."""

    _fields_ = [
        ("sequence", c_uint64),
        ("kind", c_uint32),
        ("result_code", c_int32),
        ("event_count", c_uint32),
    ]


class OfExecutionTwapConfig(ctypes.Structure):
    """ctypes mirror of ``of_execution_twap_config_t``."""

    _fields_ = [
        ("parent_order_id", ctypes.c_char_p),
        ("account_id", ctypes.c_char_p),
        ("route_id", ctypes.c_char_p),
        ("strategy_id", ctypes.c_char_p),
        ("venue", ctypes.c_char_p),
        ("instrument", ctypes.c_char_p),
        ("side", ctypes.c_uint32),
        ("order_type", ctypes.c_uint32),
        ("time_in_force", ctypes.c_uint32),
        ("total_qty", ctypes.c_int64),
        ("limit_price", ctypes.c_int64),
        ("stop_price", ctypes.c_int64),
        ("start_ns", ctypes.c_uint64),
        ("end_ns", ctypes.c_uint64),
        ("min_clip", ctypes.c_int64),
        ("max_clip", ctypes.c_int64),
        ("participation_cap_bps", ctypes.c_uint16),
        ("slice_interval_ns", ctypes.c_uint64),
    ]


class OfExecutionAlgoChildPlan(ctypes.Structure):
    """ctypes mirror of ``of_execution_algo_child_plan_t``."""

    _fields_ = [
        ("child_order_id", ctypes.c_char * 41),
        ("parent_order_id", ctypes.c_char * 41),
        ("client_order_id", ctypes.c_char * 41),
        ("account_id", ctypes.c_char * 33),
        ("route_id", ctypes.c_char * 33),
        ("strategy_id", ctypes.c_char * 33),
        ("venue", ctypes.c_char * 17),
        ("instrument", ctypes.c_char * 33),
        ("side", ctypes.c_uint32),
        ("order_type", ctypes.c_uint32),
        ("time_in_force", ctypes.c_uint32),
        ("quantity", ctypes.c_int64),
        ("limit_price", ctypes.c_int64),
        ("stop_price", ctypes.c_int64),
        ("due_ns", ctypes.c_uint64),
        ("ts_recv_ns", ctypes.c_uint64),
        ("has_plan", ctypes.c_uint8),
    ]


class OfExecutionAlgoProgress(ctypes.Structure):
    """ctypes mirror of ``of_execution_algo_progress_t``."""

    _fields_ = [
        ("target_qty", ctypes.c_int64),
        ("released_qty", ctypes.c_int64),
        ("completed_qty", ctypes.c_int64),
        ("open_qty", ctypes.c_int64),
        ("rejected_children", ctypes.c_uint64),
        ("terminal_children", ctypes.c_uint64),
        ("has_pending_plan", ctypes.c_uint8),
    ]


class OfSignalConfigParameter(ctypes.Structure):
    """ctypes mirror of ``of_signal_config_parameter_t``."""

    _fields_ = [
        ("name", ctypes.c_char_p),
        ("kind", ctypes.c_uint32),
        ("integer_value", ctypes.c_int64),
        ("float_value", ctypes.c_double),
        ("boolean_value", ctypes.c_uint8),
        ("text_value", ctypes.c_char_p),
    ]


class OfSignalValidationConfig(ctypes.Structure):
    """ctypes mirror of ``of_signal_validation_config_t``."""

    _fields_ = [
        ("markout_horizon_events", ctypes.c_uint32),
        ("flat_price_threshold", ctypes.c_int64),
        ("min_confidence_bps", ctypes.c_uint16),
        ("store_samples", ctypes.c_uint8),
        ("check_monotonic_timestamps", ctypes.c_uint8),
    ]


class OfSignalValidationEvent(ctypes.Structure):
    """ctypes mirror of ``of_signal_validation_event_t``."""

    _fields_ = [
        ("delta", ctypes.c_int64),
        ("cumulative_delta", ctypes.c_int64),
        ("buy_volume", ctypes.c_int64),
        ("sell_volume", ctypes.c_int64),
        ("last_price", ctypes.c_int64),
        ("point_of_control", ctypes.c_int64),
        ("value_area_low", ctypes.c_int64),
        ("value_area_high", ctypes.c_int64),
        ("ts_exchange_ns", ctypes.c_uint64),
        ("has_ts_exchange_ns", ctypes.c_uint8),
    ]


def _library_filename() -> str:
    if sys.platform == "win32":
        return "of_ffi_c.dll"
    if sys.platform == "darwin":
        return "libof_ffi_c.dylib"
    return "libof_ffi_c.so"


def _workspace_debug_library_path() -> Path:
    return Path(__file__).resolve().parents[3] / "target" / "debug" / _library_filename()


def _package_native_library_path() -> Path:
    return Path(__file__).resolve().parent / "native" / _library_filename()


def _library_search_paths() -> list[Path]:
    env_path = os.environ.get("ORDERFLOW_LIBRARY_PATH", "").strip()
    if env_path:
        return [Path(env_path)]
    return [
        _package_native_library_path(),
        _workspace_debug_library_path(),
    ]


def default_library_path() -> Path:
    """Return the first available shared library path."""
    paths = _library_search_paths()
    for path in paths:
        if path.exists():
            return path
    return paths[-1]


class OrderflowLib:
    """Loaded C ABI symbols."""

    def __init__(self, library_path: Optional[str] = None) -> None:
        """Loads shared library and binds native symbols."""
        path = Path(library_path) if library_path else default_library_path()
        self.path = path
        if not path.exists():
            candidates = [path] if library_path else _library_search_paths()
            searched = ", ".join(str(candidate) for candidate in candidates)
            raise FileNotFoundError(
                "Orderflow shared library not found. "
                f"Searched: {searched}. Build with: cargo build -p of_ffi_c"
            )
        self.lib = ctypes.CDLL(str(path))
        self._bind_symbols()

    def _bind_symbols(self) -> None:
        """Binds all C ABI function signatures from generated declarations."""
        _bind_generated_symbols(self.lib, globals())
