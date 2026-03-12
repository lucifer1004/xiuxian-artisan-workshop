use super::builder::resolve_agenda_validation_policy;
use super::hot_reload::{
    resolve_wendao_incremental_policy, resolve_wendao_watch_patterns, resolve_wendao_watch_roots,
};
use super::memory::resolve_memory_embed_base_url;
use super::qianhuan::{
    load_skill_personas_from_embedded_registry, resolve_persona_dirs, resolve_template_dirs,
};
use super::zhixing::{
    load_skill_templates_from_embedded_registry, resolve_notebook_root, resolve_prj_data_home,
    resolve_project_root, resolve_template_globs,
};
use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use xiuxian_qianhuan::{ManifestationInterface, ManifestationManager, PersonaRegistry};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn acquire_env_lock() -> MutexGuard<'static, ()> {
    env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn restore_env_var(name: &str, previous: Option<String>) {
    #[allow(unsafe_code)]
    unsafe {
        if let Some(value) = previous {
            env::set_var(name, value);
        } else {
            env::remove_var(name);
        }
    }
}

#[test]
#[allow(unsafe_code)]
fn resolve_agenda_validation_policy_prefers_env_then_settings() {
    let _guard = acquire_env_lock();
    let previous = env::var("OMNI_AGENT_AGENDA_VALIDATION_POLICY").ok();
    let mut settings = crate::config::RuntimeSettings::default();
    settings.agent.agenda_validation_policy = Some("auto".to_string());

    unsafe {
        env::set_var("OMNI_AGENT_AGENDA_VALIDATION_POLICY", "never");
    }
    assert_eq!(resolve_agenda_validation_policy(&settings), "never");

    unsafe {
        env::remove_var("OMNI_AGENT_AGENDA_VALIDATION_POLICY");
    }
    assert_eq!(resolve_agenda_validation_policy(&settings), "auto");

    restore_env_var("OMNI_AGENT_AGENDA_VALIDATION_POLICY", previous);
}

#[test]
#[allow(unsafe_code)]
fn resolve_agenda_validation_policy_invalid_values_fallback_to_default() {
    let _guard = acquire_env_lock();
    let previous = env::var("OMNI_AGENT_AGENDA_VALIDATION_POLICY").ok();
    let settings = crate::config::RuntimeSettings::default();

    unsafe {
        env::set_var("OMNI_AGENT_AGENDA_VALIDATION_POLICY", "unexpected");
    }
    assert_eq!(resolve_agenda_validation_policy(&settings), "always");

    restore_env_var("OMNI_AGENT_AGENDA_VALIDATION_POLICY", previous);
}

#[test]
#[allow(unsafe_code)]
fn resolve_project_root_prefers_prj_root_env() {
    let _guard = acquire_env_lock();
    let previous = env::var("PRJ_ROOT").ok();
    unsafe {
        env::set_var("PRJ_ROOT", "/tmp/xiuxian-root");
    }

    let resolved = resolve_project_root();
    assert_eq!(resolved, PathBuf::from("/tmp/xiuxian-root"));

    restore_env_var("PRJ_ROOT", previous);
}

#[test]
#[allow(unsafe_code)]
fn resolve_prj_data_home_prefers_env_then_defaults() {
    let _guard = acquire_env_lock();
    let previous = env::var("PRJ_DATA_HOME").ok();

    let project_root = Path::new("/tmp/project");

    unsafe {
        env::set_var("PRJ_DATA_HOME", "/tmp/custom-data");
    }
    assert_eq!(
        resolve_prj_data_home(project_root),
        PathBuf::from("/tmp/custom-data")
    );

    unsafe {
        env::remove_var("PRJ_DATA_HOME");
    }
    assert_eq!(
        resolve_prj_data_home(project_root),
        PathBuf::from("/tmp/project/.data")
    );

    restore_env_var("PRJ_DATA_HOME", previous);
}

#[test]
fn resolve_notebook_root_precedence() {
    let data_home = Path::new("/tmp/project/.data");

    let from_env = resolve_notebook_root(
        data_home,
        Some("/tmp/notebook-env".to_string()),
        Some("/tmp/notebook-config".to_string()),
    );
    assert_eq!(from_env, PathBuf::from("/tmp/notebook-env"));

    let from_config = resolve_notebook_root(data_home, None, Some("/tmp/notebook-config".into()));
    assert_eq!(from_config, PathBuf::from("/tmp/notebook-config"));

    let fallback = resolve_notebook_root(data_home, None, None);
    assert_eq!(
        fallback,
        PathBuf::from("/tmp/project/.data/xiuxian/notebook")
    );
}

#[test]
fn resolve_memory_embed_base_url_uses_inproc_label_for_mistral_sdk_backend() {
    let memory_cfg = crate::config::MemoryConfig {
        embedding_backend: Some("mistral_sdk".to_string()),
        embedding_base_url: Some("http://127.0.0.1:3002".to_string()),
        ..crate::config::MemoryConfig::default()
    };

    let mut runtime_settings = crate::config::RuntimeSettings::default();
    runtime_settings.embedding.litellm_api_base = Some("http://127.0.0.1:11434".to_string());
    runtime_settings.mistral.base_url = Some("http://127.0.0.1:11500".to_string());

    let resolved = resolve_memory_embed_base_url(&memory_cfg, &runtime_settings);
    assert_eq!(resolved, "inproc://mistral-sdk");
}

#[test]
fn resolve_template_globs_prefers_configured_existing_paths() {
    let project_root = std::env::temp_dir().join(format!(
        "xiuxian-template-globs-project-{}",
        std::process::id()
    ));
    let relative_templates = project_root.join("custom/templates");
    let absolute_templates = std::env::temp_dir().join(format!(
        "xiuxian-template-globs-absolute-{}",
        std::process::id()
    ));
    if let Err(error) = fs::create_dir_all(&relative_templates) {
        panic!("create relative templates dir: {error}");
    }
    if let Err(error) = fs::create_dir_all(&absolute_templates) {
        panic!("create absolute templates dir: {error}");
    }

    let globs = resolve_template_globs(
        &project_root,
        Some(vec![
            "custom/templates".to_string(),
            absolute_templates.display().to_string(),
            "   ".to_string(),
        ]),
    );
    assert_eq!(
        globs,
        vec![
            relative_templates.join("*.md").display().to_string(),
            absolute_templates.join("*.md").display().to_string()
        ]
    );

    let _ = fs::remove_dir_all(&project_root);
    let _ = fs::remove_dir_all(&absolute_templates);
}

#[test]
fn resolve_template_globs_returns_empty_when_no_external_paths_exist() {
    let project_root = Path::new("/tmp/project");
    let globs = resolve_template_globs(project_root, None);
    assert!(globs.is_empty());
}

#[test]
#[allow(unsafe_code)]
fn resolve_template_globs_prefers_xiuxian_resource_root_when_present() {
    let _guard = acquire_env_lock();
    let previous = env::var("XIUXIAN_RESOURCE_ROOT").ok();
    let temp_root = std::env::temp_dir().join(format!(
        "xiuxian-resource-root-{}-{}",
        std::process::id(),
        "bootstrap-tests"
    ));
    let template_root = temp_root
        .join("omni-agent")
        .join("zhixing")
        .join("templates");
    if let Err(error) = fs::create_dir_all(&template_root) {
        panic!("create temp template root: {error}");
    }

    unsafe {
        env::set_var(
            "XIUXIAN_RESOURCE_ROOT",
            temp_root.to_string_lossy().to_string(),
        );
    }

    let globs = resolve_template_globs(Path::new("/tmp/project"), None);
    assert_eq!(
        globs[0],
        template_root.join("*.md").to_string_lossy().into_owned()
    );

    restore_env_var("XIUXIAN_RESOURCE_ROOT", previous);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn load_skill_templates_from_embedded_registry_uses_semantic_wendao_uri_links() {
    let manager = ManifestationManager::new_with_embedded_templates(
        &[],
        &[("probe.md", "Skill bridge probe: {{ marker }}")],
    )
    .unwrap_or_else(|error| panic!("create manifestation manager probe: {error}"));
    let summary = load_skill_templates_from_embedded_registry(&manager)
        .unwrap_or_else(|error| panic!("load skill templates from embedded registry: {error}"));
    assert!(summary.linked_ids >= 1);
    assert!(summary.template_records >= 1);
    assert!(summary.loaded_template_names >= 1);

    let rendered = manager
        .render_template(
            "probe.md",
            json!({
                "marker": "ok"
            }),
        )
        .unwrap_or_else(|error| panic!("render probe template after bridge load: {error}"));
    assert!(rendered.contains("Skill bridge probe: ok"));

    let agenda_rendered = manager
        .render_template(
            "draft_agenda.j2",
            json!({
                "user_request": "Test semantic skill bus loading",
            }),
        )
        .unwrap_or_else(|error| panic!("render semantic linked draft agenda: {error}"));
    assert!(agenda_rendered.contains("<agenda_draft>"));
}

#[test]
fn load_skill_personas_from_embedded_registry_accepts_no_persona_blocks() {
    let mut registry = PersonaRegistry::new();
    let summary = load_skill_personas_from_embedded_registry(&mut registry)
        .unwrap_or_else(|error| panic!("load skill personas from embedded registry: {error}"));
    assert_eq!(summary.persona_blocks, 0);
    assert_eq!(summary.loaded_personas, 0);
    assert!(registry.is_empty());
}

#[test]
fn resolve_persona_dirs_prefers_qianhuan_persona_dirs() {
    let project_root = Path::new("/tmp/project");
    let mut config = crate::config::XiuxianConfig::default();
    config.qianhuan.persona.persona_dirs = Some(vec![
        "assets/personas".to_string(),
        "/opt/personas".to_string(),
    ]);

    let dirs = resolve_persona_dirs(project_root, &config);
    assert_eq!(
        dirs,
        vec![
            PathBuf::from("/tmp/project/assets/personas"),
            PathBuf::from("/opt/personas")
        ]
    );
}

#[test]
fn resolve_persona_dirs_persona_dir_uses_explicit_override_only() {
    let project_root = Path::new("/tmp/project");
    let mut config = crate::config::XiuxianConfig::default();
    config.qianhuan.persona.persona_dir = Some("./local-personas".to_string());

    let dirs = resolve_persona_dirs(project_root, &config);
    assert_eq!(dirs, vec![PathBuf::from("/tmp/project/local-personas"),]);
}

#[test]
#[allow(unsafe_code)]
fn resolve_persona_dirs_defaults_to_prj_config_home_personas() {
    let _guard = acquire_env_lock();
    let previous = env::var("PRJ_CONFIG_HOME").ok();
    unsafe {
        env::set_var("PRJ_CONFIG_HOME", "/tmp/custom-config");
    }

    let project_root = Path::new("/tmp/project");
    let config = crate::config::XiuxianConfig::default();
    let dirs = resolve_persona_dirs(project_root, &config);
    assert_eq!(
        dirs,
        vec![PathBuf::from("/tmp/custom-config/omni-dev-fusion/personas")]
    );

    restore_env_var("PRJ_CONFIG_HOME", previous);
}

#[test]
fn resolve_template_dirs_prefers_qianhuan_template_dirs() {
    let project_root = Path::new("/tmp/project");
    let mut config = crate::config::XiuxianConfig::default();
    config.qianhuan.template.template_dirs = Some(vec![
        "assets/qianhuan/templates".to_string(),
        "/opt/qianhuan/templates".to_string(),
    ]);

    let dirs = resolve_template_dirs(project_root, &config);
    assert_eq!(
        dirs,
        vec![
            PathBuf::from("/tmp/project/assets/qianhuan/templates"),
            PathBuf::from("/opt/qianhuan/templates")
        ]
    );
}

#[test]
fn resolve_template_dirs_template_dir_uses_explicit_override_only() {
    let project_root = Path::new("/tmp/project");
    let mut config = crate::config::XiuxianConfig::default();
    config.qianhuan.template.template_dir = Some("./local-qianhuan/templates".to_string());

    let dirs = resolve_template_dirs(project_root, &config);
    assert_eq!(
        dirs,
        vec![PathBuf::from("/tmp/project/local-qianhuan/templates")]
    );
}

#[test]
#[allow(unsafe_code)]
fn resolve_template_dirs_defaults_to_prj_config_home_qianhuan_templates() {
    let _guard = acquire_env_lock();
    let previous = env::var("PRJ_CONFIG_HOME").ok();
    unsafe {
        env::set_var("PRJ_CONFIG_HOME", "/tmp/custom-config");
    }

    let project_root = Path::new("/tmp/project");
    let config = crate::config::XiuxianConfig::default();
    let dirs = resolve_template_dirs(project_root, &config);
    assert_eq!(
        dirs,
        vec![PathBuf::from(
            "/tmp/custom-config/omni-dev-fusion/qianhuan/templates"
        )]
    );

    restore_env_var("PRJ_CONFIG_HOME", previous);
}

#[test]
fn resolve_wendao_watch_roots_prefers_configured_watch_dirs() {
    let project_root = Path::new("/tmp/project");
    let roots = resolve_wendao_watch_roots(
        project_root,
        Path::new("/tmp/project/.data/xiuxian/notebook"),
        Some(&vec![
            "docs".to_string(),
            "/opt/shared-notes".to_string(),
            " ".to_string(),
        ]),
        None,
    );
    assert_eq!(
        roots,
        vec![
            PathBuf::from("/opt/shared-notes"),
            PathBuf::from("/tmp/project/docs")
        ]
    );
}

#[test]
fn resolve_wendao_watch_roots_falls_back_to_default_notebook_root() {
    let project_root = Path::new("/tmp/project");
    let roots = resolve_wendao_watch_roots(
        project_root,
        Path::new("/tmp/project/.data/xiuxian/notebook"),
        None,
        None,
    );
    assert_eq!(
        roots,
        vec![PathBuf::from("/tmp/project/.data/xiuxian/notebook")]
    );
}

#[test]
fn resolve_wendao_incremental_policy_prefers_explicit_extensions() {
    let patterns = vec!["**/*.md".to_string(), "**/*.markdown".to_string()];
    let configured = vec!["org".to_string(), "j2".to_string(), "toml".to_string()];
    let policy = resolve_wendao_incremental_policy(&patterns, Some(&configured));
    assert_eq!(
        policy.extensions(),
        &["j2".to_string(), "org".to_string(), "toml".to_string()]
    );
}

#[test]
fn resolve_wendao_incremental_policy_extracts_from_patterns_when_no_override() {
    let patterns = vec!["**/*.{md,org,template.md.j2}".to_string()];
    let policy = resolve_wendao_incremental_policy(&patterns, None);
    assert_eq!(
        policy.extensions(),
        &["j2".to_string(), "md".to_string(), "org".to_string()]
    );
}

#[test]
fn resolve_wendao_watch_patterns_prefers_configured_patterns() {
    let patterns = vec!["**/SKILL.md".to_string(), "docs/**/*.md".to_string()];
    let resolved = resolve_wendao_watch_patterns(Some(&patterns), None);
    assert_eq!(resolved, patterns);
}

#[test]
fn resolve_wendao_watch_patterns_derives_from_extensions_when_patterns_absent() {
    let extensions = vec!["org".to_string(), "j2".to_string(), "toml".to_string()];
    let resolved = resolve_wendao_watch_patterns(None, Some(&extensions));
    assert_eq!(
        resolved,
        vec![
            "**/*.org".to_string(),
            "**/*.j2".to_string(),
            "**/*.toml".to_string()
        ]
    );
}

#[test]
fn resolve_wendao_watch_patterns_falls_back_to_default_set() {
    let resolved = resolve_wendao_watch_patterns(None, None);
    assert_eq!(
        resolved,
        vec![
            "**/*.md".to_string(),
            "**/*.markdown".to_string(),
            "**/*.org".to_string(),
            "**/*.orgm".to_string(),
            "**/*.j2".to_string(),
            "**/*.toml".to_string()
        ]
    );
}

#[test]
fn resolve_wendao_watch_patterns_normalizes_extensions_and_drops_invalid_tokens() {
    let extensions = vec![
        " .ORG ".to_string(),
        "J2".to_string(),
        "bad^token".to_string(),
        String::new(),
    ];
    let resolved = resolve_wendao_watch_patterns(None, Some(&extensions));
    assert_eq!(
        resolved,
        vec!["**/*.org".to_string(), "**/*.j2".to_string()]
    );
}
