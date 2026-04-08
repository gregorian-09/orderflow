use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};

use serde::Deserialize;

use of_adapters::{CredentialsRef, ProviderKind};

use crate::{EngineConfig, RuntimeError};

/// Indicates how a runtime config file was accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigCompatibilityMode {
    /// Parsed by the typed TOML/JSON loader without compatibility fallback.
    Strict,
    /// Parsed through the legacy flat-key compatibility fallback.
    LegacyFallback,
}

/// Detailed result for config-file loading.
#[derive(Debug, Clone)]
pub struct ConfigLoadReport {
    /// Loaded runtime configuration.
    pub config: EngineConfig,
    /// Source file format (`json` or `toml`).
    pub format: &'static str,
    /// Indicates whether strict parsing or legacy fallback was used.
    pub compatibility_mode: ConfigCompatibilityMode,
    /// Optional compatibility warning for callers who want to surface migration guidance.
    pub warning: Option<String>,
}

impl ConfigLoadReport {
    /// Returns `true` when the legacy flat-key compatibility parser was required.
    pub fn used_legacy_fallback(&self) -> bool {
        self.compatibility_mode == ConfigCompatibilityMode::LegacyFallback
    }
}

/// Loads engine config from `.toml` or `.json`-like config file.
pub fn load_engine_config_from_path(path: &str) -> Result<EngineConfig, RuntimeError> {
    load_engine_config_report_from_path(path).map(|report| report.config)
}

/// Loads engine config and reports whether legacy compatibility fallback was required.
pub fn load_engine_config_report_from_path(path: &str) -> Result<ConfigLoadReport, RuntimeError> {
    let raw = fs::read_to_string(path).map_err(|e| RuntimeError::Io(e.to_string()))?;
    if path.ends_with(".json") {
        parse_config_json(&raw)
    } else if path.ends_with(".toml") {
        parse_config_toml(&raw)
    } else {
        Err(RuntimeError::Config(
            "unsupported config format; use .json or .toml".to_string(),
        ))
    }
}

/// Validates startup configuration and environment prerequisites.
pub fn validate_startup_config(cfg: &EngineConfig) -> Result<(), RuntimeError> {
    if cfg.instance_id.trim().is_empty() {
        return Err(RuntimeError::Config("instance_id must not be empty".to_string()));
    }

    if cfg.signal_threshold <= 0 {
        return Err(RuntimeError::Config(
            "signal_threshold must be > 0".to_string(),
        ));
    }

    if cfg.audit_log_path.trim().is_empty() {
        return Err(RuntimeError::Config(
            "audit_log_path must not be empty".to_string(),
        ));
    }
    if cfg.audit_max_bytes == 0 {
        return Err(RuntimeError::Config(
            "audit_max_bytes must be > 0".to_string(),
        ));
    }
    if cfg.audit_max_files > 1000 {
        return Err(RuntimeError::Config(
            "audit_max_files must be <= 1000".to_string(),
        ));
    }

    if cfg.enable_persistence && cfg.data_root.trim().is_empty() {
        return Err(RuntimeError::Config(
            "data_root must not be empty when persistence is enabled".to_string(),
        ));
    }
    if cfg.enable_persistence && cfg.data_retention_max_bytes == 0 && cfg.data_retention_max_age_secs == 0 {
        return Err(RuntimeError::Config(
            "set at least one of data_retention_max_bytes or data_retention_max_age_secs when persistence is enabled".to_string(),
        ));
    }

    match cfg.adapter.provider {
        ProviderKind::Mock => Ok(()),
        ProviderKind::Rithmic | ProviderKind::Cqg | ProviderKind::Binance => {
            if cfg
                .adapter
                .endpoint
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err(RuntimeError::Config(
                    "non-mock providers require adapter.endpoint".to_string(),
                ));
            }

            if matches!(cfg.adapter.provider, ProviderKind::Rithmic | ProviderKind::Cqg) {
                let creds = cfg.adapter.credentials.as_ref().ok_or_else(|| {
                    RuntimeError::Config(
                        "rithmic/cqg providers require adapter.credentials references".to_string(),
                    )
                })?;

                validate_env_var(&creds.key_id_env)?;
                validate_env_var(&creds.secret_env)?;
            }
            Ok(())
        }
    }
}

fn validate_env_var(name: &str) -> Result<(), RuntimeError> {
    let value = std::env::var(name)
        .map_err(|_| RuntimeError::Config(format!("missing required env var: {name}")))?;
    if value.trim().is_empty() {
        return Err(RuntimeError::Config(format!(
            "required env var is empty: {name}"
        )));
    }
    Ok(())
}

pub(crate) fn config_hash(cfg: &EngineConfig) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    cfg.instance_id.hash(&mut hasher);
    cfg.enable_persistence.hash(&mut hasher);
    cfg.data_root.hash(&mut hasher);
    cfg.audit_log_path.hash(&mut hasher);
    cfg.audit_max_bytes.hash(&mut hasher);
    cfg.audit_max_files.hash(&mut hasher);
    cfg.data_retention_max_bytes.hash(&mut hasher);
    cfg.data_retention_max_age_secs.hash(&mut hasher);
    cfg.signal_threshold.hash(&mut hasher);
    let provider = match cfg.adapter.provider {
        ProviderKind::Mock => 0u8,
        ProviderKind::Rithmic => 1u8,
        ProviderKind::Cqg => 2u8,
        ProviderKind::Binance => 3u8,
    };
    provider.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Debug, Default, Deserialize)]
struct RuntimeConfigFile {
    instance_id: Option<String>,
    enable_persistence: Option<bool>,
    signal_threshold: Option<i64>,
    data_root: Option<String>,
    audit_log_path: Option<String>,
    audit_max_bytes: Option<u64>,
    audit_max_files: Option<u32>,
    audit_redact_tokens: Option<StringListOrCsv>,
    data_retention_max_bytes: Option<u64>,
    data_retention_max_age_secs: Option<u64>,
    provider: Option<String>,
    endpoint: Option<String>,
    app_name: Option<String>,
    credentials_key_id_env: Option<String>,
    credentials_secret_env: Option<String>,
    adapter: Option<AdapterConfigFile>,
    credentials: Option<CredentialsRefFile>,
}

#[derive(Debug, Default, Deserialize)]
struct AdapterConfigFile {
    provider: Option<String>,
    endpoint: Option<String>,
    app_name: Option<String>,
    credentials: Option<CredentialsRefFile>,
}

#[derive(Debug, Default, Deserialize)]
struct CredentialsRefFile {
    key_id_env: Option<String>,
    secret_env: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum StringListOrCsv {
    List(Vec<String>),
    Csv(String),
}

impl StringListOrCsv {
    fn into_vec(self) -> Vec<String> {
        match self {
            StringListOrCsv::List(values) => values
                .into_iter()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect(),
            StringListOrCsv::Csv(value) => parse_csv(&value),
        }
    }
}

fn parse_config_json(raw: &str) -> Result<ConfigLoadReport, RuntimeError> {
    match serde_json::from_str::<RuntimeConfigFile>(raw) {
        Ok(parsed) => Ok(ConfigLoadReport {
            config: config_from_typed(parsed)?,
            format: "json",
            compatibility_mode: ConfigCompatibilityMode::Strict,
            warning: None,
        }),
        Err(strict_err) => {
            let mut kv = HashMap::new();
            parse_json_like(raw, &mut kv)?;
            let config = config_from_map(&kv).map_err(|fallback_err| {
                RuntimeError::Config(format!(
                    "strict json parse failed: {strict_err}; legacy fallback failed: {fallback_err}"
                ))
            })?;
            Ok(ConfigLoadReport {
                config,
                format: "json",
                compatibility_mode: ConfigCompatibilityMode::LegacyFallback,
                warning: Some(format!(
                    "loaded config via legacy json fallback after strict parse failed: {strict_err}; prefer typed top-level runtime keys with nested adapter and adapter.credentials sections"
                )),
            })
        }
    }
}

fn parse_config_toml(raw: &str) -> Result<ConfigLoadReport, RuntimeError> {
    match toml::from_str::<RuntimeConfigFile>(raw) {
        Ok(parsed) => Ok(ConfigLoadReport {
            config: config_from_typed(parsed)?,
            format: "toml",
            compatibility_mode: ConfigCompatibilityMode::Strict,
            warning: None,
        }),
        Err(strict_err) => {
            let mut kv = HashMap::new();
            parse_toml_like(raw, &mut kv)?;
            let config = config_from_map(&kv).map_err(|fallback_err| {
                RuntimeError::Config(format!(
                    "strict toml parse failed: {strict_err}; legacy fallback failed: {fallback_err}"
                ))
            })?;
            Ok(ConfigLoadReport {
                config,
                format: "toml",
                compatibility_mode: ConfigCompatibilityMode::LegacyFallback,
                warning: Some(format!(
                    "loaded config via legacy toml fallback after strict parse failed: {strict_err}; prefer typed top-level runtime keys with nested adapter and adapter.credentials sections"
                )),
            })
        }
    }
}

fn config_from_typed(parsed: RuntimeConfigFile) -> Result<EngineConfig, RuntimeError> {
    let mut cfg = EngineConfig::default();

    if let Some(v) = parsed.instance_id {
        cfg.instance_id = v;
    }
    if let Some(v) = parsed.enable_persistence {
        cfg.enable_persistence = v;
    }
    if let Some(v) = parsed.signal_threshold {
        cfg.signal_threshold = v;
    }
    if let Some(v) = parsed.data_root {
        cfg.data_root = v;
    }
    if let Some(v) = parsed.audit_log_path {
        cfg.audit_log_path = v;
    }
    if let Some(v) = parsed.audit_max_bytes {
        cfg.audit_max_bytes = v;
    }
    if let Some(v) = parsed.audit_max_files {
        cfg.audit_max_files = v;
    }
    if let Some(v) = parsed.audit_redact_tokens {
        cfg.audit_redact_tokens = v.into_vec();
    }
    if let Some(v) = parsed.data_retention_max_bytes {
        cfg.data_retention_max_bytes = v;
    }
    if let Some(v) = parsed.data_retention_max_age_secs {
        cfg.data_retention_max_age_secs = v;
    }

    let adapter = parsed.adapter.unwrap_or_default();
    let provider = adapter.provider.or(parsed.provider);
    let endpoint = adapter.endpoint.or(parsed.endpoint);
    let app_name = adapter.app_name.or(parsed.app_name);
    let creds = adapter.credentials.or(parsed.credentials);
    let key_ref = creds
        .as_ref()
        .and_then(|c| c.key_id_env.clone())
        .or(parsed.credentials_key_id_env);
    let secret_ref = creds
        .as_ref()
        .and_then(|c| c.secret_env.clone())
        .or(parsed.credentials_secret_env);

    if let Some(v) = provider {
        cfg.adapter.provider = parse_provider(&v)?;
    }
    if let Some(v) = endpoint {
        cfg.adapter.endpoint = Some(v);
    }
    if let Some(v) = app_name {
        cfg.adapter.app_name = Some(v);
    }

    match (key_ref, secret_ref) {
        (Some(k), Some(s)) => {
            cfg.adapter.credentials = Some(CredentialsRef {
                key_id_env: k,
                secret_env: s,
            });
        }
        (None, None) => {}
        _ => {
            return Err(RuntimeError::Config(
                "credentials require both key_id_env and secret_env".to_string(),
            ));
        }
    }

    Ok(cfg)
}

fn config_from_map(map: &HashMap<String, String>) -> Result<EngineConfig, RuntimeError> {
    let mut cfg = EngineConfig::default();

    if let Some(v) = map.get("instance_id") {
        cfg.instance_id = v.to_string();
    }

    if let Some(v) = map.get("enable_persistence") {
        cfg.enable_persistence = parse_bool(v, "enable_persistence")?;
    }

    if let Some(v) = map.get("signal_threshold") {
        cfg.signal_threshold = parse_i64(v, "signal_threshold")?;
    }

    if let Some(v) = map.get("data_root") {
        cfg.data_root = v.to_string();
    }
    if let Some(v) = map.get("audit_log_path") {
        cfg.audit_log_path = v.to_string();
    }
    if let Some(v) = map.get("audit_max_bytes") {
        cfg.audit_max_bytes = parse_u64(v, "audit_max_bytes")?;
    }
    if let Some(v) = map.get("audit_max_files") {
        cfg.audit_max_files = parse_u32(v, "audit_max_files")?;
    }
    if let Some(v) = map.get("audit_redact_tokens") {
        cfg.audit_redact_tokens = parse_csv(v);
    }
    if let Some(v) = map.get("data_retention_max_bytes") {
        cfg.data_retention_max_bytes = parse_u64(v, "data_retention_max_bytes")?;
    }
    if let Some(v) = map.get("data_retention_max_age_secs") {
        cfg.data_retention_max_age_secs = parse_u64(v, "data_retention_max_age_secs")?;
    }

    if let Some(v) = map.get("adapter.provider").or_else(|| map.get("provider")) {
        cfg.adapter.provider = parse_provider(v)?;
    }

    if let Some(v) = map.get("adapter.endpoint").or_else(|| map.get("endpoint")) {
        cfg.adapter.endpoint = Some(v.to_string());
    }

    if let Some(v) = map.get("adapter.app_name").or_else(|| map.get("app_name")) {
        cfg.adapter.app_name = Some(v.to_string());
    }

    let key_ref = map
        .get("adapter.credentials.key_id_env")
        .or_else(|| map.get("credentials.key_id_env"))
        .or_else(|| map.get("credentials_key_id_env"));
    let secret_ref = map
        .get("adapter.credentials.secret_env")
        .or_else(|| map.get("credentials.secret_env"))
        .or_else(|| map.get("credentials_secret_env"));

    match (key_ref, secret_ref) {
        (Some(k), Some(s)) => {
            cfg.adapter.credentials = Some(CredentialsRef {
                key_id_env: k.to_string(),
                secret_env: s.to_string(),
            });
        }
        (None, None) => {}
        _ => {
            return Err(RuntimeError::Config(
                "credentials require both key_id_env and secret_env".to_string(),
            ));
        }
    }

    Ok(cfg)
}

fn parse_provider(v: &str) -> Result<ProviderKind, RuntimeError> {
    match v.trim().to_ascii_lowercase().as_str() {
        "mock" => Ok(ProviderKind::Mock),
        "rithmic" => Ok(ProviderKind::Rithmic),
        "cqg" => Ok(ProviderKind::Cqg),
        "binance" | "binance_spot" | "crypto_binance" => Ok(ProviderKind::Binance),
        _ => Err(RuntimeError::Config(format!("unknown provider: {v}"))),
    }
}

fn parse_bool(v: &str, key: &str) -> Result<bool, RuntimeError> {
    match v.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(RuntimeError::Config(format!("invalid bool for {key}: {v}"))),
    }
}

fn parse_i64(v: &str, key: &str) -> Result<i64, RuntimeError> {
    v.trim()
        .parse::<i64>()
        .map_err(|_| RuntimeError::Config(format!("invalid i64 for {key}: {v}")))
}

fn parse_u64(v: &str, key: &str) -> Result<u64, RuntimeError> {
    v.trim()
        .parse::<u64>()
        .map_err(|_| RuntimeError::Config(format!("invalid u64 for {key}: {v}")))
}

fn parse_u32(v: &str, key: &str) -> Result<u32, RuntimeError> {
    v.trim()
        .parse::<u32>()
        .map_err(|_| RuntimeError::Config(format!("invalid u32 for {key}: {v}")))
}

fn parse_csv(v: &str) -> Vec<String> {
    v.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn parse_json_like(raw: &str, out: &mut HashMap<String, String>) -> Result<(), RuntimeError> {
    for line in raw.lines() {
        let mut s = line.trim();
        if s.is_empty() || s == "{" || s == "}" {
            continue;
        }
        if s.ends_with(',') {
            s = &s[..s.len() - 1];
        }
        if s.ends_with('{') {
            continue;
        }

        let (k, v) = match s.split_once(':') {
            Some(parts) => parts,
            None => continue,
        };

        let key = trim_quotes(k.trim());
        let value = trim_quotes(v.trim());
        if !key.is_empty() {
            out.insert(key.to_string(), value.to_string());
        }
    }

    Ok(())
}

fn parse_toml_like(raw: &str, out: &mut HashMap<String, String>) -> Result<(), RuntimeError> {
    let mut section = String::new();

    for line in raw.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }

        let (k, v) = line
            .split_once('=')
            .ok_or_else(|| RuntimeError::Config("invalid toml line".to_string()))?;
        let key = k.trim();
        let value = trim_quotes(v.trim());
        let full_key = if section.is_empty() {
            key.to_string()
        } else {
            format!("{section}.{key}")
        };
        out.insert(full_key, value.to_string());
    }

    Ok(())
}

fn trim_quotes(v: &str) -> &str {
    let t = v.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        &t[1..t.len() - 1]
    } else {
        t
    }
}
