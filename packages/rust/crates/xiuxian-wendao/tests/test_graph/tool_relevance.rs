#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
use super::*;

#[test]
fn test_query_tool_relevance_finds_tools_by_keyword() {
    let graph = KnowledgeGraph::new();

    let docs = vec![
        SkillDoc {
            id: "git".to_string(),
            doc_type: "skill".to_string(),
            skill_name: "git".to_string(),
            tool_name: String::new(),
            content: "Git operations".to_string(),
            routing_keywords: vec![],
        },
        SkillDoc {
            id: "git.commit".to_string(),
            doc_type: "command".to_string(),
            skill_name: "git".to_string(),
            tool_name: "git.commit".to_string(),
            content: "Create a commit".to_string(),
            routing_keywords: vec!["commit".to_string(), "git".to_string()],
        },
        SkillDoc {
            id: "git.status".to_string(),
            doc_type: "command".to_string(),
            skill_name: "git".to_string(),
            tool_name: "git.status".to_string(),
            content: "Show status".to_string(),
            routing_keywords: vec!["status".to_string(), "git".to_string()],
        },
    ];
    graph.register_skill_entities(&docs).unwrap();

    let results = graph.query_tool_relevance(&["commit".to_string()], 2, 10);

    let tool_names: Vec<&str> = results.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        tool_names.contains(&"git.commit"),
        "Expected git.commit in results, got: {:?}",
        tool_names
    );

    let commit_score = results
        .iter()
        .find(|(n, _)| n == "git.commit")
        .map(|(_, s)| *s);
    let status_score = results
        .iter()
        .find(|(n, _)| n == "git.status")
        .map(|(_, s)| *s);
    if let (Some(cs), Some(ss)) = (commit_score, status_score) {
        assert!(
            cs > ss,
            "git.commit ({}) should score higher than git.status ({})",
            cs,
            ss,
        );
    }
}

#[test]
fn test_query_tool_relevance_empty_graph() {
    let graph = KnowledgeGraph::new();
    let results = graph.query_tool_relevance(&["anything".to_string()], 2, 10);
    assert!(results.is_empty());
}

#[test]
fn test_query_tool_relevance_multi_term() {
    let graph = KnowledgeGraph::new();

    let docs = vec![
        SkillDoc {
            id: "knowledge".to_string(),
            doc_type: "skill".to_string(),
            skill_name: "knowledge".to_string(),
            tool_name: String::new(),
            content: "Knowledge base".to_string(),
            routing_keywords: vec![],
        },
        SkillDoc {
            id: "knowledge.recall".to_string(),
            doc_type: "command".to_string(),
            skill_name: "knowledge".to_string(),
            tool_name: "knowledge.recall".to_string(),
            content: "Recall knowledge".to_string(),
            routing_keywords: vec!["search".to_string(), "recall".to_string()],
        },
        SkillDoc {
            id: "researcher".to_string(),
            doc_type: "skill".to_string(),
            skill_name: "researcher".to_string(),
            tool_name: String::new(),
            content: "Web research".to_string(),
            routing_keywords: vec![],
        },
        SkillDoc {
            id: "researcher.search".to_string(),
            doc_type: "command".to_string(),
            skill_name: "researcher".to_string(),
            tool_name: "researcher.search".to_string(),
            content: "Search the web".to_string(),
            routing_keywords: vec!["search".to_string(), "web".to_string()],
        },
    ];
    graph.register_skill_entities(&docs).unwrap();

    let results = graph.query_tool_relevance(&["search".to_string(), "recall".to_string()], 2, 10);

    let tool_names: Vec<&str> = results.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        tool_names.contains(&"knowledge.recall"),
        "Expected knowledge.recall, got: {:?}",
        tool_names,
    );
    assert!(
        tool_names.contains(&"researcher.search"),
        "Expected researcher.search, got: {:?}",
        tool_names,
    );
}
