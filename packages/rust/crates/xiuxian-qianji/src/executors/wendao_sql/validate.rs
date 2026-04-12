use async_trait::async_trait;
use serde_json::json;

use crate::contracts::{FlowInstruction, QianjiMechanism, QianjiOutput};

use super::contract::{SqlAuthorSpec, SqlFilter, SqlOrderTerm, SurfaceBundle, SurfaceColumn};
use super::input::required_context_string;
use super::render::validation_report_xml;

/// Deterministic validation gate for XML-authored Wendao SQL specs.
pub struct WendaoSqlValidateMechanism {
    /// Context key containing the discovery bundle XML.
    pub surface_bundle_key: String,
    /// Context key containing the author output XML.
    pub author_spec_key: String,
    /// Output context key storing the canonical validated SQL.
    pub output_key: String,
    /// Output context key storing the validation report XML.
    pub report_key: String,
    /// Output context key storing rejection details.
    pub error_key: String,
    /// Branch label selected when validation succeeds.
    pub accepted_branch_label: Option<String>,
    /// Branch label selected when validation fails.
    pub rejected_branch_label: Option<String>,
}

#[async_trait]
impl QianjiMechanism for WendaoSqlValidateMechanism {
    async fn execute(&self, context: &serde_json::Value) -> Result<QianjiOutput, String> {
        let surface_bundle_raw =
            required_context_string(context, self.surface_bundle_key.as_str())?;
        let author_spec_raw = required_context_string(context, self.author_spec_key.as_str())?;
        let bundle = super::contract::parse_surface_bundle_xml(surface_bundle_raw.as_str())?;
        let spec = super::contract::parse_sql_author_spec_xml(author_spec_raw.as_str())?;

        match validate_and_render_sql(&bundle, &spec) {
            Ok(canonical_sql) => Ok(QianjiOutput {
                data: json!({
                    self.output_key.clone(): canonical_sql.clone(),
                    self.report_key.clone(): validation_report_xml("accepted", "SQL author spec accepted", Some(canonical_sql.as_str())),
                }),
                instruction: branch_or_continue(self.accepted_branch_label.as_deref()),
            }),
            Err(message) => {
                let report = validation_report_xml("rejected", message.as_str(), None);
                if let Some(label) = self.rejected_branch_label.as_deref() {
                    Ok(QianjiOutput {
                        data: json!({
                            self.report_key.clone(): report,
                            self.error_key.clone(): message,
                        }),
                        instruction: FlowInstruction::SelectBranch(label.to_string()),
                    })
                } else {
                    Err(message)
                }
            }
        }
    }

    fn weight(&self) -> f32 {
        1.0
    }
}

fn validate_and_render_sql(bundle: &SurfaceBundle, spec: &SqlAuthorSpec) -> Result<String, String> {
    let object = bundle
        .find_object(spec.target_object.as_str())
        .ok_or_else(|| {
            format!(
                "target object `{}` is not exposed by the surface bundle",
                spec.target_object
            )
        })?;

    if spec.projection.is_empty() {
        return Err("projection must include at least one column".to_string());
    }
    if spec.projection.iter().any(|column| column == "*") {
        return Err("SELECT * is not allowed".to_string());
    }
    if spec.limit == 0 {
        return Err("limit must be greater than zero".to_string());
    }
    if spec.limit > bundle.policy.max_limit {
        return Err(format!(
            "limit {} exceeds max_limit {}",
            spec.limit, bundle.policy.max_limit
        ));
    }
    if bundle.policy.requires_filter_for(object.name.as_str()) && spec.filters.is_empty() {
        return Err(format!(
            "object `{}` requires at least one narrowing filter",
            object.name
        ));
    }

    let projection = spec
        .projection
        .iter()
        .map(|column| {
            object.find_column(column.as_str()).ok_or_else(|| {
                format!(
                    "projection column `{column}` is not exposed for `{}`",
                    object.name
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let filters = spec
        .filters
        .iter()
        .map(|filter| validate_filter(filter, object, bundle))
        .collect::<Result<Vec<_>, _>>()?;

    let order_by = spec
        .order_by
        .iter()
        .map(|term| validate_order_term(term, object))
        .collect::<Result<Vec<_>, _>>()?;

    let mut sql = format!(
        "SELECT {} FROM {}",
        projection
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        object.name
    );
    if !filters.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(filters.join(" AND ").as_str());
    }
    if !order_by.is_empty() {
        sql.push_str(" ORDER BY ");
        sql.push_str(order_by.join(", ").as_str());
    }
    sql.push_str(format!(" LIMIT {}", spec.limit).as_str());
    Ok(sql)
}

fn validate_filter(
    filter: &SqlFilter,
    object: &super::contract::SurfaceObject,
    bundle: &SurfaceBundle,
) -> Result<String, String> {
    let column = object.find_column(filter.column.as_str()).ok_or_else(|| {
        format!(
            "filter column `{}` is not exposed for `{}`",
            filter.column, object.name
        )
    })?;
    if !bundle.policy.allows_op(filter.op.as_str()) {
        return Err(format!("filter op `{}` is not allowed", filter.op));
    }

    match normalize_token(filter.op.as_str()).as_str() {
        "eq" => Ok(format!(
            "{} = {}",
            column.name,
            render_sql_value(column, filter.value.as_str())?
        )),
        "contains" => {
            if !is_text_like(column) {
                return Err(format!(
                    "filter op `contains` is only allowed on text columns; `{}` has type `{}`",
                    column.name, column.data_type
                ));
            }
            Ok(format!(
                "{} LIKE {}",
                column.name,
                quote_string_literal(format!("%{}%", filter.value).as_str())
            ))
        }
        _ => Err(format!("unsupported filter op `{}`", filter.op)),
    }
}

fn validate_order_term(
    term: &SqlOrderTerm,
    object: &super::contract::SurfaceObject,
) -> Result<String, String> {
    let column = object.find_column(term.column.as_str()).ok_or_else(|| {
        format!(
            "order_by column `{}` is not exposed for `{}`",
            term.column, object.name
        )
    })?;
    let direction = normalize_token(term.direction.as_str());
    match direction.as_str() {
        "asc" => Ok(format!("{} ASC", column.name)),
        "desc" => Ok(format!("{} DESC", column.name)),
        _ => Err(format!(
            "order_by direction `{}` must be `asc` or `desc`",
            term.direction
        )),
    }
}

fn render_sql_value(column: &SurfaceColumn, raw: &str) -> Result<String, String> {
    let data_type = normalize_token(column.data_type.as_str());
    if data_type.contains("bool") {
        return raw
            .parse::<bool>()
            .map(|value| value.to_string().to_ascii_uppercase())
            .map_err(|_| {
                format!(
                    "column `{}` expects a boolean value, received `{raw}`",
                    column.name
                )
            });
    }
    if data_type.contains("uint") {
        return raw
            .parse::<u64>()
            .map(|value| value.to_string())
            .map_err(|_| {
                format!(
                    "column `{}` expects an unsigned integer value, received `{raw}`",
                    column.name
                )
            });
    }
    if data_type.contains("int") {
        return raw
            .parse::<i64>()
            .map(|value| value.to_string())
            .map_err(|_| {
                format!(
                    "column `{}` expects an integer value, received `{raw}`",
                    column.name
                )
            });
    }
    if data_type.contains("float") || data_type.contains("double") || data_type.contains("decimal")
    {
        return raw
            .parse::<f64>()
            .map_err(|_| {
                format!(
                    "column `{}` expects a numeric value, received `{raw}`",
                    column.name
                )
            })
            .and_then(|value| {
                if value.is_finite() {
                    Ok(value.to_string())
                } else {
                    Err(format!(
                        "column `{}` expects a finite numeric value",
                        column.name
                    ))
                }
            });
    }
    Ok(quote_string_literal(raw))
}

fn quote_string_literal(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "''"))
}

fn is_text_like(column: &SurfaceColumn) -> bool {
    let data_type = normalize_token(column.data_type.as_str());
    data_type.contains("utf8") || data_type.contains("string") || data_type.contains("text")
}

fn normalize_token(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

fn branch_or_continue(label: Option<&str>) -> FlowInstruction {
    if let Some(label) = label {
        FlowInstruction::SelectBranch(label.to_string())
    } else {
        FlowInstruction::Continue
    }
}
