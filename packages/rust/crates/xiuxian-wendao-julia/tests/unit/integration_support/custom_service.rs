use super::custom_service_cache_relative_dir;

#[test]
fn custom_service_cache_relative_dir_stays_relative() {
    let relative_dir = custom_service_cache_relative_dir();
    assert!(!relative_dir.is_absolute());
    assert_eq!(
        relative_dir,
        std::path::PathBuf::from("xiuxian-wendao-julia/integration_support/wendaoarrow")
    );
}
