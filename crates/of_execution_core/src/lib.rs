//! Low-latency execution-domain primitives for Orderflow.
#![doc = include_str!("../README.md")]

pub(crate) use std::error::Error;
pub(crate) use std::fmt;
pub(crate) use std::hash::{Hash, Hasher};

#[path = "execution_core_foundation.rs"]
mod foundation;
#[path = "execution_core_orders.rs"]
mod orders;
#[path = "execution_core_risk.rs"]
mod risk;
#[path = "execution_core_wal.rs"]
mod wal;

pub use foundation::*;
pub use orders::*;
pub use risk::*;
pub use wal::*;

include!("execution_core_tests.rs");
