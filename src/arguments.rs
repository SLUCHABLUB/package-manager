use std::path::Path;

#[derive(clap::Parser)]
pub(crate) struct Arguments {
    pub manifest: Box<Path>,
}
