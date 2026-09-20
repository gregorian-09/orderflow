use super::*;

/// Bounded in-memory FIX resend store.
///
/// The store is intentionally storage-neutral. It retains outbound raw frames
/// in memory for fast resend planning, while durable session stores can persist
/// the same frames separately according to their own latency and sync policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixResendStore {
    pub(crate) config: FixResendStoreConfig,
    pub(crate) messages: VecDeque<FixStoredMessage>,
    pub(crate) retained_bytes: usize,
    pub(crate) dropped_messages: u64,
    pub(crate) dropped_bytes: u64,
    pub(crate) evicted_messages: u64,
    pub(crate) evicted_bytes: u64,
    pub(crate) newest_seq_no: Option<u64>,
}

impl Default for FixResendStore {
    fn default() -> Self {
        Self::new(FixResendStoreConfig::default())
    }
}

impl FixResendStore {
    /// Creates an empty resend store.
    pub fn new(config: FixResendStoreConfig) -> Self {
        Self {
            config,
            messages: VecDeque::with_capacity(config.max_messages.min(64)),
            retained_bytes: 0,
            dropped_messages: 0,
            dropped_bytes: 0,
            evicted_messages: 0,
            evicted_bytes: 0,
            newest_seq_no: None,
        }
    }

    /// Returns the configured retention bounds.
    pub const fn config(&self) -> FixResendStoreConfig {
        self.config
    }

    /// Returns retained messages in sequence order.
    pub fn messages(&self) -> impl Iterator<Item = &FixStoredMessage> {
        self.messages.iter()
    }

    /// Returns a retained message by outbound sequence number.
    pub fn get(&self, seq_no: u64) -> Option<&FixStoredMessage> {
        self.messages
            .iter()
            .find(|message| message.seq_no == seq_no)
    }

    /// Records a sent outbound frame.
    ///
    /// Call this only for original outbound sends. Retransmitted `PossDupFlag`
    /// messages should not be recorded as new sends because their original
    /// sequence number may be lower than the latest observed sequence.
    ///
    /// # Errors
    ///
    /// Returns [`FixResendStoreError`] when `seq_no` is zero or lower than the
    /// latest sequence already observed by this store.
    pub fn record_sent(
        &mut self,
        seq_no: u64,
        kind: FixSentMessageKind,
        raw: &[u8],
    ) -> Result<FixResendRetention, FixResendStoreError> {
        if seq_no == 0 {
            return Err(FixResendStoreError::ZeroSeqNo);
        }
        if let Some(latest) = self.newest_seq_no {
            if seq_no <= latest {
                return Err(FixResendStoreError::SequenceRegression {
                    latest,
                    received: seq_no,
                });
            }
        }
        self.newest_seq_no = Some(seq_no);

        if self.config.max_messages == 0
            || self.config.max_bytes == 0
            || raw.len() > self.config.max_bytes
        {
            self.dropped_messages = self.dropped_messages.saturating_add(1);
            self.dropped_bytes = self.dropped_bytes.saturating_add(raw.len() as u64);
            return Ok(FixResendRetention {
                retained: false,
                evicted_messages: 0,
                evicted_bytes: 0,
            });
        }

        self.messages.push_back(FixStoredMessage {
            seq_no,
            kind,
            raw: raw.to_vec(),
        });
        self.retained_bytes = self.retained_bytes.saturating_add(raw.len());

        let mut evicted_messages = 0_u64;
        let mut evicted_bytes = 0_u64;
        while self.messages.len() > self.config.max_messages
            || self.retained_bytes > self.config.max_bytes
        {
            if let Some(evicted) = self.messages.pop_front() {
                let len = evicted.raw.len();
                self.retained_bytes = self.retained_bytes.saturating_sub(len);
                evicted_messages = evicted_messages.saturating_add(1);
                evicted_bytes = evicted_bytes.saturating_add(len as u64);
            } else {
                break;
            }
        }
        self.evicted_messages = self.evicted_messages.saturating_add(evicted_messages);
        self.evicted_bytes = self.evicted_bytes.saturating_add(evicted_bytes);

        Ok(FixResendRetention {
            retained: true,
            evicted_messages,
            evicted_bytes,
        })
    }

    /// Plans replay and gap-fill actions for an inclusive resend range.
    ///
    /// `EndSeqNo(16)=0` is interpreted as "through the newest observed outbound
    /// sequence" for the purpose of bounded planning. The `out` vector is
    /// cleared before actions are appended.
    pub fn plan_resend_range<'a>(
        &'a self,
        range: FixResendRange,
        out: &mut Vec<FixResendAction<'a>>,
    ) -> FixResendPlanSummary {
        out.clear();
        let Some(end_seq_no) = self.resolve_resend_end(range.end_seq_no) else {
            return FixResendPlanSummary {
                replay_messages: 0,
                gap_fill_messages: 0,
                gap_fill_sequences: 0,
            };
        };
        if range.begin_seq_no == 0 || range.begin_seq_no > end_seq_no {
            return FixResendPlanSummary {
                replay_messages: 0,
                gap_fill_messages: 0,
                gap_fill_sequences: 0,
            };
        }

        let mut cursor = range.begin_seq_no;
        let mut replay_messages = 0_u64;
        let mut gap_fill_messages = 0_u64;
        let mut gap_fill_sequences = 0_u64;

        for message in self
            .messages
            .iter()
            .filter(|message| message.seq_no >= range.begin_seq_no && message.seq_no <= end_seq_no)
        {
            if !message.replayable() {
                continue;
            }
            if cursor < message.seq_no {
                push_gap_fill(
                    out,
                    cursor,
                    message.seq_no.saturating_sub(1),
                    &mut gap_fill_messages,
                    &mut gap_fill_sequences,
                );
            }
            out.push(FixResendAction::Replay {
                seq_no: message.seq_no,
                raw: message.raw(),
            });
            replay_messages = replay_messages.saturating_add(1);
            cursor = message.seq_no.saturating_add(1);
        }

        if cursor <= end_seq_no {
            push_gap_fill(
                out,
                cursor,
                end_seq_no,
                &mut gap_fill_messages,
                &mut gap_fill_sequences,
            );
        }

        FixResendPlanSummary {
            replay_messages,
            gap_fill_messages,
            gap_fill_sequences,
        }
    }

    /// Returns resend-store metrics.
    pub fn metrics(&self) -> FixResendStoreMetrics {
        FixResendStoreMetrics {
            retained_messages: self.messages.len() as u64,
            retained_bytes: self.retained_bytes as u64,
            dropped_messages: self.dropped_messages,
            dropped_bytes: self.dropped_bytes,
            evicted_messages: self.evicted_messages,
            evicted_bytes: self.evicted_bytes,
            oldest_seq_no: self.messages.front().map(|message| message.seq_no),
            newest_seq_no: self.newest_seq_no,
        }
    }

    fn resolve_resend_end(&self, requested_end: u64) -> Option<u64> {
        if requested_end == 0 {
            self.newest_seq_no
        } else {
            Some(requested_end)
        }
    }
}

/// Deterministic inbound/outbound FIX sequence tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixSequenceTracker {
    pub(crate) next_inbound: u64,
    pub(crate) next_outbound: u64,
}

impl Default for FixSequenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl FixSequenceTracker {
    /// Creates a tracker with both inbound and outbound sequence numbers set to
    /// `1`.
    pub const fn new() -> Self {
        Self {
            next_inbound: 1,
            next_outbound: 1,
        }
    }

    /// Creates a tracker from persisted next inbound and outbound values.
    ///
    /// Values lower than `1` are clamped to `1` so restored state remains valid.
    pub const fn from_next(next_inbound: u64, next_outbound: u64) -> Self {
        Self {
            next_inbound: clamp_seq_no(next_inbound),
            next_outbound: clamp_seq_no(next_outbound),
        }
    }

    /// Returns the next inbound sequence number expected from the counterparty.
    pub const fn next_inbound(&self) -> u64 {
        self.next_inbound
    }

    /// Returns the next outbound sequence number to assign.
    pub const fn next_outbound(&self) -> u64 {
        self.next_outbound
    }

    /// Assigns and advances the next outbound sequence number.
    pub fn assign_outbound(&mut self) -> u64 {
        let seq_no = self.next_outbound;
        self.next_outbound = self.next_outbound.saturating_add(1);
        seq_no
    }

    /// Observes an inbound message and returns the sequence action.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceError`] when `MsgSeqNum(34)` is missing or zero.
    pub fn observe_message(
        &mut self,
        message: &FixMessageView<'_>,
    ) -> Result<FixSequenceAction, FixSequenceError> {
        let seq_no = message
            .msg_seq_num()
            .ok_or(FixSequenceError::MissingMsgSeqNum)?;
        self.observe_inbound(seq_no, message.poss_dup())
    }

    /// Observes an inbound sequence number.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceError::ZeroSeqNo`] when `seq_no` is zero.
    pub fn observe_inbound(
        &mut self,
        seq_no: u64,
        poss_dup: bool,
    ) -> Result<FixSequenceAction, FixSequenceError> {
        if seq_no == 0 {
            return Err(FixSequenceError::ZeroSeqNo);
        }

        let expected = self.next_inbound;
        if seq_no == expected {
            self.next_inbound = self.next_inbound.saturating_add(1);
            Ok(FixSequenceAction::Accept { seq_no })
        } else if seq_no > expected {
            Ok(FixSequenceAction::Gap {
                expected,
                received: seq_no,
                resend: FixResendRange {
                    begin_seq_no: expected,
                    end_seq_no: seq_no.saturating_sub(1),
                },
            })
        } else if poss_dup {
            Ok(FixSequenceAction::Duplicate { seq_no, expected })
        } else {
            Ok(FixSequenceAction::TooLow {
                expected,
                received: seq_no,
            })
        }
    }

    /// Applies `NewSeqNo(36)` as the next expected inbound sequence number.
    ///
    /// # Errors
    ///
    /// Returns [`FixSequenceError::ZeroSeqNo`] for zero and
    /// [`FixSequenceError::SequenceResetWouldDecrease`] when the reset would
    /// lower the current expected sequence number.
    pub fn apply_sequence_reset(&mut self, new_seq_no: u64) -> Result<(), FixSequenceError> {
        if new_seq_no == 0 {
            return Err(FixSequenceError::ZeroSeqNo);
        }
        if new_seq_no < self.next_inbound {
            return Err(FixSequenceError::SequenceResetWouldDecrease {
                current: self.next_inbound,
                requested: new_seq_no,
            });
        }
        self.next_inbound = new_seq_no;
        Ok(())
    }

    /// Sets the next inbound sequence number from trusted persisted state.
    pub fn set_next_inbound(&mut self, next_inbound: u64) {
        self.next_inbound = clamp_seq_no(next_inbound);
    }

    /// Sets the next outbound sequence number from trusted persisted state.
    pub fn set_next_outbound(&mut self, next_outbound: u64) {
        self.next_outbound = clamp_seq_no(next_outbound);
    }

    /// Creates a persistable snapshot for this tracker.
    ///
    /// # Errors
    ///
    /// Returns [`FixEncodeError`] when `trading_day` contains SOH.
    pub fn snapshot<'a>(
        &self,
        session_id: FixSessionId<'a>,
        trading_day: &'a [u8],
    ) -> Result<FixSequenceSnapshot<'a>, FixEncodeError> {
        FixSequenceSnapshot::new(
            session_id,
            self.next_inbound,
            self.next_outbound,
            trading_day,
        )
    }

    /// Restores tracker counters from a sequence snapshot.
    pub const fn from_snapshot(snapshot: &FixSequenceSnapshot<'_>) -> Self {
        Self::from_next(snapshot.next_inbound(), snapshot.next_outbound())
    }

    /// Resets both inbound and outbound counters to `1`.
    pub fn reset_to_one(&mut self) {
        self.next_inbound = 1;
        self.next_outbound = 1;
    }
}
