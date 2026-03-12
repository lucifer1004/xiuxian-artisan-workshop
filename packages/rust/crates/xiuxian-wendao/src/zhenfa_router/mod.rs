#[cfg(feature = "zhenfa-router")]
mod http;
mod models;
mod native;
mod rpc;

#[cfg(feature = "zhenfa-router")]
pub use http::WendaoZhenfaRouter;
pub use native::{WendaoContextExt, WendaoSearchTool, render_xml_lite_hits};
pub use rpc::{execute_search, search_from_rpc_params};
