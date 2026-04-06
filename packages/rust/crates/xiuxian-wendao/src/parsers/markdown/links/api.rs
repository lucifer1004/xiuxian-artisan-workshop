use super::scan;
use super::types::ExtractedLinkTargets;
use std::path::Path;

pub(in crate::parsers::markdown) fn extract_link_targets(
    body: &str,
    source_path: &Path,
    root: &Path,
) -> ExtractedLinkTargets {
    scan::extract_markdown_links_with_comrak(body, source_path, root)
}
