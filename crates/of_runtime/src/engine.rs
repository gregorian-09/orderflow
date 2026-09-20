use std::collections::{BTreeMap, HashMap, HashSet};
pub(crate) use std::error::Error;
pub(crate) use std::fmt;
pub(crate) use std::fs::{self, create_dir_all, OpenOptions};
pub(crate) use std::io::Write;
pub(crate) use std::path::{Path, PathBuf};

pub(crate) use crate::config::config_hash;
pub(crate) use crate::validate_startup_config;
pub(crate) use of_adapters::{
    adapter_descriptors, create_adapter, describe_adapter, AdapterConfig, AdapterConnectionState,
    AdapterDescriptor, AdapterHealth, AdapterOperationalStatus, AdapterRuntimeMode,
    MarketDataAdapter, ProviderKind, RawEvent, SubscribeReq,
};
#[cfg(feature = "tickbar")]
pub(crate) use of_core::CompletedBar;
pub(crate) use of_core::{
    compute_book_analytics, compute_depth_slope, compute_effective_spread_bps,
    compute_lob_features, compute_mid_price, compute_weighted_average_price, ACDModel, ACDSnapshot,
    AgentTypeDetector, AgentTypeSnapshot, AlmgrenChriss, AlmgrenChrissSnapshot, AmihudSnapshot,
    AmihudTracker, AnalyticsAccumulator, AnalyticsConfig, AnalyticsSnapshot, BookAnalyticsSnapshot,
    BookEventAnalyticsSnapshot, BookEventTracker, BookLevel, BookSnapshot, BookUpdate,
    ClassificationVote, CvdEnhancementSnapshot, CvdEnhancements, DarkLitCorrelationSnapshot,
    DarkLitCorrelator, DarkPoolSnapshot, DarkPoolTracker, DataQualityFlags,
    DerivedAnalyticsSnapshot, FuturesSnapshot, FuturesTracker, HasbrouckSnapshot, HasbrouckVAR,
    InstitutionalFlowSnapshot, InstitutionalFlowTracker, IntervalCandleSnapshot,
    KineticEnergySnapshot, KineticEnergyTracker, KyleLambdaSnapshot, KyleLambdaTracker,
    LOBFeatureSnapshot, MicrostructureNoise, NoiseSnapshot, OIAnalysisSnapshot, OIAnalyzer,
    OptionsFlowSnapshot, OptionsFlowTracker, PatternDetector, PatternSnapshot, RegimeDetector,
    RegimeSnapshot, ResiliencySnapshot, ResiliencyTracker, SessionCandleSnapshot, SignalSnapshot,
    SignalState, SpreadDecomposition, SpreadDecompositionSnapshot, SpreadTracker, SymbolId,
    TradeClassifier, TradePrint, VolatilityEstimator, VolatilitySignature,
    VolatilitySignatureSnapshot, VolatilitySnapshot, VpinSnapshot, VpinTracker,
};
pub(crate) use of_persist::{
    decode_normalized_market_data_record, BoundedMarketDataWalWriter,
    BoundedMarketDataWriterConfig, BoundedMarketDataWriterMetrics,
    MarketDataPersistenceFailureAction, MarketDataPersistenceHealth, MarketDataPersistenceMode,
    MarketDataPersistencePolicy, MarketDataWalProducer, MarketDataWalRecord,
    NormalizedMarketDataRecordInput, RetentionPolicy, RollingStore, SegmentedMarketDataWalConfig,
};
pub(crate) use of_signals::{
    SignalExplanation, SignalGateDecision, SignalModule, SignalReasonCode,
};

#[path = "engine_core.rs"]
mod core;
#[path = "engine_diagnostics.rs"]
mod diagnostics;
#[path = "engine_types.rs"]
mod types;

pub use core::*;
pub(crate) use diagnostics::*;
pub use types::*;
