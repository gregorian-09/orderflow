use super::*;

/// Open-order reconciliation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationAction {
    /// Local and venue state match.
    Matched,
    /// Venue has an order not present locally.
    VenueOnly,
    /// Local has an order not present at the venue.
    LocalOnly,
    /// Local state should be restated from venue state.
    RestateFromVenue,
}

/// One reconciliation difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconciliationItem {
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Reconciliation action.
    pub action: ReconciliationAction,
    /// Local state when present.
    pub local: Option<OrderState>,
    /// Venue state when present.
    pub venue: Option<OrderState>,
}

/// Open-order reconciliation report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReconciliationReport {
    /// Reconciliation items.
    pub items: Vec<ReconciliationItem>,
}

impl ReconciliationReport {
    /// Returns true when no local/venue differences were found.
    pub fn is_clean(&self) -> bool {
        self.items
            .iter()
            .all(|item| item.action == ReconciliationAction::Matched)
    }
}

/// Fine-grained reconciliation issue classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReconciliationIssueKind {
    /// Local and venue state match.
    Matched,
    /// Venue has an order not present locally.
    VenueOnly,
    /// Local has an order not present at the venue.
    LocalOnly,
    /// Cumulative, leaves, or original quantity differs.
    QuantityMismatch,
    /// Local and venue lifecycle statuses differ.
    StatusMismatch,
    /// Average execution price differs.
    PriceMismatch,
    /// The discrepancy does not fit a more specific category.
    Unknown,
}

/// One detailed reconciliation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReconciliationDetail {
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Fine-grained issue kind.
    pub issue: ReconciliationIssueKind,
    /// Local state when present.
    pub local: Option<OrderState>,
    /// Venue state when present.
    pub venue: Option<OrderState>,
}

/// Detailed venue reconciliation report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct VenueReconciliationReport {
    /// Detailed reconciliation findings.
    pub details: Vec<ReconciliationDetail>,
}

impl VenueReconciliationReport {
    /// Returns true when all details are matched.
    pub fn is_clean(&self) -> bool {
        self.details
            .iter()
            .all(|detail| detail.issue == ReconciliationIssueKind::Matched)
    }

    /// Returns true when at least one detail requires reconciliation action.
    pub fn has_discrepancies(&self) -> bool {
        !self.is_clean()
    }
}

/// Host action selected for a reconciliation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ReconciliationPolicyAction {
    /// No action is required.
    #[default]
    Noop,
    /// Stop recovery or live submissions until a human or host system resolves it.
    FailClosed,
    /// Accept venue state as truth and restate local cache.
    AcceptVenueTruth,
    /// Cancel the venue-only order before resuming submissions.
    CancelVenueOrder,
    /// Restate a venue-only order locally before resuming submissions.
    RestateVenueOrder,
    /// Require explicit operator approval.
    RequireOperatorApproval,
}

impl ReconciliationPolicyAction {
    /// Returns true when the action blocks immediate strategy submissions.
    pub const fn blocks_submissions(self) -> bool {
        !matches!(self, Self::Noop)
    }
}

/// Policy for mapping reconciliation issues to host actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ReconciliationPolicy {
    pub(crate) venue_only: ReconciliationPolicyAction,
    pub(crate) local_only: ReconciliationPolicyAction,
    pub(crate) quantity_mismatch: ReconciliationPolicyAction,
    pub(crate) status_mismatch: ReconciliationPolicyAction,
    pub(crate) price_mismatch: ReconciliationPolicyAction,
    pub(crate) unknown: ReconciliationPolicyAction,
}

impl ReconciliationPolicy {
    /// Creates a fail-closed reconciliation policy.
    pub const fn fail_closed() -> Self {
        Self {
            venue_only: ReconciliationPolicyAction::FailClosed,
            local_only: ReconciliationPolicyAction::FailClosed,
            quantity_mismatch: ReconciliationPolicyAction::FailClosed,
            status_mismatch: ReconciliationPolicyAction::FailClosed,
            price_mismatch: ReconciliationPolicyAction::FailClosed,
            unknown: ReconciliationPolicyAction::FailClosed,
        }
    }

    /// Creates an operator-approval reconciliation policy.
    pub const fn require_operator_approval() -> Self {
        Self {
            venue_only: ReconciliationPolicyAction::RequireOperatorApproval,
            local_only: ReconciliationPolicyAction::RequireOperatorApproval,
            quantity_mismatch: ReconciliationPolicyAction::RequireOperatorApproval,
            status_mismatch: ReconciliationPolicyAction::RequireOperatorApproval,
            price_mismatch: ReconciliationPolicyAction::RequireOperatorApproval,
            unknown: ReconciliationPolicyAction::RequireOperatorApproval,
        }
    }

    /// Sets the action for venue-only orders.
    pub const fn with_venue_only(mut self, action: ReconciliationPolicyAction) -> Self {
        self.venue_only = action;
        self
    }

    /// Sets the action for local-only orders.
    pub const fn with_local_only(mut self, action: ReconciliationPolicyAction) -> Self {
        self.local_only = action;
        self
    }

    /// Sets the action for quantity mismatches.
    pub const fn with_quantity_mismatch(mut self, action: ReconciliationPolicyAction) -> Self {
        self.quantity_mismatch = action;
        self
    }

    /// Sets the action for status mismatches.
    pub const fn with_status_mismatch(mut self, action: ReconciliationPolicyAction) -> Self {
        self.status_mismatch = action;
        self
    }

    /// Sets the action for price mismatches.
    pub const fn with_price_mismatch(mut self, action: ReconciliationPolicyAction) -> Self {
        self.price_mismatch = action;
        self
    }

    /// Sets the action for unknown mismatches.
    pub const fn with_unknown(mut self, action: ReconciliationPolicyAction) -> Self {
        self.unknown = action;
        self
    }

    /// Returns the action for `issue`.
    pub const fn action_for(self, issue: ReconciliationIssueKind) -> ReconciliationPolicyAction {
        match issue {
            ReconciliationIssueKind::Matched => ReconciliationPolicyAction::Noop,
            ReconciliationIssueKind::VenueOnly => self.venue_only,
            ReconciliationIssueKind::LocalOnly => self.local_only,
            ReconciliationIssueKind::QuantityMismatch => self.quantity_mismatch,
            ReconciliationIssueKind::StatusMismatch => self.status_mismatch,
            ReconciliationIssueKind::PriceMismatch => self.price_mismatch,
            ReconciliationIssueKind::Unknown => self.unknown,
        }
    }
}

impl Default for ReconciliationPolicy {
    fn default() -> Self {
        Self::fail_closed()
    }
}

/// Policy decision for one reconciliation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReconciliationPolicyItem {
    /// Client order id.
    pub client_order_id: ClientOrderId,
    /// Issue found during reconciliation.
    pub issue: ReconciliationIssueKind,
    /// Action selected by policy.
    pub action: ReconciliationPolicyAction,
    /// Local state when present.
    pub local: Option<OrderState>,
    /// Venue state when present.
    pub venue: Option<OrderState>,
}

/// Aggregate policy decision for a reconciliation report.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ReconciliationPolicyDecision {
    /// Per-order policy decisions.
    pub items: Vec<ReconciliationPolicyItem>,
    /// True when submissions may resume immediately.
    pub submissions_enabled: bool,
    /// True when at least one action fails closed.
    pub fail_closed: bool,
    /// True when at least one action requires operator approval.
    pub operator_approval_required: bool,
    /// True when at least one venue order should be cancelled.
    pub venue_cancels_required: bool,
    /// True when local state must be restated from venue truth.
    pub local_restates_required: bool,
}

/// Compares local open-order state against venue open-order state with
/// fine-grained discrepancy classification.
pub fn reconcile_open_orders_detailed(
    local: &[OrderState],
    venue: &[OrderState],
) -> VenueReconciliationReport {
    let mut report = VenueReconciliationReport::default();
    let mut venue_by_id: HashMap<ClientOrderId, OrderState> = HashMap::with_capacity(venue.len());
    for state in venue {
        venue_by_id.insert(state.client_order_id, *state);
    }

    for local_state in local {
        match venue_by_id.remove(&local_state.client_order_id) {
            Some(venue_state) => {
                report.details.push(ReconciliationDetail {
                    client_order_id: local_state.client_order_id,
                    issue: classify_reconciliation_issue(local_state, &venue_state),
                    local: Some(*local_state),
                    venue: Some(venue_state),
                });
            }
            None => report.details.push(ReconciliationDetail {
                client_order_id: local_state.client_order_id,
                issue: ReconciliationIssueKind::LocalOnly,
                local: Some(*local_state),
                venue: None,
            }),
        }
    }

    for venue_state in venue_by_id.into_values() {
        report.details.push(ReconciliationDetail {
            client_order_id: venue_state.client_order_id,
            issue: ReconciliationIssueKind::VenueOnly,
            local: None,
            venue: Some(venue_state),
        });
    }
    report
}

/// Evaluates a detailed reconciliation report against a host policy.
pub fn evaluate_reconciliation_policy(
    report: &VenueReconciliationReport,
    policy: ReconciliationPolicy,
) -> ReconciliationPolicyDecision {
    let mut decision = ReconciliationPolicyDecision {
        submissions_enabled: true,
        ..ReconciliationPolicyDecision::default()
    };

    for detail in &report.details {
        let action = policy.action_for(detail.issue);
        decision.fail_closed |= action == ReconciliationPolicyAction::FailClosed;
        decision.operator_approval_required |=
            action == ReconciliationPolicyAction::RequireOperatorApproval;
        decision.venue_cancels_required |= action == ReconciliationPolicyAction::CancelVenueOrder;
        decision.local_restates_required |= matches!(
            action,
            ReconciliationPolicyAction::AcceptVenueTruth
                | ReconciliationPolicyAction::RestateVenueOrder
        );
        if action.blocks_submissions() {
            decision.submissions_enabled = false;
        }
        decision.items.push(ReconciliationPolicyItem {
            client_order_id: detail.client_order_id,
            issue: detail.issue,
            action,
            local: detail.local,
            venue: detail.venue,
        });
    }

    decision
}

/// Reconciles local open-order state against venue state.
pub fn reconcile_open_orders(local: &[OrderState], venue: &[OrderState]) -> ReconciliationReport {
    let mut report = ReconciliationReport::default();
    let mut venue_by_id: HashMap<ClientOrderId, OrderState> = HashMap::with_capacity(venue.len());
    for state in venue {
        venue_by_id.insert(state.client_order_id, *state);
    }

    for local_state in local {
        match venue_by_id.remove(&local_state.client_order_id) {
            Some(venue_state) if venue_state == *local_state => {
                report.items.push(ReconciliationItem {
                    client_order_id: local_state.client_order_id,
                    action: ReconciliationAction::Matched,
                    local: Some(*local_state),
                    venue: Some(venue_state),
                })
            }
            Some(venue_state) => report.items.push(ReconciliationItem {
                client_order_id: local_state.client_order_id,
                action: ReconciliationAction::RestateFromVenue,
                local: Some(*local_state),
                venue: Some(venue_state),
            }),
            None => report.items.push(ReconciliationItem {
                client_order_id: local_state.client_order_id,
                action: ReconciliationAction::LocalOnly,
                local: Some(*local_state),
                venue: None,
            }),
        }
    }

    for venue_state in venue_by_id.into_values() {
        report.items.push(ReconciliationItem {
            client_order_id: venue_state.client_order_id,
            action: ReconciliationAction::VenueOnly,
            local: None,
            venue: Some(venue_state),
        });
    }
    report
}
