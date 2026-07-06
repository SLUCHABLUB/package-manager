use crate::Ledger;
use crate::Recipe;
use crate::State;
use crate::build;
use crate::check_runtime_dependencies;
use crate::download;
use tracing::info;

pub(crate) fn prepare_to_install<'state>(
    recipe: &'state Recipe,
    state: &'state State,
) -> anyhow::Result<Ledger> {
    let name = &recipe.name;
    let directories = &recipe.directories;

    directories
        .source(recipe.download_lock(state)?, state)?
        .as_populated_then_run_or_populate_with(
            |_| info!("using the cached source directory for the `{name}` recipe"),
            |into| {
                info!("downloading the source code for the `{name}` recipe");
                download(recipe, into, state)?;
                info!("downloaded the source code for the `{name}` recipe");
                anyhow::Ok(())
            },
        )?;

    directories
        .target(recipe, state)?
        .as_populated_then_run_or_populate_with(
            |_| info!("using the cached target directory for the `{name}` recipe"),
            |into| {
                info!("building the `{name}` recipe");
                build(recipe, into, state)?;
                info!("built the `{name}` recipe");
                anyhow::Ok(())
            },
        )?;

    let ledger = Ledger::new(recipe, state)?;

    check_runtime_dependencies(
        &ledger,
        recipe.directories.target(recipe, state)?.path(),
        recipe,
    )?;

    info!("the `{}` recipe is ready to install", recipe.name);

    Ok(ledger)
}
