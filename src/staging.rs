use crate::Recipe;
use crate::State;
use anyhow::Context;
use fn_error_context::context;
use fs_err::copy;
use fs_err::create_dir_all;
use std::path::Path;

pub(crate) fn stage_recipes(
    recipes: &[&Recipe],
    staging: &Path,
    state: &State,
) -> anyhow::Result<()> {
    for recipe in recipes {
        stage_single(recipe, staging, state)?;
    }

    Ok(())
}

#[context("staging the `{}` recipe into `{}`", recipe.name, staging.display())]
fn stage_single(recipe: &Recipe, staging: &Path, state: &State) -> anyhow::Result<()> {
    let ledger = recipe.ledger.get().context("retrieving the ledger")?;

    let target = recipe.directories.target(recipe, state)?.path();

    for entry in &ledger.files {
        let source = target.join(entry);
        let destination = staging.join(entry);

        let destination_parent = destination
            .parent()
            .with_context(|| format!("getting the parent of `{}`", destination.display()))?;

        // TODO: Directory permissions?
        create_dir_all(destination_parent)?;
        copy(source, destination)?;
    }

    Ok(())
}
