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
        """Binds all C ABI function signatures for ctypes calls."""
        lib = self.lib

        lib.of_api_version.argtypes = []
        lib.of_api_version.restype = c_uint32

        lib.of_build_info.argtypes = []
        lib.of_build_info.restype = c_char_p

        lib.of_execution_api_version.argtypes = []
        lib.of_execution_api_version.restype = c_uint32

        lib.of_execution_wal_integrity_report.argtypes = [
            c_char_p,
            ctypes.POINTER(OfExecutionWalIntegrityReport),
        ]
        lib.of_execution_wal_integrity_report.restype = c_int32

        lib.of_execution_segmented_wal_integrity_report.argtypes = [
            c_char_p,
            ctypes.POINTER(OfExecutionSegmentedWalIntegrityReport),
        ]
        lib.of_execution_segmented_wal_integrity_report.restype = c_int32

        lib.of_execution_checkpoint_store_integrity_report.argtypes = [
            c_char_p,
            ctypes.POINTER(OfExecutionCheckpointStoreIntegrityReport),
        ]
        lib.of_execution_checkpoint_store_integrity_report.restype = c_int32

        lib.of_execution_engine_create.argtypes = [
            ctypes.POINTER(OfExecutionRouteConfig),
            ctypes.POINTER(c_void_p),
        ]
        lib.of_execution_engine_create.restype = c_int32

        lib.of_execution_engine_create_multi.argtypes = [
            ctypes.POINTER(OfExecutionRouteConfig),
            c_uint32,
            ctypes.POINTER(c_void_p),
        ]
        lib.of_execution_engine_create_multi.restype = c_int32

        lib.of_execution_engine_start.argtypes = [c_void_p]
        lib.of_execution_engine_start.restype = c_int32

        lib.of_execution_engine_stop.argtypes = [c_void_p]
        lib.of_execution_engine_stop.restype = c_int32

        lib.of_execution_engine_destroy.argtypes = [c_void_p]
        lib.of_execution_engine_destroy.restype = None

        lib.of_execution_submit_order.argtypes = [
            c_void_p,
            ctypes.POINTER(OfExecutionOrderRequest),
            ctypes.POINTER(OfExecutionEvent),
            ctypes.POINTER(c_uint32),
        ]
        lib.of_execution_submit_order.restype = c_int32

        lib.of_execution_cancel_order.argtypes = [
            c_void_p,
            ctypes.POINTER(OfExecutionCancelRequest),
            ctypes.POINTER(OfExecutionEvent),
            ctypes.POINTER(c_uint32),
        ]
        lib.of_execution_cancel_order.restype = c_int32

        lib.of_execution_amend_order.argtypes = [
            c_void_p,
            ctypes.POINTER(OfExecutionAmendRequest),
            ctypes.POINTER(OfExecutionEvent),
            ctypes.POINTER(c_uint32),
        ]
        lib.of_execution_amend_order.restype = c_int32

        lib.of_execution_poll.argtypes = [
            c_void_p,
            ctypes.POINTER(OfExecutionEvent),
            ctypes.POINTER(c_uint32),
        ]
        lib.of_execution_poll.restype = c_int32

        lib.of_execution_get_order_state.argtypes = [
            c_void_p,
            c_char_p,
            ctypes.POINTER(OfExecutionOrderState),
        ]
        lib.of_execution_get_order_state.restype = c_int32

        lib.of_execution_health.argtypes = [c_void_p, ctypes.POINTER(OfExecutionHealth)]
        lib.of_execution_health.restype = c_int32

        lib.of_execution_metrics.argtypes = [c_void_p, ctypes.POINTER(OfExecutionMetrics)]
        lib.of_execution_metrics.restype = c_int32

        lib.of_execution_concurrent_engine_create_multi.argtypes = [
            ctypes.POINTER(OfExecutionRouteConfig),
            c_uint32,
            ctypes.POINTER(OfExecutionConcurrentConfig),
            ctypes.POINTER(c_void_p),
        ]
        lib.of_execution_concurrent_engine_create_multi.restype = c_int32

        lib.of_execution_concurrent_engine_destroy.argtypes = [c_void_p]
        lib.of_execution_concurrent_engine_destroy.restype = None

        lib.of_execution_concurrent_stop.argtypes = [c_void_p, ctypes.POINTER(c_uint64)]
        lib.of_execution_concurrent_stop.restype = c_int32

        lib.of_execution_concurrent_submit_order.argtypes = [
            c_void_p,
            ctypes.POINTER(OfExecutionOrderRequest),
            ctypes.POINTER(c_uint64),
        ]
        lib.of_execution_concurrent_submit_order.restype = c_int32

        lib.of_execution_concurrent_cancel_order.argtypes = [
            c_void_p,
            ctypes.POINTER(OfExecutionCancelRequest),
            ctypes.POINTER(c_uint64),
        ]
        lib.of_execution_concurrent_cancel_order.restype = c_int32

        lib.of_execution_concurrent_amend_order.argtypes = [
            c_void_p,
            ctypes.POINTER(OfExecutionAmendRequest),
            ctypes.POINTER(c_uint64),
        ]
        lib.of_execution_concurrent_amend_order.restype = c_int32

        lib.of_execution_concurrent_poll.argtypes = [c_void_p, ctypes.POINTER(c_uint64)]
        lib.of_execution_concurrent_poll.restype = c_int32

        lib.of_execution_concurrent_try_recv_report.argtypes = [
            c_void_p,
            ctypes.POINTER(OfExecutionCommandReport),
            ctypes.POINTER(OfExecutionEvent),
            ctypes.POINTER(c_uint32),
        ]
        lib.of_execution_concurrent_try_recv_report.restype = c_int32

        lib.of_engine_create.argtypes = [ctypes.POINTER(OfEngineConfig), ctypes.POINTER(c_void_p)]
        lib.of_engine_create.restype = c_int32

        lib.of_engine_start.argtypes = [c_void_p]
        lib.of_engine_start.restype = c_int32

        lib.of_engine_stop.argtypes = [c_void_p]
        lib.of_engine_stop.restype = c_int32

        lib.of_engine_destroy.argtypes = [c_void_p]
        lib.of_engine_destroy.restype = None

        lib.of_subscribe.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_uint32,
            OfEventCallback,
            c_void_p,
            ctypes.POINTER(c_void_p),
        ]
        lib.of_subscribe.restype = c_int32

        lib.of_unsubscribe.argtypes = [c_void_p]
        lib.of_unsubscribe.restype = c_int32

        lib.of_unsubscribe_symbol.argtypes = [c_void_p, ctypes.POINTER(OfSymbol)]
        lib.of_unsubscribe_symbol.restype = c_int32

        lib.of_reset_symbol_session.argtypes = [c_void_p, ctypes.POINTER(OfSymbol)]
        lib.of_reset_symbol_session.restype = c_int32

        lib.of_ingest_trade.argtypes = [c_void_p, ctypes.POINTER(OfTrade), c_uint32]
        lib.of_ingest_trade.restype = c_int32

        lib.of_ingest_book.argtypes = [c_void_p, ctypes.POINTER(OfBook), c_uint32]
        lib.of_ingest_book.restype = c_int32

        lib.of_configure_external_feed.argtypes = [
            c_void_p,
            ctypes.POINTER(OfExternalFeedPolicy),
        ]
        lib.of_configure_external_feed.restype = c_int32

        lib.of_external_set_reconnecting.argtypes = [c_void_p, c_uint8]
        lib.of_external_set_reconnecting.restype = c_int32

        lib.of_external_health_tick.argtypes = [c_void_p]
        lib.of_external_health_tick.restype = c_int32

        lib.of_engine_poll_once.argtypes = [c_void_p, c_uint32]
        lib.of_engine_poll_once.restype = c_int32

        lib.of_engine_set_tickbar_interval.argtypes = [c_void_p, c_int64]
        lib.of_engine_set_tickbar_interval.restype = c_int32

        lib.of_get_book_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_book_snapshot.restype = c_int32

        lib.of_get_book_analytics_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_book_analytics_snapshot.restype = c_int32

        lib.of_compute_weighted_average_price.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            ctypes.c_int64,
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_compute_weighted_average_price.restype = c_int32

        lib.of_compute_depth_slope.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            ctypes.c_uint32,
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_compute_depth_slope.restype = c_int32

        lib.of_get_mid_price.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_mid_price.restype = c_int32

        lib.of_get_effective_spread_bps.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_effective_spread_bps.restype = c_int32

        lib.of_get_half_spread_cost_bps.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            ctypes.c_uint32,
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_half_spread_cost_bps.restype = c_int32

        lib.of_get_realised_spread_bps.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            ctypes.c_uint32,
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_realised_spread_bps.restype = c_int32

        lib.of_get_book_event_analytics.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            ctypes.c_uint64,
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_book_event_analytics.restype = c_int32

        lib.of_get_resiliency_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_resiliency_snapshot.restype = c_int32

        lib.of_get_vpin_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_vpin_snapshot.restype = c_int32

        lib.of_get_kyle_lambda_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_kyle_lambda_snapshot.restype = c_int32

        lib.of_get_amihud_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_amihud_snapshot.restype = c_int32

        lib.of_get_cvd_enhancement_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_cvd_enhancement_snapshot.restype = c_int32

        lib.of_get_pattern_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_pattern_snapshot.restype = c_int32

        # T3-T7 analytics
        t3_funcs = [
            "of_get_volatility_snapshot",
            "of_get_noise_snapshot",
            "of_get_hasbrouck_snapshot",
            "of_get_almgren_chriss_snapshot",
            "of_get_spread_decomp_snapshot",
            "of_get_acd_snapshot",
            "of_get_regime_snapshot",
            "of_get_kinetic_energy_snapshot",
            "of_get_dark_pool_snapshot",
            "of_get_options_flow_snapshot",
            "of_get_futures_snapshot",
            "of_get_vol_signature_snapshot",
            "of_get_agent_type_snapshot",
            "of_get_dark_lit_correlation_snapshot",
            "of_get_institutional_flow_snapshot",
            "of_get_oi_analysis_snapshot",
        ]
        for fname in t3_funcs:
            fn = getattr(lib, fname)
            fn.argtypes = [c_void_p, ctypes.POINTER(OfSymbol), c_void_p, ctypes.POINTER(c_uint32)]
            fn.restype = c_int32

        lib.of_get_analytics_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_analytics_snapshot.restype = c_int32

        lib.of_get_derived_analytics_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_derived_analytics_snapshot.restype = c_int32

        lib.of_get_session_candle_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_session_candle_snapshot.restype = c_int32

        lib.of_get_interval_candle_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_uint64,
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_interval_candle_snapshot.restype = c_int32

        lib.of_get_signal_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_signal_snapshot.restype = c_int32

        lib.of_get_bar_series.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_bar_series.restype = c_int32

        lib.of_get_metrics_json.argtypes = [
            c_void_p,
            ctypes.POINTER(c_char_p),
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_metrics_json.restype = c_int32

        lib.of_compute_lob_features.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_double,
            c_double,
            c_double,
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_compute_lob_features.restype = c_int32

        lib.of_engine_set_analytics_config.argtypes = [c_void_p, c_void_p]
        lib.of_engine_set_analytics_config.restype = c_int32

        lib.of_string_free.argtypes = [c_char_p]
        lib.of_string_free.restype = None
