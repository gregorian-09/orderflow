use super::*;

/// Monotonic command identifier assigned before a command enters an OMS queue.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CommandId(pub u64);

/// Request identifier used to correlate strategy intent, command queue entry,
/// and downstream execution reports.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RequestId(pub FixedAscii<40>);

impl RequestId {
    /// Creates a request id from ASCII text.
    ///
    /// # Errors
    ///
    /// Returns an error when the id exceeds capacity or is not ASCII.
    pub fn new(value: &str) -> Result<Self, of_execution_core::ExecutionCoreError> {
        Ok(Self(FixedAscii::new(value)?))
    }

    /// Returns the request id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Lock-free monotonic command id generator.
#[derive(Debug, Default)]
pub struct CommandIdGenerator {
    pub(crate) next: AtomicU64,
}

impl CommandIdGenerator {
    /// Creates a generator starting at `first`.
    pub const fn new(first: u64) -> Self {
        Self {
            next: AtomicU64::new(first),
        }
    }

    /// Returns the next command id.
    pub fn next(&self) -> CommandId {
        CommandId(self.next.fetch_add(1, Ordering::Relaxed))
    }
}

/// Correlation envelope for an execution command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandCorrelation {
    /// Monotonic command id.
    pub command_id: CommandId,
    /// Optional strategy/request id.
    pub request_id: RequestId,
    /// Client order id associated with the command.
    pub client_order_id: ClientOrderId,
    /// Command kind.
    pub kind: ExecutionCommandKind,
}

impl CommandCorrelation {
    /// Creates a command correlation envelope.
    pub const fn new(
        command_id: CommandId,
        request_id: RequestId,
        client_order_id: ClientOrderId,
        kind: ExecutionCommandKind,
    ) -> Self {
        Self {
            command_id,
            request_id,
            client_order_id,
            kind,
        }
    }
}

/// Event subscriber for execution fanout.
#[derive(Debug)]
pub struct ExecutionEventSubscriber {
    pub(crate) receiver: Receiver<ExecutionEvent>,
}

impl ExecutionEventSubscriber {
    /// Receives the next execution event.
    ///
    /// # Errors
    ///
    /// Returns an error when the fanout source has been dropped.
    pub fn recv(&self) -> Result<ExecutionEvent, ExecutionError> {
        self.receiver
            .recv()
            .map_err(|_| ExecutionError::Adapter("execution event fanout closed".to_string()))
    }

    /// Attempts to receive one execution event without blocking.
    pub fn try_recv(&self) -> Option<ExecutionEvent> {
        self.receiver.try_recv().ok()
    }
}

#[derive(Debug)]
pub(crate) struct FanoutInner {
    pub(crate) subscribers: Vec<SyncSender<ExecutionEvent>>,
    pub(crate) dropped_events: u64,
}

/// Bounded execution-event fanout for multiple consumers.
#[derive(Debug, Clone)]
pub struct ExecutionEventFanout {
    pub(crate) inner: Arc<Mutex<FanoutInner>>,
    pub(crate) subscriber_capacity: usize,
}

impl ExecutionEventFanout {
    /// Creates an empty fanout with per-subscriber queue capacity.
    pub fn new(subscriber_capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(FanoutInner {
                subscribers: Vec::new(),
                dropped_events: 0,
            })),
            subscriber_capacity,
        }
    }

    /// Adds a subscriber.
    pub fn subscribe(&self) -> ExecutionEventSubscriber {
        let (tx, receiver) = mpsc::sync_channel(self.subscriber_capacity);
        let mut inner = self.inner.lock().expect("fanout mutex");
        inner.subscribers.push(tx);
        ExecutionEventSubscriber { receiver }
    }

    /// Publishes an event to all active subscribers.
    pub fn publish(&self, event: ExecutionEvent) {
        let mut inner = self.inner.lock().expect("fanout mutex");
        let mut dropped = 0_u64;
        inner
            .subscribers
            .retain(|subscriber| match subscriber.try_send(event) {
                Ok(()) => true,
                Err(TrySendError::Full(_)) => {
                    dropped = dropped.saturating_add(1);
                    true
                }
                Err(TrySendError::Disconnected(_)) => false,
            });
        inner.dropped_events = inner.dropped_events.saturating_add(dropped);
    }

    /// Publishes all events in `events`.
    pub fn publish_buffer(&self, events: &ExecutionEventBuffer) {
        for event in events.as_slice() {
            self.publish(*event);
        }
    }

    /// Returns the number of event deliveries dropped because a subscriber
    /// queue was full.
    pub fn dropped_events(&self) -> u64 {
        self.inner.lock().expect("fanout mutex").dropped_events
    }

    /// Returns current active subscriber count.
    pub fn subscriber_count(&self) -> usize {
        self.inner.lock().expect("fanout mutex").subscribers.len()
    }
}

/// Venue adapter/session lifecycle state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExecutionAdapterState {
    /// Transport is disconnected.
    #[default]
    Disconnected = 0,
    /// Transport connect is in progress.
    Connecting = 1,
    /// Protocol logon/authentication is in progress.
    LogonPending = 2,
    /// Session is ready for order flow.
    Ready = 3,
    /// Session is recovering state after reconnect.
    Recovering = 4,
    /// Session is connected but degraded.
    Degraded = 5,
    /// Session is stopped intentionally.
    Stopped = 6,
}

/// Execution lifecycle snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecutionLifecycleSnapshot {
    /// Current adapter/session state.
    pub state: ExecutionAdapterState,
    /// Monotonic lifecycle sequence.
    pub sequence: u64,
    /// Last transition timestamp in nanoseconds.
    pub updated_ns: u64,
    /// Last lifecycle error.
    pub last_error: Option<String>,
}

/// Mutable lifecycle tracker for adapters and supervisors.
#[derive(Debug, Clone, Default)]
pub struct ExecutionLifecycle {
    pub(crate) snapshot: ExecutionLifecycleSnapshot,
}

impl ExecutionLifecycle {
    /// Creates a disconnected lifecycle tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Transitions to `state`.
    pub fn transition(
        &mut self,
        state: ExecutionAdapterState,
        updated_ns: u64,
        last_error: Option<String>,
    ) -> ExecutionLifecycleSnapshot {
        self.snapshot.state = state;
        self.snapshot.sequence = self.snapshot.sequence.saturating_add(1);
        self.snapshot.updated_ns = updated_ns;
        self.snapshot.last_error = last_error;
        self.snapshot.clone()
    }

    /// Returns current lifecycle snapshot.
    pub fn snapshot(&self) -> ExecutionLifecycleSnapshot {
        self.snapshot.clone()
    }
}
