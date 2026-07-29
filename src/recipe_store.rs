use crate::Recipe;
use crate::VersionRequirement;
use anyhow::anyhow;
use fn_error_context::context;

pub struct RecipeStore {
    recipes: Box<[Recipe]>,
}

impl RecipeStore {
    pub(crate) fn from_recipes(recipes: impl IntoIterator<Item = Recipe>) -> RecipeStore {
        RecipeStore {
            recipes: recipes.into_iter().collect(),
        }
    }

    #[context("searching for a recipe for the `{name}` package matching version {version}")]
    pub(crate) fn find(&self, name: &str, version: &VersionRequirement) -> anyhow::Result<&Recipe> {
        self.recipes
            .iter()
            .find(|recipe| recipe.name() == name && recipe.version().satisfies(version))
            .ok_or_else(|| anyhow!("no recipe found"))
    }
}
