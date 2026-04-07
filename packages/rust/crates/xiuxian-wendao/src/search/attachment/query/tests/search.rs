use crate::search::attachment::query::search::search_attachment_hits;

use super::fixtures::{fixture_service, publish_attachment_hits, sample_hit};

#[tokio::test]
async fn attachment_query_reads_hits_from_published_epoch() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let hits = vec![
        sample_hit(
            "topology.png",
            "docs/alpha.md",
            "assets/topology.png",
            "image",
        ),
        sample_hit("spec.pdf", "docs/alpha.md", "files/spec.pdf", "pdf"),
    ];
    publish_attachment_hits(&service, "fp-1", &hits).await;

    let results = search_attachment_hits(&service, "topology", 5, &[], &[], false)
        .await
        .unwrap_or_else(|error| panic!("query should succeed: {error}"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].attachment_name, "topology.png");
    assert!(results[0].score > 0.0);
}
