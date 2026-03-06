//! CLI argument model for the `xiuxian-tui` demo example.

/// Simple TUI demo for testing xiuxian-tui.
#[derive(clap::Parser, Debug)]
#[command(name = "xiuxian-tui-demo")]
#[command(author = "Omni Dev Fusion")]
#[command(version = "0.1.0")]
#[command(about = "Demo TUI for testing xiuxian-tui", long_about = None)]
pub struct DemoArgs {
    /// Unix socket path for receiving events.
    #[arg(short, long, default_value = "/tmp/xiuxian-tui.sock")]
    pub socket: String,

    /// Run in demo mode (auto-generate events).
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub demo: bool,
}
