use std::fs::{self, create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use of_core::{BookUpdate, TradePrint};

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
