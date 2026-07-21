use crate::HostPath;
use crate::Recipe;
use crate::State;
use anyhow::Context;
use fn_error_context::context;
use fs_err as fs;

pub(crate) fn stage_recipes(
    recipes: &[&Recipe],
    staging: &HostPath,
    state: &State,
) -> anyhow::Result<()> {
    for recipe in recipes {
        stage_single(recipe, staging, state)?;
    }

    Ok(())
}

#[context("staging the `{}` recipe into `{}`", recipe.name, staging)]
fn stage_single(recipe: &Recipe, staging: &HostPath, state: &State) -> anyhow::Result<()> {
    let ledger = recipe.ledger.get().context("retrieving the ledger")?;

    let target = recipe.directories.target(recipe, state)?.path();

    for entry in &ledger.files {
        let source = entry.in_target(target);
        let destination = entry.in_staging(staging);

        let destination_parent = destination
            .parent()
            .with_context(|| format!("getting the parent of `{destination}`"))?;

        // TODO: Directory permissions?
        fs::create_dir_all(destination_parent)?;
        fs::copy(source, destination)?;
    }

    Ok(())
}
