use std::path::Path;

#[test]
fn canonical_parser_namespace_parses_workflow_fixture() {
    let workflow_content = include_str!("../../../fixtures/workflow_demo/input/workflow.md");
    let root = Path::new("/tmp/workflow-demo");
    let path = Path::new("/tmp/workflow-demo/workflow.md");

    let canonical = crate::parsers::markdown::parse_note(path, root, workflow_content)
        .expect("canonical parser should parse workflow fixture");

    assert_eq!(canonical.doc.id, "workflow");
    assert_eq!(canonical.doc.title, "Task: Refactor Authentication Logic");
    assert!(!canonical.sections.is_empty());
    assert_eq!(
        canonical.sections[0].heading_title,
        "Task: Refactor Authentication Logic"
    );
}
