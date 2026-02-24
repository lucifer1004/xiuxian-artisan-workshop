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
fn test_link_graph_search_temporal_filters_and_sorting() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(
        &tmp.path().join("docs/a.md"),
        "---\ncreated: 2024-01-01\nmodified: 2024-01-05\n---\n# A\n",
    )?;
    write_file(
        &tmp.path().join("docs/b.md"),
        "---\ncreated: 2024-01-03\nmodified: 2024-01-02\n---\n# B\n",
    )?;
    write_file(
        &tmp.path().join("docs/c.md"),
        "---\ncreated: 2024-01-10\nmodified: 2024-01-12\n---\n# C\n",
    )?;
    let index = LinkGraphIndex::build(tmp.path()).map_err(|e| e.to_string())?;

    let created_window = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(
            LinkGraphSortField::Created,
            LinkGraphSortOrder::Asc,
        )],
        created_after: Some(1_704_153_600),  // 2024-01-02
        created_before: Some(1_704_758_400), // 2024-01-09
        ..LinkGraphSearchOptions::default()
    };
    let created_hits = index.search_planned("", 10, created_window).1;
    assert_eq!(created_hits.len(), 1);
    assert_eq!(created_hits[0].path, "docs/b.md");

    let modified_sorted = LinkGraphSearchOptions {
        sort_terms: vec![sort_term(
            LinkGraphSortField::Modified,
            LinkGraphSortOrder::Desc,
        )],
        modified_after: Some(1_704_153_600), // 2024-01-02
        ..LinkGraphSearchOptions::default()
    };
    let modified_hits = index.search_planned("", 10, modified_sorted).1;
    assert_eq!(modified_hits.len(), 3);
    assert_eq!(modified_hits[0].path, "docs/c.md");
    assert_eq!(modified_hits[1].path, "docs/a.md");
    assert_eq!(modified_hits[2].path, "docs/b.md");
    Ok(())
}
