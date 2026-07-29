use crate::Image;
use crate::ImageLedger;
use crate::Recipe;
use crate::check_runtime_dependencies;
use tracing::info;

pub(crate) fn check_image(recipe: &Recipe, image: &Image) -> anyhow::Result<ImageLedger> {
    info!("checking {}", recipe.name());

    let ledger = ImageLedger::new(recipe, image)?;

    check_runtime_dependencies(&ledger, image, recipe)?;

    info!("{} is ready for staging", recipe.name());

    Ok(ledger)
}
