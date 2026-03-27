use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;
use url::Url;

/// Default content type for Arrow IPC transport over HTTP.
pub const ARROW_TRANSPORT_CONTENT_TYPE: &str = "application/vnd.apache.arrow.stream";
/// Default base URL for a local Arrow transport service.
pub const ARROW_TRANSPORT_DEFAULT_BASE_URL: &str = "http://127.0.0.1:8080";
/// Default request route for Arrow IPC processing.
pub const ARROW_TRANSPORT_DEFAULT_ROUTE: &str = "/arrow-ipc";
/// Default health route for Arrow transport probing.
pub const ARROW_TRANSPORT_DEFAULT_HEALTH_ROUTE: &str = "/health";
/// Default Wendao Arrow schema contract version.
pub const ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION: &str = "v1";
/// Canonical Arrow schema metadata key for the Wendao schema version.
pub const ARROW_TRANSPORT_SCHEMA_VERSION_METADATA_KEY: &str = "wendao.schema_version";
/// Canonical Arrow schema metadata key for request/response trace identifiers.
pub const ARROW_TRANSPORT_TRACE_ID_METADATA_KEY: &str = "trace_id";
const ARROW_TRANSPORT_DEFAULT_TIMEOUT_SECS: u64 = 10;

/// Runtime config for Arrow-over-HTTP transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrowTransportConfig {
    base_url: String,
    route: String,
    health_route: String,
    content_type: String,
    schema_version: String,
    timeout_secs: u64,
}

impl Default for ArrowTransportConfig {
    fn default() -> Self {
        Self {
            base_url: ARROW_TRANSPORT_DEFAULT_BASE_URL.to_string(),
            route: ARROW_TRANSPORT_DEFAULT_ROUTE.to_string(),
            health_route: ARROW_TRANSPORT_DEFAULT_HEALTH_ROUTE.to_string(),
            content_type: ARROW_TRANSPORT_CONTENT_TYPE.to_string(),
            schema_version: ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION.to_string(),
            timeout_secs: ARROW_TRANSPORT_DEFAULT_TIMEOUT_SECS,
        }
    }
}

impl ArrowTransportConfig {
    /// Create a config with a custom base URL and default routes.
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Self::default()
        }
    }

    /// Return the configured base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        self.base_url.as_str()
    }

    /// Return the configured Arrow request route.
    #[must_use]
    pub fn route(&self) -> &str {
        self.route.as_str()
    }

    /// Return the configured health route.
    #[must_use]
    pub fn health_route(&self) -> &str {
        self.health_route.as_str()
    }

    /// Return the configured content type.
    #[must_use]
    pub fn content_type(&self) -> &str {
        self.content_type.as_str()
    }

    /// Return the configured Arrow schema contract version.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        self.schema_version.as_str()
    }

    /// Return the configured request timeout.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    /// Override the Arrow request route.
    #[must_use]
    pub fn with_route(mut self, route: impl Into<String>) -> Self {
        self.route = normalize_route(route.into());
        self
    }

    /// Override the health route.
    #[must_use]
    pub fn with_health_route(mut self, route: impl Into<String>) -> Self {
        self.health_route = normalize_route(route.into());
        self
    }

    /// Override the content type.
    #[must_use]
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = content_type.into();
        self
    }

    /// Override the Arrow schema contract version.
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportConfigError::InvalidSchemaVersion`] when the
    /// provided version is blank.
    pub fn with_schema_version(
        mut self,
        schema_version: impl Into<String>,
    ) -> Result<Self, ArrowTransportConfigError> {
        let schema_version = schema_version.into();
        if schema_version.trim().is_empty() {
            return Err(ArrowTransportConfigError::InvalidSchemaVersion);
        }
        self.schema_version = schema_version;
        Ok(self)
    }

    /// Override the request timeout in seconds.
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportConfigError::InvalidTimeoutSecs`] when the
    /// provided timeout is zero.
    pub fn with_timeout_secs(
        mut self,
        timeout_secs: u64,
    ) -> Result<Self, ArrowTransportConfigError> {
        if timeout_secs == 0 {
            return Err(ArrowTransportConfigError::InvalidTimeoutSecs);
        }
        self.timeout_secs = timeout_secs;
        Ok(self)
    }

    /// Resolve the full Arrow processing endpoint URL.
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportConfigError`] when the configured base URL or
    /// route is invalid.
    pub fn endpoint_url(&self) -> Result<Url, ArrowTransportConfigError> {
        join_route(self.base_url.as_str(), self.route.as_str())
    }

    /// Resolve the full health endpoint URL.
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportConfigError`] when the configured base URL or
    /// route is invalid.
    pub fn health_url(&self) -> Result<Url, ArrowTransportConfigError> {
        join_route(self.base_url.as_str(), self.health_route.as_str())
    }

    /// Load `[gateway.arrow_transport]` from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportConfigError`] when the file cannot be read,
    /// parsed, or the resulting config is invalid.
    pub fn from_toml_path(path: &Path) -> Result<Option<Self>, ArrowTransportConfigError> {
        let content = fs::read_to_string(path).map_err(|source| {
            ArrowTransportConfigError::ReadConfigFile {
                path: path.to_path_buf(),
                source,
            }
        })?;
        Self::from_toml_str(content.as_str())
    }

    /// Load `[gateway.arrow_transport]` from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportConfigError`] when the TOML or resulting config
    /// is invalid.
    pub fn from_toml_str(input: &str) -> Result<Option<Self>, ArrowTransportConfigError> {
        let root: ArrowTransportTomlRoot =
            toml::from_str(input).map_err(ArrowTransportConfigError::ParseConfig)?;
        let Some(partial) = root.gateway.and_then(|gateway| gateway.arrow_transport) else {
            return Ok(None);
        };

        let mut config = Self::default();
        if let Some(base_url) = partial.base_url {
            config.base_url = base_url;
        }
        if let Some(route) = partial.route {
            config.route = normalize_route(route);
        }
        if let Some(health_route) = partial.health_route {
            config.health_route = normalize_route(health_route);
        }
        if let Some(content_type) = partial.content_type {
            config.content_type = content_type;
        }
        if let Some(schema_version) = partial.schema_version {
            config = config.with_schema_version(schema_version)?;
        }
        if let Some(timeout_secs) = partial.timeout_secs {
            config = config.with_timeout_secs(timeout_secs)?;
        }

        config.endpoint_url()?;
        config.health_url()?;
        Ok(Some(config))
    }
}

/// Error returned when Arrow transport config cannot be resolved.
#[derive(Debug, Error)]
pub enum ArrowTransportConfigError {
    /// The config file could not be read from disk.
    #[error("failed to read Arrow transport config file `{path}`: {source}")]
    ReadConfigFile {
        /// File path that failed.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// The TOML payload could not be parsed.
    #[error("failed to parse Arrow transport config TOML: {0}")]
    ParseConfig(#[source] toml::de::Error),
    /// The configured base URL or route is invalid.
    #[error("invalid Arrow transport URL derived from `{base_url}` and `{route}`: {source}")]
    InvalidUrl {
        /// Configured base URL.
        base_url: String,
        /// Configured route.
        route: String,
        /// URL parse error.
        source: url::ParseError,
    },
    /// Timeout must be greater than zero.
    #[error("Arrow transport timeout_secs must be greater than zero")]
    InvalidTimeoutSecs,
    /// Schema version must not be blank.
    #[error("Arrow transport schema_version must not be blank")]
    InvalidSchemaVersion,
}

#[derive(Debug, Deserialize)]
struct ArrowTransportTomlRoot {
    gateway: Option<ArrowTransportTomlGateway>,
}

#[derive(Debug, Deserialize)]
struct ArrowTransportTomlGateway {
    arrow_transport: Option<ArrowTransportTomlSection>,
}

#[derive(Debug, Deserialize)]
struct ArrowTransportTomlSection {
    base_url: Option<String>,
    route: Option<String>,
    health_route: Option<String>,
    content_type: Option<String>,
    schema_version: Option<String>,
    timeout_secs: Option<u64>,
}

fn normalize_route(route: String) -> String {
    if route.starts_with('/') {
        route
    } else {
        format!("/{route}")
    }
}

fn join_route(base_url: &str, route: &str) -> Result<Url, ArrowTransportConfigError> {
    let normalized_route = normalize_route(route.to_string());
    Url::parse(base_url)
        .and_then(|base| base.join(normalized_route.as_str()))
        .map_err(|source| ArrowTransportConfigError::InvalidUrl {
            base_url: base_url.to_string(),
            route: normalized_route,
            source,
        })
}
