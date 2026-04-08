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
from ctypes import c_char_p, c_int32, c_int64, c_uint16, c_uint32, c_uint64, c_uint8, c_void_p
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


def default_library_path() -> Path:
    """Return default shared library path relative to workspace root."""
    env_path = os.environ.get("ORDERFLOW_LIBRARY_PATH", "").strip()
    if env_path:
        return Path(env_path)

    suffix = {
        "linux": "so",
        "darwin": "dylib",
        "win32": "dll",
    }
    import sys

    ext = suffix.get(sys.platform, "so")
    root = Path(__file__).resolve().parents[3]
    if ext == "dll":
        return root / "target" / "debug" / "of_ffi_c.dll"
    return root / "target" / "debug" / f"libof_ffi_c.{ext}"


class OrderflowLib:
    """Loaded C ABI symbols."""

    def __init__(self, library_path: Optional[str] = None) -> None:
        """Loads shared library and binds native symbols."""
        path = Path(library_path) if library_path else default_library_path()
        self.path = path
        if not path.exists():
            raise FileNotFoundError(
                f"Orderflow shared library not found at '{path}'. Build with: cargo build -p of_ffi_c"
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

        lib.of_get_book_snapshot.argtypes = [
            c_void_p,
            ctypes.POINTER(OfSymbol),
            c_void_p,
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_book_snapshot.restype = c_int32

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

        lib.of_get_metrics_json.argtypes = [
            c_void_p,
            ctypes.POINTER(c_char_p),
            ctypes.POINTER(c_uint32),
        ]
        lib.of_get_metrics_json.restype = c_int32

        lib.of_string_free.argtypes = [c_char_p]
        lib.of_string_free.restype = None
