use super::validate_sql_query_request;

#[test]
fn sql_query_request_validation_accepts_read_only_query() {
    assert!(validate_sql_query_request("SELECT doc_id FROM repo_entity").is_ok());
}

#[test]
fn sql_query_request_validation_rejects_blank_query() {
    assert_eq!(
        validate_sql_query_request("   "),
        Err("SQL query text must not be blank".to_string())
    );
}

#[test]
fn sql_query_request_validation_rejects_multiple_statements() {
    assert_eq!(
        validate_sql_query_request("SELECT 1; SELECT 2"),
        Err("SQL query text must contain exactly one statement".to_string())
    );
}

#[test]
fn sql_query_request_validation_rejects_non_query_statement() {
    assert_eq!(
        validate_sql_query_request("CREATE VIEW demo AS SELECT 1"),
        Err("SQL query text must be a read-only query statement".to_string())
    );
}
