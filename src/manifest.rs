use crate::Recipe;
use crate::ResultExtension as _;
use crate::VersionRequirement;
use anyhow::Context;
use fs_err::read_dir;
use fs_err::read_to_string;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use tracing::warn;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Manifest {
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(skip)]
    pub parent_directory: PathBuf,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub packages: HashMap<Box<str>, VersionRequirement>,
    /// A map from package name to recipe name.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub providers: HashMap<Box<str>, Box<str>>,
    #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
    pub recipe_directories: Box<[Box<Path>]>,
}

impl Manifest {
    pub(crate) fn read_from(path: PathBuf) -> anyhow::Result<Manifest> {
        let manifest = read_to_string(&path)?;
        let mut manifest: Manifest = toml::from_str(&manifest)?;

        path.parent()
            .with_context(|| format!("getting the parent of `{}`", path.display()))?
            .clone_into(&mut manifest.parent_directory);
        manifest.path = path;

        Ok(manifest)
    }

    pub(crate) fn read_recipes(&self) -> impl Iterator<Item = Recipe> {
        self.recipe_directories
            .iter()
            .filter_map(|directory| {
                Some(
                    read_dir(self.parent_directory.join(directory))
                        .ok_or_log()?
                        .filter_map(|entry| {
                            let entry = entry.ok_or_log()?;
                            let path = entry.path();

                            if entry.file_type().ok_or_log()?.is_dir() {
                                warn!("skipping the directory {}", path.display());
                                return None;
                            }

                            Recipe::read_from(&path).ok_or_log()
                        }),
                )
            })
            .flatten()
    }
}
