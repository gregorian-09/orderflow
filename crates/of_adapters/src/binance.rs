use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use of_core::{BookAction, BookUpdate, Side, SymbolId, TradePrint};

use crate::{
    AdapterConfig, AdapterError, AdapterHealth, AdapterResult, MarketDataAdapter, RawEvent,
    SubscribeReq,
};

const PRICE_SCALE: i64 = 1_000_000;
const SIZE_SCALE: i64 = 1_000;

#[derive(Debug, Clone)]
pub struct BinanceConfig {
    endpoint: String,
}

impl BinanceConfig {
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
}

impl WsTextTransport {
    fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            connected: false,
            outbound_tx: None,
            inbound_rx: None,
        }
    }

    fn connect(&mut self) -> AdapterResult<()> {
        let parsed = ParsedEndpoint::parse(&self.endpoint)?;
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
                spawn_text_ws_workers(writer, stream, out_rx, in_tx, out_tx.clone());
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
                let mut stdout = child
                    .stdout
                    .take()
                    .ok_or(AdapterError::Other("openssl stdout unavailable".to_string()))?;

                websocket_handshake_rw(
                    &mut stdin,
                    &mut stdout,
                    &parsed.host,
                    parsed.port,
                    &parsed.path,
                )?;
                spawn_text_ws_workers(stdin, stdout, out_rx, in_tx, out_tx.clone());
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
        Ok(())
    }

    fn send_text(&mut self, text: String) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        let tx = self.outbound_tx.as_ref().ok_or(AdapterError::Disconnected)?;
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
}

#[derive(Debug)]
pub struct BinanceAdapter {
    cfg: BinanceConfig,
    transport: BinanceTransport,
    connected: bool,
    degraded: bool,
    last_error: Option<String>,
    subscribed: HashMap<SymbolId, u16>,
    queue: VecDeque<RawEvent>,
    seq: u64,
    request_id: u64,
}

impl BinanceAdapter {
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
            queue: VecDeque::new(),
            seq: 0,
            request_id: 0,
        })
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
        self.queue.push_back(RawEvent::Trade(TradePrint {
            symbol: symbol.clone(),
            price: base + (self.seq % 25) as i64 * 10_000,
            size: 1 + (self.seq % 3) as i64,
            aggressor_side: if self.seq % 2 == 0 { Side::Ask } else { Side::Bid },
            sequence: self.seq,
            ts_exchange_ns: Self::now_ns(),
            ts_recv_ns: Self::now_ns(),
        }));
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

    fn parse_live_message(&mut self, msg: &str) {
        let payload = extract_data_object(msg).unwrap_or(msg);
        if payload.contains("\"e\":\"aggTrade\"") {
            if let Some(trade) = parse_agg_trade(payload, &mut self.seq) {
                self.queue.push_back(RawEvent::Trade(trade));
            }
            return;
        }

        if payload.contains("\"e\":\"depthUpdate\"") {
            let symbol = match extract_string_field(payload, "s") {
                Some(s) => s.to_string(),
                None => return,
            };
            let sym_id = SymbolId {
                venue: "BINANCE".to_string(),
                symbol: symbol,
            };
            let depth_limit = self.subscribed.get(&sym_id).copied().unwrap_or(10) as usize;
            let sequence = extract_u64_field(payload, "u").unwrap_or_else(|| {
                self.seq = self.seq.saturating_add(1);
                self.seq
            });
            let ts_exchange_ns = extract_u64_field(payload, "E")
                .map(|ms| ms.saturating_mul(1_000_000))
                .unwrap_or_else(Self::now_ns);
            let ts_recv_ns = Self::now_ns();

            for (level, (price, size)) in extract_depth_pairs(payload, "b")
                .into_iter()
                .take(depth_limit)
                .enumerate()
            {
                self.queue.push_back(RawEvent::Book(BookUpdate {
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
                self.queue.push_back(RawEvent::Book(BookUpdate {
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
        }
    }
}

impl MarketDataAdapter for BinanceAdapter {
    fn connect(&mut self) -> AdapterResult<()> {
        self.degraded = false;
        self.last_error = None;
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
        Ok(())
    }

    fn subscribe(&mut self, req: SubscribeReq) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        if req.depth_levels == 0 {
            self.subscribed.remove(&req.symbol);
            self.send_binance_unsubscribe(&req.symbol)?;
            return Ok(());
        }

        self.subscribed.insert(req.symbol.clone(), req.depth_levels);
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
        self.send_binance_unsubscribe(&symbol)
    }

    fn poll(&mut self, out: &mut Vec<RawEvent>) -> AdapterResult<usize> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }

        if self.is_mock_mode() {
            let symbols: Vec<SymbolId> = self.subscribed.keys().cloned().collect();
            for s in symbols {
                self.synth_trade(&s);
            }
        } else if let BinanceTransport::Live(ws) = &mut self.transport {
            let mut inbound = Vec::new();
            loop {
                match ws.recv_text() {
                    Ok(Some(msg)) => inbound.push(msg),
                    Ok(None) => break,
                    Err(e) => {
                        self.connected = false;
                        self.degraded = true;
                        self.last_error = Some(e.to_string());
                        return Err(e);
                    }
                }
            }
            for msg in inbound {
                self.parse_live_message(&msg);
            }
        }

        let n = self.queue.len();
        out.extend(self.queue.drain(..));
        Ok(n)
    }

    fn health(&self) -> AdapterHealth {
        let mode = if self.is_mock_mode() { "mock" } else { "live_ws" };
        AdapterHealth {
            connected: self.connected
                && match &self.transport {
                    BinanceTransport::Mock => true,
                    BinanceTransport::Live(ws) => ws.is_connected(),
                },
            degraded: self.degraded,
            last_error: self.last_error.clone(),
            protocol_info: Some(format!(
                "provider=binance;market=crypto;mode={mode};endpoint={}",
                self.cfg.endpoint
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
        for i in 0..bytes.len() {
            match bytes[i] {
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
    let scaled =
        whole_i.saturating_mul(scale) + frac_i.saturating_mul(scale).saturating_div(1_000_000_000_000);
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

fn parse_agg_trade(raw: &str, seq: &mut u64) -> Option<TradePrint> {
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
        ts_recv_ns: BinanceAdapter::now_ns(),
    })
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
        let trade = parse_agg_trade(raw, &mut seq).expect("trade");
        assert_eq!(trade.symbol.symbol, "BTCUSDT");
        assert_eq!(trade.price, 66_107_980_000);
        assert_eq!(trade.size, 12);
        assert_eq!(trade.aggressor_side, Side::Bid);
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
}
