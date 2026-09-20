use super::*;

/// Advanced analytics error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalyticsError {
    /// Quote prices, sizes, or timestamps are invalid.
    InvalidQuote,
    /// Trade price, size, side, or timestamp is invalid.
    InvalidTrade,
    /// Depth configuration or book levels are invalid.
    InvalidDepth,
    /// Feed-quality event fields are invalid.
    InvalidQuality,
    /// Feature schema, feature id, or vector index is invalid.
    InvalidFeature,
    /// Resiliency configuration or sample fields are invalid.
    InvalidResiliency,
    /// Queue/fill probability configuration or event fields are invalid.
    InvalidQueue,
    /// Pattern-risk configuration or input fields are invalid.
    InvalidPattern,
    /// Venue/route analytics configuration or event fields are invalid.
    InvalidRoute,
    /// Cross-asset analytics configuration or sample fields are invalid.
    InvalidCrossAsset,
    /// Derivatives analytics configuration or sample fields are invalid.
    InvalidDerivative,
    /// Execution-quality benchmark fields are invalid.
    InvalidExecution,
    /// Requested analytics require a quote but no quote is available.
    MissingQuote,
}

impl fmt::Display for AnalyticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuote => write!(f, "invalid quote context"),
            Self::InvalidTrade => write!(f, "invalid trade context"),
            Self::InvalidDepth => write!(f, "invalid depth context"),
            Self::InvalidQuality => write!(f, "invalid feed quality context"),
            Self::InvalidFeature => write!(f, "invalid feature vector context"),
            Self::InvalidResiliency => write!(f, "invalid resiliency context"),
            Self::InvalidQueue => write!(f, "invalid queue/fill context"),
            Self::InvalidPattern => write!(f, "invalid pattern risk context"),
            Self::InvalidRoute => write!(f, "invalid venue/route context"),
            Self::InvalidCrossAsset => write!(f, "invalid cross-asset context"),
            Self::InvalidDerivative => write!(f, "invalid derivatives context"),
            Self::InvalidExecution => write!(f, "invalid execution quality context"),
            Self::MissingQuote => write!(f, "missing quote context"),
        }
    }
}

impl Error for AnalyticsError {}
