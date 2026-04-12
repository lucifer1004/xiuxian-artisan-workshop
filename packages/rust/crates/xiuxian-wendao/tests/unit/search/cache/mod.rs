use std::collections::BTreeMap;

use crate::search::cache::SearchPlaneCache;
use crate::search::{
    SearchCorpusKind, SearchFileFingerprint, SearchManifestKeyspace,
    SearchPublicationStorageFormat, SearchRepoCorpusRecord, SearchRepoCorpusSnapshotRecord,
    SearchRepoPublicationInput, SearchRepoPublicationRecord,
};

#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct TestCacheShadow {
    pub(crate) generic_json_payloads: BTreeMap<String, String>,
    pub(crate) repo_corpus_records: BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord>,
    pub(crate) repo_corpus_snapshot: Option<SearchRepoCorpusSnapshotRecord>,
    pub(crate) repo_publications_by_revision:
        BTreeMap<(SearchCorpusKind, String, String), SearchRepoPublicationRecord>,
    pub(crate) repo_publication_revision_indexes: BTreeMap<(SearchCorpusKind, String), Vec<String>>,
    pub(crate) corpus_file_fingerprints:
        BTreeMap<SearchCorpusKind, BTreeMap<String, SearchFileFingerprint>>,
    pub(crate) repo_corpus_file_fingerprints:
        BTreeMap<(SearchCorpusKind, String), BTreeMap<String, SearchFileFingerprint>>,
}

#[cfg(test)]
impl SearchPlaneCache {
    pub(crate) fn clear_repo_shadow_for_tests(&self, repo_id: &str) {
        let mut shadow = self
            .shadow
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        shadow
            .repo_corpus_records
            .retain(|(_, candidate_repo_id), _| candidate_repo_id != repo_id);
        if let Some(snapshot) = shadow.repo_corpus_snapshot.as_mut() {
            snapshot.records.retain(|record| record.repo_id != repo_id);
            if snapshot.records.is_empty() {
                shadow.repo_corpus_snapshot = None;
            }
        }
        shadow
            .repo_corpus_file_fingerprints
            .retain(|(_, candidate_repo_id), _| candidate_repo_id != repo_id);
        shadow
            .repo_publications_by_revision
            .retain(|(_, candidate_repo_id, _), _| candidate_repo_id != repo_id);
        shadow
            .repo_publication_revision_indexes
            .retain(|(_, candidate_repo_id), _| candidate_repo_id != repo_id);
    }
}

#[cfg(test)]
fn required_cache_key(key: Option<String>, context: &str) -> String {
    key.unwrap_or_else(|| panic!("{context}"))
}

#[cfg(test)]
fn cache_for_tests() -> SearchPlaneCache {
    SearchPlaneCache::for_tests(SearchManifestKeyspace::new("xiuxian:test:search_plane"))
}

#[cfg(test)]
#[test]
fn autocomplete_key_is_stable_for_epoch_prefix_and_limit() {
    let cache = cache_for_tests();
    let key = required_cache_key(
        cache.autocomplete_cache_key(" Alpha Handler ", 8, 7),
        "autocomplete key",
    );
    assert_eq!(
        key,
        required_cache_key(
            cache.autocomplete_cache_key("alpha    handler", 8, 7),
            "stable autocomplete key",
        )
    );
    assert_ne!(
        key,
        required_cache_key(
            cache.autocomplete_cache_key("alpha handler", 8, 8),
            "epoch-specific autocomplete key",
        )
    );
}

#[cfg(test)]
#[test]
fn search_query_key_tracks_scope_epochs_and_query_shape() {
    let cache = cache_for_tests();
    let key = required_cache_key(
        cache.search_query_cache_key(
            "intent",
            &[
                (SearchCorpusKind::KnowledgeSection, 3),
                (SearchCorpusKind::LocalSymbol, 11),
            ],
            "  alpha_handler  ",
            10,
            Some("semantic_lookup"),
            None,
        ),
        "search query key",
    );
    assert_eq!(
        key,
        required_cache_key(
            cache.search_query_cache_key(
                "intent",
                &[
                    (SearchCorpusKind::KnowledgeSection, 3),
                    (SearchCorpusKind::LocalSymbol, 11),
                ],
                "alpha_handler",
                10,
                Some("semantic_lookup"),
                None,
            ),
            "stable search query key",
        )
    );
    assert_ne!(
        key,
        required_cache_key(
            cache.search_query_cache_key(
                "intent",
                &[
                    (SearchCorpusKind::KnowledgeSection, 3),
                    (SearchCorpusKind::LocalSymbol, 12),
                ],
                "alpha_handler",
                10,
                Some("semantic_lookup"),
                None,
            ),
            "epoch-specific search query key",
        )
    );
}

#[cfg(test)]
#[test]
fn search_query_key_tracks_repo_versions_and_sorts_components() {
    let cache = cache_for_tests();
    let key = required_cache_key(
        cache.search_query_cache_key_from_versions(
            "intent_code",
            &[
                "repo_entity:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                    .to_string(),
                "knowledge_section:schema:1:epoch:3".to_string(),
                "repo_content_chunk:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                    .to_string(),
            ],
            " lang:julia reexport ",
            10,
            Some("debug_lookup"),
            Some("alpha"),
        ),
        "repo search query key",
    );
    assert_eq!(
        key,
        required_cache_key(
            cache.search_query_cache_key_from_versions(
                "intent_code",
                &[
                    "repo_content_chunk:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                        .to_string(),
                    "knowledge_section:schema:1:epoch:3".to_string(),
                    "repo_entity:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                        .to_string(),
                ],
                "lang:julia   reexport",
                10,
                Some("debug_lookup"),
                Some("alpha"),
            ),
            "stable repo search query key",
        )
    );
    assert_ne!(
        key,
        required_cache_key(
            cache.search_query_cache_key_from_versions(
                "intent_code",
                &[
                    "repo_entity:schema:1:repo:alpha:phase:ready:revision:def:updated:2026-03-23t09:00:00z"
                        .to_string(),
                    "knowledge_section:schema:1:epoch:3".to_string(),
                    "repo_content_chunk:schema:1:repo:alpha:phase:ready:revision:def:updated:2026-03-23t09:00:00z"
                        .to_string(),
                ],
                "lang:julia reexport",
                10,
                Some("debug_lookup"),
                Some("alpha"),
            ),
            "repo-specific search query key",
        )
    );
}

#[cfg(test)]
#[test]
fn disabled_cache_skips_key_generation() {
    let cache = SearchPlaneCache::disabled(SearchManifestKeyspace::new("xiuxian:test"));
    assert!(cache.autocomplete_cache_key("alpha", 8, 1).is_none());
    assert!(
        cache
            .search_query_cache_key(
                "knowledge",
                &[(SearchCorpusKind::KnowledgeSection, 1)],
                "alpha",
                10,
                None,
                None,
            )
            .is_none()
    );
}

#[cfg(test)]
#[tokio::test]
async fn delete_repo_publication_revision_cache_clears_retained_revision_entries() {
    let cache = cache_for_tests();
    let publication = SearchRepoPublicationRecord::new_with_storage_format(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: "repo_entity_alpha_repo".to_string(),
            schema_version: 1,
            source_revision: Some("rev-clean-build".to_string()),
            table_version_id: 7,
            row_count: 5,
            fragment_count: 1,
            published_at: "2026-04-06T00:00:00Z".to_string(),
        },
        SearchPublicationStorageFormat::Lance,
    );

    cache
        .set_repo_publication_for_revision(SearchCorpusKind::RepoEntity, "alpha/repo", &publication)
        .await;
    assert!(
        cache
            .get_repo_publication_for_revision(
                SearchCorpusKind::RepoEntity,
                "alpha/repo",
                "rev-clean-build",
            )
            .await
            .is_some()
    );

    cache
        .delete_repo_publication_revision_cache(SearchCorpusKind::RepoEntity, "alpha/repo")
        .await;

    assert!(
        cache
            .get_repo_publication_for_revision(
                SearchCorpusKind::RepoEntity,
                "alpha/repo",
                "rev-clean-build",
            )
            .await
            .is_none()
    );
    assert!(
        cache
            .get_repo_publication_revisions(SearchCorpusKind::RepoEntity, "alpha/repo")
            .await
            .is_empty()
    );
}

#[cfg(test)]
#[tokio::test]
async fn generic_json_cache_uses_test_shadow_without_live_client() {
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct ProbePayload {
        value: String,
    }

    let cache = cache_for_tests();
    let key = "xiuxian:test:search_plane:hot_query:probe";
    let payload = ProbePayload {
        value: "cached".to_string(),
    };

    cache
        .set_json(
            key,
            crate::search::cache::SearchPlaneCacheTtl::HotQuery,
            &payload,
        )
        .await;

    let cached: Option<ProbePayload> = cache.get_json(key).await;
    assert_eq!(cached, Some(payload));
}

#[cfg(test)]
#[tokio::test]
async fn delete_repo_publication_revision_cache_preserves_latest_repo_corpus_record() {
    let cache = cache_for_tests();
    let publication = SearchRepoPublicationRecord::new_with_storage_format(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        SearchRepoPublicationInput {
            table_name: "repo_entity_alpha_repo".to_string(),
            schema_version: 1,
            source_revision: Some("rev-clean-build".to_string()),
            table_version_id: 7,
            row_count: 5,
            fragment_count: 1,
            published_at: "2026-04-06T00:00:00Z".to_string(),
        },
        SearchPublicationStorageFormat::Parquet,
    );

    cache
        .set_repo_corpus_record(&SearchRepoCorpusRecord::new(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            None,
            Some(publication.clone()),
        ))
        .await;
    cache
        .set_repo_publication_for_revision(SearchCorpusKind::RepoEntity, "alpha/repo", &publication)
        .await;

    cache
        .delete_repo_publication_revision_cache(SearchCorpusKind::RepoEntity, "alpha/repo")
        .await;

    let record = cache
        .get_repo_corpus_record(SearchCorpusKind::RepoEntity, "alpha/repo")
        .await
        .unwrap_or_else(|| panic!("latest repo corpus record should remain available"));
    assert_eq!(
        record
            .publication
            .as_ref()
            .and_then(|publication| publication.source_revision.as_deref()),
        Some("rev-clean-build")
    );
    assert!(
        cache
            .get_repo_publication_for_revision(
                SearchCorpusKind::RepoEntity,
                "alpha/repo",
                "rev-clean-build",
            )
            .await
            .is_none()
    );
}
