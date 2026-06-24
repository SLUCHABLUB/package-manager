use package_manager::Version;

#[derive(clap::Parser)]
pub(crate) struct Arguments {
    pub recipe: Box<str>,
    pub version: Version,
}
