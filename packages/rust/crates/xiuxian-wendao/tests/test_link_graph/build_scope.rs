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
fn test_link_graph_build_with_excluded_dirs_skips_cache_tree()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# Alpha\n\n[[b]]\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# Beta\n\n[[a]]\n")?;
    write_file(
        &tmp.path().join(".cache/huge.md"),
        "# Should Be Skipped\n\n[[docs/a]]\n",
    )?;

    let excluded = vec![".cache".to_string()];
    let index = LinkGraphIndex::build_with_excluded_dirs(tmp.path(), &excluded)
        .map_err(|e| e.to_string())?;

    let stats = index.stats();
    assert_eq!(stats.total_notes, 2);
    assert_eq!(stats.links_in_graph, 2);
    assert_eq!(stats.orphans, 0);

    let toc_paths: Vec<String> = index.toc(10).into_iter().map(|row| row.path).collect();
    assert!(!toc_paths.iter().any(|path| path.contains(".cache/")));
    Ok(())
}

#[test]
fn test_link_graph_build_skips_hidden_dirs_by_default() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# Alpha\n\n[[b]]\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# Beta\n\n[[a]]\n")?;
    write_file(
        &tmp.path().join(".github/hidden.md"),
        "# Hidden\n\n[[docs/a]]\n",
    )?;

    let index = LinkGraphIndex::build(tmp.path()).map_err(|e| e.to_string())?;
    let stats = index.stats();
    assert_eq!(stats.total_notes, 2);
    assert_eq!(stats.links_in_graph, 2);

    let toc_paths: Vec<String> = index.toc(10).into_iter().map(|row| row.path).collect();
    assert!(!toc_paths.iter().any(|path| path.starts_with(".github/")));
    Ok(())
}

#[test]
fn test_link_graph_build_with_include_dirs_limits_scope() -> Result<(), Box<dyn std::error::Error>>
{
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# Alpha\n\n[[b]]\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# Beta\n\n[[a]]\n")?;
    write_file(
        &tmp.path().join("assets/knowledge/c.md"),
        "# Gamma\n\n[[docs/a]]\n",
    )?;

    let include = vec!["docs".to_string()];
    let index =
        LinkGraphIndex::build_with_filters(tmp.path(), &include, &[]).map_err(|e| e.to_string())?;

    let stats = index.stats();
    assert_eq!(stats.total_notes, 2);
    assert_eq!(stats.links_in_graph, 2);
    assert_eq!(stats.orphans, 0);

    let toc_paths: Vec<String> = index.toc(10).into_iter().map(|row| row.path).collect();
    assert!(toc_paths.iter().all(|path| path.starts_with("docs/")));
    Ok(())
}
