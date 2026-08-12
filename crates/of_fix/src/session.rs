//! Deterministic, transport-independent FIX session coordination.

use std::error::Error;
use std::fmt;

use crate::{
    encode_heartbeat, encode_logon, encode_logout, encode_resend_request, encode_test_request,
    FixEncodeError, FixMessageView, FixMsgType, FixOwnedSessionId, FixResendRange,
    FixSequenceAction, FixSequenceError, FixSequenceTracker, FixSessionHeader, FixSessionState,
    FixTag,
};

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const TEST_REQUEST_ID_CAPACITY: usize = 32;
const TEST_REQUEST_PREFIX: &[u8] = b"OF-TEST-";

/// Configuration for a deterministic FIX session engine.
///
/// The engine owns protocol state but not a socket, TLS implementation, clock,
/// scheduler, or persistence backend. Hosts inject timestamps and transmit the
/// caller-owned output buffer returned by each action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixSessionEngineConfig {
    heartbeat_interval_secs: u32,
    heartbeat_interval_ns: u64,
    test_request_after_ns: u64,
    disconnect_after_test_request_ns: u64,
    logout_timeout_ns: u64,
    reset_seq_num_on_logon: bool,
    validate_comp_ids: bool,
}

impl FixSessionEngineConfig {
    /// Creates a session configuration from the negotiated heartbeat interval.
    ///
    /// By default the engine sends a TestRequest after 1.5 heartbeat intervals
    /// without inbound traffic and requests disconnect one heartbeat interval
    /// after an unanswered TestRequest. Logout uses the same one-heartbeat
    /// timeout.
    ///
    /// # Errors
    ///
    /// Returns [`FixSessionConfigError`] when the interval is zero or cannot be
    /// represented in nanoseconds.
    pub fn new(heartbeat_interval_secs: u32) -> Result<Self, FixSessionConfigError> {
        let heartbeat_interval_ns = u64::from(heartbeat_interval_secs)
            .checked_mul(NANOS_PER_SECOND)
            .ok_or(FixSessionConfigError::DurationOverflow)?;
        if heartbeat_interval_ns == 0 {
            return Err(FixSessionConfigError::ZeroHeartbeatInterval);
        }
        let half = heartbeat_interval_ns / 2;
        let test_request_after_ns = heartbeat_interval_ns
            .checked_add(half)
            .ok_or(FixSessionConfigError::DurationOverflow)?;
        let disconnect_after_test_request_ns = heartbeat_interval_ns;
        Ok(Self {
            heartbeat_interval_secs,
            heartbeat_interval_ns,
            test_request_after_ns,
            disconnect_after_test_request_ns,
            logout_timeout_ns: heartbeat_interval_ns,
            reset_seq_num_on_logon: false,
            validate_comp_ids: true,
        })
    }

    /// Overrides liveness and logout durations in nanoseconds.
    ///
    /// `test_request_after_ns` is measured from the latest inbound frame.
    /// `disconnect_after_test_request_ns` is measured from the TestRequest send
    /// time. All durations must be non-zero.
    ///
    /// # Errors
    ///
    /// Returns [`FixSessionConfigError`] when any duration is zero.
    pub fn with_timeouts(
        mut self,
        test_request_after_ns: u64,
        disconnect_after_test_request_ns: u64,
        logout_timeout_ns: u64,
    ) -> Result<Self, FixSessionConfigError> {
        if test_request_after_ns == 0 {
            return Err(FixSessionConfigError::ZeroTestRequestTimeout);
        }
        if disconnect_after_test_request_ns == 0 {
            return Err(FixSessionConfigError::ZeroDisconnectTimeout);
        }
        if logout_timeout_ns == 0 {
            return Err(FixSessionConfigError::ZeroLogoutTimeout);
        }
        self.test_request_after_ns = test_request_after_ns;
        self.disconnect_after_test_request_ns = disconnect_after_test_request_ns;
        self.logout_timeout_ns = logout_timeout_ns;
        Ok(self)
    }

    /// Configures whether the next connection sends `ResetSeqNumFlag(141)=Y`.
    ///
    /// When enabled, both local counters are reset to one immediately before
    /// the Logon sequence is assigned. This must only be enabled under an
    /// explicit bilateral session-reset agreement.
    pub const fn with_reset_seq_num_on_logon(mut self, enabled: bool) -> Self {
        self.reset_seq_num_on_logon = enabled;
        self
    }

    /// Configures strict sender/target component-id validation.
    ///
    /// Validation is enabled by default. Disabling it is intended only for
    /// gateways whose authenticated transport rewrites component identifiers.
    pub const fn with_comp_id_validation(mut self, enabled: bool) -> Self {
        self.validate_comp_ids = enabled;
        self
    }

    /// Returns the negotiated `HeartBtInt(108)` value in seconds.
    pub const fn heartbeat_interval_secs(&self) -> u32 {
        self.heartbeat_interval_secs
    }

    /// Returns the outbound-heartbeat idle duration in nanoseconds.
    pub const fn heartbeat_interval_ns(&self) -> u64 {
        self.heartbeat_interval_ns
    }

    /// Returns the inbound-idle duration before a TestRequest is sent.
    pub const fn test_request_after_ns(&self) -> u64 {
        self.test_request_after_ns
    }

    /// Returns the unanswered-TestRequest duration before disconnect is
    /// requested.
    pub const fn disconnect_after_test_request_ns(&self) -> u64 {
        self.disconnect_after_test_request_ns
    }

    /// Returns the Logout response timeout in nanoseconds.
    pub const fn logout_timeout_ns(&self) -> u64 {
        self.logout_timeout_ns
    }

    /// Returns whether Logon requests a bilateral sequence reset.
    pub const fn reset_seq_num_on_logon(&self) -> bool {
        self.reset_seq_num_on_logon
    }

    /// Returns whether inbound component identifiers are validated.
    pub const fn validate_comp_ids(&self) -> bool {
        self.validate_comp_ids
    }
}

/// Invalid FIX session-engine configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSessionConfigError {
    /// `HeartBtInt` was zero.
    ZeroHeartbeatInterval,
    /// The TestRequest idle timeout was zero.
    ZeroTestRequestTimeout,
    /// The unanswered-TestRequest timeout was zero.
    ZeroDisconnectTimeout,
    /// The Logout timeout was zero.
    ZeroLogoutTimeout,
    /// A seconds-to-nanoseconds conversion overflowed.
    DurationOverflow,
}

impl fmt::Display for FixSessionConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroHeartbeatInterval => f.write_str("FIX heartbeat interval must be non-zero"),
            Self::ZeroTestRequestTimeout => {
                f.write_str("FIX TestRequest idle timeout must be non-zero")
            }
            Self::ZeroDisconnectTimeout => {
                f.write_str("FIX TestRequest disconnect timeout must be non-zero")
            }
            Self::ZeroLogoutTimeout => f.write_str("FIX Logout timeout must be non-zero"),
            Self::DurationOverflow => f.write_str("FIX session duration overflow"),
        }
    }
}

impl Error for FixSessionConfigError {}

/// Administrative message kind emitted into the caller-owned output buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSessionSendKind {
    /// Logon `<A>`.
    Logon,
    /// Heartbeat `<0>`.
    Heartbeat,
    /// TestRequest `<1>`.
    TestRequest,
    /// ResendRequest `<2>`.
    ResendRequest,
    /// Logout `<5>`.
    Logout,
}

/// Reason the session asks its host to close the transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSessionDisconnectReason {
    /// A TestRequest was not answered within the configured timeout.
    HeartbeatTimeout,
    /// A Logout handshake did not complete before its timeout.
    LogoutTimeout,
    /// The peer acknowledged a locally initiated Logout.
    PeerLogout,
}

/// Deterministic action produced by a FIX session-engine call.
///
/// `Send` and `GapDetected` actions populate the mutable output buffer supplied
/// to the call. The host must persist/transmit those bytes before invoking the
/// next mutating session operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSessionAction {
    /// No protocol action is currently due.
    None,
    /// An administrative frame was encoded for transmission.
    Send {
        /// Encoded administrative message kind.
        kind: FixSessionSendKind,
        /// Outbound FIX sequence number in the encoded frame.
        sequence: u64,
    },
    /// Peer Logon was accepted and application flow may begin.
    Ready {
        /// Inbound Logon sequence number.
        sequence: u64,
    },
    /// An application message passed session validation.
    Application {
        /// Accepted inbound sequence number.
        sequence: u64,
    },
    /// A possible duplicate was observed below the current inbound horizon.
    Duplicate {
        /// Duplicate sequence number.
        sequence: u64,
        /// Next inbound sequence expected by the session.
        expected: u64,
    },
    /// An inbound gap caused a ResendRequest to be encoded.
    GapDetected {
        /// Out-of-order inbound sequence that exposed the gap.
        received: u64,
        /// Missing inbound range requested from the peer.
        range: FixResendRange,
        /// Sequence number assigned to the outbound ResendRequest.
        request_sequence: u64,
    },
    /// The peer requested replay or gap-fill for an outbound range.
    PeerResendRequested {
        /// Inbound ResendRequest sequence number.
        sequence: u64,
        /// Requested outbound sequence range. `end_seq_no == 0` means current
        /// end of retained history.
        range: FixResendRange,
    },
    /// A SequenceReset was applied to the inbound horizon.
    SequenceReset {
        /// Inbound SequenceReset message sequence number.
        sequence: u64,
        /// New next inbound sequence number.
        next_inbound: u64,
    },
    /// A non-flow administrative message was accepted.
    Administrative {
        /// Accepted inbound sequence number.
        sequence: u64,
        /// Known administrative message kind.
        message_type: FixMsgType,
    },
    /// The host should close the transport.
    Disconnect {
        /// Protocol reason for the disconnect request.
        reason: FixSessionDisconnectReason,
    },
}

/// Allocation-free FIX session counters and timing snapshot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixSessionMetrics {
    /// Transport connections that reached Logon emission.
    pub connections: u64,
    /// Transport disconnect notifications.
    pub disconnects: u64,
    /// Accepted inbound frames, including administrative frames.
    pub inbound_messages: u64,
    /// Newly sequenced outbound frames.
    pub outbound_messages: u64,
    /// Accepted inbound application frames.
    pub inbound_application_messages: u64,
    /// Assigned outbound application frames.
    pub outbound_application_messages: u64,
    /// Inbound sequence gaps.
    pub sequence_gaps: u64,
    /// Possible-duplicate inbound frames.
    pub duplicate_messages: u64,
    /// Unflagged inbound sequence regressions.
    pub sequence_too_low: u64,
    /// ResendRequests sent after inbound gaps.
    pub resend_requests_sent: u64,
    /// ResendRequests accepted from the peer.
    pub resend_requests_received: u64,
    /// Heartbeats sent, including TestRequest responses.
    pub heartbeats_sent: u64,
    /// Heartbeats accepted from the peer.
    pub heartbeats_received: u64,
    /// TestRequests sent by the liveness timer.
    pub test_requests_sent: u64,
    /// TestRequests accepted from the peer.
    pub test_requests_received: u64,
    /// Logon messages sent.
    pub logons_sent: u64,
    /// Logon messages accepted.
    pub logons_received: u64,
    /// Logout messages sent.
    pub logouts_sent: u64,
    /// Logout messages accepted.
    pub logouts_received: u64,
    /// Liveness or Logout timeout disconnect requests.
    pub timeout_disconnects: u64,
    /// Latest accepted/observed inbound timestamp.
    pub last_inbound_ns: Option<u64>,
    /// Latest newly sequenced outbound timestamp.
    pub last_outbound_ns: Option<u64>,
}

/// FIX session protocol and state-machine errors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixSessionError {
    /// Operation is invalid in the current lifecycle state.
    InvalidState {
        /// Attempted operation name.
        operation: &'static str,
        /// Current state.
        state: FixSessionState,
    },
    /// A caller-supplied timestamp moved backwards.
    TimestampRegression {
        /// Latest timestamp previously observed.
        previous_ns: u64,
        /// Regressing timestamp.
        observed_ns: u64,
    },
    /// A required session tag was absent.
    MissingTag(FixTag),
    /// A required numeric session tag was malformed or overflowed.
    MalformedNumericTag(FixTag),
    /// Inbound begin string did not match the configured session.
    VersionMismatch,
    /// Inbound sender component id did not match the configured target.
    SenderCompIdMismatch,
    /// Inbound target component id did not match the configured sender.
    TargetCompIdMismatch,
    /// Peer Logon used a different heartbeat interval.
    HeartbeatIntervalMismatch {
        /// Locally configured interval in seconds.
        expected: u32,
        /// Peer-provided interval in seconds.
        received: u64,
    },
    /// Peer requested a sequence reset without a bilateral reset policy.
    UnexpectedResetSeqNumFlag,
    /// An inbound sequence regressed without `PossDupFlag(43)=Y`.
    SequenceTooLow {
        /// Current expected inbound sequence.
        expected: u64,
        /// Received sequence.
        received: u64,
    },
    /// A ResendRequest range was malformed.
    InvalidResendRange {
        /// Requested beginning sequence.
        begin_seq_no: u64,
        /// Requested ending sequence.
        end_seq_no: u64,
    },
    /// A heartbeat did not echo the outstanding TestReqID.
    TestRequestIdMismatch,
    /// Application flow arrived outside a ready/recovery state.
    UnexpectedApplicationMessage,
    /// FIX message encoding failed.
    Encode(FixEncodeError),
    /// FIX sequence validation failed.
    Sequence(FixSequenceError),
}

impl fmt::Display for FixSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState { operation, state } => {
                write!(
                    f,
                    "FIX session operation {operation} is invalid in state {state:?}"
                )
            }
            Self::TimestampRegression {
                previous_ns,
                observed_ns,
            } => write!(
                f,
                "FIX session timestamp regressed from {previous_ns} to {observed_ns}"
            ),
            Self::MissingTag(tag) => write!(f, "FIX session message is missing tag {tag}"),
            Self::MalformedNumericTag(tag) => {
                write!(f, "FIX session tag {tag} is not a valid unsigned integer")
            }
            Self::VersionMismatch => f.write_str("FIX session begin string mismatch"),
            Self::SenderCompIdMismatch => f.write_str("FIX session SenderCompID mismatch"),
            Self::TargetCompIdMismatch => f.write_str("FIX session TargetCompID mismatch"),
            Self::HeartbeatIntervalMismatch { expected, received } => write!(
                f,
                "FIX HeartBtInt mismatch: expected {expected}, received {received}"
            ),
            Self::UnexpectedResetSeqNumFlag => {
                f.write_str("FIX peer requested an unconfigured sequence reset")
            }
            Self::SequenceTooLow { expected, received } => write!(
                f,
                "FIX inbound sequence {received} is below expected {expected} without PossDupFlag"
            ),
            Self::InvalidResendRange {
                begin_seq_no,
                end_seq_no,
            } => write!(f, "invalid FIX resend range {begin_seq_no}..={end_seq_no}"),
            Self::TestRequestIdMismatch => f.write_str("FIX Heartbeat TestReqID mismatch"),
            Self::UnexpectedApplicationMessage => {
                f.write_str("FIX application message arrived before session readiness")
            }
            Self::Encode(error) => write!(f, "FIX session encode error: {error}"),
            Self::Sequence(error) => write!(f, "FIX session sequence error: {error}"),
        }
    }
}

impl Error for FixSessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::Sequence(error) => Some(error),
            _ => None,
        }
    }
}

impl From<FixEncodeError> for FixSessionError {
    fn from(value: FixEncodeError) -> Self {
        Self::Encode(value)
    }
}

impl From<FixSequenceError> for FixSessionError {
    fn from(value: FixSequenceError) -> Self {
        Self::Sequence(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingTestRequest {
    bytes: [u8; TEST_REQUEST_ID_CAPACITY],
    len: u8,
    sent_ns: u64,
}

impl PendingTestRequest {
    fn id(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}

/// Single-owner deterministic FIX session state machine.
///
/// The engine performs no network, filesystem, wall-clock, thread, or async
/// operations. It reuses caller-owned output buffers and stores its outstanding
/// TestReqID inline. External synchronization is required if multiple threads
/// access one instance.
#[derive(Debug, Clone)]
pub struct FixSessionEngine {
    config: FixSessionEngineConfig,
    session_id: FixOwnedSessionId,
    sequences: FixSequenceTracker,
    state: FixSessionState,
    metrics: FixSessionMetrics,
    last_observed_ns: Option<u64>,
    pending_test_request: Option<PendingTestRequest>,
    next_test_request_id: u64,
    logout_sent_ns: Option<u64>,
    recovery_target: Option<u64>,
    disconnect_requested: bool,
}

impl FixSessionEngine {
    /// Creates a disconnected session with sequence numbers starting at one.
    pub fn new(config: FixSessionEngineConfig, session_id: FixOwnedSessionId) -> Self {
        Self::with_sequences(config, session_id, FixSequenceTracker::new())
    }

    /// Creates a disconnected session from restored sequence counters.
    pub fn with_sequences(
        config: FixSessionEngineConfig,
        session_id: FixOwnedSessionId,
        sequences: FixSequenceTracker,
    ) -> Self {
        Self {
            config,
            session_id,
            sequences,
            state: FixSessionState::Disconnected,
            metrics: FixSessionMetrics::default(),
            last_observed_ns: None,
            pending_test_request: None,
            next_test_request_id: 1,
            logout_sent_ns: None,
            recovery_target: None,
            disconnect_requested: false,
        }
    }

    /// Returns immutable session configuration.
    pub const fn config(&self) -> &FixSessionEngineConfig {
        &self.config
    }

    /// Returns the owned session identity.
    pub const fn session_id(&self) -> &FixOwnedSessionId {
        &self.session_id
    }

    /// Returns current lifecycle state.
    pub const fn state(&self) -> FixSessionState {
        self.state
    }

    /// Returns current sequence counters.
    pub const fn sequences(&self) -> &FixSequenceTracker {
        &self.sequences
    }

    /// Returns an allocation-free metrics snapshot.
    pub const fn metrics(&self) -> FixSessionMetrics {
        self.metrics
    }

    /// Marks the beginning of a transport connection attempt.
    ///
    /// # Errors
    ///
    /// Returns [`FixSessionError::InvalidState`] unless currently disconnected
    /// or stopped.
    pub fn on_transport_connecting(&mut self) -> Result<(), FixSessionError> {
        match self.state {
            FixSessionState::Disconnected | FixSessionState::Stopped => {
                self.state = FixSessionState::Connecting;
                self.disconnect_requested = false;
                Ok(())
            }
            state => Err(FixSessionError::InvalidState {
                operation: "transport_connecting",
                state,
            }),
        }
    }

    /// Encodes Logon after the transport has connected.
    ///
    /// `sending_time` is the caller-formatted FIX UTC timestamp. `out` is
    /// cleared and reused.
    ///
    /// # Errors
    ///
    /// Returns [`FixSessionError`] for invalid lifecycle/timestamp state or
    /// encoding failure.
    pub fn on_transport_connected(
        &mut self,
        now_ns: u64,
        sending_time: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<FixSessionAction, FixSessionError> {
        match self.state {
            FixSessionState::Disconnected
            | FixSessionState::Connecting
            | FixSessionState::Stopped => {}
            state => {
                return Err(FixSessionError::InvalidState {
                    operation: "transport_connected",
                    state,
                })
            }
        }
        validate_wire_value(FixTag::SENDING_TIME, sending_time)?;
        self.observe_time(now_ns)?;
        if self.config.reset_seq_num_on_logon {
            self.sequences.reset_to_one();
        }
        let sequence = self.sequences.assign_outbound();
        let header = self.header(sequence, sending_time);
        encode_logon(
            out,
            self.session_id.version(),
            header,
            u64::from(self.config.heartbeat_interval_secs),
            self.config.reset_seq_num_on_logon,
        )?;
        self.state = FixSessionState::LogonSent;
        self.pending_test_request = None;
        self.logout_sent_ns = None;
        self.recovery_target = None;
        self.disconnect_requested = false;
        self.metrics.connections = self.metrics.connections.saturating_add(1);
        self.metrics.logons_sent = self.metrics.logons_sent.saturating_add(1);
        self.mark_outbound(now_ns, false);
        Ok(FixSessionAction::Send {
            kind: FixSessionSendKind::Logon,
            sequence,
        })
    }

    /// Records that the transport closed.
    ///
    /// This call never rewinds sequence numbers.
    ///
    /// # Errors
    ///
    /// Returns [`FixSessionError`] if `now_ns` regresses.
    pub fn on_transport_disconnected(&mut self, now_ns: u64) -> Result<(), FixSessionError> {
        self.observe_time(now_ns)?;
        if !matches!(self.state, FixSessionState::Disconnected) {
            self.metrics.disconnects = self.metrics.disconnects.saturating_add(1);
        }
        self.state = FixSessionState::Disconnected;
        self.pending_test_request = None;
        self.logout_sent_ns = None;
        self.recovery_target = None;
        self.disconnect_requested = false;
        Ok(())
    }

    /// Stops the session without emitting Logout.
    ///
    /// Use [`FixSessionEngine::request_logout`] for a graceful protocol close.
    pub fn stop(&mut self) {
        self.state = FixSessionState::Stopped;
        self.pending_test_request = None;
        self.logout_sent_ns = None;
        self.recovery_target = None;
        self.disconnect_requested = false;
    }

    /// Encodes a graceful Logout request.
    ///
    /// # Errors
    ///
    /// Returns [`FixSessionError`] unless session flow is active, when the
    /// timestamp regresses, or when text cannot be encoded.
    pub fn request_logout(
        &mut self,
        now_ns: u64,
        sending_time: &[u8],
        text: Option<&[u8]>,
        out: &mut Vec<u8>,
    ) -> Result<FixSessionAction, FixSessionError> {
        if !matches!(
            self.state,
            FixSessionState::Ready
                | FixSessionState::ResendRequested
                | FixSessionState::Recovering
                | FixSessionState::Degraded
        ) {
            return Err(FixSessionError::InvalidState {
                operation: "request_logout",
                state: self.state,
            });
        }
        validate_wire_value(FixTag::SENDING_TIME, sending_time)?;
        if let Some(text) = text {
            validate_wire_value(FixTag::TEXT, text)?;
        }
        self.observe_time(now_ns)?;
        let sequence = self.sequences.assign_outbound();
        encode_logout(
            out,
            self.session_id.version(),
            self.header(sequence, sending_time),
            text,
        )?;
        self.state = FixSessionState::LogoutSent;
        self.logout_sent_ns = Some(now_ns);
        self.metrics.logouts_sent = self.metrics.logouts_sent.saturating_add(1);
        self.mark_outbound(now_ns, false);
        Ok(FixSessionAction::Send {
            kind: FixSessionSendKind::Logout,
            sequence,
        })
    }

    /// Runs deterministic heartbeat, TestRequest, and Logout timers.
    ///
    /// The method reads no clock. The host supplies a monotonic timestamp and a
    /// separately formatted FIX sending time. At most one frame/action is
    /// produced per call.
    ///
    /// # Errors
    ///
    /// Returns [`FixSessionError`] for timestamp regression or encoding failure.
    pub fn on_timer(
        &mut self,
        now_ns: u64,
        sending_time: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<FixSessionAction, FixSessionError> {
        validate_wire_value(FixTag::SENDING_TIME, sending_time)?;
        self.observe_time(now_ns)?;
        if self.disconnect_requested {
            return Ok(FixSessionAction::None);
        }
        if self.state == FixSessionState::LogoutSent {
            if elapsed_at_least(self.logout_sent_ns, now_ns, self.config.logout_timeout_ns) {
                self.state = FixSessionState::Degraded;
                self.disconnect_requested = true;
                self.metrics.timeout_disconnects =
                    self.metrics.timeout_disconnects.saturating_add(1);
                return Ok(FixSessionAction::Disconnect {
                    reason: FixSessionDisconnectReason::LogoutTimeout,
                });
            }
            return Ok(FixSessionAction::None);
        }
        if !matches!(
            self.state,
            FixSessionState::Ready | FixSessionState::ResendRequested | FixSessionState::Recovering
        ) {
            return Ok(FixSessionAction::None);
        }
        if let Some(pending) = self.pending_test_request {
            if now_ns.saturating_sub(pending.sent_ns)
                >= self.config.disconnect_after_test_request_ns
            {
                self.state = FixSessionState::Degraded;
                self.disconnect_requested = true;
                self.metrics.timeout_disconnects =
                    self.metrics.timeout_disconnects.saturating_add(1);
                return Ok(FixSessionAction::Disconnect {
                    reason: FixSessionDisconnectReason::HeartbeatTimeout,
                });
            }
            return Ok(FixSessionAction::None);
        }
        if elapsed_at_least(
            self.metrics.last_inbound_ns,
            now_ns,
            self.config.test_request_after_ns,
        ) {
            return self.encode_liveness_test(now_ns, sending_time, out);
        }
        if elapsed_at_least(
            self.metrics.last_outbound_ns,
            now_ns,
            self.config.heartbeat_interval_ns,
        ) {
            let sequence = self.sequences.assign_outbound();
            encode_heartbeat(
                out,
                self.session_id.version(),
                self.header(sequence, sending_time),
                None,
            )?;
            self.metrics.heartbeats_sent = self.metrics.heartbeats_sent.saturating_add(1);
            self.mark_outbound(now_ns, false);
            return Ok(FixSessionAction::Send {
                kind: FixSessionSendKind::Heartbeat,
                sequence,
            });
        }
        Ok(FixSessionAction::None)
    }

    /// Processes one already-decoded inbound FIX frame.
    ///
    /// The method validates session identity and sequence before dispatching
    /// administrative behavior. It never buffers out-of-order application
    /// messages; on [`FixSessionAction::GapDetected`] the host should hold or
    /// redeliver the triggering frame after the missing range is recovered.
    ///
    /// # Errors
    ///
    /// Returns [`FixSessionError`] for identity, sequence, state, required-tag,
    /// timer, or encoding failures.
    pub fn on_inbound(
        &mut self,
        message: &FixMessageView<'_>,
        now_ns: u64,
        sending_time: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<FixSessionAction, FixSessionError> {
        if matches!(
            self.state,
            FixSessionState::Disconnected | FixSessionState::Connecting | FixSessionState::Stopped
        ) {
            return Err(FixSessionError::InvalidState {
                operation: "inbound",
                state: self.state,
            });
        }
        validate_wire_value(FixTag::SENDING_TIME, sending_time)?;
        self.observe_time(now_ns)?;
        self.validate_identity(message)?;
        self.metrics.inbound_messages = self.metrics.inbound_messages.saturating_add(1);
        self.metrics.last_inbound_ns = Some(now_ns);

        let sequence_action = self.sequences.observe_message(message)?;
        let sequence = match sequence_action {
            FixSequenceAction::Accept { seq_no } => seq_no,
            FixSequenceAction::Duplicate { seq_no, expected } => {
                self.metrics.duplicate_messages = self.metrics.duplicate_messages.saturating_add(1);
                return Ok(FixSessionAction::Duplicate {
                    sequence: seq_no,
                    expected,
                });
            }
            FixSequenceAction::Gap {
                received, resend, ..
            } => {
                self.metrics.sequence_gaps = self.metrics.sequence_gaps.saturating_add(1);
                self.metrics.resend_requests_sent =
                    self.metrics.resend_requests_sent.saturating_add(1);
                self.recovery_target = Some(received);
                self.state = FixSessionState::ResendRequested;
                let request_sequence = self.sequences.assign_outbound();
                encode_resend_request(
                    out,
                    self.session_id.version(),
                    self.header(request_sequence, sending_time),
                    resend,
                )?;
                self.mark_outbound(now_ns, false);
                return Ok(FixSessionAction::GapDetected {
                    received,
                    range: resend,
                    request_sequence,
                });
            }
            FixSequenceAction::TooLow { expected, received } => {
                self.metrics.sequence_too_low = self.metrics.sequence_too_low.saturating_add(1);
                self.state = FixSessionState::Degraded;
                return Err(FixSessionError::SequenceTooLow { expected, received });
            }
        };

        let msg_type = message
            .msg_type()
            .ok_or(FixSessionError::MissingTag(FixTag::MSG_TYPE))?;
        match FixMsgType::from_bytes(msg_type) {
            Some(FixMsgType::LOGON) => self.accept_logon(message, sequence),
            Some(FixMsgType::HEARTBEAT) => self.accept_heartbeat(message, sequence),
            Some(FixMsgType::TEST_REQUEST) => {
                self.respond_to_test_request(message, sequence, now_ns, sending_time, out)
            }
            Some(FixMsgType::RESEND_REQUEST) => self.accept_resend_request(message, sequence),
            Some(FixMsgType::SEQUENCE_RESET) => self.accept_sequence_reset(message, sequence),
            Some(FixMsgType::LOGOUT) => self.accept_logout(
                sequence,
                now_ns,
                sending_time,
                message.get(FixTag::TEXT),
                out,
            ),
            Some(msg_type @ (FixMsgType::REJECT | FixMsgType::BUSINESS_MESSAGE_REJECT)) => {
                Ok(FixSessionAction::Administrative {
                    sequence,
                    message_type: msg_type,
                })
            }
            _ => self.accept_application(sequence),
        }
    }

    /// Assigns the next outbound application sequence number.
    ///
    /// The caller should fully validate its order request before calling this
    /// method, then encode, durably retain, and transmit the resulting frame.
    /// Sequence numbers are never rolled back after assignment; an encode or
    /// send failure must be represented through retry/resend or gap-fill policy.
    ///
    /// # Errors
    ///
    /// Returns [`FixSessionError`] unless the session is ready or the timestamp
    /// regresses.
    pub fn assign_application_sequence(&mut self, now_ns: u64) -> Result<u64, FixSessionError> {
        if self.state != FixSessionState::Ready {
            return Err(FixSessionError::InvalidState {
                operation: "assign_application_sequence",
                state: self.state,
            });
        }
        self.observe_time(now_ns)?;
        let sequence = self.sequences.assign_outbound();
        self.mark_outbound(now_ns, true);
        Ok(sequence)
    }

    /// Records transmission of a possible-duplicate replay frame.
    ///
    /// Replays retain their original FIX sequence and therefore do not advance
    /// the outbound counter.
    ///
    /// # Errors
    ///
    /// Returns [`FixSessionError`] if the timestamp regresses.
    pub fn record_replay_sent(&mut self, now_ns: u64) -> Result<(), FixSessionError> {
        self.observe_time(now_ns)?;
        self.metrics.outbound_messages = self.metrics.outbound_messages.saturating_add(1);
        self.metrics.last_outbound_ns = Some(now_ns);
        Ok(())
    }

    fn accept_logon(
        &mut self,
        message: &FixMessageView<'_>,
        sequence: u64,
    ) -> Result<FixSessionAction, FixSessionError> {
        if self.state != FixSessionState::LogonSent {
            return Err(FixSessionError::InvalidState {
                operation: "accept_logon",
                state: self.state,
            });
        }
        let heartbeat = parse_required_u64(message, FixTag::HEART_BT_INT)?;
        if heartbeat != u64::from(self.config.heartbeat_interval_secs) {
            self.state = FixSessionState::Degraded;
            return Err(FixSessionError::HeartbeatIntervalMismatch {
                expected: self.config.heartbeat_interval_secs,
                received: heartbeat,
            });
        }
        let peer_reset = message.get(FixTag::RESET_SEQ_NUM_FLAG) == Some(b"Y".as_slice());
        if peer_reset != self.config.reset_seq_num_on_logon {
            self.state = FixSessionState::Degraded;
            return Err(FixSessionError::UnexpectedResetSeqNumFlag);
        }
        self.state = FixSessionState::Ready;
        self.metrics.logons_received = self.metrics.logons_received.saturating_add(1);
        Ok(FixSessionAction::Ready { sequence })
    }

    fn accept_heartbeat(
        &mut self,
        message: &FixMessageView<'_>,
        sequence: u64,
    ) -> Result<FixSessionAction, FixSessionError> {
        self.metrics.heartbeats_received = self.metrics.heartbeats_received.saturating_add(1);
        if let Some(pending) = self.pending_test_request {
            if let Some(test_req_id) = message.get(FixTag::TEST_REQ_ID) {
                if test_req_id != pending.id() {
                    self.state = FixSessionState::Degraded;
                    return Err(FixSessionError::TestRequestIdMismatch);
                }
                self.pending_test_request = None;
            }
        }
        self.finish_recovery_if_ready();
        Ok(FixSessionAction::Administrative {
            sequence,
            message_type: FixMsgType::HEARTBEAT,
        })
    }

    fn respond_to_test_request(
        &mut self,
        message: &FixMessageView<'_>,
        _inbound_sequence: u64,
        now_ns: u64,
        sending_time: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<FixSessionAction, FixSessionError> {
        let test_req_id = message
            .get(FixTag::TEST_REQ_ID)
            .ok_or(FixSessionError::MissingTag(FixTag::TEST_REQ_ID))?;
        self.metrics.test_requests_received = self.metrics.test_requests_received.saturating_add(1);
        let sequence = self.sequences.assign_outbound();
        encode_heartbeat(
            out,
            self.session_id.version(),
            self.header(sequence, sending_time),
            Some(test_req_id),
        )?;
        self.metrics.heartbeats_sent = self.metrics.heartbeats_sent.saturating_add(1);
        self.mark_outbound(now_ns, false);
        self.finish_recovery_if_ready();
        Ok(FixSessionAction::Send {
            kind: FixSessionSendKind::Heartbeat,
            sequence,
        })
    }

    fn accept_resend_request(
        &mut self,
        message: &FixMessageView<'_>,
        sequence: u64,
    ) -> Result<FixSessionAction, FixSessionError> {
        let begin_seq_no = parse_required_u64(message, FixTag::BEGIN_SEQ_NO)?;
        let end_seq_no = parse_required_u64(message, FixTag::END_SEQ_NO)?;
        if begin_seq_no == 0 || (end_seq_no != 0 && end_seq_no < begin_seq_no) {
            return Err(FixSessionError::InvalidResendRange {
                begin_seq_no,
                end_seq_no,
            });
        }
        self.metrics.resend_requests_received =
            self.metrics.resend_requests_received.saturating_add(1);
        self.finish_recovery_if_ready();
        Ok(FixSessionAction::PeerResendRequested {
            sequence,
            range: FixResendRange {
                begin_seq_no,
                end_seq_no,
            },
        })
    }

    fn accept_sequence_reset(
        &mut self,
        message: &FixMessageView<'_>,
        sequence: u64,
    ) -> Result<FixSessionAction, FixSessionError> {
        let next_inbound = parse_required_u64(message, FixTag::NEW_SEQ_NO)?;
        self.sequences.apply_sequence_reset(next_inbound)?;
        self.finish_recovery_if_ready();
        Ok(FixSessionAction::SequenceReset {
            sequence,
            next_inbound,
        })
    }

    fn accept_logout(
        &mut self,
        _inbound_sequence: u64,
        now_ns: u64,
        sending_time: &[u8],
        text: Option<&[u8]>,
        out: &mut Vec<u8>,
    ) -> Result<FixSessionAction, FixSessionError> {
        self.metrics.logouts_received = self.metrics.logouts_received.saturating_add(1);
        if self.state == FixSessionState::LogoutSent {
            self.state = FixSessionState::Stopped;
            self.logout_sent_ns = None;
            self.disconnect_requested = true;
            return Ok(FixSessionAction::Disconnect {
                reason: FixSessionDisconnectReason::PeerLogout,
            });
        }
        let sequence = self.sequences.assign_outbound();
        encode_logout(
            out,
            self.session_id.version(),
            self.header(sequence, sending_time),
            text,
        )?;
        self.state = FixSessionState::LogoutSent;
        self.logout_sent_ns = Some(now_ns);
        self.metrics.logouts_sent = self.metrics.logouts_sent.saturating_add(1);
        self.mark_outbound(now_ns, false);
        Ok(FixSessionAction::Send {
            kind: FixSessionSendKind::Logout,
            sequence,
        })
    }

    fn accept_application(&mut self, sequence: u64) -> Result<FixSessionAction, FixSessionError> {
        if !matches!(
            self.state,
            FixSessionState::Ready | FixSessionState::ResendRequested | FixSessionState::Recovering
        ) {
            return Err(FixSessionError::UnexpectedApplicationMessage);
        }
        self.metrics.inbound_application_messages =
            self.metrics.inbound_application_messages.saturating_add(1);
        self.finish_recovery_if_ready();
        Ok(FixSessionAction::Application { sequence })
    }

    fn encode_liveness_test(
        &mut self,
        now_ns: u64,
        sending_time: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<FixSessionAction, FixSessionError> {
        let mut bytes = [0u8; TEST_REQUEST_ID_CAPACITY];
        bytes[..TEST_REQUEST_PREFIX.len()].copy_from_slice(TEST_REQUEST_PREFIX);
        let digits = write_decimal(
            &mut bytes[TEST_REQUEST_PREFIX.len()..],
            self.next_test_request_id,
        );
        let len = TEST_REQUEST_PREFIX.len().saturating_add(digits);
        let sequence = self.sequences.assign_outbound();
        encode_test_request(
            out,
            self.session_id.version(),
            self.header(sequence, sending_time),
            &bytes[..len],
        )?;
        self.pending_test_request = Some(PendingTestRequest {
            bytes,
            len: u8::try_from(len).unwrap_or(u8::MAX),
            sent_ns: now_ns,
        });
        self.next_test_request_id = self.next_test_request_id.saturating_add(1);
        self.metrics.test_requests_sent = self.metrics.test_requests_sent.saturating_add(1);
        self.mark_outbound(now_ns, false);
        Ok(FixSessionAction::Send {
            kind: FixSessionSendKind::TestRequest,
            sequence,
        })
    }

    fn validate_identity(&self, message: &FixMessageView<'_>) -> Result<(), FixSessionError> {
        if message.begin_string() != Some(self.session_id.version().as_bytes()) {
            return Err(FixSessionError::VersionMismatch);
        }
        if !self.config.validate_comp_ids {
            return Ok(());
        }
        if message.get(FixTag::SENDER_COMP_ID) != Some(self.session_id.target_comp_id()) {
            return Err(FixSessionError::SenderCompIdMismatch);
        }
        if message.get(FixTag::TARGET_COMP_ID) != Some(self.session_id.sender_comp_id()) {
            return Err(FixSessionError::TargetCompIdMismatch);
        }
        Ok(())
    }

    fn header<'a>(&'a self, sequence: u64, sending_time: &'a [u8]) -> FixSessionHeader<'a> {
        FixSessionHeader::new(
            self.session_id.sender_comp_id(),
            self.session_id.target_comp_id(),
            sequence,
            sending_time,
        )
    }

    fn observe_time(&mut self, now_ns: u64) -> Result<(), FixSessionError> {
        if let Some(previous_ns) = self.last_observed_ns {
            if now_ns < previous_ns {
                return Err(FixSessionError::TimestampRegression {
                    previous_ns,
                    observed_ns: now_ns,
                });
            }
        }
        self.last_observed_ns = Some(now_ns);
        Ok(())
    }

    fn mark_outbound(&mut self, now_ns: u64, application: bool) {
        self.metrics.outbound_messages = self.metrics.outbound_messages.saturating_add(1);
        if application {
            self.metrics.outbound_application_messages =
                self.metrics.outbound_application_messages.saturating_add(1);
        }
        self.metrics.last_outbound_ns = Some(now_ns);
    }

    fn finish_recovery_if_ready(&mut self) {
        if let Some(target) = self.recovery_target {
            if self.sequences.next_inbound() >= target {
                self.recovery_target = None;
                if matches!(
                    self.state,
                    FixSessionState::ResendRequested | FixSessionState::Recovering
                ) {
                    self.state = FixSessionState::Ready;
                }
            } else if self.state == FixSessionState::ResendRequested {
                self.state = FixSessionState::Recovering;
            }
        }
    }
}

fn elapsed_at_least(start_ns: Option<u64>, now_ns: u64, duration_ns: u64) -> bool {
    start_ns.is_some_and(|start| now_ns.saturating_sub(start) >= duration_ns)
}

fn parse_required_u64(message: &FixMessageView<'_>, tag: FixTag) -> Result<u64, FixSessionError> {
    let value = message.get(tag).ok_or(FixSessionError::MissingTag(tag))?;
    parse_decimal(value).ok_or(FixSessionError::MalformedNumericTag(tag))
}

fn validate_wire_value(tag: FixTag, value: &[u8]) -> Result<(), FixSessionError> {
    if value.contains(&crate::SOH) {
        return Err(FixEncodeError::ValueContainsSoh(tag).into());
    }
    Ok(())
}

fn parse_decimal(value: &[u8]) -> Option<u64> {
    if value.is_empty() {
        return None;
    }
    let mut parsed = 0u64;
    for byte in value {
        if !byte.is_ascii_digit() {
            return None;
        }
        parsed = parsed
            .checked_mul(10)?
            .checked_add(u64::from(byte - b'0'))?;
    }
    Some(parsed)
}

fn write_decimal(out: &mut [u8], mut value: u64) -> usize {
    let mut reversed = [0u8; 20];
    let mut len = 0usize;
    loop {
        reversed[len] = b'0' + (value % 10) as u8;
        len += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for index in 0..len {
        out[index] = reversed[len - index - 1];
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encode_message, parse_message, FixFieldView, FixVersion};

    const SEND_TIME: &[u8] = b"20260810-12:00:00.000";
    const HEARTBEAT_DUE_NS: u64 = 10_000_000_002;
    const TEST_REQUEST_DUE_NS: u64 = 15_000_000_002;
    const DISCONNECT_DUE_NS: u64 = 25_000_000_002;

    fn engine() -> FixSessionEngine {
        FixSessionEngine::new(
            FixSessionEngineConfig::new(10)
                .unwrap()
                .with_timeouts(15_000_000_000, 10_000_000_000, 10_000_000_000)
                .unwrap(),
            FixOwnedSessionId::new(FixVersion::Fix44, b"CLIENT".to_vec(), b"VENUE".to_vec())
                .unwrap(),
        )
    }

    fn inbound(
        msg_type: FixMsgType,
        sequence: u64,
        extra: &[(FixTag, &[u8])],
    ) -> (Vec<u8>, Vec<FixFieldView<'static>>) {
        let mut sequence_bytes = [0u8; 20];
        let sequence_len = write_decimal(&mut sequence_bytes, sequence);
        let mut fields = vec![
            (FixTag::SENDER_COMP_ID, b"VENUE".as_slice()),
            (FixTag::TARGET_COMP_ID, b"CLIENT".as_slice()),
            (FixTag::MSG_SEQ_NUM, &sequence_bytes[..sequence_len]),
            (FixTag::SENDING_TIME, SEND_TIME),
        ];
        fields.extend_from_slice(extra);
        let mut raw = Vec::new();
        encode_message(&mut raw, b"FIX.4.4", msg_type.as_bytes(), &fields).unwrap();
        let scratch = vec![FixFieldView::empty(); 64];
        (raw, scratch)
    }

    fn with_message<T>(
        msg_type: FixMsgType,
        sequence: u64,
        extra: &[(FixTag, &[u8])],
        apply: impl FnOnce(&FixMessageView<'_>) -> T,
    ) -> T {
        let (raw, mut scratch) = inbound(msg_type, sequence, extra);
        let message = parse_message(&raw, &mut scratch).unwrap();
        apply(&message)
    }

    fn connect_and_logon(engine: &mut FixSessionEngine, out: &mut Vec<u8>) {
        engine.on_transport_connecting().unwrap();
        let action = engine.on_transport_connected(1, SEND_TIME, out).unwrap();
        assert_eq!(
            action,
            FixSessionAction::Send {
                kind: FixSessionSendKind::Logon,
                sequence: 1
            }
        );
        with_message(
            FixMsgType::LOGON,
            1,
            &[(FixTag::HEART_BT_INT, b"10")],
            |message| {
                assert_eq!(
                    engine.on_inbound(message, 2, SEND_TIME, out).unwrap(),
                    FixSessionAction::Ready { sequence: 1 }
                );
            },
        );
    }

    #[test]
    fn connect_and_logon_establish_ready_session() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);
        assert_eq!(engine.state(), FixSessionState::Ready);
        assert_eq!(engine.sequences().next_inbound(), 2);
        assert_eq!(engine.sequences().next_outbound(), 2);
        assert_eq!(engine.metrics().connections, 1);
        assert_eq!(engine.metrics().logons_received, 1);
    }

    #[test]
    fn timer_sends_heartbeat_test_request_and_disconnects() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);

        assert_eq!(
            engine
                .on_timer(HEARTBEAT_DUE_NS, SEND_TIME, &mut out)
                .unwrap(),
            FixSessionAction::Send {
                kind: FixSessionSendKind::Heartbeat,
                sequence: 2
            }
        );
        assert_eq!(
            engine
                .on_timer(TEST_REQUEST_DUE_NS, SEND_TIME, &mut out)
                .unwrap(),
            FixSessionAction::Send {
                kind: FixSessionSendKind::TestRequest,
                sequence: 3
            }
        );
        assert_eq!(
            engine
                .on_timer(DISCONNECT_DUE_NS, SEND_TIME, &mut out)
                .unwrap(),
            FixSessionAction::Disconnect {
                reason: FixSessionDisconnectReason::HeartbeatTimeout
            }
        );
        assert_eq!(engine.state(), FixSessionState::Degraded);
        assert_eq!(engine.metrics().timeout_disconnects, 1);
    }

    #[test]
    fn heartbeat_must_echo_pending_test_request_id() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);
        engine
            .on_timer(TEST_REQUEST_DUE_NS, SEND_TIME, &mut out)
            .unwrap();

        with_message(
            FixMsgType::HEARTBEAT,
            2,
            &[(FixTag::TEST_REQ_ID, b"wrong")],
            |message| {
                assert_eq!(
                    engine.on_inbound(message, TEST_REQUEST_DUE_NS + 1, SEND_TIME, &mut out),
                    Err(FixSessionError::TestRequestIdMismatch)
                );
            },
        );
        assert_eq!(engine.state(), FixSessionState::Degraded);
    }

    #[test]
    fn peer_test_request_encodes_correlated_heartbeat() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);
        with_message(
            FixMsgType::TEST_REQUEST,
            2,
            &[(FixTag::TEST_REQ_ID, b"peer-1")],
            |message| {
                assert_eq!(
                    engine.on_inbound(message, 3, SEND_TIME, &mut out).unwrap(),
                    FixSessionAction::Send {
                        kind: FixSessionSendKind::Heartbeat,
                        sequence: 2
                    }
                );
            },
        );
        let mut scratch = [FixFieldView::empty(); 32];
        let response = parse_message(&out, &mut scratch).unwrap();
        assert_eq!(response.msg_type(), Some(b"0".as_slice()));
        assert_eq!(
            response.get(FixTag::TEST_REQ_ID),
            Some(b"peer-1".as_slice())
        );
    }

    #[test]
    fn gap_recovery_requests_missing_range_and_applies_reset() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);
        with_message(FixMsgType::EXECUTION_REPORT, 4, &[], |message| {
            assert_eq!(
                engine.on_inbound(message, 3, SEND_TIME, &mut out).unwrap(),
                FixSessionAction::GapDetected {
                    received: 4,
                    range: FixResendRange {
                        begin_seq_no: 2,
                        end_seq_no: 3
                    },
                    request_sequence: 2
                }
            );
        });
        assert_eq!(engine.state(), FixSessionState::ResendRequested);

        with_message(
            FixMsgType::SEQUENCE_RESET,
            2,
            &[(FixTag::NEW_SEQ_NO, b"4"), (FixTag::GAP_FILL_FLAG, b"Y")],
            |message| {
                assert_eq!(
                    engine.on_inbound(message, 4, SEND_TIME, &mut out).unwrap(),
                    FixSessionAction::SequenceReset {
                        sequence: 2,
                        next_inbound: 4
                    }
                );
            },
        );
        assert_eq!(engine.state(), FixSessionState::Ready);
        with_message(FixMsgType::EXECUTION_REPORT, 4, &[], |message| {
            assert_eq!(
                engine.on_inbound(message, 5, SEND_TIME, &mut out).unwrap(),
                FixSessionAction::Application { sequence: 4 }
            );
        });
    }

    #[test]
    fn gap_recovery_accepts_replayed_application_messages_in_order() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);
        with_message(FixMsgType::EXECUTION_REPORT, 3, &[], |message| {
            assert!(matches!(
                engine.on_inbound(message, 3, SEND_TIME, &mut out).unwrap(),
                FixSessionAction::GapDetected { received: 3, .. }
            ));
        });
        assert_eq!(engine.state(), FixSessionState::ResendRequested);

        with_message(FixMsgType::EXECUTION_REPORT, 2, &[], |message| {
            assert_eq!(
                engine.on_inbound(message, 4, SEND_TIME, &mut out).unwrap(),
                FixSessionAction::Application { sequence: 2 }
            );
        });
        assert_eq!(engine.state(), FixSessionState::Ready);
    }

    #[test]
    fn peer_resend_request_is_exposed_without_store_coupling() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);
        with_message(
            FixMsgType::RESEND_REQUEST,
            2,
            &[(FixTag::BEGIN_SEQ_NO, b"1"), (FixTag::END_SEQ_NO, b"0")],
            |message| {
                assert_eq!(
                    engine.on_inbound(message, 3, SEND_TIME, &mut out).unwrap(),
                    FixSessionAction::PeerResendRequested {
                        sequence: 2,
                        range: FixResendRange {
                            begin_seq_no: 1,
                            end_seq_no: 0
                        }
                    }
                );
            },
        );
    }

    #[test]
    fn duplicate_is_ignored_and_unflagged_regression_fails_closed() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);
        with_message(
            FixMsgType::HEARTBEAT,
            1,
            &[(FixTag::POSS_DUP_FLAG, b"Y")],
            |message| {
                assert_eq!(
                    engine.on_inbound(message, 3, SEND_TIME, &mut out).unwrap(),
                    FixSessionAction::Duplicate {
                        sequence: 1,
                        expected: 2
                    }
                );
            },
        );
        with_message(FixMsgType::HEARTBEAT, 1, &[], |message| {
            assert_eq!(
                engine.on_inbound(message, 4, SEND_TIME, &mut out),
                Err(FixSessionError::SequenceTooLow {
                    expected: 2,
                    received: 1
                })
            );
        });
    }

    #[test]
    fn graceful_logout_waits_for_peer_then_requests_disconnect() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);
        assert_eq!(
            engine
                .request_logout(3, SEND_TIME, Some(b"shutdown"), &mut out)
                .unwrap(),
            FixSessionAction::Send {
                kind: FixSessionSendKind::Logout,
                sequence: 2
            }
        );
        with_message(FixMsgType::LOGOUT, 2, &[], |message| {
            assert_eq!(
                engine.on_inbound(message, 4, SEND_TIME, &mut out).unwrap(),
                FixSessionAction::Disconnect {
                    reason: FixSessionDisconnectReason::PeerLogout
                }
            );
        });
        assert_eq!(engine.state(), FixSessionState::Stopped);
    }

    #[test]
    fn identity_and_timestamp_regressions_fail_closed() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        engine
            .on_transport_connected(10, SEND_TIME, &mut out)
            .unwrap();
        assert_eq!(
            engine.on_timer(9, SEND_TIME, &mut out),
            Err(FixSessionError::TimestampRegression {
                previous_ns: 10,
                observed_ns: 9
            })
        );

        let mut sequence_bytes = [0u8; 20];
        let len = write_decimal(&mut sequence_bytes, 1);
        let mut raw = Vec::new();
        encode_message(
            &mut raw,
            b"FIX.4.4",
            b"A",
            &[
                (FixTag::SENDER_COMP_ID, b"WRONG"),
                (FixTag::TARGET_COMP_ID, b"CLIENT"),
                (FixTag::MSG_SEQ_NUM, &sequence_bytes[..len]),
                (FixTag::SENDING_TIME, SEND_TIME),
                (FixTag::HEART_BT_INT, b"10"),
            ],
        )
        .unwrap();
        let mut scratch = [FixFieldView::empty(); 32];
        let message = parse_message(&raw, &mut scratch).unwrap();
        assert_eq!(
            engine.on_inbound(&message, 11, SEND_TIME, &mut out),
            Err(FixSessionError::SenderCompIdMismatch)
        );
    }

    #[test]
    fn application_sequence_assignment_requires_ready_state() {
        let mut engine = engine();
        assert!(matches!(
            engine.assign_application_sequence(1),
            Err(FixSessionError::InvalidState { .. })
        ));
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);
        assert_eq!(engine.assign_application_sequence(3).unwrap(), 2);
        assert_eq!(engine.metrics().outbound_application_messages, 1);
    }

    #[test]
    fn custom_profile_message_is_accepted_as_application_flow() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        connect_and_logon(&mut engine, &mut out);
        with_message(FixMsgType::from_static(b"U99"), 2, &[], |message| {
            assert_eq!(
                engine.on_inbound(message, 3, SEND_TIME, &mut out).unwrap(),
                FixSessionAction::Application { sequence: 2 }
            );
        });
    }

    #[test]
    fn invalid_sending_time_does_not_consume_outbound_sequence() {
        let mut engine = engine();
        let mut out = Vec::with_capacity(512);
        engine.on_transport_connecting().unwrap();
        assert_eq!(
            engine.on_transport_connected(1, b"bad\x01time", &mut out),
            Err(FixSessionError::Encode(FixEncodeError::ValueContainsSoh(
                FixTag::SENDING_TIME
            )))
        );
        assert_eq!(engine.sequences().next_outbound(), 1);
        assert_eq!(engine.state(), FixSessionState::Connecting);
    }
}
