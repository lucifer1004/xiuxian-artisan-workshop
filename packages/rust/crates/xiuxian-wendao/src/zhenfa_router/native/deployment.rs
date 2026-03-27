use schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;
use xiuxian_zhenfa::{ZhenfaContext, ZhenfaError, zhenfa_tool};

use crate::{
    export_link_graph_julia_deployment_artifact_toml, resolve_link_graph_julia_deployment_artifact,
};

/// Output formats for visible Julia deployment artifact export.
#[derive(Debug, Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WendaoJuliaDeploymentArtifactOutputFormat {
    /// Render the deployment artifact as TOML.
    #[default]
    Toml,
    /// Render the deployment artifact as structured JSON.
    Json,
}

/// Arguments for exporting the resolved Julia deployment artifact.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct WendaoJuliaDeploymentArtifactArgs {
    /// Optional output format. Defaults to TOML.
    #[serde(default)]
    pub output_format: WendaoJuliaDeploymentArtifactOutputFormat,
    /// Optional destination path for persisting the rendered artifact.
    #[serde(default)]
    pub output_path: Option<String>,
}

/// Export the resolved Julia deployment artifact.
///
/// # Errors
///
/// Returns a [`ZhenfaError`] when the current Julia deployment artifact cannot
/// be serialized into the requested format.
#[allow(missing_docs)]
#[allow(clippy::needless_pass_by_value)]
#[zhenfa_tool(
    name = "wendao.julia_deployment_artifact",
    description = "Export the resolved Julia deployment artifact as TOML or structured JSON.",
    tool_struct = "WendaoJuliaDeploymentArtifactTool"
)]
pub fn wendao_julia_deployment_artifact(
    _ctx: &ZhenfaContext,
    args: WendaoJuliaDeploymentArtifactArgs,
) -> Result<String, ZhenfaError> {
    export_julia_deployment_artifact(args)
}

/// Render the resolved Julia deployment artifact as TOML.
///
/// # Errors
///
/// Returns a [`ZhenfaError`] when TOML serialization fails.
pub fn render_julia_deployment_artifact_toml() -> Result<String, ZhenfaError> {
    export_link_graph_julia_deployment_artifact_toml().map_err(|error| {
        ZhenfaError::execution(format!("export Julia deployment artifact: {error}"))
    })
}

/// Render the resolved Julia deployment artifact as structured JSON.
///
/// # Errors
///
/// Returns a [`ZhenfaError`] when JSON serialization fails.
pub fn render_julia_deployment_artifact_json() -> Result<String, ZhenfaError> {
    resolve_link_graph_julia_deployment_artifact()
        .to_json_string()
        .map_err(|error| {
            ZhenfaError::execution(format!("export Julia deployment artifact as json: {error}"))
        })
}

/// Render the resolved Julia deployment artifact using the selected format.
///
/// # Errors
///
/// Returns a [`ZhenfaError`] when serialization fails.
pub fn render_julia_deployment_artifact(
    output_format: WendaoJuliaDeploymentArtifactOutputFormat,
) -> Result<String, ZhenfaError> {
    match output_format {
        WendaoJuliaDeploymentArtifactOutputFormat::Toml => render_julia_deployment_artifact_toml(),
        WendaoJuliaDeploymentArtifactOutputFormat::Json => render_julia_deployment_artifact_json(),
    }
}

/// Export the resolved Julia deployment artifact, optionally writing it to a file.
///
/// # Errors
///
/// Returns a [`ZhenfaError`] when serialization or file writing fails.
pub fn export_julia_deployment_artifact(
    args: WendaoJuliaDeploymentArtifactArgs,
) -> Result<String, ZhenfaError> {
    if let Some(output_path) = args.output_path.as_deref() {
        let artifact = resolve_link_graph_julia_deployment_artifact();
        let path = Path::new(output_path);
        match args.output_format {
            WendaoJuliaDeploymentArtifactOutputFormat::Toml => artifact.write_toml_file(path),
            WendaoJuliaDeploymentArtifactOutputFormat::Json => artifact.write_json_file(path),
        }
        .map_err(|error| {
            ZhenfaError::execution(format!(
                "write Julia deployment artifact to {}: {error}",
                path.display()
            ))
        })?;

        return Ok(format!(
            "Wrote Julia deployment artifact ({}) to {}",
            match args.output_format {
                WendaoJuliaDeploymentArtifactOutputFormat::Toml => "toml",
                WendaoJuliaDeploymentArtifactOutputFormat::Json => "json",
            },
            path.display()
        ));
    }

    render_julia_deployment_artifact(args.output_format)
}

#[cfg(test)]
#[path = "../../../tests/unit/zhenfa_router/native/deployment.rs"]
mod tests;
