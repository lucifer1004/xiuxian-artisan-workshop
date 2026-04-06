use std::collections::{HashMap, HashSet};

use crate::link_graph::index::build::assemble::types::EdgeTables;
use crate::parsers::markdown::{ParsedNote, normalize_alias};

pub(crate) fn build_edge_tables(
    parsed_notes: &[ParsedNote],
    alias_to_doc_id: &HashMap<String, String>,
) -> EdgeTables {
    let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
    let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();
    let mut edge_count = 0usize;

    for parsed in parsed_notes {
        let from_id = &parsed.doc.id;

        for raw_target in &parsed.link_targets {
            let normalized = normalize_alias(raw_target);
            if normalized.is_empty() {
                continue;
            }
            let Some(to_id) = alias_to_doc_id.get(&normalized).cloned() else {
                continue;
            };
            if &to_id == from_id {
                continue;
            }
            let inserted = outgoing
                .entry(from_id.clone())
                .or_default()
                .insert(to_id.clone());
            if inserted {
                incoming.entry(to_id).or_default().insert(from_id.clone());
                edge_count += 1;
            }
        }

        for section in &parsed.sections {
            let pd_edges =
                crate::link_graph::index::build::property_drawer_edges::extract_property_drawer_edges(
                    from_id,
                    section,
                    alias_to_doc_id,
                );

            for edge in pd_edges {
                if edge.to == *from_id {
                    continue;
                }
                let inserted = outgoing
                    .entry(edge.from.clone())
                    .or_default()
                    .insert(edge.to.clone());
                if inserted {
                    incoming
                        .entry(edge.to)
                        .or_default()
                        .insert(edge.from.clone());
                    edge_count += 1;
                }
            }
        }
    }

    EdgeTables {
        outgoing,
        incoming,
        edge_count,
    }
}
