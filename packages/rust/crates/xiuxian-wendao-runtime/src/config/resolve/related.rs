use crate::config::LinkGraphRelatedRuntimeConfig;
use crate::config::constants::{
    LINK_GRAPH_RELATED_MAX_CANDIDATES_ENV, LINK_GRAPH_RELATED_MAX_PARTITIONS_ENV,
    LINK_GRAPH_RELATED_TIME_BUDGET_MS_ENV,
};
use crate::settings::{
    first_non_empty, get_setting_string, parse_positive_f64, parse_positive_usize,
};
use serde_yaml::Value;

/// Resolve related-query runtime settings from merged Wendao configuration.
#[must_use]
pub fn resolve_link_graph_related_runtime_with_settings(
    settings: &Value,
) -> LinkGraphRelatedRuntimeConfig {
    let mut resolved = LinkGraphRelatedRuntimeConfig::default();

    if let Some(value) = first_non_empty(&[
        get_setting_string(settings, "link_graph.related.max_candidates"),
        std::env::var(LINK_GRAPH_RELATED_MAX_CANDIDATES_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.max_candidates = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(settings, "link_graph.related.max_partitions"),
        std::env::var(LINK_GRAPH_RELATED_MAX_PARTITIONS_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_usize)
    {
        resolved.max_partitions = value;
    }

    if let Some(value) = first_non_empty(&[
        get_setting_string(settings, "link_graph.related.time_budget_ms"),
        std::env::var(LINK_GRAPH_RELATED_TIME_BUDGET_MS_ENV).ok(),
    ])
    .as_deref()
    .and_then(parse_positive_f64)
    {
        resolved.time_budget_ms = value;
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::resolve_link_graph_related_runtime_with_settings;
    use crate::config::test_support;
    use std::fs;

    #[test]
    fn resolve_related_runtime_reads_override_values() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r"[link_graph.related]
max_candidates = 512
max_partitions = 4
time_budget_ms = 75.0
",
        )?;

        let settings = test_support::load_test_settings_from_path(&config_path)?;
        let runtime = resolve_link_graph_related_runtime_with_settings(&settings);
        assert_eq!(runtime.max_candidates, 512);
        assert_eq!(runtime.max_partitions, 4);
        assert!((runtime.time_budget_ms - 75.0).abs() <= f64::EPSILON);

        Ok(())
    }
}
