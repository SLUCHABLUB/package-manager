mod build;
mod download;

use crate::Version;
use crate::VersionRequirement;
use anyhow::Context;
use fn_error_context::context;
use fs_err::read;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

pub(crate) use build::Build;
pub(crate) use build::BuildSystem;
pub(crate) use download::Compression;
pub(crate) use download::Download;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Recipe {
    #[serde(skip)]
    pub name: Box<str>,

    #[serde(default)]
    pub provides: HashMap<Box<str>, Version>,

    #[serde(default)]
    pub download: Download,
    #[serde(default)]
    pub dependencies: Dependencies,

    #[serde(default)]
    pub build: Build,
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

        recipe.name = Box::from(file_name);

        Ok(recipe)
    }

    pub(crate) fn provides(&self, package_name: &str, version: &VersionRequirement) -> bool {
        self.provides
            .get(package_name)
            .is_some_and(|provided_version| provided_version.satisfies(version))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Dependencies {
    #[serde(flatten)]
    pub versions: HashMap<Box<str>, VersionRequirement>,
}
