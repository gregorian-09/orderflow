# Live Dashboard

This dashboard provides a real-time UI over the Python binding and renders:

- Delta and cumulative delta
- Market profile levels (POC / VAH / VAL)
- Signal state/confidence
- Health/reconnect state
- Footprint-style buy/sell proxy bars
- Cumulative delta timeline

The backend also exposes:

- `GET /state`: full UI state snapshot
- `GET /session`: lightweight session/replay metadata for external controls or diagnostics

`/state` bar payloads now include derived footprint fields such as `buy_volume`, `sell_volume`, `imbalance_ask`, `imbalance_bid`, `stacked_ask`, and `stacked_bid` so the frontend can reuse backend-calculated bar summaries.

It also provides a **Reset Session** control in the header, which calls the engine session reset path for the active symbol and clears dashboard history.

## Replay mode

The dashboard supports deterministic replay from persisted trade events (`trades.jsonl`):

- Switch mode with **Live** / **Replay** buttons.
- Control replay with **Play / Pause / Step**.
- Change replay speed (`1x`, `2x`, `5x`, `10x`).
- Use the replay slider scrubber to jump directly to a target replay index.
- Current replay elapsed/total time is shown in the header.
- Use **Jump Sec** to seek replay by elapsed seconds.

Replay source selection:

- `OF_DASH_REPLAY_FILE=/path/to/trades.jsonl` to force a file.
- Otherwise, server auto-discovers first `trades.jsonl` under `OF_DASH_DATA_ROOT` (default `data`).

## Run

From repo root:

```bash
python3 dashboard/server.py
```

Then open:

```text
http://127.0.0.1:8080
```

## Smoke test

From repo root:

```bash
python3 tools/dashboard_smoke_test.py
```

This boots the dashboard server on a test port, validates `GET /session`, and checks that `/state` bars expose derived fields (`buy_volume`, `sell_volume`, `imbalance_ask`, `imbalance_bid`, `stacked_ask`, `stacked_bid`).

## Optional environment variables

- `OF_DASH_CONFIG_PATH`: runtime config file path (`.toml` or `.json`)
- `OF_DASH_INSTANCE_ID`: runtime instance id
- `OF_DASH_VENUE`: symbol venue (default `CME`)
- `OF_DASH_SYMBOL`: symbol code (default `ESM6`)
- `OF_DASH_DEPTH`: depth levels (default `10`)
- `OF_DASH_HOST`: bind host (default `127.0.0.1`)
- `OF_DASH_PORT`: bind port (default `8080`)

If no live events are flowing, the UI falls back to simulated deltas so users can still see panel behavior and layout.
