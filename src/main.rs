mod arguments;

use crate::arguments::Arguments;
use anyhow::Context as _;
use anyhow::anyhow;
use clap::Parser;
use directories::ProjectDirs;
use package_manager::PACKAGE_NAME;
use package_manager::find_recipe;
use package_manager::prepare_install;
use std::path::PathBuf;
use tracing::error;
use tracing::info;

fn main() {
    tracing_subscriber::fmt::init();

    let arguments = Arguments::parse();

    match try_main(arguments) {
        Ok(()) => (),
        Err(error) => error!("{:#}", error),
    };
}

fn try_main(arguments: Arguments) -> anyhow::Result<()> {
    // Use a pure rust cryptography provider for rustls to avoid a C-compiler build dependency.
    rustls_rustcrypto::provider()
        .install_default()
        .map_err(|_provider| anyhow!("failed to set the rustls cryptography provider"))?;

    let project_directories = ProjectDirs::from_path(PathBuf::from(PACKAGE_NAME))
        .context("determining project directories")?;

    let recipe = find_recipe(&arguments.recipe, &arguments.version)?;

    let (ledger, _target_directory) = prepare_install(&recipe, &project_directories)?;

    info!(
        "{}",
        toml::to_string(&ledger).context("serialising the ledger")?
    );

    Ok(())
}
