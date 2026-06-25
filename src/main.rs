mod arguments;

use crate::arguments::Arguments;
use anyhow::Context as _;
use anyhow::anyhow;
use clap::Parser;
use directories::ProjectDirs;
use fs_err::read_to_string;
use package_manager::Manifest;
use package_manager::PACKAGE_NAME;
use package_manager::ResultExtension as _;
use std::path::PathBuf;

fn main() {
    tracing_subscriber::fmt::init();

    let arguments = Arguments::parse();

    try_main(arguments).ok_or_log();
}

fn try_main(arguments: Arguments) -> anyhow::Result<()> {
    // Use a pure rust cryptography provider for rustls to avoid a C-compiler build dependency.
    rustls_rustcrypto::provider()
        .install_default()
        .map_err(|_provider| anyhow!("failed to set the rustls cryptography provider"))?;

    let project_directories = ProjectDirs::from_path(PathBuf::from(PACKAGE_NAME))
        .context("determining project directories")?;

    let manifest = read_to_string(&arguments.manifest)?;
    let manifest = toml::from_str::<Manifest>(&manifest)?;

    manifest
        .package_set()?
        .prepare_install(&project_directories)?;

    Ok(())
}
