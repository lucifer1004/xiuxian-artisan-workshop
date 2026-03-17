#![doc = "Integration tests for Wendao native search tools under qianhuan feature wiring."]
#![cfg(feature = "zhenfa-router")]

use std::fs;
use std::path::Path;

use serde_json::json;
use tempfile::TempDir;
use xiuxian_wendao::{LinkGraphIndex, WendaoSearchTool};
use xiuxian_zhenfa::{ZhenfaContext, ZhenfaTool};

fn search_tool() -> WendaoSearchTool {
    WendaoSearchTool
}

fn context_with_index(root: &Path) -> ZhenfaContext {
    let mut ctx = ZhenfaContext::default();
    let index = LinkGraphIndex::build(root)
        .unwrap_or_else(|error| panic!("build link graph index for test context: {error}"));
    let _ = ctx.insert_extension(index);
    ctx
}

#[tokio::test]
async fn wendao_search_tool_emits_semantic_hit_type_for_journal_paths() {
    let notebook = TempDir::new().unwrap_or_else(|error| panic!("create temp dir: {error}"));
    let journal_dir = notebook.path().join("journal");
    fs::create_dir_all(&journal_dir).unwrap_or_else(|error| panic!("create journal dir: {error}"));
    fs::write(
        journal_dir.join("daily.md"),
        "# Daily Journal\n\njournal semantic type marker unique-native-tool.\n",
    )
    .unwrap_or_else(|error| panic!("write journal note: {error}"));

    let tool = search_tool();
    let ctx = context_with_index(notebook.path());
    let output = tool
        .call_native(
            &ctx,
            json!({
                "query": "unique-native-tool",
                "limit": 5
            }),
        )
        .await
        .unwrap_or_else(|error| panic!("native dispatch should classify journal path: {error}"));

    assert!(output.contains("<hit id=\"journal/daily.md\""));
    assert!(output.contains("type=\"journal\""));
}

#[tokio::test]
async fn wendao_search_tool_prefers_tag_driven_hit_type_for_ambiguous_paths() {
    let notebook = TempDir::new().unwrap_or_else(|error| panic!("create temp dir: {error}"));
    let notes_dir = notebook.path().join("notes");
    fs::create_dir_all(&notes_dir).unwrap_or_else(|error| panic!("create notes dir: {error}"));
    fs::write(
        notes_dir.join("entry.md"),
        "---\ntags:\n  - journal\n---\n# Entry\n\ntag-driven-classification-marker.\n",
    )
    .unwrap_or_else(|error| panic!("write tagged note: {error}"));

    let tool = search_tool();
    let ctx = context_with_index(notebook.path());
    let output = tool
        .call_native(
            &ctx,
            json!({
                "query": "tag-driven-classification-marker",
                "limit": 5
            }),
        )
        .await
        .unwrap_or_else(|error| panic!("native dispatch should classify from tags: {error}"));

    assert!(output.contains("<hit id=\"notes/entry.md\""));
    assert!(output.contains("type=\"journal\""));
}

#[tokio::test]
async fn wendao_search_tool_prefers_frontmatter_type_over_path_and_tags() {
    let notebook = TempDir::new().unwrap_or_else(|error| panic!("create temp dir: {error}"));
    let journal_dir = notebook.path().join("journal");
    fs::create_dir_all(&journal_dir).unwrap_or_else(|error| panic!("create journal dir: {error}"));
    fs::write(
        journal_dir.join("override.md"),
        "---\ntype: agenda\ntags:\n  - journal\n---\n# Override\n\ndoc-type-precedence-marker.\n",
    )
    .unwrap_or_else(|error| panic!("write typed note: {error}"));

    let tool = search_tool();
    let ctx = context_with_index(notebook.path());
    let output = tool
        .call_native(
            &ctx,
            json!({
                "query": "doc-type-precedence-marker",
                "limit": 5
            }),
        )
        .await
        .unwrap_or_else(|error| {
            panic!("native dispatch should classify from frontmatter type: {error}")
        });

    assert!(output.contains("<hit id=\"journal/override.md\""));
    assert!(output.contains("type=\"agenda\""));
}
