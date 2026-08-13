use std::collections::HashMap;

/// Tracks the last accepted market-data sequence for each CQG contract.
#[derive(Debug, Clone, Default)]
pub struct BookSequencer {
    last_sequence: HashMap<i64, u64>,
}

impl BookSequencer {
    /// Classifies a sequence and records it when it is usable for progression.
    pub fn apply_sequence(&mut self, contract_id: i64, sequence: u64) -> SequenceStatus {
        let prev = self.last_sequence.get(&contract_id).copied().unwrap_or(0);
        let status = if prev == 0 || sequence == prev + 1 {
            SequenceStatus::Ok
        } else if sequence <= prev {
            SequenceStatus::OutOfOrder
        } else {
            SequenceStatus::Gap {
                expected: prev + 1,
                actual: sequence,
            }
        };
        if matches!(status, SequenceStatus::Ok | SequenceStatus::Gap { .. }) {
            self.last_sequence.insert(contract_id, sequence);
        }
        status
    }
}

/// Result of applying a market-data sequence to a contract stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceStatus {
    Ok,
    OutOfOrder,
    Gap { expected: u64, actual: u64 },
}
