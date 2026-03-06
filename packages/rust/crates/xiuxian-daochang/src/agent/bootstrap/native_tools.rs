use std::sync::Arc;

use xiuxian_wendao::SpiderWendaoBridge;

use super::service_mount::{ServiceMountCatalog, ServiceMountMeta};
use super::zhixing::register_zhixing_native_tools;
use crate::agent::native_tools::macros::register_native_tools;

/// Unified Native Tool cauldron: central registration entrypoint.
pub(super) fn mount_native_tool_cauldron(
    heyi: Option<&Arc<super::super::ZhixingHeyi>>,
    native_tools: &mut super::super::NativeToolRegistry,
    mounts: &mut ServiceMountCatalog,
) {
    if let Some(runtime) = heyi {
        register_zhixing_native_tools(runtime, native_tools, mounts);
    } else {
        mounts.skipped(
            "zhixing.native_tools",
            "tooling",
            ServiceMountMeta::default().detail("heyi runtime unavailable"),
        );
    }

    let ingress = heyi.map(|runtime| {
        let graph = runtime.graph.as_ref().clone();
        Arc::new(SpiderWendaoBridge::for_knowledge_graph(graph))
    });

    register_native_tools!(
        native_tools,
        super::super::native_tools::spider::SpiderCrawlTool { ingress }
    );

    let detail = if heyi.is_some() {
        "tools=web.crawl,ingress=wendao.graph"
    } else {
        "tools=web.crawl,ingress=disabled(no_heyi_graph)"
    };
    mounts.mounted(
        "web.native_tools",
        "tooling",
        ServiceMountMeta::default().detail(detail),
    );
}
