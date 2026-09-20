#![doc = include_str!("../README.md")]

pub(crate) use of_core::{BookUpdate, SymbolId, TradePrint};
pub(crate) use std::error::Error;
pub(crate) use std::fmt;
#[cfg(any(feature = "binance", feature = "cqg", feature = "rithmic"))]
pub(crate) use std::{env, path::Path};

#[path = "adapter_config.rs"]
mod config;
#[path = "adapter_contract.rs"]
mod contract;
#[path = "adapter_discovery.rs"]
mod discovery;
#[path = "adapter_factory.rs"]
mod factory;
#[path = "adapter_mock.rs"]
mod mock;
#[path = "adapter_status.rs"]
mod status;

pub(crate) use config::openssl_s_client_args;
pub use config::*;
pub use contract::*;
pub use discovery::*;
pub use factory::*;
pub use mock::*;
pub use status::*;

#[cfg(feature = "rithmic")]
/// Rithmic adapter implementation (feature-gated).
pub mod rithmic;

#[cfg(feature = "cqg")]
/// CQG adapter implementation (feature-gated).
pub mod cqg;

#[cfg(feature = "binance")]
/// Binance adapter implementation (feature-gated).
pub mod binance;

include!("adapter_tests.rs");
