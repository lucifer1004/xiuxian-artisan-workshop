use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryFrom;
use std::sync::Arc;

use arrow::array::{
    Array, BooleanArray, Int32Array, Int64Array, LargeStringArray, StringArray, StringViewArray,
};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use xiuxian_wendao_core::repo_intelligence::{ImportKind, RepoIntelligenceError, RepoSymbolKind};

use super::transport::ParserSummaryRouteKind;
use super::types::ModelicaParserFileSummary;
use crate::modelica_plugin::types::{ParsedDeclaration, ParsedImport};

pub(crate) const MODELICA_PARSER_SUMMARY_REQUEST_ID_COLUMN: &str = "request_id";
pub(crate) const MODELICA_PARSER_SUMMARY_SOURCE_ID_COLUMN: &str = "source_id";
pub(crate) const MODELICA_PARSER_SUMMARY_SOURCE_TEXT_COLUMN: &str = "source_text";
pub(crate) const MODELICA_PARSER_SUMMARY_KIND_COLUMN: &str = "summary_kind";
pub(crate) const MODELICA_PARSER_SUMMARY_BACKEND_COLUMN: &str = "backend";
pub(crate) const MODELICA_PARSER_SUMMARY_SUCCESS_COLUMN: &str = "success";
pub(crate) const MODELICA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN: &str = "primary_name";
pub(crate) const MODELICA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN: &str = "error_message";
pub(crate) const MODELICA_PARSER_SUMMARY_CLASS_NAME_COLUMN: &str = "class_name";
pub(crate) const MODELICA_PARSER_SUMMARY_RESTRICTION_COLUMN: &str = "restriction";
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
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_VISIBILITY_COLUMN: &str = "item_visibility";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_TYPE_NAME_COLUMN: &str = "item_type_name";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_VARIABILITY_COLUMN: &str = "item_variability";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_DIRECTION_COLUMN: &str = "item_direction";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_COMPONENT_KIND_COLUMN: &str = "item_component_kind";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_ARRAY_DIMENSIONS_COLUMN: &str =
    "item_array_dimensions";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_DEFAULT_VALUE_COLUMN: &str = "item_default_value";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_START_VALUE_COLUMN: &str = "item_start_value";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_MODIFIER_NAMES_COLUMN: &str = "item_modifier_names";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_UNIT_COLUMN: &str = "item_unit";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_CLASS_PATH_COLUMN: &str = "item_class_path";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_TOP_LEVEL_COLUMN: &str = "item_top_level";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_IS_PARTIAL_COLUMN: &str = "item_is_partial";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_IS_FINAL_COLUMN: &str = "item_is_final";
pub(crate) const MODELICA_PARSER_SUMMARY_ITEM_IS_ENCAPSULATED_COLUMN: &str = "item_is_encapsulated";

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
    pub(crate) restriction: Option<String>,
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
    pub(crate) item_visibility: Option<String>,
    pub(crate) item_type_name: Option<String>,
    pub(crate) item_variability: Option<String>,
    pub(crate) item_direction: Option<String>,
    pub(crate) item_component_kind: Option<String>,
    pub(crate) item_array_dimensions: Option<String>,
    pub(crate) item_default_value: Option<String>,
    pub(crate) item_start_value: Option<String>,
    pub(crate) item_modifier_names: Option<String>,
    pub(crate) item_unit: Option<String>,
    pub(crate) item_class_path: Option<String>,
    pub(crate) item_top_level: Option<bool>,
    pub(crate) item_is_partial: Option<bool>,
    pub(crate) item_is_final: Option<bool>,
    pub(crate) item_is_encapsulated: Option<bool>,
}

struct ModelicaParserSummaryResponseColumns {
    request_id: Vec<String>,
    source_id: Vec<String>,
    summary_kind: Vec<String>,
    backend: Vec<String>,
    success: Vec<bool>,
    primary_name: Vec<Option<String>>,
    error_message: Vec<Option<String>>,
    class_name: Vec<Option<String>>,
    restriction: Vec<Option<String>>,
    item_group: Vec<Option<String>>,
    item_name: Vec<Option<String>>,
    item_kind: Vec<Option<String>>,
    item_signature: Vec<Option<String>>,
    item_dependency_form: Vec<Option<String>>,
    item_dependency_target: Vec<Option<String>>,
    item_dependency_alias: Vec<Option<String>>,
    item_dependency_local_name: Vec<Option<String>>,
    item_text: Vec<Option<String>>,
    item_line_start: Vec<Option<i64>>,
    item_line_end: Vec<Option<i64>>,
    item_owner_name: Vec<Option<String>>,
    item_owner_path: Vec<Option<String>>,
    item_visibility: Vec<Option<String>>,
    item_type_name: Vec<Option<String>>,
    item_variability: Vec<Option<String>>,
    item_direction: Vec<Option<String>>,
    item_component_kind: Vec<Option<String>>,
    item_array_dimensions: Vec<Option<String>>,
    item_default_value: Vec<Option<String>>,
    item_start_value: Vec<Option<String>>,
    item_modifier_names: Vec<Option<String>>,
    item_unit: Vec<Option<String>>,
    item_class_path: Vec<Option<String>>,
    item_top_level: Vec<Option<bool>>,
    item_is_partial: Vec<Option<bool>>,
    item_is_final: Vec<Option<bool>>,
    item_is_encapsulated: Vec<Option<bool>>,
}

struct ModelicaParserSummaryBaseColumns {
    request_id: Vec<String>,
    source_id: Vec<String>,
    summary_kind: Vec<String>,
    backend: Vec<String>,
    success: Vec<bool>,
    primary_name: Vec<Option<String>>,
    error_message: Vec<Option<String>>,
    class_name: Vec<Option<String>>,
    restriction: Vec<Option<String>>,
    item_group: Vec<Option<String>>,
    item_name: Vec<Option<String>>,
    item_kind: Vec<Option<String>>,
    item_signature: Vec<Option<String>>,
    item_text: Vec<Option<String>>,
    item_line_start: Vec<Option<i64>>,
    item_line_end: Vec<Option<i64>>,
    item_owner_name: Vec<Option<String>>,
    item_owner_path: Vec<Option<String>>,
}

struct ModelicaParserSummaryDependencyColumns {
    form: Vec<Option<String>>,
    target: Vec<Option<String>>,
    alias: Vec<Option<String>>,
    local_name: Vec<Option<String>>,
}

struct ModelicaParserSummaryDetailColumns {
    visibility: Vec<Option<String>>,
    type_name: Vec<Option<String>>,
    variability: Vec<Option<String>>,
    direction: Vec<Option<String>>,
    component_kind: Vec<Option<String>>,
    array_dimensions: Vec<Option<String>>,
    default_value: Vec<Option<String>>,
    start_value: Vec<Option<String>>,
    modifier_names: Vec<Option<String>>,
    unit: Vec<Option<String>>,
    class_path: Vec<Option<String>>,
    top_level: Vec<Option<bool>>,
    is_partial: Vec<Option<bool>>,
    is_final: Vec<Option<bool>>,
    is_encapsulated: Vec<Option<bool>>,
}

impl ModelicaParserSummaryBaseColumns {
    fn read(batch: &RecordBatch) -> Result<Self, RepoIntelligenceError> {
        Ok(Self {
            request_id: required_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_REQUEST_ID_COLUMN,
                "response",
            )?,
            source_id: required_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_SOURCE_ID_COLUMN,
                "response",
            )?,
            summary_kind: required_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_KIND_COLUMN,
                "response",
            )?,
            backend: required_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_BACKEND_COLUMN,
                "response",
            )?,
            success: required_bool_values(
                batch,
                MODELICA_PARSER_SUMMARY_SUCCESS_COLUMN,
                "response",
            )?,
            primary_name: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN,
                "response",
            )?,
            error_message: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN,
                "response",
            )?,
            class_name: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_CLASS_NAME_COLUMN,
                "response",
            )?,
            restriction: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_RESTRICTION_COLUMN,
                "response",
            )?,
            item_group: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_GROUP_COLUMN,
                "response",
            )?,
            item_name: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_NAME_COLUMN,
                "response",
            )?,
            item_kind: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_KIND_COLUMN,
                "response",
            )?,
            item_signature: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_SIGNATURE_COLUMN,
                "response",
            )?,
            item_text: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_TEXT_COLUMN,
                "response",
            )?,
            item_line_start: optional_int_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_LINE_START_COLUMN,
                "response",
            )?,
            item_line_end: optional_int_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_LINE_END_COLUMN,
                "response",
            )?,
            item_owner_name: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_OWNER_NAME_COLUMN,
                "response",
            )?,
            item_owner_path: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_OWNER_PATH_COLUMN,
                "response",
            )?,
        })
    }
}

impl ModelicaParserSummaryDependencyColumns {
    fn read(batch: &RecordBatch) -> Result<Self, RepoIntelligenceError> {
        Ok(Self {
            form: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_FORM_COLUMN,
                "response",
            )?,
            target: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_TARGET_COLUMN,
                "response",
            )?,
            alias: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_ALIAS_COLUMN,
                "response",
            )?,
            local_name: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_DEPENDENCY_LOCAL_NAME_COLUMN,
                "response",
            )?,
        })
    }
}

impl ModelicaParserSummaryDetailColumns {
    fn read(batch: &RecordBatch) -> Result<Self, RepoIntelligenceError> {
        Ok(Self {
            visibility: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_VISIBILITY_COLUMN,
                "response",
            )?,
            type_name: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_TYPE_NAME_COLUMN,
                "response",
            )?,
            variability: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_VARIABILITY_COLUMN,
                "response",
            )?,
            direction: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_DIRECTION_COLUMN,
                "response",
            )?,
            component_kind: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_COMPONENT_KIND_COLUMN,
                "response",
            )?,
            array_dimensions: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_ARRAY_DIMENSIONS_COLUMN,
                "response",
            )?,
            default_value: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_DEFAULT_VALUE_COLUMN,
                "response",
            )?,
            start_value: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_START_VALUE_COLUMN,
                "response",
            )?,
            modifier_names: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_MODIFIER_NAMES_COLUMN,
                "response",
            )?,
            unit: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_UNIT_COLUMN,
                "response",
            )?,
            class_path: optional_utf8_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_CLASS_PATH_COLUMN,
                "response",
            )?,
            top_level: optional_bool_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_TOP_LEVEL_COLUMN,
                "response",
            )?,
            is_partial: optional_bool_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_IS_PARTIAL_COLUMN,
                "response",
            )?,
            is_final: optional_bool_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_IS_FINAL_COLUMN,
                "response",
            )?,
            is_encapsulated: optional_bool_values(
                batch,
                MODELICA_PARSER_SUMMARY_ITEM_IS_ENCAPSULATED_COLUMN,
                "response",
            )?,
        })
    }
}

impl ModelicaParserSummaryResponseColumns {
    fn read(batch: &RecordBatch) -> Result<Self, RepoIntelligenceError> {
        let base = ModelicaParserSummaryBaseColumns::read(batch)?;
        let dependency = ModelicaParserSummaryDependencyColumns::read(batch)?;
        let details = ModelicaParserSummaryDetailColumns::read(batch)?;
        Ok(Self {
            request_id: base.request_id,
            source_id: base.source_id,
            summary_kind: base.summary_kind,
            backend: base.backend,
            success: base.success,
            primary_name: base.primary_name,
            error_message: base.error_message,
            class_name: base.class_name,
            restriction: base.restriction,
            item_group: base.item_group,
            item_name: base.item_name,
            item_kind: base.item_kind,
            item_signature: base.item_signature,
            item_dependency_form: dependency.form,
            item_dependency_target: dependency.target,
            item_dependency_alias: dependency.alias,
            item_dependency_local_name: dependency.local_name,
            item_text: base.item_text,
            item_line_start: base.item_line_start,
            item_line_end: base.item_line_end,
            item_owner_name: base.item_owner_name,
            item_owner_path: base.item_owner_path,
            item_visibility: details.visibility,
            item_type_name: details.type_name,
            item_variability: details.variability,
            item_direction: details.direction,
            item_component_kind: details.component_kind,
            item_array_dimensions: details.array_dimensions,
            item_default_value: details.default_value,
            item_start_value: details.start_value,
            item_modifier_names: details.modifier_names,
            item_unit: details.unit,
            item_class_path: details.class_path,
            item_top_level: details.top_level,
            item_is_partial: details.is_partial,
            item_is_final: details.is_final,
            item_is_encapsulated: details.is_encapsulated,
        })
    }

    fn into_rows(self) -> Vec<ModelicaParserSummaryResponseRow> {
        let row_count = self.request_id.len();
        (0..row_count)
            .map(|row_index| ModelicaParserSummaryResponseRow {
                request_id: self.request_id[row_index].clone(),
                source_id: self.source_id[row_index].clone(),
                summary_kind: self.summary_kind[row_index].clone(),
                backend: self.backend[row_index].clone(),
                success: self.success[row_index],
                primary_name: self.primary_name[row_index].clone(),
                error_message: self.error_message[row_index].clone(),
                class_name: self.class_name[row_index].clone(),
                restriction: self.restriction[row_index].clone(),
                item_group: self.item_group[row_index].clone(),
                item_name: self.item_name[row_index].clone(),
                item_kind: self.item_kind[row_index].clone(),
                item_signature: self.item_signature[row_index].clone(),
                item_dependency_form: self.item_dependency_form[row_index].clone(),
                item_dependency_target: self.item_dependency_target[row_index].clone(),
                item_dependency_alias: self.item_dependency_alias[row_index].clone(),
                item_dependency_local_name: self.item_dependency_local_name[row_index].clone(),
                item_text: self.item_text[row_index].clone(),
                item_line_start: self.item_line_start[row_index],
                item_line_end: self.item_line_end[row_index],
                item_owner_name: self.item_owner_name[row_index].clone(),
                item_owner_path: self.item_owner_path[row_index].clone(),
                item_visibility: self.item_visibility[row_index].clone(),
                item_type_name: self.item_type_name[row_index].clone(),
                item_variability: self.item_variability[row_index].clone(),
                item_direction: self.item_direction[row_index].clone(),
                item_component_kind: self.item_component_kind[row_index].clone(),
                item_array_dimensions: self.item_array_dimensions[row_index].clone(),
                item_default_value: self.item_default_value[row_index].clone(),
                item_start_value: self.item_start_value[row_index].clone(),
                item_modifier_names: self.item_modifier_names[row_index].clone(),
                item_unit: self.item_unit[row_index].clone(),
                item_class_path: self.item_class_path[row_index].clone(),
                item_top_level: self.item_top_level[row_index],
                item_is_partial: self.item_is_partial[row_index],
                item_is_final: self.item_is_final[row_index],
                item_is_encapsulated: self.item_is_encapsulated[row_index],
            })
            .collect()
    }
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
        let _ = ModelicaParserSummaryResponseColumns::read(batch)?;
    }

    Ok(())
}

pub(crate) fn decode_modelica_parser_summary_response_rows(
    batches: &[RecordBatch],
) -> Result<Vec<ModelicaParserSummaryResponseRow>, RepoIntelligenceError> {
    validate_modelica_parser_summary_response_batches(batches)?;
    let mut rows = Vec::new();

    for batch in batches {
        rows.extend(ModelicaParserSummaryResponseColumns::read(batch)?.into_rows());
    }

    Ok(rows)
}

pub(crate) fn decode_modelica_parser_file_summary(
    route_kind: ParserSummaryRouteKind,
    rows: &[ModelicaParserSummaryResponseRow],
) -> Result<ModelicaParserFileSummary, RepoIntelligenceError> {
    let _summary_context = modelica_response_context(route_kind, rows)?;
    let class_name = rows.iter().find_map(|row| row.class_name.clone());
    let mut equations_by_owner = collect_equations_by_owner(rows);
    let imports = collect_modelica_imports(rows)?;
    let declarations = collect_modelica_declarations(rows, &mut equations_by_owner)?;

    Ok(ModelicaParserFileSummary {
        class_name,
        imports,
        declarations,
    })
}

fn modelica_response_context(
    route_kind: ParserSummaryRouteKind,
    rows: &[ModelicaParserSummaryResponseRow],
) -> Result<&ModelicaParserSummaryResponseRow, RepoIntelligenceError> {
    let Some(first) = rows.first() else {
        return Err(parser_summary_contract_error(
            "response",
            format!(
                "Modelica parser-summary response for route `{}` did not contain any rows",
                route_kind.route(),
            ),
        ));
    };
    let expected_summary_kind = "modelica_file_summary";
    for row in rows {
        if row.summary_kind != expected_summary_kind {
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
    Ok(first)
}

fn collect_equations_by_owner(
    rows: &[ModelicaParserSummaryResponseRow],
) -> BTreeMap<String, Vec<String>> {
    let mut equations_by_owner = BTreeMap::<String, Vec<String>>::new();
    for row in rows
        .iter()
        .filter(|row| row.item_group.as_deref() == Some("equation"))
    {
        let Some(text) = row.item_text.clone() else {
            continue;
        };
        equations_by_owner
            .entry(modelica_owner_key(row))
            .or_default()
            .push(text);
    }
    equations_by_owner
}

fn collect_modelica_imports(
    rows: &[ModelicaParserSummaryResponseRow],
) -> Result<Vec<ParsedImport>, RepoIntelligenceError> {
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
            line_start: modelica_line_number(row.item_line_start)?,
            attributes: build_import_attributes(row),
        });
    }
    Ok(imports)
}

fn collect_modelica_declarations(
    rows: &[ModelicaParserSummaryResponseRow],
    equations_by_owner: &mut BTreeMap<String, Vec<String>>,
) -> Result<Vec<ParsedDeclaration>, RepoIntelligenceError> {
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
        let mut attributes = build_declaration_attributes(row);
        let equations = equations_by_owner
            .remove(&modelica_owner_key(row))
            .unwrap_or_default();
        if !equations.is_empty() {
            attributes.insert("equation_latex".to_string(), equations.join("\n\n"));
        }
        declarations.push(ParsedDeclaration {
            name,
            kind,
            signature: row
                .item_signature
                .clone()
                .or_else(|| row.item_name.clone())
                .unwrap_or_default(),
            line_start: modelica_line_number(row.item_line_start)?,
            line_end: modelica_line_number(row.item_line_end)?,
            equations,
            attributes,
        });
    }
    Ok(declarations)
}

fn modelica_owner_key(row: &ModelicaParserSummaryResponseRow) -> String {
    row.item_owner_path
        .clone()
        .or_else(|| row.item_owner_name.clone())
        .unwrap_or_default()
}

fn modelica_line_number(value: Option<i64>) -> Result<Option<usize>, RepoIntelligenceError> {
    value
        .map(usize::try_from)
        .transpose()
        .map_err(|error| parser_summary_contract_error("response", error.to_string()))
}

fn modelica_import_kind(form: Option<&str>) -> ImportKind {
    match form {
        Some("named_import" | "unqualified_import" | "group_import") => ImportKind::Module,
        _ => ImportKind::Symbol,
    }
}

fn build_import_attributes(row: &ModelicaParserSummaryResponseRow) -> BTreeMap<String, String> {
    let mut attributes = BTreeMap::new();
    insert_text_attribute(
        &mut attributes,
        "dependency_form",
        row.item_dependency_form.as_ref(),
    );
    insert_text_attribute(
        &mut attributes,
        "dependency_alias",
        row.item_dependency_alias.as_ref(),
    );
    insert_text_attribute(
        &mut attributes,
        "dependency_local_name",
        row.item_dependency_local_name.as_ref(),
    );
    insert_text_attribute(
        &mut attributes,
        "dependency_target",
        row.item_dependency_target.as_ref(),
    );
    attributes
}

fn modelica_kind_to_repo_kind(kind: Option<&str>) -> Option<RepoSymbolKind> {
    match kind {
        Some("function") => Some(RepoSymbolKind::Function),
        Some("model" | "record" | "block" | "connector" | "type") => Some(RepoSymbolKind::Type),
        Some("constant" | "parameter") => Some(RepoSymbolKind::Constant),
        _ => None,
    }
}

fn build_declaration_attributes(
    row: &ModelicaParserSummaryResponseRow,
) -> BTreeMap<String, String> {
    let mut attributes = BTreeMap::new();
    insert_text_attribute(&mut attributes, "parser_kind", row.item_kind.as_ref());
    insert_text_attribute(&mut attributes, "class_name", row.class_name.as_ref());
    insert_text_attribute(&mut attributes, "restriction", row.restriction.as_ref());
    insert_text_attribute(&mut attributes, "visibility", row.item_visibility.as_ref());
    insert_text_attribute(&mut attributes, "type_name", row.item_type_name.as_ref());
    insert_text_attribute(
        &mut attributes,
        "variability",
        row.item_variability.as_ref(),
    );
    insert_text_attribute(&mut attributes, "direction", row.item_direction.as_ref());
    insert_text_attribute(
        &mut attributes,
        "component_kind",
        row.item_component_kind.as_ref(),
    );
    insert_text_attribute(
        &mut attributes,
        "array_dimensions",
        row.item_array_dimensions.as_ref(),
    );
    insert_text_attribute(
        &mut attributes,
        "default_value",
        row.item_default_value.as_ref(),
    );
    insert_text_attribute(
        &mut attributes,
        "start_value",
        row.item_start_value.as_ref(),
    );
    insert_text_attribute(
        &mut attributes,
        "modifier_names",
        row.item_modifier_names.as_ref(),
    );
    insert_text_attribute(&mut attributes, "unit", row.item_unit.as_ref());
    insert_text_attribute(&mut attributes, "owner_name", row.item_owner_name.as_ref());
    insert_text_attribute(&mut attributes, "owner_path", row.item_owner_path.as_ref());
    insert_text_attribute(&mut attributes, "class_path", row.item_class_path.as_ref());
    insert_bool_attribute(&mut attributes, "top_level", row.item_top_level);
    insert_bool_attribute(&mut attributes, "is_partial", row.item_is_partial);
    insert_bool_attribute(&mut attributes, "is_final", row.item_is_final);
    insert_bool_attribute(&mut attributes, "is_encapsulated", row.item_is_encapsulated);
    attributes
}

fn insert_text_attribute(
    attributes: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<&String>,
) {
    let Some(value) = value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    attributes.insert(key.to_string(), value.to_string());
}

fn insert_bool_attribute(
    attributes: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<bool>,
) {
    if let Some(value) = value {
        attributes.insert(key.to_string(), value.to_string());
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
        .map(Option::unwrap_or_default)
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

fn optional_bool_values(
    batch: &RecordBatch,
    column: &str,
    stage: &str,
) -> Result<Vec<Option<bool>>, RepoIntelligenceError> {
    let Some(array) = batch.column_by_name(column) else {
        return Ok(vec![None; batch.num_rows()]);
    };
    if matches!(array.data_type(), DataType::Null) {
        return Ok(vec![None; array.len()]);
    }
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
    Ok((0..values.len())
        .map(|index| (!values.is_null(index)).then(|| values.value(index)))
        .collect())
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

fn parser_summary_request_error(message: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to build Modelica parser-summary request batch: {}",
            message.into()
        ),
    }
}

fn parser_summary_contract_error(stage: &str, message: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "invalid Modelica parser-summary {stage} contract: {}",
            message.into()
        ),
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/modelica_plugin/parser_summary_contract.rs"]
mod tests;
