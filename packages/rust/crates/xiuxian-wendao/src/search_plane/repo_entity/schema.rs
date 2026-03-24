use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema, LanceStringArray,
    VectorStoreError,
};

use crate::analyzers::saliency::compute_repository_saliency;
use crate::analyzers::service::{
    backlinks_for, documents_backlink_lookup, example_relation_lookup,
    hierarchy_segments_from_path, infer_ecosystem, record_hierarchical_uri,
    related_modules_for_example, related_symbols_for_example,
};
use crate::analyzers::{
    ExampleRecord, ModuleRecord, RepoBacklinkItem, RepoSymbolKind, RepositoryAnalysisOutput,
    SymbolRecord,
};
use crate::gateway::studio::types::{SearchBacklinkItem, SearchHit, StudioNavigationTarget};

const CHUNK_SIZE: usize = 1_000;

const COLUMN_ID: &str = "id";
const COLUMN_ENTITY_KIND: &str = "entity_kind";
const COLUMN_NAME: &str = "name";
const COLUMN_NAME_FOLDED: &str = "name_folded";
const COLUMN_QUALIFIED_NAME_FOLDED: &str = "qualified_name_folded";
const COLUMN_PATH: &str = "path";
const COLUMN_PATH_FOLDED: &str = "path_folded";
const COLUMN_LANGUAGE: &str = "language";
const COLUMN_SYMBOL_KIND: &str = "symbol_kind";
const COLUMN_SIGNATURE_FOLDED: &str = "signature_folded";
const COLUMN_SUMMARY_FOLDED: &str = "summary_folded";
const COLUMN_RELATED_SYMBOLS_FOLDED: &str = "related_symbols_folded";
const COLUMN_RELATED_MODULES_FOLDED: &str = "related_modules_folded";
const COLUMN_SALIENCY_SCORE: &str = "saliency_score";
const COLUMN_SEARCH_TEXT: &str = "search_text";
const COLUMN_HIT_JSON: &str = "hit_json";

const ENTITY_KIND_SYMBOL: &str = "symbol";
const ENTITY_KIND_MODULE: &str = "module";
const ENTITY_KIND_EXAMPLE: &str = "example";

#[derive(Debug, Clone)]
pub(crate) struct RepoEntityRow {
    id: String,
    entity_kind: String,
    name: String,
    name_folded: String,
    qualified_name_folded: String,
    path: String,
    path_folded: String,
    language: String,
    symbol_kind: String,
    signature_folded: String,
    summary_folded: String,
    related_symbols_folded: String,
    related_modules_folded: String,
    saliency_score: f64,
    search_text: String,
    hit_json: String,
}

pub(super) fn repo_entity_schema() -> Arc<LanceSchema> {
    Arc::new(LanceSchema::new(vec![
        LanceField::new(COLUMN_ID, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_ENTITY_KIND, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_NAME, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_NAME_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_QUALIFIED_NAME_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_PATH, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_PATH_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_LANGUAGE, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_SYMBOL_KIND, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_SIGNATURE_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_SUMMARY_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_RELATED_SYMBOLS_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_RELATED_MODULES_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_SALIENCY_SCORE, LanceDataType::Float64, false),
        LanceField::new(COLUMN_SEARCH_TEXT, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_HIT_JSON, LanceDataType::Utf8, false),
    ]))
}

pub(super) fn rows_from_analysis(
    repo_id: &str,
    analysis: &RepositoryAnalysisOutput,
) -> Result<Vec<RepoEntityRow>, VectorStoreError> {
    let context = RepoEntityContext::new(repo_id, analysis);
    let mut rows = Vec::new();

    for module in &analysis.modules {
        rows.push(build_module_row(&context, module)?);
    }

    for symbol in &analysis.symbols {
        rows.push(build_symbol_row(&context, symbol)?);
    }

    for example in &analysis.examples {
        rows.push(build_example_row(&context, example)?);
    }

    Ok(rows)
}

struct RepoEntityContext<'a> {
    repo_id: &'a str,
    analysis: &'a RepositoryAnalysisOutput,
    backlink_lookup: BTreeMap<String, Vec<RepoBacklinkItem>>,
    saliency_map: HashMap<String, f64>,
    example_relations: BTreeSet<(String, String)>,
    ecosystem: &'static str,
}

impl<'a> RepoEntityContext<'a> {
    fn new(repo_id: &'a str, analysis: &'a RepositoryAnalysisOutput) -> Self {
        Self {
            repo_id,
            analysis,
            backlink_lookup: documents_backlink_lookup(&analysis.relations, &analysis.docs),
            saliency_map: compute_repository_saliency(analysis),
            example_relations: example_relation_lookup(&analysis.relations),
            ecosystem: infer_ecosystem(repo_id),
        }
    }
}

fn build_module_row(
    context: &RepoEntityContext<'_>,
    module: &ModuleRecord,
) -> Result<RepoEntityRow, VectorStoreError> {
    let module_id = module.module_id.clone();
    let path = module.path.clone();
    let language = infer_code_language(path.as_str());
    let (implicit_backlinks, implicit_backlink_items) =
        backlinks_for(module_id.as_str(), &context.backlink_lookup);
    let saliency_score = context
        .saliency_map
        .get(module_id.as_str())
        .copied()
        .unwrap_or(0.0);
    let hit = SearchHit {
        stem: module.qualified_name.clone(),
        title: Some(module.qualified_name.clone()),
        path: path.clone(),
        doc_type: Some(ENTITY_KIND_MODULE.to_string()),
        tags: repo_entity_tags(
            context.repo_id,
            ENTITY_KIND_MODULE,
            language.clone(),
            Some("module"),
            None,
        ),
        score: saliency_score,
        best_section: Some(module.module_id.clone()),
        match_reason: Some("repo_module_search".to_string()),
        hierarchical_uri: Some(record_hierarchical_uri(
            context.repo_id,
            context.ecosystem,
            "api",
            path.as_str(),
            module_id.as_str(),
        )),
        hierarchy: hierarchy_segments_from_path(path.as_str()),
        saliency_score: Some(saliency_score),
        audit_status: None,
        verification_state: None,
        implicit_backlinks,
        implicit_backlink_items: map_backlink_items(implicit_backlink_items),
        navigation_target: Some(repo_navigation_target(
            context.repo_id,
            path.as_str(),
            Some(1),
            None,
        )),
    };
    Ok(RepoEntityRow {
        id: module_id,
        entity_kind: ENTITY_KIND_MODULE.to_string(),
        name: module.qualified_name.clone(),
        name_folded: module.qualified_name.to_ascii_lowercase(),
        qualified_name_folded: module.qualified_name.to_ascii_lowercase(),
        path: path.clone(),
        path_folded: path.to_ascii_lowercase(),
        language: language.unwrap_or_default(),
        symbol_kind: "module".to_string(),
        signature_folded: String::new(),
        summary_folded: String::new(),
        related_symbols_folded: String::new(),
        related_modules_folded: String::new(),
        saliency_score,
        search_text: [module.qualified_name.as_str(), path.as_str()].join(" "),
        hit_json: serialize_hit_json(&hit)?,
    })
}

fn build_symbol_row(
    context: &RepoEntityContext<'_>,
    symbol: &SymbolRecord,
) -> Result<RepoEntityRow, VectorStoreError> {
    let symbol_id = symbol.symbol_id.clone();
    let path = symbol.path.clone();
    let language = infer_code_language(path.as_str());
    let signature = symbol.signature.clone().unwrap_or_default();
    let symbol_kind = symbol_kind_tag(symbol.kind).to_string();
    let (implicit_backlinks, implicit_backlink_items) =
        backlinks_for(symbol_id.as_str(), &context.backlink_lookup);
    let saliency_score = context
        .saliency_map
        .get(symbol_id.as_str())
        .copied()
        .unwrap_or(0.0);
    let hit = SearchHit {
        stem: symbol.name.clone(),
        title: Some(symbol.qualified_name.clone()),
        path: path.clone(),
        doc_type: Some(ENTITY_KIND_SYMBOL.to_string()),
        tags: repo_entity_tags(
            context.repo_id,
            ENTITY_KIND_SYMBOL,
            language.clone(),
            Some(symbol_kind.as_str()),
            symbol.audit_status.as_deref(),
        ),
        score: saliency_score,
        best_section: symbol
            .signature
            .clone()
            .or_else(|| Some(symbol.qualified_name.clone())),
        match_reason: Some("repo_symbol_search".to_string()),
        hierarchical_uri: Some(record_hierarchical_uri(
            context.repo_id,
            context.ecosystem,
            "api",
            path.as_str(),
            symbol_id.as_str(),
        )),
        hierarchy: hierarchy_segments_from_path(path.as_str()),
        saliency_score: Some(saliency_score),
        audit_status: symbol.audit_status.clone(),
        verification_state: symbol.verification_state.clone(),
        implicit_backlinks,
        implicit_backlink_items: map_backlink_items(implicit_backlink_items),
        navigation_target: Some(repo_navigation_target(
            context.repo_id,
            path.as_str(),
            symbol.line_start,
            symbol.line_end,
        )),
    };
    Ok(RepoEntityRow {
        id: symbol_id,
        entity_kind: ENTITY_KIND_SYMBOL.to_string(),
        name: symbol.name.clone(),
        name_folded: symbol.name.to_ascii_lowercase(),
        qualified_name_folded: symbol.qualified_name.to_ascii_lowercase(),
        path: path.clone(),
        path_folded: path.to_ascii_lowercase(),
        language: language.unwrap_or_default(),
        symbol_kind,
        signature_folded: signature.to_ascii_lowercase(),
        summary_folded: String::new(),
        related_symbols_folded: String::new(),
        related_modules_folded: String::new(),
        saliency_score,
        search_text: [
            symbol.name.as_str(),
            symbol.qualified_name.as_str(),
            signature.as_str(),
            path.as_str(),
        ]
        .join(" "),
        hit_json: serialize_hit_json(&hit)?,
    })
}

fn build_example_row(
    context: &RepoEntityContext<'_>,
    example: &ExampleRecord,
) -> Result<RepoEntityRow, VectorStoreError> {
    let example_id = example.example_id.clone();
    let path = example.path.clone();
    let language = infer_code_language(path.as_str());
    let summary = example.summary.clone().unwrap_or_default();
    let related_symbols = related_symbols_for_example(
        example_id.as_str(),
        &context.example_relations,
        &context.analysis.symbols,
    );
    let related_modules = related_modules_for_example(
        example_id.as_str(),
        &context.example_relations,
        &context.analysis.modules,
    );
    let related_symbols_text = related_symbols.join(" ");
    let related_modules_text = related_modules.join(" ");
    let (implicit_backlinks, implicit_backlink_items) =
        backlinks_for(example_id.as_str(), &context.backlink_lookup);
    let saliency_score = context
        .saliency_map
        .get(example_id.as_str())
        .copied()
        .unwrap_or(0.0);
    let hit = SearchHit {
        stem: example.title.clone(),
        title: Some(example.title.clone()),
        path: path.clone(),
        doc_type: Some(ENTITY_KIND_EXAMPLE.to_string()),
        tags: repo_entity_tags(
            context.repo_id,
            ENTITY_KIND_EXAMPLE,
            language.clone(),
            Some("example"),
            None,
        ),
        score: saliency_score,
        best_section: example.summary.clone(),
        match_reason: Some("repo_example_search".to_string()),
        hierarchical_uri: Some(record_hierarchical_uri(
            context.repo_id,
            context.ecosystem,
            "examples",
            path.as_str(),
            example_id.as_str(),
        )),
        hierarchy: hierarchy_segments_from_path(path.as_str()),
        saliency_score: Some(saliency_score),
        audit_status: None,
        verification_state: None,
        implicit_backlinks,
        implicit_backlink_items: map_backlink_items(implicit_backlink_items),
        navigation_target: Some(repo_navigation_target(
            context.repo_id,
            path.as_str(),
            Some(1),
            None,
        )),
    };
    Ok(RepoEntityRow {
        id: example_id,
        entity_kind: ENTITY_KIND_EXAMPLE.to_string(),
        name: example.title.clone(),
        name_folded: example.title.to_ascii_lowercase(),
        qualified_name_folded: example.title.to_ascii_lowercase(),
        path: path.clone(),
        path_folded: path.to_ascii_lowercase(),
        language: language.unwrap_or_default(),
        symbol_kind: "example".to_string(),
        signature_folded: String::new(),
        summary_folded: summary.to_ascii_lowercase(),
        related_symbols_folded: related_symbols.join("\n"),
        related_modules_folded: related_modules.join("\n"),
        saliency_score,
        search_text: [
            example.title.as_str(),
            summary.as_str(),
            related_symbols_text.as_str(),
            related_modules_text.as_str(),
            path.as_str(),
        ]
        .join(" "),
        hit_json: serialize_hit_json(&hit)?,
    })
}

pub(super) fn repo_entity_batches(
    rows: &[RepoEntityRow],
) -> Result<Vec<LanceRecordBatch>, VectorStoreError> {
    rows.chunks(CHUNK_SIZE)
        .map(batch_from_rows)
        .collect::<Result<Vec<_>, _>>()
}

fn batch_from_rows(rows: &[RepoEntityRow]) -> Result<LanceRecordBatch, VectorStoreError> {
    let schema = repo_entity_schema();
    let ids = rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
    let entity_kind = rows
        .iter()
        .map(|row| row.entity_kind.clone())
        .collect::<Vec<_>>();
    let names = rows.iter().map(|row| row.name.clone()).collect::<Vec<_>>();
    let name_folded = rows
        .iter()
        .map(|row| row.name_folded.clone())
        .collect::<Vec<_>>();
    let qualified_name_folded = rows
        .iter()
        .map(|row| row.qualified_name_folded.clone())
        .collect::<Vec<_>>();
    let paths = rows.iter().map(|row| row.path.clone()).collect::<Vec<_>>();
    let path_folded = rows
        .iter()
        .map(|row| row.path_folded.clone())
        .collect::<Vec<_>>();
    let languages = rows
        .iter()
        .map(|row| row.language.clone())
        .collect::<Vec<_>>();
    let symbol_kind = rows
        .iter()
        .map(|row| row.symbol_kind.clone())
        .collect::<Vec<_>>();
    let signature_folded = rows
        .iter()
        .map(|row| row.signature_folded.clone())
        .collect::<Vec<_>>();
    let summary_folded = rows
        .iter()
        .map(|row| row.summary_folded.clone())
        .collect::<Vec<_>>();
    let related_symbols_folded = rows
        .iter()
        .map(|row| row.related_symbols_folded.clone())
        .collect::<Vec<_>>();
    let related_modules_folded = rows
        .iter()
        .map(|row| row.related_modules_folded.clone())
        .collect::<Vec<_>>();
    let saliency_scores = rows
        .iter()
        .map(|row| row.saliency_score)
        .collect::<Vec<_>>();
    let search_text = rows
        .iter()
        .map(|row| row.search_text.clone())
        .collect::<Vec<_>>();
    let hit_json = rows
        .iter()
        .map(|row| row.hit_json.clone())
        .collect::<Vec<_>>();

    LanceRecordBatch::try_new(
        schema,
        vec![
            Arc::new(LanceStringArray::from(ids)),
            Arc::new(LanceStringArray::from(entity_kind)),
            Arc::new(LanceStringArray::from(names)),
            Arc::new(LanceStringArray::from(name_folded)),
            Arc::new(LanceStringArray::from(qualified_name_folded)),
            Arc::new(LanceStringArray::from(paths)),
            Arc::new(LanceStringArray::from(path_folded)),
            Arc::new(LanceStringArray::from(languages)),
            Arc::new(LanceStringArray::from(symbol_kind)),
            Arc::new(LanceStringArray::from(signature_folded)),
            Arc::new(LanceStringArray::from(summary_folded)),
            Arc::new(LanceStringArray::from(related_symbols_folded)),
            Arc::new(LanceStringArray::from(related_modules_folded)),
            Arc::new(LanceFloat64Array::from(saliency_scores)),
            Arc::new(LanceStringArray::from(search_text)),
            Arc::new(LanceStringArray::from(hit_json)),
        ],
    )
    .map_err(VectorStoreError::Arrow)
}

pub(super) const fn projected_columns() -> [&'static str; 13] {
    [
        COLUMN_ENTITY_KIND,
        COLUMN_NAME,
        COLUMN_NAME_FOLDED,
        COLUMN_QUALIFIED_NAME_FOLDED,
        COLUMN_PATH,
        COLUMN_PATH_FOLDED,
        COLUMN_LANGUAGE,
        COLUMN_SYMBOL_KIND,
        COLUMN_SIGNATURE_FOLDED,
        COLUMN_SUMMARY_FOLDED,
        COLUMN_RELATED_SYMBOLS_FOLDED,
        COLUMN_RELATED_MODULES_FOLDED,
        COLUMN_SALIENCY_SCORE,
    ]
}

pub(super) const fn hit_json_column() -> &'static str {
    COLUMN_HIT_JSON
}

pub(super) const fn search_text_column() -> &'static str {
    COLUMN_SEARCH_TEXT
}

pub(super) const fn language_column() -> &'static str {
    COLUMN_LANGUAGE
}

pub(super) const fn entity_kind_column() -> &'static str {
    COLUMN_ENTITY_KIND
}

pub(super) const fn symbol_kind_column() -> &'static str {
    COLUMN_SYMBOL_KIND
}

fn serialize_hit_json(hit: &SearchHit) -> Result<String, VectorStoreError> {
    serde_json::to_string(hit)
        .map_err(|error| VectorStoreError::General(format!("serialize repo entity hit: {error}")))
}

fn repo_entity_tags(
    repo_id: &str,
    entity_kind: &str,
    language: Option<String>,
    normalized_kind: Option<&str>,
    audit_status: Option<&str>,
) -> Vec<String> {
    let mut tags = vec![
        repo_id.to_string(),
        "code".to_string(),
        entity_kind.to_string(),
    ];
    if let Some(kind) = normalized_kind {
        tags.push(format!("kind:{kind}"));
    }
    if let Some(language) = language {
        tags.push(language.clone());
        tags.push(format!("lang:{language}"));
    }
    if let Some(audit_status) = audit_status {
        tags.push(audit_status.to_string());
    }
    tags
}

fn map_backlink_items(items: Option<Vec<RepoBacklinkItem>>) -> Option<Vec<SearchBacklinkItem>> {
    items.map(|items| {
        items
            .into_iter()
            .map(|item| SearchBacklinkItem {
                id: item.id,
                title: item.title,
                path: item.path,
                kind: item.kind,
            })
            .collect()
    })
}

fn repo_navigation_target(
    repo_id: &str,
    path: &str,
    line: Option<usize>,
    line_end: Option<usize>,
) -> StudioNavigationTarget {
    let normalized_path = path.replace('\\', "/");
    let path = if normalized_path.starts_with(&format!("{repo_id}/")) {
        normalized_path
    } else {
        format!("{repo_id}/{normalized_path}")
    };
    StudioNavigationTarget {
        path,
        category: "repo_code".to_string(),
        project_name: Some(repo_id.to_string()),
        root_label: Some(repo_id.to_string()),
        line,
        line_end,
        column: None,
    }
}

fn infer_code_language(path: &str) -> Option<String> {
    if path_has_extension(path, "jl") {
        return Some("julia".to_string());
    }
    if path_has_extension(path, "mo") {
        return Some("modelica".to_string());
    }
    if path_has_extension(path, "rs") {
        return Some("rust".to_string());
    }
    if path_has_extension(path, "py") {
        return Some("python".to_string());
    }
    if path_has_extension(path, "ts") || path_has_extension(path, "tsx") {
        return Some("typescript".to_string());
    }
    None
}

fn path_has_extension(path: &str, expected: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

fn symbol_kind_tag(kind: RepoSymbolKind) -> &'static str {
    match kind {
        RepoSymbolKind::Function => "function",
        RepoSymbolKind::Type => "type",
        RepoSymbolKind::Constant => "constant",
        RepoSymbolKind::ModuleExport => "module_export",
        RepoSymbolKind::Other => "other",
    }
}
