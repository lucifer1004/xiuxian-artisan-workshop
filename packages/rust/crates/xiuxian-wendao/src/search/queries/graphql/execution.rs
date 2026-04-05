use datafusion::logical_expr::Operator;
use datafusion::prelude::{Column, Expr, binary_expr, col};
use datafusion::scalar::ScalarValue;
use serde_json::{Map, Value};

use crate::search::queries::SearchQueryService;
use crate::search::queries::graphql::context::GraphqlExecutionContext;
use crate::search::queries::graphql::document::{
    StudioGraphqlFilterOperator, StudioGraphqlFilterPredicate, StudioGraphqlScalarValue,
    StudioGraphqlTableQuery, parse_graphql_document,
};
use crate::search::queries::graphql::payload::GraphqlQueryPayload;
use crate::search::queries::sql::engine_batches_rows_payload;

/// Execute one `GraphQL` document against the shared request-scoped SQL surface.
///
/// # Errors
///
/// Returns an error when the `GraphQL` document cannot be parsed or when the
/// shared `DataFusion` query surface cannot be planned, executed, or
/// serialized.
pub async fn query_graphql_payload(
    service: &SearchQueryService,
    document: &str,
) -> Result<GraphqlQueryPayload, String> {
    let context = GraphqlExecutionContext::new().with_query_service(service.clone());
    query_graphql_payload_with_context(&context, document).await
}

pub(crate) async fn query_graphql_payload_with_context(
    context: &GraphqlExecutionContext,
    document: &str,
) -> Result<GraphqlQueryPayload, String> {
    let query = parse_graphql_document(document)?;
    let value = execute_table_query(context, &query).await?;
    let mut data = Map::new();
    data.insert(query.response_key, value);
    Ok(GraphqlQueryPayload { data })
}

async fn execute_table_query(
    context: &GraphqlExecutionContext,
    query: &StudioGraphqlTableQuery,
) -> Result<Value, String> {
    let Some(query_service) = context.query_service() else {
        return Err("GraphQL queries require a shared query service".to_string());
    };

    let query_core = query_service.open_core().await?;
    let mut dataframe = query_core
        .engine()
        .table(query.table_name.as_str())
        .await
        .map_err(|error| {
            format!(
                "GraphQL table `{}` is not visible in the request-scoped SQL surface: {error}",
                query.table_name
            )
        })?;

    for predicate in &query.filters {
        dataframe = dataframe
            .filter(filter_expression(predicate))
            .map_err(|error| format!("GraphQL filter planning failed: {error}"))?;
    }

    let projected_columns = query.columns.iter().map(String::as_str).collect::<Vec<_>>();
    dataframe = dataframe
        .select_columns(projected_columns.as_slice())
        .map_err(|error| format!("GraphQL projection planning failed: {error}"))?;

    if !query.sort.is_empty() {
        let sort = query
            .sort
            .iter()
            .map(|entry| col(entry.field_name.as_str()).sort(!entry.descending, true))
            .collect::<Vec<_>>();
        dataframe = dataframe
            .sort(sort)
            .map_err(|error| format!("GraphQL sort planning failed: {error}"))?;
    }

    if let Some(limit) = query.limit {
        let skip = query
            .page
            .map_or(0, |page| page.saturating_sub(1).saturating_mul(limit));
        dataframe = dataframe
            .limit(skip, Some(limit))
            .map_err(|error| format!("GraphQL limit planning failed: {error}"))?;
    }

    let batches = query_core
        .engine()
        .collect_dataframe(dataframe)
        .await
        .map_err(|error| format!("GraphQL query execution failed: {error}"))?;
    let rows = engine_batches_rows_payload(batches.as_slice())?;
    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

fn filter_expression(predicate: &StudioGraphqlFilterPredicate) -> Expr {
    binary_expr(
        Expr::Column(Column::from_name(predicate.column_name.clone())),
        filter_operator(predicate.operator),
        scalar_expression(&predicate.value),
    )
}

fn filter_operator(operator: StudioGraphqlFilterOperator) -> Operator {
    match operator {
        StudioGraphqlFilterOperator::Eq => Operator::Eq,
        StudioGraphqlFilterOperator::Lt => Operator::Lt,
        StudioGraphqlFilterOperator::LtEq => Operator::LtEq,
        StudioGraphqlFilterOperator::Gt => Operator::Gt,
        StudioGraphqlFilterOperator::GtEq => Operator::GtEq,
    }
}

fn scalar_expression(value: &StudioGraphqlScalarValue) -> Expr {
    let scalar = match value {
        StudioGraphqlScalarValue::Boolean(value) => ScalarValue::Boolean(Some(*value)),
        StudioGraphqlScalarValue::String(value) => ScalarValue::Utf8(Some(value.clone())),
        StudioGraphqlScalarValue::Int(value) => ScalarValue::Int64(Some(*value)),
        StudioGraphqlScalarValue::Float(value) => ScalarValue::Float64(Some(*value)),
    };
    Expr::Literal(scalar, None)
}
