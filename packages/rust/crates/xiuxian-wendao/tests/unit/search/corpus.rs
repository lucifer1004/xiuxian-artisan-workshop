use super::SearchCorpusKind;

#[test]
fn datafusion_cutover_corpora_no_longer_require_legacy_lance_indices() {
    for corpus in SearchCorpusKind::ALL {
        assert!(
            !SearchCorpusKind::requires_legacy_lance_indices(),
            "{corpus} should not build legacy Lance indices after the DataFusion cutover"
        );
    }
}
