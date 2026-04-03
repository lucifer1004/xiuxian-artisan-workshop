use crate::runtime_config::LinkGraphIndexRuntimeConfig;
use crate::settings::{
    dedup_dirs, get_setting_bool, get_setting_string_list, normalize_relative_dir,
};
use serde_yaml::Value;
use std::path::Path;

/// Resolve `LinkGraph` index scope from merged `wendao` settings.
#[must_use]
pub fn resolve_link_graph_index_runtime_with_settings(
    root_dir: &Path,
    settings: &Value,
) -> LinkGraphIndexRuntimeConfig {
    let explicit_include = dedup_dirs(
        get_setting_string_list(settings, "link_graph.include_dirs")
            .into_iter()
            .filter_map(|item| normalize_relative_dir(&item))
            .collect(),
    );

    let include_dirs = if explicit_include.is_empty()
        && get_setting_bool(settings, "link_graph.include_dirs_auto").unwrap_or(true)
    {
        dedup_dirs(
            get_setting_string_list(settings, "link_graph.include_dirs_auto_candidates")
                .into_iter()
                .filter_map(|item| normalize_relative_dir(&item))
                .filter(|candidate| root_dir.join(candidate).is_dir())
                .collect(),
        )
    } else {
        explicit_include
    };

    let exclude_dirs = dedup_dirs(
        get_setting_string_list(settings, "link_graph.exclude_dirs")
            .into_iter()
            .filter_map(|item| normalize_relative_dir(&item))
            .filter(|value| !value.starts_with('.'))
            .collect(),
    );

    LinkGraphIndexRuntimeConfig {
        include_dirs,
        exclude_dirs,
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_link_graph_index_runtime_with_settings;
    use crate::runtime_config::test_support;
    use std::fs;

    #[test]
    fn resolve_index_runtime_filters_hidden_excludes_and_uses_auto_candidates()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        fs::create_dir_all(temp.path().join("src"))?;
        fs::create_dir_all(temp.path().join("docs"))?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph]
include_dirs_auto = true
include_dirs_auto_candidates = ["src", "docs", "missing"]
exclude_dirs = [".git", "target", "target"]
"#,
        )?;

        let settings = test_support::load_test_settings_from_path(&config_path)?;
        let runtime = resolve_link_graph_index_runtime_with_settings(temp.path(), &settings);
        assert_eq!(
            runtime.include_dirs,
            vec!["src".to_string(), "docs".to_string()]
        );
        assert_eq!(runtime.exclude_dirs, vec!["target".to_string()]);

        Ok(())
    }
}
