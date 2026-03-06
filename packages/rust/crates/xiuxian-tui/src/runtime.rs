use log::info;

use crate::{AppState, TuiRenderer};

/// Initialize the TUI subsystem with logging.
pub fn init_logger() {
    let _ = xiuxian_logging::init("xiuxian_tui", &xiuxian_logging::LogSettings::default());
}

/// Main entry point for running the TUI application.
///
/// # Errors
/// Returns an error when renderer initialization, app bootstrap, or runtime
/// event loop fails.
pub fn run_tui<F>(title: &str, app_creator: F) -> Result<(), anyhow::Error>
where
    F: FnOnce(&mut AppState) -> Result<(), anyhow::Error>,
{
    init_logger();

    let mut renderer = TuiRenderer::new()?;
    let mut state = AppState::new(title.to_string());
    app_creator(&mut state)?;

    info!("Starting TUI application: {title}");
    renderer.run(&mut state)
}
