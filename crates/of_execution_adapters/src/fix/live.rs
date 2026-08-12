//! Transport-injected FIX execution adapter.

use super::{
    encode_amend_request, encode_cancel_request, encode_order_request, encode_stop_amend_request,
    map_execution_report, map_order_cancel_reject, parse_execution_report,
    parse_order_cancel_reject, FixAmendEncodeContext, FixCancelEncodeContext, FixReportParseConfig,
    FixReportParseError, FixRequestEncodeConfig, FixRequestEncodeError, FixSessionConfig,
    FixStopAmendEncodeContext,
};
use of_execution::{
    ExecutionAdapter, ExecutionCapabilities, ExecutionError, ExecutionEventBuffer, ExecutionHealth,
    ExecutionResult, LatencyClass,
};
use of_execution_core::{
    AmendRequest, CancelRequest, ClientOrderId, ExecutionEvent, ExecutionType, OrderPrice,
    OrderRequest, OrderSide, OrderStatus, OrderType, TimeInForce,
};
use of_fix::{
    encode_order_mass_status_request, encode_poss_dup_replay, encode_sequence_reset_gap_fill,
    parse_business_message_reject, parse_message, parse_session_reject, FixEncodeError,
    FixFieldView, FixMassStatusReqType, FixMessageView, FixMsgType, FixOrderMassStatusRequest,
    FixOwnedSequenceSnapshot, FixOwnedSessionId, FixResendRange, FixResendStore,
    FixResendStoreConfig, FixResendStoreError, FixSentMessageKind, FixSequenceTracker,
    FixSessionAction, FixSessionEngine, FixSessionEngineConfig, FixSessionError, FixSessionHeader,
    FixSessionMetrics, FixSessionState, FixTag, FixVersion,
};
use std::error::Error;
use std::fmt;

const DEFAULT_MAX_FRAME_BYTES: usize = 64 * 1024;
const DEFAULT_MAX_FRAMES_PER_POLL: usize = 64;
const DEFAULT_MAX_PENDING_GAP_FRAMES: usize = 128;
const DEFAULT_MAX_WORKING_ORDERS: usize = 16_384;
const DEFAULT_MAX_RESEND_SEQUENCES: usize = 8_192;
const DEFAULT_MAX_RESEND_ACTIONS: usize = 2_048;
const FIX_FIELD_SCRATCH_CAPACITY: usize = 192;
const FIX_TIMESTAMP_CAPACITY: usize = 32;

/// Result of one non-blocking transport receive attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixTransportPoll {
    /// No complete frame is currently available.
    Idle,
    /// One complete FIX frame was written to the supplied receive buffer.
    Frame {
        /// Number of initialized bytes in the receive buffer.
        len: usize,
    },
    /// The peer or underlying transport closed the connection.
    Disconnected,
}

/// Frame-oriented transport contract for a live FIX session.
///
/// Implementations own TCP, TLS, WebSocket, leased-line, or test transports and
/// must preserve complete FIX frame boundaries. `poll_receive` is non-blocking;
/// it writes into adapter-owned memory so the common receive path does not need
/// to allocate or copy into an intermediate protocol object.
pub trait FixFrameTransport: Send {
    /// Transport-specific error.
    type Error: Error + Send + Sync + 'static;

    /// Establishes the physical transport.
    fn connect(&mut self) -> Result<(), Self::Error>;

    /// Sends one complete FIX frame.
    fn send(&mut self, frame: &[u8]) -> Result<(), Self::Error>;

    /// Attempts to receive one complete FIX frame without blocking.
    fn poll_receive(&mut self, out: &mut [u8]) -> Result<FixTransportPoll, Self::Error>;

    /// Closes the physical transport.
    fn disconnect(&mut self) -> Result<(), Self::Error>;
}

/// One monotonic/wall-clock sample for FIX protocol work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixTimeSample {
    monotonic_ns: u64,
    unix_ns: u64,
    sending_time_len: usize,
}

impl FixTimeSample {
    /// Creates a timestamp sample.
    ///
    /// `sending_time_len` is the initialized prefix written by
    /// [`FixTimeSource::sample`] in the venue's accepted UTC timestamp format.
    pub const fn new(monotonic_ns: u64, unix_ns: u64, sending_time_len: usize) -> Self {
        Self {
            monotonic_ns,
            unix_ns,
            sending_time_len,
        }
    }

    /// Returns the monotonic timestamp used for liveness and latency tracking.
    pub const fn monotonic_ns(self) -> u64 {
        self.monotonic_ns
    }

    /// Returns UTC nanoseconds since the Unix epoch for canonical receive
    /// timestamping and exchange-to-adapter latency.
    pub const fn unix_ns(self) -> u64 {
        self.unix_ns
    }

    /// Returns initialized FIX `SendingTime(52)` bytes.
    pub const fn sending_time_len(self) -> usize {
        self.sending_time_len
    }
}

/// Injected monotonic clock and FIX UTC timestamp formatter.
///
/// Combining both values in one sample keeps their relationship explicit and
/// lets low-latency hosts use a cached/vDSO clock or a dedicated time service.
pub trait FixTimeSource: Send {
    /// Clock/formatting error.
    type Error: Error + Send + Sync + 'static;

    /// Writes `SendingTime(52)` into `out` and returns its initialized length
    /// together with a monotonic nanosecond timestamp.
    fn sample(&mut self, out: &mut [u8]) -> Result<FixTimeSample, Self::Error>;
}

/// Durable original-message journal used before transport transmission.
///
/// Implementations retain exact original frames by sequence number. Replay
/// frames are deliberately excluded. A journal may enqueue writes to a
/// persistence worker, but success must mean the frame met that implementation's
/// configured durability contract.
pub trait FixOutboundJournal: Send {
    /// Journal-specific error.
    type Error: Error + Send + Sync + 'static;

    /// Records one newly sequenced original frame.
    fn record_sent(
        &mut self,
        sequence: u64,
        kind: FixSentMessageKind,
        frame: &[u8],
    ) -> Result<(), Self::Error>;
}

/// Outbound journal that performs no durable I/O.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoopFixOutboundJournal;

impl FixOutboundJournal for NoopFixOutboundJournal {
    type Error = InfallibleFixJournalError;

    fn record_sent(
        &mut self,
        _sequence: u64,
        _kind: FixSentMessageKind,
        _frame: &[u8],
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Uninhabited error used by [`NoopFixOutboundJournal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfallibleFixJournalError {}

impl fmt::Display for InfallibleFixJournalError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl Error for InfallibleFixJournalError {}

/// Adapts an `of_fix` durable resend store to the live-adapter journal hook.
#[derive(Debug)]
pub struct DurableFixOutboundJournal<S> {
    store: S,
}

impl<S> DurableFixOutboundJournal<S> {
    /// Wraps a durable resend-message store.
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Returns immutable store access.
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Returns mutable store access.
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// Consumes the wrapper and returns the underlying store.
    pub fn into_inner(self) -> S {
        self.store
    }
}

impl<S> FixOutboundJournal for DurableFixOutboundJournal<S>
where
    S: of_fix::FixDurableResendMessageStore + Send,
{
    type Error = of_fix::FixDurableResendStoreError;

    fn record_sent(
        &mut self,
        sequence: u64,
        kind: FixSentMessageKind,
        frame: &[u8],
    ) -> Result<(), Self::Error> {
        self.store.record_sent(sequence, kind, frame).map(|_| ())
    }
}

/// Original-order context needed by FIX cancel and replace messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixWorkingOrderContext {
    side: OrderSide,
    order_type: OrderType,
    time_in_force: TimeInForce,
    stop_price: OrderPrice,
}

impl FixWorkingOrderContext {
    /// Creates original-order context.
    pub const fn new(
        side: OrderSide,
        order_type: OrderType,
        time_in_force: TimeInForce,
        stop_price: OrderPrice,
    ) -> Self {
        Self {
            side,
            order_type,
            time_in_force,
            stop_price,
        }
    }

    /// Returns order side.
    pub const fn side(self) -> OrderSide {
        self.side
    }

    /// Returns original order type.
    pub const fn order_type(self) -> OrderType {
        self.order_type
    }

    /// Returns original time in force.
    pub const fn time_in_force(self) -> TimeInForce {
        self.time_in_force
    }

    /// Returns original stop price.
    pub const fn stop_price(self) -> OrderPrice {
        self.stop_price
    }
}

/// Venue/profile policy used by the transport adapter.
///
/// The adapter owns session reliability and bounded queues. Profiles own
/// venue-specific application mapping, capabilities, custom tags, and recovery
/// request shape. Static dispatch keeps this extension point off a virtual-call
/// path for users that instantiate the generic adapter directly.
pub trait FixExecutionProfile: Send {
    /// Profile-specific error.
    type Error: Error + Send + Sync + 'static;

    /// Returns the FIX begin-string version for this profile.
    fn version(&self) -> FixVersion;

    /// Returns truthful order-entry capabilities.
    fn capabilities(&self) -> ExecutionCapabilities;

    /// Encodes a new order into `out`.
    fn encode_submit(
        &mut self,
        out: &mut Vec<u8>,
        header: FixSessionHeader<'_>,
        request: &OrderRequest,
        transact_time: &[u8],
    ) -> Result<(), Self::Error>;

    /// Encodes an order cancel into `out`.
    fn encode_cancel(
        &mut self,
        out: &mut Vec<u8>,
        header: FixSessionHeader<'_>,
        request: &CancelRequest,
        original: FixWorkingOrderContext,
        transact_time: &[u8],
    ) -> Result<(), Self::Error>;

    /// Encodes an order cancel/replace into `out`.
    fn encode_amend(
        &mut self,
        out: &mut Vec<u8>,
        header: FixSessionHeader<'_>,
        request: &AmendRequest,
        original: FixWorkingOrderContext,
        transact_time: &[u8],
    ) -> Result<(), Self::Error>;

    /// Maps one session-validated application message.
    ///
    /// Returning `Ok(None)` accepts a profile-specific message that does not
    /// produce an OMS event.
    fn map_application(
        &mut self,
        message: &FixMessageView<'_>,
        ts_recv_ns: u64,
    ) -> Result<Option<ExecutionEvent>, Self::Error>;

    /// Encodes an open-order recovery request.
    ///
    /// Returning `Ok(false)` means this profile requires host-specific recovery
    /// rather than the standard mass-status workflow.
    fn encode_open_order_recovery(
        &mut self,
        _out: &mut Vec<u8>,
        _header: FixSessionHeader<'_>,
        _request_id: &[u8],
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }
}

/// Error from the standard FIX execution profile.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StandardFixProfileError {
    /// Outbound canonical request could not be encoded.
    Request(FixRequestEncodeError),
    /// Inbound execution message could not be mapped.
    Report(FixReportParseError),
    /// A standard recovery request could not be encoded.
    Encode(FixEncodeError),
}

impl fmt::Display for StandardFixProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(error) => write!(f, "FIX profile request error: {error}"),
            Self::Report(error) => write!(f, "FIX profile report error: {error}"),
            Self::Encode(error) => write!(f, "FIX profile encode error: {error}"),
        }
    }
}

impl Error for StandardFixProfileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
            Self::Report(error) => Some(error),
            Self::Encode(error) => Some(error),
        }
    }
}

impl From<FixRequestEncodeError> for StandardFixProfileError {
    fn from(value: FixRequestEncodeError) -> Self {
        Self::Request(value)
    }
}

impl From<FixReportParseError> for StandardFixProfileError {
    fn from(value: FixReportParseError) -> Self {
        Self::Report(value)
    }
}

impl From<FixEncodeError> for StandardFixProfileError {
    fn from(value: FixEncodeError) -> Self {
        Self::Encode(value)
    }
}

/// Standard FIX 4.2/4.4 application profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardFixExecutionProfile {
    version: FixVersion,
    request_config: FixRequestEncodeConfig,
    report_config: FixReportParseConfig,
    capabilities: ExecutionCapabilities,
}

impl StandardFixExecutionProfile {
    /// Creates a standard application profile.
    pub const fn new(
        version: FixVersion,
        request_config: FixRequestEncodeConfig,
        report_config: FixReportParseConfig,
    ) -> Self {
        Self {
            version,
            request_config,
            report_config,
            capabilities: ExecutionCapabilities {
                latency_class: LatencyClass::NativeFix,
                market: true,
                limit: true,
                stop: true,
                stop_limit: true,
                tif_day: true,
                tif_gtc: true,
                tif_ioc: true,
                tif_fok: true,
                tif_gtd: true,
                amend: true,
                native_client_order_id: true,
            },
        }
    }

    /// Overrides profile capabilities after counterparty certification.
    pub const fn with_capabilities(mut self, capabilities: ExecutionCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }
}

impl FixExecutionProfile for StandardFixExecutionProfile {
    type Error = StandardFixProfileError;

    fn version(&self) -> FixVersion {
        self.version
    }

    fn capabilities(&self) -> ExecutionCapabilities {
        self.capabilities
    }

    fn encode_submit(
        &mut self,
        out: &mut Vec<u8>,
        header: FixSessionHeader<'_>,
        request: &OrderRequest,
        transact_time: &[u8],
    ) -> Result<(), Self::Error> {
        encode_order_request(
            out,
            self.version,
            header,
            self.request_config,
            request,
            transact_time,
        )?;
        Ok(())
    }

    fn encode_cancel(
        &mut self,
        out: &mut Vec<u8>,
        header: FixSessionHeader<'_>,
        request: &CancelRequest,
        original: FixWorkingOrderContext,
        transact_time: &[u8],
    ) -> Result<(), Self::Error> {
        encode_cancel_request(
            out,
            self.version,
            header,
            request,
            FixCancelEncodeContext::new(original.side(), transact_time),
        )?;
        Ok(())
    }

    fn encode_amend(
        &mut self,
        out: &mut Vec<u8>,
        header: FixSessionHeader<'_>,
        request: &AmendRequest,
        original: FixWorkingOrderContext,
        transact_time: &[u8],
    ) -> Result<(), Self::Error> {
        if matches!(
            original.order_type(),
            OrderType::Stop | OrderType::StopLimit
        ) {
            encode_stop_amend_request(
                out,
                self.version,
                header,
                self.request_config,
                request,
                FixStopAmendEncodeContext::new(
                    original.side(),
                    original.order_type(),
                    original.time_in_force(),
                    original.stop_price(),
                    transact_time,
                ),
            )?;
        } else {
            encode_amend_request(
                out,
                self.version,
                header,
                self.request_config,
                request,
                FixAmendEncodeContext::new(
                    original.side(),
                    original.order_type(),
                    original.time_in_force(),
                    transact_time,
                ),
            )?;
        }
        Ok(())
    }

    fn map_application(
        &mut self,
        message: &FixMessageView<'_>,
        ts_recv_ns: u64,
    ) -> Result<Option<ExecutionEvent>, Self::Error> {
        match message.typed_msg_type() {
            Some(FixMsgType::EXECUTION_REPORT) => Ok(Some(map_execution_report(
                &parse_execution_report(message, self.report_config, ts_recv_ns)?,
            ))),
            Some(FixMsgType::ORDER_CANCEL_REJECT) => Ok(Some(map_order_cancel_reject(
                &parse_order_cancel_reject(message, self.report_config, ts_recv_ns)?,
            ))),
            _ => Ok(None),
        }
    }

    fn encode_open_order_recovery(
        &mut self,
        out: &mut Vec<u8>,
        header: FixSessionHeader<'_>,
        request_id: &[u8],
    ) -> Result<bool, Self::Error> {
        if self.version < FixVersion::Fix43 {
            return Ok(false);
        }
        encode_order_mass_status_request(
            out,
            self.version,
            header,
            FixOrderMassStatusRequest::new(request_id, FixMassStatusReqType::AllOrders)
                .with_account(self.report_config.account_id.as_str().as_bytes()),
        )?;
        Ok(true)
    }
}

/// Invalid live FIX adapter configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixLiveAdapterConfigError {
    /// Begin string is not a known FIX version.
    UnsupportedVersion,
    /// Profile and session begin-string versions differ.
    ProfileVersionMismatch,
    /// Heartbeat interval is invalid.
    SessionConfig,
    /// A bounded capacity is zero.
    ZeroCapacity(&'static str),
    /// Session identity contains an invalid FIX value.
    SessionIdentity,
}

impl fmt::Display for FixLiveAdapterConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion => f.write_str("unsupported FIX begin string"),
            Self::ProfileVersionMismatch => {
                f.write_str("FIX profile version differs from session begin string")
            }
            Self::SessionConfig => f.write_str("invalid FIX session configuration"),
            Self::ZeroCapacity(name) => write!(f, "FIX adapter capacity {name} must be non-zero"),
            Self::SessionIdentity => f.write_str("invalid FIX session identity"),
        }
    }
}

impl Error for FixLiveAdapterConfigError {}

/// Bounded configuration for a transport-injected FIX execution adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixLiveAdapterConfig {
    session: FixSessionConfig,
    session_engine: FixSessionEngineConfig,
    resend_store: FixResendStoreConfig,
    max_frame_bytes: usize,
    max_frames_per_poll: usize,
    max_pending_gap_frames: usize,
    max_working_orders: usize,
    max_resend_sequences: usize,
    max_resend_actions: usize,
}

impl FixLiveAdapterConfig {
    /// Creates a bounded live adapter configuration.
    ///
    /// # Errors
    ///
    /// Returns [`FixLiveAdapterConfigError`] for an unknown begin string or an
    /// invalid heartbeat interval.
    pub fn new(session: FixSessionConfig) -> Result<Self, FixLiveAdapterConfigError> {
        FixVersion::from_bytes(session.begin_string.as_str().as_bytes())
            .ok_or(FixLiveAdapterConfigError::UnsupportedVersion)?;
        let session_engine = FixSessionEngineConfig::new(u32::from(session.heartbeat_secs))
            .map_err(|_| FixLiveAdapterConfigError::SessionConfig)?;
        Ok(Self {
            session,
            session_engine,
            resend_store: FixResendStoreConfig::default(),
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
            max_frames_per_poll: DEFAULT_MAX_FRAMES_PER_POLL,
            max_pending_gap_frames: DEFAULT_MAX_PENDING_GAP_FRAMES,
            max_working_orders: DEFAULT_MAX_WORKING_ORDERS,
            max_resend_sequences: DEFAULT_MAX_RESEND_SEQUENCES,
            max_resend_actions: DEFAULT_MAX_RESEND_ACTIONS,
        })
    }

    /// Overrides session-engine liveness/reset policy.
    pub const fn with_session_engine(mut self, session_engine: FixSessionEngineConfig) -> Self {
        self.session_engine = session_engine;
        self
    }

    /// Overrides in-memory resend retention bounds.
    pub const fn with_resend_store(mut self, resend_store: FixResendStoreConfig) -> Self {
        self.resend_store = resend_store;
        self
    }

    /// Sets the maximum complete frame size.
    ///
    /// # Errors
    ///
    /// Returns [`FixLiveAdapterConfigError::ZeroCapacity`] for zero.
    pub fn with_max_frame_bytes(mut self, value: usize) -> Result<Self, FixLiveAdapterConfigError> {
        require_capacity("max_frame_bytes", value)?;
        self.max_frame_bytes = value;
        Ok(self)
    }

    /// Sets the maximum transport frames processed by one `poll` call.
    pub fn with_max_frames_per_poll(
        mut self,
        value: usize,
    ) -> Result<Self, FixLiveAdapterConfigError> {
        require_capacity("max_frames_per_poll", value)?;
        self.max_frames_per_poll = value;
        Ok(self)
    }

    /// Sets the maximum held out-of-order frames.
    pub fn with_max_pending_gap_frames(
        mut self,
        value: usize,
    ) -> Result<Self, FixLiveAdapterConfigError> {
        require_capacity("max_pending_gap_frames", value)?;
        self.max_pending_gap_frames = value;
        Ok(self)
    }

    /// Sets the maximum locally tracked working orders.
    pub fn with_max_working_orders(
        mut self,
        value: usize,
    ) -> Result<Self, FixLiveAdapterConfigError> {
        require_capacity("max_working_orders", value)?;
        self.max_working_orders = value;
        Ok(self)
    }

    /// Sets maximum sequence span served by one peer resend request.
    pub fn with_max_resend_sequences(
        mut self,
        value: usize,
    ) -> Result<Self, FixLiveAdapterConfigError> {
        require_capacity("max_resend_sequences", value)?;
        self.max_resend_sequences = value;
        Ok(self)
    }

    /// Sets maximum replay/gap-fill actions generated by one resend request.
    pub fn with_max_resend_actions(
        mut self,
        value: usize,
    ) -> Result<Self, FixLiveAdapterConfigError> {
        require_capacity("max_resend_actions", value)?;
        self.max_resend_actions = value;
        Ok(self)
    }

    /// Returns legacy session identity/configuration.
    pub const fn session(&self) -> FixSessionConfig {
        self.session
    }

    /// Returns session-engine policy.
    pub const fn session_engine(&self) -> &FixSessionEngineConfig {
        &self.session_engine
    }

    /// Returns maximum frame bytes.
    pub const fn max_frame_bytes(&self) -> usize {
        self.max_frame_bytes
    }

    /// Returns maximum frames per poll.
    pub const fn max_frames_per_poll(&self) -> usize {
        self.max_frames_per_poll
    }

    /// Returns maximum held gap frames.
    pub const fn max_pending_gap_frames(&self) -> usize {
        self.max_pending_gap_frames
    }

    /// Returns maximum locally tracked working orders.
    pub const fn max_working_orders(&self) -> usize {
        self.max_working_orders
    }
}

/// Allocation-free operational counters for the live FIX adapter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixLiveAdapterMetrics {
    /// Transport frames received.
    pub frames_received: u64,
    /// Transport bytes received.
    pub bytes_received: u64,
    /// Original frames sent, excluding replay/gap-fill responses.
    pub frames_sent: u64,
    /// Original bytes sent.
    pub bytes_sent: u64,
    /// Canonical execution events emitted.
    pub events_emitted: u64,
    /// Malformed/profile-invalid inbound frames.
    pub inbound_errors: u64,
    /// Transport send failures.
    pub send_errors: u64,
    /// Out-of-order frames retained while recovering a gap.
    pub gap_frames_held: u64,
    /// Duplicate held frames suppressed.
    pub gap_frame_duplicates: u64,
    /// Held frames discarded because a SequenceReset advanced beyond them.
    pub gap_frames_discarded: u64,
    /// Replay frames sent for peer resend requests.
    pub replay_frames_sent: u64,
    /// Gap-fill frames sent for peer resend requests.
    pub gap_fill_frames_sent: u64,
    /// Peer resend requests rejected by configured work bounds.
    pub resend_requests_rejected: u64,
    /// Open-order recovery requests sent.
    pub recovery_requests_sent: u64,
    /// Session Reject `<3>` messages accepted and diagnosed.
    pub session_rejects: u64,
    /// BusinessMessageReject `<j>` messages accepted and diagnosed.
    pub business_rejects: u64,
    /// Latest non-negative exchange-to-local receive latency.
    pub last_exchange_to_receive_ns: u64,
    /// Maximum observed exchange-to-local receive latency.
    pub max_exchange_to_receive_ns: u64,
    /// Reports whose exchange timestamp was ahead of local receive time.
    pub exchange_clock_skew_reports: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WorkingOrder {
    current_id: ClientOrderId,
    pending_replace_id: Option<ClientOrderId>,
    context: FixWorkingOrderContext,
}

#[derive(Debug)]
struct HeldFrame {
    sequence: u64,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnedResendAction {
    Replay(u64),
    GapFill { begin: u64, end: u64 },
}

/// Synchronous, single-owner FIX execution adapter over injected infrastructure.
///
/// The adapter performs no hidden thread creation, async scheduling, socket
/// setup, TLS, or wall-clock reads. It reuses bounded buffers and statically
/// dispatches transport, time, and venue profile hooks. Callers drive network
/// and timer progress through [`ExecutionAdapter::poll`].
pub struct FixTransportExecutionAdapter<T, C, P, J = NoopFixOutboundJournal>
where
    T: FixFrameTransport,
    C: FixTimeSource,
    P: FixExecutionProfile,
    J: FixOutboundJournal,
{
    config: FixLiveAdapterConfig,
    transport: T,
    clock: C,
    profile: P,
    journal: J,
    session: FixSessionEngine,
    resend_store: FixResendStore,
    inbound: Vec<u8>,
    outbound: Vec<u8>,
    sending_time: [u8; FIX_TIMESTAMP_CAPACITY],
    working_orders: Vec<WorkingOrder>,
    held_frames: Vec<HeldFrame>,
    free_frame_buffers: Vec<Vec<u8>>,
    resend_actions: Vec<OwnedResendAction>,
    pending_event: Option<ExecutionEvent>,
    transport_connected: bool,
    recovery_requested: bool,
    protocol_degraded: bool,
    health_seq: u64,
    last_error: Option<String>,
    metrics: FixLiveAdapterMetrics,
}

impl<T, C, P, J> FixTransportExecutionAdapter<T, C, P, J>
where
    T: FixFrameTransport,
    C: FixTimeSource,
    P: FixExecutionProfile,
    J: FixOutboundJournal,
{
    /// Creates a live adapter with sequence numbers starting at one.
    ///
    /// # Errors
    ///
    /// Returns [`FixLiveAdapterConfigError`] when the profile version or FIX
    /// session identity is inconsistent.
    pub fn new(
        config: FixLiveAdapterConfig,
        transport: T,
        clock: C,
        profile: P,
    ) -> Result<Self, FixLiveAdapterConfigError>
    where
        J: Default,
    {
        Self::with_sequences(config, transport, clock, profile, FixSequenceTracker::new())
    }

    /// Creates a live adapter from restored FIX sequence counters.
    ///
    /// # Errors
    ///
    /// Returns [`FixLiveAdapterConfigError`] when the profile version or FIX
    /// session identity is inconsistent.
    pub fn with_sequences(
        config: FixLiveAdapterConfig,
        transport: T,
        clock: C,
        profile: P,
        sequences: FixSequenceTracker,
    ) -> Result<Self, FixLiveAdapterConfigError>
    where
        J: Default,
    {
        Self::with_journal_and_resend_store(
            config,
            transport,
            clock,
            profile,
            J::default(),
            FixResendStore::new(config.resend_store),
            sequences,
        )
    }

    /// Creates an adapter with a caller-owned durable outbound journal.
    ///
    /// # Errors
    ///
    /// Returns [`FixLiveAdapterConfigError`] when profile/session identity is
    /// inconsistent.
    pub fn with_journal(
        config: FixLiveAdapterConfig,
        transport: T,
        clock: C,
        profile: P,
        journal: J,
    ) -> Result<Self, FixLiveAdapterConfigError> {
        Self::with_journal_and_resend_store(
            config,
            transport,
            clock,
            profile,
            journal,
            FixResendStore::new(config.resend_store),
            FixSequenceTracker::new(),
        )
    }

    /// Creates an adapter from validated, caller-restored resend and sequence
    /// state plus a durable outbound journal.
    ///
    /// Recovery I/O remains outside the adapter's order path. This lets hosts
    /// validate durable state and fail closed before opening a transport.
    ///
    /// # Errors
    ///
    /// Returns [`FixLiveAdapterConfigError`] when profile/session identity is
    /// inconsistent.
    pub fn with_journal_and_resend_store(
        config: FixLiveAdapterConfig,
        transport: T,
        clock: C,
        profile: P,
        journal: J,
        resend_store: FixResendStore,
        sequences: FixSequenceTracker,
    ) -> Result<Self, FixLiveAdapterConfigError> {
        let version = FixVersion::from_bytes(config.session.begin_string.as_str().as_bytes())
            .ok_or(FixLiveAdapterConfigError::UnsupportedVersion)?;
        if profile.version() != version {
            return Err(FixLiveAdapterConfigError::ProfileVersionMismatch);
        }
        let session_id = FixOwnedSessionId::new(
            version,
            config.session.sender_comp_id.as_str().as_bytes().to_vec(),
            config.session.target_comp_id.as_str().as_bytes().to_vec(),
        )
        .map_err(|_| FixLiveAdapterConfigError::SessionIdentity)?;

        let mut free_frame_buffers = Vec::with_capacity(config.max_pending_gap_frames);
        for _ in 0..config.max_pending_gap_frames {
            free_frame_buffers.push(Vec::with_capacity(config.max_frame_bytes));
        }
        Ok(Self {
            session: FixSessionEngine::with_sequences(config.session_engine, session_id, sequences),
            resend_store,
            inbound: vec![0; config.max_frame_bytes],
            outbound: Vec::with_capacity(config.max_frame_bytes),
            sending_time: [0; FIX_TIMESTAMP_CAPACITY],
            working_orders: Vec::with_capacity(config.max_working_orders),
            held_frames: Vec::with_capacity(config.max_pending_gap_frames),
            free_frame_buffers,
            resend_actions: Vec::with_capacity(config.max_resend_actions),
            pending_event: None,
            transport_connected: false,
            recovery_requested: false,
            protocol_degraded: false,
            health_seq: 0,
            last_error: None,
            metrics: FixLiveAdapterMetrics::default(),
            config,
            transport,
            clock,
            profile,
            journal,
        })
    }

    /// Returns adapter configuration.
    pub const fn config(&self) -> &FixLiveAdapterConfig {
        &self.config
    }

    /// Returns FIX session state.
    pub const fn session_state(&self) -> FixSessionState {
        self.session.state()
    }

    /// Returns FIX session counters.
    pub const fn session_metrics(&self) -> FixSessionMetrics {
        self.session.metrics()
    }

    /// Returns adapter counters.
    pub const fn metrics(&self) -> FixLiveAdapterMetrics {
        self.metrics
    }

    /// Returns the next inbound and outbound sequence tracker.
    pub const fn sequences(&self) -> &FixSequenceTracker {
        self.session.sequences()
    }

    /// Returns immutable transport access for diagnostics/tests.
    pub const fn transport(&self) -> &T {
        &self.transport
    }

    /// Returns mutable transport access for host integration.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Returns immutable profile access.
    pub const fn profile(&self) -> &P {
        &self.profile
    }

    /// Returns mutable profile access.
    pub fn profile_mut(&mut self) -> &mut P {
        &mut self.profile
    }

    /// Returns immutable outbound-journal access.
    pub const fn journal(&self) -> &J {
        &self.journal
    }

    /// Returns mutable outbound-journal access.
    pub fn journal_mut(&mut self) -> &mut J {
        &mut self.journal
    }

    /// Builds an owned, checksummed sequence snapshot for an atomic
    /// `of_fix::FixSequenceSnapshotStore` checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when `trading_day` is not a valid FIX value.
    pub fn sequence_snapshot(
        &self,
        trading_day: impl Into<Vec<u8>>,
    ) -> ExecutionResult<FixOwnedSequenceSnapshot> {
        FixOwnedSequenceSnapshot::new(
            self.session.session_id().clone(),
            self.session.sequences().next_inbound(),
            self.session.sequences().next_outbound(),
            trading_day,
        )
        .map_err(|error| adapter_error("FIX sequence snapshot", error))
    }

    /// Restores original-order context needed for post-restart cancel/replace.
    ///
    /// The OMS should call this from its validated checkpoint before enabling
    /// submissions. Exact duplicates are idempotent; conflicting duplicates
    /// fail closed.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::BufferFull`] at the configured working-order
    /// bound or an adapter error for conflicting context.
    pub fn restore_working_order(
        &mut self,
        client_order_id: ClientOrderId,
        context: FixWorkingOrderContext,
    ) -> ExecutionResult<()> {
        if let Some(index) = self.find_working(client_order_id) {
            if self.working_orders[index].current_id == client_order_id
                && self.working_orders[index].pending_replace_id.is_none()
                && self.working_orders[index].context == context
            {
                return Ok(());
            }
            return Err(ExecutionError::Adapter(
                "conflicting FIX working-order recovery context".to_string(),
            ));
        }
        if self.working_orders.len() >= self.config.max_working_orders {
            return Err(ExecutionError::BufferFull);
        }
        self.working_orders.push(WorkingOrder {
            current_id: client_order_id,
            pending_replace_id: None,
            context,
        });
        Ok(())
    }

    /// Returns locally tracked working-order count.
    pub const fn working_order_count(&self) -> usize {
        self.working_orders.len()
    }

    /// Clears an operator-observed protocol reject degradation.
    ///
    /// This does not change session lifecycle or erase the last diagnostic.
    pub fn acknowledge_protocol_degradation(&mut self) {
        if self.protocol_degraded {
            self.protocol_degraded = false;
            self.bump_health(None);
        }
    }

    /// Sends a graceful FIX Logout when the session is active.
    ///
    /// # Errors
    ///
    /// Returns a clock, session, retention, bounds, or transport error.
    pub fn request_logout(&mut self, text: Option<&[u8]>) -> ExecutionResult<()> {
        let sample = self.sample_time()?;
        let action = self
            .session
            .request_logout(
                sample.monotonic_ns(),
                &self.sending_time[..sample.sending_time_len()],
                text,
                &mut self.outbound,
            )
            .map_err(session_error)?;
        self.send_session_action(action)
    }

    /// Forces transport closure and informs the session state machine.
    ///
    /// # Errors
    ///
    /// Returns a clock, transport, or session error.
    pub fn disconnect(&mut self) -> ExecutionResult<()> {
        let sample = self.sample_time()?;
        self.transport
            .disconnect()
            .map_err(|error| adapter_error("FIX transport disconnect", error))?;
        self.session
            .on_transport_disconnected(sample.monotonic_ns())
            .map_err(session_error)?;
        self.transport_connected = false;
        self.recovery_requested = false;
        self.bump_health(None);
        Ok(())
    }

    fn sample_time(&mut self) -> ExecutionResult<FixTimeSample> {
        let sample = self
            .clock
            .sample(&mut self.sending_time)
            .map_err(|error| adapter_error("FIX time source", error))?;
        if sample.sending_time_len() == 0 || sample.sending_time_len() > self.sending_time.len() {
            return Err(ExecutionError::Adapter(
                "FIX time source returned invalid SendingTime length".to_string(),
            ));
        }
        Ok(sample)
    }

    fn ensure_ready(&self) -> ExecutionResult<()> {
        if self.transport_connected && self.session.state() == FixSessionState::Ready {
            Ok(())
        } else {
            Err(ExecutionError::Disconnected)
        }
    }

    fn send_session_action(&mut self, action: FixSessionAction) -> ExecutionResult<()> {
        match action {
            FixSessionAction::Send { sequence, .. } => {
                self.retain_original(sequence, FixSentMessageKind::Administrative)?;
                self.send_outbound_original()
            }
            FixSessionAction::GapDetected {
                request_sequence, ..
            } => {
                self.retain_original(request_sequence, FixSentMessageKind::Administrative)?;
                self.send_outbound_original()
            }
            FixSessionAction::Disconnect { .. } => {
                self.transport
                    .disconnect()
                    .map_err(|error| adapter_error("FIX transport disconnect", error))?;
                self.transport_connected = false;
                self.recovery_requested = false;
                self.bump_health(None);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn retain_original(&mut self, sequence: u64, kind: FixSentMessageKind) -> ExecutionResult<()> {
        self.enforce_outbound_bound()?;
        self.journal
            .record_sent(sequence, kind, &self.outbound)
            .map_err(|error| adapter_error("FIX outbound journal", error))?;
        self.resend_store
            .record_sent(sequence, kind, &self.outbound)
            .map_err(resend_error)?;
        Ok(())
    }

    fn send_outbound_original(&mut self) -> ExecutionResult<()> {
        self.enforce_outbound_bound()?;
        let len = self.outbound.len();
        match self.transport.send(&self.outbound) {
            Ok(()) => {
                self.metrics.frames_sent = self.metrics.frames_sent.saturating_add(1);
                self.metrics.bytes_sent = self.metrics.bytes_sent.saturating_add(len as u64);
                Ok(())
            }
            Err(error) => {
                self.metrics.send_errors = self.metrics.send_errors.saturating_add(1);
                let message = format!("FIX transport send: {error}");
                self.bump_health(Some(message.clone()));
                Err(ExecutionError::Adapter(message))
            }
        }
    }

    fn enforce_outbound_bound(&self) -> ExecutionResult<()> {
        if self.outbound.len() <= self.config.max_frame_bytes {
            Ok(())
        } else {
            Err(ExecutionError::Adapter(format!(
                "FIX outbound frame {} exceeds configured maximum {}",
                self.outbound.len(),
                self.config.max_frame_bytes
            )))
        }
    }

    fn bump_health(&mut self, error: Option<String>) {
        self.health_seq = self.health_seq.saturating_add(1);
        if error.is_some() {
            self.last_error = error;
        }
    }

    fn find_working(&self, id: ClientOrderId) -> Option<usize> {
        self.working_orders.iter().position(|order| {
            order.current_id == id
                || order
                    .pending_replace_id
                    .is_some_and(|pending| pending == id)
        })
    }

    fn update_working_order(&mut self, event: &ExecutionEvent) {
        let lookup = if !event.orig_client_order_id.is_empty() {
            event.orig_client_order_id
        } else {
            event.client_order_id
        };
        let Some(index) = self.find_working(lookup) else {
            return;
        };
        match event.exec_type {
            ExecutionType::ReplaceAck => {
                self.working_orders[index].current_id = event.client_order_id;
                self.working_orders[index].pending_replace_id = None;
            }
            ExecutionType::ReplaceReject => {
                self.working_orders[index].pending_replace_id = None;
            }
            _ => {}
        }
        if matches!(
            event.order_status,
            OrderStatus::Filled
                | OrderStatus::Cancelled
                | OrderStatus::Rejected
                | OrderStatus::Expired
        ) {
            self.working_orders.swap_remove(index);
        }
    }

    fn hold_current_frame(&mut self, sequence: u64, len: usize) -> ExecutionResult<()> {
        match self
            .held_frames
            .binary_search_by_key(&sequence, |frame| frame.sequence)
        {
            Ok(_) => {
                self.metrics.gap_frame_duplicates =
                    self.metrics.gap_frame_duplicates.saturating_add(1);
                return Ok(());
            }
            Err(index) => {
                let Some(mut bytes) = self.free_frame_buffers.pop() else {
                    return Err(ExecutionError::BufferFull);
                };
                bytes.clear();
                bytes.extend_from_slice(&self.inbound[..len]);
                self.held_frames
                    .insert(index, HeldFrame { sequence, bytes });
            }
        }
        self.metrics.gap_frames_held = self.metrics.gap_frames_held.saturating_add(1);
        Ok(())
    }

    fn process_held_ready(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        let mut emitted = 0usize;
        let expected = self.session.sequences().next_inbound();
        let obsolete = self
            .held_frames
            .partition_point(|frame| frame.sequence < expected);
        if obsolete > 0 {
            self.metrics.gap_frames_discarded = self
                .metrics
                .gap_frames_discarded
                .saturating_add(obsolete as u64);
            for frame in self.held_frames.drain(..obsolete) {
                self.free_frame_buffers.push(frame.bytes);
            }
        }
        while !self.held_frames.is_empty() {
            let frame = &self.held_frames[0];
            if frame.sequence != self.session.sequences().next_inbound() {
                break;
            }
            let frame = self.held_frames.remove(0);
            let len = frame.bytes.len();
            self.inbound[..len].copy_from_slice(&frame.bytes);
            self.free_frame_buffers.push(frame.bytes);
            emitted = emitted.saturating_add(self.process_inbound_frame(len, out)?);
            if self.pending_event.is_some() {
                break;
            }
        }
        Ok(emitted)
    }

    fn process_inbound_frame(
        &mut self,
        len: usize,
        out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<usize> {
        if len == 0 || len > self.inbound.len() {
            self.metrics.inbound_errors = self.metrics.inbound_errors.saturating_add(1);
            return Err(ExecutionError::Adapter(
                "FIX transport returned invalid frame length".to_string(),
            ));
        }
        self.metrics.frames_received = self.metrics.frames_received.saturating_add(1);
        self.metrics.bytes_received = self.metrics.bytes_received.saturating_add(len as u64);
        let sample = self.sample_time()?;
        let mut scratch = [FixFieldView::empty(); FIX_FIELD_SCRATCH_CAPACITY];

        #[derive(Default)]
        struct Outcome {
            event: Option<ExecutionEvent>,
            session: Option<FixSessionAction>,
            hold: Option<(u64, Option<u64>)>,
            peer_resend: Option<FixResendRange>,
            diagnostic: Option<(String, bool)>,
        }

        let outcome = {
            let message = parse_message(&self.inbound[..len], &mut scratch).map_err(|error| {
                self.metrics.inbound_errors = self.metrics.inbound_errors.saturating_add(1);
                ExecutionError::Adapter(format!("FIX parse error: {error}"))
            })?;
            let sequence = parse_message_sequence(&message)?;
            if matches!(
                self.session.state(),
                FixSessionState::ResendRequested | FixSessionState::Recovering
            ) && sequence > self.session.sequences().next_inbound()
            {
                Outcome {
                    hold: Some((sequence, None)),
                    ..Outcome::default()
                }
            } else {
                let action = self
                    .session
                    .on_inbound(
                        &message,
                        sample.monotonic_ns(),
                        &self.sending_time[..sample.sending_time_len()],
                        &mut self.outbound,
                    )
                    .map_err(session_error)?;
                match action {
                    FixSessionAction::Application { .. } => Outcome {
                        event: self
                            .profile
                            .map_application(&message, sample.unix_ns())
                            .map_err(|error| adapter_error("FIX profile inbound", error))?,
                        ..Outcome::default()
                    },
                    FixSessionAction::GapDetected {
                        received,
                        request_sequence,
                        ..
                    } => Outcome {
                        hold: Some((received, Some(request_sequence))),
                        ..Outcome::default()
                    },
                    FixSessionAction::PeerResendRequested { range, .. } => Outcome {
                        peer_resend: Some(range),
                        ..Outcome::default()
                    },
                    FixSessionAction::Send { .. } | FixSessionAction::Disconnect { .. } => {
                        Outcome {
                            session: Some(action),
                            ..Outcome::default()
                        }
                    }
                    FixSessionAction::Administrative {
                        message_type: FixMsgType::REJECT,
                        ..
                    } => Outcome {
                        diagnostic: Some((format_session_reject(&message)?, false)),
                        ..Outcome::default()
                    },
                    FixSessionAction::Administrative {
                        message_type: FixMsgType::BUSINESS_MESSAGE_REJECT,
                        ..
                    } => Outcome {
                        diagnostic: Some((format_business_reject(&message)?, true)),
                        ..Outcome::default()
                    },
                    _ => Outcome::default(),
                }
            }
        };

        if let Some(event) = outcome.event {
            if event.ts_exchange_ns > 0 {
                if event.ts_recv_ns >= event.ts_exchange_ns {
                    let latency = event.ts_recv_ns - event.ts_exchange_ns;
                    self.metrics.last_exchange_to_receive_ns = latency;
                    self.metrics.max_exchange_to_receive_ns =
                        self.metrics.max_exchange_to_receive_ns.max(latency);
                } else {
                    self.metrics.exchange_clock_skew_reports =
                        self.metrics.exchange_clock_skew_reports.saturating_add(1);
                }
            }
            self.update_working_order(&event);
            if let Err(error) = out.push(event) {
                self.pending_event = Some(event);
                return Err(error);
            }
            self.metrics.events_emitted = self.metrics.events_emitted.saturating_add(1);
            return Ok(1);
        }
        if let Some(action) = outcome.session {
            self.send_session_action(action)?;
            return Ok(0);
        }
        if let Some((sequence, request_sequence)) = outcome.hold {
            if let Some(request_sequence) = request_sequence {
                self.send_session_action(FixSessionAction::Send {
                    kind: of_fix::FixSessionSendKind::ResendRequest,
                    sequence: request_sequence,
                })?;
            }
            if let Err(error) = self.hold_current_frame(sequence, len) {
                let message = "FIX pending gap-frame buffer is full".to_string();
                self.bump_health(Some(message));
                return Err(error);
            }
            return Ok(0);
        }
        if let Some(range) = outcome.peer_resend {
            self.respond_to_resend(range, sample)?;
        }
        if let Some((diagnostic, business)) = outcome.diagnostic {
            if business {
                self.metrics.business_rejects = self.metrics.business_rejects.saturating_add(1);
            } else {
                self.metrics.session_rejects = self.metrics.session_rejects.saturating_add(1);
            }
            self.protocol_degraded = true;
            self.bump_health(Some(diagnostic));
        }
        Ok(0)
    }

    fn respond_to_resend(
        &mut self,
        range: FixResendRange,
        sample: FixTimeSample,
    ) -> ExecutionResult<()> {
        self.build_resend_actions(range)?;
        for index in 0..self.resend_actions.len() {
            let action = self.resend_actions[index];
            match action {
                OwnedResendAction::Replay(sequence) => {
                    let raw = self
                        .resend_store
                        .get(sequence)
                        .ok_or_else(|| ExecutionError::Adapter("FIX resend source missing".into()))?
                        .raw();
                    let mut scratch = [FixFieldView::empty(); FIX_FIELD_SCRATCH_CAPACITY];
                    let message = parse_message(raw, &mut scratch).map_err(|error| {
                        ExecutionError::Adapter(format!("FIX resend parse: {error}"))
                    })?;
                    encode_poss_dup_replay(
                        &mut self.outbound,
                        &message,
                        &self.sending_time[..sample.sending_time_len()],
                    )
                    .map_err(|error| {
                        ExecutionError::Adapter(format!("FIX resend encode: {error}"))
                    })?;
                    self.enforce_outbound_bound()?;
                    self.transport
                        .send(&self.outbound)
                        .map_err(|error| adapter_error("FIX resend transport", error))?;
                    self.session
                        .record_replay_sent(sample.monotonic_ns())
                        .map_err(session_error)?;
                    self.metrics.replay_frames_sent =
                        self.metrics.replay_frames_sent.saturating_add(1);
                }
                OwnedResendAction::GapFill { begin, end } => {
                    let session_id = self.session.session_id();
                    let header = FixSessionHeader::new(
                        session_id.sender_comp_id(),
                        session_id.target_comp_id(),
                        begin,
                        &self.sending_time[..sample.sending_time_len()],
                    );
                    encode_sequence_reset_gap_fill(
                        &mut self.outbound,
                        session_id.version(),
                        header,
                        end.saturating_add(1),
                    )
                    .map_err(|error| {
                        ExecutionError::Adapter(format!("FIX gap-fill encode: {error}"))
                    })?;
                    self.enforce_outbound_bound()?;
                    self.transport
                        .send(&self.outbound)
                        .map_err(|error| adapter_error("FIX gap-fill transport", error))?;
                    self.session
                        .record_replay_sent(sample.monotonic_ns())
                        .map_err(session_error)?;
                    self.metrics.gap_fill_frames_sent =
                        self.metrics.gap_fill_frames_sent.saturating_add(1);
                }
            }
        }
        Ok(())
    }

    fn build_resend_actions(&mut self, range: FixResendRange) -> ExecutionResult<()> {
        self.resend_actions.clear();
        let newest = self.resend_store.metrics().newest_seq_no();
        let Some(end) = (if range.end_seq_no == 0 {
            newest
        } else {
            Some(range.end_seq_no)
        }) else {
            return Ok(());
        };
        if range.begin_seq_no == 0 || range.begin_seq_no > end {
            self.metrics.resend_requests_rejected =
                self.metrics.resend_requests_rejected.saturating_add(1);
            return Err(ExecutionError::Adapter("invalid FIX resend range".into()));
        }
        let span = end.saturating_sub(range.begin_seq_no).saturating_add(1);
        if span > self.config.max_resend_sequences as u64 {
            self.metrics.resend_requests_rejected =
                self.metrics.resend_requests_rejected.saturating_add(1);
            return Err(ExecutionError::Adapter(format!(
                "FIX resend range {span} exceeds configured maximum {}",
                self.config.max_resend_sequences
            )));
        }

        let mut gap_start = None;
        for sequence in range.begin_seq_no..=end {
            let replayable = self
                .resend_store
                .get(sequence)
                .is_some_and(|message| message.replayable());
            if replayable {
                if let Some(begin) = gap_start.take() {
                    self.push_resend_action(OwnedResendAction::GapFill {
                        begin,
                        end: sequence.saturating_sub(1),
                    })?;
                }
                self.push_resend_action(OwnedResendAction::Replay(sequence))?;
            } else if gap_start.is_none() {
                gap_start = Some(sequence);
            }
        }
        if let Some(begin) = gap_start {
            self.push_resend_action(OwnedResendAction::GapFill { begin, end })?;
        }
        Ok(())
    }

    fn push_resend_action(&mut self, action: OwnedResendAction) -> ExecutionResult<()> {
        if self.resend_actions.len() >= self.config.max_resend_actions {
            self.metrics.resend_requests_rejected =
                self.metrics.resend_requests_rejected.saturating_add(1);
            return Err(ExecutionError::BufferFull);
        }
        self.resend_actions.push(action);
        Ok(())
    }

    fn encode_application<F>(
        &mut self,
        now_ns: u64,
        sending_time_len: usize,
        encode: F,
    ) -> ExecutionResult<u64>
    where
        F: FnOnce(&mut P, &mut Vec<u8>, FixSessionHeader<'_>, &[u8]) -> Result<(), P::Error>,
    {
        let expected = self.session.sequences().next_outbound();
        let session_id = self.session.session_id();
        let sending_time = &self.sending_time;
        let header = FixSessionHeader::new(
            session_id.sender_comp_id(),
            session_id.target_comp_id(),
            expected,
            &sending_time[..sending_time_len],
        );
        encode(
            &mut self.profile,
            &mut self.outbound,
            header,
            &sending_time[..sending_time_len],
        )
        .map_err(|error| adapter_error("FIX profile outbound", error))?;
        self.enforce_outbound_bound()?;
        let assigned = self
            .session
            .assign_application_sequence(now_ns)
            .map_err(session_error)?;
        debug_assert_eq!(assigned, expected);
        self.retain_original(assigned, FixSentMessageKind::Application)?;
        Ok(assigned)
    }
}

impl<T, C, P, J> ExecutionAdapter for FixTransportExecutionAdapter<T, C, P, J>
where
    T: FixFrameTransport,
    C: FixTimeSource,
    P: FixExecutionProfile,
    J: FixOutboundJournal,
{
    fn connect(&mut self) -> ExecutionResult<()> {
        self.session
            .on_transport_connecting()
            .map_err(session_error)?;
        if let Err(error) = self.transport.connect() {
            let message = format!("FIX transport connect: {error}");
            self.session.stop();
            self.bump_health(Some(message.clone()));
            return Err(ExecutionError::Adapter(message));
        }
        self.transport_connected = true;
        self.recovery_requested = false;
        self.protocol_degraded = false;
        let result = (|| {
            let sample = self.sample_time()?;
            let action = self
                .session
                .on_transport_connected(
                    sample.monotonic_ns(),
                    &self.sending_time[..sample.sending_time_len()],
                    &mut self.outbound,
                )
                .map_err(session_error)?;
            self.send_session_action(action)
        })();
        if let Err(error) = result {
            let _ = self.transport.disconnect();
            self.transport_connected = false;
            self.session.stop();
            self.bump_health(Some(error.to_string()));
            return Err(error);
        }
        self.bump_health(None);
        Ok(())
    }

    fn submit(
        &mut self,
        request: &OrderRequest,
        _out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        self.ensure_ready()?;
        request.validate()?;
        let capabilities = self.profile.capabilities();
        if !capabilities.supports_order_type(request.order_type)
            || !capabilities.supports_tif(request.time_in_force)
        {
            return Err(ExecutionError::Adapter(
                "FIX profile does not support requested order type or TIF".to_string(),
            ));
        }
        if self.find_working(request.client_order_id).is_some() {
            return Err(ExecutionError::Adapter(
                "duplicate FIX client order id".to_string(),
            ));
        }
        if self.working_orders.len() >= self.config.max_working_orders {
            return Err(ExecutionError::BufferFull);
        }
        let sample = self.sample_time()?;
        self.encode_application(
            sample.monotonic_ns(),
            sample.sending_time_len(),
            |profile, out, header, time| profile.encode_submit(out, header, request, time),
        )?;
        self.working_orders.push(WorkingOrder {
            current_id: request.client_order_id,
            pending_replace_id: None,
            context: FixWorkingOrderContext::new(
                request.side,
                request.order_type,
                request.time_in_force,
                request.stop_price,
            ),
        });
        self.send_outbound_original()
    }

    fn cancel(
        &mut self,
        request: &CancelRequest,
        _out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        self.ensure_ready()?;
        let index = self
            .find_working(request.orig_client_order_id)
            .ok_or_else(|| ExecutionError::Adapter("FIX cancel original order unknown".into()))?;
        let original = self.working_orders[index].context;
        let sample = self.sample_time()?;
        self.encode_application(
            sample.monotonic_ns(),
            sample.sending_time_len(),
            |profile, out, header, time| {
                profile.encode_cancel(out, header, request, original, time)
            },
        )?;
        self.send_outbound_original()
    }

    fn amend(
        &mut self,
        request: &AmendRequest,
        _out: &mut ExecutionEventBuffer,
    ) -> ExecutionResult<()> {
        self.ensure_ready()?;
        if !self.profile.capabilities().amend {
            return Err(ExecutionError::Adapter(
                "FIX profile does not support cancel/replace".to_string(),
            ));
        }
        if request.quantity.0 <= 0 {
            return Err(ExecutionError::Core(
                of_execution_core::ExecutionCoreError::InvalidQuantity,
            ));
        }
        let index = self
            .find_working(request.orig_client_order_id)
            .ok_or_else(|| ExecutionError::Adapter("FIX amend original order unknown".into()))?;
        if self.working_orders[index].pending_replace_id.is_some() {
            return Err(ExecutionError::Adapter(
                "FIX amend already pending for order".into(),
            ));
        }
        let original = self.working_orders[index].context;
        let sample = self.sample_time()?;
        self.encode_application(
            sample.monotonic_ns(),
            sample.sending_time_len(),
            |profile, out, header, time| profile.encode_amend(out, header, request, original, time),
        )?;
        self.working_orders[index].pending_replace_id = Some(request.client_order_id);
        self.send_outbound_original()
    }

    fn poll(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        if !self.transport_connected {
            return Err(ExecutionError::Disconnected);
        }
        let mut emitted = 0usize;
        if let Some(event) = self.pending_event.take() {
            out.push(event)?;
            emitted = emitted.saturating_add(1);
            self.metrics.events_emitted = self.metrics.events_emitted.saturating_add(1);
        }

        emitted = emitted.saturating_add(self.process_held_ready(out)?);
        for _ in 0..self.config.max_frames_per_poll {
            match self
                .transport
                .poll_receive(&mut self.inbound)
                .map_err(|error| adapter_error("FIX transport receive", error))?
            {
                FixTransportPoll::Idle => break,
                FixTransportPoll::Disconnected => {
                    let sample = self.sample_time()?;
                    self.session
                        .on_transport_disconnected(sample.monotonic_ns())
                        .map_err(session_error)?;
                    self.transport_connected = false;
                    self.recovery_requested = false;
                    self.bump_health(Some("FIX peer disconnected".to_string()));
                    break;
                }
                FixTransportPoll::Frame { len } => {
                    let previous_state = self.session.state();
                    emitted = emitted.saturating_add(self.process_inbound_frame(len, out)?);
                    emitted = emitted.saturating_add(self.process_held_ready(out)?);
                    if self.session.state() != previous_state {
                        self.bump_health(None);
                    }
                    if self.pending_event.is_some() {
                        break;
                    }
                }
            }
        }

        if self.transport_connected {
            let sample = self.sample_time()?;
            let action = self
                .session
                .on_timer(
                    sample.monotonic_ns(),
                    &self.sending_time[..sample.sending_time_len()],
                    &mut self.outbound,
                )
                .map_err(session_error)?;
            self.send_session_action(action)?;
        }
        Ok(emitted)
    }

    fn recover_open_orders(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize> {
        self.ensure_ready()?;
        let emitted = self.poll(out)?;
        self.ensure_ready()?;
        if self.recovery_requested {
            return Ok(emitted);
        }
        let sample = self.sample_time()?;
        let expected = self.session.sequences().next_outbound();
        let mut request_id = [0u8; 32];
        let request_len = write_recovery_id(&mut request_id, expected);
        let session_id = self.session.session_id();
        let header = FixSessionHeader::new(
            session_id.sender_comp_id(),
            session_id.target_comp_id(),
            expected,
            &self.sending_time[..sample.sending_time_len()],
        );
        let encoded = self
            .profile
            .encode_open_order_recovery(&mut self.outbound, header, &request_id[..request_len])
            .map_err(|error| adapter_error("FIX profile recovery", error))?;
        if !encoded {
            return Err(ExecutionError::Adapter(
                "FIX profile does not define open-order recovery".into(),
            ));
        }
        let assigned = self
            .session
            .assign_application_sequence(sample.monotonic_ns())
            .map_err(session_error)?;
        debug_assert_eq!(assigned, expected);
        self.retain_original(assigned, FixSentMessageKind::Application)?;
        self.send_outbound_original()?;
        self.recovery_requested = true;
        self.metrics.recovery_requests_sent = self.metrics.recovery_requests_sent.saturating_add(1);
        Ok(emitted)
    }

    fn capabilities(&self) -> ExecutionCapabilities {
        self.profile.capabilities()
    }

    fn health(&self) -> ExecutionHealth {
        let ready = self.transport_connected && self.session.state() == FixSessionState::Ready;
        ExecutionHealth {
            connected: ready,
            degraded: self.protocol_degraded
                || !ready
                || matches!(
                    self.session.state(),
                    FixSessionState::Degraded
                        | FixSessionState::ResendRequested
                        | FixSessionState::Recovering
                ),
            health_seq: self.health_seq,
            last_error: self.last_error.clone(),
            protocol_info: Some(format!(
                "{}:{}->{}:{:?}:in={}:out={}:held={}",
                self.config.session.begin_string,
                self.config.session.sender_comp_id,
                self.config.session.target_comp_id,
                self.session.state(),
                self.session.sequences().next_inbound(),
                self.session.sequences().next_outbound(),
                self.held_frames.len()
            )),
        }
    }
}

fn require_capacity(name: &'static str, value: usize) -> Result<(), FixLiveAdapterConfigError> {
    if value == 0 {
        Err(FixLiveAdapterConfigError::ZeroCapacity(name))
    } else {
        Ok(())
    }
}

fn parse_message_sequence(message: &FixMessageView<'_>) -> ExecutionResult<u64> {
    let bytes = message
        .get(FixTag::MSG_SEQ_NUM)
        .ok_or_else(|| ExecutionError::Adapter("FIX frame missing MsgSeqNum(34)".into()))?;
    if bytes.is_empty() {
        return Err(ExecutionError::Adapter("FIX MsgSeqNum(34) is empty".into()));
    }
    let mut value = 0u64;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return Err(ExecutionError::Adapter(
                "FIX MsgSeqNum(34) is malformed".into(),
            ));
        }
        value = value
            .checked_mul(10)
            .and_then(|current| current.checked_add(u64::from(*byte - b'0')))
            .ok_or_else(|| ExecutionError::Adapter("FIX MsgSeqNum(34) overflow".into()))?;
    }
    if value == 0 {
        return Err(ExecutionError::Adapter(
            "FIX MsgSeqNum(34) must be non-zero".into(),
        ));
    }
    Ok(value)
}

fn format_session_reject(message: &FixMessageView<'_>) -> ExecutionResult<String> {
    let reject = parse_session_reject(message)
        .map_err(|error| adapter_error("FIX Session Reject parse", error))?;
    Ok(format!(
        "FIX Session Reject: ref_seq_num={}, ref_tag={:?}, ref_msg_type={}, reason={:?}, text={}",
        reject.ref_seq_num(),
        reject.ref_tag_id(),
        display_fix_bytes(reject.ref_msg_type()),
        reject.session_reject_reason(),
        display_fix_bytes(reject.text())
    ))
}

fn format_business_reject(message: &FixMessageView<'_>) -> ExecutionResult<String> {
    let reject = parse_business_message_reject(message)
        .map_err(|error| adapter_error("FIX BusinessMessageReject parse", error))?;
    Ok(format!(
        "FIX BusinessMessageReject: ref_seq_num={:?}, ref_msg_type={}, ref_id={}, reason={}, text={}",
        reject.ref_seq_num(),
        display_fix_bytes(Some(reject.ref_msg_type())),
        display_fix_bytes(reject.business_reject_ref_id()),
        reject.business_reject_reason(),
        display_fix_bytes(reject.text())
    ))
}

fn display_fix_bytes(value: Option<&[u8]>) -> String {
    const MAX_DIAGNOSTIC_BYTES: usize = 256;
    let Some(bytes) = value else {
        return "<absent>".to_string();
    };
    let truncated = bytes.len() > MAX_DIAGNOSTIC_BYTES;
    let mut rendered =
        String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_DIAGNOSTIC_BYTES)]).into_owned();
    if truncated {
        rendered.push_str("...[truncated]");
    }
    rendered
}

fn write_recovery_id(out: &mut [u8; 32], sequence: u64) -> usize {
    const PREFIX: &[u8] = b"OF-RECOVERY-";
    out[..PREFIX.len()].copy_from_slice(PREFIX);
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    let mut value = sequence;
    if value == 0 {
        digits[0] = b'0';
        len = 1;
    } else {
        while value > 0 {
            digits[len] = b'0' + (value % 10) as u8;
            value /= 10;
            len += 1;
        }
    }
    for index in 0..len {
        out[PREFIX.len() + index] = digits[len - index - 1];
    }
    PREFIX.len() + len
}

fn adapter_error(context: &str, error: impl fmt::Display) -> ExecutionError {
    ExecutionError::Adapter(format!("{context}: {error}"))
}

fn session_error(error: FixSessionError) -> ExecutionError {
    adapter_error("FIX session", error)
}

fn resend_error(error: FixResendStoreError) -> ExecutionError {
    adapter_error("FIX resend store", error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use of_execution_core::{
        AccountId, ExecutionId, ExecutionSymbol, FixedAscii, InstrumentId, OrderQty, RouteId,
        StrategyId, VenueId, VenueOrderId,
    };
    use of_fix::{encode_logon, encode_message, FixSessionId};
    use std::collections::VecDeque;
    use std::convert::Infallible;

    #[derive(Debug, Default)]
    struct MemoryTransport {
        connected: bool,
        connect_fails: bool,
        sent: Vec<Vec<u8>>,
        inbound: VecDeque<Vec<u8>>,
    }

    impl MemoryTransport {
        fn queue(&mut self, frame: Vec<u8>) {
            self.inbound.push_back(frame);
        }
    }

    #[derive(Debug)]
    struct TestTransportError(&'static str);

    impl fmt::Display for TestTransportError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl Error for TestTransportError {}

    impl FixFrameTransport for MemoryTransport {
        type Error = TestTransportError;

        fn connect(&mut self) -> Result<(), Self::Error> {
            if self.connect_fails {
                return Err(TestTransportError("connect failed"));
            }
            self.connected = true;
            Ok(())
        }

        fn send(&mut self, frame: &[u8]) -> Result<(), Self::Error> {
            if !self.connected {
                return Err(TestTransportError("disconnected"));
            }
            self.sent.push(frame.to_vec());
            Ok(())
        }

        fn poll_receive(&mut self, out: &mut [u8]) -> Result<FixTransportPoll, Self::Error> {
            if !self.connected {
                return Ok(FixTransportPoll::Disconnected);
            }
            let Some(frame) = self.inbound.pop_front() else {
                return Ok(FixTransportPoll::Idle);
            };
            if frame.len() > out.len() {
                return Ok(FixTransportPoll::Frame { len: frame.len() });
            }
            out[..frame.len()].copy_from_slice(&frame);
            Ok(FixTransportPoll::Frame { len: frame.len() })
        }

        fn disconnect(&mut self) -> Result<(), Self::Error> {
            self.connected = false;
            Ok(())
        }
    }

    #[derive(Debug)]
    struct ManualClock {
        next_ns: u64,
    }

    impl ManualClock {
        const fn new() -> Self {
            Self { next_ns: 1 }
        }
    }

    impl FixTimeSource for ManualClock {
        type Error = Infallible;

        fn sample(&mut self, out: &mut [u8]) -> Result<FixTimeSample, Self::Error> {
            let timestamp = b"20260812-12:34:56.123456789";
            out[..timestamp.len()].copy_from_slice(timestamp);
            let now = self.next_ns;
            self.next_ns = self.next_ns.saturating_add(1);
            Ok(FixTimeSample::new(
                now,
                1_786_538_096_123_456_789,
                timestamp.len(),
            ))
        }
    }

    type Adapter =
        FixTransportExecutionAdapter<MemoryTransport, ManualClock, StandardFixExecutionProfile>;

    #[derive(Debug, Default)]
    struct RecordingJournal {
        records: Vec<(u64, FixSentMessageKind, Vec<u8>)>,
    }

    impl FixOutboundJournal for RecordingJournal {
        type Error = Infallible;

        fn record_sent(
            &mut self,
            sequence: u64,
            kind: FixSentMessageKind,
            frame: &[u8],
        ) -> Result<(), Self::Error> {
            self.records.push((sequence, kind, frame.to_vec()));
            Ok(())
        }
    }

    fn fixed<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).expect("fixed")
    }

    fn symbol() -> ExecutionSymbol {
        ExecutionSymbol {
            venue: VenueId::new("XNAS").unwrap(),
            instrument: InstrumentId::new("AAPL").unwrap(),
        }
    }

    fn order(id: &str) -> OrderRequest {
        OrderRequest {
            client_order_id: fixed(id),
            account_id: AccountId::new("ACC1").unwrap(),
            route_id: RouteId::new("FIX-A").unwrap(),
            strategy_id: StrategyId::new("S1").unwrap(),
            symbol: symbol(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Day,
            quantity: OrderQty(100),
            limit_price: OrderPrice(12_345),
            stop_price: OrderPrice(0),
            ts_exchange_ns: 0,
            ts_recv_ns: 1,
        }
    }

    fn config() -> FixLiveAdapterConfig {
        FixLiveAdapterConfig::new(FixSessionConfig::new("FIX.4.4", "SENDER", "TARGET", 30).unwrap())
            .unwrap()
            .with_resend_store(FixResendStoreConfig::new(128, 256 * 1024))
    }

    fn profile() -> StandardFixExecutionProfile {
        StandardFixExecutionProfile::new(
            FixVersion::Fix44,
            FixRequestEncodeConfig::new()
                .with_quantity_scale(1)
                .with_price_scale(100),
            FixReportParseConfig::new(
                AccountId::new("ACC1").unwrap(),
                RouteId::new("FIX-A").unwrap(),
                VenueId::new("XNAS").unwrap(),
            )
            .with_quantity_scale(1)
            .with_price_scale(100),
        )
    }

    fn inbound_header(sequence: u64) -> FixSessionHeader<'static> {
        FixSessionHeader::new(
            b"TARGET",
            b"SENDER",
            sequence,
            b"20260812-12:34:56.123456789",
        )
    }

    fn logon(sequence: u64) -> Vec<u8> {
        let mut raw = Vec::new();
        encode_logon(
            &mut raw,
            FixVersion::Fix44,
            inbound_header(sequence),
            30,
            false,
        )
        .unwrap();
        raw
    }

    fn app_frame(sequence: u64, msg_type: &[u8], fields: &[(FixTag, &[u8])]) -> Vec<u8> {
        let mut all = Vec::with_capacity(fields.len() + 4);
        let mut seq = [0u8; 20];
        let seq_len = write_digits(&mut seq, sequence);
        all.push((FixTag::SENDER_COMP_ID, b"TARGET".as_slice()));
        all.push((FixTag::TARGET_COMP_ID, b"SENDER".as_slice()));
        all.push((FixTag::MSG_SEQ_NUM, &seq[..seq_len]));
        all.push((
            FixTag::SENDING_TIME,
            b"20260812-12:34:56.123456789".as_slice(),
        ));
        all.extend_from_slice(fields);
        let mut raw = Vec::new();
        encode_message(&mut raw, b"FIX.4.4", msg_type, &all).unwrap();
        raw
    }

    fn execution_report(sequence: u64, id: &str, status: &[u8]) -> Vec<u8> {
        app_frame(
            sequence,
            b"8",
            &[
                (FixTag::EXEC_TYPE, status),
                (FixTag::ORD_STATUS, status),
                (FixTag::CL_ORD_ID, id.as_bytes()),
                (FixTag::ORDER_ID, b"VENUE-1"),
                (FixTag::EXEC_ID, b"EXEC-1"),
                (FixTag::SYMBOL, b"AAPL"),
                (FixTag::LEAVES_QTY, b"100"),
                (FixTag::TRANSACT_TIME, b"20260812-12:34:56.123456789"),
            ],
        )
    }

    fn adapter() -> Adapter {
        Adapter::new(
            config(),
            MemoryTransport::default(),
            ManualClock::new(),
            profile(),
        )
        .unwrap()
    }

    fn establish(adapter: &mut Adapter, out: &mut ExecutionEventBuffer) {
        adapter.connect().unwrap();
        assert_eq!(adapter.transport().sent.len(), 1);
        adapter.transport_mut().queue(logon(1));
        assert_eq!(adapter.poll(out).unwrap(), 0);
        assert_eq!(adapter.session_state(), FixSessionState::Ready);
        assert!(adapter.health().connected);
    }

    fn write_digits(out: &mut [u8], mut value: u64) -> usize {
        if value == 0 {
            out[0] = b'0';
            return 1;
        }
        let mut reverse = [0u8; 20];
        let mut len = 0;
        while value > 0 {
            reverse[len] = b'0' + (value % 10) as u8;
            value /= 10;
            len += 1;
        }
        for index in 0..len {
            out[index] = reverse[len - index - 1];
        }
        len
    }

    fn msg_type(frame: &[u8]) -> Vec<u8> {
        let mut scratch = [FixFieldView::empty(); 64];
        parse_message(frame, &mut scratch)
            .unwrap()
            .msg_type()
            .unwrap()
            .to_vec()
    }

    fn sequence(frame: &[u8]) -> u64 {
        let mut scratch = [FixFieldView::empty(); 64];
        let message = parse_message(frame, &mut scratch).unwrap();
        parse_message_sequence(&message).unwrap()
    }

    #[test]
    fn lifecycle_submit_and_execution_report_are_end_to_end() {
        let mut adapter = adapter();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish(&mut adapter, &mut out);

        adapter.submit(&order("C1"), &mut out).unwrap();
        assert_eq!(adapter.transport().sent.len(), 2);
        assert_eq!(msg_type(&adapter.transport().sent[1]), b"D");
        assert_eq!(sequence(&adapter.transport().sent[1]), 2);
        assert_eq!(adapter.sequences().next_outbound(), 3);

        adapter
            .transport_mut()
            .queue(execution_report(2, "C1", b"0"));
        assert_eq!(adapter.poll(&mut out).unwrap(), 1);
        assert_eq!(out.as_slice()[0].exec_type, ExecutionType::Ack);
        assert_eq!(out.as_slice()[0].ts_exchange_ns, 1_786_538_096_123_456_789);
        assert_eq!(adapter.metrics().events_emitted, 1);
    }

    #[test]
    fn cancel_and_amend_reuse_tracked_original_context() {
        let mut adapter = adapter();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish(&mut adapter, &mut out);
        adapter.submit(&order("C1"), &mut out).unwrap();

        let amend = AmendRequest {
            client_order_id: fixed("C2"),
            orig_client_order_id: fixed("C1"),
            venue_order_id: VenueOrderId::new("VENUE-1").unwrap(),
            account_id: AccountId::new("ACC1").unwrap(),
            route_id: RouteId::new("FIX-A").unwrap(),
            symbol: symbol(),
            quantity: OrderQty(80),
            limit_price: OrderPrice(12_300),
            ts_recv_ns: 2,
        };
        adapter.amend(&amend, &mut out).unwrap();
        assert_eq!(msg_type(adapter.transport().sent.last().unwrap()), b"G");

        let cancel = CancelRequest {
            client_order_id: fixed("CXL-1"),
            orig_client_order_id: fixed("C1"),
            venue_order_id: VenueOrderId::new("VENUE-1").unwrap(),
            account_id: AccountId::new("ACC1").unwrap(),
            route_id: RouteId::new("FIX-A").unwrap(),
            symbol: symbol(),
            ts_recv_ns: 3,
        };
        adapter.cancel(&cancel, &mut out).unwrap();
        assert_eq!(msg_type(adapter.transport().sent.last().unwrap()), b"F");
    }

    #[test]
    fn sequence_gap_holds_frame_then_replays_after_missing_message() {
        let mut adapter = adapter();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish(&mut adapter, &mut out);
        adapter.submit(&order("C1"), &mut out).unwrap();

        adapter
            .transport_mut()
            .queue(execution_report(3, "C1", b"0"));
        assert_eq!(adapter.poll(&mut out).unwrap(), 0);
        assert_eq!(adapter.session_state(), FixSessionState::ResendRequested);
        assert_eq!(msg_type(adapter.transport().sent.last().unwrap()), b"2");

        adapter
            .transport_mut()
            .queue(execution_report(2, "C1", b"0"));
        assert_eq!(adapter.poll(&mut out).unwrap(), 2);
        assert_eq!(adapter.session_state(), FixSessionState::Ready);
        assert_eq!(adapter.metrics().gap_frames_held, 1);
    }

    #[test]
    fn peer_resend_replays_application_and_gap_fills_admin() {
        let mut adapter = adapter();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish(&mut adapter, &mut out);
        adapter.submit(&order("C1"), &mut out).unwrap();

        let resend = app_frame(
            2,
            b"2",
            &[(FixTag::BEGIN_SEQ_NO, b"1"), (FixTag::END_SEQ_NO, b"2")],
        );
        adapter.transport_mut().queue(resend);
        adapter.poll(&mut out).unwrap();

        let sent = &adapter.transport().sent;
        assert_eq!(msg_type(&sent[sent.len() - 2]), b"4");
        assert_eq!(sequence(&sent[sent.len() - 2]), 1);
        assert_eq!(msg_type(sent.last().unwrap()), b"D");
        let mut scratch = [FixFieldView::empty(); 64];
        let replay = parse_message(sent.last().unwrap(), &mut scratch).unwrap();
        assert_eq!(replay.get(FixTag::POSS_DUP_FLAG), Some(b"Y".as_slice()));
        assert_eq!(adapter.metrics().replay_frames_sent, 1);
        assert_eq!(adapter.metrics().gap_fill_frames_sent, 1);
    }

    #[test]
    fn recovery_is_async_idempotent_and_standard_for_fix44() {
        let mut adapter = adapter();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish(&mut adapter, &mut out);

        assert_eq!(adapter.recover_open_orders(&mut out).unwrap(), 0);
        assert_eq!(msg_type(adapter.transport().sent.last().unwrap()), b"AF");
        let sent = adapter.transport().sent.len();
        assert_eq!(adapter.recover_open_orders(&mut out).unwrap(), 0);
        assert_eq!(adapter.transport().sent.len(), sent);
        assert_eq!(adapter.metrics().recovery_requests_sent, 1);
    }

    #[test]
    fn bounded_gap_frames_fail_closed() {
        let cfg = config().with_max_pending_gap_frames(1).unwrap();
        let mut adapter = Adapter::new(
            cfg,
            MemoryTransport::default(),
            ManualClock::new(),
            profile(),
        )
        .unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish(&mut adapter, &mut out);
        adapter.submit(&order("C1"), &mut out).unwrap();
        adapter
            .transport_mut()
            .queue(execution_report(4, "C1", b"0"));
        adapter.poll(&mut out).unwrap();
        adapter
            .transport_mut()
            .queue(execution_report(5, "C1", b"0"));
        assert_eq!(adapter.poll(&mut out), Err(ExecutionError::BufferFull));
        assert!(adapter.health().degraded);
    }

    #[test]
    fn connect_failure_can_be_retried_without_rebuilding_adapter() {
        let transport = MemoryTransport {
            connect_fails: true,
            ..MemoryTransport::default()
        };
        let mut adapter = Adapter::new(config(), transport, ManualClock::new(), profile()).unwrap();
        assert!(adapter.connect().is_err());
        assert_eq!(adapter.session_state(), FixSessionState::Stopped);
        adapter.transport_mut().connect_fails = false;
        adapter.connect().unwrap();
        assert_eq!(adapter.session_state(), FixSessionState::LogonSent);
    }

    #[test]
    fn fix44_trade_and_order_status_exec_types_are_supported() {
        assert_eq!(
            super::super::parse_exec_type(b"F"),
            Ok(super::super::FixExecType::Trade)
        );
        assert_eq!(
            super::super::parse_exec_type(b"I"),
            Ok(super::super::FixExecType::Restated)
        );
    }

    #[test]
    fn standard_profile_requires_matching_session_version() {
        let mismatched = StandardFixExecutionProfile::new(
            FixVersion::Fix42,
            FixRequestEncodeConfig::new(),
            FixReportParseConfig::new(fixed("ACC"), fixed("R"), fixed("X")),
        );
        assert!(matches!(
            Adapter::new(
                config(),
                MemoryTransport::default(),
                ManualClock::new(),
                mismatched,
            ),
            Err(FixLiveAdapterConfigError::ProfileVersionMismatch)
        ));
    }

    #[test]
    fn owned_session_identity_matches_injected_configuration() {
        let session = FixSessionId::new(FixVersion::Fix44, b"SENDER", b"TARGET").unwrap();
        let owned = FixOwnedSessionId::from_borrowed(session);
        assert_eq!(owned.sender_comp_id(), b"SENDER");
        assert_eq!(owned.target_comp_id(), b"TARGET");
    }

    #[test]
    fn event_buffer_backpressure_preserves_one_pending_event() {
        let mut adapter = adapter();
        let mut out = ExecutionEventBuffer::with_capacity(1);
        establish(&mut adapter, &mut out);
        adapter.submit(&order("C1"), &mut out).unwrap();
        out.push(ExecutionEvent {
            exec_type: ExecutionType::Status,
            order_status: OrderStatus::New,
            client_order_id: fixed("OTHER"),
            orig_client_order_id: ClientOrderId::empty(),
            venue_order_id: VenueOrderId::empty(),
            execution_id: ExecutionId::empty(),
            account_id: fixed("ACC1"),
            route_id: fixed("FIX-A"),
            symbol: symbol(),
            last_qty: OrderQty(0),
            last_price: OrderPrice(0),
            cumulative_qty: OrderQty(0),
            leaves_qty: OrderQty(0),
            average_price: OrderPrice(0),
            ts_exchange_ns: 0,
            ts_recv_ns: 0,
            reason: of_execution_core::RiskRejectReason::None,
            text: FixedAscii::empty(),
        })
        .unwrap();
        adapter
            .transport_mut()
            .queue(execution_report(2, "C1", b"0"));
        assert_eq!(adapter.poll(&mut out), Err(ExecutionError::BufferFull));
        out.clear();
        assert_eq!(adapter.poll(&mut out).unwrap(), 1);
        assert_eq!(out.as_slice()[0].client_order_id, fixed("C1"));
    }

    #[test]
    fn durable_journal_and_sequence_snapshot_cover_restart_state() {
        let mut adapter = FixTransportExecutionAdapter::with_journal(
            config(),
            MemoryTransport::default(),
            ManualClock::new(),
            profile(),
            RecordingJournal::default(),
        )
        .unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish_journaled(&mut adapter, &mut out);
        adapter.submit(&order("C1"), &mut out).unwrap();

        assert_eq!(adapter.journal().records.len(), 2);
        assert_eq!(adapter.journal().records[0].0, 1);
        assert_eq!(
            adapter.journal().records[0].1,
            FixSentMessageKind::Administrative
        );
        assert_eq!(adapter.journal().records[1].0, 2);
        assert_eq!(
            adapter.journal().records[1].1,
            FixSentMessageKind::Application
        );
        let snapshot = adapter.sequence_snapshot(b"20260812".to_vec()).unwrap();
        assert_eq!(snapshot.next_inbound(), 2);
        assert_eq!(snapshot.next_outbound(), 3);
        assert!(snapshot.validate_checksum());
    }

    #[test]
    fn session_and_business_rejects_degrade_with_diagnostics() {
        let mut adapter = adapter();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish(&mut adapter, &mut out);
        adapter.transport_mut().queue(app_frame(
            2,
            b"3",
            &[
                (FixTag::REF_SEQ_NUM, b"1"),
                (FixTag::REF_MSG_TYPE, b"D"),
                (FixTag::SESSION_REJECT_REASON, b"5"),
                (FixTag::TEXT, b"invalid value"),
            ],
        ));
        adapter.poll(&mut out).unwrap();
        assert_eq!(adapter.metrics().session_rejects, 1);
        assert!(adapter.health().degraded);
        assert!(adapter
            .health()
            .last_error
            .as_deref()
            .unwrap()
            .contains("Session Reject"));

        adapter.acknowledge_protocol_degradation();
        assert!(!adapter.health().degraded);
        adapter.transport_mut().queue(app_frame(
            3,
            b"j",
            &[
                (FixTag::REF_MSG_TYPE, b"D"),
                (FixTag::BUSINESS_REJECT_REASON, b"3"),
                (FixTag::TEXT, b"unsupported message"),
            ],
        ));
        adapter.poll(&mut out).unwrap();
        assert_eq!(adapter.metrics().business_rejects, 1);
        assert!(adapter.health().degraded);
        assert!(adapter
            .health()
            .last_error
            .as_deref()
            .unwrap()
            .contains("BusinessMessageReject"));
    }

    #[test]
    fn sequence_reset_discards_obsolete_held_frames() {
        let mut adapter = adapter();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish(&mut adapter, &mut out);
        adapter.submit(&order("C1"), &mut out).unwrap();
        adapter
            .transport_mut()
            .queue(execution_report(4, "C1", b"0"));
        adapter.poll(&mut out).unwrap();
        assert_eq!(adapter.metrics().gap_frames_held, 1);

        adapter.transport_mut().queue(app_frame(
            2,
            b"4",
            &[(FixTag::GAP_FILL_FLAG, b"Y"), (FixTag::NEW_SEQ_NO, b"5")],
        ));
        adapter.poll(&mut out).unwrap();
        assert_eq!(adapter.sequences().next_inbound(), 5);
        assert_eq!(adapter.metrics().gap_frames_discarded, 1);
        assert_eq!(adapter.held_frames.len(), 0);
    }

    #[test]
    fn restored_working_context_enables_post_restart_cancel() {
        let mut adapter = adapter();
        let context = FixWorkingOrderContext::new(
            OrderSide::Buy,
            OrderType::Limit,
            TimeInForce::Day,
            OrderPrice(0),
        );
        adapter.restore_working_order(fixed("C1"), context).unwrap();
        adapter.restore_working_order(fixed("C1"), context).unwrap();
        assert_eq!(adapter.working_order_count(), 1);
        assert!(adapter
            .restore_working_order(
                fixed("C1"),
                FixWorkingOrderContext::new(
                    OrderSide::Sell,
                    OrderType::Limit,
                    TimeInForce::Day,
                    OrderPrice(0),
                ),
            )
            .is_err());

        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish(&mut adapter, &mut out);
        adapter
            .cancel(
                &CancelRequest {
                    client_order_id: fixed("CXL-1"),
                    orig_client_order_id: fixed("C1"),
                    venue_order_id: VenueOrderId::new("VENUE-1").unwrap(),
                    account_id: fixed("ACC1"),
                    route_id: fixed("FIX-A"),
                    symbol: symbol(),
                    ts_recv_ns: 1,
                },
                &mut out,
            )
            .unwrap();
        assert_eq!(msg_type(adapter.transport().sent.last().unwrap()), b"F");
    }

    #[test]
    fn profile_capabilities_fail_closed_before_sequence_assignment() {
        let capabilities = ExecutionCapabilities {
            market: false,
            amend: false,
            ..profile().capabilities()
        };
        let limited_profile = profile().with_capabilities(capabilities);
        let mut adapter = Adapter::new(
            config(),
            MemoryTransport::default(),
            ManualClock::new(),
            limited_profile,
        )
        .unwrap();
        let mut out = ExecutionEventBuffer::with_capacity(8);
        establish(&mut adapter, &mut out);
        let mut market = order("C1");
        market.order_type = OrderType::Market;
        market.limit_price = OrderPrice(0);
        assert!(adapter.submit(&market, &mut out).is_err());
        assert_eq!(adapter.sequences().next_outbound(), 2);
    }

    fn establish_journaled(
        adapter: &mut FixTransportExecutionAdapter<
            MemoryTransport,
            ManualClock,
            StandardFixExecutionProfile,
            RecordingJournal,
        >,
        out: &mut ExecutionEventBuffer,
    ) {
        adapter.connect().unwrap();
        adapter.transport_mut().queue(logon(1));
        adapter.poll(out).unwrap();
        assert_eq!(adapter.session_state(), FixSessionState::Ready);
    }
}
