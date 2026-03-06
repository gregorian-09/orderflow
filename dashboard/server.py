#!/usr/bin/env python3
"""Live dashboard server for orderflow visuals (SSE + static UI)."""

from __future__ import annotations

import json
import os
import sys
import threading
import time
from dataclasses import dataclass, field
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any, Dict, List, Optional

ROOT = Path(__file__).resolve().parents[1]
PY_BINDING = ROOT / "bindings" / "python"
if str(PY_BINDING) not in sys.path:
    sys.path.insert(0, str(PY_BINDING))

from orderflow import DataQualityFlags, Engine, EngineConfig, StreamKind, Symbol  # type: ignore


@dataclass
class DashboardState:
    lock: threading.Lock = field(default_factory=threading.Lock)
    seq: int = 0
    running: bool = True
    analytics: Dict[str, Any] = field(default_factory=dict)
    signal: Dict[str, Any] = field(default_factory=dict)
    health: Dict[str, Any] = field(default_factory=dict)
    metrics: Dict[str, Any] = field(default_factory=dict)
    history: List[Dict[str, float]] = field(default_factory=list)
    bars: List[Dict[str, Any]] = field(default_factory=list)
    simulated: bool = False
    reset_requested: bool = False
    mode: str = "live"  # live | replay
    replay_loaded: bool = False
    replay_paused: bool = True
    replay_speed: float = 1.0
    replay_index: int = 0
    replay_events: List[Dict[str, Any]] = field(default_factory=list)
    replay_step_requested: bool = False
    replay_reload_requested: bool = False
    replay_file: str = ""
    replay_seek_target: Optional[int] = None

    def update(self, **kwargs: Any) -> None:
        with self.lock:
            for k, v in kwargs.items():
                setattr(self, k, v)
            self.seq += 1

    def snapshot(self) -> Dict[str, Any]:
        with self.lock:
            replay_elapsed_secs, replay_total_secs = _replay_elapsed_for_index(
                self.replay_events, self.replay_index
            )
            return {
                "seq": self.seq,
                "analytics": self.analytics,
                "signal": self.signal,
                "health": self.health,
                "metrics": self.metrics,
                "history": list(self.history),
                "bars": list(self.bars),
                "simulated": self.simulated,
                "mode": self.mode,
                "replay_loaded": self.replay_loaded,
                "replay_paused": self.replay_paused,
                "replay_speed": self.replay_speed,
                "replay_index": self.replay_index,
                "replay_total": len(self.replay_events),
                "replay_file": self.replay_file,
                "replay_elapsed_secs": replay_elapsed_secs,
                "replay_total_secs": replay_total_secs,
                "ts": time.time(),
            }


STATE = DashboardState()


def _level_summary(levels: List[Dict[str, Any]]) -> Dict[str, int]:
    buy_volume = 0
    sell_volume = 0
    imbalance_ask = 0
    imbalance_bid = 0
    stacked_ask = 0
    stacked_bid = 0
    ask_run = 0
    bid_run = 0
    for idx, row in enumerate(levels):
        ask = max(0, int(row.get("ask", 0) or 0))
        bid = max(0, int(row.get("bid", 0) or 0))
        buy_volume += ask
        sell_volume += bid
        below_bid = max(0, int(levels[idx + 1].get("bid", 0) or 0)) if idx + 1 < len(levels) else 0
        above_ask = max(0, int(levels[idx - 1].get("ask", 0) or 0)) if idx > 0 else 0
        ask_imbalance = ask > 0 and (below_bid == 0 or ask >= below_bid * 3)
        bid_imbalance = bid > 0 and (above_ask == 0 or bid >= above_ask * 3)
        if ask_imbalance:
            imbalance_ask += 1
            ask_run += 1
        else:
            ask_run = 0
        if bid_imbalance:
            imbalance_bid += 1
            bid_run += 1
        else:
            bid_run = 0
        stacked_ask = max(stacked_ask, ask_run)
        stacked_bid = max(stacked_bid, bid_run)
    return {
        "buy_volume": buy_volume,
        "sell_volume": sell_volume,
        "imbalance_ask": imbalance_ask,
        "imbalance_bid": imbalance_bid,
        "stacked_ask": stacked_ask,
        "stacked_bid": stacked_bid,
    }


def _reset_state_buffers(*, replay_mode: bool, replay_index: Optional[int] = None) -> None:
    with STATE.lock:
        STATE.history.clear()
        STATE.bars.clear()
        if replay_mode and replay_index is not None:
            STATE.replay_index = replay_index
        STATE.seq += 1


class DashboardController:
    def snapshot(self) -> Dict[str, Any]:
        return STATE.snapshot()

    def request_reset(self) -> None:
        with STATE.lock:
            STATE.reset_requested = True
            STATE.seq += 1

    def set_mode(self, mode: str) -> bool:
        normalized = str(mode).strip().lower()
        if normalized not in ("live", "replay"):
            return False
        STATE.update(mode=normalized)
        return True

    def replay_control(self, payload: Dict[str, Any]) -> None:
        updates: Dict[str, Any] = {}
        action = str(payload.get("action", "")).strip().lower()
        if "speed" in payload:
            try:
                updates["replay_speed"] = float(payload["speed"])
            except Exception:
                pass
        if action == "play":
            updates["replay_paused"] = False
        elif action == "pause":
            updates["replay_paused"] = True
        elif action == "step":
            updates["replay_step_requested"] = True
            updates["replay_paused"] = True
        elif action == "reload":
            if "file" in payload and str(payload["file"]).strip():
                updates["replay_file"] = str(payload["file"]).strip()
            updates["replay_reload_requested"] = True
        elif action == "seek":
            if "index" in payload:
                try:
                    updates["replay_seek_target"] = int(payload["index"])
                except Exception:
                    pass
            updates["replay_paused"] = True
        elif action == "seek_time":
            try:
                seconds = float(payload.get("seconds", 0))
            except Exception:
                seconds = 0.0
            with STATE.lock:
                events = list(STATE.replay_events)
            updates["replay_seek_target"] = _seek_index_by_elapsed(events, seconds)
            updates["replay_paused"] = True
        elif action == "reset":
            updates["reset_requested"] = True
        if updates:
            STATE.update(**updates)

    def session_metadata(self) -> Dict[str, Any]:
        snap = STATE.snapshot()
        metrics = snap.get("metrics", {})
        return {
            "instance_id": metrics.get("instance_id") or os.getenv("OF_DASH_INSTANCE_ID", "orderflow-dashboard"),
            "venue": os.getenv("OF_DASH_VENUE", "CME"),
            "symbol": os.getenv("OF_DASH_SYMBOL", "ESM6"),
            "depth_levels": int(os.getenv("OF_DASH_DEPTH", "10")),
            "mode": snap.get("mode", "live"),
            "replay_loaded": snap.get("replay_loaded", False),
            "replay_file": snap.get("replay_file", ""),
            "replay_index": snap.get("replay_index", 0),
            "replay_total": snap.get("replay_total", 0),
            "replay_paused": snap.get("replay_paused", True),
        }


CONTROLLER = DashboardController()


class ReplayAccumulator:
    def __init__(self, threshold: int = 100) -> None:
        self.threshold = threshold
        self.delta = 0
        self.cumulative_delta = 0
        self.buy_volume = 0
        self.sell_volume = 0
        self.last_price = 0
        self.profile: Dict[int, int] = {}
        self.processed = 0

    def apply_trade(self, price: int, size: int, aggressor: str) -> None:
        self.last_price = price
        self.profile[price] = self.profile.get(price, 0) + size
        if aggressor.lower() == "ask":
            self.buy_volume += size
            self.delta += size
            self.cumulative_delta += size
        else:
            self.sell_volume += size
            self.delta -= size
            self.cumulative_delta -= size
        self.processed += 1

    def snapshot(self) -> Dict[str, Any]:
        poc, val, vah = self._profile_levels()
        return {
            "delta": self.delta,
            "cumulative_delta": self.cumulative_delta,
            "buy_volume": self.buy_volume,
            "sell_volume": self.sell_volume,
            "last_price": self.last_price,
            "point_of_control": poc,
            "value_area_low": val,
            "value_area_high": vah,
        }

    def signal(self) -> Dict[str, Any]:
        if self.delta >= self.threshold:
            state, reason = "long_bias", "delta_above_threshold"
        elif self.delta <= -self.threshold:
            state, reason = "short_bias", "delta_below_threshold"
        else:
            state, reason = "neutral", "delta_inside_band"
        return {
            "module": "delta_momentum_v1",
            "state": state,
            "confidence_bps": 500,
            "quality_flags": 0,
            "reason": reason,
        }

    def _profile_levels(self) -> tuple[int, int, int]:
        if not self.profile:
            return 0, 0, 0
        prices = sorted(self.profile.keys())
        total = sum(self.profile.values())
        poc = max(prices, key=lambda p: (self.profile[p], p))
        target = int(total * 0.70 + 0.999)
        covered = self.profile[poc]
        low = high = poc
        i = prices.index(poc)
        left = i - 1
        right = i + 1
        while covered < target and (left >= 0 or right < len(prices)):
            lv = self.profile[prices[left]] if left >= 0 else -1
            rv = self.profile[prices[right]] if right < len(prices) else -1
            if rv > lv:
                covered += max(0, rv)
                high = prices[right]
                right += 1
            else:
                covered += max(0, lv)
                low = prices[left]
                left -= 1
        return poc, low, high


def _load_replay_events(path: Path) -> List[Dict[str, Any]]:
    events: List[Dict[str, Any]] = []
    if not path.exists():
        return events
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                ev = json.loads(line)
            except Exception:
                continue
            if "price" in ev and "size" in ev:
                events.append(ev)
    return events


def _event_time_secs(ev: Dict[str, Any], fallback_idx: int) -> float:
    for key, scale in (("ts_exchange_ns", 1_000_000_000.0), ("ts_recv_ns", 1_000_000_000.0)):
        if key in ev:
            try:
                return float(ev[key]) / scale
            except Exception:
                pass
    for key in ("ts", "sequence"):
        if key in ev:
            try:
                return float(ev[key])
            except Exception:
                pass
    return float(fallback_idx)


def _replay_elapsed_for_index(events: List[Dict[str, Any]], index: int) -> tuple[float, float]:
    if not events:
        return 0.0, 0.0
    idx = max(0, min(index, len(events)))
    t0 = _event_time_secs(events[0], 0)
    t_cur = _event_time_secs(events[idx - 1], idx - 1) if idx > 0 else t0
    t_end = _event_time_secs(events[-1], len(events) - 1)
    return max(0.0, t_cur - t0), max(0.0, t_end - t0)


def _seek_index_by_elapsed(events: List[Dict[str, Any]], elapsed_secs: float) -> int:
    if not events:
        return 0
    t0 = _event_time_secs(events[0], 0)
    target = t0 + max(0.0, elapsed_secs)
    for i, ev in enumerate(events):
        if _event_time_secs(ev, i) >= target:
            return i
    return len(events)


def _default_replay_file() -> Optional[Path]:
    explicit = os.getenv("OF_DASH_REPLAY_FILE", "").strip()
    if explicit:
        return Path(explicit)
    data_root = ROOT / os.getenv("OF_DASH_DATA_ROOT", "data")
    if not data_root.exists():
        return None
    for p in data_root.rglob("trades.jsonl"):
        return p
    return None


def _push_history(delta: float, cum_delta: float) -> None:
    with STATE.lock:
        STATE.history.append({"delta": delta, "cum_delta": cum_delta, "ts": time.time()})
        if len(STATE.history) > 600:
            STATE.history = STATE.history[-600:]
        STATE.seq += 1


class FootprintBarBuilder:
    def __init__(self, events_per_bar: int = 24, max_bars: int = 120) -> None:
        self.events_per_bar = max(1, events_per_bar)
        self.max_bars = max(8, max_bars)
        self._bars: List[Dict[str, Any]] = []
        self._current: Optional[Dict[str, Any]] = None

    def reset(self) -> None:
        self._bars.clear()
        self._current = None

    def apply_trade(
        self, price: int, size: int, aggressor: str, ts: Optional[float] = None
    ) -> None:
        if price <= 0:
            return
        qty = max(1, int(size))
        side = str(aggressor or "Bid").lower()
        bar = self._ensure_bar(price, ts)
        if bar["count"] >= self.events_per_bar:
            self._finalize_current()
            bar = self._ensure_bar(price, ts)

        bar["close"] = price
        bar["high"] = max(bar["high"], price)
        bar["low"] = min(bar["low"], price)
        bar["volume"] += qty
        bar["delta"] += qty if side == "ask" else -qty
        bar["count"] += 1
        bar["ts"] = float(ts if ts is not None else time.time())
        levels = bar["levels"]
        level = levels.setdefault(price, {"price": price, "bid": 0, "ask": 0})
        if side == "ask":
            level["ask"] += qty
        else:
            level["bid"] += qty

    def apply_analytics(self, analytics: Dict[str, Any]) -> None:
        price = int(analytics.get("last_price", 0) or 0)
        if price <= 0:
            return
        buy = max(0, int(analytics.get("buy_volume", 0) or 0))
        sell = max(0, int(analytics.get("sell_volume", 0) or 0))
        delta = int(analytics.get("delta", 0) or 0)
        size = max(1, buy + sell, abs(delta))
        side = "Ask" if delta >= 0 else "Bid"
        self.apply_trade(price, size, side)

    def snapshot(self) -> List[Dict[str, Any]]:
        out: List[Dict[str, Any]] = []
        bars = [*self._bars]
        if self._current is not None:
            bars.append(self._current)
        for idx, bar in enumerate(bars[-self.max_bars :]):
            levels = list(bar["levels"].values())
            levels.sort(key=lambda row: row["price"], reverse=True)
            summary = _level_summary(levels)
            poc = 0
            if levels:
                poc = max(levels, key=lambda row: (row["bid"] + row["ask"], row["price"]))["price"]
            out.append(
                {
                    "index": idx,
                    "ts": bar["ts"],
                    "open": bar["open"],
                    "high": bar["high"],
                    "low": bar["low"],
                    "close": bar["close"],
                    "delta": bar["delta"],
                    "volume": bar["volume"],
                    "poc": poc,
                    "buy_volume": summary["buy_volume"],
                    "sell_volume": summary["sell_volume"],
                    "imbalance_ask": summary["imbalance_ask"],
                    "imbalance_bid": summary["imbalance_bid"],
                    "stacked_ask": summary["stacked_ask"],
                    "stacked_bid": summary["stacked_bid"],
                    "levels": levels,
                }
            )
        return out

    def _ensure_bar(self, price: int, ts: Optional[float]) -> Dict[str, Any]:
        if self._current is None:
            stamp = float(ts if ts is not None else time.time())
            self._current = {
                "open": price,
                "high": price,
                "low": price,
                "close": price,
                "delta": 0,
                "volume": 0,
                "count": 0,
                "ts": stamp,
                "levels": {},
            }
        return self._current

    def _finalize_current(self) -> None:
        if self._current is None:
            return
        frozen_levels = {
            price: dict(levels) for price, levels in self._current["levels"].items()
        }
        finished = {
            **self._current,
            "levels": frozen_levels,
        }
        self._bars.append(finished)
        if len(self._bars) > self.max_bars:
            self._bars = self._bars[-self.max_bars :]
        self._current = None


def _engine_loop() -> None:
    cfg = EngineConfig(
        instance_id=os.getenv("OF_DASH_INSTANCE_ID", "orderflow-dashboard"),
        config_path=os.getenv("OF_DASH_CONFIG_PATH", ""),
    )
    symbol = Symbol(
        os.getenv("OF_DASH_VENUE", "CME"),
        os.getenv("OF_DASH_SYMBOL", "ESM6"),
        depth_levels=int(os.getenv("OF_DASH_DEPTH", "10")),
    )

    sim_step = 0
    replay = ReplayAccumulator(threshold=int(os.getenv("OF_DASH_SIGNAL_THRESHOLD", "100")))
    live_bars = FootprintBarBuilder(
        events_per_bar=int(os.getenv("OF_DASH_BAR_EVENTS", "24")),
        max_bars=int(os.getenv("OF_DASH_MAX_BARS", "120")),
    )
    replay_bars = FootprintBarBuilder(
        events_per_bar=int(os.getenv("OF_DASH_BAR_EVENTS", "24")),
        max_bars=int(os.getenv("OF_DASH_MAX_BARS", "120")),
    )
    engine: Optional[Engine] = None
    try:
        engine = Engine(cfg)
        try:
            engine.start()
        except Exception as exc:
            metrics: Dict[str, Any] = {}
            try:
                metrics = engine.metrics()
            except Exception:
                metrics = {}
            STATE.update(
                metrics=metrics,
                health={
                    "degraded": True,
                    "last_error": (
                        f"dashboard engine start failed: {exc}; "
                        f"adapter_last_error={metrics.get('adapter_last_error')}"
                    ),
                    "reconnect_state": "failed",
                    "protocol_info": metrics.get("adapter_protocol_info"),
                },
            )
            return
        try:
            engine.subscribe(
                symbol,
                StreamKind.TRADES,
                callback=lambda ev: live_bars.apply_trade(
                    int(ev.get("price", 0) or 0),
                    int(ev.get("size", 0) or 0),
                    str(ev.get("aggressor", "Bid")),
                    (
                        (float(ev.get("ts_exchange_ns", 0) or 0) / 1_000_000_000.0)
                        if ev.get("ts_exchange_ns")
                        else None
                    ),
                ),
            )
            engine.subscribe(symbol, StreamKind.ANALYTICS, callback=lambda ev: STATE.update(analytics=ev))
            engine.subscribe(symbol, StreamKind.HEALTH, callback=lambda ev: STATE.update(health=ev))
            default_replay = _default_replay_file()
            if default_replay is not None:
                events = _load_replay_events(default_replay)
                STATE.update(
                    replay_loaded=len(events) > 0,
                    replay_events=events,
                    replay_file=str(default_replay),
                    replay_index=0,
                    replay_paused=True,
                )

            while STATE.running:
                with STATE.lock:
                    do_reset = STATE.reset_requested
                    if do_reset:
                        STATE.reset_requested = False
                    mode = STATE.mode
                    replay_step = STATE.replay_step_requested
                    if replay_step:
                        STATE.replay_step_requested = False
                    replay_reload = STATE.replay_reload_requested
                    if replay_reload:
                        STATE.replay_reload_requested = False
                    replay_paused = STATE.replay_paused
                    replay_speed = max(0.1, STATE.replay_speed)
                    replay_index = STATE.replay_index
                    replay_total = len(STATE.replay_events)
                    replay_events = list(STATE.replay_events)
                    replay_file = STATE.replay_file
                    replay_seek_target = STATE.replay_seek_target
                    if replay_seek_target is not None:
                        STATE.replay_seek_target = None
                if do_reset:
                    if mode == "live":
                        engine.reset_symbol_session(symbol)
                        live_bars.reset()
                    else:
                        replay = ReplayAccumulator(
                            threshold=int(os.getenv("OF_DASH_SIGNAL_THRESHOLD", "100"))
                        )
                        replay_bars.reset()
                    _reset_state_buffers(replay_mode=mode == "replay", replay_index=0)

                if replay_reload:
                    p = Path(replay_file) if replay_file else (_default_replay_file() or Path(""))
                    events = _load_replay_events(p)
                    replay = ReplayAccumulator(
                        threshold=int(os.getenv("OF_DASH_SIGNAL_THRESHOLD", "100"))
                    )
                    replay_bars.reset()
                    STATE.update(
                        replay_loaded=len(events) > 0,
                        replay_events=events,
                        replay_index=0,
                        replay_paused=True,
                        replay_file=str(p) if p else "",
                    )
                    _reset_state_buffers(replay_mode=False)

                if replay_seek_target is not None and replay_total > 0:
                    target = max(0, min(int(replay_seek_target), replay_total))
                    replay = ReplayAccumulator(
                        threshold=int(os.getenv("OF_DASH_SIGNAL_THRESHOLD", "100"))
                    )
                    replay_bars.reset()
                    for idx, ev in enumerate(replay_events[:target]):
                        price = int(ev.get("price", 0))
                        size = int(ev.get("size", 0))
                        aggressor = str(ev.get("aggressor", "Bid"))
                        replay.apply_trade(
                            price,
                            size,
                            aggressor,
                        )
                        replay_bars.apply_trade(
                            price,
                            size,
                            aggressor,
                            _event_time_secs(ev, idx),
                        )
                    replay_index = target
                    _reset_state_buffers(replay_mode=False)

                analytics: Dict[str, Any] = {}
                signal: Dict[str, Any] = {}
                metrics: Dict[str, Any] = {}

                if mode == "replay" and replay_total > 0:
                    steps = 0
                    if replay_step:
                        steps = 1
                    elif not replay_paused:
                        steps = max(1, int(round(replay_speed)))
                    while steps > 0 and replay_index < replay_total:
                        ev = replay_events[replay_index]
                        price = int(ev.get("price", 0))
                        size = int(ev.get("size", 0))
                        aggressor = str(ev.get("aggressor", "Bid"))
                        replay.apply_trade(
                            price,
                            size,
                            aggressor,
                        )
                        replay_bars.apply_trade(
                            price,
                            size,
                            aggressor,
                            _event_time_secs(ev, replay_index),
                        )
                        replay_index += 1
                        steps -= 1
                    analytics = replay.snapshot()
                    signal = replay.signal()
                    metrics = {
                        "instance_id": cfg.instance_id,
                        "started": True,
                        "processed_events": replay.processed,
                        "symbols": 1,
                        "persistence": False,
                        "adapter_connected": True,
                        "adapter_degraded": False,
                        "adapter_last_error": None,
                        "adapter_protocol_info": "replay_jsonl",
                    }
                    STATE.update(
                        replay_index=replay_index,
                        health={
                            "health_seq": replay_index,
                            "started": True,
                            "connected": True,
                            "degraded": False,
                            "reconnect_state": "replay",
                            "quality_flags": 0,
                            "last_error": None,
                            "protocol_info": "replay_jsonl",
                        },
                        simulated=False,
                    )
                else:
                    engine.poll_once(DataQualityFlags.NONE)
                    analytics = engine.analytics_snapshot(symbol)
                    signal = engine.signal_snapshot(symbol)
                    metrics = engine.metrics()

                # Fallback simulation keeps the dashboard alive when feed is idle.
                if not analytics:
                    sim_step += 1
                    delta = ((sim_step % 15) - 7) * 3
                    cum = (STATE.history[-1]["cum_delta"] if STATE.history else 0) + delta
                    last_price = 500000 + (sim_step % 20)
                    poc = 500008
                    analytics = {
                        "delta": delta,
                        "cumulative_delta": cum,
                        "buy_volume": max(delta, 0),
                        "sell_volume": max(-delta, 0),
                        "last_price": last_price,
                        "point_of_control": poc,
                        "value_area_low": poc - 4,
                        "value_area_high": poc + 4,
                    }
                    STATE.update(simulated=True)
                    if mode == "live":
                        live_bars.apply_analytics(analytics)
                else:
                    STATE.update(simulated=False)

                _push_history(
                    float(analytics.get("delta", 0)),
                    float(analytics.get("cumulative_delta", 0)),
                )
                STATE.update(
                    analytics=analytics,
                    signal=signal,
                    metrics=metrics,
                    bars=replay_bars.snapshot() if mode == "replay" else live_bars.snapshot(),
                )
                time.sleep(0.2)
        finally:
            engine.close()
    except Exception as exc:  # pragma: no cover
        STATE.update(
            health={
                "degraded": True,
                "last_error": f"dashboard engine loop failed: {exc}",
                "reconnect_state": "failed",
            }
        )


class DashboardHandler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:  # noqa: N802
        if self.path in ("/", "/index.html"):
            self._serve_file(ROOT / "dashboard" / "static" / "index.html", "text/html; charset=utf-8")
            return
        if self.path == "/state":
            self._write_json(CONTROLLER.snapshot())
            return
        if self.path == "/session":
            self._write_json(CONTROLLER.session_metadata())
            return
        if self.path == "/events":
            self._serve_sse()
            return
        self.send_error(HTTPStatus.NOT_FOUND, "Not found")

    def do_POST(self) -> None:  # noqa: N802
        if self.path == "/reset":
            CONTROLLER.request_reset()
            self._write_json({"ok": True})
            return
        if self.path == "/mode":
            body_json = self._read_json_body()
            if not CONTROLLER.set_mode(body_json.get("mode", "live")):
                self.send_error(HTTPStatus.BAD_REQUEST, "invalid mode")
                return
            self._write_json({"ok": True})
            return
        if self.path == "/replay/control":
            CONTROLLER.replay_control(self._read_json_body())
            self._write_json({"ok": True})
            return
        self.send_error(HTTPStatus.NOT_FOUND, "Not found")

    def log_message(self, format: str, *args: Any) -> None:  # noqa: A003
        return

    def _serve_file(self, path: Path, content_type: str) -> None:
        if not path.exists():
            self.send_error(HTTPStatus.NOT_FOUND, "File not found")
            return
        body = path.read_bytes()
        self.send_response(HTTPStatus.OK)
        self.send_header("Content-Type", content_type)
        self.send_header("Cache-Control", "no-store, max-age=0")
        self.send_header("Pragma", "no-cache")
        self.send_header("Expires", "0")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _read_json_body(self) -> Dict[str, Any]:
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length) if length > 0 else b"{}"
        try:
            return json.loads(raw.decode("utf-8") or "{}")
        except Exception:
            return {}

    def _write_json(self, payload: Dict[str, Any], status: HTTPStatus = HTTPStatus.OK) -> None:
        body = json.dumps(payload, separators=(",", ":")).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _serve_sse(self) -> None:
        self.send_response(HTTPStatus.OK)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "keep-alive")
        self.end_headers()

        last_seq = -1
        while STATE.running:
            snap = STATE.snapshot()
            if snap["seq"] != last_seq:
                payload = json.dumps(snap, separators=(",", ":"))
                chunk = f"event: state\ndata: {payload}\n\n".encode("utf-8")
                try:
                    self.wfile.write(chunk)
                    self.wfile.flush()
                except Exception:
                    break
                last_seq = snap["seq"]
            time.sleep(0.25)


def main() -> None:
    host = os.getenv("OF_DASH_HOST", "127.0.0.1")
    port = int(os.getenv("OF_DASH_PORT", "8080"))
    t = threading.Thread(target=_engine_loop, daemon=True)
    t.start()
    try:
        httpd = ThreadingHTTPServer((host, port), DashboardHandler)
        print(f"dashboard running: http://{host}:{port}")
        httpd.serve_forever()
    finally:
        STATE.running = False


if __name__ == "__main__":
    main()
