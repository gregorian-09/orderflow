use super::*;

/// Direction of a captured FIX transcript frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixTranscriptDirection {
    /// Frame received from the counterparty.
    Inbound,
    /// Frame sent to the counterparty.
    Outbound,
}

impl FixTranscriptDirection {
    pub(crate) const fn as_byte(self) -> u8 {
        match self {
            Self::Inbound => b'I',
            Self::Outbound => b'O',
        }
    }
}

/// Fixed-size transcript message-type copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixTranscriptMsgType {
    pub(crate) len: u8,
    pub(crate) bytes: [u8; 8],
}

impl FixTranscriptMsgType {
    /// Creates an empty message-type marker.
    pub const fn empty() -> Self {
        Self {
            len: 0,
            bytes: [0; 8],
        }
    }

    /// Creates a transcript message type from wire bytes.
    ///
    /// # Errors
    ///
    /// Returns [`FixTranscriptError::MsgTypeTooLong`] when the message type
    /// exceeds the fixed transcript capacity.
    pub fn new(value: &[u8]) -> Result<Self, FixTranscriptError> {
        if value.len() > 8 {
            return Err(FixTranscriptError::MsgTypeTooLong {
                capacity: 8,
                actual: value.len(),
            });
        }
        let mut bytes = [0u8; 8];
        bytes[..value.len()].copy_from_slice(value);
        Ok(Self {
            len: value.len() as u8,
            bytes,
        })
    }

    /// Returns the copied message-type bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    /// Returns true when no message type was available.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for FixTranscriptMsgType {
    fn default() -> Self {
        Self::empty()
    }
}

/// Bounded transcript capture configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixTranscriptConfig {
    pub(crate) max_records: usize,
    pub(crate) max_raw_bytes: usize,
    pub(crate) retain_raw: bool,
}

impl FixTranscriptConfig {
    /// Creates a transcript capture configuration.
    ///
    /// A zero `max_records` disables record retention while counters and the
    /// rolling hash still advance. `max_raw_bytes` bounds retained raw frame
    /// bytes when `retain_raw` is true.
    pub const fn new(max_records: usize, max_raw_bytes: usize, retain_raw: bool) -> Self {
        Self {
            max_records,
            max_raw_bytes,
            retain_raw,
        }
    }

    /// Returns the maximum retained record count.
    pub const fn max_records(&self) -> usize {
        self.max_records
    }

    /// Returns the maximum retained raw byte count.
    pub const fn max_raw_bytes(&self) -> usize {
        self.max_raw_bytes
    }

    /// Returns whether raw FIX frames are retained when they fit.
    pub const fn retain_raw(&self) -> bool {
        self.retain_raw
    }
}

impl Default for FixTranscriptConfig {
    fn default() -> Self {
        Self {
            max_records: 4096,
            max_raw_bytes: 4 * 1024 * 1024,
            retain_raw: true,
        }
    }
}

/// Transcript capture errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixTranscriptError {
    /// Message type exceeded the fixed transcript capacity.
    MsgTypeTooLong {
        /// Configured capacity in bytes.
        capacity: usize,
        /// Actual supplied byte length.
        actual: usize,
    },
}

impl fmt::Display for FixTranscriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MsgTypeTooLong { capacity, actual } => write!(
                f,
                "FIX transcript message type length {actual} exceeds capacity {capacity}"
            ),
        }
    }
}

impl Error for FixTranscriptError {}

/// Retained transcript frame metadata and optional raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixTranscriptRecord {
    pub(crate) ordinal: u64,
    pub(crate) timestamp_ns: u64,
    pub(crate) direction: FixTranscriptDirection,
    pub(crate) seq_no: Option<u64>,
    pub(crate) msg_type: FixTranscriptMsgType,
    pub(crate) raw_len: usize,
    pub(crate) raw_checksum: u8,
    pub(crate) raw_hash: u64,
    pub(crate) raw_retained: bool,
    pub(crate) raw: Vec<u8>,
}

impl FixTranscriptRecord {
    /// Returns the one-based capture ordinal.
    pub const fn ordinal(&self) -> u64 {
        self.ordinal
    }

    /// Returns the caller-provided capture timestamp in nanoseconds.
    pub const fn timestamp_ns(&self) -> u64 {
        self.timestamp_ns
    }

    /// Returns the capture direction.
    pub const fn direction(&self) -> FixTranscriptDirection {
        self.direction
    }

    /// Returns `MsgSeqNum(34)` when known.
    pub const fn seq_no(&self) -> Option<u64> {
        self.seq_no
    }

    /// Returns the copied `MsgType(35)` bytes when known.
    pub fn msg_type(&self) -> &[u8] {
        self.msg_type.as_bytes()
    }

    /// Returns the original raw frame length.
    pub const fn raw_len(&self) -> usize {
        self.raw_len
    }

    /// Returns the FIX modulo-256 checksum over the raw frame bytes.
    pub const fn raw_checksum(&self) -> u8 {
        self.raw_checksum
    }

    /// Returns the FNV-1a hash over the raw frame bytes.
    pub const fn raw_hash(&self) -> u64 {
        self.raw_hash
    }

    /// Returns whether raw frame bytes were retained.
    pub const fn raw_retained(&self) -> bool {
        self.raw_retained
    }

    /// Returns retained raw frame bytes, or an empty slice when raw retention
    /// was disabled, oversized, or evicted with the record.
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }
}

/// Result of recording a transcript frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixTranscriptRetention {
    pub(crate) retained: bool,
    pub(crate) raw_retained: bool,
    pub(crate) evicted_records: u64,
    pub(crate) evicted_raw_bytes: u64,
}

impl FixTranscriptRetention {
    /// Returns whether the transcript record metadata was retained.
    pub const fn retained(&self) -> bool {
        self.retained
    }

    /// Returns whether raw frame bytes were retained.
    pub const fn raw_retained(&self) -> bool {
        self.raw_retained
    }

    /// Returns records evicted to satisfy configured bounds.
    pub const fn evicted_records(&self) -> u64 {
        self.evicted_records
    }

    /// Returns retained raw bytes evicted to satisfy configured bounds.
    pub const fn evicted_raw_bytes(&self) -> u64 {
        self.evicted_raw_bytes
    }
}

/// Snapshot of transcript capture counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixTranscriptMetrics {
    pub(crate) captured_records: u64,
    pub(crate) retained_records: u64,
    pub(crate) retained_raw_bytes: u64,
    pub(crate) dropped_records: u64,
    pub(crate) dropped_raw_bytes: u64,
    pub(crate) evicted_records: u64,
    pub(crate) evicted_raw_bytes: u64,
    pub(crate) oldest_ordinal: Option<u64>,
    pub(crate) newest_ordinal: Option<u64>,
    pub(crate) rolling_hash: u64,
}

impl FixTranscriptMetrics {
    /// Returns the total number of frames observed by the capture.
    pub const fn captured_records(&self) -> u64 {
        self.captured_records
    }

    /// Returns the number of retained transcript records.
    pub const fn retained_records(&self) -> u64 {
        self.retained_records
    }

    /// Returns retained raw frame bytes.
    pub const fn retained_raw_bytes(&self) -> u64 {
        self.retained_raw_bytes
    }

    /// Returns records dropped because record retention was disabled.
    pub const fn dropped_records(&self) -> u64 {
        self.dropped_records
    }

    /// Returns raw bytes not retained because raw retention was disabled,
    /// oversized, or record retention was disabled.
    pub const fn dropped_raw_bytes(&self) -> u64 {
        self.dropped_raw_bytes
    }

    /// Returns records evicted by bounded retention.
    pub const fn evicted_records(&self) -> u64 {
        self.evicted_records
    }

    /// Returns raw bytes evicted by bounded retention.
    pub const fn evicted_raw_bytes(&self) -> u64 {
        self.evicted_raw_bytes
    }

    /// Returns the oldest retained transcript ordinal.
    pub const fn oldest_ordinal(&self) -> Option<u64> {
        self.oldest_ordinal
    }

    /// Returns the newest captured transcript ordinal.
    pub const fn newest_ordinal(&self) -> Option<u64> {
        self.newest_ordinal
    }

    /// Returns the deterministic rolling hash over captured metadata and raw
    /// bytes.
    pub const fn rolling_hash(&self) -> u64 {
        self.rolling_hash
    }
}

/// Bounded in-memory FIX transcript capture.
///
/// The capture is intentionally transport- and storage-neutral. It records
/// frame metadata and optional raw bytes for certification/audit workflows,
/// while durable transcript archives can persist records according to their own
/// sync and redaction policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixTranscriptCapture {
    pub(crate) config: FixTranscriptConfig,
    pub(crate) records: VecDeque<FixTranscriptRecord>,
    pub(crate) retained_raw_bytes: usize,
    pub(crate) captured_records: u64,
    pub(crate) dropped_records: u64,
    pub(crate) dropped_raw_bytes: u64,
    pub(crate) evicted_records: u64,
    pub(crate) evicted_raw_bytes: u64,
    pub(crate) rolling_hash: u64,
}

impl Default for FixTranscriptCapture {
    fn default() -> Self {
        Self::new(FixTranscriptConfig::default())
    }
}

impl FixTranscriptCapture {
    /// Creates an empty transcript capture.
    pub fn new(config: FixTranscriptConfig) -> Self {
        Self {
            config,
            records: VecDeque::with_capacity(config.max_records.min(64)),
            retained_raw_bytes: 0,
            captured_records: 0,
            dropped_records: 0,
            dropped_raw_bytes: 0,
            evicted_records: 0,
            evicted_raw_bytes: 0,
            rolling_hash: FNV_OFFSET_BASIS,
        }
    }

    /// Returns the configured capture bounds.
    pub const fn config(&self) -> FixTranscriptConfig {
        self.config
    }

    /// Returns retained transcript records in capture order.
    pub fn records(&self) -> impl Iterator<Item = &FixTranscriptRecord> {
        self.records.iter()
    }

    /// Records a parsed validated FIX message.
    ///
    /// # Errors
    ///
    /// Returns [`FixTranscriptError`] when the parsed message type exceeds the
    /// fixed transcript capacity.
    pub fn record_message(
        &mut self,
        direction: FixTranscriptDirection,
        timestamp_ns: u64,
        message: &FixMessageView<'_>,
    ) -> Result<FixTranscriptRetention, FixTranscriptError> {
        self.record_frame(
            direction,
            timestamp_ns,
            message.msg_seq_num(),
            message.msg_type().unwrap_or(&[]),
            message.raw(),
        )
    }

    /// Records a raw FIX frame with caller-provided sequence and message type
    /// metadata.
    ///
    /// # Errors
    ///
    /// Returns [`FixTranscriptError`] when `msg_type` exceeds the fixed
    /// transcript capacity.
    pub fn record_frame(
        &mut self,
        direction: FixTranscriptDirection,
        timestamp_ns: u64,
        seq_no: Option<u64>,
        msg_type: &[u8],
        raw: &[u8],
    ) -> Result<FixTranscriptRetention, FixTranscriptError> {
        let msg_type = FixTranscriptMsgType::new(msg_type)?;
        self.captured_records = self.captured_records.saturating_add(1);
        let ordinal = self.captured_records;
        let raw_hash = hash_bytes(raw);
        let raw_checksum = checksum(raw);
        self.rolling_hash = update_transcript_hash(
            self.rolling_hash,
            ordinal,
            timestamp_ns,
            direction,
            seq_no,
            msg_type.as_bytes(),
            raw,
        );

        if self.config.max_records == 0 {
            self.dropped_records = self.dropped_records.saturating_add(1);
            self.dropped_raw_bytes = self
                .dropped_raw_bytes
                .saturating_add(usize_to_u64(raw.len()));
            return Ok(FixTranscriptRetention {
                retained: false,
                raw_retained: false,
                evicted_records: 0,
                evicted_raw_bytes: 0,
            });
        }

        let retain_raw = self.config.retain_raw
            && self.config.max_raw_bytes > 0
            && raw.len() <= self.config.max_raw_bytes;
        let raw_vec = if retain_raw { raw.to_vec() } else { Vec::new() };
        if !retain_raw && !raw.is_empty() {
            self.dropped_raw_bytes = self
                .dropped_raw_bytes
                .saturating_add(usize_to_u64(raw.len()));
        }
        let retained_raw_len = raw_vec.len();
        self.retained_raw_bytes = self.retained_raw_bytes.saturating_add(retained_raw_len);
        self.records.push_back(FixTranscriptRecord {
            ordinal,
            timestamp_ns,
            direction,
            seq_no,
            msg_type,
            raw_len: raw.len(),
            raw_checksum,
            raw_hash,
            raw_retained: retain_raw,
            raw: raw_vec,
        });

        let (evicted_records, evicted_raw_bytes) = self.evict_to_bounds();
        Ok(FixTranscriptRetention {
            retained: true,
            raw_retained: retain_raw,
            evicted_records,
            evicted_raw_bytes,
        })
    }

    /// Clears retained records and byte counters without resetting cumulative
    /// capture/drop/eviction counters or the rolling hash.
    pub fn clear_retained(&mut self) {
        self.records.clear();
        self.retained_raw_bytes = 0;
    }

    /// Returns transcript capture metrics.
    pub fn metrics(&self) -> FixTranscriptMetrics {
        FixTranscriptMetrics {
            captured_records: self.captured_records,
            retained_records: usize_to_u64(self.records.len()),
            retained_raw_bytes: usize_to_u64(self.retained_raw_bytes),
            dropped_records: self.dropped_records,
            dropped_raw_bytes: self.dropped_raw_bytes,
            evicted_records: self.evicted_records,
            evicted_raw_bytes: self.evicted_raw_bytes,
            oldest_ordinal: self.records.front().map(FixTranscriptRecord::ordinal),
            newest_ordinal: if self.captured_records == 0 {
                None
            } else {
                Some(self.captured_records)
            },
            rolling_hash: self.rolling_hash,
        }
    }

    fn evict_to_bounds(&mut self) -> (u64, u64) {
        let mut evicted_records = 0u64;
        let mut evicted_raw_bytes = 0u64;
        while self.records.len() > self.config.max_records
            || self.retained_raw_bytes > self.config.max_raw_bytes
        {
            let Some(record) = self.records.pop_front() else {
                break;
            };
            evicted_records = evicted_records.saturating_add(1);
            let raw_len = if record.raw_retained {
                record.raw.len()
            } else {
                0
            };
            self.retained_raw_bytes = self.retained_raw_bytes.saturating_sub(raw_len);
            evicted_raw_bytes = evicted_raw_bytes.saturating_add(usize_to_u64(raw_len));
        }
        self.evicted_records = self.evicted_records.saturating_add(evicted_records);
        self.evicted_raw_bytes = self.evicted_raw_bytes.saturating_add(evicted_raw_bytes);
        (evicted_records, evicted_raw_bytes)
    }
}
