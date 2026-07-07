mod build;
mod directories;
mod download;

pub(crate) use build::Build;
pub(crate) use build::BuildSystem;
pub(crate) use directories::CacheDirectory;
pub(crate) use directories::RecipeDirectories;
pub(crate) use download::Compression;
pub(crate) use download::Download;
pub(crate) use download::DownloadLock;

use crate::Ledger;
use crate::State;
use crate::Version;
use crate::VersionRequirement;
use crate::serde::once_cell_as_option;
use anyhow::Context;
use anyhow::bail;
use fn_error_context::context;
use fs_err::read;
use once_cell::unsync::OnceCell;
use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use std::collections::HashMap;
use std::path::Path;

// TODO: Split this into a "simple" and "cached" type.
// TODO: Make this opaque.
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Recipe {
    #[serde(skip)]
    pub name: Box<str>,

    #[serde(default)]
    pub provides: HashMap<Box<str>, Version>,

    #[serde(default)]
    pub download: Download,
    #[serde(default)]
    pub build: Build,

    #[serde(default)]
    pub dependencies: Dependencies,

    #[serde(default, with = "once_cell_as_option")]
    pub download_lock: OnceCell<DownloadLock>,
    #[serde(default)]
    pub directories: RecipeDirectories,
    #[serde(default, with = "once_cell_as_option")]
    pub ledger: OnceCell<Ledger>,
}

impl Recipe {
    #[context("parsing the recipe at `{}`", path.display())]
    pub(crate) fn read_from(path: &Path) -> anyhow::Result<Recipe> {
        let file_name = path
            .file_name()
            .context("determining the recipe's file name")?;
        let file_name = file_name.to_string_lossy();
        let file_name = file_name.strip_suffix(".toml").unwrap_or(&file_name);

        let bytes = read(path)?;
        let mut recipe: Recipe = toml::from_slice(&bytes)?;

        if recipe.download_lock.get().is_some() {
            bail!("a normal recipe may not provide a download lock");
        }
        if recipe.ledger.get().is_some() {
            bail!("a normal recipe may not provide a ledger");
        }

        recipe.name = Box::from(file_name);

        Ok(recipe)
    }

    pub(crate) fn provides(&self, package_name: &str, version: &VersionRequirement) -> bool {
        self.provides
            .get(package_name)
            .is_some_and(|provided_version| provided_version.satisfies(version))
    }

    #[context("locking the download for the `{}` recipe", self.name)]
    pub(crate) fn download_lock(&self, state: &State) -> anyhow::Result<&DownloadLock> {
        self.download_lock
            .get_or_try_init(|| self.download.lock(&self.directories, state))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Dependencies {
    #[serde(flatten)]
    pub versions: HashMap<Box<str>, VersionRequirement>,
}
