"""High-level Python API for the Orderflow C ABI.

This module is the main user-facing interface for Python applications. It
provides a thin abstraction over the native runtime while preserving predictable
behavior suitable for production and replay workflows.

Core objects:
- :class:`Engine`: lifecycle, subscriptions, polling, ingest, snapshots.
- :class:`EngineConfig`: runtime configuration passed at engine creation.
- :class:`Symbol`: normalized venue/symbol identifier.
- :class:`ExternalFeedPolicy`: stale and sequence supervision rules.

Typical flow:
1. Create :class:`Engine` with :class:`EngineConfig`.
2. Start the engine.
3. Subscribe one or more symbols.
4. Poll and/or ingest external events.
5. Read analytics/signal/metrics snapshots.
6. Stop/close the engine.
"""

from __future__ import annotations

import ctypes
import json
from dataclasses import dataclass
from typing import Any, Callable, Dict, Optional, Sequence

from ._ffi import (
    OfAnalyticsConfig,
    OfBook,
    OfEngineConfig,
    OfEvent,
    OfEventCallback,
    OfExecutionAmendRequest,
    OfExecutionCancelRequest,
    OfExecutionCommandReport,
    OfExecutionConcurrentConfig,
    OfExecutionEvent,
    OfExecutionHealth,
    OfExecutionMetrics,
    OfExecutionOrderRequest,
    OfExecutionOrderState,
    OfExecutionRouteConfig,
    OfExecutionSegmentedWalIntegrityReport,
    OfExecutionWalIntegrityReport,
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
    BOOK_SNAPSHOT = 6
    DERIVED_ANALYTICS = 7


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


class ExecutionSide:
    """Execution order side constants."""

    BUY = 1
    SELL = 2


class ExecutionOrderType:
    """Execution order type constants."""

    MARKET = 1
    LIMIT = 2
    STOP = 3
    STOP_LIMIT = 4


class ExecutionTimeInForce:
    """Execution time-in-force constants."""

    DAY = 1
    GTC = 2
    IOC = 3
    FOK = 4
    GTD = 5


class OrderflowError(RuntimeError):
    """Base exception for Python binding errors."""

    pass


class OrderflowStateError(OrderflowError):
    """Raised when API calls are invalid for current engine state."""

    pass


class OrderflowArgError(OrderflowError):
    """Raised when invalid arguments are passed to C ABI calls."""

    pass


class OrderflowRiskError(OrderflowError):
    """Raised when execution pre-trade risk rejects a command."""

    def __init__(self, message: str, events: list["ExecutionEvent"]) -> None:
        """Creates a risk exception carrying native rejection events."""
        super().__init__(message)
        self.events = events


_ERROR_MAP = {
    0: None,
    1: OrderflowArgError,
    2: OrderflowStateError,
    3: OrderflowError,
    4: OrderflowError,
    5: OrderflowError,
    6: OrderflowError,
    7: OrderflowRiskError,
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


@dataclass(frozen=True)
class RiskLimits:
    """Execution pre-trade risk limits."""

    kill_switch: bool = True
    max_order_qty: int = 0
    max_order_notional: int = 0
    max_open_orders: int = 0
    max_open_notional: int = 0
    price_band_ticks: int = 0


@dataclass(frozen=True)
class RouteConfig:
    """Execution route, account, symbol, and risk configuration."""

    route_id: str
    account_id: str
    venue: str
    instrument: str
    enabled: bool = True
    risk_limits: RiskLimits = RiskLimits()


@dataclass(frozen=True)
class OrderRequest:
    """Execution new-order request."""

    client_order_id: str
    account_id: str
    route_id: str
    strategy_id: str
    venue: str
    instrument: str
    side: int
    order_type: int
    time_in_force: int
    quantity: int
    limit_price: int = 0
    stop_price: int = 0
    ts_exchange_ns: int = 0
    ts_recv_ns: int = 0


@dataclass(frozen=True)
class CancelRequest:
    """Execution cancel request."""

    client_order_id: str
    orig_client_order_id: str
    venue_order_id: str
    account_id: str
    route_id: str
    venue: str
    instrument: str
    ts_recv_ns: int = 0


@dataclass(frozen=True)
class AmendRequest:
    """Execution amend/cancel-replace request."""

    client_order_id: str
    orig_client_order_id: str
    venue_order_id: str
    account_id: str
    route_id: str
    venue: str
    instrument: str
    quantity: int
    limit_price: int
    ts_recv_ns: int = 0


@dataclass(frozen=True)
class ExecutionEvent:
    """Execution event returned by the native execution engine."""

    exec_type: int
    order_status: int
    client_order_id: str
    orig_client_order_id: str
    venue_order_id: str
    execution_id: str
    account_id: str
    route_id: str
    venue: str
    instrument: str
    last_qty: int
    last_price: int
    cumulative_qty: int
    leaves_qty: int
    average_price: int
    ts_exchange_ns: int
    ts_recv_ns: int
    reason: int
    text: str


@dataclass(frozen=True)
class ExecutionOrderState:
    """Current native execution order state."""

    client_order_id: str
    venue_order_id: str
    account_id: str
    route_id: str
    venue: str
    instrument: str
    status: int
    order_qty: int
    cumulative_qty: int
    leaves_qty: int
    average_price: int
    updated_ns: int


@dataclass(frozen=True)
class ExecutionHealth:
    """Execution engine health snapshot."""

    connected: bool
    degraded: bool
    health_seq: int


@dataclass(frozen=True)
class ExecutionMetrics:
    """Execution engine metrics snapshot."""

    submitted: int
    cancelled: int
    amended: int
    events_applied: int
    risk_rejected: int
    adapter_errors: int
    recovered: int


@dataclass(frozen=True)
class ExecutionWalIntegrityReport:
    """Execution WAL integrity report for offline operator diagnostics."""

    records: int
    bytes: int
    first_sequence: Optional[int]
    last_sequence: Optional[int]
    checksum_failures: int
    sequence_failures: int
    truncated_tail: bool
    valid: bool


@dataclass(frozen=True)
class ExecutionSegmentedWalIntegrityReport:
    """Segmented execution WAL integrity report for offline diagnostics."""

    segments: int
    records: int
    bytes: int
    first_sequence: Optional[int]
    last_sequence: Optional[int]
    checksum_failures: int
    sequence_failures: int
    valid: bool


@dataclass(frozen=True)
class ConcurrentExecutionConfig:
    """Concurrent execution worker queue configuration."""

    command_capacity: int = 1024
    report_capacity: int = 1024
    event_buffer_capacity: int = 64


@dataclass(frozen=True)
class ExecutionCommandReport:
    """Concurrent execution command report."""

    sequence: int
    kind: int
    result_code: int
    event_count: int
    events: list[ExecutionEvent]


def inspect_execution_wal(
    path: str,
    library_path: Optional[str] = None,
) -> ExecutionWalIntegrityReport:
    """Inspects an execution WAL file without creating an execution engine.

    Args:
        path: UTF-8 filesystem path to a single execution WAL file.
        library_path: Optional explicit path to ``libof_ffi_c``.

    Returns:
        A typed integrity report with decoded record counts, byte position,
        sequence range, failure counters, and validity flags.
    """
    ffi = OrderflowLib(library_path=library_path)
    report = OfExecutionWalIntegrityReport()
    encoded_path = str(path).encode("utf-8")
    rc = ffi.lib.of_execution_wal_integrity_report(encoded_path, ctypes.byref(report))
    if int(rc) != 0:
        exc = _ERROR_MAP.get(int(rc), OrderflowError)
        raise exc(f"of_execution_wal_integrity_report failed with code {rc}")
    return ExecutionWalIntegrityReport(
        records=int(report.records),
        bytes=int(report.bytes),
        first_sequence=int(report.first_sequence) if report.has_first_sequence else None,
        last_sequence=int(report.last_sequence) if report.has_last_sequence else None,
        checksum_failures=int(report.checksum_failures),
        sequence_failures=int(report.sequence_failures),
        truncated_tail=bool(report.truncated_tail),
        valid=bool(report.valid),
    )


def inspect_execution_segmented_wal(
    root: str,
    library_path: Optional[str] = None,
) -> ExecutionSegmentedWalIntegrityReport:
    """Inspects a segmented execution WAL directory without opening it.

    Args:
        root: UTF-8 filesystem path to a segmented WAL root directory.
        library_path: Optional explicit path to ``libof_ffi_c``.

    Returns:
        A typed integrity report with segment count, decoded record counts,
        byte position, sequence range, failure counters, and validity flag.
    """
    ffi = OrderflowLib(library_path=library_path)
    report = OfExecutionSegmentedWalIntegrityReport()
    encoded_root = str(root).encode("utf-8")
    rc = ffi.lib.of_execution_segmented_wal_integrity_report(
        encoded_root, ctypes.byref(report)
    )
    if int(rc) != 0:
        exc = _ERROR_MAP.get(int(rc), OrderflowError)
        raise exc(f"of_execution_segmented_wal_integrity_report failed with code {rc}")
    return ExecutionSegmentedWalIntegrityReport(
        segments=int(report.segments),
        records=int(report.records),
        bytes=int(report.bytes),
        first_sequence=int(report.first_sequence) if report.has_first_sequence else None,
        last_sequence=int(report.last_sequence) if report.has_last_sequence else None,
        checksum_failures=int(report.checksum_failures),
        sequence_failures=int(report.sequence_failures),
        valid=bool(report.valid),
    )


class ExecutionEngine:
    """High-level Python wrapper around the native execution C ABI."""

    def __init__(
        self,
        route: RouteConfig | Sequence[RouteConfig],
        library_path: Optional[str] = None,
    ) -> None:
        """Creates a simulated execution engine for one or more configured routes."""
        self._ffi = OrderflowLib(library_path=library_path)
        self._engine = ctypes.c_void_p()
        if isinstance(route, RouteConfig):
            cfg = self._to_c_route(route)
            rc = self._ffi.lib.of_execution_engine_create(
                ctypes.byref(cfg), ctypes.byref(self._engine)
            )
            self._check(rc, "of_execution_engine_create", [])
        else:
            cfgs = self._to_c_routes(route)
            rc = self._ffi.lib.of_execution_engine_create_multi(
                cfgs, ctypes.c_uint32(len(cfgs)), ctypes.byref(self._engine)
            )
            self._check(rc, "of_execution_engine_create_multi", [])

    def __enter__(self) -> "ExecutionEngine":
        """Context manager entry that starts execution."""
        self.start()
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        """Context manager exit that closes execution."""
        self.close()

    @property
    def api_version(self) -> int:
        """Returns execution ABI version reported by native library."""
        return int(self._ffi.lib.of_execution_api_version())

    def start(self) -> None:
        """Starts execution adapter/session."""
        self._require_handle()
        rc = self._ffi.lib.of_execution_engine_start(self._engine)
        self._check(rc, "of_execution_engine_start", [])

    def stop(self) -> None:
        """Stops execution adapter/session."""
        if self._engine:
            rc = self._ffi.lib.of_execution_engine_stop(self._engine)
            self._check(rc, "of_execution_engine_stop", [])

    def close(self) -> None:
        """Destroys native execution engine handle."""
        if self._engine:
            self._ffi.lib.of_execution_engine_destroy(self._engine)
            self._engine = ctypes.c_void_p()

    def submit_order(self, request: OrderRequest) -> list[ExecutionEvent]:
        """Submits an order and returns generated execution events."""
        self._require_handle()
        req = self._to_c_order(request)
        events, length = self._event_array()
        rc = self._ffi.lib.of_execution_submit_order(
            self._engine, ctypes.byref(req), events, ctypes.byref(length)
        )
        decoded = self._decode_events(events, length.value)
        self._check(rc, "of_execution_submit_order", decoded)
        return decoded

    def cancel_order(self, request: CancelRequest) -> list[ExecutionEvent]:
        """Cancels an order and returns generated execution events."""
        self._require_handle()
        req = self._to_c_cancel(request)
        events, length = self._event_array()
        rc = self._ffi.lib.of_execution_cancel_order(
            self._engine, ctypes.byref(req), events, ctypes.byref(length)
        )
        decoded = self._decode_events(events, length.value)
        self._check(rc, "of_execution_cancel_order", decoded)
        return decoded

    def amend_order(self, request: AmendRequest) -> list[ExecutionEvent]:
        """Amends an order and returns generated execution events."""
        self._require_handle()
        req = self._to_c_amend(request)
        events, length = self._event_array()
        rc = self._ffi.lib.of_execution_amend_order(
            self._engine, ctypes.byref(req), events, ctypes.byref(length)
        )
        decoded = self._decode_events(events, length.value)
        self._check(rc, "of_execution_amend_order", decoded)
        return decoded

    def poll_execution(self) -> list[ExecutionEvent]:
        """Polls execution events into a Python list."""
        self._require_handle()
        events, length = self._event_array()
        rc = self._ffi.lib.of_execution_poll(self._engine, events, ctypes.byref(length))
        decoded = self._decode_events(events, length.value)
        self._check(rc, "of_execution_poll", decoded)
        return decoded

    def order_state(self, client_order_id: str) -> ExecutionOrderState:
        """Returns current order state for a client order id."""
        self._require_handle()
        state = OfExecutionOrderState()
        rc = self._ffi.lib.of_execution_get_order_state(
            self._engine, self._encode(client_order_id), ctypes.byref(state)
        )
        self._check(rc, "of_execution_get_order_state", [])
        return ExecutionOrderState(
            client_order_id=self._decode_c_array(state.client_order_id),
            venue_order_id=self._decode_c_array(state.venue_order_id),
            account_id=self._decode_c_array(state.account_id),
            route_id=self._decode_c_array(state.route_id),
            venue=self._decode_c_array(state.venue),
            instrument=self._decode_c_array(state.instrument),
            status=int(state.status),
            order_qty=int(state.order_qty),
            cumulative_qty=int(state.cumulative_qty),
            leaves_qty=int(state.leaves_qty),
            average_price=int(state.average_price),
            updated_ns=int(state.updated_ns),
        )

    def execution_health(self) -> ExecutionHealth:
        """Returns execution health."""
        self._require_handle()
        health = OfExecutionHealth()
        rc = self._ffi.lib.of_execution_health(self._engine, ctypes.byref(health))
        self._check(rc, "of_execution_health", [])
        return ExecutionHealth(
            connected=bool(health.connected),
            degraded=bool(health.degraded),
            health_seq=int(health.health_seq),
        )

    def execution_metrics(self) -> ExecutionMetrics:
        """Returns execution metrics."""
        self._require_handle()
        metrics = OfExecutionMetrics()
        rc = self._ffi.lib.of_execution_metrics(self._engine, ctypes.byref(metrics))
        self._check(rc, "of_execution_metrics", [])
        return ExecutionMetrics(
            submitted=int(metrics.submitted),
            cancelled=int(metrics.cancelled),
            amended=int(metrics.amended),
            events_applied=int(metrics.events_applied),
            risk_rejected=int(metrics.risk_rejected),
            adapter_errors=int(metrics.adapter_errors),
            recovered=int(metrics.recovered),
        )

    @staticmethod
    def _event_array() -> tuple[ctypes.Array[OfExecutionEvent], ctypes.c_uint32]:
        events = (OfExecutionEvent * 32)()
        return events, ctypes.c_uint32(32)

    @staticmethod
    def _encode(value: str) -> bytes:
        return value.encode("ascii") if value else b""

    @staticmethod
    def _decode_c_array(value) -> str:
        return bytes(value).split(b"\0", 1)[0].decode("ascii")

    def _decode_events(self, events, count: int) -> list[ExecutionEvent]:
        return [self._decode_event(events[idx]) for idx in range(count)]

    def _decode_event(self, event: OfExecutionEvent) -> ExecutionEvent:
        return ExecutionEvent(
            exec_type=int(event.exec_type),
            order_status=int(event.order_status),
            client_order_id=self._decode_c_array(event.client_order_id),
            orig_client_order_id=self._decode_c_array(event.orig_client_order_id),
            venue_order_id=self._decode_c_array(event.venue_order_id),
            execution_id=self._decode_c_array(event.execution_id),
            account_id=self._decode_c_array(event.account_id),
            route_id=self._decode_c_array(event.route_id),
            venue=self._decode_c_array(event.venue),
            instrument=self._decode_c_array(event.instrument),
            last_qty=int(event.last_qty),
            last_price=int(event.last_price),
            cumulative_qty=int(event.cumulative_qty),
            leaves_qty=int(event.leaves_qty),
            average_price=int(event.average_price),
            ts_exchange_ns=int(event.ts_exchange_ns),
            ts_recv_ns=int(event.ts_recv_ns),
            reason=int(event.reason),
            text=self._decode_c_array(event.text),
        )

    def _to_c_route(self, route: RouteConfig) -> OfExecutionRouteConfig:
        limits = route.risk_limits
        return OfExecutionRouteConfig(
            route_id=self._encode(route.route_id),
            account_id=self._encode(route.account_id),
            venue=self._encode(route.venue),
            instrument=self._encode(route.instrument),
            enabled=ctypes.c_uint8(1 if route.enabled else 0),
            kill_switch=ctypes.c_uint8(1 if limits.kill_switch else 0),
            max_order_qty=ctypes.c_int64(limits.max_order_qty),
            max_order_notional=ctypes.c_int64(limits.max_order_notional),
            max_open_orders=ctypes.c_uint32(limits.max_open_orders),
            max_open_notional=ctypes.c_int64(limits.max_open_notional),
            price_band_ticks=ctypes.c_int64(limits.price_band_ticks),
        )

    def _to_c_routes(
        self, routes: Sequence[RouteConfig]
    ) -> ctypes.Array[OfExecutionRouteConfig]:
        if not routes:
            raise OrderflowArgError("at least one execution route is required")
        cfgs = (OfExecutionRouteConfig * len(routes))()
        for idx, route in enumerate(routes):
            cfgs[idx] = self._to_c_route(route)
        return cfgs

    def _to_c_order(self, request: OrderRequest) -> OfExecutionOrderRequest:
        return OfExecutionOrderRequest(
            client_order_id=self._encode(request.client_order_id),
            account_id=self._encode(request.account_id),
            route_id=self._encode(request.route_id),
            strategy_id=self._encode(request.strategy_id),
            venue=self._encode(request.venue),
            instrument=self._encode(request.instrument),
            side=ctypes.c_uint32(request.side),
            order_type=ctypes.c_uint32(request.order_type),
            time_in_force=ctypes.c_uint32(request.time_in_force),
            quantity=ctypes.c_int64(request.quantity),
            limit_price=ctypes.c_int64(request.limit_price),
            stop_price=ctypes.c_int64(request.stop_price),
            ts_exchange_ns=ctypes.c_uint64(request.ts_exchange_ns),
            ts_recv_ns=ctypes.c_uint64(request.ts_recv_ns),
        )

    def _to_c_cancel(self, request: CancelRequest) -> OfExecutionCancelRequest:
        return OfExecutionCancelRequest(
            client_order_id=self._encode(request.client_order_id),
            orig_client_order_id=self._encode(request.orig_client_order_id),
            venue_order_id=self._encode(request.venue_order_id),
            account_id=self._encode(request.account_id),
            route_id=self._encode(request.route_id),
            venue=self._encode(request.venue),
            instrument=self._encode(request.instrument),
            ts_recv_ns=ctypes.c_uint64(request.ts_recv_ns),
        )

    def _to_c_amend(self, request: AmendRequest) -> OfExecutionAmendRequest:
        return OfExecutionAmendRequest(
            client_order_id=self._encode(request.client_order_id),
            orig_client_order_id=self._encode(request.orig_client_order_id),
            venue_order_id=self._encode(request.venue_order_id),
            account_id=self._encode(request.account_id),
            route_id=self._encode(request.route_id),
            venue=self._encode(request.venue),
            instrument=self._encode(request.instrument),
            quantity=ctypes.c_int64(request.quantity),
            limit_price=ctypes.c_int64(request.limit_price),
            ts_recv_ns=ctypes.c_uint64(request.ts_recv_ns),
        )

    @staticmethod
    def _check(rc: int, fn_name: str, events: list[ExecutionEvent]) -> None:
        if int(rc) == 0:
            return
        if int(rc) == 7:
            raise OrderflowRiskError(f"{fn_name} failed with code {rc}", events)
        exc = _ERROR_MAP.get(int(rc), OrderflowError)
        raise exc(f"{fn_name} failed with code {rc}")

    def _require_handle(self) -> None:
        if not self._engine:
            raise OrderflowStateError("execution engine is closed")


class ConcurrentExecutionEngine(ExecutionEngine):
    """Python wrapper around the concurrent native execution worker."""

    def __init__(
        self,
        routes: RouteConfig | Sequence[RouteConfig],
        config: ConcurrentExecutionConfig = ConcurrentExecutionConfig(),
        library_path: Optional[str] = None,
    ) -> None:
        """Creates and starts a concurrent simulated execution worker."""
        self._ffi = OrderflowLib(library_path=library_path)
        self._engine = ctypes.c_void_p()
        route_list: Sequence[RouteConfig] = [routes] if isinstance(routes, RouteConfig) else routes
        cfgs = self._to_c_routes(route_list)
        native_cfg = OfExecutionConcurrentConfig(
            command_capacity=ctypes.c_uint32(config.command_capacity),
            report_capacity=ctypes.c_uint32(config.report_capacity),
            event_buffer_capacity=ctypes.c_uint32(config.event_buffer_capacity),
        )
        rc = self._ffi.lib.of_execution_concurrent_engine_create_multi(
            cfgs, ctypes.c_uint32(len(cfgs)), ctypes.byref(native_cfg), ctypes.byref(self._engine)
        )
        self._check(rc, "of_execution_concurrent_engine_create_multi", [])

    def __enter__(self) -> "ConcurrentExecutionEngine":
        """Context manager entry."""
        return self

    def start(self) -> None:
        """Concurrent workers start during construction."""
        self._require_handle()

    def close(self) -> None:
        """Destroys native concurrent execution worker handle."""
        if self._engine:
            self._ffi.lib.of_execution_concurrent_engine_destroy(self._engine)
            self._engine = ctypes.c_void_p()

    def submit_order(self, request: OrderRequest) -> int:
        """Queues a submit command and returns its command sequence."""
        self._require_handle()
        req = self._to_c_order(request)
        sequence = ctypes.c_uint64(0)
        rc = self._ffi.lib.of_execution_concurrent_submit_order(
            self._engine, ctypes.byref(req), ctypes.byref(sequence)
        )
        self._check(rc, "of_execution_concurrent_submit_order", [])
        return int(sequence.value)

    def cancel_order(self, request: CancelRequest) -> int:
        """Queues a cancel command and returns its command sequence."""
        self._require_handle()
        req = self._to_c_cancel(request)
        sequence = ctypes.c_uint64(0)
        rc = self._ffi.lib.of_execution_concurrent_cancel_order(
            self._engine, ctypes.byref(req), ctypes.byref(sequence)
        )
        self._check(rc, "of_execution_concurrent_cancel_order", [])
        return int(sequence.value)

    def amend_order(self, request: AmendRequest) -> int:
        """Queues an amend command and returns its command sequence."""
        self._require_handle()
        req = self._to_c_amend(request)
        sequence = ctypes.c_uint64(0)
        rc = self._ffi.lib.of_execution_concurrent_amend_order(
            self._engine, ctypes.byref(req), ctypes.byref(sequence)
        )
        self._check(rc, "of_execution_concurrent_amend_order", [])
        return int(sequence.value)

    def poll_execution(self) -> int:
        """Queues a poll command and returns its command sequence."""
        self._require_handle()
        sequence = ctypes.c_uint64(0)
        rc = self._ffi.lib.of_execution_concurrent_poll(
            self._engine, ctypes.byref(sequence)
        )
        self._check(rc, "of_execution_concurrent_poll", [])
        return int(sequence.value)

    def stop(self) -> int:
        """Queues worker stop and returns its command sequence."""
        self._require_handle()
        sequence = ctypes.c_uint64(0)
        rc = self._ffi.lib.of_execution_concurrent_stop(
            self._engine, ctypes.byref(sequence)
        )
        self._check(rc, "of_execution_concurrent_stop", [])
        return int(sequence.value)

    def try_recv_report(self) -> Optional[ExecutionCommandReport]:
        """Attempts to receive one command report without blocking."""
        self._require_handle()
        report = OfExecutionCommandReport()
        events, length = self._event_array()
        rc = self._ffi.lib.of_execution_concurrent_try_recv_report(
            self._engine, ctypes.byref(report), events, ctypes.byref(length)
        )
        if int(rc) == 5:
            return None
        decoded = self._decode_events(events, length.value)
        self._check(rc, "of_execution_concurrent_try_recv_report", decoded)
        return ExecutionCommandReport(
            sequence=int(report.sequence),
            kind=int(report.kind),
            result_code=int(report.result_code),
            event_count=int(report.event_count),
            events=decoded,
        )


class Engine:
    """High-level engine wrapper around the Orderflow C ABI.

    The engine controls the runtime session and acts as the single access point
    for subscriptions, event ingestion, and snapshots.

    Notes:
    - Use as a context manager for deterministic start/stop behavior.
    - Callbacks are dispatched during ``poll_once`` and external ``ingest_*``.
    - Snapshot methods return decoded ``dict`` objects from runtime JSON.
    """

    def __init__(self, config: EngineConfig, library_path: Optional[str] = None) -> None:
        """Creates an engine instance.

        Args:
            config: Runtime configuration values.
            library_path: Optional explicit path to ``libof_ffi_c`` shared library.
                When omitted, default lookup rules from ``orderflow._ffi`` apply.
        """
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
        # `of_subscribe` callback arg is typed as `OfEventCallback`; pass a typed
        # null function pointer when callback delivery is not requested.
        cb_fn: OfEventCallback = ctypes.cast(None, OfEventCallback)
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

    def set_tickbar_interval(self, interval_ns: int) -> None:
        """Configures tickbar aggregation interval for new per-symbol accumulators.

        Args:
            interval_ns: Aggregation interval in nanoseconds. Pass 0 or negative
                to disable tickbar for future accumulators.

        Requires the native library to be built with the ``tickbar`` feature.
        """
        self._require_handle()
        rc = self._ffi.lib.of_engine_set_tickbar_interval(
            self._engine, ctypes.c_int64(interval_ns)
        )
        self._check(rc, "of_engine_set_tickbar_interval")

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
        """Returns current book snapshot decoded as a dict with bids/asks and timestamps."""
        return self._snapshot_call(self._ffi.lib.of_get_book_snapshot, symbol)

    def book_analytics_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns current book analytics snapshot with spread, depth, imbalance, microprice."""
        return self._snapshot_call(self._ffi.lib.of_get_book_analytics_snapshot, symbol)

    def weighted_average_price(self, symbol: Symbol, qty: int) -> Optional[Dict[str, Any]]:
        """Computes weighted average price for an order of `qty` by walking the book.

        Positive qty = buy (walks asks), negative qty = sell (walks bids).
        Returns dict with 'price' key, or empty dict if insufficient liquidity.
        """
        return self._snapshot_call(
            self._ffi.lib.of_compute_weighted_average_price,
            symbol,
            ctypes.c_int64(qty),
        )

    def depth_slope(self, symbol: Symbol, levels: int = 5) -> Dict[str, Any]:
        """Computes depth slope (avg volume decay per level) over first `levels` levels.

        Returns dict with 'slope' key. Returns {'slope': 0.0} if fewer than 2 levels.
        """
        return self._snapshot_call(
            self._ffi.lib.of_compute_depth_slope,
            symbol,
            ctypes.c_uint32(levels),
        )

    def mid_price(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns mid price as dict with 'mid' key, or empty dict if no book data."""
        return self._snapshot_call(self._ffi.lib.of_get_mid_price, symbol)

    def effective_spread_bps(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns last effective spread in bps as dict with 'bps' key."""
        return self._snapshot_call(self._ffi.lib.of_get_effective_spread_bps, symbol)

    def half_spread_cost_bps(self, symbol: Symbol, window: int = 10) -> Dict[str, Any]:
        """Returns average half-spread cost in bps over last `window` trades."""
        return self._snapshot_call(
            self._ffi.lib.of_get_half_spread_cost_bps,
            symbol,
            ctypes.c_uint32(window),
        )

    def realised_spread_bps(self, symbol: Symbol, hold_ticks: int = 5) -> Dict[str, Any]:
        """Returns realised spread in bps for trade `hold_ticks` ago."""
        return self._snapshot_call(
            self._ffi.lib.of_get_realised_spread_bps,
            symbol,
            ctypes.c_uint32(hold_ticks),
        )

    def book_event_analytics(self, symbol: Symbol, window_ns: int = 1_000_000_000) -> Dict[str, Any]:
        """Returns book-event analytics snapshot (rates, volumes, intensity)."""
        return self._snapshot_call(
            self._ffi.lib.of_get_book_event_analytics,
            symbol,
            ctypes.c_uint64(window_ns),
        )

    def resiliency_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns resiliency snapshot (recovery time, depth elasticity)."""
        return self._snapshot_call(self._ffi.lib.of_get_resiliency_snapshot, symbol)

    def vpin_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns VPIN snapshot (vpin, z-score, mean, std, toxicity)."""
        return self._snapshot_call(self._ffi.lib.of_get_vpin_snapshot, symbol)

    def kyle_lambda_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns Kyle's Lambda snapshot (lambda, R², avg lambda)."""
        return self._snapshot_call(self._ffi.lib.of_get_kyle_lambda_snapshot, symbol)

    def amihud_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns Amihud illiquidity snapshot."""
        return self._snapshot_call(self._ffi.lib.of_get_amihud_snapshot, symbol)

    def cvd_enhancement_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """CVD enhancement analytics (delta ratio, z-score, divergence)."""
        return self._snapshot_call(self._ffi.lib.of_get_cvd_enhancement_snapshot, symbol)

    def pattern_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Pattern detection snapshot (imbalance, iceberg, hidden accumulation/distribution, session type)."""
        return self._snapshot_call(self._ffi.lib.of_get_pattern_snapshot, symbol)

    def volatility_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns realised volatility estimator snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_volatility_snapshot, symbol)

    def noise_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns microstructure noise snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_noise_snapshot, symbol)

    def hasbrouck_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns Hasbrouck impact snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_hasbrouck_snapshot, symbol)

    def almgren_chriss_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns Almgren-Chriss impact snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_almgren_chriss_snapshot, symbol)

    def spread_decomp_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns spread decomposition snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_spread_decomp_snapshot, symbol)

    def acd_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns ACD duration-model snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_acd_snapshot, symbol)

    def regime_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns regime detection snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_regime_snapshot, symbol)

    def kinetic_energy_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns order-book kinetic-energy snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_kinetic_energy_snapshot, symbol)

    def dark_pool_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns dark-pool analytics snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_dark_pool_snapshot, symbol)

    def options_flow_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns options-flow analytics snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_options_flow_snapshot, symbol)

    def futures_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns futures basis and roll snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_futures_snapshot, symbol)

    def vol_signature_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns volatility signature snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_vol_signature_snapshot, symbol)

    def agent_type_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns agent type identification snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_agent_type_snapshot, symbol)

    def dark_lit_correlation_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns dark-lit correlation snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_dark_lit_correlation_snapshot, symbol)

    def institutional_flow_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns institutional flow snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_institutional_flow_snapshot, symbol)

    def oi_analysis_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns OI analysis snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_oi_analysis_snapshot, symbol)

    def lob_features(self, symbol: Symbol, trade_imbalance: float = 0.0, cancel_rate: float = 0.0, arrival_rate: float = 0.0) -> Dict[str, Any]:
        """Computes LOB feature snapshot from engine book state and flow metrics."""
        return self._snapshot_call(self._ffi.lib.of_compute_lob_features, symbol, trade_imbalance, cancel_rate, arrival_rate)

    def set_analytics_config(self, config: Optional["OfAnalyticsConfig"] = None) -> None:
        """Override analytics thresholds and buffer sizes. None resets to defaults."""
        ptr = ctypes.byref(config) if config is not None else None
        rc = self._ffi.lib.of_engine_set_analytics_config(self._handle, ptr)
        if rc != 0:
            raise RuntimeError(f"set_analytics_config failed with error code {rc}")

    def analytics_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns current analytics snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_analytics_snapshot, symbol)

    def derived_analytics_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns current derived analytics snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_derived_analytics_snapshot, symbol)

    def session_candle_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns current session candle snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_session_candle_snapshot, symbol)

    def interval_candle_snapshot(self, symbol: Symbol, window_ns: int) -> Dict[str, Any]:
        """Returns rolling interval candle snapshot JSON decoded as dict."""
        return self._snapshot_call(
            self._ffi.lib.of_get_interval_candle_snapshot,
            symbol,
            ctypes.c_uint64(window_ns),
        )

    def signal_snapshot(self, symbol: Symbol) -> Dict[str, Any]:
        """Returns current signal snapshot JSON decoded as dict."""
        return self._snapshot_call(self._ffi.lib.of_get_signal_snapshot, symbol)

    def bar_series(self, symbol: Symbol) -> Any:
        """Returns completed bar series JSON decoded as list of dicts.

        Requires the native library to be built with the ``tickbar`` feature.
        Returns an empty list when tickbar aggregation is not configured for the symbol.
        """
        return self._snapshot_call(self._ffi.lib.of_get_bar_series, symbol)

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

    def _snapshot_call(self, fn, symbol: Symbol, *extra_args) -> Dict[str, Any]:
        self._require_handle()
        c_symbol = self._to_c_symbol(symbol)
        cap = ctypes.c_uint32(4096)
        for _ in range(3):
            buf = ctypes.create_string_buffer(cap.value)
            rc = fn(self._engine, ctypes.byref(c_symbol), *extra_args, buf, ctypes.byref(cap))
            if rc == 0:
                raw = bytes(buf[: cap.value]).decode("utf-8")
                return self._decode_json(raw)
            if rc != 1 or cap.value <= len(buf):
                self._check(rc, fn.__name__)
            cap = ctypes.c_uint32(cap.value)

        self._check(1, fn.__name__)
        return {}

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
