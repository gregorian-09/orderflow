//! Advanced market microstructure analytics for Orderflow.
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::error::Error;
use std::fmt;

use of_core::{BookLevel, Side};

mod cross_asset;
mod depth;
mod derivatives;
mod error;
mod features;
mod impact;
mod liquidity;
mod market_quality;
mod patterns;
mod quality;
mod queue;
mod regime;
mod resiliency;
mod route;
mod support;
mod toxicity;
mod volatility;

pub use cross_asset::*;
pub use depth::*;
pub use derivatives::*;
pub use error::*;
pub use features::*;
pub use impact::*;
pub use liquidity::*;
pub use market_quality::*;
pub use patterns::*;
pub use quality::*;
pub use queue::*;
pub use regime::*;
pub use resiliency::*;
pub use route::*;
pub(crate) use support::*;
pub use toxicity::*;
pub use volatility::*;

include!("tests.rs");
