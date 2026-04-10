use super::document::{
    StudioGraphqlFilterOperator, StudioGraphqlFilterPredicate, StudioGraphqlScalarValue,
    StudioGraphqlSortOption, StudioGraphqlTableQuery,
};

pub(crate) fn build_graphql_sql_query(query: &StudioGraphqlTableQuery) -> Result<String, String> {
    let projection = query
        .columns
        .iter()
        .map(|column| sql_identifier(column))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    let table_name = sql_identifier(&query.table_name)?;

    let mut sql = format!("SELECT {projection} FROM {table_name}");

    if !query.filters.is_empty() {
        let mut predicates = query
            .filters
            .iter()
            .map(predicate_sql)
            .collect::<Result<Vec<_>, _>>()?;
        predicates.sort();
        let where_clause = predicates.join(" AND ");
        sql.push_str(" WHERE ");
        sql.push_str(where_clause.as_str());
    }

    if !query.sort.is_empty() {
        let order_by = query
            .sort
            .iter()
            .map(sort_sql)
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        sql.push_str(" ORDER BY ");
        sql.push_str(order_by.as_str());
    }

    if let Some(limit) = query.limit {
        sql.push_str(format!(" LIMIT {limit}").as_str());
        let offset = query
            .page
            .map_or(0, |page| page.saturating_sub(1).saturating_mul(limit));
        if offset > 0 {
            sql.push_str(format!(" OFFSET {offset}").as_str());
        }
    }

    Ok(sql)
}

fn predicate_sql(predicate: &StudioGraphqlFilterPredicate) -> Result<String, String> {
    Ok(format!(
        "{} {} {}",
        sql_identifier(&predicate.column_name)?,
        filter_operator_sql(predicate.operator),
        scalar_sql_literal(&predicate.value)?,
    ))
}

fn sort_sql(sort: &StudioGraphqlSortOption) -> Result<String, String> {
    Ok(format!(
        "{} {} NULLS FIRST",
        sql_identifier(&sort.field_name)?,
        if sort.descending { "DESC" } else { "ASC" },
    ))
}

fn filter_operator_sql(operator: StudioGraphqlFilterOperator) -> &'static str {
    match operator {
        StudioGraphqlFilterOperator::Eq => "=",
        StudioGraphqlFilterOperator::Lt => "<",
        StudioGraphqlFilterOperator::LtEq => "<=",
        StudioGraphqlFilterOperator::Gt => ">",
        StudioGraphqlFilterOperator::GtEq => ">=",
    }
}

fn scalar_sql_literal(value: &StudioGraphqlScalarValue) -> Result<String, String> {
    match value {
        StudioGraphqlScalarValue::Boolean(value) => Ok(if *value {
            "TRUE".to_string()
        } else {
            "FALSE".to_string()
        }),
        StudioGraphqlScalarValue::String(value) => Ok(format!("'{}'", value.replace('\'', "''"))),
        StudioGraphqlScalarValue::Int(value) => Ok(value.to_string()),
        StudioGraphqlScalarValue::Float(value) => {
            if value.is_finite() {
                Ok(value.to_string())
            } else {
                Err("GraphQL float filters must be finite".to_string())
            }
        }
    }
}

fn sql_identifier(value: &str) -> Result<String, String> {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return Err("GraphQL SQL translation requires non-empty identifiers".to_string());
    };
    if !matches!(first, 'a'..='z' | 'A'..='Z' | '_') {
        return Err(format!(
            "GraphQL SQL translation rejected identifier `{value}`"
        ));
    }
    if !characters.all(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_')) {
        return Err(format!(
            "GraphQL SQL translation rejected identifier `{value}`"
        ));
    }
    Ok(format!("\"{value}\""))
}

#[cfg(test)]
#[path = "../../../../tests/unit/search/queries/graphql/translation.rs"]
mod tests;
