#![doc = include_str!("../README.md")]

use std::ops::{BitOr, BitOrAssign};

use of_core::{
    AnalyticsSnapshot, BookSnapshot, DataQualityFlags, SignalSnapshot, SignalState, SymbolId,
};

mod builtin;
mod calibration;
mod checkpoint;
mod ensemble;
mod explanation;
mod features;
mod foundation;
mod models;
mod registry;
mod shadow;
mod stabilizer;
mod validation;

#[allow(unused_imports)]
pub(crate) use builtin::{
    classify_transition, config_value_is_above, config_value_is_below, config_value_to_static,
    create_absorption_signal, create_composite_signal, create_cumulative_delta_signal,
    create_delta_momentum_signal, create_exhaustion_signal, create_sweep_detection_signal,
    create_volume_imbalance_signal, default_quality_gate, integer_parameter, is_directional,
    neutral_like, output_semantics_name, parameter_kind_name, push_confidence_components_json,
    push_descriptor_json, push_explanation_json, push_input_mask_json, push_input_values_json,
    push_json_bool_field, push_json_field, push_json_i64_field, push_json_string,
    push_json_u16_field, push_json_usize_field, push_optional_parameter_value_json,
    push_parameter_value_json, push_parameters_json, push_thresholds_json,
    push_validation_sample_json, push_validation_warning_json, push_warmup_json,
    same_pending_state, signal_state_name, validate_signal_config, ABSORPTION_PARAMS,
    ANALYTICS_AND_QUALITY, BREAKOUT_TICKS_PARAM, BUILT_IN_SIGNAL_DESCRIPTORS,
    BUILT_IN_SIGNAL_REGISTRATIONS, COMPOSITE_PARAMS, CUMULATIVE_DELTA_PARAMS,
    DELTA_MOMENTUM_PARAMS, EXHAUSTION_PARAMS, PRICE_BAND_PARAM, SWEEP_DETECTION_PARAMS,
    THRESHOLD_PARAM_100, THRESHOLD_PARAM_150, THRESHOLD_PARAM_250, VOLUME_IMBALANCE_PARAMS,
};

pub(crate) use calibration::{average_bps, push_optional_u16_json, ratio_bps};
pub(crate) use explanation::PendingSignal;
pub(crate) use validation::score_direction;

pub use builtin::*;
pub use calibration::*;
pub use checkpoint::*;
pub use ensemble::*;
pub use explanation::*;
pub use features::*;
pub use foundation::*;
pub use models::*;
pub use registry::*;
pub use shadow::*;
pub use stabilizer::*;
pub use validation::*;

include!("tests.rs");
