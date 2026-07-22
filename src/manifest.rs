use crate::HostPath;
use crate::Recipe;
use crate::ResultExtension as _;
use crate::VersionRequirement;
use anyhow::Context;
use fs_err::read_dir;
use fs_err::read_to_string;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Deref;
use std::path::Path;
use tracing::warn;

// TODO: Make this opaque and add a transparent `ManifestData` type.
#[derive(Debug)]
pub(crate) struct Manifest {
    path: Box<HostPath>,
    parent_directory: Box<HostPath>,

    data: ManifestData,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ManifestData {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub packages: HashMap<Box<str>, VersionRequirement>,
    /// A map from package name to recipe name.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub providers: HashMap<Box<str>, Box<str>>,
    #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
    pub recipe_directories: Box<[Box<Path>]>,
}

impl Manifest {
    pub(crate) fn read_from(path: Box<HostPath>) -> anyhow::Result<Manifest> {
        let manifest = read_to_string(&path)?;
        let data: ManifestData = toml::from_str(&manifest)?;

        let parent_directory = path
            .parent()
            .with_context(|| format!("getting the parent of `{path}`"))?
            .into();

        Ok(Manifest {
            path,
            parent_directory,
            data,
        })
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

                            // TODO: Don't try here, should be infallible.
                            let path = HostPath::new(&path)?;

                            if entry.file_type().ok_or_log()?.is_dir() {
                                warn!("skipping the directory `{path}`");
                                return None;
                            }

                            Recipe::read_from(path).ok_or_log()
                        }),
                )
            })
            .flatten()
    }
}

impl Display for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the manifest at `{}`", self.path)
    }
}

// Is this a sin?
impl Deref for Manifest {
    type Target = ManifestData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
