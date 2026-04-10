use crate::search::queries::graphql::document::parse_graphql_document;

use super::build_graphql_sql_query;

#[test]
fn graphql_translation_builds_sql_text_for_table_query() {
    let query = parse_graphql_document(
        r#"
        {
          reference_occurrence(
            filter: { name: { eq: "AlphaService" }, line: { gte: 10 } }
            sort: [{ field: "line", order: "desc" }]
            limit: 5
            page: 2
          ) {
            name
            path
            line
          }
        }
        "#,
    )
    .unwrap_or_else(|error| panic!("parse graphql document: {error}"));

    let sql = build_graphql_sql_query(&query)
        .unwrap_or_else(|error| panic!("build graphql sql: {error}"));

    assert_eq!(
        sql,
        "SELECT \"name\", \"path\", \"line\" FROM \"reference_occurrence\" WHERE \"line\" >= 10 AND \"name\" = 'AlphaService' ORDER BY \"line\" DESC NULLS FIRST LIMIT 5 OFFSET 5"
    );
}

#[test]
fn graphql_translation_escapes_string_literals() {
    let query = parse_graphql_document(
        r#"
        {
          reference_occurrence(filter: { name: "Alpha'Service" }) {
            name
          }
        }
        "#,
    )
    .unwrap_or_else(|error| panic!("parse graphql document: {error}"));

    let sql = build_graphql_sql_query(&query)
        .unwrap_or_else(|error| panic!("build graphql sql: {error}"));

    assert_eq!(
        sql,
        "SELECT \"name\" FROM \"reference_occurrence\" WHERE \"name\" = 'Alpha''Service'"
    );
}
