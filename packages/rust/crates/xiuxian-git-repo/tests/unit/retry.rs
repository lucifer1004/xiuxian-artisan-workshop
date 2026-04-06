#[test]
fn managed_checkout_git_open_retry_only_retries_descriptor_pressure_messages() {
    assert!(
        xiuxian_git_repo::RepoError::classify_message(
            "could not open '/tmp/example.git/config': Too many open files; class=Os (2)"
        ) == xiuxian_git_repo::RepoErrorKind::DescriptorPressure
    );
    assert!(
        xiuxian_git_repo::RepoError::classify_message(
            "could not open '/tmp/example.git/config': No such file or directory"
        ) != xiuxian_git_repo::RepoErrorKind::DescriptorPressure
    );
}
