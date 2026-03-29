use super::apply;
use super::finalize;
use crate::runtime_config::LinkGraphAgenticRuntimeConfig;
use serde_yaml::Value;

/// Resolve agentic runtime settings from merged Wendao configuration.
#[must_use]
pub fn resolve_link_graph_agentic_runtime_with_settings(
    settings: &Value,
) -> LinkGraphAgenticRuntimeConfig {
    let mut resolved = LinkGraphAgenticRuntimeConfig::default();

    apply::apply_suggested_link_settings(settings, &mut resolved);
    apply::apply_search_settings(settings, &mut resolved);
    apply::apply_expansion_settings(settings, &mut resolved);
    apply::apply_execution_settings(settings, &mut resolved);

    finalize::finalize_execution_defaults(&mut resolved);
    resolved
}

#[cfg(test)]
mod tests {
    use super::resolve_link_graph_agentic_runtime_with_settings;
    use crate::settings::{merged_toml_settings, set_link_graph_wendao_config_override};
    use serial_test::serial;
    use std::fs;

    #[test]
    #[serial]
    fn resolve_agentic_runtime_reads_override_values() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.agentic.suggested_link]
max_entries = 111
ttl_seconds = 600

[link_graph.agentic.search]
include_provisional_default = true
provisional_limit = 17

[link_graph.agentic.expansion]
max_workers = 3
max_candidates = 90
max_pairs_per_worker = 11
time_budget_ms = 44.0

[link_graph.agentic.execution]
worker_time_budget_ms = 33.0
persist_suggestions_default = true
persist_retry_attempts = 4
idempotency_scan_limit = 77
relation = "supports"
agent_id = "runtime-agent"
evidence_prefix = "runtime-prefix"
"#,
        )?;
        let config_path_string = config_path.to_string_lossy().to_string();
        set_link_graph_wendao_config_override(&config_path_string);

        let settings = merged_toml_settings("link_graph", "", "", "wendao.toml");
        let runtime = resolve_link_graph_agentic_runtime_with_settings(&settings);
        assert_eq!(runtime.suggested_link_max_entries, 111);
        assert_eq!(runtime.suggested_link_ttl_seconds, Some(600));
        assert!(runtime.search_include_provisional_default);
        assert_eq!(runtime.search_provisional_limit, 17);
        assert_eq!(runtime.expansion_max_workers, 3);
        assert_eq!(runtime.expansion_max_candidates, 90);
        assert_eq!(runtime.expansion_max_pairs_per_worker, 11);
        assert!((runtime.expansion_time_budget_ms - 44.0).abs() <= f64::EPSILON);
        assert!((runtime.execution_worker_time_budget_ms - 33.0).abs() <= f64::EPSILON);
        assert!(runtime.execution_persist_suggestions_default);
        assert_eq!(runtime.execution_persist_retry_attempts, 4);
        assert_eq!(runtime.execution_idempotency_scan_limit, 77);
        assert_eq!(runtime.execution_relation, "supports");
        assert_eq!(runtime.execution_agent_id, "runtime-agent");
        assert_eq!(runtime.execution_evidence_prefix, "runtime-prefix");

        Ok(())
    }
}
