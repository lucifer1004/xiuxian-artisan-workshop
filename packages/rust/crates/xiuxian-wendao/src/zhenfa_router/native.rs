use schemars::JsonSchema;
use serde::Deserialize;
use xiuxian_zhenfa::{ZhenfaContext, ZhenfaError, zhenfa_tool};

use crate::link_graph::LinkGraphPlannedSearchPayload;
use crate::{
    AssetRequest, LinkGraphIndex, LinkGraphSearchOptions, SkillVfsResolver, WendaoAssetHandle,
};

mod xml_lite;

const DEFAULT_SEARCH_LIMIT: usize = 20;
const MAX_SEARCH_LIMIT: usize = 200;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct WendaoSearchArgs {
    query: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    root_dir: Option<String>,
    #[serde(default)]
    options: Option<LinkGraphSearchOptions>,
    #[serde(default)]
    include_provisional: Option<bool>,
    #[serde(default)]
    provisional_limit: Option<usize>,
}

/// Typed extension accessors for Wendao native tools.
pub trait WendaoContextExt {
    /// Resolve the injected immutable `LinkGraph` index from zhenfa context.
    ///
    /// # Errors
    /// Returns execution error when the index is not present in context.
    fn link_graph_index(&self) -> Result<std::sync::Arc<LinkGraphIndex>, ZhenfaError>;

    /// Resolve the injected semantic skill VFS resolver from zhenfa context.
    ///
    /// # Errors
    /// Returns execution error when resolver is not present in context.
    fn vfs(&self) -> Result<std::sync::Arc<SkillVfsResolver>, ZhenfaError>;

    /// Builds one skill-scoped asset request.
    ///
    /// # Errors
    /// Returns execution error when semantic URI mapping arguments are invalid.
    fn skill_asset(
        &self,
        semantic_name: &str,
        relative_path: &str,
    ) -> Result<AssetRequest, ZhenfaError>;
}

impl WendaoContextExt for ZhenfaContext {
    fn link_graph_index(&self) -> Result<std::sync::Arc<LinkGraphIndex>, ZhenfaError> {
        self.get_extension::<LinkGraphIndex>().ok_or_else(|| {
            ZhenfaError::execution("missing LinkGraphIndex in zhenfa context extensions")
        })
    }

    fn vfs(&self) -> Result<std::sync::Arc<SkillVfsResolver>, ZhenfaError> {
        self.get_extension::<SkillVfsResolver>().ok_or_else(|| {
            ZhenfaError::execution("missing SkillVfsResolver in zhenfa context extensions")
        })
    }

    fn skill_asset(
        &self,
        semantic_name: &str,
        relative_path: &str,
    ) -> Result<AssetRequest, ZhenfaError> {
        WendaoAssetHandle::skill_reference_asset(semantic_name, relative_path).map_err(|error| {
            ZhenfaError::invalid_arguments(format!(
                "invalid skill asset mapping (`{semantic_name}`, `{relative_path}`): {error}"
            ))
        })
    }
}

/// Search the Wendao graph index and return stripped XML-Lite `<hit>` records.
/// Native tool for searching the wendao graph index.
#[allow(missing_docs)]
#[zhenfa_tool(
    name = "wendao.search",
    description = "Search the Wendao graph index and return stripped XML-Lite <hit> records.",
    tool_struct = "WendaoSearchTool",
    mutation_scope = "wendao.search"
)]
pub async fn wendao_search(
    ctx: &ZhenfaContext,
    args: WendaoSearchArgs,
) -> Result<String, ZhenfaError> {
    let query = args.query.trim();
    if query.is_empty() {
        return Err(ZhenfaError::invalid_arguments(
            "`query` must be a non-empty string",
        ));
    }

    validate_root_dir_argument(args.root_dir.as_deref())?;
    let options = args.options.unwrap_or_default();
    let index = ctx.link_graph_index()?;
    let payload = index.search_planned_payload_with_agentic(
        query,
        normalize_limit(args.limit),
        options,
        args.include_provisional,
        args.provisional_limit,
    );
    Ok(xml_lite::render_xml_lite(&payload))
}

/// Render one planned payload into XML-Lite hit rows.
///
/// This is a thin public adapter over native XML-Lite rendering logic, used by
/// integration tests and tool-facing formatting call sites.
#[must_use]
pub fn render_xml_lite_hits(payload: &LinkGraphPlannedSearchPayload) -> String {
    xml_lite::render_xml_lite(payload)
}

fn normalize_limit(raw: Option<usize>) -> usize {
    raw.unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT)
}

fn validate_root_dir_argument(root_dir: Option<&str>) -> Result<(), ZhenfaError> {
    if let Some(value) = root_dir {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ZhenfaError::invalid_arguments(
                "`root_dir` must be non-empty when provided",
            ));
        }
    }
    Ok(())
}
