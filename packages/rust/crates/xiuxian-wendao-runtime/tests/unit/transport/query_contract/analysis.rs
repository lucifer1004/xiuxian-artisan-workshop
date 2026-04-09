use super::{validate_code_ast_analysis_request, validate_markdown_analysis_request};

#[test]
fn markdown_analysis_request_validation_accepts_stable_request() {
    assert!(validate_markdown_analysis_request("docs/analysis.md").is_ok());
}

#[test]
fn markdown_analysis_request_validation_rejects_blank_path() {
    assert_eq!(
        validate_markdown_analysis_request("   "),
        Err("markdown analysis path must not be blank".to_string())
    );
}

#[test]
fn code_ast_analysis_request_validation_accepts_stable_request() {
    assert!(validate_code_ast_analysis_request("src/lib.jl", "demo", Some(7)).is_ok());
}

#[test]
fn code_ast_analysis_request_validation_rejects_blank_repo() {
    assert_eq!(
        validate_code_ast_analysis_request("src/lib.jl", "   ", Some(7)),
        Err("code AST analysis repo must not be blank".to_string())
    );
}

#[test]
fn code_ast_analysis_request_validation_rejects_zero_line_hint() {
    assert_eq!(
        validate_code_ast_analysis_request("src/lib.jl", "demo", Some(0)),
        Err("code AST analysis line hint must be greater than zero".to_string())
    );
}
