use std::fs;
use std::path::Path;

use git2::{Repository, Signature};

pub(super) fn init_test_repository(root: &Path) {
    let repository = Repository::init(root).expect("init repository");
    fs::write(root.join("Project.toml"), "name = \"CheckoutTest\"\n").expect("write file");

    let mut index = repository.index().expect("open index");
    index
        .add_path(Path::new("Project.toml"))
        .expect("stage project file");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repository.find_tree(tree_id).expect("find tree");
    let signature =
        Signature::now("checkout-test", "checkout-test@example.com").expect("signature");
    repository
        .commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
        .expect("commit");
}

pub(super) fn append_repo_file_and_commit(
    root: &Path,
    relative_path: &str,
    contents: &str,
    message: &str,
) {
    let repository = Repository::open(root).expect("open repository");
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(&path, contents).expect("write file");

    let mut index = repository.index().expect("open index");
    index
        .add_path(Path::new(relative_path))
        .expect("stage changed file");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repository.find_tree(tree_id).expect("find tree");
    let signature =
        Signature::now("checkout-test", "checkout-test@example.com").expect("signature");
    let parent = repository
        .head()
        .expect("head")
        .peel_to_commit()
        .expect("parent commit");
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent],
        )
        .expect("commit");
}
