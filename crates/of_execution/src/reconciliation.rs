//! Generalized OMS reconciliation across ordered recovery evidence.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use of_execution_core::{AccountId, ClientOrderId, ExecutionSymbol, OrderState, RouteId};

use crate::{PositionReconciliationBuffer, PositionReconciliationItem, ProductionPositionKey};

const RECONCILIATION_SOURCE_COUNT: usize = 7;
const RECONCILIATION_ISSUE_COUNT: usize = 13;

/// Evidence source participating in one OMS reconciliation cycle.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OmsReconciliationSource {
    /// Current single-owner OMS state.
    LocalOms = 0,
    /// State reconstructed from ordered WAL replay.
    WalReplay = 1,
    /// State decoded from the selected checkpoint generation.
    Checkpoint = 2,
    /// Broker/venue open-order recovery response.
    AdapterRecovery = 3,
    /// Independent drop-copy execution evidence.
    DropCopy = 4,
    /// Broker, venue, or clearing position evidence.
    BrokerPositions = 5,
    /// Authoritative local production position ledger.
    PositionLedger = 6,
}

impl OmsReconciliationSource {
    const ALL: [Self; RECONCILIATION_SOURCE_COUNT] = [
        Self::LocalOms,
        Self::WalReplay,
        Self::Checkpoint,
        Self::AdapterRecovery,
        Self::DropCopy,
        Self::BrokerPositions,
        Self::PositionLedger,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

/// Compact required/observed source set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct OmsReconciliationSourceSet(u16);

impl OmsReconciliationSourceSet {
    /// Returns an empty source set.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Returns a set containing every known source.
    pub const fn all() -> Self {
        Self((1_u16 << RECONCILIATION_SOURCE_COUNT) - 1)
    }

    /// Returns a set containing `source`.
    pub const fn one(source: OmsReconciliationSource) -> Self {
        Self(1_u16 << source as u8)
    }

    /// Adds `source` and returns the updated set.
    pub const fn with(mut self, source: OmsReconciliationSource) -> Self {
        self.0 |= 1_u16 << source as u8;
        self
    }

    /// Returns whether `source` is present.
    pub const fn contains(self, source: OmsReconciliationSource) -> bool {
        self.0 & (1_u16 << source as u8) != 0
    }

    /// Returns raw source bits.
    pub const fn bits(self) -> u16 {
        self.0
    }
}

/// Integrity/availability state supplied for one evidence source.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OmsEvidenceStatus {
    /// Source completed successfully and claims a complete snapshot.
    Valid = 1,
    /// Required source was unavailable.
    Missing = 2,
    /// Integrity or checksum validation failed.
    Corrupt = 3,
    /// Source is older than the configured recovery horizon.
    Stale = 4,
    /// Source returned only a partial snapshot.
    Incomplete = 5,
}

/// Source watermark and integrity evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OmsEvidenceWatermark {
    /// Evidence source.
    pub source: OmsReconciliationSource,
    /// Source-reported integrity status.
    pub status: OmsEvidenceStatus,
    /// Highest complete WAL/provider sequence represented, or zero if absent.
    pub sequence: u64,
    /// Source as-of timestamp, or zero if unavailable.
    pub as_of_ns: u64,
    /// Number of order rows represented.
    pub order_count: u32,
    /// Number of position rows represented.
    pub position_count: u32,
}

impl OmsEvidenceWatermark {
    /// Creates one source watermark.
    pub const fn new(
        source: OmsReconciliationSource,
        status: OmsEvidenceStatus,
        sequence: u64,
        as_of_ns: u64,
        order_count: u32,
        position_count: u32,
    ) -> Self {
        Self {
            source,
            status,
            sequence,
            as_of_ns,
            order_count,
            position_count,
        }
    }
}

/// Fine-grained generalized reconciliation classification.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OmsReconciliationIssue {
    /// Compared state matches.
    Matched = 0,
    /// Observed source contains an order absent locally.
    VenueOnly = 1,
    /// Local state contains an order absent from the observed source.
    LocalOnly = 2,
    /// Order lifecycle status differs.
    StatusMismatch = 3,
    /// Order quantity/progress differs.
    QuantityMismatch = 4,
    /// Order average price differs.
    PriceMismatch = 5,
    /// Position-ledger comparison reported any position issue.
    PositionMismatch = 6,
    /// Required source is missing.
    SourceMissing = 7,
    /// Source failed integrity validation.
    SourceCorrupt = 8,
    /// Source watermark is too old.
    SourceStale = 9,
    /// Source is explicitly incomplete.
    SourceIncomplete = 10,
    /// Input repeats a supposedly unique order/position identity.
    DuplicateEvidence = 11,
    /// Compared state differs in identity/direction fields outside other classes.
    Unknown = 12,
}

impl OmsReconciliationIssue {
    const fn index(self) -> usize {
        self as usize
    }
}

/// Host action selected for a generalized finding.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum OmsReconciliationAction {
    /// No action is required.
    #[default]
    Noop = 0,
    /// Keep submissions blocked until evidence is repaired.
    FailClosed = 1,
    /// Accept authoritative observed state and restate local state.
    AcceptObservedTruth = 2,
    /// Cancel an observed-only venue order before resume.
    CancelObservedOrder = 3,
    /// Restate local state from authoritative evidence.
    RestateLocal = 4,
    /// Require explicit operator approval.
    RequireOperatorApproval = 5,
}

impl OmsReconciliationAction {
    /// Returns whether the action blocks new submissions.
    pub const fn blocks_submissions(self) -> bool {
        self as u8 != Self::Noop as u8
    }
}

/// Complete issue-to-action policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OmsReconciliationPolicy {
    actions: [OmsReconciliationAction; RECONCILIATION_ISSUE_COUNT],
}

impl OmsReconciliationPolicy {
    /// Creates a policy that fails closed for every discrepancy.
    pub const fn fail_closed() -> Self {
        let mut actions = [OmsReconciliationAction::FailClosed; RECONCILIATION_ISSUE_COUNT];
        actions[OmsReconciliationIssue::Matched as usize] = OmsReconciliationAction::Noop;
        Self { actions }
    }

    /// Creates a policy requiring operator approval for every discrepancy.
    pub const fn require_operator_approval() -> Self {
        let mut actions =
            [OmsReconciliationAction::RequireOperatorApproval; RECONCILIATION_ISSUE_COUNT];
        actions[OmsReconciliationIssue::Matched as usize] = OmsReconciliationAction::Noop;
        Self { actions }
    }

    /// Sets the action for one issue.
    ///
    /// `Matched` always remains `Noop`.
    pub const fn with_action(
        mut self,
        issue: OmsReconciliationIssue,
        action: OmsReconciliationAction,
    ) -> Self {
        if issue as u8 != OmsReconciliationIssue::Matched as u8 {
            self.actions[issue as usize] = action;
        }
        self
    }

    /// Returns the configured action for `issue`.
    pub const fn action_for(self, issue: OmsReconciliationIssue) -> OmsReconciliationAction {
        self.actions[issue.index()]
    }
}

impl Default for OmsReconciliationPolicy {
    fn default() -> Self {
        Self::fail_closed()
    }
}

/// Reconciliation cycle bounds and required evidence policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OmsReconciliationConfig {
    /// Sources that must report a watermark before `finish`.
    pub required_sources: OmsReconciliationSourceSet,
    /// Maximum allowed lag behind the expected complete sequence; zero disables.
    pub max_sequence_lag: u64,
    /// Maximum source age; zero disables timestamp staleness checks.
    pub stale_after_ns: u64,
}

impl OmsReconciliationConfig {
    /// Creates a fail-closed source requirement.
    pub const fn new(required_sources: OmsReconciliationSourceSet) -> Self {
        Self {
            required_sources,
            max_sequence_lag: 0,
            stale_after_ns: 0,
        }
    }

    /// Sets maximum allowed source sequence lag.
    pub const fn with_max_sequence_lag(mut self, value: u64) -> Self {
        self.max_sequence_lag = value;
        self
    }

    /// Sets maximum allowed source age.
    pub const fn with_stale_after_ns(mut self, value: u64) -> Self {
        self.stale_after_ns = value;
        self
    }
}

/// Entity represented by a reconciliation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OmsReconciliationEntity {
    /// Source-level integrity or availability finding.
    Source(OmsReconciliationSource),
    /// Order-level finding.
    Order(ClientOrderId),
    /// Position-level finding.
    Position(ProductionPositionKey),
}

/// One machine-readable generalized reconciliation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OmsReconciliationFinding {
    /// Evidence source producing the comparison.
    pub source: OmsReconciliationSource,
    /// Compared entity.
    pub entity: OmsReconciliationEntity,
    /// Fine-grained issue.
    pub issue: OmsReconciliationIssue,
    /// Host action selected by policy.
    pub action: OmsReconciliationAction,
    /// Local order state when relevant.
    pub local_order: Option<OrderState>,
    /// Observed order state when relevant.
    pub observed_order: Option<OrderState>,
    /// Existing position-ledger comparison row when relevant.
    pub position: Option<PositionReconciliationItem>,
}

/// Caller-owned bounded generalized reconciliation output.
#[derive(Debug, Clone)]
pub struct OmsReconciliationBuffer {
    findings: Vec<OmsReconciliationFinding>,
    capacity: usize,
}

impl OmsReconciliationBuffer {
    /// Creates an empty fixed-capacity output.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            findings: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Clears findings without releasing allocation.
    pub fn clear(&mut self) {
        self.findings.clear();
    }

    /// Returns current findings.
    pub fn as_slice(&self) -> &[OmsReconciliationFinding] {
        &self.findings
    }

    /// Returns configured maximum findings.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    fn push(&mut self, finding: OmsReconciliationFinding) -> Result<(), OmsReconciliationError> {
        if self.findings.len() >= self.capacity {
            return Err(OmsReconciliationError::BufferFull);
        }
        self.findings.push(finding);
        Ok(())
    }
}

/// Aggregate result for one completed reconciliation cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct OmsReconciliationSummary {
    /// Cycle identifier supplied by the host.
    pub cycle_id: u64,
    /// Expected complete WAL/provider sequence.
    pub expected_sequence: u64,
    /// Number of matching order/position rows.
    pub matched: u32,
    /// Number of discrepancy findings.
    pub mismatched: u32,
    /// Source-level discrepancies.
    pub source_issues: u32,
    /// Order-level discrepancies.
    pub order_issues: u32,
    /// Position-level discrepancies.
    pub position_issues: u32,
    /// Observed sources.
    pub observed_sources: OmsReconciliationSourceSet,
    /// Whether new submissions may resume.
    pub submissions_enabled: bool,
    /// Whether any finding requires operator approval.
    pub operator_approval_required: bool,
    /// Whether any observed-only order must be cancelled.
    pub observed_cancels_required: bool,
    /// Whether local state must be restated.
    pub local_restates_required: bool,
}

/// Generalized reconciliation lifecycle/capacity error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OmsReconciliationError {
    /// No active cycle exists.
    CycleNotStarted,
    /// A cycle is already active.
    CycleAlreadyStarted,
    /// Cycle or expected sequence is zero.
    InvalidCycle,
    /// Source watermark was already supplied.
    DuplicateSource,
    /// Snapshot comparison was attempted before valid source evidence.
    SourceUnavailable,
    /// Caller-owned finding buffer is full.
    BufferFull,
}

impl fmt::Display for OmsReconciliationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CycleNotStarted => "OMS reconciliation cycle is not started",
            Self::CycleAlreadyStarted => "OMS reconciliation cycle is already active",
            Self::InvalidCycle => "OMS reconciliation cycle identifier or sequence is invalid",
            Self::DuplicateSource => "OMS reconciliation source was already observed",
            Self::SourceUnavailable => "OMS reconciliation source is not valid",
            Self::BufferFull => "OMS reconciliation finding buffer is full",
        };
        f.write_str(message)
    }
}

impl Error for OmsReconciliationError {}

/// Single-owner generalized reconciliation cycle coordinator.
#[derive(Debug)]
pub struct OmsReconciliationCoordinator {
    config: OmsReconciliationConfig,
    policy: OmsReconciliationPolicy,
    watermarks: [Option<OmsEvidenceWatermark>; RECONCILIATION_SOURCE_COUNT],
    summary: OmsReconciliationSummary,
    now_ns: u64,
    active: bool,
}

impl OmsReconciliationCoordinator {
    /// Creates an idle coordinator.
    pub const fn new(config: OmsReconciliationConfig, policy: OmsReconciliationPolicy) -> Self {
        Self {
            config,
            policy,
            watermarks: [None; RECONCILIATION_SOURCE_COUNT],
            summary: OmsReconciliationSummary {
                cycle_id: 0,
                expected_sequence: 0,
                matched: 0,
                mismatched: 0,
                source_issues: 0,
                order_issues: 0,
                position_issues: 0,
                observed_sources: OmsReconciliationSourceSet::empty(),
                submissions_enabled: false,
                operator_approval_required: false,
                observed_cancels_required: false,
                local_restates_required: false,
            },
            now_ns: 0,
            active: false,
        }
    }

    /// Begins a fresh cycle and clears `out` while retaining allocation.
    ///
    /// # Errors
    ///
    /// Returns an error for an active cycle or zero cycle/sequence.
    pub fn begin_cycle(
        &mut self,
        cycle_id: u64,
        expected_sequence: u64,
        now_ns: u64,
        out: &mut OmsReconciliationBuffer,
    ) -> Result<(), OmsReconciliationError> {
        if self.active {
            return Err(OmsReconciliationError::CycleAlreadyStarted);
        }
        if cycle_id == 0 || expected_sequence == 0 {
            return Err(OmsReconciliationError::InvalidCycle);
        }
        self.watermarks = [None; RECONCILIATION_SOURCE_COUNT];
        self.summary = OmsReconciliationSummary {
            cycle_id,
            expected_sequence,
            submissions_enabled: true,
            ..OmsReconciliationSummary::default()
        };
        self.now_ns = now_ns;
        self.active = true;
        out.clear();
        Ok(())
    }

    /// Records and validates one source watermark.
    ///
    /// # Errors
    ///
    /// Returns an error without mutation for inactive cycles, duplicate
    /// sources, or insufficient output capacity.
    pub fn observe_source(
        &mut self,
        mut watermark: OmsEvidenceWatermark,
        out: &mut OmsReconciliationBuffer,
    ) -> Result<(), OmsReconciliationError> {
        self.require_active()?;
        let index = watermark.source.index();
        if self.watermarks[index].is_some() {
            return Err(OmsReconciliationError::DuplicateSource);
        }
        if watermark.status == OmsEvidenceStatus::Valid
            && self.config.max_sequence_lag > 0
            && self
                .summary
                .expected_sequence
                .saturating_sub(watermark.sequence)
                > self.config.max_sequence_lag
        {
            watermark.status = OmsEvidenceStatus::Stale;
        }
        if watermark.status == OmsEvidenceStatus::Valid
            && self.config.stale_after_ns > 0
            && (watermark.as_of_ns == 0
                || self.now_ns.saturating_sub(watermark.as_of_ns) > self.config.stale_after_ns)
        {
            watermark.status = OmsEvidenceStatus::Stale;
        }
        let issue = source_issue(watermark.status);
        if let Some(issue) = issue {
            self.push_finding(
                OmsReconciliationFinding {
                    source: watermark.source,
                    entity: OmsReconciliationEntity::Source(watermark.source),
                    issue,
                    action: self.policy.action_for(issue),
                    local_order: None,
                    observed_order: None,
                    position: None,
                },
                out,
            )?;
        }
        self.watermarks[index] = Some(watermark);
        self.summary.observed_sources = self.summary.observed_sources.with(watermark.source);
        Ok(())
    }

    /// Compares local orders with one valid observed source snapshot.
    ///
    /// Output order is deterministic: local input order first, then unmatched
    /// observed input order. Duplicate scoped identities are explicit findings.
    ///
    /// # Errors
    ///
    /// Returns an error for inactive/invalid sources or output exhaustion.
    pub fn reconcile_orders(
        &mut self,
        source: OmsReconciliationSource,
        local: &[OrderState],
        observed: &[OrderState],
        out: &mut OmsReconciliationBuffer,
    ) -> Result<(), OmsReconciliationError> {
        self.require_valid_source(source)?;
        if self.watermarks[source.index()]
            .is_some_and(|watermark| watermark.order_count as usize != observed.len())
        {
            self.mark_source_incomplete(source, out)?;
            return Ok(());
        }
        let mut observed_index = HashMap::with_capacity(observed.len());
        let mut observed_duplicates = HashSet::with_capacity(observed.len());
        for (index, state) in observed.iter().enumerate() {
            let key = ScopedOrderKey::from_state(*state);
            if observed_index.insert(key, index).is_some() {
                observed_duplicates.insert(index);
            }
        }
        observed_index.clear();
        for (index, state) in observed.iter().enumerate() {
            observed_index
                .entry(ScopedOrderKey::from_state(*state))
                .or_insert(index);
        }
        let mut local_seen = HashSet::with_capacity(local.len());
        let mut matched_observed = HashSet::with_capacity(observed.len());
        for local_state in local {
            let key = ScopedOrderKey::from_state(*local_state);
            if !local_seen.insert(key) {
                self.push_order_finding(
                    source,
                    OmsReconciliationIssue::DuplicateEvidence,
                    Some(*local_state),
                    None,
                    out,
                )?;
                continue;
            }
            let Some(index) = observed_index.get(&key).copied() else {
                self.push_order_finding(
                    source,
                    OmsReconciliationIssue::LocalOnly,
                    Some(*local_state),
                    None,
                    out,
                )?;
                continue;
            };
            matched_observed.insert(index);
            let observed_state = observed[index];
            let issue = classify_order_issue(*local_state, observed_state);
            if issue == OmsReconciliationIssue::Matched {
                self.summary.matched = self.summary.matched.saturating_add(1);
            } else {
                self.push_order_finding(
                    source,
                    issue,
                    Some(*local_state),
                    Some(observed_state),
                    out,
                )?;
            }
        }
        for (index, observed_state) in observed.iter().enumerate() {
            if observed_duplicates.contains(&index) {
                self.push_order_finding(
                    source,
                    OmsReconciliationIssue::DuplicateEvidence,
                    None,
                    Some(*observed_state),
                    out,
                )?;
            } else if !matched_observed.contains(&index) {
                self.push_order_finding(
                    source,
                    OmsReconciliationIssue::VenueOnly,
                    None,
                    Some(*observed_state),
                    out,
                )?;
            }
        }
        Ok(())
    }

    /// Adds an existing authoritative position-ledger reconciliation report.
    ///
    /// # Errors
    ///
    /// Returns an error for inactive/invalid sources or output exhaustion.
    pub fn observe_position_report(
        &mut self,
        source: OmsReconciliationSource,
        positions: &PositionReconciliationBuffer,
        out: &mut OmsReconciliationBuffer,
    ) -> Result<(), OmsReconciliationError> {
        self.require_valid_source(source)?;
        if self.watermarks[source.index()].is_some_and(|watermark| {
            watermark.position_count as usize != positions.as_slice().len()
        }) {
            self.mark_source_incomplete(source, out)?;
            return Ok(());
        }
        for item in positions.as_slice() {
            if item.issues.is_empty() {
                self.summary.matched = self.summary.matched.saturating_add(1);
            } else {
                let issue = if item
                    .issues
                    .contains(crate::PositionReconciliationIssueFlags::DUPLICATE_EXTERNAL_KEY)
                {
                    OmsReconciliationIssue::DuplicateEvidence
                } else {
                    OmsReconciliationIssue::PositionMismatch
                };
                self.push_finding(
                    OmsReconciliationFinding {
                        source,
                        entity: OmsReconciliationEntity::Position(item.key),
                        issue,
                        action: self.policy.action_for(issue),
                        local_order: None,
                        observed_order: None,
                        position: Some(*item),
                    },
                    out,
                )?;
            }
        }
        Ok(())
    }

    /// Finishes a cycle, emitting findings for every unobserved required source.
    ///
    /// # Errors
    ///
    /// Returns an error for an inactive cycle or insufficient output capacity.
    pub fn finish(
        &mut self,
        out: &mut OmsReconciliationBuffer,
    ) -> Result<OmsReconciliationSummary, OmsReconciliationError> {
        self.require_active()?;
        for source in OmsReconciliationSource::ALL {
            if self.config.required_sources.contains(source)
                && self.watermarks[source.index()].is_none()
            {
                let issue = OmsReconciliationIssue::SourceMissing;
                self.push_finding(
                    OmsReconciliationFinding {
                        source,
                        entity: OmsReconciliationEntity::Source(source),
                        issue,
                        action: self.policy.action_for(issue),
                        local_order: None,
                        observed_order: None,
                        position: None,
                    },
                    out,
                )?;
            }
        }
        self.active = false;
        Ok(self.summary)
    }

    /// Returns the in-progress summary without finishing the cycle.
    pub const fn summary(&self) -> OmsReconciliationSummary {
        self.summary
    }

    fn push_order_finding(
        &mut self,
        source: OmsReconciliationSource,
        issue: OmsReconciliationIssue,
        local_order: Option<OrderState>,
        observed_order: Option<OrderState>,
        out: &mut OmsReconciliationBuffer,
    ) -> Result<(), OmsReconciliationError> {
        let client_order_id = local_order
            .map(|state| state.client_order_id)
            .or_else(|| observed_order.map(|state| state.client_order_id))
            .unwrap_or_default();
        self.push_finding(
            OmsReconciliationFinding {
                source,
                entity: OmsReconciliationEntity::Order(client_order_id),
                issue,
                action: self.policy.action_for(issue),
                local_order,
                observed_order,
                position: None,
            },
            out,
        )
    }

    fn mark_source_incomplete(
        &mut self,
        source: OmsReconciliationSource,
        out: &mut OmsReconciliationBuffer,
    ) -> Result<(), OmsReconciliationError> {
        let issue = OmsReconciliationIssue::SourceIncomplete;
        self.push_finding(
            OmsReconciliationFinding {
                source,
                entity: OmsReconciliationEntity::Source(source),
                issue,
                action: self.policy.action_for(issue),
                local_order: None,
                observed_order: None,
                position: None,
            },
            out,
        )?;
        if let Some(watermark) = self.watermarks[source.index()].as_mut() {
            watermark.status = OmsEvidenceStatus::Incomplete;
        }
        Ok(())
    }

    fn push_finding(
        &mut self,
        finding: OmsReconciliationFinding,
        out: &mut OmsReconciliationBuffer,
    ) -> Result<(), OmsReconciliationError> {
        out.push(finding)?;
        self.summary.mismatched = self.summary.mismatched.saturating_add(1);
        match finding.entity {
            OmsReconciliationEntity::Source(_) => {
                self.summary.source_issues = self.summary.source_issues.saturating_add(1)
            }
            OmsReconciliationEntity::Order(_) => {
                self.summary.order_issues = self.summary.order_issues.saturating_add(1)
            }
            OmsReconciliationEntity::Position(_) => {
                self.summary.position_issues = self.summary.position_issues.saturating_add(1)
            }
        }
        self.summary.submissions_enabled &= !finding.action.blocks_submissions();
        self.summary.operator_approval_required |=
            finding.action == OmsReconciliationAction::RequireOperatorApproval;
        self.summary.observed_cancels_required |=
            finding.action == OmsReconciliationAction::CancelObservedOrder;
        self.summary.local_restates_required |= matches!(
            finding.action,
            OmsReconciliationAction::AcceptObservedTruth | OmsReconciliationAction::RestateLocal
        );
        Ok(())
    }

    fn require_active(&self) -> Result<(), OmsReconciliationError> {
        if self.active {
            Ok(())
        } else {
            Err(OmsReconciliationError::CycleNotStarted)
        }
    }

    fn require_valid_source(
        &self,
        source: OmsReconciliationSource,
    ) -> Result<(), OmsReconciliationError> {
        self.require_active()?;
        if self.watermarks[source.index()]
            .is_some_and(|item| item.status == OmsEvidenceStatus::Valid)
        {
            Ok(())
        } else {
            Err(OmsReconciliationError::SourceUnavailable)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ScopedOrderKey {
    account_id: AccountId,
    route_id: RouteId,
    symbol: ExecutionSymbol,
    client_order_id: ClientOrderId,
}

impl ScopedOrderKey {
    fn from_state(state: OrderState) -> Self {
        Self {
            account_id: state.account_id,
            route_id: state.route_id,
            symbol: state.symbol,
            client_order_id: state.client_order_id,
        }
    }
}

fn source_issue(status: OmsEvidenceStatus) -> Option<OmsReconciliationIssue> {
    match status {
        OmsEvidenceStatus::Valid => None,
        OmsEvidenceStatus::Missing => Some(OmsReconciliationIssue::SourceMissing),
        OmsEvidenceStatus::Corrupt => Some(OmsReconciliationIssue::SourceCorrupt),
        OmsEvidenceStatus::Stale => Some(OmsReconciliationIssue::SourceStale),
        OmsEvidenceStatus::Incomplete => Some(OmsReconciliationIssue::SourceIncomplete),
    }
}

fn classify_order_issue(local: OrderState, observed: OrderState) -> OmsReconciliationIssue {
    if local.order_qty != observed.order_qty
        || local.cumulative_qty != observed.cumulative_qty
        || local.leaves_qty != observed.leaves_qty
    {
        OmsReconciliationIssue::QuantityMismatch
    } else if local.status != observed.status {
        OmsReconciliationIssue::StatusMismatch
    } else if local.average_price != observed.average_price {
        OmsReconciliationIssue::PriceMismatch
    } else if local.side != observed.side
        || local.last_accepted_client_order_id != observed.last_accepted_client_order_id
        || !local.venue_order_id.is_empty()
            && !observed.venue_order_id.is_empty()
            && local.venue_order_id != observed.venue_order_id
    {
        OmsReconciliationIssue::Unknown
    } else {
        OmsReconciliationIssue::Matched
    }
}

#[cfg(test)]
mod tests {
    use of_execution_core::{
        ExecutionSymbol, FixedAscii, OrderPrice, OrderQty, OrderSide, OrderStatus, VenueOrderId,
    };

    use super::*;
    use crate::{
        reconcile_production_positions, ExternalPositionSnapshot, LedgerCurrency,
        PositionReconciliationTolerance, ProductionPositionLedger, ProductionPositionLedgerConfig,
    };

    fn id<const N: usize>(value: &str) -> FixedAscii<N> {
        FixedAscii::new(value).unwrap()
    }

    fn state(client: &str, qty: i64, status: OrderStatus) -> OrderState {
        OrderState {
            client_order_id: id(client),
            last_accepted_client_order_id: id(client),
            venue_order_id: VenueOrderId::new("venue-1").unwrap(),
            account_id: id("account-a"),
            route_id: id("route-a"),
            symbol: ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            side: OrderSide::Buy,
            status,
            order_qty: OrderQty(qty),
            cumulative_qty: OrderQty(0),
            leaves_qty: OrderQty(qty),
            average_price: OrderPrice(0),
            updated_ns: 100,
        }
    }

    fn watermark(source: OmsReconciliationSource) -> OmsEvidenceWatermark {
        OmsEvidenceWatermark::new(source, OmsEvidenceStatus::Valid, 100, 1_000, 1, 0)
    }

    #[test]
    fn clean_required_sources_and_orders_enable_submissions() {
        let required = OmsReconciliationSourceSet::one(OmsReconciliationSource::LocalOms)
            .with(OmsReconciliationSource::AdapterRecovery);
        let mut coordinator = OmsReconciliationCoordinator::new(
            OmsReconciliationConfig::new(required),
            OmsReconciliationPolicy::fail_closed(),
        );
        let mut out = OmsReconciliationBuffer::with_capacity(4);
        coordinator.begin_cycle(1, 100, 1_000, &mut out).unwrap();
        coordinator
            .observe_source(watermark(OmsReconciliationSource::LocalOms), &mut out)
            .unwrap();
        coordinator
            .observe_source(
                watermark(OmsReconciliationSource::AdapterRecovery),
                &mut out,
            )
            .unwrap();
        let orders = [state("c1", 2, OrderStatus::New)];
        coordinator
            .reconcile_orders(
                OmsReconciliationSource::AdapterRecovery,
                &orders,
                &orders,
                &mut out,
            )
            .unwrap();
        let summary = coordinator.finish(&mut out).unwrap();
        assert!(summary.submissions_enabled);
        assert_eq!(summary.matched, 1);
        assert!(out.as_slice().is_empty());
    }

    #[test]
    fn stale_missing_and_order_mismatches_fail_closed() {
        let required = OmsReconciliationSourceSet::one(OmsReconciliationSource::WalReplay)
            .with(OmsReconciliationSource::Checkpoint);
        let mut coordinator = OmsReconciliationCoordinator::new(
            OmsReconciliationConfig::new(required).with_max_sequence_lag(2),
            OmsReconciliationPolicy::fail_closed(),
        );
        let mut out = OmsReconciliationBuffer::with_capacity(8);
        coordinator.begin_cycle(2, 100, 1_000, &mut out).unwrap();
        coordinator
            .observe_source(
                OmsEvidenceWatermark::new(
                    OmsReconciliationSource::WalReplay,
                    OmsEvidenceStatus::Valid,
                    90,
                    1_000,
                    1,
                    0,
                ),
                &mut out,
            )
            .unwrap();
        let summary = coordinator.finish(&mut out).unwrap();
        assert!(!summary.submissions_enabled);
        assert_eq!(summary.source_issues, 2);
        assert!(out
            .as_slice()
            .iter()
            .any(|item| item.issue == OmsReconciliationIssue::SourceStale));
        assert!(out
            .as_slice()
            .iter()
            .any(|item| item.issue == OmsReconciliationIssue::SourceMissing));
    }

    #[test]
    fn deterministic_order_comparison_reports_duplicates_and_all_mismatch_classes() {
        let source = OmsReconciliationSource::AdapterRecovery;
        let mut coordinator = OmsReconciliationCoordinator::new(
            OmsReconciliationConfig::new(OmsReconciliationSourceSet::one(source)),
            OmsReconciliationPolicy::fail_closed()
                .with_action(
                    OmsReconciliationIssue::VenueOnly,
                    OmsReconciliationAction::CancelObservedOrder,
                )
                .with_action(
                    OmsReconciliationIssue::StatusMismatch,
                    OmsReconciliationAction::RestateLocal,
                ),
        );
        let mut out = OmsReconciliationBuffer::with_capacity(12);
        coordinator.begin_cycle(3, 100, 1_000, &mut out).unwrap();
        coordinator
            .observe_source(
                OmsEvidenceWatermark::new(source, OmsEvidenceStatus::Valid, 100, 1_000, 5, 0),
                &mut out,
            )
            .unwrap();
        let local = [
            state("quantity", 2, OrderStatus::New),
            state("status", 2, OrderStatus::New),
            state("local", 2, OrderStatus::New),
            state("local", 2, OrderStatus::New),
            state("unknown", 2, OrderStatus::New),
        ];
        let mut unknown = state("unknown", 2, OrderStatus::New);
        unknown.side = OrderSide::Sell;
        let observed = [
            state("quantity", 3, OrderStatus::New),
            state("status", 2, OrderStatus::Cancelled),
            state("venue", 2, OrderStatus::New),
            state("venue", 2, OrderStatus::New),
            unknown,
        ];
        coordinator
            .reconcile_orders(source, &local, &observed, &mut out)
            .unwrap();
        let summary = coordinator.finish(&mut out).unwrap();
        assert_eq!(summary.order_issues, 7);
        assert!(summary.observed_cancels_required);
        assert!(summary.local_restates_required);
        assert_eq!(
            out.as_slice()[0].issue,
            OmsReconciliationIssue::QuantityMismatch
        );
        assert_eq!(
            out.as_slice()[1].issue,
            OmsReconciliationIssue::StatusMismatch
        );
        assert!(out
            .as_slice()
            .iter()
            .any(|item| item.issue == OmsReconciliationIssue::Unknown));
    }

    #[test]
    fn watermark_row_count_mismatch_is_incomplete_evidence() {
        let source = OmsReconciliationSource::AdapterRecovery;
        let mut coordinator = OmsReconciliationCoordinator::new(
            OmsReconciliationConfig::new(OmsReconciliationSourceSet::one(source)),
            OmsReconciliationPolicy::fail_closed(),
        );
        let mut out = OmsReconciliationBuffer::with_capacity(2);
        coordinator.begin_cycle(5, 100, 1_000, &mut out).unwrap();
        coordinator
            .observe_source(
                OmsEvidenceWatermark::new(source, OmsEvidenceStatus::Valid, 100, 1_000, 2, 0),
                &mut out,
            )
            .unwrap();
        let one = [state("c1", 2, OrderStatus::New)];
        coordinator
            .reconcile_orders(source, &one, &one, &mut out)
            .unwrap();
        let summary = coordinator.finish(&mut out).unwrap();
        assert_eq!(summary.source_issues, 1);
        assert!(!summary.submissions_enabled);
        assert_eq!(
            out.as_slice()[0].issue,
            OmsReconciliationIssue::SourceIncomplete
        );
    }

    #[test]
    fn position_ledger_report_is_folded_into_general_policy() {
        let source = OmsReconciliationSource::BrokerPositions;
        let ledger = ProductionPositionLedger::new(ProductionPositionLedgerConfig::new(2, 2, 2));
        let key = ProductionPositionKey::new(
            id("account-a"),
            id("strategy-a"),
            ExecutionSymbol::new("XCME", "ESM6").unwrap(),
            LedgerCurrency::new("USD").unwrap(),
        );
        let external = [ExternalPositionSnapshot::new(key, 1, 100, 50, 1_000)];
        let mut position_out = PositionReconciliationBuffer::with_capacity(1);
        reconcile_production_positions(
            &ledger,
            &external,
            PositionReconciliationTolerance::new(0, 0, 0, 0),
            &mut position_out,
        )
        .unwrap();

        let mut coordinator = OmsReconciliationCoordinator::new(
            OmsReconciliationConfig::new(OmsReconciliationSourceSet::one(source)),
            OmsReconciliationPolicy::fail_closed(),
        );
        let mut out = OmsReconciliationBuffer::with_capacity(2);
        coordinator.begin_cycle(6, 100, 1_000, &mut out).unwrap();
        coordinator
            .observe_source(
                OmsEvidenceWatermark::new(source, OmsEvidenceStatus::Valid, 100, 1_000, 0, 1),
                &mut out,
            )
            .unwrap();
        coordinator
            .observe_position_report(source, &position_out, &mut out)
            .unwrap();
        let summary = coordinator.finish(&mut out).unwrap();
        assert_eq!(summary.position_issues, 1);
        assert_eq!(
            out.as_slice()[0].issue,
            OmsReconciliationIssue::PositionMismatch
        );
        assert!(!summary.submissions_enabled);
    }

    #[test]
    fn source_and_buffer_failures_do_not_silently_truncate() {
        let source = OmsReconciliationSource::AdapterRecovery;
        let mut coordinator = OmsReconciliationCoordinator::new(
            OmsReconciliationConfig::new(OmsReconciliationSourceSet::one(source)),
            OmsReconciliationPolicy::fail_closed(),
        );
        let mut out = OmsReconciliationBuffer::with_capacity(0);
        coordinator.begin_cycle(4, 100, 1_000, &mut out).unwrap();
        assert_eq!(
            coordinator.observe_source(
                OmsEvidenceWatermark::new(source, OmsEvidenceStatus::Corrupt, 0, 0, 0, 0),
                &mut out
            ),
            Err(OmsReconciliationError::BufferFull)
        );
        assert_eq!(coordinator.summary().mismatched, 0);
    }
}
