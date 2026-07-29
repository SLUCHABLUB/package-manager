mod build;
mod directories;
mod download;

pub(crate) use build::Build;
pub(crate) use build::BuildSystem;
pub(crate) use directories::BuildRoot;
pub(crate) use directories::BuildWorkingDirectory;
pub(crate) use directories::Image;
pub(crate) use directories::Source;
pub(crate) use download::Compression;
pub(crate) use download::Download;
pub(crate) use download::DownloadLock;

use crate::HostPath;
use crate::Version;
use crate::VersionRequirement;
use anyhow::Context;
use directories::find_cached_repository_or_initialise;
use fn_error_context::context;
use fs_err::read_to_string;
use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use std::collections::HashMap;
use std::fmt::Display;

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

    pub(crate) fn download_data(&self) -> &Download {
        &self.data.download
    }

    pub(crate) fn build_data(&self) -> &Build {
        &self.data.build
    }

    pub(crate) fn dependencies(&self) -> &Dependencies {
        &self.data.dependencies
    }
}

impl Display for Recipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the `{}` recipe at `{}`", self.name, self.path)
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct RecipeData {
    version: Version,

    download: Download,
    build: Build,

    #[serde(default)]
    dependencies: Dependencies,
}

// TODO: Make this opaque and add a `PackageRequirement` type.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Dependencies {
    #[serde(flatten)]
    pub versions: HashMap<Box<str>, VersionRequirement>,
}
