use crate::Recipe;
use crate::RecipeLedger;
use crate::State;
use crate::TargetDirectories;
use crate::check_runtime_dependencies;
use crate::ensure_built;
use tracing::info;

// TODO: This is really just "prepare to stage".
pub(crate) fn prepare_to_install<'state>(
    recipe: &'state Recipe,
    into: &TargetDirectories,
    state: &'state State,
) -> anyhow::Result<&'state RecipeLedger> {
    ensure_built(recipe, into, state)?;

    let ledger = recipe
        .ledger
        .get_or_try_init(|| RecipeLedger::new(recipe, state))?;

    check_runtime_dependencies(
        ledger,
        recipe.directories.target(recipe, state)?.path(),
        recipe,
    )?;

    info!("the `{}` recipe is ready to install", recipe.name);

    Ok(ledger)
}
