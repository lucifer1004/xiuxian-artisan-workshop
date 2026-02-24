use super::LinkGraphHit;
use std::fmt::Write;

/// Generates a GRAG-compliant hierarchical narrative from search hits.
///
/// This function transforms a list of graph hits into a structured "Hard Prompt"
/// that explicitly narrates the topological connections and atomic claims.
///
/// Ref: GRAG (2025) "Narrative Topology"
#[must_use]
pub fn narrate_subgraph(hits: &[LinkGraphHit]) -> String {
    if hits.is_empty() {
        return String::new();
    }

    let mut buffer = String::with_capacity(hits.len() * 256);
    let _ = writeln!(&mut buffer, "### Hierarchical Subgraph Narrative (GRAG-v1)");

    for hit in hits {
        let title = if hit.title.is_empty() {
            &hit.stem
        } else {
            &hit.title
        };

        let _ = writeln!(&mut buffer, "\n[Concept: {title}]");
        let _ = writeln!(&mut buffer, "  Path: {}", hit.path);
        let _ = writeln!(&mut buffer, "  Score: {:.4}", hit.score);

        if let Some(best_section) = hit.best_section.as_deref()
            && !best_section.is_empty()
        {
            let _ = writeln!(&mut buffer, "  Best Section: {best_section}");
        }

        if let Some(reason) = hit.match_reason.as_deref()
            && !reason.is_empty()
        {
            let _ = writeln!(&mut buffer, "  Match Reason: {reason}");
        }
    }

    let _ = writeln!(&mut buffer, "\n---");
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_narrate_subgraph_empty() {
        assert_eq!(narrate_subgraph(&[]), "");
    }

    #[test]
    fn test_narrate_single_hit() {
        let hit = LinkGraphHit {
            stem: "node_a".to_string(),
            title: "Node A".to_string(),
            path: "a.md".to_string(),
            score: 1.0,
            best_section: None,
            match_reason: Some("graph_rank>fts".to_string()),
        };
        let output = narrate_subgraph(&[hit]);
        assert!(output.contains("[Concept: Node A]"));
        assert!(output.contains("  Path: a.md"));
        assert!(output.contains("  Score: 1.0000"));
        assert!(output.contains("  Match Reason: graph_rank>fts"));
    }
}
