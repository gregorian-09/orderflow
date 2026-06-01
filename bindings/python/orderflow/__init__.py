"""Python binding for the Orderflow runtime C ABI.

This package wraps the stable ``of_ffi_c`` interface using ``ctypes`` and exposes
an ergonomic, Pythonic API for:

- runtime lifecycle management (create/start/poll/stop)
- subscriptions and callback delivery
- external ingest (trade/book injection)
- snapshots (book, analytics, signal, metrics)
- feed-quality supervision (stale/sequence/reconnect policy)

Quick start::

    from orderflow import Engine, EngineConfig, Symbol, StreamKind

    with Engine(EngineConfig(instance_id="py-client")) as eng:
        sym = Symbol("CME", "ESM6", 10)
        eng.subscribe(sym, StreamKind.ANALYTICS)
        eng.poll_once()
        print(eng.analytics_snapshot(sym))

If the native shared library is not in the default location, set
``ORDERFLOW_LIBRARY_PATH`` or pass ``library_path=...`` to ``Engine``.
"""

from ._ffi import OfAnalyticsConfig
from .api import (
    AmendRequest,
    BookAction,
    CancelRequest,
    DataQualityFlags,
    Engine,
    EngineConfig,
    ExecutionEngine,
    ExecutionEvent,
    ExecutionHealth,
    ExecutionMetrics,
    ExecutionOrderState,
    ExecutionOrderType,
    ExecutionSide,
    ExecutionTimeInForce,
    ExternalFeedPolicy,
    OrderRequest,
    OrderflowArgError,
    OrderflowError,
    OrderflowRiskError,
    OrderflowStateError,
    RiskLimits,
    RouteConfig,
    Side,
    StreamKind,
    Symbol,
)

__all__ = [
    "AmendRequest",
    "BookAction",
    "CancelRequest",
    "DataQualityFlags",
    "Engine",
    "EngineConfig",
    "ExecutionEngine",
    "ExecutionEvent",
    "ExecutionHealth",
    "ExecutionMetrics",
    "ExecutionOrderState",
    "ExecutionOrderType",
    "ExecutionSide",
    "ExecutionTimeInForce",
    "ExternalFeedPolicy",
    "OfAnalyticsConfig",
    "OrderRequest",
    "OrderflowArgError",
    "OrderflowError",
    "OrderflowRiskError",
    "OrderflowStateError",
    "RiskLimits",
    "RouteConfig",
    "Side",
    "StreamKind",
    "Symbol",
]
