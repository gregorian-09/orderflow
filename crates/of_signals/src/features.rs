use super::*;

/// Quality flags attached to one feature value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FeatureQualityFlags(u32);

impl FeatureQualityFlags {
    /// No feature-quality issues.
    pub const NONE: Self = Self(0);
    /// Feature value is missing.
    pub const MISSING: Self = Self(1 << 0);
    /// Feature value is stale relative to the schema freshness policy.
    pub const STALE: Self = Self(1 << 1);
    /// Feature value is outside the descriptor range.
    pub const OUT_OF_RANGE: Self = Self(1 << 2);
    /// Feature value was imputed.
    pub const IMPUTED: Self = Self(1 << 3);
    /// Feature pipeline marked the value as degraded.
    pub const DEGRADED: Self = Self(1 << 4);

    /// Returns raw flag bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Creates flags from raw bits, ignoring unknown bits.
    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self(bits & Self::all_bits())
    }

    /// Returns true when all `other` flags are present.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns true when any `other` flag is present.
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    /// Returns the union of two flag sets.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    const fn all_bits() -> u32 {
        Self::MISSING.0 | Self::STALE.0 | Self::OUT_OF_RANGE.0 | Self::IMPUTED.0 | Self::DEGRADED.0
    }
}

impl BitOr for FeatureQualityFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl BitOrAssign for FeatureQualityFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

/// Semantic kind for one feature value.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FeatureValueKind {
    /// Generic floating-point feature.
    #[default]
    Float,
    /// Integer-valued feature encoded as `f64` in a vector.
    Integer,
    /// Boolean feature encoded as `0.0` or `1.0`.
    Boolean,
    /// Price-normalized feature.
    Price,
    /// Size/quantity feature.
    Size,
    /// Basis-point feature.
    BasisPoints,
}

/// Missing-value policy for a feature descriptor.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FeatureMissingPolicy {
    /// Reject vectors where this feature is missing.
    #[default]
    Reject,
    /// Treat missing values as zero.
    TreatAsZero,
    /// Use the configured default value.
    UseDefault(f64),
    /// Keep the value unavailable for downstream model code.
    MarkUnavailable,
}

/// One feature in a signal/model feature schema.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureDescriptor {
    /// Stable feature id.
    pub id: String,
    /// Feature value kind.
    pub value_kind: FeatureValueKind,
    /// Human-readable unit name.
    pub unit: String,
    /// Human-readable description.
    pub description: String,
    /// Missing-value handling policy.
    pub missing_policy: FeatureMissingPolicy,
    /// Optional minimum accepted value.
    pub min_value: Option<f64>,
    /// Optional maximum accepted value.
    pub max_value: Option<f64>,
    /// Optional freshness limit in nanoseconds.
    pub freshness_ns: Option<u64>,
}

impl FeatureDescriptor {
    /// Creates a feature descriptor with conservative defaults.
    pub fn new(id: impl Into<String>, value_kind: FeatureValueKind) -> Self {
        Self {
            id: id.into(),
            value_kind,
            unit: String::new(),
            description: String::new(),
            missing_policy: FeatureMissingPolicy::Reject,
            min_value: None,
            max_value: None,
            freshness_ns: None,
        }
    }

    /// Returns this descriptor with unit metadata.
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Returns this descriptor with a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Returns this descriptor with a missing-value policy.
    pub const fn with_missing_policy(mut self, missing_policy: FeatureMissingPolicy) -> Self {
        self.missing_policy = missing_policy;
        self
    }

    /// Returns this descriptor with an inclusive value range.
    pub const fn with_range(mut self, min_value: f64, max_value: f64) -> Self {
        self.min_value = Some(min_value);
        self.max_value = Some(max_value);
        self
    }

    /// Returns this descriptor with a freshness limit.
    pub const fn with_freshness_ns(mut self, freshness_ns: u64) -> Self {
        self.freshness_ns = Some(freshness_ns);
        self
    }
}

/// Stable feature schema used by feature-vector and model-backed signals.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureSchema {
    /// Stable schema id.
    pub id: String,
    /// Schema version.
    pub version: String,
    /// Host-defined config hash for the schema.
    pub config_hash: u64,
    /// Ordered feature descriptors.
    pub features: Vec<FeatureDescriptor>,
}

impl FeatureSchema {
    /// Creates an empty feature schema.
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            config_hash: 0,
            features: Vec::new(),
        }
    }

    /// Returns this schema with a config hash.
    pub const fn with_config_hash(mut self, config_hash: u64) -> Self {
        self.config_hash = config_hash;
        self
    }

    /// Returns this schema with an appended feature descriptor.
    pub fn with_feature(mut self, feature: FeatureDescriptor) -> Self {
        self.features.push(feature);
        self
    }

    /// Appends a feature descriptor.
    pub fn push_feature(&mut self, feature: FeatureDescriptor) {
        self.features.push(feature);
    }

    /// Returns the index for a feature id.
    pub fn feature_index(&self, id: &str) -> Option<usize> {
        self.features.iter().position(|feature| feature.id == id)
    }

    /// Returns a feature descriptor by id.
    pub fn feature(&self, id: &str) -> Option<&FeatureDescriptor> {
        self.feature_index(id)
            .and_then(|index| self.features.get(index))
    }
}

/// Borrowed feature vector plus schema and per-feature quality flags.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct FeatureVectorView<'a> {
    /// Schema describing value order and semantics.
    pub schema: &'a FeatureSchema,
    /// Feature values in schema order.
    pub values: &'a [f64],
    /// Per-feature quality flags in schema order.
    pub quality: &'a [FeatureQualityFlags],
    /// Event/inference timestamp in nanoseconds.
    pub timestamp_ns: u64,
}

impl<'a> FeatureVectorView<'a> {
    /// Creates a borrowed feature vector view.
    pub const fn new(
        schema: &'a FeatureSchema,
        values: &'a [f64],
        quality: &'a [FeatureQualityFlags],
        timestamp_ns: u64,
    ) -> Self {
        Self {
            schema,
            values,
            quality,
            timestamp_ns,
        }
    }

    /// Returns a feature value by id.
    pub fn value(&self, id: &str) -> Option<f64> {
        self.schema
            .feature_index(id)
            .and_then(|index| self.values.get(index).copied())
    }

    /// Returns feature quality flags by id.
    pub fn quality(&self, id: &str) -> Option<FeatureQualityFlags> {
        self.schema
            .feature_index(id)
            .and_then(|index| self.quality.get(index).copied())
    }

    /// Validates this feature vector against its schema.
    pub fn validate(&self, now_ns: Option<u64>) -> FeatureVectorValidationReport {
        validate_feature_vector(self, now_ns)
    }
}

/// One feature-vector validation issue.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum FeatureVectorValidationIssue {
    /// Value/quality slices do not match schema length.
    LengthMismatch {
        /// Number of descriptors in the schema.
        expected: usize,
        /// Number of values supplied.
        values: usize,
        /// Number of quality entries supplied.
        quality: usize,
    },
    /// A feature was missing while its descriptor requires a value.
    MissingFeature {
        /// Feature index.
        index: usize,
        /// Feature id.
        feature_id: String,
    },
    /// Feature vector timestamp exceeded the descriptor freshness limit.
    StaleFeature {
        /// Feature index.
        index: usize,
        /// Feature id.
        feature_id: String,
        /// Observed age in nanoseconds.
        age_ns: u64,
        /// Accepted freshness in nanoseconds.
        freshness_ns: u64,
    },
    /// Feature value was outside the accepted descriptor range.
    OutOfRange {
        /// Feature index.
        index: usize,
        /// Feature id.
        feature_id: String,
        /// Feature value.
        value: f64,
        /// Minimum accepted value.
        min: Option<f64>,
        /// Maximum accepted value.
        max: Option<f64>,
    },
}

/// Validation report for a feature vector.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureVectorValidationReport {
    /// Whether validation passed.
    pub valid: bool,
    /// Validation issues.
    pub issues: Vec<FeatureVectorValidationIssue>,
    /// Aggregate quality flags observed in the vector.
    pub aggregate_quality: FeatureQualityFlags,
}

impl FeatureVectorValidationReport {
    /// Returns `true` when validation found issues.
    pub const fn has_errors(&self) -> bool {
        !self.valid
    }
}

/// Validates a feature vector view against schema metadata.
pub fn validate_feature_vector(
    view: &FeatureVectorView<'_>,
    now_ns: Option<u64>,
) -> FeatureVectorValidationReport {
    let expected = view.schema.features.len();
    let mut issues = Vec::new();
    let mut aggregate_quality = FeatureQualityFlags::NONE;

    if view.values.len() != expected || view.quality.len() != expected {
        issues.push(FeatureVectorValidationIssue::LengthMismatch {
            expected,
            values: view.values.len(),
            quality: view.quality.len(),
        });
    }

    for (index, descriptor) in view.schema.features.iter().enumerate() {
        let quality = view
            .quality
            .get(index)
            .copied()
            .unwrap_or(FeatureQualityFlags::MISSING);
        aggregate_quality |= quality;

        if quality.contains(FeatureQualityFlags::MISSING)
            && descriptor.missing_policy == FeatureMissingPolicy::Reject
        {
            issues.push(FeatureVectorValidationIssue::MissingFeature {
                index,
                feature_id: descriptor.id.clone(),
            });
        }

        if let Some(value) = view.values.get(index).copied() {
            let below_min = descriptor.min_value.is_some_and(|min| value < min);
            let above_max = descriptor.max_value.is_some_and(|max| value > max);
            if below_min || above_max {
                issues.push(FeatureVectorValidationIssue::OutOfRange {
                    index,
                    feature_id: descriptor.id.clone(),
                    value,
                    min: descriptor.min_value,
                    max: descriptor.max_value,
                });
                aggregate_quality |= FeatureQualityFlags::OUT_OF_RANGE;
            }
        }

        if let (Some(now_ns), Some(freshness_ns)) = (now_ns, descriptor.freshness_ns) {
            let age_ns = now_ns.saturating_sub(view.timestamp_ns);
            if age_ns > freshness_ns {
                issues.push(FeatureVectorValidationIssue::StaleFeature {
                    index,
                    feature_id: descriptor.id.clone(),
                    age_ns,
                    freshness_ns,
                });
                aggregate_quality |= FeatureQualityFlags::STALE;
            }
        }
    }

    FeatureVectorValidationReport {
        valid: issues.is_empty(),
        issues,
        aggregate_quality,
    }
}
