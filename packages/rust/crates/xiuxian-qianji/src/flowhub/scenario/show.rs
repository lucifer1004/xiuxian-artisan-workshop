use std::path::{Path, PathBuf};

use crate::error::QianjiError;
use crate::markdown::{MarkdownShowSection, render_show_surface};
use serde_json::json;
use xiuxian_qianhuan::EmbeddedManifestationTemplateCatalog;

use crate::flowhub::{
    derive_flowchart_aliases, load_flowhub_scenario_manifest, render_flowchart,
    resolve_flowhub_scenario_modules,
};

const SCENARIO_FLOWCHART_SECTION_TEMPLATE_NAME: &str = "flowhub_scenario_flowchart.md.j2";
const SCENARIO_FLOWCHART_SECTION_TEMPLATE_SOURCE: &str =
    include_str!("../../../resources/templates/control_plane/flowhub_scenario_flowchart.md.j2");
const SCENARIO_SURFACE_SECTION_TEMPLATE_NAME: &str = "flowhub_scenario_surface.md.j2";
const SCENARIO_SURFACE_SECTION_TEMPLATE_SOURCE: &str =
    include_str!("../../../resources/templates/control_plane/flowhub_scenario_surface.md.j2");
const SCENARIO_HIDDEN_ALIASES_TEMPLATE_NAME: &str = "flowhub_scenario_hidden_aliases.md.j2";
const SCENARIO_HIDDEN_ALIASES_TEMPLATE_SOURCE: &str = include_str!(
    "../../../resources/templates/control_plane/flowhub_scenario_hidden_aliases.md.j2"
);
const SCENARIO_LINKS_TEMPLATE_NAME: &str = "flowhub_scenario_links.md.j2";
const SCENARIO_LINKS_TEMPLATE_SOURCE: &str =
    include_str!("../../../resources/templates/control_plane/flowhub_scenario_links.md.j2");

static SCENARIO_TEMPLATE_CATALOG: EmbeddedManifestationTemplateCatalog =
    EmbeddedManifestationTemplateCatalog::new(
        "Flowhub scenario show template renderer",
        &[
            (
                SCENARIO_FLOWCHART_SECTION_TEMPLATE_NAME,
                SCENARIO_FLOWCHART_SECTION_TEMPLATE_SOURCE,
            ),
            (
                SCENARIO_SURFACE_SECTION_TEMPLATE_NAME,
                SCENARIO_SURFACE_SECTION_TEMPLATE_SOURCE,
            ),
            (
                SCENARIO_HIDDEN_ALIASES_TEMPLATE_NAME,
                SCENARIO_HIDDEN_ALIASES_TEMPLATE_SOURCE,
            ),
            (SCENARIO_LINKS_TEMPLATE_NAME, SCENARIO_LINKS_TEMPLATE_SOURCE),
        ],
    );

/// One visible surface preview derived from a scenario alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowhubScenarioSurfacePreview {
    /// Alias that will become a top-level bounded work-surface directory.
    pub alias: String,
    /// Resolved Flowhub module reference for this alias.
    pub module_ref: String,
    /// Conceptual target path inside the future work surface.
    pub target_path: PathBuf,
    /// Source node manifest inside Flowhub.
    pub source_manifest_path: PathBuf,
}

/// One hidden composite alias that participates in the scenario graph but does
/// not materialize into a top-level bounded surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowhubScenarioHiddenAlias {
    /// Alias declared by the scenario manifest.
    pub alias: String,
    /// Resolved Flowhub module reference.
    pub module_ref: String,
}

/// First-order preview of the bounded work surface implied by a scenario root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowhubScenarioShow {
    /// Stable scenario/plan name.
    pub plan_name: String,
    /// Scenario root directory.
    pub scenario_dir: PathBuf,
    /// Resolved Flowhub root used for module lookups.
    pub flowhub_root: PathBuf,
    /// Derived preview of the materialized root flowchart.
    pub flowchart_preview: String,
    /// Ordered visible leaf surfaces that will materialize.
    pub surfaces: Vec<FlowhubScenarioSurfacePreview>,
    /// Ordered composite aliases hidden behind the top-level bounded surface.
    pub hidden_aliases: Vec<FlowhubScenarioHiddenAlias>,
    /// Declared scenario links rendered as stable references.
    pub links: Vec<String>,
}

/// Build a first-order work-surface preview from a Flowhub scenario directory.
///
/// # Errors
///
/// Returns [`QianjiError::Topology`] when the scenario manifest cannot be
/// loaded or Flowhub modules cannot be resolved.
pub fn show_flowhub_scenario(
    flowhub_root: impl AsRef<Path>,
    scenario_dir: impl AsRef<Path>,
) -> Result<FlowhubScenarioShow, QianjiError> {
    let flowhub_root = flowhub_root.as_ref();
    let scenario_dir = scenario_dir.as_ref();
    let manifest_path = scenario_dir.join("qianji.toml");
    let manifest = load_flowhub_scenario_manifest(&manifest_path)?;
    let resolved_modules = resolve_flowhub_scenario_modules(flowhub_root, &manifest)?;

    let mut surfaces = Vec::new();
    let mut hidden_aliases = Vec::new();
    let mut visible_aliases = Vec::new();
    for module in &resolved_modules {
        if module.manifest.template.is_some() {
            hidden_aliases.push(FlowhubScenarioHiddenAlias {
                alias: module.alias.clone(),
                module_ref: module.module_ref.clone(),
            });
            continue;
        }

        visible_aliases.push(module.alias.clone());
        surfaces.push(FlowhubScenarioSurfacePreview {
            alias: module.alias.clone(),
            module_ref: module.module_ref.clone(),
            target_path: scenario_dir.join(&module.alias),
            source_manifest_path: module.manifest_path.clone(),
        });
    }

    if surfaces.is_empty() {
        return Err(QianjiError::Topology(format!(
            "Flowhub scenario `{}` does not expose any leaf nodes that can anchor a bounded work surface",
            manifest.planning.name
        )));
    }

    let flowchart_aliases = derive_flowchart_aliases(&manifest, &visible_aliases);
    let flowchart_preview = render_flowchart(&manifest, &visible_aliases, &flowchart_aliases);
    let links = manifest
        .template
        .link
        .iter()
        .map(|link| {
            format!(
                "{} -> {}",
                display_link_ref(&link.from),
                display_link_ref(&link.to)
            )
        })
        .collect::<Vec<_>>();

    Ok(FlowhubScenarioShow {
        plan_name: manifest.planning.name,
        scenario_dir: scenario_dir.to_path_buf(),
        flowhub_root: flowhub_root.to_path_buf(),
        flowchart_preview,
        surfaces,
        hidden_aliases,
        links,
    })
}

/// Render a scenario-derived work-surface preview into markdown.
#[must_use]
pub fn render_flowhub_scenario_show(show: &FlowhubScenarioShow) -> String {
    let mut sections = vec![MarkdownShowSection {
        title: "flowchart.mmd".into(),
        lines: render_scenario_flowchart_section_lines(show),
    }];

    for surface in &show.surfaces {
        sections.push(MarkdownShowSection {
            title: surface.alias.as_str().into(),
            lines: render_scenario_surface_section_lines(surface),
        });
    }

    if !show.hidden_aliases.is_empty() {
        sections.push(MarkdownShowSection {
            title: "Hidden Composite Aliases".into(),
            lines: render_scenario_hidden_aliases_section_lines(&show.hidden_aliases),
        });
    }

    if !show.links.is_empty() {
        sections.push(MarkdownShowSection {
            title: "Links".into(),
            lines: render_scenario_links_section_lines(&show.links),
        });
    }

    render_show_surface(
        "Scenario Work Surface Preview",
        &[
            format!("Scenario: {}", show.plan_name),
            format!("Location: {}", show.scenario_dir.display()),
            format!("Flowhub: {}", show.flowhub_root.display()),
        ],
        &sections,
    )
}

fn display_link_ref(reference: &crate::contracts::TemplateLinkRef) -> String {
    match (&reference.alias, &reference.symbol) {
        (Some(alias), symbol) => format!("{alias}::{symbol}"),
        (None, symbol) => symbol.clone(),
    }
}

fn render_scenario_flowchart_section_lines(show: &FlowhubScenarioShow) -> Vec<String> {
    render_embedded_scenario_block(
        SCENARIO_FLOWCHART_SECTION_TEMPLATE_NAME,
        json!({
            "flowchart_preview": show.flowchart_preview.trim_end(),
        }),
    )
    .unwrap_or_else(|error| {
        log::warn!(
            "failed to render Flowhub scenario flowchart preview through qianhuan; falling back to inline format: {error}"
        );
        vec![
            "Status: preview".to_string(),
            "Preview:".to_string(),
            "```mermaid".to_string(),
            show.flowchart_preview.trim_end().to_string(),
            "```".to_string(),
        ]
    })
}

fn render_scenario_surface_section_lines(surface: &FlowhubScenarioSurfacePreview) -> Vec<String> {
    render_embedded_scenario_block(
        SCENARIO_SURFACE_SECTION_TEMPLATE_NAME,
        json!({
            "module_ref": surface.module_ref,
            "target_path": surface.target_path.display().to_string(),
            "source_manifest_path": surface.source_manifest_path.display().to_string(),
        }),
    )
    .unwrap_or_else(|error| {
        log::warn!(
            "failed to render Flowhub scenario surface preview through qianhuan; falling back to inline format: {error}"
        );
        vec![
            format!("Module: {}", surface.module_ref),
            format!("Target Path: {}", surface.target_path.display()),
            format!("Source Manifest: {}", surface.source_manifest_path.display()),
        ]
    })
}

fn render_scenario_hidden_aliases_section_lines(
    hidden_aliases: &[FlowhubScenarioHiddenAlias],
) -> Vec<String> {
    render_embedded_scenario_block(
        SCENARIO_HIDDEN_ALIASES_TEMPLATE_NAME,
        json!({
            "aliases_block": hidden_aliases
                .iter()
                .map(|hidden| format!("- {} -> {}", hidden.alias, hidden.module_ref))
                .collect::<Vec<_>>()
                .join("\n"),
        }),
    )
    .unwrap_or_else(|error| {
        log::warn!(
            "failed to render Flowhub scenario hidden aliases through qianhuan; falling back to inline format: {error}"
        );
        hidden_aliases
            .iter()
            .map(|hidden| format!("- {} -> {}", hidden.alias, hidden.module_ref))
            .collect()
    })
}

fn render_scenario_links_section_lines(links: &[String]) -> Vec<String> {
    render_embedded_scenario_block(
        SCENARIO_LINKS_TEMPLATE_NAME,
        json!({
            "links_block": links
                .iter()
                .map(|link| format!("- {link}"))
                .collect::<Vec<_>>()
                .join("\n"),
        }),
    )
    .unwrap_or_else(|error| {
        log::warn!(
            "failed to render Flowhub scenario links through qianhuan; falling back to inline format: {error}"
        );
        links.iter().map(|link| format!("- {link}")).collect()
    })
}

fn render_embedded_scenario_block(
    template_name: &str,
    payload: serde_json::Value,
) -> Result<Vec<String>, String> {
    SCENARIO_TEMPLATE_CATALOG.render_lines(template_name, payload)
}

#[cfg(test)]
#[path = "../../../tests/unit/flowhub/scenario_show.rs"]
mod tests;
