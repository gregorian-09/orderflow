#!/usr/bin/env python3
"""Capture public market data (trades + orderbook snapshots) to jsonl files.

Default source is Binance public REST for crypto symbols (no auth required).
"""

from __future__ import annotations

import argparse
import json
import time
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any, Dict, List, Optional


BINANCE_REST = "https://api.binance.com"


def http_get_json(url: str, timeout: float = 10.0) -> Any:
    req = urllib.request.Request(url, method="GET")
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


def fetch_depth(symbol: str, limit: int) -> Dict[str, Any]:
    q = urllib.parse.urlencode({"symbol": symbol.upper(), "limit": str(limit)})
    return http_get_json(f"{BINANCE_REST}/api/v3/depth?{q}")


def fetch_agg_trades(symbol: str, limit: int, from_id: Optional[int]) -> List[Dict[str, Any]]:
    params = {"symbol": symbol.upper(), "limit": str(limit)}
    if from_id is not None:
        params["fromId"] = str(from_id)
    q = urllib.parse.urlencode(params)
    data = http_get_json(f"{BINANCE_REST}/api/v3/aggTrades?{q}")
    if isinstance(data, list):
        return data
    return []


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--symbol", default="BTCUSDT")
    p.add_argument("--duration-secs", type=int, default=60)
    p.add_argument("--poll-ms", type=int, default=500)
    p.add_argument("--depth-levels", type=int, default=20)
    p.add_argument("--trade-batch", type=int, default=1000)
    p.add_argument("--out-dir", default="data_capture")
    return p.parse_args()


def main() -> None:
    args = parse_args()
    out_dir = Path(args.out_dir) / "BINANCE" / args.symbol.upper()
    out_dir.mkdir(parents=True, exist_ok=True)
    trades_path = out_dir / "trades.jsonl"
    depth_path = out_dir / "book.jsonl"

    end_ts = time.time() + args.duration_secs
    last_trade_id: Optional[int] = None

    with trades_path.open("a", encoding="utf-8") as tf, depth_path.open("a", encoding="utf-8") as df:
        while time.time() < end_ts:
            now_ns = int(time.time() * 1_000_000_000)

            depth = fetch_depth(args.symbol, args.depth_levels)
            depth_row = {
                "ts": time.time(),
                "ts_recv_ns": now_ns,
                "symbol": args.symbol.upper(),
                "lastUpdateId": depth.get("lastUpdateId", 0),
                "bids": depth.get("bids", []),
                "asks": depth.get("asks", []),
            }
            df.write(json.dumps(depth_row, separators=(",", ":")) + "\n")

            trades = fetch_agg_trades(args.symbol, args.trade_batch, last_trade_id)
            for t in trades:
                trade_id = int(t.get("a", 0))
                price = int(float(t.get("p", "0")) * 1000000)
                size = int(float(t.get("q", "0")) * 1000000)
                # Binance 'm' true => buyer is maker => sell aggressor
                aggressor = "Bid" if bool(t.get("m", False)) else "Ask"
                ts_exchange_ns = int(t.get("T", 0)) * 1_000_000
                row = {
                    "sequence": trade_id,
                    "price": price,
                    "size": max(size, 1),
                    "aggressor": aggressor,
                    "ts_exchange_ns": ts_exchange_ns,
                    "ts_recv_ns": now_ns,
                }
                tf.write(json.dumps(row, separators=(",", ":")) + "\n")
                last_trade_id = trade_id + 1

            tf.flush()
            df.flush()
            time.sleep(max(0.05, args.poll_ms / 1000.0))

    print(f"wrote trades: {trades_path}")
    print(f"wrote book:   {depth_path}")


if __name__ == "__main__":
    main()
