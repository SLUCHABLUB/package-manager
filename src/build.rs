use crate::recipe::Recipe;
use std::path::Path;

pub fn build(recipe: &Recipe, source: &Path, target: &Path) -> anyhow::Result<()> {
    // TODO: Build dependencies.
    // TODO: Sandbox.
    // TODO: Build.

    todo!(
        "build {} at {} into {}",
        recipe.name,
        source.display(),
        target.display()
    );
}
