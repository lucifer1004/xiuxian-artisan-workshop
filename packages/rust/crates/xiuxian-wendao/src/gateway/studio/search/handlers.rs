use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::link_graph::{
    LinkGraphAttachmentKind, LinkGraphDisplayHit, LinkGraphIndex, LinkGraphRetrievalMode,
    LinkGraphSearchOptions,
};
use crate::repo_intelligence::{
    ExampleSearchQuery as RepoExampleSearchQuery, ExampleSearchResult as RepoExampleSearchResult,
    ModuleSearchQuery as RepoModuleSearchQuery, ModuleSearchResult as RepoModuleSearchResult,
    RegisteredRepository, RepoBacklinkItem, RepoIntelligenceError,
    RepoSyncMode, RepoSyncQuery,
    SymbolSearchQuery as RepoSymbolSearchQuery, SymbolSearchResult as RepoSymbolSearchResult,
    analyze_registered_repository, build_example_search, build_module_search,
    repo_sync_for_registered_repository,
    build_symbol_search,
};
use crate::unified_symbol::UnifiedSymbolIndex;

use super::super::pathing;
use super::super::router::{GatewayState, StudioApiError, configured_repositories};
use super::super::types::{
    AstSearchHit, AstSearchResponse, AttachmentSearchHit, AttachmentSearchKind,
    AttachmentSearchResponse, AutocompleteResponse, AutocompleteSuggestion,
    AutocompleteSuggestionType, DefinitionResolveResponse, ReferenceSearchResponse,
    SearchBacklinkItem, SearchHit, SearchResponse, StudioNavigationTarget, SymbolSearchHit,
    SymbolSearchResponse, SymbolSearchSource, UiProjectConfig,
};
use super::super::vfs::{graph_lookup_candidates, studio_display_path};

use super::definition::{
    DefinitionResolveOptions, ast_hit_matches, enrich_ast_hit_project_metadata,
    resolve_definition_candidates, score_ast_hit,
};
use super::observation_hints::definition_observation_hints;
use super::project_scope::project_metadata_for_path;
use super::source_index;
use super::source_index::build_reference_hits;
use super::support::source_language_label;

const DEFAULT_SEARCH_LIMIT: usize = 10;
const MAX_SEARCH_LIMIT: usize = 200;
const DEFAULT_ATTACHMENT_SEARCH_LIMIT: usize = 10;
const MAX_ATTACHMENT_SEARCH_LIMIT: usize = 200;
const DEFAULT_AST_SEARCH_LIMIT: usize = 10;
const MAX_AST_SEARCH_LIMIT: usize = 200;
const DEFAULT_REFERENCE_SEARCH_LIMIT: usize = 10;
const MAX_REFERENCE_SEARCH_LIMIT: usize = 200;
const DEFAULT_SYMBOL_SEARCH_LIMIT: usize = 10;
const MAX_SYMBOL_SEARCH_LIMIT: usize = 200;
const DEFAULT_AUTOCOMPLETE_LIMIT: usize = 5;
const MAX_AUTOCOMPLETE_LIMIT: usize = 20;
const CODE_CONTENT_SNIPPET_MAX_CHARS: usize = 160;
const CODE_CONTENT_RIPGREP_MAX_FILE_SIZE: &str = "512K";
const CODE_CONTENT_EXCLUDE_GLOBS: [&str; 8] = [
    "!.git/**",
    "!node_modules/**",
    "!target/**",
    "!dist/**",
    "!build/**",
    "!.cache/**",
    "!**/*.min.js",
    "!**/*.min.css",
];
const CODE_CONTENT_EXTENSIONS: [&str; 15] = [
    "jl", "julia", "mo", "rs", "py", "ts", "tsx", "js", "jsx", "m", "c", "cpp", "h", "hpp",
    "java",
];

pub(crate) fn build_ast_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Result<Vec<AstSearchHit>, String> {
    source_index::build_ast_index(project_root, config_root, projects)
}

pub(crate) fn build_symbol_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Result<UnifiedSymbolIndex, String> {
    source_index::build_symbol_index(project_root, config_root, projects)
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct SearchQuery {
    q: Option<String>,
    #[serde(default)]
    intent: Option<String>,
    #[serde(default)]
    repo: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct AttachmentSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    ext: Vec<String>,
    #[serde(default)]
    kind: Vec<String>,
    #[serde(default)]
    case_sensitive: bool,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct AutocompleteQuery {
    prefix: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct SymbolSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct AstSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct DefinitionResolveQuery {
    q: Option<String>,
    path: Option<String>,
    #[serde(default)]
    line: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct ReferenceSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

pub(in crate::gateway::studio) async fn search_knowledge(
    Query(query): Query<SearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let payload = run_knowledge_search(query, state, None).await?;
    Ok(Json(payload))
}

pub(in crate::gateway::studio) async fn search_intent(
    Query(query): Query<SearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let payload = run_knowledge_search(query, state, Some("intent_search")).await?;
    Ok(Json(payload))
}

async fn run_knowledge_search(
    query: SearchQuery,
    state: Arc<GatewayState>,
    default_intent: Option<&str>,
) -> Result<SearchResponse, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT);
    let requested_intent = normalized_intent(query.intent.as_deref());
    let requested_repo = normalized_repo_filter(query.repo.as_deref());

    if is_code_search_intent(requested_intent.as_deref()) {
        return run_repo_code_search(
            raw_query,
            limit,
            requested_repo.as_deref(),
            requested_intent,
            default_intent,
            state,
        )
        .await;
    }

    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.link_graph_index().await?;
    let payload = index.search_planned_payload(raw_query, limit, LinkGraphSearchOptions::default());

    let hits = payload
        .hits
        .into_iter()
        .filter_map(|hit| {
            let canonical_path =
                canonical_graph_path(state.as_ref(), index.as_ref(), hit.path.as_str());
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                canonical_path.as_str(),
            )
            .then_some((hit, canonical_path))
        })
        .map(|(hit, canonical_path)| {
            let path = studio_display_path(state.studio.as_ref(), canonical_path.as_str());
            let navigation_target = crate::gateway::studio::vfs::resolve_navigation_target(
                state.studio.as_ref(),
                path.as_str(),
            );
            let hierarchy = hierarchy_segments(path.as_str());
            let hierarchical_uri = hierarchical_uri_for_path(path.as_str());
            SearchHit {
                stem: hit.stem,
                title: strip_option(&hit.title),
                path,
                doc_type: hit.doc_type,
                tags: hit.tags,
                score: hit.score.max(0.0),
                best_section: strip_option(&hit.best_section),
                match_reason: strip_option(&hit.match_reason),
                hierarchical_uri,
                hierarchy,
                saliency_score: Some(hit.score.max(0.0)),
                audit_status: None,
                verification_state: None,
                implicit_backlinks: None,
                implicit_backlink_items: None,
                navigation_target,
            }
        })
        .collect::<Vec<_>>();
    let hit_count = hits.len();
    let selected_mode = retrieval_mode_to_string(payload.selected_mode);
    let intent = requested_intent
        .or_else(|| default_intent.map(str::to_string))
        .or_else(|| inferred_intent_from_mode(selected_mode.as_str()));
    let intent_confidence = intent
        .as_ref()
        .map(|_| payload.graph_confidence_score.clamp(0.0, 1.0));

    Ok(SearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        graph_confidence_score: Some(payload.graph_confidence_score),
        selected_mode: Some(selected_mode.clone()),
        intent,
        intent_confidence,
        search_mode: Some(selected_mode),
    })
}

#[derive(Debug)]
struct RankedCodeSearchHit {
    hit: SearchHit,
    score: f64,
    rank: usize,
}

async fn run_repo_code_search(
    raw_query: &str,
    limit: usize,
    repo_filter: Option<&str>,
    requested_intent: Option<String>,
    default_intent: Option<&str>,
    state: Arc<GatewayState>,
) -> Result<SearchResponse, StudioApiError> {
    let worker_query = raw_query.to_string();
    let worker_repo_filter = repo_filter.map(str::to_string);
    let worker_project_root = state.studio.project_root.clone();
    let worker_repositories = configured_repositories(state.studio.as_ref());
    let ranked_hits = tokio::task::spawn_blocking(move || {
        collect_repo_code_search_hits(
            worker_project_root.as_path(),
            worker_repositories,
            worker_query.as_str(),
            limit,
            worker_repo_filter.as_deref(),
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_CODE_SEARCH_PANIC",
            "Repo code search task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)?;

    let hits = ranked_hits
        .into_iter()
        .map(|ranked| ranked.hit)
        .collect::<Vec<_>>();
    let hit_count = hits.len();
    let intent = requested_intent
        .or_else(|| default_intent.map(str::to_string))
        .or_else(|| Some("code_search".to_string()));
    let intent_confidence = intent.as_ref().map(|_| 1.0);

    Ok(SearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        graph_confidence_score: None,
        selected_mode: Some("code_search".to_string()),
        intent,
        intent_confidence,
        search_mode: Some("code_search".to_string()),
    })
}

fn collect_repo_code_search_hits(
    project_root: &Path,
    repositories: Vec<RegisteredRepository>,
    query: &str,
    limit: usize,
    repo_filter: Option<&str>,
) -> Result<Vec<RankedCodeSearchHit>, RepoIntelligenceError> {
    let mut repositories = repositories;
    repositories.sort_by(|left, right| left.id.cmp(&right.id));
    repositories.dedup_by(|left, right| left.id == right.id);
    if let Some(filter) = normalized_repo_filter(repo_filter) {
        if !repositories.iter().any(|repository| repository.id == filter) {
            return Err(RepoIntelligenceError::UnknownRepository { repo_id: filter });
        }
        repositories.retain(|repository| repository.id == filter);
    }
    if repositories.is_empty() {
        return Ok(Vec::new());
    }

    let source_limit = repo_code_source_limit(limit);
    let mut ranked = Vec::new();
    for repository in repositories {
        let repo_id = repository.id.clone();
        let content_hits = rank_content_search_hits(
            project_root,
            &repository,
            query,
            source_limit,
        )?;
        let content_hit_count = content_hits.len();
        ranked.extend(content_hits);
        if content_hit_count > 0 {
            continue;
        }

        let structured_limit = source_limit;
        let analysis = analyze_registered_repository(&repository, project_root)?;
        let symbol_result = build_symbol_search(
            &RepoSymbolSearchQuery {
                repo_id: repo_id.clone(),
                query: query.to_string(),
                limit: structured_limit,
            },
            &analysis,
        );
        ranked.extend(rank_symbol_search_hits(repo_id.as_str(), symbol_result));

        let module_result = build_module_search(
            &RepoModuleSearchQuery {
                repo_id: repo_id.clone(),
                query: query.to_string(),
                limit: structured_limit,
            },
            &analysis,
        );
        ranked.extend(rank_module_search_hits(repo_id.as_str(), module_result));

        let example_result = build_example_search(
            &RepoExampleSearchQuery {
                repo_id: repo_id.clone(),
                query: query.to_string(),
                limit: structured_limit,
            },
            &analysis,
        );
        ranked.extend(rank_example_search_hits(repo_id.as_str(), example_result));
    }

    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.rank.cmp(&right.rank))
            .then_with(|| left.hit.path.cmp(&right.hit.path))
            .then_with(|| left.hit.stem.cmp(&right.hit.stem))
    });

    let mut dedup = HashSet::new();
    ranked.retain(|entry| {
        let key = format!(
            "{}|{}|{}",
            entry.hit.doc_type.as_deref().unwrap_or("unknown"),
            entry.hit.path,
            entry.hit.stem
        );
        dedup.insert(key)
    });
    ranked.truncate(limit);
    Ok(ranked)
}

fn rank_symbol_search_hits(
    repo_id: &str,
    result: RepoSymbolSearchResult,
) -> Vec<RankedCodeSearchHit> {
    if !result.symbol_hits.is_empty() {
        return result
            .symbol_hits
            .into_iter()
            .enumerate()
            .map(|(index, hit)| {
                let rank = hit.rank.unwrap_or(index + 1);
                let score = score_with_rank_fallback(hit.saliency_score.or(hit.score), rank);
                let path = normalize_repo_record_path(hit.symbol.path.as_str());
                let hierarchy = hit
                    .hierarchy
                    .or_else(|| repo_hierarchy_segments(repo_id, path.as_str()));
                let hierarchical_uri = hit
                    .hierarchical_uri
                    .or_else(|| repo_hierarchical_uri(repo_id, path.as_str()));
                let audit_status = hit.audit_status.or(hit.symbol.audit_status.clone());
                let verification_state = hit.verification_state.or_else(|| {
                    verification_state_from_audit_status(audit_status.as_deref())
                        .map(str::to_string)
                });
                let kind = format!("{:?}", hit.symbol.kind).to_ascii_lowercase();
                RankedCodeSearchHit {
                    hit: SearchHit {
                        stem: hit.symbol.name.clone(),
                        title: Some(hit.symbol.qualified_name.clone()),
                        path: path.clone(),
                        doc_type: Some("symbol".to_string()),
                        tags: vec![
                            repo_id.to_string(),
                            "code".to_string(),
                            "symbol".to_string(),
                            format!("kind:{kind}"),
                        ],
                        score,
                        best_section: hit.symbol.signature.clone(),
                        match_reason: Some("repo_symbol_search".to_string()),
                        hierarchical_uri,
                        hierarchy,
                        saliency_score: hit.saliency_score.or(hit.score),
                        audit_status,
                        verification_state,
                        implicit_backlinks: hit.implicit_backlinks,
                        implicit_backlink_items: map_repo_backlink_items(
                            hit.implicit_backlink_items,
                        ),
                        navigation_target: repo_navigation_target(repo_id, path.as_str()),
                    },
                    score,
                    rank,
                }
            })
            .collect();
    }

    result
        .symbols
        .into_iter()
        .enumerate()
        .map(|(index, symbol)| {
            let rank = index + 1;
            let score = score_with_rank_fallback(None, rank);
            let path = normalize_repo_record_path(symbol.path.as_str());
            let hierarchy = repo_hierarchy_segments(repo_id, path.as_str());
            let hierarchical_uri = repo_hierarchical_uri(repo_id, path.as_str());
            let kind = format!("{:?}", symbol.kind).to_ascii_lowercase();
            RankedCodeSearchHit {
                hit: SearchHit {
                    stem: symbol.name.clone(),
                    title: Some(symbol.qualified_name.clone()),
                    path: path.clone(),
                    doc_type: Some("symbol".to_string()),
                    tags: vec![
                        repo_id.to_string(),
                        "code".to_string(),
                        "symbol".to_string(),
                        format!("kind:{kind}"),
                    ],
                    score,
                    best_section: symbol.signature,
                    match_reason: Some("repo_symbol_search".to_string()),
                    hierarchical_uri,
                    hierarchy,
                    saliency_score: Some(score),
                    audit_status: symbol.audit_status.clone(),
                    verification_state: verification_state_from_audit_status(
                        symbol.audit_status.as_deref(),
                    )
                    .map(str::to_string),
                    implicit_backlinks: None,
                    implicit_backlink_items: None,
                    navigation_target: repo_navigation_target(repo_id, path.as_str()),
                },
                score,
                rank,
            }
        })
        .collect()
}

fn rank_module_search_hits(
    repo_id: &str,
    result: RepoModuleSearchResult,
) -> Vec<RankedCodeSearchHit> {
    if !result.module_hits.is_empty() {
        return result
            .module_hits
            .into_iter()
            .enumerate()
            .map(|(index, hit)| {
                let rank = hit.rank.unwrap_or(index + 1);
                let score = score_with_rank_fallback(hit.saliency_score.or(hit.score), rank);
                let path = normalize_repo_record_path(hit.module.path.as_str());
                let hierarchy = hit
                    .hierarchy
                    .or_else(|| repo_hierarchy_segments(repo_id, path.as_str()));
                let hierarchical_uri = hit
                    .hierarchical_uri
                    .or_else(|| repo_hierarchical_uri(repo_id, path.as_str()));
                RankedCodeSearchHit {
                    hit: SearchHit {
                        stem: module_stem(hit.module.qualified_name.as_str()),
                        title: Some(hit.module.qualified_name.clone()),
                        path: path.clone(),
                        doc_type: Some("module".to_string()),
                        tags: vec![
                            repo_id.to_string(),
                            "code".to_string(),
                            "module".to_string(),
                        ],
                        score,
                        best_section: Some(hit.module.module_id.clone()),
                        match_reason: Some("repo_module_search".to_string()),
                        hierarchical_uri,
                        hierarchy,
                        saliency_score: hit.saliency_score.or(hit.score),
                        audit_status: None,
                        verification_state: None,
                        implicit_backlinks: hit.implicit_backlinks,
                        implicit_backlink_items: map_repo_backlink_items(
                            hit.implicit_backlink_items,
                        ),
                        navigation_target: repo_navigation_target(repo_id, path.as_str()),
                    },
                    score,
                    rank,
                }
            })
            .collect();
    }

    result
        .modules
        .into_iter()
        .enumerate()
        .map(|(index, module)| {
            let rank = index + 1;
            let score = score_with_rank_fallback(None, rank);
            let path = normalize_repo_record_path(module.path.as_str());
            RankedCodeSearchHit {
                hit: SearchHit {
                    stem: module_stem(module.qualified_name.as_str()),
                    title: Some(module.qualified_name),
                    path: path.clone(),
                    doc_type: Some("module".to_string()),
                    tags: vec![
                        repo_id.to_string(),
                        "code".to_string(),
                        "module".to_string(),
                    ],
                    score,
                    best_section: Some(module.module_id),
                    match_reason: Some("repo_module_search".to_string()),
                    hierarchical_uri: repo_hierarchical_uri(repo_id, path.as_str()),
                    hierarchy: repo_hierarchy_segments(repo_id, path.as_str()),
                    saliency_score: Some(score),
                    audit_status: None,
                    verification_state: None,
                    implicit_backlinks: None,
                    implicit_backlink_items: None,
                    navigation_target: repo_navigation_target(repo_id, path.as_str()),
                },
                score,
                rank,
            }
        })
        .collect()
}

fn rank_example_search_hits(
    repo_id: &str,
    result: RepoExampleSearchResult,
) -> Vec<RankedCodeSearchHit> {
    if !result.example_hits.is_empty() {
        return result
            .example_hits
            .into_iter()
            .enumerate()
            .map(|(index, hit)| {
                let rank = hit.rank.unwrap_or(index + 1);
                let score = score_with_rank_fallback(hit.saliency_score.or(hit.score), rank);
                let path = normalize_repo_record_path(hit.example.path.as_str());
                let hierarchy = hit
                    .hierarchy
                    .or_else(|| repo_hierarchy_segments(repo_id, path.as_str()));
                let hierarchical_uri = hit
                    .hierarchical_uri
                    .or_else(|| repo_hierarchical_uri(repo_id, path.as_str()));
                RankedCodeSearchHit {
                    hit: SearchHit {
                        stem: hit.example.title.clone(),
                        title: Some(hit.example.title),
                        path: path.clone(),
                        doc_type: Some("example".to_string()),
                        tags: vec![
                            repo_id.to_string(),
                            "code".to_string(),
                            "example".to_string(),
                        ],
                        score,
                        best_section: hit.example.summary,
                        match_reason: Some("repo_example_search".to_string()),
                        hierarchical_uri,
                        hierarchy,
                        saliency_score: hit.saliency_score.or(hit.score),
                        audit_status: None,
                        verification_state: None,
                        implicit_backlinks: hit.implicit_backlinks,
                        implicit_backlink_items: map_repo_backlink_items(
                            hit.implicit_backlink_items,
                        ),
                        navigation_target: repo_navigation_target(repo_id, path.as_str()),
                    },
                    score,
                    rank,
                }
            })
            .collect();
    }

    result
        .examples
        .into_iter()
        .enumerate()
        .map(|(index, example)| {
            let rank = index + 1;
            let score = score_with_rank_fallback(None, rank);
            let path = normalize_repo_record_path(example.path.as_str());
            RankedCodeSearchHit {
                hit: SearchHit {
                    stem: example.title.clone(),
                    title: Some(example.title),
                    path: path.clone(),
                    doc_type: Some("example".to_string()),
                    tags: vec![
                        repo_id.to_string(),
                        "code".to_string(),
                        "example".to_string(),
                    ],
                    score,
                    best_section: example.summary,
                    match_reason: Some("repo_example_search".to_string()),
                    hierarchical_uri: repo_hierarchical_uri(repo_id, path.as_str()),
                    hierarchy: repo_hierarchy_segments(repo_id, path.as_str()),
                    saliency_score: Some(score),
                    audit_status: None,
                    verification_state: None,
                    implicit_backlinks: None,
                    implicit_backlink_items: None,
                    navigation_target: repo_navigation_target(repo_id, path.as_str()),
                },
                score,
                rank,
            }
        })
        .collect()
}

fn rank_content_search_hits(
    project_root: &Path,
    repository: &RegisteredRepository,
    query: &str,
    limit: usize,
) -> Result<Vec<RankedCodeSearchHit>, RepoIntelligenceError> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let checkout_root = resolve_content_search_checkout_root(project_root, repository)?;
    if !checkout_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut command = Command::new("rg");
    command
        .arg("--line-number")
        .arg("--with-filename")
        .arg("--no-heading")
        .arg("--color")
        .arg("never")
        .arg("--fixed-strings")
        .arg("--ignore-case")
        .arg("--max-count")
        .arg("1")
        .arg("--max-filesize")
        .arg(CODE_CONTENT_RIPGREP_MAX_FILE_SIZE);
    for glob in CODE_CONTENT_EXCLUDE_GLOBS {
        command.arg("--glob").arg(glob);
    }
    command.arg(query).arg(checkout_root.as_path());

    let output = match command.output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to execute content search for repo `{}`: {error}",
                    repository.id
                ),
            });
        }
    };
    if !output.status.success() {
        if output.status.code() == Some(1) {
            return Ok(Vec::new());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            "unknown ripgrep failure".to_string()
        } else {
            stderr
        };
        return Err(RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "content search failed for repo `{}`: {detail}",
                repository.id
            ),
        });
    }

    let repo_id = repository.id.clone();
    let query_lc = query.to_ascii_lowercase();
    let mut ranked = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if ranked.len() >= limit {
            break;
        }
        let Some((raw_path, line_number, line_snippet)) = parse_content_search_line(line) else {
            continue;
        };
        let normalized_path = normalize_repo_record_path(
            Path::new(raw_path)
                .strip_prefix(checkout_root.as_path())
                .unwrap_or_else(|_| Path::new(raw_path))
                .to_string_lossy()
                .as_ref(),
        );
        if normalized_path.is_empty() || !is_supported_code_extension(normalized_path.as_str()) {
            continue;
        }
        let rank = ranked.len() + 1;
        let score = content_search_score(
            query_lc.as_str(),
            normalized_path.as_str(),
            line_snippet,
            rank,
        );
        let stem = Path::new(normalized_path.as_str())
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(normalized_path.as_str())
            .to_string();
        ranked.push(RankedCodeSearchHit {
            hit: SearchHit {
                stem,
                title: Some(normalized_path.clone()),
                path: normalized_path.clone(),
                doc_type: Some("code".to_string()),
                tags: vec![
                    repo_id.clone(),
                    "code".to_string(),
                    "content".to_string(),
                ],
                score,
                best_section: Some(content_search_best_section(line_number, line_snippet)),
                match_reason: Some("repo_content_search".to_string()),
                hierarchical_uri: repo_hierarchical_uri(repo_id.as_str(), normalized_path.as_str()),
                hierarchy: repo_hierarchy_segments(repo_id.as_str(), normalized_path.as_str()),
                saliency_score: Some(score),
                audit_status: None,
                verification_state: None,
                implicit_backlinks: None,
                implicit_backlink_items: None,
                navigation_target: repo_navigation_target(repo_id.as_str(), normalized_path.as_str()),
            },
            score,
            rank,
        });
    }

    Ok(ranked)
}

fn resolve_content_search_checkout_root(
    project_root: &Path,
    repository: &RegisteredRepository,
) -> Result<PathBuf, RepoIntelligenceError> {
    let sync = repo_sync_for_registered_repository(
        &RepoSyncQuery {
            repo_id: repository.id.clone(),
            mode: RepoSyncMode::Status,
        },
        repository,
        project_root,
    )?;
    let checkout_path = PathBuf::from(sync.checkout_path);
    if checkout_path.is_absolute() {
        Ok(checkout_path)
    } else {
        Ok(project_root.join(checkout_path))
    }
}

fn parse_content_search_line(line: &str) -> Option<(&str, usize, &str)> {
    let mut segments = line.splitn(3, ':');
    let path = segments.next()?.trim();
    let line_number = segments.next()?.trim().parse::<usize>().ok()?.max(1);
    let snippet = segments.next().unwrap_or_default().trim();
    (!path.is_empty()).then_some((path, line_number, snippet))
}

fn is_supported_code_extension(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .is_some_and(|extension| CODE_CONTENT_EXTENSIONS.contains(&extension.as_str()))
}

fn content_search_score(query_lc: &str, path: &str, snippet: &str, rank: usize) -> f64 {
    let path_lc = path.to_ascii_lowercase();
    let file_name_lc = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let snippet_lc = snippet.to_ascii_lowercase();

    let mut score = 0.58;
    if file_name_lc.contains(query_lc) {
        score += 0.2;
    } else if path_lc.contains(query_lc) {
        score += 0.12;
    }
    if snippet_lc.contains(query_lc) {
        score += 0.12;
    }
    if snippet_lc.starts_with(query_lc) {
        score += 0.05;
    }
    score -= (rank.saturating_sub(1) as f64) * 0.001;
    score.clamp(0.0, 0.94)
}

fn content_search_best_section(line_number: usize, line_snippet: &str) -> String {
    format!(
        "L{line_number}: {}",
        truncate_content_search_snippet(line_snippet, CODE_CONTENT_SNIPPET_MAX_CHARS)
    )
}

fn truncate_content_search_snippet(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated = value.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn repo_code_source_limit(limit: usize) -> usize {
    limit
        .saturating_mul(3)
        .clamp(DEFAULT_SEARCH_LIMIT, MAX_SEARCH_LIMIT)
}

fn score_with_rank_fallback(score: Option<f64>, rank: usize) -> f64 {
    score.unwrap_or_else(|| (1.0 / rank.max(1) as f64).clamp(0.0, 1.0))
}

fn module_stem(qualified_name: &str) -> String {
    qualified_name
        .rsplit(['.', ':'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(qualified_name)
        .to_string()
}

fn normalize_repo_record_path(path: &str) -> String {
    path.trim().replace('\\', "/")
}

fn repo_navigation_target(repo_id: &str, path: &str) -> StudioNavigationTarget {
    let normalized_path = path.trim().trim_start_matches('/');
    let rooted_path = if normalized_path.is_empty() {
        repo_id.to_string()
    } else if normalized_path.starts_with(&format!("{repo_id}/")) {
        normalized_path.to_string()
    } else {
        format!("{repo_id}/{normalized_path}")
    };

    StudioNavigationTarget {
        path: rooted_path,
        category: "repo_code".to_string(),
        project_name: Some(repo_id.to_string()),
        root_label: Some(repo_id.to_string()),
        line: None,
        line_end: None,
        column: None,
    }
}

fn repo_hierarchy_segments(repo_id: &str, path: &str) -> Option<Vec<String>> {
    let mut segments = vec!["repo".to_string(), repo_id.to_string()];
    for segment in path
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
    {
        segments.push(segment.to_string());
    }
    (!segments.is_empty()).then_some(segments)
}

fn repo_hierarchical_uri(repo_id: &str, path: &str) -> Option<String> {
    let normalized = path.trim().trim_start_matches('/');
    (!normalized.is_empty()).then(|| format!("wendao://repo/{repo_id}/{normalized}"))
}

fn map_repo_backlink_items(
    items: Option<Vec<RepoBacklinkItem>>,
) -> Option<Vec<SearchBacklinkItem>> {
    items.and_then(|items| {
        let mapped = items
            .into_iter()
            .filter_map(|item| {
                let id = item.id.trim().to_string();
                (!id.is_empty()).then_some(SearchBacklinkItem {
                    id,
                    title: item.title,
                    path: item.path,
                    kind: item.kind,
                })
            })
            .collect::<Vec<_>>();
        (!mapped.is_empty()).then_some(mapped)
    })
}

fn normalized_repo_filter(repo: Option<&str>) -> Option<String> {
    repo.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn is_code_search_intent(intent: Option<&str>) -> bool {
    matches!(intent, Some("code_search" | "code"))
}

fn verification_state_from_audit_status(audit_status: Option<&str>) -> Option<&'static str> {
    let normalized = audit_status?.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        None
    } else if normalized.contains("verify")
        || normalized.contains("approved")
        || normalized.contains("pass")
    {
        Some("verified")
    } else if normalized.contains("pending")
        || normalized.contains("review")
        || normalized.contains("triage")
    {
        Some("pending")
    } else if normalized.contains("reject")
        || normalized.contains("fail")
        || normalized.contains("error")
    {
        Some("failed")
    } else {
        None
    }
}

fn map_repo_intelligence_error(error: RepoIntelligenceError) -> StudioApiError {
    match error {
        RepoIntelligenceError::UnknownRepository { repo_id } => StudioApiError::bad_request(
            "UNKNOWN_REPOSITORY",
            format!("Repo Intelligence repository `{repo_id}` is not registered"),
        ),
        RepoIntelligenceError::MissingRequiredPlugin { repo_id, plugin_id } => {
            StudioApiError::bad_request(
                "MISSING_REQUIRED_PLUGIN",
                format!("repo `{repo_id}` requires plugin `{plugin_id}`"),
            )
        }
        RepoIntelligenceError::MissingPlugin { plugin_id } => StudioApiError::bad_request(
            "MISSING_PLUGIN",
            format!("repo intelligence plugin `{plugin_id}` is not registered"),
        ),
        RepoIntelligenceError::MissingRepositoryPath { repo_id } => StudioApiError::bad_request(
            "MISSING_REPOSITORY_PATH",
            format!("repo `{repo_id}` does not declare a local path"),
        ),
        RepoIntelligenceError::MissingRepositorySource { repo_id } => StudioApiError::bad_request(
            "MISSING_REPOSITORY_SOURCE",
            format!("repo `{repo_id}` must declare a local path or upstream url"),
        ),
        RepoIntelligenceError::InvalidRepositoryPath { path, reason, .. } => {
            StudioApiError::bad_request(
                "INVALID_REPOSITORY_PATH",
                format!("invalid repository path `{path}`: {reason}"),
            )
        }
        RepoIntelligenceError::UnsupportedRepositoryLayout { repo_id, message } => {
            StudioApiError::bad_request(
                "UNSUPPORTED_REPOSITORY_LAYOUT",
                format!("repo `{repo_id}` has unsupported layout: {message}"),
            )
        }
        RepoIntelligenceError::UnknownProjectedPage { repo_id, page_id } => {
            StudioApiError::not_found(format!(
                "repo `{repo_id}` does not contain projected page `{page_id}`"
            ))
        }
        RepoIntelligenceError::UnknownProjectedPageFamilyCluster {
            repo_id,
            page_id,
            kind,
        } => StudioApiError::not_found(format!(
            "repo `{repo_id}` does not contain projected page family `{kind:?}` in page `{page_id}`"
        )),
        RepoIntelligenceError::UnknownProjectedPageIndexNode {
            repo_id,
            page_id,
            node_id,
        } => StudioApiError::not_found(format!(
            "repo `{repo_id}` does not contain projected page-index node `{node_id}` in page `{page_id}`"
        )),
        RepoIntelligenceError::ConfigLoad { message } => {
            StudioApiError::bad_request("CONFIG_LOAD_FAILED", message)
        }
        RepoIntelligenceError::DuplicatePlugin { plugin_id } => StudioApiError::internal(
            "DUPLICATE_PLUGIN",
            "Repo intelligence plugin registry is inconsistent",
            Some(format!("duplicate plugin `{plugin_id}`")),
        ),
        RepoIntelligenceError::AnalysisFailed { message } => StudioApiError::internal(
            "REPO_INTELLIGENCE_FAILED",
            "Repo intelligence task failed",
            Some(message),
        ),
    }
}

pub(in crate::gateway::studio) async fn search_attachments(
    Query(query): Query<AttachmentSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<AttachmentSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;
    let limit = query
        .limit
        .unwrap_or(DEFAULT_ATTACHMENT_SEARCH_LIMIT)
        .clamp(1, MAX_ATTACHMENT_SEARCH_LIMIT);
    let kinds = query
        .kind
        .iter()
        .map(|kind| LinkGraphAttachmentKind::from_alias(kind.as_str()))
        .collect::<Vec<_>>();

    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.link_graph_index().await?;
    let hits = index
        .search_attachments(
            raw_query,
            limit,
            query.ext.as_slice(),
            kinds.as_slice(),
            query.case_sensitive,
        )
        .into_iter()
        .filter_map(|hit| {
            let canonical_source_path =
                canonical_graph_path(state.as_ref(), index.as_ref(), hit.source_path.as_str());
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                canonical_source_path.as_str(),
            )
            .then_some((hit, canonical_source_path))
        })
        .map(|(hit, canonical_source_path)| {
            let source_path =
                studio_display_path(state.studio.as_ref(), canonical_source_path.as_str());
            let source_id = hit.source_id;
            let attachment_path = hit.attachment_path;
            AttachmentSearchHit {
                path: source_path.clone(),
                source_id: source_id.clone(),
                source_stem: hit.source_stem,
                source_title: strip_option(hit.source_title.as_str()),
                source_path,
                attachment_id: attachment_id_for(source_id.as_str(), attachment_path.as_str()),
                attachment_path,
                attachment_name: hit.attachment_name,
                attachment_ext: hit.attachment_ext,
                kind: attachment_kind_to_api(hit.kind),
                score: hit.score.max(0.0),
                vision_snippet: hit
                    .vision_snippet
                    .and_then(|value| strip_option(value.as_str())),
            }
        })
        .collect::<Vec<_>>();
    let hit_count = hits.len();

    Ok(Json(AttachmentSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "attachments".to_string(),
    }))
}

pub(in crate::gateway::studio) async fn search_autocomplete(
    Query(query): Query<AutocompleteQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<AutocompleteResponse>, StudioApiError> {
    let prefix = query
        .prefix
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PREFIX", "`prefix` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_AUTOCOMPLETE_LIMIT)
        .clamp(1, MAX_AUTOCOMPLETE_LIMIT);
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.link_graph_index().await?;
    let payload =
        index.search_planned_payload(prefix, limit.max(2), LinkGraphSearchOptions::default());
    let filtered_hits = payload
        .hits
        .into_iter()
        .filter_map(|hit| {
            let canonical_path =
                canonical_graph_path(state.as_ref(), index.as_ref(), hit.path.as_str());
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                canonical_path.as_str(),
            )
            .then(|| {
                let mut hit = hit;
                hit.path = canonical_path;
                hit
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(AutocompleteResponse {
        prefix: prefix.to_string(),
        suggestions: collect_autocomplete_suggestions(prefix, filtered_hits.as_slice(), limit),
    }))
}

pub(in crate::gateway::studio) async fn search_ast(
    Query(query): Query<AstSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<AstSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_AST_SEARCH_LIMIT)
        .clamp(1, MAX_AST_SEARCH_LIMIT);
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.studio.ast_index().await?;
    let mut hits = index
        .iter()
        .filter(|hit| {
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                hit.path.as_str(),
            )
        })
        .filter(|hit| ast_hit_matches(hit, raw_query))
        .map(|hit| {
            let mut hit = hit.clone();
            enrich_ast_hit_project_metadata(
                &mut hit,
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
            );
            hit.score = score_ast_hit(&hit, raw_query);
            hit.navigation_target = ast_navigation_target(&hit);
            hit.path = studio_display_path(state.studio.as_ref(), hit.path.as_str());
            hit.navigation_target.path = hit.path.clone();
            hit
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    hits.truncate(limit);
    let hit_count = hits.len();

    Ok(Json(AstSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "definitions".to_string(),
    }))
}

pub(in crate::gateway::studio) async fn search_definition(
    Query(query): Query<DefinitionResolveQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<DefinitionResolveResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let source_path = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let source_path_candidates = source_path
        .as_deref()
        .map(|path| graph_lookup_candidates(state.studio.as_ref(), path))
        .filter(|candidates| !candidates.is_empty());
    let source_line = query.line.filter(|line| *line > 0);
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.studio.ast_index().await?;
    let markdown_observation_hints = definition_observation_hints(
        state.as_ref(),
        source_path_candidates.as_deref(),
        source_line,
        raw_query,
    )
    .await;
    let mut candidates = resolve_definition_candidates(
        project_root.as_path(),
        config_root.as_path(),
        projects.as_slice(),
        index.as_slice(),
        raw_query,
        DefinitionResolveOptions {
            source_paths: source_path_candidates.as_deref(),
            scope_patterns: markdown_observation_hints
                .as_ref()
                .map(|hints| hints.scope_patterns.as_slice()),
            languages: markdown_observation_hints
                .as_ref()
                .map(|hints| hints.languages.as_slice()),
            ..DefinitionResolveOptions::default()
        },
    );
    for hit in &mut candidates {
        hit.navigation_target = ast_navigation_target(hit);
        hit.path = studio_display_path(state.studio.as_ref(), hit.path.as_str());
        hit.navigation_target.path = hit.path.clone();
    }

    let candidate_count = candidates.len();
    let definition = candidates.into_iter().next().ok_or_else(|| {
        StudioApiError::not_found(format!("No definition found for `{raw_query}`"))
    })?;

    Ok(Json(DefinitionResolveResponse {
        query: raw_query.to_string(),
        source_path,
        source_line,
        navigation_target: definition.navigation_target.clone(),
        definition,
        candidate_count,
        selected_scope: "definition".to_string(),
    }))
}

pub(in crate::gateway::studio) async fn search_symbols(
    Query(query): Query<SymbolSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<SymbolSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_SYMBOL_SEARCH_LIMIT)
        .clamp(1, MAX_SYMBOL_SEARCH_LIMIT);
    let search_window = limit.saturating_mul(4).min(MAX_SYMBOL_SEARCH_LIMIT);
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.studio.symbol_index().await?;
    let mut hits = index
        .search_project(raw_query, search_window)
        .into_iter()
        .map(|symbol| {
            let mut hit = symbol_to_hit(
                symbol,
                raw_query,
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
            );
            hit.path = studio_display_path(state.studio.as_ref(), hit.path.as_str());
            hit.navigation_target.path = hit.path.clone();
            hit
        })
        .filter(|hit| {
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                hit.path.as_str(),
            )
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });
    hits.truncate(limit);
    let hit_count = hits.len();

    Ok(Json(SymbolSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "project".to_string(),
    }))
}

pub(in crate::gateway::studio) async fn search_references(
    Query(query): Query<ReferenceSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<ReferenceSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_REFERENCE_SEARCH_LIMIT)
        .clamp(1, MAX_REFERENCE_SEARCH_LIMIT);
    let ast_index = state.studio.ast_index().await?;
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let worker_project_root = project_root.clone();
    let worker_config_root = config_root.clone();
    let worker_projects = projects.clone();
    let query_owned = raw_query.to_string();
    let ast_hits = ast_index.as_ref().clone();
    let hits = tokio::task::spawn_blocking(move || {
        build_reference_hits(
            worker_project_root.as_path(),
            worker_config_root.as_path(),
            worker_projects.as_slice(),
            ast_hits.as_slice(),
            query_owned.as_str(),
            limit,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REFERENCE_SEARCH_PANIC",
            "Failed to execute studio reference search",
            Some(error.to_string()),
        )
    })?
    .map_err(|error| {
        StudioApiError::internal(
            "REFERENCE_SEARCH_FAILED",
            "Failed to execute studio reference search",
            Some(error),
        )
    })?;
    let mut hits = hits;
    hits.retain(|hit| {
        pathing::path_matches_project_file_filters(
            project_root.as_path(),
            config_root.as_path(),
            projects.as_slice(),
            hit.path.as_str(),
        )
    });
    for hit in &mut hits {
        hit.path = studio_display_path(state.studio.as_ref(), hit.path.as_str());
        hit.navigation_target.path = hit.path.clone();
    }
    let hit_count = hits.len();

    Ok(Json(ReferenceSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "references".to_string(),
    }))
}

fn retrieval_mode_to_string(mode: LinkGraphRetrievalMode) -> String {
    match mode {
        LinkGraphRetrievalMode::GraphOnly => "graph_only".to_string(),
        LinkGraphRetrievalMode::Hybrid => "hybrid".to_string(),
        LinkGraphRetrievalMode::VectorOnly => "vector_only".to_string(),
    }
}

fn attachment_kind_to_api(kind: LinkGraphAttachmentKind) -> AttachmentSearchKind {
    match kind {
        LinkGraphAttachmentKind::Image => AttachmentSearchKind::Image,
        LinkGraphAttachmentKind::Pdf => AttachmentSearchKind::Pdf,
        LinkGraphAttachmentKind::Gpg => AttachmentSearchKind::Gpg,
        LinkGraphAttachmentKind::Document => AttachmentSearchKind::Document,
        LinkGraphAttachmentKind::Archive => AttachmentSearchKind::Archive,
        LinkGraphAttachmentKind::Audio => AttachmentSearchKind::Audio,
        LinkGraphAttachmentKind::Video => AttachmentSearchKind::Video,
        LinkGraphAttachmentKind::Other => AttachmentSearchKind::Other,
    }
}

fn attachment_id_for(source_id: &str, attachment_path: &str) -> String {
    let owner = source_id.trim();
    let owner = if owner.is_empty() { "unknown" } else { owner };
    let normalized_attachment = attachment_path
        .trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();
    if normalized_attachment.is_empty() {
        format!("att://{owner}")
    } else {
        format!("att://{owner}/{normalized_attachment}")
    }
}

fn strip_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalized_intent(intent: Option<&str>) -> Option<String> {
    intent
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn inferred_intent_from_mode(mode: &str) -> Option<String> {
    match mode {
        "graph_only" => Some("graph_navigation".to_string()),
        "vector_only" => Some("semantic_lookup".to_string()),
        "hybrid" => Some("hybrid_search".to_string()),
        _ => None,
    }
}

fn hierarchy_segments(path: &str) -> Option<Vec<String>> {
    let segments = path
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    (!segments.is_empty()).then_some(segments)
}

fn hierarchical_uri_for_path(path: &str) -> Option<String> {
    let normalized = path.trim_start_matches('/').trim();
    (!normalized.is_empty()).then(|| format!("wendao://{normalized}"))
}

fn canonical_graph_path(state: &GatewayState, index: &LinkGraphIndex, raw_path: &str) -> String {
    graph_lookup_candidates(state.studio.as_ref(), raw_path)
        .into_iter()
        .find_map(|candidate| {
            index
                .metadata(candidate.as_str())
                .map(|metadata| metadata.path)
        })
        .unwrap_or_else(|| raw_path.replace('\\', "/"))
}

fn symbol_to_hit(
    symbol: &crate::unified_symbol::UnifiedSymbol,
    query: &str,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> SymbolSearchHit {
    let (path, line) = split_location(symbol.location.as_str());
    let metadata = project_metadata_for_path(project_root, config_root, projects, path.as_str());
    let navigation_target = symbol_navigation_target(
        path.as_str(),
        symbol.crate_or_local(),
        metadata.project_name.as_deref(),
        metadata.root_label.as_deref(),
        line,
    );

    SymbolSearchHit {
        name: symbol.name.clone(),
        kind: symbol.kind.clone(),
        path: path.clone(),
        line,
        location: symbol.location.clone(),
        language: source_language_label(Path::new(path.as_str()))
            .unwrap_or("unknown")
            .to_string(),
        crate_name: symbol.crate_or_local().to_string(),
        project_name: metadata.project_name,
        root_label: metadata.root_label,
        navigation_target,
        source: if symbol.is_project() {
            SymbolSearchSource::Project
        } else {
            SymbolSearchSource::External
        },
        score: score_symbol(symbol.name.as_str(), path.as_str(), query),
    }
}

fn ast_navigation_target(hit: &AstSearchHit) -> StudioNavigationTarget {
    StudioNavigationTarget {
        path: hit.path.clone(),
        category: "doc".to_string(),
        project_name: hit
            .project_name
            .clone()
            .or_else(|| Some(hit.crate_name.clone())),
        root_label: hit.root_label.clone(),
        line: Some(hit.line_start),
        line_end: Some(hit.line_end),
        column: None,
    }
}

fn symbol_navigation_target(
    path: &str,
    crate_name: &str,
    project_name: Option<&str>,
    root_label: Option<&str>,
    line: usize,
) -> StudioNavigationTarget {
    StudioNavigationTarget {
        path: path.to_string(),
        category: "doc".to_string(),
        project_name: project_name
            .map(ToString::to_string)
            .or_else(|| Some(crate_name.to_string())),
        root_label: root_label.map(ToString::to_string),
        line: Some(line),
        line_end: Some(line),
        column: None,
    }
}

struct AutocompleteCollector<'a> {
    suggestions: Vec<AutocompleteSuggestion>,
    seen: HashSet<String>,
    prefix_lc: &'a str,
    limit: usize,
}

fn split_location(location: &str) -> (String, usize) {
    match location.rsplit_once(':') {
        Some((path, line)) => (
            path.to_string(),
            line.parse::<usize>().unwrap_or_default().max(1),
        ),
        None => (location.to_string(), 1),
    }
}

fn score_symbol(name: &str, path: &str, query: &str) -> f64 {
    let name_lc = name.to_ascii_lowercase();
    let path_lc = path.to_ascii_lowercase();
    let query_lc = query.to_ascii_lowercase();

    if name_lc == query_lc {
        1.0
    } else if name_lc.starts_with(query_lc.as_str()) {
        0.95
    } else if name_lc.contains(query_lc.as_str()) {
        0.88
    } else if path_lc.contains(query_lc.as_str()) {
        0.72
    } else {
        0.5
    }
}

impl<'a> AutocompleteCollector<'a> {
    fn new(prefix_lc: &'a str, limit: usize) -> Self {
        Self {
            suggestions: Vec::with_capacity(limit),
            seen: HashSet::new(),
            prefix_lc,
            limit,
        }
    }

    fn add(
        &mut self,
        text: &str,
        path: &str,
        doc_type: Option<&str>,
        suggestion_type: AutocompleteSuggestionType,
    ) {
        if self.suggestions.len() >= self.limit {
            return;
        }

        let normalized_text = text.trim();
        if normalized_text.is_empty()
            || !normalized_text
                .to_ascii_lowercase()
                .starts_with(self.prefix_lc)
        {
            return;
        }

        let key = format!("{suggestion_type:?}|{normalized_text}|{path}");
        if !self.seen.insert(key) {
            return;
        }

        self.suggestions.push(AutocompleteSuggestion {
            text: normalized_text.to_string(),
            suggestion_type,
            path: Some(path.to_string()),
            doc_type: doc_type.map(ToString::to_string),
        });
    }
}

fn collect_autocomplete_suggestions(
    prefix: &str,
    hits: &[LinkGraphDisplayHit],
    limit: usize,
) -> Vec<AutocompleteSuggestion> {
    let prefix_lc = prefix.to_ascii_lowercase();
    let mut collector = AutocompleteCollector::new(&prefix_lc, limit);

    for hit in hits {
        collector.add(
            &hit.stem,
            hit.path.as_str(),
            hit.doc_type.as_deref(),
            AutocompleteSuggestionType::Stem,
        );

        if !hit.title.is_empty() {
            collector.add(
                &hit.title,
                hit.path.as_str(),
                hit.doc_type.as_deref(),
                AutocompleteSuggestionType::Title,
            );
        }

        for tag in &hit.tags {
            collector.add(
                tag,
                hit.path.as_str(),
                hit.doc_type.as_deref(),
                AutocompleteSuggestionType::Tag,
            );
        }

        if collector.suggestions.len() >= limit {
            break;
        }
    }

    collector.suggestions
}

#[cfg(test)]
#[path = "../../../../tests/unit/gateway/studio/search.rs"]
mod tests;
