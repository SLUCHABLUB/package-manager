mod build;
mod cache;
mod download;

pub(crate) use build::Build;
pub(crate) use build::BuildSystem;
pub(crate) use cache::BuildRoot;
pub(crate) use cache::BuildWorkingDirectory;
pub(crate) use cache::Image;
pub(crate) use cache::Source;
pub(crate) use cache::find_cached_download_lock_or_create;
pub(crate) use download::Compression;
pub(crate) use download::Download;
pub(crate) use download::DownloadLock;

use crate::HostDirectories;
use crate::HostPath;
use crate::Version;
use crate::VersionRequirement;
use anyhow::Context;
use cache::find_cached_repository_or_initialise;
use fn_error_context::context;
use fs_err::read_to_string;
use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use std::collections::BTreeMap;
use std::fmt::Display;
use tracing::info;

#[derive(Debug)]
pub(crate) struct Recipe {
    name: Box<str>,
    path: Box<HostPath>,

    data: RecipeData,
}

impl Recipe {
    // TODO: Take the path owned.
    #[context("parsing the recipe at `{}`", path.display())]
    pub(crate) fn read_from(path: &HostPath) -> anyhow::Result<Recipe> {
        let file_name = path.file_name().context("determining the file name")?;
        let file_name = file_name
            .to_str()
            .context("parsing the file name as utf-8")?;
        let file_name = file_name
            .strip_suffix(".toml")
            .context("stripping the `.toml` extension")?;

        let data = read_to_string(path)?;
        let data: RecipeData = toml::from_str(&data)?;

        Ok(Recipe {
            name: Box::from(file_name),
            path: Box::from(path),
            data,
        })
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn version(&self) -> &Version {
        &self.data.version
    }

    pub(crate) fn build_data(&self) -> &Build {
        &self.data.build
    }

    pub(crate) fn download_data(&self) -> &Download {
        &self.data.download
    }

    pub(crate) fn dependencies(&self) -> &Dependencies {
        &self.data.dependencies
    }

    #[context("locking {self}")]
    pub(crate) fn lock(&self, host: &HostDirectories) -> anyhow::Result<DownloadLock> {
        info!("locking {self}");
        self.data.download.lock(host, &self.data.version)
    }
}

impl Display for Recipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the `{}` recipe at `{}`", self.name, self.path)
    }
}

#[serde_as]
#[derive(Hash, Debug, Serialize, Deserialize)]
struct RecipeData {
    version: Version,

    download: Download,
    build: Build,

    #[serde(default)]
    dependencies: Dependencies,
}

// TODO: Make this opaque and add a `PackageRequirement` type.
#[derive(Hash, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Dependencies {
    #[serde(flatten)]
    pub versions: BTreeMap<Box<str>, VersionRequirement>,
}
