use std::collections::{BTreeMap, BTreeSet};

use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::records::{DocRecord, ExampleRecord, RelationKind};

use super::contracts::{
    ProjectionInputBundle, ProjectionPageKind, ProjectionPageSeed, projection_kind_from_doc_format,
};

#[derive(Debug, Clone, Default)]
struct TargetAnchors {
    module_ids: Vec<String>,
    symbol_ids: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct SourceAssociations {
    doc_ids: Vec<String>,
    example_ids: Vec<String>,
    doc_paths: Vec<String>,
    example_paths: Vec<String>,
    format_hints: Vec<String>,
}

/// Build deterministic projection inputs from Stage-1 Repo Intelligence output.
#[must_use]
pub fn build_projection_inputs(analysis: &RepositoryAnalysisOutput) -> ProjectionInputBundle {
    let repo_id = projection_repo_id(analysis);
    let symbol_ids_by_module = symbol_ids_by_module(analysis);
    let doc_lookup = analysis
        .docs
        .iter()
        .map(|doc| (doc.doc_id.clone(), doc))
        .collect::<BTreeMap<_, _>>();
    let example_lookup = analysis
        .examples
        .iter()
        .map(|example| (example.example_id.clone(), example))
        .collect::<BTreeMap<_, _>>();

    let mut docs_by_target = BTreeMap::<String, SourceAssociations>::new();
    let mut targets_by_doc = BTreeMap::<String, TargetAnchors>::new();
    let mut examples_by_target = BTreeMap::<String, SourceAssociations>::new();
    let mut targets_by_example = BTreeMap::<String, TargetAnchors>::new();

    for relation in &analysis.relations {
        match relation.kind {
            RelationKind::Documents => {
                if let Some(doc) = doc_lookup.get(&relation.source_id) {
                    attach_doc_source(
                        docs_by_target
                            .entry(relation.target_id.clone())
                            .or_default(),
                        doc,
                    );
                    attach_target(
                        targets_by_doc
                            .entry(relation.source_id.clone())
                            .or_default(),
                        &relation.target_id,
                    );
                }
            }
            RelationKind::ExampleOf => {
                if let Some(example) = example_lookup.get(&relation.source_id) {
                    attach_example_source(
                        examples_by_target
                            .entry(relation.target_id.clone())
                            .or_default(),
                        example,
                    );
                    attach_target(
                        targets_by_example
                            .entry(relation.source_id.clone())
                            .or_default(),
                        &relation.target_id,
                    );
                }
            }
            _ => {}
        }
    }

    let mut pages = Vec::new();

    for module in &analysis.modules {
        let docs = source_associations_for_module(
            &docs_by_target,
            &module.module_id,
            symbol_ids_by_module.get(&module.module_id),
        );
        let examples = source_associations_for_module(
            &examples_by_target,
            &module.module_id,
            symbol_ids_by_module.get(&module.module_id),
        );
        pages.push(ProjectionPageSeed {
            repo_id: repo_id.clone(),
            page_id: format!(
                "repo:{repo_id}:projection:reference:module:{}",
                module.module_id
            ),
            kind: ProjectionPageKind::Reference,
            title: module.qualified_name.clone(),
            module_ids: vec![module.module_id.clone()],
            symbol_ids: Vec::new(),
            example_ids: examples.example_ids,
            doc_ids: docs.doc_ids,
            paths: sorted_strings(
                [module.path.clone()],
                docs.doc_paths,
                examples.example_paths,
            ),
            format_hints: sorted_strings(Vec::<String>::new(), docs.format_hints, Vec::new()),
        });
    }

    for symbol in &analysis.symbols {
        let docs = docs_by_target
            .get(&symbol.symbol_id)
            .cloned()
            .unwrap_or_default();
        let examples = examples_by_target
            .get(&symbol.symbol_id)
            .cloned()
            .unwrap_or_default();
        pages.push(ProjectionPageSeed {
            repo_id: repo_id.clone(),
            page_id: format!(
                "repo:{repo_id}:projection:reference:symbol:{}",
                symbol.symbol_id
            ),
            kind: ProjectionPageKind::Reference,
            title: symbol.qualified_name.clone(),
            module_ids: symbol.module_id.clone().into_iter().collect(),
            symbol_ids: vec![symbol.symbol_id.clone()],
            example_ids: examples.example_ids,
            doc_ids: docs.doc_ids,
            paths: sorted_strings(
                [symbol.path.clone()],
                docs.doc_paths,
                examples.example_paths,
            ),
            format_hints: sorted_strings(Vec::<String>::new(), docs.format_hints, Vec::new()),
        });
    }

    for example in &analysis.examples {
        let targets = targets_by_example
            .get(&example.example_id)
            .cloned()
            .unwrap_or_default();
        let related_docs = source_associations_for_targets(&docs_by_target, &targets);
        pages.push(ProjectionPageSeed {
            repo_id: repo_id.clone(),
            page_id: format!(
                "repo:{repo_id}:projection:howto:example:{}",
                example.example_id
            ),
            kind: ProjectionPageKind::HowTo,
            title: example.title.clone(),
            module_ids: targets.module_ids,
            symbol_ids: targets.symbol_ids,
            example_ids: vec![example.example_id.clone()],
            doc_ids: related_docs.doc_ids,
            paths: sorted_strings([example.path.clone()], related_docs.doc_paths, Vec::new()),
            format_hints: sorted_strings(
                Vec::<String>::new(),
                related_docs.format_hints,
                Vec::new(),
            ),
        });
    }

    for doc in &analysis.docs {
        let targets = targets_by_doc.get(&doc.doc_id).cloned().unwrap_or_default();
        let related_examples = source_associations_for_targets(&examples_by_target, &targets);
        let kind = doc_projection_kind(doc, &targets);
        pages.push(ProjectionPageSeed {
            repo_id: repo_id.clone(),
            page_id: format!(
                "repo:{repo_id}:projection:{}:doc:{}",
                projection_kind_token(kind),
                doc.doc_id
            ),
            kind,
            title: doc.title.clone(),
            module_ids: targets.module_ids,
            symbol_ids: targets.symbol_ids,
            example_ids: related_examples.example_ids,
            doc_ids: vec![doc.doc_id.clone()],
            paths: sorted_strings(
                [doc.path.clone()],
                related_examples.example_paths,
                Vec::new(),
            ),
            format_hints: doc.format.clone().into_iter().collect(),
        });
    }

    pages.sort_by(|left, right| {
        projection_kind_token(left.kind)
            .cmp(projection_kind_token(right.kind))
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| left.page_id.cmp(&right.page_id))
    });

    ProjectionInputBundle { repo_id, pages }
}

fn projection_repo_id(analysis: &RepositoryAnalysisOutput) -> String {
    analysis
        .repository
        .as_ref()
        .map(|repository| repository.repo_id.clone())
        .or_else(|| {
            analysis
                .modules
                .first()
                .map(|module| module.repo_id.clone())
        })
        .or_else(|| {
            analysis
                .symbols
                .first()
                .map(|symbol| symbol.repo_id.clone())
        })
        .or_else(|| {
            analysis
                .examples
                .first()
                .map(|example| example.repo_id.clone())
        })
        .or_else(|| analysis.docs.first().map(|doc| doc.repo_id.clone()))
        .unwrap_or_default()
}

fn projection_kind_token(kind: ProjectionPageKind) -> &'static str {
    match kind {
        ProjectionPageKind::Reference => "reference",
        ProjectionPageKind::HowTo => "howto",
        ProjectionPageKind::Tutorial => "tutorial",
        ProjectionPageKind::Explanation => "explanation",
    }
}

fn doc_projection_kind(doc: &DocRecord, targets: &TargetAnchors) -> ProjectionPageKind {
    let kind = projection_kind_from_doc_format(doc.format.as_deref());
    if kind == ProjectionPageKind::Explanation && !targets.symbol_ids.is_empty() {
        ProjectionPageKind::Reference
    } else {
        kind
    }
}

fn symbol_ids_by_module(analysis: &RepositoryAnalysisOutput) -> BTreeMap<String, Vec<String>> {
    let mut symbol_ids = BTreeMap::<String, Vec<String>>::new();
    for symbol in &analysis.symbols {
        let Some(module_id) = symbol.module_id.as_ref() else {
            continue;
        };
        push_unique(
            symbol_ids.entry(module_id.clone()).or_default(),
            symbol.symbol_id.clone(),
        );
    }
    symbol_ids
}

fn source_associations_for_module(
    by_target: &BTreeMap<String, SourceAssociations>,
    module_id: &str,
    symbol_ids: Option<&Vec<String>>,
) -> SourceAssociations {
    let mut target_ids = vec![module_id.to_string()];
    if let Some(symbol_ids) = symbol_ids {
        for symbol_id in symbol_ids {
            push_unique(&mut target_ids, symbol_id.clone());
        }
    }
    source_associations_for_target_ids(by_target, &target_ids)
}

fn source_associations_for_targets(
    by_target: &BTreeMap<String, SourceAssociations>,
    targets: &TargetAnchors,
) -> SourceAssociations {
    let mut target_ids = Vec::new();
    for module_id in &targets.module_ids {
        push_unique(&mut target_ids, module_id.clone());
    }
    for symbol_id in &targets.symbol_ids {
        push_unique(&mut target_ids, symbol_id.clone());
    }
    source_associations_for_target_ids(by_target, &target_ids)
}

fn source_associations_for_target_ids(
    by_target: &BTreeMap<String, SourceAssociations>,
    target_ids: &[String],
) -> SourceAssociations {
    let mut merged = SourceAssociations::default();
    for target_id in target_ids {
        let Some(associations) = by_target.get(target_id) else {
            continue;
        };
        for doc_id in &associations.doc_ids {
            push_unique(&mut merged.doc_ids, doc_id.clone());
        }
        for example_id in &associations.example_ids {
            push_unique(&mut merged.example_ids, example_id.clone());
        }
        for doc_path in &associations.doc_paths {
            push_unique(&mut merged.doc_paths, doc_path.clone());
        }
        for example_path in &associations.example_paths {
            push_unique(&mut merged.example_paths, example_path.clone());
        }
        for format_hint in &associations.format_hints {
            push_unique(&mut merged.format_hints, format_hint.clone());
        }
    }
    merged
}

fn attach_target(targets: &mut TargetAnchors, target_id: &str) {
    if target_id.contains(":module:") {
        push_unique(&mut targets.module_ids, target_id.to_string());
    } else if target_id.contains(":symbol:") {
        push_unique(&mut targets.symbol_ids, target_id.to_string());
    }
}

fn attach_doc_source(associations: &mut SourceAssociations, doc: &DocRecord) {
    push_unique(&mut associations.doc_ids, doc.doc_id.clone());
    push_unique(&mut associations.doc_paths, doc.path.clone());
    if let Some(format) = &doc.format {
        push_unique(&mut associations.format_hints, format.clone());
    }
}

fn attach_example_source(associations: &mut SourceAssociations, example: &ExampleRecord) {
    push_unique(&mut associations.example_ids, example.example_id.clone());
    push_unique(&mut associations.example_paths, example.path.clone());
}

fn sorted_strings<I, J, K>(primary: I, secondary: J, tertiary: K) -> Vec<String>
where
    I: IntoIterator<Item = String>,
    J: IntoIterator<Item = String>,
    K: IntoIterator<Item = String>,
{
    let mut values = BTreeSet::new();
    values.extend(primary);
    values.extend(secondary);
    values.extend(tertiary);
    values.into_iter().collect()
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_projection_kind_honors_reference_format_without_symbol_targets() {
        let doc = DocRecord {
            repo_id: "repo".to_string(),
            doc_id: "repo:doc:solve".to_string(),
            title: "Solve Linear Systems".to_string(),
            path: "docs/solve.md".to_string(),
            format: Some("reference".to_string()),
        };

        assert_eq!(
            doc_projection_kind(&doc, &TargetAnchors::default()),
            ProjectionPageKind::Reference
        );
    }

    #[test]
    fn doc_projection_kind_upgrades_explanation_docs_when_symbol_anchored() {
        let doc = DocRecord {
            repo_id: "repo".to_string(),
            doc_id: "repo:doc:solver".to_string(),
            title: "Solver Notes".to_string(),
            path: "docs/solver.md".to_string(),
            format: None,
        };

        let targets = TargetAnchors {
            module_ids: Vec::new(),
            symbol_ids: vec!["repo:symbol:solve".to_string()],
        };

        assert_eq!(
            doc_projection_kind(&doc, &targets),
            ProjectionPageKind::Reference
        );
    }
}
