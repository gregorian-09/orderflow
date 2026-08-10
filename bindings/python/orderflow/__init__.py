"""Python binding for the Orderflow runtime C ABI.

This package wraps the stable ``of_ffi_c`` interface using ``ctypes`` and exposes
an ergonomic, Pythonic API for:

- runtime lifecycle management (create/start/poll/stop)
- subscriptions and callback delivery
- external ingest (trade/book injection)
- snapshots (book, analytics, signal, metrics)
- feed-quality supervision (stale/sequence/reconnect policy)
- offline config-driven built-in signal replay validation

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
    AlgoChildPlan,
    AlgoProgress,
    BookAction,
    CancelRequest,
    ConcurrentExecutionConfig,
    ConcurrentExecutionEngine,
    DataQualityFlags,
    Engine,
    EngineConfig,
    ExecutionCheckpointStoreIntegrityReport,
    ExecutionEngine,
    ExecutionCommandReport,
    ExecutionEvent,
    ExecutionHealth,
    ExecutionMetrics,
    ExecutionOrderState,
    ExecutionRecoveryReplay,
    ExecutionRecoveryReport,
    ExecutionOrderType,
    ExecutionSegmentedWalIntegrityReport,
    ExecutionSide,
    ExecutionTimeInForce,
    ExecutionWalIntegrityReport,
    ExternalFeedPolicy,
    OrderRequest,
    OrderflowArgError,
    OrderflowError,
    OrderflowRiskError,
    OrderflowStateError,
    RiskLimits,
    RouteConfig,
    Side,
    SignalConfig,
    SignalConfigParameter,
    SignalValidationConfig,
    SignalValidationEvent,
    SignalValidationReport,
    StreamKind,
    Symbol,
    TwapConfig,
    TwapExecutionAlgo,
    adapter_inventory,
    available_adapters,
    inspect_execution_checkpoint_store,
    inspect_execution_recovery,
    inspect_execution_segmented_wal,
    inspect_execution_wal,
    signal_descriptors,
    validate_signal_config,
    validate_signal_replay,
)

__all__ = [
    "AmendRequest",
    "AlgoChildPlan",
    "AlgoProgress",
    "BookAction",
    "CancelRequest",
    "ConcurrentExecutionConfig",
    "ConcurrentExecutionEngine",
    "DataQualityFlags",
    "Engine",
    "EngineConfig",
    "ExecutionCheckpointStoreIntegrityReport",
    "ExecutionEngine",
    "ExecutionCommandReport",
    "ExecutionEvent",
    "ExecutionHealth",
    "ExecutionMetrics",
    "ExecutionOrderState",
    "ExecutionRecoveryReplay",
    "ExecutionRecoveryReport",
    "ExecutionOrderType",
    "ExecutionSegmentedWalIntegrityReport",
    "ExecutionSide",
    "ExecutionTimeInForce",
    "ExecutionWalIntegrityReport",
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
    "SignalConfig",
    "SignalConfigParameter",
    "SignalValidationConfig",
    "SignalValidationEvent",
    "SignalValidationReport",
    "StreamKind",
    "Symbol",
    "TwapConfig",
    "TwapExecutionAlgo",
    "adapter_inventory",
    "available_adapters",
    "inspect_execution_checkpoint_store",
    "inspect_execution_recovery",
    "inspect_execution_segmented_wal",
    "inspect_execution_wal",
    "signal_descriptors",
    "validate_signal_config",
    "validate_signal_replay",
]
