#!/usr/bin/env python3
"""Minimal end-to-end smoke check for the Python binding."""

from __future__ import annotations

import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "bindings" / "python"))

from orderflow import (  # noqa: E402
    Engine,
    EngineConfig,
    ExecutionEngine,
    ExecutionOrderType,
    ExecutionSide,
    ExecutionTimeInForce,
    OrderRequest,
    RiskLimits,
    RouteConfig,
    Side,
    SignalConfig,
    SignalConfigParameter,
    SignalValidationConfig,
    SignalValidationEvent,
    StreamKind,
    Symbol,
    TwapConfig,
    TwapExecutionAlgo,
    validate_signal_config,
    validate_signal_replay,
)


def shared_library_path() -> Path:
    if sys.platform == "darwin":
        name = "libof_ffi_c.dylib"
    elif sys.platform.startswith("win"):
        name = "of_ffi_c.dll"
    else:
        name = "libof_ffi_c.so"
    return ROOT / "target" / "debug" / name


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> int:
    lib_path = shared_library_path()
    require(lib_path.exists(), f"native library missing: {lib_path}")

    symbol = Symbol("CME", "ESM6", depth_levels=10)
    callbacks: list[dict[str, object]] = []

    with Engine(
        EngineConfig(instance_id="python-binding-smoke"),
        library_path=str(lib_path),
    ) as engine:
        engine.subscribe(symbol, StreamKind.ANALYTICS, callback=lambda ev: callbacks.append(ev))
        engine.ingest_trade(
            symbol,
            price=505000,
            size=2,
            aggressor_side=Side.ASK,
            sequence=1,
            ts_exchange_ns=10,
            ts_recv_ns=11,
        )

        analytics = engine.analytics_snapshot(symbol)
        interval = engine.interval_candle_snapshot(symbol, 60)
        signal = engine.signal_snapshot(symbol)
        metrics = engine.metrics()

        require("delta" in analytics, "analytics snapshot missing delta")
        require(analytics.get("delta") == 2, "analytics snapshot delta mismatch")
        require(interval.get("window_ns") == 60, "interval candle snapshot window mismatch")
        require(interval.get("trade_count") == 1, "interval candle snapshot trade count mismatch")
        require("state" in signal, "signal snapshot missing state")
        require("started" in metrics and metrics["started"] is True, "metrics missing started=true")
        require(len(callbacks) > 0, "no callbacks observed in smoke run")
        require("delta" in callbacks[0], "analytics callback missing delta")

    route = RouteConfig(
        "SIM",
        "ACC",
        "SIM",
        "ES",
        True,
        RiskLimits(False, 100, 1_000_000, 10, 10_000_000, 0),
    )
    with ExecutionEngine(route, library_path=str(lib_path)) as execution:
        events = execution.submit_order(
            OrderRequest(
                "SMOKE-1",
                "ACC",
                "SIM",
                "SMOKE",
                "SIM",
                "ES",
                ExecutionSide.BUY,
                ExecutionOrderType.LIMIT,
                ExecutionTimeInForce.DAY,
                1,
                5_000,
            )
        )
        require(bool(events), "execution binding returned no events")
        require(
            execution.order_state("SMOKE-1").client_order_id == "SMOKE-1",
            "execution binding order-state mismatch",
        )

        with TwapExecutionAlgo(
            TwapConfig(
                "TWAP-PARENT",
                "ACC",
                "SIM",
                "TWAP",
                "SIM",
                "ES",
                ExecutionSide.BUY,
                ExecutionOrderType.LIMIT,
                ExecutionTimeInForce.DAY,
                100,
                5_000,
                1_000,
                11_000,
                10,
                25,
                2_000,
            ),
            library_path=str(lib_path),
        ) as twap:
            child = twap.plan(1_000, "TWAP-CHILD-1", "TWAP-ORDER-1", 1_001)
            require(child is not None, "TWAP binding returned no due child")
            child_events = execution.submit_order(child.request)
            twap.commit_pending()
            for event in child_events:
                twap.record_execution(
                    event.last_qty, event.leaves_qty, event.order_status
                )
            progress = twap.progress()
            require(progress.released_qty == 20, "TWAP released quantity mismatch")
            require(progress.completed_qty == 20, "TWAP completed quantity mismatch")

    signal_config = SignalConfig(
        "delta_momentum_v1",
        (SignalConfigParameter("threshold", 10),),
    )
    config_result = validate_signal_config(signal_config, library_path=str(lib_path))
    require(config_result.get("valid") is True, "signal config validation failed")
    validation_report = validate_signal_replay(
        signal_config,
        (
            SignalValidationEvent(delta=20, last_price=100, ts_exchange_ns=1),
            SignalValidationEvent(delta=-20, last_price=90, ts_exchange_ns=2),
            SignalValidationEvent(delta=-20, last_price=80, ts_exchange_ns=3),
        ),
        SignalValidationConfig(store_samples=True),
        library_path=str(lib_path),
    )
    require(validation_report.evaluated_events == 3, "signal validation event count mismatch")
    require(validation_report.directional_accuracy_bps == 5_000, "signal accuracy mismatch")
    require(len(validation_report.samples) == 2, "signal validation sample count mismatch")

    print("python binding smoke: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
