use std::fmt::Write as _;

use serde_json::Value;

use super::contract::{SurfaceBundle, SurfaceColumn, SurfaceObject};
use super::gateway::SqlQueryPayload;

pub(super) fn surface_bundle_xml(bundle: &SurfaceBundle) -> String {
    let mut xml = String::new();
    xml.push_str("<surface_bundle>");
    text_tag(&mut xml, "project_root", bundle.project_root.as_str());
    text_tag(
        &mut xml,
        "catalog_table_name",
        bundle.catalog_table_name.as_str(),
    );
    text_tag(
        &mut xml,
        "column_catalog_table_name",
        bundle.column_catalog_table_name.as_str(),
    );
    text_tag(
        &mut xml,
        "view_source_catalog_table_name",
        bundle.view_source_catalog_table_name.as_str(),
    );
    xml.push_str("<policy>");
    number_tag(&mut xml, "max_limit", bundle.policy.max_limit);
    for op in &bundle.policy.allowed_ops {
        text_tag(&mut xml, "allowed_op", op.as_str());
    }
    for object in &bundle.policy.require_filter_for {
        text_tag(&mut xml, "require_filter_for", object.as_str());
    }
    xml.push_str("</policy>");
    xml.push_str("<objects>");
    for object in &bundle.objects {
        write_object(&mut xml, object);
    }
    xml.push_str("</objects>");
    xml.push_str("</surface_bundle>");
    xml
}

pub(super) fn validation_report_xml(
    status: &str,
    message: &str,
    canonical_sql: Option<&str>,
) -> String {
    let mut xml = String::new();
    xml.push_str("<validation_report>");
    text_tag(&mut xml, "status", status);
    text_tag(&mut xml, "message", message);
    if let Some(sql) = canonical_sql {
        text_tag(&mut xml, "canonical_sql", sql);
    }
    xml.push_str("</validation_report>");
    xml
}

pub(super) fn execution_report_xml(
    status: &str,
    message: &str,
    payload: Option<&SqlQueryPayload>,
    max_rows: usize,
) -> String {
    let mut xml = String::new();
    xml.push_str("<execution_report>");
    text_tag(&mut xml, "status", status);
    text_tag(&mut xml, "message", message);
    if let Some(payload) = payload {
        xml.push_str("<metadata>");
        text_tag(
            &mut xml,
            "catalog_table_name",
            payload.metadata.catalog_table_name.as_str(),
        );
        text_tag(
            &mut xml,
            "column_catalog_table_name",
            payload.metadata.column_catalog_table_name.as_str(),
        );
        text_tag(
            &mut xml,
            "view_source_catalog_table_name",
            payload.metadata.view_source_catalog_table_name.as_str(),
        );
        number_tag(
            &mut xml,
            "registered_table_count",
            payload.metadata.registered_table_count,
        );
        number_tag(
            &mut xml,
            "result_batch_count",
            payload.metadata.result_batch_count,
        );
        number_tag(
            &mut xml,
            "result_row_count",
            payload.metadata.result_row_count,
        );
        xml.push_str("<registered_tables>");
        for table in &payload.metadata.registered_tables {
            text_tag(&mut xml, "table", table.as_str());
        }
        xml.push_str("</registered_tables>");
        xml.push_str("</metadata>");
        xml.push_str("<preview_rows>");
        for row in payload
            .batches
            .iter()
            .flat_map(|batch| batch.rows.iter())
            .take(max_rows)
        {
            text_tag(
                &mut xml,
                "row",
                Value::Object(row.clone()).to_string().as_str(),
            );
        }
        xml.push_str("</preview_rows>");
    }
    xml.push_str("</execution_report>");
    xml
}

fn write_object(xml: &mut String, object: &SurfaceObject) {
    xml.push_str("<object>");
    text_tag(xml, "name", object.name.as_str());
    text_tag(xml, "kind", object.kind.as_str());
    text_tag(xml, "scope", object.scope.as_str());
    text_tag(xml, "corpus", object.corpus.as_str());
    if let Some(repo_id) = &object.repo_id {
        text_tag(xml, "repo_id", repo_id.as_str());
    }
    number_tag(xml, "source_count", object.source_count);
    xml.push_str("<columns>");
    for column in &object.columns {
        write_column(xml, column);
    }
    xml.push_str("</columns>");
    xml.push_str("</object>");
}

fn write_column(xml: &mut String, column: &SurfaceColumn) {
    xml.push_str("<column>");
    text_tag(xml, "name", column.name.as_str());
    text_tag(xml, "data_type", column.data_type.as_str());
    text_tag(
        xml,
        "nullable",
        if column.nullable { "true" } else { "false" },
    );
    number_tag(xml, "ordinal_position", column.ordinal_position);
    text_tag(xml, "origin_kind", column.origin_kind.as_str());
    xml.push_str("</column>");
}

fn text_tag(xml: &mut String, tag: &str, value: &str) {
    let _ = write!(xml, "<{tag}>{}</{tag}>", escape_xml(value));
}

fn number_tag(xml: &mut String, tag: &str, value: usize) {
    let _ = write!(xml, "<{tag}>{value}</{tag}>");
}

fn escape_xml(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&apos;".to_string(),
            _ => ch.to_string(),
        })
        .collect()
}
