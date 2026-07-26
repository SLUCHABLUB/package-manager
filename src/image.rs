use crate::ImageLedger;
use crate::Recipe;
use crate::State;
use crate::TargetDirectories;
use crate::check_runtime_dependencies;
use crate::ensure_built;
use tracing::info;

pub(crate) fn make_image<'state>(
    recipe: &'state Recipe,
    into: &TargetDirectories,
    state: &'state State,
) -> anyhow::Result<&'state ImageLedger> {
    ensure_built(recipe, into, state)?;

    let ledger = recipe
        .ledger()
        .get_or_try_init(|| ImageLedger::new(recipe, state))?;

    check_runtime_dependencies(
        ledger,
        recipe.directories().image(recipe, state)?.path(),
        recipe,
    )?;

    info!("{recipe} is ready to install");

    Ok(ledger)
}
