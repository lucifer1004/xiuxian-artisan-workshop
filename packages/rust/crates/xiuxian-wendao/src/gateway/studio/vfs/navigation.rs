use crate::gateway::studio::pathing::studio_display_path;
use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::types::StudioNavigationTarget;

pub(crate) fn resolve_navigation_target(state: &StudioState, path: &str) -> StudioNavigationTarget {
    let normalized = studio_display_path(state, path);
    let project_name = state
        .configured_projects()
        .into_iter()
        .find(|project| {
            normalized == project.name
                || normalized.starts_with(format!("{}/", project.name).as_str())
        })
        .map(|project| project.name);

    StudioNavigationTarget {
        path: normalized,
        category: "file".to_string(),
        project_name,
        root_label: None,
        line: None,
        line_end: None,
        column: None,
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gateway/studio/vfs/navigation.rs"]
mod tests;
