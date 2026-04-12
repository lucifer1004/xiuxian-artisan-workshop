use xiuxian_wendao_core::{
    LinkGraphDirection, LinkGraphLinkFilter, LinkGraphMatchStrategy, LinkGraphRelatedFilter,
    LinkGraphRelatedPprOptions, LinkGraphSearchFilters, LinkGraphSearchOptions, LinkGraphSortField,
    LinkGraphSortOrder, LinkGraphSortTerm,
};

#[test]
fn direction_aliases_parse_to_expected_values() {
    assert_eq!(
        LinkGraphDirection::from_alias("to"),
        LinkGraphDirection::Incoming
    );
    assert_eq!(
        LinkGraphDirection::from_alias("outgoing"),
        LinkGraphDirection::Outgoing
    );
    assert_eq!(
        LinkGraphDirection::from_alias("unknown"),
        LinkGraphDirection::Both
    );
}

#[test]
fn match_strategy_aliases_parse_to_expected_values() {
    assert_eq!(
        LinkGraphMatchStrategy::from_alias("fuzzy"),
        LinkGraphMatchStrategy::PathFuzzy
    );
    assert_eq!(
        LinkGraphMatchStrategy::from_alias("exact"),
        LinkGraphMatchStrategy::Exact
    );
    assert_eq!(
        LinkGraphMatchStrategy::from_alias("regex"),
        LinkGraphMatchStrategy::Re
    );
    assert_eq!(
        LinkGraphMatchStrategy::from_alias("other"),
        LinkGraphMatchStrategy::Fts
    );
}

#[test]
fn search_options_default_matches_runtime_contract() {
    let options = LinkGraphSearchOptions::default();

    assert_eq!(options.match_strategy, LinkGraphMatchStrategy::Fts);
    assert!(!options.case_sensitive);
    assert_eq!(
        options.sort_terms,
        vec![LinkGraphSortTerm {
            field: LinkGraphSortField::Score,
            order: LinkGraphSortOrder::Desc,
        }]
    );
    assert_eq!(options.filters, LinkGraphSearchFilters::default());
    assert!(options.style_anchors.is_empty());
}

#[test]
fn validate_rejects_zero_distance_link_filter() {
    let options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            link_to: Some(LinkGraphLinkFilter {
                max_distance: Some(0),
                ..LinkGraphLinkFilter::default()
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };

    let error = match options.validate() {
        Ok(()) => panic!("expected validation failure"),
        Err(error) => error,
    };
    assert!(error.contains("filters.link_to.max_distance"));
}

#[test]
fn validate_rejects_invalid_related_ppr_constraints() {
    let options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            related: Some(LinkGraphRelatedFilter {
                max_distance: Some(2),
                ppr: Some(LinkGraphRelatedPprOptions {
                    alpha: Some(1.5),
                    max_iter: Some(10),
                    tol: Some(1e-6),
                    ..LinkGraphRelatedPprOptions::default()
                }),
                ..LinkGraphRelatedFilter::default()
            }),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };

    let error = match options.validate() {
        Ok(()) => panic!("expected validation failure"),
        Err(error) => error,
    };
    assert!(error.contains("filters.related.ppr.alpha"));
}

#[test]
fn validate_rejects_invalid_heading_level_and_section_cap() {
    let options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            max_heading_level: Some(7),
            per_doc_section_cap: Some(0),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };

    let error = match options.validate() {
        Ok(()) => panic!("expected validation failure"),
        Err(error) => error,
    };
    assert!(error.contains("filters.max_heading_level"));
}

#[test]
fn validate_accepts_well_formed_options() {
    let options = LinkGraphSearchOptions {
        filters: LinkGraphSearchFilters {
            related: Some(LinkGraphRelatedFilter {
                max_distance: Some(3),
                ppr: Some(LinkGraphRelatedPprOptions {
                    alpha: Some(0.85),
                    max_iter: Some(20),
                    tol: Some(1e-6),
                    ..LinkGraphRelatedPprOptions::default()
                }),
                ..LinkGraphRelatedFilter::default()
            }),
            max_heading_level: Some(4),
            per_doc_section_cap: Some(5),
            ..LinkGraphSearchFilters::default()
        },
        ..LinkGraphSearchOptions::default()
    };

    assert!(options.validate().is_ok());
}
