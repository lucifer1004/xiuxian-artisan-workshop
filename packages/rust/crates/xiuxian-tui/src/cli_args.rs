//! CLI argument model for the `xiuxian-tui` binary.

use xiuxian_logging::LogCliArgs;

/// Omni TUI - Headless-compatible renderer for Python Agent events.
#[derive(clap::Parser, Debug)]
#[command(name = "xiuxian-tui")]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Unix socket path for IPC.
    #[arg(short, long)]
    pub socket: String,

    /// Connection role: "server" (binds socket) or "client" (connects to Python).
    #[arg(long, default_value = "client")]
    pub role: String,

    /// Parent process PID (for cleanup on parent death).
    #[arg(short, long)]
    pub pid: Option<i32>,

    /// Run in headless mode (no TUI rendering, just process events).
    #[arg(long, default_value = "false")]
    pub headless: bool,

    /// Global structured logging controls.
    #[command(flatten)]
    pub logging: LogCliArgs,
}
