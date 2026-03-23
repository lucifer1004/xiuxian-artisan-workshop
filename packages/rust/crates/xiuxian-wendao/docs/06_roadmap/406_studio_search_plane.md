# 406 Studio Search Plane

## Goal

Replace Studio request-path search hot spots with a background-built search plane backed by Lance tables, Arrow batch reranking, and Valkey coordination.

## Active Scope

- unify Lance and Arrow dependency ownership under `xiuxian-vector`
- add reusable columnar table APIs in `xiuxian-vector`
- introduce `xiuxian_wendao::search_plane`
- expose Studio search index lifecycle through `/api/search/index/status`
- migrate Studio search handlers away from direct AST cache construction

## Constraints

- `LinkGraphIndex` stays responsible for graph traversal and topology APIs
- search payload contracts remain stable during the migration
- the strategic blueprint referenced by repository policy is absent from this checkout, so this roadmap item records the gap explicitly

## Current Slice

- foundation for corpus status, epoch publication, and single-flight builds is landed
- `local_symbol` backs `search_ast`, `search_autocomplete`, and `search_definition`
- `reference_occurrence` now backs `search_references`
- `attachment` now backs `search_attachments`
- `repo_content_chunk` now backs code-search file fallback and removes long-lived source blob storage from `RepoIndexSnapshot`
- `repo_entity` now materializes repo analyzer modules, symbols, and examples into per-repository Lance tables
- `knowledge_section` now backs `search_knowledge` and non-code `search_intent`, with note body and section text materialized into Lance `search_text`
- non-code `search_intent` now merges `knowledge_section`, `local_symbol`, and repo-content hits into a single hybrid response path instead of treating intent as a pure knowledge lookup
- code-biased Studio search now queries `repo_entity` before repo-content fallback, and hybrid intent merges repo-entity hits into the same ranked response path
- `search_plane::cache` now fronts repeat autocomplete, knowledge, non-repo intent, repo-scoped code search, and code-biased hybrid intent requests with corpus-aware Valkey keys and silent fallback to direct Lance reads when Valkey is unavailable
- repo-backed query keys now derive from local corpus state plus repo-index status fragments for `repo_entity` and `repo_content_chunk`, so repo-aware caching no longer has to bypass Valkey just because the response depends on repo publication state
- `repo_entity` and `repo_content_chunk` now emit explicit publication records after successful table writes, so the search plane can distinguish "published rows that remain readable" from transient repo-index phase churn
- repo publication manifests now also carry `source_revision`, and repo indexing threads `sync_result.revision` into both repo-backed publish paths so published tables are pinned to the exact source revision that produced them
- `code_search` and code-biased hybrid `intent` now keep serving published repo-backed tables while a repo refresh is in flight, instead of collapsing to snapshot-miss pending state as soon as repo indexing starts
- repo-aware hot-query keys now preserve stable publication identity for steady-state ready reads, but append repo phase plus current/published revision fragments while refresh or ready-state drift is present, so cache hits no longer hide refresh-state or revision-mismatch responses
- repo-backed status synthesis now preserves published row counts, fragment counts, fingerprints, and publish timestamps while refresh work is active, which gives `/api/search/index/status` a stable read-availability view even before repo corpora have first-class coordinator epochs
- repo-backed status synthesis now also surfaces ready-state revision drift explicitly. A repo reported as `ready` but backed by a manifest for a different revision is treated as a manifest consistency error instead of being silently accepted as current
- search-plane lifecycle now includes a first-class `degraded` phase for readable-but-inconsistent corpora, and repo-backed status synthesis upgrades to `degraded` whenever published rows still serve reads but manifest drift or partial repo failures are present
- Studio `/api/search/index/status` now exposes `degraded` in both aggregate counters and per-corpus phases, so clients can distinguish fully ready corpora from stale/partial repo-backed availability without parsing human error strings
- repo-backed status now also emits machine-readable `issues` metadata. Each issue carries a stable code plus repo/revision/readability context, so clients can branch on manifest-missing, revision-missing, revision-mismatch, and repo-index-failed conditions without scraping `last_error`
- repo-backed status now also emits `issueSummary`, which compresses the raw issue list into `family`, `primaryCode`, `issueCount`, and `readableIssueCount` so UI consumers can render a stable summary without reimplementing issue bucketing logic
- each corpus status row now also emits `statusReason`, which projects lifecycle phase plus issue state into one direct decision surface: `code`, `severity`, `action`, and `readable`. Initial indexing reports `warming_up`, refresh indexing reports `refreshing`, failed rebuilds report `build_failed`, and repo-backed consistency issues map directly to repo resync or repo-sync inspection actions
- `statusReason` now also absorbs maintenance for healthy readable corpora. A ready corpus with queued background compaction reports `compaction_pending`, so status consumers no longer need to independently join `phase` and `maintenance.compaction_pending` to detect search-plane optimization work
- the old production dependency on `RepoIndexSnapshot` has been removed; repo snapshots are now only a test shim and no longer determine whether repo-backed search paths can read data
- repo-backed publication records now carry explicit `publication_id` values and can be persisted into Valkey. Request-path repo search and repo-aware cache keys can therefore hydrate publication state from Valkey after process restarts instead of relying only on in-memory state
- publication reads are now unified by trust boundary: search, cache invalidation, and repo-backed status synthesis all use in-memory publication state plus Valkey manifests only. Disk tables are no longer treated as an implicit publication fallback when the manifest is missing
- corpus publish now schedules background `xiuxian-vector` compaction when maintenance thresholds trip, and the recorded fragment count is refreshed after compaction completes
- same-fingerprint corpus requests no longer short-circuit across schema changes; schema version now participates in build identity so a schema bump forces a fresh staging epoch
- Studio now exposes `/api/search/index/status` for corpus lifecycle visibility without changing existing search payload contracts
- `/api/search/index/status` now synthesizes `repo_content_chunk` readiness from live repo-index phases plus published per-repo Lance table metadata, so repo content no longer appears as a permanently idle corpus
- `/api/search/index/status` now also synthesizes `repo_entity` readiness from live repo-index phases plus published per-repo Lance table metadata, so both repo-backed corpora report real readiness and failure state
- search snapshots now assert non-empty knowledge hits from published corpora rather than empty request-path placeholders
- next critical slice is deciding whether repo publication manifests should graduate into a first-class repo-backed coordinator/epoch model, followed by deciding whether `/api/search/index/status` should expose aggregate `statusReason` or true compaction-runtime telemetry before streaming Arrow rerank limits are added for larger repo tables
