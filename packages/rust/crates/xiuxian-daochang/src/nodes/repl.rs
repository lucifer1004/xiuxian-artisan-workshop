use std::path::PathBuf;

use xiuxian_daochang::{RuntimeSettings, build_agent, run_stdio};

pub(crate) async fn run_repl_mode(
    query: Option<String>,
    session_id: String,
    mcp_config_path: PathBuf,
    runtime_settings: &RuntimeSettings,
) -> anyhow::Result<()> {
    let agent = build_agent(&mcp_config_path, runtime_settings).await?;
    if let Some(q) = query {
        let out = agent.run_turn(&session_id, q.trim()).await?;
        println!("{out}");
        Ok(())
    } else {
        Box::pin(run_stdio(agent, session_id)).await
    }
}
