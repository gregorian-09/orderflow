use super::*;

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
    pub(crate) root: PathBuf,
    pub(crate) retention: Option<RetentionPolicy>,
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
                "{{\"schema\":{},\"seq\":{},\"side\":\"{:?}\",\"level\":{},\"price\":{},\"size\":{},\"action\":\"{:?}\",\"ts_exchange_ns\":{},\"ts_recv_ns\":{}}}",
                JSONL_SCHEMA_VERSION,
                event.sequence,
                event.side,
                event.level,
                event.price,
                event.size,
                event.action,
                event.ts_exchange_ns,
                event.ts_recv_ns
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
                "{{\"schema\":{},\"seq\":{},\"price\":{},\"size\":{},\"aggressor\":\"{:?}\",\"ts_exchange_ns\":{},\"ts_recv_ns\":{}}}",
                JSONL_SCHEMA_VERSION,
                event.sequence,
                event.price,
                event.size,
                event.aggressor_side,
                event.ts_exchange_ns,
                event.ts_recv_ns
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

    /// Reads persisted book events filtered by an inclusive sequence range.
    ///
    /// `from_sequence` and `to_sequence` are optional inclusive bounds.
    pub fn read_books_in_range(
        &self,
        venue: &str,
        symbol: &str,
        from_sequence: Option<u64>,
        to_sequence: Option<u64>,
    ) -> PersistResult<Vec<StoredBookEvent>> {
        let events = self.read_books(venue, symbol)?;
        Ok(filter_by_sequence_range(events, from_sequence, to_sequence))
    }

    /// Reads persisted trade events for the given venue and symbol.
    ///
    /// Missing streams return an empty vector.
    pub fn read_trades(&self, venue: &str, symbol: &str) -> PersistResult<Vec<StoredTradeEvent>> {
        let path = self.stream_path(venue, symbol, "trades");
        read_jsonl_stream(&path, parse_trade_line)
    }

    /// Reads persisted trade events filtered by an inclusive sequence range.
    ///
    /// `from_sequence` and `to_sequence` are optional inclusive bounds.
    pub fn read_trades_in_range(
        &self,
        venue: &str,
        symbol: &str,
        from_sequence: Option<u64>,
        to_sequence: Option<u64>,
    ) -> PersistResult<Vec<StoredTradeEvent>> {
        let events = self.read_trades(venue, symbol)?;
        Ok(filter_by_sequence_range(events, from_sequence, to_sequence))
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
            .chain(
                self.read_trades(venue, symbol)?
                    .into_iter()
                    .map(StoredEvent::Trade),
            )
            .collect::<Vec<_>>();
        events.sort_by(|left, right| {
            left.sequence()
                .cmp(&right.sequence())
                .then_with(|| stored_event_kind_rank(left).cmp(&stored_event_kind_rank(right)))
        });
        Ok(events)
    }

    /// Reads merged persisted events filtered by an inclusive sequence range.
    ///
    /// `from_sequence` and `to_sequence` are optional inclusive bounds.
    pub fn read_events_in_range(
        &self,
        venue: &str,
        symbol: &str,
        from_sequence: Option<u64>,
        to_sequence: Option<u64>,
    ) -> PersistResult<Vec<StoredEvent>> {
        let events = self.read_events(venue, symbol)?;
        Ok(filter_by_sequence_range(events, from_sequence, to_sequence))
    }

    /// Lists venue directories currently present under the store root.
    ///
    /// The returned list is sorted for deterministic discovery and replay tooling.
    pub fn list_venues(&self) -> PersistResult<Vec<String>> {
        let mut venues = BTreeSet::new();
        for entry in read_dir_if_exists(&self.root)? {
            if entry.file_type()?.is_dir() {
                venues.insert(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(venues.into_iter().collect())
    }

    /// Lists symbol directories for a given venue currently present under the store root.
    ///
    /// Missing venues return an empty vector. The returned list is sorted for deterministic discovery.
    pub fn list_symbols(&self, venue: &str) -> PersistResult<Vec<String>> {
        let mut path = self.root.clone();
        path.push(venue);

        let mut symbols = BTreeSet::new();
        for entry in read_dir_if_exists(&path)? {
            if entry.file_type()?.is_dir() {
                symbols.insert(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(symbols.into_iter().collect())
    }

    /// Lists stream files currently present for a given venue and symbol.
    ///
    /// Returned names omit the `.jsonl` suffix and are sorted for deterministic replay tooling.
    /// Missing symbols return an empty vector.
    pub fn list_streams(&self, venue: &str, symbol: &str) -> PersistResult<Vec<String>> {
        let mut path = self.root.clone();
        path.push(venue);
        path.push(symbol);

        let mut streams = BTreeSet::new();
        for entry in read_dir_if_exists(&path)? {
            if !entry.file_type()?.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if let Some(stem) = name.strip_suffix(".jsonl") {
                streams.insert(stem.to_string());
            }
        }
        Ok(streams.into_iter().collect())
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
                if age >= policy.max_age_secs {
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
pub(crate) struct FileMeta {
    pub(crate) path: PathBuf,
    pub(crate) len: u64,
    pub(crate) modified: SystemTime,
}

pub(crate) fn collect_files(root: &Path, out: &mut Vec<FileMeta>) -> PersistResult<()> {
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

pub(crate) fn read_dir_if_exists(path: &Path) -> PersistResult<Vec<fs::DirEntry>> {
    match fs::read_dir(path) {
        Ok(dir) => dir.collect::<Result<Vec<_>, _>>().map_err(PersistError::Io),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(PersistError::Io(err)),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct StoredBookEventWire {
    pub(crate) seq: u64,
    pub(crate) side: String,
    pub(crate) level: u16,
    pub(crate) price: i64,
    pub(crate) size: i64,
    pub(crate) action: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StoredTradeEventWire {
    pub(crate) seq: u64,
    pub(crate) price: i64,
    pub(crate) size: i64,
    pub(crate) aggressor: String,
}

pub(crate) fn read_jsonl_stream<T>(
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

pub(crate) fn parse_book_line(
    path: &Path,
    line_no: usize,
    line: &str,
) -> PersistResult<StoredBookEvent> {
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

pub(crate) fn parse_trade_line(
    path: &Path,
    line_no: usize,
    line: &str,
) -> PersistResult<StoredTradeEvent> {
    let raw: StoredTradeEventWire = serde_json::from_str(line)
        .map_err(|err| invalid_data(path, line_no, format!("invalid trade json: {err}")))?;
    Ok(StoredTradeEvent {
        sequence: raw.seq,
        price: raw.price,
        size: raw.size,
        aggressor_side: parse_side(path, line_no, "aggressor", &raw.aggressor)?,
    })
}

pub(crate) fn parse_side(
    path: &Path,
    line_no: usize,
    field: &str,
    raw: &str,
) -> PersistResult<Side> {
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

pub(crate) fn parse_book_action(
    path: &Path,
    line_no: usize,
    raw: &str,
) -> PersistResult<BookAction> {
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

pub(crate) fn invalid_data(path: &Path, line_no: usize, message: String) -> PersistError {
    PersistError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("{}:{line_no}: {message}", path.display()),
    ))
}

pub(crate) fn stored_event_kind_rank(event: &StoredEvent) -> u8 {
    match event {
        StoredEvent::Book(_) => 0,
        StoredEvent::Trade(_) => 1,
    }
}

pub(crate) trait SequenceNumber {
    fn sequence(&self) -> u64;
}

impl SequenceNumber for StoredBookEvent {
    fn sequence(&self) -> u64 {
        self.sequence
    }
}

impl SequenceNumber for StoredTradeEvent {
    fn sequence(&self) -> u64 {
        self.sequence
    }
}

impl SequenceNumber for StoredEvent {
    fn sequence(&self) -> u64 {
        StoredEvent::sequence(self)
    }
}

pub(crate) fn filter_by_sequence_range<T>(
    events: Vec<T>,
    from_sequence: Option<u64>,
    to_sequence: Option<u64>,
) -> Vec<T>
where
    T: SequenceNumber,
{
    events
        .into_iter()
        .filter(|event| {
            let seq = event.sequence();
            from_sequence.is_none_or(|from| seq >= from) && to_sequence.is_none_or(|to| seq <= to)
        })
        .collect()
}
