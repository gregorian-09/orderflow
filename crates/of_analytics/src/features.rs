use super::*;

/// Stable feature identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureId {
    raw: u32,
}

impl FeatureId {
    /// Creates a feature id.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidFeature`] when id is zero.
    pub const fn new(raw: u32) -> Result<Self, AnalyticsError> {
        if raw == 0 {
            return Err(AnalyticsError::InvalidFeature);
        }
        Ok(Self { raw })
    }

    /// Returns raw feature id.
    pub const fn raw(&self) -> u32 {
        self.raw
    }
}

/// Feature value unit.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FeatureUnit {
    /// Unitless raw value.
    Raw = 0,
    /// Integer-normalized price units.
    Price = 1,
    /// Quantity units.
    Quantity = 2,
    /// Basis points.
    BasisPoints = 3,
    /// Parts per million.
    PartsPerMillion = 4,
    /// Boolean encoded as zero or one.
    Boolean = 5,
    /// Nanoseconds.
    Nanoseconds = 6,
    /// Score in basis points, usually 0..=10,000.
    ScoreBasisPoints = 7,
}

impl FeatureUnit {
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Raw => 0,
            Self::Price => 1,
            Self::Quantity => 2,
            Self::BasisPoints => 3,
            Self::PartsPerMillion => 4,
            Self::Boolean => 5,
            Self::Nanoseconds => 6,
            Self::ScoreBasisPoints => 7,
        }
    }
}

/// Per-feature extraction quality.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FeatureQuality {
    /// Feature value is present and usable.
    Good = 0,
    /// Feature value is missing.
    Missing = 1,
    /// Feature was extracted from stale input.
    Stale = 2,
    /// Feature was extracted from degraded input.
    Degraded = 3,
    /// Feature value failed validation.
    Invalid = 4,
}

/// Missing-feature fill policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MissingValuePolicy {
    /// Fill missing integer feature values with zero.
    Zero,
    /// Fill missing values with an explicit sentinel.
    Sentinel(i64),
    /// Reuse the last known value when the host maintains one.
    LastKnown,
}

impl MissingValuePolicy {
    /// Returns deterministic integer fill value for this policy.
    pub const fn fill_value(&self) -> i64 {
        match self {
            Self::Zero | Self::LastKnown => 0,
            Self::Sentinel(value) => *value,
        }
    }

    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::Sentinel(_) => 1,
            Self::LastKnown => 2,
        }
    }
}

/// Feature definition inside a stable schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureDefinition {
    id: FeatureId,
    name: &'static str,
    unit: FeatureUnit,
    scale: i32,
    missing_policy: MissingValuePolicy,
}

impl FeatureDefinition {
    /// Creates a feature definition.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidFeature`] when the name is empty.
    pub const fn new(
        id: FeatureId,
        name: &'static str,
        unit: FeatureUnit,
        scale: i32,
        missing_policy: MissingValuePolicy,
    ) -> Result<Self, AnalyticsError> {
        if name.is_empty() {
            return Err(AnalyticsError::InvalidFeature);
        }
        Ok(Self {
            id,
            name,
            unit,
            scale,
            missing_policy,
        })
    }

    /// Returns feature id.
    pub const fn id(&self) -> FeatureId {
        self.id
    }

    /// Returns stable feature name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns feature unit.
    pub const fn unit(&self) -> FeatureUnit {
        self.unit
    }

    /// Returns feature scale.
    pub const fn scale(&self) -> i32 {
        self.scale
    }

    /// Returns missing-value policy.
    pub const fn missing_policy(&self) -> MissingValuePolicy {
        self.missing_policy
    }
}

/// Fixed-capacity feature schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureSchema<const N: usize> {
    definitions: [Option<FeatureDefinition>; N],
    len: usize,
}

impl<const N: usize> FeatureSchema<N> {
    /// Creates an empty feature schema.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidFeature`] when capacity is zero.
    pub const fn new() -> Result<Self, AnalyticsError> {
        if N == 0 {
            return Err(AnalyticsError::InvalidFeature);
        }
        Ok(Self {
            definitions: [None; N],
            len: 0,
        })
    }

    /// Registers a feature definition at the next stable index.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidFeature`] when capacity is full or the
    /// id is already registered.
    pub fn register(&mut self, definition: FeatureDefinition) -> Result<usize, AnalyticsError> {
        if self.len >= N || self.contains_id(definition.id()) {
            return Err(AnalyticsError::InvalidFeature);
        }
        let index = self.len;
        self.definitions[index] = Some(definition);
        self.len += 1;
        Ok(index)
    }

    /// Returns registered feature count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when no features are registered.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns definition by stable index.
    pub const fn definition(&self, index: usize) -> Option<FeatureDefinition> {
        if index >= self.len {
            None
        } else {
            self.definitions[index]
        }
    }

    /// Returns true when a feature id is registered.
    pub fn contains_id(&self, id: FeatureId) -> bool {
        self.index_of(id).is_some()
    }

    /// Returns stable index for a feature id.
    pub fn index_of(&self, id: FeatureId) -> Option<usize> {
        for index in 0..self.len {
            if let Some(definition) = self.definitions[index] {
                if definition.id() == id {
                    return Some(index);
                }
            }
        }
        None
    }

    /// Returns deterministic schema hash.
    pub fn schema_hash(&self) -> u64 {
        let mut hash = fnv1a64(0xcbf2_9ce4_8422_2325, &(self.len as u64).to_le_bytes());
        for index in 0..self.len {
            if let Some(definition) = self.definitions[index] {
                hash = hash_feature_definition(hash, definition);
            }
        }
        hash
    }
}

/// Fixed-capacity feature registry alias.
pub type FeatureRegistry<const N: usize> = FeatureSchema<N>;

/// Completed fixed-capacity feature vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureVector<const N: usize> {
    values: [i64; N],
    qualities: [FeatureQuality; N],
    len: usize,
    schema_hash: u64,
}

impl<const N: usize> FeatureVector<N> {
    /// Returns feature values in schema order.
    pub fn values(&self) -> &[i64] {
        &self.values[..self.len]
    }

    /// Returns feature qualities in schema order.
    pub fn qualities(&self) -> &[FeatureQuality] {
        &self.qualities[..self.len]
    }

    /// Returns feature count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true when the vector is empty.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns schema hash used to produce this vector.
    pub const fn schema_hash(&self) -> u64 {
        self.schema_hash
    }

    /// Returns feature value by stable index.
    pub fn value(&self, index: usize) -> Option<i64> {
        self.values().get(index).copied()
    }

    /// Returns feature quality by stable index.
    pub fn quality(&self, index: usize) -> Option<FeatureQuality> {
        self.qualities().get(index).copied()
    }
}

/// Reusable fixed-capacity feature-vector writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureVectorWriter<const N: usize> {
    values: [i64; N],
    qualities: [FeatureQuality; N],
    len: usize,
    schema_hash: u64,
}

impl<const N: usize> FeatureVectorWriter<N> {
    /// Creates a writer initialized from a schema.
    pub fn new(schema: &FeatureSchema<N>) -> Self {
        let mut writer = Self {
            values: [0; N],
            qualities: [FeatureQuality::Missing; N],
            len: schema.len(),
            schema_hash: schema.schema_hash(),
        };
        writer.reset(schema);
        writer
    }

    /// Resets the writer from schema missing-value policies.
    pub fn reset(&mut self, schema: &FeatureSchema<N>) {
        self.len = schema.len();
        self.schema_hash = schema.schema_hash();
        for index in 0..self.len {
            if let Some(definition) = schema.definition(index) {
                self.values[index] = definition.missing_policy().fill_value();
                self.qualities[index] = FeatureQuality::Missing;
            }
        }
    }

    /// Sets a feature value by stable index.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError::InvalidFeature`] when index is outside the
    /// active schema length.
    pub fn set(
        &mut self,
        index: usize,
        value: i64,
        quality: FeatureQuality,
    ) -> Result<(), AnalyticsError> {
        if index >= self.len {
            return Err(AnalyticsError::InvalidFeature);
        }
        self.values[index] = value;
        self.qualities[index] = quality;
        Ok(())
    }

    /// Finishes the current vector.
    pub const fn finish(&self) -> FeatureVector<N> {
        FeatureVector {
            values: self.values,
            qualities: self.qualities,
            len: self.len,
            schema_hash: self.schema_hash,
        }
    }
}

/// Feature extractor contract.
pub trait FeatureExtractor<const N: usize> {
    /// Input consumed by the extractor.
    type Input;

    /// Returns extractor schema.
    fn schema(&self) -> &FeatureSchema<N>;

    /// Extracts a feature vector into a caller-owned writer.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsError`] when the input cannot be converted into a
    /// valid vector.
    fn extract(
        &mut self,
        input: Self::Input,
        writer: &mut FeatureVectorWriter<N>,
    ) -> Result<FeatureVector<N>, AnalyticsError>;
}
