//! Runtime-backed Wendao Flight server binary that reads repo-search data from
//! the active search-plane store.

#[cfg(not(feature = "julia"))]
fn main() {
    eprintln!("wendao_search_flight_server requires the `julia` feature");
    std::process::exit(1);
}

#[cfg(feature = "julia")]
use std::env;
#[cfg(feature = "julia")]
use std::net::SocketAddr;
#[cfg(feature = "julia")]
use std::path::PathBuf;
#[cfg(feature = "julia")]
use std::sync::Arc;

#[cfg(feature = "julia")]
use anyhow::{Result, anyhow};
#[cfg(feature = "julia")]
use arrow_flight::flight_service_server::FlightServiceServer;
#[cfg(feature = "julia")]
use tokio::net::TcpListener;
#[cfg(feature = "julia")]
use tokio_stream::wrappers::TcpListenerStream;
#[cfg(feature = "julia")]
use tonic::transport::Server;
#[cfg(feature = "julia")]
use xiuxian_wendao::link_graph::plugin_runtime::{
    bootstrap_sample_repo_search_content, build_search_plane_flight_service,
};
#[cfg(feature = "julia")]
use xiuxian_wendao::search_plane::SearchPlaneService;

#[cfg(feature = "julia")]
#[tokio::main]
async fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let bind_addr = args
        .next()
        .unwrap_or_else(|| "127.0.0.1:0".to_string())
        .parse::<SocketAddr>()
        .map_err(|error| anyhow!("invalid bind address: {error}"))?;
    let schema_version = args.next().unwrap_or_else(|| "v2".to_string());
    let repo_id = args.next().unwrap_or_else(|| "alpha/repo".to_string());
    let project_root = match args.next() {
        Some(path) => PathBuf::from(path),
        None => {
            env::current_dir().map_err(|error| anyhow!("failed to resolve current dir: {error}"))?
        }
    };
    let rerank_dimension = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| anyhow!("invalid rerank dimension: {error}"))
        })
        .transpose()?
        .unwrap_or(3);

    let search_plane = Arc::new(SearchPlaneService::new(project_root));
    if env::var_os("WENDAO_BOOTSTRAP_SAMPLE_REPO").is_some() {
        bootstrap_sample_repo_search_content(search_plane.as_ref(), repo_id.as_str())
            .await
            .map_err(|error| anyhow!(error))?;
    }
    let flight_service =
        build_search_plane_flight_service(search_plane, repo_id, schema_version, rerank_dimension)
            .map_err(|error| anyhow!(error))?;

    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(|error| anyhow!("failed to bind Wendao search Flight server: {error}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| anyhow!("failed to read Wendao search Flight server address: {error}"))?;
    println!("READY http://{local_addr}");

    Server::builder()
        .add_service(FlightServiceServer::new(flight_service))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await
        .map_err(|error| anyhow!("Wendao search Flight server failed: {error}"))?;

    Ok(())
}
