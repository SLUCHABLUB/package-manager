use crate::Version;
use crate::recipe::Recipe;
use anyhow::Context;
use anyhow::anyhow;
use anyhow::bail;
use fs_err::read_dir;
use fs_err::read_to_string;
use tracing::warn;

// TODO: Cache the recipes.
pub fn find_recipe(package_name: &str, version: &Version) -> anyhow::Result<Recipe> {
    let recipe_directories = ["./examples/recipes"];
    warn!("hardcoding the recipe directories to {recipe_directories:?}");

    let mut found_recipe = None;

    for directory in recipe_directories {
        for entry in read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();

            if entry.file_type()?.is_dir() {
                warn!("skipping the directory {}", path.display());
                continue;
            }

            let recipe = read_to_string(&path)?;
            let recipe = toml::from_str::<Recipe>(&recipe)
                .with_context(|| format!("parsing `{}`", path.display()))?;

            warn!("skipping the provides field in the recipe");

            if &*recipe.name == package_name && recipe.version.satisfies(version) {
                if found_recipe.is_some() {
                    warn!("not consulting the manifest");
                    bail!("multiple recipes provide `{package_name}` version `{version}`");
                }

                found_recipe = Some(recipe);
            }
        }
    }

    found_recipe.ok_or(anyhow!(
        "could not find a recipe for `{package_name}` version `{version}`"
    ))
}
