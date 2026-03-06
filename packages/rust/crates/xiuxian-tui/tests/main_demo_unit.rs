//! Integration harness for `xiuxian-tui` main argument parsing unit tests.

mod main_demo_module {
    pub(crate) use clap::Parser;
    pub(crate) use xiuxian_tui::cli_args::CliArgs as Args;

    mod tests;
}
