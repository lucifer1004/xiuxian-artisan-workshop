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
fn embedded_template_catalog_renders_text() -> Result<(), Box<dyn std::error::Error>> {
    let rendered = TEST_TEMPLATE_CATALOG.render_text(
        TEST_TEMPLATE_NAME,
        json!({
            "value": "alpha",
            "tail": "omega",
        }),
    )?;

    assert_eq!(rendered, "Value: alpha\nTail: omega");
    Ok(())
}

#[test]
fn embedded_template_catalog_renders_lines() -> Result<(), Box<dyn std::error::Error>> {
    let rendered = TEST_TEMPLATE_CATALOG.render_lines(
        TEST_TEMPLATE_NAME,
        json!({
            "value": "alpha",
            "tail": "omega",
        }),
    )?;

    assert_eq!(rendered, vec!["Value: alpha", "Tail: omega"]);
    Ok(())
}
