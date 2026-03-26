//! A/B smoke tests for native Lance FTS versus Tantivy keyword retrieval.

use std::collections::HashSet;
use std::time::Instant;

use anyhow::{Result, anyhow};
use serde_json::json;
use xiuxian_vector::{
    CONTENT_COLUMN, ColumnarScanOptions, ID_COLUMN, KeywordSearchBackend, VectorStore,
};

async fn build_store_with_tool_data() -> Result<VectorStore> {
    let temp_dir = tempfile::Builder::new()
        .prefix("xiuxian_vector_fts_")
        .tempdir()?
        .keep();
    let db_path = temp_dir.join("fts_store");
    let db_path_str = db_path.to_string_lossy();
    let store =
        VectorStore::new_with_keyword_index(db_path_str.as_ref(), Some(8), true, None, None)
            .await?;

    let ids = vec![
        "git.commit".to_string(),
        "git.rebase".to_string(),
        "docker.build".to_string(),
        "python.pytest".to_string(),
    ];
    let vectors = vec![
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ];
    let contents = vec![
        "Commit staged changes with conventional commit format".to_string(),
        "Interactive rebase for squash and history rewrite".to_string(),
        "Build docker image with cache and tag controls".to_string(),
        "Run pytest with filters and coverage".to_string(),
    ];
    let metadatas = vec![
        json!({
            "type":"command",
            "skill_name":"git",
            "tool_name":"git.commit",
            "keywords":["commit","message","conventional"],
            "intents":["commit code","save changes"]
        })
        .to_string(),
        json!({
            "type":"command",
            "skill_name":"git",
            "tool_name":"git.rebase",
            "keywords":["rebase","squash","history"],
            "intents":["rewrite history"]
        })
        .to_string(),
        json!({
            "type":"command",
            "skill_name":"docker",
            "tool_name":"docker.build",
            "keywords":["docker","build","image"],
            "intents":["build container"]
        })
        .to_string(),
        json!({
            "type":"command",
            "skill_name":"python",
            "tool_name":"python.pytest",
            "keywords":["pytest","test","coverage"],
            "intents":["run tests"]
        })
        .to_string(),
    ];

    store
        .add_documents("tools", ids, vectors, contents, metadatas)
        .await?;
    store.create_fts_index("tools").await?;
    Ok(store)
}

async fn build_store_with_lance_backend() -> Result<VectorStore> {
    let temp_dir = tempfile::Builder::new()
        .prefix("xiuxian_vector_fts_backend_")
        .tempdir()?
        .keep();
    let db_path = temp_dir.join("fts_store");
    let db_path_str = db_path.to_string_lossy();
    let store = VectorStore::new_with_keyword_backend(
        db_path_str.as_ref(),
        Some(8),
        true,
        KeywordSearchBackend::LanceFts,
        None,
        None,
    )
    .await?;

    store
        .add_documents(
            "tools",
            vec!["git.commit".to_string(), "docker.build".to_string()],
            vec![vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], vec![0.0; 8]],
            vec![
                "Commit staged changes with conventional commit format".to_string(),
                "Build docker image".to_string(),
            ],
            vec![
                json!({
                    "type":"command",
                    "skill_name":"git",
                    "tool_name":"git.commit",
                    "keywords":["commit"]
                })
                .to_string(),
                json!({
                    "type":"command",
                    "skill_name":"docker",
                    "tool_name":"docker.build",
                    "keywords":["build"]
                })
                .to_string(),
            ],
        )
        .await?;
    store.create_fts_index("tools").await?;
    Ok(store)
}

#[tokio::test]
async fn test_lance_fts_returns_expected_hits() -> Result<()> {
    let store = build_store_with_tool_data().await?;

    let results = store.search_fts("tools", "commit", 10, None).await?;
    assert!(!results.is_empty());
    assert_eq!(results[0].tool_name, "git.commit");

    Ok(())
}

#[tokio::test]
async fn test_lance_fts_and_tantivy_overlap() -> Result<()> {
    let store = build_store_with_tool_data().await?;

    let keyword_index = store
        .keyword_index
        .as_ref()
        .ok_or_else(|| anyhow!("keyword index missing"))?;

    let tantivy_hits = keyword_index.search("build", 10)?;
    let fts_hits = store.search_fts("tools", "build", 10, None).await?;

    assert!(!tantivy_hits.is_empty());
    assert!(!fts_hits.is_empty());

    let tantivy_names: HashSet<_> = tantivy_hits.iter().map(|h| h.tool_name.clone()).collect();
    let fts_names: HashSet<_> = fts_hits.iter().map(|h| h.tool_name.clone()).collect();
    let overlap = tantivy_names.intersection(&fts_names).count();
    assert!(
        overlap >= 1,
        "expected at least one common hit, tantivy={tantivy_names:?}, fts={fts_names:?}"
    );

    Ok(())
}

#[tokio::test]
async fn test_lance_fts_tantivy_smoke_latency() -> Result<()> {
    let store = build_store_with_tool_data().await?;
    let rounds: u32 = 25;

    let keyword_index = store
        .keyword_index
        .as_ref()
        .ok_or_else(|| anyhow!("keyword index missing"))?;

    let t0 = Instant::now();
    for _ in 0..rounds {
        let _ = store.search_fts("tools", "history", 5, None).await?;
    }
    let fts_avg_ms = t0.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds);

    let t1 = Instant::now();
    for _ in 0..rounds {
        let _ = keyword_index.search("history", 5)?;
    }
    let tantivy_avg_ms = t1.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds);

    // Benchmark smoke output only; no strict pass/fail threshold in CI.
    eprintln!("fts_avg_ms={fts_avg_ms:.2}, tantivy_avg_ms={tantivy_avg_ms:.2}, rounds={rounds}");
    assert!(fts_avg_ms >= 0.0);
    assert!(tantivy_avg_ms >= 0.0);

    Ok(())
}

#[tokio::test]
async fn test_hybrid_search_with_lance_keyword_backend() -> Result<()> {
    let store = build_store_with_lance_backend().await?;
    let results = store
        .hybrid_search(
            "tools",
            "commit",
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            5,
        )
        .await?;

    assert!(!results.is_empty());
    assert_eq!(results[0].tool_name, "git.commit");

    Ok(())
}

#[tokio::test]
async fn test_streaming_fts_projection_does_not_autoproject_score_column() -> Result<()> {
    let store = build_store_with_tool_data().await?;
    let mut batches = Vec::new();
    store
        .search_fts_batches_streaming(
            "tools",
            "commit",
            ColumnarScanOptions {
                projected_columns: vec![ID_COLUMN.to_string(), CONTENT_COLUMN.to_string()],
                limit: Some(4),
                ..ColumnarScanOptions::default()
            },
            |batch| -> Result<()> {
                batches.push(batch);
                Ok(())
            },
        )
        .await?;

    assert!(!batches.is_empty());
    for batch in batches {
        let fields = batch.schema_ref().fields();
        assert!(
            fields.iter().all(|field| field.name() != "_score"),
            "unexpected implicit _score projection: {:?}",
            fields
                .iter()
                .map(|field| field.name().clone())
                .collect::<Vec<_>>()
        );
    }

    Ok(())
}
