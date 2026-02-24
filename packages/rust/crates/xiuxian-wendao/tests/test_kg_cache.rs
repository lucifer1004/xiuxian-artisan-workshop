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
//! Integration tests for KG cache (load_from_valkey_cached, invalidate).
//!
//! Tests share a static cache; run with: cargo test -p xiuxian-wendao kg_cache -- --test-threads=1

use std::sync::{LazyLock, Mutex, MutexGuard};
use tempfile::TempDir;
use xiuxian_wendao::graph::KnowledgeGraph;
use xiuxian_wendao::kg_cache::{cache_len, invalidate, invalidate_all, load_from_valkey_cached};
use xiuxian_wendao::{Entity, EntityType};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn test_guard() -> MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|_| panic!("kg cache test lock poisoned"))
}

fn has_valkey() -> bool {
    std::env::var("VALKEY_URL")
        .ok()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn create_test_kg_with_entity() -> (TempDir, String) {
    let tmp = TempDir::new().unwrap();
    let scope_key = tmp.path().join("kg").to_string_lossy().into_owned();

    let graph = KnowledgeGraph::new();
    let entity = Entity::new(
        "test:foo".to_string(),
        "Foo".to_string(),
        EntityType::Concept,
        "Test entity".to_string(),
    );
    graph.add_entity(entity).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime
        .block_on(graph.save_to_valkey(&scope_key, 8))
        .unwrap();
    (tmp, scope_key)
}

#[test]
fn test_cache_miss_then_hit() {
    if !has_valkey() {
        return;
    }
    let _guard = test_guard();
    invalidate_all();
    let (_tmp, scope_key) = create_test_kg_with_entity();

    let g1 = load_from_valkey_cached(&scope_key).unwrap().unwrap();
    assert_eq!(g1.get_stats().total_entities, 1);

    let g2 = load_from_valkey_cached(&scope_key).unwrap().unwrap();
    assert_eq!(g2.get_stats().total_entities, 1);
    assert_eq!(cache_len(), 1, "cache should have one entry");
}

#[test]
fn test_cache_invalidation_after_save() {
    if !has_valkey() {
        return;
    }
    let _guard = test_guard();
    invalidate_all();
    let (_tmp, scope_key) = create_test_kg_with_entity();

    let g1 = load_from_valkey_cached(&scope_key).unwrap().unwrap();
    assert_eq!(g1.get_stats().total_entities, 1);
    assert_eq!(cache_len(), 1);

    invalidate(&scope_key);
    assert_eq!(cache_len(), 0, "invalidate should remove the entry");

    let g2 = load_from_valkey_cached(&scope_key).unwrap().unwrap();
    assert_eq!(g2.get_stats().total_entities, 1);
}

#[test]
fn test_nonexistent_path_returns_empty() {
    if !has_valkey() {
        return;
    }
    let _guard = test_guard();
    invalidate_all();
    let result = load_from_valkey_cached("nonexistent.scope").unwrap();
    assert!(result.is_some());
    let g = result.unwrap();
    assert_eq!(g.get_stats().total_entities, 0);
    assert_eq!(g.get_stats().total_relations, 0);
    assert_eq!(cache_len(), 0);
}

#[test]
fn test_path_normalization() {
    if !has_valkey() {
        return;
    }
    let _guard = test_guard();
    invalidate_all();
    let (_tmp, scope_key) = create_test_kg_with_entity();
    let scope_key_trailing = format!("{scope_key}/");

    let g1 = load_from_valkey_cached(&scope_key).unwrap().unwrap();
    let g2 = load_from_valkey_cached(&scope_key_trailing)
        .unwrap()
        .unwrap();
    assert_eq!(g1.get_stats().total_entities, g2.get_stats().total_entities);
    assert_eq!(
        cache_len(),
        1,
        "normalized paths should share one cache entry"
    );
}
