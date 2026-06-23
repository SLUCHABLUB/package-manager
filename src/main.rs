mod arguments;

use crate::arguments::Arguments;
use anyhow::Context;
use clap::Parser;
use fs_err::read_to_string;
use package_manager::Directories;
use package_manager::build;
use package_manager::download;
use package_manager::recipe::Recipe;
use tracing::error;

fn main() {
    tracing_subscriber::fmt::init();

    let arguments = Arguments::parse();

    match try_main(arguments) {
        Ok(()) => (),
        Err(error) => error!("{:#}", error),
    };
}

fn try_main(arguments: Arguments) -> anyhow::Result<()> {
    let recipe = read_to_string(&arguments.recipe)?;
    let recipe = toml::from_str::<Recipe>(&recipe)
        .with_context(|| format!("parsing the recipe at `{}`", arguments.recipe.display()))?;

    let directories = Directories::new().context("determining user directories")?;

    download(
        &recipe,
        &directories.repository_directory(&recipe),
        &directories.source_directory(&recipe),
    )
    .with_context(|| format!("downloading the source code for `{}`", recipe.name))?;

    build(
        &recipe,
        &directories.source_directory(&recipe),
        &directories.target_directory(&recipe),
    )
    .with_context(|| format!("building `{}`", recipe.name))?;

    Ok(())
}
