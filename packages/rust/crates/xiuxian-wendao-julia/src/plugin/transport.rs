use serde_json::Value;
use xiuxian_vector::{ArrowTransportClient, ArrowTransportConfig};
use xiuxian_wendao::analyzers::config::{RegisteredRepository, RepositoryPluginConfig};
use xiuxian_wendao::analyzers::errors::RepoIntelligenceError;

const JULIA_PLUGIN_ID: &str = "julia";
const ARROW_TRANSPORT_KEY: &str = "arrow_transport";

/// Build a Julia Arrow transport client from repository plugin config.
///
/// The function looks for a `RepositoryPluginConfig::Config` entry whose `id`
/// is `julia`, and then reads either a nested `arrow_transport` object or
/// direct transport keys from that plugin's `options`.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the plugin config uses invalid types
/// or cannot be converted into a valid [`ArrowTransportConfig`].
pub fn build_julia_arrow_transport_client(
    repository: &RegisteredRepository,
) -> Result<Option<ArrowTransportClient>, RepoIntelligenceError> {
    let Some(config) = build_arrow_transport_config(repository)? else {
        return Ok(None);
    };

    ArrowTransportClient::new(config)
        .map(Some)
        .map_err(|error| RepoIntelligenceError::ConfigLoad {
            message: format!(
                "failed to build Julia Arrow transport client for repo `{}`: {error}",
                repository.id
            ),
        })
}

fn build_arrow_transport_config(
    repository: &RegisteredRepository,
) -> Result<Option<ArrowTransportConfig>, RepoIntelligenceError> {
    let Some(options) = resolve_transport_options(repository)? else {
        return Ok(None);
    };

    if let Some(false) = bool_option(options, "enabled", repository)? {
        return Ok(None);
    }

    let mut config = if let Some(base_url) = string_option(options, "base_url", repository)? {
        ArrowTransportConfig::new(base_url)
    } else {
        ArrowTransportConfig::default()
    };
    if let Some(route) = string_option(options, "route", repository)? {
        config = config.with_route(route);
    }
    if let Some(health_route) = string_option(options, "health_route", repository)? {
        config = config.with_health_route(health_route);
    }
    if let Some(content_type) = string_option(options, "content_type", repository)? {
        config = config.with_content_type(content_type);
    }
    if let Some(timeout_secs) = u64_option(options, "timeout_secs", repository)? {
        config = config.with_timeout_secs(timeout_secs).map_err(|error| {
            RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "repo `{}` Julia arrow_transport timeout is invalid: {error}",
                    repository.id
                ),
            }
        })?;
    }

    Ok(Some(config))
}

fn resolve_transport_options<'a>(
    repository: &'a RegisteredRepository,
) -> Result<Option<&'a Value>, RepoIntelligenceError> {
    for plugin in &repository.plugins {
        let RepositoryPluginConfig::Config { id, options } = plugin else {
            continue;
        };
        if id != JULIA_PLUGIN_ID {
            continue;
        }

        if let Some(transport) = options.get(ARROW_TRANSPORT_KEY) {
            return object_option(transport, ARROW_TRANSPORT_KEY, repository)
                .map(Some)
                .or_else(|error| Err(error));
        }
        if contains_transport_keys(options) {
            return object_option(options, "options", repository)
                .map(Some)
                .or_else(|error| Err(error));
        }
    }
    Ok(None)
}

fn contains_transport_keys(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    [
        "enabled",
        "base_url",
        "route",
        "health_route",
        "content_type",
        "timeout_secs",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
}

fn object_option<'a>(
    value: &'a Value,
    field: &str,
    repository: &RegisteredRepository,
) -> Result<&'a Value, RepoIntelligenceError> {
    if value.is_object() {
        return Ok(value);
    }

    Err(plugin_config_type_error(repository, field, "an object"))
}

fn string_option(
    value: &Value,
    field: &str,
    repository: &RegisteredRepository,
) -> Result<Option<String>, RepoIntelligenceError> {
    let Some(raw) = value.get(field) else {
        return Ok(None);
    };
    let Some(string) = raw.as_str() else {
        return Err(plugin_config_type_error(repository, field, "a string"));
    };
    Ok(Some(string.to_string()))
}

fn bool_option(
    value: &Value,
    field: &str,
    repository: &RegisteredRepository,
) -> Result<Option<bool>, RepoIntelligenceError> {
    let Some(raw) = value.get(field) else {
        return Ok(None);
    };
    let Some(boolean) = raw.as_bool() else {
        return Err(plugin_config_type_error(repository, field, "a boolean"));
    };
    Ok(Some(boolean))
}

fn u64_option(
    value: &Value,
    field: &str,
    repository: &RegisteredRepository,
) -> Result<Option<u64>, RepoIntelligenceError> {
    let Some(raw) = value.get(field) else {
        return Ok(None);
    };
    let Some(number) = raw.as_u64() else {
        return Err(plugin_config_type_error(
            repository,
            field,
            "an unsigned integer",
        ));
    };
    Ok(Some(number))
}

fn plugin_config_type_error(
    repository: &RegisteredRepository,
    field: &str,
    expected: &str,
) -> RepoIntelligenceError {
    RepoIntelligenceError::ConfigLoad {
        message: format!(
            "repo `{}` Julia plugin field `{field}` must be {expected}",
            repository.id
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::build_julia_arrow_transport_client;
    use xiuxian_wendao::analyzers::config::{RegisteredRepository, RepositoryPluginConfig};

    #[test]
    fn build_julia_arrow_transport_client_returns_none_without_inline_config() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
            ..RegisteredRepository::default()
        };

        let client = match build_julia_arrow_transport_client(&repository) {
            Ok(client) => client,
            Err(error) => panic!("expected missing inline config to be ignored: {error}"),
        };
        assert!(client.is_none());
    }

    #[test]
    fn build_julia_arrow_transport_client_reads_nested_arrow_transport_options() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "arrow_transport": {
                        "base_url": "http://127.0.0.1:8081",
                        "route": "/analysis",
                        "health_route": "/ready",
                        "timeout_secs": 30
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let client = match build_julia_arrow_transport_client(&repository) {
            Ok(Some(client)) => client,
            Ok(None) => panic!("expected inline Julia arrow transport config"),
            Err(error) => panic!("expected nested config to build successfully: {error}"),
        };

        assert_eq!(client.config().base_url(), "http://127.0.0.1:8081");
        assert_eq!(client.config().route(), "/analysis");
        assert_eq!(client.config().health_route(), "/ready");
        assert_eq!(client.config().timeout().as_secs(), 30);
    }

    #[test]
    fn build_julia_arrow_transport_client_rejects_invalid_field_types() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "arrow_transport": {
                        "timeout_secs": "fast"
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let error = match build_julia_arrow_transport_client(&repository) {
            Ok(_) => panic!("expected invalid timeout type to fail"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("Julia plugin field `timeout_secs` must be an unsigned integer"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn build_julia_arrow_transport_client_honors_enabled_false() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "arrow_transport": {
                        "enabled": false,
                        "base_url": "http://127.0.0.1:8081"
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let client = match build_julia_arrow_transport_client(&repository) {
            Ok(client) => client,
            Err(error) => panic!("expected disabled config to be ignored: {error}"),
        };
        assert!(client.is_none());
    }
}
