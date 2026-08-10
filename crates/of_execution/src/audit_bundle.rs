//! Bounded, integrity-verifiable incident audit bundle export.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ExecutionAuditBundleManifest, ExecutionMetrics, ExecutionRunbookSnapshot};

const MANIFEST_FILE: &str = "manifest.json";
const MANIFEST_DIGEST_FILE: &str = "manifest.sha256";
const BUNDLE_SCHEMA_VERSION: u16 = 1;
const MAX_INCIDENT_ID_BYTES: usize = 64;
const MAX_SOURCE_LABEL_BYTES: usize = 128;
const MAX_BUNDLE_PATH_BYTES: usize = 240;

/// Evidence class stored in an execution incident bundle.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionAuditArtifactKind {
    /// One or more execution write-ahead-log segments.
    ExecutionWal,
    /// Latest execution checkpoint covering the incident range.
    ExecutionCheckpoint,
    /// Recovery plan, execution result, or integrity report.
    RecoveryReport,
    /// Venue or drop-copy reconciliation report.
    ReconciliationReport,
    /// Redacted route configuration.
    RouteConfig,
    /// Redacted risk configuration.
    RiskConfig,
    /// Adapter health snapshot or transition history.
    AdapterHealth,
    /// Execution metrics and service-level objective snapshot.
    ExecutionMetrics,
    /// Relevant normalized or raw market-data WAL range.
    MarketDataWal,
    /// Strategy intent and parent/child lineage records.
    StrategyIntent,
    /// Drop-copy execution records.
    DropCopy,
    /// Version, build, dependency, and deployment metadata.
    BuildMetadata,
    /// Operator-command audit records.
    OperatorAudit,
    /// Deployment-specific evidence not represented by another kind.
    Other,
}

impl ExecutionAuditArtifactKind {
    /// Evidence classes required by the production incident profile.
    pub const PRODUCTION_REQUIRED: [Self; 12] = [
        Self::ExecutionWal,
        Self::ExecutionCheckpoint,
        Self::RecoveryReport,
        Self::ReconciliationReport,
        Self::RouteConfig,
        Self::RiskConfig,
        Self::AdapterHealth,
        Self::ExecutionMetrics,
        Self::MarketDataWal,
        Self::StrategyIntent,
        Self::DropCopy,
        Self::BuildMetadata,
    ];

    /// Returns the stable manifest spelling for this evidence class.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExecutionWal => "execution_wal",
            Self::ExecutionCheckpoint => "execution_checkpoint",
            Self::RecoveryReport => "recovery_report",
            Self::ReconciliationReport => "reconciliation_report",
            Self::RouteConfig => "route_config",
            Self::RiskConfig => "risk_config",
            Self::AdapterHealth => "adapter_health",
            Self::ExecutionMetrics => "execution_metrics",
            Self::MarketDataWal => "market_data_wal",
            Self::StrategyIntent => "strategy_intent",
            Self::DropCopy => "drop_copy",
            Self::BuildMetadata => "build_metadata",
            Self::OperatorAudit => "operator_audit",
            Self::Other => "other",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "execution_wal" => Self::ExecutionWal,
            "execution_checkpoint" => Self::ExecutionCheckpoint,
            "recovery_report" => Self::RecoveryReport,
            "reconciliation_report" => Self::ReconciliationReport,
            "route_config" => Self::RouteConfig,
            "risk_config" => Self::RiskConfig,
            "adapter_health" => Self::AdapterHealth,
            "execution_metrics" => Self::ExecutionMetrics,
            "market_data_wal" => Self::MarketDataWal,
            "strategy_intent" => Self::StrategyIntent,
            "drop_copy" => Self::DropCopy,
            "build_metadata" => Self::BuildMetadata,
            "operator_audit" => Self::OperatorAudit,
            "other" => Self::Other,
            _ => return None,
        })
    }
}

impl fmt::Display for ExecutionAuditArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Evidence-coverage policy applied to an audit bundle.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionAuditBundleProfile {
    /// Require every production incident evidence class.
    #[default]
    ProductionIncident,
    /// Require only artifacts explicitly marked as required.
    Custom,
}

impl ExecutionAuditBundleProfile {
    /// Returns the stable manifest spelling for this profile.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProductionIncident => "production_incident",
            Self::Custom => "custom",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "production_incident" => Some(Self::ProductionIncident),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Inclusive incident time range in Unix nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionAuditTimeRange {
    start_ns: u64,
    end_ns: u64,
}

impl ExecutionAuditTimeRange {
    /// Creates an inclusive incident time range.
    ///
    /// Range ordering is validated when the bundle is exported.
    pub const fn new(start_ns: u64, end_ns: u64) -> Self {
        Self { start_ns, end_ns }
    }

    /// Returns the inclusive range start in Unix nanoseconds.
    pub const fn start_ns(self) -> u64 {
        self.start_ns
    }

    /// Returns the inclusive range end in Unix nanoseconds.
    pub const fn end_ns(self) -> u64 {
        self.end_ns
    }
}

#[derive(Debug, Clone)]
enum ExecutionAuditArtifactSource {
    File(PathBuf),
    Bytes(Vec<u8>),
}

/// One file or in-memory payload requested for an audit bundle.
#[derive(Debug, Clone)]
pub struct ExecutionAuditArtifact {
    kind: ExecutionAuditArtifactKind,
    bundle_path: PathBuf,
    source_label: String,
    required: bool,
    source: ExecutionAuditArtifactSource,
}

impl ExecutionAuditArtifact {
    /// Creates an artifact copied from an existing regular file.
    pub fn from_file(
        kind: ExecutionAuditArtifactKind,
        source_path: impl Into<PathBuf>,
        bundle_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            kind,
            bundle_path: bundle_path.into(),
            source_label: kind.as_str().to_string(),
            required: true,
            source: ExecutionAuditArtifactSource::File(source_path.into()),
        }
    }

    /// Creates an artifact from caller-owned bytes.
    ///
    /// The bytes are retained by the request and copied during export. This is
    /// intended for small control-plane snapshots such as redacted config or
    /// build metadata, not live order-path payloads.
    pub fn from_bytes(
        kind: ExecutionAuditArtifactKind,
        bytes: impl Into<Vec<u8>>,
        bundle_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            kind,
            bundle_path: bundle_path.into(),
            source_label: kind.as_str().to_string(),
            required: true,
            source: ExecutionAuditArtifactSource::Bytes(bytes.into()),
        }
    }

    /// Sets a non-sensitive logical source label recorded in the manifest.
    ///
    /// Source filesystem paths are deliberately never written to the manifest.
    pub fn with_source_label(mut self, source_label: impl Into<String>) -> Self {
        self.source_label = source_label.into();
        self
    }

    /// Marks this artifact optional for a custom bundle.
    ///
    /// A production profile still requires at least one present artifact for
    /// each class in [`ExecutionAuditArtifactKind::PRODUCTION_REQUIRED`].
    pub const fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Returns the evidence class.
    pub const fn kind(&self) -> ExecutionAuditArtifactKind {
        self.kind
    }

    /// Returns the relative destination path inside the bundle.
    pub fn bundle_path(&self) -> &Path {
        &self.bundle_path
    }

    /// Returns the non-sensitive logical source label.
    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    /// Returns whether absence must fail export.
    pub const fn is_required(&self) -> bool {
        self.required
    }
}

/// Caller-built request for one incident audit bundle.
#[derive(Debug, Clone)]
pub struct ExecutionAuditBundleRequest {
    incident_id: String,
    generated_ns: u64,
    time_range: ExecutionAuditTimeRange,
    profile: ExecutionAuditBundleProfile,
    execution_manifest: Option<ExecutionAuditBundleManifest>,
    artifacts: Vec<ExecutionAuditArtifact>,
}

impl ExecutionAuditBundleRequest {
    /// Creates a production-profile request with no artifacts.
    pub fn new(
        incident_id: impl Into<String>,
        generated_ns: u64,
        time_range: ExecutionAuditTimeRange,
    ) -> Self {
        Self {
            incident_id: incident_id.into(),
            generated_ns,
            time_range,
            profile: ExecutionAuditBundleProfile::ProductionIncident,
            execution_manifest: None,
            artifacts: Vec::new(),
        }
    }

    /// Selects the bundle evidence profile.
    pub const fn with_profile(mut self, profile: ExecutionAuditBundleProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Attaches the engine's read-only incident manifest.
    pub const fn with_execution_manifest(mut self, manifest: ExecutionAuditBundleManifest) -> Self {
        self.execution_manifest = Some(manifest);
        self
    }

    /// Appends one artifact while preserving caller order in the manifest.
    pub fn with_artifact(mut self, artifact: ExecutionAuditArtifact) -> Self {
        self.artifacts.push(artifact);
        self
    }

    /// Appends one artifact while preserving caller order in the manifest.
    pub fn push_artifact(&mut self, artifact: ExecutionAuditArtifact) {
        self.artifacts.push(artifact);
    }

    /// Returns the incident identifier.
    pub fn incident_id(&self) -> &str {
        &self.incident_id
    }

    /// Returns the manifest creation timestamp in Unix nanoseconds.
    pub const fn generated_ns(&self) -> u64 {
        self.generated_ns
    }

    /// Returns the inclusive incident time range.
    pub const fn time_range(&self) -> ExecutionAuditTimeRange {
        self.time_range
    }

    /// Returns the evidence profile.
    pub const fn profile(&self) -> ExecutionAuditBundleProfile {
        self.profile
    }

    /// Returns requested artifacts in deterministic manifest order.
    pub fn artifacts(&self) -> &[ExecutionAuditArtifact] {
        &self.artifacts
    }
}

/// Filesystem and capacity policy for audit bundle export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionAuditBundleConfig {
    root: PathBuf,
    max_artifacts: usize,
    max_artifact_bytes: u64,
    max_total_bytes: u64,
    max_manifest_bytes: u64,
    copy_buffer_bytes: usize,
    sync_on_write: bool,
}

impl ExecutionAuditBundleConfig {
    /// Default maximum number of manifest artifact entries.
    pub const DEFAULT_MAX_ARTIFACTS: usize = 256;
    /// Default maximum bytes for one artifact (4 GiB).
    pub const DEFAULT_MAX_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
    /// Default maximum bytes across present artifacts (32 GiB).
    pub const DEFAULT_MAX_TOTAL_BYTES: u64 = 32 * 1024 * 1024 * 1024;
    /// Default maximum encoded manifest bytes (4 MiB).
    pub const DEFAULT_MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;
    /// Default streaming copy buffer bytes.
    pub const DEFAULT_COPY_BUFFER_BYTES: usize = 64 * 1024;

    /// Creates a conservative, durability-enabled export policy.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            max_artifacts: Self::DEFAULT_MAX_ARTIFACTS,
            max_artifact_bytes: Self::DEFAULT_MAX_ARTIFACT_BYTES,
            max_total_bytes: Self::DEFAULT_MAX_TOTAL_BYTES,
            max_manifest_bytes: Self::DEFAULT_MAX_MANIFEST_BYTES,
            copy_buffer_bytes: Self::DEFAULT_COPY_BUFFER_BYTES,
            sync_on_write: true,
        }
    }

    /// Sets artifact-count and byte ceilings.
    pub const fn with_limits(
        mut self,
        max_artifacts: usize,
        max_artifact_bytes: u64,
        max_total_bytes: u64,
    ) -> Self {
        self.max_artifacts = max_artifacts;
        self.max_artifact_bytes = max_artifact_bytes;
        self.max_total_bytes = max_total_bytes;
        self
    }

    /// Sets the encoded manifest byte ceiling.
    pub const fn with_max_manifest_bytes(mut self, max_manifest_bytes: u64) -> Self {
        self.max_manifest_bytes = max_manifest_bytes;
        self
    }

    /// Sets the streaming copy buffer size.
    pub const fn with_copy_buffer_bytes(mut self, copy_buffer_bytes: usize) -> Self {
        self.copy_buffer_bytes = copy_buffer_bytes;
        self
    }

    /// Sets whether files and the destination directory are synchronized.
    pub const fn with_sync_on_write(mut self, sync_on_write: bool) -> Self {
        self.sync_on_write = sync_on_write;
        self
    }

    /// Returns the destination root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the maximum manifest artifact count.
    pub const fn max_artifacts(&self) -> usize {
        self.max_artifacts
    }

    /// Returns the maximum bytes for one artifact.
    pub const fn max_artifact_bytes(&self) -> u64 {
        self.max_artifact_bytes
    }

    /// Returns the maximum aggregate artifact bytes.
    pub const fn max_total_bytes(&self) -> u64 {
        self.max_total_bytes
    }

    /// Returns the maximum encoded manifest bytes.
    pub const fn max_manifest_bytes(&self) -> u64 {
        self.max_manifest_bytes
    }

    /// Returns the streaming copy buffer size.
    pub const fn copy_buffer_bytes(&self) -> usize {
        self.copy_buffer_bytes
    }

    /// Returns whether durable file and directory synchronization is enabled.
    pub const fn sync_on_write(&self) -> bool {
        self.sync_on_write
    }
}

/// Successful audit bundle installation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionAuditBundleReport {
    bundle_path: PathBuf,
    manifest_path: PathBuf,
    incident_id: String,
    profile: ExecutionAuditBundleProfile,
    artifact_count: usize,
    present_artifact_count: usize,
    missing_optional_count: usize,
    total_artifact_bytes: u64,
    manifest_sha256: [u8; 32],
}

impl ExecutionAuditBundleReport {
    /// Returns the atomically installed bundle directory.
    pub fn bundle_path(&self) -> &Path {
        &self.bundle_path
    }

    /// Returns the JSON manifest path.
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    /// Returns the incident identifier.
    pub fn incident_id(&self) -> &str {
        &self.incident_id
    }

    /// Returns the evidence profile.
    pub const fn profile(&self) -> ExecutionAuditBundleProfile {
        self.profile
    }

    /// Returns the number of manifest artifact entries.
    pub const fn artifact_count(&self) -> usize {
        self.artifact_count
    }

    /// Returns the number of copied or generated artifacts.
    pub const fn present_artifact_count(&self) -> usize {
        self.present_artifact_count
    }

    /// Returns the number of absent optional artifacts.
    pub const fn missing_optional_count(&self) -> usize {
        self.missing_optional_count
    }

    /// Returns aggregate bytes across present artifacts.
    pub const fn total_artifact_bytes(&self) -> u64 {
        self.total_artifact_bytes
    }

    /// Returns the SHA-256 digest of the exact JSON manifest bytes.
    pub const fn manifest_sha256(&self) -> [u8; 32] {
        self.manifest_sha256
    }
}

/// Successful independent bundle verification result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionAuditBundleVerification {
    bundle_path: PathBuf,
    incident_id: String,
    profile: ExecutionAuditBundleProfile,
    artifact_count: usize,
    present_artifact_count: usize,
    missing_optional_count: usize,
    total_artifact_bytes: u64,
    manifest_sha256: [u8; 32],
}

impl ExecutionAuditBundleVerification {
    /// Returns the verified bundle directory.
    pub fn bundle_path(&self) -> &Path {
        &self.bundle_path
    }

    /// Returns the verified incident identifier.
    pub fn incident_id(&self) -> &str {
        &self.incident_id
    }

    /// Returns the verified evidence profile.
    pub const fn profile(&self) -> ExecutionAuditBundleProfile {
        self.profile
    }

    /// Returns the number of verified manifest artifact entries.
    pub const fn artifact_count(&self) -> usize {
        self.artifact_count
    }

    /// Returns the number of verified present artifacts.
    pub const fn present_artifact_count(&self) -> usize {
        self.present_artifact_count
    }

    /// Returns the number of verified absent optional artifacts.
    pub const fn missing_optional_count(&self) -> usize {
        self.missing_optional_count
    }

    /// Returns aggregate verified artifact bytes.
    pub const fn total_artifact_bytes(&self) -> u64 {
        self.total_artifact_bytes
    }

    /// Returns the verified SHA-256 digest of the manifest bytes.
    pub const fn manifest_sha256(&self) -> [u8; 32] {
        self.manifest_sha256
    }
}

/// Audit bundle validation, export, or verification error.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionAuditBundleError {
    /// Export configuration is internally inconsistent.
    InvalidConfig(&'static str),
    /// Incident identifier is empty, too long, or not filename-safe ASCII.
    InvalidIncidentId,
    /// Incident range ends before it starts.
    InvalidTimeRange,
    /// Artifact path is absolute, non-portable, too long, or contains traversal.
    InvalidBundlePath(PathBuf),
    /// Artifact path collides with a reserved manifest path.
    ReservedBundlePath(PathBuf),
    /// Two artifact paths collide, including case-insensitive platforms.
    DuplicateBundlePath(PathBuf),
    /// Artifact count exceeds the configured ceiling.
    ArtifactCapacityExceeded {
        /// Observed entry count.
        actual: usize,
        /// Configured entry ceiling.
        maximum: usize,
    },
    /// One artifact exceeds the configured byte ceiling.
    ArtifactTooLarge {
        /// Logical artifact or filesystem path.
        path: PathBuf,
        /// Observed byte count.
        actual: u64,
        /// Configured per-artifact byte ceiling.
        maximum: u64,
    },
    /// Aggregate artifact bytes exceed the configured ceiling.
    BundleTooLarge {
        /// Observed or conservatively saturated aggregate byte count.
        actual: u64,
        /// Configured aggregate byte ceiling.
        maximum: u64,
    },
    /// An explicitly required source file is absent.
    MissingRequiredArtifact {
        /// Evidence class whose source is missing.
        kind: ExecutionAuditArtifactKind,
        /// Missing source or logical artifact path.
        path: PathBuf,
    },
    /// The production profile is missing an evidence class.
    MissingProductionArtifact(ExecutionAuditArtifactKind),
    /// Source or bundle entry is a symbolic link.
    SymbolicLink(PathBuf),
    /// Source or bundle entry is not a regular file.
    NotRegularFile(PathBuf),
    /// Final bundle destination already exists.
    DestinationExists(PathBuf),
    /// Manifest is missing, malformed, inconsistent, or unsupported.
    InvalidManifest(String),
    /// A payload or manifest digest differs from its recorded digest.
    DigestMismatch {
        /// Manifest or artifact path whose bytes failed verification.
        path: PathBuf,
    },
    /// Filesystem operation failed.
    Io {
        /// Static filesystem operation description.
        operation: &'static str,
        /// Filesystem path involved in the failed operation.
        path: PathBuf,
        /// Operating-system error text.
        message: String,
    },
}

impl fmt::Display for ExecutionAuditBundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(reason) => write!(f, "invalid audit bundle config: {reason}"),
            Self::InvalidIncidentId => write!(f, "invalid audit bundle incident id"),
            Self::InvalidTimeRange => write!(f, "invalid audit bundle time range"),
            Self::InvalidBundlePath(path) => {
                write!(f, "invalid audit bundle path: {}", path.display())
            }
            Self::ReservedBundlePath(path) => {
                write!(f, "reserved audit bundle path: {}", path.display())
            }
            Self::DuplicateBundlePath(path) => {
                write!(f, "duplicate audit bundle path: {}", path.display())
            }
            Self::ArtifactCapacityExceeded { actual, maximum } => write!(
                f,
                "audit bundle artifact count {actual} exceeds maximum {maximum}"
            ),
            Self::ArtifactTooLarge {
                path,
                actual,
                maximum,
            } => write!(
                f,
                "audit artifact {} has {actual} bytes, maximum is {maximum}",
                path.display()
            ),
            Self::BundleTooLarge { actual, maximum } => write!(
                f,
                "audit bundle has {actual} artifact bytes, maximum is {maximum}"
            ),
            Self::MissingRequiredArtifact { kind, path } => write!(
                f,
                "required audit artifact {kind} is missing: {}",
                path.display()
            ),
            Self::MissingProductionArtifact(kind) => {
                write!(f, "production audit bundle is missing {kind}")
            }
            Self::SymbolicLink(path) => {
                write!(f, "symbolic links are not allowed: {}", path.display())
            }
            Self::NotRegularFile(path) => {
                write!(
                    f,
                    "audit artifact is not a regular file: {}",
                    path.display()
                )
            }
            Self::DestinationExists(path) => {
                write!(f, "audit bundle destination exists: {}", path.display())
            }
            Self::InvalidManifest(reason) => write!(f, "invalid audit bundle manifest: {reason}"),
            Self::DigestMismatch { path } => {
                write!(f, "audit bundle digest mismatch: {}", path.display())
            }
            Self::Io {
                operation,
                path,
                message,
            } => write!(
                f,
                "audit bundle {operation} failed for {}: {message}",
                path.display()
            ),
        }
    }
}

impl Error for ExecutionAuditBundleError {}

/// Control-plane exporter and verifier for execution incident bundles.
///
/// Export performs filesystem I/O and SHA-256 hashing. Call it from a bounded
/// operator worker after WAL rotation/checkpoint capture, never from an order,
/// market-data, or execution-report hot path.
#[derive(Debug, Clone)]
pub struct ExecutionAuditBundleExporter {
    config: ExecutionAuditBundleConfig,
}

impl ExecutionAuditBundleExporter {
    /// Creates an exporter without touching the filesystem.
    pub const fn new(config: ExecutionAuditBundleConfig) -> Self {
        Self { config }
    }

    /// Returns exporter configuration.
    pub const fn config(&self) -> &ExecutionAuditBundleConfig {
        &self.config
    }

    /// Exports, verifies, and atomically installs one bundle directory.
    ///
    /// Existing final destinations are never overwritten. Source filesystem
    /// paths are omitted from the manifest; callers provide non-sensitive
    /// logical labels and already-redacted config artifacts.
    ///
    /// # Errors
    ///
    /// Returns a typed validation, capacity, integrity, or filesystem error.
    pub fn export(
        &self,
        request: &ExecutionAuditBundleRequest,
    ) -> Result<ExecutionAuditBundleReport, ExecutionAuditBundleError> {
        validate_config(&self.config)?;
        validate_request(request, &self.config)?;
        prepare_root(&self.config.root)?;

        let final_path = self.config.root.join(format!(
            "orderflow-audit-{}-{}",
            request.incident_id, request.generated_ns
        ));
        if path_exists(&final_path)? {
            return Err(ExecutionAuditBundleError::DestinationExists(final_path));
        }

        let mut staging = StagingDirectory::create(&self.config.root, &final_path)?;
        let mut records = Vec::with_capacity(request.artifacts.len());
        let mut total_bytes = 0_u64;
        let mut copy_buffer = vec![0_u8; self.config.copy_buffer_bytes];

        for artifact in &request.artifacts {
            let path_string = portable_bundle_path(&artifact.bundle_path)?;
            let destination = staging.path().join(&artifact.bundle_path);
            let outcome = match &artifact.source {
                ExecutionAuditArtifactSource::File(source) => {
                    match inspect_source(source, artifact)? {
                        Some(metadata_len) => {
                            enforce_artifact_size(
                                &artifact.bundle_path,
                                metadata_len,
                                self.config.max_artifact_bytes,
                            )?;
                            enforce_total_size(
                                total_bytes,
                                metadata_len,
                                self.config.max_total_bytes,
                            )?;
                            let (bytes, digest) = copy_file(
                                source,
                                &destination,
                                &mut copy_buffer,
                                self.config.max_artifact_bytes,
                                self.config.max_total_bytes - total_bytes,
                                self.config.sync_on_write,
                            )?;
                            total_bytes =
                                checked_total(total_bytes, bytes, self.config.max_total_bytes)?;
                            ArtifactOutcome::Present { bytes, digest }
                        }
                        None => ArtifactOutcome::MissingOptional,
                    }
                }
                ExecutionAuditArtifactSource::Bytes(bytes) => {
                    let bytes_len = u64::try_from(bytes.len()).map_err(|_| {
                        ExecutionAuditBundleError::ArtifactTooLarge {
                            path: artifact.bundle_path.clone(),
                            actual: u64::MAX,
                            maximum: self.config.max_artifact_bytes,
                        }
                    })?;
                    enforce_artifact_size(
                        &artifact.bundle_path,
                        bytes_len,
                        self.config.max_artifact_bytes,
                    )?;
                    total_bytes =
                        checked_total(total_bytes, bytes_len, self.config.max_total_bytes)?;
                    let digest = write_bytes(&destination, bytes, self.config.sync_on_write)?;
                    ArtifactOutcome::Present {
                        bytes: bytes_len,
                        digest,
                    }
                }
            };

            records.push(ManifestArtifact::from_outcome(
                artifact,
                path_string,
                outcome,
            ));
        }

        validate_production_coverage(request.profile, &records)?;
        let document = BundleManifestV1::new(request, records, total_bytes);
        let manifest_bytes = encode_manifest(&document, self.config.max_manifest_bytes)?;
        let manifest_digest = digest_bytes(&manifest_bytes);
        write_bytes(
            &staging.path().join(MANIFEST_FILE),
            &manifest_bytes,
            self.config.sync_on_write,
        )?;
        let digest_line = format!("{}  {MANIFEST_FILE}\n", encode_hex(&manifest_digest));
        write_bytes(
            &staging.path().join(MANIFEST_DIGEST_FILE),
            digest_line.as_bytes(),
            self.config.sync_on_write,
        )?;

        if self.config.sync_on_write {
            sync_directory(staging.path())?;
        }
        let verification = self.verify_path(staging.path())?;
        fs::rename(staging.path(), &final_path)
            .map_err(|err| io_error("publish", &final_path, err))?;
        staging.disarm();
        if self.config.sync_on_write {
            sync_directory(&self.config.root)?;
        }

        Ok(ExecutionAuditBundleReport {
            bundle_path: final_path.clone(),
            manifest_path: final_path.join(MANIFEST_FILE),
            incident_id: verification.incident_id,
            profile: verification.profile,
            artifact_count: verification.artifact_count,
            present_artifact_count: verification.present_artifact_count,
            missing_optional_count: verification.missing_optional_count,
            total_artifact_bytes: verification.total_artifact_bytes,
            manifest_sha256: verification.manifest_sha256,
        })
    }

    /// Independently verifies a previously installed bundle below this root.
    ///
    /// Verification checks the manifest digest, schema, evidence coverage,
    /// payload sizes and SHA-256 digests, missing optional entries, path safety,
    /// symlinks, duplicate paths, and unlisted files.
    ///
    /// # Errors
    ///
    /// Returns a typed validation, capacity, integrity, or filesystem error.
    pub fn verify(
        &self,
        bundle_path: impl AsRef<Path>,
    ) -> Result<ExecutionAuditBundleVerification, ExecutionAuditBundleError> {
        validate_config(&self.config)?;
        let bundle_path = bundle_path.as_ref();
        let root_metadata = fs::symlink_metadata(&self.config.root)
            .map_err(|err| io_error("inspect root", &self.config.root, err))?;
        if root_metadata.file_type().is_symlink() {
            return Err(ExecutionAuditBundleError::SymbolicLink(
                self.config.root.clone(),
            ));
        }
        if !root_metadata.is_dir() {
            return Err(ExecutionAuditBundleError::NotRegularFile(
                self.config.root.clone(),
            ));
        }
        ensure_below_root(&self.config.root, bundle_path)?;
        self.verify_path(bundle_path)
    }

    fn verify_path(
        &self,
        bundle_path: &Path,
    ) -> Result<ExecutionAuditBundleVerification, ExecutionAuditBundleError> {
        let root_metadata = fs::symlink_metadata(bundle_path)
            .map_err(|err| io_error("inspect", bundle_path, err))?;
        if root_metadata.file_type().is_symlink() {
            return Err(ExecutionAuditBundleError::SymbolicLink(
                bundle_path.to_path_buf(),
            ));
        }
        if !root_metadata.is_dir() {
            return Err(ExecutionAuditBundleError::NotRegularFile(
                bundle_path.to_path_buf(),
            ));
        }

        let manifest_path = bundle_path.join(MANIFEST_FILE);
        let digest_path = bundle_path.join(MANIFEST_DIGEST_FILE);
        let manifest_bytes = read_bounded(&manifest_path, self.config.max_manifest_bytes)?;
        let digest_bytes_on_disk = read_bounded(&digest_path, 256)?;
        let expected_manifest_digest = parse_digest_file(&digest_bytes_on_disk)?;
        let actual_manifest_digest = digest_bytes(&manifest_bytes);
        if expected_manifest_digest != actual_manifest_digest {
            return Err(ExecutionAuditBundleError::DigestMismatch {
                path: manifest_path,
            });
        }

        let document: BundleManifestV1 = serde_json::from_slice(&manifest_bytes)
            .map_err(|err| ExecutionAuditBundleError::InvalidManifest(err.to_string()))?;
        validate_manifest_header(&document, &self.config)?;

        let profile =
            ExecutionAuditBundleProfile::from_str(&document.profile).ok_or_else(|| {
                ExecutionAuditBundleError::InvalidManifest("unknown bundle profile".to_string())
            })?;
        let mut listed_paths = HashSet::with_capacity(document.artifacts.len() + 2);
        listed_paths.insert(MANIFEST_FILE.to_string());
        listed_paths.insert(MANIFEST_DIGEST_FILE.to_string());
        let mut present_count = 0_usize;
        let mut missing_optional_count = 0_usize;
        let mut total_bytes = 0_u64;

        for artifact in &document.artifacts {
            let relative = PathBuf::from(&artifact.path);
            let normalized = portable_bundle_path(&relative)?;
            let key = collision_key(&normalized);
            if !listed_paths.insert(key) {
                return Err(ExecutionAuditBundleError::DuplicateBundlePath(relative));
            }
            validate_source_label(&artifact.source_label)?;
            let kind = ExecutionAuditArtifactKind::from_str(&artifact.kind).ok_or_else(|| {
                ExecutionAuditBundleError::InvalidManifest(format!(
                    "unknown artifact kind {}",
                    artifact.kind
                ))
            })?;
            let artifact_path = bundle_path.join(&relative);
            if artifact.present {
                let expected_bytes = artifact.bytes.ok_or_else(|| {
                    ExecutionAuditBundleError::InvalidManifest(format!(
                        "present artifact {} has no byte count",
                        artifact.path
                    ))
                })?;
                let expected_digest = artifact.sha256.as_deref().ok_or_else(|| {
                    ExecutionAuditBundleError::InvalidManifest(format!(
                        "present artifact {} has no digest",
                        artifact.path
                    ))
                })?;
                let expected_digest = decode_hex_digest(expected_digest)?;
                let (actual_bytes, actual_digest) = hash_regular_file(
                    &artifact_path,
                    self.config.max_artifact_bytes,
                    self.config.max_total_bytes - total_bytes,
                    self.config.copy_buffer_bytes,
                )?;
                if actual_bytes != expected_bytes || actual_digest != expected_digest {
                    return Err(ExecutionAuditBundleError::DigestMismatch {
                        path: artifact_path,
                    });
                }
                total_bytes =
                    checked_total(total_bytes, actual_bytes, self.config.max_total_bytes)?;
                present_count += 1;
            } else {
                if artifact.required {
                    return Err(ExecutionAuditBundleError::MissingRequiredArtifact {
                        kind,
                        path: relative,
                    });
                }
                if artifact.bytes.is_some() || artifact.sha256.is_some() {
                    return Err(ExecutionAuditBundleError::InvalidManifest(format!(
                        "missing artifact {} carries size or digest",
                        artifact.path
                    )));
                }
                if path_exists(&artifact_path)? {
                    return Err(ExecutionAuditBundleError::InvalidManifest(format!(
                        "optional artifact {} is marked absent but exists",
                        artifact.path
                    )));
                }
                missing_optional_count += 1;
            }
        }

        validate_production_manifest_coverage(profile, &document.artifacts)?;
        if document.artifact_count != document.artifacts.len()
            || document.present_artifact_count != present_count
            || document.missing_optional_count != missing_optional_count
            || document.total_artifact_bytes != total_bytes
        {
            return Err(ExecutionAuditBundleError::InvalidManifest(
                "manifest aggregate counts do not match artifacts".to_string(),
            ));
        }
        reject_unlisted_entries(bundle_path, &listed_paths, self.config.max_artifacts)?;

        Ok(ExecutionAuditBundleVerification {
            bundle_path: bundle_path.to_path_buf(),
            incident_id: document.incident_id,
            profile,
            artifact_count: document.artifact_count,
            present_artifact_count: present_count,
            missing_optional_count,
            total_artifact_bytes: total_bytes,
            manifest_sha256: actual_manifest_digest,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum ArtifactOutcome {
    Present { bytes: u64, digest: [u8; 32] },
    MissingOptional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleManifestV1 {
    schema_version: u16,
    incident_id: String,
    profile: String,
    generated_ns: u64,
    time_range: ManifestTimeRange,
    execution_manifest: Option<ManifestExecutionSnapshot>,
    artifact_count: usize,
    present_artifact_count: usize,
    missing_optional_count: usize,
    total_artifact_bytes: u64,
    artifacts: Vec<ManifestArtifact>,
}

impl BundleManifestV1 {
    fn new(
        request: &ExecutionAuditBundleRequest,
        artifacts: Vec<ManifestArtifact>,
        total_artifact_bytes: u64,
    ) -> Self {
        let present_artifact_count = artifacts.iter().filter(|item| item.present).count();
        let missing_optional_count = artifacts.len().saturating_sub(present_artifact_count);
        Self {
            schema_version: BUNDLE_SCHEMA_VERSION,
            incident_id: request.incident_id.clone(),
            profile: request.profile.as_str().to_string(),
            generated_ns: request.generated_ns,
            time_range: ManifestTimeRange {
                start_ns: request.time_range.start_ns,
                end_ns: request.time_range.end_ns,
            },
            execution_manifest: request
                .execution_manifest
                .map(ManifestExecutionSnapshot::from),
            artifact_count: artifacts.len(),
            present_artifact_count,
            missing_optional_count,
            total_artifact_bytes,
            artifacts,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestTimeRange {
    start_ns: u64,
    end_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestArtifact {
    kind: String,
    path: String,
    source_label: String,
    required: bool,
    present: bool,
    bytes: Option<u64>,
    sha256: Option<String>,
}

impl ManifestArtifact {
    fn from_outcome(
        artifact: &ExecutionAuditArtifact,
        path: String,
        outcome: ArtifactOutcome,
    ) -> Self {
        let (present, bytes, sha256) = match outcome {
            ArtifactOutcome::Present { bytes, digest } => {
                (true, Some(bytes), Some(encode_hex(&digest)))
            }
            ArtifactOutcome::MissingOptional => (false, None, None),
        };
        Self {
            kind: artifact.kind.as_str().to_string(),
            path,
            source_label: artifact.source_label.clone(),
            required: artifact.required,
            present,
            bytes,
            sha256,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestExecutionSnapshot {
    schema_version: u16,
    generated_ns: u64,
    runbook: ManifestRunbookSnapshot,
    route_count: usize,
    enabled_route_count: usize,
    open_order_count: usize,
    terminal_order_count: usize,
    journal_record_count: usize,
    journal_command_count: usize,
    journal_event_count: usize,
    metrics: ManifestExecutionMetrics,
    submissions_enabled: bool,
    operator_attention_required: bool,
}

impl From<ExecutionAuditBundleManifest> for ManifestExecutionSnapshot {
    fn from(value: ExecutionAuditBundleManifest) -> Self {
        Self {
            schema_version: value.schema_version,
            generated_ns: value.generated_ns,
            runbook: value.runbook.into(),
            route_count: value.route_count,
            enabled_route_count: value.enabled_route_count,
            open_order_count: value.open_order_count,
            terminal_order_count: value.terminal_order_count,
            journal_record_count: value.journal_record_count,
            journal_command_count: value.journal_command_count,
            journal_event_count: value.journal_event_count,
            metrics: value.metrics.into(),
            submissions_enabled: value.submissions_enabled,
            operator_attention_required: value.operator_attention_required,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestRunbookSnapshot {
    started: bool,
    connected: bool,
    degraded: bool,
    health_seq: u64,
    route_count: usize,
    enabled_route_count: usize,
    disabled_route_count: usize,
    kill_switch_route_count: usize,
    open_order_count: usize,
    terminal_order_count: usize,
    submitted: u64,
    cancelled: u64,
    amended: u64,
    events_applied: u64,
    risk_rejected: u64,
    adapter_errors: u64,
    recovered: u64,
    submissions_paused: bool,
    draining_route_count: usize,
    degraded_route_count: usize,
    available_route_count: usize,
    new_submissions_blocked: bool,
    operator_attention_required: bool,
}

impl From<ExecutionRunbookSnapshot> for ManifestRunbookSnapshot {
    fn from(value: ExecutionRunbookSnapshot) -> Self {
        Self {
            started: value.started,
            connected: value.connected,
            degraded: value.degraded,
            health_seq: value.health_seq,
            route_count: value.route_count,
            enabled_route_count: value.enabled_route_count,
            disabled_route_count: value.disabled_route_count,
            kill_switch_route_count: value.kill_switch_route_count,
            open_order_count: value.open_order_count,
            terminal_order_count: value.terminal_order_count,
            submitted: value.submitted,
            cancelled: value.cancelled,
            amended: value.amended,
            events_applied: value.events_applied,
            risk_rejected: value.risk_rejected,
            adapter_errors: value.adapter_errors,
            recovered: value.recovered,
            submissions_paused: value.submissions_paused,
            draining_route_count: value.draining_route_count,
            degraded_route_count: value.degraded_route_count,
            available_route_count: value.available_route_count,
            new_submissions_blocked: value.new_submissions_blocked,
            operator_attention_required: value.operator_attention_required,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestExecutionMetrics {
    submitted: u64,
    cancelled: u64,
    amended: u64,
    events_applied: u64,
    risk_rejected: u64,
    adapter_errors: u64,
    recovered: u64,
}

impl From<ExecutionMetrics> for ManifestExecutionMetrics {
    fn from(value: ExecutionMetrics) -> Self {
        Self {
            submitted: value.submitted,
            cancelled: value.cancelled,
            amended: value.amended,
            events_applied: value.events_applied,
            risk_rejected: value.risk_rejected,
            adapter_errors: value.adapter_errors,
            recovered: value.recovered,
        }
    }
}

struct StagingDirectory {
    path: PathBuf,
    armed: bool,
}

impl StagingDirectory {
    fn create(root: &Path, final_path: &Path) -> Result<Self, ExecutionAuditBundleError> {
        let final_name = final_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                ExecutionAuditBundleError::InvalidBundlePath(final_path.to_path_buf())
            })?;
        for suffix in 0_u16..=1024 {
            let candidate = root.join(format!(
                ".{final_name}.staging-{}-{suffix}",
                std::process::id()
            ));
            match create_private_directory(&candidate) {
                Ok(()) => {
                    return Ok(Self {
                        path: candidate,
                        armed: true,
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(io_error("create staging directory", &candidate, err)),
            }
        }
        Err(ExecutionAuditBundleError::Io {
            operation: "create staging directory",
            path: root.to_path_buf(),
            message: "staging name space exhausted".to_string(),
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

fn create_private_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        let mut builder = fs::DirBuilder::new();
        builder.mode(0o700).create(path)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir(path)
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn validate_config(config: &ExecutionAuditBundleConfig) -> Result<(), ExecutionAuditBundleError> {
    if config.root.as_os_str().is_empty() {
        return Err(ExecutionAuditBundleError::InvalidConfig(
            "destination root is empty",
        ));
    }
    if config.max_artifacts == 0 {
        return Err(ExecutionAuditBundleError::InvalidConfig(
            "max_artifacts must be positive",
        ));
    }
    if config.max_artifact_bytes == 0 {
        return Err(ExecutionAuditBundleError::InvalidConfig(
            "max_artifact_bytes must be positive",
        ));
    }
    if config.max_total_bytes == 0 {
        return Err(ExecutionAuditBundleError::InvalidConfig(
            "max_total_bytes must be positive",
        ));
    }
    if config.max_manifest_bytes == 0 {
        return Err(ExecutionAuditBundleError::InvalidConfig(
            "max_manifest_bytes must be positive",
        ));
    }
    if !(4 * 1024..=1024 * 1024).contains(&config.copy_buffer_bytes) {
        return Err(ExecutionAuditBundleError::InvalidConfig(
            "copy_buffer_bytes must be between 4096 and 1048576",
        ));
    }
    Ok(())
}

fn validate_request(
    request: &ExecutionAuditBundleRequest,
    config: &ExecutionAuditBundleConfig,
) -> Result<(), ExecutionAuditBundleError> {
    validate_incident_id(&request.incident_id)?;
    if request.time_range.end_ns < request.time_range.start_ns {
        return Err(ExecutionAuditBundleError::InvalidTimeRange);
    }
    if request.artifacts.len() > config.max_artifacts {
        return Err(ExecutionAuditBundleError::ArtifactCapacityExceeded {
            actual: request.artifacts.len(),
            maximum: config.max_artifacts,
        });
    }

    let mut paths = HashSet::with_capacity(request.artifacts.len());
    let mut present_kinds = HashSet::new();
    let mut total_bytes = 0_u64;
    for artifact in &request.artifacts {
        let path = portable_bundle_path(&artifact.bundle_path)?;
        if !paths.insert(collision_key(&path)) {
            return Err(ExecutionAuditBundleError::DuplicateBundlePath(
                artifact.bundle_path.clone(),
            ));
        }
        validate_source_label(&artifact.source_label)?;
        match &artifact.source {
            ExecutionAuditArtifactSource::File(source) => {
                if let Some(bytes) = inspect_source(source, artifact)? {
                    enforce_artifact_size(&artifact.bundle_path, bytes, config.max_artifact_bytes)?;
                    total_bytes = checked_total(total_bytes, bytes, config.max_total_bytes)?;
                    present_kinds.insert(artifact.kind);
                }
            }
            ExecutionAuditArtifactSource::Bytes(bytes) => {
                let bytes = u64::try_from(bytes.len()).map_err(|_| {
                    ExecutionAuditBundleError::ArtifactTooLarge {
                        path: artifact.bundle_path.clone(),
                        actual: u64::MAX,
                        maximum: config.max_artifact_bytes,
                    }
                })?;
                enforce_artifact_size(&artifact.bundle_path, bytes, config.max_artifact_bytes)?;
                total_bytes = checked_total(total_bytes, bytes, config.max_total_bytes)?;
                present_kinds.insert(artifact.kind);
            }
        }
    }
    if request.profile == ExecutionAuditBundleProfile::ProductionIncident {
        for kind in ExecutionAuditArtifactKind::PRODUCTION_REQUIRED {
            if !present_kinds.contains(&kind) {
                return Err(ExecutionAuditBundleError::MissingProductionArtifact(kind));
            }
        }
    }
    Ok(())
}

fn validate_incident_id(value: &str) -> Result<(), ExecutionAuditBundleError> {
    if value.is_empty()
        || value.len() > MAX_INCIDENT_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(ExecutionAuditBundleError::InvalidIncidentId);
    }
    Ok(())
}

fn validate_source_label(value: &str) -> Result<(), ExecutionAuditBundleError> {
    if value.is_empty()
        || value.len() > MAX_SOURCE_LABEL_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(ExecutionAuditBundleError::InvalidManifest(
            "source label is empty, too long, or contains control characters".to_string(),
        ));
    }
    Ok(())
}

fn portable_bundle_path(path: &Path) -> Result<String, ExecutionAuditBundleError> {
    let value = path
        .to_str()
        .ok_or_else(|| ExecutionAuditBundleError::InvalidBundlePath(path.to_path_buf()))?;
    if value.is_empty()
        || value.len() > MAX_BUNDLE_PATH_BYTES
        || value.contains('\\')
        || value.contains(':')
        || value.chars().any(char::is_control)
    {
        return Err(ExecutionAuditBundleError::InvalidBundlePath(
            path.to_path_buf(),
        ));
    }
    let mut normal_count = 0_usize;
    for component in path.components() {
        match component {
            Component::Normal(value) if !value.is_empty() => normal_count += 1,
            _ => {
                return Err(ExecutionAuditBundleError::InvalidBundlePath(
                    path.to_path_buf(),
                ));
            }
        }
    }
    if normal_count == 0 {
        return Err(ExecutionAuditBundleError::InvalidBundlePath(
            path.to_path_buf(),
        ));
    }
    let normalized = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    if normalized.eq_ignore_ascii_case(MANIFEST_FILE)
        || normalized.eq_ignore_ascii_case(MANIFEST_DIGEST_FILE)
    {
        return Err(ExecutionAuditBundleError::ReservedBundlePath(
            path.to_path_buf(),
        ));
    }
    Ok(normalized)
}

fn collision_key(path: &str) -> String {
    path.to_ascii_lowercase()
}

fn prepare_root(root: &Path) -> Result<(), ExecutionAuditBundleError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(ExecutionAuditBundleError::SymbolicLink(root.to_path_buf()));
            }
            if !metadata.is_dir() {
                return Err(ExecutionAuditBundleError::NotRegularFile(
                    root.to_path_buf(),
                ));
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(root).map_err(|err| io_error("create root", root, err))?;
        }
        Err(err) => return Err(io_error("inspect root", root, err)),
    }
    Ok(())
}

fn inspect_source(
    source: &Path,
    artifact: &ExecutionAuditArtifact,
) -> Result<Option<u64>, ExecutionAuditBundleError> {
    match fs::symlink_metadata(source) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(ExecutionAuditBundleError::SymbolicLink(
                    source.to_path_buf(),
                ));
            }
            if !metadata.is_file() {
                return Err(ExecutionAuditBundleError::NotRegularFile(
                    source.to_path_buf(),
                ));
            }
            Ok(Some(metadata.len()))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound && !artifact.required => Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Err(ExecutionAuditBundleError::MissingRequiredArtifact {
                kind: artifact.kind,
                path: source.to_path_buf(),
            })
        }
        Err(err) => Err(io_error("inspect source", source, err)),
    }
}

fn copy_file(
    source: &Path,
    destination: &Path,
    buffer: &mut [u8],
    max_artifact_bytes: u64,
    remaining_total_bytes: u64,
    sync_on_write: bool,
) -> Result<(u64, [u8; 32]), ExecutionAuditBundleError> {
    ensure_parent(destination)?;
    let mut input = File::open(source).map_err(|err| io_error("open source", source, err))?;
    let metadata = input
        .metadata()
        .map_err(|err| io_error("inspect opened source", source, err))?;
    if !metadata.is_file() {
        return Err(ExecutionAuditBundleError::NotRegularFile(
            source.to_path_buf(),
        ));
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|err| io_error("create artifact", destination, err))?;
    let mut hasher = Sha256::new();
    let mut copied = 0_u64;
    loop {
        let count = input
            .read(buffer)
            .map_err(|err| io_error("read source", source, err))?;
        if count == 0 {
            break;
        }
        let count_u64 =
            u64::try_from(count).map_err(|_| ExecutionAuditBundleError::ArtifactTooLarge {
                path: destination.to_path_buf(),
                actual: u64::MAX,
                maximum: max_artifact_bytes,
            })?;
        copied = copied.checked_add(count_u64).ok_or_else(|| {
            ExecutionAuditBundleError::ArtifactTooLarge {
                path: destination.to_path_buf(),
                actual: u64::MAX,
                maximum: max_artifact_bytes,
            }
        })?;
        if copied > max_artifact_bytes {
            return Err(ExecutionAuditBundleError::ArtifactTooLarge {
                path: destination.to_path_buf(),
                actual: copied,
                maximum: max_artifact_bytes,
            });
        }
        if copied > remaining_total_bytes {
            return Err(ExecutionAuditBundleError::BundleTooLarge {
                actual: copied,
                maximum: remaining_total_bytes,
            });
        }
        output
            .write_all(&buffer[..count])
            .map_err(|err| io_error("write artifact", destination, err))?;
        hasher.update(&buffer[..count]);
    }
    output
        .flush()
        .map_err(|err| io_error("flush artifact", destination, err))?;
    if sync_on_write {
        output
            .sync_data()
            .map_err(|err| io_error("sync artifact", destination, err))?;
    }
    Ok((copied, hasher.finalize().into()))
}

fn write_bytes(
    destination: &Path,
    bytes: &[u8],
    sync_on_write: bool,
) -> Result<[u8; 32], ExecutionAuditBundleError> {
    ensure_parent(destination)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|err| io_error("create artifact", destination, err))?;
    output
        .write_all(bytes)
        .map_err(|err| io_error("write artifact", destination, err))?;
    output
        .flush()
        .map_err(|err| io_error("flush artifact", destination, err))?;
    if sync_on_write {
        output
            .sync_data()
            .map_err(|err| io_error("sync artifact", destination, err))?;
    }
    Ok(digest_bytes(bytes))
}

fn hash_regular_file(
    path: &Path,
    max_artifact_bytes: u64,
    remaining_total_bytes: u64,
    copy_buffer_bytes: usize,
) -> Result<(u64, [u8; 32]), ExecutionAuditBundleError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|err| io_error("inspect artifact", path, err))?;
    if metadata.file_type().is_symlink() {
        return Err(ExecutionAuditBundleError::SymbolicLink(path.to_path_buf()));
    }
    if !metadata.is_file() {
        return Err(ExecutionAuditBundleError::NotRegularFile(
            path.to_path_buf(),
        ));
    }
    enforce_artifact_size(path, metadata.len(), max_artifact_bytes)?;
    if metadata.len() > remaining_total_bytes {
        return Err(ExecutionAuditBundleError::BundleTooLarge {
            actual: metadata.len(),
            maximum: remaining_total_bytes,
        });
    }
    let mut file = File::open(path).map_err(|err| io_error("open artifact", path, err))?;
    let mut buffer = vec![0_u8; copy_buffer_bytes];
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|err| io_error("read artifact", path, err))?;
        if count == 0 {
            break;
        }
        bytes = bytes.checked_add(count as u64).ok_or_else(|| {
            ExecutionAuditBundleError::ArtifactTooLarge {
                path: path.to_path_buf(),
                actual: u64::MAX,
                maximum: max_artifact_bytes,
            }
        })?;
        enforce_artifact_size(path, bytes, max_artifact_bytes)?;
        if bytes > remaining_total_bytes {
            return Err(ExecutionAuditBundleError::BundleTooLarge {
                actual: bytes,
                maximum: remaining_total_bytes,
            });
        }
        hasher.update(&buffer[..count]);
    }
    Ok((bytes, hasher.finalize().into()))
}

fn ensure_parent(path: &Path) -> Result<(), ExecutionAuditBundleError> {
    let parent = path
        .parent()
        .ok_or_else(|| ExecutionAuditBundleError::InvalidBundlePath(path.to_path_buf()))?;
    fs::create_dir_all(parent).map_err(|err| io_error("create artifact directory", parent, err))
}

fn enforce_artifact_size(
    path: &Path,
    actual: u64,
    maximum: u64,
) -> Result<(), ExecutionAuditBundleError> {
    if actual > maximum {
        return Err(ExecutionAuditBundleError::ArtifactTooLarge {
            path: path.to_path_buf(),
            actual,
            maximum,
        });
    }
    Ok(())
}

fn enforce_total_size(
    current: u64,
    additional: u64,
    maximum: u64,
) -> Result<(), ExecutionAuditBundleError> {
    let _ = checked_total(current, additional, maximum)?;
    Ok(())
}

fn checked_total(
    current: u64,
    additional: u64,
    maximum: u64,
) -> Result<u64, ExecutionAuditBundleError> {
    let actual = current.saturating_add(additional);
    if actual > maximum {
        return Err(ExecutionAuditBundleError::BundleTooLarge { actual, maximum });
    }
    Ok(actual)
}

fn encode_manifest(
    document: &BundleManifestV1,
    max_bytes: u64,
) -> Result<Vec<u8>, ExecutionAuditBundleError> {
    let mut bytes = serde_json::to_vec_pretty(document)
        .map_err(|err| ExecutionAuditBundleError::InvalidManifest(err.to_string()))?;
    bytes.push(b'\n');
    let actual = bytes.len() as u64;
    if actual > max_bytes {
        return Err(ExecutionAuditBundleError::InvalidManifest(format!(
            "encoded manifest has {actual} bytes, maximum is {max_bytes}"
        )));
    }
    Ok(bytes)
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, ExecutionAuditBundleError> {
    let metadata = fs::symlink_metadata(path).map_err(|err| io_error("inspect file", path, err))?;
    if metadata.file_type().is_symlink() {
        return Err(ExecutionAuditBundleError::SymbolicLink(path.to_path_buf()));
    }
    if !metadata.is_file() {
        return Err(ExecutionAuditBundleError::NotRegularFile(
            path.to_path_buf(),
        ));
    }
    if metadata.len() > maximum {
        return Err(ExecutionAuditBundleError::InvalidManifest(format!(
            "{} has {} bytes, maximum is {maximum}",
            path.display(),
            metadata.len()
        )));
    }
    let capacity = usize::try_from(metadata.len()).map_err(|_| {
        ExecutionAuditBundleError::InvalidManifest("file size does not fit memory".to_string())
    })?;
    let mut bytes = Vec::with_capacity(capacity);
    File::open(path)
        .map_err(|err| io_error("open file", path, err))?
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|err| io_error("read file", path, err))?;
    if bytes.len() as u64 > maximum {
        return Err(ExecutionAuditBundleError::InvalidManifest(format!(
            "{} grew beyond maximum while reading",
            path.display()
        )));
    }
    Ok(bytes)
}

fn validate_manifest_header(
    document: &BundleManifestV1,
    config: &ExecutionAuditBundleConfig,
) -> Result<(), ExecutionAuditBundleError> {
    if document.schema_version != BUNDLE_SCHEMA_VERSION {
        return Err(ExecutionAuditBundleError::InvalidManifest(format!(
            "unsupported schema version {}",
            document.schema_version
        )));
    }
    validate_incident_id(&document.incident_id)?;
    if document.time_range.end_ns < document.time_range.start_ns {
        return Err(ExecutionAuditBundleError::InvalidTimeRange);
    }
    if document.artifacts.len() > config.max_artifacts {
        return Err(ExecutionAuditBundleError::ArtifactCapacityExceeded {
            actual: document.artifacts.len(),
            maximum: config.max_artifacts,
        });
    }
    if document.total_artifact_bytes > config.max_total_bytes {
        return Err(ExecutionAuditBundleError::BundleTooLarge {
            actual: document.total_artifact_bytes,
            maximum: config.max_total_bytes,
        });
    }
    if let Some(snapshot) = &document.execution_manifest {
        validate_execution_snapshot(snapshot)?;
    }
    Ok(())
}

fn validate_execution_snapshot(
    snapshot: &ManifestExecutionSnapshot,
) -> Result<(), ExecutionAuditBundleError> {
    let runbook = &snapshot.runbook;
    let metrics = &snapshot.metrics;
    let consistent = snapshot.schema_version == ExecutionAuditBundleManifest::SCHEMA_VERSION
        && snapshot.route_count == runbook.route_count
        && snapshot.enabled_route_count == runbook.enabled_route_count
        && snapshot.open_order_count == runbook.open_order_count
        && snapshot.terminal_order_count == runbook.terminal_order_count
        && snapshot.submissions_enabled != runbook.new_submissions_blocked
        && snapshot.operator_attention_required == runbook.operator_attention_required
        && metrics.submitted == runbook.submitted
        && metrics.cancelled == runbook.cancelled
        && metrics.amended == runbook.amended
        && metrics.events_applied == runbook.events_applied
        && metrics.risk_rejected == runbook.risk_rejected
        && metrics.adapter_errors == runbook.adapter_errors
        && metrics.recovered == runbook.recovered;
    if !consistent {
        return Err(ExecutionAuditBundleError::InvalidManifest(
            "embedded execution manifest is internally inconsistent".to_string(),
        ));
    }
    Ok(())
}

fn validate_production_coverage(
    profile: ExecutionAuditBundleProfile,
    artifacts: &[ManifestArtifact],
) -> Result<(), ExecutionAuditBundleError> {
    if profile != ExecutionAuditBundleProfile::ProductionIncident {
        return Ok(());
    }
    for required in ExecutionAuditArtifactKind::PRODUCTION_REQUIRED {
        if !artifacts
            .iter()
            .any(|item| item.present && item.kind == required.as_str())
        {
            return Err(ExecutionAuditBundleError::MissingProductionArtifact(
                required,
            ));
        }
    }
    Ok(())
}

fn validate_production_manifest_coverage(
    profile: ExecutionAuditBundleProfile,
    artifacts: &[ManifestArtifact],
) -> Result<(), ExecutionAuditBundleError> {
    validate_production_coverage(profile, artifacts)
}

fn reject_unlisted_entries(
    root: &Path,
    listed_paths: &HashSet<String>,
    max_artifacts: usize,
) -> Result<(), ExecutionAuditBundleError> {
    let max_entries = max_artifacts.saturating_mul(4).saturating_add(16);
    let mut stack = vec![root.to_path_buf()];
    let mut seen_entries = 0_usize;
    while let Some(directory) = stack.pop() {
        for entry in
            fs::read_dir(&directory).map_err(|err| io_error("list bundle", &directory, err))?
        {
            let entry = entry.map_err(|err| io_error("read bundle entry", &directory, err))?;
            seen_entries = seen_entries.saturating_add(1);
            if seen_entries > max_entries {
                return Err(ExecutionAuditBundleError::ArtifactCapacityExceeded {
                    actual: seen_entries,
                    maximum: max_entries,
                });
            }
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|err| io_error("inspect bundle entry", &path, err))?;
            if metadata.file_type().is_symlink() {
                return Err(ExecutionAuditBundleError::SymbolicLink(path));
            }
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if !metadata.is_file() {
                return Err(ExecutionAuditBundleError::NotRegularFile(path));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| ExecutionAuditBundleError::InvalidBundlePath(path.clone()))?;
            let relative = relative.to_str().ok_or_else(|| {
                ExecutionAuditBundleError::InvalidBundlePath(relative.to_path_buf())
            })?;
            let key = collision_key(&relative.replace('\\', "/"));
            if !listed_paths.contains(&key) {
                return Err(ExecutionAuditBundleError::InvalidManifest(format!(
                    "unlisted file {relative}"
                )));
            }
        }
    }
    Ok(())
}

fn ensure_below_root(root: &Path, bundle_path: &Path) -> Result<(), ExecutionAuditBundleError> {
    let canonical_root =
        fs::canonicalize(root).map_err(|err| io_error("canonicalize root", root, err))?;
    let canonical_bundle = fs::canonicalize(bundle_path)
        .map_err(|err| io_error("canonicalize bundle", bundle_path, err))?;
    if canonical_bundle == canonical_root || !canonical_bundle.starts_with(&canonical_root) {
        return Err(ExecutionAuditBundleError::InvalidBundlePath(
            bundle_path.to_path_buf(),
        ));
    }
    Ok(())
}

fn path_exists(path: &Path) -> Result<bool, ExecutionAuditBundleError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(io_error("inspect path", path, err)),
    }
}

fn digest_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_hex_digest(value: &str) -> Result<[u8; 32], ExecutionAuditBundleError> {
    if value.len() != 64 {
        return Err(ExecutionAuditBundleError::InvalidManifest(
            "SHA-256 digest must contain 64 hexadecimal characters".to_string(),
        ));
    }
    let mut output = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        output[index] = (high << 4) | low;
    }
    Ok(output)
}

fn hex_value(value: u8) -> Result<u8, ExecutionAuditBundleError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(ExecutionAuditBundleError::InvalidManifest(
            "SHA-256 digest contains non-hexadecimal data".to_string(),
        )),
    }
}

fn parse_digest_file(bytes: &[u8]) -> Result<[u8; 32], ExecutionAuditBundleError> {
    let value = std::str::from_utf8(bytes).map_err(|err| {
        ExecutionAuditBundleError::InvalidManifest(format!("manifest digest is not UTF-8: {err}"))
    })?;
    let mut fields = value.split_whitespace();
    let digest = fields.next().ok_or_else(|| {
        ExecutionAuditBundleError::InvalidManifest("manifest digest is empty".to_string())
    })?;
    let filename = fields.next().ok_or_else(|| {
        ExecutionAuditBundleError::InvalidManifest("manifest digest has no filename".to_string())
    })?;
    if filename != MANIFEST_FILE || fields.next().is_some() {
        return Err(ExecutionAuditBundleError::InvalidManifest(
            "manifest digest has unexpected fields".to_string(),
        ));
    }
    decode_hex_digest(digest)
}

fn sync_directory(path: &Path) -> Result<(), ExecutionAuditBundleError> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(|err| io_error("sync directory", path, err))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn io_error(
    operation: &'static str,
    path: &Path,
    error: std::io::Error,
) -> ExecutionAuditBundleError {
    ExecutionAuditBundleError::Io {
        operation,
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(1);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(name: &str) -> Self {
            let id = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "orderflow-audit-{name}-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create test root");
            Self(path)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn custom_request(id: &str) -> ExecutionAuditBundleRequest {
        ExecutionAuditBundleRequest::new(id, 300, ExecutionAuditTimeRange::new(100, 200))
            .with_profile(ExecutionAuditBundleProfile::Custom)
    }

    fn production_request(id: &str) -> ExecutionAuditBundleRequest {
        let mut request =
            ExecutionAuditBundleRequest::new(id, 300, ExecutionAuditTimeRange::new(100, 200));
        for (index, kind) in ExecutionAuditArtifactKind::PRODUCTION_REQUIRED
            .iter()
            .copied()
            .enumerate()
        {
            request.push_artifact(ExecutionAuditArtifact::from_bytes(
                kind,
                format!("evidence-{index}").into_bytes(),
                format!("evidence/{index:02}-{}.bin", kind.as_str()),
            ));
        }
        request
    }

    #[test]
    fn custom_bundle_exports_optional_absence_and_verifies() {
        let root = TestRoot::new("custom");
        let source = root.0.join("source.wal");
        fs::write(&source, b"wal-data").unwrap();
        let missing = root.0.join("missing.log");
        let request = custom_request("INC_42")
            .with_artifact(ExecutionAuditArtifact::from_file(
                ExecutionAuditArtifactKind::ExecutionWal,
                &source,
                "wal/execution.ofwal",
            ))
            .with_artifact(ExecutionAuditArtifact::from_bytes(
                ExecutionAuditArtifactKind::BuildMetadata,
                b"version=0.4.0".to_vec(),
                "metadata/build.txt",
            ))
            .with_artifact(
                ExecutionAuditArtifact::from_file(
                    ExecutionAuditArtifactKind::OperatorAudit,
                    missing,
                    "operator/audit.log",
                )
                .optional(),
            );
        let exporter = ExecutionAuditBundleExporter::new(
            ExecutionAuditBundleConfig::new(root.0.join("bundles")).with_sync_on_write(false),
        );

        let report = exporter.export(&request).unwrap();
        assert_eq!(report.artifact_count(), 3);
        assert_eq!(report.present_artifact_count(), 2);
        assert_eq!(report.missing_optional_count(), 1);
        assert_eq!(report.total_artifact_bytes(), 21);
        assert_eq!(
            fs::read(report.bundle_path().join("wal/execution.ofwal")).unwrap(),
            b"wal-data"
        );
        let verified = exporter.verify(report.bundle_path()).unwrap();
        assert_eq!(verified.manifest_sha256(), report.manifest_sha256());
    }

    #[test]
    fn production_bundle_requires_and_verifies_every_evidence_class() {
        let root = TestRoot::new("production");
        let exporter = ExecutionAuditBundleExporter::new(
            ExecutionAuditBundleConfig::new(root.0.join("bundles")).with_sync_on_write(false),
        );
        let request = production_request("PROD_7");

        let report = exporter.export(&request).unwrap();
        assert_eq!(
            report.present_artifact_count(),
            ExecutionAuditArtifactKind::PRODUCTION_REQUIRED.len()
        );
        assert_eq!(
            report.profile(),
            ExecutionAuditBundleProfile::ProductionIncident
        );
        exporter.verify(report.bundle_path()).unwrap();
    }

    #[test]
    fn production_bundle_fails_closed_without_full_coverage() {
        let root = TestRoot::new("coverage");
        let destination = root.0.join("bundles");
        let exporter = ExecutionAuditBundleExporter::new(
            ExecutionAuditBundleConfig::new(&destination).with_sync_on_write(false),
        );
        let request =
            ExecutionAuditBundleRequest::new("PROD_8", 300, ExecutionAuditTimeRange::new(100, 200));

        assert_eq!(
            exporter.export(&request),
            Err(ExecutionAuditBundleError::MissingProductionArtifact(
                ExecutionAuditArtifactKind::ExecutionWal
            ))
        );
        assert!(!destination.exists());
    }

    #[test]
    fn request_rejects_traversal_reserved_and_case_colliding_paths() {
        let root = TestRoot::new("paths");
        let exporter = ExecutionAuditBundleExporter::new(
            ExecutionAuditBundleConfig::new(root.0.join("bundles")).with_sync_on_write(false),
        );
        let traversal = custom_request("PATH_1").with_artifact(ExecutionAuditArtifact::from_bytes(
            ExecutionAuditArtifactKind::Other,
            b"x".to_vec(),
            "../escape",
        ));
        assert!(matches!(
            exporter.export(&traversal),
            Err(ExecutionAuditBundleError::InvalidBundlePath(_))
        ));
        let reserved = custom_request("PATH_2").with_artifact(ExecutionAuditArtifact::from_bytes(
            ExecutionAuditArtifactKind::Other,
            b"x".to_vec(),
            "MANIFEST.JSON",
        ));
        assert!(matches!(
            exporter.export(&reserved),
            Err(ExecutionAuditBundleError::ReservedBundlePath(_))
        ));
        let duplicate = custom_request("PATH_3")
            .with_artifact(ExecutionAuditArtifact::from_bytes(
                ExecutionAuditArtifactKind::Other,
                b"x".to_vec(),
                "Evidence/A.bin",
            ))
            .with_artifact(ExecutionAuditArtifact::from_bytes(
                ExecutionAuditArtifactKind::Other,
                b"y".to_vec(),
                "evidence/a.bin",
            ));
        assert!(matches!(
            exporter.export(&duplicate),
            Err(ExecutionAuditBundleError::DuplicateBundlePath(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn source_symlink_is_rejected() {
        use std::os::unix::fs::symlink;

        let root = TestRoot::new("symlink");
        let source = root.0.join("source");
        let link = root.0.join("link");
        fs::write(&source, b"evidence").unwrap();
        symlink(&source, &link).unwrap();
        let exporter = ExecutionAuditBundleExporter::new(
            ExecutionAuditBundleConfig::new(root.0.join("bundles")).with_sync_on_write(false),
        );
        let request = custom_request("LINK_1").with_artifact(ExecutionAuditArtifact::from_file(
            ExecutionAuditArtifactKind::Other,
            link.clone(),
            "evidence.bin",
        ));
        assert_eq!(
            exporter.export(&request),
            Err(ExecutionAuditBundleError::SymbolicLink(link))
        );
    }

    #[test]
    fn artifact_and_total_limits_fail_before_publication() {
        let root = TestRoot::new("limits");
        let destination = root.0.join("bundles");
        let exporter = ExecutionAuditBundleExporter::new(
            ExecutionAuditBundleConfig::new(&destination)
                .with_limits(2, 4, 4)
                .with_sync_on_write(false),
        );
        let request = custom_request("LIMIT_1").with_artifact(ExecutionAuditArtifact::from_bytes(
            ExecutionAuditArtifactKind::Other,
            b"12345".to_vec(),
            "too-large.bin",
        ));
        assert!(matches!(
            exporter.export(&request),
            Err(ExecutionAuditBundleError::ArtifactTooLarge { .. })
        ));
        assert!(!destination.exists());
    }

    #[test]
    fn verifier_detects_payload_corruption_and_unlisted_files() {
        let root = TestRoot::new("tamper");
        let exporter = ExecutionAuditBundleExporter::new(
            ExecutionAuditBundleConfig::new(root.0.join("bundles")).with_sync_on_write(false),
        );
        let request = custom_request("TAMPER_1").with_artifact(ExecutionAuditArtifact::from_bytes(
            ExecutionAuditArtifactKind::Other,
            b"original".to_vec(),
            "evidence.bin",
        ));
        let report = exporter.export(&request).unwrap();
        fs::write(report.bundle_path().join("evidence.bin"), b"modified").unwrap();
        assert!(matches!(
            exporter.verify(report.bundle_path()),
            Err(ExecutionAuditBundleError::DigestMismatch { .. })
        ));

        let second = custom_request("TAMPER_2").with_artifact(ExecutionAuditArtifact::from_bytes(
            ExecutionAuditArtifactKind::Other,
            b"original".to_vec(),
            "evidence.bin",
        ));
        let report = exporter.export(&second).unwrap();
        fs::write(report.bundle_path().join("unlisted.txt"), b"surprise").unwrap();
        assert!(matches!(
            exporter.verify(report.bundle_path()),
            Err(ExecutionAuditBundleError::InvalidManifest(_))
        ));
    }

    #[test]
    fn destination_collision_and_inconsistent_execution_snapshot_are_rejected() {
        let root = TestRoot::new("immutable");
        let exporter = ExecutionAuditBundleExporter::new(
            ExecutionAuditBundleConfig::new(root.0.join("bundles")).with_sync_on_write(false),
        );
        let request = custom_request("IMMUTABLE_1")
            .with_execution_manifest(ExecutionAuditBundleManifest {
                schema_version: 1,
                generated_ns: 250,
                submissions_enabled: true,
                ..ExecutionAuditBundleManifest::default()
            })
            .with_artifact(ExecutionAuditArtifact::from_bytes(
                ExecutionAuditArtifactKind::Other,
                b"evidence".to_vec(),
                "evidence.bin",
            ));
        let report = exporter.export(&request).unwrap();
        let manifest = fs::read_to_string(report.manifest_path()).unwrap();
        assert!(manifest.contains("\"execution_manifest\""));
        assert!(manifest.contains("\"generated_ns\": 250"));
        assert_eq!(
            exporter.export(&request),
            Err(ExecutionAuditBundleError::DestinationExists(
                report.bundle_path().to_path_buf()
            ))
        );

        let mut document: BundleManifestV1 = serde_json::from_str(&manifest).unwrap();
        document.execution_manifest.as_mut().unwrap().route_count = 1;
        let manifest = encode_manifest(&document, 1024 * 1024).unwrap();
        fs::write(report.manifest_path(), &manifest).unwrap();
        let digest = digest_bytes(&manifest);
        fs::write(
            report.bundle_path().join(MANIFEST_DIGEST_FILE),
            format!("{}  {MANIFEST_FILE}\n", encode_hex(&digest)),
        )
        .unwrap();
        assert!(matches!(
            exporter.verify(report.bundle_path()),
            Err(ExecutionAuditBundleError::InvalidManifest(_))
        ));
    }
}
