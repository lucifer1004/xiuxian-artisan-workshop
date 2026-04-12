use serde_json::Value;
use xiuxian_qianji::WorkflowReport;

const MAX_ROW_PREVIEW: usize = 3;

pub(super) fn render_search_report(
    request: &str,
    project_root: &str,
    report: &WorkflowReport,
) -> String {
    let final_context = &report.final_context;
    if let Some(payload) = first_value(final_context, &["query_result_2", "query_result_1"]) {
        return render_success_report(request, project_root, report, payload);
    }
    render_failure_report(request, project_root, report)
}

fn render_success_report(
    request: &str,
    project_root: &str,
    report: &WorkflowReport,
    payload: &Value,
) -> String {
    let sql = first_string(
        &report.final_context,
        &["validated_sql_2", "validated_sql_1"],
    );
    let row_count = payload
        .get("metadata")
        .and_then(|value| value.get("resultRowCount"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut lines = vec![
        "## Wendao Search".to_string(),
        "- Status: success".to_string(),
        format!("- Request: {request}"),
        format!("- Project Root: {project_root}"),
        format!("- Workflow: {}", report.manifest_name),
        format!("- Duration: {} ms", report.duration_ms),
        format!("- Rows: {row_count}"),
    ];

    if let Some(sql) = sql {
        lines.push(String::new());
        lines.push("### SQL".to_string());
        lines.push(format!("```sql\n{sql}\n```"));
    }

    let row_preview = render_row_preview(payload, MAX_ROW_PREVIEW);
    if !row_preview.is_empty() {
        lines.push(String::new());
        lines.push("### Rows".to_string());
        lines.extend(row_preview);
    }

    lines.join("\n")
}

fn render_failure_report(request: &str, project_root: &str, report: &WorkflowReport) -> String {
    let reason = first_string(
        &report.final_context,
        &[
            "execution_error_2",
            "validation_error_2",
            "execution_error_1",
            "validation_error_1",
        ],
    )
    .unwrap_or("workflow completed without a SQL result");
    let sql = first_string(
        &report.final_context,
        &["validated_sql_2", "validated_sql_1"],
    );

    let mut lines = vec![
        "## Wendao Search".to_string(),
        "- Status: failed".to_string(),
        format!("- Request: {request}"),
        format!("- Project Root: {project_root}"),
        format!("- Workflow: {}", report.manifest_name),
        format!("- Duration: {} ms", report.duration_ms),
        format!("- Reason: {reason}"),
    ];
    if let Some(sql) = sql {
        lines.push(String::new());
        lines.push("### Last SQL".to_string());
        lines.push(format!("```sql\n{sql}\n```"));
    }

    lines.join("\n")
}

fn render_row_preview(payload: &Value, max_rows: usize) -> Vec<String> {
    let Some(batch) = payload
        .get("batches")
        .and_then(Value::as_array)
        .and_then(|batches| batches.first())
    else {
        return Vec::new();
    };
    let Some(rows) = batch.get("rows").and_then(Value::as_array) else {
        return Vec::new();
    };
    let column_names = batch
        .get("columns")
        .and_then(Value::as_array)
        .map(|columns| {
            columns
                .iter()
                .filter_map(|column| {
                    column
                        .get("name")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    rows.iter()
        .take(max_rows)
        .enumerate()
        .map(|(index, row)| {
            let rendered = if let Some(object) = row.as_object() {
                let ordered = if column_names.is_empty() {
                    let mut keys = object.keys().cloned().collect::<Vec<_>>();
                    keys.sort_unstable();
                    keys
                } else {
                    column_names.clone()
                };
                ordered
                    .into_iter()
                    .filter_map(|key| {
                        object
                            .get(key.as_str())
                            .map(|value| format!("{key}={}", render_value(value)))
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            } else {
                render_value(row)
            };
            format!("{}. {}", index + 1, rendered)
        })
        .collect()
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(raw) => raw.to_string(),
        Value::Number(raw) => raw.to_string(),
        Value::String(raw) => raw.clone(),
        _ => value.to_string(),
    }
}

fn first_string<'a>(context: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        context
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn first_value<'a>(context: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| context.get(*key))
}
