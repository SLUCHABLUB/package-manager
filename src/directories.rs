use crate::PACKAGE_NAME;
use crate::recipe::Recipe;
use anyhow::Context;
use directories::ProjectDirs;
use fs_err::create_dir_all;
use fs_err::remove_dir_all;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[non_exhaustive]
pub struct Directories {
    sources: PathBuf,
    targets: PathBuf,
    repositories: PathBuf,
}

impl Directories {
    pub fn new() -> anyhow::Result<Directories> {
        let project_directories =
            ProjectDirs::from("", "", PACKAGE_NAME).context("determining home directory")?;

        Ok(Directories {
            sources: project_directories.cache_dir().join("sources"),
            targets: project_directories.cache_dir().join("targets"),
            repositories: project_directories.cache_dir().join("repositories"),
        })
    }

    pub fn source_directory(&self, recipe: &Recipe) -> PathBuf {
        self.sources.join(&*recipe.name)
    }

    pub fn target_directory(&self, recipe: &Recipe) -> PathBuf {
        self.targets.join(&*recipe.name)
    }

    pub fn repository_directory(&self, recipe: &Recipe) -> PathBuf {
        self.repositories
            .join(&*recipe.name)
            .with_added_extension("git")
    }

    pub fn make_empty(directory: &Path) -> anyhow::Result<()> {
        match remove_dir_all(directory) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }?;
        create_dir_all(directory)?;

        Ok(())
    }
}
