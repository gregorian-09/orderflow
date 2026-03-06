#!/usr/bin/env python3
"""Analyze captured trades/book jsonl files for quick core-logic validation."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Dict, List, Tuple


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--trades", required=True, help="Path to trades.jsonl")
    p.add_argument("--book", required=False, help="Path to book.jsonl")
    return p.parse_args()


def load_jsonl(path: Path) -> List[Dict]:
    rows: List[Dict] = []
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except Exception:
                continue
    return rows


def profile_levels(profile: Dict[int, int]) -> Tuple[int, int, int]:
    if not profile:
        return 0, 0, 0
    prices = sorted(profile.keys())
    total = sum(profile.values())
    poc = max(prices, key=lambda p: (profile[p], p))
    target = int(total * 0.70 + 0.999)
    covered = profile[poc]
    low = high = poc
    i = prices.index(poc)
    left = i - 1
    right = i + 1
    while covered < target and (left >= 0 or right < len(prices)):
        lv = profile[prices[left]] if left >= 0 else -1
        rv = profile[prices[right]] if right < len(prices) else -1
        if rv > lv:
            covered += max(0, rv)
            high = prices[right]
            right += 1
        else:
            covered += max(0, lv)
            low = prices[left]
            left -= 1
    return poc, low, high


def main() -> None:
    args = parse_args()
    trades = load_jsonl(Path(args.trades))
    if not trades:
        raise SystemExit("no trades loaded")

    delta = 0
    cum = 0
    buy = 0
    sell = 0
    profile: Dict[int, int] = {}
    for t in trades:
        px = int(t.get("price", 0))
        sz = max(1, int(t.get("size", 1)))
        profile[px] = profile.get(px, 0) + sz
        if str(t.get("aggressor", "")).lower() == "ask":
            buy += sz
            delta += sz
            cum += sz
        else:
            sell += sz
            delta -= sz
            cum -= sz
    poc, val, vah = profile_levels(profile)

    print("=== Trades Analysis ===")
    print(f"count:           {len(trades)}")
    print(f"delta:           {delta}")
    print(f"cumulative:      {cum}")
    print(f"buy_volume:      {buy}")
    print(f"sell_volume:     {sell}")
    print(f"point_of_control:{poc}")
    print(f"value_area_low:  {val}")
    print(f"value_area_high: {vah}")

    if args.book:
        book = load_jsonl(Path(args.book))
        print("=== Book Analysis ===")
        print(f"snapshots:       {len(book)}")
        if book:
            last = book[-1]
            print(f"last_update_id:  {last.get('lastUpdateId')}")
            print(f"top_bid:         {last.get('bids', [[None, None]])[0]}")
            print(f"top_ask:         {last.get('asks', [[None, None]])[0]}")


if __name__ == "__main__":
    main()
