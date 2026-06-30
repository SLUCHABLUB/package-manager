use crate::State;
use crate::recipe::Recipe;
use std::path::PathBuf;

// TODO: Make this opaque.
#[derive(Debug)]
pub struct RecipeDirectories {
    /// The path to the source code.
    pub source: PathBuf,
    /// The path to the (to be) built package tree.
    pub target: PathBuf,
    /// The path to the bare repository (`.git` directory).
    pub repository: PathBuf,
    /// The path to the working directory of the build.
    pub build: PathBuf,
}

impl RecipeDirectories {
    pub(crate) fn new(recipe: &Recipe, state: &State) -> anyhow::Result<RecipeDirectories> {
        // TODO: Base these on `recipe.download` to allow for cache reuse.
        Ok(RecipeDirectories {
            source: state.cache_directory().join("sources").join(&*recipe.name),
            target: state.cache_directory().join("targets").join(&*recipe.name),
            repository: state
                .cache_directory()
                .join("repositories")
                .join(&*recipe.name)
                .with_added_extension("git"),
            build: state.cache_directory().join("build").join(&*recipe.name),
        })
    }
}
