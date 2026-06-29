use crate::recipe::Recipe;
use directories::ProjectDirs;
use std::path::PathBuf;

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
    pub fn new(recipe: &Recipe, project: &ProjectDirs) -> anyhow::Result<RecipeDirectories> {
        Ok(RecipeDirectories {
            source: project.cache_dir().join("sources").join(&*recipe.name),
            target: project.cache_dir().join("targets").join(&*recipe.name),
            repository: project
                .cache_dir()
                .join("repositories")
                .join(&*recipe.name)
                .with_added_extension("git"),
            build: project.cache_dir().join("build").join(&*recipe.name),
        })
    }
}
