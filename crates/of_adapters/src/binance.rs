use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use of_core::{BookAction, BookUpdate, Side, SymbolId, TradePrint};

use crate::{
    AdapterConfig, AdapterError, AdapterHealth, AdapterResult, MarketDataAdapter, RawEvent,
    SubscribeReq,
};

const PRICE_SCALE: i64 = 1_000_000;
const SIZE_SCALE: i64 = 1_000;
const REPLAY_RECV_TS_NS: u64 = 0;

#[derive(Debug, Clone, Copy, Default)]
struct BinanceDepthState {
    last_update_id: Option<u64>,
}

impl BinanceDepthState {
    fn classify(
        &mut self,
        first_update_id: u64,
        final_update_id: u64,
        previous_update_id: Option<u64>,
    ) -> BinanceDepthDecision {
        let last = self.last_update_id;
        if let Some(last_update_id) = last {
            if final_update_id <= last_update_id {
                return BinanceDepthDecision::Duplicate;
            }
            if let Some(previous_update_id) = previous_update_id {
                if previous_update_id != last_update_id {
                    return BinanceDepthDecision::Gap;
                }
            } else if first_update_id <= last_update_id {
                self.last_update_id = Some(final_update_id);
                return BinanceDepthDecision::ApplyOutOfOrder;
            } else if first_update_id > last_update_id.saturating_add(1) {
                return BinanceDepthDecision::Gap;
            }
        }
        self.last_update_id = Some(final_update_id);
        BinanceDepthDecision::Apply
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinanceDepthDecision {
    Apply,
    ApplyOutOfOrder,
    Duplicate,
    Gap,
}

/// Resolved Binance adapter configuration.
#[derive(Debug, Clone)]
pub struct BinanceConfig {
    endpoint: String,
}

impl BinanceConfig {
    /// Builds Binance config from generic adapter config with default mock endpoint.
    pub fn from_adapter_config(cfg: &AdapterConfig) -> AdapterResult<Self> {
        let endpoint = cfg
            .endpoint
            .clone()
            .unwrap_or_else(|| "mock://binance".to_string());
        if !endpoint.starts_with("wss://")
            && !endpoint.starts_with("ws://")
            && !endpoint.starts_with("mock://")
        {
            return Err(AdapterError::NotConfigured(
                "binance endpoint must use wss://, ws://, or mock://",
            ));
        }
        Ok(Self { endpoint })
    }
}

#[derive(Debug)]
enum BinanceTransport {
    Mock,
    Live(WsTextTransport),
}

#[derive(Debug, Clone)]
enum Outbound {
    Text(String),
    Pong(Vec<u8>),
}

#[derive(Debug)]
struct WsTextTransport {
    endpoint: String,
    connected: bool,
    outbound_tx: Option<Sender<Outbound>>,
    inbound_rx: Option<Receiver<String>>,
    inbound_tx: Option<Sender<String>>,
}

impl WsTextTransport {
    fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            connected: false,
            outbound_tx: None,
            inbound_rx: None,
            inbound_tx: None,
        }
    }

    fn connect(&mut self) -> AdapterResult<()> {
        let parsed = ParsedEndpoint::parse(&self.endpoint)?;
        #[cfg(test)]
        if parsed.host == "test.live" {
            let (out_tx, out_rx) = mpsc::channel::<Outbound>();
            let (in_tx, in_rx) = mpsc::channel::<String>();
            let _ = thread::spawn(move || while out_rx.recv().is_ok() {});
            self.connected = true;
            self.outbound_tx = Some(out_tx);
            self.inbound_rx = Some(in_rx);
            self.inbound_tx = Some(in_tx);
            return Ok(());
        }
        let (out_tx, out_rx) = mpsc::channel::<Outbound>();
        let (in_tx, in_rx) = mpsc::channel::<String>();

        match parsed.scheme.as_str() {
            "ws" => {
                let mut stream = TcpStream::connect((parsed.host.as_str(), parsed.port))
                    .map_err(|e| AdapterError::Other(format!("binance ws connect failed: {e}")))?;
                let _ = stream.set_nodelay(true);
                websocket_handshake(&mut stream, &parsed.host, parsed.port, &parsed.path)?;
                let writer = stream
                    .try_clone()
                    .map_err(|e| AdapterError::Other(format!("binance ws clone failed: {e}")))?;
                spawn_text_ws_workers(writer, stream, out_rx, in_tx.clone(), out_tx.clone());
            }
            "wss" => {
                let mut child = Command::new("openssl")
                    .args([
                        "s_client",
                        "-quiet",
                        "-connect",
                        &format!("{}:{}", parsed.host, parsed.port),
                        "-servername",
                        &parsed.host,
                    ])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| AdapterError::Other(format!("openssl spawn failed: {e}")))?;

                let mut stdin = child
                    .stdin
                    .take()
                    .ok_or(AdapterError::Other("openssl stdin unavailable".to_string()))?;
                let mut stdout = child.stdout.take().ok_or(AdapterError::Other(
                    "openssl stdout unavailable".to_string(),
                ))?;

                websocket_handshake_rw(
                    &mut stdin,
                    &mut stdout,
                    &parsed.host,
                    parsed.port,
                    &parsed.path,
                )?;
                spawn_text_ws_workers(stdin, stdout, out_rx, in_tx.clone(), out_tx.clone());
                let _ = thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            _ => {
                return Err(AdapterError::NotConfigured(
                    "binance websocket endpoint must use ws:// or wss://",
                ))
            }
        }

        self.connected = true;
        self.outbound_tx = Some(out_tx);
        self.inbound_rx = Some(in_rx);
        self.inbound_tx = Some(in_tx);
        Ok(())
    }

    fn send_text(&mut self, text: String) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        let tx = self
            .outbound_tx
            .as_ref()
            .ok_or(AdapterError::Disconnected)?;
        tx.send(Outbound::Text(text))
            .map_err(|_| AdapterError::Other("binance transport send failed".to_string()))
    }

    fn recv_text(&mut self) -> AdapterResult<Option<String>> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        let rx = self.inbound_rx.as_ref().ok_or(AdapterError::Disconnected)?;
        match rx.try_recv() {
            Ok(v) => Ok(Some(v)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.connected = false;
                Err(AdapterError::Disconnected)
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    #[cfg(test)]
    fn inject_text(&mut self, text: &str) {
        if let Some(tx) = &self.inbound_tx {
            let _ = tx.send(text.to_string());
        }
    }

    #[cfg(test)]
    fn force_disconnect(&mut self) {
        self.connected = false;
    }
}

/// Binance websocket adapter with mock/live transport support.
#[derive(Debug)]
pub struct BinanceAdapter {
    cfg: BinanceConfig,
    transport: BinanceTransport,
    connected: bool,
    degraded: bool,
    last_error: Option<String>,
    subscribed: HashMap<SymbolId, u16>,
    depth_state: HashMap<SymbolId, BinanceDepthState>,
    queue: VecDeque<RawEvent>,
    max_queue_depth: usize,
    raw_capture_capacity: usize,
    raw_capture: VecDeque<String>,
    seq: u64,
    request_id: u64,
    messages_received: u64,
    normalized_events: u64,
    parse_errors: u64,
    dropped_events: u64,
    backpressure_events: u64,
    raw_capture_dropped: u64,
    parse_latency_samples: u64,
    parse_latency_total_ns: u64,
    parse_latency_max_ns: u64,
    normalization_latency_samples: u64,
    normalization_latency_total_ns: u64,
    normalization_latency_max_ns: u64,
    duplicate_depth_updates: u64,
    out_of_order_depth_updates: u64,
    gap_count: u64,
    snapshot_rebuild_count: u64,
    reconnect_attempt: u32,
    next_reconnect_at: Option<Instant>,
    last_message_at: Option<Instant>,
    last_market_data_at: Option<Instant>,
    healthy_since: Option<Instant>,
}

impl BinanceAdapter {
    /// Creates a Binance adapter from generic adapter configuration.
    pub fn from_config(cfg: &AdapterConfig) -> AdapterResult<Self> {
        let cfg = BinanceConfig::from_adapter_config(cfg)?;
        let transport = if cfg.endpoint.starts_with("mock://") {
            BinanceTransport::Mock
        } else {
            BinanceTransport::Live(WsTextTransport::new(cfg.endpoint.clone()))
        };
        Ok(Self {
            cfg,
            transport,
            connected: false,
            degraded: false,
            last_error: None,
            subscribed: HashMap::new(),
            depth_state: HashMap::new(),
            queue: VecDeque::new(),
            max_queue_depth: 0,
            raw_capture_capacity: 0,
            raw_capture: VecDeque::new(),
            seq: 0,
            request_id: 0,
            messages_received: 0,
            normalized_events: 0,
            parse_errors: 0,
            dropped_events: 0,
            backpressure_events: 0,
            raw_capture_dropped: 0,
            parse_latency_samples: 0,
            parse_latency_total_ns: 0,
            parse_latency_max_ns: 0,
            normalization_latency_samples: 0,
            normalization_latency_total_ns: 0,
            normalization_latency_max_ns: 0,
            duplicate_depth_updates: 0,
            out_of_order_depth_updates: 0,
            gap_count: 0,
            snapshot_rebuild_count: 0,
            reconnect_attempt: 0,
            next_reconnect_at: None,
            last_message_at: None,
            last_market_data_at: None,
            healthy_since: None,
        })
    }

    /// Returns a copy of this adapter with a maximum pending event queue depth.
    ///
    /// A depth of `0` keeps the default unbounded queue behavior for backward
    /// compatibility. Non-zero values cap the internal pending event queue; when
    /// the queue is full, the candidate event is shed, backpressure counters are
    /// incremented, and health is marked degraded.
    pub fn with_max_queue_depth(mut self, max_queue_depth: usize) -> Self {
        self.max_queue_depth = max_queue_depth;
        self
    }

    /// Sets the maximum pending event queue depth.
    ///
    /// A depth of `0` disables the bound. This does not drop events already in
    /// the queue; the bound is enforced on subsequent event normalization.
    pub fn set_max_queue_depth(&mut self, max_queue_depth: usize) {
        self.max_queue_depth = max_queue_depth;
    }

    /// Returns the configured maximum pending event queue depth.
    ///
    /// A value of `0` means the queue is unbounded, which preserves historical
    /// adapter behavior.
    pub fn max_queue_depth(&self) -> usize {
        self.max_queue_depth
    }

    /// Returns a copy of this adapter with raw inbound message capture enabled.
    ///
    /// A capacity of `0` keeps capture disabled. Non-zero capacity stores the
    /// most recent raw inbound provider messages in a bounded ring buffer for
    /// incident analysis and fixture generation.
    pub fn with_raw_capture_capacity(mut self, capacity: usize) -> Self {
        self.set_raw_capture_capacity(capacity);
        self
    }

    /// Sets the raw inbound message capture capacity.
    ///
    /// A capacity of `0` disables capture and clears buffered raw messages.
    /// Lowering capacity prunes oldest buffered messages first.
    pub fn set_raw_capture_capacity(&mut self, capacity: usize) {
        self.raw_capture_capacity = capacity;
        if capacity == 0 {
            self.raw_capture.clear();
            return;
        }
        while self.raw_capture.len() > capacity {
            self.raw_capture.pop_front();
            self.raw_capture_dropped = self.raw_capture_dropped.saturating_add(1);
        }
    }

    /// Returns the configured raw inbound message capture capacity.
    ///
    /// A value of `0` means raw capture is disabled.
    pub fn raw_capture_capacity(&self) -> usize {
        self.raw_capture_capacity
    }

    /// Returns the number of raw inbound messages currently buffered.
    pub fn raw_capture_len(&self) -> usize {
        self.raw_capture.len()
    }

    /// Returns the number of raw inbound messages dropped from the capture ring.
    pub fn raw_capture_dropped(&self) -> u64 {
        self.raw_capture_dropped
    }

    /// Drains captured raw inbound messages into `out`.
    ///
    /// Messages are appended in capture order. The return value is the number of
    /// messages appended.
    pub fn drain_raw_messages(&mut self, out: &mut Vec<String>) -> usize {
        let n = self.raw_capture.len();
        out.extend(self.raw_capture.drain(..));
        n
    }

    /// Replays raw Binance JSON messages into normalized events.
    ///
    /// This helper is intended for fixture tests and incident reproduction. It
    /// uses the same parser and normalizer as the live path, records the same
    /// parse/normalization counters, honors raw capture and queue bounds, and
    /// appends newly produced events to `out`. Fixture replay sets local receive
    /// timestamps to `0` so repeated fixture runs are deterministic.
    pub fn replay_raw_messages(&mut self, messages: &[&str], out: &mut Vec<RawEvent>) -> usize {
        let queue_start = self.queue.len();
        for msg in messages {
            self.process_raw_message(msg, REPLAY_RECV_TS_NS);
        }
        let produced = self.queue.len().saturating_sub(queue_start);
        out.extend(self.queue.drain(queue_start..));
        produced
    }

    fn is_mock_mode(&self) -> bool {
        matches!(self.transport, BinanceTransport::Mock)
    }

    fn next_request_id(&mut self) -> u64 {
        self.request_id = self.request_id.saturating_add(1);
        self.request_id
    }

    fn now_ns() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }

    fn synth_trade(&mut self, symbol: &SymbolId) {
        self.seq = self.seq.saturating_add(1);
        let base = if symbol.symbol.to_ascii_uppercase().contains("BTC") {
            66_000 * PRICE_SCALE
        } else {
            300 * PRICE_SCALE
        };
        self.push_event(RawEvent::Trade(TradePrint {
            symbol: symbol.clone(),
            price: base + (self.seq % 25) as i64 * 10_000,
            size: 1 + (self.seq % 3) as i64,
            aggressor_side: if self.seq.is_multiple_of(2) {
                Side::Ask
            } else {
                Side::Bid
            },
            sequence: self.seq,
            ts_exchange_ns: Self::now_ns(),
            ts_recv_ns: Self::now_ns(),
        }));
    }

    fn push_event(&mut self, event: RawEvent) -> bool {
        if self.max_queue_depth != 0 && self.queue.len() >= self.max_queue_depth {
            self.dropped_events = self.dropped_events.saturating_add(1);
            self.backpressure_events = self.backpressure_events.saturating_add(1);
            self.degraded = true;
            self.healthy_since = None;
            self.last_error = Some(format!(
                "binance queue backpressure queue_depth={} max_queue_depth={}",
                self.queue.len(),
                self.max_queue_depth
            ));
            return false;
        }

        self.queue.push_back(event);
        self.normalized_events = self.normalized_events.saturating_add(1);
        true
    }

    fn capture_raw_message(&mut self, msg: &str) {
        if self.raw_capture_capacity == 0 {
            return;
        }
        if self.raw_capture.len() >= self.raw_capture_capacity {
            self.raw_capture.pop_front();
            self.raw_capture_dropped = self.raw_capture_dropped.saturating_add(1);
        }
        self.raw_capture.push_back(msg.to_string());
    }

    fn record_parse_latency(&mut self, elapsed: Duration) {
        let elapsed_ns = duration_ns(elapsed);
        self.parse_latency_samples = self.parse_latency_samples.saturating_add(1);
        self.parse_latency_total_ns = self.parse_latency_total_ns.saturating_add(elapsed_ns);
        self.parse_latency_max_ns = self.parse_latency_max_ns.max(elapsed_ns);
    }

    fn record_normalization_latency(&mut self, elapsed: Duration) {
        let elapsed_ns = duration_ns(elapsed);
        self.normalization_latency_samples = self.normalization_latency_samples.saturating_add(1);
        self.normalization_latency_total_ns = self
            .normalization_latency_total_ns
            .saturating_add(elapsed_ns);
        self.normalization_latency_max_ns = self.normalization_latency_max_ns.max(elapsed_ns);
    }

    fn process_raw_message(&mut self, msg: &str, ts_recv_ns: u64) {
        self.capture_raw_message(msg);
        let parse_started = Instant::now();
        self.parse_live_message(msg, ts_recv_ns);
        self.record_parse_latency(parse_started.elapsed());
    }

    fn send_binance_subscribe(&mut self, symbol: &SymbolId) -> AdapterResult<()> {
        let sym = symbol.symbol.to_ascii_lowercase();
        let payload = format!(
            "{{\"method\":\"SUBSCRIBE\",\"params\":[\"{}@aggTrade\",\"{}@depth@100ms\"],\"id\":{}}}",
            sym,
            sym,
            self.next_request_id()
        );
        match &mut self.transport {
            BinanceTransport::Live(ws) => ws.send_text(payload),
            BinanceTransport::Mock => Ok(()),
        }
    }

    fn send_binance_unsubscribe(&mut self, symbol: &SymbolId) -> AdapterResult<()> {
        let sym = symbol.symbol.to_ascii_lowercase();
        let payload = format!(
            "{{\"method\":\"UNSUBSCRIBE\",\"params\":[\"{}@aggTrade\",\"{}@depth@100ms\"],\"id\":{}}}",
            sym,
            sym,
            self.next_request_id()
        );
        match &mut self.transport {
            BinanceTransport::Live(ws) => ws.send_text(payload),
            BinanceTransport::Mock => Ok(()),
        }
    }

    fn parse_live_message(&mut self, msg: &str, ts_recv_ns: u64) {
        self.last_message_at = Some(Instant::now());
        self.messages_received = self.messages_received.saturating_add(1);
        let payload = extract_data_object(msg).unwrap_or(msg);
        if payload.contains("\"result\":null") || payload.contains("\"type\":\"subscribed\"") {
            self.healthy_since.get_or_insert_with(Instant::now);
            return;
        }
        if payload.contains("\"error\"") || payload.contains("\"code\"") {
            self.degraded = true;
            self.healthy_since = None;
            self.last_error = extract_string_field(payload, "msg")
                .or_else(|| extract_string_field(payload, "error"))
                .map(str::to_string)
                .or_else(|| Some("binance live error".to_string()));
            return;
        }
        if payload.contains("\"e\":\"aggTrade\"") {
            let normalization_started = Instant::now();
            if let Some(trade) = parse_agg_trade(payload, &mut self.seq, ts_recv_ns) {
                self.push_event(RawEvent::Trade(trade));
                self.record_normalization_latency(normalization_started.elapsed());
                self.last_market_data_at = Some(Instant::now());
                self.healthy_since.get_or_insert_with(Instant::now);
            } else {
                self.parse_errors = self.parse_errors.saturating_add(1);
            }
            return;
        }

        if payload.contains("\"e\":\"depthUpdate\"") {
            let symbol = match extract_string_field(payload, "s") {
                Some(s) => s.to_string(),
                None => {
                    self.parse_errors = self.parse_errors.saturating_add(1);
                    return;
                }
            };
            let sym_id = SymbolId {
                venue: "BINANCE".to_string(),
                symbol,
            };
            let depth_limit = self.subscribed.get(&sym_id).copied().unwrap_or(10) as usize;
            let first_update_id = extract_u64_field(payload, "U");
            let final_update_id = extract_u64_field(payload, "u");
            let Some(final_update_id) = final_update_id else {
                self.parse_errors = self.parse_errors.saturating_add(1);
                return;
            };
            let first_update_id = first_update_id.unwrap_or(final_update_id);
            let previous_update_id = extract_u64_field(payload, "pu");
            match self
                .depth_state
                .entry(sym_id.clone())
                .or_default()
                .classify(first_update_id, final_update_id, previous_update_id)
            {
                BinanceDepthDecision::Apply => {}
                BinanceDepthDecision::ApplyOutOfOrder => {
                    self.out_of_order_depth_updates =
                        self.out_of_order_depth_updates.saturating_add(1);
                }
                BinanceDepthDecision::Duplicate => {
                    self.duplicate_depth_updates = self.duplicate_depth_updates.saturating_add(1);
                    return;
                }
                BinanceDepthDecision::Gap => {
                    self.gap_count = self.gap_count.saturating_add(1);
                    self.snapshot_rebuild_count = self.snapshot_rebuild_count.saturating_add(1);
                    self.degraded = true;
                    self.healthy_since = None;
                    self.last_error = Some(format!(
                        "binance depth gap symbol={} first_update_id={} final_update_id={} previous_update_id={:?}",
                        sym_id.symbol, first_update_id, final_update_id, previous_update_id
                    ));
                    self.depth_state.remove(&sym_id);
                    if self.next_reconnect_at.is_none() {
                        self.schedule_reconnect();
                    }
                    return;
                }
            }
            let sequence = final_update_id;
            let ts_exchange_ns = extract_u64_field(payload, "E")
                .map(|ms| ms.saturating_mul(1_000_000))
                .unwrap_or_else(Self::now_ns);
            let mut accepted_depth_event = false;
            let mut candidate_depth_events = 0u64;
            let normalization_started = Instant::now();

            for (level, (price, size)) in extract_depth_pairs(payload, "b")
                .into_iter()
                .take(depth_limit)
                .enumerate()
            {
                candidate_depth_events = candidate_depth_events.saturating_add(1);
                accepted_depth_event |= self.push_event(RawEvent::Book(BookUpdate {
                    symbol: sym_id.clone(),
                    side: Side::Bid,
                    level: level as u16,
                    price,
                    size,
                    action: if size == 0 {
                        BookAction::Delete
                    } else {
                        BookAction::Upsert
                    },
                    sequence,
                    ts_exchange_ns,
                    ts_recv_ns,
                }));
            }
            for (level, (price, size)) in extract_depth_pairs(payload, "a")
                .into_iter()
                .take(depth_limit)
                .enumerate()
            {
                candidate_depth_events = candidate_depth_events.saturating_add(1);
                accepted_depth_event |= self.push_event(RawEvent::Book(BookUpdate {
                    symbol: sym_id.clone(),
                    side: Side::Ask,
                    level: level as u16,
                    price,
                    size,
                    action: if size == 0 {
                        BookAction::Delete
                    } else {
                        BookAction::Upsert
                    },
                    sequence,
                    ts_exchange_ns,
                    ts_recv_ns,
                }));
            }
            if candidate_depth_events != 0 {
                self.record_normalization_latency(normalization_started.elapsed());
            }
            self.last_market_data_at = Some(Instant::now());
            if accepted_depth_event {
                self.healthy_since.get_or_insert_with(Instant::now);
            }
        }
    }

    fn schedule_reconnect(&mut self) {
        self.reconnect_attempt = self.reconnect_attempt.saturating_add(1);
        let base_ms = 250u64;
        let max_ms = 5_000u64;
        let delay_ms = (base_ms.saturating_mul(1u64 << self.reconnect_attempt.min(5))).min(max_ms);
        self.next_reconnect_at = Some(Instant::now() + Duration::from_millis(delay_ms));
    }

    fn reconnect_if_due(&mut self) -> AdapterResult<()> {
        if self.is_mock_mode() {
            return Ok(());
        }
        let due = self
            .next_reconnect_at
            .map(|t| Instant::now() >= t)
            .unwrap_or(false);
        if !due {
            return Ok(());
        }

        match &mut self.transport {
            BinanceTransport::Live(ws) => ws.connect()?,
            BinanceTransport::Mock => return Ok(()),
        }
        self.connected = true;
        let existing: Vec<SymbolId> = self.subscribed.keys().cloned().collect();
        self.depth_state.clear();
        for sym in existing {
            self.send_binance_subscribe(&sym)?;
        }
        self.next_reconnect_at = None;
        self.last_message_at = Some(Instant::now());
        self.last_market_data_at = None;
        self.healthy_since = None;
        self.degraded = true;
        self.last_error = Some("binance reconnect warming".to_string());
        Ok(())
    }

    fn check_market_data_timeout(&mut self) {
        if self.is_mock_mode() || !self.connected || self.subscribed.is_empty() {
            return;
        }
        let now = Instant::now();
        let last = self
            .last_market_data_at
            .or(self.last_message_at)
            .unwrap_or(now);
        if now.duration_since(last) > Duration::from_secs(15) {
            self.connected = false;
            self.degraded = true;
            self.healthy_since = None;
            self.last_error = Some("binance market data timeout".to_string());
            if self.next_reconnect_at.is_none() {
                self.schedule_reconnect();
            }
        }
    }

    fn maybe_clear_degraded(&mut self) {
        if !self.degraded || !self.connected {
            return;
        }
        let now = Instant::now();
        let since = self.healthy_since.get_or_insert(now);
        if now.duration_since(*since) >= Duration::from_secs(2) {
            self.degraded = false;
            self.last_error = None;
        }
    }
}

impl MarketDataAdapter for BinanceAdapter {
    fn connect(&mut self) -> AdapterResult<()> {
        self.degraded = false;
        self.last_error = None;
        self.next_reconnect_at = None;
        self.reconnect_attempt = 0;
        match &mut self.transport {
            BinanceTransport::Mock => {
                self.connected = true;
            }
            BinanceTransport::Live(ws) => {
                if let Err(err) = ws.connect() {
                    self.connected = false;
                    self.degraded = true;
                    self.last_error = Some(err.to_string());
                    return Err(err);
                }
                self.connected = true;
                let existing: Vec<SymbolId> = self.subscribed.keys().cloned().collect();
                for sym in existing {
                    self.send_binance_subscribe(&sym)?;
                }
            }
        }
        self.last_message_at = Some(Instant::now());
        self.last_market_data_at = None;
        self.healthy_since = None;
        Ok(())
    }

    fn subscribe(&mut self, req: SubscribeReq) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        if req.depth_levels == 0 {
            self.subscribed.remove(&req.symbol);
            self.depth_state.remove(&req.symbol);
            self.send_binance_unsubscribe(&req.symbol)?;
            return Ok(());
        }

        self.subscribed.insert(req.symbol.clone(), req.depth_levels);
        self.depth_state.remove(&req.symbol);
        if self.is_mock_mode() {
            self.synth_trade(&req.symbol);
            return Ok(());
        }

        self.send_binance_subscribe(&req.symbol)
    }

    fn unsubscribe(&mut self, symbol: SymbolId) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        self.subscribed.remove(&symbol);
        self.depth_state.remove(&symbol);
        self.send_binance_unsubscribe(&symbol)
    }

    fn poll(&mut self, out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
        if !self.connected {
            self.reconnect_if_due()?;
            if !self.connected {
                return Err(AdapterError::Disconnected);
            }
        }

        if self.is_mock_mode() {
            let symbols: Vec<SymbolId> = self.subscribed.keys().cloned().collect();
            for s in symbols {
                self.synth_trade(&s);
            }
            self.last_market_data_at = Some(Instant::now());
        } else {
            self.check_market_data_timeout();
            if !self.connected {
                self.reconnect_if_due()?;
                if !self.connected {
                    return Err(AdapterError::Disconnected);
                }
            }
            let mut inbound = Vec::new();
            if let BinanceTransport::Live(ws) = &mut self.transport {
                loop {
                    match ws.recv_text() {
                        Ok(Some(msg)) => inbound.push(msg),
                        Ok(None) => break,
                        Err(e) => {
                            self.connected = false;
                            self.degraded = true;
                            self.healthy_since = None;
                            self.last_error = Some(e.to_string());
                            if self.next_reconnect_at.is_none() {
                                self.schedule_reconnect();
                            }
                            return Err(e);
                        }
                    }
                }
            }
            for msg in inbound {
                self.process_raw_message(&msg, Self::now_ns());
            }
            self.maybe_clear_degraded();
        }

        let n = self.queue.len();
        out.extend(self.queue.drain(..));
        Ok(n)
    }

    fn health(&self) -> AdapterHealth {
        let mode = if self.is_mock_mode() {
            "mock"
        } else {
            "live_ws"
        };
        let last_message_age_ms = self
            .last_message_at
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let last_market_data_age_ms = self
            .last_market_data_at
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let queue_depth = self.queue.len();
        let raw_capture_depth = self.raw_capture.len();
        let endpoint = redact_endpoint(&self.cfg.endpoint);
        let depth_update_ids = format_depth_update_ids(&self.depth_state);
        let parse_latency_avg_ns =
            average_ns(self.parse_latency_total_ns, self.parse_latency_samples);
        let normalization_latency_avg_ns = average_ns(
            self.normalization_latency_total_ns,
            self.normalization_latency_samples,
        );
        AdapterHealth {
            connected: self.connected
                && match &self.transport {
                    BinanceTransport::Mock => true,
                    BinanceTransport::Live(ws) => ws.is_connected(),
                },
            degraded: self.degraded,
            last_error: self.last_error.clone(),
            protocol_info: Some(format!(
                "provider=binance;market=crypto;mode={mode};endpoint={endpoint};reconnect_attempt={};subscribed={};messages_received={};normalized_events={};parse_errors={};dropped_events={};backpressure_events={};raw_capture_depth={raw_capture_depth};raw_capture_capacity={};raw_capture_dropped={};parse_latency_samples={};parse_latency_avg_ns={parse_latency_avg_ns};parse_latency_max_ns={};normalization_latency_samples={};normalization_latency_avg_ns={normalization_latency_avg_ns};normalization_latency_max_ns={};duplicate_depth_updates={};out_of_order_depth_updates={};gap_count={};snapshot_rebuild_count={};queue_depth={queue_depth};max_queue_depth={};last_update_ids={depth_update_ids};last_message_age_ms={last_message_age_ms};last_market_data_age_ms={last_market_data_age_ms}",
                self.reconnect_attempt,
                self.subscribed.len(),
                self.messages_received,
                self.normalized_events,
                self.parse_errors,
                self.dropped_events,
                self.backpressure_events,
                self.raw_capture_capacity,
                self.raw_capture_dropped,
                self.parse_latency_samples,
                self.parse_latency_max_ns,
                self.normalization_latency_samples,
                self.normalization_latency_max_ns,
                self.duplicate_depth_updates,
                self.out_of_order_depth_updates,
                self.gap_count,
                self.snapshot_rebuild_count,
                self.max_queue_depth
            )),
        }
    }
}

#[derive(Debug)]
struct ParsedEndpoint {
    scheme: String,
    host: String,
    port: u16,
    path: String,
}

impl ParsedEndpoint {
    fn parse(endpoint: &str) -> AdapterResult<Self> {
        let (scheme, rest) = endpoint
            .split_once("://")
            .ok_or(AdapterError::NotConfigured("invalid endpoint format"))?;
        let default_port = match scheme {
            "ws" => 80,
            "wss" => 443,
            _ => return Err(AdapterError::NotConfigured("unsupported endpoint scheme")),
        };
        let (authority, path) = if let Some((a, p)) = rest.split_once('/') {
            (a, format!("/{p}"))
        } else {
            (rest, "/ws".to_string())
        };
        let (host, port) = if let Some((h, p)) = authority.rsplit_once(':') {
            let parsed_port = p
                .parse::<u16>()
                .map_err(|_| AdapterError::NotConfigured("invalid endpoint port"))?;
            (h.to_string(), parsed_port)
        } else {
            (authority.to_string(), default_port)
        };
        if host.trim().is_empty() {
            return Err(AdapterError::NotConfigured("endpoint host is empty"));
        }
        Ok(Self {
            scheme: scheme.to_string(),
            host,
            port,
            path,
        })
    }
}

fn websocket_handshake(
    stream: &mut TcpStream,
    host: &str,
    port: u16,
    path: &str,
) -> AdapterResult<()> {
    let mut reader = stream
        .try_clone()
        .map_err(|e| AdapterError::Other(format!("tcp clone for handshake failed: {e}")))?;
    websocket_handshake_rw(stream, &mut reader, host, port, path)
}

fn websocket_handshake_rw<W: Write, R: Read>(
    writer: &mut W,
    reader: &mut R,
    host: &str,
    port: u16,
    path: &str,
) -> AdapterResult<()> {
    let host_header = if port == 80 || port == 443 {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nUser-Agent: orderflow/0.1\r\nOrigin: https://{}\r\n\r\n",
        path, host_header, host
    );
    writer
        .write_all(request.as_bytes())
        .map_err(|e| AdapterError::Other(format!("websocket handshake write failed: {e}")))?;
    writer
        .flush()
        .map_err(|e| AdapterError::Other(format!("websocket handshake flush failed: {e}")))?;

    let mut response = Vec::new();
    let mut buf = [0u8; 1];
    while !response.ends_with(b"\r\n\r\n") {
        let n = reader
            .read(&mut buf)
            .map_err(|e| AdapterError::Other(format!("websocket handshake read failed: {e}")))?;
        if n == 0 {
            break;
        }
        response.push(buf[0]);
        if response.len() > 16 * 1024 {
            return Err(AdapterError::Other(
                "websocket handshake response too large".to_string(),
            ));
        }
    }
    let text = String::from_utf8_lossy(&response);
    if !text.starts_with("HTTP/1.1 101") && !text.starts_with("HTTP/1.0 101") {
        return Err(AdapterError::Other(format!(
            "websocket upgrade failed: {}",
            text.lines().next().unwrap_or("<empty>")
        )));
    }
    Ok(())
}

fn spawn_text_ws_workers<W, R>(
    writer: W,
    reader: R,
    out_rx: Receiver<Outbound>,
    in_tx: Sender<String>,
    pong_tx: Sender<Outbound>,
) where
    W: Write + Send + 'static,
    R: Read + Send + 'static,
{
    let mut writer_owned = writer;
    let mut reader_owned = reader;
    let _ = thread::spawn(move || {
        while let Ok(msg) = out_rx.recv() {
            let frame = match msg {
                Outbound::Text(t) => encode_client_frame(0x1, t.as_bytes()),
                Outbound::Pong(p) => encode_client_frame(0xA, &p),
            };
            if writer_owned.write_all(&frame).is_err() {
                break;
            }
            let _ = writer_owned.flush();
        }
    });

    let _ = thread::spawn(move || loop {
        match read_ws_frame(&mut reader_owned) {
            Ok((0x1, payload)) => {
                if let Ok(text) = String::from_utf8(payload) {
                    let _ = in_tx.send(text);
                }
            }
            Ok((0x9, payload)) => {
                let _ = pong_tx.send(Outbound::Pong(payload));
            }
            Ok((0xA, _)) => {}
            Ok((0x8, _)) => break,
            Ok((_other, _payload)) => {}
            Err(_) => break,
        }
    });
}

fn encode_client_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let fin_opcode = 0x80u8 | (opcode & 0x0f);
    let mut out = vec![fin_opcode];
    let mask_key = [0x31u8, 0x41, 0x59, 0x26];

    if payload.len() <= 125 {
        out.push(0x80u8 | payload.len() as u8);
    } else if payload.len() <= 65535 {
        out.push(0x80u8 | 126u8);
        out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        out.push(0x80u8 | 127u8);
        out.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }

    out.extend_from_slice(&mask_key);
    for (i, b) in payload.iter().enumerate() {
        out.push(*b ^ mask_key[i % 4]);
    }
    out
}

fn read_ws_frame<R: Read>(reader: &mut R) -> Result<(u8, Vec<u8>), ()> {
    let mut hdr = [0u8; 2];
    reader.read_exact(&mut hdr).map_err(|_| ())?;

    let opcode = hdr[0] & 0x0f;
    let masked = (hdr[1] & 0x80) != 0;
    let mut len = (hdr[1] & 0x7f) as usize;

    if len == 126 {
        let mut b = [0u8; 2];
        reader.read_exact(&mut b).map_err(|_| ())?;
        len = u16::from_be_bytes(b) as usize;
    } else if len == 127 {
        let mut b = [0u8; 8];
        reader.read_exact(&mut b).map_err(|_| ())?;
        len = u64::from_be_bytes(b) as usize;
    }

    let mut mask = [0u8; 4];
    if masked {
        reader.read_exact(&mut mask).map_err(|_| ())?;
    }

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).map_err(|_| ())?;
    if masked {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
    }
    Ok((opcode, payload))
}

fn extract_data_object(raw: &str) -> Option<&str> {
    let key_pos = raw.find("\"data\"")?;
    let colon = raw[key_pos..].find(':')? + key_pos;
    let start_rel = raw[colon + 1..].find('{')?;
    let start = colon + 1 + start_rel;
    find_matching_brace_slice(raw, start)
}

fn find_matching_brace_slice(raw: &str, start: usize) -> Option<&str> {
    let bytes = raw.as_bytes();
    if bytes.get(start).copied()? != b'{' {
        return None;
    }
    let mut depth = 0i32;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return raw.get(start..=i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn extract_field_value<'a>(raw: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\"");
    let key_pos = raw.find(&pat)?;
    let after_key = &raw[key_pos + pat.len()..];
    let colon = after_key.find(':')?;
    let mut v = after_key[colon + 1..].trim_start();
    if let Some(stripped) = v.strip_prefix('"') {
        let end = stripped.find('"')?;
        return Some(&stripped[..end]);
    }
    if v.starts_with('[') {
        let mut depth = 0i32;
        let bytes = v.as_bytes();
        for (i, byte) in bytes.iter().enumerate() {
            match *byte {
                b'[' => depth += 1,
                b']' => {
                    depth -= 1;
                    if depth == 0 {
                        return v.get(..=i);
                    }
                }
                _ => {}
            }
        }
        return None;
    }
    let end = v
        .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
        .unwrap_or(v.len());
    v = &v[..end];
    Some(v.trim())
}

fn extract_string_field<'a>(raw: &'a str, key: &str) -> Option<&'a str> {
    extract_field_value(raw, key)
}

fn extract_u64_field(raw: &str, key: &str) -> Option<u64> {
    extract_field_value(raw, key)?.trim().parse::<u64>().ok()
}

fn extract_bool_field(raw: &str, key: &str) -> Option<bool> {
    match extract_field_value(raw, key)?.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_scaled_decimal(v: &str, scale: i64) -> Option<i64> {
    let s = v.trim();
    if s.is_empty() {
        return None;
    }
    let negative = s.starts_with('-');
    let abs = if negative { &s[1..] } else { s };
    let (whole, frac) = abs.split_once('.').unwrap_or((abs, ""));
    let whole_i = whole.parse::<i64>().ok()?;
    let mut frac_digits = frac.chars().take(12).collect::<String>();
    while frac_digits.len() < 12 {
        frac_digits.push('0');
    }
    let frac_i = frac_digits.parse::<i64>().ok()?;
    let scaled = whole_i.saturating_mul(scale)
        + frac_i
            .saturating_mul(scale)
            .saturating_div(1_000_000_000_000);
    Some(if negative { -scaled } else { scaled })
}

fn extract_depth_pairs(raw: &str, key: &str) -> Vec<(i64, i64)> {
    let arr = match extract_field_value(raw, key) {
        Some(v) => v,
        None => return Vec::new(),
    };
    let mut tokens = Vec::new();
    let bytes = arr.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            if j < bytes.len() {
                if let Some(tok) = arr.get(i + 1..j) {
                    tokens.push(tok.to_string());
                }
                i = j + 1;
                continue;
            }
            break;
        }
        i += 1;
    }
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx + 1 < tokens.len() {
        let price = parse_scaled_decimal(&tokens[idx], PRICE_SCALE).unwrap_or(0);
        let size = parse_scaled_decimal(&tokens[idx + 1], SIZE_SCALE).unwrap_or(0);
        out.push((price, size));
        idx += 2;
    }
    out
}

fn parse_agg_trade(raw: &str, seq: &mut u64, ts_recv_ns: u64) -> Option<TradePrint> {
    let symbol = extract_string_field(raw, "s")?.to_string();
    let price = parse_scaled_decimal(extract_string_field(raw, "p")?, PRICE_SCALE)?;
    let size = parse_scaled_decimal(extract_string_field(raw, "q")?, SIZE_SCALE).unwrap_or(1);
    let ts_exchange_ns = extract_u64_field(raw, "T")
        .map(|ms| ms.saturating_mul(1_000_000))
        .unwrap_or_else(BinanceAdapter::now_ns);
    let is_buyer_maker = extract_bool_field(raw, "m").unwrap_or(false);
    let aggressor_side = if is_buyer_maker { Side::Bid } else { Side::Ask };

    *seq = seq.saturating_add(1);
    Some(TradePrint {
        symbol: SymbolId {
            venue: "BINANCE".to_string(),
            symbol,
        },
        price,
        size: size.max(1),
        aggressor_side,
        sequence: *seq,
        ts_exchange_ns,
        ts_recv_ns,
    })
}

fn duration_ns(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX)
}

fn average_ns(total_ns: u64, samples: u64) -> u64 {
    total_ns.checked_div(samples).unwrap_or(0)
}

fn redact_endpoint(endpoint: &str) -> String {
    let Some((scheme, rest)) = endpoint.split_once("://") else {
        return "<redacted>".to_string();
    };
    let authority_and_path = rest.split_once('?').map_or(rest, |(before, _)| before);
    let authority_and_path = authority_and_path
        .split_once('#')
        .map_or(authority_and_path, |(before, _)| before);
    let without_userinfo = authority_and_path
        .rsplit_once('@')
        .map_or(authority_and_path, |(_, after)| after);
    format!("{scheme}://{without_userinfo}")
}

fn format_depth_update_ids(depth_state: &HashMap<SymbolId, BinanceDepthState>) -> String {
    if depth_state.is_empty() {
        return "-".to_string();
    }
    let mut entries = depth_state
        .iter()
        .filter_map(|(symbol, state)| {
            state
                .last_update_id
                .map(|last| format!("{}:{}", symbol.symbol, last))
        })
        .collect::<Vec<_>>();
    entries.sort_unstable();
    entries.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AdapterConfig, ProviderKind};

    fn cfg(endpoint: &str) -> AdapterConfig {
        AdapterConfig {
            provider: ProviderKind::Binance,
            credentials: None,
            endpoint: Some(endpoint.to_string()),
            app_name: Some("orderflow-tests".to_string()),
        }
    }

    #[test]
    fn connects_and_streams_mock_crypto() {
        let mut adapter = BinanceAdapter::from_config(&cfg("mock://binance")).expect("cfg");
        adapter.connect().expect("connect");
        adapter
            .subscribe(SubscribeReq {
                symbol: SymbolId {
                    venue: "BINANCE".to_string(),
                    symbol: "BTCUSDT".to_string(),
                },
                depth_levels: 20,
            })
            .expect("sub");
        let mut out = Vec::new();
        let n = adapter.poll(&mut out).expect("poll");
        assert!(n > 0);
    }

    #[test]
    fn parses_agg_trade_payload() {
        let raw = r#"{"e":"aggTrade","E":1710000000123,"s":"BTCUSDT","a":1,"p":"66107.98000000","q":"0.01200000","f":1,"l":1,"T":1710000000001,"m":true,"M":true}"#;
        let mut seq = 0;
        let trade = parse_agg_trade(raw, &mut seq, 123).expect("trade");
        assert_eq!(trade.symbol.symbol, "BTCUSDT");
        assert_eq!(trade.price, 66_107_980_000);
        assert_eq!(trade.size, 12);
        assert_eq!(trade.aggressor_side, Side::Bid);
        assert_eq!(trade.ts_recv_ns, 123);
    }

    #[test]
    fn parses_depth_pairs() {
        let raw = r#"{"e":"depthUpdate","E":1710000000123,"s":"BTCUSDT","U":157,"u":160,"b":[["66107.97","1.99161"],["66107.96","0.10000"]],"a":[["66107.98","1.83166"]]}"#;
        let bids = extract_depth_pairs(raw, "b");
        let asks = extract_depth_pairs(raw, "a");
        assert_eq!(bids.len(), 2);
        assert_eq!(asks.len(), 1);
        assert_eq!(bids[0].0, 66_107_970_000);
        assert_eq!(asks[0].0, 66_107_980_000);
    }

    #[test]
    fn extracts_combined_stream_data_object() {
        let wrapped = r#"{"stream":"btcusdt@aggTrade","data":{"e":"aggTrade","s":"BTCUSDT","p":"1.00","q":"2.00","T":1,"m":false}}"#;
        let data = extract_data_object(wrapped).expect("data");
        assert!(data.contains("\"e\":\"aggTrade\""));
    }

    #[test]
    fn redacts_endpoint_userinfo_and_query_from_health() {
        let endpoint = "wss://user:secret-token@test.live/ws?listenKey=super-secret#frag";
        let adapter = BinanceAdapter::from_config(&cfg(endpoint)).expect("cfg");

        let protocol = adapter.health().protocol_info.unwrap_or_default();

        assert!(protocol.contains("endpoint=wss://test.live/ws"));
        assert!(!protocol.contains("secret-token"));
        assert!(!protocol.contains("listenKey"));
        assert!(!protocol.contains("super-secret"));
        assert_eq!(
            redact_endpoint("wss://user:secret-token@test.live/ws?listenKey=super-secret#frag"),
            "wss://test.live/ws"
        );
    }

    #[test]
    fn formats_depth_update_ids_deterministically() {
        let mut depth_state = HashMap::new();
        depth_state.insert(
            SymbolId {
                venue: "BINANCE".to_string(),
                symbol: "ETHUSDT".to_string(),
            },
            BinanceDepthState {
                last_update_id: Some(42),
            },
        );
        depth_state.insert(
            SymbolId {
                venue: "BINANCE".to_string(),
                symbol: "BTCUSDT".to_string(),
            },
            BinanceDepthState {
                last_update_id: Some(7),
            },
        );

        assert_eq!(
            format_depth_update_ids(&depth_state),
            "BTCUSDT:7,ETHUSDT:42"
        );
    }

    #[test]
    fn live_mode_parses_trade_depth_and_ack_messages() {
        let mut adapter = BinanceAdapter::from_config(&cfg("ws://test.live/ws")).expect("cfg");
        adapter.connect().expect("connect");
        let symbol = SymbolId {
            venue: "BINANCE".to_string(),
            symbol: "BTCUSDT".to_string(),
        };
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 5,
            })
            .expect("sub");

        adapter.degraded = true;
        adapter.last_error = Some("warming".to_string());
        if let BinanceTransport::Live(ws) = &mut adapter.transport {
            ws.inject_text(r#"{"result":null,"id":1}"#);
            ws.inject_text(r#"{"e":"aggTrade","E":1710000000123,"s":"BTCUSDT","a":1,"p":"66107.98000000","q":"0.01200000","f":1,"l":1,"T":1710000000001,"m":true,"M":true}"#);
            ws.inject_text(r#"{"e":"depthUpdate","E":1710000000123,"s":"BTCUSDT","U":157,"u":160,"b":[["66107.97","1.99161"]],"a":[["66107.98","1.83166"]]}"#);
        }

        let mut out = Vec::new();
        let n = adapter.poll(&mut out).expect("poll");
        assert!(n >= 3);
        assert!(out.iter().any(|ev| matches!(ev, RawEvent::Trade(_))));
        assert!(out.iter().any(|ev| matches!(ev, RawEvent::Book(_))));
        assert!(adapter
            .health()
            .protocol_info
            .unwrap_or_default()
            .contains("subscribed=1"));
        assert!(adapter
            .health()
            .protocol_info
            .unwrap_or_default()
            .contains("last_update_ids=BTCUSDT:160"));
        let protocol = adapter.health().protocol_info.unwrap_or_default();
        assert!(protocol.contains("parse_latency_samples=3"));
        assert!(protocol.contains("parse_latency_avg_ns="));
        assert!(protocol.contains("parse_latency_max_ns="));
        assert!(protocol.contains("normalization_latency_samples=2"));
        assert!(protocol.contains("normalization_latency_avg_ns="));
        assert!(protocol.contains("normalization_latency_max_ns="));
    }

    #[test]
    fn live_mode_sheds_events_when_queue_bound_is_reached() {
        let mut adapter = BinanceAdapter::from_config(&cfg("ws://test.live/ws"))
            .expect("cfg")
            .with_max_queue_depth(1);
        assert_eq!(adapter.max_queue_depth(), 1);
        adapter.set_max_queue_depth(1);

        adapter.connect().expect("connect");
        adapter
            .subscribe(SubscribeReq {
                symbol: SymbolId {
                    venue: "BINANCE".to_string(),
                    symbol: "BTCUSDT".to_string(),
                },
                depth_levels: 5,
            })
            .expect("sub");

        if let BinanceTransport::Live(ws) = &mut adapter.transport {
            ws.inject_text(r#"{"e":"aggTrade","E":1710000000123,"s":"BTCUSDT","a":1,"p":"66107.98000000","q":"0.01200000","f":1,"l":1,"T":1710000000001,"m":true,"M":true}"#);
            ws.inject_text(r#"{"e":"depthUpdate","E":1710000000124,"s":"BTCUSDT","U":157,"u":160,"b":[["66107.97","1.99161"]],"a":[["66107.98","1.83166"]]}"#);
        }

        let mut out = Vec::new();
        let n = adapter.poll(&mut out).expect("poll");
        let health = adapter.health();
        let protocol = health.protocol_info.unwrap_or_default();

        assert_eq!(n, 1);
        assert_eq!(out.len(), 1);
        assert!(matches!(out[0], RawEvent::Trade(_)));
        assert!(health.degraded);
        assert!(protocol.contains("normalized_events=1"));
        assert!(protocol.contains("dropped_events=2"));
        assert!(protocol.contains("backpressure_events=2"));
        assert!(protocol.contains("queue_depth=0"));
        assert!(protocol.contains("max_queue_depth=1"));
        assert!(adapter
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("binance queue backpressure"));
    }

    #[test]
    fn live_mode_captures_raw_messages_with_bounded_capacity() {
        let mut adapter = BinanceAdapter::from_config(&cfg("ws://test.live/ws"))
            .expect("cfg")
            .with_raw_capture_capacity(2);
        assert_eq!(adapter.raw_capture_capacity(), 2);
        adapter.set_raw_capture_capacity(2);

        adapter.connect().expect("connect");
        if let BinanceTransport::Live(ws) = &mut adapter.transport {
            ws.inject_text(r#"{"result":null,"id":1}"#);
            ws.inject_text(r#"{"result":null,"id":2}"#);
            ws.inject_text(r#"{"result":null,"id":3}"#);
        }

        let mut out = Vec::new();
        let n = adapter.poll(&mut out).expect("poll");
        let protocol = adapter.health().protocol_info.unwrap_or_default();

        assert_eq!(n, 0);
        assert_eq!(adapter.raw_capture_len(), 2);
        assert_eq!(adapter.raw_capture_dropped(), 1);
        assert!(protocol.contains("raw_capture_depth=2"));
        assert!(protocol.contains("raw_capture_capacity=2"));
        assert!(protocol.contains("raw_capture_dropped=1"));

        let mut raw = Vec::new();
        assert_eq!(adapter.drain_raw_messages(&mut raw), 2);
        assert_eq!(adapter.raw_capture_len(), 0);
        assert!(raw[0].contains("\"id\":2"));
        assert!(raw[1].contains("\"id\":3"));

        adapter.set_raw_capture_capacity(0);
        assert_eq!(adapter.raw_capture_capacity(), 0);
    }

    #[test]
    fn fixture_replay_uses_live_parser_with_deterministic_receive_time() {
        let fixtures = [
            r#"{"e":"aggTrade","E":1710000000123,"s":"BTCUSDT","a":1,"p":"66107.98000000","q":"0.01200000","f":1,"l":1,"T":1710000000001,"m":true,"M":true}"#,
            r#"{"e":"depthUpdate","E":1710000000124,"s":"BTCUSDT","U":157,"u":160,"b":[["66107.97","1.99161"]],"a":[["66107.98","1.83166"]]}"#,
        ];
        let mut first = BinanceAdapter::from_config(&cfg("mock://binance"))
            .expect("cfg")
            .with_raw_capture_capacity(4);
        let mut second = BinanceAdapter::from_config(&cfg("mock://binance")).expect("cfg");
        let mut out_first = Vec::new();
        let mut out_second = Vec::new();

        assert_eq!(first.replay_raw_messages(&fixtures, &mut out_first), 3);
        assert_eq!(second.replay_raw_messages(&fixtures, &mut out_second), 3);

        assert_eq!(format!("{out_first:?}"), format!("{out_second:?}"));
        assert!(matches!(out_first[0], RawEvent::Trade(_)));
        assert!(out_first[1..]
            .iter()
            .all(|event| matches!(event, RawEvent::Book(_))));
        for event in &out_first {
            match event {
                RawEvent::Trade(trade) => assert_eq!(trade.ts_recv_ns, REPLAY_RECV_TS_NS),
                RawEvent::Book(book) => assert_eq!(book.ts_recv_ns, REPLAY_RECV_TS_NS),
            }
        }
        assert_eq!(first.raw_capture_len(), 2);
        assert_eq!(first.raw_capture_dropped(), 0);
        assert!(first
            .health()
            .protocol_info
            .unwrap_or_default()
            .contains("parse_latency_samples=2"));
    }

    #[test]
    fn live_mode_drops_duplicate_depth_updates() {
        let mut adapter = BinanceAdapter::from_config(&cfg("ws://test.live/ws")).expect("cfg");
        adapter.connect().expect("connect");
        let symbol = SymbolId {
            venue: "BINANCE".to_string(),
            symbol: "BTCUSDT".to_string(),
        };
        adapter
            .subscribe(SubscribeReq {
                symbol,
                depth_levels: 5,
            })
            .expect("sub");

        if let BinanceTransport::Live(ws) = &mut adapter.transport {
            ws.inject_text(r#"{"e":"depthUpdate","E":1710000000123,"s":"BTCUSDT","U":157,"u":160,"b":[["66107.97","1.99161"]],"a":[]}"#);
            ws.inject_text(r#"{"e":"depthUpdate","E":1710000000124,"s":"BTCUSDT","U":157,"u":160,"b":[["66107.96","2.00000"]],"a":[]}"#);
        }

        let mut out = Vec::new();
        adapter.poll(&mut out).expect("poll");
        let protocol = adapter.health().protocol_info.unwrap_or_default();

        assert_eq!(
            out.iter()
                .filter(|event| matches!(event, RawEvent::Book(_)))
                .count(),
            1
        );
        assert!(protocol.contains("duplicate_depth_updates=1"));
        assert!(protocol.contains("gap_count=0"));
    }

    #[test]
    fn live_mode_detects_depth_update_gap_from_previous_update_id() {
        let mut adapter = BinanceAdapter::from_config(&cfg("ws://test.live/ws")).expect("cfg");
        adapter.connect().expect("connect");
        let symbol = SymbolId {
            venue: "BINANCE".to_string(),
            symbol: "BTCUSDT".to_string(),
        };
        adapter
            .subscribe(SubscribeReq {
                symbol,
                depth_levels: 5,
            })
            .expect("sub");

        if let BinanceTransport::Live(ws) = &mut adapter.transport {
            ws.inject_text(r#"{"e":"depthUpdate","E":1710000000123,"s":"BTCUSDT","U":157,"u":160,"b":[["66107.97","1.99161"]],"a":[]}"#);
            ws.inject_text(r#"{"e":"depthUpdate","E":1710000000124,"s":"BTCUSDT","U":170,"u":175,"pu":169,"b":[["66107.96","2.00000"]],"a":[]}"#);
        }

        let mut out = Vec::new();
        adapter.poll(&mut out).expect("poll");
        let health = adapter.health();
        let protocol = health.protocol_info.unwrap_or_default();

        assert!(health.degraded);
        assert!(adapter.next_reconnect_at.is_some());
        assert!(protocol.contains("gap_count=1"));
        assert!(protocol.contains("snapshot_rebuild_count=1"));
        assert!(adapter
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("binance depth gap"));
        assert_eq!(
            out.iter()
                .filter(|event| matches!(event, RawEvent::Book(_)))
                .count(),
            1
        );
    }

    #[test]
    fn depth_state_accepts_contiguous_previous_update_id() {
        let mut state = BinanceDepthState::default();

        assert_eq!(state.classify(157, 160, None), BinanceDepthDecision::Apply);
        assert_eq!(
            state.classify(161, 165, Some(160)),
            BinanceDepthDecision::Apply
        );
        assert_eq!(
            state.classify(164, 166, None),
            BinanceDepthDecision::ApplyOutOfOrder
        );
        assert_eq!(
            state.classify(161, 165, Some(160)),
            BinanceDepthDecision::Duplicate
        );
        assert_eq!(
            state.classify(180, 185, Some(179)),
            BinanceDepthDecision::Gap
        );
    }

    #[test]
    fn market_data_timeout_marks_live_path_degraded() {
        let mut adapter = BinanceAdapter::from_config(&cfg("ws://test.live/ws")).expect("cfg");
        adapter.connect().expect("connect");
        adapter.subscribed.insert(
            SymbolId {
                venue: "BINANCE".to_string(),
                symbol: "BTCUSDT".to_string(),
            },
            5,
        );
        adapter.last_market_data_at = Some(Instant::now() - Duration::from_secs(20));
        adapter.last_message_at = adapter.last_market_data_at;

        let mut out = Vec::new();
        let err = adapter
            .poll(&mut out)
            .expect_err("timeout should disconnect");
        assert!(matches!(err, AdapterError::Disconnected));
        assert!(adapter.health().degraded);
        assert!(adapter.next_reconnect_at.is_some());
    }

    #[test]
    fn live_disconnect_schedules_and_recovers_with_reconnect() {
        let mut adapter = BinanceAdapter::from_config(&cfg("ws://test.live/ws")).expect("cfg");
        adapter.connect().expect("connect");
        let symbol = SymbolId {
            venue: "BINANCE".to_string(),
            symbol: "BTCUSDT".to_string(),
        };
        adapter
            .subscribe(SubscribeReq {
                symbol: symbol.clone(),
                depth_levels: 5,
            })
            .expect("sub");

        if let BinanceTransport::Live(ws) = &mut adapter.transport {
            ws.force_disconnect();
        }
        let mut out = Vec::new();
        let err = adapter
            .poll(&mut out)
            .expect_err("disconnect should surface");
        assert!(matches!(err, AdapterError::Disconnected));
        assert!(adapter.next_reconnect_at.is_some());

        adapter.next_reconnect_at = Some(Instant::now());
        adapter.poll(&mut out).expect("reconnect poll");
        assert!(adapter.health().connected);

        if let BinanceTransport::Live(ws) = &mut adapter.transport {
            ws.inject_text(r#"{"e":"aggTrade","E":1710000000123,"s":"BTCUSDT","a":1,"p":"66107.98000000","q":"0.01200000","f":1,"l":1,"T":1710000000001,"m":true,"M":true}"#);
        }
        let mut recovered = Vec::new();
        let n = adapter.poll(&mut recovered).expect("post reconnect poll");
        assert!(n > 0);
        assert!(recovered.iter().any(|ev| matches!(ev, RawEvent::Trade(_))));
    }
}
