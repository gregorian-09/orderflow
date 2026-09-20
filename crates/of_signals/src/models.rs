use super::*;

/// Supported model artifact/runtime family.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SignalModelKind {
    /// Model kind is not specified.
    #[default]
    Unknown,
    /// Host-provided native model implementation.
    Native,
    /// ONNX model artifact.
    Onnx,
    /// Linear model coefficients interpreted by the host.
    Linear,
    /// Tree or boosted-tree model artifact.
    TreeEnsemble,
    /// External service or process.
    External,
}

/// Model output semantics for model-backed signals.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SignalModelOutputKind {
    /// Model returns directional state and confidence directly.
    #[default]
    DirectionalState,
    /// Model returns up/down/flat probabilities.
    DirectionalProbabilities,
    /// Model returns a continuous score.
    Score,
}

/// Metadata describing a model-backed signal artifact.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct SignalModelMetadata {
    /// Stable model id.
    pub model_id: String,
    /// Model version.
    pub model_version: String,
    /// Model artifact/runtime kind.
    pub model_kind: SignalModelKind,
    /// Feature schema id expected by the model.
    pub feature_schema_id: String,
    /// Feature schema version expected by the model.
    pub feature_schema_version: String,
    /// Optional model artifact hash.
    pub artifact_hash: Option<String>,
    /// Optional training window start timestamp.
    pub training_start_ns: Option<u64>,
    /// Optional training window end timestamp.
    pub training_end_ns: Option<u64>,
    /// Optional calibration id.
    pub calibration_id: Option<u64>,
    /// Output semantics.
    pub output_kind: SignalModelOutputKind,
    /// Whether inference should be deterministic for identical input vectors.
    pub deterministic: bool,
}

impl SignalModelMetadata {
    /// Creates model metadata.
    pub fn new(
        model_id: impl Into<String>,
        model_version: impl Into<String>,
        feature_schema_id: impl Into<String>,
        feature_schema_version: impl Into<String>,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            model_version: model_version.into(),
            model_kind: SignalModelKind::Unknown,
            feature_schema_id: feature_schema_id.into(),
            feature_schema_version: feature_schema_version.into(),
            artifact_hash: None,
            training_start_ns: None,
            training_end_ns: None,
            calibration_id: None,
            output_kind: SignalModelOutputKind::DirectionalState,
            deterministic: true,
        }
    }

    /// Returns metadata with a model kind.
    pub const fn with_model_kind(mut self, model_kind: SignalModelKind) -> Self {
        self.model_kind = model_kind;
        self
    }

    /// Returns metadata with artifact hash.
    pub fn with_artifact_hash(mut self, artifact_hash: impl Into<String>) -> Self {
        self.artifact_hash = Some(artifact_hash.into());
        self
    }

    /// Returns metadata with training window timestamps.
    pub const fn with_training_window(mut self, start_ns: u64, end_ns: u64) -> Self {
        self.training_start_ns = Some(start_ns);
        self.training_end_ns = Some(end_ns);
        self
    }

    /// Returns metadata with calibration id.
    pub const fn with_calibration_id(mut self, calibration_id: u64) -> Self {
        self.calibration_id = Some(calibration_id);
        self
    }

    /// Returns metadata with output kind.
    pub const fn with_output_kind(mut self, output_kind: SignalModelOutputKind) -> Self {
        self.output_kind = output_kind;
        self
    }
}

/// Input binding between a model and a feature schema.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalModelInputBinding {
    /// Model input name.
    pub input_name: String,
    /// Ordered feature ids required by this input.
    pub feature_ids: Vec<String>,
}

impl SignalModelInputBinding {
    /// Creates a model input binding.
    pub fn new(input_name: impl Into<String>, feature_ids: Vec<String>) -> Self {
        Self {
            input_name: input_name.into(),
            feature_ids,
        }
    }

    /// Returns `true` when all bound feature ids exist in the schema.
    pub fn is_compatible_with(&self, schema: &FeatureSchema) -> bool {
        self.feature_ids
            .iter()
            .all(|feature_id| schema.feature_index(feature_id).is_some())
    }
}

/// Output returned by model-backed signal inference.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct SignalModelOutput {
    /// Signal state produced by the model.
    pub state: SignalState,
    /// Confidence in basis points.
    pub confidence_bps: u16,
    /// Optional continuous model score.
    pub score: Option<f64>,
    /// Human-readable model reason.
    pub reason: String,
}

impl SignalModelOutput {
    /// Creates a model output from state and confidence.
    pub fn new(state: SignalState, confidence_bps: u16) -> Self {
        Self {
            state,
            confidence_bps: confidence_bps.min(10_000),
            score: None,
            reason: String::new(),
        }
    }

    /// Returns this output with a score.
    pub const fn with_score(mut self, score: f64) -> Self {
        self.score = Some(score);
        self
    }

    /// Returns this output with reason text.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }
}

/// Optional extension trait for model-backed signal implementations.
///
/// This trait does not prescribe a runtime such as ONNX or TensorRT. Hosts and
/// optional crates can implement inference while the core `of_signals` crate
/// remains dependency-light.
pub trait ModelBackedSignal: SignalModule {
    /// Returns model artifact metadata.
    fn model_metadata(&self) -> &SignalModelMetadata;

    /// Returns the feature schema consumed by this model.
    fn feature_schema(&self) -> &FeatureSchema;

    /// Runs model inference over a validated feature vector.
    fn infer_features(&mut self, features: &FeatureVectorView<'_>) -> SignalModelOutput;
}

impl SignalDescriptor {
    /// Creates static metadata for a signal module with conservative defaults.
    ///
    /// Use the `with_*` methods to attach inputs, warmup policy, parameters,
    /// output semantics, and capability flags.
    pub const fn new(
        id: &'static str,
        name: &'static str,
        version: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            id,
            name,
            version,
            description,
            required_inputs: SignalInputMask::NONE,
            warmup: SignalWarmupRequirement::None,
            parameters: &[],
            output_semantics: SignalOutputSemantics::DirectionalBias,
            deterministic: true,
            checkpointable: false,
        }
    }

    /// Returns a descriptor with required input metadata changed.
    pub const fn with_required_inputs(mut self, required_inputs: SignalInputMask) -> Self {
        self.required_inputs = required_inputs;
        self
    }

    /// Returns a descriptor with warmup metadata changed.
    pub const fn with_warmup(mut self, warmup: SignalWarmupRequirement) -> Self {
        self.warmup = warmup;
        self
    }

    /// Returns a descriptor with parameter metadata changed.
    pub const fn with_parameters(
        mut self,
        parameters: &'static [SignalParameterDescriptor],
    ) -> Self {
        self.parameters = parameters;
        self
    }

    /// Returns a descriptor with output semantics metadata changed.
    pub const fn with_output_semantics(mut self, output_semantics: SignalOutputSemantics) -> Self {
        self.output_semantics = output_semantics;
        self
    }

    /// Returns a descriptor with the deterministic flag changed.
    pub const fn with_deterministic(mut self, deterministic: bool) -> Self {
        self.deterministic = deterministic;
        self
    }

    /// Returns a descriptor with the checkpointable flag changed.
    pub const fn with_checkpointable(mut self, checkpointable: bool) -> Self {
        self.checkpointable = checkpointable;
        self
    }

    /// Returns `true` when the descriptor requires `input`.
    pub const fn requires_input(&self, input: SignalInputMask) -> bool {
        self.required_inputs.contains(input)
    }

    /// Finds a parameter descriptor by stable name.
    pub fn parameter(&self, name: &str) -> Option<&'static SignalParameterDescriptor> {
        self.parameters
            .iter()
            .find(|parameter| parameter.name == name)
    }
}
