use crate::contracts::{WorkdirCheck, WorkdirManifest, WorkdirPlan};
use crate::error::QianjiError;

pub(super) fn render_root_manifest(
    plan_name: &str,
    visible_aliases: &[String],
    flowchart_aliases: &[String],
) -> Result<String, QianjiError> {
    let mut surface = Vec::with_capacity(visible_aliases.len() + 1);
    surface.push("flowchart.mmd".to_string());
    surface.extend(visible_aliases.iter().cloned());

    let mut require = Vec::with_capacity(visible_aliases.len() * 2 + 1);
    require.push("flowchart.mmd".to_string());
    for alias in visible_aliases {
        require.push(alias.clone());
    }
    for alias in visible_aliases {
        require.push(format!("{alias}/**/*.md"));
    }

    let manifest = WorkdirManifest {
        version: 1,
        plan: WorkdirPlan {
            name: plan_name.to_string(),
            surface,
        },
        check: WorkdirCheck {
            require,
            flowchart: flowchart_aliases.to_vec(),
        },
    };

    toml::to_string_pretty(&manifest).map_err(|error| {
        QianjiError::Topology(format!(
            "Failed to serialize bounded work-surface manifest for `{plan_name}`: {error}"
        ))
    })
}
