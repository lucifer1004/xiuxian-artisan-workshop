use std::sync::Arc;

use crate::agent::bootstrap::service_mount::{ServiceMountCatalog, ServiceMountMeta};
use crate::agent::native_tools::NativeAliasTool;
use crate::agent::native_tools::macros::register_native_tools;
use crate::agent::native_tools::registry::NativeToolRegistry;
use crate::agent::native_tools::spider::SpiderCrawlTool;
use crate::agent::native_tools::zhixing::{AgendaViewTool, JournalRecordTool, TaskAddTool};
use xiuxian_skills::{InternalSkillNativeAliasSpec, InternalSkillWorkflowType};
use xiuxian_wendao::skill_vfs::internal_authority::AuthorizedInternalSkillNativeAliasScan;
use xiuxian_wendao::{SkillVfsResolver, SpiderWendaoBridge};
use xiuxian_zhixing::ZhixingHeyi;

pub(in crate::agent::bootstrap) fn mount_native_tool_cauldron(
    heyi: Option<&Arc<ZhixingHeyi>>,
    skill_vfs_resolver: Option<&Arc<SkillVfsResolver>>,
    native_tools: &mut NativeToolRegistry,
    mounts: &mut ServiceMountCatalog,
) {
    if let Some(heyi) = heyi {
        register_zhixing_native_tools(heyi, native_tools, mounts);
    } else {
        mounts.skipped(
            "zhixing.native_tools",
            "tooling",
            ServiceMountMeta::default().detail("heyi_unavailable"),
        );
    }

    mount_spider_tool(heyi, native_tools, mounts);
    mount_internal_skill_aliases(skill_vfs_resolver, native_tools, mounts);
}

pub(in crate::agent::bootstrap) fn register_zhixing_native_tools(
    heyi: &Arc<ZhixingHeyi>,
    native_tools: &mut NativeToolRegistry,
    mounts: &mut ServiceMountCatalog,
) {
    register_native_tools!(
        native_tools,
        JournalRecordTool {
            heyi: Arc::clone(heyi),
        },
        TaskAddTool {
            heyi: Arc::clone(heyi),
        },
        AgendaViewTool {
            heyi: Arc::clone(heyi),
        }
    );
    mounts.mounted(
        "zhixing.native_tools",
        "tooling",
        ServiceMountMeta::default().detail("tools=journal.record,task.add,agenda.view"),
    );
}

type InternalAliasSpec = InternalSkillNativeAliasSpec<InternalSkillWorkflowType>;

fn mount_spider_tool(
    heyi: Option<&Arc<ZhixingHeyi>>,
    native_tools: &mut NativeToolRegistry,
    mounts: &mut ServiceMountCatalog,
) {
    let ingress = heyi.map(|heyi| {
        Arc::new(SpiderWendaoBridge::for_knowledge_graph(
            heyi.graph.as_ref().clone(),
        ))
    });
    native_tools.register(Arc::new(SpiderCrawlTool {
        ingress: ingress.clone(),
    }));

    let detail = if ingress.is_some() {
        "ingress=enabled"
    } else {
        "ingress=disabled"
    };
    mounts.mounted(
        "native.web_crawl",
        "tooling",
        ServiceMountMeta::default().detail(detail),
    );
}

fn mount_internal_skill_aliases(
    skill_vfs_resolver: Option<&Arc<SkillVfsResolver>>,
    native_tools: &mut NativeToolRegistry,
    mounts: &mut ServiceMountCatalog,
) {
    let Some(resolver) = skill_vfs_resolver else {
        mounts.skipped(
            "native.internal_skill_aliases",
            "tooling",
            ServiceMountMeta::default().detail("skill_vfs_unavailable"),
        );
        return;
    };

    let Some(root) = resolver.internal_roots().first() else {
        mounts.skipped(
            "native.internal_skill_aliases",
            "tooling",
            ServiceMountMeta::default().detail("no_internal_roots"),
        );
        return;
    };

    let scan = match resolver.scan_authorized_internal_native_aliases(root.as_path()) {
        Ok(scan) => scan,
        Err(error) => {
            mounts.failed(
                "native.internal_skill_aliases",
                "tooling",
                ServiceMountMeta::default().detail(format!("scan_failed: {error}")),
            );
            return;
        }
    };

    let AuthorizedInternalSkillNativeAliasScan {
        report,
        compiled_specs,
    } = scan;
    let compiled_count = compiled_specs.len();
    if report.is_critically_failed() {
        mounts.failed(
            "native.internal_skill_aliases",
            "tooling",
            ServiceMountMeta::default()
                .detail(format!("ghost_links_detected: {}", report.ghost_count())),
        );
        return;
    }

    if report.unauthorized_count() > 0 {
        tracing::warn!(
            event = "agent.bootstrap.internal_alias.unauthorized",
            unauthorized = report.unauthorized_count(),
            "unauthorized internal skill manifests detected; skipping those manifests"
        );
    }

    let (mounted_names, alias_issues) = register_internal_aliases(compiled_specs, native_tools);
    let issue_count = report.issues.len() + alias_issues.len();
    if issue_count > 0 {
        tracing::warn!(
            event = "agent.bootstrap.internal_alias.issues",
            issues = issue_count,
            "internal skill alias mounting reported issues"
        );
    }

    let detail = format!(
        "authorized={},compiled={},mounted={},ghosts={},unauthorized={},issues={}",
        report.authorized_count(),
        compiled_count,
        mounted_names.len(),
        report.ghost_count(),
        report.unauthorized_count(),
        issue_count,
    );
    mounts.mounted(
        "native.internal_skill_aliases",
        "tooling",
        ServiceMountMeta::default().detail(detail),
    );
}

fn register_internal_aliases(
    compiled_specs: Vec<InternalAliasSpec>,
    native_tools: &mut NativeToolRegistry,
) -> (Vec<String>, Vec<String>) {
    let mut mounted = Vec::new();
    let mut issues = Vec::new();

    for spec in compiled_specs {
        let alias_name = spec.tool_name.trim().to_string();
        if alias_name.is_empty() {
            issues.push("internal alias missing tool_name".to_string());
            continue;
        }

        if native_tools.get(alias_name.as_str()).is_some() {
            issues.push(format!("alias name already registered: {alias_name}"));
            continue;
        }

        let Some(target_tool) = native_tools.get(spec.target_tool_name.as_str()) else {
            issues.push(format!(
                "alias target not registered: {} -> {}",
                alias_name, spec.target_tool_name
            ));
            continue;
        };

        if target_tool.name() == alias_name {
            issues.push(format!("alias target matches alias name: {alias_name}"));
            continue;
        }

        let tool = NativeAliasTool::new(
            alias_name.clone(),
            spec.description.clone(),
            target_tool.parameters(),
            target_tool,
        );
        native_tools.register(Arc::new(tool));
        mounted.push(alias_name);
    }

    (mounted, issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use tempfile::TempDir;
    use xiuxian_qianhuan::MockManifestation;
    use xiuxian_wendao::graph::KnowledgeGraph;
    use xiuxian_zhixing::storage::MarkdownStorage;

    const AGENDA_ADD_MANIFEST: &str = r#"id = \"xiuxian.zhixing.agenda.add\"

[qianhuan]
background = \"$wendao://skills-internal/agenda/SKILL.md\"

[tool_contract]
category = \"write\"
name = \"agenda.add\"
description = \"\"\"
Draft and add a new vow.

Args:
    - title: str - The title of the task.

Examples:
    Input:
        { \"title\": \"Ship loader\" }
    Output:
        \"done\"
\"\"\"

[workflow]
type = \"qianji_flow\"
internal_id = \"xiuxian.native.zhixing.add\"
flow_definition = \"$wendao://skills-internal/agenda/references/add/qianji.toml\"
"#;

    const AGENDA_VIEW_MANIFEST: &str = r#"id = \"xiuxian.zhixing.agenda.view\"

[qianhuan]
background = \"$wendao://skills-internal/agenda/SKILL.md\"

[tool_contract]
category = \"read\"
name = \"agenda.view\"
description = \"\"\"
Review agenda entries.

Args:
    - limit: int - Maximum number of items to show.

Examples:
    Input:
        { \"limit\": 5 }
    Output:
        \"ok\"
\"\"\"

[workflow]
type = \"native_dispatch\"
internal_id = \"xiuxian.native.zhixing.view\"
"#;

    fn build_heyi() -> anyhow::Result<(Arc<ZhixingHeyi>, TempDir)> {
        let graph = Arc::new(KnowledgeGraph::new());
        let tmp = TempDir::new()?;
        let storage = Arc::new(MarkdownStorage::new(tmp.path().to_path_buf()));
        let manifestation = Arc::new(MockManifestation);
        let heyi = ZhixingHeyi::new(
            graph,
            manifestation,
            storage,
            "internal-alias-test".to_string(),
            "UTC",
        )?;
        Ok((Arc::new(heyi), tmp))
    }

    fn write_fixture_tree(root: &std::path::Path, include_ghost: bool, include_unauthorized: bool) {
        let skill_root = root.join("internal_skills").join("agenda");
        let references_root = skill_root.join("references");
        let add_root = references_root.join("add");
        let view_root = references_root.join("view");
        fs::create_dir_all(&add_root).unwrap();
        fs::write(add_root.join("qianji.toml"), AGENDA_ADD_MANIFEST).unwrap();

        if include_unauthorized {
            fs::create_dir_all(&view_root).unwrap();
            fs::write(view_root.join("qianji.toml"), AGENDA_VIEW_MANIFEST).unwrap();
        }

        let mut lines = vec![
            "# Agenda",
            "",
            "- [Authorized add](references/add/qianji.toml)",
        ];
        if include_ghost {
            lines.push("- [Ghost flow](references/missing/qianji.toml)");
        }
        fs::create_dir_all(&skill_root).unwrap();
        fs::write(skill_root.join("SKILL.md"), lines.join("\n")).unwrap();
    }

    fn build_resolver(root: &std::path::Path) -> Arc<SkillVfsResolver> {
        let internal_root = root.join("internal_skills");
        Arc::new(
            SkillVfsResolver::from_roots_with_embedded_and_internal(&[], &[internal_root])
                .expect("resolver builds"),
        )
    }

    #[test]
    fn blocks_alias_mount_on_ghost_links() {
        let tmp = TempDir::new().unwrap();
        write_fixture_tree(tmp.path(), true, false);
        let resolver = build_resolver(tmp.path());
        let (heyi, _storage) = build_heyi().unwrap();

        let mut registry = NativeToolRegistry::new();
        let mut mounts = ServiceMountCatalog::new();
        mount_native_tool_cauldron(Some(&heyi), Some(&resolver), &mut registry, &mut mounts);

        assert!(registry.get("agenda.add").is_none());
        let records = mounts.finish();
        let alias_record = records
            .iter()
            .find(|record| record.service == "native.internal_skill_aliases")
            .expect("alias mount record should exist");
        assert_eq!(
            alias_record.status,
            crate::agent::bootstrap::service_mount::ServiceMountStatus::Failed
        );
    }

    #[test]
    fn mounts_authorized_aliases_and_skips_unauthorized() {
        let tmp = TempDir::new().unwrap();
        write_fixture_tree(tmp.path(), false, true);
        let resolver = build_resolver(tmp.path());
        let (heyi, _storage) = build_heyi().unwrap();

        let mut registry = NativeToolRegistry::new();
        let mut mounts = ServiceMountCatalog::new();
        mount_native_tool_cauldron(Some(&heyi), Some(&resolver), &mut registry, &mut mounts);

        assert!(registry.get("agenda.add").is_some());
        assert!(registry.get("agenda.view").is_none());

        let alias_tool = registry.get("agenda.add").expect("alias tool mounted");
        assert!(alias_tool.description().contains("Draft and add a new vow"));

        let records = mounts.finish();
        let alias_record = records
            .iter()
            .find(|record| record.service == "native.internal_skill_aliases")
            .expect("alias mount record should exist");
        assert_eq!(
            alias_record.status,
            crate::agent::bootstrap::service_mount::ServiceMountStatus::Mounted
        );
        assert!(
            alias_record
                .detail
                .as_deref()
                .unwrap_or_default()
                .contains("unauthorized=1")
        );
    }
}
