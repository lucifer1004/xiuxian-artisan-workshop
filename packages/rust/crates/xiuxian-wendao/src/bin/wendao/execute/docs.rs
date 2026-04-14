use std::env;

use anyhow::{Context, Result};
use xiuxian_wendao::analyzers::{
    DocsNavigationOptions, DocsRetrievalContextOptions, DocsToolService,
};

use crate::helpers::emit;
use crate::types::{Cli, Command, DocsCommand};

pub(super) fn handle(cli: &Cli) -> Result<()> {
    let Command::Docs { command } = &cli.command else {
        unreachable!("docs handler called with non-docs command");
    };

    match command {
        DocsCommand::Page(args) => {
            let cwd = env::current_dir()?;
            let service = DocsToolService::from_project_root(cwd, args.repo.clone())
                .with_optional_config_path(cli.config_file.clone());
            let result = service
                .get_document(&args.page_id)
                .with_context(|| format!("failed to open docs page `{}`", args.page_id))?;
            emit(&result, cli.output)
        }
        DocsCommand::Tree(args) => {
            let cwd = env::current_dir()?;
            let service = DocsToolService::from_project_root(cwd, args.repo.clone())
                .with_optional_config_path(cli.config_file.clone());
            let result = service
                .get_document_structure(&args.page_id)
                .with_context(|| {
                    format!(
                        "failed to open docs page-index tree for page `{}`",
                        args.page_id
                    )
                })?;
            emit(&result, cli.output)
        }
        DocsCommand::TreeOutline(args) => {
            let cwd = env::current_dir()?;
            let service = DocsToolService::from_project_root(cwd, args.repo.clone())
                .with_optional_config_path(cli.config_file.clone());
            let result = service
                .get_document_structure_outline(&args.page_id)
                .with_context(|| {
                    format!(
                        "failed to open docs text-free page-index tree for page `{}`",
                        args.page_id
                    )
                })?;
            emit(&result, cli.output)
        }
        DocsCommand::StructureCatalog(args) => {
            let cwd = env::current_dir()?;
            let service = DocsToolService::from_project_root(cwd, args.repo.clone())
                .with_optional_config_path(cli.config_file.clone());
            let result = service.get_document_structure_catalog().with_context(|| {
                format!(
                    "failed to open docs text-free structure catalog for repo `{}`",
                    args.repo
                )
            })?;
            emit(&result, cli.output)
        }
        DocsCommand::Segment(args) => {
            let cwd = env::current_dir()?;
            let service = DocsToolService::from_project_root(cwd, args.repo.clone())
                .with_optional_config_path(cli.config_file.clone());
            let result = service
                .get_document_segment(&args.page_id, args.line_start, args.line_end)
                .with_context(|| {
                    format!(
                        "failed to open docs segment {}-{} for page `{}`",
                        args.line_start, args.line_end, args.page_id
                    )
                })?;
            emit(&result, cli.output)
        }
        DocsCommand::SearchStructure(args) => {
            let cwd = env::current_dir()?;
            let service = DocsToolService::from_project_root(cwd, args.repo.clone())
                .with_optional_config_path(cli.config_file.clone());
            let result = service
                .search_document_structure(&args.query, args.kind.map(Into::into), args.limit)
                .with_context(|| {
                    format!(
                        "failed to search docs page-index structure for query `{}`",
                        args.query
                    )
                })?;
            emit(&result, cli.output)
        }
        DocsCommand::Node(args) => {
            let cwd = env::current_dir()?;
            let service = DocsToolService::from_project_root(cwd, args.repo.clone())
                .with_optional_config_path(cli.config_file.clone());
            let result = service
                .get_document_node(&args.page_id, &args.node_id)
                .with_context(|| {
                    format!(
                        "failed to open docs page-index node `{}` for page `{}`",
                        args.node_id, args.page_id
                    )
                })?;
            emit(&result, cli.output)
        }
        DocsCommand::Toc(args) => {
            let cwd = env::current_dir()?;
            let service = DocsToolService::from_project_root(cwd, args.repo.clone())
                .with_optional_config_path(cli.config_file.clone());
            let result = service.get_toc_documents().with_context(|| {
                format!(
                    "failed to open docs markdown TOC documents for repo `{}`",
                    args.repo
                )
            })?;
            emit(&result, cli.output)
        }
        DocsCommand::Navigation(args) => {
            let cwd = env::current_dir()?;
            let service = DocsToolService::from_project_root(cwd, args.repo.clone())
                .with_optional_config_path(cli.config_file.clone());
            let result = service
                .get_navigation_with_options(
                    &args.page_id,
                    DocsNavigationOptions {
                        node_id: args.node_id.clone(),
                        family_kind: args.family_kind.map(Into::into),
                        related_limit: args.related_limit,
                        family_limit: args.family_limit,
                    },
                )
                .with_context(|| {
                    format!(
                        "failed to open docs navigation bundle for page `{}`",
                        args.page_id
                    )
                })?;
            emit(&result, cli.output)
        }
        DocsCommand::Context(args) => {
            let cwd = env::current_dir()?;
            let service = DocsToolService::from_project_root(cwd, args.repo.clone())
                .with_optional_config_path(cli.config_file.clone());
            let result = service
                .get_retrieval_context_with_options(
                    &args.page_id,
                    DocsRetrievalContextOptions {
                        node_id: args.node_id.clone(),
                        related_limit: args.related_limit,
                    },
                )
                .with_context(|| {
                    format!(
                        "failed to open docs retrieval context for page `{}`",
                        args.page_id
                    )
                })?;
            emit(&result, cli.output)
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/bin/wendao/execute/docs.rs"]
mod tests;
