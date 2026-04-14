use clap::Args;

#[derive(Args, Debug, Clone)]
pub(crate) struct DocsStructureCatalogArgs {
    #[arg(long)]
    pub repo: String,
}
