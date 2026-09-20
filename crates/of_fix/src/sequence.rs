use super::*;

/// FIX session lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixSessionState {
    /// No active transport connection exists.
    Disconnected,
    /// Transport connection attempt is in progress.
    Connecting,
    /// Logon has been sent and the session is awaiting acceptance.
    LogonSent,
    /// Session is active and can process application flow.
    Ready,
    /// A resend request has been emitted and the session is waiting for gap
    /// recovery.
    ResendRequested,
    /// Session is applying recovery messages or sequence resets.
    Recovering,
    /// Logout has been sent and the session is draining.
    LogoutSent,
    /// Session has been intentionally stopped.
    Stopped,
    /// Session is alive but operating under a degraded policy.
    Degraded,
}

/// Resend range requested after an inbound sequence gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixResendRange {
    /// First missing sequence number.
    pub begin_seq_no: u64,
    /// Last missing sequence number.
    pub end_seq_no: u64,
}

/// Result of observing an inbound sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSequenceAction {
    /// The sequence number matched the expected inbound value and should be
    /// processed normally.
    Accept {
        /// Accepted inbound sequence number.
        seq_no: u64,
    },
    /// The sequence number is lower than expected and carries
    /// `PossDupFlag(43)=Y`, so the caller should ignore it if already applied.
    Duplicate {
        /// Duplicate inbound sequence number.
        seq_no: u64,
        /// Current expected inbound sequence number.
        expected: u64,
    },
    /// The sequence number is higher than expected and a resend request should
    /// be emitted for the missing range.
    Gap {
        /// Current expected inbound sequence number.
        expected: u64,
        /// Received inbound sequence number.
        received: u64,
        /// Missing range to request.
        resend: FixResendRange,
    },
    /// The sequence number is lower than expected without a duplicate marker.
    TooLow {
        /// Current expected inbound sequence number.
        expected: u64,
        /// Received inbound sequence number.
        received: u64,
    },
}

/// FIX sequence tracking errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSequenceError {
    /// `MsgSeqNum(34)` is missing.
    MissingMsgSeqNum,
    /// A sequence number was zero. FIX sequence numbers are one-based.
    ZeroSeqNo,
    /// A sequence reset attempted to lower the expected inbound sequence.
    SequenceResetWouldDecrease {
        /// Current expected inbound sequence number.
        current: u64,
        /// Requested new expected inbound sequence number.
        requested: u64,
    },
}

impl fmt::Display for FixSequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMsgSeqNum => write!(f, "FIX MsgSeqNum(34) is missing"),
            Self::ZeroSeqNo => write!(f, "FIX sequence number must be greater than zero"),
            Self::SequenceResetWouldDecrease { current, requested } => write!(
                f,
                "FIX sequence reset would decrease next inbound sequence: current {current}, requested {requested}"
            ),
        }
    }
}

impl Error for FixSequenceError {}

/// Borrowed FIX session identity.
///
/// A FIX session is commonly identified by begin string, sender, target, and an
/// optional qualifier used to disambiguate otherwise identical sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixSessionId<'a> {
    pub(crate) version: FixVersion,
    pub(crate) sender_comp_id: &'a [u8],
    pub(crate) target_comp_id: &'a [u8],
    pub(crate) qualifier: &'a [u8],
}

impl<'a> FixSessionId<'a> {
    /// Creates a session id without a qualifier.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when a value contains SOH.
    pub fn new(
        version: FixVersion,
        sender_comp_id: &'a [u8],
        target_comp_id: &'a [u8],
    ) -> Result<Self, FixEncodeError> {
        Self::with_qualifier(version, sender_comp_id, target_comp_id, b"")
    }

    /// Creates a session id with an optional qualifier.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when a value contains SOH.
    pub fn with_qualifier(
        version: FixVersion,
        sender_comp_id: &'a [u8],
        target_comp_id: &'a [u8],
        qualifier: &'a [u8],
    ) -> Result<Self, FixEncodeError> {
        validate_value(FixTag::SENDER_COMP_ID, sender_comp_id)?;
        validate_value(FixTag::TARGET_COMP_ID, target_comp_id)?;
        validate_value(FixTag::TEXT, qualifier)?;
        Ok(Self {
            version,
            sender_comp_id,
            target_comp_id,
            qualifier,
        })
    }

    /// Returns the FIX version.
    pub const fn version(&self) -> FixVersion {
        self.version
    }

    /// Returns `SenderCompID(49)`.
    pub const fn sender_comp_id(&self) -> &'a [u8] {
        self.sender_comp_id
    }

    /// Returns `TargetCompID(56)`.
    pub const fn target_comp_id(&self) -> &'a [u8] {
        self.target_comp_id
    }

    /// Returns the optional session qualifier bytes.
    pub const fn qualifier(&self) -> &'a [u8] {
        self.qualifier
    }
}

/// Borrowed persistable sequence-state snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixSequenceSnapshot<'a> {
    pub(crate) session_id: FixSessionId<'a>,
    pub(crate) next_inbound: u64,
    pub(crate) next_outbound: u64,
    pub(crate) trading_day: &'a [u8],
}

impl<'a> FixSequenceSnapshot<'a> {
    /// Creates a sequence snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when `trading_day` contains SOH.
    pub fn new(
        session_id: FixSessionId<'a>,
        next_inbound: u64,
        next_outbound: u64,
        trading_day: &'a [u8],
    ) -> Result<Self, FixEncodeError> {
        validate_value(FixTag::TEXT, trading_day)?;
        Ok(Self {
            session_id,
            next_inbound: clamp_seq_no(next_inbound),
            next_outbound: clamp_seq_no(next_outbound),
            trading_day,
        })
    }

    /// Returns the session id.
    pub const fn session_id(&self) -> FixSessionId<'a> {
        self.session_id
    }

    /// Returns the next inbound sequence number.
    pub const fn next_inbound(&self) -> u64 {
        self.next_inbound
    }

    /// Returns the next outbound sequence number.
    pub const fn next_outbound(&self) -> u64 {
        self.next_outbound
    }

    /// Returns the trading day or session date bytes.
    pub const fn trading_day(&self) -> &'a [u8] {
        self.trading_day
    }
}

/// Owned FIX session identity loaded from durable storage.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixOwnedSessionId {
    pub(crate) version: FixVersion,
    pub(crate) sender_comp_id: Vec<u8>,
    pub(crate) target_comp_id: Vec<u8>,
    pub(crate) qualifier: Vec<u8>,
}

impl FixOwnedSessionId {
    /// Creates an owned session id.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when an identifier contains SOH.
    pub fn new(
        version: FixVersion,
        sender_comp_id: impl Into<Vec<u8>>,
        target_comp_id: impl Into<Vec<u8>>,
    ) -> Result<Self, FixEncodeError> {
        Self::with_qualifier(version, sender_comp_id, target_comp_id, Vec::new())
    }

    /// Creates an owned session id with a qualifier.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when an identifier contains SOH.
    pub fn with_qualifier(
        version: FixVersion,
        sender_comp_id: impl Into<Vec<u8>>,
        target_comp_id: impl Into<Vec<u8>>,
        qualifier: impl Into<Vec<u8>>,
    ) -> Result<Self, FixEncodeError> {
        let sender_comp_id = sender_comp_id.into();
        let target_comp_id = target_comp_id.into();
        let qualifier = qualifier.into();
        validate_value(FixTag::SENDER_COMP_ID, &sender_comp_id)?;
        validate_value(FixTag::TARGET_COMP_ID, &target_comp_id)?;
        validate_value(FixTag::TEXT, &qualifier)?;
        Ok(Self {
            version,
            sender_comp_id,
            target_comp_id,
            qualifier,
        })
    }

    /// Creates an owned id from a borrowed session id.
    pub fn from_borrowed(session_id: FixSessionId<'_>) -> Self {
        Self {
            version: session_id.version(),
            sender_comp_id: session_id.sender_comp_id().to_vec(),
            target_comp_id: session_id.target_comp_id().to_vec(),
            qualifier: session_id.qualifier().to_vec(),
        }
    }

    /// Returns a borrowed session id view.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] if stored bytes are invalid.
    pub fn as_borrowed(&self) -> Result<FixSessionId<'_>, FixEncodeError> {
        FixSessionId::with_qualifier(
            self.version,
            &self.sender_comp_id,
            &self.target_comp_id,
            &self.qualifier,
        )
    }

    /// Returns the FIX version.
    pub const fn version(&self) -> FixVersion {
        self.version
    }

    /// Returns `SenderCompID(49)`.
    pub fn sender_comp_id(&self) -> &[u8] {
        &self.sender_comp_id
    }

    /// Returns `TargetCompID(56)`.
    pub fn target_comp_id(&self) -> &[u8] {
        &self.target_comp_id
    }

    /// Returns the optional session qualifier.
    pub fn qualifier(&self) -> &[u8] {
        &self.qualifier
    }
}

/// Owned persistable sequence-state snapshot loaded from storage.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixOwnedSequenceSnapshot {
    pub(crate) session_id: FixOwnedSessionId,
    pub(crate) next_inbound: u64,
    pub(crate) next_outbound: u64,
    pub(crate) trading_day: Vec<u8>,
    pub(crate) checksum: u64,
}

impl FixOwnedSequenceSnapshot {
    /// Creates an owned sequence snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when `trading_day` contains SOH.
    pub fn new(
        session_id: FixOwnedSessionId,
        next_inbound: u64,
        next_outbound: u64,
        trading_day: impl Into<Vec<u8>>,
    ) -> Result<Self, FixEncodeError> {
        let trading_day = trading_day.into();
        validate_value(FixTag::TEXT, &trading_day)?;
        let mut snapshot = Self {
            session_id,
            next_inbound: clamp_seq_no(next_inbound),
            next_outbound: clamp_seq_no(next_outbound),
            trading_day,
            checksum: 0,
        };
        snapshot.checksum = sequence_snapshot_checksum_owned(&snapshot);
        Ok(snapshot)
    }

    /// Creates an owned snapshot from a borrowed sequence snapshot.
    pub fn from_borrowed(snapshot: &FixSequenceSnapshot<'_>) -> Self {
        let mut owned = Self {
            session_id: FixOwnedSessionId::from_borrowed(snapshot.session_id()),
            next_inbound: snapshot.next_inbound(),
            next_outbound: snapshot.next_outbound(),
            trading_day: snapshot.trading_day().to_vec(),
            checksum: 0,
        };
        owned.checksum = sequence_snapshot_checksum_owned(&owned);
        owned
    }

    /// Returns a borrowed snapshot view.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] if stored identity or trading-day bytes are invalid.
    pub fn as_borrowed(&self) -> Result<FixSequenceSnapshot<'_>, FixEncodeError> {
        FixSequenceSnapshot::new(
            self.session_id.as_borrowed()?,
            self.next_inbound,
            self.next_outbound,
            &self.trading_day,
        )
    }

    /// Returns the owned session id.
    pub const fn session_id(&self) -> &FixOwnedSessionId {
        &self.session_id
    }

    /// Returns the next inbound sequence number.
    pub const fn next_inbound(&self) -> u64 {
        self.next_inbound
    }

    /// Returns the next outbound sequence number.
    pub const fn next_outbound(&self) -> u64 {
        self.next_outbound
    }

    /// Returns the trading day bytes.
    pub fn trading_day(&self) -> &[u8] {
        &self.trading_day
    }

    /// Returns the stored snapshot checksum.
    pub const fn checksum(&self) -> u64 {
        self.checksum
    }

    /// Returns true when the stored checksum matches the snapshot payload.
    pub fn validate_checksum(&self) -> bool {
        self.checksum == sequence_snapshot_checksum_owned(self)
    }
}

/// File-backed FIX sequence snapshot store configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixSequenceStoreConfig {
    pub(crate) root: PathBuf,
    pub(crate) sync_on_save: bool,
}

impl FixSequenceStoreConfig {
    /// Creates a sequence store config rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            sync_on_save: true,
        }
    }

    /// Sets whether snapshot files are synced before atomic rename.
    pub const fn with_sync_on_save(mut self, sync_on_save: bool) -> Self {
        self.sync_on_save = sync_on_save;
        self
    }

    /// Returns the sequence snapshot root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns whether save operations sync snapshot bytes.
    pub const fn sync_on_save(&self) -> bool {
        self.sync_on_save
    }
}

/// Metadata for an installed FIX sequence snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixSequenceSnapshotManifest {
    /// Snapshot file path.
    pub path: PathBuf,
    /// Encoded snapshot bytes.
    pub bytes: u64,
    /// Snapshot checksum.
    pub checksum: u64,
    /// Next inbound sequence number.
    pub next_inbound: u64,
    /// Next outbound sequence number.
    pub next_outbound: u64,
}

/// Error returned by FIX sequence snapshot persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSequenceStoreError {
    /// Filesystem operation failed.
    Io(String),
    /// Snapshot value validation failed.
    Encode(FixEncodeError),
    /// Snapshot file magic does not match the expected format.
    InvalidMagic,
    /// Snapshot file version is not supported.
    UnsupportedVersion(u16),
    /// Snapshot file ended before a complete field could be decoded.
    Truncated,
    /// Snapshot payload has an invalid known FIX begin string.
    InvalidVersion,
    /// Encoded field length exceeds the supported snapshot format.
    FieldTooLarge,
    /// Snapshot checksum does not match the encoded payload.
    ChecksumMismatch {
        /// Checksum stored in the snapshot file.
        expected: u64,
        /// Checksum recomputed from the decoded snapshot payload.
        actual: u64,
    },
}

impl fmt::Display for FixSequenceStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "FIX sequence store I/O error: {err}"),
            Self::Encode(err) => write!(f, "FIX sequence store encode error: {err}"),
            Self::InvalidMagic => write!(f, "invalid FIX sequence snapshot magic"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported FIX sequence snapshot version {version}")
            }
            Self::Truncated => write!(f, "truncated FIX sequence snapshot"),
            Self::InvalidVersion => write!(f, "invalid FIX begin string in sequence snapshot"),
            Self::FieldTooLarge => write!(f, "FIX sequence snapshot field is too large"),
            Self::ChecksumMismatch { expected, actual } => write!(
                f,
                "FIX sequence snapshot checksum mismatch: expected {expected}, actual {actual}"
            ),
        }
    }
}

impl Error for FixSequenceStoreError {}

impl From<FixEncodeError> for FixSequenceStoreError {
    fn from(value: FixEncodeError) -> Self {
        Self::Encode(value)
    }
}

/// FIX sequence snapshot persistence contract.
pub trait FixSequenceSnapshotStore {
    /// Saves a sequence snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceStoreError`] when validation or storage fails.
    fn save_snapshot(
        &mut self,
        snapshot: &FixSequenceSnapshot<'_>,
    ) -> Result<FixSequenceSnapshotManifest, FixSequenceStoreError>;

    /// Loads the latest sequence snapshot, if present.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceStoreError`] when the snapshot cannot be decoded or
    /// its checksum does not validate.
    fn load_latest(&self) -> Result<Option<FixOwnedSequenceSnapshot>, FixSequenceStoreError>;
}

/// Atomic file-backed FIX sequence snapshot store.
#[derive(Debug, Clone)]
pub struct FileFixSequenceSnapshotStore {
    pub(crate) config: FixSequenceStoreConfig,
}

impl FileFixSequenceSnapshotStore {
    /// Opens or creates a file-backed sequence snapshot store.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceStoreError`] when the root cannot be created.
    pub fn open(config: FixSequenceStoreConfig) -> Result<Self, FixSequenceStoreError> {
        fs::create_dir_all(config.root()).map_err(io_error)?;
        Ok(Self { config })
    }

    /// Returns the store configuration.
    pub const fn config(&self) -> &FixSequenceStoreConfig {
        &self.config
    }

    /// Returns the latest snapshot path.
    pub fn snapshot_path(&self) -> PathBuf {
        self.config.root().join(SEQUENCE_SNAPSHOT_FILE)
    }

    fn temp_path(&self) -> PathBuf {
        self.config.root().join(SEQUENCE_SNAPSHOT_TMP_FILE)
    }
}

impl FixSequenceSnapshotStore for FileFixSequenceSnapshotStore {
    fn save_snapshot(
        &mut self,
        snapshot: &FixSequenceSnapshot<'_>,
    ) -> Result<FixSequenceSnapshotManifest, FixSequenceStoreError> {
        let owned = FixOwnedSequenceSnapshot::from_borrowed(snapshot);
        let bytes = encode_sequence_snapshot(&owned)?;
        let checksum = owned.checksum();
        let final_path = self.snapshot_path();
        let tmp_path = self.temp_path();

        {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&tmp_path)
                .map_err(io_error)?;
            file.write_all(&bytes).map_err(io_error)?;
            file.flush().map_err(io_error)?;
            if self.config.sync_on_save() {
                file.sync_all().map_err(io_error)?;
            }
        }

        fs::rename(&tmp_path, &final_path).map_err(io_error)?;

        Ok(FixSequenceSnapshotManifest {
            path: final_path,
            bytes: usize_to_u64(bytes.len()),
            checksum,
            next_inbound: owned.next_inbound(),
            next_outbound: owned.next_outbound(),
        })
    }

    fn load_latest(&self) -> Result<Option<FixOwnedSequenceSnapshot>, FixSequenceStoreError> {
        let path = self.snapshot_path();
        if !path.exists() {
            return Ok(None);
        }
        let mut file = File::open(path).map_err(io_error)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(io_error)?;
        decode_sequence_snapshot(&bytes).map(Some)
    }
}

/// Classification for outbound messages retained for resend handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum FixSentMessageKind {
    /// Application-level order or execution-flow message that may be replayed.
    Application,
    /// Session administrative message that should normally be gap-filled.
    Administrative,
    /// Session-level reject. FIX recovery rules allow reject messages to be
    /// replayed when a profile chooses to retain them.
    Reject,
}

impl FixSentMessageKind {
    /// Returns whether this message kind is replayable by default.
    pub const fn replayable(self) -> bool {
        matches!(self, Self::Application | Self::Reject)
    }
}

/// Bounded resend-store configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixResendStoreConfig {
    pub(crate) max_messages: usize,
    pub(crate) max_bytes: usize,
}

impl FixResendStoreConfig {
    /// Creates a bounded resend-store configuration.
    ///
    /// A zero `max_messages` or `max_bytes` disables retention while keeping
    /// counters observable through [`FixResendStore::metrics`].
    pub const fn new(max_messages: usize, max_bytes: usize) -> Self {
        Self {
            max_messages,
            max_bytes,
        }
    }

    /// Returns the maximum retained message count.
    pub const fn max_messages(&self) -> usize {
        self.max_messages
    }

    /// Returns the maximum retained raw-byte count.
    pub const fn max_bytes(&self) -> usize {
        self.max_bytes
    }
}

impl Default for FixResendStoreConfig {
    fn default() -> Self {
        Self {
            max_messages: 1024,
            max_bytes: 1024 * 1024,
        }
    }
}

/// Resend-store append errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixResendStoreError {
    /// FIX sequence numbers are one-based.
    ZeroSeqNo,
    /// A retained outbound message was recorded out of order or reused a
    /// sequence number already observed by the store.
    SequenceRegression {
        /// Latest retained or observed outbound sequence.
        latest: u64,
        /// Sequence number supplied by the caller.
        received: u64,
    },
}

impl fmt::Display for FixResendStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSeqNo => write!(
                f,
                "FIX resend store sequence number must be greater than zero"
            ),
            Self::SequenceRegression { latest, received } => write!(
                f,
                "FIX resend store sequence regression: latest {latest}, received {received}"
            ),
        }
    }
}

impl Error for FixResendStoreError {}

/// File-backed durable resend-message store configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixDurableResendStoreConfig {
    pub(crate) path: PathBuf,
    pub(crate) sync_on_record: bool,
}

impl FixDurableResendStoreConfig {
    /// Creates a durable resend-store config for `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            sync_on_record: true,
        }
    }

    /// Sets whether each appended resend record is synced before returning.
    pub const fn with_sync_on_record(mut self, sync_on_record: bool) -> Self {
        self.sync_on_record = sync_on_record;
        self
    }

    /// Returns the durable resend log path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns whether append operations sync record bytes.
    pub const fn sync_on_record(&self) -> bool {
        self.sync_on_record
    }
}

/// Metadata for one durable resend append.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixDurableResendAppend {
    /// Outbound `MsgSeqNum(34)` recorded by this append.
    pub seq_no: u64,
    /// Message replayability classification.
    pub kind: FixSentMessageKind,
    /// Byte offset where the encoded durable frame starts.
    pub offset: u64,
    /// Encoded durable frame length.
    pub bytes: u64,
    /// Rolling checksum after this append.
    pub checksum: u64,
}

/// Summary produced by replaying durable resend frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct FixDurableResendReplayReport {
    /// Durable records decoded.
    pub records: u64,
    /// Encoded durable bytes consumed.
    pub bytes: u64,
    /// First decoded outbound sequence number.
    pub first_seq_no: Option<u64>,
    /// Last decoded outbound sequence number.
    pub last_seq_no: Option<u64>,
    /// Rolling checksum after the last decoded record.
    pub checksum: u64,
    /// Messages retained by the target in-memory resend store.
    pub retained_messages: u64,
    /// Messages dropped by the target in-memory resend store.
    pub dropped_messages: u64,
}

/// Error returned by durable FIX resend-message persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixDurableResendStoreError {
    /// Filesystem operation failed.
    Io(String),
    /// The in-memory resend store rejected a decoded record.
    ResendStore(FixResendStoreError),
    /// Durable frame magic does not match the expected format.
    InvalidMagic,
    /// Durable frame version is not supported.
    UnsupportedVersion(u16),
    /// Durable frame ended before a complete field could be decoded.
    Truncated,
    /// Durable frame message kind is unknown.
    InvalidKind(u8),
    /// Durable frame raw payload exceeds the supported format.
    FrameTooLarge,
    /// Durable frame sequence regressed or repeated.
    SequenceRegression {
        /// Latest decoded outbound sequence.
        latest: u64,
        /// Sequence number supplied by the durable frame.
        received: u64,
    },
    /// Durable frame raw hash does not match the payload bytes.
    RawHashMismatch {
        /// Hash stored in the durable frame.
        expected: u64,
        /// Hash recomputed from the raw FIX frame bytes.
        actual: u64,
    },
    /// Durable frame checksum chain is broken.
    PreviousChecksumMismatch {
        /// Previous checksum stored in the durable frame.
        expected: u64,
        /// Checksum produced by the previous decoded frame.
        actual: u64,
    },
    /// Durable frame checksum does not match the encoded frame payload.
    FrameChecksumMismatch {
        /// Checksum stored in the durable frame.
        expected: u64,
        /// Checksum recomputed from the durable frame payload.
        actual: u64,
    },
}

impl fmt::Display for FixDurableResendStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "FIX durable resend store I/O error: {err}"),
            Self::ResendStore(err) => write!(f, "FIX durable resend replay error: {err}"),
            Self::InvalidMagic => write!(f, "invalid FIX durable resend frame magic"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported FIX durable resend frame version {version}")
            }
            Self::Truncated => write!(f, "truncated FIX durable resend frame"),
            Self::InvalidKind(kind) => write!(f, "invalid FIX durable resend kind {kind}"),
            Self::FrameTooLarge => write!(f, "FIX durable resend frame is too large"),
            Self::SequenceRegression { latest, received } => write!(
                f,
                "FIX durable resend sequence regression: latest {latest}, received {received}"
            ),
            Self::RawHashMismatch { expected, actual } => write!(
                f,
                "FIX durable resend raw hash mismatch: expected {expected}, actual {actual}"
            ),
            Self::PreviousChecksumMismatch { expected, actual } => write!(
                f,
                "FIX durable resend previous checksum mismatch: expected {expected}, actual {actual}"
            ),
            Self::FrameChecksumMismatch { expected, actual } => write!(
                f,
                "FIX durable resend frame checksum mismatch: expected {expected}, actual {actual}"
            ),
        }
    }
}

impl Error for FixDurableResendStoreError {}

impl From<FixResendStoreError> for FixDurableResendStoreError {
    fn from(value: FixResendStoreError) -> Self {
        Self::ResendStore(value)
    }
}

/// Durable resend-message persistence contract.
pub trait FixDurableResendMessageStore {
    /// Records an original outbound FIX frame for future resend handling.
    ///
    /// # Errors
    ///
    /// Returns [`FixDurableResendStoreError`] when validation or storage fails.
    fn record_sent(
        &mut self,
        seq_no: u64,
        kind: FixSentMessageKind,
        raw: &[u8],
    ) -> Result<FixDurableResendAppend, FixDurableResendStoreError>;

    /// Replays durable records into an in-memory resend store.
    ///
    /// # Errors
    ///
    /// Returns [`FixDurableResendStoreError`] when the durable log cannot be
    /// decoded or the target resend store rejects a record.
    fn load_into(
        &self,
        target: &mut FixResendStore,
    ) -> Result<FixDurableResendReplayReport, FixDurableResendStoreError>;
}

/// Append-only file-backed durable FIX resend-message store.
#[derive(Debug)]
pub struct FileFixDurableResendStore {
    pub(crate) config: FixDurableResendStoreConfig,
    pub(crate) file: File,
    pub(crate) scratch: Vec<u8>,
    pub(crate) records: u64,
    pub(crate) bytes: u64,
    pub(crate) newest_seq_no: Option<u64>,
    pub(crate) previous_checksum: u64,
}

impl FileFixDurableResendStore {
    /// Opens or creates an append-only durable resend-message store.
    ///
    /// Existing bytes are validated before new records can be appended.
    ///
    /// # Errors
    ///
    /// Returns [`FixDurableResendStoreError`] when parent directories cannot be
    /// created, the log cannot be opened, or existing frames fail validation.
    pub fn open(config: FixDurableResendStoreConfig) -> Result<Self, FixDurableResendStoreError> {
        if let Some(parent) = config.path().parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(durable_resend_io_error)?;
            }
        }
        let report = inspect_durable_resend_path(config.path())?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(config.path())
            .map_err(durable_resend_io_error)?;
        file.seek(SeekFrom::End(0))
            .map_err(durable_resend_io_error)?;
        Ok(Self {
            config,
            file,
            scratch: Vec::with_capacity(512),
            records: report.records,
            bytes: report.bytes,
            newest_seq_no: report.last_seq_no,
            previous_checksum: report.checksum,
        })
    }

    /// Returns the durable resend-store configuration.
    pub const fn config(&self) -> &FixDurableResendStoreConfig {
        &self.config
    }

    /// Inspects a durable resend log without opening it for append.
    ///
    /// # Errors
    ///
    /// Returns [`FixDurableResendStoreError`] when the file cannot be read or
    /// any durable frame fails validation.
    pub fn inspect_path(
        path: impl AsRef<Path>,
    ) -> Result<FixDurableResendReplayReport, FixDurableResendStoreError> {
        inspect_durable_resend_path(path.as_ref())
    }
}

impl FixDurableResendMessageStore for FileFixDurableResendStore {
    fn record_sent(
        &mut self,
        seq_no: u64,
        kind: FixSentMessageKind,
        raw: &[u8],
    ) -> Result<FixDurableResendAppend, FixDurableResendStoreError> {
        if seq_no == 0 {
            return Err(FixDurableResendStoreError::ResendStore(
                FixResendStoreError::ZeroSeqNo,
            ));
        }
        if let Some(latest) = self.newest_seq_no {
            if seq_no <= latest {
                return Err(FixDurableResendStoreError::SequenceRegression {
                    latest,
                    received: seq_no,
                });
            }
        }

        let offset = self.bytes;
        encode_durable_resend_record(&mut self.scratch, seq_no, kind, raw, self.previous_checksum)?;
        self.file
            .write_all(&self.scratch)
            .map_err(durable_resend_io_error)?;
        self.file.flush().map_err(durable_resend_io_error)?;
        if self.config.sync_on_record() {
            self.file.sync_data().map_err(durable_resend_io_error)?;
        }

        let bytes = usize_to_u64(self.scratch.len());
        let checksum = durable_resend_frame_checksum(seq_no, kind, raw, self.previous_checksum);
        self.records = self.records.saturating_add(1);
        self.bytes = self.bytes.saturating_add(bytes);
        self.newest_seq_no = Some(seq_no);
        self.previous_checksum = checksum;

        Ok(FixDurableResendAppend {
            seq_no,
            kind,
            offset,
            bytes,
            checksum,
        })
    }

    fn load_into(
        &self,
        target: &mut FixResendStore,
    ) -> Result<FixDurableResendReplayReport, FixDurableResendStoreError> {
        let records = read_durable_resend_records(self.config.path())?;
        let mut report = FixDurableResendReplayReport::default();
        for record in records {
            report.records = report.records.saturating_add(1);
            report.bytes = report.bytes.saturating_add(record.encoded_bytes);
            report.first_seq_no.get_or_insert(record.seq_no);
            report.last_seq_no = Some(record.seq_no);
            report.checksum = record.checksum;
            target.record_sent(record.seq_no, record.kind, &record.raw)?;
        }
        let metrics = target.metrics();
        report.retained_messages = metrics.retained_messages();
        report.dropped_messages = metrics.dropped_messages();
        Ok(report)
    }
}

/// Retained outbound FIX frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixStoredMessage {
    pub(crate) seq_no: u64,
    pub(crate) kind: FixSentMessageKind,
    pub(crate) raw: Vec<u8>,
}

impl FixStoredMessage {
    /// Returns the outbound `MsgSeqNum(34)`.
    pub const fn seq_no(&self) -> u64 {
        self.seq_no
    }

    /// Returns the retained message kind.
    pub const fn kind(&self) -> FixSentMessageKind {
        self.kind
    }

    /// Returns the retained raw FIX frame.
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// Returns whether the message is replayable by default.
    pub const fn replayable(&self) -> bool {
        self.kind.replayable()
    }
}

/// Result of recording a sent message into a resend store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixResendRetention {
    pub(crate) retained: bool,
    pub(crate) evicted_messages: u64,
    pub(crate) evicted_bytes: u64,
}

impl FixResendRetention {
    /// Returns whether the message was retained.
    pub const fn retained(&self) -> bool {
        self.retained
    }

    /// Returns messages evicted to satisfy configured bounds.
    pub const fn evicted_messages(&self) -> u64 {
        self.evicted_messages
    }

    /// Returns bytes evicted to satisfy configured bounds.
    pub const fn evicted_bytes(&self) -> u64 {
        self.evicted_bytes
    }
}

/// Snapshot of resend-store counters and retained range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixResendStoreMetrics {
    pub(crate) retained_messages: u64,
    pub(crate) retained_bytes: u64,
    pub(crate) dropped_messages: u64,
    pub(crate) dropped_bytes: u64,
    pub(crate) evicted_messages: u64,
    pub(crate) evicted_bytes: u64,
    pub(crate) oldest_seq_no: Option<u64>,
    pub(crate) newest_seq_no: Option<u64>,
}

impl FixResendStoreMetrics {
    /// Returns the number of retained messages.
    pub const fn retained_messages(&self) -> u64 {
        self.retained_messages
    }

    /// Returns the number of retained raw bytes.
    pub const fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// Returns messages dropped because retention was disabled or the frame
    /// exceeded the byte budget.
    pub const fn dropped_messages(&self) -> u64 {
        self.dropped_messages
    }

    /// Returns bytes dropped because retention was disabled or the frame
    /// exceeded the byte budget.
    pub const fn dropped_bytes(&self) -> u64 {
        self.dropped_bytes
    }

    /// Returns messages evicted by bounded retention.
    pub const fn evicted_messages(&self) -> u64 {
        self.evicted_messages
    }

    /// Returns bytes evicted by bounded retention.
    pub const fn evicted_bytes(&self) -> u64 {
        self.evicted_bytes
    }

    /// Returns the oldest retained outbound sequence number.
    pub const fn oldest_seq_no(&self) -> Option<u64> {
        self.oldest_seq_no
    }

    /// Returns the newest observed outbound sequence number.
    pub const fn newest_seq_no(&self) -> Option<u64> {
        self.newest_seq_no
    }
}

/// One planned response for an outbound resend request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixResendAction<'a> {
    /// Replay a retained application or reject frame.
    Replay {
        /// Original outbound sequence number.
        seq_no: u64,
        /// Retained raw FIX frame.
        raw: &'a [u8],
    },
    /// Emit a SequenceReset `<4>` gap-fill for an inclusive sequence range.
    GapFill {
        /// First skipped sequence number.
        begin_seq_no: u64,
        /// Last skipped sequence number.
        end_seq_no: u64,
    },
}

/// Summary produced while planning a resend response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixResendPlanSummary {
    pub(crate) replay_messages: u64,
    pub(crate) gap_fill_messages: u64,
    pub(crate) gap_fill_sequences: u64,
}

impl FixResendPlanSummary {
    /// Returns replay actions produced by the planner.
    pub const fn replay_messages(&self) -> u64 {
        self.replay_messages
    }

    /// Returns gap-fill actions produced by the planner.
    pub const fn gap_fill_messages(&self) -> u64 {
        self.gap_fill_messages
    }

    /// Returns total skipped sequence numbers covered by gap fills.
    pub const fn gap_fill_sequences(&self) -> u64 {
        self.gap_fill_sequences
    }
}
