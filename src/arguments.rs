#[derive(clap::Parser)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub action: Action,
}

#[derive(clap::Subcommand)]
pub(crate) enum Action {
    Install,
}
