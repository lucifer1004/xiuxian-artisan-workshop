use super::{
    GatewayRuntimeTomlConfig, parse_gateway_runtime_from_toml, parse_port_from_toml,
    parse_webhook_from_toml, resolve_config_path, resolve_config_path_with_project_root,
    resolve_config_path_with_project_root_value,
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
fn resolve_config_path_uses_shared_relative_prj_root_resolution() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace_path = temp.path().join("workspace");
    let nested_path = workspace_path.join("apps/studio");
    fs::create_dir_all(&nested_path)?;
    let base_path = workspace_path.join("wendao.toml");
    fs::write(&base_path, "[gateway]\nport = 9517\n")?;

    let resolved = resolve_config_path_with_project_root_value(
        None,
        Some("../.."),
        Some(nested_path.as_path()),
    )
    .unwrap_or_else(|| panic!("shared PRJ_ROOT config path should resolve"));
    assert_eq!(resolved.canonicalize()?, base_path.canonicalize()?);
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

#[test]
fn parse_gateway_runtime_from_overlay_imports() -> TestResult {
    let temp = tempfile::tempdir()?;
    let base_path = temp.path().join("wendao.toml");
    let overlay_path = temp.path().join("wendao.studio.overlay.toml");
    fs::write(
        &base_path,
        "[gateway.runtime]\nlisten_backlog = 1024\nstudio_concurrency_limit = 48\n",
    )?;
    fs::write(
        &overlay_path,
        "imports = [\"wendao.toml\"]\n[gateway.runtime]\nstudio_request_timeout_secs = 21\nstudio_concurrency_limit = 64\n",
    )?;

    assert_eq!(
        parse_gateway_runtime_from_toml(&overlay_path),
        Some(GatewayRuntimeTomlConfig {
            listen_backlog: Some(1024),
            studio_concurrency_limit: Some(64),
            studio_request_timeout_secs: Some(21),
        })
    );
    Ok(())
}
