use std::path::Path;

#[derive(clap::Parser)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub action: Action,
}

#[derive(clap::Subcommand)]
pub(crate) enum Action {
    Update {
        // TODO: Allow the use to specify multiple.
        #[arg(long)]
        manifest: Box<Path>,
    },
}
