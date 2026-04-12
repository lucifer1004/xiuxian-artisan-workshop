use regex::Regex;
use serde_yaml::Value;
use std::sync::LazyLock;

fn compile_regex(pattern: &str) -> Regex {
    match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(_compile_err) => match Regex::new(r"$^") {
            Ok(fallback) => fallback,
            Err(fallback_err) => panic!("hardcoded fallback regex must compile: {fallback_err}"),
        },
    }
}

static FRONTMATTER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"(?s)\A---\s*\n(.*?)\n(?:---|\.\.\.)\s*\n?"));

/// Split one Markdown document into an optional parsed YAML frontmatter value
/// and the remaining body content.
#[must_use]
pub fn split_frontmatter(content: &str) -> (Option<Value>, &str) {
    let Some(caps) = FRONTMATTER_REGEX.captures(content) else {
        return (None, content);
    };
    let body = caps.get(0).map_or(content, |m| &content[m.end()..]);
    let parsed = caps
        .get(1)
        .and_then(|m| serde_yaml::from_str::<Value>(m.as_str()).ok());
    (parsed, body)
}
