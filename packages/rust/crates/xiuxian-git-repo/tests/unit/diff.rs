use xiuxian_git_repo::{RevisionChangeKind, diff_checkout_revisions};

use crate::support::{
    append_repo_file_and_commit, head_revision, init_test_repository, must,
    remove_repo_file_and_commit, rename_repo_file_and_commit, temp_dir,
};

#[test]
fn diff_checkout_revisions_reports_modify_delete_and_rename_paths() {
    let repository = temp_dir();
    init_test_repository(repository.path());
    append_repo_file_and_commit(
        repository.path(),
        "src/Alpha.jl",
        "module Alpha\nalpha() = 1\nend\n",
        "add alpha",
    );
    append_repo_file_and_commit(
        repository.path(),
        "examples/demo.jl",
        "using Alpha\nalpha()\n",
        "add example",
    );
    let previous_revision = head_revision(repository.path());

    append_repo_file_and_commit(
        repository.path(),
        "src/Alpha.jl",
        "module Alpha\nalpha() = 2\nend\n",
        "modify alpha",
    );
    remove_repo_file_and_commit(repository.path(), "examples/demo.jl", "remove example");
    rename_repo_file_and_commit(
        repository.path(),
        "src/Alpha.jl",
        "src/Beta.jl",
        "rename alpha",
    );
    let revision = head_revision(repository.path());

    let diff = must(
        diff_checkout_revisions(repository.path(), &previous_revision, &revision),
        "diff revisions",
    );

    assert_eq!(diff.previous_revision, previous_revision);
    assert_eq!(diff.revision, revision);
    assert!(diff.changes.iter().any(|change| {
        change.kind == RevisionChangeKind::Deleted
            && change.previous_path.as_deref() == Some("examples/demo.jl")
    }));
    assert!(diff.changes.iter().any(|change| {
        change.kind == RevisionChangeKind::Renamed
            && change.previous_path.as_deref() == Some("src/Alpha.jl")
            && change.path == "src/Beta.jl"
    }));
    assert!(diff.changed_paths().contains("src/Beta.jl"));
    assert!(diff.deleted_paths().contains("src/Alpha.jl"));
    assert!(diff.deleted_paths().contains("examples/demo.jl"));
}

#[test]
fn diff_checkout_revisions_returns_empty_summary_for_identical_revisions() {
    let repository = temp_dir();
    init_test_repository(repository.path());
    let revision = head_revision(repository.path());

    let diff = must(
        diff_checkout_revisions(repository.path(), &revision, &revision),
        "diff revisions",
    );

    assert!(diff.is_empty());
    assert!(diff.changed_paths().is_empty());
    assert!(diff.deleted_paths().is_empty());
}
