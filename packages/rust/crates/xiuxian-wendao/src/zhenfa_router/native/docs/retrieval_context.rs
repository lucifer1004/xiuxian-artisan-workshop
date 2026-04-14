use xiuxian_zhenfa::{ZhenfaContext, ZhenfaError, zhenfa_tool};

use super::shared::{optional_non_empty_argument, require_non_empty_argument, serialize_payload};
use crate::analyzers::{DocsRetrievalContextOptions, DocsRetrievalContextToolArgs};
use crate::zhenfa_router::native::resolve_docs_tool_runtime;

/// Arguments for the `wendao.docs.get_retrieval_context` native tool.
pub type WendaoDocsGetRetrievalContextArgs = DocsRetrievalContextToolArgs;

/// Resolve one docs-facing retrieval-context payload and return its serialized
/// result.
///
/// # Errors
///
/// Returns a [`ZhenfaError`] when arguments are invalid, the docs capability
/// service is missing from the native context, or the underlying docs lookup
/// fails.
#[allow(missing_docs)]
#[zhenfa_tool(
    name = "wendao.docs.get_retrieval_context",
    description = "Open one docs-facing retrieval-context payload and return its serialized result.",
    tool_struct = "WendaoDocsGetRetrievalContextTool"
)]
pub fn wendao_docs_get_retrieval_context(
    ctx: &ZhenfaContext,
    args: WendaoDocsGetRetrievalContextArgs,
) -> Result<String, ZhenfaError> {
    let page_id = require_non_empty_argument(&args.page_id, "page_id")?;
    let runtime = resolve_docs_tool_runtime(ctx)?;
    let mut options = DocsRetrievalContextOptions::default();
    options.node_id = optional_non_empty_argument(args.node_id, "node_id")?;
    if let Some(limit) = args.related_limit {
        options.related_limit = limit;
    }
    let result = runtime
        .get_retrieval_context_with_options(&page_id, options)
        .map_err(|error| ZhenfaError::execution(error.to_string()))?;
    serialize_payload(&result)
}
