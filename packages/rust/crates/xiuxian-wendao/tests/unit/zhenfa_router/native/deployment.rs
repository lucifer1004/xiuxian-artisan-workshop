use super::*;
use crate::set_link_graph_wendao_config_override;
use std::fs;

#[test]
fn wendao_julia_deployment_artifact_args_deserialize_empty_object() {
    let args: WendaoJuliaDeploymentArtifactArgs =
        serde_json::from_value(serde_json::json!({})).expect("empty args should deserialize");
    assert!(matches!(
        args.output_format,
        WendaoJuliaDeploymentArtifactOutputFormat::Toml
    ));
}

#[test]
fn wendao_julia_deployment_artifact_args_deserialize_json_output() {
    let args: WendaoJuliaDeploymentArtifactArgs =
        serde_json::from_value(serde_json::json!({ "output_format": "json" }))
            .expect("json args should deserialize");
    assert!(matches!(
        args.output_format,
        WendaoJuliaDeploymentArtifactOutputFormat::Json
    ));
}

#[test]
fn wendao_julia_deployment_artifact_args_deserialize_output_path() {
    let args: WendaoJuliaDeploymentArtifactArgs = serde_json::from_value(serde_json::json!({
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
fn render_julia_deployment_artifact_toml_uses_runtime_config() {
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

    let rendered = render_julia_deployment_artifact_toml().expect("render toml");
    assert!(rendered.contains("artifact_schema_version = \"v1\""));
    assert!(rendered.contains("generated_at = "));
    assert!(rendered.contains("base_url = \"http://127.0.0.1:8088\""));
    assert!(
        rendered
            .contains("launcher_path = \".data/WendaoAnalyzer/scripts/run_analyzer_service.sh\"")
    );
    assert!(rendered.contains("\"similarity_only\""));
}

#[test]
fn render_julia_deployment_artifact_json_uses_runtime_config() {
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

    let rendered = render_julia_deployment_artifact_json().expect("render json");
    assert!(rendered.contains("\"artifact_schema_version\": \"v1\""));
    assert!(rendered.contains("\"generated_at\": "));
    assert!(rendered.contains("\"base_url\": \"http://127.0.0.1:8088\""));
    assert!(rendered.contains("\"route\": \"/arrow-ipc\""));
    assert!(
        rendered.contains(
            "\"launcher_path\": \".data/WendaoAnalyzer/scripts/run_analyzer_service.sh\""
        )
    );
}

#[test]
fn export_julia_deployment_artifact_writes_json_file_when_requested() {
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
    let message = export_julia_deployment_artifact(WendaoJuliaDeploymentArtifactArgs {
        output_format: WendaoJuliaDeploymentArtifactOutputFormat::Json,
        output_path: Some(output_path.to_string_lossy().to_string()),
    })
    .expect("export json file");

    assert!(message.contains("Wrote Julia deployment artifact (json)"));
    let written = fs::read_to_string(&output_path).expect("read written json");
    assert!(written.contains("\"artifact_schema_version\": \"v1\""));
    assert!(written.contains("\"route\": \"/arrow-ipc\""));
}
