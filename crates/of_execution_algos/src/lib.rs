//! Parent/child execution algorithm primitives for Orderflow.
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::error::Error;
use std::fmt;

use of_execution_core::{
    AccountId, ClientOrderId, ExecutionCoreError, ExecutionEvent, ExecutionId, ExecutionSymbol,
    ExecutionText, ExecutionType, FixedAscii, OrderPrice, OrderQty, OrderRequest, OrderSide,
    OrderStatus, OrderType, RiskRejectReason, RouteId, StrategyId, TimeInForce, VenueOrderId,
};

/// Default maximum number of actions retained in an [`AlgoDecision`].
pub const DEFAULT_ALGO_DECISION_CAPACITY: usize = 16;
/// Default maximum number of retained violations in an [`AlgoRiskReport`].
pub const DEFAULT_ALGO_RISK_VIOLATION_CAPACITY: usize = 16;
/// Current algorithm checkpoint schema version.
pub const ALGO_CHECKPOINT_SCHEMA_VERSION: u16 = 1;

mod liquidity_seeking;
mod market_making;
mod model;
mod passive;
mod portfolio;
mod replay;
mod shortfall;
mod slicing;
mod sor;
mod support;
mod sweep;

pub use liquidity_seeking::*;
pub use market_making::*;
pub use model::*;
pub use passive::*;
pub use portfolio::*;
pub use replay::*;
pub use shortfall::*;
pub use slicing::*;
pub use sor::*;
pub(crate) use support::*;
pub use sweep::*;

include!("tests.rs");
