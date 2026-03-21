use std::fs;
use std::path::Path;

use serde::Serialize;

#[path = "repo_fixture.rs"]
mod repo_fixture;

pub use repo_fixture::{TestResultPath, create_sample_julia_repo};

pub fn assert_repo_json_snapshot(name: &str, value: impl Serialize) {
    insta::with_settings!({
        snapshot_path => "../snapshots/repo_intelligence",
        prepend_module_to_snapshot => false,
        sort_maps => true,
    }, {
        insta::assert_json_snapshot!(name, value);
    });
}

pub fn write_repo_config(base: &Path, repo_dir: &Path, repo_id: &str) -> TestResultPath {
    let config_path = base.join(format!("{repo_id}.wendao.toml"));
    fs::write(
        &config_path,
        format!(
            r#"[link_graph.projects.{repo_id}]
root = "{}"
plugins = ["julia"]
"#,
            repo_dir.display()
        ),
    )?;
    Ok(config_path)
}
