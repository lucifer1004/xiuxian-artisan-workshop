use super::parse_symbol_declarations;
use serde_json::json;

#[test]
fn parse_symbol_declarations_supports_secondary_keywords() {
    let payload = parse_symbol_declarations(
        r"
record ControllerState
end ControllerState;

parameter Gain = 1;

block Limiter
end Limiter;
",
    )
    .into_iter()
    .map(|declaration| {
        json!({
            "name": declaration.name,
            "kind": format!("{:?}", declaration.kind),
            "signature": declaration.signature,
            "line_start": declaration.line_start,
            "equations_count": declaration.equations.len(),
        })
    })
    .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "parse_symbol_declarations_supports_secondary_keywords",
        payload
    );
}
