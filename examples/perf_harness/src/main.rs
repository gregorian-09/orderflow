use std::collections::VecDeque;
use std::time::{Duration, Instant};

use of_adapters::{AdapterHealth, AdapterResult, MarketDataAdapter, RawEvent, SubscribeReq};
use of_core::{DataQualityFlags, Side, SymbolId, TradePrint};
use of_runtime::{Engine, EngineConfig};
use of_signals::DeltaMomentumSignal;

#[derive(Debug)]
struct ReplayAdapter {
    connected: bool,
    events: VecDeque<RawEvent>,
}

impl ReplayAdapter {
    fn with_trade_events(symbol: SymbolId, count: usize) -> Self {
        let mut events = VecDeque::with_capacity(count);
        for i in 0..count {
            events.push_back(RawEvent::Trade(TradePrint {
                symbol: symbol.clone(),
                price: 500_000 + (i % 8) as i64,
                size: 1 + (i % 5) as i64,
                aggressor_side: if i % 2 == 0 { Side::Ask } else { Side::Bid },
                sequence: i as u64 + 1,
                ts_exchange_ns: i as u64,
                ts_recv_ns: i as u64,
            }));
        }
        Self {
            connected: false,
            events,
        }
    }
}

impl MarketDataAdapter for ReplayAdapter {
    fn connect(&mut self) -> AdapterResult<()> {
        self.connected = true;
        Ok(())
    }

    fn subscribe(&mut self, _req: SubscribeReq) -> AdapterResult<()> {
        Ok(())
    }

    fn unsubscribe(&mut self, _symbol: SymbolId) -> AdapterResult<()> {
        Ok(())
    }

    fn poll(&mut self, out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
        if !self.connected {
            return Err(of_adapters::AdapterError::Disconnected);
        }
        if let Some(ev) = self.events.pop_front() {
            out.push(ev);
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth {
            connected: self.connected,
            degraded: false,
            last_error: None,
            protocol_info: Some("replay_adapter".to_string()),
        }
    }
}

fn parse_arg_usize(name: &str, default: usize) -> usize {
    let key = format!("--{name}=");
    std::env::args()
        .find_map(|a| a.strip_prefix(&key).map(str::to_string))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn parse_arg_u64(name: &str, default: u64) -> u64 {
    let key = format!("--{name}=");
    std::env::args()
        .find_map(|a| a.strip_prefix(&key).map(str::to_string))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn parse_mode() -> String {
    std::env::args()
        .nth(1)
        .unwrap_or_else(|| "benchmark".to_string())
}

fn p99_ns(mut v: Vec<u128>) -> u128 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    let idx = ((v.len() as f64) * 0.99).floor() as usize;
    v[idx.min(v.len() - 1)]
}

fn current_rss_kb() -> u64 {
    let Ok(s) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest
                .split_whitespace()
                .next()
                .and_then(|x| x.parse::<u64>().ok())
                .unwrap_or(0);
        }
    }
    0
}

fn run_benchmark(events: usize) {
    let symbol = SymbolId {
        venue: "CME".to_string(),
        symbol: "ESM6".to_string(),
    };
    let adapter = ReplayAdapter::with_trade_events(symbol.clone(), events);
    let mut engine = Engine::new(
        EngineConfig::default(),
        adapter,
        DeltaMomentumSignal::new(100),
    );

    engine.start().expect("engine start");
    engine.subscribe(symbol, 10).expect("subscribe");

    let start = Instant::now();
    let mut latencies = Vec::with_capacity(events);
    let mut processed = 0usize;

    while processed < events {
        let t0 = Instant::now();
        let n = engine
            .poll_once(DataQualityFlags::NONE)
            .expect("poll_once should succeed");
        latencies.push(t0.elapsed().as_nanos());
        processed += n;
        if n == 0 {
            break;
        }
    }

    let elapsed = start.elapsed();
    let eps = if elapsed.as_secs_f64() > 0.0 {
        (processed as f64) / elapsed.as_secs_f64()
    } else {
        0.0
    };

    println!("mode=benchmark");
    println!("events={}", processed);
    println!("elapsed_ms={:.3}", elapsed.as_secs_f64() * 1_000.0);
    println!("events_per_sec={:.0}", eps);
    println!("poll_p99_us={:.3}", p99_ns(latencies) as f64 / 1_000.0);
    println!("rss_kb={}", current_rss_kb());
}

fn run_soak(duration_secs: u64, batch: usize) {
    let symbol = SymbolId {
        venue: "CME".to_string(),
        symbol: "ESM6".to_string(),
    };
    let total_events = (duration_secs as usize) * batch * 10;
    let adapter = ReplayAdapter::with_trade_events(symbol.clone(), total_events);
    let mut engine = Engine::new(
        EngineConfig::default(),
        adapter,
        DeltaMomentumSignal::new(100),
    );

    engine.start().expect("engine start");
    engine.subscribe(symbol, 10).expect("subscribe");

    println!("mode=soak duration_secs={} batch={}", duration_secs, batch);
    println!("sec,processed_total,poll_p99_us,rss_kb");

    let started = Instant::now();
    let mut since_last = Instant::now();
    let mut sec_lat = Vec::new();
    let mut processed_total = 0usize;

    while started.elapsed() < Duration::from_secs(duration_secs) {
        for _ in 0..batch {
            let t0 = Instant::now();
            let n = engine
                .poll_once(DataQualityFlags::NONE)
                .expect("poll_once should succeed");
            sec_lat.push(t0.elapsed().as_nanos());
            processed_total += n;
        }

        if since_last.elapsed() >= Duration::from_secs(1) {
            let sec = started.elapsed().as_secs();
            let p99 = p99_ns(std::mem::take(&mut sec_lat));
            println!(
                "{},{},{:.3},{}",
                sec,
                processed_total,
                p99 as f64 / 1_000.0,
                current_rss_kb()
            );
            since_last = Instant::now();
        }
    }

    println!("soak_done processed_total={}", processed_total);
}

fn main() {
    let mode = parse_mode();
    match mode.as_str() {
        "benchmark" => run_benchmark(parse_arg_usize("events", 200_000)),
        "soak" => run_soak(parse_arg_u64("duration", 30), parse_arg_usize("batch", 2000)),
        _ => {
            eprintln!("usage:");
            eprintln!("  perf_harness benchmark --events=200000");
            eprintln!("  perf_harness soak --duration=30 --batch=2000");
            std::process::exit(2);
        }
    }
}
