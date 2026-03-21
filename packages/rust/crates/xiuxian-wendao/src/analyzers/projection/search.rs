use std::collections::HashMap;

use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::projection::contracts::{ProjectedPageRecord, ProjectionPageKind};
use crate::analyzers::query::{RepoProjectedPageSearchQuery, RepoProjectedPageSearchResult};
use crate::search::{
    FuzzyMatch, FuzzyMatcher, FuzzySearchOptions, LexicalMatcher, SearchDocument,
    SearchDocumentIndex,
};

use super::pages::build_projected_pages;

/// Build projected-page search results for one repository query.
pub fn build_repo_projected_page_search(
    query: &RepoProjectedPageSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPageSearchResult {
    build_repo_projected_page_search_with_options(
        query,
        analysis,
        FuzzySearchOptions::document_search(),
    )
}

pub fn build_repo_projected_page_search_with_options(
    query: &RepoProjectedPageSearchQuery,
    analysis: &RepositoryAnalysisOutput,
    options: FuzzySearchOptions,
) -> RepoProjectedPageSearchResult {
    RepoProjectedPageSearchResult {
        repo_id: query.repo_id.clone(),
        pages: ranked_projected_page_matches(
            query.query.as_str(),
            query.kind,
            analysis,
            query.limit,
            options,
        )
        .into_iter()
        .map(|(_, page)| page)
        .collect(),
    }
}

fn ranked_projected_page_matches(
    query: &str,
    kind_filter: Option<ProjectionPageKind>,
    analysis: &RepositoryAnalysisOutput,
    limit: usize,
    options: FuzzySearchOptions,
) -> Vec<(u8, ProjectedPageRecord)> {
    let query = query.trim();
    if query.is_empty() || limit == 0 {
        return Vec::new();
    }

    let pages = build_projected_pages(analysis);
    let limit = limit.min(pages.len());
    if limit == 0 {
        return Vec::new();
    }

    if let Some(indexed_matches) =
        search_indexed_projected_pages(query, kind_filter, pages.as_slice(), limit, options)
    {
        if !indexed_matches.is_empty() {
            return indexed_matches;
        }
    }

    let normalized_query = query.to_ascii_lowercase();
    let scored_matches =
        heuristic_projected_page_matches(normalized_query.as_str(), kind_filter, pages.as_slice());
    if !scored_matches.is_empty() {
        return scored_matches.into_iter().take(limit).collect();
    }

    lexical_projected_page_matches(query, kind_filter, pages.as_slice(), limit, options)
}

fn search_indexed_projected_pages(
    query: &str,
    kind_filter: Option<ProjectionPageKind>,
    pages: &[ProjectedPageRecord],
    limit: usize,
    options: FuzzySearchOptions,
) -> Option<Vec<(u8, ProjectedPageRecord)>> {
    let (search_index, page_by_id) = build_projected_page_search_index(pages).ok()?;
    let normalized_query = query.to_ascii_lowercase();

    let exact_matches = search_index
        .search_exact(query, limit.saturating_mul(2))
        .ok()
        .map(|records| {
            map_search_documents_to_pages(
                records,
                &page_by_id,
                kind_filter,
                limit,
                normalized_query.as_str(),
                Some(50),
            )
        })
        .unwrap_or_default();
    if !exact_matches.is_empty() {
        return Some(exact_matches);
    }

    let fuzzy_matches = search_index
        .search_fuzzy(query, limit.saturating_mul(2), options)
        .ok()
        .map(|records| {
            map_fuzzy_search_documents_to_pages(
                records,
                &page_by_id,
                kind_filter,
                limit,
                normalized_query.as_str(),
            )
        })
        .unwrap_or_default();
    if !fuzzy_matches.is_empty() {
        return Some(fuzzy_matches);
    }

    Some(Vec::new())
}

fn build_projected_page_search_index(
    pages: &[ProjectedPageRecord],
) -> Result<(SearchDocumentIndex, HashMap<String, ProjectedPageRecord>), String> {
    let search_index = SearchDocumentIndex::new();
    let mut page_by_id = HashMap::new();
    let mut documents = Vec::new();

    for page in pages {
        page_by_id.insert(page.page_id.clone(), page.clone());
        documents.push(projected_page_search_document(page));
    }

    search_index
        .add_documents(documents)
        .map_err(|error| error.to_string())?;

    Ok((search_index, page_by_id))
}

fn projected_page_search_document(page: &ProjectedPageRecord) -> SearchDocument {
    let mut terms = page.keywords.clone();
    terms.extend(page.doc_ids.iter().cloned());
    terms.extend(page.paths.iter().cloned());
    terms.extend(page.module_ids.iter().cloned());
    terms.extend(page.symbol_ids.iter().cloned());
    terms.extend(page.example_ids.iter().cloned());
    terms.extend(page.format_hints.iter().cloned());
    terms.push(page.doc_id.clone());
    terms.push(projection_kind_token(page.kind).to_string());
    terms.sort();
    terms.dedup();

    SearchDocument {
        id: page.page_id.clone(),
        title: page.title.clone(),
        kind: projection_kind_token(page.kind).to_string(),
        path: page.path.clone(),
        scope: page.repo_id.clone(),
        namespace: page.doc_id.clone(),
        terms,
    }
}

fn map_search_documents_to_pages(
    records: Vec<SearchDocument>,
    page_by_id: &HashMap<String, ProjectedPageRecord>,
    kind_filter: Option<ProjectionPageKind>,
    limit: usize,
    query: &str,
    fallback_score: Option<u8>,
) -> Vec<(u8, ProjectedPageRecord)> {
    let mut pages = Vec::new();
    for record in records {
        let Some(page) = page_by_id.get(record.id.as_str()) else {
            continue;
        };
        if !page_matches_kind(page, kind_filter) {
            continue;
        }
        let score = stable_page_score(page, query, fallback_score.unwrap_or_default());
        pages.push((score, page.clone()));
        if pages.len() >= limit {
            break;
        }
    }
    sort_ranked_pages(&mut pages);
    pages
}

fn map_fuzzy_search_documents_to_pages(
    records: Vec<FuzzyMatch<SearchDocument>>,
    page_by_id: &HashMap<String, ProjectedPageRecord>,
    kind_filter: Option<ProjectionPageKind>,
    limit: usize,
    query: &str,
) -> Vec<(u8, ProjectedPageRecord)> {
    let mut pages = Vec::new();
    for record in records {
        let Some(page) = page_by_id.get(record.item.id.as_str()) else {
            continue;
        };
        if !page_matches_kind(page, kind_filter) {
            continue;
        }
        let score = stable_page_score(page, query, fuzzy_match_score(record.score));
        pages.push((score, page.clone()));
        if pages.len() >= limit {
            break;
        }
    }
    sort_ranked_pages(&mut pages);
    pages
}

pub fn scored_projected_page_matches(
    query: &str,
    kind_filter: Option<ProjectionPageKind>,
    analysis: &RepositoryAnalysisOutput,
) -> Vec<(u8, ProjectedPageRecord)> {
    ranked_projected_page_matches(
        query,
        kind_filter,
        analysis,
        usize::MAX,
        FuzzySearchOptions::document_search(),
    )
}

fn heuristic_projected_page_matches(
    query: &str,
    kind_filter: Option<ProjectionPageKind>,
    pages: &[ProjectedPageRecord],
) -> Vec<(u8, ProjectedPageRecord)> {
    let mut matches = Vec::new();

    for page in pages {
        if !page_matches_kind(page, kind_filter) {
            continue;
        }

        let score = calculate_search_score(page, query);
        if score > 0 {
            matches.push((score, page.clone()));
        }
    }

    matches.sort_by(
        |(left_score, left_page): &(u8, ProjectedPageRecord),
         (right_score, right_page): &(u8, ProjectedPageRecord)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_page.title.cmp(&right_page.title))
                .then_with(|| left_page.page_id.cmp(&right_page.page_id))
        },
    );

    matches
}

fn lexical_projected_page_matches(
    query: &str,
    kind_filter: Option<ProjectionPageKind>,
    pages: &[ProjectedPageRecord],
    limit: usize,
    options: FuzzySearchOptions,
) -> Vec<(u8, ProjectedPageRecord)> {
    let filtered_pages = pages
        .iter()
        .filter(|page| page_matches_kind(page, kind_filter))
        .cloned()
        .collect::<Vec<_>>();

    fn projected_page_title(page: &ProjectedPageRecord) -> &str {
        page.title.as_str()
    }

    let matcher = LexicalMatcher::new(filtered_pages.as_slice(), projected_page_title, options);
    let mut matches = matcher
        .search(query, limit)
        .expect("lexical matcher is infallible")
        .into_iter()
        .map(|matched_page| {
            (
                stable_page_score(
                    &matched_page.item,
                    &query.to_ascii_lowercase(),
                    fuzzy_match_score(matched_page.score),
                ),
                matched_page.item,
            )
        })
        .collect::<Vec<_>>();
    sort_ranked_pages(&mut matches);
    matches
}

fn calculate_search_score(page: &ProjectedPageRecord, query: &str) -> u8 {
    let title_lc = page.title.to_ascii_lowercase();
    if title_lc == query {
        return 100;
    }
    if title_lc.starts_with(query) {
        return 85;
    }
    if title_lc.contains(query) {
        return 70;
    }

    if page
        .keywords
        .iter()
        .any(|keyword: &String| keyword.to_ascii_lowercase().contains(query))
    {
        return 60;
    }

    if page.path.to_ascii_lowercase().contains(query) {
        return 40;
    }

    0
}

fn page_matches_kind(page: &ProjectedPageRecord, kind_filter: Option<ProjectionPageKind>) -> bool {
    match kind_filter {
        None => true,
        Some(kind) => page.kind == kind,
    }
}

fn projection_kind_token(kind: ProjectionPageKind) -> &'static str {
    match kind {
        ProjectionPageKind::Reference => "reference",
        ProjectionPageKind::HowTo => "howto",
        ProjectionPageKind::Tutorial => "tutorial",
        ProjectionPageKind::Explanation => "explanation",
    }
}

fn sort_ranked_pages(matches: &mut [(u8, ProjectedPageRecord)]) {
    matches.sort_by(
        |(left_score, left_page): &(u8, ProjectedPageRecord),
         (right_score, right_page): &(u8, ProjectedPageRecord)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_page.title.cmp(&right_page.title))
                .then_with(|| left_page.page_id.cmp(&right_page.page_id))
        },
    );
}

fn stable_page_score(page: &ProjectedPageRecord, query: &str, fallback_score: u8) -> u8 {
    calculate_search_score(page, query).max(fallback_score)
}

fn fuzzy_match_score(score: f32) -> u8 {
    let bounded = score.clamp(0.0, 1.0);
    let scaled = 45.0 + (bounded * 35.0);
    scaled as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projected_page_search_uses_shared_tantivy_fuzzy_index_for_typos() {
        let analysis = test_analysis(vec![test_page(
            "repo:projection:reference:solve",
            "Solve Linear Systems",
            "docs/solve.md",
            ProjectionPageKind::Reference,
            vec!["solver".to_string(), "matrix".to_string()],
        )]);

        let matches = ranked_projected_page_matches(
            "slove",
            Some(ProjectionPageKind::Reference),
            &analysis,
            10,
            FuzzySearchOptions::document_search(),
        );

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.title, "Solve Linear Systems");
        assert_eq!(matches[0].1.path, "docs/solve.md");
        assert!(matches[0].0 >= 45);
    }

    #[test]
    fn scored_projected_page_matches_preserves_keyword_fallback() {
        let pages = vec![test_page(
            "repo:projection:reference:solve",
            "Linear Systems",
            "docs/solve.md",
            ProjectionPageKind::Reference,
            vec!["solver".to_string(), "matrix".to_string()],
        )];

        let matches = heuristic_projected_page_matches(
            "solver",
            Some(ProjectionPageKind::Reference),
            pages.as_slice(),
        );

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, 60);
    }

    #[test]
    fn scored_projected_page_matches_exposes_fuzzy_ranked_hits_for_consumers() {
        let analysis = test_analysis(vec![test_page(
            "repo:projection:reference:solve",
            "Solve Linear Systems",
            "docs/solve.md",
            ProjectionPageKind::Reference,
            vec!["solver".to_string(), "matrix".to_string()],
        )]);

        let matches =
            scored_projected_page_matches("slove", Some(ProjectionPageKind::Reference), &analysis);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.title, "Solve Linear Systems");
        assert_eq!(matches[0].1.path, "docs/solve.md");
        assert!(matches[0].0 >= 45);
    }

    fn test_page(
        page_id: &str,
        title: &str,
        path: &str,
        kind: ProjectionPageKind,
        keywords: Vec<String>,
    ) -> ProjectedPageRecord {
        ProjectedPageRecord {
            repo_id: "repo".to_string(),
            page_id: page_id.to_string(),
            kind,
            title: title.to_string(),
            doc_ids: vec![page_id.to_string()],
            paths: vec![path.to_string()],
            format_hints: vec!["reference".to_string()],
            doc_id: format!("{page_id}:doc"),
            path: path.to_string(),
            keywords,
            ..ProjectedPageRecord::default()
        }
    }

    fn test_analysis(pages: Vec<ProjectedPageRecord>) -> RepositoryAnalysisOutput {
        RepositoryAnalysisOutput {
            docs: pages
                .into_iter()
                .map(|page| crate::analyzers::DocRecord {
                    repo_id: page.repo_id,
                    doc_id: page.doc_id,
                    title: page.title,
                    path: page.path,
                    format: page.format_hints.first().cloned(),
                })
                .collect(),
            ..RepositoryAnalysisOutput::default()
        }
    }
}
