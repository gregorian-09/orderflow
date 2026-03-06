"""High-level Python API for Orderflow C ABI."""

from __future__ import annotations

import ctypes
import json
from dataclasses import dataclass
from typing import Any, Callable, Dict, Optional

from ._ffi import (
    OfBook,
    OfEngineConfig,
    OfEvent,
    OfEventCallback,
    OfExternalFeedPolicy,
    OfSymbol,
    OfTrade,
    OrderflowLib,
)


class StreamKind:
    """Stream kind identifiers used for subscriptions and callbacks."""

    BOOK = 1
    TRADES = 2
    ANALYTICS = 3
    SIGNALS = 4
    HEALTH = 5


class Side:
    """Side constants for trade/book payloads."""

    BID = 0
    ASK = 1


class BookAction:
    """Book action constants for book payloads."""

    UPSERT = 0
    DELETE = 1


class DataQualityFlags:
    """Bit flags describing feed quality constraints."""

    NONE = 0
    STALE_FEED = 1 << 0
    SEQUENCE_GAP = 1 << 1
    CLOCK_SKEW = 1 << 2
    DEPTH_TRUNCATED = 1 << 3
    OUT_OF_ORDER = 1 << 4
    ADAPTER_DEGRADED = 1 << 5


class OrderflowError(RuntimeError):
    """Base exception for Python binding errors."""

    pass


class OrderflowStateError(OrderflowError):
    """Raised when API calls are invalid for current engine state."""

    pass


class OrderflowArgError(OrderflowError):
    """Raised when invalid arguments are passed to C ABI calls."""

    pass


_ERROR_MAP = {
    0: None,
    1: OrderflowArgError,
    2: OrderflowStateError,
    3: OrderflowError,
    4: OrderflowError,
    5: OrderflowError,
    6: OrderflowError,
    255: OrderflowError,
}


@dataclass(frozen=True)
class Symbol:
    """Symbol descriptor used by subscriptions, snapshots, and ingest calls."""

    venue: str
    symbol: str
    depth_levels: int = 10


@dataclass(frozen=True)
class EngineConfig:
    """Runtime engine configuration passed to `of_engine_create`."""

    instance_id: str = "python"
    config_path: str = ""
    log_level: int = 0
    enable_persistence: bool = False
    audit_max_bytes: int = 10 * 1024 * 1024
    audit_max_files: int = 5
    audit_redact_tokens_csv: str = "secret,password,token,api_key"
    data_retention_max_bytes: int = 10 * 1024 * 1024
    data_retention_max_age_secs: int = 7 * 24 * 60 * 60


@dataclass(frozen=True)
class ExternalFeedPolicy:
    """External-feed supervision policy for stale/sequence checks."""

    stale_after_ms: int = 15_000
    enforce_sequence: bool = True


class Engine:
    """High-level engine wrapper around the Orderflow C ABI."""

    def __init__(self, config: EngineConfig, library_path: Optional[str] = None) -> None:
        """Creates an engine instance from config and optional shared library path."""
        self._ffi = OrderflowLib(library_path=library_path)
        self._engine = ctypes.c_void_p()
        self._subs: list[ctypes.c_void_p] = []
        self._callbacks: list[OfEventCallback] = []
        self._alive = False

        # Keep C string buffers alive for c_char_p fields passed into C.
        self._cfg_cstr: dict[str, ctypes.Array[ctypes.c_char]] = {}
        instance_id = self._make_c_string(config.instance_id, "instance_id")
        config_path = self._make_c_string(config.config_path, "config_path")
        redact_csv = self._make_c_string(config.audit_redact_tokens_csv, "audit_redact_tokens_csv")
        cfg = OfEngineConfig(
            instance_id=instance_id,
            config_path=config_path,
            log_level=ctypes.c_uint32(config.log_level),
            enable_persistence=ctypes.c_uint8(1 if config.enable_persistence else 0),
            audit_max_bytes=ctypes.c_uint64(config.audit_max_bytes),
            audit_max_files=ctypes.c_uint32(config.audit_max_files),
            audit_redact_tokens_csv=redact_csv,
            data_retention_max_bytes=ctypes.c_uint64(config.data_retention_max_bytes),
            data_retention_max_age_secs=ctypes.c_uint64(config.data_retention_max_age_secs),
        )
        rc = self._ffi.lib.of_engine_create(ctypes.byref(cfg), ctypes.byref(self._engine))
        self._check(rc, "of_engine_create")

    def __enter__(self) -> "Engine":
        """Context manager entry that starts the engine."""
        self.start()
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        """Context manager exit that closes the engine."""
        self.close()

    @property
    def api_version(self) -> int:
        """Returns ABI version reported by native library."""
        return int(self._ffi.lib.of_api_version())

    @property
    def build_info(self) -> str:
        """Returns native build info string."""
        ptr = self._ffi.lib.of_build_info()
        return ptr.decode("utf-8") if ptr else ""

    def start(self) -> None:
        """Starts engine adapter/session."""
        self._require_handle()
        rc = self._ffi.lib.of_engine_start(self._engine)
        self._check(rc, "of_engine_start")
        self._alive = True

    def stop(self) -> None:
        """Stops engine adapter/session."""
        if self._engine:
            rc = self._ffi.lib.of_engine_stop(self._engine)
            self._check(rc, "of_engine_stop")
            self._alive = False

    def close(self) -> None:
        """Unsubscribes callbacks and destroys native engine handle."""
        if self._engine:
            for sub in self._subs:
                self._ffi.lib.of_unsubscribe(sub)
            self._subs.clear()
            self._callbacks.clear()
            self._ffi.lib.of_engine_destroy(self._engine)
            self._engine = ctypes.c_void_p()
            self._alive = False

    def subscribe(
        self,
        symbol: Symbol,
        stream_kind: int = StreamKind.ANALYTICS,
        callback: Optional[Callable[[Dict[str, Any]], None]] = None,
    ) -> None:
        """Subscribes a symbol stream with optional callback delivery."""
        self._require_handle()
        sub = ctypes.c_void_p()
        c_symbol = self._to_c_symbol(symbol)
        cb_fn = None
        if callback is not None:
            cb_fn = self._make_callback(callback)
            self._callbacks.append(cb_fn)
        rc = self._ffi.lib.of_subscribe(
            self._engine,
            ctypes.byref(c_symbol),
            ctypes.c_uint32(stream_kind),
            cb_fn,
            None,
            ctypes.byref(sub),
        )
        self._check(rc, "of_subscribe")
        self._subs.append(sub)

    def poll_once(self, quality_flags: int = DataQualityFlags.NONE) -> None:
        """Polls adapter once and dispatches any events."""
        self._require_handle()
        rc = self._ffi.lib.of_engine_poll_once(self._engine, ctypes.c_uint32(quality_flags))
        self._check(rc, "of_engine_poll_once")

    def unsubscribe(self, symbol: Symbol) -> None:
        """Unsubscribes all streams for the given symbol."""
        self._require_handle()
        c_symbol = self._to_c_symbol(symbol)
        rc = self._ffi.lib.of_unsubscribe_symbol(self._engine, ctypes.byref(c_symbol))
        self._check(rc, "of_unsubscribe_symbol")

    def reset_symbol_session(self, symbol: Symbol) -> None:
        """Resets per-symbol analytics session state."""
        self._require_handle()
        c_symbol = self._to_c_symbol(symbol)
        rc = self._ffi.lib.of_reset_symbol_session(self._engine, ctypes.byref(c_symbol))
        self._check(rc, "of_reset_symbol_session")

    def configure_external_feed(self, policy: ExternalFeedPolicy) -> None:
        """Configures stale/sequence supervision for external ingest flow."""
        self._require_handle()
        c_policy = OfExternalFeedPolicy(
            stale_after_ms=policy.stale_after_ms,
            enforce_sequence=ctypes.c_uint8(1 if policy.enforce_sequence else 0),
        )
        rc = self._ffi.lib.of_configure_external_feed(self._engine, ctypes.byref(c_policy))
        self._check(rc, "of_configure_external_feed")

    def set_external_reconnecting(self, reconnecting: bool) -> None:
        """Marks external feed reconnecting/degraded status."""
        self._require_handle()
        rc = self._ffi.lib.of_external_set_reconnecting(
            self._engine, ctypes.c_uint8(1 if reconnecting else 0)
        )
        self._check(rc, "of_external_set_reconnecting")

    def external_health_tick(self) -> None:
        """Re-evaluates external-feed stale status without ingesting data."""
        self._require_handle()
        rc = self._ffi.lib.of_external_health_tick(self._engine)
        self._check(rc, "of_external_health_tick")

    def ingest_trade(
        self,
        symbol: Symbol,
        price: int,
        size: int,
        aggressor_side: int,
        sequence: int = 0,
        ts_exchange_ns: int = 0,
        ts_recv_ns: int = 0,
        quality_flags: int = DataQualityFlags.NONE,
    ) -> None:
        """Injects one external trade event into runtime processing."""
        self._require_handle()
        trade = OfTrade(
            symbol=self._to_c_symbol(symbol),
            price=price,
            size=size,
            aggressor_side=aggressor_side,
            sequence=sequence,
            ts_exchange_ns=ts_exchange_ns,
            ts_recv_ns=ts_recv_ns,
        )
        rc = self._ffi.lib.of_ingest_trade(
            self._engine,
            ctypes.byref(trade),
            ctypes.c_uint32(quality_flags),
        )
        self._check(rc, "of_ingest_trade")

    def ingest_book(
        self,
        symbol: Symbol,
        side: int,
        level: int,
        price: int,
        size: int,
        action: int = BookAction.UPSERT,
        sequence: int = 0,
        ts_exchange_ns: int = 0,
        ts_recv_ns: int = 0,
        quality_flags: int = DataQualityFlags.NONE,
    ) -> None:
        """Injects one external book event into runtime processing."""
        self._require_handle()
        book = OfBook(
            symbol=self._to_c_symbol(symbol),
            side=side,
            level=level,
            price=price,
            size=size,
            action=action,
            sequence=sequence,
            ts_exchange_ns=ts_exchange_ns,
            ts_recv_ns=ts_recv_ns,
        )
        rc = self._ffi.lib.of_ingest_book(
            self._engine,
            ctypes.byref(book),
            ctypes.c_uint32(quality_flags),
        )
        self._check(rc, "of_ingest_book")

    def book_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns current book snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_book_snapshot, symbol)

    def analytics_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns current analytics snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_analytics_snapshot, symbol)

    def signal_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns current signal snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_signal_snapshot, symbol)

    def metrics(self) -> Dict[str, Any]:
        """Returns engine metrics JSON decoded as dict."""
        self._require_handle()
        out = ctypes.c_char_p()
        out_len = ctypes.c_uint32(0)
        rc = self._ffi.lib.of_get_metrics_json(self._engine, ctypes.byref(out), ctypes.byref(out_len))
        self._check(rc, "of_get_metrics_json")
        try:
            raw = ctypes.string_at(out, out_len.value).decode("utf-8")
            return self._decode_json(raw)
        finally:
            self._ffi.lib.of_string_free(out)

    def _snapshot_call(self, fn, symbol: Symbol) -> Dict[str, Any]:
        self._require_handle()
        c_symbol = self._to_c_symbol(symbol)
        cap = ctypes.c_uint32(4096)
        buf = ctypes.create_string_buffer(cap.value)
        rc = fn(self._engine, ctypes.byref(c_symbol), buf, ctypes.byref(cap))
        self._check(rc, fn.__name__)
        raw = bytes(buf[: cap.value]).decode("utf-8")
        return self._decode_json(raw)

    def _to_c_symbol(self, symbol: Symbol) -> OfSymbol:
        return OfSymbol(
            venue=self._encode(symbol.venue),
            symbol=self._encode(symbol.symbol),
            depth_levels=ctypes.c_uint16(symbol.depth_levels),
        )

    @staticmethod
    def _encode(value: str) -> bytes:
        return value.encode("utf-8") if value else b""

    def _make_c_string(self, value: str, key: str) -> ctypes.c_char_p:
        if not value:
            return ctypes.c_char_p()
        buf = ctypes.create_string_buffer(value.encode("utf-8"))
        self._cfg_cstr[key] = buf
        return ctypes.cast(buf, ctypes.c_char_p)

    @staticmethod
    def _decode_json(raw: str) -> Dict[str, Any]:
        raw = raw.strip()
        if not raw:
            return {}
        return json.loads(raw)

    @staticmethod
    def _check(rc: int, fn_name: str) -> None:
        exc = _ERROR_MAP.get(int(rc), OrderflowError)
        if exc is None:
            return
        raise exc(f"{fn_name} failed with code {rc}")

    def _require_handle(self) -> None:
        if not self._engine:
            raise OrderflowStateError("engine is closed")

    def _make_callback(self, fn: Callable[[Dict[str, Any]], None]) -> OfEventCallback:
        def _cb(ev_ptr, _user_data) -> None:
            ev: OfEvent = ev_ptr.contents
            raw = "{}"
            if ev.payload and ev.payload_len > 0:
                raw = ctypes.string_at(ev.payload, ev.payload_len).decode("utf-8")
            fn(self._decode_json(raw))

        return OfEventCallback(_cb)
