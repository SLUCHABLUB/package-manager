use crate::Version;
use crate::VersionRequirement;
use anyhow::Context;
use fs_err::read;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    #[serde(skip)]
    pub name: Box<str>,

    #[serde(default)]
    pub provides: HashMap<Box<str>, Version>,

    pub download: Download,
    pub build: Build,
    #[serde(default)]
    pub install: Install,
    #[serde(default)]
    pub dependencies: Dependencies,
}

impl Recipe {
    pub fn read_from(path: &Path) -> anyhow::Result<Recipe> {
        let filename = path
            .file_name()
            .context("determining the recipe's filename")?;
        let filename = filename.to_string_lossy();
        let filename = filename.strip_suffix(".toml").unwrap_or(&filename);

        let bytes = read(path)?;
        let mut recipe: Recipe = toml::from_slice(&bytes)?;

        recipe.name = Box::from(filename);

        Ok(recipe)
    }

    pub fn provides(&self, package_name: &str, version: &VersionRequirement) -> bool {
        self.provides
            .get(package_name)
            .is_some_and(|provided_version| provided_version.satisfies(version))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Download {
    pub subdirectory: Option<Box<Path>>,
    #[serde(flatten)]
    pub source: DownloadSource,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadSource {
    Github {
        repository: Box<str>,
        version: VersionRequirement,
    },
    Tarball {
        url: Url,
        compression: Option<Compression>,
    },
    TarballIndex {
        url: Url,
        version: VersionRequirement,
        filename_prefix: Box<str>,
    },
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compression {
    None,
    Xz,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    #[serde(default)]
    pub dependencies: Dependencies,

    // The keys and variables should be `OsStr`s but those serialise weirdly.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub environment_variables: HashMap<Box<str>, Box<str>>,
    #[serde(flatten)]
    pub system: BuildSystem,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "system", rename_all = "snake_case")]
pub enum BuildSystem {
    Cargo {
        // TODO: locked: bool,
        // TODO: profile: Box<str>,
        // TODO: no-default-features: bool,
        // TODO: bins/examples
        #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
        features: Box<[Box<str>]>,
        target: Option<Box<str>>,
    },
    ConfigureMake {
        #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
        configure_flags: Box<[Box<str>]>,
    },
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Install {
    /// Install the package into `/usr/local`.
    Local,
    /// Install the package into `/opt`.
    Opt,
    /// Install the package into `/usr`.
    #[default]
    System,
    /// Install the package into `$HOME` using the `$XDG_*_HOME`.
    User,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Dependencies {
    #[serde(flatten)]
    pub versions: HashMap<Box<str>, VersionRequirement>,
}
