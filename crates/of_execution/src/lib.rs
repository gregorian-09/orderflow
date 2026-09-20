//! Execution routing and adapter contracts for Orderflow.
#![doc = include_str!("../README.md")]

pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use std::error::Error;
pub(crate) use std::fmt;
pub(crate) use std::sync::atomic::{AtomicU64, Ordering};
pub(crate) use std::sync::mpsc::{
    self, Receiver, RecvError, RecvTimeoutError, SyncSender, TryRecvError, TrySendError,
};
pub(crate) use std::sync::Arc;
pub(crate) use std::thread::{self, JoinHandle};
pub(crate) use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) use of_execution_core::{
    AccountId, AmendRequest, BasicRiskGate, CancelRequest, ClientOrderId, ExecutionCoreError,
    ExecutionEvent, ExecutionId, ExecutionSymbol, ExecutionText, ExecutionType, OrderPrice,
    OrderQty, OrderRequest, OrderState, OrderStateMachine, OrderStatus, OrderType, RiskCheck,
    RiskContext, RiskLimits, RiskRejectReason, RouteId, TimeInForce, VenueOrderId,
};

#[path = "execution_concurrent.rs"]
mod concurrent;
#[path = "execution_contract.rs"]
mod contract;
#[path = "execution_engine.rs"]
mod engine;
#[path = "execution_sim.rs"]
mod simulation;

mod audit_bundle;
mod certification_venue;
mod drop_copy;
mod execution_slo;
mod idempotency;
mod kill_switch;
mod oms;
mod operator_control;
mod order_intent;
mod position_ledger;
mod production_risk;
mod reconciliation;

pub use audit_bundle::*;
pub use certification_venue::*;
pub use concurrent::*;
pub use contract::*;
pub use drop_copy::*;
pub use engine::*;
pub use execution_slo::*;
pub use idempotency::*;
pub use kill_switch::*;
pub use oms::*;
pub use operator_control::*;
pub use order_intent::*;
pub use position_ledger::*;
pub use production_risk::*;
pub use reconciliation::*;
pub use simulation::*;

pub(crate) use simulation::{
    build_route_index, execution_price, route_reject, state_matches_route, unix_ts_nanos,
};

include!("execution_tests.rs");
