use super::*;

/// Current schema version for signal checkpoints.
pub const SIGNAL_CHECKPOINT_SCHEMA_VERSION: u16 = 1;

/// Versioned signal checkpoint metadata and payload.
///
/// The core type is intentionally generic: the signal crate validates stable
/// identity/config metadata, while individual signal implementations own their
/// payload encoding.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SignalCheckpoint {
    /// Checkpoint schema version.
    pub schema_version: u16,
    /// Stable signal module id.
    pub module_id: String,
    /// Signal implementation or descriptor version.
    pub signal_version: String,
    /// Host-defined signal configuration hash.
    pub config_hash: u64,
    /// Optional symbol associated with the checkpoint.
    pub symbol: Option<SymbolId>,
    /// Last emitted state captured in the checkpoint.
    pub state: SignalState,
    /// Last emitted confidence in basis points.
    pub confidence_bps: u16,
    /// Last emitted quality flags.
    pub quality_flags: u32,
    /// Last emitted reason text.
    pub reason: String,
    /// Optional lifecycle state captured by the host.
    pub lifecycle_state: Option<SignalLifecycleState>,
    /// Optional calibration artifact id used by the signal.
    pub calibration_id: Option<u64>,
    /// Checkpoint creation timestamp in nanoseconds.
    pub created_at_ns: u64,
    /// Last signal update timestamp represented by this checkpoint.
    pub last_update_ns: u64,
    /// Opaque signal-owned payload bytes.
    pub payload: Vec<u8>,
}

impl SignalCheckpoint {
    /// Creates checkpoint metadata for a signal state.
    pub fn new(
        module_id: impl Into<String>,
        signal_version: impl Into<String>,
        state: SignalState,
    ) -> Self {
        Self {
            schema_version: SIGNAL_CHECKPOINT_SCHEMA_VERSION,
            module_id: module_id.into(),
            signal_version: signal_version.into(),
            config_hash: 0,
            symbol: None,
            state,
            confidence_bps: 0,
            quality_flags: 0,
            reason: String::new(),
            lifecycle_state: None,
            calibration_id: None,
            created_at_ns: 0,
            last_update_ns: 0,
            payload: Vec::new(),
        }
    }

    /// Creates checkpoint metadata from a signal snapshot.
    pub fn from_snapshot(snapshot: &SignalSnapshot, signal_version: impl Into<String>) -> Self {
        Self {
            schema_version: SIGNAL_CHECKPOINT_SCHEMA_VERSION,
            module_id: snapshot.module_id.to_string(),
            signal_version: signal_version.into(),
            config_hash: 0,
            symbol: None,
            state: snapshot.state,
            confidence_bps: snapshot.confidence_bps.min(10_000),
            quality_flags: snapshot.quality_flags,
            reason: snapshot.reason.clone(),
            lifecycle_state: None,
            calibration_id: None,
            created_at_ns: 0,
            last_update_ns: 0,
            payload: Vec::new(),
        }
    }

    /// Returns this checkpoint with a configuration hash.
    pub const fn with_config_hash(mut self, config_hash: u64) -> Self {
        self.config_hash = config_hash;
        self
    }

    /// Returns this checkpoint with a symbol.
    pub fn with_symbol(mut self, symbol: SymbolId) -> Self {
        self.symbol = Some(symbol);
        self
    }

    /// Returns this checkpoint with lifecycle state.
    pub const fn with_lifecycle_state(mut self, lifecycle_state: SignalLifecycleState) -> Self {
        self.lifecycle_state = Some(lifecycle_state);
        self
    }

    /// Returns this checkpoint with calibration id.
    pub const fn with_calibration_id(mut self, calibration_id: u64) -> Self {
        self.calibration_id = Some(calibration_id);
        self
    }

    /// Returns this checkpoint with creation and last-update timestamps.
    pub const fn with_timestamps(mut self, created_at_ns: u64, last_update_ns: u64) -> Self {
        self.created_at_ns = created_at_ns;
        self.last_update_ns = last_update_ns;
        self
    }

    /// Returns this checkpoint with opaque payload bytes.
    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self
    }
}

/// Restore-time validation policy for a signal checkpoint.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCheckpointRestorePolicy {
    /// Expected signal module id.
    pub expected_module_id: Option<String>,
    /// Expected signal implementation/descriptor version.
    pub expected_signal_version: Option<String>,
    /// Expected config hash.
    pub expected_config_hash: Option<u64>,
    /// Expected symbol.
    pub expected_symbol: Option<SymbolId>,
    /// Minimum accepted checkpoint schema version.
    pub min_schema_version: u16,
    /// Maximum accepted checkpoint schema version.
    pub max_schema_version: u16,
    /// Minimum accepted last-update timestamp.
    pub min_last_update_ns: Option<u64>,
    /// Expected calibration artifact id.
    pub expected_calibration_id: Option<u64>,
}

impl SignalCheckpointRestorePolicy {
    /// Creates a restore policy that accepts the current checkpoint schema.
    pub const fn new() -> Self {
        Self {
            expected_module_id: None,
            expected_signal_version: None,
            expected_config_hash: None,
            expected_symbol: None,
            min_schema_version: SIGNAL_CHECKPOINT_SCHEMA_VERSION,
            max_schema_version: SIGNAL_CHECKPOINT_SCHEMA_VERSION,
            min_last_update_ns: None,
            expected_calibration_id: None,
        }
    }

    /// Returns this policy with expected signal identity.
    pub fn with_signal(mut self, module_id: impl Into<String>, version: impl Into<String>) -> Self {
        self.expected_module_id = Some(module_id.into());
        self.expected_signal_version = Some(version.into());
        self
    }

    /// Returns this policy with an expected config hash.
    pub const fn with_config_hash(mut self, config_hash: u64) -> Self {
        self.expected_config_hash = Some(config_hash);
        self
    }

    /// Returns this policy with an expected symbol.
    pub fn with_symbol(mut self, symbol: SymbolId) -> Self {
        self.expected_symbol = Some(symbol);
        self
    }

    /// Returns this policy with an accepted schema version range.
    pub const fn with_schema_range(
        mut self,
        min_schema_version: u16,
        max_schema_version: u16,
    ) -> Self {
        self.min_schema_version = min_schema_version;
        self.max_schema_version = max_schema_version;
        self
    }

    /// Returns this policy with a minimum last-update timestamp.
    pub const fn with_min_last_update_ns(mut self, min_last_update_ns: u64) -> Self {
        self.min_last_update_ns = Some(min_last_update_ns);
        self
    }

    /// Returns this policy with an expected calibration id.
    pub const fn with_calibration_id(mut self, calibration_id: u64) -> Self {
        self.expected_calibration_id = Some(calibration_id);
        self
    }
}

impl Default for SignalCheckpointRestorePolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// One restore validation issue for a signal checkpoint.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalCheckpointValidationIssue {
    /// Checkpoint schema version was outside the accepted range.
    SchemaVersionOutOfRange {
        /// Minimum accepted schema version.
        min: u16,
        /// Maximum accepted schema version.
        max: u16,
        /// Actual checkpoint schema version.
        actual: u16,
    },
    /// Signal module id did not match.
    ModuleIdMismatch {
        /// Expected module id.
        expected: String,
        /// Actual module id.
        actual: String,
    },
    /// Signal version did not match.
    SignalVersionMismatch {
        /// Expected signal version.
        expected: String,
        /// Actual signal version.
        actual: String,
    },
    /// Signal config hash did not match.
    ConfigHashMismatch {
        /// Expected config hash.
        expected: u64,
        /// Actual config hash.
        actual: u64,
    },
    /// Symbol did not match.
    SymbolMismatch {
        /// Expected symbol.
        expected: SymbolId,
        /// Actual symbol.
        actual: Option<SymbolId>,
    },
    /// Calibration id did not match.
    CalibrationMismatch {
        /// Expected calibration id.
        expected: u64,
        /// Actual calibration id.
        actual: Option<u64>,
    },
    /// Checkpoint timestamp was older than allowed.
    NonMonotonicTimestamp {
        /// Minimum accepted last-update timestamp.
        min_last_update_ns: u64,
        /// Actual checkpoint last-update timestamp.
        checkpoint_last_update_ns: u64,
    },
}

/// Restore validation report for a signal checkpoint.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCheckpointValidationReport {
    /// Whether restore validation passed.
    pub valid: bool,
    /// Validation issues.
    pub issues: Vec<SignalCheckpointValidationIssue>,
}

impl SignalCheckpointValidationReport {
    /// Returns `true` when validation failed.
    pub const fn has_errors(&self) -> bool {
        !self.valid
    }
}

/// Error returned by checkpoint-aware signal restore operations.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalCheckpointRestoreError {
    /// Checkpoint metadata failed restore validation.
    InvalidCheckpoint(SignalCheckpointValidationReport),
    /// Checkpoint payload is not supported by this signal implementation.
    UnsupportedPayload,
}

/// Optional extension trait for signals that support checkpoint restore.
///
/// This is intentionally separate from [`SignalModule`] so existing downstream
/// signal implementations remain source-compatible.
pub trait CheckpointableSignal: SignalModule {
    /// Returns a checkpoint for the current signal state.
    fn checkpoint(&self) -> SignalCheckpoint;

    /// Restores signal state from a previously validated checkpoint.
    fn restore_checkpoint(
        &mut self,
        checkpoint: &SignalCheckpoint,
    ) -> Result<(), SignalCheckpointRestoreError>;
}

/// Validates checkpoint metadata against a restore policy.
pub fn validate_signal_checkpoint_restore(
    checkpoint: &SignalCheckpoint,
    policy: &SignalCheckpointRestorePolicy,
) -> SignalCheckpointValidationReport {
    let mut issues = Vec::new();

    if checkpoint.schema_version < policy.min_schema_version
        || checkpoint.schema_version > policy.max_schema_version
    {
        issues.push(SignalCheckpointValidationIssue::SchemaVersionOutOfRange {
            min: policy.min_schema_version,
            max: policy.max_schema_version,
            actual: checkpoint.schema_version,
        });
    }

    if let Some(expected) = &policy.expected_module_id {
        if checkpoint.module_id != *expected {
            issues.push(SignalCheckpointValidationIssue::ModuleIdMismatch {
                expected: expected.clone(),
                actual: checkpoint.module_id.clone(),
            });
        }
    }

    if let Some(expected) = &policy.expected_signal_version {
        if checkpoint.signal_version != *expected {
            issues.push(SignalCheckpointValidationIssue::SignalVersionMismatch {
                expected: expected.clone(),
                actual: checkpoint.signal_version.clone(),
            });
        }
    }

    if let Some(expected) = policy.expected_config_hash {
        if checkpoint.config_hash != expected {
            issues.push(SignalCheckpointValidationIssue::ConfigHashMismatch {
                expected,
                actual: checkpoint.config_hash,
            });
        }
    }

    if let Some(expected) = &policy.expected_symbol {
        if checkpoint.symbol.as_ref() != Some(expected) {
            issues.push(SignalCheckpointValidationIssue::SymbolMismatch {
                expected: expected.clone(),
                actual: checkpoint.symbol.clone(),
            });
        }
    }

    if let Some(expected) = policy.expected_calibration_id {
        if checkpoint.calibration_id != Some(expected) {
            issues.push(SignalCheckpointValidationIssue::CalibrationMismatch {
                expected,
                actual: checkpoint.calibration_id,
            });
        }
    }

    if let Some(min_last_update_ns) = policy.min_last_update_ns {
        if checkpoint.last_update_ns < min_last_update_ns {
            issues.push(SignalCheckpointValidationIssue::NonMonotonicTimestamp {
                min_last_update_ns,
                checkpoint_last_update_ns: checkpoint.last_update_ns,
            });
        }
    }

    SignalCheckpointValidationReport {
        valid: issues.is_empty(),
        issues,
    }
}
