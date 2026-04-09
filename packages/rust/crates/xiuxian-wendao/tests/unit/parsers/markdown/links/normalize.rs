use super::{normalize_attachment_target, normalize_markdown_note_target};

#[test]
fn normalize_markdown_note_target_resolves_against_source_directory() {
    let source_path = std::path::Path::new("/workspace/docs/index.md");
    let root = std::path::Path::new("/workspace");

    let resolved = normalize_markdown_note_target("testing/README.md", source_path, root);

    assert_eq!(resolved.as_deref(), Some("docs/testing/readme"));
}

#[test]
fn normalize_attachment_target_resolves_against_source_directory() {
    let source_path = std::path::Path::new("/workspace/docs/index.md");
    let root = std::path::Path::new("/workspace");

    let resolved = normalize_attachment_target("assets/diagram.svg", source_path, root);

    assert_eq!(resolved.as_deref(), Some("docs/assets/diagram.svg"));
}
