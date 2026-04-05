use super::{validate_vfs_content_request, validate_vfs_resolve_request};

#[test]
fn vfs_resolve_request_validation_accepts_stable_request() {
    assert!(validate_vfs_resolve_request("main/docs/index.md").is_ok());
}

#[test]
fn vfs_resolve_request_validation_rejects_blank_path() {
    assert_eq!(
        validate_vfs_resolve_request("   "),
        Err("VFS resolve requires a non-empty path".to_string())
    );
}

#[test]
fn vfs_content_request_validation_accepts_stable_request() {
    assert!(validate_vfs_content_request("main/docs/index.md").is_ok());
}

#[test]
fn vfs_content_request_validation_rejects_blank_path() {
    assert_eq!(
        validate_vfs_content_request("   "),
        Err("VFS content requires a non-empty path".to_string())
    );
}
