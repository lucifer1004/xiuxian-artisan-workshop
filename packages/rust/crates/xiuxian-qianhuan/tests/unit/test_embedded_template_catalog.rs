//! Focused coverage for the shared embedded manifestation-template catalog.

use serde_json::json;
use xiuxian_qianhuan::EmbeddedManifestationTemplateCatalog;

const TEST_TEMPLATE_NAME: &str = "test.md.j2";
const TEST_TEMPLATE_SOURCE: &str = "Value: {{ value }}\nTail: {{ tail }}";

static TEST_TEMPLATE_CATALOG: EmbeddedManifestationTemplateCatalog =
    EmbeddedManifestationTemplateCatalog::new(
        "unit embedded template catalog",
        &[(TEST_TEMPLATE_NAME, TEST_TEMPLATE_SOURCE)],
    );

#[test]
fn embedded_template_catalog_renders_text() {
    let rendered = TEST_TEMPLATE_CATALOG
        .render_text(
            TEST_TEMPLATE_NAME,
            json!({
                "value": "alpha",
                "tail": "omega",
            }),
        )
        .expect("embedded template catalog should render text");

    assert_eq!(rendered, "Value: alpha\nTail: omega");
}

#[test]
fn embedded_template_catalog_renders_lines() {
    let rendered = TEST_TEMPLATE_CATALOG
        .render_lines(
            TEST_TEMPLATE_NAME,
            json!({
                "value": "alpha",
                "tail": "omega",
            }),
        )
        .expect("embedded template catalog should render lines");

    assert_eq!(rendered, vec!["Value: alpha", "Tail: omega"]);
}
