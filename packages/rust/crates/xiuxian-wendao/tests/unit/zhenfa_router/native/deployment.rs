use super::*;
use crate::set_link_graph_wendao_config_override;
use serial_test::serial;
use std::fs;
use xiuxian_wendao_julia::compatibility::link_graph::{
    DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH, JULIA_DEPLOYMENT_ARTIFACT_ID, JULIA_PLUGIN_ID,
};

#[test]
fn wendao_plugin_artifact_args_deserialize_selector_and_format() {
    let args: WendaoPluginArtifactArgs = serde_json::from_value(serde_json::json!({
        "plugin_id": JULIA_PLUGIN_ID,
        "artifact_id": JULIA_DEPLOYMENT_ARTIFACT_ID,
        "output_format": "json"
    }))
    .expect("generic plugin-artifact args should deserialize");

    assert_eq!(args.plugin_id, JULIA_PLUGIN_ID);
    assert_eq!(args.artifact_id, JULIA_DEPLOYMENT_ARTIFACT_ID);
    assert!(matches!(
        args.output_format,
        WendaoPluginArtifactOutputFormat::Json
    ));
}

#[test]
fn compat_deployment_artifact_args_deserialize_empty_object() {
    let args: WendaoCompatDeploymentArtifactArgs =
        serde_json::from_value(serde_json::json!({})).expect("empty args should deserialize");
    assert!(matches!(
        args.output_format,
        WendaoCompatDeploymentArtifactOutputFormat::Toml
    ));
}

#[test]
fn compat_deployment_artifact_args_deserialize_json_output() {
    let args: WendaoCompatDeploymentArtifactArgs =
        serde_json::from_value(serde_json::json!({ "output_format": "json" }))
            .expect("json args should deserialize");
    assert!(matches!(
        args.output_format,
        WendaoCompatDeploymentArtifactOutputFormat::Json
    ));
}

#[test]
fn compat_deployment_artifact_args_deserialize_output_path() {
    let args: WendaoCompatDeploymentArtifactArgs = serde_json::from_value(serde_json::json!({
        "output_format": "json",
        "output_path": ".run/julia/artifact.json"
    }))
    .expect("args with output_path should deserialize");

    assert_eq!(
        args.output_path.as_deref(),
        Some(".run/julia/artifact.json")
    );
}

#[test]
fn compat_deployment_artifact_args_accept_legacy_empty_shape() {
    let args: WendaoCompatDeploymentArtifactArgs =
        serde_json::from_value(serde_json::json!({ "output_format": "json" }))
            .expect("compat args should deserialize");
    assert!(matches!(
        args.output_format,
        WendaoCompatDeploymentArtifactOutputFormat::Json
    ));
}

#[test]
#[serial]
fn render_compat_deployment_artifact_toml_uses_runtime_config() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/arrow-ipc"
schema_version = "v1"
service_mode = "stream"
analyzer_strategy = "similarity_only"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let rendered = render_compat_deployment_artifact_toml().expect("render toml");
    assert!(rendered.contains("artifact_schema_version = \"v1\""));
    assert!(rendered.contains("generated_at = "));
    assert!(rendered.contains("base_url = \"http://127.0.0.1:8088\""));
    assert!(rendered.contains(&format!(
        "launcher_path = \"{DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH}\""
    )));
    assert!(rendered.contains("\"similarity_only\""));
}

#[test]
#[serial]
fn render_compat_deployment_artifact_json_uses_runtime_config() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/arrow-ipc"
schema_version = "v1"
service_mode = "stream"
analyzer_strategy = "similarity_only"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let rendered = render_compat_deployment_artifact_json().expect("render json");
    assert!(rendered.contains("\"artifact_schema_version\": \"v1\""));
    assert!(rendered.contains("\"generated_at\": "));
    assert!(rendered.contains("\"base_url\": \"http://127.0.0.1:8088\""));
    assert!(rendered.contains("\"route\": \"/arrow-ipc\""));
    assert!(rendered.contains(&format!(
        "\"launcher_path\": \"{DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH}\""
    )));
}

#[test]
#[serial]
fn export_compat_deployment_artifact_writes_json_file_when_requested() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/arrow-ipc"
schema_version = "v1"
service_mode = "stream"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let output_path = temp.path().join("exports").join("artifact.json");
    let message = export_compat_deployment_artifact(WendaoCompatDeploymentArtifactArgs {
        output_format: WendaoCompatDeploymentArtifactOutputFormat::Json,
        output_path: Some(output_path.to_string_lossy().to_string()),
    })
    .expect("export json file");

    assert!(message.contains("Wrote compatibility deployment artifact (json)"));
    let written = fs::read_to_string(&output_path).expect("read written json");
    assert!(written.contains("\"artifact_schema_version\": \"v1\""));
    assert!(written.contains("\"route\": \"/arrow-ipc\""));
}

#[test]
#[serial]
fn export_plugin_artifact_writes_json_file_when_requested() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/arrow-ipc"
schema_version = "v1"
service_mode = "stream"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let output_path = temp.path().join("exports").join("plugin-artifact.json");
    let message = export_plugin_artifact(WendaoPluginArtifactArgs {
        plugin_id: JULIA_PLUGIN_ID.to_string(),
        artifact_id: JULIA_DEPLOYMENT_ARTIFACT_ID.to_string(),
        output_format: WendaoPluginArtifactOutputFormat::Json,
        output_path: Some(output_path.to_string_lossy().to_string()),
    })
    .expect("export generic plugin artifact");

    assert!(message.contains("Wrote plugin artifact"));
    assert!(message.contains(JULIA_PLUGIN_ID));
    assert!(message.contains(JULIA_DEPLOYMENT_ARTIFACT_ID));
    let written = fs::read_to_string(&output_path).expect("read written json");
    assert!(written.contains("\"artifact_schema_version\": \"v1\""));
    assert!(written.contains("\"route\": \"/arrow-ipc\""));
}

#[test]
#[serial]
fn compat_deployment_artifact_helpers_still_render_and_export() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/arrow-ipc"
schema_version = "v1"
service_mode = "stream"
"#,
    )
    .expect("write config");
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let rendered_toml = render_compat_deployment_artifact_toml().expect("compat render toml");
    assert!(rendered_toml.contains("artifact_schema_version = \"v1\""));

    let rendered_json = render_compat_deployment_artifact_json().expect("compat render json");
    assert!(rendered_json.contains("\"artifact_schema_version\": \"v1\""));

    let rendered_via_format = render_compat_deployment_artifact(
        WendaoCompatDeploymentArtifactOutputFormat::Toml,
    )
    .expect("compat render by format");
    assert!(rendered_via_format.contains("route = \"/arrow-ipc\""));

    let exported = export_compat_deployment_artifact(WendaoCompatDeploymentArtifactArgs {
        output_format: WendaoCompatDeploymentArtifactOutputFormat::Json,
        output_path: None,
    })
    .expect("compat export");
    assert!(exported.contains("\"route\": \"/arrow-ipc\""));
}
