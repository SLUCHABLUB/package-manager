use semver::VersionReq;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub name: Box<str>,
    pub author: Box<str>,

    pub download: Download,
    pub build: Build,
    #[serde(default)]
    pub install: Install,
    #[serde(default)]
    pub dependencies: Dependencies,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Download {
    /// The version range that this recipe can build.
    pub version: VersionReq,
    #[serde(flatten)]
    pub source: DownloadSource,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadSource {
    Github { repository: Box<str> },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    #[serde(default)]
    pub dependencies: Dependencies,
    #[serde(flatten)]
    pub system: BuildSystem,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSystem {
    Cargo {
        // TODO: locked: bool,
        // TODO: profile: Box<str>,
        // TODO: no-default-features: bool,
        // TODO: bins/examples
        #[serde(default, skip_serializing_if = "<[_]>::is_empty")]
        features: Box<[Box<str>]>,
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
    pub versions: HashMap<Box<str>, VersionReq>,
}
