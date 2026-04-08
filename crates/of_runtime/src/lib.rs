#![doc = include_str!("../README.md")]

mod config;
mod engine;

pub use config::{
    load_engine_config_from_path, load_engine_config_report_from_path, validate_startup_config,
    ConfigCompatibilityMode, ConfigLoadReport,
};
pub use engine::{build_default_engine, DefaultEngine, Engine, EngineConfig, ExternalFeedPolicy, RuntimeError};

#[cfg(test)]
pub(crate) use engine::rotated_path;

#[cfg(test)]
include!("tests.rs");
