use std::sync::Arc;

use arrow::array::{ArrayRef, Float64Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use axum::Json;
use axum::extract::{Query, State};
use axum::response::Response;

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::search::handlers::arrow_transport::{
    arrow_payload_response, build_arrow_search_ipc, encode_json,
};
use crate::gateway::studio::types::{ReferenceSearchHit, ReferenceSearchResponse};

use super::queries::ReferenceSearchQuery;

pub async fn search_references(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<ReferenceSearchQuery>,
) -> Result<Json<ReferenceSearchResponse>, StudioApiError> {
    let response = load_reference_search_response(state.as_ref(), query).await?;
    Ok(Json(response))
}

pub async fn search_references_hits_arrow(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<ReferenceSearchQuery>,
) -> Result<Response, StudioApiError> {
    let response = load_reference_search_response(state.as_ref(), query).await?;
    reference_hits_arrow_response(&response.hits).map_err(|error| {
        StudioApiError::internal("SEARCH_REFERENCE_HITS_ARROW_FAILED", error, None)
    })
}

pub(crate) async fn load_reference_search_response(
    state: &GatewayState,
    query: ReferenceSearchQuery,
) -> Result<ReferenceSearchResponse, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Reference search requires a non-empty query",
        ));
    }
    state
        .studio
        .ensure_reference_occurrence_index_ready()
        .await?;
    let hits = state
        .studio
        .search_reference_occurrences(query_text, query.limit.unwrap_or(20).max(1))
        .await?;

    Ok(ReferenceSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "references".to_string(),
    })
}

pub(crate) fn reference_hits_arrow_response(
    hits: &[ReferenceSearchHit],
) -> Result<Response, String> {
    encode_reference_hits_ipc(hits).map(arrow_payload_response)
}

fn encode_reference_hits_ipc(hits: &[ReferenceSearchHit]) -> Result<Vec<u8>, String> {
    let names: Vec<&str> = hits.iter().map(|hit| hit.name.as_str()).collect();
    let paths: Vec<&str> = hits.iter().map(|hit| hit.path.as_str()).collect();
    let languages: Vec<&str> = hits.iter().map(|hit| hit.language.as_str()).collect();
    let crate_names: Vec<&str> = hits.iter().map(|hit| hit.crate_name.as_str()).collect();
    let project_names: Vec<Option<&str>> =
        hits.iter().map(|hit| hit.project_name.as_deref()).collect();
    let root_labels: Vec<Option<&str>> = hits.iter().map(|hit| hit.root_label.as_deref()).collect();
    let navigation_targets_json: Vec<String> = hits
        .iter()
        .map(|hit| encode_json(&hit.navigation_target))
        .collect::<Result<_, _>>()?;
    let line_texts: Vec<&str> = hits.iter().map(|hit| hit.line_text.as_str()).collect();

    let schema = Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("path", DataType::Utf8, false),
        Field::new("language", DataType::Utf8, false),
        Field::new("crateName", DataType::Utf8, false),
        Field::new("projectName", DataType::Utf8, true),
        Field::new("rootLabel", DataType::Utf8, true),
        Field::new("navigationTargetJson", DataType::Utf8, false),
        Field::new("line", DataType::UInt64, false),
        Field::new("column", DataType::UInt64, false),
        Field::new("lineText", DataType::Utf8, false),
        Field::new("score", DataType::Float64, false),
    ]);

    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(names)),
        Arc::new(StringArray::from(paths)),
        Arc::new(StringArray::from(languages)),
        Arc::new(StringArray::from(crate_names)),
        Arc::new(StringArray::from(project_names)),
        Arc::new(StringArray::from(root_labels)),
        Arc::new(StringArray::from(
            navigation_targets_json
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )),
        Arc::new(UInt64Array::from(
            hits.iter().map(|hit| hit.line as u64).collect::<Vec<_>>(),
        )),
        Arc::new(UInt64Array::from(
            hits.iter().map(|hit| hit.column as u64).collect::<Vec<_>>(),
        )),
        Arc::new(StringArray::from(line_texts)),
        Arc::new(Float64Array::from(
            hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
        )),
    ];
    build_arrow_search_ipc(schema, columns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use crate::gateway::studio::types::StudioNavigationTarget;
    use arrow::ipc::reader::StreamReader;

    #[test]
    fn search_reference_arrow_roundtrip_preserves_navigation_target() {
        let hits = vec![ReferenceSearchHit {
            name: "solve".to_string(),
            path: "src/pkg.jl".to_string(),
            language: "julia".to_string(),
            crate_name: "pkg".to_string(),
            project_name: Some("pkg".to_string()),
            root_label: Some("pkg".to_string()),
            navigation_target: StudioNavigationTarget {
                path: "pkg/src/pkg.jl".to_string(),
                category: "repo_code".to_string(),
                project_name: Some("pkg".to_string()),
                root_label: Some("pkg".to_string()),
                line: Some(42),
                line_end: Some(42),
                column: Some(5),
            },
            line: 42,
            column: 5,
            line_text: "solve(x)".to_string(),
            score: 0.87,
        }];

        let encoded =
            encode_reference_hits_ipc(&hits).expect("reference hit arrow encoding should succeed");
        let reader = StreamReader::try_new(Cursor::new(encoded), None)
            .expect("reference hit stream reader should open");
        let batches = reader
            .collect::<Result<Vec<_>, _>>()
            .expect("reference hit stream should decode");
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert_eq!(batch.num_rows(), 1);
        assert!(batch.column_by_name("navigationTargetJson").is_some());
        assert!(batch.column_by_name("lineText").is_some());
    }
}
