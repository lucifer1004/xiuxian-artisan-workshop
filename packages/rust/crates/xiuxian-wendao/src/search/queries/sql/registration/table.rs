use arrow::datatypes::{DataType, Field};

use crate::search::SearchCorpusKind;

pub(crate) const STUDIO_SQL_CATALOG_TABLE_NAME: &str = "wendao_sql_tables";
pub(crate) const STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME: &str = "wendao_sql_columns";
pub(crate) const STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME: &str = "wendao_sql_view_sources";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegisteredSqlTable {
    pub(crate) sql_table_name: String,
    pub(crate) engine_table_name: String,
    pub(crate) corpus: String,
    pub(crate) scope: String,
    pub(crate) sql_object_kind: String,
    pub(crate) source_count: u64,
    pub(crate) repo_id: Option<String>,
}

impl RegisteredSqlTable {
    pub(crate) fn local(
        corpus: SearchCorpusKind,
        sql_table_name: impl Into<String>,
        engine_table_name: impl Into<String>,
    ) -> Self {
        let sql_table_name = sql_table_name.into();
        let engine_table_name = engine_table_name.into();
        Self {
            sql_table_name,
            engine_table_name,
            corpus: corpus.to_string(),
            scope: "local".to_string(),
            sql_object_kind: "table".to_string(),
            source_count: 0,
            repo_id: None,
        }
    }

    pub(crate) fn repo(
        corpus: SearchCorpusKind,
        repo_id: &str,
        sql_table_name: impl Into<String>,
        engine_table_name: impl Into<String>,
    ) -> Self {
        let sql_table_name = sql_table_name.into();
        let engine_table_name = engine_table_name.into();
        Self {
            sql_table_name,
            engine_table_name,
            corpus: corpus.to_string(),
            scope: "repo".to_string(),
            sql_object_kind: "table".to_string(),
            source_count: 0,
            repo_id: Some(repo_id.to_string()),
        }
    }

    pub(crate) fn repo_logical(
        corpus: SearchCorpusKind,
        sql_table_name: impl Into<String>,
        source_count: usize,
    ) -> Self {
        let sql_table_name = sql_table_name.into();
        Self {
            engine_table_name: sql_table_name.clone(),
            sql_table_name,
            corpus: corpus.to_string(),
            scope: "repo_logical".to_string(),
            sql_object_kind: "view".to_string(),
            source_count: u64::try_from(source_count).unwrap_or(u64::MAX),
            repo_id: None,
        }
    }

    pub(crate) fn local_logical(
        corpus: SearchCorpusKind,
        sql_table_name: impl Into<String>,
        source_count: usize,
    ) -> Self {
        let sql_table_name = sql_table_name.into();
        Self {
            engine_table_name: sql_table_name.clone(),
            sql_table_name,
            corpus: corpus.to_string(),
            scope: "local_logical".to_string(),
            sql_object_kind: "view".to_string(),
            source_count: u64::try_from(source_count).unwrap_or(u64::MAX),
            repo_id: None,
        }
    }

    pub(crate) fn system(table_name: impl Into<String>) -> Self {
        let table_name = table_name.into();
        Self {
            sql_table_name: table_name.clone(),
            engine_table_name: table_name,
            corpus: "system".to_string(),
            scope: "system".to_string(),
            sql_object_kind: "system".to_string(),
            source_count: 0,
            repo_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqlQuerySurface {
    pub(crate) catalog_table_name: String,
    pub(crate) column_catalog_table_name: String,
    pub(crate) view_source_catalog_table_name: String,
    pub(crate) tables: Vec<RegisteredSqlTable>,
    pub(crate) columns: Vec<RegisteredSqlColumn>,
    pub(crate) view_sources: Vec<RegisteredSqlViewSource>,
}

impl SqlQuerySurface {
    pub(crate) fn new(
        catalog_table_name: impl Into<String>,
        column_catalog_table_name: impl Into<String>,
        view_source_catalog_table_name: impl Into<String>,
        tables: Vec<RegisteredSqlTable>,
        columns: Vec<RegisteredSqlColumn>,
        view_sources: Vec<RegisteredSqlViewSource>,
    ) -> Self {
        Self {
            catalog_table_name: catalog_table_name.into(),
            column_catalog_table_name: column_catalog_table_name.into(),
            view_source_catalog_table_name: view_source_catalog_table_name.into(),
            tables,
            columns,
            view_sources,
        }
    }

    pub(crate) fn registered_table_names(&self) -> Vec<String> {
        self.tables
            .iter()
            .map(|table| table.sql_table_name.clone())
            .collect()
    }

    pub(crate) fn registered_table_count(&self) -> usize {
        self.tables.len()
    }

    pub(crate) fn registered_column_count(&self) -> usize {
        self.columns.len()
    }

    pub(crate) fn registered_view_source_count(&self) -> usize {
        self.view_sources.len()
    }

    pub(crate) fn registered_view_count(&self) -> usize {
        self.tables
            .iter()
            .filter(|table| table.sql_object_kind == "view")
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegisteredSqlColumn {
    pub(crate) sql_table_name: String,
    pub(crate) engine_table_name: String,
    pub(crate) column_name: String,
    pub(crate) source_column_name: Option<String>,
    pub(crate) data_type: String,
    pub(crate) is_nullable: bool,
    pub(crate) ordinal_position: u64,
    pub(crate) corpus: String,
    pub(crate) scope: String,
    pub(crate) sql_object_kind: String,
    pub(crate) column_origin_kind: String,
    pub(crate) repo_id: Option<String>,
}

impl RegisteredSqlColumn {
    fn from_registered_table(
        table: &RegisteredSqlTable,
        ordinal_position: usize,
        column_name: impl Into<String>,
        source_column_name: Option<String>,
        data_type: impl Into<String>,
        is_nullable: bool,
        column_origin_kind: impl Into<String>,
    ) -> Self {
        Self {
            sql_table_name: table.sql_table_name.clone(),
            engine_table_name: table.engine_table_name.clone(),
            column_name: column_name.into(),
            source_column_name,
            data_type: data_type.into(),
            is_nullable,
            ordinal_position: u64::try_from(ordinal_position).unwrap_or(u64::MAX),
            corpus: table.corpus.clone(),
            scope: table.scope.clone(),
            sql_object_kind: table.sql_object_kind.clone(),
            column_origin_kind: column_origin_kind.into(),
            repo_id: table.repo_id.clone(),
        }
    }

    pub(crate) fn from_arrow_field(
        table: &RegisteredSqlTable,
        ordinal_position: usize,
        field: &Field,
    ) -> Self {
        let column_name = field.name().clone();
        let (source_column_name, column_origin_kind) =
            registered_sql_column_origin(table, column_name.as_str());
        Self::from_registered_table(
            table,
            ordinal_position,
            column_name,
            source_column_name,
            canonical_sql_data_type_name(field.data_type()),
            field.is_nullable(),
            column_origin_kind,
        )
    }
}

fn canonical_sql_data_type_name(data_type: &DataType) -> String {
    match data_type {
        DataType::Utf8View => "Utf8".to_string(),
        DataType::BinaryView => "Binary".to_string(),
        _ => data_type.to_string(),
    }
}

fn registered_sql_column_origin(
    table: &RegisteredSqlTable,
    column_name: &str,
) -> (Option<String>, &'static str) {
    if table.sql_object_kind == "system" {
        return (None, "synthetic");
    }
    if table.sql_object_kind == "view" {
        if column_name == "repo_id"
            || is_synthetic_repo_content_chunk_logical_column(table, column_name)
        {
            return (None, "synthetic");
        }
        return (Some(column_name.to_string()), "projected");
    }
    (Some(column_name.to_string()), "stored")
}

fn is_synthetic_repo_content_chunk_logical_column(
    table: &RegisteredSqlTable,
    column_name: &str,
) -> bool {
    table.corpus == SearchCorpusKind::RepoContentChunk.to_string()
        && table.scope == "repo_logical"
        && matches!(
            column_name,
            "title" | "doc_type" | "code_tag" | "file_tag" | "kind_tag" | "language_tag"
        )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegisteredSqlViewSource {
    pub(crate) sql_view_name: String,
    pub(crate) source_sql_table_name: String,
    pub(crate) source_engine_table_name: String,
    pub(crate) corpus: String,
    pub(crate) repo_id: Option<String>,
    pub(crate) source_ordinal: u64,
}

impl RegisteredSqlViewSource {
    pub(crate) fn logical(
        sql_view_name: &str,
        source_table: &RegisteredSqlTable,
        source_ordinal: usize,
    ) -> Self {
        Self {
            sql_view_name: sql_view_name.to_string(),
            source_sql_table_name: source_table.sql_table_name.clone(),
            source_engine_table_name: source_table.engine_table_name.clone(),
            corpus: source_table.corpus.clone(),
            repo_id: source_table.repo_id.clone(),
            source_ordinal: u64::try_from(source_ordinal).unwrap_or(u64::MAX),
        }
    }
}
