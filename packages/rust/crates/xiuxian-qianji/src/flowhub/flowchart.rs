use std::collections::BTreeSet;

use crate::contracts::FlowhubScenarioManifest;

pub(crate) fn derive_flowchart_aliases(
    manifest: &FlowhubScenarioManifest,
    visible_aliases: &[String],
) -> Vec<String> {
    let visible = visible_aliases
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut aliases = Vec::new();

    for link in &manifest.template.link {
        let Some(from_alias) = link.from.alias.as_deref() else {
            continue;
        };
        let Some(to_alias) = link.to.alias.as_deref() else {
            continue;
        };
        if !visible.contains(from_alias) || !visible.contains(to_alias) {
            continue;
        }
        push_unique(&mut aliases, from_alias.to_string());
        push_unique(&mut aliases, to_alias.to_string());
    }

    if aliases.is_empty() {
        aliases.extend(visible_aliases.iter().cloned());
    }

    aliases
}

pub(crate) fn render_flowchart(
    manifest: &FlowhubScenarioManifest,
    visible_aliases: &[String],
    flowchart_aliases: &[String],
) -> String {
    let visible = visible_aliases
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut lines = vec!["flowchart LR".to_string()];

    for alias in flowchart_aliases {
        lines.push(format!("  {alias}[\"{alias}\"]"));
    }

    let mut edges = BTreeSet::new();
    for link in &manifest.template.link {
        let Some(from_alias) = link.from.alias.as_deref() else {
            continue;
        };
        let Some(to_alias) = link.to.alias.as_deref() else {
            continue;
        };
        if !visible.contains(from_alias) || !visible.contains(to_alias) {
            continue;
        }
        if edges.insert((from_alias.to_string(), to_alias.to_string())) {
            lines.push(format!("  {from_alias} --> {to_alias}"));
        }
    }

    lines.join("\n") + "\n"
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}
