use super::*;

/// Shape of output produced by a signal module.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalOutputSemantics {
    /// Emits directional bias such as long, short, neutral, or blocked.
    DirectionalBias,
    /// Aggregates child signals into a combined output.
    CompositeBias,
    /// Emits an informational state that should not be treated as direction.
    Informational,
    /// Emits a veto or gate over another signal or strategy.
    Veto,
}

/// Parameter value used in signal metadata.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignalParameterValue {
    /// Signed integer parameter.
    Integer(i64),
    /// Floating-point parameter.
    Float(f64),
    /// Boolean parameter.
    Boolean(bool),
    /// Static text parameter.
    Text(&'static str),
}

/// Parameter type advertised by a signal descriptor.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalParameterKind {
    /// Signed integer value.
    Integer,
    /// Floating-point value.
    Float,
    /// Boolean value.
    Boolean,
    /// Static text value.
    Text,
}

/// Metadata for one configurable signal parameter.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalParameterDescriptor {
    /// Stable parameter name.
    pub name: &'static str,
    /// Human-readable parameter description.
    pub description: &'static str,
    /// Expected parameter value type.
    pub kind: SignalParameterKind,
    /// Default value used by the built-in implementation.
    pub default: Option<SignalParameterValue>,
    /// Inclusive minimum value when the parameter is range-bound.
    pub min: Option<SignalParameterValue>,
    /// Inclusive maximum value when the parameter is range-bound.
    pub max: Option<SignalParameterValue>,
}

impl SignalParameterDescriptor {
    /// Creates metadata for a signal parameter.
    pub const fn new(
        name: &'static str,
        description: &'static str,
        kind: SignalParameterKind,
        default: Option<SignalParameterValue>,
        min: Option<SignalParameterValue>,
        max: Option<SignalParameterValue>,
    ) -> Self {
        Self {
            name,
            description,
            kind,
            default,
            min,
            max,
        }
    }

    /// Creates metadata for an integer signal parameter.
    pub const fn integer(
        name: &'static str,
        description: &'static str,
        default: Option<i64>,
        min: Option<i64>,
        max: Option<i64>,
    ) -> Self {
        Self::new(
            name,
            description,
            SignalParameterKind::Integer,
            match default {
                Some(value) => Some(SignalParameterValue::Integer(value)),
                None => None,
            },
            match min {
                Some(value) => Some(SignalParameterValue::Integer(value)),
                None => None,
            },
            match max {
                Some(value) => Some(SignalParameterValue::Integer(value)),
                None => None,
            },
        )
    }
}

/// Static metadata describing a signal module.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalDescriptor {
    /// Stable signal identifier. This should match `SignalSnapshot::module_id`.
    pub id: &'static str,
    /// Human-readable signal name.
    pub name: &'static str,
    /// Descriptor/schema version for this signal definition.
    pub version: &'static str,
    /// Human-readable signal description.
    pub description: &'static str,
    /// Inputs required by the signal.
    pub required_inputs: SignalInputMask,
    /// Warmup needed before production use.
    pub warmup: SignalWarmupRequirement,
    /// Public parameter metadata.
    pub parameters: &'static [SignalParameterDescriptor],
    /// Output semantics for consumers and dashboards.
    pub output_semantics: SignalOutputSemantics,
    /// Whether the signal is deterministic for the same ordered input stream.
    pub deterministic: bool,
    /// Whether the current implementation exposes checkpointable state.
    pub checkpointable: bool,
}

/// Configuration value used when constructing a signal from a registry.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignalConfigValue<'a> {
    /// Signed integer configuration value.
    Integer(i64),
    /// Floating-point configuration value.
    Float(f64),
    /// Boolean configuration value.
    Boolean(bool),
    /// Borrowed text configuration value.
    Text(&'a str),
}

impl SignalConfigValue<'_> {
    /// Returns the parameter kind represented by this value.
    pub const fn kind(self) -> SignalParameterKind {
        match self {
            Self::Integer(_) => SignalParameterKind::Integer,
            Self::Float(_) => SignalParameterKind::Float,
            Self::Boolean(_) => SignalParameterKind::Boolean,
            Self::Text(_) => SignalParameterKind::Text,
        }
    }
}

/// One named parameter supplied in a signal configuration.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalConfigParameter<'a> {
    /// Stable parameter name.
    pub name: &'a str,
    /// Supplied parameter value.
    pub value: SignalConfigValue<'a>,
}

impl<'a> SignalConfigParameter<'a> {
    /// Creates a named signal configuration parameter.
    pub const fn new(name: &'a str, value: SignalConfigValue<'a>) -> Self {
        Self { name, value }
    }

    /// Creates an integer signal configuration parameter.
    pub const fn integer(name: &'a str, value: i64) -> Self {
        Self::new(name, SignalConfigValue::Integer(value))
    }

    /// Creates a floating-point signal configuration parameter.
    pub const fn float(name: &'a str, value: f64) -> Self {
        Self::new(name, SignalConfigValue::Float(value))
    }

    /// Creates a boolean signal configuration parameter.
    pub const fn boolean(name: &'a str, value: bool) -> Self {
        Self::new(name, SignalConfigValue::Boolean(value))
    }

    /// Creates a text signal configuration parameter.
    pub const fn text(name: &'a str, value: &'a str) -> Self {
        Self::new(name, SignalConfigValue::Text(value))
    }
}

/// Borrowed signal configuration for registry validation and construction.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalConfig<'a> {
    /// Stable signal id.
    pub id: &'a str,
    /// Supplied configuration parameters.
    pub parameters: &'a [SignalConfigParameter<'a>],
}

impl<'a> SignalConfig<'a> {
    /// Creates a signal configuration with no parameters.
    pub const fn new(id: &'a str) -> Self {
        Self {
            id,
            parameters: &[],
        }
    }

    /// Creates a signal configuration with explicit parameters.
    pub const fn with_parameters(id: &'a str, parameters: &'a [SignalConfigParameter<'a>]) -> Self {
        Self { id, parameters }
    }

    /// Finds a supplied parameter by name.
    pub fn parameter(&self, name: &str) -> Option<SignalConfigValue<'a>> {
        self.parameters
            .iter()
            .find(|parameter| parameter.name == name)
            .map(|parameter| parameter.value)
    }
}

/// Error returned by signal registry validation or construction.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum SignalRegistryError {
    /// The requested signal id is not registered.
    UnknownSignal {
        /// Requested signal id.
        id: String,
    },
    /// A signal id was registered more than once.
    DuplicateSignal {
        /// Duplicate signal id.
        id: &'static str,
    },
    /// A configuration supplied the same parameter more than once.
    DuplicateParameter {
        /// Signal id being validated.
        signal_id: &'static str,
        /// Duplicate parameter name.
        name: String,
    },
    /// A configuration supplied a parameter unknown to the signal descriptor.
    UnknownParameter {
        /// Signal id being validated.
        signal_id: &'static str,
        /// Unknown parameter name.
        name: String,
    },
    /// A configuration supplied a value with the wrong type.
    InvalidParameterType {
        /// Signal id being validated.
        signal_id: &'static str,
        /// Parameter name.
        name: &'static str,
        /// Expected kind.
        expected: SignalParameterKind,
        /// Actual kind.
        actual: SignalParameterKind,
    },
    /// A configuration supplied a value below the descriptor minimum.
    ParameterBelowMinimum {
        /// Signal id being validated.
        signal_id: &'static str,
        /// Parameter name.
        name: &'static str,
        /// Minimum allowed value.
        min: SignalParameterValue,
        /// Supplied value.
        actual: SignalConfigValue<'static>,
    },
    /// A configuration supplied a value above the descriptor maximum.
    ParameterAboveMaximum {
        /// Signal id being validated.
        signal_id: &'static str,
        /// Parameter name.
        name: &'static str,
        /// Maximum allowed value.
        max: SignalParameterValue,
        /// Supplied value.
        actual: SignalConfigValue<'static>,
    },
    /// The registered signal has no factory.
    MissingFactory {
        /// Signal id being constructed.
        signal_id: &'static str,
    },
}

impl std::fmt::Display for SignalRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSignal { id } => write!(f, "unknown signal id `{id}`"),
            Self::DuplicateSignal { id } => write!(f, "duplicate signal id `{id}`"),
            Self::DuplicateParameter { signal_id, name } => {
                write!(f, "duplicate parameter `{name}` for signal `{signal_id}`")
            }
            Self::UnknownParameter { signal_id, name } => {
                write!(f, "unknown parameter `{name}` for signal `{signal_id}`")
            }
            Self::InvalidParameterType {
                signal_id,
                name,
                expected,
                actual,
            } => write!(
                f,
                "invalid parameter type for `{signal_id}.{name}`: expected {expected:?}, got {actual:?}"
            ),
            Self::ParameterBelowMinimum {
                signal_id, name, ..
            } => write!(
                f,
                "parameter `{signal_id}.{name}` is below the descriptor minimum"
            ),
            Self::ParameterAboveMaximum {
                signal_id, name, ..
            } => write!(
                f,
                "parameter `{signal_id}.{name}` is above the descriptor maximum"
            ),
            Self::MissingFactory { signal_id } => {
                write!(f, "signal `{signal_id}` has no construction factory")
            }
        }
    }
}

impl std::error::Error for SignalRegistryError {}

/// Result returned by signal registry operations.
pub type SignalRegistryResult<T> = Result<T, SignalRegistryError>;

/// Factory function used by [`SignalRegistry`] to build a signal module.
pub type SignalFactory = fn(&SignalConfig<'_>) -> SignalRegistryResult<Box<dyn SignalModule>>;

/// One signal registration containing descriptor metadata and optional factory.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct SignalRegistration {
    /// Static signal descriptor.
    pub descriptor: &'static SignalDescriptor,
    /// Optional factory for constructing the signal from config.
    pub factory: Option<SignalFactory>,
}

impl SignalRegistration {
    /// Creates a signal registration.
    pub const fn new(
        descriptor: &'static SignalDescriptor,
        factory: Option<SignalFactory>,
    ) -> Self {
        Self {
            descriptor,
            factory,
        }
    }
}

/// Registry for discovering, validating, and constructing signal modules.
///
/// The registry is intended for startup/configuration paths, dashboards, and
/// bindings. Signal evaluation remains owned by concrete modules and the
/// `SignalModule` trait.
#[derive(Debug, Clone)]
pub struct SignalRegistry {
    registrations: Vec<SignalRegistration>,
}

impl SignalRegistry {
    /// Creates an empty signal registry.
    pub const fn new() -> Self {
        Self {
            registrations: Vec::new(),
        }
    }

    /// Creates a registry containing the built-in signal modules.
    pub fn with_built_ins() -> Self {
        Self {
            registrations: built_in_signal_registrations().to_vec(),
        }
    }

    /// Adds a signal registration.
    pub fn register(
        &mut self,
        registration: SignalRegistration,
    ) -> SignalRegistryResult<&mut Self> {
        if self
            .registrations
            .iter()
            .any(|existing| existing.descriptor.id == registration.descriptor.id)
        {
            return Err(SignalRegistryError::DuplicateSignal {
                id: registration.descriptor.id,
            });
        }
        self.registrations.push(registration);
        Ok(self)
    }

    /// Returns registered signal metadata.
    pub fn registrations(&self) -> &[SignalRegistration] {
        &self.registrations
    }

    /// Finds a registered signal descriptor by id.
    pub fn descriptor(&self, id: &str) -> Option<&'static SignalDescriptor> {
        self.registration(id)
            .map(|registration| registration.descriptor)
    }

    /// Returns descriptors whose required inputs are included in `available_inputs`.
    pub fn descriptors_matching_inputs(
        &self,
        available_inputs: SignalInputMask,
    ) -> Vec<&'static SignalDescriptor> {
        self.registrations
            .iter()
            .filter_map(|registration| {
                available_inputs
                    .contains(registration.descriptor.required_inputs)
                    .then_some(registration.descriptor)
            })
            .collect()
    }

    /// Validates a signal configuration without constructing the module.
    pub fn validate_config(&self, config: &SignalConfig<'_>) -> SignalRegistryResult<()> {
        let descriptor =
            self.descriptor(config.id)
                .ok_or_else(|| SignalRegistryError::UnknownSignal {
                    id: config.id.to_string(),
                })?;
        validate_signal_config(descriptor, config)
    }

    /// Validates a configuration and returns a stable JSON result for bindings.
    ///
    /// Registry validation failures are represented by `valid: false` in the
    /// returned document. The method itself does not panic or discard the
    /// diagnostic message.
    pub fn validate_config_json(&self, config: &SignalConfig<'_>) -> String {
        let result = self.validate_config(config);
        let mut out = String::from("{\"schema_version\":1,\"signal_id\":");
        push_json_string(&mut out, config.id);
        out.push_str(",\"valid\":");
        out.push_str(if result.is_ok() { "true" } else { "false" });
        out.push_str(",\"error\":");
        match result {
            Ok(()) => out.push_str("null"),
            Err(error) => push_json_string(&mut out, &error.to_string()),
        }
        out.push('}');
        out
    }

    /// Constructs a signal module from configuration.
    pub fn create_signal(
        &self,
        config: &SignalConfig<'_>,
    ) -> SignalRegistryResult<Box<dyn SignalModule>> {
        let registration =
            self.registration(config.id)
                .ok_or_else(|| SignalRegistryError::UnknownSignal {
                    id: config.id.to_string(),
                })?;
        self.validate_config(config)?;
        let Some(factory) = registration.factory else {
            return Err(SignalRegistryError::MissingFactory {
                signal_id: registration.descriptor.id,
            });
        };
        factory(config)
    }

    /// Exports registered descriptors as compact JSON for bindings and dashboards.
    pub fn descriptors_json(&self) -> String {
        let mut out = String::from("[");
        for (index, registration) in self.registrations.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            push_descriptor_json(&mut out, registration.descriptor);
        }
        out.push(']');
        out
    }

    fn registration(&self, id: &str) -> Option<&SignalRegistration> {
        self.registrations
            .iter()
            .find(|registration| registration.descriptor.id == id)
    }
}

impl Default for SignalRegistry {
    fn default() -> Self {
        Self::with_built_ins()
    }
}
