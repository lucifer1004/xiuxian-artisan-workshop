//! Gateway configuration resolution.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use log::info;
use serde::Deserialize;
use xiuxian_config_core::load_toml_value_with_imports;
use xiuxian_wendao::gateway::studio::studio_effective_wendao_toml_path;
use xiuxian_zhenfa::WebhookConfig;

use crate::execute::gateway::shared::DEFAULT_PORT;

/// Resolve the effective config file from CLI override, local project file, or
/// `PRJ_ROOT`.
pub(crate) fn resolve_config_path(cli_config: Option<&Path>) -> Option<PathBuf> {
    let project_root = std::env::var_os("PRJ_ROOT").map(PathBuf::from);
    resolve_config_path_with_project_root(cli_config, project_root.as_deref())
}

fn resolve_config_path_with_project_root(
    cli_config: Option<&Path>,
    project_root: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(path) = cli_config {
        return Some(resolve_effective_config_path(path));
    }

    let local_config = Path::new("wendao.toml");
    if local_config.exists() {
        return Some(resolve_effective_config_path(local_config));
    }

    let config_path = project_root.map(|root| root.join("wendao.toml"))?;
    config_path
        .exists()
        .then(|| resolve_effective_config_path(config_path.as_path()))
}

/// Resolve the port from CLI arg, config file, or default.
pub(crate) fn resolve_port(cli_port: Option<u16>, config_path: Option<&Path>) -> u16 {
    if let Some(port) = cli_port {
        return port;
    }

    if let Some(config_port) = get_port_from_config(config_path) {
        return config_port;
    }

    DEFAULT_PORT
}

/// Get port from wendao.toml config file.
pub(crate) fn get_port_from_config(config_path: Option<&Path>) -> Option<u16> {
    parse_port_from_toml(config_path?)
}

/// Parse port from a TOML config file.
pub(crate) fn parse_port_from_toml(path: &Path) -> Option<u16> {
    load_gateway_toml(path).and_then(|config| config.gateway.port())
}

/// Resolve webhook config with priority: TOML > env var > defaults.
pub(crate) fn resolve_webhook_config(config_path: Option<&Path>) -> WebhookConfig {
    if let Some(config) = get_webhook_from_config(config_path) {
        info!("Gateway: Using webhook config from wendao.toml");
        return config;
    }

    let url = std::env::var("WENDAO_WEBHOOK_URL").unwrap_or_default();
    if !url.is_empty() {
        info!("Gateway: Using webhook config from WENDAO_WEBHOOK_URL env var");
    }

    WebhookConfig {
        url,
        secret: std::env::var("WENDAO_WEBHOOK_SECRET").ok(),
        timeout_secs: 10,
        retry_on_failure: true,
    }
}

/// Get webhook config from wendao.toml config file.
pub(crate) fn get_webhook_from_config(config_path: Option<&Path>) -> Option<WebhookConfig> {
    parse_webhook_from_toml(config_path?)
}

/// Parse webhook config from a TOML config file.
pub(crate) fn parse_webhook_from_toml(path: &Path) -> Option<WebhookConfig> {
    load_gateway_toml(path).and_then(|config| config.gateway.webhook_config())
}

fn resolve_effective_config_path(path: &Path) -> PathBuf {
    let Some(file_name) = path.file_name() else {
        return path.to_path_buf();
    };
    if file_name != "wendao.toml" {
        return path.to_path_buf();
    }

    let Some(config_root) = path.parent() else {
        return path.to_path_buf();
    };
    let effective_path = studio_effective_wendao_toml_path(config_root);
    if effective_path.is_file() {
        effective_path
    } else {
        path.to_path_buf()
    }
}

fn load_gateway_toml(path: &Path) -> Option<GatewayTomlConfig> {
    let merged = load_toml_value_with_imports(path).ok()?;
    merged.try_into().ok()
}

#[derive(Debug, Default, Deserialize)]
struct GatewayTomlConfig {
    #[serde(default)]
    gateway: GatewayTomlSection,
}

#[derive(Debug, Default, Deserialize)]
struct GatewayTomlSection {
    #[serde(default)]
    bind: Option<String>,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    webhook_url: Option<String>,
    #[serde(default)]
    webhook_secret: Option<String>,
    #[serde(default)]
    webhook_enabled: Option<bool>,
}

impl GatewayTomlSection {
    fn port(&self) -> Option<u16> {
        self.port
            .or_else(|| self.bind.as_deref().and_then(parse_bind_port))
    }

    fn webhook_config(&self) -> Option<WebhookConfig> {
        if self.webhook_enabled == Some(false) {
            return None;
        }

        self.webhook_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|url| WebhookConfig {
                url: url.to_string(),
                secret: self
                    .webhook_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                timeout_secs: 10,
                retry_on_failure: true,
            })
    }
}

fn parse_bind_port(bind: &str) -> Option<u16> {
    bind.parse::<SocketAddr>()
        .ok()
        .map(|address| address.port())
        .or_else(|| {
            bind.rsplit_once(':')
                .and_then(|(_, port)| port.trim().parse::<u16>().ok())
        })
}

#[cfg(test)]
mod tests {
    use super::{
        parse_port_from_toml, parse_webhook_from_toml, resolve_config_path,
        resolve_config_path_with_project_root,
    };
    use std::fs;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn resolve_config_path_prefers_studio_overlay_when_present() -> TestResult {
        let temp = tempfile::tempdir()?;
        let base_path = temp.path().join("wendao.toml");
        let overlay_path = temp.path().join("wendao.studio.overlay.toml");
        fs::write(&base_path, "[gateway]\nport = 9517\n")?;
        fs::write(
            &overlay_path,
            "imports = [\"wendao.toml\"]\n[gateway]\nport = 9610\n",
        )?;

        let resolved = resolve_config_path(Some(base_path.as_path()))
            .unwrap_or_else(|| panic!("effective config path should resolve"));
        assert_eq!(resolved, overlay_path);
        Ok(())
    }

    #[test]
    fn resolve_config_path_falls_back_to_prj_root_wendao_toml() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace_path = temp.path();
        let base_path = workspace_path.join("wendao.toml");
        fs::write(&base_path, "[gateway]\nport = 9517\n")?;

        let resolved = resolve_config_path_with_project_root(None, Some(workspace_path))
            .unwrap_or_else(|| panic!("PRJ_ROOT config path should resolve"));
        assert_eq!(resolved, base_path);
        Ok(())
    }

    #[test]
    fn parse_gateway_config_from_overlay_imports() -> TestResult {
        let temp = tempfile::tempdir()?;
        let base_path = temp.path().join("wendao.toml");
        let overlay_path = temp.path().join("wendao.studio.overlay.toml");
        fs::write(
            &base_path,
            "[gateway]\nport = 9517\nwebhook_url = \"http://127.0.0.1:9000/base\"\n",
        )?;
        fs::write(
            &overlay_path,
            "imports = [\"wendao.toml\"]\n[gateway]\nport = 9610\nwebhook_url = \"http://127.0.0.1:9000/overlay\"\n",
        )?;

        assert_eq!(parse_port_from_toml(&overlay_path), Some(9610));
        let webhook = parse_webhook_from_toml(&overlay_path)
            .unwrap_or_else(|| panic!("webhook config should resolve from overlay"));
        assert_eq!(webhook.url, "http://127.0.0.1:9000/overlay");
        Ok(())
    }
}
