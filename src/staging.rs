use crate::HostPath;
use crate::Recipe;
use crate::State;
use crate::SystemLedger;
use anyhow::Context;
use fn_error_context::context;
use fs_err as fs;
use fs_err::create_dir_all;

pub(crate) fn stage_recipes(recipes: &[&Recipe], state: &State) -> anyhow::Result<SystemLedger> {
    let staging = &state.directories().staging;

    let mut ledger = SystemLedger::new();

    for recipe in recipes {
        stage_single(recipe, staging, &mut ledger, state)?;
    }

    let ledger_file = state
        .directories()
        .ledger_file
        .to_target_path()
        .with_root(staging);

    if let Some(parent) = ledger_file.parent() {
        create_dir_all(parent)?;
    }

    let serialised_ledger = toml::to_string(&ledger).context("serialising the ledger")?;
    fs::write(ledger_file, serialised_ledger)?;

    Ok(ledger)
}

#[context("staging the `{}` recipe into `{}`", recipe.name, directory)]
fn stage_single(
    recipe: &Recipe,
    directory: &HostPath,
    system_ledger: &mut SystemLedger,
    state: &State,
) -> anyhow::Result<()> {
    let package_ledger = recipe.ledger.get().context("retrieving the ledger")?;

    let target = recipe.directories.target(recipe, state)?.path();

    for entry in package_ledger.files() {
        let source = entry.with_root(target);
        let destination = entry.with_root(directory);

        let destination_parent = destination
            .parent()
            .with_context(|| format!("getting the parent of `{destination}`"))?;

        // TODO: Directory permissions?
        fs::create_dir_all(destination_parent)?;
        fs::copy(source, destination)?;

        system_ledger
            .recipes
            .insert(recipe.name.clone(), package_ledger.clone());
    }

    Ok(())
}
