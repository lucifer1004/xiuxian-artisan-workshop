use xiuxian_wendao_parsers::frontmatter::{parse_frontmatter, split_frontmatter};

#[test]
fn split_frontmatter_returns_yaml_value_and_body() {
    let content = "---\ntitle: Example\n---\n# Body\n";
    let (frontmatter, body) = split_frontmatter(content);
    let title = frontmatter
        .as_ref()
        .and_then(|value| value.get("title"))
        .and_then(serde_yaml::Value::as_str);
    assert_eq!(title, Some("Example"));
    assert_eq!(body, "# Body\n");
}

#[test]
fn parse_frontmatter_extracts_top_level_fields() {
    let content =
        "---\ntitle: My Note\ndescription: A test\ntags:\n  - python\n  - rust\n---\n# Content";
    let frontmatter = parse_frontmatter(content);
    assert_eq!(frontmatter.title.as_deref(), Some("My Note"));
    assert_eq!(frontmatter.description.as_deref(), Some("A test"));
    assert_eq!(frontmatter.tags, vec!["python", "rust"]);
}

#[test]
fn parse_frontmatter_extracts_skill_metadata() {
    let content = "---\nname: git\ndescription: Git ops\nmetadata:\n  routing_keywords:\n    - commit\n    - branch\n  intents:\n    - version_control\n---\n# SKILL";
    let frontmatter = parse_frontmatter(content);
    assert_eq!(frontmatter.name.as_deref(), Some("git"));
    assert_eq!(frontmatter.routing_keywords, vec!["commit", "branch"]);
    assert_eq!(frontmatter.intents, vec!["version_control"]);
}

#[test]
fn parse_frontmatter_falls_back_to_metadata_tags() {
    let content = "---\nmetadata:\n  tags:\n    - search\n    - vector\n---\n# Content";
    let frontmatter = parse_frontmatter(content);
    assert_eq!(frontmatter.tags, vec!["search", "vector"]);
}

#[test]
fn parse_frontmatter_without_yaml_returns_default() {
    let frontmatter = parse_frontmatter("# No frontmatter");
    assert!(frontmatter.title.is_none());
    assert!(frontmatter.tags.is_empty());
}

#[test]
fn parse_frontmatter_malformed_returns_default() {
    let frontmatter = parse_frontmatter("---\n: bad [[\n---\n");
    assert!(frontmatter.title.is_none());
}
