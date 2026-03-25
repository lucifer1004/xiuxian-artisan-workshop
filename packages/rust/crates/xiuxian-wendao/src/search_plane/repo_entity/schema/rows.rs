use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use xiuxian_vector::{LanceDataType, LanceField, LanceSchema, VectorStoreError};

use crate::analyzers::saliency::compute_repository_saliency;
use crate::analyzers::service::{
    backlinks_for, documents_backlink_lookup, example_relation_lookup,
    hierarchy_segments_from_path, infer_ecosystem, projection_page_lookup, projection_pages_for,
    record_hierarchical_uri, related_modules_for_example, related_symbols_for_example,
};
use crate::analyzers::{
    ExampleRecord, ModuleRecord, RepoBacklinkItem, RepositoryAnalysisOutput, SymbolRecord,
};
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::repo_entity::schema::definitions::{
    COLUMN_ATTRIBUTES_JSON, COLUMN_AUDIT_STATUS, COLUMN_ENTITY_KIND, COLUMN_HIERARCHICAL_URI,
    COLUMN_HIERARCHY, COLUMN_HIT_JSON, COLUMN_ID, COLUMN_IMPLICIT_BACKLINK_ITEMS_JSON,
    COLUMN_IMPLICIT_BACKLINKS, COLUMN_LANGUAGE, COLUMN_LINE_END, COLUMN_LINE_START,
    COLUMN_MODULE_ID, COLUMN_NAME, COLUMN_NAME_FOLDED, COLUMN_PATH, COLUMN_PATH_FOLDED,
    COLUMN_PROJECTION_PAGE_IDS, COLUMN_QUALIFIED_NAME, COLUMN_QUALIFIED_NAME_FOLDED,
    COLUMN_RELATED_MODULES_FOLDED, COLUMN_RELATED_SYMBOLS_FOLDED, COLUMN_SALIENCY_SCORE,
    COLUMN_SEARCH_TEXT, COLUMN_SIGNATURE, COLUMN_SIGNATURE_FOLDED, COLUMN_SUMMARY,
    COLUMN_SUMMARY_FOLDED, COLUMN_SYMBOL_KIND, COLUMN_VERIFICATION_STATE, ENTITY_KIND_EXAMPLE,
    ENTITY_KIND_MODULE, ENTITY_KIND_SYMBOL, RepoEntityRow,
};
use crate::search_plane::repo_entity::schema::helpers::{
    infer_code_language, map_backlink_items, repo_entity_tags, repo_navigation_target,
    serialize_backlink_items_json, serialize_hit_json, serialize_symbol_attributes_json,
    symbol_kind_tag,
};

pub(crate) fn repo_entity_schema() -> Arc<LanceSchema> {
    Arc::new(LanceSchema::new(vec![
        LanceField::new(COLUMN_ID, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_ENTITY_KIND, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_NAME, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_NAME_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_QUALIFIED_NAME, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_QUALIFIED_NAME_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_PATH, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_PATH_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_LANGUAGE, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_SYMBOL_KIND, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_MODULE_ID, LanceDataType::Utf8, true),
        LanceField::new(COLUMN_SIGNATURE, LanceDataType::Utf8, true),
        LanceField::new(COLUMN_SIGNATURE_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_SUMMARY, LanceDataType::Utf8, true),
        LanceField::new(COLUMN_SUMMARY_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_RELATED_SYMBOLS_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_RELATED_MODULES_FOLDED, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_LINE_START, LanceDataType::UInt32, true),
        LanceField::new(COLUMN_LINE_END, LanceDataType::UInt32, true),
        LanceField::new(COLUMN_AUDIT_STATUS, LanceDataType::Utf8, true),
        LanceField::new(COLUMN_VERIFICATION_STATE, LanceDataType::Utf8, true),
        LanceField::new(COLUMN_ATTRIBUTES_JSON, LanceDataType::Utf8, true),
        LanceField::new(COLUMN_HIERARCHICAL_URI, LanceDataType::Utf8, true),
        LanceField::new(
            COLUMN_HIERARCHY,
            LanceDataType::List(Arc::new(LanceField::new("item", LanceDataType::Utf8, true))),
            false,
        ),
        LanceField::new(
            COLUMN_IMPLICIT_BACKLINKS,
            LanceDataType::List(Arc::new(LanceField::new("item", LanceDataType::Utf8, true))),
            false,
        ),
        LanceField::new(
            COLUMN_IMPLICIT_BACKLINK_ITEMS_JSON,
            LanceDataType::Utf8,
            true,
        ),
        LanceField::new(
            COLUMN_PROJECTION_PAGE_IDS,
            LanceDataType::List(Arc::new(LanceField::new("item", LanceDataType::Utf8, true))),
            false,
        ),
        LanceField::new(COLUMN_SALIENCY_SCORE, LanceDataType::Float64, false),
        LanceField::new(COLUMN_SEARCH_TEXT, LanceDataType::Utf8, false),
        LanceField::new(COLUMN_HIT_JSON, LanceDataType::Utf8, false),
    ]))
}

pub(crate) fn rows_from_analysis(
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
    projection_lookup: BTreeMap<String, Vec<String>>,
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
            projection_lookup: projection_page_lookup(analysis),
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
    let hierarchy = hierarchy_segments_from_path(path.as_str());
    let (implicit_backlinks, implicit_backlink_items) =
        backlinks_for(module_id.as_str(), &context.backlink_lookup);
    let saliency_score = context
        .saliency_map
        .get(module_id.as_str())
        .copied()
        .unwrap_or(0.0);
    let projection_page_ids =
        projection_pages_for(module_id.as_str(), &context.projection_lookup).unwrap_or_default();
    let hierarchical_uri = record_hierarchical_uri(
        context.repo_id,
        context.ecosystem,
        "api",
        path.as_str(),
        module_id.as_str(),
    );
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
        hierarchical_uri: Some(hierarchical_uri.clone()),
        hierarchy: hierarchy.clone(),
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
        qualified_name: module.qualified_name.clone(),
        qualified_name_folded: module.qualified_name.to_ascii_lowercase(),
        path: path.clone(),
        path_folded: path.to_ascii_lowercase(),
        language: language.unwrap_or_default(),
        symbol_kind: "module".to_string(),
        module_id: Some(module.module_id.clone()),
        signature: None,
        signature_folded: String::new(),
        summary: None,
        summary_folded: String::new(),
        related_symbols_folded: String::new(),
        related_modules_folded: String::new(),
        line_start: Some(1),
        line_end: None,
        audit_status: None,
        verification_state: None,
        attributes_json: None,
        hierarchical_uri: Some(hierarchical_uri),
        hierarchy: hierarchy.clone().unwrap_or_default(),
        implicit_backlinks: hit.implicit_backlinks.clone().unwrap_or_default(),
        implicit_backlink_items_json: serialize_backlink_items_json(
            hit.implicit_backlink_items.as_ref(),
        )?,
        projection_page_ids,
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
    let hierarchy = hierarchy_segments_from_path(path.as_str());
    let (implicit_backlinks, implicit_backlink_items) =
        backlinks_for(symbol_id.as_str(), &context.backlink_lookup);
    let saliency_score = context
        .saliency_map
        .get(symbol_id.as_str())
        .copied()
        .unwrap_or(0.0);
    let projection_page_ids =
        projection_pages_for(symbol_id.as_str(), &context.projection_lookup).unwrap_or_default();
    let hierarchical_uri = record_hierarchical_uri(
        context.repo_id,
        context.ecosystem,
        "api",
        path.as_str(),
        symbol_id.as_str(),
    );
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
        hierarchical_uri: Some(hierarchical_uri.clone()),
        hierarchy: hierarchy.clone(),
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
        qualified_name: symbol.qualified_name.clone(),
        qualified_name_folded: symbol.qualified_name.to_ascii_lowercase(),
        path: path.clone(),
        path_folded: path.to_ascii_lowercase(),
        language: language.unwrap_or_default(),
        symbol_kind,
        module_id: symbol.module_id.clone(),
        signature: symbol.signature.clone(),
        signature_folded: signature.to_ascii_lowercase(),
        summary: None,
        summary_folded: String::new(),
        related_symbols_folded: String::new(),
        related_modules_folded: String::new(),
        line_start: symbol
            .line_start
            .and_then(|value| u32::try_from(value).ok()),
        line_end: symbol.line_end.and_then(|value| u32::try_from(value).ok()),
        audit_status: symbol.audit_status.clone(),
        verification_state: symbol.verification_state.clone(),
        attributes_json: serialize_symbol_attributes_json(&symbol.attributes)?,
        hierarchical_uri: Some(hierarchical_uri),
        hierarchy: hierarchy.clone().unwrap_or_default(),
        implicit_backlinks: hit.implicit_backlinks.clone().unwrap_or_default(),
        implicit_backlink_items_json: serialize_backlink_items_json(
            hit.implicit_backlink_items.as_ref(),
        )?,
        projection_page_ids,
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
    let hierarchy = hierarchy_segments_from_path(path.as_str());
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
    let projection_page_ids =
        projection_pages_for(example_id.as_str(), &context.projection_lookup).unwrap_or_default();
    let hierarchical_uri = record_hierarchical_uri(
        context.repo_id,
        context.ecosystem,
        "examples",
        path.as_str(),
        example_id.as_str(),
    );
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
        hierarchical_uri: Some(hierarchical_uri.clone()),
        hierarchy: hierarchy.clone(),
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
        qualified_name: example.title.clone(),
        qualified_name_folded: example.title.to_ascii_lowercase(),
        path: path.clone(),
        path_folded: path.to_ascii_lowercase(),
        language: language.unwrap_or_default(),
        symbol_kind: "example".to_string(),
        module_id: None,
        signature: None,
        signature_folded: String::new(),
        summary: example.summary.clone(),
        summary_folded: summary.to_ascii_lowercase(),
        related_symbols_folded: related_symbols.join("\n"),
        related_modules_folded: related_modules.join("\n"),
        line_start: Some(1),
        line_end: None,
        audit_status: None,
        verification_state: None,
        attributes_json: None,
        hierarchical_uri: Some(hierarchical_uri),
        hierarchy: hierarchy.clone().unwrap_or_default(),
        implicit_backlinks: hit.implicit_backlinks.clone().unwrap_or_default(),
        implicit_backlink_items_json: serialize_backlink_items_json(
            hit.implicit_backlink_items.as_ref(),
        )?,
        projection_page_ids,
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
