use crate::Ledger;
use crate::RecipeDirectories;
use crate::build;
use crate::dependencies::check_runtime_dependencies;
use crate::download;
use crate::recipe::Recipe;
use anyhow::Context as _;
use directories::ProjectDirs;
use std::path::PathBuf;

pub fn prepare_install(
    recipe: &Recipe,
    project_directories: &ProjectDirs,
) -> anyhow::Result<(Ledger, PathBuf)> {
    let directories = RecipeDirectories::new(recipe, project_directories)
        .context("determining recipe directories")?;

    download(recipe, &directories)
        .with_context(|| format!("downloading the source code for `{}`", recipe.name))?;

    build(recipe, &directories).with_context(|| format!("building `{}`", recipe.name))?;

    let ledger = Ledger::new(&directories)
        .with_context(|| format!("creating ledger for package `{}`", recipe.name))?;

    check_runtime_dependencies(&ledger, &directories.target, recipe)
        .context("checking runtime dependencies")?;

    Ok((ledger, directories.target))
}
