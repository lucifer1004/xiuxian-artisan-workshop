use super::{MarkdownShowSection, render_show_surface};

#[test]
fn render_show_surface_uses_shared_shape() {
    let rendered = render_show_surface(
        "Flowhub",
        &[
            "Location: /tmp/flowhub".to_string(),
            "Modules: 2".to_string(),
        ],
        &[MarkdownShowSection {
            title: "rust".into(),
            lines: vec![
                "Path: /tmp/flowhub/rust".to_string(),
                "Kind: leaf".to_string(),
            ],
        }],
    );

    assert_eq!(
        rendered,
        "# Flowhub\n\nLocation: /tmp/flowhub\nModules: 2\n\n## rust\nPath: /tmp/flowhub/rust\nKind: leaf"
    );
}

#[test]
fn render_show_surface_skips_header_block_when_empty() {
    let rendered = render_show_surface(
        "Scenario Work Surface Preview",
        &[],
        &[MarkdownShowSection {
            title: "Links".into(),
            lines: vec!["- blueprint -> plan".to_string()],
        }],
    );

    assert_eq!(
        rendered,
        "# Scenario Work Surface Preview\n\n## Links\n- blueprint -> plan"
    );
}
