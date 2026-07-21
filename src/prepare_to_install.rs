use crate::Ledger;
use crate::Recipe;
use crate::State;
use crate::check_runtime_dependencies;
use crate::ensure_built;
use tracing::info;

pub(crate) fn prepare_to_install<'state>(
    recipe: &'state Recipe,
    state: &'state State,
) -> anyhow::Result<()> {
    ensure_built(recipe, state)?;

    let ledger = recipe
        .ledger
        .get_or_try_init(|| Ledger::new(recipe, state))?;

    check_runtime_dependencies(
        ledger,
        recipe.directories.target(recipe, state)?.path(),
        recipe,
    )?;

    info!("the `{}` recipe is ready to install", recipe.name);

    Ok(())
}
