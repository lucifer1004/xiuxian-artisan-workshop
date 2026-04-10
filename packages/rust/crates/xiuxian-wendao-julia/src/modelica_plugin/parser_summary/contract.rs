use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryFrom;
use std::sync::Arc;

use arrow::array::{
    Array, BooleanArray, Int32Array, Int64Array, LargeStringArray, StringArray, StringViewArray,
};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use xiuxian_wendao_core::repo_intelligence::{ImportKind, RepoIntelligenceError, RepoSymbolKind};

use super::super::types::{ParsedDeclaration, ParsedImport};
use super::transport::ParserSummaryRouteKind;
use super::types::ModelicaParserFileSummary;

pub(crate) const MODELICA_PARSER_SUMMARY_REQUEST_ID_COLUMN: &str = "request_id";
pub(crate) const MODELICA_PARSER_SUMMARY_SOURCE_ID_COLUMN: &str = "source_id";
pub(crate) const MODELICA_PARSER_SUMMARY_SOURCE_TEXT_COLUMN: &str = "source_text";
pub(crate) const MODELICA_PARSER_SUMMARY_KIND_COLUMN: &str = "summary_kind";
pub(crate) const MODELICA_PARSER_SUMMARY_BACKEND_COLUMN: &str = "backend";
pub(crate) const MODELICA_PARSER_SUMMARY_SUCCESS_COLUMN: &str = "success";
pub(crate) const MODELICA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN: &str = "primary_name";
pub(crate) const MODELICA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN: &str = "error_message";
pub(crate) const MODELICA_PARSER_SUMMARY_CLASS_NAME_COLUMN: &str = "class_name";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_GROUP_COLUMN: &str = "item_group";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_NAME_COLUMN: &str = "item_name";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_KIND_COLUMN: &str = "item_kind";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_SIGNATURE_COLUMN: &str = "item_signature";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_FORM_COLUMN: &str = "item_dependency_form";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_TARGET_COLUMN: &str =
    "item_dependency_target";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_ALIAS_COLUMN: &str =
    "item_dependency_alias";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_LOCAL_NAME_COLUMN: &str =
    "item_dependency_local_name";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_TEXT_COLUMN: &str = "item_text";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_LINE_START_COLUMN: &str = "item_line_start";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_LINE_END_COLUMN: &str = "item_line_end";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_OWNER_NAME_COLUMN: &str = "item_owner_name";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_OWNER_PATH_COLUMN: &str = "item_owner_path";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelicaParserSummaryRequestRow {
    pub(crate) request_id: String,
    pub(crate) source_id: String,
    pub(crate) source_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelicaParserSummaryResponseRow {
    pub(crate) request_id: String,
    pub(crate) source_id: String,
    pub(crate) summary_kind: String,
    pub(crate) backend: String,
    pub(crate) success: bool,
    pub(crate) primary_name: Option<String>,
    pub(crate) error_message: Option<String>,
    pub(crate) class_name: Option<String>,
    pub(crate) item_group: Option<String>,
    pub(crate) item_name: Option<String>,
    pub(crate) item_kind: Option<String>,
    pub(crate) item_signature: Option<String>,
    pub(crate) item_dependency_form: Option<String>,
    pub(crate) item_dependency_target: Option<String>,
    pub(crate) item_dependency_alias: Option<String>,
    pub(crate) item_dependency_local_name: Option<String>,
    pub(crate) item_text: Option<String>,
    pub(crate) item_line_start: Option<i64>,
    pub(crate) item_line_end: Option<i64>,
    pub(crate) item_owner_name: Option<String>,
    pub(crate) item_owner_path: Option<String>,
}

pub(crate) fn build_modelica_parser_summary_request_batch(
    rows: &[ModelicaParserSummaryRequestRow],
) -> Result<RecordBatch, RepoIntelligenceError> {
    let batch = RecordBatch::try_new(
        modelica_parser_summary_request_schema(),
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
    validate_modelica_parser_summary_request_batches(std::slice::from_ref(&batch))?;
    Ok(batch)
}

pub(crate) fn validate_modelica_parser_summary_request_batches(
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    for batch in batches {
        if batch.num_rows() == 0 {
            return Err(parser_summary_contract_error(
                "request",
                "Modelica parser-summary request batch must contain at least one row".to_string(),
            ));
        }
        let _request_id =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_REQUEST_ID_COLUMN, "request")?;
        let _source_id =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_SOURCE_ID_COLUMN, "request")?;
        let _source_text =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_SOURCE_TEXT_COLUMN, "request")?;
    }
    Ok(())
}

pub(crate) fn validate_modelica_parser_summary_response_batches(
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    for batch in batches {
        if batch.num_rows() == 0 {
            return Err(parser_summary_contract_error(
                "response",
                "Modelica parser-summary response batch must contain at least one row".to_string(),
            ));
        }
        let _request_id =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_REQUEST_ID_COLUMN, "response")?;
        let _source_id =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_SOURCE_ID_COLUMN, "response")?;
        let _summary_kind =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_KIND_COLUMN, "response")?;
        let _backend =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_BACKEND_COLUMN, "response")?;
        let _success =
            required_bool_values(batch, MODELICA_PARSER_SUMMARY_SUCCESS_COLUMN, "response")?;
        let _primary_name = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN,
            "response",
        )?;
        let _error_message = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN,
            "response",
        )?;
        let _class_name =
            optional_utf8_values(batch, MODELICA_PARSER_SUMMARY_CLASS_NAME_COLUMN, "response")?;
        let _item_group =
            optional_utf8_values(batch, MODELICA_PARSER_SUMMARY_ITEM_GROUP_COLUMN, "response")?;
        let _item_name =
            optional_utf8_values(batch, MODELICA_PARSER_SUMMARY_ITEM_NAME_COLUMN, "response")?;
        let _item_kind =
            optional_utf8_values(batch, MODELICA_PARSER_SUMMARY_ITEM_KIND_COLUMN, "response")?;
        let _item_signature = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_SIGNATURE_COLUMN,
            "response",
        )?;
        let _item_dependency_form = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_FORM_COLUMN,
            "response",
        )?;
        let _item_dependency_target = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_TARGET_COLUMN,
            "response",
        )?;
        let _item_dependency_alias = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_ALIAS_COLUMN,
            "response",
        )?;
        let _item_dependency_local_name = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_LOCAL_NAME_COLUMN,
            "response",
        )?;
        let _item_text =
            optional_utf8_values(batch, MODELICA_PARSER_SUMMARY_ITEM_TEXT_COLUMN, "response")?;
        let _item_line_start = optional_int_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_LINE_START_COLUMN,
            "response",
        )?;
        let _item_line_end = optional_int_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_LINE_END_COLUMN,
            "response",
        )?;
        let _item_owner_name = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_OWNER_NAME_COLUMN,
            "response",
        )?;
        let _item_owner_path = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_OWNER_PATH_COLUMN,
            "response",
        )?;
    }

    Ok(())
}

pub(crate) fn decode_modelica_parser_summary_response_rows(
    batches: &[RecordBatch],
) -> Result<Vec<ModelicaParserSummaryResponseRow>, RepoIntelligenceError> {
    validate_modelica_parser_summary_response_batches(batches)?;
    let mut rows = Vec::new();

    for batch in batches {
        let request_id =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_REQUEST_ID_COLUMN, "response")?;
        let source_id =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_SOURCE_ID_COLUMN, "response")?;
        let summary_kind =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_KIND_COLUMN, "response")?;
        let backend =
            required_utf8_values(batch, MODELICA_PARSER_SUMMARY_BACKEND_COLUMN, "response")?;
        let success =
            required_bool_values(batch, MODELICA_PARSER_SUMMARY_SUCCESS_COLUMN, "response")?;
        let primary_name = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN,
            "response",
        )?;
        let error_message = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN,
            "response",
        )?;
        let class_name =
            optional_utf8_values(batch, MODELICA_PARSER_SUMMARY_CLASS_NAME_COLUMN, "response")?;
        let item_group =
            optional_utf8_values(batch, MODELICA_PARSER_SUMMARY_ITEM_GROUP_COLUMN, "response")?;
        let item_name =
            optional_utf8_values(batch, MODELICA_PARSER_SUMMARY_ITEM_NAME_COLUMN, "response")?;
        let item_kind =
            optional_utf8_values(batch, MODELICA_PARSER_SUMMARY_ITEM_KIND_COLUMN, "response")?;
        let item_signature = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_SIGNATURE_COLUMN,
            "response",
        )?;
        let item_dependency_form = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_FORM_COLUMN,
            "response",
        )?;
        let item_dependency_target = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_TARGET_COLUMN,
            "response",
        )?;
        let item_dependency_alias = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_ALIAS_COLUMN,
            "response",
        )?;
        let item_dependency_local_name = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_LOCAL_NAME_COLUMN,
            "response",
        )?;
        let item_text =
            optional_utf8_values(batch, MODELICA_PARSER_SUMMARY_ITEM_TEXT_COLUMN, "response")?;
        let item_line_start = optional_int_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_LINE_START_COLUMN,
            "response",
        )?;
        let item_line_end = optional_int_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_LINE_END_COLUMN,
            "response",
        )?;
        let item_owner_name = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_OWNER_NAME_COLUMN,
            "response",
        )?;
        let item_owner_path = optional_utf8_values(
            batch,
            MODELICA_PARSER_SUMMARY_ITEM_OWNER_PATH_COLUMN,
            "response",
        )?;

        for index in 0..batch.num_rows() {
            rows.push(ModelicaParserSummaryResponseRow {
                request_id: request_id[index].clone(),
                source_id: source_id[index].clone(),
                summary_kind: summary_kind[index].clone(),
                backend: backend[index].clone(),
                success: success[index],
                primary_name: primary_name[index].clone(),
                error_message: error_message[index].clone(),
                class_name: class_name[index].clone(),
                item_group: item_group[index].clone(),
                item_name: item_name[index].clone(),
                item_kind: item_kind[index].clone(),
                item_signature: item_signature[index].clone(),
                item_dependency_form: item_dependency_form[index].clone(),
                item_dependency_target: item_dependency_target[index].clone(),
                item_dependency_alias: item_dependency_alias[index].clone(),
                item_dependency_local_name: item_dependency_local_name[index].clone(),
                item_text: item_text[index].clone(),
                item_line_start: item_line_start[index],
                item_line_end: item_line_end[index],
                item_owner_name: item_owner_name[index].clone(),
                item_owner_path: item_owner_path[index].clone(),
            });
        }
    }

    Ok(rows)
}

pub(crate) fn decode_modelica_parser_file_summary(
    route_kind: ParserSummaryRouteKind,
    rows: &[ModelicaParserSummaryResponseRow],
) -> Result<ModelicaParserFileSummary, RepoIntelligenceError> {
    if rows.is_empty() {
        return Err(parser_summary_contract_error(
            "response",
            format!(
                "Modelica parser-summary response for route `{}` did not contain any rows",
                route_kind.route(),
            ),
        ));
    }

    for row in rows {
        if row.summary_kind != "modelica_file_summary" {
            return Err(parser_summary_contract_error(
                "response",
                format!(
                    "Modelica parser-summary route `{}` returned unexpected summary kind `{}`",
                    route_kind.route(),
                    row.summary_kind,
                ),
            ));
        }
        if !row.success {
            return Err(RepoIntelligenceError::AnalysisFailed {
                message: row
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Modelica parser-summary request failed".to_string()),
            });
        }
    }

    let class_name = rows.iter().find_map(|row| row.class_name.clone());
    let mut equations_by_owner = BTreeMap::<String, Vec<String>>::new();
    for row in rows
        .iter()
        .filter(|row| row.item_group.as_deref() == Some("equation"))
    {
        let Some(text) = row.item_text.clone() else {
            continue;
        };
        let owner_key = row
            .item_owner_path
            .clone()
            .or_else(|| row.item_owner_name.clone())
            .unwrap_or_default();
        equations_by_owner.entry(owner_key).or_default().push(text);
    }

    let mut imports = Vec::new();
    let mut seen_imports = BTreeSet::new();
    for row in rows
        .iter()
        .filter(|row| row.item_group.as_deref() == Some("import"))
    {
        let name = row
            .item_dependency_target
            .clone()
            .or_else(|| row.item_name.clone())
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let alias = row.item_dependency_alias.clone();
        let key = (
            name.clone(),
            alias.clone().unwrap_or_default(),
            row.item_dependency_form.clone().unwrap_or_default(),
        );
        if !seen_imports.insert(key) {
            continue;
        }
        imports.push(ParsedImport {
            name,
            alias,
            kind: modelica_import_kind(row.item_dependency_form.as_deref()),
            line_start: row
                .item_line_start
                .map(usize::try_from)
                .transpose()
                .map_err(|error| parser_summary_contract_error("response", error.to_string()))?,
        });
    }

    let mut declarations = Vec::new();
    for row in rows
        .iter()
        .filter(|row| row.item_group.as_deref() == Some("symbol"))
    {
        let Some(kind) = modelica_kind_to_repo_kind(row.item_kind.as_deref()) else {
            continue;
        };
        let name = row.item_name.clone().unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let owner_key = row
            .item_owner_path
            .clone()
            .or_else(|| row.item_owner_name.clone())
            .unwrap_or_default();
        declarations.push(ParsedDeclaration {
            name,
            kind,
            signature: row
                .item_signature
                .clone()
                .or_else(|| row.item_name.clone())
                .unwrap_or_default(),
            line_start: row
                .item_line_start
                .map(usize::try_from)
                .transpose()
                .map_err(|error| parser_summary_contract_error("response", error.to_string()))?,
            line_end: row
                .item_line_end
                .map(usize::try_from)
                .transpose()
                .map_err(|error| parser_summary_contract_error("response", error.to_string()))?,
            equations: equations_by_owner.remove(&owner_key).unwrap_or_default(),
        });
    }

    Ok(ModelicaParserFileSummary {
        class_name,
        imports,
        declarations,
    })
}

fn modelica_import_kind(form: Option<&str>) -> ImportKind {
    match form {
        Some("named_import" | "unqualified_import" | "group_import") => ImportKind::Module,
        _ => ImportKind::Symbol,
    }
}

fn modelica_kind_to_repo_kind(kind: Option<&str>) -> Option<RepoSymbolKind> {
    match kind {
        Some("function") => Some(RepoSymbolKind::Function),
        Some("model" | "record" | "block" | "connector" | "type") => Some(RepoSymbolKind::Type),
        Some("constant" | "parameter") => Some(RepoSymbolKind::Constant),
        _ => None,
    }
}

fn modelica_parser_summary_request_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new(
            MODELICA_PARSER_SUMMARY_REQUEST_ID_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            MODELICA_PARSER_SUMMARY_SOURCE_ID_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            MODELICA_PARSER_SUMMARY_SOURCE_TEXT_COLUMN,
            DataType::Utf8,
            false,
        ),
    ]))
}

fn required_utf8_values(
    batch: &RecordBatch,
    column: &str,
    stage: &str,
) -> Result<Vec<String>, RepoIntelligenceError> {
    let array = batch.column_by_name(column).ok_or_else(|| {
        parser_summary_contract_error(stage, format!("missing required column `{column}`"))
    })?;
    let values = utf8_values(array, column, stage)?;
    if values.iter().any(Option::is_none) {
        return Err(parser_summary_contract_error(
            stage,
            format!("required column `{column}` contains null rows"),
        ));
    }
    Ok(values
        .into_iter()
        .map(|value| value.unwrap_or_default())
        .collect::<Vec<_>>())
}

fn optional_utf8_values(
    batch: &RecordBatch,
    column: &str,
    stage: &str,
) -> Result<Vec<Option<String>>, RepoIntelligenceError> {
    match batch.column_by_name(column) {
        Some(array) => utf8_values(array, column, stage),
        None => Ok(vec![None; batch.num_rows()]),
    }
}

fn utf8_values(
    array: &Arc<dyn Array>,
    column: &str,
    stage: &str,
) -> Result<Vec<Option<String>>, RepoIntelligenceError> {
    if matches!(array.data_type(), DataType::Null) {
        return Ok(vec![None; array.len()]);
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return Ok((0..values.len())
            .map(|index| (!values.is_null(index)).then(|| values.value(index).to_string()))
            .collect());
    }
    if let Some(values) = array.as_any().downcast_ref::<LargeStringArray>() {
        return Ok((0..values.len())
            .map(|index| (!values.is_null(index)).then(|| values.value(index).to_string()))
            .collect());
    }
    if let Some(values) = array.as_any().downcast_ref::<StringViewArray>() {
        return Ok((0..values.len())
            .map(|index| (!values.is_null(index)).then(|| values.value(index).to_string()))
            .collect());
    }
    Err(parser_summary_contract_error(
        stage,
        format!(
            "column `{column}` expected Utf8-compatible values but found {:?}",
            array.data_type()
        ),
    ))
}

fn required_bool_values(
    batch: &RecordBatch,
    column: &str,
    stage: &str,
) -> Result<Vec<bool>, RepoIntelligenceError> {
    let array = batch.column_by_name(column).ok_or_else(|| {
        parser_summary_contract_error(stage, format!("missing required column `{column}`"))
    })?;
    let values = array
        .as_any()
        .downcast_ref::<BooleanArray>()
        .ok_or_else(|| {
            parser_summary_contract_error(
                stage,
                format!(
                    "column `{column}` expected Boolean values but found {:?}",
                    array.data_type()
                ),
            )
        })?;
    if (0..values.len()).any(|index| values.is_null(index)) {
        return Err(parser_summary_contract_error(
            stage,
            format!("required column `{column}` contains null rows"),
        ));
    }
    Ok((0..values.len()).map(|index| values.value(index)).collect())
}

fn optional_int_values(
    batch: &RecordBatch,
    column: &str,
    stage: &str,
) -> Result<Vec<Option<i64>>, RepoIntelligenceError> {
    let Some(array) = batch.column_by_name(column) else {
        return Ok(vec![None; batch.num_rows()]);
    };
    if matches!(array.data_type(), DataType::Null) {
        return Ok(vec![None; array.len()]);
    }
    if let Some(values) = array.as_any().downcast_ref::<Int32Array>() {
        return Ok((0..values.len())
            .map(|index| (!values.is_null(index)).then(|| i64::from(values.value(index))))
            .collect());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Ok((0..values.len())
            .map(|index| (!values.is_null(index)).then(|| values.value(index)))
            .collect());
    }
    Err(parser_summary_contract_error(
        stage,
        format!(
            "column `{column}` expected Int32 or Int64 values but found {:?}",
            array.data_type()
        ),
    ))
}

fn parser_summary_request_error(message: String) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!("failed to build Modelica parser-summary request batch: {message}"),
    }
}

fn parser_summary_contract_error(stage: &str, message: String) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!("invalid Modelica parser-summary {stage} contract: {message}"),
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/modelica_plugin/parser_summary_contract.rs"]
mod tests;
