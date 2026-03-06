#!/usr/bin/env python3
"""Provider conformance harness for live/cert environments."""

from __future__ import annotations

import argparse
import json
import sys
import time
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Any, Dict, List

ROOT = Path(__file__).resolve().parents[1]
PY_BINDING = ROOT / "bindings" / "python"
if str(PY_BINDING) not in sys.path:
    sys.path.insert(0, str(PY_BINDING))

from orderflow import Engine, EngineConfig, StreamKind, Symbol  # type: ignore


@dataclass
class Report:
    ok: bool
    provider: str
    symbol: str
    duration_secs: int
    health_events: int
    analytics_events: int
    degraded_events: int
    last_error: str
    points: List[str]


def run(args: argparse.Namespace) -> Report:
    points: List[str] = []
    health_events = 0
    analytics_events = 0
    degraded_events = 0
    last_error = ""

    cfg = EngineConfig(instance_id=args.instance_id, config_path=args.config_path)
    symbol = Symbol(args.venue, args.symbol, depth_levels=args.depth)

    def on_health(ev: Dict[str, Any]) -> None:
        nonlocal health_events, degraded_events, last_error
        health_events += 1
        if ev.get("degraded"):
            degraded_events += 1
        if ev.get("last_error"):
            last_error = str(ev.get("last_error"))

    def on_analytics(_ev: Dict[str, Any]) -> None:
        nonlocal analytics_events
        analytics_events += 1

    start = time.time()
    with Engine(cfg) as engine:
        engine.subscribe(symbol, StreamKind.HEALTH, callback=on_health)
        engine.subscribe(symbol, StreamKind.ANALYTICS, callback=on_analytics)

        while time.time() - start < args.duration:
            engine.poll_once()
            time.sleep(0.05)

        metrics = engine.metrics()
        points.append(f"processed_events={metrics.get('processed_events', 0)}")
        points.append(f"adapter_connected={metrics.get('adapter_connected')}")
        points.append(f"adapter_protocol_info={metrics.get('adapter_protocol_info')}")

        ok = bool(metrics.get("adapter_connected")) and analytics_events > 0 and health_events > 0
        if not ok and not last_error:
            last_error = "conformance criteria not met (check provider credentials/connectivity)"

    return Report(
        ok=ok,
        provider=args.provider,
        symbol=f"{args.venue}:{args.symbol}",
        duration_secs=args.duration,
        health_events=health_events,
        analytics_events=analytics_events,
        degraded_events=degraded_events,
        last_error=last_error,
        points=points,
    )


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--provider", default="unknown")
    p.add_argument("--config-path", required=True)
    p.add_argument("--instance-id", default="provider-conformance")
    p.add_argument("--venue", default="CME")
    p.add_argument("--symbol", default="ESM6")
    p.add_argument("--depth", type=int, default=10)
    p.add_argument("--duration", type=int, default=20)
    return p.parse_args()


def main() -> None:
    args = parse_args()
    report = run(args)
    print(json.dumps(asdict(report), indent=2))
    if not report.ok:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
