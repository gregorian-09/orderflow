#!/usr/bin/env python3
"""Smoke test for dashboard server endpoints and derived bar fields."""

from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional
from urllib.error import URLError
from urllib.request import urlopen


ROOT = Path(__file__).resolve().parents[1]
HOST = "127.0.0.1"
PORT = int(os.getenv("OF_DASH_TEST_PORT", "8095"))
BASE_URL = f"http://{HOST}:{PORT}"


def _http_json(path: str, timeout: float = 1.0) -> Dict[str, Any]:
    with urlopen(f"{BASE_URL}{path}", timeout=timeout) as resp:  # noqa: S310
        data = resp.read().decode("utf-8")
    return json.loads(data)


def _wait_json(path: str, deadline: float) -> Dict[str, Any]:
    last_exc: Optional[Exception] = None
    while time.time() < deadline:
        try:
            return _http_json(path)
        except (URLError, TimeoutError, OSError, json.JSONDecodeError) as exc:
            last_exc = exc
            time.sleep(0.2)
    raise RuntimeError(f"timeout waiting for {path}: {last_exc}")


def _first_bar_with_levels(deadline: float) -> Dict[str, Any]:
    last_state: Dict[str, Any] = {}
    while time.time() < deadline:
        state = _wait_json("/state", time.time() + 2.0)
        last_state = state
        bars = state.get("bars") or []
        for bar in bars:
            if (bar.get("levels") or []) and bar.get("volume", 0) > 0:
                return bar
        time.sleep(0.25)
    raise RuntimeError(f"no populated bars observed before deadline; last state keys={list(last_state.keys())}")


def _assert_session_shape(session: Dict[str, Any]) -> None:
    required = {
        "instance_id",
        "venue",
        "symbol",
        "depth_levels",
        "mode",
        "replay_loaded",
        "replay_file",
        "replay_index",
        "replay_total",
        "replay_paused",
    }
    missing = sorted(required.difference(session.keys()))
    if missing:
        raise AssertionError(f"/session missing keys: {missing}")


def _assert_bar_derived_fields(bar: Dict[str, Any]) -> None:
    required = {
        "buy_volume",
        "sell_volume",
        "imbalance_ask",
        "imbalance_bid",
        "stacked_ask",
        "stacked_bid",
    }
    missing = sorted(required.difference(bar.keys()))
    if missing:
        raise AssertionError(f"bar missing derived fields: {missing}")
    for key in required:
        value = bar[key]
        if not isinstance(value, (int, float)):
            raise AssertionError(f"bar field {key} expected numeric, got {type(value).__name__}")


def _start_server() -> subprocess.Popen[bytes]:
    env = os.environ.copy()
    env["OF_DASH_HOST"] = HOST
    env["OF_DASH_PORT"] = str(PORT)
    return subprocess.Popen(  # noqa: S603
        [sys.executable, "dashboard/server.py"],
        cwd=str(ROOT),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def _stop_server(proc: subprocess.Popen[bytes]) -> None:
    if proc.poll() is not None:
        return
    proc.send_signal(signal.SIGTERM)
    try:
        proc.wait(timeout=3)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=3)


def main() -> int:
    proc = _start_server()
    try:
        ready_by = time.time() + 15
        session = _wait_json("/session", ready_by)
        _assert_session_shape(session)
        bar = _first_bar_with_levels(time.time() + 20)
        _assert_bar_derived_fields(bar)
        print("dashboard smoke test: PASS")
        print(f"session.mode={session.get('mode')} symbol={session.get('venue')}:{session.get('symbol')}")
        fields = ["buy_volume", "sell_volume", "imbalance_ask", "imbalance_bid", "stacked_ask", "stacked_bid"]
        print("derived:", {k: bar.get(k) for k in fields})
        return 0
    finally:
        _stop_server(proc)


if __name__ == "__main__":
    raise SystemExit(main())
