use crate::PACKAGE_NAME;
use crate::recipe::Recipe;
use anyhow::Context;
use directories::ProjectDirs;
use std::path::PathBuf;

#[non_exhaustive]
pub struct Directories {
    sources: PathBuf,
    targets: PathBuf,
}

impl Directories {
    pub fn new() -> anyhow::Result<Directories> {
        let project_directories =
            ProjectDirs::from("", "", PACKAGE_NAME).context("determining home directory")?;

        Ok(Directories {
            sources: project_directories.cache_dir().join("sources"),
            targets: project_directories.cache_dir().join("targets"),
        })
    }

    pub fn source_directory(&self, recipe: &Recipe) -> PathBuf {
        self.sources.join(&*recipe.name)
    }

    pub fn target_directory(&self, recipe: &Recipe) -> PathBuf {
        self.targets.join(&*recipe.name)
    }
}
