use crate::search::SearchCorpusKind;
use crate::search::coordinator::SearchCompactionReason;
use crate::search::service::core::RepoMaintenanceTaskKind;

use crate::search::service::core::maintenance::tests::{make_compaction_task, make_prewarm_task};

#[test]
fn repo_maintenance_task_key_tracks_kind() {
    let prewarm = make_prewarm_task(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        "repo_entity_alpha",
        &["path"],
    );
    let compaction = make_compaction_task(
        SearchCorpusKind::RepoEntity,
        "alpha/repo",
        "publication-1",
        "repo_entity_alpha",
        12,
        SearchCompactionReason::PublishThreshold,
    );

    assert_eq!(prewarm.task_key().3, RepoMaintenanceTaskKind::Prewarm);
    assert_eq!(compaction.task_key().3, RepoMaintenanceTaskKind::Compaction);
    assert_ne!(prewarm.task_key(), compaction.task_key());
}
