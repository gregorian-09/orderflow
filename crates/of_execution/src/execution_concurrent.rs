use super::*;

/// Configuration for the concurrent execution worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConcurrentExecutionConfig {
    /// Bounded command queue capacity.
    pub command_capacity: usize,
    /// Bounded report queue capacity.
    pub report_capacity: usize,
    /// Per-command event buffer capacity used by the worker.
    pub event_buffer_capacity: usize,
}

impl Default for ConcurrentExecutionConfig {
    fn default() -> Self {
        Self {
            command_capacity: 1024,
            report_capacity: 1024,
            event_buffer_capacity: 64,
        }
    }
}

/// Command kind sent to a concurrent execution worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionCommandKind {
    /// Submit a new order.
    Submit,
    /// Cancel an order.
    Cancel,
    /// Amend an order.
    Amend,
    /// Poll adapter events.
    Poll,
    /// Recover open orders.
    RecoverOpenOrders,
    /// Stop the worker.
    Stop,
}

/// Command payload sent to a concurrent execution worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionCommand {
    /// Submit a new order.
    Submit(OrderRequest),
    /// Cancel an existing order.
    Cancel(CancelRequest),
    /// Amend an existing order.
    Amend(AmendRequest),
    /// Poll adapter events.
    Poll,
    /// Recover open orders.
    RecoverOpenOrders,
    /// Stop the worker after prior queued commands are processed.
    Stop,
}

impl ExecutionCommand {
    /// Returns this command's kind.
    pub const fn kind(self) -> ExecutionCommandKind {
        match self {
            Self::Submit(_) => ExecutionCommandKind::Submit,
            Self::Cancel(_) => ExecutionCommandKind::Cancel,
            Self::Amend(_) => ExecutionCommandKind::Amend,
            Self::Poll => ExecutionCommandKind::Poll,
            Self::RecoverOpenOrders => ExecutionCommandKind::RecoverOpenOrders,
            Self::Stop => ExecutionCommandKind::Stop,
        }
    }
}

/// Result report emitted by a concurrent execution worker.
#[derive(Debug, Clone)]
pub struct ExecutionCommandReport {
    /// Monotonic local command sequence assigned by the sender.
    pub sequence: u64,
    /// Command kind.
    pub kind: ExecutionCommandKind,
    /// Command result. The `Ok` value is the number of events in `events`.
    pub result: ExecutionResult<usize>,
    /// Events produced while processing the command.
    pub events: ExecutionEventBuffer,
}

/// Error returned by the concurrent execution wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcurrentExecutionError {
    /// The bounded command queue is full.
    Backpressure,
    /// The worker has stopped or the channel is disconnected.
    Stopped,
    /// The worker thread panicked.
    WorkerPanic,
    /// The synchronous execution engine failed.
    Execution(ExecutionError),
}

impl fmt::Display for ConcurrentExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backpressure => write!(f, "concurrent execution command queue is full"),
            Self::Stopped => write!(f, "concurrent execution worker is stopped"),
            Self::WorkerPanic => write!(f, "concurrent execution worker panicked"),
            Self::Execution(err) => write!(f, "{err}"),
        }
    }
}

impl Error for ConcurrentExecutionError {}

impl From<ExecutionError> for ConcurrentExecutionError {
    fn from(value: ExecutionError) -> Self {
        Self::Execution(value)
    }
}

#[derive(Debug)]
struct WorkerCommand {
    sequence: u64,
    command: ExecutionCommand,
}

/// Cloneable concurrent command handle for an execution worker.
#[derive(Debug, Clone)]
pub struct ExecutionCommandSender {
    tx: SyncSender<WorkerCommand>,
    next_sequence: Arc<AtomicU64>,
}

impl ExecutionCommandSender {
    /// Sends a command, blocking when the bounded command queue is full.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::Stopped`] if the worker has stopped.
    pub fn send(&self, command: ExecutionCommand) -> Result<u64, ConcurrentExecutionError> {
        let sequence = self.next_sequence();
        self.tx
            .send(WorkerCommand { sequence, command })
            .map_err(|_| ConcurrentExecutionError::Stopped)?;
        Ok(sequence)
    }

    /// Attempts to send a command without blocking.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::Backpressure`] when the command queue
    /// is full, or [`ConcurrentExecutionError::Stopped`] when the worker has
    /// stopped.
    pub fn try_send(&self, command: ExecutionCommand) -> Result<u64, ConcurrentExecutionError> {
        let sequence = self.next_sequence();
        match self.tx.try_send(WorkerCommand { sequence, command }) {
            Ok(()) => Ok(sequence),
            Err(TrySendError::Full(_)) => Err(ConcurrentExecutionError::Backpressure),
            Err(TrySendError::Disconnected(_)) => Err(ConcurrentExecutionError::Stopped),
        }
    }

    /// Sends a submit command, blocking on command-queue capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn submit(&self, req: OrderRequest) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::Submit(req))
    }

    /// Attempts to send a submit command without blocking.
    ///
    /// # Errors
    ///
    /// Returns an error on backpressure or stopped worker.
    pub fn try_submit(&self, req: OrderRequest) -> Result<u64, ConcurrentExecutionError> {
        self.try_send(ExecutionCommand::Submit(req))
    }

    /// Sends a cancel command, blocking on command-queue capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn cancel(&self, req: CancelRequest) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::Cancel(req))
    }

    /// Sends an amend command, blocking on command-queue capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn amend(&self, req: AmendRequest) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::Amend(req))
    }

    /// Sends a poll command, blocking on command-queue capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn poll(&self) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::Poll)
    }

    /// Sends a recovery command, blocking on command-queue capacity.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn recover_open_orders(&self) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::RecoverOpenOrders)
    }

    /// Requests worker shutdown after previously queued commands complete.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has already stopped.
    pub fn stop(&self) -> Result<u64, ConcurrentExecutionError> {
        self.send(ExecutionCommand::Stop)
    }

    fn next_sequence(&self) -> u64 {
        self.next_sequence.fetch_add(1, Ordering::Relaxed)
    }
}

/// Concurrent owner for a synchronous execution engine.
pub struct ConcurrentExecutionEngine {
    sender: ExecutionCommandSender,
    reports: Option<Receiver<ExecutionCommandReport>>,
    join: Option<JoinHandle<()>>,
}

impl ConcurrentExecutionEngine {
    /// Spawns a worker thread that owns and serially drives `engine`.
    ///
    /// # Errors
    ///
    /// Returns an error when the engine fails to start or the worker cannot
    /// publish its startup result.
    pub fn spawn<A, R, J>(
        engine: ExecutionEngine<A, R, J>,
        config: ConcurrentExecutionConfig,
    ) -> Result<Self, ConcurrentExecutionError>
    where
        A: ExecutionAdapter + 'static,
        R: RiskCheck + 'static,
        J: ExecutionJournal + 'static,
    {
        let (tx, rx) = mpsc::sync_channel(config.command_capacity);
        let (report_tx, reports) = mpsc::sync_channel(config.report_capacity);
        let (start_tx, start_rx) = mpsc::sync_channel(1);
        let join = thread::Builder::new()
            .name("orderflow-execution-worker".to_string())
            .spawn(move || {
                run_execution_worker(
                    engine,
                    rx,
                    report_tx,
                    start_tx,
                    config.event_buffer_capacity,
                );
            })
            .map_err(|_| ConcurrentExecutionError::Stopped)?;

        match start_rx
            .recv()
            .map_err(|_| ConcurrentExecutionError::Stopped)?
        {
            Ok(()) => Ok(Self {
                sender: ExecutionCommandSender {
                    tx,
                    next_sequence: Arc::new(AtomicU64::new(1)),
                },
                reports: Some(reports),
                join: Some(join),
            }),
            Err(err) => {
                let _ = join.join();
                Err(ConcurrentExecutionError::Execution(err))
            }
        }
    }

    /// Returns a cloneable command sender for concurrent producer threads.
    pub fn command_sender(&self) -> ExecutionCommandSender {
        self.sender.clone()
    }

    /// Sends a command, blocking when the bounded command queue is full.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn send(&self, command: ExecutionCommand) -> Result<u64, ConcurrentExecutionError> {
        self.sender.send(command)
    }

    /// Attempts to send a command without blocking.
    ///
    /// # Errors
    ///
    /// Returns an error on backpressure or stopped worker.
    pub fn try_send(&self, command: ExecutionCommand) -> Result<u64, ConcurrentExecutionError> {
        self.sender.try_send(command)
    }

    /// Receives the next command report.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::Stopped`] if the report channel is
    /// disconnected.
    pub fn recv_report(&self) -> Result<ExecutionCommandReport, ConcurrentExecutionError> {
        self.reports
            .as_ref()
            .ok_or(ConcurrentExecutionError::Stopped)?
            .recv()
            .map_err(|_: RecvError| ConcurrentExecutionError::Stopped)
    }

    /// Attempts to receive the next command report without blocking.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::Backpressure`] when no report is
    /// available, or [`ConcurrentExecutionError::Stopped`] when the worker has
    /// stopped and the report channel is disconnected.
    pub fn try_recv_report(&self) -> Result<ExecutionCommandReport, ConcurrentExecutionError> {
        let reports = self
            .reports
            .as_ref()
            .ok_or(ConcurrentExecutionError::Stopped)?;
        match reports.try_recv() {
            Ok(report) => Ok(report),
            Err(TryRecvError::Empty) => Err(ConcurrentExecutionError::Backpressure),
            Err(TryRecvError::Disconnected) => Err(ConcurrentExecutionError::Stopped),
        }
    }

    /// Receives the next report with a timeout.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::Backpressure`] on timeout.
    pub fn recv_report_timeout(
        &self,
        timeout: Duration,
    ) -> Result<ExecutionCommandReport, ConcurrentExecutionError> {
        let reports = self
            .reports
            .as_ref()
            .ok_or(ConcurrentExecutionError::Stopped)?;
        match reports.recv_timeout(timeout) {
            Ok(report) => Ok(report),
            Err(RecvTimeoutError::Timeout) => Err(ConcurrentExecutionError::Backpressure),
            Err(RecvTimeoutError::Disconnected) => Err(ConcurrentExecutionError::Stopped),
        }
    }

    /// Requests worker shutdown after previously queued commands complete.
    ///
    /// # Errors
    ///
    /// Returns an error when the worker has stopped.
    pub fn request_stop(&self) -> Result<u64, ConcurrentExecutionError> {
        self.sender.stop()
    }

    /// Joins the worker thread.
    ///
    /// # Errors
    ///
    /// Returns [`ConcurrentExecutionError::WorkerPanic`] if the worker panicked.
    pub fn join(&mut self) -> Result<(), ConcurrentExecutionError> {
        if let Some(join) = self.join.take() {
            join.join()
                .map_err(|_| ConcurrentExecutionError::WorkerPanic)?;
        }
        Ok(())
    }
}

impl Drop for ConcurrentExecutionEngine {
    fn drop(&mut self) {
        let _ = self.sender.try_send(ExecutionCommand::Stop);
        drop(self.reports.take());
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn run_execution_worker<A, R, J>(
    mut engine: ExecutionEngine<A, R, J>,
    rx: Receiver<WorkerCommand>,
    report_tx: SyncSender<ExecutionCommandReport>,
    start_tx: SyncSender<ExecutionResult<()>>,
    event_buffer_capacity: usize,
) where
    A: ExecutionAdapter,
    R: RiskCheck,
    J: ExecutionJournal,
{
    let start = engine.start();
    let started = start.is_ok();
    if start_tx.send(start).is_err() || !started {
        return;
    }

    let mut events = ExecutionEventBuffer::with_capacity(event_buffer_capacity);
    while let Ok(worker_command) = rx.recv() {
        events.clear();
        let kind = worker_command.command.kind();
        let result = match worker_command.command {
            ExecutionCommand::Submit(req) => engine.submit(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Cancel(req) => engine.cancel(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Amend(req) => engine.amend(req, &mut events).map(|()| events.len()),
            ExecutionCommand::Poll => engine.poll(&mut events),
            ExecutionCommand::RecoverOpenOrders => engine.recover_open_orders(&mut events),
            ExecutionCommand::Stop => Ok(0),
        };
        let report = ExecutionCommandReport {
            sequence: worker_command.sequence,
            kind,
            result,
            events: events.clone(),
        };
        if report_tx.send(report).is_err() || kind == ExecutionCommandKind::Stop {
            break;
        }
    }
}
