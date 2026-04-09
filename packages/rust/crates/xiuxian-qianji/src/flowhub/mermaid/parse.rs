use std::collections::BTreeMap;
use std::fmt;

use merman_core::{Engine, ParseOptions, RenderSemanticModel};
use regex::Regex;

use super::model::{MermaidEdge, MermaidFlowchart, MermaidNode, MermaidNodeKind};

/// One Mermaid parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MermaidParseError {
    message: String,
}

impl MermaidParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for MermaidParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MermaidParseError {}

pub(crate) fn parse_mermaid_flowchart(
    source: &str,
    merimind_graph_name: &str,
    registered_module_names: &[String],
) -> Result<MermaidFlowchart, MermaidParseError> {
    validate_explicit_node_labels(source)?;

    let parsed = Engine::new()
        .parse_diagram_for_render_model_sync(source, ParseOptions::strict())
        .map_err(|error| MermaidParseError::new(error.to_string()))?
        .ok_or_else(|| MermaidParseError::new("mermaid flowchart is empty"))?;

    let RenderSemanticModel::Flowchart(flowchart) = parsed.model else {
        return Err(MermaidParseError::new(format!(
            "expected a Mermaid flowchart, but parsed `{}`",
            parsed.meta.diagram_type
        )));
    };

    let nodes = flowchart
        .nodes
        .into_iter()
        .map(|node| {
            let label = node
                .label
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| node.id.clone());
            let kind = if registered_module_names
                .iter()
                .any(|module| module == &label)
            {
                MermaidNodeKind::Module
            } else {
                MermaidNodeKind::Scenario
            };

            MermaidNode {
                id: node.id,
                label,
                kind,
            }
        })
        .collect();

    let edges = flowchart
        .edges
        .into_iter()
        .map(|edge| MermaidEdge {
            from: edge.from,
            to: edge.to,
        })
        .collect();

    Ok(MermaidFlowchart {
        merimind_graph_name: merimind_graph_name.to_string(),
        direction: flowchart.direction.unwrap_or_else(|| "LR".to_string()),
        nodes,
        edges,
    })
}

fn validate_explicit_node_labels(source: &str) -> Result<(), MermaidParseError> {
    let explicit_node_pattern = Regex::new(
        r#"(?m)\b(?P<id>[A-Za-z][A-Za-z0-9_]*)\[(?:"(?P<quoted>[^"\n]+)"|(?P<plain>[^\]\n]+))\]"#,
    )
    .map_err(|error| MermaidParseError::new(error.to_string()))?;
    let mut labels_by_id = BTreeMap::<String, String>::new();

    for captures in explicit_node_pattern.captures_iter(source) {
        let node_id = captures["id"].trim();
        let label = captures
            .name("quoted")
            .or_else(|| captures.name("plain"))
            .map_or("", |value| value.as_str().trim());
        if label.is_empty() {
            continue;
        }

        match labels_by_id.get(node_id) {
            Some(previous_label) if previous_label != label => {
                return Err(MermaidParseError::new(format!(
                    "conflicting labels for Mermaid node `{node_id}`: `{previous_label}` vs `{label}`"
                )));
            }
            Some(_) => {}
            None => {
                labels_by_id.insert(node_id.to_string(), label.to_string());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/unit/flowhub/mermaid/parse.rs"]
mod tests;
