#!/usr/bin/env python3
"""Minimal end-to-end smoke check for the Python binding."""

from __future__ import annotations

import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "bindings" / "python"))

from orderflow import Engine, EngineConfig, Side, StreamKind, Symbol  # noqa: E402


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

    print("python binding smoke: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
