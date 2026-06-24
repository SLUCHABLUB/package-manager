mod arguments;

use crate::arguments::Arguments;
use anyhow::Context;
use clap::Parser;
use directories::ProjectDirs;
use fs_err::read_to_string;
use package_manager::PACKAGE_NAME;
use package_manager::prepare_install;
use package_manager::recipe::Recipe;
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
    let project_directories = ProjectDirs::from_path(PathBuf::from(PACKAGE_NAME))
        .context("determining project directories")?;

    let recipe = read_to_string(&arguments.recipe)?;
    let recipe = toml::from_str::<Recipe>(&recipe)
        .with_context(|| format!("parsing the recipe at `{}`", arguments.recipe.display()))?;

    let (ledger, _target_directory) = prepare_install(&recipe, &project_directories)?;

    info!(
        "{}",
        toml::to_string(&ledger).context("serialising the ledger")?
    );

    Ok(())
}
