use super::{BlockKindSpecifier, MarkdownBlock, MarkdownBlockKind};

/// Internal helper so Wendao consumers can use block-kind matching without
/// relying on trait imports.
pub(crate) fn markdown_block_matches_kind(
    block: &MarkdownBlock,
    specifier: &BlockKindSpecifier,
) -> bool {
    match specifier {
        BlockKindSpecifier::Paragraph => block.kind == MarkdownBlockKind::Paragraph,
        BlockKindSpecifier::CodeFence => {
            matches!(block.kind, MarkdownBlockKind::CodeFence { .. })
        }
        BlockKindSpecifier::List => matches!(block.kind, MarkdownBlockKind::List { .. }),
        BlockKindSpecifier::BlockQuote => block.kind == MarkdownBlockKind::BlockQuote,
        BlockKindSpecifier::Item => false,
    }
}
