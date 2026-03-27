use std::sync::Arc;

use arrow::array::{ArrayRef, Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use axum::extract::{Query, State};
use axum::response::Response;

use super::entry::load_intent_search_response;
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::search::handlers::arrow_transport::{
    arrow_payload_response, build_arrow_search_ipc, encode_json, encode_optional_json,
};
use crate::gateway::studio::search::handlers::queries::SearchQuery;
use crate::gateway::studio::types::SearchHit;

pub async fn search_intent_hits_arrow(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Response, StudioApiError> {
    let response = load_intent_search_response(state.studio.as_ref(), query).await?;
    search_hits_arrow_response(&response.hits)
        .map_err(|error| StudioApiError::internal("SEARCH_HITS_ARROW_FAILED", error, None))
}

pub(crate) fn search_hits_arrow_response(hits: &[SearchHit]) -> Result<Response, String> {
    encode_search_hits_ipc(hits).map(arrow_payload_response)
}

fn encode_search_hits_ipc(hits: &[SearchHit]) -> Result<Vec<u8>, String> {
    let stems: Vec<&str> = hits.iter().map(|hit| hit.stem.as_str()).collect();
    let titles: Vec<Option<&str>> = hits.iter().map(|hit| hit.title.as_deref()).collect();
    let paths: Vec<&str> = hits.iter().map(|hit| hit.path.as_str()).collect();
    let doc_types: Vec<Option<&str>> = hits.iter().map(|hit| hit.doc_type.as_deref()).collect();
    let tags_json: Vec<String> = hits
        .iter()
        .map(|hit| encode_json(&hit.tags))
        .collect::<Result<_, _>>()?;
    let best_sections: Vec<Option<&str>> =
        hits.iter().map(|hit| hit.best_section.as_deref()).collect();
    let match_reasons: Vec<Option<&str>> =
        hits.iter().map(|hit| hit.match_reason.as_deref()).collect();
    let hierarchical_uris: Vec<Option<&str>> = hits
        .iter()
        .map(|hit| hit.hierarchical_uri.as_deref())
        .collect();
    let hierarchy_json: Vec<Option<String>> = hits
        .iter()
        .map(|hit| encode_optional_json(hit.hierarchy.as_ref()))
        .collect::<Result<_, _>>()?;
    let saliency_scores: Vec<Option<f64>> = hits.iter().map(|hit| hit.saliency_score).collect();
    let audit_statuses: Vec<Option<&str>> =
        hits.iter().map(|hit| hit.audit_status.as_deref()).collect();
    let verification_states: Vec<Option<&str>> = hits
        .iter()
        .map(|hit| hit.verification_state.as_deref())
        .collect();
    let implicit_backlinks_json: Vec<Option<String>> = hits
        .iter()
        .map(|hit| encode_optional_json(hit.implicit_backlinks.as_ref()))
        .collect::<Result<_, _>>()?;
    let implicit_backlink_items_json: Vec<Option<String>> = hits
        .iter()
        .map(|hit| encode_optional_json(hit.implicit_backlink_items.as_ref()))
        .collect::<Result<_, _>>()?;
    let navigation_targets_json: Vec<Option<String>> = hits
        .iter()
        .map(|hit| encode_optional_json(hit.navigation_target.as_ref()))
        .collect::<Result<_, _>>()?;

    let schema = Schema::new(vec![
        Field::new("stem", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, true),
        Field::new("path", DataType::Utf8, false),
        Field::new("docType", DataType::Utf8, true),
        Field::new("tagsJson", DataType::Utf8, false),
        Field::new("score", DataType::Float64, false),
        Field::new("bestSection", DataType::Utf8, true),
        Field::new("matchReason", DataType::Utf8, true),
        Field::new("hierarchicalUri", DataType::Utf8, true),
        Field::new("hierarchyJson", DataType::Utf8, true),
        Field::new("saliencyScore", DataType::Float64, true),
        Field::new("auditStatus", DataType::Utf8, true),
        Field::new("verificationState", DataType::Utf8, true),
        Field::new("implicitBacklinksJson", DataType::Utf8, true),
        Field::new("implicitBacklinkItemsJson", DataType::Utf8, true),
        Field::new("navigationTargetJson", DataType::Utf8, true),
    ]);

    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(stems)),
        Arc::new(StringArray::from(titles)),
        Arc::new(StringArray::from(paths)),
        Arc::new(StringArray::from(doc_types)),
        Arc::new(StringArray::from(
            tags_json.iter().map(String::as_str).collect::<Vec<_>>(),
        )),
        Arc::new(Float64Array::from(
            hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
        )),
        Arc::new(StringArray::from(best_sections)),
        Arc::new(StringArray::from(match_reasons)),
        Arc::new(StringArray::from(hierarchical_uris)),
        Arc::new(StringArray::from(hierarchy_json)),
        Arc::new(Float64Array::from(saliency_scores)),
        Arc::new(StringArray::from(audit_statuses)),
        Arc::new(StringArray::from(verification_states)),
        Arc::new(StringArray::from(implicit_backlinks_json)),
        Arc::new(StringArray::from(implicit_backlink_items_json)),
        Arc::new(StringArray::from(navigation_targets_json)),
    ];
    build_arrow_search_ipc(schema, columns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use crate::gateway::studio::types::{SearchBacklinkItem, StudioNavigationTarget};
    use arrow::ipc::reader::StreamReader;

    #[test]
    fn search_hit_arrow_roundtrip_preserves_nested_fields() {
        let hits = vec![SearchHit {
            stem: "BaseModelica".to_string(),
            title: Some("BaseModelicaPackage".to_string()),
            path: "src/BaseModelica.jl".to_string(),
            doc_type: Some("symbol".to_string()),
            tags: vec!["code".to_string(), "kind:function".to_string()],
            score: 0.92,
            best_section: Some("BaseModelicaPackage(input)".to_string()),
            match_reason: Some("repo_symbol_search".to_string()),
            hierarchical_uri: Some("repo://sciml/symbol/BaseModelica".to_string()),
            hierarchy: Some(vec!["src".to_string(), "BaseModelica.jl".to_string()]),
            saliency_score: Some(0.88),
            audit_status: Some("verified".to_string()),
            verification_state: Some("verified".to_string()),
            implicit_backlinks: Some(vec!["repo:sciml:doc:README.md".to_string()]),
            implicit_backlink_items: Some(vec![SearchBacklinkItem {
                id: "repo:sciml:doc:README.md".to_string(),
                title: Some("README".to_string()),
                path: Some("README.md".to_string()),
                kind: Some("documents".to_string()),
            }]),
            navigation_target: Some(StudioNavigationTarget {
                path: "sciml/src/BaseModelica.jl".to_string(),
                category: "repo_code".to_string(),
                project_name: Some("sciml".to_string()),
                root_label: Some("sciml".to_string()),
                line: Some(12),
                line_end: Some(18),
                column: Some(1),
            }),
        }];

        let encoded =
            encode_search_hits_ipc(&hits).expect("search hit arrow encoding should succeed");
        let reader = StreamReader::try_new(Cursor::new(encoded), None)
            .expect("search hit stream reader should open");
        let batches = reader
            .collect::<Result<Vec<_>, _>>()
            .expect("search hit stream should decode");
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert_eq!(batch.num_rows(), 1);
        assert!(batch.column_by_name("tagsJson").is_some());
        assert!(batch.column_by_name("navigationTargetJson").is_some());
    }
}
