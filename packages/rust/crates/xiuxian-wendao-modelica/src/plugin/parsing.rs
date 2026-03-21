use xiuxian_wendao::repo_intelligence::RepoSymbolKind;

use super::types::ParsedDeclaration;

pub(crate) fn parse_package_name(contents: &str) -> Option<String> {
    contents
        .lines()
        .find_map(|line| parse_named_declaration(line, &["package"]))
}

pub(crate) fn contains_documentation_annotation(contents: &str) -> bool {
    contents.contains("Documentation(")
}

pub(crate) fn parse_symbol_declarations(contents: &str) -> Vec<ParsedDeclaration> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("end ") || trimmed == "end;" {
                return None;
            }
            if let Some(name) = parse_named_declaration(trimmed, &["function"]) {
                return Some(ParsedDeclaration {
                    name,
                    kind: RepoSymbolKind::Function,
                    signature: trimmed.to_string(),
                });
            }
            if let Some(name) =
                parse_named_declaration(trimmed, &["model", "record", "block", "connector", "type"])
            {
                return Some(ParsedDeclaration {
                    name,
                    kind: RepoSymbolKind::Type,
                    signature: trimmed.to_string(),
                });
            }
            if let Some(name) = parse_named_declaration(trimmed, &["constant", "parameter"]) {
                return Some(ParsedDeclaration {
                    name,
                    kind: RepoSymbolKind::Constant,
                    signature: trimmed.to_string(),
                });
            }
            None
        })
        .collect()
}

fn parse_named_declaration(line: &str, keywords: &[&str]) -> Option<String> {
    for keyword in keywords {
        let Some(suffix) = line.strip_prefix(keyword) else {
            continue;
        };
        let first = suffix.chars().next()?;
        if !first.is_whitespace() {
            continue;
        }
        let ident = suffix
            .trim_start()
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .next()?;
        if !ident.is_empty() {
            return Some(ident.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_symbol_declarations;

    #[test]
    fn parse_symbol_declarations_supports_secondary_keywords() {
        let payload = parse_symbol_declarations(
            r#"
record ControllerState
end ControllerState;

parameter Gain = 1;

block Limiter
end Limiter;
"#,
        )
        .into_iter()
        .map(|declaration| {
            json!({
                "name": declaration.name,
                "kind": format!("{:?}", declaration.kind),
                "signature": declaration.signature,
            })
        })
        .collect::<Vec<_>>();

        insta::assert_json_snapshot!(
            "parse_symbol_declarations_supports_secondary_keywords",
            payload
        );
    }
}
