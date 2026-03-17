use super::*;
use tempfile::tempdir;

struct StudioStateFixture {
    state: Arc<StudioState>,
    _temp_dir: tempfile::TempDir,
}

fn create_temp_dir() -> tempfile::TempDir {
    match tempdir() {
        Ok(temp_dir) => temp_dir,
        Err(err) => panic!("failed to create temp dir fixture: {err}"),
    }
}

fn write_doc(root: &std::path::Path, name: &str, content: &str) {
    if let Err(err) = std::fs::write(root.join(name), content) {
        panic!("failed to write fixture doc {name}: {err}");
    }
}

fn make_state_with_docs(docs: Vec<(&str, &str)>) -> StudioStateFixture {
    let temp_dir = create_temp_dir();
    for (name, content) in docs {
        write_doc(temp_dir.path(), name, content);
    }

    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.data_root = temp_dir.path().to_path_buf();
    state.knowledge_root = temp_dir.path().to_path_buf();
    state.internal_skill_root = temp_dir.path().to_path_buf();

    StudioStateFixture {
        state: Arc::new(state),
        _temp_dir: temp_dir,
    }
}

#[test]
fn test_strip_option() {
    assert_eq!(strip_option(""), None);
    assert_eq!(strip_option("value"), Some("value".to_string()));
    assert_eq!(strip_option(" value "), Some("value".to_string()));
}

#[tokio::test]
async fn search_knowledge_requires_query() {
    let fixture = make_state_with_docs(Vec::new());

    let result = search_knowledge(
        Query(SearchQuery {
            q: Some("   ".to_string()),
            limit: None,
        }),
        State(Arc::clone(&fixture.state)),
    )
    .await;

    let Err(error) = result else {
        panic!("expected missing-query request to fail");
    };

    assert_eq!(error.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "MISSING_QUERY");
}

#[tokio::test]
async fn search_knowledge_returns_payload() {
    let fixture = make_state_with_docs(vec![
        (
            "alpha.md",
            "# Alpha\n\nThis note contains search target keyword: wendao.\n",
        ),
        (
            "beta.md",
            "# Beta\n\nAnother note mentions wendao in text.\n",
        ),
    ]);

    let result = search_knowledge(
        Query(SearchQuery {
            q: Some("wendao".to_string()),
            limit: Some(5),
        }),
        State(fixture.state),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected search request to succeed");
    };

    assert_eq!(response.0.query, "wendao");
    assert!(response.0.hit_count >= response.0.hits.len());
    assert!(response.0.selected_mode.is_some());
}

#[tokio::test]
async fn autocomplete_limits_and_filters_prefix() {
    let fixture = make_state_with_docs(vec![
        (
            "doc.md",
            "# Search Design\n\nThis doc starts with Search and discusses Search.\n",
        ),
        ("note.md", "# Search Notes\n\nTaggable text.\n"),
    ]);

    let result = search_autocomplete(
        Query(AutocompleteQuery {
            prefix: Some("se".to_string()),
            limit: Some(2),
        }),
        State(fixture.state),
    )
    .await;

    let Ok(response) = result else {
        panic!("expected autocomplete request to succeed");
    };

    assert_eq!(response.0.prefix, "se");
    assert!(response.0.suggestions.len() <= 2);
    for suggestion in response.0.suggestions {
        assert!(suggestion.text.to_ascii_lowercase().starts_with("se"));
    }
}
