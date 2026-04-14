mod context;
mod navigation;
mod node;
mod page;
mod search_structure;
mod segment;
mod structure_catalog;
mod toc;
mod tree;
mod tree_outline;

use clap::Subcommand;

pub(crate) use self::context::DocsContextArgs;
pub(crate) use self::navigation::DocsNavigationArgs;
pub(crate) use self::node::DocsNodeArgs;
pub(crate) use self::page::DocsPageArgs;
pub(crate) use self::search_structure::DocsSearchStructureArgs;
pub(crate) use self::segment::DocsSegmentArgs;
pub(crate) use self::structure_catalog::DocsStructureCatalogArgs;
pub(crate) use self::toc::DocsTocArgs;
pub(crate) use self::tree::DocsTreeArgs;
pub(crate) use self::tree_outline::DocsTreeOutlineArgs;

#[derive(Debug, Subcommand, Clone)]
pub(crate) enum DocsCommand {
    /// Open one deterministic docs-facing projected page.
    Page(DocsPageArgs),
    /// Open one deterministic docs-facing projected page-index tree.
    Tree(DocsTreeArgs),
    /// Open one text-free docs-facing projected page-index tree.
    TreeOutline(DocsTreeOutlineArgs),
    /// Open one repo-scoped text-free docs-facing projected page-index tree catalog.
    StructureCatalog(DocsStructureCatalogArgs),
    /// Open one precise docs-facing projected markdown segment.
    Segment(DocsSegmentArgs),
    /// Search deterministic docs-facing projected page-index nodes.
    SearchStructure(DocsSearchStructureArgs),
    /// Open one deterministic docs-facing projected page-index node.
    Node(DocsNodeArgs),
    /// Open repository-scoped docs markdown TOC/page-index documents.
    Toc(DocsTocArgs),
    /// Open one deterministic docs-facing navigation bundle.
    Navigation(DocsNavigationArgs),
    /// Open one deterministic docs-facing retrieval context bundle.
    Context(DocsContextArgs),
}

#[cfg(test)]
pub(crate) fn docs(command: DocsCommand) -> super::Command {
    super::Command::Docs { command }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/bin/wendao/types/commands/docs.rs"]
mod tests;
