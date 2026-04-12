use xiuxian_wendao_core::LinkGraphRefreshMode;

#[test]
fn link_graph_refresh_mode_variants_are_stable() {
    let variants = [
        LinkGraphRefreshMode::Noop,
        LinkGraphRefreshMode::Delta,
        LinkGraphRefreshMode::Full,
    ];
    assert_eq!(variants.len(), 3);
    assert_eq!(variants[0], LinkGraphRefreshMode::Noop);
    assert_eq!(variants[1], LinkGraphRefreshMode::Delta);
    assert_eq!(variants[2], LinkGraphRefreshMode::Full);
}
