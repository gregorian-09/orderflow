use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use crate::{AdapterError, AdapterResult};

#[derive(Debug, Clone)]
pub(crate) enum WebSocketOutbound {
    Text(String),
    Pong(Vec<u8>),
}

#[derive(Debug)]
pub(crate) struct TextWebSocket {
    endpoint: String,
    connected: bool,
    outbound_tx: Option<Sender<WebSocketOutbound>>,
    inbound_rx: Option<Receiver<String>>,
    inbound_tx: Option<Sender<String>>,
}

impl TextWebSocket {
    pub(crate) fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            connected: false,
            outbound_tx: None,
            inbound_rx: None,
            inbound_tx: None,
        }
    }

    pub(crate) fn connect(&mut self, provider: &str) -> AdapterResult<()> {
        let parsed = ParsedEndpoint::parse(&self.endpoint)?;
        #[cfg(test)]
        if parsed.host == "test.live" {
            let (out_tx, out_rx) = mpsc::channel::<WebSocketOutbound>();
            let (in_tx, in_rx) = mpsc::channel::<String>();
            let _ = thread::spawn(move || while out_rx.recv().is_ok() {});
            self.connected = true;
            self.outbound_tx = Some(out_tx);
            self.inbound_rx = Some(in_rx);
            self.inbound_tx = Some(in_tx);
            return Ok(());
        }
        let (out_tx, out_rx) = mpsc::channel::<WebSocketOutbound>();
        let (in_tx, in_rx) = mpsc::channel::<String>();

        match parsed.scheme.as_str() {
            "ws" => {
                let mut stream =
                    TcpStream::connect((parsed.host.as_str(), parsed.port)).map_err(|e| {
                        AdapterError::Other(format!("{provider} ws connect failed: {e}"))
                    })?;
                let _ = stream.set_nodelay(true);
                websocket_handshake(&mut stream, &parsed.host, parsed.port, &parsed.path)?;
                let writer = stream
                    .try_clone()
                    .map_err(|e| AdapterError::Other(format!("{provider} ws clone failed: {e}")))?;
                spawn_text_ws_workers(writer, stream, out_rx, in_tx.clone(), out_tx.clone());
            }
            "wss" => {
                let openssl_args =
                    crate::openssl_s_client_args(provider, &parsed.host, parsed.port)?;
                let mut child = Command::new("openssl")
                    .args(&openssl_args)
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
                let message = match provider {
                    "binance" => "binance websocket endpoint must use ws:// or wss://",
                    "rithmic" => "rithmic websocket endpoint must use ws:// or wss://",
                    _ => "websocket endpoint must use ws:// or wss://",
                };
                return Err(AdapterError::NotConfigured(message));
            }
        }

        self.connected = true;
        self.outbound_tx = Some(out_tx);
        self.inbound_rx = Some(in_rx);
        self.inbound_tx = Some(in_tx);
        Ok(())
    }

    pub(crate) fn send_text(&mut self, provider: &str, text: String) -> AdapterResult<()> {
        if !self.connected {
            return Err(AdapterError::Disconnected);
        }
        let tx = self
            .outbound_tx
            .as_ref()
            .ok_or(AdapterError::Disconnected)?;
        tx.send(WebSocketOutbound::Text(text))
            .map_err(|_| AdapterError::Other(format!("{provider} transport send failed")))
    }

    pub(crate) fn recv_text(&mut self) -> AdapterResult<Option<String>> {
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

    pub(crate) fn is_connected(&self) -> bool {
        self.connected
    }

    #[cfg(test)]
    pub(crate) fn inject_text(&mut self, text: &str) {
        if let Some(tx) = &self.inbound_tx {
            let _ = tx.send(text.to_string());
        }
    }

    #[cfg(test)]
    pub(crate) fn force_disconnect(&mut self) {
        self.connected = false;
    }
}

#[derive(Debug)]
pub(crate) struct ParsedEndpoint {
    pub(crate) scheme: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) path: String,
}

impl ParsedEndpoint {
    pub(crate) fn parse(endpoint: &str) -> AdapterResult<Self> {
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
    out_rx: Receiver<WebSocketOutbound>,
    in_tx: Sender<String>,
    pong_tx: Sender<WebSocketOutbound>,
) where
    W: Write + Send + 'static,
    R: Read + Send + 'static,
{
    let mut writer_owned = writer;
    let mut reader_owned = reader;
    let _ = thread::spawn(move || {
        while let Ok(msg) = out_rx.recv() {
            let frame = match msg {
                WebSocketOutbound::Text(t) => encode_client_frame(0x1, t.as_bytes()),
                WebSocketOutbound::Pong(p) => encode_client_frame(0xA, &p),
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
                let _ = pong_tx.send(WebSocketOutbound::Pong(payload));
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
