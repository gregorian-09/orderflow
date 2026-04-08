#![doc = include_str!("../README.md")]

use std::fs::{self, create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use of_core::{BookAction, BookUpdate, Side, TradePrint};
use serde::Deserialize;

/// Persistence-layer errors.
#[derive(Debug)]
pub enum PersistError {
    /// Filesystem I/O failure.
    Io(std::io::Error),
}

impl From<std::io::Error> for PersistError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Result type alias used by persistence APIs.
pub type PersistResult<T> = Result<T, PersistError>;

/// Retention policy used by [`RollingStore`].
#[derive(Debug, Clone, Copy)]
pub struct RetentionPolicy {
    /// Maximum bytes to keep under store root (0 disables size pruning).
    pub max_total_bytes: u64,
    /// Maximum file age in seconds (0 disables age pruning).
    pub max_age_secs: u64,
}

/// JSONL rolling store for book/trade stream persistence.
#[derive(Debug, Clone)]
pub struct RollingStore {
    root: PathBuf,
    retention: Option<RetentionPolicy>,
}

/// Parsed book event read back from persisted JSONL storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBookEvent {
    /// Event sequence number.
    pub sequence: u64,
    /// Book side for the level update.
    pub side: Side,
    /// Price level index carried by the persisted update.
    pub level: u16,
    /// Price for the persisted update.
    pub price: i64,
    /// Size for the persisted update.
    pub size: i64,
    /// Book action recorded for the update.
    pub action: BookAction,
}

/// Parsed trade event read back from persisted JSONL storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTradeEvent {
    /// Event sequence number.
    pub sequence: u64,
    /// Trade price.
    pub price: i64,
    /// Trade size.
    pub size: i64,
    /// Aggressor side stored for the trade.
    pub aggressor_side: Side,
}

/// Merged persisted event used for replay-oriented symbol reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredEvent {
    /// Materialized book update record.
    Book(StoredBookEvent),
    /// Materialized trade record.
    Trade(StoredTradeEvent),
}

impl StoredEvent {
    /// Returns the persisted sequence number used for replay ordering.
    pub fn sequence(&self) -> u64 {
        match self {
            Self::Book(book) => book.sequence,
            Self::Trade(trade) => trade.sequence,
        }
    }
}

impl RollingStore {
    /// Creates a store rooted at `root`, creating directories as needed.
    pub fn new(root: impl AsRef<Path>) -> PersistResult<Self> {
        create_dir_all(root.as_ref())?;
        Ok(Self {
            root: root.as_ref().to_path_buf(),
            retention: None,
        })
    }

    /// Sets optional retention policy used after each append.
    pub fn with_retention(mut self, retention: Option<RetentionPolicy>) -> Self {
        self.retention = retention;
        self
    }

    /// Appends a single book event as JSON line.
    pub fn append_book(&self, event: &BookUpdate) -> PersistResult<()> {
        self.append_line(
            &event.symbol.venue,
            &event.symbol.symbol,
            "book",
            &format!(
                "{{\"seq\":{},\"side\":\"{:?}\",\"level\":{},\"price\":{},\"size\":{},\"action\":\"{:?}\"}}",
                event.sequence, event.side, event.level, event.price, event.size, event.action
            ),
        )
    }

    /// Appends a single trade event as JSON line.
    pub fn append_trade(&self, event: &TradePrint) -> PersistResult<()> {
        self.append_line(
            &event.symbol.venue,
            &event.symbol.symbol,
            "trades",
            &format!(
                "{{\"seq\":{},\"price\":{},\"size\":{},\"aggressor\":\"{:?}\"}}",
                event.sequence, event.price, event.size, event.aggressor_side
            ),
        )
    }

    /// Reads persisted book events for the given venue and symbol.
    ///
    /// Missing streams return an empty vector.
    pub fn read_books(&self, venue: &str, symbol: &str) -> PersistResult<Vec<StoredBookEvent>> {
        let path = self.stream_path(venue, symbol, "book");
        read_jsonl_stream(&path, parse_book_line)
    }

    /// Reads persisted trade events for the given venue and symbol.
    ///
    /// Missing streams return an empty vector.
    pub fn read_trades(&self, venue: &str, symbol: &str) -> PersistResult<Vec<StoredTradeEvent>> {
        let path = self.stream_path(venue, symbol, "trades");
        read_jsonl_stream(&path, parse_trade_line)
    }

    /// Reads and merges persisted book and trade events for the given venue and symbol.
    ///
    /// Events are ordered by ascending sequence number. When two events share the
    /// same sequence, book events are returned before trade events so replay order
    /// remains deterministic across runs.
    pub fn read_events(&self, venue: &str, symbol: &str) -> PersistResult<Vec<StoredEvent>> {
        let mut events = self
            .read_books(venue, symbol)?
            .into_iter()
            .map(StoredEvent::Book)
            .chain(self.read_trades(venue, symbol)?.into_iter().map(StoredEvent::Trade))
            .collect::<Vec<_>>();
        events.sort_by(|left, right| {
            left.sequence()
                .cmp(&right.sequence())
                .then_with(|| stored_event_kind_rank(left).cmp(&stored_event_kind_rank(right)))
        });
        Ok(events)
    }

    fn append_line(
        &self,
        venue: &str,
        symbol: &str,
        stream: &str,
        line: &str,
    ) -> PersistResult<()> {
        let mut dir = self.root.clone();
        dir.push(venue);
        dir.push(symbol);
        create_dir_all(&dir)?;

        let mut path = dir;
        path.push(format!("{stream}.jsonl"));

        let mut f = OpenOptions::new().create(true).append(true).open(path)?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;

        self.prune_if_needed()?;
        Ok(())
    }

    fn stream_path(&self, venue: &str, symbol: &str, stream: &str) -> PathBuf {
        let mut path = self.root.clone();
        path.push(venue);
        path.push(symbol);
        path.push(format!("{stream}.jsonl"));
        path
    }

    fn prune_if_needed(&self) -> PersistResult<()> {
        let Some(policy) = self.retention else {
            return Ok(());
        };

        let mut files = Vec::new();
        collect_files(&self.root, &mut files)?;

        if policy.max_age_secs > 0 {
            let now = SystemTime::now();
            for f in &files {
                let age = now
                    .duration_since(f.modified)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                if age > policy.max_age_secs {
                    let _ = fs::remove_file(&f.path);
                }
            }
            files.clear();
            collect_files(&self.root, &mut files)?;
        }

        if policy.max_total_bytes > 0 {
            let mut total: u64 = files.iter().map(|f| f.len).sum();
            if total > policy.max_total_bytes {
                files.sort_by_key(|f| f.modified);
                for f in files {
                    if total <= policy.max_total_bytes {
                        break;
                    }
                    if fs::remove_file(&f.path).is_ok() {
                        total = total.saturating_sub(f.len);
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct FileMeta {
    path: PathBuf,
    len: u64,
    modified: SystemTime,
}

fn collect_files(root: &Path, out: &mut Vec<FileMeta>) -> PersistResult<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let ty = entry.file_type()?;
        if ty.is_dir() {
            collect_files(&path, out)?;
        } else if ty.is_file() {
            let meta = entry.metadata()?;
            out.push(FileMeta {
                path,
                len: meta.len(),
                modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct StoredBookEventWire {
    seq: u64,
    side: String,
    level: u16,
    price: i64,
    size: i64,
    action: String,
}

#[derive(Debug, Deserialize)]
struct StoredTradeEventWire {
    seq: u64,
    price: i64,
    size: i64,
    aggressor: String,
}

fn read_jsonl_stream<T>(
    path: &Path,
    parse_line: fn(&Path, usize, &str) -> PersistResult<T>,
) -> PersistResult<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(parse_line(path, line_no + 1, &line)?);
    }
    Ok(out)
}

fn parse_book_line(path: &Path, line_no: usize, line: &str) -> PersistResult<StoredBookEvent> {
    let raw: StoredBookEventWire = serde_json::from_str(line)
        .map_err(|err| invalid_data(path, line_no, format!("invalid book json: {err}")))?;
    Ok(StoredBookEvent {
        sequence: raw.seq,
        side: parse_side(path, line_no, "side", &raw.side)?,
        level: raw.level,
        price: raw.price,
        size: raw.size,
        action: parse_book_action(path, line_no, &raw.action)?,
    })
}

fn parse_trade_line(path: &Path, line_no: usize, line: &str) -> PersistResult<StoredTradeEvent> {
    let raw: StoredTradeEventWire = serde_json::from_str(line)
        .map_err(|err| invalid_data(path, line_no, format!("invalid trade json: {err}")))?;
    Ok(StoredTradeEvent {
        sequence: raw.seq,
        price: raw.price,
        size: raw.size,
        aggressor_side: parse_side(path, line_no, "aggressor", &raw.aggressor)?,
    })
}

fn parse_side(path: &Path, line_no: usize, field: &str, raw: &str) -> PersistResult<Side> {
    match raw {
        "Bid" => Ok(Side::Bid),
        "Ask" => Ok(Side::Ask),
        _ => Err(invalid_data(
            path,
            line_no,
            format!("invalid {field} value: {raw}"),
        )),
    }
}

fn parse_book_action(path: &Path, line_no: usize, raw: &str) -> PersistResult<BookAction> {
    match raw {
        "Upsert" => Ok(BookAction::Upsert),
        "Delete" => Ok(BookAction::Delete),
        _ => Err(invalid_data(
            path,
            line_no,
            format!("invalid action value: {raw}"),
        )),
    }
}

fn invalid_data(path: &Path, line_no: usize, message: String) -> PersistError {
    PersistError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("{}:{line_no}: {message}", path.display()),
    ))
}

fn stored_event_kind_rank(event: &StoredEvent) -> u8 {
    match event {
        StoredEvent::Book(_) => 0,
        StoredEvent::Trade(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use of_core::{BookAction, BookUpdate, Side, SymbolId};

    use super::*;

    #[test]
    fn prunes_by_total_size() {
        let root = temp_dir("persist_prune_size");
        let store = RollingStore::new(&root)
            .expect("store")
            .with_retention(Some(RetentionPolicy {
                max_total_bytes: 150,
                max_age_secs: 0,
            }));

        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        for seq in 0..20 {
            store
                .append_book(&BookUpdate {
                    symbol: symbol.clone(),
                    side: Side::Bid,
                    level: 0,
                    price: 100,
                    size: 1,
                    action: BookAction::Upsert,
                    sequence: seq,
                    ts_exchange_ns: 0,
                    ts_recv_ns: 0,
                })
                .expect("append");
        }

        let mut files = Vec::new();
        collect_files(&root, &mut files).expect("collect");
        let total: u64 = files.iter().map(|f| f.len).sum();
        assert!(total <= 150);
    }

    #[test]
    fn prunes_by_age() {
        let root = temp_dir("persist_prune_age");
        let old_path = root.join("old.jsonl");
        fs::write(&old_path, b"old").expect("write old");
        std::thread::sleep(std::time::Duration::from_millis(2200));

        let store = RollingStore::new(&root)
            .expect("store")
            .with_retention(Some(RetentionPolicy {
                max_total_bytes: 0,
                max_age_secs: 1,
            }));

        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_book(&BookUpdate {
                symbol,
                side: Side::Bid,
                level: 0,
                price: 100,
                size: 1,
                action: BookAction::Upsert,
                sequence: 1,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append");

        assert!(!old_path.exists());
    }

    #[test]
    fn reads_back_appended_book_and_trade_streams() {
        let root = temp_dir("persist_readback");
        let store = RollingStore::new(&root).expect("store");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_book(&BookUpdate {
                symbol: symbol.clone(),
                side: Side::Bid,
                level: 1,
                price: 505_000,
                size: 7,
                action: BookAction::Upsert,
                sequence: 10,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append book");
        store
            .append_trade(&TradePrint {
                symbol: symbol.clone(),
                price: 505_025,
                size: 3,
                aggressor_side: Side::Ask,
                sequence: 11,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append trade");

        let books = store.read_books(&symbol.venue, &symbol.symbol).expect("read books");
        let trades = store
            .read_trades(&symbol.venue, &symbol.symbol)
            .expect("read trades");

        assert_eq!(
            books,
            vec![StoredBookEvent {
                sequence: 10,
                side: Side::Bid,
                level: 1,
                price: 505_000,
                size: 7,
                action: BookAction::Upsert,
            }]
        );
        assert_eq!(
            trades,
            vec![StoredTradeEvent {
                sequence: 11,
                price: 505_025,
                size: 3,
                aggressor_side: Side::Ask,
            }]
        );
    }

    #[test]
    fn missing_stream_reads_back_as_empty() {
        let root = temp_dir("persist_missing_stream");
        let store = RollingStore::new(&root).expect("store");

        let books = store.read_books("CME", "ESM6").expect("read books");
        let trades = store.read_trades("CME", "ESM6").expect("read trades");

        assert!(books.is_empty());
        assert!(trades.is_empty());
    }

    #[test]
    fn invalid_stream_data_returns_invalid_data_error() {
        let root = temp_dir("persist_invalid_stream");
        let stream_dir = root.join("CME").join("ESM6");
        fs::create_dir_all(&stream_dir).expect("create dir");
        fs::write(
            stream_dir.join("book.jsonl"),
            b"{\"seq\":1,\"side\":\"Middle\",\"level\":0,\"price\":1,\"size\":1,\"action\":\"Upsert\"}\n",
        )
        .expect("write");

        let store = RollingStore::new(&root).expect("store");
        let err = store.read_books("CME", "ESM6").expect_err("invalid data");

        match err {
            PersistError::Io(inner) => assert_eq!(inner.kind(), std::io::ErrorKind::InvalidData),
        }
    }

    #[test]
    fn reads_merged_symbol_events_in_sequence_order() {
        let root = temp_dir("persist_merged_readback");
        let store = RollingStore::new(&root).expect("store");
        let symbol = SymbolId {
            venue: "CME".to_string(),
            symbol: "ESM6".to_string(),
        };

        store
            .append_trade(&TradePrint {
                symbol: symbol.clone(),
                price: 505_050,
                size: 2,
                aggressor_side: Side::Ask,
                sequence: 12,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append trade");
        store
            .append_book(&BookUpdate {
                symbol: symbol.clone(),
                side: Side::Bid,
                level: 0,
                price: 505_000,
                size: 10,
                action: BookAction::Upsert,
                sequence: 10,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append book");
        store
            .append_book(&BookUpdate {
                symbol: symbol.clone(),
                side: Side::Ask,
                level: 0,
                price: 505_075,
                size: 9,
                action: BookAction::Upsert,
                sequence: 12,
                ts_exchange_ns: 0,
                ts_recv_ns: 0,
            })
            .expect("append book");

        let events = store.read_events(&symbol.venue, &symbol.symbol).expect("read events");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].sequence(), 10);
        assert_eq!(events[1].sequence(), 12);
        assert_eq!(events[2].sequence(), 12);
        assert!(matches!(events[1], StoredEvent::Book(_)));
        assert!(matches!(events[2], StoredEvent::Trade(_)));
    }

    fn temp_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{}_{}_{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock ok")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("temp dir");
        path
    }
}
