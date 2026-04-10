use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryFrom;
use std::sync::Arc;

use arrow::array::{
    Array, BooleanArray, Int32Array, Int64Array, LargeStringArray, StringArray, StringViewArray,
};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;

use super::transport::ParserSummaryRouteKind;
use super::types::{
    JuliaParserDocAttachment, JuliaParserDocTargetKind, JuliaParserFileSummary, JuliaParserImport,
    JuliaParserSourceSummary, JuliaParserSymbol, JuliaParserSymbolKind,
};

pub(crate) const JULIA_PARSER_SUMMARY_REQUEST_ID_COLUMN: &str = "request_id";
pub(crate) const JULIA_PARSER_SUMMARY_SOURCE_ID_COLUMN: &str = "source_id";
pub(crate) const JULIA_PARSER_SUMMARY_SOURCE_TEXT_COLUMN: &str = "source_text";

pub(crate) const JULIA_PARSER_SUMMARY_KIND_COLUMN: &str = "summary_kind";
pub(crate) const JULIA_PARSER_SUMMARY_BACKEND_COLUMN: &str = "backend";
pub(crate) const JULIA_PARSER_SUMMARY_SUCCESS_COLUMN: &str = "success";
pub(crate) const JULIA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN: &str = "primary_name";
pub(crate) const JULIA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN: &str = "error_message";
pub(crate) const JULIA_PARSER_SUMMARY_MODULE_NAME_COLUMN: &str = "module_name";
pub(crate) const JULIA_PARSER_SUMMARY_MODULE_KIND_COLUMN: &str = "module_kind";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_GROUP_COLUMN: &str = "item_group";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_NAME_COLUMN: &str = "item_name";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_KIND_COLUMN: &str = "item_kind";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_SIGNATURE_COLUMN: &str = "item_signature";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_TARGET_KIND_COLUMN: &str = "item_target_kind";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_TARGET_NAME_COLUMN: &str = "item_target_name";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_KIND_COLUMN: &str = "item_dependency_kind";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_FORM_COLUMN: &str = "item_dependency_form";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_TARGET_COLUMN: &str =
    "item_dependency_target";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_IS_RELATIVE_COLUMN: &str =
    "item_dependency_is_relative";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_RELATIVE_LEVEL_COLUMN: &str =
    "item_dependency_relative_level";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_LOCAL_NAME_COLUMN: &str =
    "item_dependency_local_name";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_PARENT_COLUMN: &str =
    "item_dependency_parent";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_MEMBER_COLUMN: &str =
    "item_dependency_member";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_ALIAS_COLUMN: &str = "item_dependency_alias";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_CONTENT_COLUMN: &str = "item_content";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_REEXPORTED_COLUMN: &str = "item_reexported";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_PATH_COLUMN: &str = "item_path";
pub(crate) const JULIA_PARSER_SUMMARY_ITEM_BINDING_KIND_COLUMN: &str = "item_binding_kind";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JuliaParserSummaryRequestRow {
    pub(crate) request_id: String,
    pub(crate) source_id: String,
    pub(crate) source_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JuliaParserSummaryResponseRow {
    pub(crate) request_id: String,
    pub(crate) source_id: String,
    pub(crate) summary_kind: String,
    pub(crate) backend: String,
    pub(crate) success: bool,
    pub(crate) primary_name: Option<String>,
    pub(crate) error_message: Option<String>,
    pub(crate) module_name: Option<String>,
    pub(crate) module_kind: Option<String>,
    pub(crate) item_group: Option<String>,
    pub(crate) item_name: Option<String>,
    pub(crate) item_kind: Option<String>,
    pub(crate) item_signature: Option<String>,
    pub(crate) item_target_kind: Option<String>,
    pub(crate) item_target_name: Option<String>,
    pub(crate) item_dependency_kind: Option<String>,
    pub(crate) item_dependency_form: Option<String>,
    pub(crate) item_dependency_target: Option<String>,
    pub(crate) item_dependency_is_relative: Option<bool>,
    pub(crate) item_dependency_relative_level: Option<i32>,
    pub(crate) item_dependency_local_name: Option<String>,
    pub(crate) item_dependency_parent: Option<String>,
    pub(crate) item_dependency_member: Option<String>,
    pub(crate) item_dependency_alias: Option<String>,
    pub(crate) item_content: Option<String>,
    pub(crate) item_reexported: Option<bool>,
    pub(crate) item_path: Option<String>,
    pub(crate) item_binding_kind: Option<String>,
}

pub(crate) fn build_julia_parser_summary_request_batch(
    rows: &[JuliaParserSummaryRequestRow],
) -> Result<RecordBatch, RepoIntelligenceError> {
    let batch = RecordBatch::try_new(
        julia_parser_summary_request_schema(),
        vec![
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.request_id.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.source_id.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.source_text.as_str())
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| parser_summary_request_error(error.to_string()))?;
    validate_julia_parser_summary_request_batches(std::slice::from_ref(&batch))?;
    Ok(batch)
}

pub(crate) fn validate_julia_parser_summary_request_batches(
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    for batch in batches {
        if batch.num_rows() == 0 {
            return Err(parser_summary_contract_error(
                "request",
                "parser-summary request batch must contain at least one row".to_string(),
            ));
        }

        let _request_id =
            required_utf8_values(batch, JULIA_PARSER_SUMMARY_REQUEST_ID_COLUMN, "request")?;
        let _source_id =
            required_utf8_values(batch, JULIA_PARSER_SUMMARY_SOURCE_ID_COLUMN, "request")?;
        let _source_text =
            required_utf8_values(batch, JULIA_PARSER_SUMMARY_SOURCE_TEXT_COLUMN, "request")?;
    }

    Ok(())
}

pub(crate) fn validate_julia_parser_summary_response_batches(
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    for batch in batches {
        if batch.num_rows() == 0 {
            return Err(parser_summary_contract_error(
                "response",
                "parser-summary response batch must contain at least one row".to_string(),
            ));
        }

        let _request_id =
            required_utf8_values(batch, JULIA_PARSER_SUMMARY_REQUEST_ID_COLUMN, "response")?;
        let _source_id =
            required_utf8_values(batch, JULIA_PARSER_SUMMARY_SOURCE_ID_COLUMN, "response")?;
        let _summary_kind =
            required_utf8_values(batch, JULIA_PARSER_SUMMARY_KIND_COLUMN, "response")?;
        let _backend =
            required_utf8_values(batch, JULIA_PARSER_SUMMARY_BACKEND_COLUMN, "response")?;
        let _success =
            required_bool_values(batch, JULIA_PARSER_SUMMARY_SUCCESS_COLUMN, "response")?;
        let _primary_name =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN, "response")?;
        let _error_message =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN, "response")?;
        let _module_name =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_MODULE_NAME_COLUMN, "response")?;
        let _module_kind =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_MODULE_KIND_COLUMN, "response")?;
        let _item_group =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ITEM_GROUP_COLUMN, "response")?;
        let _item_name =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ITEM_NAME_COLUMN, "response")?;
        let _item_kind =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ITEM_KIND_COLUMN, "response")?;
        let _item_signature = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_SIGNATURE_COLUMN,
            "response",
        )?;
        let _item_target_kind = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_TARGET_KIND_COLUMN,
            "response",
        )?;
        let _item_target_name = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_TARGET_NAME_COLUMN,
            "response",
        )?;
        let _item_dependency_kind = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_KIND_COLUMN,
            "response",
        )?;
        let _item_dependency_form = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_FORM_COLUMN,
            "response",
        )?;
        let _item_dependency_target = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_TARGET_COLUMN,
            "response",
        )?;
        let _item_dependency_is_relative = optional_bool_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_IS_RELATIVE_COLUMN,
            "response",
        )?;
        let _item_dependency_relative_level = optional_int32_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_RELATIVE_LEVEL_COLUMN,
            "response",
        )?;
        let _item_dependency_local_name = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_LOCAL_NAME_COLUMN,
            "response",
        )?;
        let _item_dependency_parent = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_PARENT_COLUMN,
            "response",
        )?;
        let _item_dependency_member = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_MEMBER_COLUMN,
            "response",
        )?;
        let _item_dependency_alias = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_ALIAS_COLUMN,
            "response",
        )?;
        let _item_content =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ITEM_CONTENT_COLUMN, "response")?;
        let _item_reexported = optional_bool_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_REEXPORTED_COLUMN,
            "response",
        )?;
        let _item_path =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ITEM_PATH_COLUMN, "response")?;
        let _item_binding_kind = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_BINDING_KIND_COLUMN,
            "response",
        )?;
    }

    Ok(())
}

pub(crate) fn decode_julia_parser_summary_response_rows(
    batches: &[RecordBatch],
) -> Result<Vec<JuliaParserSummaryResponseRow>, RepoIntelligenceError> {
    validate_julia_parser_summary_response_batches(batches)?;
    let mut rows = Vec::new();

    for batch in batches {
        let request_id =
            required_utf8_values(batch, JULIA_PARSER_SUMMARY_REQUEST_ID_COLUMN, "response")?;
        let source_id =
            required_utf8_values(batch, JULIA_PARSER_SUMMARY_SOURCE_ID_COLUMN, "response")?;
        let summary_kind =
            required_utf8_values(batch, JULIA_PARSER_SUMMARY_KIND_COLUMN, "response")?;
        let backend = required_utf8_values(batch, JULIA_PARSER_SUMMARY_BACKEND_COLUMN, "response")?;
        let success = required_bool_values(batch, JULIA_PARSER_SUMMARY_SUCCESS_COLUMN, "response")?;
        let primary_name =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN, "response")?;
        let error_message =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN, "response")?;
        let module_name =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_MODULE_NAME_COLUMN, "response")?;
        let module_kind =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_MODULE_KIND_COLUMN, "response")?;
        let item_group =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ITEM_GROUP_COLUMN, "response")?;
        let item_name =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ITEM_NAME_COLUMN, "response")?;
        let item_kind =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ITEM_KIND_COLUMN, "response")?;
        let item_signature = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_SIGNATURE_COLUMN,
            "response",
        )?;
        let item_target_kind = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_TARGET_KIND_COLUMN,
            "response",
        )?;
        let item_target_name = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_TARGET_NAME_COLUMN,
            "response",
        )?;
        let item_dependency_kind = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_KIND_COLUMN,
            "response",
        )?;
        let item_dependency_form = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_FORM_COLUMN,
            "response",
        )?;
        let item_dependency_target = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_TARGET_COLUMN,
            "response",
        )?;
        let item_dependency_is_relative = optional_bool_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_IS_RELATIVE_COLUMN,
            "response",
        )?;
        let item_dependency_relative_level = optional_int32_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_RELATIVE_LEVEL_COLUMN,
            "response",
        )?;
        let item_dependency_local_name = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_LOCAL_NAME_COLUMN,
            "response",
        )?;
        let item_dependency_parent = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_PARENT_COLUMN,
            "response",
        )?;
        let item_dependency_member = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_MEMBER_COLUMN,
            "response",
        )?;
        let item_dependency_alias = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_DEPENDENCY_ALIAS_COLUMN,
            "response",
        )?;
        let item_content =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ITEM_CONTENT_COLUMN, "response")?;
        let item_reexported = optional_bool_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_REEXPORTED_COLUMN,
            "response",
        )?;
        let item_path =
            optional_utf8_values(batch, JULIA_PARSER_SUMMARY_ITEM_PATH_COLUMN, "response")?;
        let item_binding_kind = optional_utf8_values(
            batch,
            JULIA_PARSER_SUMMARY_ITEM_BINDING_KIND_COLUMN,
            "response",
        )?;

        for row_index in 0..batch.num_rows() {
            rows.push(JuliaParserSummaryResponseRow {
                request_id: request_id[row_index].clone(),
                source_id: source_id[row_index].clone(),
                summary_kind: summary_kind[row_index].clone(),
                backend: backend[row_index].clone(),
                success: success[row_index],
                primary_name: primary_name[row_index].clone(),
                error_message: error_message[row_index].clone(),
                module_name: module_name[row_index].clone(),
                module_kind: module_kind[row_index].clone(),
                item_group: item_group[row_index].clone(),
                item_name: item_name[row_index].clone(),
                item_kind: item_kind[row_index].clone(),
                item_signature: item_signature[row_index].clone(),
                item_target_kind: item_target_kind[row_index].clone(),
                item_target_name: item_target_name[row_index].clone(),
                item_dependency_kind: item_dependency_kind[row_index].clone(),
                item_dependency_form: item_dependency_form[row_index].clone(),
                item_dependency_target: item_dependency_target[row_index].clone(),
                item_dependency_is_relative: item_dependency_is_relative[row_index],
                item_dependency_relative_level: item_dependency_relative_level[row_index],
                item_dependency_local_name: item_dependency_local_name[row_index].clone(),
                item_dependency_parent: item_dependency_parent[row_index].clone(),
                item_dependency_member: item_dependency_member[row_index].clone(),
                item_dependency_alias: item_dependency_alias[row_index].clone(),
                item_content: item_content[row_index].clone(),
                item_reexported: item_reexported[row_index],
                item_path: item_path[row_index].clone(),
                item_binding_kind: item_binding_kind[row_index].clone(),
            });
        }
    }

    Ok(rows)
}

pub(crate) fn decode_julia_parser_file_summary(
    route_kind: ParserSummaryRouteKind,
    rows: &[JuliaParserSummaryResponseRow],
) -> Result<JuliaParserFileSummary, RepoIntelligenceError> {
    let context = response_context(route_kind, rows)?;

    let exports = rows
        .iter()
        .filter(|row| row.item_group.as_deref() == Some("export"))
        .filter_map(|row| row.item_name.as_ref())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut import_map = BTreeMap::<String, JuliaParserImport>::new();
    for row in rows
        .iter()
        .filter(|row| row.item_group.as_deref() == Some("import"))
    {
        let Some(module) = row
            .item_dependency_target
            .as_ref()
            .or(row.item_name.as_ref())
            .cloned()
        else {
            continue;
        };
        let candidate = JuliaParserImport {
            module: module.clone(),
            reexported: row.item_reexported.unwrap_or(false),
            dependency_kind: row
                .item_dependency_kind
                .clone()
                .unwrap_or_else(|| "import".to_string()),
            dependency_form: row
                .item_dependency_form
                .clone()
                .unwrap_or_else(|| "path".to_string()),
            dependency_is_relative: row.item_dependency_is_relative.unwrap_or(false),
            dependency_relative_level: row.item_dependency_relative_level.unwrap_or(0),
            dependency_local_name: row.item_dependency_local_name.clone(),
            dependency_parent: row.item_dependency_parent.clone(),
            dependency_member: row.item_dependency_member.clone(),
            dependency_alias: row.item_dependency_alias.clone(),
        };
        match import_map.get(&module) {
            Some(existing) if existing.reexported || !candidate.reexported => {}
            _ => {
                import_map.insert(module, candidate);
            }
        }
    }

    let includes = rows
        .iter()
        .filter(|row| row.item_group.as_deref() == Some("include"))
        .filter_map(|row| {
            row.item_path
                .as_ref()
                .or(row.item_dependency_target.as_ref())
                .cloned()
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let mut symbol_map = BTreeMap::<String, JuliaParserSymbol>::new();
    for row in rows
        .iter()
        .filter(|row| row.item_group.as_deref() == Some("symbol"))
    {
        let Some(name) = row.item_name.as_ref().cloned() else {
            continue;
        };
        let symbol = JuliaParserSymbol {
            name: name.clone(),
            kind: map_symbol_kind(row.item_kind.as_deref(), row.item_binding_kind.as_deref()),
            signature: row.item_signature.clone(),
        };
        match symbol_map.get(&name) {
            Some(existing) if symbol_kind_rank(existing.kind) > symbol_kind_rank(symbol.kind) => {}
            _ => {
                symbol_map.insert(name, symbol);
            }
        }
    }

    let docstrings = rows
        .iter()
        .filter(|row| row.item_group.as_deref() == Some("docstring"))
        .filter_map(|row| {
            Some(JuliaParserDocAttachment {
                target_name: row
                    .item_target_name
                    .as_ref()
                    .or(row.item_name.as_ref())?
                    .clone(),
                target_kind: map_doc_target_kind(row.item_target_kind.as_deref()),
                content: row.item_content.as_ref()?.clone(),
            })
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(JuliaParserFileSummary {
        module_name: context.module_name.clone(),
        exports,
        imports: import_map.into_values().collect(),
        symbols: symbol_map.into_values().collect(),
        docstrings,
        includes,
    })
}

pub(crate) fn decode_julia_parser_root_summary(
    route_kind: ParserSummaryRouteKind,
    rows: &[JuliaParserSummaryResponseRow],
) -> Result<JuliaParserSourceSummary, RepoIntelligenceError> {
    let summary = decode_julia_parser_file_summary(route_kind, rows)?;
    let Some(module_name) = summary.module_name else {
        return Err(parser_summary_contract_error(
            "response",
            format!(
                "Julia parser-summary route `{}` did not return `module_name`",
                route_kind.summary_kind(),
            ),
        ));
    };
    Ok(JuliaParserSourceSummary {
        module_name,
        exports: summary.exports,
        imports: summary.imports,
        symbols: summary.symbols,
        docstrings: summary.docstrings,
        includes: summary.includes,
    })
}

fn response_context<'a>(
    route_kind: ParserSummaryRouteKind,
    rows: &'a [JuliaParserSummaryResponseRow],
) -> Result<&'a JuliaParserSummaryResponseRow, RepoIntelligenceError> {
    let Some(first) = rows.first() else {
        return Err(parser_summary_contract_error(
            "response",
            "parser-summary response rows must contain at least one row".to_string(),
        ));
    };
    let expected_summary_kind = route_kind.summary_kind();
    for row in rows {
        if row.summary_kind != expected_summary_kind {
            return Err(parser_summary_contract_error(
                "response",
                format!(
                    "parser-summary response row for request `{}` returned summary kind `{}` but expected `{expected_summary_kind}`",
                    row.request_id, row.summary_kind,
                ),
            ));
        }
        if row.request_id != first.request_id {
            return Err(parser_summary_contract_error(
                "response",
                "parser-summary response rows must not mix request ids".to_string(),
            ));
        }
        if row.source_id != first.source_id {
            return Err(parser_summary_contract_error(
                "response",
                "parser-summary response rows must not mix source ids".to_string(),
            ));
        }
        if row.success != first.success {
            return Err(parser_summary_contract_error(
                "response",
                "parser-summary response rows must agree on `success`".to_string(),
            ));
        }
    }
    if !first.success {
        return Err(RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "Julia parser-summary route `{}` failed for source `{}`: {}",
                route_kind.summary_kind(),
                first.source_id,
                first
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "unknown parser error".to_string()),
            ),
        });
    }
    Ok(first)
}

fn map_symbol_kind(kind: Option<&str>, binding_kind: Option<&str>) -> JuliaParserSymbolKind {
    match (kind, binding_kind) {
        (Some("function"), _) => JuliaParserSymbolKind::Function,
        (Some("type"), _) => JuliaParserSymbolKind::Type,
        (Some("binding"), Some("const")) => JuliaParserSymbolKind::Constant,
        _ => JuliaParserSymbolKind::Other,
    }
}

fn symbol_kind_rank(kind: JuliaParserSymbolKind) -> u8 {
    match kind {
        JuliaParserSymbolKind::Type => 3,
        JuliaParserSymbolKind::Constant => 2,
        JuliaParserSymbolKind::Function => 1,
        JuliaParserSymbolKind::Other => 0,
    }
}

fn map_doc_target_kind(target_kind: Option<&str>) -> JuliaParserDocTargetKind {
    match target_kind {
        Some("module") => JuliaParserDocTargetKind::Module,
        _ => JuliaParserDocTargetKind::Symbol,
    }
}

fn julia_parser_summary_request_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new(
            JULIA_PARSER_SUMMARY_REQUEST_ID_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(JULIA_PARSER_SUMMARY_SOURCE_ID_COLUMN, DataType::Utf8, false),
        Field::new(
            JULIA_PARSER_SUMMARY_SOURCE_TEXT_COLUMN,
            DataType::Utf8,
            false,
        ),
    ]))
}

fn column_by_name<'a>(
    batch: &'a RecordBatch,
    field_name: &str,
    contract_side: &str,
) -> Result<&'a dyn Array, RepoIntelligenceError> {
    Ok(batch
        .column_by_name(field_name)
        .ok_or_else(|| {
            parser_summary_contract_error(
                contract_side,
                format!("missing parser-summary column `{field_name}`"),
            )
        })?
        .as_ref())
}

fn required_utf8_values(
    batch: &RecordBatch,
    field_name: &str,
    contract_side: &str,
) -> Result<Vec<String>, RepoIntelligenceError> {
    let values = optional_utf8_values(batch, field_name, contract_side)?;
    values
        .into_iter()
        .enumerate()
        .map(|(row_index, value)| {
            let Some(value) = value else {
                return Err(parser_summary_contract_error(
                    contract_side,
                    format!(
                        "parser-summary column `{field_name}` must not contain null values; row {row_index} is null"
                    ),
                ));
            };
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(parser_summary_contract_error(
                    contract_side,
                    format!(
                        "parser-summary column `{field_name}` must not contain blank values; row {row_index} is blank"
                    ),
                ));
            }
            Ok(trimmed.to_string())
        })
        .collect()
}

fn optional_utf8_values(
    batch: &RecordBatch,
    field_name: &str,
    contract_side: &str,
) -> Result<Vec<Option<String>>, RepoIntelligenceError> {
    let column = column_by_name(batch, field_name, contract_side)?;
    match column.data_type() {
        DataType::Utf8 => {
            let array = column
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| {
                    parser_summary_contract_error(
                        contract_side,
                        format!("parser-summary column `{field_name}` must decode as Utf8"),
                    )
                })?;
            Ok((0..array.len())
                .map(|row_index| {
                    if array.is_null(row_index) {
                        None
                    } else {
                        let value = array.value(row_index).trim();
                        if value.is_empty() {
                            None
                        } else {
                            Some(value.to_string())
                        }
                    }
                })
                .collect())
        }
        DataType::LargeUtf8 => {
            let array = column
                .as_any()
                .downcast_ref::<LargeStringArray>()
                .ok_or_else(|| {
                    parser_summary_contract_error(
                        contract_side,
                        format!("parser-summary column `{field_name}` must decode as LargeUtf8"),
                    )
                })?;
            Ok((0..array.len())
                .map(|row_index| {
                    if array.is_null(row_index) {
                        None
                    } else {
                        let value = array.value(row_index).trim();
                        if value.is_empty() {
                            None
                        } else {
                            Some(value.to_string())
                        }
                    }
                })
                .collect())
        }
        DataType::Utf8View => {
            let array = column
                .as_any()
                .downcast_ref::<StringViewArray>()
                .ok_or_else(|| {
                    parser_summary_contract_error(
                        contract_side,
                        format!("parser-summary column `{field_name}` must decode as Utf8View"),
                    )
                })?;
            Ok((0..array.len())
                .map(|row_index| {
                    if array.is_null(row_index) {
                        None
                    } else {
                        let value = array.value(row_index).trim();
                        if value.is_empty() {
                            None
                        } else {
                            Some(value.to_string())
                        }
                    }
                })
                .collect())
        }
        DataType::Null => Ok(vec![None; column.len()]),
        _ => Err(parser_summary_contract_error(
            contract_side,
            format!(
                "parser-summary column `{field_name}` must decode as a nullable string-compatible Arrow column"
            ),
        )),
    }
}

fn required_bool_values(
    batch: &RecordBatch,
    field_name: &str,
    contract_side: &str,
) -> Result<Vec<bool>, RepoIntelligenceError> {
    let values = optional_bool_values(batch, field_name, contract_side)?;
    values
        .into_iter()
        .enumerate()
        .map(|(row_index, value)| {
            value.ok_or_else(|| {
                parser_summary_contract_error(
                    contract_side,
                    format!(
                        "parser-summary column `{field_name}` must not contain null values; row {row_index} is null"
                    ),
                )
            })
        })
        .collect()
}

fn optional_bool_values(
    batch: &RecordBatch,
    field_name: &str,
    contract_side: &str,
) -> Result<Vec<Option<bool>>, RepoIntelligenceError> {
    let column = column_by_name(batch, field_name, contract_side)?;
    match column.data_type() {
        DataType::Boolean => {
            let array = column
                .as_any()
                .downcast_ref::<BooleanArray>()
                .ok_or_else(|| {
                    parser_summary_contract_error(
                        contract_side,
                        format!("parser-summary column `{field_name}` must decode as Boolean"),
                    )
                })?;
            Ok((0..array.len())
                .map(|row_index| {
                    if array.is_null(row_index) {
                        None
                    } else {
                        Some(array.value(row_index))
                    }
                })
                .collect())
        }
        DataType::Null => Ok(vec![None; column.len()]),
        _ => Err(parser_summary_contract_error(
            contract_side,
            format!(
                "parser-summary column `{field_name}` must decode as a nullable Boolean Arrow column"
            ),
        )),
    }
}

fn optional_int32_values(
    batch: &RecordBatch,
    field_name: &str,
    contract_side: &str,
) -> Result<Vec<Option<i32>>, RepoIntelligenceError> {
    let column = column_by_name(batch, field_name, contract_side)?;
    match column.data_type() {
        DataType::Int32 => {
            let array = column
                .as_any()
                .downcast_ref::<Int32Array>()
                .ok_or_else(|| {
                    parser_summary_contract_error(
                        contract_side,
                        format!("parser-summary column `{field_name}` must decode as Int32"),
                    )
                })?;
            Ok((0..array.len())
                .map(|row_index| {
                    if array.is_null(row_index) {
                        None
                    } else {
                        Some(array.value(row_index))
                    }
                })
                .collect())
        }
        DataType::Int64 => {
            let array = column
                .as_any()
                .downcast_ref::<Int64Array>()
                .ok_or_else(|| {
                    parser_summary_contract_error(
                        contract_side,
                        format!("parser-summary column `{field_name}` must decode as Int64"),
                    )
                })?;
            (0..array.len())
                .map(|row_index| {
                    if array.is_null(row_index) {
                        Ok(None)
                    } else {
                        i32::try_from(array.value(row_index))
                            .map(Some)
                            .map_err(|error| {
                                parser_summary_contract_error(
                                    contract_side,
                                    format!(
                                        "parser-summary column `{field_name}` row {row_index} cannot narrow Int64 to Int32: {error}"
                                    ),
                                )
                            })
                    }
                })
                .collect()
        }
        DataType::Null => Ok(vec![None; column.len()]),
        _ => Err(parser_summary_contract_error(
            contract_side,
            format!(
                "parser-summary column `{field_name}` must decode as a nullable Int32-compatible Arrow column"
            ),
        )),
    }
}

fn parser_summary_request_error(message: String) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!("invalid Julia parser-summary request batch: {message}"),
    }
}

fn parser_summary_contract_error(contract_side: &str, message: String) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!("Julia parser-summary {contract_side} contract violation: {message}"),
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/parser_summary/contract.rs"]
mod tests;
