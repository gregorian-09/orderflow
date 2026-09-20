#![doc = include_str!("../README.md")]

pub(crate) use std::collections::HashMap;
pub(crate) use std::fmt;
pub(crate) use std::hash::Hash;
pub(crate) use std::ops::BitOr;

mod accumulator;
mod domain;
mod patterns;
mod quality;
mod specialized;
mod statistics;
mod support;
mod tracking;

pub use accumulator::*;
pub use domain::*;
pub use patterns::*;
pub use quality::*;
pub use specialized::*;
pub use statistics::*;
pub(crate) use support::*;
pub use tracking::*;

include!("tests.rs");
