use std::fs;
use std::path::{Path, PathBuf};

use globset::Glob;
use regex::Regex;
use walkdir::WalkDir;

use crate::error::QianjiError;
use crate::markdown::{
    MarkdownDiagnostic, render_follow_up_query_section, render_validation_failed,
    render_validation_pass,
};

use super::load::load_workdir_manifest;
use super::query::build_workdir_check_follow_up_query;

/// One bounded markdown retrieval surface supported by the compact workdir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkdirMarkdownSurface {
    /// The `blueprint/` markdown surface.
    Blueprint,
    /// The `plan/` markdown surface.
    Plan,
}

impl WorkdirMarkdownSurface {
    /// Return the stable SQL-visible surface name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Blueprint => "blueprint",
            Self::Plan => "plan",
        }
    }

    fn from_top_level_name(surface: &str) -> Option<Self> {
        match surface {
            "blueprint" => Some(Self::Blueprint),
            "plan" => Some(Self::Plan),
            _ => None,
        }
    }
}

/// One user-facing validation diagnostic for a bounded work surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkdirDiagnostic {
    /// Short diagnostic title.
    pub title: String,
    /// On-disk location of the failing surface.
    pub location: PathBuf,
    /// Concrete failing condition.
    pub problem: String,
    /// Why the issue blocks continued bounded work.
    pub why_it_blocks: String,
    /// Concrete next action for repairing the surface.
    pub fix: String,
    /// Bounded markdown surfaces that should be queried during repair follow-up.
    pub follow_up_surfaces: Vec<WorkdirMarkdownSurface>,
}

/// Structural validation result for one bounded work surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkdirCheckReport {
    /// Stable plan name from the root manifest.
    pub plan_name: String,
    /// Checked bounded workdir root.
    pub workdir: PathBuf,
    /// Collected blocking diagnostics.
    pub diagnostics: Vec<WorkdirDiagnostic>,
}

impl WorkdirCheckReport {
    /// Returns `true` when no blocking diagnostics were emitted.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Validate the bounded work-surface contract on disk.
///
/// # Errors
///
/// Returns [`QianjiError::Topology`] when the root manifest cannot be loaded,
/// the filesystem cannot be inspected, or the flowchart companion cannot be
/// read.
pub fn check_workdir(workdir: impl AsRef<Path>) -> Result<WorkdirCheckReport, QianjiError> {
    let workdir = workdir.as_ref();
    let manifest = load_workdir_manifest(workdir.join("qianji.toml"))?;
    let mut diagnostics = Vec::new();

    for requirement in &manifest.check.require {
        if is_glob_pattern(requirement) {
            let matches = count_glob_matches(workdir, requirement)?;
            if matches == 0 {
                diagnostics.push(WorkdirDiagnostic {
                    title: "Missing required glob matches".to_string(),
                    location: workdir.to_path_buf(),
                    problem: format!(
                        "bounded work-surface contract requires at least one match for `{requirement}`, but none were found"
                    ),
                    why_it_blocks: "the bounded surface is structurally incomplete".to_string(),
                    fix: format!(
                        "create at least one file matching `{requirement}` or relax `check.require`"
                    ),
                    follow_up_surfaces: follow_up_surfaces_for_requirement(requirement),
                });
            }
        } else if !workdir.join(requirement).exists() {
            diagnostics.push(WorkdirDiagnostic {
                title: "Missing required path".to_string(),
                location: workdir.join(requirement),
                problem: format!(
                    "bounded work-surface contract requires `{requirement}`, but the path is absent"
                ),
                why_it_blocks: "Codex cannot rely on the declared bounded surface".to_string(),
                fix: format!("create `{requirement}` or relax `check.require`"),
                follow_up_surfaces: follow_up_surfaces_for_requirement(requirement),
            });
        }
    }

    let flowchart_path = workdir.join("flowchart.mmd");
    if flowchart_path.is_file() {
        let flowchart = fs::read_to_string(&flowchart_path).map_err(|error| {
            QianjiError::Topology(format!(
                "Failed to read bounded work-surface flowchart `{}`: {error}",
                flowchart_path.display()
            ))
        })?;

        for surface in &manifest.check.flowchart {
            if !flowchart_contains_surface(&flowchart, surface)? {
                diagnostics.push(WorkdirDiagnostic {
                    title: "Missing flowchart surface".to_string(),
                    location: flowchart_path.clone(),
                    problem: format!(
                        "`flowchart.mmd` does not visibly contain the principal surface `{surface}`"
                    ),
                    why_it_blocks:
                        "the graph companion no longer aligns with the bounded work surface"
                            .to_string(),
                    fix: format!("add a visible `{surface}` node or label to `flowchart.mmd`"),
                    follow_up_surfaces: follow_up_surfaces_for_flowchart(&manifest.check.flowchart),
                });
            }
        }

        for pair in manifest.check.flowchart.windows(2) {
            let from = &pair[0];
            let to = &pair[1];
            if !flowchart_contains_backbone(&flowchart, from, to)? {
                diagnostics.push(WorkdirDiagnostic {
                    title: "Missing flowchart backbone".to_string(),
                    location: flowchart_path.clone(),
                    problem: format!(
                        "`flowchart.mmd` does not visibly express the backbone `{from} --> {to}`"
                    ),
                    why_it_blocks:
                        "Codex cannot trust the visible backbone direction of the bounded work"
                            .to_string(),
                    fix: format!("add a visible `{from} --> {to}` relation to `flowchart.mmd`"),
                    follow_up_surfaces: follow_up_surfaces_for_flowchart(&manifest.check.flowchart),
                });
            }
        }
    } else {
        diagnostics.push(WorkdirDiagnostic {
            title: "Missing flowchart companion".to_string(),
            location: flowchart_path,
            problem:
                "`flowchart.mmd` is required for flowchart alignment checks, but the file is absent"
                    .to_string(),
            why_it_blocks: "the bounded work surface has no direct graph companion".to_string(),
            fix: "create `flowchart.mmd` at the work-surface root".to_string(),
            follow_up_surfaces: follow_up_surfaces_for_flowchart(&manifest.check.flowchart),
        });
    }

    Ok(WorkdirCheckReport {
        plan_name: manifest.plan.name,
        workdir: workdir.to_path_buf(),
        diagnostics,
    })
}

/// Render a bounded work-surface validation report into markdown diagnostics.
#[must_use]
pub fn render_workdir_check_markdown(report: &WorkdirCheckReport) -> String {
    if report.is_valid() {
        return render_validation_pass(&[
            format!("Plan: {}", report.plan_name),
            format!("Location: {}", report.workdir.display()),
        ]);
    }

    let diagnostics = report
        .diagnostics
        .iter()
        .map(|diagnostic| MarkdownDiagnostic {
            title: diagnostic.title.as_str(),
            location: diagnostic.location.display().to_string().into(),
            problem: diagnostic.problem.as_str(),
            why_it_blocks: diagnostic.why_it_blocks.as_str(),
            fix: diagnostic.fix.as_str(),
        })
        .collect::<Vec<_>>();

    let mut rendered = render_validation_failed(&[], &diagnostics);
    if let Some(follow_up_query) = build_workdir_check_follow_up_query(report) {
        let surface_names = follow_up_query
            .surfaces
            .iter()
            .map(|surface| surface.as_str())
            .collect::<Vec<_>>()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        rendered.push_str("\n\n");
        rendered.push_str(&render_follow_up_query_section(
            &surface_names,
            &follow_up_query.query_text,
        ));
    }

    rendered
}

fn count_glob_matches(workdir: &Path, pattern: &str) -> Result<usize, QianjiError> {
    let matcher = Glob::new(pattern)
        .map_err(|error| {
            QianjiError::Topology(format!(
                "invalid `check.require` glob pattern `{pattern}`: {error}"
            ))
        })?
        .compile_matcher();

    let mut match_count = 0_usize;
    for entry in WalkDir::new(workdir) {
        let entry = entry.map_err(|error| {
            QianjiError::Topology(format!(
                "Failed to walk bounded work surface `{}`: {error}",
                workdir.display()
            ))
        })?;
        if entry.path() == workdir {
            continue;
        }
        let relative = entry.path().strip_prefix(workdir).map_err(|error| {
            QianjiError::Topology(format!(
                "Failed to relativize bounded work-surface path `{}` against `{}`: {error}",
                entry.path().display(),
                workdir.display()
            ))
        })?;
        let normalized = relative.to_string_lossy().replace('\\', "/");
        if matcher.is_match(normalized.as_str()) {
            match_count += 1;
        }
    }

    Ok(match_count)
}

fn flowchart_contains_surface(flowchart: &str, surface: &str) -> Result<bool, QianjiError> {
    let regex = surface_regex(surface)?;
    Ok(regex.is_match(flowchart))
}

fn flowchart_contains_backbone(flowchart: &str, from: &str, to: &str) -> Result<bool, QianjiError> {
    let from_regex = surface_regex(from)?;
    let to_regex = surface_regex(to)?;

    for line in flowchart.lines().filter(|line| line.contains("-->")) {
        let Some(arrow_index) = line.find("-->") else {
            continue;
        };
        let from_match = from_regex
            .find_iter(line)
            .find(|capture| capture.start() < arrow_index);
        let to_match = to_regex
            .find_iter(line)
            .find(|capture| capture.start() > arrow_index);
        if from_match.is_some() && to_match.is_some() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn surface_regex(surface: &str) -> Result<Regex, QianjiError> {
    Regex::new(&format!(
        r"(^|[^A-Za-z0-9_-]){}([^A-Za-z0-9_-]|$)",
        regex::escape(surface)
    ))
    .map_err(|error| {
        QianjiError::Topology(format!(
            "failed to build flowchart surface matcher for `{surface}`: {error}"
        ))
    })
}

fn is_glob_pattern(value: &str) -> bool {
    value
        .chars()
        .any(|character| matches!(character, '*' | '?' | '[' | ']'))
}

fn follow_up_surfaces_for_requirement(requirement: &str) -> Vec<WorkdirMarkdownSurface> {
    let mut surfaces = Vec::new();
    if requirement.starts_with("blueprint") {
        surfaces.push(WorkdirMarkdownSurface::Blueprint);
    }
    if requirement.starts_with("plan") {
        surfaces.push(WorkdirMarkdownSurface::Plan);
    }
    surfaces
}

fn follow_up_surfaces_for_flowchart(entries: &[String]) -> Vec<WorkdirMarkdownSurface> {
    let mut surfaces = entries
        .iter()
        .filter_map(|entry| WorkdirMarkdownSurface::from_top_level_name(entry))
        .collect::<Vec<_>>();
    if surfaces.is_empty() {
        surfaces.push(WorkdirMarkdownSurface::Blueprint);
        surfaces.push(WorkdirMarkdownSurface::Plan);
    }
    surfaces
}
