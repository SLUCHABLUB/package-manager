use crate::Ledger;
use crate::RecipeDirectories;
use crate::State;
use crate::build;
use crate::dependencies::check_runtime_dependencies;
use crate::download;
use crate::recipe::Recipe;
use anyhow::Context as _;
use std::path::PathBuf;

pub fn prepare_to_install<'state>(
    recipe: &'state Recipe,
    state: &'state State,
) -> anyhow::Result<(Ledger, PathBuf)> {
    let directories =
        RecipeDirectories::new(recipe, state).context("determining recipe directories")?;

    download(recipe, &directories)
        .with_context(|| format!("downloading the source code for `{}`", recipe.name))?;

    build(recipe, &directories).with_context(|| format!("building `{}`", recipe.name))?;

    let ledger = Ledger::new(&directories)
        .with_context(|| format!("creating ledger for package `{}`", recipe.name))?;

    check_runtime_dependencies(&ledger, directories.target()?.path(), recipe)
        .context("checking runtime dependencies")?;

    // TODO: Don't clone.
    Ok((ledger, directories.target()?.path().to_owned()))
}
