//! Integration harness for `xiuxian-tui` demo CLI argument parsing tests.

mod demo_cli_args_module {
    pub(crate) use clap::Parser;
    pub(crate) use xiuxian_tui::demo_cli_args::DemoArgs;

    mod tests;
}
