mod arguments;

use crate::arguments::Arguments;
use anyhow::Context;
use clap::Parser;
use directories::ProjectDirs;
use fs_err::read_to_string;
use package_manager::Ledger;
use package_manager::RecipeDirectories;
use package_manager::build;
use package_manager::download;
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
    let project_directories = ProjectDirs::from_path(PathBuf::from(env!("CARGO_PKG_NAME")))
        .context("determining project directories")?;

    let recipe = read_to_string(&arguments.recipe)?;
    let recipe = toml::from_str::<Recipe>(&recipe)
        .with_context(|| format!("parsing the recipe at `{}`", arguments.recipe.display()))?;

    let recipe_directories = RecipeDirectories::new(&recipe, &project_directories)
        .context("determining recipe directories")?;

    download(&recipe, &recipe_directories)
        .with_context(|| format!("downloading the source code for `{}`", recipe.name))?;

    build(&recipe, &recipe_directories).with_context(|| format!("building `{}`", recipe.name))?;

    let ledger = Ledger::new(&recipe_directories)
        .with_context(|| format!("creating ledger for package `{}`", recipe.name))?;

    info!(
        "{}",
        toml::to_string(&ledger).context("serialising the ledger")?
    );

    Ok(())
}
