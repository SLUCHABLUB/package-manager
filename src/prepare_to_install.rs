use crate::Ledger;
use crate::RecipeDirectories;
use crate::State;
use crate::build;
use crate::dependencies::check_runtime_dependencies;
use crate::download;
use crate::recipe::Recipe;
use std::path::PathBuf;
use tracing::info;

pub(crate) fn prepare_to_install<'state>(
    recipe: &'state Recipe,
    state: &'state State,
) -> anyhow::Result<(Ledger, PathBuf)> {
    let directories = RecipeDirectories::new(recipe, state);

    info!(
        "downloading the source code for the `{}` recipe",
        recipe.name
    );
    download(recipe, &directories)?;
    info!(
        "downloaded the source code for the `{}` recipe",
        recipe.name
    );

    info!("building the `{}` recipe", recipe.name);
    build(recipe, &directories)?;
    info!("built the `{}` recipe", recipe.name);

    let ledger = Ledger::new(&directories)?;

    check_runtime_dependencies(&ledger, directories.target()?.path(), recipe)?;

    info!("the `{}` recipe is ready to install", recipe.name);

    // TODO: Don't clone.
    Ok((ledger, directories.target()?.path().to_owned()))
}
