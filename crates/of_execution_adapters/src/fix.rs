//! FIX execution adapters, transport contracts, and report mapping.

pub(crate) use of_execution::{
    ExecutionAdapter, ExecutionCapabilities, ExecutionError, ExecutionEventBuffer, ExecutionHealth,
    ExecutionResult, LatencyClass,
};
pub(crate) use of_execution_core::{
    AccountId, AmendRequest, CancelRequest, ClientOrderId, ExecutionCoreError, ExecutionEvent,
    ExecutionId, ExecutionSymbol, ExecutionText, ExecutionType, FixedAscii, InstrumentId,
    OrderPrice, OrderQty, OrderRequest, OrderSide, OrderStatus, OrderType, RiskRejectReason,
    RouteId, TimeInForce, VenueId, VenueOrderId,
};
pub(crate) use of_fix::{
    encode_new_order_single, encode_order_cancel_replace_request, encode_order_cancel_request,
    FixEncodeError, FixMessageView, FixMsgType, FixNewOrderSingle, FixOrdType,
    FixOrderCancelReplaceRequest, FixOrderCancelRequest, FixOrderSide, FixSessionHeader, FixTag,
    FixTimeInForce, FixVersion,
};
pub(crate) use std::error::Error;
pub(crate) use std::fmt;

#[path = "fix_mapping.rs"]
mod mapping;
#[path = "fix_model.rs"]
mod model;
#[path = "fix_shell.rs"]
mod shell;
#[path = "fix_support.rs"]
mod support;

pub use certification::*;
pub use live::*;
pub use mapping::*;
pub use model::*;
pub use shell::*;
pub(crate) use support::*;

mod certification;
mod live;

include!("fix_tests.rs");
