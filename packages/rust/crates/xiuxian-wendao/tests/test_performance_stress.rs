#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
//! Performance guardrails for narrator throughput.

use std::time::Instant;
use xiuxian_wendao::LinkGraphHit;

#[tokio::test]
async fn test_narrator_performance_scaling() {
    // 1. Setup a synthetic large subgraph (100 hits)
    let mut hits = Vec::new();
    for i in 0..100 {
        hits.push(LinkGraphHit {
            stem: format!("node_{i}"),
            score: 1.0 - (i as f64 * 0.01),
            title: format!("Deep Scaling Analysis node {i}"),
            path: format!("path/{i}.md"),
            best_section: None,
            match_reason: None,
        });
    }

    // 2. Measure narration latency
    let start = Instant::now();
    let _output = xiuxian_wendao::narrate_subgraph(&hits);
    let duration = start.elapsed();

    println!("Narration of 100 hits took: {duration:?}");

    // Artisan Threshold: Narration should be < 5ms for 100 hits
    assert!(
        duration.as_millis() < 5,
        "Narration is too slow for large subgraphs"
    );
}
