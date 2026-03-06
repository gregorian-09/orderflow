use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use super::proto::{encode_inbound_for_test, is_ping_outbound_frame, CqgInbound};
use crate::{AdapterError, AdapterResult};

pub trait CqgTransport: Send + std::fmt::Debug {
    fn connect(&mut self) -> AdapterResult<()>;
    fn send_frame(&mut self, frame: Vec<u8>) -> AdapterResult<()>;
    fn recv_next_frame(&mut self) -> AdapterResult<Option<Vec<u8>>>;
    fn is_connected(&self) -> bool;
    fn inject_test_frame(&mut self, _frame: Vec<u8>) {}
    fn force_disconnect(&mut self) {}
}

#[derive(Debug, Default)]
pub struct MockTransport {
    connected: bool,
    pub sent_frames: Vec<Vec<u8>>,
    recv_frames: VecDeque<Vec<u8>>,
}

impl CqgTransport for MockTransport {
    fn connect(&mut self) -> AdapterResult<()> {
        self.connected = true;
        Ok(())
    }

    fn send_frame(&mut self, frame: Vec<u8>) -> AdapterResult<()> {
        self.sent_frames.push(frame);
        Ok(())
    }

    fn recv_next_frame(&mut self) -> AdapterResult<Option<Vec<u8>>> {
        Ok(self.recv_frames.pop_front())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn inject_test_frame(&mut self, frame: Vec<u8>) {
        self.recv_frames.push_back(frame);
    }

    fn force_disconnect(&mut self) {
        self.connected = false;
    }
}

#[derive(Debug)]
pub struct WsProtobufTransport {
    endpoint: String,
    connected: bool,
    outbound_tx: Option<Sender<Vec<u8>>>,
    inbound_rx: Option<Receiver<Vec<u8>>>,
}

impl WsProtobufTransport {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            connected: false,
            outbound_tx: None,
            inbound_rx: None,
        }
    }

    fn connect_simulated(&mut self) {
        let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>();
        let (in_tx, in_rx) = mpsc::channel::<Vec<u8>>();

        let _ = thread::spawn(move || {
            while let Ok(payload) = out_rx.recv() {
                if is_ping_outbound_frame(&payload) {
                    let _ = in_tx.send(encode_inbound_for_test(&CqgInbound::Heartbeat));
                }
            }
        });

        self.outbound_tx = Some(out_tx);
        self.inbound_rx = Some(in_rx);
        self.connected = true;
    }

    fn connect_live(&mut self) -> AdapterResult<()> {
        let parsed = ParsedEndpoint::parse(&self.endpoint)?;
        let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>();
        let (in_tx, in_rx) = mpsc::channel::<Vec<u8>>();

        match parsed.scheme.as_str() {
            "ws" => {
                let mut stream = TcpStream::connect((parsed.host.as_str(), parsed.port))
                    .map_err(|e| AdapterError::Other(format!("ws tcp connect failed: {e}")))?;
                let _ = stream.set_nodelay(true);
                websocket_handshake(
                    &mut stream,
                    &parsed.host,
                    &parsed.path,
                )?;

                let writer = stream
                    .try_clone()
                    .map_err(|e| AdapterError::Other(format!("ws clone failed: {e}")))?;
                let reader = stream;

                spawn_ws_workers(writer, reader, out_rx, in_tx);
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

                let stdin = child
                    .stdin
                    .take()
                    .ok_or(AdapterError::Other("openssl stdin unavailable".to_string()))?;
                let stdout = child
                    .stdout
                    .take()
                    .ok_or(AdapterError::Other("openssl stdout unavailable".to_string()))?;

                let mut hs_w = stdin;
                let mut hs_r = stdout;
                websocket_handshake_rw(&mut hs_w, &mut hs_r, &parsed.host, &parsed.path)?;

                spawn_ws_workers(hs_w, hs_r, out_rx, in_tx);
                let _ = thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            _ => {
                return Err(AdapterError::NotConfigured(
                    "ws transport requires ws:// or wss:// endpoint",
                ))
            }
        }

        self.outbound_tx = Some(out_tx);
        self.inbound_rx = Some(in_rx);
        self.connected = true;
        Ok(())
    }
}

impl CqgTransport for WsProtobufTransport {
    fn connect(&mut self) -> AdapterResult<()> {
        if !self.endpoint.starts_with("wss://") && !self.endpoint.starts_with("ws://") {
            return Err(AdapterError::NotConfigured(
                "ws transport requires ws:// or wss:// endpoint",
            ));
        }

        // Live mode is opt-in to keep CI/tests deterministic and offline-safe.
        if std::env::var("OF_ENABLE_LIVE_WS").ok().as_deref() == Some("1") {
            self.connect_live()
        } else {
            self.connect_simulated();
            Ok(())
        }
    }

    fn send_frame(&mut self, frame: Vec<u8>) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        match &self.outbound_tx {
            Some(tx) => tx
                .send(frame)
                .map_err(|_| AdapterError::Other("transport send failed".to_string())),
            None => Err(AdapterError::Disconnected),
        }
    }

    fn recv_next_frame(&mut self) -> AdapterResult<Option<Vec<u8>>> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }

        match &self.inbound_rx {
            Some(rx) => match rx.try_recv() {
                Ok(frame) => Ok(Some(frame)),
                Err(TryRecvError::Empty) => Ok(None),
                Err(TryRecvError::Disconnected) => {
                    self.connected = false;
                    Err(AdapterError::Disconnected)
                }
            },
            None => Err(AdapterError::Disconnected),
        }
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn force_disconnect(&mut self) {
        self.connected = false;
        self.outbound_tx = None;
        self.inbound_rx = None;
    }
}

fn spawn_ws_workers<W, R>(writer: W, reader: R, out_rx: Receiver<Vec<u8>>, in_tx: Sender<Vec<u8>>)
where
    W: Write + Send + 'static,
    R: Read + Send + 'static,
{
    let mut writer_owned = writer;
    let mut reader_owned = reader;

    let _ = thread::spawn(move || {
        while let Ok(payload) = out_rx.recv() {
            let ws_frame = encode_client_binary_frame(&payload);
            if writer_owned.write_all(&ws_frame).is_err() {
                break;
            }
            let _ = writer_owned.flush();
        }
    });

    let _ = thread::spawn(move || loop {
        match read_ws_frame(&mut reader_owned) {
            Ok(payload) => {
                let _ = in_tx.send(payload);
            }
            Err(_) => break,
        }
    });

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
            (rest, "/".to_string())
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

fn websocket_handshake(stream: &mut TcpStream, host: &str, path: &str) -> AdapterResult<()> {
    let mut reader = stream
        .try_clone()
        .map_err(|e| AdapterError::Other(format!("tcp clone for handshake failed: {e}")))?;
    websocket_handshake_rw(stream, &mut reader, host, path)
}

fn websocket_handshake_rw<W: Write, R: Read>(
    writer: &mut W,
    reader: &mut R,
    host: &str,
    path: &str,
) -> AdapterResult<()> {
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
        path, host
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

fn encode_client_binary_frame(payload: &[u8]) -> Vec<u8> {
    let fin_opcode = 0x80u8 | 0x02u8; // FIN + binary
    let mut out = vec![fin_opcode];

    let mask_key = [0x12u8, 0x34, 0x56, 0x78];
    if payload.len() <= 125 {
        out.push(0x80u8 | payload.len() as u8); // masked
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

fn read_ws_frame<R: Read>(reader: &mut R) -> Result<Vec<u8>, ()> {
    let mut hdr = [0u8; 2];
    reader.read_exact(&mut hdr).map_err(|_| ())?;

    let opcode = hdr[0] & 0x0f;
    if opcode != 0x02 {
        return Err(());
    }

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

    Ok(payload)
}

#[cfg(test)]
fn decode_server_binary_frame(frame: &[u8]) -> Option<Vec<u8>> {
    if frame.len() < 2 {
        return None;
    }

    let opcode = frame[0] & 0x0f;
    if opcode != 0x02 {
        return None;
    }

    let masked = (frame[1] & 0x80) != 0;
    let mut idx = 2usize;
    let mut len = (frame[1] & 0x7f) as usize;

    if len == 126 {
        if frame.len() < idx + 2 {
            return None;
        }
        len = u16::from_be_bytes([frame[idx], frame[idx + 1]]) as usize;
        idx += 2;
    } else if len == 127 {
        if frame.len() < idx + 8 {
            return None;
        }
        len = u64::from_be_bytes([
            frame[idx],
            frame[idx + 1],
            frame[idx + 2],
            frame[idx + 3],
            frame[idx + 4],
            frame[idx + 5],
            frame[idx + 6],
            frame[idx + 7],
        ]) as usize;
        idx += 8;
    }

    let mask_key = if masked {
        if frame.len() < idx + 4 {
            return None;
        }
        let key = [frame[idx], frame[idx + 1], frame[idx + 2], frame[idx + 3]];
        idx += 4;
        Some(key)
    } else {
        None
    };

    if frame.len() < idx + len {
        return None;
    }

    let mut payload = frame[idx..idx + len].to_vec();
    if let Some(key) = mask_key {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= key[i % 4];
        }
    }

    Some(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_frame_roundtrip() {
        let payload = b"OUT|PING".to_vec();
        let frame = encode_client_binary_frame(&payload);
        let decoded = decode_server_binary_frame(&frame).expect("decode");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn parse_ws_endpoint() {
        let p = ParsedEndpoint::parse("wss://demoapi.cqg.com:443/feed").expect("parse");
        assert_eq!(p.scheme, "wss");
        assert_eq!(p.host, "demoapi.cqg.com");
        assert_eq!(p.port, 443);
        assert_eq!(p.path, "/feed");
    }
}
