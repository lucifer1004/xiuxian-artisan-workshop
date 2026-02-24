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
fn test_link_graph_related_filter_seed_accuracy_is_cluster_grounded()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(
        &tmp.path().join("docs/arch-seed.md"),
        "# Arch Seed\n\nplatform note\n\n[[arch-a]] [[arch-b]]\n",
    )?;
    write_file(
        &tmp.path().join("docs/arch-a.md"),
        "# Arch A\n\nplatform note\n\n[[arch-c]]\n",
    )?;
    write_file(
        &tmp.path().join("docs/arch-b.md"),
        "# Arch B\n\nplatform note\n",
    )?;
    write_file(
        &tmp.path().join("docs/arch-c.md"),
        "# Arch C\n\nplatform note\n",
    )?;

    write_file(
        &tmp.path().join("docs/db-seed.md"),
        "# DB Seed\n\nplatform note\n\n[[db-a]] [[db-b]]\n",
    )?;
    write_file(
        &tmp.path().join("docs/db-a.md"),
        "# DB A\n\nplatform note\n",
    )?;
    write_file(
        &tmp.path().join("docs/db-b.md"),
        "# DB B\n\nplatform note\n",
    )?;

    let index = LinkGraphIndex::build(tmp.path()).map_err(|e| e.to_string())?;
    let ppr = LinkGraphRelatedPprOptions {
        alpha: Some(0.9),
        max_iter: Some(32),
        tol: Some(1e-6),
        subgraph_mode: Some(LinkGraphPprSubgraphMode::Force),
    };

    // Emulates external semantic seed handoff (for example Librarian/vector stage) by
    // passing a path-form seed alias into related filter and verifying cluster grounding.
    let arch_options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            related: Some(LinkGraphRelatedFilter {
                seeds: vec!["docs/arch-seed.md".to_string()],
                max_distance: Some(3),
                ppr: Some(ppr.clone()),
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let arch_hits = index.search_planned("platform note", 16, arch_options).1;
    assert!(
        !arch_hits.is_empty(),
        "expected hits for arch semantic seed"
    );
    let arch_stems: HashSet<String> = arch_hits.iter().map(|row| row.stem.clone()).collect();
    assert!(
        arch_stems.contains("arch-a")
            || arch_stems.contains("arch-b")
            || arch_stems.contains("arch-c"),
        "expected architecture cluster hits, got {:?}",
        arch_stems
    );
    assert!(
        !arch_stems.iter().any(|stem| stem.starts_with("db-")),
        "arch seed should not return db cluster hits: {:?}",
        arch_stems
    );

    let db_options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            related: Some(LinkGraphRelatedFilter {
                seeds: vec!["db-seed".to_string()],
                max_distance: Some(3),
                ppr: Some(ppr),
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };
    let db_hits = index.search_planned("platform note", 16, db_options).1;
    assert!(!db_hits.is_empty(), "expected hits for db semantic seed");
    let db_stems: HashSet<String> = db_hits.iter().map(|row| row.stem.clone()).collect();
    assert!(
        db_stems.contains("db-a") || db_stems.contains("db-b"),
        "expected db cluster hits, got {:?}",
        db_stems
    );
    assert!(
        !db_stems.iter().any(|stem| stem.starts_with("arch-")),
        "db seed should not return architecture cluster hits: {:?}",
        db_stems
    );
    Ok(())
}
