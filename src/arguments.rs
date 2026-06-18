use std::path::Path;

#[derive(clap::Parser)]
pub(crate) struct Arguments {
    #[arg(long)]
    pub install_recipe: Box<Path>,
}
