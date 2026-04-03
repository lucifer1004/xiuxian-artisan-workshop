use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use xiuxian_vector::{
    LanceBooleanArray, LanceDataType, LanceField, LanceFloat64Array, LanceInt32Array,
    LanceRecordBatch, LanceSchema, LanceStringArray as StringArray,
};

use crate::transport::{
    AnalysisFlightRouteResponse, AstSearchFlightRouteProvider, AttachmentSearchFlightRouteProvider,
    AutocompleteFlightRouteProvider, AutocompleteFlightRouteResponse,
    CodeAstAnalysisFlightRouteProvider, DefinitionFlightRouteProvider,
    DefinitionFlightRouteResponse, GraphNeighborsFlightRouteProvider,
    GraphNeighborsFlightRouteResponse, MarkdownAnalysisFlightRouteProvider,
    RepoSearchFlightRequest, RepoSearchFlightRouteProvider, SearchFlightRouteProvider,
    SearchFlightRouteResponse, SqlFlightRouteProvider, SqlFlightRouteResponse,
    VfsResolveFlightRouteProvider, VfsResolveFlightRouteResponse,
};

type SearchRequestRecord = (String, String, usize, Option<String>, Option<String>);
type DefinitionRequestRecord = (String, Option<String>, Option<usize>);
type AutocompleteRequestRecord = (String, usize);
type GraphNeighborsRequestRecord = (String, String, usize, usize);
type AttachmentSearchRequestRecord = (String, usize, Vec<String>, Vec<String>, bool);
type AstSearchRequestRecord = (String, usize);
type CodeAstAnalysisRequestRecord = (String, String, Option<usize>);

fn lock_or_panic<'a, T>(mutex: &'a Mutex<T>, context: &str) -> std::sync::MutexGuard<'a, T> {
    mutex.lock().unwrap_or_else(|_| panic!("{context}"))
}

#[derive(Debug)]
pub(super) struct RecordingRepoSearchProvider;

#[async_trait]
impl RepoSearchFlightRouteProvider for RecordingRepoSearchProvider {
    async fn repo_search_batch(
        &self,
        request: &RepoSearchFlightRequest,
    ) -> Result<LanceRecordBatch, String> {
        LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("doc_id", LanceDataType::Utf8, false),
                LanceField::new("path", LanceDataType::Utf8, false),
                LanceField::new("title", LanceDataType::Utf8, false),
                LanceField::new("score", LanceDataType::Float64, false),
                LanceField::new("language", LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![format!(
                    "doc:{}:{}",
                    request.query_text, request.limit
                )])),
                Arc::new(StringArray::from(vec!["src/lib.rs"])),
                Arc::new(StringArray::from(vec!["Repo Search Result"])),
                Arc::new(LanceFloat64Array::from(vec![0.91_f64])),
                Arc::new(StringArray::from(vec!["rust"])),
            ],
        )
        .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingSearchProvider {
    request: Mutex<Option<SearchRequestRecord>>,
    call_count: Mutex<usize>,
}

impl RecordingSearchProvider {
    pub(super) fn recorded_request(&self) -> Option<SearchRequestRecord> {
        lock_or_panic(&self.request, "search-family provider record should lock").clone()
    }

    pub(super) fn call_count(&self) -> usize {
        *lock_or_panic(
            &self.call_count,
            "search-family provider call count should lock",
        )
    }
}

#[async_trait]
impl SearchFlightRouteProvider for RecordingSearchProvider {
    async fn search_batch(
        &self,
        route: &str,
        query_text: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Result<SearchFlightRouteResponse, String> {
        *lock_or_panic(&self.request, "search-family provider record should lock") = Some((
            route.to_string(),
            query_text.to_string(),
            limit,
            intent.map(ToString::to_string),
            repo_hint.map(ToString::to_string),
        ));
        *lock_or_panic(
            &self.call_count,
            "search-family provider call count should lock",
        ) += 1;
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("doc_id", LanceDataType::Utf8, false),
                LanceField::new("route", LanceDataType::Utf8, false),
                LanceField::new("query_text", LanceDataType::Utf8, false),
                LanceField::new("score", LanceDataType::Float64, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![format!(
                    "{route}:{query_text}:{limit}"
                )])),
                Arc::new(StringArray::from(vec![route.to_string()])),
                Arc::new(StringArray::from(vec![query_text.to_string()])),
                Arc::new(LanceFloat64Array::from(vec![0.99_f64])),
            ],
        )
        .map_err(|error| error.to_string())?;
        Ok(SearchFlightRouteResponse::new(batch).with_app_metadata(
            serde_json::json!({
                "query": query_text,
                "hitCount": 1,
                "selectedMode": route,
                "intent": intent,
                "repoHint": repo_hint,
            })
            .to_string()
            .into_bytes(),
        ))
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingDefinitionProvider {
    request: Mutex<Option<DefinitionRequestRecord>>,
    call_count: Mutex<usize>,
}

impl RecordingDefinitionProvider {
    pub(super) fn recorded_request(&self) -> Option<DefinitionRequestRecord> {
        lock_or_panic(&self.request, "definition provider record should lock").clone()
    }

    pub(super) fn call_count(&self) -> usize {
        *lock_or_panic(
            &self.call_count,
            "definition provider call count should lock",
        )
    }
}

#[async_trait]
impl DefinitionFlightRouteProvider for RecordingDefinitionProvider {
    async fn definition_batch(
        &self,
        query_text: &str,
        source_path: Option<&str>,
        source_line: Option<usize>,
    ) -> Result<DefinitionFlightRouteResponse, tonic::Status> {
        *lock_or_panic(&self.request, "definition provider record should lock") = Some((
            query_text.to_string(),
            source_path.map(ToString::to_string),
            source_line,
        ));
        *lock_or_panic(
            &self.call_count,
            "definition provider call count should lock",
        ) += 1;
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("name", LanceDataType::Utf8, false),
                LanceField::new("path", LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![query_text.to_string()])),
                Arc::new(StringArray::from(vec![
                    source_path.unwrap_or("src/lib.rs").to_string(),
                ])),
            ],
        )
        .map_err(|error| tonic::Status::internal(error.to_string()))?;
        Ok(DefinitionFlightRouteResponse::new(batch).with_app_metadata(
            serde_json::json!({
                "query": query_text,
                "sourcePath": source_path,
                "sourceLine": source_line,
                "candidateCount": 1,
                "selectedScope": "definition",
            })
            .to_string()
            .into_bytes(),
        ))
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingAutocompleteProvider {
    request: Mutex<Option<AutocompleteRequestRecord>>,
    call_count: Mutex<usize>,
}

impl RecordingAutocompleteProvider {
    pub(super) fn recorded_request(&self) -> Option<AutocompleteRequestRecord> {
        lock_or_panic(&self.request, "autocomplete provider record should lock").clone()
    }

    pub(super) fn call_count(&self) -> usize {
        *lock_or_panic(
            &self.call_count,
            "autocomplete provider call count should lock",
        )
    }
}

#[async_trait]
impl AutocompleteFlightRouteProvider for RecordingAutocompleteProvider {
    async fn autocomplete_batch(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<AutocompleteFlightRouteResponse, tonic::Status> {
        *lock_or_panic(&self.request, "autocomplete provider record should lock") =
            Some((prefix.to_string(), limit));
        *lock_or_panic(
            &self.call_count,
            "autocomplete provider call count should lock",
        ) += 1;
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("text", LanceDataType::Utf8, false),
                LanceField::new("suggestionType", LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![format!("{prefix}_suggestion")])),
                Arc::new(StringArray::from(vec!["symbol"])),
            ],
        )
        .map_err(|error| tonic::Status::internal(error.to_string()))?;
        Ok(
            AutocompleteFlightRouteResponse::new(batch).with_app_metadata(
                serde_json::json!({
                    "prefix": prefix,
                })
                .to_string()
                .into_bytes(),
            ),
        )
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingSqlProvider {
    request: Mutex<Option<String>>,
    call_count: Mutex<usize>,
}

impl RecordingSqlProvider {
    pub(super) fn call_count(&self) -> usize {
        *lock_or_panic(&self.call_count, "SQL provider call count should lock")
    }
}

#[async_trait]
impl SqlFlightRouteProvider for RecordingSqlProvider {
    async fn sql_query_batches(&self, query_text: &str) -> Result<SqlFlightRouteResponse, String> {
        *lock_or_panic(&self.request, "SQL provider record should lock") =
            Some(query_text.to_string());
        *lock_or_panic(&self.call_count, "SQL provider call count should lock") += 1;
        let schema = Arc::new(LanceSchema::new(vec![
            LanceField::new("table_name", LanceDataType::Utf8, false),
            LanceField::new("row_id", LanceDataType::Int32, false),
        ]));
        let first_batch = LanceRecordBatch::try_new(
            Arc::clone(&schema),
            vec![
                Arc::new(StringArray::from(vec!["repo_entity"])),
                Arc::new(LanceInt32Array::from(vec![1])),
            ],
        )
        .map_err(|error| error.to_string())?;
        let second_batch = LanceRecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["repo_content_chunk"])),
                Arc::new(LanceInt32Array::from(vec![2])),
            ],
        )
        .map_err(|error| error.to_string())?;
        Ok(
            SqlFlightRouteResponse::new(vec![first_batch, second_batch]).with_app_metadata(
                serde_json::json!({
                    "query": query_text,
                    "batchCount": 2,
                    "registeredTables": ["repo_entity", "repo_content_chunk"],
                })
                .to_string()
                .into_bytes(),
            ),
        )
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingVfsResolveProvider {
    request: Mutex<Option<String>>,
    call_count: Mutex<usize>,
}

impl RecordingVfsResolveProvider {
    pub(super) fn recorded_request(&self) -> Option<String> {
        lock_or_panic(&self.request, "VFS resolve provider record should lock").clone()
    }

    pub(super) fn call_count(&self) -> usize {
        *lock_or_panic(
            &self.call_count,
            "VFS resolve provider call count should lock",
        )
    }
}

#[async_trait]
impl VfsResolveFlightRouteProvider for RecordingVfsResolveProvider {
    async fn resolve_vfs_navigation_batch(
        &self,
        path: &str,
    ) -> Result<VfsResolveFlightRouteResponse, tonic::Status> {
        *lock_or_panic(&self.request, "VFS resolve provider record should lock") =
            Some(path.to_string());
        *lock_or_panic(
            &self.call_count,
            "VFS resolve provider call count should lock",
        ) += 1;
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("path", LanceDataType::Utf8, false),
                LanceField::new("category", LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![path.to_string()])),
                Arc::new(StringArray::from(vec!["file".to_string()])),
            ],
        )
        .map_err(|error| tonic::Status::internal(error.to_string()))?;
        Ok(VfsResolveFlightRouteResponse::new(batch).with_app_metadata(
            serde_json::json!({
                "path": path,
                "navigationTarget": {
                    "path": path,
                    "category": "file",
                },
            })
            .to_string()
            .into_bytes(),
        ))
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingGraphNeighborsProvider {
    request: Mutex<Option<GraphNeighborsRequestRecord>>,
    call_count: Mutex<usize>,
}

impl RecordingGraphNeighborsProvider {
    pub(super) fn recorded_request(&self) -> Option<GraphNeighborsRequestRecord> {
        lock_or_panic(&self.request, "graph-neighbors provider record should lock").clone()
    }

    pub(super) fn call_count(&self) -> usize {
        *lock_or_panic(
            &self.call_count,
            "graph-neighbors provider call count should lock",
        )
    }
}

#[async_trait]
impl GraphNeighborsFlightRouteProvider for RecordingGraphNeighborsProvider {
    async fn graph_neighbors_batch(
        &self,
        node_id: &str,
        direction: &str,
        hops: usize,
        limit: usize,
    ) -> Result<GraphNeighborsFlightRouteResponse, tonic::Status> {
        *lock_or_panic(&self.request, "graph-neighbors provider record should lock") =
            Some((node_id.to_string(), direction.to_string(), hops, limit));
        *lock_or_panic(
            &self.call_count,
            "graph-neighbors provider call count should lock",
        ) += 1;
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("rowType", LanceDataType::Utf8, false),
                LanceField::new("nodeId", LanceDataType::Utf8, true),
                LanceField::new("nodeLabel", LanceDataType::Utf8, true),
                LanceField::new("nodePath", LanceDataType::Utf8, true),
                LanceField::new("nodeType", LanceDataType::Utf8, true),
                LanceField::new("nodeIsCenter", LanceDataType::Boolean, true),
                LanceField::new("nodeDistance", LanceDataType::Int32, true),
                LanceField::new("navigationPath", LanceDataType::Utf8, true),
                LanceField::new("navigationCategory", LanceDataType::Utf8, true),
                LanceField::new("navigationProjectName", LanceDataType::Utf8, true),
                LanceField::new("navigationRootLabel", LanceDataType::Utf8, true),
                LanceField::new("navigationLine", LanceDataType::Int32, true),
                LanceField::new("navigationLineEnd", LanceDataType::Int32, true),
                LanceField::new("navigationColumn", LanceDataType::Int32, true),
                LanceField::new("linkSource", LanceDataType::Utf8, true),
                LanceField::new("linkTarget", LanceDataType::Utf8, true),
                LanceField::new("linkDirection", LanceDataType::Utf8, true),
                LanceField::new("linkDistance", LanceDataType::Int32, true),
            ])),
            vec![
                Arc::new(StringArray::from(vec!["node", "link"])),
                Arc::new(StringArray::from(vec![Some(node_id.to_string()), None])),
                Arc::new(StringArray::from(vec![Some("Index".to_string()), None])),
                Arc::new(StringArray::from(vec![Some(node_id.to_string()), None])),
                Arc::new(StringArray::from(vec![Some("doc".to_string()), None])),
                Arc::new(LanceBooleanArray::from(vec![Some(true), None])),
                Arc::new(LanceInt32Array::from(vec![Some(0), None])),
                Arc::new(StringArray::from(vec![Some(node_id.to_string()), None])),
                Arc::new(StringArray::from(vec![Some("doc".to_string()), None])),
                Arc::new(StringArray::from(vec![Some("kernel".to_string()), None])),
                Arc::new(StringArray::from(vec![Some("project".to_string()), None])),
                Arc::new(LanceInt32Array::from(vec![Some(7), None])),
                Arc::new(LanceInt32Array::from(vec![Some(9), None])),
                Arc::new(LanceInt32Array::from(vec![Some(3), None])),
                Arc::new(StringArray::from(vec![None, Some(node_id.to_string())])),
                Arc::new(StringArray::from(vec![
                    None,
                    Some(format!("{node_id}::neighbor")),
                ])),
                Arc::new(StringArray::from(vec![None, Some(direction.to_string())])),
                Arc::new(LanceInt32Array::from(vec![
                    None,
                    Some(i32::try_from(hops.min(limit)).unwrap_or(i32::MAX)),
                ])),
            ],
        )
        .map_err(|error| tonic::Status::internal(error.to_string()))?;
        Ok(GraphNeighborsFlightRouteResponse::new(batch))
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingAttachmentSearchProvider {
    request: Mutex<Option<AttachmentSearchRequestRecord>>,
}

impl RecordingAttachmentSearchProvider {
    pub(super) fn recorded_request(&self) -> Option<AttachmentSearchRequestRecord> {
        lock_or_panic(
            &self.request,
            "attachment-search provider record should lock",
        )
        .clone()
    }
}

#[async_trait]
impl AttachmentSearchFlightRouteProvider for RecordingAttachmentSearchProvider {
    async fn attachment_search_batch(
        &self,
        query_text: &str,
        limit: usize,
        ext_filters: &std::collections::HashSet<String>,
        kind_filters: &std::collections::HashSet<String>,
        case_sensitive: bool,
    ) -> Result<LanceRecordBatch, String> {
        let mut ext_filters = ext_filters.iter().cloned().collect::<Vec<_>>();
        ext_filters.sort();
        let mut kind_filters = kind_filters.iter().cloned().collect::<Vec<_>>();
        kind_filters.sort();
        *lock_or_panic(
            &self.request,
            "attachment-search provider record should lock",
        ) = Some((
            query_text.to_string(),
            limit,
            ext_filters,
            kind_filters,
            case_sensitive,
        ));
        LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("doc_id", LanceDataType::Utf8, false),
                LanceField::new("query_text", LanceDataType::Utf8, false),
                LanceField::new("score", LanceDataType::Float64, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![format!(
                    "attachment:{query_text}:{limit}"
                )])),
                Arc::new(StringArray::from(vec![query_text.to_string()])),
                Arc::new(LanceFloat64Array::from(vec![0.77_f64])),
            ],
        )
        .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingAstSearchProvider {
    request: Mutex<Option<AstSearchRequestRecord>>,
}

impl RecordingAstSearchProvider {
    pub(super) fn recorded_request(&self) -> Option<AstSearchRequestRecord> {
        lock_or_panic(&self.request, "AST-search provider record should lock").clone()
    }
}

#[async_trait]
impl AstSearchFlightRouteProvider for RecordingAstSearchProvider {
    async fn ast_search_batch(
        &self,
        query_text: &str,
        limit: usize,
    ) -> Result<LanceRecordBatch, String> {
        *lock_or_panic(&self.request, "AST-search provider record should lock") =
            Some((query_text.to_string(), limit));
        LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("doc_id", LanceDataType::Utf8, false),
                LanceField::new("query_text", LanceDataType::Utf8, false),
                LanceField::new("score", LanceDataType::Float64, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![format!("ast:{query_text}:{limit}")])),
                Arc::new(StringArray::from(vec![query_text.to_string()])),
                Arc::new(LanceFloat64Array::from(vec![0.81_f64])),
            ],
        )
        .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingMarkdownAnalysisProvider {
    request: Mutex<Option<String>>,
}

impl RecordingMarkdownAnalysisProvider {
    pub(super) fn recorded_request(&self) -> Option<String> {
        lock_or_panic(
            &self.request,
            "markdown analysis provider record should lock",
        )
        .clone()
    }
}

#[async_trait]
impl MarkdownAnalysisFlightRouteProvider for RecordingMarkdownAnalysisProvider {
    async fn markdown_analysis_batch(
        &self,
        path: &str,
    ) -> Result<AnalysisFlightRouteResponse, String> {
        *lock_or_panic(
            &self.request,
            "markdown analysis provider record should lock",
        ) = Some(path.to_string());
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("ownerId", LanceDataType::Utf8, false),
                LanceField::new("chunkId", LanceDataType::Utf8, false),
                LanceField::new("semanticType", LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![format!("markdown:{path}")])),
                Arc::new(StringArray::from(vec!["chunk:0"])),
                Arc::new(StringArray::from(vec!["section"])),
            ],
        )
        .map_err(|error| error.to_string())?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(
            serde_json::json!({
                "path": path,
                "documentHash": "fp:markdown",
                "nodeCount": 1,
                "edgeCount": 0,
                "nodes": [],
                "edges": [],
                "projections": [],
                "diagnostics": [],
            })
            .to_string()
            .into_bytes(),
        ))
    }
}

#[derive(Debug, Default)]
pub(super) struct RecordingCodeAstAnalysisProvider {
    request: Mutex<Option<CodeAstAnalysisRequestRecord>>,
}

impl RecordingCodeAstAnalysisProvider {
    pub(super) fn recorded_request(&self) -> Option<CodeAstAnalysisRequestRecord> {
        lock_or_panic(
            &self.request,
            "code-AST analysis provider record should lock",
        )
        .clone()
    }
}

#[async_trait]
impl CodeAstAnalysisFlightRouteProvider for RecordingCodeAstAnalysisProvider {
    async fn code_ast_analysis_batch(
        &self,
        path: &str,
        repo_id: &str,
        line_hint: Option<usize>,
    ) -> Result<AnalysisFlightRouteResponse, String> {
        *lock_or_panic(
            &self.request,
            "code-AST analysis provider record should lock",
        ) = Some((path.to_string(), repo_id.to_string(), line_hint));
        let batch = LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("ownerId", LanceDataType::Utf8, false),
                LanceField::new("chunkId", LanceDataType::Utf8, false),
                LanceField::new("semanticType", LanceDataType::Utf8, false),
            ])),
            vec![
                Arc::new(StringArray::from(vec![format!(
                    "code-ast:{repo_id}:{path}"
                )])),
                Arc::new(StringArray::from(vec!["chunk:0"])),
                Arc::new(StringArray::from(vec!["declaration"])),
            ],
        )
        .map_err(|error| error.to_string())?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(
            serde_json::json!({
                "repoId": repo_id,
                "path": path,
                "language": "julia",
                "nodeCount": 1,
                "edgeCount": 0,
                "nodes": [],
                "edges": [],
                "projections": [],
                "focusNodeId": line_hint.map(|line| format!("line:{line}")),
                "diagnostics": [],
            })
            .to_string()
            .into_bytes(),
        ))
    }
}
