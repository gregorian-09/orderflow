//! Execution adapter infrastructure for Orderflow.
#![doc = include_str!("../README.md")]

#[cfg(feature = "fix")]
/// FIX execution adapter integration.
pub mod fix;
