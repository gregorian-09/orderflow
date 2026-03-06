# Real Data Test (Core Logic + Dashboard)

Use this flow to test core analytics and dashboard with real public data first (no broker account).

## 1) Capture public orderbook + trades (Binance)

```bash
python3 tools/capture_public_market_data.py \
  --symbol BTCUSDT \
  --duration-secs 120 \
  --poll-ms 500 \
  --depth-levels 20 \
  --out-dir data_capture
```

Outputs:

- `data_capture/BINANCE/BTCUSDT/trades.jsonl`
- `data_capture/BINANCE/BTCUSDT/book.jsonl`

## 2) Analyze captured data

```bash
python3 tools/analyze_captured_data.py \
  --trades data_capture/BINANCE/BTCUSDT/trades.jsonl \
  --book data_capture/BINANCE/BTCUSDT/book.jsonl
```

This validates:

- delta + cumulative delta accumulation
- buy/sell volume split
- POC/VAH/VAL profile math
- basic top-of-book snapshot quality

## 3) Replay in dashboard

```bash
export OF_DASH_REPLAY_FILE=data_capture/BINANCE/BTCUSDT/trades.jsonl
python3 dashboard/server.py
```

Open:

```text
http://127.0.0.1:8080
```

In UI:

- switch to **Replay**
- use **Play/Pause/Step**
- use slider + **Jump Sec** for timeline navigation
- verify profile bands and health panels update correctly
