# Performance Harness

`examples/perf_harness` provides two runtime modes to validate throughput, p99 poll latency, and memory trend.

## Build

```bash
cargo build --release -p perf_harness
```

## Benchmark mode

Runs a fixed synthetic replay and reports summary stats.

```bash
cargo run --release -p perf_harness -- benchmark --events=200000
```

Outputs:
- processed events
- elapsed ms
- events/sec
- poll p99 latency (us)
- RSS memory (KB)

## Soak mode

Runs continuous replay and prints CSV-like per-second telemetry.

```bash
cargo run --release -p perf_harness -- soak --duration=300 --batch=2000
```

Columns:
- second
- processed_total
- poll_p99_us
- rss_kb

Use soak mode to detect memory growth, latency drift, or throughput collapse under long runs.
