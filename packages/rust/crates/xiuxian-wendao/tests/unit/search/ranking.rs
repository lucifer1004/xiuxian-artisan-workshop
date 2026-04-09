use std::collections::HashMap;

use super::{
    RetainedWindow, StreamingRerankSource, StreamingRerankTelemetry, trim_ranked_string_map,
    trim_ranked_vec,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    key: String,
    score: i32,
}

fn compare_candidates(left: &Candidate, right: &Candidate) -> std::cmp::Ordering {
    right
        .score
        .cmp(&left.score)
        .then_with(|| left.key.cmp(&right.key))
}

fn candidate_key(candidate: &Candidate) -> String {
    candidate.key.clone()
}

#[test]
fn retained_window_scales_limit() {
    let small = RetainedWindow::new(4, 8, 128);
    assert_eq!(small.target, 128);
    assert_eq!(small.threshold, 256);

    let large = RetainedWindow::new(64, 8, 128);
    assert_eq!(large.target, 512);
    assert_eq!(large.threshold, 1024);
}

#[test]
fn trim_ranked_vec_keeps_top_ranked_entries() {
    let mut candidates = vec![
        Candidate {
            key: "zeta".to_string(),
            score: 1,
        },
        Candidate {
            key: "beta".to_string(),
            score: 3,
        },
        Candidate {
            key: "alpha".to_string(),
            score: 3,
        },
    ];

    trim_ranked_vec(&mut candidates, 2, compare_candidates);

    assert_eq!(
        candidates,
        vec![
            Candidate {
                key: "alpha".to_string(),
                score: 3,
            },
            Candidate {
                key: "beta".to_string(),
                score: 3,
            },
        ]
    );
}

#[test]
fn trim_ranked_string_map_keeps_top_ranked_entries() {
    let mut candidates = HashMap::from([
        (
            "zeta".to_string(),
            Candidate {
                key: "zeta".to_string(),
                score: 1,
            },
        ),
        (
            "beta".to_string(),
            Candidate {
                key: "beta".to_string(),
                score: 3,
            },
        ),
        (
            "alpha".to_string(),
            Candidate {
                key: "alpha".to_string(),
                score: 3,
            },
        ),
    ]);

    trim_ranked_string_map(&mut candidates, 2, compare_candidates, candidate_key);

    let mut retained = candidates.into_values().collect::<Vec<_>>();
    retained.sort_by(compare_candidates);
    assert_eq!(
        retained,
        vec![
            Candidate {
                key: "alpha".to_string(),
                score: 3,
            },
            Candidate {
                key: "beta".to_string(),
                score: 3,
            },
        ]
    );
}

#[test]
fn streaming_rerank_telemetry_tracks_batches_and_trims() {
    let mut telemetry =
        StreamingRerankTelemetry::new(RetainedWindow::new(8, 4, 16), Some(256), Some(128));
    telemetry.observe_batch(64);
    telemetry.observe_match();
    telemetry.observe_match();
    telemetry.observe_working_set(18);
    telemetry.observe_trim(18, 16);

    let record = telemetry.finish(
        StreamingRerankSource::Scan,
        Some("alpha/repo".to_string()),
        8,
    );

    assert_eq!(record.scope.as_deref(), Some("alpha/repo"));
    assert_eq!(
        record.source,
        crate::search::SearchQueryTelemetrySource::Scan
    );
    assert_eq!(record.batch_count, 1);
    assert_eq!(record.rows_scanned, 64);
    assert_eq!(record.matched_rows, 2);
    assert_eq!(record.result_count, 8);
    assert_eq!(record.batch_row_limit, Some(256));
    assert_eq!(record.recall_limit_rows, Some(128));
    assert_eq!(record.working_set_budget_rows, 32);
    assert_eq!(record.trim_threshold_rows, 64);
    assert_eq!(record.peak_working_set_rows, 18);
    assert_eq!(record.trim_count, 1);
    assert_eq!(record.dropped_candidate_count, 2);
}
