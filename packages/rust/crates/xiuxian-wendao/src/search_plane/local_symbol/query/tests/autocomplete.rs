use crate::search_plane::local_symbol::query::autocomplete::autocomplete_local_symbols;
use crate::search_plane::{SearchCorpusKind, SearchQueryTelemetrySource};

use super::fixtures::{
    fixture_service, publish_local_symbol_hits, sample_hit, sample_markdown_hit,
};

#[tokio::test]
async fn local_symbol_autocomplete_reads_suggestions_from_published_epoch() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let hits = vec![
        sample_hit("AlphaSymbol", "src/lib.rs", 10),
        sample_markdown_hit("Search Design", Some("section"), None),
        sample_markdown_hit("Search Metadata", Some("property"), Some("Owner")),
    ];
    publish_local_symbol_hits(&service, "fp-2", &hits).await;

    let results = autocomplete_local_symbols(&service, "se", 5)
        .await
        .unwrap_or_else(|error| panic!("autocomplete should succeed: {error}"));

    assert_eq!(
        results
            .into_iter()
            .map(|item| (item.text, item.suggestion_type))
            .collect::<Vec<_>>(),
        vec![
            ("Search Design".to_string(), "heading".to_string()),
            ("Search Metadata".to_string(), "metadata".to_string()),
        ]
    );

    let snapshot = service.status();
    let corpus = snapshot
        .corpora
        .iter()
        .find(|entry| entry.corpus == SearchCorpusKind::LocalSymbol)
        .unwrap_or_else(|| panic!("local symbol corpus row should exist"));
    let telemetry = corpus
        .last_query_telemetry
        .as_ref()
        .unwrap_or_else(|| panic!("autocomplete telemetry should be present"));
    assert_eq!(telemetry.source, SearchQueryTelemetrySource::Scan);
    assert_eq!(telemetry.scope.as_deref(), Some("autocomplete"));
    assert!(telemetry.rows_scanned >= 1);
    assert!(telemetry.matched_rows >= 2);
}
