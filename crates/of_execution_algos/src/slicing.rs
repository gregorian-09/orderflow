use super::*;

/// Deterministic TWAP slice planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwapSlicePlanner {
    slice_interval_ns: u64,
}

impl TwapSlicePlanner {
    /// Creates a TWAP planner.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidSliceInterval`] when the interval is zero.
    pub const fn try_new(slice_interval_ns: u64) -> Result<Self, AlgoError> {
        if slice_interval_ns == 0 {
            return Err(AlgoError::InvalidSliceInterval);
        }
        Ok(Self { slice_interval_ns })
    }

    /// Creates a TWAP planner, panicking when the interval is zero.
    pub const fn new(slice_interval_ns: u64) -> Self {
        assert!(slice_interval_ns > 0, "slice interval must be positive");
        Self { slice_interval_ns }
    }

    /// Returns the configured slice interval.
    pub const fn slice_interval_ns(&self) -> u64 {
        self.slice_interval_ns
    }

    /// Plans one due child slice for `now_ns`.
    ///
    /// The planner uses only caller-provided timestamps and integer arithmetic.
    /// It returns `Ok(None)` when no additional child quantity is due.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when the parent/progress state is invalid or the
    /// generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_due_slice(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<Option<ChildOrderPlan>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if progress.released_qty().0 > parent.total_qty().0
            || progress.completed_qty().0 > parent.total_qty().0
        {
            return Err(AlgoError::InvalidProgress);
        }
        if now_ns < parent.start_ns() || progress.is_complete() {
            return Ok(None);
        }

        let total_slices = div_ceil_u64(
            parent.end_ns().saturating_sub(parent.start_ns()),
            self.slice_interval_ns,
        )
        .max(1);
        let elapsed_ns = now_ns
            .min(parent.end_ns())
            .saturating_sub(parent.start_ns());
        let due_slices = (elapsed_ns / self.slice_interval_ns)
            .saturating_add(1)
            .min(total_slices);
        let desired = div_ceil_i128(
            i128::from(parent.total_qty().0) * i128::from(due_slices),
            i128::from(total_slices),
        );
        let desired_qty = i64::try_from(desired).unwrap_or(i64::MAX);
        let due_qty = desired_qty.saturating_sub(progress.released_qty().0);
        if due_qty <= 0 {
            return Ok(None);
        }
        let leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        let mut child_qty = due_qty.min(parent.max_clip().0).min(leaves);
        let final_slice = progress.released_qty().0.saturating_add(child_qty)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if child_qty < parent.min_clip().0 && !final_slice {
            return Ok(None);
        }
        if child_qty <= 0 {
            return Ok(None);
        }
        child_qty = child_qty.min(
            parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0),
        );
        let request = parent.build_order_request(client_order_id, OrderQty(child_qty), ts_recv_ns);
        Ok(Some(ChildOrderPlan::new(
            child_id,
            parent.id(),
            request,
            now_ns,
        )?))
    }
}

/// Deterministic percentage-of-volume child slice planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PovSlicePlanner {
    target_participation_bps: u16,
    max_participation_bps: u16,
}

impl PovSlicePlanner {
    /// Creates a POV planner.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidParticipationRate`] when target or cap is
    /// zero, or when target exceeds cap.
    pub const fn try_new(
        target_participation_bps: u16,
        max_participation_bps: u16,
    ) -> Result<Self, AlgoError> {
        if target_participation_bps == 0
            || max_participation_bps == 0
            || target_participation_bps > max_participation_bps
        {
            return Err(AlgoError::InvalidParticipationRate);
        }
        Ok(Self {
            target_participation_bps,
            max_participation_bps,
        })
    }

    /// Creates a POV planner, panicking when rates are invalid.
    pub const fn new(target_participation_bps: u16, max_participation_bps: u16) -> Self {
        assert!(
            target_participation_bps > 0
                && max_participation_bps > 0
                && target_participation_bps <= max_participation_bps,
            "participation target must be positive and <= cap"
        );
        Self {
            target_participation_bps,
            max_participation_bps,
        }
    }

    /// Returns target participation in basis points.
    pub const fn target_participation_bps(&self) -> u16 {
        self.target_participation_bps
    }

    /// Returns maximum participation in basis points.
    pub const fn max_participation_bps(&self) -> u16 {
        self.max_participation_bps
    }

    /// Plans one child slice from cumulative observed market volume.
    ///
    /// The planner assumes `observed_market_volume` excludes the algo's own
    /// child fills when the host can provide that view. If self-volume cannot
    /// be excluded, hosts should choose conservative rates and caps.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress state is invalid or the
    /// generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_volume_slice(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        observed_market_volume: OrderQty,
        now_ns: u64,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<Option<ChildOrderPlan>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if observed_market_volume.0 <= 0 || now_ns < parent.start_ns() || progress.is_complete() {
            return Ok(None);
        }

        let effective_max_bps = if parent.participation_cap_bps() == 0 {
            self.max_participation_bps
        } else {
            self.max_participation_bps
                .min(parent.participation_cap_bps())
        };
        if self.target_participation_bps == 0
            || effective_max_bps == 0
            || self.target_participation_bps > effective_max_bps
        {
            return Err(AlgoError::InvalidParticipationRate);
        }

        let desired = participation_qty(observed_market_volume.0, self.target_participation_bps)
            .min(parent.total_qty().0);
        let max_allowed = participation_qty(observed_market_volume.0, effective_max_bps)
            .min(parent.total_qty().0);
        let due_qty = desired
            .min(max_allowed)
            .saturating_sub(progress.released_qty().0);
        if due_qty <= 0 {
            return Ok(None);
        }

        let leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        let mut child_qty = due_qty.min(parent.max_clip().0).min(leaves);
        let final_slice = progress.released_qty().0.saturating_add(child_qty)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if child_qty < parent.min_clip().0 && !final_slice {
            return Ok(None);
        }
        if child_qty <= 0 {
            return Ok(None);
        }
        child_qty = child_qty.min(
            parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0),
        );
        let request = parent.build_order_request(client_order_id, OrderQty(child_qty), ts_recv_ns);
        Ok(Some(ChildOrderPlan::new(
            child_id,
            parent.id(),
            request,
            now_ns,
        )?))
    }
}

/// Borrowed cumulative volume curve for VWAP planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VwapVolumeCurve<'a> {
    start_ns: u64,
    bucket_interval_ns: u64,
    cumulative_weights: &'a [u64],
}

impl<'a> VwapVolumeCurve<'a> {
    /// Creates a borrowed cumulative VWAP volume curve.
    ///
    /// `cumulative_weights` must be non-empty, strictly positive at the end,
    /// and monotonically non-decreasing. The last element is the total expected
    /// volume weight for the parent interval.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidVolumeProfile`] when the interval is zero,
    /// the curve is empty, total weight is zero, or cumulative weights regress.
    pub fn new(
        start_ns: u64,
        bucket_interval_ns: u64,
        cumulative_weights: &'a [u64],
    ) -> Result<Self, AlgoError> {
        if bucket_interval_ns == 0 || cumulative_weights.is_empty() {
            return Err(AlgoError::InvalidVolumeProfile);
        }
        let Some(total) = cumulative_weights.last().copied() else {
            return Err(AlgoError::InvalidVolumeProfile);
        };
        if total == 0 {
            return Err(AlgoError::InvalidVolumeProfile);
        }
        let mut previous = 0_u64;
        for weight in cumulative_weights {
            if *weight < previous {
                return Err(AlgoError::InvalidVolumeProfile);
            }
            previous = *weight;
        }
        Ok(Self {
            start_ns,
            bucket_interval_ns,
            cumulative_weights,
        })
    }

    /// Returns curve start timestamp.
    pub const fn start_ns(&self) -> u64 {
        self.start_ns
    }

    /// Returns bucket interval in nanoseconds.
    pub const fn bucket_interval_ns(&self) -> u64 {
        self.bucket_interval_ns
    }

    /// Returns cumulative profile weights.
    pub const fn cumulative_weights(&self) -> &'a [u64] {
        self.cumulative_weights
    }

    /// Returns total curve weight.
    pub fn total_weight(&self) -> u64 {
        self.cumulative_weights
            .last()
            .copied()
            .expect("validated curve is non-empty")
    }

    fn cumulative_weight_at(&self, now_ns: u64) -> u64 {
        if now_ns < self.start_ns {
            return 0;
        }
        let elapsed = now_ns.saturating_sub(self.start_ns);
        let index = (elapsed / self.bucket_interval_ns) as usize;
        let capped = index.min(self.cumulative_weights.len().saturating_sub(1));
        self.cumulative_weights[capped]
    }
}

/// Deterministic VWAP child slice planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VwapSlicePlanner<'a> {
    curve: VwapVolumeCurve<'a>,
}

impl<'a> VwapSlicePlanner<'a> {
    /// Creates a VWAP planner from a validated volume curve.
    pub const fn new(curve: VwapVolumeCurve<'a>) -> Self {
        Self { curve }
    }

    /// Returns the borrowed volume curve.
    pub const fn curve(&self) -> VwapVolumeCurve<'a> {
        self.curve
    }

    /// Plans one child slice from the expected cumulative volume curve.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress state is invalid or the
    /// generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_curve_slice(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<Option<ChildOrderPlan>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if now_ns < parent.start_ns() || progress.is_complete() {
            return Ok(None);
        }

        let cumulative_weight = self.curve.cumulative_weight_at(now_ns);
        if cumulative_weight == 0 {
            return Ok(None);
        }
        let desired = vwap_target_qty(
            parent.total_qty().0,
            cumulative_weight,
            self.curve.total_weight(),
        );
        let due_qty = desired.saturating_sub(progress.released_qty().0);
        if due_qty <= 0 {
            return Ok(None);
        }

        let leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        let mut child_qty = due_qty.min(parent.max_clip().0).min(leaves);
        let final_slice = progress.released_qty().0.saturating_add(child_qty)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if child_qty < parent.min_clip().0 && !final_slice {
            return Ok(None);
        }
        if child_qty <= 0 {
            return Ok(None);
        }
        child_qty = child_qty.min(
            parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0),
        );
        let request = parent.build_order_request(client_order_id, OrderQty(child_qty), ts_recv_ns);
        Ok(Some(ChildOrderPlan::new(
            child_id,
            parent.id(),
            request,
            now_ns,
        )?))
    }
}

/// Deterministic synthetic iceberg replenishment planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IcebergSlicePlanner {
    display_qty: OrderQty,
    replenish_threshold: OrderQty,
}

impl IcebergSlicePlanner {
    /// Creates an iceberg planner.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError::InvalidDisplayQuantity`] when display quantity is
    /// non-positive, threshold is negative, or threshold exceeds display size.
    pub const fn try_new(
        display_qty: OrderQty,
        replenish_threshold: OrderQty,
    ) -> Result<Self, AlgoError> {
        if display_qty.0 <= 0 || replenish_threshold.0 < 0 || replenish_threshold.0 > display_qty.0
        {
            return Err(AlgoError::InvalidDisplayQuantity);
        }
        Ok(Self {
            display_qty,
            replenish_threshold,
        })
    }

    /// Creates an iceberg planner, panicking when display settings are invalid.
    pub const fn new(display_qty: OrderQty, replenish_threshold: OrderQty) -> Self {
        assert!(
            display_qty.0 > 0
                && replenish_threshold.0 >= 0
                && replenish_threshold.0 <= display_qty.0,
            "iceberg display quantity must be positive and threshold must be within display"
        );
        Self {
            display_qty,
            replenish_threshold,
        }
    }

    /// Returns the target displayed child quantity.
    pub const fn display_qty(&self) -> OrderQty {
        self.display_qty
    }

    /// Returns the open-quantity threshold at or below which replenishment is
    /// due.
    pub const fn replenish_threshold(&self) -> OrderQty {
        self.replenish_threshold
    }

    /// Plans one synthetic iceberg replenishment child.
    ///
    /// The host remains responsible for deciding whether to use native venue
    /// reserve/iceberg order support or submit synthetic child orders through
    /// the OMS.
    ///
    /// # Errors
    ///
    /// Returns [`AlgoError`] when parent/progress state is invalid or the
    /// generated child order would be invalid.
    #[allow(
        clippy::too_many_arguments,
        reason = "caller owns identifiers and timestamps"
    )]
    pub fn plan_replenishment(
        &self,
        parent: &ParentOrder,
        progress: AlgoProgress,
        now_ns: u64,
        child_id: ChildOrderId,
        client_order_id: ClientOrderId,
        ts_recv_ns: u64,
    ) -> Result<Option<ChildOrderPlan>, AlgoError> {
        parent.validate()?;
        if parent.status().is_terminal() {
            return Err(AlgoError::ParentTerminal);
        }
        if progress.parent_id() != parent.id() || progress.target_qty() != parent.total_qty() {
            return Err(AlgoError::InvalidProgress);
        }
        if now_ns < parent.start_ns()
            || progress.is_complete()
            || progress.open_qty().0 > self.replenish_threshold.0
        {
            return Ok(None);
        }

        let leaves = parent
            .total_qty()
            .0
            .saturating_sub(progress.released_qty().0);
        if leaves <= 0 {
            return Ok(None);
        }
        let mut child_qty = self.display_qty.0.min(parent.max_clip().0).min(leaves);
        let final_slice = progress.released_qty().0.saturating_add(child_qty)
            >= parent.total_qty().0
            || now_ns >= parent.end_ns();
        if child_qty < parent.min_clip().0 && !final_slice {
            return Ok(None);
        }
        if child_qty <= 0 {
            return Ok(None);
        }
        child_qty = child_qty.min(
            parent
                .total_qty()
                .0
                .saturating_sub(progress.released_qty().0),
        );
        let request = parent.build_order_request(client_order_id, OrderQty(child_qty), ts_recv_ns);
        Ok(Some(ChildOrderPlan::new(
            child_id,
            parent.id(),
            request,
            now_ns,
        )?))
    }
}
