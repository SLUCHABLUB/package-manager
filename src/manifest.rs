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
use std::path::Path;
use tracing::warn;

#[derive(Debug)]
pub(crate) struct Manifest {
    path: Box<HostPath>,
    parent_directory: Box<HostPath>,

    data: ManifestData,
}

#[derive(Debug, Serialize, Deserialize)]
struct ManifestData {
    #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
    recipe_directories: Box<[Box<Path>]>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    packages: HashMap<Box<str>, VersionRequirement>,
    /// A map from package name to recipe name.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    providers: HashMap<Box<str>, Box<str>>,
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
        self.data
            .recipe_directories
            .iter()
            .filter_map(|directory| {
                Some(
                    read_dir(self.parent_directory.join(directory))
                        .ok_or_log()?
                        .filter_map(|entry| {
                            let entry = entry.ok_or_log()?;
                            let path = entry.path();

                            let path = HostPath::new(&path)
                                .expect("readdir output should be absolute for absolute input");

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

    pub(crate) fn provider(&self, package: &str) -> Option<&str> {
        self.data.providers.get(package).map(Box::as_ref)
    }

    pub(crate) fn packages(&self) -> impl Iterator<Item = (&str, &VersionRequirement)> {
        self.data
            .packages
            .iter()
            .map(|(package, version)| (&**package, version))
    }
}

impl Display for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the manifest at `{}`", self.path)
    }
}
