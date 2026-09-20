use super::*;

/// Generic adapter factory configuration.
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    /// Provider selection.
    pub provider: ProviderKind,
    /// Optional credentials env-key references.
    pub credentials: Option<CredentialsRef>,
    /// Provider endpoint URI.
    pub endpoint: Option<String>,
    /// Optional client/app name.
    pub app_name: Option<String>,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Mock,
            credentials: None,
            endpoint: None,
            app_name: None,
        }
    }
}

/// Credential environment-variable references for adapter auth bootstrap.
#[derive(Debug, Clone)]
pub struct CredentialsRef {
    /// Environment variable name for key id/user id.
    pub key_id_env: String,
    /// Environment variable name for secret/password.
    pub secret_env: String,
}

#[cfg(any(feature = "binance", feature = "cqg", feature = "rithmic"))]
#[derive(Debug, Clone, Default)]
pub(crate) struct TlsFileConfig {
    pub(crate) ca_file: Option<String>,
    pub(crate) client_cert_file: Option<String>,
    pub(crate) client_chain_file: Option<String>,
    pub(crate) client_key_file: Option<String>,
    pub(crate) client_key_password_env: Option<String>,
}

#[cfg(any(feature = "binance", feature = "cqg", feature = "rithmic"))]
impl TlsFileConfig {
    fn from_env(provider: &str) -> AdapterResult<Self> {
        let ca_file = tls_env(provider, "CA_FILE");
        let client_cert_file = tls_env(provider, "CLIENT_CERT_FILE");
        let client_chain_file = tls_env(provider, "CLIENT_CHAIN_FILE");
        let client_key_file = tls_env(provider, "CLIENT_KEY_FILE");
        let client_key_password_env = tls_env(provider, "CLIENT_KEY_PASSWORD_ENV");

        if client_cert_file.is_some() != client_key_file.is_some() {
            return Err(AdapterError::NotConfigured(
                "TLS client certificate and private key must be configured together",
            ));
        }
        if let Some(password_env) = &client_key_password_env {
            if env::var_os(password_env).is_none() {
                return Err(AdapterError::Other(format!(
                    "TLS private-key password env var is not set: {password_env}"
                )));
            }
        }

        for (name, path) in [
            ("CA file", ca_file.as_deref()),
            ("client certificate file", client_cert_file.as_deref()),
            (
                "client certificate chain file",
                client_chain_file.as_deref(),
            ),
            ("client private key file", client_key_file.as_deref()),
        ] {
            if let Some(path) = path {
                if !Path::new(path).is_file() {
                    return Err(AdapterError::Other(format!(
                        "TLS {name} does not exist or is not a file: {path}"
                    )));
                }
            }
        }

        Ok(Self {
            ca_file,
            client_cert_file,
            client_chain_file,
            client_key_file,
            client_key_password_env,
        })
    }

    pub(crate) fn openssl_args(&self, host: &str, port: u16) -> Vec<String> {
        let mut args = vec![
            "s_client".to_string(),
            "-quiet".to_string(),
            "-verify_return_error".to_string(),
            "-verify_hostname".to_string(),
            host.to_string(),
            "-connect".to_string(),
            format!("{host}:{port}"),
            "-servername".to_string(),
            host.to_string(),
        ];
        if let Some(path) = &self.ca_file {
            args.extend(["-CAfile".to_string(), path.clone()]);
        }
        if let Some(path) = &self.client_cert_file {
            args.extend(["-cert".to_string(), path.clone()]);
        }
        if let Some(path) = &self.client_chain_file {
            args.extend(["-cert_chain".to_string(), path.clone()]);
        }
        if let Some(path) = &self.client_key_file {
            args.extend(["-key".to_string(), path.clone()]);
        }
        if let Some(password_env) = &self.client_key_password_env {
            args.extend(["-passin".to_string(), format!("env:{password_env}")]);
        }
        args
    }
}

#[cfg(any(feature = "binance", feature = "cqg", feature = "rithmic"))]
pub(crate) fn openssl_s_client_args(
    provider: &str,
    host: &str,
    port: u16,
) -> AdapterResult<Vec<String>> {
    Ok(TlsFileConfig::from_env(provider)?.openssl_args(host, port))
}

#[cfg(any(feature = "binance", feature = "cqg", feature = "rithmic"))]
fn tls_env(provider: &str, suffix: &str) -> Option<String> {
    let provider_name = provider.to_ascii_uppercase();
    let scoped = format!("ORDERFLOW_{provider_name}_TLS_{suffix}");
    env::var(&scoped)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var(format!("ORDERFLOW_TLS_{suffix}"))
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
}
