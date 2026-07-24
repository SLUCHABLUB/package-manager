use crate::HostPath;
use crate::Recipe;
use crate::State;
use crate::SystemLedger;
use crate::TargetDirectories;
use anyhow::Context;
use fn_error_context::context;
use fs_err as fs;

pub(crate) fn stage_recipes(
    recipes: &[&Recipe],
    target_directories: &TargetDirectories,
    state: &State,
) -> anyhow::Result<SystemLedger> {
    let staging = &state.directories().staging;

    let mut ledger = SystemLedger::new(target_directories);

    for recipe in recipes {
        stage_single(recipe, staging, &mut ledger, state)?;
    }

    ledger.write_to_root(staging)?;

    Ok(ledger)
}

#[context("staging the `{}` recipe into `{}`", recipe.name, directory)]
fn stage_single(
    recipe: &Recipe,
    directory: &HostPath,
    system_ledger: &mut SystemLedger,
    state: &State,
) -> anyhow::Result<()> {
    let recipe_ledger = recipe.ledger.get().context("retrieving the ledger")?;

    let target = recipe.directories.target(recipe, state)?.path();

    for entry in &recipe_ledger.files {
        let source = entry.with_root(target);
        let destination = entry.with_root(directory);

        let destination_parent = destination
            .parent()
            .with_context(|| format!("getting the parent of `{destination}`"))?;

        // TODO: Directory permissions?
        fs::create_dir_all(destination_parent)?;
        fs::copy(source, destination)?;
    }

    system_ledger.add_recipe(recipe_ledger.clone());

    Ok(())
}
